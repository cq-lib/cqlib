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

#[test]
fn test_new_single_qubit() {
    let s = StabilizerState::new(1);
    // Destabilizer row 0: X on qubit 0, phase +1
    assert!(s.x_bit(0, 0));
    assert!(!s.z_bit(0, 0));
    assert_eq!(s.phase(0), 0);
    // Stabilizer row 1: Z on qubit 0, phase +1
    assert!(!s.x_bit(1, 0));
    assert!(s.z_bit(1, 0));
    assert_eq!(s.phase(1), 0);
}

#[test]
fn test_new_two_qubits() {
    let s = StabilizerState::new(2);
    // Destabilizer row 0: X on q0 only
    assert!(s.x_bit(0, 0));
    assert!(!s.x_bit(0, 1));
    // Destabilizer row 1: X on q1 only
    assert!(!s.x_bit(1, 0));
    assert!(s.x_bit(1, 1));
    // Stabilizer row 2: Z on q0 only
    assert!(s.z_bit(2, 0));
    assert!(!s.z_bit(2, 1));
    // Stabilizer row 3: Z on q1 only
    assert!(!s.z_bit(3, 0));
    assert!(s.z_bit(3, 1));
}

#[test]
fn test_get_stabilizers_initial() {
    let s = StabilizerState::new(2);
    let stabs = s.get_stabilizers();
    assert_eq!(stabs.len(), 2);
    // |00⟩ stabilizers: Z₀ = +ZI, Z₁ = +IZ
    // (PauliString display: highest index first)
    assert_eq!(stabs[0].to_string(), "+IZ");
    assert_eq!(stabs[1].to_string(), "+ZI");
}

#[test]
fn test_apply_h_plus_state() {
    // H|0⟩ = |+⟩, stabilized by +X
    let mut s = StabilizerState::new(1);
    s.apply_h(0).unwrap();
    let stabs = s.get_stabilizers();
    assert_eq!(stabs[0].to_string(), "+X");
}

#[test]
fn test_apply_h_twice_is_identity() {
    let mut s = StabilizerState::new(2);
    s.apply_h(0).unwrap();
    s.apply_h(0).unwrap();
    // Should be back to initial |00⟩ stabilizers
    let stabs = s.get_stabilizers();
    assert_eq!(stabs[0].to_string(), "+IZ");
    assert_eq!(stabs[1].to_string(), "+ZI");
}

#[test]
fn test_apply_s_on_plus_state() {
    // S|+⟩ = |Y+⟩ stabilized by +Y (since S·X·S† = Y)
    let mut s = StabilizerState::new(1);
    s.apply_h(0).unwrap();
    s.apply_s(0).unwrap();
    let stabs = s.get_stabilizers();
    assert_eq!(stabs[0].to_string(), "+Y");
}

#[test]
fn test_swap_twice_is_identity() {
    let mut s = StabilizerState::new(2);
    s.apply_h(0).unwrap(); // |+0⟩
    s.apply_swap(0, 1).unwrap();
    s.apply_swap(0, 1).unwrap();
    let stabs = s.get_stabilizers();
    // Should be back to |+0⟩ stabilizers: +XI, +IZ
    let stab_strs: Vec<String> = stabs.iter().map(|p| p.to_string()).collect();
    assert!(
        stab_strs.contains(&"+IX".to_string()),
        "got {:?}",
        stab_strs
    );
    assert!(
        stab_strs.contains(&"+ZI".to_string()),
        "got {:?}",
        stab_strs
    );
}

#[test]
fn test_swap_exchanges_qubits() {
    // |+0⟩ → SWAP → |0+⟩: stabilizers +XI on q1 and +IZ on q0
    let mut s = StabilizerState::new(2);
    s.apply_h(0).unwrap(); // q0=|+⟩, q1=|0⟩
    s.apply_swap(0, 1).unwrap(); // q0=|0⟩, q1=|+⟩
    let stabs = s.get_stabilizers();
    let stab_strs: Vec<String> = stabs.iter().map(|p| p.to_string()).collect();
    // q0 stabilized by Z, q1 stabilized by X
    assert!(
        stab_strs.contains(&"+IZ".to_string()),
        "got {:?}",
        stab_strs
    );
    assert!(
        stab_strs.contains(&"+XI".to_string()),
        "got {:?}",
        stab_strs
    );
}

#[test]
fn test_cz_twice_is_identity() {
    let mut s = StabilizerState::new(2);
    s.apply_cz(0, 1).unwrap();
    s.apply_cz(0, 1).unwrap();
    let stabs = s.get_stabilizers();
    assert_eq!(stabs[0].to_string(), "+IZ");
    assert_eq!(stabs[1].to_string(), "+ZI");
}

#[test]
fn test_cz_creates_entanglement() {
    // |++⟩ → (H⊗H)|00⟩, then CZ → entangled
    let mut s = StabilizerState::new(2);
    s.apply_h(0).unwrap();
    s.apply_h(1).unwrap();
    s.apply_cz(0, 1).unwrap();
    // CZ|++⟩ = (|00⟩+|01⟩+|10⟩-|11⟩)/2
    // Stabilizers: +XZ, +ZX
    let stabs = s.get_stabilizers();
    let stab_strs: Vec<String> = stabs.iter().map(|p| p.to_string()).collect();
    assert!(
        stab_strs.contains(&"+XZ".to_string()) && stab_strs.contains(&"+ZX".to_string()),
        "Expected both +XZ and +ZX, got: {:?}",
        stab_strs
    );
}

#[test]
fn test_ghz_state_stabilizers() {
    // GHZ = H(q0), CX(0,1), CX(0,2) → stabilizers +XXX, +ZZI, +IZZ
    let mut s = StabilizerState::new(3);
    s.apply_h(0).unwrap();
    s.apply_cx(0, 1).unwrap();
    s.apply_cx(1, 2).unwrap();
    let stab_strs: Vec<String> = s.get_stabilizers().iter().map(|p| p.to_string()).collect();
    // Expected display order is highest qubit first.
    assert!(
        stab_strs.contains(&"+XXX".to_string()),
        "+XXX not found, got {:?}",
        stab_strs
    );
    assert!(
        stab_strs.contains(&"+ZZI".to_string()),
        "+ZZI not found, got {:?}",
        stab_strs
    );
    assert!(
        stab_strs.contains(&"+IZZ".to_string()),
        "+IZZ not found, got {:?}",
        stab_strs
    );
}

