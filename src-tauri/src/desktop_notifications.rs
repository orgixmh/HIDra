// SPDX-License-Identifier: GPL-3.0-or-later
//! A private, read-only session-bus monitor with narrowly scoped match rules.
use crate::smart_notifications::Command;
#[cfg(target_os = "linux")]
use crate::smart_notifications::{DesktopNotification, NotificationId};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::SyncSender,
    Arc,
};
use std::thread::JoinHandle;
#[cfg(target_os = "linux")]
use std::{thread, time::Duration};

pub struct Listener {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(target_os = "linux")]
pub fn start(sender: SyncSender<Command>) -> Result<Listener, String> {
    use dbus::blocking::Connection;
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let connect = (|| -> Result<(Connection, Tracker), String> {
            let connection = Connection::new_session().map_err(|e| e.to_string())?;
            let proxy = connection.with_proxy(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                Duration::from_secs(2),
            );
            let (owner,): (String,) = proxy
                .method_call(
                    "org.freedesktop.DBus",
                    "GetNameOwner",
                    ("org.freedesktop.Notifications",),
                )
                .map_err(|e| format!("Desktop notification monitoring unavailable: {e}"))?;
            let shell: Result<(String,), _> =
                proxy.method_call("org.freedesktop.DBus", "GetNameOwner", ("org.gnome.Shell",));
            let tracker = Tracker::new(owner.clone(), shell.ok().map(|entry| entry.0));
            let rules = vec![
                "type='method_call',interface='org.freedesktop.Notifications',member='Notify'".to_string(),
                "type='method_call',interface='org.gtk.Notifications',member='AddNotification'".to_string(),
                "type='method_call',interface='org.gtk.Notifications',member='RemoveNotification'".to_string(),
                "type='signal',interface='org.freedesktop.Notifications',member='NotificationClosed'".to_string(),
                "type='signal',interface='org.freedesktop.DBus',member='NameOwnerChanged',arg0='org.freedesktop.Notifications'".to_string(),
                format!("type='method_return',sender='{owner}'"),
                format!("type='error',sender='{owner}'"),
            ];
            let result: Result<(), dbus::Error> = proxy.method_call(
                "org.freedesktop.DBus.Monitoring",
                "BecomeMonitor",
                (rules, 0u32),
            );
            result.map_err(|e| format!("Desktop notification monitoring unavailable: {e}"))?;
            Ok((connection, tracker))
        })();
        let (connection, mut tracker) = match connect {
            Ok(pair) => {
                let _ = ready_tx.send(Ok(()));
                pair
            }
            Err(error) => {
                let _ = ready_tx.send(Err(error));
                return;
            }
        };
        while !stopped.load(Ordering::Acquire) {
            // This blocks on D-Bus; it never queries or polls the keyboard.
            match connection
                .channel()
                .blocking_pop_message(Duration::from_millis(500))
            {
                Ok(Some(message)) => {
                    if let Some(event) = tracker.process(&message) {
                        // Bound memory under notification storms. Never block the bus.
                        let _ = sender.try_send(event);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    let _ = sender.try_send(Command::ListenerFailed(error.to_string()));
                    break;
                }
            }
        }
    });
    let listener = Listener {
        stop,
        thread: Some(worker),
    };
    ready_rx
        .recv_timeout(Duration::from_secs(4))
        .map_err(|_| "Desktop notification monitor did not start".to_string())??;
    Ok(listener)
}

#[cfg(not(target_os = "linux"))]
pub fn start(_sender: SyncSender<Command>) -> Result<Listener, String> {
    Err("Smart Notifications currently requires a Linux desktop session".into())
}

