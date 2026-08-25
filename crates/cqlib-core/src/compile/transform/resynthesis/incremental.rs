// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Run-scoped incremental collection for native fixed-point resynthesis.

use super::collector::TwoQubitNumericBlock;
use super::commutation::{CachedCommutation, OperationView};
use super::config::TwoQubitBlockResynthesisConfig;
use super::dag_collector::{AnchorDependencyTrace, DagCollectionContext, is_two_qubit_anchor};
use super::maximal_run::collect_maximal_two_qubit_runs;
use super::synthesis_cache::{TwoQubitSynthesisCache, TwoQubitSynthesisCacheStats};
use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, Instruction, Operation, Parameter, Qubit,
};
use crate::compile::CompilerError;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};

const DIRTY_FULL_SCAN_PERCENT: usize = 50;
const MAX_CACHED_DEPENDENCY_IDS: usize = 8_388_608;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeResynthesisPolicy {
    FullScan,
    Incremental,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NativeWorksetStats {
    pub(crate) scopes_total: usize,
    pub(crate) scopes_unchanged: usize,
    pub(crate) scopes_full_scan: usize,
    pub(crate) anchors_total: usize,
    pub(crate) anchors_reused: usize,
    pub(crate) anchors_recomputed: usize,
    pub(crate) dependency_failures: usize,
    pub(crate) cache_rejections: usize,
    pub(crate) maximal_blocks_total: usize,
    pub(crate) maximal_blocks_reused: usize,
    pub(crate) maximal_blocks_recomputed: usize,
    pub(crate) cycle_early_exits: usize,
    pub(crate) synthesis_cache: TwoQubitSynthesisCacheStats,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum NativeScopeSegment {
    IfThen(u64),
    IfElse(u64),
    WhileBody(u64),
    ForBody(u64),
    SwitchCase { operation: u64, case: usize },
    SwitchDefault(u64),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct NativeScopeId(pub(crate) Vec<NativeScopeSegment>);

impl NativeScopeId {
    pub(crate) fn child(&self, segment: NativeScopeSegment) -> Self {
        let mut path = self.0.clone();
        path.push(segment);
        Self(path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct NativeOperationId(u64);

impl NativeOperationId {
    fn ordered_pair(self, other: Self) -> (Self, Self) {
        if self <= other {
            (self, other)
        } else {
            (other, self)
        }
    }
}

#[derive(Debug, Clone)]
enum SnapshotParam {
    Fixed(u64),
    Symbolic(Parameter),
}

#[derive(Debug, Clone)]
struct OperationSnapshot {
    instruction: Instruction,
    qubits: Vec<Qubit>,
    params: Vec<SnapshotParam>,
    label: Option<Box<str>>,
    fast_hash: u64,
}

impl OperationSnapshot {
    fn new(circuit: &Circuit, operation: &Operation) -> Result<Self, CompilerError> {
        let params = operation
            .params
            .iter()
            .map(|parameter| match parameter {
                CircuitParam::Fixed(value) => Ok(SnapshotParam::Fixed(value.to_bits())),
                CircuitParam::Index(index) => circuit
                    .parameters()
                    .get_index(*index as usize)
                    .cloned()
                    .map(SnapshotParam::Symbolic)
                    .ok_or_else(|| {
                        CompilerError::InvalidInput(format!("missing parameter index {index}"))
                    }),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        match &operation.instruction {
            Instruction::Standard(gate) => {
                0_u8.hash(&mut hasher);
                gate.hash(&mut hasher);
            }
            Instruction::McGate(gate) => {
                1_u8.hash(&mut hasher);
                gate.hash(&mut hasher);
            }
            Instruction::UnitaryGate(gate) => {
                2_u8.hash(&mut hasher);
                gate.hash(&mut hasher);
            }
            Instruction::CircuitGate(gate) => {
                3_u8.hash(&mut hasher);
                gate.name.hash(&mut hasher);
                gate.num_qubits().hash(&mut hasher);
                gate.num_params().hash(&mut hasher);
            }
            Instruction::Directive(_) => 4_u8.hash(&mut hasher),
            Instruction::ClassicalData(_) => 5_u8.hash(&mut hasher),
            Instruction::ClassicalControl(control) => {
                6_u8.hash(&mut hasher);
                Self::control_shape(control).hash(&mut hasher);
            }
            Instruction::Delay => 7_u8.hash(&mut hasher),
        }
        operation.qubits.hash(&mut hasher);
        for parameter in &params {
            match parameter {
                SnapshotParam::Fixed(bits) => {
                    0_u8.hash(&mut hasher);
                    bits.hash(&mut hasher);
                }
                SnapshotParam::Symbolic(parameter) => {
                    1_u8.hash(&mut hasher);
                    parameter.hash(&mut hasher);
                }
            }
        }
        operation.label.hash(&mut hasher);
        Ok(Self {
            instruction: operation.instruction.clone(),
            qubits: operation.qubits.iter().copied().collect(),
            params,
            label: operation.label.clone(),
            fast_hash: hasher.finish(),
        })
    }

    fn exact_eq(&self, other: &Self) -> bool {
        self.fast_hash == other.fast_hash
            && match (&self.instruction, &other.instruction) {
                (Instruction::ClassicalControl(left), Instruction::ClassicalControl(right)) => {
                    Self::control_shape(left) == Self::control_shape(right)
                }
                (left, right) => left == right,
            }
            && self.qubits == other.qubits
            && self.params.len() == other.params.len()
            && self
                .params
                .iter()
                .zip(&other.params)
                .all(|(left, right)| match (left, right) {
                    (SnapshotParam::Fixed(left), SnapshotParam::Fixed(right)) => left == right,
                    (SnapshotParam::Symbolic(left), SnapshotParam::Symbolic(right)) => {
                        left == right
                    }
                    _ => false,
                })
            && self.label == other.label
    }

    fn is_hard_boundary(&self, config: &TwoQubitBlockResynthesisConfig) -> bool {
        (config.skip_labeled_ops && self.label.is_some())
            || !matches!(self.instruction, Instruction::Standard(_))
            || self.qubits.len() > 2
    }

    fn control_shape(
        control: &ClassicalControlOp,
    ) -> (crate::circuit::ClassicalControlKind, usize, bool) {
        match control {
            ClassicalControlOp::If(operation) => {
                (control.kind(), 1, operation.else_body().is_some())
            }
            ClassicalControlOp::While(_) | ClassicalControlOp::For(_) => (control.kind(), 1, false),
            ClassicalControlOp::Switch(operation) => (
                control.kind(),
                operation.cases().len(),
                operation.default().is_some(),
            ),
            ClassicalControlOp::Break | ClassicalControlOp::Continue => (control.kind(), 0, false),
        }
    }
}

#[derive(Debug, Clone)]
struct CachedBlock {
    qubits: [Qubit; 2],
    matched: Vec<NativeOperationId>,
    crossed: Vec<NativeOperationId>,
    matched_1q_count: usize,
    matched_2q_count: usize,
    contains_swap: bool,
}

impl CachedBlock {
    fn new(block: &TwoQubitNumericBlock, ids: &[NativeOperationId]) -> Self {
        Self {
            qubits: block.qubits,
            matched: block
                .matched_orders
                .iter()
                .map(|order| ids[*order])
                .collect(),
            crossed: block
                .crossed_orders
                .iter()
                .map(|order| ids[*order])
                .collect(),
            matched_1q_count: block.matched_1q_count,
            matched_2q_count: block.matched_2q_count,
            contains_swap: block.contains_swap,
        }
    }

    fn materialize(
        &self,
        orders: &BTreeMap<NativeOperationId, usize>,
    ) -> Option<TwoQubitNumericBlock> {
        let mut matched_orders = self
            .matched
            .iter()
            .map(|id| orders.get(id).copied())
            .collect::<Option<Vec<_>>>()?;
        let mut crossed_orders = self
            .crossed
            .iter()
            .map(|id| orders.get(id).copied())
            .collect::<Option<Vec<_>>>()?;
        matched_orders.sort_unstable();
        crossed_orders.sort_unstable();
        Some(TwoQubitNumericBlock {
            qubits: self.qubits,
            matched_orders,
            crossed_orders,
            matched_1q_count: self.matched_1q_count,
            matched_2q_count: self.matched_2q_count,
            contains_swap: self.contains_swap,
        })
    }

    fn retained_id_count(&self) -> usize {
        self.matched.len().saturating_add(self.crossed.len())
    }
}

#[derive(Debug, Clone)]
struct CachedTrace {
    observed: Vec<NativeOperationId>,
    adjacency: BTreeSet<(NativeOperationId, NativeOperationId)>,
}

impl CachedTrace {
    fn new(trace: &AnchorDependencyTrace, ids: &[NativeOperationId]) -> Self {
        Self {
            observed: trace
                .observed_orders
                .iter()
                .map(|order| ids[*order])
                .collect(),
            adjacency: trace
                .adjacency
                .iter()
                .map(|(left, right)| ids[*left].ordered_pair(ids[*right]))
                .collect(),
        }
    }

    fn is_current(
        &self,
        dag: &DagCollectionContext,
        orders: &BTreeMap<NativeOperationId, usize>,
        ids: &[NativeOperationId],
    ) -> bool {
        let Some(observed_orders) = self
            .observed
            .iter()
            .map(|id| orders.get(id).copied())
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        if !observed_orders.windows(2).all(|pair| pair[0] < pair[1]) {
            return false;
        }
        let Some(adjacency) = dag.adjacency_for_orders(observed_orders) else {
            return false;
        };
        let current = adjacency
            .into_iter()
            .map(|(left, right)| ids[left].ordered_pair(ids[right]))
            .collect::<BTreeSet<_>>();
        current == self.adjacency
    }

    fn retained_id_count(&self) -> usize {
        self.observed
            .len()
            .saturating_add(self.adjacency.len().saturating_mul(2))
    }
}

#[derive(Debug, Clone)]
struct CachedAnchor {
    block: Option<CachedBlock>,
    trace: CachedTrace,
}

impl CachedAnchor {
    fn retained_id_count(&self) -> usize {
        self.trace.retained_id_count().saturating_add(
            self.block
                .as_ref()
                .map_or(0, CachedBlock::retained_id_count),
        )
    }
}

#[derive(Debug, Clone)]
struct ScopeCache {
    snapshots: Vec<OperationSnapshot>,
    operation_ids: Vec<NativeOperationId>,
    anchors: BTreeMap<NativeOperationId, CachedAnchor>,
    maximal_blocks: Vec<CachedBlock>,
    maximal_blocks_valid: bool,
}

#[derive(Debug, Clone, Default)]
struct PreparedScopeDiff {
    inserted: BTreeSet<NativeOperationId>,
    removed: BTreeSet<NativeOperationId>,
    touched_qubits: BTreeSet<Qubit>,
    ambiguous_boundary: bool,
}

impl ScopeCache {
    fn retained_id_count(&self) -> usize {
        let anchors = self
            .anchors
            .values()
            .map(CachedAnchor::retained_id_count)
            .fold(0usize, usize::saturating_add);
        anchors.saturating_add(
            self.maximal_blocks
                .iter()
                .map(CachedBlock::retained_id_count)
                .fold(0usize, usize::saturating_add),
        )
    }
}

pub(crate) struct NativeResynthesisSession {
    policy: NativeResynthesisPolicy,
    config: Option<TwoQubitBlockResynthesisConfig>,
    next_operation_id: u64,
    scopes: BTreeMap<NativeScopeId, ScopeCache>,
    next_scopes: BTreeMap<NativeScopeId, ScopeCache>,
    prepared_diffs: BTreeMap<NativeScopeId, PreparedScopeDiff>,
    next_cached_dependency_ids: usize,
    stats: NativeWorksetStats,
    pub(super) synthesis_cache: TwoQubitSynthesisCache,
}

impl NativeResynthesisSession {
    pub(crate) fn new(policy: NativeResynthesisPolicy) -> Self {
        Self {
            policy,
            config: None,
            next_operation_id: 0,
            scopes: BTreeMap::new(),
            next_scopes: BTreeMap::new(),
            prepared_diffs: BTreeMap::new(),
            next_cached_dependency_ids: 0,
            stats: NativeWorksetStats::default(),
            synthesis_cache: TwoQubitSynthesisCache::new_native_session(),
        }
    }

    pub(crate) fn begin_round(&mut self, config: &TwoQubitBlockResynthesisConfig) {
        if self.config.as_ref() != Some(config) {
            self.scopes.clear();
            self.config = Some(config.clone());
        }
        self.next_scopes.clear();
        self.prepared_diffs.clear();
        self.next_cached_dependency_ids = 0;
    }

    pub(crate) fn finish_round(&mut self) {
        self.scopes = std::mem::take(&mut self.next_scopes);
    }

    pub(crate) fn stats(&self) -> NativeWorksetStats {
        NativeWorksetStats {
            synthesis_cache: self.synthesis_cache.stats(),
            ..self.stats
        }
    }

    pub(crate) fn record_cycle_early_exit(&mut self) {
        self.stats.cycle_early_exits = self.stats.cycle_early_exits.saturating_add(1);
    }

    pub(crate) fn current_operation_key(&self, scope: &NativeScopeId, order: usize) -> Option<u64> {
        self.next_scopes
            .get(scope)?
            .operation_ids
            .get(order)
            .map(|id| id.0)
    }

    /// Reuses maximal runs without preparing the more expensive bounded-DAG
    /// workset when the entire scope is bit-exactly unchanged.
    pub(crate) fn reuse_unchanged_maximal_blocks(
        &mut self,
        scope: &NativeScopeId,
        circuit: &Circuit,
        operations: &[Operation],
    ) -> Result<Option<Vec<TwoQubitNumericBlock>>, CompilerError> {
        if self.policy != NativeResynthesisPolicy::Incremental {
            return Ok(None);
        }
        let snapshots = operations
            .iter()
            .map(|operation| OperationSnapshot::new(circuit, operation))
            .collect::<Result<Vec<_>, _>>()?;
        let Some(previous) = self.scopes.get(scope) else {
            return Ok(None);
        };
        if !previous.maximal_blocks_valid
            || previous.snapshots.len() != snapshots.len()
            || !previous
                .snapshots
                .iter()
                .zip(&snapshots)
                .all(|(left, right)| left.exact_eq(right))
        {
            return Ok(None);
        }

        let id_to_order = previous
            .operation_ids
            .iter()
            .copied()
            .enumerate()
            .map(|(order, id)| (id, order))
            .collect::<BTreeMap<_, _>>();
        let blocks = previous
            .maximal_blocks
            .iter()
            .map(|block| block.materialize(&id_to_order))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "unchanged maximal block lost an operation mapping".to_string(),
                )
            })?;
        let retained = previous.retained_id_count();
        self.next_cached_dependency_ids = self.next_cached_dependency_ids.saturating_add(retained);
        self.next_scopes.insert(scope.clone(), previous.clone());
        self.prepared_diffs
            .insert(scope.clone(), PreparedScopeDiff::default());
        self.stats.scopes_total = self.stats.scopes_total.saturating_add(1);
        self.stats.scopes_unchanged = self.stats.scopes_unchanged.saturating_add(1);
        self.stats.maximal_blocks_total =
            self.stats.maximal_blocks_total.saturating_add(blocks.len());
        self.stats.maximal_blocks_reused = self
            .stats
            .maximal_blocks_reused
            .saturating_add(blocks.len());
        Ok(Some(blocks))
    }

    /// Prepares only the exact operation identity map needed to cache maximal
    /// runs. Bounded-DAG construction remains deferred until maximal patches
    /// prove insufficient for this scope.
    pub(crate) fn prepare_scope_for_maximal(
        &mut self,
        scope: &NativeScopeId,
        circuit: &Circuit,
        operations: &[Operation],
        config: &TwoQubitBlockResynthesisConfig,
    ) -> Result<(), CompilerError> {
        let snapshots = operations
            .iter()
            .map(|operation| OperationSnapshot::new(circuit, operation))
            .collect::<Result<Vec<_>, _>>()?;
        let previous = self.scopes.get(scope).cloned();
        let (operation_ids, inserted, removed, touched_qubits, ambiguous_boundary) =
            self.reconcile(previous.as_ref(), &snapshots, config);
        self.stats.scopes_total = self.stats.scopes_total.saturating_add(1);
        self.next_scopes.insert(
            scope.clone(),
            ScopeCache {
                snapshots,
                operation_ids,
                // A changed scope cannot reuse bounded anchor traces until the
                // DAG validates their dependencies. Keep them empty here so a
                // later `collect_blocks` takes its normal dirty-check path.
                anchors: BTreeMap::new(),
                maximal_blocks: Vec::new(),
                maximal_blocks_valid: false,
            },
        );
        self.prepared_diffs.insert(
            scope.clone(),
            PreparedScopeDiff {
                inserted,
                removed,
                touched_qubits,
                ambiguous_boundary,
            },
        );
        Ok(())
    }

    pub(crate) fn collect_blocks(
        &mut self,
        scope: &NativeScopeId,
        circuit: &Circuit,
        operations: &[Operation],
        views: &[OperationView<'_>],
        commutation: &mut CachedCommutation,
        config: &TwoQubitBlockResynthesisConfig,
    ) -> Result<Vec<TwoQubitNumericBlock>, CompilerError> {
        // A maximal-only fast path may already have committed an exact copy of
        // this scope. Reuse its anchors if complete. If bounded collection was
        // not admitted previously, fall through and rebuild only that workset.
        let prepared = self.next_scopes.remove(scope);
        let already_counted = prepared.is_some();
        if let Some(prepared) = &prepared {
            let anchor_orders = views
                .iter()
                .enumerate()
                .filter_map(|(order, view)| is_two_qubit_anchor(view, config).then_some(order))
                .collect::<Vec<_>>();
            if anchor_orders.iter().all(|order| {
                prepared
                    .anchors
                    .contains_key(&prepared.operation_ids[*order])
            }) {
                let id_to_order = prepared
                    .operation_ids
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(order, id)| (id, order))
                    .collect::<BTreeMap<_, _>>();
                let blocks = anchor_orders
                    .iter()
                    .filter_map(|order| {
                        prepared.anchors[&prepared.operation_ids[*order]]
                            .block
                            .as_ref()
                    })
                    .map(|block| block.materialize(&id_to_order))
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        CompilerError::InvariantViolation(
                            "prepared resynthesis scope lost an operation mapping".to_string(),
                        )
                    })?;
                self.stats.anchors_total =
                    self.stats.anchors_total.saturating_add(anchor_orders.len());
                self.stats.anchors_reused = self
                    .stats
                    .anchors_reused
                    .saturating_add(anchor_orders.len());
                self.next_scopes.insert(scope.clone(), prepared.clone());
                self.prepared_diffs.remove(scope);
                return Ok(blocks);
            }
            self.next_cached_dependency_ids = self
                .next_cached_dependency_ids
                .saturating_sub(prepared.retained_id_count());
        }
        if !already_counted {
            self.stats.scopes_total = self.stats.scopes_total.saturating_add(1);
        }
        let previous = self.scopes.get(scope).cloned();
        let (
            snapshots,
            operation_ids,
            inserted,
            removed,
            touched_qubits,
            ambiguous_boundary,
            mut maximal_blocks,
            mut maximal_blocks_valid,
        ) = if let Some(prepared) = prepared {
            let diff = self.prepared_diffs.remove(scope).ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "prepared resynthesis scope lost its exact change set".to_string(),
                )
            })?;
            (
                prepared.snapshots,
                prepared.operation_ids,
                diff.inserted,
                diff.removed,
                diff.touched_qubits,
                diff.ambiguous_boundary,
                prepared.maximal_blocks,
                prepared.maximal_blocks_valid,
            )
        } else {
            let snapshots = operations
                .iter()
                .map(|operation| OperationSnapshot::new(circuit, operation))
                .collect::<Result<Vec<_>, _>>()?;
            let (operation_ids, inserted, removed, touched_qubits, ambiguous_boundary) =
                self.reconcile(previous.as_ref(), &snapshots, config);
            (
                snapshots,
                operation_ids,
                inserted,
                removed,
                touched_qubits,
                ambiguous_boundary,
                Vec::new(),
                false,
            )
        };
        let id_to_order = operation_ids
            .iter()
            .copied()
            .enumerate()
            .map(|(order, id)| (id, order))
            .collect::<BTreeMap<_, _>>();
        let anchor_orders = views
            .iter()
            .enumerate()
            .filter_map(|(order, view)| is_two_qubit_anchor(view, config).then_some(order))
            .collect::<Vec<_>>();
        self.stats.anchors_total = self.stats.anchors_total.saturating_add(anchor_orders.len());

        let unchanged = previous.as_ref().is_some_and(|previous| {
            previous.operation_ids == operation_ids
                && previous.snapshots.len() == snapshots.len()
                && previous
                    .snapshots
                    .iter()
                    .zip(&snapshots)
                    .all(|(left, right)| left.exact_eq(right))
                && anchor_orders
                    .iter()
                    .all(|order| previous.anchors.contains_key(&operation_ids[*order]))
        });
        if self.policy == NativeResynthesisPolicy::Incremental && unchanged {
            let previous = previous.expect("unchanged scope has previous cache");
            let blocks = anchor_orders
                .iter()
                .filter_map(|order| previous.anchors[&operation_ids[*order]].block.as_ref())
                .map(|block| block.materialize(&id_to_order))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "unchanged resynthesis scope lost an operation mapping".to_string(),
                    )
                })?;
            self.stats.scopes_unchanged = self.stats.scopes_unchanged.saturating_add(1);
            self.stats.anchors_reused = self
                .stats
                .anchors_reused
                .saturating_add(anchor_orders.len());
            self.next_cached_dependency_ids = self
                .next_cached_dependency_ids
                .saturating_add(previous.retained_id_count());
            self.next_scopes.insert(scope.clone(), previous);
            return Ok(blocks);
        }

        let dag = DagCollectionContext::build(views)?;
        let mut dirty = BTreeSet::new();
        let mut force_full = self.policy == NativeResynthesisPolicy::FullScan
            || previous.is_none()
            || ambiguous_boundary;
        if let Some(previous) = &previous {
            for (&anchor_id, cached) in &previous.anchors {
                if id_to_order.contains_key(&anchor_id)
                    && removed.iter().any(|id| cached.trace.observed.contains(id))
                {
                    dirty.insert(anchor_id);
                }
            }
        }
        for &order in &anchor_orders {
            let id = operation_ids[order];
            if inserted.contains(&id) {
                dirty.insert(id);
            }
            if views[order]
                .operation
                .qubits
                .iter()
                .any(|qubit| touched_qubits.contains(qubit))
            {
                dirty.insert(id);
            }
        }

        if let Some(previous) = &previous
            && !force_full
        {
            for &order in &anchor_orders {
                let id = operation_ids[order];
                if dirty.contains(&id) {
                    continue;
                }
                let Some(cached) = previous.anchors.get(&id) else {
                    dirty.insert(id);
                    continue;
                };
                if !cached.trace.is_current(&dag, &id_to_order, &operation_ids) {
                    dirty.insert(id);
                    self.stats.dependency_failures =
                        self.stats.dependency_failures.saturating_add(1);
                }
            }
        }
        if !anchor_orders.is_empty()
            && dirty.len().saturating_mul(100)
                >= anchor_orders.len().saturating_mul(DIRTY_FULL_SCAN_PERCENT)
        {
            force_full = true;
        }
        if force_full {
            self.stats.scopes_full_scan = self.stats.scopes_full_scan.saturating_add(1);
            dirty.extend(anchor_orders.iter().map(|order| operation_ids[*order]));
        }

        let mut anchors = BTreeMap::new();
        let mut blocks = Vec::new();
        for order in anchor_orders {
            let anchor_id = operation_ids[order];
            if !dirty.contains(&anchor_id)
                && let Some(cached) = previous
                    .as_ref()
                    .and_then(|previous| previous.anchors.get(&anchor_id))
            {
                if let Some(block) = &cached.block {
                    blocks.push(block.materialize(&id_to_order).ok_or_else(|| {
                        CompilerError::InvariantViolation(
                            "cached resynthesis block lost an operation mapping".to_string(),
                        )
                    })?);
                }
                if self.admit_anchor(cached) {
                    anchors.insert(anchor_id, cached.clone());
                }
                self.stats.anchors_reused = self.stats.anchors_reused.saturating_add(1);
                continue;
            }

            let (block, trace) = dag.collect_anchor(views, order, commutation, config)?;
            let cached_block = block
                .as_ref()
                .map(|block| CachedBlock::new(block, &operation_ids));
            let cached_trace = CachedTrace::new(&trace, &operation_ids);
            if let Some(block) = block {
                blocks.push(block);
            }
            let cached = CachedAnchor {
                block: cached_block,
                trace: cached_trace,
            };
            if self.admit_anchor(&cached) {
                anchors.insert(anchor_id, cached);
            }
            self.stats.anchors_recomputed = self.stats.anchors_recomputed.saturating_add(1);
        }
        if maximal_blocks_valid {
            let retained = maximal_blocks
                .iter()
                .map(CachedBlock::retained_id_count)
                .fold(0usize, usize::saturating_add);
            if self.next_cached_dependency_ids.saturating_add(retained) <= MAX_CACHED_DEPENDENCY_IDS
            {
                self.next_cached_dependency_ids =
                    self.next_cached_dependency_ids.saturating_add(retained);
            } else {
                maximal_blocks.clear();
                maximal_blocks_valid = false;
                self.stats.cache_rejections = self.stats.cache_rejections.saturating_add(1);
            }
        }
        self.next_scopes.insert(
            scope.clone(),
            ScopeCache {
                snapshots,
                operation_ids,
                anchors,
                maximal_blocks,
                maximal_blocks_valid,
            },
        );
        Ok(blocks)
    }

    /// Reuses maximal uninterrupted runs for an exact unchanged scope. Changed
    /// scopes are conservatively recollected; their unchanged blocks still hit
    /// the run-scoped block-facts and synthesis caches.
    pub(crate) fn collect_maximal_blocks(
        &mut self,
        scope: &NativeScopeId,
        views: &[OperationView<'_>],
    ) -> Result<Vec<TwoQubitNumericBlock>, CompilerError> {
        let current = self.next_scopes.get_mut(scope).ok_or_else(|| {
            CompilerError::InvariantViolation(
                "maximal collection requested before incremental scope preparation".to_string(),
            )
        })?;
        if self.policy == NativeResynthesisPolicy::Incremental && current.maximal_blocks_valid {
            let id_to_order = current
                .operation_ids
                .iter()
                .copied()
                .enumerate()
                .map(|(order, id)| (id, order))
                .collect::<BTreeMap<_, _>>();
            let blocks = current
                .maximal_blocks
                .iter()
                .map(|block| block.materialize(&id_to_order))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "cached maximal block lost an operation mapping".to_string(),
                    )
                })?;
            self.stats.maximal_blocks_total =
                self.stats.maximal_blocks_total.saturating_add(blocks.len());
            self.stats.maximal_blocks_reused = self
                .stats
                .maximal_blocks_reused
                .saturating_add(blocks.len());
            return Ok(blocks);
        }

        let blocks = collect_maximal_two_qubit_runs(views);
        if self.policy == NativeResynthesisPolicy::Incremental {
            let cached = blocks
                .iter()
                .map(|block| CachedBlock::new(block, &current.operation_ids))
                .collect::<Vec<_>>();
            let retained = cached
                .iter()
                .map(CachedBlock::retained_id_count)
                .fold(0usize, usize::saturating_add);
            if self.next_cached_dependency_ids.saturating_add(retained) <= MAX_CACHED_DEPENDENCY_IDS
            {
                current.maximal_blocks = cached;
                current.maximal_blocks_valid = true;
                self.next_cached_dependency_ids =
                    self.next_cached_dependency_ids.saturating_add(retained);
            } else {
                current.maximal_blocks.clear();
                current.maximal_blocks_valid = false;
                self.stats.cache_rejections = self.stats.cache_rejections.saturating_add(1);
            }
        }
        self.stats.maximal_blocks_total =
            self.stats.maximal_blocks_total.saturating_add(blocks.len());
        self.stats.maximal_blocks_recomputed = self
            .stats
            .maximal_blocks_recomputed
            .saturating_add(blocks.len());
        Ok(blocks)
    }

    #[allow(clippy::type_complexity)]
    fn reconcile(
        &mut self,
        previous: Option<&ScopeCache>,
        current: &[OperationSnapshot],
        config: &TwoQubitBlockResynthesisConfig,
    ) -> (
        Vec<NativeOperationId>,
        BTreeSet<NativeOperationId>,
        BTreeSet<NativeOperationId>,
        BTreeSet<Qubit>,
        bool,
    ) {
        let Some(previous) = previous else {
            let ids = (0..current.len())
                .map(|_| self.allocate_operation_id())
                .collect::<Vec<_>>();
            let inserted = ids.iter().copied().collect();
            let touched = current
                .iter()
                .flat_map(|snapshot| snapshot.qubits.iter().copied())
                .collect();
            return (ids, inserted, BTreeSet::new(), touched, false);
        };

        let mut current_ids = vec![None; current.len()];
        let mut old_matched = vec![false; previous.snapshots.len()];
        let mut prefix = 0;
        while prefix < previous.snapshots.len()
            && prefix < current.len()
            && previous.snapshots[prefix].exact_eq(&current[prefix])
        {
            current_ids[prefix] = Some(previous.operation_ids[prefix]);
            old_matched[prefix] = true;
            prefix += 1;
        }
        let mut old_end = previous.snapshots.len();
        let mut new_end = current.len();
        while old_end > prefix
            && new_end > prefix
            && previous.snapshots[old_end - 1].exact_eq(&current[new_end - 1])
        {
            old_end -= 1;
            new_end -= 1;
            current_ids[new_end] = Some(previous.operation_ids[old_end]);
            old_matched[old_end] = true;
        }

        let mut old_by_hash = HashMap::<u64, Vec<usize>>::new();
        let mut new_by_hash = HashMap::<u64, Vec<usize>>::new();
        for index in prefix..old_end {
            old_by_hash
                .entry(previous.snapshots[index].fast_hash)
                .or_default()
                .push(index);
        }
        for (index, snapshot) in current.iter().enumerate().take(new_end).skip(prefix) {
            new_by_hash
                .entry(snapshot.fast_hash)
                .or_default()
                .push(index);
        }
        let mut candidates = old_by_hash
            .iter()
            .filter_map(|(hash, old)| {
                let new = new_by_hash.get(hash)?;
                (old.len() == 1
                    && new.len() == 1
                    && previous.snapshots[old[0]].exact_eq(&current[new[0]]))
                .then_some((old[0], new[0]))
            })
            .collect::<Vec<_>>();
        candidates.sort_unstable();
        let mut last_new = prefix;
        for (old, new) in candidates {
            if new < last_new {
                continue;
            }
            current_ids[new] = Some(previous.operation_ids[old]);
            old_matched[old] = true;
            last_new = new.saturating_add(1);
        }

        let mut inserted = BTreeSet::new();
        for id in &mut current_ids {
            if id.is_none() {
                let allocated = self.allocate_operation_id();
                *id = Some(allocated);
                inserted.insert(allocated);
            }
        }
        let removed = previous
            .operation_ids
            .iter()
            .copied()
            .zip(&old_matched)
            .filter_map(|(id, matched)| (!*matched).then_some(id))
            .collect::<BTreeSet<_>>();
        let mut touched = BTreeSet::new();
        let mut ambiguous_boundary = false;
        for (snapshot, matched) in previous.snapshots.iter().zip(&old_matched) {
            if !matched {
                touched.extend(snapshot.qubits.iter().copied());
                ambiguous_boundary |= snapshot.is_hard_boundary(config);
            }
        }
        for (snapshot, id) in current.iter().zip(&current_ids) {
            if inserted.contains(&id.expect("all current IDs assigned")) {
                touched.extend(snapshot.qubits.iter().copied());
                ambiguous_boundary |= snapshot.is_hard_boundary(config);
            }
        }
        (
            current_ids
                .into_iter()
                .map(|id| id.expect("all current IDs assigned"))
                .collect(),
            inserted,
            removed,
            touched,
            ambiguous_boundary,
        )
    }

    fn allocate_operation_id(&mut self) -> NativeOperationId {
        let id = NativeOperationId(self.next_operation_id);
        self.next_operation_id = self.next_operation_id.saturating_add(1);
        id
    }

    fn admit_anchor(&mut self, anchor: &CachedAnchor) -> bool {
        let retained_ids = anchor.retained_id_count();
        if self.next_cached_dependency_ids.saturating_add(retained_ids) > MAX_CACHED_DEPENDENCY_IDS
        {
            self.stats.cache_rejections = self.stats.cache_rejections.saturating_add(1);
            return false;
        }
        self.next_cached_dependency_ids =
            self.next_cached_dependency_ids.saturating_add(retained_ids);
        true
    }
}

#[cfg(test)]
#[path = "incremental_test.rs"]
mod incremental_test;
