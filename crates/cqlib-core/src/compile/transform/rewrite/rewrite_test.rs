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

use super::matcher::rule_preserves_two_qubit_connectivity;
use super::{KnowledgeRewriter, RewriteConfig};
use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, ClassicalExpr, Directive, Instruction, MCGate,
    Parameter, ParameterValue, Qubit, StandardGate,
};
use crate::compile::CompilerError;
use crate::compile::knowledge::RuleLibrary;
use crate::compile::knowledge::library::RuleKind;
use crate::compile::knowledge::rule_dsl::load::load_rules_from_str;
use crate::compile::test_utils::standard_ops;

#[test]
fn cancels_adjacent_self_inverse_gates() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();
    circuit.h(q0).unwrap();

    let result = KnowledgeRewriter::production().run(&circuit).unwrap();

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert!(result.stats.reached_fixpoint);
}

#[test]
fn cancels_across_commuting_disjoint_operation() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.x(q1).unwrap();
    circuit.h(q0).unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Cancel]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::X]);
    assert_eq!(result.circuit.operations()[0].qubits.as_slice(), &[q1]);
}

#[test]
fn does_not_cancel_across_non_commuting_operation() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();
    circuit.x(q0).unwrap();
    circuit.h(q0).unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Cancel]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(!result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::H, StandardGate::X, StandardGate::H]
    );
}

#[test]
fn protects_labeled_operations_by_default() {
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
    circuit.h(q0).unwrap();

    let result = KnowledgeRewriter::production().run(&circuit).unwrap();

    assert!(!result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::H, StandardGate::H]
    );
}

#[test]
fn does_not_cross_labeled_skipped_operation() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit
        .append(
            Instruction::Standard(StandardGate::X),
            [q1],
            std::iter::empty(),
            Some("skip"),
        )
        .unwrap();
    circuit.h(q0).unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Cancel]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(!result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::H, StandardGate::X, StandardGate::H]
    );
}

#[test]
fn barrier_splits_rewrite_blocks() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();
    circuit.barrier(vec![q0]).unwrap();
    circuit.h(q0).unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Cancel]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(!result.changed);
    assert_eq!(result.circuit.operations().len(), 3);
    assert!(matches!(
        result.circuit.operations()[1].instruction,
        Instruction::Directive(Directive::Barrier)
    ));
}

#[test]
fn merges_numeric_rotations() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rz(q0, 0.25).unwrap();
    circuit.rz(q0, 0.5).unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Merge]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::RZ]);
    assert!(matches!(
        result.circuit.operations()[0].params[0],
        CircuitParam::Fixed(value) if (value - 0.75).abs() < 1e-12
    ));
}

#[test]
fn merges_symbolic_rotations() {
    let q0 = Qubit::new(0);
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(1);
    circuit.rz(q0, theta.clone()).unwrap();
    circuit.rz(q0, 0.5).unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Merge]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::RZ]);
    let merged = operation_param(&result.circuit, &result.circuit.operations()[0].params[0]);
    let expected = theta + Parameter::from(0.5);
    assert!(merged.provably_equal(&expected, 1e-12));
}

#[test]
fn merges_rz_across_same_qubit_commuting_s_gate() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rz(q0, 0.25).unwrap();
    circuit.s(q0).unwrap();
    circuit.rz(q0, 0.5).unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Merge]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::RZ, StandardGate::S]
    );
    assert!(matches!(
        result.circuit.operations()[0].params[0],
        CircuitParam::Fixed(value) if (value - 0.75).abs() < 1e-12
    ));
}

#[test]
fn merges_rz_across_same_qubit_commuting_phase_gate() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rz(q0, 0.25).unwrap();
    circuit.phase(q0, 0.125).unwrap();
    circuit.rz(q0, 0.5).unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Merge]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::RZ, StandardGate::Phase]
    );
    assert!(matches!(
        result.circuit.operations()[0].params[0],
        CircuitParam::Fixed(value) if (value - 0.75).abs() < 1e-12
    ));
    assert!(matches!(
        result.circuit.operations()[1].params[0],
        CircuitParam::Fixed(value) if (value - 0.125).abs() < 1e-12
    ));
}

#[test]
fn cancels_z_across_same_qubit_commuting_s_gate() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.z(q0).unwrap();
    circuit.s(q0).unwrap();
    circuit.z(q0).unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Cancel]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::S]);
}

#[test]
fn does_not_merge_rz_across_non_commuting_h_gate() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rz(q0, 0.25).unwrap();
    circuit.h(q0).unwrap();
    circuit.rz(q0, 0.5).unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Merge]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(!result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::RZ, StandardGate::H, StandardGate::RZ]
    );
}

#[test]
fn does_not_cancel_x_across_non_commuting_z_gate() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.x(q0).unwrap();
    circuit.z(q0).unwrap();
    circuit.x(q0).unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Cancel]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(!result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::X, StandardGate::Z, StandardGate::X]
    );
}

