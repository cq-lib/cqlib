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

//! Tests for [`PauliEvolutionAnsatz`] and related types.
//!
//! # Test Categories
//!
//! 1. **Structural unit tests** — validate inputs, angles, step counts, commutativity.
//! 2. **Matrix-level mathematical verification** — for small (1–2 qubit) Hamiltonians,
//!    construct the circuit unitary and compare it to $e^{-iHt}$ computed analytically
//!    or via Taylor-series matrix exponentiation.

use std::collections::HashMap;

use approx::assert_abs_diff_eq;
use ndarray::Array2;
use num_complex::Complex64;

use crate::circuit::ansatz::Ansatz;
use crate::circuit::circuit_to_matrix;
use crate::qis::evolution::TrotterMode;
use crate::qis::hamiltonian::Hamiltonian;

use super::{EvolutionStrategy, PauliEvolutionAnsatz};

/// Builds the full Hamiltonian matrix $H = \sum_k c_k P_k$.
fn hamiltonian_matrix(h: &Hamiltonian) -> Array2<Complex64> {
    h.to_matrix()
        .expect("Hamiltonian evolution tests use dimensionally valid terms")
}

/// Computes $e^{-iHt}$ via Taylor series: $\sum_{k=0}^{K} \frac{(-iHt)^k}{k!}$.
///
/// Uses 60 terms, which gives double-precision accuracy for $\|H\| \cdot |t| \lesssim 10$.
fn matrix_exp_iht(h_mat: &Array2<Complex64>, t: f64) -> Array2<Complex64> {
    let n = h_mat.nrows();
    let neg_i = Complex64::new(0.0, -1.0);

    // iH * t (note: exponent is -iHt so we compute (-i*t)*H iteratively)
    let a: Array2<Complex64> = h_mat.mapv(|v| v * neg_i * t);

    let mut result = Array2::<Complex64>::eye(n); // I
    let mut term = Array2::<Complex64>::eye(n); // current power / k!

    for k in 1..=60usize {
        term = term.dot(&a) / Complex64::from(k as f64);
        result += &term;

        // Early termination if converged
        if term.iter().all(|v| v.norm() < 1e-16) {
            break;
        }
    }
    result
}

/// Frobenius distance between two matrices: $\|A - B\|_F$.
fn frob_dist(a: &Array2<Complex64>, b: &Array2<Complex64>) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).norm_sqr())
        .sum::<f64>()
        .sqrt()
}

/// Builds a circuit, assigns `t = t_val`, and returns the unitary matrix.
fn circuit_matrix_at_t(
    ansatz: &PauliEvolutionAnsatz,
    prefix: &str,
    t_name: &str,
    t_val: f64,
) -> Array2<Complex64> {
    let circuit = ansatz.build_circuit(prefix).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert(t_name, t_val);
    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    circuit_to_matrix(&bound, None).unwrap()
}

#[test]
fn test_new_simplifies_hamiltonian() {
    // Add two identical terms: should merge into one with double coefficient
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), Complex64::new(0.5, 0.0))
        .unwrap();
    h.add_term("Z".parse().unwrap(), Complex64::new(0.5, 0.0))
        .unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h).unwrap();
    // After simplify() inside new(), there must be exactly 1 term
    assert_eq!(ansatz.hamiltonian.terms.len(), 1);
    let (_, coeff) = &ansatz.hamiltonian.terms[0];
    assert_abs_diff_eq!(coeff.re, 1.0, epsilon = 1e-12);
}

#[test]
fn test_new_rejects_empty_hamiltonian() {
    let h = Hamiltonian::new(2);
    let result = PauliEvolutionAnsatz::new(h);
    assert!(result.is_err(), "Empty Hamiltonian should fail");
}

#[test]
fn test_new_rejects_non_hermitian_hamiltonian() {
    let mut h = Hamiltonian::new(1);
    // Imaginary coefficient → not Hermitian
    h.add_term("Z".parse().unwrap(), Complex64::new(0.0, 1.0))
        .unwrap();
    let result = PauliEvolutionAnsatz::new(h);
    assert!(
        result.is_err(),
        "Non-Hermitian Hamiltonian (imaginary coeff) should be rejected"
    );
}

