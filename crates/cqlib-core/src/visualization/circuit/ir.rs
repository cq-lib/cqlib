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

//! Visualization IR data model and transformations.
//!
//! This module defines backend-agnostic intermediate representation (IR) types used by
//! visualization backends such as text and figure drawers. It also contains small IR
//! transformations shared by those backends.
//!
//! # Design Goals
//!
//! - Decouple circuit semantics from rendering implementation.
//! - Preserve lane/column layout decisions for reuse across backends.
//! - Carry enough metadata (style/children/span) for control-flow-aware drawing.

use crate::circuit::Qubit;
use std::collections::BTreeSet;

/// Draw style used by visualization backends.
///
/// Each variant determines how one [`VisualOperation`] should be rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualOpStyle {
    /// Generic gate-like box.
    Gate,
    /// Controlled operation where the first `num_controls` operands are controls.
    Controlled {
        /// Number of control qubits at the start of the operand list.
        num_controls: usize,
    },
    /// Controlled-Z marker rendered as two dots connected vertically.
    Cz,
    /// Swap marker across two qubits.
    Swap,
    /// Barrier marker.
    Barrier,
    /// Measurement marker.
    Measure,
    /// Reset marker.
    Reset,
    /// Delay marker.
    Delay,
    /// Control-flow marker (if/while).
    ControlFlow {
        /// Specific control-flow marker family rendered by backends.
        kind: VisualControlFlowKind,
    },
}

/// Control-flow operation family used by visualization backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualControlFlowKind {
    /// Source IfElse block before flattening.
    IfElseBlock {
        /// Whether the source block includes a false branch.
        has_false_branch: bool,
        /// Display metadata for the branch condition.
        condition: VisualCondition,
    },
    /// Source While-loop block before flattening.
    WhileBlock {
        /// Display metadata for the loop condition.
        condition: VisualCondition,
    },
    /// Source For-loop block before flattening.
    ForBlock {
        /// Display metadata for the loop range expression.
        range: VisualCondition,
    },
    /// Source Switch block before flattening.
    SwitchBlock {
        /// Display metadata for the switch target expression.
        target: VisualCondition,
    },
    /// Structured break marker.
    Break,
    /// Structured continue marker.
    Continue,
    /// Flattened marker: `If ...`.
    IfStart,
    /// Flattened marker: `Else-...`.
    ElseStart,
    /// Flattened marker: `While ...`.
    WhileStart,
    /// Flattened marker: `For ...`.
    ForStart,
    /// Flattened marker: `Switch ...`.
    SwitchStart,
    /// Flattened marker: `Case ...`.
    CaseStart,
    /// Flattened marker: `Default-...`.
    DefaultStart,
    /// Flattened marker: `End-...`.
    End,
}

/// Structured condition metadata used by control-flow visualization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualCondition {
    /// Display label for the classical expression controlling the branch.
    pub label: String,
}

/// Backend-agnostic operation prepared for rendering.
///
/// This is the atomic IR node consumed by drawers.
#[derive(Debug, Clone)]
pub struct VisualOperation {
    /// Time column after layering.
    pub column: usize,
    /// Operand lanes (in original operand order).
    pub lanes: Vec<usize>,
    /// Lanes that reserve this column to avoid overlap.
    pub covered_lanes: Vec<usize>,
    /// Primary display label.
    pub label: String,
    /// Parameter labels, already formatted.
    pub params: Vec<String>,
    /// Rendering style.
    pub style: VisualOpStyle,
    /// If true, render this operation as a span box on multi-qubit lanes.
    pub span_box: bool,
    /// Optional child circuits for control-flow operations.
    pub children: Option<VisualChildren>,
    /// Number of logical columns reserved by this operation.
    pub span_cols: usize,
}

