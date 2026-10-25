// SPDX-License-Identifier: GPL-3.0-or-later
//! Dispatch keyboard commands by protocol; the Sonix implementation stays intact.
use crate::devices::{KeyboardKey, Protocol};
use crate::error::{DevError, Result};
use crate::hid::{self, Kind};
use crate::{keyboard, sinowealth};
use hidapi::HidApi;
use serde::Serialize;

pub use keyboard::{
    encode_remap, remap, restore_keymap, set_active_profile, set_brightness, set_color,
    set_direction, set_full_rgb, set_mode, set_speed,
};

#[derive(Serialize)]
pub struct Capabilities {
    pub profiles: u8,
    pub remap: bool,
    pub per_key: bool,
    pub brightness_max: u8,
    pub speed_min: u8,
    pub speed_max: u8,
    pub custom_effect: u8,
    pub layout_aspect: Option<f32>,
    pub layout: Option<&'static [KeyboardKey]>,
}

pub fn capabilities(api: &HidApi) -> Option<Capabilities> {
    let found = hid::find(api, Kind::Keyboard)?;
    if let Some(spec) = found.device.sinowealth {
        Some(Capabilities {
            profiles: 1,
            remap: false,
            per_key: true,
            brightness_max: spec.brightness_max,
            speed_min: 0,
            speed_max: spec.speed_max,
            custom_effect: 21,
            layout: Some(spec.keys),
            layout_aspect: Some(spec.aspect),
        })
    } else {
        Some(Capabilities {
            profiles: 3,
            remap: true,
            per_key: true,
            brightness_max: 5,
            speed_min: 1,
            speed_max: 5,
            custom_effect: 20,
            layout: None,
            layout_aspect: None,
        })
    }
}

pub fn find(api: &HidApi) -> Option<hid::Found> {
    let found = hid::find(api, Kind::Keyboard)?;
    if found.device.protocol == Protocol::Sinowealth520 && !sinowealth::identify(api) {
        return None;
    }
    Some(found)
}

pub fn read_state(api: &HidApi, profile: u8) -> Result<keyboard::KbState> {
    match hid::find(api, Kind::Keyboard).map(|f| f.device.protocol) {
        Some(Protocol::Sinowealth520) => sinowealth::read_state(api),
        _ => Ok(keyboard::read_state(api, profile)),
    }
}

pub fn effects(api: &HidApi) -> Vec<keyboard::Effect> {
    if let Some(spec) = hid::find(api, Kind::Keyboard).and_then(|f| f.device.sinowealth) {
        sinowealth::effects(spec)
    } else {
        keyboard::effects()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn set_effect(
    api: &HidApi,
    profile: u8,
    effect: u8,
    r: u8,
    g: u8,
    b: u8,
    brightness: u8,
    speed: u8,
    direction: u8,
    full_rgb: bool,
) -> Result<()> {
    match hid::find(api, Kind::Keyboard).map(|f| f.device.protocol) {
        Some(Protocol::Sinowealth520) => {
            if profile != 1 || direction != 0 {
                return Err(DevError::Invalid("unsupported profile or direction".into()));
            }
            sinowealth::set_effect(api, effect, [r, g, b], brightness, speed, full_rgb)
        }
        _ => keyboard::set_effect(
            api, profile, effect, r, g, b, brightness, speed, direction, full_rgb,
        ),
    }
}

pub fn set_custom_colors(
    api: &HidApi,
    profile: u8,
    cells: Vec<keyboard::CustomCell>,
) -> Result<()> {
    match hid::find(api, Kind::Keyboard).map(|f| f.device.protocol) {
        Some(Protocol::Sinowealth520) => {
            if profile != 1 {
                return Err(DevError::Invalid(
                    "this keyboard has one lighting configuration".into(),
                ));
            }
            sinowealth::set_custom_colors(api, cells)
        }
        _ => keyboard::set_custom_colors(api, profile, cells),
    }
}

pub fn remap_keys(api: &HidApi) -> Result<Vec<keyboard::RemapKey>> {
    if hid::find(api, Kind::Keyboard).is_some_and(|f| f.device.protocol != Protocol::Sonix64) {
        return Err(DevError::Invalid(
            "key remapping is not implemented for this keyboard".into(),
        ));
    }
    Ok(keyboard::remap_keys())
}
