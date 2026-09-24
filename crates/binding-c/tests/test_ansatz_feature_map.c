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

/* Builds a Hamiltonian from a single Pauli word (coefficient 1.0).
 * `hamiltonian_from_pauli` takes ownership of the pauli handle, so the
 * caller must free only the returned Hamiltonian, never the pauli
 * handle once this helper returns. */
static struct CHamiltonian* pauli_hamiltonian(const char* text) {
    struct CPauliString* pauli = pauli_string_parse(text);
    if (pauli == NULL) {
        return NULL;
    }
    return hamiltonian_from_pauli(pauli);
}

static void test_z_feature_map(void) {
    struct CZFeatureMap* map = z_feature_map_new(3);
    assert(map != NULL);
    assert(z_feature_map_num_qubits(map) == 3);

    /* Defaults: reps = 2, one feature parameter per qubit. */
    assert(z_feature_map_num_parameters(map) == 3);
    assert(z_feature_map_validate(map) == 0);

    /* reps = 0 drops all parameters; restoring it brings them back. */
    assert(z_feature_map_reps(map, 0) == 0);
    assert(z_feature_map_num_parameters(map) == 0);
    assert(z_feature_map_reps(map, 2) == 0);
    assert(z_feature_map_num_parameters(map) == 3);

    /* Each layer is a Hadamard plus one RZ per qubit: 2 * (3 + 3) = 12 ops
     * over the 3 unique feature parameters. */
    struct CCircuit* circuit = z_feature_map_build_circuit(map, "z");
    assert(circuit != NULL);
    assert(circuit_num_qubits(circuit) == 3);
    assert(circuit_num_parameters(circuit) == 3);
    assert(circuit_num_operations(circuit) == 2 * (3 + 3));
    circuit_free(circuit);

    /* NULL and non-UTF-8 prefixes are rejected. */
    assert(z_feature_map_build_circuit(map, NULL) == NULL);
    const char invalid_utf8[2] = {(char)0xFF, '\0'};
    assert(z_feature_map_build_circuit(map, invalid_utf8) == NULL);
    z_feature_map_free(map);
}

static void test_zz_feature_map(void) {
    struct CZZFeatureMap* map = zz_feature_map_new(3);
    assert(map != NULL);
    assert(zz_feature_map_num_qubits(map) == 3);

    /* Defaults: reps = 2, full entanglement; one feature parameter per
     * qubit regardless of the interaction layer count. */
    assert(zz_feature_map_num_parameters(map) == 3);
    assert(zz_feature_map_validate(map) == 0);

    /* Topologies are accepted; unknown tags and malformed pair lists are
     * rejected with distinct error codes. */
    assert(zz_feature_map_entanglement(map, ENTANGLEMENT_LINEAR, NULL, 0) == 0);
    assert(zz_feature_map_entanglement(map, ENTANGLEMENT_CIRCULAR, NULL, 0) == 0);
    const uint32_t pairs[4] = {0, 1, 1, 2};
    assert(zz_feature_map_entanglement(map, ENTANGLEMENT_CUSTOM, pairs, 4) == 0);
    assert(zz_feature_map_validate(map) == 0);
    assert(zz_feature_map_entanglement(map, 99, NULL, 0) == -8);
    assert(zz_feature_map_entanglement(map, ENTANGLEMENT_CUSTOM, pairs, 3) == -8);
    assert(zz_feature_map_entanglement(map, ENTANGLEMENT_CUSTOM, NULL, 2) == -1);

    /* reps = 0 drops all parameters. */
    assert(zz_feature_map_reps(map, 0) == 0);
    assert(zz_feature_map_num_parameters(map) == 0);
    assert(zz_feature_map_reps(map, 1) == 0);

    /* Restore full entanglement and build. The ZZ interactions lower to
     * Pauli evolution gates, so only the H + RZ floor is pinned. Each
     * gate interns its whole angle expression, so the circuit exposes
     * 3 RZ expressions + 3 evolution expressions (pi is nested inside
     * them, never interned on its own). */
    assert(zz_feature_map_entanglement(map, ENTANGLEMENT_FULL, NULL, 0) == 0);
    struct CCircuit* circuit = zz_feature_map_build_circuit(map, "x");
    assert(circuit != NULL);
    assert(circuit_num_qubits(circuit) == 3);
    assert(circuit_num_parameters(circuit) == 6);
    assert(circuit_num_operations(circuit) >= 3 + 3);
    circuit_free(circuit);
    zz_feature_map_free(map);
}

