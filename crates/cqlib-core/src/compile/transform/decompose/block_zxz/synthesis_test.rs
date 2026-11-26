// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::super::factors::{self, adjoint, residual};
use super::*;
use crate::circuit::symbolic_matrix::SymbolicComplex;
use crate::circuit::{
    ClassicalControlOp, ClassicalExpr, Instruction, ParameterValue, StandardGate, UnitaryGate,
    storage_operation_to_value,
};
use crate::compile::transform::decompose::{
    decompose_unitaries, decompose_unitaries_with_rule_stats,
};
use num_complex::Complex64 as C;
use rand::{Rng, SeedableRng, rngs::StdRng};

pub(crate) fn seeded_unitary(n: usize, seed: u64) -> Matrix {
    let mut rng = StdRng::seed_from_u64(seed);
    let source = Matrix::from_shape_simple_fn((n, n), || {
        C::new(rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0))
    });
    let q = factors::to_faer(source.view()).qr().compute_Q();
    Matrix::from_shape_fn((n, n), |(i, j)| q[(i, j)])
}

pub(crate) fn assert_matrix_close(actual: &Matrix, expected: &Matrix, tol: f64) {
    assert_eq!(actual.dim(), expected.dim());
    let error = residual(actual, expected);
    assert!(
        error <= tol,
        "maximum matrix error {error:e}, allowed {tol:e}"
    );
}

#[test]
fn unoptimized_then_a1_then_a2() {
    for (n, counts) in [(8, [24, 22, 21, 19]), (16, [120, 110, 105, 95])] {
        let u = seeded_unitary(n, 7);
        for (index, (opt_a1, opt_a2)) in
            [(false, false), (true, false), (false, true), (true, true)]
                .into_iter()
                .enumerate()
        {
            let options = BlockZxzDecomposeOptions {
                opt_a1,
                opt_a2,
                opt_a2_force: false,
            };
            let c = decompose_block_zxz_with_options(&u, 1e-10, &options).unwrap();
            assert_matrix_close(&c.to_matrix(None).unwrap(), &u, 1e-10);
            let cx = c
                .operations()
                .iter()
                .filter(|op| op.instruction.standard_gate() == Some(StandardGate::CX))
                .count();
            assert_eq!(cx, counts[index], "dim {n}, {options:?}");
        }
    }
}

#[test]
fn base_cases_structured_and_five_qubit_phase() {
    let scalar = C::from_polar(1.0, 0.321);
    let roll = Matrix::from_shape_fn(
        (8, 8),
        |(i, j)| if i == (j + 1) % 8 { C::ONE } else { C::ZERO },
    );
    let diagonal = Matrix::from_diag(&Array1::from_shape_fn(8, |i| {
        C::from_polar(1.0, (i * i) as f64 * 0.17)
    }));
    for u in [
        seeded_unitary(2, 2) * scalar,
        seeded_unitary(4, 3) * scalar,
        Matrix::eye(8),
        Matrix::eye(8) * scalar,
        diagonal,
        roll,
        seeded_unitary(32, 13) * scalar,
    ] {
        assert_matrix_close(
            &decompose_block_zxz(&u, 1e-10)
                .unwrap()
                .to_matrix(None)
                .unwrap(),
            &u,
            1e-10,
        );
    }
}

#[test]
fn block_diagonal_policy() {
    let mut u = Matrix::zeros((8, 8));
    u.slice_mut(ndarray::s![..4, ..4])
        .assign(&seeded_unitary(4, 9));
    u.slice_mut(ndarray::s![4.., 4..])
        .assign(&seeded_unitary(4, 11));
    let run = |opt_a2, opt_a2_force| {
        decompose_block_zxz_with_options(
            &u,
            1e-10,
            &BlockZxzDecomposeOptions {
                opt_a1: true,
                opt_a2,
                opt_a2_force,
            },
        )
        .unwrap()
    };
    let normal = run(true, false);
    assert_eq!(normal.operations(), run(false, false).operations());
    let forced = run(true, true);
    assert_ne!(normal.operations(), forced.operations());
    for c in [normal, forced] {
        assert_matrix_close(&c.to_matrix(None).unwrap(), &u, 1e-10);
    }
}

#[test]
fn schur_clusters_and_ucrz_branch_cut() {
    let pi = std::f64::consts::PI;
    let phases = ndarray::array![pi - 1e-9, -pi + 1e-9, 0.37, 0.37];
    let v = seeded_unitary(4, 8);
    let u1 = seeded_unitary(4, 12);
    let u0 = (&v * &phases.mapv(|p| C::from_polar(1.0, p)))
        .dot(&adjoint(&v))
        .dot(&u1);
    let factors::Demux { v, d, w } = demux(&u0, &u1, 1e-11).unwrap();
    // With 4×4 factors, 1e-11 resolves the 2e-9 phase separation
    // while allowing rounding in the clustered eigenspace.
    assert_matrix_close(&(&v * &d).dot(&w), &u0, 1e-11);
    assert_matrix_close(&(&v * &d.mapv(|z| z.conj())).dot(&w), &u1, 1e-11);
    let d = phases.mapv(|p| C::from_polar(1.0, p));
    let mut circuit = Circuit::new(3);
    ucrz::append(&mut circuit, &d, Chain::Full).unwrap();
    let expected = Matrix::from_diag(&ndarray::concatenate![
        ndarray::Axis(0),
        d,
        d.mapv(|z| z.conj())
    ]);
    assert_matrix_close(&circuit.to_matrix(None).unwrap(), &expected, 1e-13);
}

