/*!
Process module (EXE/DLL) utilities.
*/
use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};

use derive_more::{Deref, From};
use windows::{
    Win32::{
        Foundation::{HMODULE, MAX_PATH},
        System::LibraryLoader::{
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            GetModuleFileNameW, GetModuleHandleExW,
        },
    },
    core::PCWSTR,
};

/// A process module (EXE/DLL).
#[derive(Clone, Copy, From, Deref, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Module(pub HMODULE);

impl Module {
    /// Get the handle of the current executable or DLL.
    ///
    /// Ref: https://github.com/compio-rs/winio/issues/35
    pub fn current() -> Self {
        let mut module = Module::default();
        _ = unsafe {
            GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                    | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                PCWSTR(Module::current as *const _),
                &mut module.0,
            )
        };
        module
    }

    /// Get the file path of a module (EXE/DLL).
    ///
    /// [GetModuleFileNameW function (libloaderapi.h)](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulefilenamew)
    pub fn get_path(self) -> PathBuf {
        let hmodule = Some(self.0);

        let mut buf_stack = [0; MAX_PATH as usize];
        let mut buf = buf_stack.as_mut_slice();
        let result = unsafe { GetModuleFileNameW(hmodule, buf) };

        let mut buf_heap;
        let len = if result == 0 {
            // Error occurred
            0
        } else if result == buf.len() as u32 {
            // Buffer was too small (truncated), try with a larger buffer
            // Extended path length
            let mut size = 512;
            loop {
                buf_heap = vec![0; size];
                buf = buf_heap.as_mut_slice();
                let result = unsafe { GetModuleFileNameW(hmodule, buf) };
                if result == 0 {
                    break 0;
                }
                if result != size as u32 {
                    // Success - result is the actual length
                    break result as usize;
                }
                // Still truncated, try larger buffer
                size *= 2;
            }
        } else {
            // Success - result is the actual length
            result as usize
        };

        let path_str = OsString::from_wide(&buf[..len]);
        PathBuf::from(path_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_path() {
        let module = Module::current();
        let path = module.get_path();
        println!("Current module path: {:?}", path);
        assert!(path.exists(), "Module path should exist: {:?}", path);
    }
}
