// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of the License in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Shared gate-aware scoring for adjacent logical interactions.

use super::analysis::GateInteraction;
use crate::compile::physical_target::{PhysicalLayoutGraph, TwoQubitGateStatus};

/// Raw layout costs contributed by one adjacent logical interaction.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct AdjacentInteractionScore {
    pub(super) direction: f64,
    pub(super) effective_two_qubit_error: f64,
}

/// Scores gate-specific contributions on one adjacent mapped physical pair.
///
/// Missing capability metadata preserves the historical topology-direction
/// heuristic for asymmetric standard gates. Once capability metadata exists,
/// only gates that are native somewhere on the device receive direction or
/// fidelity costs; non-native gates are left for later decomposition.
pub(super) fn score_adjacent_interaction(
    contributions: &[GateInteraction],
    left_physical: usize,
    right_physical: usize,
    physical: &PhysicalLayoutGraph,
) -> AdjacentInteractionScore {
    let mut score = AdjacentInteractionScore::default();

    if !physical.has_native_two_qubit_capabilities() {
        for contribution in contributions {
            if contribution.gate.is_invariant_under_operand_swap() {
                continue;
            }
            if contribution.left_to_right_weight > 0.0
                && !physical.supports_directed_coupling_by_index(left_physical, right_physical)
            {
                score.direction += contribution.left_to_right_weight;
            }
            if contribution.right_to_left_weight > 0.0
                && !physical.supports_directed_coupling_by_index(right_physical, left_physical)
            {
                score.direction += contribution.right_to_left_weight;
            }
        }
        return score;
    }

    for contribution in contributions {
        if !physical.is_two_qubit_gate_native_anywhere(contribution.gate) {
            continue;
        }

        if contribution.gate.is_invariant_under_operand_swap() {
            let weight = contribution.left_to_right_weight + contribution.right_to_left_weight;
            let left_to_right = physical.two_qubit_gate_status_by_index(
                left_physical,
                right_physical,
                contribution.gate,
            );
            let right_to_left = physical.two_qubit_gate_status_by_index(
                right_physical,
                left_physical,
                contribution.gate,
            );
            score.effective_two_qubit_error +=
                weight * symmetric_effective_cost(left_to_right, right_to_left);
            continue;
        }

        score_directed_contribution(
            contribution.left_to_right_weight,
            left_physical,
            right_physical,
            contribution.gate,
            physical,
            &mut score,
        );
        score_directed_contribution(
            contribution.right_to_left_weight,
            right_physical,
            left_physical,
            contribution.gate,
            physical,
            &mut score,
        );
    }

    score
}

fn score_directed_contribution(
    weight: f64,
    control: usize,
    target: usize,
    gate: crate::circuit::StandardGate,
    physical: &PhysicalLayoutGraph,
    score: &mut AdjacentInteractionScore,
) {
    if weight == 0.0 {
        return;
    }

    if !physical.supports_directed_coupling_by_index(control, target) {
        score.direction += weight;
    }

    let effective_cost = match physical.two_qubit_gate_status_by_index(control, target, gate) {
        TwoQubitGateStatus::Calibrated(error) => error,
        TwoQubitGateStatus::Unsupported | TwoQubitGateStatus::Uncalibrated => 1.0,
    };
    score.effective_two_qubit_error += weight * effective_cost;
}

fn symmetric_effective_cost(
    left_to_right: TwoQubitGateStatus,
    right_to_left: TwoQubitGateStatus,
) -> f64 {
    match (left_to_right.error(), right_to_left.error()) {
        (Some(left), Some(right)) => left.min(right),
        (Some(error), None) | (None, Some(error)) => error,
        (None, None) => 1.0,
    }
}
