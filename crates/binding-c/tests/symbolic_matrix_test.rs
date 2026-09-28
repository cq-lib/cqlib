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

//! Rust FFI tests for symbolic complex values, element-level matrix
//! construction and the in-place `apply_*` gate family.

use binding_c::circuit::symbolic_matrix_element;
use binding_c::circuit::{
    CParameter, CSymbolicComplex, CSymbolicMatrix, circuit_cx, circuit_free, circuit_h,
    circuit_new, circuit_to_symbolic_matrix, param_add, param_evaluate, param_free,
    param_from_double, param_sub, param_symbol, standard_gate_symbolic_matrix,
    symbolic_complex_evaluate, symbolic_complex_exp_i, symbolic_complex_free,
    symbolic_complex_from_complex, symbolic_complex_from_real, symbolic_complex_i,
    symbolic_complex_im, symbolic_complex_is_one_exact, symbolic_complex_is_zero_exact,
    symbolic_complex_new, symbolic_complex_one, symbolic_complex_re, symbolic_complex_replace,
    symbolic_complex_simplifies_to_zero, symbolic_complex_simplify, symbolic_complex_zero,
    symbolic_eye, symbolic_matrices_equivalent, symbolic_matrix_apply_diagonal_gate,
    symbolic_matrix_apply_diagonal_gate_num, symbolic_matrix_apply_gate,
    symbolic_matrix_apply_gate_num, symbolic_matrix_apply_general_gate,
    symbolic_matrix_apply_general_gate_num, symbolic_matrix_apply_permutation_gate,
    symbolic_matrix_apply_permutation_gate_num, symbolic_matrix_apply_single_qubit_gate,
    symbolic_matrix_apply_single_qubit_gate_num, symbolic_matrix_apply_standard_gate,
    symbolic_matrix_apply_two_qubit_gate, symbolic_matrix_apply_two_qubit_gate_num,
    symbolic_matrix_evaluate, symbolic_matrix_evaluate_len, symbolic_matrix_free,
    symbolic_matrix_new, symbolic_matrix_rows,
};
use num_complex::Complex64;
use std::ffi::CString;

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn eval_re(param: *const CParameter, bindings: &str) -> f64 {
    param_evaluate(param, cstr(bindings).as_ptr())
}

fn eval_complex(ptr: *const CSymbolicComplex, bindings: &str) -> Result<Complex64, i32> {
    let bindings = cstr(bindings);
    let mut out = Complex64::new(f64::NAN, f64::NAN);
    let code = symbolic_complex_evaluate(ptr, bindings.as_ptr(), &mut out);
    if code == 0 { Ok(out) } else { Err(code) }
}

/// Evaluates a fully numeric matrix with no bindings.
fn evaluated(matrix: *const CSymbolicMatrix) -> Vec<Complex64> {
    let len = symbolic_matrix_evaluate_len(matrix);
    let mut buffer = vec![Complex64::new(0.0, 0.0); len];
    let code =
        symbolic_matrix_evaluate(matrix, std::ptr::null(), buffer.as_mut_ptr(), buffer.len());
    assert_eq!(code, 0);
    buffer
}

fn assert_close(a: &[Complex64], b: &[Complex64]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        assert!((x.re - y.re).abs() < 1e-9, "{x:?} != {y:?}");
        assert!((x.im - y.im).abs() < 1e-9, "{x:?} != {y:?}");
    }
}

const X_NUM: [Complex64; 4] = [
    Complex64::new(0.0, 0.0),
    Complex64::new(1.0, 0.0),
    Complex64::new(1.0, 0.0),
    Complex64::new(0.0, 0.0),
];

