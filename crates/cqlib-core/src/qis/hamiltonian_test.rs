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

use super::Hamiltonian;
use crate::qis::{PauliString, Phase, QisError};
use ndarray::{Array2, arr2};
use num_complex::Complex64;

/// Test-only Kronecker product, independent of production Pauli operations.
fn matrix_tensor(a: &Array2<Complex64>, b: &Array2<Complex64>) -> Array2<Complex64> {
    Array2::from_shape_fn((a.nrows() * b.nrows(), a.ncols() * b.ncols()), |(r, c)| {
        a[[r / b.nrows(), c / b.ncols()]] * b[[r % b.nrows(), c % b.ncols()]]
    })
}

fn matrix_adjoint(a: &Array2<Complex64>) -> Array2<Complex64> {
    a.t().mapv(|c| c.conj())
}

/// Literal single-qubit matrices and explicit labels, not production masks/phases.
fn matrix_oracle(terms: &[(&str, Phase, Complex64)]) -> Array2<Complex64> {
    let width = terms[0].0.len();
    let mut sum = Array2::zeros((1 << width, 1 << width));
    let zero = Complex64::new(0.0, 0.0);
    let one = Complex64::new(1.0, 0.0);
    let i = Complex64::new(0.0, 1.0);
    for &(label, phase, coefficient) in terms {
        let mut product = arr2(&[[one]]);
        for letter in label.chars() {
            let local = match letter {
                'I' => arr2(&[[one, zero], [zero, one]]),
                'X' => arr2(&[[zero, one], [one, zero]]),
                'Y' => arr2(&[[zero, -i], [i, zero]]),
                'Z' => arr2(&[[one, zero], [zero, -one]]),
                _ => panic!("invalid test label"),
            };
            product = matrix_tensor(&product, &local);
        }
        let phase = match phase {
            Phase::Plus => one,
            Phase::Minus => -one,
            Phase::I => i,
            Phase::MinusI => -i,
        };
        sum += &product.mapv(|c| c * phase * coefficient);
    }
    sum
}

fn raw_sum(terms: &[(&str, Phase, Complex64)]) -> Hamiltonian {
    Hamiltonian::from_list(
        terms
            .iter()
            .map(|&(label, phase, coefficient)| {
                let mut pauli: PauliString = label.parse().unwrap();
                pauli.phase = phase;
                (pauli, coefficient)
            })
            .collect(),
    )
    .unwrap()
}

fn assert_matrix_close(actual: &Array2<Complex64>, expected: &Array2<Complex64>) {
    assert_eq!(actual.dim(), expected.dim());
    for (a, b) in actual.iter().zip(expected) {
        assert!((*a - *b).norm() < 1e-12, "{actual:?} != {expected:?}");
    }
}

#[test]
fn sparse_arithmetic_matches_independent_matrices() {
    let a_terms = [
        ("XYZ", Phase::MinusI, Complex64::new(0.5, 0.25)),
        ("XYZ", Phase::Plus, Complex64::new(-0.125, 0.75)),
        ("YIX", Phase::Minus, Complex64::new(1.0, -0.5)),
        ("III", Phase::I, Complex64::new(-0.25, 0.5)),
    ];
    let b_terms = [
        ("ZZY", Phase::I, Complex64::new(0.75, -0.25)),
        ("XYY", Phase::Plus, Complex64::new(-0.5, 0.5)),
    ];
    let a = raw_sum(&a_terms);
    let b = raw_sum(&b_terms);
    let saved = (a.clone(), b.clone());
    let am = matrix_oracle(&a_terms);
    let bm = matrix_oracle(&b_terms);
    assert_matrix_close(&a.to_matrix().unwrap(), &am);
    assert_matrix_close(&a.try_add(&b).unwrap().to_matrix().unwrap(), &(&am + &bm));
    assert_matrix_close(&a.try_sub(&b).unwrap().to_matrix().unwrap(), &(&am - &bm));
    assert_matrix_close(&a.try_mul(&b).unwrap().to_matrix().unwrap(), &am.dot(&bm));
    assert_matrix_close(
        &a.adjoint().unwrap().to_matrix().unwrap(),
        &matrix_adjoint(&am),
    );
    let factor = Complex64::new(0.25, -0.5);
    assert_matrix_close(
        &a.scaled(factor).unwrap().to_matrix().unwrap(),
        &am.mapv(|c| c * factor),
    );
    assert_eq!((a, b), saved);
}

