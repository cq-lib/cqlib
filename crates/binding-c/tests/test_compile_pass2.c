// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// use this file except in compliance with the License. You may obtain
// a copy of the License in the LICENSE.txt file in the root directory
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

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

static void test_decompose_passes(void) {
    // expand_definitions: a frozen circuit-gate is expanded back into plain H.
    CCircuit* inner = circuit_new(1);
    assert(inner != NULL);
    assert(circuit_h(inner, 0) == 0);
    CCircuitGate* gate = circuit_to_gate(inner, "my_h");
    assert(gate != NULL);
    CCircuit* outer = circuit_new(2);
    assert(outer != NULL);
    uint32_t gate_qubits[1] = {0};
    assert(circuit_circuit_gate(outer, gate, gate_qubits, 1, NULL, 0) == 0);
    CCircuit* expanded = decompose_expand_definitions(outer);
    assert(expanded != NULL);
    assert(circuit_num_operations(expanded) == 1);
    circuit_free(expanded);
    circuit_gate_free(gate);
    circuit_free(outer);
    circuit_free(inner);

    // unitaries: a matrix-backed H is synthesized into standard operations.
    Complex64 h_matrix[4] = {
        {1.0 / sqrt(2.0), 0.0},
        {1.0 / sqrt(2.0), 0.0},
        {1.0 / sqrt(2.0), 0.0},
        {-1.0 / sqrt(2.0), 0.0},
    };
    CCircuit* unitary_circuit = circuit_new(2);
    assert(unitary_circuit != NULL);
    uint32_t unitary_targets[1] = {0};
    assert(circuit_unitary(unitary_circuit, "my_u", 1, h_matrix, unitary_targets, 1) == 0);
    CCircuit* synthesized =
        decompose_unitaries(unitary_circuit, decompose_unitary_config_default());
    assert(synthesized != NULL);
    assert(circuit_num_operations(synthesized) >= 1);
    circuit_free(synthesized);
    CDecompositionRuleStats stats;
    memset(&stats, 0, sizeof(stats));
    synthesized = decompose_unitaries_with_rule_stats(unitary_circuit,
                                                      decompose_unitary_config_default(), &stats);
    assert(synthesized != NULL);
    circuit_free(synthesized);
    circuit_free(unitary_circuit);

    // mc_gates: a doubly-controlled X is rewritten, bounded or not.
    CCircuit* mc_circuit = circuit_new(3);
    assert(mc_circuit != NULL);
    uint32_t controls[2] = {0, 1};
    uint32_t targets[1] = {2};
    assert(circuit_multi_control(mc_circuit, "X", controls, 2, targets, 1, NULL, 0) == 0);
    CCircuit* rewritten = decompose_mc_gates(mc_circuit, mc_gate_config_default());
    assert(rewritten != NULL);
    assert(circuit_num_operations(rewritten) >= 1);
    circuit_free(rewritten);
    memset(&stats, 0, sizeof(stats));
    rewritten = decompose_mc_gates_with_rule_stats(mc_circuit, mc_gate_config_default(), &stats);
    assert(rewritten != NULL);
    circuit_free(rewritten);
    uint32_t edges[4] = {0, 1, 1, 2};
    CDevice* device = device_from_edges("line-3", 3, edges, 2);
    assert(device != NULL);
    rewritten = decompose_mc_gates_for_device(mc_circuit, device, NULL);
    assert(rewritten != NULL);
    circuit_free(rewritten);
    device_free(device);
    circuit_free(mc_circuit);

    // resynthesis: both budgets rebuild the two-qubit blocks.
    CCircuit* block_circuit = circuit_new(2);
    assert(block_circuit != NULL);
    assert(circuit_h(block_circuit, 0) == 0);
    assert(circuit_cx(block_circuit, 0, 1) == 0);
    CCircuit* resynthesized =
        resynthesize_two_qubit_blocks(block_circuit, resynthesis_config_normal());
    assert(resynthesized != NULL);
    assert(circuit_num_operations(resynthesized) >= 1);
    circuit_free(resynthesized);
    resynthesized = resynthesize_two_qubit_blocks(block_circuit, resynthesis_config_enhanced());
    assert(resynthesized != NULL);
    circuit_free(resynthesized);
    circuit_free(block_circuit);
}

