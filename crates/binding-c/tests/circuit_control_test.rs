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

//! FFI tests for structured classical control flow (callback-based bodies).

use binding_c::circuit::{
    CCircuit, CClassicalExpr, CClassicalVar, circuit_append_control, circuit_break_loop,
    circuit_continue_loop, circuit_depth, circuit_for_uint, circuit_free, circuit_h, circuit_if,
    circuit_if_else, circuit_measure, circuit_new, circuit_num_operations, circuit_switch,
    circuit_validate, circuit_while, circuit_x, circuit_z,
};
use cqlib_core::circuit::{ClassicalExpr, ClassicalType};
use std::os::raw::c_void;

fn bool_expr(value: bool) -> *mut CClassicalExpr {
    Box::into_raw(Box::new(CClassicalExpr {
        inner: ClassicalExpr::bool_literal(value),
    }))
}

fn uint_expr(width: u32, value: u128) -> *mut CClassicalExpr {
    Box::into_raw(Box::new(CClassicalExpr {
        inner: ClassicalExpr::uint_literal(width, value).unwrap(),
    }))
}

fn drop_expr(ptr: *mut CClassicalExpr) {
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr) });
    }
}

/// Counting callback context shared through `user_data`.
struct BodyCtx {
    calls: u32,
}

extern "C" fn add_h_then_x(circuit: *mut CCircuit, user_data: *mut c_void) -> i32 {
    bump(user_data);
    let rc = circuit_h(circuit, 0);
    if rc != 0 {
        return rc;
    }
    circuit_x(circuit, 0)
}

extern "C" fn add_z(circuit: *mut CCircuit, user_data: *mut c_void) -> i32 {
    bump(user_data);
    circuit_z(circuit, 1)
}

extern "C" fn add_measure_then_x(circuit: *mut CCircuit, user_data: *mut c_void) -> i32 {
    bump(user_data);
    let rc = circuit_measure(circuit, 0);
    if rc != 0 {
        return rc;
    }
    circuit_x(circuit, 0)
}

extern "C" fn loop_body_with_continue(circuit: *mut CCircuit, user_data: *mut c_void) -> i32 {
    bump(user_data);
    let rc = circuit_h(circuit, 0);
    if rc != 0 {
        return rc;
    }
    let rc = circuit_x(circuit, 0);
    if rc != 0 {
        return rc;
    }
    // Terminal continue: legal as the final operation of a loop body.
    circuit_continue_loop(circuit)
}

extern "C" fn loop_body_with_break(circuit: *mut CCircuit, user_data: *mut c_void) -> i32 {
    bump(user_data);
    let rc = circuit_h(circuit, 1);
    if rc != 0 {
        return rc;
    }
    // Terminal break: legal as the final operation of a loop body.
    circuit_break_loop(circuit)
}

extern "C" fn loop_body_non_terminal_continue(
    circuit: *mut CCircuit,
    user_data: *mut c_void,
) -> i32 {
    bump(user_data);
    let rc = circuit_continue_loop(circuit);
    if rc != 0 {
        return rc;
    }
    circuit_x(circuit, 0)
}

extern "C" fn failing_body(circuit: *mut CCircuit, user_data: *mut c_void) -> i32 {
    bump(user_data);
    circuit_h(circuit, 0);
    circuit_h(circuit, 99) // qubit out of bounds -> -2
}

extern "C" fn nested_if_body(circuit: *mut CCircuit, user_data: *mut c_void) -> i32 {
    bump(user_data);
    let condition = bool_expr(true);
    let rc = circuit_if(circuit, condition, Some(add_h_then_x), std::ptr::null_mut());
    drop_expr(condition);
    rc
}

fn bump(user_data: *mut c_void) {
    if !user_data.is_null() {
        unsafe { (*(user_data as *mut BodyCtx)).calls += 1 };
    }
}

fn ctx_ptr(ctx: &mut BodyCtx) -> *mut c_void {
    ctx as *mut BodyCtx as *mut c_void
}

#[test]
fn circuit_if_builds_body_from_callback() {
    let circuit = circuit_new(2);
    let condition = bool_expr(true);
    let mut ctx = BodyCtx { calls: 0 };

    assert_eq!(
        circuit_if(circuit, condition, Some(add_h_then_x), ctx_ptr(&mut ctx)),
        0
    );
    assert_eq!(ctx.calls, 1);
    // Only the control operation itself is visible at the top level.
    assert_eq!(circuit_num_operations(circuit), 1);
    // Recursive depth: the control op is a scheduling boundary, so
    // 1 + body depth (h(0) then x(0) -> 2) = 3.
    assert_eq!(circuit_depth(circuit, true), 3);
    assert_eq!(circuit_validate(circuit), 0);

    drop_expr(condition);
    circuit_free(circuit);
}

