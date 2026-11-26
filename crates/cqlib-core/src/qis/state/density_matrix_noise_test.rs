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
use crate::circuit::{Circuit, Instruction, ParameterValue, Qubit, StandardGate};
use crate::device::NoiseModel;
use crate::device::noise::{ReadoutError, SingleQubitNoise, TwoQubitNoise};
use crate::qis::pauli::{Pauli, PauliString};
use ndarray::array;
use num_complex::Complex64;
use std::f64::consts::PI;

#[test]
fn duplicate_targets_leave_noisy_density_matrix_unchanged() {
    let mut model = NoiseModel::new();
    model
        .add_two_qubit_error(
            StandardGate::CX,
            Qubit::new(0),
            Qubit::new(2),
            TwoQubitNoise::CorrelatedPauli {
                op_q0: Pauli::X,
                op_q1: Pauli::X,
                p: 0.2,
            },
        )
        .unwrap();
    let mut initial = DensityMatrixNoise::new(3, Some(model));
    initial.apply_h(0).unwrap();
    initial.apply_s(0).unwrap();
    for gate in [StandardGate::CX, StandardGate::CZ] {
        for q in 0..3 {
            let mut sim = initial.clone();
            assert!(matches!(
                sim.apply_standard_gate_noise(gate, &[q, q], &[]),
                Err(QisError::InvalidParameterValue(msg))
                    if msg.contains(&format!("Duplicate qubit index {q} "))
            ));
            assert_eq!(sim.state.data(), initial.state.data());
        }
    }
}

#[test]
fn test_bit_flip_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(0.1),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
    sim.apply_x(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!((probs[1] - 0.9).abs() < 1e-6, "P(|1>) was {}", probs[1]);
    assert!((probs[0] - 0.1).abs() < 1e-6, "P(|0>) was {}", probs[0]);
}

#[test]
fn test_readout_error() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_readout_error(
            Qubit::new(0),
            ReadoutError {
                p_0_given_1: 0.0,
                p_1_given_0: 0.1,
            },
        )
        .unwrap();

    let sim = DensityMatrixNoise::new(1, Some(noise_model));
    let probs = sim.probabilities_with_readout(&[0]).unwrap();

    assert!((probs[1] - 0.1).abs() < 1e-6, "P(|1>) was {}", probs[1]);
    assert!((probs[0] - 0.9).abs() < 1e-6, "P(|0>) was {}", probs[0]);
}

#[test]
fn test_ccx() {
    let mut sim = DensityMatrixNoise::new(3, None);
    sim.apply_x(0).unwrap();
    sim.apply_x(1).unwrap();
    sim.apply_ccx(0, 1, 2).unwrap();
    let probs = sim.probabilities_with_readout(&[2]).unwrap();
    let p1: f64 = probs
        .iter()
        .enumerate()
        .filter(|(s, _)| (s >> 2) & 1usize == 1usize)
        .map(|(_, p)| p)
        .sum();
    assert!((p1 - 1.0).abs() < 1e-6);
}

#[test]
fn test_apply_kraus_memory() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(0.01),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
    for _ in 0..100 {
        sim.apply_x(0).unwrap();
    }
    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!(probs[0] > 0.0);
    assert!(probs[1] > 0.0);
}

#[test]
fn test_from_circuit_with_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(0.1),
        )
        .unwrap();

    let mut circuit = Circuit::new(1);
    circuit.x(Qubit::new(0)).unwrap();

    let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
    let probs = sim.probabilities_with_readout(&[0]).unwrap();

    assert!((probs[1] - 0.9).abs() < 1e-6);
    assert!((probs[0] - 0.1).abs() < 1e-6);
}

#[test]
fn test_apply_circuit_with_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(0.1),
        )
        .unwrap();

    let mut circuit = Circuit::new(1);
    circuit.x(Qubit::new(0)).unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
    sim.apply_circuit(&circuit).unwrap();
    let probs = sim.probabilities_with_readout(&[0]).unwrap();

    assert!((probs[1] - 0.9).abs() < 1e-6);
    assert!((probs[0] - 0.1).abs() < 1e-6);
}

#[test]
fn test_phase_flip_noise_exact() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::H,
            Qubit::new(0),
            SingleQubitNoise::PhaseFlip(0.1),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
    sim.apply_h(0).unwrap();
    sim.apply_h(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!((probs[1] - 0.1).abs() < 1e-6, "P(|1>) was {}", probs[1]);
    assert!((probs[0] - 0.9).abs() < 1e-6, "P(|0>) was {}", probs[0]);
}

#[test]
fn test_amplitude_damping_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X,
            Qubit::new(0),
            SingleQubitNoise::AmplitudeDamping(0.2),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
    sim.apply_x(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!((probs[1] - 0.8).abs() < 1e-6, "P(|1>) was {}", probs[1]);
    assert!((probs[0] - 0.2).abs() < 1e-6, "P(|0>) was {}", probs[0]);
}

#[test]
fn test_phase_damping_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::H,
            Qubit::new(0),
            SingleQubitNoise::PhaseDamping(0.2),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
    sim.apply_h(0).unwrap();
    sim.apply_h(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    let expected_p0 = 0.5 * (1.0 + 0.8f64.sqrt());
    let expected_p1 = 0.5 * (1.0 - 0.8f64.sqrt());
    assert!(
        (probs[0] - expected_p0).abs() < 1e-6,
        "P(|0>) was {}",
        probs[0]
    );
    assert!(
        (probs[1] - expected_p1).abs() < 1e-6,
        "P(|1>) was {}",
        probs[1]
    );
}

#[test]
fn test_pauli_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X,
            Qubit::new(0),
            SingleQubitNoise::Pauli {
                px: 0.1,
                py: 0.2,
                pz: 0.3,
            },
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
    sim.apply_x(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!((probs[0] - 0.3).abs() < 1e-6, "P(|0>) was {}", probs[0]);
    assert!((probs[1] - 0.7).abs() < 1e-6, "P(|1>) was {}", probs[1]);
}

#[test]
fn test_depolarizing_1q_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X,
            Qubit::new(0),
            SingleQubitNoise::Depolarizing(0.3),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
    sim.apply_x(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!((probs[1] - 0.8).abs() < 1e-6, "P(|1>) was {}", probs[1]);
    assert!((probs[0] - 0.2).abs() < 1e-6, "P(|0>) was {}", probs[0]);
}

