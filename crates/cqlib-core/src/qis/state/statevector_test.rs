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
use crate::circuit::{Qubit, StandardGate};
use crate::qis::hamiltonian::Hamiltonian;
use crate::qis::pauli::{Pauli, PauliString};
use num_complex::Complex64;
use std::f64::consts::{FRAC_1_SQRT_2, PI};

// Tolerance for floating point comparisons
const EPSILON: f64 = 1e-10;

/// Helper: Create a complex number
fn c(re: f64, im: f64) -> Complex64 {
    Complex64::new(re, im)
}

/// Helper: Check if two complex numbers are approximately equal
fn assert_complex_eq(a: Complex64, b: Complex64, msg: &str) {
    assert!(
        (a - b).norm() < EPSILON,
        "{}: expected {:?}, got {:?}, diff = {:?}",
        msg,
        b,
        a,
        (a - b).norm()
    );
}

/// Helper: Check if statevector is normalized
fn assert_normalized(sv: &Statevector) {
    let norm: f64 = sv.data.iter().map(|c| c.norm_sqr()).sum();
    assert!(
        (norm - 1.0).abs() < EPSILON,
        "Statevector not normalized: norm = {}",
        norm
    );
}

#[test]
fn test_new_initialization() {
    let sv = Statevector::new(3);
    assert_eq!(sv.num_qubits, 3);
    assert_eq!(sv.data.len(), 8);

    // Check |000⟩ state
    assert_complex_eq(sv.data[0], c(1.0, 0.0), "First element should be |000⟩");

    // All other elements should be zero
    for i in 1..8 {
        assert_complex_eq(
            sv.data[i],
            c(0.0, 0.0),
            &format!("Element {} should be 0", i),
        );
    }
}

#[test]
fn test_probabilities() {
    let mut sv = Statevector::new(2);

    // Apply Hadamard to qubit 0: |00⟩ -> (|00⟩ + |01⟩)/√2
    let h_matrix = [
        [c(FRAC_1_SQRT_2, 0.0), c(FRAC_1_SQRT_2, 0.0)],
        [c(FRAC_1_SQRT_2, 0.0), c(-FRAC_1_SQRT_2, 0.0)],
    ];
    sv.apply_single_qubit_gate(0, h_matrix).unwrap();

    let probs = sv.probabilities();
    assert_eq!(probs.len(), 4);

    // |00⟩: 0.5, |01⟩: 0.5, |10⟩: 0, |11⟩: 0
    assert!((probs[0] - 0.5).abs() < EPSILON, "P(|00⟩) should be 0.5");
    assert!((probs[1] - 0.5).abs() < EPSILON, "P(|01⟩) should be 0.5");
    assert!(probs[2].abs() < EPSILON, "P(|10⟩) should be 0");
    assert!(probs[3].abs() < EPSILON, "P(|11⟩) should be 0");
}

#[test]
fn test_apply_x() {
    // |0⟩ -> |1⟩
    let mut sv = Statevector::new(1);
    sv.apply_x(0).unwrap();

    assert_complex_eq(sv.data[0], c(0.0, 0.0), "X|0⟩[0] should be 0");
    assert_complex_eq(sv.data[1], c(1.0, 0.0), "X|0⟩[1] should be 1");

    // X * X = I
    let mut sv2 = Statevector::new(2);
    sv2.apply_x(0).unwrap();
    sv2.apply_x(0).unwrap();
    assert_complex_eq(sv2.data[0], c(1.0, 0.0), "XX|00⟩ should be |00⟩");

    // Test on higher qubit in multi-qubit system
    let mut sv3 = Statevector::new(2);
    sv3.apply_x(1).unwrap(); // Flip qubit 1 (second qubit): |00⟩ -> |10⟩
    assert_complex_eq(sv3.data[0], c(0.0, 0.0), "X[1]|00⟩[0] should be 0");
    assert_complex_eq(sv3.data[1], c(0.0, 0.0), "X[1]|00⟩[1] should be 0");
    assert_complex_eq(sv3.data[2], c(1.0, 0.0), "X[1]|00⟩[2] should be 1");
    assert_complex_eq(sv3.data[3], c(0.0, 0.0), "X[1]|00⟩[3] should be 0");
}

#[test]
fn test_apply_y() {
    // |0⟩ -> i|1⟩
    let mut sv = Statevector::new(1);
    sv.apply_y(0).unwrap();

    assert_complex_eq(sv.data[0], c(0.0, 0.0), "Y|0⟩[0] should be 0");
    assert_complex_eq(sv.data[1], c(0.0, 1.0), "Y|0⟩[1] should be i");

    // Y^4 = I
    let mut sv2 = Statevector::new(1);
    for _ in 0..4 {
        sv2.apply_y(0).unwrap();
    }
    assert_complex_eq(sv2.data[0], c(1.0, 0.0), "Y^4|0⟩ should be |0⟩");
    assert_complex_eq(sv2.data[1], c(0.0, 0.0), "Y^4|0⟩[1] should be 0");
}

#[test]
fn test_apply_z() {
    // Z|0⟩ = |0⟩
    let mut sv = Statevector::new(1);
    sv.apply_z(0).unwrap();
    assert_complex_eq(sv.data[0], c(1.0, 0.0), "Z|0⟩ should be |0⟩");
    assert_complex_eq(sv.data[1], c(0.0, 0.0), "Z|0⟩[1] should be 0");

    // Z|1⟩ = -|1⟩
    let mut sv2 = Statevector::new(1);
    sv2.apply_x(0).unwrap();
    sv2.apply_z(0).unwrap();
    assert_complex_eq(sv2.data[0], c(0.0, 0.0), "ZX|0⟩[0] should be 0");
    assert_complex_eq(sv2.data[1], c(-1.0, 0.0), "ZX|0⟩ should be -|1⟩");

    // Z^2 = I
    let mut sv3 = Statevector::new(2);
    sv3.apply_x(0).unwrap();
    sv3.apply_z(0).unwrap();
    sv3.apply_z(0).unwrap();
    assert_complex_eq(sv3.data[1], c(1.0, 0.0), "Z^2|01⟩ should be |01⟩");
}

#[test]
fn test_apply_hadamard() {
    // H = [[1/√2, 1/√2], [1/√2, -1/√2]]
    let h_matrix = [
        [c(FRAC_1_SQRT_2, 0.0), c(FRAC_1_SQRT_2, 0.0)],
        [c(FRAC_1_SQRT_2, 0.0), c(-FRAC_1_SQRT_2, 0.0)],
    ];

    // H|0⟩ = (|0⟩ + |1⟩)/√2
    let mut sv = Statevector::new(1);
    sv.apply_single_qubit_gate(0, h_matrix).unwrap();

    assert_complex_eq(sv.data[0], c(FRAC_1_SQRT_2, 0.0), "H|0⟩[0] should be 1/√2");
    assert_complex_eq(sv.data[1], c(FRAC_1_SQRT_2, 0.0), "H|0⟩[1] should be 1/√2");

    // H|1⟩ = (|0⟩ - |1⟩)/√2
    let mut sv2 = Statevector::new(1);
    sv2.apply_x(0).unwrap();
    sv2.apply_single_qubit_gate(0, h_matrix).unwrap();

    assert_complex_eq(sv2.data[0], c(FRAC_1_SQRT_2, 0.0), "H|1⟩[0] should be 1/√2");
    assert_complex_eq(
        sv2.data[1],
        c(-FRAC_1_SQRT_2, 0.0),
        "H|1⟩[1] should be -1/√2",
    );

    // H^2 = I
    let mut sv3 = Statevector::new(1);
    sv3.apply_single_qubit_gate(0, h_matrix).unwrap();
    sv3.apply_single_qubit_gate(0, h_matrix).unwrap();
    assert_complex_eq(sv3.data[0], c(1.0, 0.0), "H^2|0⟩ should be |0⟩");
    assert_complex_eq(sv3.data[1], c(0.0, 0.0), "H^2|0⟩[1] should be 0");
}

#[test]
fn test_apply_s_gate() {
    // S|0⟩ = |0⟩
    let mut sv = Statevector::new(1);
    sv.apply_s(0).unwrap();
    assert_complex_eq(sv.data[0], c(1.0, 0.0), "S|0⟩ should be |0⟩");

    // S|1⟩ = i|1⟩
    let mut sv2 = Statevector::new(1);
    sv2.apply_x(0).unwrap();
    sv2.apply_s(0).unwrap();
    assert_complex_eq(sv2.data[1], c(0.0, 1.0), "S|1⟩ should be i|1⟩");

    // S^2 = Z
    let mut sv3 = Statevector::new(1);
    sv3.apply_x(0).unwrap();
    sv3.apply_s(0).unwrap();
    sv3.apply_s(0).unwrap();
    assert_complex_eq(sv3.data[1], c(-1.0, 0.0), "S^2|1⟩ should be -|1⟩");
}

#[test]
fn test_apply_t_gate() {
    // T|0⟩ = |0⟩
    let mut sv = Statevector::new(1);
    sv.apply_t(0).unwrap();
    assert_complex_eq(sv.data[0], c(1.0, 0.0), "T|0⟩ should be |0⟩");

    // T|1⟩ = e^(iπ/4)|1⟩
    let mut sv2 = Statevector::new(1);
    sv2.apply_x(0).unwrap();
    sv2.apply_t(0).unwrap();
    let expected_phase = c(FRAC_1_SQRT_2, FRAC_1_SQRT_2);
    assert_complex_eq(sv2.data[1], expected_phase, "T|1⟩ should be e^(iπ/4)|1⟩");

    // T^2 = S
    let mut sv3 = Statevector::new(1);
    sv3.apply_x(0).unwrap();
    sv3.apply_t(0).unwrap();
    sv3.apply_t(0).unwrap();
    assert_complex_eq(sv3.data[1], c(0.0, 1.0), "T^2|1⟩ should be i|1⟩ = S|1⟩");
}

#[test]
fn test_apply_rx() {
    // Rx(π)|0⟩ = -i|1⟩
    let mut sv = Statevector::new(1);
    sv.apply_rx(0, PI).unwrap();
    assert_complex_eq(sv.data[0], c(0.0, 0.0), "Rx(π)|0⟩[0] should be 0");
    assert_complex_eq(sv.data[1], c(0.0, -1.0), "Rx(π)|0⟩ should be -i|1⟩");

    // Rx(2π) = -I (global phase -1)
    let mut sv2 = Statevector::new(1);
    sv2.apply_rx(0, 2.0 * PI).unwrap();
    assert_complex_eq(sv2.data[0], c(-1.0, 0.0), "Rx(2π)|0⟩ should be -|0⟩");

    // Rx(0) = I
    let mut sv3 = Statevector::new(1);
    sv3.apply_rx(0, 0.0).unwrap();
    assert_complex_eq(sv3.data[0], c(1.0, 0.0), "Rx(0)|0⟩ should be |0⟩");
}

#[test]
fn test_apply_ry() {
    // Ry(π)|0⟩ = |1⟩
    let mut sv = Statevector::new(1);
    sv.apply_ry(0, PI).unwrap();
    assert_complex_eq(sv.data[0], c(0.0, 0.0), "Ry(π)|0⟩[0] should be 0");
    assert_complex_eq(sv.data[1], c(1.0, 0.0), "Ry(π)|0⟩ should be |1⟩");

    // Ry(π/2)|0⟩ = (|0⟩ + |1⟩)/√2
    let mut sv2 = Statevector::new(1);
    sv2.apply_ry(0, PI / 2.0).unwrap();
    assert_complex_eq(
        sv2.data[0],
        c(FRAC_1_SQRT_2, 0.0),
        "Ry(π/2)|0⟩[0] should be 1/√2",
    );
    assert_complex_eq(
        sv2.data[1],
        c(FRAC_1_SQRT_2, 0.0),
        "Ry(π/2)|0⟩[1] should be 1/√2",
    );
}

