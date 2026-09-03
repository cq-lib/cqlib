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

use super::{TargetBasisCostModel, TargetBasisLowerer};
use crate::circuit::gate::FrozenCircuit;
use crate::circuit::{
    Circuit, CircuitGate, ClassicalControlOp, ClassicalExpr, ClassicalType, Instruction, MCGate,
    Operation, Parameter, ParameterValue, Qubit, StandardGate, UnitaryGate, ValueOperation,
};
use crate::compile::CompilerError;
use crate::compile::knowledge::KnowledgeInstructionKey;
use crate::compile::test_utils::{assert_compiled_circuit_equivalent, standard_ops};
use crate::compile::transform::TransformerTestExt;
use std::sync::Arc;

fn target_basis(gates: &[StandardGate]) -> Vec<Instruction> {
    gates.iter().copied().map(Instruction::Standard).collect()
}

fn run_target_lowering(circuit: &Circuit, basis: &[StandardGate]) -> Circuit {
    TargetBasisLowerer::new(target_basis(basis))
        .unwrap()
        .transform_resolved(circuit, None)
        .unwrap()
        .circuit
}

fn assert_only_target_standard_gates(circuit: &Circuit, basis: &[StandardGate]) {
    for operation in circuit.operations() {
        match operation.instruction {
            Instruction::Standard(gate) => assert!(
                basis.contains(&gate),
                "gate {gate} is outside target basis {basis:?}"
            ),
            ref instruction => panic!("unexpected non-standard instruction {instruction}"),
        }
    }
}

fn assert_target_lowering_fails_with(
    circuit: &Circuit,
    basis: &[StandardGate],
    expected_snippets: &[&str],
) {
    let err = TargetBasisLowerer::new(target_basis(basis))
        .unwrap()
        .transform_resolved(circuit, None)
        .unwrap_err();
    let CompilerError::InvalidInput(message) = err else {
        panic!("expected invalid input error, got {err:?}");
    };
    for snippet in expected_snippets {
        assert!(
            message.contains(snippet),
            "expected error message {message:?} to contain {snippet:?}"
        );
    }
}

#[test]
fn lowerer_clones_share_basis_and_planning_graph() {
    let instructions: Arc<[Instruction]> =
        target_basis(&[StandardGate::RZ, StandardGate::X2P, StandardGate::CZ]).into();
    let lowerer = TargetBasisLowerer::from_shared_basis(Arc::clone(&instructions)).unwrap();
    let cloned = lowerer.clone();

    assert!(Arc::ptr_eq(&lowerer.target_basis, &instructions));
    assert!(Arc::ptr_eq(&lowerer.target_basis, &cloned.target_basis));
    assert!(Arc::ptr_eq(&lowerer.plans, &cloned.plans));
}

#[test]
fn cost_model_reuses_supplied_lowerer() {
    let lowerer = Arc::new(
        TargetBasisLowerer::new(target_basis(&[
            StandardGate::RZ,
            StandardGate::X2P,
            StandardGate::CZ,
        ]))
        .unwrap(),
    );

    let model = TargetBasisCostModel::from_lowerer(Arc::clone(&lowerer)).unwrap();

    assert!(Arc::ptr_eq(&model.lowerer, &lowerer));
}

#[test]
fn cost_model_rejects_non_standard_shared_lowerer() {
    let lowerer = Arc::new(
        TargetBasisLowerer::new(vec![Instruction::McGate(Box::new(MCGate::new(
            2,
            StandardGate::X,
        )))])
        .unwrap(),
    );

    let error = TargetBasisCostModel::from_lowerer(lowerer).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("target-basis cost model requires standard instructions")
    );
}

#[test]
fn requires_lowering_skips_empty_physical_and_non_gate_operations() {
    let lowerer =
        TargetBasisLowerer::new(target_basis(&[StandardGate::X, StandardGate::CX])).unwrap();
    assert!(!lowerer.requires_lowering(&Circuit::new(1)));

    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.x(q0).unwrap();
    circuit.barrier(vec![q0]).unwrap();
    circuit.delay(q0, ParameterValue::Fixed(4.0)).unwrap();
    circuit.measure(q0).unwrap();
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| body.break_loop())
        .unwrap();
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.continue_loop()
        })
        .unwrap();

    assert!(!lowerer.requires_lowering(&circuit));
}

