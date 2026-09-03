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

//! VF2++ perfect initial layout.
//!
//! VF2 layout searches for a topology-perfect initial mapping: every required
//! logical interaction must land on an adjacent physical edge. The search is
//! non-induced, so the physical topology may contain extra edges that are not
//! present in the logical interaction graph. This matches the compiler use
//! case: extra device connectivity is harmless and can help later routing.

use super::vf2_engine::{Vf2Graph, Vf2SearchConfig, find_non_induced_mappings};
use super::{
    CircuitLayoutAnalysis, Interaction, LayoutDiagnostics, LayoutObjective, LayoutResult,
    LayoutScore, PhysicalLayoutGraph, analyze_circuit_for_layout,
};
use crate::circuit::Circuit;
use crate::compile::CompilerError;
use crate::device::{Device, Layout, LogicalQubit, PhysicalQubit};
use rustworkx_core::petgraph::graph::NodeIndex;
use std::collections::{BTreeMap, BTreeSet};

/// Selects which logical interaction edges must be matched by VF2.
///
/// This controls only the hard topology constraint. Coupling direction and
/// calibration quality are still handled by [`LayoutObjective`] after complete
/// topology candidates are found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vf2EdgeRequirement {
    /// Require only interactions with positive accumulated weight.
    PositiveInteractions,
    /// Require every interaction stored in the interaction graph.
    AllInteractions,
}

impl Vf2EdgeRequirement {
    /// Returns whether `interaction` is part of the VF2 hard topology graph.
    fn requires(self, interaction: &Interaction) -> bool {
        match self {
            Self::PositiveInteractions => interaction.weight > 0.0,
            Self::AllInteractions => true,
        }
    }
}

/// Configuration for [`vf2_perfect_layout`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vf2LayoutConfig {
    /// Maximum number of complete perfect candidates to score.
    ///
    /// Larger values can improve calibration-aware ranking but increase search
    /// and scoring cost. A value of zero is invalid.
    pub candidate_limit: usize,
    /// Maximum number of partial mapping extensions attempted by the search.
    ///
    /// `None` means no explicit extension limit. When the limit is reached,
    /// diagnostics record that the search was truncated.
    pub call_limit: Option<usize>,
    /// Selects which logical interaction edges are hard topology constraints.
    pub edge_requirement: Vf2EdgeRequirement,
}

impl Default for Vf2LayoutConfig {
    fn default() -> Self {
        Self {
            candidate_limit: 10,
            call_limit: None,
            edge_requirement: Vf2EdgeRequirement::PositiveInteractions,
        }
    }
}

/// Outcome of an opportunistic prepared VF2 search.
///
/// SABRE layout treats VF2 as a bounded candidate generator.  A search that
/// proves there is no perfect mapping and a search that merely exhausts its
/// call budget are deliberately distinct outcomes.
pub(crate) enum Vf2PreparedOutcome {
    Found(LayoutResult),
    NoCandidate,
    BudgetExhausted,
}

/// Searches for a perfect initial layout using non-induced VF2++ matching.
///
/// A perfect layout maps every required logical interaction onto adjacent
/// physical qubits. Physical extra edges are allowed because the logical
/// interaction graph is matched as a non-induced subgraph. Coupling direction
/// is not a hard feasibility constraint; direction mismatch is scored by
/// [`LayoutObjective`].
/// Logical qubits that do not participate in required interactions are filled
/// deterministically after the VF2 match is found.
///
/// # Errors
///
/// Returns [`CompilerError::InvalidInput`] if physical capacity is insufficient,
/// `candidate_limit` is zero, no perfect mapping is found, or scoring rejects
/// all complete candidates.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::circuit::{Circuit, Qubit};
/// use cqlib_core::compile::transform::{
///     LayoutObjective, Vf2LayoutConfig, vf2_perfect_layout,
/// };
/// use cqlib_core::device::Device;
///
/// let mut circuit = Circuit::new(3);
/// circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
/// circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
/// let device = Device::line("line-3", 3).unwrap();
///
/// let result = vf2_perfect_layout(
///     &circuit,
///     &device,
///     &LayoutObjective::topology_only(),
///     &Vf2LayoutConfig::default(),
/// )
/// .unwrap();
/// assert!(result.diagnostics.is_perfect);
/// ```
pub fn vf2_perfect_layout(
    circuit: &Circuit,
    device: &Device,
    objective: &LayoutObjective,
    config: &Vf2LayoutConfig,
) -> Result<LayoutResult, CompilerError> {
    let analysis = analyze_circuit_for_layout(circuit)?;
    let physical = PhysicalLayoutGraph::from_device(device)?;
    vf2_perfect_layout_prepared(&analysis, &physical, objective, config)
}