#[test]
fn invalid_inputs() {
    for bad in [
        Matrix::zeros((2, 3)),
        Matrix::eye(1),
        Matrix::eye(3),
        Matrix::zeros((4, 4)),
        Matrix::eye(2) * C::new(f64::INFINITY, 0.0),
        Matrix::eye(2) * C::new(0.0, f64::NAN),
    ] {
        assert!(decompose_block_zxz(&bad, 1e-10).is_err());
    }
    for atol in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(decompose_block_zxz(&Matrix::eye(2), atol).is_err());
    }
}

#[test]
fn singular_quadrants_reconstruct_equation_five() {
    // The cyclic permutation has a rank-three X and rank-one Y.
    let u = Matrix::from_shape_fn((8, 8), |(i, j)| {
        if i == (j + 1) % 8 {
            C::from_polar(1.0, 0.23 * j as f64)
        } else {
            C::ZERO
        }
    });
    let b = split(&u).unwrap();
    let plus = (Matrix::eye(4) + &b.b) * C::new(0.5, 0.0);
    let minus = Matrix::eye(4) - &plus;
    let mut rebuilt = Matrix::zeros((8, 8));
    rebuilt
        .slice_mut(ndarray::s![..4, ..4])
        .assign(&b.a1.dot(&plus));
    rebuilt
        .slice_mut(ndarray::s![..4, 4..])
        .assign(&b.a1.dot(&minus).dot(&b.c));
    rebuilt
        .slice_mut(ndarray::s![4.., ..4])
        .assign(&b.a2.dot(&minus));
    rebuilt
        .slice_mut(ndarray::s![4.., 4..])
        .assign(&b.a2.dot(&plus).dot(&b.c));
    assert_matrix_close(&rebuilt, &u, 1e-13);
}

#[test]
fn block_zxz_cache_remaps_sparse_arguments_and_phase() {
    let gate = UnitaryGate::new("asymmetric", 3, 0)
        .with_matrix(seeded_unitary(8, 47))
        .unwrap();
    let mut circuit = Circuit::new(6);
    for args in [[5, 1, 3], [2, 4, 0]] {
        circuit
            .unitary(gate.clone(), args.map(Qubit::new).to_vec())
            .unwrap();
    }
    let (out, stats) = decompose_unitaries_with_rule_stats(&circuit, Default::default()).unwrap();
    assert_eq!((stats.hits, stats.misses), (1, 1));
    assert_matrix_close(
        &out.into_circuit(&circuit).to_matrix(None).unwrap(),
        &circuit.to_matrix(None).unwrap(),
        1e-10,
    );
}

#[test]
fn block_zxz_phase_is_local_to_the_control_flow_body() {
    let u = seeded_unitary(8, 37) * Complex64::from_polar(1., 0.73);
    let gate = UnitaryGate::new("conditional", 3, 0)
        .with_matrix(u.clone())
        .unwrap();
    let mut circuit = Circuit::new(3);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.append(
                    Instruction::UnitaryGate(Box::new(gate)),
                    [Qubit::new(2), Qubit::new(1), Qubit::new(0)],
                    Vec::<ParameterValue>::new(),
                    None,
                )
            },
            |_| Ok(()),
        )
        .unwrap();
    let out = decompose_unitaries(&circuit, Default::default()).unwrap();
    assert!(out.global_phase().is_zero());
    assert_matrix_close(&then_matrix(&out), &u, 1e-10);
}

fn then_matrix(out: &Circuit) -> Matrix {
    let Instruction::ClassicalControl(ClassicalControlOp::If(branch)) =
        &out.operations()[0].instruction
    else {
        panic!("expected conditional")
    };
    assert!(branch.else_body().unwrap().operations().is_empty());
    let operations = branch
        .then_body()
        .operations()
        .iter()
        .cloned()
        .map(|op| storage_operation_to_value(op, &|p| out.parameter_value(p)))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let body = Circuit::from_operations(out.qubits().to_vec(), operations, None, None).unwrap();
    body.to_matrix(None).unwrap()
}

#[test]
fn block_zxz_requires_resolved_matrix_parameters() {
    let theta = Parameter::symbol("theta");
    let matrix = Array2::from_shape_fn((8, 8), |(i, j)| match (i, j) {
        (7, 7) => SymbolicComplex::exp_i(theta.clone()),
        _ if i == j => SymbolicComplex::one(),
        _ => SymbolicComplex::zero(),
    });
    let gate = UnitaryGate::new("conditional_phase", 3, 1)
        .with_symbolic_matrix(["theta"], matrix)
        .unwrap();
    let mut circuit = Circuit::new(3);
    circuit
        .unitary_with_params(
            gate,
            vec![Qubit::new(1), Qubit::new(0), Qubit::new(2)],
            vec![Parameter::symbol("alpha").into()],
        )
        .unwrap();
    let err = decompose_unitaries(&circuit, Default::default()).unwrap_err();
    assert!(err.to_string().contains("unresolved symbols: alpha"));
}

