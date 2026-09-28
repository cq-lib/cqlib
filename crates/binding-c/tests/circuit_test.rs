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

use binding_c::circuit::{
    circuit_add_qubits, circuit_assign_params, circuit_barrier, circuit_ccx, circuit_compose,
    circuit_crx, circuit_cry, circuit_crz, circuit_crz_param, circuit_cx, circuit_cy, circuit_cz,
    circuit_decompose, circuit_depth, circuit_free, circuit_fsim, circuit_h, circuit_i,
    circuit_inverse, circuit_measure, circuit_new, circuit_num_operations, circuit_num_parameters,
    circuit_num_qubits, circuit_phase, circuit_phase_param, circuit_qubits, circuit_qubits_len,
    circuit_remove_operation, circuit_reset, circuit_rx, circuit_rx_param, circuit_rxx,
    circuit_rxx_param, circuit_rxy, circuit_rxy_param, circuit_ry, circuit_ry_param, circuit_ryy,
    circuit_rz, circuit_rz_param, circuit_rzx, circuit_rzz, circuit_rzz_param, circuit_s,
    circuit_sdg, circuit_set_global_phase, circuit_set_global_phase_param, circuit_swap, circuit_t,
    circuit_tdg, circuit_to_matrix, circuit_to_matrix_len, circuit_u, circuit_u_param,
    circuit_validate, circuit_width, circuit_x, circuit_x2m, circuit_x2p, circuit_xy, circuit_xy2m,
    circuit_xy2p, circuit_y, circuit_y2m, circuit_y2p, circuit_z, param_evaluate, param_free,
    param_parse,
};
use std::ffi::CString;

#[test]
fn circuit_lifecycle_and_basic_gates() {
    let circuit = circuit_new(2);
    assert!(!circuit.is_null());
    assert_eq!(circuit_num_qubits(circuit), 2);
    assert_eq!(circuit_num_operations(circuit), 0);

    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_x(circuit, 1), 0);
    assert_eq!(circuit_y(circuit, 0), 0);
    assert_eq!(circuit_z(circuit, 1), 0);
    assert_eq!(circuit_rx(circuit, 0, 0.25), 0);
    assert_eq!(circuit_ry(circuit, 1, 0.5), 0);
    assert_eq!(circuit_rz(circuit, 0, 0.75), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    assert_eq!(circuit_cz(circuit, 1, 0), 0);
    assert_eq!(circuit_measure(circuit, 0), 0);
    assert_eq!(circuit_reset(circuit, 1), 0);

    assert_eq!(circuit_num_operations(circuit), 11);
    assert_eq!(circuit_validate(circuit), 0);
    circuit_free(circuit);
}

#[test]
fn circuit_error_codes_are_stable() {
    assert_eq!(circuit_num_qubits(std::ptr::null()), 0);
    assert_eq!(circuit_num_operations(std::ptr::null()), 0);
    assert_eq!(circuit_num_parameters(std::ptr::null()), 0);
    assert_eq!(circuit_validate(std::ptr::null()), -1);
    assert_eq!(circuit_h(std::ptr::null_mut(), 0), -1);
    circuit_free(std::ptr::null_mut());

    let circuit = circuit_new(1);
    assert_eq!(circuit_h(circuit, 1), -2);
    assert_eq!(circuit_cx(circuit, 0, 1), -2);
    assert_eq!(circuit_rx(circuit, 0, f64::NAN), -3);
    circuit_free(circuit);
}

