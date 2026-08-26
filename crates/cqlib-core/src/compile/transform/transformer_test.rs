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
use crate::circuit::{ClassicalExpr, Instruction, Qubit, StandardGate};
use crate::compile::transform::decompose::{
    DecomposeDefinitions, DecomposeMcGates, DecomposeUnitaries,
};
use crate::compile::transform::{LowerToRoutingBasis, TargetBasisLowerer};

fn assert_proven_noop<P: WorkflowPass + ?Sized>(pass: &P, analysis: &WorkflowCircuitAnalysis) {
    assert!(matches!(
        pass.applicability(analysis),
        PassApplicability::ProvenNoOp(_)
    ));
}

#[test]
fn outcome_reports_change_and_resolves_owned_circuit() {
    let original = Circuit::new(1);
    assert!(!TransformOutcome::Unchanged.changed());
    assert_eq!(
        TransformOutcome::Unchanged.into_circuit(&original),
        original
    );

    let changed = Circuit::new(2);
    let outcome = TransformOutcome::Changed(changed.clone());
    assert!(outcome.changed());
    assert_eq!(outcome.into_circuit(&original), changed);
}

#[test]
fn declared_proven_noop_passes_match_transform_contract() {
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.measure(Qubit::new(0)).unwrap();
    circuit.reset(Qubit::new(0)).unwrap();
    circuit
        .while_(ClassicalExpr::bool_literal(false), |body| {
            body.h(Qubit::new(0))
        })
        .unwrap();
    let analysis = WorkflowCircuitAnalysis::analyze(&circuit);
    let target_lowerer =
        TargetBasisLowerer::new(vec![Instruction::Standard(StandardGate::H)]).unwrap();
    let definitions = DecomposeDefinitions;
    let unitaries = DecomposeUnitaries::default();
    let mc_gates = DecomposeMcGates::default();
    let routing_basis = LowerToRoutingBasis::default();
    let passes: [(&dyn WorkflowPass, &dyn Transformer); 5] = [
        (&definitions, &definitions),
        (&unitaries, &unitaries),
        (&mc_gates, &mc_gates),
        (&routing_basis, &routing_basis),
        (&target_lowerer, &target_lowerer),
    ];

    for (workflow_pass, transformer) in passes {
        assert_proven_noop(workflow_pass, &analysis);
        assert_eq!(
            transformer
                .transform(&circuit, Some(analysis.public()))
                .unwrap(),
            TransformOutcome::Unchanged,
            "{} violated its declared no-op proof",
            transformer.name()
        );
    }
}
