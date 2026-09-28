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

//! Integration tests for the remaining classical-expression, Hamiltonian,
//! and state-inspection C ABI entries.

use binding_c::circuit::{
    CClassicalValueInfo, CQLIB_CLASSICAL_TYPE_BIT, CQLIB_CLASSICAL_TYPE_BIT_VEC,
    CQLIB_CLASSICAL_TYPE_BOOL, CQLIB_CLASSICAL_TYPE_UINT, circuit_free, circuit_measure_into,
    circuit_new, circuit_var, classical_expr_bit_literal, classical_expr_bool_literal,
    classical_expr_free, classical_expr_is_bit_false, classical_expr_is_bit_true,
    classical_expr_is_bool_false, classical_expr_is_bool_true, classical_expr_remap_classical_ids,
    classical_expr_ty, classical_expr_values, classical_expr_values_len, classical_expr_var,
    classical_expr_vars, classical_expr_vars_len, classical_expr_xor, classical_type_one_literal,
    classical_type_zero_literal, classical_var_free, classical_var_id,
};
use binding_c::qis::{
    CProbMeasurement, hamiltonian_add_term, hamiltonian_expectation_probs, hamiltonian_free,
    hamiltonian_new, hamiltonian_variance_statevector, pauli_string_free, pauli_string_parse,
    statevector_apply_cx, statevector_apply_h, statevector_apply_x, statevector_free,
    statevector_new, statevector_num_qubits,
};
use binding_c::visualization::{
    local_bloch_vectors, local_bloch_vectors_len, state_to_density_matrix,
    state_to_density_matrix_len,
};
use num_complex::Complex64;
use std::ffi::CString;
use std::os::raw::c_char;

const EPS: f64 = 1e-10;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < EPS,
        "expected {expected}, got {actual}"
    );
}

/// Checks the four literal predicates against the expected mutually exclusive
/// pattern `(bit_true, bit_false, bool_true, bool_false)`.
fn assert_predicates(
    expr: *const binding_c::circuit::CClassicalExpr,
    expected: (i32, i32, i32, i32),
) {
    assert_eq!(classical_expr_is_bit_true(expr), expected.0);
    assert_eq!(classical_expr_is_bit_false(expr), expected.1);
    assert_eq!(classical_expr_is_bool_true(expr), expected.2);
    assert_eq!(classical_expr_is_bool_false(expr), expected.3);
}

#[test]
fn classical_expr_literal_predicates_are_mutually_exclusive() {
    let bit_true = classical_expr_bit_literal(true);
    let bit_false = classical_expr_bit_literal(false);
    let bool_true = classical_expr_bool_literal(true);
    let bool_false = classical_expr_bool_literal(false);

    assert_predicates(bit_true, (1, 0, 0, 0));
    assert_predicates(bit_false, (0, 1, 0, 0));
    assert_predicates(bool_true, (0, 0, 1, 0));
    assert_predicates(bool_false, (0, 0, 0, 1));

    // Non-literal expressions satisfy none of the predicates.
    let circuit = circuit_new(1);
    let var = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    let var_expr = classical_expr_var(var);
    assert_predicates(var_expr, (0, 0, 0, 0));

    // NULL input maps to -1.
    assert_eq!(classical_expr_is_bit_true(std::ptr::null()), -1);
    assert_eq!(classical_expr_is_bit_false(std::ptr::null()), -1);
    assert_eq!(classical_expr_is_bool_true(std::ptr::null()), -1);
    assert_eq!(classical_expr_is_bool_false(std::ptr::null()), -1);

    classical_expr_free(var_expr);
    classical_var_free(var);
    circuit_free(circuit);
    classical_expr_free(bit_true);
    classical_expr_free(bit_false);
    classical_expr_free(bool_true);
    classical_expr_free(bool_false);
}

