// SPDX-License-Identifier: GPL-3.0-or-later
//! A private, read-only session-bus monitor with narrowly scoped match rules.
use crate::smart_notifications::{Command, DesktopNotification};
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
        let connect = (|| -> Result<Connection, String> {
            let connection = Connection::new_session().map_err(|e| e.to_string())?;
            let proxy = connection.with_proxy(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                Duration::from_secs(2),
            );
            // Portal backends forward to one of these two desktop endpoints.
            // Observing only the final hop also avoids processing a portal
            // notification twice and gives us its resolved application identity.
            let rules = vec![
                "type='method_call',interface='org.freedesktop.Notifications',member='Notify'"
                    .to_string(),
                "type='method_call',interface='org.gtk.Notifications',member='AddNotification'"
                    .to_string(),
            ];
            let result: Result<(), dbus::Error> = proxy.method_call(
                "org.freedesktop.DBus.Monitoring",
                "BecomeMonitor",
                (rules, 0u32),
            );
            result.map_err(|e| format!("Desktop notification monitoring unavailable: {e}"))?;
            Ok(connection)
        })();
        let connection = match connect {
            Ok(connection) => {
                let _ = ready_tx.send(Ok(()));
                connection
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
                    if let Some(event) = parse(&message) {
                        // Bound memory under notification storms. Never block the bus.
                        let _ = sender.try_send(Command::Notification(event));
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
            if let Ok(Command::Notification(event)) =
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
}