static void test_numeric_synthesis_and_lowering(void) {
    // 1q synthesis of the identity yields U(0, 0, 0) with zero global phase.
    Complex64 identity2[4] = {{1.0, 0.0}, {0.0, 0.0}, {0.0, 0.0}, {1.0, 0.0}};
    COneQubitUnitaryDecomposition one_q;
    memset(&one_q, 0, sizeof(one_q));
    assert(synthesize_numeric_1q_unitary(identity2, &one_q) == 0);
    assert(fabs(one_q.theta) < 1e-10);
    assert(fabs(one_q.phi) < 1e-10);
    assert(fabs(one_q.lambda) < 1e-10);
    assert(fabs(one_q.global_phase) < 1e-10);

    // 2q synthesis of the identity succeeds and exposes its operations.
    Complex64 identity4[16];
    memset(identity4, 0, sizeof(identity4));
    for (int i = 0; i < 16; i += 5) {
        identity4[i].re = 1.0;
    }
    CTwoQubitUnitarySynthesis* synthesis =
        synthesize_numeric_2q_unitary(identity4, 0, 1, TWO_QUBIT_BASIS_CX);
    assert(synthesis != NULL);
    assert(isfinite(two_qubit_synthesis_global_phase(synthesis)));
    for (uintptr_t i = 0; i < two_qubit_synthesis_num_operations(synthesis); i++) {
        char* op_name = two_qubit_synthesis_operation_name(synthesis, i);
        assert(op_name != NULL);
        cqlib_string_free(op_name);
        uintptr_t qubits_len = two_qubit_synthesis_operation_qubits_len(synthesis, i);
        assert(qubits_len >= 1);
        assert(qubits_len <= 8);
        uint32_t qubits[8];
        assert(two_qubit_synthesis_operation_qubits(synthesis, i, qubits, qubits_len) ==
               qubits_len);
    }
    two_qubit_synthesis_free(synthesis);

    // KAK decomposition of the identity has zero interaction coordinates.
    CKakDecomposition* kak = kak_decompose(identity4);
    assert(kak != NULL);
    double a = 0.0;
    double b = 0.0;
    double c = 0.0;
    assert(kak_decomposition_coordinates(kak, &a, &b, &c) == 0);
    assert(fabs(a) < 1e-6);
    assert(fabs(b) < 1e-6);
    assert(fabs(c) < 1e-6);
    assert(fabs(kak_decomposition_global_phase(kak)) < 1e-6);
    Complex64 factor[4];
    memset(factor, 0, sizeof(factor));
    assert(kak_decomposition_local_factor(kak, KAK_FACTOR_K1L, factor) == 0);
    assert(kak_decomposition_local_factor(kak, 99, factor) == -8);
    kak_decomposition_free(kak);

    // Target-basis lowering: T is outside {H, RZ, CX}.
    const char* basis_names[3] = {"H", "RZ", "CX"};
    CTargetBasisLowerer* lowerer = target_basis_lowerer_new(basis_names, 3);
    assert(lowerer != NULL);
    assert(target_basis_lowerer_num_gates(lowerer) == 3);
    CCircuit* t_circuit = circuit_new(1);
    assert(t_circuit != NULL);
    assert(circuit_t(t_circuit, 0) == 0);
    assert(target_basis_lowerer_requires_lowering(lowerer, t_circuit) == 1);
    CCircuit* lowered = target_basis_lowerer_apply(lowerer, t_circuit);
    assert(lowered != NULL);
    assert(circuit_num_operations(lowered) >= 1);
    circuit_free(lowered);
    circuit_free(t_circuit);

    // A circuit already in the basis needs no lowering.
    CCircuit* basis_circuit = circuit_new(2);
    assert(basis_circuit != NULL);
    assert(circuit_h(basis_circuit, 0) == 0);
    assert(circuit_cx(basis_circuit, 0, 1) == 0);
    assert(target_basis_lowerer_requires_lowering(lowerer, basis_circuit) == 0);
    circuit_free(basis_circuit);
    target_basis_lowerer_free(lowerer);

    // Unknown gate names and empty bases are rejected.
    const char* bad_names[1] = {"NOPE"};
    assert(target_basis_lowerer_new(bad_names, 1) == NULL);
    assert(target_basis_lowerer_new(NULL, 0) == NULL);

    // Device lowering: a native circuit passes through unchanged.
    uint32_t edges[2] = {0, 1};
    CDevice* device = device_from_edges("line-2", 2, edges, 1);
    assert(device != NULL);
    assert(device_with_native_gates(device, "H,RZ,CX") == 0);
    CCircuit* native_circuit = circuit_new(2);
    assert(native_circuit != NULL);
    assert(circuit_h(native_circuit, 0) == 0);
    assert(circuit_cx(native_circuit, 0, 1) == 0);
    CCircuit* device_lowered = decompose_lower_to_device(native_circuit, device);
    assert(device_lowered != NULL);
    assert(circuit_num_operations(device_lowered) == 2);
    circuit_free(device_lowered);
    circuit_free(native_circuit);
    device_free(device);
}