#[test]
fn commuting_match_respects_max_window_ops() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rz(q0, 0.25).unwrap();
    circuit.s(q0).unwrap();
    circuit.t(q0).unwrap();
    circuit.rz(q0, 0.5).unwrap();

    let tight_config = RewriteConfig::production()
        .with_enabled_kinds(vec![RuleKind::Merge])
        .with_max_window_ops(1);
    let tight_result = KnowledgeRewriter::new(tight_config).run(&circuit).unwrap();

    assert!(!tight_result.changed);
    assert_eq!(
        standard_ops(&tight_result.circuit),
        vec![
            StandardGate::RZ,
            StandardGate::S,
            StandardGate::T,
            StandardGate::RZ
        ]
    );

    let wide_config = RewriteConfig::production()
        .with_enabled_kinds(vec![RuleKind::Merge])
        .with_max_window_ops(4);
    let wide_result = KnowledgeRewriter::new(wide_config).run(&circuit).unwrap();

    assert!(wide_result.changed);
    assert_eq!(
        standard_ops(&wide_result.circuit),
        vec![StandardGate::RZ, StandardGate::S, StandardGate::T]
    );
    assert!(matches!(
        wide_result.circuit.operations()[0].params[0],
        CircuitParam::Fixed(value) if (value - 0.75).abs() < 1e-12
    ));
}

#[test]
fn folds_top_level_gphase_into_circuit_global_phase() {
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            Instruction::Standard(StandardGate::GPhase),
            std::iter::empty::<Qubit>(),
            [Parameter::from(0.25).into()],
            None,
        )
        .unwrap();

    let result = KnowledgeRewriter::production().run(&circuit).unwrap();

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert!(
        result
            .circuit
            .global_phase()
            .provably_equal(&Parameter::from(0.25), 1e-12)
    );
}

#[test]
fn lowers_to_explicit_target_basis() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();

    let config = RewriteConfig::lowering()
        .with_target_instructions(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CZ),
        ])
        .unwrap();
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::H, StandardGate::CZ, StandardGate::H]
    );
    assert_eq!(result.circuit.operations()[0].qubits.as_slice(), &[q1]);
    assert_eq!(result.circuit.operations()[1].qubits.as_slice(), &[q0, q1]);
    assert_eq!(result.circuit.operations()[2].qubits.as_slice(), &[q1]);
}

#[test]
fn target_basis_lowering_preserves_physical_source_gate() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.x2p(q0).unwrap();

    let config = RewriteConfig::lowering()
        .with_target_instructions(vec![
            Instruction::Standard(StandardGate::RZ),
            Instruction::Standard(StandardGate::X2P),
        ])
        .unwrap();
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::X2P]);
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::RY));
}

#[test]
fn one_round_limit_stops_before_second_step_lowering() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
            [q0, q1, q2],
            std::iter::empty::<crate::circuit::ParameterValue>(),
            None,
        )
        .unwrap();

    let config = RewriteConfig::lowering()
        .with_enabled_kinds(vec![RuleKind::Decompose])
        .with_max_rounds(1);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::CCX]);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::CCX)
    ));
}

#[test]
fn two_rounds_continue_chain_beyond_first_replacement() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
            [q0, q1, q2],
            std::iter::empty::<crate::circuit::ParameterValue>(),
            None,
        )
        .unwrap();

    let config = RewriteConfig::lowering()
        .with_enabled_kinds(vec![RuleKind::Decompose])
        .with_target_instructions(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
            Instruction::Standard(StandardGate::T),
            Instruction::Standard(StandardGate::TDG),
        ])
        .unwrap()
        .with_max_rounds(2);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(result.changed);
    assert!(result.circuit.operations().iter().all(|operation| {
        !matches!(
            operation.instruction,
            Instruction::McGate(_) | Instruction::Standard(StandardGate::CCX)
        )
    }));
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
            StandardGate::CX
        ]
    );
}

#[test]
fn lowering_reaches_target_basis_through_multiple_steps() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
            [q0, q1, q2],
            std::iter::empty::<crate::circuit::ParameterValue>(),
            None,
        )
        .unwrap();

    let config = RewriteConfig::lowering()
        .with_enabled_kinds(vec![RuleKind::Decompose])
        .with_target_instructions(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
            Instruction::Standard(StandardGate::T),
            Instruction::Standard(StandardGate::TDG),
        ])
        .unwrap()
        .with_max_rounds(4);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(result.changed);
    assert!(result.circuit.operations().iter().all(|operation| matches!(
        operation.instruction,
        Instruction::Standard(
            StandardGate::H | StandardGate::CX | StandardGate::T | StandardGate::TDG
        )
    )));
    assert!(result.stats.rules_applied >= 2);
    assert!(result.stats.rounds_executed >= 3);
}

