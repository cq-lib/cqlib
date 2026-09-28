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

//! C ABI for standalone circuit operations (instruction + qubits + params).

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::circuit::classical::{CClassicalExpr, CClassicalValueInfo, classical_type_parts};
use crate::device::standard_gate_from_name;
use cqlib_core::circuit::gate::instruction::Instruction;
use cqlib_core::circuit::{CircuitParam, ClassicalExprKind, Directive, Operation, Qubit};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Opaque handle around an [`Operation`].
pub struct COperation {
    pub inner: Operation,
}

/// Parameter tag written by [`operation_params`]: value is fixed inline.
pub const OPERATION_PARAM_FIXED: u32 = 0;
/// Parameter tag written by [`operation_params`]: value is an index into
/// the owning circuit's parameter table and cannot be resolved here.
pub const OPERATION_PARAM_INDEX: u32 = 1;

/// `operation_directive` result tag: the operation carries no directive
/// instruction.
pub const OPERATION_DIRECTIVE_NONE: u32 = 0;
/// `operation_directive` result tag: `Directive::Barrier`.
pub const OPERATION_DIRECTIVE_BARRIER: u32 = 1;
/// `operation_directive` result tag: `Directive::Measure`.
pub const OPERATION_DIRECTIVE_MEASURE: u32 = 2;
/// `operation_directive` result tag: `Directive::Reset`.
pub const OPERATION_DIRECTIVE_RESET: u32 = 3;

/// Creates a standard-gate operation from a gate name (e.g. "H", "CX",
/// "RZZ"). `params` holds fixed double values. Returns a newly allocated
/// `COperation*` (free with `operation_free`), or NULL on NULL input or
/// unknown gate name.
#[unsafe(no_mangle)]
pub extern "C" fn operation_new(
    gate_name: *const c_char,
    qubits: *const u32,
    qubits_len: usize,
    params: *const f64,
    params_len: usize,
) -> *mut COperation {
    if gate_name.is_null() {
        return std::ptr::null_mut();
    }
    if qubits_len > 0 && qubits.is_null() {
        return std::ptr::null_mut();
    }
    if params_len > 0 && params.is_null() {
        return std::ptr::null_mut();
    }
    let name = unsafe { CStr::from_ptr(gate_name) };
    let Some(gate) = standard_gate_from_name(name.to_string_lossy().as_ref()) else {
        return std::ptr::null_mut();
    };
    let qubit_list = if qubits_len == 0 {
        Default::default()
    } else {
        unsafe { std::slice::from_raw_parts(qubits, qubits_len) }
            .iter()
            .map(|&id| Qubit::new(id))
            .collect()
    };
    let param_list = if params_len == 0 {
        Default::default()
    } else {
        unsafe { std::slice::from_raw_parts(params, params_len) }
            .iter()
            .map(|&value| CircuitParam::Fixed(value))
            .collect()
    };
    let inner = Operation {
        instruction: Instruction::Standard(gate),
        qubits: qubit_list,
        params: param_list,
        label: None,
    };
    Box::into_raw(Box::new(COperation { inner }))
}

/// Frees an operation handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn operation_free(ptr: *mut COperation) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Returns the instruction name (e.g. "H", "RZZ") as a newly allocated C
/// string (free with `cqlib_string_free`), or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn operation_name(ptr: *const COperation) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let name = unsafe { (*ptr).inner.instruction.name() };
    match CString::new(name) {
        Ok(text) => text.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns whether the operation is driven by a standard-gate instruction:
/// 1 when yes, 0 when no, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn operation_is_standard_gate(ptr: *const COperation) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.standard_gate().is_some() as i32 }
}

/// Returns the number of qubits the operation acts on, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn operation_num_qubits(ptr: *const COperation) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.qubits.len() }
}

/// Returns the number of qubits the operation acts on, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn operation_qubits_len(ptr: *const COperation) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.qubits.len() }
}

/// Copies the qubit ids in application order into `buffer`. Returns 0 on
/// success, -1 on NULL input, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn operation_qubits(ptr: *const COperation, buffer: *mut u32, len: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let qubits = unsafe { (*ptr).inner.qubits.as_slice() };
    if len < qubits.len() {
        return -8;
    }
    if qubits.is_empty() {
        return 0;
    }
    if buffer.is_null() {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(buffer, qubits.len()) };
    for (slot, qubit) in slice.iter_mut().zip(qubits.iter()) {
        *slot = qubit.id();
    }
    0
}

/// Returns the number of parameters carried by the operation, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn operation_params_len(ptr: *const COperation) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.params.len() }
}

