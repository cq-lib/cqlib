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

use binding_c::{circuit::*, qis::*};
use num_complex::Complex64 as C;
use std::ptr::{null, null_mut};

struct F(*mut CFermionicOp);
impl F {
    fn make(call: impl FnOnce(*mut *mut CFermionicOp) -> i32) -> Self {
        let mut out = null_mut();
        assert_eq!(call(&mut out), 0);
        assert!(!out.is_null());
        Self(out)
    }
    fn mapped(&self) -> H {
        H::make(|out| jordan_wigner_map_operator(self.0, out))
    }
    fn equiv(&self, other: &Self) -> bool {
        let mut result = 99;
        assert_eq!(fermionic_op_equiv(self.0, other.0, 0.0, &mut result), 0);
        result == 1
    }
}
impl Drop for F {
    fn drop(&mut self) {
        fermionic_op_free(self.0);
    }
}
struct H(*mut CHamiltonian);
impl H {
    fn make(call: impl FnOnce(*mut *mut CHamiltonian) -> i32) -> Self {
        let mut out = null_mut();
        assert_eq!(call(&mut out), 0);
        assert!(!out.is_null());
        Self(out)
    }
    fn from_pauli(text: &std::ffi::CStr, coefficient: C) -> Self {
        let p = pauli_string_parse(text.as_ptr());
        assert!(!p.is_null());
        let terms = [CHamiltonianTerm {
            pauli: p,
            coefficient,
        }];
        let h = Self::make(|out| {
            hamiltonian_from_terms(pauli_string_num_qubits(p), terms.as_ptr(), 1, out)
        });
        pauli_string_free(p);
        h
    }
    fn equiv(&self, other: &Self) -> bool {
        let mut result = 99;
        assert_eq!(hamiltonian_equiv(self.0, other.0, 0.0, &mut result), 0);
        result == 1
    }
    fn matrix(&self) -> Vec<C> {
        let dim = hamiltonian_to_matrix_len(self.0);
        let mut result = vec![C::default(); dim * dim];
        assert_eq!(
            hamiltonian_to_matrix(self.0, result.as_mut_ptr(), result.len()),
            0
        );
        result
    }
}
impl Drop for H {
    fn drop(&mut self) {
        hamiltonian_free(self.0);
    }
}

#[test]
fn empty_spaces_and_owned_snapshots() {
    let zero = F::make(|out| fermionic_op_from_terms(0, null(), 0, out));
    let identity = F::make(|out| fermionic_op_from_term(0, null(), 0, C::new(1.0, 0.0), out));
    let total = F::make(|out| fermionic_op_total_number(0, out));
    assert!(zero.equiv(&total));
    assert!(!zero.equiv(&identity));
    let mut n = 99;
    assert_eq!(fermionic_op_num_modes(zero.0, &mut n), 0);
    assert_eq!(n, 0);
    assert_eq!(fermionic_op_num_terms(identity.0, &mut n), 0);
    assert_eq!(n, 1);
    assert_eq!(fermionic_op_term_len(identity.0, 0, &mut n), 0);
    assert_eq!(n, 0);
    let mut coefficient = C::default();
    assert_eq!(
        fermionic_op_get_term(identity.0, 0, null_mut(), 0, &mut coefficient),
        0
    );
    assert_eq!(coefficient, C::new(1.0, 0.0));
    let h = identity.mapped();
    let mut pauli = null_mut();
    assert_eq!(
        hamiltonian_get_term(h.0, 0, &mut pauli, &mut coefficient),
        0
    );
    drop(h);
    assert_eq!(pauli_string_num_qubits(pauli), 0);
    pauli_string_free(pauli);
    let scalar = H::make(|out| hamiltonian_identity(0, out));
    assert_eq!(scalar.matrix(), vec![C::new(1.0, 0.0)]);
    let huge_zero = F::make(|out| fermionic_op_new(usize::MAX, out));
    let mapped = huge_zero.mapped();
    assert_eq!(hamiltonian_num_qubits(mapped.0), usize::MAX);
    let huge_identity = F::make(|out| fermionic_op_identity(usize::MAX, out));
    let mut out = scalar.0;
    assert_eq!(jordan_wigner_map_operator(huge_identity.0, &mut out), -7);
    assert_eq!(out, scalar.0);
    assert_eq!(hamiltonian_identity(usize::MAX, &mut out), -7);
    assert_eq!(out, scalar.0);
}

