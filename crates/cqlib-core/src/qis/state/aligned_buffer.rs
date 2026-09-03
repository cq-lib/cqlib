// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! 64-byte aligned heap buffer for SIMD-friendly memory allocation.
//!
//! The standard `Vec<T>` only guarantees alignment equal to `align_of::<T>()`,
//! which is 8 bytes for `u64` and `f64` (and therefore 16 bytes for
//! `Complex64`). This is insufficient for AVX2 aligned loads (`vmovdqa`,
//! `_mm256_load_si256`) which require 32-byte alignment, or AVX-512 which
//! requires 64 bytes.
//!
//! `AlignedBuffer<T>` allocates with a 64-byte `Layout`, satisfying all
//! SIMD tiers up to AVX-512. Aligned loads/stores eliminate the 10–20%
//! throughput penalty of cross-cache-line accesses on x86.
//!
//! # Safety contract
//!
//! Callers that use the raw pointer for SIMD must ensure:
//! - Index arithmetic stays within `len` elements.
//! - No aliasing between concurrent mutable references (the usual Rust rules).
//!
use std::alloc::{Layout, alloc_zeroed, dealloc, handle_alloc_error};
use std::ptr::NonNull;

/// A heap-allocated buffer of `T` elements with 64-byte alignment.
///
/// Implements [`Deref<Target=[T]>`] and [`DerefMut`], so it behaves like a
/// slice in most contexts. All SIMD intrinsics that require aligned memory
/// (e.g. `_mm256_load_si256`, `_mm256_load_pd`) are safe to use on the raw
/// pointer returned by `as_ptr()`.
pub(super) struct AlignedBuffer<T> {
    ptr: NonNull<T>,
    len: usize,
    layout: Layout,
}

impl<T: Copy + Default> AlignedBuffer<T> {
    /// Allocates `len` zero-initialised elements at 64-byte alignment.
    ///
    /// Uses `alloc_zeroed` so all bytes are set to 0; this is equivalent to
    /// `T::default()` for integer and float types (which have zero-value at
    /// all-bits-zero).
    ///
    /// # Panics
    /// Panics (via `handle_alloc_error`) if the allocation fails.
    pub(super) fn new_zeroed(len: usize) -> Self {
        let size = len * size_of::<T>();
        // SAFETY: size > 0 for len > 0; align is a valid power-of-two.
        let layout = Layout::from_size_align(size.max(1), 64).expect("valid layout");
        let raw = unsafe { alloc_zeroed(layout) };
        let ptr = NonNull::new(raw as *mut T).unwrap_or_else(|| handle_alloc_error(layout));
        AlignedBuffer { ptr, len, layout }
    }

    /// Returns the raw pointer to the first element (64-byte aligned).
    #[allow(dead_code)]
    #[inline(always)]
    pub(super) fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Returns a shared slice view of the buffer.
    #[inline(always)]
    pub(super) fn as_slice(&self) -> &[T] {
        // SAFETY: ptr valid for `len` elements, allocated and initialised.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// Returns a mutable slice view of the buffer.
    #[inline(always)]
    pub(super) fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: ptr valid for `len` elements, unique (we hold &mut self).
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> Drop for AlignedBuffer<T> {
    fn drop(&mut self) {
        // SAFETY: ptr was allocated with this exact layout; not yet freed.
        unsafe { dealloc(self.ptr.as_ptr() as *mut u8, self.layout) }
    }
}

impl<T: Copy + Default> Clone for AlignedBuffer<T> {
    fn clone(&self) -> Self {
        let new_buf = AlignedBuffer::new_zeroed(self.len);
        // SAFETY: both buffers valid for `len` elements, non-overlapping.
        unsafe {
            std::ptr::copy_nonoverlapping(self.ptr.as_ptr(), new_buf.ptr.as_ptr(), self.len);
        }
        new_buf
    }
}

impl<T: Copy + Default> std::ops::Deref for AlignedBuffer<T> {
    type Target = [T];
    #[inline(always)]
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: Copy + Default> std::ops::DerefMut for AlignedBuffer<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: Copy + Default + std::fmt::Debug> std::fmt::Debug for AlignedBuffer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AlignedBuffer<{}>(len={})",
            std::any::type_name::<T>(),
            self.len
        )
    }
}

// SAFETY: AlignedBuffer owns its allocation uniquely; no shared mutable state.
unsafe impl<T: Send + Copy + Default> Send for AlignedBuffer<T> {}
unsafe impl<T: Sync + Copy + Default> Sync for AlignedBuffer<T> {}
