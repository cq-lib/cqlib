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

#include <assert.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cqlib_c.h"

static void test_device_topology_variants(void) {
    CDevice* line = device_line("line", 4);
    assert(line != NULL);
    CDevice* biline = device_bidirectional_line("biline", 4);
    assert(biline != NULL);
    CDevice* ring = device_ring("ring", 4);
    assert(ring != NULL);
    CDevice* grid = device_grid("grid", 2, 2);
    assert(grid != NULL);
    CDevice* star = device_star("star", 4, 0);
    assert(star != NULL);

    device_free(line);
    device_free(biline);
    device_free(ring);
    device_free(grid);
    device_free(star);
    device_free(NULL);
    assert(device_new(NULL, 2) == NULL);
}

static void test_device_edges_and_properties(void) {
    const uint32_t edges[] = {0, 1, 1, 2, 1, 3, 3, 4};
    CDevice* device = device_from_edges("t-shape", 5, edges, 4);
    assert(device != NULL);

    char* name = device_name(device);
    assert(name != NULL);
    assert(strcmp(name, "t-shape") == 0);
    cqlib_string_free(name);

    assert(device_num_qubits(device) == 5);

    CTopology* topology = device_topology(device);
    assert(topology != NULL);
    assert(topology_num_qubits(topology) == 5);
    assert(topology_num_couplings(topology) == 4);
    topology_free(topology);
    topology_free(NULL);

    /* Edge referencing a qubit beyond num_qubits must be rejected. */
    const uint32_t bad_edges[] = {0, 9};
    assert(device_from_edges("bad", 2, bad_edges, 1) == NULL);

    device_free(device);
}

static void test_device_native_gates_and_validation(void) {
    CDevice* device = device_line("dev", 2);
    assert(device != NULL);
    assert(device_with_native_gates(device, "H,CX,RZ") == 0);

    char* gates = device_native_gates(device);
    assert(gates != NULL);
    assert(strstr(gates, "H") != NULL);
    assert(strstr(gates, "CX") != NULL);
    cqlib_string_free(gates);

    /* Circuit uses only native gates -> valid. */
    CCircuit* circuit = circuit_new(2);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);
    assert(device_validate_circuit(device, circuit) == 0);

    /* T is not a native gate here -> invalid. */
    assert(circuit_t(circuit, 0) == 0);
    assert(device_validate_circuit(device, circuit) == -3);

    circuit_free(circuit);
    assert(device_with_native_gates(device, "NOT_A_GATE") == -4);
    device_free(device);
}

static void test_layout(void) {
    const uint32_t logical[] = {0, 1, 2};
    const uint32_t physical[] = {4, 3, 1};
    CLayout* layout = layout_new(logical, 3, physical, 3);
    assert(layout != NULL);
    assert(layout_num_logical(layout) == 3);
    assert(layout_num_physical(layout) == 3);
    assert(layout_get(layout, 0) == 4);
    assert(layout_get(layout, 2) == 1);
    /* Unmapped logical qubit -> u32 max sentinel. */
    assert(layout_get(layout, 9) == UINT32_MAX);
    layout_free(layout);

    const uint32_t pairs[] = {0, 4, 1, 3, 2, 1};
    CLayout* from_pairs = layout_from_pairs(pairs, 3, 5);
    assert(from_pairs != NULL);
    assert(layout_get(from_pairs, 1) == 3);
    layout_free(from_pairs);

    /* Mismatched array lengths must be rejected. */
    assert(layout_new(logical, 3, physical, 2) == NULL);
    layout_free(NULL);
}

static void test_noise_model(void) {
    CNoiseModel* model = noise_model_new();
    assert(model != NULL);

    assert(noise_model_add_single_qubit(model, "H", 0, NOISE_DEPOLARIZING, 0.001) == 0);
    assert(noise_model_add_single_qubit(model, "BOGUS", 0, NOISE_DEPOLARIZING, 0.001) == -4);
    assert(noise_model_add_single_qubit(model, "H", 0, 99, 0.001) == -8);
    assert(noise_model_add_single_qubit(model, "H", 0, NOISE_DEPOLARIZING, 1.5) == -8);

    assert(noise_model_add_two_qubit(model, "CX", 0, 1, NOISE_TWO_DEPOLARIZING, 0.01) == 0);
    assert(noise_model_add_two_qubit(model, "CX", 1, 1, NOISE_TWO_DEPOLARIZING, 0.01) == -8);

    assert(noise_model_add_readout(model, 0, 0.02, 0.01) == 0);
    assert(noise_model_add_readout(model, 1, 0.5, 1.7) == -8);

    noise_model_free(model);
    noise_model_free(NULL);
}

