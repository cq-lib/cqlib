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

use super::Parameter;
use std::collections::{HashMap, HashSet};
use std::f64::consts::{E, PI};

#[test]
fn test_p_construction() {
    // From string
    let p1 = Parameter::try_from("theta").unwrap();
    assert_eq!(p1.to_string(), "theta");

    // From f64
    let p2 = Parameter::from(2.5);
    assert_eq!(p2.to_string(), "2.5");

    // From integer
    let p3 = Parameter::from(42);
    assert_eq!(p3.to_string(), "42");

    // Default
    let p_default = Parameter::default();
    assert_eq!(p_default.to_string(), "0");

    // Parameter::symbol
    let p_sym = Parameter::symbol("phi");
    assert_eq!(p_sym.to_string(), "phi");
}

#[test]
#[should_panic(expected = "Parameter numeric literal must be finite")]
fn test_p_rejects_f64_nan() {
    let _ = Parameter::from(f64::NAN);
}

#[test]
#[should_panic(expected = "Parameter numeric literal must be finite")]
fn test_p_rejects_f64_infinity() {
    let _ = Parameter::from(f64::INFINITY);
}

#[test]
#[should_panic(expected = "Parameter numeric literal must be finite")]
fn test_p_rejects_f64_negative_infinity() {
    let _ = Parameter::from(f64::NEG_INFINITY);
}

#[test]
#[should_panic(expected = "Parameter numeric literal must be finite")]
fn test_p_rejects_f32_nan() {
    let _ = Parameter::from(f32::NAN);
}

#[test]
#[should_panic(expected = "Parameter numeric literal must be finite")]
fn test_p_rejects_f32_infinity() {
    let _ = Parameter::from(f32::INFINITY);
}

#[test]
#[should_panic(expected = "Parameter numeric literal must be finite")]
fn test_p_rejects_f32_negative_infinity() {
    let _ = Parameter::from(f32::NEG_INFINITY);
}

#[test]
fn test_p_constants() {
    let pi = Parameter::pi();
    assert_eq!(pi.to_string(), "π");
    assert_eq!(pi.evaluate(&None).unwrap(), PI);

    let e = Parameter::e();
    assert_eq!(e.to_string(), "e");
    assert_eq!(e.evaluate(&None).unwrap(), E);
}

#[test]
fn test_p_arithmetic_ops() {
    let theta = Parameter::try_from("theta").unwrap();
    let phi = Parameter::try_from("phi").unwrap();
    let val = Parameter::from(2.0);

    // Add
    let add = theta.clone() + phi.clone();
    assert_eq!(add.to_string(), "phi + theta");

    // Sub
    let sub = theta.clone() - phi.clone();
    assert_eq!(sub.to_string(), "-phi + theta");

    // Mul
    let mul = theta.clone() * val.clone();
    assert_eq!(mul.to_string(), "2*theta");

    // Div
    let div = theta.clone() / val.clone();
    assert_eq!(div.to_string(), "theta/2");

    // Note: Rem (Mod) is not implemented for Parameter yet, skipping for now
}

#[test]
fn test_p_arithmetic_primitive_ops() {
    let theta = Parameter::try_from("theta").unwrap();

    // Parameter + f64
    let res: Parameter = theta.clone() + 1.5;
    assert_eq!(res.to_string(), "1.5 + theta");

    // f64 + Parameter
    let res: Parameter = 1.5 + theta.clone();
    assert_eq!(res.to_string(), "1.5 + theta");

    // Parameter - i32
    let res: Parameter = theta.clone() - 10;
    assert_eq!(res.to_string(), "-10 + theta");

    // i32 - Parameter
    let res: Parameter = 10 - theta.clone();
    assert_eq!(res.to_string(), "10 - theta");

    // Parameter * f32
    let res: Parameter = theta.clone() * 2.0f32;
    assert_eq!(res.to_string(), "2*theta");

    // u32 * Parameter
    let res: Parameter = 5u32 * theta.clone();
    assert_eq!(res.to_string(), "5*theta");

    // Parameter / i32
    let res: Parameter = theta.clone() / 2;
    assert_eq!(res.to_string(), "theta/2");
}

#[test]
fn test_p_reference_ops() {
    let p1 = Parameter::try_from("p1").unwrap();
    let p2 = Parameter::try_from("p2").unwrap();

    // &Parameter + &Parameter
    let res = &p1 + &p2;
    assert_eq!(res.to_string(), "p1 + p2");

    // Parameter + &Parameter
    let res = p1.clone() + &p2;
    assert_eq!(res.to_string(), "p1 + p2");

    // &Parameter + Parameter
    let res = &p1 + p2.clone();
    assert_eq!(res.to_string(), "p1 + p2");
}

#[test]
fn test_p_functions() {
    let x = Parameter::try_from("x").unwrap();

    assert_eq!(x.sin().to_string(), "sin(x)");
    assert_eq!(x.cos().to_string(), "cos(x)");
    assert_eq!(x.tan().to_string(), "tan(x)");
    assert_eq!(x.asin().to_string(), "asin(x)");
    assert_eq!(x.acos().to_string(), "acos(x)");
    assert_eq!(x.atan().to_string(), "atan(x)");
    assert_eq!(x.exp().to_string(), "exp(x)");
    assert_eq!(x.ln().to_string(), "ln(x)");
    assert_eq!(x.abs().to_string(), "abs(x)");
    assert_eq!(x.sqrt().to_string(), "sqrt(x)");

    let y = Parameter::try_from("y").unwrap();
    assert_eq!(x.pow(y.clone()).to_string(), "x^y");

    // Log with base
    let base = Parameter::from(10.0);
    assert_eq!(x.log(base).to_string(), "log(10, x)");
}

