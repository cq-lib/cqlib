// This code is part of Cqlib.
//
// (C) Copyright China Telecom Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! C ABI for circuit parameter metadata and advanced circuit instructions
//! (binding gap checklist sections 1.4 and 1.5).
//!
//! Section 1.4 exposes the circuit's parameter registry: interning
//! ([`circuit_add_parameter`]), two-step list access, the storage form
//! ([`CCircuitParam`]) produced by [`circuit_map_param`] and consumed by
//! [`circuit_resolve_parameter`]/[`circuit_parameter_value`], the symbol
//! tables and the global phase ([`CParameterValue`]).
//!
//! Section 1.5 covers advanced construction: identity-gate alias
//! ([`circuit_id`]), explicit qubit sets ([`circuit_from_qubits`]), custom
//! unitary matrices ([`circuit_unitary`], [`circuit_unitary_with_params`]),
//! composite circuit-backed gates ([`CCircuitGate`] via [`circuit_to_gate`]
//! and [`circuit_circuit_gate`]), multi-controlled gates
//! ([`circuit_multi_control`]), delays ([`circuit_delay`]), resolved
//! operation snapshots ([`CValueOperation`] via [`circuit_index`]) and
//! batch removal ([`circuit_remove_operations`]).
//!
//! `circuit_set_global_phase_param` and `circuit_add_qubits` are already
//! exported from `properties.rs` and are therefore not duplicated here.

use crate::circuit::symbolic_matrix::CSymbolicMatrix;
use crate::circuit::{CCircuit, CParameter, apply_single, check_qubit};
use crate::device::standard_gate_from_name;
use cqlib_core::circuit::symbolic_matrix::SymbolicComplex;
use cqlib_core::circuit::{
    Circuit, CircuitGate, CircuitParam, Instruction, Parameter, ParameterValue, Qubit, UnitaryGate,
    ValueOperation,
};
use ndarray::Array2;
use num_complex::Complex64;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// =====  Tags, C-side mirrors and handles  =====

/// `CCircuitParam.tag` value: the parameter is a fixed number stored in `value`.
pub const CIRCUIT_PARAM_TAG_FIXED: u8 = 0;
/// `CCircuitParam.tag` value: the parameter is an interned expression stored
/// as `index` into the circuit's parameter table.
pub const CIRCUIT_PARAM_TAG_INDEX: u8 = 1;

/// `CParameterValue.tag` value: the value is the fixed number in `value`.
pub const PARAMETER_VALUE_TAG_FIXED: u8 = 0;
/// `CParameterValue.tag` value: the value is symbolic; `param` holds a newly
/// allocated `CParameter*` that the caller frees with `param_free`.
pub const PARAMETER_VALUE_TAG_PARAM: u8 = 1;

/// Upper bound on `num_qubits` accepted by the unitary helpers. Keeps the
/// flattened matrix dimension (`4^n` doubles) far away from `usize` overflow.
const MAX_UNITARY_QUBITS: usize = 20;

/// C-side mirror of the storage-form `CircuitParam`.
///
/// Instances are produced by `circuit_map_param` and consumed by
/// `circuit_resolve_parameter` / `circuit_parameter_value`.
#[repr(C)]
pub struct CCircuitParam {
    /// One of `CIRCUIT_PARAM_TAG_*`.
    pub tag: u8,
    /// Parameter-table index, valid when `tag == CIRCUIT_PARAM_TAG_INDEX`.
    pub index: u32,
    /// Fixed value, valid when `tag == CIRCUIT_PARAM_TAG_FIXED`.
    pub value: f64,
}

/// C-side mirror of `ParameterValue` (a resolved parameter value).
///
/// When `tag == PARAMETER_VALUE_TAG_PARAM`, `param` is a newly allocated
/// `CParameter*` that the caller owns and must free with `param_free`;
/// `param` is NULL for the fixed variant.
#[repr(C)]
pub struct CParameterValue {
    /// One of `PARAMETER_VALUE_TAG_*`.
    pub tag: u8,
    /// Fixed value, valid when `tag == PARAMETER_VALUE_TAG_FIXED`.
    pub value: f64,
    /// Symbolic parameter handle, valid when `tag == PARAMETER_VALUE_TAG_PARAM`.
    pub param: *mut CParameter,
}

