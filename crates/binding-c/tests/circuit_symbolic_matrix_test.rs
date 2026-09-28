// This code is part of Cqlib.
//
// (C) Copyright China Telecom Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Rust FFI tests for symbolic matrices (`circuit/symbolic_matrix.rs`).

use binding_c::circuit::{
    CParameter, circuit_cx, circuit_free, circuit_h, circuit_new, circuit_rx, circuit_rx_param,
    circuit_to_symbolic_matrix, circuit_x, circuits_equivalent, param_free, param_parse,
    standard_gate_symbolic_matrix, symbolic_eye, symbolic_matrices_equivalent,
    symbolic_matrix_cols, symbolic_matrix_controlled, symbolic_matrix_element_str,
    symbolic_matrix_evaluate, symbolic_matrix_evaluate_len, symbolic_matrix_free,
    symbolic_matrix_rows, symbolic_matrix_simplify, symbolic_matrix_substitute,
};
use binding_c::cqlib_string_free;
use num_complex::Complex64;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

#[test]
fn symbolic_eye_shape() {
    // NULL tolerance for the shape readers and the free routine.
    assert_eq!(symbolic_matrix_rows(std::ptr::null()), 0);
    assert_eq!(symbolic_matrix_cols(std::ptr::null()), 0);
    assert_eq!(symbolic_matrix_evaluate_len(std::ptr::null()), 0);
    assert!(symbolic_matrix_element_str(std::ptr::null(), 0, 0).is_null());
    assert!(symbolic_matrix_simplify(std::ptr::null()).is_null());
    symbolic_matrix_free(std::ptr::null_mut());

    let eye = symbolic_eye(2);
    assert!(!eye.is_null());
    assert_eq!(symbolic_matrix_rows(eye), 2);
    assert_eq!(symbolic_matrix_cols(eye), 2);
    assert_eq!(symbolic_matrix_evaluate_len(eye), 4);

    // Element strings: both diagonal entries match (and are non-empty), both
    // off-diagonal entries match, and the two groups differ.
    let d00 = symbolic_matrix_element_str(eye, 0, 0);
    let d11 = symbolic_matrix_element_str(eye, 1, 1);
    assert!(!d00.is_null());
    assert!(!d11.is_null());
    let d00_text = unsafe { CStr::from_ptr(d00) }.to_str().unwrap().to_string();
    let d11_text = unsafe { CStr::from_ptr(d11) }.to_str().unwrap().to_string();
    cqlib_string_free(d00);
    cqlib_string_free(d11);
    assert!(!d00_text.is_empty());
    assert_eq!(d00_text, d11_text);

    let o01 = symbolic_matrix_element_str(eye, 0, 1);
    let o10 = symbolic_matrix_element_str(eye, 1, 0);
    assert!(!o01.is_null());
    assert!(!o10.is_null());
    let o01_text = unsafe { CStr::from_ptr(o01) }.to_str().unwrap().to_string();
    let o10_text = unsafe { CStr::from_ptr(o10) }.to_str().unwrap().to_string();
    cqlib_string_free(o01);
    cqlib_string_free(o10);
    assert_eq!(o01_text, o10_text);
    assert_ne!(d00_text, o01_text);
    assert_ne!(d11_text, o10_text);

    // Out-of-bounds indices return NULL.
    assert!(symbolic_matrix_element_str(eye, 2, 0).is_null());

    // Simplification keeps the shape.
    let simplified = symbolic_matrix_simplify(eye);
    assert!(!simplified.is_null());
    assert_eq!(symbolic_matrix_rows(simplified), 2);
    assert_eq!(symbolic_matrix_cols(simplified), 2);

    // A constant matrix evaluates with no bindings at all.
    let mut buf = [Complex64::new(0.0, 0.0); 4];
    assert_eq!(
        symbolic_matrix_evaluate(eye, std::ptr::null(), buf.as_mut_ptr(), 4),
        0
    );
    assert_eq!(buf[0].re, 1.0);
    assert_eq!(buf[0].im, 0.0);
    assert_eq!(buf[1].re, 0.0);
    assert_eq!(buf[1].im, 0.0);

    symbolic_matrix_free(simplified);
    symbolic_matrix_free(eye);
}

