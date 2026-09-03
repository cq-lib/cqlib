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

//! Greedy initial layout.
//!
//! The greedy algorithm is a deterministic construction heuristic. It walks the
//! logical interaction graph from highest to lowest weight, places the most
//! important two-qubit interactions first, and then fills any idle logical
//! qubits. It does not insert SWAP operations; it only prepares an initial
//! mapping for later routing.

use super::analysis::GateInteraction;
use super::scoring::score_adjacent_interaction;
use super::{
    CircuitLayoutAnalysis, Interaction, LayoutDiagnostics, LayoutObjective, LayoutResult,
    PhysicalLayoutGraph, analyze_circuit_for_layout, is_perfect_layout,
};
use crate::circuit::Circuit;
use crate::compile::CompilerError;
use crate::device::{Device, Layout, PhysicalQubit};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Builds a deterministic greedy initial layout from weighted interactions.
///
/// The algorithm places higher-weight logical interactions first. For each
/// interaction it chooses the closest currently feasible physical placement,
/// using the shared [`LayoutObjective`] as a local tie-breaker for direction
/// and calibration data. Routing remains a later pass: this function only
/// selects an initial logical-to-physical mapping.
///
/// This is a good default candidate generator for larger devices where VF2 may
/// be too expensive or where a perfect topology embedding does not exist.
///
/// # Errors
///
/// Returns [`CompilerError::InvalidInput`] if there are fewer usable physical
/// qubits than logical qubits, no physical candidate can be selected for a
/// required placement, or if final objective scoring rejects the layout.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::circuit::{Circuit, Qubit};
/// use cqlib_core::compile::transform::{LayoutObjective, greedy_layout};
/// use cqlib_core::device::{Device, LogicalQubit, PhysicalQubit};
///
/// let device = Device::line("line-3", 3).unwrap();
/// let objective = LayoutObjective::topology_only();
///
/// let mut circuit = Circuit::new(3);
/// circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
/// circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
///
/// let result = greedy_layout(&circuit, &device, &objective).unwrap();
/// assert_eq!(
///     result.layout.get_physical(LogicalQubit::new(0)),
///     Some(PhysicalQubit::new(0)),
/// );
/// assert!(result.diagnostics.is_perfect);
/// ```
pub fn greedy_layout(
    circuit: &Circuit,
    device: &Device,
    objective: &LayoutObjective,
) -> Result<LayoutResult, CompilerError> {
    let analysis = analyze_circuit_for_layout(circuit)?;
    let physical = PhysicalLayoutGraph::from_device(device)?;
    greedy_layout_prepared(&analysis, &physical, objective)
}

/// Builds a deterministic greedy layout from an already-built physical graph.
///
/// This lower-level entry point is useful when a workflow has already prepared
/// circuit analysis and physical graph data for one or more layout algorithms.
pub fn greedy_layout_prepared(
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    objective: &LayoutObjective,
) -> Result<LayoutResult, CompilerError> {
    match greedy_layout_candidate_prepared(analysis, physical, objective)? {
        GreedyCandidateOutcome::Found(result) => Ok(result),
        GreedyCandidateOutcome::Disconnected { left, right } => Err(CompilerError::InvalidInput(
            format!("physical qubits {left} and {right} are disconnected in the usable topology"),
        )),
    }
}

/// Structured result used when greedy layout is an optional SABRE prepass.
pub(crate) enum GreedyCandidateOutcome {
    Found(LayoutResult),
    Disconnected {
        left: PhysicalQubit,
        right: PhysicalQubit,
    },
}

