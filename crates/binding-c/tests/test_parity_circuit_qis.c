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
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "cqlib_c.h"

static void test_circuit_dag_add_parameter(void) {
    CParameter* theta = param_parse("theta");
    assert(theta != NULL);

    // A DAG built from a circuit already interns "theta" via the RX gate.
    CCircuit* circuit = circuit_new(1);
    assert(circuit_rx_param(circuit, 0, theta) == 0);
    CCircuitDag* dag = circuit_dag_from_circuit(circuit);
    assert(dag != NULL);
    assert(circuit_dag_parameters_len(dag) == 1);

    // Re-adding the resident parameter reports "not inserted".
    uintptr_t index = 99;
    int32_t inserted = -1;
    assert(circuit_dag_add_parameter(dag, theta, &index, &inserted) == 0);
    assert(index == 0);
    assert(inserted == 0);
    assert(circuit_dag_parameters_len(dag) == 1);

    // A fresh parameter lands at the next table index.
    CParameter* phi = param_parse("phi");
    assert(phi != NULL);
    assert(circuit_dag_add_parameter(dag, phi, &index, &inserted) == 0);
    assert(index == 1);
    assert(inserted == 1);
    assert(circuit_dag_parameters_len(dag) == 2);

    // Adding it again keeps the index and reports "not inserted".
    assert(circuit_dag_add_parameter(dag, phi, &index, &inserted) == 0);
    assert(index == 1);
    assert(inserted == 0);

    // Output pointers may be omitted.
    assert(circuit_dag_add_parameter(dag, phi, NULL, NULL) == 0);

    // NULL handles and NULL parameters are rejected.
    assert(circuit_dag_add_parameter(NULL, phi, &index, &inserted) == -1);
    assert(circuit_dag_add_parameter(dag, NULL, &index, &inserted) == -1);

    circuit_dag_free(dag);
    circuit_free(circuit);
    param_free(theta);
    param_free(phi);
}

static void test_pauli_string_expectation(void) {
    CPauliString* ps = pauli_string_new(2);
    assert(ps != NULL);
    assert(pauli_string_set_pauli(ps, 0, PAULI_Z) == 0);

    // Z0 on |00> gives +1; on little-endian "01" (qubit 0 = 1) gives -1.
    const char* state00 = "00";
    const char* state01 = "01";
    const char* bits1[1] = {state00};
    const double one[1] = {1.0};
    double out = 99.0;

    assert(pauli_string_expectation(ps, bits1, one, 1, &out) == 0);
    assert(out == 1.0);

    const char* bits01[1] = {state01};
    assert(pauli_string_expectation(ps, bits01, one, 1, &out) == 0);
    assert(out == -1.0);

    // An even mixture of the two eigenstates averages to zero.
    const char* bits2[2] = {state00, state01};
    const double half[2] = {0.5, 0.5};
    assert(pauli_string_expectation(ps, bits2, half, 2, &out) == 0);
    assert(fabs(out) < 1e-12);

    // Off-diagonal strings vanish on computational-basis distributions.
    assert(pauli_string_set_pauli(ps, 0, PAULI_X) == 0);
    assert(pauli_string_expectation(ps, bits1, one, 1, &out) == 0);
    assert(out == 0.0);

    // NULL handle, NULL output, and NULL arrays with data are rejected.
    assert(pauli_string_expectation(NULL, bits1, one, 1, &out) == -1);
    assert(pauli_string_expectation(ps, bits1, one, 1, NULL) == -1);
    assert(pauli_string_expectation(ps, NULL, one, 1, &out) == -1);
    assert(pauli_string_expectation(ps, bits1, NULL, 1, &out) == -1);

    // A NULL bitstring entry inside the arrays is an invalid parameter.
    const char* bits_null[1] = {NULL};
    assert(pauli_string_expectation(ps, bits_null, one, 1, &out) == -8);

    // An empty distribution is valid and yields zero.
    assert(pauli_string_expectation(ps, NULL, NULL, 0, &out) == 0);
    assert(out == 0.0);

    pauli_string_free(ps);
}

static void test_density_matrix_zeros_and_validate_physical(void) {
    CDensityMatrix* dm = density_matrix_zeros(2);
    assert(dm != NULL);
    assert(density_matrix_num_qubits(dm) == 2);

    // Every element is zero, so the trace vanishes.
    double trace_re = 9.0, trace_im = 9.0;
    assert(density_matrix_trace(dm, &trace_re, &trace_im) == 0);
    assert(trace_re == 0.0 && trace_im == 0.0);

    // Zero trace fails the unit-trace check (NotNormalized -> -8).
    assert(density_matrix_validate_physical(dm, 1e-10) == -8);
    density_matrix_free(dm);

    // Physical states pass: the pure |0> state and the maximally mixed I/2.
    CDensityMatrix* pure = density_matrix_new(1);
    assert(pure != NULL);
    assert(density_matrix_validate_physical(pure, 1e-10) == 0);
    density_matrix_free(pure);

    CDensityMatrix* mixed = density_matrix_maximally_mixed(1);
    assert(mixed != NULL);
    assert(density_matrix_validate_physical(mixed, 1e-10) == 0);

    // NULL handle and non-finite tolerance are rejected; an oversized
    // qubit count fails the constructor instead of crashing.
    assert(density_matrix_validate_physical(NULL, 1e-10) == -1);
    assert(density_matrix_validate_physical(mixed, NAN) == -8);
    density_matrix_free(mixed);
    assert(density_matrix_zeros(100) == NULL);
}

static void test_statevector_entanglement_entropy_pure(void) {
    // Bell state |Phi+> = (|00> + |11>)/sqrt(2).
    CStatevector* sv = statevector_new(2);
    assert(sv != NULL);
    assert(statevector_apply_h(sv, 0) == 0);
    assert(statevector_apply_cx(sv, 0, 1) == 0);

    // One bit (log2) of entanglement across the 0|1 split.
    const uint32_t subsys[1] = {0};
    double out = 99.0;
    assert(statevector_entanglement_entropy_pure(sv, subsys, 1, &out) == 0);
    assert(fabs(out - 1.0) < 1e-9);

    // A product state carries no entanglement.
    CStatevector* product = statevector_new(2);
    assert(product != NULL);
    assert(statevector_entanglement_entropy_pure(product, subsys, 1, &out) == 0);
    assert(fabs(out) < 1e-9);
    statevector_free(product);

    // NULL handle, NULL output, and a NULL subsystem buffer with data.
    assert(statevector_entanglement_entropy_pure(NULL, subsys, 1, &out) == -1);
    assert(statevector_entanglement_entropy_pure(sv, subsys, 1, NULL) == -1);
    assert(statevector_entanglement_entropy_pure(sv, NULL, 1, &out) == -1);

    statevector_free(sv);
}

int main(void) {
    test_circuit_dag_add_parameter();
    test_pauli_string_expectation();
    test_density_matrix_zeros_and_validate_physical();
    test_statevector_entanglement_entropy_pure();
    printf("binding-c parity circuit/qis tests passed\n");
    return 0;
}
