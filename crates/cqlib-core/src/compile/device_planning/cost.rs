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

//! Shared calibration and physical-cost evaluation for exact device plans.

use crate::circuit::{Instruction, StandardGate};
use crate::compile::knowledge::KnowledgeInstructionKey;
use crate::device::{Device, PhysicalQubit};
use smallvec::SmallVec;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// One exact native leaf in a selected device-lowering plan.
#[derive(Debug, Clone)]
pub(crate) struct NativePlanLeaf {
    pub(crate) instruction: Instruction,
    pub(crate) ordered_qargs: SmallVec<[PhysicalQubit; 2]>,
    pub(crate) error_rate: Option<f64>,
    pub(crate) duration: Option<f64>,
}

/// Exact native resource summary for a selected device-lowering plan.
#[derive(Debug, Clone)]
pub(crate) struct NativePlanSummary {
    pub(crate) native_two_qubit_ops: u32,
    pub(crate) native_total_ops: u32,
    pub(crate) leaves: Vec<NativePlanLeaf>,
}

/// Max-plus transition induced by one leaf sequence on its exact qargs.
///
/// Unlike a scalar depth, this transition remains valid when the sequence is
/// appended after an arbitrary prefix with unequal per-qubit readiness.
#[derive(Debug, Clone, PartialEq)]
struct MaxPlusProfile {
    dimension: usize,
    entries: SmallVec<[f64; 4]>,
}

impl MaxPlusProfile {
    fn identity(dimension: usize) -> Self {
        let mut entries = SmallVec::<[f64; 4]>::new();
        entries.resize(dimension * dimension, f64::NEG_INFINITY);
        for index in 0..dimension {
            entries[index * dimension + index] = 0.0;
        }
        Self { dimension, entries }
    }

    fn apply(&mut self, qargs: &[usize], weight: f64) {
        if qargs.is_empty() {
            return;
        }
        let mut merged = vec![f64::NEG_INFINITY; self.dimension];
        for &qarg in qargs {
            let row = &self.entries[qarg * self.dimension..(qarg + 1) * self.dimension];
            for (column, value) in row.iter().copied().enumerate() {
                merged[column] = merged[column].max(value);
            }
        }
        for value in &mut merged {
            *value += weight;
        }
        for &qarg in qargs {
            self.entries[qarg * self.dimension..(qarg + 1) * self.dimension]
                .copy_from_slice(&merged);
        }
    }

    /// Appends `next` to this transition.
    fn append(&mut self, next: &Self) {
        debug_assert_eq!(self.dimension, next.dimension);
        let dimension = self.dimension;
        let mut entries = SmallVec::<[f64; 4]>::new();
        entries.resize(dimension * dimension, f64::NEG_INFINITY);
        for row in 0..dimension {
            for column in 0..dimension {
                let mut value = f64::NEG_INFINITY;
                for middle in 0..dimension {
                    value = value.max(
                        next.entries[row * dimension + middle]
                            + self.entries[middle * dimension + column],
                    );
                }
                entries[row * dimension + column] = value;
            }
        }
        self.entries = entries;
    }

    fn embedded(&self, dimension: usize, positions: &[usize]) -> Self {
        debug_assert_eq!(self.dimension, positions.len());
        let mut embedded = Self::identity(dimension);
        for &row in positions {
            embedded.entries[row * dimension..(row + 1) * dimension].fill(f64::NEG_INFINITY);
        }
        for (source_row, &target_row) in positions.iter().enumerate() {
            for (source_column, &target_column) in positions.iter().enumerate() {
                embedded.entries[target_row * dimension + target_column] =
                    self.entries[source_row * self.dimension + source_column];
            }
        }
        embedded
    }

