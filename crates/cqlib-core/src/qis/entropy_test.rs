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

//! Tests for quantum entropy and entanglement measures.

use crate::qis::entropy::{
    concurrence, entanglement_entropy_pure, entanglement_of_formation, linear_entropy, negativity,
    renyi_entropy,
};
use crate::qis::state::{DensityMatrix, Statevector};
use num_complex::Complex64;

// Tolerance for floating-point comparisons
const EPSILON: f64 = 1e-10;

fn werner_state(p: f64) -> DensityMatrix {
    let mut bell = DensityMatrix::new(2);
    bell.apply_h(0).unwrap();
    bell.apply_cx(0, 1).unwrap();

    let dim = 4;
    let mut data: Vec<_> = bell.data().iter().map(|value| *value * p).collect();
    for i in 0..dim {
        data[i * dim + i] += Complex64::new((1.0 - p) / dim as f64, 0.0);
    }

    DensityMatrix::from_density_matrix_state(2, data).unwrap()
}

#[test]
fn test_linear_entropy_pure_state() {
    // Pure state |0⟩ should have linear entropy = 0
    let dm = DensityMatrix::new(1);
    let s_l = linear_entropy(&dm).unwrap();
    assert!(
        s_l.abs() < EPSILON,
        "Pure state linear entropy should be 0, got {}",
        s_l
    );
}

#[test]
fn test_linear_entropy_bell_state() {
    // Bell state |Φ+⟩ is pure, linear entropy = 0
    let mut dm = DensityMatrix::new(2);
    dm.apply_h(0).unwrap();
    dm.apply_cx(0, 1).unwrap();
    let s_l = linear_entropy(&dm).unwrap();
    assert!(
        s_l.abs() < EPSILON,
        "Bell state linear entropy should be 0, got {}",
        s_l
    );
}

#[test]
fn test_linear_entropy_maximally_mixed() {
    // Maximally mixed state: ρ = I/2^N
    // For N qubits: Tr(ρ²) = 1/2^N, so S_L = 1 - 1/2^N
    let n = 2;
    let dim = 1 << n;
    let mut data = vec![Complex64::new(0.0, 0.0); dim * dim];
    for i in 0..dim {
        data[i * dim + i] = Complex64::new(1.0 / dim as f64, 0.0);
    }
    let dm = DensityMatrix::from_density_matrix_state(n, data).unwrap();
    let s_l = linear_entropy(&dm).unwrap();
    let expected = 1.0 - 1.0 / (dim as f64);
    assert!(
        (s_l - expected).abs() < EPSILON,
        "Maximally mixed state linear entropy should be {}, got {}",
        expected,
        s_l
    );
}

#[test]
fn test_linear_entropy_single_qubit_mixed() {
    // Single qubit mixed state: ρ = 0.75|0⟩⟨0| + 0.25|1⟩⟨1|
    // Tr(ρ²) = 0.75² + 0.25² = 0.625, so S_L = 0.375
    let mut data = vec![Complex64::new(0.0, 0.0); 4];
    data[0] = Complex64::new(0.75, 0.0); // |0⟩⟨0|
    data[3] = Complex64::new(0.25, 0.0); // |1⟩⟨1|
    let dm = DensityMatrix::from_density_matrix_state(1, data).unwrap();
    let s_l = linear_entropy(&dm).unwrap();
    let expected = 1.0 - (0.75f64.powi(2) + 0.25f64.powi(2));
    assert!(
        (s_l - expected).abs() < EPSILON,
        "Mixed state linear entropy should be {}, got {}",
        expected,
        s_l
    );
}
#[test]
fn test_renyi_entropy_alpha_positive_check() {
    // α <= 0 should return error
    let dm = DensityMatrix::new(1);
    let result = renyi_entropy(&dm, -1.0);
    assert!(
        result.is_err(),
        "Rényi entropy with α <= 0 should return error"
    );

    let result = renyi_entropy(&dm, 0.0);
    assert!(
        result.is_err(),
        "Rényi entropy with α = 0 should return error"
    );
}

