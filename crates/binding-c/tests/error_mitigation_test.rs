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