static void test_resource_lifecycle(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);

    CResourcePolicy policy = resource_policy_default();
    policy.max_pre_layout_clean_ancillas = 2;

    CResourceManager* manager = resource_manager_from_circuit(circuit, &policy, NULL);
    assert(manager != NULL);
    assert(resource_manager_verify_consistency(manager) == 0);
    assert(resource_manager_verify_idle(manager) == 0);

    // Invalid requirement tags and oversized requests are rejected.
    assert(resource_manager_preview(manager, 99, 1, NULL, 0) == NULL);
    assert(resource_manager_preview(manager, RESOURCE_REQUIREMENT_CLEAN_ZERO, 5, NULL, 0) == NULL);

    // Preview plans one fresh clean ancilla beyond the input width.
    CResourcePlan* plan =
        resource_manager_preview(manager, RESOURCE_REQUIREMENT_CLEAN_ZERO, 1, NULL, 0);
    assert(plan != NULL);
    assert(resource_plan_qubits_len(plan) == 1);
    assert(resource_plan_num_new_qubits(plan) == 1);
    assert(resource_plan_requirement(plan) == RESOURCE_REQUIREMENT_CLEAN_ZERO);
    uint32_t planned[1] = {7};
    assert(resource_plan_qubits(plan, planned, 1) == 1);
    assert(planned[0] == 2);

    // Commit mutates the manager's working circuit and reserves the qubit.
    CResourceLease* lease = resource_manager_commit(manager, plan);
    assert(lease != NULL);
    assert(resource_lease_qubits_len(lease) == 1);
    assert(resource_lease_requirement(lease) == RESOURCE_REQUIREMENT_CLEAN_ZERO);
    CCircuit* working = resource_manager_circuit(manager);
    assert(working != NULL);
    assert(circuit_num_qubits(working) == 3);
    circuit_free(working);

    // While the lease is active the manager is consistent but not idle.
    assert(resource_manager_verify_consistency(manager) == 0);
    assert(resource_manager_verify_idle(manager) == -6);

    // Release frees the lease; a second release is rejected.
    assert(resource_manager_release(manager, lease) == 0);
    assert(resource_manager_verify_idle(manager) == 0);
    assert(resource_manager_release(manager, lease) == -6);

    resource_lease_free(lease);
    resource_manager_free(manager);
    circuit_free(circuit);
}

