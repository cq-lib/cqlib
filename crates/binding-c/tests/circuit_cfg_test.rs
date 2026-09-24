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

//! Rust FFI tests for circuit control-flow graphs (`circuit/cfg.rs`).

use binding_c::circuit::CqlibBodyFn;
use binding_c::circuit::{
    CCircuit, CClassicalExpr, CClassicalType, CClassicalVar, CFG_FLOW_FALSE_BRANCH,
    CFG_FLOW_TRUE_BRANCH, CFG_FLOW_UNCONDITIONAL, CFG_REGION_FOR, CFG_REGION_IF, CFG_REGION_SWITCH,
    CFG_REGION_WHILE, CFG_TERMINATOR_BRANCH, CFG_TERMINATOR_BREAK, CFG_TERMINATOR_CONTINUE,
    CFG_TERMINATOR_FOR_LOOP, CFG_TERMINATOR_JUMP, CFG_TERMINATOR_RETURN, COperation,
    CQLIB_CLASSICAL_TYPE_BIT, CQLIB_CLASSICAL_TYPE_BIT_VEC, CQLIB_CLASSICAL_TYPE_BOOL,
    CQLIB_CLASSICAL_TYPE_UINT, circuit_cfg_add_block, circuit_cfg_add_edge,
    circuit_cfg_block_has_terminator, circuit_cfg_block_is_empty, circuit_cfg_block_label,
    circuit_cfg_block_operations, circuit_cfg_block_operations_len,
    circuit_cfg_block_terminator_tag, circuit_cfg_blocks, circuit_cfg_classical_values,
    circuit_cfg_classical_values_len, circuit_cfg_classical_vars, circuit_cfg_classical_vars_len,
    circuit_cfg_entry_block, circuit_cfg_free, circuit_cfg_from_circuit, circuit_cfg_from_qubits,
    circuit_cfg_is_loop_header, circuit_cfg_new, circuit_cfg_num_blocks, circuit_cfg_num_qubits,
    circuit_cfg_outgoing_edges, circuit_cfg_outgoing_edges_len, circuit_cfg_push_operation,
    circuit_cfg_qubits, circuit_cfg_region_if, circuit_cfg_region_loop, circuit_cfg_region_switch,
    circuit_cfg_region_switch_cases_len, circuit_cfg_region_tag, circuit_cfg_set_entry_block,
    circuit_cfg_set_for_region, circuit_cfg_set_if_region, circuit_cfg_set_switch_region,
    circuit_cfg_set_terminator_break, circuit_cfg_set_terminator_continue,
    circuit_cfg_set_terminator_jump, circuit_cfg_set_terminator_return,
    circuit_cfg_set_while_region, circuit_cfg_terminator_condition,
    circuit_cfg_terminator_for_info, circuit_cfg_terminator_target, circuit_cfg_to_circuit,
    circuit_cfg_validate, circuit_classical_vars_len, circuit_cx, circuit_for_uint, circuit_free,
    circuit_h, circuit_measure_bits, circuit_new, circuit_num_operations, circuit_num_qubits,
    circuit_var, circuit_while, classical_expr_free, classical_expr_uint_literal,
    classical_expr_var, classical_var_free, operation_free, operation_new,
};
use binding_c::cqlib_string_free;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Loop-body callback appending one gate; `user_data` is ignored.
extern "C" fn add_h(circuit: *mut CCircuit, _user_data: *mut c_void) -> i32 {
    circuit_h(circuit, 0)
}

#[test]
fn cfg_from_circuit_roundtrip() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    assert_eq!(circuit_num_operations(circuit), 2);

    // A straight-line circuit expands into a single entry block.
    let cfg = circuit_cfg_from_circuit(circuit);
    assert!(!cfg.is_null());
    assert_eq!(circuit_cfg_validate(cfg), 0);
    assert_eq!(circuit_cfg_num_qubits(cfg), 2);
    assert_eq!(circuit_cfg_num_blocks(cfg), 1);
    assert_eq!(circuit_cfg_entry_block(cfg), 0);

    let mut ids = [9u32; 2];
    assert_eq!(circuit_cfg_qubits(cfg, ids.as_mut_ptr(), 2), 0);
    assert_eq!(ids, [0, 1]);

    let entry_label = circuit_cfg_block_label(cfg, 0);
    assert!(!entry_label.is_null());
    assert_eq!(
        unsafe { CStr::from_ptr(entry_label) }.to_str().unwrap(),
        "entry"
    );
    cqlib_string_free(entry_label);

    // Entry block: two ordinary operations plus a `Return` terminator.
    assert_eq!(circuit_cfg_block_is_empty(cfg, 0), 0);
    assert_eq!(circuit_cfg_block_operations_len(cfg, 0), 2);
    assert_eq!(circuit_cfg_block_has_terminator(cfg, 0), 1);
    assert_eq!(
        circuit_cfg_block_terminator_tag(cfg, 0),
        CFG_TERMINATOR_RETURN as i32
    );
    assert_eq!(circuit_cfg_terminator_target(cfg, 0), u32::MAX);
    assert!(circuit_cfg_terminator_condition(cfg, 0).is_null());

    // Cloned operation handles are independent of the cfg and of each other.
    let mut cloned: [*mut COperation; 2] = [std::ptr::null_mut(); 2];
    assert_eq!(
        circuit_cfg_block_operations(cfg, 0, cloned.as_mut_ptr(), 2),
        0
    );
    assert!(!cloned[0].is_null());
    assert!(!cloned[1].is_null());
    operation_free(cloned[0]);
    operation_free(cloned[1]);

    // An undersized buffer is rejected before any write.
    assert_eq!(
        circuit_cfg_block_operations(cfg, 0, cloned.as_mut_ptr(), 1),
        -8
    );

    // Straight-line circuits carry no classical state.
    assert_eq!(circuit_cfg_classical_vars_len(cfg), 0);
    assert_eq!(circuit_cfg_classical_values_len(cfg), 0);

    // Round trip: qubit count and operation count are preserved.
    let lowered = circuit_cfg_to_circuit(cfg);
    assert!(!lowered.is_null());
    assert_eq!(circuit_num_qubits(lowered), 2);
    assert_eq!(circuit_num_operations(lowered), 2);

    // The source circuit stays alive and unchanged.
    assert_eq!(circuit_num_operations(circuit), 2);

    circuit_free(lowered);
    circuit_cfg_free(cfg);
    circuit_free(circuit);
}

