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

//! C ABI for classical variables, expressions, and measurement targets.

use crate::circuit::CCircuit;
use cqlib_core::circuit::{
    ClassicalExpr, ClassicalExprKind, ClassicalType, ClassicalValue, ClassicalVar, Qubit,
};
use std::collections::HashMap;

/// Opaque handle around [`ClassicalVar`] for C ABI.
pub struct CClassicalVar {
    pub inner: ClassicalVar,
}

/// Opaque handle around [`ClassicalExpr`] for C ABI.
pub struct CClassicalExpr {
    pub inner: ClassicalExpr,
}

// =====  Classical type tags  =====

/// Classical type tag exposed to C.
/// | Value | Type   |
/// |-------|--------|
/// | 0     | Bit    |
/// | 1     | Bool   |
/// | 2     | UInt   |
/// | 3     | BitVec |
pub const CQLIB_CLASSICAL_TYPE_BIT: u32 = 0;

pub const CQLIB_CLASSICAL_TYPE_BOOL: u32 = 1;

pub const CQLIB_CLASSICAL_TYPE_UINT: u32 = 2;

pub const CQLIB_CLASSICAL_TYPE_BIT_VEC: u32 = 3;

// =====  Classical expression kind tags  =====

/// Classical expression node kind tag exposed to C by `classical_expr_kind`.
/// | Value | Kind          |
/// |-------|---------------|
/// | 0     | Var           |
/// | 1     | Value         |
/// | 2     | BoolLiteral   |
/// | 3     | BitLiteral    |
/// | 4     | UIntLiteral   |
/// | 5     | BitVecLiteral |
/// | 6     | Unary         |
/// | 7     | Binary        |
/// | 8     | Compare       |
/// | 9     | Cast          |
/// | 10    | Select        |
/// | 11    | ExtractBit    |
/// | 12    | ExtractBits   |
/// | 13    | Concat        |
/// | 14    | PackBits      |
pub const CQLIB_CLASSICAL_EXPR_VAR: u32 = 0;

pub const CQLIB_CLASSICAL_EXPR_VALUE: u32 = 1;

pub const CQLIB_CLASSICAL_EXPR_BOOL_LITERAL: u32 = 2;

pub const CQLIB_CLASSICAL_EXPR_BIT_LITERAL: u32 = 3;

pub const CQLIB_CLASSICAL_EXPR_UINT_LITERAL: u32 = 4;

pub const CQLIB_CLASSICAL_EXPR_BIT_VEC_LITERAL: u32 = 5;

pub const CQLIB_CLASSICAL_EXPR_UNARY: u32 = 6;

pub const CQLIB_CLASSICAL_EXPR_BINARY: u32 = 7;

pub const CQLIB_CLASSICAL_EXPR_COMPARE: u32 = 8;

pub const CQLIB_CLASSICAL_EXPR_CAST: u32 = 9;

pub const CQLIB_CLASSICAL_EXPR_SELECT: u32 = 10;

pub const CQLIB_CLASSICAL_EXPR_EXTRACT_BIT: u32 = 11;

pub const CQLIB_CLASSICAL_EXPR_EXTRACT_BITS: u32 = 12;

pub const CQLIB_CLASSICAL_EXPR_CONCAT: u32 = 13;

pub const CQLIB_CLASSICAL_EXPR_PACK_BITS: u32 = 14;

/// C-side snapshot of a [`ClassicalType`]: tag + width in bits.
#[repr(C)]
pub struct CClassicalType {
    /// One of `CQLIB_CLASSICAL_TYPE_*`.
    pub tag: u32,
    /// Type width in bits (1 for Bit/Bool).
    pub width: u32,
}

/// C-side snapshot of an immutable classical value referenced by an expression.
#[repr(C)]
pub struct CClassicalValueInfo {
    /// Circuit-local value table index.
    pub index: u32,
    /// One of `CQLIB_CLASSICAL_TYPE_*`.
    pub tag: u32,
    /// Type width in bits (1 for Bit/Bool).
    pub width: u32,
}

