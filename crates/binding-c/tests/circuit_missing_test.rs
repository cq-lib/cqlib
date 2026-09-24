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

//! Rust FFI tests for the previously missing circuit FFI surface:
//! operation kind discriminants, CFG block editing, structural
//! construction/queries, and checkpoint transactions.

use binding_c::circuit::{
    CCircuitCFG, CClassicalValueInfo, COperation, CQLIB_CLASSICAL_TYPE_BIT, CValueOperation,
    OPERATION_DIRECTIVE_BARRIER, checkpoint_free, circuit_append_value_operation, circuit_barrier,
    circuit_begin, circuit_cfg_add_block, circuit_cfg_block_label, circuit_cfg_block_operations,
    circuit_cfg_block_operations_len, circuit_cfg_extend_operations, circuit_cfg_free,
    circuit_cfg_from_circuit, circuit_cfg_new, circuit_cfg_with_label, circuit_checkpoint,
    circuit_commit, circuit_contains_qubit, circuit_delay, circuit_free, circuit_from_operations,
    circuit_from_qubits, circuit_h, circuit_has_same_qubits, circuit_index, circuit_measure_into,
    circuit_new, circuit_num_operations, circuit_num_qubits, circuit_operations_structurally_equal,
    circuit_parameters_len, circuit_rollback_control_body_transaction, circuit_rollback_to,
    circuit_rx, circuit_store, circuit_var, classical_expr_bit_literal, classical_expr_free,
    classical_var_free, operation_as_instruction, operation_directive, operation_free,
    operation_gate_arity, operation_is_circuit_gate, operation_is_classical_control,
    operation_is_classical_data, operation_is_delay, operation_is_directive,
    operation_is_instruction, operation_is_mcgate, operation_is_quantum_gate,
    operation_is_standard_gate, operation_is_unitary, operation_name, operation_new,
    operation_reads_value, operation_result, value_operation_free,
};
use binding_c::cqlib_string_free;
use std::ffi::{CStr, CString};

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Builds a fixed-parameter standard-gate operation handle.
fn op_new(name: &str, qubits: &[u32], params: &[f64]) -> *mut COperation {
    let gate = cstr(name);
    operation_new(
        gate.as_ptr(),
        qubits.as_ptr(),
        qubits.len(),
        params.as_ptr(),
        params.len(),
    )
}

/// Copies `count` operation handles out of the entry block of `cfg`.
fn block_ops(cfg: *const CCircuitCFG, node: u32, count: usize) -> Vec<*mut COperation> {
    let mut ops: Vec<*mut COperation> = vec![std::ptr::null_mut(); count];
    assert_eq!(
        circuit_cfg_block_operations(cfg, node, ops.as_mut_ptr(), count),
        0
    );
    assert!(ops.iter().all(|op| !op.is_null()));
    ops
}

fn op_name(op: *const COperation) -> String {
    let name = operation_name(op);
    assert!(!name.is_null());
    let text = unsafe { CStr::from_ptr(name) }
        .to_str()
        .unwrap()
        .to_string();
    cqlib_string_free(name);
    text
}

#[test]
fn operation_kind_predicates_for_standard_gate() {
    let op = op_new("H", &[0], &[]);

    // An H gate is a standard gate and a quantum gate, nothing else.
    assert_eq!(operation_is_standard_gate(op), 1);
    assert_eq!(operation_is_quantum_gate(op), 1);
    assert_eq!(operation_is_instruction(op), 1);
    assert_eq!(operation_is_unitary(op), 0);
    assert_eq!(operation_is_mcgate(op), 0);
    assert_eq!(operation_is_circuit_gate(op), 0);
    assert_eq!(operation_is_directive(op), 0);
    assert_eq!(operation_is_delay(op), 0);
    assert_eq!(operation_is_classical_control(op), 0);
    assert_eq!(operation_is_classical_data(op), 0);

    // NULL input is reported as -1 by every predicate.
    let null = std::ptr::null::<COperation>();
    assert_eq!(operation_is_standard_gate(null), -1);
    assert_eq!(operation_is_quantum_gate(null), -1);
    assert_eq!(operation_is_instruction(null), -1);
    assert_eq!(operation_is_unitary(null), -1);
    assert_eq!(operation_is_mcgate(null), -1);
    assert_eq!(operation_is_circuit_gate(null), -1);
    assert_eq!(operation_is_directive(null), -1);
    assert_eq!(operation_is_delay(null), -1);
    assert_eq!(operation_is_classical_control(null), -1);
    assert_eq!(operation_is_classical_data(null), -1);

    operation_free(op);
}

