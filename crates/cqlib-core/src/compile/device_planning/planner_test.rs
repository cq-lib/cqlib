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

use super::super::session::{
    DevicePlanningSessionCache, populate_canonical_cache, prepare_cache_baseline,
};
use super::*;
use crate::compile::knowledge::rule::{Rule, RuleItem};
use crate::device::{EdgeProp, InstructionProp, PhysicalQubit, QubitProp};

fn build_planner<'a>(
    device: &'a Device,
    library: &RuleLibrary,
    roots: impl IntoIterator<Item = DeviceGateState>,
) -> Result<DevicePlanner<'a>, DevicePlannerError> {
    let physical_qubits = device.usable_qubits().collect::<Vec<_>>();
    DevicePlanner::build_with_estimator(
        device,
        library,
        roots,
        Arc::new(CalibrationEstimator::from_device(device, &physical_qubits)),
    )
}

fn build_planner_with_budget<'a>(
    device: &'a Device,
    library: &RuleLibrary,
    roots: impl IntoIterator<Item = DeviceGateState>,
    budget: PlannerBudget,
) -> Result<DevicePlanner<'a>, DevicePlannerError> {
    let physical_qubits = device.usable_qubits().collect::<Vec<_>>();
    DevicePlanner::build_with_estimator_and_budget(
        device,
        library,
        roots,
        budget,
        Arc::new(CalibrationEstimator::from_device(device, &physical_qubits)),
    )
}

fn plan_for(planner: &DevicePlanner<'_>, state: &DeviceGateState) -> Option<PlanChoice> {
    planner
        .state_ids
        .get(state)
        .and_then(|id| planner.plans[*id])
}

fn item(gate: StandardGate, qubits: &[u32]) -> RuleItem {
    RuleItem::standard(gate, qubits, Vec::new())
}

fn rule(name: &str, source: StandardGate, source_qubits: &[u32], target: Vec<RuleItem>) -> Rule {
    Rule::new(name, vec![item(source, source_qubits)], target)
}

fn state(gate: StandardGate, qargs: &[u32]) -> DeviceGateState {
    DeviceGateState::standard(
        gate,
        qargs.iter().copied().map(PhysicalQubit::new).collect(),
    )
}

fn library(rules: Vec<Rule>) -> RuleLibrary {
    RuleLibrary::from_rules(rules, RuleKind::Decompose).unwrap()
}

fn assert_rule_equivalent(rule: &Rule) {
    assert!(rule.verify().unwrap().is_verified());
}

#[test]
fn dead_two_state_cycle_terminates_without_a_plan() {
    let library = library(vec![
        rule(
            "x_to_h",
            StandardGate::X,
            &[0],
            vec![item(StandardGate::H, &[0])],
        ),
        rule(
            "h_to_x",
            StandardGate::H,
            &[0],
            vec![item(StandardGate::X, &[0])],
        ),
    ]);
    let device = Device::line("dead-cycle", 1).unwrap();
    let root = state(StandardGate::X, &[0]);

    let planner = build_planner(&device, &library, [root.clone()]).unwrap();

    assert!(plan_for(&planner, &root).is_none());
}

