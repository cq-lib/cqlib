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

use crate::compile::layout::{CCircuitLayoutAnalysis, CPhysicalLayoutGraph, CPreparedSabreCircuit};
use crate::device::{CLayout, write_u32_list, write_u32_pairs};
use crate::error::CqlibError;
use cqlib_core::device::error::LayoutError;
use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};

/// Maps a `LayoutError` to a C error code: unknown physical qubits map to
/// -2, conflicting or missing bindings map to -8.
fn layout_error_code(error: &LayoutError) -> i32 {
    match error {
        LayoutError::InvalidPhysicalQubit(_) => CqlibError::QubitOutOfBounds as i32,
        _ => CqlibError::InvalidParam as i32,
    }
}

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

// =====  Section 3.2: binding, reverse lookup, and map introspection  =====

/// Binds an unmapped `logical` qubit to a vacant `physical` qubit.
///
/// Returns 0 on success, -1 on NULL, -2 when `physical` does not belong to
/// the layout, or -8 when either side already participates in a mapping.
#[unsafe(no_mangle)]
pub extern "C" fn layout_bind(ptr: *mut CLayout, logical: u32, physical: u32) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    match wrapper
        .inner
        .bind(LogicalQubit::new(logical), PhysicalQubit::new(physical))
    {
        Ok(()) => CqlibError::Ok as i32,
        Err(e) => layout_error_code(&e),
    }
}

/// Removes the mapping for `logical` and writes the released physical qubit
/// ID to `*out_physical`.
///
/// Returns 0 on success, -1 on NULL, or -8 when `logical` is not bound.
#[unsafe(no_mangle)]
pub extern "C" fn layout_unbind(ptr: *mut CLayout, logical: u32, out_physical: *mut u32) -> i32 {
    if ptr.is_null() || out_physical.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.unbind(LogicalQubit::new(logical)) {
        Ok(physical) => {
            unsafe { *out_physical = physical.id() };
            CqlibError::Ok as i32
        }
        Err(e) => layout_error_code(&e),
    }
}

/// Returns the logical qubit carried by `physical` (reverse lookup of
/// `layout_get`).
///
/// Returns:
/// - `uint32_t`: logical qubit ID; `physical` **vacant** or unknown returns
///   `UINT32_MAX` (`u32::MAX`), which callers must treat as a sentinel.
#[unsafe(no_mangle)]
pub extern "C" fn layout_get_logical(ptr: *const CLayout, physical: u32) -> u32 {
    if ptr.is_null() {
        return u32::MAX;
    }
    let layout = unsafe { &*ptr };
    match layout.inner.get_logical(PhysicalQubit::new(physical)) {
        Some(logical) => logical.id(),
        None => u32::MAX,
    }
}

/// Returns 1 when `physical` belongs to the layout and is vacant, 0 when it
/// is occupied or unknown, or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_is_physical_vacant(ptr: *const CLayout, physical: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe {
        (*ptr)
            .inner
            .is_physical_vacant(PhysicalQubit::new(physical)) as i32
    }
}

/// Returns the number of logical-to-physical mapping entries, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_l2p_map_len(ptr: *const CLayout) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.l2p_map().len() }
}

/// Copies the logical-to-physical mapping into `out` as consecutive
/// `(logical, physical)` u32 pairs sorted by logical ID (two-step pattern;
/// pair with `layout_l2p_map_len`). `out` must have room for `2 * len`
/// entries. Returns the total number of pairs.
#[unsafe(no_mangle)]
pub extern "C" fn layout_l2p_map(ptr: *const CLayout, out: *mut u32, len: usize) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let pairs: Vec<(u32, u32)> = unsafe {
        (*ptr)
            .inner
            .l2p_map()
            .iter()
            .map(|(l, p)| (l.id(), p.id()))
            .collect()
    };
    write_u32_pairs(&pairs, out, len)
}

/// Returns the number of physical-to-logical mapping entries, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_p2l_map_len(ptr: *const CLayout) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.p2l_map().len() }
}

/// Copies the physical-to-logical mapping into `out` as consecutive
/// `(physical, logical)` u32 pairs sorted by physical ID (two-step pattern;
/// pair with `layout_p2l_map_len`). `out` must have room for `2 * len`
/// entries. Returns the total number of pairs.
#[unsafe(no_mangle)]
pub extern "C" fn layout_p2l_map(ptr: *const CLayout, out: *mut u32, len: usize) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let pairs: Vec<(u32, u32)> = unsafe {
        (*ptr)
            .inner
            .p2l_map()
            .iter()
            .map(|(p, l)| (p.id(), l.id()))
            .collect()
    };
    write_u32_pairs(&pairs, out, len)
}

/// Returns the number of mapped logical qubit IDs, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_logical_qubits_len(ptr: *const CLayout) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.logical_qubits().count() }
}

