// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.
//
// This file is an original modification of Cqlib.

//! Device-aware cost and placement domains for exact two-qubit synthesis.
//!
//! The public two-qubit synthesis target describes a logical gate basis. A
//! device target is different: before layout it changes the set of physical
//! terminals available to routing, while after layout it must be evaluated on
//! one exact ordered pair. This module keeps that distinction internal and
//! evaluates source and synthesized sequences through the same exact-qargs
//! plans later used by device lowering.
//!
//! Exact-physical catalogs are request-scoped rather than device-wide: they
//! prepare the circuit's actual gate states plus the fixed synthesis families.
//! Diagnostic cost APIs therefore distinguish a missing prepared root from a
//! root that was prepared and proven unsupported.

use crate::circuit::{Circuit, Instruction, Qubit, StandardGate, ValueInstruction, ValueOperation};
use crate::compile::CompilerError;
use crate::compile::device_planning::{
    CalibrationEstimator, DeviceGateState, DevicePlanSnapshot, DevicePlanningSession,
    NativePlanAvailability, NativePlanCatalog, NativePlanCost, NativePlanLeaf, NativePlanSummary,
};
use crate::compile::error::DeviceLoweringFailure;
use crate::device::{Device, PhysicalQubit};
use smallvec::smallvec;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DEVICE_SYNTHESIS_CONTEXT_GENERATION: AtomicU64 = AtomicU64::new(1);
const PHYSICAL_SEQUENCE_COST_CACHE_BUDGET: usize = 4096;

fn next_device_synthesis_context_generation() -> u64 {
    let generation = NEXT_DEVICE_SYNTHESIS_CONTEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
    assert_ne!(
        generation, 0,
        "device synthesis context generation exhausted"
    );
    generation
}

/// Explicit interpretation of circuit qubit identifiers during device synthesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeviceSynthesisPlacement {
    /// Circuit qubits are logical; compare candidates over physical placement domains.
    PreLayoutEnvelope,
    /// Circuit qubits are routed physical identifiers and may be converted explicitly.
    ExactPhysical,
}

pub(crate) use crate::compile::device_planning::DevicePhysicalCost;

/// Placement coverage compared before pre-layout physical costs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DeviceCoverageKey {
    /// Eligible movement components with no executable ordered pair.
    pub(crate) uncovered_components: usize,
    /// Eligible ordered topology pairs outside the candidate's terminal domain.
    pub(crate) infeasible_ordered_pairs: usize,
}

pub(crate) type OrderedPairDomain = BTreeSet<[PhysicalQubit; 2]>;

/// Pre-layout feasibility and worst physical cost for one operation sequence.
#[derive(Debug, Clone)]
pub(crate) struct DevicePreLayoutEvaluation {
    pub(crate) domain: OrderedPairDomain,
    pub(crate) coverage: DeviceCoverageKey,
    pub(crate) worst_cost: DevicePhysicalCost,
    // Sorted by ordered pair. Arc keeps candidate/template clones shallow and
    // a packed slice avoids one BTree node allocation per topology edge.
    pair_costs: Arc<[([PhysicalQubit; 2], DevicePhysicalCost)]>,
}

impl DevicePreLayoutEvaluation {
    /// Returns the worst already-computed cost on `domain`.
    ///
    /// Pre-layout evaluation deliberately retains the exact per-pair results
    /// used to derive its public comparison fields. Candidate selection can
    /// therefore compare a candidate on a source domain without scheduling
    /// the same native sequence a second time.
    pub(crate) fn worst_cost_on_domain(
        &self,
        domain: &OrderedPairDomain,
    ) -> Option<DevicePhysicalCost> {
        domain
            .iter()
            .filter_map(|pair| {
                self.pair_costs
                    .binary_search_by_key(pair, |(candidate, _)| *candidate)
                    .ok()
                    .map(|index| self.pair_costs[index].1)
            })
            .max_by(|left, right| left.compare(*right))
    }
}

