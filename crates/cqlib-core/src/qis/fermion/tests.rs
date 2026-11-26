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

use super::*;
use crate::circuit::ansatz::{Ansatz, BasisEncoding, EvolutionStrategy, PauliEvolutionAnsatz};
use crate::circuit::circuit_to_matrix::circuit_to_matrix;
use crate::qis::{
    DensityMatrix, Hamiltonian, Observable, PauliString, Phase, QisError, Statevector, TrotterMode,
};
use FermionicLadder::{Annihilate, Create};
use ndarray::Array2;
use num_complex::Complex64;
use std::collections::{BTreeMap, HashMap};

// Independent Fock-basis action: no Pauli matrices or mapping formulas are used.
fn fock_matrix(op: &FermionicOp) -> Array2<Complex64> {
    let dim = 1 << op.num_modes();
    let mut matrix = Array2::zeros((dim, dim));
    for (word, coefficient) in op.terms() {
        for source in 0..dim {
            let mut state = source;
            let mut amplitude = coefficient;
            for ladder in word.iter().rev() {
                let p = ladder.mode();
                let occupied = state & (1 << p) != 0;
                if matches!(ladder, Create(_)) == occupied {
                    amplitude = Complex64::new(0.0, 0.0);
                    break;
                }
                if (state & ((1 << p) - 1)).count_ones() % 2 == 1 {
                    amplitude = -amplitude;
                }
                state ^= 1 << p;
            }
            matrix[[state, source]] += amplitude;
        }
    }
    matrix
}

fn assert_matrix_close(left: &Array2<Complex64>, right: &Array2<Complex64>) {
    assert_eq!(left.dim(), right.dim());
    for (a, b) in left.iter().zip(right) {
        assert!((*a - *b).norm() < 1e-11, "{a} != {b}");
    }
}

// Independent occupation transitions for T: remove sources in input order,
// then insert targets in reverse order. This does not inspect FermionicOp words
// or use its algebra, normal ordering or JW mapping.
fn excitation_fock_matrix(n: usize, sources: &[usize], targets: &[usize]) -> Array2<Complex64> {
    let dim = 1usize << n;
    let mut transition = Array2::<Complex64>::zeros((dim, dim));
    for initial in 0..dim {
        let mut state = initial;
        let mut sign = 1.0;
        let mut allowed = true;
        for (&mode, create) in sources
            .iter()
            .map(|p| (p, false))
            .chain(targets.iter().rev().map(|p| (p, true)))
        {
            if (state & (1 << mode) != 0) == create {
                allowed = false;
                break;
            }
            if (state & ((1 << mode) - 1)).count_ones() % 2 != 0 {
                sign = -sign;
            }
            state ^= 1 << mode;
        }
        if allowed {
            transition[[state, initial]] = Complex64::new(sign, 0.0);
        }
    }
    (&transition - &transition.t().mapv(|c| c.conj())).mapv(|c| Complex64::new(0.0, 1.0) * c)
}

#[test]
fn excitation_generator_validates_all_index_constraints() {
    for (sources, targets) in [
        (vec![], vec![]),
        (vec![], vec![1]),
        (vec![0, 1], vec![2]),
        (vec![0, 0], vec![2, 3]),
        (vec![0, 1], vec![2, 2]),
        (vec![0], vec![0]),
        (vec![0, 1], vec![1, 2]),
    ] {
        assert!(matches!(
            FermionicOp::excitation_generator(4, &sources, &targets),
            Err(QisError::InvalidParameterValue(_))
        ));
    }
    for (n, sources, targets, invalid) in [
        (0, vec![0], vec![1], 0),
        (4, vec![4], vec![1], 4),
        (4, vec![0], vec![4], 4),
        (4, vec![0], vec![usize::MAX], usize::MAX),
    ] {
        assert!(matches!(
            FermionicOp::excitation_generator(n, &sources, &targets),
            Err(QisError::ModeOutOfBounds { mode, num_modes })
                if mode == invalid && num_modes == n
        ));
    }
}

#[test]
fn excitation_generator_preserves_input_order_and_permutation_signs() {
    let g = FermionicOp::excitation_generator(5, &[1, 0], &[4, 2]).unwrap();
    let expected = FermionicOp::from_terms(
        5,
        [
            (
                vec![Create(4), Create(2), Annihilate(0), Annihilate(1)],
                Complex64::new(0.0, 1.0),
            ),
            (
                vec![Create(1), Create(0), Annihilate(2), Annihilate(4)],
                Complex64::new(0.0, -1.0),
            ),
        ],
    )
    .unwrap();
    assert_eq!(g, expected);
    assert_eq!(g.num_modes(), 5);
    assert_eq!(g.num_terms(), 2);
    assert_eq!(g.adjoint(), g);
    let ordered = g.normal_order().unwrap();
    assert_eq!(
        FermionicOp::excitation_generator(5, &[0, 1], &[4, 2])
            .unwrap()
            .normal_order()
            .unwrap(),
        -&ordered
    );
    assert_eq!(
        FermionicOp::excitation_generator(5, &[1, 0], &[2, 4])
            .unwrap()
            .normal_order()
            .unwrap(),
        -&ordered
    );
    assert_eq!(
        FermionicOp::excitation_generator(5, &[0, 1], &[2, 4])
            .unwrap()
            .normal_order()
            .unwrap(),
        ordered
    );
    assert_eq!(
        FermionicOp::excitation_generator(5, &[4, 2], &[1, 0]).unwrap(),
        -&g
    );

    let triple = FermionicOp::excitation_generator(6, &[2, 0, 1], &[4, 5, 3]).unwrap();
    assert_matrix_close(
        &fock_matrix(&triple),
        &excitation_fock_matrix(6, &[2, 0, 1], &[4, 5, 3]),
    );
}

