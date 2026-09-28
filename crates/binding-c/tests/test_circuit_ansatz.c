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

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

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

static void test_two_local_structure(void) {
    struct CTwoLocal* ansatz = two_local_new(3);
    CHECK(ansatz != NULL, "two_local_new");
    CHECK(two_local_num_qubits(ansatz) == 3, "two_local_num_qubits");
    CHECK(two_local_validate(ansatz) == 0, "two_local_validate default");

    /* Defaults: reps = 3, RY rotations, final rotation layer kept =>
     * (3 + 1) layers * 3 qubits * 1 gate = 12 parameters. */
    CHECK(two_local_num_parameters(ansatz) == 12, "num_parameters reps=3");

    /* reps = 2 => (2 + 1) * 3 * 1 = 9. */
    CHECK(two_local_reps(ansatz, 2) == 0, "two_local_reps");
    CHECK(two_local_num_parameters(ansatz) == 9, "num_parameters reps=2");

    /* Skipping the final rotation layer leaves exactly `reps` layers. */
    CHECK(two_local_skip_final_rotation_layer(ansatz, true) == 0,
          "two_local_skip_final_rotation_layer");
    CHECK(two_local_num_parameters(ansatz) == 2 * 3, "num_parameters skip final");

    /* The entangler choice never changes the parameter count. */
    CHECK(two_local_entanglement_gate(ansatz, "CZ") == 0, "two_local_entanglement_gate CZ");
    CHECK(two_local_num_parameters(ansatz) == 6, "CZ keeps parameter count");
    CHECK(two_local_validate(ansatz) == 0, "two_local_validate CZ");

    /* Two rotation gates per qubit: 2 layers * 3 qubits * 2 gates = 12. */
    const char* names[2] = {"RY", "RZ"};
    CHECK(two_local_rotation_gates(ansatz, names, 2) == 0, "two_local_rotation_gates RY RZ");
    CHECK(two_local_num_parameters(ansatz) == 12, "dual rotation gates double parameters");

    /* Topologies are accepted and orthogonal to the parameter count. */
    CHECK(two_local_entanglement(ansatz, ENTANGLEMENT_CIRCULAR, NULL, 0) == 0,
          "entanglement circular");
    CHECK(two_local_entanglement(ansatz, ENTANGLEMENT_FULL, NULL, 0) == 0, "entanglement full");
    CHECK(two_local_entanglement(ansatz, ENTANGLEMENT_LINEAR, NULL, 0) == 0, "entanglement linear");
    const uint32_t pairs[4] = {1, 2, 0, 2};
    CHECK(two_local_entanglement(ansatz, ENTANGLEMENT_CUSTOM, pairs, 4) == 0,
          "entanglement custom");
    CHECK(two_local_num_parameters(ansatz) == 12, "topologies keep parameter count");
    CHECK(two_local_validate(ansatz) == 0, "two_local_validate final");

    two_local_free(ansatz);
}