#[test]
fn test_new_accepts_real_coeff_after_simplify() {
    // A PauliString with Phase::Minus: simplify() absorbs -1 into coefficient
    // Result: coeff = 0.5 * (-1) = -0.5 which is still real → Hermitian
    let mut ps: crate::qis::pauli::PauliString = "Z".parse().unwrap();
    use crate::qis::pauli::Phase;
    ps.phase = Phase::Minus; // phase = -1

    let mut h = Hamiltonian::new(1);
    h.add_term(ps, Complex64::new(0.5, 0.0)).unwrap();
    // After simplify inside new(): phase absorbed → coeff = -0.5 (real) → OK
    let result = PauliEvolutionAnsatz::new(h);
    assert!(
        result.is_ok(),
        "Hermitian after simplify() should be accepted"
    );
}

#[test]
fn test_all_terms_commute_true_for_diagonal_hamiltonian() {
    // ZZ and IZ both diagonal → commute
    let mut h = Hamiltonian::new(2);
    h.add_term("ZZ".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("IZ".parse().unwrap(), 0.5.into()).unwrap();
    assert!(h.all_terms_commute());
}

#[test]
fn test_all_terms_commute_false_for_x_z() {
    // X and Z anti-commute on the same qubit
    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();
    assert!(!h.all_terms_commute());
}

#[test]
fn test_all_terms_commute_empty_is_true() {
    // Vacuously true: no pairs to violate commutativity
    let h = Hamiltonian::new(2);
    assert!(h.all_terms_commute());
}

#[test]
fn test_exact_strategy_rejects_noncommuting_hamiltonian() {
    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h)
        .unwrap()
        .with_strategy(EvolutionStrategy::Exact);

    let err = ansatz.validate();
    assert!(
        err.is_err(),
        "Exact strategy on non-commuting H must return error"
    );
}

#[test]
fn test_auto_selects_exact_path_for_commuting_hamiltonian() {
    let mut h = Hamiltonian::new(2);
    h.add_term("ZZ".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("IZ".parse().unwrap(), 0.5.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h)
        .unwrap()
        .with_strategy(EvolutionStrategy::Auto { steps: 3 });

    let info = ansatz.evolution_info();
    assert!(info.is_exact, "Auto should choose exact for commuting H");
    assert!(info.all_terms_commute);
    assert_eq!(info.steps, 1, "Exact → 1 step");
    assert!(info.trotter_mode.is_none());
}

#[test]
fn test_auto_selects_trotter_for_noncommuting_hamiltonian() {
    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h)
        .unwrap()
        .with_strategy(EvolutionStrategy::Auto { steps: 5 });

    let info = ansatz.evolution_info();
    assert!(
        !info.is_exact,
        "Auto should choose Trotter for non-commuting H"
    );
    assert!(!info.all_terms_commute);
    assert_eq!(info.steps, 5);
}

#[test]
fn test_zero_steps_returns_error() {
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h)
        .unwrap()
        .with_strategy(EvolutionStrategy::Auto { steps: 0 });
    assert!(ansatz.validate().is_err(), "steps=0 must be rejected");

    let mut h2 = Hamiltonian::new(1);
    h2.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();
    let ansatz2 =
        PauliEvolutionAnsatz::new(h2)
            .unwrap()
            .with_strategy(EvolutionStrategy::Trotter {
                mode: TrotterMode::FirstOrder,
                steps: 0,
            });
    assert!(
        ansatz2.validate().is_err(),
        "Trotter steps=0 must be rejected"
    );
}

#[test]
fn test_num_parameters_always_one() {
    let mut h = Hamiltonian::new(2);
    h.add_term("ZZ".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("XX".parse().unwrap(), 0.5.into()).unwrap();

    for strategy in [
        EvolutionStrategy::Auto { steps: 3 },
        EvolutionStrategy::Trotter {
            mode: TrotterMode::FirstOrder,
            steps: 5,
        },
        EvolutionStrategy::Trotter {
            mode: TrotterMode::SecondOrder,
            steps: 2,
        },
    ] {
        let ansatz = PauliEvolutionAnsatz::new(h.clone())
            .unwrap()
            .with_strategy(strategy);
        assert_eq!(
            ansatz.num_parameters(),
            1,
            "PauliEvolutionAnsatz always has exactly 1 parameter"
        );
    }
}

#[test]
fn test_num_qubits_matches_hamiltonian() {
    for n in [1, 2, 3] {
        let mut h = Hamiltonian::new(n);
        let mut ps = crate::qis::pauli::PauliString::new(n);
        ps.set_pauli(0, crate::qis::pauli::Pauli::Z);
        h.add_term(ps, 1.0.into()).unwrap();

        let ansatz = PauliEvolutionAnsatz::new(h).unwrap();
        assert_eq!(ansatz.num_qubits(), n);
    }
}

#[test]
fn test_time_param_name_default_uses_prefix() {
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h).unwrap();
    let circuit = ansatz.build_circuit("myevo").unwrap();

    // Must contain a parameter named "myevo_t"
    let symbols: std::collections::HashSet<String> = circuit
        .parameters()
        .iter()
        .flat_map(|p| p.get_symbols())
        .collect();
    assert!(
        symbols.contains("myevo_t"),
        "Default param name must be '{{prefix}}_t'; got {:?}",
        symbols
    );
}