pub(crate) fn greedy_layout_candidate_prepared(
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    objective: &LayoutObjective,
) -> Result<GreedyCandidateOutcome, CompilerError> {
    if analysis.logical_qubits.len() > physical.physical_qubits().len() {
        return Err(CompilerError::InvalidInput(format!(
            "greedy layout requires at least as many usable physical qubits as logical qubits; got {} logical qubits and {} usable physical qubits",
            analysis.logical_qubits.len(),
            physical.physical_qubits().len()
        )));
    }

    // Dense indices let the placement loop update mapping and vacancy state
    // without repeatedly searching the logical-qubit list.
    let logical_index = analysis
        .logical_qubits
        .iter()
        .copied()
        .enumerate()
        .map(|(index, logical)| (logical, index))
        .collect::<BTreeMap<_, _>>();
    let mut activity = vec![None; analysis.logical_qubits.len()];
    for interaction in analysis.interactions.interactions() {
        for logical in [interaction.left, interaction.right] {
            let entry = &mut activity[logical_index[&logical]];
            *entry = Some(entry.unwrap_or(0.0) + interaction.weight);
        }
    }
    let pair_candidates = sorted_physical_pair_candidates(physical);
    let mut mapping = vec![None; analysis.logical_qubits.len()];
    let mut vacant = vec![true; physical.physical_qubits().len()];
    let mut candidates_evaluated = 0usize;

    // Highest-weight interactions are placed first. Stable tie-breakers keep
    // the result reproducible when several edges have equal weight.
    let mut interaction_indices = analysis
        .interactions
        .interactions()
        .iter()
        .enumerate()
        .filter(|(_, interaction)| interaction.weight > 0.0)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    interaction_indices.sort_by(|a, b| {
        let a = &analysis.interactions.interactions()[*a];
        let b = &analysis.interactions.interactions()[*b];
        b.weight
            .total_cmp(&a.weight)
            .then_with(|| a.first_seen_order.cmp(&b.first_seen_order))
            .then_with(|| a.left.cmp(&b.left))
            .then_with(|| a.right.cmp(&b.right))
    });

    for interaction_index in interaction_indices {
        let interaction = &analysis.interactions.interactions()[interaction_index];
        let scored_interaction = ScoredInteraction {
            interaction,
            gate_contributions: analysis.interactions.gate_contributions(interaction_index),
        };
        let left_logical = logical_index[&interaction.left];
        let right_logical = logical_index[&interaction.right];
        match (mapping[left_logical], mapping[right_logical]) {
            (Some(_), Some(_)) => {}
            // If one endpoint is already placed, expand from that anchor.
            (Some(left_physical), None) => {
                let (right_physical, evaluated) = choose_single_candidate(
                    scored_interaction,
                    right_logical,
                    left_physical,
                    true,
                    &vacant,
                    physical,
                    objective,
                    &activity,
                )?;
                candidates_evaluated += evaluated;
                mapping[right_logical] = Some(right_physical);
                vacant[right_physical] = false;
            }
            (None, Some(right_physical)) => {
                let (left_physical, evaluated) = choose_single_candidate(
                    scored_interaction,
                    left_logical,
                    right_physical,
                    false,
                    &vacant,
                    physical,
                    objective,
                    &activity,
                )?;
                candidates_evaluated += evaluated;
                mapping[left_logical] = Some(left_physical);
                vacant[left_physical] = false;
            }
            // If neither endpoint is placed, choose the best ordered physical
            // pair and reserve both qubits.
            (None, None) => {
                let (left_physical, right_physical, evaluated) = choose_pair_candidate(
                    scored_interaction,
                    left_logical,
                    right_logical,
                    &vacant,
                    physical,
                    objective,
                    &activity,
                    &pair_candidates,
                )?;
                candidates_evaluated += evaluated;
                mapping[left_logical] = Some(left_physical);
                mapping[right_logical] = Some(right_physical);
                vacant[left_physical] = false;
                vacant[right_physical] = false;
            }
        }
    }

    for logical_index in 0..analysis.logical_qubits.len() {
        if mapping[logical_index].is_some() {
            continue;
        }
        // Logical qubits with no positive two-qubit interactions still need a
        // deterministic physical assignment for a complete Layout.
        let (physical_qubit, evaluated) = choose_idle_candidate(
            logical_index,
            &mapping,
            &vacant,
            physical,
            objective,
            &activity,
        )?;
        candidates_evaluated += evaluated;
        mapping[logical_index] = Some(physical_qubit);
        vacant[physical_qubit] = false;
    }

    let mut layout_mapping = BTreeMap::new();
    for (logical_index, physical_index) in mapping.iter().copied().enumerate() {
        let physical_index = physical_index.ok_or_else(|| {
            CompilerError::InvariantViolation(
                "greedy layout left a logical qubit unmapped".to_string(),
            )
        })?;
        let physical_qubit = physical.physical_at(physical_index).ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "greedy layout selected physical index {physical_index} outside target topology"
            ))
        })?;
        layout_mapping.insert(analysis.logical_qubits[logical_index], physical_qubit);
    }
    let layout = Layout::new(
        analysis.logical_qubits.clone(),
        physical.physical_qubits().to_vec(),
        Some(layout_mapping),
    )
    .map_err(|error| {
        CompilerError::InvariantViolation(format!(
            "greedy layout failed to construct a valid layout: {error}"
        ))
    })?;

    if let Some((left, right)) = analysis
        .interactions
        .interactions()
        .iter()
        .filter_map(|interaction| {
            Some((
                layout.get_physical(interaction.left)?,
                layout.get_physical(interaction.right)?,
            ))
        })
        .find(|(left, right)| physical.distance(*left, *right).is_none())
    {
        return Ok(GreedyCandidateOutcome::Disconnected { left, right });
    }
    let score = objective.score_layout(analysis, physical, &layout)?;
    let diagnostics = LayoutDiagnostics {
        is_perfect: is_perfect_layout(analysis, physical, &layout),
        candidates_evaluated,
        used_fidelity: score.used_fidelity,
        notes: Vec::new(),
    };

    Ok(GreedyCandidateOutcome::Found(LayoutResult {
        layout,
        score: Some(score),
        diagnostics,
    }))
}

