/*
 * Created on Thu Jun 29 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

#![doc = include_str!("../README.md")]

use std::collections::HashMap;
use core::{ptr, mem, marker::PhantomData};

use bumpalo::Bump;
use type_key::TypeKey;

#[derive(Debug)]
/// A Persistent value store using closure as key and storing its return value.
pub struct FnStore<'a> {
    map: HashMap<TypeKey, ManuallyDealloc>,

    // Ensure allocator always drops later than its value to prevent UB
    bump: Bump,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> FnStore<'a> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            bump: Bump::new(),
            _phantom: PhantomData,
        }
    }

    pub fn get<T: 'a>(&mut self, key: impl FnOnce() -> T) -> &mut T {
        // SAFETY: The pointer does not outlive its value and exclusively borrowed.
        unsafe { &mut *self.get_ptr(key).cast_mut() }
    }

    pub fn get_ptr<T: 'a>(&mut self, key: impl FnOnce() -> T) -> *const T {
        let val = self
            .map
            .entry(TypeKey::of_val(&key))
            .or_insert_with(|| 
                // SAFETY: Exclusively borrowed reference, original value is forgotten by Bump allocator and does not outlive.
                unsafe { ManuallyDealloc::new(self.bump.alloc((key)())) });

         val.ptr().cast::<T>()
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.bump.reset();
    }
}

impl Default for FnStore<'_> {
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
        Self(
            mem::transmute::<&mut dyn Erased, &mut dyn Erased>(reference)
                as *mut _,
        )
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
    use crate::FnStore;

    #[test]
    fn test() {
        let mut store = FnStore::new();
        
        let a = store.get(|| 1);
        assert_eq!(*a, 1);
    }
}