#[test]
fn symbolic_complex_construction() {
    // NULL tolerance and invalid-input handling.
    assert!(symbolic_complex_new(std::ptr::null(), std::ptr::null()).is_null());
    assert!(symbolic_complex_from_real(std::ptr::null()).is_null());
    assert!(symbolic_complex_exp_i(std::ptr::null()).is_null());
    assert!(symbolic_complex_from_complex(f64::NAN, 0.0).is_null());
    assert!(symbolic_complex_re(std::ptr::null()).is_null());
    assert!(symbolic_complex_im(std::ptr::null()).is_null());
    assert_eq!(symbolic_complex_is_zero_exact(std::ptr::null()), -1);
    assert_eq!(symbolic_complex_is_one_exact(std::ptr::null()), -1);
    assert_eq!(symbolic_complex_simplifies_to_zero(std::ptr::null()), -1);
    assert!(symbolic_complex_simplify(std::ptr::null()).is_null());
    symbolic_complex_free(std::ptr::null_mut());

    // zero / one / i constants.
    let zero = symbolic_complex_zero();
    assert_eq!(symbolic_complex_is_zero_exact(zero), 1);
    assert_eq!(symbolic_complex_simplifies_to_zero(zero), 1);
    assert_eq!(eval_complex(zero, "").unwrap(), Complex64::new(0.0, 0.0));

    let one = symbolic_complex_one();
    assert_eq!(symbolic_complex_is_one_exact(one), 1);
    assert_eq!(symbolic_complex_is_zero_exact(one), 0);
    assert_eq!(eval_complex(one, "").unwrap(), Complex64::new(1.0, 0.0));

    let i = symbolic_complex_i();
    assert_eq!(symbolic_complex_is_zero_exact(i), 0);
    assert_eq!(eval_complex(i, "").unwrap(), Complex64::new(0.0, 1.0));
    assert_eq!(symbolic_complex_is_one_exact(i), 0);

    // from_complex / from_real.
    let raw = symbolic_complex_from_complex(0.5, -0.25);
    assert_eq!(eval_complex(raw, "").unwrap(), Complex64::new(0.5, -0.25));

    let theta = param_symbol(cstr("theta").as_ptr());
    let real = symbolic_complex_from_real(theta);
    assert_eq!(
        eval_complex(real, "theta:0.75").unwrap(),
        Complex64::new(0.75, 0.0)
    );

    // new(re, im) and the re/im accessors.
    let two = param_from_double(2.0);
    let value = symbolic_complex_new(theta, two);
    assert!(!value.is_null());
    let re = symbolic_complex_re(value);
    let im = symbolic_complex_im(value);
    assert!((eval_re(re, "theta:0.5") - 0.5).abs() < 1e-12);
    assert!((eval_re(im, "") - 2.0).abs() < 1e-12);
    assert_eq!(
        eval_complex(value, "theta:0.5").unwrap(),
        Complex64::new(0.5, 2.0)
    );

    // exp_i(theta) evaluates to cos(theta) + i*sin(theta).
    let phase = symbolic_complex_exp_i(theta);
    let at_zero = eval_complex(phase, "theta:0").unwrap();
    assert!((at_zero.re - 1.0).abs() < 1e-12);
    assert!(at_zero.im.abs() < 1e-12);
    let at_pi = eval_complex(phase, "theta:3.141592653589793").unwrap();
    assert!((at_pi.re + 1.0).abs() < 1e-9);
    assert!(at_pi.im.abs() < 1e-9);

    param_free(re);
    param_free(im);
    symbolic_complex_free(zero);
    symbolic_complex_free(one);
    symbolic_complex_free(i);
    symbolic_complex_free(raw);
    symbolic_complex_free(real);
    symbolic_complex_free(value);
    symbolic_complex_free(phase);
    param_free(theta);
    param_free(two);
}

