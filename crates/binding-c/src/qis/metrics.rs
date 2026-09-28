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

//! C ABI for quantum information metrics (`qis/metrics.rs`).

use crate::error::CqlibError;
use crate::qis::{CDensityMatrix, CStatevector, qis_err_code};
use cqlib_core::qis::metrics;

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

/// Compute the purity `Tr(rho^2)`-style norm of a statevector (1.0 for a
/// normalized pure state). Writes the value to `out`, returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_purity(sv: *const CStatevector, out: *mut f64) -> i32 {
    if sv.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*sv };
    match metrics::purity_pure(&wrapper.inner) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the purity `Tr(rho^2)` of a density matrix. Writes the value
/// to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_purity(dm: *const CDensityMatrix, out: *mut f64) -> i32 {
    if dm.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*dm };
    match metrics::purity_mixed(&wrapper.inner) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the state fidelity `|<psi|phi>|^2` between two statevectors.
/// Writes the value to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_fidelity(
    sv1: *const CStatevector,
    sv2: *const CStatevector,
    out: *mut f64,
) -> i32 {
    if sv1.is_null() || sv2.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let a = unsafe { &(*sv1).inner };
    let b = unsafe { &(*sv2).inner };
    match metrics::state_fidelity_pure(a, b) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the trace distance `sqrt(1 - |<psi|phi>|^2)` between two pure
/// states. Writes the value to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_trace_distance(
    sv1: *const CStatevector,
    sv2: *const CStatevector,
    out: *mut f64,
) -> i32 {
    if sv1.is_null() || sv2.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let a = unsafe { &(*sv1).inner };
    let b = unsafe { &(*sv2).inner };
    match metrics::trace_distance_pure(a, b) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the fidelity `<psi|rho|psi>` between a pure statevector and a
/// density matrix. Writes the value to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_fidelity_pure_mixed(
    sv: *const CStatevector,
    dm: *const CDensityMatrix,
    out: *mut f64,
) -> i32 {
    if sv.is_null() || dm.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let a = unsafe { &(*sv).inner };
    let b = unsafe { &(*dm).inner };
    match metrics::state_fidelity_pure_mixed(a, b) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the Von Neumann entropy `-Tr(rho log2 rho)` of a density
/// matrix (in bits). Writes the value to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_entropy(dm: *const CDensityMatrix, out: *mut f64) -> i32 {
    if dm.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*dm };
    match metrics::entropy(&wrapper.inner) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the trace distance `1/2 Tr|rho - sigma|` between two density
/// matrices. Writes the value to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_trace_distance(
    dm1: *const CDensityMatrix,
    dm2: *const CDensityMatrix,
    out: *mut f64,
) -> i32 {
    if dm1.is_null() || dm2.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let a = unsafe { &(*dm1).inner };
    let b = unsafe { &(*dm2).inner };
    match metrics::trace_distance_mixed(a, b) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the state fidelity `(Tr sqrt(sqrt(rho) sigma sqrt(rho)))^2`
/// between two density matrices. Writes the value to `out` and returns
/// 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_fidelity(
    dm1: *const CDensityMatrix,
    dm2: *const CDensityMatrix,
    out: *mut f64,
) -> i32 {
    if dm1.is_null() || dm2.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let a = unsafe { &(*dm1).inner };
    let b = unsafe { &(*dm2).inner };
    match metrics::state_fidelity_mixed(a, b) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Perform the partial transpose on `qubits` (array of `len` indices).
/// Returns a new `CDensityMatrix*` (free with `density_matrix_free`),
/// or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_partial_transpose(
    dm: *const CDensityMatrix,
    qubits: *const u32,
    len: usize,
) -> *mut CDensityMatrix {
    if dm.is_null() {
        return std::ptr::null_mut();
    }
    let indices = match qubit_indices(qubits, len) {
        Ok(indices) => indices,
        Err(_) => return std::ptr::null_mut(),
    };
    let wrapper = unsafe { &*dm };
    match metrics::partial_transpose(&wrapper.inner, &indices) {
        Ok(result) => Box::into_raw(Box::new(CDensityMatrix { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Compute the logarithmic negativity `log2 ||rho^{T_A}||_1` with respect
/// to subsystem A (`sys_a` is an array of `len` qubit indices). Writes the
/// value to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_logarithmic_negativity(
    dm: *const CDensityMatrix,
    sys_a: *const u32,
    len: usize,
    out: *mut f64,
) -> i32 {
    if dm.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let indices = match qubit_indices(sys_a, len) {
        Ok(indices) => indices,
        Err(code) => return code,
    };
    let wrapper = unsafe { &*dm };
    match metrics::logarithmic_negativity(&wrapper.inner, &indices) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}