#[test]
fn test_apply_rz() {
    // Rz(θ)|0⟩ = e^(-iθ/2)|0⟩
    let mut sv = Statevector::new(1);
    sv.apply_rz(0, PI).unwrap();
    assert_complex_eq(sv.data[0], c(0.0, -1.0), "Rz(π)|0⟩ should be -i|0⟩");

    // Rz(θ)|1⟩ = e^(iθ/2)|1⟩
    let mut sv2 = Statevector::new(1);
    sv2.apply_x(0).unwrap();
    sv2.apply_rz(0, PI).unwrap();
    assert_complex_eq(sv2.data[1], c(0.0, 1.0), "Rz(π)|1⟩ should be i|1⟩");

    // Rz(2π) = -I
    let mut sv3 = Statevector::new(1);
    sv3.apply_rz(0, 2.0 * PI).unwrap();
    assert_complex_eq(sv3.data[0], c(-1.0, 0.0), "Rz(2π)|0⟩ should be -|0⟩");
}

#[test]
fn test_y2p_y2m() {
    // Y2P = Ry(π/2)
    let mut sv1 = Statevector::new(1);
    sv1.apply_y2p(0).unwrap();

    let mut sv2 = Statevector::new(1);
    sv2.apply_ry(0, PI / 2.0).unwrap();

    assert_complex_eq(sv1.data[0], sv2.data[0], "Y2P[0] should equal Ry(π/2)[0]");
    assert_complex_eq(sv1.data[1], sv2.data[1], "Y2P[1] should equal Ry(π/2)[1]");

    // Y2P * Y2M = I (approximately)
    let mut sv3 = Statevector::new(1);
    sv3.apply_y2p(0).unwrap();
    sv3.apply_y2m(0).unwrap();
    assert_complex_eq(sv3.data[0], c(1.0, 0.0), "Y2P * Y2M should be I");
}

#[test]
fn test_x2p_x2m() {
    // X2P = Rx(π/2)
    let mut sv1 = Statevector::new(1);
    sv1.apply_x2p(0).unwrap();

    let mut sv2 = Statevector::new(1);
    sv2.apply_rx(0, PI / 2.0).unwrap();

    assert_complex_eq(sv1.data[0], sv2.data[0], "X2P[0] should equal Rx(π/2)[0]");
    assert_complex_eq(sv1.data[1], sv2.data[1], "X2P[1] should equal Rx(π/2)[1]");

    // X2P * X2M = I
    let mut sv3 = Statevector::new(1);
    sv3.apply_x2p(0).unwrap();
    sv3.apply_x2m(0).unwrap();
    assert_complex_eq(sv3.data[0], c(1.0, 0.0), "X2P * X2M should be I");
}

#[test]
fn test_apply_p_gate() {
    // P(0) = I
    let mut sv1 = Statevector::new(1);
    sv1.apply_phase(0, 0.0).unwrap();
    assert_complex_eq(sv1.data[0], c(1.0, 0.0), "P(0)|0⟩ should be |0⟩");

    // P(π)|1⟩ = -|1⟩
    let mut sv2 = Statevector::new(1);
    sv2.apply_x(0).unwrap();
    sv2.apply_phase(0, PI).unwrap();
    assert_complex_eq(sv2.data[1], c(-1.0, 0.0), "P(π)|1⟩ should be -|1⟩");

    // P(π/2)|1⟩ = i|1⟩
    let mut sv3 = Statevector::new(1);
    sv3.apply_x(0).unwrap();
    sv3.apply_phase(0, PI / 2.0).unwrap();
    assert_complex_eq(sv3.data[1], c(0.0, 1.0), "P(π/2)|1⟩ should be i|1⟩");
}

#[test]
fn test_cx_control_lt_target() {
    // CX(0, 1): control=0 (low), target=1 (high)
    // |10⟩ -> |11⟩ (control qubit 0 is 0, no flip)
    // |10⟩: binary 10 = index 2, control bit is 0, so target shouldn't flip

    // Actually: |10⟩ means qubit1=1, qubit0=0
    // CX(0,1): if qubit0=1, flip qubit1
    // |10⟩: qubit0=0, so no change -> |10⟩
    let mut sv = Statevector::new(2);
    sv.apply_x(1).unwrap(); // |00⟩ -> |10⟩ (index 2)
    sv.apply_cx(0, 1).unwrap();
    assert_complex_eq(sv.data[2], c(1.0, 0.0), "CX(0,1)|10⟩ should be |10⟩");

    // |01⟩: qubit0=1, flip qubit1: |01⟩ -> |11⟩
    let mut sv2 = Statevector::new(2);
    sv2.apply_x(0).unwrap(); // |00⟩ -> |01⟩ (index 1)
    sv2.apply_cx(0, 1).unwrap();
    assert_complex_eq(sv2.data[0], c(0.0, 0.0), "CX(0,1)|01⟩[0] should be 0");
    assert_complex_eq(sv2.data[1], c(0.0, 0.0), "CX(0,1)|01⟩[1] should be 0");
    assert_complex_eq(sv2.data[2], c(0.0, 0.0), "CX(0,1)|01⟩[2] should be 0");
    assert_complex_eq(sv2.data[3], c(1.0, 0.0), "CX(0,1)|01⟩ should be |11⟩");
}

#[test]
fn test_cx_control_gt_target() {
    // CX(1, 0): control=1 (high), target=0 (low)
    // |01⟩: qubit1=0, no flip
    let mut sv = Statevector::new(2);
    sv.apply_x(0).unwrap(); // |00⟩ -> |01⟩ (index 1)
    sv.apply_cx(1, 0).unwrap();
    assert_complex_eq(sv.data[1], c(1.0, 0.0), "CX(1,0)|01⟩ should be |01⟩");

    // |10⟩: qubit1=1, flip qubit0: |10⟩ -> |11⟩
    let mut sv2 = Statevector::new(2);
    sv2.apply_x(1).unwrap(); // |00⟩ -> |10⟩ (index 2)
    sv2.apply_cx(1, 0).unwrap();
    assert_complex_eq(sv2.data[3], c(1.0, 0.0), "CX(1,0)|10⟩ should be |11⟩");
}

#[test]
fn test_cx_bell_state() {
    // Create Bell state: |00⟩ -> (|00⟩ + |11⟩)/√2
    let mut sv = Statevector::new(2);

    // Apply H to qubit 0
    let h_matrix = [
        [c(FRAC_1_SQRT_2, 0.0), c(FRAC_1_SQRT_2, 0.0)],
        [c(FRAC_1_SQRT_2, 0.0), c(-FRAC_1_SQRT_2, 0.0)],
    ];
    sv.apply_single_qubit_gate(0, h_matrix).unwrap();

    // Apply CX(0, 1)
    sv.apply_cx(0, 1).unwrap();

    // Result should be (|00⟩ + |11⟩)/√2
    assert_complex_eq(sv.data[0], c(FRAC_1_SQRT_2, 0.0), "Bell state |00⟩ amp");
    assert_complex_eq(sv.data[1], c(0.0, 0.0), "Bell state |01⟩ amp should be 0");
    assert_complex_eq(sv.data[2], c(0.0, 0.0), "Bell state |10⟩ amp should be 0");
    assert_complex_eq(sv.data[3], c(FRAC_1_SQRT_2, 0.0), "Bell state |11⟩ amp");

    assert_normalized(&sv);
}

#[test]
fn test_cx_cx_identity() {
    // CX * CX = I
    let mut sv = Statevector::new(2);
    sv.apply_x(0).unwrap();
    sv.apply_cx(0, 1).unwrap();
    sv.apply_cx(0, 1).unwrap();
    assert_complex_eq(sv.data[1], c(1.0, 0.0), "CX^2 should be I");
}

#[test]
fn test_cz_symmetric() {
    // CZ should be symmetric: CZ(0,1) = CZ(1,0)

    // CZ(0,1) on |11⟩
    let mut sv1 = Statevector::new(2);
    sv1.apply_x(0).unwrap();
    sv1.apply_x(1).unwrap();
    sv1.apply_cz(0, 1).unwrap();

    // CZ(1,0) on |11⟩
    let mut sv2 = Statevector::new(2);
    sv2.apply_x(0).unwrap();
    sv2.apply_x(1).unwrap();
    sv2.apply_cz(1, 0).unwrap();

    assert_complex_eq(sv1.data[3], sv2.data[3], "CZ should be symmetric");
    assert_complex_eq(sv1.data[3], c(-1.0, 0.0), "CZ|11⟩ should be -|11⟩");
}

#[test]
fn test_cz_phases() {
    // CZ|00⟩ = |00⟩
    // CZ|01⟩ = |01⟩
    // CZ|10⟩ = |10⟩
    // CZ|11⟩ = -|11⟩

    for i in 0..4 {
        let mut sv = Statevector::new(2);
        if i & 1 != 0 {
            sv.apply_x(0).unwrap();
        }
        if i & 2 != 0 {
            sv.apply_x(1).unwrap();
        }

        sv.apply_cz(0, 1).unwrap();

        let expected_phase = if i == 3 { c(-1.0, 0.0) } else { c(1.0, 0.0) };
        assert_complex_eq(sv.data[i], expected_phase, &format!("CZ|{:02b}⟩", i));
    }
}

#[test]
fn test_swap() {
    // SWAP|01⟩ = |10⟩
    let mut sv = Statevector::new(2);
    sv.apply_x(0).unwrap(); // |01⟩ (index 1)
    sv.apply_swap(0, 1).unwrap();
    assert_complex_eq(sv.data[2], c(1.0, 0.0), "SWAP|01⟩ should be |10⟩");

    // SWAP|10⟩ = |01⟩
    let mut sv2 = Statevector::new(2);
    sv2.apply_x(1).unwrap(); // |10⟩ (index 2)
    sv2.apply_swap(0, 1).unwrap();
    assert_complex_eq(sv2.data[1], c(1.0, 0.0), "SWAP|10⟩ should be |01⟩");

    // SWAP|11⟩ = |11⟩
    let mut sv3 = Statevector::new(2);
    sv3.apply_x(0).unwrap();
    sv3.apply_x(1).unwrap(); // |11⟩ (index 3)
    sv3.apply_swap(0, 1).unwrap();
    assert_complex_eq(sv3.data[3], c(1.0, 0.0), "SWAP|11⟩ should be |11⟩");

    // SWAP * SWAP = I
    let mut sv4 = Statevector::new(2);
    sv4.apply_x(0).unwrap();
    sv4.apply_swap(0, 1).unwrap();
    sv4.apply_swap(0, 1).unwrap();
    assert_complex_eq(sv4.data[1], c(1.0, 0.0), "SWAP^2 should be I");
}

#[test]
#[should_panic(expected = "InvalidParameterValue")]
fn test_swap_same_qubit() {
    // SWAP(q, q) should panic with InvalidParameterValue
    let mut sv = Statevector::new(2);
    sv.apply_x(0).unwrap();
    sv.apply_swap(0, 0).unwrap();
}

#[test]
fn test_cy() {
    // Y matrix: [[0, -i], [i, 0]]
    // Y|0⟩ = i|1⟩, Y|1⟩ = -i|0⟩

    // For CY(0,1): control=q0, target=q1
    // |11⟩ (q0=1, q1=1): control=1, so apply Y to target
    // Y|q1=1⟩ = -i|q1=0⟩, so result is q0=1, q1=0 = |01⟩
    let mut sv = Statevector::new(2);
    sv.apply_x(0).unwrap();
    sv.apply_x(1).unwrap(); // |11⟩
    sv.apply_cy(0, 1).unwrap();

    assert_complex_eq(sv.data[3], c(0.0, 0.0), "CY|11⟩[11] should be 0");
    assert_complex_eq(sv.data[1], c(0.0, -1.0), "CY(0,1)|11⟩ should be -i|01⟩");

    // |01⟩ (q0=1, q1=0): control=1, apply Y to target
    // Y|q1=0⟩ = i|q1=1⟩, so result is q0=1, q1=1 = |11⟩
    let mut sv3 = Statevector::new(2);
    sv3.apply_x(0).unwrap(); // |01⟩
    sv3.apply_cy(0, 1).unwrap();
    assert_complex_eq(sv3.data[3], c(0.0, 1.0), "CY(0,1)|01⟩ should be i|11⟩");

    // |10⟩: control=0, no change (but due to bug, might be affected)
    let mut sv2 = Statevector::new(2);
    sv2.apply_x(1).unwrap(); // |10⟩
    sv2.apply_cy(0, 1).unwrap();
    assert_complex_eq(sv2.data[2], c(1.0, 0.0), "CY|10⟩ should be unchanged");
}

