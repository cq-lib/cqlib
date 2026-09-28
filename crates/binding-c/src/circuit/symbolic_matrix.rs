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

//! C ABI for symbolic matrices and global-phase equivalence checks.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::circuit::{CCircuit, CParameter, binding_refs, parse_bindings};
use crate::device::standard_gate_from_name;
use cqlib_core::circuit::Parameter;
use cqlib_core::circuit::symbolic_matrix::gate::{
    apply_gate_to_matrix as core_apply_gate_to_matrix,
    apply_gate_to_matrix_num as core_apply_gate_to_matrix_num,
    apply_general_gate as core_apply_general_gate,
    apply_general_gate_num as core_apply_general_gate_num,
    apply_single_qubit_gate as core_apply_single_qubit_gate,
    apply_single_qubit_gate_num as core_apply_single_qubit_gate_num,
    apply_standard_gate_to_matrix as core_apply_standard_gate_to_matrix,
    apply_two_qubit_gate as core_apply_two_qubit_gate,
    apply_two_qubit_gate_num as core_apply_two_qubit_gate_num, control_matrix,
};
use cqlib_core::circuit::symbolic_matrix::matrix::{
    apply_numeric_diagonal_gate as core_apply_numeric_diagonal_gate,
    apply_numeric_permutation_gate as core_apply_numeric_permutation_gate,
    apply_symbolic_diagonal_gate as core_apply_symbolic_diagonal_gate,
    apply_symbolic_permutation_gate as core_apply_symbolic_permutation_gate, simplify_matrix,
};
use cqlib_core::circuit::symbolic_matrix::{
    SymbolicComplex, SymbolicMatrix, circuit_to_symbolic_matrix as core_circuit_to_symbolic_matrix,
    circuits_equivalent as core_circuits_equivalent, evaluate_symbolic_matrix,
    standard_gate_symbolic_matrix as core_standard_gate_symbolic_matrix,
    substitute_symbolic_matrix, symbolic_eye as core_symbolic_eye,
    symbolic_matrices_equivalent as core_symbolic_matrices_equivalent,
};
use ndarray::Array2;
use num_complex::Complex64;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Opaque handle around a symbolic matrix ([`SymbolicMatrix`]).
pub struct CSymbolicMatrix {
    pub inner: SymbolicMatrix,
}

/// Creates a `dim x dim` symbolic identity matrix. Returns NULL only on
/// allocation failure.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_eye(dim: usize) -> *mut CSymbolicMatrix {
    Box::into_raw(Box::new(CSymbolicMatrix {
        inner: core_symbolic_eye(dim),
    }))
}

/// Frees a symbolic matrix handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_free(ptr: *mut CSymbolicMatrix) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Returns the row count, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_rows(ptr: *const CSymbolicMatrix) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.nrows() }
}

/// Returns the column count, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_cols(ptr: *const CSymbolicMatrix) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.ncols() }
}

