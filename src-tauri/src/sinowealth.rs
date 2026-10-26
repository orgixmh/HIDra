// SPDX-License-Identifier: GPL-3.0-or-later
//! SinoWealth feature-report lighting protocol. Model facts live in devices.rs.
//! See PROTOCOL.md for the captured identity, OEM data and query/write framing.
use crate::devices::{Protocol, SinowealthSpec};
use crate::error::{DevError, Result};
use crate::hid::{self, Kind, Link};
use crate::keyboard::{CustomCell, Effect, KbState};
use hidapi::HidApi;
use std::{thread, time::Duration};

const REPORT_LEN: usize = 520;
const CONFIG_LEN: usize = 128;
#[cfg(test)]
const PALETTE_LEN: usize = 512;
const CUSTOM_LEN: usize = 384;
const PLANE_LEN: usize = 126;

fn invalid(message: &str) -> DevError {
    DevError::Invalid(message.into())
}

pub(crate) fn packet(command: u8, arg: u8, len: usize, data: &[u8]) -> Result<[u8; REPORT_LEN]> {
    if len > 512 || data.len() > len {
        return Err(invalid("invalid feature payload length"));
    }
    let mut out = [0; REPORT_LEN];
    out[..8].copy_from_slice(&[6, command, arg, 0, 1, 0, len as u8, (len >> 8) as u8]);
    out[8..8 + data.len()].copy_from_slice(data);
    Ok(out)
}

fn decode_reply<'a>(request: &[u8; REPORT_LEN], reply: &'a [u8]) -> Result<&'a [u8]> {
    let len = u16::from_le_bytes([request[6], request[7]]) as usize;
    if reply.len() < 8 + len || reply[..8] != request[..8] {
        return Err(invalid("incomplete or mismatched keyboard reply"));
    }
    let data = &reply[8..8 + len];
    if request[1] == 0x84 && (len != CONFIG_LEN || data[126..128] != [0x5a, 0xa5]) {
        return Err(invalid(
            "keyboard config marker missing; no settings were written",
        ));
    }
    Ok(data)
}

pub(crate) fn read(link: &Link, command: u8, arg: u8, len: usize) -> Result<Vec<u8>> {
    if link.protocol == Protocol::CompxKeyboard20 {
        return crate::compx_keyboard::read(link, command, arg, len);
    }
    let request = packet(command, arg, len, &[])?;
    let mut last = invalid("keyboard did not reply");
    for _ in 0..4 {
        link.feature_write(&request)?;
        thread::sleep(Duration::from_millis(50));
        let mut reply = [0; REPORT_LEN];
        reply[0] = 6;
        let count = link.feature_read(&mut reply)?;
        match decode_reply(&request, &reply[..count]) {
            Ok(data) => return Ok(data.to_vec()),
            Err(e) => last = e,
        }
        thread::sleep(Duration::from_millis(40));
    }
    Err(last)
}

pub(crate) fn write_verified(link: &Link, command: u8, data: &[u8]) -> Result<()> {
    if link.protocol == Protocol::CompxKeyboard20 {
        return crate::compx_keyboard::write_verified(link, command, data);
    }
    // Only configuration registers. No arbitrary command or firmware interface.
    if !matches!(command, 0x04 | 0x0a | 0x06) {
        return Err(invalid("unsupported write"));
    }
    if command == 0x04 && (data.len() != CONFIG_LEN || data[126..] != [0x5a, 0xa5]) {
        return Err(invalid("invalid configuration block"));
    }
    link.feature_write(&packet(command, 0, data.len(), data)?)?;
    thread::sleep(Duration::from_millis(60));
    let actual = read(link, command | 0x80, 0, data.len())?;
    if actual != data {
        return Err(invalid(
            "keyboard did not confirm the saved settings; read it again",
        ));
    }
    Ok(())
}

pub(crate) fn open(api: &HidApi) -> Result<(Link, &'static SinowealthSpec)> {
    let found = hid::find(api, Kind::Keyboard).ok_or(DevError::NotFound)?;
    if !matches!(
        found.device.protocol,
        Protocol::Sinowealth520 | Protocol::CompxKeyboard20
    ) {
        return Err(DevError::Unsupported(found.full_name()));
    }
    let spec = found
        .device
        .sinowealth
        .ok_or_else(|| invalid("missing keyboard model data"))?;
    // Windows separates collections with the same usage page. Query each matching
    // collection instead of relying on a platform-specific ColXX path suffix.
    let mut paths = Vec::new();
    let mut last = DevError::NotFound;
    for info in api.device_list() {
        if info.vendor_id() != found.device.vid || info.product_id() != found.device.pid {
            continue;
        }
        if info.interface_number() >= 0 && info.interface_number() != found.device.interface {
            continue;
        }
        if info.usage_page() != found.device.config_usage_page {
            continue;
        }
        let path = info.path().to_owned();
        if paths.contains(&path) {
            continue;
        }
        paths.push(path.clone());
        let mut candidate = found.clone();
        candidate.write_path = path.clone();
        candidate.read_path = path;
        let result = Link::open(api, &candidate).and_then(|link| {
            let identity = read(&link, 0x82, 1, 6)?;
            if identity.as_slice() != spec.identity {
                return Err(invalid(
                    "this USB ID belongs to another keyboard model; configuration refused",
                ));
            }
            Ok(link)
        });
        match result {
            Ok(link) => return Ok((link, spec)),
            Err(e) => last = e,
        }
    }
    Err(last)
}

