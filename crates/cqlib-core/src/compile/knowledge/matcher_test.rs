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
use crate::circuit::{
    Directive, Instruction, MCGate, Parameter, ParameterValue, Qubit, StandardGate,
};
use crate::compile::knowledge::rule::{Condition, Rule, RuleItem};

#[test]
fn instruction_keys_support_standard_and_multi_controlled_gates() {
    let x = Instruction::Standard(StandardGate::X);
    let mcx = Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X)));
    let barrier = Instruction::Directive(Directive::Barrier);

    assert_eq!(
        KnowledgeInstructionKey::from_instruction(&x),
        Some(KnowledgeInstructionKey::Standard(StandardGate::X))
    );
    assert_eq!(
        KnowledgeInstructionKey::from_instruction(&mcx),
        Some(KnowledgeInstructionKey::McGate(MCGate::new(
            2,
            StandardGate::X
        )))
    );
    for instruction in [&x, &mcx] {
        let key = KnowledgeInstructionKey::from_instruction(instruction).unwrap();
        let restored = key.to_instruction();
        assert_eq!(&restored, instruction);
        assert_eq!(
            KnowledgeInstructionKey::from_instruction(&restored),
            Some(key)
        );
    }
    assert!(KnowledgeInstructionKey::from_instruction(&barrier).is_none());
}

#[test]
fn matches_rule_item_with_one_to_one_qubit_bindings() {
    let item = RuleItem::standard(StandardGate::CX, &[0, 1], vec![]);
    let instruction = Instruction::Standard(StandardGate::CX);
    let qubits = [Qubit::new(3), Qubit::new(5)];
    let params = [];
    let mut bindings = MatchBindings::new();

    assert!(
        match_rule_item(
            &item,
            ConcreteOperationView::new(&instruction, &qubits, &params),
            &mut bindings
        )
        .unwrap()
    );
    assert_eq!(bindings.qubit(0), Some(Qubit::new(3)));
    assert_eq!(bindings.qubit(1), Some(Qubit::new(5)));
}

#[test]
fn rejects_reusing_concrete_qubit_for_different_rule_qubits() {
    let first = RuleItem::standard(StandardGate::H, &[0], vec![]);
    let second = RuleItem::standard(StandardGate::H, &[1], vec![]);
    let instruction = Instruction::Standard(StandardGate::H);
    let qubits = [Qubit::new(0)];
    let params = [];
    let mut bindings = MatchBindings::new();

    assert!(
        match_rule_item(
            &first,
            ConcreteOperationView::new(&instruction, &qubits, &params),
            &mut bindings
        )
        .unwrap()
    );
    assert!(
        !match_rule_item(
            &second,
            ConcreteOperationView::new(&instruction, &qubits, &params),
            &mut bindings
        )
        .unwrap()
    );
    assert!(bindings.qubit(1).is_none());
}

#[test]
fn failed_item_match_does_not_leave_partial_bindings() {
    let item = RuleItem::standard(
        StandardGate::RZ,
        &[0],
        vec![ParameterValue::Param(Parameter::symbol("theta"))],
    );
    let instruction = Instruction::Standard(StandardGate::RZ);
    let qubits = [Qubit::new(0)];
    let params = [Parameter::symbol("alpha")];
    let mut bindings = MatchBindings::new();

    assert!(
        match_rule_item(
            &item,
            ConcreteOperationView::new(&instruction, &qubits, &params),
            &mut bindings
        )
        .unwrap()
    );

    let conflicting = RuleItem::standard(
        StandardGate::RZ,
        &[1],
        vec![ParameterValue::Param(Parameter::symbol("theta"))],
    );
    let params = [Parameter::symbol("beta")];
    assert!(
        !match_rule_item(
            &conflicting,
            ConcreteOperationView::new(&instruction, &qubits, &params),
            &mut bindings
        )
        .unwrap()
    );

    assert_eq!(bindings.qubit(0), Some(Qubit::new(0)));
    assert!(bindings.qubit(1).is_none());
    assert_eq!(bindings.param("theta"), Some(&Parameter::symbol("alpha")));
}

#[test]
fn repeated_parameter_symbol_must_match_existing_binding() {
    let item = RuleItem::standard(
        StandardGate::RZ,
        &[0],
        vec![ParameterValue::Param(Parameter::symbol("theta"))],
    );
    let instruction = Instruction::Standard(StandardGate::RZ);
    let qubits = [Qubit::new(0)];
    let mut bindings = MatchBindings::new();

    assert!(
        match_rule_item(
            &item,
            ConcreteOperationView::new(&instruction, &qubits, &[Parameter::from(0.5)]),
            &mut bindings,
        )
        .unwrap()
    );
    assert!(
        match_rule_item(
            &item,
            ConcreteOperationView::new(&instruction, &qubits, &[Parameter::from(0.5)]),
            &mut bindings,
        )
        .unwrap()
    );
    assert!(
        !match_rule_item(
            &item,
            ConcreteOperationView::new(&instruction, &qubits, &[Parameter::from(0.75)]),
            &mut bindings,
        )
        .unwrap()
    );
}