#[test]
fn cfg_layout_and_blocks() {
    let h_name = cstr("H");
    let zero = [0u32];
    let op = operation_new(h_name.as_ptr(), zero.as_ptr(), 1, std::ptr::null(), 0);
    assert!(!op.is_null());

    // A fresh CFG is empty and has no designated entry.
    let cfg = circuit_cfg_new(2);
    assert!(!cfg.is_null());
    assert_eq!(circuit_cfg_num_qubits(cfg), 2);
    assert_eq!(circuit_cfg_num_blocks(cfg), 0);
    assert_eq!(circuit_cfg_entry_block(cfg), u32::MAX);
    assert_eq!(circuit_cfg_blocks(cfg, std::ptr::null_mut(), 0), 0);

    // Blocks are appended with stable sequential indices.
    let b0 = circuit_cfg_add_block(cfg);
    let b1 = circuit_cfg_add_block(cfg);
    assert_eq!(b0, 0);
    assert_eq!(b1, 1);
    assert_eq!(circuit_cfg_num_blocks(cfg), 2);
    let b2 = circuit_cfg_add_block(cfg);
    assert_eq!(b2, 2);
    assert_eq!(circuit_cfg_num_blocks(cfg), 3);

    // Block enumeration into the caller-supplied buffer.
    let mut nodes = [9u32; 3];
    assert_eq!(circuit_cfg_blocks(cfg, nodes.as_mut_ptr(), 3), 0);
    assert_eq!(nodes, [0, 1, 2]);
    assert_eq!(circuit_cfg_blocks(cfg, nodes.as_mut_ptr(), 2), -8);
    assert_eq!(circuit_cfg_blocks(cfg, std::ptr::null_mut(), 3), -1);

    // Entry switching accepts any non-sentinel index (`validate` checks it).
    assert_eq!(circuit_cfg_set_entry_block(cfg, b1), 0);
    assert_eq!(circuit_cfg_entry_block(cfg), b1);
    assert_eq!(circuit_cfg_set_entry_block(cfg, u32::MAX), -2);

    // Fresh blocks are empty and unlabeled.
    assert_eq!(circuit_cfg_block_is_empty(cfg, b2), 1);
    assert!(circuit_cfg_block_label(cfg, b2).is_null());
    assert_eq!(circuit_cfg_block_operations_len(cfg, b2), 0);
    assert_eq!(
        circuit_cfg_block_operations(cfg, b2, std::ptr::null_mut(), 0),
        0
    );

    // Semantic edges between existing blocks; flow tags are checked first.
    assert_eq!(
        circuit_cfg_add_edge(cfg, b0, b1, CFG_FLOW_UNCONDITIONAL, 0, 0),
        0
    );
    assert_eq!(
        circuit_cfg_add_edge(cfg, b0, 3, CFG_FLOW_UNCONDITIONAL, 0, 0),
        -2
    );
    assert_eq!(
        circuit_cfg_add_edge(cfg, u32::MAX, b1, CFG_FLOW_UNCONDITIONAL, 0, 0),
        -2
    );
    assert_eq!(circuit_cfg_add_edge(cfg, b0, 3, 99, 0, 0), -8);
    assert_eq!(circuit_cfg_add_edge(cfg, b0, b1, 99, 0, 0), -8);

    let mut targets = [9u32; 1];
    let mut tags = [9u32; 1];
    let mut lo = [9u64; 1];
    let mut hi = [9u64; 1];
    assert_eq!(circuit_cfg_outgoing_edges_len(cfg, b0), 1);
    assert_eq!(
        circuit_cfg_outgoing_edges(
            cfg,
            b0,
            targets.as_mut_ptr(),
            tags.as_mut_ptr(),
            lo.as_mut_ptr(),
            hi.as_mut_ptr(),
            1
        ),
        0
    );
    assert_eq!(targets, [b1]);
    assert_eq!(tags, [CFG_FLOW_UNCONDITIONAL]);
    assert_eq!(lo, [0]);
    assert_eq!(hi, [0]);
    // Undersized then NULL parallel buffers.
    assert_eq!(
        circuit_cfg_outgoing_edges(
            cfg,
            b0,
            targets.as_mut_ptr(),
            tags.as_mut_ptr(),
            lo.as_mut_ptr(),
            hi.as_mut_ptr(),
            0
        ),
        -8
    );
    assert_eq!(
        circuit_cfg_outgoing_edges(
            cfg,
            b0,
            std::ptr::null_mut(),
            tags.as_mut_ptr(),
            lo.as_mut_ptr(),
            hi.as_mut_ptr(),
            1
        ),
        -1
    );

    // Pushing one operation flips `block_is_empty` and grows the count.
    assert_eq!(circuit_cfg_push_operation(cfg, b2, op), 0);
    assert_eq!(circuit_cfg_block_is_empty(cfg, b2), 0);
    assert_eq!(circuit_cfg_block_operations_len(cfg, b2), 1);
    let mut single: [*mut COperation; 1] = [std::ptr::null_mut(); 1];
    assert_eq!(
        circuit_cfg_block_operations(cfg, b2, single.as_mut_ptr(), 1),
        0
    );
    assert!(!single[0].is_null());
    operation_free(single[0]);
    assert_eq!(
        circuit_cfg_block_operations(cfg, b2, single.as_mut_ptr(), 0),
        -8
    );
    assert_eq!(
        circuit_cfg_block_operations(cfg, b2, std::ptr::null_mut(), 1),
        -1
    );

    // Unknown blocks are rejected by mutation entry points.
    assert_eq!(circuit_cfg_push_operation(cfg, 7, op), -2);
    assert_eq!(circuit_cfg_push_operation(cfg, u32::MAX, op), -2);
    assert_eq!(circuit_cfg_push_operation(cfg, b2, std::ptr::null()), -1);

    circuit_cfg_free(cfg);
    operation_free(op);
}