#[test]
fn lowers_ccx_directly_to_multiple_qiskit_basis_sets() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.ccx(q0, q1, q2).unwrap();

    for basis in [
        vec![StandardGate::U, StandardGate::CX],
        vec![StandardGate::U, StandardGate::CZ],
        vec![StandardGate::RZ, StandardGate::X2P, StandardGate::CX],
        vec![
            StandardGate::RZ,
            StandardGate::X2P,
            StandardGate::X,
            StandardGate::CZ,
        ],
        vec![StandardGate::RX, StandardGate::RY, StandardGate::CX],
        vec![StandardGate::RX, StandardGate::RY, StandardGate::CZ],
        vec![StandardGate::RX, StandardGate::RY, StandardGate::RXX],
        vec![StandardGate::RX, StandardGate::RY, StandardGate::RZZ],
        vec![
            StandardGate::RZ,
            StandardGate::X2P,
            StandardGate::X,
            StandardGate::RZZ,
        ],
    ] {
        let target_instructions = basis.iter().copied().map(Instruction::Standard).collect();
        let result = KnowledgeRewriter::new(
            RewriteConfig::lowering()
                .with_enabled_kinds(vec![RuleKind::Decompose])
                .with_target_instructions(target_instructions)
                .unwrap(),
        )
        .run(&circuit)
        .unwrap();

        assert!(result.changed);
        assert!(
            standard_ops(&result.circuit)
                .iter()
                .all(|gate| basis.contains(gate)),
            "CCX lowering emitted a gate outside basis {basis:?}"
        );
    }
}

#[test]
fn qiskit_rzz_ccx_rule_uses_five_entanglers() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.ccx(q0, q1, q2).unwrap();

    let result = KnowledgeRewriter::new(
        RewriteConfig::lowering()
            .with_enabled_kinds(vec![RuleKind::Decompose])
            .with_target_instructions(vec![
                Instruction::Standard(StandardGate::RX),
                Instruction::Standard(StandardGate::RY),
                Instruction::Standard(StandardGate::RZZ),
            ])
            .unwrap(),
    )
    .run(&circuit)
    .unwrap();

    assert_eq!(
        standard_ops(&result.circuit)
            .iter()
            .filter(|&&gate| gate == StandardGate::RZZ)
            .count(),
        5
    );
}

#[test]
fn qiskit_cz_ccx_rule_uses_native_x2p_x_rz_cz_template() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.ccx(q0, q1, q2).unwrap();

    let result = KnowledgeRewriter::new(
        RewriteConfig::lowering()
            .with_enabled_kinds(vec![RuleKind::Decompose])
            .with_target_instructions(vec![
                Instruction::Standard(StandardGate::RZ),
                Instruction::Standard(StandardGate::X2P),
                Instruction::Standard(StandardGate::X),
                Instruction::Standard(StandardGate::CZ),
            ])
            .unwrap(),
    )
    .run(&circuit)
    .unwrap();

    let gates = standard_ops(&result.circuit);
    assert_eq!(
        gates
            .iter()
            .filter(|&&gate| gate == StandardGate::RZ)
            .count(),
        15
    );
    assert_eq!(
        gates
            .iter()
            .filter(|&&gate| gate == StandardGate::X2P)
            .count(),
        12
    );
    assert_eq!(
        gates
            .iter()
            .filter(|&&gate| gate == StandardGate::X)
            .count(),
        3
    );
    assert_eq!(
        gates
            .iter()
            .filter(|&&gate| gate == StandardGate::CZ)
            .count(),
        6
    );
}

fn lower_to_standard_basis(circuit: &Circuit, basis: &[StandardGate]) -> Circuit {
    KnowledgeRewriter::new(
        RewriteConfig::lowering()
            .with_target_instructions(basis.iter().copied().map(Instruction::Standard).collect())
            .unwrap(),
    )
    .run(circuit)
    .unwrap()
    .circuit
}

fn count_gate(circuit: &Circuit, gate: StandardGate) -> usize {
    standard_ops(circuit)
        .iter()
        .filter(|&&candidate| candidate == gate)
        .count()
}

#[test]
fn benchpress_h_ccx_h_motif_uses_direct_target_templates() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.h(q2).unwrap();
    circuit.ccx(q0, q1, q2).unwrap();
    circuit.h(q2).unwrap();

    for (basis, entangler, expected_count) in [
        (
            vec![
                StandardGate::RZ,
                StandardGate::X2P,
                StandardGate::X,
                StandardGate::CZ,
            ],
            StandardGate::CZ,
            6,
        ),
        (
            vec![
                StandardGate::RZ,
                StandardGate::X2P,
                StandardGate::X,
                StandardGate::CX,
            ],
            StandardGate::CX,
            6,
        ),
        (
            vec![StandardGate::RX, StandardGate::RY, StandardGate::RZZ],
            StandardGate::RZZ,
            5,
        ),
    ] {
        let lowered = lower_to_standard_basis(&circuit, &basis);
        assert_eq!(count_gate(&lowered, entangler), expected_count);
        assert!(
            standard_ops(&lowered)
                .iter()
                .all(|gate| basis.contains(gate))
        );
    }
}

#[test]
fn benchpress_cx_rz_cx_motif_composes_to_one_rzz() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.rz(q1, 0.37).unwrap();
    circuit.cx(q0, q1).unwrap();

    let lowered = lower_to_standard_basis(
        &circuit,
        &[StandardGate::RX, StandardGate::RY, StandardGate::RZZ],
    );

    assert_eq!(standard_ops(&lowered), vec![StandardGate::RZZ]);
}

