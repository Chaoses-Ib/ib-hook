/*!
Windows binary and system hooking library.

Features:
- [Inline hooking](#inline-hooking):
  Hook functions on x86/x64/ARM64, `no_std` and `Ntdll.dll` only.
- [DLL injection](#dll-injection):
  Inject DLL into processes with optional RPC and auto self unload.
- [Windows shell hook (`WH_SHELL`)](#windows-shell-hook-wh_shell):
  Monitor window operations: creating, activating, title redrawing, monitor changing...
- [GUI process watcher](#gui-process-watcher):
  Monitor GUI processes.

## Inline hooking
- Supported CPU architectures: x86, x64, ARM64.
- `no_std` and depend on `Ntdll.dll` only.

See [`inline`] module for more details. Here is a quick example:
```
// cargo add ib-hook --features inline
use ib_hook::inline::InlineHook;

extern "system" fn original(x: u32) -> u32 { x + 1 }

// Hook the function with a detour
extern "system" fn hooked(x: u32) -> u32 { x + 0o721 }
let mut hook = InlineHook::<extern "system" fn(u32) -> u32>::new_enabled(original, hooked).unwrap();

// Now calls to original are redirected to hooked
assert_eq!(original(0x100), 721); // redirected to hooked: 0x100 + 0o721 = 721

// Access original via trampoline
assert_eq!(hook.trampoline()(0x100), 0x101); // 0x100 + 1

// Disable the hook manually (or automatically on drop)
hook.disable().unwrap();
assert_eq!(original(0x100), 0x101); // back to original
```

## DLL injection
Inject DLL into processes with optional RPC and auto self unload.

- Optional RPC with `serde` input and output.
- RAII (drop guard) design with optional `leak()`.
- Single DLL injection / Multiple DLL injection manager.
- Optioanlly, in the DLL, unload self automatically if the injector process aborted.

See [`inject::dll`] module for more details. Here is a quick example:
```no_run
use ib_hook::inject::dll::app::{DllApp, DllInjectionVec};

struct MyDll;
impl DllApp for MyDll {
    const APPLY: &str = "apply_hook";
    type Input = String;
    type Output = ();
}

// Inject into all processes named Notepad.exe
let mut injections = DllInjectionVec::<MyDll>::new();
injections.inject_with_process_name("Notepad.exe")
    .dll_path(std::path::Path::new("hook.dll"))
    .apply(&"input".into())
    .on_error(|pid, err| ())
    .call()
    .unwrap();

// Eject all manually or let drop handle it
injections.eject().on_error(|pid, err| ()).call();
```

## Windows shell hook (`WH_SHELL`)
Monitor window operations: creating, activating, title redrawing, monitor changing...

See [`windows::shell`] module for more details. Here is a quick example:
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
See also [ib-shell: Some desktop environment libraries, mainly for Windows Shell.](https://github.com/Chaoses-Ib/ib-shell)

## GUI process watcher
Monitor GUI processes.

See [`process`] module for more details. Here is a quick example:
```no_run
use ib_hook::process::{GuiProcessEvent, GuiProcessWatcher};

let watcher = GuiProcessWatcher::new(Box::new(|event| {
    println!("Process event: {event:?}");
})).unwrap();

println!("Monitoring GUI processes...");
std::thread::sleep(std::time::Duration::from_secs(60));
```

Apply a function on every existing and new GUI process exactly once:
```no_run
// cargo add ib-hook --features sysinfo
use ib_hook::process::GuiProcessWatcher;

let _watcher = GuiProcessWatcher::for_each(|pid| println!("pid: {pid}"))
    .filter_image_path(|path| {
        path.and_then(|p| p.file_name())
            .is_some_and(|n| n.to_ascii_lowercase() == "notepad.exe")
    })
    .build();
std::thread::sleep(std::time::Duration::from_secs(60));
```

## Crate features
*/
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(feature = "doc", doc = document_features::document_features!())]
#![cfg_attr(not(any(feature = "std", test)), no_std)]
pub mod inject;
#[cfg(feature = "inline")]
pub mod inline;
mod log;
pub mod process;
pub mod windows;

/// Marker trait for [function pointers (`fn`)](https://doc.rust-lang.org/nightly/core/primitive.fn.html).
pub trait FnPtr:
    PartialEq
    + Eq
    + PartialOrd
    + Ord
    + core::hash::Hash
    + core::fmt::Pointer
    + core::fmt::Debug
    + Clone
    + Copy
    + Send
    + Sync
    + Unpin
    + core::panic::UnwindSafe
    + core::panic::RefUnwindSafe
    + Sized
    + 'static
{
}

impl<F> FnPtr for F where
    F: PartialEq
        + Eq
        + PartialOrd
        + Ord
        + core::hash::Hash
        + core::fmt::Pointer
        + core::fmt::Debug
        + Clone
        + Copy
        + Send
        + Sync
        + Unpin
        + core::panic::UnwindSafe
        + core::panic::RefUnwindSafe
        + Sized
        + 'static
{
}