#[test]
fn test_cy_control_gt_target() {
    // CY(1, 0): control=1 (high), target=0 (low)
    // When control > target, the implementation uses a different path
    // |10⟩: q1=1 (control), q0=0 (target) -> Y|0⟩ = i|1⟩
    // Result: q1=1, q0=1 -> |11⟩
    let mut sv = Statevector::new(2);
    sv.apply_x(1).unwrap(); // |10⟩
    sv.apply_cy(1, 0).unwrap();
    assert_complex_eq(sv.data[3], c(0.0, 1.0), "CY(1,0)|10⟩ should be i|11⟩");
}

#[test]
fn test_double_qubit_gate_cnot() {
    // Test apply_two_qubit_gate with CNOT matrix
    // CNOT matrix: [[1,0,0,0], [0,1,0,0], [0,0,0,1], [0,0,1,0]]
    let cnot_matrix = [
        [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)],
        [c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0)],
    ];

    // Test with control < target (q0=0, q1=1)
    // For CNOT with q0 as control, q1 as target
    // This should be equivalent to CX(0,1)
    let mut sv1 = Statevector::new(2);
    sv1.apply_x(0).unwrap(); // |01⟩ (control=1, target=0)
    sv1.apply_two_qubit_gate(0, 1, cnot_matrix).unwrap();
    // CNOT|01⟩ should be |01⟩ if q0 is control, q1 is target (target was 0, flips to 1?)
    // Wait: |01⟩ = q0=1, q1=0. If q0 is control, q1 is target, target flips to 1
    // Result: |11⟩
    assert_complex_eq(
        sv1.data[3],
        c(1.0, 0.0),
        "CNOT matrix (0,1)|01⟩ should be |11⟩",
    );

    // Test with control > target (q0=1, q1=0)
    // CNOT with q0=1 as control, q1=0 as target
    let mut sv2 = Statevector::new(2);
    sv2.apply_x(1).unwrap(); // |10⟩ (q0=0, q1=1)
    sv2.apply_two_qubit_gate(1, 0, cnot_matrix).unwrap();
    // |10⟩: q0=0, q1=1. If q0 is target, q1 is control, q1=1 so q0 flips to 1
    // Result: |11⟩
    assert_complex_eq(
        sv2.data[3],
        c(1.0, 0.0),
        "CNOT matrix (1,0)|10⟩ should be |11⟩",
    );
}

#[test]
fn test_three_qubit_cx() {
    // Test CX on non-adjacent qubits: CX(0, 2) on 3-qubit system
    // |001⟩: qubit0=1, flip qubit2 -> |101⟩ (index 5)
    let mut sv = Statevector::new(3);
    sv.apply_x(0).unwrap(); // |001⟩ (index 1)
    sv.apply_cx(0, 2).unwrap();
    assert_complex_eq(sv.data[5], c(1.0, 0.0), "CX(0,2)|001⟩ should be |101⟩");

    // Test CX(2, 0): |100⟩ -> |101⟩
    let mut sv2 = Statevector::new(3);
    sv2.apply_x(2).unwrap(); // |100⟩ (index 4)
    sv2.apply_cx(2, 0).unwrap();
    assert_complex_eq(sv2.data[5], c(1.0, 0.0), "CX(2,0)|100⟩ should be |101⟩");
}

#[test]
fn test_three_qubit_swap() {
    // SWAP(0, 2) on 3-qubit system
    // |001⟩ -> |100⟩ (swap qubit 0 and qubit 2)
    let mut sv = Statevector::new(3);
    sv.apply_x(0).unwrap(); // |001⟩ (index 1)
    sv.apply_swap(0, 2).unwrap();
    assert_complex_eq(sv.data[4], c(1.0, 0.0), "SWAP(0,2)|001⟩ should be |100⟩");
}

#[test]
fn test_ghz_state() {
    // Create GHZ state on 3 qubits: (|000⟩ + |111⟩)/√2
    let mut sv = Statevector::new(3);

    // Apply H to qubit 0
    let h_matrix = [
        [c(FRAC_1_SQRT_2, 0.0), c(FRAC_1_SQRT_2, 0.0)],
        [c(FRAC_1_SQRT_2, 0.0), c(-FRAC_1_SQRT_2, 0.0)],
    ];
    sv.apply_single_qubit_gate(0, h_matrix).unwrap();

    // Apply CX(0, 1) and CX(0, 2)
    sv.apply_cx(0, 1).unwrap();
    sv.apply_cx(0, 2).unwrap();

    // Result should be (|000⟩ + |111⟩)/√2
    assert_complex_eq(sv.data[0], c(FRAC_1_SQRT_2, 0.0), "GHZ |000⟩ amp");
    assert_complex_eq(sv.data[7], c(FRAC_1_SQRT_2, 0.0), "GHZ |111⟩ amp");

    for i in 1..7 {
        assert_complex_eq(
            sv.data[i],
            c(0.0, 0.0),
            &format!("GHZ |{:03b}⟩ should be 0", i),
        );
    }

    assert_normalized(&sv);
}

#[test]
#[should_panic(expected = "IndexOutOfBounds")]
fn test_single_qubit_out_of_bounds() {
    let mut sv = Statevector::new(2); // qubits 0, 1
    let h_matrix = [
        [c(FRAC_1_SQRT_2, 0.0), c(FRAC_1_SQRT_2, 0.0)],
        [c(FRAC_1_SQRT_2, 0.0), c(-FRAC_1_SQRT_2, 0.0)],
    ];
    sv.apply_single_qubit_gate(2, h_matrix).unwrap();
}

#[test]
#[should_panic(expected = "InvalidParameterValue")]
fn test_cx_same_qubit() {
    let mut sv = Statevector::new(2);
    sv.apply_cx(0, 0).unwrap();
}

#[test]
#[should_panic(expected = "InvalidParameterValue")]
fn test_cy_same_qubit() {
    let mut sv = Statevector::new(2);
    sv.apply_cy(0, 0).unwrap();
}

#[test]
#[should_panic(expected = "InvalidParameterValue")]
fn test_cz_same_qubit() {
    let mut sv = Statevector::new(2);
    sv.apply_cz(0, 0).unwrap();
}

#[test]
fn test_hadamard_unitary() {
    // H is unitary: H * H† = I
    // Since H is Hermitian (H = H†), H^2 = I
    let h_matrix = [
        [c(FRAC_1_SQRT_2, 0.0), c(FRAC_1_SQRT_2, 0.0)],
        [c(FRAC_1_SQRT_2, 0.0), c(-FRAC_1_SQRT_2, 0.0)],
    ];

    // Test on a 2-qubit system with random initial state
    let mut sv = Statevector::new(2);
    sv.apply_ry(0, 0.7).unwrap();
    sv.apply_rx(1, 1.2).unwrap();

    let original: Vec<Complex64> = sv.data.to_vec();

    sv.apply_single_qubit_gate(0, h_matrix).unwrap();
    sv.apply_single_qubit_gate(0, h_matrix).unwrap();

    for (i, (a, b)) in sv.data.iter().zip(original.iter()).enumerate() {
        assert_complex_eq(*a, *b, &format!("H^2 should be I at index {}", i));
    }
}

#[test]
fn test_cz_unitary() {
    // CZ^2 = I
    let mut sv = Statevector::new(2);
    sv.apply_x(0).unwrap();
    sv.apply_x(1).unwrap(); // |11⟩

    sv.apply_cz(0, 1).unwrap();
    sv.apply_cz(0, 1).unwrap();

    assert_complex_eq(sv.data[3], c(1.0, 0.0), "CZ^2 should be I");
}

#[test]
fn test_rx_ry_rz_unitarity() {
    // Rotation gates should preserve normalization
    let mut sv = Statevector::new(3);

    // Apply random rotations
    sv.apply_rx(0, 1.23).unwrap();
    sv.apply_ry(1, 2.34).unwrap();
    sv.apply_rz(2, 3.45).unwrap();
    sv.apply_cx(0, 1).unwrap();
    sv.apply_cz(1, 2).unwrap();
    sv.apply_swap(0, 2).unwrap();

    assert_normalized(&sv);
}

#[test]
fn test_sdg_gate() {
    // S† = S^3, S† * S = I
    // S|1⟩ = i|1⟩, S†|1⟩ = -i|1⟩
    let mut sv = Statevector::new(1);
    sv.apply_x(0).unwrap();
    sv.apply_s(0).unwrap();
    sv.apply_sdg(0).unwrap();
    assert_complex_eq(sv.data[1], c(1.0, 0.0), "S†S|1⟩ should be |1⟩");
}

#[test]
fn test_tdg_gate() {
    // T† * T = I
    let mut sv = Statevector::new(1);
    sv.apply_x(0).unwrap();
    sv.apply_t(0).unwrap();
    sv.apply_tdg(0).unwrap();
    assert_complex_eq(sv.data[1], c(1.0, 0.0), "T†T|1⟩ should be |1⟩");
}

#[test]
fn test_xy2p_xy2m() {
    // XY2P * XY2M should be approximately I
    let mut sv = Statevector::new(1);
    sv.apply_ry(0, 0.5).unwrap(); // Some arbitrary rotation
    sv.apply_xy2p(0, 1.0).unwrap();
    sv.apply_xy2m(0, 1.0).unwrap();

    // Since XY2P and XY2M are inverses, result should be close to original
    // Actually XY2M(θ) should be the inverse of XY2P(θ)
    // Let's verify with a simpler test
    let mut sv2 = Statevector::new(1);
    sv2.apply_x(0).unwrap();

    let original = sv2.data.clone();

    sv2.apply_xy2p(0, 0.5).unwrap();
    sv2.apply_xy2m(0, 0.5).unwrap();

    assert_complex_eq(sv2.data[1], original[1], "XY2M should invert XY2P");
}

#[test]
fn test_rxy_gate() {
    // RXY(θ, phi) rotates around axis cos(phi)X + sin(phi)Y
    // Matrix: [[cos(t/2), -i*e^(-i*phi)*sin(t/2)], [-i*e^(i*phi)*sin(t/2), cos(t/2)]]
    //
    // When phi = 0: RXY(θ, 0) = Rx(θ)
    //   e^(-i*0) = e^(i*0) = 1
    //   Matrix = [[cos(t/2), -i*sin(t/2)], [-i*sin(t/2), cos(t/2)]] = Rx(θ)
    //
    // When phi = π/2: RXY(θ, π/2) = Ry(θ)
    //   e^(-i*π/2) = -i, e^(i*π/2) = i
    //   off_diag = -i*(-i)*sin(t/2) = -sin(t/2) and -i*(i)*sin(t/2) = sin(t/2)
    //   Matrix = [[cos(t/2), -sin(t/2)], [sin(t/2), cos(t/2)]] = Ry(θ)

    // Test RXY(θ, 0) = Rx(θ)
    let mut sv1 = Statevector::new(1);
    sv1.apply_rxy(0, PI / 2.0, 0.0).unwrap();

    let mut sv2 = Statevector::new(1);
    sv2.apply_rx(0, PI / 2.0).unwrap();

    assert_complex_eq(
        sv1.data[0],
        sv2.data[0],
        "RXY(θ,0)[0] should equal Rx(θ)[0]",
    );
    assert_complex_eq(
        sv1.data[1],
        sv2.data[1],
        "RXY(θ,0)[1] should equal Rx(θ)[1]",
    );

    // Test RXY(θ, π/2) = Ry(θ)
    let mut sv3 = Statevector::new(1);
    sv3.apply_rxy(0, PI / 2.0, PI / 2.0).unwrap();

    let mut sv4 = Statevector::new(1);
    sv4.apply_ry(0, PI / 2.0).unwrap();

    assert_complex_eq(
        sv3.data[0],
        sv4.data[0],
        "RXY(θ,π/2)[0] should equal Ry(θ)[0]",
    );
    assert_complex_eq(
        sv3.data[1],
        sv4.data[1],
        "RXY(θ,π/2)[1] should equal Ry(θ)[1]",
    );

    // Verify normalization for arbitrary phi
    let mut sv5 = Statevector::new(1);
    sv5.apply_rxy(0, PI / 2.0, PI / 4.0).unwrap();
    assert_normalized(&sv5);
}

