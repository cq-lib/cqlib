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

use crate::circuit::{CCircuit, CParameter, binding_refs, parse_bindings};
use cqlib_core::circuit::Parameter;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Parse a symbolic parameter expression.
#[unsafe(no_mangle)]
pub extern "C" fn param_parse(expr: *const c_char) -> *mut CParameter {
    if expr.is_null() {
        return std::ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(expr) };
    let expr = match c_str.to_str() {
        Ok(expr) => expr,
        Err(_) => return std::ptr::null_mut(),
    };

    match Parameter::try_from(expr) {
        Ok(param) => Box::into_raw(Box::new(CParameter { inner: param })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a parameter. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn param_free(ptr: *mut CParameter) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Evaluate a parameter expression with bindings formatted as "name:value,name2:value2".
#[unsafe(no_mangle)]
pub extern "C" fn param_evaluate(ptr: *const CParameter, bindings: *const c_char) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    let bindings = parse_bindings(bindings);
    let refs = bindings.as_ref().map(binding_refs);
    unsafe { (*ptr).inner.evaluate(&refs).unwrap_or(0.0) }
}

/// Return a new circuit with matching symbolic parameters assigned.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_assign_params(
    circuit: *const CCircuit,
    bindings: *const c_char,
) -> *mut CCircuit {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }

    let bindings = parse_bindings(bindings);
    let refs = bindings.as_ref().map(binding_refs);
    match unsafe { (*circuit).inner.assign_parameters(&refs) } {
        Ok(inner) => Box::into_raw(Box::new(CCircuit { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  Construction helpers  =====

fn box_param(param: Parameter) -> *mut CParameter {
    Box::into_raw(Box::new(CParameter { inner: param }))
}

fn unary_param(
    a: *const CParameter,
    build: impl FnOnce(&Parameter) -> Parameter,
) -> *mut CParameter {
    if a.is_null() {
        return std::ptr::null_mut();
    }
    box_param(build(unsafe { &(*a).inner }))
}

fn binary_param(
    a: *const CParameter,
    b: *const CParameter,
    combine: impl FnOnce(Parameter, Parameter) -> Parameter,
) -> *mut CParameter {
    if a.is_null() || b.is_null() {
        return std::ptr::null_mut();
    }
    let lhs = unsafe { (*a).inner.clone() };
    let rhs = unsafe { (*b).inner.clone() };
    box_param(combine(lhs, rhs))
}

// =====  Construction  =====

/// Create a symbolic parameter for a free variable. Returns NULL on NULL or
/// non-UTF-8 input.
#[unsafe(no_mangle)]
pub extern "C" fn param_symbol(name: *const c_char) -> *mut CParameter {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(name) => box_param(Parameter::symbol(name)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create the mathematical constant pi as a symbolic parameter. The symbol
/// is resolved automatically during evaluation.
#[unsafe(no_mangle)]
pub extern "C" fn param_pi() -> *mut CParameter {
    box_param(Parameter::pi())
}

/// Create Euler's number e as a symbolic parameter. The symbol is resolved
/// automatically during evaluation.
#[unsafe(no_mangle)]
pub extern "C" fn param_e() -> *mut CParameter {
    box_param(Parameter::e())
}

/// Create a numeric parameter from a finite double value. Returns NULL when
/// the value is NaN or infinite.
#[unsafe(no_mangle)]
pub extern "C" fn param_from_double(value: f64) -> *mut CParameter {
    if !value.is_finite() {
        return std::ptr::null_mut();
    }
    box_param(Parameter::from(value))
}

// =====  Arithmetic  =====

/// Return a new parameter computing `a + b`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_add(a: *const CParameter, b: *const CParameter) -> *mut CParameter {
    binary_param(a, b, |lhs, rhs| lhs + rhs)
}

/// Return a new parameter computing `a - b`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_sub(a: *const CParameter, b: *const CParameter) -> *mut CParameter {
    binary_param(a, b, |lhs, rhs| lhs - rhs)
}

/// Return a new parameter computing `a * b`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_mul(a: *const CParameter, b: *const CParameter) -> *mut CParameter {
    binary_param(a, b, |lhs, rhs| lhs * rhs)
}

/// Return a new parameter computing `a / b`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_div(a: *const CParameter, b: *const CParameter) -> *mut CParameter {
    binary_param(a, b, |lhs, rhs| lhs / rhs)
}

/// Return a new parameter computing `-a`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_neg(a: *const CParameter) -> *mut CParameter {
    unary_param(a, |p| -p)
}

// =====  Mathematical functions  =====

/// Return a new parameter computing `|a|`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_abs(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::abs)
}

/// Return a new parameter computing `sqrt(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_sqrt(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::sqrt)
}