#[test]
fn test_two_qubit_correlated_pauli_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_two_qubit_error(
            StandardGate::CX,
            Qubit::new(0),
            Qubit::new(1),
            TwoQubitNoise::CorrelatedPauli {
                op_q0: Pauli::X,
                op_q1: Pauli::X,
                p: 0.2,
            },
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(2, Some(noise_model));
    sim.apply_cx(0, 1).unwrap();

    let probs = sim.probabilities_with_readout(&[0, 1]).unwrap();
    assert!((probs[0] - 0.8).abs() < 1e-6, "P(|00>) was {}", probs[0]);
    assert!((probs[3] - 0.2).abs() < 1e-6, "P(|11>) was {}", probs[3]);
    assert!(probs[1] < 1e-6);
    assert!(probs[2] < 1e-6);
}

#[test]
fn test_two_qubit_independent_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_two_qubit_error(
            StandardGate::CX,
            Qubit::new(0),
            Qubit::new(1),
            TwoQubitNoise::Independent {
                q0_noise: SingleQubitNoise::BitFlip(0.1),
                q1_noise: SingleQubitNoise::BitFlip(0.2),
            },
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(2, Some(noise_model));
    sim.apply_cx(0, 1).unwrap();

    let probs = sim.probabilities_with_readout(&[0, 1]).unwrap();
    assert!((probs[0] - 0.72).abs() < 1e-6, "P(|00>) was {}", probs[0]);
    assert!((probs[1] - 0.08).abs() < 1e-6, "P(|01>) was {}", probs[1]);
    assert!((probs[2] - 0.18).abs() < 1e-6, "P(|10>) was {}", probs[2]);
    assert!((probs[3] - 0.02).abs() < 1e-6, "P(|11>) was {}", probs[3]);
}

#[test]
fn test_two_qubit_depolarizing_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_two_qubit_error(
            StandardGate::CX,
            Qubit::new(0),
            Qubit::new(1),
            TwoQubitNoise::Depolarizing(0.15),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(2, Some(noise_model));
    sim.apply_x(0).unwrap();
    sim.apply_cx(0, 1).unwrap();

    let probs = sim.probabilities_with_readout(&[0, 1]).unwrap();
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);

    assert!(probs[3] < 1.0);
    assert!(probs[3] > 0.8);
    assert!(probs[0] > 0.0);
    assert!(probs[1] > 0.0);
    assert!(probs[2] > 0.0);
}

#[test]
fn test_multi_qubit_readout_error() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_readout_error(
            Qubit::new(0),
            ReadoutError {
                p_0_given_1: 0.1,
                p_1_given_0: 0.2,
            },
        )
        .unwrap();
    noise_model
        .add_readout_error(
            Qubit::new(1),
            ReadoutError {
                p_0_given_1: 0.3,
                p_1_given_0: 0.4,
            },
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(2, Some(noise_model));
    sim.apply_x(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0, 1]).unwrap();
    assert!((probs[0] - 0.06).abs() < 1e-6, "P(|00>) was {}", probs[0]);
    assert!((probs[1] - 0.54).abs() < 1e-6, "P(|01>) was {}", probs[1]);
    assert!((probs[2] - 0.04).abs() < 1e-6, "P(|10>) was {}", probs[2]);
    assert!((probs[3] - 0.36).abs() < 1e-6, "P(|11>) was {}", probs[3]);
}

#[test]
fn test_from_circuit_complex() {
    let mut circuit = Circuit::new(3);

    circuit.h(Qubit::new(0)).unwrap();
    circuit.rx(Qubit::new(1), PI / 2.0).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit
        .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();
    circuit.swap(Qubit::new(0), Qubit::new(2)).unwrap();

    let mut noise_model = NoiseModel::new();
    noise_model
        .add_two_qubit_error(
            StandardGate::CX,
            Qubit::new(0),
            Qubit::new(1),
            TwoQubitNoise::Independent {
                q0_noise: SingleQubitNoise::BitFlip(0.05),
                q1_noise: SingleQubitNoise::BitFlip(0.05),
            },
        )
        .unwrap();

    let sim_result = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model));
    assert!(sim_result.is_ok());
    let sim = sim_result.unwrap();
    let probs = sim.probabilities_with_readout(&[0, 1, 2]).unwrap();
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);
}

#[test]
fn test_from_circuit_unitary_gate() {
    use crate::circuit::gate::UnitaryGate;

    let mut circuit = Circuit::new(1);
    let u_mat = array![
        [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
        [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)]
    ];
    let u_gate = UnitaryGate::new("U", 1, 0).with_matrix(u_mat).unwrap();
    circuit
        .append(
            Instruction::UnitaryGate(Box::new(u_gate)),
            vec![Qubit::new(0)],
            vec![],
            None::<&str>,
        )
        .unwrap();

    let sim = DensityMatrixNoise::from_circuit(&circuit, None).unwrap();
    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!((probs[1] - 1.0).abs() < 1e-6);
    assert!((probs[0] - 0.0).abs() < 1e-6);
}

#[test]
fn test_x2p_y2m_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X2P,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(0.1),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
    sim.apply_x2p(0).unwrap();
    sim.apply_y2m(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!(probs[0] >= 0.0);
    assert!(probs[1] >= 0.0);
    assert!((probs[0] + probs[1] - 1.0).abs() < 1e-6);
}

#[test]
fn test_cz_xy2p_rz_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_two_qubit_error(
            StandardGate::CZ,
            Qubit::new(0),
            Qubit::new(1),
            TwoQubitNoise::Depolarizing(0.1),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(2, Some(noise_model));
    sim.apply_rz(0, PI / 2.0).unwrap();
    sim.apply_xy2p(1, PI).unwrap();
    sim.apply_cz(0, 1).unwrap();

    let probs = sim.probabilities_with_readout(&[0, 1]).unwrap();
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);
}

#[test]
fn test_complex_circuit_with_native_gates() {
    let mut circuit = Circuit::new(3);

    circuit.x2p(Qubit::new(0)).unwrap();
    circuit.y2m(Qubit::new(1)).unwrap();
    circuit.xy2p(Qubit::new(0), PI / 4.0).unwrap();
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.rz(Qubit::new(2), PI).unwrap();
    // xy is a single-qubit gate, not two-qubit
    circuit.xy(Qubit::new(1), PI / 2.0).unwrap();
    circuit.cz(Qubit::new(1), Qubit::new(2)).unwrap();

    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X2P,
            Qubit::new(0),
            SingleQubitNoise::PhaseDamping(0.05),
        )
        .unwrap();
    noise_model
        .add_two_qubit_error(
            StandardGate::CZ,
            Qubit::new(0),
            Qubit::new(1),
            TwoQubitNoise::Independent {
                q0_noise: SingleQubitNoise::BitFlip(0.01),
                q1_noise: SingleQubitNoise::BitFlip(0.01),
            },
        )
        .unwrap();

    let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
    let probs = sim.probabilities_with_readout(&[0, 1, 2]).unwrap();
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);
}