static void test_iqp_feature_map(void) {
    struct CIQPFeatureMap* map = iqp_feature_map_new(2);
    assert(map != NULL);
    assert(iqp_feature_map_num_qubits(map) == 2);
    assert(iqp_feature_map_num_parameters(map) == 2);
    assert(iqp_feature_map_validate(map) == 0);

    assert(iqp_feature_map_reps(map, 1) == 0);
    assert(iqp_feature_map_num_parameters(map) == 2);

    /* A custom pair list constrains the two-qubit diagonal interactions.
     * The built circuit interns one angle expression per gate: 2 RZ + 1
     * evolution expression. */
    const uint32_t pairs[2] = {0, 1};
    assert(iqp_feature_map_entanglement(map, ENTANGLEMENT_CUSTOM, pairs, 2) == 0);
    struct CCircuit* circuit = iqp_feature_map_build_circuit(map, "x");
    assert(circuit != NULL);
    assert(circuit_num_qubits(circuit) == 2);
    assert(circuit_num_parameters(circuit) == 3);
    circuit_free(circuit);
    iqp_feature_map_free(map);
}

static void test_pauli_feature_map(void) {
    struct CPauliFeatureMap* map = pauli_feature_map_new(3);
    assert(map != NULL);
    assert(pauli_feature_map_num_qubits(map) == 3);

    /* Defaults: reps = 2, paulis ["Z", "ZZ"], prefix "x". */
    assert(pauli_feature_map_num_parameters(map) == 3);
    assert(pauli_feature_map_validate(map) == 0);

    /* Parallel text/label arrays configure the Pauli templates. */
    const char* texts[2] = {"X", "XX"};
    const char* labels[2] = {"x", "xx"};
    assert(pauli_feature_map_paulis(map, texts, labels, 2) == 0);
    assert(pauli_feature_map_validate(map) == 0);

    /* Unparsable Pauli words, NULL entries and invalid UTF-8 are rejected
     * without clobbering the current configuration. */
    const char* bad_texts[1] = {"QZ"};
    const char* bad_labels[1] = {"qz"};
    assert(pauli_feature_map_paulis(map, bad_texts, bad_labels, 1) == -8);
    const char* null_texts[1] = {NULL};
    assert(pauli_feature_map_paulis(map, null_texts, labels, 1) == -1);
    const char invalid_utf8[2] = {(char)0xFF, '\0'};
    const char* utf8_labels[1] = {invalid_utf8};
    assert(pauli_feature_map_paulis(map, texts, utf8_labels, 1) == -4);
    assert(pauli_feature_map_validate(map) == 0);

    /* The parameter prefix is a fallback for empty build prefixes. */
    assert(pauli_feature_map_parameter_prefix(map, "f") == 0);
    assert(pauli_feature_map_parameter_prefix(map, invalid_utf8) == -4);
    assert(pauli_feature_map_parameter_prefix(map, NULL) == -1);

    /* An empty prefix falls back to the configured parameter prefix.
     * Angle expressions are interned whole: 3 single-qubit + 3 pair
     * expressions for the ["X", "XX"] templates on 3 qubits. */
    struct CCircuit* circuit = pauli_feature_map_build_circuit(map, "");
    assert(circuit != NULL);
    assert(circuit_num_qubits(circuit) == 3);
    assert(circuit_num_parameters(circuit) == 6);
    circuit_free(circuit);
    assert(pauli_feature_map_build_circuit(map, NULL) == NULL);
    pauli_feature_map_free(map);
}