/// Converts a C (tag, width) pair into a core `ClassicalType`.
fn classical_type_from_tag(tag: u32, width: u32) -> Option<ClassicalType> {
    match tag {
        CQLIB_CLASSICAL_TYPE_BIT => Some(ClassicalType::Bit),
        CQLIB_CLASSICAL_TYPE_BOOL => Some(ClassicalType::Bool),
        CQLIB_CLASSICAL_TYPE_UINT => ClassicalType::uint(width),
        CQLIB_CLASSICAL_TYPE_BIT_VEC => ClassicalType::bit_vec(width),
        _ => None,
    }
}

/// Converts a core `ClassicalType` into a C (tag, width) pair.
pub(crate) fn classical_type_parts(ty: ClassicalType) -> (u32, u32) {
    match ty {
        ClassicalType::Bit => (CQLIB_CLASSICAL_TYPE_BIT, 1),
        ClassicalType::Bool => (CQLIB_CLASSICAL_TYPE_BOOL, 1),
        ClassicalType::UInt(width) => (CQLIB_CLASSICAL_TYPE_UINT, width.get()),
        ClassicalType::BitVec(width) => (CQLIB_CLASSICAL_TYPE_BIT_VEC, width.get()),
    }
}

/// Maps an expression node kind to its C tag.
fn expr_kind_tag(kind: &ClassicalExprKind) -> u32 {
    match kind {
        ClassicalExprKind::Var(_) => CQLIB_CLASSICAL_EXPR_VAR,
        ClassicalExprKind::Value(_) => CQLIB_CLASSICAL_EXPR_VALUE,
        ClassicalExprKind::BoolLiteral(_) => CQLIB_CLASSICAL_EXPR_BOOL_LITERAL,
        ClassicalExprKind::BitLiteral(_) => CQLIB_CLASSICAL_EXPR_BIT_LITERAL,
        ClassicalExprKind::UIntLiteral { .. } => CQLIB_CLASSICAL_EXPR_UINT_LITERAL,
        ClassicalExprKind::BitVecLiteral { .. } => CQLIB_CLASSICAL_EXPR_BIT_VEC_LITERAL,
        ClassicalExprKind::Unary { .. } => CQLIB_CLASSICAL_EXPR_UNARY,
        ClassicalExprKind::Binary { .. } => CQLIB_CLASSICAL_EXPR_BINARY,
        ClassicalExprKind::Compare { .. } => CQLIB_CLASSICAL_EXPR_COMPARE,
        ClassicalExprKind::Cast { .. } => CQLIB_CLASSICAL_EXPR_CAST,
        ClassicalExprKind::Select { .. } => CQLIB_CLASSICAL_EXPR_SELECT,
        ClassicalExprKind::ExtractBit { .. } => CQLIB_CLASSICAL_EXPR_EXTRACT_BIT,
        ClassicalExprKind::ExtractBits { .. } => CQLIB_CLASSICAL_EXPR_EXTRACT_BITS,
        ClassicalExprKind::Concat { .. } => CQLIB_CLASSICAL_EXPR_CONCAT,
        ClassicalExprKind::PackBits { .. } => CQLIB_CLASSICAL_EXPR_PACK_BITS,
    }
}

/// Clones the expression behind a handle, or `None` for NULL.
fn take_expr(ptr: *const CClassicalExpr) -> Option<ClassicalExpr> {
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { (*ptr).inner.clone() })
}

/// Boxes an expression into a new owned C handle.
fn boxed_expr(expr: ClassicalExpr) -> *mut CClassicalExpr {
    Box::into_raw(Box::new(CClassicalExpr { inner: expr }))
}

// =====  Section 1.1: classical variables and measurement  =====

