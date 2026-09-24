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

//! C ABI for Pauli evolution gates appended to a circuit.

use crate::circuit::{CCircuit, check_qubit};
use crate::error::CqlibError;
use crate::qis::CPauliString;
use cqlib_core::circuit::ParameterValue;
use cqlib_core::qis::evolution::PauliEvolution;

/// Append the Pauli evolution operator `exp(-i * angle / 2 * P)` to `circuit`.
///
/// `pauli` is the Pauli string `P` to exponentiate. Its phase must be
/// Hermitian (+/-1). `angle` is a fixed rotation angle. `qubits` lists the
/// circuit positions the Pauli string acts on, in the same order as the
/// Pauli string positions; its length must equal
/// `pauli_string_num_qubits(pauli)`.
///
/// Returns 0 on success, -1 on NULL arguments, -2 when a qubit index is out
/// of bounds, -3 when the underlying circuit construction fails (for example
/// a position-count mismatch or a non-Hermitian Pauli phase), or -8 when the
/// angle is not finite.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_pauli_evolution(
    circuit: *mut CCircuit,
    pauli: *const CPauliString,
    angle: f64,
    qubits: *const u32,
    qubits_len: usize,
) -> i32 {
    if circuit.is_null() || pauli.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if qubits.is_null() && qubits_len > 0 {
        return CqlibError::NullPtr as i32;
    }
    if !angle.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    let wrapper = unsafe { &mut *circuit };
    let pauli = unsafe { &(*pauli).inner };
    let mut qubit_list = Vec::with_capacity(qubits_len);
    for i in 0..qubits_len {
        let id = unsafe { *qubits.add(i) };
        match check_qubit(&wrapper.inner, id) {
            Ok(qubit) => qubit_list.push(qubit),
            Err(code) => return code,
        }
    }
    match wrapper
        .inner
        .pauli_evolution(pauli, ParameterValue::Fixed(angle), &qubit_list)
    {
        Ok(()) => 0,
        Err(_) => CqlibError::CircuitError as i32,
    }
}