#[test]
fn operation_gate_arity_reports_fixed_arity() {
    let mut qubits: usize = 9;
    let mut params: usize = 9;

    let h = op_new("H", &[0], &[]);
    assert_eq!(operation_gate_arity(h, &mut qubits, &mut params), 0);
    assert_eq!((qubits, params), (1, 0));

    let rx = op_new("RX", &[0], &[0.5]);
    assert_eq!(operation_gate_arity(rx, &mut qubits, &mut params), 0);
    assert_eq!((qubits, params), (1, 1));

    let cx = op_new("CX", &[0, 1], &[]);
    assert_eq!(operation_gate_arity(cx, &mut qubits, &mut params), 0);
    assert_eq!((qubits, params), (2, 0));

    // NULL inputs, and barriers whose arity is variable, are rejected.
    assert_eq!(
        operation_gate_arity(std::ptr::null(), &mut qubits, &mut params),
        -1
    );
    assert_eq!(
        operation_gate_arity(h, std::ptr::null_mut(), &mut params),
        -1
    );
    operation_free(h);
    operation_free(rx);
    operation_free(cx);
}

#[test]
fn operation_as_instruction_clones_plain_operation() {
    let op = op_new("H", &[0], &[]);
    let cloned = operation_as_instruction(op);
    assert!(!cloned.is_null());
    assert_eq!(op_name(cloned), "H");
    operation_free(cloned);
    operation_free(op);

    assert!(operation_as_instruction(std::ptr::null()).is_null());
}

#[test]
fn operation_directive_and_delay_flags_via_cfg() {
    let circuit = circuit_new(2);
    let var = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 1);
    assert!(!var.is_null());
    assert_eq!(circuit_barrier(circuit, std::ptr::null(), 0), 0);
    assert_eq!(circuit_delay(circuit, 0, 1.5), 0);
    let meas = circuit_measure_into(circuit, 1, var);
    assert!(!meas.is_null());
    classical_expr_free(meas);

    let cfg = circuit_cfg_from_circuit(circuit);
    assert!(!cfg.is_null());
    // Storage order: [barrier, delay, MeasureBit, Store]; measure_into
    // appends both the measurement and its store.
    assert_eq!(circuit_cfg_block_operations_len(cfg, 0), 4);
    let ops = block_ops(cfg, 0, 4);

    // Barrier: directive, non-gate, variable arity.
    assert_eq!(operation_is_directive(ops[0]), 1);
    assert_eq!(
        operation_directive(ops[0]),
        OPERATION_DIRECTIVE_BARRIER as i32
    );
    assert_eq!(operation_is_quantum_gate(ops[0]), 0);
    assert_eq!(operation_is_instruction(ops[0]), 1);
    let mut qubits: usize = 0;
    let mut params: usize = 0;
    assert_eq!(operation_gate_arity(ops[0], &mut qubits, &mut params), -3);

    // Delay: delay instruction with fixed (1, 1) arity, not a gate.
    assert_eq!(operation_is_delay(ops[1]), 1);
    assert_eq!(operation_is_quantum_gate(ops[1]), 0);
    assert_eq!(operation_directive(ops[1]), 0);
    assert_eq!(operation_gate_arity(ops[1], &mut qubits, &mut params), 0);
    assert_eq!((qubits, params), (1, 1));

    // MeasureBit: classical data with fixed (1, 0) arity.
    assert_eq!(operation_is_classical_data(ops[2]), 1);
    assert_eq!(operation_is_directive(ops[2]), 0);
    assert_eq!(operation_is_quantum_gate(ops[2]), 0);
    assert_eq!(operation_gate_arity(ops[2], &mut qubits, &mut params), 0);
    assert_eq!((qubits, params), (1, 0));

    for op in &ops {
        operation_free(*op);
    }
    circuit_cfg_free(cfg);
    classical_var_free(var);
    circuit_free(circuit);
}

#[test]
fn operation_result_reports_measurement_value() {
    let circuit = circuit_new(1);
    let var = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 1);
    let meas = circuit_measure_into(circuit, 0, var);
    assert!(!meas.is_null());
    classical_expr_free(meas);

    let cfg = circuit_cfg_from_circuit(circuit);
    // Storage order: [MeasureBit, Store]; measure_into appends both.
    assert_eq!(circuit_cfg_block_operations_len(cfg, 0), 2);
    let ops = block_ops(cfg, 0, 2);
    let meas_op = ops[0];

    let mut info = CClassicalValueInfo {
        index: u32::MAX,
        tag: u32::MAX,
        width: u32::MAX,
    };
    assert_eq!(operation_result(meas_op, &mut info), 0);
    assert_eq!(info.index, 0);
    assert_eq!(info.tag, CQLIB_CLASSICAL_TYPE_BIT);
    assert_eq!(info.width, 1);

    // Non-measurement operations and NULL inputs are rejected.
    let h = op_new("H", &[0], &[]);
    assert_eq!(operation_result(h, &mut info), -3);
    assert_eq!(operation_result(std::ptr::null(), &mut info), -1);
    assert_eq!(operation_result(meas_op, std::ptr::null_mut()), -1);

    operation_free(h);
    for op in &ops {
        operation_free(*op);
    }
    circuit_cfg_free(cfg);
    classical_var_free(var);
    circuit_free(circuit);
}

