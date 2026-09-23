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

int main(void) {
    test_device_topology_variants();
    test_device_edges_and_properties();
    test_device_native_gates_and_validation();
    test_layout();
    test_noise_model();
    test_execution_result();
    printf("binding-c device tests passed\n");
    return 0;
}
