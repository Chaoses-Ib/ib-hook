#[cfg(feature = "sysinfo")]
use std::path::Path;
use std::{collections::HashMap, time::SystemTime};

use bon::bon;

use crate::{
    process::Pid,
    windows::shell::{ShellHook, ShellHookMessage},
};

/// Callback for GUI process events
pub trait GuiProcessCallback: FnMut(GuiProcessEvent) + Send + 'static {}

impl<T: FnMut(GuiProcessEvent) + Send + 'static> GuiProcessCallback for T {}

/// Event types for GUI process monitoring.
///
/// These events are triggered by shell hook messages that indicate GUI process
/// activity.
///
/// - For a process started after the watcher, [`CreateOrAlive`](Self::CreateOrAlive) must occur before
///   [`Alive`](Self::Alive) with the same PID.
#[derive(Debug, Clone, Copy)]
pub enum GuiProcessEvent {
    /// A new GUI process has been created, or an existing process is detected.
    CreateOrAlive(Pid),

    /// An existing GUI process is detected.
    Alive(Pid),
}

impl GuiProcessEvent {
    pub fn pid(&self) -> Pid {
        match self {
            GuiProcessEvent::CreateOrAlive(pid) => *pid,
            GuiProcessEvent::Alive(pid) => *pid,
        }
    }
}

/**
Monitors GUI processes, using the Windows shell hook API.

## Examples
```no_run
use ib_hook::process::{GuiProcessEvent, GuiProcessWatcher};

let watcher = GuiProcessWatcher::new(Box::new(|event| {
    println!("Process event: {event:?}");
})).unwrap();

println!("Monitoring GUI processes...");
std::thread::sleep(std::time::Duration::from_secs(60));
```
*/
pub struct GuiProcessWatcher {
    _shell: ShellHook,
}

#[bon]
impl GuiProcessWatcher {
    /// Creates a new GUI process watcher with the given callback.
    ///
    /// The callback will be called for each process event (window creation,
    /// activation, rude activation, and replacement).
    pub fn new(callback: impl GuiProcessCallback) -> windows::core::Result<Self> {
        Self::with_on_hooked(callback, || ())
    }

    pub fn with_on_hooked(
        mut callback: impl GuiProcessCallback,
        on_hooked: impl FnOnce() + Send + 'static,
    ) -> windows::core::Result<Self> {
        let shell_callback = move |msg: ShellHookMessage| {
            match msg {
                ShellHookMessage::WindowCreated(hwnd) => {
                    if let Ok(pid) = hwnd.try_into() {
                        callback(GuiProcessEvent::CreateOrAlive(pid));
                    }
                }
                ShellHookMessage::WindowActivated(hwnd)
                | ShellHookMessage::RudeAppActivated(hwnd)
                | ShellHookMessage::WindowReplacing(hwnd) => {
                    if let Ok(pid) = hwnd.try_into() {
                        callback(GuiProcessEvent::Alive(pid));
                    }
                }
                _ => {}
            }
            false
        };
        let shell = ShellHook::with_on_hooked(Box::new(shell_callback), |_| on_hooked())?;
        Ok(GuiProcessWatcher { _shell: shell })
    }

    /// Creates a new GUI process watcher with a deduplication buffer.
    ///
    /// This version deduplicates process events to avoid duplicate notifications
    /// when multiple windows are created by the same process.
    pub fn with_dedup(callback: impl GuiProcessCallback) -> windows::core::Result<Self> {
        Self::with_filter_dedup(callback).filter(|_| true).build()
    }

