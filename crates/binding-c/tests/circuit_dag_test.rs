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

//! Rust FFI tests for circuit dependency DAGs (`circuit/dag.rs`).

use binding_c::circuit::{
    COperation, DAG_WIRE_CLASSICAL_VAR, DAG_WIRE_GLOBAL_ORDER, DAG_WIRE_QUBIT, circuit_cx,
    circuit_dag_apply_operation_back, circuit_dag_apply_operation_front, circuit_dag_depth,
    circuit_dag_free, circuit_dag_from_circuit, circuit_dag_from_operations,
    circuit_dag_front_layer, circuit_dag_front_layer_len, circuit_dag_has_control_flow,
    circuit_dag_has_measurement, circuit_dag_has_wire, circuit_dag_is_empty,
    circuit_dag_is_operation, circuit_dag_is_wire_idle, circuit_dag_layers, circuit_dag_layers_len,
    circuit_dag_node_kind, circuit_dag_node_layers, circuit_dag_node_layers_len,
    circuit_dag_nodes_on_wire, circuit_dag_nodes_on_wire_len, circuit_dag_num_ops,
    circuit_dag_num_qubits, circuit_dag_op_nodes, circuit_dag_op_nodes_len, circuit_dag_operation,
    circuit_dag_operation_count_by_name, circuit_dag_operation_count_by_name_len,
    circuit_dag_parameters, circuit_dag_parameters_len, circuit_dag_predecessors,
    circuit_dag_predecessors_len, circuit_dag_quantum_predecessors,
    circuit_dag_quantum_predecessors_len, circuit_dag_qubits, circuit_dag_qubits_len,
    circuit_dag_remove_op_node, circuit_dag_substitute_node, circuit_dag_successors,
    circuit_dag_successors_len, circuit_dag_symbols, circuit_dag_symbols_len,
    circuit_dag_to_circuit, circuit_dag_topological_op_nodes, circuit_dag_topological_op_nodes_len,
    circuit_dag_validate, circuit_dag_wire_in, circuit_dag_wire_out, circuit_dag_wires,
    circuit_dag_wires_len, circuit_free, circuit_h, circuit_measure, circuit_new,
    circuit_num_operations, circuit_num_parameters, circuit_num_qubits, circuit_rx_param,
    operation_free, operation_name, operation_new, param_free, param_parse,
};
use binding_c::cqlib_string_free;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn cstr_of(ptr: *mut c_char) -> &'static str {
    unsafe { CStr::from_ptr(ptr) }.to_str().unwrap()
}

#[test]
fn dag_from_circuit_reflects_structure() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    assert_eq!(circuit_num_operations(circuit), 2);

    let dag = circuit_dag_from_circuit(circuit);
    assert!(!dag.is_null());
    assert_eq!(circuit_dag_validate(dag), 0);
    assert_eq!(circuit_dag_is_empty(dag), 0);
    assert_eq!(circuit_dag_num_qubits(dag), 2);
    assert_eq!(circuit_dag_num_ops(dag), 2);
    assert_eq!(circuit_dag_qubits_len(dag), 2);

    let mut ids = [9u32; 2];
    assert_eq!(circuit_dag_qubits(dag, ids.as_mut_ptr(), 2), 0);
    assert_eq!(ids, [0, 1]);

    // Wire inventory: the global-order resource first, then qubit timelines.
    assert_eq!(circuit_dag_wires_len(dag), 3);
    let mut tags = [99u32; 3];
    let mut wire_ids = [99u32; 3];
    assert_eq!(
        circuit_dag_wires(dag, tags.as_mut_ptr(), wire_ids.as_mut_ptr(), 3),
        0
    );
    assert_eq!(
        tags,
        [DAG_WIRE_GLOBAL_ORDER, DAG_WIRE_QUBIT, DAG_WIRE_QUBIT]
    );
    assert_eq!(wire_ids, [0, 0, 1]);

    // Wire membership probes.
    assert_eq!(circuit_dag_has_wire(dag, DAG_WIRE_QUBIT, 0), 1);
    assert_eq!(circuit_dag_has_wire(dag, DAG_WIRE_QUBIT, 1), 1);
    assert_eq!(circuit_dag_has_wire(dag, DAG_WIRE_QUBIT, 5), 0);
    assert_eq!(circuit_dag_has_wire(dag, DAG_WIRE_GLOBAL_ORDER, 0), 1);

    // Round trip back to a circuit of identical shape. The source handle
    // stays alive and unchanged.
    let lowered = circuit_dag_to_circuit(dag);
    assert!(!lowered.is_null());
    assert_eq!(circuit_num_qubits(lowered), 2);
    assert_eq!(circuit_num_operations(lowered), 2);

    circuit_free(lowered);
    circuit_dag_free(dag);
    assert_eq!(circuit_num_operations(circuit), 2);
    circuit_free(circuit);
}