#[test]
fn symbolic_complex_evaluate_simplify_replace() {
    let theta = param_symbol(cstr("theta").as_ptr());
    // (x - theta, 0): after replacing x by theta both parts simplify to
    // exact zero.
    let x = param_symbol(cstr("x").as_ptr());
    let diff = param_sub(x, theta);
    let zero_param = param_from_double(0.0);
    let value = symbolic_complex_new(diff, zero_param);

    // Evaluation needs bound symbols: an unbound one yields -3 and a
    // malformed bindings string yields -4.
    let mut out = Complex64::new(0.0, 0.0);
    assert_eq!(
        symbolic_complex_evaluate(value, cstr("theta:1").as_ptr(), &mut out),
        -3
    );
    assert_eq!(eval_complex(value, "theta:1"), Err(-3));
    assert_eq!(eval_complex(value, "bogus;"), Err(-4));

    // Replace x by theta, then both parts simplify cleanly.
    let replaced = symbolic_complex_replace(value, cstr("x").as_ptr(), theta);
    assert!(!replaced.is_null());
    assert_eq!(symbolic_complex_simplifies_to_zero(replaced), 1);

    // symbolic_matrix-style simplification through the complex API.
    let simplified = symbolic_complex_simplify(replaced);
    assert!(!simplified.is_null());
    assert_eq!(symbolic_complex_is_zero_exact(simplified), 1);
    assert_eq!(
        eval_complex(simplified, "").unwrap(),
        Complex64::new(0.0, 0.0)
    );

    // A replacement handle takes precedence over simplification errors.
    assert!(symbolic_complex_replace(std::ptr::null(), cstr("x").as_ptr(), theta).is_null());
    assert!(symbolic_complex_replace(value, std::ptr::null(), theta).is_null());
    assert!(symbolic_complex_replace(value, cstr("x").as_ptr(), std::ptr::null()).is_null());

    // simplify failure path: exp_i with a constant beyond [-1, 1] is fine,
    // so exercise the Ok branch instead through nested arithmetic.
    let sum = param_add(theta, theta);
    let composed = symbolic_complex_from_real(sum);
    assert!(!symbolic_complex_simplify(composed).is_null());

    symbolic_complex_free(simplified);
    symbolic_complex_free(replaced);
    symbolic_complex_free(value);
    symbolic_complex_free(composed);
    param_free(diff);
    param_free(sum);
    param_free(zero_param);
    param_free(x);
    param_free(theta);
}

#[test]
fn symbolic_matrix_new_and_element() {
    // eye(2): build an element snapshot, then rebuild a matrix from it.
    let eye = symbolic_eye(2);
    let e00 = symbolic_matrix_element(eye, 0, 0);
    let e01 = symbolic_matrix_element(eye, 0, 1);
    let e10 = symbolic_matrix_element(eye, 1, 0);
    let e11 = symbolic_matrix_element(eye, 1, 1);
    assert!(!e00.is_null() && !e01.is_null() && !e10.is_null() && !e11.is_null());
    assert_eq!(symbolic_complex_is_one_exact(e00), 1);
    assert_eq!(symbolic_complex_is_zero_exact(e01), 1);

    let elements: [*const CSymbolicComplex; 4] = [e00, e01, e10, e11];
    let rebuilt = symbolic_matrix_new(2, 2, elements.as_ptr(), 4);
    assert!(!rebuilt.is_null());
    assert_eq!(symbolic_matrix_rows(rebuilt), 2);
    assert_eq!(symbolic_matrices_equivalent(eye, rebuilt), 1);

    // Out-of-bounds element access returns NULL.
    assert!(symbolic_matrix_element(eye, 2, 0).is_null());
    assert!(symbolic_matrix_element(eye, 0, 2).is_null());
    assert!(symbolic_matrix_element(std::ptr::null(), 0, 0).is_null());

    // Constructors reject shape/length mismatches and NULL handles.
    assert!(symbolic_matrix_new(2, 2, elements.as_ptr(), 3).is_null());
    assert!(symbolic_matrix_new(0, 2, elements.as_ptr(), 0).is_null());
    assert!(symbolic_matrix_new(2, 0, elements.as_ptr(), 0).is_null());
    assert!(symbolic_matrix_new(2, 2, std::ptr::null(), 4).is_null());
    let bad: [*const CSymbolicComplex; 4] = [e00, std::ptr::null(), e10, e11];
    assert!(symbolic_matrix_new(2, 2, bad.as_ptr(), 4).is_null());

    for handle in elements {
        symbolic_complex_free(handle as *mut CSymbolicComplex);
    }
    symbolic_matrix_free(eye);
    symbolic_matrix_free(rebuilt);
}

