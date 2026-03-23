/*!
Inject DLL into processes with optional RPC and auto self unload.

- Optional RPC with `serde` input and output.
- RAII (drop guard) design with optional `leak()`.
- Single DLL injection / Multiple DLL injection manager.
- Optioanlly, in the DLL, unload self automatically if the injector process aborted.

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
    .apply("input".into())
    .on_error(|pid, err| ())
    .call()
    .unwrap();

// Eject all manually or let drop handle it
injections.eject().on_error(|pid, err| ()).call();
```

See [`src/bin/inject-app-dll.rs`](https://github.com/Chaoses-Ib/ib-hook/blob/master/ib-hook/src/bin/inject-app-dll.rs)
and [`examples/app-dll.rs`](https://github.com/Chaoses-Ib/ib-hook/blob/master/ib-hook/examples/app-dll.rs)
for a complete example.
*/
pub mod app;
#[cfg(feature = "inject-dll-dll")]
pub mod dll;
