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

//! Rust FFI parity tests for the circuit/QIS binding additions:
//! `circuit_dag_add_parameter`, `pauli_string_expectation`,
//! `density_matrix_zeros`, `density_matrix_validate_physical`, and
//! `statevector_entanglement_entropy_pure`.

use binding_c::circuit::{
    circuit_dag_add_parameter, circuit_dag_free, circuit_dag_from_circuit,
    circuit_dag_parameters_len, circuit_free, circuit_new, circuit_rx_param, param_free,
    param_parse,
};
use binding_c::qis::{
    PAULI_X, PAULI_Z, density_matrix_free, density_matrix_maximally_mixed, density_matrix_new,
    density_matrix_num_qubits, density_matrix_trace, density_matrix_validate_physical,
    density_matrix_zeros, pauli_string_expectation, pauli_string_free, pauli_string_new,
    pauli_string_set_pauli, statevector_apply_cx, statevector_apply_h,
    statevector_entanglement_entropy_pure, statevector_free, statevector_new,
};
use std::ffi::CString;

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

#[test]
fn dag_add_parameter_interns_and_reports_insertion() {
    let theta = cstr("theta");
    let theta_ptr = param_parse(theta.as_ptr());
    assert!(!theta_ptr.is_null());

    // A DAG built from a circuit already interns "theta" through the RX.
    let circuit = circuit_new(1);
    assert_eq!(circuit_rx_param(circuit, 0, theta_ptr), 0);
    let dag = circuit_dag_from_circuit(circuit);
    assert!(!dag.is_null());
    assert_eq!(circuit_dag_parameters_len(dag), 1);

    // Re-adding the resident parameter reports "not inserted" with the
    // stable table index.
    let mut index = 99usize;
    let mut inserted = -1i32;
    assert_eq!(
        circuit_dag_add_parameter(dag, theta_ptr, &mut index, &mut inserted),
        0
    );
    assert_eq!((index, inserted), (0, 0));
    assert_eq!(circuit_dag_parameters_len(dag), 1);

    // A fresh parameter is inserted at the next table index.
    let phi = cstr("phi");
    let phi_ptr = param_parse(phi.as_ptr());
    assert!(!phi_ptr.is_null());
    assert_eq!(
        circuit_dag_add_parameter(dag, phi_ptr, &mut index, &mut inserted),
        0
    );
    assert_eq!((index, inserted), (1, 1));
    assert_eq!(circuit_dag_parameters_len(dag), 2);

    // Adding it a second time keeps the index and reports "not inserted".
    assert_eq!(
        circuit_dag_add_parameter(dag, phi_ptr, &mut index, &mut inserted),
        0
    );
    assert_eq!((index, inserted), (1, 0));
    assert_eq!(circuit_dag_parameters_len(dag), 2);

    // Output pointers may be omitted.
    assert_eq!(
        circuit_dag_add_parameter(dag, phi_ptr, std::ptr::null_mut(), std::ptr::null_mut()),
        0
    );

    // NULL handles and NULL parameters are rejected.
    assert_eq!(
        circuit_dag_add_parameter(std::ptr::null_mut(), phi_ptr, &mut index, &mut inserted),
        -1
    );
    assert_eq!(
        circuit_dag_add_parameter(dag, std::ptr::null(), &mut index, &mut inserted),
        -1
    );

    circuit_dag_free(dag);
    circuit_free(circuit);
    param_free(theta_ptr);
    param_free(phi_ptr);
}

#[test]
fn pauli_string_expectation_diagonal_and_offdiagonal() {
    let ps = pauli_string_new(2);
    assert!(!ps.is_null());
    assert_eq!(pauli_string_set_pauli(ps, 0, PAULI_Z), 0);

    // Z0 on |00> gives +1; on the little-endian "01" (qubit 0 = 1) gives -1.
    let zero0 = cstr("00");
    let one0 = cstr("01");
    let bitstrings_1 = [zero0.as_ptr()];
    let probs_1 = [1.0f64];
    let mut out = 99.0f64;
    assert_eq!(
        pauli_string_expectation(ps, bitstrings_1.as_ptr(), probs_1.as_ptr(), 1, &mut out),
        0
    );
    assert_eq!(out, 1.0);

    let bitstrings_2 = [one0.as_ptr()];
    assert_eq!(
        pauli_string_expectation(ps, bitstrings_2.as_ptr(), probs_1.as_ptr(), 1, &mut out),
        0
    );
    assert_eq!(out, -1.0);

    // An even mixture of the two eigenstates averages to zero.
    let bitstrings_mixed = [zero0.as_ptr(), one0.as_ptr()];
    let probs_mixed = [0.5f64, 0.5];
    assert_eq!(
        pauli_string_expectation(
            ps,
            bitstrings_mixed.as_ptr(),
            probs_mixed.as_ptr(),
            2,
            &mut out
        ),
        0
    );
    assert!(out.abs() < 1e-12);

    // Core rejections on a diagonal string: wrong length and invalid
    // characters map through the QisError table (both fall to the generic
    // -7 bucket). Off-diagonal strings never reach validation.
    let bad_len = cstr("000");
    let bitstrings_len = [bad_len.as_ptr()];
    assert_eq!(
        pauli_string_expectation(ps, bitstrings_len.as_ptr(), probs_1.as_ptr(), 1, &mut out),
        -7
    );
    let bad_char = cstr("0a");
    let bitstrings_char = [bad_char.as_ptr()];
    assert_eq!(
        pauli_string_expectation(ps, bitstrings_char.as_ptr(), probs_1.as_ptr(), 1, &mut out),
        -7
    );

    // Off-diagonal strings (X or Y support) vanish on basis distributions.
    assert_eq!(pauli_string_set_pauli(ps, 0, PAULI_X), 0);
    assert_eq!(
        pauli_string_expectation(ps, bitstrings_1.as_ptr(), probs_1.as_ptr(), 1, &mut out),
        0
    );
    assert_eq!(out, 0.0);

    // NULL and invalid-parameter paths.
    assert_eq!(
        pauli_string_expectation(
            std::ptr::null(),
            bitstrings_1.as_ptr(),
            probs_1.as_ptr(),
            1,
            &mut out
        ),
        -1
    );
    assert_eq!(
        pauli_string_expectation(
            ps,
            bitstrings_1.as_ptr(),
            probs_1.as_ptr(),
            1,
            std::ptr::null_mut()
        ),
        -1
    );
    assert_eq!(
        pauli_string_expectation(ps, std::ptr::null(), probs_1.as_ptr(), 1, &mut out),
        -1
    );
    assert_eq!(
        pauli_string_expectation(ps, bitstrings_1.as_ptr(), std::ptr::null(), 1, &mut out),
        -1
    );
    let bitstrings_null = [std::ptr::null()];
    assert_eq!(
        pauli_string_expectation(ps, bitstrings_null.as_ptr(), probs_1.as_ptr(), 1, &mut out),
        -8
    );

    // An empty distribution is valid and yields zero.
    assert_eq!(
        pauli_string_expectation(ps, std::ptr::null(), std::ptr::null(), 0, &mut out),
        0
    );
    assert_eq!(out, 0.0);

    pauli_string_free(ps);
}