#[test]
fn excitation_generator_single_and_double_have_expected_fock_signs() {
    let single = FermionicOp::excitation_generator(3, &[0], &[2]).unwrap();
    let mut expected = Array2::<Complex64>::zeros((8, 8));
    let i = Complex64::new(0.0, 1.0);
    expected[[4, 1]] = i;
    expected[[1, 4]] = -i;
    // A spectator in mode 1 flips the single-excitation sign.
    expected[[6, 3]] = -i;
    expected[[3, 6]] = i;
    assert_eq!(fock_matrix(&single), expected);

    let double = FermionicOp::excitation_generator(4, &[0, 1], &[2, 3]).unwrap();
    let mut expected = Array2::<Complex64>::zeros((16, 16));
    expected[[12, 3]] = i;
    expected[[3, 12]] = -i;
    assert_eq!(fock_matrix(&double), expected);
}

#[test]
fn excitation_generators_match_fock_action_and_conserve_particle_number() {
    let mapper = JordanWignerMapper::new();
    let check = |n, sources: &[usize], targets: &[usize]| {
        let op = FermionicOp::excitation_generator(n, sources, targets).unwrap();
        let expected = excitation_fock_matrix(n, sources, targets);
        assert_eq!(fock_matrix(&op), expected);
        assert_eq!(expected, expected.t().mapv(|c| c.conj()));
        assert!(op.is_hermitian(0.0).unwrap());
        assert_eq!(fock_matrix(&op.normal_order().unwrap()), expected);
        assert_matrix_close(
            &mapper.map_operator(&op).unwrap().to_matrix().unwrap(),
            &expected,
        );
        for ((row, col), value) in expected.indexed_iter() {
            if value.norm() != 0.0 {
                assert_eq!(row.count_ones(), col.count_ones());
            }
        }
        let number = FermionicOp::total_number(n);
        assert!(
            number
                .try_mul(&op)
                .unwrap()
                .try_sub(&op.try_mul(&number).unwrap())
                .unwrap()
                .normal_order()
                .unwrap()
                .is_zero()
        );
    };
    // Every ordered, disjoint single and double excitation for up to five modes.
    for n in 2..=5 {
        for source in 0..n {
            for target in 0..n {
                if source == target {
                    continue;
                }
                check(n, &[source], &[target]);
                for source2 in 0..n {
                    if [source, target].contains(&source2) {
                        continue;
                    }
                    for target2 in 0..n {
                        if [source, target, source2].contains(&target2) {
                            continue;
                        }
                        check(n, &[source, source2], &[target, target2]);
                    }
                }
            }
        }
    }
    check(6, &[2, 0, 1], &[4, 5, 3]);
}

#[test]
fn excitation_generator_evolution_matches_independent_fock_exponential() {
    let mapper = JordanWignerMapper::new();
    for (n, sources, targets) in [
        (3, vec![0], vec![2]),
        (3, vec![2], vec![0]),
        (4, vec![0, 1], vec![2, 3]),
        (5, vec![1, 0], vec![4, 2]),
    ] {
        let op = FermionicOp::excitation_generator(n, &sources, &targets).unwrap();
        let mapped = mapper.map_operator(&op).unwrap();
        assert!(mapped.all_terms_commute());
        let k =
            excitation_fock_matrix(n, &sources, &targets).mapv(|g| Complex64::new(0.0, -1.0) * g);
        for time in [-0.37, 0.0, 0.61] {
            assert_matrix_close(
                &bound_evolution(mapped.clone(), time),
                &matrix_exp(&k, time),
            );
        }
    }
}

#[test]
fn composed_excitations_have_independent_parameters_and_correct_statevectors() {
    let mapper = JordanWignerMapper::new();
    let occupation = mapper.map_occupation(&[1, 1, 0, 0]).unwrap();
    let mut circuit = BasisEncoding::new(occupation)
        .build_circuit("reference")
        .unwrap();
    for (sources, targets, name) in [
        (vec![0, 1], vec![2, 3], "theta_double"),
        (vec![0], vec![2], "theta_single"),
    ] {
        let g = FermionicOp::excitation_generator(4, &sources, &targets).unwrap();
        let block = PauliEvolutionAnsatz::new(mapper.map_operator(&g).unwrap())
            .unwrap()
            .with_time_param_name(name)
            .with_strategy(EvolutionStrategy::Exact)
            .build_circuit("excitation")
            .unwrap();
        assert_eq!(block.symbols().len(), 1);
        circuit.compose(&block, None).unwrap();
    }
    assert_eq!(circuit.symbols().len(), 2);
    assert!(circuit.symbols().contains("theta_double"));
    assert!(circuit.symbols().contains("theta_single"));
    let number = mapper.map_operator(&FermionicOp::total_number(4)).unwrap();
    for (double, single) in [
        (0.0_f64, 0.0_f64),
        (0.37, 0.0),
        (0.0, -0.21),
        (0.37, -0.21),
        (-0.43, 0.19),
    ] {
        // Bind in two stages to exercise independence and partial binding.
        let partial = circuit
            .assign_parameters(&Some(HashMap::from([("theta_double", double)])))
            .unwrap();
        assert_eq!(partial.symbols().len(), 1);
        assert!(partial.symbols().contains("theta_single"));
        let bound = partial
            .assign_parameters(&Some(HashMap::from([("theta_single", single)])))
            .unwrap();
        assert!(bound.symbols().is_empty());
        let state = Statevector::from_circuit(&bound).unwrap();
        let mut expected = vec![Complex64::new(0.0, 0.0); 16];
        expected[3] = (double.cos() * single.cos()).into();
        expected[6] = (-double.cos() * single.sin()).into();
        expected[12] = (double.sin() * single.cos()).into();
        expected[9] = (-double.sin() * single.sin()).into();
        for (actual, expected) in state.data().iter().zip(expected) {
            assert!((*actual - expected).norm() < 1e-11);
        }
        assert!((number.expectation_statevector(&state).unwrap() - 2.0).abs() < 1e-11);
    }
}

