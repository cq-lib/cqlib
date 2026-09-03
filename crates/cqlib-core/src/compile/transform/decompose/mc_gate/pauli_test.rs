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

use super::test_utils::{
    EPSILON, assert_selected_matrix_columns_equal_up_to_global_phase, assert_standard_operation,
    assert_value_operations_equal, circuit_from_value_operations, mc_gate_matrix,
};
use super::{
    mcx::{
        decompose_mcx_1_clean_b95, decompose_mcx_1_clean_kg24, decompose_mcx_1_dirty,
        decompose_mcx_2_clean, decompose_mcx_2_dirty, decompose_mcx_n_clean, decompose_mcx_n_dirty,
        decompose_mcx_no_aux, decompose_mcx_small,
    },
    pauli::{
        decompose_pauli_1_clean_b95, decompose_pauli_1_clean_kg24, decompose_pauli_1_dirty,
        decompose_pauli_2_clean, decompose_pauli_2_dirty, decompose_pauli_n_clean,
        decompose_pauli_n_dirty, decompose_pauli_no_aux, decompose_pauli_small,
    },
};
use crate::circuit::{Qubit, StandardGate, circuit_to_matrix, operation::ValueOperation};
use crate::compile::error::CompilerError;

fn wrap_exact_mcx(
    pauli: StandardGate,
    target: Qubit,
    mcx_operations: Vec<ValueOperation>,
) -> Vec<ValueOperation> {
    let (prefix, suffix) = match pauli {
        StandardGate::X => return mcx_operations,
        StandardGate::Y => (StandardGate::SDG, StandardGate::S),
        StandardGate::Z => (StandardGate::H, StandardGate::H),
        _ => panic!("test helper accepts only Pauli axes"),
    };

    let mut operations = vec![];
    operations.push(ValueOperation::from_standard(prefix, [target], []));
    operations.extend(mcx_operations);
    operations.push(ValueOperation::from_standard(suffix, [target], []));
    operations
}

#[test]
fn trivial_pauli_uses_standard_gates_and_required_basis_conjugations() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);

    let standard_cases = [
        (StandardGate::X, vec![], q0, StandardGate::X),
        (StandardGate::X, vec![q0], q1, StandardGate::CX),
        (StandardGate::X, vec![q0, q1], q2, StandardGate::CCX),
        (StandardGate::Y, vec![], q0, StandardGate::Y),
        (StandardGate::Y, vec![q0], q1, StandardGate::CY),
        (StandardGate::Z, vec![], q0, StandardGate::Z),
        (StandardGate::Z, vec![q0], q1, StandardGate::CZ),
    ];
    for (pauli, controls, target, expected) in standard_cases {
        let operations = decompose_pauli_small(pauli, &controls, target).unwrap();

        assert_eq!(operations.len(), 1);
        let mut expected_qubits = controls;
        expected_qubits.push(target);
        assert_standard_operation(&operations[0], expected, &expected_qubits);
    }

    let z_operations = decompose_pauli_small(StandardGate::Z, &[q0, q1], q2).unwrap();
    assert_eq!(z_operations.len(), 3);
    assert_standard_operation(&z_operations[0], StandardGate::H, &[q2]);
    assert_standard_operation(&z_operations[1], StandardGate::CCX, &[q0, q1, q2]);
    assert_standard_operation(&z_operations[2], StandardGate::H, &[q2]);

    let y_operations = decompose_pauli_small(StandardGate::Y, &[q0, q1], q2).unwrap();
    assert_eq!(y_operations.len(), 3);
    assert_standard_operation(&y_operations[0], StandardGate::SDG, &[q2]);
    assert_standard_operation(&y_operations[1], StandardGate::CCX, &[q0, q1, q2]);
    assert_standard_operation(&y_operations[2], StandardGate::S, &[q2]);
}

#[test]
fn standard_controlled_paulis_use_flattened_controls_and_existing_standard_gates() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);

    let cases = [
        (StandardGate::CX, vec![q0], q1, StandardGate::CX),
        (StandardGate::CCX, vec![q0, q1], q2, StandardGate::CCX),
        (StandardGate::CY, vec![q0], q1, StandardGate::CY),
        (StandardGate::CZ, vec![q0], q1, StandardGate::CZ),
    ];
    for (pauli, controls, target, expected) in cases {
        let operations = decompose_pauli_no_aux(pauli, &controls, target).unwrap();

        assert_eq!(operations.len(), 1);
        let mut expected_qubits = controls;
        expected_qubits.push(target);
        assert_standard_operation(&operations[0], expected, &expected_qubits);
    }
}