#[test]
fn dag_from_operations_build() {
    let rx_name = cstr("RX");
    let cx_name = cstr("CX");
    let h_name = cstr("H");
    let zero = [0u32];
    let one = [1u32];
    let pair = [0u32, 1];
    let half = [0.5f64];

    let rx = operation_new(rx_name.as_ptr(), zero.as_ptr(), 1, half.as_ptr(), 1);
    let cx = operation_new(cx_name.as_ptr(), pair.as_ptr(), 2, std::ptr::null(), 0);
    let h = operation_new(h_name.as_ptr(), one.as_ptr(), 1, std::ptr::null(), 0);
    assert!(!rx.is_null());
    assert!(!cx.is_null());
    assert!(!h.is_null());

    let qubits = [0u32, 1];
    let ops: [*const COperation; 3] = [rx, cx, h];
    let dag = circuit_dag_from_operations(qubits.as_ptr(), 2, ops.as_ptr(), 3);
    assert!(!dag.is_null());
    assert_eq!(circuit_dag_validate(dag), 0);
    assert_eq!(circuit_dag_num_qubits(dag), 2);
    assert_eq!(circuit_dag_num_ops(dag), 3);
    assert_eq!(circuit_dag_is_empty(dag), 0);

    // The narrow constructor has no parameter table: numeric angles yield
    // neither parameters nor symbols.
    assert_eq!(circuit_dag_parameters_len(dag), 0);
    assert_eq!(circuit_dag_symbols_len(dag), 0);

    // Round trip keeps all three operations.
    let lowered = circuit_dag_to_circuit(dag);
    assert!(!lowered.is_null());
    assert_eq!(circuit_num_qubits(lowered), 2);
    assert_eq!(circuit_num_operations(lowered), 3);

    circuit_free(lowered);
    circuit_dag_free(dag);
    operation_free(rx);
    operation_free(cx);
    operation_free(h);
}

#[test]
fn dag_builder_rejects_invalid_inputs() {
    let h_name = cstr("H");
    let zero = [0u32];
    let h = operation_new(h_name.as_ptr(), zero.as_ptr(), 1, std::ptr::null(), 0);
    assert!(!h.is_null());
    let ops: [*const COperation; 1] = [h];

    // NULL qubit buffer with a non-zero length.
    assert!(circuit_dag_from_operations(std::ptr::null(), 1, std::ptr::null(), 0).is_null());

    // NULL op buffer with a non-zero length.
    let qubits = [0u32];
    assert!(circuit_dag_from_operations(qubits.as_ptr(), 1, std::ptr::null(), 1).is_null());

    // Duplicate qubit ids fail the build.
    let dup = [3u32, 3];
    assert!(circuit_dag_from_operations(dup.as_ptr(), 2, ops.as_ptr(), 1).is_null());

    // Operations acting on qubits that were never registered.
    let sparse = [7u32, 8];
    assert!(circuit_dag_from_operations(sparse.as_ptr(), 2, ops.as_ptr(), 1).is_null());

    // NULL entries inside the op pointer list.
    let mixed: [*const COperation; 2] = [h, std::ptr::null()];
    assert!(circuit_dag_from_operations(qubits.as_ptr(), 1, mixed.as_ptr(), 2).is_null());

    // Building from a NULL circuit is rejected too.
    assert!(circuit_dag_from_circuit(std::ptr::null()).is_null());

    operation_free(h);
}