/// Copies the mapped logical qubit IDs in ascending order into `out`
/// (two-step pattern; pair with `layout_logical_qubits_len`). Returns the
/// total count.
#[unsafe(no_mangle)]
pub extern "C" fn layout_logical_qubits(ptr: *const CLayout, out: *mut u32, len: usize) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let items: Vec<u32> = unsafe { (*ptr).inner.logical_qubits() }
        .map(|q| q.id())
        .collect();
    write_u32_list(&items, out, len)
}

/// Returns the number of physical qubit IDs available to the layout
/// (including vacant ones), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_physical_qubits_len(ptr: *const CLayout) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.physical_qubits().count() }
}

/// Copies all physical qubit IDs available to the layout (including vacant
/// ones) in ascending order into `out` (two-step pattern; pair with
/// `layout_physical_qubits_len`). Returns the total count.
#[unsafe(no_mangle)]
pub extern "C" fn layout_physical_qubits(ptr: *const CLayout, out: *mut u32, len: usize) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let items: Vec<u32> = unsafe { (*ptr).inner.physical_qubits() }
        .map(|q| q.id())
        .collect();
    write_u32_list(&items, out, len)
}

/// Returns the number of vacant physical qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_num_vacant_physical(ptr: *const CLayout) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_vacant_physical() }
}

/// Returns the number of vacant physical qubit IDs, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_vacant_physical_qubits_len(ptr: *const CLayout) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.vacant_physical_qubits().count() }
}

/// Copies the vacant physical qubit IDs in ascending order into `out`
/// (two-step pattern; pair with `layout_vacant_physical_qubits_len`).
/// Returns the total count.
#[unsafe(no_mangle)]
pub extern "C" fn layout_vacant_physical_qubits(
    ptr: *const CLayout,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let items: Vec<u32> = unsafe { (*ptr).inner.vacant_physical_qubits() }
        .map(|q| q.id())
        .collect();
    write_u32_list(&items, out, len)
}

/// Swaps the logical qubits carried by two physical qubits; either side may
/// be vacant (a vacant swap moves the logical qubit).
///
/// Returns 0 on success, -1 on NULL, or -2 when either physical qubit does
/// not belong to the layout.
#[unsafe(no_mangle)]
pub extern "C" fn layout_swap_physical(ptr: *mut CLayout, phys_a: u32, phys_b: u32) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    match wrapper
        .inner
        .swap_physical(PhysicalQubit::new(phys_a), PhysicalQubit::new(phys_b))
    {
        Ok(()) => CqlibError::Ok as i32,
        Err(e) => layout_error_code(&e),
    }
}

// =====  Section 3.2.1: layout result helpers (distance table, activity,
// analysis)  =====

/// Copies the all-pairs undirected shortest-path distance table of the
/// physical layout graph into `out` (two-step pattern).
///
/// The table is row-major over the graph's physical qubits in ascending ID
/// order (`n * n` entries for `n` qubits; pair with
/// `physical_layout_graph_physical_qubits` for the axis order). Unreachable
/// pairs are written as `UINT32_MAX`. A NULL `out` only queries the length.
/// Returns the total number of entries, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_result_distances(
    ptr: *const CPhysicalLayoutGraph,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let table = unsafe { (*ptr).inner.distances() };
    let qubits = table.qubits();
    let n = qubits.len();
    let total = n * n;
    if !out.is_null() {
        for i in 0..total.min(len) {
            let (row, col) = (i / n, i % n);
            let value = table.distance(qubits[row], qubits[col]).unwrap_or(u32::MAX);
            unsafe { *out.add(i) = value };
        }
    }
    total
}

/// Copies the per-logical-qubit activity weights (sum of incident
/// interaction weights) into parallel arrays sorted by logical ID
/// (two-step pattern): `out_qubits[i]` receives the logical qubit ID and
/// `out_activity[i]` its activity. Either array may be NULL only when both
/// are NULL (length query). Returns the total number of entries, or 0 for
/// NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_result_logical_activity(
    ptr: *const CCircuitLayoutAnalysis,
    out_qubits: *mut u32,
    out_activity: *mut f64,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let activity = unsafe { (*ptr).inner.interactions.logical_activity() };
    let total = activity.len();
    if !out_qubits.is_null() && !out_activity.is_null() {
        for (i, (logical, value)) in activity.iter().take(total.min(len)).enumerate() {
            unsafe {
                *out_qubits.add(i) = logical.id();
                *out_activity.add(i) = *value;
            }
        }
    }
    total
}

/// Extracts the reusable circuit layout analysis captured by the prepared
/// SABRE circuit as an owned `CCircuitLayoutAnalysis*` (the same handle type
/// used by `analyze_circuit_for_layout`; free with
/// `circuit_layout_analysis_free`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn layout_result_analysis(
    ptr: *const CPreparedSabreCircuit,
) -> *mut CCircuitLayoutAnalysis {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let analysis = unsafe { (*ptr).inner.analysis() }.clone();
    Box::into_raw(Box::new(CCircuitLayoutAnalysis { inner: analysis }))
}