#[test]
fn test_x2p_gate_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X2P,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(0.1),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
    sim.apply_x2p(0).unwrap();

    // X2P + X2P = X (approximately, without noise)
    // With bit-flip noise after X2P, we expect some probability in |0>
    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!(probs[0] >= 0.0);
    assert!(probs[1] >= 0.0);
    assert!((probs[0] + probs[1] - 1.0).abs() < 1e-6);

    // Apply second X2P to complete X rotation
    sim.apply_x2p(0).unwrap();
    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    // After two X2P gates with bit-flip noise, P(|0>) should be around 0.1
    assert!(probs[0] > 0.0, "P(|0>) should be > 0 due to noise");
}

#[test]
fn test_x2m_gate_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X2M,
            Qubit::new(0),
            SingleQubitNoise::Depolarizing(0.1),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));

    // Start in |1>
    sim.apply_x(0).unwrap();

    // X2M rotates around X axis by -pi/2
    sim.apply_x2m(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!(probs[0] >= 0.0);
    assert!(probs[1] >= 0.0);
    assert!((probs[0] + probs[1] - 1.0).abs() < 1e-6);
}

#[test]
fn test_y2p_gate_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::Y2P,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(0.1),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
    sim.apply_y2p(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!(probs[0] >= 0.0);
    assert!(probs[1] >= 0.0);
    assert!((probs[0] + probs[1] - 1.0).abs() < 1e-6);
}

#[test]
fn test_y2m_gate_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::Y2M,
            Qubit::new(0),
            SingleQubitNoise::PhaseFlip(0.1),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));

    // Start in |+>
    sim.apply_h(0).unwrap();

    // Y2M rotates around Y axis by -pi/2
    sim.apply_y2m(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!(probs[0] >= 0.0);
    assert!(probs[1] >= 0.0);
    assert!((probs[0] + probs[1] - 1.0).abs() < 1e-6);
}

#[test]
fn test_xy2p_gate_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::XY2P,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(0.1),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));

    // Start in |1>
    sim.apply_x(0).unwrap();

    // XY2P is a sqrt(XY) gate, theta=pi creates superposition
    // |1> -> (i|0> + |1>)/sqrt(2), so P(|0>) = 0.5 without noise
    sim.apply_xy2p(0, PI).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    // With bit-flip noise, we expect P(|0>) around 0.5 (with some noise variation)
    assert!(
        probs[0] > 0.35 && probs[0] < 0.65,
        "P(|0>) was {} (expected ~0.5 with noise)",
        probs[0]
    );
}

#[test]
fn test_xy2m_gate_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::XY2M,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(0.1),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));

    // Start in |1>
    sim.apply_x(0).unwrap();

    // XY2M is sqrt(XY)^dagger, theta=pi creates superposition
    // |1> -> (-i|0> + |1>)/sqrt(2), so P(|0>) = 0.5 without noise
    sim.apply_xy2m(0, PI).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    // With bit-flip noise, we expect P(|0>) around 0.5 (with some noise variation)
    assert!(
        probs[0] > 0.35 && probs[0] < 0.65,
        "P(|0>) was {} (expected ~0.5 with noise)",
        probs[0]
    );
}

#[test]
fn test_cz_gate_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_two_qubit_error(
            StandardGate::CZ,
            Qubit::new(0),
            Qubit::new(1),
            TwoQubitNoise::Independent {
                q0_noise: SingleQubitNoise::BitFlip(0.05),
                q1_noise: SingleQubitNoise::BitFlip(0.05),
            },
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(2, Some(noise_model));

    // Prepare |11>
    sim.apply_x(0).unwrap();
    sim.apply_x(1).unwrap();

    // Apply CZ
    sim.apply_cz(0, 1).unwrap();

    // CZ adds a phase to |11> but doesn't change probabilities
    let probs = sim.probabilities_with_readout(&[0, 1]).unwrap();

    // With bit-flip noise, we expect slight deviation from P(|11>) = 1
    assert!(probs[3] > 0.8, "P(|11>) was {} (expected > 0.8)", probs[3]);
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);
}

#[test]
fn test_rz_gate_noise() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::RZ,
            Qubit::new(0),
            SingleQubitNoise::PhaseFlip(0.2),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));

    // Prepare |+>
    sim.apply_h(0).unwrap();

    // Apply RZ(pi)
    sim.apply_rz(0, PI).unwrap();

    // RZ(pi) on |+> gives |->
    // With phase flip noise during RZ, we expect some |+> component
    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!((probs[0] + probs[1] - 1.0).abs() < 1e-6);
}

#[test]
fn test_rz_gate_multiple_rotations() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::RZ,
            Qubit::new(0),
            SingleQubitNoise::PhaseDamping(0.05),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));

    // Start in |+>
    sim.apply_h(0).unwrap();

    // Apply multiple RZ gates
    sim.apply_rz(0, PI / 4.0).unwrap();
    sim.apply_rz(0, PI / 4.0).unwrap();
    sim.apply_rz(0, PI / 4.0).unwrap();
    sim.apply_rz(0, PI / 4.0).unwrap();

    // Total rotation is pi, should give |->
    // Measure in X basis by applying H first
    sim.apply_h(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    // With phase damping, P(|1>) should be around 0.5
    assert!(probs[0] >= 0.0);
    assert!(probs[1] >= 0.0);
    assert!((probs[0] + probs[1] - 1.0).abs() < 1e-6);
}

#[test]
fn test_native_gates_combination() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X2P,
            Qubit::new(0),
            SingleQubitNoise::Depolarizing(0.05),
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::Y2P,
            Qubit::new(0),
            SingleQubitNoise::Depolarizing(0.05),
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::RZ,
            Qubit::new(0),
            SingleQubitNoise::Depolarizing(0.05),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));

    // X2P + Y2P + RZ(pi) sequence
    sim.apply_x2p(0).unwrap();
    sim.apply_y2p(0).unwrap();
    sim.apply_rz(0, PI).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!((probs[0] + probs[1] - 1.0).abs() < 1e-6);
}

