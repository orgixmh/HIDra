// SPDX-License-Identifier: GPL-3.0-or-later
//! Linux login startup uses the user's XDG autostart directory, without a daemon.
use serde::Serialize;
#[cfg(target_os = "linux")]
use std::{fs, io::Write, path::PathBuf};
use tauri::AppHandle;
#[derive(Serialize)]
pub struct Status {
    pub enabled: bool,
    pub supported: bool,
    pub reason: Option<String>,
}
#[cfg(target_os = "linux")]
fn path(app: &AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    app.path()
        .config_dir()
        .map(|path| path.join("autostart/io.github.luizjr.hidra.desktop"))
        .map_err(|error| error.to_string())
}
#[cfg(target_os = "linux")]
pub fn status(app: &AppHandle) -> Result<Status, String> {
    let text = match fs::read_to_string(path(app)?) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.to_string()),
    };
    let enabled = text.lines().any(|line| line.trim().starts_with("Exec="))
        && !text.lines().any(|line| {
            matches!(
                line.trim(),
                "Hidden=true" | "X-GNOME-Autostart-enabled=false"
            )
        });
    Ok(Status {
        enabled,
        supported: !tauri::is_dev(),
        reason: tauri::is_dev().then(|| {
            "Automatic startup requires a standalone HIDra build, not the development server."
                .into()
        }),
    })
}
#[cfg(target_os = "linux")]
pub fn set(app: &AppHandle, enabled: bool) -> Result<Status, String> {
    let destination = path(app)?;
    if !enabled {
        match fs::remove_file(&destination) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        return status(app);
    }
    if tauri::is_dev() {
        return Err(
            "Automatic startup requires a standalone HIDra build, not the development server."
                .into(),
        );
    }
    let executable = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(std::env::current_exe)
        .map_err(|error| error.to_string())?;
    let path = executable
        .to_str()
        .ok_or("Application path cannot be used for automatic startup")?;
    if !executable.is_absolute() || !executable.is_file() || path.contains(['\n', '\r', '\0', '='])
    {
        return Err("Application path cannot be used for automatic startup".into());
    }
    // First escape the quoted Exec argument, then the desktop string layer.
    let quoted: String = path
        .chars()
        .flat_map(|c| {
            if matches!(c, '\\' | '"' | '`' | '$') {
                vec!['\\', c]
            } else {
                vec![c]
            }
        })
        .collect();
    let quoted = quoted.replace('\\', "\\\\").replace('%', "%%");
    let text = format!("[Desktop Entry]\nType=Application\nName=HIDra\nComment=Start HIDra in the system tray\nExec=\"{quoted}\" --tray\nTerminal=false\nStartupNotify=false\nX-GNOME-Autostart-enabled=true\nX-GNOME-Autostart-Delay=5\nHidden=false\n");
    let directory = destination.parent().ok_or("Invalid startup directory")?;
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let temporary = destination.with_extension("desktop.tmp");
    let mut file = fs::File::create(&temporary).map_err(|error| error.to_string())?;
    file.write_all(text.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|error| error.to_string())?;
    fs::rename(&temporary, &destination).map_err(|error| error.to_string())?;
    status(app)
}
#[cfg(not(target_os = "linux"))]
pub fn status(_app: &AppHandle) -> Result<Status, String> {
    Ok(Status {
        enabled: false,
        supported: false,
        reason: Some("Automatic startup is currently available on Linux.".into()),
    })
}
#[cfg(not(target_os = "linux"))]
pub fn set(app: &AppHandle, _enabled: bool) -> Result<Status, String> {
    status(app)
}
