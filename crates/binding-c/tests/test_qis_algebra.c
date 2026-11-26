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
#include <limits.h>
#include <math.h>
#include <stddef.h>
#include <stdio.h>

#include "cqlib_c.h"

_Static_assert(sizeof(Complex64) == 2 * sizeof(double), "complex layout");
_Static_assert(offsetof(Complex64, im) == sizeof(double), "complex imaginary offset");
_Static_assert(offsetof(CFermionicLadder, kind) == sizeof(size_t), "ladder tag offset");
_Static_assert(offsetof(CFermionicTerm, len) == sizeof(void*), "term length offset");
_Static_assert(offsetof(CHamiltonianTerm, coefficient) == sizeof(void*),
               "Pauli coefficient offset");

static void test_fermions(void) {
    CFermionicOp *a = NULL, *c = NULL, *ac = NULL, *ca = NULL;
    CFermionicOp *sum = NULL, *identity = NULL, *ordered = NULL, *copy = NULL;
    CFermionicOp *number = NULL, *total = NULL, *zero = NULL, *scaled = NULL;
    CFermionicOp *difference = NULL, *adjoint = NULL;
    uint8_t yes = 99;
    size_t n = 99;
    assert(fermionic_op_annihilation(1, 0, &a) == 0);
    assert(fermionic_op_creation(1, 0, &c) == 0);
    assert(fermionic_op_mul(a, c, &ac) == 0);
    assert(fermionic_op_mul(c, a, &ca) == 0);
    assert(fermionic_op_add(ac, ca, &sum) == 0);
    assert(fermionic_op_identity(1, &identity) == 0);
    assert(fermionic_op_equiv(sum, identity, 0, &yes) == 0 && yes == 1);
    assert(fermionic_op_equals(sum, identity, &yes) == 0 && yes == 0);
    assert(fermionic_op_normal_order(sum, &ordered) == 0);
    assert(fermionic_op_equals(ordered, identity, &yes) == 0 && yes == 1);
    assert(fermionic_op_num_modes(c, &n) == 0 && n == 1);
    assert(fermionic_op_num_terms(c, &n) == 0 && n == 1);
    assert(fermionic_op_term_len(c, 0, &n) == 0 && n == 1);
    CFermionicLadder buffer[2] = {{99, 99}, {99, 99}};
    Complex64 coefficient = {99, 99};
    assert(fermionic_op_get_term(c, 0, buffer, 0, &coefficient) == -8);
    assert(buffer[0].mode == 99 && coefficient.re == 99);
    assert(fermionic_op_get_term(c, 0, buffer, 2, &coefficient) == 0);
    assert(buffer[0].mode == 0 && buffer[0].kind == FERMIONIC_CREATE);
    assert(buffer[1].mode == 99);
    assert(fermionic_op_copy(c, &copy) == 0);
    assert(fermionic_op_chop(copy, 1) == 0);
    assert(fermionic_op_is_zero(copy, &yes) == 0 && yes == 1);
    assert(fermionic_op_number(1, 0, &number) == 0);
    assert(fermionic_op_total_number(1, &total) == 0);
    assert(fermionic_op_equiv(number, total, 0, &yes) == 0 && yes == 1);
    assert(fermionic_op_new(1, &zero) == 0);
    assert(fermionic_op_scaled(c, (Complex64){1e-20, 0}, &scaled) == 0);
    assert(fermionic_op_num_terms(scaled, &n) == 0 && n == 1);
    assert(fermionic_op_sub(c, c, &difference) == 0);
    assert(fermionic_op_equiv(difference, zero, 0, &yes) == 0 && yes == 1);
    assert(fermionic_op_adjoint(a, &adjoint) == 0);
    assert(fermionic_op_equals(adjoint, c, &yes) == 0 && yes == 1);
    assert(fermionic_op_is_hermitian(number, 0, &yes) == 0 && yes == 1);
    CFermionicLadder creator = {0, FERMIONIC_CREATE};
    CFermionicTerm terms[] = {{&creator, 1, {1, 0}}, {&creator, 1, {2, 0}}};
    CFermionicOp *batch = NULL, *single = NULL;
    assert(fermionic_op_from_terms(1, terms, 2, &batch) == 0);
    assert(fermionic_op_from_term(1, &creator, 1, (Complex64){3, 0}, &single) == 0);
    assert(fermionic_op_equiv(batch, single, 0, &yes) == 0 && yes == 1);
    assert(fermionic_op_add_term(batch, &creator, 1, (Complex64){-3, 0}) == 0);
    assert(fermionic_op_is_zero(batch, &yes) == 0 && yes == 1);
    CFermionicOp* out = c;
    assert(fermionic_op_creation(1, 1, &out) == -2 && out == c);
    creator.kind = 255;
    assert(fermionic_op_from_term(1, &creator, 1, (Complex64){0, 0}, &out) == -8 && out == c);
    assert(fermionic_op_scaled(c, (Complex64){NAN, 0}, &out) == -8 && out == c);
    assert(fermionic_op_chop(c, -1) == -8);
    assert(fermionic_op_equals(c, adjoint, &yes) == 0 && yes == 1);
    fermionic_op_free(batch);
    fermionic_op_free(single);
    fermionic_op_free(a);
    fermionic_op_free(c);
    fermionic_op_free(ac);
    fermionic_op_free(ca);
    fermionic_op_free(sum);
    fermionic_op_free(identity);
    fermionic_op_free(ordered);
    fermionic_op_free(copy);
    fermionic_op_free(number);
    fermionic_op_free(total);
    fermionic_op_free(zero);
    fermionic_op_free(scaled);
    fermionic_op_free(difference);
    fermionic_op_free(adjoint);
    fermionic_op_free(NULL);
}