#[test]
fn anticommutation_normal_order_and_structural_equality() {
    let a = F::make(|out| fermionic_op_annihilation(2, 0, out));
    let creator = F::make(|out| fermionic_op_creation(2, 0, out));
    let ac = F::make(|out| fermionic_op_mul(a.0, creator.0, out));
    let ca = F::make(|out| fermionic_op_mul(creator.0, a.0, out));
    let sum = F::make(|out| fermionic_op_add(ac.0, ca.0, out));
    let identity = F::make(|out| fermionic_op_identity(2, out));
    assert!(sum.equiv(&identity));
    let mut same = 99;
    assert_eq!(fermionic_op_equals(sum.0, identity.0, &mut same), 0);
    assert_eq!(same, 0);
    let ordered = F::make(|out| fermionic_op_normal_order(sum.0, out));
    assert_eq!(fermionic_op_equals(ordered.0, identity.0, &mut same), 0);
    assert_eq!(same, 1);
    let square = F::make(|out| fermionic_op_mul(a.0, a.0, out));
    assert_eq!(fermionic_op_is_zero(square.0, &mut same), 0);
    assert_eq!(same, 0);
    let ordered = F::make(|out| fermionic_op_normal_order(square.0, out));
    assert_eq!(fermionic_op_is_zero(ordered.0, &mut same), 0);
    assert_eq!(same, 1);
    let other = F::make(|out| fermionic_op_creation(2, 1, out));
    let ao = F::make(|out| fermionic_op_mul(a.0, other.0, out));
    let oa = F::make(|out| fermionic_op_mul(other.0, a.0, out));
    let cross = F::make(|out| fermionic_op_add(ao.0, oa.0, out));
    let zero = F::make(|out| fermionic_op_new(2, out));
    assert!(cross.equiv(&zero));
    let adjoint = F::make(|out| fermionic_op_adjoint(a.0, out));
    assert_eq!(fermionic_op_equals(adjoint.0, creator.0, &mut same), 0);
    assert_eq!(same, 1);
    let copy = F::make(|out| fermionic_op_copy(creator.0, out));
    assert_eq!(fermionic_op_chop(copy.0, 1.0), 0);
    assert!(!copy.equiv(&creator));
}

#[test]
fn excitation_order_signs_and_number_conservation() {
    let source = [0, 1];
    let target = [2, 3];
    let generator = F::make(|out| {
        fermionic_op_excitation_generator(4, source.as_ptr(), 2, target.as_ptr(), 2, out)
    });
    let reversed = [3, 2];
    let swapped = F::make(|out| {
        fermionic_op_excitation_generator(4, source.as_ptr(), 2, reversed.as_ptr(), 2, out)
    });
    let negative = F::make(|out| fermionic_op_scaled(generator.0, C::new(-1.0, 0.0), out));
    assert!(swapped.equiv(&negative));
    let mut yes = 99;
    assert_eq!(fermionic_op_is_hermitian(generator.0, 0.0, &mut yes), 0);
    assert_eq!(yes, 1);
    let number = F::make(|out| fermionic_op_total_number(4, out));
    let ng = F::make(|out| fermionic_op_mul(number.0, generator.0, out));
    let gn = F::make(|out| fermionic_op_mul(generator.0, number.0, out));
    let commutator = F::make(|out| fermionic_op_sub(ng.0, gn.0, out));
    let zero = F::make(|out| fermionic_op_new(4, out));
    assert!(commutator.equiv(&zero));
    let mut out = generator.0;
    for (sources, targets, expected) in [
        (&[][..], &[][..], -8),
        (&source[..], &target[..1], -8),
        (&[0, 0][..], &target[..], -8),
        (&source[..], &[1, 3][..], -8),
        (&source[..], &[2, 4][..], -2),
    ] {
        assert_eq!(
            fermionic_op_excitation_generator(
                4,
                sources.as_ptr(),
                sources.len(),
                targets.as_ptr(),
                targets.len(),
                &mut out
            ),
            expected
        );
        assert_eq!(out, generator.0);
    }
}

