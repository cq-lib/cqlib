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

//! Internal relative-phase helpers for compositional MCX synthesis.
//!
//! Relative-phase operations are not exact replacements for ordinary MCX.
//! They are internal building blocks for constructions whose surrounding
//! compute-uncompute structure cancels the introduced phases.

use crate::circuit::{Qubit, StandardGate, operation::ValueOperation};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::validation::find_duplicate_qubit;

use super::DECOMPOSE_MCX_NAME;

/// Appends a relative-phase Toffoli (RCCX) operation sequence.
///
/// RCCX has the same computational-basis bit-flip behavior as exact `CCX`,
/// but may introduce relative phases. It is not a general replacement for
/// `CCX`; callers must use it only within constructions that cancel those
/// phases.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when any qubit is repeated.
pub(super) fn emit_relative_phase_toffoli(
    operations: &mut Vec<ValueOperation>,
    first_control: Qubit,
    second_control: Qubit,
    target: Qubit,
) -> Result<(), CompilerError> {
    let qubits = [first_control, second_control, target];
    if let Some(qubit) = find_duplicate_qubit(&[&qubits]) {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
            ),
        });
    }

    operations.push(ValueOperation::from_standard(StandardGate::H, [target], []));
    operations.push(ValueOperation::from_standard(StandardGate::T, [target], []));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [second_control, target],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::TDG,
        [target],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [first_control, target],
        [],
    ));
    operations.push(ValueOperation::from_standard(StandardGate::T, [target], []));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [second_control, target],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::TDG,
        [target],
        [],
    ));
    operations.push(ValueOperation::from_standard(StandardGate::H, [target], []));

    Ok(())
}
