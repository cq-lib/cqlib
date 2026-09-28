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

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cqlib_c.h"

static int g_failures = 0;

#define CHECK(cond, name)                 \
    do {                                  \
        if (cond) {                       \
            printf("ok: %s\n", (name));   \
        } else {                          \
            printf("FAIL: %s\n", (name)); \
            g_failures++;                 \
        }                                 \
    } while (0)

static void test_dag_from_circuit(void) {
    struct CCircuit* circuit = circuit_new(2);
    CHECK(circuit != NULL, "dag fixture circuit_new");
    CHECK(circuit_h(circuit, 0) == 0, "dag fixture circuit_h");
    CHECK(circuit_cx(circuit, 0, 1) == 0, "dag fixture circuit_cx");
    CHECK(circuit_num_operations(circuit) == 2, "dag fixture two ops");

    struct CCircuitDag* dag = circuit_dag_from_circuit(circuit);
    CHECK(dag != NULL, "circuit_dag_from_circuit");
    CHECK(circuit_dag_validate(dag) == 0, "circuit_dag_validate");
    CHECK(circuit_dag_is_empty(dag) == 0, "circuit_dag_is_empty");
    CHECK(circuit_dag_num_qubits(dag) == 2, "circuit_dag_num_qubits");
    CHECK(circuit_dag_num_ops(dag) == 2, "circuit_dag_num_ops");
    CHECK(circuit_dag_qubits_len(dag) == 2, "circuit_dag_qubits_len");

    uint32_t ids[2] = {9, 9};
    CHECK(circuit_dag_qubits(dag, ids, 2) == 0, "circuit_dag_qubits");
    CHECK(ids[0] == 0 && ids[1] == 1, "circuit_dag_qubits values");

    /* Wire inventory: the global-order resource first, then qubit timelines. */
    CHECK(circuit_dag_wires_len(dag) == 3, "circuit_dag_wires_len");
    uint32_t tags[3] = {99, 99, 99};
    uint32_t wire_ids[3] = {99, 99, 99};
    CHECK(circuit_dag_wires(dag, tags, wire_ids, 3) == 0, "circuit_dag_wires");
    CHECK(
        tags[0] == DAG_WIRE_GLOBAL_ORDER && tags[1] == DAG_WIRE_QUBIT && tags[2] == DAG_WIRE_QUBIT,
        "circuit_dag_wires tags");
    CHECK(wire_ids[0] == 0 && wire_ids[1] == 0 && wire_ids[2] == 1, "circuit_dag_wires ids");

    /* Wire membership probes. */
    CHECK(circuit_dag_has_wire(dag, DAG_WIRE_QUBIT, 0) == 1, "circuit_dag_has_wire qubit 0");
    CHECK(circuit_dag_has_wire(dag, DAG_WIRE_QUBIT, 1) == 1, "circuit_dag_has_wire qubit 1");
    CHECK(circuit_dag_has_wire(dag, DAG_WIRE_QUBIT, 5) == 0, "circuit_dag_has_wire missing qubit");
    CHECK(circuit_dag_has_wire(dag, DAG_WIRE_GLOBAL_ORDER, 0) == 1,
          "circuit_dag_has_wire global order");

    /* Round trip back to a circuit of identical shape. The source handle
     * stays alive and unchanged. */
    struct CCircuit* lowered = circuit_dag_to_circuit(dag);
    CHECK(lowered != NULL, "circuit_dag_to_circuit");
    CHECK(circuit_num_qubits(lowered) == 2, "lowered num_qubits");
    CHECK(circuit_num_operations(lowered) == 2, "lowered num_operations");

    circuit_free(lowered);
    circuit_dag_free(dag);
    CHECK(circuit_num_operations(circuit) == 2, "source circuit unchanged");
    circuit_free(circuit);
}

