// SPDX-License-Identifier: GPL-3.0-or-later
//! Host-side white balance. Saved keyboard blocks and recovery remain raw.
use crate::devices::SinowealthSpec;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Write, path::PathBuf, sync::OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Profile {
    pub enabled: bool,
    pub white: [u8; 3],
}
impl Profile {
    pub fn apply(self, rgb: [u8; 3]) -> [u8; 3] {
        if !self.enabled {
            return rgb;
        }
        std::array::from_fn(|i| ((rgb[i] as u32 * self.white[i] as u32 + 127) / 255) as u8)
    }
    pub fn decode(self, rgb: [u8; 3]) -> [u8; 3] {
        if !self.enabled {
            return rgb;
        }
        std::array::from_fn(|i| {
            ((rgb[i] as u32 * 255 + self.white[i] as u32 / 2) / self.white[i].max(1) as u32)
                .min(255) as u8
        })
    }
}
#[derive(Serialize)]
pub struct View {
    pub profile: Profile,
    pub suggested_white: [u8; 3],
    pub error: Option<String>,
}
struct Store {
    path: PathBuf,
    profiles: BTreeMap<String, Profile>,
    error: Option<String>,
}
static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
fn key(spec: &SinowealthSpec) -> String {
    spec.identity.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn init(directory: PathBuf) {
    let path = directory.join("keyboard-calibration.json");
    let loaded = match fs::read(&path) {
        Ok(bytes) => {
            serde_json::from_slice::<BTreeMap<String, Profile>>(&bytes).map_err(|e| e.to_string())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(e.to_string()),
    }
    .and_then(|profiles| {
        if profiles.values().any(|p| p.white.contains(&0)) {
            Err("Calibration channels must be between 1 and 255".into())
        } else {
            Ok(profiles)
        }
    });
    let (profiles, error) = match loaded {
        Ok(profiles) => (profiles, None),
        Err(error) => {
            log::warn!("Could not load keyboard calibration: {error}");
            (BTreeMap::new(), Some(error))
        }
    };
    let _ = STORE.set(Mutex::new(Store {
        path,
        profiles,
        error,
    }));
}
pub fn profile(spec: &SinowealthSpec) -> Profile {
    STORE
        .get()
        .and_then(|store| store.lock().profiles.get(&key(spec)).copied())
        .unwrap_or(Profile {
            enabled: false,
            white: spec.calibration_white,
        })
}
pub fn get(spec: &SinowealthSpec) -> View {
    View {
        profile: profile(spec),
        suggested_white: spec.calibration_white,
        error: STORE.get().and_then(|store| store.lock().error.clone()),
    }
}
pub fn save(spec: &SinowealthSpec, profile: Profile) -> Result<View, String> {
    if profile.white.contains(&0) {
        return Err("Calibration channels must be between 1 and 255".into());
    }
    let mut store = STORE
        .get()
        .ok_or("Calibration settings are not ready")?
        .lock();
    let mut profiles = store.profiles.clone();
    profiles.insert(key(spec), profile);
    let parent = store
        .path
        .parent()
        .ok_or("Invalid calibration settings path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = store.path.with_extension("tmp");
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
    fs::rename(temporary, &store.path).map_err(|e| e.to_string())?;
    store.profiles = profiles;
    store.error = None;
    Ok(View {
        profile,
        suggested_white: spec.calibration_white,
        error: None,
    })
}
