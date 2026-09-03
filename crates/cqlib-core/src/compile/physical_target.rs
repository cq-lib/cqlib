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

//! Physical topology view used by layout and routing preparation.
//!
//! This module converts a [`Device`] into the compiler-local view needed by
//! layout scoring and SABRE routing. The graph contains only usable physical
//! qubits. Distances and adjacency are undirected because routing can insert
//! SWAPs along either orientation of a connected edge, while directed coupling
//! information is retained separately for target-aware scoring and diagnostics.
//!
//! Calibration data is optional. Missing readout or two-qubit error data does
//! not make the graph invalid; callers decide whether topology-only scoring is
//! acceptable or whether fidelity data is required.

use crate::circuit::{Instruction, StandardGate};
use crate::compile::CompilerError;
use crate::device::{Device, PhysicalQubit, Topology};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

type DirectedQubitPair = (PhysicalQubit, PhysicalQubit);
type TwoQubitGateMap = BTreeMap<DirectedQubitPair, Vec<TwoQubitGateStatus>>;

/// Resolved native capability and calibration for one gate on one directed edge.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) enum TwoQubitGateStatus {
    #[default]
    Unsupported,
    Uncalibrated,
    Calibrated(f64),
}

impl TwoQubitGateStatus {
    pub(super) fn is_supported(self) -> bool {
        !matches!(self, Self::Unsupported)
    }

    pub(super) fn error(self) -> Option<f64> {
        match self {
            Self::Calibrated(error) => Some(error),
            Self::Unsupported | Self::Uncalibrated => None,
        }
    }
}

/// Compiler-local physical graph with usable qubits, distances, and calibration data.
///
/// The graph intentionally separates undirected routing connectivity from
/// directed coupling support. Use [`Self::distance`] or
/// [`Self::is_adjacent_undirected`] for routing reachability, and
/// [`Self::supports_directed_coupling`] when a target-specific direction
/// matters.
#[derive(Debug, Clone, PartialEq)]
pub struct PhysicalLayoutGraph {
    physical_qubits: Vec<PhysicalQubit>,
    physical_index: BTreeMap<PhysicalQubit, usize>,
    adjacency: Vec<Vec<usize>>,
    distances: DistanceTable,
    directed_couplings: BTreeSet<DirectedQubitPair>,
    readout_errors: BTreeMap<PhysicalQubit, f64>,
    two_qubit_gates: TwoQubitGateMap,
    native_two_qubit_gates: Vec<bool>,
    has_readout_error_data: bool,
    has_native_two_qubit_capabilities: bool,
    has_two_qubit_error_data: bool,
}

impl PhysicalLayoutGraph {
    /// Builds a layout physical graph from a device, excluding unusable qubits.
    ///
    /// The resulting graph includes all usable qubits even when the topology is
    /// disconnected. Disconnected pairs have no finite distance and are rejected
    /// later by layout or routing checks when an interaction needs them.
    pub fn from_device(device: &Device) -> Result<Self, CompilerError> {
        let physical_qubits: Vec<_> = device.usable_qubits().collect();
        if physical_qubits.is_empty() {
            return Err(CompilerError::InvalidInput(
                "layout requires at least one usable physical qubit".to_string(),
            ));
        }

        let usable: BTreeSet<_> = physical_qubits.iter().copied().collect();
        let physical_index = build_physical_index(&physical_qubits);
        let adjacency = build_undirected_adjacency(device.topology(), &physical_qubits, &usable);
        let distances =
            DistanceTable::from_adjacency(&physical_qubits, &physical_index, &adjacency);
        let directed_couplings = collect_directed_couplings(device.topology(), &usable);
        let (readout_errors, has_readout_error_data) =
            collect_readout_errors(device, &physical_qubits)?;
        let two_qubit_data = collect_two_qubit_data(device, &usable)?;

        Ok(Self {
            physical_qubits,
            physical_index,
            adjacency,
            distances,
            directed_couplings,
            readout_errors,
            two_qubit_gates: two_qubit_data.gates,
            native_two_qubit_gates: two_qubit_data.native_gates,
            has_readout_error_data,
            has_native_two_qubit_capabilities: two_qubit_data.has_native_capabilities,
            has_two_qubit_error_data: two_qubit_data.has_error_data,
        })
    }