#[test]
fn exhaustive_two_qubit_products_include_all_global_phases() {
    for a in ['I', 'X', 'Y', 'Z'] {
        for b in ['I', 'X', 'Y', 'Z'] {
            for c in ['I', 'X', 'Y', 'Z'] {
                for d in ['I', 'X', 'Y', 'Z'] {
                    for phase in [Phase::Plus, Phase::Minus, Phase::I, Phase::MinusI] {
                        let left_label = format!("{a}{b}");
                        let right_label = format!("{c}{d}");
                        let left = [(left_label.as_str(), phase, Complex64::new(0.5, 0.25))];
                        let right = [(
                            right_label.as_str(),
                            Phase::MinusI,
                            Complex64::new(-0.25, 0.5),
                        )];
                        let product = raw_sum(&left).try_mul(&raw_sum(&right)).unwrap();
                        assert_matrix_close(
                            &product.to_matrix().unwrap(),
                            &matrix_oracle(&left).dot(&matrix_oracle(&right)),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn cancellations_and_adjoint_product_order_follow_operator_algebra() {
    let x = Hamiltonian::from_pauli("X".parse().unwrap());
    let z = Hamiltonian::from_pauli("Z".parse().unwrap());
    let sum = x.try_add(&z).unwrap();
    assert_eq!(
        sum.try_mul(&sum).unwrap(),
        Hamiltonian::identity(1).scaled(2.0.into()).unwrap()
    );
    let product = x.try_mul(&z).unwrap();
    assert_eq!(
        product.terms,
        vec![("Y".parse().unwrap(), Complex64::new(0.0, -1.0))]
    );
    assert!(!product.is_hermitian(0.0).unwrap());
    assert_eq!(
        product.adjoint().unwrap(),
        z.adjoint().unwrap().try_mul(&x.adjoint().unwrap()).unwrap()
    );
    let commutator = product.try_sub(&z.try_mul(&x).unwrap()).unwrap();
    assert_eq!(commutator.terms[0].1, Complex64::new(0.0, -2.0));
    assert!(
        product
            .try_add(&z.try_mul(&x).unwrap())
            .unwrap()
            .terms
            .is_empty()
    );
}

#[test]
fn checked_construction_and_comparison_keep_legacy_structure_semantics() {
    let p: PauliString = "X".parse().unwrap();
    let raw =
        Hamiltonian::from_list(vec![(p.clone(), 0.25.into()), (p.clone(), 0.75.into())]).unwrap();
    let canonical = Hamiltonian::from_terms(1, raw.terms.clone()).unwrap();
    assert_eq!(canonical, Hamiltonian::from_pauli(p));
    assert_ne!(raw, canonical);
    assert!(raw.equiv(&canonical, 0.0).unwrap());
    assert_eq!((raw.clone() + canonical.clone()).terms.len(), 3);
    assert_eq!(raw.try_add(&canonical).unwrap().terms.len(), 1);
    assert_eq!(Hamiltonian::from_list(vec![]).unwrap().num_qubits, 0);
    assert_eq!(Hamiltonian::from_terms(7, []).unwrap(), Hamiltonian::new(7));
    assert_eq!(
        canonical.normalized().unwrap().normalized().unwrap(),
        canonical
    );
    let reversed = Hamiltonian::from_list(raw.terms.into_iter().rev().collect()).unwrap();
    assert_eq!(reversed.normalized().unwrap(), canonical);
}

#[test]
fn explicit_chopping_follows_merging_and_preserves_tiny_coefficients() {
    let tiny = f64::from_bits(1);
    let p: PauliString = "Y".parse().unwrap();
    let mut phased = p.clone();
    phased.phase = Phase::I;
    let small = Hamiltonian::from_terms(1, [(phased, tiny.into())]).unwrap();
    assert_eq!(small.terms[0].1, Complex64::new(0.0, tiny));
    for result in [
        small.try_mul(&Hamiltonian::identity(1)).unwrap(),
        small.scaled(1.0.into()).unwrap(),
        small.try_add(&Hamiltonian::new(1)).unwrap(),
        small.adjoint().unwrap().adjoint().unwrap(),
    ] {
        assert_eq!(result, small);
    }
    let mut h = Hamiltonian::from_list(vec![(p.clone(), 0.75.into()), (p, 0.75.into())]).unwrap();
    h.chop(1.0).unwrap();
    assert_eq!(h.terms[0].1, Complex64::new(1.5, 0.0));
    h.chop(1.5).unwrap();
    assert_eq!(h, Hamiltonian::new(1));
    let mut small = small;
    small.chop(0.0).unwrap();
    assert_eq!(small.terms.len(), 1);
    small.chop(tiny).unwrap();
    assert!(small.terms.is_empty());
    let mut complex =
        Hamiltonian::from_terms(1, [("X".parse().unwrap(), Complex64::new(3.0, 4.0))]).unwrap();
    complex.chop(4.0).unwrap();
    assert_eq!(complex.terms.len(), 1);
    complex.chop(5.0).unwrap();
    assert!(complex.terms.is_empty());
}

#[test]
fn hermiticity_uses_adjoint_difference_tolerance_without_projection() {
    let h =
        Hamiltonian::from_terms(1, [("Z".parse().unwrap(), Complex64::new(1.0, 0.125))]).unwrap();
    assert!(h.is_hermitian(0.25).unwrap());
    assert!(!h.is_hermitian(0.249).unwrap());
    assert_eq!(h.terms[0].1.im, 0.125);
    let phased = raw_sum(&[("X", Phase::I, Complex64::new(0.0, -1.0))]);
    assert!(phased.is_hermitian(0.0).unwrap());
    assert!(!h.equiv(&Hamiltonian::identity(1), 0.0).unwrap());
}

#[test]
fn tensor_and_permutation_embedding_match_independent_matrices() {
    let at = [("YX", Phase::MinusI, Complex64::new(0.5, 0.25))];
    let bt = [
        ("Z", Phase::I, Complex64::new(-0.25, 1.0)),
        ("X", Phase::Minus, 0.5.into()),
    ];
    let a = raw_sum(&at);
    let b = raw_sum(&bt);
    assert_matrix_close(
        &a.tensor(&b).unwrap().to_matrix().unwrap(),
        &matrix_tensor(&matrix_oracle(&at), &matrix_oracle(&bt)),
    );
    let swapped = a.remap_qubits(&[1, 0], 2).unwrap();
    assert_matrix_close(
        &swapped.to_matrix().unwrap(),
        &matrix_oracle(&[("XY", Phase::MinusI, Complex64::new(0.5, 0.25))]),
    );
    let embedded = a.remap_qubits(&[3, 1], 4).unwrap();
    assert_matrix_close(
        &embedded.to_matrix().unwrap(),
        &matrix_oracle(&[("XIYI", Phase::MinusI, Complex64::new(0.5, 0.25))]),
    );
    let scalar = Hamiltonian::identity(0)
        .scaled(Complex64::new(2.0, -0.5))
        .unwrap();
    assert_eq!(
        a.tensor(&scalar).unwrap(),
        a.scaled(Complex64::new(2.0, -0.5)).unwrap()
    );
    assert_eq!(
        scalar.tensor(&a).unwrap(),
        a.scaled(Complex64::new(2.0, -0.5)).unwrap()
    );
}

#[test]
fn zero_and_scalar_operators_retain_declared_spaces() {
    for width in [0, 1, 128] {
        let zero = Hamiltonian::new(width);
        let identity = Hamiltonian::identity(width);
        assert_eq!(zero.try_mul(&identity).unwrap(), zero);
        assert_eq!(identity.try_mul(&zero).unwrap(), zero);
        assert_eq!(identity.try_sub(&identity).unwrap(), zero);
        assert_eq!(identity.scaled(0.0.into()).unwrap(), zero);
        assert_eq!(
            zero.tensor(&Hamiltonian::new(2)).unwrap().num_qubits,
            width + 2
        );
        let map: Vec<_> = (0..width).map(|q| q + 1).collect();
        assert_eq!(
            zero.remap_qubits(&map, width + 2).unwrap(),
            Hamiltonian::new(width + 2)
        );
        assert_eq!(
            identity.remap_qubits(&map, width + 2).unwrap(),
            Hamiltonian::identity(width + 2)
        );
    }
    assert_eq!(
        Hamiltonian::identity(0).to_matrix().unwrap(),
        arr2(&[[Complex64::new(1.0, 0.0)]])
    );
}

#[test]
fn wide_register_arithmetic_never_needs_machine_word_masks() {
    use crate::qis::Pauli;
    for width in [65, 128, 1000] {
        let mut p = PauliString::new(width);
        p.set_pauli(width - 1, Pauli::Y);
        p.set_pauli(0, Pauli::Z);
        let a = Hamiltonian::from_terms(width, [(p.clone(), 0.5.into())]).unwrap();
        assert_eq!(
            a.try_mul(&a).unwrap(),
            Hamiltonian::identity(width).scaled(0.25.into()).unwrap()
        );
        assert!(a.is_hermitian(0.0).unwrap());
        assert!(
            a.try_add(&a)
                .unwrap()
                .equiv(&a.scaled(2.0.into()).unwrap(), 0.0)
                .unwrap()
        );
        let mapping: Vec<_> = (0..width).rev().collect();
        let reversed = a.remap_qubits(&mapping, width).unwrap();
        assert_eq!(reversed.terms[0].0.get_pauli(0), Pauli::Y);
        assert_eq!(reversed.terms[0].0.get_pauli(width - 1), Pauli::Z);
        assert_eq!(reversed.remap_qubits(&mapping, width).unwrap(), a);
        let tensor = a
            .tensor(&Hamiltonian::from_pauli("X".parse().unwrap()))
            .unwrap();
        assert_eq!(tensor.num_qubits, width + 1);
        assert_eq!(tensor.terms[0].0.get_pauli(0), Pauli::X);
        assert_eq!(tensor.terms[0].0.get_pauli(width), Pauli::Y);
    }
}

#[test]
fn invalid_inputs_are_rejected_even_when_other_operand_is_zero() {
    let mut short = PauliString::new(1);
    short.x.clear();
    let mut long = PauliString::new(1);
    long.z.push(false);
    for terms in [
        vec![(PauliString::new(2), 1.0.into())],
        vec![(short, 1.0.into())],
        vec![(long, 1.0.into())],
        vec![(PauliString::new(1), f64::NAN.into())],
        vec![(PauliString::new(1), f64::INFINITY.into())],
        vec![(PauliString::new(1), Complex64::new(0.0, f64::NEG_INFINITY))],
    ] {
        let h = Hamiltonian {
            num_qubits: 1,
            terms: terms.clone(),
        };
        let zero = Hamiltonian::new(1);
        assert!(Hamiltonian::from_terms(1, terms).is_err());
        assert!(h.try_add(&zero).is_err());
        assert!(h.try_sub(&zero).is_err());
        assert!(h.try_mul(&zero).is_err());
        assert!(zero.try_mul(&h).is_err());
        assert!(h.scaled(0.0.into()).is_err());
        assert!(h.adjoint().is_err());
        assert!(h.tensor(&zero).is_err());
        assert!(zero.tensor(&h).is_err());
        assert!(h.remap_qubits(&[0], 1).is_err());
        assert!(h.equiv(&h, 0.0).is_err());
        assert!(h.is_hermitian(0.0).is_err());
        assert!(h.to_matrix().is_err());
        let mut chopped = h.clone();
        assert!(chopped.chop(0.0).is_err());
        // Compare coefficient bits because NaN is not equal to itself.
        assert_eq!(chopped.num_qubits, h.num_qubits);
        assert_eq!(chopped.terms.len(), h.terms.len());
        for ((actual_p, actual_c), (expected_p, expected_c)) in chopped.terms.iter().zip(&h.terms) {
            assert_eq!(actual_p, expected_p);
            assert_eq!(actual_c.re.to_bits(), expected_c.re.to_bits());
            assert_eq!(actual_c.im.to_bits(), expected_c.im.to_bits());
        }
    }
    let a = Hamiltonian::new(1);
    let b = Hamiltonian::new(2);
    for result in [a.try_add(&b), a.try_sub(&b), a.try_mul(&b)] {
        assert!(matches!(result, Err(QisError::QubitMismatch { .. })));
    }
    assert!(a.equiv(&b, 0.0).is_err());
}

#[test]
fn invalid_tolerances_mappings_and_scalars_do_not_mutate_input() {
    let h = Hamiltonian::identity(2);
    for atol in [-1.0, f64::NAN, f64::INFINITY] {
        let mut chopped = h.clone();
        assert!(chopped.chop(atol).is_err());
        assert_eq!(chopped, h);
        assert!(h.equiv(&h, atol).is_err());
        assert!(h.is_hermitian(atol).is_err());
    }
    for mapping in [&[0][..], &[0, 0], &[0, 2]] {
        assert!(h.remap_qubits(mapping, 2).is_err());
        assert!(Hamiltonian::new(2).remap_qubits(mapping, 2).is_err());
    }
    for factor in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Hamiltonian::new(2).scaled(factor.into()).is_err());
    }
    assert!(
        Hamiltonian::new(usize::MAX)
            .tensor(&Hamiltonian::new(1))
            .is_err()
    );
}

#[test]
fn unrepresentable_embeddings_return_errors_without_panicking() {
    let limit = bitvec::slice::BitSlice::<usize, bitvec::order::Lsb0>::MAX_BITS;
    for width in [limit + 1, usize::MAX] {
        // Empty source mapping is valid, but scalar identity still requires a
        // target Pauli string: reject its unrepresentable width before allocation.
        let scalar = Hamiltonian::identity(0);
        assert!(matches!(
            scalar.remap_qubits(&[], width),
            Err(QisError::UnsupportedOperation(_))
        ));
        assert!(matches!(
            Hamiltonian::identity(1).remap_qubits(&[width - 1], width),
            Err(QisError::UnsupportedOperation(_))
        ));
        // Zero sums do not allocate target bit vectors and retain declared width.
        assert_eq!(
            Hamiltonian::new(0).remap_qubits(&[], width).unwrap(),
            Hamiltonian::new(width)
        );
        assert_eq!(
            Hamiltonian::new(width).tensor(&scalar).unwrap(),
            Hamiltonian::new(width)
        );
    }
}

#[test]
fn intermediate_and_dense_coefficient_overflow_is_reported() {
    let max = Hamiltonian::identity(1).scaled(f64::MAX.into()).unwrap();
    let two = Hamiltonian::identity(1).scaled(2.0.into()).unwrap();
    for result in [
        max.try_add(&max),
        max.scaled(2.0.into()),
        max.try_mul(&two),
        max.tensor(&two),
        max.try_sub(&max.scaled((-1.0).into()).unwrap()),
    ] {
        assert!(matches!(result, Err(QisError::NonFiniteCoefficient { .. })));
    }
    // A later cancellation cannot hide an earlier nonfinite accumulation.
    let raw = Hamiltonian::from_list(
        [f64::MAX, f64::MAX, -f64::MAX]
            .into_iter()
            .map(|c| (PauliString::new(1), c.into()))
            .collect(),
    )
    .unwrap();
    assert!(matches!(
        raw.normalized(),
        Err(QisError::NonFiniteCoefficient { .. })
    ));
    let mut chopped = raw.clone();
    assert!(chopped.chop(0.0).is_err());
    assert_eq!(chopped, raw);
    let dense = Hamiltonian::from_terms(
        1,
        [
            ("I".parse().unwrap(), f64::MAX.into()),
            ("Z".parse().unwrap(), f64::MAX.into()),
        ],
    )
    .unwrap();
    assert!(matches!(
        dense.to_matrix(),
        Err(QisError::NonFiniteCoefficient { .. })
    ));
}

#[test]
fn dense_conversion_checks_shift_element_and_byte_overflow_before_allocation() {
    // On 64-bit: 64 overflows the shift, 32 the element count, 30 the byte range.
    for width in [
        usize::BITS as usize,
        usize::BITS as usize / 2,
        (usize::BITS as usize - 4) / 2,
        usize::MAX,
    ] {
        assert!(matches!(
            Hamiltonian::new(width).to_matrix(),
            Err(QisError::UnsupportedOperation(_))
        ));
    }
}

#[test]
fn mapped_fermionic_products_and_adjoint_use_public_pauli_algebra() {
    use crate::qis::{FermionToQubitMapper, FermionicOp, JordanWignerMapper};
    let mapper = JordanWignerMapper::new();
    for p in 0..3 {
        for q in 0..3 {
            let a = FermionicOp::creation(3, p).unwrap();
            let b = FermionicOp::annihilation(3, q).unwrap();
            let mapped_a = mapper.map_operator(&a).unwrap();
            let mapped_b = mapper.map_operator(&b).unwrap();
            assert!(
                mapped_a
                    .try_mul(&mapped_b)
                    .unwrap()
                    .equiv(&mapper.map_operator(&a.try_mul(&b).unwrap()).unwrap(), 0.0)
                    .unwrap()
            );
            assert!(
                mapped_a
                    .adjoint()
                    .unwrap()
                    .equiv(&mapper.map_operator(&a.adjoint()).unwrap(), 0.0)
                    .unwrap()
            );
        }
    }
}

#[test]
fn hermitian_normalization_projects_only_accepted_imaginary_residuals() {
    let h = Hamiltonian::from_list(vec![
        ("I".parse().unwrap(), Complex64::new(0.0, 1e-12)),
        ("Z".parse().unwrap(), Complex64::new(1e-16, -1e-10)),
    ])
    .unwrap();
    let normalized = h.normalized_hermitian().unwrap();
    assert_eq!(normalized.num_qubits, 1);
    assert_eq!(
        normalized.terms,
        vec![("Z".parse().unwrap(), Complex64::new(1e-16, 0.0))]
    );
    assert_eq!(normalized.normalized_hermitian().unwrap(), normalized);
    // General complex normalization must not apply the real projection.
    assert_eq!(h.normalized().unwrap().terms.len(), 2);

    let mut invalid = h.clone();
    invalid.terms[0].1.im = 1.01e-10;
    assert!(matches!(
        invalid.normalized_hermitian(),
        Err(QisError::NotHermitian)
    ));

    // Validate the merged imaginary coefficient before removing zero real parts.
    let merged_invalid = Hamiltonian::from_list(vec![
        ("Z".parse().unwrap(), Complex64::new(0.0, 0.75e-10)),
        ("Z".parse().unwrap(), Complex64::new(0.0, 0.75e-10)),
    ])
    .unwrap();
    assert!(matches!(
        merged_invalid.normalized_hermitian(),
        Err(QisError::NotHermitian)
    ));
}

#[test]
fn empty_hamiltonian_to_matrix_is_zero() {
    assert_eq!(
        Hamiltonian::new(2).to_matrix().unwrap(),
        Array2::from_elem((4, 4), Complex64::new(0.0, 0.0))
    );
}

#[test]
fn hamiltonian_to_matrix_sums_coefficients_and_pauli_phases() {
    let mut x: PauliString = "X".parse().unwrap();
    x.phase = Phase::Minus;
    let z: PauliString = "Z".parse().unwrap();
    let hamiltonian = Hamiltonian::from_list(vec![
        (x, Complex64::new(2.0, 0.0)),
        (z, Complex64::new(0.0, 1.0)),
    ])
    .unwrap();

    assert_eq!(
        hamiltonian.to_matrix().unwrap(),
        arr2(&[
            [Complex64::new(0.0, 1.0), Complex64::new(-2.0, 0.0)],
            [Complex64::new(-2.0, 0.0), Complex64::new(0.0, -1.0)],
        ])
    );
}

#[test]
fn zero_qubit_hamiltonian_to_matrix_is_scalar() {
    let mut pauli = PauliString::new(0);
    pauli.phase = Phase::I;
    let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(2.0, 0.0))]).unwrap();

    assert_eq!(
        hamiltonian.to_matrix().unwrap(),
        arr2(&[[Complex64::new(0.0, 2.0)]])
    );
}

#[test]
fn hamiltonian_to_matrix_rejects_mismatched_terms() {
    let mut hamiltonian = Hamiltonian::new(1);
    hamiltonian
        .terms
        .push((PauliString::new(2), Complex64::new(1.0, 0.0)));

    assert!(matches!(
        hamiltonian.to_matrix(),
        Err(QisError::QubitMismatch {
            expected: 1,
            actual: 2
        })
    ));
}

#[test]
fn hamiltonian_standard_conversions_preserve_validation() {
    let pauli: PauliString = "XZ".parse().unwrap();
    let single = Hamiltonian::from(pauli.clone());
    assert_eq!(single.num_qubits, 2);
    assert_eq!(
        single.terms,
        vec![(pauli.clone(), Complex64::new(1.0, 0.0))]
    );

    let terms = vec![(pauli, Complex64::new(0.5, 0.0))];
    assert_eq!(Hamiltonian::try_from(terms.clone()).unwrap().terms, terms);
    assert!(matches!(
        Hamiltonian::try_from(vec![
            (PauliString::new(1), Complex64::new(1.0, 0.0)),
            (PauliString::new(2), Complex64::new(1.0, 0.0)),
        ]),
        Err(QisError::QubitMismatch { .. })
    ));
}