#[test]
fn benchpress_cx_phase_cx_motif_composes_to_one_rzz() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.phase(q1, 0.37).unwrap();
    circuit.cx(q0, q1).unwrap();

    let lowered = lower_to_standard_basis(
        &circuit,
        &[StandardGate::RX, StandardGate::RY, StandardGate::RZZ],
    );

    assert_eq!(standard_ops(&lowered), vec![StandardGate::RZZ]);
}

#[test]
fn cz_rx_cz_motifs_compose_to_oriented_rzx() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    for (rotation_qubit, expected_qubits) in [(q1, [q0, q1]), (q0, [q1, q0])] {
        let mut circuit = Circuit::new(2);
        circuit.cz(q0, q1).unwrap();
        circuit.rx(rotation_qubit, 0.37).unwrap();
        circuit.cz(q0, q1).unwrap();

        let lowered = lower_to_standard_basis(&circuit, &[StandardGate::RZX]);
        assert_eq!(standard_ops(&lowered), vec![StandardGate::RZX]);
        assert_eq!(
            lowered.operations()[0].qubits.as_slice(),
            expected_qubits.as_slice()
        );
    }
}

#[test]
fn cx_cz_motifs_compose_to_one_controlled_pauli() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    for (cx_first, cz_qubits) in [
        (true, [q0, q1]),
        (true, [q1, q0]),
        (false, [q0, q1]),
        (false, [q1, q0]),
    ] {
        let mut circuit = Circuit::new(2);
        if cx_first {
            circuit.cx(q0, q1).unwrap();
        }
        circuit.cz(cz_qubits[0], cz_qubits[1]).unwrap();
        if !cx_first {
            circuit.cx(q0, q1).unwrap();
        }

        let lowered = lower_to_standard_basis(&circuit, &[StandardGate::Phase, StandardGate::CY]);
        assert_eq!(
            standard_ops(&lowered),
            vec![StandardGate::Phase, StandardGate::CY]
        );
    }
}

#[test]
fn degenerate_u_and_fsim_lower_directly_to_specialized_targets() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    let mut u_circuit = Circuit::new(1);
    u_circuit.u(q0, 0.0, 0.23, -0.41).unwrap();
    let lowered_u = lower_to_standard_basis(&u_circuit, &[StandardGate::Phase]);
    assert_eq!(standard_ops(&lowered_u), vec![StandardGate::Phase]);

    let mut fsim_circuit = Circuit::new(2);
    fsim_circuit.fsim(q0, q1, 0.0, 0.37).unwrap();
    let lowered_fsim =
        lower_to_standard_basis(&fsim_circuit, &[StandardGate::Phase, StandardGate::CRZ]);
    assert_eq!(
        standard_ops(&lowered_fsim),
        vec![StandardGate::Phase, StandardGate::CRZ]
    );
}

#[test]
fn cancels_symmetric_multi_controlled_gate_applications() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);

    for qubits in [[q0, q1, q2], [q1, q0, q2]] {
        circuit
            .append(
                Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
                qubits,
                std::iter::empty::<ParameterValue>(),
                None,
            )
            .unwrap();
    }
    for qubits in [[q0, q1, q2], [q0, q2, q1]] {
        circuit
            .append(
                Instruction::McGate(Box::new(MCGate::new(1, StandardGate::SWAP))),
                qubits,
                std::iter::empty::<ParameterValue>(),
                None,
            )
            .unwrap();
    }

    let result = KnowledgeRewriter::new(
        RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Cancel]),
    )
    .run(&circuit)
    .unwrap();
    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
}

#[test]
fn normalizes_periodic_controlled_rotations_and_phases() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);

    for gate in [StandardGate::CRX, StandardGate::CRY, StandardGate::CRZ] {
        circuit
            .append(
                Instruction::Standard(gate),
                [q0, q1],
                [ParameterValue::from(4.0 * std::f64::consts::PI)],
                None,
            )
            .unwrap();
    }
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::Phase))),
            [q0, q1, q2],
            [ParameterValue::from(2.0 * std::f64::consts::PI)],
            None,
        )
        .unwrap();

    let result = KnowledgeRewriter::new(
        RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Canonicalize]),
    )
    .run(&circuit)
    .unwrap();
    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
}

#[test]
fn benchpress_controlled_phase_uses_minimal_entangler_templates() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(1, StandardGate::Phase))),
            [q0, q1],
            [ParameterValue::from(0.37)],
            None,
        )
        .unwrap();

    for (basis, entangler, expected_count) in [
        (
            vec![
                StandardGate::RZ,
                StandardGate::X2P,
                StandardGate::X,
                StandardGate::CZ,
            ],
            StandardGate::CZ,
            2,
        ),
        (
            vec![
                StandardGate::RZ,
                StandardGate::X2P,
                StandardGate::X,
                StandardGate::CX,
            ],
            StandardGate::CX,
            2,
        ),
        (
            vec![StandardGate::RX, StandardGate::RY, StandardGate::RZZ],
            StandardGate::RZZ,
            1,
        ),
    ] {
        let lowered = lower_to_standard_basis(&circuit, &basis);
        assert_eq!(count_gate(&lowered, entangler), expected_count);
    }
}

