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

//! Shared test helpers for MCX synthesis tests.

use crate::circuit::Qubit;
use crate::circuit::operation::ValueOperation;
use crate::compile::transform::decompose::mc_gate::mcx::relative_phase::emit_relative_phase_toffoli;
use crate::compile::transform::decompose::mc_gate::test_utils::assert_value_operations_equal;
use crate::qis::Statevector;
use num_complex::Complex64;

pub const EPSILON: f64 = 1e-10;

/// Returns a selection of computational basis states useful for
/// exhaustive-like testing of controlled operations.
pub fn selected_basis_states(total_width: usize) -> Vec<usize> {
    let mask = (1_usize << total_width) - 1;
    let alternating_low = (0..total_width)
        .filter(|index| index % 2 == 0)
        .fold(0_usize, |state, index| state | (1_usize << index));
    let mut states = vec![
        0,
        1,
        1_usize << (total_width - 1),
        mask,
        alternating_low,
        mask ^ alternating_low,
    ];
    states.sort_unstable();
    states.dedup();
    states
}

/// Returns the unique nonzero (index, amplitude) pair from a statevector
/// that is expected to be a computational basis state.
pub fn single_nonzero_statevector_output(statevector: &Statevector) -> (usize, Complex64) {
    let outputs: Vec<_> = statevector
        .data()
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, amplitude)| amplitude.norm() >= EPSILON)
        .collect();
    assert_eq!(outputs.len(), 1, "statevector has outputs {outputs:?}");
    outputs[0]
}

/// Asserts that `operations` expand to the same sequence as a
/// relative-phase CCX.
pub fn assert_rccx_expansion(
    operations: &[ValueOperation],
    first_control: Qubit,
    second_control: Qubit,
    target: Qubit,
) {
    let mut expected = vec![];
    emit_relative_phase_toffoli(&mut expected, first_control, second_control, target).unwrap();
    assert_value_operations_equal(operations, &expected);
}