/// Formats one matrix element as a human-readable parameter expression
/// (e.g. `"cos(theta*0.5)"`). Returns a newly allocated C string (free with
/// `cqlib_string_free`), or NULL on NULL input or out-of-bounds indices.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_element_str(
    ptr: *const CSymbolicMatrix,
    row: usize,
    col: usize,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let inner = unsafe { &(*ptr).inner };
    match inner.get([row, col]) {
        Some(value) => match CString::new(value.to_string()) {
            Ok(text) => text.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Simplifies every element of the matrix. Returns a newly allocated
/// `CSymbolicMatrix*` (free with `symbolic_matrix_free`), or NULL on NULL
/// input or simplification failure.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_simplify(ptr: *const CSymbolicMatrix) -> *mut CSymbolicMatrix {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match simplify_matrix(unsafe { &(*ptr).inner }) {
        Ok(inner) => Box::into_raw(Box::new(CSymbolicMatrix { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns the flattened element count (`rows * cols`), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_evaluate_len(ptr: *const CSymbolicMatrix) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let inner = unsafe { &(*ptr).inner };
    inner.nrows().saturating_mul(inner.ncols())
}

/// Evaluates every element under parameter bindings into `buffer` in
/// row-major order. `bindings` is a `"name:value,name:value"` string (NULL
/// means no bindings). `len` must be at least `symbolic_matrix_evaluate_len`.
/// Returns 0 on success, -1 on NULL input, -3 on unevaluable symbols,
/// -4 on a malformed bindings string, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_evaluate(
    ptr: *const CSymbolicMatrix,
    bindings: *const c_char,
    buffer: *mut Complex64,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let mut bindings_owned = HashMap::new();
    let has_bindings = !bindings.is_null();
    if has_bindings {
        match parse_bindings(bindings) {
            Some(map) => bindings_owned = map,
            None => return -4,
        }
    }
    let bindings_map = if has_bindings {
        Some(binding_refs(&bindings_owned))
    } else {
        None
    };
    match evaluate_symbolic_matrix(unsafe { &(*ptr).inner }, &bindings_map) {
        Ok(out) => {
            if buffer.is_null() {
                return -1;
            }
            let count = out.len();
            if len < count {
                return -8;
            }
            let slice = unsafe { std::slice::from_raw_parts_mut(buffer, count) };
            for (slot, value) in slice.iter_mut().zip(out.iter()) {
                *slot = *value;
            }
            0
        }
        Err(_) => -3,
    }
}

/// Simultaneously substitutes symbols in the matrix. `names[i]` is a symbol
/// name and `params[i]` the replacement expression. Returns a newly allocated
/// `CSymbolicMatrix*` (free with `symbolic_matrix_free`), or NULL on NULL
/// input, invalid UTF-8 or a substitution error (e.g. reserved prefix).
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_substitute(
    ptr: *const CSymbolicMatrix,
    names: *const *const c_char,
    params: *const *const CParameter,
    len: usize,
) -> *mut CSymbolicMatrix {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let mut replacements: HashMap<String, Parameter> = HashMap::new();
    if len > 0 {
        if names.is_null() || params.is_null() {
            return std::ptr::null_mut();
        }
        let names = unsafe { std::slice::from_raw_parts(names, len) };
        let params = unsafe { std::slice::from_raw_parts(params, len) };
        for (name, param) in names.iter().zip(params.iter()) {
            if name.is_null() || param.is_null() {
                return std::ptr::null_mut();
            }
            let name = match unsafe { CStr::from_ptr(*name) }.to_str() {
                Ok(name) => name,
                Err(_) => return std::ptr::null_mut(),
            };
            let param = unsafe { (*(*param)).inner.clone() };
            replacements.insert(name.to_string(), param);
        }
    }
    let matrix = unsafe { (*ptr).inner.clone() };
    match substitute_symbolic_matrix(matrix, &replacements) {
        Ok(inner) => Box::into_raw(Box::new(CSymbolicMatrix { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Builds the symbolic unitary matrix of a standard gate selected by name
/// (e.g. `"RX"`, `"RZZ"`; same name set as `circuit_multi_control`).
/// Returns a newly allocated `CSymbolicMatrix*`, or NULL on NULL input,
/// invalid UTF-8, an unknown gate name or a parameter-count mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn standard_gate_symbolic_matrix(
    gate_name: *const c_char,
    params: *const *const CParameter,
    len: usize,
) -> *mut CSymbolicMatrix {
    if gate_name.is_null() {
        return std::ptr::null_mut();
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(name) => name,
        Err(_) => return std::ptr::null_mut(),
    };
    let gate = match standard_gate_from_name(name) {
        Some(gate) => gate,
        None => return std::ptr::null_mut(),
    };
    let mut values: Vec<Parameter> = Vec::with_capacity(len);
    if len > 0 {
        if params.is_null() {
            return std::ptr::null_mut();
        }
        let params = unsafe { std::slice::from_raw_parts(params, len) };
        for param in params {
            if param.is_null() {
                return std::ptr::null_mut();
            }
            values.push(unsafe { (*(*param)).inner.clone() });
        }
    }
    match core_standard_gate_symbolic_matrix(gate, &values) {
        Ok(inner) => Box::into_raw(Box::new(CSymbolicMatrix { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Computes the symbolic unitary matrix of a circuit. `order` optionally
/// lists the qubit ids from least-significant to most-significant bit (NULL
/// sorts by qubit index). Returns a newly allocated `CSymbolicMatrix*`, or
/// NULL on NULL input, an order/qubit-set mismatch or non-unitary operations.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_to_symbolic_matrix(
    ptr: *const CCircuit,
    order: *const u32,
    order_len: usize,
) -> *mut CSymbolicMatrix {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let order = match read_order(order, order_len) {
        Ok(order) => order,
        Err(_) => return std::ptr::null_mut(),
    };
    match core_circuit_to_symbolic_matrix(unsafe { &(*ptr).inner }, order.as_deref()) {
        Ok(inner) => Box::into_raw(Box::new(CSymbolicMatrix { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns whether two symbolic matrices are equivalent up to a global
/// phase: 1 when equivalent, 0 when not, -1 on NULL input, -3 on
/// simplification failure.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrices_equivalent(
    lhs: *const CSymbolicMatrix,
    rhs: *const CSymbolicMatrix,
) -> i32 {
    if lhs.is_null() || rhs.is_null() {
        return -1;
    }
    match core_symbolic_matrices_equivalent(unsafe { &(*lhs).inner }, unsafe { &(*rhs).inner }) {
        Ok(equivalent) => equivalent as i32,
        Err(_) => -3,
    }
}

/// Compares two circuits up to a global phase in the requested qubit order.
/// Returns 1 when equivalent, 0 when not, -1 on NULL input, -3 on matrix
/// construction failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuits_equivalent(
    lhs: *const CCircuit,
    rhs: *const CCircuit,
    order: *const u32,
    order_len: usize,
) -> i32 {
    if lhs.is_null() || rhs.is_null() {
        return -1;
    }
    let order = match read_order(order, order_len) {
        Ok(order) => order,
        Err(_) => return -8,
    };
    match core_circuits_equivalent(
        unsafe { &(*lhs).inner },
        unsafe { &(*rhs).inner },
        order.as_deref(),
    ) {
        Ok(equivalent) => equivalent as i32,
        Err(_) => -3,
    }
}

/// Embeds `base` into the bottom-right block of a larger identity matrix
/// for `num_ctrls` control qubits. Returns a newly allocated
/// `CSymbolicMatrix*`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_controlled(
    base: *const CSymbolicMatrix,
    num_ctrls: usize,
) -> *mut CSymbolicMatrix {
    if base.is_null() {
        return std::ptr::null_mut();
    }
    let inner = control_matrix(unsafe { &(*base).inner }, num_ctrls);
    Box::into_raw(Box::new(CSymbolicMatrix { inner }))
}

// ---------------------------------------------------------------------------
// Symbolic complex values
// ---------------------------------------------------------------------------

/// Opaque handle around a symbolic complex value ([`SymbolicComplex`]): both
/// the real and the imaginary part are parameter expressions.
pub struct CSymbolicComplex {
    pub inner: SymbolicComplex,
}

fn box_complex(value: SymbolicComplex) -> *mut CSymbolicComplex {
    Box::into_raw(Box::new(CSymbolicComplex { inner: value }))
}

fn box_param_clone(param: Parameter) -> *mut CParameter {
    Box::into_raw(Box::new(CParameter { inner: param }))
}

/// Creates a symbolic complex value from real and imaginary expressions.
/// Returns NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_new(
    re: *const CParameter,
    im: *const CParameter,
) -> *mut CSymbolicComplex {
    if re.is_null() || im.is_null() {
        return std::ptr::null_mut();
    }
    let re = unsafe { (*re).inner.clone() };
    let im = unsafe { (*im).inner.clone() };
    box_complex(SymbolicComplex::new(re, im))
}

/// Returns the additive identity `0 + 0i`.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_zero() -> *mut CSymbolicComplex {
    box_complex(SymbolicComplex::zero())
}

/// Returns the multiplicative identity `1 + 0i`.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_one() -> *mut CSymbolicComplex {
    box_complex(SymbolicComplex::one())
}

/// Returns the imaginary unit `0 + 1i`.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_i() -> *mut CSymbolicComplex {
    box_complex(SymbolicComplex::i())
}

/// Creates a symbolic complex value with zero imaginary part from the given
/// real expression. Returns NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_from_real(re: *const CParameter) -> *mut CSymbolicComplex {
    if re.is_null() {
        return std::ptr::null_mut();
    }
    box_complex(SymbolicComplex::from_real(unsafe { (*re).inner.clone() }))
}

/// Creates a symbolic complex constant from a raw `Complex64` pair. Returns
/// NULL when either component is not finite.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_from_complex(re: f64, im: f64) -> *mut CSymbolicComplex {
    if !re.is_finite() || !im.is_finite() {
        return std::ptr::null_mut();
    }
    box_complex(SymbolicComplex::from_complex(Complex64::new(re, im)))
}

/// Builds `exp(i*theta)`, i.e. `cos(theta) + i*sin(theta)` (free via
/// `symbolic_complex_free`). Returns NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_exp_i(theta: *const CParameter) -> *mut CSymbolicComplex {
    if theta.is_null() {
        return std::ptr::null_mut();
    }
    box_complex(SymbolicComplex::exp_i(unsafe { (*theta).inner.clone() }))
}

/// Frees a symbolic complex handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_free(ptr: *mut CSymbolicComplex) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Returns a clone of the real part (free with `param_free`); NULL on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_re(ptr: *const CSymbolicComplex) -> *mut CParameter {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    box_param_clone(unsafe { (*ptr).inner.re.clone() })
}

/// Returns a clone of the imaginary part (free with `param_free`); NULL on
/// NULL.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_im(ptr: *const CSymbolicComplex) -> *mut CParameter {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    box_param_clone(unsafe { (*ptr).inner.im.clone() })
}

/// Evaluates both parts under `bindings` (`"name:value,..."`; NULL means no
/// bindings) and stores the `Complex64` result in `out`. Returns 0 on
/// success, -1 on NULL input, -3 on unevaluable symbols, -4 on a malformed
/// bindings string.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_evaluate(
    ptr: *const CSymbolicComplex,
    bindings: *const c_char,
    out: *mut Complex64,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let mut bindings_owned = HashMap::new();
    let has_bindings = !bindings.is_null();
    if has_bindings {
        match parse_bindings(bindings) {
            Some(map) => bindings_owned = map,
            None => return -4,
        }
    }
    let bindings_map = if has_bindings {
        Some(binding_refs(&bindings_owned))
    } else {
        None
    };
    match unsafe { (*ptr).inner.evaluate(&bindings_map) } {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(_) => -3,
    }
}

/// Simplifies both parts (free with `symbolic_complex_free`). Returns NULL
/// on NULL input or simplification failure.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_simplify(ptr: *const CSymbolicComplex) -> *mut CSymbolicComplex {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*ptr).inner.simplify() } {
        Ok(inner) => box_complex(inner),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Replaces every occurrence of `symbol` in both parts with `replacement`
/// (free with `symbolic_complex_free`). Returns NULL on NULL input or a
/// non-UTF-8 symbol name.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_replace(
    ptr: *const CSymbolicComplex,
    symbol: *const c_char,
    replacement: *const CParameter,
) -> *mut CSymbolicComplex {
    if ptr.is_null() || symbol.is_null() || replacement.is_null() {
        return std::ptr::null_mut();
    }
    let symbol = match unsafe { CStr::from_ptr(symbol) }.to_str() {
        Ok(name) => name,
        Err(_) => return std::ptr::null_mut(),
    };
    let value = unsafe { (*replacement).inner.clone() };
    box_complex(unsafe { (*ptr).inner.replace(symbol, value) })
}

/// Returns whether both parts are exactly zero: 1/0, or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_is_zero_exact(ptr: *const CSymbolicComplex) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.is_zero_exact() as i32 }
}

/// Returns whether the real part is one and the imaginary part zero:
/// 1/0, or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_is_one_exact(ptr: *const CSymbolicComplex) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.is_one_exact() as i32 }
}

