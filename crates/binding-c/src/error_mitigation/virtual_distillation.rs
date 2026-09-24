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

//! C ABI for `VirtualDistillation`.

use crate::circuit::CCircuit;
use crate::error::CqlibError;
use crate::error_mitigation::{CEstimatorFn, CVirtualDistillation, em_error_code};
use crate::qis::CHamiltonian;
use cqlib_core::error_mitigation::VirtualDistillation;

/// Creates a virtual distillation helper over `circuit` with `copies`
/// replicated registers. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn virtual_distillation_new(
    circuit: *const CCircuit,
    copies: usize,
) -> *mut CVirtualDistillation {
    if circuit.is_null() || copies == 0 {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner }.clone();
    match VirtualDistillation::new(circuit, copies) {
        Ok(inner) => Box::into_raw(Box::new(CVirtualDistillation { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a virtual distillation helper. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn virtual_distillation_free(ptr: *mut CVirtualDistillation) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Builds the copy-swap circuit used by the virtual distillation protocol.
///
/// Returns an owned `CCircuit*` (free with `circuit_free`), or NULL
/// on error.
#[unsafe(no_mangle)]
pub extern "C" fn virtual_distillation_build_circuit(
    ptr: *const CVirtualDistillation,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let vd = unsafe { &(*ptr).inner };
    match vd.build_copy_swap_circuit() {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Builds the copy-swap circuit from the base circuit (the C mirror of
/// `VirtualDistillation::build_copy_swap_circuit`).
///
/// The circuit contains one preparation per configured copy plus pairwise
/// SWAPs against the first copy. Returns an owned `CCircuit*` (free with
/// `circuit_free`), or NULL on NULL input or a build failure.
#[unsafe(no_mangle)]
pub extern "C" fn virtual_distillation_build_copy_swap_circuit(
    ptr: *const CVirtualDistillation,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let vd = unsafe { &(*ptr).inner };
    match vd.build_copy_swap_circuit() {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

// ===== Fine-grained VirtualDistillation API (checklist section 5) =====

/// Returns the configured number of copies, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn virtual_distillation_copies(ptr: *const CVirtualDistillation) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.copies() }
}

/// Updates the configured number of copies (at least 2 required).
///
/// Returns 0 on success, -1 for NULL, or -8 for invalid copies.
#[unsafe(no_mangle)]
pub extern "C" fn virtual_distillation_set_copies(
    ptr: *mut CVirtualDistillation,
    copies: usize,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let vd = unsafe { &mut *ptr };
    match vd.inner.set_copies(copies) {
        Ok(()) => CqlibError::Ok as i32,
        Err(err) => em_error_code(&err),
    }
}

/// Runs the numerator circuit through `estimator` and writes the estimated
/// mean and variance to the out pointers.
///
/// The estimator receives the copy-swap circuit, a Hamiltonian expanded to
/// the full copy-swap width, and `shots`. Returns 0 on success or a negative
/// error code.
#[unsafe(no_mangle)]
pub extern "C" fn virtual_distillation_run_numerator_circuit(
    ptr: *const CVirtualDistillation,
    hamiltonian: *const CHamiltonian,
    shots: usize,
    estimator: CEstimatorFn,
    out_mean: *mut f64,
    out_variance: *mut f64,
) -> i32 {
    if ptr.is_null() || hamiltonian.is_null() || out_mean.is_null() || out_variance.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let vd = unsafe { &(*ptr).inner };
    let hamiltonian = unsafe { &(*hamiltonian).inner };
    let estimator = crate::error_mitigation::wrap_estimator(estimator);
    match vd.run_numerator_circuit(hamiltonian, shots, &estimator) {
        Ok((mean, variance)) => {
            unsafe {
                *out_mean = mean;
                *out_variance = variance;
            }
            CqlibError::Ok as i32
        }
        Err(err) => em_error_code(&err),
    }
}

/// Runs the denominator circuit through `estimator` and writes the estimated
/// mean and variance to the out pointers.
///
/// The estimator receives the copy-swap circuit, no Hamiltonian (NULL), and
/// `shots`. Returns 0 on success or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn virtual_distillation_run_denominator_circuit(
    ptr: *const CVirtualDistillation,
    shots: usize,
    estimator: CEstimatorFn,
    out_mean: *mut f64,
    out_variance: *mut f64,
) -> i32 {
    if ptr.is_null() || out_mean.is_null() || out_variance.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let vd = unsafe { &(*ptr).inner };
    let estimator = crate::error_mitigation::wrap_estimator(estimator);
    match vd.run_denominator_circuit(shots, &estimator) {
        Ok((mean, variance)) => {
            unsafe {
                *out_mean = mean;
                *out_variance = variance;
            }
            CqlibError::Ok as i32
        }
        Err(_) => CqlibError::CircuitError as i32,
    }
}

/// Runs the full virtual distillation protocol and writes the mitigated mean
/// and variance to the out pointers (`mu_vd = mu_numerator / mu_denominator`).
///
/// `shots_numerator` and `shots_denominator` are forwarded to the estimator;
/// sampling randomness is owned by the estimator (the core API takes no
/// seed). Returns 0 on success or a negative error code (-8 when the
/// Hamiltonian width does not match the circuit, -7 when the denominator
/// mean is zero).
#[unsafe(no_mangle)]
pub extern "C" fn virtual_distillation_run_vd(
    ptr: *const CVirtualDistillation,
    hamiltonian: *const CHamiltonian,
    shots_numerator: usize,
    shots_denominator: usize,
    estimator: CEstimatorFn,
    out_mean: *mut f64,
    out_variance: *mut f64,
) -> i32 {
    if ptr.is_null() || hamiltonian.is_null() || out_mean.is_null() || out_variance.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let vd = unsafe { &(*ptr).inner };
    let hamiltonian = unsafe { &(*hamiltonian).inner };
    let estimator = crate::error_mitigation::wrap_estimator(estimator);
    match vd.run_vd(hamiltonian, shots_numerator, shots_denominator, &estimator) {
        Ok((mean, variance)) => {
            unsafe {
                *out_mean = mean;
                *out_variance = variance;
            }
            CqlibError::Ok as i32
        }
        Err(err) => em_error_code(&err),
    }
}