#[test]
fn cycle_with_native_exit_propagates_a_finite_plan() {
    let library = library(vec![
        rule(
            "x_to_h",
            StandardGate::X,
            &[0],
            vec![item(StandardGate::H, &[0])],
        ),
        rule(
            "h_to_x",
            StandardGate::H,
            &[0],
            vec![item(StandardGate::X, &[0])],
        ),
    ]);
    let device = Device::line("cycle-exit", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();
    let root = state(StandardGate::X, &[0]);

    let planner = build_planner(&device, &library, [root.clone()]).unwrap();

    assert!(matches!(
        plan_for(&planner, &root),
        Some(PlanChoice::Template(PlanTemplate::Rule(_)))
    ));
}

#[test]
fn repeated_child_occurrences_contribute_multiplicity_to_cost() {
    let library = library(vec![rule(
        "swap_to_three_cx",
        StandardGate::SWAP,
        &[0, 1],
        vec![
            item(StandardGate::CX, &[0, 1]),
            item(StandardGate::CX, &[0, 1]),
            item(StandardGate::CX, &[0, 1]),
        ],
    )]);
    let device = Device::line("multiplicity", 2)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap();
    let root = state(StandardGate::SWAP, &[0, 1]);

    let planner = build_planner(&device, &library, [root.clone()]).unwrap();
    let root_id = planner.state_ids[&root];

    assert_eq!(
        planner.costs[root_id],
        Some(DevicePlanCost {
            two_qubit_ops: 3,
            total_ops: 3,
            derivation_steps: 1,
        })
    );
    let selected = planner.selected[root_id].unwrap();
    assert_eq!(
        planner.nodes[selected.0]
            .physical_cost
            .native_two_qubit_depth,
        3
    );
}

#[test]
fn equal_shape_plans_select_lower_calibrated_error() {
    let library = RuleLibrary::new();
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let mut device = Device::bidirectional_line("calibrated-plan", 2).unwrap();
    for physical in [p0, p1] {
        device
            .add_qubit_properties(
                physical,
                QubitProp::new(0.0)
                    .with_native_instruction(InstructionProp::new(
                        Instruction::Standard(StandardGate::H),
                        0.001,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }
    for (control, target, error) in [(p0, p1, 0.20), (p1, p0, 0.01)] {
        device
            .add_edge_properties(
                control,
                target,
                EdgeProp::new()
                    .with_native_instruction(InstructionProp::new(
                        Instruction::Standard(StandardGate::CX),
                        error,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }
    let root = state(StandardGate::CX, &[0, 1]);

    let planner = build_planner(&device, &library, [root.clone()]).unwrap();
    assert!(matches!(
        plan_for(&planner, &root),
        Some(PlanChoice::Template(PlanTemplate::Direction(
            DirectionTemplate::Cx
        )))
    ));
}

#[test]
fn frontier_retains_two_qubit_count_fidelity_tradeoff() {
    let low_error = rule(
        "cx_to_three_cz",
        StandardGate::CX,
        &[0, 1],
        vec![
            item(StandardGate::H, &[1]),
            item(StandardGate::CZ, &[0, 1]),
            item(StandardGate::CZ, &[0, 1]),
            item(StandardGate::CZ, &[0, 1]),
            item(StandardGate::H, &[1]),
        ],
    );
    assert_rule_equivalent(&low_error);
    let library = library(vec![low_error]);
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let mut device = Device::line("pareto-plan", 2).unwrap();
    for physical in [p0, p1] {
        device
            .add_qubit_properties(
                physical,
                QubitProp::new(0.0)
                    .with_native_instruction(InstructionProp::new(
                        Instruction::Standard(StandardGate::H),
                        0.001,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }
    device
        .add_edge_properties(
            p0,
            p1,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.30,
                ))
                .unwrap()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CZ),
                    0.01,
                ))
                .unwrap(),
        )
        .unwrap();
    let root = state(StandardGate::CX, &[0, 1]);

    let planner = build_planner(&device, &library, [root.clone()]).unwrap();
    let root_id = planner.state_ids[&root];
    assert_eq!(planner.frontiers[root_id].len(), 2);
    let selected = planner.selected[root_id].unwrap();
    assert_eq!(
        planner.nodes[selected.0].physical_cost.native_two_qubit_ops,
        1
    );
}

#[test]
fn parent_depth_keeps_equal_scalar_children_with_different_readiness_profiles() {
    let worse_in_parent = rule(
        "a_cx_tail_target",
        StandardGate::CX,
        &[0, 1],
        vec![
            item(StandardGate::H, &[1]),
            item(StandardGate::CZ, &[0, 1]),
            item(StandardGate::H, &[1]),
            item(StandardGate::I, &[0]),
            item(StandardGate::I, &[1]),
        ],
    );
    let better_in_parent = rule(
        "z_cx_tail_control",
        StandardGate::CX,
        &[0, 1],
        vec![
            item(StandardGate::H, &[1]),
            item(StandardGate::CZ, &[0, 1]),
            item(StandardGate::H, &[1]),
            item(StandardGate::I, &[0]),
            item(StandardGate::I, &[0]),
        ],
    );
    let parent = rule(
        "cy_via_cx",
        StandardGate::CY,
        &[0, 1],
        vec![
            item(StandardGate::SDG, &[1]),
            item(StandardGate::CX, &[0, 1]),
            item(StandardGate::S, &[1]),
        ],
    );
    for rule in [&worse_in_parent, &better_in_parent, &parent] {
        assert_rule_equivalent(rule);
    }
    let library = library(vec![worse_in_parent, better_in_parent, parent]);
    let device = Device::line("schedule-profile", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::I),
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::S),
            Instruction::Standard(StandardGate::SDG),
            Instruction::Standard(StandardGate::CZ),
        ])
        .unwrap();
    let child = state(StandardGate::CX, &[0, 1]);
    let root = state(StandardGate::CY, &[0, 1]);

    let planner = build_planner(&device, &library, [root.clone()]).unwrap();
    let child_id = planner.state_ids[&child];
    assert_eq!(planner.frontiers[child_id].len(), 2);
    let root_plan = planner.selected_plan_for(&root).unwrap();
    let cx_plan = planner.children_for_plan(root_plan).unwrap()[1];
    let PlanChoice::Template(PlanTemplate::Rule(rule_id)) =
        planner.choice_for_plan(cx_plan).unwrap()
    else {
        panic!("expected the parent to select a rule-backed CX plan");
    };

    assert_eq!(library.get(rule_id).unwrap().name, "z_cx_tail_control");
    assert_eq!(
        planner.nodes[root_plan.0].physical_cost.total_native_depth,
        5
    );

    // The parent legitimately selects the context-sensitive CX variant above,
    // while CX as an independent root has the opposite deterministic tie-break.
    // Exporting the parent's dependency tree must preserve both decisions.
    let standalone_cx = planner.selected_plan_for(&child).unwrap();
    let PlanChoice::Template(PlanTemplate::Rule(rule_id)) =
        planner.choice_for_plan(standalone_cx).unwrap()
    else {
        panic!("expected standalone CX to select a rule-backed plan");
    };
    assert_eq!(library.get(rule_id).unwrap().name, "a_cx_tail_target");

    let mut cache = DevicePlanningSessionCache::default();
    let mut memo = HashMap::new();
    populate_canonical_cache(&planner, [root.clone()], &mut cache, &mut memo).unwrap();

    let cached_parent = cache.selected_plan(&root).unwrap();
    let PlanChoice::Template(PlanTemplate::Rule(rule_id)) = cached_parent.children[1].choice else {
        panic!("expected cached parent to retain its contextual CX plan");
    };
    assert_eq!(library.get(rule_id).unwrap().name, "z_cx_tail_control");

    let cached_child = cache.selected_plan(&child).unwrap();
    let PlanChoice::Template(PlanTemplate::Rule(rule_id)) = cached_child.choice else {
        panic!("expected cached standalone CX to retain its canonical plan");
    };
    assert_eq!(library.get(rule_id).unwrap().name, "a_cx_tail_target");
    assert_eq!(
        cached_child.physical_cost,
        planner.cost_for_plan(standalone_cx).unwrap()
    );

    // Independent planner batches must merge to the same roots regardless of
    // which state warmed the session first. Existing canonical entries remain
    // pinned instead of being overwritten by a later parent's child variant.
    let physical_qubits = device.usable_qubits().collect::<Vec<_>>();
    let estimator = Arc::new(CalibrationEstimator::from_device(&device, &physical_qubits));
    let mut child_first = DevicePlanningSessionCache::default();
    prepare_cache_baseline(
        &device,
        &library,
        [child.clone()],
        Arc::clone(&estimator),
        &mut child_first,
    )
    .unwrap();
    let child_before_parent = child_first.selected_plan(&child).unwrap();
    prepare_cache_baseline(
        &device,
        &library,
        [root.clone()],
        estimator,
        &mut child_first,
    )
    .unwrap();
    assert!(Arc::ptr_eq(
        &child_before_parent,
        &child_first.selected_plan(&child).unwrap()
    ));
    let PlanChoice::Template(PlanTemplate::Rule(rule_id)) =
        child_first.selected_plan(&root).unwrap().children[1].choice
    else {
        panic!("expected child-first history to retain the contextual CX plan");
    };
    assert_eq!(library.get(rule_id).unwrap().name, "z_cx_tail_control");
}

#[test]
fn planner_budget_exhaustion_is_explicit() {
    let library = library(vec![rule(
        "x_to_h",
        StandardGate::X,
        &[0],
        vec![item(StandardGate::H, &[0])],
    )]);
    let device = Device::line("budget", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();
    let error = build_planner_with_budget(
        &device,
        &library,
        [state(StandardGate::X, &[0])],
        PlannerBudget {
            max_total_generated_candidates: 0,
            ..PlannerBudget::default()
        },
    )
    .err()
    .expect("zero generated-candidate budget must fail");

    assert!(matches!(
        &error,
        DevicePlannerError::ComplexityExceeded {
            resource: "total generated candidates",
            limit: 0,
            observed: 1,
        }
    ));
    assert!(matches!(
        error.into_compiler_error(),
        crate::compile::CompilerError::TransformFailed {
            name: "device_planning",
            ..
        }
    ));
}

#[test]
fn ordered_qargs_have_independent_capability_states() {
    let library = RuleLibrary::new();
    let device = Device::line("one-qubit", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::X)])
        .unwrap();
    let supported = state(StandardGate::X, &[0]);
    let unsupported = state(StandardGate::X, &[1]);

    let planner =
        build_planner(&device, &library, [supported.clone(), unsupported.clone()]).unwrap();

    assert_eq!(plan_for(&planner, &supported), Some(PlanChoice::Native));
    assert!(plan_for(&planner, &unsupported).is_none());
}

#[test]
fn cost_prefers_fewer_two_qubit_leaves_before_total_gate_count() {
    let library = library(vec![
        rule(
            "swap_to_cx",
            StandardGate::SWAP,
            &[0, 1],
            vec![item(StandardGate::CX, &[0, 1])],
        ),
        rule(
            "swap_to_h_pair",
            StandardGate::SWAP,
            &[0, 1],
            vec![item(StandardGate::H, &[0]), item(StandardGate::H, &[1])],
        ),
    ]);
    let device = Device::line("cost-order", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let root = state(StandardGate::SWAP, &[0, 1]);

    let planner = build_planner(&device, &library, [root.clone()]).unwrap();
    let Some(PlanChoice::Template(PlanTemplate::Rule(rule_id))) = plan_for(&planner, &root) else {
        panic!("expected a rule plan");
    };

    assert_eq!(library.get(rule_id).unwrap().name, "swap_to_h_pair");
}

#[test]
fn equal_cost_rule_choice_is_stable_across_library_order() {
    let make_rules = || {
        vec![
            rule(
                "a_cx_via_cz",
                StandardGate::CX,
                &[0, 1],
                vec![
                    item(StandardGate::H, &[1]),
                    item(StandardGate::CZ, &[0, 1]),
                    item(StandardGate::H, &[1]),
                ],
            ),
            rule(
                "b_cx_via_cz",
                StandardGate::CX,
                &[0, 1],
                vec![
                    item(StandardGate::H, &[1]),
                    item(StandardGate::CZ, &[0, 1]),
                    item(StandardGate::H, &[1]),
                ],
            ),
        ]
    };
    for rule in make_rules() {
        assert_rule_equivalent(&rule);
    }
    let mut reversed = make_rules();
    reversed.reverse();
    let libraries = [library(make_rules()), library(reversed)];
    let device = Device::line("stable-tie", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CZ),
        ])
        .unwrap();
    let root = state(StandardGate::CX, &[0, 1]);

    for library in &libraries {
        let planner = build_planner(&device, library, [root.clone()]).unwrap();
        let Some(PlanChoice::Template(PlanTemplate::Rule(rule_id))) = plan_for(&planner, &root)
        else {
            panic!("expected a rule plan");
        };
        assert_eq!(library.get(rule_id).unwrap().name, "a_cx_via_cz");
    }
}

#[test]
fn direction_template_preserves_exact_ordered_qargs() {
    let parent = state(StandardGate::CX, &[0, 1]);
    let children = DirectionTemplate::Cx.child_states(&parent);

    assert_eq!(children[2], state(StandardGate::CX, &[1, 0]));
    assert_eq!(
        children[0].ordered_qargs.as_slice(),
        &[PhysicalQubit::new(0)]
    );
    assert_eq!(
        children[1].ordered_qargs.as_slice(),
        &[PhysicalQubit::new(1)]
    );
}