#[derive(Debug)]
struct DeviceTwoQubitSynthesisData {
    generation: u64,
    placement: DeviceSynthesisPlacement,
    eligible_pairs: BTreeSet<[PhysicalQubit; 2]>,
    eligible_components: BTreeSet<usize>,
    component_by_qubit: HashMap<PhysicalQubit, usize>,
    native_backends: BTreeMap<[PhysicalQubit; 2], HashSet<StandardGate>>,
    planning_session: Arc<DevicePlanningSession>,
    estimator: Arc<CalibrationEstimator>,
    physical_cost_cache: Mutex<PhysicalSequenceCostCache>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PhysicalSequenceCostKey {
    ordered_pair: [PhysicalQubit; 2],
    states: Vec<DeviceGateState>,
}

#[derive(Debug, Default)]
struct PhysicalSequenceCostCache {
    entries: HashMap<PhysicalSequenceCostKey, Result<DevicePhysicalCost, DeviceContextCostFailure>>,
}

/// Pass-local exact device planning data shared by all matrices and blocks.
#[derive(Debug, Clone)]
pub(crate) struct DeviceTwoQubitSynthesisContext {
    data: Arc<DeviceTwoQubitSynthesisData>,
}

/// Diagnostic reason why an operation sequence cannot be costed by a device context.
#[derive(Debug, Clone)]
pub(crate) enum DeviceContextCostFailure {
    /// The requested exact cost was evaluated with a pre-layout context.
    WrongPlacement,
    /// The sequence contains an instruction or qarg shape outside the cost API contract.
    InvalidOperation(String),
    /// The catalog was not built with this exact gate state as a root.
    Unprepared(DeviceGateState),
    /// The root was prepared, but no native lowering plan exists.
    Unsupported(DeviceLoweringFailure),
}

impl DeviceTwoQubitSynthesisContext {
    /// Builds a synthesis view over one workflow-scoped planning session.
    pub(crate) fn build_with_session(
        device: &Device,
        circuit: &Circuit,
        placement: DeviceSynthesisPlacement,
        planning_session: Arc<DevicePlanningSession>,
    ) -> Result<Self, CompilerError> {
        let physical_qubits = device.usable_qubits().collect::<Vec<_>>();
        let topology_pairs = ordered_topology_pairs(device, &physical_qubits);
        let ordered_pairs = match placement {
            DeviceSynthesisPlacement::PreLayoutEnvelope => topology_pairs.clone(),
            DeviceSynthesisPlacement::ExactPhysical => {
                collect_exact_physical_pairs(circuit.operations())
            }
        };
        // Only SWAP reachability is device-wide. Candidate KAK gates and exact
        // circuit roots are prepared on first use by `catalog_summary`.
        let movement_catalog = match placement {
            DeviceSynthesisPlacement::PreLayoutEnvelope => {
                let movement_roots = topology_pairs.iter().map(|pair| {
                    DeviceGateState::standard(StandardGate::SWAP, smallvec![pair[0], pair[1]])
                });
                NativePlanCatalog::build_with_session(&planning_session, movement_roots)?
            }
            DeviceSynthesisPlacement::ExactPhysical => {
                NativePlanCatalog::build_with_session(&planning_session, [])?
            }
        };
        if placement == DeviceSynthesisPlacement::ExactPhysical {
            planning_session.prepare(collect_exact_physical_gate_roots(circuit.operations()))?;
        }
        let estimator = planning_session.estimator();
        let (component_by_qubit, eligible_components, eligible_pairs) = match placement {
            DeviceSynthesisPlacement::PreLayoutEnvelope => {
                let (component_by_qubit, _component_sizes) =
                    movement_components(&physical_qubits, &topology_pairs, &movement_catalog);
                // Terminal pairs remain useful even when no lowerable SWAP
                // joins their movement components. Excluding such pairs would
                // reject valid fixed placements on devices that can execute a
                // 2Q gate but cannot synthesize SWAP.
                let eligible_components = topology_pairs
                    .iter()
                    .flat_map(|pair| pair.iter())
                    .filter_map(|qubit| component_by_qubit.get(qubit).copied())
                    .collect::<BTreeSet<_>>();
                let eligible_pairs = topology_pairs.iter().copied().collect();
                (component_by_qubit, eligible_components, eligible_pairs)
            }
            DeviceSynthesisPlacement::ExactPhysical => {
                (HashMap::new(), BTreeSet::new(), BTreeSet::new())
            }
        };
        let native_backends = native_backend_map(device, &ordered_pairs);

        Ok(Self {
            data: Arc::new(DeviceTwoQubitSynthesisData {
                generation: next_device_synthesis_context_generation(),
                placement,
                eligible_pairs,
                eligible_components,
                component_by_qubit,
                native_backends,
                planning_session,
                estimator,
                physical_cost_cache: Mutex::new(PhysicalSequenceCostCache::default()),
            }),
        })
    }

