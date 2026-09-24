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
use crate::qis::{CPauliString, qis_err_code};
use cqlib_core::qis::{Pauli, PauliString};
use std::collections::HashMap;
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

/// Multiply two single-qubit Pauli operators as `left * right` and return
/// the result together with the Pauli-group phase factor (e.g. X * Z = -iY).
///
/// On success, writes the resulting Pauli tag (one of `PAULI_I/X/Y/Z`) to
/// `out_pauli` and the complex phase (one of 1, i, -1, -i) to `out_phase`,
/// then returns 0. Returns -1 when an output pointer is NULL and -8 when an
/// input tag is not one of `PAULI_I/X/Y/Z`.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_mul_with_phase(
    left: u8,
    right: u8,
    out_pauli: *mut u8,
    out_phase: *mut num_complex::Complex64,
) -> i32 {
    if out_pauli.is_null() || out_phase.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let left = match from_c_pauli(left) {
        Some(p) => p,
        None => return CqlibError::InvalidParam as i32,
    };
    let right = match from_c_pauli(right) {
        Some(p) => p,
        None => return CqlibError::InvalidParam as i32,
    };
    let (result, phase) = left.mul_with_phase(right);
    unsafe {
        *out_pauli = to_c_pauli(result);
        *out_phase = phase.to_complex();
    }
    0
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

/// Return 1 if this Pauli string commutes with `other`, 0 otherwise.
/// Returns -1 for NULL handles and -8 when the qubit counts differ.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_commutes_with(
    ptr: *const CPauliString,
    other: *const CPauliString,
) -> i32 {
    if ptr.is_null() || other.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let a = unsafe { &(*ptr).inner };
    let b = unsafe { &(*other).inner };
    if a.num_qubits != b.num_qubits {
        return CqlibError::InvalidParam as i32;
    }
    if a.commutes_with(b) { 1 } else { 0 }
}

/// Return the X-component bitmask: bit i is set when qubit i carries an
/// X or Y operator. Returns 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_x_mask(ptr: *const CPauliString) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.x_mask() }
}

/// Return the Z-component bitmask: bit i is set when qubit i carries a
/// Z or Y operator. Returns 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_z_mask(ptr: *const CPauliString) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.z_mask() }
}

/// Compute the phase factor `i^n` contributed by the n Y operators of
/// the string. Writes the complex value to `out` and returns 0 on
/// success.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_y_phase(
    ptr: *const CPauliString,
    out: *mut num_complex::Complex64,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    unsafe { *out = wrapper.inner.y_phase() };
    0
}

/// Return the number of non-identity qubits (support size), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_support_len(ptr: *const CPauliString) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.support().len() }
}

/// Copy the support (indices of non-identity operators, ascending) into
/// `buffer` as `uint32_t` values. `len` must equal
/// `pauli_string_support_len(ptr)`. Returns 0 on success, or a negative
/// error code.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_support(
    ptr: *const CPauliString,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let support = wrapper.inner.support();
    if support.len() != len {
        return CqlibError::InvalidParam as i32;
    }
    for (i, &q) in support.iter().enumerate() {
        unsafe {
            *buffer.add(i) = q as u32;
        }
    }
    0
}

/// Get the Pauli operator at qubit index `idx` without panicking.
/// Writes one of `PAULI_I/X/Y/Z` to `out` and returns 0 on success, or
/// a negative error code (`-2` out of bounds).
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_try_get_pauli(
    ptr: *const CPauliString,
    idx: usize,
    out: *mut u8,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    match wrapper.inner.try_get_pauli(idx) {
        Ok(pauli) => {
            unsafe { *out = to_c_pauli(pauli) };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Set the Pauli operator at qubit index `idx` without panicking.
/// `pauli` must be one of `PAULI_I/X/Y/Z`. Returns 0 on success, or a
/// negative error code (`-2` out of bounds, `-8` invalid tag).
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_try_set_pauli(ptr: *mut CPauliString, idx: usize, pauli: u8) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let p = match from_c_pauli(pauli) {
        Some(p) => p,
        None => return CqlibError::InvalidParam as i32,
    };
    match wrapper.inner.try_set_pauli(idx, p) {
        Ok(()) => 0,
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the expectation value `<P>` given a probability distribution
/// over computational basis states. `bitstrings[i]` pairs with `probs[i]`;
/// each bitstring uses the little-endian convention (the rightmost
/// character is qubit 0) and must be exactly `num_qubits` characters of
/// '0'/'1'. Strings containing X or Y operators yield 0.0. Writes the
/// value to `out` and returns 0 on success, or a negative error code
/// (`-1` NULL handle, buffer or output; `-4` non-UTF-8 bitstring; `-8`
/// NULL bitstring entry; `-7` for core rejections such as a wrong-length
/// or invalid-character bitstring).
#[unsafe(no_mangle)]
pub extern "C" fn pauli_string_expectation(
    ptr: *const CPauliString,
    bitstrings: *const *const c_char,
    probs: *const f64,
    len: usize,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if len > 0 && (bitstrings.is_null() || probs.is_null()) {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let mut distribution: HashMap<String, f64> = HashMap::with_capacity(len);
    for i in 0..len {
        let raw = unsafe { *bitstrings.add(i) };
        if raw.is_null() {
            return CqlibError::InvalidParam as i32;
        }
        let state = match unsafe { CStr::from_ptr(raw) }.to_str() {
            Ok(state) => state.to_string(),
            Err(_) => return CqlibError::ParseError as i32,
        };
        distribution.insert(state, unsafe { *probs.add(i) });
    }
    match wrapper.inner.expectation(&distribution) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}
