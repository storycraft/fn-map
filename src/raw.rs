/*
 * Created on Thu Jul 06 2023
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use core::{hash::BuildHasherDefault, marker::PhantomData, mem, mem::ManuallyDrop, ptr};

use bumpalo::Bump;
use hashbrown::HashMap;
use rustc_hash::FxHasher;
use type_key::TypeKey;

#[derive(Debug)]
/// A raw persistent value store
pub struct RawStore<'a> {
    map: HashMap<TypeKey, ManuallyDealloc, BuildHasherDefault<FxHasher>>,

    bump: ManuallyDrop<Bump>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> RawStore<'a> {
    pub fn new() -> Self {
        Self {
            map: HashMap::default(),

            bump: ManuallyDrop::new(Bump::new()),
            _phantom: PhantomData,
        }
    }

    pub fn get<T: 'a>(&self, key: &TypeKey) -> Option<*const T> {
        Some(self.map.get(key)?.ptr().cast::<T>())
    }

    /// insert value
    ///
    /// Returned pointer is covariant to lifetime 'a where &'a mut self
    pub fn insert<T: 'a>(&mut self, key: TypeKey, value: T) -> *const T {
        // SAFETY: Exclusively borrowed reference, original value is forgotten by Bump allocator and does not outlive
        let value =
            unsafe { ManuallyDealloc(mem::transmute(self.bump.alloc(value) as &mut dyn Erased)) };
        let ptr = value.ptr();

        self.map.insert(key, value);

        ptr.cast::<T>()
    }

    pub fn reset(&mut self) {
        self.map.clear();
        self.bump.reset();
    }
}

impl Default for RawStore<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RawStore<'_> {
    fn drop(&mut self) {
        self.map.clear();

        // SAFETY: Manually dropped to ensure allocated objects to drop first
        unsafe { ManuallyDrop::drop(&mut self.bump) }
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
    pub const fn ptr(&self) -> *const dyn Erased {
        self.0.cast_const()
    }
}

impl Drop for ManuallyDealloc {
    fn drop(&mut self) {
        // SAFETY: Safe to drop since it is the only unique pointer
        unsafe { ptr::drop_in_place(self.0) }
    }
}