fn matrix_exp(matrix: &Array2<Complex64>, scale: f64) -> Array2<Complex64> {
    let mut term = Array2::<Complex64>::eye(matrix.nrows());
    let mut result = term.clone();
    for order in 1..=48 {
        term = term.dot(matrix).mapv(|c| c * scale / order as f64);
        result += &term;
    }
    result
}

fn bound_evolution(h: Hamiltonian, time: f64) -> Array2<Complex64> {
    let circuit = PauliEvolutionAnsatz::new(h)
        .unwrap()
        .build_circuit("fermion")
        .unwrap();
    let bindings = HashMap::from([("fermion_t", time)]);
    let circuit = circuit.assign_parameters(&Some(bindings)).unwrap();
    circuit_to_matrix(&circuit, None).unwrap()
}

fn pauli_terms(h: &Hamiltonian) -> BTreeMap<String, Complex64> {
    assert!(h.terms.iter().all(|(p, _)| p.phase == Phase::Plus));
    h.terms.iter().map(|(p, c)| (p.to_string(), *c)).collect()
}

#[test]
fn canonical_anticommutation_relations() {
    for p in 0..4 {
        for q in 0..4 {
            let a = FermionicOp::annihilation(4, p).unwrap();
            let b = FermionicOp::annihilation(4, q).unwrap();
            let bd = b.adjoint();
            let anticommutator = (a.try_mul(&bd).unwrap() + bd.try_mul(&a).unwrap()).unwrap();
            let expected = if p == q {
                FermionicOp::identity(4)
            } else {
                FermionicOp::new(4)
            };
            assert_eq!(anticommutator.normal_order().unwrap(), expected);
            let annihilators = (a.try_mul(&b).unwrap() + b.try_mul(&a).unwrap()).unwrap();
            assert!(annihilators.normal_order().unwrap().is_zero());
            let creators =
                (a.adjoint().try_mul(&bd).unwrap() + bd.try_mul(&a.adjoint()).unwrap()).unwrap();
            assert!(creators.normal_order().unwrap().is_zero());
        }
    }
}

#[test]
fn contractions_and_normal_order_preserve_remaining_branches() {
    let a = FermionicOp::annihilation(3, 0).unwrap();
    let ad = a.adjoint();
    let hole = (&a * &ad).unwrap();
    assert_eq!(
        hole.normal_order().unwrap(),
        (FermionicOp::identity(3) - FermionicOp::number(3, 0).unwrap()).unwrap()
    );
    assert_eq!((&hole * &a).unwrap().normal_order().unwrap(), a);
    assert_eq!((&ad * &hole).unwrap().normal_order().unwrap(), ad);
    let n = FermionicOp::number(3, 0).unwrap();
    assert_eq!((&n * &n).unwrap().normal_order().unwrap(), n);
    let input = FermionicOp::from_term(
        3,
        [Annihilate(0), Create(0), Create(1), Annihilate(2)],
        1.0.into(),
    )
    .unwrap();
    let expected = FermionicOp::from_terms(
        3,
        [
            (vec![Create(1), Annihilate(2)], 1.0.into()),
            (
                vec![Create(0), Create(1), Annihilate(0), Annihilate(2)],
                1.0.into(),
            ),
        ],
    )
    .unwrap();
    assert_eq!(input.normal_order().unwrap(), expected);
    assert_eq!(expected.normal_order().unwrap(), expected);
}

#[test]
fn long_single_branch_normal_order_preserves_sign_and_nilpotency() {
    for n in [399, 400, 799, 800] {
        let word: Vec<_> = (0..n).rev().map(Create).collect();
        let input = FermionicOp::from_term(n, word.clone(), 1.0.into()).unwrap();
        let sign = if (n * (n - 1) / 2) % 2 == 0 {
            1.0
        } else {
            -1.0
        };
        let expected = FermionicOp::from_term(n, (0..n).map(Create), sign.into()).unwrap();
        assert_eq!(input.normal_order().unwrap(), expected);
        assert_eq!(expected.normal_order().unwrap(), expected);

        let mut repeated = word;
        repeated.push(Create(n - 1));
        assert!(
            FermionicOp::from_term(n, repeated, 1.0.into())
                .unwrap()
                .normal_order()
                .unwrap()
                .is_zero()
        );
    }
}

