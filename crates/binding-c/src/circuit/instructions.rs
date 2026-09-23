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
use cqlib_core::circuit::Qubit;

// =====  Section 2.1.6: Instructions  =====

/// Insert a barrier over the given qubits. Passing NULL qubits or count=0
/// creates a global barrier over all circuit qubits.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_barrier(ptr: *mut CCircuit, qubits: *const u32, count: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let qubit_vec: Vec<Qubit> = if qubits.is_null() || count == 0 {
        wrapper.inner.qubits()
    } else {
        let slice = unsafe { std::slice::from_raw_parts(qubits, count) };
        slice.iter().map(|&id| Qubit::new(id)).collect()
    };
    wrapper.inner.barrier(qubit_vec).map_or(-3, |_| 0)
}