    /// Returns usable physical qubits in deterministic order.
    pub fn physical_qubits(&self) -> &[PhysicalQubit] {
        &self.physical_qubits
    }

    /// Returns all-pairs undirected shortest-path distances over usable qubits.
    pub fn distances(&self) -> &DistanceTable {
        &self.distances
    }

    pub(super) fn physical_index(&self, qubit: PhysicalQubit) -> Option<usize> {
        self.physical_index.get(&qubit).copied()
    }

    pub(super) fn physical_at(&self, index: usize) -> Option<PhysicalQubit> {
        self.physical_qubits.get(index).copied()
    }

    pub(super) fn distance_by_index(&self, a: usize, b: usize) -> Option<u32> {
        self.distances.distance_by_index(a, b)
    }

    /// Returns the undirected shortest-path distance between two usable qubits.
    ///
    /// Returns `None` when either qubit is unusable or when the two usable
    /// qubits are disconnected.
    pub fn distance(&self, a: PhysicalQubit, b: PhysicalQubit) -> Option<u32> {
        let a_index = self.physical_index(a)?;
        let b_index = self.physical_index(b)?;
        self.distance_by_index(a_index, b_index)
    }

    /// Returns whether two usable physical qubits are adjacent in either direction.
    pub fn is_adjacent_undirected(&self, a: PhysicalQubit, b: PhysicalQubit) -> bool {
        let Some(a_index) = self.physical_index(a) else {
            return false;
        };
        let Some(b_index) = self.physical_index(b) else {
            return false;
        };
        self.adjacency[a_index].contains(&b_index)
    }

    /// Returns every usable undirected topology edge exactly once as dense
    /// physical indices.
    pub(super) fn undirected_edges_by_index(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.adjacency
            .iter()
            .enumerate()
            .flat_map(|(left, neighbors)| {
                neighbors
                    .iter()
                    .copied()
                    .filter(move |right| left < *right)
                    .map(move |right| (left, right))
            })
    }

    /// Returns the readout error for a physical qubit, if known.
    pub fn readout_error(&self, qubit: PhysicalQubit) -> Option<f64> {
        self.readout_errors.get(&qubit).copied()
    }

    pub(super) fn readout_error_by_index(&self, index: usize) -> Option<f64> {
        self.physical_at(index)
            .and_then(|qubit| self.readout_error(qubit))
    }

    /// Returns whether `gate` is a native capability on the directed edge.
    pub fn supports_two_qubit_gate_directed(
        &self,
        control: PhysicalQubit,
        target: PhysicalQubit,
        gate: StandardGate,
    ) -> bool {
        self.two_qubit_gate_status(control, target, gate)
            .is_supported()
    }

    /// Returns the calibrated error for `gate` on the directed edge.
    ///
    /// `None` means either that the gate is unsupported or that it is supported
    /// without a known calibration. Use
    /// [`Self::supports_two_qubit_gate_directed`] to distinguish those states.
    pub fn two_qubit_gate_error_directed(
        &self,
        control: PhysicalQubit,
        target: PhysicalQubit,
        gate: StandardGate,
    ) -> Option<f64> {
        self.two_qubit_gate_status(control, target, gate).error()
    }

    pub(super) fn two_qubit_gate_status(
        &self,
        control: PhysicalQubit,
        target: PhysicalQubit,
        gate: StandardGate,
    ) -> TwoQubitGateStatus {
        self.two_qubit_gates
            .get(&(control, target))
            .and_then(|gates| gates.get(gate as usize))
            .copied()
            .unwrap_or_default()
    }

    pub(super) fn two_qubit_gate_status_by_index(
        &self,
        control: usize,
        target: usize,
        gate: StandardGate,
    ) -> TwoQubitGateStatus {
        match (self.physical_at(control), self.physical_at(target)) {
            (Some(control), Some(target)) => self.two_qubit_gate_status(control, target, gate),
            _ => TwoQubitGateStatus::Unsupported,
        }
    }

