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

/* Compares two doubles with a 1e-9 tolerance. */
static int close_enough(double a, double b) { return fabs(a - b) < 1e-9; }

static void test_fixed_param_operation(void) {
    const uint32_t qubits[1] = {0};
    const double params[1] = {0.5};
    struct COperation* op = operation_new("RX", qubits, 1, params, 1);
    CHECK(op != NULL, "operation_new(RX)");

    char* name = operation_name(op);
    CHECK(name != NULL && strcmp(name, "RX") == 0, "operation_name");
    cqlib_string_free(name);

    CHECK(operation_is_standard_gate(op) == 1, "operation_is_standard_gate");
    CHECK(operation_num_qubits(op) == 1, "operation_num_qubits");
    CHECK(operation_qubits_len(op) == 1, "operation_qubits_len");

    uint32_t ids[1] = {9};
    CHECK(operation_qubits(op, ids, 1) == 0, "operation_qubits");
    CHECK(ids[0] == 0, "operation_qubits value");
    CHECK(operation_qubits(op, NULL, 1) == -1, "operation_qubits null buffer");
    CHECK(operation_qubits(op, ids, 0) == -8, "operation_qubits short buffer");

    uint32_t tags[1] = {255};
    double values[1] = {0.0};
    CHECK(operation_params_len(op) == 1, "operation_params_len");
    CHECK(operation_params(op, tags, values, 1) == 0, "operation_params");
    CHECK(tags[0] == OPERATION_PARAM_FIXED, "operation_params tag FIXED");
    CHECK(close_enough(values[0], 0.5), "operation_params value");
    CHECK(operation_params(op, tags, values, 0) == -8, "operation_params short buffer");

    CHECK(operation_matrix_len(op) == 4, "operation_matrix_len");
    uintptr_t rows = 0;
    uintptr_t cols = 0;
    CHECK(operation_matrix_dims(op, &rows, &cols) == 0, "operation_matrix_dims");
    CHECK(rows == 2 && cols == 2, "operation_matrix_dims shape");
    CHECK(operation_matrix_dims(op, NULL, &cols) == -1, "operation_matrix_dims null rows");
    CHECK(operation_matrix_dims(op, &rows, NULL) == -1, "operation_matrix_dims null cols");

    double out[8] = {0};
    CHECK(operation_matrix(op, out, 8) == 4, "operation_matrix");
    /* RX(0.5) = [[cos(0.25), -i*sin(0.25)],
                   [-i*sin(0.25), cos(0.25)]] row-major interleaved. */
    const double c = cos(0.25);
    const double s = sin(0.25);
    CHECK(close_enough(out[0], c) && close_enough(out[1], 0.0) && close_enough(out[2], 0.0) &&
              close_enough(out[3], -s) && close_enough(out[4], 0.0) && close_enough(out[5], -s) &&
              close_enough(out[6], c) && close_enough(out[7], 0.0),
          "operation_matrix RX(0.5) values");
    CHECK(operation_matrix(op, out, 7) == 0, "operation_matrix short buffer");
    CHECK(operation_matrix(op, NULL, 8) == 0, "operation_matrix null buffer");

    /* Operations built by `operation_new` never carry a label. */
    CHECK(operation_label(op) == NULL, "operation_label");

    operation_free(op);
}

static void test_parameterless_gate(void) {
    const uint32_t pair[2] = {0, 1};
    struct COperation* op = operation_new("CX", pair, 2, NULL, 0);
    CHECK(op != NULL, "operation_new(CX)");

    uint32_t ids[2] = {9, 9};
    CHECK(operation_qubits_len(op) == 2, "CX qubits_len");
    CHECK(operation_qubits(op, ids, 2) == 0, "CX qubits");
    CHECK(ids[0] == 0 && ids[1] == 1, "CX qubit ids");

    /* Zero parameters: `operation_params` succeeds with len = 0. */
    CHECK(operation_params_len(op) == 0, "CX params_len");
    CHECK(operation_params(op, NULL, NULL, 0) == 0, "CX params empty");

    /* The 4x4 matrix is returned as 16 interleaved doubles. */
    CHECK(operation_matrix_len(op) == 16, "CX matrix_len");
    double out[32] = {0};
    CHECK(operation_matrix(op, out, 32) == 16, "CX matrix");

    operation_free(op);
}

