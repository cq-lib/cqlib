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
//
// Modified to validate terminal measurements and clarify or correct reset behavior.

//! Simulation consistency tests between Statevector and DensityMatrix

use crate::circuit::{Circuit, Qubit};
use crate::qis::pauli::{Pauli, PauliString};
use crate::qis::{DensityMatrix, DensityMatrixNoise, QisError, StabilizerState, Statevector};
use std::f64::consts::PI;
const EPSILON: f64 = 1e-10;

#[test]
fn mid_circuit_measurement_is_rejected_before_mutating_any_state() {
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.measure(Qubit::new(0)).unwrap();
    circuit.h(Qubit::new(0)).unwrap();

    let constructors = [
        Statevector::from_circuit(&circuit).map(|_| ()),
        DensityMatrix::from_circuit(&circuit).map(|_| ()),
        DensityMatrixNoise::from_circuit(&circuit, None).map(|_| ()),
        StabilizerState::from_circuit(&circuit).map(|_| ()),
    ];
    for result in constructors {
        let Err(QisError::UnsupportedOperation(message)) = result else {
            panic!("expected unsupported mid-circuit measurement");
        };
        assert!(message.contains("mid-circuit measurement on qubit 0 at operation 1"));
        assert!(message.contains("later 'H' at operation 2"));
    }

    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);
    let mut noisy = DensityMatrixNoise::new(1, None);
    let mut stab = StabilizerState::new(1);
    let sv_before = sv.data().to_vec();
    let dm_before = dm.data().to_vec();
    let noisy_before = noisy.state.data().to_vec();
    let stab_before = stab.to_stim_format();
    for result in [
        sv.apply_circuit(&circuit),
        dm.apply_circuit(&circuit),
        noisy.apply_circuit(&circuit),
        stab.apply_circuit(&circuit),
    ] {
        assert!(matches!(result, Err(QisError::UnsupportedOperation(_))));
    }
    assert_eq!(sv.data(), sv_before);
    assert_eq!(dm.data(), dm_before);
    assert_eq!(noisy.state.data(), noisy_before);
    assert_eq!(stab.to_stim_format(), stab_before);
}

#[test]
fn terminal_measurements_allow_independent_gates_and_ignored_operations() {
    use crate::circuit::ClassicalType;

    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    let bit = circuit.var(ClassicalType::Bit);
    circuit.measure_into(Qubit::new(0), bit).unwrap();
    circuit.barrier(vec![Qubit::new(0), Qubit::new(1)]).unwrap();
    circuit.delay(Qubit::new(0), 1.0.into()).unwrap();
    circuit.h(Qubit::new(1)).unwrap();
    circuit
        .measure_bits([Qubit::new(1), Qubit::new(0)])
        .unwrap();

    let probabilities = [
        Statevector::from_circuit(&circuit).unwrap().probabilities(),
        DensityMatrix::from_circuit(&circuit)
            .unwrap()
            .probabilities(),
        DensityMatrixNoise::from_circuit(&circuit, None)
            .unwrap()
            .probabilities(),
        StabilizerState::from_circuit(&circuit)
            .unwrap()
            .probabilities()
            .unwrap(),
    ];
    for probs in probabilities {
        compare_probs(&probs, &[0.25; 4], "terminal measurements");
    }
}

/// Compare probabilities from statevector and density matrix
fn compare_probs(sv_probs: &[f64], dm_probs: &[f64], desc: &str) {
    assert_eq!(
        sv_probs.len(),
        dm_probs.len(),
        "{}: probability length mismatch",
        desc
    );
    for (i, (sv_p, dm_p)) in sv_probs.iter().zip(dm_probs.iter()).enumerate() {
        assert!(
            (sv_p - dm_p).abs() < EPSILON,
            "{}: P(|{}>) mismatch: sv={}, dm={}",
            desc,
            i,
            sv_p,
            dm_p
        );
    }
}

fn compare_three_probs(sv_probs: &[f64], dm_probs: &[f64], stab_probs: &[f64], desc: &str) {
    compare_probs(sv_probs, dm_probs, desc);
    compare_probs(sv_probs, stab_probs, desc);
}

fn pauli_string(num_qubits: usize, terms: &[(usize, Pauli)]) -> PauliString {
    let mut ps = PauliString::new(num_qubits);
    for &(q, p) in terms {
        ps.set_pauli(q, p);
    }
    ps
}

#[test]
fn test_single_qubit_x_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_x(0).unwrap();
    dm.apply_x(0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "X gate");
}

#[test]
fn test_single_qubit_y_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_y(0).unwrap();
    dm.apply_y(0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "Y gate");
}

