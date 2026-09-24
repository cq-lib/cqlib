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

static void test_lifecycle(void) {
    CDensityMatrixNoise* sim = density_matrix_noise_new(2, NULL);
    assert(sim != NULL);
    assert(density_matrix_noise_num_qubits(sim) == 2);
    density_matrix_noise_free(sim);

    density_matrix_noise_free(NULL);
    assert(density_matrix_noise_num_qubits(NULL) == 0);
    assert(density_matrix_noise_probabilities_len(NULL) == 0);
    assert(density_matrix_noise_apply_h(NULL, 0) == -1);
    assert(density_matrix_noise_measure(NULL, 0) == -1);
    assert(density_matrix_noise_measure_all(NULL) == NULL);
    assert(density_matrix_noise_sample_shots(NULL, 4) == NULL);
}

static void test_ideal_bell_state(void) {
    CDensityMatrixNoise* sim = density_matrix_noise_new(2, NULL);
    assert(sim != NULL);

    assert(density_matrix_noise_apply_h(sim, 0) == 0);
    assert(density_matrix_noise_apply_cx(sim, 0, 1) == 0);

    uintptr_t len = density_matrix_noise_probabilities_len(sim);
    assert(len == 4);
    double probs[4];
    assert(density_matrix_noise_probabilities(sim, probs, len) == 0);
    assert(fabs(probs[0] - 0.5) < 1e-9);
    assert(fabs(probs[1]) < 1e-9);
    assert(fabs(probs[2]) < 1e-9);
    assert(fabs(probs[3] - 0.5) < 1e-9);

    /* Wrong buffer length is rejected. */
    assert(density_matrix_noise_probabilities(sim, probs, 2) == -8);

    /* Collapse the state with a full measurement. */
    char* outcome = density_matrix_noise_measure_all(sim);
    assert(outcome != NULL);
    assert(strlen(outcome) == 2);
    assert(outcome[0] == outcome[1]); /* |00> or |11> */
    cqlib_string_free(outcome);

    /* Sampling the collapsed state keeps results consistent. */
    COutcomeList* shots = density_matrix_noise_sample_shots(sim, 16);
    assert(shots != NULL);
    assert(outcome_list_len(shots) == 16);
    char* first = outcome_list_get(shots, 0);
    assert(first != NULL);
    assert(strlen(first) == 2);
    cqlib_string_free(first);
    outcome_list_free(shots);

    /* Reset then measure: qubit 0 is back to |0>. */
    assert(density_matrix_noise_reset(sim, 0) == 0);
    assert(density_matrix_noise_measure(sim, 0) == 0);

    /* Qubit out of bounds. */
    assert(density_matrix_noise_apply_h(sim, 2) == -2);
    assert(density_matrix_noise_apply_cx(sim, 0, 2) == -2);
    density_matrix_noise_free(sim);
}

static void test_gate_noise(void) {
    /* X followed by 10% bit-flip noise: P(|1>) = 0.9, P(|0>) = 0.1. */
    CNoiseModel* nm = noise_model_new();
    assert(nm != NULL);
    assert(noise_model_add_single_qubit(nm, "X", 0, NOISE_BIT_FLIP, 0.1) == 0);

    CDensityMatrixNoise* sim = density_matrix_noise_new(1, nm);
    assert(sim != NULL);
    assert(density_matrix_noise_apply_x(sim, 0) == 0);

    double probs[2];
    assert(density_matrix_noise_probabilities(sim, probs, 2) == 0);
    assert(fabs(probs[1] - 0.9) < 1e-9);
    assert(fabs(probs[0] - 0.1) < 1e-9);

    density_matrix_noise_free(sim);
    noise_model_free(nm);
}

static void test_readout_noise(void) {
    /* |0> with P(measure 1 | true 0) = 0.1. */
    CNoiseModel* nm = noise_model_new();
    assert(noise_model_add_readout(nm, 0, 0.0, 0.1) == 0);

    CDensityMatrixNoise* sim = density_matrix_noise_new(1, nm);

    /* Ideal distribution ignores readout errors. */
    double ideal[2];
    assert(density_matrix_noise_probabilities(sim, ideal, 2) == 0);
    assert(fabs(ideal[0] - 1.0) < 1e-9);
    assert(fabs(ideal[1]) < 1e-9);

    /* With readout: P(|0>) = 0.9, P(|1>) = 0.1. */
    assert(density_matrix_noise_probabilities_with_readout_len(sim) == 2);
    const uint32_t qs[1] = {0};
    double probs[2];
    assert(density_matrix_noise_probabilities_with_readout(sim, probs, 2, qs, 1) == 0);
    assert(fabs(probs[0] - 0.9) < 1e-9);
    assert(fabs(probs[1] - 0.1) < 1e-9);

    /* Out-of-bounds readout qubit is rejected. */
    const uint32_t bad[1] = {99};
    assert(density_matrix_noise_probabilities_with_readout(sim, probs, 2, bad, 1) == -2);

    density_matrix_noise_free(sim);
    noise_model_free(nm);
}