#[test]
fn jordan_wigner_signs_occupations_and_nonhermitian_results() {
    let creator = F::make(|out| fermionic_op_creation(1, 0, out));
    let annihilator = F::make(|out| fermionic_op_annihilation(1, 0, out));
    let creation_matrix = creator.mapped().matrix();
    assert_eq!(
        creation_matrix,
        vec![C::default(), C::default(), C::new(1.0, 0.0), C::default()]
    );
    assert_eq!(
        annihilator.mapped().matrix(),
        vec![C::default(), C::new(1.0, 0.0), C::default(), C::default()]
    );
    let number = F::make(|out| fermionic_op_number(1, 0, out));
    assert_eq!(
        number.mapped().matrix(),
        vec![C::default(), C::default(), C::default(), C::new(1.0, 0.0)]
    );
    let high = F::make(|out| fermionic_op_creation(2, 1, out));
    let matrix = high.mapped().matrix();
    assert_eq!(matrix[2 * 4], C::new(1.0, 0.0));
    assert_eq!(matrix[3 * 4 + 1], C::new(-1.0, 0.0));
    let mut bits = [99; 4];
    let occupation = [1, 0, 1];
    assert_eq!(
        jordan_wigner_map_occupation(occupation.as_ptr(), 3, bits.as_mut_ptr(), 4),
        0
    );
    assert_eq!(bits, [1, 0, 1, 99]);
    let before = bits;
    assert_eq!(
        jordan_wigner_map_occupation([1, 2, 0].as_ptr(), 3, bits.as_mut_ptr(), 4),
        -8
    );
    assert_eq!(bits, before);
    assert_eq!(
        jordan_wigner_map_occupation(occupation.as_ptr(), 3, bits.as_mut_ptr(), 2),
        -8
    );
    assert_eq!(bits, before);
    assert_eq!(jordan_wigner_map_occupation(null(), 0, null_mut(), 0), 0);
    assert_eq!(
        jordan_wigner_map_occupation(bits.as_ptr(), 3, bits.as_mut_ptr(), 4),
        0
    );
    let mut n = 99;
    assert_eq!(jordan_wigner_num_qubits(3, &mut n), 0);
    assert_eq!(n, 3);
}

#[test]
fn checked_pauli_phases_tensor_and_remap() {
    let x = H::from_pauli(c"X", C::new(1.0, 0.0));
    let z = H::from_pauli(c"Z", C::new(1.0, 0.0));
    let minus_i_y = H::from_pauli(c"Y", C::new(0.0, -1.0));
    let xz = H::make(|out| hamiltonian_mul(x.0, z.0, out));
    assert!(xz.equiv(&minus_i_y));
    let dagger = H::make(|out| hamiltonian_adjoint(xz.0, out));
    let negative = H::make(|out| hamiltonian_scaled(xz.0, C::new(-1.0, 0.0), out));
    assert!(dagger.equiv(&negative));
    let sum = H::make(|out| hamiltonian_add(x.0, z.0, out));
    let difference = H::make(|out| hamiltonian_sub(sum.0, z.0, out));
    assert!(difference.equiv(&x));
    let tensor = H::make(|out| hamiltonian_tensor(x.0, z.0, out));
    let expected = H::from_pauli(c"XZ", C::new(1.0, 0.0));
    assert!(tensor.equiv(&expected));
    let embed = H::make(|out| hamiltonian_remap_qubits(x.0, [2].as_ptr(), 1, 3, out));
    let expected = H::from_pauli(c"XII", C::new(1.0, 0.0));
    assert!(embed.equiv(&expected));
    let mut out = embed.0;
    assert_eq!(
        hamiltonian_remap_qubits(tensor.0, [0, 0].as_ptr(), 2, 3, &mut out),
        -8
    );
    assert_eq!(
        hamiltonian_remap_qubits(x.0, [3].as_ptr(), 1, 3, &mut out),
        -8
    );
    assert_eq!(hamiltonian_remap_qubits(x.0, null(), 0, 3, &mut out), -8);
    assert_eq!(out, embed.0);
    let zero = H::make(|out| hamiltonian_from_terms(0, null(), 0, out));
    let scalar_embed = H::make(|out| hamiltonian_remap_qubits(zero.0, null(), 0, 3, out));
    assert_eq!(hamiltonian_num_qubits(scalar_embed.0), 3);
    let phased = H::from_pauli(c"iX", C::new(0.0, -1.0));
    assert!(phased.equiv(&x));
}

