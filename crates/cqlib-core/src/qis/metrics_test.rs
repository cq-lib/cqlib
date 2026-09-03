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

use crate::qis::metrics::*;
use crate::qis::state::{DensityMatrix, Statevector};
use num_complex::Complex64;

#[test]
fn test_purity_pure() {
    let sv = Statevector::new(2);
    let purity = purity_pure(&sv).unwrap();
    assert!((purity - 1.0).abs() < 1e-10);
}

#[test]
fn test_purity_mixed() {
    // Identity state I / 2 (Maximally mixed for 1 qubit)
    let dm = DensityMatrix::maximally_mixed(1);
    let purity = purity_mixed(&dm).unwrap();
    assert!((purity - 0.5).abs() < 1e-10); // Tr( (I/2)^2 ) = 1/4 + 1/4 = 0.5
}

#[test]
fn test_state_fidelity_pure() {
    let sv1 = Statevector::new(1); // |0>
    let mut sv2 = Statevector::new(1);
    sv2.data_mut()[0] = Complex64::new(0.0, 0.0);
    sv2.data_mut()[1] = Complex64::new(1.0, 0.0); // |1>

    let fid_same = state_fidelity_pure(&sv1, &sv1).unwrap();
    assert!((fid_same - 1.0).abs() < 1e-10);

    let fid_ortho = state_fidelity_pure(&sv1, &sv2).unwrap();
    assert!((fid_ortho - 0.0).abs() < 1e-10);
}

#[test]
fn test_trace_distance_pure() {
    let sv1 = Statevector::new(1); // |0>
    let mut sv2 = Statevector::new(1);
    sv2.data_mut()[0] = Complex64::new(0.0, 0.0);
    sv2.data_mut()[1] = Complex64::new(1.0, 0.0); // |1>

    let dist_same = trace_distance_pure(&sv1, &sv1).unwrap();
    assert!((dist_same - 0.0).abs() < 1e-10);

    let dist_ortho = trace_distance_pure(&sv1, &sv2).unwrap();
    assert!((dist_ortho - 1.0).abs() < 1e-10);
}

#[test]
fn test_state_fidelity_pure_mixed() {
    let sv = Statevector::new(1); // |0>

    // Pure state density matrix |0><0|
    let dm_pure = DensityMatrix::new(1);

    // Maximally mixed density matrix I / 2
    let dm_mixed = DensityMatrix::maximally_mixed(1);

    let fid_pure = state_fidelity_pure_mixed(&sv, &dm_pure).unwrap();
    assert!((fid_pure - 1.0).abs() < 1e-10);

    let fid_mixed = state_fidelity_pure_mixed(&sv, &dm_mixed).unwrap();
    assert!((fid_mixed - 0.5).abs() < 1e-10);
}

#[test]
fn test_entropy_pure() {
    let dm = DensityMatrix::new(1); // |0><0|
    let ent = entropy(&dm).unwrap();
    assert!(ent.abs() < 1e-10); // pure state entropy is 0
}

#[test]
fn test_entropy_mixed() {
    let dm = DensityMatrix::maximally_mixed(1);
    let ent = entropy(&dm).unwrap();
    // 1 qubit maximally mixed state entropy is 1.0 (using log2)
    assert!((ent - 1.0).abs() < 1e-10);
}

#[test]
fn test_trace_distance_mixed() {
    let dm1 = DensityMatrix::new(1); // |0><0|

    let dm2 = DensityMatrix::from_density_matrix_state(
        1,
        vec![
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(1.0, 0.0),
        ],
    )
    .unwrap();

    let dist = trace_distance_mixed(&dm1, &dm2).unwrap();
    assert!((dist - 1.0).abs() < 1e-10); // orthogonal states have dist 1

    let dist_same = trace_distance_mixed(&dm1, &dm1).unwrap();
    assert!((dist_same - 0.0).abs() < 1e-10); // same states have dist 0
}

#[test]
fn test_state_fidelity_mixed() {
    let dm1 = DensityMatrix::new(1); // |0><0|

    let dm2 = DensityMatrix::from_density_matrix_state(
        1,
        vec![
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(1.0, 0.0),
        ],
    )
    .unwrap();

    let fid_ortho = state_fidelity_mixed(&dm1, &dm2).unwrap();
    assert!((fid_ortho - 0.0).abs() < 1e-10); // orthogonal

    let fid_same = state_fidelity_mixed(&dm1, &dm1).unwrap();
    assert!((fid_same - 1.0).abs() < 1e-10); // same
}

