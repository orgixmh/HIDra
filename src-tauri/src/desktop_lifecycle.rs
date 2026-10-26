// SPDX-License-Identifier: GPL-3.0-or-later
//! Keep the webview and notification workers alive when the window is hidden.
use crate::smart_notifications::SmartNotifications;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Window, WindowEvent, Wry,
};

const CLOSE_EVENT: &str = "hidra-close-requested";

#[derive(Clone, Serialize)]
pub struct Status {
    pub tray_available: bool,
    pub close_requested: bool,
    pub error: Option<String>,
}
pub struct DesktopLifecycle {
    ready: bool,
    error: Option<String>,
    pending: AtomicBool,
    exiting: AtomicBool,
    items: Option<(MenuItem<Wry>, MenuItem<Wry>)>,
}

impl DesktopLifecycle {
    pub fn setup(app: &AppHandle) -> Self {
        let result = (|| -> tauri::Result<_> {
            let show = MenuItem::with_id(app, "hidra-show", "Show HIDra", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "hidra-exit", "Exit HIDra", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&show, &separator, &exit])?;
            let mut tray = TrayIconBuilder::with_id("hidra-tray")
                .tooltip("HIDra")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "hidra-show" => show_window(app),
                    "hidra-exit" => exit_app(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_window(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            // Tauri retains the registered tray in the application's tray manager.
            tray.build(app)?;
            Ok((show, exit))
        })();
        match result {
            Ok(items) => Self {
                ready: true,
                error: None,
                pending: AtomicBool::new(false),
                exiting: AtomicBool::new(false),
                items: Some(items),
            },
            Err(error) => Self {
                ready: false,
                error: Some(error.to_string()),
                pending: AtomicBool::new(false),
                exiting: AtomicBool::new(false),
                items: None,
            },
        }
    }
    pub fn status(&self) -> Status {
        let available = self.ready && tray_host_available();
        Status {
            tray_available: available,
            close_requested: self.pending.load(Ordering::Acquire),
            error: if available {
                None
            } else {
                Some(self.error.clone().unwrap_or_else(|| "The top bar icon is unavailable. Keep this window open to continue Smart Notifications.".into()))
            },
        }
    }
    pub fn language(&self, language: &str) -> Result<(), String> {
        if let Some((show, exit)) = &self.items {
            let (show_label, exit_label) = if language == "pt-BR" {
                ("Mostrar HIDra", "Sair do HIDra")
            } else {
                ("Show HIDra", "Exit HIDra")
            };
            show.set_text(show_label).map_err(|e| e.to_string())?;
            exit.set_text(exit_label).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn close(&self, app: &AppHandle, choice: &str) -> Result<(), String> {
        match choice {
            "background" => {
                if !self.status().tray_available {
                    return Err("The top bar icon is unavailable. Keep this window open to continue Smart Notifications.".into());
                }
                let window = app
                    .get_webview_window("main")
                    .ok_or("HIDra window is unavailable")?;
                window.hide().map_err(|e| e.to_string())?;
                self.pending.store(false, Ordering::Release);
            }
            "exit" => exit_app(app),
            "cancel" => self.pending.store(false, Ordering::Release),
            _ => return Err("Unknown close action".into()),
        }
        Ok(())
    }
}

pub fn startup(app: &AppHandle) {
    let tray_mode = std::env::args_os()
        .skip(1)
        .any(|arg| arg == "--tray" || arg == "--tray-mode");
    if !tray_mode {
        show_window(app);
        return;
    }
    // All windows start hidden in config, preventing a startup flash. At login,
    // the desktop's tray host may arrive after this application, so allow it time.
    let app = app.clone();
    std::thread::spawn(move || {
        for _ in 0..20 {
            if !app.state::<DesktopLifecycle>().ready {
                show_window(&app);
                return;
            }
            if tray_host_available() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        // Never leave the user with an inaccessible hidden application.
        show_window(&app);
    });
}

pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    if let WindowEvent::CloseRequested { api, .. } = event {
        let app = window.app_handle();
        let Some(shell) = app.try_state::<DesktopLifecycle>() else {
            return;
        };
        if shell.exiting.load(Ordering::Acquire) {
            return;
        }
        let protected = app.try_state::<SmartNotifications>().is_some_and(|smart| {
            let view = smart.get();
            view.settings.enabled || view.active
        });
        if protected {
            api.prevent_close();
            shell.pending.store(true, Ordering::Release);
            show_window(app);
            let _ = app.emit(CLOSE_EVENT, shell.status());
        }
    }
}
fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
fn exit_app(app: &AppHandle) {
    if let Some(shell) = app.try_state::<DesktopLifecycle>() {
        shell.pending.store(false, Ordering::Release);
        shell.exiting.store(true, Ordering::Release);
    }
    // ExitRequested performs the normal notification shutdown and lighting restore.
    app.exit(0);
}

#[cfg(target_os = "linux")]
fn tray_host_available() -> bool {
    use dbus::blocking::{stdintf::org_freedesktop_dbus::Properties, Connection};
    let Ok(connection) = Connection::new_session() else {
        return false;
    };
    let proxy = connection.with_proxy(
        "org.kde.StatusNotifierWatcher",
        "/StatusNotifierWatcher",
        std::time::Duration::from_millis(500),
    );
    proxy
        .get::<bool>(
            "org.kde.StatusNotifierWatcher",
            "IsStatusNotifierHostRegistered",
        )
        .unwrap_or(false)
}
#[cfg(not(target_os = "linux"))]
fn tray_host_available() -> bool {
    true
}
