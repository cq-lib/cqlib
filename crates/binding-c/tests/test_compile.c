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
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cqlib_c.h"

static CCircuit* build_bell_circuit(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);
    return circuit;
}

static CompileConfigC logical_config(uint8_t mode) {
    CompileConfigC config;
    memset(&config, 0, sizeof(config));
    config.mode = mode;
    config.target = COMPILE_TARGET_LOGICAL;
    return config;
}

static void test_compile_logical_normal(void) {
    CCircuit* circuit = build_bell_circuit();

    CCompileResult* result = compile(circuit, logical_config(COMPILE_MODE_NORMAL));
    assert(result != NULL);
    assert(compile_result_mode(result) == COMPILE_MODE_NORMAL);

    CCircuit* compiled = compile_result_circuit(result);
    assert(compiled != NULL);
    assert(circuit_num_qubits(compiled) == 2);

    assert(compile_result_num_steps(result) >= 1);
    char* step = compile_result_step_name(result, 0);
    assert(step != NULL);
    cqlib_string_free(step);

    compile_result_free(result);
    circuit_free(compiled);
    circuit_free(circuit);
}

static void test_compile_enhanced(void) {
    CCircuit* circuit = build_bell_circuit();

    CCompileResult* result = compile(circuit, logical_config(COMPILE_MODE_ENHANCED));
    assert(result != NULL);
    assert(compile_result_mode(result) == COMPILE_MODE_ENHANCED);
    assert(compile_result_changed(result) == 0 || compile_result_changed(result) == 1);

    compile_result_free(result);
    circuit_free(circuit);
}

static void test_compile_errors(void) {
    assert(compile(NULL, logical_config(COMPILE_MODE_NORMAL)) == NULL);
    assert(compile_result_circuit(NULL) == NULL);
    assert(compile_result_num_steps(NULL) == 0);
    compile_result_free(NULL);
}

int main(void) {
    test_compile_logical_normal();
    test_compile_enhanced();
    test_compile_errors();
    printf("binding-c compile tests passed\n");
    return 0;
}
