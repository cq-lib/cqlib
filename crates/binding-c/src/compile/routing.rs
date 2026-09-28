// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// use this file except in compliance with the License. You may obtain
// a copy of the License in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! C ABI for transform-level routing entry points: `route_with_layout` and
//! `route_sabre`, plus accessors over the routed circuit and the full
//! SABRE layout-plus-route result.

use crate::circuit::CCircuit;
use crate::compile::layout::{CLayoutScore, objective_from_tag, write_layout_score};
use crate::compile::sabre::{
    CSabreRoutingDiagnostics, SabreConfigC, resolve_sabre_config, write_sabre_routing_diagnostics,
};
use crate::device::{CDevice, CLayout};
use crate::error::CqlibError;
use cqlib_core::compile::transform::routing::{RoutedCircuit, SabreRouteResult};
use cqlib_core::compile::transform::routing::{
    route_sabre as core_route_sabre, route_with_layout as core_route_with_layout,
};
use std::ffi::CString;
use std::os::raw::c_char;

/// Opaque handle around a [`RoutedCircuit`].
pub struct CRoutedCircuit {
    pub inner: RoutedCircuit,
}

/// Opaque handle around a [`SabreRouteResult`].
pub struct CSabreRouteResult {
    pub inner: SabreRouteResult,
}

// =====  Routing entry points  =====

/// Routes `circuit` on `device` from a caller-supplied initial layout.
///
/// This is the low-level entry point for callers that already have a layout
/// (for example from `vf2_perfect_layout`, `greedy_layout`, or a previous
/// SABRE run). It does not perform layout refinement or scoring. `config` may
/// be NULL to use the core default. Returns a heap-allocated
/// `CRoutedCircuit*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn route_with_layout(
    circuit: *const CCircuit,
    device: *const CDevice,
    initial_layout: *const CLayout,
    config: *const SabreConfigC,
) -> *mut CRoutedCircuit {
    if circuit.is_null() || device.is_null() || initial_layout.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(config) = resolve_sabre_config(config) else {
        return std::ptr::null_mut();
    };
    let circuit = unsafe { &(*circuit).inner };
    let device = unsafe { &(*device).inner };
    let layout = unsafe { &(*initial_layout).inner };
    match core_route_with_layout(circuit, device, layout, &config) {
        Ok(routed) => Box::into_raw(Box::new(CRoutedCircuit { inner: routed })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Selects a SABRE initial layout for `circuit` on `device`, then routes the
/// original forward circuit from the selected layout.
///
/// `objective` is one of the `LAYOUT_OBJECTIVE_*` tags. `config` may be NULL
/// to use the core default. Returns a heap-allocated `CSabreRouteResult*`, or
/// NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn route_sabre(
    circuit: *const CCircuit,
    device: *const CDevice,
    objective: u8,
    config: *const SabreConfigC,
) -> *mut CSabreRouteResult {
    if circuit.is_null() || device.is_null() {
        return std::ptr::null_mut();
    }
    let device = unsafe { &(*device).inner };
    let Ok(Some(objective)) = objective_from_tag(objective, None, Some(device)) else {
        return std::ptr::null_mut();
    };
    let Ok(config) = resolve_sabre_config(config) else {
        return std::ptr::null_mut();
    };
    let circuit = unsafe { &(*circuit).inner };
    match core_route_sabre(circuit, device, &objective, &config) {
        Ok(result) => Box::into_raw(Box::new(CSabreRouteResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  RoutedCircuit accessors  =====

/// Frees a routed circuit. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn routed_circuit_free(ptr: *mut CRoutedCircuit) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the routed physical circuit as an owned `CCircuit*` (free with
/// `circuit_free`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn routed_circuit_circuit(ptr: *const CRoutedCircuit) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let routed = unsafe { &*ptr };
    Box::into_raw(Box::new(CCircuit {
        inner: routed.inner.circuit().clone(),
    }))
}

/// Returns the initial logical-to-physical layout used for routing, as an
/// owned `CLayout*` (free with `layout_free`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn routed_circuit_initial_layout(ptr: *const CRoutedCircuit) -> *mut CLayout {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let routed = unsafe { &*ptr };
    Box::into_raw(Box::new(CLayout {
        inner: routed.inner.initial_layout().clone(),
    }))
}

/// Returns the final logical-to-physical layout after all routed operations,
/// as an owned `CLayout*` (free with `layout_free`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn routed_circuit_final_layout(ptr: *const CRoutedCircuit) -> *mut CLayout {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let routed = unsafe { &*ptr };
    Box::into_raw(Box::new(CLayout {
        inner: routed.inner.final_layout().clone(),
    }))
}

/// Returns the number of inserted SWAP operations, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn routed_circuit_swap_count(ptr: *const CRoutedCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.swap_count() }
}

/// Writes the routing-diagnostics snapshot to `*out`.
///
/// Returns 0 on success or -1 on NULL arguments.
#[unsafe(no_mangle)]
pub extern "C" fn routed_circuit_diagnostics(
    ptr: *const CRoutedCircuit,
    out: *mut CSabreRoutingDiagnostics,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let routed = unsafe { &*ptr };
    write_sabre_routing_diagnostics(routed.inner.diagnostics(), out);
    CqlibError::Ok as i32
}

/// Returns whether routing observably changed `original`.
///
/// Returns 1 when changed, 0 when unchanged, or -1 on NULL arguments.
#[unsafe(no_mangle)]
pub extern "C" fn routed_circuit_changed(
    ptr: *const CRoutedCircuit,
    original: *const CCircuit,
) -> i32 {
    if ptr.is_null() || original.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let routed = unsafe { &*ptr };
    let original = unsafe { &(*original).inner };
    routed.inner.changed(original) as i32
}

// =====  SabreRouteResult accessors  =====

/// Frees a SABRE route result. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_free(ptr: *mut CSabreRouteResult) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the routed circuit and routing metadata as an owned
/// `CRoutedCircuit*` (free with `routed_circuit_free`). Returns NULL on
/// error.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_routed(ptr: *const CSabreRouteResult) -> *mut CRoutedCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    Box::into_raw(Box::new(CRoutedCircuit {
        inner: result.inner.routed().clone(),
    }))
}

