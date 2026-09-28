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

//! C ABI for `ZNEMitigation` (zero-noise extrapolation helper).

use crate::circuit::CCircuit;
use crate::device::standard_gate_from_name;
use crate::error::CqlibError;
use crate::error_mitigation::{CCircuitList, CEstimatorFn, em_error_code};
use crate::qis::CHamiltonian;
use cqlib_core::circuit::gate::Instruction;
use cqlib_core::error_mitigation::{ExtrapolateMethod, ZNEMitigation};
use std::ffi::CStr;
use std::os::raw::c_char;

/// Opaque handle around [`ZNEMitigation`] for C ABI.
pub struct CZneMitigation {
    pub inner: ZNEMitigation,
}

/// Creates a ZNE helper for `circuit` with the given fold levels
/// (noise factors are derived as `2 * level + 1`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn zne_mitigation_new(
    circuit: *const CCircuit,
    fold_levels: *const i32,
    num_levels: usize,
) -> *mut CZneMitigation {
    if circuit.is_null() || num_levels == 0 || fold_levels.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner }.clone();
    let levels = unsafe { std::slice::from_raw_parts(fold_levels, num_levels) }.to_vec();
    Box::into_raw(Box::new(CZneMitigation {
        inner: ZNEMitigation::new(circuit, levels),
    }))
}

/// Frees a ZNE helper. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn zne_mitigation_free(ptr: *mut CZneMitigation) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Folds the circuit at every configured level.
///
/// `gate_names` is an optional comma-separated list of gate names to fold
/// selectively (e.g. "H,CX"); NULL folds the whole circuit globally.
/// Returns an owned `CCircuitList*` (one folded circuit per level),
/// or NULL on error. Free with `circuit_list_free`.
#[unsafe(no_mangle)]
pub extern "C" fn zne_mitigation_fold_circuit(
    ptr: *const CZneMitigation,
    gate_names: *const c_char,
) -> *mut CCircuitList {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let gate_set = if gate_names.is_null() {
        None
    } else {
        let names = match unsafe { CStr::from_ptr(gate_names) }.to_str() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        let mut gates = Vec::new();
        for part in names.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            match standard_gate_from_name(part) {
                Some(gate) => gates.push(Instruction::Standard(gate)),
                None => return std::ptr::null_mut(),
            }
        }
        Some(gates)
    };
    let zne = unsafe { &(*ptr).inner };
    match zne.fold_circuits(gate_set.as_deref()) {
        Ok(circuits) => Box::into_raw(Box::new(CCircuitList { items: circuits })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns the number of circuits in the list, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_list_len(ptr: *const CCircuitList) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).items.len() }
}

/// Returns the circuit at `index` as an owned `CCircuit*`.
/// Caller must free with `circuit_free`. Returns NULL on out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_list_get(ptr: *const CCircuitList, index: usize) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let list = unsafe { &*ptr };
    match list.items.get(index) {
        Some(circuit) => Box::into_raw(Box::new(CCircuit {
            inner: circuit.clone(),
        })),
        None => std::ptr::null_mut(),
    }
}

/// Frees a circuit list. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_list_free(ptr: *mut CCircuitList) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the noise factor for fold level `index` (`2 * level + 1`),
/// or 0 on error.
#[unsafe(no_mangle)]
pub extern "C" fn zne_mitigation_noise_factor(ptr: *const CZneMitigation, index: usize) -> i32 {
    if ptr.is_null() {
        return 0;
    }
    let zne = unsafe { &(*ptr).inner };
    zne.noise_factors().get(index).copied().unwrap_or(0)
}

// ===== Fine-grained ZNE API (checklist section 5) =====

/// Extrapolation method tags for `zne_extrapolate`.
///
/// | Value | Method      |
/// |-------|-------------|
/// | 0     | Polynomial  |
/// | 1     | Exponential |
pub const ZNE_EXTRAPOLATE_POLYNOMIAL: u8 = 0;
pub const ZNE_EXTRAPOLATE_EXPONENTIAL: u8 = 1;

/// Parses an optional comma-separated gate-name list into a selective-folding
/// gate set. `Ok(None)` means global folding; `Err(())` means an unknown gate
/// name or invalid UTF-8.
fn parse_gate_set(gate_names: *const c_char) -> Result<Option<Vec<Instruction>>, ()> {
    if gate_names.is_null() {
        return Ok(None);
    }
    let names = unsafe { CStr::from_ptr(gate_names) }
        .to_str()
        .map_err(|_| ())?;
    let mut gates = Vec::new();
    for part in names.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match standard_gate_from_name(part) {
            Some(gate) => gates.push(Instruction::Standard(gate)),
            None => return Err(()),
        }
    }
    Ok(Some(gates))
}

