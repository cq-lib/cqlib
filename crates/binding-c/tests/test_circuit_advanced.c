// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cqlib_c.h"

/* Large enough for a 3-qubit unitary (64 complex = 128 doubles). */
#define MAX_MATRIX_DOUBLES 512

static uintptr_t matrix_of(const CCircuit* circuit, double* buf) {
    uintptr_t len = circuit_to_matrix_len(circuit, NULL, 0);
    assert(len > 0);
    assert(len * 2 <= MAX_MATRIX_DOUBLES);
    assert(circuit_to_matrix(circuit, NULL, 0, buf, len * 2) == len);
    return len;
}

static void assert_matrices_equal(const double* a, const double* b, uintptr_t len) {
    for (uintptr_t i = 0; i < len; i++) {
        assert(fabs(a[i] - b[i]) < 1e-12);
    }
}

static void test_parameter_metadata(void) {
    CParameter* theta = param_parse("theta");
    assert(theta != NULL);

    CCircuit* circuit = circuit_new(1);
    assert(circuit != NULL);

    /* Interning registers the parameter and its symbols but does not mark it used. */
    assert(circuit_add_parameter(circuit, theta) == 0);
    assert(circuit_parameters_len(circuit) == 1);
    assert(circuit_num_parameters(circuit) == 1);
    assert(circuit_symbols_len(circuit) == 1);
    assert(circuit_used_symbols_len(circuit) == 0);
    assert(circuit_uses_symbol(circuit, "theta") == 0);

    /* Referencing it through a gate marks the symbol as used. */
    assert(circuit_rx_param(circuit, 0, theta) == 0);
    assert(circuit_uses_symbol(circuit, "theta") == 1);
    assert(circuit_used_symbols_len(circuit) == 1);

    /* Two-step symbol lists; each string is freed by the caller. */
    char* names[1] = {NULL};
    assert(circuit_symbols(circuit, names, 1) == 1);
    assert(strcmp(names[0], "theta") == 0);
    cqlib_string_free(names[0]);
    assert(circuit_used_symbols(circuit, names, 1) == 1);
    assert(strcmp(names[0], "theta") == 0);
    cqlib_string_free(names[0]);

    /* Two-step parameter list: cloned handles the caller frees. */
    CParameter* params[1] = {NULL};
    assert(circuit_parameters(circuit, params, 1) == 1);
    assert(fabs(param_evaluate(params[0], "theta:2") - 2.0) < 1e-12);
    param_free(params[0]);

    circuit_free(circuit);
    param_free(theta);
}

static void test_map_resolve_parameter_value(void) {
    CCircuit* circuit = circuit_new(1);
    CParameter* theta = param_parse("theta");
    CParameter* phi = param_parse("phi * 2");
    assert(circuit_rx_param(circuit, 0, theta) == 0);

    /* Mapping a symbolic expression yields a parameter-table index. */
    CCircuitParam mapped = {255, 0, 0.0};
    assert(circuit_map_param(circuit, phi, &mapped) == 0);
    assert(mapped.tag == CIRCUIT_PARAM_TAG_INDEX);
    assert(mapped.index == 1);

    /* Mapping a constant yields a fixed value. */
    CParameter* half = param_parse("0.5");
    CCircuitParam fixed = {255, 0, 0.0};
    assert(circuit_map_param(circuit, half, &fixed) == 0);
    assert(fixed.tag == CIRCUIT_PARAM_TAG_FIXED);
    assert(fabs(fixed.value - 0.5) < 1e-12);

    /* Resolve an index back into its expression and evaluate it. */
    CParameter* resolved = circuit_resolve_parameter(circuit, &mapped);
    assert(resolved != NULL);
    assert(fabs(param_evaluate(resolved, "theta:0.25,phi:1.5") - 3.0) < 1e-12);
    param_free(resolved);

    /* Parameter values: index -> symbolic handle, fixed -> number. */
    CParameterValue value = {255, 0.0, NULL};
    assert(circuit_parameter_value(circuit, &mapped, &value) == 0);
    assert(value.tag == PARAMETER_VALUE_TAG_PARAM);
    assert(value.param != NULL);
    assert(fabs(param_evaluate(value.param, "theta:0.25,phi:1.5") - 3.0) < 1e-12);
    param_free(value.param);

    assert(circuit_parameter_value(circuit, &fixed, &value) == 0);
    assert(value.tag == PARAMETER_VALUE_TAG_FIXED);
    assert(fabs(value.value - 0.5) < 1e-12);

    /* Invalid index -> -3, invalid tag -> -8. */
    CCircuitParam bad_index = {CIRCUIT_PARAM_TAG_INDEX, 99, 0.0};
    assert(circuit_parameter_value(circuit, &bad_index, &value) == -3);
    assert(circuit_resolve_parameter(circuit, &bad_index) == NULL);
    CCircuitParam bad_tag = {7, 0, 1.0};
    assert(circuit_parameter_value(circuit, &bad_tag, &value) == -8);

    circuit_free(circuit);
    param_free(half);
    param_free(phi);
    param_free(theta);
}