/// Opaque handle around a resolved value-level operation snapshot returned
/// by `circuit_index`. Read-only; free with `value_operation_free`.
pub struct CValueOperation {
    pub inner: ValueOperation,
}

/// Opaque handle around a composite circuit-backed gate (`CircuitGate`),
/// produced by `circuit_to_gate`. Free with `circuit_gate_free`.
pub struct CCircuitGate {
    pub inner: CircuitGate,
}

// =====  Internal helpers  =====

/// Converts a `CCircuitParam` mirror into the core storage form.
fn circuit_param_from_c(raw: &CCircuitParam) -> Result<CircuitParam, i32> {
    match raw.tag {
        CIRCUIT_PARAM_TAG_FIXED => Ok(CircuitParam::Fixed(raw.value)),
        CIRCUIT_PARAM_TAG_INDEX => Ok(CircuitParam::Index(raw.index)),
        _ => Err(-8),
    }
}

/// Converts a core storage form into the `CCircuitParam` mirror.
fn circuit_param_to_c(param: CircuitParam) -> CCircuitParam {
    match param {
        CircuitParam::Fixed(value) => CCircuitParam {
            tag: CIRCUIT_PARAM_TAG_FIXED,
            index: 0,
            value,
        },
        CircuitParam::Index(index) => CCircuitParam {
            tag: CIRCUIT_PARAM_TAG_INDEX,
            index,
            value: 0.0,
        },
    }
}

/// Converts a `ParameterValue` into the `CParameterValue` mirror, allocating
/// a fresh `CParameter` handle for the symbolic variant.
fn parameter_value_to_c(value: ParameterValue) -> CParameterValue {
    match value {
        ParameterValue::Fixed(value) => CParameterValue {
            tag: PARAMETER_VALUE_TAG_FIXED,
            value,
            param: std::ptr::null_mut(),
        },
        ParameterValue::Param(param) => CParameterValue {
            tag: PARAMETER_VALUE_TAG_PARAM,
            value: 0.0,
            param: Box::into_raw(Box::new(CParameter { inner: param })),
        },
    }
}

/// Parses a NUL-terminated parameter expression string.
fn parse_parameter_expr(expr: *const c_char) -> Result<Parameter, i32> {
    if expr.is_null() {
        return Err(-1);
    }
    let text = unsafe { CStr::from_ptr(expr) }.to_str().map_err(|_| -4)?;
    Parameter::try_from(text).map_err(|_| -4)
}

/// Reads a qubit-id array, validating every id against the circuit width.
fn checked_qubits(circuit: &Circuit, qubits: *const u32, len: usize) -> Result<Vec<Qubit>, i32> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if qubits.is_null() {
        return Err(-1);
    }
    let slice = unsafe { std::slice::from_raw_parts(qubits, len) };
    let mut out = Vec::with_capacity(len);
    for &id in slice {
        out.push(check_qubit(circuit, id)?);
    }
    Ok(out)
}

/// Reads an array of parameter handles into owned parameter values.
fn param_ptrs_to_values(
    params: *const *const CParameter,
    len: usize,
) -> Result<Vec<ParameterValue>, i32> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if params.is_null() {
        return Err(-1);
    }
    let slice = unsafe { std::slice::from_raw_parts(params, len) };
    let mut out = Vec::with_capacity(len);
    for &param in slice {
        if param.is_null() {
            return Err(-1);
        }
        let param = unsafe { (*param).inner.clone() };
        out.push(ParameterValue::Param(param));
    }
    Ok(out)
}

/// Copies a symbol-name list into `out` (two-step pattern). Each written
/// entry is a freshly allocated C string the caller frees with
/// `cqlib_string_free`. Returns the total number of symbols.
fn write_symbol_list(symbols: &[String], out: *mut *mut c_char, len: usize) -> usize {
    let total = symbols.len();
    if !out.is_null() {
        for (i, symbol) in symbols.iter().take(total.min(len)).enumerate() {
            let written = CString::new(symbol.as_str())
                .map(|s| s.into_raw())
                .unwrap_or(std::ptr::null_mut());
            unsafe { *out.add(i) = written };
        }
    }
    total
}