#[test]
fn idle_high_bits_preserve_weak_xx_interactions() {
    for dim in [8, 16] {
        for angle in [1e-4_f64, 1e-6, 1e-9] {
            let phase = C::from_polar(1., 0.31);
            let u = Matrix::from_shape_fn((dim, dim), |(i, j)| match i ^ j {
                0 => phase * angle.cos(),
                3 => phase * C::i() * angle.sin(),
                _ => C::ZERO,
            });
            let circuit = decompose_block_zxz(&u, 1e-10).unwrap();
            assert_matrix_close(&circuit.to_matrix(None).unwrap(), &u, 1e-12);
            assert!(
                circuit
                    .operations()
                    .iter()
                    .flat_map(|op| &op.qubits)
                    .all(|q| q.index() < 2)
            );
        }
    }
}

#[test]
fn structured_fourier_and_adjoint_on_nonstandard_array_layouts() {
    for dim in [8, 16] {
        let u = Matrix::from_shape_fn((dim, dim), |(i, j)| {
            let angle = (std::f64::consts::TAU * (i * j) as f64 / dim as f64)
                + 0.19 * (i * i) as f64
                - 0.31 * j as f64;
            C::from_polar(1. / (dim as f64).sqrt(), angle)
        });
        for m in [
            u.clone().reversed_axes(),
            adjoint(&u),
            u.slice_move(s![..;-1,..;-1]),
        ] {
            assert_matrix_close(
                &decompose_block_zxz(&m, 1e-10)
                    .unwrap()
                    .to_matrix(None)
                    .unwrap(),
                &m,
                1e-10,
            );
        }
    }
}

#[test]
fn small_nonzero_off_diagonal_blocks_survive_the_a2_policy_threshold() {
    let angle = 4e-9_f64;
    let u = Matrix::from_shape_fn((8, 8), |(i, j)| match i ^ j {
        0 => C::from(angle.cos()),
        4 => C::i() * angle.sin(),
        _ => C::ZERO,
    });
    // The policy classifies this as block-diagonal at 1e-8. The structural
    // shortcut must still retain the MSB rotation, visible at 1e-11.
    assert_matrix_close(
        &decompose_block_zxz(&u, 1e-8)
            .unwrap()
            .to_matrix(None)
            .unwrap(),
        &u,
        1e-11,
    );
}

#[test]
fn force_alone_does_not_enable_diagonal_transfer() {
    let options = BlockZxzDecomposeOptions {
        opt_a2: false,
        ..Default::default()
    };
    let mut u = Matrix::eye(8);
    u.slice_mut(s![4.., 4..]).assign(&seeded_unitary(4, 5));
    let normal = decompose_block_zxz_with_options(&u, 1e-10, &options).unwrap();
    let forced = decompose_block_zxz_with_options(
        &u,
        1e-10,
        &BlockZxzDecomposeOptions {
            opt_a2_force: true,
            ..options
        },
    )
    .unwrap();
    assert_eq!(normal.operations(), forced.operations());
    assert_matrix_close(&forced.to_matrix(None).unwrap(), &u, 1e-10);
}

#[test]
fn optimized_recursion_reconstructs_multiple_seeded_unitaries() {
    for dim in [8, 16, 32] {
        for seed in [17, 31, 65] {
            let u = seeded_unitary(dim, seed) * C::from_polar(1., -(seed as f64));
            let c = decompose_block_zxz(&u, 1e-10).unwrap();
            assert!(c.operations().iter().all(|op| matches!(
                op.instruction.standard_gate(),
                Some(StandardGate::U | StandardGate::RZ | StandardGate::H | StandardGate::CX)
            )));
            assert_matrix_close(&c.to_matrix(None).unwrap(), &u, 1e-10);
        }
    }
}

#[test]
fn a_slightly_perturbed_controlled_x_can_use_euler_leaves() {
    // CX · F diag(exp(i ε [0,1,3,7])) F†. The nearly coincident
    // eigenphases can leave the KAK planner without a validated CX template.
    let f = ndarray::array![
        [C::ONE, C::ONE, C::ONE, C::ONE],
        [C::ONE, -C::i(), -C::ONE, C::i()],
        [C::ONE, -C::ONE, C::ONE, -C::ONE],
        [C::ONE, C::i(), -C::ONE, -C::i()]
    ] * 0.5;
    for gap in [1e-7, 1e-8, 1e-9] {
        let phases = ndarray::array![0., 1., 3., 7.].mapv(|k| C::from_polar(1., k * gap));
        let u = (&f * &phases)
            .dot(&adjoint(&f))
            .select(ndarray::Axis(0), &[0, 1, 3, 2]);
        assert_matrix_close(
            &decompose_block_zxz(&u, 1e-10)
                .unwrap()
                .to_matrix(None)
                .unwrap(),
            &u,
            1e-10,
        );
    }
}

#[test]
fn tight_leaf_tolerance_preserves_small_but_resolvable_interactions() {
    let angle = 1e-11_f64;
    let u = Matrix::from_shape_fn((4, 4), |(i, j)| match i ^ j {
        0 => C::from(angle.cos()),
        3 => C::i() * angle.sin(),
        _ => C::ZERO,
    });
    assert_matrix_close(
        &decompose_block_zxz(&u, 1e-13)
            .unwrap()
            .to_matrix(None)
            .unwrap(),
        &u,
        1e-13,
    );
}