static void test_global_phase(void) {
    CCircuit* circuit = circuit_new(1);
    CParameterValue value = {255, 0.0, NULL};

    assert(circuit_set_global_phase(circuit, 0.25) == 0);
    assert(circuit_global_phase(circuit, &value) == 0);
    assert(value.tag == PARAMETER_VALUE_TAG_FIXED);
    assert(fabs(value.value - 0.25) < 1e-12);

    /* A symbolic phase is reported as a parameter handle. */
    CParameter* alpha = param_parse("alpha");
    assert(circuit_set_global_phase_param(circuit, alpha) == 0);
    assert(circuit_global_phase(circuit, &value) == 0);
    assert(value.tag == PARAMETER_VALUE_TAG_PARAM);
    assert(fabs(param_evaluate(value.param, "alpha:1.5") - 1.5) < 1e-12);
    param_free(value.param);

    assert(circuit_global_phase(NULL, &value) == -1);
    circuit_free(circuit);
    param_free(alpha);
}

static void test_id_and_from_qubits(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit_id(circuit, 0) == 0);
    assert(circuit_id(circuit, 1) == 0);
    assert(circuit_num_operations(circuit) == 2);
    assert(circuit_id(circuit, 5) == -2);
    assert(circuit_id(NULL, 0) == -1);
    assert(circuit_validate(circuit) == 0);
    circuit_free(circuit);

    /* Explicit sparse qubit ids. */
    const uint32_t ids[3] = {0, 2, 5};
    CCircuit* sparse = circuit_from_qubits(ids, 3);
    assert(sparse != NULL);
    assert(circuit_num_qubits(sparse) == 3);
    uint32_t out[3];
    assert(circuit_qubits(sparse, out, 3) == 3);
    assert(out[0] == 0 && out[1] == 2 && out[2] == 5);
    circuit_free(sparse);

    /* Duplicate ids are rejected; an empty list is allowed. */
    const uint32_t dup[2] = {1, 1};
    assert(circuit_from_qubits(dup, 2) == NULL);
    assert(circuit_from_qubits(NULL, 1) == NULL);
    CCircuit* empty = circuit_from_qubits(NULL, 0);
    assert(empty != NULL);
    circuit_free(empty);
}

static void test_unitary(void) {
    /* Pauli-X as a custom unitary matches the standard X gate matrix. */
    const Complex64 x_matrix[4] = {
        {0.0, 0.0},
        {1.0, 0.0},
        {1.0, 0.0},
        {0.0, 0.0},
    };
    const uint32_t qs[1] = {0};

    CCircuit* circuit = circuit_new(2);
    assert(circuit_unitary(circuit, "my_x", 1, x_matrix, qs, 1) == 0);

    CCircuit* reference = circuit_new(2);
    assert(circuit_x(reference, 0) == 0);

    double a[MAX_MATRIX_DOUBLES], b[MAX_MATRIX_DOUBLES];
    uintptr_t len_a = matrix_of(circuit, a);
    uintptr_t len_b = matrix_of(reference, b);
    assert(len_a == len_b);
    assert_matrices_equal(a, b, 2 * len_a);

    /* Out-of-bounds qubit -> -2; too many qubits -> -8; NULL circuit -> -1. */
    const uint32_t oob[1] = {5};
    assert(circuit_unitary(circuit, "my_x", 1, x_matrix, oob, 1) == -2);
    assert(circuit_unitary(circuit, "my_x", 21, x_matrix, qs, 1) == -8);
    assert(circuit_unitary(NULL, "my_x", 1, x_matrix, qs, 1) == -1);

    circuit_free(reference);
    circuit_free(circuit);
}

