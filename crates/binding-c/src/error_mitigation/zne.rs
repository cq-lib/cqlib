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

//! C ABI for `ZNEMitigation` (zero-noise extrapolation helper).

use crate::circuit::CCircuit;
use crate::device::standard_gate_from_name;
use crate::error_mitigation::CCircuitList;
use cqlib_core::circuit::gate::Instruction;
use cqlib_core::error_mitigation::ZNEMitigation;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Opaque handle around [`ZNEMitigation`] for C ABI.
pub struct CZneMitigation {
    pub inner: ZNEMitigation,
}

/// Creates a ZNE helper for `circuit` with the given fold levels
/// (noise factors are derived as `2 * level + 1`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn zne_mitigation_new(
    circuit: *const CCircuit,
    fold_levels: *const i32,
    num_levels: usize,
) -> *mut CZneMitigation {
    if circuit.is_null() || num_levels == 0 || fold_levels.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner }.clone();
    let levels = unsafe { std::slice::from_raw_parts(fold_levels, num_levels) }.to_vec();
    Box::into_raw(Box::new(CZneMitigation {
        inner: ZNEMitigation::new(circuit, levels),
    }))
}

/// Frees a ZNE helper. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn zne_mitigation_free(ptr: *mut CZneMitigation) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Folds the circuit at every configured level.
///
/// `gate_names` is an optional comma-separated list of gate names to fold
/// selectively (e.g. "H,CX"); NULL folds the whole circuit globally.
/// Returns an owned `CCircuitList*` (one folded circuit per level),
/// or NULL on error. Free with `circuit_list_free`.
#[unsafe(no_mangle)]
pub extern "C" fn zne_mitigation_fold_circuit(
    ptr: *const CZneMitigation,
    gate_names: *const c_char,
) -> *mut CCircuitList {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let gate_set = if gate_names.is_null() {
        None
    } else {
        let names = match unsafe { CStr::from_ptr(gate_names) }.to_str() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        let mut gates = Vec::new();
        for part in names.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            match standard_gate_from_name(part) {
                Some(gate) => gates.push(Instruction::Standard(gate)),
                None => return std::ptr::null_mut(),
            }
        }
        Some(gates)
    };
    let zne = unsafe { &(*ptr).inner };
    match zne.fold_circuits(gate_set.as_deref()) {
        Ok(circuits) => Box::into_raw(Box::new(CCircuitList { items: circuits })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns the number of circuits in the list, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_list_len(ptr: *const CCircuitList) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).items.len() }
}

/// Returns the circuit at `index` as an owned `CCircuit*`.
/// Caller must free with `circuit_free`. Returns NULL on out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_list_get(ptr: *const CCircuitList, index: usize) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let list = unsafe { &*ptr };
    match list.items.get(index) {
        Some(circuit) => Box::into_raw(Box::new(CCircuit {
            inner: circuit.clone(),
        })),
        None => std::ptr::null_mut(),
    }
}

/// Frees a circuit list. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_list_free(ptr: *mut CCircuitList) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the noise factor for fold level `index` (`2 * level + 1`),
/// or 0 on error.
#[unsafe(no_mangle)]
pub extern "C" fn zne_mitigation_noise_factor(ptr: *const CZneMitigation, index: usize) -> i32 {
    if ptr.is_null() {
        return 0;
    }
    let zne = unsafe { &(*ptr).inner };
    zne.noise_factors().get(index).copied().unwrap_or(0)
}