#[test]
fn test_single_qubit_z_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_h(0).unwrap();
    sv.apply_z(0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_z(0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "Z gate");
}

#[test]
fn test_single_qubit_h_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_h(0).unwrap();
    dm.apply_h(0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "H gate");
}

#[test]
fn test_single_qubit_s_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_h(0).unwrap();
    sv.apply_s(0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_s(0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "S gate");
}

#[test]
fn test_single_qubit_sdg_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_h(0).unwrap();
    sv.apply_sdg(0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_sdg(0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "SDG gate");
}

#[test]
fn test_single_qubit_t_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_h(0).unwrap();
    sv.apply_t(0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_t(0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "T gate");
}

#[test]
fn test_single_qubit_tdg_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_h(0).unwrap();
    sv.apply_tdg(0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_tdg(0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "TDG gate");
}

#[test]
fn test_single_qubit_rx_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_rx(0, PI / 3.0).unwrap();
    dm.apply_rx(0, PI / 3.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "RX gate");
}

#[test]
fn test_single_qubit_ry_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_ry(0, PI / 4.0).unwrap();
    dm.apply_ry(0, PI / 4.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "RY gate");
}

#[test]
fn test_single_qubit_rz_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_h(0).unwrap();
    sv.apply_rz(0, PI / 6.0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_rz(0, PI / 6.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "RZ gate");
}

#[test]
fn test_single_qubit_p_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_h(0).unwrap();
    sv.apply_phase(0, PI / 3.0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_phase(0, PI / 3.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "P gate");
}

#[test]
fn test_single_qubit_x2p_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_x2p(0).unwrap();
    dm.apply_x2p(0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "X2P gate");
}

#[test]
fn test_single_qubit_x2m_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_x2m(0).unwrap();
    dm.apply_x2m(0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "X2M gate");
}

#[test]
fn test_single_qubit_y2p_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_y2p(0).unwrap();
    dm.apply_y2p(0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "Y2P gate");
}

#[test]
fn test_single_qubit_y2m_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_y2m(0).unwrap();
    dm.apply_y2m(0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "Y2M gate");
}

#[test]
fn test_single_qubit_xy_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_xy(0, PI / 4.0).unwrap();
    dm.apply_xy(0, PI / 4.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "XY gate");
}

#[test]
fn test_single_qubit_xy2p_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_xy2p(0, PI / 4.0).unwrap();
    dm.apply_xy2p(0, PI / 4.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "XY2P gate");
}

#[test]
fn test_single_qubit_xy2m_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_xy2m(0, PI / 4.0).unwrap();
    dm.apply_xy2m(0, PI / 4.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "XY2M gate");
}

#[test]
fn test_single_qubit_rxy_gate() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_rxy(0, PI / 3.0, PI / 6.0).unwrap();
    dm.apply_rxy(0, PI / 3.0, PI / 6.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "RXY gate");
}

#[test]
fn test_two_qubit_cx_gate() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    sv.apply_h(0).unwrap();
    sv.apply_cx(0, 1).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_cx(0, 1).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "CX gate");
}

#[test]
fn test_two_qubit_cy_gate() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    sv.apply_h(0).unwrap();
    sv.apply_cy(0, 1).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_cy(0, 1).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "CY gate");
}

#[test]
fn test_two_qubit_cz_gate() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    sv.apply_h(0).unwrap();
    sv.apply_h(1).unwrap();
    sv.apply_cz(0, 1).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_h(1).unwrap();
    dm.apply_cz(0, 1).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "CZ gate");
}

#[test]
fn test_two_qubit_swap_gate() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    sv.apply_x(0).unwrap();
    sv.apply_swap(0, 1).unwrap();

    dm.apply_x(0).unwrap();
    dm.apply_swap(0, 1).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "SWAP gate");
}

#[test]
fn test_two_qubit_crx_gate() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    sv.apply_h(0).unwrap();
    sv.apply_crx(0, 1, PI / 3.0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_crx(0, 1, PI / 3.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "CRX gate");
}

#[test]
fn test_two_qubit_cry_gate() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    sv.apply_h(0).unwrap();
    sv.apply_cry(0, 1, PI / 4.0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_cry(0, 1, PI / 4.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "CRY gate");
}

#[test]
fn test_two_qubit_crz_gate() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    sv.apply_h(0).unwrap();
    sv.apply_h(1).unwrap();
    sv.apply_crz(0, 1, PI / 6.0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_h(1).unwrap();
    dm.apply_crz(0, 1, PI / 6.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "CRZ gate");
}