#[test]
fn test_complex_circuit_native_gates_with_cz() {
    let mut circuit = Circuit::new(3);

    // Layer 1: Single-qubit rotations
    circuit.x2p(Qubit::new(0)).unwrap();
    circuit.y2p(Qubit::new(1)).unwrap();
    circuit.x2m(Qubit::new(2)).unwrap();

    // Layer 2: Two-qubit gates
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cz(Qubit::new(1), Qubit::new(2)).unwrap();

    // Layer 3: Single-qubit rotations with RZ
    circuit.rz(Qubit::new(0), PI / 3.0).unwrap();
    circuit.rz(Qubit::new(1), PI / 2.0).unwrap();
    circuit.rz(Qubit::new(2), PI / 4.0).unwrap();

    // Layer 4: XY gates
    circuit.xy2p(Qubit::new(0), PI / 6.0).unwrap();
    circuit.xy2m(Qubit::new(1), PI / 6.0).unwrap();
    circuit.xy(Qubit::new(2), PI / 3.0).unwrap();

    // Layer 5: Final CZ
    circuit.cz(Qubit::new(0), Qubit::new(2)).unwrap();

    let mut noise_model = NoiseModel::new();
    // Add noise to various gates
    noise_model
        .add_single_qubit_error(
            StandardGate::X2P,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(0.02),
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::RZ,
            Qubit::new(1),
            SingleQubitNoise::PhaseFlip(0.03),
        )
        .unwrap();
    noise_model
        .add_two_qubit_error(
            StandardGate::CZ,
            Qubit::new(0),
            Qubit::new(1),
            TwoQubitNoise::Depolarizing(0.01),
        )
        .unwrap();

    let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
    let probs = sim.probabilities_with_readout(&[0, 1, 2]).unwrap();
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);
}

#[test]
fn test_complex_circuit_entanglement_with_noise() {
    let mut circuit = Circuit::new(4);

    // Create GHZ-like state with native gates
    circuit.x2p(Qubit::new(0)).unwrap();
    circuit.x2p(Qubit::new(0)).unwrap(); // X

    // Entangle with CZ gates
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cz(Qubit::new(1), Qubit::new(2)).unwrap();
    circuit.cz(Qubit::new(2), Qubit::new(3)).unwrap();

    // Apply rotations
    circuit.rz(Qubit::new(0), PI / 4.0).unwrap();
    circuit.rz(Qubit::new(1), PI / 4.0).unwrap();
    circuit.rz(Qubit::new(2), PI / 4.0).unwrap();
    circuit.rz(Qubit::new(3), PI / 4.0).unwrap();

    // Additional entanglement
    circuit.cz(Qubit::new(0), Qubit::new(3)).unwrap();

    // Final rotations
    circuit.y2p(Qubit::new(0)).unwrap();
    circuit.y2m(Qubit::new(3)).unwrap();

    let mut noise_model = NoiseModel::new();
    // Add comprehensive noise
    for i in 0..4 {
        noise_model
            .add_single_qubit_error(
                StandardGate::RZ,
                Qubit::new(i),
                SingleQubitNoise::PhaseDamping(0.02),
            )
            .unwrap();
    }
    noise_model
        .add_two_qubit_error(
            StandardGate::CZ,
            Qubit::new(0),
            Qubit::new(1),
            TwoQubitNoise::Independent {
                q0_noise: SingleQubitNoise::BitFlip(0.01),
                q1_noise: SingleQubitNoise::BitFlip(0.01),
            },
        )
        .unwrap();
    noise_model
        .add_two_qubit_error(
            StandardGate::CZ,
            Qubit::new(2),
            Qubit::new(3),
            TwoQubitNoise::Independent {
                q0_noise: SingleQubitNoise::BitFlip(0.01),
                q1_noise: SingleQubitNoise::BitFlip(0.01),
            },
        )
        .unwrap();

    let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
    let probs = sim.probabilities_with_readout(&[0, 1, 2, 3]).unwrap();
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);

    // Verify we have some probability distribution across states
    let non_zero_probs: Vec<_> = probs.iter().filter(|&&p| p > 1e-10).collect();
    assert!(
        !non_zero_probs.is_empty(),
        "Should have non-zero probabilities"
    );
}

#[test]
fn test_complex_circuit_all_native_gates() {
    let mut circuit = Circuit::new(3);

    // Test all native gates in a complex circuit
    circuit.x2p(Qubit::new(0)).unwrap();
    circuit.x2m(Qubit::new(0)).unwrap();
    circuit.y2p(Qubit::new(1)).unwrap();
    circuit.y2m(Qubit::new(1)).unwrap();
    circuit.xy(Qubit::new(0), PI / 2.0).unwrap();
    circuit.xy2p(Qubit::new(1), PI / 4.0).unwrap();
    circuit.xy2m(Qubit::new(2), PI / 4.0).unwrap();
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.rz(Qubit::new(0), PI / 3.0).unwrap();
    circuit.rz(Qubit::new(1), PI / 2.0).unwrap();
    circuit.rz(Qubit::new(2), PI).unwrap();
    circuit.cz(Qubit::new(1), Qubit::new(2)).unwrap();
    circuit.xy(Qubit::new(2), PI).unwrap();

    let mut noise_model = NoiseModel::new();

    // Add noise to all gate types
    noise_model
        .add_single_qubit_error(
            StandardGate::X2P,
            Qubit::new(0),
            SingleQubitNoise::Depolarizing(0.01),
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::X2M,
            Qubit::new(0),
            SingleQubitNoise::Depolarizing(0.01),
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::Y2P,
            Qubit::new(1),
            SingleQubitNoise::Depolarizing(0.01),
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::Y2M,
            Qubit::new(1),
            SingleQubitNoise::Depolarizing(0.01),
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::XY,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(0.01),
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::XY2P,
            Qubit::new(1),
            SingleQubitNoise::BitFlip(0.01),
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::XY2M,
            Qubit::new(2),
            SingleQubitNoise::BitFlip(0.01),
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::RZ,
            Qubit::new(0),
            SingleQubitNoise::PhaseDamping(0.02),
        )
        .unwrap();
    noise_model
        .add_two_qubit_error(
            StandardGate::CZ,
            Qubit::new(0),
            Qubit::new(1),
            TwoQubitNoise::Depolarizing(0.02),
        )
        .unwrap();
    noise_model
        .add_two_qubit_error(
            StandardGate::CZ,
            Qubit::new(1),
            Qubit::new(2),
            TwoQubitNoise::Depolarizing(0.02),
        )
        .unwrap();

    let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
    let probs = sim.probabilities_with_readout(&[0, 1, 2]).unwrap();
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);
}