#[test]
fn test_cascade_cx() {
    // Test cascading CX gates
    // |011⟩: apply CX(0,1), then CX(1,2)
    // |011⟩: q0=1, q1=1, q2=0
    // CX(0,1): q0=1, flip q1: q1 goes 1->0, result |001⟩
    // CX(1,2): q1=0, no flip, result |001⟩
    let mut sv = Statevector::new(3);
    sv.apply_x(0).unwrap();
    sv.apply_x(1).unwrap(); // |011⟩ (index 3)
    sv.apply_cx(0, 1).unwrap();
    sv.apply_cx(1, 2).unwrap();
    assert_complex_eq(sv.data[1], c(1.0, 0.0), "Cascade CX result should be |001⟩");
}

#[test]
fn test_single_qubit_system() {
    // Test all gates on a single qubit
    let mut sv = Statevector::new(1);

    // Apply H then X
    let h_matrix = [
        [c(FRAC_1_SQRT_2, 0.0), c(FRAC_1_SQRT_2, 0.0)],
        [c(FRAC_1_SQRT_2, 0.0), c(-FRAC_1_SQRT_2, 0.0)],
    ];
    sv.apply_single_qubit_gate(0, h_matrix).unwrap();
    sv.apply_x(0).unwrap();

    // H|0⟩ = (|0⟩+|1⟩)/√2, X((|0⟩+|1⟩)/√2) = (|1⟩+|0⟩)/√2 = same
    assert_complex_eq(sv.data[0], c(FRAC_1_SQRT_2, 0.0), "XH|0⟩[0] should be 1/√2");
    assert_complex_eq(sv.data[1], c(FRAC_1_SQRT_2, 0.0), "XH|0⟩[1] should be 1/√2");
}

#[test]
fn test_max_qubits_edge_case() {
    // Test with 4 qubits (16 states)
    let mut sv = Statevector::new(4);

    // Create superposition on qubit 3 (highest)
    sv.apply_ry(3, PI / 2.0).unwrap();

    // Verify normalization
    assert_normalized(&sv);

    // Apply CX from qubit 3 to qubit 0 (non-adjacent)
    sv.apply_cx(3, 0).unwrap();

    assert_normalized(&sv);
}

#[test]
fn test_zero_angle_rotations() {
    // All rotation gates with angle 0 should be identity
    let mut sv = Statevector::new(2);
    sv.apply_x(0).unwrap(); // |01⟩

    sv.apply_rx(0, 0.0).unwrap();
    sv.apply_ry(0, 0.0).unwrap();
    sv.apply_rz(0, 0.0).unwrap();
    sv.apply_phase(0, 0.0).unwrap();
    sv.apply_rxy(0, 0.0, 1.0).unwrap();

    assert_complex_eq(sv.data[1], c(1.0, 0.0), "Zero angle rotations should be I");
}

#[test]
fn test_global_phase() {
    // Test that global phases work correctly
    // Rz(2π)|0⟩ = -|0⟩ (global phase -1)
    let mut sv = Statevector::new(1);
    sv.apply_rz(0, 2.0 * PI).unwrap();
    assert_complex_eq(sv.data[0], c(-1.0, 0.0), "Rz(2π)|0⟩ should be -|0⟩");

    // Verify probability is still 1
    let probs = sv.probabilities();
    assert!(
        (probs[0] - 1.0).abs() < EPSILON,
        "Probability should be 1 despite global phase"
    );
}

#[test]
fn test_relative_phase() {
    // S gate introduces relative phase
    // S(H|0⟩) = S((|0⟩+|1⟩)/√2) = (|0⟩+i|1⟩)/√2
    let mut sv = Statevector::new(1);
    let h_matrix = [
        [c(FRAC_1_SQRT_2, 0.0), c(FRAC_1_SQRT_2, 0.0)],
        [c(FRAC_1_SQRT_2, 0.0), c(-FRAC_1_SQRT_2, 0.0)],
    ];
    sv.apply_single_qubit_gate(0, h_matrix).unwrap();
    sv.apply_s(0).unwrap();

    assert_complex_eq(sv.data[0], c(FRAC_1_SQRT_2, 0.0), "SH|0⟩[0] should be 1/√2");
    assert_complex_eq(sv.data[1], c(0.0, FRAC_1_SQRT_2), "SH|0⟩[1] should be i/√2");

    // Probability should be 0.5 each
    let probs = sv.probabilities();
    assert!((probs[0] - 0.5).abs() < EPSILON, "P(|0⟩) should be 0.5");
    assert!((probs[1] - 0.5).abs() < EPSILON, "P(|1⟩) should be 0.5");
}

#[test]
fn test_apply_h() {
    // H|0⟩ = (|0⟩ + |1⟩)/√2
    let mut sv = Statevector::new(1);
    sv.apply_h(0).unwrap();

    assert_complex_eq(sv.data[0], c(FRAC_1_SQRT_2, 0.0), "H|0⟩[0] should be 1/√2");
    assert_complex_eq(sv.data[1], c(FRAC_1_SQRT_2, 0.0), "H|0⟩[1] should be 1/√2");

    // H|1⟩ = (|0⟩ - |1⟩)/√2
    let mut sv2 = Statevector::new(1);
    sv2.apply_x(0).unwrap();
    sv2.apply_h(0).unwrap();

    assert_complex_eq(sv2.data[0], c(FRAC_1_SQRT_2, 0.0), "H|1⟩[0] should be 1/√2");
    assert_complex_eq(
        sv2.data[1],
        c(-FRAC_1_SQRT_2, 0.0),
        "H|1⟩[1] should be -1/√2",
    );

    // H^2 = I
    let mut sv3 = Statevector::new(1);
    sv3.apply_h(0).unwrap();
    sv3.apply_h(0).unwrap();
    assert_complex_eq(sv3.data[0], c(1.0, 0.0), "H^2|0⟩ should be |0⟩");
    assert_complex_eq(sv3.data[1], c(0.0, 0.0), "H^2|0⟩[1] should be 0");
}

#[test]
fn test_apply_u() {
    // U(θ, φ, λ) = [[cos(θ/2), -e^(iλ)sin(θ/2)], [e^(iφ)sin(θ/2), e^(i(φ+λ))cos(θ/2)]]
    //
    // Test: U(θ, 0, 0) = [[cos(θ/2), -sin(θ/2)], [sin(θ/2), cos(θ/2)]] = Ry(θ)
    let mut sv1 = Statevector::new(1);
    sv1.apply_u(0, PI, 0.0, 0.0).unwrap();

    let mut sv2 = Statevector::new(1);
    sv2.apply_ry(0, PI).unwrap();

    assert_complex_eq(sv1.data[0], sv2.data[0], "U(π,0,0) should equal Ry(π)[0]");
    assert_complex_eq(sv1.data[1], sv2.data[1], "U(π,0,0) should equal Ry(π)[1]");

    // Test: U(0, 0, λ) should be equivalent to P(λ) (phase gate) up to global phase
    // U(0, 0, λ) = [[1, 0], [0, e^(iλ)]]
    let mut sv3 = Statevector::new(1);
    sv3.apply_x(0).unwrap();
    sv3.apply_u(0, 0.0, 0.0, PI / 2.0).unwrap();

    let mut sv4 = Statevector::new(1);
    sv4.apply_x(0).unwrap();
    sv4.apply_phase(0, PI / 2.0).unwrap();

    assert_complex_eq(sv3.data[1], sv4.data[1], "U(0,0,λ)|1⟩ should equal P(λ)|1⟩");

    // Test: U(π/2, 0, 0) = Ry(π/2)
    let mut sv5 = Statevector::new(1);
    sv5.apply_u(0, PI / 2.0, 0.0, 0.0).unwrap();

    let mut sv6 = Statevector::new(1);
    sv6.apply_ry(0, PI / 2.0).unwrap();

    assert_complex_eq(
        sv5.data[0],
        sv6.data[0],
        "U(π/2,0,0) should equal Ry(π/2)[0]",
    );
    assert_complex_eq(
        sv5.data[1],
        sv6.data[1],
        "U(π/2,0,0) should equal Ry(π/2)[1]",
    );

    // Test: U(π, -π/2, π/2) = Rx(π) = -iX
    // cos(π/2)=0, sin(π/2)=1
    // [[0, -e^(iπ/2)], [e^(-iπ/2), 0]] = [[0, -i], [-i, 0]] = -iX
    let mut sv7 = Statevector::new(1);
    sv7.apply_u(0, PI, -PI / 2.0, PI / 2.0).unwrap();

    let mut sv8 = Statevector::new(1);
    sv8.apply_rx(0, PI).unwrap();

    assert_complex_eq(
        sv7.data[0],
        sv8.data[0],
        "U(π,-π/2,π/2) should equal Rx(π)[0]",
    );
    assert_complex_eq(
        sv7.data[1],
        sv8.data[1],
        "U(π,-π/2,π/2) should equal Rx(π)[1]",
    );

    // Verify unitarity for random parameters
    let mut sv9 = Statevector::new(2);
    sv9.apply_ry(0, 0.7).unwrap();
    sv9.apply_u(1, 1.2, 0.5, 0.3).unwrap();
    assert_normalized(&sv9);
}

#[test]
fn test_apply_ccx() {
    // Toffoli gate: flips target when both controls are 1

    // |110⟩: controls=1,1, target=0 → flips to |111⟩
    let mut sv = Statevector::new(3);
    sv.apply_x(1).unwrap();
    sv.apply_x(2).unwrap(); // |110⟩ (index 6)
    sv.apply_ccx(1, 2, 0).unwrap();
    assert_complex_eq(sv.data[7], c(1.0, 0.0), "CCX|110⟩ should be |111⟩");

    // |111⟩: controls=1,1, target=1 → flips to |110⟩
    let mut sv2 = Statevector::new(3);
    sv2.apply_x(0).unwrap();
    sv2.apply_x(1).unwrap();
    sv2.apply_x(2).unwrap(); // |111⟩ (index 7)
    sv2.apply_ccx(0, 1, 2).unwrap();
    assert_complex_eq(
        sv2.data[3],
        c(1.0, 0.0),
        "CCX|111⟩ should be |011⟩ when target is highest",
    );

    // |101⟩: one control is 0, no flip
    let mut sv3 = Statevector::new(3);
    sv3.apply_x(0).unwrap();
    sv3.apply_x(2).unwrap(); // |101⟩ (index 5)
    sv3.apply_ccx(0, 1, 2).unwrap();
    assert_complex_eq(sv3.data[5], c(1.0, 0.0), "CCX|101⟩ should be unchanged");

    // |011⟩: one control is 0, no flip
    let mut sv4 = Statevector::new(3);
    sv4.apply_x(0).unwrap();
    sv4.apply_x(1).unwrap(); // |011⟩ (index 3)
    sv4.apply_ccx(1, 2, 0).unwrap();
    assert_complex_eq(sv4.data[3], c(1.0, 0.0), "CCX|011⟩ should be unchanged");

    // CCX is self-inverse: CCX^2 = I
    let mut sv5 = Statevector::new(3);
    sv5.apply_x(0).unwrap();
    sv5.apply_x(1).unwrap(); // |011⟩ (controls=1,1, target=0)
    sv5.apply_ccx(0, 1, 2).unwrap();
    sv5.apply_ccx(0, 1, 2).unwrap();
    assert_complex_eq(sv5.data[3], c(1.0, 0.0), "CCX^2 should be I");
}

