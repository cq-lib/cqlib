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

use crate::error::CqlibError;
use crate::qis::{CHamiltonian, CPauliString, qis_err_code};
use cqlib_core::qis::Hamiltonian;
use num_complex::Complex64;

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