pub fn identify(api: &HidApi) -> bool {
    open(api).is_ok()
}

pub fn effects(spec: &SinowealthSpec) -> Vec<Effect> {
    let mut effects = vec![Effect {
        id: 0,
        name: "Off".into(),
        has_color: false,
        has_direction: false,
    }];
    effects.extend(spec.effects.iter().map(|e| Effect {
        id: e.id,
        name: e.name.into(),
        has_color: e.color,
        has_direction: false,
    }));
    effects.push(Effect {
        id: 21,
        name: "Custom".into(),
        has_color: false,
        has_direction: false,
    });
    effects
}

pub fn read_state(api: &HidApi) -> Result<KbState> {
    let (link, spec) = open(api)?;
    let config = read(&link, 0x84, 0, CONFIG_LEN)?;
    let palette = read(&link, 0x8a, 0, spec.palette_len)?;
    let effect = config[10];
    let meta = spec.effects.iter().find(|e| e.id == effect);
    let entry = meta.map(|e| e.config_offset);
    let color_slot = entry.map(|o| config[o + 1] & 15).unwrap_or(0);
    let color_off = effect as usize * 21 + color_slot.min(6) as usize * 3;
    Ok(KbState {
        present: true,
        active: None,
        effect: Some(effect),
        brightness: entry.map(|o| config[o]),
        speed: entry.map(|o| config[o + 1] >> 4),
        direction: None,
        full_rgb: Some(color_slot == 7),
        color: meta.filter(|e| e.color).map(|_| {
            crate::calibration::profile(spec).decode([
                palette[color_off],
                palette[color_off + 1],
                palette[color_off + 2],
            ])
        }),
    })
}

