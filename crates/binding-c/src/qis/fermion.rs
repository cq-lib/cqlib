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

//! Checked fermionic algebra. All outputs and mutable inputs are unchanged on
//! failure. Object outputs are owned copies freed with `fermionic_op_free`;
//! inputs are borrowed. Multiplication applies the right operand first.

use super::CHamiltonian;
use super::algebra::{algebra_err_code, check_pauli_width, input_slice, write_result};
use cqlib_core::qis::fermion::mapping::{
    FermionToQubitMapper, JordanWignerMapper, OccupationStateMapper,
};
use cqlib_core::qis::{FermionicLadder, FermionicOp};
use libc::size_t;
use num_complex::Complex64;

/// Opaque owned fermionic operator. No internal storage is exposed to C.
/// All checked functions return 0 on success and leave outputs unchanged on error.
/// Pass empty object output slots; results must be freed with the matching free
/// function. Caller storage must be live, aligned and large enough; output slots
/// must not overlap one another or handle internals. Inputs are borrowed.
/// Do not read a handle concurrently with mutation. Empty arrays permit NULL.
pub struct CFermionicOp {
    pub inner: FermionicOp,
}

/// Creation ladder tag.
pub const FERMIONIC_CREATE: u8 = 0;
/// Annihilation ladder tag.
pub const FERMIONIC_ANNIHILATE: u8 = 1;

/// One ladder on a zero-based mode. Unknown kind tags return -8.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CFermionicLadder {
    pub mode: size_t,
    pub kind: u8,
}

/// Borrowed ordered product and coefficient; the rightmost ladder acts first.
/// A NULL ladders pointer with len=0 denotes the identity product.
#[repr(C)]
pub struct CFermionicTerm {
    pub ladders: *const CFermionicLadder,
    pub len: size_t,
    pub coefficient: Complex64,
}

unsafe fn ladders(ptr: *const CFermionicLadder, len: size_t) -> Result<Vec<FermionicLadder>, i32> {
    unsafe { input_slice(ptr, len) }?
        .iter()
        .map(|ladder| match ladder.kind {
            FERMIONIC_CREATE => Ok(FermionicLadder::Create(ladder.mode)),
            FERMIONIC_ANNIHILATE => Ok(FermionicLadder::Annihilate(ladder.mode)),
            _ => Err(-8),
        })
        .collect()
}

fn owned(inner: FermionicOp) -> *mut CFermionicOp {
    Box::into_raw(Box::new(CFermionicOp { inner }))
}

/// Release an owned operator. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_free(ptr: *mut CFermionicOp) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Construct zero, retaining the declared width including zero modes.
/// Returns -1 for NULL out; writes an owned handle only on success.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_new(num_modes: size_t, out: *mut *mut CFermionicOp) -> i32 {
    if out.is_null() {
        return -1;
    }
    unsafe {
        *out = owned(FermionicOp::new(num_modes));
    }
    0
}

/// Construct identity, retaining the declared width including zero modes.
/// Returns -1 for NULL out; writes an owned handle only on success.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_identity(num_modes: size_t, out: *mut *mut CFermionicOp) -> i32 {
    if out.is_null() {
        return -1;
    }
    unsafe {
        *out = owned(FermionicOp::identity(num_modes));
    }
    0
}

/// Construct total particle number, retaining the declared width including zero modes.
/// Returns -1 for NULL out; writes an owned handle only on success.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_total_number(num_modes: size_t, out: *mut *mut CFermionicOp) -> i32 {
    if out.is_null() {
        return -1;
    }
    unsafe {
        *out = owned(FermionicOp::total_number(num_modes));
    }
    0
}

/// Construct a mode creation operator. Returns -1 for NULL out, -2 for invalid mode.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_creation(
    num_modes: size_t,
    mode: size_t,
    out: *mut *mut CFermionicOp,
) -> i32 {
    if out.is_null() {
        return -1;
    }
    unsafe { write_result(out, FermionicOp::creation(num_modes, mode).map(owned)) }
}

/// Construct a mode annihilation operator. Returns -1 for NULL out, -2 for invalid mode.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_annihilation(
    num_modes: size_t,
    mode: size_t,
    out: *mut *mut CFermionicOp,
) -> i32 {
    if out.is_null() {
        return -1;
    }
    unsafe { write_result(out, FermionicOp::annihilation(num_modes, mode).map(owned)) }
}

