// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only host statistics, sampled independently of the HID transport.
use crate::devices::SinowealthSpec;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Meter {
    pub enabled: bool,
    pub color: [u8; 3],
    pub bar: bool,
}
impl Default for Meter {
    fn default() -> Self {
        Self {
            enabled: false,
            color: [0, 190, 255],
            bar: false,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub cpu: Meter,
    pub ram: Meter,
    pub disk: Meter,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            cpu: Meter::default(),
            ram: Meter {
                color: [150, 80, 255],
                ..Meter::default()
            },
            disk: Meter {
                color: [255, 140, 0],
                ..Meter::default()
            },
        }
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Reading {
    pub percent: Option<f64>,
    pub error: Option<String>,
}
impl Reading {
    fn from_result(value: Result<f64, String>) -> Self {
        match value {
            Ok(p) => Self {
                percent: Some(p.clamp(0.0, 100.0)),
                error: None,
            },
            Err(error) => Self {
                percent: None,
                error: Some(error),
            },
        }
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Sample {
    pub cpu: Reading,
    pub ram: Reading,
    pub disk: Reading,
}
#[derive(Default)]
pub struct Sampler {
    last: Option<Instant>,
    previous_cpu: Option<(u64, u64)>,
    flags: [bool; 3],
}
impl Sampler {
    pub fn sample(&mut self, config: &Config, now: Instant) -> Option<Sample> {
        let flags = [config.cpu.enabled, config.ram.enabled, config.disk.enabled];
        let changed = flags != self.flags;
        if changed {
            if flags[0] != self.flags[0] {
                self.previous_cpu = None;
            }
            self.last = None;
            self.flags = flags;
        }
        if !flags.iter().any(|on| *on) {
            return changed.then(Sample::default);
        }
        if self
            .last
            .is_some_and(|last| now.duration_since(last) < Duration::from_secs(1))
        {
            return None;
        }
        self.last = Some(now);
        Some(Sample {
            cpu: if flags[0] {
                self.cpu()
            } else {
                Reading::default()
            },
            ram: if flags[1] {
                Reading::from_result(ram_percent())
            } else {
                Reading::default()
            },
            disk: if flags[2] {
                Reading::from_result(disk_percent())
            } else {
                Reading::default()
            },
        })
    }
    fn cpu(&mut self) -> Reading {
        match cpu_counters() {
            Ok(current) => {
                let previous = self.previous_cpu.replace(current);
                match previous {
                    Some((total, idle)) if current.0 > total && current.1 >= idle => {
                        let elapsed = current.0 - total;
                        let idle = (current.1 - idle).min(elapsed);
                        Reading::from_result(Ok(100.0 * (elapsed - idle) as f64 / elapsed as f64))
                    }
                    _ => Reading::default(), // Two samples are required; no boot-time average.
                }
            }
            Err(error) => {
                self.previous_cpu = None;
                Reading {
                    percent: None,
                    error: Some(error),
                }
            }
        }
    }
}

/// A moving LED or a filled bar per configured row: 0–4% dark, 5–15% first, 16–25% second,
/// ... 96–100% tenth. Monitoring sits below notification alerts in composition.
pub fn cells(
    spec: &SinowealthSpec,
    config: &Config,
    sample: &Sample,
    visible: bool,
) -> Vec<(u16, [u8; 3])> {
    let mut cells = Vec::new();
    for ((meter, reading), row) in [
        (&config.cpu, &sample.cpu),
        (&config.ram, &sample.ram),
        (&config.disk, &sample.disk),
    ]
    .into_iter()
    .zip(spec.monitor_rows)
    {
        if !meter.enabled {
            continue;
        }
        let Some(percent) = reading.percent else {
            continue;
        };
        let percent = percent.clamp(0.0, 100.0);
        let rounded = percent.round() as usize;
        if rounded < 5 || (percent > 90.0 && !visible) {
            continue;
        }
        let index = (rounded.saturating_sub(6) / 10).min(row.len() - 1);
        let first = if meter.bar { 0 } else { index };
        for label in &row[first..=index] {
            if let Some(key) = spec.keys.iter().find(|key| key.label == *label) {
                cells.push((key.off, meter.color));
            }
        }
    }
    cells
}

#[cfg(target_os = "linux")]
fn cpu_counters() -> Result<(u64, u64), String> {
    let text = std::fs::read_to_string("/proc/stat").map_err(|e| e.to_string())?;
    let line = text
        .lines()
        .find(|line| line.starts_with("cpu "))
        .ok_or("CPU counters are unavailable")?;
    // guest/guest_nice are already included in user/nice; sum only the first eight.
    let counters: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .take(8)
        .map(|v| v.parse::<u64>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    if counters.len() < 4 {
        return Err("CPU counters are incomplete".into());
    }
    Ok((
        counters.iter().sum(),
        counters[3] + counters.get(4).copied().unwrap_or(0),
    ))
}
#[cfg(target_os = "linux")]
fn ram_percent() -> Result<f64, String> {
    let text = std::fs::read_to_string("/proc/meminfo").map_err(|e| e.to_string())?;
    let value = |name: &str| {
        text.lines().find_map(|line| {
            let (key, rest) = line.split_once(':')?;
            (key == name)
                .then(|| rest.split_whitespace().next()?.parse::<u64>().ok())
                .flatten()
        })
    };
    let total = value("MemTotal")
        .filter(|v| *v > 0)
        .ok_or("RAM total is unavailable")?;
    let available = value("MemAvailable")
        .ok_or("Available RAM is unavailable")?
        .min(total);
    Ok(100.0 * (total - available) as f64 / total as f64)
}
#[cfg(target_os = "linux")]
fn disk_percent() -> Result<f64, String> {
    let mut info = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: the NUL-terminated root path is valid; statvfs initializes `info` on success.
    if unsafe { libc::statvfs(c"/".as_ptr(), info.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    // SAFETY: the successful call initialized every statvfs field.
    let info = unsafe { info.assume_init() };
    let used = info.f_blocks.saturating_sub(info.f_bfree) as f64;
    let usable = used + info.f_bavail as f64; // Match df's user-available-space denominator.
    if usable == 0.0 {
        return Err("Disk capacity is unavailable".into());
    }
    Ok(100.0 * used / usable)
}
#[cfg(not(target_os = "linux"))]
fn cpu_counters() -> Result<(u64, u64), String> {
    Err("System monitoring is available on Linux.".into())
}
#[cfg(not(target_os = "linux"))]
fn ram_percent() -> Result<f64, String> {
    Err("System monitoring is available on Linux.".into())
}
#[cfg(not(target_os = "linux"))]
fn disk_percent() -> Result<f64, String> {
    Err("System monitoring is available on Linux.".into())
}
