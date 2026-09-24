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

/* Bell state |Phi+> = (|00> + |11>) / sqrt(2) as a density matrix. */
static CDensityMatrix* bell_dm(void) {
    CDensityMatrix* dm = density_matrix_new(2);
    assert(dm != NULL);
    assert(density_matrix_apply_h(dm, 0) == 0);
    assert(density_matrix_apply_cx(dm, 0, 1) == 0);
    return dm;
}

/* Bell state as a statevector. */
static CStatevector* bell_sv(void) {
    CStatevector* sv = statevector_new(2);
    assert(sv != NULL);
    assert(statevector_apply_h(sv, 0) == 0);
    assert(statevector_apply_cx(sv, 0, 1) == 0);
    return sv;
}

static void test_entropy(void) {
    double out = NAN;

    /* Pure state: linear entropy ~ 0. */
    CDensityMatrix* bell = bell_dm();
    assert(density_matrix_linear_entropy(bell, &out) == 0);
    assert(fabs(out) < 1e-9);

    /* Pure state: Renyi entropy of any order ~ 0. */
    assert(density_matrix_renyi_entropy(bell, 2.0, &out) == 0);
    assert(fabs(out) < 1e-9);
    /* Invalid order is rejected. */
    assert(density_matrix_renyi_entropy(bell, 0.0, &out) == -8);

    /* Bell entanglement entropy = 1 bit. */
    CStatevector* sv = bell_sv();
    uint32_t subsys_a[1] = {0};
    assert(statevector_entanglement_entropy(sv, subsys_a, 1, &out) == 0);
    assert(fabs(out - 1.0) < 1e-9);

    /* Negativity = 0.5, concurrence = 1, E_F = 1 for the Bell state. */
    assert(density_matrix_negativity(bell, subsys_a, 1, &out) == 0);
    assert(fabs(out - 0.5) < 1e-9);
    assert(density_matrix_concurrence(bell, &out) == 0);
    assert(fabs(out - 1.0) < 1e-9);
    assert(density_matrix_entanglement_of_formation(bell, &out) == 0);
    assert(fabs(out - 1.0) < 1e-9);

    /* Separable pure state: concurrence = 0, E_F = 0. */
    CDensityMatrix* sep = density_matrix_new(2);
    assert(density_matrix_concurrence(sep, &out) == 0);
    assert(fabs(out) < 1e-9);
    assert(density_matrix_entanglement_of_formation(sep, &out) == 0);
    assert(fabs(out) < 1e-9);

    /* NULL handling. */
    assert(density_matrix_linear_entropy(NULL, &out) == -1);
    assert(density_matrix_negativity(bell, NULL, 1, &out) == -1);

    density_matrix_free(sep);
    statevector_free(sv);
    density_matrix_free(bell);
}

static void test_metrics(void) {
    double out = NAN;

    /* Purity of pure states is 1. */
    CStatevector* sv = bell_sv();
    assert(statevector_purity(sv, &out) == 0);
    assert(fabs(out - 1.0) < 1e-9);

    CDensityMatrix* bell = bell_dm();
    assert(density_matrix_purity(bell, &out) == 0);
    assert(fabs(out - 1.0) < 1e-9);

    /* Identical states: fidelity 1, trace distance 0. */
    CStatevector* sv2 = bell_sv();
    assert(statevector_fidelity(sv, sv2, &out) == 0);
    assert(fabs(out - 1.0) < 1e-9);
    assert(statevector_trace_distance(sv, sv2, &out) == 0);
    assert(fabs(out) < 1e-9);

    assert(statevector_fidelity_pure_mixed(sv, bell, &out) == 0);
    assert(fabs(out - 1.0) < 1e-9);

    CDensityMatrix* bell2 = bell_dm();
    assert(density_matrix_trace_distance(bell, bell2, &out) == 0);
    assert(fabs(out) < 1e-9);
    assert(density_matrix_fidelity(bell, bell2, &out) == 0);
    assert(fabs(out - 1.0) < 1e-9);

    /* Pure state: von Neumann entropy ~ 0. */
    assert(density_matrix_entropy(bell, &out) == 0);
    assert(fabs(out) < 1e-9);

    /* Logarithmic negativity of the Bell state = 1. */
    uint32_t sys_a[1] = {0};
    assert(density_matrix_logarithmic_negativity(bell, sys_a, 1, &out) == 0);
    assert(fabs(out - 1.0) < 1e-9);

    /* Partial transpose: new 2-qubit density matrix. */
    CDensityMatrix* pt = density_matrix_partial_transpose(bell, sys_a, 1);
    assert(pt != NULL);
    assert(density_matrix_num_qubits(pt) == 2);
    /* Eigenvalues (1/2, 1/2, 1/2, -1/2): purity stays 1. */
    assert(density_matrix_purity(pt, &out) == 0);
    assert(fabs(out - 1.0) < 1e-9);
    /* Out-of-bounds qubit yields NULL. */
    uint32_t oob[1] = {7};
    assert(density_matrix_partial_transpose(bell, oob, 1) == NULL);

    density_matrix_free(pt);
    density_matrix_free(bell2);
    density_matrix_free(bell);
    statevector_free(sv2);
    statevector_free(sv);
}