    pub(crate) fn placement(&self) -> DeviceSynthesisPlacement {
        self.data.placement
    }

    /// Opaque identity of the immutable planning and calibration snapshot.
    /// Clones retain it; every rebuilt context receives a fresh generation.
    pub(crate) fn generation(&self) -> u64 {
        self.data.generation
    }

    /// KAK backends directly native somewhere relevant to this request.
    pub(crate) fn native_two_qubit_backends(&self, qubits: [Qubit; 2]) -> HashSet<StandardGate> {
        match self.data.placement {
            DeviceSynthesisPlacement::PreLayoutEnvelope => self
                .data
                .eligible_pairs
                .iter()
                .filter_map(|pair| self.data.native_backends.get(pair))
                .flat_map(|gates| gates.iter().copied())
                .collect(),
            DeviceSynthesisPlacement::ExactPhysical => {
                let pair = qubits.map(PhysicalQubit::from_qubit);
                [pair, [pair[1], pair[0]]]
                    .into_iter()
                    .filter_map(|ordered| self.data.native_backends.get(&ordered))
                    .flat_map(|gates| gates.iter().copied())
                    .collect()
            }
        }
    }

    pub(crate) fn evaluate_pre_layout(
        &self,
        operations: &[ValueOperation],
        logical_qubits: [Qubit; 2],
    ) -> Option<DevicePreLayoutEvaluation> {
        if self.data.placement != DeviceSynthesisPlacement::PreLayoutEnvelope {
            return None;
        }
        // Prepare this candidate's roots as one deterministic batch. The
        // session can then serve each pair lookup from the bounded snapshot.
        let roots = self
            .data
            .eligible_pairs
            .iter()
            .flat_map(|pair| states_on_pair(operations, logical_qubits, *pair).unwrap_or_default())
            .collect::<Vec<_>>();
        let plans = self.data.planning_session.prepare(roots).ok()?;
        let pair_costs = self
            .data
            .eligible_pairs
            .iter()
            .filter_map(|pair| {
                self.cost_on_pair_diagnostic(operations, logical_qubits, *pair, Some(&plans))
                    .ok()
                    .map(|cost| (*pair, cost))
            })
            .collect::<Vec<_>>();
        let domain = pair_costs
            .iter()
            .map(|(pair, _)| *pair)
            .collect::<OrderedPairDomain>();
        let worst_cost = pair_costs
            .iter()
            .map(|(_, cost)| *cost)
            .max_by(|left, right| left.compare(*right))?;
        Some(DevicePreLayoutEvaluation {
            coverage: self.coverage_key(&domain),
            domain,
            worst_cost,
            pair_costs: pair_costs.into(),
        })
    }

    /// Costs a two-qubit sequence and preserves the reason costing is unavailable.
    pub(crate) fn exact_cost_diagnostic(
        &self,
        operations: &[ValueOperation],
        physical_qubits: [Qubit; 2],
    ) -> Result<DevicePhysicalCost, DeviceContextCostFailure> {
        if self.data.placement != DeviceSynthesisPlacement::ExactPhysical {
            return Err(DeviceContextCostFailure::WrongPlacement);
        }
        let pair = physical_qubits.map(PhysicalQubit::from_qubit);
        self.cost_on_pair_diagnostic(operations, physical_qubits, pair, None)
    }

