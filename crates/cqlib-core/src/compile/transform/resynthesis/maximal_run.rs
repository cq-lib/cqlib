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

//! Linear collection of maximal uninterrupted two-qubit numeric runs.
//!
//! A run owns one unordered qubit pair and contains fixed-parameter, unlabeled
//! standard gates acting on either or both qubits. Operations on disjoint
//! qubits do not interrupt a run. An operation that touches a run qubit but is
//! not eligible for that same pair closes the run immediately.

use super::collector::{TwoQubitNumericBlock, is_fixed_numeric_standard};
use super::commutation::OperationView;
use crate::circuit::{Instruction, Qubit, StandardGate};
use std::collections::HashMap;

#[derive(Debug)]
struct MaximalRun {
    qubits: [Qubit; 2],
    matched_orders: Vec<usize>,
    matched_1q_count: usize,
    matched_2q_count: usize,
    contains_swap: bool,
    active: bool,
}

impl MaximalRun {
    fn new(
        qubits: [Qubit; 2],
        matched_orders: Vec<usize>,
        first_two_qubit: usize,
        gate: StandardGate,
    ) -> Self {
        let matched_1q_count = matched_orders.len();
        let mut matched_orders = matched_orders;
        matched_orders.push(first_two_qubit);
        Self {
            qubits,
            matched_orders,
            matched_1q_count,
            matched_2q_count: 1,
            contains_swap: gate == StandardGate::SWAP,
            active: true,
        }
    }

    fn add_one_qubit(&mut self, order: usize) {
        self.matched_orders.push(order);
        self.matched_1q_count += 1;
    }

    fn add_two_qubit(&mut self, order: usize, gate: StandardGate) {
        self.matched_orders.push(order);
        self.matched_2q_count += 1;
        self.contains_swap |= gate == StandardGate::SWAP;
    }

    fn finish(mut self) -> TwoQubitNumericBlock {
        self.matched_orders.sort_unstable();
        TwoQubitNumericBlock::maximal_closed_run(
            self.qubits,
            self.matched_orders,
            self.matched_1q_count,
            self.matched_2q_count,
            self.contains_swap,
        )
    }
}

