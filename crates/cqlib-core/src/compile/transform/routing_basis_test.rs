// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::LowerToRoutingBasis;
use crate::circuit::{Circuit, Instruction, MCGate, ParameterValue, Qubit, StandardGate};
use crate::compile::CompilerError;
use crate::compile::test_utils::{assert_compiled_circuit_equivalent, standard_ops};
use crate::compile::transform::TransformerTestExt;

fn assert_no_gate_like_operation_exceeds_two_qubits(circuit: &Circuit) {
    for operation in circuit.operations() {
        assert!(
            !matches!(
                operation.instruction,
                Instruction::Standard(_)
                    | Instruction::McGate(_)
                    | Instruction::UnitaryGate(_)
                    | Instruction::CircuitGate(_)
            ) || operation.qubits.len() <= 2,
            "operation should satisfy routing contract: {operation:?}"
        );
    }
}

fn ccx_circuit() -> Circuit {
    let mut circuit = Circuit::new(3);
    circuit
        .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();
    circuit
}

#[test]
fn routing_basis_lowers_ccx_to_two_qubit_operations() {
    let source = ccx_circuit();

    let result = LowerToRoutingBasis::default()
        .transform_resolved(&source, None)
        .unwrap();

    assert!(result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![
            StandardGate::H,
            StandardGate::CX,
            StandardGate::TDG,
            StandardGate::CX,
            StandardGate::T,
            StandardGate::CX,
            StandardGate::TDG,
            StandardGate::CX,
            StandardGate::T,
            StandardGate::T,
            StandardGate::H,
            StandardGate::CX,
            StandardGate::T,
            StandardGate::TDG,
            StandardGate::CX,
        ]
    );
    assert_compiled_circuit_equivalent(&result.circuit, &source);
    assert_no_gate_like_operation_exceeds_two_qubits(&result.circuit);
}

#[test]
fn routing_basis_lowers_trivial_mcx_to_two_qubit_operations() {
    let mut source = Circuit::new(3);
    source
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
            vec![Qubit::new(0), Qubit::new(1), Qubit::new(2)],
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();

    let result = LowerToRoutingBasis::default()
        .transform_resolved(&source, None)
        .unwrap();

    assert!(result.changed);
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::CCX));
    assert_no_gate_like_operation_exceeds_two_qubits(&result.circuit);
}

#[test]
fn routing_basis_prefers_cz_for_cz_only_basis() {
    let source = ccx_circuit();
    let transform = LowerToRoutingBasis::new(Some(vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::T),
        Instruction::Standard(StandardGate::TDG),
        Instruction::Standard(StandardGate::CZ),
        Instruction::Standard(StandardGate::GPhase),
    ]));

    let result = transform.transform_resolved(&source, None).unwrap();
    let gates = standard_ops(&result.circuit);

    assert!(!gates.contains(&StandardGate::CCX));
    assert!(!gates.contains(&StandardGate::CX));
    assert!(gates.contains(&StandardGate::CZ));
    assert_no_gate_like_operation_exceeds_two_qubits(&result.circuit);
}

#[test]
fn routing_basis_preserves_existing_two_qubit_standard_gates() {
    let mut source = Circuit::new(2);
    source.rzz(Qubit::new(0), Qubit::new(1), 0.37).unwrap();
    source.crz(Qubit::new(0), Qubit::new(1), 0.19).unwrap();
    source
        .fsim(Qubit::new(0), Qubit::new(1), 0.11, -0.23)
        .unwrap();

    let result = LowerToRoutingBasis::default()
        .transform_resolved(&source, None)
        .unwrap();

    assert!(!result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::RZZ, StandardGate::CRZ, StandardGate::FSIM]
    );
}

#[test]
fn routing_basis_fast_path_does_not_run_optimization_rewrites() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut source = Circuit::new(2);
    source.x(q0).unwrap();
    source.x(q0).unwrap();
    source.cx(q0, q1).unwrap();

    let result = LowerToRoutingBasis::default()
        .transform_resolved(&source, None)
        .unwrap();

    assert!(!result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::X, StandardGate::X, StandardGate::CX]
    );
}

#[test]
fn routing_basis_validation_reports_route_sabre_contract() {
    let mut source = Circuit::new(3);
    let gate = crate::circuit::UnitaryGate::new("THREE_Q", 3, 0);
    source
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1), Qubit::new(2)])
        .unwrap();

    let err = LowerToRoutingBasis::default()
        .transform_resolved(&source, None)
        .unwrap_err();

    assert!(matches!(
        err,
        CompilerError::InvalidInput(reason)
            if reason.contains("routing-basis lowering did not satisfy route.sabre input contract")
                && reason.contains("3-qubit operation THREE_Q")
    ));
}
