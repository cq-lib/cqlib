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
use crate::circuit::Operation;
use crate::circuit::ansatz::Ansatz;
use crate::circuit::gate::standard_gate::StandardGate;

fn is_gate_on_qubit(op: &Operation, gate: StandardGate, qubit: usize) -> bool {
    op.standard_gate() == Some(gate) && op.qubits.len() == 1 && op.qubits[0].index() == qubit
}

fn is_two_qubit_gate(op: &Operation, gate: StandardGate, control: usize, target: usize) -> bool {
    op.standard_gate() == Some(gate)
        && op.qubits.len() == 2
        && op.qubits[0].index() == control
        && op.qubits[1].index() == target
}

#[test]
fn test_basic_entangler_layers_default_structure() {
    let ansatz = BasicEntanglerLayers::new(3);
    let circuit = ansatz.build_circuit("theta").unwrap();
    let ops = circuit.operations();

    assert_eq!(ansatz.num_parameters(), 3);
    assert_eq!(ops.len(), 6);
    assert!(is_gate_on_qubit(&ops[0], StandardGate::RX, 0));
    assert!(is_gate_on_qubit(&ops[1], StandardGate::RX, 1));
    assert!(is_gate_on_qubit(&ops[2], StandardGate::RX, 2));
    assert!(is_two_qubit_gate(&ops[3], StandardGate::CX, 0, 1));
    assert!(is_two_qubit_gate(&ops[4], StandardGate::CX, 1, 2));
    assert!(is_two_qubit_gate(&ops[5], StandardGate::CX, 2, 0));
}

#[test]
fn test_basic_entangler_layers_two_qubits_has_single_edge() {
    let circuit = BasicEntanglerLayers::new(2).build_circuit("theta").unwrap();
    let ops = circuit.operations();

    assert_eq!(ops.len(), 3);
    assert!(is_two_qubit_gate(&ops[2], StandardGate::CX, 0, 1));
}

#[test]
fn test_basic_entangler_layers_custom_gates() {
    let ansatz = BasicEntanglerLayers::new(2)
        .rotation_gate(StandardGate::RY)
        .entanglement_gate(StandardGate::CZ);
    let circuit = ansatz.build_circuit("theta").unwrap();
    let ops = circuit.operations();

    assert!(is_gate_on_qubit(&ops[0], StandardGate::RY, 0));
    assert!(is_gate_on_qubit(&ops[1], StandardGate::RY, 1));
    assert!(is_two_qubit_gate(&ops[2], StandardGate::CZ, 0, 1));
}

#[test]
fn test_basic_entangler_layers_rejects_invalid_gates() {
    assert!(
        BasicEntanglerLayers::new(2)
            .rotation_gate(StandardGate::CX)
            .validate()
            .is_err()
    );
    assert!(
        BasicEntanglerLayers::new(2)
            .entanglement_gate(StandardGate::RX)
            .validate()
            .is_err()
    );
}

#[test]
fn test_strongly_entangling_layers_default_structure() {
    let ansatz = StronglyEntanglingLayers::new(3);
    let circuit = ansatz.build_circuit("theta").unwrap();
    let ops = circuit.operations();

    assert_eq!(ansatz.num_parameters(), 9);
    assert_eq!(ops.len(), 6);
    assert!(is_gate_on_qubit(&ops[0], StandardGate::U, 0));
    assert_eq!(ops[0].params.len(), 3);
    assert!(is_gate_on_qubit(&ops[1], StandardGate::U, 1));
    assert!(is_gate_on_qubit(&ops[2], StandardGate::U, 2));
    assert!(is_two_qubit_gate(&ops[3], StandardGate::CX, 0, 1));
    assert!(is_two_qubit_gate(&ops[4], StandardGate::CX, 1, 2));
    assert!(is_two_qubit_gate(&ops[5], StandardGate::CX, 2, 0));
}

#[test]
fn test_strongly_entangling_layers_supports_custom_entanglement_gate() {
    let ansatz = StronglyEntanglingLayers::new(3).entanglement_gate(StandardGate::CZ);
    let circuit = ansatz.build_circuit("theta").unwrap();
    let ops = circuit.operations();

    assert!(is_two_qubit_gate(&ops[3], StandardGate::CZ, 0, 1));
    assert!(is_two_qubit_gate(&ops[4], StandardGate::CZ, 1, 2));
    assert!(is_two_qubit_gate(&ops[5], StandardGate::CZ, 2, 0));
}

#[test]
fn test_strongly_entangling_layers_custom_ranges() {
    let ansatz = StronglyEntanglingLayers::new(4).reps(2).ranges(vec![2]);
    let circuit = ansatz.build_circuit("theta").unwrap();
    let ops = circuit.operations();

    assert_eq!(ansatz.num_parameters(), 24);
    assert_eq!(ops.len(), 16);
    assert!(is_two_qubit_gate(&ops[4], StandardGate::CX, 0, 2));
    assert!(is_two_qubit_gate(&ops[5], StandardGate::CX, 1, 3));
    assert!(is_two_qubit_gate(&ops[6], StandardGate::CX, 2, 0));
    assert!(is_two_qubit_gate(&ops[7], StandardGate::CX, 3, 1));
}

#[test]
fn test_strongly_entangling_layers_rejects_invalid_ranges() {
    assert!(
        StronglyEntanglingLayers::new(3)
            .ranges(vec![])
            .validate()
            .is_err()
    );
    assert!(
        StronglyEntanglingLayers::new(3)
            .ranges(vec![0])
            .validate()
            .is_err()
    );
    assert!(
        StronglyEntanglingLayers::new(3)
            .ranges(vec![3])
            .validate()
            .is_err()
    );
}
