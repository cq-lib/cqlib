// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! C ABI for `Measurement` handles and their `ClassicalValue` credentials.
//!
//! A [`Measurement`] pairs the immutable circuit value produced by a
//! measurement with the measured qubits and their result bit order. The
//! qubit-order utilities (`project`, `project_basis`, `check_qubits`) are
//! self-contained and work on standalone handles, while the value side is
//! meant to flow back into the classical expression layer.

use crate::circuit::{
    CClassicalExpr, CQLIB_CLASSICAL_TYPE_BIT, CQLIB_CLASSICAL_TYPE_BIT_VEC,
    CQLIB_CLASSICAL_TYPE_BOOL, CQLIB_CLASSICAL_TYPE_UINT,
};
use crate::error::CqlibError;
use crate::qis::{CClassicalValue, CMeasurement, qis_err_code};
use cqlib_core::circuit::{CircuitId, ClassicalType, ClassicalValue, Measurement, Qubit};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Converts a C (tag, width) pair into a core `ClassicalType`.
fn classical_type_from_tag(tag: u32, width: u32) -> Option<ClassicalType> {
    match tag {
        CQLIB_CLASSICAL_TYPE_BIT => Some(ClassicalType::Bit),
        CQLIB_CLASSICAL_TYPE_BOOL => Some(ClassicalType::Bool),
        CQLIB_CLASSICAL_TYPE_UINT => ClassicalType::uint(width),
        CQLIB_CLASSICAL_TYPE_BIT_VEC => ClassicalType::bit_vec(width),
        _ => None,
    }
}

/// Converts a core `ClassicalType` into a C (tag, width) pair.
fn classical_type_parts(ty: ClassicalType) -> (u32, u32) {
    match ty {
        ClassicalType::Bit => (CQLIB_CLASSICAL_TYPE_BIT, 1),
        ClassicalType::Bool => (CQLIB_CLASSICAL_TYPE_BOOL, 1),
        ClassicalType::UInt(width) => (CQLIB_CLASSICAL_TYPE_UINT, width.get()),
        ClassicalType::BitVec(width) => (CQLIB_CLASSICAL_TYPE_BIT_VEC, width.get()),
    }
}

/// Create a standalone measurement descriptor from raw components.
///
/// The wrapped `ClassicalValue` is built with a fresh circuit identity, so it
/// is **not registered in any circuit**: expressions produced from it are
/// rejected by `circuit_validate_classical_expr`. Use the `circuit_measure_*`
/// builders for circuit-integrated measurements; this constructor targets the
/// self-contained qubit-order utilities (`measurement_project`,
/// `measurement_project_basis`, `measurement_check_qubits`).
///
/// `ty_tag` / `ty_width` describe the result type (one of
/// `CQLIB_CLASSICAL_TYPE_*`; `Bit` and `BitVec` are the direct
/// measurement-target types). `qubits` holds `len` qubit indices where
/// `qubits[0]` becomes result bit 0 (the least-significant bit). Returns NULL
/// on NULL `qubits`, an empty list, or an invalid type.
#[unsafe(no_mangle)]
pub extern "C" fn measurement_new(
    value_index: u32,
    ty_tag: u32,
    ty_width: u32,
    qubits: *const u32,
    len: usize,
) -> *mut CMeasurement {
    if qubits.is_null() || len == 0 {
        return std::ptr::null_mut();
    }
    let Some(ty) = classical_type_from_tag(ty_tag, ty_width) else {
        return std::ptr::null_mut();
    };
    let value = ClassicalValue::new(CircuitId::new(), value_index, ty);
    let qubits: Vec<Qubit> = unsafe { std::slice::from_raw_parts(qubits, len) }
        .iter()
        .map(|&id| Qubit::new(id))
        .collect();
    Box::into_raw(Box::new(CMeasurement {
        inner: Measurement::new(value, qubits.into()),
    }))
}

/// Free a measurement handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn measurement_free(ptr: *mut CMeasurement) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Return the immutable classical value produced by this measurement as a new
/// `CClassicalValue*` handle (free with `classical_value_free`).
/// Returns NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn measurement_value(ptr: *const CMeasurement) -> *mut CClassicalValue {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(CClassicalValue {
        inner: unsafe { (*ptr).inner.value() },
    }))
}

/// Create an expression that reads this measurement's immutable value.
/// Returns a new `CClassicalExpr*` handle (free with `classical_expr_free`),
/// or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn measurement_expr(ptr: *const CMeasurement) -> *mut CClassicalExpr {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(CClassicalExpr {
        inner: unsafe { (*ptr).inner.expr() },
    }))
}