#[test]
fn multiplication_keeps_order_and_adjoint_reverses_it() {
    let a =
        FermionicOp::from_term(3, [Create(2), Annihilate(0)], Complex64::new(2.0, 3.0)).unwrap();
    let expected =
        FermionicOp::from_term(3, [Create(0), Annihilate(2)], Complex64::new(2.0, -3.0)).unwrap();
    assert_eq!(a.adjoint(), expected);
    assert_eq!(a.adjoint().adjoint(), a);
    let b = FermionicOp::creation(3, 1).unwrap();
    let product = (&a * &b).unwrap();
    assert_eq!(
        product.terms().next().unwrap().0,
        [Create(2), Annihilate(0), Create(1)]
    );
    assert_eq!(product.adjoint(), (&b.adjoint() * &a.adjoint()).unwrap());
    assert_eq!(Create(1).adjoint(), Annihilate(1));
}

#[test]
fn exact_merging_explicit_chopping_and_structural_equality() {
    let mut op = FermionicOp::from_terms(
        4,
        [
            (vec![Create(1)], 0.5.into()),
            (vec![Create(1)], (-0.5).into()),
            (vec![Annihilate(3)], Complex64::new(1e-14, 0.0)),
        ],
    )
    .unwrap();
    assert_eq!(op.num_terms(), 1);
    assert_eq!(op.num_modes(), 4);
    op.chop(0.0).unwrap();
    assert_eq!(op.num_terms(), 1);
    op.chop(1e-14).unwrap();
    assert_eq!(op, FermionicOp::new(4));
    let input = FermionicOp::from_term(2, [Create(1), Create(0)], 1.0.into()).unwrap();
    let ordered = FermionicOp::from_term(2, [Create(0), Create(1)], (-1.0).into()).unwrap();
    assert_ne!(input, ordered);
    assert!(input.equiv(&ordered, 0.0).unwrap());
    assert_ne!(FermionicOp::new(1), FermionicOp::new(2));
}

#[test]
fn fallible_owned_and_borrowed_arithmetic() {
    let a = FermionicOp::creation(2, 0).unwrap();
    let b = FermionicOp::annihilation(2, 1).unwrap();
    let sum = a.try_add(&b).unwrap();
    assert_eq!((&a + &b).unwrap(), sum);
    assert_eq!((a.clone() + &b).unwrap(), sum);
    assert_eq!((&a + b.clone()).unwrap(), sum);
    assert_eq!((a.clone() + b.clone()).unwrap(), sum);
    let difference = a.try_sub(&b).unwrap();
    assert_eq!((&a - &b).unwrap(), difference);
    assert_eq!((a.clone() - &b).unwrap(), difference);
    assert_eq!((&a - b.clone()).unwrap(), difference);
    assert_eq!((a.clone() - b.clone()).unwrap(), difference);
    let product = a.try_mul(&b).unwrap();
    assert_eq!((&a * &b).unwrap(), product);
    assert_eq!((a.clone() * &b).unwrap(), product);
    assert_eq!((&a * b.clone()).unwrap(), product);
    assert_eq!((a.clone() * b.clone()).unwrap(), product);
    for factor in [Complex64::new(2.0, 0.0), Complex64::new(0.0, 1.0)] {
        let scaled = a.scaled(factor).unwrap();
        assert_eq!((&a * factor).unwrap(), scaled);
        assert_eq!((a.clone() * factor).unwrap(), scaled);
        assert_eq!((factor * &a).unwrap(), scaled);
        assert_eq!((factor * a.clone()).unwrap(), scaled);
    }
    assert_eq!((&a * 2.0).unwrap(), (2.0 * &a).unwrap());
    assert_eq!((a.clone() * 2.0).unwrap(), (2.0 * a.clone()).unwrap());
    assert_eq!(-&a, -a.clone());
    assert!((&a + -&a).unwrap().is_zero());
}

#[test]
fn invalid_inputs_and_failed_mutation_do_not_corrupt_operators() {
    assert!(matches!(
        FermionicOp::creation(0, 0),
        Err(QisError::ModeOutOfBounds {
            mode: 0,
            num_modes: 0
        })
    ));
    assert!(FermionicOp::annihilation(2, 2).is_err());
    assert!(FermionicOp::number(1, usize::MAX).is_err());
    assert!(FermionicOp::from_term(1, [Create(2)], 0.0.into()).is_err());
    let a = FermionicOp::creation(1, 0).unwrap();
    let b = FermionicOp::new(2);
    for result in [&a + &b, &a - &b, &a * &b] {
        assert!(matches!(
            result,
            Err(QisError::ModeMismatch {
                expected: 1,
                actual: 2
            })
        ));
    }
    assert!(a.equiv(&b, 0.0).is_err());
    for c in [
        Complex64::new(f64::NAN, 0.0),
        Complex64::new(0.0, f64::INFINITY),
    ] {
        assert!(matches!(
            FermionicOp::from_term(1, [], c),
            Err(QisError::NonFiniteCoefficient { .. })
        ));
        assert!(FermionicOp::new(1).scaled(c).is_err());
    }
    let mut large = FermionicOp::from_term(1, [Create(0)], f64::MAX.into()).unwrap();
    let saved = large.clone();
    assert!(large.add_term([Create(0)], f64::MAX.into()).is_err());
    assert_eq!(large, saved);
    assert!(large.add_term([Create(1)], 1.0.into()).is_err());
    assert_eq!(large, saved);
    assert!(large.scaled(2.0.into()).is_err());
    for tolerance in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(large.chop(tolerance).is_err());
        assert_eq!(large, saved);
        assert!(large.is_hermitian(tolerance).is_err());
        assert!(large.equiv(&saved, tolerance).is_err());
    }
}