static void test_dag_from_operations(void) {
    const uint32_t zero[1] = {0};
    const uint32_t one[1] = {1};
    const uint32_t pair[2] = {0, 1};
    const double half[1] = {0.5};
    struct COperation* rx = operation_new("RX", zero, 1, half, 1);
    struct COperation* cx = operation_new("CX", pair, 2, NULL, 0);
    struct COperation* h = operation_new("H", one, 1, NULL, 0);
    CHECK(rx != NULL && cx != NULL && h != NULL, "dag ops fixture");

    const uint32_t qubits[2] = {0, 1};
    const struct COperation* ops[3];
    ops[0] = rx;
    ops[1] = cx;
    ops[2] = h;
    struct CCircuitDag* dag = circuit_dag_from_operations(qubits, 2, ops, 3);
    CHECK(dag != NULL, "circuit_dag_from_operations");
    CHECK(circuit_dag_validate(dag) == 0, "from_operations validate");
    CHECK(circuit_dag_num_qubits(dag) == 2, "from_operations num_qubits");
    CHECK(circuit_dag_num_ops(dag) == 3, "from_operations num_ops");
    CHECK(circuit_dag_is_empty(dag) == 0, "from_operations is_empty");

    /* The narrow constructor has no parameter table: numeric angles yield
     * neither parameters nor symbols. */
    CHECK(circuit_dag_parameters_len(dag) == 0, "from_operations parameters_len");
    CHECK(circuit_dag_symbols_len(dag) == 0, "from_operations symbols_len");

    /* Round trip keeps all three operations. */
    struct CCircuit* lowered = circuit_dag_to_circuit(dag);
    CHECK(lowered != NULL, "from_operations to_circuit");
    CHECK(circuit_num_qubits(lowered) == 2, "from_operations lowered qubits");
    CHECK(circuit_num_operations(lowered) == 3, "from_operations lowered ops");

    circuit_free(lowered);
    circuit_dag_free(dag);
    operation_free(rx);
    operation_free(cx);
    operation_free(h);
}

static void test_dag_builder_rejects_invalid_inputs(void) {
    const uint32_t zero[1] = {0};
    struct COperation* h = operation_new("H", zero, 1, NULL, 0);
    CHECK(h != NULL, "dag builder fixture");
    const struct COperation* ops[1];
    ops[0] = h;

    /* NULL qubit buffer with a non-zero length. */
    CHECK(circuit_dag_from_operations(NULL, 1, NULL, 0) == NULL, "from_operations null qubits");

    /* NULL op buffer with a non-zero length. */
    const uint32_t qubits[1] = {0};
    CHECK(circuit_dag_from_operations(qubits, 1, NULL, 1) == NULL, "from_operations null ops");

    /* Duplicate qubit ids fail the build. */
    const uint32_t dup[2] = {3, 3};
    CHECK(circuit_dag_from_operations(dup, 2, ops, 1) == NULL, "from_operations duplicate qubits");

    /* Operations acting on qubits that were never registered. */
    const uint32_t sparse[2] = {7, 8};
    CHECK(circuit_dag_from_operations(sparse, 2, ops, 1) == NULL,
          "from_operations unregistered qubits");

    /* NULL entries inside the op pointer list. */
    const struct COperation* mixed[2];
    mixed[0] = h;
    mixed[1] = NULL;
    CHECK(circuit_dag_from_operations(qubits, 1, mixed, 2) == NULL,
          "from_operations null op entry");

    /* Building from a NULL circuit is rejected too. */
    CHECK(circuit_dag_from_circuit(NULL) == NULL, "from_circuit null");

    operation_free(h);
}