#[test]
fn standard_gate_matrix_evaluation() {
    // RX(theta) as a symbolic standard-gate matrix.
    let theta_expr = cstr("theta");
    let theta_ptr = param_parse(theta_expr.as_ptr());
    assert!(!theta_ptr.is_null());
    let rx_name = cstr("RX");
    let params = [theta_ptr as *const CParameter; 1];
    let rx_mat = standard_gate_symbolic_matrix(rx_name.as_ptr(), params.as_ptr(), 1);
    assert!(!rx_mat.is_null());
    assert_eq!(symbolic_matrix_rows(rx_mat), 2);
    assert_eq!(symbolic_matrix_cols(rx_mat), 2);
    assert_eq!(symbolic_matrix_evaluate_len(rx_mat), 4);

    // The diagonal entry renders as a cos(theta/2) expression.
    let diag = symbolic_matrix_element_str(rx_mat, 0, 0);
    assert!(!diag.is_null());
    assert!(
        unsafe { CStr::from_ptr(diag) }
            .to_str()
            .unwrap()
            .contains("cos")
    );
    cqlib_string_free(diag);

    // Bound evaluation: RX(0.5) = [[cos(0.25), -i*sin(0.25)],
    //                               [-i*sin(0.25), cos(0.25)]] in row-major order.
    let bindings = cstr("theta:0.5");
    let mut buf = [Complex64::new(0.0, 0.0); 4];
    assert_eq!(
        symbolic_matrix_evaluate(rx_mat, bindings.as_ptr(), buf.as_mut_ptr(), 4),
        0
    );
    assert!((buf[0].re - (0.25f64).cos()).abs() < 1e-12);
    assert_eq!(buf[0].im, 0.0);
    assert_eq!(buf[1].re, 0.0);
    assert!((buf[1].im + (0.25f64).sin()).abs() < 1e-12);
    assert!((buf[2].re - buf[1].re).abs() < 1e-12);
    assert!((buf[2].im - buf[1].im).abs() < 1e-12);
    assert!((buf[3].re - buf[0].re).abs() < 1e-12);
    assert!((buf[3].im - buf[0].im).abs() < 1e-12);

    // Cross-check against the matrix of a numeric RX(0.5) circuit.
    let circuit = circuit_new(1);
    assert_eq!(circuit_rx(circuit, 0, 0.5), 0);
    let circ_mat = circuit_to_symbolic_matrix(circuit, std::ptr::null(), 0);
    assert!(!circ_mat.is_null());
    let mut buf2 = [Complex64::new(0.0, 0.0); 4];
    assert_eq!(
        symbolic_matrix_evaluate(circ_mat, std::ptr::null(), buf2.as_mut_ptr(), 4),
        0
    );
    for (a, b) in buf.iter().zip(buf2.iter()) {
        assert!((a.re - b.re).abs() < 1e-12);
        assert!((a.im - b.im).abs() < 1e-12);
    }

    symbolic_matrix_free(circ_mat);
    symbolic_matrix_free(rx_mat);
    circuit_free(circuit);
    param_free(theta_ptr);
}

#[test]
fn standard_gate_matrix_invalid_inputs() {
    // The parameterless X gate builds with no params buffer at all.
    let x_name = cstr("X");
    let x_mat = standard_gate_symbolic_matrix(x_name.as_ptr(), std::ptr::null(), 0);
    assert!(!x_mat.is_null());
    symbolic_matrix_free(x_mat);

    let theta_expr = cstr("theta");
    let theta_ptr = param_parse(theta_expr.as_ptr());
    assert!(!theta_ptr.is_null());
    let params = [theta_ptr as *const CParameter; 1];
    let rx_name = cstr("RX");

    // NULL gate name.
    assert!(standard_gate_symbolic_matrix(std::ptr::null(), std::ptr::null(), 0).is_null());

    // Invalid UTF-8 gate name.
    let invalid_utf8 = CString::new(vec![0xFF]).unwrap();
    assert!(standard_gate_symbolic_matrix(invalid_utf8.as_ptr(), std::ptr::null(), 0).is_null());

    // Unknown gate name.
    let unknown = cstr("NOT_A_GATE");
    assert!(standard_gate_symbolic_matrix(unknown.as_ptr(), std::ptr::null(), 0).is_null());

    // RX with a NULL params buffer but a non-zero length.
    assert!(standard_gate_symbolic_matrix(rx_name.as_ptr(), std::ptr::null(), 1).is_null());

    // RX with zero parameters.
    assert!(standard_gate_symbolic_matrix(rx_name.as_ptr(), std::ptr::null(), 0).is_null());

    // X with one parameter too many (parameter-count mismatch).
    assert!(standard_gate_symbolic_matrix(x_name.as_ptr(), params.as_ptr(), 1).is_null());

    // A NULL entry inside the params array.
    let null_params: [*const CParameter; 1] = [std::ptr::null()];
    assert!(standard_gate_symbolic_matrix(rx_name.as_ptr(), null_params.as_ptr(), 1).is_null());

    param_free(theta_ptr);
}

