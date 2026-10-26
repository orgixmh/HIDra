// SPDX-License-Identifier: GPL-3.0-or-later
//! Compx receiver transport: 20-byte report 0x13, independent of wired features.
use crate::{
    error::{DevError, Result},
    hid::Link,
};
use std::{
    collections::BTreeMap,
    thread,
    time::{Duration, Instant},
};
const ATTEMPTS: usize = 10;

fn invalid(message: &str) -> DevError {
    DevError::Invalid(message.into())
}
fn packet(command: u8, count: u8, index: u8, payload: &[u8], streaming: bool) -> [u8; 20] {
    let mut frame = [0; 20];
    frame[0] = 0x13;
    frame[1] = command;
    frame[2] = count;
    frame[3] = index;
    frame[4] = payload.len() as u8 + if streaming { 16 } else { 0 };
    frame[5..5 + payload.len()].copy_from_slice(payload);
    frame[19] = frame[..19].iter().fold(0u8, |sum, &v| sum.wrapping_add(v));
    frame
}
fn drain(link: &Link) -> Result<()> {
    for _ in 0..128 {
        if link.mouse_read(0)?.is_none() {
            break;
        }
    }
    Ok(())
}
pub fn read(link: &Link, logical: u8, arg: u8, length: usize) -> Result<Vec<u8>> {
    read_attempts(link, logical, arg, length, ATTEMPTS)
}
fn read_attempts(
    link: &Link,
    logical: u8,
    arg: u8,
    length: usize,
    attempts: usize,
) -> Result<Vec<u8>> {
    let (command, expected) = match (logical, arg, length) {
        (0x82, 1, 6) => (0x05, 10),
        (0x84, 0, 128) => (0x44, 128),
        (0x8a, 0, 490) => (0x49, 490),
        _ => {
            return Err(invalid(
                "Saved per-key colours over RF are not supported yet. Use the USB cable.",
            ))
        }
    };
    // A receiver can lose an entire query or one page of a larger reply.
    // Collect each retry independently; never accept an incomplete block.
    let mut last_pages = 0;
    let mut last_total = None;
    for attempt in 0..attempts {
        thread::sleep(Duration::from_millis(50));
        drain(link)?;
        link.output_write(&packet(command, 0, 0, &[], false))?;
        let mut pages = BTreeMap::new();
        let mut total = None;
        let deadline = Instant::now() + Duration::from_millis(1500);
        let mut last_page = None;
        while Instant::now() < deadline {
            // Normal replies finish in about 150 ms. Once a partial reply has
            // gone quiet, request a fresh block instead of waiting indefinitely.
            if last_page.is_some_and(|time: Instant| time.elapsed() > Duration::from_millis(400)) {
                break;
            }
            let Some(frame) = link.mouse_read(100)? else {
                continue;
            };
            if frame.len() != 20 || frame[0] != 0x13 || frame[1] != command {
                continue;
            }
            let checksum = frame[..19].iter().fold(0u8, |sum, &v| sum.wrapping_add(v));
            if checksum != frame[19]
                || frame[2] == 0
                || frame[2] > 37
                || frame[3] >= frame[2]
                || frame[4] > 14
            {
                return Err(invalid("Invalid RF receiver reply"));
            }
            if total.is_some_and(|count| count != frame[2]) {
                return Err(invalid("RF reply changed page count"));
            }
            total = Some(frame[2]);
            let payload = frame[5..5 + frame[4] as usize].to_vec();
            if let Some(previous) = pages.insert(frame[3], payload.clone()) {
                if previous != payload {
                    return Err(invalid("RF reply contained conflicting pages"));
                }
            }
            last_page = Some(Instant::now());
            if pages.len() == frame[2] as usize {
                let data: Vec<u8> = pages.into_values().flatten().collect();
                if data.len() != expected {
                    return Err(invalid("RF receiver returned an incomplete configuration"));
                }
                return Ok(if logical == 0x82 {
                    data[..6].to_vec()
                } else {
                    data
                });
            }
        }
        last_pages = pages.len();
        last_total = total;
        log::warn!(
            "RF query {command:02x}, attempt {}: received {last_pages}/{last_total:?} pages",
            attempt + 1
        );
    }
    Err(invalid(&format!(
        "RF communication incomplete (query {command:02X}, {last_pages}/{} pages after {attempts} attempts). Check the receiver connection and retry.",
        last_total.map_or_else(|| "?".to_owned(), |count| count.to_string())
    )))
}

fn send_message(link: &Link, command: u8, bytes: &[u8], streaming: bool) -> Result<()> {
    let count = bytes.len().div_ceil(14);
    if count == 0 || count > 37 {
        return Err(invalid("Invalid RF message length"));
    }
    for (index, chunk) in bytes.chunks(14).enumerate() {
        link.output_write(&packet(command, count as u8, index as u8, chunk, streaming))?;
        thread::sleep(Duration::from_millis(if streaming { 20 } else { 60 }));
        // Consume acknowledgements between persistent write pages rather than
        // queueing the entire block while the receiver is still processing it.
        if !streaming {
            drain(link)?;
        }
    }
    drain(link)
}
pub fn write_verified(link: &Link, logical: u8, data: &[u8]) -> Result<()> {
    let (command, query) = match (logical, data.len()) {
        (0x04, 128) if data[126..] == [0x5a, 0xa5] => (0x04, 0x84),
        (0x0a, 490) => (0x09, 0x8a),
        _ => return Err(invalid("Unsupported RF configuration write")),
    };
    let mut send = true;
    let mut last = invalid("RF keyboard did not confirm the saved settings");
    for attempt in 0..ATTEMPTS {
        if send {
            send_message(link, command, data, false)?;
            send = false;
        }
        thread::sleep(Duration::from_millis(250));
        match read_attempts(link, query, 0, data.len(), 1) {
            Ok(actual) if actual == data => return Ok(()),
            Ok(actual) => {
                let different = actual.iter().zip(data).filter(|(a, b)| a != b).count();
                log::warn!(
                    "RF write {command:02x}, attempt {}: {different} bytes differ",
                    attempt + 1
                );
                // A complete read-back proves the lighting block was not saved.
                // Resend that same validated block, never a partial-page patch.
                send = true;
                last = invalid("RF keyboard did not confirm the saved settings after 10 attempts");
            }
            Err(error) => {
                // An uncertain read is not evidence of a failed write. Retry
                // verification without sending another persistent write.
                last = error;
            }
        }
    }
    Err(last)
}

pub fn stream(link: &Link, cells: &[(u16, [u8; 3])]) -> Result<()> {
    let mut groups: BTreeMap<[u8; 3], Vec<u8>> = BTreeMap::new();
    for &(off, rgb) in cells {
        groups.entry(rgb).or_default().push((off / 3) as u8);
    }
    let mut message = Vec::new();
    for (rgb, indices) in groups {
        for group in indices.chunks(u8::MAX as usize) {
            message.extend(rgb);
            message.push(group.len() as u8);
            message.extend(group);
        }
    }
    // One logical frame must remain one message, fragmented only by the HID
    // transport. Splitting at 80 keys made the final message contain just the
    // last key on an 81-key board. Refuse an oversized frame before sending any.
    if message.len() > 37 * 14 {
        return Err(invalid(
            "RF lighting frame exceeds the receiver message limit",
        ));
    }
    if !message.is_empty() {
        send_message(link, 0x88, &message, true)?;
    }
    Ok(())
}
