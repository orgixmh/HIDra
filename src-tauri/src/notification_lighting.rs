// SPDX-License-Identifier: GPL-3.0-or-later
//! Temporary notification frames. No flash writes occur on incoming alerts.
use crate::devices::{Protocol, SinowealthSpec};
use crate::error::{DevError, Result};
use crate::hid::Link;
use crate::sinowealth;
use hidapi::HidApi;
use serde::{Deserialize, Serialize};
use std::{thread, time::Duration};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Snapshot {
    pub identity: [u8; 6],
    #[serde(default)]
    pub receiver: bool,
    pub config: Vec<u8>,
}

pub struct Lighting {
    link: Link,
    spec: &'static SinowealthSpec,
    pub snapshot: Snapshot,
}

impl Lighting {
    pub fn capture() -> Result<Self> {
        let api = HidApi::new()?;
        let (link, spec) = sinowealth::open(&api)?;
        if spec.stream_config.is_empty() {
            return Err(DevError::Invalid(
                "Smart Notifications is not supported by this keyboard".into(),
            ));
        }
        let config = sinowealth::read(&link, 0x84, 0, 128)?;
        let snapshot = Snapshot {
            identity: spec.identity,
            receiver: link.protocol == Protocol::CompxKeyboard20,
            config,
        };
        validate_snapshot(spec, &snapshot)?;
        Ok(Self {
            link,
            spec,
            snapshot,
        })
    }

    pub fn activate(&self) -> Result<()> {
        // Change only the mode configuration. Never rewrite the saved per-key
        // colour table: notification frames are volatile command 0x08 packets.
        let mut config = self.snapshot.config.clone();
        for &(offset, value) in self.spec.stream_config {
            config[offset] = value;
        }
        if config != self.snapshot.config {
            sinowealth::write_verified(&self.link, 0x04, &config)?;
        }
        // The config write schedules a firmware repaint. Streaming before that
        // repaint races it and makes a correctly transmitted frame disappear.
        if self.link.protocol == Protocol::CompxKeyboard20 {
            thread::sleep(Duration::from_millis(1500));
            // The worker sends the chosen background next, then continues
            // sending full volatile frames on its existing keepalive cadence.
            Ok(())
        } else {
            thread::sleep(Duration::from_millis(400));
            self.frame(&[])
        }
    }

    pub fn frame(&self, cells: &[(u16, [u8; 3])]) -> Result<()> {
        let frame = live_frame(self.spec, cells)?;
        if self.link.protocol == Protocol::CompxKeyboard20 {
            let next: Vec<_> = self
                .spec
                .keys
                .iter()
                .map(|key| {
                    let offset = key.off as usize;
                    (
                        key.off,
                        [frame[offset], frame[offset + 1], frame[offset + 2]],
                    )
                })
                .collect();
            crate::compx_keyboard::stream(&self.link, &next)?;
            return Ok(());
        }
        self.link
            .feature_write(&sinowealth::packet(0x08, 0, frame.len(), &frame)?)
    }

    pub fn restore(&self, snapshot: &Snapshot) -> Result<()> {
        validate_snapshot(self.spec, snapshot)?;
        if snapshot.receiver != (self.link.protocol == Protocol::CompxKeyboard20) {
            return Err(DevError::Invalid("Reconnect the original USB/RF connection to restore the previous lighting. Configuration from another transport was refused.".into()));
        }
        // Restore the captured mode configuration; saved per-key colours stay untouched.
        sinowealth::write_verified(&self.link, 0x04, &snapshot.config)
    }
}

/// A pending snapshot belongs to one transport. Defer it without writing when
/// the owner has switched connections; the worker retains it for later recovery.
pub fn recover(snapshot: &Snapshot) -> Result<bool> {
    let lighting = Lighting::capture()?;
    validate_snapshot(lighting.spec, snapshot)?;
    if snapshot.receiver != lighting.snapshot.receiver {
        return Ok(false);
    }
    lighting.restore(snapshot)?;
    Ok(true)
}

fn validate_snapshot(spec: &SinowealthSpec, snapshot: &Snapshot) -> Result<()> {
    if snapshot.identity != spec.identity
        || snapshot.config.len() != 128
        || snapshot.config[126..] != [0x5a, 0xa5]
    {
        return Err(DevError::Invalid(
            "The lighting recovery snapshot does not match this keyboard".into(),
        ));
    }
    Ok(())
}

fn live_frame(spec: &SinowealthSpec, cells: &[(u16, [u8; 3])]) -> Result<[u8; 378]> {
    let mut frame = [0; 378];
    for &(off, rgb) in cells {
        let offset = off as usize;
        if offset % 3 != 0
            || offset + 3 > frame.len()
            || !spec.keys.iter().any(|key| key.off == off)
        {
            return Err(DevError::Invalid("unknown physical key".into()));
        }
        frame[offset..offset + 3].copy_from_slice(&rgb);
    }
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::SUPPORTED;
    fn spec() -> &'static SinowealthSpec {
        SUPPORTED
            .iter()
            .find_map(|device| device.sinowealth)
            .unwrap()
    }
    #[test]
    fn live_frames_use_oem_offsets_and_leave_other_keys_black() {
        let key = spec().keys.iter().find(|key| key.label == "Q").unwrap();
        let frame = live_frame(spec(), &[(key.off, [0x12, 0x34, 0x56])]).unwrap();
        assert_eq!(&frame[24..27], &[0x12, 0x34, 0x56]);
        assert_eq!(frame.iter().filter(|&&v| v != 0).count(), 3);
        assert!(live_frame(spec(), &[(1, [255; 3])]).is_err());
        assert!(live_frame(spec(), &[(600, [255; 3])]).is_err());
    }
    #[test]
    fn recovery_requires_the_original_model_and_marker() {
        let mut snapshot = Snapshot {
            identity: spec().identity,
            receiver: false,
            config: vec![0; 128],
        };
        assert!(validate_snapshot(spec(), &snapshot).is_err());
        snapshot.config[126..].copy_from_slice(&[0x5a, 0xa5]);
        assert!(validate_snapshot(spec(), &snapshot).is_ok());
        snapshot.identity[5] ^= 1;
        assert!(validate_snapshot(spec(), &snapshot).is_err());
    }
    #[test]
    #[ignore = "temporarily takes over lighting for a live-stream test, then restores it"]
    fn hardware_stream_roundtrip() {
        let lighting = Lighting::capture().unwrap();
        let result = (|| -> Result<()> {
            lighting.activate()?;
            let off = lighting
                .spec
                .keys
                .iter()
                .find(|key| key.label == "Q")
                .unwrap()
                .off;
            for _ in 0..12 {
                lighting.frame(&[(off, [0, 160, 255])])?;
                thread::sleep(Duration::from_millis(350));
            }
            lighting.frame(&[])?;
            Ok(())
        })();
        if let Err(error) = &result {
            eprintln!("Streaming test error: {error}");
        }
        let restore = lighting.restore(&lighting.snapshot);
        restore.unwrap();
        result.unwrap();
        println!("Live Q-key frames sent; mode configuration restored; saved per-key colours were not rewritten");
    }
}
