/*!
Inline hooking.

- Supported CPU architectures: x86, x64, ARM64.
- Support system ABI (`system`, `stdcall`/`win64`) only.
- `no_std` and depend on `Ntdll.dll` only (if `tracing` is not enabled).
- RAII (drop guard) design.

  To leak the hook, wrap [`InlineHook`] as [`std::mem::ManuallyDrop<InlineHook>`]
  (or call [`std::mem::forget()`]).
- Thread unsafe at the moment.

  If you may enable/disable hooks from multiple threads at the same time,
  use a [`std::sync::Mutex`] lock.
- To init a (`mut`) `static`, [`InlineHook::new_disabled()`] can be used.

## Examples
```
// cargo add ib-hook --features inline
use ib_hook::inline::InlineHook;

extern "system" fn original(x: u32) -> u32 { x + 1 }

// Hook the function with a detour
extern "system" fn hooked(x: u32) -> u32 { x + 0o721 }
let mut hook = InlineHook::<extern "system" fn(u32) -> u32>::new(original, hooked).unwrap();
assert!(hook.is_enabled());

// Now calls to original are redirected to hooked
assert_eq!(original(0x100), 721); // redirected to hooked: 0x100 + 0o721 = 721

// Access original via trampoline
assert_eq!(hook.trampoline()(0x100), 0x101); // 0x100 + 1

// Disable the hook manually (or automatically on drop)
hook.disable().unwrap();
assert!(!hook.is_enabled());
assert_eq!(original(0x100), 0x101); // back to original
```

## Disclaimer
This is currently implemented as a wrapper of
[KNSoft.SlimDetours](https://github.com/KNSoft/KNSoft.SlimDetours),
for type safety and RAII (drop guard).

Ref: https://github.com/Chaoses-Ib/ib-shell/pull/1
*/
use core::{ffi::c_void, fmt::Debug, mem::transmute_copy};

use slim_detours_sys::SlimDetoursInlineHook;
use windows::core::HRESULT;

use crate::{FnPtr, log::*};

/// Type-safe and RAII (drop guard) wrapper of an inline hook.
///
/// Manages the lifetime of a detour hook, providing easy enable/disable
/// and cleanup through RAII principles.
///
/// See [`inline`](super::inline) module for details.
///
/// ## Type Parameters
/// - `F`: The function type being hooked.
#[derive(Debug)]
pub struct InlineHook<F: FnPtr> {
    /// Sometimes statically known.
    target: F,
    /// The trampoline function (original, before hooking).
    /// If `target == trampoline`, the hook is not enabled.
    trampoline: F,
    /// Hooked function pointer
    ///
    /// Detour is usually statically known, but we still need to keep it for RAII.
    detour: F,
}

impl<F: FnPtr> InlineHook<F> {
    /// Creates a new `InlineHookGuard` and immediately applies the hook.
    ///
    /// ## Arguments
    /// - `enable`: Whether to enable the hook immediately (true = enable, false = disable)
    /// - `target`: Pointer to the target function to hook
    /// - `detour`: Pointer to the detour/hooked function
    ///
    /// ## Returns
    /// - `Ok(InlineHookGuard)` if hook creation succeeds
    /// - `HRESULT` error if hook creation fails
    pub fn with_enabled(target: F, detour: F, enable: bool) -> Result<Self, HRESULT> {
        let target_ptr: *mut c_void = unsafe { transmute_copy(&target) };
        let detour_ptr: *mut c_void = unsafe { transmute_copy(&detour) };

        let mut trampoline_ptr: *mut c_void = target_ptr;
        let res = unsafe { SlimDetoursInlineHook(enable as _, &mut trampoline_ptr, detour_ptr) };
        let hr = HRESULT(res);

        if hr.is_ok() {
            let trampoline: F = unsafe { transmute_copy(&trampoline_ptr) };
            let guard = Self {
                target,
                trampoline,
                detour,
            };
            debug!(?target, ?detour, ?trampoline, ?enable, "InlineHook");
            Ok(guard)
        } else {
            Err(hr)
        }
    }

    /// Creates a new `InlineHookGuard` without immediately enabling it.
    ///
    /// ## Arguments
    /// - `target`: Pointer to the target function to hook
    /// - `detour`: Pointer to the detour/hooked function
    ///
    /// ## Returns
    /// `InlineHookGuard` with the hook not yet applied.
    /// Call `enable()` to apply it.
    pub const fn new_disabled(target: F, detour: F) -> Self {
        Self {
            target,
            trampoline: target,
            detour,
        }
    }

    /// Creates a new `InlineHookGuard` with the hook enabled.
    ///
    /// ## Arguments
    /// - `target`: Pointer to the target function to hook
    /// - `detour`: Pointer to the detour/hooked function
    ///
    /// ## Returns
    /// - `Ok(InlineHookGuard)` with the hook created and enabled
    /// - `HRESULT` error if hook creation fails
    pub fn new(target: F, detour: F) -> Result<Self, HRESULT> {
        Self::with_enabled(target, detour, true)
    }

