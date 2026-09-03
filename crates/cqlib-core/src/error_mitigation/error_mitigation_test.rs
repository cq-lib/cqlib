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

use super::{
    ErrorMitigation, ErrorMitigationError, ExtrapolateMethod, MitigatedResult, MitigationMethod,
    ProcessArgs, RunArgs, VirtualDistillationConfig, ZneConfig,
};
use crate::circuit::Qubit;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::gate::{FrozenCircuit, Instruction, StandardGate, UnitaryGate};
use crate::qis::{Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;
use std::sync::Arc;

#[test]
fn run_args_preserves_eq_and_stable_gate_set_comparison() {
    fn assert_eq_trait<T: Eq>() {}

    let left = Instruction::UnitaryGate(Box::new(UnitaryGate::new("custom", 1, 0)));
    let right = Instruction::UnitaryGate(Box::new(UnitaryGate::new("custom", 1, 0)));

    assert_eq_trait::<RunArgs>();
    assert_eq!(
        RunArgs::Zne {
            gate_set: Some(vec![left]),
            shots: Some(100),
        },
        RunArgs::Zne {
            gate_set: Some(vec![right]),
            shots: Some(100),
        }
    );

    assert_ne!(
        RunArgs::Zne {
            gate_set: Some(vec![Instruction::UnitaryGate(Box::new(UnitaryGate::new(
                "custom", 1, 0,
            )))]),
            shots: Some(100),
        },
        RunArgs::Zne {
            gate_set: Some(vec![Instruction::UnitaryGate(Box::new(UnitaryGate::new(
                "other", 1, 0,
            )))]),
            shots: Some(100),
        }
    );
}

#[test]
fn zne_selective_folding_uses_stable_custom_gate_names() {
    let mut definition = Circuit::new(1);
    definition.h(Qubit::new(0)).unwrap();
    let applied = UnitaryGate::new("selected", 1, 0)
        .with_circuit(Arc::new(FrozenCircuit::new(definition)))
        .unwrap();
    let mut circuit = Circuit::new(1);
    circuit.unitary(applied, vec![Qubit::new(0)]).unwrap();
    let mitigation = super::ZNEMitigation::new(circuit, vec![1]);

    let same_name = Instruction::UnitaryGate(Box::new(UnitaryGate::new("selected", 1, 0)));
    let different_name = Instruction::UnitaryGate(Box::new(UnitaryGate::new("other", 1, 0)));

    let selected = mitigation.fold_circuits(Some(&[same_name])).unwrap();
    let skipped = mitigation.fold_circuits(Some(&[different_name])).unwrap();
    assert_eq!(selected[0].operations().len(), 3);
    assert_eq!(skipped[0].operations().len(), 1);
}

fn single_qubit_z_hamiltonian() -> Hamiltonian {
    let mut pauli_string = PauliString::new(1);
    pauli_string.set_pauli(0, Pauli::Z);
    Hamiltonian::from_list(vec![(pauli_string, Complex64::new(1.0, 0.0))])
        .expect("single-qubit Z Hamiltonian should be valid")
}

#[test]
fn test_error_mitigation_new_validates_supported_methods() {
    let zne = ErrorMitigation::new(
        Circuit::new(1),
        MitigationMethod::Zne(ZneConfig {
            fold_levels: vec![0, 1, 2],
        }),
    );
    assert!(zne.is_ok());

    let vd = ErrorMitigation::new(
        Circuit::new(1),
        MitigationMethod::VirtualDistillation(VirtualDistillationConfig { copies: 2 }),
    );
    assert!(vd.is_ok());

    let invalid_zne = ErrorMitigation::new(
        Circuit::new(1),
        MitigationMethod::Zne(ZneConfig {
            fold_levels: vec![0, -1],
        }),
    )
    .unwrap_err();
    assert!(matches!(
        invalid_zne,
        ErrorMitigationError::InvalidFoldLevel(-1)
    ));
}

#[test]
fn test_error_mitigation_requires_run_before_get_mitigated() {
    let mut mitigation = ErrorMitigation::new(
        Circuit::new(1),
        MitigationMethod::Zne(ZneConfig {
            fold_levels: vec![0, 1],
        }),
    )
    .unwrap();

    let err = mitigation
        .get_mitigated(ProcessArgs::Zne {
            method: ExtrapolateMethod::Polynomial,
            degree: Some(1),
        })
        .unwrap_err();

    assert!(matches!(
        err,
        ErrorMitigationError::RunRequiredBeforeMitigation
    ));
}

#[test]
fn test_error_mitigation_rejects_mismatched_run_and_process_args() {
    let hamiltonian = single_qubit_z_hamiltonian();
    let mut mitigation = ErrorMitigation::new(
        Circuit::new(1),
        MitigationMethod::Zne(ZneConfig {
            fold_levels: vec![0, 1],
        }),
    )
    .unwrap();

    let run_err = mitigation
        .run(
            &hamiltonian,
            RunArgs::VirtualDistillation {
                shots_numerator: 2,
                shots_denominator: 2,
            },
            &|_circuit, _hamiltonian, _shots| (0.0, 0.0),
        )
        .unwrap_err();
    assert!(matches!(
        run_err,
        ErrorMitigationError::RunArgsMethodMismatch
    ));

    mitigation
        .run(
            &hamiltonian,
            RunArgs::Zne {
                gate_set: Some(vec![Instruction::Standard(StandardGate::X)]),
                shots: Some(32),
            },
            &|circuit, hamiltonian_arg, shots| {
                assert!(hamiltonian_arg.is_some());
                assert_eq!(shots, Some(32));
                (circuit.operations().len() as f64, 0.0)
            },
        )
        .unwrap();

    let process_err = mitigation
        .get_mitigated(ProcessArgs::VirtualDistillation)
        .unwrap_err();
    assert!(matches!(
        process_err,
        ErrorMitigationError::ProcessArgsMethodMismatch
    ));
}

#[test]
fn test_error_mitigation_zne_run_then_get_mitigated() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.x(q0).unwrap();

    let hamiltonian = single_qubit_z_hamiltonian();
    let mut mitigation = ErrorMitigation::new(
        circuit,
        MitigationMethod::Zne(ZneConfig {
            fold_levels: vec![0, 1, 2],
        }),
    )
    .unwrap();

    mitigation
        .run(
            &hamiltonian,
            RunArgs::Zne {
                gate_set: None,
                shots: Some(256),
            },
            &|circuit, hamiltonian_arg, shots| {
                assert!(hamiltonian_arg.is_some());
                assert_eq!(shots, Some(256));
                (circuit.operations().len() as f64 + 0.5, 0.0)
            },
        )
        .unwrap();

    let rerun_err = mitigation
        .run(
            &hamiltonian,
            RunArgs::Zne {
                gate_set: None,
                shots: Some(256),
            },
            &|_circuit, _hamiltonian, _shots| (0.0, 0.0),
        )
        .unwrap_err();
    assert!(matches!(rerun_err, ErrorMitigationError::AlreadyRun));

    let mitigated = mitigation
        .get_mitigated(ProcessArgs::Zne {
            method: ExtrapolateMethod::Polynomial,
            degree: None,
        })
        .unwrap();

    assert_eq!(
        mitigated,
        MitigatedResult {
            expectation: 0.5,
            variance: None,
        }
    );

    let second_err = mitigation
        .get_mitigated(ProcessArgs::Zne {
            method: ExtrapolateMethod::Polynomial,
            degree: Some(1),
        })
        .unwrap_err();
    assert!(matches!(second_err, ErrorMitigationError::AlreadyMitigated));
}