#[test]
fn test_two_qubit_rxx_gate() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    sv.apply_h(0).unwrap();
    sv.apply_h(1).unwrap();
    sv.apply_rxx(0, 1, PI / 3.0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_h(1).unwrap();
    dm.apply_rxx(0, 1, PI / 3.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "RXX gate");
}

#[test]
fn test_two_qubit_ryy_gate() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    sv.apply_h(0).unwrap();
    sv.apply_h(1).unwrap();
    sv.apply_ryy(0, 1, PI / 4.0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_h(1).unwrap();
    dm.apply_ryy(0, 1, PI / 4.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "RYY gate");
}

#[test]
fn test_two_qubit_rzz_gate() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    sv.apply_h(0).unwrap();
    sv.apply_h(1).unwrap();
    sv.apply_rzz(0, 1, PI / 6.0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_h(1).unwrap();
    dm.apply_rzz(0, 1, PI / 6.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "RZZ gate");
}

#[test]
fn test_two_qubit_rzx_gate() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    sv.apply_h(0).unwrap();
    sv.apply_h(1).unwrap();
    sv.apply_rzx(0, 1, PI / 3.0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_h(1).unwrap();
    dm.apply_rzx(0, 1, PI / 3.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "RZX gate");
}

#[test]
fn test_two_qubit_fsim_gate() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    sv.apply_h(0).unwrap();
    sv.apply_h(1).unwrap();
    sv.apply_fsim(0, 1, PI / 4.0, PI / 8.0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_h(1).unwrap();
    dm.apply_fsim(0, 1, PI / 4.0, PI / 8.0).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "FSIM gate");
}

#[test]
fn test_three_qubit_ccx_gate() {
    let mut sv = Statevector::new(3);
    let mut dm = DensityMatrix::new(3);

    sv.apply_h(0).unwrap();
    sv.apply_h(1).unwrap();
    sv.apply_ccx(0, 1, 2).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_h(1).unwrap();
    dm.apply_ccx(0, 1, 2).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "CCX gate");
}

#[test]
fn test_bell_state_circuit() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    // Create Bell state
    sv.apply_h(0).unwrap();
    sv.apply_cx(0, 1).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_cx(0, 1).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "Bell state");
}

#[test]
fn test_ghz_state_circuit() {
    let mut sv = Statevector::new(3);
    let mut dm = DensityMatrix::new(3);

    // Create GHZ state
    sv.apply_h(0).unwrap();
    sv.apply_cx(0, 1).unwrap();
    sv.apply_cx(1, 2).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_cx(0, 1).unwrap();
    dm.apply_cx(1, 2).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "GHZ state");
}

#[test]
fn test_complex_circuit_native_gates() {
    let mut sv = Statevector::new(3);
    let mut dm = DensityMatrix::new(3);

    sv.apply_x2p(0).unwrap();
    sv.apply_y2m(1).unwrap();
    sv.apply_xy2p(2, PI / 4.0).unwrap();
    sv.apply_cz(0, 1).unwrap();
    sv.apply_rz(0, PI / 3.0).unwrap();
    sv.apply_swap(1, 2).unwrap();

    dm.apply_x2p(0).unwrap();
    dm.apply_y2m(1).unwrap();
    dm.apply_xy2p(2, PI / 4.0).unwrap();
    dm.apply_cz(0, 1).unwrap();
    dm.apply_rz(0, PI / 3.0).unwrap();
    dm.apply_swap(1, 2).unwrap();

    compare_probs(
        &sv.probabilities(),
        &dm.probabilities(),
        "Complex native gates",
    );
}

#[test]
fn test_from_circuit_consistency() {
    use crate::circuit::Qubit;

    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.rx(Qubit::new(1), PI / 3.0).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();

    let sv = Statevector::from_circuit(&circuit).unwrap();
    let dm = DensityMatrix::from_circuit(&circuit).unwrap();

    compare_probs(&sv.probabilities(), &dm.probabilities(), "from_circuit");
}

#[test]
fn test_zero_angle_rotations() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_h(0).unwrap();
    sv.apply_rx(0, 0.0).unwrap();
    sv.apply_ry(0, 0.0).unwrap();
    sv.apply_rz(0, 0.0).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_rx(0, 0.0).unwrap();
    dm.apply_ry(0, 0.0).unwrap();
    dm.apply_rz(0, 0.0).unwrap();

    compare_probs(
        &sv.probabilities(),
        &dm.probabilities(),
        "Zero angle rotations",
    );
}