/// Returns whether this value simplifies to exactly zero: 1 when it does,
/// 0 when not, -1 on NULL, -3 on simplification failure.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_complex_simplifies_to_zero(ptr: *const CSymbolicComplex) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match unsafe { (*ptr).inner.simplifies_to_zero() } {
        Ok(value) => value as i32,
        Err(_) => -3,
    }
}

// ---------------------------------------------------------------------------
// Matrix element access and construction
// ---------------------------------------------------------------------------

/// Builds a `rows x cols` symbolic matrix from `rows * cols` element handles
/// given in row-major order (free with `symbolic_matrix_free`). Returns NULL
/// on NULL elements, a length/shape mismatch or a zero dimension.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_new(
    rows: usize,
    cols: usize,
    elements: *const *const CSymbolicComplex,
    len: usize,
) -> *mut CSymbolicMatrix {
    if rows == 0 || cols == 0 || elements.is_null() || rows.checked_mul(cols) != Some(len) {
        return std::ptr::null_mut();
    }
    let handles = unsafe { std::slice::from_raw_parts(elements, len) };
    let mut values = Vec::with_capacity(len);
    for handle in handles {
        if handle.is_null() {
            return std::ptr::null_mut();
        }
        values.push(unsafe { (**handle).inner.clone() });
    }
    match SymbolicMatrix::from_shape_vec((rows, cols), values) {
        Ok(inner) => Box::into_raw(Box::new(CSymbolicMatrix { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns a clone of the `[row, col]` element (free with
/// `symbolic_complex_free`); NULL on NULL input or out-of-bounds indices.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_element(
    ptr: *const CSymbolicMatrix,
    row: usize,
    col: usize,
) -> *mut CSymbolicComplex {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*ptr).inner.get([row, col]) } {
        Some(value) => box_complex(value.clone()),
        None => std::ptr::null_mut(),
    }
}

// ---------------------------------------------------------------------------
// In-place gate application
// ---------------------------------------------------------------------------

/// Reads the target-bit array. `Err(())` marks a NULL/empty buffer, a bit
/// outside the matrix dimension (rows must be a power of two) or duplicate
/// bits.
fn read_target_bits(bits: *const usize, len: usize, dim: usize) -> Result<Vec<usize>, ()> {
    if len == 0 || bits.is_null() || !dim.is_power_of_two() {
        return Err(());
    }
    let slice = unsafe { std::slice::from_raw_parts(bits, len) };
    for (pos, &bit) in slice.iter().enumerate() {
        if bit >= dim.trailing_zeros() as usize || slice[..pos].contains(&bit) {
            return Err(());
        }
    }
    Ok(slice.to_vec())
}

/// Builds a square `gate_dim x gate_dim` numeric matrix from `gate_dim *
/// gate_dim` row-major `Complex64` values. Returns None on NULL input.
fn numeric_gate_from_raw(gate: *const Complex64, gate_dim: usize) -> Option<Array2<Complex64>> {
    if gate.is_null() || gate_dim == 0 {
        return None;
    }
    let count = gate_dim.checked_mul(gate_dim)?;
    let values = unsafe { std::slice::from_raw_parts(gate, count) }.to_vec();
    Array2::from_shape_vec((gate_dim, gate_dim), values).ok()
}

/// Applies `1 << bits_len` as a checked power of two; 0 marks an overflow.
fn gate_dim_for(bits_len: usize) -> usize {
    (1usize).checked_shl(bits_len as u32).unwrap_or(0)
}

/// Applies a named standard gate (same name set as `circuit_multi_control`)
/// with symbolic parameters to the target bits, mutating the matrix in
/// place. Returns 0 on success, -1 on NULL input, -3 on a gate/matrix
/// mismatch, -4 on a non-UTF-8 gate name, -8 on an unknown gate name or
/// invalid bits.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_standard_gate(
    ptr: *mut CSymbolicMatrix,
    gate_name: *const c_char,
    bits: *const usize,
    bits_len: usize,
    params: *const *const CParameter,
    params_len: usize,
) -> i32 {
    if ptr.is_null() || gate_name.is_null() {
        return -1;
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(name) => name,
        Err(_) => return -4,
    };
    let gate = match standard_gate_from_name(name) {
        Some(gate) => gate,
        None => return -8,
    };
    let bits = match read_target_bits(bits, bits_len, unsafe { (*ptr).inner.nrows() }) {
        Ok(bits) => bits,
        Err(()) => return -8,
    };
    let mut values: Vec<Parameter> = Vec::with_capacity(params_len);
    if params_len > 0 {
        if params.is_null() {
            return -1;
        }
        let handles = unsafe { std::slice::from_raw_parts(params, params_len) };
        for handle in handles {
            if handle.is_null() {
                return -1;
            }
            values.push(unsafe { (**handle).inner.clone() });
        }
    }
    match core_apply_standard_gate_to_matrix(unsafe { &mut (*ptr).inner }, gate, &bits, &values) {
        Ok(()) => 0,
        Err(_) => -3,
    }
}

/// Applies a symbolic gate matrix (dimension `1 << bits_len`) to the target
/// bits, mutating the matrix in place. Returns 0 on success, -1 on NULL,
/// -3 on a gate/matrix mismatch, -8 on invalid bits.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_gate(
    ptr: *mut CSymbolicMatrix,
    gate: *const CSymbolicMatrix,
    bits: *const usize,
    bits_len: usize,
) -> i32 {
    if ptr.is_null() || gate.is_null() {
        return -1;
    }
    let bits = match read_target_bits(bits, bits_len, unsafe { (*ptr).inner.nrows() }) {
        Ok(bits) => bits,
        Err(()) => return -8,
    };
    match core_apply_gate_to_matrix(
        unsafe { &mut (*ptr).inner },
        unsafe { &(*gate).inner },
        &bits,
    ) {
        Ok(()) => 0,
        Err(_) => -3,
    }
}

/// Numeric variant of `symbolic_matrix_apply_gate`: `gate` points at
/// `gate_dim * gate_dim` row-major `Complex64` values and `gate_dim` must
/// equal `1 << bits_len`.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_gate_num(
    ptr: *mut CSymbolicMatrix,
    gate: *const Complex64,
    gate_dim: usize,
    bits: *const usize,
    bits_len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let bits = match read_target_bits(bits, bits_len, unsafe { (*ptr).inner.nrows() }) {
        Ok(bits) => bits,
        Err(()) => return -8,
    };
    if gate_dim == 0 || gate_dim != gate_dim_for(bits_len) {
        return -8;
    }
    let matrix = match numeric_gate_from_raw(gate, gate_dim) {
        Some(matrix) => matrix,
        None => return -1,
    };
    match core_apply_gate_to_matrix_num(unsafe { &mut (*ptr).inner }, &matrix, &bits) {
        Ok(()) => 0,
        Err(_) => -3,
    }
}