#[cfg(target_os = "linux")]
struct Tracker {
    owner: String,
    shell: Option<String>,
    pending: std::collections::VecDeque<(String, u32, std::time::Instant, DesktopNotification)>,
}
#[cfg(target_os = "linux")]
impl Tracker {
    fn new(owner: String, shell: Option<String>) -> Self {
        Self {
            owner,
            shell,
            pending: std::collections::VecDeque::new(),
        }
    }
    fn process(&mut self, message: &dbus::Message) -> Option<Command> {
        use dbus::MessageType;
        self.pending
            .retain(|entry| entry.2.elapsed() < Duration::from_secs(5));
        let interface = message
            .interface()
            .map(|v| v.to_string())
            .unwrap_or_default();
        let member = message.member().map(|v| v.to_string()).unwrap_or_default();
        let sender = message.sender().map(|v| v.to_string()).unwrap_or_default();
        if message.msg_type() == MessageType::Signal {
            if interface == "org.freedesktop.DBus" && member == "NameOwnerChanged" {
                let (name, old, new): (String, String, String) = message.read_all().ok()?;
                if name == "org.freedesktop.Notifications" && old == self.owner && new != old {
                    return Some(Command::ListenerFailed(
                        "The desktop notification service restarted. Retry the connection.".into(),
                    ));
                }
            }
            if interface == "org.freedesktop.Notifications"
                && member == "NotificationClosed"
                && (sender == self.owner || self.shell.as_deref() == Some(&sender))
            {
                let (id, _reason): (u32, u32) = message.read_all().ok()?;
                return Some(Command::Closed(NotificationId::Freedesktop(id)));
            }
            return None;
        }
        if matches!(
            message.msg_type(),
            MessageType::MethodReturn | MessageType::Error
        ) && sender == self.owner
        {
            let serial = message.get_reply_serial()?;
            let destination = message.destination()?.to_string();
            let index = self
                .pending
                .iter()
                .position(|entry| entry.0 == destination && entry.1 == serial)?;
            let (_, _, _, notification) = self.pending.remove(index)?;
            if message.msg_type() == MessageType::Error {
                return None;
            }
            let (id,): (u32,) = message.read_all().ok()?;
            return (id != 0).then_some(Command::Notification(
                notification,
                NotificationId::Freedesktop(id),
            ));
        }
        if message.msg_type() != MessageType::MethodCall {
            return None;
        }
        if interface == "org.freedesktop.Notifications" && member == "Notify" {
            // GNOME forwards public calls to its shell. Track only the public
            // request/return pair; the shell and proxy use the same server ID.
            let destination = message.destination()?.to_string();
            if destination != "org.freedesktop.Notifications" && destination != self.owner {
                return None;
            }
            let event = parse(message)?;
            let serial = message.get_serial()?;
            if self.pending.len() >= 128 {
                self.pending.pop_front();
            }
            self.pending
                .push_back((sender, serial, std::time::Instant::now(), event));
        } else if interface == "org.gtk.Notifications" && member == "AddNotification" {
            let (app, id, _): (String, String, dbus::arg::PropMap) = message.read_all().ok()?;
            return Some(Command::Notification(
                parse(message)?,
                NotificationId::Gtk(app, id),
            ));
        } else if interface == "org.gtk.Notifications" && member == "RemoveNotification" {
            let (app, id): (String, String) = message.read_all().ok()?;
            return Some(Command::Closed(NotificationId::Gtk(app, id)));
        }
        None
    }
}

#[cfg(target_os = "linux")]
fn parse(message: &dbus::Message) -> Option<DesktopNotification> {
    use dbus::arg::PropMap;
    let interface = message.interface()?.to_string();
    let member = message.member()?.to_string();
    let (app, title, body) = if interface == "org.freedesktop.Notifications" && member == "Notify" {
        let (app, _, _, title, body, _, hints, _): (
            String,
            u32,
            String,
            String,
            String,
            Vec<String>,
            PropMap,
            i32,
        ) = message.read_all().ok()?;
        let desktop = hints
            .get("desktop-entry")
            .and_then(|v| v.0.as_str())
            .unwrap_or("");
        let name = if app.is_empty() {
            desktop.into()
        } else if !desktop.is_empty() && !app.to_lowercase().contains(&desktop.to_lowercase()) {
            format!("{app} ({desktop})")
        } else {
            app
        };
        (name, title, body)
    } else if interface == "org.gtk.Notifications" && member == "AddNotification" {
        let (app, _, properties): (String, String, PropMap) = message.read_all().ok()?;
        let text = |key| {
            properties
                .get(key)
                .and_then(|v| v.0.as_str())
                .unwrap_or("")
                .to_string()
        };
        (app, text("title"), text("body"))
    } else {
        return None;
    };
    Some(DesktopNotification {
        app: app.chars().take(160).collect(),
        title: plain_text(&title).chars().take(300).collect(),
        body: plain_text(&body).chars().take(1500).collect(),
    })
}