    fn output_maximum(&self) -> f64 {
        self.entries
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Returns whether `self` is no worse for every possible input readiness,
    /// together with whether it is strictly better for at least one input.
    fn dominance(&self, other: &Self) -> Option<bool> {
        if self.dimension != other.dimension {
            return None;
        }
        let mut strict = false;
        for (left, right) in self.entries.iter().zip(&other.entries) {
            match left.total_cmp(right) {
                Ordering::Greater => return None,
                Ordering::Less => strict = true,
                Ordering::Equal => {}
            }
        }
        Some(strict)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ScheduleAvailability {
    Disabled,
    Available(MaxPlusProfile),
    Inconsistent,
}

/// Context-independent scheduling information used for safe Pareto pruning.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeviceScheduleProfile {
    total_depth: MaxPlusProfile,
    two_qubit_depth: MaxPlusProfile,
    makespan: ScheduleAvailability,
}

impl DeviceScheduleProfile {
    /// Appends a child transition after the current sequence, preserving the
    /// child's ordered roles inside the parent's exact qargs.
    pub(crate) fn append_child(
        &mut self,
        child: &Self,
        child_qargs: &[PhysicalQubit],
        parent_qargs: &[PhysicalQubit],
    ) -> Result<(), String> {
        if child_qargs.len() != child.total_depth.dimension {
            return Err(format!(
                "device schedule profile dimension does not match child qargs {child_qargs:?}"
            ));
        }
        let positions = child_qargs
            .iter()
            .map(|child| {
                parent_qargs
                    .iter()
                    .position(|parent| parent == child)
                    .ok_or_else(|| {
                        format!(
                            "child schedule qarg {child:?} is outside parent qargs {parent_qargs:?}"
                        )
                    })
            })
            .collect::<Result<SmallVec<[usize; 2]>, _>>()?;
        if positions
            .iter()
            .enumerate()
            .any(|(index, position)| positions[..index].contains(position))
        {
            return Err(format!(
                "child schedule has duplicate qargs {child_qargs:?} inside {parent_qargs:?}"
            ));
        }

        self.total_depth
            .append(&child.total_depth.embedded(parent_qargs.len(), &positions));
        self.two_qubit_depth.append(
            &child
                .two_qubit_depth
                .embedded(parent_qargs.len(), &positions),
        );
        self.makespan = match (&self.makespan, &child.makespan) {
            (ScheduleAvailability::Disabled, ScheduleAvailability::Disabled) => {
                ScheduleAvailability::Disabled
            }
            (ScheduleAvailability::Available(current), ScheduleAvailability::Available(next)) => {
                let mut current = current.clone();
                current.append(&next.embedded(parent_qargs.len(), &positions));
                ScheduleAvailability::Available(current)
            }
            _ => ScheduleAvailability::Inconsistent,
        };
        Ok(())
    }

    pub(crate) fn physical_cost(&self, aggregate: NativePlanCost) -> DevicePhysicalCost {
        DevicePhysicalCost {
            native_two_qubit_ops: aggregate.native_two_qubit_ops,
            native_two_qubit_depth: self.two_qubit_depth.output_maximum() as u32,
            error: aggregate.error,
            total_native_depth: self.total_depth.output_maximum() as u32,
            native_total_ops: aggregate.native_total_ops,
            duration: aggregate.duration,
            makespan: match &self.makespan {
                ScheduleAvailability::Disabled => MetricAvailability::Disabled,
                ScheduleAvailability::Available(profile) => {
                    MetricAvailability::Available(profile.output_maximum())
                }
                ScheduleAvailability::Inconsistent => MetricAvailability::Inconsistent,
            },
        }
    }

    /// Returns whether `self` is no worse under every prefix readiness, and
    /// whether at least one scheduling component is strictly better.
    pub(crate) fn dominance(&self, other: &Self) -> Option<bool> {
        let mut strict = self.total_depth.dominance(&other.total_depth)?;
        strict |= self.two_qubit_depth.dominance(&other.two_qubit_depth)?;
        strict |= match (&self.makespan, &other.makespan) {
            (ScheduleAvailability::Available(left), ScheduleAvailability::Available(right)) => {
                left.dominance(right)?
            }
            (ScheduleAvailability::Disabled, ScheduleAvailability::Disabled)
            | (ScheduleAvailability::Inconsistent, ScheduleAvailability::Inconsistent) => false,
            (ScheduleAvailability::Available(_), _)
            | (ScheduleAvailability::Disabled, ScheduleAvailability::Inconsistent) => true,
            (ScheduleAvailability::Disabled, ScheduleAvailability::Available(_))
            | (ScheduleAvailability::Inconsistent, ScheduleAvailability::Available(_))
            | (ScheduleAvailability::Inconsistent, ScheduleAvailability::Disabled) => return None,
        };
        Some(strict)
    }
}

/// Conservative error comparison key. Lower is better.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RobustErrorKey {
    pub(crate) unavailable_count: u32,
    pub(crate) imputed_count: u32,
    pub(crate) log_error: f64,
}

impl PartialEq for RobustErrorKey {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for RobustErrorKey {}

impl PartialOrd for RobustErrorKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RobustErrorKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.unavailable_count
            .cmp(&other.unavailable_count)
            .then_with(|| self.imputed_count.cmp(&other.imputed_count))
            .then_with(|| self.log_error.total_cmp(&other.log_error))
    }
}