/// Returns the original (unfolded) circuit as an owned `CCircuit*`.
/// Caller must free with `circuit_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn zne_circuit(ptr: *const CZneMitigation) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let zne = unsafe { &(*ptr).inner };
    Box::into_raw(Box::new(CCircuit {
        inner: zne.circuit().clone(),
    }))
}

/// Returns the number of configured fold levels, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn zne_fold_levels_len(ptr: *const CZneMitigation) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.fold_levels().len() }
}

/// Copies the configured fold levels into `buffer` (two-step pattern; pair
/// with `zne_fold_levels_len`). `len` must equal `zne_fold_levels_len(ptr)`.
/// Returns 0 on success or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn zne_fold_levels(ptr: *const CZneMitigation, buffer: *mut i32, len: usize) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let levels = unsafe { &(*ptr).inner }.fold_levels();
    if len != levels.len() {
        return CqlibError::InvalidParam as i32;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(levels.as_ptr(), buffer, len);
    }
    CqlibError::Ok as i32
}

/// Returns the number of noise factors, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn zne_noise_factors_len(ptr: *const CZneMitigation) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.noise_factors().len() }
}

/// Copies the noise factors (`2 * level + 1` per fold level) into `buffer`
/// (two-step pattern; pair with `zne_noise_factors_len`). `len` must equal
/// `zne_noise_factors_len(ptr)`. Returns 0 on success or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn zne_noise_factors(
    ptr: *const CZneMitigation,
    buffer: *mut i32,
    len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let factors = unsafe { &(*ptr).inner }.noise_factors();
    if len != factors.len() {
        return CqlibError::InvalidParam as i32;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(factors.as_ptr(), buffer, len);
    }
    CqlibError::Ok as i32
}

/// Returns the number of folded circuits produced for the configured fold
/// levels (one per level), or 0 for NULL or an invalid `gate_names` string.
#[unsafe(no_mangle)]
pub extern "C" fn zne_fold_circuits_len(
    ptr: *const CZneMitigation,
    gate_names: *const c_char,
) -> usize {
    if ptr.is_null() || parse_gate_set(gate_names).is_err() {
        return 0;
    }
    unsafe { (*ptr).inner.fold_levels().len() }
}

/// Folds the circuit at every configured level (two-step pattern; pair with
/// `zne_fold_circuits_len`).
///
/// `gate_names` is an optional comma-separated list of gate names to fold
/// selectively (e.g. "H,CX"); NULL folds the whole circuit globally. `buffer`
/// receives `len` owned `CCircuit*` handles, one per fold level; the caller
/// must free every entry with `circuit_free`. Returns 0 on success or a
/// negative error code (-4 for unknown gate names, -8 for a length
/// mismatch, -3 for folding failures).
#[unsafe(no_mangle)]
pub extern "C" fn zne_fold_circuits(
    ptr: *const CZneMitigation,
    gate_names: *const c_char,
    buffer: *mut *mut CCircuit,
    len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let gate_set = match parse_gate_set(gate_names) {
        Ok(gate_set) => gate_set,
        Err(()) => return CqlibError::ParseError as i32,
    };
    let zne = unsafe { &(*ptr).inner };
    if len != zne.fold_levels().len() {
        return CqlibError::InvalidParam as i32;
    }
    match zne.fold_circuits(gate_set.as_deref()) {
        Ok(circuits) => {
            for (slot, circuit) in unsafe { std::slice::from_raw_parts_mut(buffer, len) }
                .iter_mut()
                .zip(circuits)
            {
                *slot = Box::into_raw(Box::new(CCircuit { inner: circuit }));
            }
            CqlibError::Ok as i32
        }
        Err(_) => CqlibError::CircuitError as i32,
    }
}

/// Extrapolates the zero-noise expectation from `noisy_results` (one value
/// per noise factor, in fold-level order).
///
/// `method` is `ZNE_EXTRAPOLATE_POLYNOMIAL` (uses `degree`; it must be
/// smaller than the number of data points) or `ZNE_EXTRAPOLATE_EXPONENTIAL`
/// (requires all values to be positive). Writes the extrapolated value to
/// `out_value`. Returns 0 on success or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn zne_extrapolate(
    ptr: *const CZneMitigation,
    noisy_results: *const f64,
    len: usize,
    method: u8,
    degree: usize,
    out_value: *mut f64,
) -> i32 {
    if ptr.is_null() || noisy_results.is_null() || out_value.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let method = match method {
        ZNE_EXTRAPOLATE_POLYNOMIAL => ExtrapolateMethod::Polynomial,
        ZNE_EXTRAPOLATE_EXPONENTIAL => ExtrapolateMethod::Exponential,
        _ => return CqlibError::InvalidParam as i32,
    };
    let results = unsafe { std::slice::from_raw_parts(noisy_results, len) };
    let zne = unsafe { &(*ptr).inner };
    match zne.extrapolate(results, method, degree) {
        Ok(value) => {
            unsafe { *out_value = value };
            CqlibError::Ok as i32
        }
        Err(err) => em_error_code(&err),
    }
}