#[test]
fn circuit_matrix_conversion_and_order() {
    // A 1q parametric circuit yields a 2x2 matrix with a free symbol.
    let theta_expr = cstr("theta");
    let theta_ptr = param_parse(theta_expr.as_ptr());
    let rx_circuit = circuit_new(1);
    assert_eq!(circuit_rx_param(rx_circuit, 0, theta_ptr), 0);
    let rx_mat = circuit_to_symbolic_matrix(rx_circuit, std::ptr::null(), 0);
    assert!(!rx_mat.is_null());
    assert_eq!(symbolic_matrix_rows(rx_mat), 2);
    assert_eq!(symbolic_matrix_cols(rx_mat), 2);

    // The free symbol blocks evaluation without bindings.
    let mut buf = [Complex64::new(0.0, 0.0); 4];
    assert_eq!(
        symbolic_matrix_evaluate(rx_mat, std::ptr::null(), buf.as_mut_ptr(), 4),
        -3
    );

    // A constant 2q circuit yields a 4x4 matrix of 16 elements.
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    let mat = circuit_to_symbolic_matrix(circuit, std::ptr::null(), 0);
    assert!(!mat.is_null());
    assert_eq!(symbolic_matrix_rows(mat), 4);
    assert_eq!(symbolic_matrix_cols(mat), 4);
    assert_eq!(symbolic_matrix_evaluate_len(mat), 16);
    let mut buf16 = [Complex64::new(0.0, 0.0); 16];
    assert_eq!(
        symbolic_matrix_evaluate(mat, std::ptr::null(), buf16.as_mut_ptr(), 16),
        0
    );

    // Order handling: a NULL order buffer with a non-zero length is rejected.
    assert!(circuit_to_symbolic_matrix(circuit, std::ptr::null(), 1).is_null());

    // A full order for the 2q qubit set works.
    let order = [0u32, 1];
    let ordered = circuit_to_symbolic_matrix(circuit, order.as_ptr(), 2);
    assert!(!ordered.is_null());

    // A one-entry order misses qubit 1 of the 2q set.
    let partial = [0u32];
    assert!(circuit_to_symbolic_matrix(circuit, partial.as_ptr(), 1).is_null());

    // A two-entry order references a qubit the 1q circuit does not have.
    assert!(circuit_to_symbolic_matrix(rx_circuit, order.as_ptr(), 2).is_null());

    symbolic_matrix_free(ordered);
    symbolic_matrix_free(mat);
    symbolic_matrix_free(rx_mat);
    circuit_free(circuit);
    circuit_free(rx_circuit);
    param_free(theta_ptr);
}

#[test]
fn matrix_evaluate_error_codes() {
    let theta_expr = cstr("theta");
    let theta_ptr = param_parse(theta_expr.as_ptr());
    let rx_name = cstr("RX");
    let params = [theta_ptr as *const CParameter; 1];
    let rx_mat = standard_gate_symbolic_matrix(rx_name.as_ptr(), params.as_ptr(), 1);
    assert!(!rx_mat.is_null());

    let good = cstr("theta:0.5");
    let mut buf = [Complex64::new(0.0, 0.0); 4];

    // NULL matrix handle.
    assert_eq!(
        symbolic_matrix_evaluate(std::ptr::null(), good.as_ptr(), buf.as_mut_ptr(), 4),
        -1
    );

    // NULL output buffer (valid bindings, sufficient capacity).
    assert_eq!(
        symbolic_matrix_evaluate(rx_mat, good.as_ptr(), std::ptr::null_mut(), 4),
        -1
    );

    // Undersized buffer.
    assert_eq!(
        symbolic_matrix_evaluate(rx_mat, good.as_ptr(), buf.as_mut_ptr(), 3),
        -8
    );

    // Malformed bindings value.
    let bad_value = cstr("theta:abc");
    assert_eq!(
        symbolic_matrix_evaluate(rx_mat, bad_value.as_ptr(), buf.as_mut_ptr(), 4),
        -4
    );

    // Bindings for an unrelated symbol leave theta unevaluable.
    let ghost = cstr("ghost:1.0");
    assert_eq!(
        symbolic_matrix_evaluate(rx_mat, ghost.as_ptr(), buf.as_mut_ptr(), 4),
        -3
    );

    // No bindings at all.
    assert_eq!(
        symbolic_matrix_evaluate(rx_mat, std::ptr::null(), buf.as_mut_ptr(), 4),
        -3
    );

    // Constant matrices evaluate without any bindings.
    let eye = symbolic_eye(2);
    assert_eq!(
        symbolic_matrix_evaluate(eye, std::ptr::null(), buf.as_mut_ptr(), 4),
        0
    );

    symbolic_matrix_free(eye);
    symbolic_matrix_free(rx_mat);
    param_free(theta_ptr);
}

