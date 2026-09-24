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
use crate::circuit::circuit_to_matrix;
use crate::circuit::gate::StandardGate;
use crate::circuit::{Circuit, Instruction};
use crate::qis::Hamiltonian;
use crate::qis::evolution::TrotterMode;
use crate::qis::pauli::PauliString;
use approx::assert_abs_diff_eq;
use ndarray::array;
use num_complex::Complex64;
use std::f64::consts::PI;

#[test]
fn test_pauli_evolution_x() {
    let mut circuit = Circuit::new(1);
    let qubits = circuit.qubits();

    // X rotation should be equivalent to RX
    let pauli: PauliString = "X".parse().unwrap();
    circuit
        .pauli_evolution(&pauli, std::f64::consts::PI, &qubits)
        .unwrap();

    // Check that we have the right gates: H - RZ - H
    let ops = circuit.operations();
    assert_eq!(ops.len(), 3);
}

#[test]
fn test_pauli_evolution_z() {
    let mut circuit = Circuit::new(1);
    let qubits = circuit.qubits();

    // Z rotation should be just RZ
    let pauli: PauliString = "Z".parse().unwrap();
    circuit
        .pauli_evolution(&pauli, std::f64::consts::PI, &qubits)
        .unwrap();

    // Check that we have just one gate: RZ
    let ops = circuit.operations();
    assert_eq!(ops.len(), 1);
}