static void test_dag_parameters_and_symbols(void) {
    struct CParameter* theta = param_parse("theta");
    CHECK(theta != NULL, "symbolic fixture param_parse");

    struct CCircuit* circuit = circuit_new(1);
    CHECK(circuit_rx_param(circuit, 0, theta) == 0, "circuit_rx_param");
    CHECK(circuit_num_parameters(circuit) == 1, "circuit_num_parameters");

    struct CCircuitDag* dag = circuit_dag_from_circuit(circuit);
    CHECK(dag != NULL, "symbolic dag from_circuit");

    /* Parameter names in table order. */
    CHECK(circuit_dag_parameters_len(dag) == 1, "circuit_dag_parameters_len");
    char* names[1] = {NULL};
    CHECK(circuit_dag_parameters(dag, names, 1) == 0, "circuit_dag_parameters");
    CHECK(names[0] != NULL && strcmp(names[0], "theta") == 0, "circuit_dag_parameters value");
    cqlib_string_free(names[0]);

    /* Free-symbol inventory mirrors the parameter expression. */
    CHECK(circuit_dag_symbols_len(dag) == 1, "circuit_dag_symbols_len");
    char* symbols[1] = {NULL};
    CHECK(circuit_dag_symbols(dag, symbols, 1) == 0, "circuit_dag_symbols");
    CHECK(symbols[0] != NULL && strcmp(symbols[0], "theta") == 0, "circuit_dag_symbols value");
    cqlib_string_free(symbols[0]);

    /* Undersized or NULL output buffers. */
    CHECK(circuit_dag_parameters(dag, names, 0) == -8, "circuit_dag_parameters short buffer");
    CHECK(circuit_dag_parameters(dag, NULL, 1) == -1, "circuit_dag_parameters null buffer");
    CHECK(circuit_dag_symbols(dag, symbols, 0) == -8, "circuit_dag_symbols short buffer");
    CHECK(circuit_dag_symbols(dag, NULL, 1) == -1, "circuit_dag_symbols null buffer");

    circuit_dag_free(dag);
    circuit_free(circuit);
    param_free(theta);
}

static void test_dag_empty_and_null_paths(void) {
    /* Every exported function tolerates a NULL handle. */
    CHECK(circuit_dag_from_circuit(NULL) == NULL, "from_circuit(null)");
    CHECK(circuit_dag_to_circuit(NULL) == NULL, "to_circuit(null)");
    CHECK(circuit_dag_validate(NULL) == -1, "validate(null)");
    CHECK(circuit_dag_is_empty(NULL) == -1, "is_empty(null)");
    CHECK(circuit_dag_num_qubits(NULL) == 0, "num_qubits(null)");
    CHECK(circuit_dag_num_ops(NULL) == 0, "num_ops(null)");
    CHECK(circuit_dag_qubits_len(NULL) == 0, "qubits_len(null)");
    CHECK(circuit_dag_parameters_len(NULL) == 0, "parameters_len(null)");
    CHECK(circuit_dag_symbols_len(NULL) == 0, "symbols_len(null)");
    CHECK(circuit_dag_wires_len(NULL) == 0, "wires_len(null)");
    CHECK(circuit_dag_qubits(NULL, NULL, 0) == -1, "qubits(null)");
    CHECK(circuit_dag_parameters(NULL, NULL, 0) == -1, "parameters(null)");
    CHECK(circuit_dag_symbols(NULL, NULL, 0) == -1, "symbols(null)");
    CHECK(circuit_dag_wires(NULL, NULL, NULL, 0) == -1, "wires(null)");
    CHECK(circuit_dag_has_wire(NULL, DAG_WIRE_QUBIT, 0) == -1, "has_wire(null)");
    circuit_dag_free(NULL);
    CHECK(1, "circuit_dag_free(null)");

    /* An empty DAG carries only the global-order wire. */
    struct CCircuitDag* dag = circuit_dag_from_operations(NULL, 0, NULL, 0);
    CHECK(dag != NULL, "empty dag from_operations");
    CHECK(circuit_dag_validate(dag) == 0, "empty dag validate");
    CHECK(circuit_dag_is_empty(dag) == 1, "empty dag is_empty");
    CHECK(circuit_dag_num_qubits(dag) == 0, "empty dag num_qubits");
    CHECK(circuit_dag_num_ops(dag) == 0, "empty dag num_ops");
    CHECK(circuit_dag_qubits_len(dag) == 0, "empty dag qubits_len");
    CHECK(circuit_dag_wires_len(dag) == 1, "empty dag wires_len");
    CHECK(circuit_dag_has_wire(dag, DAG_WIRE_GLOBAL_ORDER, 0) == 1,
          "empty dag has_wire(global order)");
    CHECK(circuit_dag_has_wire(dag, DAG_WIRE_QUBIT, 0) == 0, "empty dag has_wire(qubit)");

    /* Unknown or unsupported wire tags are rejected, not dereferenced. */
    CHECK(circuit_dag_has_wire(dag, DAG_WIRE_CLASSICAL_VAR, 0) == -8, "has_wire classical var tag");
    CHECK(circuit_dag_has_wire(dag, 99, 0) == -8, "has_wire unknown tag");

    /* Wire buffers: undersized then NULL. */
    uint32_t tags[1] = {99};
    uint32_t ids[1] = {99};
    CHECK(circuit_dag_wires(dag, tags, ids, 0) == -8, "wires short buffer");
    CHECK(circuit_dag_wires(dag, NULL, ids, 1) == -1, "wires null tags");
    CHECK(circuit_dag_wires(dag, tags, NULL, 1) == -1, "wires null ids");
    CHECK(circuit_dag_wires(dag, tags, ids, 1) == 0, "wires copy");
    CHECK(tags[0] == DAG_WIRE_GLOBAL_ORDER, "wires tag value");
    CHECK(ids[0] == 0, "wires id value");

    /* A qubit buffer is fine to omit when the DAG has no qubits; the empty
     * parameter/symbol readers also accept len = 0 with no buffer. */
    CHECK(circuit_dag_qubits(dag, NULL, 0) == 0, "empty dag qubits copy");
    CHECK(circuit_dag_parameters(dag, NULL, 0) == 0, "empty dag parameters copy");
    CHECK(circuit_dag_symbols(dag, NULL, 0) == 0, "empty dag symbols copy");

    circuit_dag_free(dag);
}

