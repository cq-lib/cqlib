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

use super::two_qubit_kak::kak_decompose;
use super::unitary_2q::{
    NumericTwoQubitCandidateValidation, TwoQubitSynthesisRequest, TwoQubitSynthesisTarget,
    TwoQubitUnitaryDecomposeBasis, check_numeric_2q_candidate, plan_numeric_2q_unitary_from_kak,
    select_validated_numeric_2q_candidate_from_kak, validate_numeric_2q_candidate,
};
use crate::circuit::{Instruction, Qubit, StandardGate, ValueInstruction};

#[test]
fn weyl_boundary_all_backends_preserve_phase_and_qubit_order() {
    use super::unitary_2q::{TwoQubitSynthesisCandidate, synthesize_numeric_2q_unitary};
    use crate::compile::test_utils::kak_boundary_matrix;
    use num_complex::Complex64;
    for phase in [0.0, 0.37] {
        let matrix = kak_boundary_matrix().mapv(|value| value * Complex64::from_polar(1.0, phase));
        for qubits in [
            [Qubit::new(0), Qubit::new(1)],
            [Qubit::new(3), Qubit::new(1)],
        ] {
            for backend in [
                TwoQubitUnitaryDecomposeBasis::Cx,
                TwoQubitUnitaryDecomposeBasis::Cy,
                TwoQubitUnitaryDecomposeBasis::Cz,
                TwoQubitUnitaryDecomposeBasis::Rzz,
                TwoQubitUnitaryDecomposeBasis::PauliRotations,
            ] {
                let result = synthesize_numeric_2q_unitary(&matrix, qubits, backend).unwrap();
                let candidate = TwoQubitSynthesisCandidate {
                    backend,
                    operations: result.operations,
                    global_phase: result.global_phase,
                    cost: Default::default(),
                };
                validate_numeric_2q_candidate(&matrix, qubits, &candidate).unwrap();
                assert!(
                    candidate
                        .operations
                        .iter()
                        .all(|op| op.qubits.iter().all(|q| qubits.contains(q)))
                );
                assert!(
                    candidate
                        .operations
                        .iter()
                        .filter(|op| op.qubits.len() == 2)
                        .all(|op| {
                            let ValueInstruction::Instruction(Instruction::Standard(gate)) =
                                op.instruction
                            else {
                                return false;
                            };
                            match backend {
                                TwoQubitUnitaryDecomposeBasis::Cx => gate == StandardGate::CX,
                                TwoQubitUnitaryDecomposeBasis::Cy => gate == StandardGate::CY,
                                TwoQubitUnitaryDecomposeBasis::Cz => gate == StandardGate::CZ,
                                TwoQubitUnitaryDecomposeBasis::Rzz => gate == StandardGate::RZZ,
                                TwoQubitUnitaryDecomposeBasis::PauliRotations => matches!(
                                    gate,
                                    StandardGate::RXX | StandardGate::RYY | StandardGate::RZZ
                                ),
                            }
                        })
                );
            }
        }
    }
}

fn count_cx(candidate: &super::unitary_2q::TwoQubitSynthesisCandidate) -> usize {
    candidate
        .operations
        .iter()
        .filter(|operation| {
            matches!(
                operation.instruction,
                ValueInstruction::Instruction(Instruction::Standard(StandardGate::CX))
            )
        })
        .count()
}

#[test]
fn selected_boundary_inexact_candidate_falls_back_to_higher_cx_template() {
    let matrix = StandardGate::RXX.matrix(&[0.23]).unwrap().into_owned();
    let qubits = [Qubit::new(0), Qubit::new(1)];
    let target = TwoQubitSynthesisTarget::from_standard_gates(
        vec![StandardGate::U],
        vec![StandardGate::CX],
        false,
    )
    .unwrap();
    let decomp = kak_decompose(&matrix).unwrap();
    let mut minimum = plan_numeric_2q_unitary_from_kak(
        TwoQubitSynthesisRequest {
            matrix: &matrix,
            qubits,
            target: target.clone(),
        },
        &decomp,
    )
    .unwrap()
    .into_iter()
    .find(|candidate| candidate.backend == TwoQubitUnitaryDecomposeBasis::Cx)
    .unwrap();
    assert_eq!(count_cx(&minimum), 2);

    minimum.global_phase += 5.0e-10;
    let NumericTwoQubitCandidateValidation::Inexact { max_error } =
        check_numeric_2q_candidate(&matrix, qubits, &minimum).unwrap()
    else {
        panic!("phase-perturbed analytic template must be numerically inexact");
    };
    assert!((1.0e-10..1.0e-9).contains(&max_error));

    let selected = select_validated_numeric_2q_candidate_from_kak(
        &matrix,
        qubits,
        &target,
        &decomp,
        vec![minimum],
    )
    .unwrap()
    .expect("the three-CX fallback must certify");
    assert_eq!(count_cx(&selected), 3);
    validate_numeric_2q_candidate(&matrix, qubits, &selected).unwrap();
}

#[test]
fn local_cleanup_scoring_agrees_for_primary_cx_and_pauli_fallbacks() {
    use super::unitary_2q::{
        plan_cx_family_fallbacks_from_kak, plan_pauli_fallback_from_kak,
        target_aware_cost_of_value_operations,
    };
    let matrix = StandardGate::RXX.matrix(&[0.23]).unwrap().into_owned();
    let qubits = [Qubit::new(1), Qubit::new(0)];
    let target = TwoQubitSynthesisTarget::from_standard_gates(
        vec![StandardGate::RZ, StandardGate::X2P],
        vec![StandardGate::CX],
        true,
    )
    .unwrap();
    let decomp = kak_decompose(&matrix).unwrap();
    let request = || TwoQubitSynthesisRequest {
        matrix: &matrix,
        qubits,
        target: target.clone(),
    };
    for candidates in [
        plan_numeric_2q_unitary_from_kak(request(), &decomp).unwrap(),
        plan_cx_family_fallbacks_from_kak(request(), &decomp, TwoQubitUnitaryDecomposeBasis::Cx)
            .unwrap(),
        plan_pauli_fallback_from_kak(request(), &decomp).unwrap(),
    ] {
        assert!(!candidates.is_empty());
        for candidate in candidates {
            validate_numeric_2q_candidate(&matrix, qubits, &candidate).unwrap();
            let model = target.lowering_cost_model().unwrap();
            let cost = model
                .cost_after_local_cleanup(qubits.to_vec(), candidate.operations.clone())
                .unwrap();
            assert_eq!(candidate.cost.lowered_two_qubit_ops, cost.two_qubit_ops);
            assert_eq!(candidate.cost.lowered_depth, cost.depth);
            assert_eq!(candidate.cost.lowered_total_ops, cost.total_ops);
            assert_eq!(candidate.cost.parameterized_ops, cost.parameterized_ops);
            assert_eq!(
                candidate.cost,
                target_aware_cost_of_value_operations(
                    &candidate.operations,
                    &target,
                    candidate.backend,
                )
                .unwrap()
            );
        }
    }
}