#[test]
fn test_renyi_entropy_pure_state() {
    // For pure state, all Rényi entropies should be 0
    let dm = DensityMatrix::new(2);

    // α = 2 (collision entropy)
    let s2 = renyi_entropy(&dm, 2.0).unwrap();
    assert!(
        s2.abs() < EPSILON,
        "Collision entropy of pure state should be 0, got {}",
        s2
    );

    // α = 0.5
    let s_half = renyi_entropy(&dm, 0.5).unwrap();
    assert!(
        s_half.abs() < EPSILON,
        "Rényi entropy (α=0.5) of pure state should be 0, got {}",
        s_half
    );

    // Large α
    let s_large = renyi_entropy(&dm, 10.0).unwrap();
    assert!(
        s_large.abs() < EPSILON,
        "Rényi entropy (α=10) of pure state should be 0, got {}",
        s_large
    );
}

#[test]
fn test_renyi_entropy_graceful_degradation() {
    // When α is very close to 1, should smoothly degrade to Von Neumann entropy
    let mut dm = DensityMatrix::new(1);
    dm.apply_h(0).unwrap(); // |+⟩ state

    // Von Neumann entropy should be 0 for pure state
    let vn_entropy = crate::qis::metrics::entropy(&dm).unwrap();

    // α = 1.0 + EPSILON should give nearly identical result
    let alpha = 1.0 + f64::EPSILON;
    let renyi = renyi_entropy(&dm, alpha).unwrap();

    assert!(
        (renyi - vn_entropy).abs() < EPSILON,
        "Rényi entropy near α=1 should match Von Neumann entropy: {} vs {}",
        renyi,
        vn_entropy
    );
}

#[test]
fn test_renyi_entropy_maximally_mixed() {
    // For maximally mixed state of N qubits: all eigenvalues = 1/2^N
    // Rényi entropy: S_α = log2(2^N) = N (independent of α)
    let n = 2;
    let dim = 1 << n;
    let mut data = vec![Complex64::new(0.0, 0.0); dim * dim];
    for i in 0..dim {
        data[i * dim + i] = Complex64::new(1.0 / dim as f64, 0.0);
    }
    let dm = DensityMatrix::from_density_matrix_state(n, data).unwrap();

    // Test different α values - all should give N = 2
    for alpha in [0.5, 2.0, 5.0] {
        let s_alpha = renyi_entropy(&dm, alpha).unwrap();
        assert!(
            (s_alpha - n as f64).abs() < 1e-6,
            "Rényi entropy (α={}) of maximally mixed state should be {}, got {}",
            alpha,
            n,
            s_alpha
        );
    }
}

#[test]
fn test_renyi_entropy_partially_mixed() {
    // Werner-like state: ρ = p|Φ+⟩⟨Φ+| + (1-p)I/4
    // For p = 0.5, eigenvalues are: (3/8, 1/8, 1/8, 1/8)
    let p = 0.5;
    let dm = werner_state(p);

    // Eigenvalues of Werner state with p=0.5: (5/8, 1/8, 1/8, 1/8) = (0.625, 0.125, 0.125, 0.125)
    // For α = 2: Tr(ρ²) = (5/8)² + 3*(1/8)² = 25/64 + 3/64 = 28/64 = 7/16
    // S_2 = -log2(7/16) = log2(16/7)
    let s2 = renyi_entropy(&dm, 2.0).unwrap();
    let expected_s2 = (16.0f64 / 7.0).log2();
    assert!(
        (s2 - expected_s2).abs() < 1e-6,
        "Rényi entropy (α=2) should be {}, got {}",
        expected_s2,
        s2
    );
}