#[test]
fn test_apply_rxx() {
    // RXX(π)|00⟩ should give something non-trivial
    // RXX(0) = I
    let mut sv1 = Statevector::new(2);
    sv1.apply_rxx(0, 1, 0.0).unwrap();
    assert_complex_eq(sv1.data[0], c(1.0, 0.0), "RXX(0) should be I");

    // RXX(π)|00⟩ = -i|11⟩ approximately (when θ=π, cos=0, sin=1)
    let mut sv2 = Statevector::new(2);
    sv2.apply_rxx(0, 1, PI).unwrap();
    assert_complex_eq(sv2.data[0], c(0.0, 0.0), "RXX(π)|00⟩[0] should be 0");
    assert_complex_eq(sv2.data[3], c(0.0, -1.0), "RXX(π)|00⟩ should be -i|11⟩");

    // RXX^2 = I for θ=π (up to global phase)
    let mut sv3 = Statevector::new(2);
    sv3.apply_x(0).unwrap(); // |01⟩
    sv3.apply_rxx(0, 1, PI).unwrap();
    sv3.apply_rxx(0, 1, PI).unwrap();
    assert_complex_eq(sv3.data[1], c(-1.0, 0.0), "RXX(π)^2 should be -I");

    // Verify normalization
    let mut sv4 = Statevector::new(3);
    sv4.apply_h(0).unwrap();
    sv4.apply_rxx(0, 2, 0.7).unwrap();
    assert_normalized(&sv4);
}

#[test]
fn test_apply_ryy() {
    // RYY(0) = I
    let mut sv1 = Statevector::new(2);
    sv1.apply_ryy(0, 1, 0.0).unwrap();
    assert_complex_eq(sv1.data[0], c(1.0, 0.0), "RYY(0) should be I");

    // RYY(π)|00⟩ = i|11⟩
    let mut sv2 = Statevector::new(2);
    sv2.apply_ryy(0, 1, PI).unwrap();
    assert_complex_eq(sv2.data[0], c(0.0, 0.0), "RYY(π)|00⟩[0] should be 0");
    assert_complex_eq(sv2.data[3], c(0.0, 1.0), "RYY(π)|00⟩ should be i|11⟩");

    // Verify normalization
    let mut sv3 = Statevector::new(3);
    sv3.apply_h(0).unwrap();
    sv3.apply_ryy(1, 2, 0.7).unwrap();
    assert_normalized(&sv3);
}

#[test]
fn test_apply_rzz() {
    // RZZ(0) = I
    let mut sv1 = Statevector::new(2);
    sv1.apply_rzz(0, 1, 0.0).unwrap();
    assert_complex_eq(sv1.data[0], c(1.0, 0.0), "RZZ(0) should be I");

    // RZZ adds phases based on parity
    // RZZ(π)|00⟩ = e^(-iπ/2)|00⟩ = -i|00⟩
    let mut sv2 = Statevector::new(2);
    sv2.apply_rzz(0, 1, PI).unwrap();
    assert_complex_eq(sv2.data[0], c(0.0, -1.0), "RZZ(π)|00⟩ should be -i|00⟩");

    // RZZ(π)|01⟩ = e^(iπ/2)|01⟩ = i|01⟩
    let mut sv3 = Statevector::new(2);
    sv3.apply_x(0).unwrap();
    sv3.apply_rzz(0, 1, PI).unwrap();
    assert_complex_eq(sv3.data[1], c(0.0, 1.0), "RZZ(π)|01⟩ should be i|01⟩");

    // RZZ is diagonal, so RZZ^2 ≠ I but adds phases
    // Verify normalization
    let mut sv4 = Statevector::new(3);
    sv4.apply_h(0).unwrap();
    sv4.apply_rzz(0, 2, 0.7).unwrap();
    assert_normalized(&sv4);
}

#[test]
fn test_apply_rzx() {
    // RZX(0) = I
    let mut sv1 = Statevector::new(2);
    sv1.apply_rzx(0, 1, 0.0).unwrap();
    assert_complex_eq(sv1.data[0], c(1.0, 0.0), "RZX(0) should be I");

    // RZX(π)|00⟩ with q0<q1 (Z on q0, X on q1)
    // |00⟩ -> 0 (cos(π/2)=0) + something
    let mut sv2 = Statevector::new(2);
    sv2.apply_rzx(0, 1, PI).unwrap();
    // When theta=π: cos=0, sin=1
    // |00⟩ -> -i|10⟩
    assert_complex_eq(sv2.data[0], c(0.0, 0.0), "RZX(π)|00⟩[0]");
    assert_complex_eq(sv2.data[2], c(0.0, -1.0), "RZX(π)|00⟩ should be -i|10⟩");

    // Verify normalization
    let mut sv3 = Statevector::new(3);
    sv3.apply_h(0).unwrap();
    sv3.apply_rzx(0, 2, 0.7).unwrap();
    assert_normalized(&sv3);

    // Test q0 > q1 case (Z on q1, X on q0)
    let mut sv4 = Statevector::new(2);
    sv4.apply_rzx(1, 0, PI).unwrap();
    // X on q0, Z on q1
    // |00⟩ -> -i|01⟩
    assert_complex_eq(sv4.data[0], c(0.0, 0.0), "RZX(π, rev)|00⟩[0]");
    assert_complex_eq(
        sv4.data[1],
        c(0.0, -1.0),
        "RZX(π, rev)|00⟩ should be -i|01⟩",
    );
}

#[test]
fn test_apply_crx() {
    // CRX(0) = I
    let mut sv1 = Statevector::new(2);
    sv1.apply_crx(0, 1, 0.0).unwrap();
    assert_complex_eq(sv1.data[0], c(1.0, 0.0), "CRX(0) should be I");

    // CRX(π)|11⟩ with control=0, target=1:
    // Control=1, so apply RX(π) to target
    // RX(π)|1⟩ = -i|0⟩, so |11⟩ -> -i|01⟩
    let mut sv2 = Statevector::new(2);
    sv2.apply_x(0).unwrap();
    sv2.apply_x(1).unwrap(); // |11⟩
    sv2.apply_crx(0, 1, PI).unwrap();
    assert_complex_eq(sv2.data[3], c(0.0, 0.0), "CRX(π)|11⟩[11] should be 0");
    assert_complex_eq(sv2.data[1], c(0.0, -1.0), "CRX(π)|11⟩ should be -i|01⟩");

    // CRX(π)|10⟩ with control=0, target=1:
    // Control=0, so no change
    let mut sv2b = Statevector::new(2);
    sv2b.apply_x(1).unwrap(); // |10⟩
    sv2b.apply_crx(0, 1, PI).unwrap();
    assert_complex_eq(
        sv2b.data[2],
        c(1.0, 0.0),
        "CRX(π)|10⟩ should be unchanged (control=0)",
    );

    // |01⟩: control=1, so apply RX(π)|0⟩ = -i|1⟩
    // |01⟩ -> -i|11⟩
    let mut sv3 = Statevector::new(2);
    sv3.apply_x(0).unwrap(); // |01⟩
    sv3.apply_crx(0, 1, PI).unwrap();
    assert_complex_eq(sv3.data[1], c(0.0, 0.0), "CRX(π)|01⟩[01] should be 0");
    assert_complex_eq(sv3.data[3], c(0.0, -1.0), "CRX(π)|01⟩ should be -i|11⟩");

    // Verify normalization
    let mut sv4 = Statevector::new(3);
    sv4.apply_h(0).unwrap();
    sv4.apply_crx(0, 2, 0.7).unwrap();
    assert_normalized(&sv4);

    // Test control < target with |10⟩ (control=0, should be unchanged)
    // This tests the specific bug where control < target logic was inverted
    let mut sv5 = Statevector::new(2);
    sv5.apply_x(1).unwrap(); // |10⟩: control=0, target=1
    sv5.apply_crx(0, 1, PI).unwrap();
    assert_complex_eq(
        sv5.data[2],
        c(1.0, 0.0),
        "CRX(0,1,π)|10⟩ should be unchanged (control=0)",
    );

    // Test control > target with |10⟩ (control=1, target should flip)
    let mut sv6 = Statevector::new(2);
    sv6.apply_x(1).unwrap(); // |10⟩: q0=0, q1=1
    sv6.apply_crx(1, 0, PI).unwrap(); // control=1, target=0
    // control=1, so apply RX(π) to target |0⟩ -> -i|1⟩
    // |10⟩ -> -i|11⟩
    assert_complex_eq(sv6.data[2], c(0.0, 0.0), "CRX(1,0,π)|10⟩[10] should be 0");
    assert!((sv6.data[3].re.abs() < EPSILON), "Real part should be 0");
    assert!(
        (sv6.data[3].im + 1.0).abs() < EPSILON,
        "Imag part should be -1"
    );
}

#[test]
fn test_apply_cry() {
    // CRY(0) = I
    let mut sv1 = Statevector::new(2);
    sv1.apply_cry(0, 1, 0.0).unwrap();
    assert_complex_eq(sv1.data[0], c(1.0, 0.0), "CRY(0) should be I");

    // CRY(π)|11⟩ with control=0, target=1:
    // Control=1, so apply RY(π) to target
    // RY(π)|1⟩ = -|0⟩, so |11⟩ -> -|01⟩
    let mut sv2 = Statevector::new(2);
    sv2.apply_x(0).unwrap();
    sv2.apply_x(1).unwrap(); // |11⟩
    sv2.apply_cry(0, 1, PI).unwrap();
    assert_complex_eq(sv2.data[3], c(0.0, 0.0), "CRY(π)|11⟩[11] should be 0");
    assert_complex_eq(sv2.data[1], c(-1.0, 0.0), "CRY(π)|11⟩ should be -|01⟩");

    // CRY(π)|10⟩ with control=0, target=1:
    // Control=0, so no change
    let mut sv2b = Statevector::new(2);
    sv2b.apply_x(1).unwrap(); // |10⟩
    sv2b.apply_cry(0, 1, PI).unwrap();
    assert_complex_eq(sv2b.data[2], c(1.0, 0.0), "CRY(π)|10⟩ should be unchanged");

    // |01⟩: control=1, target flips |0⟩ -> |1⟩ with phase
    // RY(π)|0⟩ = |1⟩, so |01⟩ -> |11⟩
    let mut sv3 = Statevector::new(2);
    sv3.apply_x(0).unwrap(); // |01⟩
    sv3.apply_cry(0, 1, PI).unwrap();
    assert_complex_eq(sv3.data[1], c(0.0, 0.0), "CRY(π)|01⟩[01] should be 0");
    assert_complex_eq(sv3.data[3], c(1.0, 0.0), "CRY(π)|01⟩ should be |11⟩");

    // Verify normalization
    let mut sv4 = Statevector::new(3);
    sv4.apply_h(0).unwrap();
    sv4.apply_cry(0, 2, 0.7).unwrap();
    assert_normalized(&sv4);

    // Test control > target: CRY(1, 0, π)|11⟩
    // control=1, target=0
    // |11⟩: q1=1 (control), q0=1 (target)
    // Apply RY(π) to target |1⟩ = -|0⟩
    // Result: -|10⟩ (q1=1, q0=0 with phase)
    let mut sv5 = Statevector::new(2);
    sv5.apply_x(0).unwrap();
    sv5.apply_x(1).unwrap(); // |11⟩
    sv5.apply_cry(1, 0, PI).unwrap();
    assert_complex_eq(sv5.data[3], c(0.0, 0.0), "CRY(1,0,π)|11⟩[11] should be 0");
    assert_complex_eq(sv5.data[2], c(-1.0, 0.0), "CRY(1,0,π)|11⟩ should be -|10⟩");
}