static void test_execution_result(void) {
    const char* bitstrings[] = {"00", "01", "11"};
    const uint64_t counts[] = {480, 40, 480};
    CExecutionResult* result =
        execution_result_from_counts("task-001", 2, 1000, bitstrings, counts, 3);
    assert(result != NULL);
    assert(execution_result_shots(result) == 1000);
    assert(execution_result_num_qubits(result) == 2);

    CCountsList* counts_list = execution_result_counts(result);
    assert(counts_list != NULL);
    assert(counts_list_len(counts_list) == 3);
    double total = 0.0;
    for (uintptr_t i = 0; i < counts_list_len(counts_list); i++) {
        char* key = counts_list_get_key(counts_list, i);
        assert(key != NULL);
        total += counts_list_get_value(counts_list, i);
        cqlib_string_free(key);
    }
    assert(total == 1000.0);
    counts_list_free(counts_list);

    CCountsList* probs = execution_result_probabilities(result);
    assert(probs != NULL);
    double prob_total = 0.0;
    for (uintptr_t i = 0; i < counts_list_len(probs); i++) {
        prob_total += counts_list_get_value(probs, i);
    }
    assert(fabs(prob_total - 1.0) < 1e-9);
    counts_list_free(probs);

    execution_result_free(result);
    execution_result_free(NULL);
    assert(execution_result_from_counts(NULL, 2, 10, bitstrings, counts, 3) == NULL);
    counts_list_free(NULL);
}

/* =====  Section 3.1: device attribute queries and updates  ===== */

static void test_device_attributes(void) {
    CDevice* device = device_new("dev", 4);
    assert(device != NULL);

    /* Defaults start unset (-8, NaN) and round-trip after set. */
    double v = 0.0;
    assert(device_default_t1(device, &v) == -8 && isnan(v));
    assert(device_default_t2(device, &v) == -8);
    assert(device_default_readout_error(device, &v) == -8);
    assert(device_default_single_qubit_error(device, &v) == -8);
    assert(device_default_two_qubit_error(device, &v) == -8);
    assert(device_set_default_t1(device, 40.0) == 0);
    assert(device_set_default_t2(device, 35.0) == 0);
    assert(device_set_default_readout_error(device, 0.02) == 0);
    assert(device_set_default_single_qubit_error(device, 0.005) == 0);
    assert(device_set_default_two_qubit_error(device, 0.01) == 0);
    assert(device_default_t1(device, &v) == 0 && v == 40.0);
    assert(device_default_two_qubit_error(device, &v) == 0 && v == 0.01);

    /* Per-qubit queries fall back to the defaults. */
    assert(device_get_t1(device, 0, &v) == 0 && v == 40.0);
    assert(device_get_t2(device, 2, &v) == 0 && v == 35.0);
    assert(device_get_readout_error(device, 3, &v) == 0 && v == 0.02);

    int64_t ms = 0;
    assert(device_calibration_time(device, &ms) == -8);

    /* Invalid-qubit bookkeeping (two-step array output). */
    const uint32_t invalid[2] = {1, 3};
    assert(device_set_invalid_qubits(device, invalid, 2) == 0);
    assert(device_invalid_qubits_len(device) == 2);
    uint32_t got_invalid[2] = {0};
    assert(device_invalid_qubits(device, got_invalid, 2) == 2);
    assert(((got_invalid[0] == 1 && got_invalid[1] == 3)) ||
           (got_invalid[0] == 3 && got_invalid[1] == 1));
    assert(device_is_usable_qubit(device, 0) == 1);
    assert(device_is_usable_qubit(device, 1) == 0);
    assert(device_is_usable_qubit(device, 9) == 0);
    assert(device_num_usable_qubits(device) == 2);
    uint32_t usable[2] = {0};
    assert(device_usable_qubits_len(device) == 2);
    assert(device_usable_qubits(device, usable, 2) == 2);
    assert(((usable[0] == 0 && usable[1] == 2)) || (usable[0] == 2 && usable[1] == 0));
    assert(device_qubits_len(device) == 4);
    uint32_t all[4] = {0};
    assert(device_qubits(device, all, 4) == 4);
    for (uintptr_t i = 0; i < 4; i++) {
        assert(all[i] == (uint32_t)i);
    }

    /* Unregistered qubits are rejected and the set survives. */
    const uint32_t bad[1] = {9};
    assert(device_set_invalid_qubits(device, bad, 1) == -2);
    assert(device_invalid_qubits_len(device) == 2);
    /* An empty set restores full usability. */
    assert(device_set_invalid_qubits(device, NULL, 0) == 0);
    assert(device_num_usable_qubits(device) == 4);

    /* NULL guards. */
    assert(device_get_t1(NULL, 0, &v) == -1);
    assert(device_set_default_t1(NULL, 1.0) == -1);
    assert(device_set_invalid_qubits(NULL, invalid, 2) == -1);
    assert(device_is_usable_qubit(NULL, 0) == -1);

    device_free(device);
}