#[test]
fn pass_keeps_a_sub_picoradian_phase_at_root_and_in_a_conditional() {
    let phase = 4e-13;
    let u = Matrix::eye(8) * C::from_polar(1., phase);
    let gate = UnitaryGate::new("small_phase", 3, 0)
        .with_matrix(u.clone())
        .unwrap();
    let q = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let mut source = Circuit::new(3);
    source.unitary(gate.clone(), q.to_vec()).unwrap();
    let root = decompose_unitaries(&source, Default::default()).unwrap();
    assert_matrix_close(&root.to_matrix(None).unwrap(), &u, 1e-14);
    let mut source = Circuit::new(3);
    source
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.append(
                    Instruction::UnitaryGate(Box::new(gate)),
                    q,
                    Vec::<ParameterValue>::new(),
                    None,
                )
            },
            |_| Ok(()),
        )
        .unwrap();
    let conditional = decompose_unitaries(&source, Default::default()).unwrap();
    assert!(conditional.global_phase().is_zero());
    assert_matrix_close(&then_matrix(&conditional), &u, 1e-14);
}

#[test]
fn weak_interaction_with_a_relative_phase_on_the_high_bit() {
    // diag(1,i) ⊗ I ⊗ exp(i ε XX), times a global phase. A dense basis
    // change of the scalar relative unitary can amplify error in later leaves.
    let angle = 1e-6_f64;
    let u = Matrix::from_shape_fn((16, 16), |(i, j)| {
        let phase = C::from_polar(1., 0.31) * if i >= 8 { C::i() } else { C::ONE };
        match i ^ j {
            0 => phase * angle.cos(),
            3 => phase * C::i() * angle.sin(),
            _ => C::ZERO,
        }
    });
    assert_matrix_close(
        &decompose_block_zxz(&u, 1e-10)
            .unwrap()
            .to_matrix(None)
            .unwrap(),
        &u,
        1e-12,
    );
}

#[test]
fn dense_and_block_diagonal_adversarial_demux_spectra() {
    let half_pi = std::f64::consts::FRAC_PI_2;
    let quarter_pi = std::f64::consts::FRAC_PI_4;
    let v4 = seeded_unitary(4, 101);
    let u1_4 = seeded_unitary(4, 102);
    let l1 = seeded_unitary(4, 103);
    let l2 = seeded_unitary(4, 104);

    for gap in [1.5e-8, 2e-8, 5e-8, 1e-7, 1e-6] {
        let phases = ndarray::array![half_pi, -half_pi + gap, quarter_pi, -3.0 * quarter_pi + gap];
        // Block-diagonal 3q matrix whose relative unitary has pairs near both 0 and pi/4.
        let m = (&v4 * &phases.mapv(|p| C::from_polar(1., p))).dot(&adjoint(&v4));
        let mut bd = Matrix::zeros((8, 8));
        bd.slice_mut(s![..4, ..4]).assign(&m.dot(&u1_4));
        bd.slice_mut(s![4.., 4..]).assign(&u1_4);
        assert_matrix_close(
            &decompose_block_zxz(&bd, 1e-10)
                .unwrap()
                .to_matrix(None)
                .unwrap(),
            &bd,
            1e-10,
        );

        // Fully dense non-block-diagonal 3q matrix whose split() yields C = M†.
        let c_mat = (&v4 * &phases.mapv(|p| C::from_polar(1., -p))).dot(&adjoint(&v4));
        let uy = &c_mat * C::i();
        let (sin_t, cos_t) = 0.63_f64.sin_cos();
        let mut dense = Matrix::zeros((8, 8));
        dense
            .slice_mut(s![..4, ..4])
            .assign(&(&l1 * C::from(cos_t)));
        dense
            .slice_mut(s![..4, 4..])
            .assign(&(l1.dot(&uy) * C::from(sin_t)));
        dense
            .slice_mut(s![4.., ..4])
            .assign(&(&l2 * C::from(-sin_t)));
        dense
            .slice_mut(s![4.., 4..])
            .assign(&(l2.dot(&uy) * C::from(cos_t)));
        assert_matrix_close(
            &decompose_block_zxz(&dense, 1e-10)
                .unwrap()
                .to_matrix(None)
                .unwrap(),
            &dense,
            1e-10,
        );
        // Also verify through the circuit-level pass.
        let gate = UnitaryGate::new("adv_dense", 3, 0)
            .with_matrix(dense.clone())
            .unwrap();
        let mut circuit = Circuit::new(3);
        circuit
            .unitary(gate, vec![Qubit::new(2), Qubit::new(1), Qubit::new(0)])
            .unwrap();
        let out = decompose_unitaries(&circuit, Default::default()).unwrap();
        assert_matrix_close(&out.to_matrix(None).unwrap(), &dense, 1e-10);
    }

    // 4q block-diagonal matrix with 3-way chains near both 0 and pi/4.
    let v8 = seeded_unitary(8, 105);
    let u1_8 = seeded_unitary(8, 106);
    for step in [1e-8, 1.4e-8, 2e-8] {
        let phases = ndarray::array![
            half_pi - step,
            -half_pi,
            half_pi + step,
            0.1,
            quarter_pi - step,
            -3.0 * quarter_pi,
            quarter_pi + step,
            -0.3
        ];
        let m8 = (&v8 * &phases.mapv(|p| C::from_polar(1., p))).dot(&adjoint(&v8));
        let mut u16 = Matrix::zeros((16, 16));
        u16.slice_mut(s![..8, ..8]).assign(&m8.dot(&u1_8));
        u16.slice_mut(s![8.., 8..]).assign(&u1_8);
        assert_matrix_close(
            &decompose_block_zxz(&u16, 1e-10)
                .unwrap()
                .to_matrix(None)
                .unwrap(),
            &u16,
            1e-10,
        );
    }
}