/* Looks up the ASAP layer index of `node` in a (node, layer) pair table. */
static int node_layer_of(const uint32_t* nodes, const uintptr_t* layers, uintptr_t len,
                         uint32_t node) {
    for (uintptr_t i = 0; i < len; i++) {
        if (nodes[i] == node) {
            return (int)layers[i];
        }
    }
    return -1;
}

static void test_dag_node_identification_and_traversal(void) {
    /* H(0) -> CX(0,1) -> H(1): a chain on qubit 0 with a tail on qubit 1. */
    struct CCircuit* circuit = circuit_new(2);
    CHECK(circuit_h(circuit, 0) == 0, "traversal fixture h0");
    CHECK(circuit_cx(circuit, 0, 1) == 0, "traversal fixture cx");
    CHECK(circuit_h(circuit, 1) == 0, "traversal fixture h1");

    struct CCircuitDag* dag = circuit_dag_from_circuit(circuit);
    CHECK(dag != NULL, "traversal dag from_circuit");
    CHECK(circuit_dag_depth(dag) == 3, "circuit_dag_depth chain");

    /* Operation nodes in source order; a too-small buffer reports -8. */
    CHECK(circuit_dag_op_nodes_len(dag) == 3, "circuit_dag_op_nodes_len");
    uint32_t ops[3] = {9, 9, 9};
    CHECK(circuit_dag_op_nodes(dag, ops, 3) == 0, "circuit_dag_op_nodes");
    CHECK(circuit_dag_op_nodes(dag, ops, 2) == -8, "circuit_dag_op_nodes short buffer");

    /* Every op node reports the operation kind; the middle node holds CX. */
    for (int i = 0; i < 3; i++) {
        CHECK(circuit_dag_is_operation(dag, ops[i]) == 1, "circuit_dag_is_operation");
        char* kind = circuit_dag_node_kind(dag, ops[i]);
        CHECK(kind != NULL && strcmp(kind, "operation") == 0, "circuit_dag_node_kind op");
        cqlib_string_free(kind);
    }
    struct COperation* op = circuit_dag_operation(dag, ops[1]);
    CHECK(op != NULL, "circuit_dag_operation");
    char* name = operation_name(op);
    CHECK(name != NULL && strcmp(name, "CX") == 0, "circuit_dag_operation name");
    cqlib_string_free(name);
    operation_free(op);

    /* Topological order matches source order for this chain. */
    CHECK(circuit_dag_topological_op_nodes_len(dag) == 3, "circuit_dag_topological_op_nodes_len");
    uint32_t topo[3] = {9, 9, 9};
    CHECK(circuit_dag_topological_op_nodes(dag, topo, 3) == 0, "circuit_dag_topological_op_nodes");
    CHECK(topo[0] == ops[0] && topo[1] == ops[1] && topo[2] == ops[2],
          "topological order matches source order");

    /* Predecessors of CX: H(0) plus the qubit-1 input sentinel; quantum
     * neighbors filter the sentinel out. */
    CHECK(circuit_dag_predecessors_len(dag, ops[1]) == 2, "circuit_dag_predecessors_len");
    uint32_t preds[2] = {9, 9};
    CHECK(circuit_dag_predecessors(dag, ops[1], preds, 2) == 0, "circuit_dag_predecessors");
    CHECK(preds[0] == ops[0] || preds[1] == ops[0], "predecessors contain H(0)");

    CHECK(circuit_dag_quantum_predecessors_len(dag, ops[1]) == 1,
          "circuit_dag_quantum_predecessors_len");
    uint32_t qpreds[1] = {9};
    CHECK(circuit_dag_quantum_predecessors(dag, ops[1], qpreds, 1) == 0,
          "circuit_dag_quantum_predecessors");
    CHECK(qpreds[0] == ops[0], "quantum predecessor is H(0)");

    /* Successors of CX: H(1) plus the qubit-0 output sentinel. */
    CHECK(circuit_dag_successors_len(dag, ops[1]) == 2, "circuit_dag_successors_len");
    uint32_t succs[2] = {9, 9};
    CHECK(circuit_dag_successors(dag, ops[1], succs, 2) == 0, "circuit_dag_successors");
    CHECK(succs[0] == ops[2] || succs[1] == ops[2], "successors contain H(1)");

    /* Unknown node ids are reported, not dereferenced. */
    CHECK(circuit_dag_is_operation(dag, 999) == -2, "is_operation unknown node");
    CHECK(circuit_dag_node_kind(dag, 999) == NULL, "node_kind unknown node");
    CHECK(circuit_dag_operation(dag, 999) == NULL, "operation unknown node");

    circuit_dag_free(dag);
    circuit_free(circuit);
}