/// Searches for a perfect initial layout on an already-built physical graph.
///
/// This lower-level entry point is useful when a workflow has already prepared
/// circuit analysis and physical graph data for one or more layout algorithms.
pub fn vf2_perfect_layout_prepared(
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    objective: &LayoutObjective,
    config: &Vf2LayoutConfig,
) -> Result<LayoutResult, CompilerError> {
    match try_vf2_perfect_layout_prepared(analysis, physical, objective, config)? {
        Vf2PreparedOutcome::Found(result) => Ok(result),
        Vf2PreparedOutcome::NoCandidate => Err(CompilerError::InvalidInput(
            "vf2 perfect layout could not find a perfect mapping".to_string(),
        )),
        Vf2PreparedOutcome::BudgetExhausted => Err(CompilerError::InvalidInput(
            "vf2 perfect layout could not find a perfect mapping before the call limit was reached"
                .to_string(),
        )),
    }
}

/// Runs the prepared VF2 search while preserving no-solution versus budget
/// exhaustion for callers that use VF2 as an optional prepass.
pub(crate) fn try_vf2_perfect_layout_prepared(
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    objective: &LayoutObjective,
    config: &Vf2LayoutConfig,
) -> Result<Vf2PreparedOutcome, CompilerError> {
    if config.candidate_limit == 0 {
        return Err(CompilerError::InvalidInput(
            "vf2 perfect layout candidate_limit must be greater than zero".to_string(),
        ));
    }
    if analysis.logical_qubits.len() > physical.physical_qubits().len() {
        return Err(CompilerError::InvalidInput(format!(
            "vf2 perfect layout requires at least as many usable physical qubits as logical qubits; got {} logical qubits and {} usable physical qubits",
            analysis.logical_qubits.len(),
            physical.physical_qubits().len()
        )));
    }

    let activity = analysis.interactions.logical_activity();
    let required_interactions = analysis
        .interactions
        .interactions()
        .iter()
        .filter(|interaction| config.edge_requirement.requires(interaction))
        .collect::<Vec<_>>();

    if required_interactions.is_empty() {
        // Interaction-free circuits are trivially topology-perfect. We still
        // complete and score the mapping so fidelity-aware objectives can rank
        // physical qubits by readout quality.
        let mapping = complete_mapping(BTreeMap::new(), analysis, physical, objective, &activity);
        let layout = layout_from_mapping(analysis, physical, mapping, "vf2 perfect layout")?;
        let score = objective.score_layout(analysis, physical, &layout)?;
        return Ok(Vf2PreparedOutcome::Found(LayoutResult {
            layout,
            diagnostics: LayoutDiagnostics {
                is_perfect: true,
                candidates_evaluated: 1,
                used_fidelity: score.used_fidelity,
                notes: Vec::new(),
            },
            score: Some(score),
        }));
    }

    // Build a compact graph containing only logical qubits that participate in
    // hard VF2 constraints. Idle qubits are handled by complete_mapping.
    let mut logical_index = BTreeMap::new();
    for interaction in &required_interactions {
        if !logical_index.contains_key(&interaction.left) {
            logical_index.insert(interaction.left, logical_index.len());
        }
        if !logical_index.contains_key(&interaction.right) {
            logical_index.insert(interaction.right, logical_index.len());
        }
    }
    let mut logical_nodes = vec![None; logical_index.len()];
    for (logical, node_index) in &logical_index {
        logical_nodes[*node_index] = Some(*logical);
    }
    let logical_nodes = logical_nodes
        .into_iter()
        .map(|logical| logical.expect("logical node indices are contiguous"))
        .collect::<Vec<_>>();

    let mut logical_graph =
        Vf2Graph::with_capacity(logical_index.len(), required_interactions.len());
    for _ in 0..logical_index.len() {
        logical_graph.add_node(());
    }
    for interaction in &required_interactions {
        let left = NodeIndex::new(logical_index[&interaction.left]);
        let right = NodeIndex::new(logical_index[&interaction.right]);
        if left != right && logical_graph.find_edge(left, right).is_none() {
            logical_graph.add_edge(left, right, ());
        }
    }

    // The matching graph is undirected: direction is not a feasibility
    // condition for perfect layout, only a scoring term.
    let mut physical_graph = Vf2Graph::with_capacity(physical.physical_qubits().len(), 0);
    for _ in physical.physical_qubits() {
        physical_graph.add_node(());
    }
    for left in 0..physical.physical_qubits().len() {
        for right in (left + 1)..physical.physical_qubits().len() {
            if physical.is_adjacent_undirected(
                physical.physical_qubits()[left],
                physical.physical_qubits()[right],
            ) {
                physical_graph.add_edge(NodeIndex::new(left), NodeIndex::new(right), ());
            }
        }
    }

    let (mappings, search_stats) = find_non_induced_mappings(
        &logical_graph,
        &physical_graph,
        Vf2SearchConfig {
            candidate_limit: config.candidate_limit,
            call_limit: config.call_limit,
        },
    );

    let mut best: Option<(Layout, LayoutScore)> = None;
    let mut scored = 0usize;
    for mapping in mappings {
        let mut active_mapping = BTreeMap::new();
        for (logical_index, physical_index) in mapping.into_iter().enumerate() {
            active_mapping.insert(
                logical_nodes[logical_index],
                physical.physical_qubits()[physical_index],
            );
        }
        let mapping = complete_mapping(active_mapping, analysis, physical, objective, &activity);
        let layout = layout_from_mapping(analysis, physical, mapping, "vf2 perfect layout")?;
        let score = objective.score_layout(analysis, physical, &layout)?;
        scored += 1;

        if best.as_ref().is_none_or(|(best_layout, best_score)| {
            let layout_signature = analysis
                .logical_qubits
                .iter()
                .map(|logical| {
                    layout
                        .get_physical(*logical)
                        .expect("layout results map every logical qubit")
                        .id()
                })
                .collect::<Vec<_>>();
            let best_signature = analysis
                .logical_qubits
                .iter()
                .map(|logical| {
                    best_layout
                        .get_physical(*logical)
                        .expect("layout results map every logical qubit")
                        .id()
                })
                .collect::<Vec<_>>();

            score
                .total
                .total_cmp(&best_score.total)
                .then_with(|| layout_signature.cmp(&best_signature))
                .is_lt()
        }) {
            best = Some((layout, score));
        }
    }

    let Some((layout, score)) = best else {
        return Ok(if search_stats.stopped_by_call_limit {
            Vf2PreparedOutcome::BudgetExhausted
        } else {
            Vf2PreparedOutcome::NoCandidate
        });
    };

    let mut notes = Vec::new();
    if search_stats.stopped_by_call_limit {
        notes.push("vf2 search stopped after reaching call_limit".to_string());
    }

    Ok(Vf2PreparedOutcome::Found(LayoutResult {
        layout,
        diagnostics: LayoutDiagnostics {
            is_perfect: true,
            candidates_evaluated: scored,
            used_fidelity: score.used_fidelity,
            notes,
        },
        score: Some(score),
    }))
}

