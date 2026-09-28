// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating that
// they have been altered from the originals.

#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "cqlib_c.h"

// Sorts a small array of freshly allocated gate names in place.
static void sort_names(char** names, uintptr_t len) {
    for (uintptr_t i = 0; i + 1 < len; i++) {
        for (uintptr_t j = i + 1; j < len; j++) {
            if (strcmp(names[i], names[j]) > 0) {
                char* tmp = names[i];
                names[i] = names[j];
                names[j] = tmp;
            }
        }
    }
}

static void test_transform_config_defaults(void) {
    // Production defaults: conservative optimization mode.
    struct CTransformConfig* config = transform_config_default();
    assert(config != NULL);
    assert(transform_config_mode(config) == REWRITE_MODE_OPTIMIZE);
    assert(transform_config_max_rounds(config) == 8u);
    assert(transform_config_max_window_ops(config) == 16u);
    assert(transform_config_max_pattern_len(config) == 8u);
    assert(transform_config_recurse_control_flow(config) == 1);
    assert(transform_config_skip_labeled_ops(config) == 1);
    assert(transform_config_target_instruction_basis_len(config) == 0u);

    // Default rule categories: simplify, cancel, merge, canonicalize.
    assert(transform_config_enabled_kinds_len(config) == 4u);
    uint8_t kinds[4] = {0};
    assert(transform_config_enabled_kinds(config, kinds, 4u) == 4u);
    assert(kinds[0] == REWRITE_KIND_SIMPLIFY);
    assert(kinds[1] == REWRITE_KIND_CANCEL);
    assert(kinds[2] == REWRITE_KIND_MERGE);
    assert(kinds[3] == REWRITE_KIND_CANONICALIZE);

    transform_config_free(config);

    // Lowering defaults switch the mode and widen the rule set.
    struct CTransformConfig* lowering = transform_config_lowering();
    assert(lowering != NULL);
    assert(transform_config_mode(lowering) == REWRITE_MODE_LOWERING);
    assert(transform_config_enabled_kinds_len(lowering) == 6u);
    transform_config_free(lowering);

    // NULL handling: getters degrade, setters reject, free allows NULL.
    assert(transform_config_mode(NULL) == -1);
    assert(transform_config_max_rounds(NULL) == 0u);
    assert(transform_config_enabled_kinds_len(NULL) == 0u);
    assert(transform_config_with_mode(NULL, REWRITE_MODE_OPTIMIZE) == -1);
    transform_config_free(NULL);
}

static void test_transform_config_setters(void) {
    struct CTransformConfig* config = transform_config_default();
    assert(config != NULL);

    // Setters mutate the config in place and return a status code.
    assert(transform_config_with_mode(config, REWRITE_MODE_LOWERING) == 0);
    assert(transform_config_mode(config) == REWRITE_MODE_LOWERING);
    assert(transform_config_with_mode(config, 99) == -8);

    assert(transform_config_with_max_rounds(config, 3) == 0);
    assert(transform_config_max_rounds(config) == 3u);
    assert(transform_config_with_max_window_ops(config, 64) == 0);
    assert(transform_config_max_window_ops(config) == 64u);
    assert(transform_config_with_max_pattern_len(config, 12) == 0);
    assert(transform_config_max_pattern_len(config) == 12u);

    // Replacing the enabled rule categories.
    const uint8_t kinds[2] = {REWRITE_KIND_CANCEL, REWRITE_KIND_DECOMPOSE};
    assert(transform_config_with_enabled_kinds(config, kinds, 2u) == 0);
    assert(transform_config_enabled_kinds_len(config) == 2u);
    uint8_t read_back[2] = {0};
    assert(transform_config_enabled_kinds(config, read_back, 2u) == 2u);
    assert(read_back[0] == REWRITE_KIND_CANCEL);
    assert(read_back[1] == REWRITE_KIND_DECOMPOSE);

    // Unknown kind tags are rejected and the config stays unchanged.
    const uint8_t bad_kinds[1] = {200};
    assert(transform_config_with_enabled_kinds(config, bad_kinds, 1u) == -8);
    assert(transform_config_enabled_kinds_len(config) == 2u);

    // An explicit standard-gate target basis is parsed by name.
    const char* const gates[2] = {"H", "CX"};
    assert(transform_config_with_target_instructions(config, gates, 2u) == 0);
    assert(transform_config_target_instruction_basis_len(config) == 2u);
    char* basis[2] = {NULL, NULL};
    assert(transform_config_target_instruction_basis(config, basis, 2u) == 2u);
    assert(basis[0] != NULL && strcmp(basis[0], "H") == 0);
    assert(basis[1] != NULL && strcmp(basis[1], "CX") == 0);
    cqlib_string_free(basis[0]);
    cqlib_string_free(basis[1]);

    // Unknown gate names map to -4 and leave the basis unchanged.
    const char* const bad_gates[2] = {"H", "NOPE"};
    assert(transform_config_with_target_instructions(config, bad_gates, 2u) == -4);
    assert(transform_config_target_instruction_basis_len(config) == 2u);

    // An empty basis is rejected (-8).
    assert(transform_config_with_target_instructions(config, NULL, 0) == -8);
    assert(transform_config_target_instruction_basis_len(config) == 2u);

    // NULL setters are rejected.
    assert(transform_config_with_max_rounds(NULL, 1) == -1);

    transform_config_free(config);
}