// =====  Section 1.4: parameter metadata  =====

/// Intern a symbolic parameter into the circuit's parameter registry.
///
/// Interning alone does not mark the parameter as used; it only registers
/// the expression and its symbols. Returns 0 on success, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_add_parameter(ptr: *mut CCircuit, param: *const CParameter) -> i32 {
    if ptr.is_null() || param.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let param = unsafe { (*param).inner.clone() };
    wrapper.inner.add_parameter(param);
    0
}

/// Return the number of interned parameters, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_parameters_len(ptr: *const CCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.parameters().len() }
}

/// Copy cloned parameter handles into `out` (two-step pattern; call
/// `circuit_parameters_len` first). Each written element is a newly
/// allocated `CParameter*` the caller frees with `param_free`. Returns the
/// total number of interned parameters.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_parameters(
    ptr: *const CCircuit,
    out: *mut *mut CParameter,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let parameters = unsafe { (*ptr).inner.parameters() };
    let total = parameters.len();
    if !out.is_null() {
        for (i, param) in parameters.iter().take(total.min(len)).enumerate() {
            let handle = Box::into_raw(Box::new(CParameter {
                inner: param.clone(),
            }));
            unsafe { *out.add(i) = handle };
        }
    }
    total
}

/// Canonicalize a parameter expression and intern it, returning its
/// storage form (fixed value or parameter-table index). Returns 0 on
/// success, -1 on NULL input, -3 on evaluation failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_map_param(
    ptr: *mut CCircuit,
    param: *const CParameter,
    out: *mut CCircuitParam,
) -> i32 {
    if ptr.is_null() || param.is_null() || out.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let param = unsafe { (*param).inner.clone() };
    match wrapper.inner.map_param(param) {
        Ok(mapped) => {
            unsafe { *out = circuit_param_to_c(mapped) };
            0
        }
        Err(_) => -3,
    }
}

/// Resolve a storage-form parameter back into its symbolic expression.
/// Returns a newly allocated `CParameter*` (free with `param_free`), or
/// NULL on NULL input, invalid tag or resolution failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_resolve_parameter(
    ptr: *const CCircuit,
    param: *const CCircuitParam,
) -> *mut CParameter {
    if ptr.is_null() || param.is_null() {
        return std::ptr::null_mut();
    }
    let storage = match circuit_param_from_c(unsafe { &*param }) {
        Ok(storage) => storage,
        Err(_) => return std::ptr::null_mut(),
    };
    match unsafe { (*ptr).inner.resolve_parameter(&storage) } {
        Ok(parameter) => Box::into_raw(Box::new(CParameter { inner: parameter })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Inspect a storage-form parameter without interning anything new.
///
/// Writes a `CParameterValue` to `*out`: a fixed number, or a newly
/// allocated `CParameter*` for the symbolic case (free with `param_free`).
/// Returns 0 on success, -1 on NULL input, -3 for an invalid index, -8 for
/// an invalid tag.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_parameter_value(
    ptr: *const CCircuit,
    param: *const CCircuitParam,
    out: *mut CParameterValue,
) -> i32 {
    if ptr.is_null() || param.is_null() || out.is_null() {
        return -1;
    }
    let storage = match circuit_param_from_c(unsafe { &*param }) {
        Ok(storage) => storage,
        Err(code) => return code,
    };
    match unsafe { (*ptr).inner.parameter_value(&storage) } {
        Ok(value) => {
            unsafe { *out = parameter_value_to_c(value) };
            0
        }
        Err(_) => -3,
    }
}

/// Return the number of registered symbol names, or 0 for NULL.
///
/// This is the full registry and may include symbols that are interned but
/// no longer referenced by the circuit's operations.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_symbols_len(ptr: *const CCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.symbols().len() }
}

/// Copy registered symbol names into `out` (two-step pattern; call
/// `circuit_symbols_len` first). Each written element is a freshly
/// allocated C string the caller frees with `cqlib_string_free`. Returns
/// the total number of registered symbols.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_symbols(
    ptr: *const CCircuit,
    out: *mut *mut c_char,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let symbols: Vec<String> = unsafe { (*ptr).inner.symbols().iter().cloned().collect() };
    write_symbol_list(&symbols, out, len)
}

