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
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cqlib_c.h"

static void test_statevector_bell_state(void) {
    CStatevector* sv = statevector_new(2);
    assert(sv != NULL);
    assert(statevector_num_qubits(sv) == 2);

    assert(statevector_apply_h(sv, 0) == 0);
    assert(statevector_apply_cx(sv, 0, 1) == 0);

    uintptr_t len = statevector_probabilities_len(sv);
    assert(len == 4);
    double probs[4];
    assert(statevector_probabilities(sv, probs, len) == 0);
    assert(fabs(probs[0] - 0.5) < 1e-9);
    assert(fabs(probs[1]) < 1e-9);
    assert(fabs(probs[2]) < 1e-9);
    assert(fabs(probs[3] - 0.5) < 1e-9);

    char* outcome = statevector_measure_all(sv);
    assert(outcome != NULL);
    assert(strlen(outcome) == 2);
    assert(outcome[0] == outcome[1]); /* |00> or |11> */
    cqlib_string_free(outcome);

    assert(statevector_reset(sv, 0) == 0);

    /* Sampling */
    COutcomeList* shots = statevector_sample_shots(sv, 16);
    assert(shots != NULL);
    assert(outcome_list_len(shots) == 16);
    char* first = outcome_list_get(shots, 0);
    assert(first != NULL);
    assert(strlen(first) == 2);
    cqlib_string_free(first);
    outcome_list_free(shots);

    /* Errors */
    assert(statevector_apply_h(sv, 2) == -2);
    statevector_free(sv);
    statevector_free(NULL);
    assert(statevector_num_qubits(NULL) == 0);
}

static void test_statevector_expectation(void) {
    CStatevector* sv = statevector_new(2);
    assert(sv != NULL);

    /* <II> = 1.0 on any state. */
    CPauliString* pauli = pauli_string_parse("II");
    assert(pauli != NULL);
    CHamiltonian* ham = hamiltonian_from_pauli(pauli);
    assert(ham != NULL);

    double expectation = NAN;
    assert(statevector_expectation(sv, ham, &expectation) == 0);
    assert(fabs(expectation - 1.0) < 1e-9);

    hamiltonian_free(ham);
    statevector_free(sv);
}

static void test_statevector_y_expectation_from_circuit(void) {
    for (int sign = -1; sign <= 1; sign += 2) {
        CCircuit* circuit = circuit_new(1);
        assert(circuit != NULL);
        assert(circuit_h(circuit, 0) == 0);
        assert(circuit_rz(circuit, 0, sign * acos(-1.0) / 2.0) == 0);
        CStatevector* sv = statevector_from_circuit(circuit);
        assert(sv != NULL);
        CPauliString* pauli = pauli_string_parse("Y");
        assert(pauli != NULL);
        /* from_pauli consumes the Pauli handle. */
        CHamiltonian* ham = hamiltonian_from_pauli(pauli);
        assert(ham != NULL);

        double expectation = NAN;
        assert(statevector_expectation(sv, ham, &expectation) == 0);
        assert(fabs(expectation - sign) < 1e-10);

        hamiltonian_free(ham);
        statevector_free(sv);
        circuit_free(circuit);
    }
}

