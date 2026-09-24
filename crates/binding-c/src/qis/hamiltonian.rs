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

//! C ABI for `Hamiltonian` observables.

use crate::circuit::CCircuit;
use crate::error::CqlibError;
use crate::qis::{CHamiltonian, CPauliString, CStatevector, qis_err_code};
use cqlib_core::qis::evolution::TrotterMode;
use cqlib_core::qis::{Hamiltonian, Observable};
use num_complex::Complex64;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;

/// C-side snapshot of one measurement basis with its observed outcome
/// probabilities, used by `hamiltonian_expectation_probs`.
#[repr(C)]
pub struct CProbMeasurement {
    /// Measurement basis as a Pauli string (borrowed, not consumed).
    pub basis: *const CPauliString,
    /// Array of `len` computational-basis bitstrings, one `'0'`/`'1'` char per
    /// qubit, big-endian (qubit 0 is the last character).
    pub states: *const *const c_char,
    /// Array of `len` observed probabilities, paired with `states` by index.
    pub probs: *const f64,
    /// Number of entries in `states` and `probs`.
    pub len: usize,
}

/// Trotter-Suzuki decomposition mode tag: first-order Lie-Trotter.
pub const TROTTER_MODE_FIRST_ORDER: u32 = 0;
/// Trotter-Suzuki decomposition mode tag: second-order Strang splitting.
pub const TROTTER_MODE_SECOND_ORDER: u32 = 1;

/// Create a new empty Hamiltonian acting on `num_qubits` qubits.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_new(num_qubits: usize) -> *mut CHamiltonian {
    Box::into_raw(Box::new(CHamiltonian {
        inner: Hamiltonian::new(num_qubits),
    }))
}

/// Free a Hamiltonian. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_free(ptr: *mut CHamiltonian) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Construct a Hamiltonian from a single Pauli string (coefficient = 1.0).
/// Takes ownership of the `CPauliString` (frees it).
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_from_pauli(pauli: *mut CPauliString) -> *mut CHamiltonian {
    if pauli.is_null() {
        return std::ptr::null_mut();
    }
    let pauli = unsafe { Box::from_raw(pauli) };
    let ham = Hamiltonian::from_pauli(pauli.inner);
    Box::into_raw(Box::new(CHamiltonian { inner: ham }))
}

/// Add a Pauli term with a complex coefficient.
/// The Pauli string is cloned; the handle is not consumed.
/// `coeff_re` and `coeff_im` form the real and imaginary parts.
/// Returns 0 on success, or a negative error code on qubit-count mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_add_term(
    ptr: *mut CHamiltonian,
    pauli: *const CPauliString,
    coeff_re: f64,
    coeff_im: f64,
) -> i32 {
    if ptr.is_null() || pauli.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let p = unsafe { &(*pauli).inner };
    let coeff = Complex64::new(coeff_re, coeff_im);
    match wrapper.inner.add_term(p.clone(), coeff) {
        Ok(()) => 0,
        Err(err) => qis_err_code(&err),
    }
}

/// Return the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_num_qubits(ptr: *const CHamiltonian) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits }
}

/// Return the number of Pauli terms, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_num_terms(ptr: *const CHamiltonian) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.terms.len() }
}

/// Simplify the Hamiltonian in place: combine terms with the same Pauli
/// string and remove near-zero coefficients. Returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_simplify(ptr: *mut CHamiltonian) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.simplify();
    0
}

/// Scale all coefficients by a complex factor in place.
/// `factor_re` and `factor_im` form the real and imaginary parts.
/// Returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_scale(ptr: *mut CHamiltonian, factor_re: f64, factor_im: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if !factor_re.is_finite() || !factor_im.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.scale(Complex64::new(factor_re, factor_im));
    0
}

/// Return the side length of the dense matrix (`2^N`), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_to_matrix_len(ptr: *const CHamiltonian) -> usize {
    if ptr.is_null() {
        return 0;
    }
    1usize << unsafe { (*ptr).inner.num_qubits }
}

/// Copy the dense `2^N x 2^N` matrix into `buffer` (row-major,
/// `Complex64` values). `len` must equal
/// `hamiltonian_to_matrix_len(ptr) * hamiltonian_to_matrix_len(ptr)`.
/// Returns 0 on success, or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_to_matrix(
    ptr: *const CHamiltonian,
    buffer: *mut Complex64,
    len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let matrix = match wrapper.inner.to_matrix() {
        Ok(matrix) => matrix,
        Err(err) => return qis_err_code(&err),
    };
    if matrix.len() != len {
        return CqlibError::InvalidParam as i32;
    }
    let flat: Vec<Complex64> = matrix.iter().cloned().collect();
    unsafe {
        std::ptr::copy_nonoverlapping(flat.as_ptr(), buffer, len);
    }
    0
}

/// Return 1 if all Pauli terms of the Hamiltonian mutually commute,
/// 0 otherwise, or -1 for NULL. When all terms commute, the time
/// evolution decomposes exactly into per-term Pauli rotations.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_all_terms_commute(ptr: *const CHamiltonian) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    if wrapper.inner.all_terms_commute() {
        1
    } else {
        0
    }
}