    /// Creates a new GUI process watcher with a deduplication buffer and filters
    /// to reduce syscalls.
    ///
    /// This version deduplicates process events to avoid duplicate notifications
    /// when multiple windows are created by the same process.
    #[builder(finish_fn = build)]
    pub fn with_filter_dedup(
        #[builder(start_fn)] mut callback: impl GuiProcessCallback,
        #[builder(default)] create_only: bool,
        mut filter: impl FnMut(GuiProcessEvent) -> bool + Send + 'static,
        start_time_filter: Option<SystemTime>,
        /// Call `callback` with every process and skip them afterwards.
        existing_processes: Option<HashMap<Pid, SystemTime>>,
    ) -> windows::core::Result<Self> {
        // To deal with PID conflict
        let mut dedup = match existing_processes {
            Some(processes) => {
                processes
                    .keys()
                    .for_each(|&pid| callback(GuiProcessEvent::CreateOrAlive(pid)));
                processes
            }
            None => Default::default(),
        };
        /*
        let shell_callback = move |msg: ShellHookMessage| {
            match msg {
                ShellHookMessage::WindowCreated(hwnd)
                | ShellHookMessage::WindowActivated(hwnd)
                | ShellHookMessage::RudeAppActivated(hwnd)
                | ShellHookMessage::WindowReplacing(hwnd) => {
                    if let Ok((pid, tid)) = Pid::from_hwnd_with_thread(hwnd) {
                        debug!(%pid, tid);
                        if filter(GuiProcessEvent::Alive(pid)) {
                            dedup
                                .entry(pid)
                                .and_modify(|old_tid| {
                                    if *old_tid != tid {
                                        match Pid::from_tid(*old_tid) {
                                            // The same process with new GUI thread
                                            Ok(new_pid) if new_pid == pid => (),
                                            // New thread with the same TID from new process
                                            Ok(_) => {
                                                ()
                                            }
                                            // Old thread died
                                            Err(_) => {
                                                // callback(GuiProcessEvent::Alive(pid));
                                                // *old_tid = tid;
                                                ()
                                            }
                                        }
                                    }
                                })
                                .or_insert_with(|| {
                                    callback(GuiProcessEvent::Alive(pid));
                                    tid
                                });
                        }
                    }
                }
                _ => (),
            }
            false
        };
        let shell = ShellHook::new(Box::new(shell_callback))?;
        Ok(GuiProcessWatcher { _shell: shell })
        */

        let callback = move |event: GuiProcessEvent| {
            if (!create_only || matches!(event, GuiProcessEvent::CreateOrAlive(_))) && filter(event)
            {
                let pid = event.pid();
                // We need start_time to deal with PID conflict
                let start_time = pid.get_start_time_or_max();
                if start_time_filter.is_none_or(|f| start_time >= f) {
                    dedup
                        .entry(pid)
                        .and_modify(|old_start_time| {
                            if *old_start_time != start_time {
                                callback(event);
                                *old_start_time = start_time;
                            }
                        })
                        .or_insert_with(|| {
                            callback(event);
                            start_time
                        });
                }
            }
        };
        Self::new(callback)
    }
}

#[cfg(feature = "sysinfo")]
#[bon]
impl GuiProcessWatcher {
    /**
    Apply a function on every existing and new GUI process exactly once.

    Race condition / TOCTOU is handled in this function, although not perfect.
    (Processes created after `start_time` before hooked will be lost,
    but they can still be detected if they create new windows (and activate windows if `create_only` is `false`)
    in the future, which is likely to happen.)

    ## Examples
    ```no_run
    use ib_hook::process::GuiProcessWatcher;

    let _watcher = GuiProcessWatcher::for_each(|pid| println!("pid: {pid}"))
        .filter_image_path(|path| {
            path.and_then(|p| p.file_name())
                .is_some_and(|n| n.to_ascii_lowercase() == "notepad.exe")
        })
        .build();
    std::thread::sleep(std::time::Duration::from_secs(60));
    ```
    */
    #[builder(finish_fn = build)]
    pub fn for_each(
        #[builder(start_fn)] mut f: impl FnMut(Pid) + Send + 'static,
        mut filter_image_path: impl FnMut(Option<&Path>) -> bool + Send + 'static,
        /// Mitigate TOCTOU issue further at the cost of some system performance.
        #[builder(default = true)]
        create_only: bool,
    ) -> windows::core::Result<Self> {
        let start_time = SystemTime::now();

        // TODO: Filter GUI processes?
        // TODO: Avoid using sysinfo for this
        let mut system = sysinfo::System::new();
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::nothing().with_exe(sysinfo::UpdateKind::Always),
        );
        let processes = system
            .processes()
            .values()
            .filter(|process| filter_image_path(process.exe()))
            .map(|process| process.pid().into())
            .map(|pid: Pid| (pid, pid.get_start_time_or_max()))
            .collect();

