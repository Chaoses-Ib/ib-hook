use windows::{
    Win32::{
        Foundation::HMODULE,
        System::LibraryLoader::{
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            GetModuleHandleExW,
        },
    },
    core::PCWSTR,
};

pub mod shell;

/// Get the handle of the current executable or DLL.
///
/// Ref: https://github.com/compio-rs/winio/issues/35
pub fn get_current_module_handle() -> HMODULE {
    let mut module = HMODULE::default();
    _ = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(get_current_module_handle as *const _),
            &mut module,
        )
    };
    module
}