#[test]
fn controlled_pauli_forms_select_axis_without_adding_controls() {
    let controls = [Qubit::new(0), Qubit::new(1)];
    let target = Qubit::new(2);

    let cx_operations = decompose_pauli_no_aux(StandardGate::CX, &controls, target).unwrap();
    assert_eq!(cx_operations.len(), 1);
    assert_standard_operation(
        &cx_operations[0],
        StandardGate::CCX,
        &[controls[0], controls[1], target],
    );

    let cy_operations = decompose_pauli_no_aux(StandardGate::CY, &controls, target).unwrap();
    assert_eq!(cy_operations.len(), 3);
    assert_standard_operation(&cy_operations[0], StandardGate::SDG, &[target]);
    assert_standard_operation(
        &cy_operations[1],
        StandardGate::CCX,
        &[controls[0], controls[1], target],
    );
    assert_standard_operation(&cy_operations[2], StandardGate::S, &[target]);

    let cz_operations = decompose_pauli_no_aux(StandardGate::CZ, &controls, target).unwrap();
    assert_eq!(cz_operations.len(), 3);
    assert_standard_operation(&cz_operations[0], StandardGate::H, &[target]);
    assert_standard_operation(
        &cz_operations[1],
        StandardGate::CCX,
        &[controls[0], controls[1], target],
    );
    assert_standard_operation(&cz_operations[2], StandardGate::H, &[target]);
}

#[test]
fn no_ancilla_pauli_decompositions_match_mcgate_semantics() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let target = Qubit::new(3);

    for pauli in [StandardGate::X, StandardGate::Y, StandardGate::Z] {
        let operations = decompose_pauli_no_aux(pauli, &controls, target).unwrap();
        let actual =
            circuit_to_matrix(&circuit_from_value_operations(4, operations), None).unwrap();
        let mut qubits = controls.to_vec();
        qubits.push(target);
        let expected = mc_gate_matrix(4, controls.len() as u8, pauli, qubits, []);

        assert_selected_matrix_columns_equal_up_to_global_phase(
            &actual,
            &expected,
            0..expected.ncols(),
            EPSILON,
        );
    }
}

#[test]
fn public_pauli_algorithms_delegate_to_matching_exact_mcx_algorithms() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let target = Qubit::new(3);
    let first_ancilla = Qubit::new(4);
    let second_ancilla = Qubit::new(5);
    let pauli = StandardGate::Y;

    assert_value_operations_equal(
        &decompose_pauli_small(pauli, &controls[..2], target).unwrap(),
        &wrap_exact_mcx(
            pauli,
            target,
            decompose_mcx_small(&controls[..2], target).unwrap(),
        ),
    );
    assert_value_operations_equal(
        &decompose_pauli_no_aux(pauli, &controls, target).unwrap(),
        &wrap_exact_mcx(
            pauli,
            target,
            decompose_mcx_no_aux(&controls, target).unwrap(),
        ),
    );
    assert_value_operations_equal(
        &decompose_pauli_n_clean(pauli, &controls, target, &[first_ancilla]).unwrap(),
        &wrap_exact_mcx(
            pauli,
            target,
            decompose_mcx_n_clean(&controls, target, &[first_ancilla]).unwrap(),
        ),
    );
    assert_value_operations_equal(
        &decompose_pauli_n_dirty(pauli, &controls, target, &[first_ancilla]).unwrap(),
        &wrap_exact_mcx(
            pauli,
            target,
            decompose_mcx_n_dirty(&controls, target, &[first_ancilla]).unwrap(),
        ),
    );
    assert_value_operations_equal(
        &decompose_pauli_1_clean_b95(pauli, &controls, target, first_ancilla).unwrap(),
        &wrap_exact_mcx(
            pauli,
            target,
            decompose_mcx_1_clean_b95(&controls, target, first_ancilla).unwrap(),
        ),
    );
    assert_value_operations_equal(
        &decompose_pauli_1_clean_kg24(pauli, &controls, target, first_ancilla).unwrap(),
        &wrap_exact_mcx(
            pauli,
            target,
            decompose_mcx_1_clean_kg24(&controls, target, first_ancilla).unwrap(),
        ),
    );
    assert_value_operations_equal(
        &decompose_pauli_1_dirty(pauli, &controls, target, first_ancilla).unwrap(),
        &wrap_exact_mcx(
            pauli,
            target,
            decompose_mcx_1_dirty(&controls, target, first_ancilla).unwrap(),
        ),
    );
    assert_value_operations_equal(
        &decompose_pauli_2_clean(pauli, &controls, target, [first_ancilla, second_ancilla])
            .unwrap(),
        &wrap_exact_mcx(
            pauli,
            target,
            decompose_mcx_2_clean(&controls, target, [first_ancilla, second_ancilla]).unwrap(),
        ),
    );
    assert_value_operations_equal(
        &decompose_pauli_2_dirty(pauli, &controls, target, [first_ancilla, second_ancilla])
            .unwrap(),
        &wrap_exact_mcx(
            pauli,
            target,
            decompose_mcx_2_dirty(&controls, target, [first_ancilla, second_ancilla]).unwrap(),
        ),
    );
}

