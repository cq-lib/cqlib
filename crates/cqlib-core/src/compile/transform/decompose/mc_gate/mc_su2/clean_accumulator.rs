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

//! MC-SU(2) synthesis using a clean accumulator.
//!
//! For at least two controls, this construction computes the conjunction of
//! all controls into one clean accumulator, applies one controlled rotation,
//! and uncomputes the accumulator. The MCX operations use the existing
//! many-clean-ancilla V-chain.

use super::{DECOMPOSE_MC_SU2_NAME, Su2RotationAxis, utils::validate_distinct_qubits};
use crate::circuit::{ParameterValue, Qubit, operation::ValueOperation};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::mcx::decompose_mcx_n_clean;

/// Decomposes a multi-controlled single-qubit rotation using a clean
/// accumulator and clean MCX workspace.
///
/// Inputs with zero or one control emit the corresponding standard rotation
/// directly and ignore `clean_ancillas`. For at least two controls, the
/// algorithm consumes the first `controls.len() - 1` ancillary qubits. The
/// first consumed qubit is the accumulator and the rest are clean MCX
/// workspace. Every consumed ancillary qubit must enter in `|0>` and is
/// restored to `|0>`. Extra ancillary qubits are ignored.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when too few clean ancillary
/// qubits are provided, any consumed qubit is repeated, or the underlying
/// clean-ancilla MCX synthesis fails.
pub fn decompose_mc_su2_n_clean(
    axis: Su2RotationAxis,
    theta: &ParameterValue,
    controls: &[Qubit],
    target: Qubit,
    clean_ancillas: &[Qubit],
) -> Result<Vec<ValueOperation>, CompilerError> {
    let target_group = [target];
    if controls.len() <= 1 {
        validate_distinct_qubits(&[controls, &target_group])?;
        let gate = if controls.is_empty() {
            axis.rotation_gate()
        } else {
            axis.controlled_rotation_gate()
        };
        return Ok(vec![ValueOperation::from_standard(
            gate,
            controls.iter().copied().chain([target]),
            [theta.clone()],
        )]);
    }

    let required_ancillas = controls.len() - 1;
    if clean_ancillas.len() < required_ancillas {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MC_SU2_NAME,
            reason: format!(
                "clean-accumulator MC-SU(2) decomposition with {} controls requires {} clean ancillas, got {}",
                controls.len(),
                required_ancillas,
                clean_ancillas.len()
            ),
        });
    }

    let used_ancillas = &clean_ancillas[..required_ancillas];
    validate_distinct_qubits(&[controls, &target_group, used_ancillas])?;
    let accumulator = used_ancillas[0];
    let workspace = &used_ancillas[1..];
    let mcx = decompose_mcx_n_clean(controls, accumulator, workspace)?;
    let mut operations = Vec::with_capacity(2 * mcx.len() + 1);
    operations.extend(mcx.iter().cloned());
    operations.push(ValueOperation::from_standard(
        axis.controlled_rotation_gate(),
        [accumulator, target],
        [theta.clone()],
    ));
    operations.extend(mcx);

    Ok(operations)
}
