/*!
Setup:
```sh
cargo add ib-hook --features inject-dll-dll
cargo add serde windows
```
And set `crate-type = ["cdylib"]`.

To run this example:
```sh
cargo build -p ib-hook --example app-dll
cargo run -p ib-hook --bin inject-app-dll --features inject-dll
```
*/
use std::ffi::CString;

use ib_hook::{inject::dll::app::DllApp, process::Pid};
use windows::{Win32::UI::WindowsAndMessaging::MessageBoxA, core::PCSTR};

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

#[ib_hook::inject::dll::app::payload_procedure]
fn apply_hook(input: Option<<MyDll as DllApp>::Input>) -> <MyDll as DllApp>::Output {
    if let Some(input) = input {
        ib_hook::inject::dll::dll::spawn_wait_and_free_current_module_once(input.injector, || {
            unapply();
            0
        });

        let s = CString::new(input.s).unwrap();
        unsafe {
            MessageBoxA(
                None,
                PCSTR(s.as_ptr().cast()),
                PCSTR::null(),
                Default::default(),
            )
        };
    } else {
        unapply();
    }
}

fn unapply() {
    let s = CString::new("unapply").unwrap();
    unsafe {
        MessageBoxA(
            None,
            PCSTR(s.as_ptr().cast()),
            PCSTR::null(),
            Default::default(),
        )
    };
}