static void test_feature_map_facades(void) {
    /* zz_feature_map_build mirrors the new + setters chain. */
    struct CZZFeatureMap* zz = zz_feature_map_build(3, 1, ENTANGLEMENT_CIRCULAR, NULL, 0);
    assert(zz != NULL);
    assert(zz_feature_map_num_qubits(zz) == 3);
    assert(zz_feature_map_num_parameters(zz) == 3);
    assert(zz_feature_map_validate(zz) == 0);
    zz_feature_map_free(zz);

    const uint32_t pairs[2] = {0, 2};
    struct CZZFeatureMap* custom = zz_feature_map_build(3, 1, ENTANGLEMENT_CUSTOM, pairs, 2);
    assert(custom != NULL);
    assert(zz_feature_map_validate(custom) == 0);
    zz_feature_map_free(custom);
    assert(zz_feature_map_build(3, 1, 99, NULL, 0) == NULL);

    /* pauli_feature_map_build takes explicit Pauli templates. The ZZ
     * angles add the symbolic pi constant to the 3 features. */
    const char* texts[2] = {"Z", "ZZ"};
    const char* labels[2] = {"z", "zz"};
    struct CPauliFeatureMap* pf =
        pauli_feature_map_build(3, 1, texts, labels, 2, ENTANGLEMENT_FULL, NULL, 0);
    assert(pf != NULL);
    assert(pauli_feature_map_num_qubits(pf) == 3);
    assert(pauli_feature_map_num_parameters(pf) == 3);
    assert(pauli_feature_map_validate(pf) == 0);
    pauli_feature_map_free(pf);
    /* Unparsable Pauli words and unknown topologies fail construction. */
    const char* bad_texts[1] = {"QZ"};
    const char* bad_labels[1] = {"qz"};
    assert(pauli_feature_map_build(3, 1, bad_texts, bad_labels, 1, ENTANGLEMENT_FULL, NULL, 0) ==
           NULL);
    assert(pauli_feature_map_build(3, 1, texts, labels, 2, 99, NULL, 0) == NULL);
}

static void test_ansatz_facades(void) {
    /* EfficientSU2: [RY, RZ] rotations, CX entanglement, full topology.
     * 3 rotation layers * 3 qubits * 2 gates + 2 entanglement layers * 3
     * CX = 24 operations over (2 + 1) * 3 * 2 = 18 parameters. */
    struct CTwoLocal* su2 = efficient_su2(3, 2, ENTANGLEMENT_FULL, NULL, 0);
    assert(su2 != NULL);
    assert(two_local_num_qubits(su2) == 3);
    assert(two_local_num_parameters(su2) == 18);
    assert(two_local_validate(su2) == 0);
    struct CCircuit* circuit = two_local_build_circuit(su2, "p");
    assert(circuit != NULL);
    assert(circuit_num_qubits(circuit) == 3);
    assert(circuit_num_operations(circuit) == 3 * 3 * 2 + 2 * 3);
    assert(circuit_num_parameters(circuit) == 18);
    circuit_free(circuit);
    two_local_free(su2);

    /* RealAmplitudes: [RY] rotations, linear topology gives 2 CX per
     * entanglement layer: 9 rotations + 4 CX = 13 operations. */
    struct CTwoLocal* ra = real_amplitudes(3, 2, ENTANGLEMENT_LINEAR, NULL, 0);
    assert(ra != NULL);
    assert(two_local_num_parameters(ra) == 9);
    struct CCircuit* ra_circuit = two_local_build_circuit(ra, "t");
    assert(ra_circuit != NULL);
    assert(circuit_num_qubits(ra_circuit) == 3);
    assert(circuit_num_operations(ra_circuit) == 13);
    assert(circuit_num_parameters(ra_circuit) == 9);
    circuit_free(ra_circuit);
    two_local_free(ra);

    /* Invalid topologies are rejected at construction time. */
    assert(efficient_su2(3, 2, 99, NULL, 0) == NULL);
    const uint32_t pairs[3] = {0, 1, 1};
    assert(real_amplitudes(3, 2, ENTANGLEMENT_CUSTOM, pairs, 3) == NULL);
}

