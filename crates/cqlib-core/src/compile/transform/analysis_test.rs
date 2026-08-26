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

use super::{CircuitAnalysis, InstructionKind, WorkflowCircuitAnalysis};
use crate::circuit::{
    Circuit, ClassicalExpr, Directive, Instruction, ParameterValue, Qubit, StandardGate,
};

#[test]
fn analysis_detects_runtime_classical_and_definitions_recursively() {
    let mut inner = Circuit::new(1);
    let measured = inner.measure(Qubit::new(0)).unwrap();
    inner
        .if_(measured.expr().to_bool().unwrap(), |body| {
            body.x(Qubit::new(0))?;
            Ok(())
        })
        .unwrap();
    let gate = inner.to_gate("measured").unwrap();

    let mut circuit = Circuit::new(1);
    let value = circuit.measure(Qubit::new(0)).unwrap();
    circuit
        .if_(ClassicalExpr::bit_to_bool(value.expr()).unwrap(), |body| {
            body.append(gate.clone(), [Qubit::new(0)], [], None)?;
            Ok(())
        })
        .unwrap();

    let analysis = CircuitAnalysis::analyze(&circuit);
    assert!(analysis.has_measurement);
    assert!(analysis.has_classical_data);
    assert!(analysis.has_classical_control);
    assert!(analysis.has_runtime_classical);
    assert!(analysis.needs_classical_handle_preservation);
    assert!(analysis.has_circuit_gate_definitions);
    assert!(!analysis.has_unitary_circuit_definitions);
    assert!(!analysis.has_mc_gates);
}

#[test]
fn workflow_analysis_detects_legacy_measure_reset_and_nested_gate_kinds() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::Directive(Directive::Measure),
            [q0],
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();
    circuit
        .append(
            Instruction::Directive(Directive::Reset),
            [q1],
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.ccx(q0, q1, q2)
        })
        .unwrap();

    let analysis = WorkflowCircuitAnalysis::analyze(&circuit);
    assert!(analysis.public().has_measurement);
    assert!(analysis.has_reset);
    assert!(analysis.has_control_flow);
    assert!(analysis.has_gate_like_operation_over_two_qubits());
    assert_eq!(
        analysis.standard_gates().collect::<Vec<_>>(),
        vec![StandardGate::CCX]
    );
    assert!(
        analysis
            .instruction_kinds
            .contains(InstructionKind::Directive)
    );
    assert!(
        analysis
            .instruction_kinds
            .contains(InstructionKind::ClassicalControl)
    );
}

#[test]
fn workflow_analysis_distinguishes_direction_sensitive_two_qubit_gates() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    let mut symmetric = Circuit::new(2);
    symmetric.cz(q0, q1).unwrap();
    assert!(
        !WorkflowCircuitAnalysis::analyze(&symmetric).has_direction_sensitive_two_qubit_operation
    );

    let mut direction_sensitive = Circuit::new(2);
    direction_sensitive.cx(q0, q1).unwrap();
    assert!(
        WorkflowCircuitAnalysis::analyze(&direction_sensitive)
            .has_direction_sensitive_two_qubit_operation
    );
}
