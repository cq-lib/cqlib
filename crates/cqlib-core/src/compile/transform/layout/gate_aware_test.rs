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

use super::*;
use crate::circuit::{Circuit, Instruction, Qubit, StandardGate, UnitaryGate};
use crate::device::{
    Device, EdgeProp, InstructionProp, Layout, LogicalQubit, PhysicalQubit, QubitProp, Topology,
};
use std::collections::{BTreeMap, HashSet};

#[test]
fn analysis_keeps_gate_and_orientation_contributions_aligned() {
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cz(Qubit::new(1), Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(0)).unwrap();

    let analysis = analyze_circuit_for_layout(&circuit).unwrap();
    assert_eq!(analysis.interactions.len(), 1);
    assert_eq!(
        analysis.interactions.contribution_slot_count(),
        analysis.interactions.len()
    );

    let contributions = analysis.interactions.gate_contributions(0);
    assert_eq!(contributions.len(), 2);
    let cx = contributions
        .iter()
        .find(|item| item.gate == StandardGate::CX)
        .unwrap();
    assert_eq!(cx.left_to_right_weight, 1.0);
    assert_eq!(cx.right_to_left_weight, 1.0);
    let cz = contributions
        .iter()
        .find(|item| item.gate == StandardGate::CZ)
        .unwrap();
    assert_eq!(cz.left_to_right_weight, 0.0);
    assert_eq!(cz.right_to_left_weight, 1.0);
}

#[test]
fn gate_specific_errors_do_not_leak_between_native_gates() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let p3 = PhysicalQubit::new(3);
    let topology = Topology::new(
        vec![p0, p1, p2, p3],
        vec![(p0, p1, "a".to_string()), (p2, p3, "b".to_string())],
    )
    .unwrap();
    let mut device =
        Device::new("gate-specific", HashSet::from([p0, p1, p2, p3]), topology).unwrap();
    device
        .add_edge_properties(
            p0,
            p1,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.10,
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
            p2,
            p3,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.01,
                ))
                .unwrap(),
        )
        .unwrap();

    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let objective = LayoutObjective::fidelity_aware();
    let layout_a = layout_on_pair(&[p0, p1, p2, p3], p0, p1);
    let layout_b = layout_on_pair(&[p0, p1, p2, p3], p2, p3);

    let mut cx_circuit = Circuit::new(2);
    cx_circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let cx = analyze_circuit_for_layout(&cx_circuit).unwrap();
    assert_eq!(
        objective
            .score_layout(&cx, &physical, &layout_a)
            .unwrap()
            .two_qubit_error,
        0.10
    );
    assert_eq!(
        objective
            .score_layout(&cx, &physical, &layout_b)
            .unwrap()
            .two_qubit_error,
        0.01
    );

    let mut cz_circuit = Circuit::new(2);
    cz_circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();
    let cz = analyze_circuit_for_layout(&cz_circuit).unwrap();
    assert_eq!(
        objective
            .score_layout(&cz, &physical, &layout_a)
            .unwrap()
            .two_qubit_error,
        0.001
    );
    assert_eq!(
        objective
            .score_layout(&cz, &physical, &layout_b)
            .unwrap()
            .two_qubit_error,
        1.0
    );
}

#[test]
fn unsupported_and_uncalibrated_native_gates_receive_conservative_cost() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let p3 = PhysicalQubit::new(3);
    let topology = Topology::new(
        vec![p0, p1, p2, p3],
        vec![
            (p0, p1, "known".to_string()),
            (p2, p3, "unknown".to_string()),
        ],
    )
    .unwrap();
    let mut device = Device::new("partial", HashSet::from([p0, p1, p2, p3]), topology)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap();
    device
        .add_edge_properties(
            p0,
            p1,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.10,
                ))
                .unwrap(),
        )
        .unwrap();

    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let objective = LayoutObjective::auto_from_physical(&physical);
    assert_eq!(objective.two_qubit_error_weight, 10.0);
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let analysis = analyze_circuit_for_layout(&circuit).unwrap();

    assert_eq!(
        objective
            .score_layout(
                &analysis,
                &physical,
                &layout_on_pair(&[p0, p1, p2, p3], p0, p1),
            )
            .unwrap()
            .two_qubit_error,
        0.10
    );
    assert_eq!(
        objective
            .score_layout(
                &analysis,
                &physical,
                &layout_on_pair(&[p0, p1, p2, p3], p2, p3),
            )
            .unwrap()
            .two_qubit_error,
        1.0
    );
}

