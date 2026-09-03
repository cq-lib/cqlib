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

//! Tests for circuit_to_matrix function
//!
//! These tests verify the correctness of converting quantum circuits to their
//! unitary matrix representations.

use super::*;
use crate::circuit::Qubit;
use crate::circuit::circuit_param::ParameterValue;
use crate::circuit::error::CircuitError;
use crate::circuit::gate::{FrozenCircuit, Instruction, StandardGate, UnitaryGate};
use crate::circuit::parameter::Parameter;
use crate::circuit::symbolic_matrix::{SymbolicMatrix, standard_gate_symbolic_matrix};
use crate::circuit::test_utils::{assert_is_unitary, assert_matrix_approx_eq};
use ndarray::array;
use num_complex::Complex64;
use std::f64::consts::{PI, SQRT_2};
use std::sync::Arc;

/// Create identity matrix of given dimension
fn eye(n: usize) -> Array2<Complex64> {
    Array2::eye(n)
}

/// Create complex number from real and imaginary parts
fn c(re: f64, im: f64) -> Complex64 {
    Complex64::new(re, im)
}

fn symbolic_rx_matrix(symbol: &str) -> SymbolicMatrix {
    standard_gate_symbolic_matrix(StandardGate::RX, &[Parameter::symbol(symbol)]).unwrap()
}

fn symbolic_phase_matrix(symbol: &str) -> SymbolicMatrix {
    standard_gate_symbolic_matrix(StandardGate::Phase, &[Parameter::symbol(symbol)]).unwrap()
}

