//! Integration tests for error mitigation module C ABI.

#![allow(dead_code)]

use binding_c::circuit::{
    circuit_cx, circuit_free, circuit_h, circuit_new, circuit_num_operations,
};
use binding_c::error_mitigation::*;
use binding_c::qis::{
    hamiltonian_add_term, hamiltonian_free, hamiltonian_new, pauli_string_free, pauli_string_parse,
};
use std::os::raw::c_char;

/// Simple C estimator: returns the operation count as the "expectation value"
/// and 0.0 variance. Mirrors what a device/simulator backend would provide.
extern "C" fn counting_estimator(
    circuit: *const binding_c::circuit::CCircuit,
    _hamiltonian: *const binding_c::qis::CHamiltonian,
    _shots: usize,
    expectation: *mut f64,
    variance: *mut f64,
) {
    if circuit.is_null() || expectation.is_null() || variance.is_null() {
        return;
    }
    let ops = circuit_num_operations(circuit);
    unsafe {
        *expectation = ops as f64;
        *variance = 0.0;
    }
}

fn build_test_hamiltonian() -> *mut binding_c::qis::CHamiltonian {
    let h = hamiltonian_new(2);
    assert!(!h.is_null());
    let pauli = pauli_string_parse(c"ZZ".as_ptr() as *const c_char);
    assert!(!pauli.is_null());
    assert_eq!(hamiltonian_add_term(h, pauli, 1.0, 0.0), 0);
    pauli_string_free(pauli);
    h
}

#[test]
fn test_zne_mitigation_new_and_free() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let levels: [i32; 3] = [0, 1, 2];
    let zne = zne_mitigation_new(circuit, levels.as_ptr(), 3);
    assert!(!zne.is_null());
    assert_eq!(zne_mitigation_noise_factor(zne, 0), 1);
    assert_eq!(zne_mitigation_noise_factor(zne, 1), 3);
    assert_eq!(zne_mitigation_noise_factor(zne, 2), 5);
    assert_eq!(zne_mitigation_noise_factor(zne, 9), 0);

    zne_mitigation_free(zne);
    zne_mitigation_free(std::ptr::null_mut());
    circuit_free(circuit);
}

#[test]
fn test_zne_mitigation_fold_circuit() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let levels: [i32; 3] = [0, 1, 2];
    let zne = zne_mitigation_new(circuit, levels.as_ptr(), 3);
    assert!(!zne.is_null());

    // Global folding produces one circuit per level; higher levels fold in
    // more gate copies.
    let list = zne_mitigation_fold_circuit(zne, std::ptr::null());
    assert!(!list.is_null());
    assert_eq!(circuit_list_len(list), 3);
    let op0 = circuit_num_operations(circuit_list_get(list, 0));
    let op2 = circuit_num_operations(circuit_list_get(list, 2));
    assert_eq!(op0, 2);
    assert!(op2 > op0, "level-2 fold should add operations");
    circuit_list_free(list);

    // Selective folding on H only.
    let list = zne_mitigation_fold_circuit(zne, c"H".as_ptr() as *const c_char);
    assert!(!list.is_null());
    assert_eq!(circuit_list_len(list), 3);
    circuit_list_free(list);

    // Unknown gate name -> NULL.
    let list = zne_mitigation_fold_circuit(zne, c"NOT_A_GATE".as_ptr() as *const c_char);
    assert!(list.is_null());

    zne_mitigation_free(zne);
    circuit_free(circuit);
}

#[test]
fn test_error_mitigation_zne_full_pipeline() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let levels: [i32; 3] = [0, 1, 2];
    let em = error_mitigation_new(circuit, MITIGATION_ZNE, levels.as_ptr(), 3, 0);
    assert!(!em.is_null());

    let hamiltonian = build_test_hamiltonian();
    assert_eq!(
        error_mitigation_run(em, MITIGATION_ZNE, hamiltonian, 0, 0, counting_estimator),
        0
    );

    let mut expectation = f64::NAN;
    let mut variance = f64::NAN;
    assert_eq!(
        error_mitigation_get_mitigated(
            em,
            PROCESS_ZNE_POLYNOMIAL,
            1,
            &mut expectation,
            &mut variance
        ),
        0
    );
    assert!(
        expectation.is_finite(),
        "mitigated expectation should be finite"
    );

    hamiltonian_free(hamiltonian);
    error_mitigation_free(em);
    circuit_free(circuit);
}

