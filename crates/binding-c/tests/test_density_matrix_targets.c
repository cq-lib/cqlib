// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// Licensed under the Apache License, Version 2.0. See LICENSE.txt or
// http://www.apache.org/licenses/LICENSE-2.0.

#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

#include "cqlib_c.h"

int main(void) {
    Complex64 amplitudes[4];
    Complex64 operator[16];
    const double vector[4] = {4.0, -1.0, 2.0, 3.0};
    for (int i = 0; i < 4; ++i) {
        amplitudes[i] = (Complex64){(i + 1) / sqrt(30.0), 0.0};
        for (int j = 0; j < 4; ++j) {
            operator[4 * i + j] =
                (Complex64){(i == j ? 1.0 : 0.0) - 2.0 * vector[i] * vector[j] / 30.0, 0.0};
        }
    }
    CDensityMatrix* dm = density_matrix_from_state(2, amplitudes, 4);
    assert(dm != NULL);
    Complex64 before[16], after[16];
    assert(density_matrix_data(dm, before, 16) == 0);
    for (uint32_t q = 0; q < 2; ++q) {
        const uint32_t targets[2] = {q, q};
        /* Check Kraus first: a panic must fail the host-process test. */
        assert(density_matrix_apply_kraus(dm, operator, 1, 4, targets, 2) == -8);
        assert(density_matrix_data(dm, after, 16) == 0);
        assert(memcmp(before, after, sizeof(before)) == 0);
        assert(density_matrix_apply_cx(dm, q, q) == -8);
        assert(density_matrix_apply_cz(dm, q, q) == -8);
        assert(density_matrix_data(dm, after, 16) == 0);
        assert(memcmp(before, after, sizeof(before)) == 0);
    }
    /* A valid channel must still work after the rejected operations. */
    const uint32_t targets[2] = {0, 1};
    assert(density_matrix_apply_kraus(dm, operator, 1, 4, targets, 2) == 0);
    assert(density_matrix_data(dm, after, 16) == 0);
    double evolved[4] = {0};
    for (int i = 0; i < 4; ++i) {
        for (int j = 0; j < 4; ++j) {
            evolved[i] += operator[4 * i + j].re * amplitudes[((j & 1) << 1) | (j >> 1)].re;
        }
    }
    for (int i = 0; i < 4; ++i) {
        for (int j = 0; j < 4; ++j) {
            const int row = ((i & 1) << 1) | (i >> 1);
            const int col = ((j & 1) << 1) | (j >> 1);
            assert(fabs(after[4 * row + col].re - evolved[i] * evolved[j]) < 1e-12);
            assert(fabs(after[4 * row + col].im) < 1e-12);
        }
    }
    density_matrix_free(dm);
    puts("density matrix target validation passed");
    return 0;
}