/// Map a C Trotter-mode tag to the Rust enum. The `Randomized` variant
/// is not exposed through the C ABI.
#[inline]
fn trotter_mode_from_tag(mode: u32) -> Option<TrotterMode> {
    match mode {
        TROTTER_MODE_FIRST_ORDER => Some(TrotterMode::FirstOrder),
        TROTTER_MODE_SECOND_ORDER => Some(TrotterMode::SecondOrder),
        _ => None,
    }
}

/// Convert the Hamiltonian to a Trotterized time-evolution circuit
/// approximating `U(t) = exp(-i H t)`.
///
/// `mode` must be `TROTTER_MODE_FIRST_ORDER` (0) or
/// `TROTTER_MODE_SECOND_ORDER` (1). Returns a new `CCircuit*` (free with
/// `circuit_free`), or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_to_trotter_circuit(
    ptr: *const CHamiltonian,
    time: f64,
    steps: usize,
    mode: u32,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    if !time.is_finite() {
        return std::ptr::null_mut();
    }
    let mode = match trotter_mode_from_tag(mode) {
        Some(mode) => mode,
        None => return std::ptr::null_mut(),
    };
    let wrapper = unsafe { &*ptr };
    match wrapper.inner.to_trotter_circuit(time, steps, mode) {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Convert the Hamiltonian to a time-evolution circuit, using the exact
/// single-pass decomposition when all terms commute and falling back to
/// the Trotter approximation (`mode`, `steps`) otherwise.
///
/// `mode` must be `TROTTER_MODE_FIRST_ORDER` (0) or
/// `TROTTER_MODE_SECOND_ORDER` (1). Returns a new `CCircuit*` (free with
/// `circuit_free`), or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_to_evolution_circuit(
    ptr: *const CHamiltonian,
    time: f64,
    steps: usize,
    mode: u32,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    if !time.is_finite() {
        return std::ptr::null_mut();
    }
    let mode = match trotter_mode_from_tag(mode) {
        Some(mode) => mode,
        None => return std::ptr::null_mut(),
    };
    let wrapper = unsafe { &*ptr };
    match wrapper.inner.to_evolution_circuit(time, steps, mode) {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Compute the expectation value of the Hamiltonian from measurement outcome
/// probabilities.
///
/// `measurements` lists one entry per measured basis; each term of the
/// Hamiltonian is evaluated in the first compatible measured basis. The
/// result is written to `out`.
///
/// Returns 0 on success, -1 on NULL inputs (including NULL `measurements`
/// for a non-zero count, NULL basis, or NULL state/prob arrays for a
/// non-zero entry length), -4 for invalid UTF-8 state strings, or another
/// negative error code for core failures (e.g. -7 when a term has no
/// compatible measurement basis).
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_expectation_probs(
    ptr: *const CHamiltonian,
    measurements: *const CProbMeasurement,
    measurements_len: usize,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if measurements_len > 0 && measurements.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let mut entries: Vec<(cqlib_core::qis::PauliString, HashMap<String, f64>)> =
        Vec::with_capacity(measurements_len);
    for i in 0..measurements_len {
        let entry = unsafe { &*measurements.add(i) };
        if entry.basis.is_null() {
            return CqlibError::NullPtr as i32;
        }
        if entry.len > 0 && (entry.states.is_null() || entry.probs.is_null()) {
            return CqlibError::NullPtr as i32;
        }
        let mut probs = HashMap::with_capacity(entry.len);
        for j in 0..entry.len {
            let state_ptr = unsafe { *entry.states.add(j) };
            if state_ptr.is_null() {
                return CqlibError::NullPtr as i32;
            }
            let state = match unsafe { CStr::from_ptr(state_ptr) }.to_str() {
                Ok(state) => state,
                Err(_) => return CqlibError::ParseError as i32,
            };
            probs.insert(state.to_string(), unsafe { *entry.probs.add(j) });
        }
        entries.push((unsafe { (*entry.basis).inner.clone() }, probs));
    }
    let wrapper = unsafe { &*ptr };
    match wrapper.inner.expectation_probs(&entries) {
        Ok(value) => {
            unsafe {
                *out = value;
            }
            CqlibError::Ok as i32
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the variance of the Hamiltonian for a statevector and write it to
/// `out`. Returns 0 on success, -1 on NULL pointers, -8 on qubit-count
/// mismatch, non-Hermitian terms, or other invalid parameters, or -7 for
/// other simulation errors.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_variance_statevector(
    ptr: *const CHamiltonian,
    sv: *const CStatevector,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || sv.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let sv = unsafe { &(*sv).inner };
    match wrapper.inner.variance_statevector(sv) {
        Ok(variance) => {
            unsafe {
                *out = variance;
            }
            CqlibError::Ok as i32
        }
        Err(err) => qis_err_code(&err),
    }
}