#[test]
fn classical_type_zero_and_one_literals() {
    // Bit: zero/false and one/true, verified via the literal predicates.
    let bit_zero = classical_type_zero_literal(CQLIB_CLASSICAL_TYPE_BIT, 0);
    let bit_one = classical_type_one_literal(CQLIB_CLASSICAL_TYPE_BIT, 0);
    assert_eq!(classical_expr_is_bit_false(bit_zero), 1);
    assert_eq!(classical_expr_is_bit_true(bit_one), 1);

    // Bool: same pattern on the Bool predicates.
    let bool_zero = classical_type_zero_literal(CQLIB_CLASSICAL_TYPE_BOOL, 0);
    let bool_one = classical_type_one_literal(CQLIB_CLASSICAL_TYPE_BOOL, 0);
    assert_eq!(classical_expr_is_bool_false(bool_zero), 1);
    assert_eq!(classical_expr_is_bool_true(bool_one), 1);

    // Width-bearing types produce the matching literal kind and type.
    let uint_one = classical_type_one_literal(CQLIB_CLASSICAL_TYPE_UINT, 8);
    let (mut tag, mut width) = (u32::MAX, u32::MAX);
    assert_eq!(classical_expr_ty(uint_one, &mut tag, &mut width), 0);
    assert_eq!(tag, CQLIB_CLASSICAL_TYPE_UINT);
    assert_eq!(width, 8);

    let bit_vec_zero = classical_type_zero_literal(CQLIB_CLASSICAL_TYPE_BIT_VEC, 4);
    assert_eq!(classical_expr_ty(bit_vec_zero, &mut tag, &mut width), 0);
    assert_eq!(tag, CQLIB_CLASSICAL_TYPE_BIT_VEC);
    assert_eq!(width, 4);

    // Invalid parameters return NULL.
    assert!(classical_type_zero_literal(99, 0).is_null());
    assert!(classical_type_zero_literal(CQLIB_CLASSICAL_TYPE_UINT, 0).is_null());
    assert!(classical_type_zero_literal(CQLIB_CLASSICAL_TYPE_BIT_VEC, 129).is_null());
    assert!(classical_type_one_literal(CQLIB_CLASSICAL_TYPE_UINT, 129).is_null());

    classical_expr_free(bit_zero);
    classical_expr_free(bit_one);
    classical_expr_free(bool_zero);
    classical_expr_free(bool_one);
    classical_expr_free(uint_one);
    classical_expr_free(bit_vec_zero);
}

#[test]
fn classical_expr_remap_classical_ids_rewrites_vars() {
    let circuit = circuit_new(2);
    let var0 = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    let var1 = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    let expr0 = classical_expr_var(var0);
    let expr1 = classical_expr_var(var1);
    let expr = classical_expr_xor(expr0, expr1);
    assert_eq!(classical_expr_vars_len(expr), 2);

    let old = [0u32, 1u32];
    let new = [5u32, 6u32];
    let remapped = classical_expr_remap_classical_ids(
        expr,
        old.as_ptr(),
        new.as_ptr(),
        old.len(),
        std::ptr::null(),
        std::ptr::null(),
        0,
    );
    assert!(!remapped.is_null());
    assert_eq!(classical_expr_vars_len(remapped), 2);

    let mut handles = [std::ptr::null_mut(); 2];
    assert_eq!(classical_expr_vars(remapped, handles.as_mut_ptr(), 2), 0);
    let mut ids = [0u32; 2];
    for (i, handle) in handles.iter().enumerate() {
        ids[i] = classical_var_id(*handle);
        classical_var_free(*handle);
    }
    ids.sort_unstable();
    assert_eq!(ids, [5, 6]);

    // Every referenced variable must be covered by the mapping.
    let missing = classical_expr_remap_classical_ids(
        expr,
        old.as_ptr(),
        new.as_ptr(),
        1,
        std::ptr::null(),
        std::ptr::null(),
        0,
    );
    assert!(missing.is_null());

    // NULL arrays for a non-zero length fail.
    assert!(
        classical_expr_remap_classical_ids(
            expr,
            std::ptr::null(),
            new.as_ptr(),
            2,
            std::ptr::null(),
            std::ptr::null(),
            0
        )
        .is_null()
    );
    // NULL expression fails.
    assert!(
        classical_expr_remap_classical_ids(
            std::ptr::null(),
            old.as_ptr(),
            new.as_ptr(),
            2,
            std::ptr::null(),
            std::ptr::null(),
            0
        )
        .is_null()
    );

    classical_expr_free(remapped);
    classical_expr_free(expr);
    classical_expr_free(expr0);
    classical_expr_free(expr1);
    classical_var_free(var0);
    classical_var_free(var1);
    circuit_free(circuit);
}