/// Backend-agnostic circuit after layout.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::circuit::Qubit;
/// use cqlib_core::visualization::{VisualCircuit, VisualOpStyle, VisualOperation};
///
/// let visual = VisualCircuit {
///     qubits: vec![Qubit::new(0)],
///     operations: vec![VisualOperation {
///         column: 0,
///         lanes: vec![0],
///         covered_lanes: vec![0],
///         label: "H".to_string(),
///         params: vec![],
///         style: VisualOpStyle::Gate,
///         span_box: false,
///         children: None,
///         span_cols: 1,
///     }],
///     num_columns: 1,
/// };
/// assert_eq!(visual.num_qubits(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct VisualCircuit {
    /// Logical qubits in lane order.
    pub qubits: Vec<Qubit>,
    /// Layered operations.
    pub operations: Vec<VisualOperation>,
    /// Number of occupied columns.
    pub num_columns: usize,
}

impl VisualCircuit {
    /// Number of qubit lanes.
    pub fn num_qubits(&self) -> usize {
        self.qubits.len()
    }
}

/// Optional child circuits attached to control-flow operations.
///
/// These child circuits keep lane alignment with the parent visualization context.
#[derive(Debug, Clone)]
pub enum VisualChildren {
    /// IfElse branch children.
    IfElse {
        /// Child circuit executed when the branch condition is true.
        then_circuit: Box<VisualCircuit>,
        /// Optional child circuit executed when the branch condition is false.
        else_circuit: Option<Box<VisualCircuit>>,
    },
    /// While-loop body child.
    While {
        /// Child circuit executed inside the loop body.
        body_circuit: Box<VisualCircuit>,
    },
    /// For-loop body child.
    For {
        /// Child circuit executed inside the loop body.
        body_circuit: Box<VisualCircuit>,
    },
    /// Switch case/default children.
    Switch {
        /// Case label and child circuit pairs in source order.
        case_circuits: Vec<(String, Box<VisualCircuit>)>,
        /// Optional child circuit executed for the default case.
        default_circuit: Option<Box<VisualCircuit>>,
    },
}

/// Expand control-flow operations into timeline markers and body operations.
///
/// This mirrors text rendering semantics: `If/Else/End` and `While/End` markers
/// become explicit operations in the resulting visual circuit.
pub(crate) fn flatten_control_flow_visual(visual: &VisualCircuit) -> VisualCircuit {
    let mut flat_ops = Vec::new();
    let mut next_cf_id = 0usize;
    let root_scope_lanes: Vec<usize> = (0..visual.num_qubits()).collect();
    flatten_ops_recursive(visual, &mut next_cf_id, &mut flat_ops, &root_scope_lanes);

    let mut next_free = vec![0usize; visual.num_qubits()];
    let mut num_columns = 0usize;
    for op in &mut flat_ops {
        let schedule_lanes = scheduling_covered_lanes(op, visual.num_qubits());
        let column = schedule_lanes
            .iter()
            .filter_map(|lane| next_free.get(*lane).copied())
            .max()
            .unwrap_or(0);
        op.column = column;
        let span = op.span_cols.max(1);
        for lane in schedule_lanes {
            if let Some(slot) = next_free.get_mut(lane) {
                *slot = column + span;
            }
        }
        num_columns = num_columns.max(column + span);
    }

    VisualCircuit {
        qubits: visual.qubits.clone(),
        operations: flat_ops,
        num_columns,
    }
}

/// Reverse displayed qubit order for a pre-built visual circuit.
pub(crate) fn reverse_visual_lanes(mut visual: VisualCircuit) -> VisualCircuit {
    let n = visual.num_qubits();
    if n == 0 {
        return visual;
    }
    visual.qubits.reverse();
    for op in &mut visual.operations {
        for lane in &mut op.lanes {
            *lane = n - 1 - *lane;
        }
        for lane in &mut op.covered_lanes {
            *lane = n - 1 - *lane;
        }
    }
    visual
}