    pub(super) fn is_two_qubit_gate_native_anywhere(&self, gate: StandardGate) -> bool {
        self.native_two_qubit_gates
            .get(gate as usize)
            .copied()
            .unwrap_or(false)
    }

    /// Returns whether there is a directed coupling from `control` to `target`.
    pub fn supports_directed_coupling(
        &self,
        control: PhysicalQubit,
        target: PhysicalQubit,
    ) -> bool {
        self.directed_couplings.contains(&(control, target))
    }

    pub(super) fn supports_directed_coupling_by_index(
        &self,
        control: usize,
        target: usize,
    ) -> bool {
        match (self.physical_at(control), self.physical_at(target)) {
            (Some(control), Some(target)) => self.supports_directed_coupling(control, target),
            _ => false,
        }
    }

    /// Returns whether any readout or two-qubit calibration data is available.
    pub fn has_fidelity_data(&self) -> bool {
        self.has_readout_error_data || self.has_two_qubit_error_data
    }

    /// Returns whether readout-error data is available.
    pub fn has_readout_error_data(&self) -> bool {
        self.has_readout_error_data
    }

    /// Returns whether any usable directed edge has native two-qubit capabilities.
    pub(crate) fn has_native_two_qubit_capabilities(&self) -> bool {
        self.has_native_two_qubit_capabilities
    }

    /// Returns whether two-qubit error data is available.
    pub fn has_two_qubit_error_data(&self) -> bool {
        self.has_two_qubit_error_data
    }
}

/// All-pairs undirected shortest-path distances over usable physical qubits.
///
/// A missing distance means the pair is disconnected in the usable undirected
/// topology or at least one queried qubit is outside this table.
#[derive(Debug, Clone, PartialEq)]
pub struct DistanceTable {
    qubits: Vec<PhysicalQubit>,
    index: BTreeMap<PhysicalQubit, usize>,
    distances: Vec<Vec<Option<u32>>>,
}

impl DistanceTable {
    /// Builds all-pairs distances by BFS from every usable physical qubit.
    fn from_adjacency(
        qubits: &[PhysicalQubit],
        index: &BTreeMap<PhysicalQubit, usize>,
        adjacency: &[Vec<usize>],
    ) -> Self {
        let mut distances = vec![vec![None; qubits.len()]; qubits.len()];

        for (start_index, item) in distances.iter_mut().enumerate().take(qubits.len()) {
            let mut queue = VecDeque::new();
            item[start_index] = Some(0);
            queue.push_back(start_index);

            while let Some(current_index) = queue.pop_front() {
                let current_distance =
                    item[current_index].expect("queued nodes have assigned distances");

                for neighbor_index in adjacency[current_index].iter().copied() {
                    if item[neighbor_index].is_none() {
                        item[neighbor_index] = Some(current_distance + 1);
                        queue.push_back(neighbor_index);
                    }
                }
            }
        }

        Self {
            qubits: qubits.to_vec(),
            index: index.clone(),
            distances,
        }
    }

    /// Returns the physical qubits covered by this table.
    pub fn qubits(&self) -> &[PhysicalQubit] {
        &self.qubits
    }

    /// Returns the shortest-path distance between two physical qubits.
    ///
    /// Returns `None` when either qubit is absent from the table or the pair is
    /// disconnected.
    pub fn distance(&self, a: PhysicalQubit, b: PhysicalQubit) -> Option<u32> {
        let a_index = self.index.get(&a)?;
        let b_index = self.index.get(&b)?;
        self.distance_by_index(*a_index, *b_index)
    }

    pub(super) fn distance_by_index(&self, a: usize, b: usize) -> Option<u32> {
        self.distances.get(a)?.get(b).copied().flatten()
    }
}

fn build_physical_index(physical_qubits: &[PhysicalQubit]) -> BTreeMap<PhysicalQubit, usize> {
    physical_qubits
        .iter()
        .copied()
        .enumerate()
        .map(|(index, qubit)| (qubit, index))
        .collect()
}