#[test]
fn test_entanglement_entropy_empty_subsys() {
    // Empty subsystem should return error
    let sv = Statevector::new(2);
    let result = entanglement_entropy_pure(&sv, &[]);
    assert!(result.is_err(), "Empty subsystem should return error");
}

#[test]
fn test_entanglement_entropy_full_subsys() {
    // Subsystem containing all qubits should return error
    let sv = Statevector::new(2);
    let result = entanglement_entropy_pure(&sv, &[0, 1]);
    assert!(result.is_err(), "Full subsystem should return error");
}

#[test]
fn test_entanglement_entropy_duplicate_indices() {
    // Duplicate indices should return error
    let sv = Statevector::new(2);
    let result = entanglement_entropy_pure(&sv, &[0, 0]);
    assert!(result.is_err(), "Duplicate indices should return error");
}

#[test]
fn test_entanglement_entropy_out_of_bounds() {
    // Out of bounds index should return error
    let sv = Statevector::new(2);
    let result = entanglement_entropy_pure(&sv, &[10]);
    assert!(result.is_err(), "Out of bounds index should return error");
}

#[test]
fn test_entanglement_entropy_bell_state() {
    // Bell state |Φ+⟩ = (|00⟩ + |11⟩)/√2
    // Reduced density matrix for either qubit is maximally mixed
    // So entanglement entropy = 1.0
    let mut sv = Statevector::new(2);
    sv.apply_h(0).unwrap();
    sv.apply_cx(0, 1).unwrap();

    // Test subsystem A = [0]
    let ee_0 = entanglement_entropy_pure(&sv, &[0]).unwrap();
    assert!(
        (ee_0 - 1.0).abs() < EPSILON,
        "Bell state entanglement entropy should be 1.0, got {}",
        ee_0
    );

    // Test subsystem A = [1] - should give same result
    let ee_1 = entanglement_entropy_pure(&sv, &[1]).unwrap();
    assert!(
        (ee_1 - 1.0).abs() < EPSILON,
        "Bell state entanglement entropy should be 1.0, got {}",
        ee_1
    );
}

#[test]
fn test_entanglement_entropy_separable_state() {
    // Product state |+0⟩ = |+⟩ ⊗ |0⟩ - no entanglement
    let mut sv = Statevector::new(2);
    sv.apply_h(0).unwrap(); // Only acts on qubit 0

    let ee = entanglement_entropy_pure(&sv, &[0]).unwrap();
    assert!(
        ee.abs() < EPSILON,
        "Separable state entanglement entropy should be 0, got {}",
        ee
    );
}

#[test]
fn test_entanglement_entropy_ghz_state() {
    // GHZ state |GHZ⟩ = (|000⟩ + |111⟩)/√2
    // Reduced density matrix for any single qubit is maximally mixed
    let mut sv = Statevector::new(3);
    sv.apply_h(0).unwrap();
    sv.apply_cx(0, 1).unwrap();
    sv.apply_cx(0, 2).unwrap();

    // Single qubit subsystem
    let ee_single = entanglement_entropy_pure(&sv, &[0]).unwrap();
    assert!(
        (ee_single - 1.0).abs() < EPSILON,
        "GHZ state single-qubit entanglement entropy should be 1.0, got {}",
        ee_single
    );

    // Two-qubit subsystem - should also have entropy = 1.0
    let ee_double = entanglement_entropy_pure(&sv, &[0, 1]).unwrap();
    assert!(
        (ee_double - 1.0).abs() < EPSILON,
        "GHZ state two-qubit entanglement entropy should be 1.0, got {}",
        ee_double
    );
}