static void test_unitary_with_params(void) {
    /* diag(1, e^{i*theta}) as a symbolic matrix; bind theta = 0.5. */
    const char* re[4] = {"1", "0", "0", "cos(theta)"};
    const char* im[4] = {"0", "0", "0", "sin(theta)"};
    const char* names[1] = {"theta"};
    const uint32_t qs[1] = {0};

    CParameter* arg = param_parse("0.5");
    const CParameter* args[1] = {arg};

    CCircuit* circuit = circuit_new(1);
    assert(circuit_unitary_with_params(circuit, "phase_like", 1, re, im, names, 1, qs, 1, args,
                                       1) == 0);

    /* The constant argument is interned as an index; fold it before reading
     * the numeric matrix. */
    CCircuit* resolved = circuit_assign_params(circuit, NULL);
    assert(resolved != NULL);
    double m[MAX_MATRIX_DOUBLES];
    assert(matrix_of(resolved, m) == 4);
    assert(fabs(m[0] - 1.0) < 1e-12);                        /* [0][0].re */
    assert(fabs(m[1]) < 1e-12);                              /* [0][0].im */
    assert(fabs(m[(1 * 2 + 1) * 2] - cos(0.5)) < 1e-12);     /* [1][1].re */
    assert(fabs(m[(1 * 2 + 1) * 2 + 1] - sin(0.5)) < 1e-12); /* [1][1].im */

    /* A NULL imaginary-part array is treated as all zeros: with theta = 0
     * the gate becomes the identity. */
    CParameter* zero = param_parse("0");
    const CParameter* zero_args[1] = {zero};
    CCircuit* plain = circuit_new(1);
    assert(circuit_unitary_with_params(plain, "phase_like", 1, re, NULL, names, 1, qs, 1, zero_args,
                                       1) == 0);
    CCircuit* resolved_plain = circuit_assign_params(plain, NULL);
    assert(resolved_plain != NULL);
    double p[MAX_MATRIX_DOUBLES];
    assert(matrix_of(resolved_plain, p) == 4);
    assert(fabs(p[(1 * 2 + 1) * 2] - 1.0) < 1e-12); /* cos(0) = 1 */
    assert(fabs(p[(1 * 2 + 1) * 2 + 1]) < 1e-12);

    /* Wrong parameter count -> -3. */
    assert(circuit_unitary_with_params(circuit, "phase_like", 1, re, im, names, 1, qs, 1, NULL,
                                       0) == -3);

    circuit_free(resolved_plain);
    circuit_free(plain);
    circuit_free(resolved);
    circuit_free(circuit);
    param_free(zero);
    param_free(arg);
}

static void test_to_gate_circuit_gate(void) {
    /* Bell circuit frozen into a composite gate and re-applied. */
    CCircuit* inner = circuit_new(2);
    assert(circuit_h(inner, 0) == 0);
    assert(circuit_cx(inner, 0, 1) == 0);

    CCircuitGate* gate = circuit_to_gate(inner, "bell");
    assert(gate != NULL);
    char* name = circuit_gate_name(gate);
    assert(name != NULL && strcmp(name, "bell") == 0);
    cqlib_string_free(name);
    assert(circuit_gate_num_qubits(gate) == 2);
    assert(circuit_gate_num_params(gate) == 0);
    assert(circuit_gate_signature_params_len(gate) == 0);

    const uint32_t qs[2] = {0, 1};
    CCircuit* outer = circuit_new(2);
    assert(circuit_circuit_gate(outer, gate, qs, 2, NULL, 0) == 0);

    /* Applying the composite gate reproduces the original circuit. */
    double a[MAX_MATRIX_DOUBLES], b[MAX_MATRIX_DOUBLES];
    uintptr_t len_a = matrix_of(outer, a);
    uintptr_t len_b = matrix_of(inner, b);
    assert(len_a == len_b);
    assert_matrices_equal(a, b, 2 * len_a);

    /* Qubit arity mismatch -> -3; out-of-bounds qubit -> -2. */
    const uint32_t single[1] = {0};
    assert(circuit_circuit_gate(outer, gate, single, 1, NULL, 0) == -3);
    const uint32_t oob[2] = {0, 9};
    assert(circuit_circuit_gate(outer, gate, oob, 2, NULL, 0) == -2);

    circuit_free(outer);
    circuit_gate_free(gate);
    circuit_free(inner);
}