impl RobustErrorKey {
    pub(crate) fn compare(self, other: Self) -> Ordering {
        self.cmp(&other)
    }

    pub(crate) fn combine(self, other: Self) -> Self {
        Self {
            unavailable_count: self
                .unavailable_count
                .saturating_add(other.unavailable_count),
            imputed_count: self.imputed_count.saturating_add(other.imputed_count),
            log_error: self.log_error + other.log_error,
        }
    }
}

/// Conservative duration-work comparison key. Lower is better.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RobustDurationKey {
    pub(crate) unavailable_count: u32,
    pub(crate) imputed_count: u32,
    pub(crate) duration_work: f64,
}

impl PartialEq for RobustDurationKey {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for RobustDurationKey {}

impl PartialOrd for RobustDurationKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RobustDurationKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.unavailable_count
            .cmp(&other.unavailable_count)
            .then_with(|| self.imputed_count.cmp(&other.imputed_count))
            .then_with(|| self.duration_work.total_cmp(&other.duration_work))
    }
}

impl RobustDurationKey {
    pub(crate) fn compare(self, other: Self) -> Ordering {
        self.cmp(&other)
    }

    pub(crate) fn combine(self, other: Self) -> Self {
        Self {
            unavailable_count: self
                .unavailable_count
                .saturating_add(other.unavailable_count),
            imputed_count: self.imputed_count.saturating_add(other.imputed_count),
            duration_work: self.duration_work + other.duration_work,
        }
    }
}

/// Availability of one optional physical metric.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) enum MetricAvailability<T> {
    #[default]
    Disabled,
    Available(T),
    Inconsistent,
}

impl<T: Copy> MetricAvailability<T> {
    fn combine(self, other: Self, combine: impl FnOnce(T, T) -> T) -> Self {
        match (self, other) {
            (Self::Disabled, Self::Disabled) => Self::Disabled,
            (Self::Available(left), Self::Available(right)) => {
                Self::Available(combine(left, right))
            }
            (Self::Inconsistent, _)
            | (_, Self::Inconsistent)
            | (Self::Disabled, Self::Available(_))
            | (Self::Available(_), Self::Disabled) => Self::Inconsistent,
        }
    }

    pub(crate) fn compare_by(
        self,
        other: Self,
        compare: impl FnOnce(T, T) -> Ordering,
    ) -> Ordering {
        match (self, other) {
            (Self::Available(left), Self::Available(right)) => compare(left, right),
            (Self::Disabled, Self::Disabled) | (Self::Inconsistent, Self::Inconsistent) => {
                Ordering::Equal
            }
            (Self::Available(_), Self::Disabled | Self::Inconsistent)
            | (Self::Disabled, Self::Inconsistent) => Ordering::Less,
            (Self::Disabled | Self::Inconsistent, Self::Available(_))
            | (Self::Inconsistent, Self::Disabled) => Ordering::Greater,
        }
    }
}

/// Additive native resource cost for one exact lowering plan.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct NativePlanCost {
    pub(crate) native_two_qubit_ops: u32,
    pub(crate) native_total_ops: u32,
    pub(crate) error: MetricAvailability<RobustErrorKey>,
    pub(crate) duration: MetricAvailability<RobustDurationKey>,
}

impl NativePlanCost {
    pub(crate) fn combine(self, other: Self) -> Self {
        Self {
            native_two_qubit_ops: self
                .native_two_qubit_ops
                .saturating_add(other.native_two_qubit_ops),
            native_total_ops: self.native_total_ops.saturating_add(other.native_total_ops),
            error: self.error.combine(other.error, RobustErrorKey::combine),
            duration: self
                .duration
                .combine(other.duration, RobustDurationKey::combine),
        }
    }
}

/// Physical quality of one fully device-lowered operation sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DevicePhysicalCost {
    pub(crate) native_two_qubit_ops: u32,
    pub(crate) native_two_qubit_depth: u32,
    pub(crate) error: MetricAvailability<RobustErrorKey>,
    pub(crate) total_native_depth: u32,
    pub(crate) native_total_ops: u32,
    pub(crate) duration: MetricAvailability<RobustDurationKey>,
    pub(crate) makespan: MetricAvailability<f64>,
}