#[test]
fn test_complex_circuit_with_readout_and_gate_noise() {
    let mut circuit = Circuit::new(2);

    // Create Bell-like state using native gates
    circuit.x2p(Qubit::new(0)).unwrap();
    circuit.x2p(Qubit::new(0)).unwrap(); // X
    circuit.y2p(Qubit::new(1)).unwrap();
    circuit.y2m(Qubit::new(1)).unwrap();

    // Entangle
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();

    // Rotate to superposition
    circuit.rz(Qubit::new(0), PI / 2.0).unwrap();
    circuit.xy(Qubit::new(1), PI / 2.0).unwrap();

    // Another CZ
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();

    let mut noise_model = NoiseModel::new();

    // Gate noise
    noise_model
        .add_single_qubit_error(
            StandardGate::X2P,
            Qubit::new(0),
            SingleQubitNoise::Depolarizing(0.03),
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::RZ,
            Qubit::new(0),
            SingleQubitNoise::PhaseFlip(0.02),
        )
        .unwrap();
    noise_model
        .add_two_qubit_error(
            StandardGate::CZ,
            Qubit::new(0),
            Qubit::new(1),
            TwoQubitNoise::Independent {
                q0_noise: SingleQubitNoise::BitFlip(0.02),
                q1_noise: SingleQubitNoise::BitFlip(0.02),
            },
        )
        .unwrap();

    // Readout noise
    noise_model
        .add_readout_error(
            Qubit::new(0),
            ReadoutError {
                p_0_given_1: 0.05,
                p_1_given_0: 0.05,
            },
        )
        .unwrap();
    noise_model
        .add_readout_error(
            Qubit::new(1),
            ReadoutError {
                p_0_given_1: 0.03,
                p_1_given_0: 0.03,
            },
        )
        .unwrap();

    let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
    let probs = sim.probabilities_with_readout(&[0, 1]).unwrap();
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);

    // Verify all states have some probability
    for (i, &p) in probs.iter().enumerate() {
        assert!(
            (0.0..=1.0).contains(&p),
            "P(|{:02b}>) = {} is out of range",
            i,
            p
        );
    }
}

#[test]
fn test_native_gates_pauli_noise() {
    // Test Pauli noise on native gates
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X2P,
            Qubit::new(0),
            SingleQubitNoise::Pauli {
                px: 0.05,
                py: 0.05,
                pz: 0.05,
            },
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::Y2P,
            Qubit::new(0),
            SingleQubitNoise::Pauli {
                px: 0.05,
                py: 0.05,
                pz: 0.05,
            },
        )
        .unwrap();
    noise_model
        .add_single_qubit_error(
            StandardGate::RZ,
            Qubit::new(0),
            SingleQubitNoise::Pauli {
                px: 0.05,
                py: 0.05,
                pz: 0.05,
            },
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));

    // Sequence of native gates
    sim.apply_x2p(0).unwrap();
    sim.apply_y2p(0).unwrap();
    sim.apply_rz(0, PI / 2.0).unwrap();
    sim.apply_xy2p(0, PI / 4.0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!((probs[0] + probs[1] - 1.0).abs() < 1e-6);
    // Due to Pauli noise, both states should have some probability
    assert!(probs[0] > 0.0);
    assert!(probs[1] > 0.0);
}

#[test]
fn test_native_gates_amplitude_damping() {
    // Test amplitude damping on native gates
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X2P,
            Qubit::new(0),
            SingleQubitNoise::AmplitudeDamping(0.1),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));

    // Start in |1>
    sim.apply_x(0).unwrap();

    // Apply X2P with amplitude damping
    sim.apply_x2p(0).unwrap();

    let probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!((probs[0] + probs[1] - 1.0).abs() < 1e-6);
    // Amplitude damping increases P(|0>)
    assert!(probs[0] > 0.0);
}

#[test]
fn test_readout_uses_circuit_ids_at_selected_storage_positions() {
    use crate::device::Outcome;

    for ids in [[0, 1, 2], [2, 0, 1], [2, 5, 9], [9, 2, 5]] {
        for basis in 0..8 {
            for (noise_position, &noise_id) in ids.iter().enumerate() {
                let mut circuit =
                    Circuit::from_qubits(ids.iter().copied().map(Qubit::new).collect()).unwrap();
                for (position, &id) in ids.iter().enumerate() {
                    if basis & (1 << position) != 0 {
                        circuit.x(Qubit::new(id)).unwrap();
                    }
                }
                let receipt = circuit.measure(Qubit::new(noise_id)).unwrap();
                let mut model = NoiseModel::new();
                model
                    .add_readout_error(
                        Qubit::new(noise_id),
                        ReadoutError {
                            p_0_given_1: 0.2,
                            p_1_given_0: 0.7,
                        },
                    )
                    .unwrap();
                let sim = DensityMatrixNoise::from_circuit(&circuit, Some(model)).unwrap();
                let original = sim.state.data().to_vec();
                let bit = (basis >> noise_position) & 1;
                let flip_probability = if bit == 0 { 0.7 } else { 0.2 };

                for selection in 0..8 {
                    let positions: Vec<_> = (0..3)
                        .rev()
                        .filter(|&position| selection & (1 << position) != 0)
                        .collect();
                    let mut expected = [0.0; 8];
                    expected[basis] = 1.0;
                    if selection & (1 << noise_position) != 0 {
                        expected[basis] = 1.0 - flip_probability;
                        expected[basis ^ (1 << noise_position)] = flip_probability;
                    }
                    let actual = sim.probabilities_with_readout(&positions).unwrap();
                    assert_eq!(actual.len(), expected.len());
                    for (actual, expected) in actual.iter().zip(&expected) {
                        assert!(
                            (actual - expected).abs() < 1e-12,
                            "IDs={ids:?}, basis={basis}, noise_position={noise_position}, positions={positions:?}: {actual} != {expected}"
                        );
                    }
                }

                let marginal = sim.probs_with_readout(&receipt).unwrap();
                let p_one = if bit == 0 { 0.7 } else { 0.8 };
                assert!((marginal[&Outcome::from_bitstring("1").unwrap()] - p_one).abs() < 1e-12);
                assert_eq!(sim.state.data(), original.as_slice());
            }
        }
    }
}

