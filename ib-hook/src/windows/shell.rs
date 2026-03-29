/*!
Monitor window operations: creating, activating, title redrawing, monitor changing...

Installs a hook procedure that receives notifications useful to Windows shell applications.

See:
- [ShellProc callback function](https://learn.microsoft.com/en-us/windows/win32/winmsg/shellproc#parameters)
- [RegisterShellHookWindow function (winuser.h)](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registershellhookwindow#remarks)
- [Shell Events (a.k.a. Shell Hooks) - zhuman/ShellReplacement](https://github.com/zhuman/ShellReplacement/blob/master/wiki/ShellEvents.md)

## Examples
```no_run
use ib_hook::windows::shell::{ShellHook, ShellHookMessage};
{
    let hook = ShellHook::new(Box::new(|msg: ShellHookMessage| {
        println!("{msg:?}");
        false
    }))
    .unwrap();

    // Perform window operations to see received events...
    std::thread::sleep(std::time::Duration::from_secs(30));
}
// Shell hook unregistered
```

## Disclaimer
Ref:
- https://github.com/YousefAliUK/FerroDock/blob/b405832a64c763f073b37d9a42a0690d0c15416b/src/events.rs
- https://gist.github.com/Aetopia/347e7329158aa2c69df97bdf0b761d6f
*/
use std::sync::{Once, OnceLock};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DeregisterShellHookWindow, DestroyWindow,
        DispatchMessageW, GWLP_USERDATA, GetMessageW, GetWindowLongPtrW, HWND_MESSAGE, MSG,
        RegisterClassW, RegisterShellHookWindow, RegisterWindowMessageW, SHELLHOOKINFO,
        SetWindowLongPtrW, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW,
    },
};
use windows::core::w;

use crate::{log::*, process::module::Module};

pub use windows::Win32::UI::WindowsAndMessaging::{
    HSHELL_ACCESSIBILITYSTATE, HSHELL_ACTIVATESHELLWINDOW, HSHELL_APPCOMMAND, HSHELL_ENDTASK,
    HSHELL_GETMINRECT, HSHELL_HIGHBIT, HSHELL_LANGUAGE, HSHELL_MONITORCHANGED, HSHELL_REDRAW,
    HSHELL_SYSMENU, HSHELL_TASKMAN, HSHELL_WINDOWACTIVATED, HSHELL_WINDOWCREATED,
    HSHELL_WINDOWDESTROYED, HSHELL_WINDOWREPLACED, HSHELL_WINDOWREPLACING,
};

// Missing shell hook constants from the windows crate
pub const HSHELL_RUDEAPPACTIVATED: u32 = HSHELL_WINDOWACTIVATED | HSHELL_HIGHBIT;
pub const HSHELL_FLASH: u32 = HSHELL_REDRAW | HSHELL_HIGHBIT;

/// The return value should be `false` unless the message is [`ShellHookMessage::AppCommand`]
/// and the callback handles the [`WM_COMMAND`] message. In this case, the return should be `true`.
pub type ShellHookCallback = dyn FnMut(ShellHookMessage) -> bool + Send + 'static;

/// Shell hook message variants.
///
/// These correspond to the shell hook messages sent via [`RegisterShellHookWindow`].
///
/// Ref:
/// - [ShellProc callback function - Win32 apps | Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/winmsg/shellproc#parameters)
/// - [RegisterShellHookWindow function (winuser.h) - Win32 apps | Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registershellhookwindow#remarks)
#[derive(Debug, Clone, Copy)]
pub enum ShellHookMessage {
    /// A top-level, unowned window has been created.
    /// The window exists when the system calls this hook.
    ///
    /// A handle to the window being created.
    WindowCreated(HWND),

    /// A top-level, unowned window is about to be destroyed.
    /// The window still exists when the system calls this hook.
    ///
    /// A handle to the top-level window being destroyed.
    WindowDestroyed(HWND),

    /// The shell should activate its main window.
    ActivateShellWindow,

    /// The activation has changed to a different top-level, unowned window.
    ///
    /// A handle to the activated window.
    WindowActivated(HWND),

    /// The activation has changed to a different top-level, unowned window in full-screen mode.
    ///
    /// A handle to the activated window.
    ///
    /// Ref: [c# - Does anybody know what means ShellHook message `HSHELL_RUDEAPPACTIVATED`? - Stack Overflow](https://stackoverflow.com/questions/1178020/does-anybody-know-what-means-shellhook-message-hshell-rudeappactivated)
    RudeAppActivated(HWND),

    /// A window is being minimized or maximized.
    /// The system needs the coordinates of the minimized rectangle for the window.
    ///
    /// - A handle to the minimized or maximized window.
    /// - A pointer to a RECT structure.
    GetMinRect(HWND, RECT),