#[test]
fn test_time_param_name_override() {
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h)
        .unwrap()
        .with_time_param_name("tau");
    let circuit = ansatz.build_circuit("ignored_prefix").unwrap();

    let symbols: std::collections::HashSet<String> = circuit
        .parameters()
        .iter()
        .flat_map(|p| p.get_symbols())
        .collect();
    assert!(
        symbols.contains("tau"),
        "Explicit time param name 'tau' must appear; got {:?}",
        symbols
    );
    assert!(
        !symbols.contains("ignored_prefix_t"),
        "Prefix-based name must NOT appear when explicit name is set"
    );
}

/// Verifies that the Trotter-1 circuit has exactly `steps * num_terms` pauli_evolution
/// layers by checking the circuit gate count.
///
/// Each `pauli_evolution` call on an n-qubit Pauli string generates a block of gates
/// (basis transforms + CNOT ladder + RZ + reverse). For a 1-qubit Z string, each
/// `pauli_evolution` generates exactly 1 RZ gate, so the total RZ count == steps * n_terms.
#[test]
fn test_trotter1_gate_structure_1qubit() {
    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse().unwrap(), 0.5.into()).unwrap();
    h.add_term("Z".parse().unwrap(), 0.3.into()).unwrap();

    let steps = 4;
    let ansatz = PauliEvolutionAnsatz::new(h)
        .unwrap()
        .with_strategy(EvolutionStrategy::Trotter {
            mode: TrotterMode::FirstOrder,
            steps,
        });

    let circuit = ansatz.build_circuit("test").unwrap();

    // 1-qubit: X → [H, RZ, H]; Z → [RZ]. Total ops = steps * (3 + 1) = 16.
    // We just verify circuit is non-empty and has the right qubit count.
    assert_eq!(circuit.num_qubits(), 1);
    assert!(!circuit.operations().is_empty());
    // The number of "operations" in the circuit equals steps * (gates_for_X + gates_for_Z)
    // For X: H + RZ + H = 3 ops. For Z: RZ = 1 op. Total per step = 4. Total = 4*4 = 16.
    let expected_ops = steps * 4;
    assert_eq!(
        circuit.operations().len(),
        expected_ops,
        "Trotter-1 with steps={steps} on H=0.5X+0.3Z should produce {expected_ops} ops"
    );
}

/// Verifies that Suzuki-2 circuit has twice as many pauli_evolution passes as Trotter-1
/// per step (forward + backward). For equal steps, Suzuki-2 has 2x the gate count.
#[test]
fn test_trotter2_has_double_gate_count_vs_trotter1() {
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();

    let steps = 3;
    let trotter1_ansatz =
        PauliEvolutionAnsatz::new(h.clone())
            .unwrap()
            .with_strategy(EvolutionStrategy::Trotter {
                mode: TrotterMode::FirstOrder,
                steps,
            });
    let trotter2_ansatz =
        PauliEvolutionAnsatz::new(h)
            .unwrap()
            .with_strategy(EvolutionStrategy::Trotter {
                mode: TrotterMode::SecondOrder,
                steps,
            });

    let c1 = trotter1_ansatz.build_circuit("t1").unwrap();
    let c2 = trotter2_ansatz.build_circuit("t2").unwrap();

    assert_eq!(
        c2.operations().len(),
        2 * c1.operations().len(),
        "Suzuki-2 should have 2× the gate count of Trotter-1 for a single-term H"
    );
}