/// Allocate a mutable classical variable owned by the circuit.
///
/// `ty_tag` is one of `CQLIB_CLASSICAL_TYPE_*`; `width` is the bit width used
/// for `UInt`/`BitVec` (zero width is rejected). Returns NULL on NULL circuit
/// or invalid type parameters.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_var(ptr: *mut CCircuit, ty_tag: u32, width: u32) -> *mut CClassicalVar {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let Some(ty) = classical_type_from_tag(ty_tag, width) else {
        return std::ptr::null_mut();
    };
    let wrapper = unsafe { &mut *ptr };
    let var = wrapper.inner.var(ty);
    Box::into_raw(Box::new(CClassicalVar { inner: var }))
}

/// Free a classical variable handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn classical_var_free(ptr: *mut CClassicalVar) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Return the circuit-local variable id, or `UINT32_MAX` for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn classical_var_id(ptr: *const CClassicalVar) -> u32 {
    if ptr.is_null() {
        return u32::MAX;
    }
    unsafe { (*ptr).inner.id() }
}

/// Return the circuit-local variable table index, or `UINT32_MAX` for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn classical_var_index(ptr: *const CClassicalVar) -> u32 {
    if ptr.is_null() {
        return u32::MAX;
    }
    unsafe { (*ptr).inner.index() }
}

/// Write the variable type (tag + width) to the out-parameters.
/// Returns 0 on success, -1 on NULL pointers.
#[unsafe(no_mangle)]
pub extern "C" fn classical_var_ty(
    ptr: *const CClassicalVar,
    tag: *mut u32,
    width: *mut u32,
) -> i32 {
    if ptr.is_null() || tag.is_null() || width.is_null() {
        return -1;
    }
    let (ty_tag, ty_width) = classical_type_parts(unsafe { (*ptr).inner.ty() });
    unsafe {
        *tag = ty_tag;
        *width = ty_width;
    }
    0
}

/// Create a new expression that reads the current runtime value of the
/// variable. Returns NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn classical_var_expr(ptr: *const CClassicalVar) -> *mut CClassicalExpr {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    boxed_expr(unsafe { (*ptr).inner.expr() })
}