#[test]
fn test_error_mitigation_run_requires_run_before_mitigate() {
    let circuit = circuit_new(1);
    let levels: [i32; 2] = [0, 1];
    let em = error_mitigation_new(circuit, MITIGATION_ZNE, levels.as_ptr(), 2, 0);
    assert!(!em.is_null());

    let mut expectation = f64::NAN;
    let mut variance = f64::NAN;
    // Calling get_mitigated before run must fail (-7).
    assert_eq!(
        error_mitigation_get_mitigated(
            em,
            PROCESS_ZNE_POLYNOMIAL,
            0,
            &mut expectation,
            &mut variance
        ),
        -7
    );

    error_mitigation_free(em);
    circuit_free(circuit);
}

#[test]
fn test_error_mitigation_invalid_params() {
    let circuit = circuit_new(1);

    // ZNE without fold levels -> NULL.
    assert!(error_mitigation_new(circuit, MITIGATION_ZNE, std::ptr::null(), 0, 0).is_null());
    // VD with zero copies -> NULL.
    assert!(
        error_mitigation_new(
            circuit,
            MITIGATION_VIRTUAL_DISTILLATION,
            std::ptr::null(),
            0,
            0
        )
        .is_null()
    );
    // Unknown method -> NULL.
    assert!(error_mitigation_new(circuit, 42, std::ptr::null(), 0, 0).is_null());

    circuit_free(circuit);
}

#[test]
fn test_error_mitigation_null_and_free() {
    error_mitigation_free(std::ptr::null_mut());
    assert!(
        error_mitigation_new(std::ptr::null(), MITIGATION_ZNE, std::ptr::null(), 0, 0).is_null()
    );
}

#[test]
fn test_virtual_distillation_new_and_build() {
    let circuit = circuit_new(1);
    assert_eq!(circuit_h(circuit, 0), 0);

    let vd = virtual_distillation_new(circuit, 2);
    assert!(!vd.is_null());

    let copy_swap = virtual_distillation_build_circuit(vd);
    assert!(!copy_swap.is_null());
    assert!(
        circuit_num_operations(copy_swap) > 0,
        "copy-swap circuit should contain operations"
    );
    circuit_free(copy_swap);

    virtual_distillation_free(vd);
    virtual_distillation_free(std::ptr::null_mut());

    // Zero copies -> NULL.
    assert!(virtual_distillation_new(circuit, 0).is_null());
    circuit_free(circuit);
}

// ===== Fine-grained ZNE API (checklist section 5) =====

#[test]
fn test_zne_circuit_levels_and_noise_factors_roundtrip() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let levels: [i32; 3] = [0, 1, 2];
    let zne = zne_mitigation_new(circuit, levels.as_ptr(), 3);
    assert!(!zne.is_null());

    // zne_circuit returns the original (unfolded) circuit.
    let original = zne_circuit(zne);
    assert!(!original.is_null());
    assert_eq!(circuit_num_operations(original), 2);
    circuit_free(original);

    // Fold levels roundtrip (two-step).
    assert_eq!(zne_fold_levels_len(zne), 3);
    let mut out_levels = [0i32; 3];
    assert_eq!(zne_fold_levels(zne, out_levels.as_mut_ptr(), 3), 0);
    assert_eq!(out_levels, [0, 1, 2]);
    assert_eq!(zne_fold_levels(zne, out_levels.as_mut_ptr(), 2), -8);

    // Noise factors roundtrip (two-step).
    assert_eq!(zne_noise_factors_len(zne), 3);
    let mut factors = [0i32; 3];
    assert_eq!(zne_noise_factors(zne, factors.as_mut_ptr(), 3), 0);
    assert_eq!(factors, [1, 3, 5]);
    assert_eq!(zne_noise_factors(zne, factors.as_mut_ptr(), 4), -8);

    // NULL handling.
    assert!(zne_circuit(std::ptr::null()).is_null());
    assert_eq!(zne_fold_levels_len(std::ptr::null()), 0);
    assert_eq!(zne_noise_factors_len(std::ptr::null()), 0);
    assert_eq!(
        zne_fold_levels(std::ptr::null(), out_levels.as_mut_ptr(), 3),
        -1
    );

    zne_mitigation_free(zne);
    circuit_free(circuit);
}