static void test_dag_layers_and_depth(void) {
    /* Disjoint H(0), H(1) share the front layer; CX depends on both. */
    struct CCircuit* circuit = circuit_new(2);
    CHECK(circuit_h(circuit, 0) == 0, "layers fixture h0");
    CHECK(circuit_h(circuit, 1) == 0, "layers fixture h1");
    CHECK(circuit_cx(circuit, 0, 1) == 0, "layers fixture cx");

    struct CCircuitDag* dag = circuit_dag_from_circuit(circuit);
    CHECK(dag != NULL, "layers dag from_circuit");
    CHECK(circuit_dag_depth(dag) == 2, "circuit_dag_depth disjoint");
    CHECK(circuit_dag_depth(NULL) == -1, "circuit_dag_depth null");

    /* Front layer holds the two disjoint roots. */
    CHECK(circuit_dag_front_layer_len(dag) == 2, "circuit_dag_front_layer_len");
    uint32_t front[2] = {9, 9};
    CHECK(circuit_dag_front_layer(dag, front, 2) == 0, "circuit_dag_front_layer");
    CHECK(circuit_dag_front_layer(dag, front, 1) == -8, "front_layer short buffer");

    /* ASAP layers in CSR style: two nodes in layer 0, CX in layer 1. */
    CHECK(circuit_dag_layers_len(dag) == 2, "circuit_dag_layers_len");
    uint32_t nodes[3] = {9, 9, 9};
    uintptr_t offsets[3] = {9, 9, 9};
    CHECK(circuit_dag_layers(dag, nodes, 3, offsets, 3) == 0, "circuit_dag_layers");
    CHECK(offsets[0] == 0 && offsets[1] == 2 && offsets[2] == 3, "circuit_dag_layers offsets");

    /* Per-node layer entries agree: the H nodes sit in layer 0, CX in 1. */
    CHECK(circuit_dag_node_layers_len(dag) == 3, "circuit_dag_node_layers_len");
    uint32_t pair_nodes[3] = {9, 9, 9};
    uintptr_t pair_layers[3] = {9, 9, 9};
    CHECK(circuit_dag_node_layers(dag, pair_nodes, pair_layers, 3) == 0, "circuit_dag_node_layers");
    CHECK(node_layer_of(pair_nodes, pair_layers, 3, front[0]) == 0, "front[0] in layer 0");
    CHECK(node_layer_of(pair_nodes, pair_layers, 3, front[1]) == 0, "front[1] in layer 0");
    CHECK(node_layer_of(pair_nodes, pair_layers, 3, nodes[2]) == 1, "cx in layer 1");

    circuit_dag_free(dag);
    circuit_free(circuit);
}

