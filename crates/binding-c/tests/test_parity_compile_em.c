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
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "cqlib_c.h"

static void test_evolution_info(void) {
    CHamiltonian* h = hamiltonian_new(2);
    assert(h != NULL);

    CPauliString* zz = pauli_string_parse("ZZ");
    assert(zz != NULL);
    assert(hamiltonian_add_term(h, zz, 0.5, 0.0) == 0);
    pauli_string_free(zz);

    CPauliString* zi = pauli_string_parse("ZI");
    assert(zi != NULL);
    assert(hamiltonian_add_term(h, zi, 0.3, 0.0) == 0);
    pauli_string_free(zi);

    CPauliEvolutionAnsatz* ansatz = pauli_evolution_ansatz_new(h);
    assert(ansatz != NULL);

    // ZZ and ZI mutually commute: Auto takes the exact single-pass path.
    CPauliEvolutionInfo info;
    memset(&info, 0, sizeof(info));
    assert(pauli_evolution_ansatz_evolution_info(ansatz, &info) == 0);
    assert(info.is_exact == 1);
    assert(info.all_terms_commute == 1);
    assert(info.steps == 1);
    assert(info.num_terms == 2);
    assert(info.trotter_mode == TROTTER_MODE_NONE);
    assert(info.trotter_seed == 0);

    // NULL guards.
    CPauliEvolutionInfo sink;
    assert(pauli_evolution_ansatz_evolution_info(NULL, &sink) == -1);
    assert(pauli_evolution_ansatz_evolution_info(ansatz, NULL) == -1);

    pauli_evolution_ansatz_free(ansatz);
    hamiltonian_free(h);
}

static void test_copy_swap_circuit(void) {
    CCircuit* circuit = circuit_new(1);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);

    CVirtualDistillation* vd = virtual_distillation_new(circuit, 2);
    assert(vd != NULL);

    CCircuit* copy_swap = virtual_distillation_build_copy_swap_circuit(vd);
    assert(copy_swap != NULL);
    assert(circuit_num_qubits(copy_swap) == 2);
    assert(circuit_num_operations(copy_swap) > 0);
    circuit_free(copy_swap);

    assert(virtual_distillation_build_copy_swap_circuit(NULL) == NULL);

    virtual_distillation_free(vd);
    circuit_free(circuit);
}

static void assert_symbol_list(char** out, uintptr_t len, const char* expected0,
                               const char* expected1) {
    assert(strcmp(out[0], expected0) == 0);
    assert(strcmp(out[1], expected1) == 0);
    for (uintptr_t i = 0; i < len; i++) {
        cqlib_string_free(out[i]);
    }
}

static void test_rule_symbols(void) {
    const char* merge_dsl =
        "rule merge_rz {\n"
        "    match {\n"
        "        RZ(a) 0\n"
        "        RZ(b) 0\n"
        "    }\n"
        "    rewrite {\n"
        "        RZ(a + b) 0\n"
        "    }\n"
        "}";
    const char* cancel_dsl =
        "rule cancel_h {\n"
        "    match {\n"
        "        H 0\n"
        "        H 0\n"
        "    }\n"
        "    rewrite {}\n"
        "}";
    const char* condition_dsl =
        "rule cancel_rz_inverse {\n"
        "    match { RZ(a) 0, RZ(b) 0 }\n"
        "    require { a + b == 0 mod 4*π }\n"
        "    rewrite {}\n"
        "}";

    CKnowledgeRule* merge = NULL;
    assert(knowledge_rule_from_dsl(merge_dsl, &merge) == 0);
    assert(merge != NULL);
    CKnowledgeRule* cancel = NULL;
    assert(knowledge_rule_from_dsl(cancel_dsl, &cancel) == 0);
    assert(cancel != NULL);
    CKnowledgeRule* condition = NULL;
    assert(knowledge_rule_from_dsl(condition_dsl, &condition) == 0);
    assert(condition != NULL);

    // Match-block symbols: the merge pattern binds a and b (sorted).
    assert(knowledge_rule_operation_symbols_len(merge) == 2);
    char* ops[2] = {NULL, NULL};
    assert(knowledge_rule_operation_symbols(merge, ops, 2) == 0);
    assert_symbol_list(ops, 2, "a", "b");

    // Rewrite-target symbols: the merge target references a and b; the
    // cancel target is empty.
    assert(knowledge_rule_target_symbols_len(merge) == 2);
    char* targets[2] = {NULL, NULL};
    assert(knowledge_rule_target_symbols(merge, targets, 2) == 0);
    assert_symbol_list(targets, 2, "a", "b");
    assert(knowledge_rule_target_symbols_len(cancel) == 0);
    assert(knowledge_rule_target_symbols(cancel, NULL, 0) == 0);

    // Condition symbols: only the condition rule carries a require block.
    assert(knowledge_rule_condition_symbols_len(condition) == 2);
    char* conds[2] = {NULL, NULL};
    assert(knowledge_rule_condition_symbols(condition, conds, 2) == 0);
    assert_symbol_list(conds, 2, "a", "b");
    assert(knowledge_rule_condition_symbols_len(merge) == 0);
    assert(knowledge_rule_condition_symbols_len(cancel) == 0);

    // Wrong buffer lengths and NULL buffers are rejected.
    char* short_out[1] = {NULL};
    assert(knowledge_rule_operation_symbols(merge, short_out, 1) == -8);
    assert(short_out[0] == NULL);
    assert(knowledge_rule_operation_symbols(merge, NULL, 2) == -1);

    // NULL guards.
    assert(knowledge_rule_operation_symbols_len(NULL) == 0);
    assert(knowledge_rule_target_symbols_len(NULL) == 0);
    assert(knowledge_rule_condition_symbols_len(NULL) == 0);
    assert(knowledge_rule_operation_symbols(NULL, NULL, 0) == -1);
    assert(knowledge_rule_target_symbols(NULL, NULL, 0) == -1);
    assert(knowledge_rule_condition_symbols(NULL, NULL, 0) == -1);

    knowledge_rule_free(condition);
    knowledge_rule_free(cancel);
    knowledge_rule_free(merge);
}

static void test_step_reason(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);

    CompileConfigC config;
    memset(&config, 0, sizeof(config));
    config.mode = COMPILE_MODE_NORMAL;
    config.target = COMPILE_TARGET_LOGICAL;

    CCompileResult* result = compile(circuit, config);
    assert(result != NULL);

    uintptr_t steps = compile_result_num_steps(result);
    assert(steps > 0);

    // Every step has a name; a logical-target compile skips the target-basis
    // steps with documented reasons, so at least one reason must be present.
    uintptr_t reasons = 0;
    for (uintptr_t i = 0; i < steps; i++) {
        char* name = compile_result_step_name(result, i);
        assert(name != NULL && strlen(name) > 0);
        cqlib_string_free(name);

        char* reason = compile_result_step_reason(result, i);
        if (reason != NULL) {
            assert(strlen(reason) > 0);
            cqlib_string_free(reason);
            reasons++;
        }
    }
    assert(reasons > 0);

    // Out-of-bounds indices and NULL input yield NULL.
    assert(compile_result_step_reason(result, steps + 10) == NULL);
    assert(compile_result_step_reason(NULL, 0) == NULL);

    compile_result_free(result);
    circuit_free(circuit);
}

int main(void) {
    test_evolution_info();
    test_copy_swap_circuit();
    test_rule_symbols();
    test_step_reason();
    printf("binding-c parity compile/em tests passed\n");
    return 0;
}
