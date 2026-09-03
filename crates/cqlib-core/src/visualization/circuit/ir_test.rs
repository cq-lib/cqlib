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

//! Tests for the visualization data model module.

use super::*;
use crate::circuit::Qubit;
use crate::visualization::VisualCondition;
use crate::visualization::circuit::ir::flatten_control_flow_visual;

#[test]
fn test_visual_op_style_clone() {
    let style = VisualOpStyle::Gate;
    let cloned = style.clone();
    assert_eq!(style, cloned);

    let controlled = VisualOpStyle::Controlled { num_controls: 2 };
    let controlled_clone = controlled.clone();
    assert_eq!(controlled, controlled_clone);
}

#[test]
fn test_visual_op_style_equality() {
    assert_eq!(VisualOpStyle::Gate, VisualOpStyle::Gate);
    assert_eq!(
        VisualOpStyle::Controlled { num_controls: 1 },
        VisualOpStyle::Controlled { num_controls: 1 }
    );
    assert_ne!(
        VisualOpStyle::Controlled { num_controls: 1 },
        VisualOpStyle::Controlled { num_controls: 2 }
    );
}

#[test]
fn test_visual_op_style_all_variants() {
    let styles = vec![
        VisualOpStyle::Gate,
        VisualOpStyle::Controlled { num_controls: 1 },
        VisualOpStyle::Cz,
        VisualOpStyle::Swap,
        VisualOpStyle::Barrier,
        VisualOpStyle::Measure,
        VisualOpStyle::Reset,
        VisualOpStyle::Delay,
        VisualOpStyle::ControlFlow {
            kind: VisualControlFlowKind::IfStart,
        },
    ];

    for style in styles {
        let _cloned = style.clone();
        let _debug = format!("{style:?}");
    }
}

#[test]
fn test_visual_control_flow_kind_clone() {
    let if_else = VisualControlFlowKind::IfElseBlock {
        has_false_branch: true,
        condition: VisualCondition {
            label: "true".to_string(),
        },
    };
    let cloned = if_else.clone();
    assert_eq!(if_else, cloned);

    let while_block = VisualControlFlowKind::WhileBlock {
        condition: VisualCondition {
            label: "false".to_string(),
        },
    };
    let while_clone = while_block.clone();
    assert_eq!(while_block, while_clone);
}

#[test]
fn test_visual_control_flow_kind_equality() {
    let cond1 = VisualCondition {
        label: "true".to_string(),
    };
    let cond2 = VisualCondition {
        label: "true".to_string(),
    };
    let cond3 = VisualCondition {
        label: "false".to_string(),
    };

    assert_eq!(
        VisualControlFlowKind::IfElseBlock {
            has_false_branch: true,
            condition: cond1.clone()
        },
        VisualControlFlowKind::IfElseBlock {
            has_false_branch: true,
            condition: cond2.clone()
        }
    );

    assert_ne!(
        VisualControlFlowKind::IfElseBlock {
            has_false_branch: true,
            condition: cond1.clone()
        },
        VisualControlFlowKind::IfElseBlock {
            has_false_branch: false,
            condition: cond2.clone()
        }
    );

    assert_ne!(
        VisualControlFlowKind::WhileBlock { condition: cond1 },
        VisualControlFlowKind::WhileBlock { condition: cond3 }
    );
}

#[test]
fn test_visual_control_flow_kind_all_variants() {
    let kinds = vec![
        VisualControlFlowKind::IfElseBlock {
            has_false_branch: true,
            condition: VisualCondition {
                label: "true".to_string(),
            },
        },
        VisualControlFlowKind::IfElseBlock {
            has_false_branch: false,
            condition: VisualCondition {
                label: "false".to_string(),
            },
        },
        VisualControlFlowKind::WhileBlock {
            condition: VisualCondition {
                label: "loop".to_string(),
            },
        },
        VisualControlFlowKind::IfStart,
        VisualControlFlowKind::ElseStart,
        VisualControlFlowKind::WhileStart,
        VisualControlFlowKind::End,
    ];

    for kind in kinds {
        let _cloned = kind.clone();
        let _debug = format!("{kind:?}");
    }
}