#[test]
fn classical_expr_remap_classical_ids_rewrites_measurement_values() {
    let circuit = circuit_new(1);
    let target = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    let measurement = circuit_measure_into(circuit, 0, target);
    assert!(!measurement.is_null());
    assert_eq!(classical_expr_values_len(measurement), 1);

    let mut infos = [CClassicalValueInfo {
        index: u32::MAX,
        tag: u32::MAX,
        width: u32::MAX,
    }];
    assert_eq!(classical_expr_values(measurement, infos.as_mut_ptr(), 1), 0);
    let old_index = infos[0].index;

    let old = [old_index];
    let new = [7u32];
    let remapped = classical_expr_remap_classical_ids(
        measurement,
        std::ptr::null(),
        std::ptr::null(),
        0,
        old.as_ptr(),
        new.as_ptr(),
        old.len(),
    );
    assert!(!remapped.is_null());

    let mut remapped_infos = [CClassicalValueInfo {
        index: u32::MAX,
        tag: u32::MAX,
        width: u32::MAX,
    }];
    assert_eq!(
        classical_expr_values(remapped, remapped_infos.as_mut_ptr(), 1),
        0
    );
    assert_eq!(remapped_infos[0].index, 7);
    assert_eq!(remapped_infos[0].tag, infos[0].tag);
    assert_eq!(remapped_infos[0].width, infos[0].width);

    // Missing value mapping fails.
    let missing = classical_expr_remap_classical_ids(
        measurement,
        std::ptr::null(),
        std::ptr::null(),
        0,
        [old_index + 100].as_ptr(),
        new.as_ptr(),
        1,
    );
    assert!(missing.is_null());

    classical_expr_free(remapped);
    classical_expr_free(measurement);
    classical_var_free(target);
    circuit_free(circuit);
}

/// Builds a 1-qubit Hamiltonian `H = Z` and returns both handles. The Pauli
/// string stays owned by the caller (`add_term` clones it).
fn z_hamiltonian() -> (
    *mut binding_c::qis::CHamiltonian,
    *mut binding_c::qis::CPauliString,
) {
    let source = CString::new("Z").unwrap();
    let pauli = pauli_string_parse(source.as_ptr());
    assert!(!pauli.is_null());
    let hamiltonian = hamiltonian_new(1);
    assert!(!hamiltonian.is_null());
    assert_eq!(hamiltonian_add_term(hamiltonian, pauli, 1.0, 0.0), 0);
    (hamiltonian, pauli)
}

