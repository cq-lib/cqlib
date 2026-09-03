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

//! Candidate collection for two-qubit block resynthesis.
//!
//! Collection starts from each fixed numeric two-qubit standard gate and
//! expands through a temporary dependency DAG. Operations on the same qubit
//! pair, plus fixed numeric one-qubit gates on either qubit, become matched
//! block operations. Other dependency-path operations may be crossed only when
//! they commute exactly with every matched operation they move across.

use super::commutation::{CachedCommutation, OperationView};
use super::config::TwoQubitBlockResynthesisConfig;
use crate::circuit::{Instruction, Qubit, StandardGate};

/// Collector provenance used to select the applicable validation rules.
///
/// This tag is not trusted as a proof by itself: the selector rechecks the
/// origin-specific extraction invariants before synthesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockOrigin {
    /// A linear run closed at every operation touching its qubit pair.
    MaximalClosedRun,
    /// A dependency-closed DAG block whose crossings were proved exact.
    DagDependencyClosed,
}

/// Dependency-closed operation block that can be converted into a 4x4 unitary.
///
/// `matched_orders` are the source operations consumed by numerical synthesis.
/// `crossed_orders` are source operations skipped during collection and emitted
/// unchanged at their original source positions. The selector later verifies
/// that synthesized replacement operations still commute with every crossed
/// operation before accepting a patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TwoQubitNumericBlock {
    /// Canonical qubit order used for matrix construction and synthesis.
    pub qubits: [Qubit; 2],
    /// Source orders included in the synthesized unitary.
    pub matched_orders: Vec<usize>,
    /// Source orders skipped while collecting the block and preserved in place.
    pub crossed_orders: Vec<usize>,
    pub matched_1q_count: usize,
    pub matched_2q_count: usize,
    pub contains_swap: bool,
    origin: BlockOrigin,
}

impl TwoQubitNumericBlock {
    pub(super) fn maximal_closed_run(
        qubits: [Qubit; 2],
        matched_orders: Vec<usize>,
        matched_1q_count: usize,
        matched_2q_count: usize,
        contains_swap: bool,
    ) -> Self {
        Self {
            qubits,
            matched_orders,
            crossed_orders: Vec::new(),
            matched_1q_count,
            matched_2q_count,
            contains_swap,
            origin: BlockOrigin::MaximalClosedRun,
        }
    }

    pub(super) fn dag_dependency_closed(
        qubits: [Qubit; 2],
        matched_orders: Vec<usize>,
        crossed_orders: Vec<usize>,
        matched_1q_count: usize,
        matched_2q_count: usize,
        contains_swap: bool,
    ) -> Self {
        Self {
            qubits,
            matched_orders,
            crossed_orders,
            matched_1q_count,
            matched_2q_count,
            contains_swap,
            origin: BlockOrigin::DagDependencyClosed,
        }
    }

    pub(super) fn origin(&self) -> BlockOrigin {
        self.origin
    }

    pub(crate) fn first_order(&self) -> usize {
        self.matched_orders[0]
    }

    pub(crate) fn last_order(&self) -> usize {
        *self
            .matched_orders
            .last()
            .unwrap_or(&self.matched_orders[0])
    }

    pub(crate) fn span(&self) -> usize {
        self.last_order() - self.first_order() + 1
    }

    pub(crate) fn is_promising(&self) -> bool {
        self.matched_2q_count >= 2
            || self.contains_swap
            || (self.matched_2q_count >= 1 && self.matched_1q_count >= 1)
    }
}

/// Mutable state for one anchor expansion.
///
/// The builder keeps positions rather than cloned operations so all
/// commutation checks and later synthesis use the same resolved
/// [`OperationView`] table.
pub(super) struct BlockBuilder<'a> {
    pub(super) qubits: [Qubit; 2],
    matched_positions: Vec<usize>,
    crossed_positions: Vec<usize>,
    pub(super) ops: &'a [OperationView<'a>],
}