/// Return a new parameter computing `exp(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_exp(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::exp)
}

/// Return a new parameter computing `ln(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_ln(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::ln)
}

/// Return a new parameter computing `log(a) / log(base)`, or NULL on NULL
/// input.
#[unsafe(no_mangle)]
pub extern "C" fn param_log(a: *const CParameter, base: *const CParameter) -> *mut CParameter {
    if base.is_null() {
        return std::ptr::null_mut();
    }
    let base = unsafe { (*base).inner.clone() };
    unary_param(a, move |p| p.log(base))
}

/// Return a new parameter computing `sin(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_sin(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::sin)
}

/// Return a new parameter computing `cos(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_cos(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::cos)
}

/// Return a new parameter computing `tan(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_tan(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::tan)
}

/// Return a new parameter computing `asin(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_asin(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::asin)
}

/// Return a new parameter computing `acos(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_acos(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::acos)
}

/// Return a new parameter computing `atan(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_atan(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::atan)
}

/// Return a new parameter computing `sinh(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_sinh(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::sinh)
}

/// Return a new parameter computing `cosh(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_cosh(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::cosh)
}

/// Return a new parameter computing `tanh(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_tanh(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::tanh)
}

/// Return a new parameter computing `floor(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_floor(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::floor)
}

/// Return a new parameter computing `ceil(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_ceil(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::ceil)
}

/// Return a new parameter computing `round(a)`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_round(a: *const CParameter) -> *mut CParameter {
    unary_param(a, Parameter::round)
}

/// Return a new parameter computing `a` raised to the power `exp`, or NULL
/// on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_pow(a: *const CParameter, exp: *const CParameter) -> *mut CParameter {
    if exp.is_null() {
        return std::ptr::null_mut();
    }
    let exp = unsafe { (*exp).inner.clone() };
    unary_param(a, move |p| p.pow(exp))
}

// =====  Symbol and state queries  =====

/// Return the number of free symbols (constants pi and e excluded), or 0
/// for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn param_symbols_len(a: *const CParameter) -> usize {
    if a.is_null() {
        return 0;
    }
    unsafe { (*a).inner.get_symbols().len() }
}

/// Copy the free symbol names, sorted lexicographically, into `out`. Each
/// written element is a freshly allocated C string the caller frees with
/// `cqlib_string_free`. Returns 0 on success, -1 on NULL input, -8 when
/// `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn param_symbols(a: *const CParameter, out: *mut *mut c_char, len: usize) -> i32 {
    if a.is_null() {
        return -1;
    }
    let mut symbols: Vec<String> = unsafe { (*a).inner.get_symbols().into_iter().collect() };
    symbols.sort();
    if len < symbols.len() {
        return -8;
    }
    if symbols.is_empty() {
        return 0;
    }
    if out.is_null() {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(out, symbols.len()) };
    for (slot, symbol) in slice.iter_mut().zip(symbols.iter()) {
        match CString::new(symbol.as_str()) {
            Ok(text) => *slot = text.into_raw(),
            Err(_) => return -3,
        }
    }
    0
}

/// Return the symbol name when the parameter is exactly one free symbol.
/// The result is a freshly allocated C string the caller frees with
/// `cqlib_string_free`. Returns NULL for NULL input or when the expression
/// is not a single symbol.
#[unsafe(no_mangle)]
pub extern "C" fn param_as_symbol(a: *const CParameter) -> *mut c_char {
    if a.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*a).inner.as_symbol() } {
        Some(symbol) => match CString::new(symbol) {
            Ok(text) => text.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Return 1 when the parameter has no free symbols, else 0. Returns -1 on
/// NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_is_constant(a: *const CParameter) -> i32 {
    if a.is_null() {
        return -1;
    }
    unsafe { (*a).inner.is_constant() as i32 }
}

/// Return 1 when a constant parameter evaluates exactly to zero, else 0.
/// Returns -1 on NULL input, -3 when a constant parameter cannot be
/// evaluated.
#[unsafe(no_mangle)]
pub extern "C" fn param_is_exact_zero(a: *const CParameter) -> i32 {
    if a.is_null() {
        return -1;
    }
    unsafe { (*a).inner.is_exact_zero().map_or(-3, |flag| flag as i32) }
}

/// Return 1 when the parameter evaluates to zero, else 0. Parameters with
/// unbound symbols return 0. Returns -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_is_zero(a: *const CParameter) -> i32 {
    if a.is_null() {
        return -1;
    }
    unsafe { (*a).inner.is_zero() as i32 }
}