/// Collects each maximal uninterrupted pair run exactly once.
///
/// The source stream is visited once. Single-qubit gates before the first
/// two-qubit gate remain pending and are admitted only if a compatible pair is
/// subsequently opened. When a qubit changes partners, its existing run is
/// closed before the new pair is opened, so source operations never belong to
/// overlapping maximal runs.
pub(super) fn collect_maximal_two_qubit_runs(
    ops: &[OperationView<'_>],
) -> Vec<TwoQubitNumericBlock> {
    let mut runs = Vec::<MaximalRun>::new();
    let mut active_by_qubit = HashMap::<Qubit, usize>::new();
    let mut pending_by_qubit = HashMap::<Qubit, Vec<usize>>::new();
    let mut last_boundary_by_qubit = HashMap::<Qubit, usize>::new();
    let mut global_boundary = None;

    for (order, view) in ops.iter().enumerate() {
        debug_assert_eq!(view.order, order);

        let eligible = view.operation.label.is_none() && is_fixed_numeric_standard(view);
        if !eligible {
            close_touched_runs(
                &mut runs,
                &mut active_by_qubit,
                &mut pending_by_qubit,
                &mut last_boundary_by_qubit,
                &mut global_boundary,
                &view.operation.qubits,
                order,
            );
            continue;
        }

        let Instruction::Standard(gate) = view.operation.instruction else {
            unreachable!("fixed numeric standard predicate accepted a non-standard instruction");
        };
        match view.operation.qubits.as_slice() {
            [qubit] => {
                if let Some(&run_id) = active_by_qubit.get(qubit) {
                    runs[run_id].add_one_qubit(order);
                } else {
                    pending_by_qubit.entry(*qubit).or_default().push(order);
                }
            }
            [left, right] => {
                let existing = active_by_qubit
                    .get(left)
                    .copied()
                    .filter(|run_id| active_by_qubit.get(right) == Some(run_id))
                    .filter(|run_id| same_pair(runs[*run_id].qubits, [*left, *right]));
                if let Some(run_id) = existing {
                    runs[run_id].add_two_qubit(order, gate);
                    continue;
                }

                close_run_on_qubit(
                    &mut runs,
                    &mut active_by_qubit,
                    &mut last_boundary_by_qubit,
                    *left,
                );
                close_run_on_qubit(
                    &mut runs,
                    &mut active_by_qubit,
                    &mut last_boundary_by_qubit,
                    *right,
                );

                let mut prefix = pending_by_qubit.remove(left).unwrap_or_default();
                prefix.extend(pending_by_qubit.remove(right).unwrap_or_default());
                let cutoff = [
                    global_boundary,
                    last_boundary_by_qubit.get(left).copied(),
                    last_boundary_by_qubit.get(right).copied(),
                ]
                .into_iter()
                .flatten()
                .max();
                prefix.retain(|prefix_order| cutoff.is_none_or(|cutoff| *prefix_order > cutoff));
                prefix.sort_unstable();
                let run_id = runs.len();
                let run = MaximalRun::new([*left, *right], prefix, order, gate);
                runs.push(run);
                active_by_qubit.insert(*left, run_id);
                active_by_qubit.insert(*right, run_id);
            }
            [] => {
                // A zero-qubit instruction has scope-wide semantics. Keeping it
                // outside every local unitary requires closing all open runs.
                close_all_runs(
                    &mut runs,
                    &mut active_by_qubit,
                    &mut pending_by_qubit,
                    &mut last_boundary_by_qubit,
                );
                global_boundary = Some(order);
            }
            _ => close_touched_runs(
                &mut runs,
                &mut active_by_qubit,
                &mut pending_by_qubit,
                &mut last_boundary_by_qubit,
                &mut global_boundary,
                &view.operation.qubits,
                order,
            ),
        }
    }

    runs.into_iter()
        .map(MaximalRun::finish)
        .filter(TwoQubitNumericBlock::is_promising)
        .collect()
}

fn same_pair(lhs: [Qubit; 2], rhs: [Qubit; 2]) -> bool {
    (lhs[0] == rhs[0] && lhs[1] == rhs[1]) || (lhs[0] == rhs[1] && lhs[1] == rhs[0])
}

fn close_touched_runs(
    runs: &mut [MaximalRun],
    active_by_qubit: &mut HashMap<Qubit, usize>,
    pending_by_qubit: &mut HashMap<Qubit, Vec<usize>>,
    last_boundary_by_qubit: &mut HashMap<Qubit, usize>,
    global_boundary: &mut Option<usize>,
    qubits: &[Qubit],
    order: usize,
) {
    if qubits.is_empty() {
        close_all_runs(
            runs,
            active_by_qubit,
            pending_by_qubit,
            last_boundary_by_qubit,
        );
        *global_boundary = Some(order);
        return;
    }
    for &qubit in qubits {
        close_run_on_qubit(runs, active_by_qubit, last_boundary_by_qubit, qubit);
        pending_by_qubit.remove(&qubit);
        last_boundary_by_qubit.insert(qubit, order);
    }
}

fn close_run_on_qubit(
    runs: &mut [MaximalRun],
    active_by_qubit: &mut HashMap<Qubit, usize>,
    last_boundary_by_qubit: &mut HashMap<Qubit, usize>,
    qubit: Qubit,
) {
    let Some(run_id) = active_by_qubit.remove(&qubit) else {
        return;
    };
    if !runs[run_id].active {
        return;
    }
    runs[run_id].active = false;
    let last_order = *runs[run_id]
        .matched_orders
        .last()
        .expect("an active maximal run contains its opening two-qubit gate");
    for pair_qubit in runs[run_id].qubits {
        if active_by_qubit.get(&pair_qubit) == Some(&run_id) {
            active_by_qubit.remove(&pair_qubit);
        }
        last_boundary_by_qubit
            .entry(pair_qubit)
            .and_modify(|boundary| *boundary = (*boundary).max(last_order))
            .or_insert(last_order);
    }
}

fn close_all_runs(
    runs: &mut [MaximalRun],
    active_by_qubit: &mut HashMap<Qubit, usize>,
    pending_by_qubit: &mut HashMap<Qubit, Vec<usize>>,
    last_boundary_by_qubit: &mut HashMap<Qubit, usize>,
) {
    for run in runs {
        if !run.active {
            continue;
        }
        run.active = false;
        let last_order = *run
            .matched_orders
            .last()
            .expect("an active maximal run contains its opening two-qubit gate");
        for qubit in run.qubits {
            last_boundary_by_qubit
                .entry(qubit)
                .and_modify(|boundary| *boundary = (*boundary).max(last_order))
                .or_insert(last_order);
        }
    }
    active_by_qubit.clear();
    pending_by_qubit.clear();
}

#[cfg(test)]
#[path = "maximal_run_test.rs"]
mod maximal_run_test;