#[test]
fn loose_tolerance_projects_slightly_non_unitary_inputs() {
    for (dim, seed) in [(2, 111), (4, 112), (8, 113), (16, 114)] {
        let exact = seeded_unitary(dim, seed) * C::from_polar(1., 0.42);
        // Unitarity residual is ~4e-7, well below atol = 1e-5, but above 1q/2q UNITARY_EPS (1e-8).
        let noisy = &exact * C::from(1.0 + 2e-7);
        let circuit = decompose_block_zxz(&noisy, 1e-5).unwrap();
        assert_matrix_close(&circuit.to_matrix(None).unwrap(), &noisy, 1e-5);
        assert_matrix_close(&circuit.to_matrix(None).unwrap(), &exact, 1e-10);
    }
}

#[test]
fn zero_and_negative_zero_tolerance_on_exact_unitaries() {
    for dim in [2, 4, 8] {
        let eye = Matrix::eye(dim);
        for atol in [0.0, -0.0] {
            let circuit = decompose_block_zxz(&eye, atol).unwrap();
            assert_matrix_close(&circuit.to_matrix(None).unwrap(), &eye, 0.0);
        }
        let perturbed = &eye * C::from(1.0 + 1e-12);
        assert!(matches!(
            decompose_block_zxz(&perturbed, 0.0),
            Err(CompilerError::InvalidInput(_))
        ));
    }
}

#[test]
fn near_singular_quadrants_reconstruct_for_all_options() {
    let l1 = seeded_unitary(4, 121);
    let l2 = seeded_unitary(4, 122);
    let r1 = seeded_unitary(4, 123);
    let r2 = seeded_unitary(4, 124);
    let half_pi = std::f64::consts::FRAC_PI_2;
    let quarter_pi = std::f64::consts::FRAC_PI_4;

    for thetas in [
        [half_pi; 4],
        [0.0, 0.0, half_pi, half_pi],
        [0.3, half_pi, half_pi, half_pi],
        [half_pi - 1e-16, half_pi, 0.4, 0.9],
        [half_pi - 1e-12, half_pi - 2e-12, 0.0, 1e-12],
        [half_pi - 1e-8, half_pi - 1e-8, 1e-8, 1e-8],
        [
            quarter_pi,
            quarter_pi + 1e-12,
            quarter_pi - 1e-12,
            quarter_pi + 1e-8,
        ],
    ] {
        let cos_v = Array1::from_vec(thetas.iter().map(|t| C::from(t.cos())).collect());
        let sin_v = Array1::from_vec(thetas.iter().map(|t| C::from(t.sin())).collect());
        let mut u = Matrix::zeros((8, 8));
        u.slice_mut(s![..4, ..4])
            .assign(&(&l1 * &cos_v).dot(&adjoint(&r1)));
        u.slice_mut(s![..4, 4..])
            .assign(&(-(&l1 * &sin_v).dot(&adjoint(&r2))));
        u.slice_mut(s![4.., ..4])
            .assign(&(&l2 * &sin_v).dot(&adjoint(&r1)));
        u.slice_mut(s![4.., 4..])
            .assign(&(&l2 * &cos_v).dot(&adjoint(&r2)));
        for options in [
            BlockZxzDecomposeOptions::default(),
            BlockZxzDecomposeOptions {
                opt_a1: false,
                opt_a2: false,
                opt_a2_force: false,
            },
            BlockZxzDecomposeOptions {
                opt_a1: true,
                opt_a2: true,
                opt_a2_force: true,
            },
        ] {
            let c = decompose_block_zxz_with_options(&u, 1e-10, &options).unwrap();
            assert_matrix_close(&c.to_matrix(None).unwrap(), &u, 1e-10);
        }
    }
}

#[test]
fn structured_quantum_gates_preserve_matrix_and_global_phase() {
    let phase = C::from_polar(1.0, -0.57);
    let qft = |dim: usize| {
        let scale = 1.0 / (dim as f64).sqrt();
        Matrix::from_shape_fn((dim, dim), |(i, j)| {
            C::from_polar(scale, std::f64::consts::TAU * (i * j) as f64 / dim as f64)
        })
    };
    let mut ccx = Matrix::eye(8);
    ccx[[6, 6]] = C::ZERO;
    ccx[[7, 7]] = C::ZERO;
    ccx[[6, 7]] = C::ONE;
    ccx[[7, 6]] = C::ONE;

    let mut cswap = Matrix::eye(8);
    cswap[[5, 5]] = C::ZERO;
    cswap[[6, 6]] = C::ZERO;
    cswap[[5, 6]] = C::ONE;
    cswap[[6, 5]] = C::ONE;

    let mut cccz = Matrix::eye(16);
    cccz[[15, 15]] = -C::ONE;

    let grover8 = Matrix::from_elem((8, 8), C::from(0.25)) - Matrix::eye(8);
    let mut anti_bd = Matrix::zeros((8, 8));
    anti_bd
        .slice_mut(s![..4, 4..])
        .assign(&seeded_unitary(4, 131));
    anti_bd
        .slice_mut(s![4.., ..4])
        .assign(&seeded_unitary(4, 132));

    for u in [
        ccx * phase,
        cswap * phase,
        cccz * phase,
        grover8 * phase,
        qft(8) * phase,
        qft(16) * phase,
        anti_bd * phase,
        -Matrix::eye(8),
    ] {
        let c = decompose_block_zxz(&u, 1e-10).unwrap();
        assert_matrix_close(&c.to_matrix(None).unwrap(), &u, 1e-10);
    }
}