static CHamiltonian* hamiltonian_zz(double coeff) {
    CPauliString* zz = pauli_string_parse("ZZ");
    assert(zz != NULL);
    CHamiltonian* h = hamiltonian_new(2);
    assert(h != NULL);
    assert(hamiltonian_add_term(h, zz, coeff, 0.0) == 0);
    pauli_string_free(zz);
    return h;
}

static void test_evolution(void) {
    CHamiltonian* h = hamiltonian_zz(1.0);

    /* Trotter circuits for both modes. */
    CCircuit* circuit = hamiltonian_to_trotter_circuit(h, 1.0, 3, TROTTER_MODE_FIRST_ORDER);
    assert(circuit != NULL);
    assert(circuit_num_qubits(circuit) == 2);
    circuit_free(circuit);

    circuit = hamiltonian_to_trotter_circuit(h, 1.0, 3, TROTTER_MODE_SECOND_ORDER);
    assert(circuit != NULL);
    assert(circuit_num_qubits(circuit) == 2);
    circuit_free(circuit);

    /* Invalid mode tag and zero steps yield NULL. */
    assert(hamiltonian_to_trotter_circuit(h, 1.0, 3, 99) == NULL);
    assert(hamiltonian_to_trotter_circuit(h, 1.0, 0, TROTTER_MODE_FIRST_ORDER) == NULL);

    /* Auto-selecting evolution circuit (ZZ commutes: exact path). */
    circuit = hamiltonian_to_evolution_circuit(h, 1.0, 1, TROTTER_MODE_FIRST_ORDER);
    assert(circuit != NULL);
    assert(circuit_num_qubits(circuit) == 2);
    circuit_free(circuit);

    hamiltonian_free(h);
}

static void test_hamiltonian_supplements(void) {
    /* Scale: 0.5 * ZZ scaled by 2 equals ZZ. */
    CHamiltonian* h = hamiltonian_zz(0.5);
    assert(hamiltonian_scale(h, 2.0, 0.0) == 0);

    uintptr_t len = hamiltonian_to_matrix_len(h);
    assert(len == 4);

    Complex64 buf[16];
    assert(hamiltonian_to_matrix(h, buf, len * len) == 0);
    /* diag(1, -1, -1, 1) for ZZ. */
    assert(fabs(buf[0].re - 1.0) < 1e-9);
    assert(fabs(buf[5].re + 1.0) < 1e-9);
    assert(fabs(buf[10].re + 1.0) < 1e-9);
    assert(fabs(buf[15].re - 1.0) < 1e-9);
    assert(fabs(buf[1].re) < 1e-9 && fabs(buf[1].im) < 1e-9);

    /* Wrong buffer length is rejected. */
    assert(hamiltonian_to_matrix(h, buf, 15) == -8);
    assert(hamiltonian_to_matrix_len(NULL) == 0);

    /* All ZZ-type terms commute. */
    assert(hamiltonian_all_terms_commute(h) == 1);

    /* X + Z on one qubit does not commute. */
    CPauliString* px = pauli_string_parse("X");
    CPauliString* pz = pauli_string_parse("Z");
    CHamiltonian* h2 = hamiltonian_new(1);
    assert(hamiltonian_add_term(h2, px, 1.0, 0.0) == 0);
    assert(hamiltonian_add_term(h2, pz, 1.0, 0.0) == 0);
    assert(hamiltonian_all_terms_commute(h2) == 0);
    assert(hamiltonian_all_terms_commute(NULL) == -1);

    pauli_string_free(pz);
    pauli_string_free(px);
    hamiltonian_free(h2);
    hamiltonian_free(h);
}