#[test]
fn test_pauli_evolution_multi_qubit() {
    let mut circuit = Circuit::new(2);
    let qubits = circuit.qubits();

    // XX evolution: e^(-iθ/2 * X⊗X)
    // Chain structure: H(q0), H(q1), CNOT(0->1), RZ(1), CNOT(0->1), H(q0), H(q1)
    let pauli: PauliString = "XX".parse().unwrap();
    circuit
        .pauli_evolution(&pauli, std::f64::consts::PI / 2.0, &qubits)
        .unwrap();

    // Gate sequence: H - H - CNOT(0->1) - RZ(1) - CNOT(0->1) - H - H
    let ops = circuit.operations();

    // Check total gate count: 2 H (X->Z) + 2 CNOTs + 1 RZ + 2 H (reverse)
    assert_eq!(
        ops.len(),
        7,
        "Expected 7 gates for XX evolution: H, H, CNOT, RZ, CNOT, H, H"
    );

    // Verify the structure
    // First two should be H gates (basis transformation for X)
    assert!(matches!(
        ops[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert!(matches!(
        ops[1].instruction,
        Instruction::Standard(StandardGate::H)
    ));

    // Third should be CNOT
    assert!(matches!(
        ops[2].instruction,
        Instruction::Standard(StandardGate::CX)
    ));

    // Fourth should be RZ on last qubit (qubit 1)
    assert!(matches!(
        ops[3].instruction,
        Instruction::Standard(StandardGate::RZ)
    ));
    assert_eq!(ops[3].qubits[0], qubits[1]);

    // Fifth should be CNOT (reverse chain)
    assert!(matches!(
        ops[4].instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    assert_eq!(ops[4].qubits[0], qubits[0]); // control
    assert_eq!(ops[4].qubits[1], qubits[1]); // target

    // Last two should be H gates (reverse basis)
    assert!(matches!(
        ops[5].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert!(matches!(
        ops[6].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn test_pauli_evolution_identity() {
    let mut circuit = Circuit::new(2);
    let qubits = circuit.qubits();

    // Identity evolution should add global phase
    let pauli: PauliString = "II".parse().unwrap();
    circuit
        .pauli_evolution(&pauli, std::f64::consts::PI, &qubits)
        .unwrap();

    // No gates should be added, only global phase
    let ops = circuit.operations();
    assert_eq!(ops.len(), 0);
}

#[test]
fn test_pauli_evolution_qubit_mismatch() {
    let mut circuit = Circuit::new(2);
    let qubits = circuit.qubits();

    // Mismatched qubit count
    let pauli: PauliString = "XXX".parse().unwrap();
    let result = circuit.pauli_evolution(&pauli, 1.0, &qubits);

    assert!(result.is_err());
}

#[test]
fn test_pauli_evolution_with_phase() {
    let mut circuit = Circuit::new(1);
    let qubits = circuit.qubits();

    // Pauli string with -1 phase
    let pauli: PauliString = "-X".parse().unwrap();
    circuit
        .pauli_evolution(&pauli, std::f64::consts::PI, &qubits)
        .unwrap();

    // The phase should flip the sign of the angle
    // So e^(-i * π/2 * (-X)) = e^(i * π/2 * X)
    // This should work without error
    let ops = circuit.operations();
    assert_eq!(ops.len(), 3);
}

#[test]
fn test_pauli_evolution_y() {
    let mut circuit = Circuit::new(1);
    let qubits = circuit.qubits();

    // Y rotation: S† - H - RZ - H - S
    let pauli: PauliString = "Y".parse().unwrap();
    circuit
        .pauli_evolution(&pauli, std::f64::consts::PI / 2.0, &qubits)
        .unwrap();

    let ops = circuit.operations();
    assert_eq!(ops.len(), 5);
}

#[test]
fn test_pauli_evolution_mixed() {
    let mut circuit = Circuit::new(3);
    let qubits = circuit.qubits();

    // XZY evolution
    let pauli: PauliString = "XZY".parse().unwrap();
    circuit.pauli_evolution(&pauli, 1.0, &qubits).unwrap();

    // Structure: basis_transforms + cnot_ladder + rz + reverse_cnot_ladder + reverse_basis
    // X -> H, Z -> nothing, Y -> H·S†
    // CNOT ladder: CNOT(2->0), CNOT(1->0) assuming 0 is first non-I
    let ops = circuit.operations();

    // Should have gates
    assert!(
        ops.len() >= 7,
        "Expected at least 7 gates for 3-qubit XZY evolution"
    );

    // Verify we have CNOTs in the circuit
    let cnot_count = ops
        .iter()
        .filter(|op| {
            matches!(
                op.instruction,
                crate::circuit::Instruction::Standard(StandardGate::CX)
            )
        })
        .count();
    assert!(
        cnot_count >= 2,
        "Expected at least 2 CNOTs for 3-qubit evolution, got {}",
        cnot_count
    );
}

#[test]
fn test_pauli_evolution_zz_two_qubit() {
    let mut circuit = Circuit::new(2);
    let qubits = circuit.qubits();

    // ZZ evolution: e^(-iθ/2 * Z⊗Z)
    // Chain structure: CNOT(0->1) - RZ(1) - CNOT(0->1)
    let pauli: PauliString = "ZZ".parse().unwrap();
    circuit
        .pauli_evolution(&pauli, std::f64::consts::PI / 2.0, &qubits)
        .unwrap();

    let ops = circuit.operations();
    assert_eq!(
        ops.len(),
        3,
        "Expected 3 gates for ZZ evolution: CNOT, RZ, CNOT"
    );

    // Verify chain structure: CNOT(q0,q1), RZ(q1), CNOT(q0,q1)
    assert!(matches!(
        ops[0].instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    assert_eq!(ops[0].qubits[0], qubits[0]); // control
    assert_eq!(ops[0].qubits[1], qubits[1]); // target

    assert!(matches!(
        ops[1].instruction,
        Instruction::Standard(StandardGate::RZ)
    ));
    assert_eq!(ops[1].qubits[0], qubits[1]); // RZ on last qubit

    assert!(matches!(
        ops[2].instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    assert_eq!(ops[2].qubits[0], qubits[0]); // control
    assert_eq!(ops[2].qubits[1], qubits[1]); // target
}

#[test]
fn test_pauli_evolution_three_qubit_zzz() {
    let mut circuit = Circuit::new(3);
    let qubits = circuit.qubits();

    // ZZZ evolution: e^(-iθ/2 * Z⊗Z⊗Z)
    // Chain structure: CNOT(0->1) - CNOT(1->2) - RZ(2) - CNOT(1->2) - CNOT(0->1)
    let pauli: PauliString = "ZZZ".parse().unwrap();
    circuit.pauli_evolution(&pauli, 1.0, &qubits).unwrap();

    let ops = circuit.operations();
    assert_eq!(
        ops.len(),
        5,
        "Expected 5 gates for ZZZ evolution: 2 CNOTs + RZ + 2 CNOTs"
    );

    // Verify chain structure: CNOT(q0,q1), CNOT(q1,q2), RZ(q2), CNOT(q1,q2), CNOT(q0,q1)
    assert!(matches!(
        ops[0].instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    assert_eq!(ops[0].qubits[0], qubits[0]); // control
    assert_eq!(ops[0].qubits[1], qubits[1]); // target

    assert!(matches!(
        ops[1].instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    assert_eq!(ops[1].qubits[0], qubits[1]); // control
    assert_eq!(ops[1].qubits[1], qubits[2]); // target

    assert!(matches!(
        ops[2].instruction,
        Instruction::Standard(StandardGate::RZ)
    ));
    assert_eq!(ops[2].qubits[0], qubits[2]); // RZ on last qubit

    assert!(matches!(
        ops[3].instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    assert_eq!(ops[3].qubits[0], qubits[1]); // control
    assert_eq!(ops[3].qubits[1], qubits[2]); // target

    assert!(matches!(
        ops[4].instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    assert_eq!(ops[4].qubits[0], qubits[0]); // control
    assert_eq!(ops[4].qubits[1], qubits[1]); // target
}

#[test]
fn test_trotter_first_order_basic() {
    // H = 0.5 * ZZ
    let mut h = Hamiltonian::new(2);
    let pauli: PauliString = "ZZ".parse().unwrap();
    h.add_term(pauli, 0.5.into()).unwrap();

    // Create Trotter circuit: t=1.0, steps=2
    let circuit = h
        .to_trotter_circuit(1.0, 2, TrotterMode::FirstOrder)
        .unwrap();

    // Should have 2 steps * (CNOT + RZ + CNOT) = 6 gates
    let ops = circuit.operations();
    assert_eq!(ops.len(), 6, "Expected 6 gates for 2-step ZZ Trotter");
}

#[test]
fn test_trotter_two_terms() {
    // H = 0.5 * ZZ + 0.3 * XX
    let mut h = Hamiltonian::new(2);
    h.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();
    h.add_term("XX".parse().unwrap(), 0.3.into()).unwrap();

    // Create Trotter circuit: t=1.0, steps=1
    let circuit = h
        .to_trotter_circuit(1.0, 1, TrotterMode::FirstOrder)
        .unwrap();

    // For ZZ: CNOT-RZ-CNOT (3 gates)
    // For XX: H-H-CNOT-RZ-CNOT-H-H (7 gates)
    // Total: 10 gates
    let ops = circuit.operations();
    assert_eq!(ops.len(), 10, "Expected 10 gates for 1-step ZZ+XX Trotter");
}

#[test]
fn test_trotter_second_order() {
    // H = 0.5 * ZZ
    let mut h = Hamiltonian::new(2);
    h.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();

    // Create 2nd-order Trotter circuit: t=1.0, steps=1
    let circuit = h
        .to_trotter_circuit(1.0, 1, TrotterMode::SecondOrder)
        .unwrap();

    // Second order: forward half + backward half
    // ZZ forward half: CNOT-RZ_half-CNOT (3 gates)
    // ZZ backward half: CNOT-RZ_half-CNOT (3 gates)
    // Total: 6 gates
    let ops = circuit.operations();
    assert_eq!(
        ops.len(),
        6,
        "Expected 6 gates for 1-step 2nd-order Trotter"
    );
}

#[test]
fn test_trotter_empty_hamiltonian_error() {
    let h = Hamiltonian::new(2);

    let result = h.to_trotter_circuit(1.0, 1, TrotterMode::FirstOrder);
    assert!(result.is_err(), "Should error on empty Hamiltonian");
}

#[test]
fn test_trotter_zero_steps_error() {
    let mut h = Hamiltonian::new(2);
    h.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();

    let result = h.to_trotter_circuit(1.0, 0, TrotterMode::FirstOrder);
    assert!(result.is_err(), "Should error on zero steps");
}

#[test]
fn test_trotter_scale_factor_overflow_returns_error() {
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), f64::MAX.into()).unwrap();

    for mode in [TrotterMode::FirstOrder, TrotterMode::Randomized(42)] {
        // All inputs are finite, but doubling the coefficient overflows.
        let result = h.to_trotter_circuit(0.25, 2, mode);
        assert!(matches!(result, Err(QisError::UnsupportedOperation(_))));
    }
}

#[test]
fn test_trotter_merged_coefficient_overflow_returns_error() {
    let mut h = Hamiltonian::new(1);
    for _ in 0..2 {
        h.add_term("Z".parse().unwrap(), f64::MAX.into()).unwrap();
    }

    for mode in [
        TrotterMode::FirstOrder,
        TrotterMode::SecondOrder,
        TrotterMode::Randomized(42),
    ] {
        // Simplification can overflow even before the scale factor is computed.
        let result = h.to_trotter_circuit(0.25, 2, mode);
        assert!(matches!(result, Err(QisError::UnsupportedOperation(_))));
    }
}

#[test]
fn test_trotter_randomized() {
    // H = 0.5 * ZZ + 0.3 * XX
    let mut h = Hamiltonian::new(2);
    h.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();
    h.add_term("XX".parse().unwrap(), 0.3.into()).unwrap();

    // Create randomized Trotter circuit
    let circuit = h
        .to_trotter_circuit(1.0, 2, TrotterMode::Randomized(42))
        .unwrap();

    // Should have the same number of gates as first order
    // 2 steps * (3 for ZZ + 7 for XX) = 20 gates
    let ops = circuit.operations();
    assert_eq!(
        ops.len(),
        20,
        "Expected 20 gates for 2-step randomized Trotter"
    );
}

#[test]
fn test_trotter_multiple_steps() {
    // H = 0.5 * Z
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), 0.5.into()).unwrap();

    // 5 steps
    let circuit = h
        .to_trotter_circuit(1.0, 5, TrotterMode::FirstOrder)
        .unwrap();

    // Each step has 1 RZ gate
    let ops = circuit.operations();
    assert_eq!(
        ops.len(),
        5,
        "Expected 5 RZ gates for 5-step single-qubit Trotter"
    );
}

#[test]
fn test_trotter_circuit_num_qubits() {
    // H on 3 qubits
    let mut h = Hamiltonian::new(3);
    h.add_term("ZZZ".parse().unwrap(), 1.0.into()).unwrap();

    let circuit = h
        .to_trotter_circuit(1.0, 1, TrotterMode::FirstOrder)
        .unwrap();

    assert_eq!(
        circuit.num_qubits(),
        3,
        "Circuit should have same qubits as Hamiltonian"
    );
}

/// Test that Trotter evolution direction is correct (e^{-iHt}, not e^{+iHt})
///
/// For H = Z, U(t) = e^{-i Z t}.
/// When applied to |0⟩, Z|0⟩ = +|0⟩, so U(t)|0⟩ = e^{-it}|0⟩.
/// The phase should be e^{-it} (clockwise rotation in Bloch sphere).
///
/// The pauli_evolution angle should be θ = 2*c*t (positive for positive c and t).
#[test]
fn test_trotter_time_evolution_direction() {
    // H = 0.5 * Z
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), 0.5.into()).unwrap();

    // Time t = 2.0, steps = 1
    // Expected: θ = 2 * c * t = 2 * 0.5 * 2.0 = 2.0
    // Bug: θ = -2 * c * t = -2.0 would give wrong direction
    let circuit = h
        .to_trotter_circuit(2.0, 1, TrotterMode::FirstOrder)
        .unwrap();

    let ops = circuit.operations();
    assert_eq!(ops.len(), 1);

    // Check RZ gate has positive angle
    if let Instruction::Standard(StandardGate::RZ) = ops[0].instruction {
        // Get the parameter - should be Fixed(2.0) not Fixed(-2.0)
        match &ops[0].params[0] {
            crate::circuit::CircuitParam::Fixed(val) => {
                // θ = 2 * 0.5 * 2.0 = 2.0
                assert!(
                    *val > 0.0,
                    "RZ angle should be positive for positive coefficient and time, got {}",
                    val
                );
                assert!(
                    (val - 2.0).abs() < 1e-10,
                    "RZ angle should be 2.0, got {}",
                    val
                );
            }
            _ => panic!("Expected Fixed parameter for RZ angle"),
        }
    } else {
        panic!("Expected RZ gate");
    }
}

/// Test that negative time gives opposite angle (time reversal)
#[test]
fn test_trotter_negative_time() {
    // H = 0.5 * Z
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), 0.5.into()).unwrap();

    // Time t = -1.0 (backward evolution)
    // Expected: θ = 2 * c * t = 2 * 0.5 * (-1.0) = -1.0
    let circuit = h
        .to_trotter_circuit(-1.0, 1, TrotterMode::FirstOrder)
        .unwrap();

    let ops = circuit.operations();
    if let Instruction::Standard(StandardGate::RZ) = ops[0].instruction {
        match &ops[0].params[0] {
            crate::circuit::CircuitParam::Fixed(val) => {
                assert!(*val < 0.0, "RZ angle should be negative for negative time");
                assert!(
                    (val + 1.0).abs() < 1e-10,
                    "RZ angle should be -1.0, got {}",
                    val
                );
            }
            _ => panic!("Expected Fixed parameter"),
        }
    }
}

/// Assert that two complex matrices are approximately equal
fn assert_matrix_approx_eq(
    actual: &ndarray::Array2<num_complex::Complex64>,
    expected: &ndarray::Array2<num_complex::Complex64>,
    eps: f64,
) {
    assert_eq!(
        actual.shape(),
        expected.shape(),
        "Matrix shapes differ: {:?} vs {:?}",
        actual.shape(),
        expected.shape()
    );
    for (a, e) in actual.iter().zip(expected.iter()) {
        if (a - e).norm() > eps {
            panic!(
                "Matrices differ.\nActual:\n{:?}\nExpected:\n{:?}",
                actual, expected
            );
        }
    }
}

#[test]
fn test_trotter_matrix_equivalence_z() {
    // H = 0.5 * Z
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), 0.5.into()).unwrap();

    let circuit = h
        .to_trotter_circuit(1.0, 1, TrotterMode::FirstOrder)
        .unwrap();
    let matrix = circuit.to_matrix(None).unwrap();

    let c = |re: f64, im: f64| Complex64::new(re, im);

    // U = e^{-i * 0.5 * Z} = [[e^{-0.5i}, 0], [0, e^{0.5i}]]
    let expected = array![
        [c(0.5_f64.cos(), -0.5_f64.sin()), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.5_f64.cos(), 0.5_f64.sin())],
    ];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_trotter_matrix_equivalence_xx() {
    // H = 0.5 * XX
    let mut h = Hamiltonian::new(2);
    h.add_term("XX".parse().unwrap(), 0.5.into()).unwrap();

    let circuit = h
        .to_trotter_circuit(1.0, 1, TrotterMode::FirstOrder)
        .unwrap();
    let matrix = circuit.to_matrix(None).unwrap();

    let c = |re: f64, im: f64| Complex64::new(re, im);

    // U = e^{-i * 0.5 * XX} = cos(0.5)I - i sin(0.5)XX
    let cos_val = 0.5_f64.cos();
    let sin_val = 0.5_f64.sin();

    let expected = array![
        [c(cos_val, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, -sin_val)],
        [c(0.0, 0.0), c(cos_val, 0.0), c(0.0, -sin_val), c(0.0, 0.0)],
        [c(0.0, 0.0), c(0.0, -sin_val), c(cos_val, 0.0), c(0.0, 0.0)],
        [c(0.0, -sin_val), c(0.0, 0.0), c(0.0, 0.0), c(cos_val, 0.0)],
    ];

    assert_matrix_approx_eq(&matrix, &expected, 1e-10);
}

#[test]
fn test_evolution_circuit_exact_for_commuting_hamiltonian() {
    // ZZ and ZI commute → exact path should be taken (1 gate per term, no Trotter steps)
    let mut h = Hamiltonian::new(2);
    h.add_term("ZZ".parse::<PauliString>().unwrap(), 0.5.into())
        .unwrap();
    h.add_term("ZI".parse::<PauliString>().unwrap(), 0.3.into())
        .unwrap();

    // steps=1 is irrelevant for commuting H; mode is also irrelevant
    let circuit = h
        .to_evolution_circuit(1.0, 1, TrotterMode::FirstOrder)
        .unwrap();

    // Exact path: one pauli_evolution per term (ZZ → 1 gate, ZI → 1 gate = 2 RZ + 2 CNOT chain)
    // The number of ops is determined by the circuit structure; we just check it builds without error.
    assert_eq!(circuit.num_qubits(), 2);
}

#[test]
fn test_evolution_circuit_exact_matches_to_trotter_circuit_for_commuting() {
    // For commuting H, to_evolution_circuit should produce the same matrix as
    // to_trotter_circuit with 1 step (which for commuting H is also exact).
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse::<PauliString>().unwrap(), 0.7.into())
        .unwrap();

    let t = 0.4_f64;
    let exact_circ = h
        .to_evolution_circuit(t, 1, TrotterMode::FirstOrder)
        .unwrap();
    let trotter_circ = h.to_trotter_circuit(t, 1, TrotterMode::FirstOrder).unwrap();

    let m_exact = circuit_to_matrix(&exact_circ, None).unwrap();
    let m_trotter = circuit_to_matrix(&trotter_circ, None).unwrap();

    // Both should give the same 2×2 matrix
    assert_eq!(m_exact.shape(), m_trotter.shape());
    for (a, b) in m_exact.iter().zip(m_trotter.iter()) {
        assert_abs_diff_eq!(a.re, b.re, epsilon = 1e-12);
        assert_abs_diff_eq!(a.im, b.im, epsilon = 1e-12);
    }
}

#[test]
fn test_evolution_circuit_noncommuting_uses_trotter() {
    // X and Z do not commute → falls back to Trotter
    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse::<PauliString>().unwrap(), 1.0.into())
        .unwrap();
    h.add_term("Z".parse::<PauliString>().unwrap(), 1.0.into())
        .unwrap();

    let circuit_trotter = h
        .to_evolution_circuit(0.5, 10, TrotterMode::FirstOrder)
        .unwrap();
    let circuit_direct = h
        .to_trotter_circuit(0.5, 10, TrotterMode::FirstOrder)
        .unwrap();

    // Both should produce the same gate count since the same logic is used
    assert_eq!(
        circuit_trotter.operations().len(),
        circuit_direct.operations().len()
    );
}

#[test]
fn test_evolution_circuit_rejects_empty_hamiltonian() {
    let h = Hamiltonian::new(1);
    assert!(
        h.to_evolution_circuit(1.0, 1, TrotterMode::FirstOrder)
            .is_err()
    );
}

#[test]
fn test_evolution_circuit_rejects_zero_steps() {
    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse::<PauliString>().unwrap(), 1.0.into())
        .unwrap();
    assert!(
        h.to_evolution_circuit(1.0, 0, TrotterMode::FirstOrder)
            .is_err()
    );
}

#[test]
fn test_evolution_circuit_exact_matrix_correctness() {
    // H = 0.5*Z, t = π  →  e^{-i*0.5*π*Z} = [[e^{-iπ/2}, 0], [0, e^{iπ/2}]]
    //                                        = [[-i, 0], [0, i]]
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse::<PauliString>().unwrap(), 0.5.into())
        .unwrap();

    let circuit = h
        .to_evolution_circuit(PI, 1, TrotterMode::FirstOrder)
        .unwrap();
    let matrix = circuit_to_matrix(&circuit, None).unwrap();

    // Expected: diagonal [e^{-i π/2}, e^{i π/2}] = [-i, i]
    assert_abs_diff_eq!(matrix[[0, 0]].re, 0.0, epsilon = 1e-12);
    assert_abs_diff_eq!(matrix[[0, 0]].im, -1.0, epsilon = 1e-12);
    assert_abs_diff_eq!(matrix[[1, 1]].re, 0.0, epsilon = 1e-12);
    assert_abs_diff_eq!(matrix[[1, 1]].im, 1.0, epsilon = 1e-12);
    assert_abs_diff_eq!(matrix[[0, 1]].re, 0.0, epsilon = 1e-12);
    assert_abs_diff_eq!(matrix[[1, 0]].re, 0.0, epsilon = 1e-12);
}