fn build_undirected_adjacency(
    topology: &Topology,
    physical_qubits: &[PhysicalQubit],
    usable: &BTreeSet<PhysicalQubit>,
) -> Vec<Vec<usize>> {
    let index = build_physical_index(physical_qubits);
    let mut adjacency = vec![Vec::new(); physical_qubits.len()];

    for qubit in physical_qubits {
        let qubit_index = index[qubit];
        for neighbor in topology.successors(*qubit) {
            if usable.contains(&neighbor) {
                adjacency[qubit_index].push(index[&neighbor]);
            }
        }
        for neighbor in topology.predecessors(*qubit) {
            if usable.contains(&neighbor) {
                adjacency[qubit_index].push(index[&neighbor]);
            }
        }
    }

    for neighbors in &mut adjacency {
        neighbors.sort_unstable_by_key(|index| physical_qubits[*index]);
        neighbors.dedup();
    }

    adjacency
}

fn collect_readout_errors(
    device: &Device,
    physical_qubits: &[PhysicalQubit],
) -> Result<(BTreeMap<PhysicalQubit, f64>, bool), CompilerError> {
    let mut errors = BTreeMap::new();
    for qubit in physical_qubits {
        if let Some(error) = device.get_readout_error(*qubit) {
            validate_probability(error, "readout error")?;
            errors.insert(*qubit, error);
        }
    }
    let has_data = !errors.is_empty();
    Ok((errors, has_data))
}

fn collect_directed_couplings(
    topology: &Topology,
    usable: &BTreeSet<PhysicalQubit>,
) -> BTreeSet<DirectedQubitPair> {
    let mut couplings = BTreeSet::new();
    for control in usable {
        for target in topology.successors(*control) {
            if usable.contains(&target) {
                couplings.insert((*control, target));
            }
        }
    }
    couplings
}

struct TwoQubitData {
    gates: TwoQubitGateMap,
    native_gates: Vec<bool>,
    has_native_capabilities: bool,
    has_error_data: bool,
}

fn collect_two_qubit_data(
    device: &Device,
    usable: &BTreeSet<PhysicalQubit>,
) -> Result<TwoQubitData, CompilerError> {
    if let Some(error) = device.default_two_qubit_error() {
        validate_probability(error, "default two-qubit error")?;
    }

    let mut gates = BTreeMap::new();
    let mut native_gates = vec![false; StandardGate::all().len()];
    let mut has_native_capabilities = false;
    let mut has_error_data = false;
    for control in usable {
        for target in device.topology().successors(*control) {
            if !usable.contains(&target) {
                continue;
            }

            let edge = device.edge_properties(*control, target);
            if let Some(edge) = edge {
                for property in edge.native_instructions() {
                    validate_probability(property.error_rate(), "edge two-qubit error")?;
                }
            }

            let mut edge_gates = None;
            for gate in StandardGate::all()
                .iter()
                .copied()
                .filter(|gate| gate.num_qubits() == 2)
            {
                let instruction = Instruction::Standard(gate);
                if !device.supports_native_instruction(&instruction, &[*control, target]) {
                    continue;
                }

                has_native_capabilities = true;
                native_gates[gate as usize] = true;
                let status = match device.two_qubit_error(*control, target, &instruction) {
                    Some(error) => {
                        validate_probability(error, "gate-specific two-qubit error")?;
                        has_error_data = true;
                        TwoQubitGateStatus::Calibrated(error)
                    }
                    None => TwoQubitGateStatus::Uncalibrated,
                };
                edge_gates.get_or_insert_with(|| {
                    vec![TwoQubitGateStatus::Unsupported; StandardGate::all().len()]
                })[gate as usize] = status;
            }
            if let Some(edge_gates) = edge_gates {
                gates.insert((*control, target), edge_gates);
            }
        }
    }

    Ok(TwoQubitData {
        gates,
        native_gates,
        has_native_capabilities,
        has_error_data,
    })
}

fn validate_probability(value: f64, name: &str) -> Result<(), CompilerError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(CompilerError::InvalidInput(format!(
            "device {} must be a finite probability in [0, 1], got {}",
            name, value
        )))
    }
}