#[test]
fn symbolic_matrix_apply_standard_gate_matches_circuit() {
    // Reference: a 2-qubit circuit H(0); CX(1, 0).
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    let reference = circuit_to_symbolic_matrix(circuit, std::ptr::null(), 0);
    assert!(!reference.is_null());

    // Same unitary assembled in place from the identity matrix.
    let matrix = symbolic_eye(4);
    let bits: [usize; 1] = [0];
    assert_eq!(
        symbolic_matrix_apply_standard_gate(
            matrix,
            cstr("H").as_ptr(),
            bits.as_ptr(),
            1,
            std::ptr::null(),
            0,
        ),
        0
    );
    // `apply_gate_to_matrix` expects the target bits in the system's
    // Little-Endian order, and `circuit_to_symbolic_matrix` reverses the
    // circuit's [control, target] pair before applying, so
    // CX(control=0, target=1) maps to bits [1, 0].
    let two_bits: [usize; 2] = [1, 0];
    assert_eq!(
        symbolic_matrix_apply_standard_gate(
            matrix,
            cstr("CX").as_ptr(),
            two_bits.as_ptr(),
            2,
            std::ptr::null(),
            0,
        ),
        0
    );
    assert_eq!(symbolic_matrices_equivalent(reference, matrix), 1);

    // Error paths: NULL handles, unknown gate, bad bit arrays.
    assert_eq!(
        symbolic_matrix_apply_standard_gate(
            std::ptr::null_mut(),
            cstr("H").as_ptr(),
            bits.as_ptr(),
            1,
            std::ptr::null(),
            0,
        ),
        -1
    );
    assert_eq!(
        symbolic_matrix_apply_standard_gate(
            matrix,
            cstr("NOPE").as_ptr(),
            bits.as_ptr(),
            1,
            std::ptr::null(),
            0
        ),
        -8
    );
    assert_eq!(
        symbolic_matrix_apply_standard_gate(
            matrix,
            cstr("H").as_ptr(),
            std::ptr::null(),
            1,
            std::ptr::null(),
            0
        ),
        -8
    );
    let out_of_range: [usize; 1] = [9];
    assert_eq!(
        symbolic_matrix_apply_standard_gate(
            matrix,
            cstr("H").as_ptr(),
            out_of_range.as_ptr(),
            1,
            std::ptr::null(),
            0
        ),
        -8
    );
    let duplicated: [usize; 2] = [1, 1];
    assert_eq!(
        symbolic_matrix_apply_standard_gate(
            matrix,
            cstr("CX").as_ptr(),
            duplicated.as_ptr(),
            2,
            std::ptr::null(),
            0
        ),
        -8
    );

    // RX with one parameter works and NULL parameter handles are rejected.
    let rebuilt = symbolic_eye(4);
    let params: [*const CParameter; 1] = [param_from_double(0.5)];
    assert_eq!(
        symbolic_matrix_apply_standard_gate(
            rebuilt,
            cstr("RX").as_ptr(),
            bits.as_ptr(),
            1,
            params.as_ptr(),
            1,
        ),
        0
    );
    let null_params: [*const CParameter; 1] = [std::ptr::null()];
    assert_eq!(
        symbolic_matrix_apply_standard_gate(
            rebuilt,
            cstr("RX").as_ptr(),
            bits.as_ptr(),
            1,
            null_params.as_ptr(),
            1
        ),
        -1
    );

    param_free(params[0] as *mut CParameter);
    symbolic_matrix_free(matrix);
    symbolic_matrix_free(rebuilt);
    symbolic_matrix_free(reference);
    circuit_free(circuit);
}

