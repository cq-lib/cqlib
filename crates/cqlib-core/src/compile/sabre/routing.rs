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

use super::cost::NativePlanCost;
use super::dag::{SabreControlFlow, SabreDag, SabreNode, SabreNodeKind, SabreOperation};
use super::dense_layout::DenseRoutingLayout;
use super::heuristic::{SabreConfig, SabreHeuristicConfig};
use super::layer::{Layer, RequirementPlacement};
use crate::circuit::value_instruction::storage_operation_to_value;
use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, ControlBody, ForOp, IfOp, Instruction, Operation,
    Parameter, Qubit, StandardGate, SwitchCase, SwitchOp, WhileOp,
};
use crate::compile::device_planning::{
    DeviceGateState, DevicePlanningSession, NativePlanAvailability, NativePlanCatalog,
    NativePlanSummary,
};
use crate::compile::error::DeviceLoweringFailure;
use crate::compile::knowledge::KnowledgeInstructionKey;
use crate::compile::physical_target::PhysicalLayoutGraph;
use crate::compile::{CompilerError, SabreRoutingFailure, compare_some_first_by};
use crate::device::{Device, Layout, LogicalQubit, PhysicalQubit};
use indexmap::IndexSet;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use rayon::prelude::*;
use rustworkx_core::petgraph::Direction;
use rustworkx_core::petgraph::graph::{NodeIndex, UnGraph};
use rustworkx_core::petgraph::visit::EdgeRef;
use rustworkx_core::token_swapper::token_swapper;
use smallvec::{SmallVec, smallvec};
use std::cell::{Cell, RefCell};
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::sync::{Arc, Condvar, Mutex};

const CONTROL_FLOW_EPILOGUE_TRIALS: usize = 4;
const EAGER_PAIR_STATE_BUDGET: usize = 1_000_000;
const LAZY_PAIR_CACHE_BUDGET: usize = 100_000;
const TRIAL_PAIR_CACHE_BUDGET: usize = 4_096;
const HIGH_PAIR_REUSE_FACTOR: usize = 16;
const WIDE_FRONT_FRACTION: usize = 8;

/// Routed circuit and layout metadata produced by [`sabre_route`].
#[derive(Debug, Clone)]
pub struct SabreRoutingResult {
    /// Physical circuit with inserted SWAP operations.
    pub circuit: Circuit,
    /// Initial logical-to-physical layout used by the selected trial.
    pub initial_layout: Layout,
    /// Final logical-to-physical layout after all routed operations.
    pub final_layout: Layout,
    /// Number of inserted SWAP operations, including control-flow epilogues.
    pub swap_count: usize,
    /// Diagnostics describing routing search behavior.
    pub diagnostics: SabreRoutingDiagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteOperationProvenance {
    Source(usize),
    Inserted,
}

#[derive(Debug, Clone)]
pub(crate) struct TrackedSabreRoutingResult {
    pub(crate) routing: SabreRoutingResult,
    pub(crate) provenance: Vec<RouteOperationProvenance>,
}

/// Diagnostics emitted by SABRE routing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SabreRoutingDiagnostics {
    /// Number of routing trials evaluated.
    pub trials_evaluated: usize,
    /// Zero-based index of the selected routing trial.
    pub selected_trial_index: usize,
    /// Number of times the shortest-path fallback was used.
    pub fallback_count: usize,
    /// Number of recursively routed control-flow blocks.
    pub control_flow_blocks_routed: usize,
    /// ASAP two-qubit depth of the selected routed operation stream.
    pub two_qubit_depth: usize,
    /// Total number of operations in the selected routed operation stream.
    pub operation_count: usize,
    /// Predicted native two-qubit operation count after device lowering.
    pub native_two_qubit_count: usize,
    /// ASAP two-qubit depth using the selected exact-qargs native plans.
    pub native_two_qubit_depth: usize,
    /// ASAP total depth using the selected exact-qargs native plans.
    pub native_total_depth: usize,
    /// Predicted total native operation count after device lowering.
    pub native_operation_count: usize,
    /// Dynamic `for`/`while` loops whose total execution cost is unknown.
    pub unknown_loop_count: usize,
    /// Number of distinct unary/pair requirement signatures prepared.
    pub requirement_signature_count: usize,
    /// Pair-placement lower-bound states retained eagerly by the target.
    pub eager_pair_state_count: usize,
    /// Lazy pair-state lower-bound probes made by the selected trial.
    pub lazy_pair_l1_lookup_count: usize,
    /// Selected-trial lazy pair-state probes served by its local L1 cache.
    pub lazy_pair_l1_hit_count: usize,
    /// Sum of pair-state entries retained by the selected trial's bounded L1
    /// cache and the independent L1 caches of all recursively routed bodies.
    /// This aggregate can exceed one cache's per-trial capacity.
    pub lazy_pair_l1_cached_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct TrialResult {
    pub(crate) operations: Vec<Operation>,
    pub(crate) provenance: Vec<RouteOperationProvenance>,
    pub(crate) final_layout: Layout,
    pub(crate) swap_count: usize,
    pub(crate) fallback_count: usize,
    pub(crate) control_flow_blocks_routed: usize,
    lazy_pair_l1_lookup_count: usize,
    lazy_pair_l1_hit_count: usize,
    lazy_pair_l1_cached_count: usize,
    pub(crate) quality: TrialQuality,
}

/// Compact per-trial route representation.
///
/// Topology-only trials retain ordinary operations by shared source plus mapped
/// qargs, and inserted SWAPs as physical endpoint pairs. Full operations are
/// built only when a trial participates in a depth tie or wins the search.
#[derive(Debug, Clone, Default)]
struct CompactRoutePlan {
    steps: Vec<CompactRouteStep>,
}

#[derive(Debug, Clone)]
enum CompactRouteStep {
    Owned {
        operation: Box<Operation>,
        source_order: Option<usize>,
    },
    Mapped {
        source: Arc<Operation>,
        qubits: SmallVec<[Qubit; 3]>,
        source_order: Option<usize>,
    },
    Swap([PhysicalQubit; 2]),
}

impl CompactRoutePlan {
    fn push_operation(&mut self, operation: Operation, source_order: Option<usize>) {
        self.steps.push(CompactRouteStep::Owned {
            operation: Box::new(operation),
            source_order,
        });
    }

    fn push_mapped_operation(
        &mut self,
        source: Arc<Operation>,
        qubits: SmallVec<[Qubit; 3]>,
        source_order: Option<usize>,
    ) {
        self.steps.push(CompactRouteStep::Mapped {
            source,
            qubits,
            source_order,
        });
    }

    fn push_swap(&mut self, swap: [PhysicalQubit; 2]) {
        self.steps.push(CompactRouteStep::Swap(swap));
    }

    fn last_operation(&self) -> Option<&Operation> {
        match self.steps.last() {
            Some(CompactRouteStep::Owned { operation, .. }) => Some(operation),
            Some(CompactRouteStep::Mapped { source, .. }) => Some(source),
            Some(CompactRouteStep::Swap(_)) | None => None,
        }
    }

    fn pop_last_operation(&mut self) -> Option<Operation> {
        match self.steps.last() {
            Some(CompactRouteStep::Owned { .. } | CompactRouteStep::Mapped { .. }) => {
                match self.steps.pop() {
                    Some(CompactRouteStep::Owned { operation, .. }) => Some(*operation),
                    Some(CompactRouteStep::Mapped { source, qubits, .. }) => Some(Operation {
                        instruction: source.instruction.clone(),
                        qubits,
                        params: source.params.clone(),
                        label: source.label.clone(),
                    }),
                    _ => unreachable!("the last route-plan step was checked as an operation"),
                }
            }
            Some(CompactRouteStep::Swap(_)) | None => None,
        }
    }

    fn materialize(&self, target: &RoutingTarget) -> Result<Vec<Operation>, CompilerError> {
        let mut operations = Vec::with_capacity(self.steps.len());
        for step in &self.steps {
            match step {
                CompactRouteStep::Owned { operation, .. } => operations.push((**operation).clone()),
                CompactRouteStep::Mapped { source, qubits, .. } => operations.push(Operation {
                    instruction: source.instruction.clone(),
                    qubits: qubits.clone(),
                    params: source.params.clone(),
                    label: source.label.clone(),
                }),
                CompactRouteStep::Swap(swap) => operations.push(target.swap_operation(*swap)?),
            }
        }
        Ok(operations)
    }

