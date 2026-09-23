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
use cqlib_core::circuit::Circuit;

/// Create a new quantum circuit with `num_qubits` logical qubits.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_new(num_qubits: usize) -> *mut CCircuit {
    Box::into_raw(Box::new(CCircuit {
        inner: Circuit::new(num_qubits),
    }))
}

/// Free a quantum circuit. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_free(ptr: *mut CCircuit) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Return the number of qubits in the circuit, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_num_qubits(ptr: *const CCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Return the number of operations in the circuit, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_num_operations(ptr: *const CCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.operations().len() }
}

/// Return the number of interned symbolic parameters in the circuit, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_num_parameters(ptr: *const CCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.parameters().len() }
}

/// Validate circuit consistency.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_validate(ptr: *const CCircuit) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.validate().map_or(-3, |_| 0) }
}