#[test]
fn symbolic_matrix_apply_gate_and_single_qubit() {
    let x_gate = standard_gate_symbolic_matrix(cstr("X").as_ptr(), std::ptr::null(), 0);
    assert!(!x_gate.is_null());

    // symbolic apply_gate mirrors the numeric X application.
    let symbolic = symbolic_eye(4);
    let bit: [usize; 1] = [1];
    assert_eq!(
        symbolic_matrix_apply_gate(std::ptr::null_mut(), x_gate, bit.as_ptr(), 1),
        -1
    );
    assert_eq!(
        symbolic_matrix_apply_gate(symbolic, std::ptr::null(), bit.as_ptr(), 1),
        -1
    );
    assert_eq!(
        symbolic_matrix_apply_gate(symbolic, x_gate, std::ptr::null(), 1),
        -8
    );
    assert_eq!(
        symbolic_matrix_apply_gate(symbolic, x_gate, bit.as_ptr(), 1),
        0
    );

    let numeric = symbolic_eye(4);
    assert_eq!(
        symbolic_matrix_apply_gate_num(numeric, X_NUM.as_ptr(), 2, bit.as_ptr(), 1),
        0
    );
    // gate_dim must equal 1 << bits_len.
    assert_eq!(
        symbolic_matrix_apply_gate_num(numeric, X_NUM.as_ptr(), 4, bit.as_ptr(), 1),
        -8
    );
    // The gate dimension power-of-two overflow guard cannot be reached on
    // 64-bit targets, but a dim/len mismatch is checkable.
    assert_eq!(
        symbolic_matrix_apply_gate_num(std::ptr::null_mut(), X_NUM.as_ptr(), 2, bit.as_ptr(), 1),
        -1
    );

    // The two application paths agree numerically.
    assert_close(&evaluated(symbolic), &evaluated(numeric));
    // Applying X on bit 1 swaps matrix rows r <-> r ^ 2, so the identity
    // becomes the row-permutation matrix with values[4*r + c] = 1 iff
    // c == r ^ 2.
    let values = evaluated(numeric);
    let idx = |row: usize, col: usize| 4 * row + col;
    assert_eq!(values[idx(0b00, 0b10)], Complex64::new(1.0, 0.0));
    assert_eq!(values[idx(0b10, 0b00)], Complex64::new(1.0, 0.0));
    assert_eq!(values[idx(0b01, 0b11)], Complex64::new(1.0, 0.0));
    assert_eq!(values[idx(0b11, 0b01)], Complex64::new(1.0, 0.0));
    assert_eq!(values[idx(0b00, 0b00)], Complex64::new(0.0, 0.0));
    assert_eq!(values[idx(0b01, 0b01)], Complex64::new(0.0, 0.0));

    // single-qubit variants. A non-2x2 gate payload is rejected.
    let swapped = symbolic_eye(2);
    let cx_gate = standard_gate_symbolic_matrix(cstr("CX").as_ptr(), std::ptr::null(), 0);
    assert_eq!(
        symbolic_matrix_apply_single_qubit_gate(swapped, cx_gate, 0),
        -8 // gate must be 2x2
    );
    symbolic_matrix_free(cx_gate);
    assert_eq!(
        symbolic_matrix_apply_single_qubit_gate(swapped, x_gate, 0),
        0
    );
    assert_eq!(
        symbolic_matrix_apply_single_qubit_gate(swapped, x_gate, 5),
        -8 // bit out of range
    );

    let swapped_num = symbolic_eye(2);
    assert_eq!(
        symbolic_matrix_apply_single_qubit_gate_num(swapped_num, X_NUM.as_ptr(), 0),
        0
    );
    assert_eq!(
        symbolic_matrix_apply_single_qubit_gate_num(swapped_num, X_NUM.as_ptr(), 2),
        -8
    );
    assert_close(&evaluated(swapped), &evaluated(swapped_num));

    symbolic_matrix_free(swapped_num);
    symbolic_matrix_free(swapped);
    symbolic_matrix_free(numeric);
    symbolic_matrix_free(symbolic);
    symbolic_matrix_free(x_gate);
}