/// Copies the parameters into `tags` and `values`. `tags[i]` is one of
/// `OPERATION_PARAM_*`; `values[i]` is the fixed value, or the parameter
/// table index when tagged `OPERATION_PARAM_INDEX`. Returns 0 on success,
/// -1 on NULL input, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn operation_params(
    ptr: *const COperation,
    tags: *mut u32,
    values: *mut f64,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let params = unsafe { (*ptr).inner.params.as_slice() };
    if len < params.len() {
        return -8;
    }
    if params.is_empty() {
        return 0;
    }
    if tags.is_null() || values.is_null() {
        return -1;
    }
    let tag_slice = unsafe { std::slice::from_raw_parts_mut(tags, params.len()) };
    let value_slice = unsafe { std::slice::from_raw_parts_mut(values, params.len()) };
    for ((tag_slot, value_slot), param) in tag_slice
        .iter_mut()
        .zip(value_slice.iter_mut())
        .zip(params.iter())
    {
        match param {
            CircuitParam::Fixed(value) => {
                *tag_slot = OPERATION_PARAM_FIXED;
                *value_slot = *value;
            }
            CircuitParam::Index(index) => {
                *tag_slot = OPERATION_PARAM_INDEX;
                *value_slot = *index as f64;
            }
        }
    }
    0
}

/// Returns the optional label as a newly allocated C string (free with
/// `cqlib_string_free`), or NULL when the operation has no label or the
/// handle is NULL.
#[unsafe(no_mangle)]
pub extern "C" fn operation_label(ptr: *const COperation) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let Some(label) = (unsafe { (*ptr).inner.label.as_ref() }) else {
        return std::ptr::null_mut();
    };
    match CString::new(&**label) {
        Ok(text) => text.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns the number of complex elements (rows * cols) in the unitary
/// matrix, or 0 on NULL input or when the operation has no numeric matrix
/// (non-unitary instruction or unresolved symbolic parameter).
#[unsafe(no_mangle)]
pub extern "C" fn operation_matrix_len(ptr: *const COperation) -> usize {
    if ptr.is_null() {
        return 0;
    }
    match unsafe { (*ptr).inner.matrix() } {
        Ok(matrix) => matrix.len(),
        Err(_) => 0,
    }
}

/// Writes the matrix shape into `rows` and `cols`. Returns 0 on success,
/// -1 on NULL input, -3 when the operation has no numeric matrix.
#[unsafe(no_mangle)]
pub extern "C" fn operation_matrix_dims(
    ptr: *const COperation,
    rows: *mut usize,
    cols: *mut usize,
) -> i32 {
    if ptr.is_null() || rows.is_null() || cols.is_null() {
        return -1;
    }
    match unsafe { (*ptr).inner.matrix() } {
        Ok(matrix) => {
            let (nrows, ncols) = matrix.dim();
            unsafe {
                *rows = nrows;
                *cols = ncols;
            }
            0
        }
        Err(_) => -3,
    }
}

/// Copies the unitary matrix into `out` as interleaved (real, imag)
/// doubles in row-major order. Returns the number of complex elements
/// written, or 0 on error. The buffer must hold at least
/// `2 * operation_matrix_len(...)` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn operation_matrix(
    ptr: *const COperation,
    out: *mut f64,
    buffer_len: usize,
) -> usize {
    if ptr.is_null() || out.is_null() {
        return 0;
    }
    match unsafe { (*ptr).inner.matrix() } {
        Ok(matrix) => {
            let total = matrix.len();
            if buffer_len < total * 2 {
                return 0;
            }
            for (i, value) in matrix.iter().enumerate() {
                unsafe {
                    *out.add(i * 2) = value.re;
                    *out.add(i * 2 + 1) = value.im;
                }
            }
            total
        }
        Err(_) => 0,
    }
}

// =====  Instruction-kind discriminants  =====

/// Returns whether the operation uses a user-defined unitary instruction:
/// 1 when yes, 0 when no, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn operation_is_unitary(ptr: *const COperation) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.instruction.is_unitary() as i32 }
}

/// Returns whether the operation uses a multi-controlled-gate instruction:
/// 1 when yes, 0 when no, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn operation_is_mcgate(ptr: *const COperation) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.instruction.is_mcgate() as i32 }
}

/// Returns whether the operation uses a circuit-backed gate instruction:
/// 1 when yes, 0 when no, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn operation_is_circuit_gate(ptr: *const COperation) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.instruction.is_circuit_gate() as i32 }
}

/// Returns whether the operation uses a non-unitary directive instruction
/// (barrier, measure, reset): 1 when yes, 0 when no, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn operation_is_directive(ptr: *const COperation) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.instruction.is_directive() as i32 }
}

/// Returns whether the operation uses a delay instruction: 1 when yes,
/// 0 when no, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn operation_is_delay(ptr: *const COperation) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.instruction.is_delay() as i32 }
}