    /// The user has selected the task list.
    /// A shell application that provides a task list should return `TRUE` to prevent Windows from starting its task list.
    ///
    /// The param can be ignored.
    TaskMan(LPARAM),

    /// Keyboard language was changed or a new keyboard layout was loaded.
    ///
    /// - A handle to the window.
    /// - A handle to a keyboard layout.
    ///
    /// May require DLL hook.
    Language(HWND),

    /// May require DLL hook.
    SysMenu(LPARAM),

    /// A handle to the window that should be forced to exit.
    EndTask(HWND),

    /// The accessibility state has changed.
    ///
    /// Indicates which accessibility feature has changed state.
    /// This value is one of the following: `ACCESS_FILTERKEYS`, `ACCESS_MOUSEKEYS`, or `ACCESS_STICKYKEYS`.
    ///
    /// May require DLL hook.
    AccessibilityState(LPARAM),

    /// The title of a window in the task bar has been redrawn.
    ///
    /// A handle to the window that needs to be redrawn.
    Redraw(HWND),

    /// A handle to the window that needs to be flashed.
    Flash(HWND),

    /// The user completed an input event (for example, pressed an application command button on the mouse or an application command key on the keyboard),
    /// and the application did not handle the [`WM_APPCOMMAND`] message generated by that input.
    ///
    /// - The [`APPCOMMAND`] which has been unhandled by the application or other hooks.
    AppCommand(LPARAM),

    /// A top-level window is being replaced.
    /// The window exists when the system calls this hook.
    ///
    /// A handle to the window being replaced.
    WindowReplaced(HWND),

    /// A handle to the window replacing the top-level window.
    WindowReplacing(HWND),

    /// A handle to the window that moved to a different monitor.
    MonitorChanged(HWND),

    /// Unknown shell hook message.
    Unknown(WPARAM, LPARAM),
}

impl From<(WPARAM, LPARAM)> for ShellHookMessage {
    fn from(value: (WPARAM, LPARAM)) -> Self {
        let (wparam, lparam) = value;
        match wparam.0 as u32 {
            HSHELL_WINDOWCREATED => Self::WindowCreated(HWND(lparam.0 as _)),
            HSHELL_WINDOWDESTROYED => Self::WindowDestroyed(HWND(lparam.0 as _)),
            HSHELL_ACTIVATESHELLWINDOW => Self::ActivateShellWindow,
            HSHELL_WINDOWACTIVATED => Self::WindowActivated(HWND(lparam.0 as _)),
            HSHELL_RUDEAPPACTIVATED => Self::RudeAppActivated(HWND(lparam.0 as _)),
            HSHELL_GETMINRECT => {
                let info = unsafe { &*(lparam.0 as *const SHELLHOOKINFO) };
                Self::GetMinRect(info.hwnd, info.rc)
            }
            HSHELL_TASKMAN => Self::TaskMan(lparam),
            HSHELL_LANGUAGE => Self::Language(HWND(lparam.0 as _)),
            HSHELL_SYSMENU => Self::SysMenu(lparam),
            HSHELL_ENDTASK => Self::EndTask(HWND(lparam.0 as _)),
            HSHELL_ACCESSIBILITYSTATE => Self::AccessibilityState(lparam),
            HSHELL_REDRAW => Self::Redraw(HWND(lparam.0 as _)),
            HSHELL_FLASH => Self::Flash(HWND(lparam.0 as _)),
            HSHELL_APPCOMMAND => Self::AppCommand(lparam),
            HSHELL_WINDOWREPLACED => Self::WindowReplaced(HWND(lparam.0 as _)),
            HSHELL_WINDOWREPLACING => Self::WindowReplacing(HWND(lparam.0 as _)),
            HSHELL_MONITORCHANGED => Self::MonitorChanged(HWND(lparam.0 as _)),
            _ => Self::Unknown(wparam, lparam),
        }
    }
}

pub struct ShellHook {
    _thread: Option<std::thread::JoinHandle<()>>,
    hwnd: OnceLock<usize>,
}

impl ShellHook {
    pub fn new(callback: Box<ShellHookCallback>) -> windows::core::Result<Self> {
        Self::with_on_hooked(callback, |_| ())
    }

