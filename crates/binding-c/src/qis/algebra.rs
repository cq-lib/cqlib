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

//! Internal helpers for the checked algebra ABI. Legacy QIS error mapping is unchanged.

use cqlib_core::qis::QisError;

pub(crate) fn algebra_err_code(err: &QisError) -> i32 {
    match err {
        QisError::ModeOutOfBounds { .. } | QisError::IndexOutOfBounds { .. } => -2,
        QisError::ModeMismatch { .. }
        | QisError::QubitMismatch { .. }
        | QisError::DimensionMismatch { .. }
        | QisError::NonFiniteCoefficient { .. }
        | QisError::InvalidOccupation { .. }
        | QisError::InvalidParameterValue(_)
        | QisError::NotHermitian => -8,
        _ => super::qis_err_code(err),
    }
}

/// C callers must provide valid, aligned storage, live handles and exclusive
/// mutable outputs. NULL is allowed for empty arrays. Check the Rust slice size
/// bound before constructing a slice, including for malicious length values.
pub(crate) unsafe fn input_slice<'a, T>(ptr: *const T, len: usize) -> Result<&'a [T], i32> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(-1);
    }
    if len > (isize::MAX as usize) / std::mem::size_of::<T>() {
        return Err(-8);
    }
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

/// Only called after validating the output pointer. Write once on success.
pub(crate) unsafe fn write_result<T>(out: *mut T, result: Result<T, QisError>) -> i32 {
    match result {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => algebra_err_code(&err),
    }
}

/// Check the same BitVec materialization limit used by core's checked algebra.
pub(crate) fn check_pauli_width(width: usize) -> Result<(), i32> {
    if width > bitvec::slice::BitSlice::<usize, bitvec::order::Lsb0>::MAX_BITS {
        Err(-7)
    } else {
        Ok(())
    }
}

/// Return 2^width only when the side length is representable. Do not truncate
/// usize widths to u32: zero sums may legally declare widths as large as usize::MAX.
pub(crate) fn dense_matrix_dim(width: usize) -> Option<usize> {
    1usize.checked_shl(u32::try_from(width).ok()?)
}

/// Check element count and signed addressable byte size before allocation or
/// writing C buffers, matching Hamiltonian::to_matrix's allocation bounds.
pub(crate) fn dense_matrix_len(width: usize) -> Option<usize> {
    let dim = dense_matrix_dim(width)?;
    let len = dim.checked_mul(dim)?;
    let bytes = len.checked_mul(std::mem::size_of::<num_complex::Complex64>())?;
    (bytes <= isize::MAX as usize).then_some(len)
}
