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

//! C ABI for structured classical control flow: `if`, `if`/`else`, `while`,
//! unsigned `for`, exact-value `switch`, `break`, `continue`, and raw control
//! operation re-append.
//!
//! The Rust builders take closures that append operations into a temporary
//! body region of the circuit. C has no closures, so every builder in this
//! module drives the same core "external control body transaction" machinery
//! through a C callback ([`CqlibBodyFn`]): the callback receives the owning
//! circuit pointer plus an opaque `user_data` pointer and appends body
//! operations with the regular circuit functions (`circuit_h`,
//! `circuit_measure`, ...). When the callback returns, the appended
//! operations are captured into the control body.
//!
//! Callback contract:
//!
//! - The `CCircuit` pointer passed to the callback is the same pointer that
//!   was passed to the control-flow function. It is only valid for the
//!   duration of the callback; body operations must be appended through it.
//! - `user_data` is passed through untouched and may be NULL.
//! - Return `0` on success. Any non-zero return aborts the whole control
//!   operation: everything the callback appended is rolled back and the
//!   value is returned to the original caller.

use crate::circuit::CCircuit;
use crate::circuit::classical::{CClassicalExpr, CClassicalVar};
use cqlib_core::circuit::{
    ClassicalControlOp, ClassicalExpr, ControlBody, ControlBodyTransaction, ExternalControlScope,
    ForOp, IfOp, Instruction, SwitchCase, SwitchOp, WhileOp,
};
use std::os::raw::c_void;

/// Callback that fills one structured control-flow body.
///
/// Contract:
///
/// - `circuit` is the same `CCircuit` pointer that was passed to the
///   control-flow function being built. It is valid only for the duration of
///   the callback. Body operations must be appended through it using the
///   regular circuit functions (`circuit_h`, `circuit_x`, `circuit_measure`,
///   nested control-flow builders, ...).
/// - `user_data` is the pointer that was passed to the control-flow function.
///   It is never dereferenced by the bindings and may be NULL.
/// - Return `0` on success. Any non-zero return aborts the whole control
///   operation: all operations appended by the callback are rolled back and
///   the returned value is propagated to the caller of the control-flow
///   function. Returning the error code of a failed gate call (for example
///   `return circuit_x(circuit, 0);`) is the idiomatic pattern.
pub type CqlibBodyFn = Option<extern "C" fn(circuit: *mut CCircuit, user_data: *mut c_void) -> i32>;

/// Runs one callback body region and captures the appended operations.
///
/// `ptr` must be a valid circuit with `transaction` open. On a non-zero
/// callback status the half-built region is left for the caller to roll back
/// with `rollback_control_body_transaction`.
fn run_body_callback(
    ptr: *mut CCircuit,
    transaction: &ControlBodyTransaction,
    scope: ExternalControlScope,
    callback: extern "C" fn(*mut CCircuit, *mut c_void) -> i32,
    user_data: *mut c_void,
) -> Result<ControlBody, i32> {
    unsafe {
        (*ptr).inner.enter_external_control_body(transaction, scope);
    }
    let status = callback(ptr, user_data);
    if status != 0 {
        return Err(status);
    }
    Ok(unsafe { (*ptr).inner.finish_external_control_body(transaction) })
}

/// Commits the transaction on success or rolls the circuit back on failure.
fn close_control_region<T>(
    ptr: *mut CCircuit,
    transaction: ControlBodyTransaction,
    result: Result<T, i32>,
) -> Result<T, i32> {
    if result.is_ok() {
        unsafe {
            (*ptr).inner.commit_control_body_transaction(transaction);
        }
    } else {
        unsafe {
            (*ptr).inner.rollback_control_body_transaction(transaction);
        }
    }
    result
}

/// Builds an `if` operation from captured bodies and appends it.
fn append_if_op(
    ptr: *mut CCircuit,
    condition: ClassicalExpr,
    then_body: ControlBody,
    else_body: Option<ControlBody>,
) -> Result<(), i32> {
    let op = IfOp::new(condition, then_body, else_body).map_err(|_| -3)?;
    append_control_op(ptr, ClassicalControlOp::If(op))
}

