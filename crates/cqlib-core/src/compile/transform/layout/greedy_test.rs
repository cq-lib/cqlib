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

use super::*;
use crate::circuit::{Circuit, Instruction, Qubit, StandardGate};
use crate::compile::CompilerError;
use crate::device::{Device, EdgeProp, InstructionProp, LogicalQubit, PhysicalQubit, Topology};
use std::collections::HashSet;

#[test]
fn greedy_layout_prioritizes_highest_weight_interaction() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let device = Device::line_from_qubits("line", vec![p0, p1, p2]).unwrap();
    let objective = LayoutObjective::topology_only();

    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = greedy_layout(&circuit, &device, &objective).unwrap();

    assert_eq!(result.layout.get_physical(LogicalQubit::new(0)), Some(p0));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(1)), Some(p1));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(2)), Some(p2));
    assert!(!result.diagnostics.is_perfect);
    assert_eq!(result.score.unwrap().distance, 5.0);
}

#[test]
fn greedy_layout_extends_from_mapped_endpoint() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let p3 = PhysicalQubit::new(3);
    let device = Device::line_from_qubits("line", vec![p0, p1, p2, p3]).unwrap();
    let objective = LayoutObjective::topology_only();

    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();

    let result = greedy_layout(&circuit, &device, &objective).unwrap();

    assert_eq!(result.layout.get_physical(LogicalQubit::new(0)), Some(p0));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(1)), Some(p1));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(2)), Some(p2));
    assert_eq!(result.layout.num_vacant_physical(), 1);
    assert!(result.diagnostics.is_perfect);
}

#[test]
fn greedy_layout_maps_idle_logical_qubits_deterministically() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let p3 = PhysicalQubit::new(3);
    let device = Device::line_from_qubits("line", vec![p0, p1, p2, p3]).unwrap();
    let objective = LayoutObjective::topology_only();
    let circuit = Circuit::new(3);

    let result = greedy_layout(&circuit, &device, &objective).unwrap();

    assert_eq!(result.layout.get_physical(LogicalQubit::new(0)), Some(p0));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(1)), Some(p1));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(2)), Some(p2));
    assert_eq!(result.layout.num_vacant_physical(), 1);
    assert!(result.diagnostics.is_perfect);
}

#[test]
fn greedy_layout_prefers_connected_pair_before_disconnected_tie_break() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let p3 = PhysicalQubit::new(3);
    let topology = Topology::new(vec![p0, p1, p2, p3], vec![(p2, p3, "cx".to_string())]).unwrap();
    let device = Device::new("sparse", HashSet::from_iter([p0, p1, p2, p3]), topology).unwrap();
    let objective = LayoutObjective::topology_only();

    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = greedy_layout(&circuit, &device, &objective).unwrap();

    assert_eq!(result.layout.get_physical(LogicalQubit::new(0)), Some(p2));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(1)), Some(p3));
    assert!(result.diagnostics.is_perfect);
}

#[test]
fn greedy_layout_rejects_insufficient_physical_qubits() {
    let p0 = PhysicalQubit::new(0);
    let topology = Topology::new(vec![p0], vec![]).unwrap();
    let device = Device::new("one", HashSet::from_iter([p0]), topology).unwrap();
    let objective = LayoutObjective::topology_only();
    let circuit = Circuit::new(2);

    let error = greedy_layout(&circuit, &device, &objective).unwrap_err();

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("2 logical qubits") && message.contains("1 usable physical qubits"))
    );
}

#[test]
fn greedy_layout_reports_non_perfect_for_non_adjacent_interaction() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let device = Device::line_from_qubits("line", vec![p0, p1, p2]).unwrap();
    let objective = LayoutObjective::topology_only();

    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = greedy_layout(&circuit, &device, &objective).unwrap();

    assert!(!result.diagnostics.is_perfect);
    assert_eq!(result.score.unwrap().distance, 5.0);
}

#[test]
fn greedy_layout_uses_fidelity_objective_for_ties() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let topology = Topology::new(
        vec![p0, p1, p2],
        vec![(p0, p1, "cx".to_string()), (p1, p2, "cx".to_string())],
    )
    .unwrap();
    let mut device = Device::new("line", HashSet::from_iter([p0, p1, p2]), topology).unwrap();
    device
        .add_edge_properties(
            p0,
            p1,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.09,
                ))
                .unwrap()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CZ),
                    0.001,
                ))
                .unwrap(),
        )
        .unwrap();
    device
        .add_edge_properties(
            p1,
            p2,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.01,
                ))
                .unwrap(),
        )
        .unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let objective = LayoutObjective::auto_from_physical(&physical);

    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let analysis = analyze_circuit_for_layout(&circuit).unwrap();

    let result = greedy_layout_prepared(&analysis, &physical, &objective).unwrap();

    assert_eq!(result.layout.get_physical(LogicalQubit::new(0)), Some(p1));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(1)), Some(p2));
    assert!(result.diagnostics.used_fidelity);
    assert_eq!(result.score.unwrap().two_qubit_error, 0.01);
}