    fn provenance(&self) -> Vec<RouteOperationProvenance> {
        self.steps
            .iter()
            .map(|step| match step {
                CompactRouteStep::Owned { source_order, .. } => source_order.map_or(
                    RouteOperationProvenance::Inserted,
                    RouteOperationProvenance::Source,
                ),
                CompactRouteStep::Mapped { source_order, .. } => source_order.map_or(
                    RouteOperationProvenance::Inserted,
                    RouteOperationProvenance::Source,
                ),
                CompactRouteStep::Swap(_) => RouteOperationProvenance::Inserted,
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct UnscoredTrial {
    plan: CompactRoutePlan,
    pub(crate) final_layout: Layout,
    pub(crate) swap_count: usize,
    pub(crate) fallback_count: usize,
    pub(crate) control_flow_blocks_routed: usize,
    lazy_pair_l1_lookup_count: usize,
    lazy_pair_l1_hit_count: usize,
    lazy_pair_l1_cached_count: usize,
    native_cost: NativeCircuitCost,
    two_qubit_operation_count: usize,
    operation_count: usize,
}

impl UnscoredTrial {
    pub(crate) fn abstract_quality(&self) -> AbstractTrialQuality {
        AbstractTrialQuality {
            swap_count: self.swap_count,
            two_qubit_depth: 0,
            operation_count: self.operation_count,
            two_qubit_operation_count: self.two_qubit_operation_count,
        }
    }

    fn materialize_operations(
        &self,
        target: &RoutingTarget,
    ) -> Result<Vec<Operation>, CompilerError> {
        self.plan.materialize(target)
    }

    fn append_swaps(
        &mut self,
        target: &RoutingTarget,
        swaps: impl IntoIterator<Item = [PhysicalQubit; 2]>,
    ) -> Result<(), CompilerError> {
        for swap in swaps {
            self.plan.push_swap(swap);
            self.swap_count = self.swap_count.saturating_add(1);
            self.operation_count = self.operation_count.saturating_add(1);
            self.two_qubit_operation_count = self.two_qubit_operation_count.saturating_add(1);
            if target.native_cost_enabled {
                self.native_cost.append_gate(target.swap_cost(swap)?);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RankedTrial {
    trial: UnscoredTrial,
    abstract_quality: AbstractTrialQuality,
    native_two_qubit_ops: usize,
    native_total_ops: usize,
    unknown_loop_count: usize,
    native_two_qubit_depth: Option<usize>,
    native_total_depth: Option<usize>,
    materialized_operations: Option<Vec<Operation>>,
}

impl RankedTrial {
    fn from_unscored(trial: UnscoredTrial, target: &RoutingTarget) -> Result<Self, CompilerError> {
        let abstract_quality = trial.abstract_quality();
        let (native_two_qubit_ops, native_total_ops, unknown_loop_count) =
            if target.native_cost_enabled {
                let static_native = trial.native_cost.static_native.unwrap_or_default();
                (
                    static_native.native_two_qubit_ops as usize,
                    static_native.native_total_ops as usize,
                    trial.native_cost.path.unknown_loop_count,
                )
            } else {
                (
                    abstract_quality.two_qubit_operation_count,
                    abstract_quality.operation_count,
                    0,
                )
            };
        Ok(Self {
            trial,
            abstract_quality,
            native_two_qubit_ops,
            native_total_ops,
            unknown_loop_count,
            native_two_qubit_depth: None,
            native_total_depth: None,
            materialized_operations: None,
        })
    }

    fn ensure_materialized_operations(
        &mut self,
        target: &RoutingTarget,
    ) -> Result<&[Operation], CompilerError> {
        if self.materialized_operations.is_none() {
            self.materialized_operations = Some(self.trial.materialize_operations(target)?);
        }
        Ok(self
            .materialized_operations
            .as_deref()
            .expect("materialized route operations were initialized"))
    }

    fn ensure_abstract_two_qubit_depth(
        &mut self,
        target: &RoutingTarget,
    ) -> Result<usize, CompilerError> {
        if self.abstract_quality.two_qubit_depth == 0
            && self.abstract_quality.two_qubit_operation_count != 0
        {
            self.abstract_quality.two_qubit_depth =
                two_qubit_depth(self.ensure_materialized_operations(target)?);
        }
        Ok(self.abstract_quality.two_qubit_depth)
    }

    fn ensure_native_two_qubit_depth(
        &mut self,
        target: &RoutingTarget,
    ) -> Result<usize, CompilerError> {
        if let Some(depth) = self.native_two_qubit_depth {
            return Ok(depth);
        }
        let depth = if target.native_cost_enabled {
            self.ensure_materialized_operations(target)?;
            native_two_qubit_depth(
                self.materialized_operations
                    .as_deref()
                    .expect("materialized route operations were initialized"),
                target,
            )?
        } else {
            self.ensure_abstract_two_qubit_depth(target)?
        };
        self.native_two_qubit_depth = Some(depth);
        Ok(depth)
    }

    fn ensure_native_total_depth(
        &mut self,
        target: &RoutingTarget,
    ) -> Result<usize, CompilerError> {
        if let Some(depth) = self.native_total_depth {
            return Ok(depth);
        }
        let depth = if target.native_cost_enabled {
            self.ensure_materialized_operations(target)?;
            native_total_depth(
                self.materialized_operations
                    .as_deref()
                    .expect("materialized route operations were initialized"),
                target,
            )?
        } else {
            self.abstract_quality.operation_count
        };
        self.native_total_depth = Some(depth);
        Ok(depth)
    }

    pub(crate) fn swap_count(&self) -> usize {
        self.trial.swap_count
    }

    pub(crate) fn finish(self, target: &RoutingTarget) -> Result<TrialResult, CompilerError> {
        self.finish_impl(target, true)
    }

    pub(crate) fn finish_without_provenance(
        self,
        target: &RoutingTarget,
    ) -> Result<TrialResult, CompilerError> {
        self.finish_impl(target, false)
    }

    fn finish_impl(
        mut self,
        target: &RoutingTarget,
        retain_provenance: bool,
    ) -> Result<TrialResult, CompilerError> {
        let abstract_two_qubit_depth = self.ensure_abstract_two_qubit_depth(target)?;
        let native_two_qubit_depth = self.ensure_native_two_qubit_depth(target)?;
        let native_total_depth = self.ensure_native_total_depth(target)?;
        self.ensure_materialized_operations(target)?;
        let operations = self
            .materialized_operations
            .take()
            .expect("materialized route operations were initialized");
        self.abstract_quality.two_qubit_depth = abstract_two_qubit_depth;
        let quality = TrialQuality {
            abstract_quality: self.abstract_quality,
            native_two_qubit_ops: self.native_two_qubit_ops,
            native_two_qubit_depth,
            native_total_depth,
            native_total_ops: self.native_total_ops,
            unknown_loop_count: self.unknown_loop_count,
        };
        let provenance = if retain_provenance {
            self.trial.plan.provenance()
        } else {
            Vec::new()
        };
        let trial = self.trial;
        Ok(TrialResult {
            operations,
            provenance,
            final_layout: trial.final_layout,
            swap_count: trial.swap_count,
            fallback_count: trial.fallback_count,
            control_flow_blocks_routed: trial.control_flow_blocks_routed,
            lazy_pair_l1_lookup_count: trial.lazy_pair_l1_lookup_count,
            lazy_pair_l1_hit_count: trial.lazy_pair_l1_hit_count,
            lazy_pair_l1_cached_count: trial.lazy_pair_l1_cached_count,
            quality,
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct AbstractTrialQuality {
    pub(crate) swap_count: usize,
    pub(crate) two_qubit_depth: usize,
    pub(crate) operation_count: usize,
    pub(crate) two_qubit_operation_count: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct TrialQuality {
    pub(crate) abstract_quality: AbstractTrialQuality,
    pub(crate) native_two_qubit_ops: usize,
    pub(crate) native_two_qubit_depth: usize,
    pub(crate) native_total_depth: usize,
    pub(crate) native_total_ops: usize,
    pub(crate) unknown_loop_count: usize,
}

/// Routes `circuit` from `initial_layout` by inserting SWAP operations.
///
/// The returned circuit uses physical qubit identifiers as its circuit qubits.
/// Two-qubit operations in the routed circuit are adjacent in the usable
/// physical topology.  Control-flow bodies are routed recursively and restored
/// to their entry layout before leaving the block.
pub fn sabre_route(
    circuit: &Circuit,
    device: &Device,
    initial_layout: &Layout,
    config: &SabreConfig,
) -> Result<SabreRoutingResult, CompilerError> {
    sabre_route_topology_with_tracking(circuit, device, initial_layout, config, false)
        .map(|result| result.routing)
}

pub(crate) fn sabre_route_with_provenance_and_session_on_physical(
    circuit: &Circuit,
    device: &Device,
    physical: &PhysicalLayoutGraph,
    initial_layout: &Layout,
    config: &SabreConfig,
    planning_session: &DevicePlanningSession,
) -> Result<TrackedSabreRoutingResult, CompilerError> {
    config.validate()?;
    let sabre = SabreDag::from_operations(circuit.operations())?;
    let target =
        RoutingTarget::from_device_with_session(device, physical, &sabre, planning_session)?;
    sabre_route_with_prepared_target(circuit, &sabre, &target, initial_layout, config, true)
}

pub(crate) fn sabre_route_with_provenance_on_target(
    circuit: &Circuit,
    target: &RoutingTarget,
    initial_layout: &Layout,
    config: &SabreConfig,
) -> Result<TrackedSabreRoutingResult, CompilerError> {
    config.validate()?;
    let sabre = SabreDag::from_operations(circuit.operations())?;
    sabre_route_with_prepared_target(circuit, &sabre, target, initial_layout, config, true)
}

fn sabre_route_topology_with_tracking(
    circuit: &Circuit,
    device: &Device,
    initial_layout: &Layout,
    config: &SabreConfig,
    retain_provenance: bool,
) -> Result<TrackedSabreRoutingResult, CompilerError> {
    config.validate()?;
    // Build a dense, reusable view of the physical topology once. The routing
    // loop indexes into this structure heavily for adjacency, distance, and
    // deterministic candidate ordering.
    let physical = PhysicalLayoutGraph::from_device(device)?;
    let sabre = SabreDag::from_operations(circuit.operations())?;
    let target = RoutingTarget::from_physical(&physical)?;
    sabre_route_with_prepared_target(
        circuit,
        &sabre,
        &target,
        initial_layout,
        config,
        retain_provenance,
    )
}

fn sabre_route_with_prepared_target(
    circuit: &Circuit,
    sabre: &SabreDag,
    target: &RoutingTarget,
    initial_layout: &Layout,
    config: &SabreConfig,
    retain_provenance: bool,
) -> Result<TrackedSabreRoutingResult, CompilerError> {
    let metadata = PreparedRouteMetadata::new(sabre, target)?;
    sabre_route_prepared_impl(
        circuit,
        sabre,
        target,
        &metadata,
        initial_layout,
        config,
        retain_provenance,
    )
}

fn sabre_route_prepared_impl(
    circuit: &Circuit,
    sabre: &SabreDag,
    target: &RoutingTarget,
    metadata: &PreparedRouteMetadata,
    initial_layout: &Layout,
    config: &SabreConfig,
    retain_provenance: bool,
) -> Result<TrackedSabreRoutingResult, CompilerError> {
    let logical_qubits = circuit
        .qubits()
        .into_iter()
        .map(LogicalQubit::from_qubit)
        .collect::<Vec<_>>();
    let initial_layout =
        normalize_initial_layout_for_target(&logical_qubits, target, initial_layout)?;
    validate_reachable_interactions_for_target_with_metadata(
        sabre,
        target,
        metadata,
        &initial_layout,
    )?;
    // Trials share the normalized layout and DAG but use independent seeds for
    // tie-breaking. Selection stays deterministic for a configured seed because
    // result comparison falls back to the trial index.
    let (best_index, best) = trial_seeds(config.seed, config.routing_trials)
        .into_par_iter()
        .enumerate()
        .try_fold(
            || None,
            |best, (index, seed)| {
                let trial = route_ranked_trial_with_metadata(
                    sabre,
                    target,
                    metadata,
                    &initial_layout,
                    &config.heuristic,
                    seed,
                )?;
                select_ranked_trial(best, Some((index, trial)), target)
            },
        )
        .try_reduce(
            || None,
            |left, right| select_ranked_trial(left, right, target),
        )?
        .expect("routing_trials is validated to be non-zero");

    let best = if retain_provenance {
        best.finish(target)?
    } else {
        best.finish_without_provenance(target)?
    };
    finish_sabre_route(
        circuit,
        target,
        initial_layout,
        best,
        best_index,
        config.routing_trials,
    )
}

fn select_ranked_trial(
    left: Option<(usize, RankedTrial)>,
    right: Option<(usize, RankedTrial)>,
    target: &RoutingTarget,
) -> Result<Option<(usize, RankedTrial)>, CompilerError> {
    match (left, right) {
        (Some(mut left), Some(mut right)) => {
            if compare_ranked_trials(&mut left.1, (0, left.0), &mut right.1, (0, right.0), target)?
                .is_le()
            {
                Ok(Some(left))
            } else {
                Ok(Some(right))
            }
        }
        (Some(trial), None) | (None, Some(trial)) => Ok(Some(trial)),
        (None, None) => Ok(None),
    }
}

pub(crate) fn route_ranked_trial_with_metadata(
    sabre: &SabreDag,
    target: &RoutingTarget,
    metadata: &PreparedRouteMetadata,
    initial_layout: &Layout,
    heuristic: &SabreHeuristicConfig,
    seed: u64,
) -> Result<RankedTrial, CompilerError> {
    let trial = route_unscored_trial_with_metadata(
        sabre,
        target,
        metadata,
        initial_layout,
        heuristic,
        seed,
    )?;
    RankedTrial::from_unscored(trial, target)
}

pub(crate) fn finish_sabre_route(
    circuit: &Circuit,
    target: &RoutingTarget,
    initial_layout: Layout,
    mut best: TrialResult,
    best_index: usize,
    trials_evaluated: usize,
) -> Result<TrackedSabreRoutingResult, CompilerError> {
    // Routing rewrites operation qubits but keeps symbolic parameters by index.
    // Rebuild the routed circuit's parameter table in first-use order, then
    // remap nested control-flow bodies to the new table.
    let mut parameter_order = IndexSet::<Parameter>::new();
    for operation in &best.operations {
        for param in &operation.params {
            if let CircuitParam::Index(index) = param {
                let parameter = circuit
                    .parameters()
                    .get_index(*index as usize)
                    .cloned()
                    .ok_or(crate::circuit::CircuitError::InvalidParameterIndex(*index))?;
                parameter_order.insert(parameter);
            }
        }
    }
    for parameter in circuit.parameters() {
        parameter_order.insert(parameter.clone());
    }
    let parameter_indices = circuit
        .parameters()
        .iter()
        .map(|parameter| {
            parameter_order
                .get_index_of(parameter)
                .expect("source parameters are included in routed parameter order")
                as u32
        })
        .collect::<Vec<_>>();
    let routed_operations = std::mem::take(&mut best.operations)
        .into_iter()
        .map(|operation| {
            let operation = remap_parameter_indices(operation, &parameter_indices)?;
            storage_operation_to_value(operation, &|param| match param {
                CircuitParam::Fixed(value) => Ok((*value).into()),
                CircuitParam::Index(index) => parameter_order
                    .get_index(*index as usize)
                    .cloned()
                    .map(Into::into)
                    .ok_or(crate::circuit::CircuitError::InvalidParameterIndex(*index)),
            })
            .map_err(CompilerError::Circuit)
        })
        .collect::<Result<Vec<_>, CompilerError>>()?;
    let mut routed = Circuit::from_operations(
        target
            .physical_qubits
            .iter()
            .copied()
            .map(PhysicalQubit::qubit)
            .collect(),
        routed_operations,
        Some(circuit.classical_vars().to_vec()),
        Some(circuit.classical_values().to_vec()),
    )?;
    for parameter in parameter_order {
        routed.add_parameter(parameter);
    }
    routed.set_global_phase(circuit.global_phase());
    Ok(TrackedSabreRoutingResult {
        routing: SabreRoutingResult {
            circuit: routed,
            initial_layout,
            final_layout: best.final_layout,
            swap_count: best.swap_count,
            diagnostics: SabreRoutingDiagnostics {
                trials_evaluated,
                selected_trial_index: best_index,
                fallback_count: best.fallback_count,
                control_flow_blocks_routed: best.control_flow_blocks_routed,
                two_qubit_depth: best.quality.abstract_quality.two_qubit_depth,
                operation_count: best.quality.abstract_quality.operation_count,
                native_two_qubit_count: best.quality.native_two_qubit_ops,
                native_two_qubit_depth: best.quality.native_two_qubit_depth,
                native_total_depth: best.quality.native_total_depth,
                native_operation_count: best.quality.native_total_ops,
                unknown_loop_count: best.quality.unknown_loop_count,
                requirement_signature_count: target.requirements.len(),
                eager_pair_state_count: target.eager_pair_state_count,
                lazy_pair_l1_lookup_count: best.lazy_pair_l1_lookup_count,
                lazy_pair_l1_hit_count: best.lazy_pair_l1_hit_count,
                lazy_pair_l1_cached_count: best.lazy_pair_l1_cached_count,
            },
        },
        provenance: best.provenance,
    })
}

pub(crate) fn route_unscored_trial_with_metadata(
    sabre: &SabreDag,
    target: &RoutingTarget,
    metadata: &PreparedRouteMetadata,
    initial_layout: &Layout,
    heuristic: &SabreHeuristicConfig,
    seed: u64,
) -> Result<UnscoredTrial, CompilerError> {
    route_trial_with_metadata(
        sabre,
        target,
        metadata,
        initial_layout,
        heuristic,
        seed,
        true,
    )
}

pub(crate) fn refine_layout_with_metadata(
    sabre: &SabreDag,
    target: &RoutingTarget,
    metadata: &PreparedRouteMetadata,
    initial_layout: &Layout,
    heuristic: &SabreHeuristicConfig,
    seed: u64,
) -> Result<Layout, CompilerError> {
    route_trial_with_metadata(
        sabre,
        target,
        metadata,
        initial_layout,
        heuristic,
        seed,
        false,
    )
    .map(|trial| trial.final_layout)
}

fn route_trial_with_metadata(
    sabre: &SabreDag,
    target: &RoutingTarget,
    metadata: &PreparedRouteMetadata,
    initial_layout: &Layout,
    heuristic: &SabreHeuristicConfig,
    seed: u64,
    emit_operations: bool,
) -> Result<UnscoredTrial, CompilerError> {
    let mut output = TrialOutput::new(seed, emit_operations);
    let mut state = RoutingState::new(sabre, target, metadata, initial_layout, heuristic, seed)?;

    // Initial operations are dependency-free one-qubit or non-quantum work.
    // They can be emitted immediately under the starting layout.
    if output.emit_operations {
        for operation in &sabre.initial {
            output.push_mapped_operation(
                &operation.operation,
                Some(operation.source_order),
                &state.layout,
                target,
            )?;
        }
    }

    state.update_route(
        sabre,
        target,
        metadata,
        heuristic,
        &mut output,
        &sabre.first_layer,
        None,
    )?;
    state.populate_extended_set(sabre, target)?;

    let mut routable_nodes = Vec::with_capacity(2);
    let mut current_swaps = Vec::new();
    let mut mapping_cycles = MappingCycleDetector::new(state.layout_hash);
    let mut search_steps_since_decay_reset = 0usize;
    while !state.front_layer.is_empty() {
        current_swaps.clear();
        mapping_cycles.reset(state.layout_hash);
        let mut repeated_mapping = false;
        // Search accumulates speculative SWAPs until at least one front-layer
        // node becomes adjacent. Those SWAPs are emitted only when the routed
        // node is actually emitted, preserving a compact operation stream.
        while routable_nodes.is_empty()
            && !repeated_mapping
            && current_swaps.len() < heuristic.attempt_limit
        {
            let best_swap =
                state.choose_best_swap(target, heuristic, current_swaps.last().copied())?;
            state.apply_swap(best_swap.physical, target)?;
            current_swaps.push(best_swap.emitted);
            repeated_mapping = mapping_cycles.record_hash(state.layout_hash);
            let executable =
                |requirement, placement| target.terminal_cost_for(requirement, placement).is_some();
            for candidate in best_swap
                .indices
                .into_iter()
                .filter_map(|index| state.front_layer.routable_node_on_index(index, &executable))
            {
                if !routable_nodes.contains(&candidate) {
                    routable_nodes.push(candidate);
                }
            }

            if let Some(increment) = heuristic.decay_increment {
                search_steps_since_decay_reset += 1;
                if search_steps_since_decay_reset >= heuristic.decay_reset {
                    for value in &mut state.decay {
                        *value = 1.0;
                    }
                    search_steps_since_decay_reset = 0;
                } else {
                    state.decay[best_swap.indices[0]] += increment;
                    state.decay[best_swap.indices[1]] += increment;
                }
            }
        }

        if routable_nodes.is_empty() {
            // The heuristic failed to make progress within its attempt budget.
            // Roll back speculative swaps, then force progress along a shortest
            // path so the router cannot livelock on a poor local score.
            for swap in current_swaps.drain(..).rev() {
                state.apply_swap(swap, target)?;
            }
            output.fallback_count += 1;
            let forced = state.force_enable_closest_node(target, &mut current_swaps)?;
            routable_nodes.extend(forced);
        }

        let distance = |requirement, placement| {
            target.distance_for_cached(requirement, placement, Some(&state.lower_bound_cache))
        };
        for node in &routable_nodes {
            state.front_layer.remove(*node, &distance)?;
        }
        state.update_route(
            sabre,
            target,
            metadata,
            heuristic,
            &mut output,
            &routable_nodes,
            Some(&mut current_swaps),
        )?;
        state.populate_extended_set(sabre, target)?;
        if metadata.high_pair_reuse
            && state.front_layer.len().saturating_mul(WIDE_FRONT_FRACTION)
                >= target.physical_qubits.len()
            && heuristic.decay_increment.is_some()
        {
            state.decay.fill(1.0);
            search_steps_since_decay_reset = 0;
        }
        routable_nodes.clear();
    }

    let own_lazy_stats = state.lower_bound_cache.stats();
    Ok(UnscoredTrial {
        plan: output.plan,
        final_layout: state.layout.to_layout(&target.physical_qubits)?,
        swap_count: output.swap_count,
        fallback_count: output.fallback_count,
        control_flow_blocks_routed: output.control_flow_blocks_routed,
        lazy_pair_l1_lookup_count: output
            .lazy_pair_l1_lookup_count
            .saturating_add(own_lazy_stats.lookup_count),
        lazy_pair_l1_hit_count: output
            .lazy_pair_l1_hit_count
            .saturating_add(own_lazy_stats.hit_count),
        lazy_pair_l1_cached_count: output
            .lazy_pair_l1_cached_count
            .saturating_add(own_lazy_stats.cached_count),
        native_cost: output.native_cost,
        two_qubit_operation_count: output.two_qubit_operation_count,
        operation_count: output.operation_count,
    })
}

/// Derives per-trial seeds from an optional workflow seed.
///
/// A configured seed is the stable base for index-derived trial streams. This
/// gives reproducible but distinct tie-breaking streams without depending on
/// execution order. When no seed is configured, a random base is chosen.
pub(crate) fn trial_seeds(seed: Option<u64>, count: usize) -> Vec<u64> {
    let base = seed.unwrap_or_else(rand::random);
    (0..count)
        .map(|index| {
            let mut value = base ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            value ^ (value >> 31)
        })
        .collect()
}

/// Normalizes an initial layout against a device's usable physical topology.
///
/// The returned layout contains the supplied logical qubits and every usable
/// physical qubit from `device`. Logical qubits must already be mapped by
/// `initial_layout`; extra usable physical qubits remain vacant.
pub fn normalize_initial_layout(
    logical_qubits: &[LogicalQubit],
    device: &Device,
    initial_layout: &Layout,
) -> Result<Layout, CompilerError> {
    let physical = PhysicalLayoutGraph::from_device(device)?;
    let target = RoutingTarget::from_physical(&physical)?;
    normalize_initial_layout_for_target(logical_qubits, &target, initial_layout)
}

pub(crate) fn normalize_initial_layout_for_target(
    logical_qubits: &[LogicalQubit],
    target: &RoutingTarget,
    initial_layout: &Layout,
) -> Result<Layout, CompilerError> {
    let mut mapping = BTreeMap::new();
    for logical in logical_qubits {
        let physical = initial_layout.get_physical(*logical).ok_or_else(|| {
            CompilerError::InvalidInput(format!(
                "sabre initial layout does not map logical qubit {logical}"
            ))
        })?;
        if !target.physical_set.contains(&physical) {
            return Err(CompilerError::InvalidInput(format!(
                "sabre initial layout maps logical qubit {logical} to unusable physical qubit {physical}"
            )));
        }
        mapping.insert(*logical, physical);
    }
    Layout::new(
        logical_qubits.to_vec(),
        target.physical_qubits.clone(),
        Some(mapping),
    )
    .map_err(|error| {
        CompilerError::InvalidInput(format!("sabre initial layout is invalid: {error}"))
    })
}

/// Validates that every two-qubit interaction in `circuit` is reachable.
///
/// The check uses the usable physical topology of `device` and the logical to
/// physical mapping in `initial_layout`. It validates reachability, not current
/// adjacency; non-adjacent but connected interactions can still be routed by
/// SABRE.
pub fn validate_reachable_interactions(
    circuit: &Circuit,
    device: &Device,
    initial_layout: &Layout,
) -> Result<(), CompilerError> {
    let physical = PhysicalLayoutGraph::from_device(device)?;
    let sabre = SabreDag::from_operations(circuit.operations())?;
    let target = RoutingTarget::from_physical(&physical)?;
    let logical_qubits = circuit
        .qubits()
        .into_iter()
        .map(LogicalQubit::from_qubit)
        .collect::<Vec<_>>();
    let initial_layout =
        normalize_initial_layout_for_target(&logical_qubits, &target, initial_layout)?;
    validate_reachable_interactions_for_target(&sabre, &target, &initial_layout)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InteractionOperation {
    instruction: KnowledgeInstructionKey,
    qarg_roles: SmallVec<[u8; 2]>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum InteractionSignature {
    /// Topology-only routing model for arbitrary one-qubit operations.
    GenericUnary,
    /// Layout refinement DAGs deliberately omit concrete operations.
    GenericPair,
    Unary(Vec<InteractionOperation>),
    Pair(Vec<InteractionOperation>),
}

#[derive(Debug, Clone)]
enum RequirementTable {
    Unary {
        terminals: Vec<Option<NativePlanCost>>,
        lower_bounds: Vec<Option<RouteLowerBound>>,
    },
    Pair {
        terminals: BTreeMap<[usize; 2], NativePlanCost>,
        lower_bounds: Option<PairStateTable<RouteLowerBound>>,
    },
}

#[derive(Debug, Clone, Default)]
struct StructuralNativePlan {
    cost: NativePlanCost,
    leaves: Vec<SmallVec<[PhysicalQubit; 2]>>,
}

impl StructuralNativePlan {
    fn from_summary(summary: &NativePlanSummary) -> Self {
        Self {
            cost: structural_native_cost(summary),
            leaves: summary
                .leaves
                .iter()
                .map(|leaf| leaf.ordered_qargs.clone())
                .collect(),
        }
    }
}

fn structural_native_cost(summary: &NativePlanSummary) -> NativePlanCost {
    NativePlanCost {
        native_two_qubit_ops: summary.native_two_qubit_ops,
        native_total_ops: summary.native_total_ops,
        ..NativePlanCost::default()
    }
}

#[derive(Debug, Clone)]
struct PairStateTable<T> {
    width: usize,
    values: Vec<Option<T>>,
}

impl<T: Copy> PairStateTable<T> {
    fn new(width: usize) -> Self {
        Self {
            width,
            values: vec![None; width.saturating_mul(width.saturating_sub(1))],
        }
    }

    fn get(&self, left: usize, right: usize) -> Option<T> {
        Self::index(self.width, left, right)
            .and_then(|index| self.values.get(index))
            .copied()
            .flatten()
    }

    fn set(&mut self, left: usize, right: usize, value: T) {
        if let Some(index) = Self::index(self.width, left, right) {
            self.values[index] = Some(value);
        }
    }

    fn state_count(&self) -> usize {
        self.values.len()
    }

    fn index(width: usize, left: usize, right: usize) -> Option<usize> {
        if width < 2 || left >= width || right >= width || left == right {
            return None;
        }
        let right_without_diagonal = if right < left { right } else { right - 1 };
        left.checked_mul(width - 1)?
            .checked_add(right_without_diagonal)
    }
}

#[derive(Debug, Default)]
struct LazyPairCache {
    values: HashMap<(usize, usize, usize), Option<RouteLowerBound>>,
    flights: HashMap<(usize, usize, usize), Arc<LazyPairFlight>>,
}

#[derive(Debug)]
struct LazyPairFlight {
    state: Mutex<LazyPairFlightState>,
    ready: Condvar,
}

#[derive(Debug, Clone, Copy)]
enum LazyPairFlightState {
    Pending,
    Ready(Option<RouteLowerBound>),
    Aborted,
}

impl Default for LazyPairFlight {
    fn default() -> Self {
        Self {
            state: Mutex::new(LazyPairFlightState::Pending),
            ready: Condvar::new(),
        }
    }
}

struct LazyPairComputation<'a> {
    cache: &'a Mutex<LazyPairCache>,
    key: (usize, usize, usize),
    flight: Arc<LazyPairFlight>,
    completed: bool,
}

impl LazyPairComputation<'_> {
    fn publish(mut self, value: Option<RouteLowerBound>) {
        {
            let mut state = self
                .flight
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *state = LazyPairFlightState::Ready(value);
        }
        {
            let mut cache = self
                .cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if cache.values.len() < LAZY_PAIR_CACHE_BUDGET {
                cache.values.entry(self.key).or_insert(value);
            }
            cache.flights.remove(&self.key);
        }
        self.completed = true;
        self.flight.ready.notify_all();
    }
}

impl Drop for LazyPairComputation<'_> {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        {
            let mut state = self
                .flight
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *state = LazyPairFlightState::Aborted;
        }
        {
            let mut cache = self
                .cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if cache
                .flights
                .get(&self.key)
                .is_some_and(|flight| Arc::ptr_eq(flight, &self.flight))
            {
                cache.flights.remove(&self.key);
            }
        }
        self.flight.ready.notify_all();
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct TrialPairCacheStats {
    lookup_count: usize,
    hit_count: usize,
    cached_count: usize,
}

#[derive(Debug, Default)]
struct TrialPairCache {
    values: RefCell<HashMap<(usize, usize, usize), Option<RouteLowerBound>>>,
    lookup_count: Cell<usize>,
    hit_count: Cell<usize>,
}

impl TrialPairCache {
    fn get(&self, key: &(usize, usize, usize)) -> Option<Option<RouteLowerBound>> {
        self.lookup_count
            .set(self.lookup_count.get().saturating_add(1));
        let value = self.values.borrow().get(key).copied();
        if value.is_some() {
            self.hit_count.set(self.hit_count.get().saturating_add(1));
        }
        value
    }

    fn insert(&self, key: (usize, usize, usize), value: Option<RouteLowerBound>) {
        let mut values = self.values.borrow_mut();
        if values.len() < TRIAL_PAIR_CACHE_BUDGET {
            values.entry(key).or_insert(value);
        }
    }

    fn stats(&self) -> TrialPairCacheStats {
        TrialPairCacheStats {
            lookup_count: self.lookup_count.get(),
            hit_count: self.hit_count.get(),
            cached_count: self.values.borrow().len(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct RouteLowerBound {
    remaining_swaps: u32,
    native: NativePlanCost,
}

#[derive(Debug, Clone, Copy)]
struct MovementNeighbor {
    index: usize,
    swap: VerifiedSwap,
}

#[derive(Debug, Clone, Copy)]
struct VerifiedSwap {
    emitted_indices: [usize; 2],
    cost: NativePlanCost,
}

#[derive(Debug, Clone, Copy)]
struct MovementEdge {
    endpoints: [usize; 2],
    swap: VerifiedSwap,
}

impl RouteLowerBound {
    fn with_swap(self, swap: NativePlanCost) -> Self {
        Self {
            remaining_swaps: self.remaining_swaps.saturating_add(1),
            native: swap.combine(self.native),
        }
    }

    fn combine(self, other: Self) -> Self {
        Self {
            remaining_swaps: self.remaining_swaps.saturating_add(other.remaining_swaps),
            native: self.native.combine(other.native),
        }
    }

    fn compare(self, other: Self) -> Ordering {
        compare_optional_native_cost(Some(self.native), Some(other.native))
            .then_with(|| self.remaining_swaps.cmp(&other.remaining_swaps))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RoutingTarget {
    pub(crate) physical_qubits: Vec<PhysicalQubit>,
    physical_set: BTreeSet<PhysicalQubit>,
    physical_index: BTreeMap<PhysicalQubit, usize>,
    neighbors_by_index: Vec<Vec<MovementNeighbor>>,
    interaction_ids: HashMap<InteractionSignature, usize>,
    requirements: Vec<RequirementTable>,
    native_plans: HashMap<DeviceGateState, StructuralNativePlan>,
    native_unsupported: HashMap<DeviceGateState, DeviceLoweringFailure>,
    native_cost_enabled: bool,
    eager_pair_state_count: usize,
    lazy_pair_cache: Arc<Mutex<LazyPairCache>>,
    graph: UnGraph<(), ()>,
    graph_index: BTreeMap<PhysicalQubit, NodeIndex>,
    physical_by_index: Vec<PhysicalQubit>,
    topology_distance_values: Box<[u32]>,
}

struct PreparedRoutingParts {
    movement_edges: Vec<MovementEdge>,
    interaction_ids: HashMap<InteractionSignature, usize>,
    requirements: Vec<RequirementTable>,
    native_plans: HashMap<DeviceGateState, StructuralNativePlan>,
    native_unsupported: HashMap<DeviceGateState, DeviceLoweringFailure>,
    native_cost_enabled: bool,
}

impl RoutingTarget {
    pub(crate) fn uses_native_costs(&self) -> bool {
        self.native_cost_enabled
    }

    /// Builds the indexed routing view used by SABRE scoring.
    ///
    /// The target keeps both semantic physical-qubit ids and dense indices.
    /// Dense indices make layer scoring cheap; semantic ids keep diagnostics
    /// and emitted SWAP operations stable.
    pub(crate) fn from_physical(physical: &PhysicalLayoutGraph) -> Result<Self, CompilerError> {
        let count = physical.physical_qubits().len();
        let edges = undirected_topology_edges(physical);
        let movement_edges = edges
            .iter()
            .copied()
            .map(|(left, right)| MovementEdge {
                endpoints: [left, right],
                swap: VerifiedSwap {
                    emitted_indices: [left, right],
                    cost: NativePlanCost::default(),
                },
            })
            .collect::<Vec<_>>();
        let unary_terminals = vec![Some(NativePlanCost::default()); count];
        let unary_lower_bounds = unary_route_lower_bounds(
            &movement_adjacency(count, &movement_edges),
            &unary_terminals,
        );
        let mut pair_terminals = BTreeMap::new();
        for &(left, right) in &edges {
            pair_terminals.insert([left, right], NativePlanCost::default());
            pair_terminals.insert([right, left], NativePlanCost::default());
        }
        Self::from_prepared_parts(
            physical,
            PreparedRoutingParts {
                movement_edges,
                interaction_ids: HashMap::from([
                    (InteractionSignature::GenericUnary, 0),
                    (InteractionSignature::GenericPair, 1),
                ]),
                requirements: vec![
                    RequirementTable::Unary {
                        terminals: unary_terminals,
                        lower_bounds: unary_lower_bounds,
                    },
                    RequirementTable::Pair {
                        lower_bounds: None,
                        terminals: pair_terminals,
                    },
                ],
                native_plans: HashMap::new(),
                native_unsupported: HashMap::new(),
                native_cost_enabled: false,
            },
        )
    }

    /// Builds a device-aware target whose movement edges are exactly those on
    /// which the final device lowerer can realize an emitted SWAP.
    pub(crate) fn from_device_with_session(
        device: &Device,
        physical: &PhysicalLayoutGraph,
        sabre: &SabreDag,
        planning_session: &DevicePlanningSession,
    ) -> Result<Self, CompilerError> {
        Self::from_device_with_pair_state_budget_and_session(
            device,
            physical,
            sabre,
            EAGER_PAIR_STATE_BUDGET,
            planning_session,
        )
    }

    fn from_device_with_pair_state_budget_and_session(
        device: &Device,
        physical: &PhysicalLayoutGraph,
        sabre: &SabreDag,
        pair_state_budget: usize,
        planning_session: &DevicePlanningSession,
    ) -> Result<Self, CompilerError> {
        let topology_edges = undirected_topology_edges(physical);
        let physical_qubits = physical.physical_qubits();
        if !device_declares_routing_capability(device, physical_qubits) {
            return Err(CompilerError::InvalidInput(format!(
                "exact device-native SABRE routing requires native capabilities on device '{}'",
                device.name()
            )));
        }
        let mut pair_state_budget = pair_state_budget;
        let signatures = sabre.ordered_interaction_signatures()?;
        let interaction_ids = signatures
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, signature)| (signature, index))
            .collect::<HashMap<_, _>>();

        let roots = collect_device_plan_roots(physical_qubits, &topology_edges, &signatures);

        let catalog = NativePlanCatalog::build_with_session(planning_session, roots)?;
        let native_identity = NativePlanCost::default();
        let native_plans = catalog
            .iter()
            .map(|(state, summary)| (state.clone(), StructuralNativePlan::from_summary(summary)))
            .collect();
        let native_unsupported = catalog
            .iter_availability()
            .filter_map(|(state, availability)| {
                let NativePlanAvailability::Unsupported(failure) = availability else {
                    return None;
                };
                Some((state.clone(), failure.clone()))
            })
            .collect();
        let count = physical_qubits.len();
        let mut movement_edges = Vec::new();
        for &(left_index, right_index) in &topology_edges {
            let mut candidates = [[left_index, right_index], [right_index, left_index]]
                .into_iter()
                .filter_map(|emitted_indices| {
                    let state = DeviceGateState::standard(
                        StandardGate::SWAP,
                        smallvec![
                            physical_qubits[emitted_indices[0]],
                            physical_qubits[emitted_indices[1]]
                        ],
                    );
                    catalog.summary(&state).map(|summary| VerifiedSwap {
                        emitted_indices,
                        cost: structural_native_cost(summary),
                    })
                })
                .collect::<Vec<_>>();
            candidates.sort_by(|left, right| {
                compare_optional_native_cost(Some(left.cost), Some(right.cost)).then_with(|| {
                    left.emitted_indices
                        .map(|index| physical_qubits[index])
                        .cmp(&right.emitted_indices.map(|index| physical_qubits[index]))
                })
            });
            if let Some(swap) = candidates.into_iter().next() {
                movement_edges.push(MovementEdge {
                    endpoints: [left_index, right_index],
                    swap,
                });
            }
        }

        let neighbors = movement_adjacency(count, &movement_edges);
        let requirements = build_device_requirement_tables(
            &signatures,
            DeviceRequirementInputs {
                physical_qubits,
                topology_edges: &topology_edges,
                movement_edges: &movement_edges,
                neighbors: &neighbors,
                catalog: &catalog,
                native_identity,
            },
            &mut pair_state_budget,
        );

        Self::from_prepared_parts(
            physical,
            PreparedRoutingParts {
                movement_edges,
                interaction_ids,
                requirements,
                native_plans,
                native_unsupported,
                native_cost_enabled: true,
            },
        )
    }

    fn from_prepared_parts(
        physical: &PhysicalLayoutGraph,
        parts: PreparedRoutingParts,
    ) -> Result<Self, CompilerError> {
        let PreparedRoutingParts {
            movement_edges,
            interaction_ids,
            requirements,
            native_plans,
            native_unsupported,
            native_cost_enabled,
        } = parts;
        let physical_qubits = physical.physical_qubits().to_vec();
        let physical_set = physical_qubits.iter().copied().collect::<BTreeSet<_>>();
        let mut graph = UnGraph::with_capacity(physical_qubits.len(), 0);
        let mut graph_index = BTreeMap::new();
        let mut physical_index = BTreeMap::new();
        let mut physical_by_index = Vec::with_capacity(physical_qubits.len());

        for (dense_index, physical) in physical_qubits.iter().copied().enumerate() {
            let graph_node = graph.add_node(());
            graph_index.insert(physical, graph_node);
            physical_index.insert(physical, dense_index);
            physical_by_index.push(physical);
        }
        let mut neighbors_by_index = vec![Vec::new(); physical_qubits.len()];

        for edge in movement_edges {
            let [left_index, right_index] = edge.endpoints;
            let left = physical_qubits[left_index];
            let right = physical_qubits[right_index];
            neighbors_by_index[left_index].push(MovementNeighbor {
                index: right_index,
                swap: edge.swap,
            });
            neighbors_by_index[right_index].push(MovementNeighbor {
                index: left_index,
                swap: edge.swap,
            });
            graph.add_edge(graph_index[&left], graph_index[&right], ());
        }
        for items in &mut neighbors_by_index {
            items.sort_unstable_by_key(|neighbor| physical_qubits[neighbor.index]);
        }

        let eager_pair_state_count = requirements
            .iter()
            .filter_map(|requirement| match requirement {
                RequirementTable::Pair {
                    lower_bounds: Some(lower_bounds),
                    ..
                } => Some(lower_bounds.state_count()),
                RequirementTable::Unary { .. }
                | RequirementTable::Pair {
                    lower_bounds: None, ..
                } => None,
            })
            .sum();

        let topology_distances = physical.distances().clone();
        let topology_distance_values = (0..physical_qubits.len())
            .flat_map(|left| {
                let distances = &topology_distances;
                (0..physical_qubits.len())
                    .map(move |right| distances.distance_by_index(left, right).unwrap_or(u32::MAX))
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Ok(Self {
            physical_qubits,
            physical_set,
            physical_index,
            neighbors_by_index,
            interaction_ids,
            requirements,
            native_plans,
            native_unsupported,
            native_cost_enabled,
            eager_pair_state_count,
            lazy_pair_cache: Arc::new(Mutex::new(LazyPairCache::default())),
            graph,
            graph_index,
            physical_by_index,
            topology_distance_values,
        })
    }

    fn physical_index(&self, physical: PhysicalQubit) -> Result<usize, CompilerError> {
        self.physical_index.get(&physical).copied().ok_or_else(|| {
            CompilerError::InvalidInput(format!(
                "physical qubit {physical} is not usable in the target topology"
            ))
        })
    }

    fn physical_at(&self, index: usize) -> Result<PhysicalQubit, CompilerError> {
        self.physical_qubits.get(index).copied().ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "physical index {index} is outside target topology of length {}",
                self.physical_qubits.len()
            ))
        })
    }

    #[inline(always)]
    fn distance_for_cached(
        &self,
        requirement: usize,
        placement: RequirementPlacement,
        cache: Option<&TrialPairCache>,
    ) -> Result<f64, CompilerError> {
        if !self.native_cost_enabled {
            return match placement {
                RequirementPlacement::Unary(_) => Ok(1.0),
                RequirementPlacement::Pair([left, right]) => {
                    let distance = self.topology_distance(left, right).ok_or_else(|| {
                            CompilerError::InvalidInput(format!(
                                "routing requirement {requirement} at {placement:?} cannot reach an executable terminal using lowerable SWAPs"
                            ))
                        })?;
                    Ok(f64::from(distance))
                }
            };
        }
        self.route_lower_bound_for_cached(requirement, placement, cache)
            .map(|bound| f64::from(bound.remaining_swaps) + 1.0)
            .ok_or_else(|| {
                CompilerError::InvalidInput(format!(
                    "routing requirement {requirement} at {placement:?} cannot reach an executable terminal using lowerable SWAPs"
                ))
            })
    }

    #[inline(always)]
    fn topology_distance(&self, left: usize, right: usize) -> Option<u32> {
        let width = self.physical_qubits.len();
        left.checked_mul(width)
            .and_then(|offset| offset.checked_add(right))
            .and_then(|index| self.topology_distance_values.get(index))
            .copied()
            .filter(|distance| *distance != u32::MAX)
    }

    fn terminal_cost_for(
        &self,
        requirement: usize,
        placement: RequirementPlacement,
    ) -> Option<NativePlanCost> {
        match (self.requirements.get(requirement)?, placement) {
            (RequirementTable::Unary { terminals, .. }, RequirementPlacement::Unary(physical)) => {
                terminals.get(physical).copied().flatten()
            }
            (
                RequirementTable::Pair { terminals, .. },
                RequirementPlacement::Pair([left, right]),
            ) => terminals.get(&[left, right]).copied(),
            _ => None,
        }
    }

    fn verified_swap(
        &self,
        swap: [PhysicalQubit; 2],
    ) -> Result<([PhysicalQubit; 2], VerifiedSwap), CompilerError> {
        let left = self.physical_index(swap[0])?;
        let right = self.physical_index(swap[1])?;
        let verified = self.neighbors_by_index[left]
            .iter()
            .find(|neighbor| neighbor.index == right)
            .map(|neighbor| neighbor.swap)
            .ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "SABRE attempted to emit non-movement SWAP({}, {})",
                    swap[0], swap[1]
                ))
            })?;
        let emitted = verified
            .emitted_indices
            .map(|index| self.physical_qubits[index]);
        Ok((emitted, verified))
    }

    fn swap_cost(&self, swap: [PhysicalQubit; 2]) -> Result<NativePlanCost, CompilerError> {
        let (_, verified) = self.verified_swap(swap)?;
        Ok(verified.cost)
    }

    fn swap_operation(&self, swap: [PhysicalQubit; 2]) -> Result<Operation, CompilerError> {
        let (emitted, _) = self.verified_swap(swap)?;
        if self.native_cost_enabled {
            let state =
                DeviceGateState::standard(StandardGate::SWAP, SmallVec::from_slice(&emitted));
            if !self.native_plans.contains_key(&state) {
                let detail = self.native_unsupported.get(&state).map_or_else(
                    || "native SWAP plan was not prepared".to_string(),
                    ToString::to_string,
                );
                return Err(CompilerError::InvariantViolation(format!(
                    "SABRE attempted to emit SWAP({}, {}) without a verified exact-qargs native plan: {detail}",
                    emitted[0], emitted[1]
                )));
            }
        }
        Ok(swap_operation(emitted))
    }

    fn has_terminal(&self, requirement: usize) -> bool {
        match self.requirements.get(requirement) {
            Some(RequirementTable::Unary { terminals, .. }) => {
                terminals.iter().any(Option::is_some)
            }
            Some(RequirementTable::Pair { terminals, .. }) => !terminals.is_empty(),
            None => false,
        }
    }

    fn distance_steps_for(
        &self,
        requirement: usize,
        placement: RequirementPlacement,
    ) -> Option<u32> {
        self.route_lower_bound_for_cached(requirement, placement, None)
            .map(|bound| bound.remaining_swaps)
    }

    fn route_lower_bound_for_cached(
        &self,
        requirement: usize,
        placement: RequirementPlacement,
        cache: Option<&TrialPairCache>,
    ) -> Option<RouteLowerBound> {
        if !self.native_cost_enabled {
            let remaining_swaps = match placement {
                RequirementPlacement::Unary(physical) => {
                    (physical < self.physical_qubits.len()).then_some(0)?
                }
                RequirementPlacement::Pair([left, right]) => {
                    self.topology_distance(left, right)?.checked_sub(1)?
                }
            };
            return Some(RouteLowerBound {
                remaining_swaps,
                native: NativePlanCost::default(),
            });
        }
        match (self.requirements.get(requirement)?, placement) {
            (
                RequirementTable::Unary { lower_bounds, .. },
                RequirementPlacement::Unary(physical),
            ) => lower_bounds.get(physical).copied().flatten(),
            (
                RequirementTable::Pair {
                    terminals,
                    lower_bounds,
                },
                RequirementPlacement::Pair([left, right]),
            ) => lower_bounds.as_ref().map_or_else(
                || {
                    let key = (requirement, left, right);
                    if let Some(cache) = cache
                        && let Some(value) = cache.get(&key)
                    {
                        return value;
                    }
                    let value =
                        self.lazy_pair_route_lower_bound(requirement, terminals, left, right);
                    if let Some(cache) = cache {
                        cache.insert(key, value);
                    }
                    value
                },
                |bounds| bounds.get(left, right),
            ),
            _ => None,
        }
    }

    fn lazy_pair_route_lower_bound(
        &self,
        requirement: usize,
        terminals: &BTreeMap<[usize; 2], NativePlanCost>,
        left: usize,
        right: usize,
    ) -> Option<RouteLowerBound> {
        let key = (requirement, left, right);
        lazy_pair_lookup_or_compute(&self.lazy_pair_cache, key, || {
            pair_route_lower_bound_from_state(&self.neighbors_by_index, terminals, [left, right])
        })
    }

    fn interaction_id_for_node(
        &self,
        sabre: &SabreDag,
        node: NodeIndex,
    ) -> Result<usize, CompilerError> {
        let signature = sabre.graph[node].interaction_signature()?;
        self.interaction_ids
            .get(&signature)
            .copied()
            .or_else(|| {
                let generic = match &sabre.graph[node].kind {
                    SabreNodeKind::Unary(_) => InteractionSignature::GenericUnary,
                    SabreNodeKind::TwoQ(_)
                    | SabreNodeKind::Synchronize
                    | SabreNodeKind::ControlFlow(_) => InteractionSignature::GenericPair,
                };
                self.interaction_ids.get(&generic).copied()
            })
            .ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "routing target has no generic interaction model".to_string(),
                )
            })
    }

    fn native_cost(&self, state: &DeviceGateState) -> Option<NativePlanCost> {
        self.native_plans.get(state).map(|plan| plan.cost)
    }

    fn native_plan(&self, state: &DeviceGateState) -> Option<&StructuralNativePlan> {
        self.native_plans.get(state)
    }

    fn unsupported_native_plan(&self, state: &DeviceGateState) -> Option<&DeviceLoweringFailure> {
        self.native_unsupported.get(state)
    }
}

fn lazy_pair_lookup_or_compute(
    cache: &Mutex<LazyPairCache>,
    key: (usize, usize, usize),
    compute: impl FnOnce() -> Option<RouteLowerBound>,
) -> Option<RouteLowerBound> {
    enum Lookup {
        Ready(Option<RouteLowerBound>),
        Wait(Arc<LazyPairFlight>),
        Compute(Arc<LazyPairFlight>),
    }

    let mut compute = Some(compute);
    loop {
        let lookup = {
            let mut cache = cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(value) = cache.values.get(&key).copied() {
                Lookup::Ready(value)
            } else if let Some(flight) = cache.flights.get(&key) {
                Lookup::Wait(Arc::clone(flight))
            } else {
                let flight = Arc::new(LazyPairFlight::default());
                cache.flights.insert(key, Arc::clone(&flight));
                Lookup::Compute(flight)
            }
        };

        match lookup {
            Lookup::Ready(value) => return value,
            Lookup::Wait(flight) => {
                let mut state = flight
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                while matches!(*state, LazyPairFlightState::Pending) {
                    state = flight
                        .ready
                        .wait(state)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                match *state {
                    LazyPairFlightState::Ready(value) => return value,
                    LazyPairFlightState::Aborted => continue,
                    LazyPairFlightState::Pending => continue,
                }
            }
            Lookup::Compute(flight) => {
                let computation = LazyPairComputation {
                    cache,
                    key,
                    flight,
                    completed: false,
                };
                let compute = compute
                    .take()
                    .expect("a lazy pair lookup computes at most once");
                let value = compute();
                computation.publish(value);
                return value;
            }
        }
    }
}

fn undirected_topology_edges(physical: &PhysicalLayoutGraph) -> Vec<(usize, usize)> {
    physical.undirected_edges_by_index().collect()
}

fn device_declares_routing_capability(device: &Device, physical_qubits: &[PhysicalQubit]) -> bool {
    if !device.native_gates().is_empty() {
        return true;
    }
    if physical_qubits.iter().copied().any(|physical| {
        device
            .qubit_properties(physical)
            .is_some_and(|properties| !properties.native_instructions().is_empty())
    }) {
        return true;
    }
    let usable = physical_qubits.iter().copied().collect::<BTreeSet<_>>();
    device.topology().undirected_edges().any(|(left, right)| {
        usable.contains(&left)
            && usable.contains(&right)
            && [
                device.edge_properties(left, right),
                device.edge_properties(right, left),
            ]
            .into_iter()
            .flatten()
            .any(|properties| !properties.native_instructions().is_empty())
    })
}

impl SabreNode {
    fn interaction_signature(&self) -> Result<InteractionSignature, CompilerError> {
        let logicals: SmallVec<[LogicalQubit; 2]> = match &self.kind {
            SabreNodeKind::Unary(logical) => smallvec![*logical],
            SabreNodeKind::TwoQ(pair) => SmallVec::from_slice(pair),
            SabreNodeKind::Synchronize | SabreNodeKind::ControlFlow(_) => {
                return Ok(InteractionSignature::GenericPair);
            }
        };
        if self.operations.is_empty() {
            return Ok(match &self.kind {
                SabreNodeKind::Unary(_) => InteractionSignature::GenericUnary,
                SabreNodeKind::TwoQ(_)
                | SabreNodeKind::Synchronize
                | SabreNodeKind::ControlFlow(_) => InteractionSignature::GenericPair,
            });
        }

        let mut operations = Vec::new();
        for operation in &self.operations {
            let Some(instruction) =
                KnowledgeInstructionKey::from_instruction(&operation.instruction)
            else {
                match operation.instruction {
                    Instruction::UnitaryGate(_) | Instruction::CircuitGate(_) => {
                        return Err(CompilerError::InvalidInput(format!(
                            "sabre device preparation requires {} to be decomposed before routing",
                            operation.instruction
                        )));
                    }
                    _ => continue,
                }
            };
            if matches!(
                instruction,
                KnowledgeInstructionKey::Standard(StandardGate::GPhase)
            ) {
                continue;
            }
            let mut qarg_roles = SmallVec::new();
            for qubit in &operation.qubits {
                let logical = LogicalQubit::from_qubit(*qubit);
                let Some(role) = logicals.iter().position(|candidate| *candidate == logical) else {
                    return Err(CompilerError::InvariantViolation(format!(
                        "folded SABRE node on {logicals:?} contains operation {} on unrelated logical qubit {logical}",
                        operation.instruction
                    )));
                };
                qarg_roles.push(role as u8);
            }
            operations.push(InteractionOperation {
                instruction,
                qarg_roles,
            });
        }
        if operations.is_empty() {
            Ok(match &self.kind {
                SabreNodeKind::Unary(_) => InteractionSignature::GenericUnary,
                SabreNodeKind::TwoQ(_)
                | SabreNodeKind::Synchronize
                | SabreNodeKind::ControlFlow(_) => InteractionSignature::GenericPair,
            })
        } else {
            match logicals.len() {
                1 => Ok(InteractionSignature::Unary(operations)),
                2 => Ok(InteractionSignature::Pair(operations)),
                arity => Err(CompilerError::InvariantViolation(format!(
                    "routing interaction signature has unsupported arity {arity}"
                ))),
            }
        }
    }
}

impl SabreDag {
    fn collect_interaction_signatures(
        &self,
        output: &mut HashSet<InteractionSignature>,
    ) -> Result<(), CompilerError> {
        for node in self.graph.node_weights() {
            match &node.kind {
                SabreNodeKind::Unary(_) | SabreNodeKind::TwoQ(_) => {
                    output.insert(node.interaction_signature()?);
                }
                SabreNodeKind::ControlFlow(SabreControlFlow::If {
                    then_body,
                    else_body,
                    ..
                }) => {
                    then_body.collect_interaction_signatures(output)?;
                    if let Some(else_body) = else_body {
                        else_body.collect_interaction_signatures(output)?;
                    }
                }
                SabreNodeKind::ControlFlow(
                    SabreControlFlow::While { body, .. } | SabreControlFlow::For { body, .. },
                ) => body.collect_interaction_signatures(output)?,
                SabreNodeKind::ControlFlow(SabreControlFlow::Switch { cases, default, .. }) => {
                    for case in cases {
                        case.body.collect_interaction_signatures(output)?;
                    }
                    if let Some(default) = default {
                        default.collect_interaction_signatures(output)?;
                    }
                }
                SabreNodeKind::Synchronize => {}
            }
        }
        Ok(())
    }

    fn ordered_interaction_signatures(&self) -> Result<Vec<InteractionSignature>, CompilerError> {
        let mut signatures = HashSet::from([
            InteractionSignature::GenericUnary,
            InteractionSignature::GenericPair,
        ]);
        self.collect_interaction_signatures(&mut signatures)?;
        let mut signatures = signatures.into_iter().collect::<Vec<_>>();
        signatures.sort();
        Ok(signatures)
    }
}

fn states_for_requirement(
    operations: &[InteractionOperation],
    ordered_qargs: &[PhysicalQubit],
) -> Vec<DeviceGateState> {
    operations
        .iter()
        .map(|operation| DeviceGateState {
            instruction: operation.instruction.clone(),
            ordered_qargs: operation
                .qarg_roles
                .iter()
                .map(|role| ordered_qargs[usize::from(*role)])
                .collect(),
        })
        .collect()
}

fn combined_catalog_cost(
    catalog: &NativePlanCatalog,
    states: Vec<DeviceGateState>,
) -> Option<NativePlanCost> {
    let mut cost = None;
    for state in states {
        let summary = catalog.summary(&state)?;
        let next = structural_native_cost(summary);
        cost = Some(cost.map_or(next, |current: NativePlanCost| current.combine(next)));
    }
    Some(cost.unwrap_or_default())
}

fn collect_device_plan_roots(
    physical_qubits: &[PhysicalQubit],
    topology_edges: &[(usize, usize)],
    signatures: &[InteractionSignature],
) -> Vec<DeviceGateState> {
    let mut roots = Vec::new();
    for signature in signatures {
        match signature {
            InteractionSignature::Unary(operations) => {
                for &physical in physical_qubits {
                    roots.extend(states_for_requirement(operations, &[physical]));
                }
            }
            InteractionSignature::GenericUnary
            | InteractionSignature::GenericPair
            | InteractionSignature::Pair(_) => {}
        }
    }
    for &(left_index, right_index) in topology_edges {
        let left = physical_qubits[left_index];
        let right = physical_qubits[right_index];
        for ordered in [[left, right], [right, left]] {
            roots.push(DeviceGateState::standard(
                StandardGate::SWAP,
                SmallVec::from_slice(&ordered),
            ));
        }
        for signature in signatures {
            let InteractionSignature::Pair(operations) = signature else {
                continue;
            };
            for ordered in [[left, right], [right, left]] {
                roots.extend(states_for_requirement(operations, &ordered));
            }
        }
    }
    roots
}

struct DeviceRequirementInputs<'a> {
    physical_qubits: &'a [PhysicalQubit],
    topology_edges: &'a [(usize, usize)],
    movement_edges: &'a [MovementEdge],
    neighbors: &'a [Vec<MovementNeighbor>],
    catalog: &'a NativePlanCatalog,
    native_identity: NativePlanCost,
}

enum PreparedRequirementTerminals {
    Unary(Vec<Option<NativePlanCost>>),
    Pair(BTreeMap<[usize; 2], NativePlanCost>),
}

fn build_device_requirement_tables(
    signatures: &[InteractionSignature],
    inputs: DeviceRequirementInputs<'_>,
    pair_state_budget: &mut usize,
) -> Vec<RequirementTable> {
    let DeviceRequirementInputs {
        physical_qubits,
        topology_edges,
        movement_edges,
        neighbors,
        catalog,
        native_identity,
    } = inputs;
    let count = physical_qubits.len();
    let mut prepared = signatures
        .iter()
        .map(|signature| match signature {
            InteractionSignature::GenericUnary => {
                PreparedRequirementTerminals::Unary(vec![Some(native_identity); count])
            }
            InteractionSignature::GenericPair => {
                let mut terminals = BTreeMap::new();
                for edge in movement_edges {
                    let [left, right] = edge.endpoints;
                    terminals.insert([left, right], native_identity);
                    terminals.insert([right, left], native_identity);
                }
                PreparedRequirementTerminals::Pair(terminals)
            }
            InteractionSignature::Unary(operations) => {
                let mut terminals = vec![None; count];
                for (physical_index, physical) in physical_qubits.iter().copied().enumerate() {
                    let states = states_for_requirement(operations, &[physical]);
                    if let Some(cost) = combined_catalog_cost(catalog, states) {
                        terminals[physical_index] = Some(cost);
                    }
                }
                PreparedRequirementTerminals::Unary(terminals)
            }
            InteractionSignature::Pair(operations) => {
                let mut terminals = BTreeMap::new();
                for &(left_index, right_index) in topology_edges {
                    for (source, target) in [(left_index, right_index), (right_index, left_index)] {
                        let ordered = [physical_qubits[source], physical_qubits[target]];
                        let states = states_for_requirement(operations, &ordered);
                        if let Some(cost) = combined_catalog_cost(catalog, states) {
                            terminals.insert([source, target], cost);
                        }
                    }
                }
                PreparedRequirementTerminals::Pair(terminals)
            }
        })
        .collect::<Vec<_>>();

    // Refinement DAGs deliberately use a generic pair signature. It is
    // terminal wherever at least one exact source interaction is terminal;
    // final route scoring still checks the exact folded signature. Collect the
    // final terminal set before building any lower-bound table, so the generic
    // table is built once. Exact source signatures receive eager-state budget
    // first; the generic refinement table uses only the remaining budget.
    if let Some(generic_index) = signatures
        .iter()
        .position(|signature| matches!(signature, InteractionSignature::GenericPair))
    {
        let generic_terminals = prepared
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != generic_index)
            .map(|(_, requirement)| requirement)
            .filter_map(|requirement| match requirement {
                PreparedRequirementTerminals::Pair(terminals) => Some(terminals.keys().copied()),
                PreparedRequirementTerminals::Unary(_) => None,
            })
            .flatten()
            .collect::<BTreeSet<_>>();
        if let PreparedRequirementTerminals::Pair(terminals) = &mut prepared[generic_index] {
            terminals.extend(
                generic_terminals
                    .into_iter()
                    .map(|placement| (placement, native_identity)),
            );
        }
    }

    let mut prepared = prepared.into_iter().enumerate().collect::<Vec<_>>();
    prepared
        .sort_by_key(|(index, _)| matches!(signatures[*index], InteractionSignature::GenericPair));
    let mut requirements = BTreeMap::new();
    for (index, terminals) in prepared {
        let requirement = match terminals {
            PreparedRequirementTerminals::Unary(terminals) => RequirementTable::Unary {
                lower_bounds: unary_route_lower_bounds(neighbors, &terminals),
                terminals,
            },
            PreparedRequirementTerminals::Pair(terminals) => RequirementTable::Pair {
                lower_bounds: eager_pair_route_lower_bounds(
                    neighbors,
                    &terminals,
                    pair_state_budget,
                ),
                terminals,
            },
        };
        requirements.insert(index, requirement);
    }
    requirements.into_values().collect()
}

fn movement_adjacency(count: usize, edges: &[MovementEdge]) -> Vec<Vec<MovementNeighbor>> {
    let mut sparse_costs = vec![BTreeMap::new(); count];
    for edge in edges {
        let [left, right] = edge.endpoints;
        sparse_costs[left].insert(right, edge.swap);
        sparse_costs[right].insert(left, edge.swap);
    }
    sparse_costs
        .into_iter()
        .map(|adjacent| {
            adjacent
                .into_iter()
                .map(|(index, swap)| MovementNeighbor { index, swap })
                .collect()
        })
        .collect()
}

fn unary_route_lower_bounds(
    swap_neighbors: &[Vec<MovementNeighbor>],
    terminals: &[Option<NativePlanCost>],
) -> Vec<Option<RouteLowerBound>> {
    let mut bounds = terminals
        .iter()
        .map(|terminal| {
            terminal.map(|native| RouteLowerBound {
                remaining_swaps: 0,
                native,
            })
        })
        .collect::<Vec<_>>();
    let mut queue = bounds
        .iter()
        .enumerate()
        .filter_map(|(physical, bound)| bound.map(|_| physical))
        .collect::<VecDeque<_>>();
    while let Some(physical) = queue.pop_front() {
        let current = bounds[physical].expect("queued unary state has a lower bound");
        for &neighbor in &swap_neighbors[physical] {
            let predecessor = neighbor.index;
            let candidate = current.with_swap(neighbor.swap.cost);
            if bounds[predecessor].is_none_or(|previous| candidate.compare(previous).is_lt()) {
                bounds[predecessor] = Some(candidate);
                queue.push_back(predecessor);
            }
        }
    }
    bounds
}

fn pair_after_swap(pair: [usize; 2], swap: [usize; 2]) -> [usize; 2] {
    pair.map(|physical| {
        if physical == swap[0] {
            swap[1]
        } else if physical == swap[1] {
            swap[0]
        } else {
            physical
        }
    })
}

fn pair_route_lower_bounds(
    swap_neighbors: &[Vec<MovementNeighbor>],
    terminals: &BTreeMap<[usize; 2], NativePlanCost>,
) -> PairStateTable<RouteLowerBound> {
    let count = swap_neighbors.len();
    let mut bounds = PairStateTable::new(count);
    let mut queue = VecDeque::new();
    for (&[left, right], &native) in terminals {
        if left == right || left >= count || right >= count {
            continue;
        }
        bounds.set(
            left,
            right,
            RouteLowerBound {
                remaining_swaps: 0,
                native,
            },
        );
        queue.push_back((left, right));
    }

    while let Some((left, right)) = queue.pop_front() {
        let current = bounds
            .get(left, right)
            .expect("queued pair state has a lower bound");
        for endpoint in [left, right] {
            for &neighbor in &swap_neighbors[endpoint] {
                let [previous_left, previous_right] =
                    pair_after_swap([left, right], [endpoint, neighbor.index]);
                let candidate = current.with_swap(neighbor.swap.cost);
                if bounds
                    .get(previous_left, previous_right)
                    .is_none_or(|bound| candidate.compare(bound).is_lt())
                {
                    bounds.set(previous_left, previous_right, candidate);
                    queue.push_back((previous_left, previous_right));
                }
            }
        }
    }
    bounds
}

fn eager_pair_route_lower_bounds(
    swap_neighbors: &[Vec<MovementNeighbor>],
    terminals: &BTreeMap<[usize; 2], NativePlanCost>,
    remaining_budget: &mut usize,
) -> Option<PairStateTable<RouteLowerBound>> {
    let count = swap_neighbors.len();
    let states = count.saturating_mul(count.saturating_sub(1));
    if states > *remaining_budget {
        return None;
    }
    *remaining_budget -= states;
    Some(pair_route_lower_bounds(swap_neighbors, terminals))
}

fn pair_route_lower_bound_from_state(
    swap_neighbors: &[Vec<MovementNeighbor>],
    terminals: &BTreeMap<[usize; 2], NativePlanCost>,
    start: [usize; 2],
) -> Option<RouteLowerBound> {
    let count = swap_neighbors.len();
    PairStateTable::<RouteLowerBound>::index(count, start[0], start[1])?;
    let mut bounds = BTreeMap::<[usize; 2], RouteLowerBound>::new();
    let mut queue = BinaryHeap::new();
    for (&placement, &native) in terminals {
        if PairStateTable::<RouteLowerBound>::index(count, placement[0], placement[1]).is_none() {
            continue;
        }
        let bound = RouteLowerBound {
            remaining_swaps: 0,
            native,
        };
        bounds.insert(placement, bound);
        queue.push(Reverse((native.native_two_qubit_ops, 0_u32, placement)));
    }

    while let Some(Reverse((native_two_qubit_ops, remaining_swaps, placement))) = queue.pop() {
        let current = bounds.get(&placement).copied()?;
        if current.native.native_two_qubit_ops != native_two_qubit_ops
            || current.remaining_swaps != remaining_swaps
        {
            continue;
        }
        if placement == start {
            return Some(current);
        }
        for endpoint in placement {
            for &neighbor in &swap_neighbors[endpoint] {
                let predecessor = pair_after_swap(placement, [endpoint, neighbor.index]);
                let candidate = current.with_swap(neighbor.swap.cost);
                if bounds
                    .get(&predecessor)
                    .is_none_or(|previous| candidate.compare(*previous).is_lt())
                {
                    bounds.insert(predecessor, candidate);
                    queue.push(Reverse((
                        candidate.native.native_two_qubit_ops,
                        candidate.remaining_swaps,
                        predecessor,
                    )));
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedRouteMetadata {
    required_predecessors: Vec<u32>,
    requirement_ids: Vec<Option<usize>>,
    /// Prepared metadata for control-flow bodies, indexed by parent DAG node.
    /// Bodies use `SabreControlFlow::bodies` canonical traversal order.
    nested: Vec<Vec<PreparedRouteMetadata>>,
    high_pair_reuse: bool,
}

impl PreparedRouteMetadata {
    pub(crate) fn new(sabre: &SabreDag, target: &RoutingTarget) -> Result<Self, CompilerError> {
        let mut required_predecessors = vec![0; sabre.graph.node_count()];
        for edge in sabre.graph.edge_references() {
            required_predecessors[edge.target().index()] += 1;
        }
        let requirement_ids = sabre
            .graph
            .node_indices()
            .map(|node| match sabre.graph[node].kind {
                SabreNodeKind::Unary(_) | SabreNodeKind::TwoQ(_) => {
                    target.interaction_id_for_node(sabre, node).map(Some)
                }
                SabreNodeKind::Synchronize | SabreNodeKind::ControlFlow(_) => Ok(None),
            })
            .collect::<Result<Vec<_>, CompilerError>>()?;
        let nested = sabre
            .graph
            .node_indices()
            .map(|node| match &sabre.graph[node].kind {
                SabreNodeKind::ControlFlow(flow) => flow
                    .bodies()
                    .into_iter()
                    .map(|body| Self::new(body, target))
                    .collect(),
                _ => Ok(Vec::new()),
            })
            .collect::<Result<Vec<_>, CompilerError>>()?;
        let mut two_qubit_count = 0usize;
        let mut unique_pairs = HashSet::new();
        for node in sabre.graph.node_weights() {
            if let SabreNodeKind::TwoQ([left, right]) = &node.kind {
                two_qubit_count += 1;
                let pair = if left <= right {
                    (*left, *right)
                } else {
                    (*right, *left)
                };
                unique_pairs.insert(pair);
            }
        }
        let high_pair_reuse = !unique_pairs.is_empty()
            && two_qubit_count >= unique_pairs.len().saturating_mul(HIGH_PAIR_REUSE_FACTOR);
        Ok(Self {
            required_predecessors,
            requirement_ids,
            nested,
            high_pair_reuse,
        })
    }
}

#[derive(Debug)]
struct RoutingState {
    layout: DenseRoutingLayout,
    layout_hash: u64,
    front_layer: Layer,
    lookahead_layers: Vec<Layer>,
    required_predecessors: Vec<u32>,
    requirement_ids: Vec<Option<usize>>,
    lower_bound_cache: TrialPairCache,
    decay: Vec<f64>,
    active_marks: Vec<u32>,
    active_generation: u32,
    active_index_scratch: Vec<usize>,
    candidate_scratch: Vec<SwapChoice>,
    scored_scratch: Vec<ScoredCandidate>,
    eligible_scratch: Vec<ScoredCandidate>,
    lookahead_next_scratch: Vec<NodeIndex>,
    lookahead_visit_scratch: Vec<NodeIndex>,
    lookahead_decrements: Vec<u32>,
    lookahead_touched_scratch: Vec<usize>,
    rng: StdRng,
}

#[derive(Debug, Clone, Copy)]
struct SwapChoice {
    physical: [PhysicalQubit; 2],
    emitted: [PhysicalQubit; 2],
    indices: [usize; 2],
    cost: NativePlanCost,
}

#[derive(Debug, Clone, Copy)]
struct ScoredCandidate {
    choice: SwapChoice,
    adjusted_score: f64,
    /// `None` means the exact native route cost has not been queried yet.
    /// `Some(None)` records an unreachable candidate and must be cached too.
    route_cost: Option<Option<RouteLowerBound>>,
}

impl ScoredCandidate {
    fn cached_route_cost(
        &mut self,
        calculate: impl FnOnce(SwapChoice) -> Option<RouteLowerBound>,
    ) -> Option<RouteLowerBound> {
        *self
            .route_cost
            .get_or_insert_with(|| calculate(self.choice))
    }

    fn prepared_route_cost(self) -> Option<RouteLowerBound> {
        self.route_cost
            .expect("native route cost must be cached before candidate comparison")
    }
}

impl RoutingState {
    fn prepared_requirement_id(
        requirement_ids: &[Option<usize>],
        node: NodeIndex,
    ) -> Result<usize, CompilerError> {
        requirement_ids
            .get(node.index())
            .copied()
            .flatten()
            .ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "routing node {} has no prepared requirement id",
                    node.index()
                ))
            })
    }

    /// Creates mutable state for one SABRE routing trial.
    ///
    /// `required_predecessors` is the mutable readiness counter for DAG
    /// scheduling. Lookahead temporarily edits the same counters and restores
    /// them before returning to the real routing loop.
    fn new(
        sabre: &SabreDag,
        target: &RoutingTarget,
        metadata: &PreparedRouteMetadata,
        layout: &Layout,
        heuristic: &SabreHeuristicConfig,
        seed: u64,
    ) -> Result<Self, CompilerError> {
        let layout = DenseRoutingLayout::from_layout(layout, &target.physical_qubits)?;
        let layout_hash = MappingCycleDetector::mapping_hash(layout.signature());
        Ok(Self {
            layout,
            layout_hash,
            front_layer: Layer::new(sabre.graph.node_count(), target.physical_qubits.len()),
            lookahead_layers: vec![
                Layer::new(
                    sabre.graph.node_count(),
                    target.physical_qubits.len()
                );
                heuristic.lookahead_weights.len()
            ],
            required_predecessors: metadata.required_predecessors.clone(),
            requirement_ids: metadata.requirement_ids.clone(),
            lower_bound_cache: TrialPairCache::default(),
            decay: vec![1.0; target.physical_qubits.len()],
            active_marks: vec![0; target.physical_qubits.len()],
            active_generation: 0,
            active_index_scratch: Vec::new(),
            candidate_scratch: Vec::new(),
            scored_scratch: Vec::new(),
            eligible_scratch: Vec::new(),
            lookahead_next_scratch: Vec::new(),
            lookahead_visit_scratch: Vec::new(),
            lookahead_decrements: vec![0; sabre.graph.node_count()],
            lookahead_touched_scratch: Vec::new(),
            rng: StdRng::seed_from_u64(seed),
        })
    }

    /// Applies a physical SWAP to the layout and all cached layer scores.
    ///
    /// The layout and every cached layer must move together; otherwise future
    /// SWAP deltas would be scored against stale physical positions.
    fn apply_swap(
        &mut self,
        swap: [PhysicalQubit; 2],
        target: &RoutingTarget,
    ) -> Result<(), CompilerError> {
        let swap_indices = [
            target.physical_index(swap[0])?,
            target.physical_index(swap[1])?,
        ];
        let before_left = self.layout.signature()[swap_indices[0]];
        let before_right = self.layout.signature()[swap_indices[1]];
        if target.native_cost_enabled {
            let distance = |requirement, placement| {
                target.distance_for_cached(requirement, placement, Some(&self.lower_bound_cache))
            };
            self.front_layer.apply_swap(swap_indices, &distance)?;
            for layer in &mut self.lookahead_layers {
                layer.apply_swap(swap_indices, &distance)?;
            }
        } else {
            let distances = target.topology_distance_values.as_ref();
            let width = target.physical_qubits.len();
            self.front_layer
                .apply_topology_swap(swap_indices, distances, width)?;
            for layer in &mut self.lookahead_layers {
                layer.apply_topology_swap(swap_indices, distances, width)?;
            }
        }
        self.layout
            .swap_physical_indices(swap_indices[0], swap_indices[1]);
        self.layout_hash = MappingCycleDetector::hash_after_swap(
            self.layout_hash,
            swap_indices,
            before_left,
            before_right,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn update_route(
        &mut self,
        sabre: &SabreDag,
        target: &RoutingTarget,
        metadata: &PreparedRouteMetadata,
        heuristic: &SabreHeuristicConfig,
        output: &mut TrialOutput,
        nodes: &[NodeIndex],
        initial_swaps: Option<&mut Vec<[PhysicalQubit; 2]>>,
    ) -> Result<(), CompilerError> {
        let mut to_visit = nodes.iter().copied().collect::<VecDeque<_>>();
        let mut pending_swaps = initial_swaps;

        while let Some(node_id) = to_visit.pop_front() {
            let node = &sabre.graph[node_id];
            match &node.kind {
                SabreNodeKind::Unary(logical) => {
                    let physical_index = self.layout.physical_index(*logical)?;
                    let requirement =
                        Self::prepared_requirement_id(&self.requirement_ids, node_id)?;
                    let placement = RequirementPlacement::Unary(physical_index);
                    if target.terminal_cost_for(requirement, placement).is_none() {
                        let distance = |requirement, placement| {
                            target.distance_for_cached(
                                requirement,
                                placement,
                                Some(&self.lower_bound_cache),
                            )
                        };
                        self.front_layer
                            .insert(node_id, requirement, placement, &distance)?;
                        continue;
                    }
                    output.apply_pending_swaps(target, pending_swaps.take())?;
                    if output.emit_operations {
                        for operation in &node.operations {
                            output.push_mapped_operation(
                                &operation.operation,
                                Some(operation.source_order),
                                &self.layout,
                                target,
                            )?;
                        }
                    }
                }
                SabreNodeKind::TwoQ(pair) => {
                    // A two-qubit node that is still non-adjacent becomes part
                    // of the front layer. Adjacent nodes flush any pending
                    // SWAPs and are emitted under the current layout.
                    let physical = [
                        target.physical_at(self.layout.physical_index(pair[0])?)?,
                        target.physical_at(self.layout.physical_index(pair[1])?)?,
                    ];
                    let interaction =
                        Self::prepared_requirement_id(&self.requirement_ids, node_id)?;
                    let physical_indices = [
                        target.physical_index(physical[0])?,
                        target.physical_index(physical[1])?,
                    ];
                    if target
                        .terminal_cost_for(
                            interaction,
                            RequirementPlacement::Pair(physical_indices),
                        )
                        .is_none()
                    {
                        let distance = |requirement, placement| {
                            target.distance_for_cached(
                                requirement,
                                placement,
                                Some(&self.lower_bound_cache),
                            )
                        };
                        self.front_layer.insert(
                            node_id,
                            interaction,
                            RequirementPlacement::Pair(physical_indices),
                            &distance,
                        )?;
                        continue;
                    }
                    output.apply_pending_swaps(target, pending_swaps.take())?;
                    if output.emit_operations {
                        for operation in &node.operations {
                            output.push_mapped_operation(
                                &operation.operation,
                                Some(operation.source_order),
                                &self.layout,
                                target,
                            )?;
                        }
                    }
                }
                SabreNodeKind::Synchronize => {
                    // Synchronize nodes preserve parent-level ordering around
                    // classical or barrier-like effects but do not add an
                    // adjacency constraint of their own.
                    output.apply_pending_swaps(target, pending_swaps.take())?;
                    if output.emit_operations {
                        for operation in &node.operations {
                            output.push_mapped_operation(
                                &operation.operation,
                                Some(operation.source_order),
                                &self.layout,
                                target,
                            )?;
                        }
                    }
                }
                SabreNodeKind::ControlFlow(flow) => {
                    // Control-flow bodies are routed recursively from the
                    // current entry layout and restore that layout on exit, so
                    // parent routing can continue with a single layout state.
                    output.apply_pending_swaps(target, pending_swaps.take())?;
                    self.route_control_flow_node(
                        flow,
                        &node.operations,
                        metadata.nested.get(node_id.index()).ok_or_else(|| {
                            CompilerError::InvariantViolation(format!(
                                "routing node {} has no prepared nested metadata slot",
                                node_id.index()
                            ))
                        })?,
                        target,
                        heuristic,
                        output,
                    )?;
                }
            }

            for edge in sabre.graph.edges_directed(node_id, Direction::Outgoing) {
                let successor = edge.target();
                self.required_predecessors[successor.index()] -= 1;
                if self.required_predecessors[successor.index()] == 0 {
                    to_visit.push_back(successor);
                }
            }
        }

        if pending_swaps.is_some() {
            return Err(CompilerError::InvariantViolation(
                "sabre selected swaps that did not route any front-layer node".to_string(),
            ));
        }
        Ok(())
    }

    fn route_control_flow_node(
        &mut self,
        flow: &SabreControlFlow,
        operations: &[SabreOperation],
        body_metadata: &[PreparedRouteMetadata],
        target: &RoutingTarget,
        heuristic: &SabreHeuristicConfig,
        output: &mut TrialOutput,
    ) -> Result<(), CompilerError> {
        let Some((first, rest)) = operations.split_first() else {
            return Ok(());
        };
        let entry_layout = self.layout.to_layout(&target.physical_qubits)?;
        // The SABRE DAG keeps the representative control-flow operation first
        // and may attach additional bookkeeping operations after it. Rebuild the
        // first operation with routed bodies, then map the remaining operations
        // through the unchanged parent layout.
        let emit_operations = output.emit_operations;
        let mut metadata = body_metadata.iter();
        let routed = match flow {
            SabreControlFlow::If {
                condition,
                then_body,
                else_body,
            } => {
                let then_result = route_control_flow_body(
                    then_body,
                    target,
                    metadata
                        .next()
                        .ok_or_else(|| missing_nested_metadata("if then"))?,
                    &entry_layout,
                    heuristic,
                    output.next_nested_seed(),
                    emit_operations,
                )?;
                let else_result = else_body
                    .as_ref()
                    .map(|body| {
                        route_control_flow_body(
                            body,
                            target,
                            metadata
                                .next()
                                .ok_or_else(|| missing_nested_metadata("if else"))?,
                            &entry_layout,
                            heuristic,
                            output.next_nested_seed(),
                            emit_operations,
                        )
                    })
                    .transpose()?;
                output.merge_nested(&then_result);
                if let Some(result) = &else_result {
                    output.merge_nested(result);
                }
                let then_operations = then_result.materialize_operations(target)?;
                let else_operations = else_result
                    .as_ref()
                    .map(|result| result.materialize_operations(target))
                    .transpose()?;
                let flow = ClassicalControlOp::If(IfOp::new(
                    condition.clone(),
                    ControlBody::new(then_operations),
                    else_operations.map(ControlBody::new),
                )?);
                let qubits = flow.used_qubits().into_iter().collect();
                Operation {
                    instruction: Instruction::ClassicalControl(flow),
                    qubits,
                    params: SmallVec::new(),
                    label: first.label.clone(),
                }
            }
            SabreControlFlow::While { condition, body } => {
                let body_result = route_control_flow_body(
                    body,
                    target,
                    metadata
                        .next()
                        .ok_or_else(|| missing_nested_metadata("while"))?,
                    &entry_layout,
                    heuristic,
                    output.next_nested_seed(),
                    emit_operations,
                )?;
                output.merge_nested(&body_result);
                let body_operations = body_result.materialize_operations(target)?;
                let flow = ClassicalControlOp::While(WhileOp::new(
                    condition.clone(),
                    ControlBody::new(body_operations),
                )?);
                let qubits = flow.used_qubits().into_iter().collect();
                Operation {
                    instruction: Instruction::ClassicalControl(flow),
                    qubits,
                    params: SmallVec::new(),
                    label: first.label.clone(),
                }
            }
            SabreControlFlow::For {
                var,
                start,
                stop,
                step,
                body,
            } => {
                let body_result = route_control_flow_body(
                    body,
                    target,
                    metadata
                        .next()
                        .ok_or_else(|| missing_nested_metadata("for"))?,
                    &entry_layout,
                    heuristic,
                    output.next_nested_seed(),
                    emit_operations,
                )?;
                output.merge_nested(&body_result);
                let body_operations = body_result.materialize_operations(target)?;
                let flow = ClassicalControlOp::For(ForOp::new(
                    *var,
                    start.clone(),
                    stop.clone(),
                    step.clone(),
                    ControlBody::new(body_operations),
                )?);
                let qubits = flow.used_qubits().into_iter().collect();
                Operation {
                    instruction: Instruction::ClassicalControl(flow),
                    qubits,
                    params: SmallVec::new(),
                    label: first.label.clone(),
                }
            }
            SabreControlFlow::Switch {
                target: switch_target,
                cases,
                default,
            } => {
                let mut routed_cases = Vec::with_capacity(cases.len());
                for case in cases {
                    let result = route_control_flow_body(
                        &case.body,
                        target,
                        metadata
                            .next()
                            .ok_or_else(|| missing_nested_metadata("switch case"))?,
                        &entry_layout,
                        heuristic,
                        output.next_nested_seed(),
                        emit_operations,
                    )?;
                    output.merge_nested(&result);
                    routed_cases.push(SwitchCase::new(
                        case.value,
                        ControlBody::new(result.materialize_operations(target)?),
                    ));
                }
                let routed_default = default
                    .as_ref()
                    .map(|body| {
                        route_control_flow_body(
                            body,
                            target,
                            metadata
                                .next()
                                .ok_or_else(|| missing_nested_metadata("switch default"))?,
                            &entry_layout,
                            heuristic,
                            output.next_nested_seed(),
                            emit_operations,
                        )
                    })
                    .transpose()?;
                if let Some(result) = &routed_default {
                    output.merge_nested(result);
                }
                let default_operations = routed_default
                    .as_ref()
                    .map(|result| result.materialize_operations(target))
                    .transpose()?;
                let flow = ClassicalControlOp::Switch(SwitchOp::new(
                    switch_target.clone(),
                    routed_cases,
                    default_operations.map(ControlBody::new),
                )?);
                let qubits = flow.used_qubits().into_iter().collect();
                Operation {
                    instruction: Instruction::ClassicalControl(flow),
                    qubits,
                    params: SmallVec::new(),
                    label: first.label.clone(),
                }
            }
        };
        if output.emit_operations {
            output.push_operation(routed, None, target)?;
            for operation in rest {
                output.push_mapped_operation(
                    &operation.operation,
                    Some(operation.source_order),
                    &self.layout,
                    target,
                )?;
            }
        }
        Ok(())
    }

    fn populate_extended_set(
        &mut self,
        sabre: &SabreDag,
        target: &RoutingTarget,
    ) -> Result<(), CompilerError> {
        // Reuse a dense shadow of the readiness counters and visit buffers.
        // Exploration touches only successors of the current front, while the
        // real scheduler counters are restored before returning.
        let mut next_visit = std::mem::take(&mut self.lookahead_next_scratch);
        let mut to_visit = std::mem::take(&mut self.lookahead_visit_scratch);
        let mut touched = std::mem::take(&mut self.lookahead_touched_scratch);
        next_visit.clear();
        to_visit.clear();
        touched.clear();
        next_visit.extend(self.front_layer.iter_nodes());
        self.lookahead_layers.iter_mut().for_each(Layer::clear);

        let layout = &self.layout;
        let requirement_ids = &self.requirement_ids;
        let cache = &self.lower_bound_cache;
        let required_predecessors = &mut self.required_predecessors;
        let decrements = &mut self.lookahead_decrements;

        for layer in &mut self.lookahead_layers {
            for node in next_visit.drain(..) {
                release_shadow_successors(
                    sabre,
                    node,
                    required_predecessors,
                    decrements,
                    &mut touched,
                    &mut to_visit,
                );
            }

            let mut index = 0;
            while index < to_visit.len() {
                let node = to_visit[index];
                match &sabre.graph[node].kind {
                    SabreNodeKind::Unary(logical) => {
                        if !target.native_cost_enabled {
                            // Topology-only unary requirements are executable
                            // at every usable physical position. They do not
                            // consume routing horizon: release their successors
                            // in the same shadow layer, matching the scheduler.
                            release_shadow_successors(
                                sabre,
                                node,
                                required_predecessors,
                                decrements,
                                &mut touched,
                                &mut to_visit,
                            );
                        } else if let Ok(physical_index) = layout.physical_index(*logical) {
                            let requirement = Self::prepared_requirement_id(requirement_ids, node)?;
                            let placement = RequirementPlacement::Unary(physical_index);
                            let distance = |requirement, placement| {
                                target.distance_for_cached(requirement, placement, Some(cache))
                            };
                            layer.insert(node, requirement, placement, &distance)?;
                            next_visit.push(node);
                        }
                    }
                    SabreNodeKind::TwoQ(pair) => {
                        if let (Ok(left), Ok(right)) = (
                            layout.physical_index(pair[0]),
                            layout.physical_index(pair[1]),
                        ) {
                            let interaction = Self::prepared_requirement_id(requirement_ids, node)?;
                            let placement = RequirementPlacement::Pair([left, right]);
                            let distance = |requirement, placement| {
                                target.distance_for_cached(requirement, placement, Some(cache))
                            };
                            layer.insert(node, interaction, placement, &distance)?;
                            next_visit.push(node);
                        }
                    }
                    SabreNodeKind::Synchronize | SabreNodeKind::ControlFlow(_) => {
                        release_shadow_successors(
                            sabre,
                            node,
                            required_predecessors,
                            decrements,
                            &mut touched,
                            &mut to_visit,
                        );
                    }
                }
                index += 1;
            }
            to_visit.clear();
        }

        for index in touched.iter().copied() {
            required_predecessors[index] += decrements[index];
            decrements[index] = 0;
        }
        self.lookahead_next_scratch = next_visit;
        self.lookahead_visit_scratch = to_visit;
        self.lookahead_touched_scratch = touched;
        Ok(())
    }

    fn choose_best_swap(
        &mut self,
        target: &RoutingTarget,
        heuristic: &SabreHeuristicConfig,
        previous_swap: Option<[PhysicalQubit; 2]>,
    ) -> Result<SwapChoice, CompilerError> {
        if self.front_layer.is_empty() {
            return Err(CompilerError::InvariantViolation(
                "sabre cannot select a SWAP for an empty front layer".to_string(),
            ));
        }
        self.active_generation = self.active_generation.wrapping_add(1);
        if self.active_generation == 0 {
            self.active_marks.fill(0);
            self.active_generation = 1;
        }
        let mut active_indices = std::mem::take(&mut self.active_index_scratch);
        active_indices.clear();
        for active_index in self.front_layer.active_indices() {
            if self.active_marks[active_index] != self.active_generation {
                self.active_marks[active_index] = self.active_generation;
                active_indices.push(active_index);
            }
        }
        // Preserve the candidate order of the previous full physical-table
        // scan so seeded tie-breaking remains bit-for-bit deterministic.
        active_indices.sort_unstable();

        let mut candidates = std::mem::take(&mut self.candidate_scratch);
        candidates.clear();
        for &active_index in &active_indices {
            let active = target.physical_at(active_index)?;
            for &movement_neighbor in &target.neighbors_by_index[active_index] {
                let neighbor_index = movement_neighbor.index;
                if neighbor_index < active_index
                    && self.active_marks[neighbor_index] == self.active_generation
                {
                    continue;
                }
                let neighbor = target.physical_at(neighbor_index)?;
                candidates.push(if active <= neighbor {
                    SwapChoice {
                        physical: [active, neighbor],
                        emitted: movement_neighbor
                            .swap
                            .emitted_indices
                            .map(|index| target.physical_qubits[index]),
                        indices: [active_index, neighbor_index],
                        cost: movement_neighbor.swap.cost,
                    }
                } else {
                    SwapChoice {
                        physical: [neighbor, active],
                        emitted: movement_neighbor
                            .swap
                            .emitted_indices
                            .map(|index| target.physical_qubits[index]),
                        indices: [neighbor_index, active_index],
                        cost: movement_neighbor.swap.cost,
                    }
                });
            }
        }
        if candidates.len() > 1
            && let Some(previous_swap) = previous_swap
        {
            let previous_swap = if previous_swap[0] <= previous_swap[1] {
                previous_swap
            } else {
                [previous_swap[1], previous_swap[0]]
            };
            candidates.retain(|candidate| candidate.physical != previous_swap);
        }
        if candidates.is_empty() {
            return Err(CompilerError::TransformFailed {
                name: "sabre_route",
                reason: "no candidate SWAP can affect the front layer".to_string(),
            });
        }

        let mut best_score = f64::INFINITY;
        let mut scored = std::mem::take(&mut self.scored_scratch);
        scored.clear();
        scored.reserve(candidates.len().saturating_sub(scored.capacity()));
        for candidate in candidates.drain(..) {
            let structural_score = if target.native_cost_enabled {
                let distance = |requirement, placement| {
                    target.distance_for_cached(
                        requirement,
                        placement,
                        Some(&self.lower_bound_cache),
                    )
                };
                heuristic_score_after_swap(
                    &self.front_layer,
                    &self.lookahead_layers,
                    heuristic,
                    candidate.indices,
                    &distance,
                )?
            } else {
                heuristic_topology_score_after_swap(
                    &self.front_layer,
                    &self.lookahead_layers,
                    heuristic,
                    candidate.indices,
                    target.topology_distance_values.as_ref(),
                    target.physical_qubits.len(),
                )
            };
            let score = if heuristic.decay_increment.is_some() {
                structural_score
                    * self.decay[candidate.indices[0]].max(self.decay[candidate.indices[1]])
            } else {
                structural_score
            };

            best_score = best_score.min(score);
            scored.push(ScoredCandidate {
                choice: candidate,
                adjusted_score: score,
                route_cost: None,
            });
        }

        let mut eligible = std::mem::take(&mut self.eligible_scratch);
        eligible.clear();
        let candidate_window = 0.25 * heuristic.basic_weight;
        eligible.extend(scored.drain(..).filter(|candidate| {
            candidate.adjusted_score <= best_score + candidate_window + heuristic.best_epsilon
        }));
        if target.native_cost_enabled {
            for candidate in &mut eligible {
                candidate.cached_route_cost(|choice| self.route_cost_after_swap(target, choice));
            }
            eligible.sort_by(|left, right| {
                compare_optional_route_bound(
                    left.prepared_route_cost(),
                    right.prepared_route_cost(),
                )
                .then_with(|| left.adjusted_score.total_cmp(&right.adjusted_score))
                .then_with(|| left.choice.physical.cmp(&right.choice.physical))
            });
            if let Some(best) = eligible.first().copied() {
                let best_cost = best.prepared_route_cost();
                eligible.retain(|candidate| {
                    compare_optional_route_bound(candidate.prepared_route_cost(), best_cost)
                        == Ordering::Equal
                });
            }
        }
        let best_adjusted = eligible
            .iter()
            .map(|candidate| candidate.adjusted_score)
            .min_by(f64::total_cmp)
            .unwrap_or(best_score);
        eligible
            .retain(|candidate| candidate.adjusted_score <= best_adjusted + heuristic.best_epsilon);
        let selected = eligible
            .choose(&mut self.rng)
            .map(|candidate| candidate.choice)
            .ok_or_else(|| {
                CompilerError::InvariantViolation("sabre found no best SWAP".to_string())
            })?;
        candidates.clear();
        scored.clear();
        eligible.clear();
        self.candidate_scratch = candidates;
        self.active_index_scratch = active_indices;
        self.scored_scratch = scored;
        self.eligible_scratch = eligible;
        Ok(selected)
    }

    fn route_cost_after_swap(
        &self,
        target: &RoutingTarget,
        candidate: SwapChoice,
    ) -> Option<RouteLowerBound> {
        let mut cost = RouteLowerBound {
            remaining_swaps: 1,
            native: candidate.cost,
        };
        for (requirement, placement) in self.front_layer.placements_after_swap(candidate.indices) {
            cost = cost.combine(target.route_lower_bound_for_cached(
                requirement,
                placement,
                Some(&self.lower_bound_cache),
            )?);
        }
        Some(cost)
    }

    fn force_enable_closest_node(
        &mut self,
        target: &RoutingTarget,
        current_swaps: &mut Vec<[PhysicalQubit; 2]>,
    ) -> Result<Vec<NodeIndex>, CompilerError> {
        // Fallback follows the exact unary/pair state lower bound. Each step is
        // a lowerable SWAP and strictly reduces the remaining distance.
        let mut closest = None;
        for (node, requirement, placement) in self.front_layer.iter() {
            let distance = target
                .route_lower_bound_for_cached(requirement, placement, Some(&self.lower_bound_cache))
                .map_or(u32::MAX, |bound| bound.remaining_swaps);
            if closest
                .as_ref()
                .is_none_or(|(_, _, _, closest_distance)| distance < *closest_distance)
            {
                closest = Some((node, requirement, placement, distance));
            }
        }
        let (closest_node, requirement, mut placement, _) = closest.ok_or_else(|| {
            CompilerError::InvariantViolation(
                "sabre fallback called with an empty front layer".to_string(),
            )
        })?;
        while let Some(distance) = target
            .route_lower_bound_for_cached(requirement, placement, Some(&self.lower_bound_cache))
            .map(|bound| bound.remaining_swaps)
        {
            if distance == 0 {
                break;
            }
            let mut improving = Vec::new();
            let endpoints: SmallVec<[usize; 2]> = match placement {
                RequirementPlacement::Unary(physical) => smallvec![physical],
                RequirementPlacement::Pair(pair) => SmallVec::from_slice(&pair),
            };
            for endpoint in endpoints {
                for &neighbor in &target.neighbors_by_index[endpoint] {
                    let swap = [endpoint, neighbor.index];
                    let next = placement.after_swap(swap);
                    if target
                        .route_lower_bound_for_cached(
                            requirement,
                            next,
                            Some(&self.lower_bound_cache),
                        )
                        .map(|bound| bound.remaining_swaps)
                        == Some(distance - 1)
                    {
                        improving.push((swap, next, neighbor.swap));
                    }
                }
            }
            improving.sort_by(|(left_swap, _, left), (right_swap, _, right)| {
                compare_optional_native_cost(Some(left.cost), Some(right.cost)).then_with(|| {
                    let left = [
                        target.physical_qubits[left_swap[0]],
                        target.physical_qubits[left_swap[1]],
                    ];
                    let right = [
                        target.physical_qubits[right_swap[0]],
                        target.physical_qubits[right_swap[1]],
                    ];
                    left.cmp(&right)
                })
            });
            let Some((swap_indices, next, verified)) = improving.into_iter().next() else {
                return Err(CompilerError::InvariantViolation(format!(
                    "routing-state distance {distance} has no improving lowerable SWAP"
                )));
            };
            let swap = [
                target.physical_at(swap_indices[0])?,
                target.physical_at(swap_indices[1])?,
            ];
            self.apply_swap(swap, target)?;
            current_swaps.push(
                verified
                    .emitted_indices
                    .map(|index| target.physical_qubits[index]),
            );
            placement = next;
        }

        let routed = self
            .front_layer
            .iter()
            .filter_map(|(node, requirement, placement)| {
                target
                    .terminal_cost_for(requirement, placement)
                    .is_some()
                    .then_some(node)
            })
            .collect::<Vec<_>>();
        if !routed.contains(&closest_node) {
            return Err(CompilerError::InvariantViolation(
                "routing-state fallback did not enable its selected node".to_string(),
            ));
        }
        Ok(routed)
    }
}

fn release_shadow_successors(
    sabre: &SabreDag,
    node: NodeIndex,
    required_predecessors: &mut [u32],
    decrements: &mut [u32],
    touched: &mut Vec<usize>,
    to_visit: &mut Vec<NodeIndex>,
) {
    for edge in sabre.graph.edges_directed(node, Direction::Outgoing) {
        let successor = edge.target();
        let successor_index = successor.index();
        if decrements[successor_index] == 0 {
            touched.push(successor_index);
        }
        decrements[successor_index] += 1;
        required_predecessors[successor_index] -= 1;
        if required_predecessors[successor_index] == 0 {
            to_visit.push(successor);
        }
    }
}

/// Detects recent mapping cycles with an incrementally maintained hash.
///
/// A hit is only a search signal: routing rolls back and takes the guaranteed
/// progress fallback. Therefore a theoretical hash collision cannot invalidate
/// the routed circuit.
struct MappingCycleDetector {
    current_hash: u64,
    recent: VecDeque<u64>,
    seen: HashSet<u64>,
}

impl MappingCycleDetector {
    const WINDOW: usize = 32;

    fn new(current_hash: u64) -> Self {
        Self {
            current_hash,
            recent: VecDeque::from([current_hash]),
            seen: HashSet::from([current_hash]),
        }
    }

    fn reset(&mut self, current_hash: u64) {
        self.current_hash = current_hash;
        self.recent.clear();
        self.recent.push_back(current_hash);
        self.seen.clear();
        self.seen.insert(current_hash);
    }

    fn record_hash(&mut self, current_hash: u64) -> bool {
        self.current_hash = current_hash;
        if self.seen.contains(&self.current_hash) {
            return true;
        }
        self.recent.push_back(self.current_hash);
        self.seen.insert(self.current_hash);
        if self.recent.len() > Self::WINDOW
            && let Some(expired) = self.recent.pop_front()
        {
            self.seen.remove(&expired);
        }
        false
    }

    fn hash_after_swap(
        current_hash: u64,
        [left, right]: [usize; 2],
        before_left: Option<usize>,
        before_right: Option<usize>,
    ) -> u64 {
        current_hash
            ^ Self::entry_hash(left, before_left)
            ^ Self::entry_hash(right, before_right)
            ^ Self::entry_hash(left, before_right)
            ^ Self::entry_hash(right, before_left)
    }

    fn mapping_hash(mapping: &[Option<usize>]) -> u64 {
        mapping
            .iter()
            .copied()
            .enumerate()
            .fold(0, |hash, (physical, logical)| {
                hash ^ Self::entry_hash(physical, logical)
            })
    }

    fn entry_hash(physical: usize, logical: Option<usize>) -> u64 {
        let logical = logical.map_or(0, |logical| logical as u64 + 1);
        let mut value = (physical as u64)
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(logical);
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

fn compare_optional_native_cost(
    left: Option<NativePlanCost>,
    right: Option<NativePlanCost>,
) -> Ordering {
    compare_some_first_by(left, right, |left, right| {
        left.native_two_qubit_ops.cmp(&right.native_two_qubit_ops)
    })
}

fn compare_optional_route_bound(
    left: Option<RouteLowerBound>,
    right: Option<RouteLowerBound>,
) -> Ordering {
    compare_some_first_by(left, right, |left, right| left.compare(right))
}

fn heuristic_score_after_swap(
    front_layer: &Layer,
    lookahead_layers: &[Layer],
    heuristic: &SabreHeuristicConfig,
    swap: [usize; 2],
    distances: &impl Fn(usize, RequirementPlacement) -> Result<f64, CompilerError>,
) -> Result<f64, CompilerError> {
    let mut score = heuristic.basic_weight * front_layer.total_score_after_swap(swap, distances)?;
    for (layer, weight) in lookahead_layers
        .iter()
        .zip(heuristic.lookahead_weights.iter().copied())
    {
        score +=
            weight * layer.total_score_after_swap(swap, distances)? / layer.len().max(1) as f64;
    }
    Ok(score)
}

fn heuristic_topology_score_after_swap(
    front_layer: &Layer,
    lookahead_layers: &[Layer],
    heuristic: &SabreHeuristicConfig,
    swap: [usize; 2],
    distances: &[u32],
    width: usize,
) -> f64 {
    let mut score = heuristic.basic_weight
        * front_layer.total_topology_score_after_swap(swap, distances, width);
    for (layer, weight) in lookahead_layers
        .iter()
        .zip(heuristic.lookahead_weights.iter().copied())
    {
        score += weight * layer.total_topology_score_after_swap(swap, distances, width)
            / layer.len().max(1) as f64;
    }
    score
}

#[derive(Debug, Default)]
struct TrialOutput {
    plan: CompactRoutePlan,
    emit_operations: bool,
    swap_count: usize,
    fallback_count: usize,
    control_flow_blocks_routed: usize,
    lazy_pair_l1_lookup_count: usize,
    lazy_pair_l1_hit_count: usize,
    lazy_pair_l1_cached_count: usize,
    nested_seed_counter: u64,
    native_cost: NativeCircuitCost,
    two_qubit_operation_count: usize,
    operation_count: usize,
}

impl TrialOutput {
    fn new(seed: u64, emit_operations: bool) -> Self {
        Self {
            nested_seed_counter: seed,
            emit_operations,
            ..Self::default()
        }
    }

    fn push_operation(
        &mut self,
        operation: Operation,
        source_order: Option<usize>,
        target: &RoutingTarget,
    ) -> Result<(), CompilerError> {
        debug_assert!(self.emit_operations);
        self.two_qubit_operation_count = self
            .two_qubit_operation_count
            .saturating_add(two_qubit_operation_count(std::slice::from_ref(&operation)));
        self.operation_count = self
            .operation_count
            .saturating_add(operation_count(std::slice::from_ref(&operation)));
        if target.native_cost_enabled {
            self.native_cost
                .append_sequence(native_plan_cost_for_operations(
                    std::slice::from_ref(&operation),
                    target,
                )?);
        }
        self.plan.push_operation(operation, source_order);
        Ok(())
    }

    fn push_mapped_operation(
        &mut self,
        operation: &Arc<Operation>,
        source_order: Option<usize>,
        layout: &DenseRoutingLayout,
        target: &RoutingTarget,
    ) -> Result<(), CompilerError> {
        debug_assert!(self.emit_operations);
        if target.native_cost_enabled {
            let mapped = map_operation_dense(operation, layout, target)?;
            return self.push_operation(mapped, source_order, target);
        }

        let qubits = map_operation_qubits_dense(operation, layout, target)?;
        self.operation_count = self.operation_count.saturating_add(1);
        if qubits.len() == 2 {
            self.two_qubit_operation_count = self.two_qubit_operation_count.saturating_add(1);
        }
        self.plan
            .push_mapped_operation(Arc::clone(operation), qubits, source_order);
        Ok(())
    }

    fn apply_pending_swaps(
        &mut self,
        target: &RoutingTarget,
        swaps: Option<&mut Vec<[PhysicalQubit; 2]>>,
    ) -> Result<(), CompilerError> {
        if let Some(swaps) = swaps {
            self.swap_count += swaps.len();
            if self.emit_operations {
                for swap in swaps.drain(..) {
                    self.plan.push_swap(swap);
                    self.operation_count = self.operation_count.saturating_add(1);
                    self.two_qubit_operation_count =
                        self.two_qubit_operation_count.saturating_add(1);
                    if target.native_cost_enabled {
                        self.native_cost.append_gate(target.swap_cost(swap)?);
                    }
                }
            } else {
                swaps.clear();
            }
        }
        Ok(())
    }

    fn next_nested_seed(&mut self) -> u64 {
        let seed = self.nested_seed_counter;
        self.nested_seed_counter = self.nested_seed_counter.wrapping_add(1);
        seed
    }

    fn merge_nested(&mut self, nested: &UnscoredTrial) {
        self.swap_count += nested.swap_count;
        self.fallback_count += nested.fallback_count;
        self.control_flow_blocks_routed += nested.control_flow_blocks_routed + 1;
        self.lazy_pair_l1_lookup_count = self
            .lazy_pair_l1_lookup_count
            .saturating_add(nested.lazy_pair_l1_lookup_count);
        self.lazy_pair_l1_hit_count = self
            .lazy_pair_l1_hit_count
            .saturating_add(nested.lazy_pair_l1_hit_count);
        self.lazy_pair_l1_cached_count = self
            .lazy_pair_l1_cached_count
            .saturating_add(nested.lazy_pair_l1_cached_count);
    }
}

/// Routes a control-flow body and restores its entry layout before exit.
///
/// A control-flow body must be layout-neutral from the parent router's point of
/// view: whichever branch or iteration executes, the body exits with the same
/// logical-to-physical mapping it entered with.
fn route_control_flow_body(
    sabre: &SabreDag,
    target: &RoutingTarget,
    metadata: &PreparedRouteMetadata,
    entry_layout: &Layout,
    heuristic: &SabreHeuristicConfig,
    seed: u64,
    emit_operations: bool,
) -> Result<UnscoredTrial, CompilerError> {
    let mut result = route_trial_with_metadata(
        sabre,
        target,
        metadata,
        entry_layout,
        heuristic,
        seed,
        emit_operations,
    )?;
    let epilogue_swaps = restore_layout_swaps(target, &result.final_layout, entry_layout, seed)?;
    let mut layout = result.final_layout.clone();
    for swap in &epilogue_swaps {
        layout.swap_physical(swap[0], swap[1]).map_err(|error| {
            CompilerError::InvariantViolation(format!(
                "sabre control-flow epilogue generated an invalid SWAP: {error}"
            ))
        })?;
    }
    let ends_with_control_transfer = emit_operations
        && matches!(
            result
                .plan
                .last_operation()
                .map(|operation| &operation.instruction),
            Some(Instruction::ClassicalControl(
                ClassicalControlOp::Break | ClassicalControlOp::Continue
            ))
        );
    let control_transfer = if ends_with_control_transfer {
        result.plan.pop_last_operation()
    } else {
        None
    };
    if emit_operations {
        result.append_swaps(target, epilogue_swaps.iter().copied())?;
        if let Some(control_transfer) = control_transfer {
            result.plan.push_operation(control_transfer, None);
        }
    } else {
        result.swap_count = result.swap_count.saturating_add(epilogue_swaps.len());
    }
    result.final_layout = layout;
    if result.final_layout.l2p_map() != entry_layout.l2p_map() {
        return Err(CompilerError::InvariantViolation(
            "sabre control-flow epilogue did not restore the entry layout".to_string(),
        ));
    }
    Ok(result)
}

fn missing_nested_metadata(body: &str) -> CompilerError {
    CompilerError::InvariantViolation(format!(
        "control-flow {body} body has no prepared routing metadata"
    ))
}

/// Computes SWAPs that restore one layout to another on the target graph.
///
/// Token swapping computes an epilogue that returns every live logical qubit to
/// its entry physical location. Vacant physical qubits are irrelevant and are
/// omitted from the token mapping.
fn restore_layout_swaps(
    target: &RoutingTarget,
    current: &Layout,
    desired: &Layout,
    seed: u64,
) -> Result<Vec<[PhysicalQubit; 2]>, CompilerError> {
    let mut mapping_entries = Vec::new();
    for physical in desired.physical_qubits() {
        let Some(logical) = current.get_logical(physical) else {
            continue;
        };
        let desired_physical = desired.get_physical(logical).ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "desired control-flow layout does not map logical qubit {logical}"
            ))
        })?;
        let current_node = target.graph_index.get(&physical).copied().ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "current control-flow layout contains physical qubit {physical} outside the routing graph"
            ))
        })?;
        let desired_node = target
            .graph_index
            .get(&desired_physical)
            .copied()
            .ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "desired control-flow layout contains physical qubit {desired_physical} outside the routing graph"
                ))
            })?;
        mapping_entries.push((current_node, desired_node));
    }
    let mapping = mapping_entries.into_iter().collect();