/// Measure several qubits and return an expression reading the immutable
/// `BitVec` result. The first qubit maps to bit index 0 (least-significant).
/// Returns NULL on error (NULL pointers, empty list, qubit out of bounds).
#[unsafe(no_mangle)]
pub extern "C" fn circuit_measure_bits(
    ptr: *mut CCircuit,
    qubits: *const u32,
    len: usize,
) -> *mut CClassicalExpr {
    if ptr.is_null() || qubits.is_null() || len == 0 {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &mut *ptr };
    let slice = unsafe { std::slice::from_raw_parts(qubits, len) };
    let qubits: Vec<Qubit> = slice.iter().map(|&id| Qubit::new(id)).collect();
    match wrapper.inner.measure_bits(qubits) {
        Ok(measurement) => boxed_expr(measurement.expr()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Measure one qubit and store the result into `target` (a `Bit` variable).
/// Returns an expression reading the immutable measurement value, or NULL on
/// error (NULL pointers, qubit out of bounds, wrong target type).
#[unsafe(no_mangle)]
pub extern "C" fn circuit_measure_into(
    ptr: *mut CCircuit,
    qubit: u32,
    target: *const CClassicalVar,
) -> *mut CClassicalExpr {
    if ptr.is_null() || target.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &mut *ptr };
    let target = unsafe { (*target).inner };
    match wrapper.inner.measure_into(Qubit::new(qubit), target) {
        Ok(measurement) => boxed_expr(measurement.expr()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Measure several qubits and store the `BitVec` result into `target`.
/// The target width must equal the number of qubits. Returns an expression
/// reading the immutable measurement value, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_measure_bits_into(
    ptr: *mut CCircuit,
    qubits: *const u32,
    len: usize,
    target: *const CClassicalVar,
) -> *mut CClassicalExpr {
    if ptr.is_null() || qubits.is_null() || len == 0 || target.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &mut *ptr };
    let slice = unsafe { std::slice::from_raw_parts(qubits, len) };
    let qubits: Vec<Qubit> = slice.iter().map(|&id| Qubit::new(id)).collect();
    let target = unsafe { (*target).inner };
    match wrapper.inner.measure_bits_into(qubits, target) {
        Ok(measurement) => boxed_expr(measurement.expr()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return the number of classical variables allocated by the circuit, or 0
/// for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_classical_vars_len(ptr: *const CCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.classical_vars().len() }
}

/// Copy the circuit's classical variable types into `buffer` as (tag, width)
/// snapshots. Returns 0 on success, -1 on NULL pointers, -8 when `len` does
/// not equal `circuit_classical_vars_len`.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_classical_vars(
    ptr: *const CCircuit,
    buffer: *mut CClassicalType,
    len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return -1;
    }
    let types = unsafe { (*ptr).inner.classical_vars() };
    if types.len() != len {
        return -8;
    }
    for (i, ty) in types.iter().enumerate() {
        let (tag, width) = classical_type_parts(*ty);
        unsafe {
            *buffer.add(i) = CClassicalType { tag, width };
        }
    }
    0
}

/// Check that a classical variable belongs to this circuit and keeps its
/// recorded type. Returns 0 on success, -1 on NULL pointers, -3 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_validate_classical_var(
    ptr: *const CCircuit,
    var: *const CClassicalVar,
) -> i32 {
    if ptr.is_null() || var.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    let var = unsafe { (*var).inner };
    wrapper.inner.validate_classical_var(var).map_or(-3, |_| 0)
}

/// Check that every classical variable and value read by `expr` belongs to
/// this circuit. Returns 0 on success, -1 on NULL pointers, -3 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_validate_classical_expr(
    ptr: *const CCircuit,
    expr: *const CClassicalExpr,
) -> i32 {
    if ptr.is_null() || expr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    let expr = unsafe { &(*expr).inner };
    wrapper
        .inner
        .validate_classical_expr(expr)
        .map_or(-3, |_| 0)
}

// =====  Section 1.2: classical expressions and store  =====

/// Free a classical expression handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_free(ptr: *mut CClassicalExpr) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Create a `Bit` literal expression (false = 0, true = 1).
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_bit_literal(value: bool) -> *mut CClassicalExpr {
    boxed_expr(ClassicalExpr::bit_literal(value))
}

/// Create a `Bool` literal expression.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_bool_literal(value: bool) -> *mut CClassicalExpr {
    boxed_expr(ClassicalExpr::bool_literal(value))
}

/// Create a `UInt(width)` literal. The 128-bit value is passed as two 64-bit
/// halves (lo = least-significant). Returns NULL when width is zero, exceeds
/// 128, or the value does not fit in the width.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_uint_literal(width: u32, lo: u64, hi: u64) -> *mut CClassicalExpr {
    let value = u128::from(lo) | (u128::from(hi) << 64);
    match ClassicalExpr::uint_literal(width, value) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create a `BitVec(width)` literal. The 128-bit packed little-endian value is
/// passed as two 64-bit halves (lo = least-significant). Returns NULL when
/// width is zero, exceeds 128, or the value does not fit in the width.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_bit_vec_literal(
    width: u32,
    lo: u64,
    hi: u64,
) -> *mut CClassicalExpr {
    let value = u128::from(lo) | (u128::from(hi) << 64);
    match ClassicalExpr::bit_vec_literal(width, value) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create an expression that reads the current runtime value of `var`.
/// Returns NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_var(var: *const CClassicalVar) -> *mut CClassicalExpr {
    if var.is_null() {
        return std::ptr::null_mut();
    }
    boxed_expr(unsafe { (*var).inner.expr() })
}

/// Create a `not` expression over a `Bool` or `Bit` operand.
/// Returns NULL on NULL input or invalid operand type.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_not(expr: *const CClassicalExpr) -> *mut CClassicalExpr {
    let Some(expr) = take_expr(expr) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::try_not(expr) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create an `and` expression over matching `Bool` or matching `Bit`