static void test_two_local_invalid_inputs(void) {
    /* Gate-name parsing: unknown names, case and invalid UTF-8. */
    struct CTwoLocal* ansatz = two_local_new(3);
    CHECK(ansatz != NULL, "invalid-input fixture");
    CHECK(two_local_entanglement_gate(ansatz, "NOT_A_GATE") == -8,
          "entanglement_gate unknown name");
    CHECK(two_local_entanglement_gate(ansatz, "cx") == -8, "entanglement_gate lowercase name");
    const char invalid_utf8[3] = {(char)0xFF, (char)0xFF, '\0'};
    CHECK(two_local_entanglement_gate(ansatz, invalid_utf8) == -4,
          "entanglement_gate invalid utf-8");
    CHECK(two_local_entanglement_gate(ansatz, NULL) == -1, "entanglement_gate null name");

    /* A known single-qubit gate parses fine, but as an entangler it
     * fails validate and build (the gate must be CX, CY or CZ). */
    CHECK(two_local_entanglement_gate(ansatz, "H") == 0, "entanglement_gate H accepted");
    CHECK(two_local_validate(ansatz) == -3, "H entangler validate");
    CHECK(two_local_build_circuit(ansatz, "t") == NULL, "H entangler build fails");
    two_local_free(ansatz);

    /* Zero-qubit templates are created but never valid. */
    struct CTwoLocal* zero = two_local_new(0);
    CHECK(zero != NULL, "two_local_new(0)");
    CHECK(two_local_num_qubits(zero) == 0, "zero num_qubits");
    CHECK(two_local_validate(zero) == -3, "zero validate");
    CHECK(two_local_build_circuit(zero, "t") == NULL, "zero build fails");
    two_local_free(zero);

    /* reps = 0 is legal: exactly one rotation layer remains. */
    struct CTwoLocal* sparse = two_local_new(3);
    CHECK(two_local_reps(sparse, 0) == 0, "reps zero");
    CHECK(two_local_num_parameters(sparse) == 3, "reps zero params");
    CHECK(two_local_validate(sparse) == 0, "reps zero valid");
    /* Skipping the final layer removes those last parameters. */
    CHECK(two_local_skip_final_rotation_layer(sparse, true) == 0, "reps zero skip final");
    CHECK(two_local_num_parameters(sparse) == 0, "reps zero skip params");
    two_local_free(sparse);

    /* rotation_gates argument validation. */
    struct CTwoLocal* setter = two_local_new(2);
    CHECK(setter != NULL, "setter fixture");
    CHECK(two_local_rotation_gates(setter, NULL, 1) == -1, "rotation_gates null buffer");
    const char* null_names[1] = {NULL};
    CHECK(two_local_rotation_gates(setter, null_names, 1) == -1, "rotation_gates null entry");
    /* SWAP parses as a standard gate but is rejected by validate as a
     * rotation gate (must be RX, RY or RZ). */
    const char* swap_names[1] = {"SWAP"};
    CHECK(two_local_rotation_gates(setter, swap_names, 1) == 0, "rotation_gates SWAP accepted");
    CHECK(two_local_validate(setter) == -3, "SWAP rotation validate");
    two_local_free(setter);

    /* Topology argument validation. */
    struct CTwoLocal* topo = two_local_new(3);
    CHECK(two_local_entanglement(topo, 99, NULL, 0) == -8, "entanglement unknown tag");
    const uint32_t pairs[2] = {0, 1};
    CHECK(two_local_entanglement(topo, ENTANGLEMENT_CUSTOM, pairs, 3) == -8,
          "entanglement odd pairs_len");
    CHECK(two_local_entanglement(topo, ENTANGLEMENT_CUSTOM, NULL, 2) == -1,
          "entanglement null pairs");
    /* A self-loop pair is stored but fails validate. */
    const uint32_t self_loop[2] = {1, 1};
    CHECK(two_local_entanglement(topo, ENTANGLEMENT_CUSTOM, self_loop, 2) == 0,
          "entanglement self-loop stored");
    CHECK(two_local_validate(topo) == -3, "self-loop validate");
    two_local_free(topo);

    /* build_circuit prefix validation. */
    struct CTwoLocal* builder = two_local_new(2);
    CHECK(two_local_build_circuit(builder, NULL) == NULL, "build null prefix");
    CHECK(two_local_build_circuit(builder, invalid_utf8) == NULL, "build invalid utf-8 prefix");
    two_local_free(builder);
}

