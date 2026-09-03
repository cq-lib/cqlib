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

//! Tests for Pauli operators and Pauli strings.

use crate::qis::pauli::{Pauli, PauliString, Phase};
use ndarray::arr2;
use num_complex::Complex64;

#[test]
fn test_phase_from_u8() {
    // Basic conversions
    assert_eq!(Phase::from(0), Phase::Plus);
    assert_eq!(Phase::from(1), Phase::I);
    assert_eq!(Phase::from(2), Phase::Minus);
    assert_eq!(Phase::from(3), Phase::MinusI);

    // Modulo 4 behavior
    assert_eq!(Phase::from(4), Phase::Plus); // 4 % 4 = 0
    assert_eq!(Phase::from(5), Phase::I); // 5 % 4 = 1
    assert_eq!(Phase::from(6), Phase::Minus); // 6 % 4 = 2
    assert_eq!(Phase::from(7), Phase::MinusI); // 7 % 4 = 3
    assert_eq!(Phase::from(100), Phase::Plus); // 100 % 4 = 0
}

#[test]
fn test_phase_addition() {
    // i^a * i^b = i^(a+b)
    assert_eq!(Phase::Plus + Phase::Plus, Phase::Plus); // 1 * 1 = 1
    assert_eq!(Phase::Plus + Phase::I, Phase::I); // 1 * i = i
    assert_eq!(Phase::I + Phase::I, Phase::Minus); // i * i = -1
    assert_eq!(Phase::I + Phase::Minus, Phase::MinusI); // i * -1 = -i
    assert_eq!(Phase::Minus + Phase::Minus, Phase::Plus); // -1 * -1 = 1
    assert_eq!(Phase::MinusI + Phase::MinusI, Phase::Minus); // -i * -i = -1

    // Wrap around
    assert_eq!(Phase::MinusI + Phase::I, Phase::Plus); // -i * i = 1
}

#[test]
fn test_phase_add_assign() {
    let mut p = Phase::I;
    p += Phase::I;
    assert_eq!(p, Phase::Minus);

    p += 2; // Add u8
    assert_eq!(p, Phase::Plus);
}

#[test]
fn test_phase_multiplication() {
    // Phase multiplication is same as addition
    assert_eq!(Phase::I * Phase::I, Phase::Minus);
    assert_eq!(Phase::Minus * Phase::MinusI, Phase::I);
}

#[test]
fn test_phase_to_complex() {
    assert_eq!(Phase::Plus.to_complex(), Complex64::new(1.0, 0.0));
    assert_eq!(Phase::I.to_complex(), Complex64::new(0.0, 1.0));
    assert_eq!(Phase::Minus.to_complex(), Complex64::new(-1.0, 0.0));
    assert_eq!(Phase::MinusI.to_complex(), Complex64::new(0.0, -1.0));
}

#[test]
fn test_pauli_display() {
    assert_eq!(format!("{}", Pauli::I), "I");
    assert_eq!(format!("{}", Pauli::X), "X");
    assert_eq!(format!("{}", Pauli::Y), "Y");
    assert_eq!(format!("{}", Pauli::Z), "Z");
}

#[test]
fn test_pauli_to_symplectic() {
    assert_eq!(Pauli::I.to_symplectic(), (0, 0));
    assert_eq!(Pauli::X.to_symplectic(), (1, 0));
    assert_eq!(Pauli::Y.to_symplectic(), (1, 1));
    assert_eq!(Pauli::Z.to_symplectic(), (0, 1));
}

#[test]
fn test_pauli_to_matrix_identity() {
    let mat = Pauli::I.to_matrix();
    let one = Complex64::new(1.0, 0.0);
    let zero = Complex64::new(0.0, 0.0);

    assert_eq!(mat[[0, 0]], one);
    assert_eq!(mat[[0, 1]], zero);
    assert_eq!(mat[[1, 0]], zero);
    assert_eq!(mat[[1, 1]], one);
}

