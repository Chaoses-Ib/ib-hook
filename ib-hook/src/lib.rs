/*!
Windows binary and system hooking library.

Features:
- [Windows shell hook (`WH_SHELL`)](#windows-shell-hook-wh_shell)
- [GUI process watcher](#gui-process-watcher)

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

## GUI process watcher
```no_run
use ib_hook::windows::process::{GuiProcessEvent, GuiProcessWatcher};

let watcher = GuiProcessWatcher::new(Box::new(|event| {
    println!("Process event: {event:?}");
})).unwrap();

println!("Monitoring GUI processes...");
std::thread::sleep(std::time::Duration::from_secs(60));
```
*/
pub mod windows;
