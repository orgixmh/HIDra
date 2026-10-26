#!/usr/bin/env python3
"""Passively retain the last two minutes of one receiver's vendor HID reports.

Run with sudo; press Ctrl+C immediately after the lighting fault. No HID writes
are performed. Boot keyboard, NKRO and media-key payloads are never retained.
"""
import argparse
from collections import deque
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import select
import signal
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--vid', type=lambda v: int(v, 16), required=True)
    parser.add_argument('--pid', type=lambda v: int(v, 16), required=True)
    parser.add_argument('--interface', type=int, required=True)
    args = parser.parse_args()
    if os.geteuid() != 0:
        parser.error('Run this capture with sudo.')

    def locate():
        matches = []
        for node in Path('/sys/bus/usb/devices').iterdir():
            if ':' in node.name:
                continue
            try:
                if (int((node / 'idVendor').read_text(), 16), int((node / 'idProduct').read_text(), 16)) == (args.vid, args.pid):
                    matches.append((node, int((node / 'busnum').read_text()), int((node / 'devnum').read_text())))
            except (OSError, ValueError):
                continue
        if len(matches) != 1:
            raise RuntimeError(f'Expected one matching receiver, found {len(matches)}')
        return matches[0]

    node, bus, device = locate()
    interface = node / f'{node.name}:1.{args.interface}'
    inputs = set()
    for endpoint in interface.glob('ep_*'):
        address = int((endpoint / 'bEndpointAddress').read_text(), 16)
        if address & 0x80:
            inputs.add(address & 0x0f)
    if not inputs:
        raise RuntimeError('The selected receiver interface has no input endpoint')
    subprocess.run(['modprobe', 'usbmon'], check=True)
    fd = os.open(f'/sys/kernel/debug/usb/usbmon/{bus}u', os.O_RDONLY | os.O_NONBLOCK)
    output = Path('/tmp') / f'hidra-rf-{datetime.now().strftime("%Y%m%d-%H%M%S")}-{os.getpid()}.jsonl'
    saved = output.open('x')
    os.fchmod(saved.fileno(), 0o600)
    if 'SUDO_UID' in os.environ and 'SUDO_GID' in os.environ:
        os.fchown(saved.fileno(), int(os.environ['SUDO_UID']), int(os.environ['SUDO_GID']))
    started = time.monotonic()
    header = dict(type='capture', utc=datetime.now(timezone.utc).isoformat(), vid=f'{args.vid:04x}',
                  pid=f'{args.pid:04x}', interface=args.interface, bus=bus, device=device,
                  history_seconds=120, report_id='13')
    events = deque()
    pending = set()
    stopped = None
    reason = 'Ctrl+C'

    def stop(_signum, _frame):
        nonlocal stopped
        if stopped is None:
            stopped = time.monotonic()

    signal.signal(signal.SIGINT, stop)
    signal.signal(signal.SIGTERM, stop)
    buffer = b''
    refresh = started
    present = True
    reports = 0
    print(f'Capturing RF lighting only; retaining the last 120 seconds.\nOutput: {output}\nPress Ctrl+C immediately when all LEDs flicker.', flush=True)
    try:
        while stopped is None:
            now = time.monotonic()
            while events and events[0]['seconds'] < now - started - 120:
                events.popleft()
            if now - refresh >= .5:
                refresh = now
                try:
                    _, new_bus, new_device = locate()
                    if new_bus != bus:
                        reason = 'Receiver moved to another USB bus'; stopped = now; continue
                    if not present or new_device != device:
                        events.append(dict(type='reconnected', seconds=now-started, device=new_device))
                        device = new_device; pending.clear()
                    present = True
                except RuntimeError:
                    if present:
                        events.append(dict(type='disconnected', seconds=now-started))
                    present = False
            if not select.select([fd], [], [], .1)[0]:
                continue
            try:
                buffer += os.read(fd, 65536)
            except BlockingIOError:
                continue
            while b'\n' in buffer:
                raw, buffer = buffer.split(b'\n', 1)
                line = raw.decode('ascii', errors='replace')
                parts = line.split()
                if len(parts) < 5:
                    continue
                if not present:
                    continue
                match = re.fullmatch(r'([A-Za-z]+):(\d+):(\d+):(\d+)', parts[3])
                if not match or int(match[2]) != bus or int(match[3]) != device:
                    continue
                transfer, endpoint = match[1], int(match[4])
                event = dict(seconds=round(time.monotonic()-started, 6), kernel_us=int(parts[1]),
                             event=parts[2], transfer=parts[3], urb=parts[0])
                payload = b''
                if ' = ' in line:
                    try:
                        payload = bytes.fromhex(''.join(line.split(' = ', 1)[1].split()))
                    except ValueError:
                        pass
                vendor = len(payload) == 20 and payload[0] == 0x13
                outbound = parts[2] == 'S' and transfer in ('Co', 'Bo', 'Io')
                inbound = parts[2] == 'C' and transfer in ('Ci', 'Bi', 'Ii') and endpoint in inputs
                if vendor and (outbound or inbound):
                    event.update(type='report', report=payload.hex(), checksum_ok=sum(payload[:19]) % 256 == payload[19])
                    events.append(event); reports += 1
                    if outbound:
                        pending.add(parts[0])
                elif parts[2] in ('C', 'E') and (parts[0] in pending or (transfer == 'Ii' and endpoint in inputs and parts[4] != '0')):
                    event.update(type='completion', status=parts[4])
                    events.append(event)
                if parts[2] in ('C', 'E'):
                    pending.discard(parts[0])
    except Exception as error:
        reason = f'Capture error: {error}'
    finally:
        ended = stopped if stopped is not None else time.monotonic()
        saved.write(json.dumps(header) + '\n')
        for event in events:
            saved.write(json.dumps(event) + '\n')
        saved.write(json.dumps(dict(type='stop', seconds=ended-started,
                                   utc=datetime.now(timezone.utc).isoformat(), reason=reason,
                                   total_reports=reports)) + '\n')
        saved.flush(); os.fsync(saved.fileno()); saved.close(); os.close(fd)
        print(f'Capture stopped. Saved {len(events)} recent events to:\n{output}', flush=True)


if __name__ == '__main__':
    main()