#[test]
fn test_partial_transpose_single_qubit() {
    // For a single qubit, partial transpose is just regular transpose
    // |0><0|^T = |0><0|
    let dm = DensityMatrix::new(1); // |0><0|
    let pt = partial_transpose(&dm, &[0]).unwrap();

    // Should be unchanged (diagonal matrix is symmetric)
    assert!((dm.data()[0] - pt.data()[0]).norm() < 1e-10);
    assert!((dm.data()[3] - pt.data()[3]).norm() < 1e-10);
}

#[test]
fn test_partial_transpose_bell_state() {
    // Create Bell state |Φ+> = (|00> + |11>)/sqrt(2)
    // Density matrix has non-zero elements at (0,0), (0,3), (3,0), (3,3)
    let factor = 0.5;
    let mut data = vec![Complex64::new(0.0, 0.0); 16];
    data[0] = Complex64::new(factor, 0.0); // (0,0) = |00><00|
    data[3] = Complex64::new(factor, 0.0); // (0,3) = |00><11|
    data[12] = Complex64::new(factor, 0.0); // (3,0) = |11><00| (row-major: 3*4+0=12)
    data[15] = Complex64::new(factor, 0.0); // (3,3) = |11><11|
    let dm = DensityMatrix::from_density_matrix_state(2, data).unwrap();

    // Partial transpose on first qubit
    let pt = partial_transpose(&dm, &[0]).unwrap();

    // For Bell state, partial transpose on either qubit should swap |00><11| <-> |10><01|
    // This creates negative eigenvalues (entanglement detection)
    // Check that the matrix structure changed appropriately
    assert!((pt.data()[0] - Complex64::new(factor, 0.0)).norm() < 1e-10);
    assert!((pt.data()[15] - Complex64::new(factor, 0.0)).norm() < 1e-10);
}

#[test]
fn test_partial_transpose_invalid_qubit() {
    let dm = DensityMatrix::new(2);
    let result = partial_transpose(&dm, &[2]); // qubit 2 doesn't exist in 2-qubit system
    assert!(result.is_err());
}

#[test]
fn test_partial_transpose_separable_state() {
    // Product state |01> = |0> ⊗ |1>
    // Partial transpose should leave eigenvalues unchanged (still positive)
    let mut data = vec![Complex64::new(0.0, 0.0); 16];
    data[5] = Complex64::new(1.0, 0.0); // |01><01| at (1,1) in 4x4 = 1*4+1=5
    let dm = DensityMatrix::from_density_matrix_state(2, data).unwrap();

    let pt = partial_transpose(&dm, &[0]).unwrap();

    // Product state should remain unchanged under partial transpose
    assert!((pt.data()[5] - Complex64::new(1.0, 0.0)).norm() < 1e-10);
}

#[test]
fn test_logarithmic_negativity_separable() {
    // Product state |01> - should have zero entanglement
    let mut data = vec![Complex64::new(0.0, 0.0); 16];
    data[5] = Complex64::new(1.0, 0.0); // |01><01| at (1,1) in 4x4 = 1*4+1=5
    let dm = DensityMatrix::from_density_matrix_state(2, data).unwrap();

    let neg = logarithmic_negativity(&dm, &[0]).unwrap();
    assert!(
        neg.abs() < 1e-10,
        "Expected near-zero negativity for separable state, got {}",
        neg
    ); // No entanglement
}

#[test]
fn test_logarithmic_negativity_bell_state() {
    // Maximally entangled Bell state |Φ+> should have log neg = 1
    let factor = 0.5;
    let mut data = vec![Complex64::new(0.0, 0.0); 16];
    data[0] = Complex64::new(factor, 0.0);
    data[3] = Complex64::new(factor, 0.0);
    data[12] = Complex64::new(factor, 0.0);
    data[15] = Complex64::new(factor, 0.0);
    let dm = DensityMatrix::from_density_matrix_state(2, data).unwrap();

    let neg = logarithmic_negativity(&dm, &[0]).unwrap();
    assert!((neg - 1.0).abs() < 1e-10); // Maximum entanglement for 2-qubit system
}