/// Applies a 2x2 symbolic gate to the target bit in place. Returns 0 on
/// success, -1 on NULL, -8 when the gate is not 2x2 or the bit is invalid.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_single_qubit_gate(
    ptr: *mut CSymbolicMatrix,
    gate: *const CSymbolicMatrix,
    bit: usize,
) -> i32 {
    if ptr.is_null() || gate.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    let target = unsafe { &(*gate).inner };
    if target.nrows() != 2
        || target.ncols() != 2
        || !inner.nrows().is_power_of_two()
        || bit >= inner.nrows().trailing_zeros() as usize
    {
        return -8;
    }
    core_apply_single_qubit_gate(inner, target, bit);
    0
}

/// Numeric variant of `symbolic_matrix_apply_single_qubit_gate`: `gate`
/// points at 4 row-major `Complex64` values.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_single_qubit_gate_num(
    ptr: *mut CSymbolicMatrix,
    gate: *const Complex64,
    bit: usize,
) -> i32 {
    if ptr.is_null() || gate.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    if !inner.nrows().is_power_of_two() || bit >= inner.nrows().trailing_zeros() as usize {
        return -8;
    }
    let values = unsafe { std::slice::from_raw_parts(gate, 4) }.to_vec();
    let target = match Array2::from_shape_vec((2, 2), values) {
        Ok(target) => target,
        Err(_) => return -3,
    };
    core_apply_single_qubit_gate_num(inner, &target, bit);
    0
}