#[test]
fn test_pauli_to_matrix_x() {
    let mat = Pauli::X.to_matrix();
    let one = Complex64::new(1.0, 0.0);
    let zero = Complex64::new(0.0, 0.0);

    assert_eq!(mat[[0, 0]], zero);
    assert_eq!(mat[[0, 1]], one);
    assert_eq!(mat[[1, 0]], one);
    assert_eq!(mat[[1, 1]], zero);
}

#[test]
fn test_pauli_to_matrix_y() {
    let mat = Pauli::Y.to_matrix();
    let zero = Complex64::new(0.0, 0.0);
    let i = Complex64::new(0.0, 1.0);
    let neg_i = Complex64::new(0.0, -1.0);

    assert_eq!(mat[[0, 0]], zero);
    assert_eq!(mat[[0, 1]], neg_i);
    assert_eq!(mat[[1, 0]], i);
    assert_eq!(mat[[1, 1]], zero);
}

#[test]
fn test_pauli_to_matrix_z() {
    let mat = Pauli::Z.to_matrix();
    let one = Complex64::new(1.0, 0.0);
    let neg_one = Complex64::new(-1.0, 0.0);

    assert_eq!(mat[[0, 0]], one);
    assert_eq!(mat[[0, 1]], Complex64::new(0.0, 0.0));
    assert_eq!(mat[[1, 0]], Complex64::new(0.0, 0.0));
    assert_eq!(mat[[1, 1]], neg_one);
}

#[test]
fn test_pauli_mul_with_phase_identity() {
    // I * P = P with phase 1
    assert_eq!(Pauli::I.mul_with_phase(Pauli::X), (Pauli::X, Phase::Plus));
    assert_eq!(Pauli::I.mul_with_phase(Pauli::Y), (Pauli::Y, Phase::Plus));
    assert_eq!(Pauli::I.mul_with_phase(Pauli::Z), (Pauli::Z, Phase::Plus));
    assert_eq!(Pauli::X.mul_with_phase(Pauli::I), (Pauli::X, Phase::Plus));
}

#[test]
fn test_pauli_mul_with_phase_square() {
    // P^2 = I
    assert_eq!(Pauli::X.mul_with_phase(Pauli::X), (Pauli::I, Phase::Plus));
    assert_eq!(Pauli::Y.mul_with_phase(Pauli::Y), (Pauli::I, Phase::Plus));
    assert_eq!(Pauli::Z.mul_with_phase(Pauli::Z), (Pauli::I, Phase::Plus));
}

#[test]
fn test_pauli_mul_with_phase_xyz_cyclic() {
    // XY = iZ, YZ = iX, ZX = iY
    assert_eq!(Pauli::X.mul_with_phase(Pauli::Y), (Pauli::Z, Phase::I));
    assert_eq!(Pauli::Y.mul_with_phase(Pauli::Z), (Pauli::X, Phase::I));
    assert_eq!(Pauli::Z.mul_with_phase(Pauli::X), (Pauli::Y, Phase::I));
}

#[test]
fn test_pauli_mul_with_phase_anticommutation() {
    // YX = -iZ, ZY = -iX, XZ = -iY (reverse order gives -i)
    assert_eq!(Pauli::Y.mul_with_phase(Pauli::X), (Pauli::Z, Phase::MinusI));
    assert_eq!(Pauli::Z.mul_with_phase(Pauli::Y), (Pauli::X, Phase::MinusI));
    assert_eq!(Pauli::X.mul_with_phase(Pauli::Z), (Pauli::Y, Phase::MinusI));
}

#[test]
fn test_pauli_string_new() {
    let ps = PauliString::new(3);
    assert_eq!(ps.num_qubits, 3);
    assert_eq!(ps.phase, Phase::Plus);
    assert_eq!(ps.to_string(), "+III");
}