    /// Costs one flat operation sequence on the physical qargs carried by the
    /// operations themselves.
    ///
    /// Every gate-like operation is expanded through the same selected native
    /// plan used by [`DeviceLowerer`](crate::compile::transform::DeviceLowerer).
    /// Control flow and non-gate instructions are deliberately outside this
    /// sequence-level API and make the sequence unavailable for costing.
    /// Costs a flat exact-physical sequence with explicit coverage diagnostics.
    ///
    /// Unlike [`Self::exact_sequence_cost`], this method distinguishes a missing
    /// catalog root from a prepared root for which planning found no lowering.
    pub(crate) fn exact_sequence_cost_diagnostic(
        &self,
        operations: &[ValueOperation],
    ) -> Result<DevicePhysicalCost, DeviceContextCostFailure> {
        if self.data.placement != DeviceSynthesisPlacement::ExactPhysical {
            return Err(DeviceContextCostFailure::WrongPlacement);
        }

        let mut accumulator = self.exact_sequence_cost_accumulator()?;
        for operation in operations {
            let ValueInstruction::Instruction(instruction) = &operation.instruction else {
                return Err(DeviceContextCostFailure::InvalidOperation(
                    "device sequence cost requires gate-like instructions".to_string(),
                ));
            };
            accumulator.add_gate(instruction, &operation.qubits)?;
        }
        Ok(accumulator.finish())
    }

    /// Starts a streaming exact-physical sequence cost accumulation.
    ///
    /// Gates are fed one at a time, avoiding the intermediate operation vector
    /// that [`Self::exact_sequence_cost_diagnostic`] requires. The accumulation
    /// order and the resulting cost are identical.
    pub(crate) fn exact_sequence_cost_accumulator(
        &self,
    ) -> Result<ExactSequenceCostAccumulator<'_>, DeviceContextCostFailure> {
        if self.data.placement != DeviceSynthesisPlacement::ExactPhysical {
            return Err(DeviceContextCostFailure::WrongPlacement);
        }
        Ok(ExactSequenceCostAccumulator {
            context: self,
            leaves: Vec::new(),
            aggregate: self.data.estimator.identity_cost(),
        })
    }

    fn coverage_key(&self, domain: &OrderedPairDomain) -> DeviceCoverageKey {
        let covered_components = domain
            .iter()
            .flat_map(|pair| pair.iter())
            .filter_map(|qubit| self.data.component_by_qubit.get(qubit).copied())
            .collect::<HashSet<_>>();
        DeviceCoverageKey {
            uncovered_components: self
                .data
                .eligible_components
                .iter()
                .filter(|component| !covered_components.contains(component))
                .count(),
            infeasible_ordered_pairs: self.data.eligible_pairs.len().saturating_sub(domain.len()),
        }
    }

    fn cost_on_pair_diagnostic(
        &self,
        operations: &[ValueOperation],
        circuit_qubits: [Qubit; 2],
        physical_pair: [PhysicalQubit; 2],
        plans: Option<&DevicePlanSnapshot>,
    ) -> Result<DevicePhysicalCost, DeviceContextCostFailure> {
        let mut cacheable = true;
        let mut states = Vec::with_capacity(operations.len());
        for operation in operations {
            let ValueInstruction::Instruction(instruction) = &operation.instruction else {
                return Err(DeviceContextCostFailure::InvalidOperation(
                    "device pair cost requires gate-like instructions".to_string(),
                ));
            };
            if matches!(instruction, Instruction::Standard(StandardGate::GPhase)) {
                continue;
            }
            cacheable &= self
                .data
                .estimator
                .gate_cost_is_parameter_invariant(instruction);
            let ordered_qargs = operation
                .qubits
                .iter()
                .map(|qubit| {
                    if *qubit == circuit_qubits[0] {
                        Some(physical_pair[0])
                    } else if *qubit == circuit_qubits[1] {
                        Some(physical_pair[1])
                    } else {
                        None
                    }
                })
                .collect::<Option<_>>()
                .ok_or_else(|| {
                    DeviceContextCostFailure::InvalidOperation(
                        "device pair cost references a qubit outside the requested pair"
                            .to_string(),
                    )
                })?;
            let state =
                DeviceGateState::from_instruction(instruction, ordered_qargs).ok_or_else(|| {
                    DeviceContextCostFailure::InvalidOperation(format!(
                        "missing device-planning key for {instruction}"
                    ))
                })?;
            states.push(state);
        }

        let key = PhysicalSequenceCostKey {
            ordered_pair: physical_pair,
            states,
        };
        if cacheable {
            let cached = {
                let cache = self
                    .data
                    .physical_cost_cache
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                cache.entries.get(&key).cloned()
            };
            if let Some(cached) = cached {
                return cached;
            }
        }

        let result = self.compute_physical_sequence_cost(&key, plans);
        if cacheable {
            let mut cache = self
                .data
                .physical_cost_cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if cache.entries.len() < PHYSICAL_SEQUENCE_COST_CACHE_BUDGET {
                cache.entries.insert(key, result.clone());
            }
        }
        result
    }

