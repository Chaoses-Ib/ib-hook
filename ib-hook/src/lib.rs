/*!
Windows binary and system hooking library.

Supported hooks:
- [Windows shell hook (`WH_SHELL`)](#windows-shell-hook-wh_shell)

## Windows shell hook (`WH_SHELL`)
Applications:
- Monitor window operations: creating, activating, title redrawing, monitor changing...
- Monitor GUI processes

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
*/
pub mod windows;
