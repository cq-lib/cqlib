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

//! Multi-controlled SWAP synthesis primitives.

use super::mcx::{decompose_mcx_n_clean, decompose_mcx_no_aux};
use super::validation::find_duplicate_qubit;
use crate::circuit::{Qubit, StandardGate, operation::ValueOperation};
use crate::compile::error::CompilerError;

const DECOMPOSE_SWAP_NAME: &str = "decompose.swap";

/// Decomposes a multi-controlled SWAP gate without ancillary qubits.
///
/// Inputs with no controls emit the original standard `SWAP` directly.
/// Controlled inputs use the generalized Fredkin construction with three
/// exact MCX operations.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when an input qubit is repeated
/// or an underlying MCX synthesis fails.
pub fn decompose_swap_no_aux(
    controls: &[Qubit],
    first: Qubit,
    second: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    validate_swap_qubits(controls, first, second, &[])?;
    if controls.is_empty() {
        return Ok(vec![ValueOperation::from_standard(
            StandardGate::SWAP,
            [first, second],
            [],
        )]);
    }

    let mut first_controls = controls.to_vec();
    first_controls.push(first);
    let mut second_controls = controls.to_vec();
    second_controls.push(second);
    let first_mcx = decompose_mcx_no_aux(&first_controls, second)?;
    let second_mcx = decompose_mcx_no_aux(&second_controls, first)?;
    let mut operations = Vec::with_capacity(2 * first_mcx.len() + second_mcx.len());
    operations.extend(first_mcx.iter().cloned());
    operations.extend(second_mcx);
    operations.extend(first_mcx);
    Ok(operations)
}

/// Decomposes a multi-controlled SWAP gate using clean ancillas.
///
/// Inputs with zero or one control ignore `clean_ancillas`. For at least two
/// controls, the first `controls.len() - 1` ancillary qubits are consumed.
/// The first consumed qubit accumulates the conjunction of the controls and
/// the remaining qubits are clean MCX workspace. All consumed ancillas must
/// enter in `|0>` and are restored to `|0>`. Extra ancillas are ignored.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when too few clean ancillary
/// qubits are provided, a consumed qubit is repeated, or the underlying MCX
/// synthesis fails.
pub fn decompose_swap_n_clean(
    controls: &[Qubit],
    first: Qubit,
    second: Qubit,
    clean_ancillas: &[Qubit],
) -> Result<Vec<ValueOperation>, CompilerError> {
    if controls.len() <= 1 {
        validate_swap_qubits(controls, first, second, &[])?;
        return emit_small_controlled_swap(controls, first, second);
    }

    let required_ancillas = controls.len() - 1;
    if clean_ancillas.len() < required_ancillas {
        return Err(invalid_swap(format!(
            "clean-accumulator multi-controlled SWAP decomposition with {} controls requires {} clean ancillas, got {}",
            controls.len(),
            required_ancillas,
            clean_ancillas.len()
        )));
    }

    let used_ancillas = &clean_ancillas[..required_ancillas];
    validate_swap_qubits(controls, first, second, used_ancillas)?;
    let accumulator = used_ancillas[0];
    let workspace = &used_ancillas[1..];
    let mcx = decompose_mcx_n_clean(controls, accumulator, workspace)?;
    let mut operations = Vec::with_capacity(2 * mcx.len() + 3);
    operations.extend(mcx.iter().cloned());
    emit_fredkin(&mut operations, accumulator, first, second);
    operations.extend(mcx);
    Ok(operations)
}

fn emit_small_controlled_swap(
    controls: &[Qubit],
    first: Qubit,
    second: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    let mut operations = vec![];
    match controls {
        [] => operations.push(ValueOperation::from_standard(
            StandardGate::SWAP,
            [first, second],
            [],
        )),
        [control] => emit_fredkin(&mut operations, *control, first, second),
        _ => unreachable!("small controlled SWAP supports at most one control"),
    }
    Ok(operations)
}

fn emit_fredkin(operations: &mut Vec<ValueOperation>, control: Qubit, first: Qubit, second: Qubit) {
    operations.push(ValueOperation::from_standard(
        StandardGate::CCX,
        [control, first, second],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CCX,
        [control, second, first],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CCX,
        [control, first, second],
        [],
    ));
}

fn validate_swap_qubits(
    controls: &[Qubit],
    first: Qubit,
    second: Qubit,
    ancillas: &[Qubit],
) -> Result<(), CompilerError> {
    let targets = [first, second];
    if let Some(qubit) = find_duplicate_qubit(&[controls, &targets, ancillas]) {
        return Err(invalid_swap(format!(
            "multi-controlled SWAP controls, targets, and ancillas must be distinct; duplicate {qubit}"
        )));
    }
    Ok(())
}

fn invalid_swap(reason: impl Into<String>) -> CompilerError {
    CompilerError::TransformFailed {
        name: DECOMPOSE_SWAP_NAME,
        reason: reason.into(),
    }
}