#[test]
fn test_single_qubit_identity() {
    let mut circuit = Circuit::new(1);
    circuit.i(Qubit::new(0)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let expected = eye(2);

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_single_qubit_pauli_x() {
    let mut circuit = Circuit::new(1);
    circuit.x(Qubit::new(0)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let expected = array![[c(0.0, 0.0), c(1.0, 0.0)], [c(1.0, 0.0), c(0.0, 0.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_single_qubit_pauli_y() {
    let mut circuit = Circuit::new(1);
    circuit.y(Qubit::new(0)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let expected = array![[c(0.0, 0.0), c(0.0, -1.0)], [c(0.0, 1.0), c(0.0, 0.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_single_qubit_pauli_z() {
    let mut circuit = Circuit::new(1);
    circuit.z(Qubit::new(0)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let expected = array![[c(1.0, 0.0), c(0.0, 0.0)], [c(0.0, 0.0), c(-1.0, 0.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_single_qubit_hadamard() {
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let s = 1.0 / SQRT_2;
    let expected = array![[c(s, 0.0), c(s, 0.0)], [c(s, 0.0), c(-s, 0.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_single_qubit_s_gate() {
    let mut circuit = Circuit::new(1);
    circuit.s(Qubit::new(0)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let expected = array![[c(1.0, 0.0), c(0.0, 0.0)], [c(0.0, 0.0), c(0.0, 1.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_single_qubit_t_gate() {
    let mut circuit = Circuit::new(1);
    circuit.t(Qubit::new(0)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let s = 1.0 / SQRT_2;
    let expected = array![[c(1.0, 0.0), c(0.0, 0.0)], [c(0.0, 0.0), c(s, s)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_single_qubit_rx() {
    let mut circuit = Circuit::new(1);
    circuit.rx(Qubit::new(0), PI / 2.0).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let s = 1.0 / SQRT_2;
    let expected = array![[c(s, 0.0), c(0.0, -s)], [c(0.0, -s), c(s, 0.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_single_qubit_ry() {
    let mut circuit = Circuit::new(1);
    circuit.ry(Qubit::new(0), PI / 2.0).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let s = 1.0 / SQRT_2;
    let expected = array![[c(s, 0.0), c(-s, 0.0)], [c(s, 0.0), c(s, 0.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_single_qubit_rz() {
    let mut circuit = Circuit::new(1);
    circuit.rz(Qubit::new(0), PI / 2.0).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let s = 1.0 / SQRT_2;
    let expected = array![[c(s, -s), c(0.0, 0.0)], [c(0.0, 0.0), c(s, s)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_single_qubit_rx_pi() {
    // RX(pi) = -iX, should be close to X up to global phase
    let mut circuit = Circuit::new(1);
    circuit.rx(Qubit::new(0), PI).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    // RX(pi) = [[0, -i], [-i, 0]] = -i * X
    let expected = array![[c(0.0, 0.0), c(0.0, -1.0)], [c(0.0, -1.0), c(0.0, 0.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_two_qubit_cnot_control_low() {
    // CNOT with control=0, target=1 (natural order)
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    // Expected CNOT matrix (control=0, target=1)
    // |00> -> |00>
    // |01> -> |11> (control q0=1, target q1 flips 0->1)
    // |10> -> |10>
    // |11> -> |01> (control q0=1, target q1 flips 1->0)
    let expected = array![
        [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
    ];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_two_qubit_cnot_control_high() {
    // CNOT with control=1, target=0 (reversed order)
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(1), Qubit::new(0)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    // Expected CNOT matrix (control=1, target=0)
    // |00> -> |00>
    // |01> -> |01>
    // |10> -> |11> (control q1=1, target q0 flips 0->1)
    // |11> -> |10> (control q1=1, target q0 flips 1->0)
    let expected = array![
        [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0)],
    ];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_two_qubit_cz() {
    // CZ gate
    let mut circuit = Circuit::new(2);
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    let expected = array![
        [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(-1.0, 0.0)],
    ];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_two_qubit_swap() {
    let mut circuit = Circuit::new(2);
    circuit.swap(Qubit::new(0), Qubit::new(1)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    // SWAP: |01> <-> |10>
    let expected = array![
        [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)],
    ];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_two_qubit_rxx() {
    let mut circuit = Circuit::new(2);
    circuit.rxx(Qubit::new(0), Qubit::new(1), PI / 2.0).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    let s = 1.0 / SQRT_2;
    let expected = array![
        [c(s, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, -s)],
        [c(0.0, 0.0), c(s, 0.0), c(0.0, -s), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, -s), c(s, 0.0), c(0.0, 0.0)],
        [c(0.0, -s), c(0.0, 0.0), c(0.0, 0.0), c(s, 0.0)],
    ];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_three_qubit_ccx() {
    // CCX (Toffoli) gate
    let mut circuit = Circuit::new(3);
    circuit
        .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    // CCX flips target (q2) when both controls (q0, q1) are |1>
    // Only |011> (3) <-> |111> (7) should be swapped
    let mut expected = eye(8);
    expected[[3, 3]] = c(0.0, 0.0); // |011>
    expected[[3, 7]] = c(1.0, 0.0);
    expected[[7, 3]] = c(1.0, 0.0); // |111>
    expected[[7, 7]] = c(0.0, 0.0);

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_three_qubit_non_adjacent_cnot() {
    // CNOT with non-adjacent qubits (control=0, target=2)
    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    assert_is_unitary(&matrix, 1e-10);

    // Verify specific transformations
    // Control q0=1 -> Target q2 flips.
    // |001> (q0=1, q1=0, q2=0) -> |101> (q0=1, q1=0, q2=1)
    // Index 1 -> Index 5
    assert!((matrix[[5, 1]] - c(1.0, 0.0)).norm() < 1e-10);
    assert!((matrix[[1, 1]] - c(0.0, 0.0)).norm() < 1e-10);
}

#[test]
fn test_mcgate_three_control_x() {
    let mut circuit = Circuit::new(4);
    circuit
        .multi_control(
            StandardGate::X,
            [Qubit::new(0), Qubit::new(1), Qubit::new(2)],
            [Qubit::new(3)],
            [],
        )
        .unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    assert_is_unitary(&matrix, 1e-10);

    // Little-endian (q3 q2 q1 q0):
    // Triggered when q0=1, q1=1, q2=1.
    // |0111> (index 7) <-> |1111> (index 15)
    let mut expected = eye(16);
    expected[[7, 7]] = c(0.0, 0.0);
    expected[[7, 15]] = c(1.0, 0.0);
    expected[[15, 7]] = c(1.0, 0.0);
    expected[[15, 15]] = c(0.0, 0.0);

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_mcgate_control_higher_than_target() {
    let mut circuit = Circuit::new(3);
    circuit
        .multi_control(StandardGate::X, [Qubit::new(2)], [Qubit::new(0)], [])
        .unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    assert_is_unitary(&matrix, 1e-10);

    // Little-endian (q2 q1 q0):
    // |101> (index 5) -> q2=1, q1=0, q0=1
    // Control bit q2=1, Target bit q0 flips: 1 -> 0
    // Result |100> (index 4)
    assert!((matrix[[4, 5]] - c(1.0, 0.0)).norm() < 1e-10);

    // When control bit q2=0, state remains unchanged
    // |001> (index 1) -> q2=0, q1=0, q0=1. Remains |001> (index 1)
    assert!((matrix[[1, 1]] - c(1.0, 0.0)).norm() < 1e-10);
}

#[test]
fn test_unitary_gate_single_qubit() {
    // Create a custom unitary (Pauli X for simplicity)
    let mat = array![[c(0.0, 0.0), c(1.0, 0.0)], [c(1.0, 0.0), c(0.0, 0.0)],];
    let u_gate = UnitaryGate::new("CustomX", 1, 0).with_matrix(mat).unwrap();

    let mut circuit = Circuit::new(1);
    circuit.unitary(u_gate, vec![Qubit::new(0)]).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let expected = array![[c(0.0, 0.0), c(1.0, 0.0)], [c(1.0, 0.0), c(0.0, 0.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_unitary_gate_two_qubit() {
    // Custom two-qubit gate (SWAP-like)
    let mat = array![
        [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)],
    ];
    let u_gate = UnitaryGate::new("CustomSwap", 2, 0)
        .with_matrix(mat)
        .unwrap();

    let mut circuit = Circuit::new(2);
    circuit
        .unitary(u_gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    // Verify SWAP-like behavior
    // |01> -> |10>
    assert!((matrix[[2, 1]] - c(1.0, 0.0)).norm() < 1e-10);
    // |10> -> |01>
    assert!((matrix[[1, 2]] - c(1.0, 0.0)).norm() < 1e-10);
}

#[test]
fn test_circuit_gate_bell_state() {
    // Create a Bell state preparation circuit as a gate
    let mut inner = Circuit::new(2);
    inner.h(Qubit::new(0)).unwrap();
    inner.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let gate = inner.to_gate("Bell").unwrap();

    let mut circuit = Circuit::new(2);
    circuit
        .append(gate, [Qubit::new(0), Qubit::new(1)], [], None)
        .unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    assert_is_unitary(&matrix, 1e-10);

    // Verify Bell state preparation
    // |00> -> (|00> + |11>) / sqrt(2)
    let s = 1.0 / SQRT_2;
    assert!((matrix[[0, 0]] - c(s, 0.0)).norm() < 1e-10);
    assert!((matrix[[3, 0]] - c(s, 0.0)).norm() < 1e-10);
    assert!((matrix[[1, 0]]).norm() < 1e-10);
    assert!((matrix[[2, 0]]).norm() < 1e-10);
}

#[test]
fn test_circuit_gate_with_params() {
    // Circuit gate with parameters
    let theta = Parameter::symbol("theta");
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), theta).unwrap();

    let gate = inner.to_gate("RxGate").unwrap();

    let mut circuit = Circuit::new(1);
    circuit
        .append(
            gate,
            [Qubit::new(0)],
            [ParameterValue::Fixed(PI / 2.0)],
            None,
        )
        .unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    let s = 1.0 / SQRT_2;
    let expected = array![[c(s, 0.0), c(0.0, -s)], [c(0.0, -s), c(s, 0.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_composite_h_cnot() {
    // H on q0, then CNOT(q0, q1) - Bell state preparation
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    assert_is_unitary(&matrix, 1e-10);

    // Expected: (|00> + |11>) / sqrt(2) from |00>
    let s = 1.0 / SQRT_2;
    assert!((matrix[[0, 0]] - c(s, 0.0)).norm() < 1e-10);
    assert!((matrix[[3, 0]] - c(s, 0.0)).norm() < 1e-10);
}

#[test]
fn test_composite_x_h() {
    // X then H: |0> -> |1> -> |->
    let mut circuit = Circuit::new(1);
    circuit.x(Qubit::new(0)).unwrap();
    circuit.h(Qubit::new(0)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    // H * X = [[1,1],[1,-1]]/sqrt(2) * [[0,1],[1,0]] = [[1,-1],[1,1]]/sqrt(2) (up to column ordering)
    // Actually: H * X = [[1/sqrt(2), 1/sqrt(2)], [1/sqrt(2), -1/sqrt(2)]] @ [[0, 1], [1, 0]]
    //                = [[1/sqrt(2), 1/sqrt(2)], [-1/sqrt(2), 1/sqrt(2)]]
    let s = 1.0 / SQRT_2;
    let expected = array![[c(s, 0.0), c(s, 0.0)], [c(-s, 0.0), c(s, 0.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_composite_swap_via_cnots() {
    // SWAP = CNOT(a,b) CNOT(b,a) CNOT(a,b)
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    // Expected SWAP matrix
    let expected = array![
        [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)],
    ];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_empty_circuit() {
    let circuit = Circuit::new(2);

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let expected = eye(4);

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_custom_qubit_order() {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();

    // Test with reversed qubit order
    let custom_order = vec![1, 0];
    let matrix = circuit_to_matrix(&circuit, Some(&custom_order)).unwrap();

    // With order [1, 0]:
    // q1 -> bit 0 (LSB)
    // q0 -> bit 1 (MSB)
    // H is on q0 (bit 1). I on q1 (bit 0).
    // Matrix = H ⊗ I = [[s, 0, s, 0], [0, s, 0, s], [s, 0, -s, 0], [0, s, 0, -s]]
    let s = 1.0 / SQRT_2;
    let expected = array![
        [c(s, 0.0), c(0.0, 0.0), c(s, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(s, 0.0), c(0.0, 0.0), c(s, 0.0)],
        [c(s, 0.0), c(0.0, 0.0), c(-s, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(s, 0.0), c(0.0, 0.0), c(-s, 0.0)],
    ];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

// ... (omitted)

#[test]
fn test_multiple_single_qubit_on_different_qubits() {
    // X on q0, H on q1
    let mut circuit = Circuit::new(2);
    circuit.x(Qubit::new(0)).unwrap();
    circuit.h(Qubit::new(1)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    // Default order (Little Endian): q0 LSB, q1 MSB.
    // X on q0, H on q1 => H ⊗ X
    // X = [[0, 1], [1, 0]]
    // H = [[s, s], [s, -s]]
    // H ⊗ X = [[0, s, 0, s], [s, 0, s, 0], [0, s, 0, -s], [s, 0, -s, 0]]
    let s = 1.0 / SQRT_2;
    let expected = array![
        [c(0.0, 0.0), c(s, 0.0), c(0.0, 0.0), c(s, 0.0)],
        [c(s, 0.0), c(0.0, 0.0), c(s, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(s, 0.0), c(0.0, 0.0), c(-s, 0.0)],
        [c(s, 0.0), c(0.0, 0.0), c(-s, 0.0), c(0.0, 0.0)],
    ];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_rzz_gate() {
    let mut circuit = Circuit::new(2);
    circuit.rzz(Qubit::new(0), Qubit::new(1), PI / 2.0).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    let s = 1.0 / SQRT_2;
    let expected = array![
        [c(s, -s), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(s, s), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(s, s), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(s, -s)],
    ];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_fsim_gate() {
    let mut circuit = Circuit::new(2);
    circuit
        .fsim(Qubit::new(0), Qubit::new(1), PI / 4.0, PI / 2.0)
        .unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    assert_is_unitary(&matrix, 1e-10);
}

#[test]
fn test_measure_returns_error() {
    let mut circuit = Circuit::new(1);
    circuit.measure(Qubit::new(0)).unwrap();

    assert!(matches!(
        circuit_to_matrix(&circuit, None),
        Err(CircuitError::NoMatrixRepresentation)
    ));
}

#[test]
fn test_reset_returns_error() {
    let mut circuit = Circuit::new(1);
    circuit.reset(Qubit::new(0)).unwrap();

    assert!(matches!(
        circuit_to_matrix(&circuit, None),
        Err(CircuitError::NoMatrixRepresentation)
    ));
}

#[test]
fn test_qubits_order_mismatch_returns_error() {
    let circuit = Circuit::new(2);

    assert!(matches!(
        circuit_to_matrix(&circuit, Some(&[0])),
        Err(CircuitError::InvalidOperation(_))
    ));
    assert!(matches!(
        circuit_to_matrix(&circuit, Some(&[0, 1, 2])),
        Err(CircuitError::InvalidOperation(_))
    ));
    assert!(matches!(
        circuit_to_matrix(&circuit, Some(&[0, 0])),
        Err(CircuitError::InvalidOperation(_))
    ));
}

#[test]
fn apply_gate_to_matrix_rejects_duplicate_bits() {
    let mut matrix = Array2::eye(4);
    let gate = StandardGate::CX.matrix(&[]).unwrap();

    assert!(matches!(
        apply_gate_to_matrix(&mut matrix, gate.as_ref(), &[0, 0]),
        Err(CircuitError::DuplicateQubits)
    ));
}

#[test]
fn apply_gate_to_matrix_rejects_non_standard_layout() {
    let mut matrix = Array2::eye(4).reversed_axes();
    let gate = StandardGate::H.matrix(&[]).unwrap();

    assert!(matches!(
        apply_gate_to_matrix(&mut matrix, gate.as_ref(), &[0]),
        Err(CircuitError::InvalidOperation(_))
    ));
}

#[test]
fn test_symbolic_parameter_returns_error() {
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(1);
    circuit.rx(Qubit::new(0), theta).unwrap();

    assert!(matches!(
        circuit_to_matrix(&circuit, None),
        Err(CircuitError::SymbolicParameterError)
    ));
}

#[test]
fn test_unitary_gate_without_matrix_or_circuit_returns_error() {
    let u_gate = UnitaryGate::new("Symbolic", 1, 0);
    let mut circuit = Circuit::new(1);
    circuit.unitary(u_gate, vec![Qubit::new(0)]).unwrap();

    assert!(matches!(
        circuit_to_matrix(&circuit, None),
        Err(CircuitError::NoMatrixRepresentation)
    ));
}

#[test]
fn test_unitary_gate_with_circuit_fallback() {
    let mut inner = Circuit::new(1);
    inner.x(Qubit::new(0)).unwrap();
    let u_gate = UnitaryGate::new("CircuitX", 1, 0)
        .with_circuit(Arc::new(FrozenCircuit::new(inner)))
        .unwrap();

    let mut circuit = Circuit::new(1);
    circuit.unitary(u_gate, vec![Qubit::new(0)]).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let expected = array![[c(0.0, 0.0), c(1.0, 0.0)], [c(1.0, 0.0), c(0.0, 0.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_parameterized_unitary_gate_matches_standard_rx() {
    let u_gate = UnitaryGate::new("CustomRX", 1, 1)
        .with_symbolic_matrix(["theta"], symbolic_rx_matrix("theta"))
        .unwrap();

    let mut circuit = Circuit::new(1);
    circuit
        .unitary_with_params(
            u_gate,
            vec![Qubit::new(0)],
            vec![ParameterValue::Fixed(PI / 3.0)],
        )
        .unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let expected = StandardGate::RX.matrix(&[PI / 3.0]).unwrap();
    assert_matrix_approx_eq(&matrix, expected.as_ref(), 1e-10);
}

#[test]
fn test_parameterized_unitary_gate_reuses_definition_with_different_params() {
    let gate = UnitaryGate::new("CustomPhase", 1, 1)
        .with_symbolic_matrix(["theta"], symbolic_phase_matrix("theta"))
        .unwrap();

    let mut circuit = Circuit::new(1);
    circuit
        .unitary_with_params(
            gate.clone(),
            vec![Qubit::new(0)],
            vec![ParameterValue::Fixed(0.2)],
        )
        .unwrap();
    circuit
        .unitary_with_params(gate, vec![Qubit::new(0)], vec![ParameterValue::Fixed(0.3)])
        .unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let expected = crate::circuit::gate::gate_matrix::phase_gate(0.5);
    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_parameterized_unitary_gate_symbolic_requires_binding() {
    let gate = UnitaryGate::new("CustomPhase", 1, 1)
        .with_symbolic_matrix(["theta"], symbolic_phase_matrix("theta"))
        .unwrap();
    let mut circuit = Circuit::new(1);
    circuit
        .unitary_with_params(
            gate,
            vec![Qubit::new(0)],
            vec![ParameterValue::from(Parameter::symbol("theta"))],
        )
        .unwrap();

    assert!(matches!(
        circuit_to_matrix(&circuit, None),
        Err(CircuitError::SymbolicParameterError)
    ));

    let mut bindings = std::collections::HashMap::new();
    bindings.insert("theta", PI / 4.0);
    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let matrix = circuit_to_matrix(&bound, None).unwrap();
    let expected = crate::circuit::gate::gate_matrix::phase_gate(PI / 4.0);
    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_parameterized_unitary_gate_rejects_non_finite_param() {
    let gate = UnitaryGate::new("CustomPhase", 1, 1)
        .with_symbolic_matrix(["theta"], symbolic_phase_matrix("theta"))
        .unwrap();
    let mut circuit = Circuit::new(1);
    let result = circuit.unitary_with_params(
        gate,
        vec![Qubit::new(0)],
        vec![ParameterValue::Fixed(f64::NAN)],
    );

    assert!(matches!(
        result,
        Err(CircuitError::InvalidParameterValue(0, value)) if value.is_nan()
    ));
}

#[test]
fn test_controlled_parameterized_unitary_gate_matrix() {
    let gate = UnitaryGate::new("CustomPhase", 1, 1)
        .with_symbolic_matrix(["theta"], symbolic_phase_matrix("theta"))
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .multi_control(
            Instruction::UnitaryGate(Box::new(gate)),
            [Qubit::new(0)],
            [Qubit::new(1)],
            [ParameterValue::Fixed(PI / 5.0)],
        )
        .unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let phase = Complex64::from_polar(1.0, PI / 5.0);
    let expected = array![
        [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), phase],
    ];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_circuit_gate_inner_error_is_propagated() {
    let mut inner = Circuit::new(1);
    inner.measure(Qubit::new(0)).unwrap();
    let gate = inner.to_gate("Measured").unwrap();

    let mut circuit = Circuit::new(1);
    circuit
        .append(gate, [Qubit::new(0)], std::iter::empty(), None)
        .unwrap();

    assert!(matches!(
        circuit_to_matrix(&circuit, None),
        Err(CircuitError::NoMatrixRepresentation)
    ));
}

#[test]
fn test_global_phase_is_included_for_empty_circuit() {
    let mut circuit = Circuit::new(2);
    circuit.set_global_phase(Parameter::from(PI / 2.0));

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let mut expected = eye(4);
    expected.mapv_inplace(|value| c(0.0, 1.0) * value);

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_global_phase_is_included_with_gate() {
    let mut circuit = Circuit::new(1);
    circuit.x(Qubit::new(0)).unwrap();
    circuit.set_global_phase(Parameter::from(PI));

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let expected = array![[c(0.0, 0.0), c(-1.0, 0.0)], [c(-1.0, 0.0), c(0.0, 0.0)],];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_symbolic_global_phase_returns_error() {
    let mut circuit = Circuit::new(1);
    circuit.set_global_phase(Parameter::symbol("phi"));

    assert!(matches!(
        circuit_to_matrix(&circuit, None),
        Err(CircuitError::SymbolicParameterError)
    ));
}

#[test]
fn test_bound_global_phase_is_included() {
    let mut circuit = Circuit::new(1);
    circuit.set_global_phase(Parameter::symbol("phi"));

    let mut bindings = std::collections::HashMap::new();
    bindings.insert("phi", PI / 2.0);
    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();

    let matrix = circuit_to_matrix(&bound, None).unwrap();
    let mut expected = eye(2);
    expected.mapv_inplace(|value| c(0.0, 1.0) * value);

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_circuit_gate_reversed_bits() {
    // Inner circuit: CNOT(q0 -> q1)  (asymmetric, so bit-order matters)
    let mut inner = Circuit::new(2);
    inner.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let gate = inner.to_gate("CnotGate").unwrap();

    // Apply as CircuitGate to (q1, q0) in a 2-qubit circuit
    let mut circuit = Circuit::new(2);
    circuit
        .append(gate, [Qubit::new(1), Qubit::new(0)], [], None)
        .unwrap();
    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    // Direct CNOT(q1 -> q0) should produce the same matrix
    let mut expected_circuit = Circuit::new(2);
    expected_circuit.cx(Qubit::new(1), Qubit::new(0)).unwrap();
    let expected = circuit_to_matrix(&expected_circuit, None).unwrap();

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_circuit_gate_param_count_mismatch_is_rejected_on_append() {
    let theta = Parameter::symbol("theta");
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), theta).unwrap();
    let gate = inner.to_gate("RxGate").unwrap();

    let mut circuit = Circuit::new(1);
    let err = circuit
        .append(
            gate,
            [Qubit::new(0)],
            [ParameterValue::Fixed(1.0), ParameterValue::Fixed(2.0)],
            None,
        )
        .unwrap_err();

    assert!(matches!(err, CircuitError::ParameterCountMismatch { .. }));
    assert!(circuit.operations().is_empty());
}