/// Verifies the angle convention: for H = c * Z, the circuit at time t must implement
/// e^{-i c t Z}. We check a specific parametric angle is 2*c*t by examining that
/// binding t=1 and running through assign_parameters produces fixed values that
/// satisfy the mathematical relation.
///
/// Specifically, for H = 0.7 * Z and t = 1.0:
/// The RZ angle should be 2 * 0.7 * 1.0 = 1.4 (since pauli_evolution angle = 2*c*t).
#[test]
fn test_angle_convention_single_qubit_z() {
    let coeff = 0.7_f64;
    let t_val = 1.0_f64;

    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), coeff.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h)
        .unwrap()
        .with_strategy(EvolutionStrategy::Exact);

    let circuit = ansatz.build_circuit("ang").unwrap();

    // Bind t = t_val
    let mut bindings = HashMap::new();
    bindings.insert("ang_t", t_val);
    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();

    // The circuit for a single Z term is: RZ(2 * coeff * t)
    // Check via matrix: e^{-i coeff t Z} = diag(e^{-i coeff t}, e^{+i coeff t})
    let expected_phase = -coeff * t_val;
    let expected_diag_0 = Complex64::from_polar(1.0, expected_phase);
    let expected_diag_1 = Complex64::from_polar(1.0, -expected_phase);

    let mat = circuit_to_matrix(&bound, None).unwrap();
    assert_abs_diff_eq!(mat[[0, 0]].re, expected_diag_0.re, epsilon = 1e-10);
    assert_abs_diff_eq!(mat[[0, 0]].im, expected_diag_0.im, epsilon = 1e-10);
    assert_abs_diff_eq!(mat[[1, 1]].re, expected_diag_1.re, epsilon = 1e-10);
    assert_abs_diff_eq!(mat[[1, 1]].im, expected_diag_1.im, epsilon = 1e-10);
    // Off-diagonal must be zero
    assert_abs_diff_eq!(mat[[0, 1]].norm(), 0.0, epsilon = 1e-10);
    assert_abs_diff_eq!(mat[[1, 0]].norm(), 0.0, epsilon = 1e-10);
}

/// Exact 1-qubit test: H = Z.
///
/// e^{-itZ} = diag(e^{-it}, e^{it}) — known analytically.
/// The Exact strategy must reproduce this with machine precision.
#[test]
fn test_matrix_exact_single_qubit_z() {
    let t_val = 0.8_f64;

    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h.clone())
        .unwrap()
        .with_strategy(EvolutionStrategy::Exact);

    let u_circuit = circuit_matrix_at_t(&ansatz, "ex", "ex_t", t_val);
    let h_mat = hamiltonian_matrix(&h);
    let u_exact = matrix_exp_iht(&h_mat, t_val);

    let dist = frob_dist(&u_circuit, &u_exact);
    assert!(
        dist < 1e-10,
        "Exact 1-qubit Z: Frobenius distance = {dist:.2e}, expected < 1e-10"
    );
}

/// Exact 1-qubit test: H = 0.5 * X.
///
/// e^{-i 0.5 t X} = cos(0.5t) I - i sin(0.5t) X (known analytically).
#[test]
fn test_matrix_exact_single_qubit_x() {
    let coeff = 0.5_f64;
    let t_val = 1.2_f64;

    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse().unwrap(), coeff.into()).unwrap();

    // X is a single term so it commutes vacuously with itself → Exact is valid
    let ansatz = PauliEvolutionAnsatz::new(h.clone())
        .unwrap()
        .with_strategy(EvolutionStrategy::Exact);

    let u_circuit = circuit_matrix_at_t(&ansatz, "ex", "ex_t", t_val);
    let h_mat = hamiltonian_matrix(&h);
    let u_exact = matrix_exp_iht(&h_mat, t_val);

    let dist = frob_dist(&u_circuit, &u_exact);
    assert!(
        dist < 1e-10,
        "Exact 1-qubit X: Frobenius distance = {dist:.2e}, expected < 1e-10"
    );
}

