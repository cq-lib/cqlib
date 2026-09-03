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

//! Multi-controlled Pauli synthesis primitives.
//!
//! This module adapts the exact [`super::mcx`] algorithms to multi-controlled
//! Pauli operations. Callers select an algorithm explicitly and provide the
//! flattened controls and any required ancillary qubits. Resource planning and
//! device inspection belong to the future `mc_gate` decomposition planner.
//!
//! `Y` and `Z` are synthesized by conjugating an exact MCX on the target:
//!
//! ```text
//! MCY = SDG(target); MCX; S(target)
//! MCZ = H(target);   MCX; H(target)
//! ```
//!
//! Relative-phase MCX operations are intentionally not exposed through this
//! layer because they are not exact multi-controlled Pauli operations.

use super::mcx::{
    DECOMPOSE_MCX_NAME, decompose_mcx_1_clean_b95, decompose_mcx_1_clean_kg24,
    decompose_mcx_1_dirty, decompose_mcx_2_clean, decompose_mcx_2_dirty, decompose_mcx_n_clean,
    decompose_mcx_n_dirty, decompose_mcx_no_aux, decompose_mcx_small,
};
use super::validation::find_duplicate_qubit;
use crate::circuit::{Qubit, StandardGate, operation::ValueOperation};
use crate::compile::error::CompilerError;

const DECOMPOSE_PAULI_NAME: &str = "decompose.pauli";

/// Decomposes a multi-controlled Pauli operation with at most two controls.
///
/// `pauli` may be `X`, `Y`, `Z`, or a controlled standard-gate form of one of
/// those axes. `controls` must contain all flattened controls, including any
/// control inherent to `pauli`. The returned operations are exact up to global
/// phase.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `pauli` is unsupported, an
/// input qubit is repeated, or more than two controls are provided.
pub fn decompose_pauli_small(
    pauli: StandardGate,
    controls: &[Qubit],
    target: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_pauli_with(pauli, controls, target, || {
        decompose_mcx_small(controls, target)
    })
}

/// Decomposes a multi-controlled Pauli operation without ancillary qubits.
///
/// `pauli` may be `X`, `Y`, `Z`, or a controlled standard-gate form of one of
/// those axes. `controls` must contain all flattened controls, including any
/// control inherent to `pauli`. The returned operations are exact up to global
/// phase.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `pauli` is unsupported, an
/// input qubit is repeated, or the underlying MCX synthesis fails.
pub fn decompose_pauli_no_aux(
    pauli: StandardGate,
    controls: &[Qubit],
    target: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_pauli_with(pauli, controls, target, || {
        decompose_mcx_no_aux(controls, target)
    })
}

/// Decomposes a multi-controlled Pauli operation using many clean ancillas.
///
/// For at least three controls, the algorithm consumes `controls.len() - 2`
/// ancillas. Each consumed ancilla must enter in `|0>` and is restored to
/// `|0>`. Extra ancillas are ignored.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `pauli` is unsupported or
/// the underlying clean-ancilla MCX synthesis fails.
pub fn decompose_pauli_n_clean(
    pauli: StandardGate,
    controls: &[Qubit],
    target: Qubit,
    clean_ancillas: &[Qubit],
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_pauli_with(pauli, controls, target, || {
        decompose_mcx_n_clean(controls, target, clean_ancillas)
    })
}

/// Decomposes a multi-controlled Pauli operation using many dirty ancillas.
///
/// For at least three controls, the algorithm consumes `controls.len() - 2`
/// borrowed ancillas. Each consumed ancilla may enter in an unknown state and
/// is restored exactly. Extra ancillas are ignored.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `pauli` is unsupported or
/// the underlying dirty-ancilla MCX synthesis fails.
pub fn decompose_pauli_n_dirty(
    pauli: StandardGate,
    controls: &[Qubit],
    target: Qubit,
    dirty_ancillas: &[Qubit],
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_pauli_with(pauli, controls, target, || {
        decompose_mcx_n_dirty(controls, target, dirty_ancillas)
    })
}

/// Decomposes a multi-controlled Pauli operation using one clean ancilla.
///
/// For at least three controls, `clean_ancilla` must enter in `|0>` and is
/// restored to `|0>` by the exact decomposition.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `pauli` is unsupported or
/// the underlying one-clean-ancilla MCX synthesis fails.
pub fn decompose_pauli_1_clean_b95(
    pauli: StandardGate,
    controls: &[Qubit],
    target: Qubit,
    clean_ancilla: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_pauli_with(pauli, controls, target, || {
        decompose_mcx_1_clean_b95(controls, target, clean_ancilla)
    })
}