#[test]
fn test_pauli_string_set_pauli() {
    let mut ps = PauliString::new(3);
    ps.set_pauli(0, Pauli::X);
    assert_eq!(ps.to_string(), "+IIX");

    ps.set_pauli(2, Pauli::Z);
    assert_eq!(ps.to_string(), "+ZIX");

    ps.set_pauli(1, Pauli::Y);
    assert_eq!(ps.to_string(), "+ZYX");
}

#[test]
#[should_panic(expected = "Index 3 out of bounds for 3 qubits")]
fn test_pauli_string_set_pauli_out_of_bounds() {
    let mut ps = PauliString::new(3);
    ps.set_pauli(3, Pauli::X); // Should panic
}

#[test]
fn test_pauli_string_display_phase() {
    let mut ps = PauliString::new(2);
    assert_eq!(ps.to_string(), "+II");

    ps.phase = Phase::I;
    assert_eq!(ps.to_string(), "+iII");

    ps.phase = Phase::Minus;
    assert_eq!(ps.to_string(), "-II");

    ps.phase = Phase::MinusI;
    assert_eq!(ps.to_string(), "-iII");
}

#[test]
fn test_pauli_string_commutes_with_same_operator() {
    // Same operators always commute
    let mut p1 = PauliString::new(2);
    p1.set_pauli(0, Pauli::X);
    p1.set_pauli(1, Pauli::Z);

    let mut p2 = PauliString::new(2);
    p2.set_pauli(0, Pauli::X);
    p2.set_pauli(1, Pauli::Z);

    assert!(p1.commutes_with(&p2));
    assert!(p2.commutes_with(&p1));
}

#[test]
fn test_pauli_string_commutes_with_identity() {
    // Everything commutes with identity
    let mut p1 = PauliString::new(2);
    p1.set_pauli(0, Pauli::X);
    p1.set_pauli(1, Pauli::Z);

    let p2 = PauliString::new(2); // All I

    assert!(p1.commutes_with(&p2));
    assert!(p2.commutes_with(&p1));
}

#[test]
fn test_pauli_string_commutes_with_anticommuting() {
    // X and Z anticommute on same qubit
    let mut p1 = PauliString::new(1);
    p1.set_pauli(0, Pauli::X);

    let mut p2 = PauliString::new(1);
    p2.set_pauli(0, Pauli::Z);

    assert!(!p1.commutes_with(&p2));
}

#[test]
fn test_pauli_string_commutes_with_different_qubits() {
    // Operators on different qubits always commute
    let mut p1 = PauliString::new(2);
    p1.set_pauli(0, Pauli::X);

    let mut p2 = PauliString::new(2);
    p2.set_pauli(1, Pauli::Z);

    assert!(p1.commutes_with(&p2));
}

#[test]
fn test_pauli_string_commutes_with_complex_case() {
    // XX commutes with ZZ (anticommutes on both qubits, even count)
    let mut p1 = PauliString::new(2);
    p1.set_pauli(0, Pauli::X);
    p1.set_pauli(1, Pauli::X);

    let mut p2 = PauliString::new(2);
    p2.set_pauli(0, Pauli::Z);
    p2.set_pauli(1, Pauli::Z);

    // XZ anticommutes on qubit 0, XZ anticommutes on qubit 1
    // Total 2 anticommutations -> even -> commutes
    assert!(p1.commutes_with(&p2));
}

#[test]
#[should_panic(expected = "assertion")]
fn test_pauli_string_commutes_with_different_size() {
    let p1 = PauliString::new(2);
    let p2 = PauliString::new(3);
    p1.commutes_with(&p2); // Should panic
}

#[test]
fn test_pauli_string_mul_identity() {
    // Multiply by identity doesn't change operator
    let mut p1 = PauliString::new(2);
    p1.set_pauli(0, Pauli::X); // qubit 0 (rightmost)

    let p2 = PauliString::new(2); // Identity

    let result = &p1 * &p2;
    assert_eq!(result.to_string(), "+IX"); // X on qubit 0 is rightmost
}

