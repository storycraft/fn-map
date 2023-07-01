/*
 * Created on Thu Jun 29 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

#![doc = include_str!("../README.md")]

use core::{marker::PhantomData, mem, ptr};
use std::{cell::RefCell, collections::HashMap};

use bumpalo::Bump;
use parking_lot::Mutex;
use type_key::TypeKey;

#[derive(Debug)]
/// A raw persistent value store using closure as key and storing its return value.
pub struct RawFnStore<'a> {
    map: HashMap<TypeKey, ManuallyDealloc>,

    // Ensure allocator always drops later than its value to prevent UB
    bump: Bump,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> RawFnStore<'a> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            bump: Bump::new(),
            _phantom: PhantomData,
        }
    }

    pub fn get_ptr<T: 'a + Send>(&mut self, key: impl FnOnce() -> T) -> *const T {
        let val = self.map.entry(TypeKey::of_val(&key)).or_insert_with(|| {
            // SAFETY: Exclusively borrowed reference, original value is forgotten by Bump allocator and does not outlive.
            unsafe { ManuallyDealloc::new(self.bump.alloc((key)())) }
        });

        val.ptr().cast::<T>()
    }

    pub fn reset(&mut self) {
        self.map.clear();
        self.bump.reset();
    }
}

impl Default for RawFnStore<'_> {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for RawFnStore<'_> {}

#[derive(Debug)]
/// Single thread only FnStore implementation.
///
/// Uses RefCell to borrow inner Map mutably.
pub struct LocalFnStore<'a>(RefCell<RawFnStore<'a>>);

impl<'a> LocalFnStore<'a> {
    pub fn new() -> Self {
        Self(RefCell::new(RawFnStore::new()))
    }

    /// Get or compute value using key
    pub fn get<T: 'a + Send>(&self, key: impl FnOnce() -> T) -> &T {
        let ptr = self.0.borrow_mut().get_ptr(key);

        // SAFETY: pointer is valid and its reference cannot outlive more than Self
        unsafe { &*ptr }
    }

    /// Reset stored values
    pub fn reset(&mut self) {
        self.0.get_mut().reset();
    }
}

impl Default for LocalFnStore<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
/// Thread safe FnStore implementation.
///
/// Uses parking_lot's [`Mutex`] to accuire mutable access to Map.
pub struct AtomicFnStore<'a>(Mutex<RawFnStore<'a>>);

impl<'a> AtomicFnStore<'a> {
    pub fn new() -> Self {
        Self(Mutex::new(RawFnStore::new()))
    }

    /// Get or compute value and insert using key
    pub fn get<T: 'a + Send>(&self, key: impl FnOnce() -> T) -> &T {
        let ptr = self.0.lock().get_ptr(key);

        // SAFETY: pointer is valid and its reference cannot outlive more than Self
        unsafe { &*ptr }
    }

    /// Reset stored values
    pub fn reset(&mut self) {
        self.0.get_mut().reset();
    }
}

impl Default for AtomicFnStore<'_> {
    fn default() -> Self {
        Self::new()
    }
}

trait Erased {}
impl<T> Erased for T {}

#[derive(Debug)]
#[repr(transparent)]
/// Manually deallocated pointer.
/// It's intended to be used with bump allocator.
///
/// # Safety
/// Dereferencing the pointer is only safe when the pointer did not outlive its value
struct ManuallyDealloc(*mut dyn Erased);

impl ManuallyDealloc {
    /// # Safety
    /// Calling this function is only safe if the value referenced by `reference is forgotten.
    pub unsafe fn new<T>(reference: &mut T) -> Self {
        Self(mem::transmute::<&mut dyn Erased, &mut dyn Erased>(reference) as *mut _)
    }

    pub const fn ptr(&self) -> *const dyn Erased {
        self.0.cast_const()
    }
}

impl Drop for ManuallyDealloc {
    fn drop(&mut self) {
        // SAFETY: Safe to drop since its original was forgotten and only the pointer is pointing the value. See [`ManuallyDealloc::new`]
        unsafe { ptr::drop_in_place(self.0) }
    }
}

#[cfg(test)]
mod tests {
    use crate::{AtomicFnStore, LocalFnStore, RawFnStore};

    #[test]
    fn test_trait() {
        const fn is_send<T: Send>() {}
        const fn is_sync<T: Sync>() {}

        is_send::<RawFnStore>();

        is_send::<LocalFnStore>();

        is_send::<AtomicFnStore>();
        is_sync::<AtomicFnStore>();
    }

    #[test]
    fn test() {
        let store = LocalFnStore::new();

        let a = store.get(|| 1);
        assert_eq!(*a, 1);
    }
}
