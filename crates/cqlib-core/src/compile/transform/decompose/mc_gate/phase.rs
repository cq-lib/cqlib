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

//! Multi-controlled phase synthesis primitives.
//!
//! A controlled `Phase(theta)` cannot be replaced by a controlled `RZ(theta)`
//! alone: the single-qubit identity
//!
//! ```text
//! Phase(theta) = exp(i * theta / 2) RZ(theta)
//! ```
//!
//! turns the scalar factor into an observable conditional phase. This module
//! recursively emits that phase on the controls and delegates the remaining
//! multi-controlled `RZ` to [`super::rotation`].

use super::rotation::{decompose_rotation_n_clean, decompose_rotation_no_aux};
use crate::circuit::{ParameterValue, Qubit, StandardGate, operation::ValueOperation};
use crate::compile::error::CompilerError;
use std::f64::consts::PI;

const DECOMPOSE_PHASE_NAME: &str = "decompose.phase";

/// Decomposes a multi-controlled standard phase gate without ancillary
/// qubits.
///
/// `phase` may be `S`, `SDG`, `T`, `TDG`, or `Phase`. Pass `Some(theta)` only
/// for `Phase`; the fixed standard gates require `None`. Inputs with no
/// controls emit the original standard gate directly.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `phase` or `theta` is
/// invalid, an input qubit is repeated, or the underlying MC-SU(2) synthesis
/// fails.
pub fn decompose_phase_no_aux(
    phase: StandardGate,
    theta: Option<&ParameterValue>,
    controls: &[Qubit],
    target: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_phase_with(phase, theta, controls, target, |theta, controls, target| {
        decompose_rotation_no_aux(StandardGate::RZ, theta, controls, target)
    })
}

/// Decomposes a multi-controlled standard phase gate using clean ancillas.
///
/// `phase` may be `S`, `SDG`, `T`, `TDG`, or `Phase`. Pass `Some(theta)` only
/// for `Phase`; the fixed standard gates require `None`. Inputs with no
/// controls emit the original standard gate directly. Recursive rotations
/// reuse the same ancillary qubits sequentially, so the ancillary-qubit
/// contract is inherited from the largest emitted multi-controlled `RZ`.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `phase` or `theta` is
/// invalid or the underlying clean-accumulator MC-SU(2) synthesis fails.
pub fn decompose_phase_n_clean(
    phase: StandardGate,
    theta: Option<&ParameterValue>,
    controls: &[Qubit],
    target: Qubit,
    clean_ancillas: &[Qubit],
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_phase_with(phase, theta, controls, target, |theta, controls, target| {
        decompose_rotation_n_clean(StandardGate::RZ, theta, controls, target, clean_ancillas)
    })
}

fn decompose_phase_with(
    phase: StandardGate,
    theta: Option<&ParameterValue>,
    controls: &[Qubit],
    target: Qubit,
    mut decompose_mcrz: impl FnMut(
        &ParameterValue,
        &[Qubit],
        Qubit,
    ) -> Result<Vec<ValueOperation>, CompilerError>,
) -> Result<Vec<ValueOperation>, CompilerError> {
    let theta = normalized_theta(phase, theta)?;
    if controls.is_empty() {
        let operation = match phase {
            StandardGate::Phase => {
                ValueOperation::from_standard(StandardGate::Phase, [target], [theta])
            }
            StandardGate::S | StandardGate::SDG | StandardGate::T | StandardGate::TDG => {
                ValueOperation::from_standard(phase, [target], [])
            }
            _ => unreachable!("phase gate was normalized before emission"),
        };
        return Ok(vec![operation]);
    }

    decompose_normalized_phase_with(&theta, controls, target, &mut decompose_mcrz)
}

fn decompose_normalized_phase_with(
    theta: &ParameterValue,
    controls: &[Qubit],
    target: Qubit,
    decompose_mcrz: &mut impl FnMut(
        &ParameterValue,
        &[Qubit],
        Qubit,
    ) -> Result<Vec<ValueOperation>, CompilerError>,
) -> Result<Vec<ValueOperation>, CompilerError> {
    let Some((phase_target, phase_controls)) = controls.split_last() else {
        return Ok(vec![ValueOperation::from_standard(
            StandardGate::Phase,
            [target],
            [theta.clone()],
        )]);
    };

    let half_theta = theta.scaled(0.5);
    let mut operations = decompose_normalized_phase_with(
        &half_theta,
        phase_controls,
        *phase_target,
        decompose_mcrz,
    )?;
    operations.extend(decompose_mcrz(theta, controls, target)?);
    Ok(operations)
}

fn normalized_theta(
    phase: StandardGate,
    theta: Option<&ParameterValue>,
) -> Result<ParameterValue, CompilerError> {
    match (phase, theta) {
        (StandardGate::Phase, Some(theta)) => Ok(theta.clone()),
        (StandardGate::Phase, None) => Err(invalid_phase(
            "Phase decomposition requires one theta parameter",
        )),
        (StandardGate::S, None) => Ok(ParameterValue::Fixed(PI / 2.0)),
        (StandardGate::SDG, None) => Ok(ParameterValue::Fixed(-PI / 2.0)),
        (StandardGate::T, None) => Ok(ParameterValue::Fixed(PI / 4.0)),
        (StandardGate::TDG, None) => Ok(ParameterValue::Fixed(-PI / 4.0)),
        (StandardGate::S | StandardGate::SDG | StandardGate::T | StandardGate::TDG, Some(_)) => {
            Err(invalid_phase(format!(
                "{phase} decomposition does not accept a theta parameter"
            )))
        }
        _ => Err(invalid_phase(format!(
            "multi-controlled phase decomposition supports only S, SDG, T, TDG, or Phase, got {phase}"
        ))),
    }
}

fn invalid_phase(reason: impl Into<String>) -> CompilerError {
    CompilerError::TransformFailed {
        name: DECOMPOSE_PHASE_NAME,
        reason: reason.into(),
    }
}
