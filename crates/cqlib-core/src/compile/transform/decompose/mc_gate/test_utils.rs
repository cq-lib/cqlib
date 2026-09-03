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

//! Shared helpers for multi-controlled gate decomposition tests.

use crate::circuit::{
    Circuit, Instruction, MCGate, ParameterValue, Qubit, StandardGate, ValueInstruction,
    circuit_to_matrix, operation::ValueOperation,
};
use crate::qis::Statevector;
use ndarray::Array2;
use num_complex::Complex64;
use std::collections::HashSet;

/// Tolerance for floating-point comparisons in mc_gate decomposition tests.
pub(super) const EPSILON: f64 = 1e-9;

/// Asserts that an operation is a parameter-free, unlabeled standard gate.
pub(super) fn assert_standard_operation(
    operation: &ValueOperation,
    expected_gate: StandardGate,
    expected_qubits: &[Qubit],
) {
    assert!(matches!(
        operation.instruction,
        ValueInstruction::Instruction(Instruction::Standard(gate)) if gate == expected_gate
    ));
    assert_eq!(operation.qubits.as_slice(), expected_qubits);
    assert!(operation.params.is_empty());
    assert!(operation.label.is_none());
}

/// Asserts equality for value-operation sequences.
pub(super) fn assert_value_operations_equal(
    actual: &[ValueOperation],
    expected: &[ValueOperation],
) {
    assert_eq!(actual.len(), expected.len());
    for (actual_operation, expected_operation) in actual.iter().zip(expected) {
        match (
            &actual_operation.instruction,
            &expected_operation.instruction,
        ) {
            (
                ValueInstruction::Instruction(Instruction::Standard(actual_gate)),
                ValueInstruction::Instruction(Instruction::Standard(expected_gate)),
            ) => {
                assert_eq!(actual_gate, expected_gate);
            }
            (actual_instruction, expected_instruction) => {
                panic!(
                    "instruction mismatch: actual={actual_instruction:?}, expected={expected_instruction:?}"
                );
            }
        }
        assert_eq!(actual_operation.qubits, expected_operation.qubits);
        assert_eq!(
            actual_operation.params.len(),
            expected_operation.params.len()
        );
        for (actual_parameter, expected_parameter) in actual_operation
            .params
            .iter()
            .zip(&expected_operation.params)
        {
            match (actual_parameter, expected_parameter) {
                (ParameterValue::Fixed(actual), ParameterValue::Fixed(expected)) => {
                    assert_eq!(actual.to_bits(), expected.to_bits());
                }
                (ParameterValue::Param(actual), ParameterValue::Param(expected)) => {
                    assert_eq!(actual, expected);
                }
                _ => panic!(
                    "parameter mismatch: actual={actual_parameter:?}, expected={expected_parameter:?}"
                ),
            }
        }
        assert_eq!(actual_operation.label, expected_operation.label);
    }
}

/// Asserts that every operation references only qubits from the allowed set.
pub(super) fn assert_value_operations_only_use_qubits(
    operations: &[ValueOperation],
    allowed_qubits: &[Qubit],
) {
    let allowed_qubits: HashSet<_> = allowed_qubits.iter().copied().collect();

    for (operation_index, operation) in operations.iter().enumerate() {
        for qubit in &operation.qubits {
            assert!(
                allowed_qubits.contains(qubit),
                "operation {operation_index} references disallowed qubit {qubit}: {operation:?}"
            );
        }
    }
}

/// Builds a circuit from self-contained value operations.
pub(super) fn circuit_from_value_operations(
    num_qubits: usize,
    operations: Vec<ValueOperation>,
) -> Circuit {
    Circuit::from_operations(
        (0..num_qubits)
            .map(|index| Qubit::new(index as u32))
            .collect(),
        operations,
        None,
        None,
    )
    .unwrap()
}

/// Applies value operations to a clone of an initial statevector.
pub(super) fn statevector_after_value_operations(
    initial_state: &Statevector,
    operations: &[ValueOperation],
) -> Statevector {
    let circuit = circuit_from_value_operations(initial_state.num_qubits, operations.to_vec());
    let mut statevector = initial_state.clone();
    statevector.apply_circuit(&circuit).unwrap();
    statevector
}