/// Exact 2-qubit test: H = ZZ + 0.5 IZ (both diagonal, mutually commuting).
///
/// Since the terms commute, the circuit must be exactly unitary-equivalent to e^{-iHt}.
#[test]
fn test_matrix_exact_commuting_2qubit() {
    let t_val = 0.6_f64;

    let mut h = Hamiltonian::new(2);
    h.add_term("ZZ".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("IZ".parse().unwrap(), 0.5.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h.clone())
        .unwrap()
        .with_strategy(EvolutionStrategy::Exact);

    let u_circuit = circuit_matrix_at_t(&ansatz, "c2", "c2_t", t_val);
    let h_mat = hamiltonian_matrix(&h);
    let u_exact = matrix_exp_iht(&h_mat, t_val);

    let dist = frob_dist(&u_circuit, &u_exact);
    assert!(
        dist < 1e-10,
        "Exact 2-qubit commuting ZZ+IZ: Frobenius distance = {dist:.2e}, expected < 1e-10"
    );
}

/// Auto strategy chooses Exact for the commuting case and matches exactly.
#[test]
fn test_matrix_auto_exact_path() {
    let t_val = 0.5;

    let mut h = Hamiltonian::new(2);
    h.add_term("ZZ".parse().unwrap(), 0.8.into()).unwrap();
    h.add_term("ZI".parse().unwrap(), 0.3.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h.clone())
        .unwrap()
        .with_strategy(EvolutionStrategy::Auto { steps: 5 });

    // evolution_info should say exact
    assert!(ansatz.evolution_info().is_exact);

    let u_circuit = circuit_matrix_at_t(&ansatz, "auto", "auto_t", t_val);
    let h_mat = hamiltonian_matrix(&h);
    let u_exact = matrix_exp_iht(&h_mat, t_val);

    let dist = frob_dist(&u_circuit, &u_exact);
    assert!(
        dist < 1e-10,
        "Auto-exact 2-qubit: distance = {dist:.2e}, expected < 1e-10"
    );
}

/// Trotter-1 error for non-commuting H = X + Z decreases as steps increase.
///
/// Expected error scaling: O(t²/n). For t = 0.5 and n = 1, 5, 20, error should
/// decrease monotonically.
#[test]
fn test_matrix_trotter1_error_decreases_with_steps() {
    let t_val = 0.5_f64;

    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();

    let h_mat = hamiltonian_matrix(&h);
    let u_exact = matrix_exp_iht(&h_mat, t_val);

    let steps_list = [1usize, 5, 20, 100];
    let mut prev_dist = f64::INFINITY;

    for &steps in &steps_list {
        let ansatz = PauliEvolutionAnsatz::new(h.clone()).unwrap().with_strategy(
            EvolutionStrategy::Trotter {
                mode: TrotterMode::FirstOrder,
                steps,
            },
        );

        let u_circuit = circuit_matrix_at_t(&ansatz, "tr1", "tr1_t", t_val);
        let dist = frob_dist(&u_circuit, &u_exact);

        assert!(
            dist < prev_dist,
            "Trotter-1 error should decrease as steps increase: \
             steps={steps} dist={dist:.2e} >= prev_dist={prev_dist:.2e}"
        );
        prev_dist = dist;
    }
}

/// Suzuki-2 should have smaller error than Trotter-1 with the same step count.
#[test]
fn test_matrix_suzuki2_better_than_trotter1_equal_steps() {
    let t_val = 0.8_f64;
    let steps = 4;

    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("Z".parse().unwrap(), 0.5.into()).unwrap();

    let h_mat = hamiltonian_matrix(&h);
    let u_exact = matrix_exp_iht(&h_mat, t_val);

    let trotter1_ansatz =
        PauliEvolutionAnsatz::new(h.clone())
            .unwrap()
            .with_strategy(EvolutionStrategy::Trotter {
                mode: TrotterMode::FirstOrder,
                steps,
            });
    let suzuki2_ansatz =
        PauliEvolutionAnsatz::new(h.clone())
            .unwrap()
            .with_strategy(EvolutionStrategy::Trotter {
                mode: TrotterMode::SecondOrder,
                steps,
            });

    let err1 = frob_dist(
        &circuit_matrix_at_t(&trotter1_ansatz, "t1", "t1_t", t_val),
        &u_exact,
    );
    let err2 = frob_dist(
        &circuit_matrix_at_t(&suzuki2_ansatz, "t2", "t2_t", t_val),
        &u_exact,
    );

    assert!(
        err2 < err1,
        "Suzuki-2 error ({err2:.2e}) should be smaller than Trotter-1 error ({err1:.2e}) \
         for the same number of steps"
    );
}

/// Suzuki-2 error for H = X + Y + Z decreases as steps increase.
#[test]
fn test_matrix_suzuki2_error_decreases_with_steps() {
    let t_val = 0.4_f64;

    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("Y".parse().unwrap(), 0.7.into()).unwrap();
    h.add_term("Z".parse().unwrap(), 0.5.into()).unwrap();

    let h_mat = hamiltonian_matrix(&h);
    let u_exact = matrix_exp_iht(&h_mat, t_val);

    let steps_list = [1usize, 4, 16];
    let mut prev_dist = f64::INFINITY;

    for &steps in &steps_list {
        let ansatz = PauliEvolutionAnsatz::new(h.clone()).unwrap().with_strategy(
            EvolutionStrategy::Trotter {
                mode: TrotterMode::SecondOrder,
                steps,
            },
        );

        let u_circuit = circuit_matrix_at_t(&ansatz, "su2", "su2_t", t_val);
        let dist = frob_dist(&u_circuit, &u_exact);

        assert!(
            dist < prev_dist,
            "Suzuki-2 error should decrease as steps increase: \
             steps={steps} dist={dist:.2e} >= prev_dist={prev_dist:.2e}"
        );
        prev_dist = dist;
    }
}

/// Randomized Trotter is deterministic for a fixed seed.
#[test]
fn test_randomized_trotter_is_deterministic() {
    let t_val = 0.5;
    let seed = 42;

    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("Z".parse().unwrap(), 0.5.into()).unwrap();

    let make_ansatz = || {
        PauliEvolutionAnsatz::new(h.clone())
            .unwrap()
            .with_strategy(EvolutionStrategy::Trotter {
                mode: TrotterMode::Randomized(seed),
                steps: 5,
            })
    };

    let m1 = circuit_matrix_at_t(&make_ansatz(), "rng", "rng_t", t_val);
    let m2 = circuit_matrix_at_t(&make_ansatz(), "rng", "rng_t", t_val);

    assert_abs_diff_eq!(frob_dist(&m1, &m2), 0.0, epsilon = 1e-15);
}

/// Large-steps Trotter-1 converges to e^{-iHt} for 1-qubit H = X + Z.
///
/// For first-order Trotter with n steps, error scales as O(t²/n).
/// With n=200 and t=0.1: expected Frobenius error ≈ t²·√2/n ≈ 7e-5 < 1e-4.
#[test]
fn test_matrix_trotter1_converges_for_large_steps() {
    let t_val = 0.1_f64; // small t so Trotter-1 error O(t²/n) is well below 1e-4
    let steps = 200;

    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();

    let ansatz =
        PauliEvolutionAnsatz::new(h.clone())
            .unwrap()
            .with_strategy(EvolutionStrategy::Trotter {
                mode: TrotterMode::FirstOrder,
                steps,
            });

    let u_circuit = circuit_matrix_at_t(&ansatz, "conv", "conv_t", t_val);
    let h_mat = hamiltonian_matrix(&h);
    let u_exact = matrix_exp_iht(&h_mat, t_val);

    let dist = frob_dist(&u_circuit, &u_exact);
    assert!(
        dist < 1e-4,
        "Trotter-1 with steps={steps} at t={t_val}: distance={dist:.2e}, expected < 1e-4"
    );
}

/// Large-steps Suzuki-2 converges better than Trotter-1 for same step count.
#[test]
fn test_matrix_suzuki2_converges_faster_than_trotter1() {
    let t_val = 0.5_f64;
    let steps = 10;

    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("Y".parse().unwrap(), 0.8.into()).unwrap();

    let h_mat = hamiltonian_matrix(&h);
    let u_exact = matrix_exp_iht(&h_mat, t_val);

    let tr1 =
        PauliEvolutionAnsatz::new(h.clone())
            .unwrap()
            .with_strategy(EvolutionStrategy::Trotter {
                mode: TrotterMode::FirstOrder,
                steps,
            });
    let su2 =
        PauliEvolutionAnsatz::new(h.clone())
            .unwrap()
            .with_strategy(EvolutionStrategy::Trotter {
                mode: TrotterMode::SecondOrder,
                steps,
            });

    let err1 = frob_dist(&circuit_matrix_at_t(&tr1, "tr1", "tr1_t", t_val), &u_exact);
    let err2 = frob_dist(&circuit_matrix_at_t(&su2, "su2", "su2_t", t_val), &u_exact);

    assert!(
        err2 < err1,
        "Suzuki-2 (err={err2:.2e}) must beat Trotter-1 (err={err1:.2e}) at steps={steps}"
    );
}

/// 2-qubit non-commuting Hamiltonian: H = XX + ZZ.
/// XX and ZZ do commute (both diagonal in Bell basis), so this is an Exact case.
/// Verify matrix equality.
#[test]
fn test_matrix_exact_xx_zz_2qubit() {
    // XX and ZZ: both are tensor products of the same Pauli → they commute
    let t_val = 0.4;

    let mut h = Hamiltonian::new(2);
    h.add_term("XX".parse().unwrap(), 0.6.into()).unwrap();
    h.add_term("ZZ".parse().unwrap(), 0.4.into()).unwrap();

    // Verify commutativity
    assert!(h.all_terms_commute(), "XX and ZZ commute");

    let ansatz = PauliEvolutionAnsatz::new(h.clone())
        .unwrap()
        .with_strategy(EvolutionStrategy::Exact);

    let u_circuit = circuit_matrix_at_t(&ansatz, "xxzz", "xxzz_t", t_val);
    let h_mat = hamiltonian_matrix(&h);
    let u_exact = matrix_exp_iht(&h_mat, t_val);

    let dist = frob_dist(&u_circuit, &u_exact);
    assert!(
        dist < 1e-10,
        "Exact 2-qubit XX+ZZ: Frobenius distance = {dist:.2e}, expected < 1e-10"
    );
}

/// Regression for P0 bug: `evolution_info()` on a *non-commuting* Hamiltonian with
/// `EvolutionStrategy::Exact` must NOT report `is_exact = true`.
///
/// Before the fix, `Exact => (true, 1, None)` always returned `is_exact=true`,
/// producing a self-contradictory `EvolutionInfo { is_exact: true, all_terms_commute: false }`.
#[test]
fn test_evolution_info_exact_strategy_noncommuting_reports_not_exact() {
    // X and Z anti-commute on 1 qubit
    let mut h = Hamiltonian::new(1);
    h.add_term("X".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h)
        .unwrap()
        .with_strategy(EvolutionStrategy::Exact);

    let info = ansatz.evolution_info();

    // The terms do NOT commute
    assert!(
        !info.all_terms_commute,
        "X and Z do not commute; all_terms_commute must be false"
    );
    // is_exact must be false — you cannot have exact evolution of non-commuting terms
    assert!(
        !info.is_exact,
        "is_exact must be false when terms are non-commuting, even with Exact strategy. \
         Got: is_exact={}, all_terms_commute={}",
        info.is_exact, info.all_terms_commute
    );
    // validate() must reject this configuration
    assert!(
        ansatz.validate().is_err(),
        "validate() must fail for Exact strategy on non-commuting Hamiltonian"
    );
}

/// Sanity check: `evolution_info()` on a commuting Hamiltonian with `Exact` strategy
/// must report `is_exact = true` and `all_terms_commute = true`.
#[test]
fn test_evolution_info_exact_strategy_commuting_reports_exact() {
    // Z and ZZ commute
    let mut h = Hamiltonian::new(2);
    h.add_term("ZI".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h)
        .unwrap()
        .with_strategy(EvolutionStrategy::Exact);

    let info = ansatz.evolution_info();

    assert!(
        info.all_terms_commute,
        "ZI and ZZ commute; all_terms_commute must be true"
    );
    assert!(
        info.is_exact,
        "Commuting Hamiltonian with Exact strategy must report is_exact=true"
    );
    assert_eq!(info.steps, 1, "Exact evolution uses 1 effective step");
    assert!(
        info.trotter_mode.is_none(),
        "Exact evolution has no Trotter mode"
    );
    // validate() must succeed
    assert!(
        ansatz.validate().is_ok(),
        "validate() must succeed for commuting Hamiltonian with Exact strategy"
    );
}

#[test]
fn test_evolution_info_trotter_strategy_commuting_reports_exact() {
    let mut h = Hamiltonian::new(2);
    h.add_term("ZI".parse().unwrap(), 1.0.into()).unwrap();
    h.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();

    for (mode, steps) in [
        (TrotterMode::FirstOrder, 3),
        (TrotterMode::SecondOrder, 2),
        (TrotterMode::Randomized(7), 4),
    ] {
        let ansatz = PauliEvolutionAnsatz::new(h.clone())
            .unwrap()
            .with_strategy(EvolutionStrategy::Trotter { mode, steps });

        let info = ansatz.evolution_info();

        assert!(
            info.all_terms_commute,
            "ZI and ZZ commute; all_terms_commute must be true"
        );
        assert!(
            info.is_exact,
            "Explicit Trotter decomposition of a commuting Hamiltonian is still mathematically exact"
        );
        assert_eq!(info.steps, steps);
        assert_eq!(info.trotter_mode, Some(mode));
    }
}

/// Verifies that `pauli_evolution` on an all-identity Pauli string correctly
/// accumulates the global phase on the circuit.
///
/// H = α·II (2-qubit, all-identity). The evolution is:
///   e^{-iHt} = e^{-iαt} · I⊗I
///
/// `pauli_evolution` stores this as:
///   circuit.global_phase() = angle * (-0.5) = 2αt * (-0.5) = -αt
///
/// This test verifies the global-phase arithmetic via direct inspection of
/// `circuit.global_phase()` after parameter binding.
#[test]
fn test_identity_term_sets_correct_global_phase() {
    let alpha = 0.7_f64;
    let t_val = 0.5_f64;

    let mut h = Hamiltonian::new(2);
    h.add_term("II".parse().unwrap(), alpha.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h).unwrap();
    let circuit = ansatz.build_circuit("id").unwrap();

    // No gates should be emitted for a pure identity Hamiltonian
    assert_eq!(
        circuit.operations().len(),
        0,
        "Identity Hamiltonian must produce zero gates"
    );

    // Bind the time parameter and read the global phase
    let mut bindings = HashMap::new();
    bindings.insert("id_t", t_val);
    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();

    // global_phase stored = angle * (-0.5) = 2*α*t * (-0.5) = -α*t
    let expected_phase = -(alpha * t_val);
    let actual_phase = bound
        .global_phase()
        .evaluate(&None)
        .expect("global_phase must be concrete after binding");

    assert_abs_diff_eq!(actual_phase, expected_phase, epsilon = 1e-12);
    assert!(
        (actual_phase - expected_phase).abs() < 1e-12,
        "Global phase should be -α*t = {:.6}; got {:.6}",
        expected_phase,
        actual_phase
    );
}

/// Verifies that a Hamiltonian with an identity term and a physical term correctly:
/// 1. Generates gates only for the physical (ZZ) part.
/// 2. Accumulates the identity-term global phase separately.
///
/// H = 0.5·ZZ + 0.3·II (2-qubit, both commute with each other).
///
/// Expected:
///   - Circuit gates implement e^{-i·0.5·t·ZZ}  (physically equivalent to the ZZ rotation)
///   - circuit.global_phase() = -0.3·t  (from the identity term)
///   - circuit_to_matrix includes global phase and equals e^{-i·0.3·t} · e^{-i·0.5·t·ZZ}
///
/// The true unitary is e^{-i·0.3·t} · e^{-i·0.5·t·ZZ}; `circuit_to_matrix`
/// returns this full matrix including the global phase.
#[test]
fn test_identity_term_mixed_with_physical() {
    let t_val = 0.4_f64;

    let mut h_full = Hamiltonian::new(2);
    h_full.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();
    h_full.add_term("II".parse().unwrap(), 0.3.into()).unwrap();

    let ansatz = PauliEvolutionAnsatz::new(h_full).unwrap();
    let circuit = ansatz.build_circuit("mix").unwrap();

    // Step 1: verify global phase is -0.3*t
    let mut bindings = HashMap::new();
    bindings.insert("mix_t", t_val);
    let bound = circuit.assign_parameters(&Some(bindings.clone())).unwrap();

    let expected_phase = -(0.3_f64 * t_val);
    let actual_phase = bound
        .global_phase()
        .evaluate(&None)
        .expect("global_phase must be concrete after binding");
    assert!(
        (actual_phase - expected_phase).abs() < 1e-12,
        "Global phase from II term: expected {:.6}, got {:.6}",
        expected_phase,
        actual_phase
    );

    // Step 2: verify the circuit matrix equals e^{-i·0.3·t} · e^{-i·0.5·t·ZZ}.
    // Build reference Hamiltonian with only ZZ.
    let mut h_zz = Hamiltonian::new(2);
    h_zz.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();

    let u_circuit = circuit_matrix_at_t(&ansatz, "mix", "mix_t", t_val);
    let h_zz_mat = hamiltonian_matrix(&h_zz);
    let mut expected = matrix_exp_iht(&h_zz_mat, t_val);
    let global_factor = Complex64::from_polar(1.0, expected_phase);
    expected.mapv_inplace(|value| global_factor * value);

    let dist = frob_dist(&u_circuit, &expected);
    assert!(
        dist < 1e-10,
        "Circuit matrix should equal e^{{-i·0.3·t}}·e^{{-i·0.5·t·ZZ}}; \
         Frobenius distance = {dist:.2e}"
    );
}

#[test]
fn test_time_param_name_rejects_reserved_names_and_expressions() {
    let mut h = Hamiltonian::new(1);
    h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();

    for invalid_name in ["e", "π", "a+b", "1.0"] {
        let ansatz = PauliEvolutionAnsatz::new(h.clone())
            .unwrap()
            .with_time_param_name(invalid_name);

        let err = ansatz.build_circuit("ignored").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid time parameter name"),
            "expected invalid time parameter error for '{invalid_name}', got: {msg}"
        );
    }
}
