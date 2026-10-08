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

//! Regression tests for numerical boundaries and labelled noisy simulation.
use cqlib_core::circuit::{Circuit, Qubit, StandardGate, circuit_to_matrix};
use cqlib_core::device::{
    NoiseModel,
    noise::{SingleQubitNoise, TwoQubitNoise},
};
use cqlib_core::qis::entropy::{entanglement_of_formation, renyi_entropy};
use cqlib_core::qis::evolution::TrotterMode;
use cqlib_core::qis::{DensityMatrix, DensityMatrixNoise, Hamiltonian};
use num_complex::Complex64;

#[test]
fn pauli_probability_boundary_is_trace_preserving() {
    let noise = SingleQubitNoise::Pauli {
        px: 0.3,
        py: 0.6,
        pz: 0.1,
    };
    assert!(noise.is_valid());
    let ops = noise.to_kraus();
    let mut completeness = ndarray::Array2::<Complex64>::zeros((2, 2));
    for op in &ops {
        assert!(op.iter().all(|v| v.re.is_finite() && v.im.is_finite()));
        completeness += &op.t().mapv(|v| v.conj()).dot(op);
    }
    for i in 0..2 {
        for j in 0..2 {
            assert!(
                (completeness[[i, j]] - Complex64::from(if i == j { 1.0 } else { 0.0 })).norm()
                    < 1e-12
            );
        }
    }
    let mut model = NoiseModel::new();
    model
        .add_single_qubit_error(StandardGate::X, Qubit::new(0), noise)
        .unwrap();
    let mut sim = DensityMatrixNoise::new(1, Some(model));
    sim.apply_x(0).unwrap();
    assert!((sim.probabilities()[0] - 0.9).abs() < 1e-12);
    assert!((sim.probabilities().iter().sum::<f64>() - 1.0).abs() < 1e-12);
    assert!(
        !SingleQubitNoise::Pauli {
            px: 0.4,
            py: 0.6,
            pz: 0.1
        }
        .is_valid()
    );
}