static void test_entanglement_helpers(void) {
    uint32_t out[12];

    /* Linear pairs on 4 qubits: (0,1), (1,2), (2,3). */
    uintptr_t len = entanglement_generate_pairs_len(ENTANGLEMENT_LINEAR, NULL, 0, 4);
    assert(len == 6);
    assert(entanglement_generate_pairs(ENTANGLEMENT_LINEAR, NULL, 0, 4, out, len) == 0);
    assert(out[0] == 0 && out[1] == 1 && out[2] == 1 && out[3] == 2);
    assert(out[4] == 2 && out[5] == 3);

    /* Full pairs run the upper triangle: (0,1), (0,2), (1,2). */
    len = entanglement_generate_pairs_len(ENTANGLEMENT_FULL, NULL, 0, 3);
    assert(len == 6);
    assert(entanglement_generate_pairs(ENTANGLEMENT_FULL, NULL, 0, 3, out, len) == 0);
    assert(out[0] == 0 && out[1] == 1 && out[2] == 0 && out[3] == 2);
    assert(out[4] == 1 && out[5] == 2);

    /* Circular wraps the ring: (0,1), (1,2), (2,3), (3,0). */
    len = entanglement_generate_pairs_len(ENTANGLEMENT_CIRCULAR, NULL, 0, 4);
    assert(len == 8);
    assert(entanglement_generate_pairs(ENTANGLEMENT_CIRCULAR, NULL, 0, 4, out, len) == 0);
    assert(out[6] == 3 && out[7] == 0);

    /* Custom pairs are echoed verbatim. */
    const uint32_t custom[4] = {2, 0, 1, 2};
    len = entanglement_generate_pairs_len(ENTANGLEMENT_CUSTOM, custom, 4, 3);
    assert(len == 4);
    uint32_t echo[4];
    assert(entanglement_generate_pairs(ENTANGLEMENT_CUSTOM, custom, 4, 3, echo, 4) == 0);
    assert(echo[0] == 2 && echo[1] == 0 && echo[2] == 1 && echo[3] == 2);

    /* Unknown topologies report length 0; a mismatched buffer length is
     * rejected with -8. */
    assert(entanglement_generate_pairs_len(99, NULL, 0, 3) == 0);
    assert(entanglement_generate_pairs(ENTANGLEMENT_FULL, NULL, 0, 3, out, 4) == -8);

    /* Full triples of 4 qubits: (0,1,2), (0,1,3), (0,2,3), (1,2,3). */
    len = entanglement_generate_k_tuples_len(ENTANGLEMENT_FULL, NULL, 0, 3, 4);
    assert(len == 12);
    assert(entanglement_generate_k_tuples(ENTANGLEMENT_FULL, NULL, 0, 3, 4, out, len) == 0);
    assert(out[0] == 0 && out[1] == 1 && out[2] == 2);
    assert(out[9] == 1 && out[10] == 2 && out[11] == 3);

    /* Linear triples slide a window: (0,1,2), (1,2,3). */
    len = entanglement_generate_k_tuples_len(ENTANGLEMENT_LINEAR, NULL, 0, 3, 4);
    assert(len == 6);
    assert(entanglement_generate_k_tuples(ENTANGLEMENT_LINEAR, NULL, 0, 3, 4, out, len) == 0);
    assert(out[0] == 0 && out[1] == 1 && out[2] == 2);
    assert(out[3] == 1 && out[4] == 2 && out[5] == 3);

    /* Degenerate k values and mismatched lengths are rejected. */
    assert(entanglement_generate_k_tuples_len(ENTANGLEMENT_FULL, NULL, 0, 0, 4) == 0);
    assert(entanglement_generate_k_tuples_len(ENTANGLEMENT_FULL, NULL, 0, 5, 4) == 0);
    assert(entanglement_generate_k_tuples(ENTANGLEMENT_FULL, NULL, 0, 0, 4, NULL, 0) == 0);
    assert(entanglement_generate_k_tuples(ENTANGLEMENT_FULL, NULL, 0, 3, 4, out, 6) == -8);
}

