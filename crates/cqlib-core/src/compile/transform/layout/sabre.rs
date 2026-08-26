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

//! SABRE initial-layout adapter.
//!
//! This module owns layout-candidate generation, objective scoring, and
//! [`LayoutResult`] construction. The SABRE core remains in
//! [`crate::compile::sabre`] and is used here only through crate-internal
//! routing primitives.
//!
//! The dependency direction is intentional: layout may call SABRE to refine and
//! score candidate initial layouts, but the standalone SABRE routing module must
//! not depend on layout algorithms or layout result types.

use super::interaction_seed::{interaction_aware_layouts, interaction_layout_cost};
use super::{
    CircuitLayoutAnalysis, GreedyCandidateOutcome, LayoutDiagnostics, LayoutObjective,
    LayoutResult, LayoutScore, PhysicalLayoutGraph, Vf2EdgeRequirement, Vf2LayoutConfig,
    Vf2PreparedOutcome, analyze_circuit_for_layout, greedy_layout_candidate_prepared,
    is_perfect_layout, try_vf2_perfect_layout_prepared,
};
use crate::circuit::Circuit;
use crate::compile::device_planning::DevicePlanningSession;
use crate::compile::parallelism::{ParallelismThresholds, parallel_chunk_size};
use crate::compile::sabre::{
    ComponentAssignmentSearch, InteractionReachability, PreparedRouteMetadata, RankedTrial,
    RequirementReachabilityFailure, RoutingTarget, SabreConfig, SabreDag, TrialResult,
    compare_ranked_trials, interaction_reachability_for_target_with_metadata,
    movement_component_assignment, normalize_initial_layout_for_target,
    refine_layout_with_metadata, route_ranked_trial_with_metadata,
};
use crate::compile::{CompilerError, SabreRoutingFailure};
use crate::device::{Device, Layout, LogicalQubit, PhysicalQubit};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Approximate routed-DAG visits needed before candidate-level scheduling
/// amortizes Rayon dispatch. Higher tiers require proportionally more work so
/// short searches do not spread a few candidates across an oversized pool.
const LAYOUT_PARALLELISM_THRESHOLDS: ParallelismThresholds =
    ParallelismThresholds::new(2_048, 8_192, 32_768);

/// Circuit-side data prepared once for repeated SABRE layout selection.
///
/// The fields are intentionally private so the interaction analysis and the
/// dependency DAGs cannot be supplied from different circuits. A prepared
/// value can be reused with different physical targets, objectives, and SABRE
/// configurations.
#[derive(Debug, Clone)]
pub struct PreparedSabreCircuit {
    analysis: CircuitLayoutAnalysis,
    routing_dag: SabreDag,
    refinement_dag: SabreDag,
    backward_refinement_dag: SabreDag,
}

/// Device-side SABRE data prepared for one circuit's interaction signatures.
///
/// The physical graph remains the layout/scoring view. The routing target owns
/// the connectivity, feasibility model, and costs chosen by the preparation
/// function. Route metadata for the original and bidirectional refinement DAGs
/// is stored alongside it so fused candidate refinement and routing reuse the
/// same preparation.
#[derive(Debug, Clone)]
pub struct PreparedSabreTarget {
    physical: Arc<PhysicalLayoutGraph>,
    routing: Arc<RoutingTarget>,
    routing_metadata: PreparedRouteMetadata,
    refinement_metadata: PreparedRouteMetadata,
    backward_refinement_metadata: PreparedRouteMetadata,
}

impl PreparedSabreTarget {
    /// Returns the physical graph used by layout objective scoring.
    pub fn physical(&self) -> &PhysicalLayoutGraph {
        &self.physical
    }

    pub(crate) fn routing_target(&self) -> &RoutingTarget {
        &self.routing
    }
}

impl PreparedSabreCircuit {
    /// Returns the reusable circuit layout analysis.
    pub fn analysis(&self) -> &CircuitLayoutAnalysis {
        &self.analysis
    }

    /// Returns logical qubits in source-circuit order.
    pub fn logical_qubits(&self) -> &[LogicalQubit] {
        &self.analysis.logical_qubits
    }
}

/// Prepares the circuit-side analysis and dependency models used by SABRE.
///
/// Preparing once and calling [`sabre_layout_prepared`] repeatedly avoids
/// rebuilding circuit analysis and dependency DAGs for every physical target
/// or SABRE configuration.
///
/// # Errors
///
/// Returns [`CompilerError::InvalidInput`] when the circuit contains an
/// operation that must be decomposed before SABRE, and propagates circuit
/// analysis or DAG-construction failures.
pub fn prepare_sabre_circuit(circuit: &Circuit) -> Result<PreparedSabreCircuit, CompilerError> {
    let analysis = analyze_circuit_for_layout(circuit)?;
    let routing_dag = SabreDag::from_operations(circuit.operations())?;
    let refinement_dag = SabreDag::refinement_workload(circuit.operations())?;
    let backward_refinement_dag = refinement_dag.reverse_interactions();
    Ok(PreparedSabreCircuit {
        analysis,
        routing_dag,
        refinement_dag,
        backward_refinement_dag,
    })
}

