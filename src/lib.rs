#![no_std]
#![doc = include_str!("../README.md")]

pub mod raw;

use core::{cell::UnsafeCell, ptr::NonNull};
use parking_lot::RwLock;
use type_key::TypeKey;

use crate::raw::RawFnMap;

#[derive(Debug, Default)]
/// Single thread only FnMap implementation.
///
/// This implementation is zero cost.
pub struct FnMap(UnsafeCell<RawFnMap>);

impl FnMap {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn get_ptr<T: 'static + Send>(&self, key_fn: impl FnOnce() -> T) -> NonNull<T> {
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
    #[inline]
    pub fn get<T: 'static + Send>(&self, key: impl FnOnce() -> T) -> &T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { self.get_ptr(key).as_ref() }
    }

    /// Get or compute value using key
    #[inline]
    pub fn get_mut<T: 'static + Send>(&mut self, key: impl FnOnce() -> T) -> &mut T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { self.get_ptr(key).as_mut() }
    }

    /// Reset stored values
    #[inline]
    pub fn reset(&mut self) {
        self.0.get_mut().reset();
    }
}

unsafe impl Send for FnMap {}

#[derive(Debug, Default)]
/// Single thread only and non-Send FnMap implementation
///
/// This implementation is zero cost.
pub struct LocalOnlyFnMap(UnsafeCell<RawFnMap>);

impl LocalOnlyFnMap {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn get_ptr<T: 'static + Send>(&self, key_fn: impl FnOnce() -> T) -> NonNull<T> {
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
    #[inline]
    pub fn get<T: 'static + Send>(&self, key: impl FnOnce() -> T) -> &T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { self.get_ptr(key).as_ref() }
    }

    /// Get or compute value using key
    #[inline]
    pub fn get_mut<T: 'static + Send>(&mut self, key: impl FnOnce() -> T) -> &mut T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { self.get_ptr(key).as_mut() }
    }

    /// Reset stored values
    #[inline]
    pub fn reset(&mut self) {
        self.0.get_mut().reset();
    }
}

#[derive(Debug, Default)]
/// Thread safe FnMap implementation.
///
/// Uses parking_lot's [`RwLock`] to accuire mutable access to Map.
pub struct ConcurrentFnMap(RwLock<RawFnMap>);

impl ConcurrentFnMap {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn get_ptr<T: 'static + Send + Sync>(&self, key_fn: impl FnOnce() -> T) -> NonNull<T> {
        let key = TypeKey::of_val(&key_fn);

        if let Some(ptr) = self.0.read().get(&key) {
            return ptr;
        }

        let value = key_fn();

        self.0.write().insert(key, value)
    }

    /// Get or compute value using key
    #[inline]
    pub fn get<T: 'static + Send + Sync>(&self, key_fn: impl FnOnce() -> T) -> &T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { self.get_ptr(key_fn).as_ref() }
    }

    /// Get or compute value using key
    #[inline]
    pub fn get_mut<T: 'static + Send + Sync, F>(&mut self, key_fn: impl FnOnce() -> T) -> &mut T {
        // SAFETY: pointer is valid and reference cannot outlive more than Self
        unsafe { self.get_ptr(key_fn).as_mut() }
    }

    /// Reset stored values
    #[inline]
    pub fn reset(&mut self) {
        self.0.get_mut().reset();
    }
}

unsafe impl Send for ConcurrentFnMap {}
unsafe impl Sync for ConcurrentFnMap {}

#[cfg(test)]
mod tests {
    use crate::LocalOnlyFnMap;

    use super::{ConcurrentFnMap, FnMap};

    #[test]
    fn test_trait() {
        const fn is_send<T: Send>() {}
        const fn is_sync<T: Sync>() {}

        is_send::<FnMap>();

        is_send::<ConcurrentFnMap>();
        is_sync::<ConcurrentFnMap>();
    }

    #[test]
    fn test_local() {
        let map = FnMap::new();

        fn one() -> i32 {
            1
        }

        let b = map.get(|| map.get(one) + 1);
        let a = map.get(one);

        assert_eq!(*b, 2);
        assert_eq!(*a, 1);
    }

    #[test]
    fn test_local_only() {
        let map = LocalOnlyFnMap::new();

        fn one() -> i32 {
            1
        }

        let b = map.get(|| map.get(one) + 1);
        let a = map.get(one);

        assert_eq!(*b, 2);
        assert_eq!(*a, 1);
    }

    #[test]
    fn test_atomic() {
        let map = ConcurrentFnMap::new();

        fn one() -> i32 {
            1
        }

        let b = map.get(|| map.get(one) + 1);
        let a = map.get(one);

        assert_eq!(*b, 2);
        assert_eq!(*a, 1);
    }
}