#[test]
fn symbolic_matrix_apply_two_qubit_and_general() {
    let cx_gate = standard_gate_symbolic_matrix(cstr("CX").as_ptr(), std::ptr::null(), 0);
    assert!(!cx_gate.is_null());

    // A 2-qubit symbolic gate must be 4x4 (CX is) and the bits distinct.
    let symbolic = symbolic_eye(4);
    assert_eq!(
        symbolic_matrix_apply_two_qubit_gate(std::ptr::null_mut(), cx_gate, 0, 1),
        -1
    );
    assert_eq!(
        symbolic_matrix_apply_two_qubit_gate(symbolic, std::ptr::null(), 0, 1),
        -1
    );
    assert_eq!(
        symbolic_matrix_apply_two_qubit_gate(symbolic, cx_gate, 0, 0),
        -8
    );
    assert_eq!(
        symbolic_matrix_apply_two_qubit_gate(symbolic, cx_gate, 0, 7),
        -8
    );
    assert_eq!(
        symbolic_matrix_apply_two_qubit_gate(symbolic, cx_gate, 0, 1),
        0
    );

    // The numeric payload is read back from the standard CX gate so both
    // paths use the exact same matrix.
    let cx_num = evaluated(cx_gate);

    let numeric = symbolic_eye(4);
    // NULL gate data.
    assert_eq!(
        symbolic_matrix_apply_two_qubit_gate_num(numeric, std::ptr::null(), 0, 1),
        -1
    );
    assert_eq!(
        symbolic_matrix_apply_two_qubit_gate_num(numeric, cx_num.as_ptr(), 1, 1),
        -8
    );
    assert_eq!(
        symbolic_matrix_apply_two_qubit_gate_num(std::ptr::null_mut(), cx_num.as_ptr(), 0, 1),
        -1
    );
    assert_eq!(
        symbolic_matrix_apply_two_qubit_gate_num(numeric, cx_num.as_ptr(), 0, 1),
        0
    );
    assert_close(&evaluated(symbolic), &evaluated(numeric));

    // Cross-check the two-qubit path against apply_standard_gate on a
    // freshly built identity: both produce the same CX permutation.
    let via_standard = symbolic_eye(4);
    let std_bits: [usize; 2] = [0, 1];
    assert_eq!(
        symbolic_matrix_apply_standard_gate(
            via_standard,
            cstr("CX").as_ptr(),
            std_bits.as_ptr(),
            2,
            std::ptr::null(),
            0,
        ),
        0
    );
    assert_close(&evaluated(via_standard), &evaluated(symbolic));

    // general-gate variants with a 2-bit target.
    let general = symbolic_eye(4);
    let bits: [usize; 2] = [0, 1];
    assert_eq!(
        symbolic_matrix_apply_general_gate(general, cx_gate, bits.as_ptr(), 2),
        0
    );
    assert_close(&evaluated(general), &evaluated(symbolic));

    // Shape mismatch on the general variant: a 2x2 gate with 2 bits.
    let x_gate = standard_gate_symbolic_matrix(cstr("X").as_ptr(), std::ptr::null(), 0);
    let bad_shape = symbolic_eye(4);
    assert_eq!(
        symbolic_matrix_apply_general_gate(bad_shape, x_gate, bits.as_ptr(), 2),
        -8
    );

    // General numeric application equals the symbolic one.
    let general_num = symbolic_eye(4);
    assert_eq!(
        symbolic_matrix_apply_general_gate_num(general_num, cx_num.as_ptr(), bits.as_ptr(), 2),
        0
    );
    assert_close(&evaluated(general), &evaluated(general_num));
    assert_eq!(
        symbolic_matrix_apply_general_gate_num(
            std::ptr::null_mut(),
            cx_num.as_ptr(),
            bits.as_ptr(),
            2
        ),
        -1
    );

    symbolic_matrix_free(general_num);
    symbolic_matrix_free(bad_shape);
    symbolic_matrix_free(x_gate);
    symbolic_matrix_free(via_standard);
    symbolic_matrix_free(general);
    symbolic_matrix_free(numeric);
    symbolic_matrix_free(symbolic);
    symbolic_matrix_free(cx_gate);
}

