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

#include "cqlib_c.h"

/* Demo estimator: stands in for a real backend execution. The ideal
 * expectation is 1.0; each additional gate copy contributes decay, so
 * heavily folded circuits yield noisier raw estimates. */
static void demo_estimator(const CCircuit* circuit, const CHamiltonian* hamiltonian,
                           uintptr_t shots, double* expectation, double* variance) {
    (void)hamiltonian;
    (void)shots;
    const double ideal = 1.0;
    const double decay = 0.99;
    *expectation = ideal * pow(decay, (double)circuit_num_operations(circuit));
    *variance = 0.0;
}

static void test_zne_mitigation_fold(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);

    const int32_t levels[] = {0, 1, 2};
    CZneMitigation* zne = zne_mitigation_new(circuit, levels, 3);
    assert(zne != NULL);

    assert(zne_mitigation_noise_factor(zne, 0) == 1);
    assert(zne_mitigation_noise_factor(zne, 1) == 3);
    assert(zne_mitigation_noise_factor(zne, 2) == 5);

    /* Global folding produces one circuit per level. */
    CCircuitList* list = zne_mitigation_fold_circuit(zne, NULL);
    assert(list != NULL);
    assert(circuit_list_len(list) == 3);
    uintptr_t ops0 = circuit_num_operations(circuit_list_get(list, 0));
    uintptr_t ops2 = circuit_num_operations(circuit_list_get(list, 2));
    assert(ops0 == 2);
    assert(ops2 > ops0);
    circuit_list_free(list);

    /* Selective folding on H only. */
    list = zne_mitigation_fold_circuit(zne, "H");
    assert(list != NULL);
    assert(circuit_list_len(list) == 3);
    circuit_list_free(list);

    /* Unknown gate name must fail. */
    assert(zne_mitigation_fold_circuit(zne, "NOT_A_GATE") == NULL);

    zne_mitigation_free(zne);
    zne_mitigation_free(NULL);
    circuit_free(circuit);
}

static void test_zne_full_pipeline(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);

    const int32_t levels[] = {0, 1, 2};
    CErrorMitigation* em = error_mitigation_new(circuit, MITIGATION_ZNE, levels, 3, 0);
    assert(em != NULL);

    CPauliString* pauli = pauli_string_parse("ZZ");
    assert(pauli != NULL);
    CHamiltonian* ham = hamiltonian_from_pauli(pauli);
    assert(ham != NULL);

    assert(error_mitigation_run(em, MITIGATION_ZNE, ham, 0, 0, demo_estimator) == 0);

    double expectation = NAN;
    double variance = NAN;
    assert(error_mitigation_get_mitigated(em, PROCESS_ZNE_POLYNOMIAL, 1, &expectation, &variance) ==
           0);
    assert(isfinite(expectation));

    hamiltonian_free(ham);
    error_mitigation_free(em);
    error_mitigation_free(NULL);
    circuit_free(circuit);
}

static void test_get_mitigated_requires_run(void) {
    CCircuit* circuit = circuit_new(1);
    assert(circuit != NULL);

    const int32_t levels[] = {0, 1};
    CErrorMitigation* em = error_mitigation_new(circuit, MITIGATION_ZNE, levels, 2, 0);
    assert(em != NULL);

    double expectation = NAN;
    double variance = NAN;
    assert(error_mitigation_get_mitigated(em, PROCESS_ZNE_POLYNOMIAL, 0, &expectation, &variance) ==
           -7);

    /* Method mismatch between new() and run() must fail. */
    assert(error_mitigation_run(em, MITIGATION_VIRTUAL_DISTILLATION, NULL, 0, 0, demo_estimator) !=
           0);

    error_mitigation_free(em);
    circuit_free(circuit);
}

static void test_virtual_distillation(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);

    CVirtualDistillation* vd = virtual_distillation_new(circuit, 2);
    assert(vd != NULL);

    CCircuit* built = virtual_distillation_build_circuit(vd);
    assert(built != NULL);
    assert(circuit_num_qubits(built) >= 2);
    circuit_free(built);

    virtual_distillation_free(vd);
    virtual_distillation_free(NULL);
    circuit_free(circuit);
}

int main(void) {
    test_zne_mitigation_fold();
    test_zne_full_pipeline();
    test_get_mitigated_requires_run();
    test_virtual_distillation();
    printf("binding-c error mitigation tests passed\n");
    return 0;
}