#[test]
fn dag_reports_parameters_and_symbols() {
    let theta = cstr("theta");
    let theta_ptr = param_parse(theta.as_ptr());
    assert!(!theta_ptr.is_null());

    let circuit = circuit_new(1);
    assert_eq!(circuit_rx_param(circuit, 0, theta_ptr), 0);
    assert_eq!(circuit_num_parameters(circuit), 1);

    let dag = circuit_dag_from_circuit(circuit);
    assert!(!dag.is_null());

    // Parameter names in table order.
    assert_eq!(circuit_dag_parameters_len(dag), 1);
    let mut names = [std::ptr::null_mut(); 1];
    assert_eq!(circuit_dag_parameters(dag, names.as_mut_ptr(), 1), 0);
    assert!(!names[0].is_null());
    assert_eq!(
        unsafe { CStr::from_ptr(names[0]) }.to_str().unwrap(),
        "theta"
    );
    cqlib_string_free(names[0]);

    // Free-symbol inventory mirrors the parameter expression.
    assert_eq!(circuit_dag_symbols_len(dag), 1);
    let mut symbols = [std::ptr::null_mut(); 1];
    assert_eq!(circuit_dag_symbols(dag, symbols.as_mut_ptr(), 1), 0);
    assert!(!symbols[0].is_null());
    assert_eq!(
        unsafe { CStr::from_ptr(symbols[0]) }.to_str().unwrap(),
        "theta"
    );
    cqlib_string_free(symbols[0]);

    // Undersized or NULL output buffers.
    assert_eq!(circuit_dag_parameters(dag, names.as_mut_ptr(), 0), -8);
    assert_eq!(circuit_dag_parameters(dag, std::ptr::null_mut(), 1), -1);
    assert_eq!(circuit_dag_symbols(dag, symbols.as_mut_ptr(), 0), -8);
    assert_eq!(circuit_dag_symbols(dag, std::ptr::null_mut(), 1), -1);

    circuit_dag_free(dag);
    circuit_free(circuit);
    param_free(theta_ptr);
}