        let watcher = {
            Self::with_filter_dedup(move |event| {
                let pid = event.pid();
                if filter_image_path(pid.image_path().as_deref()) {
                    f(pid)
                }
            })
            .create_only(create_only)
            .filter(|_| true)
            .start_time_filter(start_time)
            .existing_processes(processes)
            .build()?
        };

        Ok(watcher)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        thread,
        time::Duration,
    };

    fn test_gui_process_watcher(d: Duration) {
        println!("Testing GuiProcessWatcher - open/close some apps to see events");

        let count = std::sync::Arc::new(AtomicUsize::new(0));

        // Clone the Arc before moving into the closure
        let count_result = count.clone();

        let watcher = GuiProcessWatcher::new(Box::new(move |event: GuiProcessEvent| {
            println!("Process event: {event:?}");
            let pid = event.pid();
            let count = count.fetch_add(1, Ordering::SeqCst);
            println!("[{}] Process alive: {}", count + 1, pid);
        }))
        .expect("Failed to create GUI process watcher");

        println!("GUI process watcher registered");
        println!("Test will complete in {d:?} seconds...\n");

        // Keep the watcher alive for a bit to receive events
        thread::sleep(d);

        // Drop watcher explicitly to demonstrate cleanup
        drop(watcher);
        println!("\nGUI process watcher destroyed.");
        println!("Total events: {}", count_result.load(Ordering::SeqCst));
    }

    #[test]
    fn gui_process_watcher() {
        test_gui_process_watcher(Duration::from_secs(1))
    }

    #[ignore]
    #[test]
    fn gui_process_watcher_manual() {
        test_gui_process_watcher(Duration::from_secs(30))
    }

    fn test_gui_process_watcher_dedup(d: Duration) {
        println!("\nTesting GuiProcessWatcher with dedup - open/close some apps");

        let count = std::sync::Arc::new(AtomicUsize::new(0));

        // Clone the Arc before moving into the closure
        let count_result = count.clone();

        let watcher = GuiProcessWatcher::with_dedup(Box::new(move |event: GuiProcessEvent| {
            println!("Process event: {event:?}");
            let pid = event.pid();
            let count = count.fetch_add(1, Ordering::SeqCst);
            println!("[{}] Process alive (dedup): {}", count + 1, pid);
        }))
        .expect("Failed to create GUI process watcher with dedup");

        println!("GUI process watcher with dedup registered");
        println!("Test will complete in {d:?} seconds...\n");

        thread::sleep(d);
        drop(watcher);
        println!("Total events: {}", count_result.load(Ordering::SeqCst));
    }

    #[test]
    fn gui_process_watcher_dedup() {
        test_gui_process_watcher_dedup(Duration::from_secs(1));
    }

    #[ignore]
    #[test_log::test]
    #[test_log(default_log_filter = "trace")]
    fn gui_process_watcher_dedup_manual() {
        test_gui_process_watcher_dedup(Duration::from_secs(60));
    }

    #[cfg(feature = "sysinfo")]
    fn test_for_each(d: Duration) {
        let _watcher = GuiProcessWatcher::for_each(|pid| println!("pid: {pid}"))
            .filter_image_path(|path| {
                path.and_then(|p| p.file_name())
                    .is_some_and(|n| n.to_ascii_lowercase() == "notepad.exe")
            })
            .build();
        thread::sleep(d);
    }

    #[cfg(feature = "sysinfo")]
    #[test]
    fn for_each() {
        test_for_each(Duration::from_secs(1));
    }

    #[cfg(feature = "sysinfo")]
    #[ignore]
    #[test]
    fn for_each_manual() {
        test_for_each(Duration::from_secs(60));
    }
}
