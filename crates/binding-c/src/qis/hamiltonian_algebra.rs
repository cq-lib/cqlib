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

//! Checked Pauli sums. Inputs are borrowed and outputs owned; errors do not
//! overwrite outputs. Only exact zeros are removed unless chop is requested.

use super::algebra::{algebra_err_code, check_pauli_width, input_slice, write_result};
use super::{CHamiltonian, CPauliString};
use cqlib_core::qis::Hamiltonian;
use libc::size_t;
use num_complex::Complex64;

/// Borrowed Pauli term for checked batch construction. Pauli handles are copied.
#[repr(C)]
pub struct CHamiltonianTerm {
    pub pauli: *const CPauliString,
    pub coefficient: Complex64,
}

fn owned(inner: Hamiltonian) -> *mut CHamiltonian {
    Box::into_raw(Box::new(CHamiltonian { inner }))
}

/// Construct identity, including scalar one on zero qubits. Returns -1 NULL out,
/// -7 width beyond the BitVec representation limit. Free with hamiltonian_free.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_identity(num_qubits: size_t, out: *mut *mut CHamiltonian) -> i32 {
    if out.is_null() {
        return -1;
    }
    if let Err(e) = check_pauli_width(num_qubits) {
        return e;
    }
    unsafe {
        *out = owned(Hamiltonian::identity(num_qubits));
    }
    0
}

/// Construct a checked, phase-normalized sum, merging duplicate Paulis.
/// Empty input retains num_qubits, including zero. Inputs are copied.
/// Returns -1 NULL term/out/Pauli, -8 width mismatch, nonfinite coefficient or overflow.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_from_terms(
    num_qubits: size_t,
    terms: *const CHamiltonianTerm,
    len: size_t,
    out: *mut *mut CHamiltonian,
) -> i32 {
    if out.is_null() {
        return -1;
    }
    let terms = match unsafe { input_slice(terms, len) } {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mut copied = Vec::with_capacity(terms.len());
    for term in terms {
        if term.pauli.is_null() {
            return -1;
        }
        copied.push((unsafe { &(*term.pauli).inner }.clone(), term.coefficient));
    }
    unsafe { write_result(out, Hamiltonian::from_terms(num_qubits, copied).map(owned)) }
}

/// Return an owned independent structural copy.
/// Free with hamiltonian_free. Returns -1 NULL; copies storage without normalization.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_copy(ptr: *const CHamiltonian, out: *mut *mut CHamiltonian) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let inner = unsafe { &(*ptr).inner };
    unsafe { write_result(out, Ok(owned(inner.clone()))) }
}

/// Return an owned validated copy with phases absorbed and exact zeros removed.
/// Free with hamiltonian_free. Returns -1 NULL, -8 invalid input or merge overflow.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_normalized(
    ptr: *const CHamiltonian,
    out: *mut *mut CHamiltonian,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let inner = unsafe { &(*ptr).inner };
    unsafe { write_result(out, inner.normalized().map(owned)) }
}

/// Return an owned normalized Hermitian copy; imaginary residuals <=1e-10 are projected to zero.
/// Free with hamiltonian_free. Returns -1 NULL, -8 invalid input or non-Hermitian projection.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_normalized_hermitian(
    ptr: *const CHamiltonian,
    out: *mut *mut CHamiltonian,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let inner = unsafe { &(*ptr).inner };
    unsafe { write_result(out, inner.normalized_hermitian().map(owned)) }
}

/// Return an owned conjugate transpose after absorbing Pauli phases.
/// Free with hamiltonian_free. Returns -1 NULL, -8 invalid input or merge overflow.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_adjoint(
    ptr: *const CHamiltonian,
    out: *mut *mut CHamiltonian,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let inner = unsafe { &(*ptr).inner };
    unsafe { write_result(out, inner.adjoint().map(owned)) }
}

/// Checked add, retaining every nonzero coefficient.
/// Returns an owned result. -1 NULL, -8 width mismatch/nonfinite arithmetic,
/// -7 unrepresentable tensor width. No input is modified.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_add(
    left: *const CHamiltonian,
    right: *const CHamiltonian,
    out: *mut *mut CHamiltonian,
) -> i32 {
    if left.is_null() || right.is_null() || out.is_null() {
        return -1;
    }
    unsafe { write_result(out, (*left).inner.try_add(&(*right).inner).map(owned)) }
}

/// Checked sub, retaining every nonzero coefficient.
/// Returns an owned result. -1 NULL, -8 width mismatch/nonfinite arithmetic,
/// -7 unrepresentable tensor width. No input is modified.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_sub(
    left: *const CHamiltonian,
    right: *const CHamiltonian,
    out: *mut *mut CHamiltonian,
) -> i32 {
    if left.is_null() || right.is_null() || out.is_null() {
        return -1;
    }
    unsafe { write_result(out, (*left).inner.try_sub(&(*right).inner).map(owned)) }
}

