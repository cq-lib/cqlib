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

//! Commutation-set grouping and cancellation-key analysis.
//!
//! The analysis mirrors qiskit's `CommutativeCancellation` structure:
//! every wire's operation sequence is greedily partitioned into contiguous
//! sets of pairwise exactly-commuting operations, and self-inverse gates are
//! bucketed by a key of `(gate, canonical qargs, per-wire set indices)`.
//! Two operations share a key only when everything between them on each of
//! their wires commutes exactly with them, so removing `2 * floor(n / 2)`
//! members of a bucket of `n` identical self-inverse gates is an exact
//! identity with zero global phase.
//!
//! Sets are tracked per wire independently; a two-qubit gate therefore
//! records one set index per qubit and the key keeps both as a tuple.

use crate::circuit::{Instruction, Operation, Parameter, Qubit, StandardGate};
use crate::compile::commutation::{Commutation, CommutationChecker};
use smallvec::SmallVec;
use std::collections::HashMap;

/// Self-inverse standard gates cancelled by this pass.
///
/// Every pair of identical gates from this set cancels to an exact identity
/// with zero global phase.
pub(crate) const SELF_INVERSE_GATES: [StandardGate; 7] = [
    StandardGate::CX,
    StandardGate::CY,
    StandardGate::CZ,
    StandardGate::H,
    StandardGate::Y,
    StandardGate::X,
    StandardGate::Z,
];

/// Returns whether the instruction is a unitary gate-like operation.
///
/// This must match instruction variants explicitly: `gate_arity()` also
/// reports arities for `Measure`, `Reset`, `Delay`, and classical data
/// operations, so it cannot be used to delimit unitary blocks.
pub(crate) fn is_unitary_gate_like(instruction: &Instruction) -> bool {
    matches!(
        instruction,
        Instruction::Standard(_)
            | Instruction::McGate(_)
            | Instruction::UnitaryGate(_)
            | Instruction::CircuitGate(_)
    )
}

/// Returns whether the gate is a self-inverse cancellation candidate.
pub(crate) fn is_self_inverse_candidate(gate: StandardGate) -> bool {
    SELF_INVERSE_GATES.contains(&gate)
}

/// A block operation with parameters resolved once against the source circuit.
///
/// Symbolic parameters stay symbolic; they may still participate in
/// commutation-set membership proofs that are parameter independent.
#[derive(Debug)]
pub(crate) struct OperationView<'a> {
    /// Position of the operation inside its flat block.
    pub(crate) order: usize,
    /// The source operation.
    pub(crate) op: &'a Operation,
    /// Parameters resolved once via `Circuit::resolve_parameter`.
    pub(crate) params: SmallVec<[Parameter; 3]>,
}

/// Cancellation bucket key for self-inverse gates.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CancellationKey {
    OneQ {
        gate: StandardGate,
        qubit: Qubit,
        set_index: usize,
    },
    TwoQ {
        gate: StandardGate,
        qargs: [Qubit; 2],
        set_indices: [usize; 2],
    },
}

/// Candidate identity used by the linear pre-scan: gate plus canonical qargs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CandidateId {
    OneQ(StandardGate, Qubit),
    TwoQ(StandardGate, [Qubit; 2]),
}

/// Per-wire current commutation set.
struct WireSet {
    index: usize,
    members: Vec<usize>,
}

/// Checks whether two block operations commute exactly.
fn exact_commutes(checker: &CommutationChecker, lhs: &OperationView, rhs: &OperationView) -> bool {
    matches!(
        checker.check(
            &lhs.op.instruction,
            &lhs.op.qubits,
            &lhs.params,
            &rhs.op.instruction,
            &rhs.op.qubits,
            &rhs.params,
        ),
        Some(Commutation::Exact)
    )
}