/// Returns whether the operation uses a structured classical-control-flow
/// instruction (if / while / for / switch / break / continue): 1 when yes,
/// 0 when no, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn operation_is_classical_control(ptr: *const COperation) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.instruction.is_classical_control() as i32 }
}

/// Returns whether the operation uses a classical-data instruction (store,
/// measure-into-value): 1 when yes, 0 when no, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn operation_is_classical_data(ptr: *const COperation) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.instruction.is_classical_data() as i32 }
}

/// Returns whether the operation is a unitary quantum gate (standard,
/// multi-controlled, user-defined unitary, or circuit-backed gate): 1 when
/// yes, 0 when no, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn operation_is_quantum_gate(ptr: *const COperation) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.instruction.is_quantum_gate() as i32 }
}

/// Returns whether the operation carries a plain instruction rather than
/// structured classical control flow: 1 when yes, 0 for classical-control
/// operations, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn operation_is_instruction(ptr: *const COperation) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let is_control = matches!(
        unsafe { &(*ptr).inner.instruction },
        Instruction::ClassicalControl(_)
    );
    (!is_control) as i32
}

/// Returns the fixed `(qubit_count, parameter_count)` intrinsic to the
/// instruction, mirroring the core `Instruction::gate_arity`. Returns 0 on
/// success, -1 on NULL input, -3 when the arity is variable
/// (barrier, multi-qubit measurement, classical control).
#[unsafe(no_mangle)]
pub extern "C" fn operation_gate_arity(
    ptr: *const COperation,
    qubits: *mut usize,
    params: *mut usize,
) -> i32 {
    if ptr.is_null() || qubits.is_null() || params.is_null() {
        return -1;
    }
    match unsafe { (*ptr).inner.instruction.gate_arity() } {
        Some((num_qubits, num_params)) => {
            unsafe {
                *qubits = num_qubits;
                *params = num_params;
            }
            0
        }
        None => -3,
    }
}

/// Returns an owned clone of the operation when it carries a plain
/// (non-control-flow) instruction, mirroring the core `as_instruction`
/// unwrapping. The result is freed with `operation_free`. Returns NULL on
/// NULL input or when the operation is a classical-control instruction.
#[unsafe(no_mangle)]
pub extern "C" fn operation_as_instruction(ptr: *const COperation) -> *mut COperation {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let inner = unsafe { &(*ptr).inner };
    if matches!(inner.instruction, Instruction::ClassicalControl(_)) {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(COperation {
        inner: inner.clone(),
    }))
}

/// Returns the directive carried by the operation as one of the
/// `OPERATION_DIRECTIVE_*` tags: `OPERATION_DIRECTIVE_NONE` (0) when the
/// operation is not a directive, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn operation_directive(ptr: *const COperation) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let tag = match unsafe { &(*ptr).inner.instruction } {
        Instruction::Directive(Directive::Barrier) => OPERATION_DIRECTIVE_BARRIER,
        Instruction::Directive(Directive::Measure) => OPERATION_DIRECTIVE_MEASURE,
        Instruction::Directive(Directive::Reset) => OPERATION_DIRECTIVE_RESET,
        _ => OPERATION_DIRECTIVE_NONE,
    };
    tag as i32
}

/// Writes a snapshot of the immutable classical value produced by a
/// measurement operation into `out` (circuit-local value table index plus
/// (tag, width) type parts). Returns 0 on success, -1 on NULL input, -3
/// when the operation is not a measurement (no classical result).
#[unsafe(no_mangle)]
pub extern "C" fn operation_result(ptr: *const COperation, out: *mut CClassicalValueInfo) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let value = match unsafe { &(*ptr).inner.instruction } {
        Instruction::ClassicalData(operation) => operation.result(),
        _ => return -3,
    };
    let Some(value) = value else {
        return -3;
    };
    let (tag, width) = classical_type_parts(value.ty());
    unsafe {
        *out = CClassicalValueInfo {
            index: value.index(),
            tag,
            width,
        };
    }
    0
}

/// Returns whether the operation directly or recursively reads the
/// immutable classical value referenced by `expr`: 1 when yes, 0 when no,
/// -1 on NULL input, -8 when `expr` is not a plain value expression.
#[unsafe(no_mangle)]
pub extern "C" fn operation_reads_value(
    ptr: *const COperation,
    expr: *const CClassicalExpr,
) -> i32 {
    if ptr.is_null() || expr.is_null() {
        return -1;
    }
    let value = match unsafe { (*expr).inner.kind() } {
        ClassicalExprKind::Value(value) => value,
        _ => return -8,
    };
    unsafe { (*ptr).inner.instruction.reads_value(*value) as i32 }
}