    let swaps = token_swapper(
        &target.graph,
        mapping,
        Some(CONTROL_FLOW_EPILOGUE_TRIALS),
        Some(seed),
        None,
    )
    .map_err(|error| CompilerError::TransformFailed {
        name: "sabre_route",
        reason: format!("failed to restore control-flow layout: {error}"),
    })?;
    swaps
        .into_iter()
        .map(|(left, right)| {
            Ok([
                target.physical_by_index[left.index()],
                target.physical_by_index[right.index()],
            ])
        })
        .collect()
}

fn map_operation_dense(
    operation: &Operation,
    layout: &DenseRoutingLayout,
    target: &RoutingTarget,
) -> Result<Operation, CompilerError> {
    Ok(Operation {
        instruction: operation.instruction.clone(),
        qubits: map_operation_qubits_dense(operation, layout, target)?,
        params: operation.params.clone(),
        label: operation.label.clone(),
    })
}

#[inline]
fn map_operation_qubits_dense(
    operation: &Operation,
    layout: &DenseRoutingLayout,
    target: &RoutingTarget,
) -> Result<SmallVec<[Qubit; 3]>, CompilerError> {
    operation
        .qubits
        .iter()
        .copied()
        .map(|qubit| {
            let physical_index = layout.physical_index(LogicalQubit::from_qubit(qubit))?;
            target.physical_at(physical_index).map(PhysicalQubit::qubit)
        })
        .collect()
}