#[test]
fn operation_reads_value_matches_store_expression() {
    let circuit = circuit_new(1);
    let var = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 1);
    let meas = circuit_measure_into(circuit, 0, var);
    assert!(!meas.is_null());
    let target = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 1);
    assert_eq!(circuit_store(circuit, target, meas), 0);

    // Storage order: [MeasureBit, Store(measure_into), Store(explicit)].
    let cfg = circuit_cfg_from_circuit(circuit);
    assert_eq!(circuit_cfg_block_operations_len(cfg, 0), 3);
    let ops = block_ops(cfg, 0, 3);
    assert_eq!(op_name(ops[0]), "measure_bit");
    assert_eq!(op_name(ops[1]), "store");
    assert_eq!(op_name(ops[2]), "store");
    let meas_op = ops[0];
    let store_op = ops[1];

    // The store operations recursively read the measured value; the
    // measurement itself does not.
    assert_eq!(operation_reads_value(store_op, meas), 1);
    assert_eq!(operation_reads_value(ops[2], meas), 1);
    assert_eq!(operation_reads_value(meas_op, meas), 0);

    // Non-value expressions and NULL inputs are rejected.
    let literal = classical_expr_bit_literal(true);
    assert_eq!(operation_reads_value(store_op, literal), -8);
    assert_eq!(operation_reads_value(std::ptr::null(), meas), -1);
    assert_eq!(operation_reads_value(store_op, std::ptr::null()), -1);

    classical_expr_free(literal);
    classical_expr_free(meas);
    for op in &ops {
        operation_free(*op);
    }
    circuit_cfg_free(cfg);
    classical_var_free(var);
    classical_var_free(target);
    circuit_free(circuit);
}

#[test]
fn circuit_cfg_extend_operations_appends_batch() {
    let cfg = circuit_cfg_new(2);
    let node = circuit_cfg_add_block(cfg);
    assert_ne!(node, u32::MAX);

    let h = op_new("H", &[0], &[]);
    let cx = op_new("CX", &[0, 1], &[]);
    let batch: [*const COperation; 2] = [h as *const _, cx as *const _];
    assert_eq!(
        circuit_cfg_extend_operations(cfg, node, batch.as_ptr(), 2),
        0
    );

    // The two-step pattern agrees with the batch length.
    assert_eq!(circuit_cfg_block_operations_len(cfg, node), 2);
    let ops = block_ops(cfg, node, 2);
    assert_eq!(op_name(ops[0]), "H");
    assert_eq!(op_name(ops[1]), "CX");
    for op in &ops {
        operation_free(*op);
    }

    // Error paths: NULL cfg, sentinel/unknown node, NULL arrays and entries.
    assert_eq!(
        circuit_cfg_extend_operations(std::ptr::null_mut(), node, batch.as_ptr(), 2),
        -1
    );
    assert_eq!(
        circuit_cfg_extend_operations(cfg, u32::MAX, batch.as_ptr(), 2),
        -2
    );
    assert_eq!(
        circuit_cfg_extend_operations(cfg, 99, batch.as_ptr(), 2),
        -2
    );
    assert_eq!(
        circuit_cfg_extend_operations(cfg, node, std::ptr::null(), 2),
        -1
    );
    let with_null: [*const COperation; 2] = [h as *const _, std::ptr::null()];
    assert_eq!(
        circuit_cfg_extend_operations(cfg, node, with_null.as_ptr(), 2),
        -1
    );

    operation_free(h);
    operation_free(cx);
    circuit_cfg_free(cfg);
}