#[test]
fn test_entanglement_entropy_w_state() {
    // W state |W⟩ = (|001⟩ + |010⟩ + |100⟩)/√3
    let n = 3;
    let dim = 1 << n;
    let mut data = vec![Complex64::new(0.0, 0.0); dim];
    // |001⟩ = index 4 (binary 100, little-endian)
    data[4] = Complex64::new(1.0 / 3.0f64.sqrt(), 0.0);
    // |010⟩ = index 2 (binary 010)
    data[2] = Complex64::new(1.0 / 3.0f64.sqrt(), 0.0);
    // |100⟩ = index 1 (binary 001)
    data[1] = Complex64::new(1.0 / 3.0f64.sqrt(), 0.0);

    let sv = Statevector::from_state(n, data).unwrap();

    // For W state, single-qubit entanglement entropy = log2(3) - 2/3 ≈ 0.918
    let ee = entanglement_entropy_pure(&sv, &[0]).unwrap();
    let expected = (3.0f64).log2() - 2.0 / 3.0;
    assert!(
        (ee - expected).abs() < 1e-6,
        "W state entanglement entropy should be {}, got {}",
        expected,
        ee
    );
}

#[test]
fn test_entanglement_entropy_partially_entangled() {
    // State cos(θ)|00⟩ + sin(θ)|11⟩ with θ = π/6
    // Entanglement entropy = -cos²θ log₂(cos²θ) - sin²θ log₂(sin²θ)
    let theta = std::f64::consts::PI / 6.0;
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();
    let cos_sq = cos_theta * cos_theta;
    let sin_sq = sin_theta * sin_theta;

    let mut data = vec![Complex64::new(0.0, 0.0); 4];
    data[0] = Complex64::new(cos_theta, 0.0);
    data[3] = Complex64::new(sin_theta, 0.0);

    let sv = Statevector::from_state(2, data).unwrap();
    let ee = entanglement_entropy_pure(&sv, &[0]).unwrap();

    // Von Neumann entropy of the reduced state
    let expected = -cos_sq * cos_sq.log2() - sin_sq * sin_sq.log2();
    assert!(
        (ee - expected).abs() < EPSILON,
        "Partially entangled state entropy should be {}, got {}",
        expected,
        ee
    );
}

#[test]
fn test_negativity_bell_state() {
    // Bell state |Φ+⟩ has negativity = 0.5
    let mut dm = DensityMatrix::new(2);
    dm.apply_h(0).unwrap();
    dm.apply_cx(0, 1).unwrap();

    let neg = negativity(&dm, &[0]).unwrap();
    assert!(
        (neg - 0.5).abs() < EPSILON,
        "Bell state negativity should be 0.5, got {}",
        neg
    );
}

#[test]
fn test_negativity_separable_state() {
    // Product state |00⟩ has negativity = 0
    let dm = DensityMatrix::new(2);
    let neg = negativity(&dm, &[0]).unwrap();
    assert!(
        neg.abs() < EPSILON,
        "Separable state negativity should be 0, got {}",
        neg
    );
}

#[test]
fn test_negativity_maximally_mixed() {
    // Maximally mixed state is separable, negativity = 0
    let n = 2;
    let dim = 1 << n;
    let mut data = vec![Complex64::new(0.0, 0.0); dim * dim];
    for i in 0..dim {
        data[i * dim + i] = Complex64::new(1.0 / dim as f64, 0.0);
    }
    let dm = DensityMatrix::from_density_matrix_state(n, data).unwrap();

    let neg = negativity(&dm, &[0]).unwrap();
    assert!(
        neg.abs() < EPSILON,
        "Maximally mixed state negativity should be 0, got {}",
        neg
    );
}

#[test]
fn test_negativity_werner_state() {
    // Werner state: ρ = p|Φ+⟩⟨Φ+| + (1-p)I/4
    // For Werner states:
    // - p > 1/3: entangled (negativity > 0)
    // - p <= 1/3: separable (negativity = 0)

    // Test entangled case (p = 0.5)
    let p = 0.5;
    let dm = werner_state(p);

    let neg = negativity(&dm, &[0]).unwrap();
    assert!(
        neg > 0.0,
        "Werner state with p=0.5 should have positive negativity"
    );

    // Test separable case (p = 0.2)
    let p_sep = 0.2;
    let dm_sep = werner_state(p_sep);

    let neg_sep = negativity(&dm_sep, &[0]).unwrap();
    assert!(
        neg_sep.abs() < 1e-6,
        "Werner state with p=0.2 should have negativity ≈ 0, got {}",
        neg_sep
    );
}

