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
use crate::circuit::{ClassicalControlOp, ClassicalExpr, Instruction, Qubit, StandardGate};
use crate::compile::test_utils::assert_compiled_circuit_equivalent;
use crate::compile::transform::{OperationReplacement, RewriteEdits, TransformerTestExt};
use std::sync::Arc;

#[test]
fn basis_optimizer_reuses_supplied_cost_model() {
    let cost_model = Arc::new(
        TargetBasisCostModel::new(vec![
            Instruction::Standard(StandardGate::RZ),
            Instruction::Standard(StandardGate::X2P),
        ])
        .unwrap(),
    );

    let optimizer = OptimizeOneQubitRuns::basis_with_cost_model(Arc::clone(&cost_model));

    let LocalOptimizationPolicy::Basis(actual) = &optimizer.policy else {
        panic!("expected a target-basis optimization policy");
    };
    assert!(Arc::ptr_eq(actual, &cost_model));
}

#[test]
fn logical_fuses_numeric_run_to_one_u() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rx(q0, 0.2).unwrap();
    circuit.ry(q0, -0.4).unwrap();
    circuit.rz(q0, 0.7).unwrap();

    let first = OptimizeOneQubitRuns::logical()
        .transform_resolved(&circuit, None)
        .unwrap();
    let second = OptimizeOneQubitRuns::logical()
        .transform_resolved(&first.circuit, None)
        .unwrap();

    assert!(first.changed);
    assert_eq!(first.circuit.operations().len(), 1);
    assert!(matches!(
        first.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::U)
    ));
    assert!(!second.changed);
    assert_compiled_circuit_equivalent(&first.circuit, &circuit);
}

#[test]
fn logical_reports_sparse_edits_for_fusion_across_disjoint_operation() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.rx(q0, 0.2).unwrap();
    circuit.h(q1).unwrap();
    circuit.rz(q0, -0.3).unwrap();

    let (outcome, edits) = OptimizeOneQubitRuns::logical()
        .transform_with_rewrite_edits(&circuit)
        .unwrap();
    let TransformOutcome::Changed(optimized) = outcome else {
        panic!("expected one-qubit fusion");
    };

    assert_eq!(optimized.operations().len(), 2);
    assert_eq!(
        edits,
        RewriteEdits::linear(
            3,
            2,
            vec![
                OperationReplacement {
                    old: 0..1,
                    new: 0..1,
                },
                OperationReplacement {
                    old: 2..3,
                    new: 2..2,
                },
            ],
        )
    );
    assert_compiled_circuit_equivalent(&optimized, &circuit);
}

#[test]
fn unchanged_optimizer_reports_empty_exact_edits() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            Instruction::Standard(StandardGate::H),
            [q0],
            std::iter::empty(),
            Some("keep"),
        )
        .unwrap();

    let (outcome, edits) = OptimizeOneQubitRuns::logical()
        .transform_with_rewrite_edits(&circuit)
        .unwrap();

    assert!(matches!(outcome, TransformOutcome::Unchanged));
    assert_eq!(edits, RewriteEdits::linear(1, 1, Vec::new()));
}

#[test]
fn logical_run_crosses_disjoint_qubit_operation() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.rx(q0, 0.2).unwrap();
    circuit.h(q1).unwrap();
    circuit.rz(q0, -0.3).unwrap();

    let result = OptimizeOneQubitRuns::logical()
        .transform_resolved(&circuit, None)
        .unwrap();

    assert!(result.changed);
    assert_eq!(result.circuit.operations().len(), 2);
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn basis_uses_lowered_cost_and_keeps_exact_semantics() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();
    circuit.h(q0).unwrap();
    let optimizer = OptimizeOneQubitRuns::basis(vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::U),
    ])
    .unwrap();

    let result = optimizer.transform_resolved(&circuit, None).unwrap();

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn labeled_gate_is_a_boundary() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rx(q0, 0.2).unwrap();
    circuit
        .append(
            Instruction::Standard(StandardGate::H),
            [q0],
            std::iter::empty(),
            Some("keep"),
        )
        .unwrap();
    circuit.rz(q0, -0.3).unwrap();

    let result = OptimizeOneQubitRuns::logical()
        .transform_resolved(&circuit, None)
        .unwrap();

    assert!(!result.changed);
    assert_eq!(result.circuit, circuit);
}

#[test]
fn optimizer_recurses_into_control_flow_bodies() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.rx(q0, 0.2)?;
            body.ry(q0, -0.4)?;
            Ok(())
        })
        .unwrap();

    let result = OptimizeOneQubitRuns::logical()
        .transform_resolved(&circuit, None)
        .unwrap();

    assert!(result.changed);
    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected an if operation");
    };
    let body = if_op.then_body().operations();
    assert_eq!(
        body.iter()
            .filter(|operation| operation.qubits.len() == 1)
            .count(),
        1
    );
    assert!(body.iter().any(|operation| matches!(
        operation.instruction,
        Instruction::Standard(StandardGate::U)
    )));
}

#[test]
fn changed_control_flow_body_invalidates_only_its_top_level_operation() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.rx(q0, 0.2)?;
            body.ry(q0, -0.4)?;
            Ok(())
        })
        .unwrap();
    circuit.h(q1).unwrap();

    let (outcome, edits) = OptimizeOneQubitRuns::logical()
        .transform_with_rewrite_edits(&circuit)
        .unwrap();

    assert!(matches!(outcome, TransformOutcome::Changed(_)));
    assert_eq!(
        edits,
        RewriteEdits::linear(
            2,
            2,
            vec![OperationReplacement {
                old: 0..1,
                new: 0..1,
            }],
        )
    );
}

#[test]
fn measurement_flushes_or_discards_pending_frame_safely() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rz(q0, 0.4).unwrap();
    circuit.measure_bits([q0]).unwrap();

    let result = OptimizeOneQubitRuns::logical()
        .transform_resolved(&circuit, None)
        .unwrap();

    assert!(result.changed);
    assert_eq!(result.circuit.operations().len(), 1);
    assert!(result.circuit.operations()[0].instruction.has_measurement());
}