static void test_strongly_entangling_layers(void) {
    struct CStronglyEntanglingLayers* layers = strongly_entangling_layers_new(3);
    assert(layers != NULL);
    assert(strongly_entangling_layers_num_qubits(layers) == 3);

    /* Defaults: reps = 1 with a 3-parameter U gate per qubit. */
    assert(strongly_entangling_layers_num_parameters(layers) == 9);
    assert(strongly_entangling_layers_validate(layers) == 0);
    assert(strongly_entangling_layers_reps(layers, 2) == 0);
    assert(strongly_entangling_layers_num_parameters(layers) == 18);

    /* Gate selection tolerates CX/CY/CZ and rejects everything else. */
    assert(strongly_entangling_layers_entanglement_gate(layers, "CZ") == 0);
    assert(strongly_entangling_layers_entanglement_gate(layers, "NOT_A_GATE") == -8);
    const char invalid_utf8[2] = {(char)0xFF, '\0'};
    assert(strongly_entangling_layers_entanglement_gate(layers, invalid_utf8) == -4);
    assert(strongly_entangling_layers_entanglement_gate(layers, NULL) == -1);

    /* Ranges cycle across layers; an empty list fails validation until
     * replaced, and a range must stay below num_qubits. */
    uintptr_t ranges[2] = {2, 1};
    assert(strongly_entangling_layers_ranges(layers, ranges, 2) == 0);
    assert(strongly_entangling_layers_validate(layers) == 0);
    assert(strongly_entangling_layers_ranges(layers, NULL, 0) == 0);
    assert(strongly_entangling_layers_validate(layers) == -3);
    uintptr_t wide[1] = {3};
    assert(strongly_entangling_layers_ranges(layers, wide, 1) == 0);
    assert(strongly_entangling_layers_validate(layers) == -3);
    uintptr_t in_range[1] = {2};
    assert(strongly_entangling_layers_ranges(layers, in_range, 1) == 0);
    assert(strongly_entangling_layers_validate(layers) == 0);

    /* Two layers: 3 U gates + 3 entanglers per layer = 12 operations
     * over 18 parameters. */
    struct CCircuit* circuit = strongly_entangling_layers_build_circuit(layers, "u");
    assert(circuit != NULL);
    assert(circuit_num_qubits(circuit) == 3);
    assert(circuit_num_parameters(circuit) == 18);
    assert(circuit_num_operations(circuit) == 2 * (3 + 3));
    circuit_free(circuit);
    strongly_entangling_layers_free(layers);
}

static void test_pauli_evolution_ansatz(void) {
    struct CHamiltonian* cost = pauli_hamiltonian("ZZ");
    assert(cost != NULL);
    struct CPauliEvolutionAnsatz* ansatz = pauli_evolution_ansatz_new(cost);
    assert(ansatz != NULL);

    /* The time-parameter symbol can be overridden and reset with an
     * empty name; invalid UTF-8 and NULL handles are rejected. */
    assert(pauli_evolution_ansatz_with_time_param_name(ansatz, "t") == 0);
    assert(pauli_evolution_ansatz_with_time_param_name(ansatz, "") == 0);
    const char invalid_utf8[2] = {(char)0xFF, '\0'};
    assert(pauli_evolution_ansatz_with_time_param_name(ansatz, invalid_utf8) == -4);
    assert(pauli_evolution_ansatz_with_time_param_name(ansatz, NULL) == -1);
    pauli_evolution_ansatz_free(ansatz);

    /* NULL and empty operators are rejected at construction time. */
    assert(pauli_evolution_ansatz_new(NULL) == NULL);
    struct CHamiltonian* empty = hamiltonian_new(2);
    assert(empty != NULL);
    assert(pauli_evolution_ansatz_new(empty) == NULL);
    hamiltonian_free(empty);

    /* The constructor clones its input, so the Hamiltonian stays owned
     * by this test. Freeing NULL is a no-op. */
    hamiltonian_free(cost);
    pauli_evolution_ansatz_free(NULL);
}

