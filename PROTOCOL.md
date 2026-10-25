# Hardware protocols

Reverse-engineering notes for the devices HIDra talks to. The **keyboard**
protocol was captured live by running the official Windows tool under Wine with
a `/dev/uhid` mirror of the real HID interfaces and the GUI driven by `xdotool`.
The **mouse** protocol comes from the open-source `open-m711pro` project
(validated on identical hardware) and cross-checked against `m913-ctl` /
`mouse_m908`.

All I/O is plain HID: feature reports and output/input reports. On Linux that is
`hidraw` + `HIDIOCSFEATURE`; the udev rule (`uaccess`) is installed as
`/usr/lib/udev/rules.d/70-hidra.rules` by the packages, or
`/etc/udev/rules.d/70-hidra.rules` by `packaging/install-udev.sh`.

## Where the configuration channel lives

Neither device is configured through the collection that types letters or moves
the pointer. Both put configuration on a **vendor-defined collection on USB
interface 1**, and the exact collection matters on Windows, where each top-level
collection is a separate handle:

| Device | Interface | Write collection | Reply collection |
|---|---|---|---|
| M711-PRO mouse | 1 | usage page `0xFF02`, usage `0x02` (feature report `0x08`) | usage page `0xFF01` (input report `0x09`) |
| K586RGB-PRO keyboard | 1 | usage page `0xFF1C`, usage `0x92` (report `0x04`) | same collection |

The mouse answering on a *different* collection than the one it is written to is
the reason `devices.rs` carries both `config_usage_page` and `reply_usage_page`.

---

## Mouse — Cobra Pro / M711-PRO (Compx `25A7:FA07` receiver, `FA08` cable)

- Config interface: **1**. **Feature** reports, id `0x08`, 17 bytes.
- Write: `08 07 00 <ADDR_HI> <ADDR_LO> <LEN> <data…> <padding> <CKSUM>`
- Read: `08 08 00 <hi> <lo> <len>`; the answer arrives as input report id `0x09`
  on the interrupt endpoint, same layout.
- Commit (apply/save): `08 04 …`, sent after every burst.
- **Checksum**: every byte sums to `0x55` (mod 256), both for the 17-byte packet
  and for each memory "record": `ck = (0x55 - sum) & 0xFF`.

### Register map
| Address | Contents |
|---|---|
| `0x0000` | Polling rate: `01`=1000 Hz, `02`=500, `04`=250, `08`=125 |
| `0x0002` | Number of active DPI stages |
| `0x000C`–`0x001F` | DPI table: 5 stages × 4 bytes (`X Y 00 ck`) |
| `0x002C`–`0x003F` | Per-stage DPI LED colour: 5 × 4 bytes (`R G B ck`) |
| `0x004C` | "DPI Effect" (stage indicator LED): `01` on, `00` off |
| `0x0060`–`0x009F` | Button table: 16 records × 4 bytes (`TYPE P1 P2 ck`) |
| `0x00A0` | LED: `EFFECT R G B SPEED BRIGHTNESS ck` |
| `0x0100 + 0x20*n` | Extended slot for button n (media / key / combo) |

- **DPI**: `byte = round(dpi*3/250) - 1` (~83.33 DPI per step);
  `dpi = (byte+1)*250/3`. Ceiling `0xBD`.
- **LED effects** (the `EFFECT` byte at `0x00A0`): `00` cycle/streaming,
  `01` breathing, `02` steady, `04` off. Brightness `0x00`–`0xFF` (`0xFF` = 100%).
  Lower speed value = faster.
- **Button action types**: `00` disabled · `01` mouse button (P1 = bitmask
  L1/R2/M4/Back8/Fwd16) · `02` DPI (`01` cycle, `02` +, `03` −) · `04`
  repeated click / fire (P1 = speed, P2 = click count) · `05` action in the
  extended slot · `07` polling switch · `08` RGB on/off.
- **Extended slot** (event list):
  `<event count> [<type> <code_lo> <code_hi>]* <ck>`. Types: `80`/`40` modifier
  down/up (HID bitmask), `81`/`41` key down/up (HID keyboard usage), `82`/`42`
  media down/up (Consumer Page usage, 16-bit). Ctrl+C =
  `04 | 80 01 00 | 81 06 00 | 40 01 00 | 41 06 00 | ck`.
- Physical index → table slot: buttons 1–5 = 0–4; button 6 (behind the wheel,
  the nearer one) = 10; button 7 = 11; button 8 = 9.