#[test]
fn requires_lowering_detects_nonphysical_and_extended_gate_like_operations() {
    let lowerer = TargetBasisLowerer::new(target_basis(&[StandardGate::X])).unwrap();
    let q0 = Qubit::new(0);

    let mut nonphysical = Circuit::new(1);
    nonphysical.h(q0).unwrap();
    assert!(lowerer.requires_lowering(&nonphysical));

    let mut multi_controlled = Circuit::new(3);
    multi_controlled
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
            [Qubit::new(0), Qubit::new(1), Qubit::new(2)],
            std::iter::empty(),
            None,
        )
        .unwrap();
    assert!(lowerer.requires_lowering(&multi_controlled));

    let mut unitary = Circuit::new(1);
    unitary
        .unitary(UnitaryGate::new("opaque", 1, 0), vec![q0])
        .unwrap();
    assert!(lowerer.requires_lowering(&unitary));

    let mut definition = Circuit::new(1);
    definition.x(q0).unwrap();
    let gate = CircuitGate::new("defined_x", FrozenCircuit::new(definition)).unwrap();
    let mut circuit_gate = Circuit::new(1);
    circuit_gate
        .circuit_gate(gate, vec![q0], std::iter::empty())
        .unwrap();
    assert!(lowerer.requires_lowering(&circuit_gate));
}

#[test]
fn requires_lowering_treats_explicit_gphase_as_translation_work() {
    let lowerer =
        TargetBasisLowerer::new(target_basis(&[StandardGate::X, StandardGate::GPhase])).unwrap();
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            Instruction::Standard(StandardGate::GPhase),
            Vec::<Qubit>::new(),
            [ParameterValue::Fixed(0.25)],
            None,
        )
        .unwrap();

    assert!(lowerer.requires_lowering(&circuit));
}

#[test]
fn requires_lowering_recurses_through_every_control_flow_body() {
    let lowerer = TargetBasisLowerer::new(target_basis(&[StandardGate::X])).unwrap();
    let q0 = Qubit::new(0);

    let mut if_then = Circuit::new(1);
    if_then
        .if_(ClassicalExpr::bool_literal(true), |body| body.h(q0))
        .unwrap();
    assert!(lowerer.requires_lowering(&if_then));

    let mut if_else = Circuit::new(1);
    if_else
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| body.x(q0),
            |body| body.h(q0),
        )
        .unwrap();
    assert!(lowerer.requires_lowering(&if_else));

    let mut while_loop = Circuit::new(1);
    while_loop
        .while_(ClassicalExpr::bool_literal(true), |body| body.h(q0))
        .unwrap();
    assert!(lowerer.requires_lowering(&while_loop));

    let mut for_loop = Circuit::new(1);
    let loop_var = for_loop.var(ClassicalType::uint(2).unwrap());
    for_loop
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(2, 0).unwrap(),
            ClassicalExpr::uint_literal(2, 1).unwrap(),
            ClassicalExpr::uint_literal(2, 1).unwrap(),
            |body, _| body.h(q0),
        )
        .unwrap();
    assert!(lowerer.requires_lowering(&for_loop));

    let mut switch_case = Circuit::new(1);
    switch_case
        .switch(ClassicalExpr::uint_literal(2, 0).unwrap(), |switch| {
            switch.value(0, |body| body.h(q0))?;
            switch.default(|body| body.x(q0))?;
            Ok(())
        })
        .unwrap();
    assert!(lowerer.requires_lowering(&switch_case));

    let mut switch_default = Circuit::new(1);
    switch_default
        .switch(ClassicalExpr::uint_literal(2, 0).unwrap(), |switch| {
            switch.value(0, |body| body.x(q0))?;
            switch.default(|body| body.h(q0))?;
            Ok(())
        })
        .unwrap();
    assert!(lowerer.requires_lowering(&switch_default));
}

fn u_circuit(theta: f64, phi: f64, lambda: f64) -> Circuit {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            Instruction::Standard(StandardGate::U),
            vec![q0],
            vec![
                ParameterValue::Fixed(theta),
                ParameterValue::Fixed(phi),
                ParameterValue::Fixed(lambda),
            ],
            None,
        )
        .unwrap();
    circuit
}

fn non_gphase_ops(circuit: &Circuit) -> Vec<&Operation> {
    circuit
        .operations()
        .iter()
        .filter(|operation| {
            !matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::GPhase)
            )
        })
        .collect()
}