#[test]
fn auto_objective_enables_only_available_fidelity_components() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p0, p1, "edge".to_string())]).unwrap();
    let mut readout_only =
        Device::new("readout", HashSet::from([p0, p1]), topology.clone()).unwrap();
    readout_only
        .add_qubit_properties(p0, QubitProp::new(0.02))
        .unwrap();
    let readout_objective = LayoutObjective::auto_from_physical(
        &PhysicalLayoutGraph::from_device(&readout_only).unwrap(),
    );
    assert_eq!(readout_objective.two_qubit_error_weight, 0.0);
    assert_eq!(readout_objective.readout_error_weight, 1.0);

    let two_qubit_only = Device::new("two-qubit", HashSet::from([p0, p1]), topology)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap()
        .with_default_two_qubit_error(0.03);
    let two_qubit_objective = LayoutObjective::auto_from_physical(
        &PhysicalLayoutGraph::from_device(&two_qubit_only).unwrap(),
    );
    assert_eq!(two_qubit_objective.two_qubit_error_weight, 10.0);
    assert_eq!(two_qubit_objective.readout_error_weight, 0.0);
}

#[test]
fn default_only_error_does_not_invent_gate_capability_or_fidelity_data() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p0, p1, "edge".to_string())]).unwrap();
    let device = Device::new("default-only", HashSet::from([p0, p1]), topology)
        .unwrap()
        .with_default_two_qubit_error(0.03);

    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    assert!(!physical.has_native_two_qubit_capabilities());
    assert!(!physical.has_two_qubit_error_data());
    assert!(!physical.supports_two_qubit_gate_directed(p0, p1, StandardGate::CX));
    assert_eq!(
        physical.two_qubit_gate_error_directed(p0, p1, StandardGate::CX),
        None
    );
    assert_eq!(
        LayoutObjective::auto_from_physical(&physical).two_qubit_error_weight,
        0.0
    );
}

#[test]
fn symmetric_gates_ignore_topology_direction_and_choose_known_orientation() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(
        vec![p0, p1],
        vec![
            (p0, p1, "unknown".to_string()),
            (p1, p0, "known".to_string()),
        ],
    )
    .unwrap();
    let mut device = Device::new("symmetric", HashSet::from([p0, p1]), topology)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CZ)])
        .unwrap();
    device
        .add_edge_properties(
            p1,
            p0,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CZ),
                    0.2,
                ))
                .unwrap(),
        )
        .unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();

    let mut circuit = Circuit::new(2);
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();
    let analysis = analyze_circuit_for_layout(&circuit).unwrap();
    let score = LayoutObjective::fidelity_aware()
        .score_layout(&analysis, &physical, &layout_on_pair(&[p0, p1], p0, p1))
        .unwrap();
    assert_eq!(score.direction, 0.0);
    assert_eq!(score.two_qubit_error, 0.2);
}

#[test]
fn reverse_only_asymmetric_native_gates_keep_direction_and_support_penalties() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p1, p0, "reverse".to_string())]).unwrap();
    let mut edge = EdgeProp::new();
    for gate in [
        StandardGate::CX,
        StandardGate::RZX,
        StandardGate::CY,
        StandardGate::CRX,
    ] {
        edge.set_native_instruction(InstructionProp::new(Instruction::Standard(gate), 0.1))
            .unwrap();
    }
    let mut device = Device::new("reverse", HashSet::from([p0, p1]), topology).unwrap();
    device.add_edge_properties(p1, p0, edge).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let objective = LayoutObjective::fidelity_aware();
    let layout = layout_on_pair(&[p0, p1], p0, p1);

    for gate in [
        StandardGate::CX,
        StandardGate::RZX,
        StandardGate::CY,
        StandardGate::CRX,
    ] {
        let mut circuit = Circuit::new(2);
        let params = if gate.num_params() == 0 {
            Vec::new()
        } else {
            vec![0.37.into()]
        };
        circuit
            .append(
                Instruction::Standard(gate),
                [Qubit::new(0), Qubit::new(1)],
                params,
                None,
            )
            .unwrap();
        let analysis = analyze_circuit_for_layout(&circuit).unwrap();
        let score = objective
            .score_layout(&analysis, &physical, &layout)
            .unwrap();
        assert_eq!(score.direction, 1.0, "gate {gate}");
        assert_eq!(score.two_qubit_error, 1.0, "gate {gate}");
    }
}

#[test]
fn topology_only_fallback_keeps_asymmetric_direction_but_not_symmetric_direction() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let device = Device::from_edges("topology", 2, &[(1, 0)]).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let objective = LayoutObjective::topology_only();
    let layout = layout_on_pair(&[p0, p1], p0, p1);

    let mut cx = Circuit::new(2);
    cx.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let cx_score = objective
        .score_layout(
            &analyze_circuit_for_layout(&cx).unwrap(),
            &physical,
            &layout,
        )
        .unwrap();
    assert_eq!(cx_score.direction, 1.0);

    let mut cz = Circuit::new(2);
    cz.cz(Qubit::new(0), Qubit::new(1)).unwrap();
    let cz_score = objective
        .score_layout(
            &analyze_circuit_for_layout(&cz).unwrap(),
            &physical,
            &layout,
        )
        .unwrap();
    assert_eq!(cz_score.direction, 0.0);
}