/// Remaps operation parameter indices into the routed circuit's parameter table.
///
/// Parameter indices are scoped to a circuit table, while routed nested
/// operations are rebuilt before the final table exists. This function walks
/// recursively so every body points at the reordered routed table.
fn remap_parameter_indices(
    mut operation: Operation,
    parameter_indices: &[u32],
) -> Result<Operation, CompilerError> {
    for param in &mut operation.params {
        if let CircuitParam::Index(index) = param {
            *index = *parameter_indices
                .get(*index as usize)
                .ok_or(crate::circuit::CircuitError::InvalidParameterIndex(*index))?;
        }
    }
    operation.instruction = match &operation.instruction {
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            let then_body = op
                .then_body()
                .operations()
                .iter()
                .cloned()
                .map(|operation| remap_parameter_indices(operation, parameter_indices))
                .collect::<Result<Vec<_>, _>>()?;
            let else_body = op
                .else_body()
                .map(|body| {
                    body.operations()
                        .iter()
                        .cloned()
                        .map(|operation| remap_parameter_indices(operation, parameter_indices))
                        .collect::<Result<Vec<_>, _>>()
                        .map(ControlBody::new)
                })
                .transpose()?;
            Instruction::ClassicalControl(ClassicalControlOp::If(IfOp::new(
                op.condition().clone(),
                ControlBody::new(then_body),
                else_body,
            )?))
        }
        Instruction::ClassicalControl(ClassicalControlOp::While(op)) => {
            let body = op
                .body()
                .operations()
                .iter()
                .cloned()
                .map(|operation| remap_parameter_indices(operation, parameter_indices))
                .collect::<Result<Vec<_>, _>>()?;
            Instruction::ClassicalControl(ClassicalControlOp::While(WhileOp::new(
                op.condition().clone(),
                ControlBody::new(body),
            )?))
        }
        Instruction::ClassicalControl(ClassicalControlOp::For(op)) => {
            let body = op
                .body()
                .operations()
                .iter()
                .cloned()
                .map(|operation| remap_parameter_indices(operation, parameter_indices))
                .collect::<Result<Vec<_>, _>>()?;
            Instruction::ClassicalControl(ClassicalControlOp::For(ForOp::new(
                op.var(),
                op.start().clone(),
                op.stop().clone(),
                op.step().clone(),
                ControlBody::new(body),
            )?))
        }
        Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) => {
            let cases = op
                .cases()
                .iter()
                .map(|case| {
                    let body = case
                        .body()
                        .operations()
                        .iter()
                        .cloned()
                        .map(|operation| remap_parameter_indices(operation, parameter_indices))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(SwitchCase::new(case.value(), ControlBody::new(body)))
                })
                .collect::<Result<Vec<_>, CompilerError>>()?;
            let default = op
                .default()
                .map(|body| {
                    body.operations()
                        .iter()
                        .cloned()
                        .map(|operation| remap_parameter_indices(operation, parameter_indices))
                        .collect::<Result<Vec<_>, _>>()
                        .map(ControlBody::new)
                })
                .transpose()?;
            Instruction::ClassicalControl(ClassicalControlOp::Switch(SwitchOp::new(
                op.target().clone(),
                cases,
                default,
            )?))
        }
        _ => operation.instruction.clone(),
    };
    Ok(operation)
}