static void test_basic_entangler_layers_structure(void) {
    struct CBasicEntanglerLayers* layers = basic_entangler_layers_new(3);
    CHECK(layers != NULL, "basic_entangler_layers_new");
    CHECK(basic_entangler_layers_num_qubits(layers) == 3, "basic_entangler_layers_num_qubits");

    /* Defaults: reps = 1, one rotation per qubit per layer => 3 params
     * (parameter count = reps * num_qubits). */
    CHECK(basic_entangler_layers_num_parameters(layers) == 3, "num_parameters reps=1");

    /* reps = 2 => 2 * 3 = 6. */
    CHECK(basic_entangler_layers_reps(layers, 2) == 0, "basic_entangler_layers_reps");
    CHECK(basic_entangler_layers_num_parameters(layers) == 6, "num_parameters reps=2");

    /* Gate swaps keep the one-parameter-per-qubit-per-layer count. */
    CHECK(basic_entangler_layers_rotation_gate(layers, "RY") == 0, "rotation_gate RY");
    CHECK(basic_entangler_layers_entanglement_gate(layers, "CZ") == 0, "entanglement_gate CZ");
    CHECK(basic_entangler_layers_num_parameters(layers) == 6, "gate swaps keep parameter count");

    /* Bad gate names: unknown, non-UTF-8 and NULL handles. */
    CHECK(basic_entangler_layers_rotation_gate(layers, "NOT_A_GATE") == -8,
          "rotation_gate unknown name");
    const char invalid_utf8[2] = {(char)0xFF, '\0'};
    CHECK(basic_entangler_layers_entanglement_gate(layers, invalid_utf8) == -4,
          "entanglement_gate invalid utf-8");
    CHECK(basic_entangler_layers_rotation_gate(NULL, "RY") == -1, "rotation_gate null handle");
    CHECK(basic_entangler_layers_entanglement_gate(NULL, "CZ") == -1,
          "entanglement_gate null handle");
    /* The failed setters leave the template configured. */
    CHECK(basic_entangler_layers_num_parameters(layers) == 6,
          "failed setters keep parameter count");

    basic_entangler_layers_free(layers);
}

