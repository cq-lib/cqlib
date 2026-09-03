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

use super::{VirtualPermutation, VirtualPermutationElisionStatus, elide_virtual_permutations};
use crate::circuit::test_utils::assert_circuits_equivalent_up_to_global_phase;
use crate::circuit::{Circuit, ClassicalDataOp, Instruction, Parameter, Qubit, StandardGate};
use crate::device::{Layout, LogicalQubit, PhysicalQubit};

fn logical(id: u32) -> LogicalQubit {
    LogicalQubit::new(id)
}

fn restore_output_permutation(circuit: &Circuit, permutation: &VirtualPermutation) -> Circuit {
    let mut restored = circuit.clone();
    let positions = permutation
        .original_output_to_rewritten_output()
        .keys()
        .copied()
        .collect::<Vec<_>>();
    let mut source_at_position = positions.clone();

    for destination_index in 0..positions.len() {
        let destination = positions[destination_index];
        let required_source = permutation.rewritten_output(destination).unwrap();
        let source_index = source_at_position
            .iter()
            .position(|source| *source == required_source)
            .unwrap();
        if destination_index != source_index {
            restored
                .swap(
                    positions[destination_index].qubit(),
                    positions[source_index].qubit(),
                )
                .unwrap();
            source_at_position.swap(destination_index, source_index);
        }
    }
    restored
}

#[test]
fn direction_maps_original_outputs_to_rewritten_wires() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.swap(q0, q1).unwrap();
    circuit.x(q0).unwrap();

    let result = elide_virtual_permutations(&circuit).unwrap();
    assert_eq!(result.status(), VirtualPermutationElisionStatus::Changed);
    assert_eq!(result.elided_swap_count(), 1);
    assert_eq!(
        result.virtual_permutation().rewritten_output(logical(0)),
        Some(logical(1))
    );
    assert_eq!(
        result.virtual_permutation().rewritten_output(logical(1)),
        Some(logical(0))
    );

    let rewritten = result.changed_circuit().unwrap();
    assert_eq!(rewritten.operations().len(), 1);
    assert!(matches!(
        rewritten.operations()[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert_eq!(rewritten.operations()[0].qubits.as_slice(), &[q1]);
}

#[test]
fn multiple_swaps_compose_in_program_order() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.swap(q0, q1).unwrap();
    circuit.swap(q1, q2).unwrap();
    circuit.cx(q0, q2).unwrap();

    let result = elide_virtual_permutations(&circuit).unwrap();
    let permutation = result.virtual_permutation();
    assert_eq!(permutation.rewritten_output(logical(0)), Some(logical(1)));
    assert_eq!(permutation.rewritten_output(logical(1)), Some(logical(2)));
    assert_eq!(permutation.rewritten_output(logical(2)), Some(logical(0)));

    let rewritten = result.changed_circuit().unwrap();
    assert_eq!(rewritten.operations().len(), 1);
    assert_eq!(rewritten.operations()[0].qubits.as_slice(), &[q1, q0]);
}

#[test]
fn reported_permutation_restores_full_unitary_semantics() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.h(q0).unwrap();
    circuit.swap(q0, q2).unwrap();
    circuit.rx(q0, 0.31).unwrap();
    circuit.cx(q1, q2).unwrap();
    circuit.swap(q1, q2).unwrap();
    circuit.rz(q2, -0.47).unwrap();

    let result = elide_virtual_permutations(&circuit).unwrap();
    let restored = restore_output_permutation(
        result.changed_circuit().unwrap(),
        result.virtual_permutation(),
    );
    assert_circuits_equivalent_up_to_global_phase(&restored, &circuit, 1e-10);
}

#[test]
fn partial_measurement_keeps_classical_result_and_bit_order() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.swap(q0, q2).unwrap();
    circuit.measure_bits([q0, q1]).unwrap();
    circuit.x(q2).unwrap();

    let result = elide_virtual_permutations(&circuit).unwrap();
    let rewritten = result.changed_circuit().unwrap();
    rewritten.validate().unwrap();
    assert_eq!(rewritten.classical_values(), circuit.classical_values());
    assert!(matches!(
        rewritten.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBits { .. })
    ));
    assert_eq!(rewritten.operations()[0].qubits.as_slice(), &[q2, q1]);
    assert!(matches!(
        rewritten.operations()[1].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert_eq!(rewritten.operations()[1].qubits.as_slice(), &[q0]);
}

#[test]
fn structured_control_flow_is_conservatively_skipped() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.swap(q0, q1).unwrap();
    let measured = circuit.measure(q0).unwrap();
    circuit
        .if_(measured.expr().to_bool().unwrap(), |body| body.swap(q0, q1))
        .unwrap();

    let result = elide_virtual_permutations(&circuit).unwrap();
    assert_eq!(
        result.status(),
        VirtualPermutationElisionStatus::SkippedControlFlow
    );
    assert!(result.changed_circuit().is_none());
    assert!(result.virtual_permutation().is_identity());
    assert_eq!(result.elided_swap_count(), 0);
}

#[test]
fn labeled_swap_and_operation_metadata_are_preserved() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.swap(q0, q1).unwrap();
    circuit
        .append(
            Instruction::Standard(StandardGate::SWAP),
            [q0, q1],
            [],
            Some("physical-boundary"),
        )
        .unwrap();
    circuit
        .append(
            Instruction::Standard(StandardGate::X),
            [q0],
            [],
            Some("tagged-x"),
        )
        .unwrap();

    let result = elide_virtual_permutations(&circuit).unwrap();
    let rewritten = result.changed_circuit().unwrap();
    assert_eq!(rewritten.operations().len(), 2);
    assert!(matches!(
        rewritten.operations()[0].instruction,
        Instruction::Standard(StandardGate::SWAP)
    ));
    assert_eq!(rewritten.operations()[0].qubits.as_slice(), &[q1, q0]);
    assert_eq!(
        rewritten.operations()[0].label.as_deref(),
        Some("physical-boundary")
    );
    assert_eq!(rewritten.operations()[1].qubits.as_slice(), &[q1]);
    assert_eq!(rewritten.operations()[1].label.as_deref(), Some("tagged-x"));
}

#[test]
fn global_phase_is_preserved_exactly() {
    let mut circuit = Circuit::new(2);
    circuit.set_global_phase(Parameter::from(0.375));
    circuit.swap(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = elide_virtual_permutations(&circuit).unwrap();
    assert_eq!(
        result.changed_circuit().unwrap().global_phase(),
        Parameter::from(0.375)
    );
}

#[test]
fn final_layout_composition_uses_declared_direction() {
    let mut circuit = Circuit::new(3);
    circuit.swap(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.swap(Qubit::new(1), Qubit::new(2)).unwrap();
    let permutation = elide_virtual_permutations(&circuit)
        .unwrap()
        .virtual_permutation()
        .clone();
    let rewritten_final = Layout::from_pairs(&[(0, 4), (1, 2), (2, 5)], 6).unwrap();

    let composed = permutation.compose_final_layout(&rewritten_final).unwrap();
    assert_eq!(
        composed.get_physical(logical(0)),
        Some(PhysicalQubit::new(2))
    );
    assert_eq!(
        composed.get_physical(logical(1)),
        Some(PhysicalQubit::new(5))
    );
    assert_eq!(
        composed.get_physical(logical(2)),
        Some(PhysicalQubit::new(4))
    );
}