/// Asserts equality of complete statevectors up to one global phase.
pub(super) fn assert_statevectors_equal_up_to_global_phase(
    actual: &Statevector,
    expected: &Statevector,
    epsilon: f64,
) {
    assert_eq!(actual.num_qubits, expected.num_qubits);
    assert_eq!(actual.data().len(), expected.data().len());

    let reference_index = expected
        .data()
        .iter()
        .position(|amplitude| amplitude.norm() > epsilon)
        .expect("expected statevector must contain a nonzero amplitude");
    let actual_reference = actual.data()[reference_index];
    let expected_reference = expected.data()[reference_index];
    assert!(
        actual_reference.norm() > epsilon,
        "actual statevector has zero amplitude at reference index {reference_index}"
    );

    let global_phase = actual_reference / expected_reference;
    assert!(
        (global_phase.norm() - 1.0).abs() < epsilon,
        "statevectors differ in amplitude magnitude at reference index {reference_index}: actual={actual_reference}, expected={expected_reference}"
    );

    for (index, (actual_amplitude, expected_amplitude)) in
        actual.data().iter().zip(expected.data()).enumerate()
    {
        let phase_adjusted_expected = global_phase * expected_amplitude;
        assert!(
            (*actual_amplitude - phase_adjusted_expected).norm() < epsilon,
            "statevectors differ at index {index}: actual={actual_amplitude}, expected={phase_adjusted_expected}"
        );
    }
}

/// Asserts approximate equality for selected matrix columns.
pub(super) fn assert_selected_matrix_columns_approx_eq(
    actual: &Array2<Complex64>,
    expected: &Array2<Complex64>,
    columns: impl IntoIterator<Item = usize>,
    epsilon: f64,
) {
    assert_eq!(actual.shape(), expected.shape());
    for column in columns {
        for row in 0..expected.nrows() {
            assert!(
                (actual[[row, column]] - expected[[row, column]]).norm() < epsilon,
                "matrix mismatch at row {row}, column {column}: actual={}, expected={}",
                actual[[row, column]],
                expected[[row, column]]
            );
        }
    }
}

/// Asserts selected matrix columns are equal up to one global phase.
pub(super) fn assert_selected_matrix_columns_equal_up_to_global_phase(
    actual: &Array2<Complex64>,
    expected: &Array2<Complex64>,
    columns: impl IntoIterator<Item = usize>,
    epsilon: f64,
) {
    assert_eq!(actual.shape(), expected.shape());
    let columns: Vec<_> = columns.into_iter().collect();
    let (reference_actual, reference_expected) = columns
        .iter()
        .flat_map(|column| {
            (0..expected.nrows()).map(move |row| (actual[[row, *column]], expected[[row, *column]]))
        })
        .find(|(_, expected)| expected.norm() > epsilon)
        .expect("selected expected columns must contain a nonzero amplitude");
    let global_phase = reference_actual / reference_expected;

    assert!((global_phase.norm() - 1.0).abs() < epsilon);
    for column in columns {
        for row in 0..expected.nrows() {
            let expected_amplitude = global_phase * expected[[row, column]];
            assert!(
                (actual[[row, column]] - expected_amplitude).norm() < epsilon,
                "matrix mismatch at row {row}, column {column}: actual={}, expected={expected_amplitude}",
                actual[[row, column]]
            );
        }
    }
}

/// Asserts that an operation is a standard gate with a single fixed parameter.
pub(super) fn assert_fixed_parameter_operation(
    operation: &ValueOperation,
    expected_gate: StandardGate,
    expected_qubits: &[Qubit],
    expected_theta: f64,
) {
    assert!(matches!(
        operation.instruction,
        ValueInstruction::Instruction(Instruction::Standard(gate)) if gate == expected_gate
    ));
    assert_eq!(operation.qubits.as_slice(), expected_qubits);
    assert!(matches!(
        operation.params.as_slice(),
        [ParameterValue::Fixed(theta)] if theta.to_bits() == expected_theta.to_bits()
    ));
    assert!(operation.label.is_none());
}

/// Builds the matrix of a multi-controlled standard gate.
pub(super) fn mc_gate_matrix(
    num_qubits: usize,
    num_controls: u8,
    gate: StandardGate,
    qubits: Vec<Qubit>,
    params: impl IntoIterator<Item = ParameterValue>,
) -> Array2<Complex64> {
    let mut circuit = Circuit::new(num_qubits);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(num_controls, gate))),
            qubits,
            params,
            None,
        )
        .unwrap();
    circuit_to_matrix(&circuit, None).unwrap()
}

/// Returns the unique nonzero matrix output for one computational-basis input.
pub(super) fn single_nonzero_matrix_output(
    matrix: &Array2<Complex64>,
    input_basis_state: usize,
    epsilon: f64,
) -> (usize, Complex64) {
    let outputs: Vec<_> = (0..matrix.nrows())
        .filter_map(|output_basis_state| {
            let amplitude = matrix[[output_basis_state, input_basis_state]];
            (amplitude.norm() > epsilon).then_some((output_basis_state, amplitude))
        })
        .collect();

    assert_eq!(
        outputs.len(),
        1,
        "input basis state {input_basis_state} has outputs {outputs:?}"
    );
    outputs[0]
}