/// Construct a mode number operator. Returns -1 for NULL out, -2 for invalid mode.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_number(
    num_modes: size_t,
    mode: size_t,
    out: *mut *mut CFermionicOp,
) -> i32 {
    if out.is_null() {
        return -1;
    }
    unsafe { write_result(out, FermionicOp::number(num_modes, mode).map(owned)) }
}

/// Construct a weighted ordered product. Empty product is identity; even
/// zero coefficients validate all modes. Returns -1 NULL, -2 mode, -8 tag/coefficient.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_from_term(
    num_modes: size_t,
    term: *const CFermionicLadder,
    len: size_t,
    coefficient: Complex64,
    out: *mut *mut CFermionicOp,
) -> i32 {
    if out.is_null() {
        return -1;
    }
    let term = match unsafe { ladders(term, len) } {
        Ok(v) => v,
        Err(e) => return e,
    };
    unsafe {
        write_result(
            out,
            FermionicOp::from_term(num_modes, term, coefficient).map(owned),
        )
    }
}

/// Construct a sum of borrowed terms, merging identical products and removing
/// only exact zeros. NULL terms with len=0 creates zero. No input is consumed.
/// Returns -1 NULL, -2 invalid mode, -8 invalid tag/coefficient or length.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_from_terms(
    num_modes: size_t,
    terms: *const CFermionicTerm,
    len: size_t,
    out: *mut *mut CFermionicOp,
) -> i32 {
    if out.is_null() {
        return -1;
    }
    let terms = match unsafe { input_slice(terms, len) } {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mut result = FermionicOp::new(num_modes);
    for term in terms {
        let product = match unsafe { ladders(term.ladders, term.len) } {
            Ok(v) => v,
            Err(e) => return e,
        };
        if let Err(err) = result.add_term(product, term.coefficient) {
            return algebra_err_code(&err);
        }
    }
    unsafe {
        *out = owned(result);
    }
    0
}

/// Add an ordered term atomically. Inputs are copied. Empty product is identity.
/// Returns -1 NULL, -2 mode, -8 tag/nonfinite coefficient (including merge overflow).
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_add_term(
    ptr: *mut CFermionicOp,
    term: *const CFermionicLadder,
    len: size_t,
    coefficient: Complex64,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let term = match unsafe { ladders(term, len) } {
        Ok(v) => v,
        Err(e) => return e,
    };
    match unsafe { &mut (*ptr).inner }.add_term(term, coefficient) {
        Ok(()) => 0,
        Err(err) => algebra_err_code(&err),
    }
}

/// Construct G=i(T-T†). Target creators retain input order; source annihilators
/// use reverse input order. No sorting or factorial. Returns -1 NULL, -2 mode,
/// -8 empty/unequal lists, duplicates or overlap. Inputs are borrowed.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_excitation_generator(
    num_modes: size_t,
    sources: *const size_t,
    sources_len: size_t,
    targets: *const size_t,
    targets_len: size_t,
    out: *mut *mut CFermionicOp,
) -> i32 {
    if out.is_null() {
        return -1;
    }
    let sources = match unsafe { input_slice(sources, sources_len) } {
        Ok(v) => v,
        Err(e) => return e,
    };
    let targets = match unsafe { input_slice(targets, targets_len) } {
        Ok(v) => v,
        Err(e) => return e,
    };
    unsafe {
        write_result(
            out,
            FermionicOp::excitation_generator(num_modes, sources, targets).map(owned),
        )
    }
}

/// Read declared mode count. Returns -1 for NULL input or output.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_num_modes(ptr: *const CFermionicOp, out: *mut size_t) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let inner = unsafe { &(*ptr).inner };
    unsafe {
        *out = inner.num_modes();
    }
    0
}

/// Read stored product count. Returns -1 for NULL input or output.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_num_terms(ptr: *const CFermionicOp, out: *mut size_t) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let inner = unsafe { &(*ptr).inner };
    unsafe {
        *out = inner.num_terms();
    }
    0
}

/// Read structural zero (0/1); use equiv for algebraic zero. Returns -1 for NULL input or output.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_is_zero(ptr: *const CFermionicOp, out: *mut u8) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let inner = unsafe { &(*ptr).inner };
    unsafe {
        *out = u8::from(inner.is_zero());
    }
    0
}