#[test]
fn circuit_cfg_with_label_sets_and_replaces() {
    let cfg = circuit_cfg_new(1);
    let node = circuit_cfg_add_block(cfg);

    assert_eq!(
        circuit_cfg_with_label(cfg, node, cstr("loop-body").as_ptr()),
        0
    );
    let label = circuit_cfg_block_label(cfg, node);
    assert!(!label.is_null());
    assert_eq!(
        unsafe { CStr::from_ptr(label) }.to_str().unwrap(),
        "loop-body"
    );
    cqlib_string_free(label);

    // Setting again replaces the previous label.
    assert_eq!(circuit_cfg_with_label(cfg, node, cstr("exit").as_ptr()), 0);
    let label = circuit_cfg_block_label(cfg, node);
    assert_eq!(unsafe { CStr::from_ptr(label) }.to_str().unwrap(), "exit");
    cqlib_string_free(label);

    // Error paths: NULL inputs, unknown block, invalid UTF-8.
    assert_eq!(
        circuit_cfg_with_label(std::ptr::null_mut(), node, cstr("x").as_ptr()),
        -1
    );
    assert_eq!(circuit_cfg_with_label(cfg, node, std::ptr::null()), -1);
    assert_eq!(
        circuit_cfg_with_label(cfg, u32::MAX, cstr("x").as_ptr()),
        -2
    );
    let invalid = CString::new(vec![0xffu8, 0xfe]).unwrap();
    assert_eq!(circuit_cfg_with_label(cfg, node, invalid.as_ptr()), -4);

    circuit_cfg_free(cfg);
}

#[test]
fn circuit_from_operations_rebuilds_structurally_equal_circuit() {
    let source = circuit_new(2);
    assert_eq!(circuit_h(source, 0), 0);
    assert_eq!(circuit_rx(source, 1, 0.25), 0);

    let op0 = circuit_index(source, 0);
    let op1 = circuit_index(source, 1);
    assert!(!op0.is_null() && !op1.is_null());
    let ops = [op0, op1];
    let ops: Vec<*const CValueOperation> = ops.into_iter().map(|p| p as *const _).collect();
    let qubits = [0u32, 1];

    let rebuilt = circuit_from_operations(qubits.as_ptr(), 2, ops.as_ptr(), 2);
    assert!(!rebuilt.is_null());
    assert_eq!(circuit_num_qubits(rebuilt), 2);
    assert_eq!(circuit_num_operations(rebuilt), 2);
    assert_eq!(circuit_operations_structurally_equal(source, rebuilt), 1);
    assert_eq!(circuit_has_same_qubits(source, rebuilt), 1);
    assert_eq!(circuit_contains_qubit(rebuilt, 1), 1);
    assert_eq!(circuit_contains_qubit(rebuilt, 9), 0);

    // Error paths: NULL arrays, NULL entries, duplicate qubits.
    assert!(circuit_from_operations(qubits.as_ptr(), 2, std::ptr::null(), 2).is_null());
    let with_null: [*const CValueOperation; 2] = [op0 as *const _, std::ptr::null()];
    assert!(circuit_from_operations(qubits.as_ptr(), 2, with_null.as_ptr(), 2).is_null());
    let dup = [0u32, 0];
    assert!(circuit_from_operations(dup.as_ptr(), 2, std::ptr::null(), 0).is_null());

    // An empty construction (no qubits, no operations) is valid.
    let empty = circuit_from_operations(std::ptr::null(), 0, std::ptr::null(), 0);
    assert!(!empty.is_null());
    assert_eq!(circuit_num_qubits(empty), 0);
    assert_eq!(circuit_num_operations(empty), 0);
    circuit_free(empty);

    // NULL inputs to the query helpers.
    assert_eq!(
        circuit_operations_structurally_equal(std::ptr::null(), rebuilt),
        -1
    );
    assert_eq!(
        circuit_operations_structurally_equal(source, std::ptr::null()),
        -1
    );
    assert_eq!(circuit_has_same_qubits(std::ptr::null(), rebuilt), -1);
    assert_eq!(circuit_contains_qubit(std::ptr::null(), 0), -1);

    value_operation_free(op0);
    value_operation_free(op1);
    circuit_free(rebuilt);
    circuit_free(source);
}

#[test]
fn circuit_append_value_operation_appends_snapshot() {
    let source = circuit_new(2);
    assert_eq!(circuit_h(source, 1), 0);
    let op = circuit_index(source, 0);
    assert!(!op.is_null());

    let target = circuit_new(2);
    assert_eq!(circuit_append_value_operation(target, op), 0);
    assert_eq!(circuit_num_operations(target), 1);
    assert_eq!(circuit_operations_structurally_equal(source, target), 1);

    // Appending to a circuit that misses the operation's qubit fails.
    let narrow = circuit_new(1);
    assert_eq!(circuit_append_value_operation(narrow, op), -3);
    assert_eq!(circuit_num_operations(narrow), 0);

    assert_eq!(circuit_append_value_operation(std::ptr::null_mut(), op), -1);
    assert_eq!(circuit_append_value_operation(target, std::ptr::null()), -1);

    value_operation_free(op);
    circuit_free(narrow);
    circuit_free(target);
    circuit_free(source);
}

