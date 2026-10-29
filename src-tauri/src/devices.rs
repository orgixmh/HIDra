// HIDra — open-source configurator for HID gaming peripherals.
// Copyright (C) 2026 HIDra contributors.
// SPDX-License-Identifier: GPL-3.0-or-later

//! The table of devices HIDra knows how to talk to.
//!
//! **This is the one file you edit to add a device.** If your peripheral speaks
//! a [`Protocol`] that is already implemented, adding it is a single entry in
//! [`SUPPORTED`] — no other code changes. If it speaks something new, implement
//! the protocol in its own module and add a variant here.
//!
//! See `docs/ADDING-DEVICES.md` for how to find the numbers below.

use serde::Serialize;

/// What the device is, which decides the UI panel it gets.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Keyboard,
    Mouse,
}

/// The wire dialect a device speaks. Each variant is implemented by one module.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Protocol {
    /// Compx/Areson mice (M711-PRO family): 17-byte feature reports on report
    /// id 0x08, every packet checksummed to 0x55. Implemented in `mouse.rs`.
    Compx17,
    /// SONiX SN32F keyboards (K586RGB-PRO family): 64-byte reports on report id
    /// 0x04, 16-bit sum checksum. Implemented in `keyboard.rs`.
    Sonix64,
    /// 520-byte feature reports on ID 6, implemented in sinowealth.rs.
    Sinowealth520,
    /// Compx keyboard receiver: 20-byte output/input reports on ID 0x13.
    CompxKeyboard20,
}

/// How the device is attached — shown next to its name in the sidebar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Connection {
    /// Plugged in by cable.
    Wired,
    /// Reached through its own 2.4 GHz receiver (sleeps when idle).
    Receiver,
}

impl Connection {
    pub fn label(self) -> &'static str {
        match self {
            Connection::Wired => "wired",
            Connection::Receiver => "2.4g",
        }
    }
}

/// How much of this entry has actually been proven on real hardware.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Support {
    /// Someone ran HIDra against the device and everything applied.
    Verified,
    /// Added from a USB capture or a datasheet, never confirmed on hardware.
    Reported,
}

/// One USB product HIDra can configure.
///
/// A device that exposes the same product id over cable and over a receiver
/// gets one entry per id (see the M711-PRO below).
#[derive(Debug)]
pub struct Device {
    /// Brand printed on the box. Used for display only.
    pub vendor: &'static str,
    /// Marketing name plus internal model, e.g. `"Cobra Pro (M711-PRO)"`.
    pub model: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub kind: Kind,
    pub protocol: Protocol,
    /// USB interface that carries the configuration endpoint.
    pub interface: i32,
    /// HID usage page of the collection we *write* configuration to. On Windows
    /// each top-level collection is a separate handle, so this is what picks the
    /// right one; on Linux/macOS the whole interface is one node and this is
    /// only a hint.
    pub config_usage_page: u16,
    /// Usage page of the collection the device *answers* on. Same as
    /// `config_usage_page` for most devices; the Compx mice reply on a
    /// different collection than the one they are written to.
    pub reply_usage_page: u16,
    pub connection: Connection,
    pub support: Support,
    /// Model identity and layout for protocols whose USB ID is shared.
    pub sinowealth: Option<&'static SinowealthSpec>,
    pub custom_wheel: Option<&'static CustomWheelSpec>,
}

/// A Linux input capability required when locating the wheel's event device.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct InputCapability {
    pub event_type: u16,
    pub code: u16,
}

/// Input identity is separate from the vendor HID configuration endpoint.
/// Future input discovery must match all required capabilities, never eventN paths.
#[derive(Debug, Serialize)]
pub struct CustomWheelSpec {
    pub vendor_id: u16,
    pub product_id: u16,
    pub name: Option<&'static str>,
    pub bus_type: Option<u16>,
    pub required_capabilities: &'static [InputCapability],
    pub clockwise_keycode: u16,
    pub counterclockwise_keycode: u16,
    pub press_keycode: u16,
}

