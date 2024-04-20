/*
 * Created on Thu Jun 29 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

#![doc = include_str!("../README.md")]

pub mod raw;

use parking_lot::RwLock;
use type_key::TypeKey;
use std::cell::UnsafeCell;

use crate::raw::RawStore;

#[derive(Debug, Default)]
/// Single thread only FnStore implementation.
///
/// This implementation is zero cost.
pub struct LocalFnStore<'a>(UnsafeCell<RawStore<'a>>);

impl<'a> LocalFnStore<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_ptr<T: 'a + Send>(&self, key_fn: impl FnOnce() -> T) -> *const T {
        let key = TypeKey::of_val(&key_fn);

        // SAFETY: safe to borrow shared because self is borrowed shared
        if let Some(ptr) = unsafe { &*self.0.get().cast_const() }.get(&key) {
            return ptr;
        }

        // accuire value first before borrowing exclusively
        let value = key_fn();

        // SAFETY: safe to borrow exclusively since no one can borrow more
        unsafe { &mut *self.0.get() }.insert(key, value)
    }

    /// Get or compute value using key
    pub fn get<T: 'a + Send>(&self, key: impl FnOnce() -> T) -> &T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { &*self.get_ptr(key) }
    }

    /// Get or compute value using key
    pub fn get_mut<T: 'a + Send>(&mut self, key: impl FnOnce() -> T) -> &mut T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { &mut *self.get_ptr(key).cast_mut() }
    }

    /// Reset stored values
    pub fn reset(&mut self) {
        self.0.get_mut().reset();
    }
}

unsafe impl Send for LocalFnStore<'_> {}

#[derive(Debug, Default)]
/// Single thread only and non-Send FnStore implementation
///
/// This implementation is zero cost.
pub struct LocalOnlyFnStore<'a>(UnsafeCell<RawStore<'a>>);

impl<'a> LocalOnlyFnStore<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_ptr<T: 'a + Send>(&self, key_fn: impl FnOnce() -> T) -> *const T {
        let key = TypeKey::of_val(&key_fn);

        // SAFETY: safe to borrow shared because self is borrowed shared
        if let Some(ptr) = unsafe { &*self.0.get().cast_const() }.get(&key) {
            return ptr;
        }

        // accuire value first before borrowing exclusively
        let value = key_fn();

        // SAFETY: safe to borrow exclusively since no one can borrow more
        unsafe { &mut *self.0.get() }.insert(key, value)
    }

    /// Get or compute value using key
    pub fn get<T: 'a + Send>(&self, key: impl FnOnce() -> T) -> &T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { &*self.get_ptr(key) }
    }

    /// Get or compute value using key
    pub fn get_mut<T: 'a + Send>(&mut self, key: impl FnOnce() -> T) -> &mut T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { &mut *self.get_ptr(key).cast_mut() }
    }

    /// Reset stored values
    pub fn reset(&mut self) {
        self.0.get_mut().reset();
    }
}

#[derive(Debug, Default)]
/// Thread safe FnStore implementation.
///
/// Uses parking_lot's [`RwLock`] to accuire mutable access to Map.
pub struct AtomicFnStore<'a>(RwLock<RawStore<'a>>);

impl<'a> AtomicFnStore<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_ptr<T: 'a + Send + Sync>(&self, key_fn: impl FnOnce() -> T) -> *const T {
        let key = TypeKey::of_val(&key_fn);

        if let Some(ptr) = self.0.read().get(&key) {
            return ptr;
        }

        let value = key_fn();

        self.0.write().insert(key, value)
    }

    /// Get or compute value using key
    pub fn get<T: 'a + Send + Sync>(&self, key_fn: impl FnOnce() -> T) -> &T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { &*self.get_ptr(key_fn) }
    }

    /// Get or compute value using key
    pub fn get_mut<T: 'a + Send + Sync, F>(&mut self, key_fn: impl FnOnce() -> T) -> &mut T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { &mut *self.get_ptr(key_fn).cast_mut() }
    }

    /// Reset stored values
    pub fn reset(&mut self) {
        self.0.get_mut().reset();
    }
}

unsafe impl Send for AtomicFnStore<'_> {}
unsafe impl Sync for AtomicFnStore<'_> {}

#[cfg(test)]
mod tests {
    use crate::LocalOnlyFnStore;

    use super::{AtomicFnStore, LocalFnStore};

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
    fn test_local_only() {
        let store = LocalOnlyFnStore::new();

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