/// Return the number of measured qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn measurement_qubits_len(ptr: *const CMeasurement) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.qubits().len() }
}

/// Copy the measured qubit indices (in result bit order) into `buffer`.
/// Returns 0 on success, -1 on NULL pointers, -8 when `len` does not equal
/// `measurement_qubits_len`.
#[unsafe(no_mangle)]
pub extern "C" fn measurement_qubits(
    ptr: *const CMeasurement,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let qubits = unsafe { (*ptr).inner.qubits() };
    if qubits.len() != len {
        return CqlibError::InvalidParam as i32;
    }
    for (i, qubit) in qubits.iter().enumerate() {
        unsafe { *buffer.add(i) = qubit.index() as u32 };
    }
    0
}

/// Return the number of measured bits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn measurement_width(ptr: *const CMeasurement) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.width() }
}

/// Write the measurement result type (tag + width) to the out-parameters.
/// Returns 0 on success, -1 on NULL pointers.
#[unsafe(no_mangle)]
pub extern "C" fn measurement_ty(ptr: *const CMeasurement, tag: *mut u32, width: *mut u32) -> i32 {
    if ptr.is_null() || tag.is_null() || width.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let (ty_tag, ty_width) = classical_type_parts(unsafe { (*ptr).inner.ty() });
    unsafe {
        *tag = ty_tag;
        *width = ty_width;
    }
    0
}

/// Check that all measured qubits are valid for a state of `num_qubits`
/// qubits. Returns 0 on success, -1 on NULL input, -2 when a qubit index is
/// out of bounds.
#[unsafe(no_mangle)]
pub extern "C" fn measurement_check_qubits(ptr: *const CMeasurement, num_qubits: usize) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    wrapper
        .inner
        .check_qubits(num_qubits)
        .map_or_else(|err| qis_err_code(&err), |_| 0)
}

/// Project a full-register outcome onto this measurement's qubit order.
///
/// `bitstring` is a big-endian '0'/'1' string over the full register (MSB =
/// last qubit, matching `statevector_measure_all` output). If `qubits[i]` is
/// one in the outcome, result bit `i` is set. Returns the projected outcome
/// of `measurement_width` bits as a heap-allocated big-endian bitstring
/// (caller frees with `cqlib_string_free`), or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn measurement_project(
    ptr: *const CMeasurement,
    bitstring: *const c_char,
) -> *mut c_char {
    if ptr.is_null() || bitstring.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    let source = unsafe { CStr::from_ptr(bitstring) };
    let Ok(source) = source.to_str() else {
        return std::ptr::null_mut();
    };
    let Ok(full) = cqlib_core::device::Outcome::from_bitstring(source) else {
        return std::ptr::null_mut();
    };
    let projected = wrapper.inner.project(&full);
    match CString::new(projected.to_bitstring(wrapper.inner.width())) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Project a computational-basis index onto this measurement's qubit order.
///
/// If bit `qubits[i]` is one in `basis`, result bit `i` is set. Returns the
/// projected outcome of `measurement_width` bits as a heap-allocated
/// big-endian bitstring (caller frees with `cqlib_string_free`), or NULL on
/// error.
#[unsafe(no_mangle)]
pub extern "C" fn measurement_project_basis(ptr: *const CMeasurement, basis: usize) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    let projected = wrapper.inner.project_basis(basis);
    match CString::new(projected.to_bitstring(wrapper.inner.width())) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a classical value handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn classical_value_free(ptr: *mut CClassicalValue) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Return the circuit-local value table index, or `UINT32_MAX` for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn classical_value_index(ptr: *const CClassicalValue) -> u32 {
    if ptr.is_null() {
        return u32::MAX;
    }
    unsafe { (*ptr).inner.index() }
}

/// Write the value type (tag + width) to the out-parameters.
/// Returns 0 on success, -1 on NULL pointers.
#[unsafe(no_mangle)]
pub extern "C" fn classical_value_ty(
    ptr: *const CClassicalValue,
    tag: *mut u32,
    width: *mut u32,
) -> i32 {
    if ptr.is_null() || tag.is_null() || width.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let (ty_tag, ty_width) = classical_type_parts(unsafe { (*ptr).inner.ty() });
    unsafe {
        *tag = ty_tag;
        *width = ty_width;
    }
    0
}

/// Create an expression that reads this immutable runtime value.
/// Returns a new `CClassicalExpr*` handle (free with `classical_expr_free`),
/// or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn classical_value_expr(ptr: *const CClassicalValue) -> *mut CClassicalExpr {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(CClassicalExpr {
        inner: unsafe { (*ptr).inner.expr() },
    }))
}
