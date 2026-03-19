# ib-hook
Windows binary and system hooking Rust/C libraries.

Supported hooks:
- [Windows DLL hijacking](#ib-dll-hijack-c)
- [Windows shell hook (`WH_SHELL`)](#windows-shell-hook-wh_shell)

## [ib-hook](ib-hook/README.md)
[![crates.io](https://img.shields.io/crates/v/ib-hook.svg)](https://crates.io/crates/ib-hook)
[![Documentation](https://docs.rs/ib-hook/badge.svg)](https://docs.rs/ib-hook)
[![License](https://img.shields.io/crates/l/ib-hook.svg)](LICENSE.txt)

A Rust library for Windows binary and system hooking.

### Windows shell hook (`WH_SHELL`)
Applications:
- Monitor window operations: creating, activating, title redrawing, monitor changing...
- Monitor GUI processes

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
* [IbDOpusExt](https://github.com/Chaoses-Ib/IbDOpusExt)
* [IbEverythingExt](https://github.com/Chaoses-Ib/IbEverythingExt)
* [IbLogiSoftExt](https://github.com/Chaoses-Ib/IbLogiSoftExt)
* [IbOneNoteExt](https://github.com/Chaoses-Ib/IbOneNoteExt)