#[test]
fn clean_and_dirty_pauli_decompositions_preserve_ancilla_contracts() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let target = Qubit::new(3);
    let ancilla = Qubit::new(4);

    let clean_operations =
        decompose_pauli_n_clean(StandardGate::Z, &controls, target, &[ancilla]).unwrap();
    let clean_actual =
        circuit_to_matrix(&circuit_from_value_operations(5, clean_operations), None).unwrap();
    let mut clean_qubits = controls.to_vec();
    clean_qubits.push(target);
    let clean_expected = mc_gate_matrix(5, controls.len() as u8, StandardGate::Z, clean_qubits, []);
    let clean_columns = (0..clean_expected.ncols()).filter(|state| state & (1 << 4) == 0);
    assert_selected_matrix_columns_equal_up_to_global_phase(
        &clean_actual,
        &clean_expected,
        clean_columns,
        EPSILON,
    );

    let dirty_operations =
        decompose_pauli_n_dirty(StandardGate::Y, &controls, target, &[ancilla]).unwrap();
    let dirty_actual =
        circuit_to_matrix(&circuit_from_value_operations(5, dirty_operations), None).unwrap();
    let mut dirty_qubits = controls.to_vec();
    dirty_qubits.push(target);
    let dirty_expected = mc_gate_matrix(5, controls.len() as u8, StandardGate::Y, dirty_qubits, []);
    assert_selected_matrix_columns_equal_up_to_global_phase(
        &dirty_actual,
        &dirty_expected,
        0..dirty_expected.ncols(),
        EPSILON,
    );
}

#[test]
fn invalid_pauli_is_rejected_before_mcx_synthesis() {
    let controls = [Qubit::new(0), Qubit::new(0), Qubit::new(1)];
    let error =
        decompose_pauli_n_clean(StandardGate::H, &controls, Qubit::new(2), &[]).unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.pauli",
            ref reason,
        } if reason == "multi-controlled Pauli decomposition supports only X, CX, CCX, Y, CY, Z, or CZ, got H"
    ));
}

#[test]
fn standard_gate_fast_path_rejects_duplicate_qubits() {
    let duplicate = Qubit::new(0);
    let error = decompose_pauli_no_aux(StandardGate::CY, &[duplicate], duplicate).unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mcx",
            ref reason,
        } if reason
            == &format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {duplicate}"
            )
    ));
}

#[test]
fn mcx_errors_are_propagated_without_rewriting() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let error =
        decompose_pauli_n_clean(StandardGate::X, &controls, Qubit::new(3), &[]).unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mcx",
            ref reason,
        } if reason
            == "clean-ancilla MCX decomposition with 3 controls requires 1 clean ancillas, got 0"
    ));
}

#[test]
fn trivial_inputs_do_not_consume_or_validate_unused_ancillas() {
    let control = Qubit::new(0);
    let target = Qubit::new(1);

    let operations =
        decompose_pauli_n_clean(StandardGate::Y, &[control], target, &[target]).unwrap();
    let expected = decompose_pauli_small(StandardGate::Y, &[control], target).unwrap();

    assert_value_operations_equal(&operations, &expected);
}
