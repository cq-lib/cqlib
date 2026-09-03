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

use super::*;
use crate::circuit::{MCGate, StandardGate};

fn params_for_item(item: &RuleItem) -> Vec<Parameter> {
    item.params
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|value| match value {
            ParameterValue::Fixed(value) => Parameter::from(*value),
            ParameterValue::Param(parameter) => parameter.clone(),
        })
        .collect()
}

fn qargs_for_item(item: &RuleItem) -> Vec<Qubit> {
    item.qubits.iter().copied().map(Qubit::new).collect()
}

fn numeric_params(gate: StandardGate) -> Vec<Parameter> {
    (0..gate.num_params())
        .map(|index| Parameter::from(0.25 + index as f64 * 0.5))
        .collect()
}

fn canonical_qargs(gate: StandardGate) -> Vec<Qubit> {
    (0..gate.num_qubits())
        .map(|index| Qubit::new(index as u32))
        .collect()
}

fn linear_check(
    oracle: &RuleCommutationOracle,
    lhs_inst: &Instruction,
    lhs_qubits: &[Qubit],
    lhs_params: &[Parameter],
    rhs_inst: &Instruction,
    rhs_qubits: &[Qubit],
    rhs_params: &[Parameter],
) -> CommutationResult {
    for rule in &oracle.rules {
        if rule_matches(
            rule, lhs_inst, lhs_qubits, lhs_params, rhs_inst, rhs_qubits, rhs_params,
        ) || rule_matches(
            rule, rhs_inst, rhs_qubits, rhs_params, lhs_inst, lhs_qubits, lhs_params,
        ) {
            return Some(Commutation::Exact);
        }
    }
    None
}

#[test]
fn missing_instruction_pair_has_no_candidates() {
    let oracle = RuleCommutationOracle::builtin();
    let lhs = Instruction::Standard(StandardGate::H);
    let rhs = Instruction::Standard(StandardGate::U);
    let key = (
        KnowledgeInstructionKey::from_instruction(&lhs).unwrap(),
        KnowledgeInstructionKey::from_instruction(&rhs).unwrap(),
    );

    assert!(!oracle.rule_index.contains_key(&key));
    assert_eq!(
        oracle.check(
            &lhs,
            &[Qubit::new(0)],
            &[],
            &rhs,
            &[Qubit::new(0)],
            &[
                Parameter::from(0.1),
                Parameter::from(0.2),
                Parameter::from(0.3),
            ],
        ),
        None
    );
}

#[test]
fn every_builtin_rule_matches_in_both_query_directions() {
    let oracle = RuleCommutationOracle::builtin();

    for rule in &oracle.rules {
        let lhs = &rule.operations[0];
        let rhs = &rule.operations[1];
        let lhs_qargs = qargs_for_item(lhs);
        let rhs_qargs = qargs_for_item(rhs);
        let lhs_params = params_for_item(lhs);
        let rhs_params = params_for_item(rhs);

        assert_eq!(
            oracle.check(
                &lhs.instruction,
                &lhs_qargs,
                &lhs_params,
                &rhs.instruction,
                &rhs_qargs,
                &rhs_params,
            ),
            Some(Commutation::Exact),
            "forward query failed for {}",
            rule.name
        );
        assert_eq!(
            oracle.check(
                &rhs.instruction,
                &rhs_qargs,
                &rhs_params,
                &lhs.instruction,
                &lhs_qargs,
                &lhs_params,
            ),
            Some(Commutation::Exact),
            "reverse query failed for {}",
            rule.name
        );
    }
}

#[test]
fn same_instruction_pair_preserves_reverse_qubit_roles() {
    let first = RuleItem::standard(StandardGate::CX, &[0, 1], vec![]);
    let second = RuleItem::standard(StandardGate::CX, &[1, 2], vec![]);
    let rule = Rule::new(
        "same_key_reverse",
        vec![first.clone(), second.clone()],
        vec![second, first],
    );
    let oracle = RuleCommutationOracle::from_rules(&[rule]);
    let cx = Instruction::Standard(StandardGate::CX);

    assert_eq!(
        oracle.check(
            &cx,
            &[Qubit::new(1), Qubit::new(2)],
            &[],
            &cx,
            &[Qubit::new(0), Qubit::new(1)],
            &[],
        ),
        Some(Commutation::Exact)
    );
}

#[test]
fn indexed_rules_preserve_parameter_conditions() {
    let a = Parameter::symbol("a");
    let b = Parameter::symbol("b");
    let lhs = RuleItem::standard(StandardGate::RZ, &[0], vec![a.clone().into()]);
    let rhs = RuleItem::standard(StandardGate::Phase, &[0], vec![b.clone().into()]);
    let mut rule = Rule::new(
        "conditioned",
        vec![lhs.clone(), rhs.clone()],
        vec![rhs, lhs],
    );
    rule.conditions = Some(SmallVec::from_vec(vec![Condition::Eq(a, b)]));
    let oracle = RuleCommutationOracle::from_rules(&[rule]);
    let rz = Instruction::Standard(StandardGate::RZ);
    let phase = Instruction::Standard(StandardGate::Phase);
    let qargs = [Qubit::new(0)];

    assert_eq!(
        oracle.check(
            &rz,
            &qargs,
            &[Parameter::from(0.5)],
            &phase,
            &qargs,
            &[Parameter::from(0.5)],
        ),
        Some(Commutation::Exact)
    );
    assert_eq!(
        oracle.check(
            &rz,
            &qargs,
            &[Parameter::from(0.5)],
            &phase,
            &qargs,
            &[Parameter::from(0.75)],
        ),
        None
    );
}

#[test]
fn multi_controlled_instruction_keys_are_indexed() {
    let mcx = MCGate::new(2, StandardGate::X);
    let lhs = RuleItem::mc_gate(mcx.clone(), &[0, 1, 2], vec![]);
    let rhs = RuleItem::standard(StandardGate::Z, &[2], vec![]);
    let rule = Rule::new("mc_key", vec![lhs.clone(), rhs.clone()], vec![rhs, lhs]);
    let oracle = RuleCommutationOracle::from_rules(&[rule]);

    assert_eq!(
        oracle.check(
            &Instruction::McGate(Box::new(mcx)),
            &[Qubit::new(0), Qubit::new(1), Qubit::new(2)],
            &[],
            &Instruction::Standard(StandardGate::Z),
            &[Qubit::new(2)],
            &[],
        ),
        Some(Commutation::Exact)
    );
}

#[test]
fn indexed_lookup_matches_linear_scan_for_standard_gate_pairs() {
    let oracle = RuleCommutationOracle::builtin();

    for &lhs_gate in StandardGate::all() {
        let lhs_inst = Instruction::Standard(lhs_gate);
        let lhs_qargs = canonical_qargs(lhs_gate);
        let lhs_params = numeric_params(lhs_gate);
        for &rhs_gate in StandardGate::all() {
            let rhs_inst = Instruction::Standard(rhs_gate);
            let rhs_qargs = canonical_qargs(rhs_gate);
            let rhs_params = numeric_params(rhs_gate);
            let indexed = oracle.check(
                &lhs_inst,
                &lhs_qargs,
                &lhs_params,
                &rhs_inst,
                &rhs_qargs,
                &rhs_params,
            );
            let linear = linear_check(
                &oracle,
                &lhs_inst,
                &lhs_qargs,
                &lhs_params,
                &rhs_inst,
                &rhs_qargs,
                &rhs_params,
            );

            assert_eq!(
                indexed, linear,
                "indexed and linear lookup differ for {lhs_gate:?} and {rhs_gate:?}"
            );
        }
    }
}