#[test]
fn directed_gate_uses_actual_orientation_instead_of_reverse_minimum() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(
        vec![p0, p1],
        vec![
            (p0, p1, "forward".to_string()),
            (p1, p0, "reverse".to_string()),
        ],
    )
    .unwrap();
    let mut device = Device::new("directed", HashSet::from([p0, p1]), topology).unwrap();
    for (control, target, error) in [(p0, p1, 0.10), (p1, p0, 0.01)] {
        device
            .add_edge_properties(
                control,
                target,
                EdgeProp::new()
                    .with_native_instruction(InstructionProp::new(
                        Instruction::Standard(StandardGate::CX),
                        error,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }

    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let score = LayoutObjective::fidelity_aware()
        .score_layout(
            &analyze_circuit_for_layout(&circuit).unwrap(),
            &physical,
            &layout_on_pair(&[p0, p1], p0, p1),
        )
        .unwrap();
    assert_eq!(score.direction, 0.0);
    assert_eq!(score.two_qubit_error, 0.10);
}

#[test]
fn topology_direction_and_local_gate_override_are_distinct_costs() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let p3 = PhysicalQubit::new(3);
    let topology = Topology::new(
        vec![p0, p1, p2, p3],
        vec![
            (p0, p1, "override".to_string()),
            (p2, p3, "native".to_string()),
        ],
    )
    .unwrap();
    let mut device = Device::new("override", HashSet::from([p0, p1, p2, p3]), topology).unwrap();
    device
        .add_edge_properties(
            p0,
            p1,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CZ),
                    0.01,
                ))
                .unwrap(),
        )
        .unwrap();
    device
        .add_edge_properties(
            p2,
            p3,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.02,
                ))
                .unwrap(),
        )
        .unwrap();

    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let score = LayoutObjective::fidelity_aware()
        .score_layout(
            &analyze_circuit_for_layout(&circuit).unwrap(),
            &physical,
            &layout_on_pair(&[p0, p1, p2, p3], p0, p1),
        )
        .unwrap();
    assert_eq!(score.direction, 0.0);
    assert_eq!(score.two_qubit_error, 1.0);
}

#[test]
fn device_input_or_physical_target_rejects_every_invalid_two_qubit_probability() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    for error in [f64::NAN, f64::INFINITY, -0.01, 1.01] {
        let topology = Topology::new(vec![p0, p1], vec![(p0, p1, "edge".to_string())]).unwrap();
        let default_device = Device::new("default", HashSet::from([p0, p1]), topology.clone())
            .unwrap()
            .with_default_two_qubit_error(error);
        assert!(PhysicalLayoutGraph::from_device(&default_device).is_err());

        let valid = EdgeProp::new()
            .with_native_instruction(InstructionProp::new(
                Instruction::Standard(StandardGate::CZ),
                0.01,
            ))
            .unwrap();
        assert!(
            valid
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    error,
                ))
                .is_err()
        );
    }
}

#[test]
fn mixed_standard_gates_accumulate_their_own_weighted_calibrations() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p0, p1, "edge".to_string())]).unwrap();
    let mut device = Device::new("mixed", HashSet::from([p0, p1]), topology).unwrap();
    device
        .add_edge_properties(
            p0,
            p1,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.10,
                ))
                .unwrap()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CZ),
                    0.01,
                ))
                .unwrap(),
        )
        .unwrap();

    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cz(Qubit::new(1), Qubit::new(0)).unwrap();

    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let score = LayoutObjective::fidelity_aware()
        .score_layout(
            &analyze_circuit_for_layout(&circuit).unwrap(),
            &physical,
            &layout_on_pair(&[p0, p1], p0, p1),
        )
        .unwrap();

    assert_eq!(score.direction, 0.0);
    assert!((score.two_qubit_error - 0.12).abs() < 1e-12);
}

#[test]
fn non_standard_two_qubit_operation_contributes_only_distance() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p1, p0, "reverse".to_string())]).unwrap();
    let mut device = Device::new("non-standard", HashSet::from([p0, p1]), topology).unwrap();
    device
        .add_edge_properties(
            p1,
            p0,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.05,
                ))
                .unwrap(),
        )
        .unwrap();

    let mut circuit = Circuit::new(2);
    circuit
        .unitary(
            UnitaryGate::new("opaque-two-qubit", 2, 0),
            vec![Qubit::new(0), Qubit::new(1)],
        )
        .unwrap();

    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let score = LayoutObjective::fidelity_aware()
        .score_layout(
            &analyze_circuit_for_layout(&circuit).unwrap(),
            &physical,
            &layout_on_pair(&[p0, p1], p0, p1),
        )
        .unwrap();

    assert_eq!(score.distance, 1.0);
    assert_eq!(score.direction, 0.0);
    assert_eq!(score.two_qubit_error, 0.0);
}

fn layout_on_pair(
    physical_qubits: &[PhysicalQubit],
    first: PhysicalQubit,
    second: PhysicalQubit,
) -> Layout {
    Layout::new(
        vec![LogicalQubit::new(0), LogicalQubit::new(1)],
        physical_qubits.to_vec(),
        Some(BTreeMap::from([
            (LogicalQubit::new(0), first),
            (LogicalQubit::new(1), second),
        ])),
    )
    .unwrap()
}
