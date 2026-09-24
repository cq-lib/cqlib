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

// Smoke tests for the newly added device-module FFI: per-qubit property
// setters/getters, native-instruction editors, layout-result helpers,
// execution-result status/probability helpers, and noise-model queries.

#include <assert.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "cqlib_c.h"

/* Builds a CQubitPropInput with only the readout error set. */
static CQubitPropInput bare_input(double readout) {
    CQubitPropInput input = {0};
    input.readout_error = readout;
    input.t1 = NAN;
    input.t2 = NAN;
    input.prob_meas0_prep1 = NAN;
    input.prob_meas1_prep0 = NAN;
    input.frequency = NAN;
    input.native_gate_names = NULL;
    input.native_error_rates = NULL;
    input.native_lengths = NULL;
    input.num_native_instructions = 0;
    return input;
}

static void test_device_property_setters_and_getters(void) {
    CDevice* device = device_line("prop-line", 3);
    assert(device != NULL);

    /* Record full properties for qubit 0: readout 0.01, t1 20, t2 30,
     * prob01 0.02, prob10 0.03, freq 5.0, native H 0.001/50, X 0.002/60. */
    const char* gates[2] = {"H", "X"};
    const double rates[2] = {0.001, 0.002};
    const double lengths[2] = {50.0, 60.0};
    CQubitPropInput input = bare_input(0.01);
    input.t1 = 20.0;
    input.t2 = 30.0;
    input.prob_meas0_prep1 = 0.02;
    input.prob_meas1_prep0 = 0.03;
    input.frequency = 5.0;
    input.native_gate_names = gates;
    input.native_error_rates = rates;
    input.native_lengths = lengths;
    input.num_native_instructions = 2;
    assert(device_add_qubit_properties(device, 0, &input) == 0);

    /* Getters read back the stored values. */
    double v = NAN;
    assert(device_get_frequency(device, 0, &v) == 0 && v == 5.0);
    assert(device_get_prob_meas0_prep1(device, 0, &v) == 0 && v == 0.02);
    assert(device_get_prob_meas1_prep0(device, 0, &v) == 0 && v == 0.03);
    assert(device_get_error_rate(device, 0, 1, &v) == 0 && v == 0.002);
    assert(device_get_length(device, 0, 0, &v) == 0 && v == 50.0);
    assert(device_get_t1(device, 0, &v) == 0 && v == 20.0);
    assert(device_get_t2(device, 0, &v) == 0 && v == 30.0);

    /* Getters fail for a qubit without recorded properties. */
    assert(device_get_frequency(device, 1, &v) == -8);
    assert(device_get_error_rate(device, 1, 0, &v) == -8);

    /* NULL handling. */
    assert(device_get_frequency(NULL, 0, &v) == -1);
    assert(device_set_frequency(NULL, 0, 1.0) == -1);

    /* Setters update one field and preserve the others; all reads happen
     * after the last setter to verify the rebuild keeps every field. */
    assert(device_set_t1(device, 0, 21.0) == 0);
    assert(device_set_t2(device, 0, 31.0) == 0);
    assert(device_set_frequency(device, 0, 5.5) == 0);
    assert(device_set_prob_meas0_prep1(device, 0, 0.021) == 0);
    assert(device_set_prob_meas1_prep0(device, 0, 0.031) == 0);
    assert(device_get_t1(device, 0, &v) == 0 && v == 21.0);
    assert(device_get_t2(device, 0, &v) == 0 && v == 31.0);
    assert(device_get_frequency(device, 0, &v) == 0 && v == 5.5);
    assert(device_get_prob_meas0_prep1(device, 0, &v) == 0 && v == 0.021);
    assert(device_get_prob_meas1_prep0(device, 0, &v) == 0 && v == 0.031);
    /* Native instructions survive the property rebuilds. */
    assert(device_get_error_rate(device, 0, 1, &v) == 0 && v == 0.002);

    /* Setters fail for a qubit without recorded properties. */
    assert(device_set_t1(device, 2, 1.0) == -8);
    assert(device_set_prob_meas0_prep1(device, 2, 0.1) == -8);

    device_free(device);
}