static void test_qaoa_ansatz_structure(void) {
    /* Cost Hamiltonian: a single ZZ term on 2 qubits. */
    struct CHamiltonian* cost = pauli_hamiltonian("ZZ");
    CHECK(cost != NULL, "qaoa cost fixture");
    struct CQAOAAnsatz* ansatz = qaoa_ansatz_new(cost);
    CHECK(ansatz != NULL, "qaoa_ansatz_new");

    /* Qubit count is inferred from the cost operator; default reps = 1
     * gives one gamma and one beta angle. */
    CHECK(qaoa_ansatz_num_qubits(ansatz) == 2, "qaoa_ansatz_num_qubits");
    CHECK(qaoa_ansatz_num_parameters(ansatz) == 2, "qaoa_ansatz_num_parameters");
    CHECK(qaoa_ansatz_validate(ansatz) == 0, "qaoa_ansatz_validate");

    /* reps = 2 doubles the parameter count. */
    CHECK(qaoa_ansatz_reps(ansatz, 2) == 0, "qaoa_ansatz_reps");
    CHECK(qaoa_ansatz_num_parameters(ansatz) == 4, "num_parameters reps=2");

    /* A same-width mixer override succeeds and stays valid. The mixer is
     * cloned, so the handle stays owned by this test (freed below). */
    struct CHamiltonian* mixer = pauli_hamiltonian("XX");
    CHECK(mixer != NULL, "qaoa mixer fixture");
    CHECK(qaoa_ansatz_mixer(ansatz, mixer) == 0, "qaoa_ansatz_mixer XX");
    CHECK(qaoa_ansatz_validate(ansatz) == 0, "XX mixer valid");
    CHECK(qaoa_ansatz_num_parameters(ansatz) == 4, "XX mixer parameter count");

    /* Mixers on a different qubit count are rejected. */
    struct CHamiltonian* wide = pauli_hamiltonian("XXX");
    CHECK(wide != NULL, "qaoa wide mixer fixture");
    CHECK(qaoa_ansatz_mixer(ansatz, wide) == -3, "qaoa_ansatz_mixer XXX");
    CHECK(qaoa_ansatz_mixer(ansatz, NULL) == -1, "qaoa_ansatz_mixer null");

    /* Initial-state circuits must match the qubit count. The circuit is
     * cloned, so each one is freed here as usual. */
    struct CCircuit* init = circuit_new(2);
    CHECK(init != NULL, "qaoa init fixture");
    CHECK(qaoa_ansatz_initial_state(ansatz, init) == 0, "qaoa_ansatz_initial_state");
    circuit_free(init);
    struct CCircuit* big = circuit_new(3);
    CHECK(big != NULL, "qaoa big init fixture");
    CHECK(qaoa_ansatz_initial_state(ansatz, big) == -3, "initial_state qubit mismatch");
    circuit_free(big);
    CHECK(qaoa_ansatz_initial_state(ansatz, NULL) == -1, "initial_state null");

    /* Evolution strategy selection. ZZ and XX are single terms, so the
     * exact decomposition is valid; Trotter accepts any Hermitian
     * input. */
    CHECK(qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_AUTO, TROTTER_FIRST_ORDER, 1,
                                         0) == 0,
          "evolution strategy auto");
    CHECK(qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_EXACT, TROTTER_FIRST_ORDER, 0,
                                         0) == 0,
          "evolution strategy exact");
    CHECK(qaoa_ansatz_validate(ansatz) == 0, "exact valid");
    CHECK(qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_TROTTER, TROTTER_SECOND_ORDER,
                                         2, 7) == 0,
          "evolution strategy trotter second order");
    CHECK(qaoa_ansatz_validate(ansatz) == 0, "trotter second order valid");
    CHECK(qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_TROTTER, TROTTER_RANDOMIZED, 2,
                                         42) == 0,
          "evolution strategy trotter randomized");
    CHECK(qaoa_ansatz_validate(ansatz) == 0, "trotter randomized valid");

    /* Unknown strategy or Trotter-mode tags are rejected. */
    CHECK(qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_TROTTER, 99, 1, 0) == -8,
          "evolution strategy unknown trotter mode");
    CHECK(qaoa_ansatz_evolution_strategy(ansatz, 99, TROTTER_FIRST_ORDER, 1, 0) == -8,
          "evolution strategy unknown tag");

    /* Zero-step strategies are stored but fail validation until
     * restored. */
    CHECK(qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_AUTO, TROTTER_FIRST_ORDER, 0,
                                         0) == 0,
          "evolution strategy auto zero steps");
    CHECK(qaoa_ansatz_validate(ansatz) == -3, "auto zero steps validate fails");
    CHECK(qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_AUTO, TROTTER_FIRST_ORDER, 1,
                                         0) == 0,
          "evolution strategy auto restored");
    CHECK(qaoa_ansatz_validate(ansatz) == 0, "auto restored valid");

    qaoa_ansatz_free(ansatz);
    /* `qaoa_ansatz_new` / `qaoa_ansatz_mixer` clone their inputs, so
     * every Hamiltonian handed to the ansatz stays owned by this test
     * and is freed here. */
    hamiltonian_free(mixer);
    hamiltonian_free(wide);
    hamiltonian_free(cost);

    /* The constructor rejects NULL, empty and identity-only cost
     * operators. */
    CHECK(qaoa_ansatz_new(NULL) == NULL, "qaoa_ansatz_new null");
    struct CHamiltonian* empty = hamiltonian_new(2);
    CHECK(empty != NULL, "empty hamiltonian fixture");
    CHECK(qaoa_ansatz_new(empty) == NULL, "qaoa_ansatz_new empty terms");
    hamiltonian_free(empty);
    struct CHamiltonian* identity = pauli_hamiltonian("II");
    CHECK(identity != NULL, "identity hamiltonian fixture");
    CHECK(qaoa_ansatz_new(identity) == NULL, "qaoa_ansatz_new identity");
    hamiltonian_free(identity);
}