static void test_device_qubit_edge_properties(void) {
    CDevice* device = device_line("line", 3);
    assert(device != NULL);

    CQubitProp qprop = {0};
    CEdgeProp eprop = {0};
    double v = 0.0;

    /* Nothing recorded yet. */
    assert(device_qubit_properties(device, 0, &qprop) == -8);
    assert(device_edge_properties(device, 0, 1, &eprop) == -8);
    /* H is not a device-wide native gate -> unsupported. */
    assert(device_single_qubit_error(device, "H", 0, &v) == -8);

    /* Record qubit-0 properties with one native H instruction. */
    const char* names[1] = {"H"};
    const double rates[1] = {0.001};
    const double lengths[1] = {50.0};
    CQubitPropInput qin = {
        .readout_error = 0.01,
        .t1 = 20.0,
        .t2 = 30.0,
        .prob_meas0_prep1 = 0.02,
        .prob_meas1_prep0 = 0.03,
        .frequency = 5.0,
        .native_gate_names = names,
        .native_error_rates = rates,
        .native_lengths = lengths,
        .num_native_instructions = 1,
    };
    assert(device_add_qubit_properties(device, 0, &qin) == 0);
    assert(device_qubit_properties(device, 0, &qprop) == 0);
    assert(qprop.readout_error == 0.01 && qprop.t1 == 20.0 && qprop.t2 == 30.0);
    assert(qprop.prob_meas0_prep1 == 0.02 && qprop.prob_meas1_prep0 == 0.03);
    assert(qprop.frequency == 5.0 && qprop.num_native_instructions == 1);

    CNativeInstruction native = {0};
    assert(device_qubit_prop_native_instruction(device, 0, 0, &native) == 0);
    assert(native.name != NULL && strcmp(native.name, "H") == 0);
    assert(native.error_rate == 0.001 && native.length == 50.0);
    cqlib_string_free(native.name);
    assert(device_qubit_prop_native_instruction(device, 0, 1, &native) == -8);

    /* Per-qubit values win over the unset defaults. */
    assert(device_get_t1(device, 0, &v) == 0 && v == 20.0);
    assert(device_get_readout_error(device, 0, &v) == 0 && v == 0.01);

    /* A non-empty local list is authoritative. */
    assert(device_single_qubit_error(device, "H", 0, &v) == 0 && v == 0.001);
    assert(device_single_qubit_error(device, "X", 0, &v) == -8);
    assert(device_single_qubit_error(device, "CX", 0, &v) == -8);
    assert(device_single_qubit_error(device, "BOGUS", 0, &v) == -4);

    /* Record edge (0 -> 1) properties with one native CX instruction. */
    const char* edge_names[1] = {"CX"};
    const double edge_rates[1] = {0.02};
    const double edge_lengths[1] = {100.0};
    CEdgePropInput ein = {
        .native_gate_names = edge_names,
        .native_error_rates = edge_rates,
        .native_lengths = edge_lengths,
        .num_native_instructions = 1,
    };
    assert(device_add_edge_properties(device, 0, 1, &ein) == 0);
    assert(device_edge_properties(device, 0, 1, &eprop) == 0);
    assert(eprop.num_native_instructions == 1);
    assert(device_edge_prop_native_instruction(device, 0, 1, 0, &native) == 0);
    assert(native.name != NULL && strcmp(native.name, "CX") == 0);
    assert(native.error_rate == 0.02 && native.length == 100.0);
    cqlib_string_free(native.name);
    assert(device_edge_prop_native_instruction(device, 0, 1, 1, &native) == -8);

    assert(device_two_qubit_error(device, "CX", 0, 1, &v) == 0 && v == 0.02);
    assert(device_two_qubit_error(device, "CX", 1, 0, &v) == -8);
    assert(device_two_qubit_error(device, "CX", 0, 2, &v) == -8);
    assert(device_two_qubit_error(device, "H", 0, 1, &v) == -8);
    assert(device_two_qubit_error(device, "BOGUS", 0, 1, &v) == -4);

    assert(device_edge_error(device, 0, 1, &v) == 0 && v == 0.02);
    assert(device_edge_error(device, 1, 0, &v) == -8);
    assert(device_edge_error(device, 1, 2, &v) == -8);

    /* Unregistered targets are rejected; bad payloads map to -4/-8. */
    assert(device_add_qubit_properties(device, 9, &qin) == -2);
    assert(device_add_edge_properties(device, 2, 1, &ein) == -2);
    assert(device_add_qubit_properties(device, 1, NULL) == -1);
    const char* bogus_names[1] = {"BOGUS"};
    CEdgePropInput bad_ein = {
        .native_gate_names = bogus_names,
        .native_error_rates = edge_rates,
        .native_lengths = NULL,
        .num_native_instructions = 1,
    };
    assert(device_add_edge_properties(device, 0, 1, &bad_ein) == -4);
    const double bad_rate[1] = {1.5};
    CEdgePropInput bad_rate_ein = {
        .native_gate_names = edge_names,
        .native_error_rates = bad_rate,
        .native_lengths = NULL,
        .num_native_instructions = 1,
    };
    assert(device_add_edge_properties(device, 0, 1, &bad_rate_ein) == -8);

    /* Capability queries: exact ordered qargs decide. */
    const uint32_t q0[1] = {0};
    const uint32_t q1[1] = {1};
    const uint32_t pair01[2] = {0, 1};
    const uint32_t pair10[2] = {1, 0};
    assert(device_supports_native_instruction(device, "H", q0, 1) == 1);
    assert(device_supports_native_instruction(device, "H", q1, 1) == 0);
    assert(device_supports_native_instruction(device, "CX", pair01, 2) == 1);
    assert(device_supports_native_instruction(device, "CX", pair10, 2) == 0);
    assert(device_supports_native_instruction(device, "CX", q0, 1) == 0);
    assert(device_supports_native_instruction(device, "BOGUS", q0, 1) == -4);
    assert(device_supports_native_instruction(device, "H", NULL, 1) == -1);
    assert(device_supports_native_instruction(device, "GPhase", NULL, 0) == 0);

    device_free(device);
}