#[test]
fn hermiticity_checks_use_fermionic_algebra() {
    let hole = FermionicOp::from_term(2, [Annihilate(0), Create(0)], 1.0.into()).unwrap();
    assert!(hole.is_hermitian(0.0).unwrap());
    let t = FermionicOp::from_term(2, [Create(1), Annihilate(0)], 1.0.into()).unwrap();
    assert!(!t.is_hermitian(0.0).unwrap());
    let generator = (&t - t.adjoint())
        .unwrap()
        .scaled(Complex64::new(0.0, 1.0))
        .unwrap();
    assert!(generator.is_hermitian(0.0).unwrap());
    assert!(
        !FermionicOp::identity(0)
            .scaled(Complex64::new(0.0, 1.0))
            .unwrap()
            .is_hermitian(0.0)
            .unwrap()
    );
}

#[test]
fn ladder_number_hole_and_identity_mappings() {
    let mapper = JordanWignerMapper::new();
    let creation = mapper
        .map_operator(&FermionicOp::creation(4, 2).unwrap())
        .unwrap();
    assert_eq!(
        pauli_terms(&creation),
        BTreeMap::from([
            ("+IXZZ".into(), 0.5.into()),
            ("+IYZZ".into(), Complex64::new(0.0, -0.5)),
        ])
    );
    let annihilation = mapper
        .map_operator(&FermionicOp::annihilation(4, 2).unwrap())
        .unwrap();
    assert_eq!(
        pauli_terms(&annihilation),
        BTreeMap::from([
            ("+IXZZ".into(), 0.5.into()),
            ("+IYZZ".into(), Complex64::new(0.0, 0.5)),
        ])
    );
    assert_eq!(
        pauli_terms(
            &mapper
                .map_operator(&FermionicOp::number(4, 2).unwrap())
                .unwrap()
        ),
        BTreeMap::from([
            ("+IIII".into(), 0.5.into()),
            ("+IZII".into(), (-0.5).into()),
        ])
    );
    let hole = FermionicOp::from_term(4, [Annihilate(2), Create(2)], 1.0.into()).unwrap();
    assert_eq!(
        pauli_terms(&mapper.map_operator(&hole).unwrap()),
        BTreeMap::from([("+IIII".into(), 0.5.into()), ("+IZII".into(), 0.5.into()),])
    );
    assert_eq!(
        pauli_terms(&mapper.map_operator(&FermionicOp::identity(4)).unwrap()),
        BTreeMap::from([("+IIII".into(), 1.0.into())])
    );
}

#[test]
fn zero_scalar_and_wide_mode_spaces_are_preserved() {
    let mapper = JordanWignerMapper::new();
    for n in [0, 1, 4, 129] {
        let zero = mapper.map_operator(&FermionicOp::new(n)).unwrap();
        assert_eq!(zero.num_qubits, n);
        assert!(zero.terms.is_empty());
        assert_eq!(mapper.num_qubits(n), n);
    }
    let scalar = FermionicOp::identity(0)
        .scaled(Complex64::new(2.0, 3.0))
        .unwrap();
    assert_eq!(
        mapper.map_operator(&scalar).unwrap().to_matrix().unwrap()[[0, 0]],
        Complex64::new(2.0, 3.0)
    );
    assert!(FermionicOp::total_number(0).is_zero());
    let wide = mapper
        .map_operator(&FermionicOp::creation(129, 128).unwrap())
        .unwrap();
    assert_eq!(wide.num_qubits, 129);
    assert_eq!(wide.terms.len(), 2);
    assert!(
        wide.terms
            .iter()
            .all(|(p, _)| (0..128).all(|j| p.get_pauli(j) == crate::qis::Pauli::Z))
    );
    let nilpotent = FermionicOp::from_term(4, [Create(0), Create(0)], 1.0.into()).unwrap();
    let mapped = mapper.map_operator(&nilpotent).unwrap();
    assert!(mapped.terms.is_empty());
    assert_eq!(mapped.num_qubits, 4);
}

fn check_word(n: usize, word: Vec<FermionicLadder>) {
    let op = FermionicOp::from_term(n, word, 1.0.into()).unwrap();
    let normal = op.normal_order().unwrap();
    assert_eq!(normal.normal_order().unwrap(), normal);
    let expected = fock_matrix(&op);
    assert_matrix_close(&fock_matrix(&normal), &expected);
    let mapper = JordanWignerMapper::new();
    assert_matrix_close(
        &mapper.map_operator(&op).unwrap().to_matrix().unwrap(),
        &expected,
    );
    assert_matrix_close(
        &mapper.map_operator(&normal).unwrap().to_matrix().unwrap(),
        &expected,
    );
}

#[test]
fn exhaustive_short_products_match_independent_fock_matrices() {
    for n in 1usize..=3 {
        for length in 0u32..=4 {
            for mut encoded in 0..(2 * n).pow(length) {
                let word = (0..length)
                    .map(|_| {
                        let symbol = encoded % (2 * n);
                        encoded /= 2 * n;
                        if symbol < n {
                            Create(symbol)
                        } else {
                            Annihilate(symbol - n)
                        }
                    })
                    .collect();
                check_word(n, word);
            }
        }
    }
}