static void test_canonicalize_config(void) {
    struct CCanonicalizeConfig* config = canonicalize_config_default();
    assert(config != NULL);

    // Production defaults: round limit 8, every optional behavior enabled.
    assert(canonicalize_config_round_limit(config) == 8u);
    assert(canonicalize_config_recurse_control_flow(config) == 1);
    assert(canonicalize_config_fold_gphase(config) == 1);
    assert(canonicalize_config_canonicalize_instruction_form(config) == 1);
    assert(canonicalize_config_drop_noops(config) == 1);
    assert(canonicalize_config_canonicalize_barriers(config) == 1);

    // Setters mutate in place and report success.
    assert(canonicalize_config_with_round_limit(config, 2) == 0);
    assert(canonicalize_config_round_limit(config) == 2u);
    assert(canonicalize_config_with_recurse_control_flow(config, 0) == 0);
    assert(canonicalize_config_recurse_control_flow(config) == 0);
    assert(canonicalize_config_with_fold_gphase(config, 0) == 0);
    assert(canonicalize_config_fold_gphase(config) == 0);
    assert(canonicalize_config_with_canonicalize_instruction_form(config, 0) == 0);
    assert(canonicalize_config_canonicalize_instruction_form(config) == 0);
    assert(canonicalize_config_with_drop_noops(config, 0) == 0);
    assert(canonicalize_config_drop_noops(config) == 0);
    assert(canonicalize_config_with_canonicalize_barriers(config, 0) == 0);
    assert(canonicalize_config_canonicalize_barriers(config) == 0);

    // NULL handling.
    assert(canonicalize_config_with_round_limit(NULL, 1) == -1);
    assert(canonicalize_config_round_limit(NULL) == 0u);
    assert(canonicalize_config_recurse_control_flow(NULL) == -1);
    canonicalize_config_free(NULL);

    canonicalize_config_free(config);
}

static void test_canonicalize_circuit_with_config(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);
    assert(circuit_t(circuit, 1) == 0);

    struct CCanonicalizeConfig* config = canonicalize_config_default();
    assert(config != NULL);

    // Canonicalizing with the default config yields a valid 2-qubit circuit.
    struct CCanonicalizeResult* result = canonicalize_circuit_with_config(circuit, config);
    assert(result != NULL);
    CCircuit* canon = canonicalize_result_circuit(result);
    assert(canon != NULL);
    assert(circuit_num_qubits(canon) == 2u);
    assert(circuit_num_operations(canon) >= 1u);
    assert(canonicalize_result_changed(result) == 0 || canonicalize_result_changed(result) == 1);
    assert(canonicalize_result_rounds(result) <= canonicalize_config_round_limit(config));
    circuit_free(canon);
    canonicalize_result_free(result);

    // The config is not consumed and can be reused with a tighter limit.
    assert(canonicalize_config_with_round_limit(config, 1) == 0);
    struct CCanonicalizeResult* limited = canonicalize_circuit_with_config(circuit, config);
    assert(limited != NULL);
    CCircuit* limited_circuit = canonicalize_result_circuit(limited);
    assert(limited_circuit != NULL);
    assert(circuit_num_qubits(limited_circuit) == 2u);
    assert(canonicalize_result_rounds(limited) <= 1u);
    circuit_free(limited_circuit);
    canonicalize_result_free(limited);

    // NULL inputs are rejected.
    assert(canonicalize_circuit_with_config(NULL, config) == NULL);
    assert(canonicalize_circuit_with_config(circuit, NULL) == NULL);

    canonicalize_config_free(config);
    circuit_free(circuit);
}