    /// Enables or disables the hook.
    ///
    /// ## Arguments
    /// - `enable`: `true` to enable, `false` to disable
    ///
    /// ## Returns
    /// - `HRESULT` success or error code
    pub fn set_enabled(&mut self, enable: bool) -> HRESULT {
        let detour_ptr: *mut c_void = unsafe { transmute_copy(&self.detour) };
        let mut trampoline_ptr: *mut c_void = unsafe { transmute_copy(&self.trampoline) };

        let res = unsafe { SlimDetoursInlineHook(enable as _, &mut trampoline_ptr, detour_ptr) };
        let hr = HRESULT(res);

        if hr.is_ok() {
            self.trampoline = unsafe { transmute_copy(&trampoline_ptr) };
        }
        hr
    }

    /// Enables the hook.
    ///
    /// ## Returns
    /// - `Ok(())` if the hook is enabled successfully (or already enabled)
    /// - `HRESULT` error if enabling fails
    pub fn enable(&mut self) -> HRESULT {
        // SlimDetoursInlineHook() will report 0xD0190001 for already enabled hook
        if self.is_enabled() {
            return HRESULT(0);
        }
        self.set_enabled(true)
    }

    /// Disables the hook.
    ///
    /// ## Returns
    /// - `Ok(())` if the hook is disabled successfully (or not enabled)
    /// - `HRESULT` error if disabling fails
    pub fn disable(&mut self) -> HRESULT {
        // SlimDetoursInlineHook() will report 0xD0000173 for not enabled hook
        if !self.is_enabled() {
            return HRESULT(0);
        }
        self.set_enabled(false)
    }

    /// Toggles the hook state (enabled -> disabled, disabled -> enabled).
    ///
    /// ## Returns
    /// - `Ok(())` if toggle succeeds
    /// - `HRESULT` error if toggle fails
    pub fn toggle(&mut self) -> HRESULT {
        if self.is_enabled() {
            self.disable()
        } else {
            self.enable()
        }
    }

    /// Returns `true` if the hook is currently enabled.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.target != self.trampoline
    }

    /// Returns the target function being hooked.
    #[inline]
    pub const fn target(&self) -> F {
        self.target
    }

    /// Returns `true` if `other` is the same target function as this hook.
    ///
    /// This is mainly for avoiding the warning if not using [`std::ptr::fn_addr_eq()`].
    #[inline]
    pub fn is_target(&self, other: F) -> bool {
        self.target == other
    }

    /// Returns the detour function that will be called when the hook is active.
    #[inline]
    pub const fn detour(&self) -> F {
        self.detour
    }

    /// Returns the trampoline function holding the original target implementation.
    ///
    /// When the hook is enabled, calling `target()` redirects to `detour()`,
    /// while `trampoline()` provides access to the original target functionality.
    #[inline]
    pub const fn trampoline(&self) -> F {
        self.trampoline
    }
}

