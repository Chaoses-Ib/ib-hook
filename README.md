# ib-hook
Windows binary and system hooking Rust/C libraries.

## Features
- [Inline hooking](#inline-hooking):
  Hook functions on x86/x64/ARM64, `no_std` and `Ntdll.dll` only.
- [DLL injection](#dll-injection):
  Inject DLL into processes with optional RPC and auto self unload.
- [Windows shell hook (`WH_SHELL`)](#windows-shell-hook-wh_shell):
  Monitor window operations: creating, activating, title redrawing, monitor changing...
- [GUI process watcher](#gui-process-watcher):
  Monitor GUI processes.
- [DLL hijacking](#ib-dll-hijack-c):
  Inject DLL by hijacking load.

## [ib-hook](ib-hook/README.md)
[![crates.io](https://img.shields.io/crates/v/ib-hook.svg)](https://crates.io/crates/ib-hook)
[![Documentation](https://docs.rs/ib-hook/badge.svg)](https://docs.rs/ib-hook)
[![License](https://img.shields.io/crates/l/ib-hook.svg)](LICENSE.txt)

A Rust library for Windows binary and system hooking.

See [documentation](https://docs.rs/ib-hook) for details.

### Inline hooking
- Supported CPU architectures: x86, x64, ARM64.
- `no_std` and depend on `Ntdll.dll` only.

```rust
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

### DLL injection
Inject DLL into processes with optional RPC and auto self unload.

- Optional RPC with `serde` input and output.
- RAII (drop guard) design with optional `leak()`.
- Single DLL injection / Multiple DLL injection manager.
- Optioanlly, in the DLL, unload self automatically if the injector process aborted.

```rust
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

See [`src/bin/inject-app-dll.rs`](https://github.com/Chaoses-Ib/ib-hook/blob/master/ib-hook/src/bin/inject-app-dll.rs)
and [`examples/app-dll.rs`](https://github.com/Chaoses-Ib/ib-hook/blob/master/ib-hook/examples/app-dll.rs)
for a complete example.

### Windows shell hook (`WH_SHELL`)
Monitor window operations: creating, activating, title redrawing, monitor changing...

```rust
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

### GUI process watcher
Monitor GUI processes.

```rust
use ib_hook::process::{GuiProcessEvent, GuiProcessWatcher};

let watcher = GuiProcessWatcher::new(Box::new(|event| {
    println!("Process event: {event:?}");
})).unwrap();

println!("Monitoring GUI processes...");
std::thread::sleep(std::time::Duration::from_secs(60));
```

Apply a function on every existing and new GUI process exactly once:
```rust
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

## [ib-dll-hijack-c](ib-dll-hijack-c/README.md)
A C library for Windows DLL hijacking.

Usage:
```cpp
// Export functions of version.dll (by export forwarding)
#include <IbDllHijack/dlls/version.h>

BOOL APIENTRY DllMain( HMODULE hModule,
                       DWORD  ul_reason_for_call,
                       LPVOID lpReserved
                     )
{
    switch (ul_reason_for_call)
    {
    case DLL_PROCESS_ATTACH:
    case DLL_THREAD_ATTACH:
    case DLL_THREAD_DETACH:
    case DLL_PROCESS_DETACH:
        break;
    }
    return TRUE;
}
```

You can use the [generator](ib-dll-hijack-c/generator/README.md) to generate header files for any DLL.

## Projects using this library
- [ib-shell: Some desktop environment libraries, mainly for Windows Shell (Windows' built-in desktop environment).](https://github.com/Chaoses-Ib/ib-shell)
- [IbEverythingExt: Everything 拼音搜索, ローマ字検索, wildcard, quick select, Shell extension](https://github.com/Chaoses-Ib/IbEverythingExt)
- [IbDOpusExt: An extension for Directory Opus.](https://github.com/Chaoses-Ib/IbDOpusExt)
- [IbLogiSoftExt: An extension for Logitech Gaming Software. Support sending G-keys to AutoHotkey.](https://github.com/Chaoses-Ib/IbLogiSoftExt)
- [IbOneNoteExt: An extension for Microsoft OneNote. Support changing font Calibri to Microsoft YaHei.](https://github.com/Chaoses-Ib/IbOneNoteExt)