#[test]
fn circuit_qubit_domain_queries() {
    let qubits = [3u32, 7];
    let a = circuit_from_qubits(qubits.as_ptr(), 2);
    assert!(!a.is_null());
    let b = circuit_from_qubits(qubits.as_ptr(), 2);
    let other = circuit_new(2);

    assert_eq!(circuit_contains_qubit(a, 3), 1);
    assert_eq!(circuit_contains_qubit(a, 7), 1);
    assert_eq!(circuit_contains_qubit(a, 0), 0);
    assert_eq!(circuit_has_same_qubits(a, b), 1);
    assert_eq!(circuit_has_same_qubits(a, other), 0);

    circuit_free(other);
    circuit_free(b);
    circuit_free(a);
}

#[test]
fn circuit_checkpoint_commit_keeps_operations() {
    let circuit = circuit_new(2);

    let checkpoint = circuit_checkpoint(circuit);
    assert!(!checkpoint.is_null());
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_commit(circuit, checkpoint), 0);
    assert_eq!(circuit_num_operations(circuit), 1);

    // NULL handling: construction fails, and commit rejects NULL without
    // consuming the token.
    assert!(circuit_checkpoint(std::ptr::null()).is_null());
    assert_eq!(circuit_commit(circuit, std::ptr::null_mut()), -1);
    let spare = circuit_new(1);
    let checkpoint = circuit_checkpoint(spare);
    assert_eq!(circuit_commit(std::ptr::null_mut(), checkpoint), -1);
    checkpoint_free(checkpoint);

    // An unused token can always be released.
    let checkpoint = circuit_checkpoint(spare);
    checkpoint_free(checkpoint);
    checkpoint_free(std::ptr::null_mut());

    circuit_free(spare);
    circuit_free(circuit);
}

#[test]
fn circuit_rollback_to_restores_state() {
    let circuit = circuit_new(2);

    let checkpoint = circuit_begin(circuit);
    assert!(!checkpoint.is_null());
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_rx(circuit, 0, 0.5), 0);
    assert_eq!(circuit_num_operations(circuit), 2);
    assert_eq!(circuit_rollback_to(circuit, checkpoint), 0);
    assert_eq!(circuit_num_operations(circuit), 0);
    assert_eq!(circuit_parameters_len(circuit), 0);

    // The explicit control-body transaction rollback consumes its token too.
    let other = circuit_new(1);
    let checkpoint = circuit_checkpoint(other);
    assert_eq!(circuit_h(other, 0), 0);
    assert_eq!(
        circuit_rollback_control_body_transaction(other, checkpoint),
        0
    );
    assert_eq!(circuit_num_operations(other), 0);

    // NULL inputs are rejected without consuming anything.
    assert_eq!(
        circuit_rollback_to(std::ptr::null_mut(), std::ptr::null_mut()),
        -1
    );
    let checkpoint = circuit_checkpoint(other);
    assert_eq!(circuit_rollback_to(other, std::ptr::null_mut()), -1);
    checkpoint_free(checkpoint);

    circuit_free(other);
    circuit_free(circuit);
}

#[test]
fn cfg_from_structurally_rebuilt_circuit_roundtrips() {
    // End-to-end: rebuild a circuit from snapshots and confirm the CFG view
    // still expands, including a label set through circuit_cfg_with_label.
    let source = circuit_new(2);
    assert_eq!(circuit_h(source, 0), 0);
    let op = circuit_index(source, 0);
    let ops = [op];
    let ops: Vec<*const CValueOperation> = ops.into_iter().map(|p| p as *const _).collect();
    let qubits = [0u32, 1];
    let rebuilt = circuit_from_operations(qubits.as_ptr(), 2, ops.as_ptr(), 1);
    assert!(!rebuilt.is_null());

    let cfg = circuit_cfg_from_circuit(rebuilt);
    assert!(!cfg.is_null());
    assert_eq!(circuit_cfg_block_operations_len(cfg, 0), 1);
    assert_eq!(
        circuit_cfg_with_label(cfg, 0, cstr("rebuilt-entry").as_ptr()),
        0
    );
    let label = circuit_cfg_block_label(cfg, 0);
    assert_eq!(
        unsafe { CStr::from_ptr(label) }.to_str().unwrap(),
        "rebuilt-entry"
    );
    cqlib_string_free(label);

    circuit_cfg_free(cfg);
    circuit_free(rebuilt);
    value_operation_free(op);
    circuit_free(source);
}
