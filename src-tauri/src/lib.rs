// HIDra — open-source configurator for HID gaming peripherals.
// Copyright (C) 2026 HIDra contributors.
// SPDX-License-Identifier: GPL-3.0-or-later

mod desktop_notifications;
mod devices;
mod error;
mod hid;
mod keyboard;
mod keyboard_driver;
mod mouse;
mod notification_lighting;
mod sinowealth;
mod smart_notifications;
mod update;

use error::Result;
use hidapi::HidApi;
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use tauri::{Manager, State};

/// Shared HID context. Re-enumerated before every operation so hot-plug and the
/// wireless mouse waking up are picked up without restarting the app.
pub struct App {
    api: Mutex<HidApi>,
    keyboard_gate: Arc<Mutex<bool>>,
}

fn with_api<T>(app: &App, f: impl FnOnce(&HidApi) -> T) -> T {
    let mut guard = app.api.lock();
    let _ = guard.refresh_devices();
    f(&guard)
}

fn with_keyboard_api<T>(app: &App, f: impl FnOnce(&HidApi) -> Result<T>) -> Result<T> {
    let owned = app.keyboard_gate.lock();
    if *owned {
        return Err(error::DevError::Invalid(
            "Smart Notifications controls the lighting. Turn it off to use manual controls.".into(),
        ));
    }
    with_api(app, f)
}

#[tauri::command]
fn smart_notifications_get(
    smart: State<smart_notifications::SmartNotifications>,
) -> smart_notifications::View {
    smart.get()
}

#[tauri::command]
fn smart_notifications_save(
    smart: State<smart_notifications::SmartNotifications>,
    settings: smart_notifications::Settings,
) -> std::result::Result<smart_notifications::View, String> {
    smart.save(settings)
}

#[tauri::command]
fn smart_notifications_test(
    smart: State<smart_notifications::SmartNotifications>,
    id: String,
) -> std::result::Result<smart_notifications::View, String> {
    smart.test(id)
}

#[derive(Serialize)]
struct Presence {
    keyboard: bool,
    keyboard_capabilities: Option<keyboard_driver::Capabilities>,
    mouse: bool,
    mouse_mode: Option<String>,
    /// What was actually found, straight from the device table, so the UI names
    /// the hardware instead of hardcoding one model.
    keyboard_vendor: Option<String>,
    keyboard_model: Option<String>,
    mouse_vendor: Option<String>,
    mouse_model: Option<String>,
    /// True when the model was added to the table from a capture but never
    /// confirmed on hardware — the UI warns about it.
    keyboard_untested: bool,
    mouse_untested: bool,
}

#[tauri::command]
fn list_devices(app: State<App>) -> Presence {
    let owned = app.keyboard_gate.lock();
    with_api(&app, |api| {
        let kb = if *owned {
            hid::find(api, hid::Kind::Keyboard)
        } else {
            keyboard_driver::find(api)
        };
        let m = hid::find(api, hid::Kind::Mouse);
        Presence {
            keyboard: kb.is_some(),
            keyboard_capabilities: keyboard_driver::capabilities(api),
            mouse: m.is_some(),
            mouse_mode: m.as_ref().map(|f| f.connection().to_string()),
            keyboard_untested: kb.as_ref().is_some_and(|f| f.untested()),
            mouse_untested: m.as_ref().is_some_and(|f| f.untested()),
            keyboard_vendor: kb.as_ref().map(|f| f.device.vendor.to_string()),
            keyboard_model: kb.map(|f| f.device.model.to_string()),
            mouse_vendor: m.as_ref().map(|f| f.device.vendor.to_string()),
            mouse_model: m.map(|f| f.device.model.to_string()),
        }
    })
}

// ---- Mouse commands ----------------------------------------------------

#[tauri::command]
fn mouse_read_state(app: State<App>) -> mouse::MouseState {
    with_api(&app, mouse::read_state)
}

#[tauri::command]
fn mouse_set_led(
    app: State<App>,
    effect: String,
    r: u8,
    g: u8,
    b: u8,
    speed: u8,
    brightness: u8,
) -> Result<()> {
    with_api(&app, |api| {
        mouse::set_led(api, &effect, r, g, b, speed, brightness)
    })
}

#[tauri::command]
fn mouse_set_dpi_stage(app: State<App>, stage: usize, x: u32, y: u32) -> Result<()> {
    with_api(&app, |api| mouse::set_dpi_stage(api, stage, x, y))
}

#[tauri::command]
fn mouse_set_dpi_all(app: State<App>, dpi: u32) -> Result<()> {
    with_api(&app, |api| mouse::set_dpi_all(api, dpi))
}

#[tauri::command]
fn mouse_set_dpi_color(app: State<App>, stage: usize, r: u8, g: u8, b: u8) -> Result<()> {
    with_api(&app, |api| mouse::set_dpi_color(api, stage, r, g, b))
}

#[tauri::command]
fn mouse_set_dpi_effect(app: State<App>, on: bool) -> Result<()> {
    with_api(&app, |api| mouse::set_dpi_effect(api, on))
}

#[tauri::command]
fn mouse_set_polling(app: State<App>, hz: u32) -> Result<()> {
    with_api(&app, |api| mouse::set_polling(api, hz))
}

#[tauri::command]
fn mouse_set_button(app: State<App>, index: usize, action: mouse::Action) -> Result<()> {
    with_api(&app, |api| mouse::set_button(api, index, action))
}