static void test_expectation(void) {
    /* <Z> = -1 on |1>. */
    CDensityMatrixNoise* sim = density_matrix_noise_new(1, NULL);
    assert(density_matrix_noise_apply_x(sim, 0) == 0);

    CPauliString* pauli = pauli_string_parse("Z");
    assert(pauli != NULL);
    CHamiltonian* ham = hamiltonian_from_pauli(pauli);
    assert(ham != NULL);

    double value = NAN;
    assert(density_matrix_noise_expectation(sim, ham, &value) == 0);
    assert(fabs(value + 1.0) < 1e-9);

    hamiltonian_free(ham);
    /* `hamiltonian_from_pauli` consumed the `pauli` handle; do not free it. */
    density_matrix_noise_free(sim);
}

static void test_standard_gate_noise_entry(void) {
    CDensityMatrixNoise* sim = density_matrix_noise_new(1, NULL);

    const uint32_t qs[1] = {0};
    assert(density_matrix_noise_apply_standard_gate_noise(sim, "H", qs, 1, NULL, 0) == 0);

    double probs[2];
    assert(density_matrix_noise_probabilities(sim, probs, 2) == 0);
    assert(fabs(probs[0] - 0.5) < 1e-9);
    assert(fabs(probs[1] - 0.5) < 1e-9);

    /* Parameterised gate via the generic entry point. */
    const double theta = 3.14159265358979323846 / 2.0;
    assert(density_matrix_noise_apply_standard_gate_noise(sim, "RY", qs, 1, &theta, 1) == 0);
    assert(density_matrix_noise_probabilities(sim, probs, 2) == 0);
    assert(fabs(probs[0] + probs[1] - 1.0) < 1e-9);

    /* Unknown gate name -> -8. */
    assert(density_matrix_noise_apply_standard_gate_noise(sim, "NOT_A_GATE", qs, 1, NULL, 0) == -8);
    /* Wrong arity -> -8. */
    assert(density_matrix_noise_apply_standard_gate_noise(sim, "CX", qs, 1, NULL, 0) == -8);
    /* Wrong parameter count -> -8. */
    assert(density_matrix_noise_apply_standard_gate_noise(sim, "RX", qs, 1, NULL, 0) == -8);
    /* Out-of-bounds qubit -> -2. */
    const uint32_t oob[1] = {5};
    assert(density_matrix_noise_apply_standard_gate_noise(sim, "H", oob, 1, NULL, 0) == -2);

    density_matrix_noise_free(sim);
}

static void test_unitary_gate(void) {
    CDensityMatrixNoise* sim = density_matrix_noise_new(1, NULL);

    /* Pauli-X as a 2x2 row-major Complex64 matrix. */
    const Complex64 x_matrix[4] = {
        {0.0, 0.0},
        {1.0, 0.0},
        {1.0, 0.0},
        {0.0, 0.0},
    };
    const uint32_t qs[1] = {0};
    assert(density_matrix_noise_apply_unitary_gate(sim, qs, 1, x_matrix, 2) == 0);

    double probs[2];
    assert(density_matrix_noise_probabilities(sim, probs, 2) == 0);
    assert(fabs(probs[1] - 1.0) < 1e-9);
    assert(fabs(probs[0]) < 1e-9);

    /* Matrix dimension inconsistent with the qubit count -> -8. */
    const uint32_t two_qs[2] = {0, 0};
    assert(density_matrix_noise_apply_unitary_gate(sim, two_qs, 2, x_matrix, 2) == -8);

    density_matrix_noise_free(sim);
}

static void test_from_circuit(void) {
    CCircuit* circuit = circuit_new(1);
    assert(circuit != NULL);
    assert(circuit_x(circuit, 0) == 0);

    CNoiseModel* nm = noise_model_new();
    assert(noise_model_add_single_qubit(nm, "X", 0, NOISE_BIT_FLIP, 0.1) == 0);

    CDensityMatrixNoise* sim = density_matrix_noise_from_circuit(circuit, nm);
    assert(sim != NULL);
    assert(density_matrix_noise_num_qubits(sim) == 1);

    double probs[2];
    assert(density_matrix_noise_probabilities(sim, probs, 2) == 0);
    assert(fabs(probs[1] - 0.9) < 1e-9);
    assert(fabs(probs[0] - 0.1) < 1e-9);

    density_matrix_noise_free(sim);
    noise_model_free(nm);
    circuit_free(circuit);
    assert(density_matrix_noise_from_circuit(NULL, NULL) == NULL);
}

int main(void) {
    test_lifecycle();
    test_ideal_bell_state();
    test_gate_noise();
    test_readout_noise();
    test_expectation();
    test_standard_gate_noise_entry();
    test_unitary_gate();
    test_from_circuit();
    printf("binding-c qis noise tests passed\n");
    return 0;
}
