use core::mem::{self, transmute, transmute_copy};
use std::collections::{
    HashMap,
    hash_map::{Values, ValuesMut},
};

use bon::bon;
use windows::core::HRESULT;

use crate::{FnPtr, inline::InlineHook};

/**
A type-erased map of inline hooks indexed by target function pointer.

A collection of inline hooks that can be enabled/disabled together.

If you just want to store hooks of the same function type, just use
[`HashMap<F, InlineHook<F>>`] instead of [`InlineHookMap`].

## Examples
```no_run
// cargo add ib-hook --features inline
use ib_hook::inline::{InlineHook, InlineHookMap};

type MyFn = extern "system" fn(u32) -> u32;

extern "system" fn original1(x: u32) -> u32 { x + 1 }
extern "system" fn original2(x: u32) -> u32 { x + 2 }

extern "system" fn hooked1(x: u32) -> u32 { x + 0o721 }
extern "system" fn hooked2(x: u32) -> u32 { x + 0o722 }

// Create a collection of hooks
let mut hooks = InlineHookMap::new();
hooks.insert::<MyFn>(original1, hooked1);
// Insert and enable a hook
hooks.insert::<MyFn>(original2, hooked2).enable().unwrap();

// Enable all hooks at once
hooks.enable().on_error(|target, e| eprintln!("Target {target:?} failed: {e:?}"));

// Verify hooks are enabled
assert_eq!(original1(0x100), 721); // redirected to hooked1
assert_eq!(original2(0x100), 722); // redirected to hooked2

// Disable all hooks at once
hooks.disable().on_error(|target, e| eprintln!("Target {target:?} failed: {e:?}"));

// Verify hooks are disabled
assert_eq!(original1(0x100), 0x101); // back to original
assert_eq!(original2(0x100), 0x102); // back to original

// Access individual hooks by target function
if let Some(hook) = hooks.get::<MyFn>(original1) {
    println!("Hook is enabled: {}", hook.is_enabled());
}
```
*/
#[derive(Default)]
pub struct InlineHookMap {
    hooks: HashMap<fn(), InlineHook<fn()>>,
    leaked: bool,
}