static void test_pauli_algebra(void) {
    CPauliString *px = pauli_string_parse("X"), *pz = pauli_string_parse("Z");
    CPauliString *py = pauli_string_parse("Y"), *pxz = pauli_string_parse("XZ");
    assert(px && pz && py && pxz);
    CHamiltonianTerm tx = {px, {1, 0}}, tz = {pz, {1, 0}};
    CHamiltonianTerm ty = {py, {0, -1}}, txz = {pxz, {1, 0}};
    CHamiltonian *x = NULL, *z = NULL, *y = NULL, *expected = NULL;
    CHamiltonian *product = NULL, *tensor = NULL, *sum = NULL, *difference = NULL;
    CHamiltonian *adjoint = NULL, *scaled = NULL, *copy = NULL, *normalized = NULL;
    CHamiltonian *hermitian = NULL, *remapped = NULL, *identity = NULL;
    uint8_t yes = 99;
    assert(hamiltonian_from_terms(1, &tx, 1, &x) == 0);
    assert(hamiltonian_from_terms(1, &tz, 1, &z) == 0);
    assert(hamiltonian_from_terms(1, &ty, 1, &y) == 0);
    assert(hamiltonian_from_terms(2, &txz, 1, &expected) == 0);
    pauli_string_free(px);
    pauli_string_free(pz);
    pauli_string_free(py);
    pauli_string_free(pxz);
    assert(hamiltonian_mul(x, z, &product) == 0);
    assert(hamiltonian_equiv(product, y, 0, &yes) == 0 && yes == 1);
    assert(hamiltonian_tensor(x, z, &tensor) == 0);
    assert(hamiltonian_equiv(tensor, expected, 0, &yes) == 0 && yes == 1);
    assert(hamiltonian_add(x, z, &sum) == 0);
    assert(hamiltonian_sub(sum, z, &difference) == 0);
    assert(hamiltonian_equiv(difference, x, 0, &yes) == 0 && yes == 1);
    assert(hamiltonian_adjoint(product, &adjoint) == 0);
    assert(hamiltonian_scaled(product, (Complex64){-1, 0}, &scaled) == 0);
    assert(hamiltonian_equiv(adjoint, scaled, 0, &yes) == 0 && yes == 1);
    assert(hamiltonian_copy(x, &copy) == 0);
    assert(hamiltonian_chop(copy, 1) == 0 && hamiltonian_num_terms(copy) == 0);
    assert(hamiltonian_num_terms(x) == 1);
    assert(hamiltonian_normalized(x, &normalized) == 0);
    assert(hamiltonian_normalized_hermitian(x, &hermitian) == 0);
    assert(hamiltonian_is_hermitian(product, 0, &yes) == 0 && yes == 0);
    assert(hamiltonian_is_hermitian(hermitian, 0, &yes) == 0 && yes == 1);
    const size_t mapping[] = {1};
    assert(hamiltonian_remap_qubits(x, mapping, 1, 2, &remapped) == 0);
    assert(hamiltonian_identity(0, &identity) == 0);
    CPauliString* snapshot = NULL;
    Complex64 coefficient = {99, 99};
    assert(hamiltonian_get_term(identity, 0, &snapshot, &coefficient) == 0);
    hamiltonian_free(identity);
    assert(pauli_string_num_qubits(snapshot) == 0 && coefficient.re == 1);
    pauli_string_free(snapshot);
    CHamiltonian* out = x;
    assert(hamiltonian_add(x, tensor, &out) == -8 && out == x);
    assert(hamiltonian_normalized_hermitian(product, &out) == -8 && out == x);
    assert(hamiltonian_identity(SIZE_MAX, &out) == -7 && out == x);
    assert(hamiltonian_equiv(x, z, NAN, &yes) == -8 && yes == 1);
    hamiltonian_free(x);
    hamiltonian_free(z);
    hamiltonian_free(y);
    hamiltonian_free(expected);
    hamiltonian_free(product);
    hamiltonian_free(tensor);
    hamiltonian_free(sum);
    hamiltonian_free(difference);
    hamiltonian_free(adjoint);
    hamiltonian_free(scaled);
    hamiltonian_free(copy);
    hamiltonian_free(normalized);
    hamiltonian_free(hermitian);
    hamiltonian_free(remapped);
}