#[test]
fn test_pauli_string_mul_same_operator() {
    // X * X = I (with phase 1)
    let mut p1 = PauliString::new(1);
    p1.set_pauli(0, Pauli::X);

    let result = &p1 * &p1;
    assert_eq!(result.to_string(), "+I");
}

#[test]
fn test_pauli_string_mul_with_phase() {
    // X * Z = -iY
    let mut p1 = PauliString::new(1);
    p1.set_pauli(0, Pauli::X);

    let mut p2 = PauliString::new(1);
    p2.set_pauli(0, Pauli::Z);

    let result = &p1 * &p2;
    assert_eq!(result.to_string(), "-iY");
}

#[test]
fn test_pauli_string_mul_multi_qubit() {
    // (X ⊗ I) * (I ⊗ Z) = X ⊗ Z
    let mut p1 = PauliString::new(2);
    p1.set_pauli(0, Pauli::X);

    let mut p2 = PauliString::new(2);
    p2.set_pauli(1, Pauli::Z);

    let result = &p1 * &p2;
    assert_eq!(result.to_string(), "+ZX");
}

#[test]
fn test_pauli_string_mul_assign() {
    let mut p1 = PauliString::new(1);
    p1.set_pauli(0, Pauli::X);

    let mut p2 = PauliString::new(1);
    p2.set_pauli(0, Pauli::Y);

    p1 *= &p2; // X * Y = iZ
    assert_eq!(p1.to_string(), "+iZ");
}

#[test]
fn test_pauli_string_mul_phase_accumulation() {
    // Test phase accumulation across multiple operations
    let mut p1 = PauliString::new(1);
    p1.set_pauli(0, Pauli::X);
    p1.phase = Phase::I; // Start with i

    let mut p2 = PauliString::new(1);
    p2.set_pauli(0, Pauli::Y);
    // X * Y = iZ, so total phase = i * i = -1

    let result = &p1 * &p2;
    assert_eq!(result.to_string(), "-Z");
}

#[test]
fn test_pauli_string_mul_complex_product() {
    // (X ⊗ Y) * (Y ⊗ X) = (XY) ⊗ (YX) = (iZ) ⊗ (-iZ) = Z ⊗ Z
    // Phase: i * (-i) = 1
    let mut p1 = PauliString::new(2);
    p1.set_pauli(0, Pauli::X);
    p1.set_pauli(1, Pauli::Y);

    let mut p2 = PauliString::new(2);
    p2.set_pauli(0, Pauli::Y);
    p2.set_pauli(1, Pauli::X);

    let result = &p1 * &p2;
    assert_eq!(result.to_string(), "+ZZ");
}

#[test]
#[should_panic(expected = "Qubit count mismatch")]
fn test_pauli_string_mul_different_size() {
    let p1 = PauliString::new(2);
    let p2 = PauliString::new(3);
    let _ = &p1 * &p2; // Should panic
}

#[test]
fn test_pauli_string_large_system() {
    // Test with a larger number of qubits
    let n = 1000;
    let mut ps = PauliString::new(n);
    ps.set_pauli(0, Pauli::X);
    ps.set_pauli(n - 1, Pauli::Z);

    assert_eq!(ps.num_qubits, n);
    // Check string representation (first and last characters)
    let s = ps.to_string();
    assert!(s.starts_with("+Z"));
    assert!(s.ends_with('X'));
}

#[test]
fn test_pauli_string_all_operators() {
    let mut ps = PauliString::new(4);
    ps.set_pauli(0, Pauli::I);
    ps.set_pauli(1, Pauli::X);
    ps.set_pauli(2, Pauli::Y);
    ps.set_pauli(3, Pauli::Z);

    assert_eq!(ps.to_string(), "+ZYXI");
}