fn flatten_ops_recursive(
    visual: &VisualCircuit,
    next_cf_id: &mut usize,
    out: &mut Vec<VisualOperation>,
    scope_lanes: &[usize],
) {
    for op in &visual.operations {
        match &op.style {
            VisualOpStyle::ControlFlow {
                kind: VisualControlFlowKind::IfElseBlock { condition, .. },
            } => {
                let cf_id = *next_cf_id;
                *next_cf_id += 1;
                let covered = control_flow_marker_lanes(op, visual.num_qubits(), scope_lanes);
                let if_label = format!("If-{cf_id} {}", condition.label);
                out.push(make_control_marker(
                    if_label,
                    VisualControlFlowKind::IfStart,
                    covered.clone(),
                ));

                if let Some(VisualChildren::IfElse {
                    then_circuit,
                    else_circuit,
                }) = op.children.as_ref()
                {
                    flatten_ops_recursive(then_circuit, next_cf_id, out, &covered);
                    if let Some(else_body) = else_circuit {
                        out.push(make_control_marker(
                            format!("Else-{cf_id}"),
                            VisualControlFlowKind::ElseStart,
                            covered.clone(),
                        ));
                        flatten_ops_recursive(else_body, next_cf_id, out, &covered);
                    }
                }

                out.push(make_control_marker(
                    format!("End-{cf_id}"),
                    VisualControlFlowKind::End,
                    covered,
                ));
            }
            VisualOpStyle::ControlFlow {
                kind: VisualControlFlowKind::WhileBlock { condition },
            } => {
                let cf_id = *next_cf_id;
                *next_cf_id += 1;
                let covered = control_flow_marker_lanes(op, visual.num_qubits(), scope_lanes);
                let while_label = format!("While-{cf_id} {}", condition.label);
                out.push(make_control_marker(
                    while_label,
                    VisualControlFlowKind::WhileStart,
                    covered.clone(),
                ));
                if let Some(VisualChildren::While { body_circuit }) = op.children.as_ref() {
                    flatten_ops_recursive(body_circuit, next_cf_id, out, &covered);
                }
                out.push(make_control_marker(
                    format!("End-{cf_id}"),
                    VisualControlFlowKind::End,
                    covered,
                ));
            }
            VisualOpStyle::ControlFlow {
                kind: VisualControlFlowKind::ForBlock { range },
            } => {
                let cf_id = *next_cf_id;
                *next_cf_id += 1;
                let covered = control_flow_marker_lanes(op, visual.num_qubits(), scope_lanes);
                out.push(make_control_marker(
                    format!("For-{cf_id} {}", range.label),
                    VisualControlFlowKind::ForStart,
                    covered.clone(),
                ));
                if let Some(VisualChildren::For { body_circuit }) = op.children.as_ref() {
                    flatten_ops_recursive(body_circuit, next_cf_id, out, &covered);
                }
                out.push(make_control_marker(
                    format!("End-{cf_id}"),
                    VisualControlFlowKind::End,
                    covered,
                ));
            }
            VisualOpStyle::ControlFlow {
                kind: VisualControlFlowKind::SwitchBlock { target },
            } => {
                let cf_id = *next_cf_id;
                *next_cf_id += 1;
                let covered = control_flow_marker_lanes(op, visual.num_qubits(), scope_lanes);
                out.push(make_control_marker(
                    format!("Switch-{cf_id} {}", target.label),
                    VisualControlFlowKind::SwitchStart,
                    covered.clone(),
                ));
                if let Some(VisualChildren::Switch {
                    case_circuits,
                    default_circuit,
                }) = op.children.as_ref()
                {
                    for (case_label, case_circuit) in case_circuits {
                        out.push(make_control_marker(
                            format!("Case-{cf_id} {case_label}"),
                            VisualControlFlowKind::CaseStart,
                            covered.clone(),
                        ));
                        flatten_ops_recursive(case_circuit, next_cf_id, out, &covered);
                    }
                    if let Some(default_body) = default_circuit {
                        out.push(make_control_marker(
                            format!("Default-{cf_id}"),
                            VisualControlFlowKind::DefaultStart,
                            covered.clone(),
                        ));
                        flatten_ops_recursive(default_body, next_cf_id, out, &covered);
                    }
                }
                out.push(make_control_marker(
                    format!("End-{cf_id}"),
                    VisualControlFlowKind::End,
                    covered,
                ));
            }
            _ => {
                let mut clone = op.clone();
                clone.children = None;
                clone.span_cols = 1;
                clone.column = 0;
                if matches!(clone.style, VisualOpStyle::ControlFlow { .. }) {
                    let covered = control_flow_marker_lanes(op, visual.num_qubits(), scope_lanes);
                    clone.lanes = covered.clone();
                    clone.covered_lanes = covered;
                } else if clone.lanes.is_empty() {
                    clone.lanes = effective_covered_lanes(op, visual.num_qubits());
                }
                out.push(clone);
            }
        }
    }
}