#[test]
fn test_readout_combines_mapped_errors_without_reordering_full_probabilities() {
    use crate::device::Outcome;

    let mut circuit =
        Circuit::from_qubits(vec![Qubit::new(9), Qubit::new(2), Qubit::new(5)]).unwrap();
    circuit.x(Qubit::new(9)).unwrap();
    let receipt = circuit
        .measure_bits([Qubit::new(2), Qubit::new(9)])
        .unwrap();
    let mut model = NoiseModel::new();
    for (id, p_0_given_1, p_1_given_0) in [(9, 0.1, 0.2), (2, 0.3, 0.4)] {
        model
            .add_readout_error(
                Qubit::new(id),
                ReadoutError {
                    p_0_given_1,
                    p_1_given_0,
                },
            )
            .unwrap();
    }
    let sim = DensityMatrixNoise::from_circuit(&circuit, Some(model)).unwrap();
    let expected = [0.06, 0.54, 0.04, 0.36, 0.0, 0.0, 0.0, 0.0];
    for positions in [&[0, 1][..], &[1, 0], &[2, 1, 0]] {
        let actual = sim.probabilities_with_readout(positions).unwrap();
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-12);
        }
    }
    let marginal = sim.probs_with_readout(&receipt).unwrap();
    for (bits, probability) in [("00", 0.06), ("10", 0.54), ("01", 0.04), ("11", 0.36)] {
        assert!((marginal[&Outcome::from_bitstring(bits).unwrap()] - probability).abs() < 1e-12);
    }
    assert_eq!(
        sim.probabilities(),
        vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    );
}

#[test]
fn test_readout_validates_positions_independently_of_noise_model() {
    let mut model = NoiseModel::new();
    model
        .add_readout_error(
            Qubit::new(9),
            ReadoutError {
                p_0_given_1: 0.0,
                p_1_given_0: 1.0,
            },
        )
        .unwrap();
    for noise_model in [None, Some(NoiseModel::new()), Some(model)] {
        for bound in [false, true] {
            let mut sim = DensityMatrixNoise::new(2, noise_model.clone());
            if bound {
                let circuit = Circuit::from_qubits(vec![Qubit::new(9), Qubit::new(2)]).unwrap();
                sim.apply_circuit(&circuit).unwrap();
            }
            for index in [2, 9, usize::MAX] {
                assert!(matches!(
                    sim.probabilities_with_readout(&[0, index]),
                    Err(QisError::IndexOutOfBounds { index: actual, max: 1 }) if actual == index
                ));
            }
            assert_eq!(
                sim.probabilities_with_readout(&[]).unwrap(),
                sim.probabilities()
            );
        }
        let sim = DensityMatrixNoise::new(0, noise_model);
        assert_eq!(sim.probabilities_with_readout(&[]).unwrap(), vec![1.0]);
        assert!(matches!(
            sim.probabilities_with_readout(&[0]),
            Err(QisError::IndexOutOfBounds { index: 0, max: 0 })
        ));
    }
}

#[test]
fn test_apply_readout_noise_out_of_bounds() {
    let mut sim = DensityMatrixNoise::new(2, Some(NoiseModel::new()));
    sim.apply_h(0).unwrap();
    sim.apply_cx(0, 1).unwrap();

    assert!(matches!(
        sim.probabilities_with_readout(&[99]),
        Err(QisError::IndexOutOfBounds { index: 99, max: 1 })
    ));
}

#[test]
fn test_mcgate_fallback_applies_noise() {
    use crate::circuit::gate::MCGate;
    use crate::device::noise::TwoQubitNoise;

    let mut noise_model = NoiseModel::new();
    noise_model
        .add_two_qubit_error(
            StandardGate::H,
            Qubit::new(0),
            Qubit::new(1),
            TwoQubitNoise::Depolarizing(0.5),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(2, Some(noise_model));

    // Start with control qubit in |1⟩ so the controlled-H is active
    sim.apply_x(0).unwrap();

    // Build a circuit with a controlled-H (MCGate with base H, 1 control)
    let mut circuit = Circuit::new(2);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(1, StandardGate::H))),
            vec![Qubit::new(0), Qubit::new(1)],
            std::iter::empty::<ParameterValue>(),
            None,
        )
        .unwrap();

    sim.apply_circuit(&circuit).unwrap();

    // With 50% depolarizing noise on the CH gate, the state should be mixed.
    // Without noise, starting from |10⟩ and applying CH gives a pure state.
    // With 50% depolarizing noise, P(|10⟩) should drop significantly.
    let probs = sim.probabilities();
    assert!(
        probs[2] < 0.99,
        "Noise should have mixed the state, but P(|10⟩) = {}",
        probs[2]
    );
}

#[test]
fn test_expectation_accepts_pauli_string() {
    let mut sim = DensityMatrixNoise::new(1, None);
    sim.apply_x(0).unwrap();

    let mut z = PauliString::new(1);
    z.set_pauli(0, Pauli::Z);

    let exp = sim.expectation(&z).unwrap();
    assert!((exp + 1.0).abs() < 1e-10, "Z expectation on |1> was {exp}");
}

#[test]
fn test_sample_shots_with_readout_noise_model() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_readout_error(
            Qubit::new(0),
            ReadoutError {
                p_0_given_1: 1.0,
                p_1_given_0: 0.0,
            },
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
    sim.apply_x(0).unwrap();

    let shots = sim.sample_shots(128);
    assert_eq!(shots.len(), 128);
    assert!(
        shots.iter().all(|outcome| outcome.is_one(0)),
        "sample_shots samples the quantum state; readout noise is applied by probabilities_with_readout"
    );

    let readout_probs = sim.probabilities_with_readout(&[0]).unwrap();
    assert!((readout_probs[0] - 1.0).abs() < 1e-10);
}

#[test]
fn test_measurement_interfaces_keep_readout_noise_explicit() {
    use crate::device::Outcome;

    let mut noise_model = NoiseModel::new();
    noise_model
        .add_readout_error(
            Qubit::new(0),
            ReadoutError {
                p_0_given_1: 1.0,
                p_1_given_0: 0.0,
            },
        )
        .unwrap();

    let mut circuit = Circuit::new(1);
    circuit.x(Qubit::new(0)).unwrap();
    let out = circuit.measure(Qubit::new(0)).unwrap();

    let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
    let ideal = sim.probs(&out).unwrap();
    let with_readout = sim.probs_with_readout(&out).unwrap();

    assert!((ideal[&Outcome::from_bitstring("1").unwrap()] - 1.0).abs() < 1e-10);
    assert!((with_readout[&Outcome::from_bitstring("0").unwrap()] - 1.0).abs() < 1e-10);
}

