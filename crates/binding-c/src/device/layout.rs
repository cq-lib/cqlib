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

//! C ABI for `Layout` (logical-to-physical qubit mapping).

use crate::device::CLayout;
use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};

/// Creates a layout mapping logical qubits `0..num_logical` onto the supplied
/// physical qubits in order.
///
/// `logical` and `physical` are arrays of u32 qubit IDs; both must have the
/// same non-zero length. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn layout_new(
    logical: *const u32,
    num_logical: usize,
    physical: *const u32,
    num_physical: usize,
) -> *mut CLayout {
    if num_logical == 0 || logical.is_null() || num_physical != num_logical || physical.is_null() {
        return std::ptr::null_mut();
    }
    let logical: Vec<_> = unsafe { std::slice::from_raw_parts(logical, num_logical) }
        .iter()
        .map(|&q| LogicalQubit::new(q))
        .collect();
    let physical: Vec<_> = unsafe { std::slice::from_raw_parts(physical, num_physical) }
        .iter()
        .map(|&q| PhysicalQubit::new(q))
        .collect();
    match Layout::new(logical, physical, None) {
        Ok(layout) => Box::into_raw(Box::new(CLayout { inner: layout })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Creates a layout from `(logical, physical)` pairs.
///
/// `pairs` points to `2 * num_pairs` u32 values laid out as consecutive
/// `(logical, physical)` pairs. `physical_count` defines the total number of
/// physical qubits (`0..physical_count`); unreferenced ones stay vacant.
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn layout_from_pairs(
    pairs: *const u32,
    num_pairs: usize,
    physical_count: u32,
) -> *mut CLayout {
    if num_pairs == 0 || pairs.is_null() {
        return std::ptr::null_mut();
    }
    let raw = unsafe { std::slice::from_raw_parts(pairs, num_pairs * 2) };
    let mapped: Vec<(u32, u32)> = raw.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    match Layout::from_pairs(&mapped, physical_count) {
        Ok(layout) => Box::into_raw(Box::new(CLayout { inner: layout })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a layout. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn layout_free(ptr: *mut CLayout) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the physical qubit mapped to `logical`, or `u32::MAX` when
/// unmapped / on error.
#[unsafe(no_mangle)]
pub extern "C" fn layout_get(ptr: *const CLayout, logical: u32) -> u32 {
    if ptr.is_null() {
        return u32::MAX;
    }
    let layout = unsafe { &*ptr };
    match layout.inner.get_physical(LogicalQubit::new(logical)) {
        Some(physical) => physical.id(),
        None => u32::MAX,
    }
}

/// Returns the number of mapped logical qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_num_logical(ptr: *const CLayout) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_logical() }
}

/// Returns the total number of physical qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_num_physical(ptr: *const CLayout) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_physical() }
}
