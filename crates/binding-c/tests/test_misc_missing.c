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
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "cqlib_c.h"

static int close_to(double a, double b) { return fabs(a - b) < 1e-9; }

/* |Phi+> = (|00> + |11>) / sqrt(2). */
static CStatevector* make_bell_state(void) {
    CStatevector* sv = statevector_new(2);
    assert(sv != NULL);
    assert(statevector_apply_h(sv, 0) == 0);
    assert(statevector_apply_cx(sv, 0, 1) == 0);
    return sv;
}

static CHamiltonian* make_zz_hamiltonian(void) {
    CPauliString* zz = pauli_string_parse("ZZ");
    assert(zz != NULL);
    CHamiltonian* ham = hamiltonian_new(2);
    assert(ham != NULL);
    assert(hamiltonian_add_term(ham, zz, 1.0, 0.0) == 0);
    pauli_string_free(zz);
    return ham;
}

static void test_hamiltonian_expectation_probs(void) {
    CStatevector* sv = make_bell_state();
    CHamiltonian* ham = make_zz_hamiltonian();

    /* Full computational-basis distribution of the Bell state. */
    CPauliString* basis = pauli_string_parse("ZZ");
    assert(basis != NULL);
    const char* states[4] = {"00", "01", "10", "11"};
    double probs[4] = {0.5, 0.0, 0.0, 0.5};
    CProbMeasurement measurement;
    measurement.basis = basis;
    measurement.states = states;
    measurement.probs = probs;
    measurement.len = 4;

    /* <ZZ> = 0.5*(+1) + 0.5*(+1) = 1.0 on the Bell state. */
    double out = NAN;
    assert(hamiltonian_expectation_probs(ham, &measurement, 1, &out) == 0);
    assert(close_to(out, 1.0));

    /* Only the observed (non-zero) outcomes may be supplied; the outcome
     * eigenvalues of "00" and "11" under Z⊗Z are both +1. */
    const char* observed[2] = {"00", "11"};
    double observed_probs[2] = {0.5, 0.5};
    measurement.states = observed;
    measurement.probs = observed_probs;
    measurement.len = 2;
    assert(hamiltonian_expectation_probs(ham, &measurement, 1, &out) == 0);
    assert(close_to(out, 1.0));

    /* A term whose support is contained in the measured basis: Z on qubit 1
     * evaluated in the ZZ basis gives <Z1> = 0 on the Bell state. */
    CPauliString* zi = pauli_string_parse("ZI");
    assert(zi != NULL);
    CHamiltonian* ham_zi = hamiltonian_new(2);
    assert(ham_zi != NULL);
    assert(hamiltonian_add_term(ham_zi, zi, 1.0, 0.0) == 0);
    measurement.states = states;
    measurement.probs = probs;
    measurement.len = 4;
    assert(hamiltonian_expectation_probs(ham_zi, &measurement, 1, &out) == 0);
    assert(close_to(out, 0.0));
    hamiltonian_free(ham_zi);
    pauli_string_free(zi);

    /* Errors: NULL out, NULL measurements with a non-zero count, NULL basis,
     * and an empty measurement list for a non-identity Hamiltonian (-7). */
    assert(hamiltonian_expectation_probs(ham, &measurement, 1, NULL) == -1);
    assert(hamiltonian_expectation_probs(ham, NULL, 1, &out) == -1);
    CProbMeasurement bad = measurement;
    bad.basis = NULL;
    assert(hamiltonian_expectation_probs(ham, &bad, 1, &out) == -1);
    assert(hamiltonian_expectation_probs(ham, NULL, 0, &out) == -7);

    pauli_string_free(basis);
    hamiltonian_free(ham);
    statevector_free(sv);
    statevector_free(NULL);
}

