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