static void test_to_gate_parametrized(void) {
    /* RX(theta) template applied with theta = 0.5. */
    CParameter* theta = param_parse("theta");
    CCircuit* template = circuit_new(1);
    assert(circuit_rx_param(template, 0, theta) == 0);

    CCircuitGate* gate = circuit_to_gate(template, "rx_theta");
    assert(gate != NULL);
    assert(circuit_gate_num_params(gate) == 1);
    assert(circuit_gate_signature_params_len(gate) == 1);
    char* names[1] = {NULL};
    assert(circuit_gate_signature_params(gate, names, 1) == 1);
    assert(strcmp(names[0], "theta") == 0);
    cqlib_string_free(names[0]);

    CParameter* arg = param_parse("0.5");
    const CParameter* args[1] = {arg};
    const uint32_t qs[1] = {0};
    CCircuit* outer = circuit_new(1);
    assert(circuit_circuit_gate(outer, gate, qs, 1, args, 1) == 0);

    /* Fold the constant argument, then compare against RX(0.5). */
    CCircuit* resolved = circuit_assign_params(outer, NULL);
    assert(resolved != NULL);
    CCircuit* reference = circuit_new(1);
    assert(circuit_rx(reference, 0, 0.5) == 0);

    double a[MAX_MATRIX_DOUBLES], b[MAX_MATRIX_DOUBLES];
    uintptr_t len_a = matrix_of(resolved, a);
    uintptr_t len_b = matrix_of(reference, b);
    assert(len_a == len_b);
    assert_matrices_equal(a, b, 2 * len_a);

    /* Wrong parameter count -> -3. */
    assert(circuit_circuit_gate(outer, gate, qs, 1, NULL, 0) == -3);

    circuit_free(reference);
    circuit_free(resolved);
    circuit_free(outer);
    param_free(arg);
    circuit_gate_free(gate);
    circuit_free(template);
    param_free(theta);
}

static void test_multi_control(void) {
    CCircuit* circuit = circuit_new(3);
    const uint32_t controls[2] = {0, 1};
    const uint32_t targets[1] = {2};

    /* Double-controlled X is CCX. */
    assert(circuit_multi_control(circuit, "X", controls, 2, targets, 1, NULL, 0) == 0);

    CCircuit* reference = circuit_new(3);
    assert(circuit_ccx(reference, 0, 1, 2) == 0);

    double a[MAX_MATRIX_DOUBLES], b[MAX_MATRIX_DOUBLES];
    uintptr_t len_a = matrix_of(circuit, a);
    uintptr_t len_b = matrix_of(reference, b);
    assert(len_a == len_b);
    assert_matrices_equal(a, b, 2 * len_a);

    /* Unknown gate name -> -8; missing target -> -3; OOB control -> -2. */
    assert(circuit_multi_control(circuit, "NOT_A_GATE", controls, 2, targets, 1, NULL, 0) == -8);
    assert(circuit_multi_control(circuit, "H", controls, 2, NULL, 0, NULL, 0) == -3);
    const uint32_t oob[2] = {0, 9};
    assert(circuit_multi_control(circuit, "X", oob, 2, targets, 1, NULL, 0) == -2);

    circuit_free(reference);
    circuit_free(circuit);
}

static void test_delay_index(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit_delay(circuit, 1, 100.0) == 0);
    assert(circuit_delay(circuit, 5, 1.0) == -2); /* OOB qubit */
    assert(circuit_delay(circuit, 0, NAN) == -3); /* non-finite duration */
    assert(circuit_delay(NULL, 0, 1.0) == -1);

    CParameter* dt = param_parse("dt");
    assert(circuit_delay_param(circuit, 0, dt) == 0);
    assert(circuit_delay_param(circuit, 0, NULL) == -1);
    assert(circuit_num_operations(circuit) == 2);
    assert(circuit_num_parameters(circuit) == 1);

    /* Snapshot of the numeric delay. */
    CValueOperation* op = circuit_index(circuit, 0);
    assert(op != NULL);
    char* name = value_operation_name(op);
    assert(name != NULL && strcmp(name, "delay") == 0);
    cqlib_string_free(name);
    char* type = value_operation_instruction_type(op);
    assert(type != NULL && strcmp(type, "delay") == 0);
    cqlib_string_free(type);
    assert(value_operation_label(op) == NULL);
    assert(value_operation_num_qubits(op) == 1);
    uint32_t qubits[1];
    assert(value_operation_qubits(op, qubits, 1) == 1);
    assert(qubits[0] == 1);
    assert(value_operation_num_params(op) == 1);

    CParameterValue value = {255, 0.0, NULL};
    assert(value_operation_param(op, 0, &value) == 0);
    assert(value.tag == PARAMETER_VALUE_TAG_FIXED);
    assert(fabs(value.value - 100.0) < 1e-12);
    assert(value_operation_param(op, 1, &value) == -3);
    value_operation_free(op);

    /* Snapshot of the symbolic delay. */
    CValueOperation* symbolic = circuit_index(circuit, 1);
    assert(symbolic != NULL);
    assert(value_operation_param(symbolic, 0, &value) == 0);
    assert(value.tag == PARAMETER_VALUE_TAG_PARAM);
    assert(fabs(param_evaluate(value.param, "dt:7") - 7.0) < 1e-12);
    param_free(value.param);
    value_operation_free(symbolic);

    assert(circuit_index(circuit, 99) == NULL);
    assert(circuit_index(NULL, 0) == NULL);

    circuit_free(circuit);
    param_free(dt);
}