#[test]
fn degenerate_u_uses_dynamic_euler_synthesis() {
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::X];
    let cases: [(f64, usize); 4] = [
        (0.0, 1),
        (std::f64::consts::FRAC_PI_2, 3),
        (std::f64::consts::PI, 2),
        (0.4, 5),
    ];
    for (theta, expected_ops) in cases {
        let circuit = u_circuit(theta, 0.3, 0.7);
        let result = run_target_lowering(&circuit, &basis);
        assert_only_target_standard_gates(&result, &basis);
        assert_eq!(
            non_gphase_ops(&result).len(),
            expected_ops,
            "theta={theta} should lower to {expected_ops} gates: {:?}",
            result.operations()
        );
        assert_compiled_circuit_equivalent(&result, &circuit);
    }
}

#[test]
fn degenerate_u_reduces_to_single_rz_for_x2p_only_basis() {
    let basis = [StandardGate::RZ, StandardGate::X2P];
    let circuit = u_circuit(0.0, 0.3, 0.7);

    let result = run_target_lowering(&circuit, &basis);

    assert_only_target_standard_gates(&result, &basis);
    assert_eq!(non_gphase_ops(&result).len(), 1);
    assert_compiled_circuit_equivalent(&result, &circuit);
}

#[test]
fn mixed_basis_static_plan_is_not_regressed() {
    let q0 = Qubit::new(0);
    let basis = [
        StandardGate::RZ,
        StandardGate::RY,
        StandardGate::X2P,
        StandardGate::X2M,
    ];
    let circuit = u_circuit(0.4, 0.3, 0.7);

    let result = run_target_lowering(&circuit, &basis);

    assert_only_target_standard_gates(&result, &basis);
    let ops = non_gphase_ops(&result);
    assert_eq!(
        ops.len(),
        3,
        "static three-gate ZYZ plan must beat the five-gate dynamic candidate: {:?}",
        result.operations()
    );
    assert!(matches!(
        ops[1].instruction,
        Instruction::Standard(StandardGate::RY)
    ));
    assert!(
        ops.iter()
            .all(|operation| operation.qubits.as_slice() == [q0])
    );
    assert_compiled_circuit_equivalent(&result, &circuit);
}

#[test]
fn symbolic_u_keeps_static_rule_path() {
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::X];
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.add_parameter(Parameter::symbol("theta"));
    circuit.add_parameter(Parameter::symbol("phi"));
    circuit.add_parameter(Parameter::symbol("lambda"));
    circuit
        .append(
            Instruction::Standard(StandardGate::U),
            vec![q0],
            vec![
                ParameterValue::Param(Parameter::symbol("theta")),
                ParameterValue::Param(Parameter::symbol("phi")),
                ParameterValue::Param(Parameter::symbol("lambda")),
            ],
            None,
        )
        .unwrap();

    let result = run_target_lowering(&circuit, &basis);

    assert_only_target_standard_gates(&result, &basis);
    assert!(!standard_ops(&result).contains(&StandardGate::U));
}

#[test]
fn native_u_is_passed_through_unchanged() {
    let basis = [StandardGate::U, StandardGate::RZ, StandardGate::X2P];
    let circuit = u_circuit(0.4, 0.3, 0.7);

    let result = run_target_lowering(&circuit, &basis);

    let ops = non_gphase_ops(&result);
    assert_eq!(ops.len(), 1);
    assert!(matches!(
        ops[0].instruction,
        Instruction::Standard(StandardGate::U)
    ));
    assert_compiled_circuit_equivalent(&result, &circuit);
}

#[test]
fn u_fails_for_basis_without_complete_euler_family() {
    let circuit = u_circuit(0.4, 0.3, 0.7);

    assert_target_lowering_fails_with(
        &circuit,
        &[StandardGate::RZ, StandardGate::X2M],
        &["cannot lower", "U"],
    );
}

#[test]
fn control_flow_body_phase_stays_inside_body() {
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::X];
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.append(
                Instruction::Standard(StandardGate::U),
                vec![q0],
                vec![
                    ParameterValue::Fixed(0.0),
                    ParameterValue::Fixed(0.3),
                    ParameterValue::Fixed(0.7),
                ],
                None,
            )?;
            Ok(())
        })
        .unwrap();

    let result = run_target_lowering(&circuit, &basis);

    assert_eq!(result.operations().len(), 1);
    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &result.operations()[0].instruction
    else {
        panic!("expected an if operation, got {:?}", result.operations());
    };
    let body = if_op.then_body().operations();
    assert!(
        body.iter().any(|operation| matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::GPhase)
        )),
        "body should carry the synthesis phase locally: {body:?}"
    );
    assert!(
        body.iter().any(|operation| matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::RZ)
        )),
        "body should contain the synthesized RZ: {body:?}"
    );
}