static void test_knowledge_library(void) {
    // The builtin library carries rules under their compiler kinds.
    CKnowledgeLibrary* builtin = knowledge_library_builtin();
    assert(builtin != NULL);
    assert(knowledge_library_len(builtin) > 0);
    assert(knowledge_library_is_empty(builtin) == 0);
    assert(knowledge_library_rules_by_kind_len(builtin, RULE_KIND_DECOMPOSE) > 0);
    assert(knowledge_library_rules_by_kind_len(builtin, RULE_KIND_CANCEL) > 0);
    uintptr_t decompose_len = knowledge_library_rules_by_kind_len(builtin, RULE_KIND_DECOMPOSE);
    uint32_t* decompose_ids = malloc(decompose_len * sizeof(uint32_t));
    assert(decompose_ids != NULL);
    assert(knowledge_library_rules_by_kind(builtin, RULE_KIND_DECOMPOSE, decompose_ids,
                                           decompose_len) == decompose_len);
    free(decompose_ids);

    // Metadata accessors cover every builtin rule.
    for (uintptr_t i = 0; i < knowledge_library_len(builtin); i++) {
        char* rule_name = knowledge_library_rule_name(builtin, i);
        assert(rule_name != NULL);
        cqlib_string_free(rule_name);
        int32_t kind = knowledge_library_rule_kind(builtin, i);
        assert(kind >= RULE_KIND_SIMPLIFY);
        assert(kind <= RULE_KIND_OTHER);
        assert(knowledge_library_rule_pattern_len(builtin, i) >= 1);
    }
    char* first = knowledge_library_rule_first_instruction(builtin, 0);
    assert(first != NULL);
    cqlib_string_free(first);

    // Out-of-bounds indices are rejected.
    assert(knowledge_library_rule_name(builtin, UINTPTR_MAX) == NULL);
    assert(knowledge_library_rule_kind(builtin, UINTPTR_MAX) == -8);
    knowledge_library_free(builtin);

    // DSL text parsing with a valid merge rule.
    const char* source =
        "rule merge_rz {\n"
        "    match {\n"
        "        RZ(a) 0\n"
        "        RZ(b) 0\n"
        "    }\n"
        "    rewrite {\n"
        "        RZ(a + b) 0\n"
        "    }\n"
        "}";
    CKnowledgeLibrary* library = NULL;
    assert(knowledge_library_from_dsl_str(source, RULE_KIND_MERGE, &library) == 0);
    assert(library != NULL);
    assert(knowledge_library_len(library) == 1);
    assert(knowledge_library_contains(library, "merge_rz") == 1);
    assert(knowledge_library_id_by_name(library, "merge_rz") == 0);
    assert(knowledge_library_rule_pattern_len(library, 0) == 2);
    assert(knowledge_library_rule_rewrite_len(library, 0) == 1);
    assert(knowledge_library_rule_qubit_count(library, 0) == 1);
    assert(knowledge_library_rule_cost_delta(library, 0) == -1);
    assert(knowledge_library_rule_has_conditions(library, 0) == 0);
    assert(knowledge_library_rule_kind(library, 0) == RULE_KIND_MERGE);
    char* rule_name = knowledge_library_rule_name(library, 0);
    assert(rule_name != NULL);
    assert(strcmp(rule_name, "merge_rz") == 0);
    cqlib_string_free(rule_name);

    // Unknown names report absence, not errors.
    assert(knowledge_library_contains(library, "no_such_rule") == 0);
    assert(knowledge_library_id_by_name(library, "no_such_rule") == -1);

    // Unknown kind tags are rejected without touching `out`.
    assert(knowledge_library_from_dsl_str(source, 99, &library) == -8);
    knowledge_library_free(library);

    // Malformed DSL text and missing files map to their error codes.
    CKnowledgeLibrary* bad = NULL;
    assert(knowledge_library_from_dsl_str("this is not a rule", RULE_KIND_OTHER, &bad) == -4);
    assert(knowledge_library_from_dsl_file("no_such_rule_file.rule", RULE_KIND_OTHER, &bad) == -5);
    assert(knowledge_library_from_dsl_str(NULL, RULE_KIND_OTHER, NULL) == -1);
    assert(knowledge_library_from_dsl_file(NULL, RULE_KIND_OTHER, NULL) == -1);
}

static void test_knowledge_rule_add(void) {
    const char* source =
        "rule cancel_h {\n"
        "    match {\n"
        "        H 0\n"
        "        H 0\n"
        "    }\n"
        "    rewrite {}\n"
        "}";
    CKnowledgeRule* rule = NULL;
    assert(knowledge_rule_from_dsl(source, &rule) == 0);
    assert(rule != NULL);
    assert(knowledge_rule_validate(rule) == 0);
    assert(knowledge_rule_pattern_len(rule) == 2);
    assert(knowledge_rule_rewrite_len(rule) == 0);
    assert(knowledge_rule_num_qubits(rule) == 1);
    assert(knowledge_rule_has_conditions(rule) == 0);
    char* name = knowledge_rule_name(rule);
    assert(name != NULL);
    assert(strcmp(name, "cancel_h") == 0);
    cqlib_string_free(name);

    // Insertion into a fresh library assigns id 0.
    CKnowledgeLibrary* library = knowledge_library_new();
    assert(library != NULL);
    assert(knowledge_library_is_empty(library) == 1);
    assert(knowledge_library_add_rule(library, rule, RULE_KIND_CANCEL) == 0);
    assert(knowledge_library_len(library) == 1);
    assert(knowledge_library_rule_kind(library, 0) == RULE_KIND_CANCEL);

    // Duplicate names and unknown kind tags are rejected.
    assert(knowledge_library_add_rule(library, rule, RULE_KIND_CANCEL) == -4);
    assert(knowledge_library_add_rule(library, rule, 99) == -8);
    knowledge_library_free(library);
    knowledge_rule_free(rule);

    // Malformed DSL text fails with the parse error code.
    CKnowledgeRule* bad = NULL;
    assert(knowledge_rule_from_dsl("this is not a rule", &bad) == -4);
    assert(knowledge_rule_from_dsl(NULL, NULL) == -1);
}