#[test]
fn cfg_terminators() {
    let cfg = circuit_cfg_new(1);
    let b0 = circuit_cfg_add_block(cfg);
    let b1 = circuit_cfg_add_block(cfg);
    let b2 = circuit_cfg_add_block(cfg);

    // Setters succeed and the getter round-trips each terminator flavor.
    assert_eq!(circuit_cfg_set_terminator_jump(cfg, b0, b1), 0);
    assert_eq!(circuit_cfg_set_terminator_break(cfg, b1, b0), 0);
    assert_eq!(circuit_cfg_set_terminator_continue(cfg, b2, b0), 0);

    assert_eq!(circuit_cfg_block_has_terminator(cfg, b0), 1);
    assert_eq!(
        circuit_cfg_block_terminator_tag(cfg, b0),
        CFG_TERMINATOR_JUMP as i32
    );
    assert_eq!(circuit_cfg_terminator_target(cfg, b0), b1);
    assert_eq!(
        circuit_cfg_block_terminator_tag(cfg, b1),
        CFG_TERMINATOR_BREAK as i32
    );
    assert_eq!(circuit_cfg_terminator_target(cfg, b1), b0);
    assert_eq!(
        circuit_cfg_block_terminator_tag(cfg, b2),
        CFG_TERMINATOR_CONTINUE as i32
    );
    assert_eq!(circuit_cfg_terminator_target(cfg, b2), b0);

    // A terminator without operations still makes the block non-empty.
    assert_eq!(circuit_cfg_block_is_empty(cfg, b0), 0);
    assert_eq!(circuit_cfg_block_operations_len(cfg, b0), 0);

    // Jump/Break/Continue carry no condition and no loop info.
    assert!(circuit_cfg_terminator_condition(cfg, b0).is_null());
    let mut var: *mut CClassicalVar = std::ptr::null_mut();
    let mut start: *mut CClassicalExpr = std::ptr::null_mut();
    let mut stop: *mut CClassicalExpr = std::ptr::null_mut();
    let mut step: *mut CClassicalExpr = std::ptr::null_mut();
    assert_eq!(
        circuit_cfg_terminator_for_info(cfg, b0, &mut var, &mut start, &mut stop, &mut step),
        -8
    );

    // `Return` has no target and overwrites the previous terminator.
    assert_eq!(circuit_cfg_set_terminator_return(cfg, b2), 0);
    assert_eq!(
        circuit_cfg_block_terminator_tag(cfg, b2),
        CFG_TERMINATOR_RETURN as i32
    );
    assert_eq!(circuit_cfg_terminator_target(cfg, b2), u32::MAX);

    // NULL and unknown blocks.
    assert_eq!(
        circuit_cfg_set_terminator_jump(std::ptr::null_mut(), 0, 0),
        -1
    );
    assert_eq!(
        circuit_cfg_set_terminator_return(std::ptr::null_mut(), 0),
        -1
    );
    assert_eq!(
        circuit_cfg_set_terminator_break(std::ptr::null_mut(), 0, 0),
        -1
    );
    assert_eq!(
        circuit_cfg_set_terminator_continue(std::ptr::null_mut(), 0, 0),
        -1
    );
    assert_eq!(circuit_cfg_set_terminator_jump(cfg, 7, 0), -2);
    assert_eq!(circuit_cfg_set_terminator_jump(cfg, u32::MAX, 0), -2);

    circuit_cfg_free(cfg);
}