/// Chooses physical qubits for an interaction with both endpoints unmapped.
///
/// Connected physical pairs are considered first, ordered by distance and
/// qubit IDs. If no connected vacant pair remains, all ordered vacant pairs are
/// considered so the caller gets a deterministic failure or least-bad choice.
#[allow(clippy::too_many_arguments)]
fn choose_pair_candidate(
    interaction: ScoredInteraction<'_>,
    left_logical: usize,
    right_logical: usize,
    vacant: &[bool],
    physical: &PhysicalLayoutGraph,
    objective: &LayoutObjective,
    activity: &[Option<f64>],
    pair_candidates: &[(usize, usize)],
) -> Result<(usize, usize, usize), CompilerError> {
    let mut best: Option<(usize, usize, CandidateCost)> = None;
    let mut evaluated = 0usize;

    // Prefer connected physical pairs first. If the usable graph is fully
    // disconnected in the remaining vacant region, the fallback below still
    // returns a deterministic least-bad pair so final scoring can reject or
    // route around it consistently.
    for (left_physical, right_physical) in pair_candidates.iter().copied() {
        if !vacant[left_physical] || !vacant[right_physical] {
            continue;
        }
        evaluated += 1;
        update_best_pair(
            interaction,
            left_logical,
            right_logical,
            left_physical,
            right_physical,
            physical,
            objective,
            activity,
            &mut best,
        );
    }

    if best.is_none() {
        for left_physical in 0..vacant.len() {
            if !vacant[left_physical] {
                continue;
            }
            for (right_physical, item) in vacant.iter().enumerate() {
                if left_physical == right_physical || !item {
                    continue;
                }
                evaluated += 1;
                update_best_pair(
                    interaction,
                    left_logical,
                    right_logical,
                    left_physical,
                    right_physical,
                    physical,
                    objective,
                    activity,
                    &mut best,
                );
            }
        }
    }

    best.map(|(left, right, _)| (left, right, evaluated))
        .ok_or_else(|| {
            CompilerError::InvalidInput(
                "greedy layout could not find two vacant physical qubits for an interaction"
                    .to_string(),
            )
        })
}

/// Chooses the missing physical endpoint for a partially mapped interaction.
///
/// `anchored_physical` is the already-placed endpoint. `anchor_is_left`
/// indicates whether that endpoint corresponds to the left side of
/// `interaction`, preserving direction-sensitive scoring.
#[allow(clippy::too_many_arguments)]
fn choose_single_candidate(
    interaction: ScoredInteraction<'_>,
    candidate_logical: usize,
    anchored_physical: usize,
    anchor_is_left: bool,
    vacant: &[bool],
    physical: &PhysicalLayoutGraph,
    objective: &LayoutObjective,
    activity: &[Option<f64>],
) -> Result<(usize, usize), CompilerError> {
    let mut best: Option<(usize, CandidateCost)> = None;
    let mut evaluated = 0usize;

    // The anchored endpoint fixes one side of the logical interaction; each
    // vacant physical qubit is evaluated as the other endpoint.
    for (candidate_physical, item) in vacant.iter().enumerate() {
        if !item {
            continue;
        }
        evaluated += 1;
        let (left_physical, right_physical) = if anchor_is_left {
            (anchored_physical, candidate_physical)
        } else {
            (candidate_physical, anchored_physical)
        };
        let (left_logical, right_logical) = if anchor_is_left {
            (usize::MAX, candidate_logical)
        } else {
            (candidate_logical, usize::MAX)
        };
        let cost = CandidateCost::for_interaction(
            interaction,
            left_logical,
            right_logical,
            left_physical,
            right_physical,
            physical,
            objective,
            activity,
        );
        let candidate = (candidate_physical, cost);
        if best.as_ref().is_none_or(|best| {
            compare_cost(candidate.1, best.1)
                .then_with(|| {
                    physical.physical_qubits()[candidate.0].cmp(&physical.physical_qubits()[best.0])
                })
                .is_lt()
        }) {
            best = Some(candidate);
        }
    }

    best.map(|(physical, _)| (physical, evaluated))
        .ok_or_else(|| {
            CompilerError::InvalidInput(
                "greedy layout could not find a vacant physical qubit for an interaction"
                    .to_string(),
            )
        })
}