#[test]
fn hamiltonian_expectation_probs_basic() {
    let (hamiltonian, pauli) = z_hamiltonian();

    let states = [CString::new("0").unwrap(), CString::new("1").unwrap()];
    let state_ptrs = [
        states[0].as_ptr() as *const c_char,
        states[1].as_ptr() as *const c_char,
    ];

    // Deterministic outcome |0> is the +1 eigenstate of Z.
    let probs = [1.0f64, 0.0];
    let entry = CProbMeasurement {
        basis: pauli,
        states: state_ptrs.as_ptr(),
        probs: probs.as_ptr(),
        len: 2,
    };
    let mut out = f64::NAN;
    assert_eq!(
        hamiltonian_expectation_probs(hamiltonian, &entry, 1, &mut out),
        0
    );
    assert_close(out, 1.0);

    // Balanced distribution averages to zero.
    let balanced = [0.5f64, 0.5];
    let entry = CProbMeasurement {
        basis: pauli,
        states: state_ptrs.as_ptr(),
        probs: balanced.as_ptr(),
        len: 2,
    };
    let mut out = f64::NAN;
    assert_eq!(
        hamiltonian_expectation_probs(hamiltonian, &entry, 1, &mut out),
        0
    );
    assert_close(out, 0.0);

    pauli_string_free(pauli);
    hamiltonian_free(hamiltonian);
}

#[test]
fn hamiltonian_expectation_probs_error_paths() {
    let (hamiltonian, pauli) = z_hamiltonian();
    let states = [CString::new("0").unwrap(), CString::new("1").unwrap()];
    let state_ptrs = [
        states[0].as_ptr() as *const c_char,
        states[1].as_ptr() as *const c_char,
    ];
    let probs = [1.0f64, 0.0];
    let entry = CProbMeasurement {
        basis: pauli,
        states: state_ptrs.as_ptr(),
        probs: probs.as_ptr(),
        len: 2,
    };
    let mut out = f64::NAN;

    // NULL handling.
    assert_eq!(
        hamiltonian_expectation_probs(std::ptr::null(), &entry, 1, &mut out),
        -1
    );
    assert_eq!(
        hamiltonian_expectation_probs(hamiltonian, &entry, 1, std::ptr::null_mut()),
        -1
    );
    assert_eq!(
        hamiltonian_expectation_probs(hamiltonian, std::ptr::null(), 1, &mut out),
        -1
    );
    let null_basis = CProbMeasurement {
        basis: std::ptr::null(),
        states: state_ptrs.as_ptr(),
        probs: probs.as_ptr(),
        len: 2,
    };
    assert_eq!(
        hamiltonian_expectation_probs(hamiltonian, &null_basis, 1, &mut out),
        -1
    );

    // Empty measurement list: no compatible basis for the Z term.
    assert_eq!(
        hamiltonian_expectation_probs(hamiltonian, std::ptr::null(), 0, &mut out),
        -7
    );

    // Invalid UTF-8 state string.
    let invalid = CString::new(&[0xFFu8][..]).unwrap();
    let bad_states = [
        invalid.as_ptr() as *const c_char,
        states[1].as_ptr() as *const c_char,
    ];
    let bad_entry = CProbMeasurement {
        basis: pauli,
        states: bad_states.as_ptr(),
        probs: probs.as_ptr(),
        len: 2,
    };
    assert_eq!(
        hamiltonian_expectation_probs(hamiltonian, &bad_entry, 1, &mut out),
        -4
    );

    pauli_string_free(pauli);
    hamiltonian_free(hamiltonian);
}

#[test]
fn hamiltonian_variance_statevector_basic() {
    let (hamiltonian, pauli) = z_hamiltonian();

    // |0> is an eigenstate of Z: zero variance.
    let sv = statevector_new(1);
    let mut out = f64::NAN;
    assert_eq!(
        hamiltonian_variance_statevector(hamiltonian, sv, &mut out),
        0
    );
    assert_close(out, 0.0);

    // |+> spreads over both Z eigenstates: Var(Z) = 1.
    assert_eq!(statevector_apply_h(sv, 0), 0);
    assert_eq!(
        hamiltonian_variance_statevector(hamiltonian, sv, &mut out),
        0
    );
    assert_close(out, 1.0);

    // Qubit-count mismatch.
    let big = statevector_new(2);
    assert_eq!(
        hamiltonian_variance_statevector(hamiltonian, big, &mut out),
        -8
    );
    statevector_free(big);

    // NULL handling.
    assert_eq!(
        hamiltonian_variance_statevector(std::ptr::null(), sv, &mut out),
        -1
    );
    assert_eq!(
        hamiltonian_variance_statevector(hamiltonian, std::ptr::null(), &mut out),
        -1
    );
    assert_eq!(
        hamiltonian_variance_statevector(hamiltonian, sv, std::ptr::null_mut()),
        -1
    );

    statevector_free(sv);
    pauli_string_free(pauli);
    hamiltonian_free(hamiltonian);
}