/// Applies a 4x4 symbolic gate to the two target bits in place. Returns 0
/// on success, -1 on NULL, -8 when the gate is not 4x4 or the bits are
/// invalid/duplicated.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_two_qubit_gate(
    ptr: *mut CSymbolicMatrix,
    gate: *const CSymbolicMatrix,
    b0: usize,
    b1: usize,
) -> i32 {
    if ptr.is_null() || gate.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    let target = unsafe { &(*gate).inner };
    if target.nrows() != 4 || target.ncols() != 4 || !inner.nrows().is_power_of_two() {
        return -8;
    }
    let limit = inner.nrows().trailing_zeros() as usize;
    if b0 >= limit || b1 >= limit || b0 == b1 {
        return -8;
    }
    core_apply_two_qubit_gate(inner, target, b0, b1);
    0
}

/// Numeric variant of `symbolic_matrix_apply_two_qubit_gate`: `gate` points
/// at 16 row-major `Complex64` values.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_two_qubit_gate_num(
    ptr: *mut CSymbolicMatrix,
    gate: *const Complex64,
    b0: usize,
    b1: usize,
) -> i32 {
    if ptr.is_null() || gate.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    if !inner.nrows().is_power_of_two() {
        return -8;
    }
    let limit = inner.nrows().trailing_zeros() as usize;
    if b0 >= limit || b1 >= limit || b0 == b1 {
        return -8;
    }
    let values = unsafe { std::slice::from_raw_parts(gate, 16) }.to_vec();
    let target = match Array2::from_shape_vec((4, 4), values) {
        Ok(target) => target,
        Err(_) => return -3,
    };
    core_apply_two_qubit_gate_num(inner, &target, b0, b1);
    0
}