/// Prepares topology-only SABRE data for a circuit.
///
/// This path considers usable physical qubits and undirected connectivity. It
/// does not construct a device-planning session or native implementation
/// catalog.
pub fn prepare_sabre_topology_target(
    prepared: &PreparedSabreCircuit,
    device: &Device,
) -> Result<PreparedSabreTarget, CompilerError> {
    let physical = Arc::new(PhysicalLayoutGraph::from_device(device)?);
    let routing = Arc::new(RoutingTarget::from_physical(&physical)?);
    prepare_sabre_topology_target_with_prepared(prepared, physical, routing)
}

pub(crate) fn prepare_sabre_topology_target_with_prepared(
    prepared: &PreparedSabreCircuit,
    physical: Arc<PhysicalLayoutGraph>,
    routing: Arc<RoutingTarget>,
) -> Result<PreparedSabreTarget, CompilerError> {
    finish_sabre_target_preparation(prepared, physical, routing)
}

/// Prepares exact device-native SABRE data for a circuit.
///
/// This path includes native operation feasibility, direction, and
/// implementation costs. Use it when the result will be compiled against the
/// exact device contract rather than a topology-and-basis target.
pub fn prepare_sabre_device_target(
    prepared: &PreparedSabreCircuit,
    device: &Device,
) -> Result<PreparedSabreTarget, CompilerError> {
    let planning_session = DevicePlanningSession::new(device);
    prepare_sabre_device_target_with_session(prepared, device, &planning_session)
}

pub(crate) fn prepare_sabre_device_target_with_session(
    prepared: &PreparedSabreCircuit,
    device: &Device,
    planning_session: &DevicePlanningSession,
) -> Result<PreparedSabreTarget, CompilerError> {
    let physical = Arc::new(PhysicalLayoutGraph::from_device(device)?);
    prepare_sabre_device_target_with_session_and_physical(
        prepared,
        device,
        physical,
        planning_session,
    )
}

pub(crate) fn prepare_sabre_device_target_with_session_and_physical(
    prepared: &PreparedSabreCircuit,
    device: &Device,
    physical: Arc<PhysicalLayoutGraph>,
    planning_session: &DevicePlanningSession,
) -> Result<PreparedSabreTarget, CompilerError> {
    let routing = Arc::new(RoutingTarget::from_device_with_session(
        device,
        &physical,
        &prepared.routing_dag,
        planning_session,
    )?);
    finish_sabre_target_preparation(prepared, physical, routing)
}

fn finish_sabre_target_preparation(
    prepared: &PreparedSabreCircuit,
    physical: Arc<PhysicalLayoutGraph>,
    routing: Arc<RoutingTarget>,
) -> Result<PreparedSabreTarget, CompilerError> {
    let routing_metadata = PreparedRouteMetadata::new(&prepared.routing_dag, &routing)?;
    let refinement_metadata = PreparedRouteMetadata::new(&prepared.refinement_dag, &routing)?;
    let backward_refinement_metadata =
        PreparedRouteMetadata::new(&prepared.backward_refinement_dag, &routing)?;
    Ok(PreparedSabreTarget {
        physical,
        routing,
        routing_metadata,
        refinement_metadata,
        backward_refinement_metadata,
    })
}

/// Selects an initial layout with fused SABRE refinement and routing search.
///
/// The search completes every configured refinement iteration before routing
/// the resulting layout. Complete routes are never created for intermediate
/// refinement states.
/// This layout-only API returns the winning route's initial layout;
/// [`crate::compile::transform::route_sabre`] consumes the same fused result
/// directly and does not route the winner a second time.
///
/// Candidate layouts include deterministic component-feasible anchors,
/// reachable greedy/VF2 and interaction-aware layouts, plus the full randomized
/// budget controlled by [`SabreConfig::layout_trials`]. Each candidate is
/// refined through SABRE forward/backward passes; complete trials are ranked by
/// predicted native two-qubit count, native two-qubit depth, native total
/// depth, and stable candidate/trial order. [`LayoutObjective`] contributes
/// candidate generation and diagnostics, but is not the final selection key.
///
/// # Errors
///
/// Returns [`CompilerError::InvalidInput`] for invalid SABRE layout
/// configuration, insufficient usable physical qubits, unreachable
/// interactions in the usable topology, or unsupported circuit operations.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::circuit::{Circuit, Qubit};
/// use cqlib_core::compile::sabre::SabreConfig;
/// use cqlib_core::compile::transform::{LayoutObjective, sabre_layout};
/// use cqlib_core::device::Device;
///
/// let mut circuit = Circuit::new(3);
/// circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
/// let device = Device::line("line-3", 3).unwrap();
///
/// let result = sabre_layout(
///     &circuit,
///     &device,
///     &LayoutObjective::topology_only(),
///     &SabreConfig::default(),
/// )
/// .unwrap();
/// assert!(result.score.is_some());
/// ```
pub fn sabre_layout(
    circuit: &Circuit,
    device: &Device,
    objective: &LayoutObjective,
    config: &SabreConfig,
) -> Result<LayoutResult, CompilerError> {
    let prepared = prepare_sabre_circuit(circuit)?;
    let target = prepare_sabre_topology_target(&prepared, device)?;
    sabre_layout_prepared(&prepared, &target, objective, config)
}