static void test_dag_wire_timelines(void) {
    /* Only qubit 0 is driven; qubit 1 stays idle. */
    struct CCircuit* circuit = circuit_new(2);
    CHECK(circuit_h(circuit, 0) == 0, "wire fixture h");

    struct CCircuitDag* dag = circuit_dag_from_circuit(circuit);
    CHECK(dag != NULL, "wire dag from_circuit");

    CHECK(circuit_dag_is_wire_idle(dag, DAG_WIRE_QUBIT, 0) == 0, "is_wire_idle driven");
    CHECK(circuit_dag_is_wire_idle(dag, DAG_WIRE_QUBIT, 1) == 1, "is_wire_idle idle");
    CHECK(circuit_dag_is_wire_idle(dag, DAG_WIRE_QUBIT, 5) == -3, "is_wire_idle unknown qubit");
    CHECK(circuit_dag_is_wire_idle(dag, DAG_WIRE_CLASSICAL_VAR, 0) == -8,
          "is_wire_idle unsupported tag");

    /* The qubit-0 timeline holds exactly the H node, in wire order. */
    CHECK(circuit_dag_nodes_on_wire_len(dag, DAG_WIRE_QUBIT, 0) == 1,
          "circuit_dag_nodes_on_wire_len");
    uint32_t wire_ops[1] = {9};
    CHECK(circuit_dag_nodes_on_wire(dag, DAG_WIRE_QUBIT, 0, wire_ops, 1) == 0,
          "circuit_dag_nodes_on_wire");
    CHECK(circuit_dag_nodes_on_wire(dag, DAG_WIRE_QUBIT, 0, wire_ops, 0) == -8,
          "nodes_on_wire short buffer");
    CHECK(circuit_dag_nodes_on_wire_len(dag, DAG_WIRE_QUBIT, 5) == 0,
          "nodes_on_wire_len unknown qubit");

    /* Sentinel endpoints report their node kinds. */
    int32_t input = circuit_dag_wire_in(dag, DAG_WIRE_QUBIT, 0);
    int32_t output = circuit_dag_wire_out(dag, DAG_WIRE_QUBIT, 0);
    CHECK(input >= 0 && output >= 0 && input != output, "wire sentinels distinct");
    CHECK(circuit_dag_is_operation(dag, (uint32_t)input) == 0, "sentinel is not operation");
    char* kind = circuit_dag_node_kind(dag, (uint32_t)input);
    CHECK(kind != NULL && strcmp(kind, "wire_in") == 0, "wire_in kind");
    cqlib_string_free(kind);
    kind = circuit_dag_node_kind(dag, (uint32_t)output);
    CHECK(kind != NULL && strcmp(kind, "wire_out") == 0, "wire_out kind");
    cqlib_string_free(kind);
    CHECK(circuit_dag_wire_in(dag, DAG_WIRE_QUBIT, 5) == -3, "wire_in unknown qubit");

    circuit_dag_free(dag);
    circuit_free(circuit);
}