static void test_decompose_unitary_target(void) {
    // Unconstrained target: no native gates, neutral Pauli fallback.
    struct CDecomposeUnitaryTarget* unconstrained = decompose_unitary_target_unconstrained();
    assert(unconstrained != NULL);
    assert(decompose_unitary_target_fallback_pauli(unconstrained) == 1);
    assert(decompose_unitary_target_native_1q_len(unconstrained) == 0u);
    assert(decompose_unitary_target_native_2q_len(unconstrained) == 0u);
    assert(decompose_unitary_target_basis_len(unconstrained) == 0u);

    // Rebuilding the fallback flag needs a non-empty native basis.
    assert(decompose_unitary_target_set_fallback_pauli(unconstrained, 0) == -8);
    assert(decompose_unitary_target_fallback_pauli(unconstrained) == 1);

    // An unconstrained target has no basis to cost against.
    const char* const ops[3] = {"H", "CX", "H"};
    const uint32_t qubit_ids[4] = {0, 0, 1, 1};
    const uint32_t op_counts[3] = {1, 2, 1};
    CTargetBasisCost cost;
    memset(&cost, 0, sizeof(cost));
    assert(decompose_unitary_target_cost_of_fixed_operations(unconstrained, ops, qubit_ids,
                                                             op_counts, 3u, &cost) == -8);

    // A workflow-style basis list splits native gates by arity.
    const char* const basis_gates[3] = {"H", "RZ", "CX"};
    struct CDecomposeUnitaryTarget* target =
        decompose_unitary_target_from_instructions(basis_gates, 3u);
    assert(target != NULL);
    assert(decompose_unitary_target_fallback_pauli(target) == 1);
    assert(decompose_unitary_target_native_1q_len(target) == 2u);
    assert(decompose_unitary_target_native_2q_len(target) == 1u);
    assert(decompose_unitary_target_basis_len(target) == 3u);

    // The combined basis contains every native gate (1q gates first).
    char* basis[3] = {NULL, NULL, NULL};
    assert(decompose_unitary_target_basis(target, basis, 3u) == 3u);
    sort_names(basis, 3u);
    assert(strcmp(basis[0], "CX") == 0);
    assert(strcmp(basis[1], "H") == 0);
    assert(strcmp(basis[2], "RZ") == 0);
    for (uintptr_t i = 0; i < 3u; i++) {
        cqlib_string_free(basis[i]);
    }

    // Empty and unknown basis lists are rejected.
    assert(decompose_unitary_target_from_instructions(NULL, 0) == NULL);
    const char* const unknown_gates[1] = {"NOT_A_GATE"};
    assert(decompose_unitary_target_from_instructions(unknown_gates, 1u) == NULL);

    // A native sequence lowers to itself: 3 ops, depth 3, one 2q op.
    memset(&cost, 0, sizeof(cost));
    assert(decompose_unitary_target_cost_of_fixed_operations(target, ops, qubit_ids, op_counts, 3u,
                                                             &cost) == 0);
    assert(cost.total_ops == 3u);
    assert(cost.two_qubit_ops == 1u);
    assert(cost.parameterized_ops == 0u);
    assert(cost.depth == 3u);

    // A parameterized gate is counted as such.
    const char* const rz_ops[1] = {"RZ"};
    const uint32_t rz_counts[1] = {1};
    assert(decompose_unitary_target_cost_of_fixed_operations(target, rz_ops, qubit_ids, rz_counts,
                                                             1u, &cost) == 0);
    assert(cost.total_ops == 1u);
    assert(cost.parameterized_ops == 1u);

    // Error paths: unknown gate name, NULL out, NULL target.
    const char* const bad_ops[1] = {"NOT_A_GATE"};
    assert(decompose_unitary_target_cost_of_fixed_operations(target, bad_ops, qubit_ids, rz_counts,
                                                             1u, &cost) == -4);
    assert(decompose_unitary_target_cost_of_fixed_operations(target, ops, qubit_ids, op_counts, 3u,
                                                             NULL) == -1);
    assert(decompose_unitary_target_cost_of_fixed_operations(NULL, ops, qubit_ids, op_counts, 3u,
                                                             &cost) == -1);

    // Toggling the fallback rebuilds the target around its native lists.
    assert(decompose_unitary_target_set_fallback_pauli(target, 0) == 0);
    assert(decompose_unitary_target_fallback_pauli(target) == 0);
    assert(decompose_unitary_target_native_1q_len(target) == 2u);
    assert(decompose_unitary_target_set_fallback_pauli(target, 1) == 0);
    assert(decompose_unitary_target_fallback_pauli(target) == 1);
    assert(decompose_unitary_target_set_fallback_pauli(NULL, 1) == -1);

    // Explicit standard-gate lists with a wrong-arity or unknown gate fail.
    const char* const one_qubit[2] = {"H", "RZ"};
    const char* const two_qubit[2] = {"CX", "RZZ"};
    struct CDecomposeUnitaryTarget* explicit_target =
        decompose_unitary_target_from_standard_gates(one_qubit, 2u, two_qubit, 2u, 1);
    assert(explicit_target != NULL);
    assert(decompose_unitary_target_fallback_pauli(explicit_target) == 1);
    assert(decompose_unitary_target_native_1q_len(explicit_target) == 2u);
    assert(decompose_unitary_target_native_2q_len(explicit_target) == 2u);
    assert(decompose_unitary_target_basis_len(explicit_target) == 4u);
    decompose_unitary_target_free(explicit_target);
    assert(decompose_unitary_target_from_standard_gates(two_qubit, 2u, two_qubit, 2u, 1) == NULL);
    assert(decompose_unitary_target_from_standard_gates(unknown_gates, 1u, two_qubit, 2u, 1) ==
           NULL);

    decompose_unitary_target_free(target);
    decompose_unitary_target_free(unconstrained);
    decompose_unitary_target_free(NULL);
}

int main(void) {
    test_transform_config_defaults();
    test_transform_config_setters();
    test_canonicalize_config();
    test_canonicalize_circuit_with_config();
    test_decompose_unitary_target();
    printf("binding-c compile transform config tests passed\n");
    return 0;
}