#[test]
fn test_cyclic_property_xyz() {
    // Verify X -> Y -> Z -> X cyclic property via multiplication
    let xy = Pauli::X.mul_with_phase(Pauli::Y);
    assert_eq!(xy.0, Pauli::Z);

    let yz = Pauli::Y.mul_with_phase(Pauli::Z);
    assert_eq!(yz.0, Pauli::X);

    let zx = Pauli::Z.mul_with_phase(Pauli::X);
    assert_eq!(zx.0, Pauli::Y);

    // Check phases form the correct pattern
    assert_eq!(xy.1, Phase::I); // XY = iZ
    assert_eq!(yz.1, Phase::I); // YZ = iX
    assert_eq!(zx.1, Phase::I); // ZX = iY
}

#[test]
fn test_pauli_hermitian_property() {
    // Pauli matrices are Hermitian: P^† = P
    // This means P * P = I (since P is unitary and Hermitian)
    for p in [Pauli::I, Pauli::X, Pauli::Y, Pauli::Z] {
        let (result, phase) = p.mul_with_phase(p);
        assert_eq!(result, Pauli::I, "{:?} squared should be I", p);
        assert_eq!(phase, Phase::Plus, "{:?} squared should have phase +1", p);
    }
}

#[test]
fn test_trace_property() {
    // tr(Pauli) = 0 for X, Y, Z and tr(I) = 2
    let zero = Complex64::new(0.0, 0.0);
    let two = Complex64::new(2.0, 0.0);

    assert_eq!(
        Pauli::I.to_matrix()[[0, 0]] + Pauli::I.to_matrix()[[1, 1]],
        two
    );
    assert_eq!(
        Pauli::X.to_matrix()[[0, 0]] + Pauli::X.to_matrix()[[1, 1]],
        zero
    );
    assert_eq!(
        Pauli::Y.to_matrix()[[0, 0]] + Pauli::Y.to_matrix()[[1, 1]],
        zero
    );
    assert_eq!(
        Pauli::Z.to_matrix()[[0, 0]] + Pauli::Z.to_matrix()[[1, 1]],
        zero
    );
}

#[test]
fn test_commutation_relation_xyz() {
    // [X, Y] = 2iZ (commutator)
    // This means XY - YX = 2iZ
    let xy = Pauli::X.mul_with_phase(Pauli::Y);
    let yx = Pauli::Y.mul_with_phase(Pauli::X);

    assert_eq!(xy.0, Pauli::Z);
    assert_eq!(yx.0, Pauli::Z);

    // XY = iZ, YX = -iZ
    assert_eq!(xy.1, Phase::I);
    assert_eq!(yx.1, Phase::MinusI);

    // Phase difference should be i - (-i) = 2i, which corresponds to XY = -YX * (-1)
    // Actually: iZ - (-iZ) = 2iZ, so XY and YX have opposite signs
}

use std::collections::HashMap;

#[test]
fn test_expectation_z_on_zero() {
    // Z on qubit 0, state |0⟩ (prob 1.0)
    let mut ps = PauliString::new(1);
    ps.set_pauli(0, Pauli::Z);

    let mut probs = HashMap::new();
    probs.insert("0".to_string(), 1.0);

    let exp = ps.expectation(&probs).unwrap();
    assert!(
        (exp - 1.0).abs() < 1e-10,
        "⟨0|Z|0⟩ should be 1, got {}",
        exp
    );
}

#[test]
fn test_expectation_z_on_one() {
    // Z on qubit 0, state |1⟩ (prob 1.0)
    let mut ps = PauliString::new(1);
    ps.set_pauli(0, Pauli::Z);

    let mut probs = HashMap::new();
    probs.insert("1".to_string(), 1.0);

    let exp = ps.expectation(&probs).unwrap();
    assert!(
        (exp + 1.0).abs() < 1e-10,
        "⟨1|Z|1⟩ should be -1, got {}",
        exp
    );
}