#[test]
fn nonzero_coefficients_projection_and_legacy_semantics() {
    let tiny = 1e-20;
    let creator = F::make(|out| fermionic_op_creation(1, 0, out));
    let scaled = F::make(|out| fermionic_op_scaled(creator.0, C::new(tiny, 0.0), out));
    let mut n = 99;
    assert_eq!(fermionic_op_num_terms(scaled.0, &mut n), 0);
    assert_eq!(n, 1);
    let mapped = scaled.mapped();
    assert_eq!(hamiltonian_num_terms(mapped.0), 2);
    let h = H::from_pauli(c"X", C::new(tiny, 0.0));
    let normalized = H::make(|out| hamiltonian_normalized(h.0, out));
    assert_eq!(hamiltonian_num_terms(normalized.0), 1);
    let copy = H::make(|out| hamiltonian_copy(h.0, out));
    assert_eq!(hamiltonian_simplify(copy.0), 0);
    assert_eq!(hamiltonian_num_terms(copy.0), 0);
    assert_eq!(hamiltonian_num_terms(h.0), 1);
    assert_eq!(hamiltonian_chop(normalized.0, tiny), 0);
    assert_eq!(hamiltonian_num_terms(normalized.0), 0);
    let residual = H::from_pauli(c"Z", C::new(2.0, 4e-11));
    let mut yes = 99;
    assert_eq!(hamiltonian_is_hermitian(residual.0, 0.0, &mut yes), 0);
    assert_eq!(yes, 0);
    assert_eq!(hamiltonian_is_hermitian(residual.0, 8e-11, &mut yes), 0);
    assert_eq!(yes, 1);
    let projected = H::make(|out| hamiltonian_normalized_hermitian(residual.0, out));
    let real = H::from_pauli(c"Z", C::new(2.0, 0.0));
    assert!(projected.equiv(&real));
    let imaginary = H::from_pauli(c"Z", C::new(2.0, 2e-10));
    let mut out = real.0;
    assert_eq!(hamiltonian_normalized_hermitian(imaginary.0, &mut out), -8);
    assert_eq!(out, real.0);
}

#[test]
fn invalid_coefficients_atomicity_and_array_contracts() {
    let creator = F::make(|out| fermionic_op_creation(1, 0, out));
    let snapshot = F::make(|out| fermionic_op_copy(creator.0, out));
    let ladder = CFermionicLadder {
        mode: 0,
        kind: FERMIONIC_CREATE,
    };
    let mut out = creator.0;
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            fermionic_op_add_term(creator.0, &ladder, 1, C::new(bad, 0.0)),
            -8
        );
        assert!(creator.equiv(&snapshot));
        assert_eq!(
            fermionic_op_scaled(creator.0, C::new(0.0, bad), &mut out),
            -8
        );
        assert_eq!(out, creator.0);
    }
    for bad in [-1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(fermionic_op_chop(creator.0, bad), -8);
        assert!(creator.equiv(&snapshot));
        let mut result = 99;
        assert_eq!(
            fermionic_op_equiv(creator.0, snapshot.0, bad, &mut result),
            -8
        );
        assert_eq!(result, 99);
    }
    let invalid = CFermionicLadder { mode: 0, kind: 255 };
    assert_eq!(
        fermionic_op_from_term(1, &invalid, 1, C::default(), &mut out),
        -8
    );
    let invalid = CFermionicLadder {
        mode: 1,
        kind: FERMIONIC_CREATE,
    };
    assert_eq!(
        fermionic_op_from_term(1, &invalid, 1, C::default(), &mut out),
        -2
    );
    assert_eq!(
        fermionic_op_from_term(1, &ladder, usize::MAX, C::default(), &mut out),
        -8
    );
    assert_eq!(fermionic_op_from_terms(1, null(), 1, &mut out), -1);
    let mut buffer = [CFermionicLadder { mode: 99, kind: 99 }; 2];
    let mut coefficient = C::new(99.0, 99.0);
    assert_eq!(
        fermionic_op_get_term(creator.0, 0, buffer.as_mut_ptr(), 0, &mut coefficient),
        -8
    );
    assert_eq!(buffer[0].mode, 99);
    assert_eq!(coefficient, C::new(99.0, 99.0));
    assert_eq!(
        fermionic_op_get_term(creator.0, 9, buffer.as_mut_ptr(), 2, &mut coefficient),
        -2
    );
    assert_eq!(
        fermionic_op_get_term(creator.0, 0, null_mut(), 2, &mut coefficient),
        -1
    );
    assert_eq!(
        fermionic_op_get_term(creator.0, 0, buffer.as_mut_ptr(), 2, &mut coefficient),
        0
    );
    assert_eq!((buffer[0].mode, buffer[0].kind), (0, FERMIONIC_CREATE));
    assert_eq!(buffer[1].mode, 99);
    buffer[0].kind = FERMIONIC_ANNIHILATE;
    assert_eq!(buffer[0].kind, FERMIONIC_ANNIHILATE);
    assert!(creator.equiv(&snapshot));
    let huge = F::make(|out| fermionic_op_from_term(1, &ladder, 1, C::new(f64::MAX, 0.0), out));
    let mut out = creator.0;
    assert_eq!(fermionic_op_add(huge.0, huge.0, &mut out), -8);
    assert_eq!(
        fermionic_op_add_term(huge.0, &ladder, 1, C::new(f64::MAX, 0.0)),
        -8
    );
    assert_eq!(fermionic_op_scaled(huge.0, C::new(2.0, 0.0), &mut out), -8);
    assert_eq!(out, creator.0);
    assert_eq!(fermionic_op_creation(1, 1, &mut out), -2);
    let wrong_width = F::make(|out| fermionic_op_new(2, out));
    assert_eq!(fermionic_op_mul(creator.0, wrong_width.0, &mut out), -8);
    assert_eq!(out, creator.0);
}