#[test]
fn symbolic_matrix_apply_permutation_and_diagonal() {
    // X on bit 0 equals the permutation (i -> (1 - i, one)).
    let one_a = symbolic_complex_one();
    let one_b = symbolic_complex_one();
    let symbolic = symbolic_eye(2);
    let bit: [usize; 1] = [0];
    let indices: [usize; 2] = [1, 0];
    let values: [*const CSymbolicComplex; 2] = [one_a, one_b];
    assert_eq!(
        symbolic_matrix_apply_permutation_gate(
            std::ptr::null_mut(),
            indices.as_ptr(),
            values.as_ptr(),
            2,
            bit.as_ptr(),
            1,
        ),
        -1
    );
    assert_eq!(
        symbolic_matrix_apply_permutation_gate(
            symbolic,
            std::ptr::null(),
            values.as_ptr(),
            2,
            bit.as_ptr(),
            1,
        ),
        -8
    );
    // len must equal 1 << bits_len.
    assert_eq!(
        symbolic_matrix_apply_permutation_gate(
            symbolic,
            indices.as_ptr(),
            values.as_ptr(),
            3,
            bit.as_ptr(),
            1,
        ),
        -8
    );
    // Source index beyond the gate dimension.
    let bad_index: [usize; 2] = [5, 0];
    assert_eq!(
        symbolic_matrix_apply_permutation_gate(
            symbolic,
            bad_index.as_ptr(),
            values.as_ptr(),
            2,
            bit.as_ptr(),
            1,
        ),
        -1
    );
    // NULL value handle.
    let null_values: [*const CSymbolicComplex; 2] = [one_a, std::ptr::null()];
    assert_eq!(
        symbolic_matrix_apply_permutation_gate(
            symbolic,
            indices.as_ptr(),
            null_values.as_ptr(),
            2,
            bit.as_ptr(),
            1,
        ),
        -1
    );
    assert_eq!(
        symbolic_matrix_apply_permutation_gate(
            symbolic,
            indices.as_ptr(),
            values.as_ptr(),
            2,
            bit.as_ptr(),
            1,
        ),
        0
    );
    // The permutation swaps rows 0 <-> 1 of the identity, i.e. it is X.
    let permuted = evaluated(symbolic);
    assert_eq!(permuted[0], Complex64::new(0.0, 0.0));
    assert_eq!(permuted[1], Complex64::new(1.0, 0.0));
    assert_eq!(permuted[2], Complex64::new(1.0, 0.0));
    assert_eq!(permuted[3], Complex64::new(0.0, 0.0));

    // Numeric permutation with factors (1, -1): output row 0 comes from
    // input row 1 unchanged and output row 1 is the negated input row 0.
    let perm_num = symbolic_eye(2);
    let factors: [Complex64; 2] = [Complex64::new(1.0, 0.0), Complex64::new(-1.0, 0.0)];
    assert_eq!(
        symbolic_matrix_apply_permutation_gate_num(
            perm_num,
            indices.as_ptr(),
            factors.as_ptr(),
            2,
            bit.as_ptr(),
            1,
        ),
        0
    );
    assert_eq!(
        symbolic_matrix_apply_permutation_gate_num(
            std::ptr::null_mut(),
            indices.as_ptr(),
            factors.as_ptr(),
            2,
            bit.as_ptr(),
            1,
        ),
        -1
    );
    assert_eq!(
        symbolic_matrix_apply_permutation_gate_num(
            perm_num,
            bad_index.as_ptr(),
            factors.as_ptr(),
            2,
            bit.as_ptr(),
            1,
        ),
        -8
    );
    let flipped = evaluated(perm_num);
    assert_eq!(flipped[0], Complex64::new(0.0, 0.0));
    assert_eq!(flipped[1], Complex64::new(1.0, 0.0));
    assert_eq!(flipped[2], Complex64::new(-1.0, 0.0));
    assert_eq!(flipped[3], Complex64::new(0.0, 0.0));

    // Symbolic diagonal: diag(1, i) rotated as the S gate on bit 0.
    let i_value = symbolic_complex_i();
    let diag_one = symbolic_complex_one();
    let diagonal = symbolic_eye(2);
    let diag_values: [*const CSymbolicComplex; 2] = [diag_one, i_value];
    assert_eq!(
        symbolic_matrix_apply_diagonal_gate(
            std::ptr::null_mut(),
            diag_values.as_ptr(),
            2,
            bit.as_ptr(),
            1,
        ),
        -1
    );
    // len/dim mismatch.
    assert_eq!(
        symbolic_matrix_apply_diagonal_gate(diagonal, diag_values.as_ptr(), 3, bit.as_ptr(), 1,),
        -8
    );
    assert_eq!(
        symbolic_matrix_apply_diagonal_gate(diagonal, std::ptr::null(), 2, bit.as_ptr(), 1),
        -8
    );
    assert_eq!(
        symbolic_matrix_apply_diagonal_gate(diagonal, diag_values.as_ptr(), 2, bit.as_ptr(), 1),
        0
    );
    let rotated = evaluated(diagonal);
    assert_eq!(rotated[0], Complex64::new(1.0, 0.0));
    assert_eq!(rotated[3], Complex64::new(0.0, 1.0));

    // Numeric diagonal with the same payload.
    let diagonal_num = symbolic_eye(2);
    let raw_diag: [Complex64; 2] = [Complex64::new(1.0, 0.0), Complex64::new(0.0, 1.0)];
    assert_eq!(
        symbolic_matrix_apply_diagonal_gate_num(
            diagonal_num,
            raw_diag.as_ptr(),
            2,
            bit.as_ptr(),
            1,
        ),
        0
    );
    assert_eq!(
        symbolic_matrix_apply_diagonal_gate_num(
            std::ptr::null_mut(),
            raw_diag.as_ptr(),
            2,
            bit.as_ptr(),
            1,
        ),
        -1
    );
    assert_eq!(
        symbolic_matrix_apply_diagonal_gate_num(
            diagonal_num,
            raw_diag.as_ptr(),
            5,
            bit.as_ptr(),
            1,
        ),
        -8
    );
    assert_close(&rotated, &evaluated(diagonal_num));

    symbolic_matrix_free(diagonal_num);
    symbolic_matrix_free(diagonal);
    symbolic_matrix_free(perm_num);
    symbolic_matrix_free(symbolic);
    symbolic_complex_free(i_value);
    symbolic_complex_free(diag_one);
    symbolic_complex_free(one_a);
    symbolic_complex_free(one_b);
}

#[test]
fn symbolic_matrix_columns_of_identity_unchanged() {
    // Sanity: applying gates never changes the matrix shape.
    let matrix = symbolic_eye(4);
    let bits: [usize; 2] = [0, 1];
    assert_eq!(
        symbolic_matrix_apply_standard_gate(
            matrix,
            cstr("CX").as_ptr(),
            bits.as_ptr(),
            2,
            std::ptr::null(),
            0,
        ),
        0
    );
    assert_eq!(symbolic_matrix_rows(matrix), 4);
    assert_eq!(symbolic_matrix_evaluate_len(matrix), 16);
    symbolic_matrix_free(matrix);
}
