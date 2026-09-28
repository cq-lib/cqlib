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

static void test_workflow_new_and_free(void) {
    CCompilerWorkflow* workflow = compiler_workflow_new(logical_config(COMPILE_MODE_NORMAL));
    assert(workflow != NULL);
    compiler_workflow_free(workflow);

    workflow = compiler_workflow_new(logical_config(COMPILE_MODE_ENHANCED));
    assert(workflow != NULL);
    compiler_workflow_free(workflow);

    // Invalid mode and unsupported target tags return NULL.
    assert(compiler_workflow_new(logical_config(42)) == NULL);

    CompileConfigC config = logical_config(COMPILE_MODE_NORMAL);
    config.target = COMPILE_TARGET_DEVICE;
    assert(compiler_workflow_new(config) == NULL);

    // Freeing NULL is allowed.
    compiler_workflow_free(NULL);
}

static void test_workflow_run_logical(void) {
    CCircuit* circuit = build_bell_circuit();
    CCompilerWorkflow* workflow = compiler_workflow_new(logical_config(COMPILE_MODE_ENHANCED));
    assert(workflow != NULL);

    CCompileResult* result = compiler_workflow_run(workflow, circuit);
    assert(result != NULL);
    assert(compile_result_mode(result) == COMPILE_MODE_ENHANCED);
    assert(compile_result_num_steps(result) > 0);

    CCircuit* optimized = compile_result_circuit(result);
    assert(optimized != NULL);
    assert(circuit_num_qubits(optimized) == 2);

    // The workflow is reusable: a second run on the same circuit works.
    CCompileResult* second = compiler_workflow_run(workflow, circuit);
    assert(second != NULL);
    compile_result_free(second);

    compile_result_free(result);
    circuit_free(optimized);
    compiler_workflow_free(workflow);
    circuit_free(circuit);
}

static void test_workflow_run_null_arguments(void) {
    CCircuit* circuit = build_bell_circuit();
    CCompilerWorkflow* workflow = compiler_workflow_new(logical_config(COMPILE_MODE_NORMAL));
    assert(workflow != NULL);

    assert(compiler_workflow_run(NULL, circuit) == NULL);
    assert(compiler_workflow_run(workflow, NULL) == NULL);

    compiler_workflow_free(workflow);
    circuit_free(circuit);
}

static void test_device_metadata_empty_after_logical_compile(void) {
    CCircuit* circuit = build_bell_circuit();
    CCompilerWorkflow* workflow = compiler_workflow_new(logical_config(COMPILE_MODE_NORMAL));
    assert(workflow != NULL);

    CCompileResult* result = compiler_workflow_run(workflow, circuit);
    assert(result != NULL);

    // A logical-target compile carries no device metadata: numeric fields
    // return 0, handle fields return NULL, and query functions report
    // absence.
    assert(compile_result_has_device_metadata(result) == 0);
    assert(compile_result_device_metadata_initial_layout(result) == NULL);
    assert(compile_result_device_metadata_final_layout(result) == NULL);
    assert(compile_result_device_metadata_permutation_len(result) == 0);
    assert(compile_result_device_metadata_permutation(result, NULL, NULL, 0) == 0);

    // NULL result handling.
    assert(compile_result_has_device_metadata(NULL) == -1);
    assert(compile_result_device_metadata_initial_layout(NULL) == NULL);
    assert(compile_result_device_metadata_final_layout(NULL) == NULL);
    assert(compile_result_device_metadata_permutation_len(NULL) == 0);
    assert(compile_result_device_metadata_permutation(NULL, NULL, NULL, 0) == -1);

    compile_result_free(result);
    compiler_workflow_free(workflow);
    circuit_free(circuit);
}

int main(void) {
    test_workflow_new_and_free();
    test_workflow_run_logical();
    test_workflow_run_null_arguments();
    test_device_metadata_empty_after_logical_compile();
    printf("binding-c compile workflow tests passed\n");
    return 0;
}