#[test]
fn dag_null_and_error_paths() {
    // Every exported function tolerates a NULL handle.
    assert!(circuit_dag_from_circuit(std::ptr::null()).is_null());
    assert!(circuit_dag_to_circuit(std::ptr::null()).is_null());
    assert_eq!(circuit_dag_validate(std::ptr::null()), -1);
    assert_eq!(circuit_dag_is_empty(std::ptr::null()), -1);
    assert_eq!(circuit_dag_num_qubits(std::ptr::null()), 0);
    assert_eq!(circuit_dag_num_ops(std::ptr::null()), 0);
    assert_eq!(circuit_dag_qubits_len(std::ptr::null()), 0);
    assert_eq!(circuit_dag_parameters_len(std::ptr::null()), 0);
    assert_eq!(circuit_dag_symbols_len(std::ptr::null()), 0);
    assert_eq!(circuit_dag_wires_len(std::ptr::null()), 0);
    assert_eq!(
        circuit_dag_qubits(std::ptr::null(), std::ptr::null_mut(), 0),
        -1
    );
    assert_eq!(
        circuit_dag_parameters(std::ptr::null(), std::ptr::null_mut(), 0),
        -1
    );
    assert_eq!(
        circuit_dag_symbols(std::ptr::null(), std::ptr::null_mut(), 0),
        -1
    );
    assert_eq!(
        circuit_dag_wires(
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0
        ),
        -1
    );
    assert_eq!(
        circuit_dag_has_wire(std::ptr::null(), DAG_WIRE_QUBIT, 0),
        -1
    );
    circuit_dag_free(std::ptr::null_mut());

    // An empty DAG carries only the global-order wire.
    let dag = circuit_dag_from_operations(std::ptr::null(), 0, std::ptr::null(), 0);
    assert!(!dag.is_null());
    assert_eq!(circuit_dag_validate(dag), 0);
    assert_eq!(circuit_dag_is_empty(dag), 1);
    assert_eq!(circuit_dag_num_qubits(dag), 0);
    assert_eq!(circuit_dag_num_ops(dag), 0);
    assert_eq!(circuit_dag_qubits_len(dag), 0);
    assert_eq!(circuit_dag_wires_len(dag), 1);
    assert_eq!(circuit_dag_has_wire(dag, DAG_WIRE_GLOBAL_ORDER, 0), 1);
    assert_eq!(circuit_dag_has_wire(dag, DAG_WIRE_QUBIT, 0), 0);

    // Unknown or unsupported wire tags are rejected, not dereferenced.
    assert_eq!(circuit_dag_has_wire(dag, DAG_WIRE_CLASSICAL_VAR, 0), -8);
    assert_eq!(circuit_dag_has_wire(dag, 99, 0), -8);

    // Wire buffers: undersized then NULL.
    let mut tags = [99u32; 1];
    let mut ids = [99u32; 1];
    assert_eq!(
        circuit_dag_wires(dag, tags.as_mut_ptr(), ids.as_mut_ptr(), 0),
        -8
    );
    assert_eq!(
        circuit_dag_wires(dag, std::ptr::null_mut(), ids.as_mut_ptr(), 1),
        -1
    );
    assert_eq!(
        circuit_dag_wires(dag, tags.as_mut_ptr(), std::ptr::null_mut(), 1),
        -1
    );
    assert_eq!(
        circuit_dag_wires(dag, tags.as_mut_ptr(), ids.as_mut_ptr(), 1),
        0
    );
    assert_eq!(tags, [DAG_WIRE_GLOBAL_ORDER]);
    assert_eq!(ids, [0]);

    // A qubit buffer is fine to omit when the DAG has no qubits; the empty
    // parameter/symbol readers also accept len = 0 with no buffer.
    assert_eq!(circuit_dag_qubits(dag, std::ptr::null_mut(), 0), 0);
    assert_eq!(circuit_dag_parameters(dag, std::ptr::null_mut(), 0), 0);
    assert_eq!(circuit_dag_symbols(dag, std::ptr::null_mut(), 0), 0);

    circuit_dag_free(dag);
}