#[test]
fn cfg_classical_state() {
    // Circuit carrying two variables plus one measured value.
    let circuit = circuit_new(2);
    let bit_var = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    let uint_var = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_UINT, 8);
    assert!(!bit_var.is_null());
    assert!(!uint_var.is_null());

    let measured = [0u32, 1];
    let measured_expr = circuit_measure_bits(circuit, measured.as_ptr(), 2);
    assert!(!measured_expr.is_null());
    assert_eq!(circuit_classical_vars_len(circuit), 2);

    let cfg = circuit_cfg_from_circuit(circuit);
    assert!(!cfg.is_null());

    // The CFG snapshots the circuit's static classical tables.
    assert_eq!(circuit_cfg_classical_vars_len(cfg), 2);
    assert_eq!(circuit_cfg_classical_values_len(cfg), 1);

    let mut vars = [
        CClassicalType { tag: 9, width: 9 },
        CClassicalType { tag: 9, width: 9 },
    ];
    assert_eq!(circuit_cfg_classical_vars(cfg, vars.as_mut_ptr(), 2), 0);
    // `CClassicalType` is a repr(C) struct without derives, so compare per field.
    assert_eq!(vars[0].tag, CQLIB_CLASSICAL_TYPE_BIT);
    assert_eq!(vars[0].width, 1);
    assert_eq!(vars[1].tag, CQLIB_CLASSICAL_TYPE_UINT);
    assert_eq!(vars[1].width, 8);

    let mut values = [CClassicalType { tag: 9, width: 9 }];
    assert_eq!(circuit_cfg_classical_values(cfg, values.as_mut_ptr(), 1), 0);
    assert_eq!(values[0].tag, CQLIB_CLASSICAL_TYPE_BIT_VEC);
    assert_eq!(values[0].width, 2);

    // Undersized then NULL buffers (len >= count is allowed).
    assert_eq!(circuit_cfg_classical_vars(cfg, vars.as_mut_ptr(), 1), -8);
    assert_eq!(circuit_cfg_classical_vars(cfg, std::ptr::null_mut(), 2), -1);
    assert_eq!(
        circuit_cfg_classical_values(cfg, values.as_mut_ptr(), 0),
        -8
    );
    assert_eq!(
        circuit_cfg_classical_values(cfg, std::ptr::null_mut(), 1),
        -1
    );

    classical_expr_free(measured_expr);
    classical_var_free(uint_var);
    classical_var_free(bit_var);
    circuit_cfg_free(cfg);

    // A circuit without classical state produces empty CFG tables.
    let plain = circuit_new(1);
    let plain_cfg = circuit_cfg_from_circuit(plain);
    assert!(!plain_cfg.is_null());
    assert_eq!(circuit_cfg_classical_vars_len(plain_cfg), 0);
    assert_eq!(circuit_cfg_classical_values_len(plain_cfg), 0);
    assert_eq!(
        circuit_cfg_classical_vars(plain_cfg, std::ptr::null_mut(), 0),
        0
    );
    assert_eq!(
        circuit_cfg_classical_values(plain_cfg, std::ptr::null_mut(), 0),
        0
    );
    circuit_cfg_free(plain_cfg);
    circuit_free(plain);

    // A hand-built CFG starts with empty tables as well.
    let empty = circuit_cfg_new(0);
    assert_eq!(circuit_cfg_classical_vars_len(empty), 0);
    assert_eq!(circuit_cfg_classical_values_len(empty), 0);
    assert_eq!(
        circuit_cfg_classical_vars(empty, std::ptr::null_mut(), 0),
        0
    );
    assert_eq!(
        circuit_cfg_classical_values(empty, std::ptr::null_mut(), 0),
        0
    );
    circuit_cfg_free(empty);

    circuit_free(circuit);
}

