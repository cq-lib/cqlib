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

use binding_c::circuit::*;
use binding_c::cqlib_string_free;
use std::ffi::CString;

fn cstr(text: &str) -> CString {
    CString::new(text).unwrap()
}

fn eval(ptr: *const CParameter, bindings: &str) -> f64 {
    param_evaluate(ptr, cstr(bindings).as_ptr())
}

#[test]
fn param_construction_and_constants() {
    let theta = param_symbol(cstr("theta").as_ptr());
    assert!(!theta.is_null());
    assert_eq!(param_is_constant(theta), 0);

    let name = param_as_symbol(theta);
    assert!(!name.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(name) },
        cstr("theta").as_c_str()
    );
    cqlib_string_free(name);

    let pi = param_pi();
    assert!((eval(pi, "") - std::f64::consts::PI).abs() < 1e-12);
    assert_eq!(param_is_constant(pi), 1);

    let e = param_e();
    assert!((eval(e, "") - std::f64::consts::E).abs() < 1e-12);

    let numeric = param_from_double(0.25);
    assert!((eval(numeric, "") - 0.25).abs() < 1e-12);
    assert_eq!(param_is_constant(numeric), 1);

    assert!(param_from_double(f64::NAN).is_null());
    assert!(param_from_double(f64::INFINITY).is_null());
    assert!(param_symbol(std::ptr::null()).is_null());

    param_free(theta);
    param_free(pi);
    param_free(e);
    param_free(numeric);
}

#[test]
fn param_arithmetic_operations() {
    let theta = param_symbol(cstr("theta").as_ptr());
    let phi = param_symbol(cstr("phi").as_ptr());

    let sum = param_add(theta, phi);
    assert!((eval(sum, "theta:0.5,phi:1.5") - 2.0).abs() < 1e-12);

    let diff = param_sub(theta, phi);
    assert!((eval(diff, "theta:0.5,phi:1.5") + 1.0).abs() < 1e-12);

    let prod = param_mul(theta, phi);
    assert!((eval(prod, "theta:0.5,phi:1.5") - 0.75).abs() < 1e-12);

    let quot = param_div(theta, phi);
    assert!((eval(quot, "theta:0.75,phi:1.5") - 0.5).abs() < 1e-12);

    let neg = param_neg(theta);
    assert!((eval(neg, "theta:0.5") + 0.5).abs() < 1e-12);

    assert!(param_add(std::ptr::null(), theta).is_null());
    assert!(param_sub(theta, std::ptr::null()).is_null());
    assert!(param_neg(std::ptr::null()).is_null());

    param_free(theta);
    param_free(phi);
    param_free(sum);
    param_free(diff);
    param_free(prod);
    param_free(quot);
    param_free(neg);
}

#[test]
fn param_math_functions() {
    let x = param_from_double(2.0);

    let cases: Vec<(*mut CParameter, f64)> = vec![
        (param_abs(param_neg(x)), 2.0),
        (param_sqrt(x), std::f64::consts::SQRT_2),
        (param_exp(x), (2.0_f64).exp()),
        (param_ln(param_from_double(1.0)), 0.0),
        (
            param_log(param_from_double(8.0), param_from_double(2.0)),
            3.0,
        ),
        (param_sin(param_from_double(0.5)), (0.5_f64).sin()),
        (param_cos(param_from_double(0.5)), (0.5_f64).cos()),
        (param_tan(param_from_double(0.5)), (0.5_f64).tan()),
        (param_asin(param_from_double(0.5)), (0.5_f64).asin()),
        (param_acos(param_from_double(0.5)), (0.5_f64).acos()),
        (param_atan(param_from_double(0.5)), (0.5_f64).atan()),
        (param_sinh(param_from_double(0.5)), (0.5_f64).sinh()),
        (param_cosh(param_from_double(0.5)), (0.5_f64).cosh()),
        (param_tanh(param_from_double(0.5)), (0.5_f64).tanh()),
        (param_floor(param_from_double(1.75)), 1.0),
        (param_ceil(param_from_double(1.25)), 2.0),
        (param_round(param_from_double(1.75)), 2.0),
        (param_pow(x, param_from_double(3.0)), 8.0),
    ];
    for (ptr, expected) in &cases {
        assert!((eval(*ptr, "") - expected).abs() < 1e-12);
        param_free(*ptr);
    }

    assert!(param_sin(std::ptr::null()).is_null());
    assert!(param_log(x, std::ptr::null()).is_null());
    assert!(param_pow(std::ptr::null(), x).is_null());
    param_free(x);
}