#[test]
fn equivalence_checks() {
    // Matrix-level equivalence: a NULL side is an error.
    let x_name = cstr("X");
    let x_gate = standard_gate_symbolic_matrix(x_name.as_ptr(), std::ptr::null(), 0);
    assert!(!x_gate.is_null());
    assert_eq!(symbolic_matrices_equivalent(std::ptr::null(), x_gate), -1);
    assert_eq!(symbolic_matrices_equivalent(x_gate, std::ptr::null()), -1);

    let x_circuit = circuit_new(1);
    assert_eq!(circuit_x(x_circuit, 0), 0);
    let x_mat = circuit_to_symbolic_matrix(x_circuit, std::ptr::null(), 0);
    let h_circuit = circuit_new(1);
    assert_eq!(circuit_h(h_circuit, 0), 0);
    let h_mat = circuit_to_symbolic_matrix(h_circuit, std::ptr::null(), 0);

    // The X gate matrix matches the X circuit matrix.
    assert_eq!(symbolic_matrices_equivalent(x_gate, x_mat), 1);
    // X and H differ by more than a global phase.
    assert_eq!(symbolic_matrices_equivalent(x_mat, h_mat), 0);

    // Different shapes (2x2 identity vs 4x4 controlled-X) are not equivalent.
    let eye = symbolic_eye(2);
    let cnot = symbolic_matrix_controlled(x_gate, 1);
    assert!(!cnot.is_null());
    assert_eq!(symbolic_matrices_equivalent(eye, cnot), 0);

    // Circuit-level equivalence: RX(pi) equals X up to a global phase.
    let rx_pi = circuit_new(1);
    assert_eq!(circuit_rx(rx_pi, 0, std::f64::consts::PI), 0);
    assert_eq!(
        circuits_equivalent(x_circuit, rx_pi, std::ptr::null(), 0),
        1
    );
    assert_eq!(
        circuits_equivalent(x_circuit, x_circuit, std::ptr::null(), 0),
        1
    );
    assert_eq!(
        circuits_equivalent(x_circuit, h_circuit, std::ptr::null(), 0),
        0
    );

    // NULL circuits.
    assert_eq!(
        circuits_equivalent(std::ptr::null(), x_circuit, std::ptr::null(), 0),
        -1
    );
    assert_eq!(
        circuits_equivalent(x_circuit, std::ptr::null(), std::ptr::null(), 0),
        -1
    );

    // A NULL order buffer with a non-zero length.
    assert_eq!(
        circuits_equivalent(x_circuit, rx_pi, std::ptr::null(), 1),
        -8
    );

    // An order that does not cover the 2q qubit set fails matrix construction.
    let empty_a = circuit_new(2);
    let empty_b = circuit_new(2);
    let order = [0u32];
    assert_eq!(circuits_equivalent(empty_a, empty_b, order.as_ptr(), 1), -3);

    symbolic_matrix_free(cnot);
    symbolic_matrix_free(eye);
    symbolic_matrix_free(h_mat);
    symbolic_matrix_free(x_mat);
    symbolic_matrix_free(x_gate);
    circuit_free(empty_b);
    circuit_free(empty_a);
    circuit_free(rx_pi);
    circuit_free(h_circuit);
    circuit_free(x_circuit);
}

