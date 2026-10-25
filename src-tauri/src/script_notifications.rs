// SPDX-License-Identifier: GPL-3.0-or-later
//! User-selected executables run independently of HID I/O, without a terminal.
use crate::smart_notifications::{Command, Rule};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::SyncSender,
    Arc,
};
use std::thread::JoinHandle;

pub struct Runner {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Drop for Runner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
#[cfg(target_os = "linux")]
pub fn start(rules: Vec<Rule>, generation: u64, sender: SyncSender<Command>) -> Runner {
    use std::{
        os::unix::process::CommandExt,
        process::{Child, Command as Process, Stdio},
        time::{Duration, Instant},
    };
    struct Job {
        rule: Rule,
        next: Instant,
        child: Option<Child>,
        started: Instant,
    }
    fn terminate(child: &mut Child) {
        // Each child starts its own group, so timeout/Off also stops descendants.
        // SAFETY: this PID belongs to our unreaped child; it cannot be reused yet.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        let _ = child.wait();
    }
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let thread = std::thread::spawn(move || {
        let now = Instant::now();
        let mut jobs: Vec<_> = rules
            .into_iter()
            .map(|rule| Job {
                rule,
                next: now,
                child: None,
                started: now,
            })
            .collect();
        while !stopped.load(Ordering::Acquire) {
            for job in &mut jobs {
                if stopped.load(Ordering::Acquire) {
                    break;
                }
                let update = |running, exit, error| {
                    let _ = sender.try_send(Command::ScriptUpdate {
                        generation,
                        id: job.rule.id.clone(),
                        running,
                        exit,
                        error,
                    });
                };
                if let Some(child) = &mut job.child {
                    let result = match child.try_wait() {
                        Ok(Some(status)) => Some((
                            status.code(),
                            if matches!(status.code(), Some(0 | 1)) {
                                None
                            } else {
                                Some("Script must exit with 0 (clear) or 1 (trigger)".into())
                            },
                        )),
                        Ok(None) if job.started.elapsed() >= Duration::from_secs(10) => {
                            terminate(child);
                            Some((None, Some("Script exceeded the 10-second timeout".into())))
                        }
                        Ok(None) => None,
                        Err(error) => {
                            terminate(child);
                            Some((None, Some(error.to_string())))
                        }
                    };
                    if let Some((exit, error)) = result {
                        job.child.take();
                        job.next =
                            Instant::now() + Duration::from_secs(job.rule.interval_seconds.into());
                        update(false, exit, error);
                    }
                } else if Instant::now() >= job.next {
                    let mut process = Process::new(&job.rule.script_path);
                    process
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .process_group(0);
                    if let Some(parent) = std::path::Path::new(&job.rule.script_path).parent() {
                        process.current_dir(parent);
                    }
                    match process.spawn() {
                        Ok(child) => {
                            job.child = Some(child);
                            job.started = Instant::now();
                            update(true, None, None);
                        }
                        Err(error) => {
                            job.next = Instant::now()
                                + Duration::from_secs(job.rule.interval_seconds.into());
                            update(false, None, Some(format!("Cannot run script: {error}")));
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        for job in &mut jobs {
            if let Some(child) = &mut job.child {
                terminate(child);
            }
        }
    });
    Runner {
        stop,
        thread: Some(thread),
    }
}
#[cfg(not(target_os = "linux"))]
pub fn start(_rules: Vec<Rule>, _generation: u64, _sender: SyncSender<Command>) -> Runner {
    Runner {
        stop: Arc::new(AtomicBool::new(false)),
        thread: None,
    }
}