#[test]
fn circuit_if_else_runs_both_callbacks() {
    let circuit = circuit_new(2);
    let condition = bool_expr(false);
    let mut ctx = BodyCtx { calls: 0 };

    assert_eq!(
        circuit_if_else(
            circuit,
            condition,
            Some(add_h_then_x),
            Some(add_z),
            ctx_ptr(&mut ctx)
        ),
        0
    );
    assert_eq!(ctx.calls, 2);
    assert_eq!(circuit_num_operations(circuit), 1);
    // 1 + max(then: h(0)+x(0) = 2, else: z(1) = 1) = 3.
    assert_eq!(circuit_depth(circuit, true), 3);
    assert_eq!(circuit_validate(circuit), 0);

    drop_expr(condition);
    circuit_free(circuit);
}

#[test]
fn circuit_while_allows_terminal_break_and_continue() {
    let circuit = circuit_new(2);
    let var = unsafe { (*circuit).inner.var(ClassicalType::Bool) };
    let condition = Box::into_raw(Box::new(CClassicalExpr {
        inner: ClassicalExpr::var(var),
    }));
    let mut ctx = BodyCtx { calls: 0 };

    assert_eq!(
        circuit_while(
            circuit,
            condition,
            Some(loop_body_with_continue),
            ctx_ptr(&mut ctx)
        ),
        0
    );
    assert_eq!(
        circuit_while(
            circuit,
            condition,
            Some(loop_body_with_break),
            ctx_ptr(&mut ctx)
        ),
        0
    );
    assert_eq!(ctx.calls, 2);
    assert_eq!(circuit_num_operations(circuit), 2);
    // 1 + body once: max(continue body h+x = 2, break body h = 1) + 1 = 3.
    assert_eq!(circuit_depth(circuit, true), 3);
    assert_eq!(circuit_validate(circuit), 0);

    // break/continue outside any loop body is rejected.
    assert_eq!(circuit_break_loop(circuit), -3);
    assert_eq!(circuit_continue_loop(circuit), -3);

    drop_expr(condition);
    circuit_free(circuit);
}

#[test]
fn circuit_while_rejects_non_terminal_control_transfer() {
    let circuit = circuit_new(2);
    let var = unsafe { (*circuit).inner.var(ClassicalType::Bool) };
    let condition = Box::into_raw(Box::new(CClassicalExpr {
        inner: ClassicalExpr::var(var),
    }));
    let mut ctx = BodyCtx { calls: 0 };

    // A continue followed by another operation is rejected and rolled back.
    assert_eq!(
        circuit_while(
            circuit,
            condition,
            Some(loop_body_non_terminal_continue),
            ctx_ptr(&mut ctx)
        ),
        -3
    );
    assert_eq!(ctx.calls, 1);
    assert_eq!(circuit_num_operations(circuit), 0);

    drop_expr(condition);
    circuit_free(circuit);
}

#[test]
fn circuit_for_uint_builds_range_loop() {
    let circuit = circuit_new(1);
    let var = unsafe { (*circuit).inner.var(ClassicalType::uint(8).unwrap()) };
    let var_handle = Box::into_raw(Box::new(CClassicalVar { inner: var }));
    let start = uint_expr(8, 0);
    let stop = uint_expr(8, 4);
    let step = uint_expr(8, 1);
    let mut ctx = BodyCtx { calls: 0 };

    assert_eq!(
        circuit_for_uint(
            circuit,
            var_handle,
            start,
            stop,
            step,
            Some(add_h_then_x),
            ctx_ptr(&mut ctx)
        ),
        0
    );
    assert_eq!(ctx.calls, 1);
    assert_eq!(circuit_num_operations(circuit), 1);
    // Static range [0, 4) step 1 with body h(0)+x(0) unrolls to
    // 1 + 4 * 2 = 9.
    assert_eq!(circuit_depth(circuit, true), 9);
    assert_eq!(circuit_validate(circuit), 0);

    drop_expr(start);
    drop_expr(stop);
    drop_expr(step);
    drop(unsafe { Box::from_raw(var_handle) });
    circuit_free(circuit);
}