/// Return an owned independent copy.
/// Returns -1 for NULL input or output.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_copy(ptr: *const CFermionicOp, out: *mut *mut CFermionicOp) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let inner = unsafe { &(*ptr).inner };
    unsafe { write_result(out, Ok(owned(inner.clone()))) }
}

/// Return an owned conjugate transpose, reversing products and exchanging ladder kinds.
/// Returns -1 for NULL input or output.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_adjoint(
    ptr: *const CFermionicOp,
    out: *mut *mut CFermionicOp,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let inner = unsafe { &(*ptr).inner };
    unsafe { write_result(out, Ok(owned(inner.adjoint()))) }
}

/// Return an owned normal-ordered expansion with contraction terms and fermionic signs.
/// Returns -1 NULL; normal ordering may return -8 on coefficient overflow.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_normal_order(
    ptr: *const CFermionicOp,
    out: *mut *mut CFermionicOp,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let inner = unsafe { &(*ptr).inner };
    unsafe { write_result(out, inner.normal_order().map(owned)) }
}

/// Checked add, returning an independent operator without normal ordering.
/// Returns -1 NULL, -8 mode mismatch/nonfinite arithmetic. Right operand acts first in mul.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_add(
    left: *const CFermionicOp,
    right: *const CFermionicOp,
    out: *mut *mut CFermionicOp,
) -> i32 {
    if left.is_null() || right.is_null() || out.is_null() {
        return -1;
    }
    unsafe { write_result(out, (*left).inner.try_add(&(*right).inner).map(owned)) }
}

/// Checked sub, returning an independent operator without normal ordering.
/// Returns -1 NULL, -8 mode mismatch/nonfinite arithmetic. Right operand acts first in mul.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_sub(
    left: *const CFermionicOp,
    right: *const CFermionicOp,
    out: *mut *mut CFermionicOp,
) -> i32 {
    if left.is_null() || right.is_null() || out.is_null() {
        return -1;
    }
    unsafe { write_result(out, (*left).inner.try_sub(&(*right).inner).map(owned)) }
}

/// Checked mul, returning an independent operator without normal ordering.
/// Returns -1 NULL, -8 mode mismatch/nonfinite arithmetic. Right operand acts first in mul.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_mul(
    left: *const CFermionicOp,
    right: *const CFermionicOp,
    out: *mut *mut CFermionicOp,
) -> i32 {
    if left.is_null() || right.is_null() || out.is_null() {
        return -1;
    }
    unsafe { write_result(out, (*left).inner.try_mul(&(*right).inner).map(owned)) }
}

/// Return a scaled copy. Returns -1 NULL, -8 nonfinite factor or result.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_scaled(
    ptr: *const CFermionicOp,
    factor: Complex64,
    out: *mut *mut CFermionicOp,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    unsafe { write_result(out, (*ptr).inner.scaled(factor).map(owned)) }
}

/// Remove coefficients with magnitude <= atol in place. Returns -1 NULL,
/// -8 for negative/nonfinite atol. Failure leaves the operator unchanged.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_chop(ptr: *mut CFermionicOp, atol: f64) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match unsafe { &mut (*ptr).inner }.chop(atol) {
        Ok(()) => 0,
        Err(e) => algebra_err_code(&e),
    }
}

/// Structural equality including mode count; writes 0 or 1. Returns -1 NULL.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_equals(
    left: *const CFermionicOp,
    right: *const CFermionicOp,
    out: *mut u8,
) -> i32 {
    if left.is_null() || right.is_null() || out.is_null() {
        return -1;
    }
    unsafe {
        *out = u8::from((*left).inner == (*right).inner);
    }
    0
}

/// Algebraic equivalence after normal ordering; writes 0 or 1. Returns -1 NULL,
/// -8 mode mismatch, invalid atol or arithmetic overflow. atol=0 is exact.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_equiv(
    left: *const CFermionicOp,
    right: *const CFermionicOp,
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

/// Algebraic Hermiticity test; writes 0 or 1. Returns -1 NULL, -8 invalid atol/overflow.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_is_hermitian(
    ptr: *const CFermionicOp,
    atol: f64,
    out: *mut u8,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    unsafe { write_result(out, (*ptr).inner.is_hermitian(atol).map(u8::from)) }
}