static void test_remove_operations(void) {
    CCircuit* circuit = circuit_new(1);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_x(circuit, 0) == 0);
    assert(circuit_phase(circuit, 0, 0.5) == 0);

    /* Indices refer to pre-deletion positions: drop H and phase, keep X. */
    const uintptr_t indices[2] = {0, 2};
    assert(circuit_remove_operations(circuit, indices, 2) == 0);
    assert(circuit_num_operations(circuit) == 1);

    CValueOperation* op = circuit_index(circuit, 0);
    assert(op != NULL);
    char* name = value_operation_name(op);
    assert(name != NULL && strcmp(name, "X") == 0);
    cqlib_string_free(name);
    value_operation_free(op);

    /* Empty index list is a successful no-op. */
    assert(circuit_remove_operations(circuit, NULL, 0) == 0);
    assert(circuit_num_operations(circuit) == 1);

    /* Out-of-bounds index -> -3; NULL circuit -> -1. */
    const uintptr_t bad[2] = {0, 9};
    assert(circuit_remove_operations(circuit, bad, 2) == -3);
    assert(circuit_remove_operations(NULL, NULL, 0) == -1);

    circuit_free(circuit);
}

static void test_null_checks(void) {
    assert(circuit_add_parameter(NULL, NULL) == -1);
    assert(circuit_parameters_len(NULL) == 0);
    assert(circuit_parameters(NULL, NULL, 0) == 0);
    assert(circuit_symbols_len(NULL) == 0);
    assert(circuit_symbols(NULL, NULL, 0) == 0);
    assert(circuit_used_symbols_len(NULL) == 0);
    assert(circuit_used_symbols(NULL, NULL, 0) == 0);
    assert(circuit_uses_symbol(NULL, NULL) == -1);
    assert(circuit_global_phase(NULL, NULL) == -1);
    assert(circuit_map_param(NULL, NULL, NULL) == -1);
    assert(circuit_resolve_parameter(NULL, NULL) == NULL);
    assert(circuit_parameter_value(NULL, NULL, NULL) == -1);
    assert(circuit_unitary(NULL, NULL, 0, NULL, NULL, 0) == -1);
    assert(circuit_unitary_with_params(NULL, NULL, 0, NULL, NULL, NULL, 0, NULL, 0, NULL, 0) == -1);
    assert(circuit_to_gate(NULL, NULL) == NULL);
    assert(circuit_circuit_gate(NULL, NULL, NULL, 0, NULL, 0) == -1);
    assert(circuit_multi_control(NULL, NULL, NULL, 0, NULL, 0, NULL, 0) == -1);
    assert(circuit_delay(NULL, 0, 1.0) == -1);
    assert(circuit_delay_param(NULL, 0, NULL) == -1);
    assert(circuit_index(NULL, 0) == NULL);
    assert(circuit_remove_operations(NULL, NULL, 0) == -1);

    /* Handle accessors tolerate NULL. */
    assert(circuit_gate_name(NULL) == NULL);
    assert(circuit_gate_num_qubits(NULL) == 0);
    assert(circuit_gate_num_params(NULL) == 0);
    assert(circuit_gate_signature_params_len(NULL) == 0);
    assert(circuit_gate_signature_params(NULL, NULL, 0) == 0);
    assert(value_operation_name(NULL) == NULL);
    assert(value_operation_instruction_type(NULL) == NULL);
    assert(value_operation_label(NULL) == NULL);
    assert(value_operation_num_qubits(NULL) == 0);
    assert(value_operation_qubits(NULL, NULL, 0) == 0);
    assert(value_operation_num_params(NULL) == 0);
    assert(value_operation_param(NULL, 0, NULL) == -1);
    circuit_gate_free(NULL);
    value_operation_free(NULL);
}

int main(void) {
    test_parameter_metadata();
    test_map_resolve_parameter_value();
    test_global_phase();
    test_id_and_from_qubits();
    test_unitary();
    test_unitary_with_params();
    test_to_gate_circuit_gate();
    test_to_gate_parametrized();
    test_multi_control();
    test_delay_index();
    test_remove_operations();
    test_null_checks();
    printf("binding-c circuit advanced tests passed\n");
    return 0;
}