#[test]
fn density_matrix_zeros_is_an_unphysical_accumulator() {
    let dm = density_matrix_zeros(2);
    assert!(!dm.is_null());
    assert_eq!(density_matrix_num_qubits(dm), 2);

    // Every element is zero, so the trace vanishes.
    let mut trace_re = 9.0f64;
    let mut trace_im = 9.0f64;
    assert_eq!(density_matrix_trace(dm, &mut trace_re, &mut trace_im), 0);
    assert_eq!((trace_re, trace_im), (0.0, 0.0));

    // Zero trace fails the unit-trace check (NotNormalized -> -8).
    assert_eq!(density_matrix_validate_physical(dm, 1e-10), -8);

    density_matrix_free(dm);

    // Physical states pass: the pure |0> state and I/2.
    let pure = density_matrix_new(1);
    assert!(!pure.is_null());
    assert_eq!(density_matrix_validate_physical(pure, 1e-10), 0);
    density_matrix_free(pure);

    let mixed = density_matrix_maximally_mixed(1);
    assert!(!mixed.is_null());
    assert_eq!(density_matrix_validate_physical(mixed, 1e-10), 0);
    // NULL handle and non-finite tolerance are rejected; an oversized
    // qubit count fails the constructor instead of panicking.
    assert_eq!(
        density_matrix_validate_physical(std::ptr::null(), 1e-10),
        -1
    );
    assert_eq!(density_matrix_validate_physical(mixed, f64::NAN), -8);
    density_matrix_free(mixed);
    assert!(density_matrix_zeros(100).is_null());
}

#[test]
fn statevector_entanglement_entropy_pure_bell_state() {
    let sv = statevector_new(2);
    assert!(!sv.is_null());
    assert_eq!(statevector_apply_h(sv, 0), 0);
    assert_eq!(statevector_apply_cx(sv, 0, 1), 0);

    // A Bell state has one bit (log2) of entanglement across any split.
    let subsys = [0u32];
    let mut out = 99.0f64;
    assert_eq!(
        statevector_entanglement_entropy_pure(sv, subsys.as_ptr(), 1, &mut out),
        0
    );
    assert!((out - 1.0).abs() < 1e-9);

    // A product state carries no entanglement.
    let sv_product = statevector_new(2);
    assert!(!sv_product.is_null());
    assert_eq!(
        statevector_entanglement_entropy_pure(sv_product, subsys.as_ptr(), 1, &mut out),
        0
    );
    assert!(out.abs() < 1e-9);
    statevector_free(sv_product);

    // Core rejections: a full or empty subsystem is invalid (-7), an
    // out-of-bounds index reports -2.
    let both = [0u32, 1];
    assert_eq!(
        statevector_entanglement_entropy_pure(sv, both.as_ptr(), 2, &mut out),
        -7
    );
    assert_eq!(
        statevector_entanglement_entropy_pure(sv, std::ptr::null(), 0, &mut out),
        -7
    );
    let bad = [5u32];
    assert_eq!(
        statevector_entanglement_entropy_pure(sv, bad.as_ptr(), 1, &mut out),
        -2
    );

    // NULL handle, NULL output, and a NULL subsystem buffer with data.
    assert_eq!(
        statevector_entanglement_entropy_pure(std::ptr::null(), subsys.as_ptr(), 1, &mut out),
        -1
    );
    assert_eq!(
        statevector_entanglement_entropy_pure(sv, subsys.as_ptr(), 1, std::ptr::null_mut()),
        -1
    );

    statevector_free(sv);
}