#[test]
fn test_apply_crz() {
    // CRZ(0) = I
    let mut sv1 = Statevector::new(2);
    sv1.apply_crz(0, 1, 0.0).unwrap();
    assert_complex_eq(sv1.data[0], c(1.0, 0.0), "CRZ(0) should be I");

    // CRZ(π)|11⟩ with control=0, target=1:
    // Control=1, so apply RZ(π) to target
    // RZ(π)|1⟩ = i|1⟩
    // |11⟩ -> i|11⟩
    let mut sv2 = Statevector::new(2);
    sv2.apply_x(0).unwrap();
    sv2.apply_x(1).unwrap(); // |11⟩
    sv2.apply_crz(0, 1, PI).unwrap();
    assert_complex_eq(sv2.data[3], c(0.0, 1.0), "CRZ(π)|11⟩ should be i|11⟩");

    // CRZ(π)|10⟩ with control=0, target=1:
    // Control=0, so no change
    // |10⟩ should remain |10⟩
    let mut sv2b = Statevector::new(2);
    sv2b.apply_x(1).unwrap(); // |10⟩
    sv2b.apply_crz(0, 1, PI).unwrap();
    assert_complex_eq(
        sv2b.data[2],
        c(1.0, 0.0),
        "CRZ(π)|10⟩ should be unchanged (control=0)",
    );

    // |01⟩: control=1, target=0, apply RZ(π)|0⟩ = -i|0⟩
    // |01⟩ -> -i|01⟩
    let mut sv3 = Statevector::new(2);
    sv3.apply_x(0).unwrap(); // |01⟩
    sv3.apply_crz(0, 1, PI).unwrap();
    assert_complex_eq(sv3.data[1], c(0.0, -1.0), "CRZ(π)|01⟩ should be -i|01⟩");

    // |00⟩: control=0, no change
    let mut sv4 = Statevector::new(2);
    sv4.apply_crz(0, 1, PI).unwrap();
    assert_complex_eq(sv4.data[0], c(1.0, 0.0), "CRZ(π)|00⟩ should be unchanged");

    // Verify normalization
    let mut sv5 = Statevector::new(3);
    sv5.apply_h(0).unwrap();
    sv5.apply_crz(0, 2, 0.7).unwrap();
    assert_normalized(&sv5);

    // Test control > target: CRZ(1, 0, π)|10⟩
    // control=1, target=0
    // |10⟩: q1=1 (control), q0=0 (target)
    // Apply RZ(π)|0⟩ = -i|0⟩
    // Result: -i|10⟩
    let mut sv6 = Statevector::new(2);
    sv6.apply_x(1).unwrap(); // |10⟩
    sv6.apply_crz(1, 0, PI).unwrap();
    assert_complex_eq(sv6.data[2], c(0.0, -1.0), "CRZ(1,0,π)|10⟩ should be -i|10⟩");
}

#[test]
fn test_apply_fsim() {
    // fSim(0, 0) = I
    let mut sv1 = Statevector::new(2);
    sv1.apply_fsim(0, 1, 0.0, 0.0).unwrap();
    assert_complex_eq(sv1.data[0], c(1.0, 0.0), "fSim(0,0) should be I");

    // fSim(π/2, 0) = iSWAP (approximately)
    // |01⟩ -> -i|10⟩ when θ=π/2
    let mut sv2 = Statevector::new(2);
    sv2.apply_x(0).unwrap(); // |01⟩
    sv2.apply_fsim(0, 1, PI / 2.0, 0.0).unwrap();
    assert_complex_eq(sv2.data[1], c(0.0, 0.0), "fSim(π/2,0)|01⟩[01] should be 0");
    assert_complex_eq(
        sv2.data[2],
        c(0.0, -1.0),
        "fSim(π/2,0)|01⟩ should be -i|10⟩",
    );

    // |11⟩ gets phase e^(-iφ)
    let mut sv3 = Statevector::new(2);
    sv3.apply_x(0).unwrap();
    sv3.apply_x(1).unwrap(); // |11⟩
    sv3.apply_fsim(0, 1, 0.0, PI / 2.0).unwrap();
    assert_complex_eq(
        sv3.data[3],
        c(0.0, -1.0),
        "fSim(0,π/2)|11⟩ should be -i|11⟩",
    );

    // Verify normalization
    let mut sv4 = Statevector::new(3);
    sv4.apply_h(0).unwrap();
    sv4.apply_fsim(0, 2, 0.5, 0.3).unwrap();
    assert_normalized(&sv4);
}

#[test]
fn test_apply_gphase() {
    // Global phase doesn't affect probabilities
    let mut sv1 = Statevector::new(2);
    sv1.apply_h(0).unwrap();
    sv1.apply_gphase(PI).unwrap();

    let probs1 = sv1.probabilities();
    assert!((probs1[0] - 0.5).abs() < EPSILON, "P(|00⟩) should be 0.5");
    assert!((probs1[1] - 0.5).abs() < EPSILON, "P(|01⟩) should be 0.5");

    // GPhase(π)|0⟩ = -|0⟩
    let mut sv2 = Statevector::new(1);
    sv2.apply_gphase(PI).unwrap();
    assert_complex_eq(sv2.data[0], c(-1.0, 0.0), "GPhase(π)|0⟩ should be -|0⟩");

    // GPhase(2π) = I
    let mut sv3 = Statevector::new(1);
    sv3.apply_x(0).unwrap();
    sv3.apply_gphase(2.0 * PI).unwrap();
    assert_complex_eq(sv3.data[1], c(1.0, 0.0), "GPhase(2π) should be I");
}

#[test]
fn test_from_state() {
    // Test creating Statevector from a valid initial state
    // |+⟩ state: (|0⟩ + |1⟩)/√2
    let initial_state = vec![c(FRAC_1_SQRT_2, 0.0), c(FRAC_1_SQRT_2, 0.0)];
    let sv = Statevector::from_state(1, initial_state).unwrap();
    assert_eq!(sv.num_qubits, 1);
    assert_eq!(sv.data.len(), 2);
    assert_complex_eq(sv.data[0], c(FRAC_1_SQRT_2, 0.0), "|+⟩[0] should be 1/√2");
    assert_complex_eq(sv.data[1], c(FRAC_1_SQRT_2, 0.0), "|+⟩[1] should be 1/√2");

    // Test Bell state: (|00⟩ + |11⟩)/√2
    let bell_state = vec![
        c(FRAC_1_SQRT_2, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(FRAC_1_SQRT_2, 0.0),
    ];
    let sv2 = Statevector::from_state(2, bell_state).unwrap();
    assert_eq!(sv2.num_qubits, 2);
    assert_eq!(sv2.data.len(), 4);
    assert_complex_eq(sv2.data[0], c(FRAC_1_SQRT_2, 0.0), "Bell |00⟩ amp");
    assert_complex_eq(sv2.data[3], c(FRAC_1_SQRT_2, 0.0), "Bell |11⟩ amp");

    // Test |i⟩ state: (|0⟩ + i|1⟩)/√2
    let i_state = vec![c(FRAC_1_SQRT_2, 0.0), c(0.0, FRAC_1_SQRT_2)];
    let sv3 = Statevector::from_state(1, i_state).unwrap();
    assert_complex_eq(sv3.data[1], c(0.0, FRAC_1_SQRT_2), "|i⟩[1] should be i/√2");
}

#[test]
fn test_from_state_wrong_length() {
    // Should return error if state length doesn't match 2^num_qubits
    let state = vec![c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)]; // 3 elements for 2 qubits (should be 4)
    let result = Statevector::from_state(2, state);
    assert!(result.is_err());
}

#[test]
fn test_from_state_not_normalized() {
    // Should return error if state is not normalized
    let state = vec![c(1.0, 0.0), c(1.0, 0.0)]; // norm = sqrt(2) != 1
    let result = Statevector::from_state(1, state);
    assert!(result.is_err());
}

#[test]
fn test_from_circuit() {
    use crate::circuit::Circuit;

    // Test from_circuit with a simple H gate
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();

    let sv = Statevector::from_circuit(&circuit).unwrap();
    assert_eq!(sv.num_qubits, 1);
    assert_complex_eq(sv.data[0], c(FRAC_1_SQRT_2, 0.0), "H|0⟩[0] should be 1/√2");
    assert_complex_eq(sv.data[1], c(FRAC_1_SQRT_2, 0.0), "H|0⟩[1] should be 1/√2");

    // Test from_circuit with Bell state
    let mut circuit2 = Circuit::new(2);
    circuit2.h(Qubit::new(0)).unwrap();
    circuit2.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let sv2 = Statevector::from_circuit(&circuit2).unwrap();
    assert_eq!(sv2.num_qubits, 2);
    assert_complex_eq(sv2.data[0], c(FRAC_1_SQRT_2, 0.0), "Bell |00⟩ amp");
    assert_complex_eq(sv2.data[1], c(0.0, 0.0), "Bell |01⟩ amp should be 0");
    assert_complex_eq(sv2.data[2], c(0.0, 0.0), "Bell |10⟩ amp should be 0");
    assert_complex_eq(sv2.data[3], c(FRAC_1_SQRT_2, 0.0), "Bell |11⟩ amp");

    // Test from_circuit with parameterized gates
    let mut circuit3 = Circuit::new(1);
    circuit3.rx(Qubit::new(0), PI).unwrap();

    let sv3 = Statevector::from_circuit(&circuit3).unwrap();
    assert_complex_eq(sv3.data[0], c(0.0, 0.0), "RX(π)|0⟩[0] should be 0");
    assert_complex_eq(sv3.data[1], c(0.0, -1.0), "RX(π)|0⟩[1] should be -i");
}

#[test]
fn test_from_circuit_3qubit_ghz() {
    use crate::circuit::Circuit;

    // Create GHZ state via circuit
    let mut circuit = Circuit::new(3);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let sv = Statevector::from_circuit(&circuit).unwrap();
    assert_eq!(sv.num_qubits, 3);
    assert_complex_eq(sv.data[0], c(FRAC_1_SQRT_2, 0.0), "GHZ |000⟩ amp");
    assert_complex_eq(sv.data[7], c(FRAC_1_SQRT_2, 0.0), "GHZ |111⟩ amp");

    for i in 1..7 {
        assert_complex_eq(
            sv.data[i],
            c(0.0, 0.0),
            &format!("GHZ |{:03b}⟩ should be 0", i),
        );
    }
    assert_normalized(&sv);
}

#[test]
fn test_from_circuit_ignores_terminal_measurement_declarations() {
    use crate::circuit::{Circuit, Qubit};
    use crate::device::Outcome;

    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let out = circuit
        .measure_bits([Qubit::new(1), Qubit::new(0)])
        .unwrap();

    let sv = Statevector::from_circuit(&circuit).unwrap();
    let probs = sv.probs(&out).unwrap();

    assert_eq!(probs.len(), 2);
    assert!((probs[&Outcome::from_bitstring("00").unwrap()] - 0.5).abs() < EPSILON);
    assert!((probs[&Outcome::from_bitstring("11").unwrap()] - 0.5).abs() < EPSILON);
}

#[test]
fn test_ccx_target_middle() {
    // Case B: Target is Middle (Min=Control, Mid=Target, Max=Control)
    // Test CCX(0, 2, 1): Controls are 0 and 2, Target is 1
    // Initial state |101⟩ (q0=1, q1=0, q2=1). Indices: 1 + 0 + 4 = 5.
    // Expected result: Target flips 0->1 => |111⟩ (Index 7).
    let mut sv = Statevector::new(3);
    sv.apply_x(0).unwrap();
    sv.apply_x(2).unwrap(); // Set |101⟩

    // Apply CCX(c0=0, c1=2, target=1)
    sv.apply_ccx(0, 2, 1).unwrap();

    assert_complex_eq(
        sv.data[5],
        c(0.0, 0.0),
        "CCX(0,2,1)|101⟩ original should be 0",
    );
    assert_complex_eq(
        sv.data[7],
        c(1.0, 0.0),
        "CCX(0,2,1)|101⟩ should flip to |111⟩",
    );
}