#[test]
fn benchpress_controlled_swap_uses_seven_entanglers() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(1, StandardGate::SWAP))),
            [q0, q1, q2],
            std::iter::empty::<ParameterValue>(),
            None,
        )
        .unwrap();

    for (basis, entangler) in [
        (
            vec![
                StandardGate::RZ,
                StandardGate::X2P,
                StandardGate::X,
                StandardGate::CZ,
            ],
            StandardGate::CZ,
        ),
        (
            vec![
                StandardGate::RZ,
                StandardGate::X2P,
                StandardGate::X,
                StandardGate::CX,
            ],
            StandardGate::CX,
        ),
        (
            vec![StandardGate::RX, StandardGate::RY, StandardGate::RZZ],
            StandardGate::RZZ,
        ),
    ] {
        let lowered = lower_to_standard_basis(&circuit, &basis);
        assert_eq!(count_gate(&lowered, entangler), 7);
    }
}

#[test]
fn lowering_fails_when_target_basis_cannot_be_satisfied() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();

    let config = RewriteConfig::lowering()
        .with_target_instructions(vec![Instruction::Standard(StandardGate::CZ)])
        .unwrap();
    let err = KnowledgeRewriter::new(config).run(&circuit).unwrap_err();

    assert!(matches!(err, CompilerError::InvalidInput(_)));
}

#[test]
fn optimize_mode_does_not_apply_decomposition_rules() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Decompose]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();

    assert!(!result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::CX]);
}

#[test]
fn rejects_invalid_target_basis_configuration() {
    let err = RewriteConfig::lowering()
        .with_target_instructions(vec![Instruction::Delay])
        .unwrap_err();

    assert!(matches!(err, CompilerError::InvalidInput(_)));
}

#[test]
fn rejects_zero_round_limit() {
    let circuit = Circuit::new(1);
    let err = KnowledgeRewriter::new(RewriteConfig::production().with_max_rounds(0))
        .run(&circuit)
        .unwrap_err();

    assert!(matches!(err, CompilerError::InvalidInput(_)));
}

#[test]
fn preserves_control_flow_body_local_global_phase() {
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.h(q1)?;
                body.y(q1)?;
                body.h(q1)
            },
            |_| Ok(()),
        )
        .unwrap();

    let result = KnowledgeRewriter::production().run(&circuit).unwrap();

    assert!(result.changed);
    let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected if operation");
    };
    assert_eq!(op.then_body().operations().len(), 2);
    assert!(matches!(
        op.then_body().operations()[0].instruction,
        Instruction::Standard(StandardGate::GPhase)
    ));
    assert!(matches!(
        op.then_body().operations()[1].instruction,
        Instruction::Standard(StandardGate::Y)
    ));
}

#[test]
fn rewrites_false_branch_and_while_body() {
    let q1 = Qubit::new(1);

    let mut if_circuit = Circuit::new(2);
    if_circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| body.x(q1),
            |body| {
                body.h(q1)?;
                body.h(q1)
            },
        )
        .unwrap();
    let if_result = KnowledgeRewriter::production().run(&if_circuit).unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &if_result.circuit.operations()[0].instruction
    else {
        panic!("expected if operation");
    };
    assert_eq!(if_op.then_body().operations().len(), 1);
    assert!(if_op.else_body().unwrap().operations().is_empty());

    let mut while_circuit = Circuit::new(2);
    while_circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.h(q1)?;
            body.h(q1)
        })
        .unwrap();
    let while_result = KnowledgeRewriter::production().run(&while_circuit).unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::While(while_op)) =
        &while_result.circuit.operations()[0].instruction
    else {
        panic!("expected while operation");
    };
    assert!(while_op.body().operations().is_empty());
}

#[test]
fn rewrites_runtime_classical_control_body_preserving_handles() {
    let mut circuit = Circuit::new(1);
    let measured = circuit.measure(Qubit::new(0)).unwrap();
    circuit
        .if_(
            ClassicalExpr::bit_to_bool(measured.expr()).unwrap(),
            |body| {
                body.x(Qubit::new(0))?;
                body.x(Qubit::new(0))
            },
        )
        .unwrap();

    let result = KnowledgeRewriter::production().run(&circuit).unwrap();

    assert!(result.changed);
    assert_eq!(result.circuit.classical_values().len(), 1);
    assert!(result.circuit.validate().is_ok());
    let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
        &result.circuit.operations()[1].instruction
    else {
        panic!("expected runtime classical if operation");
    };
    assert!(op.then_body().operations().is_empty());
}

#[test]
fn rewrites_control_flow_body_with_valid_rebuilt_parameter_table() {
    let q1 = Qubit::new(1);
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(2);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.rz(q1, theta.clone())?;
                body.rz(q1, 0.5)
            },
            |_| Ok(()),
        )
        .unwrap();

    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Merge]);
    let result = KnowledgeRewriter::new(config).run(&circuit).unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected if operation");
    };
    assert_eq!(op.then_body().operations().len(), 1);
    let body_param = &op.then_body().operations()[0].params[0];
    let merged = operation_param(&result.circuit, body_param);
    assert!(merged.provably_equal(&(theta + Parameter::from(0.5)), 1e-12));
}