#[test]
fn matrix_substitute_and_controlled() {
    // Substitute theta -> 0.5 in the RX(theta) matrix.
    let theta_expr = cstr("theta");
    let theta_ptr = param_parse(theta_expr.as_ptr());
    let rx_name = cstr("RX");
    let params = [theta_ptr as *const CParameter; 1];
    let rx_mat = standard_gate_symbolic_matrix(rx_name.as_ptr(), params.as_ptr(), 1);
    assert!(!rx_mat.is_null());

    let half_expr = cstr("0.5");
    let half = param_parse(half_expr.as_ptr());
    assert!(!half.is_null());
    let names = [theta_expr.as_ptr()];
    let repl = [half as *const CParameter; 1];
    let substituted = symbolic_matrix_substitute(rx_mat, names.as_ptr(), repl.as_ptr(), 1);
    assert!(!substituted.is_null());

    // The substituted matrix evaluates without bindings...
    let mut buf = [Complex64::new(0.0, 0.0); 4];
    assert_eq!(
        symbolic_matrix_evaluate(substituted, std::ptr::null(), buf.as_mut_ptr(), 4),
        0
    );
    assert!((buf[0].re - (0.25f64).cos()).abs() < 1e-12);

    // ...while the original matrix still carries the free symbol.
    assert_eq!(
        symbolic_matrix_evaluate(rx_mat, std::ptr::null(), buf.as_mut_ptr(), 4),
        -3
    );

    // Controlled-X embeds the X matrix in the bottom-right of a 4x4 identity.
    let x_name = cstr("X");
    let x_mat = standard_gate_symbolic_matrix(x_name.as_ptr(), std::ptr::null(), 0);
    assert!(!x_mat.is_null());
    let cnot = symbolic_matrix_controlled(x_mat, 1);
    assert!(!cnot.is_null());
    assert_eq!(symbolic_matrix_rows(cnot), 4);
    assert_eq!(symbolic_matrix_cols(cnot), 4);
    let mut buf4 = [Complex64::new(0.0, 0.0); 16];
    assert_eq!(
        symbolic_matrix_evaluate(cnot, std::ptr::null(), buf4.as_mut_ptr(), 16),
        0
    );
    // Top-left identity block.
    assert_eq!(buf4[0].re, 1.0);
    assert_eq!(buf4[5].re, 1.0);
    // Bottom-right X block (CNOT pattern).
    assert_eq!(buf4[6].re, 0.0);
    assert_eq!(buf4[7].re, 0.0);
    assert_eq!(buf4[10].re, 0.0);
    assert_eq!(buf4[11].re, 1.0);
    assert_eq!(buf4[14].re, 1.0);
    assert_eq!(buf4[15].re, 0.0);

    // One more control level doubles the dimension again.
    let ccx = symbolic_matrix_controlled(cnot, 1);
    assert!(!ccx.is_null());
    assert_eq!(symbolic_matrix_rows(ccx), 8);
    assert_eq!(symbolic_matrix_cols(ccx), 8);

    // Error paths.
    assert!(symbolic_matrix_controlled(std::ptr::null(), 1).is_null());
    assert!(
        symbolic_matrix_substitute(std::ptr::null(), names.as_ptr(), repl.as_ptr(), 1).is_null()
    );
    // names = NULL with a non-zero length.
    assert!(symbolic_matrix_substitute(rx_mat, std::ptr::null(), repl.as_ptr(), 1).is_null());
    // params = NULL with a non-zero length.
    assert!(symbolic_matrix_substitute(rx_mat, names.as_ptr(), std::ptr::null(), 1).is_null());
    // A NULL entry inside the names array.
    let null_names: [*const c_char; 1] = [std::ptr::null()];
    assert!(symbolic_matrix_substitute(rx_mat, null_names.as_ptr(), repl.as_ptr(), 1).is_null());
    // Invalid UTF-8 symbol name.
    let invalid_utf8 = CString::new(vec![0xFF]).unwrap();
    let bad_names = [invalid_utf8.as_ptr()];
    assert!(symbolic_matrix_substitute(rx_mat, bad_names.as_ptr(), repl.as_ptr(), 1).is_null());

    symbolic_matrix_free(ccx);
    symbolic_matrix_free(cnot);
    symbolic_matrix_free(x_mat);
    symbolic_matrix_free(substituted);
    symbolic_matrix_free(rx_mat);
    param_free(half);
    param_free(theta_ptr);
}