    fn compute_physical_sequence_cost(
        &self,
        key: &PhysicalSequenceCostKey,
        plans: Option<&DevicePlanSnapshot>,
    ) -> Result<DevicePhysicalCost, DeviceContextCostFailure> {
        let mut scheduler = self
            .data
            .estimator
            .two_qubit_cost_accumulator(key.ordered_pair);
        let mut aggregate = self.data.estimator.identity_cost();
        for state in &key.states {
            let summary = self.catalog_summary(state, plans)?;
            aggregate = aggregate.combine(self.data.estimator.cost(&summary));
            scheduler.add_leaves(&summary.leaves).map_err(|reason| {
                DeviceContextCostFailure::InvalidOperation(format!(
                    "invalid native two-qubit schedule: {reason}"
                ))
            })?;
        }
        Ok(scheduler.finish(aggregate))
    }

    fn catalog_summary(
        &self,
        state: &DeviceGateState,
        plans: Option<&DevicePlanSnapshot>,
    ) -> Result<NativePlanSummary, DeviceContextCostFailure> {
        let prepared;
        let plans = if let Some(plans) = plans {
            plans
        } else {
            prepared = self
                .data
                .planning_session
                .prepare([state.clone()])
                .map_err(|error| {
                    DeviceContextCostFailure::InvalidOperation(format!(
                        "exact device planning failed: {error}"
                    ))
                })?;
            &prepared
        };
        match plans.availability(state) {
            Some(NativePlanAvailability::Feasible(summary)) => Ok(summary),
            Some(NativePlanAvailability::Unsupported(failure)) => {
                Err(DeviceContextCostFailure::Unsupported(failure))
            }
            None => Err(DeviceContextCostFailure::Unprepared(state.clone())),
        }
    }
}

fn states_on_pair(
    operations: &[ValueOperation],
    circuit_qubits: [Qubit; 2],
    physical_pair: [PhysicalQubit; 2],
) -> Result<Vec<DeviceGateState>, DeviceContextCostFailure> {
    let mut states = Vec::with_capacity(operations.len());
    for operation in operations {
        let ValueInstruction::Instruction(instruction) = &operation.instruction else {
            return Err(DeviceContextCostFailure::InvalidOperation(
                "device pair cost requires gate-like instructions".to_string(),
            ));
        };
        if matches!(instruction, Instruction::Standard(StandardGate::GPhase)) {
            continue;
        }
        let ordered_qargs = operation
            .qubits
            .iter()
            .map(|qubit| {
                if *qubit == circuit_qubits[0] {
                    Some(physical_pair[0])
                } else if *qubit == circuit_qubits[1] {
                    Some(physical_pair[1])
                } else {
                    None
                }
            })
            .collect::<Option<_>>()
            .ok_or_else(|| {
                DeviceContextCostFailure::InvalidOperation(
                    "device pair cost references a qubit outside the requested pair".to_string(),
                )
            })?;
        states.push(
            DeviceGateState::from_instruction(instruction, ordered_qargs).ok_or_else(|| {
                DeviceContextCostFailure::InvalidOperation(format!(
                    "missing device-planning key for {instruction}"
                ))
            })?,
        );
    }
    Ok(states)
}