/// operands. Returns NULL on NULL input or type mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_and(
    lhs: *const CClassicalExpr,
    rhs: *const CClassicalExpr,
) -> *mut CClassicalExpr {
    let (Some(lhs), Some(rhs)) = (take_expr(lhs), take_expr(rhs)) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::try_and(lhs, rhs) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create an `or` expression over matching `Bool` or matching `Bit` operands.
/// Returns NULL on NULL input or type mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_or(
    lhs: *const CClassicalExpr,
    rhs: *const CClassicalExpr,
) -> *mut CClassicalExpr {
    let (Some(lhs), Some(rhs)) = (take_expr(lhs), take_expr(rhs)) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::try_or(lhs, rhs) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create an `xor` expression over matching `Bool` or matching `Bit`
/// operands. Returns NULL on NULL input or type mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_xor(
    lhs: *const CClassicalExpr,
    rhs: *const CClassicalExpr,
) -> *mut CClassicalExpr {
    let (Some(lhs), Some(rhs)) = (take_expr(lhs), take_expr(rhs)) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::try_xor(lhs, rhs) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create an equality comparison over same-typed operands, producing `Bool`.
/// Returns NULL on NULL input or type mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_eq(
    lhs: *const CClassicalExpr,
    rhs: *const CClassicalExpr,
) -> *mut CClassicalExpr {
    let (Some(lhs), Some(rhs)) = (take_expr(lhs), take_expr(rhs)) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::eq(lhs, rhs) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create an inequality comparison over same-typed operands, producing `Bool`.
/// Returns NULL on NULL input or type mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_ne(
    lhs: *const CClassicalExpr,
    rhs: *const CClassicalExpr,
) -> *mut CClassicalExpr {
    let (Some(lhs), Some(rhs)) = (take_expr(lhs), take_expr(rhs)) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::ne(lhs, rhs) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create an unsigned less-than comparison over matching `UInt` operands,
/// producing `Bool`. Returns NULL on NULL input or invalid operand types.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_lt(
    lhs: *const CClassicalExpr,
    rhs: *const CClassicalExpr,
) -> *mut CClassicalExpr {
    let (Some(lhs), Some(rhs)) = (take_expr(lhs), take_expr(rhs)) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::lt(lhs, rhs) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create an unsigned less-than-or-equal comparison over matching `UInt`
/// operands, producing `Bool`. Returns NULL on NULL input or invalid types.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_le(
    lhs: *const CClassicalExpr,
    rhs: *const CClassicalExpr,
) -> *mut CClassicalExpr {
    let (Some(lhs), Some(rhs)) = (take_expr(lhs), take_expr(rhs)) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::le(lhs, rhs) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create an unsigned greater-than comparison over matching `UInt` operands,
/// producing `Bool`. Returns NULL on NULL input or invalid operand types.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_gt(
    lhs: *const CClassicalExpr,
    rhs: *const CClassicalExpr,
) -> *mut CClassicalExpr {
    let (Some(lhs), Some(rhs)) = (take_expr(lhs), take_expr(rhs)) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::gt(lhs, rhs) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create an unsigned greater-than-or-equal comparison over matching `UInt`
/// operands, producing `Bool`. Returns NULL on NULL input or invalid types.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_ge(
    lhs: *const CClassicalExpr,
    rhs: *const CClassicalExpr,
) -> *mut CClassicalExpr {
    let (Some(lhs), Some(rhs)) = (take_expr(lhs), take_expr(rhs)) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::ge(lhs, rhs) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create a ternary select: `condition ? then_expr : else_expr`. The condition
/// must be `Bool` and both branches must have the same type. Returns NULL on
/// NULL input or invalid operand types.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_select(
    condition: *const CClassicalExpr,
    then_expr: *const CClassicalExpr,
    else_expr: *const CClassicalExpr,
) -> *mut CClassicalExpr {
    let (Some(condition), Some(then_expr), Some(else_expr)) = (
        take_expr(condition),
        take_expr(then_expr),
        take_expr(else_expr),
    ) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::select(condition, then_expr, else_expr) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Extract one bit (index 0 = least-significant) from a `UInt` or `BitVec`
/// expression, producing `Bit`. Returns NULL on NULL input, non-integer
/// operand, or out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_extract_bit(
    value: *const CClassicalExpr,
    index: u32,
) -> *mut CClassicalExpr {
    let Some(value) = take_expr(value) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::extract_bit(value, index) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Extract a contiguous bit range `[offset, offset + width)` from a `UInt` or
/// `BitVec` expression, producing `BitVec(width)`. Offset 0 starts at the
/// least-significant bit. Returns NULL on NULL input or invalid range.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_extract_bits(
    value: *const CClassicalExpr,
    offset: u32,
    width: u32,
) -> *mut CClassicalExpr {
    let Some(value) = take_expr(value) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::extract_bits(value, offset, width) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Pack single-bit expressions into a `BitVec`; the first bit becomes output
/// bit index 0. `bits` is an array of expression handles. Returns NULL on NULL
/// entries, empty input, or non-`Bit` operands.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_pack_bits(
    bits: *const *const CClassicalExpr,
    len: usize,
) -> *mut CClassicalExpr {
    let mut exprs = Vec::with_capacity(len);
    if !bits.is_null() {
        for i in 0..len {
            match take_expr(unsafe { *bits.add(i) }) {
                Some(expr) => exprs.push(expr),
                None => return std::ptr::null_mut(),
            }
        }
    }
    match ClassicalExpr::pack_bits(exprs) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Concatenate `Bit`/`BitVec` expressions into a larger `BitVec`; the first
/// part occupies the least-significant output bits. `parts` is an array of
/// expression handles. Returns NULL on NULL entries, empty input, or invalid
/// part types.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_concat(
    parts: *const *const CClassicalExpr,
    len: usize,
) -> *mut CClassicalExpr {
    let mut exprs = Vec::with_capacity(len);
    if !parts.is_null() {
        for i in 0..len {
            match take_expr(unsafe { *parts.add(i) }) {
                Some(expr) => exprs.push(expr),
                None => return std::ptr::null_mut(),
            }
        }
    }
    match ClassicalExpr::concat(exprs) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Explicitly cast a `Bit` expression to `Bool`. Returns NULL on NULL input
/// or non-`Bit` operand.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_bit_to_bool(expr: *const CClassicalExpr) -> *mut CClassicalExpr {
    let Some(expr) = take_expr(expr) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::bit_to_bool(expr) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Explicitly cast a `BitVec` expression to a little-endian `UInt` of the same
/// width. Returns NULL on NULL input or non-`BitVec` operand.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_bit_vec_to_uint(
    expr: *const CClassicalExpr,
) -> *mut CClassicalExpr {
    let Some(expr) = take_expr(expr) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::bit_vec_to_uint(expr) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Convert this `Bit` expression to `Bool` (alias of `bit_to_bool`).
/// Returns NULL on NULL input or non-`Bit` operand.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_to_bool(expr: *const CClassicalExpr) -> *mut CClassicalExpr {
    let Some(expr) = take_expr(expr) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::to_bool(expr) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Convert this `BitVec` expression to a little-endian `UInt` (alias of
/// `bit_vec_to_uint`). Returns NULL on NULL input or non-`BitVec` operand.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_to_uint(expr: *const CClassicalExpr) -> *mut CClassicalExpr {
    let Some(expr) = take_expr(expr) else {
        return std::ptr::null_mut();
    };
    match ClassicalExpr::to_uint(expr) {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return a structurally simplified copy of the expression. Simplification
/// eliminates literal-driven redundancies without evaluating runtime reads.
/// Returns NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_simplified(ptr: *const CClassicalExpr) -> *mut CClassicalExpr {
    let Some(expr) = take_expr(ptr) else {
        return std::ptr::null_mut();
    };
    boxed_expr(expr.simplified())
}

/// Write the expression node kind tag (one of `CQLIB_CLASSICAL_EXPR_*`) to
/// `kind`. Returns 0 on success, -1 on NULL pointers.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_kind(ptr: *const CClassicalExpr, kind: *mut u32) -> i32 {
    if ptr.is_null() || kind.is_null() {
        return -1;
    }
    unsafe {
        *kind = expr_kind_tag((*ptr).inner.kind());
    }
    0
}

/// Write the expression type (tag + width) to the out-parameters.
/// Returns 0 on success, -1 on NULL pointers.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_ty(
    ptr: *const CClassicalExpr,
    tag: *mut u32,
    width: *mut u32,
) -> i32 {
    if ptr.is_null() || tag.is_null() || width.is_null() {
        return -1;
    }
    let (ty_tag, ty_width) = classical_type_parts(unsafe { (*ptr).inner.ty() });
    unsafe {
        *tag = ty_tag;
        *width = ty_width;
    }
    0
}