static void test_device_line_from_qubits_and_validate_operation(void) {
    const uint32_t qubits[3] = {5, 6, 7};
    CDevice* device = device_line_from_qubits("line7", qubits, 3);
    assert(device != NULL);
    assert(device_qubits_len(device) == 3);
    uint32_t all[3] = {0};
    assert(device_qubits(device, all, 3) == 3);
    assert(all[0] == 5 && all[1] == 6 && all[2] == 7);
    device_free(device);
    assert(device_line_from_qubits(NULL, qubits, 3) == NULL);
    assert(device_line_from_qubits("x", NULL, 3) == NULL);

    /* validate_operation consumes a CValueOperation snapshot from circuit_index. */
    CDevice* dev = device_line("dev", 2);
    assert(dev != NULL);
    assert(device_with_native_gates(dev, "H,CX") == 0);
    CCircuit* circuit = circuit_new(2);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);
    CValueOperation* op = circuit_index(circuit, 0);
    assert(op != NULL);
    assert(device_validate_operation(dev, op) == 0);
    assert(device_validate_value_operation(dev, op) == 0);
    value_operation_free(op);
    assert(circuit_t(circuit, 0) == 0);
    CValueOperation* bad_op = circuit_index(circuit, 2);
    assert(bad_op != NULL);
    assert(device_validate_operation(dev, bad_op) == -3);
    assert(device_validate_value_operation(dev, bad_op) == -3);
    value_operation_free(bad_op);
    assert(device_validate_operation(dev, NULL) == -1);
    circuit_free(circuit);
    device_free(dev);
}

