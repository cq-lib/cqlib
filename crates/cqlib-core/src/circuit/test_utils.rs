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

//! Shared matrix and circuit assertions for crate-local tests.

use super::{Circuit, circuit_to_matrix};
use ndarray::Array2;
use num_complex::Complex64;

/// Asserts approximate equality of complete matrices.
pub(crate) fn assert_matrix_approx_eq(
    actual: &Array2<Complex64>,
    expected: &Array2<Complex64>,
    epsilon: f64,
) {
    assert_eq!(actual.shape(), expected.shape());
    for ((row, column), expected_amplitude) in expected.indexed_iter() {
        assert!(
            (actual[[row, column]] - expected_amplitude).norm() < epsilon,
            "matrix mismatch at row {row}, column {column}: actual={}, expected={expected_amplitude}",
            actual[[row, column]]
        );
    }
}

/// Asserts that a matrix is unitary: U† * U = I.
pub(crate) fn assert_is_unitary(matrix: &Array2<Complex64>, epsilon: f64) {
    assert_eq!(
        matrix.nrows(),
        matrix.ncols(),
        "unitary matrix must be square, got {}x{}",
        matrix.nrows(),
        matrix.ncols()
    );

    let product = matrix.t().mapv(|value| value.conj()).dot(matrix);
    for row in 0..matrix.nrows() {
        for column in 0..matrix.ncols() {
            let expected = if row == column {
                Complex64::new(1.0, 0.0)
            } else {
                Complex64::new(0.0, 0.0)
            };
            let diff = (product[[row, column]] - expected).norm();
            assert!(
                diff < epsilon,
                "matrix is not unitary at row {row}, column {column}: actual={}, expected={expected}, diff={diff}",
                product[[row, column]]
            );
        }
    }
}

/// Asserts that two matrices are equal up to one global phase.
pub(crate) fn assert_matrices_equal_up_to_global_phase(
    actual: &Array2<Complex64>,
    expected: &Array2<Complex64>,
    epsilon: f64,
) {
    assert_eq!(actual.shape(), expected.shape());
    let (reference_actual, reference_expected) = actual
        .iter()
        .zip(expected.iter())
        .find(|(_, expected)| expected.norm() > epsilon)
        .expect("expected matrix must contain a nonzero amplitude");
    assert!(
        reference_actual.norm() > epsilon,
        "actual matrix has zero amplitude where expected matrix is nonzero"
    );

    let global_phase = reference_actual / reference_expected;
    assert!(
        (global_phase.norm() - 1.0).abs() < epsilon,
        "matrices differ in reference amplitude magnitude: actual={reference_actual}, expected={reference_expected}"
    );

    for ((row, column), expected_amplitude) in expected.indexed_iter() {
        let phase_adjusted_expected = global_phase * expected_amplitude;
        assert!(
            (actual[[row, column]] - phase_adjusted_expected).norm() < epsilon,
            "matrix mismatch at row {row}, column {column}: actual={}, expected={phase_adjusted_expected}",
            actual[[row, column]]
        );
    }
}

/// Asserts that two circuits have the same unitary matrix up to global phase.
pub(crate) fn assert_circuits_equivalent_up_to_global_phase(
    actual: &Circuit,
    expected: &Circuit,
    epsilon: f64,
) {
    let actual_matrix = circuit_to_matrix(actual, None).unwrap();
    let expected_matrix = circuit_to_matrix(expected, None).unwrap();
    assert_matrices_equal_up_to_global_phase(&actual_matrix, &expected_matrix, epsilon);
}

/// Returns whether two matrices are equal up to one global phase.
///
/// Non-panicking counterpart of [`assert_matrices_equal_up_to_global_phase`],
/// for negative assertions that must not emit panic-hook output.
pub(crate) fn matrices_equal_up_to_global_phase(
    actual: &Array2<Complex64>,
    expected: &Array2<Complex64>,
    epsilon: f64,
) -> bool {
    if actual.shape() != expected.shape() {
        return false;
    }
    let Some((reference_actual, reference_expected)) = actual
        .iter()
        .zip(expected.iter())
        .find(|(_, expected)| expected.norm() > epsilon)
    else {
        return false;
    };
    if reference_actual.norm() <= epsilon {
        return false;
    }

    let global_phase = reference_actual / reference_expected;
    if (global_phase.norm() - 1.0).abs() >= epsilon {
        return false;
    }

    actual
        .iter()
        .zip(expected.iter())
        .all(|(a, e)| (a - global_phase * e).norm() < epsilon)
}

/// Returns whether two circuits have the same unitary matrix up to global phase.
///
/// Non-panicking counterpart of [`assert_circuits_equivalent_up_to_global_phase`].
pub(crate) fn circuits_equal_up_to_global_phase(
    actual: &Circuit,
    expected: &Circuit,
    epsilon: f64,
) -> bool {
    let actual_matrix = circuit_to_matrix(actual, None).unwrap();
    let expected_matrix = circuit_to_matrix(expected, None).unwrap();
    matrices_equal_up_to_global_phase(&actual_matrix, &expected_matrix, epsilon)
}