static void test_dag_operation_counts_and_measurement_flag(void) {
    struct CCircuit* circuit = circuit_new(2);
    CHECK(circuit_h(circuit, 0) == 0, "counts fixture h0");
    CHECK(circuit_cx(circuit, 0, 1) == 0, "counts fixture cx");
    CHECK(circuit_h(circuit, 1) == 0, "counts fixture h1");
    CHECK(circuit_measure(circuit, 0) == 0, "counts fixture measure");

    struct CCircuitDag* dag = circuit_dag_from_circuit(circuit);
    CHECK(dag != NULL, "counts dag from_circuit");
    CHECK(circuit_dag_has_measurement(dag) == 1, "circuit_dag_has_measurement");
    CHECK(circuit_dag_has_measurement(NULL) == -1, "has_measurement null");
    CHECK(circuit_dag_has_control_flow(dag) == 0, "circuit_dag_has_control_flow");

    /* Top-level counts by instruction name, in insertion order. */
    CHECK(circuit_dag_operation_count_by_name_len(dag) == 3, "operation_count_by_name_len");
    char* names[3] = {NULL, NULL, NULL};
    uintptr_t counts[3] = {0, 0, 0};
    CHECK(circuit_dag_operation_count_by_name(dag, names, counts, 3) == 0,
          "circuit_dag_operation_count_by_name");
    CHECK(names[0] != NULL && strcmp(names[0], "H") == 0 && counts[0] == 2, "count H");
    CHECK(names[1] != NULL && strcmp(names[1], "CX") == 0 && counts[1] == 1, "count CX");
    CHECK(names[2] != NULL && strcmp(names[2], "measure_bit") == 0 && counts[2] == 1,
          "count measure_bit");
    for (int i = 0; i < 3; i++) {
        cqlib_string_free(names[i]);
    }

    /* NULL and undersized output buffers. */
    CHECK(circuit_dag_operation_count_by_name(dag, NULL, counts, 3) == -1, "count null names");
    CHECK(circuit_dag_operation_count_by_name(dag, names, counts, 2) == -8, "count short buffer");

    circuit_dag_free(dag);
    circuit_free(circuit);
}

