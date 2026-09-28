// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating that
// they have been altered from the originals.

//! C ABI for the `Qubit` logical-qubit identifier.
//!
//! `Qubit` is a lightweight, copyable handle wrapping a numeric ID. It does
//! not carry a circuit identity or a state-vector position; the owning
//! circuit and the conversion interfaces determine those meanings.

use crate::error::CqlibError;
use cqlib_core::circuit::Qubit;
use std::ffi::CString;
use std::os::raw::c_char;

/// Logical qubit identifier: a value handle wrapping a numeric ID, passed by
/// value. Compare and hash via the numeric ID.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CQubit {
    /// Numeric identifier of the logical qubit.
    pub id: u32,
}

impl From<Qubit> for CQubit {
    fn from(qubit: Qubit) -> Self {
        Self { id: qubit.id() }
    }
}

impl From<CQubit> for Qubit {
    fn from(qubit: CQubit) -> Self {
        Qubit::new(qubit.id)
    }
}

/// Creates a logical qubit with the supplied numeric identifier.
#[unsafe(no_mangle)]
pub extern "C" fn qubit_new(id: u32) -> CQubit {
    CQubit { id }
}

/// Returns the raw numeric identifier of the logical qubit.
#[unsafe(no_mangle)]
pub extern "C" fn qubit_id(qubit: CQubit) -> u32 {
    Qubit::from(qubit).id()
}

/// Returns the numeric identifier as a `uintptr_t` for index-style use. The
/// value does not map the qubit to its position in any circuit or simulator.
#[unsafe(no_mangle)]
pub extern "C" fn qubit_index(qubit: CQubit) -> usize {
    Qubit::from(qubit).index()
}

/// Checked conversion from a signed integer: writes the qubit to `*out` and
/// returns 0, -1 when `out` is NULL, or -8 when `value` is negative or
/// exceeds the `u32` ID range.
#[unsafe(no_mangle)]
pub extern "C" fn qubit_try_from_i64(value: i64, out: *mut CQubit) -> i32 {
    if out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if value < 0 || value > u32::MAX as i64 {
        return CqlibError::InvalidParam as i32;
    }
    unsafe { *out = CQubit { id: value as u32 } };
    CqlibError::Ok as i32
}

/// Checked conversion from an unsigned integer: writes the qubit to `*out`
/// and returns 0, -1 when `out` is NULL, or -8 when `value` exceeds the
/// `u32` ID range.
#[unsafe(no_mangle)]
pub extern "C" fn qubit_try_from_u64(value: u64, out: *mut CQubit) -> i32 {
    if out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if value > u32::MAX as u64 {
        return CqlibError::InvalidParam as i32;
    }
    unsafe { *out = CQubit { id: value as u32 } };
    CqlibError::Ok as i32
}

/// Returns 1 when both qubits carry the same numeric identifier, else 0.
#[unsafe(no_mangle)]
pub extern "C" fn qubit_equal(a: CQubit, b: CQubit) -> i32 {
    Qubit::from(a).eq(&Qubit::from(b)) as i32
}

/// Compares two qubits by numeric identifier: -1 when `a < b`, 0 when equal,
/// 1 when `a > b`.
#[unsafe(no_mangle)]
pub extern "C" fn qubit_compare(a: CQubit, b: CQubit) -> i32 {
    match Qubit::from(a).cmp(&Qubit::from(b)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Formats the qubit identifier as `Q<id>` (e.g. `Q12`); free with
/// `cqlib_string_free`. Returns NULL on allocation failure.
#[unsafe(no_mangle)]
pub extern "C" fn qubit_to_string(qubit: CQubit) -> *mut c_char {
    CString::new(format!("{}", Qubit::from(qubit))).map_or(std::ptr::null_mut(), |cs| cs.into_raw())
}
