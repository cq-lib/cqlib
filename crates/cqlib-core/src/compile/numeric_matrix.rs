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

//! Target-independent numeric matrix reconstruction for compiler passes.

use crate::circuit::{Instruction, ParameterValue, ValueInstruction, ValueOperation};
use ndarray::Array2;
use num_complex::Complex64;

/// Reconstructs a one-qubit run in circuit execution order, including its
/// global phase: the result is `U_last * ... * U_first`. An empty run is `I₂`.
///
/// Returns `None` unless every operation is a standard one-qubit gate with
/// exactly one qarg, all qargs agree, and each gate has exactly the required
/// number of finite [`ParameterValue::Fixed`] parameters. Matrix construction
/// failures also return `None`; symbolic parameters are never evaluated.
///
/// The caller must collect the run within a single logical/control-flow scope
/// and establish that any intervening operations may be crossed. Value
/// operations do not record their owning scope, so this cannot be checked here.
/// Labels, run boundaries, device feasibility, fusion costs, and the destination
/// of any subsequently extracted phase remain the caller's responsibility.
pub(crate) fn one_qubit_run_matrix(operations: &[ValueOperation]) -> Option<Array2<Complex64>> {
    let mut matrix = Array2::<Complex64>::eye(2);
    let mut run_qubit = None;
    for operation in operations {
        let ValueInstruction::Instruction(Instruction::Standard(gate)) = &operation.instruction
        else {
            return None;
        };
        if gate.num_qubits() != 1 || operation.params.len() != gate.num_params() {
            return None;
        }
        let [qubit] = operation.qubits.as_slice() else {
            return None;
        };
        if run_qubit.is_some_and(|previous| previous != *qubit) {
            return None;
        }
        run_qubit = Some(*qubit);
        let params = operation
            .params
            .iter()
            .map(|param| match param {
                ParameterValue::Fixed(value) if value.is_finite() => Some(*value),
                ParameterValue::Fixed(_) | ParameterValue::Param(_) => None,
            })
            .collect::<Option<Vec<_>>>()?;
        matrix = gate.matrix(&params).ok()?.dot(&matrix);
    }
    Some(matrix)
}

#[cfg(test)]
#[path = "numeric_matrix_test.rs"]
mod numeric_matrix_test;