#[test]
fn test_zne_fold_circuits_two_step() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let levels: [i32; 3] = [0, 1, 2];
    let zne = zne_mitigation_new(circuit, levels.as_ptr(), 3);
    assert!(!zne.is_null());

    // Two-step folding: one owned circuit per fold level.
    assert_eq!(zne_fold_circuits_len(zne, std::ptr::null()), 3);
    let mut folded: Vec<*mut binding_c::circuit::CCircuit> = vec![std::ptr::null_mut(); 3];
    assert_eq!(
        zne_fold_circuits(zne, std::ptr::null(), folded.as_mut_ptr(), 3),
        0
    );

    // Global folding scales the circuit to (2 * level + 1) copies: the
    // 2-operation Bell circuit becomes 2 / 6 / 10 operations.
    let ops: Vec<usize> = folded.iter().map(|c| circuit_num_operations(*c)).collect();
    assert_eq!(ops, vec![2, 6, 10]);
    for circuit in folded {
        circuit_free(circuit);
    }

    // Selective folding on H only keeps CX untouched: level 1 yields
    // H H H CX (4 ops) and level 2 yields 5 H's + CX (6 ops).
    assert_eq!(zne_fold_circuits_len(zne, c"H".as_ptr()), 3);
    let mut folded: Vec<*mut binding_c::circuit::CCircuit> = vec![std::ptr::null_mut(); 3];
    assert_eq!(
        zne_fold_circuits(zne, c"H".as_ptr(), folded.as_mut_ptr(), 3),
        0
    );
    assert_eq!(circuit_num_operations(folded[1]), 4);
    assert_eq!(circuit_num_operations(folded[2]), 6);
    for circuit in folded {
        circuit_free(circuit);
    }

    // Buffer length mismatch and unknown gate names must fail.
    let mut sink: *mut binding_c::circuit::CCircuit = std::ptr::null_mut();
    assert_eq!(zne_fold_circuits(zne, std::ptr::null(), &mut sink, 2), -8);
    assert_eq!(
        zne_fold_circuits(zne, c"NOT_A_GATE".as_ptr(), &mut sink, 3),
        -4
    );
    assert_eq!(zne_fold_circuits_len(zne, c"NOT_A_GATE".as_ptr()), 0);

    zne_mitigation_free(zne);
    circuit_free(circuit);
}

#[test]
fn test_zne_extrapolate_known_values() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let levels: [i32; 3] = [0, 1, 2];
    let zne = zne_mitigation_new(circuit, levels.as_ptr(), 3);
    assert!(!zne.is_null());

    // Linear data y = 2x + 1 over noise factors x = [1, 3, 5]: the degree-1
    // fit extrapolates exactly to y(0) = 1.
    let linear = [3.0, 7.0, 11.0];
    let mut value = f64::NAN;
    assert_eq!(
        zne_poly_extrapolate(zne, linear.as_ptr(), 3, 1, &mut value),
        0
    );
    assert!((value - 1.0).abs() < 1e-9);

    // The unified entry point with the method tag must agree.
    assert_eq!(
        zne_extrapolate(
            zne,
            linear.as_ptr(),
            3,
            ZNE_EXTRAPOLATE_POLYNOMIAL,
            1,
            &mut value
        ),
        0
    );
    assert!((value - 1.0).abs() < 1e-9);

    // Degenerate degree-0 fit collapses to the mean of the data.
    assert_eq!(
        zne_poly_extrapolate(zne, linear.as_ptr(), 3, 0, &mut value),
        0
    );
    assert!((value - 7.0).abs() < 1e-9);

    // Exponential decay y = exp(-x) extrapolates to A = 1 at x = 0.
    let decay = [(-1.0f64).exp(), (-3.0f64).exp(), (-5.0f64).exp()];
    assert_eq!(zne_exp_extrapolate(zne, decay.as_ptr(), 3, &mut value), 0);
    assert!((value - 1.0).abs() < 1e-9);
    assert_eq!(
        zne_extrapolate(
            zne,
            decay.as_ptr(),
            3,
            ZNE_EXTRAPOLATE_EXPONENTIAL,
            0,
            &mut value
        ),
        0
    );
    assert!((value - 1.0).abs() < 1e-9);

    // Invalid inputs must fail.
    assert_ne!(
        zne_poly_extrapolate(zne, linear.as_ptr(), 3, 3, &mut value),
        0
    );
    assert_ne!(
        zne_poly_extrapolate(zne, linear.as_ptr(), 2, 1, &mut value),
        0
    );
    assert_ne!(
        zne_exp_extrapolate(zne, [0.0, 1.0, 2.0].as_ptr(), 3, &mut value),
        0
    );
    assert_ne!(
        zne_extrapolate(zne, linear.as_ptr(), 3, 42, 1, &mut value),
        0
    );

    zne_mitigation_free(zne);
    circuit_free(circuit);
}

