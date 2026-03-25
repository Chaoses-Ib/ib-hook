/*!
Setup:
```sh
cargo add ib-hook --features inject-dll
cargo add serde --features derive
```

To run this example:
```sh
cargo build -p ib-hook --example app-dll
cargo run -p ib-hook --bin inject-app-dll --features inject-dll
```
*/
use std::path::PathBuf;

use ib_hook::{
    inject::dll::app::{DllApp, DllInjectionVec},
    process::Pid,
};

#[derive(serde::Serialize, serde::Deserialize)]
struct Input {
    injector: Pid,
    s: String,
}

/// Should be shared across the DLL and injector.
struct MyDll;
impl DllApp for MyDll {
    const APPLY: &str = "apply_hook";
    type Input = Input;
    type Output = ();
}

fn main() {
    // cargo build -p ib-hook --example app-dll
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target")
        .join("debug")
        .join("examples")
        .join("app_dll.dll");

    // Inject into all processes named Notepad.exe
    let mut injections = DllInjectionVec::<MyDll>::new();
    injections
        .inject_with_process_name("Notepad.exe")
        .dll_path(&dll_path)
        .apply(&Input {
            injector: Pid::current(),
            s: "Hello, World!".into(),
        })
        .on_error(|pid, err| eprintln!("{pid:?}: {err:?}"))
        .call()
        .unwrap();

    // Choose leak or eject
    let leak = false;
    if leak {
        injections.leak();
    } else {
        // Eject all manually or let drop handle it
        injections
            .eject()
            .on_error(|pid, err| eprintln!("{pid:?}: {err:?}"))
            .call();
    }
}
