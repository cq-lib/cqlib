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

//! Multi-controlled FSIM synthesis primitives.

use super::validation::find_duplicate_qubit;
use super::{
    pauli_rotation::{decompose_pauli_rotation_n_clean, decompose_pauli_rotation_no_aux},
    phase::{decompose_phase_n_clean, decompose_phase_no_aux},
    rotation::{decompose_rotation_n_clean, decompose_rotation_no_aux},
};
use crate::circuit::{Parameter, ParameterValue, Qubit, StandardGate, operation::ValueOperation};
use crate::compile::error::CompilerError;

const DECOMPOSE_FSIM_NAME: &str = "decompose.fsim";

/// Decomposes a multi-controlled FSIM gate without ancillary qubits.
///
/// `params` must contain `theta` and `phi`. Inputs with no controls emit the
/// original standard `FSIM` directly.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when the parameter count is
/// invalid, an input qubit is repeated, or an underlying synthesis fails.
pub fn decompose_fsim_no_aux(
    params: &[ParameterValue],
    controls: &[Qubit],
    first: Qubit,
    second: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_fsim_with(
        params,
        controls,
        first,
        second,
        |rotation, theta| decompose_pauli_rotation_no_aux(rotation, theta, controls, first, second),
        |theta| decompose_phase_no_aux(StandardGate::Phase, Some(theta), controls, first),
        |theta, flattened_controls| {
            decompose_rotation_no_aux(StandardGate::RZ, theta, flattened_controls, second)
        },
    )
}

/// Decomposes a multi-controlled FSIM gate using clean ancillas.
///
/// `params` must contain `theta` and `phi`. Inputs with no controls emit the
/// original standard `FSIM` directly and ignore `clean_ancillas`. Controlled
/// inputs reuse the same clean ancillary qubits sequentially. Extra ancillas
/// beyond the consumed prefix are ignored.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when the parameter count is
/// invalid, an input qubit is repeated, or an underlying clean-accumulator
/// synthesis fails.
pub fn decompose_fsim_n_clean(
    params: &[ParameterValue],
    controls: &[Qubit],
    first: Qubit,
    second: Qubit,
    clean_ancillas: &[Qubit],
) -> Result<Vec<ValueOperation>, CompilerError> {
    let used_ancillas = &clean_ancillas[..clean_ancillas.len().min(controls.len())];
    decompose_fsim_with(
        params,
        controls,
        first,
        second,
        |rotation, theta| {
            decompose_pauli_rotation_n_clean(
                rotation,
                theta,
                controls,
                first,
                second,
                used_ancillas,
            )
        },
        |theta| {
            decompose_phase_n_clean(
                StandardGate::Phase,
                Some(theta),
                controls,
                first,
                used_ancillas,
            )
        },
        |theta, flattened_controls| {
            decompose_rotation_n_clean(
                StandardGate::RZ,
                theta,
                flattened_controls,
                second,
                used_ancillas,
            )
        },
    )
}

fn decompose_fsim_with(
    params: &[ParameterValue],
    controls: &[Qubit],
    first: Qubit,
    second: Qubit,
    mut decompose_pauli_rotation: impl FnMut(
        StandardGate,
        &ParameterValue,
    ) -> Result<Vec<ValueOperation>, CompilerError>,
    mut decompose_phase: impl FnMut(&ParameterValue) -> Result<Vec<ValueOperation>, CompilerError>,
    mut decompose_rotation: impl FnMut(
        &ParameterValue,
        &[Qubit],
    ) -> Result<Vec<ValueOperation>, CompilerError>,
) -> Result<Vec<ValueOperation>, CompilerError> {
    if params.len() != 2 {
        return Err(invalid_fsim(format!(
            "FSIM decomposition requires 2 parameters, got {}",
            params.len()
        )));
    }
    let targets = [first, second];
    if let Some(qubit) = find_duplicate_qubit(&[controls, &targets]) {
        return Err(invalid_fsim(format!(
            "multi-controlled FSIM controls and targets must be distinct; duplicate {qubit}"
        )));
    }
    if controls.is_empty() {
        return Ok(vec![ValueOperation::from_standard(
            StandardGate::FSIM,
            [first, second],
            params.iter().cloned(),
        )]);
    }

    let theta = &params[0];
    let phi = Parameter::from(&params[1]);
    let negative_half_phi = ParameterValue::from(phi.clone() * -0.5);
    let negative_phi = ParameterValue::from(phi * -1.0);
    let mut flattened_controls = controls.to_vec();
    flattened_controls.push(first);
    let mut operations = decompose_pauli_rotation(StandardGate::RXX, theta)?;
    operations.extend(decompose_pauli_rotation(StandardGate::RYY, theta)?);
    operations.extend(decompose_phase(&negative_half_phi)?);
    operations.extend(decompose_rotation(&negative_phi, &flattened_controls)?);
    Ok(operations)
}

fn invalid_fsim(reason: impl Into<String>) -> CompilerError {
    CompilerError::TransformFailed {
        name: DECOMPOSE_FSIM_NAME,
        reason: reason.into(),
    }
}