#[bon]
impl InlineHookMap {
    /// Creates a new empty [`InlineHookMap`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns an iterator of the hooks.
    pub fn hooks<'a>(&'a self) -> Values<'a, fn(), InlineHook<fn()>> {
        self.hooks.values()
    }

    /// Returns a mutable iterator of the hooks.
    pub fn hooks_mut<'a>(&'a mut self) -> ValuesMut<'a, fn(), InlineHook<fn()>> {
        self.hooks.values_mut()
    }

    /// Returns the number of hooks in the collection.
    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    /// Returns `true` if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    /// Add a new hook to the collection.
    ///
    /// The hook is created but not enabled. Use [`enable()`](InlineHookMap::enable) to enable it.
    pub fn insert<'a, F: FnPtr>(&'a mut self, target: F, detour: F) -> &'a mut InlineHook<F> {
        let hook = unsafe { InlineHook::new(target, detour).into_type_erased() };
        // OccupiedEntry<'a, fn(), InlineHook<fn()>>
        let entry = self.hooks.entry(hook.target()).insert_entry(hook);
        // Likely undefined behavior, but anyway...
        // unsafe { transmute::<OccupiedEntry<'a, F, InlineHook<F>>>(entry) }

        // Only get()/get_mut()/into_mut() and remove() are useful,
        // but there can only be one &mut, short remove() isn't quite useful.
        let entry: &'a mut InlineHook<fn()> = entry.into_mut();
        unsafe { entry.cast_mut() }
    }

    /// Enable all hooks in the collection.
    ///
    /// Errors are reported via the `on_error` callback (if provided).
    /// Hooks that fail will remain disabled.
    ///
    /// TODO: Transaction
    #[builder]
    pub fn enable(&mut self, mut on_error: Option<impl FnMut(fn(), HRESULT)>) {
        for (target, hook) in self.hooks.iter_mut() {
            let hr = hook.enable();
            if !hr.is_ok() {
                if let Some(on_error) = on_error.as_mut() {
                    on_error(*target, hr);
                }
            }
        }
    }

    /// Disable all hooks in the collection.
    ///
    /// Errors are reported via the `on_error` callback (if provided).
    /// Hooks that fail will remain enabled.
    ///
    /// TODO: Transaction
    #[builder]
    pub fn disable(&mut self, mut on_error: Option<impl FnMut(fn(), HRESULT)>) {
        for (target, hook) in self.hooks.iter_mut() {
            let hr = hook.disable();
            if !hr.is_ok() {
                if let Some(on_error) = on_error.as_mut() {
                    on_error(*target, hr);
                }
            }
        }
    }

    /// Get a reference to a specific hook by target function.
    pub fn get<F: FnPtr>(&self, target: F) -> Option<&InlineHook<F>> {
        let target = unsafe { transmute_copy(&target) };
        let hook: Option<&InlineHook<fn()>> = self.hooks.get(&target);
        unsafe { transmute(hook) }
    }

    /// Get a mutable reference to a specific hook by target function.
    pub fn get_mut<F: FnPtr>(&mut self, target: F) -> Option<&mut InlineHook<F>> {
        let target = unsafe { transmute_copy(&target) };
        let hook: Option<&mut InlineHook<fn()>> = self.hooks.get_mut(&target);
        unsafe { transmute(hook) }
    }

    /// Remove a hook from the collection by target function.
    pub fn remove<F: FnPtr>(&mut self, target: F) -> Option<InlineHook<F>> {
        let target = unsafe { transmute_copy(&target) };
        self.hooks
            .remove(&target)
            .map(|hook| unsafe { hook.cast_into() })
    }

    /// Leak all hooks, preventing automatic [`disable()`](Self::disable) on drop.
    pub fn leak(&mut self) {
        self.leaked = true;
    }
}

impl Drop for InlineHookMap {
    fn drop(&mut self) {
        if self.leaked {
            let hooks = mem::take(&mut self.hooks);
            hooks
                .into_values()
                .map(|hook| mem::forget(hook))
                .for_each(|()| ());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::inline::tests::TEST_MUTEX;

    use super::*;

    #[test]
    fn doc() {
        let _guard = TEST_MUTEX.lock().unwrap();

        type MyFn = extern "system" fn(u32) -> u32;

        extern "system" fn original1(x: u32) -> u32 {
            x + 1
        }
        extern "system" fn original2(x: u32) -> u32 {
            x + 2
        }

        extern "system" fn hooked1(x: u32) -> u32 {
            x + 0o721
        }
        extern "system" fn hooked2(x: u32) -> u32 {
            x + 0o722
        }

        // Create a collection of hooks
        let mut hooks = InlineHookMap::new();
        hooks.insert::<MyFn>(original1, hooked1);
        // Insert and enable a hook
        hooks.insert::<MyFn>(original2, hooked2).enable().unwrap();

        // Enable all hooks at once
        hooks
            .enable()
            .on_error(|target, e| eprintln!("Target {target:?} failed: {e:?}"))
            .call();

        // Verify hooks are enabled
        assert_eq!(original1(0x100), 721); // redirected to hooked1
        assert_eq!(original2(0x100), 722); // redirected to hooked2

        // Disable all hooks at once
        hooks
            .disable()
            .on_error(|target, e| eprintln!("Target {target:?} failed: {e:?}"))
            .call();

        // Verify hooks are disabled
        assert_eq!(original1(0x100), 0x101); // back to original
        assert_eq!(original2(0x100), 0x102); // back to original

        // Access individual hooks by target function
        if let Some(hook) = hooks.get::<MyFn>(original1) {
            println!("Hook is enabled: {}", hook.is_enabled());
        }
    }
}