/// Checked mul, retaining every nonzero coefficient. Right operand acts first.
/// Returns an owned result. -1 NULL, -8 width mismatch/nonfinite arithmetic,
/// -7 unrepresentable tensor width. No input is modified.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_mul(
    left: *const CHamiltonian,
    right: *const CHamiltonian,
    out: *mut *mut CHamiltonian,
) -> i32 {
    if left.is_null() || right.is_null() || out.is_null() {
        return -1;
    }
    unsafe { write_result(out, (*left).inner.try_mul(&(*right).inner).map(owned)) }
}

/// Checked tensor, retaining every nonzero coefficient. Right operand occupies low qubits, left high qubits.
/// Returns an owned result. -1 NULL, -8 width mismatch/nonfinite arithmetic,
/// -7 unrepresentable tensor width. No input is modified.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_tensor(
    left: *const CHamiltonian,
    right: *const CHamiltonian,
    out: *mut *mut CHamiltonian,
) -> i32 {
    if left.is_null() || right.is_null() || out.is_null() {
        return -1;
    }
    unsafe { write_result(out, (*left).inner.tensor(&(*right).inner).map(owned)) }
}

/// Return a checked scaled copy. Returns -1 NULL, -8 nonfinite input/factor/result.
/// Unlike legacy hamiltonian_scale, all stored coefficients are validated.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_scaled(
    ptr: *const CHamiltonian,
    factor: Complex64,
    out: *mut *mut CHamiltonian,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    unsafe { write_result(out, (*ptr).inner.scaled(factor).map(owned)) }
}

/// Embed mapping[source]=destination. Destinations must be distinct and in range;
/// unused target qubits are identity. Not a fermionic mode permutation.
/// Returns -1 NULL, -8 invalid mapping/input, -7 unrepresentable target width.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_remap_qubits(
    ptr: *const CHamiltonian,
    mapping: *const size_t,
    len: size_t,
    target_width: size_t,
    out: *mut *mut CHamiltonian,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let mapping = match unsafe { input_slice(mapping, len) } {
        Ok(v) => v,
        Err(e) => return e,
    };
    unsafe {
        write_result(
            out,
            (*ptr).inner.remap_qubits(mapping, target_width).map(owned),
        )
    }
}

/// Normalize then remove coefficients with magnitude <=atol in place.
/// Returns -1 NULL, -8 negative/nonfinite atol or invalid input. Failure is atomic.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_chop(ptr: *mut CHamiltonian, atol: f64) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match unsafe { &mut (*ptr).inner }.chop(atol) {
        Ok(()) => 0,
        Err(e) => algebra_err_code(&e),
    }
}

/// Compare normalized coefficient differences to atol; writes 0 or 1.
/// Returns -1 NULL, -8 invalid atol, width mismatch or nonfinite arithmetic.
/// atol is a coefficient tolerance, not an operator norm bound. atol=0 is exact.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_equiv(
    left: *const CHamiltonian,
    right: *const CHamiltonian,
    atol: f64,
    out: *mut u8,
) -> i32 {
    if left.is_null() || right.is_null() || out.is_null() {
        return -1;
    }
    unsafe {
        write_result(
            out,
            (*left).inner.equiv(&(*right).inner, atol).map(u8::from),
        )
    }
}

/// Test equivalence with the adjoint; writes 0 or 1. For normalized coefficients
/// this bounds twice the imaginary magnitude. Does not project coefficients.
/// Returns -1 NULL, -8 invalid atol/input or arithmetic overflow.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_is_hermitian(
    ptr: *const CHamiltonian,
    atol: f64,
    out: *mut u8,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    unsafe { write_result(out, (*ptr).inner.is_hermitian(atol).map(u8::from)) }
}

/// Read one stored term by index, returning an independent Pauli handle and
/// coefficient. Free the Pauli with pauli_string_free; it may have zero width.
/// Both outputs are unchanged on error. Returns -1 NULL, -2 invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn hamiltonian_get_term(
    ptr: *const CHamiltonian,
    index: size_t,
    out_pauli: *mut *mut CPauliString,
    out_coefficient: *mut Complex64,
) -> i32 {
    if ptr.is_null() || out_pauli.is_null() || out_coefficient.is_null() {
        return -1;
    }
    let Some((pauli, coefficient)) = (unsafe { &(*ptr).inner }).terms.get(index) else {
        return -2;
    };
    let copied = Box::into_raw(Box::new(CPauliString {
        inner: pauli.clone(),
    }));
    unsafe {
        *out_pauli = copied;
        *out_coefficient = *coefficient;
    }
    0
}
