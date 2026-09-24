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

static void test_pauli_evolution_xz(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_num_operations(circuit) == 0);

    CPauliString* pauli = pauli_string_parse("XZ");
    assert(pauli != NULL);

    // exp(-i * pi/4 * XZ) appends basis changes, a CNOT ladder, one RZ,
    // and the inverse ladder, so the operation count must grow.
    const uint32_t qubits[2] = {0, 1};
    assert(circuit_pauli_evolution(circuit, pauli, 1.5707963267948966, qubits, 2) == 0);
    assert(circuit_num_operations(circuit) > 0);

    pauli_string_free(pauli);
    circuit_free(circuit);
}

static void test_pauli_evolution_errors(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);

    CPauliString* pauli = pauli_string_parse("XZ");
    assert(pauli != NULL);
    const uint32_t qubits[2] = {0, 1};

    // NULL circuit / pauli / qubit buffer.
    assert(circuit_pauli_evolution(NULL, pauli, 1.0, qubits, 2) == -1);
    assert(circuit_pauli_evolution(circuit, NULL, 1.0, qubits, 2) == -1);
    assert(circuit_pauli_evolution(circuit, pauli, 1.0, NULL, 2) == -1);

    // Out-of-bounds qubit index.
    const uint32_t out_of_bounds[2] = {0, 5};
    assert(circuit_pauli_evolution(circuit, pauli, 1.0, out_of_bounds, 2) == -2);

    // Position-count mismatch: 3-position Pauli string on 2 positions.
    CPauliString* big = pauli_string_parse("XXX");
    assert(big != NULL);
    assert(circuit_pauli_evolution(circuit, big, 1.0, qubits, 2) == -3);

    pauli_string_free(big);
    pauli_string_free(pauli);
    circuit_free(circuit);
}

int main(void) {
    test_pauli_evolution_xz();
    test_pauli_evolution_errors();
    printf("binding-c circuit evolution tests passed\n");
    return 0;
}