static void test_commutation(void) {
    uint32_t q0[1] = {0};
    uint32_t q1[1] = {1};
    double angle_a[1] = {0.3};
    double angle_b[1] = {0.5};
    double phase = 0.0;

    // Disjoint supports commute exactly.
    assert(commutation_check("X", q0, 1, NULL, 0, "X", q1, 1, NULL, 0, &phase) ==
           COMMUTATION_RESULT_EXACT);
    assert(phase == 0.0);

    // The same application commutes exactly.
    assert(commutation_check("X", q0, 1, NULL, 0, "X", q0, 1, NULL, 0, &phase) ==
           COMMUTATION_RESULT_EXACT);

    // X and Z on the same qubit commute up to a global phase of pi.
    phase = 0.0;
    assert(commutation_check("X", q0, 1, NULL, 0, "Z", q0, 1, NULL, 0, &phase) ==
           COMMUTATION_RESULT_UP_TO_GLOBAL_PHASE);
    assert(fabs(phase - M_PI) < 1e-9);

    // Diagonal rotations commute exactly with numeric parameters.
    assert(commutation_check("RZ", q0, 1, angle_a, 1, "RZ", q0, 1, angle_b, 1, &phase) ==
           COMMUTATION_RESULT_EXACT);

    // The algebraic-only variant proves the same facts.
    assert(commutation_check_algebraic("X", q0, 1, NULL, 0, "X", q1, 1, NULL, 0, &phase) ==
           COMMUTATION_RESULT_EXACT);
    phase = 0.0;
    assert(commutation_check_algebraic("X", q0, 1, NULL, 0, "Z", q0, 1, NULL, 0, &phase) ==
           COMMUTATION_RESULT_UP_TO_GLOBAL_PHASE);
    assert(fabs(phase - M_PI) < 1e-9);

    // Unknown gate names and NULL inputs are rejected.
    assert(commutation_check("NOPE", q0, 1, NULL, 0, "X", q0, 1, NULL, 0, &phase) == -8);
    assert(commutation_check(NULL, q0, 1, NULL, 0, "X", q0, 1, NULL, 0, &phase) == -1);

    // Reusable checkers with an explicit configuration.
    CCommutationConfig config = commutation_config_default();
    assert(config.enable_rule_oracle == 1);
    assert(config.enable_matrix_fallback == 1);
    assert(config.max_matrix_qubits == 4);
    CCommutationChecker* checker = commutation_checker_new(config);
    assert(checker != NULL);
    assert(commutation_checker_check(checker, "X", q0, 1, NULL, 0, "X", q1, 1, NULL, 0, &phase) ==
           COMMUTATION_RESULT_EXACT);
    assert(commutation_checker_check(NULL, "X", q0, 1, NULL, 0, "X", q1, 1, NULL, 0, &phase) == -1);
    commutation_checker_free(checker);

    // A checker can draw its commutation rules from a knowledge library.
    CKnowledgeLibrary* library = knowledge_library_builtin();
    assert(library != NULL);
    CCommutationChecker* from_library = commutation_checker_from_library(library, config);
    assert(from_library != NULL);
    assert(commutation_checker_from_library(NULL, config) == NULL);
    commutation_checker_free(from_library);
    knowledge_library_free(library);
}

