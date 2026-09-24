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
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>

#include "cqlib_c.h"

static int near(double a, double b) { return fabs(a - b) < 1e-10; }

static void test_statevector_data_mut(void) {
    CStatevector* sv = statevector_new(1);
    assert(sv != NULL);
    assert(statevector_data_len(sv) == 2);

    /* Overwrite |0> with |1>, then read the amplitudes back. */
    Complex64 one[2] = {{0.0, 0.0}, {1.0, 0.0}};
    assert(statevector_data_mut(sv, one, 2) == 0);
    Complex64 amps[2];
    assert(statevector_data(sv, amps, 2) == 0);
    assert(near(amps[0].re, 0.0) && near(amps[0].im, 0.0));
    assert(near(amps[1].re, 1.0) && near(amps[1].im, 0.0));

    /* The simulator keeps operating on the written data: Z|1> = -|1>. */
    assert(statevector_apply_z(sv, 0) == 0);
    assert(statevector_data(sv, amps, 2) == 0);
    assert(near(amps[0].re, 0.0));
    assert(near(amps[1].re, -1.0) && near(amps[1].im, 0.0));

    /* A normalized superposition can also be written directly. */
    double inv = 1.0 / sqrt(2.0);
    Complex64 plus[2] = {{inv, 0.0}, {inv, 0.0}};
    assert(statevector_data_mut(sv, plus, 2) == 0);
    assert(statevector_data(sv, amps, 2) == 0);
    assert(near(amps[0].re, inv) && near(amps[1].re, inv));

    /* Length mismatch is rejected; NULL pointers return -1. */
    assert(statevector_data_mut(sv, one, 1) == -8);
    assert(statevector_data_mut(NULL, one, 2) == -1);
    assert(statevector_data_mut(sv, NULL, 2) == -1);
    statevector_free(sv);
}

static void test_density_matrix_psd(void) {
    bool positive = false;

    /* Ground state |0><0| is positive semidefinite. */
    CDensityMatrix* dm = density_matrix_new(1);
    assert(dm != NULL);
    assert(density_matrix_is_positive_semidefinite(dm, 1e-10, &positive) == 0);
    assert(positive);
    density_matrix_free(dm);

    /* Maximally mixed state I/4 is positive semidefinite. */
    CDensityMatrix* mixed = density_matrix_maximally_mixed(2);
    assert(mixed != NULL);
    assert(density_matrix_is_positive_semidefinite(mixed, 1e-10, &positive) == 0);
    assert(positive);
    density_matrix_free(mixed);

    /* Pure |+><+| state is positive semidefinite. */
    double inv = 1.0 / sqrt(2.0);
    Complex64 plus[2] = {{inv, 0.0}, {inv, 0.0}};
    CDensityMatrix* pure = density_matrix_from_state(1, plus, 2);
    assert(pure != NULL);
    assert(density_matrix_is_positive_semidefinite(pure, 1e-10, &positive) == 0);
    assert(positive);
    density_matrix_free(pure);

    /* The zero matrix has zero eigenvalues and also passes. */
    CDensityMatrix* zero = density_matrix_zeros(1);
    assert(zero != NULL);
    assert(density_matrix_is_positive_semidefinite(zero, 1e-10, &positive) == 0);
    assert(positive);
    density_matrix_free(zero);

    /* Error paths: NULL handles and non-finite tolerance. */
    assert(density_matrix_is_positive_semidefinite(NULL, 1e-10, &positive) == -1);
    dm = density_matrix_new(1);
    assert(density_matrix_is_positive_semidefinite(dm, 1e-10, NULL) == -1);
    assert(density_matrix_is_positive_semidefinite(dm, NAN, &positive) == -8);
    density_matrix_free(dm);
}

static void test_pauli_mul_with_phase(void) {
    uint8_t tag = 0xFF;
    Complex64 phase = {9.0, 9.0};

    /* X * Z = -iY. */
    assert(pauli_mul_with_phase(PAULI_X, PAULI_Z, &tag, &phase) == 0);
    assert(tag == PAULI_Y);
    assert(near(phase.re, 0.0) && near(phase.im, -1.0));

    /* Z * X = iY. */
    assert(pauli_mul_with_phase(PAULI_Z, PAULI_X, &tag, &phase) == 0);
    assert(tag == PAULI_Y);
    assert(near(phase.re, 0.0) && near(phase.im, 1.0));

    /* X * X = I with phase +1. */
    assert(pauli_mul_with_phase(PAULI_X, PAULI_X, &tag, &phase) == 0);
    assert(tag == PAULI_I);
    assert(near(phase.re, 1.0) && near(phase.im, 0.0));

    /* X * Y = iZ, Y * Z = iX, Z * Y = -iX, Y * X = -iZ. */
    assert(pauli_mul_with_phase(PAULI_X, PAULI_Y, &tag, &phase) == 0);
    assert(tag == PAULI_Z && near(phase.im, 1.0));
    assert(pauli_mul_with_phase(PAULI_Y, PAULI_Z, &tag, &phase) == 0);
    assert(tag == PAULI_X && near(phase.im, 1.0));
    assert(pauli_mul_with_phase(PAULI_Z, PAULI_Y, &tag, &phase) == 0);
    assert(tag == PAULI_X && near(phase.im, -1.0));
    assert(pauli_mul_with_phase(PAULI_Y, PAULI_X, &tag, &phase) == 0);
    assert(tag == PAULI_Z && near(phase.im, -1.0));

    /* Identity leaves the other operator untouched. */
    assert(pauli_mul_with_phase(PAULI_I, PAULI_Y, &tag, &phase) == 0);
    assert(tag == PAULI_Y && near(phase.re, 1.0));
    assert(pauli_mul_with_phase(PAULI_Y, PAULI_I, &tag, &phase) == 0);
    assert(tag == PAULI_Y && near(phase.re, 1.0));

    /* Invalid tags and NULL output pointers are rejected. */
    assert(pauli_mul_with_phase(9, PAULI_X, &tag, &phase) == -8);
    assert(pauli_mul_with_phase(PAULI_X, 0xFF, &tag, &phase) == -8);
    assert(pauli_mul_with_phase(PAULI_X, PAULI_Z, NULL, &phase) == -1);
    assert(pauli_mul_with_phase(PAULI_X, PAULI_Z, &tag, NULL) == -1);
}

int main(void) {
    test_statevector_data_mut();
    test_density_matrix_psd();
    test_pauli_mul_with_phase();
    printf("binding-c parity qis3 tests passed\n");
    return 0;
}
