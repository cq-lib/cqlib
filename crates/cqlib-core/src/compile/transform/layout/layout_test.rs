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
use crate::circuit::gate::Directive;
use crate::circuit::{Circuit, ClassicalExpr, ClassicalType, Instruction, Qubit, StandardGate};
use crate::compile::CompilerError;
use crate::device::{
    Device, EdgeProp, InstructionProp, Layout, LogicalQubit, PhysicalQubit, QubitProp, Topology,
};
use std::collections::HashSet;

#[test]
fn analyze_circuit_builds_weighted_interaction_graph() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q1, q0).unwrap();
    circuit.cz(q1, q2).unwrap();
    circuit.barrier(vec![q0, q1, q2]).unwrap();

    let analysis = analyze_circuit_for_layout(&circuit).unwrap();

    assert_eq!(
        analysis.logical_qubits,
        vec![
            LogicalQubit::new(0),
            LogicalQubit::new(1),
            LogicalQubit::new(2)
        ]
    );
    assert_eq!(analysis.interactions.len(), 2);

    let interactions = analysis.interactions.interactions();
    assert_eq!(interactions[0].left, LogicalQubit::new(0));
    assert_eq!(interactions[0].right, LogicalQubit::new(1));
    assert_eq!(interactions[0].weight, 2.0);
    assert_eq!(interactions[0].directed_weight_left_to_right, 1.0);
    assert_eq!(interactions[0].directed_weight_right_to_left, 1.0);
    assert_eq!(interactions[0].first_seen_order, 0);

    assert_eq!(interactions[1].left, LogicalQubit::new(1));
    assert_eq!(interactions[1].right, LogicalQubit::new(2));
    assert_eq!(interactions[1].weight, 1.0);
}

#[test]
fn analyze_recurses_into_control_flow_operations() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.cx(q0, q1)?;
            Ok(())
        })
        .unwrap();

    let analysis = analyze_circuit_for_layout(&circuit).unwrap();
    assert_eq!(analysis.interactions.len(), 1);
    assert_eq!(analysis.interactions.interactions()[0].weight, 2.0);
}

#[test]
fn analyze_recurses_into_branch_loop_and_switch_bodies() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.cx(q0, q1)?;
                Ok(())
            },
            |body| {
                body.cz(q1, q2)?;
                Ok(())
            },
        )
        .unwrap();
    let loop_var = circuit.var(ClassicalType::uint(2).unwrap());
    circuit
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(2, 0).unwrap(),
            ClassicalExpr::uint_literal(2, 2).unwrap(),
            ClassicalExpr::uint_literal(2, 1).unwrap(),
            |body, _| {
                body.cx(q0, q2)?;
                Ok(())
            },
        )
        .unwrap();
    circuit
        .switch(ClassicalExpr::uint_literal(2, 1).unwrap(), |switch| {
            switch.value(1, |body| {
                body.cx(q0, q1)?;
                Ok(())
            })?;
            switch.default(|body| {
                body.cz(q1, q2)?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap();

    let analysis = analyze_circuit_for_layout(&circuit).unwrap();

    assert_eq!(analysis.interactions.len(), 3);
    let interactions = analysis.interactions.interactions();
    assert_eq!(interactions[0].left, LogicalQubit::new(0));
    assert_eq!(interactions[0].right, LogicalQubit::new(1));
    assert_eq!(interactions[0].weight, 2.0);
    assert_eq!(interactions[1].left, LogicalQubit::new(0));
    assert_eq!(interactions[1].right, LogicalQubit::new(2));
    assert_eq!(interactions[1].weight, 1.0);
    assert_eq!(interactions[2].left, LogicalQubit::new(1));
    assert_eq!(interactions[2].right, LogicalQubit::new(2));
    assert_eq!(interactions[2].weight, 2.0);
}

#[test]
fn analyze_circuit_rejects_undecomposed_multi_qubit_unitary() {
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::Standard(StandardGate::CCX),
            [Qubit::new(0), Qubit::new(1), Qubit::new(2)],
            std::iter::empty(),
            None,
        )
        .unwrap();

    let error = analyze_circuit_for_layout(&circuit).unwrap_err();
    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("more than two qubits"))
    );
}

#[test]
fn physical_graph_filters_invalid_qubits_and_computes_undirected_distances() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let p3 = PhysicalQubit::new(3);
    let topology = Topology::new(
        vec![p0, p1, p2, p3],
        vec![
            (p0, p1, "cx".to_string()),
            (p1, p2, "cx".to_string()),
            (p2, p3, "cx".to_string()),
        ],
    )
    .unwrap();
    let device = Device::new("line", HashSet::from_iter([p0, p1, p2, p3]), topology)
        .unwrap()
        .with_invalid_qubits(HashSet::from_iter([p2]))
        .unwrap();

    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();

    assert_eq!(physical.physical_qubits(), &[p0, p1, p3]);
    assert_eq!(physical.distance(p0, p1), Some(1));
    assert_eq!(physical.distance(p0, p3), None);
    assert!(physical.supports_directed_coupling(p0, p1));
    assert!(!physical.supports_directed_coupling(p1, p0));
}