/// Return the number of distinct immutable classical values read by the
/// expression, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_values_len(ptr: *const CClassicalExpr) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.values().len() }
}

/// Copy snapshots (index, tag, width) of every immutable classical value read
/// by the expression into `buffer`. Returns 0 on success, -1 on NULL pointers,
/// -8 when `len` does not equal `classical_expr_values_len`.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_values(
    ptr: *const CClassicalExpr,
    buffer: *mut CClassicalValueInfo,
    len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return -1;
    }
    let values = unsafe { (*ptr).inner.values() };
    if values.len() != len {
        return -8;
    }
    for (i, value) in values.iter().enumerate() {
        let (tag, width) = classical_type_parts(value.ty());
        let info = CClassicalValueInfo {
            index: value.index(),
            tag,
            width,
        };
        unsafe {
            *buffer.add(i) = info;
        }
    }
    0
}

/// Return the number of distinct classical variables read by the expression,
/// or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_vars_len(ptr: *const CClassicalExpr) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.vars().len() }
}

/// Fill `buffer` with owned handles (of type `CClassicalVar*`) for every
/// distinct classical variable read by the expression, in ascending id order.
/// Each returned handle must be released with `classical_var_free`. Returns 0
/// on success, -1 on NULL pointers, -8 when `len` does not equal
/// `classical_expr_vars_len`.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_vars(
    ptr: *const CClassicalExpr,
    buffer: *mut *mut CClassicalVar,
    len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return -1;
    }
    let vars = unsafe { (*ptr).inner.vars() };
    if vars.len() != len {
        return -8;
    }
    for (i, var) in vars.iter().enumerate() {
        let handle = Box::into_raw(Box::new(CClassicalVar { inner: *var }));
        unsafe {
            *buffer.add(i) = handle;
        }
    }
    0
}