#[test]
fn test_negativity_different_subsystems() {
    // Test negativity on different subsystems for Bell state
    let mut dm = DensityMatrix::new(2);
    dm.apply_h(0).unwrap();
    dm.apply_cx(0, 1).unwrap();

    // Subsystem [0]
    let neg_0 = negativity(&dm, &[0]).unwrap();
    assert!((neg_0 - 0.5).abs() < EPSILON);

    // Subsystem [1] - should be the same
    let neg_1 = negativity(&dm, &[1]).unwrap();
    assert!((neg_1 - 0.5).abs() < EPSILON);
}

#[test]
fn test_negativity_ghz_state() {
    // GHZ state has bipartite negativity = 0.5 for any bipartition
    let mut dm = DensityMatrix::new(3);
    dm.apply_h(0).unwrap();
    dm.apply_cx(0, 1).unwrap();
    dm.apply_cx(0, 2).unwrap();

    // Single qubit vs rest
    let neg = negativity(&dm, &[0]).unwrap();
    assert!(
        (neg - 0.5).abs() < EPSILON,
        "GHZ state negativity should be 0.5, got {}",
        neg
    );

    // Two qubits vs one qubit
    let neg_2q = negativity(&dm, &[0, 1]).unwrap();
    assert!(
        (neg_2q - 0.5).abs() < EPSILON,
        "GHZ state (2q vs 1q) negativity should be 0.5, got {}",
        neg_2q
    );
}

#[test]
fn test_concurrence_not_2qubit() {
    // Concurrence only defined for 2-qubit states
    let dm_1q = DensityMatrix::new(1);
    let result = concurrence(&dm_1q);
    assert!(
        result.is_err(),
        "Concurrence for non-2-qubit state should return error"
    );

    let dm_3q = DensityMatrix::new(3);
    let result = concurrence(&dm_3q);
    assert!(
        result.is_err(),
        "Concurrence for 3-qubit state should return error"
    );
}

#[test]
fn test_concurrence_bell_states() {
    // All four Bell states have concurrence = 1.0
    let bell_states = [
        (
            "|Φ+⟩",
            vec![(0, 1.0 / 2.0f64.sqrt()), (3, 1.0 / 2.0f64.sqrt())],
        ),
        (
            "|Φ-⟩",
            vec![(0, 1.0 / 2.0f64.sqrt()), (3, -1.0 / 2.0f64.sqrt())],
        ),
        (
            "|Ψ+⟩",
            vec![(1, 1.0 / 2.0f64.sqrt()), (2, 1.0 / 2.0f64.sqrt())],
        ),
        (
            "|Ψ-⟩",
            vec![(1, 1.0 / 2.0f64.sqrt()), (2, -1.0 / 2.0f64.sqrt())],
        ),
    ];

    for (name, amps) in &bell_states {
        let mut data = vec![Complex64::new(0.0, 0.0); 4];
        for (idx, amp) in amps {
            data[*idx] = Complex64::new(*amp, 0.0);
        }
        let _sv = Statevector::from_state(2, data.clone()).unwrap();
        let dm = DensityMatrix::from_state(2, data).unwrap();

        let c = concurrence(&dm).unwrap();
        assert!(
            (c - 1.0).abs() < EPSILON,
            "Bell state {} should have concurrence = 1.0, got {}",
            name,
            c
        );
    }
}