#[test]
fn objective_auto_uses_fidelity_data_when_available() {
    let l0 = LogicalQubit::new(0);
    let l1 = LogicalQubit::new(1);
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);

    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let analysis = analyze_circuit_for_layout(&circuit).unwrap();

    let topology = Topology::new(vec![p0, p1], vec![(p0, p1, "cx".to_string())]).unwrap();
    let mut device = Device::new("two", HashSet::from_iter([p0, p1]), topology)
        .unwrap()
        .with_default_two_qubit_error(0.07);
    device
        .add_qubit_properties(p0, QubitProp::new(0.01))
        .unwrap();
    device
        .add_qubit_properties(p1, QubitProp::new(0.03))
        .unwrap();
    device
        .add_edge_properties(
            p0,
            p1,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.02,
                ))
                .unwrap(),
        )
        .unwrap();

    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let objective = LayoutObjective::auto_from_physical(&physical);
    let layout = Layout::new(vec![l0, l1], vec![p0, p1], None).unwrap();
    let score = objective
        .score_layout(&analysis, &physical, &layout)
        .unwrap();

    assert!(score.used_fidelity);
    assert_eq!(score.distance, 1.0);
    assert_eq!(score.direction, 0.0);
    assert_eq!(score.two_qubit_error, 0.02);
    assert_eq!(score.readout_error, 0.04);
    assert!((score.total - 1.24).abs() < 1e-12);
}

#[test]
fn topology_only_objective_ignores_error_data() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p0, p1, "cx".to_string())]).unwrap();
    let device = Device::new("two", HashSet::from_iter([p0, p1]), topology)
        .unwrap()
        .with_default_readout_error(0.02)
        .with_default_two_qubit_error(0.03);
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let objective = LayoutObjective::topology_only();

    assert!(physical.has_fidelity_data());
    assert!(!objective.uses_fidelity());
}

#[test]
fn directives_with_many_qubits_do_not_create_interactions() {
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::Directive(Directive::Barrier),
            [Qubit::new(0), Qubit::new(1), Qubit::new(2)],
            std::iter::empty(),
            None,
        )
        .unwrap();

    let analysis = analyze_circuit_for_layout(&circuit).unwrap();

    assert!(analysis.interactions.is_empty());
}

#[test]
fn trivial_layout_maps_logical_qubits_to_usable_physical_qubits_in_order() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let topology = Topology::new(
        vec![p0, p1, p2],
        vec![(p0, p1, "cx".to_string()), (p1, p2, "cx".to_string())],
    )
    .unwrap();
    let device = Device::new("line", HashSet::from_iter([p0, p1, p2]), topology).unwrap();
    let objective = LayoutObjective::topology_only();

    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = trivial_layout(&circuit, &device, &objective).unwrap();

    assert_eq!(result.layout.get_physical(LogicalQubit::new(0)), Some(p0));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(1)), Some(p1));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(2)), Some(p2));
    assert_eq!(result.diagnostics.candidates_evaluated, 1);
    assert!(result.diagnostics.is_perfect);

    let score = result.score.unwrap();
    assert_eq!(score.distance, 1.0);
    assert_eq!(score.direction, 0.0);
    assert!(!score.used_fidelity);
}

#[test]
fn trivial_layout_skips_invalid_physical_qubits() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let topology = Topology::new(
        vec![p0, p1, p2],
        vec![(p0, p1, "cx".to_string()), (p1, p2, "cx".to_string())],
    )
    .unwrap();
    let device = Device::new("line", HashSet::from_iter([p0, p1, p2]), topology)
        .unwrap()
        .with_invalid_qubits(HashSet::from_iter([p1]))
        .unwrap();
    let objective = LayoutObjective::topology_only();
    let circuit = Circuit::new(2);

    let result = trivial_layout(&circuit, &device, &objective).unwrap();

    assert_eq!(result.layout.get_physical(LogicalQubit::new(0)), Some(p0));
    assert_eq!(result.layout.get_physical(LogicalQubit::new(1)), Some(p2));
    assert_eq!(result.layout.num_vacant_physical(), 0);
    assert!(result.diagnostics.is_perfect);
}

#[test]
fn trivial_layout_rejects_insufficient_physical_qubits() {
    let p0 = PhysicalQubit::new(0);
    let topology = Topology::new(vec![p0], vec![]).unwrap();
    let device = Device::new("one", HashSet::from_iter([p0]), topology).unwrap();
    let objective = LayoutObjective::topology_only();
    let circuit = Circuit::new(2);

    let error = trivial_layout(&circuit, &device, &objective).unwrap_err();

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("2 logical qubits") && message.contains("1 usable physical qubits"))
    );
}

#[test]
fn trivial_layout_reports_non_perfect_when_interaction_is_not_adjacent() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let topology = Topology::new(
        vec![p0, p1, p2],
        vec![(p0, p1, "cx".to_string()), (p1, p2, "cx".to_string())],
    )
    .unwrap();
    let device = Device::new("line", HashSet::from_iter([p0, p1, p2]), topology).unwrap();
    let objective = LayoutObjective::topology_only();

    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let result = trivial_layout(&circuit, &device, &objective).unwrap();

    assert!(!result.diagnostics.is_perfect);
    assert_eq!(result.score.unwrap().distance, 2.0);
}

#[test]
fn trivial_layout_uses_fidelity_objective_when_requested() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p0, p1, "cx".to_string())]).unwrap();
    let device = Device::new("two", HashSet::from_iter([p0, p1]), topology)
        .unwrap()
        .with_default_readout_error(0.02)
        .with_default_two_qubit_error(0.03);
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let objective = LayoutObjective::auto_from_physical(&physical);

    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let analysis = analyze_circuit_for_layout(&circuit).unwrap();

    let result = trivial_layout_prepared(&analysis, &physical, &objective).unwrap();

    assert!(result.diagnostics.used_fidelity);
    assert!(result.score.unwrap().used_fidelity);
}