fn physical_for(layout: &Layout, logical: LogicalQubit) -> Result<PhysicalQubit, CompilerError> {
    layout.get_physical(logical).ok_or_else(|| {
        CompilerError::InvariantViolation(format!(
            "sabre layout does not map logical qubit {logical}"
        ))
    })
}

/// One exact movement-component assignment for all logical qubits.
#[derive(Debug, Clone)]
pub(crate) struct MovementComponentAssignment {
    pub(crate) components: Vec<Vec<PhysicalQubit>>,
    /// Component index in the same order as the requested logical-qubit list.
    pub(crate) logical_components: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(crate) enum ComponentAssignmentSearch {
    Found(MovementComponentAssignment),
    ProvenInfeasible,
    BudgetExhausted { expansions: usize },
}

#[derive(Debug)]
struct PairComponentConstraint {
    left: usize,
    right: usize,
    allowed: BTreeSet<(usize, usize)>,
}

#[derive(Debug, Clone)]
enum RequirementComponentTerminals {
    Unary(BTreeSet<usize>),
    Pair(BTreeSet<(usize, usize)>),
}

/// Solves component-level placement constraints induced by unary and ordered
/// pair requirements. The result distinguishes a proof of infeasibility from
/// exhaustion of the caller-provided expansion budget.
pub(crate) fn movement_component_assignment(
    sabre: &SabreDag,
    target: &RoutingTarget,
    logical_qubits: &[LogicalQubit],
    expansion_budget: usize,
) -> Result<ComponentAssignmentSearch, CompilerError> {
    let component_indices = target.movement_component_indices();
    let components = component_indices
        .iter()
        .map(|component| {
            component
                .iter()
                .map(|index| target.physical_qubits[*index])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut component_by_physical = vec![0usize; target.physical_qubits.len()];
    for (component, physicals) in component_indices.iter().enumerate() {
        for &physical in physicals {
            component_by_physical[physical] = component;
        }
    }
    let logical_index = logical_qubits
        .iter()
        .copied()
        .enumerate()
        .map(|(index, logical)| (logical, index))
        .collect::<BTreeMap<_, _>>();
    let all_components = (0..components.len()).collect::<BTreeSet<_>>();
    let mut domains = vec![all_components; logical_qubits.len()];
    let mut pair_constraints = Vec::new();
    let mut terminal_components = vec![None; target.requirements.len()];
    collect_component_constraints(
        sabre,
        target,
        &component_by_physical,
        &mut terminal_components,
        &logical_index,
        &mut domains,
        &mut pair_constraints,
    )?;
    if domains.iter().any(BTreeSet::is_empty) {
        return Ok(ComponentAssignmentSearch::ProvenInfeasible);
    }

    let capacities = components.iter().map(Vec::len).collect::<Vec<_>>();
    let mut remaining = capacities;
    let mut assignment = vec![None; logical_qubits.len()];
    let mut budget = expansion_budget;
    let result = assign_logical_components(
        &domains,
        &pair_constraints,
        &mut remaining,
        &mut assignment,
        &mut budget,
    );
    match result {
        ComponentSearchProgress::Found => {
            let logical_components = assignment
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "component search reported success with an incomplete assignment"
                            .to_string(),
                    )
                })?;
            Ok(ComponentAssignmentSearch::Found(
                MovementComponentAssignment {
                    components,
                    logical_components,
                },
            ))
        }
        ComponentSearchProgress::ProvenInfeasible => {
            Ok(ComponentAssignmentSearch::ProvenInfeasible)
        }
        ComponentSearchProgress::BudgetExhausted => {
            Ok(ComponentAssignmentSearch::BudgetExhausted {
                expansions: expansion_budget - budget,
            })
        }
    }
}