/// Streaming accumulator for exact-physical sequence cost.
///
/// See [`DeviceTwoQubitSynthesisContext::exact_sequence_cost_accumulator`].
pub(crate) struct ExactSequenceCostAccumulator<'a> {
    context: &'a DeviceTwoQubitSynthesisContext,
    leaves: Vec<NativePlanLeaf>,
    aggregate: NativePlanCost,
}

impl ExactSequenceCostAccumulator<'_> {
    /// Adds one gate-like operation to the accumulated sequence.
    pub(crate) fn add_gate(
        &mut self,
        instruction: &Instruction,
        qubits: &[Qubit],
    ) -> Result<(), DeviceContextCostFailure> {
        if matches!(instruction, Instruction::Standard(StandardGate::GPhase)) {
            return Ok(());
        }
        let ordered_qargs = qubits
            .iter()
            .copied()
            .map(PhysicalQubit::from_qubit)
            .collect();
        let state =
            DeviceGateState::from_instruction(instruction, ordered_qargs).ok_or_else(|| {
                DeviceContextCostFailure::InvalidOperation(format!(
                    "missing device-planning key for {instruction}"
                ))
            })?;
        let summary = self.context.catalog_summary(&state, None)?;
        self.aggregate = self
            .aggregate
            .combine(self.context.data.estimator.cost(&summary));
        self.leaves.extend(summary.leaves.iter().cloned());
        Ok(())
    }

    /// Completes the accumulation and returns the scheduled physical cost.
    pub(crate) fn finish(self) -> DevicePhysicalCost {
        self.context
            .data
            .estimator
            .schedule_physical_cost(&self.leaves, self.aggregate)
    }
}

fn ordered_topology_pairs(
    device: &Device,
    physical_qubits: &[PhysicalQubit],
) -> Vec<[PhysicalQubit; 2]> {
    let usable = physical_qubits.iter().copied().collect::<HashSet<_>>();
    let mut pairs = BTreeSet::new();
    for (left, right) in device.topology().undirected_edges() {
        if usable.contains(&left) && usable.contains(&right) {
            pairs.insert([left, right]);
            pairs.insert([right, left]);
        }
    }
    pairs.into_iter().collect()
}

fn collect_exact_physical_gate_roots(
    operations: &[crate::circuit::Operation],
) -> Vec<DeviceGateState> {
    use crate::circuit::ClassicalControlOp;

    let mut roots = Vec::new();
    for operation in operations {
        match &operation.instruction {
            Instruction::Standard(StandardGate::GPhase) => {}
            Instruction::Standard(_) | Instruction::McGate(_) => {
                if let Some(state) = DeviceGateState::from_instruction(
                    &operation.instruction,
                    operation
                        .qubits
                        .iter()
                        .copied()
                        .map(PhysicalQubit::from_qubit)
                        .collect(),
                ) {
                    roots.push(state);
                }
            }
            Instruction::ClassicalControl(control) => match control {
                ClassicalControlOp::If(op) => {
                    roots.extend(collect_exact_physical_gate_roots(
                        op.then_body().operations(),
                    ));
                    if let Some(body) = op.else_body() {
                        roots.extend(collect_exact_physical_gate_roots(body.operations()));
                    }
                }
                ClassicalControlOp::While(op) => {
                    roots.extend(collect_exact_physical_gate_roots(op.body().operations()));
                }
                ClassicalControlOp::For(op) => {
                    roots.extend(collect_exact_physical_gate_roots(op.body().operations()));
                }
                ClassicalControlOp::Switch(op) => {
                    for case in op.cases() {
                        roots.extend(collect_exact_physical_gate_roots(case.body().operations()));
                    }
                    if let Some(body) = op.default() {
                        roots.extend(collect_exact_physical_gate_roots(body.operations()));
                    }
                }
                ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
            },
            _ => {}
        }
    }
    roots
}