fn operation_param(circuit: &Circuit, param: &CircuitParam) -> Parameter {
    match param {
        CircuitParam::Fixed(value) => Parameter::from(*value),
        CircuitParam::Index(index) => circuit
            .parameters()
            .get_index(*index as usize)
            .cloned()
            .expect("parameter index should exist in rebuilt circuit"),
    }
}

#[test]
fn incremental_matches_full_scan_for_multi_round_lowering() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
            [q0, q1, q2],
            std::iter::empty::<crate::circuit::ParameterValue>(),
            None,
        )
        .unwrap();
    let config = RewriteConfig::lowering()
        .with_enabled_kinds(vec![RuleKind::Decompose])
        .with_target_instructions(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
            Instruction::Standard(StandardGate::T),
            Instruction::Standard(StandardGate::TDG),
        ])
        .unwrap()
        .with_max_rounds(4);

    let full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(&circuit)
        .unwrap();
    let incremental = KnowledgeRewriter::new(config)
        .force_incremental()
        .run(&circuit)
        .unwrap();

    assert_eq!(incremental.circuit, full.circuit);
    assert_eq!(incremental.changed, full.changed);
    assert_eq!(incremental.stats, full.stats);
}

#[test]
fn parallel_incremental_matches_full_scan_on_large_block() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    for _ in 0..8_200 {
        circuit.h(q0).unwrap();
        circuit.h(q0).unwrap();
    }
    let config = RewriteConfig::production().with_enabled_kinds(vec![RuleKind::Cancel]);

    let full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(&circuit)
        .unwrap();
    let incremental = KnowledgeRewriter::new(config.clone())
        .force_incremental()
        .run(&circuit)
        .unwrap();
    let automatic = KnowledgeRewriter::new(config.clone())
        .run(&circuit)
        .unwrap();
    assert_eq!(incremental.circuit, full.circuit);
    assert_eq!(incremental.stats, full.stats);
    assert_eq!(automatic.circuit, full.circuit);
    assert_eq!(automatic.changed, full.changed);
    assert_eq!(automatic.stats, full.stats);

    let single_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap()
        .install(|| {
            KnowledgeRewriter::new(config.clone())
                .force_incremental()
                .run(&circuit)
                .unwrap()
        });
    let four_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            KnowledgeRewriter::new(config)
                .force_incremental()
                .run(&circuit)
                .unwrap()
        });
    assert_eq!(single_thread.circuit, four_threads.circuit);
    assert_eq!(single_thread.stats, four_threads.stats);
}

#[test]
fn incremental_matches_full_scan_with_root_phase_replacement() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();
    circuit.y(q0).unwrap();
    circuit.h(q0).unwrap();
    let config = RewriteConfig::production();

    let full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(&circuit)
        .unwrap();
    let incremental = KnowledgeRewriter::new(config)
        .force_incremental()
        .run(&circuit)
        .unwrap();

    assert_eq!(incremental.circuit, full.circuit);
    assert_eq!(incremental.stats, full.stats);
}

#[test]
fn incremental_matches_full_scan_in_control_flow_scope() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.h(q0)?;
            body.y(q0)?;
            body.h(q0)?;
            body.x(q0)?;
            body.x(q0)
        })
        .unwrap();
    let config = RewriteConfig::production();

    let full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(&circuit)
        .unwrap();
    let incremental = KnowledgeRewriter::new(config)
        .force_incremental()
        .run(&circuit)
        .unwrap();

    assert_eq!(incremental.circuit, full.circuit);
    assert_eq!(incremental.stats, full.stats);
}

#[test]
fn incremental_matches_full_scan_across_block_boundaries() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();
    circuit.h(q0).unwrap();
    circuit.barrier(vec![q0]).unwrap();
    circuit.x(q0).unwrap();
    circuit.x(q0).unwrap();
    let config = RewriteConfig::production();

    let full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(&circuit)
        .unwrap();
    let incremental = KnowledgeRewriter::new(config)
        .force_incremental()
        .run(&circuit)
        .unwrap();

    assert_eq!(incremental.circuit, full.circuit);
    assert_eq!(incremental.stats, full.stats);
}

#[test]
fn deterministic_randomized_incremental_matches_full_scan() {
    for seed in 0..16_u64 {
        let mut state = seed.wrapping_add(1);
        let mut circuit = Circuit::new(3);
        for _ in 0..64 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let qubit = Qubit::new(((state >> 32) % 3) as u32);
            match state % 7 {
                0 => circuit.h(qubit).unwrap(),
                1 => circuit.x(qubit).unwrap(),
                2 => circuit.y(qubit).unwrap(),
                3 => circuit.z(qubit).unwrap(),
                4 => circuit.s(qubit).unwrap(),
                5 => circuit.t(qubit).unwrap(),
                _ => circuit.rz(qubit, (state & 0xff) as f64 / 64.0).unwrap(),
            }
        }
        let config = RewriteConfig::production();
        let full = KnowledgeRewriter::new(config.clone())
            .force_full_scan()
            .run(&circuit)
            .unwrap();
        let incremental = KnowledgeRewriter::new(config)
            .force_incremental()
            .run(&circuit)
            .unwrap();
        assert_eq!(incremental.circuit, full.circuit, "seed={seed}");
        assert_eq!(incremental.stats, full.stats, "seed={seed}");
    }
}