/// Selects a SABRE initial layout from prepared circuit-side data.
///
/// Reuse `prepared` across targets or configurations to avoid repeating
/// circuit analysis and dependency-DAG construction.
///
/// This API intentionally replaces the former
/// `sabre_layout_prepared(circuit, analysis, physical, objective, config)`
/// signature. Keeping circuit-derived data in [`PreparedSabreCircuit`] prevents
/// callers from combining a circuit with analysis produced from another one.
///
/// # Errors
///
/// Returns [`CompilerError::InvalidInput`] for invalid SABRE configuration,
/// insufficient or component-incompatible physical capacity, and unreachable
/// interactions. Objective-scoring and routing failures are propagated.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::circuit::{Circuit, Qubit};
/// use cqlib_core::compile::sabre::SabreConfig;
/// use cqlib_core::compile::transform::{
///     LayoutObjective, prepare_sabre_circuit, prepare_sabre_topology_target,
///     sabre_layout_prepared,
/// };
/// use cqlib_core::device::Device;
///
/// let mut circuit = Circuit::new(3);
/// circuit.cx(Qubit::new(0), Qubit::new(2))?;
/// let prepared = prepare_sabre_circuit(&circuit)?;
/// let target = prepare_sabre_topology_target(&prepared, &Device::line("line-3", 3)?)?;
///
/// let result = sabre_layout_prepared(
///     &prepared,
///     &target,
///     &LayoutObjective::topology_only(),
///     &SabreConfig::deterministic_seeded(42),
/// )?;
/// assert_eq!(result.layout.logical_qubits().count(), 3);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn sabre_layout_prepared(
    prepared: &PreparedSabreCircuit,
    prepared_target: &PreparedSabreTarget,
    objective: &LayoutObjective,
    config: &SabreConfig,
) -> Result<LayoutResult, CompilerError> {
    let selection =
        sabre_route_selection_prepared(prepared, prepared_target, objective, config, false)?;
    Ok(LayoutResult {
        layout: selection.initial_layout,
        score: Some(selection.score),
        diagnostics: selection.diagnostics,
    })
}

pub(crate) struct PreparedSabreRouteSelection {
    pub(crate) initial_layout: Layout,
    pub(crate) trial: TrialResult,
    pub(crate) selected_trial_index: usize,
    pub(crate) trials_evaluated: usize,
    pub(crate) score: LayoutScore,
    pub(crate) diagnostics: LayoutDiagnostics,
}

