/*!
Utility functions for the DLL part of DLL injection:
- Unload self:
  [`free_current_module_and_exit_thread()`]
- Auto self unload, i.e. wait for the injector process and unload self:
  - Blocking: [`wait_and_free_current_module()`]
  - Non-blocking: [`spawn_wait_and_free_current_module_once!()`]
- [`ThreadGuard`]:
  A guard that terminates the thread when drop.

See [`src/bin/inject-app-dll.rs`](https://github.com/Chaoses-Ib/IbDllHijackLib/blob/master/ib-hook/src/bin/inject-app-dll.rs)
and [`examples/app-dll.rs`](https://github.com/Chaoses-Ib/IbDllHijackLib/blob/master/ib-hook/examples/app-dll.rs)
for a complete example.

## Pitfalls
<div class="warning">

`APPLY(None)` (`teardown`) should clean up all the references to the DLL's code,
including hooks and threads. Otherwise, when the DLL is unloaded
the process will crash due to memory access violation.

One more pitfall is that Rust will not drop static variables for DLL,
so you should either not use any static variables (that hold resources),
or use [`macro@dtor`] to drop manually, for example:
```no_run
use std::cell::OnceCell;
use ib_hook::inject::dll::dll::{dtor, ThreadGuard};

static mut WAIT_AND_FREE: OnceCell<ThreadGuard> = OnceCell::new();

#[dtor]
fn free() {
    unsafe { &mut *&raw mut WAIT_AND_FREE }.take();
}
```
Or, if the leaked resources won't matter, just ignoring this is fine.
([`spawn_wait_and_free_current_module_once!()`] already handled its thread.)
</div>
*/
use std::{
    cell::OnceCell,
    os::windows::io::{AsRawHandle, OwnedHandle},
    thread,
};

use windows::Win32::{
    Foundation::HANDLE,
    System::{
        LibraryLoader::FreeLibraryAndExitThread,
        Threading::{
            INFINITE, OpenProcess, PROCESS_SYNCHRONIZE, TerminateThread, WaitForSingleObject,
        },
    },
};

use crate::process::{Pid, module::get_current_module_handle};

pub use dtor::dtor;

/**
Should clean up all the references to the DLL's code before,
including hooks and threads.
*/
pub fn free_current_module_and_exit_thread(code: u32) -> ! {
    unsafe { FreeLibraryAndExitThread(get_current_module_handle(), code) }
}

/**
Auto self unload, i.e. wait for the injector process and unload self.

`teardown` should clean up all the references to the DLL's code,
including hooks and threads.

## Returns
Actually `windows::core::Result<!>` but `!` is not stable yet.

https://github.com/rust-lang/rust/issues/35121
*/
pub fn wait_and_free_current_module(
    pid: Pid,
    teardown: impl FnOnce() -> u32,
) -> windows::core::Result<()> {
    let process = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, *pid) }?;
    unsafe { WaitForSingleObject(process, INFINITE) };
    free_current_module_and_exit_thread(teardown())
}

/**
A guard that terminates the thread when drop.

Unapply must clean up all threads.
[`TerminateThread()`] has some [footguns](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminatethread#remarks),
but better than crashing the entire process.

Should be used with [`macro@dtor`], see [Pitfalls](super::dll#pitfalls).
*/
pub struct ThreadGuard(OwnedHandle);

impl<T> From<thread::JoinHandle<T>> for ThreadGuard {
    fn from(thread: thread::JoinHandle<T>) -> Self {
        Self(thread.into())
    }
}

impl Drop for ThreadGuard {
    fn drop(&mut self) {
        _ = unsafe { TerminateThread(HANDLE(self.0.as_raw_handle()), 0) };
    }
}

#[doc(hidden)]
pub mod manual {
    use super::*;

    pub unsafe fn spawn_wait_and_free_current_module(
        pid: Pid,
        teardown: impl FnOnce() -> u32 + Send + 'static,
    ) -> ThreadGuard {
        thread::spawn(move || wait_and_free_current_module(pid, teardown)).into()
    }

    static mut WAIT_AND_FREE: OnceCell<ThreadGuard> = OnceCell::new();

    /**
    `teardown` should clean up all the references to the DLL's code,
    including hooks and threads.
    */
    pub unsafe fn spawn_wait_and_free_current_module_once(
        pid: Pid,
        teardown: impl FnOnce() -> u32 + Send + 'static,
    ) {
        unsafe { &*&raw const WAIT_AND_FREE }
            .get_or_init(move || unsafe { spawn_wait_and_free_current_module(pid, teardown) });
    }

    pub fn free() {
        unsafe { &mut *&raw mut WAIT_AND_FREE }.take();
    }
}

/**
Auto self unload, i.e. wait for the injector process and unload self.

- `pid`: [`Pid`]
- `teardown`: `impl FnOnce() -> u32 + Send + 'static`

  `teardown` should clean up all the references to the DLL's code,
  including hooks and threads.

Because using `#[dtor]` will cause some data and code always be compiled even not used,
this is implemented as a macro instead of a function.
*/
#[macro_export]
macro_rules! spawn_wait_and_free_current_module_once {
    ($pid:expr, $teardown:expr) => {
        #[::ib_hook::inject::dll::dll::dtor]
        fn free() {
            ::ib_hook::inject::dll::dll::manual::free();
        }

        unsafe {
            ::ib_hook::inject::dll::dll::manual::spawn_wait_and_free_current_module_once(
                $pid, $teardown,
            )
        }
    };
}
pub use spawn_wait_and_free_current_module_once;
