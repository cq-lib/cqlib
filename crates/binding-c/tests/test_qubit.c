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
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "cqlib_c.h"

static void test_qubit_basics(void) {
    CQubit q = qubit_new(12);
    assert(q.id == 12);
    assert(qubit_id(q) == 12u);
    assert(qubit_index(q) == 12u);

    assert(qubit_equal(q, qubit_new(12)) == 1);
    assert(qubit_equal(q, qubit_new(11)) == 0);

    assert(qubit_compare(qubit_new(0), qubit_new(1)) == -1);
    assert(qubit_compare(qubit_new(7), qubit_new(7)) == 0);
    assert(qubit_compare(qubit_new(2), qubit_new(1)) == 1);

    char* text = qubit_to_string(q);
    assert(text != NULL && strcmp(text, "Q12") == 0);
    cqlib_string_free(text);
}

static void test_qubit_checked_conversions(void) {
    CQubit out = {0};

    // Negative or out-of-range signed values are rejected.
    assert(qubit_try_from_i64(-1, &out) == -8);
    assert(qubit_try_from_i64((int64_t)UINT32_MAX + 1, &out) == -8);
    assert(qubit_try_from_i64(3, &out) == 0);
    assert(out.id == 3u);

    // Unsigned values above UINT32_MAX are rejected.
    assert(qubit_try_from_u64((uint64_t)UINT32_MAX + 1, &out) == -8);
    assert(qubit_try_from_u64(20, &out) == 0);
    assert(out.id == 20u);

    // NULL out buffer.
    assert(qubit_try_from_i64(0, NULL) == -1);
    assert(qubit_try_from_u64(0, NULL) == -1);
}

static void test_logical_qubit_roundtrip(void) {
    CQubit wire = qubit_new(3);
    CLogicalQubit logical = logical_qubit_from_qubit(wire);
    assert(logical.id == 3u);
    assert(logical_qubit_id(logical) == 3u);

    CQubit back = logical_qubit_qubit(logical);
    assert(back.id == 3u);

    assert(logical_qubit_equal(logical, logical_qubit_new(3)) == 1);
    assert(logical_qubit_equal(logical, logical_qubit_new(2)) == 0);
    assert(logical_qubit_compare(logical_qubit_new(2), logical_qubit_new(0)) == 1);

    char* text = logical_qubit_to_string(logical);
    assert(text != NULL && strcmp(text, "L3") == 0);
    cqlib_string_free(text);
}

static void test_physical_qubit_roundtrip(void) {
    CQubit wire = qubit_new(11);
    CPhysicalQubit physical = physical_qubit_from_qubit(wire);
    assert(physical.id == 11u);
    assert(physical_qubit_id(physical) == 11u);

    CQubit back = physical_qubit_qubit(physical);
    assert(back.id == 11u);

    assert(physical_qubit_equal(physical, physical_qubit_new(11)) == 1);
    assert(physical_qubit_equal(physical, physical_qubit_new(10)) == 0);
    assert(physical_qubit_compare(physical_qubit_new(0), physical_qubit_new(2)) == -1);

    char* text = physical_qubit_to_string(physical);
    assert(text != NULL && strcmp(text, "P11") == 0);
    cqlib_string_free(text);
}

int main(void) {
    test_qubit_basics();
    test_qubit_checked_conversions();
    test_logical_qubit_roundtrip();
    test_physical_qubit_roundtrip();
    printf("binding-c qubit tests passed\n");
    return 0;
}