#[test]
fn test_from_circuit_rejects_non_clifford_gate_set() {
    use crate::circuit::circuit_impl::Circuit;
    use crate::circuit::gate::UnitaryGate;
    use ndarray::array;
    use num_complex::Complex64;
    use std::f64::consts::PI;

    type CircuitBuilder = (&'static str, Box<dyn Fn(&mut Circuit)>);
    let builders: Vec<CircuitBuilder> = vec![
        (
            "T",
            Box::new(|c| {
                c.t(Qubit::new(0)).unwrap();
            }),
        ),
        (
            "TDG",
            Box::new(|c| {
                c.tdg(Qubit::new(0)).unwrap();
            }),
        ),
        (
            "RX",
            Box::new(|c| {
                c.rx(Qubit::new(0), PI / 4.0).unwrap();
            }),
        ),
        (
            "RY",
            Box::new(|c| {
                c.ry(Qubit::new(0), PI / 4.0).unwrap();
            }),
        ),
        (
            "RZ",
            Box::new(|c| {
                c.rz(Qubit::new(0), PI / 4.0).unwrap();
            }),
        ),
        (
            "Phase",
            Box::new(|c| {
                c.phase(Qubit::new(0), PI / 4.0).unwrap();
            }),
        ),
        (
            "U",
            Box::new(|c| {
                c.u(Qubit::new(0), PI / 4.0, 0.0, 0.0).unwrap();
            }),
        ),
        (
            "UnitaryGate",
            Box::new(|c| {
                let mat = array![
                    [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
                    [Complex64::new(0.0, 0.0), Complex64::new(0.0, 1.0)]
                ];
                let gate = UnitaryGate::new("S_like_custom", 1, 0)
                    .with_matrix(mat)
                    .unwrap();
                c.unitary(gate, vec![Qubit::new(0)]).unwrap();
            }),
        ),
    ];

    for (name, build) in builders {
        let mut c = Circuit::new(1);
        build(&mut c);
        let result = StabilizerState::from_circuit(&c);
        assert!(result.is_err(), "{name} should be rejected, got {result:?}");
    }
}

#[test]
fn test_cy_twice_is_identity() {
    let mut s = StabilizerState::new(2);
    s.apply_cy(0, 1).unwrap();
    s.apply_cy(0, 1).unwrap();
    let stabs = s.get_stabilizers();
    assert_eq!(stabs[0].to_string(), "+IZ");
    assert_eq!(stabs[1].to_string(), "+ZI");
}

#[test]
fn test_bell_state_stabilizers() {
    // H on q0, then CX(q0, q1) → Bell state |Φ+⟩
    // Stabilizers: +XX (index order: qubit 1 highest), +ZZ
    let mut s = StabilizerState::new(2);
    s.apply_h(0).unwrap();
    s.apply_cx(0, 1).unwrap();
    let stabs = s.get_stabilizers();
    // PauliString display: highest qubit index first
    // Stabilizer 0 (from destab q0 path): XX on both qubits
    // Stabilizer 1 (from stab q0 path): ZZ on both qubits
    let stab_strs: Vec<String> = stabs.iter().map(|p| p.to_string()).collect();
    assert!(
        stab_strs.contains(&"+XX".to_string()),
        "Expected +XX, got {:?}",
        stab_strs
    );
    assert!(
        stab_strs.contains(&"+ZZ".to_string()),
        "Expected +ZZ, got {:?}",
        stab_strs
    );
}

#[test]
fn test_cx_twice_is_identity() {
    let mut s = StabilizerState::new(2);
    s.apply_cx(0, 1).unwrap();
    s.apply_cx(0, 1).unwrap();
    let stabs = s.get_stabilizers();
    assert_eq!(stabs[0].to_string(), "+IZ");
    assert_eq!(stabs[1].to_string(), "+ZI");
}

#[test]
fn test_apply_x_flips_z_stabilizer() {
    // X on |0⟩ gives |1⟩, stabilized by -Z
    let mut s = StabilizerState::new(1);
    s.apply_x(0).unwrap();
    let stabs = s.get_stabilizers();
    assert_eq!(stabs[0].to_string(), "-Z");
}

#[test]
fn test_apply_z_on_plus_state() {
    // Z|+⟩ = |-⟩, stabilized by -X
    let mut s = StabilizerState::new(1);
    s.apply_h(0).unwrap();
    s.apply_z(0).unwrap();
    let stabs = s.get_stabilizers();
    assert_eq!(stabs[0].to_string(), "-X");
}

#[test]
fn test_apply_y_on_zero_state() {
    // Y|0⟩ = i|1⟩, stabilizer -Z (phase flips for rows with Z, but row has Z in stab)
    let mut s = StabilizerState::new(1);
    s.apply_y(0).unwrap();
    let stabs = s.get_stabilizers();
    assert_eq!(stabs[0].to_string(), "-Z");
}

#[test]
fn test_pauli_xx_is_identity() {
    // X followed by X = identity
    let mut s = StabilizerState::new(1);
    s.apply_x(0).unwrap();
    s.apply_x(0).unwrap();
    let stabs = s.get_stabilizers();
    assert_eq!(stabs[0].to_string(), "+Z");
}

#[test]
fn test_apply_s_sdg_inverse() {
    // S followed by S† should be identity
    let mut s = StabilizerState::new(1);
    s.apply_h(0).unwrap(); // |+⟩
    s.apply_s(0).unwrap();
    s.apply_sdg(0).unwrap();
    let stabs = s.get_stabilizers();
    // Should be back to |+⟩ stabilized by +X
    assert_eq!(stabs[0].to_string(), "+X");
}

#[test]
fn test_apply_sdg_on_plus_state() {
    // S†|+⟩ = |Y-⟩ stabilized by -Y (since S†·X·S = -Y)
    let mut s = StabilizerState::new(1);
    s.apply_h(0).unwrap();
    s.apply_sdg(0).unwrap();
    let stabs = s.get_stabilizers();
    assert_eq!(stabs[0].to_string(), "-Y");
}

#[test]
fn test_pauli_expectation_on_stabilized_state() {
    use crate::qis::pauli::{Pauli, PauliString};
    // |+⟩ is stabilized by +X
    let mut s = StabilizerState::new(1);
    s.apply_h(0).unwrap();
    let mut zx = PauliString::new(1);
    zx.set_pauli(0, Pauli::X);
    assert_eq!(s.pauli_expectation(&zx).unwrap(), 1);
}

#[test]
fn test_pauli_expectation_qubit_mismatch_returns_error() {
    use crate::qis::pauli::PauliString;
    let s = StabilizerState::new(2);
    let wrong = PauliString::new(3); // 3 qubits vs state's 2
    let result = s.pauli_expectation(&wrong);
    assert!(
        matches!(
            result,
            Err(QisError::QubitMismatch {
                expected: 2,
                actual: 3
            })
        ),
        "Expected QubitMismatch, got {result:?}"
    );
}

#[test]
fn test_to_stim_format() {
    let s = StabilizerState::new(1);
    let out = s.to_stim_format();
    // Initial state |0⟩ has stabilizer +Z
    assert!(out.contains("+Z"), "to_stim_format: got '{}'", out.trim());
}

#[test]
fn test_measure_statistics_approx_50_50() {
    // After 200 measurements of |+⟩, should see ~50% 0 and ~50% 1.
    let mut ones = 0usize;
    let total = 200;
    for _ in 0..total {
        let mut s = StabilizerState::new(1);
        s.apply_h(0).unwrap();
        if s.measure(0).unwrap() {
            ones += 1;
        }
    }
    // Generous bounds: 30% to 70%
    assert!(
        (60..=140).contains(&ones),
        "Expected ~50% ones, got {ones}/{total}"
    );
}

#[test]
fn test_from_circuit_bell_state() {
    use crate::circuit::circuit_impl::Circuit;
    let mut c = Circuit::new(2);
    c.h(Qubit::new(0)).unwrap();
    c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let stab = StabilizerState::from_circuit(&c).unwrap();
    let stab_strs: Vec<String> = stab
        .get_stabilizers()
        .iter()
        .map(|p| p.to_string())
        .collect();
    assert!(
        stab_strs.contains(&"+XX".to_string()),
        "got {:?}",
        stab_strs
    );
    assert!(
        stab_strs.contains(&"+ZZ".to_string()),
        "got {:?}",
        stab_strs
    );
}

#[test]
fn test_from_circuit_rejects_non_clifford() {
    use crate::circuit::circuit_impl::Circuit;
    let mut c = Circuit::new(1);
    c.t(Qubit::new(0)).unwrap(); // T gate is not Clifford
    let result = StabilizerState::from_circuit(&c);
    assert!(result.is_err(), "Expected error for T gate");
}

#[test]
fn test_measure_zero_state_deterministic() {
    // |0⟩ measured always gives 0
    let mut s = StabilizerState::new(1);
    let result = s.measure(0).unwrap();
    assert!(!result, "Expected 0 from |0⟩");
    // State collapses to |0⟩: stabilizer still +Z
    let stabs = s.get_stabilizers();
    assert_eq!(stabs[0].to_string(), "+Z");
}

#[test]
fn test_measure_one_state_deterministic() {
    // X|0⟩ = |1⟩ measured always gives 1
    let mut s = StabilizerState::new(1);
    s.apply_x(0).unwrap();
    let result = s.measure(0).unwrap();
    assert!(result, "Expected 1 from |1⟩");
}

#[test]
fn test_measure_plus_state_random() {
    // |+⟩ should give 0 or 1 randomly. Run many trials, both should appear.
    let mut saw_zero = false;
    let mut saw_one = false;
    for _ in 0..100 {
        let mut s = StabilizerState::new(1);
        s.apply_h(0).unwrap();
        let b = s.measure(0).unwrap();
        if b {
            saw_one = true;
        } else {
            saw_zero = true;
        }
        if saw_zero && saw_one {
            break;
        }
    }
    assert!(saw_zero, "Never saw outcome 0 from |+⟩");
    assert!(saw_one, "Never saw outcome 1 from |+⟩");
}

#[test]
fn test_measure_collapses_superposition() {
    // After measuring |+⟩, the state is deterministic: measure again gives same result.
    let mut s = StabilizerState::new(1);
    s.apply_h(0).unwrap();
    let b1 = s.measure(0).unwrap();
    let b2 = s.measure(0).unwrap();
    assert_eq!(b1, b2, "Repeated measurement should be consistent");
}

#[test]
fn test_measure_bell_state_correlated() {
    // |Φ+⟩ = (|00⟩+|11⟩)/√2: measuring q0 and q1 gives same result.
    for _ in 0..20 {
        let mut s = StabilizerState::new(2);
        s.apply_h(0).unwrap();
        s.apply_cx(0, 1).unwrap();
        let b0 = s.measure(0).unwrap();
        let b1 = s.measure(1).unwrap();
        assert_eq!(b0, b1, "Bell state measurements must be correlated");
    }
}

#[test]
fn test_measure_all() {
    let mut s = StabilizerState::new(2);
    let result = s.measure_all();
    assert!(
        !result.is_one(0) && !result.is_one(1),
        "|00⟩ measures to 00"
    );
}

#[test]
fn test_clone() {
    let s = StabilizerState::new(3);
    let c = s.clone();
    assert_eq!(s.num_qubits, c.num_qubits);
    assert_eq!(s.row_len, c.row_len);
    assert_eq!(s.tableau.len(), c.tableau.len());
    assert_eq!(&*s.tableau, &*c.tableau);
    assert_eq!(&*s.phases, &*c.phases);
}

/// Verifies that the SIMD-dispatched rowsum produces the same tableau as
/// sequential gate application for a large (≥ 64 qubit) state, exercising
/// the full word-level popcount phase path and SIMD XOR loop.
#[test]
fn test_rowsum_simd_large_state() {
    // n=128 forces row_len=2 data words + 6 padding = 8, exercising SIMD paths.
    let n = 128;
    let mut s = StabilizerState::new(n);

    // GHZ preparation: H on qubit 0, then CNOT(0→k) for all k.
    // Result: (|00...0⟩ + |11...1⟩)/√2 — all qubits fully correlated.
    s.apply_h(0).unwrap();
    for q in 1..n {
        s.apply_cx(0, q).unwrap(); // rowsum exercises SIMD on wide rows
    }

    // All measurements must agree (all-0 or all-1).
    let outcome0 = s.measure(0).unwrap();
    for q in 1..n {
        let outcome_q = s.measure(q).unwrap();
        assert_eq!(
            outcome_q, outcome0,
            "GHZ correlation broken at qubit {q}: expected {outcome0}, got {outcome_q}"
        );
    }
}

/// Verifies g_phase_word matches the scalar g_phase on random inputs.
#[test]
fn test_g_phase_word_matches_scalar() {
    // Construct deterministic test patterns covering all 4 Pauli types.
    // Pattern: qubit 0 = I, qubit 1 = X, qubit 2 = Y, qubit 3 = Z.
    // Encode as bit 0..3 of a single u64 word.
    // h-row: xh = 0b0110 (bits 1,2 set), zh = 0b0100_u64 | 0b1000 (bits 2,3 set)
    let xh: u64 = 0b0110; // bits 1,2 → qubits 1(X) and 2(Y)
    let zh: u64 = 0b1100; // bits 2,3 → qubits 2(Y) and 3(Z)

    // i-row: all four combinations across bits 0..3.
    let xi: u64 = 0b1010; // bits 1,3
    let zi: u64 = 0b0011; // bits 0,1

    // Compute expected sum via per-bit scalar g_phase.
    let mut expected = 0i32;
    for bit in 0..4 {
        let x1 = (xh >> bit) & 1 == 1;
        let z1 = (zh >> bit) & 1 == 1;
        let x2 = (xi >> bit) & 1 == 1;
        let z2 = (zi >> bit) & 1 == 1;
        expected += StabilizerState::g_phase(x1, z1, x2, z2);
    }

    let got = StabilizerState::g_phase_word(xh, zh, xi, zi);
    assert_eq!(got, expected, "g_phase_word mismatch");
}

/// Exhaustively checks g_phase_word against scalar g_phase for all 16
/// combinations of single-bit (xh, zh, xi, zi) input patterns.
#[test]
fn test_g_phase_word_exhaustive() {
    for xh in 0u64..=1 {
        for zh in 0u64..=1 {
            for xi in 0u64..=1 {
                for zi in 0u64..=1 {
                    let scalar = StabilizerState::g_phase(xh == 1, zh == 1, xi == 1, zi == 1);
                    let word = StabilizerState::g_phase_word(xh, zh, xi, zi);
                    assert_eq!(
                        word, scalar,
                        "g_phase_word({xh},{zh},{xi},{zi}) = {word} ≠ {scalar}"
                    );
                }
            }
        }
    }
}

/// sample_shots on Bell state: all outcomes must be (0,0) or (1,1), ~50/50.
#[test]
fn test_sample_shots_bell_state() {
    let mut s = StabilizerState::new(2);
    s.apply_h(0).unwrap();
    s.apply_cx(0, 1).unwrap(); // |Φ⁺⟩ = (|00⟩+|11⟩)/√2

    let shots = 500;
    let results = s.sample_shots(shots);
    assert_eq!(results.len(), shots);

    let mut zeros = 0usize;
    for shot in &results {
        assert_eq!(
            shot.is_one(0),
            shot.is_one(1),
            "Bell state: qubits must agree"
        );
        if !shot.is_one(0) {
            zeros += 1;
        }
    }
    // Expect roughly 50% zeros; accept anything in [15%, 85%] for robustness.
    let ratio = zeros as f64 / shots as f64;
    assert!(
        (0.15..=0.85).contains(&ratio),
        "Expected ~50% |00⟩, got {zeros}/{shots} = {ratio:.2}"
    );
}

/// sample_shots on deterministic |0⟩ state: all outcomes must be |000⟩.
#[test]
fn test_sample_shots_deterministic() {
    let s = StabilizerState::new(3);
    let results = s.sample_shots(200);
    for shot in &results {
        assert!(
            !shot.is_one(0) && !shot.is_one(1) && !shot.is_one(2),
            "Expected |000⟩"
        );
    }
}

/// X2P⁴ = X2M⁴ = identity (4 applications restore the original state).
#[test]
fn test_x2p_four_is_identity() {
    let original = StabilizerState::new(2);
    let mut s = StabilizerState::new(2);
    for _ in 0..4 {
        s.apply_x2p(0).unwrap();
    }
    assert_eq!(&*s.tableau, &*original.tableau);
    assert_eq!(&*s.phases, &*original.phases);
}

#[test]
fn test_x2m_four_is_identity() {
    let original = StabilizerState::new(2);
    let mut s = StabilizerState::new(2);
    for _ in 0..4 {
        s.apply_x2m(0).unwrap();
    }
    assert_eq!(&*s.tableau, &*original.tableau);
    assert_eq!(&*s.phases, &*original.phases);
}

/// Y2P⁴ = Y2M⁴ = identity.
#[test]
fn test_y2p_four_is_identity() {
    let original = StabilizerState::new(2);
    let mut s = StabilizerState::new(2);
    for _ in 0..4 {
        s.apply_y2p(0).unwrap();
    }
    assert_eq!(&*s.tableau, &*original.tableau);
    assert_eq!(&*s.phases, &*original.phases);
}

#[test]
fn test_y2m_four_is_identity() {
    let original = StabilizerState::new(2);
    let mut s = StabilizerState::new(2);
    for _ in 0..4 {
        s.apply_y2m(0).unwrap();
    }
    assert_eq!(&*s.tableau, &*original.tableau);
    assert_eq!(&*s.phases, &*original.phases);
}

/// X2P followed by X2M = identity.
#[test]
fn test_x2p_x2m_inverse() {
    let original = StabilizerState::new(1);
    let mut s = StabilizerState::new(1);
    s.apply_x2p(0).unwrap();
    s.apply_x2m(0).unwrap();
    assert_eq!(&*s.tableau, &*original.tableau);
    assert_eq!(&*s.phases, &*original.phases);
}

/// Y2P followed by Y2M = identity.
#[test]
fn test_y2p_y2m_inverse() {
    let original = StabilizerState::new(1);
    let mut s = StabilizerState::new(1);
    s.apply_y2p(0).unwrap();
    s.apply_y2m(0).unwrap();
    assert_eq!(&*s.tableau, &*original.tableau);
    assert_eq!(&*s.phases, &*original.phases);
}

/// X2P² = X (two √X gates give X gate).
#[test]
fn test_x2p_squared_is_x() {
    let mut after_x2p2 = StabilizerState::new(2);
    after_x2p2.apply_x2p(0).unwrap();
    after_x2p2.apply_x2p(0).unwrap();

    let mut after_x = StabilizerState::new(2);
    after_x.apply_x(0).unwrap();

    assert_eq!(&*after_x2p2.tableau, &*after_x.tableau);
    assert_eq!(&*after_x2p2.phases, &*after_x.phases);
}

/// Y2P² = Y (two √Y gates give Y gate).
#[test]
fn test_y2p_squared_is_y() {
    let mut after_y2p2 = StabilizerState::new(2);
    after_y2p2.apply_y2p(0).unwrap();
    after_y2p2.apply_y2p(0).unwrap();

    let mut after_y = StabilizerState::new(2);
    after_y.apply_y(0).unwrap();

    assert_eq!(&*after_y2p2.tableau, &*after_y.tableau);
    assert_eq!(&*after_y2p2.phases, &*after_y.phases);
}

/// NonCliffordGate returned for T gate (verify renamed error variant).
#[test]
fn test_non_clifford_error_variant() {
    use crate::circuit::circuit_impl::Circuit;
    let mut c = Circuit::new(1);
    c.t(Qubit::new(0)).unwrap();
    let result = StabilizerState::from_circuit(&c);
    assert!(
        matches!(result, Err(QisError::NonCliffordGate(_))),
        "Expected NonCliffordGate, got {result:?}"
    );
}

/// |0⟩ state: probability_of([false]) = 1.0, probability_of([true]) = 0.0.
#[test]
fn test_probability_of_zero_state() {
    let s = StabilizerState::new(1);
    assert!((s.probability_of(&[false]).unwrap() - 1.0).abs() < 1e-12);
    assert_eq!(s.probability_of(&[true]).unwrap(), 0.0);
}

/// |+⟩ state: both outcomes equally probable.
#[test]
fn test_probability_of_plus_state() {
    let mut s = StabilizerState::new(1);
    s.apply_h(0).unwrap();
    assert!((s.probability_of(&[false]).unwrap() - 0.5).abs() < 1e-12);
    assert!((s.probability_of(&[true]).unwrap() - 0.5).abs() < 1e-12);
}

/// Bell state |Φ⁺⟩: only |00⟩ and |11⟩ have nonzero probability (0.5 each).
#[test]
fn test_probability_of_bell_state() {
    let mut s = StabilizerState::new(2);
    s.apply_h(0).unwrap();
    s.apply_cx(0, 1).unwrap();
    assert!((s.probability_of(&[false, false]).unwrap() - 0.5).abs() < 1e-12);
    assert!((s.probability_of(&[true, true]).unwrap() - 0.5).abs() < 1e-12);
    assert_eq!(s.probability_of(&[false, true]).unwrap(), 0.0);
    assert_eq!(s.probability_of(&[true, false]).unwrap(), 0.0);
}

/// `probability_of` is non-destructive: calling it does not change the state.
#[test]
fn test_probability_of_is_non_destructive() {
    let mut s = StabilizerState::new(2);
    s.apply_h(0).unwrap();
    s.apply_cx(0, 1).unwrap();

    let stabs_before = s.get_stabilizers();
    let _ = s.probability_of(&[false, false]).unwrap();
    let stabs_after = s.get_stabilizers();
    assert_eq!(stabs_before, stabs_after);
}

/// probabilities() sums to 1.0 for a 3-qubit GHZ state.
#[test]
fn test_probabilities_ghz_sums_to_one() {
    let mut s = StabilizerState::new(3);
    s.apply_h(0).unwrap();
    s.apply_cx(0, 1).unwrap();
    s.apply_cx(0, 2).unwrap();
    let probs = s.probabilities().unwrap();
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-10, "sum = {sum}");
    // Only |000⟩ (index 0) and |111⟩ (index 7) have nonzero probability
    assert!((probs[0b000] - 0.5).abs() < 1e-12);
    assert!((probs[0b111] - 0.5).abs() < 1e-12);
    for i in [0b001usize, 0b010, 0b011, 0b100, 0b101, 0b110] {
        assert_eq!(probs[i], 0.0, "probs[{i:03b}] = {}", probs[i]);
    }
}

/// probabilities() returns Err for n > 20.
#[test]
fn test_probabilities_rejects_large_n() {
    let s = StabilizerState::new(21);
    assert!(matches!(
        s.probabilities(),
        Err(QisError::InvalidParameterValue(_))
    ));
}

/// probability_of returns QubitMismatch for wrong bits length.
#[test]
fn test_probability_of_qubit_mismatch() {
    let s = StabilizerState::new(3);
    let result = s.probability_of(&[false, false]); // only 2 bits for 3-qubit state
    assert!(matches!(
        result,
        Err(QisError::QubitMismatch {
            expected: 3,
            actual: 2
        })
    ));
}

/// SIMD boundary test: n = 63, 64, 65.
///
/// These are the most likely sizes to trigger off-by-one errors or incorrect
/// padding in the row_len calculation, especially when row data spans two
/// 64-word blocks. We apply a CX gate across qubit 0 → qubit n-1 and verify
/// the resulting stabilizers are consistent with a maximally entangled state.
#[test]
fn test_simd_boundary_n63() {
    let n = 63;
    let mut s = StabilizerState::new(n);
    s.apply_h(0).unwrap();
    for q in 0..n - 1 {
        s.apply_cx(q, q + 1).unwrap();
    }
    // All qubits must measure identically (GHZ-like)
    let mut copy = s.clone();
    let result = copy.measure_all();
    let first = result.is_one(0);
    assert!(
        (0..n).all(|q| result.is_one(q) == first),
        "n=63: not all qubits equal"
    );
}

#[test]
fn test_simd_boundary_n64() {
    let n = 64;
    let mut s = StabilizerState::new(n);
    s.apply_h(0).unwrap();
    for q in 0..n - 1 {
        s.apply_cx(q, q + 1).unwrap();
    }
    let mut copy = s.clone();
    let result = copy.measure_all();
    let first = result.is_one(0);
    assert!(
        (0..n).all(|q| result.is_one(q) == first),
        "n=64: not all qubits equal"
    );
}

#[test]
fn test_simd_boundary_n65() {
    let n = 65;
    let mut s = StabilizerState::new(n);
    s.apply_h(0).unwrap();
    for q in 0..n - 1 {
        s.apply_cx(q, q + 1).unwrap();
    }
    let mut copy = s.clone();
    let result = copy.measure_all();
    let first = result.is_one(0);
    assert!(
        (0..n).all(|q| result.is_one(q) == first),
        "n=65: not all qubits equal"
    );
}

/// Repeated deterministic measurements must not contaminate each other.
///
/// After measuring |0...0⟩, every qubit is deterministically 0. The scratch row
/// is cleared between measurements, so the second measurement must also return 0.
#[test]
fn test_repeated_deterministic_measurements() {
    let mut s = StabilizerState::new(4);
    // First pass: all deterministically 0
    for q in 0..4 {
        assert!(!s.measure(q).unwrap(), "qubit {q} first pass");
    }
    // Second pass: state now collapsed but still |0000⟩, all deterministic
    for q in 0..4 {
        assert!(!s.measure(q).unwrap(), "qubit {q} second pass");
    }
}

/// Deep circuit collapse: chain CX, measure middle qubit, verify stabilizer split.
///
/// Prepare |+⟩^n → apply chain CNOT 0→1→2→…→n-1 → measure qubit n/2.
/// CY conjugation sign: H(0) then CY(0,1) on |+0⟩.
///
/// |+0⟩ has stabilizer +XI.
/// After CY(0,1): the stabilizer must map as X₀⊗I₁ → X₀⊗Y₁ (= +YX in display order).
/// The *wrong* implementation (S before CX) would yield −YX instead.
#[test]
fn test_cy_conjugation_sign() {
    use crate::qis::pauli::{Pauli, PauliString, Phase};
    let mut s = StabilizerState::new(2);
    s.apply_h(0).unwrap();
    s.apply_cy(0, 1).unwrap();
    // Stabilizers: +YX (from X₀I₁ → X₀Y₁) and +ZZ (from Z₀I₁ → Z₀Z₁)
    let stab_strs: Vec<String> = s.get_stabilizers().iter().map(|p| p.to_string()).collect();
    assert!(
        stab_strs.contains(&"+YX".to_string()),
        "apply_cy sign bug: expected +YX among stabilizers, got {:?}",
        stab_strs
    );
    // Verify directly via pauli_expectation
    let mut yx = PauliString::new(2);
    yx.set_pauli(0, Pauli::X);
    yx.set_pauli(1, Pauli::Y);
    yx.phase = Phase::Plus;
    assert_eq!(
        s.pauli_expectation(&yx).unwrap(),
        1,
        "apply_cy: +YX should have expectation +1"
    );
    let mut neg_yx = PauliString::new(2);
    neg_yx.set_pauli(0, Pauli::X);
    neg_yx.set_pauli(1, Pauli::Y);
    neg_yx.phase = Phase::Minus;
    assert_eq!(
        s.pauli_expectation(&neg_yx).unwrap(),
        -1,
        "apply_cy: -YX should have expectation -1"
    );
}

/// Bell state: generators are +XX and +ZZ.
/// Their product is -YY (since iZ·iX = -ZX = -iY per Pauli algebra, but more directly
/// XX·ZZ = (XZ)⊗(XZ) = (-iY)⊗(-iY) = i²·YY = -YY).
/// So ⟨Φ+|YY|Φ+⟩ = -1 and ⟨Φ+|(-YY)|Φ+⟩ = +1.
/// The old implementation returned 0 for both (not a generator match).
#[test]
fn test_pauli_expectation_product_yy_bell() {
    use crate::qis::pauli::{Pauli, PauliString, Phase};
    let mut s = StabilizerState::new(2);
    s.apply_h(0).unwrap();
    s.apply_cx(0, 1).unwrap();
    // +YY
    let mut yy = PauliString::new(2);
    yy.set_pauli(0, Pauli::Y);
    yy.set_pauli(1, Pauli::Y);
    yy.phase = Phase::Plus;
    assert_eq!(
        s.pauli_expectation(&yy).unwrap(),
        -1,
        "Bell state: ⟨+YY⟩ should be -1"
    );
    // -YY
    let mut neg_yy = PauliString::new(2);
    neg_yy.set_pauli(0, Pauli::Y);
    neg_yy.set_pauli(1, Pauli::Y);
    neg_yy.phase = Phase::Minus;
    assert_eq!(
        s.pauli_expectation(&neg_yy).unwrap(),
        1,
        "Bell state: ⟨-YY⟩ should be +1"
    );
}

/// Bell state: +ZI is not in the stabilizer group (expectation = 0).
#[test]
fn test_pauli_expectation_not_in_group_bell() {
    use crate::qis::pauli::{Pauli, PauliString, Phase};
    let mut s = StabilizerState::new(2);
    s.apply_h(0).unwrap();
    s.apply_cx(0, 1).unwrap();
    let mut zi = PauliString::new(2);
    zi.set_pauli(0, Pauli::Z);
    zi.set_pauli(1, Pauli::I);
    zi.phase = Phase::Plus;
    assert_eq!(
        s.pauli_expectation(&zi).unwrap(),
        0,
        "Bell state: ⟨ZI⟩ should be 0"
    );
}

/// Bell state: direct generator +XX should still return +1.
#[test]
fn test_pauli_expectation_direct_generator_bell() {
    use crate::qis::pauli::{Pauli, PauliString, Phase};
    let mut s = StabilizerState::new(2);
    s.apply_h(0).unwrap();
    s.apply_cx(0, 1).unwrap();
    let mut xx = PauliString::new(2);
    xx.set_pauli(0, Pauli::X);
    xx.set_pauli(1, Pauli::X);
    xx.phase = Phase::Plus;
    assert_eq!(
        s.pauli_expectation(&xx).unwrap(),
        1,
        "Bell state: ⟨XX⟩ should be +1"
    );
}

/// row_to_pauli_string should propagate the full 4-phase.
/// After creating a state with an imaginary-phase row (via rowsum of anti-commuting rows),
/// get_destabilizers() must display the correct ±i phase, not garbage.
///
/// Concrete construction: start with |0⟩ (destabilizer = +X, stabilizer = +Z).
/// rowsum(destab_row=0, stab_row=1) sets destabilizer to X·Z with accumulated phase.
/// X·Z = -iY (g_phase(X, Z) = -1), so net phase = 0 + 0 + (-1) → (−1 mod 4) = 3 → -i.
#[test]
fn test_row_to_pauli_string_imaginary_phase() {
    let mut s = StabilizerState::new(1);
    // Manually call rowsum(0, 1): destab row 0 += stab row 1
    // destab row 0: X (phase 0), stab row 1: Z (phase 0)
    // product: XZ = -iY → phase 3
    s.rowsum(0, 1);
    let destabs = s.get_destabilizers();
    // Phase should be 3 (-i) and Pauli Y
    assert_eq!(
        destabs[0].to_string(),
        "-iY",
        "rowsum destab: expected -iY, got {}",
        destabs[0]
    );
}

/// After measurement the qubits on each side remain entangled within their group
/// but are now in a product state across the cut.  Both halves' measure_all
/// calls should yield internally-uniform bitstrings.
#[test]
fn test_deep_circuit_collapse() {
    let n = 10;
    let mid = n / 2;
    let mut s = StabilizerState::new(n);
    // Create a chain-entangled state
    s.apply_h(0).unwrap();
    for q in 0..n - 1 {
        s.apply_cx(q, q + 1).unwrap();
    }
    // Measure the middle qubit — collapses the global GHZ
    let _mid_outcome = s.measure(mid).unwrap();
    // After collapse, remaining qubits must still agree with each other
    // (the chain forces all qubits to the same value in a GHZ)
    let rest: Vec<bool> = (0..n)
        .filter(|&q| q != mid)
        .map(|q| s.clone().measure(q).unwrap())
        .collect();
    // All remaining outcomes must equal the first remaining qubit's value
    let first = rest[0];
    assert!(
        rest.iter().all(|&b| b == first),
        "deep collapse: inconsistent outcomes"
    );
}
/// reset() on |1⟩ returns qubit to |0⟩.
#[test]
fn test_reset_from_one() {
    let mut s = StabilizerState::new(1);
    s.apply_x(0).unwrap(); // |0⟩ → |1⟩
    s.reset(0).unwrap(); // → |0⟩
    assert!(!s.measure(0).unwrap(), "after reset should be |0⟩");
}

/// reset() on |0⟩ is a no-op.
#[test]
fn test_reset_from_zero() {
    let mut s = StabilizerState::new(1);
    s.reset(0).unwrap();
    assert!(!s.measure(0).unwrap());
}

/// reset() on a superposition state always yields |0⟩.
#[test]
fn test_reset_from_superposition() {
    for _ in 0..20 {
        let mut s = StabilizerState::new(1);
        s.apply_h(0).unwrap(); // |+⟩ random collapse
        s.reset(0).unwrap(); // must always be |0⟩ after
        assert!(!s.measure(0).unwrap());
    }
}

/// Mid-circuit measurement records the result by `ClassicalValue`.
#[test]
fn test_apply_circuit_measure_bit_result() {
    use crate::circuit::circuit_impl::Circuit;
    use crate::qis::RuntimeValue;

    // |1⟩: X then Measure should always record true.
    let mut c = Circuit::new(1);
    c.x(Qubit::new(0)).unwrap();
    let measured = c.measure(Qubit::new(0)).unwrap();

    let result = StabilizerState::run_circuit(&c).unwrap();
    assert_eq!(
        result.classical.value(measured.value()),
        Some(&RuntimeValue::Bit(true)),
        "X → Measure should record |1⟩"
    );
}

#[test]
fn test_apply_circuit_classical_result_rejects_other_circuit_handles() {
    use crate::circuit::{Circuit, ClassicalType};

    let mut circuit = Circuit::new(1);
    let target = circuit.var(ClassicalType::Bit);
    let measured = circuit.measure_into(Qubit::new(0), target).unwrap();

    let mut other = Circuit::new(1);
    let other_target = other.var(ClassicalType::Bit);
    let other_measured = other.measure_into(Qubit::new(0), other_target).unwrap();

    let result = StabilizerState::run_circuit(&circuit).unwrap();
    assert!(result.classical.value(measured.value()).is_some());
    assert!(result.classical.var(target).is_some());
    assert_eq!(result.classical.value(other_measured.value()), None);
    assert_eq!(result.classical.var(other_target), None);
}

/// Repeated measurements of the same qubit are separate `ClassicalValue`s.
#[test]
fn test_apply_circuit_repeated_measurements_keep_distinct_values() {
    use crate::circuit::circuit_impl::Circuit;
    use crate::qis::RuntimeValue;

    let mut c = Circuit::new(1);
    c.x(Qubit::new(0)).unwrap();
    let first = c.measure(Qubit::new(0)).unwrap();
    c.reset(Qubit::new(0)).unwrap();
    let second = c.measure(Qubit::new(0)).unwrap();

    let result = StabilizerState::run_circuit(&c).unwrap();
    assert_eq!(
        result.classical.value(first.value()),
        Some(&RuntimeValue::Bit(true))
    );
    assert_eq!(
        result.classical.value(second.value()),
        Some(&RuntimeValue::Bit(false))
    );
}

/// Entangled measurements are written as separate values, but their quantum
/// correlation must still be visible through the recorded classical data.
#[test]
fn test_apply_circuit_bell_measurements_are_correlated_in_classical_state() {
    use crate::circuit::circuit_impl::Circuit;
    use crate::qis::RuntimeValue;

    let mut c = Circuit::new(2);
    c.h(Qubit::new(0)).unwrap();
    c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let left = c.measure(Qubit::new(0)).unwrap();
    let right = c.measure(Qubit::new(1)).unwrap();

    let result = StabilizerState::run_circuit(&c).unwrap();
    let Some(RuntimeValue::Bit(left)) = result.classical.value(left.value()) else {
        panic!("expected first Bell measurement to produce a bit");
    };
    let Some(RuntimeValue::Bit(right)) = result.classical.value(right.value()) else {
        panic!("expected second Bell measurement to produce a bit");
    };

    assert_eq!(
        left, right,
        "Bell-state measurements must remain correlated after being stored"
    );
}

/// A circuit measurement is not just metadata: it collapses the stabilizer
/// state immediately, so later quantum operations must see the collapsed state.
#[test]
fn test_apply_circuit_measurement_collapse_affects_later_gates() {
    use crate::circuit::circuit_impl::Circuit;
    use crate::qis::RuntimeValue;

    let mut c = Circuit::new(2);
    c.h(Qubit::new(0)).unwrap();
    c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let first = c.measure(Qubit::new(0)).unwrap();
    c.x(Qubit::new(1)).unwrap();
    let second = c.measure(Qubit::new(1)).unwrap();

    let result = StabilizerState::run_circuit(&c).unwrap();
    let Some(RuntimeValue::Bit(first)) = result.classical.value(first.value()) else {
        panic!("expected first measurement to produce a bit");
    };
    let Some(RuntimeValue::Bit(second)) = result.classical.value(second.value()) else {
        panic!("expected second measurement to produce a bit");
    };

    assert_ne!(
        first, second,
        "after measuring one Bell qubit, X on the other qubit must flip the correlated value"
    );
}

/// Multi-qubit measurement follows `measure_bits` little-endian input order.
#[test]
fn test_apply_circuit_measure_bits_result_order() {
    use crate::circuit::{Circuit, Qubit};
    use crate::qis::RuntimeValue;

    let mut c = Circuit::new(3);
    c.x(Qubit::new(0)).unwrap();
    c.x(Qubit::new(2)).unwrap();
    let measured = c
        .measure_bits([Qubit::new(2), Qubit::new(1), Qubit::new(0)])
        .unwrap();

    let result = StabilizerState::run_circuit(&c).unwrap();
    let Some(RuntimeValue::BitVec { width, bits }) = result.classical.value(measured.value())
    else {
        panic!("expected BitVec measurement result");
    };
    assert_eq!(*width, 3);
    assert_eq!(bits.to_bitstring(*width as usize), "101");
    assert!(bits.is_one(0), "first measured qubit maps to bit 0");
    assert!(!bits.is_one(1));
    assert!(bits.is_one(2));
}

/// `measure_bits_into` keeps the immutable measurement result and stores an
/// identical `BitVec` copy in the target variable.
#[test]
fn test_apply_circuit_measure_bits_into_keeps_result_and_var_copy() {
    use crate::circuit::{Circuit, ClassicalType, Qubit};
    use crate::qis::RuntimeValue;

    let mut c = Circuit::new(3);
    c.x(Qubit::new(0)).unwrap();
    c.x(Qubit::new(2)).unwrap();
    let target = c.var(ClassicalType::bit_vec(3).unwrap());
    let measured = c
        .measure_bits_into([Qubit::new(0), Qubit::new(1), Qubit::new(2)], target)
        .unwrap();

    let result = StabilizerState::run_circuit(&c).unwrap();
    let Some(RuntimeValue::BitVec { width, bits }) = result.classical.value(measured.value())
    else {
        panic!("expected BitVec measurement result");
    };
    let value = RuntimeValue::BitVec {
        width: *width,
        bits: bits.clone(),
    };

    assert_eq!(*width, 3);
    assert_eq!(
        value.to_bitstring().as_deref(),
        Some("101"),
        "display order is MSB-left, while measured qubits map to bit indexes 0.."
    );
    assert_eq!(result.classical.var(target), Some(&value));
}

/// `measure_into` records the immutable measurement and the mutable variable copy.
#[test]
fn test_apply_circuit_measure_into_updates_classical_variable() {
    use crate::circuit::{Circuit, ClassicalType};
    use crate::qis::RuntimeValue;

    let mut c = Circuit::new(1);
    c.x(Qubit::new(0)).unwrap();
    let var = c.var(ClassicalType::Bit);
    let measured = c.measure_into(Qubit::new(0), var).unwrap();

    let result = StabilizerState::run_circuit(&c).unwrap();
    assert_eq!(
        result.classical.value(measured.value()),
        Some(&RuntimeValue::Bit(true))
    );
    assert_eq!(result.classical.var(var), Some(&RuntimeValue::Bit(true)));
}

/// Store expressions are evaluated against runtime measurement values.
#[test]
fn test_apply_circuit_store_expression_updates_classical_variable() {
    use crate::circuit::{Circuit, ClassicalExpr, ClassicalType};
    use crate::qis::RuntimeValue;

    let mut c = Circuit::new(1);
    c.x(Qubit::new(0)).unwrap();
    let measured = c.measure(Qubit::new(0)).unwrap();
    let flag = c.var(ClassicalType::Bool);
    c.store(flag, ClassicalExpr::bit_to_bool(measured.expr()).unwrap())
        .unwrap();

    let result = StabilizerState::run_circuit(&c).unwrap();
    assert_eq!(result.classical.var(flag), Some(&RuntimeValue::Bool(true)));
}

/// Reset directive in circuit actually resets the qubit.
#[test]
fn test_apply_circuit_reset_directive() {
    use crate::circuit::circuit_impl::Circuit;
    use crate::qis::RuntimeValue;

    // X then Reset: final state should be |0⟩.
    let mut c = Circuit::new(1);
    c.x(Qubit::new(0)).unwrap();
    c.reset(Qubit::new(0)).unwrap();
    let measured = c.measure(Qubit::new(0)).unwrap();

    let result = StabilizerState::run_circuit(&c).unwrap();
    assert_eq!(
        result.classical.value(measured.value()),
        Some(&RuntimeValue::Bit(false)),
        "X → Reset → Measure should yield |0⟩"
    );
}

/// from_circuit() is a state-construction API: measurement declarations are
/// ignored and do not collapse the final quantum state.
#[test]
fn test_from_circuit_ignores_terminal_measurement_declarations() {
    use crate::circuit::{Circuit, Qubit};
    use crate::device::Outcome;

    let mut c = Circuit::new(2);
    c.h(Qubit::new(0)).unwrap();
    c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let out = c.measure_bits([Qubit::new(1), Qubit::new(0)]).unwrap();

    let state = StabilizerState::from_circuit(&c).unwrap();
    let probs = state.probs(&out).unwrap();

    assert_eq!(probs.len(), 2);
    assert!((probs[&Outcome::from_bitstring("00").unwrap()] - 0.5).abs() < 1e-12);
    assert!((probs[&Outcome::from_bitstring("11").unwrap()] - 0.5).abs() < 1e-12);
}

#[test]
fn test_sample_uses_measurement_qubit_order() {
    use crate::circuit::{Circuit, Qubit};
    use crate::device::{Outcome, Status};

    let mut c = Circuit::new(2);
    c.x(Qubit::new(0)).unwrap();
    let out = c.measure_bits([Qubit::new(1), Qubit::new(0)]).unwrap();

    let state = StabilizerState::from_circuit(&c).unwrap();
    let result = state.sample(&out, 16).unwrap();

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

    let mut c = Circuit::new(2);
    c.h(Qubit::new(0)).unwrap();
    c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let out = c.measure(Qubit::new(0)).unwrap();

    let state = StabilizerState::from_circuit(&c).unwrap();
    let probs = state.probs(&out).unwrap();

    assert_eq!(probs.len(), 2);
    assert!((probs[&Outcome::from_bitstring("0").unwrap()] - 0.5).abs() < 1e-12);
    assert!((probs[&Outcome::from_bitstring("1").unwrap()] - 0.5).abs() < 1e-12);
}

#[test]
fn test_sample_rejects_measurement_qubit_outside_state() {
    use crate::circuit::Circuit;

    let state = StabilizerState::new(1);
    let mut circuit = Circuit::new(3);
    let measurement = circuit.measure(Qubit::new(2)).unwrap();

    assert!(matches!(
        state.sample(&measurement, 1),
        Err(QisError::IndexOutOfBounds { index: 2, max: 0 })
    ));
}

#[test]
fn test_apply_circuit_rejects_classical_control_flow() {
    use crate::circuit::{Circuit, ClassicalExpr};
    use crate::qis::QisError;

    let mut c = Circuit::new(1);
    c.if_(ClassicalExpr::bool_literal(true), |body| {
        body.x(Qubit::new(0))?;
        Ok(())
    })
    .unwrap();

    let error = StabilizerState::run_circuit(&c).unwrap_err();
    assert!(matches!(error, QisError::UnsupportedOperation(_)));
}