#[test]
fn test_expectation_z_mixed() {
    // Z on qubit 0, mixed state: 50% |0⟩, 50% |1⟩
    let mut ps = PauliString::new(1);
    ps.set_pauli(0, Pauli::Z);

    let mut probs = HashMap::new();
    probs.insert("0".to_string(), 0.5);
    probs.insert("1".to_string(), 0.5);

    let exp = ps.expectation(&probs).unwrap();
    assert!(
        exp.abs() < 1e-10,
        "⟨Z⟩ for maximally mixed state should be 0, got {}",
        exp
    );
}

#[test]
fn test_expectation_x_is_zero() {
    // X on qubit 0 - expectation is always 0 for computational basis states
    let mut ps = PauliString::new(1);
    ps.set_pauli(0, Pauli::X);

    let mut probs = HashMap::new();
    probs.insert("0".to_string(), 0.3);
    probs.insert("1".to_string(), 0.7);

    let exp = ps.expectation(&probs).unwrap();
    assert!(
        exp.abs() < 1e-10,
        "⟨X⟩ for computational basis states should be 0, got {}",
        exp
    );
}

#[test]
fn test_expectation_y_is_zero() {
    // Y on qubit 0 - expectation is always 0 for computational basis states
    let mut ps = PauliString::new(1);
    ps.set_pauli(0, Pauli::Y);

    let mut probs = HashMap::new();
    probs.insert("0".to_string(), 0.5);
    probs.insert("1".to_string(), 0.5);

    let exp = ps.expectation(&probs).unwrap();
    assert!(
        exp.abs() < 1e-10,
        "⟨Y⟩ for computational basis states should be 0, got {}",
        exp
    );
}

#[test]
fn test_expectation_two_qubit_zz() {
    // Z⊗Z on |00⟩ and |11⟩ (Bell state probabilities)
    let mut ps = PauliString::new(2);
    ps.set_pauli(0, Pauli::Z);
    ps.set_pauli(1, Pauli::Z);

    let mut probs = HashMap::new();
    probs.insert("00".to_string(), 0.5); // |00⟩: qubit1=0, qubit0=0, ZZ eigenvalue = 1
    probs.insert("11".to_string(), 0.5); // |11⟩: qubit1=1, qubit0=1, ZZ eigenvalue = (-1)×(-1) = 1

    let exp = ps.expectation(&probs).unwrap();
    assert!(
        (exp - 1.0).abs() < 1e-10,
        "⟨ZZ⟩ for (|00⟩+|11⟩)/√2 should be 1, got {}",
        exp
    );
}

#[test]
fn test_expectation_two_qubit_zz_orthogonal() {
    // Z⊗Z on |01⟩ and |10⟩
    let mut ps = PauliString::new(2);
    ps.set_pauli(0, Pauli::Z);
    ps.set_pauli(1, Pauli::Z);

    let mut probs = HashMap::new();
    probs.insert("01".to_string(), 0.5); // |01⟩: qubit1=0, qubit0=1, ZZ eigenvalue = 1×(-1) = -1
    probs.insert("10".to_string(), 0.5); // |10⟩: qubit1=1, qubit0=0, ZZ eigenvalue = (-1)×1 = -1

    let exp = ps.expectation(&probs).unwrap();
    assert!(
        (exp + 1.0).abs() < 1e-10,
        "⟨ZZ⟩ for (|01⟩+|10⟩)/√2 should be -1, got {}",
        exp
    );
}

#[test]
fn test_expectation_identity() {
    // Identity on any state should give 1
    let ps = PauliString::new(2); // All I

    let mut probs = HashMap::new();
    probs.insert("00".to_string(), 0.25);
    probs.insert("01".to_string(), 0.25);
    probs.insert("10".to_string(), 0.25);
    probs.insert("11".to_string(), 0.25);

    let exp = ps.expectation(&probs).unwrap();
    assert!(
        (exp - 1.0).abs() < 1e-10,
        "⟨I⟩ should always be 1, got {}",
        exp
    );
}