#[test]
fn local_bloch_vectors_statevector_layout() {
    // |01>: qubit 0 flipped to |1>, qubit 1 remains |0>.
    let sv = statevector_new(2);
    assert_eq!(statevector_apply_x(sv, 0), 0);

    let len = local_bloch_vectors_len(sv);
    assert_eq!(len, 3 * statevector_num_qubits(sv));
    let mut out = vec![f64::NAN; len];
    assert_eq!(local_bloch_vectors(sv, out.as_mut_ptr(), len), 0);
    // Flat layout [x0, y0, z0, x1, y1, z1].
    assert_close(out[0], 0.0);
    assert_close(out[1], 0.0);
    assert_close(out[2], -1.0);
    assert_close(out[3], 0.0);
    assert_close(out[4], 0.0);
    assert_close(out[5], 1.0);

    // Bell state: every local Bloch vector vanishes.
    assert_eq!(statevector_apply_h(sv, 0), 0);
    assert_eq!(statevector_apply_cx(sv, 0, 1), 0);
    let mut out = vec![f64::NAN; len];
    assert_eq!(local_bloch_vectors(sv, out.as_mut_ptr(), len), 0);
    for value in &out {
        assert_close(*value, 0.0);
    }

    // Error paths.
    assert_eq!(local_bloch_vectors_len(std::ptr::null()), 0);
    assert_eq!(
        local_bloch_vectors(std::ptr::null(), out.as_mut_ptr(), len),
        -1
    );
    assert_eq!(local_bloch_vectors(sv, std::ptr::null_mut(), len), -1);
    assert_eq!(local_bloch_vectors(sv, out.as_mut_ptr(), len + 1), -8);

    statevector_free(sv);
}

#[test]
fn state_to_density_matrix_row_major() {
    // |+> = (|0> + |1>) / sqrt(2): rho is the all-0.5 matrix.
    let sv = statevector_new(1);
    assert_eq!(statevector_apply_h(sv, 0), 0);

    let len = state_to_density_matrix_len(sv);
    assert_eq!(len, 4);
    let mut out = vec![Complex64::new(f64::NAN, f64::NAN); len];
    assert_eq!(state_to_density_matrix(sv, out.as_mut_ptr(), len), 0);
    for element in &out {
        assert_close(element.re, 0.5);
        assert_close(element.im, 0.0);
    }

    // |0>: rho = [[1, 0], [0, 0]] in row-major order.
    let zero = statevector_new(1);
    let mut rho = vec![Complex64::new(f64::NAN, f64::NAN); 4];
    assert_eq!(state_to_density_matrix(zero, rho.as_mut_ptr(), 4), 0);
    assert_close(rho[0].re, 1.0);
    assert_close(rho[1].re, 0.0);
    assert_close(rho[2].re, 0.0);
    assert_close(rho[3].re, 0.0);
    for element in &rho {
        assert_close(element.im, 0.0);
    }

    // Error paths.
    assert_eq!(state_to_density_matrix_len(std::ptr::null()), 0);
    assert_eq!(
        state_to_density_matrix(std::ptr::null(), rho.as_mut_ptr(), 4),
        -1
    );
    assert_eq!(state_to_density_matrix(zero, std::ptr::null_mut(), 4), -1);
    assert_eq!(state_to_density_matrix(zero, rho.as_mut_ptr(), 5), -8);

    statevector_free(zero);
    statevector_free(sv);
}