/// Return the number of symbols referenced by the circuit's executable IR,
/// or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_used_symbols_len(ptr: *const CCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.used_symbols().len() }
}

/// Copy the IR-referenced symbol names into `out` (two-step pattern; call
/// `circuit_used_symbols_len` first). Each written element is a freshly
/// allocated C string the caller frees with `cqlib_string_free`. Returns
/// the total number of used symbols.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_used_symbols(
    ptr: *const CCircuit,
    out: *mut *mut c_char,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let symbols: Vec<String> = unsafe { (*ptr).inner.used_symbols().iter().cloned().collect() };
    write_symbol_list(&symbols, out, len)
}

/// Check whether a symbol is referenced by the circuit's executable IR
/// (global phase and operation parameters, including nested bodies).
/// Returns 1 when used, 0 when not, -1 on NULL input, -4 on invalid UTF-8.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_uses_symbol(ptr: *const CCircuit, name: *const c_char) -> i32 {
    if ptr.is_null() || name.is_null() {
        return -1;
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(name) => name,
        Err(_) => return -4,
    };
    unsafe { (*ptr).inner.uses_symbol(name) as i32 }
}

/// Inspect the circuit's global phase.
///
/// Writes a `CParameterValue` to `*out`: a fixed number, or a newly
/// allocated `CParameter*` for the symbolic case (free with `param_free`).
/// Returns 0 on success, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_global_phase(ptr: *const CCircuit, out: *mut CParameterValue) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let parameter = unsafe { (*ptr).inner.global_phase() };
    unsafe { *out = parameter_value_to_c(ParameterValue::from(parameter)) };
    0
}

// =====  Section 1.5: advanced instructions  =====

/// Append an identity gate on `qubit`; alias of `circuit_i` kept for
/// frontend naming compatibility. Returns 0 on success, negative on error.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_id(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::i)
}