/// Store the runtime value of `value` into the mutable variable `target`.
/// The expression type must match the target type. Returns 0 on success, -1
/// on NULL pointers, -3 on validation failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_store(
    ptr: *mut CCircuit,
    target: *const CClassicalVar,
    value: *const CClassicalExpr,
) -> i32 {
    if ptr.is_null() || target.is_null() || value.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let target = unsafe { (*target).inner };
    let value = unsafe { (*value).inner.clone() };
    wrapper.inner.store(target, value).map_or(-3, |_| 0)
}

// =====  Section 1.3: expression predicates, type literals, and id remapping  =====

/// Returns 1 when the expression is the `Bit` literal `true` (1), 0 otherwise,
/// or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_is_bit_true(ptr: *const CClassicalExpr) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    i32::from(unsafe { (*ptr).inner.is_bit_true() })
}

/// Returns 1 when the expression is the `Bit` literal `false` (0), 0 otherwise,
/// or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_is_bit_false(ptr: *const CClassicalExpr) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    i32::from(unsafe { (*ptr).inner.is_bit_false() })
}

/// Returns 1 when the expression is the `Bool` literal `true`, 0 otherwise,
/// or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_is_bool_true(ptr: *const CClassicalExpr) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    i32::from(unsafe { (*ptr).inner.is_bool_true() })
}

