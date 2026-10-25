// HIDra — open-source configurator for HID gaming peripherals.
// Copyright (C) 2026 HIDra contributors.
// SPDX-License-Identifier: GPL-3.0-or-later

//! HID discovery and low-level transport.
//!
//! Which devices exist and where their configuration endpoint lives is data,
//! not code: it all comes from [`crate::devices::SUPPORTED`]. This module turns
//! one of those entries into an open [`Link`].
//!
//! Picking the right endpoint differs per platform, which is why a device entry
//! carries both an interface number and a usage page:
//!
//! * **Linux** exposes the whole USB interface as one `/dev/hidraw*` node, so
//!   the interface number selects it and every collection shares the handle.
//! * **Windows** splits each top-level collection into its own handle, so the
//!   usage page is what tells the vendor collection apart from the keyboard or
//!   mouse collection sitting on the same interface.
//! * **macOS** reports one device per interface, like Linux, but does not always
//!   fill in the interface number.
//!
//! We open the device fresh for each operation, which keeps us resilient to the
//! wireless mouse going to sleep and to replug.

use crate::devices::{self, Device, Protocol};
use crate::error::{DevError, Result};
use hidapi::{HidApi, HidDevice};

pub use crate::devices::Kind;

/// A discovered configuration endpoint: the device it belongs to plus the
/// handle path(s) to reach it.
#[derive(Clone, Debug)]
pub struct Found {
    pub device: &'static Device,
    /// Handle we send configuration to.
    pub write_path: std::ffi::CString,
    /// Handle the device answers on (equal to `write_path` on Linux/macOS).
    pub read_path: std::ffi::CString,
}

impl Found {
    /// `"2.4g"` or `"wired"` — how this endpoint is attached.
    pub fn connection(&self) -> &'static str {
        self.device.connection.label()
    }

    /// Vendor and model as one label, e.g. `"Redragon Cobra Pro (M711-PRO)"`.
    pub fn full_name(&self) -> String {
        format!("{} {}", self.device.vendor, self.device.model)
    }

    /// Whether this entry has been confirmed on real hardware.
    pub fn untested(&self) -> bool {
        self.device.support == devices::Support::Reported
    }
}

/// Resolve one table entry against what is currently plugged in.
fn locate(api: &HidApi, dev: &'static Device) -> Option<Found> {
    let mut on_config_page = None;
    let mut on_reply_page = None;
    let mut on_interface = None;

    for info in api.device_list() {
        if info.vendor_id() != dev.vid || info.product_id() != dev.pid {
            continue;
        }
        // A negative interface number means the platform did not report one;
        // in that case the usage page is the only thing that can select the
        // right collection, so such entries are only kept for the page match.
        let iface = info.interface_number();
        if iface >= 0 && iface != dev.interface {
            continue;
        }
        if info.usage_page() == dev.config_usage_page && on_config_page.is_none() {
            on_config_page = Some(info.path().to_owned());
        }
        if info.usage_page() == dev.reply_usage_page && on_reply_page.is_none() {
            on_reply_page = Some(info.path().to_owned());
        }
        if iface == dev.interface && on_interface.is_none() {
            on_interface = Some(info.path().to_owned());
        }
    }

    // Prefer an exact usage-page match (Windows); fall back to the node for the
    // whole interface (Linux/macOS), which carries every collection.
    let write_path = on_config_page.or_else(|| on_interface.clone())?;
    let read_path = on_reply_page.or_else(|| Some(write_path.clone()))?;
    Some(Found {
        device: dev,
        write_path,
        read_path,
    })
}

/// Every configuration endpoint of a device kind that is currently attached.
///
/// For a mouse that is both cabled and paired to its receiver this yields both,
/// which is what lets a write be fanned out to whichever one is awake.
pub fn find_all(api: &HidApi, kind: Kind) -> Vec<Found> {
    let mut out: Vec<Found> = Vec::new();
    for dev in devices::of_kind(kind) {
        if let Some(f) = locate(api, dev) {
            if !out.iter().any(|o| o.write_path == f.write_path) {
                out.push(f);
            }
        }
    }
    out
}