#[test]
fn dag_node_identification_and_traversal() {
    // H(0) -> CX(0,1) -> H(1): a chain on qubit 0 with a tail on qubit 1.
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    assert_eq!(circuit_h(circuit, 1), 0);

    let dag = circuit_dag_from_circuit(circuit);
    assert!(!dag.is_null());
    assert_eq!(circuit_dag_depth(dag), 3);

    // Operation nodes in source order; a too-small buffer reports -8.
    assert_eq!(circuit_dag_op_nodes_len(dag), 3);
    let mut ops = [9u32; 3];
    assert_eq!(circuit_dag_op_nodes(dag, ops.as_mut_ptr(), 3), 0);
    assert_eq!(circuit_dag_op_nodes(dag, ops.as_mut_ptr(), 2), -8);

    // Every op node reports the operation kind; the middle node holds CX.
    for node in ops {
        assert_eq!(circuit_dag_is_operation(dag, node), 1);
        let kind = circuit_dag_node_kind(dag, node);
        assert_eq!(cstr_of(kind), "operation");
        cqlib_string_free(kind);
    }
    let op = circuit_dag_operation(dag, ops[1]);
    assert!(!op.is_null());
    let name = operation_name(op);
    assert_eq!(cstr_of(name), "CX");
    cqlib_string_free(name);
    operation_free(op);

    // Topological order matches source order for this chain.
    assert_eq!(circuit_dag_topological_op_nodes_len(dag), 3);
    let mut topo = [9u32; 3];
    assert_eq!(
        circuit_dag_topological_op_nodes(dag, topo.as_mut_ptr(), 3),
        0
    );
    assert_eq!(topo, ops);

    // Predecessors of CX: H(0) plus the qubit-1 input sentinel; quantum
    // neighbors filter the sentinel out.
    assert_eq!(circuit_dag_predecessors_len(dag, ops[1]), 2);
    let mut preds = [9u32; 2];
    assert_eq!(
        circuit_dag_predecessors(dag, ops[1], preds.as_mut_ptr(), 2),
        0
    );
    assert!(preds.contains(&ops[0]));

    assert_eq!(circuit_dag_quantum_predecessors_len(dag, ops[1]), 1);
    let mut qpreds = [9u32; 1];
    assert_eq!(
        circuit_dag_quantum_predecessors(dag, ops[1], qpreds.as_mut_ptr(), 1),
        0
    );
    assert_eq!(qpreds[0], ops[0]);

    // Successors of CX: H(1) plus the qubit-0 output sentinel.
    assert_eq!(circuit_dag_successors_len(dag, ops[1]), 2);
    let mut succs = [9u32; 2];
    assert_eq!(
        circuit_dag_successors(dag, ops[1], succs.as_mut_ptr(), 2),
        0
    );
    assert!(succs.contains(&ops[2]));

    // Unknown node ids and NULL handles are reported, not dereferenced.
    assert_eq!(circuit_dag_is_operation(dag, 999), -2);
    assert!(circuit_dag_node_kind(dag, 999).is_null());
    assert!(circuit_dag_operation(dag, 999).is_null());
    assert_eq!(circuit_dag_predecessors_len(std::ptr::null(), 0), 0);
    assert_eq!(
        circuit_dag_predecessors(std::ptr::null(), 0, std::ptr::null_mut(), 0),
        -1
    );

    circuit_dag_free(dag);
    circuit_free(circuit);
}

#[test]
fn dag_layers_and_depth() {
    // Disjoint H(0), H(1) share the front layer; CX depends on both.
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_h(circuit, 1), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let dag = circuit_dag_from_circuit(circuit);
    assert!(!dag.is_null());
    assert_eq!(circuit_dag_depth(dag), 2);
    assert_eq!(circuit_dag_depth(std::ptr::null()), -1);

    // Front layer holds the two disjoint roots.
    assert_eq!(circuit_dag_front_layer_len(dag), 2);
    let mut front = [9u32; 2];
    assert_eq!(circuit_dag_front_layer(dag, front.as_mut_ptr(), 2), 0);
    assert_eq!(circuit_dag_front_layer(dag, front.as_mut_ptr(), 1), -8);

    // ASAP layers in CSR style: two nodes in layer 0, CX in layer 1.
    assert_eq!(circuit_dag_layers_len(dag), 2);
    let mut nodes = [9u32; 3];
    let mut offsets = [9usize; 3];
    assert_eq!(
        circuit_dag_layers(dag, nodes.as_mut_ptr(), 3, offsets.as_mut_ptr(), 3),
        0
    );
    assert_eq!(offsets, [0, 2, 3]);
    let layer0 = [nodes[0], nodes[1]];
    assert!(layer0.contains(&front[0]) && layer0.contains(&front[1]));

    // Per-node layer entries agree: the H nodes sit in layer 0, CX in 1.
    assert_eq!(circuit_dag_node_layers_len(dag), 3);
    let mut pair_nodes = [9u32; 3];
    let mut pair_layers = [9usize; 3];
    assert_eq!(
        circuit_dag_node_layers(dag, pair_nodes.as_mut_ptr(), pair_layers.as_mut_ptr(), 3),
        0
    );
    let layer_of = |node: u32| -> usize {
        pair_nodes
            .iter()
            .zip(pair_layers.iter())
            .find(|(n, _)| **n == node)
            .map(|(_, l)| *l)
            .unwrap()
    };
    assert_eq!(layer_of(front[0]), 0);
    assert_eq!(layer_of(front[1]), 0);
    assert_eq!(layer_of(nodes[2]), 1);

    circuit_dag_free(dag);
    circuit_free(circuit);
}