#[test]
fn test_visual_condition_clone() {
    let cond = VisualCondition {
        label: "condition".to_string(),
    };
    let cloned = cond.clone();
    assert_eq!(cond, cloned);

    assert_eq!(cloned.label, "condition");
}

#[test]
fn test_visual_condition_equality() {
    let cond1 = VisualCondition {
        label: "true".to_string(),
    };
    let cond2 = VisualCondition {
        label: "true".to_string(),
    };
    let cond3 = VisualCondition {
        label: "false".to_string(),
    };

    assert_eq!(cond1, cond2);
    assert_ne!(cond1, cond3);
}

#[test]
fn test_visual_operation_clone() {
    let op = VisualOperation {
        column: 5,
        lanes: vec![0, 2],
        covered_lanes: vec![0, 1, 2],
        label: "CX".to_string(),
        params: vec!["π/2".to_string()],
        style: VisualOpStyle::Controlled { num_controls: 1 },
        span_box: true,
        children: None,
        span_cols: 2,
    };

    let cloned = op.clone();
    assert_eq!(op.column, cloned.column);
    assert_eq!(op.lanes, cloned.lanes);
    assert_eq!(op.covered_lanes, cloned.covered_lanes);
    assert_eq!(op.label, cloned.label);
    assert_eq!(op.params, cloned.params);
    assert_eq!(op.style, cloned.style);
    assert_eq!(op.span_box, cloned.span_box);
    assert_eq!(op.span_cols, cloned.span_cols);
}

#[test]
fn test_visual_operation_with_children() {
    let then_circuit = VisualCircuit {
        qubits: vec![Qubit::new(0)],
        operations: vec![],
        num_columns: 0,
    };

    let op = VisualOperation {
        column: 0,
        lanes: vec![0],
        covered_lanes: vec![0],
        label: "IF".to_string(),
        params: vec![],
        style: VisualOpStyle::ControlFlow {
            kind: VisualControlFlowKind::IfElseBlock {
                has_false_branch: false,
                condition: VisualCondition {
                    label: "true".to_string(),
                },
            },
        },
        span_box: false,
        children: Some(VisualChildren::IfElse {
            then_circuit: Box::new(then_circuit),
            else_circuit: None,
        }),
        span_cols: 3,
    };

    let cloned = op.clone();
    match cloned.children {
        Some(VisualChildren::IfElse {
            then_circuit,
            else_circuit,
        }) => {
            assert_eq!(then_circuit.num_qubits(), 1);
            assert!(else_circuit.is_none());
        }
        _ => panic!("expected IfElse children"),
    }
}

#[test]
fn test_visual_circuit_num_qubits() {
    let circuit = VisualCircuit {
        qubits: vec![Qubit::new(0), Qubit::new(1), Qubit::new(2)],
        operations: vec![],
        num_columns: 0,
    };
    assert_eq!(circuit.num_qubits(), 3);
}

#[test]
fn test_visual_circuit_empty() {
    let circuit = VisualCircuit {
        qubits: vec![],
        operations: vec![],
        num_columns: 0,
    };
    assert_eq!(circuit.num_qubits(), 0);
    assert!(circuit.operations.is_empty());
}

#[test]
fn test_visual_circuit_with_operations() {
    let ops = vec![
        VisualOperation {
            column: 0,
            lanes: vec![0],
            covered_lanes: vec![0],
            label: "H".to_string(),
            params: vec![],
            style: VisualOpStyle::Gate,
            span_box: false,
            children: None,
            span_cols: 1,
        },
        VisualOperation {
            column: 1,
            lanes: vec![0, 1],
            covered_lanes: vec![0, 1],
            label: "CX".to_string(),
            params: vec![],
            style: VisualOpStyle::Controlled { num_controls: 1 },
            span_box: false,
            children: None,
            span_cols: 1,
        },
    ];

    let circuit = VisualCircuit {
        qubits: vec![Qubit::new(0), Qubit::new(1)],
        operations: ops,
        num_columns: 2,
    };

    assert_eq!(circuit.num_qubits(), 2);
    assert_eq!(circuit.operations.len(), 2);
    assert_eq!(circuit.num_columns, 2);
}