#[test]
fn test_apply_unitary_asymmetric() {
    use ndarray::Array2;
    // Define a custom gate V that performs:
    // |00⟩ -> |00⟩
    // |01⟩ -> i|10⟩  (Swap + Phase)
    // |10⟩ -> |01⟩   (Swap)
    // |11⟩ -> |11⟩
    // This matrix is not diagonal and not purely real.
    // Matrix cols: 00, 01, 10, 11
    let matrix = Array2::from_shape_vec(
        (4, 4),
        vec![
            c(1.0, 0.0),
            c(0.0, 0.0),
            c(0.0, 0.0),
            c(0.0, 0.0), // Row 00
            c(0.0, 0.0),
            c(0.0, 0.0),
            c(1.0, 0.0),
            c(0.0, 0.0), // Row 01 (receives from 10)
            c(0.0, 0.0),
            c(0.0, 1.0),
            c(0.0, 0.0),
            c(0.0, 0.0), // Row 10 (receives from 01, *i)
            c(0.0, 0.0),
            c(0.0, 0.0),
            c(0.0, 0.0),
            c(1.0, 0.0), // Row 11
        ],
    )
    .unwrap();

    let mut sv = Statevector::new(2);
    sv.apply_x(0).unwrap(); // Start with |01⟩ (q0=1, q1=0) -> Logic |01⟩?
    // Wait, Statevector index logic:
    // index 1 = binary 01 => q1=0, q0=1.
    // If apply_unitary_gate uses logic [q1, q0], then |01⟩ is index 1.

    // Apply V to q1, q0 (Logic order |q1 q0⟩)
    // Input |01⟩ should become i|10⟩ (index 2, with phase i)
    sv.apply_unitary_gate(&[1, 0], &matrix).unwrap();

    assert_complex_eq(sv.data[1], c(0.0, 0.0), "Old state |01⟩ should be empty");
    assert_complex_eq(sv.data[2], c(0.0, 1.0), "New state should be i|10⟩");
}

#[test]
fn test_expectation_z_on_zero() {
    // |0⟩ state, ⟨Z⟩ = 1
    let sv = Statevector::new(1);
    let mut ps = PauliString::new(1);
    ps.set_pauli(0, Pauli::Z);
    let h = Hamiltonian::from_pauli(ps);

    let exp = sv.expectation(&h).unwrap();
    assert!(
        (exp - 1.0).abs() < EPSILON,
        "<0|Z|0> should be 1, got {}",
        exp
    );
}

#[test]
fn test_expectation_z_on_one() {
    // |1⟩ state, ⟨Z⟩ = -1
    let mut sv = Statevector::new(1);
    sv.apply_x(0).unwrap();

    let mut ps = PauliString::new(1);
    ps.set_pauli(0, Pauli::Z);
    let h = Hamiltonian::from_pauli(ps);

    let exp = sv.expectation(&h).unwrap();
    assert!(
        (exp + 1.0).abs() < EPSILON,
        "<1|Z|1> should be -1, got {}",
        exp
    );
}

#[test]
fn test_expectation_x_on_plus() {
    // |+⟩ state, ⟨X⟩ = 1
    let mut sv = Statevector::new(1);
    sv.apply_h(0).unwrap();

    let mut ps = PauliString::new(1);
    ps.set_pauli(0, Pauli::X);
    let h = Hamiltonian::from_pauli(ps);

    let exp = sv.expectation(&h).unwrap();
    assert!(
        (exp - 1.0).abs() < EPSILON,
        "<+|X|+> should be 1, got {}",
        exp
    );
}

#[test]
fn test_expectation_multi_qubit() {
    // Bell state |Φ+⟩ = (|00⟩ + |11⟩)/√2
    // For H = Z⊗I, ⟨H⟩ = 0
    let mut sv = Statevector::new(2);
    sv.apply_h(0).unwrap();
    sv.apply_cx(0, 1).unwrap();

    let mut ps = PauliString::new(2);
    ps.set_pauli(0, Pauli::Z); // Z on qubit 0
    let h = Hamiltonian::from_pauli(ps);

    let exp = sv.expectation(&h).unwrap();
    assert!(exp.abs() < EPSILON, "<Φ+|Z⊗I|Φ+> should be 0, got {}", exp);
}

#[test]
fn test_expectation_zz_on_bell() {
    // Bell state |Φ+⟩ = (|00⟩ + |11⟩)/√2
    // For H = Z⊗Z, ⟨H⟩ = 1
    let mut sv = Statevector::new(2);
    sv.apply_h(0).unwrap();
    sv.apply_cx(0, 1).unwrap();

    let mut ps = PauliString::new(2);
    ps.set_pauli(0, Pauli::Z);
    ps.set_pauli(1, Pauli::Z);
    let h = Hamiltonian::from_pauli(ps);

    let exp = sv.expectation(&h).unwrap();
    assert!(
        (exp - 1.0).abs() < EPSILON,
        "<Φ+|ZZ|Φ+> should be 1, got {}",
        exp
    );
}

#[test]
fn test_expectation_with_coefficient() {
    // |0⟩ state, ⟨2Z⟩ = 2
    let sv = Statevector::new(1);
    let mut ps = PauliString::new(1);
    ps.set_pauli(0, Pauli::Z);
    let h = Hamiltonian::from_list(vec![(ps, Complex64::new(2.0, 0.0))]).unwrap();

    let exp = sv.expectation(&h).unwrap();
    assert!(
        (exp - 2.0).abs() < EPSILON,
        "<0|2Z|0> should be 2, got {}",
        exp
    );
}

#[test]
fn test_expectation_qubit_mismatch() {
    let sv = Statevector::new(1);
    let h = Hamiltonian::new(2); // 2 qubit Hamiltonian

    let result = sv.expectation(&h);
    assert!(result.is_err(), "Should error on qubit mismatch");
}

/// Applies a complex unitary via `apply_single_qubit_gate` and checks result
/// matches the analytically expected amplitudes. This exercises the AVX2 path
/// on x86_64 (or scalar fallback elsewhere) for complex×complex multiplication.
#[test]
fn test_sqg_avx2_complex_phase_gate() {
    // S gate: [[1,0],[0,i]] applied to |+⟩ = (|0⟩+|1⟩)/√2 → (|0⟩+i|1⟩)/√2
    let mut sv = Statevector::new(1);
    sv.apply_h(0).unwrap();

    let i = Complex64::new(0.0, 1.0);
    let s_matrix = [
        [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
        [Complex64::new(0.0, 0.0), i],
    ];
    sv.apply_single_qubit_gate(0, s_matrix).unwrap();

    let inv_sqrt2 = 1.0 / 2.0_f64.sqrt();
    let d = sv.data();
    assert!(
        (d[0].re - inv_sqrt2).abs() < EPSILON,
        "Re(|0⟩) should be 1/√2, got {}",
        d[0].re
    );
    assert!(d[0].im.abs() < EPSILON, "Im(|0⟩) should be 0");
    assert!(d[1].re.abs() < EPSILON, "Re(|1⟩) should be 0");
    assert!(
        (d[1].im - inv_sqrt2).abs() < EPSILON,
        "Im(|1⟩) should be 1/√2, got {}",
        d[1].im
    );
}

/// Tests the AVX2 SIMD path on a larger state (n=5, qubit 2) to ensure the
/// SIMD 2-element loop and scalar tail both produce correct results.
#[test]
fn test_sqg_avx2_larger_state() {
    // Apply T = [[1,0],[0,e^{iπ/4}]] to qubit 2 of |++++⟩ state.
    // Each amplitude is 1/√(2^4) = 1/4. After T on qubit 2:
    // Amplitudes with qubit-2 = |0⟩ stay at 1/4.
    // Amplitudes with qubit-2 = |1⟩ get multiplied by e^{iπ/4}.
    use std::f64::consts::FRAC_PI_4;
    let n = 5;
    let mut sv = Statevector::new(n);
    for q in 0..n {
        sv.apply_h(q).unwrap();
    }

    let phase = Complex64::from_polar(1.0, FRAC_PI_4);
    let t_matrix = [
        [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
        [Complex64::new(0.0, 0.0), phase],
    ];
    sv.apply_single_qubit_gate(2, t_matrix).unwrap();

    let base_amp = 1.0 / (2.0_f64.powi(n as i32)).sqrt();
    let d = sv.data();
    for (idx, &amp) in d.iter().enumerate() {
        // qubit 2 bit: bit 2 of index
        let qubit2_is_one = (idx >> 2) & 1 == 1;
        let expected = if qubit2_is_one {
            phase * base_amp
        } else {
            Complex64::new(base_amp, 0.0)
        };
        assert!(
            (amp - expected).norm() < EPSILON,
            "idx={}: expected {:?}, got {:?}",
            idx,
            expected,
            amp
        );
    }
}

/// Verifies that AlignedBuffer gives 64-byte alignment, enabling safe AVX-512 loads.
#[test]
fn test_statevector_data_alignment() {
    let sv = Statevector::new(10);
    let ptr = sv.data().as_ptr() as usize;
    assert_eq!(
        ptr % 64,
        0,
        "Statevector::data must be 64-byte aligned for AVX2/AVX-512; got alignment {}",
        ptr % 64
    );
}

#[test]
fn test_apply_circuit_bell_state() {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let mut sv = Statevector::new(2);
    sv.apply_circuit(&circuit).unwrap();

    assert_complex_eq(sv.data[0], c(FRAC_1_SQRT_2, 0.0), "Bell |00⟩ amp");
    assert_complex_eq(sv.data[1], c(0.0, 0.0), "Bell |01⟩ amp should be 0");
    assert_complex_eq(sv.data[2], c(0.0, 0.0), "Bell |10⟩ amp should be 0");
    assert_complex_eq(sv.data[3], c(FRAC_1_SQRT_2, 0.0), "Bell |11⟩ amp");
    assert_normalized(&sv);
}

#[test]
fn test_apply_circuit_parameterized() {
    let mut circuit = Circuit::new(1);
    circuit.rx(Qubit::new(0), PI / 2.0).unwrap();

    let mut sv = Statevector::new(1);
    sv.apply_circuit(&circuit).unwrap();

    // RX(π/2)|0⟩ = (|0⟩ - i|1⟩)/√2
    let inv_sqrt2 = FRAC_1_SQRT_2;
    assert_complex_eq(
        sv.data[0],
        c(inv_sqrt2, 0.0),
        "RX(π/2)|0⟩[0] should be 1/√2",
    );
    assert_complex_eq(
        sv.data[1],
        c(0.0, -inv_sqrt2),
        "RX(π/2)|0⟩[1] should be -i/√2",
    );
    assert_normalized(&sv);
}

#[test]
fn test_apply_circuit_toffoli() {
    // |110⟩ with CCX(0,1,2) -> |111⟩
    let mut circuit = Circuit::new(3);
    circuit.x(Qubit::new(0)).unwrap();
    circuit.x(Qubit::new(1)).unwrap();
    circuit
        .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();

    let mut sv = Statevector::new(3);
    sv.apply_circuit(&circuit).unwrap();

    for i in 0..7 {
        assert_complex_eq(
            sv.data[i],
            c(0.0, 0.0),
            &format!(" amplitude |{:03b}⟩ should be 0", i),
        );
    }
    assert_complex_eq(sv.data[7], c(1.0, 0.0), "CCX|110⟩ should be |111⟩");
    assert_normalized(&sv);
}

#[test]
fn test_apply_circuit_incremental() {
    // Apply H first to get |+⟩, then apply Z to get |-
    let mut h_circuit = Circuit::new(1);
    h_circuit.h(Qubit::new(0)).unwrap();

    let mut z_circuit = Circuit::new(1);
    z_circuit.z(Qubit::new(0)).unwrap();

    let mut sv = Statevector::new(1);
    sv.apply_circuit(&h_circuit).unwrap();
    sv.apply_circuit(&z_circuit).unwrap();

    // ZH|0⟩ = Z|+⟩ = |-
    assert_complex_eq(sv.data[0], c(FRAC_1_SQRT_2, 0.0), "ZH|0⟩[0] should be 1/√2");
    assert_complex_eq(
        sv.data[1],
        c(-FRAC_1_SQRT_2, 0.0),
        "ZH|0⟩[1] should be -1/√2",
    );
    assert_normalized(&sv);
}

#[test]
fn test_apply_circuit_dimension_mismatch() {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();

    let mut sv = Statevector::new(1);
    let result = sv.apply_circuit(&circuit);

    assert!(
        matches!(result, Err(QisError::InvalidStateDimension(2))),
        "Should error on dimension mismatch"
    );
}

#[test]
fn test_apply_circuit_reset_directive() {
    let mut circuit = Circuit::new(1);
    circuit.x(Qubit::new(0)).unwrap();
    circuit.reset(Qubit::new(0)).unwrap();

    let mut sv = Statevector::new(1);
    sv.apply_circuit(&circuit).unwrap();

    assert_complex_eq(sv.data[0], c(1.0, 0.0), "X then reset should return |0>");
    assert_complex_eq(sv.data[1], c(0.0, 0.0), "X then reset should clear |1>");
}

#[test]
fn test_apply_circuit_classical_control_flow_error() {
    use crate::circuit::ClassicalExpr;

    let mut circuit = Circuit::new(1);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.x(Qubit::new(0))?;
            Ok(())
        })
        .unwrap();

    let mut sv = Statevector::new(1);
    assert!(matches!(
        sv.apply_circuit(&circuit),
        Err(QisError::UnsupportedOperation(_))
    ));
}

