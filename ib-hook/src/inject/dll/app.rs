/*!
Inject DLL into target processes with an opinioned RPC schema.

## API
- [`DllApp`]: DLL RPC schema.
- [`DllInjection::inject()`]: Inject a DLL into a process and optionally appliy it (call `APPLY()`).
- [`DllInjection::eject()`]: Eject a DLL (automatically unapply first if applied before).
- [`DllInjection::leak()`]: Prevent automatic ejection on drop.
- [`DllInjection::drop()`]: Automatically unapply and eject if not already ejected (or [`DllInjection::leak`]ed).
- [`DllInjectionVec`]: Manages multiple injections with batch eject support.
  - [`DllInjectionVecWithInput`]: With owned `dll_path` and `input` for `apply()`.

## Example: Single process injection

```no_run
use ib_hook::inject::dll::app::{DllApp, DllInjection, OwnedProcess};

// Define your DLL app trait implementation
struct MyDll;
impl DllApp for MyDll {
    const APPLY: &str = "apply_hook";
    type Input = String;
    type Output = ();
}

// Inject into a single process
let process = OwnedProcess::find_first_by_name("Notepad.exe").unwrap();
let mut injection = DllInjection::<MyDll>::inject(process)
    .dll_path(std::path::Path::new("hook.dll"))
    .apply(&"input".into())
    .call()
    .unwrap();

// Eject manually or let drop handle it
injection.eject().unwrap();
```

## Example: Multiple processes by name

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

## Example: Custom process iterator

```no_run
use ib_hook::inject::dll::app::{DllApp, DllInjectionVec, OwnedProcess};

struct MyDll;
impl DllApp for MyDll {
    const APPLY: &str = "apply_hook";
    type Input = ();
    type Output = ();
}

let processes = vec![
    OwnedProcess::find_first_by_name("Notepad.exe").unwrap(),
    OwnedProcess::find_first_by_name("Time.exe").unwrap(),
];

let mut injections = DllInjectionVec::<MyDll>::new();
injections.inject(processes.into_iter())
    .dll_path(std::path::Path::new("hook.dll"))
    .on_error(|pid, err| ())
    .call()
    .unwrap();
```

## Disclaimer
This is currently implemented as a wrapper of [`dll_syringe`],
for object ownership (avoiding self-references), RAII (drop guard) and `Send`.

Ref: https://github.com/Chaoses-Ib/ib-shell/blob/7dc099ea07a9c0a0e2db6aea10a74b2b53c9373e/ib-shell-item/src/hook/inject.rs
*/
use std::{
    mem::transmute,
    path::{Path, PathBuf},
};

use bon::bon;
use derive_more::{Deref, DerefMut};
use dll_syringe::{
    Syringe,
    process::{BorrowedProcessModule, Process},
    rpc::RemotePayloadProcedure,
};
use thiserror::Error;

use crate::{log::*, process::Pid};

#[doc(hidden)]
pub use dll_syringe::payload_utils::__payload_procedure_helper;
pub use dll_syringe::process::OwnedProcess;