/// Chooses a physical qubit for a logical qubit with no placed interaction.
///
/// Idle placement prefers candidates that stay near already-used physical
/// qubits, then uses readout cost and physical ID as deterministic tie-breaks.
fn choose_idle_candidate(
    logical: usize,
    mapping: &[Option<usize>],
    vacant: &[bool],
    physical: &PhysicalLayoutGraph,
    objective: &LayoutObjective,
    activity: &[Option<f64>],
) -> Result<(usize, usize), CompilerError> {
    let active_physical = mapping.iter().flatten().copied().collect::<Vec<_>>();
    let mut best: Option<(usize, usize, u64, f64)> = None;
    let mut evaluated = 0usize;

    // Idle qubits are placed near the already-used region when possible. This
    // keeps future routing options compact without pretending that idle qubits
    // impose a hard interaction constraint.
    for (candidate_physical, item) in vacant.iter().enumerate() {
        if !item {
            continue;
        }
        evaluated += 1;
        let mut disconnected = 0usize;
        let mut distance_sum = 0u64;
        for mapped_physical in &active_physical {
            match physical.distance_by_index(candidate_physical, *mapped_physical) {
                Some(distance) => distance_sum += u64::from(distance),
                None => disconnected += 1,
            }
        }

        let readout = readout_cost(logical, candidate_physical, physical, objective, activity);
        let candidate = (candidate_physical, disconnected, distance_sum, readout);
        if best.as_ref().is_none_or(|best| {
            candidate
                .1
                .cmp(&best.1)
                .then_with(|| candidate.2.cmp(&best.2))
                .then_with(|| candidate.3.total_cmp(&best.3))
                .then_with(|| {
                    physical.physical_qubits()[candidate.0].cmp(&physical.physical_qubits()[best.0])
                })
                .is_lt()
        }) {
            best = Some(candidate);
        }
    }

    best.map(|(physical, _, _, _)| (physical, evaluated))
        .ok_or_else(|| {
            CompilerError::InvalidInput(
                "greedy layout could not find a vacant physical qubit for an idle logical qubit"
                    .to_string(),
            )
        })
}

#[derive(Debug, Clone, Copy)]
struct ScoredInteraction<'a> {
    interaction: &'a Interaction,
    gate_contributions: &'a [GateInteraction],
}

#[derive(Debug, Clone, Copy)]
struct CandidateCost {
    /// Whether the two physical candidates are disconnected in the usable graph.
    disconnected: bool,
    /// Shortest-path distance between the physical candidates.
    distance: u32,
    /// Local objective score for this placement decision.
    objective_total: f64,
}