impl DevicePhysicalCost {
    /// Builds a fictional best-case cost with a proven two-qubit interaction
    /// floor. Every secondary field is the best representable value, making
    /// this suitable only as an admissible selection bound.
    pub(crate) fn optimistic_entangler_lower_bound(native_two_qubit_ops: u32) -> Self {
        Self {
            native_two_qubit_ops,
            native_two_qubit_depth: native_two_qubit_ops,
            error: MetricAvailability::Available(RobustErrorKey::default()),
            total_native_depth: native_two_qubit_ops,
            native_total_ops: native_two_qubit_ops,
            duration: MetricAvailability::Available(RobustDurationKey::default()),
            makespan: MetricAvailability::Available(0.0),
        }
    }

    /// Orders the default production objective. Lower is better.
    pub(crate) fn compare(self, other: Self) -> Ordering {
        self.native_two_qubit_ops
            .cmp(&other.native_two_qubit_ops)
            .then_with(|| {
                self.native_two_qubit_depth
                    .cmp(&other.native_two_qubit_depth)
            })
            .then_with(|| self.error.compare_by(other.error, RobustErrorKey::compare))
            .then_with(|| self.total_native_depth.cmp(&other.total_native_depth))
            .then_with(|| self.native_total_ops.cmp(&other.native_total_ops))
            .then_with(|| {
                self.duration
                    .compare_by(other.duration, RobustDurationKey::compare)
            })
            .then_with(|| {
                self.makespan
                    .compare_by(other.makespan, |left, right| left.total_cmp(&right))
            })
    }

    pub(crate) fn strictly_better_than(self, other: Self) -> bool {
        self.compare(other).is_lt()
    }
}

/// Device-wide conservative estimates used only for missing calibration.
#[derive(Debug, Clone, Default)]
pub(crate) struct CalibrationEstimator {
    error_by_gate: HashMap<KnowledgeInstructionKey, f64>,
    error_by_arity: HashMap<usize, f64>,
    duration_by_gate: HashMap<KnowledgeInstructionKey, f64>,
    duration_by_arity: HashMap<usize, f64>,
    error_enabled: bool,
    duration_enabled: bool,
}

#[derive(Default)]
struct CalibrationSamples {
    errors_by_gate: HashMap<KnowledgeInstructionKey, Vec<f64>>,
    errors_by_arity: HashMap<usize, Vec<f64>>,
    durations_by_gate: HashMap<KnowledgeInstructionKey, Vec<f64>>,
    durations_by_arity: HashMap<usize, Vec<f64>>,
}

impl CalibrationSamples {
    fn record(
        &mut self,
        key: KnowledgeInstructionKey,
        arity: usize,
        error: Option<f64>,
        duration: Option<f64>,
    ) {
        if let Some(error) = error {
            self.errors_by_gate
                .entry(key.clone())
                .or_default()
                .push(error);
            self.errors_by_arity.entry(arity).or_default().push(error);
        }
        if let Some(duration) = duration {
            self.durations_by_gate
                .entry(key)
                .or_default()
                .push(duration);
            self.durations_by_arity
                .entry(arity)
                .or_default()
                .push(duration);
        }
    }
}