/// Runs both execution engines over `circuit` and asserts bit-identical
/// results, including all four public statistics fields.
fn assert_incremental_matches_full_scan(config: RewriteConfig, circuit: &Circuit) {
    let full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(circuit)
        .unwrap();
    let incremental = KnowledgeRewriter::new(config)
        .force_incremental()
        .run(circuit)
        .unwrap();
    assert_eq!(incremental.circuit, full.circuit);
    assert_eq!(incremental.changed, full.changed);
    assert_eq!(incremental.stats, full.stats);
}

#[test]
fn incremental_matches_full_scan_with_symbolic_parameters() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    // merge_rx combines the pair into RX(θ + φ); the duplicate symbolic
    // parameters and the resulting expression must be interned identically
    // by both engines.
    circuit.rx(q0, Parameter::symbol("theta")).unwrap();
    circuit.rx(q0, Parameter::symbol("phi")).unwrap();
    circuit.rz(q0, Parameter::symbol("theta")).unwrap();
    circuit.rz(q0, Parameter::symbol("theta")).unwrap();

    assert_incremental_matches_full_scan(RewriteConfig::production(), &circuit);
}

#[test]
fn incremental_matches_full_scan_with_top_level_gphase_across_rounds() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    // A source-level GPhase is absorbed into the circuit global phase on
    // every round. The barrier makes the circuit ineligible for the linear
    // workspace, so this exercises the round-based workset engine's
    // full-scope fallback together with the H-Y-H global-phase replacement.
    circuit
        .append(
            Instruction::Standard(StandardGate::GPhase),
            std::iter::empty::<Qubit>(),
            [Parameter::from(0.25).into()],
            None,
        )
        .unwrap();
    circuit.h(q0).unwrap();
    circuit.y(q0).unwrap();
    circuit.h(q0).unwrap();
    circuit.barrier(vec![q0]).unwrap();
    circuit.x(q0).unwrap();
    circuit.x(q0).unwrap();

    assert_incremental_matches_full_scan(RewriteConfig::production(), &circuit);
}

#[test]
fn incremental_matches_full_scan_when_block_instruction_set_changes() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    // The first block keeps its X after both H gates cancel, so its
    // instruction set changes {H, X} -> {X} without the block disappearing:
    // the workset engine must mark it for a full rescan (BlockWorkset::Full)
    // because static filter results may change for every anchor.
    circuit.h(q0).unwrap();
    circuit.h(q0).unwrap();
    circuit.x(q0).unwrap();
    circuit.barrier(vec![q0]).unwrap();
    circuit.z(q0).unwrap();

    assert_incremental_matches_full_scan(RewriteConfig::production(), &circuit);
}

#[test]
fn incremental_matches_full_scan_when_dirty_ratio_forces_full_scan() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    // A long chain of mergeable rotations: round one merges adjacent pairs
    // everywhere, dirtying more than half of the eligible anchors and
    // forcing the dirty-ratio full-scan fallback on the following round.
    // Later rounds keep merging the shortened chain.
    for index in 0..64 {
        circuit.rz(q0, 0.0625 * (index % 4 + 1) as f64).unwrap();
    }

    assert_incremental_matches_full_scan(RewriteConfig::production(), &circuit);
}

#[test]
fn incremental_matches_full_scan_at_round_limits() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    // Sixteen mergeable rotations need more than four rounds to converge, so
    // small round limits truncate the fixpoint before stability.
    for _ in 0..16 {
        circuit.rz(q0, 0.125).unwrap();
    }

    for max_rounds in 1..=4u8 {
        let config = RewriteConfig::production().with_max_rounds(max_rounds);
        let full = KnowledgeRewriter::new(config.clone())
            .force_full_scan()
            .run(&circuit)
            .unwrap();
        let incremental = KnowledgeRewriter::new(config)
            .force_incremental()
            .run(&circuit)
            .unwrap();
        assert_eq!(incremental.circuit, full.circuit, "max_rounds={max_rounds}");
        assert_eq!(incremental.changed, full.changed, "max_rounds={max_rounds}");
        assert_eq!(incremental.stats, full.stats, "max_rounds={max_rounds}");
    }
}

// ---------------------------------------------------------------------------
// Two-qubit connectivity classification and the post-routing safety policy.
// ---------------------------------------------------------------------------

/// Parses a single-rule DSL source and returns its static two-qubit
/// connectivity classification.
fn classify_single_rule(dsl: &str) -> bool {
    let rules = load_rules_from_str(dsl).expect("rule DSL should parse and lower");
    assert_eq!(
        rules.len(),
        1,
        "expected exactly one rule, got {}",
        rules.len()
    );
    rule_preserves_two_qubit_connectivity(&rules[0])
}

#[test]
fn connectivity_accepts_two_qubit_direction_flip() {
    // CX 0 1 -> CX 1 0 keeps the same undirected pair {0,1}.
    assert!(classify_single_rule(
        "rule direction_flip { match { CX 0 1 } rewrite { CX 1 0 } }"
    ));
}