static void test_null_guards(void) {
    double a = 0.0;
    double b = 0.0;
    double c = 0.0;

    // Decompose entry points.
    assert(decompose_expand_definitions(NULL) == NULL);
    assert(decompose_unitaries(NULL, decompose_unitary_config_default()) == NULL);
    assert(decompose_unitaries_with_rule_stats(NULL, decompose_unitary_config_default(), NULL) ==
           NULL);
    assert(decompose_mc_gates(NULL, mc_gate_config_default()) == NULL);
    assert(decompose_mc_gates_with_rule_stats(NULL, mc_gate_config_default(), NULL) == NULL);
    assert(decompose_mc_gates_for_device(NULL, NULL, NULL) == NULL);
    assert(resynthesize_two_qubit_blocks(NULL, resynthesis_config_normal()) == NULL);
    assert(synthesize_numeric_2q_unitary(NULL, 0, 1, TWO_QUBIT_BASIS_CX) == NULL);
    assert(kak_decompose(NULL) == NULL);
    assert(synthesize_numeric_1q_unitary(NULL, NULL) == -1);
    assert(kak_decomposition_coordinates(NULL, &a, &b, &c) == -1);
    Complex64 factor[4];
    memset(factor, 0, sizeof(factor));
    assert(kak_decomposition_local_factor(NULL, KAK_FACTOR_K1L, factor) == -1);
    assert(two_qubit_synthesis_num_operations(NULL) == 0);
    assert(two_qubit_synthesis_global_phase(NULL) == 0.0);
    assert(two_qubit_synthesis_operation_name(NULL, 0) == NULL);
    assert(two_qubit_synthesis_operation_qubits(NULL, 0, NULL, 0) == 0);
    assert(two_qubit_synthesis_operation_qubits_len(NULL, 0) == 0);
    assert(kak_decomposition_global_phase(NULL) == 0.0);
    assert(target_basis_lowerer_apply(NULL, NULL) == NULL);
    assert(target_basis_lowerer_requires_lowering(NULL, NULL) == -1);
    assert(target_basis_lowerer_num_gates(NULL) == 0);
    assert(decompose_lower_to_device(NULL, NULL) == NULL);
    two_qubit_synthesis_free(NULL);
    kak_decomposition_free(NULL);
    target_basis_lowerer_free(NULL);

    // Resource entry points.
    assert(resource_manager_from_circuit(NULL, NULL, NULL) == NULL);
    assert(resource_manager_preview(NULL, RESOURCE_REQUIREMENT_CLEAN_ZERO, 1, NULL, 0) == NULL);
    assert(resource_manager_commit(NULL, NULL) == NULL);
    assert(resource_manager_circuit(NULL) == NULL);
    assert(resource_manager_release(NULL, NULL) == -1);
    assert(resource_manager_enter_post_layout(NULL) == -1);
    assert(resource_manager_verify_consistency(NULL) == -1);
    assert(resource_manager_verify_idle(NULL) == -1);
    assert(resource_plan_qubits_len(NULL) == 0);
    assert(resource_plan_num_new_qubits(NULL) == 0);
    assert(resource_plan_requirement(NULL) == -1);
    assert(resource_lease_qubits_len(NULL) == 0);
    assert(resource_lease_requirement(NULL) == -1);
    resource_manager_free(NULL);
    resource_plan_free(NULL);
    resource_lease_free(NULL);

    // Knowledge entry points.
    assert(knowledge_library_len(NULL) == 0);
    assert(knowledge_library_is_empty(NULL) == -1);
    assert(knowledge_library_contains(NULL, NULL) == -1);
    assert(knowledge_library_id_by_name(NULL, NULL) == -1);
    assert(knowledge_library_rules_by_kind_len(NULL, RULE_KIND_OTHER) == 0);
    assert(knowledge_library_rules_by_kind(NULL, RULE_KIND_OTHER, NULL, 0) == 0);
    assert(knowledge_library_rule_name(NULL, 0) == NULL);
    assert(knowledge_library_rule_kind(NULL, 0) == -1);
    assert(knowledge_library_rule_first_instruction(NULL, 0) == NULL);
    assert(knowledge_library_rule_pattern_len(NULL, 0) == 0);
    assert(knowledge_library_rule_rewrite_len(NULL, 0) == 0);
    assert(knowledge_library_rule_qubit_count(NULL, 0) == 0);
    assert(knowledge_library_rule_cost_delta(NULL, 0) == 0);
    assert(knowledge_library_rule_has_conditions(NULL, 0) == -1);
    assert(knowledge_library_add_rule(NULL, NULL, RULE_KIND_OTHER) == -1);
    assert(knowledge_rule_name(NULL) == NULL);
    assert(knowledge_rule_num_qubits(NULL) == 0);
    assert(knowledge_rule_pattern_len(NULL) == 0);
    assert(knowledge_rule_rewrite_len(NULL) == 0);
    assert(knowledge_rule_has_conditions(NULL) == -1);
    assert(knowledge_rule_validate(NULL) == -1);
    knowledge_library_free(NULL);
    knowledge_rule_free(NULL);

    // Commutation entry points.
    assert(commutation_check(NULL, NULL, 0, NULL, 0, NULL, NULL, 0, NULL, 0, NULL) == -1);
    assert(commutation_check_algebraic(NULL, NULL, 0, NULL, 0, NULL, NULL, 0, NULL, 0, NULL) == -1);
    assert(commutation_checker_check(NULL, NULL, NULL, 0, NULL, 0, NULL, NULL, 0, NULL, 0, NULL) ==
           -1);
    commutation_checker_free(NULL);
}

int main(void) {
    test_decompose_passes();
    test_numeric_synthesis_and_lowering();
    test_resource_lifecycle();
    test_knowledge_library();
    test_knowledge_rule_add();
    test_commutation();
    test_null_guards();
    printf("binding-c compile pass2 tests passed\n");
    return 0;
}