impl RoutingTarget {
    fn movement_component_indices(&self) -> Vec<Vec<usize>> {
        let mut unseen = (0..self.physical_qubits.len()).collect::<BTreeSet<_>>();
        let mut components = Vec::new();
        while let Some(start) = unseen.pop_first() {
            let mut queue = VecDeque::from([start]);
            let mut component = Vec::new();
            while let Some(physical) = queue.pop_front() {
                component.push(physical);
                for neighbor in &self.neighbors_by_index[physical] {
                    if unseen.remove(&neighbor.index) {
                        queue.push_back(neighbor.index);
                    }
                }
            }
            component.sort_unstable_by_key(|index| self.physical_qubits[*index]);
            components.push(component);
        }
        components.sort_unstable_by_key(|component| self.physical_qubits[component[0]]);
        components
    }
}

fn collect_component_constraints(
    sabre: &SabreDag,
    target: &RoutingTarget,
    component_by_physical: &[usize],
    terminal_components: &mut [Option<RequirementComponentTerminals>],
    logical_index: &BTreeMap<LogicalQubit, usize>,
    domains: &mut [BTreeSet<usize>],
    pairs: &mut Vec<PairComponentConstraint>,
) -> Result<(), CompilerError> {
    for node_index in sabre.graph.node_indices() {
        let node = &sabre.graph[node_index];
        match &node.kind {
            SabreNodeKind::Unary(logical) => {
                let index = *logical_index.get(logical).ok_or_else(|| {
                    CompilerError::InvariantViolation(format!(
                        "unary component constraint references unknown logical qubit {logical}"
                    ))
                })?;
                let requirement = target.interaction_id_for_node(sabre, node_index)?;
                let RequirementComponentTerminals::Unary(allowed) =
                    requirement_component_terminals(
                        target,
                        requirement,
                        component_by_physical,
                        terminal_components,
                    )?
                else {
                    return Err(CompilerError::InvariantViolation(format!(
                        "unary node uses pair routing requirement {requirement}"
                    )));
                };
                domains[index].retain(|component| allowed.contains(component));
            }
            SabreNodeKind::TwoQ(logicals) => {
                let left = *logical_index.get(&logicals[0]).ok_or_else(|| {
                    CompilerError::InvariantViolation(format!(
                        "pair component constraint references unknown logical qubit {}",
                        logicals[0]
                    ))
                })?;
                let right = *logical_index.get(&logicals[1]).ok_or_else(|| {
                    CompilerError::InvariantViolation(format!(
                        "pair component constraint references unknown logical qubit {}",
                        logicals[1]
                    ))
                })?;
                let requirement = target.interaction_id_for_node(sabre, node_index)?;
                let RequirementComponentTerminals::Pair(allowed) = requirement_component_terminals(
                    target,
                    requirement,
                    component_by_physical,
                    terminal_components,
                )?
                else {
                    return Err(CompilerError::InvariantViolation(format!(
                        "pair node uses unary routing requirement {requirement}"
                    )));
                };
                if allowed.is_empty() {
                    domains[left].clear();
                    domains[right].clear();
                } else {
                    domains[left].retain(|component| {
                        allowed.iter().any(|(allowed, _)| allowed == component)
                    });
                    domains[right].retain(|component| {
                        allowed.iter().any(|(_, allowed)| allowed == component)
                    });
                    pairs.push(PairComponentConstraint {
                        left,
                        right,
                        allowed: allowed.clone(),
                    });
                }
            }
            SabreNodeKind::ControlFlow(SabreControlFlow::If {
                then_body,
                else_body,
                ..
            }) => {
                collect_component_constraints(
                    then_body,
                    target,
                    component_by_physical,
                    terminal_components,
                    logical_index,
                    domains,
                    pairs,
                )?;
                if let Some(else_body) = else_body {
                    collect_component_constraints(
                        else_body,
                        target,
                        component_by_physical,
                        terminal_components,
                        logical_index,
                        domains,
                        pairs,
                    )?;
                }
            }
            SabreNodeKind::ControlFlow(
                SabreControlFlow::While { body, .. } | SabreControlFlow::For { body, .. },
            ) => collect_component_constraints(
                body,
                target,
                component_by_physical,
                terminal_components,
                logical_index,
                domains,
                pairs,
            )?,
            SabreNodeKind::ControlFlow(SabreControlFlow::Switch { cases, default, .. }) => {
                for case in cases {
                    collect_component_constraints(
                        &case.body,
                        target,
                        component_by_physical,
                        terminal_components,
                        logical_index,
                        domains,
                        pairs,
                    )?;
                }
                if let Some(default) = default {
                    collect_component_constraints(
                        default,
                        target,
                        component_by_physical,
                        terminal_components,
                        logical_index,
                        domains,
                        pairs,
                    )?;
                }
            }
            SabreNodeKind::Synchronize => {}
        }
    }
    Ok(())
}