#[test]
fn decompose_unitaries_resolved_symbolic_3q_matrix_and_parameter_cache() {
    let theta = Parameter::symbol("theta");
    let matrix = Array2::from_shape_fn((8, 8), |(i, j)| match (i, j) {
        (7, 7) => SymbolicComplex::exp_i(theta.clone()),
        _ if i == j => SymbolicComplex::one(),
        _ => SymbolicComplex::zero(),
    });
    let gate = UnitaryGate::new("ccphase", 3, 1)
        .with_symbolic_matrix(["theta"], matrix)
        .unwrap();

    // Resolved finite parameters synthesize accurately on permuted qubits,
    // reuse the rule cache for identical values, and miss for distinct values.
    let mut circuit = Circuit::new(3);
    circuit
        .unitary_with_params(
            gate.clone(),
            vec![Qubit::new(1), Qubit::new(0), Qubit::new(2)],
            vec![ParameterValue::Fixed(0.43)],
        )
        .unwrap();
    circuit
        .unitary_with_params(
            gate.clone(),
            vec![Qubit::new(2), Qubit::new(1), Qubit::new(0)],
            vec![ParameterValue::Fixed(0.43)],
        )
        .unwrap();
    circuit
        .unitary_with_params(
            gate.clone(),
            vec![Qubit::new(0), Qubit::new(2), Qubit::new(1)],
            vec![ParameterValue::Fixed(-0.71)],
        )
        .unwrap();
    let expected = circuit.to_matrix(None).unwrap();
    let (out, stats) = decompose_unitaries_with_rule_stats(&circuit, Default::default()).unwrap();
    assert_eq!((stats.hits, stats.misses, stats.inserts), (1, 2, 2));
    assert_matrix_close(
        &out.into_circuit(&circuit).to_matrix(None).unwrap(),
        &expected,
        1e-10,
    );

    // Non-finite fixed parameters are rejected at circuit construction.
    let mut bad = Circuit::new(3);
    assert!(
        bad.unitary_with_params(
            gate,
            vec![Qubit::new(0), Qubit::new(1), Qubit::new(2)],
            vec![ParameterValue::Fixed(f64::NAN)],
        )
        .is_err()
    );
}

#[test]
fn decompose_unitaries_overlapping_permuted_cache_and_mixed_arities() {
    let g1 = UnitaryGate::new("u1", 1, 0)
        .with_matrix(seeded_unitary(2, 141) * C::from_polar(1., 0.13))
        .unwrap();
    let g2 = UnitaryGate::new("u2", 2, 0)
        .with_matrix(seeded_unitary(4, 142) * C::from_polar(1., -0.27))
        .unwrap();
    let g3 = UnitaryGate::new("u3", 3, 0)
        .with_matrix(seeded_unitary(8, 143) * C::from_polar(1., 0.61))
        .unwrap();
    let g4 = UnitaryGate::new("u4", 4, 0)
        .with_matrix(seeded_unitary(16, 144) * C::from_polar(1., -0.89))
        .unwrap();

    let mut circuit = Circuit::new(5);
    circuit.unitary(g1.clone(), vec![Qubit::new(0)]).unwrap();
    circuit.unitary(g1, vec![Qubit::new(4)]).unwrap();
    circuit
        .unitary(g2.clone(), vec![Qubit::new(1), Qubit::new(2)])
        .unwrap();
    circuit
        .unitary(g2, vec![Qubit::new(4), Qubit::new(0)])
        .unwrap();
    for args in [[0, 1, 2], [1, 2, 0], [2, 0, 1]] {
        circuit
            .unitary(g3.clone(), args.map(Qubit::new).to_vec())
            .unwrap();
    }
    circuit
        .unitary(
            g4.clone(),
            vec![Qubit::new(3), Qubit::new(0), Qubit::new(4), Qubit::new(1)],
        )
        .unwrap();
    circuit
        .unitary(
            g4,
            vec![Qubit::new(1), Qubit::new(4), Qubit::new(0), Qubit::new(3)],
        )
        .unwrap();

    let (out, stats) = decompose_unitaries_with_rule_stats(&circuit, Default::default()).unwrap();
    assert_eq!((stats.hits, stats.misses, stats.inserts), (5, 4, 4));
    assert_matrix_close(
        &out.into_circuit(&circuit).to_matrix(None).unwrap(),
        &circuit.to_matrix(None).unwrap(),
        1e-9,
    );
}

