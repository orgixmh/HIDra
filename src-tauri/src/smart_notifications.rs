// SPDX-License-Identifier: GPL-3.0-or-later
//! Event-driven notification rules and exclusive lighting ownership.
use crate::desktop_notifications::{self, Listener};
use crate::notification_lighting::{self, Lighting, Snapshot};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, VecDeque},
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

const EVENT: &str = "smart-notifications-changed";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    /// Empty app = any application, including any browser.
    pub app: String,
    /// Case-insensitive substring of title OR description. No browser allowlist.
    pub text: String,
    pub keys: Vec<u16>,
    pub color: [u8; 3],
    pub duration_seconds: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DesktopNotification {
    pub app: String,
    pub title: String,
    pub body: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecentNotification {
    #[serde(flatten)]
    pub notification: DesktopNotification,
    pub matched: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct View {
    pub settings: Settings,
    pub active: bool,
    pub status: String,
    pub error: Option<String>,
    pub recent: Vec<RecentNotification>,
    pub lit_keys: Vec<u16>,
}

pub(crate) enum Command {
    Save(Settings, SyncSender<Result<View, String>>),
    Notification(DesktopNotification),
    Test(String, SyncSender<Result<View, String>>),
    ListenerFailed(String),
    Resume,
    Shutdown(SyncSender<()>),
}

pub struct SmartNotifications {
    sender: SyncSender<Command>,
    view: Arc<Mutex<View>>,
}

impl SmartNotifications {
    pub fn start(handle: AppHandle, directory: PathBuf, exclusive: Arc<Mutex<bool>>) -> Self {
        let path = directory.join("smart-notifications.json");
        let (settings, error) = match read_json::<Settings>(&path) {
            Ok(Some(settings)) => match validate(&settings) {
                Ok(()) => (settings, None),
                Err(error) => (Settings::default(), Some(error)),
            },
            Ok(None) => (Settings::default(), None),
            Err(error) => (Settings::default(), Some(error)),
        };
        let backup = directory.join("smart-lighting-recovery.json");
        let resume = settings.enabled || backup.exists();
        *exclusive.lock() = resume;
        let view = Arc::new(Mutex::new(View {
            settings,
            active: resume,
            status: if resume {
                "starting"
            } else if error.is_some() {
                "error"
            } else {
                "off"
            }
            .into(),
            error,
            recent: Vec::new(),
            lit_keys: Vec::new(),
        }));
        let (sender, receiver) = mpsc::sync_channel(128);
        let worker = Worker {
            handle,
            sender: sender.clone(),
            view: view.clone(),
            exclusive,
            path,
            backup,
            lighting: None,
            listener: None,
            alerts: Vec::new(),
            seen: VecDeque::new(),
            last_frame: Instant::now(),
        };
        thread::spawn(move || worker.run(receiver));
        let smart = Self { sender, view };
        if resume {
            let _ = smart.sender.send(Command::Resume);
        }
        smart
    }

    pub fn get(&self) -> View {
        self.view.lock().clone()
    }
    pub fn save(&self, settings: Settings) -> Result<View, String> {
        validate(&settings)?;
        let (tx, rx) = mpsc::sync_channel(1);
        self.sender
            .send(Command::Save(settings, tx))
            .map_err(|_| "Notification worker stopped")?;
        rx.recv_timeout(Duration::from_secs(12))
            .map_err(|_| "Notification settings timed out".to_string())?
    }
    pub fn test(&self, id: String) -> Result<View, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.sender
            .send(Command::Test(id, tx))
            .map_err(|_| "Notification worker stopped")?;
        rx.recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Notification test timed out".to_string())?
    }
    pub fn shutdown(&self) {
        let (tx, rx) = mpsc::sync_channel(1);
        if self.sender.send(Command::Shutdown(tx)).is_ok() {
            let _ = rx.recv_timeout(Duration::from_secs(8));
        }
    }
}

struct Alert {
    rule: Rule,
    expires: Instant,
}
struct Worker {
    handle: AppHandle,
    sender: SyncSender<Command>,
    view: Arc<Mutex<View>>,
    exclusive: Arc<Mutex<bool>>,
    path: PathBuf,
    backup: PathBuf,
    lighting: Option<Lighting>,
    listener: Option<Listener>,
    alerts: Vec<Alert>,
    seen: VecDeque<(String, Instant)>,
    last_frame: Instant,
}

impl Worker {
    fn publish(&self) {
        let mut view = self.view.lock();
        view.active = *self.exclusive.lock();
        view.lit_keys = self
            .alerts
            .iter()
            .flat_map(|alert| alert.rule.keys.iter().copied())
            .collect();
        view.lit_keys.sort_unstable();
        view.lit_keys.dedup();
        let _ = self.handle.emit(EVENT, view.clone());
    }
    fn fail(&mut self, error: String) {
        let restore = self.restore();
        let mut view = self.view.lock();
        view.status = "error".into();
        view.error = Some(match restore {
            Ok(()) => error,
            Err(restore) => format!("{error}. Lighting recovery pending: {restore}"),
        });
    }
    fn recover(&self) -> Result<(), String> {
        if let Some(snapshot) = read_json::<Snapshot>(&self.backup)? {
            *self.exclusive.lock() = true;
            notification_lighting::recover(&snapshot).map_err(|e| e.to_string())?;
            remove_backup(&self.backup)?;
        }
        Ok(())
    }
    fn enable(&mut self) -> Result<(), String> {
        if self.lighting.is_some() && self.listener.is_some() {
            return Ok(());
        }
        *self.exclusive.lock() = true;
        self.recover()?;
        self.lighting.take();
        // Obtain monitoring permission before changing any keyboard settings.
        let listener = desktop_notifications::start(self.sender.clone())?;
        let lighting = Lighting::capture().map_err(|e| e.to_string())?;
        atomic_json(&self.backup, &lighting.snapshot)?;
        self.lighting = Some(lighting);
        self.listener = Some(listener);
        self.lighting
            .as_ref()
            .unwrap()
            .activate()
            .map_err(|e| e.to_string())?;
        let mut view = self.view.lock();
        view.status = "listening".into();
        view.error = None;
        Ok(())
    }
    fn restore(&mut self) -> Result<(), String> {
        self.listener.take();
        self.alerts.clear();
        self.seen.clear();
        if let Some(lighting) = &self.lighting {
            lighting
                .restore(&lighting.snapshot)
                .map_err(|e| e.to_string())?;
            remove_backup(&self.backup)?;
            self.lighting.take();
        } else {
            self.recover()?;
        }
        *self.exclusive.lock() = false;
        Ok(())
    }
    fn configure(&mut self, settings: Settings) -> Result<View, String> {
        if settings.enabled {
            self.enable()?;
        } else {
            self.restore()?;
        }
        atomic_json(&self.path, &settings)?;
        // Saving rules clears any alert using an old colour or key assignment.
        self.alerts.clear();
        if let Some(lighting) = &self.lighting {
            lighting.frame(&[]).map_err(|e| e.to_string())?;
        }
        let mut view = self.view.lock();
        view.settings = settings;
        view.status = if view.settings.enabled {
            "listening"
        } else {
            "off"
        }
        .into();
        view.error = None;
        drop(view);
        self.publish();
        Ok(self.view.lock().clone())
    }
    fn light(&mut self) -> Result<(), String> {
        if let Some(lighting) = &self.lighting {
            let cells: Vec<_> = self
                .alerts
                .iter()
                .flat_map(|alert| alert.rule.keys.iter().map(|&off| (off, alert.rule.color)))
                .collect();
            lighting.frame(&cells).map_err(|e| e.to_string())?;
            self.last_frame = Instant::now();
        }
        Ok(())
    }
    fn alert(&mut self, rule: Rule) {
        self.alerts.retain(|alert| alert.rule.id != rule.id);
        self.alerts.push(Alert {
            expires: Instant::now() + Duration::from_secs(rule.duration_seconds.into()),
            rule,
        });
    }
    fn notification(&mut self, notification: DesktopNotification) -> Result<(), String> {
        if self.listener.is_none() {
            return Ok(());
        }
        let now = Instant::now();
        let fingerprint = format!(
            "{}\n{}\n{}",
            notification.app, notification.title, notification.body
        );
        self.seen
            .retain(|(_, time)| now.duration_since(*time) < Duration::from_secs(2));
        if self
            .seen
            .iter()
            .any(|(previous, _)| previous == &fingerprint)
        {
            return Ok(());
        }
        self.seen.push_back((fingerprint, now));
        if self.seen.len() > 32 {
            self.seen.pop_front();
        }
        let rules: Vec<_> = self
            .view
            .lock()
            .settings
            .rules
            .iter()
            .filter(|rule| rule.matches(&notification))
            .cloned()
            .collect();
        let matched = !rules.is_empty();
        for rule in rules {
            self.alert(rule);
        }
        {
            let mut view = self.view.lock();
            view.recent.insert(
                0,
                RecentNotification {
                    notification,
                    matched,
                },
            );
            view.recent.truncate(12); // memory only: notification content is never saved
        }
        if matched {
            self.light()?;
        }
        self.publish();
        Ok(())
    }
    fn run(mut self, receiver: Receiver<Command>) {
        loop {
            let message = if self.lighting.is_none() || self.listener.is_none() {
                receiver.recv().map_err(|_| RecvTimeoutError::Disconnected)
            } else {
                // Firmware needs volatile keepalives even for an all-black idle frame.
                // Reuse the open handle: no discovery, reads or saved-colour writes.
                receiver.recv_timeout(
                    Duration::from_millis(350).saturating_sub(self.last_frame.elapsed()),
                )
            };
            match message {
                Ok(Command::Save(settings, reply)) => {
                    let result = self.configure(settings);
                    if let Err(error) = &result {
                        self.fail(error.clone());
                        self.publish();
                    }
                    let _ = reply.send(result);
                }
                Ok(Command::Resume) => {
                    let result = self.recover().and_then(|_| {
                        if self.view.lock().settings.enabled {
                            self.enable()
                        } else {
                            *self.exclusive.lock() = false;
                            let mut view = self.view.lock();
                            view.status = "off".into();
                            view.error = None;
                            Ok(())
                        }
                    });
                    if let Err(error) = result {
                        self.fail(error);
                    }
                    self.publish();
                }
                Ok(Command::Notification(event)) => {
                    if let Err(error) = self.notification(event) {
                        self.fail(error);
                        self.publish();
                    }
                }
                Ok(Command::Test(id, reply)) => {
                    let rule = self
                        .view
                        .lock()
                        .settings
                        .rules
                        .iter()
                        .find(|rule| rule.id == id)
                        .cloned();
                    let result = if self.listener.is_none() {
                        Err("Turn on Smart Notifications before testing a rule".into())
                    } else if let Some(rule) = rule {
                        self.alert(rule);
                        self.light().map(|_| {
                            self.publish();
                            self.view.lock().clone()
                        })
                    } else {
                        Err("Save the rule before testing it".into())
                    };
                    if let Err(error) = &result {
                        // Missing/disabled rules do not relinquish lighting ownership.
                        if self.listener.is_some()
                            && self
                                .view
                                .lock()
                                .settings
                                .rules
                                .iter()
                                .any(|rule| rule.id == id)
                        {
                            self.fail(error.clone());
                            self.publish();
                        }
                    }
                    let _ = reply.send(result);
                }
                Ok(Command::ListenerFailed(error)) => {
                    self.fail(error);
                    self.publish();
                }
                Ok(Command::Shutdown(reply)) => {
                    let _ = self.restore();
                    let _ = reply.send(());
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    let _ = self.restore();
                    break;
                }
            }
            let before = self.alerts.len();
            self.alerts.retain(|alert| alert.expires > Instant::now());
            let expired = before != self.alerts.len();
            if expired
                || (self.listener.is_some()
                    && self.last_frame.elapsed() >= Duration::from_millis(350))
            {
                if let Err(error) = self.light() {
                    self.fail(error);
                    self.publish();
                } else if expired {
                    self.publish();
                }
            }
        }
    }
}

impl Rule {
    fn matches(&self, notification: &DesktopNotification) -> bool {
        self.enabled
            && !self.keys.is_empty()
            && (self.app.trim().is_empty()
                || notification
                    .app
                    .to_lowercase()
                    .contains(&self.app.trim().to_lowercase()))
            && (self.text.trim().is_empty()
                || format!("{}\n{}", notification.title, notification.body)
                    .to_lowercase()
                    .contains(&self.text.trim().to_lowercase()))
    }
}

fn validate(settings: &Settings) -> Result<(), String> {
    if settings.rules.len() > 64 {
        return Err("Use at most 64 notification rules".into());
    }
    let mut ids = HashSet::new();
    for rule in &settings.rules {
        if rule.id.is_empty() || rule.id.len() > 100 || !ids.insert(&rule.id) {
            return Err("Notification rule IDs must be unique".into());
        }
        if rule.name.trim().is_empty()
            || rule.name.len() > 100
            || rule.app.len() > 200
            || rule.text.len() > 300
        {
            return Err("Enter a rule name and shorter notification filters".into());
        }
        if rule.app.trim().is_empty() && rule.text.trim().is_empty() {
            return Err("Choose an application or title/description filter".into());
        }
        if !(1..=60).contains(&rule.duration_seconds) {
            return Err("Alert duration must be between 1 and 60 seconds".into());
        }
        if rule.keys.is_empty() || rule.keys.len() > 126 {
            return Err("Select at least one keyboard key".into());
        }
        let mut keys = HashSet::new();
        for &off in &rule.keys {
            if !keys.insert(off)
                || !crate::devices::SUPPORTED
                    .iter()
                    .filter_map(|device| device.sinowealth)
                    .any(|spec| spec.keys.iter().any(|key| key.off == off))
            {
                return Err("Unknown or duplicate keyboard key in notification rule".into());
            }
        }
    }
    Ok(())
}

fn remove_backup(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if bytes.len() > 128 * 1024 {
        return Err("Notification settings file is too large".into());
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| e.to_string())
}

fn atomic_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid settings path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = path.with_extension("tmp");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temp).map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rule() -> Rule {
        Rule {
            id: "one".into(),
            name: "Messages".into(),
            enabled: true,
            app: "Viber".into(),
            text: String::new(),
            keys: vec![24],
            color: [100, 30, 255],
            duration_seconds: 6,
        }
    }
    #[test]
    fn native_app_and_web_filters_are_case_insensitive_and_composable() {
        let mut rule = rule();
        let mut event = DesktopNotification {
            app: "VIBER (com.viber.Viber)".into(),
            title: "Alice".into(),
            body: "Hello".into(),
        };
        assert!(rule.matches(&event));
        rule.app.clear();
        rule.text = "Facebook".into();
        for browser in ["Firefox", "Brave", "Chromium", "Unknown future browser"] {
            event.app = browser.into();
            event.title = "Facebook: new message".into();
            assert!(rule.matches(&event));
        }
        event.title = "Mail".into();
        event.body = "A FACEBOOK notification".into();
        assert!(rule.matches(&event));
        rule.app = "Viber".into();
        assert!(!rule.matches(&event));
        rule.app.clear();
        rule.enabled = false;
        assert!(!rule.matches(&event));
    }
    #[test]
    fn invalid_rules_cannot_enable_the_lighting_engine() {
        let mut settings = Settings {
            enabled: true,
            rules: vec![rule()],
        };
        assert!(validate(&settings).is_ok());
        settings.rules[0].keys = vec![1];
        assert!(validate(&settings).is_err());
        settings.rules[0] = rule();
        settings.rules[0].text.clear();
        settings.rules[0].app.clear();
        assert!(validate(&settings).is_err());
        settings.rules[0] = rule();
        settings.rules[0].duration_seconds = 0;
        assert!(validate(&settings).is_err());
        settings.rules = vec![rule(), rule()];
        assert!(validate(&settings).is_err());
    }
    #[test]
    fn settings_default_to_off_and_do_not_store_notifications() {
        let settings = Settings::default();
        assert!(!settings.enabled);
        let encoded = serde_json::to_string(&settings).unwrap();
        assert!(!encoded.contains("body"));
    }
    #[test]
    fn on_off_preference_and_rules_survive_a_settings_roundtrip() {
        let directory =
            std::env::temp_dir().join(format!("hidra-smart-settings-test-{}", std::process::id()));
        let path = directory.join("settings.json");
        let mut settings = Settings {
            enabled: true,
            rules: vec![rule()],
        };
        atomic_json(&path, &settings).unwrap();
        let saved = read_json::<Settings>(&path).unwrap().unwrap();
        assert!(saved.enabled);
        assert_eq!(saved.rules[0].keys, vec![24]);
        settings.enabled = false;
        atomic_json(&path, &settings).unwrap();
        assert!(!read_json::<Settings>(&path).unwrap().unwrap().enabled);
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