/// Decomposes a multi-controlled Pauli operation using one conditionally clean
/// ancilla.
///
/// For at least three controls, `clean_ancilla` must enter in `|0>` and is
/// restored to `|0>` by the exact decomposition.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `pauli` is unsupported or
/// the underlying conditionally-clean MCX synthesis fails.
pub fn decompose_pauli_1_clean_kg24(
    pauli: StandardGate,
    controls: &[Qubit],
    target: Qubit,
    clean_ancilla: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_pauli_with(pauli, controls, target, || {
        decompose_mcx_1_clean_kg24(controls, target, clean_ancilla)
    })
}

/// Decomposes a multi-controlled Pauli operation using one conditionally dirty
/// ancilla.
///
/// For at least three controls, `dirty_ancilla` may enter in an unknown state
/// and is restored exactly by the exact decomposition.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `pauli` is unsupported or
/// the underlying conditionally-dirty MCX synthesis fails.
pub fn decompose_pauli_1_dirty(
    pauli: StandardGate,
    controls: &[Qubit],
    target: Qubit,
    dirty_ancilla: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_pauli_with(pauli, controls, target, || {
        decompose_mcx_1_dirty(controls, target, dirty_ancilla)
    })
}

/// Decomposes a multi-controlled Pauli operation using two conditionally clean
/// ancillas.
///
/// For at least three controls, both ancillas must enter in `|0>` and are
/// restored to `|0>` by the exact decomposition.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `pauli` is unsupported or
/// the underlying conditionally-clean MCX synthesis fails.
pub fn decompose_pauli_2_clean(
    pauli: StandardGate,
    controls: &[Qubit],
    target: Qubit,
    clean_ancillas: [Qubit; 2],
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_pauli_with(pauli, controls, target, || {
        decompose_mcx_2_clean(controls, target, clean_ancillas)
    })
}

/// Decomposes a multi-controlled Pauli operation using two conditionally dirty
/// ancillas.
///
/// For at least three controls, both ancillas may enter in unknown states and
/// are restored exactly by the exact decomposition.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when `pauli` is unsupported or
/// the underlying conditionally-dirty MCX synthesis fails.
pub fn decompose_pauli_2_dirty(
    pauli: StandardGate,
    controls: &[Qubit],
    target: Qubit,
    dirty_ancillas: [Qubit; 2],
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_pauli_with(pauli, controls, target, || {
        decompose_mcx_2_dirty(controls, target, dirty_ancillas)
    })
}

fn decompose_pauli_with(
    pauli: StandardGate,
    controls: &[Qubit],
    target: Qubit,
    synthesize_mcx: impl FnOnce() -> Result<Vec<ValueOperation>, CompilerError>,
) -> Result<Vec<ValueOperation>, CompilerError> {
    let axis = match pauli {
        StandardGate::X | StandardGate::CX | StandardGate::CCX => StandardGate::X,
        StandardGate::Y | StandardGate::CY => StandardGate::Y,
        StandardGate::Z | StandardGate::CZ => StandardGate::Z,
        _ => {
            return Err(CompilerError::TransformFailed {
                name: DECOMPOSE_PAULI_NAME,
                reason: format!(
                    "multi-controlled Pauli decomposition supports only X, CX, CCX, Y, CY, Z, or CZ, got {pauli}"
                ),
            });
        }
    };

    let standard_gate = match (axis, controls.len()) {
        (StandardGate::X, 0) => Some(StandardGate::X),
        (StandardGate::X, 1) => Some(StandardGate::CX),
        (StandardGate::X, 2) => Some(StandardGate::CCX),
        (StandardGate::Y, 0) => Some(StandardGate::Y),
        (StandardGate::Y, 1) => Some(StandardGate::CY),
        (StandardGate::Z, 0) => Some(StandardGate::Z),
        (StandardGate::Z, 1) => Some(StandardGate::CZ),
        _ => None,
    };
    if let Some(standard_gate) = standard_gate {
        let target_group = [target];
        if let Some(qubit) = find_duplicate_qubit(&[controls, &target_group]) {
            return Err(CompilerError::TransformFailed {
                name: DECOMPOSE_MCX_NAME,
                reason: format!(
                    "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
                ),
            });
        }

        return Ok(vec![ValueOperation::from_standard(
            standard_gate,
            controls.iter().copied().chain([target]),
            [],
        )]);
    }

    let mcx_operations = synthesize_mcx()?;
    let (prefix, suffix) = match axis {
        StandardGate::X => return Ok(mcx_operations),
        StandardGate::Y => (StandardGate::SDG, StandardGate::S),
        StandardGate::Z => (StandardGate::H, StandardGate::H),
        _ => unreachable!("Pauli axis was normalized before MCX synthesis"),
    };

    let mut operations = Vec::with_capacity(mcx_operations.len() + 2);
    operations.push(ValueOperation::from_standard(prefix, [target], []));
    operations.extend(mcx_operations);
    operations.push(ValueOperation::from_standard(suffix, [target], []));
    Ok(operations)
}