/// Read one stored product's length in deterministic structural order.
/// Returns -1 NULL, -2 invalid term index. No internal storage is borrowed.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_term_len(
    ptr: *const CFermionicOp,
    index: size_t,
    out: *mut size_t,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let Some((term, _)) = (unsafe { &(*ptr).inner }).terms().nth(index) else {
        return -2;
    };
    unsafe {
        *out = term.len();
    }
    0
}

/// Copy a term's ladders and coefficient in deterministic structural order.
/// Returns -1 NULL required pointer, -2 index, -8 insufficient capacity.
/// NULL buffer is allowed for an empty product. Writes no partial output on error.
#[unsafe(no_mangle)]
pub extern "C" fn fermionic_op_get_term(
    ptr: *const CFermionicOp,
    index: size_t,
    buffer: *mut CFermionicLadder,
    capacity: size_t,
    out_coefficient: *mut Complex64,
) -> i32 {
    if ptr.is_null() || out_coefficient.is_null() {
        return -1;
    }
    let Some((term, coefficient)) = (unsafe { &(*ptr).inner }).terms().nth(index) else {
        return -2;
    };
    if capacity < term.len() {
        return -8;
    }
    if !term.is_empty() && buffer.is_null() {
        return -1;
    }
    for (i, ladder) in term.iter().enumerate() {
        let kind = match ladder {
            FermionicLadder::Create(_) => FERMIONIC_CREATE,
            FermionicLadder::Annihilate(_) => FERMIONIC_ANNIHILATE,
        };
        unsafe {
            *buffer.add(i) = CFermionicLadder {
                mode: ladder.mode(),
                kind,
            };
        }
    }
    unsafe {
        *out_coefficient = coefficient;
    }
    0
}

/// Map a borrowed operator to an owned CHamiltonian; free with hamiltonian_free.
/// Non-Hermitian inputs are valid. Mode p maps to qubit p, with Z on lower modes.
/// Returns -1 NULL, -7 unrepresentable width, -8 nonfinite arithmetic.
#[unsafe(no_mangle)]
pub extern "C" fn jordan_wigner_map_operator(
    ptr: *const CFermionicOp,
    out: *mut *mut CHamiltonian,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let inner = unsafe { &(*ptr).inner };
    // Zero operators need no bit vectors. All other inputs materialize a Pauli
    // identity before mapping; guard that allocation like checked core algebra.
    if !inner.is_zero()
        && let Err(e) = check_pauli_width(inner.num_modes())
    {
        return e;
    }
    unsafe {
        write_result(
            out,
            JordanWignerMapper
                .map_operator(inner)
                .map(|inner| Box::into_raw(Box::new(CHamiltonian { inner }))),
        )
    }
}

/// Jordan-Wigner uses one qubit per mode. Returns -1 for NULL out.
#[unsafe(no_mangle)]
pub extern "C" fn jordan_wigner_num_qubits(num_modes: size_t, out: *mut size_t) -> i32 {
    if out.is_null() {
        return -1;
    }
    unsafe {
        *out = JordanWignerMapper.num_qubits(num_modes);
    }
    0
}

/// Encode occupations as 0/1 bytes: bits[p] addresses qubit p (LSB is qubit 0).
/// Empty input is vacuum. Returns -1 NULL required buffer, -8 invalid occupations,
/// length or insufficient capacity. Buffers may overlap: input is read before output.
#[unsafe(no_mangle)]
pub extern "C" fn jordan_wigner_map_occupation(
    occupation: *const u8,
    len: size_t,
    out_bits: *mut u8,
    capacity: size_t,
) -> i32 {
    if len > 0 && (occupation.is_null() || out_bits.is_null()) {
        return -1;
    }
    if capacity < len {
        return -8;
    }
    let occupation = match unsafe { input_slice(occupation, len) } {
        Ok(v) => v,
        Err(e) => return e,
    };
    let bits = match JordanWignerMapper.map_occupation(occupation) {
        Ok(v) => v,
        Err(e) => return algebra_err_code(&e),
    };
    for (i, bit) in bits.into_iter().enumerate() {
        unsafe {
            *out_bits.add(i) = u8::from(bit);
        }
    }
    0
}