impl<F: FnPtr> Drop for InlineHook<F> {
    fn drop(&mut self) {
        let hr = self.disable();
        if !hr.is_ok() {
            debug!(?hr, "Failed to disable hook on drop");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    // Static mutex to prevent race conditions in slim_detours_sys tests
    // slim_detours_sys is not thread-safe for concurrent hook operations
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    /// Mock target function - represents the function being hooked
    #[inline(never)]
    extern "system" fn inc_target(x: u32) -> u32 {
        x + 1
    }

    /// Mock detour function - represents the hook handler
    #[inline(never)]
    extern "system" fn dec_detour(x: u32) -> u32 {
        x - 1
    }

    #[test]
    fn assert_send_sync() {
        // Compile-time check that InlineHook is Send + Sync
        fn assert_send<F: FnPtr>(_: &InlineHook<F>) {}
        fn assert_sync<F: FnPtr>(_: &InlineHook<F>) {}

        type MyFn = extern "system" fn(u32) -> u32;
        extern "system" fn dummy(_x: u32) -> u32 {
            0
        }
        let hook = InlineHook::<MyFn>::new_disabled(dummy, dummy);

        assert_send(&hook);
        assert_sync(&hook);

        {
            type MyFn = unsafe extern "system" fn(*mut c_void) -> u32;
            unsafe extern "system" fn dummy(_x: *mut c_void) -> u32 {
                0
            }
            let hook = InlineHook::<MyFn>::new_disabled(dummy, dummy);
            assert_send(&hook);
            assert_sync(&hook);
        }
    }

    #[test]
    fn is_target() {
        let _guard = TEST_MUTEX.lock().unwrap();
        type MyFn = extern "system" fn(u32) -> u32;
        let target: MyFn = inc_target;
        let detour: MyFn = dec_detour;

        let hook = InlineHook::<MyFn>::new_disabled(target, detour);

        assert!(hook.is_target(target));
        assert!(!hook.is_target(detour));
    }

    #[test]
    fn inline_hook_creation() {
        let _guard = TEST_MUTEX.lock().unwrap();
        type FnType = extern "system" fn(u32) -> u32;
        let target = inc_target;
        let detour = dec_detour;

        // Verify functions work before hooking
        assert_eq!(target(5), 6); // 5 + 1
        assert_eq!(detour(5), 4); // 5 - 1

        let hook = InlineHook::<FnType>::new(target, detour).unwrap();
        assert!(hook.is_enabled());
        assert_eq!(hook.target() as *const c_void, target as *const c_void);
        assert_eq!(hook.detour() as *const c_void, detour as *const c_void);

        assert_eq!(hook.target()(5), 4); // 5 - 1 (redirected to detour)
        assert_eq!(inc_target(5), 4); // 5 - 1 (redirected to detour)
        assert_eq!(hook.trampoline()(5), 6); // 5 + 1 (original behavior via trampoline)
        assert_eq!(hook.detour()(5), 4); // 5 - 1
        assert_eq!(dec_detour(5), 4); // 5 - 1 (redirected to detour)
    }

    #[test]
    fn inline_hook_disabled_by_default() {
        let _guard = TEST_MUTEX.lock().unwrap();
        type FnType = extern "system" fn(u32) -> u32;
        let target = inc_target;
        let detour = dec_detour;

        let hook = InlineHook::<FnType>::new_disabled(target, detour);
        assert!(!hook.is_enabled());
        assert_eq!(hook.target() as *const c_void, target as *const c_void);
        assert_eq!(hook.detour() as *const c_void, detour as *const c_void);

        // Without hooking, target function works directly
        assert_eq!(target(10), 11); // 10 + 1
    }

    #[test]
    fn trampoline_is_true_original() {
        let _guard = TEST_MUTEX.lock().unwrap();
        type FnType = extern "system" fn(u32) -> u32;
        let target = inc_target;
        let detour = dec_detour;

        let hook = InlineHook::<FnType>::new(target, detour).unwrap();

        // trampoline holds the true original functionality after hooking
        // Calling through trampoline executes original target behavior
        assert_eq!(hook.trampoline()(5), 6); // 5 + 1 (mock_target's original behavior)
    }

    #[test]
    fn enable_disable() {
        let _guard = TEST_MUTEX.lock().unwrap();
        type FnType = extern "system" fn(u32) -> u32;
        let target = inc_target;
        let detour = dec_detour;

        let mut hook = InlineHook::<FnType>::new(target, detour).unwrap();
        assert!(hook.is_enabled());

        hook.disable().unwrap();
        assert!(!hook.is_enabled());

        hook.enable().unwrap();
        assert!(hook.is_enabled());
    }

    #[test]
    fn toggle() {
        let _guard = TEST_MUTEX.lock().unwrap();
        type FnType = extern "system" fn(u32) -> u32;
        let target = inc_target;
        let detour = dec_detour;

        let mut hook = InlineHook::<FnType>::new(target, detour).unwrap();
        assert!(hook.is_enabled());

        hook.toggle().unwrap();
        assert!(!hook.is_enabled());

        hook.toggle().unwrap();
        assert!(hook.is_enabled());
    }

    #[test]
    fn typed_function_pointers() {
        let _guard = TEST_MUTEX.lock().unwrap();
        type FnType = extern "system" fn(u32) -> u32;
        let target = inc_target;
        let detour = dec_detour;

        let hook = InlineHook::<FnType>::new(target, detour).unwrap();

        // Verify typed methods return callable function pointers
        assert_eq!(hook.target() as *const c_void, target as *const c_void);
        assert_eq!(hook.detour() as *const c_void, detour as *const c_void);
    }

    #[test]
    fn doc() {
        let _guard = TEST_MUTEX.lock().unwrap();
        // Hook a function with a detour
        extern "system" fn original(x: u32) -> u32 {
            x + 1
        }

        extern "system" fn hooked(x: u32) -> u32 {
            x + 0o721
        }
        let mut hook = InlineHook::<extern "system" fn(u32) -> u32>::new(original, hooked).unwrap();
        assert!(hook.is_enabled());

        // Now calls to original are redirected to hooked
        assert_eq!(original(0x100), 721); // redirected to hooked: 0x100 + 0o721 = 721

        // Access original via trampoline
        assert_eq!(hook.trampoline()(0x100), 0x101); // 0x100 + 1

        // Disable the hook manually (or automatically on drop)
        hook.disable().unwrap();
        assert!(!hook.is_enabled());
        assert_eq!(original(0x100), 0x101); // back to original
    }
}