fn collect_exact_physical_pairs(
    operations: &[crate::circuit::Operation],
) -> Vec<[PhysicalQubit; 2]> {
    use crate::circuit::ClassicalControlOp;

    let mut pairs = BTreeSet::new();
    for operation in operations {
        match &operation.instruction {
            Instruction::Standard(gate)
                if gate.num_qubits() == 2 && operation.qubits.len() == 2 =>
            {
                let pair = operation
                    .qubits
                    .iter()
                    .copied()
                    .map(PhysicalQubit::from_qubit)
                    .collect::<Vec<_>>();
                pairs.insert([pair[0], pair[1]]);
                pairs.insert([pair[1], pair[0]]);
            }
            Instruction::ClassicalControl(control) => match control {
                ClassicalControlOp::If(op) => {
                    pairs.extend(collect_exact_physical_pairs(op.then_body().operations()));
                    if let Some(body) = op.else_body() {
                        pairs.extend(collect_exact_physical_pairs(body.operations()));
                    }
                }
                ClassicalControlOp::While(op) => {
                    pairs.extend(collect_exact_physical_pairs(op.body().operations()));
                }
                ClassicalControlOp::For(op) => {
                    pairs.extend(collect_exact_physical_pairs(op.body().operations()));
                }
                ClassicalControlOp::Switch(op) => {
                    for case in op.cases() {
                        pairs.extend(collect_exact_physical_pairs(case.body().operations()));
                    }
                    if let Some(body) = op.default() {
                        pairs.extend(collect_exact_physical_pairs(body.operations()));
                    }
                }
                ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
            },
            _ => {}
        }
    }
    pairs.into_iter().collect()
}

fn native_backend_map(
    device: &Device,
    ordered_pairs: &[[PhysicalQubit; 2]],
) -> BTreeMap<[PhysicalQubit; 2], HashSet<StandardGate>> {
    const BACKENDS: [StandardGate; 6] = [
        StandardGate::CX,
        StandardGate::CY,
        StandardGate::CZ,
        StandardGate::RXX,
        StandardGate::RYY,
        StandardGate::RZZ,
    ];
    ordered_pairs
        .iter()
        .copied()
        .map(|pair| {
            let gates = BACKENDS
                .into_iter()
                .filter(|gate| {
                    device
                        .supports_native_instruction(&Instruction::Standard(*gate), pair.as_slice())
                })
                .collect();
            (pair, gates)
        })
        .collect()
}

fn movement_components(
    physical_qubits: &[PhysicalQubit],
    ordered_pairs: &[[PhysicalQubit; 2]],
    catalog: &NativePlanCatalog,
) -> (HashMap<PhysicalQubit, usize>, Vec<usize>) {
    let mut neighbors = HashMap::<PhysicalQubit, Vec<PhysicalQubit>>::new();
    for &pair in ordered_pairs {
        let state = DeviceGateState::standard(StandardGate::SWAP, smallvec![pair[0], pair[1]]);
        if catalog.summary(&state).is_some() {
            neighbors.entry(pair[0]).or_default().push(pair[1]);
            neighbors.entry(pair[1]).or_default().push(pair[0]);
        }
    }

    let mut component_by_qubit = HashMap::new();
    let mut component_sizes = Vec::new();
    for &start in physical_qubits {
        if component_by_qubit.contains_key(&start) {
            continue;
        }
        let component = component_sizes.len();
        let mut size = 0;
        let mut queue = VecDeque::from([start]);
        component_by_qubit.insert(start, component);
        while let Some(qubit) = queue.pop_front() {
            size += 1;
            for &neighbor in neighbors.get(&qubit).into_iter().flatten() {
                if component_by_qubit.insert(neighbor, component).is_none() {
                    queue.push_back(neighbor);
                }
            }
        }
        component_sizes.push(size);
    }
    (component_by_qubit, component_sizes)
}

#[cfg(test)]
#[path = "device_synthesis_test.rs"]
mod device_synthesis_test;