static void test_device_native_instruction_editors(void) {
    CDevice* device = device_line("gate-line", 2);
    assert(device != NULL);

    const char* gates[2] = {"H", "X"};
    const double rates[2] = {0.001, 0.002};
    const double lengths[2] = {50.0, 60.0};
    CQubitPropInput input = bare_input(0.01);
    input.native_gate_names = gates;
    input.native_error_rates = rates;
    input.native_lengths = lengths;
    input.num_native_instructions = 2;
    assert(device_add_qubit_properties(device, 0, &input) == 0);

    double v = NAN;
    /* Edit error rate and duration in place. */
    assert(device_set_error_rate(device, 0, 1, 0.009) == 0);
    assert(device_set_length(device, 0, 0, 70.0) == 0);
    assert(device_get_error_rate(device, 0, 1, &v) == 0 && v == 0.009);
    assert(device_get_length(device, 0, 0, &v) == 0 && v == 70.0);
    /* Untouched entries keep their values. */
    assert(device_get_error_rate(device, 0, 0, &v) == 0 && v == 0.001);
    assert(device_get_length(device, 0, 1, &v) == 0 && v == 60.0);

    /* Out-of-bounds index handling. */
    assert(device_set_error_rate(device, 0, 5, 0.1) == -8);
    assert(device_set_length(device, 0, 5, 1.0) == -8);
    assert(device_get_error_rate(device, 0, 5, &v) == -8);
    assert(device_set_error_rate(device, 1, 0, 0.1) == -8);

    /* Replace the carried gate of entry 0 (H -> X) and verify the snapshot. */
    assert(device_set_instruction(device, 0, 0, "X") == 0);
    CNativeInstruction inst = {0};
    assert(device_qubit_prop_native_instruction(device, 0, 0, &inst) == 0);
    assert(inst.name != NULL && strcmp(inst.name, "X") == 0);
    cqlib_string_free(inst.name);
    assert(inst.error_rate == 0.001 && inst.length == 70.0);

    /* Append a new native instruction. */
    assert(device_set_native_instruction(device, 0, "Z", 0.003, 80.0) == 0);
    CQubitProp snap = {0};
    assert(device_qubit_properties(device, 0, &snap) == 0);
    assert(snap.num_native_instructions == 3);
    CNativeInstruction third = {0};
    assert(device_qubit_prop_native_instruction(device, 0, 2, &third) == 0);
    assert(third.name != NULL && strcmp(third.name, "Z") == 0);
    cqlib_string_free(third.name);
    assert(third.error_rate == 0.003 && third.length == 80.0);

    /* A two-qubit gate is rejected as a qubit-native instruction (arity). */
    assert(device_set_native_instruction(device, 0, "CX", 0.01, 300.0) == -8);

    /* Unknown gate names. */
    assert(device_set_instruction(device, 0, 0, "nope") == -4);
    assert(device_set_native_instruction(device, 0, "nope", 0.01, 1.0) == -4);

    /* NULL handling. */
    assert(device_set_instruction(device, 0, 0, NULL) == -1);
    assert(device_set_native_instruction(device, 0, NULL, 0.01, 1.0) == -1);

    device_free(device);
}