fn make_control_marker(
    label: String,
    kind: VisualControlFlowKind,
    covered_lanes: Vec<usize>,
) -> VisualOperation {
    VisualOperation {
        column: 0,
        lanes: covered_lanes.clone(),
        covered_lanes,
        label,
        params: Vec::new(),
        style: VisualOpStyle::ControlFlow { kind },
        span_box: false,
        children: None,
        span_cols: 1,
    }
}

fn effective_covered_lanes(op: &VisualOperation, num_qubits: usize) -> Vec<usize> {
    if !op.covered_lanes.is_empty() {
        return op.covered_lanes.clone();
    }
    if !op.lanes.is_empty() {
        return op.lanes.clone();
    }
    (0..num_qubits).collect()
}

fn scheduling_covered_lanes(op: &VisualOperation, num_qubits: usize) -> Vec<usize> {
    if matches!(op.style, VisualOpStyle::ControlFlow { .. }) {
        return (0..num_qubits).collect();
    }
    effective_covered_lanes(op, num_qubits)
}

fn control_flow_marker_lanes(
    op: &VisualOperation,
    num_qubits: usize,
    scope_lanes: &[usize],
) -> Vec<usize> {
    let lanes = op
        .children
        .as_ref()
        .and_then(collect_children_used_lanes)
        .or_else(|| non_empty_lanes(&op.lanes))
        .or_else(|| non_empty_lanes(scope_lanes))
        .unwrap_or_else(|| effective_covered_lanes(op, num_qubits));

    normalize_lane_span(&lanes)
}

fn collect_children_used_lanes(children: &VisualChildren) -> Option<Vec<usize>> {
    let mut lanes = BTreeSet::new();
    match children {
        VisualChildren::IfElse {
            then_circuit,
            else_circuit,
        } => {
            collect_circuit_used_lanes(then_circuit, &mut lanes);
            if let Some(else_circuit) = else_circuit {
                collect_circuit_used_lanes(else_circuit, &mut lanes);
            }
        }
        VisualChildren::While { body_circuit } | VisualChildren::For { body_circuit } => {
            collect_circuit_used_lanes(body_circuit, &mut lanes);
        }
        VisualChildren::Switch {
            case_circuits,
            default_circuit,
        } => {
            for (_, case_circuit) in case_circuits {
                collect_circuit_used_lanes(case_circuit, &mut lanes);
            }
            if let Some(default_circuit) = default_circuit {
                collect_circuit_used_lanes(default_circuit, &mut lanes);
            }
        }
    }

    lanes_to_vec(lanes)
}

fn collect_circuit_used_lanes(visual: &VisualCircuit, lanes: &mut BTreeSet<usize>) {
    for op in &visual.operations {
        if let Some(children) = op.children.as_ref()
            && let Some(child_lanes) = collect_children_used_lanes(children)
        {
            lanes.extend(child_lanes);
            continue;
        }

        if !op.lanes.is_empty() {
            lanes.extend(op.lanes.iter().copied());
        } else if matches!(op.style, VisualOpStyle::Barrier) {
            lanes.extend(effective_covered_lanes(op, visual.num_qubits()));
        }
    }
}

fn non_empty_lanes(lanes: &[usize]) -> Option<Vec<usize>> {
    if lanes.is_empty() {
        None
    } else {
        Some(lanes.to_vec())
    }
}

fn lanes_to_vec(lanes: BTreeSet<usize>) -> Option<Vec<usize>> {
    if lanes.is_empty() {
        None
    } else {
        Some(lanes.into_iter().collect())
    }
}

fn normalize_lane_span(lanes: &[usize]) -> Vec<usize> {
    let Some(min_lane) = lanes.iter().copied().min() else {
        return Vec::new();
    };
    let max_lane = lanes.iter().copied().max().unwrap_or(min_lane);
    (min_lane..=max_lane).collect()
}