/// Appends a raw classical-control operation.
fn append_control_op(ptr: *mut CCircuit, op: ClassicalControlOp) -> Result<(), i32> {
    unsafe { (*ptr).inner.append_control(op) }.map_err(|_| -3)
}

/// Appends a structured `if` operation controlled by a boolean classical expression.
///
/// `body_fn` is invoked once with `ptr` and `user_data`; every operation it
/// appends becomes the `then` body of the `if` operation. See [`CqlibBodyFn`]
/// for the callback contract.
///
/// Returns `0` on success, `-1` for NULL arguments, `-3` when the condition
/// fails validation (for example a non-`Bool` type or handles from another
/// circuit), or the non-zero status returned by `body_fn` after rollback.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_if(
    ptr: *mut CCircuit,
    condition: *const CClassicalExpr,
    body_fn: CqlibBodyFn,
    user_data: *mut c_void,
) -> i32 {
    let Some(body_fn) = body_fn else {
        return -1;
    };
    if ptr.is_null() || condition.is_null() {
        return -1;
    }
    let condition = unsafe { (*condition).inner.clone() };
    if unsafe { (*ptr).inner.validate_classical_expr(&condition) }.is_err() {
        return -3;
    }

    let transaction = unsafe { (*ptr).inner.begin_control_body_transaction() };
    let result = run_body_callback(
        ptr,
        &transaction,
        ExternalControlScope::Branch,
        body_fn,
        user_data,
    )
    .and_then(|then_body| append_if_op(ptr, condition, then_body, None));
    match close_control_region(ptr, transaction, result) {
        Ok(()) => 0,
        Err(code) => code,
    }
}

/// Appends a structured `if`/`else` operation controlled by a boolean classical expression.
///
/// `then_fn` runs first, then `else_fn`; each receives `ptr` and `user_data`
/// and its appended operations become the corresponding branch body.
///
/// Returns `0` on success, `-1` for NULL arguments, `-3` on validation failure,
/// or the non-zero status returned by either callback after rollback.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_if_else(
    ptr: *mut CCircuit,
    condition: *const CClassicalExpr,
    then_fn: CqlibBodyFn,
    else_fn: CqlibBodyFn,
    user_data: *mut c_void,
) -> i32 {
    let Some(then_fn) = then_fn else {
        return -1;
    };
    let Some(else_fn) = else_fn else {
        return -1;
    };
    if ptr.is_null() || condition.is_null() {
        return -1;
    }
    let condition = unsafe { (*condition).inner.clone() };
    if unsafe { (*ptr).inner.validate_classical_expr(&condition) }.is_err() {
        return -3;
    }

    let transaction = unsafe { (*ptr).inner.begin_control_body_transaction() };
    let result = run_body_callback(
        ptr,
        &transaction,
        ExternalControlScope::Branch,
        then_fn,
        user_data,
    )
    .and_then(|then_body| {
        run_body_callback(
            ptr,
            &transaction,
            ExternalControlScope::Branch,
            else_fn,
            user_data,
        )
        .and_then(|else_body| append_if_op(ptr, condition, then_body, Some(else_body)))
    });
    match close_control_region(ptr, transaction, result) {
        Ok(()) => 0,
        Err(code) => code,
    }
}

/// Appends a structured `while` loop controlled by a boolean classical expression.
///
/// `body_fn` is invoked once with `ptr` and `user_data`; its appended
/// operations become the loop body. The condition is re-evaluated at runtime.
/// `circuit_break_loop`/`circuit_continue_loop` may be called from the
/// callback, but only as the final operation of the body (terminal control
/// transfer).
///
/// Returns `0` on success, `-1` for NULL arguments, `-3` on validation failure,
/// or the non-zero status returned by `body_fn` after rollback.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_while(
    ptr: *mut CCircuit,
    condition: *const CClassicalExpr,
    body_fn: CqlibBodyFn,
    user_data: *mut c_void,
) -> i32 {
    let Some(body_fn) = body_fn else {
        return -1;
    };
    if ptr.is_null() || condition.is_null() {
        return -1;
    }
    let condition = unsafe { (*condition).inner.clone() };
    if unsafe { (*ptr).inner.validate_classical_expr(&condition) }.is_err() {
        return -3;
    }

    let transaction = unsafe { (*ptr).inner.begin_control_body_transaction() };
    let result = run_body_callback(
        ptr,
        &transaction,
        ExternalControlScope::Loop,
        body_fn,
        user_data,
    )
    .and_then(|body| {
        let op = WhileOp::new(condition, body).map_err(|_| -3)?;
        append_control_op(ptr, ClassicalControlOp::While(op))
    });
    match close_control_region(ptr, transaction, result) {
        Ok(()) => 0,
        Err(code) => code,
    }
}