#[test]
fn test_expectation_with_global_phase() {
    // -Z (global phase Minus)
    let mut ps = PauliString::new(1);
    ps.set_pauli(0, Pauli::Z);
    ps.phase = Phase::Minus; // Multiply by -1

    let mut probs = HashMap::new();
    probs.insert("0".to_string(), 1.0);

    let exp = ps.expectation(&probs).unwrap();
    assert!(
        (exp + 1.0).abs() < 1e-10,
        "⟨-Z⟩ for |0⟩ should be -1, got {}",
        exp
    );
}

#[test]
fn test_expectation_partial_probabilities() {
    // Only some states in the distribution (others implicitly have prob 0)
    let mut ps = PauliString::new(2);
    ps.set_pauli(0, Pauli::Z); // Only Z on qubit 0

    let mut probs = HashMap::new();
    // Only include states with qubit 0 = 0, expect ⟨Z⟩ = 1
    probs.insert("00".to_string(), 0.5); // qubit0=0
    probs.insert("10".to_string(), 0.5); // qubit0=0

    let exp = ps.expectation(&probs).unwrap();
    assert!(
        (exp - 1.0).abs() < 1e-10,
        "⟨Z⊗I⟩ when qubit 0 is always 0 should be 1, got {}",
        exp
    );
}

#[test]
fn phase_display_has_no_debug_quotes() {
    for (phase, expected) in [
        (Phase::Plus, "1"),
        (Phase::I, "i"),
        (Phase::Minus, "-1"),
        (Phase::MinusI, "-i"),
    ] {
        let s = phase.to_string();
        assert!(
            !s.contains('\"'),
            "Phase::Display for {:?} produced quoted output: {:?}",
            phase,
            s
        );
        assert_eq!(
            s, expected,
            "Phase::Display for {:?}: expected {:?}, got {:?}",
            phase, expected, s
        );
    }
}

#[test]
fn phase_display_matches_expected_symbols() {
    assert_eq!(Phase::Plus.to_string(), "1");
    assert_eq!(Phase::I.to_string(), "i");
    assert_eq!(Phase::Minus.to_string(), "-1");
    assert_eq!(Phase::MinusI.to_string(), "-i");
}

#[test]
fn pauli_try_from_valid_characters() {
    use std::convert::TryFrom;
    assert_eq!(Pauli::try_from('I').unwrap(), Pauli::I);
    assert_eq!(Pauli::try_from('X').unwrap(), Pauli::X);
    assert_eq!(Pauli::try_from('Y').unwrap(), Pauli::Y);
    assert_eq!(Pauli::try_from('Z').unwrap(), Pauli::Z);
}

#[test]
fn pauli_try_from_invalid_characters() {
    use std::convert::TryFrom;
    // 'A' is not a Pauli character
    assert!(Pauli::try_from('A').is_err());
    // Lowercase 'x' (strict uppercase only)
    assert!(Pauli::try_from('x').is_err());
    // Number
    assert!(Pauli::try_from('0').is_err());
    // Whitespace
    assert!(Pauli::try_from(' ').is_err());
}

#[test]
fn pauli_try_from_does_not_panic() {
    use std::convert::TryFrom;
    // This must not panic, just return Err
    let _ = Pauli::try_from('\0');
    let _ = Pauli::try_from('\x7f');
    let _ = Pauli::try_from('!');
}

#[test]
fn pauli_from_str_valid() {
    assert_eq!("I".parse::<Pauli>().unwrap(), Pauli::I);
    assert_eq!("X".parse::<Pauli>().unwrap(), Pauli::X);
    assert_eq!("Y".parse::<Pauli>().unwrap(), Pauli::Y);
    assert_eq!("Z".parse::<Pauli>().unwrap(), Pauli::Z);
}

#[test]
fn pauli_from_str_invalid() {
    assert!("".parse::<Pauli>().is_err());
    assert!("A".parse::<Pauli>().is_err());
    assert!("x".parse::<Pauli>().is_err());
    assert!("XY".parse::<Pauli>().is_err());
    assert!("II".parse::<Pauli>().is_err());
}

