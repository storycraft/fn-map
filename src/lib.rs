/*
 * Created on Thu Jun 29 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

#![doc = include_str!("../README.md")]

use core::{marker::PhantomData, mem, ptr};
use std::{cell::RefCell, collections::HashMap};

use bumpalo::Bump;
use parking_lot::RwLock;
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

    pub fn get_ptr<T: 'a>(&self, key: &impl FnOnce() -> T) -> Option<*const T> {
        Some(self.map.get(&TypeKey::of_val(key))?.ptr().cast::<T>())
    }

    pub fn insert_ptr<F: FnOnce() -> T, T: 'a>(&mut self, value: T) -> *const T {
        // SAFETY: Exclusively borrowed reference, original value is forgotten by Bump allocator and does not outlive
        let value = unsafe { ManuallyDealloc::new(self.bump.alloc(value)) };
        let ptr = value.ptr();

        self.map.insert(TypeKey::of::<F>(), value);

        ptr.cast::<T>()
    }

    pub fn get_or_insert_ptr<T: 'a>(&mut self, key: impl FnOnce() -> T) -> *const T {
        // SAFETY: Exclusively borrowed reference, original value is forgotten by Bump allocator and does not outlive
        self.map
            .entry(TypeKey::of_val(&key))
            .or_insert_with(|| unsafe { ManuallyDealloc::new(self.bump.alloc(key())) })
            .ptr()
            .cast::<T>()
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

#[derive(Debug)]
/// Single thread only FnStore implementation.
///
/// Uses RefCell to borrow inner Map mutably.
pub struct LocalFnStore<'a>(RefCell<RawFnStore<'a>>);

impl<'a> LocalFnStore<'a> {
    pub fn new() -> Self {
        Self(RefCell::new(RawFnStore::new()))
    }

    pub fn get_ptr<T: 'a + Send, F: FnOnce() -> T>(&self, key: F) -> *const T {
        if let Some(ptr) = self.0.borrow().get_ptr(&key) {
            return ptr;
        }

        let value = key();
        self.0.borrow_mut().insert_ptr::<F, T>(value)
    }

    /// Get or compute value using key
    pub fn get<T: 'a + Send, F: FnOnce() -> T>(&self, key: F) -> &T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { &*self.get_ptr(key) }
    }

    /// Get or compute value using key
    pub fn get_mut<T: 'a + Send, F: FnOnce() -> T>(&mut self, key: F) -> &mut T {
        // SAFETY: pointer is valid and exclusively reference cannot outlive more than Self
        unsafe { &mut *self.0.get_mut().get_or_insert_ptr(key).cast_mut() }
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

unsafe impl Send for LocalFnStore<'_> {}

#[derive(Debug)]
/// Thread safe FnStore implementation.
///
/// Uses parking_lot's [`RwLock`] to accuire mutable access to Map.
pub struct AtomicFnStore<'a>(RwLock<RawFnStore<'a>>);

impl<'a> AtomicFnStore<'a> {
    pub fn new() -> Self {
        Self(RwLock::new(RawFnStore::new()))
    }

    pub fn get_ptr<T: 'a + Send + Sync, F: FnOnce() -> T>(&self, key: F) -> *const T {
        if let Some(ptr) = self.0.read().get_ptr(&key) {
            return ptr;
        }

        let value = key();
        self.0.write().insert_ptr::<F, T>(value)
    }

    /// Get or compute value and insert using key
    pub fn get<T: 'a + Send + Sync, F: FnOnce() -> T>(&self, key: F) -> &T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { &*self.get_ptr(key) }
    }

    /// Get or compute value using key
    pub fn get_mut<T: 'a + Send + Sync, F: FnOnce() -> T>(&mut self, key: F) -> &mut T {
        // SAFETY: pointer is valid and exclusive reference cannot outlive more than Self
        unsafe { &mut *self.0.get_mut().get_or_insert_ptr(key).cast_mut() }
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

unsafe impl Send for AtomicFnStore<'_> {}
unsafe impl Sync for AtomicFnStore<'_> {}

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
    /// Calling this function is only safe if the value is forgotten.
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
    use crate::{AtomicFnStore, LocalFnStore};

    #[test]
    fn test_trait() {
        const fn is_send<T: Send>() {}
        const fn is_sync<T: Sync>() {}

        is_send::<LocalFnStore>();

        is_send::<AtomicFnStore>();
        is_sync::<AtomicFnStore>();
    }

    #[test]
    fn test_local() {
        let store = LocalFnStore::new();

        fn one() -> i32 {
            1
        }

        let b = store.get(|| store.get(one) + 1);
        let a = store.get(one);

        assert_eq!(*b, 2);
        assert_eq!(*a, 1);
    }

    #[test]
    fn test_atomic() {
        let store = AtomicFnStore::new();

        fn one() -> i32 {
            1
        }

        let b = store.get(|| store.get(one) + 1);
        let a = store.get(one);

        assert_eq!(*b, 2);
        assert_eq!(*a, 1);
    }
}