#[test]
fn symbolic_parameters_can_be_evaluated_and_assigned() {
    let theta = CString::new("theta").unwrap();
    let phi = CString::new("phi").unwrap();
    let theta_ptr = param_parse(theta.as_ptr());
    let phi_ptr = param_parse(phi.as_ptr());
    assert!(!theta_ptr.is_null());
    assert!(!phi_ptr.is_null());

    let bindings = CString::new("theta:0.5,phi:1.25").unwrap();
    assert!((param_evaluate(theta_ptr, bindings.as_ptr()) - 0.5).abs() < 1e-12);
    assert!((param_evaluate(phi_ptr, bindings.as_ptr()) - 1.25).abs() < 1e-12);

    let circuit = circuit_new(2);
    assert_eq!(circuit_rx_param(circuit, 0, theta_ptr), 0);
    assert_eq!(circuit_ry_param(circuit, 1, phi_ptr), 0);
    assert_eq!(circuit_rz_param(circuit, 0, theta_ptr), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    assert_eq!(circuit_num_operations(circuit), 4);
    assert_eq!(circuit_num_parameters(circuit), 2);

    let assigned = circuit_assign_params(circuit, bindings.as_ptr());
    assert!(!assigned.is_null());
    assert_eq!(circuit_num_operations(assigned), 4);
    assert_eq!(circuit_num_parameters(assigned), 0);
    assert_eq!(circuit_validate(assigned), 0);

    circuit_free(assigned);
    circuit_free(circuit);
    param_free(theta_ptr);
    param_free(phi_ptr);
}

#[test]
fn invalid_parameters_return_null_or_error() {
    assert!(param_parse(std::ptr::null()).is_null());
    assert_eq!(param_evaluate(std::ptr::null(), std::ptr::null()), 0.0);

    let circuit = circuit_new(1);
    assert_eq!(circuit_rx_param(circuit, 0, std::ptr::null()), -1);
    assert!(circuit_assign_params(std::ptr::null(), std::ptr::null()).is_null());
    circuit_free(circuit);
    param_free(std::ptr::null_mut());
}

#[test]
fn single_qubit_no_param_gates() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_i(circuit, 0), 0);
    assert_eq!(circuit_s(circuit, 1), 0);
    assert_eq!(circuit_sdg(circuit, 0), 0);
    assert_eq!(circuit_t(circuit, 1), 0);
    assert_eq!(circuit_tdg(circuit, 0), 0);
    assert_eq!(circuit_x2p(circuit, 1), 0);
    assert_eq!(circuit_x2m(circuit, 0), 0);
    assert_eq!(circuit_y2p(circuit, 1), 0);
    assert_eq!(circuit_y2m(circuit, 0), 0);
    assert_eq!(circuit_num_operations(circuit), 9);
    assert_eq!(circuit_validate(circuit), 0);
    assert_eq!(circuit_i(circuit, 5), -2); // out of bounds
    circuit_free(circuit);
}

#[test]
fn single_qubit_param_gates_numeric() {
    let circuit = circuit_new(1);
    assert_eq!(circuit_phase(circuit, 0, 0.5), 0);
    assert_eq!(circuit_u(circuit, 0, 0.1, 0.2, 0.3), 0);
    assert_eq!(circuit_xy(circuit, 0, 0.4), 0);
    assert_eq!(circuit_xy2p(circuit, 0, 0.5), 0);
    assert_eq!(circuit_xy2m(circuit, 0, 0.6), 0);
    assert_eq!(circuit_rxy(circuit, 0, 0.7, 0.8), 0);
    assert_eq!(circuit_num_operations(circuit), 6);
    assert_eq!(circuit_u(circuit, 0, f64::NAN, 0.0, 0.0), -3);
    circuit_free(circuit);
}

#[test]
fn single_qubit_param_gates_symbolic() {
    let theta = CString::new("theta").unwrap();
    let phi = CString::new("phi").unwrap();
    let lambda = CString::new("lambda").unwrap();
    let t = param_parse(theta.as_ptr());
    let p = param_parse(phi.as_ptr());
    let l = param_parse(lambda.as_ptr());
    assert!(!t.is_null() && !p.is_null() && !l.is_null());

    let circuit = circuit_new(1);
    assert_eq!(circuit_phase_param(circuit, 0, t), 0);
    assert_eq!(circuit_u_param(circuit, 0, t, p, l), 0);
    assert_eq!(circuit_rxy_param(circuit, 0, t, p), 0);
    assert_eq!(circuit_num_parameters(circuit), 3);
    assert_eq!(circuit_u_param(circuit, 0, std::ptr::null(), p, l), -1);
    circuit_free(circuit);
    param_free(t);
    param_free(p);
    param_free(l);
}

#[test]
fn two_and_three_qubit_gates() {
    let circuit = circuit_new(3);
    assert_eq!(circuit_cy(circuit, 0, 1), 0);
    assert_eq!(circuit_swap(circuit, 1, 2), 0);
    assert_eq!(circuit_rxx(circuit, 0, 1, 0.5), 0);
    assert_eq!(circuit_ryy(circuit, 1, 2, 0.5), 0);
    assert_eq!(circuit_rzz(circuit, 0, 2, 0.5), 0);
    assert_eq!(circuit_rzx(circuit, 1, 0, 0.5), 0);
    assert_eq!(circuit_crx(circuit, 0, 1, 0.5), 0);
    assert_eq!(circuit_cry(circuit, 1, 2, 0.5), 0);
    assert_eq!(circuit_crz(circuit, 0, 2, 0.5), 0);
    assert_eq!(circuit_fsim(circuit, 1, 0, 0.3, 0.7), 0);
    assert_eq!(circuit_ccx(circuit, 0, 1, 2), 0);
    assert_eq!(circuit_num_operations(circuit), 11);
    assert_eq!(circuit_ccx(circuit, 0, 1, 5), -2); // out of bounds
    circuit_free(circuit);
}