/// Create a new circuit from an explicit list of qubit ids.
///
/// Returns a newly allocated `CCircuit*` (free with `circuit_free`), or
/// NULL on NULL input or duplicate qubits.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_from_qubits(qubits: *const u32, len: usize) -> *mut CCircuit {
    if len > 0 && qubits.is_null() {
        return std::ptr::null_mut();
    }
    let ids = if len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(qubits, len) }
    };
    let qubits: Vec<Qubit> = ids.iter().map(|&id| Qubit::new(id)).collect();
    match Circuit::from_qubits(qubits) {
        Ok(inner) => Box::into_raw(Box::new(CCircuit { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Append a custom unitary gate defined by a numeric matrix.
///
/// `matrix` is a row-major flattened array of `2^num_qubits * 2^num_qubits`
/// `Complex64` values (interleaved re/im, same layout as `circuit_to_matrix`
/// output). Returns 0 on success, -1 on NULL input, -2 on out-of-bounds
/// qubits, -3 on shape or application failure, -4 on invalid UTF-8 label,
/// -8 when `num_qubits` exceeds 20.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_unitary(
    ptr: *mut CCircuit,
    label: *const c_char,
    num_qubits: usize,
    matrix: *const Complex64,
    qubits: *const u32,
    qubits_len: usize,
) -> i32 {
    if ptr.is_null() || label.is_null() || matrix.is_null() {
        return -1;
    }
    if num_qubits > MAX_UNITARY_QUBITS {
        return -8;
    }
    let label = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(label) => label,
        Err(_) => return -4,
    };
    let dim = 1usize << num_qubits;
    let expected = dim * dim;
    let flat = unsafe { std::slice::from_raw_parts(matrix, expected) }.to_vec();
    let mat = match Array2::from_shape_vec((dim, dim), flat) {
        Ok(mat) => mat,
        Err(_) => return -8,
    };
    let gate = match UnitaryGate::new(label, num_qubits as u16, 0).with_matrix(mat) {
        Ok(gate) => gate,
        Err(_) => return -3,
    };
    let wrapper = unsafe { &mut *ptr };
    let qubits = match checked_qubits(&wrapper.inner, qubits, qubits_len) {
        Ok(qubits) => qubits,
        Err(code) => return code,
    };
    wrapper.inner.unitary(gate, qubits).map_or(-3, |_| 0)
}

/// Append a parameterized custom unitary gate defined by a symbolic matrix.
///
/// `re_exprs` and `im_exprs` are row-major flattened arrays of
/// `2^num_qubits * 2^num_qubits` parameter-expression strings (e.g.
/// `"cos(theta)"`); `im_exprs` may be NULL, in which case all imaginary
/// parts are zero. `param_names` lists the positional parameter names that
/// application arguments are bound to, in order. Returns 0 on success, -1
/// on NULL input, -2 on out-of-bounds qubits, -3 on shape or application
/// failure, -4 on invalid UTF-8 or expression syntax, -8 on invalid sizes.
#[allow(clippy::too_many_arguments)]
#[unsafe(no_mangle)]
pub extern "C" fn circuit_unitary_with_params(
    ptr: *mut CCircuit,
    label: *const c_char,
    num_qubits: usize,
    re_exprs: *const *const c_char,
    im_exprs: *const *const c_char,
    param_names: *const *const c_char,
    param_names_len: usize,
    qubits: *const u32,
    qubits_len: usize,
    params: *const *const CParameter,
    params_len: usize,
) -> i32 {
    if ptr.is_null() || label.is_null() || re_exprs.is_null() {
        return -1;
    }
    if num_qubits > MAX_UNITARY_QUBITS || param_names_len > u16::MAX as usize {
        return -8;
    }
    let label = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(label) => label,
        Err(_) => return -4,
    };
    let dim = 1usize << num_qubits;
    let expected = dim * dim;
    let re_slice = unsafe { std::slice::from_raw_parts(re_exprs, expected) };
    let im_slice = if im_exprs.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(im_exprs, expected) })
    };
    let mut entries = Vec::with_capacity(expected);
    for i in 0..expected {
        let re = match parse_parameter_expr(re_slice[i]) {
            Ok(re) => re,
            Err(code) => return code,
        };
        let im = match im_slice {
            Some(slice) => match parse_parameter_expr(slice[i]) {
                Ok(im) => im,
                Err(code) => return code,
            },
            None => Parameter::from(0.0),
        };
        entries.push(SymbolicComplex::new(re, im));
    }
    let matrix = match Array2::from_shape_vec((dim, dim), entries) {
        Ok(matrix) => matrix,
        Err(_) => return -8,
    };
    let mut names = Vec::with_capacity(param_names_len);
    if param_names_len > 0 {
        if param_names.is_null() {
            return -1;
        }
        let name_slice = unsafe { std::slice::from_raw_parts(param_names, param_names_len) };
        for &name in name_slice {
            let name = match unsafe { CStr::from_ptr(name) }.to_str() {
                Ok(name) => name,
                Err(_) => return -4,
            };
            names.push(name.to_string());
        }
    }
    let gate = match UnitaryGate::new(label, num_qubits as u16, names.len() as u16)
        .with_symbolic_matrix(names, matrix)
    {
        Ok(gate) => gate,
        Err(_) => return -3,
    };
    let wrapper = unsafe { &mut *ptr };
    let qubits = match checked_qubits(&wrapper.inner, qubits, qubits_len) {
        Ok(qubits) => qubits,
        Err(code) => return code,
    };
    let params = match param_ptrs_to_values(params, params_len) {
        Ok(params) => params,
        Err(code) => return code,
    };
    wrapper
        .inner
        .unitary_with_params(gate, qubits, params)
        .map_or(-3, |_| 0)
}

/// Returns the number of `Complex64` elements (`rows * cols`) that
/// `unitary_gate_matrix_for_params` writes for `matrix`, or 0 for NULL or a
/// non-square symbolic matrix.
#[unsafe(no_mangle)]
pub extern "C" fn unitary_gate_matrix_for_params_len(matrix: *const CSymbolicMatrix) -> usize {
    if matrix.is_null() {
        return 0;
    }
    let inner = unsafe { &(*matrix).inner };
    if inner.nrows() != inner.ncols() {
        return 0;
    }
    inner.nrows().saturating_mul(inner.ncols())
}