/// Extrapolates the zero-noise expectation with a polynomial fit of the
/// given `degree` (must be smaller than the number of noise factors).
/// Writes the extrapolated value to `out_value`. Returns 0 on success or a
/// negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn zne_poly_extrapolate(
    ptr: *const CZneMitigation,
    noisy_results: *const f64,
    len: usize,
    degree: usize,
    out_value: *mut f64,
) -> i32 {
    if ptr.is_null() || noisy_results.is_null() || out_value.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let results = unsafe { std::slice::from_raw_parts(noisy_results, len) };
    let zne = unsafe { &(*ptr).inner };
    match zne.poly_extrapolate(results, degree) {
        Ok(value) => {
            unsafe { *out_value = value };
            CqlibError::Ok as i32
        }
        Err(err) => em_error_code(&err),
    }
}

/// Extrapolates the zero-noise expectation with an exponential-decay fit
/// `y(x) = A * exp(-x / tau)` (all values must be positive). Writes `A`, the
/// extrapolated value at `x = 0`, to `out_value`. Returns 0 on success or a
/// negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn zne_exp_extrapolate(
    ptr: *const CZneMitigation,
    noisy_results: *const f64,
    len: usize,
    out_value: *mut f64,
) -> i32 {
    if ptr.is_null() || noisy_results.is_null() || out_value.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let results = unsafe { std::slice::from_raw_parts(noisy_results, len) };
    let zne = unsafe { &(*ptr).inner };
    match zne.exp_extrapolate(results) {
        Ok(value) => {
            unsafe { *out_value = value };
            CqlibError::Ok as i32
        }
        Err(err) => em_error_code(&err),
    }
}

/// Returns the number of expectation values produced by the error-mitigation
/// sequence (one per fold level), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn zne_run_em_sequence_len(ptr: *const CZneMitigation) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.noise_factors().len() }
}

/// Returns the number of expectation values produced by the error-mitigation
/// sequence with explicit shots (one per fold level), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn zne_run_em_sequence_with_shots_len(ptr: *const CZneMitigation) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.noise_factors().len() }
}

/// Runs the error-mitigation sequence: folds the circuit at every configured
/// level and estimates one expectation value per folded circuit through the
/// `estimator` callback (two-step pattern; pair with
/// `zne_run_em_sequence_len`).
///
/// `gate_names` optionally selects gates for folding (NULL folds globally).
/// The estimator receives the folded circuit, the Hamiltonian, and shot
/// count 0 ("not specified"). `buffer` receives `len` f64 values. Returns 0
/// on success or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn zne_run_em_sequence(
    ptr: *const CZneMitigation,
    gate_names: *const c_char,
    hamiltonian: *const CHamiltonian,
    estimator: CEstimatorFn,
    buffer: *mut f64,
    len: usize,
) -> i32 {
    zne_run_em_sequence_impl(ptr, gate_names, hamiltonian, None, estimator, buffer, len)
}

/// Same as [`zne_run_em_sequence`], but forwards `shots` to the estimator
/// instead of "not specified".
#[unsafe(no_mangle)]
pub extern "C" fn zne_run_em_sequence_with_shots(
    ptr: *const CZneMitigation,
    gate_names: *const c_char,
    hamiltonian: *const CHamiltonian,
    shots: usize,
    estimator: CEstimatorFn,
    buffer: *mut f64,
    len: usize,
) -> i32 {
    zne_run_em_sequence_impl(
        ptr,
        gate_names,
        hamiltonian,
        Some(shots),
        estimator,
        buffer,
        len,
    )
}

/// Shared implementation for the `zne_run_em_sequence*` entry points.
fn zne_run_em_sequence_impl(
    ptr: *const CZneMitigation,
    gate_names: *const c_char,
    hamiltonian: *const CHamiltonian,
    shots: Option<usize>,
    estimator: CEstimatorFn,
    buffer: *mut f64,
    len: usize,
) -> i32 {
    if ptr.is_null() || hamiltonian.is_null() || buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let gate_set = match parse_gate_set(gate_names) {
        Ok(gate_set) => gate_set,
        Err(()) => return CqlibError::ParseError as i32,
    };
    let zne = unsafe { &(*ptr).inner };
    if len != zne.noise_factors().len() {
        return CqlibError::InvalidParam as i32;
    }
    let hamiltonian = unsafe { &(*hamiltonian).inner };
    let estimator = crate::error_mitigation::wrap_estimator(estimator);
    match zne.run_em_sequence_with_shots(gate_set.as_deref(), hamiltonian, shots, &estimator) {
        Ok(sequence) => {
            unsafe {
                std::ptr::copy_nonoverlapping(sequence.as_ptr(), buffer, len);
            }
            CqlibError::Ok as i32
        }
        Err(err) => em_error_code(&err),
    }
}