/// Returns the routed physical circuit as an owned `CCircuit*` (free with
/// `circuit_free`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_circuit(ptr: *const CSabreRouteResult) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    Box::into_raw(Box::new(CCircuit {
        inner: result.inner.circuit().clone(),
    }))
}

/// Returns the initial logical-to-physical layout used for routing, as an
/// owned `CLayout*` (free with `layout_free`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_initial_layout(ptr: *const CSabreRouteResult) -> *mut CLayout {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    Box::into_raw(Box::new(CLayout {
        inner: result.inner.initial_layout().clone(),
    }))
}

/// Returns the final logical-to-physical layout after routing, as an owned
/// `CLayout*` (free with `layout_free`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_final_layout(ptr: *const CSabreRouteResult) -> *mut CLayout {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    Box::into_raw(Box::new(CLayout {
        inner: result.inner.final_layout().clone(),
    }))
}

/// Returns the number of inserted SWAP operations, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_swap_count(ptr: *const CSabreRouteResult) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.swap_count() }
}

/// Writes the routing-diagnostics snapshot to `*out`.
///
/// Returns 0 on success or -1 on NULL arguments.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_diagnostics(
    ptr: *const CSabreRouteResult,
    out: *mut CSabreRoutingDiagnostics,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let result = unsafe { &*ptr };
    write_sabre_routing_diagnostics(result.inner.diagnostics(), out);
    CqlibError::Ok as i32
}

/// Returns whether routing observably changed `original`.
///
/// Returns 1 when changed, 0 when unchanged, or -1 on NULL arguments.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_changed(
    ptr: *const CSabreRouteResult,
    original: *const CCircuit,
) -> i32 {
    if ptr.is_null() || original.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let result = unsafe { &*ptr };
    let original = unsafe { &(*original).inner };
    result.inner.changed(original) as i32
}

// =====  SabreRouteResult layout metadata  =====

/// Writes the layout-score snapshot of the selected initial layout to `*out`.
///
/// Returns 0 on success, -1 on NULL arguments, or -8 when the result carries
/// no layout score.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_layout_score(
    ptr: *const CSabreRouteResult,
    out: *mut CLayoutScore,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let result = unsafe { &*ptr };
    match result.inner.layout_score() {
        Some(score) => {
            write_layout_score(score, out);
            CqlibError::Ok as i32
        }
        None => CqlibError::InvalidParam as i32,
    }
}

/// Returns 1 when every positive-weight interaction lands on an adjacent
/// hardware edge, 0 otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_layout_is_perfect(ptr: *const CSabreRouteResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.layout_diagnostics().is_perfect as i32 }
}

/// Returns the number of candidate layouts considered while selecting the
/// initial layout, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_layout_candidates_evaluated(
    ptr: *const CSabreRouteResult,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.layout_diagnostics().candidates_evaluated }
}

/// Returns 1 when fidelity data contributed to the selected layout score, 0
/// otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_layout_used_fidelity(ptr: *const CSabreRouteResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.layout_diagnostics().used_fidelity as i32 }
}

/// Returns the number of layout diagnostic notes, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_layout_notes_len(ptr: *const CSabreRouteResult) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.layout_diagnostics().notes.len() }
}

/// Returns the layout diagnostic note at `index` as a heap-allocated C
/// string.
///
/// Caller must free with `cqlib_string_free`. Returns NULL on out-of-bounds or
/// error.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route_result_layout_note(
    ptr: *const CSabreRouteResult,
    index: usize,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    match result.inner.layout_diagnostics().notes.get(index) {
        Some(note) => match CString::new(note.as_str()) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}