/// Returns 1 when the expression is the `Bool` literal `false`, 0 otherwise,
/// or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_is_bool_false(ptr: *const CClassicalExpr) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    i32::from(unsafe { (*ptr).inner.is_bool_false() })
}

/// Create the zero literal of a classical type. `ty_tag` is one of
/// `CQLIB_CLASSICAL_TYPE_*`; `width` is the bit width used for `UInt`/`BitVec`.
/// Returns NULL on invalid type parameters or a width exceeding the 128-bit
/// literal representation.
#[unsafe(no_mangle)]
pub extern "C" fn classical_type_zero_literal(ty_tag: u32, width: u32) -> *mut CClassicalExpr {
    let Some(ty) = classical_type_from_tag(ty_tag, width) else {
        return std::ptr::null_mut();
    };
    match ty.zero_literal() {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create the one literal of a classical type. `ty_tag` is one of
/// `CQLIB_CLASSICAL_TYPE_*`; `width` is the bit width used for `UInt`/`BitVec`.
/// Returns NULL on invalid type parameters or a width exceeding the 128-bit
/// literal representation.
#[unsafe(no_mangle)]
pub extern "C" fn classical_type_one_literal(ty_tag: u32, width: u32) -> *mut CClassicalExpr {
    let Some(ty) = classical_type_from_tag(ty_tag, width) else {
        return std::ptr::null_mut();
    };
    match ty.one_literal() {
        Ok(expr) => boxed_expr(expr),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return a copy of the expression with circuit-local classical ids remapped.
///
/// `old_var_ids[i]` maps the variable with that id to id `new_var_ids[i]`, and
/// `old_value_indices[i]` maps the immutable value with that index to index
/// `new_value_indices[i]`. Entries whose old id does not occur in the
/// expression are ignored, but every variable and value actually read by the
/// expression must be covered, otherwise the remap fails. Returns a new
/// expression handle, or NULL on NULL inputs (including NULL arrays for
/// non-zero lengths) or a missing mapping.
#[unsafe(no_mangle)]
pub extern "C" fn classical_expr_remap_classical_ids(
    ptr: *const CClassicalExpr,
    old_var_ids: *const u32,
    new_var_ids: *const u32,
    var_len: usize,
    old_value_indices: *const u32,
    new_value_indices: *const u32,
    value_len: usize,
) -> *mut CClassicalExpr {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    if var_len > 0 && (old_var_ids.is_null() || new_var_ids.is_null()) {
        return std::ptr::null_mut();
    }
    if value_len > 0 && (old_value_indices.is_null() || new_value_indices.is_null()) {
        return std::ptr::null_mut();
    }
    let expr = unsafe { &(*ptr).inner };
    let vars = expr.vars();
    let mut var_map: HashMap<ClassicalVar, ClassicalVar> = HashMap::with_capacity(var_len);
    for i in 0..var_len {
        let (old, new) = (unsafe { *old_var_ids.add(i) }, unsafe {
            *new_var_ids.add(i)
        });
        if let Some(var) = vars.iter().find(|var| var.id() == old) {
            var_map.insert(*var, ClassicalVar::new(var.circuit_id(), new, var.ty()));
        }
    }
    let values = expr.values();
    let mut value_map: HashMap<ClassicalValue, ClassicalValue> = HashMap::with_capacity(value_len);
    for i in 0..value_len {
        let (old, new) = (unsafe { *old_value_indices.add(i) }, unsafe {
            *new_value_indices.add(i)
        });
        if let Some(value) = values.iter().find(|value| value.index() == old) {
            value_map.insert(
                *value,
                ClassicalValue::new(value.circuit_id(), new, value.ty()),
            );
        }
    }
    match expr.remap_classical_ids(&var_map, &value_map) {
        Ok(remapped) => boxed_expr(remapped),
        Err(_) => std::ptr::null_mut(),
    }
}