static void test_layout_result_distances_and_activity(void) {
    CDevice* device = device_line("dist-line", 3);
    assert(device != NULL);

    CPhysicalLayoutGraph* graph = physical_layout_graph_from_device(device);
    assert(graph != NULL);

    /* Two-step pattern: length query, then a full copy of the 3x3 table. */
    assert(layout_result_distances(graph, NULL, 0) == 9);
    const uint32_t expected[9] = {0, 1, 2, 1, 0, 1, 2, 1, 0};
    uint32_t buf[9] = {0};
    assert(layout_result_distances(graph, buf, 9) == 9);
    assert(memcmp(buf, expected, sizeof(expected)) == 0);

    /* A partial copy only touches the requested prefix. */
    uint32_t part[9];
    for (uintptr_t i = 0; i < 9; i++) {
        part[i] = 7;
    }
    assert(layout_result_distances(graph, part, 3) == 9);
    assert(part[0] == 0 && part[1] == 1 && part[2] == 2);
    for (uintptr_t i = 3; i < 9; i++) {
        assert(part[i] == 7);
    }
    assert(layout_result_distances(NULL, NULL, 0) == 0);
    physical_layout_graph_free(graph);

    /* Activity from interactions cx(0,1) x2 and cx(1,2) x1. */
    CCircuit* circuit = circuit_new(3);
    assert(circuit != NULL);
    assert(circuit_cx(circuit, 0, 1) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);
    assert(circuit_cx(circuit, 1, 2) == 0);

    CCircuitLayoutAnalysis* analysis = analyze_circuit_for_layout(circuit);
    assert(analysis != NULL);
    assert(layout_result_logical_activity(analysis, NULL, NULL, 0) == 3);
    uint32_t qubits[3] = {0};
    double act[3] = {0.0};
    assert(layout_result_logical_activity(analysis, qubits, act, 3) == 3);
    assert(qubits[0] == 0 && qubits[1] == 1 && qubits[2] == 2);
    const double expected_act[3] = {2.0, 3.0, 1.0};
    for (uintptr_t i = 0; i < 3; i++) {
        assert(fabs(act[i] - expected_act[i]) < 1e-12);
    }
    circuit_layout_analysis_free(analysis);

    /* layout_result_analysis reuses the CCircuitLayoutAnalysis handle type. */
    CPreparedSabreCircuit* prepared = prepare_sabre_circuit(circuit);
    assert(prepared != NULL);
    CCircuitLayoutAnalysis* reused = layout_result_analysis(prepared);
    assert(reused != NULL);
    assert(circuit_layout_analysis_num_logical(reused) == 3);
    circuit_layout_analysis_free(reused);
    prepared_sabre_circuit_free(prepared);
    assert(layout_result_analysis(NULL) == NULL);

    circuit_free(circuit);
    device_free(device);
}

static void test_execution_result_status_flags_and_bitstring(void) {
    const uint32_t qubits[3] = {0, 1, 2};
    CExecutionResult* result = execution_result_new("task-1", qubits, 3, 100, NULL);
    assert(result != NULL);

    /* Queued: neither success nor terminal. */
    assert(execution_result_is_success(result) == 0);
    assert(execution_result_is_terminal(result) == 0);

    /* Running is still not terminal. */
    assert(execution_result_start(result) == 0);
    assert(execution_result_is_terminal(result) == 0);

    /* Complete with counts. */
    const char* keys[3] = {"000", "111", "101"};
    const uint64_t counts[3] = {50, 30, 20};
    assert(execution_result_finish(result, keys, counts, 3) == 0);
    assert(execution_result_is_success(result) == 1);
    assert(execution_result_is_terminal(result) == 1);

    /* Bitstring formatting: bit i of the mask is measured qubit i. */
    const uint64_t masks[4] = {1, 5, 0, UINT64_MAX};
    const char* wants[4] = {"001", "101", "000", "111"};
    for (uintptr_t i = 0; i < 4; i++) {
        char* text = execution_result_to_bitstring(result, masks[i]);
        assert(text != NULL && strcmp(text, wants[i]) == 0);
        cqlib_string_free(text);
    }
    assert(execution_result_to_bitstring(NULL, 0) == NULL);

    /* Failure and cancellation are terminal but not successful. */
    assert(execution_result_fail(result, "boom", 7) == 0);
    assert(execution_result_is_success(result) == 0);
    assert(execution_result_is_terminal(result) == 1);
    assert(execution_result_cancel(result) == 0);
    assert(execution_result_is_terminal(result) == 1);

    assert(execution_result_is_success(NULL) == -1);
    assert(execution_result_is_terminal(NULL) == -1);

    execution_result_free(result);
}

static void test_execution_result_calc_probabilities(void) {
    const uint32_t qubits[2] = {0, 1};
    CExecutionResult* result = execution_result_new("task-2", qubits, 2, 50, NULL);
    assert(result != NULL);
    const char* keys[2] = {"00", "11"};
    const uint64_t counts[2] = {30, 20};
    assert(execution_result_finish(result, keys, counts, 2) == 0);

    /* Two-step pattern with ascending outcome order ("00" then "11"). */
    assert(execution_result_calc_probabilities_len(result) == 2);
    double probs[2] = {0.0};
    assert(execution_result_calc_probabilities(result, probs, 2) == 2);
    assert(fabs(probs[0] - 0.6) < 1e-12);
    assert(fabs(probs[1] - 0.4) < 1e-12);

    /* A partial copy only touches the requested prefix. */
    double part[2] = {NAN, NAN};
    assert(execution_result_calc_probabilities(result, part, 1) == 2);
    assert(fabs(part[0] - 0.6) < 1e-12);
    assert(isnan(part[1]));

    assert(execution_result_calc_probabilities_len(NULL) == 0);
    assert(execution_result_calc_probabilities(NULL, NULL, 0) == 0);

    execution_result_free(result);
}