static void test_statevector_pauli_phases_and_y_parity(void) {
    const char* prefixes[] = {"", "-", "i", "-i"};
    /* Each coefficient absorbs its Pauli phase, leaving -0.37 * P. */
    const double coeff_re[] = {-0.37, 0.37, 0.0, 0.0};
    const double coeff_im[] = {0.0, 0.0, 0.37, -0.37};
    for (uintptr_t n = 1; n <= 3; ++n) {
        CStatevector* sv = statevector_new(n);
        assert(sv != NULL);
        double single[3][4];
        for (uintptr_t q = 0; q < n; ++q) {
            double theta = 0.7 + 0.2 * q;
            double phi = -0.3 + 0.4 * q;
            assert(statevector_apply_ry(sv, (uint32_t)q, theta) == 0);
            assert(statevector_apply_rz(sv, (uint32_t)q, phi) == 0);
            /* RZ(phi) RY(theta)|0>: independently known Bloch components. */
            single[q][0] = 1.0;
            single[q][1] = sin(theta) * cos(phi);
            single[q][2] = sin(theta) * sin(phi);
            single[q][3] = cos(theta);
        }
        for (uintptr_t encoded = 0; encoded < ((uintptr_t)1 << (2 * n)); ++encoded) {
            char label[4] = {0};
            double expected = -0.37;
            for (uintptr_t q = 0; q < n; ++q) {
                uintptr_t op = (encoded >> (2 * q)) & 3;
                label[n - 1 - q] = "IXYZ"[op];
                expected *= single[q][op];
            }
            assert(fabs(expected) > 1e-10);
            for (uintptr_t phase = 0; phase < 4; ++phase) {
                char phased_label[6];
                snprintf(phased_label, sizeof(phased_label), "%s%s", prefixes[phase], label);
                CPauliString* pauli = pauli_string_parse(phased_label);
                assert(pauli != NULL);
                CHamiltonian* ham = hamiltonian_new(n);
                assert(ham != NULL);
                assert(hamiltonian_add_term(ham, pauli, coeff_re[phase], coeff_im[phase]) == 0);
                /* add_term clones the Pauli handle. */
                pauli_string_free(pauli);
                double expectation = NAN;
                assert(statevector_expectation(sv, ham, &expectation) == 0);
                assert(fabs(expectation - expected) < 1e-10);
                assert(hamiltonian_simplify(ham) == 0);
                assert(statevector_expectation(sv, ham, &expectation) == 0);
                assert(fabs(expectation - expected) < 1e-10);
                hamiltonian_free(ham);
            }
        }
        statevector_free(sv);
    }
}

static void test_statevector_mixed_hamiltonian_expectation(void) {
    CStatevector* sv = statevector_new(1);
    assert(sv != NULL);
    assert(statevector_apply_h(sv, 0) == 0);
    assert(statevector_apply_rz(sv, 0, acos(-1.0) / 4.0) == 0);
    CHamiltonian* ham = hamiltonian_new(1);
    assert(ham != NULL);
    const char* labels[] = {"I", "X", "-iY", "Y"};
    const double coeff_re[] = {0.3, 0.5, 0.0, -0.25};
    const double coeff_im[] = {0.0, 0.0, 0.75, 0.0};
    for (uintptr_t i = 0; i < 4; ++i) {
        CPauliString* pauli = pauli_string_parse(labels[i]);
        assert(pauli != NULL);
        assert(hamiltonian_add_term(ham, pauli, coeff_re[i], coeff_im[i]) == 0);
        pauli_string_free(pauli);
    }
    /* H = 0.3 I + 0.5 X + 0.5 Y, with <X> = <Y> = 1/sqrt(2). */
    double expectation = NAN;
    double expected = 0.3 + 1.0 / sqrt(2.0);
    assert(statevector_expectation(sv, ham, &expectation) == 0);
    assert(fabs(expectation - expected) < 1e-10);
    assert(hamiltonian_simplify(ham) == 0);
    assert(statevector_expectation(sv, ham, &expectation) == 0);
    assert(fabs(expectation - expected) < 1e-10);
    hamiltonian_free(ham);
    statevector_free(sv);
}

static void test_statevector_from_circuit(void) {
    CCircuit* circuit = circuit_new(1);
    assert(circuit_h(circuit, 0) == 0);

    CStatevector* sv = statevector_from_circuit(circuit);
    assert(sv != NULL);
    assert(statevector_num_qubits(sv) == 1);

    double probs[2];
    assert(statevector_probabilities(sv, probs, 2) == 0);
    assert(fabs(probs[0] - 0.5) < 1e-9);
    assert(fabs(probs[1] - 0.5) < 1e-9);

    statevector_free(sv);
    circuit_free(circuit);
}

