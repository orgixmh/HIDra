// SPDX-License-Identifier: GPL-3.0-or-later
//! Hardware-independent OSD. One passive webview, one coalescing mailbox and
//! one expiration deadline. The worker sleeps until a request or that deadline.
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub const LABEL: &str = "osd";
const DISPLAY_EVENT: &str = "hidra-osd-display";
const ERROR_EVENT: &str = "hidra-osd-error";

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    Top,
    #[default]
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub enabled: bool,
    pub position: Position,
    pub width: f64,
    pub height: f64,
    pub text_size: f64,
    pub duration: u64,
    pub edge_margin: f64,
    pub background_color: [u8; 3],
    pub transparency: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            position: Position::Bottom,
            width: 280.0,
            height: 100.0,
            text_size: 16.0,
            duration: 1500,
            edge_margin: 32.0,
            background_color: [30, 32, 36],
            transparency: 4.0,
        }
    }
}
impl Settings {
    fn validate(&self) -> Result<(), String> {
        for (value, min, max) in [
            (self.width, 120.0, 1200.0),
            (self.height, 64.0, 600.0),
            (self.text_size, 10.0, 48.0),
            (self.edge_margin, 0.0, 256.0),
        ] {
            if !value.is_finite() || !(min..=max).contains(&value) {
                return Err("Invalid OSD dimensions, text size or edge margin".into());
            }
        }
        if !self.transparency.is_finite() || !(0.0..=100.0).contains(&self.transparency) {
            return Err("OSD transparency must be between 0 and 100 percent".into());
        }
        if !(100..=30000).contains(&self.duration) {
            return Err("OSD duration must be between 100 and 30000 milliseconds".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub text: String,
    pub icon: Option<String>,
    pub text_size: Option<f64>,
    pub duration: Option<u64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub position: Option<Position>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Display {
    id: u64,
    text: String,
    icon: Option<String>,
    text_size: f64,
    settings: Settings,
}
#[derive(Serialize)]
pub struct View {
    settings: Settings,
    error: Option<String>,
}
struct Shared {
    settings: Settings,
    error: Option<String>,
    request: Option<Display>,
    next_id: u64,
    ready_epoch: u64,
    rendered: u64,
    shutdown: bool,
}
/// Available from any module, independently of the main/tray window lifecycle.
pub struct Osd {
    shared: Arc<Mutex<Shared>>,
    wake: mpsc::SyncSender<()>,
    path: PathBuf,
}

impl Osd {
    pub fn start(app: AppHandle, directory: PathBuf) -> Self {
        let path = directory.join("osd-settings.json");
        let loaded = (|| -> Result<Settings, String> {
            match fs::read(&path) {
                Ok(bytes) if bytes.len() <= 65536 => {
                    let settings: Settings =
                        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                    settings.validate()?;
                    Ok(settings)
                }
                Ok(_) => Err("OSD settings file is too large".into()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
                Err(e) => Err(e.to_string()),
            }
        })();
        let (settings, error) = match loaded {
            Ok(s) => (s, None),
            Err(e) => (Settings::default(), Some(e)),
        };
        let shared = Arc::new(Mutex::new(Shared {
            settings,
            error,
            request: None,
            next_id: 0,
            ready_epoch: 0,
            rendered: 0,
            shutdown: false,
        }));
        let (wake, receiver) = mpsc::sync_channel(1);
        let worker = shared.clone();
        thread::spawn(move || run(app, worker, receiver));
        Self { shared, wake, path }
    }
    fn notify(&self) {
        let _ = self.wake.try_send(());
    }
    pub fn get(&self) -> View {
        let shared = self.shared.lock();
        View {
            settings: shared.settings.clone(),
            error: shared.error.clone(),
        }
    }
    pub fn save(&self, settings: Settings) -> Result<View, String> {
        settings.validate()?;
        // Serialize saves so rapid requests cannot race the same atomic temp file.
        let mut shared = self.shared.lock();
        let parent = self.path.parent().ok_or("Invalid OSD settings path")?;
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
        file.write_all(&serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(temporary, &self.path).map_err(|e| e.to_string())?;
        if !settings.enabled {
            shared.request = None;
        }
        shared.settings = settings.clone();
        shared.error = None;
        drop(shared);
        self.notify();
        Ok(View {
            settings,
            error: None,
        })
    }
    pub fn show(&self, message: Message) -> Result<bool, String> {
        let mut shared = self.shared.lock();
        if shared.shutdown || !shared.settings.enabled || message.text.trim().is_empty() {
            return Ok(false);
        }
        let mut settings = shared.settings.clone();
        if let Some(v) = message.position {
            settings.position = v;
        }
        if let Some(v) = message.width {
            settings.width = v;
        }
        if let Some(v) = message.height {
            settings.height = v;
        }
        if let Some(v) = message.duration {
            settings.duration = v;
        }
        if let Some(v) = message.text_size {
            settings.text_size = v;
        }
        settings.validate()?;
        shared.next_id += 1;
        shared.request = Some(Display {
            id: shared.next_id,
            text: message.text.trim().chars().take(2048).collect(),
            icon: message.icon.map(|s| s.chars().take(64).collect()),
            text_size: settings.text_size,
            settings,
        });
        shared.error = None;
        drop(shared);
        self.notify();
        Ok(true)
    }
    pub fn ready(&self) {
        let mut shared = self.shared.lock();
        shared.ready_epoch += 1;
        shared.rendered = 0;
        drop(shared);
        self.notify();
    }
    pub fn rendered(&self, id: u64) {
        let mut shared = self.shared.lock();
        if shared.request.as_ref().is_some_and(|r| r.id == id) {
            shared.rendered = id;
        }
        drop(shared);
        self.notify();
    }
    pub fn shutdown(&self) {
        let mut shared = self.shared.lock();
        shared.shutdown = true;
        shared.request = None;
        drop(shared);
        self.notify();
    }
}

fn passive_window(app: &AppHandle, shared: &Mutex<Shared>) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(LABEL) {
        enable_click_through(&window)?;
        return Ok(window);
    }
    shared.lock().ready_epoch = 0;
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("osd.html".into()))
        .title("HIDra OSD")
        .inner_size(280.0, 100.0)
        .visible(false)
        .decorations(false)
        // GTK's non-resizable mode sizes the window from WebKit's natural
        // requisition (200px high), overriding smaller requested dimensions.
        // Allow GTK's resize path; equal min/max constraints keep the OSD fixed.
        .resizable(cfg!(target_os = "linux"))
        .min_inner_size(280.0, 100.0)
        .max_inner_size(280.0, 100.0)
        .focused(false)
        .focusable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .transparent(true)
        .shadow(false)
        .background_color(tauri::window::Color(0, 0, 0, 0))
        .build()
        .map_err(|e| format!("OSD overlay is unavailable: {e}"))?;
    // Fail closed instead of putting an interactive window over the user's work.
    enable_click_through(&window)?;
    Ok(window)
}

fn enable_click_through(window: &WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        // Tao's CursorIgnoreEvents handler unwraps the GDK window. A newly
        // created hidden GTK window has none until realized. Realize on the GTK
        // thread without mapping/showing it, then install the passive input shape.
        let target = window.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        window
            .run_on_main_thread(move || {
                use gtk::prelude::WidgetExt;
                let result = (|| -> Result<(), String> {
                    let native = target.gtk_window().map_err(|e| e.to_string())?;
                    native.realize();
                    if native.window().is_none() {
                        return Err("GTK did not realize the OSD window".into());
                    }
                    target
                        .set_ignore_cursor_events(true)
                        .map_err(|e| e.to_string())
                })();
                let _ = sender.send(result);
            })
            .map_err(|e| format!("OSD click-through is unavailable: {e}"))?;
        receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|e| format!("OSD click-through is unavailable: {e}"))?
            .map_err(|e| format!("OSD click-through is unavailable: {e}"))
    }
    #[cfg(not(target_os = "linux"))]
    window
        .set_ignore_cursor_events(true)
        .map_err(|e| format!("OSD click-through is unavailable: {e}"))
}

fn place(app: &AppHandle, window: &WebviewWindow, settings: &Settings) -> Result<(), String> {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|point| app.monitor_from_point(point.x, point.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or("OSD monitor information is unavailable")?;
    let area = monitor.work_area();
    if area.size.width == 0 || area.size.height == 0 {
        return Err("OSD monitor has no visible work area".into());
    }
    let scale = monitor.scale_factor();
    if !scale.is_finite() || scale <= 0.0 {
        return Err("OSD monitor scaling is unavailable".into());
    }
    let margin = (settings.edge_margin * scale)
        .round()
        .min((area.size.width.min(area.size.height).saturating_sub(1) / 2) as f64)
        as u32;
    let width = (settings.width * scale)
        .round()
        .min((area.size.width - 2 * margin) as f64)
        .max(1.0) as u32;
    let height = (settings.height * scale)
        .round()
        .min((area.size.height - 2 * margin) as f64)
        .max(1.0) as u32;
    let left = area.position.x as i64 + margin as i64;
    let right = area.position.x as i64 + area.size.width as i64 - margin as i64 - width as i64;
    let center = area.position.x as i64 + (area.size.width - width) as i64 / 2;
    let top = area.position.y as i64 + margin as i64;
    let bottom = area.position.y as i64 + area.size.height as i64 - margin as i64 - height as i64;
    let (x, y) = match settings.position {
        Position::Top => (center, top),
        Position::Bottom => (center, bottom),
        Position::TopLeft => (left, top),
        Position::TopRight => (right, top),
        Position::BottomLeft => (left, bottom),
        Position::BottomRight => (right, bottom),
    };
    window
        .set_position(PhysicalPosition::new(x as i32, y as i32))
        .map_err(|e| e.to_string())?;
    // Match both geometry limits to the requested viewport. GTK can otherwise
    // keep the natural WebKit height for a non-resizable window, ignoring resize.
    window
        .set_size_constraints(tauri::WindowSizeConstraints {
            min_width: Some(tauri::PhysicalUnit(width).into()),
            max_width: Some(tauri::PhysicalUnit(width).into()),
            min_height: Some(tauri::PhysicalUnit(height).into()),
            max_height: Some(tauri::PhysicalUnit(height).into()),
        })
        .map_err(|e| e.to_string())?;
    window
        .set_size(PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    Ok(())
}
fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
    }
}
fn fail(app: &AppHandle, shared: &Mutex<Shared>, id: u64, error: String) {
    log::warn!("{error}");
    let mut state = shared.lock();
    if state.shutdown || state.request.as_ref().is_some_and(|r| r.id != id) {
        return; // An obsolete request must not hide or report over its replacement.
    }
    state.request = None;
    state.error = Some(error.clone());
    drop(state);
    hide(app);
    let _ = app.emit_to("main", ERROR_EVENT, error);
}
fn run(app: AppHandle, shared: Arc<Mutex<Shared>>, receiver: mpsc::Receiver<()>) {
    let mut handled = 0;
    let mut emitted = 0;
    let mut shown = 0;
    let mut epoch = 0;
    let mut deadline: Option<Instant> = None;
    loop {
        let wake = match deadline {
            Some(time) => receiver.recv_timeout(time.saturating_duration_since(Instant::now())),
            None => receiver
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected),
        };
        match wake {
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let mut state = shared.lock();
                let expired = state.request.as_ref().is_some_and(|r| r.id == handled);
                if expired {
                    state.request = None;
                }
                let empty = state.request.is_none();
                drop(state);
                if empty {
                    hide(&app);
                }
                if expired && shown != handled {
                    fail(
                        &app,
                        &shared,
                        handled,
                        "OSD renderer did not become ready".into(),
                    );
                }
                deadline = None;
                continue;
            }
            Ok(()) => {}
        }
        let (request, ready_epoch, rendered) = {
            let state = shared.lock();
            if state.shutdown {
                break;
            }
            (state.request.clone(), state.ready_epoch, state.rendered)
        };
        let Some(request) = request else {
            hide(&app);
            deadline = None;
            continue;
        };
        if ready_epoch != epoch {
            epoch = ready_epoch;
            emitted = 0;
            shown = 0;
        }
        let result = (|| -> Result<(), String> {
            let window = passive_window(&app, &shared)?;
            if handled != request.id {
                place(&app, &window, &request.settings)?;
                handled = request.id;
                // A new webview gets a bounded startup grace period. Subsequent
                // requests replace the current message, not another native window.
                deadline = Some(
                    Instant::now()
                        + Duration::from_millis(if ready_epoch == 0 {
                            10000
                        } else {
                            request.settings.duration
                        }),
                );
            }
            let latest = shared.lock();
            if latest.shutdown || latest.request.as_ref().map(|r| r.id) != Some(request.id) {
                return Ok(());
            }
            let ready = latest.ready_epoch != 0;
            drop(latest);
            if ready && emitted != request.id {
                window
                    .emit(DISPLAY_EVENT, request.clone())
                    .map_err(|e| e.to_string())?;
                emitted = request.id;
            }
            if rendered == request.id && shown != request.id {
                // The renderer acknowledges after React commits the new text.
                // Never reveal the stale text left in a previously hidden webview.
                window.show().map_err(|e| e.to_string())?;
                shown = request.id;
                deadline = Some(Instant::now() + Duration::from_millis(request.settings.duration));
            }
            Ok(())
        })();
        if let Err(error) = result {
            fail(&app, &shared, request.id, error);
            deadline = None;
        }
    }
    // Tauri owns and destroys the webview on app exit. Do not wait for the UI
    // thread here: ExitRequested may be running on that thread already.
}
