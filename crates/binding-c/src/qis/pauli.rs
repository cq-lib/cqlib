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

//! C ABI for `PauliString` and single-qubit Pauli operators.

use crate::error::CqlibError;
use crate::qis::CPauliString;
use cqlib_core::qis::{Pauli, PauliString};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::str::FromStr;

/// C-side Pauli operator enum. Mirrors `Pauli` but with a stable `#[repr(u8)]`
/// layout suitable for direct use from C.
///
/// | Value | Operator |
/// |-------|----------|
/// | 0     | I        |
/// | 1     | X        |
/// | 2     | Y        |
/// | 3     | Z        |
pub const PAULI_I: u8 = 0;
pub const PAULI_X: u8 = 1;
pub const PAULI_Y: u8 = 2;
pub const PAULI_Z: u8 = 3;

fn from_c_pauli(value: u8) -> Option<Pauli> {
    match value {
        PAULI_I => Some(Pauli::I),
        PAULI_X => Some(Pauli::X),
        PAULI_Y => Some(Pauli::Y),
        PAULI_Z => Some(Pauli::Z),
        _ => None,
    }
}

fn to_c_pauli(pauli: Pauli) -> u8 {
    match pauli {
        Pauli::I => PAULI_I,
        Pauli::X => PAULI_X,
        Pauli::Y => PAULI_Y,
        Pauli::Z => PAULI_Z,
    }
}

/// Create a new identity Pauli string on `num_qubits` qubits.
/// Returns NULL for 0 qubits.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_new(num_qubits: usize) -> *mut CPauliString {
    if num_qubits == 0 {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(CPauliString {
        inner: PauliString::new(num_qubits),
    }))
}

/// Free a Pauli string. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_free(ptr: *mut CPauliString) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Parse a Pauli string from a C string (e.g. "XYZ", "ZZI").
/// Returns NULL on parse failure.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_parse(source: *const c_char) -> *mut CPauliString {
    if source.is_null() {
        return std::ptr::null_mut();
    }
    let source = unsafe { CStr::from_ptr(source) };
    let source = match source.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    match PauliString::from_str(source) {
        Ok(ps) => Box::into_raw(Box::new(CPauliString { inner: ps })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Set the Pauli operator at qubit index `idx`. `pauli` must be one of
/// `PAULI_I`, `PAULI_X`, `PAULI_Y`, `PAULI_Z`.
/// Returns 0 on success, or a negative error code on failure.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_set_pauli(ptr: *mut CPauliString, idx: usize, pauli: u8) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let p = match from_c_pauli(pauli) {
        Some(p) => p,
        None => return CqlibError::InvalidParam as i32,
    };
    if idx >= wrapper.inner.num_qubits {
        return CqlibError::QubitOutOfBounds as i32;
    }
    wrapper.inner.set_pauli(idx, p);
    0
}

/// Get the Pauli operator at qubit index `idx`, returned as one of
/// `PAULI_I/X/Y/Z`. Returns 255 (0xFF) on error.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_get_pauli(ptr: *const CPauliString, idx: usize) -> u8 {
    if ptr.is_null() {
        return 0xFF;
    }
    let wrapper = unsafe { &*ptr };
    if idx >= wrapper.inner.num_qubits {
        return 0xFF;
    }
    to_c_pauli(wrapper.inner.get_pauli(idx))
}

/// Return the number of qubits the Pauli string acts on, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_num_qubits(ptr: *const CPauliString) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits }
}

/// Format the Pauli string as a C string (e.g. "+XYZ").
/// Caller must free with `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_to_string(ptr: *const CPauliString) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    let s = wrapper.inner.to_string();
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Flatten the dense matrix representation into `buffer`.
///
/// The matrix has dimensions `2^N × 2^N` and is stored in row-major order
/// as `Complex64` values. Use `pauli_string_matrix_len` first to obtain
/// the side length (`2^N`). Returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_matrix_len(ptr: *const CPauliString) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let wrapper = unsafe { &*ptr };
    1usize << wrapper.inner.num_qubits
}

/// Copy the dense matrix into `buffer` (row-major, `Complex64` values).
/// `len` must equal `pauli_string_matrix_len(ptr) * pauli_string_matrix_len(ptr)`.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_matrix(
    ptr: *const CPauliString,
    buffer: *mut num_complex::Complex64,
    len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let matrix = wrapper.inner.to_matrix();
    let dim = 1usize << wrapper.inner.num_qubits;
    if matrix.len() != len || len != dim * dim {
        return CqlibError::InvalidParam as i32;
    }
    let flat: Vec<num_complex::Complex64> = matrix.iter().cloned().collect();
    unsafe {
        std::ptr::copy_nonoverlapping(flat.as_ptr(), buffer, len);
    }
    0
}
