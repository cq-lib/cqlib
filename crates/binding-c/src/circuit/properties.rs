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

use crate::circuit::CCircuit;
use cqlib_core::circuit::{Parameter, Qubit};

// =====  Section 2.1.7: Circuit properties and auxiliary operations  =====

/// Return the circuit depth. Negative values indicate errors.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_depth(ptr: *const CCircuit, recurse: bool) -> isize {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.depth(recurse).map_or(-3, |d| d as isize) }
}

/// Return the circuit width (number of qubits), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_width(ptr: *const CCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.width() }
}

/// Return the number of qubits in the circuit (for array allocation), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_qubits_len(ptr: *const CCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.qubits().len() }
}

/// Copy qubit identifiers into `out`. Returns the total number of qubits.
/// If `out` is NULL or `len` is too small, only the count is returned (no copy).
#[unsafe(no_mangle)]
pub extern "C" fn circuit_qubits(ptr: *const CCircuit, out: *mut u32, len: usize) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let qubits = unsafe { (*ptr).inner.qubits() };
    let total = qubits.len();
    if !out.is_null() {
        let count = total.min(len);
        for (i, q) in qubits.iter().take(count).enumerate() {
            unsafe {
                *out.add(i) = q.id();
            }
        }
    }
    total
}

/// Remove the operation at `index`. Returns 0 on success, negative on error.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_remove_operation(ptr: *mut CCircuit, index: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.remove_operation(index).map_or(-3, |_| 0)
}

/// Return a new circuit that is the inverse of the input, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_inverse(ptr: *const CCircuit) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*ptr).inner.inverse() } {
        Ok(inner) => Box::into_raw(Box::new(CCircuit { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return a decomposed copy of the circuit, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_decompose(ptr: *const CCircuit) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*ptr).inner.decompose() } {
        Ok(inner) => Box::into_raw(Box::new(CCircuit { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return the number of complex elements (rows * cols) in the unitary matrix,
/// or 0 on error. Pass `qubits_order` = NULL to use default ordering.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_to_matrix_len(
    ptr: *const CCircuit,
    qubits_order: *const usize,
    order_len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let order = if qubits_order.is_null() || order_len == 0 {
        None
    } else {
        let slice = unsafe { std::slice::from_raw_parts(qubits_order, order_len) };
        Some(slice)
    };
    match unsafe { (*ptr).inner.to_matrix(order) } {
        Ok(matrix) => matrix.len(),
        Err(_) => 0,
    }
}

/// Copy the unitary matrix into `out` as interleaved (real, imag) doubles.
/// Returns the number of complex elements written, or 0 on error.
/// Buffer must hold at least `2 * circuit_to_matrix_len(...)` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_to_matrix(
    ptr: *const CCircuit,
    qubits_order: *const usize,
    order_len: usize,
    out: *mut f64,
    buffer_len: usize,
) -> usize {
    if ptr.is_null() || out.is_null() {
        return 0;
    }
    let order = if qubits_order.is_null() || order_len == 0 {
        None
    } else {
        let slice = unsafe { std::slice::from_raw_parts(qubits_order, order_len) };
        Some(slice)
    };
    match unsafe { (*ptr).inner.to_matrix(order) } {
        Ok(matrix) => {
            let total = matrix.len();
            if buffer_len < total * 2 {
                return 0;
            }
            for (i, c) in matrix.iter().enumerate() {
                unsafe {
                    *out.add(i * 2) = c.re;
                    *out.add(i * 2 + 1) = c.im;
                }
            }
            total
        }
        Err(_) => 0,
    }
}

/// Compose another circuit into this one. `qubits_map` maps `other`'s qubits
/// to this circuit's qubits. NULL map uses identity mapping (merge by ID).
#[unsafe(no_mangle)]
pub extern "C" fn circuit_compose(
    ptr: *mut CCircuit,
    other: *const CCircuit,
    qubits_map: *const u32,
    map_len: usize,
) -> i32 {
    if ptr.is_null() || other.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let other_inner = unsafe { &(*other).inner };
    let result = if qubits_map.is_null() || map_len == 0 {
        wrapper.inner.compose(other_inner, None)
    } else {
        let slice = unsafe { std::slice::from_raw_parts(qubits_map, map_len) };
        let qubits: Vec<Qubit> = slice.iter().map(|&id| Qubit::new(id)).collect();
        wrapper.inner.compose(other_inner, Some(qubits.as_slice()))
    };
    result.map_or(-3, |_| 0)
}

/// Append new qubits to the circuit.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_add_qubits(ptr: *mut CCircuit, qubits: *const u32, count: usize) -> i32 {
    if ptr.is_null() || qubits.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let slice = unsafe { std::slice::from_raw_parts(qubits, count) };
    let new_qubits: Vec<Qubit> = slice.iter().map(|&id| Qubit::new(id)).collect();
    wrapper.inner.add_qubits(new_qubits).map_or(-3, |_| 0)
}

/// Set the global phase to a numeric value.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_set_global_phase(ptr: *mut CCircuit, phase: f64) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    if !phase.is_finite() {
        return -3;
    }
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.set_global_phase(Parameter::from(phase));
    0
}

/// Set the global phase to a symbolic parameter.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_set_global_phase_param(
    ptr: *mut CCircuit,
    param: *const crate::circuit::CParameter,
) -> i32 {
    if ptr.is_null() || param.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let param = unsafe { (*param).inner.clone() };
    wrapper.inner.set_global_phase(param);
    0
}