fn requirement_component_terminals<'a>(
    target: &RoutingTarget,
    requirement: usize,
    component_by_physical: &[usize],
    cache: &'a mut [Option<RequirementComponentTerminals>],
) -> Result<&'a RequirementComponentTerminals, CompilerError> {
    let entry = cache.get_mut(requirement).ok_or_else(|| {
        CompilerError::InvariantViolation(format!(
            "routing requirement {requirement} has no component cache entry"
        ))
    })?;
    if entry.is_none() {
        *entry = Some(match target.requirements.get(requirement) {
            Some(RequirementTable::Unary { terminals, .. }) => {
                RequirementComponentTerminals::Unary(
                    terminals
                        .iter()
                        .enumerate()
                        .filter_map(|(physical, terminal)| {
                            terminal.map(|_| component_by_physical[physical])
                        })
                        .collect(),
                )
            }
            Some(RequirementTable::Pair { terminals, .. }) => RequirementComponentTerminals::Pair(
                terminals
                    .keys()
                    .map(|[left, right]| {
                        (component_by_physical[*left], component_by_physical[*right])
                    })
                    .collect(),
            ),
            None => {
                return Err(CompilerError::InvariantViolation(format!(
                    "routing requirement {requirement} is missing"
                )));
            }
        });
    }
    entry.as_ref().ok_or_else(|| {
        CompilerError::InvariantViolation(format!(
            "routing requirement {requirement} component terminals were not initialized"
        ))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComponentSearchProgress {
    Found,
    ProvenInfeasible,
    BudgetExhausted,
}

fn assign_logical_components(
    domains: &[BTreeSet<usize>],
    constraints: &[PairComponentConstraint],
    remaining: &mut [usize],
    assignment: &mut [Option<usize>],
    budget: &mut usize,
) -> ComponentSearchProgress {
    if assignment.iter().all(Option::is_some) {
        return ComponentSearchProgress::Found;
    }
    let Some((logical, candidates)) = assignment
        .iter()
        .enumerate()
        .filter(|(_, assigned)| assigned.is_none())
        .map(|(logical, _)| {
            let candidates = domains[logical]
                .iter()
                .copied()
                .filter(|component| remaining[*component] > 0)
                .filter(|component| {
                    component_assignment_is_consistent(logical, *component, constraints, assignment)
                })
                .collect::<Vec<_>>();
            (logical, candidates)
        })
        .min_by_key(|(logical, candidates)| (candidates.len(), *logical))
    else {
        return ComponentSearchProgress::ProvenInfeasible;
    };
    if candidates.is_empty() {
        return ComponentSearchProgress::ProvenInfeasible;
    }
    for component in candidates {
        if *budget == 0 {
            return ComponentSearchProgress::BudgetExhausted;
        }
        *budget -= 1;
        assignment[logical] = Some(component);
        remaining[component] -= 1;
        let forward_consistent = assignment
            .iter()
            .enumerate()
            .filter(|(_, assigned)| assigned.is_none())
            .all(|(unassigned, _)| {
                domains[unassigned].iter().copied().any(|candidate| {
                    remaining[candidate] > 0
                        && component_assignment_is_consistent(
                            unassigned,
                            candidate,
                            constraints,
                            assignment,
                        )
                })
            });
        if forward_consistent {
            match assign_logical_components(domains, constraints, remaining, assignment, budget) {
                ComponentSearchProgress::Found => return ComponentSearchProgress::Found,
                ComponentSearchProgress::BudgetExhausted => {
                    remaining[component] += 1;
                    assignment[logical] = None;
                    return ComponentSearchProgress::BudgetExhausted;
                }
                ComponentSearchProgress::ProvenInfeasible => {}
            }
        }
        remaining[component] += 1;
        assignment[logical] = None;
    }
    ComponentSearchProgress::ProvenInfeasible
}

fn component_assignment_is_consistent(
    logical: usize,
    component: usize,
    constraints: &[PairComponentConstraint],
    assignment: &[Option<usize>],
) -> bool {
    constraints.iter().all(|constraint| {
        if constraint.left == logical {
            assignment[constraint.right]
                .is_none_or(|right| constraint.allowed.contains(&(component, right)))
        } else if constraint.right == logical {
            assignment[constraint.left]
                .is_none_or(|left| constraint.allowed.contains(&(left, component)))
        } else {
            true
        }
    })
}

/// Validates that every interaction in a SABRE DAG is physically reachable.
///
/// Reachability is recursive because control-flow bodies are routed with the
/// same physical topology and entry-layout contract as parent operations.
pub(crate) fn validate_reachable_interactions_for_target(
    sabre: &SabreDag,
    target: &RoutingTarget,
    layout: &Layout,
) -> Result<(), CompilerError> {
    match interaction_reachability_for_target(sabre, target, layout)? {
        InteractionReachability::Reachable => Ok(()),
        InteractionReachability::UnreachableUnary {
            logical,
            physical: _,
            cause: RequirementReachabilityFailure::NoExecutableTerminal,
        } => Err(CompilerError::SabreRoutingFailed(
            SabreRoutingFailure::NoExecutableUnaryTerminal { logical },
        )),
        InteractionReachability::UnreachableUnary {
            logical,
            physical,
            cause: RequirementReachabilityFailure::MovementDisconnected,
        } => Err(CompilerError::SabreRoutingFailed(
            SabreRoutingFailure::UnreachableUnaryPlacement { logical, physical },
        )),
        InteractionReachability::UnreachablePair {
            logical,
            physical: _,
            cause: RequirementReachabilityFailure::NoExecutableTerminal,
        } => Err(CompilerError::SabreRoutingFailed(
            SabreRoutingFailure::NoExecutablePairTerminal { logical },
        )),
        InteractionReachability::UnreachablePair {
            logical,
            physical,
            cause: RequirementReachabilityFailure::MovementDisconnected,
        } => Err(CompilerError::SabreRoutingFailed(
            SabreRoutingFailure::UnreachablePairPlacement { logical, physical },
        )),
    }
}

pub(crate) fn validate_reachable_interactions_for_target_with_metadata(
    sabre: &SabreDag,
    target: &RoutingTarget,
    metadata: &PreparedRouteMetadata,
    layout: &Layout,
) -> Result<(), CompilerError> {
    match interaction_reachability_for_target_with_metadata(sabre, target, metadata, layout)? {
        InteractionReachability::Reachable => Ok(()),
        InteractionReachability::UnreachableUnary {
            logical,
            cause: RequirementReachabilityFailure::NoExecutableTerminal,
            ..
        } => Err(CompilerError::SabreRoutingFailed(
            SabreRoutingFailure::NoExecutableUnaryTerminal { logical },
        )),
        InteractionReachability::UnreachableUnary {
            logical,
            physical,
            cause: RequirementReachabilityFailure::MovementDisconnected,
        } => Err(CompilerError::SabreRoutingFailed(
            SabreRoutingFailure::UnreachableUnaryPlacement { logical, physical },
        )),
        InteractionReachability::UnreachablePair {
            logical,
            cause: RequirementReachabilityFailure::NoExecutableTerminal,
            ..
        } => Err(CompilerError::SabreRoutingFailed(
            SabreRoutingFailure::NoExecutablePairTerminal { logical },
        )),
        InteractionReachability::UnreachablePair {
            logical,
            physical,
            cause: RequirementReachabilityFailure::MovementDisconnected,
        } => Err(CompilerError::SabreRoutingFailed(
            SabreRoutingFailure::UnreachablePairPlacement { logical, physical },
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InteractionReachability {
    Reachable,
    UnreachableUnary {
        logical: LogicalQubit,
        physical: PhysicalQubit,
        cause: RequirementReachabilityFailure,
    },
    UnreachablePair {
        logical: [LogicalQubit; 2],
        physical: [PhysicalQubit; 2],
        cause: RequirementReachabilityFailure,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequirementReachabilityFailure {
    NoExecutableTerminal,
    MovementDisconnected,
}

pub(crate) fn interaction_reachability_for_target(
    sabre: &SabreDag,
    target: &RoutingTarget,
    layout: &Layout,
) -> Result<InteractionReachability, CompilerError> {
    let metadata = PreparedRouteMetadata::new(sabre, target)?;
    interaction_reachability_for_target_with_metadata(sabre, target, &metadata, layout)
}

pub(crate) fn interaction_reachability_for_target_with_metadata(
    sabre: &SabreDag,
    target: &RoutingTarget,
    metadata: &PreparedRouteMetadata,
    layout: &Layout,
) -> Result<InteractionReachability, CompilerError> {
    for node_index in sabre.graph.node_indices() {
        let node = &sabre.graph[node_index];
        let requirement = || {
            metadata
                .requirement_ids
                .get(node_index.index())
                .copied()
                .flatten()
                .ok_or_else(|| {
                    CompilerError::InvariantViolation(format!(
                        "reachability node {} has no prepared requirement id",
                        node_index.index()
                    ))
                })
        };
        let mut nested = metadata
            .nested
            .get(node_index.index())
            .ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "reachability node {} has no prepared nested metadata slot",
                    node_index.index()
                ))
            })?
            .iter();
        match &node.kind {
            SabreNodeKind::Unary(logical) => {
                let physical = physical_for(layout, *logical)?;
                let physical_index = target.physical_index(physical)?;
                let requirement = requirement()?;
                if target
                    .distance_steps_for(requirement, RequirementPlacement::Unary(physical_index))
                    .is_none()
                {
                    return Ok(InteractionReachability::UnreachableUnary {
                        logical: *logical,
                        physical,
                        cause: if target.has_terminal(requirement) {
                            RequirementReachabilityFailure::MovementDisconnected
                        } else {
                            RequirementReachabilityFailure::NoExecutableTerminal
                        },
                    });
                }
            }
            SabreNodeKind::TwoQ(pair) => {
                let left = physical_for(layout, pair[0])?;
                let right = physical_for(layout, pair[1])?;
                let left_index = target.physical_index(left)?;
                let right_index = target.physical_index(right)?;
                let interaction = requirement()?;
                if target
                    .distance_steps_for(
                        interaction,
                        RequirementPlacement::Pair([left_index, right_index]),
                    )
                    .is_none()
                {
                    return Ok(InteractionReachability::UnreachablePair {
                        logical: *pair,
                        physical: [left, right],
                        cause: if target.has_terminal(interaction) {
                            RequirementReachabilityFailure::MovementDisconnected
                        } else {
                            RequirementReachabilityFailure::NoExecutableTerminal
                        },
                    });
                }
            }
            SabreNodeKind::ControlFlow(SabreControlFlow::If {
                then_body,
                else_body,
                ..
            }) => {
                let reachable = interaction_reachability_for_target_with_metadata(
                    then_body,
                    target,
                    nested
                        .next()
                        .ok_or_else(|| missing_nested_metadata("if then"))?,
                    layout,
                )?;
                if reachable != InteractionReachability::Reachable {
                    return Ok(reachable);
                }
                if let Some(else_body) = else_body {
                    let reachable = interaction_reachability_for_target_with_metadata(
                        else_body,
                        target,
                        nested
                            .next()
                            .ok_or_else(|| missing_nested_metadata("if else"))?,
                        layout,
                    )?;
                    if reachable != InteractionReachability::Reachable {
                        return Ok(reachable);
                    }
                }
            }
            SabreNodeKind::ControlFlow(
                SabreControlFlow::While { body, .. } | SabreControlFlow::For { body, .. },
            ) => {
                let reachable = interaction_reachability_for_target_with_metadata(
                    body,
                    target,
                    nested
                        .next()
                        .ok_or_else(|| missing_nested_metadata("loop"))?,
                    layout,
                )?;
                if reachable != InteractionReachability::Reachable {
                    return Ok(reachable);
                }
            }
            SabreNodeKind::ControlFlow(SabreControlFlow::Switch { cases, default, .. }) => {
                for case in cases {
                    let reachable = interaction_reachability_for_target_with_metadata(
                        &case.body,
                        target,
                        nested
                            .next()
                            .ok_or_else(|| missing_nested_metadata("switch case"))?,
                        layout,
                    )?;
                    if reachable != InteractionReachability::Reachable {
                        return Ok(reachable);
                    }
                }
                if let Some(default) = default {
                    let reachable = interaction_reachability_for_target_with_metadata(
                        default,
                        target,
                        nested
                            .next()
                            .ok_or_else(|| missing_nested_metadata("switch default"))?,
                        layout,
                    )?;
                    if reachable != InteractionReachability::Reachable {
                        return Ok(reachable);
                    }
                }
            }
            SabreNodeKind::Synchronize => {}
        }
    }
    Ok(InteractionReachability::Reachable)
}

fn swap_operation(swap: [PhysicalQubit; 2]) -> Operation {
    Operation {
        instruction: Instruction::Standard(StandardGate::SWAP),
        qubits: smallvec![swap[0].qubit(), swap[1].qubit()],
        params: SmallVec::new(),
        label: None,
    }
}

pub(crate) fn compare_ranked_trials(
    left: &mut RankedTrial,
    left_index: (usize, usize),
    right: &mut RankedTrial,
    right_index: (usize, usize),
    target: &RoutingTarget,
) -> Result<Ordering, CompilerError> {
    let count_order = left.native_two_qubit_ops.cmp(&right.native_two_qubit_ops);
    if !count_order.is_eq() {
        return Ok(count_order);
    }

    let depth_order = left
        .ensure_native_two_qubit_depth(target)?
        .cmp(&right.ensure_native_two_qubit_depth(target)?);
    if !depth_order.is_eq() {
        return Ok(depth_order);
    }

    Ok(left
        .ensure_native_total_depth(target)?
        .cmp(&right.ensure_native_total_depth(target)?)
        .then_with(|| left_index.cmp(&right_index)))
}

fn native_cost_for_routed_operation(
    operation: &Operation,
    target: &RoutingTarget,
) -> Result<NativePlanCost, CompilerError> {
    let state = DeviceGateState::from_instruction(
        &operation.instruction,
        operation
            .qubits
            .iter()
            .copied()
            .map(PhysicalQubit::from_qubit)
            .collect(),
    )
    .ok_or_else(|| {
        CompilerError::InvariantViolation(format!(
            "missing native-cost key for routed operation {}",
            operation.instruction
        ))
    })?;
    if let Some(cost) = target.native_cost(&state) {
        return Ok(cost);
    }
    if let Some(failure) = target.unsupported_native_plan(&state) {
        return Err(CompilerError::DeviceLoweringFailed(failure.clone()));
    }
    Err(CompilerError::InvariantViolation(format!(
        "routed operation {} on {:?} was not prepared in the native plan catalog",
        operation.instruction, state.ordered_qargs
    )))
}

#[derive(Debug, Clone, Copy, Default)]
struct NativeCircuitCost {
    static_native: Option<NativePlanCost>,
    path: ExecutionPathCost,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct ExecutionPathCost {
    native_two_qubit_ops: u32,
    native_total_ops: u32,
    unknown_loop_count: usize,
}

impl NativeCircuitCost {
    fn append_gate(&mut self, cost: NativePlanCost) {
        self.static_native = Some(match self.static_native {
            Some(current) => current.combine(cost),
            None => cost,
        });
        self.path.append_gate(cost);
    }

    fn append_sequence(&mut self, other: Self) {
        self.static_native = combine_optional_native_cost(self.static_native, other.static_native);
        self.path.append_sequence(other.path);
    }

    fn add_static_branch(&mut self, branch: Self) {
        self.static_native = combine_optional_native_cost(self.static_native, branch.static_native);
    }

    fn append_worst_branch(&mut self, left: Self, right: Self) {
        self.path.append_sequence(left.path.worse(right.path));
    }
}

impl ExecutionPathCost {
    fn append_gate(&mut self, cost: NativePlanCost) {
        self.native_two_qubit_ops = self
            .native_two_qubit_ops
            .saturating_add(cost.native_two_qubit_ops);
        self.native_total_ops = self.native_total_ops.saturating_add(cost.native_total_ops);
    }

    fn append_sequence(&mut self, other: Self) {
        self.native_two_qubit_ops = self
            .native_two_qubit_ops
            .saturating_add(other.native_two_qubit_ops);
        self.native_total_ops = self.native_total_ops.saturating_add(other.native_total_ops);
        self.unknown_loop_count = self
            .unknown_loop_count
            .saturating_add(other.unknown_loop_count);
    }

    fn repeated(mut self, count: u128) -> Self {
        self.native_two_qubit_ops =
            (u128::from(self.native_two_qubit_ops) * count).min(u128::from(u32::MAX)) as u32;
        self.native_total_ops =
            (u128::from(self.native_total_ops) * count).min(u128::from(u32::MAX)) as u32;
        self.unknown_loop_count = (self.unknown_loop_count as u128)
            .saturating_mul(count)
            .min(usize::MAX as u128) as usize;
        self
    }

    fn worse(self, other: Self) -> Self {
        if self.compare(other).is_ge() {
            self
        } else {
            other
        }
    }

    fn compare(self, other: Self) -> Ordering {
        self.unknown_loop_count
            .cmp(&other.unknown_loop_count)
            .then_with(|| self.native_two_qubit_ops.cmp(&other.native_two_qubit_ops))
            .then_with(|| self.native_total_ops.cmp(&other.native_total_ops))
    }
}

fn combine_optional_native_cost(
    left: Option<NativePlanCost>,
    right: Option<NativePlanCost>,
) -> Option<NativePlanCost> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.combine(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn native_plan_cost_for_operations(
    operations: &[Operation],
    target: &RoutingTarget,
) -> Result<NativeCircuitCost, CompilerError> {
    let mut total = NativeCircuitCost::default();
    for operation in operations {
        match &operation.instruction {
            Instruction::Standard(StandardGate::GPhase) => {}
            Instruction::Standard(_) | Instruction::McGate(_) => {
                let cost = native_cost_for_routed_operation(operation, target)?;
                total.append_gate(cost);
            }
            Instruction::ClassicalControl(control) => match control {
                ClassicalControlOp::If(op) => {
                    let then_cost =
                        native_plan_cost_for_operations(op.then_body().operations(), target)?;
                    let else_cost = op
                        .else_body()
                        .map(|body| native_plan_cost_for_operations(body.operations(), target))
                        .transpose()?
                        .unwrap_or_default();
                    total.add_static_branch(then_cost);
                    total.add_static_branch(else_cost);
                    total.append_worst_branch(then_cost, else_cost);
                }
                ClassicalControlOp::While(op) => {
                    let mut body = native_plan_cost_for_operations(op.body().operations(), target)?;
                    body.path.unknown_loop_count = body.path.unknown_loop_count.saturating_add(1);
                    total.append_sequence(body);
                }
                ClassicalControlOp::For(op) => {
                    let mut body = native_plan_cost_for_operations(op.body().operations(), target)?;
                    if let Some(iterations) = op.static_iteration_count() {
                        total.static_native = combine_optional_native_cost(
                            total.static_native,
                            repeat_native_cost(body.static_native, iterations),
                        );
                        body.static_native = None;
                        body.path = body.path.repeated(iterations);
                    } else {
                        total.add_static_branch(body);
                        body.static_native = None;
                        body.path.unknown_loop_count =
                            body.path.unknown_loop_count.saturating_add(1);
                    }
                    total.append_sequence(body);
                }
                ClassicalControlOp::Switch(op) => {
                    let mut worst_path = None::<ExecutionPathCost>;
                    for case in op.cases() {
                        let branch =
                            native_plan_cost_for_operations(case.body().operations(), target)?;
                        total.add_static_branch(branch);
                        worst_path = Some(
                            worst_path.map_or(branch.path, |current| current.worse(branch.path)),
                        );
                    }
                    if let Some(body) = op.default() {
                        let branch = native_plan_cost_for_operations(body.operations(), target)?;
                        total.add_static_branch(branch);
                        worst_path = Some(
                            worst_path.map_or(branch.path, |current| current.worse(branch.path)),
                        );
                    }
                    if let Some(worst_path) = worst_path {
                        total.path.append_sequence(worst_path);
                    }
                }
                ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
            },
            Instruction::ClassicalData(_) | Instruction::Directive(_) | Instruction::Delay => {}
            Instruction::UnitaryGate(_) | Instruction::CircuitGate(_) => {
                return Err(CompilerError::InvariantViolation(format!(
                    "unlowerable routed operation {} reached native trial scoring",
                    operation.instruction
                )));
            }
        }
    }
    Ok(total)
}

fn repeat_native_cost(cost: Option<NativePlanCost>, count: u128) -> Option<NativePlanCost> {
    cost.map(|cost| NativePlanCost {
        native_two_qubit_ops: (u128::from(cost.native_two_qubit_ops) * count)
            .min(u128::from(u32::MAX)) as u32,
        native_total_ops: (u128::from(cost.native_total_ops) * count).min(u128::from(u32::MAX))
            as u32,
        ..NativePlanCost::default()
    })
}

fn native_two_qubit_depth(
    operations: &[Operation],
    target: &RoutingTarget,
) -> Result<usize, CompilerError> {
    let mut qubit_depths = BTreeMap::<Qubit, usize>::new();
    let mut max_depth = 0;
    for operation in operations {
        let local_depth = operation_local_native_two_qubit_depth(operation, target)?;
        if local_depth == 0 {
            continue;
        }
        let base = operation
            .qubits
            .iter()
            .map(|qubit| qubit_depths.get(qubit).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);
        let depth = base + local_depth;
        for qubit in &operation.qubits {
            qubit_depths.insert(*qubit, depth);
        }
        max_depth = max_depth.max(depth);
    }
    Ok(max_depth)
}

fn native_total_depth(
    operations: &[Operation],
    target: &RoutingTarget,
) -> Result<usize, CompilerError> {
    let mut qubit_depths = BTreeMap::<Qubit, usize>::new();
    let mut max_depth = 0;
    for operation in operations {
        let local_depth = operation_local_native_total_depth(operation, target)?;
        if local_depth == 0 {
            continue;
        }
        let base = operation
            .qubits
            .iter()
            .map(|qubit| qubit_depths.get(qubit).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);
        let depth = base.saturating_add(local_depth);
        for qubit in &operation.qubits {
            qubit_depths.insert(*qubit, depth);
        }
        max_depth = max_depth.max(depth);
    }
    Ok(max_depth)
}

fn operation_local_native_two_qubit_depth(
    operation: &Operation,
    target: &RoutingTarget,
) -> Result<usize, CompilerError> {
    match &operation.instruction {
        Instruction::Standard(StandardGate::GPhase) => Ok(0),
        Instruction::Standard(_) | Instruction::McGate(_) => {
            let state = DeviceGateState::from_instruction(
                &operation.instruction,
                operation
                    .qubits
                    .iter()
                    .copied()
                    .map(PhysicalQubit::from_qubit)
                    .collect(),
            )
            .ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "missing native-depth key for routed operation {}",
                    operation.instruction
                ))
            })?;
            match target.native_plan(&state) {
                Some(plan) => Ok(structural_plan_depth(plan, true)),
                None => {
                    if let Some(failure) = target.unsupported_native_plan(&state) {
                        return Err(CompilerError::DeviceLoweringFailed(failure.clone()));
                    }
                    Err(CompilerError::InvariantViolation(format!(
                        "routed operation {} on {:?} was not prepared in the native plan catalog",
                        operation.instruction, state.ordered_qargs
                    )))
                }
            }
        }
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            let then_depth = native_two_qubit_depth(op.then_body().operations(), target)?;
            let else_depth = op
                .else_body()
                .map(|body| native_two_qubit_depth(body.operations(), target))
                .transpose()?
                .unwrap_or(0);
            Ok(then_depth.max(else_depth))
        }
        Instruction::ClassicalControl(ClassicalControlOp::While(op)) => {
            native_two_qubit_depth(op.body().operations(), target)
        }
        Instruction::ClassicalControl(ClassicalControlOp::For(op)) => {
            let body_depth = native_two_qubit_depth(op.body().operations(), target)?;
            let iterations = op
                .static_iteration_count()
                .unwrap_or(1)
                .min(usize::MAX as u128) as usize;
            Ok(body_depth.saturating_mul(iterations))
        }
        Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) => {
            let mut depth = 0;
            for case in op.cases() {
                depth = depth.max(native_two_qubit_depth(case.body().operations(), target)?);
            }
            if let Some(body) = op.default() {
                depth = depth.max(native_two_qubit_depth(body.operations(), target)?);
            }
            Ok(depth)
        }
        _ => Ok(0),
    }
}