#[test]
fn try_get_pauli_rejects_out_of_bounds_index() {
    let ps = PauliString::new(2);
    assert!(ps.try_get_pauli(0).is_ok());
    assert!(ps.try_get_pauli(1).is_ok());
    assert!(ps.try_get_pauli(2).is_err());
    assert!(ps.try_get_pauli(100).is_err());
}

#[test]
fn try_get_pauli_zero_qubits() {
    let ps = PauliString::new(0);
    assert!(ps.try_get_pauli(0).is_err());
}

#[test]
fn try_set_pauli_rejects_out_of_bounds_index() {
    let mut ps = PauliString::new(2);
    assert!(ps.try_set_pauli(0, Pauli::X).is_ok());
    assert!(ps.try_set_pauli(1, Pauli::Z).is_ok());
    assert!(ps.try_set_pauli(2, Pauli::X).is_err());
    assert!(ps.try_set_pauli(100, Pauli::I).is_err());
}

#[test]
fn try_set_pauli_preserves_state_on_error() {
    let mut ps = PauliString::new(2);
    ps.try_set_pauli(0, Pauli::X).unwrap();
    let result = ps.try_set_pauli(5, Pauli::Z);
    assert!(result.is_err());
    assert_eq!(ps.try_get_pauli(0).unwrap(), Pauli::X);
    assert_eq!(ps.try_get_pauli(1).unwrap(), Pauli::I);
}

#[test]
fn try_get_pauli_returns_correct_operator() {
    let mut ps = PauliString::new(3);
    ps.try_set_pauli(0, Pauli::X).unwrap();
    ps.try_set_pauli(1, Pauli::Y).unwrap();
    ps.try_set_pauli(2, Pauli::Z).unwrap();
    assert_eq!(ps.try_get_pauli(0).unwrap(), Pauli::X);
    assert_eq!(ps.try_get_pauli(1).unwrap(), Pauli::Y);
    assert_eq!(ps.try_get_pauli(2).unwrap(), Pauli::Z);
}

#[test]
fn pauli_string_to_matrix_uses_little_endian_tensor_order() {
    let pauli: PauliString = "ZX".parse().unwrap();
    let expected = arr2(&[
        [
            Complex64::new(0.0, 0.0),
            Complex64::new(1.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
        ],
        [
            Complex64::new(1.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
        ],
        [
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(-1.0, 0.0),
        ],
        [
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(-1.0, 0.0),
            Complex64::new(0.0, 0.0),
        ],
    ]);

    assert_eq!(pauli.to_matrix(), expected);
}

#[test]
fn pauli_string_to_matrix_includes_each_global_phase() {
    let base: PauliString = "Y".parse().unwrap();
    let base_matrix = Pauli::Y.to_matrix();

    for phase in [Phase::Plus, Phase::I, Phase::Minus, Phase::MinusI] {
        let mut pauli = base.clone();
        pauli.phase = phase;
        assert_eq!(
            pauli.to_matrix(),
            base_matrix.mapv(|value| value * phase.to_complex())
        );
    }
}

#[test]
fn zero_qubit_pauli_string_to_matrix_is_its_phase() {
    let mut pauli = PauliString::new(0);
    pauli.phase = Phase::MinusI;

    assert_eq!(pauli.to_matrix(), arr2(&[[Complex64::new(0.0, -1.0)]]));
}

#[test]
fn pauli_iterator_follows_ascending_qubit_indices() {
    let pauli: PauliString = "XYZI".parse().unwrap();
    let expected = (0..pauli.num_qubits)
        .map(|index| pauli.get_pauli(index))
        .collect::<Vec<_>>();

    assert_eq!(pauli.iter().collect::<Vec<_>>(), expected);
    assert_eq!((&pauli).into_iter().collect::<Vec<_>>(), expected);
}