impl CalibrationEstimator {
    /// Starts an allocation-free scheduler for a sequence confined to one
    /// ordered physical pair.
    pub(crate) fn two_qubit_cost_accumulator(
        &self,
        pair: [PhysicalQubit; 2],
    ) -> TwoQubitPhysicalCostAccumulator<'_> {
        TwoQubitPhysicalCostAccumulator {
            estimator: self,
            pair,
            total_depths: [0; 2],
            two_qubit_depths: [0; 2],
            total_native_depth: 0,
            native_two_qubit_depth: 0,
            availability: [0.0; 2],
            makespan: 0.0,
            timing_complete: self.duration_enabled(),
        }
    }

    /// The current calibration model keys every native metric by instruction
    /// kind and exact physical qargs, never by the numeric gate parameters.
    ///
    /// Physical sequence caches must consult this capability instead of
    /// inferring it from [`DeviceGateState`]. If a future estimator introduces
    /// parameter-sensitive duration or error models, it must return `false`
    /// until the complete parameter bits are part of the cache key.
    pub(crate) const fn gate_cost_is_parameter_invariant(
        &self,
        _instruction: &Instruction,
    ) -> bool {
        true
    }

    pub(crate) fn identity_cost(&self) -> NativePlanCost {
        NativePlanCost {
            error: if self.error_enabled {
                MetricAvailability::Available(RobustErrorKey::default())
            } else {
                MetricAvailability::Disabled
            },
            duration: if self.duration_enabled {
                MetricAvailability::Available(RobustDurationKey::default())
            } else {
                MetricAvailability::Disabled
            },
            ..NativePlanCost::default()
        }
    }

    pub(crate) fn duration_enabled(&self) -> bool {
        self.duration_enabled
    }

    pub(crate) fn leaf_duration(&self, leaf: &NativePlanLeaf) -> Option<f64> {
        if !self.duration_enabled {
            return None;
        }
        leaf.duration.or_else(|| {
            KnowledgeInstructionKey::from_instruction(&leaf.instruction)
                .as_ref()
                .and_then(|key| self.duration_by_gate.get(key))
                .or_else(|| self.duration_by_arity.get(&leaf.ordered_qargs.len()))
                .copied()
        })
    }

    pub(crate) fn from_device(device: &Device, physical_qubits: &[PhysicalQubit]) -> Self {
        let mut samples = CalibrationSamples::default();
        let usable = physical_qubits.iter().copied().collect::<HashSet<_>>();
        let mut directed_edges = Vec::new();
        for &left in physical_qubits {
            directed_edges.extend(
                device
                    .topology()
                    .successors(left)
                    .filter(|right| usable.contains(right))
                    .map(|right| [left, right]),
            );
        }

        for gate in StandardGate::all().iter().copied() {
            let arity = gate.num_qubits();
            if !(1..=2).contains(&arity) {
                continue;
            }
            let instruction = Instruction::Standard(gate);
            let key = KnowledgeInstructionKey::Standard(gate);
            if arity == 1 {
                for &physical in physical_qubits {
                    collect_device_calibration(
                        device,
                        &instruction,
                        &key,
                        &[physical],
                        &mut samples,
                    );
                }
            } else {
                for ordered in &directed_edges {
                    collect_device_calibration(device, &instruction, &key, ordered, &mut samples);
                }
            }
        }

        Self::from_samples(samples)
    }

    fn from_samples(samples: CalibrationSamples) -> Self {
        let error_enabled = !samples.errors_by_arity.is_empty();
        let duration_enabled = !samples.durations_by_arity.is_empty();
        Self {
            error_by_gate: quantiles(samples.errors_by_gate),
            error_by_arity: quantiles(samples.errors_by_arity),
            duration_by_gate: quantiles(samples.durations_by_gate),
            duration_by_arity: quantiles(samples.durations_by_arity),
            error_enabled,
            duration_enabled,
        }
    }

    pub(crate) fn cost(&self, summary: &NativePlanSummary) -> NativePlanCost {
        let mut cost = self.cost_for_leaves(&summary.leaves);
        cost.native_two_qubit_ops = summary.native_two_qubit_ops;
        cost.native_total_ops = summary.native_total_ops;
        cost
    }

    pub(crate) fn cost_for_leaves(&self, leaves: &[NativePlanLeaf]) -> NativePlanCost {
        let mut error = RobustErrorKey::default();
        let mut duration = RobustDurationKey::default();

        for leaf in leaves {
            let key = KnowledgeInstructionKey::from_instruction(&leaf.instruction);
            let arity = leaf.ordered_qargs.len();
            if self.error_enabled {
                match leaf.error_rate {
                    Some(value) => error.log_error += negative_log_success(value),
                    None => match key
                        .as_ref()
                        .and_then(|key| self.error_by_gate.get(key))
                        .or_else(|| self.error_by_arity.get(&arity))
                        .copied()
                    {
                        Some(value) => {
                            error.imputed_count += 1;
                            error.log_error += negative_log_success(value);
                        }
                        None => error.unavailable_count += 1,
                    },
                }
            }

            if self.duration_enabled {
                match leaf.duration {
                    Some(value) => duration.duration_work += value,
                    None => match key
                        .as_ref()
                        .and_then(|key| self.duration_by_gate.get(key))
                        .or_else(|| self.duration_by_arity.get(&arity))
                        .copied()
                    {
                        Some(value) => {
                            duration.imputed_count += 1;
                            duration.duration_work += value;
                        }
                        None => duration.unavailable_count += 1,
                    },
                }
            }
        }

        NativePlanCost {
            native_two_qubit_ops: leaves
                .iter()
                .filter(|leaf| leaf.ordered_qargs.len() == 2)
                .count() as u32,
            native_total_ops: leaves.len() as u32,
            error: if self.error_enabled {
                MetricAvailability::Available(error)
            } else {
                MetricAvailability::Disabled
            },
            duration: if self.duration_enabled {
                MetricAvailability::Available(duration)
            } else {
                MetricAvailability::Disabled
            },
        }
    }

    pub(crate) fn physical_cost(&self, leaves: &[NativePlanLeaf]) -> DevicePhysicalCost {
        self.schedule_physical_cost(leaves, self.cost_for_leaves(leaves))
    }

    pub(crate) fn schedule_profile(
        &self,
        leaves: &[NativePlanLeaf],
        ordered_qargs: &[PhysicalQubit],
    ) -> Result<DeviceScheduleProfile, String> {
        if ordered_qargs
            .iter()
            .enumerate()
            .any(|(index, qarg)| ordered_qargs[..index].contains(qarg))
        {
            return Err(format!(
                "device schedule profile has duplicate qargs {ordered_qargs:?}"
            ));
        }

        let mut total_depth = MaxPlusProfile::identity(ordered_qargs.len());
        let mut two_qubit_depth = MaxPlusProfile::identity(ordered_qargs.len());
        let mut makespan = if self.duration_enabled {
            ScheduleAvailability::Available(MaxPlusProfile::identity(ordered_qargs.len()))
        } else {
            ScheduleAvailability::Disabled
        };

        for leaf in leaves {
            let indices = leaf
                .ordered_qargs
                .iter()
                .map(|qubit| {
                    ordered_qargs.iter().position(|qarg| qarg == qubit).ok_or_else(|| {
                        format!(
                            "native leaf {} uses {qubit:?} outside planned qargs {ordered_qargs:?}",
                            leaf.instruction
                        )
                    })
                })
                .collect::<Result<SmallVec<[usize; 2]>, _>>()?;
            total_depth.apply(&indices, 1.0);
            if leaf.ordered_qargs.len() == 2 {
                two_qubit_depth.apply(&indices, 1.0);
            }
            if let ScheduleAvailability::Available(profile) = &mut makespan {
                match self.leaf_duration(leaf) {
                    Some(duration) => profile.apply(&indices, duration),
                    None => makespan = ScheduleAvailability::Inconsistent,
                }
            }
        }

        Ok(DeviceScheduleProfile {
            total_depth,
            two_qubit_depth,
            makespan,
        })
    }
}