#[test]
fn test_visual_children_clone() {
    let then_circuit = VisualCircuit {
        qubits: vec![Qubit::new(0)],
        operations: vec![],
        num_columns: 0,
    };
    let else_circuit = VisualCircuit {
        qubits: vec![Qubit::new(1)],
        operations: vec![],
        num_columns: 0,
    };

    let if_else = VisualChildren::IfElse {
        then_circuit: Box::new(then_circuit.clone()),
        else_circuit: Some(Box::new(else_circuit.clone())),
    };
    let cloned = if_else.clone();

    match cloned {
        VisualChildren::IfElse {
            then_circuit,
            else_circuit,
        } => {
            assert_eq!(then_circuit.num_qubits(), 1);
            assert!(else_circuit.is_some());
            assert_eq!(else_circuit.unwrap().num_qubits(), 1);
        }
        _ => panic!("expected IfElse"),
    }

    let while_body = VisualCircuit {
        qubits: vec![Qubit::new(0)],
        operations: vec![],
        num_columns: 0,
    };
    let while_children = VisualChildren::While {
        body_circuit: Box::new(while_body),
    };
    let while_cloned = while_children.clone();
    match while_cloned {
        VisualChildren::While { body_circuit } => {
            assert_eq!(body_circuit.num_qubits(), 1);
        }
        _ => panic!("expected While"),
    }
}

#[test]
fn test_visual_children_all_variants() {
    let circuit = VisualCircuit {
        qubits: vec![Qubit::new(0)],
        operations: vec![],
        num_columns: 0,
    };

    let if_else = VisualChildren::IfElse {
        then_circuit: Box::new(circuit.clone()),
        else_circuit: None,
    };
    let _if_else_debug = format!("{if_else:?}");

    let while_children = VisualChildren::While {
        body_circuit: Box::new(circuit),
    };
    let _while_debug = format!("{while_children:?}");
}

#[test]
fn test_visual_circuit_debug_format() {
    let circuit = VisualCircuit {
        qubits: vec![Qubit::new(0), Qubit::new(1)],
        operations: vec![],
        num_columns: 0,
    };
    let debug_str = format!("{circuit:?}");
    assert!(debug_str.contains("VisualCircuit"));
    assert!(debug_str.contains("num_columns: 0"));
}

#[test]
fn test_visual_operation_debug_format() {
    let op = VisualOperation {
        column: 0,
        lanes: vec![0],
        covered_lanes: vec![0],
        label: "H".to_string(),
        params: vec![],
        style: VisualOpStyle::Gate,
        span_box: false,
        children: None,
        span_cols: 1,
    };
    let debug_str = format!("{op:?}");
    assert!(debug_str.contains("VisualOperation"));
    assert!(debug_str.contains("H"));
}

#[test]
fn test_visual_condition_debug_format() {
    let cond = VisualCondition {
        label: "condition".to_string(),
    };
    let debug_str = format!("{cond:?}");
    assert!(debug_str.contains("VisualCondition"));
    assert!(debug_str.contains("condition"));
}

fn test_qubits(count: usize) -> Vec<Qubit> {
    (0..count)
        .map(|idx| Qubit::new(u32::try_from(idx).unwrap()))
        .collect()
}

fn test_visual_circuit(num_qubits: usize, operations: Vec<VisualOperation>) -> VisualCircuit {
    VisualCircuit {
        qubits: test_qubits(num_qubits),
        num_columns: operations.len(),
        operations,
    }
}

