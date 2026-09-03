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

//! Vale 2024 MC-SU(2) synthesis without external ancillary qubits.
//!
//! For at least two controls, the construction partitions the controls into
//! two groups. Each group controls an exact MCX on the target while the other
//! group is borrowed as dirty workspace. The underlying dirty-ancilla MCX
//! primitive restores that workspace before the next operation runs.
//! `RY` and `RZ` use the Vale sequence directly. `RX` is reduced to `RZ` by
//! Hadamard conjugation.

use super::{
    Su2RotationAxis,
    utils::{scale_parameter, validate_distinct_qubits},
};
use crate::circuit::{ParameterValue, Qubit, StandardGate, operation::ValueOperation};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::mcx::decompose_mcx_n_dirty;

/// Decomposes a multi-controlled single-qubit rotation without ancillary
/// qubits.
///
/// Inputs with zero or one control emit the corresponding standard rotation
/// directly. For at least two controls, the returned sequence borrows control
/// qubits as dirty workspace and restores them exactly. The sequence never
/// references a qubit outside `controls` and `target`.
///
/// Symbolic `theta` expressions are preserved. The Vale sequence emits
/// quarter-scaled clones of `theta` rather than evaluating them.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when any input qubit is
/// repeated or the underlying dirty-ancilla MCX synthesis fails.
pub fn decompose_mc_su2_no_aux(
    axis: Su2RotationAxis,
    theta: &ParameterValue,
    controls: &[Qubit],
    target: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    let target_group = [target];
    validate_distinct_qubits(&[controls, &target_group])?;

    match controls {
        [] => {
            return Ok(vec![ValueOperation::from_standard(
                axis.rotation_gate(),
                [target],
                [theta.clone()],
            )]);
        }
        [control] => {
            return Ok(vec![ValueOperation::from_standard(
                axis.controlled_rotation_gate(),
                [*control, target],
                [theta.clone()],
            )]);
        }
        _ => {}
    }

    let first_group_len = controls.len().div_ceil(2);
    let (first_group, second_group) = controls.split_at(first_group_len);
    let first_mcx = decompose_mcx_n_dirty(first_group, target, second_group)?;
    let second_mcx = decompose_mcx_n_dirty(second_group, target, first_group)?;
    let negative_quarter_theta = scale_parameter(theta, -0.25);
    let positive_quarter_theta = scale_parameter(theta, 0.25);
    let inner_rotation = match axis {
        Su2RotationAxis::X => StandardGate::RZ,
        Su2RotationAxis::Y | Su2RotationAxis::Z => axis.rotation_gate(),
    };
    let mut operations = Vec::with_capacity(2 * (first_mcx.len() + second_mcx.len()) + 6);

    if axis == Su2RotationAxis::X {
        operations.push(ValueOperation::from_standard(StandardGate::H, [target], []));
    }
    for _ in 0..2 {
        operations.extend(first_mcx.iter().cloned());
        operations.push(ValueOperation::from_standard(
            inner_rotation,
            [target],
            [negative_quarter_theta.clone()],
        ));
        operations.extend(second_mcx.iter().cloned());
        operations.push(ValueOperation::from_standard(
            inner_rotation,
            [target],
            [positive_quarter_theta.clone()],
        ));
    }
    if axis == Su2RotationAxis::X {
        operations.push(ValueOperation::from_standard(StandardGate::H, [target], []));
    }

    Ok(operations)
}