/// Return 1 when the parameter evaluates to one, else 0. Parameters with
/// unbound symbols return 0. Returns -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_is_one(a: *const CParameter) -> i32 {
    if a.is_null() {
        return -1;
    }
    unsafe { (*a).inner.is_one() as i32 }
}

// =====  Simplification  =====

/// Return a new algebraically simplified parameter, or NULL on NULL input
/// or simplification failure.
#[unsafe(no_mangle)]
pub extern "C" fn param_simplify(a: *const CParameter) -> *mut CParameter {
    if a.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*a).inner.simplify() } {
        Ok(inner) => box_param(inner),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return the canonical storage form of the parameter: simplified, with
/// constant expressions evaluated to a finite numeric value. Returns NULL
/// on NULL input or failure.
#[unsafe(no_mangle)]
pub extern "C" fn param_canonicalized(a: *const CParameter) -> *mut CParameter {
    if a.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*a).inner.canonicalized() } {
        Ok(inner) => box_param(inner),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  Substitution and differentiation  =====

/// Substitute every occurrence of `symbol` with `replacement`, returning a
/// new parameter. Returns NULL on NULL input or non-UTF-8 symbol name.
#[unsafe(no_mangle)]
pub extern "C" fn param_replace(
    a: *const CParameter,
    symbol: *const c_char,
    replacement: *const CParameter,
) -> *mut CParameter {
    if a.is_null() || symbol.is_null() || replacement.is_null() {
        return std::ptr::null_mut();
    }
    let symbol = match unsafe { CStr::from_ptr(symbol) }.to_str() {
        Ok(symbol) => symbol,
        Err(_) => return std::ptr::null_mut(),
    };
    let replacement = unsafe { (*replacement).inner.clone() };
    box_param(unsafe { (*a).inner.replace(symbol, replacement) })
}

/// Substitute several symbols in one call and simplify the result.
/// `names` and `values` are parallel arrays of length `len`. Returns a new
/// parameter, or NULL on NULL input, non-UTF-8 names or NULL elements.
#[unsafe(no_mangle)]
pub extern "C" fn param_substitute_many(
    a: *const CParameter,
    names: *const *const c_char,
    values: *const *const CParameter,
    len: usize,
) -> *mut CParameter {
    if a.is_null() {
        return std::ptr::null_mut();
    }
    let mut bindings: HashMap<String, Parameter> = HashMap::with_capacity(len);
    for i in 0..len {
        let name = unsafe { *names.add(i) };
        let value = unsafe { *values.add(i) };
        if name.is_null() || value.is_null() {
            return std::ptr::null_mut();
        }
        let name = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(name) => name.to_string(),
            Err(_) => return std::ptr::null_mut(),
        };
        bindings.insert(name, unsafe { (*value).inner.clone() });
    }
    box_param(unsafe { (*a).inner.substitute_many(&bindings) })
}

/// Return the symbolic partial derivative with respect to `var`, or NULL
/// on NULL input, non-UTF-8 name or differentiation failure.
#[unsafe(no_mangle)]
pub extern "C" fn param_derivative(a: *const CParameter, var: *const c_char) -> *mut CParameter {
    if a.is_null() || var.is_null() {
        return std::ptr::null_mut();
    }
    let var = match unsafe { CStr::from_ptr(var) }.to_str() {
        Ok(var) => var,
        Err(_) => return std::ptr::null_mut(),
    };
    match unsafe { (*a).inner.derivative(var) } {
        Ok(inner) => box_param(inner),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  Conservative equivalence  =====

/// Conservatively check whether two parameters are provably equal within
/// `tolerance`. Returns 1 when equal, 0 otherwise; -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_provably_equal(
    a: *const CParameter,
    b: *const CParameter,
    tolerance: f64,
) -> i32 {
    if a.is_null() || b.is_null() {
        return -1;
    }
    unsafe { (*a).inner.provably_equal(&(*b).inner, tolerance) as i32 }
}

/// Conservatively check whether two parameters are equal modulo `modulus`
/// within `tolerance`. Returns 1 when equal, 0 otherwise; -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn param_provably_equal_modulo(
    a: *const CParameter,
    b: *const CParameter,
    modulus: *const CParameter,
    tolerance: f64,
) -> i32 {
    if a.is_null() || b.is_null() || modulus.is_null() {
        return -1;
    }
    unsafe {
        (*a).inner
            .provably_equal_modulo(&(*b).inner, &(*modulus).inner, tolerance) as i32
    }
}