#[test]
fn cost_model_sees_degenerate_u_as_one_gate() {
    let q0 = Qubit::new(0);
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::X];
    let operations = vec![ValueOperation::from_standard(
        StandardGate::U,
        [q0],
        [0.0.into(), 0.3.into(), 0.7.into()],
    )];
    let model = TargetBasisCostModel::new(target_basis(&basis)).unwrap();

    let cost = model
        .cost_of_fixed_operations(vec![q0], operations)
        .unwrap();

    assert_eq!(cost.total_ops, 1);
    assert_eq!(cost.two_qubit_ops, 0);
    assert_eq!(cost.parameterized_ops, 1);
    assert_eq!(cost.depth, 1);
}

#[test]
fn stored_plan_cost_matches_static_lowering_output() {
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::X2M];
    let lowerer = TargetBasisLowerer::new(target_basis(&basis)).unwrap();
    let key =
        KnowledgeInstructionKey::from_instruction(&Instruction::Standard(StandardGate::U)).unwrap();
    let plan = lowerer.plans.plan_for(&key).unwrap();
    assert_eq!(plan.cost.two_qubit_ops, 0);

    // A tie keeps the static path, so the lowered circuit is the static plan's
    // output and must match the stored cost exactly.
    let circuit = u_circuit(0.4, 0.3, 0.7);
    let result = run_target_lowering(&circuit, &basis);
    let ops = non_gphase_ops(&result);
    assert_eq!(plan.cost.total_ops, ops.len());
    assert_eq!(
        plan.cost.parameterized_ops,
        ops.iter()
            .filter(|operation| !operation.params.is_empty())
            .count()
    );
    assert_compiled_circuit_equivalent(&result, &circuit);
}

#[test]
fn multi_family_lowering_is_deterministic() {
    let basis = [
        StandardGate::RZ,
        StandardGate::X2P,
        StandardGate::X2M,
        StandardGate::X,
    ];
    let circuit = u_circuit(0.4, 0.3, 0.7);

    let first = run_target_lowering(&circuit, &basis);
    let second = run_target_lowering(&circuit, &basis);

    assert_eq!(first, second);
    assert_only_target_standard_gates(&first, &basis);
}

#[test]
fn labeled_u_decomposition_drops_label_like_static_path() {
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::X];
    for (theta, expected_ops) in [(0.0_f64, 1_usize), (std::f64::consts::FRAC_PI_2, 3)] {
        let q0 = Qubit::new(0);
        let mut circuit = Circuit::new(1);
        circuit
            .append(
                Instruction::Standard(StandardGate::U),
                vec![q0],
                vec![
                    ParameterValue::Fixed(theta),
                    ParameterValue::Fixed(0.3),
                    ParameterValue::Fixed(0.7),
                ],
                Some("source-label"),
            )
            .unwrap();

        let result = run_target_lowering(&circuit, &basis);

        assert_eq!(non_gphase_ops(&result).len(), expected_ops);
        assert!(
            result.operations().iter().all(|op| op.label.is_none()),
            "decomposed gates must not inherit the source label: {:?}",
            result.operations()
        );
        assert_compiled_circuit_equivalent(&result, &circuit);
    }
}

#[test]
fn labeled_u_collapsing_to_identity_emits_no_gate_and_no_label() {
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::X];
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            Instruction::Standard(StandardGate::U),
            vec![q0],
            vec![
                ParameterValue::Fixed(0.0),
                ParameterValue::Fixed(0.3),
                ParameterValue::Fixed(-0.3),
            ],
            Some("source-label"),
        )
        .unwrap();

    let result = run_target_lowering(&circuit, &basis);

    assert!(
        result.operations().is_empty(),
        "identity-equivalent U should vanish entirely: {:?}",
        result.operations()
    );
    assert_compiled_circuit_equivalent(&result, &circuit);
}

#[test]
fn non_normalized_u_angles_are_normalized_through_the_matrix() {
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::X];
    // U(2*pi, ...) is RZ-like up to phase; U(5*pi, ...) is X-like up to phase.
    let cases: [(f64, usize); 2] = [
        (2.0 * std::f64::consts::PI, 1),
        (5.0 * std::f64::consts::PI, 2),
    ];
    for (theta, max_ops) in cases {
        let circuit = u_circuit(theta, 0.3, 0.7);
        let result = run_target_lowering(&circuit, &basis);
        assert_only_target_standard_gates(&result, &basis);
        assert!(
            non_gphase_ops(&result).len() <= max_ops,
            "theta={theta} should lower to at most {max_ops} gates: {:?}",
            result.operations()
        );
        assert_compiled_circuit_equivalent(&result, &circuit);
    }
}

