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

use super::VirtualDistillation;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::gate::Instruction;
use crate::circuit::gate::standard_gate::StandardGate;
use crate::circuit::{CircuitParam, Qubit};
use crate::error_mitigation::ErrorMitigationError;
use crate::qis::{DensityMatrix, Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;

fn single_qubit_z_hamiltonian() -> Hamiltonian {
    let mut pauli_string = PauliString::new(1);
    pauli_string.set_pauli(0, Pauli::Z);
    Hamiltonian::from_list(vec![(pauli_string, Complex64::new(1.0, 0.0))])
        .expect("single-qubit Z Hamiltonian should be valid")
}

fn single_qubit_x_hamiltonian() -> Hamiltonian {
    let mut pauli_string = PauliString::new(1);
    pauli_string.set_pauli(0, Pauli::X);
    Hamiltonian::from_list(vec![(pauli_string, Complex64::new(1.0, 0.0))])
        .expect("single-qubit X Hamiltonian should be valid")
}

/// Exact density-matrix estimator used by the numerical tests.
fn exact_estimator(
    circuit: &Circuit,
    hamiltonian: Option<&Hamiltonian>,
    _shots: Option<usize>,
) -> (f64, f64) {
    let mut state = DensityMatrix::new(circuit.num_qubits());
    state.apply_circuit(circuit).unwrap();
    let hamiltonian = hamiltonian.expect("numerator and denominator both provide a Hamiltonian");
    (state.expectation(hamiltonian).unwrap(), 0.0)
}

/// Estimator that prepares `copies` depolarized states and then applies the
/// controlled derangement. It simulates the preparation ops, injects an
/// identical depolarizing channel with strength `p` on every copy register
/// (all qubits except the ancillary), and finally applies the Hadamard on the
/// ancillary and the controlled swap ladder. Depolarizing the disjoint copy
/// registers after the whole preparation phase is equivalent to depolarizing
/// each copy right after its own preparation.
fn depolarizing_estimator(
    p: f64,
) -> impl Fn(&Circuit, Option<&Hamiltonian>, Option<usize>) -> (f64, f64) {
    move |circuit, hamiltonian, _shots| {
        let num_qubits = circuit.num_qubits();
        let mut state = DensityMatrix::new(num_qubits);

        let keep = (1.0 - p).sqrt();
        let flip = (p / 3.0).sqrt();
        let c = |re: f64, im: f64| Complex64::new(re, im);
        let k0 = vec![c(keep, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(keep, 0.0)];
        let k1 = vec![c(0.0, 0.0), c(flip, 0.0), c(flip, 0.0), c(0.0, 0.0)];
        let k2 = vec![c(0.0, 0.0), c(0.0, -flip), c(0.0, flip), c(0.0, 0.0)];
        let k3 = vec![c(flip, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(-flip, 0.0)];

        // The Hadamard on the ancillary qubit separates the preparation phase
        // from the derangement phase.
        let derangement_start = circuit
            .operations()
            .iter()
            .position(|op| matches!(op.instruction, Instruction::Standard(StandardGate::H)))
            .expect("virtual distillation circuit must contain the ancillary Hadamard");

        let mut noise_injected = false;
        for (index, op) in circuit.operations().iter().enumerate() {
            if !noise_injected && index == derangement_start {
                for qubit in 0..(num_qubits - 1) {
                    state
                        .apply_kraus(&[k0.clone(), k1.clone(), k2.clone(), k3.clone()], &[qubit])
                        .unwrap();
                }
                noise_injected = true;
            }

            let qubit_indices: Vec<usize> = op.qubits.iter().map(|q| q.id() as usize).collect();
            let params: Vec<f64> = op
                .params
                .iter()
                .map(|param| match param {
                    CircuitParam::Fixed(value) => *value,
                    CircuitParam::Index(_) => panic!("test circuits use fixed parameters only"),
                })
                .collect();
            match &op.instruction {
                Instruction::Standard(gate) => {
                    state
                        .apply_standard_gate(*gate, &qubit_indices, &params)
                        .unwrap();
                }
                Instruction::McGate(mc_gate) => {
                    let matrix = mc_gate.matrix(&params).unwrap();
                    state.apply_unitary_gate(&qubit_indices, &matrix).unwrap();
                }
                instruction => panic!("unexpected instruction in VD circuit: {instruction:?}"),
            }
        }

        let hamiltonian =
            hamiltonian.expect("numerator and denominator both provide a Hamiltonian");
        (state.expectation(hamiltonian).unwrap(), 0.0)
    }
}

#[test]
fn test_vd_new_accepts_valid_input() {
    let circuit = Circuit::new(1);
    let vd = VirtualDistillation::new(circuit, 2);
    assert!(vd.is_ok());
}

#[test]
fn test_vd_new_rejects_invalid_copies() {
    let circuit = Circuit::new(1);
    let err = VirtualDistillation::new(circuit, 1).unwrap_err();
    assert!(matches!(err, ErrorMitigationError::InvalidCopies(1)));
}

#[test]
fn test_vd_copies_getter_and_setter() {
    let circuit = Circuit::new(1);
    let mut vd = VirtualDistillation::new(circuit, 2).unwrap();

    assert_eq!(vd.copies(), 2);

    vd.set_copies(3).unwrap();
    assert_eq!(vd.copies(), 3);

    let err = vd.set_copies(1).unwrap_err();
    assert!(matches!(err, ErrorMitigationError::InvalidCopies(1)));
    assert_eq!(vd.copies(), 3);
}

#[test]
fn test_build_copy_swap_circuit_for_two_single_qubit_copies() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.x(q0).unwrap();

    let vd = VirtualDistillation::new(circuit, 2).unwrap();
    let copy_swap = vd.build_copy_swap_circuit().unwrap();
    let ops = copy_swap.operations();

    // Two base-circuit copies, a Hadamard on the ancillary qubit, and one
    // ancillary-controlled SWAP (CSWAP) between the copies.
    assert_eq!(copy_swap.width(), 3);
    assert_eq!(ops.len(), 4);

    assert!(matches!(
        ops[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert_eq!(ops[0].qubits.as_slice(), &[Qubit::new(0)]);

    assert!(matches!(
        ops[1].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert_eq!(ops[1].qubits.as_slice(), &[Qubit::new(1)]);

    assert!(matches!(
        ops[2].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert_eq!(ops[2].qubits.as_slice(), &[Qubit::new(2)]);

    match &ops[3].instruction {
        Instruction::McGate(mc_gate) => {
            assert_eq!(mc_gate.num_ctrl_qubits(), 1);
            assert_eq!(*mc_gate.base_gate(), StandardGate::SWAP);
        }
        instruction => panic!("expected a controlled SWAP, got {instruction:?}"),
    }
    assert_eq!(
        ops[3].qubits.as_slice(),
        &[Qubit::new(2), Qubit::new(0), Qubit::new(1)]
    );
}

#[test]
fn test_build_copy_swap_circuit_adds_controlled_swap_ladder_for_multiple_copies() {
    let vd = VirtualDistillation::new(Circuit::new(1), 3).unwrap();
    let copy_swap = vd.build_copy_swap_circuit().unwrap();
    let ops = copy_swap.operations();

    assert_eq!(copy_swap.width(), 4);
    assert_eq!(ops.len(), 3);

    assert!(matches!(
        ops[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert_eq!(ops[0].qubits.as_slice(), &[Qubit::new(3)]);

    let cswap_qubits: Vec<_> = ops[1..]
        .iter()
        .map(|op| {
            match &op.instruction {
                Instruction::McGate(mc_gate) => {
                    assert_eq!(mc_gate.num_ctrl_qubits(), 1);
                    assert_eq!(*mc_gate.base_gate(), StandardGate::SWAP);
                }
                instruction => panic!("expected a controlled SWAP, got {instruction:?}"),
            }
            op.qubits.as_slice().to_vec()
        })
        .collect();
    // Swap ladder between adjacent copies, all controlled by the ancillary.
    assert_eq!(
        cswap_qubits,
        vec![
            vec![Qubit::new(3), Qubit::new(0), Qubit::new(1)],
            vec![Qubit::new(3), Qubit::new(1), Qubit::new(2)],
        ]
    );
}

#[test]
fn test_expand_hamiltonian_places_x_on_ancilla_and_identity_on_extra_copies() {
    let hamiltonian = single_qubit_x_hamiltonian();
    let expanded = VirtualDistillation::expand_hamiltonian(&hamiltonian, 3).unwrap();

    assert_eq!(expanded.num_qubits, 4);
    assert_eq!(expanded.terms.len(), 1);

    let (term, coeff) = &expanded.terms[0];
    assert_eq!(*coeff, Complex64::new(1.0, 0.0));
    assert_eq!(term.num_qubits, 4);
    assert_eq!(term.phase, crate::qis::Phase::Plus);

    // Observable on the first copy, identities on the extra copies, X on the
    // ancillary qubit.
    assert_eq!((term.x[0], term.z[0]), (true, false));
    assert_eq!((term.x[1], term.z[1]), (false, false));
    assert_eq!((term.x[2], term.z[2]), (false, false));
    assert_eq!((term.x[3], term.z[3]), (true, false));
}

#[test]
fn test_run_denominator_circuit_measures_x_on_ancilla() {
    let vd = VirtualDistillation::new(Circuit::new(1), 2).unwrap();

    let observed_values = vd
        .run_denominator_circuit(128, &|denominator, hamiltonian, shots| {
            let denominator_ops = denominator.operations();

            assert_eq!(denominator.width(), 3);
            assert_eq!(denominator_ops.len(), 2);
            assert!(matches!(
                denominator_ops[0].instruction,
                Instruction::Standard(StandardGate::H)
            ));
            assert!(matches!(
                denominator_ops[1].instruction,
                Instruction::McGate(_)
            ));
            assert_eq!(shots, Some(128));

            // The denominator observable Tr(rho^M) is X on the ancillary
            // qubit with identities everywhere else.
            let denominator_hamiltonian =
                hamiltonian.expect("denominator must provide a Hamiltonian");
            assert_eq!(denominator_hamiltonian.num_qubits, 3);
            assert_eq!(denominator_hamiltonian.terms.len(), 1);
            let (term, coeff) = &denominator_hamiltonian.terms[0];
            assert_eq!(*coeff, Complex64::new(1.0, 0.0));
            assert_eq!((term.x[0], term.z[0]), (false, false));
            assert_eq!((term.x[1], term.z[1]), (false, false));
            assert_eq!((term.x[2], term.z[2]), (true, false));

            (1.0, 0.5)
        })
        .unwrap();

    assert_eq!(observed_values, (1.0, 0.5));
}

#[test]
fn test_run_numerator_circuit_passes_hamiltonian_to_estimator() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.x(q0).unwrap();

    let vd = VirtualDistillation::new(circuit, 2).unwrap();
    let hamiltonian = single_qubit_x_hamiltonian();
    let observed_values = vd
        .run_numerator_circuit(&hamiltonian, 128, &|numerator, hamiltonian_arg, shots| {
            let numerator_ops = numerator.operations();

            assert_eq!(numerator.width(), 3);
            assert_eq!(numerator_ops.len(), 4);
            assert_eq!(shots, Some(128));

            let expanded_hamiltonian =
                hamiltonian_arg.expect("numerator must provide a Hamiltonian");
            assert_eq!(expanded_hamiltonian.num_qubits, 3);
            assert_eq!(expanded_hamiltonian.terms.len(), 1);

            // Observable on the first copy, identity on the extra copy, X on
            // the ancillary qubit.
            let (term, coeff) = &expanded_hamiltonian.terms[0];
            assert_eq!(*coeff, Complex64::new(1.0, 0.0));
            assert_eq!((term.x[0], term.z[0]), (true, false));
            assert_eq!((term.x[1], term.z[1]), (false, false));
            assert_eq!((term.x[2], term.z[2]), (true, false));

            (1.0, 0.25)
        })
        .unwrap();

    assert_eq!(observed_values, (1.0, 0.25));
}

#[test]
fn test_run_vd_returns_mu_and_var() {
    let base_circuit = Circuit::new(1);
    let vd = VirtualDistillation::new(base_circuit, 2).unwrap();
    let hamiltonian = single_qubit_z_hamiltonian();
    let (mu_vd, var_vd) = vd
        .run_vd(&hamiltonian, 3, 2, &|circuit, hamiltonian_arg, shots| {
            let ops = circuit.operations();

            assert_eq!(ops.len(), 2);
            assert!(matches!(
                ops[0].instruction,
                Instruction::Standard(StandardGate::H)
            ));
            assert!(matches!(ops[1].instruction, Instruction::McGate(_)));

            let expanded_hamiltonian =
                hamiltonian_arg.expect("numerator and denominator both provide a Hamiltonian");
            assert_eq!(expanded_hamiltonian.num_qubits, 3);
            assert_eq!(expanded_hamiltonian.terms.len(), 1);
            let (term, _coeff) = &expanded_hamiltonian.terms[0];
            // X on the ancillary qubit in both numerator and denominator.
            assert_eq!((term.x[2], term.z[2]), (true, false));

            if (term.x[0], term.z[0]) != (false, false) {
                // Numerator: the observable acts on the first copy.
                assert_eq!((term.x[0], term.z[0]), (false, true));
                assert_eq!(shots, Some(3));
                (1.5, 0.25)
            } else {
                // Denominator: identities on all copy qubits.
                assert_eq!((term.x[1], term.z[1]), (false, false));
                assert_eq!(shots, Some(2));
                (2.0, 1.0)
            }
        })
        .unwrap();

    assert!((mu_vd - 0.75).abs() < 1e-12);
    assert!((var_vd - 0.203125).abs() < 1e-12);
}

#[test]
fn test_run_vd_forwards_zero_samples_to_estimator() {
    let vd = VirtualDistillation::new(Circuit::new(1), 2).unwrap();
    let hamiltonian = single_qubit_z_hamiltonian();
    let (mu_vd, var_vd) = vd
        .run_vd(&hamiltonian, 0, 0, &|_circuit, hamiltonian_arg, shots| {
            assert_eq!(shots, Some(0));
            let term = &hamiltonian_arg.unwrap().terms[0].0;
            if (term.x[0], term.z[0]) != (false, false) {
                (1.0, 0.5)
            } else {
                (2.0, 1.0)
            }
        })
        .unwrap();

    assert!((mu_vd - 0.5).abs() < 1e-12);
    assert!((var_vd - 0.1875).abs() < 1e-12);
}

#[test]
fn test_run_vd_rejects_hamiltonian_qubit_mismatch() {
    let vd = VirtualDistillation::new(Circuit::new(1), 2).unwrap();

    let mut pauli_string = PauliString::new(2);
    pauli_string.set_pauli(0, Pauli::Z);
    let hamiltonian = Hamiltonian::from_list(vec![(pauli_string, Complex64::new(1.0, 0.0))])
        .expect("two-qubit mismatch Hamiltonian should be valid");

    let err = vd
        .run_vd(&hamiltonian, 2, 2, &|_circuit, _hamiltonian, _shots| {
            (0.0, 0.0)
        })
        .unwrap_err();

    assert!(matches!(
        err,
        ErrorMitigationError::HamiltonianQubitCountMismatch {
            expected: 1,
            actual: 2
        }
    ));
}

#[test]
fn test_run_vd_rejects_zero_denominator_mean() {
    let vd = VirtualDistillation::new(Circuit::new(1), 2).unwrap();
    let hamiltonian = single_qubit_z_hamiltonian();
    let err = vd
        .run_vd(&hamiltonian, 2, 2, &|_circuit, hamiltonian_arg, _shots| {
            let term = &hamiltonian_arg.unwrap().terms[0].0;
            if (term.x[0], term.z[0]) != (false, false) {
                (1.0, 0.0)
            } else {
                (0.0, 0.0)
            }
        })
        .unwrap_err();

    assert!(matches!(err, ErrorMitigationError::ZeroDenominatorMean));
}

#[test]
fn test_vd_exact_simulation_recovers_sign_on_excited_state() {
    // Preparing |1> and measuring Z: virtual distillation must return -1.
    let mut circuit = Circuit::new(1);
    circuit.x(Qubit::new(0)).unwrap();

    let vd = VirtualDistillation::new(circuit, 2).unwrap();
    let hamiltonian = single_qubit_z_hamiltonian();
    let (mu_vd, _) = vd.run_vd(&hamiltonian, 0, 0, &exact_estimator).unwrap();

    assert!((mu_vd + 1.0).abs() < 1e-12);
}

#[test]
fn test_vd_exact_simulation_on_rotated_pure_state() {
    // Pure states are invariant under distillation: VD returns the exact
    // expectation value <Z> = cos(theta).
    let theta = 0.7;
    let mut circuit = Circuit::new(1);
    circuit.ry(Qubit::new(0), theta).unwrap();

    for copies in [2, 3] {
        let vd = VirtualDistillation::new(circuit.clone(), copies).unwrap();
        let hamiltonian = single_qubit_z_hamiltonian();
        let (mu_vd, _) = vd.run_vd(&hamiltonian, 0, 0, &exact_estimator).unwrap();

        assert!((mu_vd - theta.cos()).abs() < 1e-12);
    }
}

#[test]
fn test_vd_mitigates_depolarizing_noise() {
    let p = 0.1;
    let mut circuit = Circuit::new(1);
    circuit.x(Qubit::new(0)).unwrap();

    let vd = VirtualDistillation::new(circuit, 2).unwrap();
    let hamiltonian = single_qubit_z_hamiltonian();
    let (mu_vd, _) = vd
        .run_vd(&hamiltonian, 0, 0, &depolarizing_estimator(p))
        .unwrap();

    // Depolarized single-copy state rho = a|1><1| + b|0><0|; VD with M = 2
    // estimates Tr(Z rho^2) / Tr(rho^2).
    let a = 1.0 - 2.0 * p / 3.0;
    let b = 2.0 * p / 3.0;
    let unmitigated = b - a;
    let expected_vd = (b * b - a * a) / (a * a + b * b);

    assert!((mu_vd - expected_vd).abs() < 1e-12);
    assert!((mu_vd + 1.0).abs() < (unmitigated + 1.0).abs());
}