#[test]
fn cfg_loop_regions() {
    // `while (flag) { h(0); }` expands into entry / cond / body / exit.
    let circuit = circuit_new(2);
    let condition_var = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    assert!(!condition_var.is_null());
    let condition = classical_expr_var(condition_var);
    assert!(!condition.is_null());
    let body: CqlibBodyFn = Some(add_h);
    assert_eq!(
        circuit_while(circuit, condition, body, std::ptr::null_mut()),
        0
    );
    classical_expr_free(condition);
    classical_var_free(condition_var);

    let cfg = circuit_cfg_from_circuit(circuit);
    assert!(!cfg.is_null());
    assert_eq!(circuit_cfg_validate(cfg), 0);
    assert_eq!(circuit_cfg_num_blocks(cfg), 4);
    assert_eq!(circuit_cfg_entry_block(cfg), 0);

    // Entry jumps to the condition header, which owns the While region.
    assert_eq!(
        circuit_cfg_block_terminator_tag(cfg, 0),
        CFG_TERMINATOR_JUMP as i32
    );
    assert_eq!(circuit_cfg_terminator_target(cfg, 0), 1);
    assert_eq!(circuit_cfg_region_tag(cfg, 1), CFG_REGION_WHILE as i32);
    assert_eq!(circuit_cfg_is_loop_header(cfg, 1), 1);
    assert_eq!(circuit_cfg_region_tag(cfg, 0), 0);
    assert_eq!(circuit_cfg_is_loop_header(cfg, 0), 0);
    assert_eq!(
        circuit_cfg_block_terminator_tag(cfg, 1),
        CFG_TERMINATOR_BRANCH as i32
    );
    assert_eq!(circuit_cfg_terminator_target(cfg, 1), u32::MAX);

    // Structured labels are attached for diagnostics.
    let body_label = circuit_cfg_block_label(cfg, 2);
    assert!(!body_label.is_null());
    let body_text = unsafe { CStr::from_ptr(body_label) }
        .to_str()
        .unwrap()
        .to_owned();
    cqlib_string_free(body_label);
    assert!(body_text.starts_with("while_body_"));

    // The Branch condition is exposed as a fresh expression handle.
    let branch_condition = circuit_cfg_terminator_condition(cfg, 1);
    assert!(!branch_condition.is_null());
    classical_expr_free(branch_condition);

    // The loop layout reads back body and exit blocks.
    let mut body_block = 9u32;
    let mut exit_block = 9u32;
    assert_eq!(
        circuit_cfg_region_loop(cfg, 1, &mut body_block, &mut exit_block),
        0
    );
    assert_eq!(body_block, 2);
    assert_eq!(exit_block, 3);
    // Non-loop blocks have no region and are rejected.
    assert_eq!(
        circuit_cfg_region_loop(cfg, 0, &mut body_block, &mut exit_block),
        -8
    );
    assert_eq!(
        circuit_cfg_terminator_for_info(
            cfg,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut()
        ),
        -8
    );

    // Header edges: TrueBranch into the body, FalseBranch into the exit.
    let mut targets = [9u32; 2];
    let mut flow_tags = [9u32; 2];
    let mut lo = [9u64; 2];
    let mut hi = [9u64; 2];
    assert_eq!(circuit_cfg_outgoing_edges_len(cfg, 1), 2);
    assert_eq!(
        circuit_cfg_outgoing_edges(
            cfg,
            1,
            targets.as_mut_ptr(),
            flow_tags.as_mut_ptr(),
            lo.as_mut_ptr(),
            hi.as_mut_ptr(),
            2
        ),
        0
    );
    assert_eq!(targets, [2, 3]);
    assert_eq!(flow_tags, [CFG_FLOW_TRUE_BRANCH, CFG_FLOW_FALSE_BRANCH]);

    // The body jumps back to the header; the exit returns.
    assert_eq!(circuit_cfg_block_operations_len(cfg, 2), 1);
    assert_eq!(circuit_cfg_terminator_target(cfg, 2), 1);
    assert_eq!(
        circuit_cfg_block_terminator_tag(cfg, 3),
        CFG_TERMINATOR_RETURN as i32
    );

    circuit_cfg_free(cfg);
    circuit_free(circuit);

    // `for` loops expose their loop variable through `terminator_for_info`.
    let for_circuit = circuit_new(1);
    let counter = circuit_var(for_circuit, CQLIB_CLASSICAL_TYPE_UINT, 8);
    assert!(!counter.is_null());
    let start = classical_expr_uint_literal(8, 0, 0);
    let stop = classical_expr_uint_literal(8, 4, 0);
    let step = classical_expr_uint_literal(8, 1, 0);
    assert!(!start.is_null());
    assert!(!stop.is_null());
    assert!(!step.is_null());
    assert_eq!(
        circuit_for_uint(
            for_circuit,
            counter,
            start,
            stop,
            step,
            Some(add_h),
            std::ptr::null_mut()
        ),
        0
    );

    let for_cfg = circuit_cfg_from_circuit(for_circuit);
    assert!(!for_cfg.is_null());
    assert_eq!(circuit_cfg_num_blocks(for_cfg), 4);
    assert_eq!(
        circuit_cfg_block_terminator_tag(for_cfg, 1),
        CFG_TERMINATOR_FOR_LOOP as i32
    );
    assert_eq!(circuit_cfg_region_tag(for_cfg, 1), CFG_REGION_FOR as i32);
    assert_eq!(circuit_cfg_is_loop_header(for_cfg, 1), 1);
    assert_eq!(
        circuit_cfg_region_loop(for_cfg, 1, &mut body_block, &mut exit_block),
        0
    );
    assert_eq!(body_block, 2);
    assert_eq!(exit_block, 3);

    let mut var: *mut CClassicalVar = std::ptr::null_mut();
    let mut for_start: *mut CClassicalExpr = std::ptr::null_mut();
    let mut for_stop: *mut CClassicalExpr = std::ptr::null_mut();
    let mut for_step: *mut CClassicalExpr = std::ptr::null_mut();
    assert_eq!(
        circuit_cfg_terminator_for_info(
            for_cfg,
            1,
            &mut var,
            &mut for_start,
            &mut for_stop,
            &mut for_step
        ),
        0
    );
    assert!(!var.is_null());
    assert!(!for_start.is_null());
    assert!(!for_stop.is_null());
    assert!(!for_step.is_null());
    classical_var_free(var);
    classical_expr_free(for_step);
    classical_expr_free(for_stop);
    classical_expr_free(for_start);

    // NULL out-parameters on a present ForLoop header fail before writing.
    assert_eq!(
        circuit_cfg_terminator_for_info(
            for_cfg,
            1,
            std::ptr::null_mut(),
            &mut for_start,
            &mut for_stop,
            &mut for_step
        ),
        -1
    );
    // Non-loop blocks and unknown blocks report their failure mode.
    assert_eq!(
        circuit_cfg_terminator_for_info(
            for_cfg,
            0,
            &mut var,
            &mut for_start,
            &mut for_stop,
            &mut for_step
        ),
        -8
    );
    assert_eq!(
        circuit_cfg_terminator_for_info(
            for_cfg,
            u32::MAX,
            &mut var,
            &mut for_start,
            &mut for_stop,
            &mut for_step
        ),
        -2
    );

    circuit_cfg_free(for_cfg);
    classical_expr_free(step);
    classical_expr_free(stop);
    classical_expr_free(start);
    classical_var_free(counter);
    circuit_free(for_circuit);

    // Region setters: manual attachment and replacement on a bare CFG.
    let manual = circuit_cfg_new(1);
    let header = circuit_cfg_add_block(manual);
    let body_b = circuit_cfg_add_block(manual);
    let exit_b = circuit_cfg_add_block(manual);
    let merge_b = circuit_cfg_add_block(manual);

    assert_eq!(
        circuit_cfg_set_while_region(manual, header, body_b, exit_b, std::ptr::null()),
        0
    );
    assert_eq!(
        circuit_cfg_region_tag(manual, header),
        CFG_REGION_WHILE as i32
    );
    assert_eq!(circuit_cfg_is_loop_header(manual, header), 1);

    // Replacing the region with a For region keeps it a loop header.
    assert_eq!(
        circuit_cfg_set_for_region(manual, header, body_b, exit_b, std::ptr::null()),
        0
    );
    assert_eq!(
        circuit_cfg_region_tag(manual, header),
        CFG_REGION_FOR as i32
    );
    assert_eq!(circuit_cfg_is_loop_header(manual, header), 1);
    assert_eq!(
        circuit_cfg_region_loop(manual, header, &mut body_block, &mut exit_block),
        0
    );
    assert_eq!(body_block, body_b);
    assert_eq!(exit_block, exit_b);

    // An If region is not a loop header and carries the four-way layout.
    assert_eq!(
        circuit_cfg_set_if_region(manual, header, body_b, exit_b, merge_b, 0, std::ptr::null()),
        0
    );
    assert_eq!(circuit_cfg_region_tag(manual, header), CFG_REGION_IF as i32);
    assert_eq!(circuit_cfg_is_loop_header(manual, header), 0);
    let mut then_entry = 9u32;
    let mut else_entry = 9u32;
    let mut merge_block = 9u32;
    let mut has_else = 9i32;
    assert_eq!(
        circuit_cfg_region_if(
            manual,
            header,
            &mut then_entry,
            &mut else_entry,
            &mut merge_block,
            &mut has_else
        ),
        0
    );
    assert_eq!(then_entry, body_b);
    assert_eq!(else_entry, exit_b);
    assert_eq!(merge_block, merge_b);
    assert_eq!(has_else, 0);
    assert_eq!(
        circuit_cfg_region_loop(manual, header, &mut body_block, &mut exit_block),
        -8
    );

    // A Switch region enumerates its match values and entries.
    let case_lo: [u64; 1] = [7];
    let case_hi: [u64; 1] = [0];
    let case_entries: [u32; 1] = [body_b];
    assert_eq!(
        circuit_cfg_set_switch_region(
            manual,
            header,
            case_lo.as_ptr(),
            case_hi.as_ptr(),
            case_entries.as_ptr(),
            1,
            exit_b,
            merge_b,
            1,
            std::ptr::null()
        ),
        0
    );
    assert_eq!(
        circuit_cfg_region_tag(manual, header),
        CFG_REGION_SWITCH as i32
    );
    assert_eq!(circuit_cfg_is_loop_header(manual, header), 0);
    assert_eq!(circuit_cfg_region_switch_cases_len(manual, header), 1);
    assert_eq!(
        circuit_cfg_region_switch(
            manual,
            header,
            lo.as_mut_ptr(),
            hi.as_mut_ptr(),
            targets.as_mut_ptr(),
            1,
            &mut exit_block,
            &mut merge_block,
            &mut has_else
        ),
        0
    );
    assert_eq!(lo[0], 7);
    assert_eq!(hi[0], 0);
    assert_eq!(targets[0], body_b);
    assert_eq!(exit_block, exit_b);
    assert_eq!(merge_block, merge_b);
    assert_eq!(has_else, 1);

    // Setter failure modes: NULL handles, sentinel, and unknown headers.
    assert_eq!(
        circuit_cfg_set_while_region(std::ptr::null_mut(), 0, 0, 0, std::ptr::null()),
        -1
    );
    assert_eq!(
        circuit_cfg_set_if_region(std::ptr::null_mut(), 0, 0, 0, 0, 0, std::ptr::null()),
        -1
    );
    assert_eq!(
        circuit_cfg_set_for_region(std::ptr::null_mut(), 0, 0, 0, std::ptr::null()),
        -1
    );
    assert_eq!(
        circuit_cfg_set_switch_region(
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            0,
            0,
            0,
            std::ptr::null()
        ),
        -1
    );
    assert_eq!(
        circuit_cfg_set_switch_region(
            manual,
            header,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            1,
            exit_b,
            merge_b,
            1,
            std::ptr::null()
        ),
        -1
    );
    assert_eq!(
        circuit_cfg_set_while_region(manual, 7, 0, 0, std::ptr::null()),
        -2
    );
    assert_eq!(
        circuit_cfg_set_while_region(manual, u32::MAX, 0, 0, std::ptr::null()),
        -2
    );

    // Hand-attached regions point at real blocks everywhere.
    let region_targets: [u32; 1] = [header];
    assert_eq!(
        circuit_cfg_set_switch_region(
            manual,
            header,
            case_lo.as_ptr(),
            case_hi.as_ptr(),
            region_targets.as_ptr(),
            1,
            exit_b,
            merge_b,
            1,
            std::ptr::null()
        ),
        0
    );
    assert_eq!(circuit_cfg_region_switch_cases_len(manual, header), 1);

    circuit_cfg_free(manual);
}