#[test]
fn test_error_mitigation_vd_run_then_get_mitigated() {
    let hamiltonian = single_qubit_z_hamiltonian();
    let mut mitigation = ErrorMitigation::new(
        Circuit::new(1),
        MitigationMethod::VirtualDistillation(VirtualDistillationConfig { copies: 2 }),
    )
    .unwrap();

    mitigation
        .run(
            &hamiltonian,
            RunArgs::VirtualDistillation {
                shots_numerator: 3,
                shots_denominator: 2,
            },
            &|circuit, hamiltonian_arg, shots| {
                let ops = circuit.operations();
                assert_eq!(ops.len(), 1);
                assert!(matches!(
                    ops[0].instruction,
                    Instruction::Standard(StandardGate::SWAP)
                ));

                if let Some(expanded_hamiltonian) = hamiltonian_arg {
                    assert_eq!(expanded_hamiltonian.num_qubits, 2);
                    assert_eq!(shots, Some(3));
                    (1.5, 0.25)
                } else {
                    assert_eq!(shots, Some(2));
                    (2.0, 1.0)
                }
            },
        )
        .unwrap();

    let mitigated = mitigation
        .get_mitigated(ProcessArgs::VirtualDistillation)
        .unwrap();

    assert!((mitigated.expectation - 0.75).abs() < 1e-12);
    assert!((mitigated.variance.unwrap() - 0.203125).abs() < 1e-12);
}