const K673SP_RF_WHEEL: CustomWheelSpec = CustomWheelSpec {
    vendor_id: 0x3554,
    product_id: 0xfa09,
    name: Some("CX 2.4G Wireless Receiver Consumer Control"),
    bus_type: Some(0x0003),
    required_capabilities: &[
        InputCapability {
            event_type: 0x01,
            code: 115,
        }, // EV_KEY / KEY_VOLUMEUP
        InputCapability {
            event_type: 0x01,
            code: 114,
        }, // EV_KEY / KEY_VOLUMEDOWN
        InputCapability {
            event_type: 0x01,
            code: 113,
        }, // EV_KEY / KEY_MUTE
    ],
    clockwise_keycode: 115,
    counterclockwise_keycode: 114,
    press_keycode: 113,
};

/// Every device HIDra recognises.
pub const SUPPORTED: &[Device] = &[
    // ---- Mice ----------------------------------------------------------
    Device {
        vendor: "Redragon",
        model: "Cobra Pro (M711-PRO)",
        vid: 0x25a7,
        pid: 0xfa07,
        kind: Kind::Mouse,
        protocol: Protocol::Compx17,
        interface: 1,
        config_usage_page: 0xff02,
        reply_usage_page: 0xff01,
        connection: Connection::Receiver,
        support: Support::Verified,
        sinowealth: None,
        custom_wheel: None,
    },
    Device {
        vendor: "Redragon",
        model: "Cobra Pro (M711-PRO)",
        vid: 0x25a7,
        pid: 0xfa08,
        kind: Kind::Mouse,
        protocol: Protocol::Compx17,
        interface: 1,
        config_usage_page: 0xff02,
        reply_usage_page: 0xff01,
        connection: Connection::Wired,
        support: Support::Verified,
        sinowealth: None,
        custom_wheel: None,
    },
    // ---- Keyboards -----------------------------------------------------
    Device {
        vendor: "Redragon",
        model: "Brahma Pro (K586RGB-PRO)",
        vid: 0x0c45,
        pid: 0x5004,
        kind: Kind::Keyboard,
        protocol: Protocol::Sonix64,
        interface: 1,
        config_usage_page: 0xff1c,
        reply_usage_page: 0xff1c,
        connection: Connection::Wired,
        support: Support::Verified,
        sinowealth: None,
        custom_wheel: None,
    },
    Device {
        vendor: "Redragon",
        model: "UCAL K673SP-RGB-PRO",
        vid: 0x258a,
        pid: 0x010c,
        kind: Kind::Keyboard,
        protocol: Protocol::Sinowealth520,
        interface: 1,
        config_usage_page: 0xff00,
        reply_usage_page: 0xff00,
        connection: Connection::Wired,
        support: Support::Verified,
        sinowealth: Some(&K673SP),
        custom_wheel: None,
    },
    Device {
        vendor: "Redragon",
        model: "UCAL K673SP-RGB-PRO",
        vid: 0x3554,
        pid: 0xfa09,
        kind: Kind::Keyboard,
        protocol: Protocol::CompxKeyboard20,
        interface: 1,
        config_usage_page: 0xff02,
        reply_usage_page: 0xff02,
        connection: Connection::Receiver,
        support: Support::Reported,
        sinowealth: Some(&K673SP_RF),
        custom_wheel: Some(&K673SP_RF_WHEEL),
    },
];

/// Every supported device of a kind, in table order.
pub fn of_kind(kind: Kind) -> impl Iterator<Item = &'static Device> {
    SUPPORTED.iter().filter(move |d| d.kind == kind)
}

/// Facts from the vendor's KB.ini, not guesses based on the shared USB ID.
#[derive(Debug)]
pub struct SinowealthSpec {
    pub identity: [u8; 6],
    pub palette_len: usize,
    /// Owner-supplied starting white balance; disabled until explicitly enabled.
    pub calibration_white: [u8; 3],
    pub monitor_rows: &'static [[&'static str; 10]; 3],
    /// Mode changes required before volatile RGB streaming (offset, value).
    pub stream_config: &'static [(usize, u8)],
    pub aspect: f32,
    pub brightness_max: u8,
    pub speed_max: u8,
    pub effects: &'static [SinowealthEffect],
    pub keys: &'static [KeyboardKey],
}