extern "C" fn switch_case_zero(circuit: *mut CCircuit, user_data: *mut c_void) -> i32 {
    bump(user_data);
    circuit_x(circuit, 0)
}

extern "C" fn switch_case_one(circuit: *mut CCircuit, user_data: *mut c_void) -> i32 {
    bump(user_data);
    circuit_h(circuit, 1)
}

extern "C" fn switch_default(circuit: *mut CCircuit, user_data: *mut c_void) -> i32 {
    bump(user_data);
    circuit_z(circuit, 0)
}

#[test]
fn circuit_switch_builds_cases_and_default() {
    let circuit = circuit_new(2);
    let target = uint_expr(2, 1);
    let values_lo: [u64; 3] = [0, 1, 2];
    let values_hi: [u64; 3] = [0, 0, 0];
    let bodies: [binding_c::circuit::CqlibBodyFn; 3] = [
        Some(switch_case_zero),
        Some(switch_case_one),
        Some(switch_case_one),
    ];
    let mut ctx = BodyCtx { calls: 0 };

    assert_eq!(
        circuit_switch(
            circuit,
            target,
            values_lo.as_ptr(),
            values_hi.as_ptr(),
            bodies.as_ptr(),
            3,
            Some(switch_default),
            ctx_ptr(&mut ctx)
        ),
        0
    );
    // Three case bodies plus the default body all ran.
    assert_eq!(ctx.calls, 4);
    assert_eq!(circuit_num_operations(circuit), 1);
    // 1 + max(x(0) = 1, h(1) = 1, h(1) = 1, z(0) = 1) = 2.
    assert_eq!(circuit_depth(circuit, true), 2);
    assert_eq!(circuit_validate(circuit), 0);

    drop_expr(target);
    circuit_free(circuit);
}

#[test]
fn circuit_switch_rejects_invalid_cases() {
    let circuit = circuit_new(2);

    // Duplicate case values.
    let target = uint_expr(2, 0);
    let values_lo: [u64; 2] = [1, 1];
    let values_hi: [u64; 2] = [0, 0];
    let bodies: [binding_c::circuit::CqlibBodyFn; 2] =
        [Some(switch_case_zero), Some(switch_case_one)];
    assert_eq!(
        circuit_switch(
            circuit,
            target,
            values_lo.as_ptr(),
            values_hi.as_ptr(),
            bodies.as_ptr(),
            2,
            None,
            std::ptr::null_mut()
        ),
        -3
    );

    // Case value does not fit the UInt(2) target.
    let target = uint_expr(2, 0);
    let values_lo: [u64; 1] = [4];
    let values_hi: [u64; 1] = [0];
    let bodies: [binding_c::circuit::CqlibBodyFn; 1] = [Some(switch_case_zero)];
    assert_eq!(
        circuit_switch(
            circuit,
            target,
            values_lo.as_ptr(),
            values_hi.as_ptr(),
            bodies.as_ptr(),
            1,
            None,
            std::ptr::null_mut()
        ),
        -3
    );

    // Non-UInt target.
    let target = bool_expr(true);
    assert_eq!(
        circuit_switch(
            circuit,
            target,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            None,
            std::ptr::null_mut()
        ),
        -3
    );

    // Nothing was appended by the failed attempts.
    assert_eq!(circuit_num_operations(circuit), 0);

    drop_expr(target);
    circuit_free(circuit);
}

#[test]
fn nested_control_flow_and_measurements() {
    let circuit = circuit_new(2);
    let condition = bool_expr(true);
    let mut ctx = BodyCtx { calls: 0 };

    // An `if` whose body measures a qubit and then nests another `if`.
    assert_eq!(
        circuit_if(
            circuit,
            condition,
            Some(add_measure_then_x),
            ctx_ptr(&mut ctx)
        ),
        0
    );
    assert_eq!(ctx.calls, 1);

    // Nested: while (var) { if (true) { h(0); x(0) } }.
    let var = unsafe { (*circuit).inner.var(ClassicalType::Bool) };
    let while_condition = Box::into_raw(Box::new(CClassicalExpr {
        inner: ClassicalExpr::var(var),
    }));
    assert_eq!(
        circuit_while(
            circuit,
            while_condition,
            Some(nested_if_body),
            ctx_ptr(&mut ctx)
        ),
        0
    );
    assert_eq!(ctx.calls, 2);

    assert_eq!(circuit_num_operations(circuit), 2);
    assert_eq!(circuit_validate(circuit), 0);

    drop_expr(condition);
    drop_expr(while_condition);
    circuit_free(circuit);
}