#[test]
fn checked_hamiltonian_validates_legacy_storage_without_overwriting_outputs() {
    let p = pauli_string_parse(c"X".as_ptr());
    let h = H(hamiltonian_new(1));
    assert_eq!(hamiltonian_add_term(h.0, p, f64::NAN, 0.0), 0); // legacy behavior
    pauli_string_free(p);
    let snapshot = unsafe { (*h.0).inner.clone() };
    let valid = H::from_pauli(c"Z", C::new(1.0, 0.0));
    let mut out = valid.0;
    assert_eq!(hamiltonian_normalized(h.0, &mut out), -8);
    assert_eq!(hamiltonian_scaled(h.0, C::new(1.0, 0.0), &mut out), -8);
    assert_eq!(hamiltonian_add(h.0, valid.0, &mut out), -8);
    assert_eq!(hamiltonian_chop(h.0, 0.0), -8);
    assert_eq!(out, valid.0);
    assert!(unsafe { (&(*h.0).inner.terms)[0].1.re.is_nan() });
    assert_eq!(unsafe { (*h.0).inner.terms.len() }, snapshot.terms.len());
    let large = H::from_pauli(c"X", C::new(f64::MAX, 0.0));
    assert_eq!(hamiltonian_add(large.0, large.0, &mut out), -8);
    assert_eq!(hamiltonian_scaled(large.0, C::new(2.0, 0.0), &mut out), -8);
    let p = pauli_string_parse(c"X".as_ptr());
    let terms = [
        CHamiltonianTerm {
            pauli: p,
            coefficient: C::new(f64::MAX, 0.0),
        },
        CHamiltonianTerm {
            pauli: p,
            coefficient: C::new(f64::MAX, 0.0),
        },
    ];
    assert_eq!(hamiltonian_from_terms(1, terms.as_ptr(), 2, &mut out), -8);
    assert_eq!(hamiltonian_from_terms(2, terms.as_ptr(), 1, &mut out), -8);
    assert_eq!(
        hamiltonian_from_terms(1, terms.as_ptr(), usize::MAX, &mut out),
        -8
    );
    pauli_string_free(p);
    for bad in [-1.0, f64::NAN, f64::INFINITY] {
        let mut yes = 99;
        assert_eq!(hamiltonian_chop(valid.0, bad), -8);
        assert_eq!(hamiltonian_is_hermitian(valid.0, bad, &mut yes), -8);
        assert_eq!(yes, 99);
    }
    let mut copied = null_mut();
    let mut coefficient = C::new(99.0, 0.0);
    assert_eq!(
        hamiltonian_get_term(valid.0, 9, &mut copied, &mut coefficient),
        -2
    );
    assert!(copied.is_null());
    assert_eq!(coefficient.re, 99.0);
    assert_eq!(
        hamiltonian_get_term(valid.0, 0, &mut copied, &mut coefficient),
        0
    );
    assert_eq!(pauli_string_set_pauli(copied, 0, PAULI_X), 0);
    pauli_string_free(copied);
    assert_eq!(
        valid.matrix(),
        vec![
            C::new(1.0, 0.0),
            C::default(),
            C::default(),
            C::new(-1.0, 0.0)
        ]
    );
}