static void test_ansatz_null_handles(void) {
    /* TwoLocal: every exported entry point tolerates a NULL handle. */
    CHECK(two_local_reps(NULL, 1) == -1, "two_local_reps(null)");
    CHECK(two_local_rotation_gates(NULL, NULL, 0) == -1, "two_local_rotation_gates(null)");
    CHECK(two_local_entanglement_gate(NULL, NULL) == -1, "two_local_entanglement_gate(null)");
    CHECK(two_local_entanglement(NULL, ENTANGLEMENT_LINEAR, NULL, 0) == -1,
          "two_local_entanglement(null)");
    CHECK(two_local_skip_final_rotation_layer(NULL, true) == -1,
          "two_local_skip_final_rotation_layer(null)");
    CHECK(two_local_validate(NULL) == -1, "two_local_validate(null)");
    CHECK(two_local_num_parameters(NULL) == 0, "two_local_num_parameters(null)");
    CHECK(two_local_num_qubits(NULL) == 0, "two_local_num_qubits(null)");
    CHECK(two_local_build_circuit(NULL, "t") == NULL, "two_local_build_circuit(null)");

    /* BasicEntanglerLayers. */
    CHECK(basic_entangler_layers_reps(NULL, 1) == -1, "basic_entangler_layers_reps(null)");
    CHECK(basic_entangler_layers_rotation_gate(NULL, NULL) == -1,
          "basic_entangler_layers_rotation_gate(null)");
    CHECK(basic_entangler_layers_entanglement_gate(NULL, NULL) == -1,
          "basic_entangler_layers_entanglement_gate(null)");
    CHECK(basic_entangler_layers_num_parameters(NULL) == 0,
          "basic_entangler_layers_num_parameters(null)");
    CHECK(basic_entangler_layers_num_qubits(NULL) == 0, "basic_entangler_layers_num_qubits(null)");
    CHECK(basic_entangler_layers_build_circuit(NULL, "t") == NULL,
          "basic_entangler_layers_build_circuit(null)");

    /* QAOAAnsatz. */
    CHECK(qaoa_ansatz_new(NULL) == NULL, "qaoa_ansatz_new(null)");
    CHECK(qaoa_ansatz_reps(NULL, 1) == -1, "qaoa_ansatz_reps(null)");
    CHECK(qaoa_ansatz_mixer(NULL, NULL) == -1, "qaoa_ansatz_mixer(null)");
    CHECK(qaoa_ansatz_initial_state(NULL, NULL) == -1, "qaoa_ansatz_initial_state(null)");
    CHECK(qaoa_ansatz_evolution_strategy(NULL, EVOLUTION_STRATEGY_EXACT, TROTTER_FIRST_ORDER, 1,
                                         0) == -1,
          "qaoa_ansatz_evolution_strategy(null)");
    CHECK(qaoa_ansatz_validate(NULL) == -1, "qaoa_ansatz_validate(null)");
    CHECK(qaoa_ansatz_num_parameters(NULL) == 0, "qaoa_ansatz_num_parameters(null)");
    CHECK(qaoa_ansatz_num_qubits(NULL) == 0, "qaoa_ansatz_num_qubits(null)");
    CHECK(qaoa_ansatz_build_circuit(NULL, "p") == NULL, "qaoa_ansatz_build_circuit(null)");

    /* Freeing NULL is a no-op everywhere. */
    two_local_free(NULL);
    basic_entangler_layers_free(NULL);
    qaoa_ansatz_free(NULL);
    hamiltonian_free(NULL);
    CHECK(1, "ansatz free(null)");
}