/* =====  Section 3.2: layout binding and map introspection  ===== */

static void test_layout_mapping(void) {
    const uint32_t pairs[4] = {0, 2, 1, 0}; /* L0->P2, L1->P0 */
    CLayout* layout = layout_from_pairs(pairs, 2, 4);
    assert(layout != NULL);

    /* Reverse lookup and vacancy checks. */
    assert(layout_get_logical(layout, 2) == 0);
    assert(layout_get_logical(layout, 0) == 1);
    assert(layout_get_logical(layout, 1) == UINT32_MAX);
    assert(layout_is_physical_vacant(layout, 1) == 1);
    assert(layout_is_physical_vacant(layout, 3) == 1);
    assert(layout_is_physical_vacant(layout, 0) == 0);
    assert(layout_is_physical_vacant(layout, 9) == 0);

    /* Bind / unbind. */
    assert(layout_bind(layout, 2, 1) == 0);
    assert(layout_get(layout, 2) == 1);
    assert(layout_bind(layout, 2, 3) == -8);
    assert(layout_bind(layout, 3, 9) == -2);
    uint32_t released = 0;
    assert(layout_unbind(layout, 9, &released) == -8);

    /* Full map introspection (two-step arrays). */
    uint32_t l2p[6] = {0};
    assert(layout_l2p_map_len(layout) == 3);
    assert(layout_l2p_map(layout, l2p, 3) == 3);
    assert(l2p[0] == 0 && l2p[1] == 2 && l2p[2] == 1 && l2p[3] == 0 && l2p[4] == 2 && l2p[5] == 1);
    uint32_t p2l[6] = {0};
    assert(layout_p2l_map_len(layout) == 3);
    assert(layout_p2l_map(layout, p2l, 3) == 3);
    assert(p2l[0] == 0 && p2l[1] == 1 && p2l[2] == 1 && p2l[3] == 2 && p2l[4] == 2 && p2l[5] == 0);
    uint32_t logical[3] = {0};
    assert(layout_logical_qubits_len(layout) == 3);
    assert(layout_logical_qubits(layout, logical, 3) == 3);
    uint32_t physical[4] = {0};
    assert(layout_physical_qubits_len(layout) == 4);
    assert(layout_physical_qubits(layout, physical, 4) == 4);
    assert(physical[0] == 0 && physical[1] == 1 && physical[2] == 2 && physical[3] == 3);
    uint32_t vacant[1] = {0};
    assert(layout_num_vacant_physical(layout) == 1);
    assert(layout_vacant_physical_qubits_len(layout) == 1);
    assert(layout_vacant_physical_qubits(layout, vacant, 1) == 1);
    assert(vacant[0] == 3);

    /* Unbind releases the physical qubit; swap can move onto a vacant one. */
    assert(layout_unbind(layout, 0, &released) == 0 && released == 2);
    assert(layout_get(layout, 0) == UINT32_MAX);
    assert(layout_num_vacant_physical(layout) == 2);
    assert(layout_swap_physical(layout, 1, 3) == 0);
    assert(layout_get(layout, 2) == 3);
    assert(layout_get_logical(layout, 1) == UINT32_MAX);
    assert(layout_get_logical(layout, 3) == 2);
    assert(layout_swap_physical(layout, 1, 99) == -2);

    /* NULL guards. */
    assert(layout_bind(NULL, 0, 0) == -1);
    assert(layout_unbind(NULL, 0, &released) == -1);
    assert(layout_get_logical(NULL, 0) == UINT32_MAX);
    assert(layout_is_physical_vacant(NULL, 0) == -1);
    assert(layout_num_vacant_physical(NULL) == 0);
    assert(layout_swap_physical(NULL, 0, 1) == -1);

    layout_free(layout);
}

/* =====  Section 3.3: topology graph queries and updates  ===== */