/// Applies a symbolic gate of arbitrary arity (dimension `1 << bits_len`)
/// to the target bits in place. Returns 0 on success, -1 on NULL, -8 when
/// the gate shape or bits are invalid.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_general_gate(
    ptr: *mut CSymbolicMatrix,
    gate: *const CSymbolicMatrix,
    bits: *const usize,
    bits_len: usize,
) -> i32 {
    if ptr.is_null() || gate.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    let target = unsafe { &(*gate).inner };
    let bits = match read_target_bits(bits, bits_len, inner.nrows()) {
        Ok(bits) => bits,
        Err(()) => return -8,
    };
    let gate_dim = gate_dim_for(bits_len);
    if gate_dim == 0
        || !target.nrows().is_power_of_two()
        || target.nrows() != target.ncols()
        || target.nrows() != gate_dim
    {
        return -8;
    }
    core_apply_general_gate(inner, target, &bits);
    0
}

/// Numeric variant of `symbolic_matrix_apply_general_gate`: `gate` points at
/// `1 << bits_len` squared row-major `Complex64` values.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_general_gate_num(
    ptr: *mut CSymbolicMatrix,
    gate: *const Complex64,
    bits: *const usize,
    bits_len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    let bits = match read_target_bits(bits, bits_len, inner.nrows()) {
        Ok(bits) => bits,
        Err(()) => return -8,
    };
    let gate_dim = gate_dim_for(bits_len);
    if gate_dim == 0 {
        return -8;
    }
    let target = match numeric_gate_from_raw(gate, gate_dim) {
        Some(target) => target,
        None => return -1,
    };
    core_apply_general_gate_num(inner, &target, &bits);
    0
}