impl<'a> BlockBuilder<'a> {
    pub(super) fn new(qubits: [Qubit; 2], anchor: usize, ops: &'a [OperationView<'a>]) -> Self {
        Self {
            qubits,
            matched_positions: vec![anchor],
            crossed_positions: Vec::new(),
            ops,
        }
    }

    pub(super) fn matched_len(&self) -> usize {
        self.matched_positions.len()
    }

    pub(super) fn crossed_len(&self) -> usize {
        self.crossed_positions.len()
    }

    pub(super) fn add_matched(&mut self, position: usize) {
        self.matched_positions.push(position);
    }

    pub(super) fn add_crossed(&mut self, position: usize) {
        self.crossed_positions.push(position);
    }

    /// Returns whether a same-block candidate can be inserted after already
    /// crossed operations.
    ///
    /// This check is intentionally uncached: these checks are tied to one
    /// partially built block and are not repeated as heavily as source/source
    /// crossing checks.
    pub(super) fn can_add_candidate(
        &self,
        candidate: usize,
        commutation: &CachedCommutation,
    ) -> bool {
        let candidate_view = &self.ops[candidate];
        self.crossed_positions.iter().all(|&crossed| {
            let crossed_view = &self.ops[crossed];
            !shares_any_qubit(crossed_view, candidate_view)
                || commutation.commute_ops_skip_cache(crossed_view, candidate_view)
        })
    }

    /// Returns whether `skipped` can be crossed by all matched block
    /// operations collected so far.
    pub(super) fn can_cross(
        &mut self,
        skipped: usize,
        commutation: &mut CachedCommutation,
    ) -> bool {
        let skipped_view = &self.ops[skipped];
        self.matched_positions.iter().all(|&matched| {
            let matched_view = &self.ops[matched];
            !shares_any_qubit(skipped_view, matched_view)
                || commutation.commute_ops(skipped_view, matched_view)
        })
    }

    pub(super) fn finish(mut self) -> TwoQubitNumericBlock {
        self.matched_positions.sort_unstable();
        self.matched_positions.dedup();
        self.crossed_positions.sort_unstable();
        self.crossed_positions.dedup();

        let mut matched_1q_count = 0;
        let mut matched_2q_count = 0;
        let mut contains_swap = false;
        for &position in &self.matched_positions {
            let op = self.ops[position].operation;
            match op.qubits.len() {
                1 => matched_1q_count += 1,
                2 => matched_2q_count += 1,
                _ => {}
            }
            contains_swap |= matches!(op.instruction, Instruction::Standard(StandardGate::SWAP));
        }

        TwoQubitNumericBlock::dag_dependency_closed(
            self.qubits,
            self.matched_positions,
            self.crossed_positions,
            matched_1q_count,
            matched_2q_count,
            contains_swap,
        )
    }
}

pub(super) fn is_block_candidate(view: &OperationView<'_>, pair: [Qubit; 2]) -> bool {
    if !is_fixed_numeric_standard(view) {
        return false;
    }
    match view.operation.qubits.as_slice() {
        [q] => *q == pair[0] || *q == pair[1],
        [a, b] => (*a == pair[0] && *b == pair[1]) || (*a == pair[1] && *b == pair[0]),
        _ => false,
    }
}

pub(super) fn is_hard_boundary(
    view: &OperationView<'_>,
    config: &TwoQubitBlockResynthesisConfig,
) -> bool {
    if config.skip_labeled_ops && view.operation.label.is_some() {
        return true;
    }
    !matches!(view.operation.instruction, Instruction::Standard(_))
        || view.operation.qubits.len() > 2
        || !is_fixed_numeric_standard(view)
}

pub(super) fn is_fixed_numeric_standard(view: &OperationView<'_>) -> bool {
    let Instruction::Standard(gate) = view.operation.instruction else {
        return false;
    };
    if gate.num_qubits() != view.operation.qubits.len() || gate.num_params() != view.params.len() {
        return false;
    }
    view.params
        .iter()
        .all(|param| param.evaluate(&None).is_ok_and(f64::is_finite))
}

pub(super) fn shares_any_qubit(lhs: &OperationView<'_>, rhs: &OperationView<'_>) -> bool {
    lhs.operation
        .qubits
        .iter()
        .any(|qubit| rhs.operation.qubits.contains(qubit))
}
