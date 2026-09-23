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
use std::ffi::CStr;
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