#[test]
fn test_apply_standard_gate_arity_mismatch() {
    let mut sv = Statevector::new(2);
    let result = sv.apply_standard_gate(StandardGate::CX, &[0], &[]);
    assert!(
        result.is_err(),
        "CX with 1 qubit should fail, got {:?}",
        result
    );
}

#[test]
fn test_measure_out_of_bounds() {
    let mut sv = Statevector::new(2);
    let result = sv.measure(99);
    assert!(
        result.is_err(),
        "measure on out-of-bounds qubit should fail, got {:?}",
        result
    );
}

#[test]
fn test_measure_deterministic_zero_and_one() {
    let mut zero = Statevector::new(1);
    assert!(!zero.measure(0).unwrap());
    assert_complex_eq(zero.data[0], c(1.0, 0.0), "|0> remains |0>");
    assert_complex_eq(zero.data[1], c(0.0, 0.0), "|1> amplitude remains zero");

    let mut one = Statevector::new(1);
    one.apply_x(0).unwrap();
    assert!(one.measure(0).unwrap());
    assert_complex_eq(one.data[0], c(0.0, 0.0), "|0> amplitude remains zero");
    assert_complex_eq(one.data[1], c(1.0, 0.0), "|1> remains |1>");
}

#[test]
fn test_measure_plus_statistics_and_collapse() {
    let shots = 1000;
    let mut ones = 0;
    for _ in 0..shots {
        let mut sv = Statevector::new(1);
        sv.apply_h(0).unwrap();
        let outcome = sv.measure(0).unwrap();
        if outcome {
            ones += 1;
            assert_complex_eq(sv.data[0], c(0.0, 0.0), "collapsed |0> amp");
            assert_complex_eq(sv.data[1], c(1.0, 0.0), "collapsed |1> amp");
        } else {
            assert_complex_eq(sv.data[0], c(1.0, 0.0), "collapsed |0> amp");
            assert_complex_eq(sv.data[1], c(0.0, 0.0), "collapsed |1> amp");
        }
    }
    assert!(
        (350..=650).contains(&ones),
        "|+> measurement should be near 50/50; ones={ones}/{shots}"
    );
}

#[test]
fn test_measure_with_rng_is_reproducible() {
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    let shots = 64;
    let mut rng_a = SmallRng::seed_from_u64(1234);
    let mut rng_b = SmallRng::seed_from_u64(1234);

    let outcomes_a: Vec<bool> = (0..shots)
        .map(|_| {
            let mut sv = Statevector::new(1);
            sv.apply_h(0).unwrap();
            sv.measure_with_rng(0, &mut rng_a).unwrap()
        })
        .collect();

    let outcomes_b: Vec<bool> = (0..shots)
        .map(|_| {
            let mut sv = Statevector::new(1);
            sv.apply_h(0).unwrap();
            sv.measure_with_rng(0, &mut rng_b).unwrap()
        })
        .collect();

    assert_eq!(outcomes_a, outcomes_b);
    assert!(
        outcomes_a.iter().any(|&b| b) && outcomes_a.iter().any(|&b| !b),
        "seeded |+> samples should include both outcomes"
    );
}

#[test]
fn test_zero_qubit_measure_boundary() {
    let mut sv = Statevector::new(0);
    assert_eq!(sv.num_qubits, 0);
    assert_eq!(sv.data.len(), 1);
    assert_complex_eq(sv.data[0], c(1.0, 0.0), "0-qubit scalar state");
    assert!(sv.measure(0).is_err());
    assert_eq!(sv.probabilities(), vec![1.0]);
}

#[test]
fn test_zero_qubit_validate_error_message() {
    let mut sv = Statevector::new(0);
    let err = sv.apply_x(0).unwrap_err();
    assert!(
        matches!(&err, QisError::InvalidParameterValue(msg) if msg.contains("zero-qubit")),
        "Expected InvalidParameterValue with zero-qubit message, got {:?}",
        err
    );
}

#[test]
fn test_measure_all_bit_packing() {
    let mut sv = Statevector::new(4);
    sv.apply_x(0).unwrap();
    sv.apply_x(2).unwrap();
    let outcome = sv.measure_all();
    assert!(outcome.is_one(0));
    assert!(!outcome.is_one(1));
    assert!(outcome.is_one(2));
    assert!(!outcome.is_one(3));
}

#[test]
fn test_sample_shots_zero_and_bell_distribution() {
    let mut sv = Statevector::new(2);
    sv.apply_h(0).unwrap();
    sv.apply_cx(0, 1).unwrap();

    assert!(sv.sample_shots(0).is_empty());

    let shots = sv.sample_shots(1000);
    assert_eq!(shots.len(), 1000);
    let ones = shots
        .iter()
        .filter(|outcome| outcome.is_one(0) && outcome.is_one(1))
        .count();
    assert!(
        shots
            .iter()
            .all(|outcome| outcome.is_one(0) == outcome.is_one(1))
    );
    assert!(
        (350..=650).contains(&ones),
        "Bell samples should be near 50/50 between 00 and 11; ones={ones}"
    );
}

#[test]
fn test_sample_uses_measurement_qubit_order() {
    use crate::circuit::{Circuit, Qubit};
    use crate::device::{Outcome, Status};

    let mut circuit = Circuit::new(2);
    circuit.x(Qubit::new(0)).unwrap();
    let out = circuit
        .measure_bits([Qubit::new(1), Qubit::new(0)])
        .unwrap();

    let sv = Statevector::from_circuit(&circuit).unwrap();
    let result = sv.sample(&out, 16).unwrap();

    assert_eq!(result.shots(), 16);
    assert_eq!(result.num_qubits(), 2);
    assert_eq!(result.qubits(), &vec![Qubit::new(1), Qubit::new(0)]);
    assert_eq!(result.status(), &Status::Completed);
    assert_eq!(
        result.counts().get(&Outcome::from_bitstring("10").unwrap()),
        Some(&16)
    );
    assert_eq!(
        result.probabilities().as_ref().unwrap()[&Outcome::from_bitstring("10").unwrap()],
        1.0
    );
}

#[test]
fn test_probs_marginalizes_unmeasured_qubits() {
    use crate::circuit::Circuit;
    use crate::device::Outcome;

    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let out = circuit.measure(Qubit::new(0)).unwrap();

    let sv = Statevector::from_circuit(&circuit).unwrap();
    let probs = sv.probs(&out).unwrap();

    assert_eq!(probs.len(), 2);
    assert!((probs[&Outcome::from_bitstring("0").unwrap()] - 0.5).abs() < EPSILON);
    assert!((probs[&Outcome::from_bitstring("1").unwrap()] - 0.5).abs() < EPSILON);
}

#[test]
fn test_sample_rejects_measurement_qubit_outside_state() {
    use crate::circuit::Circuit;

    let sv = Statevector::new(1);
    let mut circuit = Circuit::new(3);
    let measurement = circuit.measure(Qubit::new(2)).unwrap();

    assert!(matches!(
        sv.sample(&measurement, 1),
        Err(QisError::IndexOutOfBounds { index: 2, max: 0 })
    ));
}

#[test]
fn test_apply_unitary_gate_duplicate_qubits() {
    let mut sv = Statevector::new(2);
    let identity = ndarray::Array2::eye(4);
    let result = sv.apply_unitary_gate(&[0, 0], &identity);
    assert!(
        result.is_err(),
        "duplicate target qubits should be rejected, got {:?}",
        result
    );
}

#[test]
fn test_probabilities_parallel_threshold_14_qubits() {
    let n = 14;
    let mut sv = Statevector::new(n);
    for q in 0..n {
        sv.apply_h(q).unwrap();
    }
    let probs = sv.probabilities();
    assert_eq!(probs.len(), 1 << n);
    let expected = 1.0 / (1 << n) as f64;
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-10);
    assert!(probs.iter().all(|p| (p - expected).abs() < 1e-10));
}

#[test]
fn test_probabilities_large_20_qubit_uniform_superposition() {
    let n = 20;
    let mut sv = Statevector::new(n);
    for q in 0..n {
        sv.apply_h(q).unwrap();
    }
    let probs = sv.probabilities();
    assert_eq!(probs.len(), 1 << n);
    let expected = 1.0 / (1 << n) as f64;
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-9);
    assert!((probs[0] - expected).abs() < 1e-12);
    assert!((probs[(1 << n) - 1] - expected).abs() < 1e-12);
}

#[test]
fn test_apply_pauli_rotation_matches_gate_decomposition() {
    use crate::qis::pauli::{Pauli, PauliString};

    let mut native = Statevector::new(4);
    native.apply_h(0).unwrap();
    native.apply_rx(1, 0.37).unwrap();
    native.apply_ry(2, -0.21).unwrap();
    native.apply_cx(0, 3).unwrap();
    let mut decomposed = native.clone();

    let mut pauli = PauliString::new(4);
    pauli.set_pauli(0, Pauli::X);
    pauli.set_pauli(1, Pauli::Y);
    pauli.set_pauli(3, Pauli::Z);
    let theta = 0.413;

    native.apply_pauli_rotation(&pauli, theta).unwrap();

    // Same decomposition used by cqlib-vqe 1.3.x.
    decomposed.apply_h(0).unwrap();
    decomposed.apply_rx(1, std::f64::consts::FRAC_PI_2).unwrap();
    decomposed.apply_cx(0, 1).unwrap();
    decomposed.apply_cx(1, 3).unwrap();
    decomposed.apply_rz(3, theta).unwrap();
    decomposed.apply_cx(1, 3).unwrap();
    decomposed.apply_cx(0, 1).unwrap();
    decomposed.apply_h(0).unwrap();
    decomposed
        .apply_rx(1, -std::f64::consts::FRAC_PI_2)
        .unwrap();

    for (actual, expected) in native.data().iter().zip(decomposed.data()) {
        assert!(
            (actual - expected).norm() < 1e-12,
            "{actual:?} != {expected:?}"
        );
    }
}

#[test]
fn test_apply_pauli_rotation_rejects_imaginary_global_phase() {
    use crate::qis::pauli::{PauliString, Phase};

    let mut state = Statevector::new(1);
    let mut pauli = PauliString::new(1);
    pauli.phase = Phase::I;
    assert!(state.apply_pauli_rotation(&pauli, 0.2).is_err());
}