#[test]
fn gate_noise_uses_original_labels_and_operand_order() {
    for ids in [[0, 1], [5, 12], [12, 5]] {
        let qs = ids.map(Qubit::new);
        let mut circuit = Circuit::from_qubits(qs.to_vec()).unwrap();
        circuit.x(qs[0]).unwrap();
        let mut model = NoiseModel::new();
        model
            .add_single_qubit_error(StandardGate::X, qs[0], SingleQubitNoise::BitFlip(1.0))
            .unwrap();
        let mut sim = DensityMatrixNoise::from_circuit(&circuit, Some(model)).unwrap();
        assert!((sim.probabilities()[0] - 1.0).abs() < 1e-12);
        sim.apply_x(0).unwrap();
        assert!((sim.probabilities()[0] - 1.0).abs() < 1e-12);
        for reverse in [false, true] {
            let mut circuit = Circuit::from_qubits(qs.to_vec()).unwrap();
            let [control, target] = if reverse { [qs[1], qs[0]] } else { qs };
            circuit.cx(control, target).unwrap();
            let mut model = NoiseModel::new();
            model
                .add_two_qubit_error(
                    StandardGate::CX,
                    control,
                    target,
                    TwoQubitNoise::Independent {
                        q0_noise: SingleQubitNoise::BitFlip(1.0),
                        q1_noise: SingleQubitNoise::BitFlip(0.0),
                    },
                )
                .unwrap();
            let sim = DensityMatrixNoise::from_circuit(&circuit, Some(model)).unwrap();
            let expected = if reverse { 2 } else { 1 };
            assert!((sim.probabilities()[expected] - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn formation_entropy_at_concurrence_roundoff_boundary() {
    let state = vec![
        Complex64::new(-0.5917562523120286, -0.14345273890626087),
        Complex64::new(0.12739228627789673, 0.3361800930257551),
        Complex64::new(-0.06600502193827436, 0.35339664207352467),
        Complex64::new(-0.5571108451004554, 0.2457267928442915),
    ];
    let dm = DensityMatrix::from_state(2, state).unwrap();
    assert!((entanglement_of_formation(&dm).unwrap() - 1.0).abs() < 1e-12);
}

#[test]
fn renyi_extreme_orders_and_small_positive_eigenvalues() {
    for n in 1..=3 {
        let dm = DensityMatrix::maximally_mixed(n);
        for alpha in [
            0.1,
            1.0 - 1e-12,
            1.0,
            1.0 + 1e-12,
            2000.0,
            f64::MAX,
            f64::INFINITY,
        ] {
            assert!(
                (renyi_entropy(&dm, alpha).unwrap() - n as f64).abs() < 1e-10,
                "n={n}, alpha={alpha}"
            );
            assert!(renyi_entropy(&DensityMatrix::new(n), alpha).unwrap().abs() < 1e-10);
        }
    }
    let dm = DensityMatrix::from_density_matrix_state(
        1,
        vec![(1.0 - 1e-14).into(), 0.0.into(), 0.0.into(), 1e-14.into()],
    )
    .unwrap();
    assert!(
        (renyi_entropy(&dm, 0.1).unwrap() - 0.06257881098503021).abs() < 1e-12,
        "got {}",
        renyi_entropy(&dm, 0.1).unwrap()
    );
    assert!(renyi_entropy(&dm, f64::NAN).is_err());
}

#[test]
fn evolution_retains_small_coefficients_before_time_scaling() {
    for coefficients in [vec![1e-11], vec![2e-11, -1e-11], vec![1e-11, -1e-11]] {
        let mut h = Hamiltonian::new(1);
        for &c in &coefficients {
            h.add_term("Z".parse().unwrap(), c.into()).unwrap();
        }
        let angle = coefficients.iter().sum::<f64>() * 1e11;
        for mode in [TrotterMode::FirstOrder, TrotterMode::SecondOrder] {
            for circuit in [
                h.to_trotter_circuit(1e11, 3, mode).unwrap(),
                h.to_evolution_circuit(1e11, 3, mode).unwrap(),
            ] {
                let m = circuit_to_matrix(&circuit, None).unwrap();
                assert!((m[[0, 0]] - Complex64::from_polar(1.0, -angle)).norm() < 1e-10);
                assert!((m[[1, 1]] - Complex64::from_polar(1.0, angle)).norm() < 1e-10);
                assert!(m[[0, 1]].norm() < 1e-10 && m[[1, 0]].norm() < 1e-10);
            }
        }
        h.simplify();
        assert!(h.terms.is_empty());
    }
}

#[test]
fn checked_constructors_reject_dimensions_without_allocating() {
    use cqlib_core::qis::Statevector;
    for n in [usize::BITS as usize - 4, usize::BITS as usize, usize::MAX] {
        assert!(Statevector::try_new(n).is_err());
        assert!(Statevector::from_state(n, vec![]).is_err());
    }
    for n in [
        (usize::BITS as usize - 4).div_ceil(2),
        usize::BITS as usize,
        usize::MAX,
    ] {
        assert!(DensityMatrix::try_new(n).is_err());
        assert!(DensityMatrixNoise::try_new(n, None).is_err());
        assert!(DensityMatrix::from_state(n, vec![]).is_err());
        assert!(DensityMatrix::from_density_matrix_state(n, vec![]).is_err());
        assert!(DensityMatrix::try_maximally_mixed(n).is_err());
    }
    for n in [0, 1, 3] {
        assert_eq!(
            Statevector::try_new(n).unwrap().data(),
            Statevector::new(n).data()
        );
        assert_eq!(
            DensityMatrix::try_new(n).unwrap().data(),
            DensityMatrix::new(n).data()
        );
        assert!(
            (DensityMatrixNoise::try_new(n, None)
                .unwrap()
                .probabilities()[0]
                - 1.0)
                .abs()
                < 1e-12
        );
    }
}

#[test]
fn failed_noisy_circuit_preserves_label_binding() {
    use cqlib_core::circuit::Parameter;
    let q = Qubit::new(5);
    let mut bad = Circuit::from_qubits(vec![q]).unwrap();
    bad.x(q).unwrap();
    bad.rz(q, Parameter::symbol("unbound")).unwrap();
    let mut model = NoiseModel::new();
    model
        .add_single_qubit_error(StandardGate::X, q, SingleQubitNoise::BitFlip(1.0))
        .unwrap();
    let mut sim = DensityMatrixNoise::new(1, Some(model.clone()));
    assert!(sim.apply_circuit(&bad).is_err());
    // Completed gates used label 5, but the failed application did not bind it.
    assert!((sim.probabilities()[0] - 1.0).abs() < 1e-12);
    sim.apply_circuit(&Circuit::new(1)).unwrap();

    let mut good = Circuit::from_qubits(vec![q]).unwrap();
    good.x(q).unwrap();
    let mut bound = DensityMatrixNoise::from_circuit(&good, Some(model)).unwrap();
    assert!(bound.apply_circuit(&bad).is_err());
    bound.apply_x(0).unwrap();
    assert!((bound.probabilities()[0] - 1.0).abs() < 1e-12);
    assert!(bound.apply_circuit(&Circuit::new(1)).is_err());
}