#[test]
fn decompose_unitaries_control_flow_both_branches_and_recurse_false() {
    use crate::compile::transform::decompose::UnitaryDecomposeConfig;

    let u_then = seeded_unitary(8, 151) * C::from_polar(1., 0.73);
    let u_else = seeded_unitary(8, 152) * C::from_polar(1., -1.19);
    let g_then = UnitaryGate::new("then3", 3, 0)
        .with_matrix(u_then.clone())
        .unwrap();
    let g_else = UnitaryGate::new("else3", 3, 0)
        .with_matrix(u_else.clone())
        .unwrap();

    let mut circuit = Circuit::new(3);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.append(
                    Instruction::UnitaryGate(Box::new(g_then)),
                    [Qubit::new(2), Qubit::new(1), Qubit::new(0)],
                    Vec::<ParameterValue>::new(),
                    None,
                )
            },
            |body| {
                body.append(
                    Instruction::UnitaryGate(Box::new(g_else)),
                    [Qubit::new(2), Qubit::new(1), Qubit::new(0)],
                    Vec::<ParameterValue>::new(),
                    None,
                )
            },
        )
        .unwrap();

    // recurse_control_flow = false keeps the branch bodies untouched.
    let no_recurse = decompose_unitaries(
        &circuit,
        UnitaryDecomposeConfig {
            recurse_control_flow: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(no_recurse, circuit);

    // recurse_control_flow = true lowers both branches with independent local GPhase.
    let out = decompose_unitaries(&circuit, Default::default()).unwrap();
    assert!(out.global_phase().is_zero());
    let Instruction::ClassicalControl(ClassicalControlOp::If(branch)) =
        &out.operations()[0].instruction
    else {
        panic!("expected conditional")
    };
    let body_matrix = |ops: &[crate::circuit::Operation]| {
        let vals = ops
            .iter()
            .cloned()
            .map(|op| storage_operation_to_value(op, &|p| out.parameter_value(p)))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        Circuit::from_operations(out.qubits().to_vec(), vals, None, None)
            .unwrap()
            .to_matrix(None)
            .unwrap()
    };
    assert_matrix_close(
        &body_matrix(branch.then_body().operations()),
        &u_then,
        1e-10,
    );
    assert_matrix_close(
        &body_matrix(branch.else_body().unwrap().operations()),
        &u_else,
        1e-10,
    );
}

#[test]
fn smb_criterion_avoids_inflating_structured_leaf_cx_count() {
    let count_cx = |c: &Circuit| {
        c.operations()
            .iter()
            .filter(|op| op.instruction.standard_gate() == Some(StandardGate::CX))
            .count()
    };

    // H ⊗ H ⊗ H has only simple leaves: A.2 must not add CX gates over opt_a2 = false.
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let h3 = Matrix::from_shape_fn((8, 8), |(i, j)| {
        let sign = if (i & j).count_ones() % 2 == 0 {
            1.0
        } else {
            -1.0
        };
        C::from(sign * s * s * s)
    });
    let h3_default = decompose_block_zxz(&h3, 1e-10).unwrap();
    let h3_no_a2 = decompose_block_zxz_with_options(
        &h3,
        1e-10,
        &BlockZxzDecomposeOptions {
            opt_a1: true,
            opt_a2: false,
            opt_a2_force: false,
        },
    )
    .unwrap();
    assert_matrix_close(&h3_default.to_matrix(None).unwrap(), &h3, 1e-10);
    assert!(
        count_cx(&h3_default) <= count_cx(&h3_no_a2),
        "default CX ({}) exceeded opt_a2=false CX ({})",
        count_cx(&h3_default),
        count_cx(&h3_no_a2)
    );

    // Diagonal 3q/4q matrices with opt_a2_force=true stay at the same CX count as opt_a2=false.
    for dim in [8, 16] {
        let diag = Matrix::from_diag(&Array1::from_shape_fn(dim, |i| {
            C::from_polar(1.0, 0.27 * (i * i + 1) as f64)
        }));
        let forced = decompose_block_zxz_with_options(
            &diag,
            1e-10,
            &BlockZxzDecomposeOptions {
                opt_a1: true,
                opt_a2: true,
                opt_a2_force: true,
            },
        )
        .unwrap();
        let plain = decompose_block_zxz_with_options(
            &diag,
            1e-10,
            &BlockZxzDecomposeOptions {
                opt_a1: true,
                opt_a2: false,
                opt_a2_force: false,
            },
        )
        .unwrap();
        assert_matrix_close(&forced.to_matrix(None).unwrap(), &diag, 1e-10);
        assert_eq!(count_cx(&forced), count_cx(&plain));
    }
}

#[test]
fn decompose_unitaries_respects_block_zxz_options() {
    use crate::compile::transform::decompose::UnitaryDecomposeConfig;

    let count_cx = |c: &Circuit| {
        c.operations()
            .iter()
            .filter(|op| op.instruction.standard_gate() == Some(StandardGate::CX))
            .count()
    };

    let u = seeded_unitary(8, 7) * C::from_polar(1., 0.29);
    let gate = UnitaryGate::new("generic3", 3, 0).with_matrix(u).unwrap();
    let mut circuit = Circuit::new(3);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1), Qubit::new(2)])
        .unwrap();
    let expected = circuit.to_matrix(None).unwrap();

    let optimized = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap();
    let unoptimized = decompose_unitaries(
        &circuit,
        UnitaryDecomposeConfig {
            block_zxz_options: BlockZxzDecomposeOptions {
                opt_a1: false,
                opt_a2: false,
                opt_a2_force: false,
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(count_cx(&optimized), 19);
    assert_eq!(count_cx(&unoptimized), 24);
    assert_matrix_close(&optimized.to_matrix(None).unwrap(), &expected, 1e-10);
    assert_matrix_close(&unoptimized.to_matrix(None).unwrap(), &expected, 1e-10);
}

#[test]
fn repeated_weak_xx_interaction_preserves_coherent_rotation() {
    let weak_xx = |theta: f64| {
        Matrix::from_shape_fn((8, 8), |(i, j)| match i ^ j {
            0 => C::from(theta.cos()),
            3 => C::i() * theta.sin(),
            _ => C::ZERO,
        })
    };
    for (angle, repeats) in [(9e-11_f64, 64usize), (8e-13_f64, 512usize)] {
        let u = weak_xx(angle);
        let gate = UnitaryGate::new("weak_xx_3q", 3, 0).with_matrix(u).unwrap();
        let mut circuit = Circuit::new(3);
        for _ in 0..repeats {
            circuit
                .unitary(
                    gate.clone(),
                    vec![Qubit::new(2), Qubit::new(1), Qubit::new(0)],
                )
                .unwrap();
        }
        let expected = weak_xx(angle * repeats as f64);
        let (out, stats) =
            decompose_unitaries_with_rule_stats(&circuit, Default::default()).unwrap();
        assert_eq!((stats.hits, stats.misses), (repeats - 1, 1));
        assert_matrix_close(
            &out.into_circuit(&circuit).to_matrix(None).unwrap(),
            &expected,
            1e-10,
        );
    }
}

#[test]
fn near_tolerance_unitary_boundary_succeeds_in_block_zxz_and_pass() {
    let base = seeded_unitary(8, 173);
    let scaled = &base * C::from(1.0 + 4.5e-11);
    let mut rng = StdRng::seed_from_u64(174);
    let noise = Matrix::from_shape_simple_fn((8, 8), || {
        C::new(rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0))
    });
    let probe = &base + &noise * C::from(1e-11);
    let probe_err = residual(&adjoint(&probe).dot(&probe), &Matrix::eye(8));
    let anisotropic = &base + &noise * C::from(1e-11 * (9e-11 / probe_err));

    for matrix in [scaled, anisotropic] {
        let unitarity_err = residual(&adjoint(&matrix).dot(&matrix), &Matrix::eye(8));
        assert!(unitarity_err > 8e-11 && unitarity_err < 1e-10);

        let gate = UnitaryGate::new("near_tol_3q", 3, 0)
            .with_matrix(matrix.clone())
            .unwrap();
        let direct = decompose_block_zxz(&matrix, 1e-10).unwrap();
        assert_matrix_close(&direct.to_matrix(None).unwrap(), &matrix, 1e-10);

        let mut circuit = Circuit::new(3);
        circuit
            .unitary(gate, vec![Qubit::new(2), Qubit::new(1), Qubit::new(0)])
            .unwrap();
        let out = decompose_unitaries(&circuit, Default::default()).unwrap();
        assert_matrix_close(&out.to_matrix(None).unwrap(), &matrix, 1e-10);
    }
}

#[test]
fn near_unitary_polar_projection_and_coherent_64x64_perturbation_succeed() {
    // 1) 8x8 matrix passing input unitarity (<= 1e-10) whose Frobenius-optimal
    // polar projection shifts a single entry by > 1e-10 in max-element norm.
    let qft = Matrix::from_shape_fn((8, 8), |(i, j)| {
        C::from_polar(
            1.0 / 8.0_f64.sqrt(),
            std::f64::consts::TAU * (i * j) as f64 / 8.0,
        )
    });
    let mut noisy = qft.clone();
    for j in 0..8 {
        noisy[[0, j]] *= C::from(1.0 + 3.6e-10);
    }
    let unitarity_err = residual(&adjoint(&noisy).dot(&noisy), &Matrix::eye(8));
    let projected = factors::nearest_unitary(&noisy).unwrap();
    let proj_err = residual(&projected, &noisy);
    assert!(
        unitarity_err <= 1e-10 && proj_err > 1e-10,
        "unitarity_err={unitarity_err:e}, proj_err={proj_err:e}"
    );
    let direct = decompose_block_zxz(&noisy, 1e-10).unwrap();
    assert_matrix_close(&direct.to_matrix(None).unwrap(), &projected, 1e-10);

    let gate = UnitaryGate::new("polar_shift_3q", 3, 0)
        .with_matrix(noisy.clone())
        .unwrap();
    let mut circuit = Circuit::new(3);
    circuit
        .unitary(gate, vec![Qubit::new(2), Qubit::new(1), Qubit::new(0)])
        .unwrap();
    let out = decompose_unitaries(&circuit, Default::default()).unwrap();
    assert_matrix_close(&out.to_matrix(None).unwrap(), &projected, 1e-10);

    // 2) 64x64 coherent rank-1 perturbation whose max-element Gram residual is
    // ~8.55e-13 (below 64*n*eps) yet whose spectral error is 64x larger.
    let n = 64usize;
    let gram_noise = 0.94 * 64.0 * (n as f64) * f64::EPSILON;
    let p = gram_noise / ((1.0 + (n as f64) * gram_noise).sqrt() + 1.0);
    let coherent = Matrix::eye(n) + Matrix::from_elem((n, n), C::from(p));
    let coherent_gram = residual(&adjoint(&coherent).dot(&coherent), &Matrix::eye(n));
    assert!(coherent_gram < 1e-10);
    let coherent_proj = factors::nearest_unitary(&coherent).unwrap();
    let coherent_out = decompose_block_zxz(&coherent, 1e-10).unwrap();
    assert_matrix_close(
        &coherent_out.to_matrix(None).unwrap(),
        &coherent_proj,
        1e-10,
    );
}