#[test]
fn dag_wire_timelines() {
    // Only qubit 0 is driven; qubit 1 stays idle.
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);

    let dag = circuit_dag_from_circuit(circuit);
    assert!(!dag.is_null());

    assert_eq!(circuit_dag_is_wire_idle(dag, DAG_WIRE_QUBIT, 0), 0);
    assert_eq!(circuit_dag_is_wire_idle(dag, DAG_WIRE_QUBIT, 1), 1);
    // Unregistered qubits and unsupported tags are rejected.
    assert_eq!(circuit_dag_is_wire_idle(dag, DAG_WIRE_QUBIT, 5), -3);
    assert_eq!(circuit_dag_is_wire_idle(dag, DAG_WIRE_CLASSICAL_VAR, 0), -8);

    // The qubit-0 timeline holds exactly the H node, in wire order.
    assert_eq!(circuit_dag_nodes_on_wire_len(dag, DAG_WIRE_QUBIT, 0), 1);
    let mut wire_ops = [9u32; 1];
    assert_eq!(
        circuit_dag_nodes_on_wire(dag, DAG_WIRE_QUBIT, 0, wire_ops.as_mut_ptr(), 1),
        0
    );
    assert_eq!(
        circuit_dag_nodes_on_wire(dag, DAG_WIRE_QUBIT, 0, wire_ops.as_mut_ptr(), 0),
        -8
    );
    assert_eq!(circuit_dag_nodes_on_wire_len(dag, DAG_WIRE_QUBIT, 5), 0);

    // Sentinel endpoints report their node kinds.
    let input = circuit_dag_wire_in(dag, DAG_WIRE_QUBIT, 0);
    let output = circuit_dag_wire_out(dag, DAG_WIRE_QUBIT, 0);
    assert!(input >= 0 && output >= 0 && input != output);
    assert_eq!(circuit_dag_is_operation(dag, input as u32), 0);
    let kind = circuit_dag_node_kind(dag, input as u32);
    assert_eq!(cstr_of(kind), "wire_in");
    cqlib_string_free(kind);
    let kind = circuit_dag_node_kind(dag, output as u32);
    assert_eq!(cstr_of(kind), "wire_out");
    cqlib_string_free(kind);
    assert_eq!(circuit_dag_wire_in(dag, DAG_WIRE_QUBIT, 5), -3);

    circuit_dag_free(dag);
    circuit_free(circuit);
}

#[test]
fn dag_operation_counts_and_measurement_flag() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    assert_eq!(circuit_h(circuit, 1), 0);
    assert_eq!(circuit_measure(circuit, 0), 0);

    let dag = circuit_dag_from_circuit(circuit);
    assert!(!dag.is_null());
    assert_eq!(circuit_dag_has_measurement(dag), 1);
    assert_eq!(circuit_dag_has_measurement(std::ptr::null()), -1);
    assert_eq!(circuit_dag_has_control_flow(dag), 0);

    // Top-level counts by instruction name, in insertion order.
    assert_eq!(circuit_dag_operation_count_by_name_len(dag), 3);
    let mut names = [std::ptr::null_mut(); 3];
    let mut counts = [0usize; 3];
    assert_eq!(
        circuit_dag_operation_count_by_name(dag, names.as_mut_ptr(), counts.as_mut_ptr(), 3),
        0
    );
    let totals: Vec<(String, usize)> = names
        .iter()
        .zip(counts.iter())
        .map(|(name, count)| (cstr_of(*name).to_string(), *count))
        .collect();
    for name in names {
        cqlib_string_free(name);
    }
    assert_eq!(
        totals,
        vec![
            ("H".to_string(), 2),
            ("CX".to_string(), 1),
            ("measure_bit".to_string(), 1),
        ]
    );

    // NULL and undersized output buffers.
    assert_eq!(
        circuit_dag_operation_count_by_name(dag, std::ptr::null_mut(), counts.as_mut_ptr(), 3),
        -1
    );
    assert_eq!(
        circuit_dag_operation_count_by_name(dag, names.as_mut_ptr(), counts.as_mut_ptr(), 2),
        -8
    );
    assert_eq!(circuit_dag_operation_count_by_name_len(std::ptr::null()), 0);

    circuit_dag_free(dag);
    circuit_free(circuit);
}

