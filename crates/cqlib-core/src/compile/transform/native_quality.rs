// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Native-loop-specific quality policy and exact scope diagnostics.

use crate::circuit::{Instruction, Operation, Qubit, ValueInstruction, ValueOperation};
use crate::compile::device_planning::cost::{
    DevicePhysicalCost, MetricAvailability, NativePlanLeaf, RobustDurationKey, RobustErrorKey,
};
use smallvec::SmallVec;
use std::cmp::Ordering;
use std::collections::HashMap;

/// Tolerance for harmless floating-point accumulation differences in predicted
/// log error. This is deliberately an absolute tolerance on the additive log
/// domain, rather than a percentage that would permit larger regressions on
/// noisier circuits.
const ERROR_LOG_TOLERANCE: f64 = 1e-12;

/// Candidate acceptance and ranking policy used by [`super::NativeOptimizer`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum NativeQualityPolicy {
    /// Preserve the historical global device-cost ordering.
    #[default]
    EntanglerFirst,
    /// Protect entangler count/depth and total depth before ranking structural quality.
    BalancedDepth,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NativeCriticalPathQuality {
    /// Largest one-qubit population among all unit-duration critical paths.
    pub(crate) one_qubit_ops: u32,
    /// Longest consecutive one-qubit segment on any unit-duration critical path.
    pub(crate) longest_one_qubit_run: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NativeQualityVector {
    pub(crate) physical: DevicePhysicalCost,
    pub(crate) critical_path: NativeCriticalPathQuality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeQualityViolation {
    ScopeShape,
    TwoQubitOps,
    TwoQubitDepth,
    TotalDepth,
    Error,
    Makespan,
    Rank,
}

impl NativeQualityVector {
    pub(crate) fn for_operations(physical: DevicePhysicalCost, operations: &[Operation]) -> Self {
        let mut path = CriticalPathBuilder::default();
        for operation in operations {
            if matches!(
                operation.instruction,
                Instruction::Standard(_) | Instruction::McGate(_)
            ) {
                path.add_gate(&operation.qubits);
            }
        }
        Self {
            physical,
            critical_path: path.finish(),
        }
    }

    pub(crate) fn for_value_operations(
        physical: DevicePhysicalCost,
        operations: &[ValueOperation],
    ) -> Self {
        let mut path = CriticalPathBuilder::default();
        for operation in operations {
            if matches!(operation.instruction, ValueInstruction::Instruction(_)) {
                path.add_gate(&operation.qubits);
            }
        }
        Self {
            physical,
            critical_path: path.finish(),
        }
    }

    /// Builds structural metrics from the exact native leaves selected by the
    /// device planner, rather than from a higher-level synthesis candidate that
    /// may expand into several physical one-qubit gates during legalization.
    pub(crate) fn for_native_plan_leaves(
        physical: DevicePhysicalCost,
        leaves: &[NativePlanLeaf],
    ) -> Self {
        let mut path = CriticalPathBuilder::default();
        for leaf in leaves {
            let qubits = leaf
                .ordered_qargs
                .iter()
                .copied()
                .map(Qubit::from)
                .collect::<SmallVec<[Qubit; 2]>>();
            path.add_gate(&qubits);
        }
        Self {
            physical,
            critical_path: path.finish(),
        }
    }

    /// Returns whether the candidate remains inside the immutable entry envelope.
    pub(crate) fn admissible_against(self, entry: Self, policy: NativeQualityPolicy) -> bool {
        self.admissibility_violation_against(entry, policy)
            .is_none()
    }

    /// Returns the first immutable-entry protection violated by the candidate.
    pub(crate) fn admissibility_violation_against(
        self,
        entry: Self,
        policy: NativeQualityPolicy,
    ) -> Option<NativeQualityViolation> {
        match policy {
            NativeQualityPolicy::EntanglerFirst => None,
            NativeQualityPolicy::BalancedDepth => {
                if self.physical.native_two_qubit_ops > entry.physical.native_two_qubit_ops {
                    return Some(NativeQualityViolation::TwoQubitOps);
                }
                if self.physical.native_two_qubit_depth > entry.physical.native_two_qubit_depth {
                    return Some(NativeQualityViolation::TwoQubitDepth);
                }
                if self.physical.total_native_depth > entry.physical.total_native_depth {
                    return Some(NativeQualityViolation::TotalDepth);
                }
                if !error_is_no_worse(self.physical.error, entry.physical.error) {
                    return Some(NativeQualityViolation::Error);
                }
                if !makespan_is_no_worse(self.physical.makespan, entry.physical.makespan) {
                    return Some(NativeQualityViolation::Makespan);
                }
                None
            }
        }
    }

    /// Orders candidates under one Native-only policy. Lower is better.
    pub(crate) fn compare(self, other: Self, policy: NativeQualityPolicy) -> Ordering {
        match policy {
            NativeQualityPolicy::EntanglerFirst => self.physical.compare(other.physical),
            NativeQualityPolicy::BalancedDepth => self
                .physical
                .total_native_depth
                .cmp(&other.physical.total_native_depth)
                .then_with(|| {
                    self.physical
                        .makespan
                        .compare_by(other.physical.makespan, |left, right| {
                            left.total_cmp(&right)
                        })
                })
                .then_with(|| {
                    self.critical_path
                        .one_qubit_ops
                        .cmp(&other.critical_path.one_qubit_ops)
                })
                .then_with(|| {
                    self.critical_path
                        .longest_one_qubit_run
                        .cmp(&other.critical_path.longest_one_qubit_run)
                })
                .then_with(|| {
                    self.physical
                        .native_two_qubit_depth
                        .cmp(&other.physical.native_two_qubit_depth)
                })
                .then_with(|| {
                    self.physical
                        .native_two_qubit_ops
                        .cmp(&other.physical.native_two_qubit_ops)
                })
                .then_with(|| {
                    self.physical
                        .native_total_ops
                        .cmp(&other.physical.native_total_ops)
                })
                .then_with(|| {
                    self.physical
                        .error
                        .compare_by(other.physical.error, |left, right| left.compare(right))
                })
                .then_with(|| {
                    self.physical
                        .duration
                        .compare_by(other.physical.duration, |left, right| left.compare(right))
                }),
        }
    }

    /// Returns Pareto dominance against one exact control-flow scope.
    ///
    /// `Some(false)` means equality across every protected component,
    /// `Some(true)` means no component is worse and at least one is strictly
    /// better, and `None` means the vectors are incomparable or `self`
    /// regresses at least one component. Optional metric availability must
    /// match exactly so missing calibration data can never masquerade as an
    /// improvement.
    pub(crate) fn exact_pareto_dominance(self, other: Self) -> Option<bool> {
        let mut strict = false;
        dominance_component(
            self.physical.native_two_qubit_ops,
            other.physical.native_two_qubit_ops,
            &mut strict,
        )?;
        dominance_component(
            self.physical.native_two_qubit_depth,
            other.physical.native_two_qubit_depth,
            &mut strict,
        )?;
        dominance_component(
            self.physical.total_native_depth,
            other.physical.total_native_depth,
            &mut strict,
        )?;
        dominance_component(
            self.physical.native_total_ops,
            other.physical.native_total_ops,
            &mut strict,
        )?;
        strict |= error_dominance(self.physical.error, other.physical.error)?;
        strict |= duration_dominance(self.physical.duration, other.physical.duration)?;
        strict |= scalar_availability_dominance(self.physical.makespan, other.physical.makespan)?;
        dominance_component(
            self.critical_path.one_qubit_ops,
            other.critical_path.one_qubit_ops,
            &mut strict,
        )?;
        dominance_component(
            self.critical_path.longest_one_qubit_run,
            other.critical_path.longest_one_qubit_run,
            &mut strict,
        )?;
        Some(strict)
    }
}

fn dominance_component<T: Ord>(candidate: T, incumbent: T, strict: &mut bool) -> Option<()> {
    match candidate.cmp(&incumbent) {
        Ordering::Less => *strict = true,
        Ordering::Equal => {}
        Ordering::Greater => return None,
    }
    Some(())
}

fn error_dominance(
    candidate: MetricAvailability<RobustErrorKey>,
    incumbent: MetricAvailability<RobustErrorKey>,
) -> Option<bool> {
    match (candidate, incumbent) {
        (MetricAvailability::Disabled, MetricAvailability::Disabled)
        | (MetricAvailability::Inconsistent, MetricAvailability::Inconsistent) => Some(false),
        (MetricAvailability::Available(candidate), MetricAvailability::Available(incumbent)) => {
            if !error_is_no_worse(
                MetricAvailability::Available(candidate),
                MetricAvailability::Available(incumbent),
            ) {
                return None;
            }
            Some(
                candidate.unavailable_count < incumbent.unavailable_count
                    || candidate.imputed_count < incumbent.imputed_count
                    || candidate.log_error + ERROR_LOG_TOLERANCE < incumbent.log_error,
            )
        }
        _ => None,
    }
}

fn duration_dominance(
    candidate: MetricAvailability<RobustDurationKey>,
    incumbent: MetricAvailability<RobustDurationKey>,
) -> Option<bool> {
    match (candidate, incumbent) {
        (MetricAvailability::Disabled, MetricAvailability::Disabled)
        | (MetricAvailability::Inconsistent, MetricAvailability::Inconsistent) => Some(false),
        (MetricAvailability::Available(candidate), MetricAvailability::Available(incumbent)) => {
            if candidate.unavailable_count > incumbent.unavailable_count
                || candidate.imputed_count > incumbent.imputed_count
                || candidate
                    .duration_work
                    .total_cmp(&incumbent.duration_work)
                    .is_gt()
            {
                return None;
            }
            Some(
                candidate.unavailable_count < incumbent.unavailable_count
                    || candidate.imputed_count < incumbent.imputed_count
                    || candidate
                        .duration_work
                        .total_cmp(&incumbent.duration_work)
                        .is_lt(),
            )
        }
        _ => None,
    }
}

fn scalar_availability_dominance(
    candidate: MetricAvailability<f64>,
    incumbent: MetricAvailability<f64>,
) -> Option<bool> {
    match (candidate, incumbent) {
        (MetricAvailability::Disabled, MetricAvailability::Disabled)
        | (MetricAvailability::Inconsistent, MetricAvailability::Inconsistent) => Some(false),
        (MetricAvailability::Available(candidate), MetricAvailability::Available(incumbent)) => {
            match candidate.total_cmp(&incumbent) {
                Ordering::Less => Some(true),
                Ordering::Equal => Some(false),
                Ordering::Greater => None,
            }
        }
        _ => None,
    }
}

fn error_is_no_worse(
    candidate: MetricAvailability<RobustErrorKey>,
    entry: MetricAvailability<RobustErrorKey>,
) -> bool {
    match (candidate, entry) {
        (MetricAvailability::Disabled, MetricAvailability::Disabled)
        | (MetricAvailability::Inconsistent, MetricAvailability::Inconsistent) => true,
        (MetricAvailability::Available(candidate), MetricAvailability::Available(entry)) => {
            candidate.unavailable_count <= entry.unavailable_count
                && candidate.imputed_count <= entry.imputed_count
                && (candidate.log_error <= entry.log_error
                    || candidate.log_error - entry.log_error <= ERROR_LOG_TOLERANCE)
        }
        _ => false,
    }
}

fn makespan_is_no_worse(
    candidate: MetricAvailability<f64>,
    entry: MetricAvailability<f64>,
) -> bool {
    match (candidate, entry) {
        (MetricAvailability::Disabled, MetricAvailability::Disabled)
        | (MetricAvailability::Inconsistent, MetricAvailability::Inconsistent) => true,
        (MetricAvailability::Available(candidate), MetricAvailability::Available(entry)) => {
            candidate <= entry
        }
        _ => false,
    }
}

#[derive(Debug)]
struct PathNode {
    one_qubit: bool,
    predecessors: SmallVec<[usize; 3]>,
    successors: SmallVec<[usize; 3]>,
    forward_depth: u32,
    backward_depth: u32,
}

#[derive(Default)]
pub(super) struct CriticalPathBuilder {
    nodes: Vec<PathNode>,
    last_on_qubit: HashMap<Qubit, usize>,
}

impl CriticalPathBuilder {
    pub(super) fn add_gate(&mut self, qubits: &[Qubit]) {
        if qubits.is_empty() {
            return;
        }
        let mut predecessors = SmallVec::<[usize; 3]>::new();
        for qubit in qubits {
            if let Some(predecessor) = self.last_on_qubit.get(qubit).copied()
                && !predecessors.contains(&predecessor)
            {
                predecessors.push(predecessor);
            }
        }
        let forward_depth = predecessors
            .iter()
            .map(|predecessor| self.nodes[*predecessor].forward_depth)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let index = self.nodes.len();
        for predecessor in &predecessors {
            self.nodes[*predecessor].successors.push(index);
        }
        self.nodes.push(PathNode {
            one_qubit: qubits.len() == 1,
            predecessors,
            successors: SmallVec::new(),
            forward_depth,
            backward_depth: 1,
        });
        for qubit in qubits {
            self.last_on_qubit.insert(*qubit, index);
        }
    }

    pub(super) fn finish(mut self) -> NativeCriticalPathQuality {
        let total_depth = self
            .nodes
            .iter()
            .map(|node| node.forward_depth)
            .max()
            .unwrap_or(0);
        for index in (0..self.nodes.len()).rev() {
            self.nodes[index].backward_depth = self.nodes[index]
                .successors
                .iter()
                .map(|successor| self.nodes[*successor].backward_depth)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
        }

        let mut one_qubit_counts = vec![0_u32; self.nodes.len()];
        let mut current_one_qubit_runs = vec![0_u32; self.nodes.len()];
        let mut longest_one_qubit_run = 0_u32;
        for index in 0..self.nodes.len() {
            if !is_critical(&self.nodes[index], total_depth) {
                continue;
            }
            let tight_predecessors =
                self.nodes[index]
                    .predecessors
                    .iter()
                    .copied()
                    .filter(|predecessor| {
                        is_critical(&self.nodes[*predecessor], total_depth)
                            && self.nodes[*predecessor].forward_depth.saturating_add(1)
                                == self.nodes[index].forward_depth
                    });
            let mut best_count = 0_u32;
            let mut best_run = 0_u32;
            for predecessor in tight_predecessors {
                best_count = best_count.max(one_qubit_counts[predecessor]);
                if self.nodes[predecessor].one_qubit {
                    best_run = best_run.max(current_one_qubit_runs[predecessor]);
                }
            }
            if self.nodes[index].one_qubit {
                one_qubit_counts[index] = best_count.saturating_add(1);
                current_one_qubit_runs[index] = best_run.saturating_add(1);
                longest_one_qubit_run = longest_one_qubit_run.max(current_one_qubit_runs[index]);
            } else {
                one_qubit_counts[index] = best_count;
            }
        }

        let one_qubit_ops = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.forward_depth == total_depth)
            .map(|(index, _)| one_qubit_counts[index])
            .max()
            .unwrap_or(0);
        NativeCriticalPathQuality {
            one_qubit_ops,
            longest_one_qubit_run,
        }
    }
}

fn is_critical(node: &PathNode, total_depth: u32) -> bool {
    node.forward_depth
        .saturating_add(node.backward_depth)
        .saturating_sub(1)
        == total_depth
}