/// Returns the candidate identity of a view, if it is a cancellation candidate.
fn candidate_id(view: &OperationView) -> Option<CandidateId> {
    let gate = view.op.instruction.standard_gate()?;
    if !is_self_inverse_candidate(gate) {
        return None;
    }
    match view.op.qubits.as_slice() {
        [qubit] => Some(CandidateId::OneQ(gate, *qubit)),
        [first, second] => {
            let qargs = if gate.is_invariant_under_operand_swap() && first > second {
                [*second, *first]
            } else {
                [*first, *second]
            };
            Some(CandidateId::TwoQ(gate, qargs))
        }
        _ => None,
    }
}

/// Returns the cancellation key of a candidate view.
fn cancellation_key(view: &OperationView, set_indices: &[usize]) -> Option<CancellationKey> {
    let gate = view.op.instruction.standard_gate()?;
    if !is_self_inverse_candidate(gate) {
        return None;
    }
    match view.op.qubits.as_slice() {
        [qubit] => Some(CancellationKey::OneQ {
            gate,
            qubit: *qubit,
            set_index: set_indices[0],
        }),
        [first, second] => {
            let pair = [(*first, set_indices[0]), (*second, set_indices[1])];
            let (qargs, indices) = if gate.is_invariant_under_operand_swap() && pair[0] > pair[1] {
                ([pair[1].0, pair[0].0], [pair[1].1, pair[0].1])
            } else {
                ([pair[0].0, pair[1].0], [pair[0].1, pair[1].1])
            };
            Some(CancellationKey::TwoQ {
                gate,
                qargs,
                set_indices: indices,
            })
        }
        _ => None,
    }
}

/// Finds block operations removable by self-inverse pair cancellation.
///
/// Returns one flag per view marking the views to delete. The analysis runs
/// in a single pass per block:
///
/// 1. a linear pre-scan skips blocks without any repeated candidate pair;
/// 2. every wire's view sequence is greedily partitioned into commutation
///    sets (a view joins a wire's current set only if it commutes exactly
///    with every current member);
/// 3. candidates are bucketed by cancellation key and each bucket drops its
///    last `2 * floor(n / 2)` members, keeping the first member when odd.
///
/// Only deletions are ever produced, so repeated runs of the pass converge.
pub(crate) fn find_cancellable_ops(
    checker: &CommutationChecker,
    views: &[OperationView],
) -> Vec<bool> {
    let mut candidate_counts: HashMap<CandidateId, usize> = HashMap::new();
    for view in views {
        if let Some(id) = candidate_id(view) {
            *candidate_counts.entry(id).or_default() += 1;
        }
    }
    if candidate_counts.values().all(|count| *count < 2) {
        return vec![false; views.len()];
    }

    let mut wire_sets: HashMap<Qubit, WireSet> = HashMap::new();
    let mut view_set_indices: Vec<SmallVec<[usize; 3]>> = Vec::with_capacity(views.len());
    for view in views {
        let mut indices = SmallVec::with_capacity(view.op.qubits.len());
        for qubit in view.op.qubits.iter().copied() {
            let wire = wire_sets.entry(qubit).or_insert_with(|| WireSet {
                index: 0,
                members: Vec::new(),
            });
            let joins = wire
                .members
                .iter()
                .all(|member| exact_commutes(checker, view, &views[*member]));
            if !joins {
                wire.index += 1;
                wire.members.clear();
            }
            wire.members.push(view.order);
            indices.push(wire.index);
        }
        view_set_indices.push(indices);
    }

    let mut buckets: HashMap<CancellationKey, Vec<usize>> = HashMap::new();
    for view in views {
        if let Some(key) = cancellation_key(view, &view_set_indices[view.order]) {
            buckets.entry(key).or_default().push(view.order);
        }
    }

    let mut deleted = vec![false; views.len()];
    for members in buckets.values() {
        let keep = members.len() % 2;
        for order in &members[keep..] {
            deleted[*order] = true;
        }
    }
    deleted
}