#[test]
fn test_logarithmic_negativity_ghz_state() {
    // GHZ state |Φ> = (|000> + |111>)/sqrt(2) for 3 qubits
    let dim = 8;
    let factor = 0.5;

    // |000><000|, |000><111|, |111><000|, |111><111|
    let mut data = vec![Complex64::new(0.0, 0.0); dim * dim];
    data[0] = Complex64::new(factor, 0.0); // (0,0)
    data[7] = Complex64::new(factor, 0.0); // (0,7)
    data[7 * dim] = Complex64::new(factor, 0.0); // (7,0)
    data[7 * dim + 7] = Complex64::new(factor, 0.0); // (7,7)
    let dm = DensityMatrix::from_density_matrix_state(3, data).unwrap();

    // Entanglement between any single qubit and the rest
    let neg = logarithmic_negativity(&dm, &[0]).unwrap();
    assert!(neg > 0.0); // GHZ state is entangled

    // All bipartitions should have the same entanglement
    let neg1 = logarithmic_negativity(&dm, &[1]).unwrap();
    let neg2 = logarithmic_negativity(&dm, &[2]).unwrap();
    assert!((neg - neg1).abs() < 1e-10);
    assert!((neg - neg2).abs() < 1e-10);
}

#[test]
fn test_purity_mixed_multiqubit() {
    // Maximally mixed state for n qubits has purity = 1/2^n
    for n in 1..=4 {
        let dim = 1 << n;
        let dm = DensityMatrix::maximally_mixed(n);

        let purity = purity_mixed(&dm).unwrap();
        let expected = 1.0 / (dim as f64);
        assert!(
            (purity - expected).abs() < 1e-10,
            "Failed for {} qubits: got {}, expected {}",
            n,
            purity,
            expected
        );
    }
}

#[test]
fn test_fidelity_superposition_state() {
    // |+> = (|0> + |1>)/sqrt(2)
    let mut sv_plus = Statevector::new(1);
    sv_plus.data_mut()[0] = Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0);
    sv_plus.data_mut()[1] = Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0);

    // |-> = (|0> - |1>)/sqrt(2)
    let mut sv_minus = Statevector::new(1);
    sv_minus.data_mut()[0] = Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0);
    sv_minus.data_mut()[1] = Complex64::new(-1.0 / 2.0_f64.sqrt(), 0.0);

    // |+> and |-> are orthogonal
    let fid = state_fidelity_pure(&sv_plus, &sv_minus).unwrap();
    assert!(fid.abs() < 1e-10);
}

#[test]
fn test_entropy_completely_mixed() {
    // n-qubit maximally mixed state has entropy = n
    for n in 1..=4 {
        let dm = DensityMatrix::maximally_mixed(n);

        let ent = entropy(&dm).unwrap();
        // Using log2, entropy should be n
        assert!(
            (ent - (n as f64)).abs() < 1e-10,
            "Failed for {} qubits: got {}, expected {}",
            n,
            ent,
            n
        );
    }
}

#[test]
fn test_fidelity_mismatched_qubits() {
    let sv1 = Statevector::new(1);
    let sv2 = Statevector::new(2);

    let result = state_fidelity_pure(&sv1, &sv2);
    assert!(result.is_err());
}

#[test]
fn test_trace_distance_mismatched_qubits() {
    let dm1 = DensityMatrix::new(1);
    let dm2 = DensityMatrix::new(2);

    let result = trace_distance_mixed(&dm1, &dm2);
    assert!(result.is_err());
}

#[test]
fn test_fidelity_pure_mixed_mismatched_qubits() {
    let sv = Statevector::new(1);
    let dm = DensityMatrix::new(2);

    let result = state_fidelity_pure_mixed(&sv, &dm);
    assert!(result.is_err());
}

#[test]
fn test_fidelity_trace_distance_inequality() {
    // Quantum fidelity and trace distance satisfy:
    // 1 - sqrt(F) <= D <= sqrt(1 - F)
    // where F is fidelity and D is trace distance

    let sv1 = Statevector::new(2);
    let mut sv2 = Statevector::new(2);
    sv2.data_mut()[0] = Complex64::new(0.5, 0.0);
    sv2.data_mut()[1] = Complex64::new(0.5, 0.0);
    sv2.data_mut()[2] = Complex64::new(0.5, 0.0);
    sv2.data_mut()[3] = Complex64::new(0.5, 0.0);
    // Normalize: current norm is 1.0 (already normalized as 4 * 0.25 = 1)

    let fid = state_fidelity_pure(&sv1, &sv2).unwrap();
    let dist = trace_distance_pure(&sv1, &sv2).unwrap();

    let lower = 1.0 - fid.sqrt();
    let upper = (1.0 - fid).sqrt();

    assert!(
        dist >= lower - 1e-10,
        "Trace distance {} should be >= {}",
        dist,
        lower
    );
    assert!(
        dist <= upper + 1e-10,
        "Trace distance {} should be <= {}",
        dist,
        upper
    );
}