#[test]
fn two_qubit_symbolic_gates() {
    let theta = CString::new("theta").unwrap();
    let t = param_parse(theta.as_ptr());
    assert!(!t.is_null());

    let circuit = circuit_new(2);
    assert_eq!(circuit_rxx_param(circuit, 0, 1, t), 0);
    assert_eq!(circuit_rzz_param(circuit, 1, 0, t), 0);
    assert_eq!(circuit_crz_param(circuit, 0, 1, t), 0);
    assert_eq!(circuit_num_parameters(circuit), 1);
    circuit_free(circuit);
    param_free(t);
}

#[test]
fn barrier_and_properties() {
    let circuit = circuit_new(3);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_h(circuit, 1), 0);
    assert_eq!(circuit_cx(circuit, 0, 2), 0);

    // Barrier over subset
    let qubits = [0u32, 1];
    assert_eq!(circuit_barrier(circuit, qubits.as_ptr(), 2), 0);

    // Global barrier (NULL + count=0)
    assert_eq!(circuit_barrier(circuit, std::ptr::null(), 0), 0);
    assert_eq!(circuit_num_operations(circuit), 5);

    // Properties
    assert_eq!(circuit_width(circuit), 3);
    assert_eq!(circuit_qubits_len(circuit), 3);
    let mut buf = [0u32; 3];
    assert_eq!(circuit_qubits(circuit, buf.as_mut_ptr(), 3), 3);
    assert_eq!(buf, [0, 1, 2]);
    let depth = circuit_depth(circuit, false);
    assert!(depth > 0);

    // Remove operation
    assert_eq!(circuit_remove_operation(circuit, 0), 0);
    assert_eq!(circuit_num_operations(circuit), 4);
    assert_eq!(circuit_remove_operation(circuit, 100), -3);
    circuit_free(circuit);
}

#[test]
fn inverse_and_decompose() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let inv = circuit_inverse(circuit);
    assert!(!inv.is_null());
    assert_eq!(circuit_num_operations(inv), 2);
    assert_eq!(circuit_validate(inv), 0);

    let dec = circuit_decompose(circuit);
    assert!(!dec.is_null());
    assert_eq!(circuit_validate(dec), 0);

    circuit_free(inv);
    circuit_free(dec);
    circuit_free(circuit);
}

#[test]
fn to_matrix_works() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let len = circuit_to_matrix_len(circuit, std::ptr::null(), 0);
    assert_eq!(len, 4 * 4); // 2^2 * 2^2 = 16 complex elements

    let mut buf = vec![0.0f64; len * 2];
    let written = circuit_to_matrix(circuit, std::ptr::null(), 0, buf.as_mut_ptr(), buf.len());
    assert_eq!(written, len);

    circuit_free(circuit);
}

#[test]
fn compose_and_add_qubits() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);

    let other = circuit_new(1);
    assert_eq!(circuit_x(other, 0), 0);

    // Compose with identity mapping: other's q0 -> circuit's q1
    let map = [1u32];
    assert_eq!(circuit_compose(circuit, other, map.as_ptr(), 1), 0);
    assert_eq!(circuit_num_operations(circuit), 2);

    // Add qubits
    let new_q = [3u32, 4];
    assert_eq!(circuit_add_qubits(circuit, new_q.as_ptr(), 2), 0);
    assert_eq!(circuit_width(circuit), 4);

    circuit_free(other);
    circuit_free(circuit);
}

#[test]
fn global_phase_operations() {
    let circuit = circuit_new(1);
    assert_eq!(circuit_set_global_phase(circuit, 0.5), 0);
    assert_eq!(circuit_set_global_phase(circuit, f64::NAN), -3);

    let expr = CString::new("alpha").unwrap();
    let p = param_parse(expr.as_ptr());
    assert!(!p.is_null());
    assert_eq!(circuit_set_global_phase_param(circuit, p), 0);
    assert_eq!(
        circuit_set_global_phase_param(circuit, std::ptr::null()),
        -1
    );

    circuit_free(circuit);
    param_free(p);
}