static void test_hamiltonian_variance_statevector(void) {
    CStatevector* sv = make_bell_state();

    /* The Bell state is an eigenstate of Z⊗Z: <O^2> = <O>^2 = 1, so the
     * variance is 0. */
    CHamiltonian* ham = make_zz_hamiltonian();
    double variance = NAN;
    assert(hamiltonian_variance_statevector(ham, sv, &variance) == 0);
    assert(close_to(variance, 0.0));
    hamiltonian_free(ham);

    /* Z on qubit 1 is not sharp: <O> = 0 and <O^2> = 1, so the variance
     * is 1. */
    CPauliString* zi = pauli_string_parse("ZI");
    assert(zi != NULL);
    CHamiltonian* ham_zi = hamiltonian_new(2);
    assert(ham_zi != NULL);
    assert(hamiltonian_add_term(ham_zi, zi, 1.0, 0.0) == 0);
    assert(hamiltonian_variance_statevector(ham_zi, sv, &variance) == 0);
    assert(close_to(variance, 1.0));
    hamiltonian_free(ham_zi);
    pauli_string_free(zi);

    /* Errors: NULL pointers and a qubit-count mismatch (-8). */
    CPauliString* zzz = pauli_string_parse("ZZZ");
    assert(zzz != NULL);
    CHamiltonian* ham3 = hamiltonian_new(3);
    assert(ham3 != NULL);
    assert(hamiltonian_add_term(ham3, zzz, 1.0, 0.0) == 0);
    assert(hamiltonian_variance_statevector(ham3, sv, &variance) == -8);
    pauli_string_free(zzz);
    hamiltonian_free(ham3);

    assert(hamiltonian_variance_statevector(NULL, sv, &variance) == -1);
    CHamiltonian* ham_tmp = make_zz_hamiltonian();
    assert(hamiltonian_variance_statevector(ham_tmp, NULL, &variance) == -1);
    assert(hamiltonian_variance_statevector(ham_tmp, sv, NULL) == -1);
    hamiltonian_free(ham_tmp);

    statevector_free(sv);
}

static void test_local_bloch_vectors(void) {
    CStatevector* sv = make_bell_state();

    /* Each qubit of the Bell state is maximally mixed: all components 0. */
    assert(local_bloch_vectors_len(sv) == 6);
    double bloch[6];
    assert(local_bloch_vectors(sv, bloch, 6) == 0);
    for (int i = 0; i < 6; i++) {
        assert(close_to(bloch[i], 0.0));
    }

    /* Product state |+> on qubit 0 and |0> on qubit 1. */
    CStatevector* product = statevector_new(2);
    assert(product != NULL);
    assert(statevector_apply_h(product, 0) == 0);
    assert(local_bloch_vectors(product, bloch, 6) == 0);
    assert(close_to(bloch[0], 1.0)); /* x0 */
    assert(close_to(bloch[1], 0.0)); /* y0 */
    assert(close_to(bloch[2], 0.0)); /* z0 */
    assert(close_to(bloch[3], 0.0)); /* x1 */
    assert(close_to(bloch[4], 0.0)); /* y1 */
    assert(close_to(bloch[5], 1.0)); /* z1 */
    statevector_free(product);

    /* Errors: wrong length (-8) and NULL pointers (-1). */
    assert(local_bloch_vectors(sv, bloch, 5) == -8);
    assert(local_bloch_vectors(sv, NULL, 6) == -1);
    assert(local_bloch_vectors(NULL, bloch, 6) == -1);
    assert(local_bloch_vectors_len(NULL) == 0);

    statevector_free(sv);
}

static void test_state_to_density_matrix(void) {
    CStatevector* sv = make_bell_state();

    /* 4^2 = 16 entries forming the 4x4 density matrix of the Bell state:
     * 0.5 on the diagonal corners and on the (0,3)/(3,0) coherences. */
    assert(state_to_density_matrix_len(sv) == 16);
    Complex64 matrix[16];
    assert(state_to_density_matrix(sv, matrix, 16) == 0);
    for (int row = 0; row < 4; row++) {
        for (int col = 0; col < 4; col++) {
            const Complex64 entry = matrix[row * 4 + col];
            const int corner = (row == 0 || row == 3) && (col == 0 || col == 3);
            assert(close_to(entry.re, corner ? 0.5 : 0.0));
            assert(close_to(entry.im, 0.0));
        }
    }

    /* Errors: wrong length (-8) and NULL pointers (-1). */
    assert(state_to_density_matrix(sv, matrix, 15) == -8);
    assert(state_to_density_matrix(sv, NULL, 16) == -1);
    assert(state_to_density_matrix(NULL, matrix, 16) == -1);
    assert(state_to_density_matrix_len(NULL) == 0);

    statevector_free(sv);
}

int main(void) {
    test_hamiltonian_expectation_probs();
    test_hamiltonian_variance_statevector();
    test_local_bloch_vectors();
    test_state_to_density_matrix();
    printf("binding-c misc missing tests passed\n");
    return 0;
}