/// The endpoint to talk to for a device kind, preferring a cabled device: the
/// 2.4 GHz radio sleeps when the mouse is idle, a cable never does.
pub fn find(api: &HidApi, kind: Kind) -> Option<Found> {
    let all = find_all(api, kind);
    all.iter()
        .find(|f| f.device.connection == devices::Connection::Wired)
        .cloned()
        .or_else(|| all.into_iter().next())
}

/// Open the config endpoint of `kind`, refusing a device whose protocol the
/// caller does not implement. Every protocol module opens its devices this way,
/// so adding a second mouse protocol cannot silently route packets of one
/// dialect to a device that speaks another.
pub fn open_speaking(api: &HidApi, kind: Kind, protocol: Protocol) -> Result<Link> {
    let f = find(api, kind).ok_or(DevError::NotFound)?;
    if f.device.protocol != protocol {
        return Err(DevError::Unsupported(f.full_name()));
    }
    Link::open(api, &f)
}

/// An open link to a device, with helpers for the two packet dialects.
pub struct Link {
    dev: HidDevice,
    /// Separate handle for replies when the device answers on another
    /// collection and the platform split them (Windows).
    reply: Option<HidDevice>,
}

impl Link {
    pub fn open(api: &HidApi, found: &Found) -> Result<Link> {
        let dev = api.open_path(&found.write_path)?;
        let reply = if found.read_path == found.write_path {
            None
        } else {
            // Losing the reply handle is not fatal: writes still apply, only
            // read-back goes quiet.
            api.open_path(&found.read_path).ok()
        };
        Ok(Link { dev, reply })
    }

    /// Handle the device answers on.
    fn reply_dev(&self) -> &HidDevice {
        self.reply.as_ref().unwrap_or(&self.dev)
    }

    // ---- keyboard dialect: 64-byte output reports, reply as input report ----

    /// Write a 64-byte output report and drain the device's reply.
    pub fn kb_write(&self, buf: &[u8; 64]) -> Result<[u8; 64]> {
        self.dev.write(buf)?;
        let mut reply = [0u8; 64];
        // The device answers every write with one input report; read it so the
        // next write does not race the previous reply. A timeout is harmless.
        let _ = self.reply_dev().read_timeout(&mut reply, 150)?;
        Ok(reply)
    }

    /// Write, then read input reports until one whose byte[3] matches `cmd`
    /// arrives (or ~500 ms elapses). Used for keyboard read-back commands.
    pub fn kb_query(&self, buf: &[u8; 64], cmd: u8) -> Result<Option<[u8; 64]>> {
        self.dev.write(buf)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            let mut reply = [0u8; 64];
            let n = self.reply_dev().read_timeout(&mut reply, 120)?;
            if n > 0 && reply[0] == 0x04 && reply[3] == cmd {
                return Ok(Some(reply));
            }
        }
        Ok(None)
    }

    /// Feature transfers shared by protocols with variable-size packets.
    pub fn feature_write(&self, data: &[u8]) -> Result<()> {
        self.dev.send_feature_report(data)?;
        Ok(())
    }

    pub fn feature_read(&self, data: &mut [u8]) -> Result<usize> {
        Ok(self.dev.get_feature_report(data)?)
    }

    // ---- mouse dialect: 17-byte feature reports, reply as input report 0x09 ----

    pub fn mouse_feature(&self, buf: &[u8; 17]) -> Result<()> {
        self.dev.send_feature_report(buf)?;
        Ok(())
    }

    /// Read one input report (up to 64 bytes) with a timeout.
    pub fn mouse_read(&self, timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        let mut buf = [0u8; 64];
        let n = self.reply_dev().read_timeout(&mut buf, timeout_ms)?;
        if n == 0 {
            Ok(None)
        } else {
            Ok(Some(buf[..n].to_vec()))
        }
    }
}