/// Applies a symbolic permutation gate in place: `permutation[i] =
/// (indices[i], values[i])` means output local row `i` is `values[i]` times
/// input row `indices[i]`; `len` must equal `1 << bits_len` and indices must
/// be within the gate dimension. Returns 0 on success, -1 on NULL, -8 on a
/// length/shape mismatch or invalid bits.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_permutation_gate(
    ptr: *mut CSymbolicMatrix,
    indices: *const usize,
    values: *const *const CSymbolicComplex,
    len: usize,
    bits: *const usize,
    bits_len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let bits = match read_target_bits(bits, bits_len, unsafe { (*ptr).inner.nrows() }) {
        Ok(bits) => bits,
        Err(()) => return -8,
    };
    let gate_dim = gate_dim_for(bits_len);
    if gate_dim == 0 || len != gate_dim || indices.is_null() || values.is_null() {
        return -8;
    }
    let idx = unsafe { std::slice::from_raw_parts(indices, len) };
    let handles = unsafe { std::slice::from_raw_parts(values, len) };
    let mut permutation: Vec<(usize, SymbolicComplex)> = Vec::with_capacity(len);
    for (source, handle) in idx.iter().zip(handles.iter()) {
        if *source >= gate_dim || handle.is_null() {
            return -1;
        }
        permutation.push((*source, unsafe { (**handle).inner.clone() }));
    }
    core_apply_symbolic_permutation_gate(unsafe { &mut (*ptr).inner }, &permutation, &bits);
    0
}