static void test_mapping_and_evolution(void) {
    size_t sources[] = {0}, targets[] = {1}, width = 99;
    CFermionicOp* generator = NULL;
    CHamiltonian* mapped = NULL;
    assert(fermionic_op_excitation_generator(2, sources, 1, targets, 1, &generator) == 0);
    assert(jordan_wigner_num_qubits(2, &width) == 0 && width == 2);
    assert(jordan_wigner_map_operator(generator, &mapped) == 0);
    uint8_t occupation[] = {1, 0}, bits[] = {99, 99};
    assert(jordan_wigner_map_occupation(occupation, 2, bits, 2) == 0 && bits[0] == 1 &&
           bits[1] == 0);
    occupation[1] = 2;
    assert(jordan_wigner_map_occupation(occupation, 2, bits, 2) == -8 && bits[0] == 1 &&
           bits[1] == 0);
    assert(jordan_wigner_map_occupation(NULL, 0, NULL, 0) == 0);
    const double theta = 0.37;
    CCircuit* evolution =
        hamiltonian_to_evolution_circuit(mapped, theta, 1, TROTTER_MODE_SECOND_ORDER);
    assert(evolution);
    Complex64 state[] = {{0, 0}, {1, 0}, {0, 0}, {0, 0}}, data[4];
    CStatevector* sv = statevector_from_state(2, state, 4);
    assert(sv);
    assert(statevector_apply_circuit(sv, evolution) == 0);
    assert(statevector_data(sv, data, 4) == 0);
    assert(fabs(data[1].re - cos(theta)) < 1e-12 && fabs(data[1].im) < 1e-12);
    assert(fabs(data[2].re - sin(theta)) < 1e-12 && fabs(data[2].im) < 1e-12);
    assert(hypot(data[0].re, data[0].im) < 1e-12 && hypot(data[3].re, data[3].im) < 1e-12);
    statevector_free(sv);
    circuit_free(evolution);
    hamiltonian_free(mapped);
    fermionic_op_free(generator);
    CCircuit* phase = circuit_new(1);
    assert(circuit_set_global_phase(phase, theta) == 0);
    sv = statevector_from_circuit(phase);
    assert(sv && statevector_data(sv, data, 2) == 0);
    assert(fabs(data[0].re - cos(theta)) < 1e-12 && fabs(data[0].im - sin(theta)) < 1e-12);
    statevector_free(sv);
    circuit_free(phase);
}

static void test_wide_sparse_registers(void) {
    const size_t bits = sizeof(size_t) * CHAR_BIT;
    const size_t widths[] = {bits, bits + 1, bits - 1, bits / 2, bits / 2 - 1, bits / 2 - 2};
    for (size_t i = 0; i < sizeof(widths) / sizeof(widths[0]); ++i) {
        CFermionicOp* identity = NULL;
        CHamiltonian* mapped = NULL;
        CPauliString* pauli = NULL;
        Complex64 coefficient = {0, 0}, buffer = {99, -99};
        assert(fermionic_op_identity(widths[i], &identity) == 0);
        assert(jordan_wigner_map_operator(identity, &mapped) == 0);
        assert(hamiltonian_get_term(mapped, 0, &pauli, &coefficient) == 0);
        const size_t dim = widths[i] < bits ? (size_t)1 << widths[i] : 0;
        assert(hamiltonian_to_matrix_len(mapped) == dim);
        assert(pauli_string_matrix_len(pauli) == dim);
        assert(hamiltonian_to_matrix(mapped, &buffer, 1) == -7);
        assert(buffer.re == 99 && buffer.im == -99);
        assert(pauli_string_matrix(pauli, &buffer, 1) == -7);
        assert(buffer.re == 99 && buffer.im == -99);
        pauli_string_free(pauli);
        hamiltonian_free(mapped);
        fermionic_op_free(identity);
    }
    CFermionicOp* zero = NULL;
    CHamiltonian* mapped = NULL;
    assert(fermionic_op_new(SIZE_MAX, &zero) == 0);
    assert(jordan_wigner_map_operator(zero, &mapped) == 0);
    assert(hamiltonian_to_matrix_len(mapped) == 0);
    Complex64 buffer = {99, -99};
    assert(hamiltonian_to_matrix(mapped, &buffer, 1) == -7);
    assert(buffer.re == 99 && buffer.im == -99);
    hamiltonian_free(mapped);
    fermionic_op_free(zero);
}

int main(void) {
    test_fermions();
    test_pauli_algebra();
    test_mapping_and_evolution();
    test_wide_sparse_registers();
    puts("binding-c checked algebra tests passed");
    return 0;
}