static void test_index_param_operation(void) {
    /* `circuit_rx_param` interns the symbolic parameter into the circuit's
     * parameter table; the expanded body operation reports the INDEX tag
     * and carries the table index as its payload value. */
    struct CCircuit* circuit = circuit_new(1);
    struct CParameter* theta = param_parse("theta");
    CHECK(circuit != NULL && theta != NULL, "index test setup");
    CHECK(circuit_rx_param(circuit, 0, theta) == 0, "circuit_rx_param");
    CHECK(circuit_num_parameters(circuit) == 1, "circuit_num_parameters");
    param_free(theta);

    struct CCircuitCFG* cfg = circuit_cfg_from_circuit(circuit);
    CHECK(cfg != NULL, "circuit_cfg_from_circuit");
    CHECK(circuit_cfg_block_operations_len(cfg, 0) == 1, "index test block ops");

    struct COperation* op = NULL;
    CHECK(circuit_cfg_block_operations(cfg, 0, &op, 1) == 0 && op != NULL,
          "circuit_cfg_block_operations clone");
    CHECK(circuit_cfg_block_operations(cfg, 0, &op, 0) == -8,
          "circuit_cfg_block_operations short buffer");

    uint32_t tags[1] = {255};
    double values[1] = {-1.0};
    CHECK(operation_params_len(op) == 1, "index params_len");
    CHECK(operation_params(op, tags, values, 1) == 0, "index operation_params");
    CHECK(tags[0] == OPERATION_PARAM_INDEX, "index param tag");
    CHECK(values[0] == 0.0, "index param value");

    /* An unresolved index parameter has no numeric matrix. */
    CHECK(operation_matrix_len(op) == 0, "index matrix_len");
    uintptr_t rows = 1;
    uintptr_t cols = 1;
    CHECK(operation_matrix_dims(op, &rows, &cols) == -3, "index matrix_dims");

    operation_free(op);
    circuit_cfg_free(cfg);
    circuit_free(circuit);
}

static void test_operation_new_errors(void) {
    const uint32_t qubits[1] = {0};
    CHECK(operation_new(NULL, qubits, 1, NULL, 0) == NULL, "operation_new null name");
    CHECK(operation_new("NOT_A_GATE", NULL, 0, NULL, 0) == NULL, "operation_new unknown gate");
    CHECK(operation_new("H", NULL, 1, NULL, 0) == NULL, "operation_new null qubits");
    CHECK(operation_new("H", qubits, 1, NULL, 1) == NULL, "operation_new null params");
}

static void test_null_handles(void) {
    const uint32_t qubits[1] = {0};
    CHECK(operation_name(NULL) == NULL, "operation_name(null)");
    CHECK(operation_is_standard_gate(NULL) == -1, "operation_is_standard_gate(null)");
    CHECK(operation_num_qubits(NULL) == 0, "operation_num_qubits(null)");
    CHECK(operation_qubits_len(NULL) == 0, "operation_qubits_len(null)");
    CHECK(operation_qubits(NULL, NULL, 0) == -1, "operation_qubits(null)");
    CHECK(operation_params_len(NULL) == 0, "operation_params_len(null)");
    CHECK(operation_params(NULL, NULL, NULL, 0) == -1, "operation_params(null)");
    CHECK(operation_label(NULL) == NULL, "operation_label(null)");
    CHECK(operation_matrix_len(NULL) == 0, "operation_matrix_len(null)");
    CHECK(operation_matrix_dims(NULL, NULL, NULL) == -1, "operation_matrix_dims(null)");
    CHECK(operation_matrix(NULL, NULL, 0) == 0, "operation_matrix(null)");

    operation_free(NULL);
    CHECK(1, "operation_free(null)");

    /* Buffer arguments may not be NULL on success paths. */
    struct COperation* op = operation_new("H", qubits, 1, NULL, 0);
    CHECK(op != NULL, "null-buffer fixture");
    CHECK(operation_qubits(op, NULL, 1) == -1, "operation_qubits null out");
    uintptr_t rows = 0;
    uintptr_t cols = 0;
    CHECK(operation_matrix_dims(op, NULL, &cols) == -1, "dims null rows(2)");
    CHECK(operation_matrix_dims(op, &rows, NULL) == -1, "dims null cols(2)");
    CHECK(operation_matrix(op, NULL, 8) == 0, "operation_matrix null out");
    operation_free(op);
}

int main(void) {
    test_fixed_param_operation();
    test_parameterless_gate();
    test_index_param_operation();
    test_operation_new_errors();
    test_null_handles();

    printf("\n%d failure(s)\n", g_failures);
    return (g_failures == 0) ? 0 : 1;
}
