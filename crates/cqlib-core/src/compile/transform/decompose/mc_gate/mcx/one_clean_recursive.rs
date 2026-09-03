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

//! B95 MCX synthesis using one clean ancillary qubit.
//!
//! This module implements the one-clean-ancilla construction from Barenco et
//! al., *Elementary gates for quantum computation*, Phys. Rev. A 52, 3457
//! (1995), [arXiv:quant-ph/9503016](https://arxiv.org/abs/quant-ph/9503016).
//! For an MCX with at least three controls, the construction consumes one
//! ancillary qubit that must enter in `|0>` and is restored to `|0>` before the
//! returned sequence completes.
//!
//! The B95 construction splits the controls into two groups and emits four I15
//! dirty-ancilla V-chains: a relative-phase MCX into the clean ancilla, an
//! exact MCX into the target, the inverse of the first MCX, and the second
//! exact MCX again. The first and third segments cancel the relative phases
//! while the full composition implements an exact MCX.
//!
//! Ancilla allocation, algorithm selection, open-control normalization,
//! fixed small-control templates, and later `CCX` basis lowering remain the
//! responsibility of higher compiler layers. The relative-phase optimization
//! follows Item et al., *Quantum Circuits for Isometries*, Phys. Rev. A 93,
//! 032318 (2016), [arXiv:1501.06911](https://arxiv.org/abs/1501.06911).

use crate::circuit::{Qubit, operation::ValueOperation};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::validation::find_duplicate_qubit;

use super::{
    DECOMPOSE_MCX_NAME,
    dirty_v_chain::{decompose_mcx_n_dirty, decompose_relative_phase_mcx_n_dirty},
    trivial::decompose_mcx_small,
};

use super::utils::invert_parameter_free_operations;

/// Decomposes an exact MCX using one clean ancillary qubit.
///
/// For at least three controls, `clean_ancilla` must enter in `|0>` and is
/// restored to `|0>` by the returned sequence. Inputs with at most two
/// controls delegate to [`decompose_mcx_small`] and do not consume or
/// validate `clean_ancilla`.
///
/// The four-segment construction is based on Barenco et al., *Elementary
/// gates for quantum computation*, Phys. Rev. A 52, 3457 (1995),
/// [arXiv:quant-ph/9503016](https://arxiv.org/abs/quant-ph/9503016), with the
/// relative-phase I15 optimization from Item et al.,
/// [arXiv:1501.06911](https://arxiv.org/abs/1501.06911).
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when any consumed qubit is
/// repeated or when an emitted relative-phase operation cannot be inverted.
pub fn decompose_mcx_1_clean_b95(
    controls: &[Qubit],
    target: Qubit,
    clean_ancilla: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    if controls.len() <= 2 {
        return decompose_mcx_small(controls, target);
    }

    let target_group = [target];
    let clean_ancilla_group = [clean_ancilla];
    if let Some(qubit) = find_duplicate_qubit(&[controls, &target_group, &clean_ancilla_group]) {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
            ),
        });
    }

    // Each sub-MCX borrows controls from the other group as dirty ancillas.
    // The split guarantees that both I15 calls receive exactly the prefix they
    // require without allocating additional work qubits.
    let middle = (controls.len() + 2) / 2;
    let first_controls = &controls[..middle];
    let first_dirty_ancillas = &controls[middle..middle + first_controls.len() - 2];
    let mut second_controls = controls[middle..].to_vec();
    second_controls.push(clean_ancilla);
    let second_dirty_ancillas = &controls[..second_controls.len() - 2];

    let first_relative_phase_mcx =
        decompose_relative_phase_mcx_n_dirty(first_controls, clean_ancilla, first_dirty_ancillas)?;
    let second_exact_mcx = decompose_mcx_n_dirty(&second_controls, target, second_dirty_ancillas)?;
    let inverse_first_relative_phase_mcx =
        invert_parameter_free_operations(&first_relative_phase_mcx)?;

    // Computing and uncomputing the clean ancilla around the two exact target
    // toggles cancels the first MCX's relative phases and yields an exact MCX.
    let mut operations =
        Vec::with_capacity(2 * first_relative_phase_mcx.len() + 2 * second_exact_mcx.len());
    operations.extend(first_relative_phase_mcx);
    operations.extend(second_exact_mcx.iter().cloned());
    operations.extend(inverse_first_relative_phase_mcx);
    operations.extend(second_exact_mcx);
    Ok(operations)
}