    pub fn with_on_hooked(
        mut callback: Box<ShellHookCallback>,
        on_hooked: impl FnOnce(&mut ShellHookCallback) + Send + 'static,
    ) -> windows::core::Result<Self> {
        let hwnd = OnceLock::new();

        // Start the message loop in a separate thread
        let _thread = std::thread::spawn({
            let hwnd_store = hwnd.clone();
            move || {
                /*
                let shell_msg = unsafe { RegisterWindowMessageW(w!("SHELLHOOK")) };
                SHELL_HOOK_MSG.set(shell_msg).ok();
                */

                let class_name = w!("ib_hook::shell");

                let wc = WNDCLASSW {
                    lpfnWndProc: Some(window_proc),
                    hInstance: Module::current().0.into(),
                    lpszClassName: class_name,
                    ..Default::default()
                };

                // Only `RegisterClass` once
                CLASS_REGISTER.call_once(|| {
                    if unsafe { RegisterClassW(&wc) } == 0 {
                        error!("Failed to register window class");
                    }
                });

                let hwnd = unsafe {
                    CreateWindowExW(
                        WINDOW_EX_STYLE::default(),
                        class_name,
                        w!("ShellHookWindow"),
                        WINDOW_STYLE::default(),
                        0,
                        0,
                        0,
                        0,
                        // Message-only window
                        Some(HWND_MESSAGE),
                        None,
                        Some(wc.hInstance),
                        None,
                    )
                }
                .unwrap();

                if hwnd.0.is_null() {
                    error!("Failed to create shell hook window");
                    return;
                }

                // Store hwnd as usize in OnceLock
                let _ = hwnd_store.set(hwnd.0 as usize);

                // Set callback in window user data
                let callback_ref = callback.as_mut() as *mut _;
                let callback_ptr = Box::into_raw(Box::new(callback)) as isize;
                unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, callback_ptr) };

                if !unsafe { RegisterShellHookWindow(hwnd) }.as_bool() {
                    error!("Failed to register shell hook window");
                    return;
                }

                debug!("Shell hook window created: {:?}", hwnd);

                // SAFETY: Callback will only be called in DispatchMessageW()
                on_hooked(unsafe { &mut *callback_ref });

                // Run the message loop
                let mut msg = MSG::default();
                while unsafe { GetMessageW(&mut msg, None, 0, 0).as_bool() } {
                    let _ = unsafe { TranslateMessage(&msg) };
                    let _ = unsafe { DispatchMessageW(&msg) };
                }
            }
        });

        Ok(ShellHook {
            _thread: Some(_thread),
            hwnd,
        })
    }

    pub fn hwnd(&self) -> Option<HWND> {
        self.hwnd.get().map(|&h| HWND(h as _))
    }
}

impl Drop for ShellHook {
    fn drop(&mut self) {
        if let Some(hwnd) = self.hwnd() {
            // Unregister from shell hook messages
            _ = unsafe { DeregisterShellHookWindow(hwnd) };

            // Clean up the callback
            unsafe {
                let callback_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if callback_ptr != 0 {
                    _ = Box::from_raw(callback_ptr as *mut Box<ShellHookCallback>);
                }
            }

            // Destroy the window
            _ = unsafe { DestroyWindow(hwnd) };
        }
    }
}

static SHELL_HOOK_MSG: OnceLock<u32> = OnceLock::new();

fn shell_hook_msg() -> u32 {
    *SHELL_HOOK_MSG.get_or_init(|| unsafe { RegisterWindowMessageW(w!("SHELLHOOK")) })
}

static CLASS_REGISTER: Once = Once::new();

/// The return value should be zero unless the value of nCode is [`HSHELL_APPCOMMAND`]
/// and the shell procedure handles the [`WM_COMMAND`] message. In this case, the return should be nonzero.
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == shell_hook_msg() {
        let callback = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
        if callback != 0 {
            let callback = unsafe { &mut *(callback as *mut Box<ShellHookCallback>) };
            let r = callback(ShellHookMessage::from((wparam, lparam)));
            return LRESULT(r as _);
        }
    }

    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Duration};

    #[test]
    fn shell_hook() {
        println!("Testing ShellHook - perform various window operations to see events");

        let hook = ShellHook::new(Box::new(|msg: ShellHookMessage| {
            println!("{msg:?}");
            false
        }))
        .expect("Failed to create shell hook");

        println!("Shell hook registered with hwnd={:?}", hook.hwnd());
        println!("Test will complete in 1 seconds...\n");

        // Keep the hook alive for a bit to receive events
        thread::sleep(Duration::from_secs(1));

        // Drop hook explicitly to demonstrate cleanup
        drop(hook);
        println!("\nShell hook destroyed.");
    }

    #[ignore]
    #[test]
    fn shell_hook_manual() {
        println!("Testing ShellHook - perform various window operations to see events");

        let hook = ShellHook::new(Box::new(|msg: ShellHookMessage| {
            println!("{msg:?}");
            false
        }))
        .expect("Failed to create shell hook");

        println!("Shell hook registered with hwnd={:?}", hook.hwnd());
        println!("Perform window operations (open/close apps, alt+tab, etc.) to see events...");
        println!("Test will complete in 30 seconds...\n");

        // Keep the hook alive for a bit to receive events
        thread::sleep(Duration::from_secs(30));

        // Drop hook explicitly to demonstrate cleanup
        drop(hook);
        println!("\nShell hook destroyed.");
    }
}