/// Evaluates a custom unitary gate's numeric matrix at positional parameter
/// values.
///
/// The C ABI has no standalone unitary-gate handle, so the gate definition is
/// supplied the same way `circuit_unitary_with_params` builds one: a symbolic
/// `matrix` (`2^n x 2^n`) plus the ordered formal `param_names` that bind
/// application arguments positionally to the matrix symbols. This wraps
/// [`UnitaryGate::matrix_for_params`], including its shape and unitarity
/// checks.
///
/// The result is written to `buffer` in row-major order as interleaved
/// (real, imag) `Complex64` values (two-step pattern; pair with
/// `unitary_gate_matrix_for_params_len`). Returns 0 on success, -1 on NULL
/// input, -3 on shape, parameter-count, non-finite-value or unitarity failure,
/// -4 on invalid UTF-8 parameter names, or -8 on invalid sizes or a too-small
/// buffer.
#[unsafe(no_mangle)]
pub extern "C" fn unitary_gate_matrix_for_params(
    matrix: *const CSymbolicMatrix,
    param_names: *const *const c_char,
    param_names_len: usize,
    params: *const f64,
    params_len: usize,
    buffer: *mut Complex64,
    buffer_len: usize,
) -> i32 {
    if matrix.is_null() || buffer.is_null() {
        return -1;
    }
    if param_names_len > 0 && param_names.is_null() {
        return -1;
    }
    if params_len > 0 && params.is_null() {
        return -1;
    }
    let symbolic = unsafe { &(*matrix).inner }.clone();
    let dim = symbolic.nrows();
    if dim == 0
        || symbolic.ncols() != dim
        || !dim.is_power_of_two()
        || dim.trailing_zeros() as usize > MAX_UNITARY_QUBITS
        || param_names_len > u16::MAX as usize
    {
        return -8;
    }
    let total = dim.saturating_mul(dim);
    if buffer_len < total {
        return -8;
    }
    let mut names = Vec::with_capacity(param_names_len);
    if param_names_len > 0 {
        let name_slice = unsafe { std::slice::from_raw_parts(param_names, param_names_len) };
        for &name in name_slice {
            if name.is_null() {
                return -1;
            }
            let name = match unsafe { CStr::from_ptr(name) }.to_str() {
                Ok(name) => name,
                Err(_) => return -4,
            };
            names.push(name.to_string());
        }
    }
    let values = if params_len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(params, params_len) }.to_vec()
    };
    let num_qubits = dim.trailing_zeros() as u16;
    let gate = match UnitaryGate::new("unitary", num_qubits, names.len() as u16)
        .with_symbolic_matrix(names, symbolic)
    {
        Ok(gate) => gate,
        Err(_) => return -3,
    };
    let evaluated = match gate.matrix_for_params(&values) {
        Ok(matrix) => matrix,
        Err(_) => return -3,
    };
    let out = unsafe { std::slice::from_raw_parts_mut(buffer, total) };
    for (slot, value) in out.iter_mut().zip(evaluated.iter()) {
        *slot = *value;
    }
    0
}

/// Freeze a copy of the circuit into a reusable composite gate.
///
/// The input circuit stays valid (it is cloned). Returns a newly allocated
/// `CCircuitGate*`, or NULL on NULL input, invalid UTF-8 name or failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_to_gate(ptr: *const CCircuit, name: *const c_char) -> *mut CCircuitGate {
    if ptr.is_null() || name.is_null() {
        return std::ptr::null_mut();
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(name) => name,
        Err(_) => return std::ptr::null_mut(),
    };
    match unsafe { (*ptr).inner.clone() }.to_gate(name) {
        Ok(Instruction::CircuitGate(gate)) => {
            Box::into_raw(Box::new(CCircuitGate { inner: *gate }))
        }
        _ => std::ptr::null_mut(),
    }
}

