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
use crate::circuit::Qubit;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::gate::Instruction;
use crate::circuit::gate::standard_gate::StandardGate;
use crate::error_mitigation::ErrorMitigationError;
use crate::qis::{Hamiltonian, Pauli, PauliString};
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

    assert_eq!(copy_swap.width(), 2);
    assert_eq!(ops.len(), 3);

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
        Instruction::Standard(StandardGate::SWAP)
    ));
    assert_eq!(ops[2].qubits.as_slice(), &[Qubit::new(0), Qubit::new(1)]);
}

#[test]
fn test_build_copy_swap_circuit_adds_pairwise_swaps_for_multiple_copies() {
    let vd = VirtualDistillation::new(Circuit::new(1), 3).unwrap();
    let copy_swap = vd.build_copy_swap_circuit().unwrap();
    let ops = copy_swap.operations();

    assert_eq!(copy_swap.width(), 3);
    assert_eq!(ops.len(), 2);

    let swap_count = ops
        .iter()
        .filter(|op| matches!(op.instruction, Instruction::Standard(StandardGate::SWAP)))
        .count();
    assert_eq!(swap_count, 2);
}

#[test]
fn test_expand_hamiltonian_appends_z_on_higher_indices() {
    let hamiltonian = single_qubit_x_hamiltonian();
    let expanded = VirtualDistillation::expand_hamiltonian(&hamiltonian, 2).unwrap();

    assert_eq!(expanded.num_qubits, 3);
    assert_eq!(expanded.terms.len(), 1);

    let (term, coeff) = &expanded.terms[0];
    assert_eq!(*coeff, Complex64::new(1.0, 0.0));
    assert_eq!(term.num_qubits, 3);
    assert_eq!(term.phase, crate::qis::Phase::Plus);

    assert_eq!((term.x[0], term.z[0]), (true, false));
    assert_eq!((term.x[1], term.z[1]), (false, true));
    assert_eq!((term.x[2], term.z[2]), (false, true));
}

#[test]
fn test_run_denominator_circuit_runs_copy_swap_circuit() {
    let vd = VirtualDistillation::new(Circuit::new(1), 2).unwrap();

    let observed_values = vd
        .run_denominator_circuit(128, &|denominator, hamiltonian, shots| {
            let denominator_ops = denominator.operations();

            assert_eq!(denominator.width(), 2);
            assert_eq!(denominator_ops.len(), 1);
            assert!(matches!(
                denominator_ops[0].instruction,
                Instruction::Standard(StandardGate::SWAP)
            ));
            assert!(hamiltonian.is_none());
            assert_eq!(shots, Some(128));

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

            assert_eq!(numerator.width(), 2);
            assert_eq!(numerator_ops.len(), 3);
            assert!(hamiltonian_arg.is_some());
            assert_eq!(shots, Some(128));

            let expanded_hamiltonian = hamiltonian_arg.unwrap();
            assert_eq!(expanded_hamiltonian.num_qubits, 2);
            assert_eq!(expanded_hamiltonian.terms.len(), 1);

            let (term, coeff) = &expanded_hamiltonian.terms[0];
            assert_eq!(*coeff, Complex64::new(1.0, 0.0));
            assert_eq!((term.x[0], term.z[0]), (true, false));
            assert_eq!((term.x[1], term.z[1]), (false, true));

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

            assert_eq!(ops.len(), 1);
            assert!(matches!(
                ops[0].instruction,
                Instruction::Standard(StandardGate::SWAP)
            ));

            if let Some(expanded_hamiltonian) = hamiltonian_arg {
                assert_eq!(expanded_hamiltonian.num_qubits, 2);
                assert_eq!(expanded_hamiltonian.terms.len(), 1);
                let (term, _coeff) = &expanded_hamiltonian.terms[0];
                assert_eq!((term.x[0], term.z[0]), (false, true));
                assert_eq!((term.x[1], term.z[1]), (false, true));
                assert_eq!(shots, Some(3));
                (1.5, 0.25)
            } else {
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
            if hamiltonian_arg.is_some() {
                assert_eq!(shots, Some(0));
                (1.0, 0.5)
            } else {
                assert_eq!(shots, Some(0));
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
            if hamiltonian_arg.is_some() {
                (1.0, 0.0)
            } else {
                (0.0, 0.0)
            }
        })
        .unwrap_err();

    assert!(matches!(err, ErrorMitigationError::ZeroDenominatorMean));
}
