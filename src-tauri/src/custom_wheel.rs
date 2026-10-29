// SPDX-License-Identifier: GPL-3.0-or-later
//! Phase 1: capability exposure and host preferences only. No input/HID I/O.
use crate::devices::{CustomWheelSpec, Device, SUPPORTED};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Write, path::PathBuf};

#[derive(Serialize)]
pub struct Capability {
    pub settings_key: String,
    pub input: &'static CustomWheelSpec,
}
fn key(device: &Device) -> String {
    format!(
        "{}/{}/{:04x}:{:04x}/{}/{}",
        device.vendor,
        device.model,
        device.vid,
        device.pid,
        device.interface,
        device.connection.label()
    )
}
pub fn capability(device: &Device) -> Option<Capability> {
    Some(Capability {
        settings_key: key(device),
        input: device.custom_wheel?,
    })
}
fn supported(device_key: &str) -> Result<(), String> {
    if SUPPORTED
        .iter()
        .any(|device| device.custom_wheel.is_some() && key(device) == device_key)
    {
        Ok(())
    } else {
        Err("Custom Wheel is unavailable for this device definition".into())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Volume,
    Brightness,
    Zoom,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    pub enabled: bool,
    /// Ordered cycle; Volume is mandatory so long press always has a fallback.
    pub modes: Vec<Mode>,
    pub show_osd: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            enabled: false,
            modes: vec![Mode::Volume, Mode::Brightness, Mode::Zoom],
            show_osd: true,
        }
    }
}
impl Preferences {
    fn validate(&self) -> Result<(), String> {
        if !self.modes.contains(&Mode::Volume)
            || self.modes.len() > 3
            || self
                .modes
                .iter()
                .enumerate()
                .any(|(i, mode)| self.modes[..i].contains(mode))
        {
            return Err("Wheel modes must include Volume and contain no duplicates".into());
        }
        Ok(())
    }
}
#[derive(Serialize)]
pub struct View {
    pub preferences: Preferences,
    pub error: Option<String>,
}
struct Saved {
    profiles: BTreeMap<String, Preferences>,
    error: Option<String>,
}
pub struct Store {
    path: PathBuf,
    saved: Mutex<Saved>,
}
impl Store {
    pub fn load(directory: PathBuf) -> Self {
        let path = directory.join("custom-wheel.json");
        let loaded = (|| -> Result<BTreeMap<String, Preferences>, String> {
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
                Err(e) => return Err(e.to_string()),
            };
            if bytes.len() > 1024 * 1024 {
                return Err("Custom Wheel settings file is too large".into());
            }
            let profiles: BTreeMap<String, Preferences> =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            for preferences in profiles.values() {
                preferences.validate()?;
            }
            Ok(profiles)
        })();
        let (profiles, error) = match loaded {
            Ok(profiles) => (profiles, None),
            Err(error) => {
                log::warn!("Could not load Custom Wheel preferences: {error}");
                (BTreeMap::new(), Some(error))
            }
        };
        Self {
            path,
            saved: Mutex::new(Saved { profiles, error }),
        }
    }
    pub fn get(&self, device_key: &str) -> Result<View, String> {
        supported(device_key)?;
        let saved = self.saved.lock();
        Ok(View {
            preferences: saved.profiles.get(device_key).cloned().unwrap_or_default(),
            error: saved.error.clone(),
        })
    }
    pub fn save(&self, device_key: &str, preferences: Preferences) -> Result<View, String> {
        supported(device_key)?;
        preferences.validate()?;
        let mut saved = self.saved.lock();
        let mut profiles = saved.profiles.clone();
        profiles.insert(device_key.to_owned(), preferences.clone());
        let parent = self
            .path
            .parent()
            .ok_or("Invalid Custom Wheel settings path")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temporary = self.path.with_extension("tmp");
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(&profiles).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(temporary, &self.path).map_err(|e| e.to_string())?;
        saved.profiles = profiles;
        saved.error = None;
        Ok(View {
            preferences,
            error: None,
        })
    }
}