#[test]
fn fixed_seed_longer_products_match_independent_fock_matrices() {
    let mut seed = 20261009u64;
    for sample in 0..256 {
        let word = (0..sample % 9)
            .map(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let symbol = (seed >> 32) as usize % 8;
                if symbol < 4 {
                    Create(symbol)
                } else {
                    Annihilate(symbol - 4)
                }
            })
            .collect();
        check_word(4, word);
    }
}

#[test]
fn mapping_preserves_linear_combinations_products_and_adjoint() {
    let mapper = JordanWignerMapper::new();
    let a = FermionicOp::from_terms(
        3,
        [
            (vec![Annihilate(1), Create(2)], Complex64::new(1.0, 2.0)),
            (vec![], Complex64::new(-2.0, 1.0)),
        ],
    )
    .unwrap();
    let b = FermionicOp::creation(3, 0).unwrap();
    let ma = mapper.map_operator(&a).unwrap().to_matrix().unwrap();
    let mb = mapper.map_operator(&b).unwrap().to_matrix().unwrap();
    assert_matrix_close(&ma, &fock_matrix(&a));
    assert_matrix_close(
        &mapper
            .map_operator(&(&a + &b).unwrap())
            .unwrap()
            .to_matrix()
            .unwrap(),
        &(&ma + &mb),
    );
    assert_matrix_close(
        &mapper
            .map_operator(&(&a * &b).unwrap())
            .unwrap()
            .to_matrix()
            .unwrap(),
        &ma.dot(&mb),
    );
    assert_matrix_close(
        &mapper
            .map_operator(&a.adjoint())
            .unwrap()
            .to_matrix()
            .unwrap(),
        &ma.t().mapv(|c| c.conj()),
    );
}

#[test]
fn occupation_encoding_prepares_basis_states_and_number_observables() {
    let mapper = JordanWignerMapper::new();
    let operator_mapper: &dyn FermionToQubitMapper = &mapper;
    let state_mapper: &dyn OccupationStateMapper = &mapper;
    for n in 1..=4 {
        let number = operator_mapper
            .map_operator(&FermionicOp::total_number(n))
            .unwrap();
        for state in 0usize..1 << n {
            let occupation: Vec<_> = (0..n).map(|p| ((state >> p) & 1) as u8).collect();
            let bits = state_mapper.map_occupation(&occupation).unwrap();
            let circuit = BasisEncoding::new(bits)
                .build_circuit("occupation")
                .unwrap();
            let sv = Statevector::from_circuit(&circuit).unwrap();
            assert_eq!(sv.probabilities()[state], 1.0);
            assert_eq!(
                number.expectation_statevector(&sv).unwrap(),
                state.count_ones() as f64
            );
            assert_eq!(number.variance_statevector(&sv).unwrap(), 0.0);
            let dm = DensityMatrix::from_circuit(&circuit).unwrap();
            assert_eq!(
                number.expectation_density_matrix(&dm).unwrap(),
                state.count_ones() as f64
            );
            for (p, occupation_value) in occupation.iter().enumerate() {
                let np = operator_mapper
                    .map_operator(&FermionicOp::number(n, p).unwrap())
                    .unwrap();
                assert_eq!(
                    np.expectation_statevector(&sv).unwrap(),
                    *occupation_value as f64
                );
            }
        }
    }
    assert_eq!(mapper.map_occupation(&[]).unwrap(), Vec::<bool>::new());
    for invalid in [2, 255] {
        assert!(
            matches!(mapper.map_occupation(&[1, invalid]), Err(QisError::InvalidOccupation { index: 1, value }) if value == invalid)
        );
    }
}

#[test]
fn single_and_double_excitation_evolution_matches_fock_exponentials() {
    let mapper = JordanWignerMapper::new();
    for (n, word) in [
        (3, vec![Create(2), Annihilate(0)]),
        (4, vec![Create(2), Create(3), Annihilate(1), Annihilate(0)]),
    ] {
        let t = FermionicOp::from_term(n, word, 1.0.into()).unwrap();
        let k = (&t - t.adjoint()).unwrap();
        let h = k.scaled(Complex64::new(0.0, 1.0)).unwrap();
        let mapped = mapper.map_operator(&h).unwrap();
        assert!(mapped.all_terms_commute());
        let time = 0.37;
        let expected = matrix_exp(&fock_matrix(&k), time);
        assert_matrix_close(&bound_evolution(mapped.clone(), time), &expected);
        let numeric = mapped
            .to_evolution_circuit(time, 1, TrotterMode::FirstOrder)
            .unwrap();
        assert_matrix_close(&circuit_to_matrix(&numeric, None).unwrap(), &expected);
        let particle_number = fock_matrix(&FermionicOp::total_number(n));
        let generator = fock_matrix(&k);
        assert_matrix_close(
            &particle_number.dot(&generator),
            &generator.dot(&particle_number),
        );
    }
}

#[test]
fn reversing_double_excitation_annihilators_changes_the_sign() {
    let a = FermionicOp::from_term(
        4,
        [Create(2), Create(3), Annihilate(1), Annihilate(0)],
        1.0.into(),
    )
    .unwrap();
    let b = FermionicOp::from_term(
        4,
        [Create(2), Create(3), Annihilate(0), Annihilate(1)],
        1.0.into(),
    )
    .unwrap();
    assert!((&a + &b).unwrap().normal_order().unwrap().is_zero());
    assert_eq!(fock_matrix(&a)[[12, 3]], Complex64::new(1.0, 0.0));
    assert_matrix_close(&fock_matrix(&b), &fock_matrix(&a).mapv(|c| -c));
}