#[test]
fn test_zne_run_em_sequence() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let levels: [i32; 3] = [0, 1, 2];
    let zne = zne_mitigation_new(circuit, levels.as_ptr(), 3);
    assert!(!zne.is_null());

    let hamiltonian = build_test_hamiltonian();
    assert_eq!(zne_run_em_sequence_len(zne), 3);
    assert_eq!(zne_run_em_sequence_with_shots_len(zne), 3);

    // The counting estimator reports the folded circuit lengths.
    let mut sequence = [f64::NAN; 3];
    assert_eq!(
        zne_run_em_sequence(
            zne,
            std::ptr::null(),
            hamiltonian,
            counting_estimator,
            sequence.as_mut_ptr(),
            3
        ),
        0
    );
    assert_eq!(sequence, [2.0, 6.0, 10.0]);

    // The with-shots variant forwards the shot count to the estimator.
    let mut sequence = [f64::NAN; 3];
    assert_eq!(
        zne_run_em_sequence_with_shots(
            zne,
            c"H".as_ptr(),
            hamiltonian,
            128,
            counting_estimator,
            sequence.as_mut_ptr(),
            3
        ),
        0
    );
    assert_eq!(sequence, [2.0, 4.0, 6.0]);

    // Buffer mismatch, Hamiltonian width mismatch, and unknown gate names
    // must fail.
    let mut sink = [0.0f64; 2];
    assert_eq!(
        zne_run_em_sequence(
            zne,
            std::ptr::null(),
            hamiltonian,
            counting_estimator,
            sink.as_mut_ptr(),
            2
        ),
        -8
    );
    let wide = hamiltonian_new(3);
    assert!(!wide.is_null());
    assert_eq!(
        zne_run_em_sequence(
            zne,
            std::ptr::null(),
            wide,
            counting_estimator,
            sequence.as_mut_ptr(),
            3
        ),
        -8
    );
    hamiltonian_free(wide);
    assert_eq!(
        zne_run_em_sequence(
            zne,
            c"NOT_A_GATE".as_ptr(),
            hamiltonian,
            counting_estimator,
            sequence.as_mut_ptr(),
            3
        ),
        -4
    );
    assert_eq!(
        zne_run_em_sequence(
            std::ptr::null(),
            std::ptr::null(),
            hamiltonian,
            counting_estimator,
            sequence.as_mut_ptr(),
            3
        ),
        -1
    );

    hamiltonian_free(hamiltonian);
    zne_mitigation_free(zne);
    circuit_free(circuit);
}

// ===== Fine-grained VirtualDistillation API (checklist section 5) =====

/// Static estimator: numerator invocations (Hamiltonian present) report
/// (4.0, 0.1); denominator invocations (Hamiltonian NULL) report (2.0, 0.05).
extern "C" fn static_vd_estimator(
    _circuit: *const binding_c::circuit::CCircuit,
    hamiltonian: *const binding_c::qis::CHamiltonian,
    _shots: usize,
    expectation: *mut f64,
    variance: *mut f64,
) {
    if expectation.is_null() || variance.is_null() {
        return;
    }
    unsafe {
        if hamiltonian.is_null() {
            *expectation = 2.0;
            *variance = 0.05;
        } else {
            *expectation = 4.0;
            *variance = 0.1;
        }
    }
}