fn test_gate(lanes: Vec<usize>, label: &str) -> VisualOperation {
    VisualOperation {
        column: 0,
        covered_lanes: lanes.clone(),
        lanes,
        label: label.to_string(),
        params: Vec::new(),
        style: VisualOpStyle::Gate,
        span_box: false,
        children: None,
        span_cols: 1,
    }
}

fn test_control_transfer(
    label: &str,
    kind: VisualControlFlowKind,
    num_qubits: usize,
) -> VisualOperation {
    VisualOperation {
        column: 0,
        lanes: Vec::new(),
        covered_lanes: (0..num_qubits).collect(),
        label: label.to_string(),
        params: Vec::new(),
        style: VisualOpStyle::ControlFlow { kind },
        span_box: false,
        children: None,
        span_cols: 1,
    }
}

fn test_if_op(
    num_qubits: usize,
    lanes: Vec<usize>,
    then_circuit: VisualCircuit,
    else_circuit: Option<VisualCircuit>,
) -> VisualOperation {
    VisualOperation {
        column: 0,
        lanes,
        covered_lanes: (0..num_qubits).collect(),
        label: "IF true".to_string(),
        params: Vec::new(),
        style: VisualOpStyle::ControlFlow {
            kind: VisualControlFlowKind::IfElseBlock {
                has_false_branch: else_circuit.is_some(),
                condition: VisualCondition {
                    label: "true".to_string(),
                },
            },
        },
        span_box: false,
        children: Some(VisualChildren::IfElse {
            then_circuit: Box::new(then_circuit),
            else_circuit: else_circuit.map(Box::new),
        }),
        span_cols: 3,
    }
}

fn covered_for_label<'a>(visual: &'a VisualCircuit, label: &str) -> &'a [usize] {
    visual
        .operations
        .iter()
        .find(|op| op.label == label)
        .map(|op| op.covered_lanes.as_slice())
        .unwrap_or_else(|| panic!("missing flattened operation {label}"))
}

fn column_for_label(visual: &VisualCircuit, label: &str) -> usize {
    visual
        .operations
        .iter()
        .find(|op| op.label == label)
        .map(|op| op.column)
        .unwrap_or_else(|| panic!("missing flattened operation {label}"))
}

#[test]
fn flatten_if_markers_prefer_then_body_lanes() {
    let then_circuit =
        test_visual_circuit(3, vec![test_gate(vec![1], "X"), test_gate(vec![2], "Z")]);
    let visual = test_visual_circuit(3, vec![test_if_op(3, vec![1, 2], then_circuit, None)]);

    let flattened = flatten_control_flow_visual(&visual);

    assert_eq!(covered_for_label(&flattened, "If-0 true"), &[1, 2]);
    assert_eq!(covered_for_label(&flattened, "End-0"), &[1, 2]);
}

#[test]
fn flatten_if_else_markers_use_branch_lane_union_span() {
    let then_circuit = test_visual_circuit(3, vec![test_gate(vec![0], "X")]);
    let else_circuit = test_visual_circuit(3, vec![test_gate(vec![2], "Z")]);
    let visual = test_visual_circuit(
        3,
        vec![test_if_op(3, vec![0, 2], then_circuit, Some(else_circuit))],
    );

    let flattened = flatten_control_flow_visual(&visual);

    assert_eq!(covered_for_label(&flattened, "If-0 true"), &[0, 1, 2]);
    assert_eq!(covered_for_label(&flattened, "Else-0"), &[0, 1, 2]);
    assert_eq!(covered_for_label(&flattened, "End-0"), &[0, 1, 2]);
}