#[derive(Debug)]
pub struct SinowealthEffect {
    pub id: u8,
    pub name: &'static str,
    pub color: bool,
    pub speed: bool,
    pub config_offset: usize,
}

#[derive(Debug, Serialize)]
pub struct KeyboardKey {
    pub label: &'static str,
    /// RGB byte offset; divided by three for planar SinoWealth transfers.
    pub off: u16,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

// Redragon K673RGB-PRO Setup v2.0 (2024-09-10), Dev/kb/PRO-MAX/KB.ini.
// User's K673SP replies to model query 0x82 with this complete six-byte ID.
const K673SP: SinowealthSpec = SinowealthSpec {
    identity: [3, 0, 0, 0, 0, 0xba],
    palette_len: 512,
    calibration_white: [255, 153, 161],
    monitor_rows: &K673SP_MONITOR_ROWS,
    stream_config: &[(9, 1), (10, 21), (59, 0x47)],
    aspect: 2.615,
    brightness_max: 4,
    speed_max: 4,
    effects: &K673SP_EFFECTS,
    keys: &K673SP_KEYS,
};
#[rustfmt::skip]
// Identity/config/palette lengths confirmed by read-only RF queries on K673SP.
const K673SP_RF: SinowealthSpec = SinowealthSpec {
    identity: [3, 0, 0, 0, 0, 0xba],
    palette_len: 490,
    calibration_white: [255, 153, 161],
    monitor_rows: &K673SP_MONITOR_ROWS,
    stream_config: &[(9, 1), (10, 21), (59, 0x47)],
    aspect: 2.615,
    brightness_max: 4,
    speed_max: 4,
    effects: &K673SP_EFFECTS,
    keys: &K673SP_KEYS,
};

const K673SP_MONITOR_ROWS: [[&str; 10]; 3] = [
    ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
    ["A", "S", "D", "F", "G", "H", "J", "K", "L", ";"],
    ["Z", "X", "C", "V", "B", "N", "M", ",", ".", "/"],
];

const K673SP_EFFECTS: [SinowealthEffect; 18] = [
    SinowealthEffect {
        id: 1,
        name: "Fixed on",
        color: true,
        speed: false,
        config_offset: 58,
    },
    SinowealthEffect {
        id: 2,
        name: "Respire",
        color: true,
        speed: true,
        config_offset: 60,
    },
    SinowealthEffect {
        id: 3,
        name: "Rainbow",
        color: false,
        speed: true,
        config_offset: 62,
    },
    SinowealthEffect {
        id: 4,
        name: "Flash away",
        color: true,
        speed: true,
        config_offset: 64,
    },
    SinowealthEffect {
        id: 5,
        name: "Raindrops",
        color: true,
        speed: true,
        config_offset: 66,
    },
    SinowealthEffect {
        id: 6,
        name: "Rainbow wheel",
        color: true,
        speed: true,
        config_offset: 68,
    },
    SinowealthEffect {
        id: 7,
        name: "Ripples shining",
        color: true,
        speed: true,
        config_offset: 70,
    },
    SinowealthEffect {
        id: 8,
        name: "Stars twinkle",
        color: true,
        speed: true,
        config_offset: 72,
    },
    SinowealthEffect {
        id: 9,
        name: "Shadow disappear",
        color: true,
        speed: true,
        config_offset: 74,
    },
    SinowealthEffect {
        id: 10,
        name: "Retro snake",
        color: true,
        speed: true,
        config_offset: 76,
    },
    SinowealthEffect {
        id: 11,
        name: "Neon stream",
        color: true,
        speed: true,
        config_offset: 78,
    },
    SinowealthEffect {
        id: 12,
        name: "Reaction",
        color: true,
        speed: true,
        config_offset: 80,
    },
    SinowealthEffect {
        id: 13,
        name: "Sine wave",
        color: true,
        speed: true,
        config_offset: 82,
    },
    SinowealthEffect {
        id: 14,
        name: "Retinue scanning",
        color: true,
        speed: true,
        config_offset: 84,
    },
    SinowealthEffect {
        id: 15,
        name: "Rotating windmill",
        color: false,
        speed: true,
        config_offset: 86,
    },
    SinowealthEffect {
        id: 16,
        name: "Colorful waterfall",
        color: false,
        speed: true,
        config_offset: 88,
    },
    SinowealthEffect {
        id: 17,
        name: "Blossoming",
        color: false,
        speed: true,
        config_offset: 90,
    },
    SinowealthEffect {
        id: 18,
        name: "Rotating storm",
        color: true,
        speed: true,
        config_offset: 92,
    },
];
#[rustfmt::skip]
const K673SP_KEYS: [KeyboardKey; 81] = [
    KeyboardKey { label: "Esc", off: 0, x: 0.000, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F1", off: 36, x: 12.521, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F2", off: 54, x: 18.698, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F3", off: 72, x: 24.875, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F4", off: 90, x: 31.052, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F5", off: 108, x: 40.401, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F6", off: 126, x: 46.578, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F7", off: 144, x: 52.755, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F8", off: 162, x: 59.098, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F9", off: 180, x: 68.280, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F10", off: 198, x: 74.624, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F11", off: 216, x: 80.801, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F12", off: 234, x: 86.978, y: 0.000, w: 4.174, h: 12.389 },
    KeyboardKey { label: "`", off: 3, x: 0.000, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "1", off: 21, x: 6.177, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "2", off: 39, x: 12.521, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "3", off: 57, x: 18.698, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "4", off: 75, x: 24.875, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "5", off: 93, x: 31.052, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "6", off: 111, x: 37.229, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "7", off: 129, x: 43.573, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "8", off: 147, x: 49.750, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "9", off: 165, x: 55.927, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "0", off: 183, x: 62.104, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "-", off: 201, x: 68.447, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "=", off: 219, x: 74.624, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "Backspace", off: 237, x: 80.801, y: 19.027, w: 10.351, h: 12.389 },
    KeyboardKey { label: "Insert", off: 273, x: 95.826, y: 19.027, w: 4.174, h: 12.389 },
    KeyboardKey { label: "Tab", off: 6, x: 0.000, y: 35.398, w: 7.346, h: 12.389 },
    KeyboardKey { label: "Q", off: 24, x: 9.349, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "W", off: 42, x: 15.526, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "E", off: 60, x: 21.703, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "R", off: 78, x: 27.880, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "T", off: 96, x: 34.224, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "Y", off: 114, x: 40.401, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "U", off: 132, x: 46.578, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "I", off: 150, x: 52.922, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "O", off: 168, x: 59.098, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "P", off: 186, x: 65.275, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "[", off: 204, x: 71.452, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "]", off: 222, x: 77.629, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "\\", off: 240, x: 83.806, y: 35.398, w: 7.346, h: 12.389 },
    KeyboardKey { label: "Delete", off: 276, x: 95.826, y: 35.398, w: 4.174, h: 12.389 },
    KeyboardKey { label: "CapsLock", off: 9, x: 0.167, y: 52.212, w: 8.848, h: 12.389 },
    KeyboardKey { label: "A", off: 27, x: 11.018, y: 52.212, w: 4.174, h: 12.389 },
    KeyboardKey { label: "S", off: 45, x: 17.195, y: 52.212, w: 4.174, h: 12.389 },
    KeyboardKey { label: "D", off: 63, x: 23.372, y: 52.212, w: 4.174, h: 12.389 },
    KeyboardKey { label: "F", off: 81, x: 29.549, y: 52.212, w: 4.174, h: 12.389 },
    KeyboardKey { label: "G", off: 99, x: 35.726, y: 52.212, w: 4.174, h: 12.389 },
    KeyboardKey { label: "H", off: 117, x: 42.070, y: 52.212, w: 4.174, h: 12.389 },
    KeyboardKey { label: "J", off: 135, x: 48.247, y: 52.212, w: 4.174, h: 12.389 },
    KeyboardKey { label: "K", off: 153, x: 54.424, y: 52.212, w: 4.174, h: 12.389 },
    KeyboardKey { label: "L", off: 171, x: 60.601, y: 52.212, w: 4.174, h: 12.389 },
    KeyboardKey { label: ";", off: 189, x: 66.778, y: 52.212, w: 4.174, h: 12.389 },
    KeyboardKey { label: "'", off: 207, x: 73.122, y: 52.212, w: 4.174, h: 12.389 },
    KeyboardKey { label: "Enter", off: 243, x: 79.299, y: 52.212, w: 11.853, h: 12.389 },
    KeyboardKey { label: "PgUp", off: 279, x: 95.826, y: 52.212, w: 4.174, h: 12.389 },
    KeyboardKey { label: "LShift", off: 12, x: 0.167, y: 68.584, w: 11.853, h: 12.389 },
    KeyboardKey { label: "Z", off: 30, x: 14.023, y: 68.584, w: 4.174, h: 12.389 },
    KeyboardKey { label: "X", off: 48, x: 20.200, y: 68.584, w: 4.174, h: 12.389 },
    KeyboardKey { label: "C", off: 66, x: 26.377, y: 68.584, w: 4.174, h: 12.389 },
    KeyboardKey { label: "V", off: 84, x: 32.554, y: 68.584, w: 4.174, h: 12.389 },
    KeyboardKey { label: "B", off: 102, x: 38.898, y: 68.584, w: 4.174, h: 12.389 },
    KeyboardKey { label: "N", off: 120, x: 45.075, y: 68.584, w: 4.174, h: 12.389 },
    KeyboardKey { label: "M", off: 138, x: 51.252, y: 68.584, w: 4.174, h: 12.389 },
    KeyboardKey { label: ",", off: 156, x: 57.429, y: 68.584, w: 4.174, h: 12.389 },
    KeyboardKey { label: ".", off: 174, x: 63.773, y: 68.584, w: 4.174, h: 12.389 },
    KeyboardKey { label: "/", off: 192, x: 69.950, y: 68.584, w: 4.174, h: 12.389 },
    KeyboardKey { label: "RShift", off: 246, x: 76.127, y: 68.584, w: 9.015, h: 12.389 },
    KeyboardKey { label: "Up", off: 264, x: 88.314, y: 71.239, w: 4.174, h: 12.389 },
    KeyboardKey { label: "PgDn", off: 282, x: 95.826, y: 68.584, w: 4.174, h: 12.389 },
    KeyboardKey { label: "LCtrl", off: 15, x: 0.167, y: 84.956, w: 5.676, h: 12.389 },
    KeyboardKey { label: "LWin", off: 33, x: 7.846, y: 84.956, w: 5.676, h: 12.389 },
    KeyboardKey { label: "LAlt", off: 51, x: 15.693, y: 84.956, w: 5.676, h: 12.389 },
    KeyboardKey { label: "Space", off: 105, x: 23.372, y: 84.956, w: 36.728, h: 12.389 },
    KeyboardKey { label: "RAlt", off: 159, x: 62.104, y: 84.956, w: 4.174, h: 12.389 },
    KeyboardKey { label: "FN", off: 177, x: 68.280, y: 84.956, w: 4.174, h: 12.389 },
    KeyboardKey { label: "Left", off: 249, x: 81.970, y: 87.611, w: 4.174, h: 12.389 },
    KeyboardKey { label: "Down", off: 267, x: 88.314, y: 87.611, w: 4.174, h: 12.389 },
    KeyboardKey { label: "Right", off: 285, x: 94.491, y: 87.611, w: 4.174, h: 12.389 },
    KeyboardKey { label: "RCtrl", off: 195, x: 74.624, y: 84.956, w: 4.174, h: 12.389 },
];