pub(crate) fn sabre_route_selection_prepared(
    prepared: &PreparedSabreCircuit,
    prepared_target: &PreparedSabreTarget,
    objective: &LayoutObjective,
    config: &SabreConfig,
    retain_provenance: bool,
) -> Result<PreparedSabreRouteSelection, CompilerError> {
    validate_layout_config(config)?;
    let physical = &prepared_target.physical;
    let target = &prepared_target.routing;
    let analysis = &prepared.analysis;
    let sabre = &prepared.routing_dag;
    let forwards = &prepared.refinement_dag;
    let backwards = &prepared.backward_refinement_dag;
    let logical_qubits = analysis.logical_qubits.clone();

    if logical_qubits.len() > target.physical_qubits.len() {
        return Err(CompilerError::InvalidInput(format!(
            "sabre layout requires at least as many usable physical qubits as logical qubits; got {} logical qubits and {} usable physical qubits",
            logical_qubits.len(),
            target.physical_qubits.len()
        )));
    }

    let base_seed = config.seed.unwrap_or_else(rand::random);
    let mut rng = StdRng::seed_from_u64(base_seed);
    let initial_candidates =
        initial_layout_candidates(prepared, prepared_target, objective, config, &mut rng)?;
    let candidates = initial_candidates.layouts;
    let mut candidate_notes = initial_candidates.notes;
    let generated_candidates = candidates.len();
    let refinement_iterations = if dense_interaction_path_skips_refinement(analysis, physical) {
        candidate_notes.push(
            "skipped bidirectional refinement because dense logical interactions already span a path topology"
                .to_string(),
        );
        0
    } else {
        config.refinement_iterations
    };
    let perfect_candidate = (!target.uses_native_costs())
        .then(|| {
            candidates
                .iter()
                .position(|layout| is_perfect_layout(analysis, physical, layout))
        })
        .flatten();
    let early_search = if let Some(index) = perfect_candidate {
        let mut optimal_config = config.clone();
        optimal_config.refinement_iterations = 0;
        optimal_config.routing_trials = 1;
        match evaluate_candidate(
            sabre,
            forwards,
            backwards,
            target,
            prepared_target,
            analysis,
            physical,
            objective,
            &optimal_config,
            0,
            CandidateTrial {
                index,
                layout: candidates[index].clone(),
                base_seed,
            },
            1,
        )? {
            CandidateOutcome::Success(evaluation) if evaluation.trial.swap_count() == 0 => {
                candidate_notes.push(format!(
                    "selected a zero-SWAP topology-optimal candidate without evaluating the remaining {} candidates",
                    generated_candidates.saturating_sub(1)
                ));
                Some(CandidateSearch::from_outcome(CandidateOutcome::Success(
                    evaluation,
                )))
            }
            CandidateOutcome::Success(_) | CandidateOutcome::Infeasible(_, _) => None,
        }
    } else {
        None
    };

    let (search, candidates_evaluated) = if let Some(search) = early_search {
        (search, 1)
    } else {
        let trials = candidates
            .into_iter()
            .enumerate()
            .map(|(index, layout)| CandidateTrial {
                index,
                layout,
                base_seed,
            })
            .collect::<Vec<_>>();
        let work_units = trials
            .len()
            .saturating_mul(
                refinement_iterations
                    .saturating_mul(2)
                    .saturating_add(config.routing_trials),
            )
            .saturating_mul(sabre.graph.node_count());
        let evaluate = |search: CandidateSearch, trial: CandidateTrial| {
            let outcome = evaluate_candidate(
                sabre,
                forwards,
                backwards,
                target,
                prepared_target,
                analysis,
                physical,
                objective,
                config,
                refinement_iterations,
                trial,
                config.routing_trials,
            )?;
            search.merge(CandidateSearch::from_outcome(outcome), target)
        };
        let search = if let Some(chunk_size) =
            parallel_chunk_size(trials.len(), work_units, LAYOUT_PARALLELISM_THRESHOLDS)
        {
            trials
                .into_par_iter()
                .chunks(chunk_size)
                .map(|chunk| {
                    chunk
                        .into_iter()
                        .try_fold(CandidateSearch::default(), &evaluate)
                })
                .try_reduce(CandidateSearch::default, |left, right| {
                    left.merge(right, target)
                })?
        } else {
            trials
                .into_iter()
                .try_fold(CandidateSearch::default(), evaluate)?
        };
        (search, generated_candidates)
    };

    let best = search.best.ok_or_else(|| {
        CompilerError::SabreRoutingFailed(SabreRoutingFailure::NoFeasibleLayoutCandidate {
            evaluated: candidates_evaluated,
            missing_terminal: search.missing_terminal,
            movement_unreachable: search.movement_unreachable,
            unsupported_native: search.unsupported_native,
        })
    })?;
    let swap_count = best.trial.swap_count();
    let is_perfect = is_perfect_layout(analysis, physical, &best.layout);

    let mut notes = candidate_notes;
    notes.push(format!(
        "selected SABRE refined layout with {swap_count} final-route swaps"
    ));
    notes.push(
        "winner selected by predicted native 2Q count/depth and total depth; layout score is diagnostic"
            .to_string(),
    );
    let trial = if retain_provenance {
        best.trial.finish(target)?
    } else {
        best.trial.finish_without_provenance(target)?
    };
    Ok(PreparedSabreRouteSelection {
        initial_layout: best.layout,
        trial,
        selected_trial_index: best.route_index,
        trials_evaluated: search.trials_evaluated,
        score: best.score.clone(),
        diagnostics: LayoutDiagnostics {
            is_perfect,
            candidates_evaluated,
            used_fidelity: best.score.used_fidelity,
            notes,
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn evaluate_candidate(
    sabre: &SabreDag,
    forwards: &SabreDag,
    backwards: &SabreDag,
    target: &RoutingTarget,
    prepared_target: &PreparedSabreTarget,
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    objective: &LayoutObjective,
    config: &SabreConfig,
    refinement_iterations: usize,
    trial: CandidateTrial,
    route_trials: usize,
) -> Result<CandidateOutcome<CandidateEvaluation>, CompilerError> {
    match interaction_reachability_for_target_with_metadata(
        sabre,
        target,
        &prepared_target.routing_metadata,
        &trial.layout,
    )? {
        InteractionReachability::Reachable => {}
        InteractionReachability::UnreachableUnary {
            cause: RequirementReachabilityFailure::NoExecutableTerminal,
            ..
        }
        | InteractionReachability::UnreachablePair {
            cause: RequirementReachabilityFailure::NoExecutableTerminal,
            ..
        } => {
            return Ok(CandidateOutcome::Infeasible(
                CandidateInfeasibleReason::MissingTerminal,
                0,
            ));
        }
        InteractionReachability::UnreachableUnary {
            cause: RequirementReachabilityFailure::MovementDisconnected,
            ..
        }
        | InteractionReachability::UnreachablePair {
            cause: RequirementReachabilityFailure::MovementDisconnected,
            ..
        } => {
            return Ok(CandidateOutcome::Infeasible(
                CandidateInfeasibleReason::MovementUnreachable,
                0,
            ));
        }
    }
    let initial_signature = layout_mapping_signature(&trial.layout, &analysis.logical_qubits)?;
    let mut seen_signatures = BTreeSet::from([initial_signature]);
    let mut refined = trial.layout;
    for iteration in 0..refinement_iterations {
        let forward_seed = derive_semantic_seed(
            trial.base_seed,
            1,
            trial.index,
            iteration,
            layout_signature(&refined),
        );
        let forward = match refine_layout_with_metadata(
            forwards,
            target,
            &prepared_target.refinement_metadata,
            &refined,
            &config.heuristic,
            forward_seed,
        ) {
            Ok(layout) => layout,
            Err(error) => return classify_candidate_error(error, 0),
        };
        let backward_seed = derive_semantic_seed(
            trial.base_seed,
            2,
            trial.index,
            iteration,
            layout_signature(&forward),
        );
        let next_refined = match refine_layout_with_metadata(
            backwards,
            target,
            &prepared_target.backward_refinement_metadata,
            &forward,
            &config.heuristic,
            backward_seed,
        ) {
            Ok(layout) => layout,
            Err(error) => return classify_candidate_error(error, 0),
        };
        let mapping_signature = layout_mapping_signature(&next_refined, &analysis.logical_qubits)?;
        if !seen_signatures.insert(mapping_signature) {
            break;
        }
        refined = next_refined;
    }

    let mut best = None::<(usize, RankedTrial)>;
    for route_index in 0..route_trials {
        let seed = derive_semantic_seed(
            trial.base_seed,
            3,
            trial.index,
            route_index,
            layout_signature(&refined),
        );
        let mut routed = match route_ranked_trial_with_metadata(
            sabre,
            target,
            &prepared_target.routing_metadata,
            &refined,
            &config.heuristic,
            seed,
        ) {
            Ok(routed) => routed,
            Err(error) => return classify_candidate_error(error, route_index + 1),
        };
        let replace = if let Some((best_index, current)) = best.as_mut() {
            compare_ranked_trials(
                &mut routed,
                (trial.index, route_index),
                current,
                (trial.index, *best_index),
                target,
            )?
            .is_lt()
        } else {
            true
        };
        if replace {
            best = Some((route_index, routed));
        }
    }
    let (route_index, routed) = best.expect("route_trials is greater than zero");
    let score = objective.score_layout(analysis, physical, &refined)?;
    Ok(CandidateOutcome::Success(CandidateEvaluation {
        index: trial.index,
        route_index,
        trial: routed,
        layout: refined,
        score,
        trials_evaluated: route_trials,
    }))
}

pub(crate) fn dense_interaction_path_skips_refinement(
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
) -> bool {
    let logical_count = analysis.logical_qubits.len();
    if logical_count < 4 {
        return false;
    }
    let possible_interactions = logical_count.saturating_mul(logical_count.saturating_sub(1)) / 2;
    if analysis.interactions.len() != possible_interactions {
        return false;
    }
    let (min_weight, max_weight) = analysis
        .interactions
        .interactions()
        .iter()
        .fold((f64::INFINITY, 0.0_f64), |(min, max), interaction| {
            (min.min(interaction.weight), max.max(interaction.weight))
        });
    if !min_weight.is_finite() || min_weight <= 0.0 || max_weight > min_weight * 4.0 {
        return false;
    }

    let physical_count = physical.physical_qubits().len();
    if physical_count < 2 {
        return false;
    }
    let mut degree = vec![0usize; physical_count];
    let mut edge_count = 0usize;
    for (left, right) in physical.undirected_edges_by_index() {
        degree[left] += 1;
        degree[right] += 1;
        edge_count += 1;
    }
    let is_connected =
        (1..physical_count).all(|index| physical.distance_by_index(0, index).is_some());
    is_connected
        && edge_count + 1 == physical_count
        && degree.iter().filter(|value| **value == 1).count() == 2
        && degree.iter().all(|value| matches!(*value, 1 | 2))
}

#[derive(Default)]
struct CandidateSearch {
    best: Option<CandidateEvaluation>,
    missing_terminal: usize,
    movement_unreachable: usize,
    unsupported_native: usize,
    trials_evaluated: usize,
}

impl CandidateSearch {
    fn from_outcome(outcome: CandidateOutcome<CandidateEvaluation>) -> Self {
        let mut search = Self::default();
        match outcome {
            CandidateOutcome::Success(evaluation) => {
                search.trials_evaluated = evaluation.trials_evaluated();
                search.best = Some(evaluation);
            }
            CandidateOutcome::Infeasible(
                CandidateInfeasibleReason::MissingTerminal,
                trials_evaluated,
            ) => {
                search.trials_evaluated = trials_evaluated;
                search.missing_terminal = 1;
            }
            CandidateOutcome::Infeasible(
                CandidateInfeasibleReason::MovementUnreachable,
                trials_evaluated,
            ) => {
                search.trials_evaluated = trials_evaluated;
                search.movement_unreachable = 1;
            }
            CandidateOutcome::Infeasible(
                CandidateInfeasibleReason::UnsupportedNative,
                trials_evaluated,
            ) => {
                search.trials_evaluated = trials_evaluated;
                search.unsupported_native = 1;
            }
        }
        search
    }

    fn merge(mut self, mut other: Self, target: &RoutingTarget) -> Result<Self, CompilerError> {
        self.missing_terminal = self.missing_terminal.saturating_add(other.missing_terminal);
        self.movement_unreachable = self
            .movement_unreachable
            .saturating_add(other.movement_unreachable);
        self.unsupported_native = self
            .unsupported_native
            .saturating_add(other.unsupported_native);
        self.trials_evaluated = self.trials_evaluated.saturating_add(other.trials_evaluated);
        if let Some(mut candidate) = other.best.take() {
            let replace = if let Some(current) = self.best.as_mut() {
                candidate.compare(current, target)?.is_lt()
            } else {
                true
            };
            if replace {
                self.best = Some(candidate);
            }
        }
        Ok(self)
    }
}

impl CandidateEvaluation {
    fn compare(
        &mut self,
        other: &mut Self,
        target: &RoutingTarget,
    ) -> Result<std::cmp::Ordering, CompilerError> {
        compare_ranked_trials(
            &mut self.trial,
            (self.index, self.route_index),
            &mut other.trial,
            (other.index, other.route_index),
            target,
        )
    }

    fn trials_evaluated(&self) -> usize {
        self.trials_evaluated
    }
}

struct CandidateTrial {
    /// Stable candidate order used as the final deterministic tie-breaker.
    index: usize,
    /// Candidate layout before forward/backward refinement.
    layout: Layout,
    /// Base seed used for semantic refinement and final-route seed derivation.
    base_seed: u64,
}

fn layout_mapping_signature(
    layout: &Layout,
    logical_qubits: &[LogicalQubit],
) -> Result<Vec<u32>, CompilerError> {
    logical_qubits
        .iter()
        .map(|logical| {
            layout
                .get_physical(*logical)
                .map(PhysicalQubit::id)
                .ok_or_else(|| {
                    CompilerError::InvariantViolation(format!(
                        "sabre layout does not map logical qubit {logical}"
                    ))
                })
        })
        .collect()
}

fn layout_signature(layout: &Layout) -> u64 {
    layout
        .l2p_map()
        .iter()
        .fold(0x6a09_e667_f3bc_c909, |hash, (logical, physical)| {
            splitmix64(hash ^ (u64::from(logical.id()) << 32) ^ u64::from(physical.id()))
        })
}

fn derive_semantic_seed(
    base: u64,
    domain: u64,
    candidate_index: usize,
    step_index: usize,
    layout_signature: u64,
) -> u64 {
    splitmix64(
        base ^ domain.wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ (candidate_index as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9)
            ^ (step_index as u64).wrapping_mul(0x94d0_49bb_1331_11eb)
            ^ layout_signature,
    )
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

enum CandidateOutcome<T> {
    Success(T),
    Infeasible(CandidateInfeasibleReason, usize),
}

enum CandidateInfeasibleReason {
    MissingTerminal,
    MovementUnreachable,
    UnsupportedNative,
}

fn classify_candidate_error<T>(
    error: CompilerError,
    trials_evaluated: usize,
) -> Result<CandidateOutcome<T>, CompilerError> {
    match error {
        CompilerError::SabreRoutingFailed(
            SabreRoutingFailure::NoExecutableUnaryTerminal { .. }
            | SabreRoutingFailure::NoExecutablePairTerminal { .. },
        ) => Ok(CandidateOutcome::Infeasible(
            CandidateInfeasibleReason::MissingTerminal,
            trials_evaluated,
        )),
        CompilerError::SabreRoutingFailed(
            SabreRoutingFailure::UnreachableUnaryPlacement { .. }
            | SabreRoutingFailure::UnreachablePairPlacement { .. },
        ) => Ok(CandidateOutcome::Infeasible(
            CandidateInfeasibleReason::MovementUnreachable,
            trials_evaluated,
        )),
        CompilerError::DeviceLoweringFailed(_) => Ok(CandidateOutcome::Infeasible(
            CandidateInfeasibleReason::UnsupportedNative,
            trials_evaluated,
        )),
        fatal => Err(fatal),
    }
}

struct CandidateEvaluation {
    /// Original candidate index, retained after parallel evaluation.
    index: usize,
    /// Stable route-trial index within this layout candidate.
    route_index: usize,
    /// Best complete final route observed for the refined layout.
    trial: RankedTrial,
    /// Refined initial layout.
    layout: Layout,
    /// Objective score for the refined initial layout.
    score: LayoutScore,
    /// Number of complete original-DAG routes evaluated for this candidate.
    trials_evaluated: usize,
}

/// Validates SABRE settings that are specific to layout selection.
///
/// The routing core owns generic SABRE validation. This adapter adds checks for
/// layout-only knobs that must be non-zero before candidate generation or
/// final-route scoring starts.
fn validate_layout_config(config: &SabreConfig) -> Result<(), CompilerError> {
    if config.layout_trials == 0 {
        return Err(CompilerError::InvalidInput(
            "sabre layout_trials must be greater than zero".to_string(),
        ));
    }
    if config.layout_assignment_budget == 0 {
        return Err(CompilerError::InvalidInput(
            "sabre layout_assignment_budget must be greater than zero".to_string(),
        ));
    }
    if let Some(vf2) = config.vf2_prepass {
        if vf2.candidate_limit == 0 {
            return Err(CompilerError::InvalidInput(
                "sabre vf2_prepass candidate_limit must be greater than zero".to_string(),
            ));
        }
        if vf2.call_limit == 0 {
            return Err(CompilerError::InvalidInput(
                "sabre vf2_prepass call_limit must be greater than zero".to_string(),
            ));
        }
    }
    config.validate()
}

struct InitialLayoutCandidates {
    layouts: Vec<Layout>,
    notes: Vec<String>,
}

/// Generates the candidate set refined by SABRE layout.
///
/// Candidates include deterministic movement anchors, interaction-graph
/// embeddings, opportunistic greedy/VF2 results, and seeded random physical
/// orders. Cheap graph bounds skip VF2 when exact embedding is oversized or
/// impossible. The result is deduplicated in logical-qubit order so duplicate
/// layouts from different sources are evaluated once.
fn initial_layout_candidates(
    prepared: &PreparedSabreCircuit,
    prepared_target: &PreparedSabreTarget,
    objective: &LayoutObjective,
    config: &SabreConfig,
    rng: &mut StdRng,
) -> Result<InitialLayoutCandidates, CompilerError> {
    let analysis = &prepared.analysis;
    let sabre = &prepared.routing_dag;
    let physical = &prepared_target.physical;
    let target = &prepared_target.routing;
    let mut candidates = Vec::new();
    let mut notes = Vec::new();
    let logical_qubits = &analysis.logical_qubits;
    let assignment = match movement_component_assignment(
        sabre,
        target,
        logical_qubits,
        config.layout_assignment_budget,
    )? {
        ComponentAssignmentSearch::Found(assignment) => assignment,
        ComponentAssignmentSearch::ProvenInfeasible => {
            return Err(CompilerError::SabreRoutingFailed(
                SabreRoutingFailure::MovementAssignmentInfeasible,
            ));
        }
        ComponentAssignmentSearch::BudgetExhausted { expansions } => {
            return Err(CompilerError::SabreRoutingFailed(
                SabreRoutingFailure::MovementAssignmentBudgetExhausted {
                    budget: config.layout_assignment_budget,
                    expansions,
                },
            ));
        }
    };

    // Deterministic anchors guarantee that exact movement reachability does not
    // depend on a random seed, including cross-component terminal pairs.
    candidates.push(movement_component_layout(
        logical_qubits,
        &assignment.components,
        &assignment.logical_components,
        target,
        false,
        None,
    )?);
    candidates.push(movement_component_layout(
        logical_qubits,
        &assignment.components,
        &assignment.logical_components,
        target,
        true,
        None,
    )?);

    let structured_budget = config.layout_trials.min(6);
    let structured = interaction_aware_layouts(analysis, physical, structured_budget)?;
    for candidate in structured {
        if interaction_reachability_for_target_with_metadata(
            sabre,
            target,
            &prepared_target.routing_metadata,
            &candidate,
        )? == InteractionReachability::Reachable
        {
            candidates.push(candidate);
        }
    }

    match greedy_layout_candidate_prepared(analysis, physical, objective)? {
        GreedyCandidateOutcome::Found(greedy) => {
            let greedy =
                normalize_initial_layout_for_target(logical_qubits, target, &greedy.layout)?;
            if interaction_reachability_for_target_with_metadata(
                sabre,
                target,
                &prepared_target.routing_metadata,
                &greedy,
            )?
                == InteractionReachability::Reachable
            {
                candidates.push(greedy);
            }
        }
        GreedyCandidateOutcome::Disconnected { left, right } => notes.push(format!(
            "SABRE greedy prepass candidate mapped an interaction across disconnected physical qubits {left} and {right}"
        )),
    }

    if let Some(vf2_prepass) = config.vf2_prepass
        && vf2_is_promising(analysis, physical)
    {
        let vf2_config = Vf2LayoutConfig {
            candidate_limit: vf2_prepass.candidate_limit,
            call_limit: Some(vf2_prepass.call_limit),
            edge_requirement: Vf2EdgeRequirement::PositiveInteractions,
        };
        match try_vf2_perfect_layout_prepared(analysis, physical, objective, &vf2_config)? {
            Vf2PreparedOutcome::Found(vf2) => {
                let vf2 = normalize_initial_layout_for_target(logical_qubits, target, &vf2.layout)?;
                if interaction_reachability_for_target_with_metadata(
                    sabre,
                    target,
                    &prepared_target.routing_metadata,
                    &vf2,
                )? == InteractionReachability::Reachable
                {
                    candidates.push(vf2);
                }
            }
            Vf2PreparedOutcome::NoCandidate => {
                notes.push("SABRE VF2 prepass found no perfect layout candidate".to_string());
            }
            Vf2PreparedOutcome::BudgetExhausted => notes.push(format!(
                "SABRE VF2 prepass exhausted its call budget of {}",
                vf2_prepass.call_limit
            )),
        }
    } else if config.vf2_prepass.is_some() {
        notes.push(
            "SABRE skipped VF2 because cheap graph bounds indicate an oversized or impossible exact embedding"
                .to_string(),
        );
    }

    // Randomized candidates complete the pool after deterministic and
    // interaction-aware seeds have supplied stable anchors.
    for _ in 0..config.layout_trials {
        candidates.push(movement_component_layout(
            logical_qubits,
            &assignment.components,
            &assignment.logical_components,
            target,
            false,
            Some(rng),
        )?);
    }

    let candidates = deduplicate_layouts(candidates, logical_qubits)?;
    Ok(InitialLayoutCandidates {
        layouts: prune_layout_candidates(
            candidates,
            analysis,
            physical,
            logical_qubits,
            config.layout_trials,
        )?,
        notes,
    })
}

fn vf2_is_promising(analysis: &CircuitLayoutAnalysis, physical: &PhysicalLayoutGraph) -> bool {
    let interactions = analysis
        .interactions
        .interactions()
        .iter()
        .filter(|interaction| interaction.weight > 0.0)
        .collect::<Vec<_>>();
    if interactions.is_empty() {
        return false;
    }
    let mut logical_degree = BTreeMap::<LogicalQubit, usize>::new();
    for interaction in &interactions {
        *logical_degree.entry(interaction.left).or_default() += 1;
        *logical_degree.entry(interaction.right).or_default() += 1;
    }
    let mut physical_degree = vec![0usize; physical.physical_qubits().len()];
    let physical_edges = physical
        .undirected_edges_by_index()
        .inspect(|(left, right)| {
            physical_degree[*left] += 1;
            physical_degree[*right] += 1;
        })
        .count();
    logical_degree.len() <= 40
        && interactions.len() <= physical_edges
        && logical_degree.values().copied().max().unwrap_or(0)
            <= physical_degree.into_iter().max().unwrap_or(0)
}

fn movement_component_layout(
    logical_qubits: &[LogicalQubit],
    physical_components: &[Vec<PhysicalQubit>],
    logical_components: &[usize],
    target: &RoutingTarget,
    reverse: bool,
    mut rng: Option<&mut StdRng>,
) -> Result<Layout, CompilerError> {
    let mut mapping = BTreeMap::new();
    let mut used = BTreeSet::new();
    for (physical_index, physical_component) in physical_components.iter().enumerate() {
        let mut logical = logical_qubits
            .iter()
            .enumerate()
            .filter(|(index, _)| logical_components[*index] == physical_index)
            .map(|(_, logical)| *logical)
            .collect::<Vec<_>>();
        let mut physical = physical_component.clone();
        if let Some(rng) = rng.as_deref_mut() {
            logical.shuffle(rng);
            physical.shuffle(rng);
        } else if reverse {
            physical.reverse();
        }
        for (logical, physical) in logical.into_iter().zip(physical) {
            mapping.insert(logical, physical);
            used.insert(physical);
        }
    }

    let mut idle = logical_qubits
        .iter()
        .copied()
        .filter(|logical| !mapping.contains_key(logical))
        .collect::<Vec<_>>();
    let mut remaining = target
        .physical_qubits
        .iter()
        .copied()
        .filter(|physical| !used.contains(physical))
        .collect::<Vec<_>>();
    if let Some(rng) = rng {
        idle.shuffle(rng);
        remaining.shuffle(rng);
    } else if reverse {
        remaining.reverse();
    }
    mapping.extend(idle.into_iter().zip(remaining));

    Layout::new(
        logical_qubits.to_vec(),
        target.physical_qubits.clone(),
        Some(mapping),
    )
    .map_err(|error| {
        CompilerError::InvariantViolation(format!(
            "sabre layout failed to construct an initial candidate: {error}"
        ))
    })
}

/// Removes duplicate candidate layouts while preserving first occurrence.
///
/// Equality is based on physical-qubit IDs in logical-qubit order, not on
/// object identity or candidate source.
fn deduplicate_layouts(
    candidates: Vec<Layout>,
    logical_qubits: &[LogicalQubit],
) -> Result<Vec<Layout>, CompilerError> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for layout in candidates {
        // Signatures use physical IDs in logical-qubit order so layouts built
        // from different candidate sources deduplicate consistently.
        let signature = logical_qubits
            .iter()
            .map(|logical| {
                layout
                    .get_physical(*logical)
                    .map(PhysicalQubit::id)
                    .ok_or_else(|| {
                        CompilerError::InvariantViolation(format!(
                            "sabre layout candidate does not map logical qubit {logical}"
                        ))
                    })
            })
            .collect::<Result<Vec<_>, CompilerError>>()?;
        if seen.insert(signature) {
            unique.push(layout);
        }
    }
    Ok(unique)
}

fn prune_layout_candidates(
    candidates: Vec<Layout>,
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    logical_qubits: &[LogicalQubit],
    limit: usize,
) -> Result<Vec<Layout>, CompilerError> {
    if candidates.len() <= limit {
        return Ok(candidates);
    }
    let mut scored = candidates
        .into_iter()
        .map(|layout| {
            let cost = interaction_layout_cost(analysis, physical, &layout)?;
            let mapping = logical_qubits
                .iter()
                .map(|logical| {
                    layout
                        .get_physical(*logical)
                        .map(PhysicalQubit::id)
                        .ok_or_else(|| {
                            CompilerError::InvariantViolation(format!(
                                "sabre layout candidate does not map logical qubit {logical}"
                            ))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut subset = mapping.clone();
            subset.sort_unstable();
            Ok((cost, subset, mapping, layout))
        })
        .collect::<Result<Vec<_>, CompilerError>>()?;
    scored.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
    });

    let mut subset_counts = BTreeMap::<Vec<u32>, usize>::new();
    let mut selected = Vec::with_capacity(limit);
    let mut deferred = Vec::new();
    for candidate in scored {
        let count = subset_counts.entry(candidate.1.clone()).or_default();
        if *count < 3 && selected.len() < limit {
            *count += 1;
            selected.push(candidate.3);
        } else {
            deferred.push(candidate.3);
        }
    }
    selected.extend(
        deferred
            .into_iter()
            .take(limit.saturating_sub(selected.len())),
    );
    Ok(selected)
}