static void test_pauli_string_supplements(void) {
    /* "XZI": qubit 2 = X, qubit 1 = Z, qubit 0 = I. */
    CPauliString* ps = pauli_string_parse("XZI");
    assert(ps != NULL);

    /* Masks: "XZI" has X on qubit 2, Z on qubit 1. */
    assert(pauli_string_x_mask(ps) == 0x4);
    assert(pauli_string_z_mask(ps) == 0x2);

    /* Support = [1, 2]. */
    uintptr_t support_len = pauli_string_support_len(ps);
    assert(support_len == 2);
    uint32_t support[2];
    assert(pauli_string_support(ps, support, 2) == 0);
    assert(support[0] == 1 && support[1] == 2);
    assert(pauli_string_support(ps, support, 3) == -8);

    /* try_get_pauli roundtrip. */
    uint8_t tag = 0xFF;
    assert(pauli_string_try_get_pauli(ps, 0, &tag) == 0);
    assert(tag == PAULI_I);
    assert(pauli_string_try_get_pauli(ps, 1, &tag) == 0);
    assert(tag == PAULI_Z);
    assert(pauli_string_try_get_pauli(ps, 2, &tag) == 0);
    assert(tag == PAULI_X);
    assert(pauli_string_try_get_pauli(ps, 3, &tag) == -2);

    /* Commutation: XZ vs ZX commutes, XI vs ZI anticommutes. */
    CPauliString* xz = pauli_string_parse("XZ");
    CPauliString* zx = pauli_string_parse("ZX");
    CPauliString* xi = pauli_string_parse("XI");
    CPauliString* zi = pauli_string_parse("ZI");
    assert(pauli_string_commutes_with(xz, zx) == 1);
    assert(pauli_string_commutes_with(xi, zi) == 0);
    /* Qubit-count mismatch. */
    assert(pauli_string_commutes_with(xz, ps) == -8);
    assert(pauli_string_commutes_with(NULL, xz) == -1);

    /* y_phase: "Y" contributes i, "XZ" contributes 1. */
    CPauliString* py = pauli_string_parse("Y");
    Complex64 phase = {0.0, 0.0};
    assert(pauli_string_y_phase(py, &phase) == 0);
    assert(fabs(phase.re) < 1e-9 && fabs(phase.im - 1.0) < 1e-9);
    assert(pauli_string_y_phase(xz, &phase) == 0);
    assert(fabs(phase.re - 1.0) < 1e-9 && fabs(phase.im) < 1e-9);

    /* try_set_pauli: set Y on qubit 0 and read it back. */
    CPauliString* editable = pauli_string_new(2);
    assert(pauli_string_try_set_pauli(editable, 0, PAULI_Y) == 0);
    assert(pauli_string_try_get_pauli(editable, 0, &tag) == 0);
    assert(tag == PAULI_Y);
    assert(pauli_string_try_set_pauli(editable, 5, PAULI_X) == -2);
    assert(pauli_string_try_set_pauli(editable, 0, 200) == -8);

    pauli_string_free(editable);
    pauli_string_free(py);
    pauli_string_free(zi);
    pauli_string_free(xi);
    pauli_string_free(zx);
    pauli_string_free(xz);
    pauli_string_free(ps);
}

int main(void) {
    test_entropy();
    test_metrics();
    test_evolution();
    test_hamiltonian_supplements();
    test_pauli_string_supplements();
    printf("binding-c qis metrics tests passed\n");
    return 0;
}
