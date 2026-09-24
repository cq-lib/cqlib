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

//! C ABI for entropy and entanglement measures (`qis/entropy.rs`).

use crate::error::CqlibError;
use crate::qis::{CDensityMatrix, CStatevector, qis_err_code};
use cqlib_core::qis::entropy;

/// Convert a C qubit-index array (`ptr`, `len`) into a `Vec<usize>`.
/// An empty array is valid; a NULL pointer with a positive length is not.
#[inline]
fn qubit_indices(ptr: *const u32, len: usize) -> Result<Vec<usize>, i32> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if ptr.is_null() {
        return Err(CqlibError::NullPtr as i32);
    }
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) }
        .iter()
        .map(|&q| q as usize)
        .collect())
}

/// Compute the linear entropy `S_L = 1 - Tr(rho^2)` of a density matrix.
/// Writes the value to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_linear_entropy(dm: *const CDensityMatrix, out: *mut f64) -> i32 {
    if dm.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*dm };
    match entropy::linear_entropy(&wrapper.inner) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the Renyi entropy of order `alpha` (base-2 logarithm).
/// `alpha` must be positive; values close to 1 fall back to the Von
/// Neumann entropy. Writes the value to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_renyi_entropy(
    dm: *const CDensityMatrix,
    alpha: f64,
    out: *mut f64,
) -> i32 {
    if dm.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if !alpha.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    let wrapper = unsafe { &*dm };
    match entropy::renyi_entropy(&wrapper.inner, alpha) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the bipartite entanglement entropy of a pure state with
/// respect to subsystem A (`subsys_a` is an array of `len` qubit indices).
/// Writes the value (in bits) to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_entanglement_entropy(
    sv: *const CStatevector,
    subsys_a: *const u32,
    len: usize,
    out: *mut f64,
) -> i32 {
    if sv.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let indices = match qubit_indices(subsys_a, len) {
        Ok(indices) => indices,
        Err(code) => return code,
    };
    let wrapper = unsafe { &*sv };
    match entropy::entanglement_entropy_pure(&wrapper.inner, &indices) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the bipartite entanglement entropy of a pure state with
/// respect to subsystem A (`subsys_a` is an array of `len` qubit indices).
/// Writes the value (in bits) to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_entanglement_entropy_pure(
    sv: *const CStatevector,
    subsys_a: *const u32,
    len: usize,
    out: *mut f64,
) -> i32 {
    if sv.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let indices = match qubit_indices(subsys_a, len) {
        Ok(indices) => indices,
        Err(code) => return code,
    };
    let wrapper = unsafe { &*sv };
    match entropy::entanglement_entropy_pure(&wrapper.inner, &indices) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the negativity entanglement measure of a bipartite density
/// matrix, transposing subsystem A (`subsys_a` is an array of `len` qubit
/// indices). Writes the value to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_negativity(
    dm: *const CDensityMatrix,
    subsys_a: *const u32,
    len: usize,
    out: *mut f64,
) -> i32 {
    if dm.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let indices = match qubit_indices(subsys_a, len) {
        Ok(indices) => indices,
        Err(code) => return code,
    };
    let wrapper = unsafe { &*dm };
    match entropy::negativity(&wrapper.inner, &indices) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the concurrence of a 2-qubit density matrix (0 for separable
/// states, 1 for maximally entangled states). Writes the value to `out`
/// and returns 0 on success; non-2-qubit inputs return an error code.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_concurrence(dm: *const CDensityMatrix, out: *mut f64) -> i32 {
    if dm.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*dm };
    match entropy::concurrence(&wrapper.inner) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the entanglement of formation (in bits) of a 2-qubit density
/// matrix, derived from the concurrence. Writes the value to `out` and
/// returns 0 on success; non-2-qubit inputs return an error code.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_entanglement_of_formation(
    dm: *const CDensityMatrix,
    out: *mut f64,
) -> i32 {
    if dm.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*dm };
    match entropy::entanglement_of_formation(&wrapper.inner) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}
