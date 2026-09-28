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

//! Error codes and shared utilities for C bindings.

#![allow(dead_code)]

use std::ffi::CString;
use std::os::raw::c_char;

/// Error codes returned by C binding functions.
#[repr(i32)]
pub enum CqlibError {
    Ok = 0,
    NullPtr = -1,
    QubitOutOfBounds = -2,
    CircuitError = -3,
    ParseError = -4,
    IoError = -5,
    CompilerError = -6,
    SimulationError = -7,
    InvalidParam = -8,
}

/// Free a string returned by the C bindings (e.g. `qasm2_dumps`).
#[unsafe(no_mangle)]
pub extern "C" fn cqlib_string_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(ptr));
    }
}