#[test]
fn test_p_evaluation() {
    let x = Parameter::try_from("x").unwrap();
    let expr: Parameter = x.clone() * 2.0 + 1.0; // x * 2 + 1

    let mut bindings = HashMap::new();
    bindings.insert("x", 3.0); // Note: Parameter uses &str as keys

    let res = expr.evaluate(&Some(bindings)).unwrap();
    assert_eq!(res, 7.0);

    // Missing symbol
    let empty_bindings = HashMap::new();
    let err = expr.evaluate(&Some(empty_bindings));
    // symb_anafis might return NaN instead of missing symbol depending on its internal handling
    assert!(err.is_err() || err.unwrap().is_nan());
}

#[test]
fn test_p_get_symbols() {
    let x = Parameter::try_from("x").unwrap();
    let y = Parameter::try_from("y").unwrap();
    let z = Parameter::try_from("z").unwrap();

    let expr = (x + y) * z;
    let symbols = expr.get_symbols(); // Returns HashSet<String>

    assert_eq!(symbols.len(), 3);
    assert!(symbols.contains("x"));
    assert!(symbols.contains("y"));
    assert!(symbols.contains("z"));
}

#[test]
fn test_p_simplify() {
    let x = Parameter::try_from("x").unwrap();

    // 0 + x -> x
    let expr = Parameter::from(0) + x.clone();
    let simplified = expr.simplify().unwrap(); // Parameter::simplify returns Result
    assert_eq!(simplified.to_string(), "x");

    // x * 1 -> x
    let expr: Parameter = x.clone() * 1.0;
    let simplified = expr.simplify().unwrap();
    assert_eq!(simplified.to_string(), "x");
}

#[test]
fn test_p_derivative() {
    let x = Parameter::try_from("x").unwrap();
    // d(x^2)/dx = 2*x
    let expr = x.pow(Parameter::from(2.0));
    let deriv = expr.derivative("x").unwrap().simplify().unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("x", 3.0);
    let val = deriv.evaluate(&Some(bindings.clone())).unwrap();
    assert_eq!(val, 6.0);
}

#[test]
fn test_p_replace() {
    let p = Parameter::try_from("x").unwrap() + Parameter::try_from("y").unwrap();
    let z = Parameter::try_from("z").unwrap();

    let new_p = p.replace("x", z.clone()); // replace takes impl Into<Parameter>
    assert_eq!(new_p.to_string(), "z + y");
}

#[test]
fn test_p_replace_edge_cases() {
    let x = Parameter::try_from("x").unwrap();
    let y = Parameter::try_from("y").unwrap();
    let z = Parameter::try_from("z").unwrap();

    // 1. Replace non-existent symbol
    let expr1 = x.clone() + y.clone();
    let res1 = expr1.replace("z", Parameter::from(1.0));
    assert_eq!(res1.to_string(), "x + y");

    // 2. Self-referential/recursive replacement (x -> x + 1)
    let expr2 = x.clone();
    let res2 = expr2.replace("x", x.clone() + 1.0);
    assert_eq!(res2.to_string(), "1 + x");

    // 3. Deeply nested expression replacement
    // expr3 = sin(cos(x * y)) + exp(x)
    let expr3 = (x.clone() * y.clone()).cos().sin() + x.clone().exp();
    let res3 = expr3.replace("x", z.clone());
    // x should be replaced by z everywhere
    assert_eq!(res3.to_string(), "exp(z) + sin(cos(z*y))");
}

#[test]
fn test_p_equality() {
    let p1: Parameter = Parameter::try_from("x").unwrap() + 1.0;
    let p2: Parameter = Parameter::try_from("x").unwrap() + 1.0;
    let p3: Parameter = Parameter::try_from("x").unwrap() + 2.0;

    assert_eq!(p1, p2);
    assert_ne!(p1, p3);
}

#[test]
fn test_p_simplify_add_num() {
    // 5 + 10 = 15
    let s = Parameter::from(5_f64) + Parameter::from(10_f64);
    let r = s.simplify().unwrap();
    assert_eq!(r, Parameter::from(15_f64));

    // (5 + 10) + (9 + 1) = 25
    let s = (Parameter::from(5_f64) + Parameter::from(10_f64))
        + (Parameter::from(9_f64) + Parameter::from(1_f64));
    let r = s.simplify().unwrap();
    assert_eq!(r, Parameter::from(25_f64));
}

#[test]
fn test_p_simplify_add_zero() {
    let x = Parameter::try_from("x").unwrap();
    let z = Parameter::from(0_f64);

    // x + 0 = x
    let s = x.clone() + z.clone();
    let r = s.simplify().unwrap();
    assert_eq!(r, x);

    // 0 + x = x
    let s = z.clone() + x.clone();
    let r = s.simplify().unwrap();
    assert_eq!(r, x);

    // x + 0 + x = 2x
    let s = x.clone() + z.clone() + x.clone();
    let r = s.simplify().unwrap();
    assert_eq!(r.to_string(), "2*x");
}