#[test]
fn test_pi_angle_rotations() {
    let mut sv = Statevector::new(1);
    let mut dm = DensityMatrix::new(1);

    sv.apply_rx(0, PI).unwrap();
    sv.apply_ry(0, PI).unwrap();

    dm.apply_rx(0, PI).unwrap();
    dm.apply_ry(0, PI).unwrap();

    compare_probs(
        &sv.probabilities(),
        &dm.probabilities(),
        "PI angle rotations",
    );
}

#[test]
fn test_sequential_single_qubit_gates() {
    let mut sv = Statevector::new(2);
    let mut dm = DensityMatrix::new(2);

    // Apply many gates in sequence
    for _ in 0..10 {
        sv.apply_h(0).unwrap();
        sv.apply_x(1).unwrap();
        sv.apply_y(0).unwrap();
        sv.apply_z(1).unwrap();
    }

    for _ in 0..10 {
        dm.apply_h(0).unwrap();
        dm.apply_x(1).unwrap();
        dm.apply_y(0).unwrap();
        dm.apply_z(1).unwrap();
    }

    compare_probs(&sv.probabilities(), &dm.probabilities(), "Sequential gates");
}

#[test]
fn test_multiple_entangling_gates() {
    let mut sv = Statevector::new(3);
    let mut dm = DensityMatrix::new(3);

    // Layer 1: Entangle
    sv.apply_h(0).unwrap();
    sv.apply_cx(0, 1).unwrap();
    sv.apply_cx(1, 2).unwrap();

    dm.apply_h(0).unwrap();
    dm.apply_cx(0, 1).unwrap();
    dm.apply_cx(1, 2).unwrap();

    // Layer 2: Single-qubit rotations
    sv.apply_rx(0, PI / 4.0).unwrap();
    sv.apply_ry(1, PI / 3.0).unwrap();
    sv.apply_rz(2, PI / 6.0).unwrap();

    dm.apply_rx(0, PI / 4.0).unwrap();
    dm.apply_ry(1, PI / 3.0).unwrap();
    dm.apply_rz(2, PI / 6.0).unwrap();

    // Layer 3: More entanglement
    sv.apply_cz(0, 2).unwrap();
    sv.apply_swap(0, 1).unwrap();

    dm.apply_cz(0, 2).unwrap();
    dm.apply_swap(0, 1).unwrap();

    compare_probs(
        &sv.probabilities(),
        &dm.probabilities(),
        "Multiple entangling gates",
    );
}

#[test]
fn test_clifford_circuits_match_stabilizer_state() {
    let mut bell = Circuit::new(2);
    bell.h(Qubit::new(0)).unwrap();
    bell.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let mut graph = Circuit::new(3);
    graph.h(Qubit::new(0)).unwrap();
    graph.h(Qubit::new(1)).unwrap();
    graph.s(Qubit::new(2)).unwrap();
    graph.cz(Qubit::new(0), Qubit::new(1)).unwrap();
    graph.cx(Qubit::new(1), Qubit::new(2)).unwrap();

    let cases = vec![
        (
            "bell",
            bell,
            vec![
                pauli_string(2, &[(0, Pauli::X), (1, Pauli::X)]),
                pauli_string(2, &[(0, Pauli::Z), (1, Pauli::Z)]),
                pauli_string(2, &[(0, Pauli::Z)]),
            ],
        ),
        (
            "graph",
            graph,
            vec![
                pauli_string(3, &[(0, Pauli::X), (1, Pauli::Z)]),
                pauli_string(3, &[(1, Pauli::X), (0, Pauli::Z), (2, Pauli::X)]),
                pauli_string(3, &[(2, Pauli::Z)]),
            ],
        ),
    ];

    for (name, circuit, observables) in cases {
        let sv = Statevector::from_circuit(&circuit).unwrap();
        let dm = DensityMatrix::from_circuit(&circuit).unwrap();
        let stab = StabilizerState::from_circuit(&circuit).unwrap();

        compare_three_probs(
            &sv.probabilities(),
            &dm.probabilities(),
            &stab.probabilities().unwrap(),
            name,
        );

        for obs in observables {
            let sv_exp = sv.expectation(&obs).unwrap();
            let dm_exp = dm.expectation(&obs).unwrap();
            let stab_exp = stab.pauli_expectation(&obs).unwrap() as f64;
            assert!(
                (sv_exp - dm_exp).abs() < EPSILON,
                "{name}: statevector/density expectation mismatch for {obs}: {sv_exp} vs {dm_exp}"
            );
            assert!(
                (sv_exp - stab_exp).abs() < EPSILON,
                "{name}: statevector/stabilizer expectation mismatch for {obs}: {sv_exp} vs {stab_exp}"
            );
        }
    }
}