#[test]
fn test_sample_uses_measurement_qubit_order_without_readout_noise() {
    use crate::device::{Outcome, Status};

    let mut noise_model = NoiseModel::new();
    noise_model
        .add_readout_error(
            Qubit::new(0),
            ReadoutError {
                p_0_given_1: 1.0,
                p_1_given_0: 0.0,
            },
        )
        .unwrap();

    let mut circuit = Circuit::new(2);
    circuit.x(Qubit::new(0)).unwrap();
    let out = circuit
        .measure_bits([Qubit::new(1), Qubit::new(0)])
        .unwrap();

    let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
    let result = sim.sample(&out, 16).unwrap();

    assert_eq!(result.shots(), 16);
    assert_eq!(result.num_qubits(), 2);
    assert_eq!(result.qubits(), &vec![Qubit::new(1), Qubit::new(0)]);
    assert_eq!(result.status(), &Status::Completed);
    assert_eq!(
        result.counts().get(&Outcome::from_bitstring("10").unwrap()),
        Some(&16)
    );
}

#[test]
fn test_from_circuit_ignores_terminal_measurement_declarations() {
    use crate::device::Outcome;

    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let out = circuit
        .measure_bits([Qubit::new(1), Qubit::new(0)])
        .unwrap();

    let sim = DensityMatrixNoise::from_circuit(&circuit, None).unwrap();
    let probs = sim.probs(&out).unwrap();

    assert_eq!(probs.len(), 2);
    assert!((probs[&Outcome::from_bitstring("00").unwrap()] - 0.5).abs() < 1e-10);
    assert!((probs[&Outcome::from_bitstring("11").unwrap()] - 0.5).abs() < 1e-10);
}

#[test]
fn test_apply_circuit_reset_directive() {
    let mut circuit = Circuit::new(1);
    circuit.x(Qubit::new(0)).unwrap();
    circuit.reset(Qubit::new(0)).unwrap();

    let mut sim = DensityMatrixNoise::new(1, None);
    sim.apply_circuit(&circuit).unwrap();

    let probs = sim.probabilities();
    assert!((probs[0] - 1.0).abs() < 1e-10);
    assert!(probs[1].abs() < 1e-10);
}

#[test]
fn test_apply_circuit_classical_control_flow_error() {
    use crate::circuit::ClassicalExpr;

    let mut circuit = Circuit::new(1);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.x(Qubit::new(0))?;
            Ok(())
        })
        .unwrap();

    let mut sim = DensityMatrixNoise::new(1, None);
    assert!(matches!(
        sim.apply_circuit(&circuit),
        Err(QisError::UnsupportedOperation(_))
    ));
}

#[test]
fn test_apply_standard_gate_noise_rejects_arity_and_param_mismatch() {
    let mut sim = DensityMatrixNoise::new(2, None);

    assert!(matches!(
        sim.apply_standard_gate_noise(StandardGate::CX, &[0], &[]),
        Err(QisError::InvalidParameterValue(_))
    ));
    assert!(matches!(
        sim.apply_standard_gate_noise(StandardGate::RX, &[0], &[]),
        Err(QisError::InvalidParameterValue(_))
    ));
}

#[test]
fn test_unitary_gate_fallback_applies_noise() {
    use crate::circuit::gate::UnitaryGate;

    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::U,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(1.0),
        )
        .unwrap();

    let mut circuit = Circuit::new(1);
    let x_matrix = array![
        [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
        [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)]
    ];
    let custom_x = UnitaryGate::new("CustomX", 1, 0)
        .with_matrix(x_matrix)
        .unwrap();
    circuit
        .append(
            Instruction::UnitaryGate(Box::new(custom_x)),
            vec![Qubit::new(0)],
            std::iter::empty::<ParameterValue>(),
            None,
        )
        .unwrap();

    let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
    let probs = sim.probabilities();
    assert!(
        (probs[0] - 1.0).abs() < 1e-10,
        "Custom X followed by configured U bit-flip noise should return to |0>, got {probs:?}"
    );
}

#[test]
fn test_ccx_with_noise_model_does_not_panic() {
    let mut noise_model = NoiseModel::new();
    noise_model
        .add_single_qubit_error(
            StandardGate::X,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(0.1),
        )
        .unwrap();

    let mut sim = DensityMatrixNoise::new(3, Some(noise_model));
    sim.apply_x(0).unwrap();
    sim.apply_x(1).unwrap();
    sim.apply_ccx(0, 1, 2).unwrap();

    let probs = sim.probabilities();
    assert!((probs.iter().sum::<f64>() - 1.0).abs() < 1e-10);
    assert!(
        probs[7] > 0.0,
        "CCX should produce support on |111>, got {probs:?}"
    );
}