/// Numeric variant of `symbolic_matrix_apply_permutation_gate`: `values`
/// points at `len` raw `Complex64` factors.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_permutation_gate_num(
    ptr: *mut CSymbolicMatrix,
    indices: *const usize,
    values: *const Complex64,
    len: usize,
    bits: *const usize,
    bits_len: usize,
) -> i32 {
    if ptr.is_null() || values.is_null() {
        return -1;
    }
    let bits = match read_target_bits(bits, bits_len, unsafe { (*ptr).inner.nrows() }) {
        Ok(bits) => bits,
        Err(()) => return -8,
    };
    let gate_dim = gate_dim_for(bits_len);
    if gate_dim == 0 || len != gate_dim || indices.is_null() {
        return -8;
    }
    let idx = unsafe { std::slice::from_raw_parts(indices, len) };
    let vals = unsafe { std::slice::from_raw_parts(values, len) };
    let permutation: Vec<(usize, Complex64)> = idx
        .iter()
        .zip(vals.iter())
        .map(|(source, value)| (*source, *value))
        .collect();
    for (source, _) in &permutation {
        if *source >= gate_dim {
            return -8;
        }
    }
    core_apply_numeric_permutation_gate(unsafe { &mut (*ptr).inner }, &permutation, &bits);
    0
}

/// Applies a symbolic diagonal gate in place: `diagonal[i]` scales local row
/// `i`; `len` must equal `1 << bits_len`. Returns 0 on success, -1 on
/// NULL, -8 on a length/shape mismatch or invalid bits.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_diagonal_gate(
    ptr: *mut CSymbolicMatrix,
    diagonal: *const *const CSymbolicComplex,
    len: usize,
    bits: *const usize,
    bits_len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let bits = match read_target_bits(bits, bits_len, unsafe { (*ptr).inner.nrows() }) {
        Ok(bits) => bits,
        Err(()) => return -8,
    };
    let gate_dim = gate_dim_for(bits_len);
    if gate_dim == 0 || len != gate_dim || diagonal.is_null() {
        return -8;
    }
    let handles = unsafe { std::slice::from_raw_parts(diagonal, len) };
    let mut values: Vec<SymbolicComplex> = Vec::with_capacity(len);
    for handle in handles {
        if handle.is_null() {
            return -1;
        }
        values.push(unsafe { (**handle).inner.clone() });
    }
    core_apply_symbolic_diagonal_gate(unsafe { &mut (*ptr).inner }, &values, &bits);
    0
}

/// Numeric variant of `symbolic_matrix_apply_diagonal_gate`: `diagonal`
/// points at `len` raw `Complex64` values.
#[unsafe(no_mangle)]
pub extern "C" fn symbolic_matrix_apply_diagonal_gate_num(
    ptr: *mut CSymbolicMatrix,
    diagonal: *const Complex64,
    len: usize,
    bits: *const usize,
    bits_len: usize,
) -> i32 {
    if ptr.is_null() || diagonal.is_null() {
        return -1;
    }
    let bits = match read_target_bits(bits, bits_len, unsafe { (*ptr).inner.nrows() }) {
        Ok(bits) => bits,
        Err(()) => return -8,
    };
    let gate_dim = gate_dim_for(bits_len);
    if gate_dim == 0 || len != gate_dim {
        return -8;
    }
    let values = unsafe { std::slice::from_raw_parts(diagonal, len) };
    core_apply_numeric_diagonal_gate(unsafe { &mut (*ptr).inner }, values, &bits);
    0
}

/// Reads an optional qubit-order array. `Err(())` marks a NULL buffer with a
/// non-zero length.
fn read_order(order: *const u32, order_len: usize) -> Result<Option<Vec<usize>>, ()> {
    if order_len == 0 {
        return Ok(None);
    }
    if order.is_null() {
        return Err(());
    }
    let slice = unsafe { std::slice::from_raw_parts(order, order_len) };
    Ok(Some(slice.iter().map(|&q| q as usize).collect()))
}
