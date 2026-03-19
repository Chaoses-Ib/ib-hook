# ib-hook
[![crates.io](https://img.shields.io/crates/v/ib-hook.svg)](https://crates.io/crates/ib-hook)
[![Documentation](https://docs.rs/ib-hook/badge.svg)](https://docs.rs/ib-hook)
[![License](https://img.shields.io/crates/l/ib-hook.svg)](../LICENSE.txt)

A Rust library for Windows binary and system hooking.

Supported hooks:
- [Windows shell hook (`WH_SHELL`)](#windows-shell-hook-wh_shell)

## Windows shell hook (`WH_SHELL`)
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