#[test]
fn callback_errors_roll_back_the_control_region() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 1), 0);
    let condition = bool_expr(true);
    let mut ctx = BodyCtx { calls: 0 };

    // The callback appends h(0) and then fails with -2; everything it
    // appended must be rolled back and the status propagated.
    assert_eq!(
        circuit_if(circuit, condition, Some(failing_body), ctx_ptr(&mut ctx)),
        -2
    );
    assert_eq!(ctx.calls, 1);
    assert_eq!(circuit_num_operations(circuit), 1);
    assert_eq!(circuit_validate(circuit), 0);

    drop_expr(condition);
    circuit_free(circuit);
}

#[test]
fn null_arguments_return_null_ptr_error() {
    let circuit = circuit_new(1);
    let condition = bool_expr(true);

    assert_eq!(
        circuit_if(
            std::ptr::null_mut(),
            condition,
            Some(add_h_then_x),
            std::ptr::null_mut()
        ),
        -1
    );
    assert_eq!(
        circuit_if(
            circuit,
            std::ptr::null(),
            Some(add_h_then_x),
            std::ptr::null_mut()
        ),
        -1
    );
    assert_eq!(
        circuit_if(circuit, condition, None, std::ptr::null_mut()),
        -1
    );
    assert_eq!(circuit_break_loop(std::ptr::null_mut()), -1);
    assert_eq!(circuit_continue_loop(std::ptr::null_mut()), -1);
    assert_eq!(circuit_append_control(std::ptr::null_mut(), circuit, 0), -1);
    assert_eq!(circuit_append_control(circuit, std::ptr::null(), 0), -1);
    assert_eq!(circuit_num_operations(circuit), 0);

    drop_expr(condition);
    circuit_free(circuit);
}

#[test]
fn foreign_classical_handles_are_rejected() {
    let first = circuit_new(1);
    let second = circuit_new(1);
    let var = unsafe { (*first).inner.var(ClassicalType::Bool) };
    let condition = Box::into_raw(Box::new(CClassicalExpr {
        inner: ClassicalExpr::var(var),
    }));
    let mut ctx = BodyCtx { calls: 0 };

    // The condition belongs to `first`, so `second` must reject it.
    assert_eq!(
        circuit_if(second, condition, Some(add_h_then_x), ctx_ptr(&mut ctx)),
        -3
    );
    assert_eq!(ctx.calls, 0);
    assert_eq!(circuit_num_operations(second), 0);

    drop_expr(condition);
    circuit_free(first);
    circuit_free(second);
}

#[test]
fn circuit_append_control_copies_existing_control_op() {
    let source = circuit_new(2);
    let target = circuit_new(2);
    let condition = bool_expr(true);
    let mut ctx = BodyCtx { calls: 0 };

    assert_eq!(
        circuit_if(source, condition, Some(add_h_then_x), ctx_ptr(&mut ctx)),
        0
    );
    assert_eq!(circuit_num_operations(source), 1);

    // Copy the control operation into another circuit. The body has no
    // classical handles, so it stays valid for `target`.
    assert_eq!(circuit_append_control(target, source, 0), 0);
    assert_eq!(circuit_num_operations(target), 1);
    assert_eq!(circuit_depth(target, true), 3);
    assert_eq!(circuit_validate(target), 0);

    // Duplicating within the same circuit is allowed too.
    assert_eq!(circuit_append_control(source, source, 0), 0);
    assert_eq!(circuit_num_operations(source), 2);

    drop_expr(condition);
    circuit_free(source);
    circuit_free(target);
}

#[test]
fn circuit_append_control_rejects_non_control_operations() {
    let circuit = circuit_new(2);
    let other = circuit_new(2);
    assert_eq!(circuit_h(other, 0), 0);

    // Index out of range.
    assert_eq!(circuit_append_control(circuit, other, 5), -8);
    // Index 0 is a plain gate, not a control-flow operation.
    assert_eq!(circuit_append_control(circuit, other, 0), -8);
    assert_eq!(circuit_num_operations(circuit), 0);

    circuit_free(circuit);
    circuit_free(other);
}