/// Appends an unsigned runtime range loop with half-open `[start, stop)` semantics.
///
/// Parameter order follows the core API: `var`, `start`, `stop`, `step`.
/// `var` must be a `UInt` variable of this circuit; `start`, `stop`, and
/// `step` must be `UInt` expressions of the same width. `body_fn` is invoked
/// once with `ptr` and `user_data`; the loop variable expression can be
/// rebuilt from `var` with `classical_expr_var` when needed inside the body.
/// `circuit_break_loop`/`circuit_continue_loop` are allowed in the callback
/// only as the final operation of the body.
///
/// Returns `0` on success, `-1` for NULL arguments, `-3` on validation failure
/// (type mismatch or foreign handles), or the non-zero status returned by
/// `body_fn` after rollback.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn circuit_for_uint(
    ptr: *mut CCircuit,
    var: *const CClassicalVar,
    start: *const CClassicalExpr,
    stop: *const CClassicalExpr,
    step: *const CClassicalExpr,
    body_fn: CqlibBodyFn,
    user_data: *mut c_void,
) -> i32 {
    let Some(body_fn) = body_fn else {
        return -1;
    };
    if ptr.is_null() || var.is_null() || start.is_null() || stop.is_null() || step.is_null() {
        return -1;
    }
    let var = unsafe { (*var).inner };
    let start = unsafe { (*start).inner.clone() };
    let stop = unsafe { (*stop).inner.clone() };
    let step = unsafe { (*step).inner.clone() };
    let valid = unsafe {
        (*ptr)
            .inner
            .validate_classical_var(var)
            .and_then(|()| (*ptr).inner.validate_classical_expr(&start))
            .and_then(|()| (*ptr).inner.validate_classical_expr(&stop))
            .and_then(|()| (*ptr).inner.validate_classical_expr(&step))
    };
    if valid.is_err() {
        return -3;
    }

    let transaction = unsafe { (*ptr).inner.begin_control_body_transaction() };
    let result = run_body_callback(
        ptr,
        &transaction,
        ExternalControlScope::Loop,
        body_fn,
        user_data,
    )
    .and_then(|body| {
        let op = ForOp::new(var, start, stop, step, body).map_err(|_| -3)?;
        append_control_op(ptr, ClassicalControlOp::For(op))
    });
    match close_control_region(ptr, transaction, result) {
        Ok(()) => 0,
        Err(code) => code,
    }
}