static void test_feature_map_null_handles(void) {
    /* Z / ZZ / IQP / Pauli feature maps. */
    assert(z_feature_map_reps(NULL, 1) == -1);
    assert(z_feature_map_validate(NULL) == -1);
    assert(z_feature_map_num_parameters(NULL) == 0);
    assert(z_feature_map_num_qubits(NULL) == 0);
    assert(z_feature_map_build_circuit(NULL, "z") == NULL);
    assert(zz_feature_map_reps(NULL, 1) == -1);
    assert(zz_feature_map_entanglement(NULL, ENTANGLEMENT_LINEAR, NULL, 0) == -1);
    assert(zz_feature_map_validate(NULL) == -1);
    assert(zz_feature_map_num_parameters(NULL) == 0);
    assert(zz_feature_map_num_qubits(NULL) == 0);
    assert(zz_feature_map_build_circuit(NULL, "x") == NULL);
    assert(iqp_feature_map_reps(NULL, 1) == -1);
    assert(iqp_feature_map_entanglement(NULL, ENTANGLEMENT_FULL, NULL, 0) == -1);
    assert(iqp_feature_map_validate(NULL) == -1);
    assert(iqp_feature_map_num_parameters(NULL) == 0);
    assert(iqp_feature_map_num_qubits(NULL) == 0);
    assert(iqp_feature_map_build_circuit(NULL, "x") == NULL);
    assert(pauli_feature_map_reps(NULL, 1) == -1);
    assert(pauli_feature_map_paulis(NULL, NULL, NULL, 0) == -1);
    assert(pauli_feature_map_entanglement(NULL, ENTANGLEMENT_FULL, NULL, 0) == -1);
    assert(pauli_feature_map_parameter_prefix(NULL, "x") == -1);
    assert(pauli_feature_map_validate(NULL) == -1);
    assert(pauli_feature_map_num_parameters(NULL) == 0);
    assert(pauli_feature_map_num_qubits(NULL) == 0);
    assert(pauli_feature_map_build_circuit(NULL, "x") == NULL);

    /* Entanglement helpers. */
    assert(entanglement_generate_pairs_len(ENTANGLEMENT_FULL, NULL, 0, 3) > 0);
    assert(entanglement_generate_pairs(ENTANGLEMENT_FULL, NULL, 0, 3, NULL, 6) == -1);

    /* StronglyEntanglingLayers. */
    assert(strongly_entangling_layers_reps(NULL, 1) == -1);
    assert(strongly_entangling_layers_entanglement_gate(NULL, "CX") == -1);
    assert(strongly_entangling_layers_ranges(NULL, NULL, 0) == -1);
    assert(strongly_entangling_layers_validate(NULL) == -1);
    assert(strongly_entangling_layers_num_parameters(NULL) == 0);
    assert(strongly_entangling_layers_num_qubits(NULL) == 0);
    assert(strongly_entangling_layers_build_circuit(NULL, "u") == NULL);

    /* PauliEvolutionAnsatz. */
    assert(pauli_evolution_ansatz_with_time_param_name(NULL, "t") == -1);

    /* Freeing NULL is a no-op everywhere. */
    z_feature_map_free(NULL);
    zz_feature_map_free(NULL);
    iqp_feature_map_free(NULL);
    pauli_feature_map_free(NULL);
    strongly_entangling_layers_free(NULL);
    pauli_evolution_ansatz_free(NULL);
}

int main(void) {
    test_z_feature_map();
    test_zz_feature_map();
    test_iqp_feature_map();
    test_pauli_feature_map();
    test_feature_map_facades();
    test_ansatz_facades();
    test_entanglement_helpers();
    test_strongly_entangling_layers();
    test_pauli_evolution_ansatz();
    test_feature_map_null_handles();
    printf("binding-c ansatz feature map tests passed\n");
    return 0;
}
