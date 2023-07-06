/*
 * Created on Thu Jul 06 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use core::{marker::PhantomData, mem, ptr};

use bumpalo::Bump;
use hashbrown::HashMap;
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