/// Appends a structured exact-value `switch` operation over a `UInt` expression.
///
/// `target` must be a `UInt` expression. Case values are `u128` split into
/// two `u64` halves: `value = lo | (hi << 64)`. Every value must fit the
/// target width and appear at most once. `case_bodies[i]` builds the body of
/// `case_values[i]`; `default_body` may be NULL, in which case the switch has
/// no default branch. All callbacks receive the same `user_data`.
///
/// Returns `0` on success, `-1` for NULL arguments (including a NULL case
/// body when `case_count > 0`), `-3` on validation failure (non-`UInt`
/// target, duplicate or out-of-range case values), or the first non-zero
/// status returned by a callback after rollback.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn circuit_switch(
    ptr: *mut CCircuit,
    target: *const CClassicalExpr,
    case_values_lo: *const u64,
    case_values_hi: *const u64,
    case_bodies: *const CqlibBodyFn,
    case_count: usize,
    default_body: CqlibBodyFn,
    user_data: *mut c_void,
) -> i32 {
    if ptr.is_null() || target.is_null() {
        return -1;
    }
    if case_count > 0
        && (case_values_lo.is_null() || case_values_hi.is_null() || case_bodies.is_null())
    {
        return -1;
    }
    let target = unsafe { (*target).inner.clone() };
    if unsafe { (*ptr).inner.validate_classical_expr(&target) }.is_err() {
        return -3;
    }

    let transaction = unsafe { (*ptr).inner.begin_control_body_transaction() };
    let mut cases = Vec::with_capacity(case_count);
    let mut captured = Ok(());
    for index in 0..case_count {
        let case_fn = unsafe { *case_bodies.add(index) };
        let Some(case_fn) = case_fn else {
            captured = Err(-1);
            break;
        };
        let value = unsafe {
            (*case_values_lo.add(index) as u128) | ((*case_values_hi.add(index) as u128) << 64)
        };
        match run_body_callback(
            ptr,
            &transaction,
            ExternalControlScope::Switch,
            case_fn,
            user_data,
        ) {
            Ok(body) => cases.push(SwitchCase::new(value, body)),
            Err(code) => {
                captured = Err(code);
                break;
            }
        }
    }

    let result = captured.and_then(|()| {
        let default_body = match default_body {
            Some(default_fn) => Some(run_body_callback(
                ptr,
                &transaction,
                ExternalControlScope::Switch,
                default_fn,
                user_data,
            )?),
            None => None,
        };
        let op = SwitchOp::new(target, cases, default_body).map_err(|_| -3)?;
        append_control_op(ptr, ClassicalControlOp::Switch(op))
    });
    match close_control_region(ptr, transaction, result) {
        Ok(()) => 0,
        Err(code) => code,
    }
}

/// Appends a `break` to the nearest enclosing loop or switch body.
///
/// Legal only inside a `circuit_while`, `circuit_for_uint`, or
/// `circuit_switch` body callback (or a body nested inside one), and only as
/// the final operation of that body (terminal control transfer).
///
/// Returns `0` on success, `-1` for a NULL circuit, `-3` when no enclosing
/// loop or switch body is active or the transfer is not terminal.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_break_loop(ptr: *mut CCircuit) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.break_loop() }.map_or(-3, |()| 0)
}

/// Appends a `continue` to the nearest enclosing loop body.
///
/// Legal only inside a `circuit_while` or `circuit_for_uint` body callback
/// (or a body nested inside one), and only as the final operation of that
/// body (terminal control transfer).
///
/// Returns `0` on success, `-1` for a NULL circuit, `-3` when no enclosing
/// loop body is active or the transfer is not terminal.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_continue_loop(ptr: *mut CCircuit) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.continue_loop() }.map_or(-3, |()| 0)
}

/// Appends a copy of an existing classical-control operation.
///
/// `ClassicalControlOp` values cannot be constructed directly from C yet
/// (they require operation handles); this function takes the control
/// operation stored at `index` in `source`'s operation list and appends a
/// validated copy to `ptr`. The same circuit may be passed as both `ptr` and
/// `source`, which duplicates one of its control operations. Cross-circuit
/// copies require the operation to be self-contained (no classical handles
/// from `source`).
///
/// Returns `0` on success, `-1` for NULL arguments, `-8` when `index` is out
/// of range or the operation at `index` is not a control-flow operation,
/// `-3` when the copy fails validation (foreign classical handles, illegal
/// `break`/`continue` scoping, or unknown qubits).
#[unsafe(no_mangle)]
pub extern "C" fn circuit_append_control(
    ptr: *mut CCircuit,
    source: *const CCircuit,
    index: usize,
) -> i32 {
    if ptr.is_null() || source.is_null() {
        return -1;
    }
    let op = unsafe {
        let operations = (*source).inner.operations();
        let Some(operation) = operations.get(index) else {
            return -8;
        };
        let Instruction::ClassicalControl(op) = &operation.instruction else {
            return -8;
        };
        op.clone()
    };
    append_control_op(ptr, op).map_or(-3, |()| 0)
}