#[test]
fn dag_mutations() {
    let z_name = cstr("Z");
    let x_name = cstr("X");
    let zero = [0u32];
    let one = [1u32];
    let z_op = operation_new(z_name.as_ptr(), zero.as_ptr(), 1, std::ptr::null(), 0);
    let x_op = operation_new(x_name.as_ptr(), one.as_ptr(), 1, std::ptr::null(), 0);
    assert!(!z_op.is_null());
    assert!(!x_op.is_null());

    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let dag = circuit_dag_from_circuit(circuit);
    assert!(!dag.is_null());
    assert_eq!(circuit_dag_num_ops(dag), 2);

    // Append Z(0) at the back: it lands last in source order.
    let mut appended = 9u32;
    assert_eq!(
        circuit_dag_apply_operation_back(dag, z_op, &mut appended),
        0
    );
    assert_eq!(circuit_dag_num_ops(dag), 3);
    let mut ops = [9u32; 3];
    assert_eq!(circuit_dag_op_nodes(dag, ops.as_mut_ptr(), 3), 0);
    assert_eq!(ops[2], appended);

    // Prepend X(1) at the front: it lands first in source order.
    let mut prepended = 9u32;
    assert_eq!(
        circuit_dag_apply_operation_front(dag, x_op, &mut prepended),
        0
    );
    assert_eq!(circuit_dag_num_ops(dag), 4);
    let mut ops = [9u32; 4];
    assert_eq!(circuit_dag_op_nodes(dag, ops.as_mut_ptr(), 4), 0);
    assert_eq!(ops[0], prepended);

    // Remove the front node: the removed operation comes back out.
    let removed = circuit_dag_remove_op_node(dag, prepended);
    assert!(!removed.is_null());
    let name = operation_name(removed);
    assert_eq!(cstr_of(name), "X");
    cqlib_string_free(name);
    operation_free(removed);
    assert_eq!(circuit_dag_num_ops(dag), 3);

    // Substitute the leading H with Z(0); the op count is unchanged and the
    // node id may be reassigned by the rebuild, so re-read the position.
    let mut ops = [9u32; 3];
    assert_eq!(circuit_dag_op_nodes(dag, ops.as_mut_ptr(), 3), 0);
    assert_eq!(circuit_dag_substitute_node(dag, ops[0], z_op), 0);
    assert_eq!(circuit_dag_num_ops(dag), 3);
    let mut ops = [9u32; 3];
    assert_eq!(circuit_dag_op_nodes(dag, ops.as_mut_ptr(), 3), 0);
    let op = circuit_dag_operation(dag, ops[0]);
    assert!(!op.is_null());
    let name = operation_name(op);
    assert_eq!(cstr_of(name), "Z");
    cqlib_string_free(name);
    operation_free(op);
    assert_eq!(circuit_dag_validate(dag), 0);

    // Error paths: unknown node, NULL operation, and a wire sentinel.
    assert_eq!(circuit_dag_substitute_node(dag, 9999, z_op), -2);
    assert_eq!(
        circuit_dag_substitute_node(dag, ops[0], std::ptr::null()),
        -1
    );
    let sentinel = circuit_dag_wire_in(dag, DAG_WIRE_QUBIT, 0);
    assert_eq!(circuit_dag_substitute_node(dag, sentinel as u32, z_op), -2);
    assert_eq!(
        circuit_dag_apply_operation_back(dag, std::ptr::null(), std::ptr::null_mut()),
        -1
    );
    assert!(circuit_dag_remove_op_node(dag, 9999).is_null());
    assert!(circuit_dag_remove_op_node(std::ptr::null_mut(), 0).is_null());

    circuit_dag_free(dag);
    circuit_free(circuit);
    operation_free(z_op);
    operation_free(x_op);
}