### Writes that do not apply
Reads work in both connection modes, but a write can be silently ignored while
the mouse is idle — the 2.4 GHz radio sleeps. HIDra fans every write out to all
present mouse interfaces (receiver and cable) and the awake one takes it. If a
setting does not stick, move the mouse while applying.

---

## Keyboard — Brahma Pro / K586RGB-PRO (SONiX SN32F248B, `0C45:5004`)

- Config interface: **1** (usage page `0xFF1C`). Output reports, id `0x04`,
  **64 bytes**. The answer comes back as input report id `0x04`.
- **Checksum**: 16-bit sum of bytes `[3..63]`, stored at `[1]` (LO) and `[2]` (HI).
- Write sequence: **BEGIN** `04 .. .. 01 00…` → the specific packets →
  **END** `04 .. .. 02 00…`.
- Command header: `04 <CK_LO> <CK_HI> <CMD> <LEN> <OFF_LO> <OFF_HI> 00 <data…>`.

### Commands
| CMD | Function |
|---|---|
| `01` / `02` | BEGIN / END (applies the batch) |
| `03` | read profile block (0x2C bytes @ 0) |
| `04` | write profile block (switches the active profile) |
| `05` | read lighting config block (0x38 @ `0x00` / `0x2A` / `0x54`) |
| `06` | write lighting parameter |
| `07` / `0f` | read keymap (factory / active) |
| `08` | write keymap (8 packets of 0x38) |
| `10` / `11` | read / write per-key colours (Custom mode) |
| `0a` | write macro table |

`07` returns the **factory** keymap and never changes; `0f` returns the active
one and reflects what was written.

### Lighting parameters (CMD `06`, shape `06 <LEN> <OFF> 00 00 <data>`)
One block per profile, at base offsets **P1 = 0x00, P2 = 0x2A, P3 = 0x54**.
Within the block:

| Offset | Field | Observed values |
|---|---|---|
| `+0` | Effect (mode) | 1..18, and 20 = Custom |
| `+1` | Brightness | 0..5 (default 5) |
| `+2` | Speed | 1..5 (default 3; lower = faster) |
| `+3` | Direction | `00` right, `FF` left |
| `+4` | Full RGB / multicolour | `00`/`01` |
| `+5..7` | Colour R,G,B | e.g. `FF 00 00`, written as `06 03 05 00 00 R G B` |

### Effects (the mode byte at `+0`)
In the order the vendor's combo box lists them (names as shipped in the
Portuguese build): 1 Corredor · 2 A Nuvem · 3 Lâmina Veloz · 4 Spectrum ·
5 Respiração · 6 Sólido · 7 Reativo · 8 Ondular · 9 Reativo (Horizontal) ·
10 Florescer Gélido · 11 Rainbow · 12 Corrida das Sombras · 13 Tornado ·
14 Recarregar · 15 A Matrix · 16 Surmount · 17 Passagem Dupla · 18 Vulto do
Spectro · **20 Custom** (per-key). IDs are index+1; Custom is `0x14`.

### Keymap (CMD `08`)
420 bytes = ~140 entries of 3 bytes, sent as 8 packets of `0x38`. Each entry:
`02 02 <usage>` (key), `02 01 <mod>` (modifier), `03 <lo> <hi>` (media),
`05 xx yy` (special functions / FN layer), `00 00 00` (empty). The factory dump
of this keyboard is in `docs/kb_default_keymap.bin`; inverting it gives
physical key → table offset. Verified: remapping Q→X applied on hardware and
read back through CMD `0f`.

### Per-key colours (CMD `11`, Custom mode)
Every key has a **fixed offset** in the LED buffer (6×20 matrix,
`offset = slot*3`). The official app paints one key at a time with
`04 .. .. 11 03 <off_lo> <off_hi> 00 <R G B>`, inside `BEGIN … END`, with the
mode set to `0x14` (Custom). The full key→offset map was measured on hardware
(clicking each key in the official app and reading the `11 03 <off>` it emitted)
and lives in `docs/custom_key_offsets.json` and `src/keyboardLayout.ts`.
Read back with CMD `10` at the same offset. Verified end to end: painting Q/A/
Space through HIDra produced exactly those keys and colours on the hardware.

