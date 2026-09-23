//! Integration tests for QIS module C ABI.

#![allow(dead_code)]

use binding_c::circuit::{circuit_cx, circuit_free, circuit_h, circuit_new};
use binding_c::cqlib_string_free;
use binding_c::qis::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn cstr_to_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

fn build_bell_circuit() -> *mut binding_c::circuit::CCircuit {
    let circuit = circuit_new(2);
    assert!(!circuit.is_null());
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    circuit
}

#[test]
fn test_statevector_new_and_free() {
    let sv = statevector_new(2);
    assert!(!sv.is_null());
    assert_eq!(statevector_num_qubits(sv), 2);
    statevector_free(sv);
}

#[test]
fn test_statevector_null_handling() {
    statevector_free(std::ptr::null_mut());
    assert_eq!(statevector_num_qubits(std::ptr::null()), 0);
    assert_eq!(statevector_apply_h(std::ptr::null_mut(), 0), -1);
    assert_eq!(statevector_measure(std::ptr::null_mut(), 0), -1);
}

#[test]
fn test_statevector_apply_h_probabilities() {
    let sv = statevector_new(1);
    assert_eq!(statevector_apply_h(sv, 0), 0);

    let len = statevector_probabilities_len(sv);
    assert_eq!(len, 2);

    let mut probs = vec![0.0f64; len];
    let ret = statevector_probabilities(sv, probs.as_mut_ptr(), len);
    assert_eq!(ret, 0);
    assert!((probs[0] - 0.5).abs() < 1e-10);
    assert!((probs[1] - 0.5).abs() < 1e-10);

    statevector_free(sv);
}

#[test]
fn test_statevector_qubit_out_of_bounds() {
    let sv = statevector_new(1);
    assert_eq!(statevector_apply_h(sv, 5), -2);
    assert_eq!(statevector_apply_cx(sv, 0, 5), -2);
    statevector_free(sv);
}

#[test]
fn test_statevector_apply_circuit_bell_state() {
    let circuit = build_bell_circuit();
    let sv = statevector_from_circuit(circuit);
    assert!(!sv.is_null());
    assert_eq!(statevector_num_qubits(sv), 2);

    let len = statevector_probabilities_len(sv);
    let mut probs = vec![0.0f64; len];
    statevector_probabilities(sv, probs.as_mut_ptr(), len);
    assert!((probs[0] - 0.5).abs() < 1e-10);
    assert!((probs[1] < 1e-10));
    assert!((probs[2] < 1e-10));
    assert!((probs[3] - 0.5).abs() < 1e-10);

    statevector_free(sv);
    circuit_free(circuit);
}

#[test]
fn test_statevector_measure_all() {
    let sv = statevector_new(2);
    statevector_apply_h(sv, 0);
    statevector_apply_cx(sv, 0, 1);

    let bitstring = statevector_measure_all(sv);
    let s = cstr_to_string(bitstring);
    cqlib_string_free(bitstring);
    assert!(s == "00" || s == "11");

    statevector_free(sv);
}

#[test]
fn test_statevector_sample_shots() {
    let sv = statevector_new(2);
    statevector_apply_h(sv, 0);
    statevector_apply_cx(sv, 0, 1);

    let list = statevector_sample_shots(sv, 50);
    assert!(!list.is_null());
    assert_eq!(outcome_list_len(list), 50);

    for i in 0..outcome_list_len(list) {
        let ptr = outcome_list_get(list, i);
        let s = cstr_to_string(ptr);
        cqlib_string_free(ptr);
        assert!(s == "00" || s == "11");
    }

    outcome_list_free(list);
    statevector_free(sv);
}

#[test]
fn test_statevector_reset() {
    let sv = statevector_new(1);
    statevector_apply_x(sv, 0);
    assert_eq!(statevector_reset(sv, 0), 0);

    let mut probs = vec![0.0f64; 2];
    statevector_probabilities(sv, probs.as_mut_ptr(), 2);
    assert!((probs[0] - 1.0).abs() < 1e-10);
    statevector_free(sv);
}

#[test]
fn test_density_matrix_new_and_gates() {
    let dm = density_matrix_new(2);
    assert!(!dm.is_null());
    assert_eq!(density_matrix_num_qubits(dm), 2);

    assert_eq!(density_matrix_apply_h(dm, 0), 0);
    assert_eq!(density_matrix_apply_cx(dm, 0, 1), 0);

    let len = density_matrix_probabilities_len(dm);
    let mut probs = vec![0.0f64; len];
    density_matrix_probabilities(dm, probs.as_mut_ptr(), len);
    assert!((probs[0] - 0.5).abs() < 1e-10);
    assert!((probs[3] - 0.5).abs() < 1e-10);

    density_matrix_free(dm);
}

#[test]
fn test_density_matrix_partial_trace() {
    let dm = density_matrix_new(2);
    density_matrix_apply_h(dm, 0);
    density_matrix_apply_cx(dm, 0, 1);

    let keep = [0u32];
    let reduced = density_matrix_partial_trace(dm, keep.as_ptr(), 1);
    assert!(!reduced.is_null());
    assert_eq!(density_matrix_num_qubits(reduced), 1);

    density_matrix_free(reduced);
    density_matrix_free(dm);
}