fn operation_local_native_total_depth(
    operation: &Operation,
    target: &RoutingTarget,
) -> Result<usize, CompilerError> {
    match &operation.instruction {
        Instruction::Standard(StandardGate::GPhase) => Ok(0),
        Instruction::Standard(_) | Instruction::McGate(_) => {
            let state = DeviceGateState::from_instruction(
                &operation.instruction,
                operation
                    .qubits
                    .iter()
                    .copied()
                    .map(PhysicalQubit::from_qubit)
                    .collect(),
            )
            .ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "missing native-total-depth key for routed operation {}",
                    operation.instruction
                ))
            })?;
            match target.native_plan(&state) {
                Some(plan) => Ok(structural_plan_depth(plan, false)),
                None => {
                    if let Some(failure) = target.unsupported_native_plan(&state) {
                        return Err(CompilerError::DeviceLoweringFailed(failure.clone()));
                    }
                    Err(CompilerError::InvariantViolation(format!(
                        "routed operation {} on {:?} was not prepared in the native plan catalog",
                        operation.instruction, state.ordered_qargs
                    )))
                }
            }
        }
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            let then_depth = native_total_depth(op.then_body().operations(), target)?;
            let else_depth = op
                .else_body()
                .map(|body| native_total_depth(body.operations(), target))
                .transpose()?
                .unwrap_or(0);
            Ok(then_depth.max(else_depth))
        }
        Instruction::ClassicalControl(ClassicalControlOp::While(op)) => {
            native_total_depth(op.body().operations(), target)
        }
        Instruction::ClassicalControl(ClassicalControlOp::For(op)) => {
            let body_depth = native_total_depth(op.body().operations(), target)?;
            let iterations = op
                .static_iteration_count()
                .unwrap_or(1)
                .min(usize::MAX as u128) as usize;
            Ok(body_depth.saturating_mul(iterations))
        }
        Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) => {
            let mut depth = 0;
            for case in op.cases() {
                depth = depth.max(native_total_depth(case.body().operations(), target)?);
            }
            if let Some(body) = op.default() {
                depth = depth.max(native_total_depth(body.operations(), target)?);
            }
            Ok(depth)
        }
        Instruction::ClassicalData(_) | Instruction::Directive(_) | Instruction::Delay => Ok(0),
        Instruction::UnitaryGate(_) | Instruction::CircuitGate(_) => {
            Err(CompilerError::InvariantViolation(format!(
                "unlowerable routed operation {} reached native depth scoring",
                operation.instruction
            )))
        }
        Instruction::ClassicalControl(ClassicalControlOp::Break | ClassicalControlOp::Continue) => {
            Ok(0)
        }
    }
}

fn structural_plan_depth(plan: &StructuralNativePlan, two_qubit_only: bool) -> usize {
    let mut qubit_depths = BTreeMap::<PhysicalQubit, usize>::new();
    let mut max_depth = 0;
    for qargs in &plan.leaves {
        if qargs.is_empty() || (two_qubit_only && qargs.len() != 2) {
            continue;
        }
        let depth = qargs
            .iter()
            .map(|qubit| qubit_depths.get(qubit).copied().unwrap_or(0))
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        for qubit in qargs {
            qubit_depths.insert(*qubit, depth);
        }
        max_depth = max_depth.max(depth);
    }
    max_depth
}

fn two_qubit_operation_count(operations: &[Operation]) -> usize {
    operations
        .iter()
        .map(|operation| match &operation.instruction {
            Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
                two_qubit_operation_count(op.then_body().operations())
                    + op.else_body()
                        .map(|body| two_qubit_operation_count(body.operations()))
                        .unwrap_or(0)
            }
            Instruction::ClassicalControl(ClassicalControlOp::While(op)) => {
                two_qubit_operation_count(op.body().operations())
            }
            Instruction::ClassicalControl(ClassicalControlOp::For(op)) => {
                two_qubit_operation_count(op.body().operations())
            }
            Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) => {
                op.cases()
                    .iter()
                    .map(|case| two_qubit_operation_count(case.body().operations()))
                    .sum::<usize>()
                    + op.default()
                        .map(|body| two_qubit_operation_count(body.operations()))
                        .unwrap_or(0)
            }
            _ => usize::from(operation.qubits.len() == 2),
        })
        .sum()
}

/// Estimates ASAP two-qubit depth for trial ranking.
///
/// This is not a full scheduler. Control-flow contributes the maximum local
/// branch or body depth, and parent operations are chained by used qubits.
fn two_qubit_depth(operations: &[Operation]) -> usize {
    let mut qubit_depths = BTreeMap::<Qubit, usize>::new();
    let mut max_depth = 0usize;

    for operation in operations {
        let local_depth = operation_local_two_qubit_depth(operation);
        if local_depth == 0 {
            continue;
        }

        let base = operation
            .qubits
            .iter()
            .map(|qubit| qubit_depths.get(qubit).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);
        let depth = base + local_depth;
        for qubit in &operation.qubits {
            qubit_depths.insert(*qubit, depth);
        }
        max_depth = max_depth.max(depth);
    }

    max_depth
}

fn operation_local_two_qubit_depth(operation: &Operation) -> usize {
    match &operation.instruction {
        Instruction::Standard(StandardGate::SWAP) => 3,
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            let then_depth = two_qubit_depth(op.then_body().operations());
            let else_depth = op
                .else_body()
                .map(|body| two_qubit_depth(body.operations()))
                .unwrap_or(0);
            then_depth.max(else_depth)
        }
        Instruction::ClassicalControl(ClassicalControlOp::While(op)) => {
            two_qubit_depth(op.body().operations())
        }
        Instruction::ClassicalControl(ClassicalControlOp::For(op)) => {
            let iterations = op
                .static_iteration_count()
                .unwrap_or(1)
                .min(usize::MAX as u128) as usize;
            two_qubit_depth(op.body().operations()).saturating_mul(iterations)
        }
        Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) => op
            .cases()
            .iter()
            .map(|case| two_qubit_depth(case.body().operations()))
            .chain(
                op.default()
                    .into_iter()
                    .map(|body| two_qubit_depth(body.operations())),
            )
            .max()
            .unwrap_or(0),
        _ if operation.qubits.len() == 2 => 1,
        _ => 0,
    }
}

fn operation_count(operations: &[Operation]) -> usize {
    operations
        .iter()
        .map(|operation| {
            1 + match &operation.instruction {
                Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
                    operation_count(op.then_body().operations())
                        + op.else_body()
                            .map(|body| operation_count(body.operations()))
                            .unwrap_or(0)
                }
                Instruction::ClassicalControl(ClassicalControlOp::While(op)) => {
                    operation_count(op.body().operations())
                }
                Instruction::ClassicalControl(ClassicalControlOp::For(op)) => {
                    operation_count(op.body().operations())
                }
                Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) => {
                    op.cases()
                        .iter()
                        .map(|case| operation_count(case.body().operations()))
                        .sum::<usize>()
                        + op.default()
                            .map(|body| operation_count(body.operations()))
                            .unwrap_or(0)
                }
                _ => 0,
            }
        })
        .sum()
}

#[cfg(test)]
#[path = "score_test.rs"]
mod score_test;
