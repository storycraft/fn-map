/*
 * Created on Thu Jun 29 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

#![doc = include_str!("../README.md")]

pub mod raw;

use parking_lot::RwLock;
use std::cell::RefCell;

use crate::raw::RawFnStore;

#[derive(Debug, Default)]
/// Single thread only FnStore implementation.
///
/// Uses RefCell to borrow inner Map mutably.
pub struct LocalFnStore<'a>(RefCell<RawFnStore<'a>>);

impl<'a> LocalFnStore<'a> {
    pub fn new() -> Self {
        Self::default()
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

unsafe impl Send for LocalFnStore<'_> {}

#[derive(Debug, Default)]
/// Single thread only and non-Send FnStore implementation.
///
/// Uses RefCell to borrow inner Map mutably.
pub struct LocalOnlyFnStore<'a>(RefCell<RawFnStore<'a>>);

impl<'a> LocalOnlyFnStore<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_ptr<T: 'a, F: FnOnce() -> T>(&self, key: F) -> *const T {
        if let Some(ptr) = self.0.borrow().get_ptr(&key) {
            return ptr;
        }

        let value = key();
        self.0.borrow_mut().insert_ptr::<F, T>(value)
    }

    /// Get or compute value using key
    pub fn get<T: 'a, F: FnOnce() -> T>(&self, key: F) -> &T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { &*self.get_ptr(key) }
    }

    /// Get or compute value using key
    pub fn get_mut<T: 'a, F: FnOnce() -> T>(&mut self, key: F) -> &mut T {
        // SAFETY: pointer is valid and exclusively reference cannot outlive more than Self
        unsafe { &mut *self.0.get_mut().get_or_insert_ptr(key).cast_mut() }
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
pub struct AtomicFnStore<'a>(RwLock<RawFnStore<'a>>);

impl<'a> AtomicFnStore<'a> {
    pub fn new() -> Self {
        Self::default()
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
