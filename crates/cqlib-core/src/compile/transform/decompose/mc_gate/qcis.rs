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

//! Multi-controlled QCIS single-qubit gate synthesis primitives.
//!
//! The QCIS half-rotation gates are exact SU(2) rotations:
//!
//! ```text
//! X2P = RX(π/2)           X2M = RX(-π/2)
//! Y2P = RY(π/2)           Y2M = RY(-π/2)
//! XY2P(phi) = RZ(-phi) RX(π/2) RZ(phi)
//! XY2M(phi) = RZ(-phi) RX(-π/2) RZ(phi)
//! ```
//!
//! The `RZ` gates around an XY-plane rotation are unconditional target-basis
//! changes. Only the central rotation receives the caller-provided controls.

use super::rotation::{decompose_rotation_n_clean, decompose_rotation_no_aux};
use crate::circuit::{ParameterValue, Qubit, StandardGate, operation::ValueOperation};
use crate::compile::error::CompilerError;
use std::f64::consts::PI;

const DECOMPOSE_QCIS_NAME: &str = "decompose.qcis";

/// Decomposes a multi-controlled QCIS half-rotation without ancillary qubits.
///
/// Supported gates are `X2P`, `X2M`, `Y2P`, `Y2M`, `XY2P`, and `XY2M`.
/// Inputs with no controls emit the original QCIS standard gate directly.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `gate` or `params` is
/// invalid, an input qubit is repeated, or the underlying MC-SU(2) synthesis
/// fails.
pub fn decompose_qcis_no_aux(
    gate: StandardGate,
    params: &[ParameterValue],
    controls: &[Qubit],
    target: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_qcis_with(gate, params, controls, target, |rotation, theta| {
        decompose_rotation_no_aux(rotation, theta, controls, target)
    })
}

/// Decomposes a multi-controlled QCIS half-rotation using clean ancillas.
///
/// Supported gates are `X2P`, `X2M`, `Y2P`, `Y2M`, `XY2P`, and `XY2M`.
/// Inputs with no controls emit the original QCIS standard gate directly and
/// ignore `clean_ancillas`. The ancillary-qubit contract is inherited from
/// [`decompose_rotation_n_clean`].
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `gate` or `params` is
/// invalid or the underlying clean-accumulator MC-SU(2) synthesis fails.
pub fn decompose_qcis_n_clean(
    gate: StandardGate,
    params: &[ParameterValue],
    controls: &[Qubit],
    target: Qubit,
    clean_ancillas: &[Qubit],
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_qcis_with(gate, params, controls, target, |rotation, theta| {
        decompose_rotation_n_clean(rotation, theta, controls, target, clean_ancillas)
    })
}

fn decompose_qcis_with(
    gate: StandardGate,
    params: &[ParameterValue],
    controls: &[Qubit],
    target: Qubit,
    mut decompose_rotation: impl FnMut(
        StandardGate,
        &ParameterValue,
    ) -> Result<Vec<ValueOperation>, CompilerError>,
) -> Result<Vec<ValueOperation>, CompilerError> {
    validate_params(gate, params)?;
    if controls.is_empty() {
        return Ok(vec![ValueOperation::from_standard(
            gate,
            [target],
            params.iter().cloned(),
        )]);
    }

    let (rotation, theta) = match gate {
        StandardGate::X2P => (StandardGate::RX, PI / 2.0),
        StandardGate::X2M => (StandardGate::RX, -PI / 2.0),
        StandardGate::Y2P => (StandardGate::RY, PI / 2.0),
        StandardGate::Y2M => (StandardGate::RY, -PI / 2.0),
        StandardGate::XY2P => {
            return decompose_xy_with(&params[0], PI / 2.0, target, decompose_rotation);
        }
        StandardGate::XY2M => {
            return decompose_xy_with(&params[0], -PI / 2.0, target, decompose_rotation);
        }
        _ => unreachable!("QCIS gate was validated before decomposition"),
    };

    decompose_rotation(rotation, &ParameterValue::Fixed(theta))
}

fn decompose_xy_with(
    phi: &ParameterValue,
    theta: f64,
    target: Qubit,
    mut decompose_rotation: impl FnMut(
        StandardGate,
        &ParameterValue,
    ) -> Result<Vec<ValueOperation>, CompilerError>,
) -> Result<Vec<ValueOperation>, CompilerError> {
    let mut operations = vec![ValueOperation::from_standard(
        StandardGate::RZ,
        [target],
        [phi.scaled(-1.0)],
    )];
    operations.extend(decompose_rotation(
        StandardGate::RX,
        &ParameterValue::Fixed(theta),
    )?);
    operations.push(ValueOperation::from_standard(
        StandardGate::RZ,
        [target],
        [phi.clone()],
    ));
    Ok(operations)
}

fn validate_params(gate: StandardGate, params: &[ParameterValue]) -> Result<(), CompilerError> {
    let expected = match gate {
        StandardGate::X2P | StandardGate::X2M | StandardGate::Y2P | StandardGate::Y2M => 0,
        StandardGate::XY2P | StandardGate::XY2M => 1,
        _ => {
            return Err(invalid_qcis(format!(
                "multi-controlled QCIS decomposition supports only X2P, X2M, Y2P, Y2M, XY2P, or XY2M, got {gate}"
            )));
        }
    };
    if params.len() != expected {
        return Err(invalid_qcis(format!(
            "{gate} decomposition requires {expected} parameters, got {}",
            params.len()
        )));
    }
    Ok(())
}

fn invalid_qcis(reason: impl Into<String>) -> CompilerError {
    CompilerError::TransformFailed {
        name: DECOMPOSE_QCIS_NAME,
        reason: reason.into(),
    }
}