static void test_ansatz_build_circuit_smoke(void) {
    /* TwoLocal: reps = 2, RY, CX, linear. Three rotation layers of 3 RY
     * plus two entanglement layers of 2 CX => 13 operations, 9 params. */
    struct CTwoLocal* ansatz = two_local_new(3);
    CHECK(ansatz != NULL, "smoke TwoLocal fixture");
    CHECK(two_local_reps(ansatz, 2) == 0, "smoke TwoLocal reps");
    struct CCircuit* circuit = two_local_build_circuit(ansatz, "t");
    CHECK(circuit != NULL, "two_local_build_circuit");
    CHECK(circuit_num_qubits(circuit) == 3, "TwoLocal built qubits");
    CHECK(circuit_num_operations(circuit) == 13, "TwoLocal built ops");
    CHECK(circuit_num_parameters(circuit) == 9, "TwoLocal built params");
    circuit_free(circuit);

    /* Skipping the final rotation layer drops the last 3 RY gates. */
    CHECK(two_local_skip_final_rotation_layer(ansatz, true) == 0, "smoke TwoLocal skip final");
    struct CCircuit* skipped = two_local_build_circuit(ansatz, "t");
    CHECK(skipped != NULL, "TwoLocal skipped build");
    CHECK(circuit_num_operations(skipped) == 2 * 3 + 2 * 2, "TwoLocal skipped ops");
    CHECK(circuit_num_parameters(skipped) == 6, "TwoLocal skipped params");
    circuit_free(skipped);

    /* Full topology: C(3,2) = 3 CX per entanglement layer => 15 ops. */
    CHECK(two_local_skip_final_rotation_layer(ansatz, false) == 0, "smoke TwoLocal restore final");
    CHECK(two_local_entanglement(ansatz, ENTANGLEMENT_FULL, NULL, 0) == 0,
          "smoke TwoLocal full topology");
    struct CCircuit* full = two_local_build_circuit(ansatz, "t");
    CHECK(full != NULL, "TwoLocal full build");
    CHECK(circuit_num_operations(full) == 9 + 2 * 3, "TwoLocal full ops");
    circuit_free(full);
    two_local_free(ansatz);

    /* BasicEntanglerLayers: reps = 2 on a 3-qubit ring => per layer
     * 3 rotations + 3 entanglers => 12 operations, 6 parameters. */
    struct CBasicEntanglerLayers* layers = basic_entangler_layers_new(3);
    CHECK(layers != NULL, "smoke layers fixture");
    CHECK(basic_entangler_layers_reps(layers, 2) == 0, "smoke layers reps");
    struct CCircuit* ring = basic_entangler_layers_build_circuit(layers, "t");
    CHECK(ring != NULL, "basic_entangler_layers_build_circuit");
    CHECK(circuit_num_qubits(ring) == 3, "layers built qubits");
    CHECK(circuit_num_operations(ring) == 12, "layers built ops");
    CHECK(circuit_num_parameters(ring) == 6, "layers built params");
    CHECK(basic_entangler_layers_build_circuit(layers, NULL) == NULL, "layers build null prefix");
    circuit_free(ring);
    basic_entangler_layers_free(layers);

    /* A single qubit has no entanglement: only the rotations remain. */
    struct CBasicEntanglerLayers* single = basic_entangler_layers_new(1);
    CHECK(single != NULL, "smoke single-qubit fixture");
    CHECK(basic_entangler_layers_reps(single, 2) == 0, "smoke single-qubit reps");
    struct CCircuit* solo = basic_entangler_layers_build_circuit(single, "t");
    CHECK(solo != NULL, "single-qubit build");
    CHECK(circuit_num_operations(solo) == 2, "single-qubit ops");
    CHECK(circuit_num_parameters(solo) == 2, "single-qubit params");
    circuit_free(solo);
    basic_entangler_layers_free(single);

    /* QAOA: the H layer plus cost/mixer evolution for one rep. The
     * exact operation count depends on the Pauli-evolution lowering,
     * so only smoke-check the floor while pinning the shape. */
    struct CHamiltonian* cost = pauli_hamiltonian("ZZ");
    CHECK(cost != NULL, "smoke qaoa cost fixture");
    struct CQAOAAnsatz* qaoa = qaoa_ansatz_new(cost);
    CHECK(qaoa != NULL, "smoke qaoa_ansatz_new");
    struct CCircuit* qc = qaoa_ansatz_build_circuit(qaoa, "t");
    CHECK(qc != NULL, "qaoa_ansatz_build_circuit");
    CHECK(circuit_num_qubits(qc) == 2, "QAOA built qubits");
    CHECK(circuit_num_parameters(qc) == 2, "QAOA built params");
    CHECK(circuit_num_operations(qc) >= 2, "QAOA built ops floor");
    CHECK(qaoa_ansatz_build_circuit(qaoa, NULL) == NULL, "QAOA build null prefix");
    circuit_free(qc);
    qaoa_ansatz_free(qaoa);
    /* The clone-based constructor keeps the cost Hamiltonian owned by
     * this test, so it is freed here. */
    hamiltonian_free(cost);
}

int main(void) {
    test_two_local_structure();
    test_two_local_invalid_inputs();
    test_basic_entangler_layers_structure();
    test_qaoa_ansatz_structure();
    test_ansatz_null_handles();
    test_ansatz_build_circuit_smoke();

    printf("\n%d failure(s)\n", g_failures);
    return (g_failures == 0) ? 0 : 1;
}