#[test]
fn expression_parameter_matches_after_dependencies_are_bound() {
    let bind_a = RuleItem::standard(
        StandardGate::RZ,
        &[0],
        vec![ParameterValue::Param(Parameter::symbol("a"))],
    );
    let match_sum = RuleItem::standard(
        StandardGate::RZ,
        &[0],
        vec![ParameterValue::Param(
            Parameter::symbol("a") + Parameter::from(0.25),
        )],
    );
    let instruction = Instruction::Standard(StandardGate::RZ);
    let qubits = [Qubit::new(0)];
    let mut bindings = MatchBindings::new();

    assert!(
        !match_rule_item(
            &match_sum,
            ConcreteOperationView::new(&instruction, &qubits, &[Parameter::from(0.75)]),
            &mut bindings,
        )
        .unwrap()
    );
    assert!(
        match_rule_item(
            &bind_a,
            ConcreteOperationView::new(&instruction, &qubits, &[Parameter::from(0.5)]),
            &mut bindings,
        )
        .unwrap()
    );
    assert!(
        match_rule_item(
            &match_sum,
            ConcreteOperationView::new(&instruction, &qubits, &[Parameter::from(0.75)]),
            &mut bindings,
        )
        .unwrap()
    );
}

#[test]
fn conditions_require_bound_symbols_and_support_modulo() {
    let item = RuleItem::standard(
        StandardGate::RZ,
        &[0],
        vec![ParameterValue::Param(Parameter::symbol("theta"))],
    );
    let instruction = Instruction::Standard(StandardGate::RZ);
    let qubits = [Qubit::new(0)];
    let mut bindings = MatchBindings::new();
    let conditions = [Condition::EqMod(
        Parameter::symbol("theta"),
        Parameter::from(0.0),
        Parameter::from(std::f64::consts::PI),
    )];

    assert!(!conditions_hold(Some(&conditions), &bindings));
    assert!(
        match_rule_item(
            &item,
            ConcreteOperationView::new(
                &instruction,
                &qubits,
                &[Parameter::from(2.0 * std::f64::consts::PI)],
            ),
            &mut bindings,
        )
        .unwrap()
    );
    assert!(conditions_hold(Some(&conditions), &bindings));
}

#[test]
fn instantiate_target_maps_qubits_and_substitutes_parameters() {
    let rule = Rule::new(
        "merge_rz",
        vec![
            RuleItem::standard(
                StandardGate::RZ,
                &[0],
                vec![ParameterValue::Param(Parameter::symbol("a"))],
            ),
            RuleItem::standard(
                StandardGate::RZ,
                &[0],
                vec![ParameterValue::Param(Parameter::symbol("b"))],
            ),
        ],
        vec![RuleItem::standard(
            StandardGate::RZ,
            &[0],
            vec![ParameterValue::Param(
                Parameter::symbol("a") + Parameter::symbol("b"),
            )],
        )],
    );
    let instruction = Instruction::Standard(StandardGate::RZ);
    let qubits = [Qubit::new(2)];
    let first_params = [Parameter::from(0.25)];
    let second_params = [Parameter::from(0.5)];
    let operations = [
        ConcreteOperationView::new(&instruction, &qubits, &first_params),
        ConcreteOperationView::new(&instruction, &qubits, &second_params),
    ];

    let bindings = rule_matches_operations(&rule, &operations)
        .unwrap()
        .expect("rule should match");
    let replacements = instantiate_target(&rule.target, &bindings).unwrap();

    assert_eq!(replacements.len(), 1);
    assert_eq!(replacements[0].qubits.as_slice(), &[Qubit::new(2)]);
    assert_eq!(
        replacements[0].key,
        KnowledgeInstructionKey::Standard(StandardGate::RZ)
    );
    assert!(matches!(
        replacements[0].params[0],
        ParameterValue::Fixed(value) if (value - 0.75).abs() < 1e-12
    ));
}

#[test]
fn instantiate_target_rejects_unbound_rewrite_references() {
    let bindings = MatchBindings::new();
    let unbound_qubit = [RuleItem::standard(StandardGate::H, &[0], vec![])];
    let unbound_symbol = [RuleItem::standard(
        StandardGate::RZ,
        &[],
        vec![ParameterValue::Param(Parameter::symbol("theta"))],
    )];

    assert_eq!(
        instantiate_target(&unbound_qubit, &bindings).unwrap_err(),
        MatchError::UnboundRewriteQubit { qubit: 0 }
    );
    assert_eq!(
        instantiate_target(&unbound_symbol, &bindings).unwrap_err(),
        MatchError::UnboundRewriteSymbol {
            symbol: "theta".to_string(),
        }
    );
}

#[test]
fn rule_matches_operations_returns_none_for_non_adjacent_policy_cases() {
    let rule = Rule::new(
        "cancel_h",
        vec![
            RuleItem::standard(StandardGate::H, &[0], vec![]),
            RuleItem::standard(StandardGate::H, &[0], vec![]),
        ],
        vec![],
    );
    let h = Instruction::Standard(StandardGate::H);
    let x = Instruction::Standard(StandardGate::X);
    let qubits = [Qubit::new(0)];
    let params = [];

    let adjacent = [
        ConcreteOperationView::new(&h, &qubits, &params),
        ConcreteOperationView::new(&h, &qubits, &params),
    ];
    let interrupted = [
        ConcreteOperationView::new(&h, &qubits, &params),
        ConcreteOperationView::new(&x, &qubits, &params),
    ];

    assert!(rule_matches_operations(&rule, &adjacent).unwrap().is_some());
    assert!(
        rule_matches_operations(&rule, &interrupted)
            .unwrap()
            .is_none()
    );
}