#[cfg(target_os = "linux")]
fn plain_text(value: &str) -> String {
    let mut markup = false;
    let text: String = value
        .chars()
        .filter(|&c| {
            if c == '<' {
                markup = true;
                false
            } else if c == '>' && markup {
                markup = false;
                false
            } else {
                !markup
            }
        })
        .collect();
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use dbus::arg::{PropMap, Variant};
    #[test]
    fn parses_native_and_portal_forwarded_notifications() {
        let message = dbus::Message::new_method_call(
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "Notify",
        )
        .unwrap()
        .append3("Viber", 0u32, "viber")
        .append3(
            "New message",
            "<b>Alice</b> &amp; Bob",
            Vec::<String>::new(),
        )
        .append2(PropMap::new(), -1i32);
        let event = parse(&message).unwrap();
        assert_eq!(event.app, "Viber");
        assert_eq!(event.body, "Alice & Bob");
        let mut map = PropMap::new();
        map.insert("title".into(), Variant(Box::new("Gmail".to_string())));
        map.insert("body".into(), Variant(Box::new("New mail".to_string())));
        let message = dbus::Message::new_method_call(
            "org.gtk.Notifications",
            "/org/gtk/Notifications",
            "org.gtk.Notifications",
            "AddNotification",
        )
        .unwrap()
        .append3("org.mozilla.firefox", "mail-1", map);
        assert_eq!(parse(&message).unwrap().title, "Gmail");
    }
    #[test]
    fn ignores_unrelated_bus_messages() {
        let msg = dbus::Message::new_method_call(
            "org.example.App",
            "/org/example/App",
            "org.example.App",
            "Notify",
        )
        .unwrap();
        assert!(parse(&msg).is_none());
    }
    #[test]
    #[ignore = "requires access to the real Linux session bus"]
    fn session_monitor_can_start_without_root() {
        let (sender, _) = std::sync::mpsc::sync_channel(8);
        let monitor = start(sender).unwrap();
        drop(monitor);
    }
    #[test]
    #[ignore = "sends a local test desktop notification and observes the real session bus"]
    fn receives_a_real_desktop_notification() {
        let (sender, receiver) = std::sync::mpsc::sync_channel(16);
        let monitor = start(sender).unwrap();
        let status = std::process::Command::new("notify-send")
            .args([
                "-a",
                "HIDra test",
                "HIDra listener test",
                "Smart Notifications capture test",
            ])
            .status()
            .unwrap();
        assert!(status.success());
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut received = false;
        while std::time::Instant::now() < deadline {
            if let Ok(Command::Notification(event, _)) =
                receiver.recv_timeout(Duration::from_millis(250))
            {
                if event.app.contains("HIDra test") && event.title == "HIDra listener test" {
                    received = true;
                    break;
                }
            }
        }
        drop(monitor);
        assert!(
            received,
            "the native notification was not delivered to the monitor"
        );
    }
    fn notify_call(sender: &str, serial: u32) -> dbus::Message {
        let mut message = dbus::Message::new_method_call(
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "Notify",
        )
        .unwrap()
        .append3("HIDra test", 0u32, "")
        .append3("HIDra", "Test", Vec::<String>::new())
        .append2(PropMap::new(), -1i32);
        message.set_sender(Some(sender.into()));
        message.set_serial(serial);
        message
    }
    #[test]
    fn matches_replies_by_sender_and_serial_and_ignores_forwarded_duplicates() {
        let mut tracker = Tracker::new(":1.20".into(), Some(":1.21".into()));
        let first = notify_call(":1.30", 7);
        let second = notify_call(":1.31", 7);
        assert!(tracker.process(&first).is_none());
        assert!(tracker.process(&second).is_none());
        let mut forwarded = notify_call(":1.20", 8);
        forwarded.set_destination(Some("org.gnome.Shell".into()));
        assert!(tracker.process(&forwarded).is_none());
        assert_eq!(tracker.pending.len(), 2);
        let mut reply = second.method_return().append1(42u32);
        reply.set_sender(Some(":1.20".into()));
        assert!(matches!(
            tracker.process(&reply),
            Some(Command::Notification(_, NotificationId::Freedesktop(42)))
        ));
        assert_eq!(tracker.pending.len(), 1);
        let mut closed = dbus::Message::new_signal(
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "NotificationClosed",
        )
        .unwrap()
        .append2(42u32, 2u32);
        closed.set_sender(Some(":1.21".into()));
        assert!(matches!(
            tracker.process(&closed),
            Some(Command::Closed(NotificationId::Freedesktop(42)))
        ));
    }
    #[test]
    fn gtk_removal_keeps_app_ids_separate() {
        let mut tracker = Tracker::new(":1.20".into(), None);
        let remove = dbus::Message::new_method_call(
            "org.gtk.Notifications",
            "/org/gtk/Notifications",
            "org.gtk.Notifications",
            "RemoveNotification",
        )
        .unwrap()
        .append2("org.example.App", "42");
        assert!(
            matches!(tracker.process(&remove), Some(Command::Closed(NotificationId::Gtk(app, id))) if app == "org.example.App" && id == "42")
        );
    }
    #[test]
    #[ignore = "sends and closes a local notification on the real session bus"]
    fn real_notification_id_and_closure_are_correlated() {
        use dbus::blocking::Connection;
        let (sender, receiver) = std::sync::mpsc::sync_channel(32);
        let monitor = start(sender).unwrap();
        let connection = Connection::new_session().unwrap();
        let proxy = connection.with_proxy(
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            Duration::from_secs(3),
        );
        let (id,): (u32,) = proxy
            .method_call(
                "org.freedesktop.Notifications",
                "Notify",
                (
                    "HIDra lifecycle test",
                    0u32,
                    "",
                    "HIDra lifecycle",
                    "Temporary local test",
                    Vec::<String>::new(),
                    PropMap::new(),
                    0i32,
                ),
            )
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut received = false;
        while std::time::Instant::now() < deadline {
            if let Ok(Command::Notification(event, NotificationId::Freedesktop(actual))) =
                receiver.recv_timeout(Duration::from_millis(100))
            {
                if actual == id && event.title == "HIDra lifecycle" {
                    received = true;
                    break;
                }
            }
        }
        let close: Result<(), dbus::Error> =
            proxy.method_call("org.freedesktop.Notifications", "CloseNotification", (id,));
        assert!(close.is_ok());
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut closed = false;
        while std::time::Instant::now() < deadline {
            if let Ok(Command::Closed(NotificationId::Freedesktop(actual))) =
                receiver.recv_timeout(Duration::from_millis(100))
            {
                if actual == id {
                    closed = true;
                    break;
                }
            }
        }
        drop(monitor);
        assert!(received, "server ID was not correlated with Notify");
        assert!(closed, "NotificationClosed was not observed");
    }
}