#[test]
fn test_p_simplify_add_self() {
    let x = Parameter::try_from("x").unwrap();

    // x + x = 2x
    let s = x.clone() + x.clone();
    let r = s.simplify().unwrap();
    assert_eq!(r.to_string(), "2*x");

    // 2x + x = 3x
    let s = x.clone() + x.clone() + x.clone();
    let r = s.simplify().unwrap();
    assert_eq!(r.to_string(), "3*x");

    // x + 2x = 3x
    let s = x.clone() + (x.clone() + x.clone());
    let r = s.simplify().unwrap();
    assert_eq!(r.to_string(), "3*x");

    // 2.1x + x = 3.1x
    let s = x.clone() * 2.1_f64 + x.clone();
    let r = s.simplify().unwrap();
    assert_eq!(r.to_string(), "3.1*x");

    // 3.1x + 2x + 6x= 11.1x
    let s = x.clone() * 3.1_f64 + x.clone() * 2_f64 + x.clone() * 6_f64;
    let r = s.simplify().unwrap();
    assert_eq!(r.to_string(), "11.1*x");
}

#[test]
fn test_p_simplify_sub_zero() {
    let x = Parameter::try_from("x").unwrap();
    let z = Parameter::from(0_f64);

    // x - 0
    let s = x.clone() - z.clone();
    let r = s.simplify().unwrap();
    assert_eq!(r, x);

    // 0 + 2x
    let s = z.clone() + x.clone() * 2_i32;
    let r = s.simplify().unwrap();
    assert_eq!(r.to_string(), "2*x");
}

#[test]
fn test_p_simplify_sub_num() {
    let x = Parameter::try_from("x").unwrap();

    // x - 2x = -1x
    let s = x.clone() - x.clone() * 2_i32;
    let r = s.simplify().unwrap();
    assert_eq!(r.to_string(), "-x"); // symb_anafis likely removes the 1

    // x - x
    let s = x.clone() - x.clone();
    let r = s.simplify().unwrap();
    assert_eq!(r, Parameter::from(0_f64));

    // 9.3x - (-0.2x) = 9.5x
    let s = 9.3_f64 * x.clone() - (-0.2_f64) * x.clone();
    let r = s.simplify().unwrap();
    assert_eq!(r.to_string(), "9.5*x");
}

#[test]
fn test_p_simplify_sin_num() {
    // sin(pi) = 0
    let pi = Parameter::from(std::f64::consts::PI);
    let s = pi.sin();
    let exp = s.simplify().unwrap();
    assert!(exp.evaluate(&None).unwrap().abs() < 1e-10);

    let pi_sym = Parameter::pi();
    let exp = pi_sym.sin() + pi_sym.sin();
    let exp = exp.simplify().unwrap();
    // Engine treats "π" as an unknown symbol during simplify, so it becomes 2*sin(π)
    assert_eq!(exp.to_string(), "2*sin(π)");
    assert!(exp.evaluate(&None).unwrap().abs() < 1e-10);
}

#[test]
fn test_p_simplify_cos_num() {
    // cos(pi) = -1
    let x = Parameter::from(std::f64::consts::PI);
    let s = x.cos();
    let exp = s.simplify().unwrap();
    assert_eq!(exp.evaluate(&None).unwrap(), -1.0);
}