### Active profile (CMD `03` read / `04` write)
Which of the 3 profiles the keyboard displays. Switch it by writing the 44-byte
block `55 aa ff 02 45 0c 04 50 04 01 <P> 38 00 00 00 00 01 02 03 04 05 06 07 08
09 0a 0b 0c 0d 0e 0f 11 10 12 14` (+9 zero bytes) through CMD `04`, where
byte[10] `<P>` is the profile index (0..2). Read it back with CMD `03`
(byte[10] of the block). Verified: selecting a profile in HIDra switches the
profile the keyboard displays, same as the official tool.

### Macros (CMD `0a`)
Header `AA 55 <len16> 00 01 00 01 00`, followed by a count and the events; the
macro name is UTF-16. Per-event repeat counts and delays are supported.

---

## Capturing a protocol yourself

The recipe that produced the keyboard notes above, on Linux with no second
machine, is written up in [docs/ADDING-DEVICES.md](docs/ADDING-DEVICES.md).

## SinoWealth keyboard — Redragon UCAL K673SP-RGB-PRO (USB cable)

This is a separate protocol from the SONiX Brahma above. The shared `258A:010C`
USB ID alone is not a model identifier. Interface 1 exposes usage page FF00,
usage 1, with a 519-byte feature payload on report ID 6 (520 bytes including
report ID). Other vendor collections use IDs 3 and 5 and are not configuration
channels for this driver.

### Sources and identification