#[test]
fn cfg_null_and_error_paths() {
    let h_name = cstr("H");
    let zero = [0u32];
    let op = operation_new(h_name.as_ptr(), zero.as_ptr(), 1, std::ptr::null(), 0);
    assert!(!op.is_null());
    let mut var: *mut CClassicalVar = std::ptr::null_mut();
    let mut start: *mut CClassicalExpr = std::ptr::null_mut();
    let mut stop: *mut CClassicalExpr = std::ptr::null_mut();
    let mut step: *mut CClassicalExpr = std::ptr::null_mut();
    let mut entry = 9u32;

    // Every exported function tolerates a NULL handle.
    assert!(circuit_cfg_from_circuit(std::ptr::null()).is_null());
    assert!(circuit_cfg_from_qubits_probe_null());
    assert!(circuit_cfg_to_circuit(std::ptr::null()).is_null());
    assert_eq!(circuit_cfg_validate(std::ptr::null()), -1);
    assert_eq!(circuit_cfg_num_qubits(std::ptr::null()), 0);
    assert_eq!(circuit_cfg_num_blocks(std::ptr::null()), 0);
    assert_eq!(
        circuit_cfg_qubits(std::ptr::null(), std::ptr::null_mut(), 0),
        -1
    );
    assert_eq!(circuit_cfg_classical_vars_len(std::ptr::null()), 0);
    assert_eq!(circuit_cfg_classical_values_len(std::ptr::null()), 0);
    assert_eq!(
        circuit_cfg_classical_vars(std::ptr::null(), std::ptr::null_mut(), 0),
        -1
    );
    assert_eq!(
        circuit_cfg_classical_values(std::ptr::null(), std::ptr::null_mut(), 0),
        -1
    );
    assert_eq!(circuit_cfg_entry_block(std::ptr::null()), u32::MAX);
    assert_eq!(circuit_cfg_add_block(std::ptr::null_mut()), u32::MAX);
    assert_eq!(
        circuit_cfg_add_edge(std::ptr::null_mut(), 0, 0, CFG_FLOW_UNCONDITIONAL, 0, 0),
        -1
    );
    assert_eq!(circuit_cfg_set_entry_block(std::ptr::null_mut(), 0), -1);
    assert_eq!(circuit_cfg_push_operation(std::ptr::null_mut(), 0, op), -1);
    assert_eq!(
        circuit_cfg_blocks(std::ptr::null(), std::ptr::null_mut(), 0),
        -1
    );
    assert!(circuit_cfg_block_label(std::ptr::null(), 0).is_null());
    assert_eq!(circuit_cfg_block_is_empty(std::ptr::null(), 0), -1);
    assert_eq!(circuit_cfg_block_operations_len(std::ptr::null(), 0), 0);
    assert_eq!(
        circuit_cfg_block_operations(std::ptr::null(), 0, std::ptr::null_mut(), 0),
        -1
    );
    assert_eq!(circuit_cfg_block_has_terminator(std::ptr::null(), 0), -1);
    assert_eq!(circuit_cfg_block_terminator_tag(std::ptr::null(), 0), -1);
    assert_eq!(circuit_cfg_terminator_target(std::ptr::null(), 0), u32::MAX);
    assert!(circuit_cfg_terminator_condition(std::ptr::null(), 0).is_null());
    assert_eq!(
        circuit_cfg_terminator_for_info(
            std::ptr::null(),
            0,
            &mut var,
            &mut start,
            &mut stop,
            &mut step
        ),
        -1
    );
    assert_eq!(circuit_cfg_outgoing_edges_len(std::ptr::null(), 0), 0);
    assert_eq!(
        circuit_cfg_outgoing_edges(
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0
        ),
        -1
    );
    assert_eq!(circuit_cfg_region_tag(std::ptr::null(), 0), -1);
    assert_eq!(circuit_cfg_is_loop_header(std::ptr::null(), 0), -1);
    assert_eq!(
        circuit_cfg_region_if(
            std::ptr::null(),
            0,
            &mut entry,
            &mut entry,
            &mut entry,
            std::ptr::null_mut()
        ),
        -1
    );
    assert_eq!(
        circuit_cfg_region_loop(std::ptr::null(), 0, &mut entry, &mut entry),
        -1
    );
    assert_eq!(circuit_cfg_region_switch_cases_len(std::ptr::null(), 0), 0);
    assert_eq!(
        circuit_cfg_region_switch(
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut entry,
            &mut entry,
            std::ptr::null_mut()
        ),
        -1
    );
    assert_eq!(
        circuit_cfg_set_terminator_jump(std::ptr::null_mut(), 0, 0),
        -1
    );
    assert_eq!(
        circuit_cfg_set_terminator_return(std::ptr::null_mut(), 0),
        -1
    );
    circuit_cfg_free(std::ptr::null_mut());

    // Out-of-range and sentinel node ids fail every block-level query.
    let cfg = circuit_cfg_new(1);
    assert_eq!(circuit_cfg_add_block(cfg), 0);
    for invalid in [7u32, u32::MAX] {
        assert_eq!(circuit_cfg_block_is_empty(cfg, invalid), -2);
        assert!(circuit_cfg_block_label(cfg, invalid).is_null());
        assert_eq!(circuit_cfg_block_operations_len(cfg, invalid), 0);
        assert_eq!(circuit_cfg_block_has_terminator(cfg, invalid), -2);
        assert_eq!(circuit_cfg_block_terminator_tag(cfg, invalid), -2);
        assert_eq!(circuit_cfg_terminator_target(cfg, invalid), u32::MAX);
        assert!(circuit_cfg_terminator_condition(cfg, invalid).is_null());
        assert_eq!(
            circuit_cfg_block_operations(cfg, invalid, std::ptr::null_mut(), 1),
            -2
        );
        assert_eq!(circuit_cfg_outgoing_edges_len(cfg, invalid), 0);
        assert_eq!(
            circuit_cfg_outgoing_edges(
                cfg,
                invalid,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1
            ),
            -2
        );
        assert_eq!(circuit_cfg_region_tag(cfg, invalid), -2);
        assert_eq!(circuit_cfg_is_loop_header(cfg, invalid), -2);
        assert_eq!(
            circuit_cfg_region_if(
                cfg,
                invalid,
                &mut entry,
                &mut entry,
                &mut entry,
                std::ptr::null_mut()
            ),
            -2
        );
        assert_eq!(
            circuit_cfg_region_loop(cfg, invalid, &mut entry, &mut entry),
            -2
        );
        assert_eq!(circuit_cfg_region_switch_cases_len(cfg, invalid), 0);
        assert_eq!(
            circuit_cfg_region_switch(
                cfg,
                invalid,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                &mut entry,
                &mut entry,
                std::ptr::null_mut()
            ),
            -2
        );
        assert_eq!(circuit_cfg_push_operation(cfg, invalid, op), -2);
        assert_eq!(circuit_cfg_set_terminator_return(cfg, invalid), -2);
        assert_eq!(
            circuit_cfg_set_while_region(cfg, invalid, 0, 0, std::ptr::null()),
            -2
        );
    }
    assert_eq!(circuit_cfg_set_entry_block(cfg, u32::MAX), -2);

    circuit_cfg_free(cfg);
    operation_free(op);
}

/// `circuit_cfg_from_qubits` with a NULL buffer plus a positive length.
fn circuit_cfg_from_qubits_probe_null() -> bool {
    circuit_cfg_from_qubits(std::ptr::null(), 1).is_null()
}