/// Free a composite gate handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_gate_free(ptr: *mut CCircuitGate) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Return the gate's name as a newly allocated C string (free with
/// `cqlib_string_free`), or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_gate_name(ptr: *const CCircuitGate) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match CString::new(unsafe { (*ptr).inner.name() }) {
        Ok(name) => name.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return the number of qubits the gate acts on, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_gate_num_qubits(ptr: *const CCircuitGate) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Return the number of parameters the gate accepts, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_gate_num_params(ptr: *const CCircuitGate) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_params() }
}

/// Return the number of positional parameter names in the gate's call
/// signature, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_gate_signature_params_len(ptr: *const CCircuitGate) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.signature_params().len() }
}

/// Copy the gate's positional parameter-signature names into `out`
/// (two-step pattern; call `circuit_gate_signature_params_len` first).
/// Each written element is a freshly allocated C string the caller frees
/// with `cqlib_string_free`. Returns the total number of signature names.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_gate_signature_params(
    ptr: *const CCircuitGate,
    out: *mut *mut c_char,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let names: Vec<String> = unsafe { (*ptr).inner.signature_params().iter().cloned().collect() };
    write_symbol_list(&names, out, len)
}

/// Append a composite gate to the circuit, binding `params` positionally
/// to the gate's parameter signature. Returns 0 on success, -1 on NULL
/// input, -2 on out-of-bounds qubits, -3 on arity or application failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_circuit_gate(
    ptr: *mut CCircuit,
    gate: *const CCircuitGate,
    qubits: *const u32,
    qubits_len: usize,
    params: *const *const CParameter,
    params_len: usize,
) -> i32 {
    if ptr.is_null() || gate.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let qubits = match checked_qubits(&wrapper.inner, qubits, qubits_len) {
        Ok(qubits) => qubits,
        Err(code) => return code,
    };
    let params = match param_ptrs_to_values(params, params_len) {
        Ok(params) => params,
        Err(code) => return code,
    };
    let gate = unsafe { (*gate).inner.clone() };
    wrapper
        .inner
        .circuit_gate(gate, qubits, params)
        .map_or(-3, |_| 0)
}

/// Append a multi-controlled standard gate.
///
/// `gate_name` is a standard-gate name ("X", "RX", "SWAP", ...; the same
/// set accepted by the noise/device modules). `controls` and `targets`
/// together form the operation's qubit list; `params` are the gate's
/// parameters. Returns 0 on success, -1 on NULL input, -2 on
/// out-of-bounds qubits, -3 when the gate cannot be controlled or the
/// arity mismatches, -4 on invalid UTF-8, -8 on unknown gate name.
#[allow(clippy::too_many_arguments)]
#[unsafe(no_mangle)]
pub extern "C" fn circuit_multi_control(
    ptr: *mut CCircuit,
    gate_name: *const c_char,
    controls: *const u32,
    controls_len: usize,
    targets: *const u32,
    targets_len: usize,
    params: *const *const CParameter,
    params_len: usize,
) -> i32 {
    if ptr.is_null() || gate_name.is_null() {
        return -1;
    }
    let gate_name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(gate_name) => gate_name,
        Err(_) => return -4,
    };
    let gate = match standard_gate_from_name(gate_name) {
        Some(gate) => gate,
        None => return -8,
    };
    let wrapper = unsafe { &mut *ptr };
    let controls = match checked_qubits(&wrapper.inner, controls, controls_len) {
        Ok(controls) => controls,
        Err(code) => return code,
    };
    let targets = match checked_qubits(&wrapper.inner, targets, targets_len) {
        Ok(targets) => targets,
        Err(code) => return code,
    };
    let params = match param_ptrs_to_values(params, params_len) {
        Ok(params) => params,
        Err(code) => return code,
    };
    wrapper
        .inner
        .multi_control(gate, controls, targets, params)
        .map_or(-3, |_| 0)
}

/// Append a delay (idle period) on `qubit` with a numeric duration.
/// Returns 0 on success, -1 on NULL input, -2 on out-of-bounds qubit, -3
/// on non-finite duration.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_delay(ptr: *mut CCircuit, qubit: u32, duration: f64) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let qubit = match check_qubit(&wrapper.inner, qubit) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    wrapper
        .inner
        .delay(qubit, ParameterValue::Fixed(duration))
        .map_or(-3, |_| 0)
}