static void test_topology_graph(void) {
    const uint32_t qubits[3] = {0, 1, 2};
    CTopology* topo = topology_line(qubits, 3);
    assert(topo != NULL);
    assert(topology_num_qubits(topo) == 3);
    assert(topology_num_couplings(topo) == 2);

    assert(topology_contains_qubit(topo, 0) == 1);
    assert(topology_contains_qubit(topo, 9) == 0);
    assert(topology_contains_qubit(NULL, 0) == -1);

    uint32_t all[3] = {0};
    assert(topology_qubits_len(topo) == 3);
    assert(topology_qubits(topo, all, 3) == 3);
    for (uintptr_t i = 0; i < 3; i++) {
        assert(all[i] == (uint32_t)i);
    }

    /* Directed line 0 -> 1 -> 2: neighbors, degrees, adjacency. */
    uint32_t neighbors[2] = {0};
    assert(topology_neighbors_undirected_len(topo, 1) == 2);
    assert(topology_neighbors_undirected(topo, 1, neighbors, 2) == 2);
    assert((neighbors[0] == 0 && neighbors[1] == 2) || (neighbors[0] == 2 && neighbors[1] == 0));
    uint32_t preds[1] = {0};
    uint32_t succs[1] = {0};
    assert(topology_predecessors_len(topo, 1) == 1);
    assert(topology_predecessors(topo, 1, preds, 1) == 1 && preds[0] == 0);
    assert(topology_successors_len(topo, 1) == 1);
    assert(topology_successors(topo, 1, succs, 1) == 1 && succs[0] == 2);
    assert(topology_in_degree(topo, 1) == 1);
    assert(topology_out_degree(topo, 1) == 1);
    assert(topology_neighbors_undirected_len(topo, 9) == 0);
    assert(topology_in_degree(topo, 9) == 0);

    assert(topology_supports_directed_coupling(topo, 0, 1) == 1);
    assert(topology_supports_directed_coupling(topo, 1, 0) == 0);
    assert(topology_supports_coupling_either_direction(topo, 1, 0) == 1);
    assert(topology_supports_coupling_either_direction(topo, 0, 2) == 0);
    assert(topology_supports_directed_coupling(NULL, 0, 1) == -1);

    uint32_t undirected[4] = {0};
    assert(topology_undirected_edges_len(topo) == 2);
    assert(topology_undirected_edges(topo, undirected, 2) == 2);
    assert(undirected[0] == 0 && undirected[1] == 1 && undirected[2] == 1 && undirected[3] == 2);

    /* Qubit and coupling updates. */
    const uint32_t five[1] = {5};
    assert(topology_add_qubits(topo, five, 1) == 0);
    assert(topology_qubits_len(topo) == 4);
    assert(topology_add_qubits(topo, five, 1) == -8);
    const uint32_t nine[1] = {9};
    assert(topology_remove_qubits(topo, nine, 1) == -2);

    const uint32_t edge[2] = {1, 5};
    const char* names[1] = {"c15"};
    assert(topology_add_couplings(topo, edge, 1, names) == 0);
    assert(topology_num_couplings(topo) == 3);
    char* name = topology_get_coupling_name(topo, 1, 5);
    assert(name != NULL && strcmp(name, "c15") == 0);
    cqlib_string_free(name);
    char* unnamed = topology_get_coupling_name(topo, 0, 1);
    assert(unnamed != NULL && unnamed[0] == '\0');
    cqlib_string_free(unnamed);
    assert(topology_get_coupling_name(topo, 5, 1) == NULL);
    assert(topology_get_coupling_name(NULL, 0, 1) == NULL);

    assert(topology_supports_directed_coupling(topo, 1, 5) == 1);
    assert(topology_supports_coupling_either_direction(topo, 5, 1) == 1);
    assert(topology_out_degree(topo, 1) == 2);
    uint32_t succs2[2] = {0};
    assert(topology_successors_len(topo, 1) == 2);
    assert(topology_successors(topo, 1, succs2, 2) == 2);
    assert(succs2[0] == 2 && succs2[1] == 5);

    assert(topology_remove_couplings(topo, edge, 1) == 0);
    assert(topology_supports_directed_coupling(topo, 1, 5) == 0);
    assert(topology_remove_couplings(topo, edge, 1) == -2);
    assert(topology_add_couplings(topo, edge, 1, NULL) == 0);
    assert(topology_add_couplings(topo, edge, 1, NULL) == -8);
    const uint32_t bad_edge[2] = {0, 9};
    assert(topology_add_couplings(topo, bad_edge, 1, NULL) == -2);
    const uint32_t self_edge[2] = {0, 0};
    assert(topology_add_couplings(topo, self_edge, 1, NULL) == -8);
    assert(topology_add_qubits(NULL, five, 1) == -1);
    assert(topology_add_couplings(NULL, edge, 1, NULL) == -1);
    assert(topology_remove_couplings(NULL, edge, 1) == -1);
    assert(topology_add_qubits(topo, NULL, 1) == -1);

    /* Removing a qubit drops every coupling touching it. */
    assert(topology_remove_qubits(topo, five, 1) == 0);
    assert(topology_contains_qubit(topo, 5) == 0);
    assert(topology_num_couplings(topo) == 2);

    topology_free(topo);
    assert(topology_line(NULL, 3) == NULL);
}