- Official [K673SP download page](https://redragonshop.com/blogs/product-download/ucal-k673-pro-side-printed-pbt-keycaps),
  package `Redragon_K673RGB-PRO_Setup_v2.0_20240910.exe`.
- Model data from `Dev/kb/PRO-MAX/KB.ini`: `Psd=3,0,0,0,0,ba`, brightness
  and speed 0–4, LedOpt1–18, and the 81 visible key rectangles/matrix slots.
  The other `PRO` entry has identity ending in 73; it is **not** enabled.
- Transport cross-checked against the [AULA capture notes](https://github.com/veysiemrah/aula-rgb-controller/blob/main/tools/protocol_notes.md)
  and [DeetsRGB protocol documentation](https://github.com/deets-137/DeetsRGB/blob/main/docs/protocol.md).
  Their model-specific effect maps must not be substituted for K673 data.

The user's K673SP on Linux X11 returned model reply
`06 82 01 00 01 00 06 00 03 00 00 00 00 BA` on 2026-10-25.
Read-only queries also returned a 136-byte config reply ending in `5A A5`,
and the complete 520-byte palette reply. These observations prove discovery
and reads, not that a visual lighting change has been verified.

### Framing and lighting

Feature packets are `06 CMD ARG 00 01 00 LEN_LO LEN_HI DATA...`, zero-padded
outbound to 520 bytes. A read first sends the query as SET_FEATURE, waits 50ms,
and GET_FEATURE retrieves report 6. Validate the returned byte count and the
entire eight-byte echoed header; reject short, zero or unrelated replies.

| Query / write | ARG | Payload | Purpose |
| --- | --- | --- | --- |
| 82 / none | 1 | 6 | Full model signature; must match the table before configuration |
| 84 / 04 | 0 | 128 | Lighting configuration, marker at payload offsets 126–127 |
| 8A / 0A | 0 | 512 | Effect colour palette; seven RGB slots per effect |
| 86 / 06 | 0 | 384 | Custom RGB planes, 126 bytes per channel plus preserved padding |

Config payload offset 9 selects preset (0) or custom (1); offset 10 selects
effect (0 off, 1–18 OEM effects, 21 custom). K673 effect parameters occupy
pairs beginning at offset 58: brightness, then `(speed << 4) | colour-slot`.
The device table explicitly records each effect's offset and capabilities.
Slot 0 is user RGB; slots 1–6 are presets and slot 7 is rainbow. Palette RGB
begins at `effect * 21 + colour-slot * 3`.

Every write reads the complete existing block first, changes only the requested
fields and preserves the rest. The config marker and full model signature are
mandatory. Palette is saved before config; after a 60ms settle delay the driver
reads the block back and compares it, returning an error on mismatch. No default
block is written after a failed read. No firmware or bootloader commands exist.

Custom colours use the OEM matrix slot, not Brahma's row order: channel bytes
are `slot`, `126+slot`, `252+slot`. UI `off` is `slot*3` for compatibility with the
existing colour editor. All requested offsets must exist in this model's table.
Key remapping, macros, Bluetooth/receiver transport and profile switching are
not implemented for this protocol; those controls are unavailable for K673.

### Local validation

Run the normal Rust tests without hardware. The ignored
`sinowealth::tests::hardware_read_only` test opens the attached model and reads
its settings/custom colour buffer without changing any settings. Hardware
validation additionally requires applying a small brightness/colour change in
the app, observing the keyboard, and reading/reopening the app to confirm it.
Keep `Support::Reported` until the owner confirms the physical result.

On 2026-10-25 the owner confirmed that both brightness and colour changes work
on the physical K673SP-RGB-PRO, on Ubuntu 24.04.5 / X11. The Rust driver also
passed a non-root hardware round trip: brightness 4 -> 3 was read back from the
keyboard, then the complete original config was restored and compared. The
384-byte custom-colour buffer was successfully read; per-key writes/layout have
not yet been visually confirmed. `Support::Verified` records the owner's
brightness/colour confirmation, not a claim that every effect or key was tested.

### Idle behaviour

Device discovery for this protocol performs an active model query. Repeating
that query every four seconds was reported to interrupt the K673's lighting
(blinking). The app therefore discovers devices at startup and when the user
presses **Atualizar dispositivos**; it has no background device polling timer.
Settings are loaded when a connected keyboard is selected or its profile changes,
and after an explicit Apply. Write verification remains part of that action.

The owner confirmed on 2026-10-25 (Ubuntu 24.04.5 / X11) that leaving HIDra
open and idle after this change no longer causes periodic keyboard blinking.

### Notification streaming (Linux)

The K673 OEM lighting stream uses command `08`, ARG 0, payload length 378.
Its RGB bytes are interleaved by matrix slot (`off = slot*3`), unlike the saved
command `06` colour planes. Only keys in the model table are accepted. To enter
streaming, preserve the captured 128-byte config and apply the table's patches:
offset 9 = 1, 10 = 21, 59 = 0x47. Wait 400ms for the firmware's mode repaint
before sending a black frame. Keep sending the current volatile frame every
350ms, including the selected background when no alert is active; the firmware does not retain
this stream indefinitely. No model queries, configuration reads, reconnects or
persistent colour writes occur per frame. Manual controls are gated while owned.

The original mode config is saved atomically before activation, restored and
verified on Off/normal exit, and retained for recovery after errors or interruption.
The saved command `06` per-key colour table is never modified by notifications.
A command `06` restoration in the first experimental stream test returned a
feature-report protocol error; production notification code therefore uses only
volatile `08` frames and mode configuration `04`, and never rewrites that table.
The owner confirmed Q illuminated cyan in both direct streaming tests on
2026-10-25; the second test also successfully verified the entry config on restore.
The owner subsequently confirmed the integrated UI, On/Off flow and a real CLI
notification matching a text filter worked on the same keyboard/desktop.

Desktop events are monitored on a private session D-Bus connection using
`org.freedesktop.DBus.Monitoring.BecomeMonitor`, restricted to FDO `Notify` and
GTK `AddNotification`. Portal-forwarded notifications arrive at these final
endpoints too. The monitor never takes ownership of the notification service.
Rules match case-insensitive app substrings and optional title/body substrings;
a blank app accepts all browsers. Notification contents remain in a bounded RAM
history, while rules and the enabled preference are stored in the app config
directory. On mode restore failure the recovery file and exclusive gate remain
so manual writes cannot overwrite the pending original configuration.

Notification rendering now fills every supported physical key with the user's
background RGB, then overlays active alerts in arrival order. Optional blinking
alternates alert/background every 700ms while retaining the 350ms volatile
keepalive cadence. Neither background nor blinking changes the saved colour table.
An expired/cleared alert reveals the previous overlapping alert or background.

Durations are 0 (until desktop dismissal), 60, 300 or 600 seconds. Legacy durations
1–59 migrate to 0; tests run for 10 seconds and Clear alerts affects only HIDra.
FDO Notify calls are correlated by caller unique name + serial with the server's
method return, producing the real notification ID. Ignore GNOME's forwarded Notify
copy; its shell and public proxy share IDs. Observe NotificationClosed from both
(the proxy may no longer forward to a client that exited). Track every notification
separately and replace its previous alerts when its ID is updated. GTK Add/Remove
uses (app ID, notification ID), but user dismissal has no standard GTK signal;
manual clearing/fixed durations remain available. A public notification daemon
restart ends monitoring with an explicit retry error rather than reusing stale IDs.
No notification-centre history is imported or persisted.