/// Allocation-free exact scheduler for native leaves confined to two qubits.
///
/// This is intentionally separate from [`CalibrationEstimator::schedule_physical_cost`],
/// whose HashMaps support arbitrary-width plans. The update order mirrors that
/// generic implementation exactly.
pub(crate) struct TwoQubitPhysicalCostAccumulator<'a> {
    estimator: &'a CalibrationEstimator,
    pair: [PhysicalQubit; 2],
    total_depths: [u32; 2],
    two_qubit_depths: [u32; 2],
    total_native_depth: u32,
    native_two_qubit_depth: u32,
    availability: [f64; 2],
    makespan: f64,
    timing_complete: bool,
}

impl TwoQubitPhysicalCostAccumulator<'_> {
    pub(crate) fn add_leaves(&mut self, leaves: &[NativePlanLeaf]) -> Result<(), String> {
        for leaf in leaves {
            let mut indices = SmallVec::<[usize; 2]>::new();
            for qubit in &leaf.ordered_qargs {
                let index = if *qubit == self.pair[0] {
                    0
                } else if *qubit == self.pair[1] {
                    1
                } else {
                    return Err(format!(
                        "native leaf {} uses {qubit:?} outside physical pair {:?}",
                        leaf.instruction, self.pair
                    ));
                };
                indices.push(index);
            }

            let next_depth = indices
                .iter()
                .map(|&index| self.total_depths[index])
                .max()
                .unwrap_or(0)
                + 1;
            for &index in &indices {
                self.total_depths[index] = next_depth;
            }
            self.total_native_depth = self.total_native_depth.max(next_depth);

            if leaf.ordered_qargs.len() == 2 {
                let next_two_qubit_depth = indices
                    .iter()
                    .map(|&index| self.two_qubit_depths[index])
                    .max()
                    .unwrap_or(0)
                    + 1;
                for &index in &indices {
                    self.two_qubit_depths[index] = next_two_qubit_depth;
                }
                self.native_two_qubit_depth = self.native_two_qubit_depth.max(next_two_qubit_depth);
            }

            if self.timing_complete {
                if let Some(duration) = self.estimator.leaf_duration(leaf) {
                    let start = indices
                        .iter()
                        .map(|&index| self.availability[index])
                        .max_by(f64::total_cmp)
                        .unwrap_or(0.0);
                    let finish = start + duration;
                    for &index in &indices {
                        self.availability[index] = finish;
                    }
                    self.makespan = self.makespan.max(finish);
                } else {
                    self.timing_complete = false;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn finish(self, aggregate: NativePlanCost) -> DevicePhysicalCost {
        DevicePhysicalCost {
            native_two_qubit_ops: aggregate.native_two_qubit_ops,
            native_two_qubit_depth: self.native_two_qubit_depth,
            error: aggregate.error,
            total_native_depth: self.total_native_depth,
            native_total_ops: aggregate.native_total_ops,
            duration: aggregate.duration,
            makespan: if !self.estimator.duration_enabled() {
                MetricAvailability::Disabled
            } else if self.timing_complete {
                MetricAvailability::Available(self.makespan)
            } else {
                MetricAvailability::Inconsistent
            },
        }
    }
}

impl CalibrationEstimator {
    pub(crate) fn schedule_physical_cost(
        &self,
        leaves: &[NativePlanLeaf],
        aggregate: NativePlanCost,
    ) -> DevicePhysicalCost {
        let mut total_depths = HashMap::<PhysicalQubit, u32>::new();
        let mut two_qubit_depths = HashMap::<PhysicalQubit, u32>::new();
        let mut total_native_depth = 0;
        let mut native_two_qubit_depth = 0;
        let mut availability = HashMap::<PhysicalQubit, f64>::new();
        let mut makespan = 0.0_f64;
        let mut timing_complete = self.duration_enabled();

        for leaf in leaves {
            let next_depth = leaf
                .ordered_qargs
                .iter()
                .filter_map(|qubit| total_depths.get(qubit))
                .copied()
                .max()
                .unwrap_or(0)
                + 1;
            for qubit in &leaf.ordered_qargs {
                total_depths.insert(*qubit, next_depth);
            }
            total_native_depth = total_native_depth.max(next_depth);

            if leaf.ordered_qargs.len() == 2 {
                let next_two_qubit_depth = leaf
                    .ordered_qargs
                    .iter()
                    .filter_map(|qubit| two_qubit_depths.get(qubit))
                    .copied()
                    .max()
                    .unwrap_or(0)
                    + 1;
                for qubit in &leaf.ordered_qargs {
                    two_qubit_depths.insert(*qubit, next_two_qubit_depth);
                }
                native_two_qubit_depth = native_two_qubit_depth.max(next_two_qubit_depth);
            }

            if timing_complete {
                if let Some(duration) = self.leaf_duration(leaf) {
                    let start = leaf
                        .ordered_qargs
                        .iter()
                        .filter_map(|qubit| availability.get(qubit))
                        .copied()
                        .max_by(f64::total_cmp)
                        .unwrap_or(0.0);
                    let finish = start + duration;
                    for qubit in &leaf.ordered_qargs {
                        availability.insert(*qubit, finish);
                    }
                    makespan = makespan.max(finish);
                } else {
                    timing_complete = false;
                }
            }
        }

        DevicePhysicalCost {
            native_two_qubit_ops: aggregate.native_two_qubit_ops,
            native_two_qubit_depth,
            error: aggregate.error,
            total_native_depth,
            native_total_ops: aggregate.native_total_ops,
            duration: aggregate.duration,
            makespan: if !self.duration_enabled() {
                MetricAvailability::Disabled
            } else if timing_complete {
                MetricAvailability::Available(makespan)
            } else {
                MetricAvailability::Inconsistent
            },
        }
    }
}

fn collect_device_calibration(
    device: &Device,
    instruction: &Instruction,
    key: &KnowledgeInstructionKey,
    qargs: &[PhysicalQubit],
    samples: &mut CalibrationSamples,
) {
    if !device.supports_native_instruction(instruction, qargs) {
        return;
    }
    let Some(calibration) = device.native_instruction_calibration(instruction, qargs) else {
        return;
    };
    samples.record(
        key.clone(),
        qargs.len(),
        calibration.error_rate,
        calibration.duration,
    );
}

pub(crate) fn negative_log_success(error: f64) -> f64 {
    if error == 1.0 {
        f64::INFINITY
    } else {
        -(-error).ln_1p()
    }
}

pub(crate) fn quantiles<K: Eq + std::hash::Hash>(values: HashMap<K, Vec<f64>>) -> HashMap<K, f64> {
    values
        .into_iter()
        .map(|(key, mut values)| {
            values.sort_by(f64::total_cmp);
            let index = ((values.len() * 9).div_ceil(10)).saturating_sub(1);
            (key, values[index])
        })
        .collect()
}

#[cfg(test)]
mod two_qubit_scheduler_tests {
    use super::*;
    use smallvec::smallvec;

    #[test]
    fn fixed_two_qubit_scheduler_matches_generic_scheduler() {
        let device = Device::line("two-qubit-scheduler", 2).unwrap();
        let pair = [PhysicalQubit::new(0), PhysicalQubit::new(1)];
        let estimator = CalibrationEstimator::from_device(&device, &pair);
        let leaves = vec![
            NativePlanLeaf {
                instruction: Instruction::Standard(StandardGate::H),
                ordered_qargs: smallvec![pair[0]],
                error_rate: Some(0.001),
                duration: None,
            },
            NativePlanLeaf {
                instruction: Instruction::Standard(StandardGate::CX),
                ordered_qargs: smallvec![pair[0], pair[1]],
                error_rate: Some(0.01),
                duration: None,
            },
            NativePlanLeaf {
                instruction: Instruction::Standard(StandardGate::X),
                ordered_qargs: smallvec![pair[1]],
                error_rate: Some(0.002),
                duration: None,
            },
        ];
        let aggregate = estimator.cost_for_leaves(&leaves);
        let generic = estimator.schedule_physical_cost(&leaves, aggregate);
        let mut fixed = estimator.two_qubit_cost_accumulator(pair);
        fixed.add_leaves(&leaves).unwrap();

        assert_eq!(fixed.finish(aggregate), generic);
    }

    #[test]
    fn fixed_two_qubit_scheduler_rejects_foreign_qargs() {
        let device = Device::line("two-qubit-scheduler-invalid", 3).unwrap();
        let qubits = device.usable_qubits().collect::<Vec<_>>();
        let estimator = CalibrationEstimator::from_device(&device, &qubits);
        let pair = [PhysicalQubit::new(0), PhysicalQubit::new(1)];
        let leaf = NativePlanLeaf {
            instruction: Instruction::Standard(StandardGate::X),
            ordered_qargs: smallvec![PhysicalQubit::new(2)],
            error_rate: None,
            duration: None,
        };
        let mut fixed = estimator.two_qubit_cost_accumulator(pair);

        assert!(fixed.add_leaves(&[leaf]).is_err());
    }

    #[test]
    fn composed_schedule_profiles_match_leaf_replay() {
        let pair = [PhysicalQubit::new(0), PhysicalQubit::new(1)];
        let estimator = CalibrationEstimator {
            error_enabled: true,
            duration_enabled: true,
            ..CalibrationEstimator::default()
        };
        let children = [
            vec![NativePlanLeaf {
                instruction: Instruction::Standard(StandardGate::H),
                ordered_qargs: smallvec![pair[0]],
                error_rate: Some(0.001),
                duration: Some(2.0),
            }],
            vec![NativePlanLeaf {
                instruction: Instruction::Standard(StandardGate::X),
                ordered_qargs: smallvec![pair[1]],
                error_rate: Some(0.002),
                duration: Some(3.0),
            }],
            vec![
                NativePlanLeaf {
                    instruction: Instruction::Standard(StandardGate::CX),
                    ordered_qargs: smallvec![pair[0], pair[1]],
                    error_rate: Some(0.01),
                    duration: Some(5.0),
                },
                NativePlanLeaf {
                    instruction: Instruction::Standard(StandardGate::H),
                    ordered_qargs: smallvec![pair[0]],
                    error_rate: Some(0.001),
                    duration: Some(7.0),
                },
            ],
        ];
        let mut combined_profile = estimator.schedule_profile(&[], &pair).unwrap();
        let mut combined_cost = estimator.cost_for_leaves(&[]);
        let mut leaves = Vec::new();
        for child in &children {
            let child_qargs = child
                .iter()
                .flat_map(|leaf| leaf.ordered_qargs.iter().copied())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let child_qargs = pair
                .iter()
                .copied()
                .filter(|qarg| child_qargs.contains(qarg))
                .collect::<Vec<_>>();
            let child_profile = estimator.schedule_profile(child, &child_qargs).unwrap();
            combined_profile
                .append_child(&child_profile, &child_qargs, &pair)
                .unwrap();
            combined_cost = combined_cost.combine(estimator.cost_for_leaves(child));
            leaves.extend(child.iter().cloned());
        }

        assert_eq!(
            combined_profile.physical_cost(combined_cost),
            estimator.physical_cost(&leaves)
        );
    }
}
