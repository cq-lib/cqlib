// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! MCX synthesis using a clean-ancilla V-chain.

use crate::circuit::{Qubit, StandardGate, operation::ValueOperation};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::validation::find_duplicate_qubit;

use super::{
    DECOMPOSE_MCX_NAME, relative_phase::emit_relative_phase_toffoli, trivial::decompose_mcx_small,
};

/// Decomposes MCX using `controls.len() - 2` clean ancillary qubits.
///
/// Each consumed ancillary qubit must enter in `|0>` and is restored to `|0>`.
/// This contract is weaker than a dirty-ancilla decomposition: the returned
/// sequence is only equivalent to MCX on the subspace where each consumed
/// ancillary qubit begins in `|0>`. Extra ancillary qubits are ignored.
///
/// The implementation is the relative-phase Toffoli V-chain described by
/// Maslov, *Advantages of using relative-phase Toffoli gates with an
/// application to multiple control Toffoli optimization*, Phys. Rev. A 93,
/// 022311 (2016), [arXiv:1508.03273](https://arxiv.org/abs/1508.03273).
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when fewer than
/// `controls.len() - 2` clean ancillary qubits are provided for an MCX with at
/// least three controls, or when any consumed qubit is repeated.
pub fn decompose_mcx_n_clean(
    controls: &[Qubit],
    target: Qubit,
    clean_ancillas: &[Qubit],
) -> Result<Vec<ValueOperation>, CompilerError> {
    if controls.len() <= 2 {
        return decompose_mcx_small(controls, target);
    }

    let required_ancillas = controls.len() - 2;
    if clean_ancillas.len() < required_ancillas {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "clean-ancilla MCX decomposition with {} controls requires {} clean ancillas, got {}",
                controls.len(),
                required_ancillas,
                clean_ancillas.len()
            ),
        });
    }

    let used_ancillas = &clean_ancillas[..required_ancillas];
    let target_group = [target];
    if let Some(qubit) = find_duplicate_qubit(&[controls, &target_group, used_ancillas]) {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
            ),
        });
    }

    let mut operations = vec![];
    emit_relative_phase_toffoli(&mut operations, controls[0], controls[1], used_ancillas[0])?;
    for i in 1..required_ancillas {
        emit_relative_phase_toffoli(
            &mut operations,
            controls[i + 1],
            used_ancillas[i - 1],
            used_ancillas[i],
        )?;
    }

    operations.push(ValueOperation::from_standard(
        StandardGate::CCX,
        [
            controls[controls.len() - 1],
            used_ancillas[required_ancillas - 1],
            target,
        ],
        [],
    ));

    for i in (1..required_ancillas).rev() {
        emit_relative_phase_toffoli(
            &mut operations,
            controls[i + 1],
            used_ancillas[i - 1],
            used_ancillas[i],
        )?;
    }
    emit_relative_phase_toffoli(&mut operations, controls[0], controls[1], used_ancillas[0])?;

    Ok(operations)
}