#[allow(clippy::too_many_arguments)]
fn update_lighting(
    spec: &SinowealthSpec,
    config: &mut [u8],
    palette: &mut [u8],
    effect: u8,
    rgb: [u8; 3],
    brightness: u8,
    speed: u8,
    full_rgb: bool,
) -> Result<bool> {
    if brightness > spec.brightness_max || speed > spec.speed_max {
        return Err(invalid("lighting value outside the model's range"));
    }
    if config.len() != CONFIG_LEN
        || config[126..] != [0x5a, 0xa5]
        || palette.len() != spec.palette_len
    {
        return Err(invalid("invalid lighting snapshot"));
    }
    let meta = if effect == 0 || effect == 21 {
        None
    } else {
        Some(
            spec.effects
                .iter()
                .find(|e| e.id == effect)
                .ok_or_else(|| invalid("unknown lighting effect"))?,
        )
    };
    config[9] = u8::from(effect == 21);
    config[10] = effect;
    if let Some(meta) = meta {
        let o = meta.config_offset;
        config[o] = brightness;
        // Preserve parameters the OEM marks inapplicable to this effect.
        if meta.speed {
            config[o + 1] = (config[o + 1] & 15) | (speed << 4);
        }
        if meta.color {
            config[o + 1] = (config[o + 1] & 0xf0) | if full_rgb { 7 } else { 0 };
            if !full_rgb {
                let color_off = effect as usize * 21;
                palette[color_off..color_off + 3].copy_from_slice(&rgb);
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[allow(clippy::too_many_arguments)]
pub fn set_effect(
    api: &HidApi,
    effect: u8,
    rgb: [u8; 3],
    brightness: u8,
    speed: u8,
    full_rgb: bool,
) -> Result<()> {
    let (link, spec) = open(api)?;
    let mut config = read(&link, 0x84, 0, CONFIG_LEN)?;
    let mut palette = read(&link, 0x8a, 0, spec.palette_len)?;
    let old_config = config.clone();
    let old_palette = palette.clone();
    let change_palette = update_lighting(
        spec,
        &mut config,
        &mut palette,
        effect,
        crate::calibration::profile(spec).apply(rgb),
        brightness,
        speed,
        full_rgb,
    )?;
    if change_palette && palette != old_palette {
        write_verified(&link, 0x0a, &palette)?;
    }
    if config != old_config {
        write_verified(&link, 0x04, &config)?;
    }
    Ok(())
}

pub fn set_custom_colors(api: &HidApi, cells: Vec<CustomCell>) -> Result<()> {
    let (link, spec) = open(api)?;
    // Validate every offset before any configuration is written.
    if cells
        .iter()
        .any(|c| !spec.keys.iter().any(|k| k.off == c.off))
    {
        return Err(invalid("unknown physical key"));
    }
    let mut config = read(&link, 0x84, 0, CONFIG_LEN)?;
    let mut colors = read(&link, 0x86, 0, CUSTOM_LEN)?;
    let calibration = crate::calibration::profile(spec);
    for cell in cells {
        let slot = cell.off as usize / 3;
        let [r, g, b] = calibration.apply([cell.r, cell.g, cell.b]);
        colors[slot] = r;
        colors[PLANE_LEN + slot] = g;
        colors[2 * PLANE_LEN + slot] = b;
    }
    write_verified(&link, 0x06, &colors)?;
    config[9] = 1;
    config[10] = 21;
    write_verified(&link, 0x04, &config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::SUPPORTED;
    fn spec() -> &'static SinowealthSpec {
        SUPPORTED.iter().find_map(|d| d.sinowealth).unwrap()
    }
    #[test]
    fn short_zero_and_wrong_command_replies_are_rejected() {
        let request = packet(0x84, 0, CONFIG_LEN, &[]).unwrap();
        assert!(decode_reply(&request, &[0; 136]).is_err());
        assert!(decode_reply(&request, &request[..135]).is_err());
        let mut valid = request;
        valid[134..136].copy_from_slice(&[0x5a, 0xa5]);
        assert_eq!(decode_reply(&request, &valid[..136]).unwrap().len(), 128);
        valid[1] = 0x8a;
        assert!(decode_reply(&request, &valid).is_err());
    }
    #[test]
    fn solid_change_preserves_every_other_setting_and_palette_slot() {
        let mut config = vec![0x24; 128];
        config[126..].copy_from_slice(&[0x5a, 0xa5]);
        let original = config.clone();
        let mut palette = vec![0x72; 512];
        update_lighting(
            spec(),
            &mut config,
            &mut palette,
            1,
            [17, 34, 51],
            3,
            4,
            false,
        )
        .unwrap();
        for i in 0..128 {
            if ![9, 10, 58, 59].contains(&i) {
                assert_eq!(config[i], original[i]);
            }
        }
        assert_eq!(&config[58..60], &[3, 0x20]); // solid has no speed parameter
        for (i, b) in palette.iter().enumerate() {
            if !(21..24).contains(&i) {
                assert_eq!(*b, 0x72);
            }
        }
        assert_eq!(&palette[21..24], &[17, 34, 51]);
    }
    #[test]
    fn invalid_effect_does_not_mutate_snapshot() {
        let mut config = vec![0; 128];
        config[126..].copy_from_slice(&[0x5a, 0xa5]);
        let original = config.clone();
        let mut palette = vec![0; 512];
        assert!(
            update_lighting(spec(), &mut config, &mut palette, 250, [0; 3], 4, 4, false).is_err()
        );
        assert_eq!(config, original);
    }
    #[test]
    fn physical_key_offsets_are_unique_and_within_each_plane() {
        let mut seen = std::collections::HashSet::new();
        for key in spec().keys {
            assert_eq!(key.off % 3, 0);
            assert!((key.off as usize / 3) < PLANE_LEN);
            assert!(seen.insert(key.off));
        }
        assert_eq!(spec().keys.iter().find(|k| k.label == "Q").unwrap().off, 24);
    }
    #[test]
    #[ignore = "requires a connected keyboard and HID permissions"]
    fn hardware_read_only() {
        let api = HidApi::new().unwrap();
        let state = read_state(&api).unwrap();
        println!("{}", serde_json::to_string(&state).unwrap());
        let (link, _) = open(&api).unwrap();
        let custom = read(&link, 0x86, 0, CUSTOM_LEN).unwrap();
        assert_eq!(custom.len(), CUSTOM_LEN);
        println!("Custom colour buffer: {} bytes", custom.len());
    }
    #[test]
    #[ignore = "temporarily changes brightness on the connected keyboard, then restores it"]
    fn hardware_brightness_roundtrip() {
        let api = HidApi::new().unwrap();
        let (link, spec) = open(&api).unwrap();
        let original = read(&link, 0x84, 0, CONFIG_LEN).unwrap();
        let original_palette = read(&link, 0x8a, 0, PALETTE_LEN).unwrap();
        let effect = spec
            .effects
            .iter()
            .find(|e| e.id == original[10])
            .expect("select a preset effect first");
        let old = original[effect.config_offset];
        let new = if old > 0 { old - 1 } else { 1 };
        let before = read_state(&api).unwrap();
        let outcome = (|| -> Result<()> {
            set_effect(
                &api,
                original[10],
                before.color.unwrap_or([0; 3]),
                new,
                before.speed.unwrap_or(0),
                before.full_rgb.unwrap_or(false),
            )?;
            let changed = read(&link, 0x84, 0, CONFIG_LEN)?;
            if changed[effect.config_offset] != new {
                return Err(invalid("brightness readback mismatch"));
            }
            println!(
                "Brightness {} -> {} confirmed by keyboard; restoring in 3 seconds",
                old, new
            );
            thread::sleep(Duration::from_secs(3));
            Ok(())
        })();
        let palette_restored =
            if read(&link, 0x8a, 0, PALETTE_LEN).ok().as_ref() != Some(&original_palette) {
                write_verified(&link, 0x0a, &original_palette)
            } else {
                Ok(())
            };
        let restored = write_verified(&link, 0x04, &original);
        palette_restored.expect("could not restore the original palette");
        restored.expect("could not restore the original configuration");
        outcome.unwrap();
        println!("Original configuration restored and verified");
    }
}