static void test_dag_mutations(void) {
    const uint32_t zero[1] = {0};
    const uint32_t one[1] = {1};
    struct COperation* z_op = operation_new("Z", zero, 1, NULL, 0);
    struct COperation* x_op = operation_new("X", one, 1, NULL, 0);
    CHECK(z_op != NULL && x_op != NULL, "mutation op fixtures");

    struct CCircuit* circuit = circuit_new(2);
    CHECK(circuit_h(circuit, 0) == 0, "mutation fixture h");
    CHECK(circuit_cx(circuit, 0, 1) == 0, "mutation fixture cx");

    struct CCircuitDag* dag = circuit_dag_from_circuit(circuit);
    CHECK(dag != NULL, "mutation dag from_circuit");
    CHECK(circuit_dag_num_ops(dag) == 2, "mutation initial num_ops");

    /* Append Z(0) at the back: it lands last in source order. */
    uint32_t appended = 9;
    CHECK(circuit_dag_apply_operation_back(dag, z_op, &appended) == 0,
          "circuit_dag_apply_operation_back");
    CHECK(circuit_dag_num_ops(dag) == 3, "num_ops after apply back");
    uint32_t ops[3] = {9, 9, 9};
    CHECK(circuit_dag_op_nodes(dag, ops, 3) == 0, "op_nodes after apply back");
    CHECK(ops[2] == appended, "appended node lands last");

    /* Prepend X(1) at the front: it lands first in source order. */
    uint32_t prepended = 9;
    CHECK(circuit_dag_apply_operation_front(dag, x_op, &prepended) == 0,
          "circuit_dag_apply_operation_front");
    CHECK(circuit_dag_num_ops(dag) == 4, "num_ops after apply front");
    uint32_t ops4[4] = {9, 9, 9, 9};
    CHECK(circuit_dag_op_nodes(dag, ops4, 4) == 0, "op_nodes after apply front");
    CHECK(ops4[0] == prepended, "prepended node lands first");

    /* Remove the front node: the removed operation comes back out. */
    struct COperation* removed = circuit_dag_remove_op_node(dag, prepended);
    CHECK(removed != NULL, "circuit_dag_remove_op_node");
    char* name = operation_name(removed);
    CHECK(name != NULL && strcmp(name, "X") == 0, "removed operation is X");
    cqlib_string_free(name);
    operation_free(removed);
    CHECK(circuit_dag_num_ops(dag) == 3, "num_ops after remove");

    /* Substitute the leading H with Z(0); the node id may be reassigned by
     * the rebuild, so re-read the position. */
    uint32_t ops3[3] = {9, 9, 9};
    CHECK(circuit_dag_op_nodes(dag, ops3, 3) == 0, "op_nodes before substitute");
    CHECK(circuit_dag_substitute_node(dag, ops3[0], z_op) == 0, "circuit_dag_substitute_node");
    CHECK(circuit_dag_num_ops(dag) == 3, "num_ops after substitute");
    CHECK(circuit_dag_op_nodes(dag, ops3, 3) == 0, "op_nodes after substitute");
    struct COperation* lead = circuit_dag_operation(dag, ops3[0]);
    CHECK(lead != NULL, "leading op after substitute");
    name = operation_name(lead);
    CHECK(name != NULL && strcmp(name, "Z") == 0, "substituted op is Z");
    cqlib_string_free(name);
    operation_free(lead);
    CHECK(circuit_dag_validate(dag) == 0, "validate after mutations");

    /* Error paths: unknown node, NULL operation, and a wire sentinel. */
    CHECK(circuit_dag_substitute_node(dag, 9999, z_op) == -2, "substitute unknown node");
    CHECK(circuit_dag_substitute_node(dag, ops3[0], NULL) == -1, "substitute null op");
    int32_t sentinel = circuit_dag_wire_in(dag, DAG_WIRE_QUBIT, 0);
    CHECK(circuit_dag_substitute_node(dag, (uint32_t)sentinel, z_op) == -2,
          "substitute wire sentinel");
    CHECK(circuit_dag_apply_operation_back(dag, NULL, NULL) == -1, "apply back null op");
    CHECK(circuit_dag_remove_op_node(dag, 9999) == NULL, "remove unknown node");
    CHECK(circuit_dag_remove_op_node(NULL, 0) == NULL, "remove null dag");

    circuit_dag_free(dag);
    circuit_free(circuit);
    operation_free(z_op);
    operation_free(x_op);
}

int main(void) {
    test_dag_from_circuit();
    test_dag_from_operations();
    test_dag_builder_rejects_invalid_inputs();
    test_dag_parameters_and_symbols();
    test_dag_empty_and_null_paths();
    test_dag_node_identification_and_traversal();
    test_dag_layers_and_depth();
    test_dag_wire_timelines();
    test_dag_operation_counts_and_measurement_flag();
    test_dag_mutations();

    printf("\n%d failure(s)\n", g_failures);
    return (g_failures == 0) ? 0 : 1;
}