#[test]
fn test_stabilizer_new_and_clifford() {
    let ss = stabilizer_new(2);
    assert!(!ss.is_null());
    assert_eq!(stabilizer_num_qubits(ss), 2);

    assert_eq!(stabilizer_apply_h(ss, 0), 0);
    assert_eq!(stabilizer_apply_cx(ss, 0, 1), 0);

    let len = stabilizer_probabilities_len(ss);
    let mut probs = vec![0.0f64; len];
    let ret = stabilizer_probabilities(ss, probs.as_mut_ptr(), len);
    assert_eq!(ret, 0);
    assert!((probs[0] - 0.5).abs() < 1e-10);
    assert!((probs[3] - 0.5).abs() < 1e-10);

    stabilizer_free(ss);
}

#[test]
fn test_stabilizer_new_zero_qubits() {
    let ss = stabilizer_new(0);
    assert!(ss.is_null(), "stabilizer_new(0) should return NULL");
}

#[test]
fn test_stabilizer_measure_all_bell() {
    let ss = stabilizer_new(2);
    stabilizer_apply_h(ss, 0);
    stabilizer_apply_cx(ss, 0, 1);

    let bitstring = stabilizer_measure_all(ss);
    let s = cstr_to_string(bitstring);
    cqlib_string_free(bitstring);
    assert!(s == "00" || s == "11");

    stabilizer_free(ss);
}

#[test]
fn test_stabilizer_pauli_expectation() {
    let ss = stabilizer_new(1);
    stabilizer_apply_h(ss, 0);

    let pauli = pauli_string_parse(CString::new("X").unwrap().as_ptr());
    assert!(!pauli.is_null());

    let mut value = 0i32;
    let ret = stabilizer_pauli_expectation(ss, pauli, &mut value);
    assert_eq!(ret, 0);
    assert_eq!(value, 1);

    pauli_string_free(pauli);
    stabilizer_free(ss);
}

#[test]
fn test_stabilizer_get_stabilizers() {
    let ss = stabilizer_new(2);
    let n = stabilizer_get_stabilizers_len(ss);
    assert_eq!(n, 2);

    let label = stabilizer_get_stabilizer(ss, 0);
    let s = cstr_to_string(label);
    cqlib_string_free(label);
    assert!(s.contains('Z'));

    stabilizer_free(ss);
}

#[test]
fn test_pauli_string_new_and_parse() {
    let ps = pauli_string_parse(CString::new("XYZ").unwrap().as_ptr());
    assert!(!ps.is_null());
    assert_eq!(pauli_string_num_qubits(ps), 3);

    let label = pauli_string_to_string(ps);
    let s = cstr_to_string(label);
    cqlib_string_free(label);
    assert!(s.contains('X'));

    pauli_string_free(ps);
}

#[test]
fn test_pauli_string_set_get() {
    let ps = pauli_string_new(2);
    assert!(!ps.is_null());

    assert_eq!(pauli_string_set_pauli(ps, 0, PAULI_X), 0);
    assert_eq!(pauli_string_set_pauli(ps, 1, PAULI_Z), 0);
    assert_eq!(pauli_string_get_pauli(ps, 0), PAULI_X);
    assert_eq!(pauli_string_get_pauli(ps, 1), PAULI_Z);

    pauli_string_free(ps);
}

#[test]
fn test_pauli_string_get_out_of_bounds() {
    let ps = pauli_string_new(1);
    assert_eq!(pauli_string_get_pauli(ps, 10), 0xFF);
    pauli_string_free(ps);
}

#[test]
fn test_hamiltonian_new_and_add_term() {
    let h = hamiltonian_new(2);
    assert!(!h.is_null());
    assert_eq!(hamiltonian_num_qubits(h), 2);
    assert_eq!(hamiltonian_num_terms(h), 0);

    let pauli = pauli_string_parse(CString::new("ZZ").unwrap().as_ptr());
    assert!(!pauli.is_null());

    assert_eq!(hamiltonian_add_term(h, pauli, 0.5, 0.0), 0);
    assert_eq!(hamiltonian_num_terms(h), 1);

    pauli_string_free(pauli);
    hamiltonian_free(h);
}

#[test]
fn test_hamiltonian_from_pauli() {
    let pauli = pauli_string_parse(CString::new("Z").unwrap().as_ptr());
    assert!(!pauli.is_null());

    let h = hamiltonian_from_pauli(pauli);
    assert!(!h.is_null());
    assert_eq!(hamiltonian_num_qubits(h), 1);
    assert_eq!(hamiltonian_num_terms(h), 1);

    hamiltonian_free(h);
}

#[test]
fn test_statevector_expectation() {
    let sv = statevector_new(1);
    statevector_apply_h(sv, 0);

    let pauli = pauli_string_parse(CString::new("X").unwrap().as_ptr());
    let h = hamiltonian_from_pauli(pauli);

    let mut value = 0.0f64;
    let ret = statevector_expectation(sv, h, &mut value);
    assert_eq!(ret, 0);
    assert!((value - 1.0).abs() < 1e-10);

    hamiltonian_free(h);
    statevector_free(sv);
}