// ---- Keyboard commands -------------------------------------------------

#[tauri::command]
fn keyboard_read_state(app: State<App>, profile: u8) -> Result<keyboard::KbState> {
    with_keyboard_api(&app, |api| keyboard_driver::read_state(api, profile))
}

#[tauri::command]
fn keyboard_effects(app: State<App>) -> Vec<keyboard::Effect> {
    with_api(&app, keyboard_driver::effects)
}

// The JS side sends snake_case keys (e.g. `full_rgb`); tell Tauri to expect
// them as-is instead of the default camelCase conversion.
#[tauri::command(rename_all = "snake_case")]
#[allow(clippy::too_many_arguments)]
fn keyboard_set_effect(
    app: State<App>,
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
    with_keyboard_api(&app, |api| {
        keyboard_driver::set_effect(
            api, profile, effect, r, g, b, brightness, speed, direction, full_rgb,
        )
    })
}

#[tauri::command]
fn keyboard_set_brightness(app: State<App>, profile: u8, value: u8) -> Result<()> {
    with_keyboard_api(&app, |api| {
        keyboard_driver::set_brightness(api, profile, value)
    })
}
#[tauri::command]
fn keyboard_set_speed(app: State<App>, profile: u8, value: u8) -> Result<()> {
    with_keyboard_api(&app, |api| keyboard_driver::set_speed(api, profile, value))
}
#[tauri::command]
fn keyboard_set_direction(app: State<App>, profile: u8, left: bool) -> Result<()> {
    with_keyboard_api(&app, |api| {
        keyboard_driver::set_direction(api, profile, left)
    })
}
#[tauri::command]
fn keyboard_set_full_rgb(app: State<App>, profile: u8, on: bool) -> Result<()> {
    with_keyboard_api(&app, |api| keyboard_driver::set_full_rgb(api, profile, on))
}
#[tauri::command]
fn keyboard_set_color(app: State<App>, profile: u8, r: u8, g: u8, b: u8) -> Result<()> {
    with_keyboard_api(&app, |api| {
        keyboard_driver::set_color(api, profile, r, g, b)
    })
}
#[tauri::command]
fn keyboard_set_mode(app: State<App>, profile: u8, effect: u8) -> Result<()> {
    with_keyboard_api(&app, |api| keyboard_driver::set_mode(api, profile, effect))
}
#[tauri::command]
fn keyboard_set_active_profile(app: State<App>, profile: u8) -> Result<()> {
    with_keyboard_api(&app, |api| {
        keyboard_driver::set_active_profile(api, profile)
    })
}

#[tauri::command]
fn keyboard_remap_keys(app: State<App>) -> Result<Vec<keyboard::RemapKey>> {
    with_api(&app, keyboard_driver::remap_keys)
}

#[tauri::command]
fn keyboard_remap(app: State<App>, index: usize, kind: String, code: u8, code2: u8) -> Result<()> {
    let opt = keyboard_driver::encode_remap(&kind, code, code2);
    with_keyboard_api(&app, |api| keyboard_driver::remap(api, index, opt))
}

#[tauri::command]
fn keyboard_restore_keymap(app: State<App>) -> Result<()> {
    with_keyboard_api(&app, keyboard_driver::restore_keymap)
}

#[tauri::command]
fn keyboard_set_custom_colors(
    app: State<App>,
    profile: u8,
    cells: Vec<keyboard::CustomCell>,
) -> Result<()> {
    with_keyboard_api(&app, |api| {
        keyboard_driver::set_custom_colors(api, profile, cells)
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let api = HidApi::new().expect("falha ao inicializar HIDAPI");
    let keyboard_gate = Arc::new(Mutex::new(false));
    let setup_gate = keyboard_gate.clone();
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        // In-app updates: the plugin handles Windows, macOS and the AppImage;
        // `update.rs` handles .deb and .rpm. `process` is what relaunches the
        // app once the plugin has swapped the binary.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(App {
            api: Mutex::new(api),
            keyboard_gate,
        })
        .setup(move |app| {
            let directory = app.path().app_config_dir()?;
            let smart = smart_notifications::SmartNotifications::start(
                app.handle().clone(),
                directory,
                setup_gate,
            );
            app.manage(smart);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            smart_notifications_get,
            smart_notifications_save,
            smart_notifications_test,
            list_devices,
            mouse_read_state,
            mouse_set_led,
            mouse_set_dpi_stage,
            mouse_set_dpi_all,
            mouse_set_dpi_color,
            mouse_set_dpi_effect,
            mouse_set_polling,
            mouse_set_button,
            keyboard_read_state,
            keyboard_effects,
            keyboard_set_effect,
            keyboard_set_brightness,
            keyboard_set_speed,
            keyboard_set_direction,
            keyboard_set_full_rgb,
            keyboard_set_color,
            keyboard_set_mode,
            keyboard_set_active_profile,
            keyboard_remap_keys,
            keyboard_remap,
            keyboard_restore_keymap,
            keyboard_set_custom_colors,
            update::update_support,
        ])
        .build(tauri::generate_context!())
        .expect("erro ao executar a aplicação Tauri")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(smart) = app.try_state::<smart_notifications::SmartNotifications>() {
                    smart.shutdown();
                }
            }
        });
}