impl CandidateCost {
    /// Computes the local greedy cost for placing one interaction.
    ///
    /// The cost is local to the current placement decision; final layout
    /// quality is still evaluated later by [`LayoutObjective::score_layout`].
    #[allow(clippy::too_many_arguments)]
    fn for_interaction(
        interaction: ScoredInteraction<'_>,
        left_logical: usize,
        right_logical: usize,
        left_physical: usize,
        right_physical: usize,
        physical: &PhysicalLayoutGraph,
        objective: &LayoutObjective,
        activity: &[Option<f64>],
    ) -> Self {
        let Some(distance) = physical.distance_by_index(left_physical, right_physical) else {
            return Self {
                disconnected: true,
                distance: u32::MAX,
                objective_total: f64::INFINITY,
            };
        };

        let adjacent = if distance == 1 {
            score_adjacent_interaction(
                interaction.gate_contributions,
                left_physical,
                right_physical,
                physical,
            )
        } else {
            Default::default()
        };
        let two_qubit_error = if distance == 1 && objective.two_qubit_error_weight != 0.0 {
            adjacent.effective_two_qubit_error
        } else {
            0.0
        };
        let readout = readout_cost(left_logical, left_physical, physical, objective, activity)
            + readout_cost(right_logical, right_physical, physical, objective, activity);

        Self {
            disconnected: false,
            distance,
            objective_total: objective.distance_weight
                * interaction.interaction.weight
                * f64::from(distance)
                + objective.direction_weight * adjacent.direction
                + objective.two_qubit_error_weight * two_qubit_error
                + objective.readout_error_weight * readout,
        }
    }
}

/// Returns the readout contribution for placing one logical qubit.
///
/// A logical index outside the activity slice is treated as activity `1.0`.
/// This is used for anchored single-endpoint scoring where the anchored side
/// should not affect the candidate endpoint's activity lookup.
fn readout_cost(
    logical: usize,
    physical_qubit: usize,
    physical: &PhysicalLayoutGraph,
    objective: &LayoutObjective,
    activity: &[Option<f64>],
) -> f64 {
    if objective.readout_error_weight == 0.0 {
        return 0.0;
    }

    physical
        .readout_error_by_index(physical_qubit)
        .map(|error| activity.get(logical).copied().flatten().unwrap_or(1.0) * error)
        .unwrap_or(0.0)
}

/// Orders two local greedy costs from better to worse.
fn compare_cost(a: CandidateCost, b: CandidateCost) -> Ordering {
    // Hard topology quality dominates local calibration tie-breaks: connected
    // beats disconnected, then shorter path, then objective score.
    a.disconnected
        .cmp(&b.disconnected)
        .then_with(|| a.distance.cmp(&b.distance))
        .then_with(|| a.objective_total.total_cmp(&b.objective_total))
}

/// Updates the current best two-endpoint placement candidate.
///
/// Ties are resolved by physical qubit IDs so the greedy result is
/// reproducible across runs.
#[allow(clippy::too_many_arguments)]
fn update_best_pair(
    interaction: ScoredInteraction<'_>,
    left_logical: usize,
    right_logical: usize,
    left_physical: usize,
    right_physical: usize,
    physical: &PhysicalLayoutGraph,
    objective: &LayoutObjective,
    activity: &[Option<f64>],
    best: &mut Option<(usize, usize, CandidateCost)>,
) {
    let cost = CandidateCost::for_interaction(
        interaction,
        left_logical,
        right_logical,
        left_physical,
        right_physical,
        physical,
        objective,
        activity,
    );
    let candidate = (left_physical, right_physical, cost);
    if best.as_ref().is_none_or(|best| {
        compare_cost(candidate.2, best.2)
            .then_with(|| {
                physical.physical_qubits()[candidate.0].cmp(&physical.physical_qubits()[best.0])
            })
            .then_with(|| {
                physical.physical_qubits()[candidate.1].cmp(&physical.physical_qubits()[best.1])
            })
            .is_lt()
    }) {
        *best = Some(candidate);
    }
}

/// Returns connected ordered physical pairs in deterministic preference order.
fn sorted_physical_pair_candidates(physical: &PhysicalLayoutGraph) -> Vec<(usize, usize)> {
    // Pre-sorting connected ordered pairs avoids repeating the same
    // distance/ID tie-break logic for every unmapped interaction.
    let mut candidates = Vec::new();
    for left in 0..physical.physical_qubits().len() {
        for right in 0..physical.physical_qubits().len() {
            if left != right && physical.distance_by_index(left, right).is_some() {
                candidates.push((left, right));
            }
        }
    }
    candidates.sort_by(|a, b| {
        physical
            .distance_by_index(a.0, a.1)
            .cmp(&physical.distance_by_index(b.0, b.1))
            .then_with(|| physical.physical_qubits()[a.0].cmp(&physical.physical_qubits()[b.0]))
            .then_with(|| physical.physical_qubits()[a.1].cmp(&physical.physical_qubits()[b.1]))
    });
    candidates
}