/* =====  Section 3.4: Pauli channels and noise queries  ===== */

static void test_noise_queries(void) {
    CNoiseModel* model = noise_model_new();
    assert(model != NULL);

    /* Pauli channel add + query. */
    assert(noise_model_add_single_qubit_pauli(model, "H", 0, 0.01, 0.02, 0.03) == 0);
    assert(noise_model_add_single_qubit_pauli(model, "BOGUS", 0, 0.01, 0.0, 0.0) == -4);
    assert(noise_model_add_single_qubit_pauli(model, "H", 0, 0.5, 0.5, 0.5) == -8);
    assert(noise_model_add_single_qubit_pauli(NULL, "H", 0, 0.01, 0.0, 0.0) == -1);

    assert(noise_model_get_single_qubit_errors_len(model, "H", 0) == 1);
    CSingleQubitNoise channels[1] = {{0, 0.0, 0.0, 0.0, 0.0}};
    assert(noise_model_get_single_qubit_errors(model, "H", 0, channels, 1) == 1);
    assert(channels[0].tag == NOISE_PAULI);
    assert(channels[0].px == 0.01 && channels[0].py == 0.02 && channels[0].pz == 0.03);
    assert(noise_model_get_single_qubit_errors_len(model, "H", 1) == 0);
    assert(noise_model_get_single_qubit_errors_len(model, "BOGUS", 0) == 0);

    /* Two-qubit queries: key includes gate and the ordered pair. */
    assert(noise_model_add_two_qubit(model, "CX", 0, 1, NOISE_TWO_DEPOLARIZING, 0.01) == 0);
    assert(noise_model_get_two_qubit_errors_len(model, "CX", 0, 1) == 1);
    CTwoQubitNoise two_q[1];
    memset(two_q, 0, sizeof(two_q));
    assert(noise_model_get_two_qubit_errors(model, "CX", 0, 1, two_q, 1) == 1);
    assert(two_q[0].tag == NOISE_TWO_DEPOLARIZING && two_q[0].p == 0.01);
    assert(noise_model_get_two_qubit_errors_len(model, "CX", 1, 0) == 0);
    assert(noise_model_get_two_qubit_errors_len(model, "CX", 1, 1) == 0);
    assert(noise_model_get_two_qubit_errors_len(NULL, "CX", 0, 1) == 0);

    /* Readout queries. */
    assert(noise_model_add_readout(model, 0, 0.02, 0.03) == 0);
    double p01 = 0.0;
    double p10 = 0.0;
    assert(noise_model_get_readout_error(model, 0, &p01, &p10) == 0);
    assert(p01 == 0.02 && p10 == 0.03);
    assert(noise_model_get_readout_error(model, 5, &p01, &p10) == -8);
    assert(noise_model_get_readout_error(NULL, 0, &p01, &p10) == -1);

    noise_model_free(model);
}

/* =====  Section 3.5: execution result lifecycle and metadata  ===== */

