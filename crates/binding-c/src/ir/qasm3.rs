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

//! C ABI for OpenQASM 3.0 load and dump.

use crate::circuit::CCircuit;
use crate::error::CqlibError;
use cqlib_core::ir;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Parse an OpenQASM 3.0 string into a circuit.
/// Returns a heap-allocated `CCircuit*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn qasm3_loads(source: *const c_char) -> *mut CCircuit {
    if source.is_null() {
        return std::ptr::null_mut();
    }
    let source = unsafe { CStr::from_ptr(source) };
    let source = match source.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    match ir::qasm3_loads(source) {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Load an OpenQASM 3.0 file into a circuit.
/// Returns a heap-allocated `CCircuit*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn qasm3_load(path: *const c_char) -> *mut CCircuit {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let path = unsafe { CStr::from_ptr(path) };
    let path = match path.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    match ir::qasm3_load(path) {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Dump a circuit to an OpenQASM 3.0 string.
/// Returns a heap-allocated C string (caller must free with `cqlib_string_free`),
/// or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn qasm3_dumps(circuit: *const CCircuit) -> *mut c_char {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    match ir::qasm3_dumps(circuit) {
        Ok(s) => match CString::new(s) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Dump a circuit to an OpenQASM 3.0 file.
/// Returns 0 on success, or a negative error code on failure.
#[unsafe(no_mangle)]
pub extern "C" fn qasm3_dump(circuit: *const CCircuit, path: *const c_char) -> i32 {
    if circuit.is_null() || path.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let circuit = unsafe { &(*circuit).inner };
    let path = unsafe { CStr::from_ptr(path) };
    let path = match path.to_str() {
        Ok(s) => s,
        Err(_) => return CqlibError::InvalidParam as i32,
    };
    match ir::qasm3_dump(circuit, path) {
        Ok(()) => CqlibError::Ok as i32,
        Err(_) => CqlibError::IoError as i32,
    }
}