static void test_density_matrix(void) {
    CDensityMatrix* dm = density_matrix_new(2);
    assert(dm != NULL);
    assert(density_matrix_num_qubits(dm) == 2);

    assert(density_matrix_apply_h(dm, 0) == 0);
    assert(density_matrix_apply_cx(dm, 0, 1) == 0);

    double probs[4];
    assert(density_matrix_probabilities_len(dm) == 4);
    assert(density_matrix_probabilities(dm, probs, 4) == 0);
    assert(fabs(probs[0] - 0.5) < 1e-9);
    assert(fabs(probs[3] - 0.5) < 1e-9);

    /* Partial trace keeping qubit 1 -> single maximally mixed qubit. */
    const uint32_t keep[] = {1};
    CDensityMatrix* reduced = density_matrix_partial_trace(dm, keep, 1);
    assert(reduced != NULL);
    assert(density_matrix_num_qubits(reduced) == 1);
    double rprobs[2];
    assert(density_matrix_probabilities(reduced, rprobs, 2) == 0);
    assert(fabs(rprobs[0] - 0.5) < 1e-9);
    assert(fabs(rprobs[1] - 0.5) < 1e-9);
    density_matrix_free(reduced);

    assert(density_matrix_apply_h(dm, 5) == -2);
    density_matrix_free(dm);
    density_matrix_free(NULL);
}

static void test_stabilizer(void) {
    CStabilizerState* stab = stabilizer_new(2);
    assert(stab != NULL);
    assert(stabilizer_num_qubits(stab) == 2);

    assert(stabilizer_apply_h(stab, 0) == 0);
    assert(stabilizer_apply_cx(stab, 0, 1) == 0);

    char* outcome = stabilizer_measure_all(stab);
    assert(outcome != NULL);
    cqlib_string_free(outcome);

    /* <ZZ> = +1 on the Bell state. */
    CPauliString* pauli = pauli_string_parse("ZZ");
    assert(pauli != NULL);
    int32_t value = 0;
    assert(stabilizer_pauli_expectation(stab, pauli, &value) == 0);
    assert(value == 1);
    pauli_string_free(pauli);

    assert(stabilizer_get_stabilizers_len(stab) == 2);
    char* gen = stabilizer_get_stabilizer(stab, 0);
    assert(gen != NULL);
    cqlib_string_free(gen);

    assert(stabilizer_apply_h(stab, 2) == -2);
    stabilizer_free(stab);
    stabilizer_free(NULL);
    assert(stabilizer_new(0) == NULL);
}

static void test_pauli_string(void) {
    CPauliString* pauli = pauli_string_parse("XYZ");
    assert(pauli != NULL);
    assert(pauli_string_num_qubits(pauli) == 3);
    /* Parsing is in reverse order: the first character maps to the highest
     * qubit index, so get(idx) is qubit-index based. */
    assert(pauli_string_get_pauli(pauli, 0) == PAULI_Z);
    assert(pauli_string_get_pauli(pauli, 1) == PAULI_Y);
    assert(pauli_string_get_pauli(pauli, 2) == PAULI_X);
    /* Out-of-bounds reads return the 0xFF sentinel. */
    assert(pauli_string_get_pauli(pauli, 3) == 0xFF);

    assert(pauli_string_set_pauli(pauli, 0, PAULI_I) == 0);
    assert(pauli_string_get_pauli(pauli, 0) == PAULI_I);

    assert(pauli_string_parse(NULL) == NULL);
    pauli_string_free(pauli);
    pauli_string_free(NULL);
}

static void test_hamiltonian(void) {
    CHamiltonian* ham = hamiltonian_new(2);
    assert(ham != NULL);
    assert(hamiltonian_num_qubits(ham) == 2);
    assert(hamiltonian_num_terms(ham) == 0);

    CPauliString* zz = pauli_string_parse("ZZ");
    assert(zz != NULL);
    assert(hamiltonian_add_term(ham, zz, 0.5, 0.0) == 0);
    assert(hamiltonian_num_terms(ham) == 1);
    pauli_string_free(zz);

    /* Qubit-count mismatch must be rejected. */
    CPauliString* single = pauli_string_parse("X");
    assert(single != NULL);
    assert(hamiltonian_add_term(ham, single, 1.0, 0.0) != 0);
    pauli_string_free(single);

    hamiltonian_free(ham);
    hamiltonian_free(NULL);
}

int main(void) {
    test_statevector_bell_state();
    test_statevector_expectation();
    test_statevector_y_expectation_from_circuit();
    test_statevector_pauli_phases_and_y_parity();
    test_statevector_mixed_hamiltonian_expectation();
    test_statevector_from_circuit();
    test_density_matrix();
    test_stabilizer();
    test_pauli_string();
    test_hamiltonian();
    printf("binding-c qis tests passed\n");
    return 0;
}