#[test]
fn test_concurrence_separable_states() {
    // All separable states have concurrence = 0
    let separable_states = [
        ("|00⟩", vec![(0, 1.0)]),
        ("|01⟩", vec![(1, 1.0)]),
        ("|10⟩", vec![(2, 1.0)]),
        ("|11⟩", vec![(3, 1.0)]),
        (
            "|+0⟩",
            vec![(0, 1.0 / 2.0f64.sqrt()), (2, 1.0 / 2.0f64.sqrt())],
        ),
        (
            "|0+⟩",
            vec![(0, 1.0 / 2.0f64.sqrt()), (1, 1.0 / 2.0f64.sqrt())],
        ),
    ];

    for (name, amps) in &separable_states {
        let mut data = vec![Complex64::new(0.0, 0.0); 4];
        for (idx, amp) in amps {
            data[*idx] = Complex64::new(*amp, 0.0);
        }
        let dm = DensityMatrix::from_state(2, data).unwrap();

        let c = concurrence(&dm).unwrap();
        assert!(
            c.abs() < EPSILON,
            "Separable state {} should have concurrence = 0, got {}",
            name,
            c
        );
    }
}

#[test]
fn test_concurrence_maximally_mixed() {
    // Maximally mixed state has concurrence = 0
    let dim = 4;
    let mut data = vec![Complex64::new(0.0, 0.0); dim * dim];
    for i in 0..dim {
        data[i * dim + i] = Complex64::new(1.0 / dim as f64, 0.0);
    }
    let dm = DensityMatrix::from_density_matrix_state(2, data).unwrap();

    let c = concurrence(&dm).unwrap();
    assert!(
        c.abs() < EPSILON,
        "Maximally mixed state should have concurrence = 0, got {}",
        c
    );
}

#[test]
fn test_concurrence_partially_entangled() {
    // State cos(θ)|00⟩ + sin(θ)|11⟩
    // Concurrence = |sin(2θ)|
    let test_angles = [
        (std::f64::consts::PI / 12.0, 0.5), // θ = 15°, C = sin(30°) = 0.5
        (std::f64::consts::PI / 6.0, 3.0f64.sqrt() / 2.0), // θ = 30°, C = sin(60°) = √3/2
        (std::f64::consts::PI / 4.0, 1.0),  // θ = 45°, C = sin(90°) = 1
    ];

    for (theta, expected_c) in &test_angles {
        let mut data = vec![Complex64::new(0.0, 0.0); 4];
        data[0] = Complex64::new(theta.cos(), 0.0);
        data[3] = Complex64::new(theta.sin(), 0.0);
        let dm = DensityMatrix::from_state(2, data).unwrap();

        let c = concurrence(&dm).unwrap();
        assert!(
            (c - *expected_c).abs() < 1e-6,
            "Partially entangled state with θ={:?} should have concurrence = {}, got {}",
            theta,
            expected_c,
            c
        );
    }
}

#[test]
fn test_concurrence_werner_state() {
    // Werner state: ρ = p|Φ+⟩⟨Φ+| + (1-p)I/4
    // For Werner states: C = max(0, (3p - 1) / 2)

    // Entangled case (p > 1/3)
    let p = 0.5;
    let expected_c = (3.0 * p - 1.0) / 2.0;

    let dm = werner_state(p);

    let c = concurrence(&dm).unwrap();
    assert!(
        (c - expected_c).abs() < 1e-6,
        "Werner state with p={} should have concurrence ≈ {}, got {}",
        p,
        expected_c,
        c
    );

    // Separable case (p = 0.2)
    let p_sep = 0.2;
    let dm_sep = werner_state(p_sep);

    let c_sep = concurrence(&dm_sep).unwrap();
    assert!(
        c_sep.abs() < EPSILON,
        "Werner state with p={} should have concurrence = 0, got {}",
        p_sep,
        c_sep
    );
}

#[test]
fn test_eof_bell_states() {
    // All Bell states have EOF = 1.0
    let mut dm = DensityMatrix::new(2);
    dm.apply_h(0).unwrap();
    dm.apply_cx(0, 1).unwrap();

    let eof = entanglement_of_formation(&dm).unwrap();
    assert!(
        (eof - 1.0).abs() < EPSILON,
        "Bell state EOF should be 1.0, got {}",
        eof
    );
}