#[test]
fn test_virtual_distillation_copies_roundtrip() {
    let circuit = circuit_new(1);
    assert_eq!(circuit_h(circuit, 0), 0);

    let vd = virtual_distillation_new(circuit, 2);
    assert!(!vd.is_null());
    assert_eq!(virtual_distillation_copies(vd), 2);

    assert_eq!(virtual_distillation_set_copies(vd, 3), 0);
    assert_eq!(virtual_distillation_copies(vd), 3);

    // Copies below 2 are rejected.
    assert_eq!(virtual_distillation_set_copies(vd, 1), -8);
    assert_eq!(virtual_distillation_set_copies(vd, 0), -8);
    assert_eq!(virtual_distillation_copies(vd), 3);

    // NULL handling.
    assert_eq!(virtual_distillation_copies(std::ptr::null()), 0);
    assert_eq!(virtual_distillation_set_copies(std::ptr::null_mut(), 3), -1);

    virtual_distillation_free(vd);
    circuit_free(circuit);
}

#[test]
fn test_virtual_distillation_run_vd_sampling() {
    let circuit = circuit_new(1);
    assert_eq!(circuit_h(circuit, 0), 0);

    let vd = virtual_distillation_new(circuit, 2);
    assert!(!vd.is_null());

    let hamiltonian = hamiltonian_new(1);
    assert!(!hamiltonian.is_null());
    let pauli = pauli_string_parse(c"Z".as_ptr() as *const c_char);
    assert!(!pauli.is_null());
    assert_eq!(hamiltonian_add_term(hamiltonian, pauli, 1.0, 0.0), 0);
    pauli_string_free(pauli);

    // Numerator: the estimator sees the copy-swap circuit and the expanded
    // Hamiltonian; denominator: no Hamiltonian.
    let mut num_mean = f64::NAN;
    let mut num_var = f64::NAN;
    assert_eq!(
        virtual_distillation_run_numerator_circuit(
            vd,
            hamiltonian,
            100,
            static_vd_estimator,
            &mut num_mean,
            &mut num_var
        ),
        0
    );
    assert_eq!(num_mean, 4.0);
    assert_eq!(num_var, 0.1);

    let mut den_mean = f64::NAN;
    let mut den_var = f64::NAN;
    assert_eq!(
        virtual_distillation_run_denominator_circuit(
            vd,
            100,
            static_vd_estimator,
            &mut den_mean,
            &mut den_var
        ),
        0
    );
    assert_eq!(den_mean, 2.0);
    assert_eq!(den_var, 0.05);

    // run_vd normalizes the numerator by the denominator: 4 / 2 = 2 with
    // var = 0.1 / 2^2 + 4^2 * 0.05 / 2^4 = 0.075.
    let mut mean = f64::NAN;
    let mut variance = f64::NAN;
    assert_eq!(
        virtual_distillation_run_vd(
            vd,
            hamiltonian,
            100,
            100,
            static_vd_estimator,
            &mut mean,
            &mut variance
        ),
        0
    );
    assert!((mean - 2.0).abs() < 1e-12);
    assert!((variance - 0.075).abs() < 1e-12);

    // Hamiltonian width mismatch must fail.
    let wide = hamiltonian_new(2);
    assert!(!wide.is_null());
    assert_eq!(
        virtual_distillation_run_vd(
            vd,
            wide,
            100,
            100,
            static_vd_estimator,
            &mut mean,
            &mut variance
        ),
        -8
    );
    hamiltonian_free(wide);

    // NULL handling.
    assert_eq!(
        virtual_distillation_run_vd(
            std::ptr::null(),
            hamiltonian,
            100,
            100,
            static_vd_estimator,
            &mut mean,
            &mut variance
        ),
        -1
    );

    hamiltonian_free(hamiltonian);
    virtual_distillation_free(vd);
    circuit_free(circuit);
}