#[test]
fn small_coefficients_survive_mapping_and_both_evolution_apis() {
    let mapper = JordanWignerMapper::new();
    let t = FermionicOp::creation(1, 0).unwrap();
    let k = (&t - t.adjoint()).unwrap();
    let tiny = k.scaled(Complex64::new(0.0, 1e-12)).unwrap();
    let mapped = mapper.map_operator(&tiny).unwrap();
    assert_eq!(mapped.terms.len(), 1);
    assert_eq!(mapped.terms[0].1, Complex64::new(1e-12, 0.0));
    assert_eq!(
        PauliEvolutionAnsatz::new(mapped.clone())
            .unwrap()
            .evolution_info()
            .num_terms,
        1
    );
    // Large time makes loss of a small coefficient observable in the matrix.
    let numeric = mapped
        .to_evolution_circuit(1e12, 1, TrotterMode::FirstOrder)
        .unwrap();
    let expected = matrix_exp(&fock_matrix(&k), 1.0);
    assert_matrix_close(&circuit_to_matrix(&numeric, None).unwrap(), &expected);
    assert_matrix_close(&bound_evolution(mapped, 1e12), &expected);
}

#[test]
fn identity_evolution_preserves_global_phase_and_nonhermitian_mapping_is_allowed() {
    let mapper = JordanWignerMapper::new();
    let h = mapper
        .map_operator(&FermionicOp::identity(2).scaled(0.3.into()).unwrap())
        .unwrap();
    let expected = Array2::<Complex64>::eye(4).mapv(|c| c * Complex64::from_polar(1.0, -0.15));
    assert_matrix_close(&bound_evolution(h, 0.5), &expected);
    let creation = mapper
        .map_operator(&FermionicOp::creation(2, 0).unwrap())
        .unwrap();
    assert!(matches!(
        PauliEvolutionAnsatz::new(creation),
        Err(QisError::NotHermitian)
    ));
    assert!(PauliEvolutionAnsatz::new(mapper.map_operator(&FermionicOp::new(2)).unwrap()).is_err());
}

#[test]
fn mapping_reports_coefficient_overflow_after_pauli_merging() {
    let op = FermionicOp::from_terms(
        1,
        [
            (vec![], f64::MAX.into()),
            (vec![Create(0), Annihilate(0)], f64::MAX.into()),
            (vec![Annihilate(0), Create(0)], f64::MAX.into()),
        ],
    )
    .unwrap();
    assert!(matches!(
        JordanWignerMapper::new().map_operator(&op),
        Err(QisError::NonFiniteCoefficient { .. })
    ));
}

#[test]
fn mapping_applies_subnormal_weights_after_merging_the_unit_product() {
    let tiny = f64::from_bits(2);
    let half = f64::from_bits(1);
    let mapper = JordanWignerMapper::new();
    for coefficient in [
        Complex64::new(tiny, 0.0),
        Complex64::new(0.0, -tiny),
        Complex64::new(tiny, -tiny),
    ] {
        for (word, z_sign) in [
            (vec![Create(0), Annihilate(0)], -1.0),
            (vec![Annihilate(0), Create(0)], 1.0),
            (
                vec![Create(0), Annihilate(0), Create(0), Annihilate(0)],
                -1.0,
            ),
        ] {
            let input = FermionicOp::from_term(1, word, coefficient).unwrap();
            let mapped = mapper.map_operator(&input).unwrap();
            let expected_half = Complex64::new(
                if coefficient.re == 0.0 { 0.0 } else { half },
                if coefficient.im == 0.0 { 0.0 } else { -half },
            );
            // Compare coefficients exactly: a matrix tolerance would hide subnormals.
            assert_eq!(
                pauli_terms(&mapped),
                BTreeMap::from([
                    ("+I".into(), expected_half),
                    ("+Z".into(), expected_half * z_sign),
                ])
            );
        }
        let identity = FermionicOp::identity(1).scaled(coefficient).unwrap();
        assert_eq!(
            pauli_terms(&mapper.map_operator(&identity).unwrap()),
            BTreeMap::from([("+I".into(), coefficient)])
        );
    }
}

#[test]
fn normal_order_and_arithmetic_report_coefficient_overflow() {
    let op = FermionicOp::from_terms(
        1,
        [
            (vec![Create(0)], f64::MAX.into()),
            (vec![Create(0), Annihilate(0), Create(0)], f64::MAX.into()),
        ],
    )
    .unwrap();
    assert!(matches!(
        op.normal_order(),
        Err(QisError::NonFiniteCoefficient { .. })
    ));
    let a = FermionicOp::from_term(1, [Create(0)], f64::MAX.into()).unwrap();
    assert!((&a + &a).is_err());
    let twice_identity = FermionicOp::identity(1).scaled(2.0.into()).unwrap();
    assert!((&a * twice_identity).is_err());
    let tiny = FermionicOp::creation(1, 0)
        .unwrap()
        .scaled(1e-310.into())
        .unwrap();
    let mut retained = tiny.clone();
    retained.chop(0.0).unwrap();
    assert_eq!(retained, tiny);
    assert_eq!(
        JordanWignerMapper::new()
            .map_operator(&tiny)
            .unwrap()
            .terms
            .len(),
        2
    );
}