#[test]
fn excitation_to_evolution_and_numeric_phase_regressions() {
    let generator =
        F::make(|out| fermionic_op_excitation_generator(2, [0].as_ptr(), 1, [1].as_ptr(), 1, out));
    let mapped = generator.mapped();
    let theta: f64 = 0.37;
    let evolution = hamiltonian_to_evolution_circuit(mapped.0, theta, 1, TROTTER_MODE_SECOND_ORDER);
    assert!(!evolution.is_null());
    let state = [C::default(), C::new(1.0, 0.0), C::default(), C::default()];
    let sv = statevector_from_state(2, state.as_ptr(), 4);
    assert_eq!(statevector_apply_circuit(sv, evolution), 0);
    let mut data = [C::default(); 4];
    assert_eq!(statevector_data(sv, data.as_mut_ptr(), 4), 0);
    assert!((data[1] - C::new(theta.cos(), 0.0)).norm() < 1e-12);
    assert!((data[2] - C::new(theta.sin(), 0.0)).norm() < 1e-12);
    assert!(data[0].norm() < 1e-12 && data[3].norm() < 1e-12);
    statevector_free(sv);
    circuit_free(evolution);
    let phase = circuit_new(1);
    assert_eq!(circuit_set_global_phase(phase, 0.37), 0);
    let sv = statevector_from_circuit(phase);
    let mut data = [C::default(); 2];
    assert_eq!(statevector_data(sv, data.as_mut_ptr(), 2), 0);
    assert!((data[0] - C::from_polar(1.0, 0.37)).norm() < 1e-12);
    statevector_free(sv);
    circuit_free(phase);
    let a = param_from_double(1e-20);
    let b = param_from_double(2.0);
    let product = param_mul(a, b);
    assert_eq!(param_evaluate(product, null()), 2e-20);
    param_free(product);
    param_free(a);
    param_free(b);
}

#[test]
fn all_checked_entry_points_reject_required_null_pointers() {
    assert_eq!(fermionic_op_new(1, null_mut()), -1);
    assert_eq!(fermionic_op_identity(1, null_mut()), -1);
    assert_eq!(fermionic_op_total_number(1, null_mut()), -1);
    assert_eq!(fermionic_op_creation(1, 1, null_mut()), -1);
    assert_eq!(fermionic_op_annihilation(1, 1, null_mut()), -1);
    assert_eq!(fermionic_op_number(1, 1, null_mut()), -1);
    assert_eq!(
        fermionic_op_from_term(1, null(), 1, C::default(), null_mut()),
        -1
    );
    assert_eq!(fermionic_op_from_terms(1, null(), 1, null_mut()), -1);
    assert_eq!(
        fermionic_op_add_term(null_mut(), null(), 1, C::default()),
        -1
    );
    assert_eq!(
        fermionic_op_excitation_generator(1, null(), 1, null(), 1, null_mut()),
        -1
    );
    assert_eq!(fermionic_op_num_modes(null(), null_mut()), -1);
    assert_eq!(fermionic_op_num_terms(null(), null_mut()), -1);
    assert_eq!(fermionic_op_is_zero(null(), null_mut()), -1);
    assert_eq!(fermionic_op_copy(null(), null_mut()), -1);
    assert_eq!(fermionic_op_adjoint(null(), null_mut()), -1);
    assert_eq!(fermionic_op_normal_order(null(), null_mut()), -1);
    assert_eq!(fermionic_op_add(null(), null(), null_mut()), -1);
    assert_eq!(fermionic_op_sub(null(), null(), null_mut()), -1);
    assert_eq!(fermionic_op_mul(null(), null(), null_mut()), -1);
    assert_eq!(fermionic_op_scaled(null(), C::default(), null_mut()), -1);
    assert_eq!(fermionic_op_chop(null_mut(), 0.0), -1);
    assert_eq!(fermionic_op_equals(null(), null(), null_mut()), -1);
    assert_eq!(fermionic_op_equiv(null(), null(), 0.0, null_mut()), -1);
    assert_eq!(fermionic_op_is_hermitian(null(), 0.0, null_mut()), -1);
    assert_eq!(fermionic_op_term_len(null(), 1, null_mut()), -1);
    assert_eq!(
        fermionic_op_get_term(null(), 1, null_mut(), 1, null_mut()),
        -1
    );
    assert_eq!(jordan_wigner_map_operator(null(), null_mut()), -1);
    assert_eq!(jordan_wigner_num_qubits(1, null_mut()), -1);
    assert_eq!(jordan_wigner_map_occupation(null(), 1, null_mut(), 1), -1);
    assert_eq!(hamiltonian_identity(1, null_mut()), -1);
    assert_eq!(hamiltonian_from_terms(1, null(), 1, null_mut()), -1);
    assert_eq!(hamiltonian_copy(null(), null_mut()), -1);
    assert_eq!(hamiltonian_normalized(null(), null_mut()), -1);
    assert_eq!(hamiltonian_normalized_hermitian(null(), null_mut()), -1);
    assert_eq!(hamiltonian_adjoint(null(), null_mut()), -1);
    assert_eq!(hamiltonian_add(null(), null(), null_mut()), -1);
    assert_eq!(hamiltonian_sub(null(), null(), null_mut()), -1);
    assert_eq!(hamiltonian_mul(null(), null(), null_mut()), -1);
    assert_eq!(hamiltonian_tensor(null(), null(), null_mut()), -1);
    assert_eq!(hamiltonian_scaled(null(), C::default(), null_mut()), -1);
    assert_eq!(
        hamiltonian_remap_qubits(null(), null(), 1, 1, null_mut()),
        -1
    );
    assert_eq!(hamiltonian_chop(null_mut(), 0.0), -1);
    assert_eq!(hamiltonian_equiv(null(), null(), 0.0, null_mut()), -1);
    assert_eq!(hamiltonian_is_hermitian(null(), 0.0, null_mut()), -1);
    assert_eq!(hamiltonian_get_term(null(), 1, null_mut(), null_mut()), -1);
}