/// Completes a partial VF2 mapping with deterministic assignments.
///
/// VF2 only maps logical qubits that participate in hard interaction
/// constraints. This function assigns the remaining logical qubits to unused
/// physical qubits, preferring high logical activity and low readout error when
/// the objective asks for readout-aware scoring.
fn complete_mapping(
    mut mapping: BTreeMap<LogicalQubit, PhysicalQubit>,
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    objective: &LayoutObjective,
    activity: &BTreeMap<LogicalQubit, f64>,
) -> BTreeMap<LogicalQubit, PhysicalQubit> {
    let occupied = mapping.values().copied().collect::<BTreeSet<_>>();
    let mut free_physical = physical
        .physical_qubits()
        .iter()
        .copied()
        .filter(|qubit| !occupied.contains(qubit))
        .collect::<Vec<_>>();
    // Remaining physical qubits are deterministic. If readout fidelity is part
    // of the objective, prefer lower readout error before qubit ID.
    free_physical.sort_by(|left, right| {
        if objective.readout_error_weight != 0.0 {
            physical
                .readout_error(*left)
                .unwrap_or(f64::INFINITY)
                .total_cmp(&physical.readout_error(*right).unwrap_or(f64::INFINITY))
                .then_with(|| left.cmp(right))
        } else {
            left.cmp(right)
        }
    });

    // Busier logical qubits get first choice among the remaining physical
    // qubits. Interaction-free qubits all get activity 1.0 and fall back to ID.
    let mut unmapped_logical = analysis
        .logical_qubits
        .iter()
        .copied()
        .filter(|logical| !mapping.contains_key(logical))
        .collect::<Vec<_>>();
    unmapped_logical.sort_by(|left, right| {
        activity
            .get(right)
            .copied()
            .unwrap_or(1.0)
            .total_cmp(&activity.get(left).copied().unwrap_or(1.0))
            .then_with(|| left.cmp(right))
    });

    for (logical, physical) in unmapped_logical.into_iter().zip(free_physical) {
        mapping.insert(logical, physical);
    }
    mapping
}

/// Converts a complete logical-to-physical map into a validated [`Layout`].
///
/// The `algorithm` label is included in invariant-violation messages so caller
/// diagnostics identify which layout implementation produced invalid internal
/// state.
fn layout_from_mapping(
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    mapping: BTreeMap<LogicalQubit, PhysicalQubit>,
    algorithm: &str,
) -> Result<Layout, CompilerError> {
    Layout::new(
        analysis.logical_qubits.clone(),
        physical.physical_qubits().to_vec(),
        Some(mapping),
    )
    .map_err(|error| {
        CompilerError::InvariantViolation(format!(
            "{algorithm} failed to construct a valid layout: {error}"
        ))
    })
}