#[derive(Error, Debug)]
pub enum InjectError {
    #[error("dll not found: {0}")]
    DllNotFound(PathBuf),
    #[error("cannot find any {0} process")]
    ProcessNotFound(String),
    #[error("inject failed: {0}")]
    InjectFailed(#[from] dll_syringe::error::InjectError),
    #[error("get apply failed: {0}")]
    GetApplyFailed(#[from] dll_syringe::error::LoadProcedureError),
    #[error("apply not found")]
    ApplyNotFound,
    #[error("apply: {0}")]
    ApplyFailed(#[from] dll_syringe::rpc::PayloadRpcError),
    #[error("eject failed: {0}")]
    EjectFailed(#[from] dll_syringe::error::EjectError),
}

/// DLL RPC schema.
///
/// Call `APPLY(Some(Input))` on [`inject`](DllInjection::inject),
/// and `APPLY(None)` on [`eject`](DllInjection::eject)
/// (and on drop if not [`leak`](DllInjection::leak)ed).
/**
<div class="warning">

`APPLY(None)` should clean up all the references to the DLL's code,
including hooks and threads. Otherwise, when the DLL is unloaded
the process will crash due to memory access violation.
</div>
*/
pub trait DllApp {
    /// The name of the exported function for RPC.
    const APPLY: &str;

    type Input: serde::Serialize + 'static;
    type Output: serde::de::DeserializeOwned + 'static;
}

/**
Usage:
```ignore
ib_hook::inject::dll::app::export_apply!(apply_hook, "apply_hook");
```

## Implementation
For example:
```ignore
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct ArgAndResultBufInfo {
    pub data: u64,
    pub len: u64,
    pub is_error: bool,
}

const _: () = {
    #[unsafe(export_name = "ib_shell_apply")]
    pub unsafe extern "system" fn _ib_hook_inject_dll_app_apply(
        __args_and_params: *mut ::core::ffi::c_void,
    ) {
        let buf_info_ptr = __args_and_params;
        let buf_info_ptr = buf_info_ptr.cast::<ArgAndResultBufInfo>();
        let buf_info = unsafe { &mut *buf_info_ptr };
        // buf_info.len = buf_info.len - 1;
        let buf =
            unsafe { slice::from_raw_parts_mut(buf_info.data as *mut u8, buf_info.len as usize) };
        // dbg!(&buf_info);

        let config = bincode::config::standard();

        // eprintln!("decode_from_slice {:02X?}", buf);
        let args: Result<((Option<Input>,), usize), bincode::error::DecodeError> =
            bincode::serde::decode_from_slice::<(Option<Input>,), _>(buf, config);
        // eprintln!("{:X}\n{:02X?}\nargs: {:?}", buf_info.data, buf, &args);

        ib_hook::inject::dll::app::__payload_procedure_helper(__args_and_params, |__args| {
            let (input,) = __args;

            // panic!(
            //     "{:X}\n{:02X?}\n{:?}\n{:?}",
            //     buf_info.data, buf, &input, &args
            // );
            apply_hook(input)
        });
    }
};
```

TODO: https://github.com/rust-lang/rust/issues/52393
TODO: https://github.com/rust-lang/rust/issues/143547 or proc macro
*/
#[macro_export]
macro_rules! export_apply {
    ($apply:ident, $export_name:literal) => {
        const _: () = {
            #[unsafe(export_name = $export_name)]
            pub unsafe extern "system" fn _ib_hook_inject_dll_app_apply(
                __args_and_params: *mut ::core::ffi::c_void,
            ) {
                $crate::inject::dll::app::__payload_procedure_helper(__args_and_params, |__args| {
                    let (input,) = __args;
                    $apply(input)
                });
            }
        };
    };
}
pub use export_apply;

/// Represents an injected DLL with its syringe, payload, and remote apply function.
pub struct DllInjection<D: DllApp> {
    syringe: Syringe,
    /// The injected DLL module (borrowed from the syringe).
    payload: BorrowedProcessModule<'static>,
    /// Remote procedure to call apply on the injected DLL.
    remote_apply: RemotePayloadProcedure<fn(Option<&'static D::Input>) -> D::Output>,
    /// PID of the target process.
    pid: Pid,
    /// Whether APPLY was successfully called.
    applied: bool,
    /// Whether the injection has been ejected (prevents cleanup on drop).
    ejected: bool,
}

/**
[`Syringe`] contains [`RemoteBoxAllocator`] which is [`Rc`] inner and thus `!Send`.
But [`Syringe`] itself is `!Clone`, it's actually `Send`.
*/
unsafe impl<D: DllApp> Send for DllInjection<D> {}

/*
Only [`apply()`] works with `&self`.
Unfortunately, it uses a `!Sync` allocator.
*/
// unsafe impl<D: DllApp> Sync for DllInjection<D> {}

#[bon]
impl<D: DllApp> DllInjection<D> {
    /// Inject the DLL into the given process and optionally appliy it (call `APPLY()`).
    #[builder]
    pub fn inject(
        #[builder(start_fn)] process: OwnedProcess,
        dll_path: &Path,
        apply: Option<&D::Input>,
    ) -> Result<Self, InjectError> {
        let pid = Pid(process.pid().unwrap().get());
        let syringe = Syringe::for_process(process);

        info!(%pid, ?dll_path, "Injecting");
        let payload = syringe.find_or_inject(dll_path)?;
        let eject = || {
            if let Err(e) = syringe.eject(payload) {
                warn!(?e, "eject");
            }
        };

        // Eject if ApplyNotFound
        let remote_apply = unsafe { syringe.get_payload_procedure(payload, D::APPLY) }
            .map_err(InjectError::from)
            .inspect_err(|_| eject())?
            .ok_or(InjectError::ApplyNotFound)
            .inspect_err(|_| eject())?;

        // Transmute payload to 'static since syringe (owner of process) is returned
        let payload = unsafe { transmute(payload) };

        let mut injection = Self {
            payload,
            syringe,
            remote_apply,
            pid,
            applied: false,
            ejected: false,
        };

        if let Some(input) = apply {
            // Drop & eject on error
            injection.apply(input)?;
            injection.applied = true;
        }

        info!(%pid, "Successfully injected");

        Ok(injection)
    }

    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// Call [`DllApp::APPLY`] with the given input.
    pub fn maybe_apply(
        &self,
        input: Option<&D::Input>,
    ) -> Result<D::Output, dll_syringe::rpc::PayloadRpcError> {
        if let Some(input) = input {
            self.apply(input)
        } else {
            self.unapply()
        }
    }

    /// Call [`DllApp::APPLY`] with the given input.
    /**
    ## Implementation
    ```ignore
    let args = (input,);
    let config = bincode::config::standard();

    let mut size_writer = bincode::enc::write::SizeWriter::default();
    bincode::serde::encode_into_writer(&args, &mut size_writer, config)?;
    let arg_bytes = size_writer.bytes_written;
    let mut local_arg_buf = Vec::with_capacity(arg_bytes);
    bincode::serde::encode_into_std_write(&args, &mut local_arg_buf, config)?;
    unsafe { local_arg_buf.set_len(arg_bytes) };
    // println!("{:02X?}", local_arg_buf);

    /*
    let buf = &local_arg_buf[..arg_bytes].to_owned();
    println!("decode_from_slice {:02X?}", buf);
    let args: Result<((Option<Input>,), usize), bincode::error::DecodeError> =
        bincode::serde::decode_from_slice::<(Option<Input>,), _>(buf, config);
    println!("{:?}", args);
    */
    ...
    ```
    */
    pub fn apply(&self, input: &D::Input) -> Result<D::Output, dll_syringe::rpc::PayloadRpcError> {
        // call() doen't really need 'static.
        // &Option<D::Input> can be used instead and was used before,
        // but it made life pathetic.
        let input: &'static D::Input = unsafe { transmute(input) };
        self.remote_apply.call(&Some(input))
    }

    pub fn unapply(&self) -> Result<D::Output, dll_syringe::rpc::PayloadRpcError> {
        self.remote_apply.call(&None)
    }

    /// Eject the DLL from the target process.
    ///
    /// This will first unapply if it was applied before.
    pub fn eject(&mut self) -> Result<(), InjectError> {
        if self.ejected {
            return Ok(());
        }
        if self.applied {
            self.unapply()?;
        }
        // Panics if the given module was not loaded in the target process.
        self.syringe.eject(self.payload)?;
        self.ejected = true;
        Ok(())
    }

    /// Leak the injection, preventing both manual and automatic `eject()`.
    pub fn leak(&mut self) {
        self.ejected = true;
    }
}

impl<D: DllApp> Drop for DllInjection<D> {
    fn drop(&mut self) {
        if let Err(e) = self.eject() {
            error!(pid = self.pid.0, ?e, "Failed to eject on drop");
        }
    }
}

/// A collection of injected processes that can be ejected together.
pub struct DllInjectionVec<D: DllApp> {
    injections: Vec<DllInjection<D>>,
    ejected: bool,
}

/// #[derive(Debug)] will require D: Default
impl<D: DllApp> Default for DllInjectionVec<D> {
    fn default() -> Self {
        Self {
            injections: Default::default(),
            ejected: Default::default(),
        }
    }
}

#[bon]
impl<D: DllApp> DllInjectionVec<D> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn injections(&self) -> &[DllInjection<D>] {
        &self.injections
    }

    pub fn injections_mut(&mut self) -> &mut [DllInjection<D>] {
        &mut self.injections
    }

    /// Inject the DLL into the given processes.
    ///
    /// Before [`DllInjectionVec::eject()`], the DLL file will be locked and can't be deleted.
    ///
    /// # Returns
    /// - `Ok(DllInjectionVec)`: Successfully injected processes
    /// - `Err(InjectError)`: Error during injection
    #[builder]
    pub fn inject(
        &mut self,
        /// Processes to inject into.
        #[builder(start_fn)]
        processes: impl Iterator<Item = OwnedProcess>,
        /// Path to the DLL
        dll_path: &Path,
        /// Optionally apply with the given input after injection.
        apply: Option<&D::Input>,
        /// Optional callback for errors during injection (called in the middle of the loop).
        ///
        /// Errors will always be logged.
        mut on_error: Option<impl FnMut(Pid, InjectError)>,
    ) -> Result<&mut Self, InjectError> {
        if !dll_path.exists() {
            return Err(InjectError::DllNotFound(dll_path.to_path_buf()));
        }

        // Store injected processes for later eject
        for target_process in processes {
            let pid = Pid(target_process.pid().unwrap().get());
            match DllInjection::inject(target_process)
                .dll_path(&dll_path)
                .maybe_apply(apply)
                .call()
            {
                Ok(injection) => {
                    self.injections.push(injection);
                }
                Err(e) => {
                    error!(%pid, ?e, "Failed to inject");
                    if let Some(cb) = on_error.as_mut() {
                        cb(pid, e);
                    }
                }
            }
        }

        Ok(self)
    }

    /// Inject the DLL into all processes with the given name.
    ///
    /// Before [`DllInjectionVec::eject()`], the DLL file will be locked and can't be deleted.
    ///
    /// # Returns
    /// - `Ok(DllInjectionVec)`: Successfully injected processes
    /// - `Err(InjectError)`: Error during injection
    #[builder]
    pub fn inject_with_process_name(
        &mut self,
        /// Name of the process to inject into.
        #[builder(start_fn)]
        process_name: &str,
        /// Path to the DLL
        dll_path: &Path,
        /// Optionally apply with the given input after injection.
        apply: Option<&D::Input>,
        /// Optional callback for errors during injection (called in the middle of the loop).
        ///
        /// Errors will always be logged.
        on_error: Option<impl FnMut(Pid, InjectError)>,
    ) -> Result<&mut Self, InjectError> {
        // Find all processes with the given name
        let processes = OwnedProcess::find_all_by_name(process_name);
        if processes.is_empty() {
            return Err(InjectError::ProcessNotFound(process_name.to_string()));
        }
        info!("Found {} {} processes", processes.len(), process_name);

        self.inject(processes.into_iter())
            .dll_path(dll_path)
            .maybe_apply(apply)
            .maybe_on_error(on_error)
            .call()
    }

    /// Call [`apply`](DllInjection::apply) on all injections.
    ///
    /// Errors are reported via the `on_error` callback.
    #[builder]
    pub fn apply(
        &self,
        #[builder(start_fn)] input: &D::Input,
        mut on_error: Option<impl FnMut(Pid, &dll_syringe::rpc::PayloadRpcError)>,
    ) {
        for injection in &self.injections {
            if let Err(e) = injection.apply(input) {
                if let Some(on_error) = on_error.as_mut() {
                    on_error(injection.pid(), &e);
                }
            }
        }
    }

    /// Call [`unapply`](DllInjection::unapply) on all injections.
    ///
    /// Errors are reported via the `on_error` callback.
    #[builder]
    pub fn unapply(
        &self,
        mut on_error: Option<impl FnMut(Pid, &dll_syringe::rpc::PayloadRpcError)>,
    ) {
        for injection in &self.injections {
            if let Err(e) = injection.unapply() {
                if let Some(on_error) = on_error.as_mut() {
                    on_error(injection.pid(), &e);
                }
            }
        }
    }

    /// Eject all DLL injections.
    #[builder]
    pub fn eject(
        &mut self,
        /// Optional callback for errors during ejection (called in the middle of the loop).
        ///
        /// Errors will always be logged.
        mut on_error: Option<impl FnMut(Pid, InjectError)>,
    ) {
        for mut injection in self.injections.drain(..) {
            let pid = injection.pid;
            if let Err(e) = injection.eject() {
                warn!(%pid, ?e, "Failed to eject");
                if let Some(cb) = on_error.as_mut() {
                    cb(pid, e);
                }
            }
        }

        info!("Successfully ejected");
    }

    /// Leak existing and new injections, preventing automatic cleanup on drop.
    /// (Unlike [`DllInjection::leak`], this doesn't prevent manual `eject()`)
    pub fn leak(&mut self) {
        self.ejected = true;
    }
}

impl<D: DllApp> Drop for DllInjectionVec<D> {
    fn drop(&mut self) {
        if self.ejected {
            for injection in &mut self.injections {
                injection.leak();
            }
            return;
        }
        self.eject().on_error(|_, _| ()).call();
    }
}

/// A collection of injected processes that can be ejected together, with a shared input for apply.
///
/// Unlike [`DllInjectionVec`], this stores the input and applies it automatically during inject.
#[derive(Deref, DerefMut)]
pub struct DllInjectionVecWithInput<D: DllApp> {
    dll_path: PathBuf,
    input: Option<D::Input>,
    #[deref]
    #[deref_mut]
    inner: DllInjectionVec<D>,
}

impl<D: DllApp> DllInjectionVecWithInput<D> {
    /// Creates a new `DllInjectionVecWithInput` with the given DLL path.
    ///
    /// The DLL path is checked to ensure it exists.
    pub fn new(dll_path: PathBuf) -> Result<Self, InjectError> {
        Self::with_input(dll_path, None)
    }

    pub fn with_input(dll_path: PathBuf, input: Option<D::Input>) -> Result<Self, InjectError> {
        if !dll_path.exists() {
            return Err(InjectError::DllNotFound(dll_path));
        }
        Ok(Self {
            dll_path,
            input,
            inner: Default::default(),
        })
    }

    pub fn dll_path(&self) -> &PathBuf {
        &self.dll_path
    }

    pub fn input(&self) -> Option<&D::Input> {
        self.input.as_ref()
    }
}

#[bon]
impl<D: DllApp> DllInjectionVecWithInput<D> {
    /// Inject the DLL into the given processes with the stored input and dll_path.
    ///
    /// Before [`DllInjectionVecWithInput::eject()`], the DLL file will be locked and can't be deleted.
    ///
    /// # Returns
    /// - `Ok(DllInjectionVecWithInput)`: Successfully injected processes
    /// - `Err(InjectError)`: Error during injection
    #[builder]
    pub fn inject(
        &mut self,
        /// Processes to inject into.
        #[builder(start_fn)]
        processes: impl Iterator<Item = OwnedProcess>,
        /// Optional callback for errors during injection (called in the middle of the loop).
        ///
        /// Errors will always be logged.
        on_error: Option<impl FnMut(Pid, InjectError)>,
    ) -> Result<&mut Self, InjectError> {
        self.inner
            .inject(processes)
            .dll_path(&self.dll_path)
            .maybe_apply(self.input.as_ref())
            .maybe_on_error(on_error)
            .call()?;
        Ok(self)
    }

    /// Inject the DLL into all processes with the given name.
    ///
    /// Before [`DllInjectionVecWithInput::eject()`], the DLL file will be locked and can't be deleted.
    ///
    /// # Returns
    /// - `Ok(DllInjectionVecWithInput)`: Successfully injected processes
    /// - `Err(InjectError)`: Error during injection
    #[builder]
    pub fn inject_with_process_name(
        &mut self,
        /// Name of the process to inject into.
        #[builder(start_fn)]
        process_name: &str,
        /// Optional callback for errors during injection (called in the middle of the loop).
        ///
        /// Errors will always be logged.
        on_error: Option<impl FnMut(Pid, InjectError)>,
    ) -> Result<&mut Self, InjectError> {
        self.inner
            .inject_with_process_name(process_name)
            .dll_path(&self.dll_path)
            .maybe_apply(self.input.as_ref())
            .maybe_on_error(on_error)
            .call()?;
        Ok(self)
    }

    /// Call [`apply`](DllInjection::apply) on all injections with the stored input.
    ///
    /// Updates the stored input to the new value.
    #[builder]
    pub fn apply(
        &mut self,
        #[builder(start_fn)] input: D::Input,
        on_error: Option<impl FnMut(Pid, &dll_syringe::rpc::PayloadRpcError)>,
    ) {
        let input = self.input.insert(input);
        self.inner.apply(input).maybe_on_error(on_error).call();
    }

    /// Call [`unapply`](DllInjection::unapply) on all injections.
    ///
    /// Updates the stored input to `None`.
    #[builder]
    pub fn unapply(
        &mut self,
        on_error: Option<impl FnMut(Pid, &dll_syringe::rpc::PayloadRpcError)>,
    ) {
        self.input = None;
        self.inner.unapply().maybe_on_error(on_error).call()
    }
}