#[test]
fn param_symbol_queries() {
    let theta = param_symbol(cstr("theta").as_ptr());
    let expr = param_parse(cstr("phi + theta * 2").as_ptr());

    assert_eq!(param_symbols_len(expr), 2);
    let mut names: Vec<*mut std::os::raw::c_char> = vec![std::ptr::null_mut(); 2];
    assert_eq!(param_symbols(expr, names.as_mut_ptr(), 2), 0);
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(names[0]) },
        cstr("phi").as_c_str()
    );
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(names[1]) },
        cstr("theta").as_c_str()
    );
    for name in names {
        cqlib_string_free(name);
    }

    assert_eq!(param_symbols(expr, std::ptr::null_mut(), 1), -8);
    assert_eq!(param_symbols(std::ptr::null(), std::ptr::null_mut(), 0), -1);
    assert_eq!(param_symbols_len(std::ptr::null()), 0);
    assert!(param_as_symbol(expr).is_null());
    assert!(param_as_symbol(std::ptr::null()).is_null());

    let single = param_as_symbol(theta);
    assert!(!single.is_null());
    cqlib_string_free(single);

    let constant = param_from_double(1.0);
    assert_eq!(param_symbols(constant, std::ptr::null_mut(), 0), 0);

    param_free(theta);
    param_free(expr);
    param_free(constant);
}

#[test]
fn param_state_checks() {
    let zero = param_from_double(0.0);
    let one = param_from_double(1.0);
    let theta = param_symbol(cstr("theta").as_ptr());

    assert_eq!(param_is_exact_zero(zero), 1);
    assert_eq!(param_is_exact_zero(one), 0);
    assert_eq!(param_is_exact_zero(theta), 0);
    assert_eq!(param_is_zero(zero), 1);
    assert_eq!(param_is_one(one), 1);
    assert_eq!(param_is_one(zero), 0);
    assert_eq!(param_is_zero(theta), 0);

    assert_eq!(param_is_constant(std::ptr::null()), -1);
    assert_eq!(param_is_exact_zero(std::ptr::null()), -1);
    assert_eq!(param_is_zero(std::ptr::null()), -1);
    assert_eq!(param_is_one(std::ptr::null()), -1);

    param_free(zero);
    param_free(one);
    param_free(theta);
}

#[test]
fn param_simplify_canonicalize_and_substitute() {
    let theta = param_symbol(cstr("theta").as_ptr());
    let expr = param_add(theta, param_from_double(0.0));

    let simplified = param_simplify(expr);
    assert!(!simplified.is_null());
    assert_eq!(param_provably_equal(simplified, theta, 1e-12), 1);

    let canonical = param_canonicalized(param_parse(cstr("pi / 2").as_ptr()));
    assert!(!canonical.is_null());
    assert_eq!(param_is_constant(canonical), 1);
    assert!((eval(canonical, "") - std::f64::consts::FRAC_PI_2).abs() < 1e-12);

    let replaced = param_replace(
        param_parse(cstr("x + 2.0").as_ptr()),
        cstr("x").as_ptr(),
        param_from_double(1.0),
    );
    assert!((eval(replaced, "") - 3.0).abs() < 1e-12);

    let name_x = cstr("x");
    let name_y = cstr("y");
    let names = [name_x.as_ptr(), name_y.as_ptr()];
    let values: [*const CParameter; 2] = [param_from_double(2.0), param_from_double(3.0)];
    let sum = param_substitute_many(
        param_parse(cstr("x + y").as_ptr()),
        names.as_ptr(),
        values.as_ptr(),
        2,
    );
    assert!((eval(sum, "") - 5.0).abs() < 1e-12);

    assert!(param_substitute_many(std::ptr::null(), names.as_ptr(), values.as_ptr(), 2).is_null());
    assert!(param_replace(std::ptr::null(), cstr("x").as_ptr(), values[0]).is_null());
    assert!(param_simplify(std::ptr::null()).is_null());
    assert!(param_canonicalized(std::ptr::null()).is_null());

    for ptr in [simplified, canonical, replaced, sum] {
        param_free(ptr);
    }
    param_free(expr);
    param_free(theta);
    for value in values {
        param_free(value as *mut CParameter);
    }
}

#[test]
fn param_symbolic_derivative() {
    let theta = param_symbol(cstr("theta").as_ptr());
    let expr = param_mul(theta, theta);

    let deriv = param_derivative(expr, cstr("theta").as_ptr());
    assert!(!deriv.is_null());
    assert!((eval(deriv, "theta:3.0") - 6.0).abs() < 1e-12);

    assert!(param_derivative(std::ptr::null(), cstr("theta").as_ptr()).is_null());
    assert!(param_derivative(expr, std::ptr::null()).is_null());

    param_free(deriv);
    param_free(expr);
    param_free(theta);
}

#[test]
fn param_provable_equivalence() {
    let half = param_from_double(0.5);
    let shifted = param_from_double(0.5 + 2.0 * std::f64::consts::PI);
    let two_pi = param_from_double(2.0 * std::f64::consts::PI);

    assert_eq!(param_provably_equal(half, half, 1e-12), 1);
    assert_eq!(param_provably_equal(half, param_from_double(0.6), 1e-12), 0);
    assert_eq!(param_provably_equal_modulo(half, shifted, two_pi, 1e-12), 1);
    assert_eq!(
        param_provably_equal_modulo(half, param_from_double(0.6), two_pi, 1e-12),
        0
    );

    assert_eq!(param_provably_equal(std::ptr::null(), half, 1e-12), -1);
    assert_eq!(
        param_provably_equal_modulo(std::ptr::null(), half, two_pi, 1e-12),
        -1
    );

    param_free(half);
    param_free(shifted);
    param_free(two_pi);
}
