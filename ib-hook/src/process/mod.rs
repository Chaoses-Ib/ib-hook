/*!
Process utilities.
*/
#[cfg(feature = "sysinfo")]
use std::path::PathBuf;
use std::time::SystemTime;

use derive_more::{Deref, Display};
use windows::Win32::{
    Foundation::{GetLastError, HWND, WIN32_ERROR},
    System::Threading::{
        GetProcessIdOfThread, GetProcessTimes, OpenProcess, OpenThread,
        PROCESS_QUERY_LIMITED_INFORMATION, THREAD_QUERY_LIMITED_INFORMATION,
    },
    UI::WindowsAndMessaging::GetWindowThreadProcessId,
};

use crate::log::*;

mod gui;
pub mod module;

pub use gui::*;

/// Process ID.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Display, Debug, Deref)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pid(pub u32);

impl Pid {
    pub fn current() -> Self {
        Self(std::process::id())
    }

    pub fn from_tid(tid: u32) -> windows::core::Result<Self> {
        let thread = unsafe { OpenThread(THREAD_QUERY_LIMITED_INFORMATION, false, tid) }?;
        match unsafe { GetProcessIdOfThread(thread) } {
            0 => Err(windows::core::Error::from_thread()),
            pid => Ok(Pid(pid)),
        }
    }

    fn from_hwnd_with_thread(hwnd: HWND) -> Result<(Self, u32), WIN32_ERROR> {
        let mut pid: u32 = 0;
        let tid = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if tid != 0 {
            Ok((Pid(pid), tid))
        } else {
            Err(unsafe { GetLastError() })
        }
    }

    /// Gets the process ID from a window handle.
    ///
    /// This uses [`GetWindowThreadProcessId`] to retrieve the PID associated with a window.
    pub fn from_hwnd(hwnd: HWND) -> Result<Self, WIN32_ERROR> {
        Self::try_from(hwnd)
    }
}

impl TryFrom<HWND> for Pid {
    type Error = WIN32_ERROR;

    /// Gets the process ID from a window handle.
    ///
    /// This uses [`GetWindowThreadProcessId`] to retrieve the PID associated with a window.
    fn try_from(hwnd: HWND) -> Result<Self, Self::Error> {
        Self::from_hwnd_with_thread(hwnd).map(|(pid, _tid)| pid)
    }
}

impl Pid {
    /// Gets the start time of the process.
    pub fn get_start_time(self) -> windows::core::Result<SystemTime> {
        let pid = self.0;
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }?;

        let mut start: nt_time::FileTime = Default::default();
        let mut x = Default::default();
        unsafe { GetProcessTimes(handle, &mut start as *mut _ as _, &mut x, &mut x, &mut x) }?;

        Ok(start.into())
    }

    /// Gets the start time of the process.
    ///
    /// [`#![feature(time_systemtime_limits)]`](https://github.com/rust-lang/rust/issues/149067) is not stable yet.
    pub fn get_start_time_or_max(self) -> SystemTime {
        self.get_start_time()
            .inspect_err(|e| debug!(%e, "get_start_time"))
            .unwrap_or_else(|_| nt_time::FileTime::MAX.into())
    }
}

#[cfg(feature = "sysinfo")]
impl Pid {
    pub fn with_process<R>(
        self,
        refresh_info: sysinfo::ProcessRefreshKind,
        f: impl FnOnce(&sysinfo::Process) -> R,
    ) -> Option<R> {
        let pid = self.clone().into();
        let mut system = sysinfo::System::new();
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[pid]),
            false,
            refresh_info,
        );
        system.process(pid).map(f)
    }

    pub fn image_path(self) -> Option<PathBuf> {
        self.with_process(
            sysinfo::ProcessRefreshKind::nothing().with_exe(sysinfo::UpdateKind::Always),
            |p| p.exe().map(|p| p.to_owned()),
        )
        .flatten()
    }
}

#[cfg(feature = "sysinfo")]
impl From<sysinfo::Pid> for Pid {
    fn from(pid: sysinfo::Pid) -> Self {
        Self(pid.as_u32())
    }
}

#[cfg(feature = "sysinfo")]
impl Into<sysinfo::Pid> for Pid {
    fn into(self) -> sysinfo::Pid {
        sysinfo::Pid::from_u32(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;

    #[test]
    fn pid_from_hwnd() {
        // Test with a simple window - use GetDesktopWindow as it always exists
        let desktop = unsafe { GetDesktopWindow() };
        let pid = Pid::from_hwnd(desktop);

        // Desktop window should have a valid PID (usually 0 or the session manager)
        assert!(pid.is_ok(), "Should be able to get PID from desktop window");
        println!("Desktop window PID: {:?}", pid.unwrap());
    }

    #[test]
    #[cfg(feature = "sysinfo")]
    fn pid_from_hwnd_desktop() {
        let desktop = unsafe { GetDesktopWindow() };
        dbg!(desktop);
        let pid = Pid::from_hwnd(desktop).expect("Should get PID from desktop window");
        dbg!(pid);

        let exe_path = pid.image_path();
        // Should be C:\Windows\System32\csrss.exe if permission enough
        assert_eq!(exe_path, None);
    }

    /// Test with GetDesktopWindow and verify the process is explorer.exe
    #[test]
    #[cfg(feature = "sysinfo")]
    fn pid_from_hwnd_shell() {
        use std::path::Path;
        use windows::Win32::UI::WindowsAndMessaging::GetShellWindow;

        let desktop = unsafe { GetShellWindow() };
        dbg!(desktop);
        let pid = Pid::from_hwnd(desktop).expect("Should get PID from desktop window");
        dbg!(pid);

        // The desktop window is typically owned by explorer.exe
        let exe_path = pid.image_path().unwrap();
        assert_eq!(exe_path, Path::new(r"C:\Windows\explorer.exe"));
    }

    #[test]
    fn get_start_time() {
        let current_pid = std::process::id();
        let pid = Pid(current_pid);

        let start_time = pid.get_start_time().unwrap();
        dbg!(start_time);
        let t = start_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(t > 0, "{t}");

        let start_time2 = pid.get_start_time().unwrap();
        dbg!(start_time2);
        assert_eq!(start_time, start_time2);
    }
}