/// Append a delay (idle period) on `qubit` with a symbolic duration.
/// Returns 0 on success, -1 on NULL input, -2 on out-of-bounds qubit, -3
/// on application failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_delay_param(
    ptr: *mut CCircuit,
    qubit: u32,
    duration: *const CParameter,
) -> i32 {
    if ptr.is_null() || duration.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let qubit = match check_qubit(&wrapper.inner, qubit) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    let duration = unsafe { (*duration).inner.clone() };
    wrapper
        .inner
        .delay(qubit, ParameterValue::Param(duration))
        .map_or(-3, |_| 0)
}

/// Return a resolved snapshot of the top-level operation at `index`.
///
/// The returned `CValueOperation*` is read-only and independent of later
/// circuit mutations; free it with `value_operation_free`. Returns NULL on
/// NULL input or out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_index(ptr: *const CCircuit, index: usize) -> *mut CValueOperation {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*ptr).inner.index(index) } {
        Ok(inner) => Box::into_raw(Box::new(CValueOperation { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free an operation snapshot handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn value_operation_free(ptr: *mut CValueOperation) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Return the operation's instruction name (e.g. "H", "delay") as a newly
/// allocated C string (free with `cqlib_string_free`), or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn value_operation_name(ptr: *const CValueOperation) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match CString::new(unsafe { (*ptr).inner.name() }) {
        Ok(name) => name.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return the operation's instruction category (e.g. "standard", "delay")
/// as a newly allocated C string (free with `cqlib_string_free`), or NULL
/// on error.
#[unsafe(no_mangle)]
pub extern "C" fn value_operation_instruction_type(ptr: *const CValueOperation) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match CString::new(unsafe { (*ptr).inner.instruction_type() }) {
        Ok(name) => name.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return the operation's optional metadata label as a newly allocated C
/// string (free with `cqlib_string_free`). Returns NULL when the operation
/// has no label or on error.
#[unsafe(no_mangle)]
pub extern "C" fn value_operation_label(ptr: *const CValueOperation) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*ptr).inner.label.as_deref() } {
        Some(label) => match CString::new(label) {
            Ok(label) => label.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Return the number of qubits the operation acts on, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn value_operation_num_qubits(ptr: *const CValueOperation) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.qubits.len() }
}

/// Copy the operation's qubit ids into `out` (two-step pattern; call
/// `value_operation_num_qubits` first). Returns the total qubit count.
#[unsafe(no_mangle)]
pub extern "C" fn value_operation_qubits(
    ptr: *const CValueOperation,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let qubits = unsafe { &(*ptr).inner.qubits };
    let total = qubits.len();
    if !out.is_null() {
        for (i, qubit) in qubits.iter().take(total.min(len)).enumerate() {
            unsafe { *out.add(i) = qubit.id() };
        }
    }
    total
}

/// Return the number of parameters carried by the operation, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn value_operation_num_params(ptr: *const CValueOperation) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.params.len() }
}

/// Inspect one of the operation's parameters.
///
/// Writes a `CParameterValue` to `*out`: a fixed number, or a newly
/// allocated `CParameter*` for the symbolic case (free with `param_free`).
/// Returns 0 on success, -1 on NULL input, -3 on out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn value_operation_param(
    ptr: *const CValueOperation,
    index: usize,
    out: *mut CParameterValue,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let params = unsafe { &(*ptr).inner.params };
    match params.get(index) {
        Some(value) => {
            unsafe { *out = parameter_value_to_c(value.clone()) };
            0
        }
        None => -3,
    }
}

/// Remove multiple top-level operations by index.
///
/// `indices` are interpreted against the operation list before deletion;
/// duplicates are ignored and deletion is atomic on failure. Returns 0 on
/// success (including an empty index list), -1 on NULL input, -3 when any
/// index is out of bounds or a classical value is still in use.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_remove_operations(
    ptr: *mut CCircuit,
    indices: *const usize,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let indices: Vec<usize> = if len == 0 {
        Vec::new()
    } else {
        if indices.is_null() {
            return -1;
        }
        unsafe { std::slice::from_raw_parts(indices, len) }.to_vec()
    };
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.remove_operations(indices).map_or(-3, |_| 0)
}