#[test]
fn test_triangle_inequality_trace_distance() {
    // D(ρ, σ) <= D(ρ, τ) + D(τ, σ)
    let dm1 = DensityMatrix::new(2);

    let mut data2 = vec![Complex64::new(0.0, 0.0); 16];
    data2[5] = Complex64::new(1.0, 0.0); // |01><01|
    let dm2 = DensityMatrix::from_density_matrix_state(2, data2).unwrap();

    let mut data3 = vec![Complex64::new(0.0, 0.0); 16];
    data3[15] = Complex64::new(1.0, 0.0); // |11><11|
    let dm3 = DensityMatrix::from_density_matrix_state(2, data3).unwrap();

    let d12 = trace_distance_mixed(&dm1, &dm2).unwrap();
    let d23 = trace_distance_mixed(&dm2, &dm3).unwrap();
    let d13 = trace_distance_mixed(&dm1, &dm3).unwrap();

    assert!(d13 <= d12 + d23 + 1e-10, "Triangle inequality violated");
}

#[test]
fn test_werner_state_entanglement() {
    // Werner state: ρ = p |Φ+><Φ+| + (1-p) I/4
    // For p > 1/3, the state is entangled (logarithmic negativity > 0)
    // For p <= 1/3, the state is separable

    let dim = 4;
    let p_values = [0.0, 0.25, 1.0 / 3.0, 0.5, 1.0];

    for &p in &p_values {
        let mixed_factor = (1.0 - p) / 4.0;
        let mut data = vec![Complex64::new(0.0, 0.0); dim * dim];

        // Maximally mixed contribution
        for i in 0..dim {
            data[i * dim + i] = Complex64::new(mixed_factor, 0.0);
        }

        // Bell state contribution
        let bell_factor = p * 0.5;
        data[0] += Complex64::new(bell_factor, 0.0);
        data[3] = Complex64::new(bell_factor, 0.0);
        data[12] = Complex64::new(bell_factor, 0.0);
        data[15] += Complex64::new(bell_factor, 0.0);
        let dm = DensityMatrix::from_density_matrix_state(2, data).unwrap();

        let neg = logarithmic_negativity(&dm, &[0]).unwrap();

        // At p = 0: no entanglement, neg = 0
        // At p = 1/3: boundary case, should be ~0
        // At p > 1/3: entangled, neg > 0
        if p < 1.0 / 3.0 - 1e-10 {
            assert!(
                neg < 0.1,
                "Expected small negativity for p={}, got {}",
                p,
                neg
            );
        } else if p > 1.0 / 3.0 + 1e-10 {
            assert!(
                neg > 0.0,
                "Expected positive negativity for p={}, got {}",
                p,
                neg
            );
        }
    }
}

#[test]
fn test_fidelity_with_phase() {
    // |ψ> and e^{iθ}|ψ> should have fidelity = 1 (global phase doesn't matter)
    let sv1 = Statevector::new(1);

    let mut sv2 = Statevector::new(1);
    use std::f64::consts::PI;
    sv2.data_mut()[0] = Complex64::new(PI.cos(), PI.sin()); // e^{iπ}|0> = -|0>

    let fid = state_fidelity_pure(&sv1, &sv2).unwrap();
    assert!(
        (fid - 1.0).abs() < 1e-10,
        "Global phase should not affect fidelity"
    );
}

#[test]
fn test_state_fidelity_pure_mixed_with_superposition() {
    // Test with |+> state and its density matrix
    let mut sv = Statevector::new(1);
    sv.data_mut()[0] = Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0);
    sv.data_mut()[1] = Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0);

    // Construct density matrix |+><+|
    let factor = 0.5;
    let dm = DensityMatrix::from_density_matrix_state(
        1,
        vec![
            Complex64::new(factor, 0.0), // |0><0|
            Complex64::new(factor, 0.0), // |0><1|
            Complex64::new(factor, 0.0), // |1><0|
            Complex64::new(factor, 0.0), // |1><1|
        ],
    )
    .unwrap();

    let fid = state_fidelity_pure_mixed(&sv, &dm).unwrap();
    assert!((fid - 1.0).abs() < 1e-10);
}