#[test]
fn swap_lowers_to_rz_x2p_cz_basis() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.swap(q0, q1).unwrap();
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::CZ];

    let result = run_target_lowering(&circuit, &basis);

    assert_only_target_standard_gates(&result, &basis);
    assert!(!standard_ops(&result).contains(&StandardGate::SWAP));
}

#[test]
fn arbitrary_u_lowers_to_qcis_half_rotation_basis() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            Instruction::Standard(StandardGate::U),
            vec![q0],
            vec![
                ParameterValue::Fixed(0.7),
                ParameterValue::Fixed(-0.4),
                ParameterValue::Fixed(0.9),
            ],
            None,
        )
        .unwrap();
    let basis = [
        StandardGate::RZ,
        StandardGate::X2P,
        StandardGate::X2M,
        StandardGate::Y2P,
        StandardGate::Y2M,
        StandardGate::CZ,
        StandardGate::GPhase,
    ];

    let result = run_target_lowering(&circuit, &basis);

    assert_only_target_standard_gates(&result, &basis);
    assert_compiled_circuit_equivalent(&result, &circuit);
}

#[test]
fn exact_cost_model_matches_the_target_lowerer_output() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::CZ];
    let operations = vec![
        ValueOperation::from_standard(StandardGate::CX, [q0, q1], []),
        ValueOperation::from_standard(
            StandardGate::U,
            [q1],
            [0.7.into(), (-0.4).into(), 0.9.into()],
        ),
    ];
    let source = Circuit::from_operations(vec![q0, q1], operations.clone(), None, None).unwrap();
    let model = TargetBasisCostModel::new(target_basis(&basis)).unwrap();

    let estimated = model
        .cost_of_fixed_operations(vec![q0, q1], operations)
        .unwrap();
    let lowered = run_target_lowering(&source, &basis);
    let physical_ops = lowered
        .operations()
        .iter()
        .filter(|operation| {
            !matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::GPhase)
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(estimated.total_ops, physical_ops.len());
    assert_eq!(
        estimated.two_qubit_ops,
        physical_ops
            .iter()
            .filter(|operation| operation.qubits.len() == 2)
            .count()
    );
    assert_eq!(
        estimated.parameterized_ops,
        physical_ops
            .iter()
            .filter(|operation| !operation.params.is_empty())
            .count()
    );
    assert_eq!(estimated.depth, lowered.depth(false).unwrap());
}

#[test]
fn swap_lowers_to_original_qcis_bug_basis() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.swap(q0, q1).unwrap();
    let basis = [
        StandardGate::I,
        StandardGate::X2P,
        StandardGate::X,
        StandardGate::RZ,
        StandardGate::CZ,
    ];

    let result = run_target_lowering(&circuit, &basis);

    assert_only_target_standard_gates(&result, &basis);
    assert!(!standard_ops(&result).contains(&StandardGate::SWAP));
}

#[test]
fn h_lowers_to_rz_x2p_cz_basis() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::CZ];

    let result = run_target_lowering(&circuit, &basis);

    assert_only_target_standard_gates(&result, &basis);
    assert!(!standard_ops(&result).contains(&StandardGate::H));
}

#[test]
fn cx_lowers_to_rz_x2p_cz_basis() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::CZ];

    let result = run_target_lowering(&circuit, &basis);

    assert_only_target_standard_gates(&result, &basis);
    assert!(!standard_ops(&result).contains(&StandardGate::CX));
}

#[test]
fn ccx_lowers_to_rz_x2p_cz_basis() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.ccx(q0, q1, q2).unwrap();
    let basis = [StandardGate::RZ, StandardGate::X2P, StandardGate::CZ];

    let result = run_target_lowering(&circuit, &basis);

    assert_only_target_standard_gates(&result, &basis);
    assert!(!standard_ops(&result).contains(&StandardGate::CCX));
}

#[test]
fn swap_fails_for_rz_x2p_basis_without_entangling_gate() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.swap(q0, q1).unwrap();

    assert_target_lowering_fails_with(
        &circuit,
        &[StandardGate::RZ, StandardGate::X2P],
        &["cannot lower", "SWAP", "RZ", "X2P"],
    );
}

#[test]
fn swap_fails_for_rz_cz_basis_without_half_rotation() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.swap(q0, q1).unwrap();

    assert_target_lowering_fails_with(
        &circuit,
        &[StandardGate::RZ, StandardGate::CZ],
        &["cannot lower", "SWAP", "RZ", "CZ"],
    );
}