#[test]
fn noncommuting_excitation_sum_uses_the_same_numeric_and_symbolic_product_formula() {
    let first = FermionicOp::from_term(3, [Create(1), Annihilate(0)], 1.0.into()).unwrap();
    let second = FermionicOp::from_term(3, [Create(2), Annihilate(1)], 1.0.into()).unwrap();
    let t = (&first + &second).unwrap();
    let k = (&t - t.adjoint()).unwrap();
    let h = JordanWignerMapper::new()
        .map_operator(&k.scaled(Complex64::new(0.0, 1.0)).unwrap())
        .unwrap();
    assert!(!h.all_terms_commute());
    assert!(
        PauliEvolutionAnsatz::new(h.clone())
            .unwrap()
            .with_strategy(EvolutionStrategy::Exact)
            .build_circuit("exact")
            .is_err()
    );
    let time = 0.2;
    let numeric = h
        .to_evolution_circuit(time, 8, TrotterMode::SecondOrder)
        .unwrap();
    let numeric = circuit_to_matrix(&numeric, None).unwrap();
    let circuit = PauliEvolutionAnsatz::new(h)
        .unwrap()
        .with_strategy(EvolutionStrategy::Trotter {
            mode: TrotterMode::SecondOrder,
            steps: 8,
        })
        .build_circuit("sum")
        .unwrap();
    let circuit = circuit
        .assign_parameters(&Some(HashMap::from([("sum_t", time)])))
        .unwrap();
    assert_matrix_close(&numeric, &circuit_to_matrix(&circuit, None).unwrap());
    let expected = matrix_exp(&fock_matrix(&k), time);
    assert!((&numeric - &expected).iter().all(|c| c.norm() < 1e-3));
}

#[test]
fn all_hamiltonian_consumers_validate_original_and_merged_inputs() {
    let sv = Statevector::new(1);
    let dm = DensityMatrix::new(1);
    let x: PauliString = "X".parse().unwrap();
    let mut malformed = x.clone();
    malformed.z.clear();
    let invalid = [
        Hamiltonian {
            num_qubits: 1,
            terms: vec![(PauliString::new(2), 1.0.into())],
        },
        Hamiltonian {
            num_qubits: 1,
            terms: vec![(malformed, 1.0.into())],
        },
        Hamiltonian {
            num_qubits: 1,
            terms: vec![(x.clone(), f64::NAN.into())],
        },
        Hamiltonian {
            num_qubits: 1,
            terms: vec![
                (x.clone(), f64::INFINITY.into()),
                (x.clone(), f64::NEG_INFINITY.into()),
            ],
        },
        Hamiltonian {
            num_qubits: 1,
            terms: vec![(x.clone(), f64::MAX.into()), (x.clone(), f64::MAX.into())],
        },
    ];
    for h in invalid {
        assert!(h.expectation_statevector(&sv).is_err());
        assert!(h.expectation_density_matrix(&dm).is_err());
        assert!(h.expectation_probs(&[]).is_err());
        assert!(h.variance_statevector(&sv).is_err());
        assert!(
            h.to_evolution_circuit(1.0, 1, TrotterMode::FirstOrder)
                .is_err()
        );
        assert!(
            h.to_trotter_circuit(1.0, 1, TrotterMode::FirstOrder)
                .is_err()
        );
        assert!(PauliEvolutionAnsatz::new(h).is_err());
    }
    let zero = Hamiltonian::from_list(vec![(x.clone(), 1.0.into()), (x, (-1.0).into())]).unwrap();
    assert_eq!(zero.expectation_statevector(&sv).unwrap(), 0.0);
    assert_eq!(zero.expectation_probs(&[]).unwrap(), 0.0);
    for mode in [
        TrotterMode::FirstOrder,
        TrotterMode::SecondOrder,
        TrotterMode::Randomized(7),
    ] {
        for circuit in [
            zero.to_evolution_circuit(1.0, 3, mode).unwrap(),
            zero.to_trotter_circuit(1.0, 3, mode).unwrap(),
        ] {
            assert_eq!(circuit.num_qubits(), 1);
            assert_eq!(circuit_to_matrix(&circuit, None).unwrap(), Array2::eye(2));
        }
    }
    assert!(PauliEvolutionAnsatz::new(zero).is_err());
}

#[test]
fn hamiltonian_consumers_preserve_coefficient_error_categories() {
    let sv = Statevector::new(1);
    let dm = DensityMatrix::new(1);
    let x: PauliString = "X".parse().unwrap();
    for coefficients in [
        vec![f64::NAN],
        vec![f64::INFINITY],
        vec![f64::MAX, f64::MAX],
    ] {
        let h = Hamiltonian::from_list(
            coefficients
                .into_iter()
                .map(|c| (x.clone(), c.into()))
                .collect(),
        )
        .unwrap();
        for result in [
            h.expectation_statevector(&sv),
            h.expectation_density_matrix(&dm),
            h.expectation_probs(&[]),
            h.variance_statevector(&sv),
        ] {
            assert!(matches!(result, Err(QisError::InvalidParameterValue(_))));
        }
        for result in [
            h.to_evolution_circuit(1.0, 1, TrotterMode::FirstOrder),
            h.to_trotter_circuit(1.0, 1, TrotterMode::FirstOrder),
        ] {
            assert!(matches!(result, Err(QisError::UnsupportedOperation(_))));
        }
    }
}