#[test]
fn flatten_switch_markers_use_all_case_body_lane_span() {
    let visual = test_visual_circuit(
        3,
        vec![VisualOperation {
            column: 0,
            lanes: vec![0, 1, 2],
            covered_lanes: vec![0, 1, 2],
            label: "SW 1".to_string(),
            params: Vec::new(),
            style: VisualOpStyle::ControlFlow {
                kind: VisualControlFlowKind::SwitchBlock {
                    target: VisualCondition {
                        label: "1".to_string(),
                    },
                },
            },
            span_box: false,
            children: Some(VisualChildren::Switch {
                case_circuits: vec![
                    (
                        "0".to_string(),
                        Box::new(test_visual_circuit(3, vec![test_gate(vec![0], "X")])),
                    ),
                    (
                        "1".to_string(),
                        Box::new(test_visual_circuit(3, vec![test_gate(vec![2], "Z")])),
                    ),
                ],
                default_circuit: Some(Box::new(test_visual_circuit(
                    3,
                    vec![test_gate(vec![1], "H")],
                ))),
            }),
            span_cols: 5,
        }],
    );

    let flattened = flatten_control_flow_visual(&visual);

    assert_eq!(covered_for_label(&flattened, "Switch-0 1"), &[0, 1, 2]);
    assert_eq!(covered_for_label(&flattened, "Case-0 0"), &[0, 1, 2]);
    assert_eq!(covered_for_label(&flattened, "Case-0 1"), &[0, 1, 2]);
    assert_eq!(covered_for_label(&flattened, "Default-0"), &[0, 1, 2]);
    assert_eq!(covered_for_label(&flattened, "End-0"), &[0, 1, 2]);
}

#[test]
fn flatten_while_ignores_empty_break_lanes_for_parent_span() {
    let body = test_visual_circuit(
        3,
        vec![
            test_gate(vec![0], "X"),
            test_control_transfer("Break", VisualControlFlowKind::Break, 3),
        ],
    );
    let visual = test_visual_circuit(
        3,
        vec![VisualOperation {
            column: 0,
            lanes: vec![0],
            covered_lanes: vec![0, 1, 2],
            label: "WH true".to_string(),
            params: Vec::new(),
            style: VisualOpStyle::ControlFlow {
                kind: VisualControlFlowKind::WhileBlock {
                    condition: VisualCondition {
                        label: "true".to_string(),
                    },
                },
            },
            span_box: false,
            children: Some(VisualChildren::While {
                body_circuit: Box::new(body),
            }),
            span_cols: 3,
        }],
    );

    let flattened = flatten_control_flow_visual(&visual);

    assert_eq!(covered_for_label(&flattened, "While-0 true"), &[0]);
    assert_eq!(covered_for_label(&flattened, "Break"), &[0]);
    assert_eq!(covered_for_label(&flattened, "End-0"), &[0]);
}

#[test]
fn flatten_control_markers_reserve_all_lanes_for_scheduling() {
    let body = test_visual_circuit(2, vec![test_gate(vec![0], "X")]);
    let visual = test_visual_circuit(
        2,
        vec![
            VisualOperation {
                column: 0,
                lanes: vec![0],
                covered_lanes: vec![0, 1],
                label: "WH true".to_string(),
                params: Vec::new(),
                style: VisualOpStyle::ControlFlow {
                    kind: VisualControlFlowKind::WhileBlock {
                        condition: VisualCondition {
                            label: "true".to_string(),
                        },
                    },
                },
                span_box: false,
                children: Some(VisualChildren::While {
                    body_circuit: Box::new(body),
                }),
                span_cols: 3,
            },
            test_gate(vec![1], "Z"),
        ],
    );

    let flattened = flatten_control_flow_visual(&visual);

    assert_eq!(covered_for_label(&flattened, "While-0 true"), &[0]);
    assert!(column_for_label(&flattened, "Z") > column_for_label(&flattened, "End-0"));
}

#[test]
fn flatten_empty_control_flow_keeps_fallback_span() {
    let visual = test_visual_circuit(
        3,
        vec![test_if_op(
            3,
            Vec::new(),
            test_visual_circuit(3, Vec::new()),
            None,
        )],
    );

    let flattened = flatten_control_flow_visual(&visual);

    assert_eq!(covered_for_label(&flattened, "If-0 true"), &[0, 1, 2]);
    assert_eq!(covered_for_label(&flattened, "End-0"), &[0, 1, 2]);
}