#[test]
fn pauli_probability_boundary_preserves_simulation_trace() {
    let noise = SingleQubitNoise::Pauli {
        px: 0.3,
        py: 0.6,
        pz: 0.1,
    };
    let mut model = NoiseModel::new();
    model
        .add_single_qubit_error(StandardGate::X, Qubit::new(0), noise)
        .unwrap();
    let mut sim = DensityMatrixNoise::new(1, Some(model));
    sim.apply_x(0).unwrap();
    assert!((sim.probabilities()[0] - 0.9).abs() < 1e-12);
    assert!((sim.probabilities().iter().sum::<f64>() - 1.0).abs() < 1e-12);
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
fn failed_noisy_circuit_preserves_label_binding() {
    use crate::circuit::Parameter;
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

#[test]
fn identity_noise_respects_labels_and_repetition() {
    for ids in [[0, 1], [12, 5]] {
        for p in [0.0, 0.25, 1.0] {
            for count in [1, 2, 3] {
                let qs = ids.map(Qubit::new);
                let mut c = Circuit::from_qubits(qs.to_vec()).unwrap();
                for _ in 0..count {
                    c.i(qs[1]).unwrap();
                }
                let mut model = NoiseModel::new();
                model
                    .add_single_qubit_error(StandardGate::I, qs[1], SingleQubitNoise::BitFlip(p))
                    .unwrap();
                let sim = DensityMatrixNoise::from_circuit(&c, Some(model)).unwrap();
                let flipped = (1.0 - (1.0 - 2.0 * p).powi(count)) / 2.0;
                for (actual, expected) in
                    sim.probabilities()
                        .iter()
                        .zip([1.0 - flipped, 0.0, flipped, 0.0])
                {
                    assert!((actual - expected).abs() < 1e-12);
                }
                assert_eq!(
                    DensityMatrixNoise::from_circuit(&c, None)
                        .unwrap()
                        .probabilities(),
                    vec![1.0, 0.0, 0.0, 0.0]
                );
            }
        }
    }
    let mut sim = DensityMatrixNoise::new(1, None);
    assert!(
        sim.apply_standard_gate_noise(StandardGate::I, &[1], &[])
            .is_err()
    );
}

#[test]
fn controlled_gphase_circuit_evolution() {
    for ids in [[0, 1, 2], [12, 5, 31]] {
        let qubits = ids.map(Qubit::new);
        for controls in [
            vec![qubits[2]],
            vec![qubits[2], qubits[0]],
            vec![qubits[0], qubits[2]],
        ] {
            for theta in [0.0, 0.37, -0.51, PI] {
                let mut preparation = Circuit::from_qubits(qubits.to_vec()).unwrap();
                for qubit in qubits {
                    preparation.h(qubit).unwrap();
                }
                preparation.s(qubits[1]).unwrap();
                let mut phase = Circuit::from_qubits(qubits.to_vec()).unwrap();
                phase
                    .multi_control(
                        StandardGate::GPhase,
                        controls.clone(),
                        Vec::<Qubit>::new(),
                        [theta.into()],
                    )
                    .unwrap();
                let mut circuit = preparation.clone();
                circuit
                    .multi_control(
                        StandardGate::GPhase,
                        controls.clone(),
                        Vec::<Qubit>::new(),
                        [theta.into()],
                    )
                    .unwrap();
                let mask = controls.iter().fold(0usize, |mask, q| {
                    mask | (1 << qubits.iter().position(|candidate| candidate == q).unwrap())
                });
                let expected: Vec<Complex64> = (0..8)
                    .map(|index| {
                        let amplitude = if index & 2 == 0 {
                            Complex64::new(1.0, 0.0)
                        } else {
                            Complex64::new(0.0, 1.0)
                        } / 8.0_f64.sqrt();
                        if index & mask == mask {
                            amplitude * Complex64::from_polar(1.0, theta)
                        } else {
                            amplitude
                        }
                    })
                    .collect();
                let expected_data = expected
                    .iter()
                    .flat_map(|a| expected.iter().map(move |b| a * b.conj()))
                    .collect::<Vec<_>>();
                let actual = DensityMatrixNoise::from_circuit(&circuit, None).unwrap();
                let mut incremental = DensityMatrixNoise::new(3, None);
                incremental.apply_circuit(&preparation).unwrap();
                incremental.apply_circuit(&phase).unwrap();
                for values in [actual.state.data(), incremental.state.data()] {
                    for (actual, expected) in values.iter().zip(&expected_data) {
                        assert!(
                            (actual - expected).norm() < 1e-12,
                            "{actual:?} != {expected:?}"
                        );
                    }
                }
                let mut inverse = Circuit::from_qubits(qubits.to_vec()).unwrap();
                inverse
                    .multi_control(
                        StandardGate::GPhase,
                        controls.clone(),
                        Vec::<Qubit>::new(),
                        [(-theta).into()],
                    )
                    .unwrap();
                incremental.apply_circuit(&inverse).unwrap();
                let baseline = DensityMatrixNoise::from_circuit(&preparation, None).unwrap();
                for (actual, expected) in incremental.state.data().iter().zip(baseline.state.data())
                {
                    assert!((actual - expected).norm() < 1e-12);
                }
            }
        }
    }
}

#[test]
fn controlled_gphase_noise_applies_after_unitary_once() {
    let qubits = [Qubit::new(12), Qubit::new(5)];
    let mut circuit = Circuit::from_qubits(qubits.to_vec()).unwrap();
    circuit.x(qubits[0]).unwrap();
    circuit.h(qubits[1]).unwrap();
    circuit
        .multi_control(
            StandardGate::GPhase,
            [qubits[1]],
            Vec::<Qubit>::new(),
            [0.37.into()],
        )
        .unwrap();
    let mut model = NoiseModel::new();
    model
        .add_single_qubit_error(
            StandardGate::GPhase,
            qubits[1],
            SingleQubitNoise::BitFlip(0.23),
        )
        .unwrap();
    model
        .add_single_qubit_error(
            StandardGate::GPhase,
            qubits[0],
            SingleQubitNoise::BitFlip(1.0),
        )
        .unwrap();
    let sim = DensityMatrixNoise::from_circuit(&circuit, Some(model)).unwrap();
    let amplitudes = [
        Complex64::new(0.0, 0.0),
        Complex64::new(1.0, 0.0) / 2.0_f64.sqrt(),
        Complex64::new(0.0, 0.0),
        Complex64::from_polar(1.0 / 2.0_f64.sqrt(), 0.37),
    ];
    for row in 0..4 {
        for col in 0..4 {
            let expected = 0.77 * amplitudes[row] * amplitudes[col].conj()
                + 0.23 * amplitudes[row ^ 2] * amplitudes[col ^ 2].conj();
            assert!((sim.state.data()[row * 4 + col] - expected).norm() < 1e-12);
        }
    }
}

#[test]
fn zero_control_gphase_circuit_evolution() {
    use crate::circuit::gate::{Instruction, MCGate};
    for num_qubits in [0, 1, 3] {
        for theta in [0.0, 0.37, -0.51, PI] {
            let mut circuit = Circuit::new(num_qubits);
            if num_qubits > 0 {
                circuit.h(Qubit::new(0)).unwrap();
            }
            circuit
                .append(
                    Instruction::McGate(Box::new(MCGate::new(0, StandardGate::GPhase))),
                    Vec::<Qubit>::new(),
                    [theta.into()],
                    None,
                )
                .unwrap();
            let actual = DensityMatrixNoise::from_circuit(&circuit, None).unwrap();
            let mut amplitudes = vec![Complex64::new(0.0, 0.0); 1 << num_qubits];
            amplitudes[0] = Complex64::from_polar(1.0, theta);
            if num_qubits > 0 {
                amplitudes[0] /= 2.0_f64.sqrt();
                amplitudes[1] = amplitudes[0];
            }
            let expected = amplitudes
                .iter()
                .flat_map(|a| amplitudes.iter().map(move |b| a * b.conj()))
                .collect::<Vec<_>>();
            for (actual, expected) in actual.state.data().iter().zip(&expected) {
                assert!((actual - expected).norm() < 1e-12);
            }
        }
    }
}