#[test]
fn connectivity_accepts_same_pair_gate_change() {
    // CX 0 1 -> CZ 0 1 keeps the same undirected pair {0,1}.
    assert!(classify_single_rule(
        "rule gate_change { match { CX 0 1 } rewrite { CZ 0 1 } }"
    ));
}

#[test]
fn connectivity_rejects_bridge_pair() {
    // The bridge collapse introduces the new pair {0,2}.
    assert!(!classify_single_rule(
        "rule bridge { match { CX 0 1, CX 1 2 } rewrite { CX 0 2 } }"
    ));
}

#[test]
fn connectivity_rejects_swap_bridge_pair() {
    // SWAP 0 1, CX 0 2 -> CX 1 2 introduces the new pair {1,2}.
    assert!(!classify_single_rule(
        "rule swap_bridge { match { SWAP 0 1, CX 0 2 } rewrite { CX 1 2 } }"
    ));
}

#[test]
fn connectivity_accepts_single_qubit_replacement() {
    // A target made only of one-qubit (and zero-qubit) operations adds no
    // two-qubit support.
    assert!(classify_single_rule(
        "rule one_qubit { match { X 0, X 0 } rewrite { Z 0, GPhase(π) } }"
    ));
}

#[test]
fn connectivity_rejects_three_qubit_target() {
    // A rewrite may not emit an operation on more than two qubits.
    assert!(!classify_single_rule(
        "rule three_qubit { match { CCX 0 1 2 } rewrite { CCX 0 1 2 } }"
    ));
}

#[test]
fn connectivity_rejects_shrinking_three_qubit_match_onto_subpair() {
    // CCX 0 1 2 -> CX 0 2 must be rejected: the three-qubit match does not
    // prove that {0,2} is a valid physical edge.
    assert!(!classify_single_rule(
        "rule shrink { match { CCX 0 1 2 } rewrite { CX 0 2 } }"
    ));
}

#[test]
fn builtin_library_has_exactly_six_connectivity_unsafe_production_rules() {
    // Tripwire: any newly added production rule (Simplify / Cancel / Merge /
    // Canonicalize) that introduces two-qubit connectivity must be classified
    // unsafe and show up here. Decompose and Commute rules are intentionally
    // excluded from the count: they also expand or move three-qubit gates and
    // are therefore classified unsafe, but they never run in the post-routing
    // production rewrite, so they are outside this contract.
    let library = RuleLibrary::builtin_rules().expect("builtin rules should load");
    let rules = library.rules();
    let mut unsafe_names: Vec<&str> = Vec::new();
    for kind in [
        RuleKind::Simplify,
        RuleKind::Cancel,
        RuleKind::Merge,
        RuleKind::Canonicalize,
    ] {
        for id in library.rules_by_kind(kind) {
            let rule = &rules[id.as_usize()];
            if !rule_preserves_two_qubit_connectivity(rule) {
                unsafe_names.push(rule.name.as_str());
            }
        }
    }
    unsafe_names.sort_unstable();

    let mut expected = vec![
        "compose_cx_bridge_4cx",
        "compose_cx_bridge_4cx_rev",
        "compose_swap_cx_bridge",
        "compose_swap_cx_bridge_far",
        "compose_swap_cz_bridge",
        "compose_swap_swap_swap",
    ];
    expected.sort_unstable();

    assert_eq!(unsafe_names, expected);
}

#[test]
fn preserve_connectivity_blocks_bridge_but_allows_same_pair_cancellation() {
    let (q0, q1, q2) = (Qubit::new(0), Qubit::new(1), Qubit::new(2));

    // Unrestricted production collapses the four-CX bridge to a single CX.
    let mut bridge = Circuit::new(3);
    bridge.cx(q0, q1).unwrap();
    bridge.cx(q1, q2).unwrap();
    bridge.cx(q0, q1).unwrap();
    bridge.cx(q1, q2).unwrap();
    let unrestricted = KnowledgeRewriter::new(RewriteConfig::production())
        .run(&bridge)
        .unwrap();
    assert_eq!(standard_ops(&unrestricted.circuit), vec![StandardGate::CX]);

    // With the policy enabled the bridge rule is filtered, so the four CX
    // operations remain untouched (no safe rule applies to this pattern).
    let preserved = KnowledgeRewriter::new(
        RewriteConfig::production().with_preserve_two_qubit_connectivity(true),
    )
    .run(&bridge)
    .unwrap();
    assert_eq!(standard_ops(&preserved.circuit), vec![StandardGate::CX; 4]);

    // Same-pair cancellation still fires under the policy.
    let mut cancel = Circuit::new(2);
    cancel.cx(q0, q1).unwrap();
    cancel.cx(q0, q1).unwrap();
    let cancelled = KnowledgeRewriter::new(
        RewriteConfig::production().with_preserve_two_qubit_connectivity(true),
    )
    .run(&cancel)
    .unwrap();
    assert!(cancelled.changed);
    assert!(cancelled.circuit.operations().is_empty());
}