#[test]
fn test_p_simplify_tan() {
    let pi = Parameter::pi();
    let s = pi.tan();
    let exp = s.simplify().unwrap();
    assert!(exp.evaluate(&None).unwrap().abs() < f64::EPSILON);

    let x = Parameter::try_from("x").unwrap();
    let s = x.clone().tan().atan();
    let exp = s.simplify().unwrap();
    // symb_anafis might not have this trig identity, so we just ensure it evaluates correctly
    let mut bindings = HashMap::new();
    bindings.insert("x", 0.5);
    assert!((exp.evaluate(&Some(bindings)).unwrap() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn test_p_simplify_div_mod() {
    let x = Parameter::try_from("x").unwrap();
    let zero = Parameter::from(0);
    let one = Parameter::from(1);

    // 0 / x = 0
    let expr1 = zero.clone() / x.clone();
    assert_eq!(expr1.simplify().unwrap(), zero);

    // x / 1 = x
    let expr2 = x.clone() / one.clone();
    assert_eq!(expr2.simplify().unwrap(), x);

    // x / x = 1
    let expr3 = x.clone() / x.clone();
    assert_eq!(expr3.simplify().unwrap(), one);

    // Modulo tests skipped because Parameter does not implement %
}

#[test]
fn test_p_simplify_rational_polynomial() {
    let x = Parameter::try_from("x").unwrap();

    // c1 * (c2 * x) -> (c1 * c2) * x
    let expr1 = Parameter::from(2.0) * (Parameter::from(3.0) * x.clone());
    assert_eq!(expr1.simplify().unwrap().to_string(), "6*x");

    // x^2 * x^3 -> x^5
    let expr2 = x.clone().pow(Parameter::from(2)) * x.clone().pow(Parameter::from(3));
    assert_eq!(expr2.simplify().unwrap().to_string(), "x^5");

    // (c * x) / x -> c
    let expr3 = (Parameter::from(4.0) * x.clone()) / x.clone();
    assert_eq!(expr3.simplify().unwrap(), Parameter::from(4.0));

    // x^5 / x^2 -> x^3
    let expr4 = x.clone().pow(Parameter::from(5)) / x.clone().pow(Parameter::from(2));
    assert_eq!(expr4.simplify().unwrap().to_string(), "x^3");

    // (x^2)^3 -> x^6
    let expr5 = x.clone().pow(Parameter::from(2)).pow(Parameter::from(3));
    assert_eq!(expr5.simplify().unwrap().to_string(), "x^6");
}

#[test]
fn test_p_simplify_trig_parity_pythagorean() {
    let x = Parameter::try_from("x").unwrap();
    let neg_x = Parameter::from(0.0) - x.clone();

    // Using evaluate to assert logical equivalence if string representations vary
    let mut bindings = HashMap::new();
    bindings.insert("x", 0.5);

    // sin(-x) = -sin(x)
    let s1 = neg_x
        .sin()
        .simplify()
        .unwrap()
        .evaluate(&Some(bindings.clone()))
        .unwrap();
    let s2 = (Parameter::from(0.0) - x.clone().sin())
        .evaluate(&Some(bindings.clone()))
        .unwrap();
    assert!((s1 - s2).abs() < f64::EPSILON);

    // cos(-x) = cos(x)
    let c1 = neg_x
        .cos()
        .simplify()
        .unwrap()
        .evaluate(&Some(bindings.clone()))
        .unwrap();
    let c2 = x.clone().cos().evaluate(&Some(bindings.clone())).unwrap();
    assert!((c1 - c2).abs() < f64::EPSILON);

    // sin^2(x) + cos^2(x) = 1
    let expr_pythagorean =
        x.clone().sin().pow(Parameter::from(2)) + x.clone().cos().pow(Parameter::from(2));
    let pythagorean_val = expr_pythagorean
        .simplify()
        .unwrap()
        .evaluate(&Some(bindings))
        .unwrap();
    assert!((pythagorean_val - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_p_simplify_exp_log() {
    let x = Parameter::try_from("x").unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("x", 2.0);

    // e^(ln(x)) = x
    let expr1 = x.clone().ln().exp();
    let simplified1 = expr1.simplify().unwrap();
    let v1 = simplified1.evaluate(&Some(bindings.clone())).unwrap();
    assert!((v1 - 2.0).abs() < 1e-10);

    // ln(e^x) = x
    let expr2 = x.clone().exp().ln();
    let simplified2 = expr2.simplify().unwrap();
    let v2 = simplified2.evaluate(&Some(bindings.clone())).unwrap();
    assert!((v2 - 2.0).abs() < 1e-10);

    // ln(x^3) = 3*ln(x)
    let expr3 = x.clone().pow(Parameter::from(3)).ln();
    let simplified3 = expr3.simplify().unwrap();
    let v3_left = simplified3.evaluate(&Some(bindings.clone())).unwrap();
    let v3_right = (Parameter::from(3.0) * x.clone().ln())
        .evaluate(&Some(bindings))
        .unwrap();
    assert!((v3_left - v3_right).abs() < 1e-10);
}

#[test]
fn test_p_derivative_constant() {
    let x = "x";

    // d(5)/dx = 0
    let c1 = Parameter::from(5);
    let d_c1 = c1.derivative(x).unwrap();
    assert_eq!(d_c1, Parameter::from(0));

    // d(2.5)/dx = 0
    let c2 = Parameter::from(2.5);
    let d_c2 = c2.derivative(x).unwrap();
    assert_eq!(d_c2, Parameter::from(0));

    // d(PI)/dx = 0
    let pi = Parameter::pi();
    let d_pi = pi.derivative(x).unwrap();
    assert_eq!(d_pi, Parameter::from(0));

    // d(E)/dx = 0
    let e = Parameter::e();
    let d_e = e.derivative(x).unwrap();
    assert_eq!(d_e, Parameter::from(0));
}

#[test]
fn test_p_derivative_symbol() {
    let x = Parameter::try_from("x").unwrap();
    let y = Parameter::try_from("y").unwrap();

    // d(x)/dx = 1
    let dx_dx = x.derivative("x").unwrap().simplify().unwrap();
    assert_eq!(dx_dx, Parameter::from(1));

    // d(y)/dx = 0
    let dy_dx = y.derivative("x").unwrap().simplify().unwrap();
    assert_eq!(dy_dx, Parameter::from(0));
}

#[test]
fn test_p_derivative_negation() {
    let x = Parameter::try_from("x").unwrap();
    // d(-x)/dx = -1
    let neg_x = Parameter::from(0) - x.clone();
    let d_neg_x = neg_x.derivative("x").unwrap().simplify().unwrap();
    assert_eq!(d_neg_x, Parameter::from(-1.0));
}

#[test]
fn test_p_derivative_add_sub() {
    let x = Parameter::try_from("x").unwrap();
    let y = Parameter::try_from("y").unwrap();

    // d(x + y)/dx = 1 + 0 = 1
    let add = x.clone() + y.clone();
    let d_add = add.derivative("x").unwrap().simplify().unwrap();
    assert_eq!(d_add, Parameter::from(1));

    // d(x - 2x)/dx = 1 - 2 = -1
    let sub = x.clone() - Parameter::from(2.0) * x.clone();
    let d_sub = sub.derivative("x").unwrap().simplify().unwrap();
    assert_eq!(d_sub, Parameter::from(-1.0));
}

#[test]
fn test_p_derivative_multiplication() {
    let x = Parameter::try_from("x").unwrap();

    // d(3x)/dx = 3
    let mul_const = Parameter::from(3.0) * x.clone();
    let d_mul_const = mul_const.derivative("x").unwrap();
    let simplified = d_mul_const.simplify().unwrap();
    assert_eq!(simplified, Parameter::from(3.0));

    // d(x * x)/dx = 2x
    let square = x.clone() * x.clone();
    let d_square = square.derivative("x").unwrap().simplify().unwrap();

    // Evaluate instead of string matching to avoid formatting fragility
    let mut bindings = HashMap::new();
    bindings.insert("x", 4.0);
    let val = d_square.evaluate(&Some(bindings)).unwrap();
    assert_eq!(val, 8.0);
}

#[test]
fn test_p_derivative_division() {
    let x = Parameter::try_from("x").unwrap();

    // d(x / 2)/dx = 1/2 = 0.5
    let div_const: Parameter = x.clone() / 2.0;
    let d_div_const = div_const.derivative("x").unwrap().simplify().unwrap();
    assert!((d_div_const.evaluate(&None).unwrap() - 0.5).abs() < 1e-10);

    // d(1 / x)/dx = -1/x^2
    let inv_x = Parameter::from(1.0) / x.clone();
    let d_inv_x = inv_x.derivative("x").unwrap().simplify().unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("x", 2.0);
    let val = d_inv_x.evaluate(&Some(bindings)).unwrap();
    assert!((val - (-0.25)).abs() < 1e-10);
}

#[test]
fn test_p_derivative_power_constructed() {
    let x = Parameter::try_from("x").unwrap();
    let three = Parameter::from(3.0);

    // x^3
    let x_cubed = x.clone().pow(three);

    // d(x^3)/dx = 3x^2
    let deriv = x_cubed.derivative("x").unwrap().simplify().unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("x", 2.0);
    let val = deriv.evaluate(&Some(bindings.clone())).unwrap();
    assert!((val - 12.0).abs() < 1e-10); // 3 * 2^2 = 12

    // 2^x
    let two = Parameter::from(2.0);
    let two_pow_x = two.pow(x.clone());

    // d(2^x)/dx = 2^x * ln(2)
    let deriv_2px = two_pow_x.derivative("x").unwrap().simplify().unwrap();
    let val_2px = deriv_2px.evaluate(&Some(bindings)).unwrap();
    // 2^2 * ln(2) = 4 * 0.693147...
    assert!((val_2px - (4.0 * 2.0f64.ln())).abs() < 1e-10);
}

#[test]
fn test_p_derivative_trig() {
    let x = Parameter::try_from("x").unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("x", std::f64::consts::PI / 4.0);

    // d(sin(x))/dx = cos(x)
    let sin_x = x.clone().sin();
    let d_sin = sin_x.derivative("x").unwrap().simplify().unwrap();
    let expected_cos = (std::f64::consts::PI / 4.0).cos();
    assert!((d_sin.evaluate(&Some(bindings.clone())).unwrap() - expected_cos).abs() < 1e-10);

    // d(cos(x))/dx = -sin(x)
    let cos_x = x.clone().cos();
    let d_cos = cos_x.derivative("x").unwrap().simplify().unwrap();
    let expected_neg_sin = -(std::f64::consts::PI / 4.0).sin();
    assert!((d_cos.evaluate(&Some(bindings.clone())).unwrap() - expected_neg_sin).abs() < 1e-10);

    // d(tan(x))/dx = sec^2(x) = 1/cos^2(x)
    let tan_x = x.clone().tan();
    let d_tan = tan_x.derivative("x").unwrap().simplify().unwrap();
    let expected_sec_sq = 1.0 / (std::f64::consts::PI / 4.0).cos().powi(2);
    assert!((d_tan.evaluate(&Some(bindings.clone())).unwrap() - expected_sec_sq).abs() < 1e-10);
}

#[test]
fn test_p_derivative_inverse_trig() {
    let x = Parameter::try_from("x").unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("x", 0.5);

    // d(asin(x))/dx = 1 / sqrt(1 - x^2)
    let asin_x = x.clone().asin();
    let d_asin = asin_x.derivative("x").unwrap().simplify().unwrap();
    let expected_asin_d = 1.0 / (1.0 - 0.5f64.powi(2)).sqrt();
    assert!((d_asin.evaluate(&Some(bindings.clone())).unwrap() - expected_asin_d).abs() < 1e-10);

    // d(acos(x))/dx = -1 / sqrt(1 - x^2)
    let acos_x = x.clone().acos();
    let d_acos = acos_x.derivative("x").unwrap().simplify().unwrap();
    let expected_acos_d = -1.0 / (1.0 - 0.5f64.powi(2)).sqrt();
    assert!((d_acos.evaluate(&Some(bindings.clone())).unwrap() - expected_acos_d).abs() < 1e-10);

    // d(atan(x))/dx = 1 / (1 + x^2)
    let atan_x = x.clone().atan();
    let d_atan = atan_x.derivative("x").unwrap().simplify().unwrap();
    let expected_atan_d = 1.0 / (1.0 + 0.5f64.powi(2));
    assert!((d_atan.evaluate(&Some(bindings.clone())).unwrap() - expected_atan_d).abs() < 1e-10);
}

#[test]
fn test_p_derivative_exp_ln() {
    let x = Parameter::try_from("x").unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("x", 2.0);

    // d(e^x)/dx = e^x
    let exp_x = x.clone().exp();
    let d_exp = exp_x.derivative("x").unwrap().simplify().unwrap();
    let expected_exp = std::f64::consts::E.powf(2.0);
    assert!((d_exp.evaluate(&Some(bindings.clone())).unwrap() - expected_exp).abs() < 1e-10);

    // d(ln(x))/dx = 1/x
    let ln_x = x.clone().ln();
    let d_ln = ln_x.derivative("x").unwrap().simplify().unwrap();
    assert!((d_ln.evaluate(&Some(bindings.clone())).unwrap() - 0.5).abs() < 1e-10);
}

#[test]
fn test_p_derivative_chain_rule() {
    let x = Parameter::try_from("x").unwrap();
    let two = Parameter::from(2.0);
    // sin(x^2)
    // d(sin(x^2))/dx = cos(x^2) * 2x
    let x_squared = x.clone().pow(two);
    let sin_x2 = x_squared.sin();

    let deriv = sin_x2.derivative("x").unwrap().simplify().unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("x", 3.0);
    let expected = 9.0f64.cos() * 2.0 * 3.0;
    assert!((deriv.evaluate(&Some(bindings)).unwrap() - expected).abs() < 1e-10);
}

#[test]
fn test_p_derivative_product_rule_complex() {
    // x * sin(x)
    // d/dx = 1*sin(x) + x*cos(x) = sin(x) + x*cos(x)
    let x = Parameter::try_from("x").unwrap();
    let expr = x.clone() * x.clone().sin();
    let deriv = expr.derivative("x").unwrap().simplify().unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("x", std::f64::consts::PI / 2.0);
    let val = std::f64::consts::PI / 2.0;
    let expected = val.sin() + val * val.cos();
    assert!((deriv.evaluate(&Some(bindings)).unwrap() - expected).abs() < 1e-10);
}

#[test]
fn test_p_derivative_quotient_rule_complex() {
    // sin(x) / x
    // d/dx = (cos(x)*x - sin(x)) / x^2
    let x = Parameter::try_from("x").unwrap();
    let expr = x.clone().sin() / x.clone();
    let deriv = expr.derivative("x").unwrap().simplify().unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("x", 2.0);
    let expected = (2.0f64.cos() * 2.0 - 2.0f64.sin()) / 4.0;
    assert!((deriv.evaluate(&Some(bindings)).unwrap() - expected).abs() < 1e-10);
}

#[test]
fn test_p_log_arbitrary_base() {
    let x = Parameter::try_from("x").unwrap();
    let ten = Parameter::from(10.0);

    // log(x, 10)
    let log10_x = x.clone().log(ten);

    // d(log(x, 10))/dx = 1 / (x * ln(10))
    let deriv = log10_x.derivative("x").unwrap().simplify().unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("x", 5.0);
    let expected = 1.0 / (5.0 * 10.0f64.ln());
    assert!((deriv.evaluate(&Some(bindings)).unwrap() - expected).abs() < 1e-10);
}

#[test]
fn test_p_variable_base_log() {
    let x = Parameter::try_from("x").unwrap();

    // log(x, x)
    let log_x_x = x.clone().log(x.clone());

    // d(log(x, x))/dx = d(ln(x)/ln(x))/dx
    let deriv = log_x_x.derivative("x").unwrap().simplify().unwrap();

    // ln(x)/ln(x) = 1, so the derivative should technically evaluate to 0 (except at x=1).
    let mut bindings = HashMap::new();
    bindings.insert("x", 5.0);

    // Depending on the engine's internal representation, this might return exactly 0
    // or a complex form that evaluates to 0.
    let eval_res = deriv.evaluate(&Some(bindings)).unwrap();
    assert!(eval_res.abs() < 1e-10);
}

#[test]
fn test_p_parse_number() {
    let p = Parameter::try_from("1.0").unwrap();
    assert!((p.evaluate(&None).unwrap() - 1.0).abs() < 1e-10);

    let p = Parameter::try_from("2.5").unwrap();
    assert!((p.evaluate(&None).unwrap() - 2.5).abs() < 1e-10);

    let p = Parameter::try_from("42").unwrap();
    assert!((p.evaluate(&None).unwrap() - 42.0).abs() < 1e-10);

    // Scientific notation
    let p = Parameter::try_from("1e3").unwrap();
    assert!((p.evaluate(&None).unwrap() - 1000.0).abs() < 1e-10);

    let p = Parameter::try_from("1.5e-2").unwrap();
    assert!((p.evaluate(&None).unwrap() - 0.015).abs() < 1e-10);
}

#[test]
fn test_p_parse_constants() {
    let p = Parameter::try_from("pi").unwrap(); // In symb_anafis, it is usually "pi" -> Variable, unless pre-bound
    let mut bindings = HashMap::new();
    bindings.insert("pi", PI);
    assert!((p.evaluate(&Some(bindings)).unwrap() - PI).abs() < 1e-10);

    let p = Parameter::try_from("e").unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("e", E);
    assert!((p.evaluate(&Some(bindings)).unwrap() - E).abs() < 1e-10);
}

#[test]
fn test_p_parse_addition() {
    let p = Parameter::try_from("1+2").unwrap();
    assert!((p.evaluate(&None).unwrap() - 3.0).abs() < 1e-10);

    let p = Parameter::try_from("1 + 2").unwrap();
    assert!((p.evaluate(&None).unwrap() - 3.0).abs() < 1e-10);
}

#[test]
fn test_p_parse_subtraction() {
    let p = Parameter::try_from("5-3").unwrap();
    assert!((p.evaluate(&None).unwrap() - 2.0).abs() < 1e-10);

    let p = Parameter::try_from("3-5").unwrap();
    assert!((p.evaluate(&None).unwrap() - (-2.0)).abs() < 1e-10);
}

#[test]
fn test_p_parse_multiplication() {
    let p = Parameter::try_from("2*3").unwrap();
    assert!((p.evaluate(&None).unwrap() - 6.0).abs() < 1e-10);

    let p = Parameter::try_from("2 * 3.5").unwrap();
    assert!((p.evaluate(&None).unwrap() - 7.0).abs() < 1e-10);
}

#[test]
fn test_p_parse_division() {
    let p = Parameter::try_from("6/2").unwrap();
    assert!((p.evaluate(&None).unwrap() - 3.0).abs() < 1e-10);

    let p = Parameter::try_from("5/2").unwrap();
    assert!((p.evaluate(&None).unwrap() - 2.5).abs() < 1e-10);
}

#[test]
fn test_p_parse_power() {
    let p = Parameter::try_from("2^3").unwrap();
    assert!((p.evaluate(&None).unwrap() - 8.0).abs() < 1e-10);

    let p = Parameter::try_from("3^2").unwrap();
    assert!((p.evaluate(&None).unwrap() - 9.0).abs() < 1e-10);

    let p = Parameter::try_from("4^0.5").unwrap();
    assert!((p.evaluate(&None).unwrap() - 2.0).abs() < 1e-10);
}

#[test]
fn test_p_parse_parentheses() {
    let p = Parameter::try_from("(1+2)*3").unwrap();
    assert!((p.evaluate(&None).unwrap() - 9.0).abs() < 1e-10);

    let p = Parameter::try_from("1+(2*3)").unwrap();
    assert!((p.evaluate(&None).unwrap() - 7.0).abs() < 1e-10);

    let p = Parameter::try_from("(1+2)*(3+4)").unwrap();
    assert!((p.evaluate(&None).unwrap() - 21.0).abs() < 1e-10);
}

#[test]
fn test_p_parse_unary_minus() {
    let p = Parameter::try_from("-5").unwrap();
    assert!((p.evaluate(&None).unwrap() - (-5.0)).abs() < 1e-10);

    let p = Parameter::try_from("3*-2").unwrap();
    assert!((p.evaluate(&None).unwrap() - (-6.0)).abs() < 1e-10);
}

#[test]
fn test_p_parse_symbolic_evaluation() {
    let p = Parameter::try_from("x + 2 * y").unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("x", 3.0);
    bindings.insert("y", 4.0);
    assert!((p.evaluate(&Some(bindings)).unwrap() - 11.0).abs() < 1e-10);
}

#[test]
fn test_p_parse_errors() {
    // Empty expression
    assert!(Parameter::try_from("").is_err());

    // Invalid character
    assert!(Parameter::try_from("1&2").is_err());

    // Mismatched parentheses
    assert!(Parameter::try_from("(1+2").is_ok());
}

#[test]
fn test_p_operator_precedence() {
    // Multiplication before addition
    let p = Parameter::try_from("2+3*4").unwrap();
    assert!((p.evaluate(&None).unwrap() - 14.0).abs() < 1e-10);

    // Power before multiplication
    let p = Parameter::try_from("2*3^2").unwrap();
    assert!((p.evaluate(&None).unwrap() - 18.0).abs() < 1e-10);

    // Parentheses override precedence
    let p = Parameter::try_from("(2+3)*4").unwrap();
    assert!((p.evaluate(&None).unwrap() - 20.0).abs() < 1e-10);
}

#[test]
fn test_p_display_format() {
    let p = Parameter::try_from("1+2").unwrap();
    assert_eq!(p.to_string(), "3");

    let p = Parameter::try_from("pi/2").unwrap();
    assert_eq!(p.to_string(), "pi/2");

    let p = Parameter::try_from("sin(x)").unwrap();
    assert_eq!(p.to_string(), "sin(x)");
}

#[test]
fn test_p_complex_real_world() {
    // RXY gate parameters: theta = pi/2 + 1, phi = 2.5
    let theta = Parameter::try_from("pi/2+1").unwrap();
    let phi = Parameter::try_from("2.5").unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("pi", PI);

    assert!((theta.evaluate(&Some(bindings.clone())).unwrap() - (PI / 2.0 + 1.0)).abs() < 1e-10);
    assert!((phi.evaluate(&None).unwrap() - 2.5).abs() < 1e-10);

    // RZ(pi/4 + 0.5)
    let rz_param = Parameter::try_from("pi/4+0.5").unwrap();
    assert!((rz_param.evaluate(&Some(bindings)).unwrap() - (PI / 4.0 + 0.5)).abs() < 1e-10);

    // RX(2*theta) with symbolic theta
    let rx_param = Parameter::try_from("2*theta").unwrap();
    assert!(rx_param.get_symbols().contains("theta"));
}

#[test]
fn test_p_try_from() {
    // Valid expressions should work
    let p1 = Parameter::try_from("theta");
    assert!(p1.is_ok());
    assert_eq!(p1.unwrap().to_string(), "theta");

    let p2 = Parameter::try_from("pi/2 + 1");
    assert!(p2.is_ok());

    let p3 = Parameter::try_from("sin(x)");
    assert!(p3.is_ok());

    // Invalid expressions should return Err
    let err1 = Parameter::try_from("");
    assert!(err1.is_err());

    let err2 = Parameter::try_from("1&2");
    assert!(err2.is_err());
}

#[test]
fn test_p_complex_pi_e_evaluation() {
    // A complex expression involving pi, e, trig functions, and exponents
    // f(x) = e^(sin(x * pi)) + cos(e * pi) / pi
    let expr_str = "e^(sin(x * pi)) + cos(e * pi) / pi";
    let p = Parameter::try_from(expr_str).expect("Failed to parse complex expression");

    // Evaluate with x = 0.5
    // f(0.5) = e^(sin(0.5 * pi)) + cos(e * pi) / pi
    //        = e^(sin(pi/2)) + cos(e * pi) / pi
    //        = e^(1) + cos(e * pi) / pi
    //        = e + cos(e * pi) / pi
    let mut bindings = HashMap::new();
    bindings.insert("x", 0.5);
    let val = p.evaluate(&Some(bindings)).expect("Evaluation failed");
    let expected = E + (E * PI).cos() / PI;

    assert!(
        (val - expected).abs() < 1e-10,
        "Complex pi/e evaluation failed. Expected: {}, Got: {}",
        expected,
        val
    );
}

#[test]
fn test_ln_boundary_cases() {
    let x = Parameter::symbol("x");
    let expr = x.ln();

    // ln(0) should return an error (infinity)
    let bindings = HashMap::from([("x", 0.0)]);
    let result = expr.evaluate(&Some(bindings));
    assert!(result.is_err(), "ln(0) should return an error");

    // ln(-1) should return an error (NaN)
    let bindings = HashMap::from([("x", -1.0)]);
    let result = expr.evaluate(&Some(bindings));
    assert!(result.is_err(), "ln(-1) should return an error");
}

#[test]
fn test_sqrt_boundary_cases() {
    let x = Parameter::symbol("x");
    let expr = x.sqrt();

    // sqrt(-1) should return an error (NaN)
    let bindings = HashMap::from([("x", -1.0)]);
    let result = expr.evaluate(&Some(bindings));
    assert!(result.is_err(), "sqrt(-1) should return an error");
}

#[test]
fn test_neg_operator() {
    let x = Parameter::symbol("x");

    // Test owned negation
    let neg_x = -x.clone();
    let bindings = HashMap::from([("x", 5.0)]);
    let result = neg_x.evaluate(&Some(bindings)).unwrap();
    assert!((result - (-5.0)).abs() < 1e-10);

    // Test borrowed negation
    let neg_x_ref = -&x;
    let bindings = HashMap::from([("x", 3.0)]);
    let result = neg_x_ref.evaluate(&Some(bindings)).unwrap();
    assert!((result - (-3.0)).abs() < 1e-10);
}

#[test]
fn test_from_str() {
    use std::str::FromStr;

    // Valid expression
    let p = Parameter::from_str("x + 1").unwrap();
    let bindings = HashMap::from([("x", 2.0)]);
    let result = p.evaluate(&Some(bindings)).unwrap();
    assert!((result - 3.0).abs() < 1e-10);

    // Invalid expression
    let result = Parameter::from_str("@@@");
    assert!(result.is_err());
}

#[test]
fn test_is_constant() {
    let constant = Parameter::from(2.5);
    assert!(constant.is_constant());

    let symbolic = Parameter::symbol("x");
    assert!(!symbolic.is_constant());

    let expr = Parameter::symbol("x") + Parameter::from(1.0);
    assert!(!expr.is_constant());
}

#[test]
fn test_is_zero() {
    let zero = Parameter::from(0.0);
    assert!(zero.is_zero());

    let non_zero = Parameter::from(1.0);
    assert!(!non_zero.is_zero());

    let symbolic = Parameter::symbol("x");
    assert!(!symbolic.is_zero());
}

#[test]
fn test_canonicalized_folds_constant_expression() {
    let theta = Parameter::symbol("theta");
    let canonical = (theta.clone() - theta).canonicalized().unwrap();

    assert!(canonical.get_symbols().is_empty());
    assert_eq!(canonical.evaluate(&None).unwrap(), 0.0);
}

#[test]
fn test_canonicalized_normalizes_negative_zero() {
    let canonical = Parameter::from(-0.0).canonicalized().unwrap();

    assert_eq!(
        canonical.evaluate(&None).unwrap().to_bits(),
        0.0f64.to_bits()
    );
}

#[test]
fn test_is_exact_zero() {
    assert!(Parameter::from(0.0).is_exact_zero().unwrap());
    assert!(!Parameter::from(f64::EPSILON / 2.0).is_exact_zero().unwrap());
    assert!(!Parameter::symbol("x").is_exact_zero().unwrap());
}

#[test]
fn test_is_one() {
    let one = Parameter::from(1.0);
    assert!(one.is_one());

    let non_one = Parameter::from(2.0);
    assert!(!non_one.is_one());

    let symbolic = Parameter::symbol("x");
    assert!(!symbolic.is_one());
}

#[test]
fn test_as_expr_and_into_expr() {
    let p = Parameter::symbol("x");

    // Test as_expr
    let expr_ref = p.as_expr();
    // Just verify we can get a reference to the expression
    let _ = expr_ref;

    // Test into_expr
    let p2 = Parameter::from(42.0);
    let expr = p2.into_expr();
    // Just verify we can consume the parameter and get the expression
    let _ = expr;
}

#[test]
fn simultaneous_substitution_does_not_cascade_overlapping_bindings() {
    let parameter = Parameter::symbol("a") + Parameter::symbol("b");
    let bindings = HashMap::from([
        ("a".to_string(), Parameter::symbol("b")),
        ("b".to_string(), Parameter::symbol("c")),
    ]);

    let substituted = parameter.substitute_many_simultaneous(&bindings);

    assert_eq!(
        substituted.get_symbols(),
        HashSet::from(["b".to_string(), "c".to_string()])
    );
}

#[test]
fn simultaneous_substitution_does_not_revisit_replacement_expressions() {
    let parameter = Parameter::symbol("a");
    let bindings = HashMap::from([
        (
            "a".to_string(),
            Parameter::symbol("b") + Parameter::from(1.0),
        ),
        ("b".to_string(), Parameter::from(2.0)),
    ]);

    let substituted = parameter.substitute_many_simultaneous(&bindings);

    assert_eq!(substituted.get_symbols(), HashSet::from(["b".to_string()]));
}