#[test]
fn wide_sparse_registers_reject_dense_sizes_without_panicking_or_writing() {
    let bits = usize::BITS as usize;
    // Exercise side-length, element-count and byte-count overflow separately.
    for width in [
        bits,
        bits + 1,
        bits - 1,
        bits / 2,
        bits / 2 - 1,
        bits / 2 - 2,
    ] {
        let identity = F::make(|out| fermionic_op_identity(width, out));
        let mapped = identity.mapped();
        let mut pauli = null_mut();
        let mut coefficient = C::default();
        assert_eq!(
            hamiltonian_get_term(mapped.0, 0, &mut pauli, &mut coefficient),
            0
        );
        let expected_dim = if width < bits { 1usize << width } else { 0 };
        assert_eq!(hamiltonian_to_matrix_len(mapped.0), expected_dim);
        assert_eq!(pauli_string_matrix_len(pauli), expected_dim);
        let sentinel = C::new(99.0, -99.0);
        let mut buffer = sentinel;
        assert_eq!(hamiltonian_to_matrix(mapped.0, &mut buffer, 1), -7);
        assert_eq!(buffer, sentinel);
        assert_eq!(pauli_string_matrix(pauli, &mut buffer, 1), -7);
        assert_eq!(buffer, sentinel);
        pauli_string_free(pauli);
    }
    let zero = F::make(|out| fermionic_op_new(usize::MAX, out)).mapped();
    assert_eq!(hamiltonian_to_matrix_len(zero.0), 0);
    let mut buffer = C::new(99.0, -99.0);
    assert_eq!(hamiltonian_to_matrix(zero.0, &mut buffer, 1), -7);
    assert_eq!(buffer, C::new(99.0, -99.0));
    // A valid width with a tiny buffer must be rejected before dense allocation.
    let identity = H::make(|out| hamiltonian_identity(20, out));
    let mut pauli = null_mut();
    let mut coefficient = C::default();
    assert_eq!(
        hamiltonian_get_term(identity.0, 0, &mut pauli, &mut coefficient),
        0
    );
    assert_eq!(hamiltonian_to_matrix(identity.0, &mut buffer, 1), -8);
    assert_eq!(pauli_string_matrix(pauli, &mut buffer, 1), -8);
    assert_eq!(buffer, C::new(99.0, -99.0));
    pauli_string_free(pauli);
    assert_eq!(hamiltonian_to_matrix_len(null()), 0);
    assert_eq!(pauli_string_matrix_len(null()), 0);
}