static void test_noise_model_validity_and_kraus(void) {
    CNoiseModel* model = noise_model_new();
    assert(model != NULL);

    assert(noise_model_add_single_qubit(model, "H", 0, NOISE_DEPOLARIZING, 0.1) == 0);
    assert(noise_model_is_valid(model, "H", 0, 0) == 1);

    /* Kraus output for depolarizing p=0.1: I*sqrt(0.9), X/Y/Z*sqrt(p/3),
     * four 2x2 operators written back to back in row-major order. */
    assert(noise_model_to_kraus_len(model, "H", 0, 0) == 16);
    Complex64 buf[16];
    memset(buf, 0, sizeof(buf));
    assert(noise_model_to_kraus(model, "H", 0, 0, buf, 16) == 16);
    assert(fabs(buf[0].re - sqrt(0.9)) < 1e-12);
    assert(fabs(buf[0].im) < 1e-12);
    assert(fabs(buf[1].re) < 1e-12);
    assert(fabs(buf[3].re - sqrt(0.9)) < 1e-12);
    const double p_other = sqrt(0.1 / 3.0);
    assert(fabs(buf[4].re) < 1e-12);
    assert(fabs(buf[5].re - p_other) < 1e-12);
    assert(fabs(buf[6].re - p_other) < 1e-12);
    assert(fabs(buf[7].re) < 1e-12);

    /* A second channel under a different gate key (X@0, its own index 0). */
    assert(noise_model_add_single_qubit(model, "X", 0, NOISE_AMPLITUDE_DAMPING, 0.25) == 0);
    assert(noise_model_is_valid(model, "X", 0, 0) == 1);
    assert(noise_model_to_kraus_len(model, "X", 0, 0) == 8);
    Complex64 buf2[8];
    memset(buf2, 0, sizeof(buf2));
    assert(noise_model_to_kraus(model, "X", 0, 0, buf2, 8) == 8);
    assert(buf2[0].re == 1.0);
    assert(fabs(buf2[3].re - sqrt(0.75)) < 1e-12);
    assert(fabs(buf2[4].re) < 1e-12 && fabs(buf2[4].im) < 1e-12);
    assert(fabs(buf2[5].re - 0.5) < 1e-12);
    assert(fabs(buf2[7].re) < 1e-12);

    /* A partial copy only touches the requested prefix. */
    Complex64 part[8];
    for (uintptr_t i = 0; i < 8; i++) {
        part[i].re = NAN;
        part[i].im = NAN;
    }
    assert(noise_model_to_kraus(model, "X", 0, 0, part, 2) == 8);
    assert(fabs(part[0].re - 1.0) < 1e-12);
    assert(isnan(part[3].re));

    /* Errors: absent key, out-of-bounds index, unknown gate, NULL. */
    assert(noise_model_is_valid(model, "H", 1, 0) == -8);
    assert(noise_model_to_kraus_len(model, "H", 1, 0) == 0);
    assert(noise_model_is_valid(model, "H", 0, 5) == -8);
    assert(noise_model_is_valid(model, "nope", 0, 0) == -4);
    assert(noise_model_to_kraus_len(model, "nope", 0, 0) == 0);
    assert(noise_model_is_valid(NULL, "H", 0, 0) == -1);
    assert(noise_model_to_kraus_len(NULL, "H", 0, 0) == 0);

    noise_model_free(model);
}

int main(void) {
    test_device_property_setters_and_getters();
    test_device_native_instruction_editors();
    test_layout_result_distances_and_activity();
    test_execution_result_status_flags_and_bitstring();
    test_execution_result_calc_probabilities();
    test_noise_model_validity_and_kraus();
    printf("binding-c device missing tests passed\n");
    return 0;
}