static void test_execution_lifecycle(void) {
    const uint32_t qubits[2] = {0, 1};
    CExecutionResult* result = execution_result_new("task-2", qubits, 2, 100, NULL);
    assert(result != NULL);

    /* Freshly created: queued, no backend, only the creation timestamp. */
    uint8_t tag = 0;
    int32_t code = 0;
    assert(execution_result_status(result, &tag, &code) == 0);
    assert(tag == EXECUTION_STATUS_QUEUED && code == 0);
    int64_t ms = 0;
    assert(execution_result_created_at(result, &ms) == 0 && ms > 1600000000000LL);
    assert(execution_result_started_at(result, &ms) == -8);
    assert(execution_result_finished_at(result, &ms) == -8);
    assert(execution_result_shots(result) == 100);
    assert(execution_result_num_qubits(result) == 2);
    uint32_t got_q[2] = {0};
    assert(execution_result_qubits_len(result) == 2);
    assert(execution_result_qubits(result, got_q, 2) == 2);
    assert(got_q[0] == 0 && got_q[1] == 1);
    char* task = execution_result_task_id(result);
    assert(task != NULL && strcmp(task, "task-2") == 0);
    cqlib_string_free(task);
    assert(execution_result_backend(result) == NULL);
    assert(execution_result_error_message(result) == NULL);

    /* start -> running -> finish with accumulated counts. */
    assert(execution_result_start(result) == 0);
    assert(execution_result_status(result, &tag, &code) == 0 && tag == EXECUTION_STATUS_RUNNING);
    assert(execution_result_started_at(result, &ms) == 0);
    const char* bitstrings[3] = {"00", "11", "00"};
    const uint64_t counts[3] = {600, 400, 50};
    assert(execution_result_finish(result, bitstrings, counts, 3) == 0);
    assert(execution_result_status(result, &tag, &code) == 0 && tag == EXECUTION_STATUS_COMPLETED);
    assert(execution_result_finished_at(result, &ms) == 0);
    CCountsList* list = execution_result_counts(result);
    assert(list != NULL && counts_list_len(list) == 2);
    double total = 0.0;
    for (uintptr_t i = 0; i < counts_list_len(list); i++) {
        total += counts_list_get_value(list, i);
    }
    assert(total == 1050.0);
    counts_list_free(list);
    assert(execution_result_error_message(result) == NULL);

    /* Invalid bitstrings are rejected. */
    const char* bad_bits[1] = {"0x"};
    const uint64_t one[1] = {1};
    assert(execution_result_finish(result, bad_bits, one, 1) == -4);

    /* fail -> failed with code and message; cancel -> cancelled. */
    assert(execution_result_fail(result, "device offline", 42) == 0);
    assert(execution_result_status(result, &tag, &code) == 0);
    assert(tag == EXECUTION_STATUS_FAILED && code == 42);
    char* err = execution_result_error_message(result);
    assert(err != NULL && strcmp(err, "device offline") == 0);
    cqlib_string_free(err);
    assert(execution_result_cancel(result) == 0);
    assert(execution_result_status(result, &tag, &code) == 0 && tag == EXECUTION_STATUS_CANCELLED);
    execution_result_free(result);

    /* Backend metadata echoes back. */
    CExecutionResult* with_backend = execution_result_new("task-3", qubits, 2, 10, "sim-ctx");
    assert(with_backend != NULL);
    char* backend = execution_result_backend(with_backend);
    assert(backend != NULL && strcmp(backend, "sim-ctx") == 0);
    cqlib_string_free(backend);
    execution_result_free(with_backend);

    /* NULL guards. */
    assert(execution_result_new(NULL, qubits, 2, 10, NULL) == NULL);
    assert(execution_result_new("t", NULL, 2, 10, NULL) == NULL);
    assert(execution_result_start(NULL) == -1);
    assert(execution_result_finish(NULL, NULL, NULL, 0) == -1);
    assert(execution_result_cancel(NULL) == -1);
    assert(execution_result_status(NULL, &tag, &code) == -1);
    assert(execution_result_task_id(NULL) == NULL);
    assert(execution_result_backend(NULL) == NULL);
    assert(execution_result_created_at(NULL, &ms) == -1);
    assert(execution_result_started_at(NULL, &ms) == -1);
    assert(execution_result_finished_at(NULL, &ms) == -1);
    assert(execution_result_qubits_len(NULL) == 0);
}

int main(void) {
    test_device_topology_variants();
    test_device_edges_and_properties();
    test_device_native_gates_and_validation();
    test_layout();
    test_noise_model();
    test_execution_result();
    test_device_attributes();
    test_device_qubit_edge_properties();
    test_device_line_from_qubits_and_validate_operation();
    test_layout_mapping();
    test_topology_graph();
    test_noise_queries();
    test_execution_lifecycle();
    printf("binding-c device tests passed\n");
    return 0;
}