#[test]
fn test_eof_clamps_concurrence_roundoff_before_square_root() {
    for c in [1.0, 1.0 + f64::EPSILON, 1.0 + 8.0 * f64::EPSILON] {
        let eof = super::entanglement_of_formation_from_concurrence(c);
        assert_eq!(eof, 1.0, "concurrence={c}, entropy={eof}");
    }
    for c in [0.0, -f64::EPSILON] {
        assert_eq!(super::entanglement_of_formation_from_concurrence(c), 0.0);
    }
    // An interior value still uses the binary entropy formula: C=0.6 gives x=0.9.
    let expected = -0.9_f64 * 0.9_f64.log2() - 0.1_f64 * 0.1_f64.log2();
    let actual = super::entanglement_of_formation_from_concurrence(0.6);
    assert!((actual - expected).abs() < 1e-14);
}

#[test]
fn test_eof_separable_states() {
    // Separable states have EOF = 0
    let dm = DensityMatrix::new(2);
    let eof = entanglement_of_formation(&dm).unwrap();
    assert!(
        eof.abs() < EPSILON,
        "Separable state EOF should be 0, got {}",
        eof
    );
}

#[test]
fn test_eof_maximally_mixed() {
    // Maximally mixed state has EOF = 0
    let dim = 4;
    let mut data = vec![Complex64::new(0.0, 0.0); dim * dim];
    for i in 0..dim {
        data[i * dim + i] = Complex64::new(1.0 / dim as f64, 0.0);
    }
    let dm = DensityMatrix::from_density_matrix_state(2, data).unwrap();

    let eof = entanglement_of_formation(&dm).unwrap();
    assert!(
        eof.abs() < EPSILON,
        "Maximally mixed state EOF should be 0, got {}",
        eof
    );
}

#[test]
fn test_eof_partially_entangled() {
    // State cos(θ)|00⟩ + sin(θ)|11⟩
    // C = sin(2θ)
    // EOF = H((1 + √(1-C²))/2)
    let theta = std::f64::consts::PI / 6.0;
    let c = (2.0 * theta).sin();

    let mut data = vec![Complex64::new(0.0, 0.0); 4];
    data[0] = Complex64::new(theta.cos(), 0.0);
    data[3] = Complex64::new(theta.sin(), 0.0);
    let dm = DensityMatrix::from_state(2, data).unwrap();

    let eof = entanglement_of_formation(&dm).unwrap();

    // Expected EOF
    let sqrt_term = (1.0 - c * c).sqrt();
    let x = (1.0 + sqrt_term) / 2.0;
    let expected_eof = -x * x.log2() - (1.0 - x) * (1.0 - x).log2();

    assert!(
        (eof - expected_eof).abs() < 1e-6,
        "Partially entangled state EOF should be {}, got {}",
        expected_eof,
        eof
    );
}

#[test]
fn test_eof_concurrence_relationship() {
    // Verify that EOF(C=1) = 1 and EOF(C=0) = 0
    // Bell state (C=1)
    let mut dm_bell = DensityMatrix::new(2);
    dm_bell.apply_h(0).unwrap();
    dm_bell.apply_cx(0, 1).unwrap();

    let c_bell = concurrence(&dm_bell).unwrap();
    let eof_bell = entanglement_of_formation(&dm_bell).unwrap();

    assert!((c_bell - 1.0).abs() < EPSILON);
    assert!((eof_bell - 1.0).abs() < EPSILON);

    // Separable state (C=0)
    let dm_sep = DensityMatrix::new(2);
    let c_sep = concurrence(&dm_sep).unwrap();
    let eof_sep = entanglement_of_formation(&dm_sep).unwrap();

    assert!(c_sep.abs() < EPSILON);
    assert!(eof_sep.abs() < EPSILON);
}
