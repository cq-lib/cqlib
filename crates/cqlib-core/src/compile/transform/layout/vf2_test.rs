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
fn vf2_perfect_layout_maps_path_to_line() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let device = Device::line_from_qubits("line", vec![p0, p1, p2]).unwrap();
    let objective = LayoutObjective::topology_only();

    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();

    let result =
        vf2_perfect_layout(&circuit, &device, &objective, &Vf2LayoutConfig::default()).unwrap();

    assert!(result.diagnostics.is_perfect);
    assert!(result.diagnostics.candidates_evaluated > 0);
    assert_eq!(result.score.unwrap().distance, 2.0);
}

#[test]
fn vf2_perfect_layout_uses_non_induced_subgraph_matching() {
    let device = Device::from_edges("device", 3, &[(0, 1), (1, 2), (2, 0)]).unwrap();
    let objective = LayoutObjective::topology_only();

    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();

    let result =
        vf2_perfect_layout(&circuit, &device, &objective, &Vf2LayoutConfig::default()).unwrap();

    assert!(result.diagnostics.is_perfect);
    assert_eq!(result.score.unwrap().distance, 2.0);
}

#[test]
fn vf2_perfect_layout_rejects_when_no_perfect_mapping_exists() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let device = Device::line_from_qubits("line", vec![p0, p1, p2]).unwrap();
    let objective = LayoutObjective::topology_only();

    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let error =
        vf2_perfect_layout(&circuit, &device, &objective, &Vf2LayoutConfig::default()).unwrap_err();

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("could not find a perfect mapping"))
    );
}

#[test]
fn vf2_perfect_layout_uses_fidelity_objective_to_choose_candidate() {
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

    let result = vf2_perfect_layout_prepared(
        &analysis,
        &physical,
        &objective,
        &Vf2LayoutConfig::default(),
    )
    .unwrap();

    assert_eq!(result.layout.get_physical(LogicalQubit::new(0)), Some(p1));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(1)), Some(p2));
    assert!(result.diagnostics.used_fidelity);
    assert_eq!(result.score.unwrap().two_qubit_error, 0.01);
}

#[test]
fn vf2_perfect_layout_keeps_direction_as_scoring_penalty() {
    let device = Device::from_edges("device", 2, &[(1, 0)]).unwrap();
    let objective = LayoutObjective::topology_only();
    let config = Vf2LayoutConfig {
        candidate_limit: 1,
        ..Vf2LayoutConfig::default()
    };

    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = vf2_perfect_layout(&circuit, &device, &objective, &config).unwrap();

    assert!(result.diagnostics.is_perfect);
    assert_eq!(result.score.unwrap().direction, 1.0);
}

#[test]
fn vf2_perfect_layout_maps_idle_logical_qubits() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let p3 = PhysicalQubit::new(3);
    let device = Device::line_from_qubits("line", vec![p0, p1, p2, p3]).unwrap();
    let objective = LayoutObjective::topology_only();

    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result =
        vf2_perfect_layout(&circuit, &device, &objective, &Vf2LayoutConfig::default()).unwrap();

    assert!(result.layout.get_physical(LogicalQubit::new(2)).is_some());
    assert_eq!(result.layout.num_vacant_physical(), 1);
    assert!(result.diagnostics.is_perfect);
}

#[test]
fn vf2_perfect_layout_scores_interaction_free_circuit() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], Vec::new()).unwrap();
    let mut device = Device::new("two", HashSet::from_iter([p0, p1]), topology).unwrap();
    device
        .add_qubit_properties(p0, crate::device::QubitProp::new(0.20))
        .unwrap();
    device
        .add_qubit_properties(p1, crate::device::QubitProp::new(0.01))
        .unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let objective = LayoutObjective::auto_from_physical(&physical);
    let circuit = Circuit::new(2);
    let analysis = analyze_circuit_for_layout(&circuit).unwrap();

    let result = vf2_perfect_layout_prepared(
        &analysis,
        &physical,
        &objective,
        &Vf2LayoutConfig::default(),
    )
    .unwrap();

    assert_eq!(result.layout.get_physical(LogicalQubit::new(0)), Some(p1));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(1)), Some(p0));
    assert!(result.diagnostics.is_perfect);
    assert_eq!(result.diagnostics.candidates_evaluated, 1);
}

#[test]
fn vf2_perfect_layout_rejects_invalid_candidate_limit() {
    let p0 = PhysicalQubit::new(0);
    let device = Device::line_from_qubits("line", vec![p0]).unwrap();
    let objective = LayoutObjective::topology_only();
    let circuit = Circuit::new(1);
    let config = Vf2LayoutConfig {
        candidate_limit: 0,
        ..Vf2LayoutConfig::default()
    };

    let error = vf2_perfect_layout(&circuit, &device, &objective, &config).unwrap_err();

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("candidate_limit"))
    );
}

#[test]
fn vf2_perfect_layout_respects_candidate_limit() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let device = Device::line_from_qubits("line", vec![p0, p1, p2]).unwrap();
    let objective = LayoutObjective::topology_only();
    let config = Vf2LayoutConfig {
        candidate_limit: 1,
        ..Vf2LayoutConfig::default()
    };

    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = vf2_perfect_layout(&circuit, &device, &objective, &config).unwrap();

    assert_eq!(result.diagnostics.candidates_evaluated, 1);
    assert!(result.diagnostics.notes.is_empty());
}

#[test]
fn vf2_perfect_layout_reports_call_limit_exhaustion() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let device = Device::line_from_qubits("line", vec![p0, p1, p2]).unwrap();
    let objective = LayoutObjective::topology_only();
    let config = Vf2LayoutConfig {
        call_limit: Some(1),
        ..Vf2LayoutConfig::default()
    };

    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();

    let error = vf2_perfect_layout(&circuit, &device, &objective, &config).unwrap_err();

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("call limit"))
    );
}

#[test]
fn vf2_perfect_layout_rejects_insufficient_physical_qubits() {
    let p0 = PhysicalQubit::new(0);
    let device = Device::line_from_qubits("line", vec![p0]).unwrap();
    let objective = LayoutObjective::topology_only();
    let circuit = Circuit::new(2);

    let error =
        vf2_perfect_layout(&circuit, &device, &objective, &Vf2LayoutConfig::default()).unwrap_err();

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("2 logical qubits") && message.contains("1 usable physical qubits"))
    );
}
