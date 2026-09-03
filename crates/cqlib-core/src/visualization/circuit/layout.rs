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

//! Visualization IR builder.
//!
//! This module converts [`Circuit`] operations into backend-agnostic [`VisualCircuit`] IR.
//!
//! The builder is responsible for:
//! - mapping qubits to lanes,
//! - formatting parameter labels,
//! - classifying operation drawing styles,
//! - scheduling operations into non-overlapping columns.

use crate::circuit::gate::{Directive, Instruction, StandardGate};
use crate::circuit::{
    Circuit, ClassicalControlOp, ClassicalDataOp, ClassicalExpr, ClassicalExprKind, Operation,
    Qubit,
};
use crate::visualization::VisualizationError;
use crate::visualization::circuit::ir::{
    VisualChildren, VisualCircuit, VisualCondition, VisualControlFlowKind, VisualOpStyle,
    VisualOperation,
};
use crate::visualization::circuit::params::{ParameterFormatOptions, ParameterFormatter};
use std::collections::HashMap;

/// Build-time options for visualization IR.
///
/// These options affect IR construction only and are shared by text and figure backends.
#[derive(Debug, Clone, Copy)]
pub struct VisualBuildOptions {
    /// If true, decompose circuit-gates before layout.
    pub decompose_circuit_gates: bool,
    /// If true, reserve the full lane span (`min..=max`) for multi-qubit operations.
    pub reserve_full_span_for_multi_qubit: bool,
    /// Parameter label formatting options.
    pub parameter_format: ParameterFormatOptions,
}

impl Default for VisualBuildOptions {
    fn default() -> Self {
        Self {
            decompose_circuit_gates: false,
            reserve_full_span_for_multi_qubit: true,
            parameter_format: ParameterFormatOptions::default(),
        }
    }
}

/// Convert a circuit into backend-agnostic visualization IR.
///
/// # Arguments
///
/// * `circuit` - Source circuit to be visualized.
/// * `options` - Builder options controlling decomposition and lane reservation policy.
///
/// # Errors
///
/// Returns [`VisualizationError`] when operations reference unknown qubits or invalid parameter indices.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::circuit::{Circuit, Qubit};
/// use cqlib_core::visualization::{VisualBuildOptions, build_visual_circuit};
///
/// let mut circuit = Circuit::new(2);
/// circuit.h(Qubit::new(0)).unwrap();
/// circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
///
/// let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
/// assert_eq!(visual.num_qubits(), 2);
/// assert!(!visual.operations.is_empty());
/// ```
pub fn build_visual_circuit(
    circuit: &Circuit,
    options: &VisualBuildOptions,
) -> Result<VisualCircuit, VisualizationError> {
    let owned;
    let source = if options.decompose_circuit_gates {
        owned = circuit.decompose()?;
        &owned
    } else {
        circuit
    };

    build_visual_circuit_from_ops(source, &source.qubits(), source.operations(), options)
}

fn build_visual_circuit_from_ops(
    source: &Circuit,
    qubits: &[Qubit],
    ops: &[Operation],
    options: &VisualBuildOptions,
) -> Result<VisualCircuit, VisualizationError> {
    let qubit_to_lane: HashMap<Qubit, usize> = qubits
        .iter()
        .enumerate()
        .map(|(idx, q)| (*q, idx))
        .collect();

    let mut operations = Vec::with_capacity(ops.len());
    let mut next_free = vec![0usize; qubits.len()];
    let mut num_columns = 0usize;
    let parameter_formatter = ParameterFormatter::new(options.parameter_format);

    for op in ops {
        let lanes = map_lanes(op, &qubit_to_lane)?;
        let params = op
            .params
            .iter()
            .map(|param| parameter_formatter.format_circuit_param(source, param))
            .collect::<Result<Vec<_>, _>>()?;
        let (style, label, span_box) = classify_instruction(&op.instruction)?;
        let children = build_children_for_instruction(source, qubits, &op.instruction, options)?;
        let span_cols = estimate_span_cols(&style, children.as_ref());
        let covered_lanes = compute_covered_lanes(
            &lanes,
            &style,
            qubits.len(),
            options.reserve_full_span_for_multi_qubit,
        );

        let column = compute_column(&covered_lanes, &next_free);
        for lane in &covered_lanes {
            next_free[*lane] = column + span_cols;
        }
        num_columns = num_columns.max(column + span_cols);

        operations.push(VisualOperation {
            column,
            lanes,
            covered_lanes,
            label,
            params,
            style,
            span_box,
            children,
            span_cols,
        });
    }

    Ok(VisualCircuit {
        qubits: qubits.to_vec(),
        operations,
        num_columns,
    })
}

/// Build nested visual children for structured control-flow instructions.
///
/// Child circuits keep the parent's qubit lane list so branch/body drawings stay aligned
/// with the surrounding circuit, even when a child only touches a subset of qubits.
fn build_children_for_instruction(
    source: &Circuit,
    parent_qubits: &[Qubit],
    instruction: &Instruction,
    options: &VisualBuildOptions,
) -> Result<Option<VisualChildren>, VisualizationError> {
    match instruction {
        Instruction::ClassicalControl(flow) => {
            build_children_for_flow(source, parent_qubits, flow, options)
        }
        _ => Ok(None),
    }
}

/// Convert a classical-control operation into visual child IR.
///
/// The source circuit is reused for parameter lookup, while operations come from the
/// control-flow body being rendered.
fn build_children_for_flow(
    source: &Circuit,
    parent_qubits: &[Qubit],
    flow: &ClassicalControlOp,
    options: &VisualBuildOptions,
) -> Result<Option<VisualChildren>, VisualizationError> {
    match flow {
        ClassicalControlOp::If(gate) => {
            let then_circuit = build_visual_circuit_from_ops(
                source,
                parent_qubits,
                gate.then_body().operations(),
                options,
            )?;
            let else_circuit = if let Some(body) = gate.else_body() {
                Some(Box::new(build_visual_circuit_from_ops(
                    source,
                    parent_qubits,
                    body.operations(),
                    options,
                )?))
            } else {
                None
            };
            Ok(Some(VisualChildren::IfElse {
                then_circuit: Box::new(then_circuit),
                else_circuit,
            }))
        }
        ClassicalControlOp::While(gate) => {
            let body_circuit = build_visual_circuit_from_ops(
                source,
                parent_qubits,
                gate.body().operations(),
                options,
            )?;
            Ok(Some(VisualChildren::While {
                body_circuit: Box::new(body_circuit),
            }))
        }
        ClassicalControlOp::For(gate) => {
            let body_circuit = build_visual_circuit_from_ops(
                source,
                parent_qubits,
                gate.body().operations(),
                options,
            )?;
            Ok(Some(VisualChildren::For {
                body_circuit: Box::new(body_circuit),
            }))
        }
        ClassicalControlOp::Switch(gate) => {
            let mut case_circuits = Vec::with_capacity(gate.cases().len());
            for case in gate.cases() {
                case_circuits.push((
                    case.value().to_string(),
                    Box::new(build_visual_circuit_from_ops(
                        source,
                        parent_qubits,
                        case.body().operations(),
                        options,
                    )?),
                ));
            }
            let default_circuit = if let Some(body) = gate.default() {
                Some(Box::new(build_visual_circuit_from_ops(
                    source,
                    parent_qubits,
                    body.operations(),
                    options,
                )?))
            } else {
                None
            };
            Ok(Some(VisualChildren::Switch {
                case_circuits,
                default_circuit,
            }))
        }
        ClassicalControlOp::Break | ClassicalControlOp::Continue => Ok(None),
    }
}

/// Estimate how many logical columns a visual operation must reserve.
///
/// Ordinary operations occupy one column. Control-flow blocks reserve enough columns for
/// visible branch/body children plus small margins and separators so later operations do
/// not overlap the expanded drawing.
fn estimate_span_cols(style: &VisualOpStyle, children: Option<&VisualChildren>) -> usize {
    if !matches!(style, VisualOpStyle::ControlFlow { .. }) {
        return 1;
    }
    const MARGIN: usize = 2;
    const SEP: usize = 2;

    match children {
        Some(VisualChildren::IfElse {
            then_circuit,
            else_circuit,
        }) => {
            let then_cols = then_circuit.num_columns.max(1);
            let else_cols = else_circuit
                .as_ref()
                .map(|c| c.num_columns.max(1))
                .unwrap_or(1);
            MARGIN + then_cols + SEP + else_cols + MARGIN
        }
        Some(VisualChildren::While { body_circuit }) => {
            let body_cols = body_circuit.num_columns.max(1);
            MARGIN + body_cols + MARGIN
        }
        Some(VisualChildren::For { body_circuit }) => {
            let body_cols = body_circuit.num_columns.max(1);
            MARGIN + body_cols + MARGIN
        }
        Some(VisualChildren::Switch {
            case_circuits,
            default_circuit,
        }) => {
            let case_cols = case_circuits
                .iter()
                .map(|(_, circuit)| circuit.num_columns.max(1))
                .sum::<usize>();
            let default_cols = default_circuit
                .as_ref()
                .map(|circuit| circuit.num_columns.max(1))
                .unwrap_or(0);
            let branch_count = case_circuits.len() + usize::from(default_circuit.is_some());
            let separators = branch_count.saturating_sub(1) * SEP;
            MARGIN + case_cols + default_cols + separators + MARGIN
        }
        None => 3,
    }
}

/// Map operation qubits to visual lane indices.
///
/// The returned order matches the operation operand order, which is important for controls,
/// targets, and measurement labels.
fn map_lanes(
    op: &Operation,
    qubit_to_lane: &HashMap<Qubit, usize>,
) -> Result<Vec<usize>, VisualizationError> {
    let mut lanes = Vec::with_capacity(op.qubits.len());
    for qubit in &op.qubits {
        let lane = qubit_to_lane
            .get(qubit)
            .ok_or(VisualizationError::UnknownQubit(qubit.id()))?;
        lanes.push(*lane);
    }
    Ok(lanes)
}

/// Classify a circuit instruction into draw style, label, and span-box policy.
///
/// The classification is intentionally rendering-agnostic; text and figure backends consume
/// the same [`VisualOpStyle`] and labels.
fn classify_instruction(
    instruction: &Instruction,
) -> Result<(VisualOpStyle, String, bool), VisualizationError> {
    match instruction {
        Instruction::Standard(gate) => {
            if *gate == StandardGate::SWAP {
                return Ok((VisualOpStyle::Swap, "SWAP".to_string(), false));
            }
            if *gate == StandardGate::CZ {
                return Ok((VisualOpStyle::Cz, "CZ".to_string(), false));
            }
            let num_ctrls = gate.num_ctrl_qubits();
            if num_ctrls > 0 {
                Ok((
                    VisualOpStyle::Controlled {
                        num_controls: num_ctrls,
                    },
                    controlled_target_label_for_standard(*gate),
                    false,
                ))
            } else {
                Ok((VisualOpStyle::Gate, standard_gate_label(*gate), false))
            }
        }
        Instruction::McGate(gate) => {
            if gate.num_ctrl_qubits() == 1 && *gate.base_gate() == StandardGate::Z {
                Ok((VisualOpStyle::Cz, "CZ".to_string(), false))
            } else {
                Ok((
                    VisualOpStyle::Controlled {
                        num_controls: gate.num_ctrl_qubits(),
                    },
                    standard_gate_label(*gate.base_gate()),
                    false,
                ))
            }
        }
        Instruction::UnitaryGate(gate) => {
            let label = fallback_if_empty(gate.label(), "Unitary");
            Ok((VisualOpStyle::Gate, label, true))
        }
        Instruction::CircuitGate(gate) => {
            let label = fallback_if_empty(gate.name(), "Gate");
            Ok((VisualOpStyle::Gate, label, true))
        }
        Instruction::Directive(Directive::Barrier) => {
            Ok((VisualOpStyle::Barrier, "B".to_string(), false))
        }
        Instruction::Directive(Directive::Measure) => {
            Ok((VisualOpStyle::Measure, "M".to_string(), false))
        }
        Instruction::Directive(Directive::Reset) => {
            Ok((VisualOpStyle::Reset, "R".to_string(), false))
        }
        Instruction::Delay => Ok((VisualOpStyle::Delay, "D".to_string(), false)),
        Instruction::ClassicalControl(flow) => {
            let (style, label) = classify_control_flow(flow);
            Ok((style, label, false))
        }
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
        | Instruction::ClassicalData(ClassicalDataOp::MeasureBits { .. }) => {
            Ok((VisualOpStyle::Measure, "M".to_string(), false))
        }
        Instruction::ClassicalData(ClassicalDataOp::Store { .. }) => {
            Ok((VisualOpStyle::Gate, "STORE".to_string(), true))
        }
    }
}

/// Use a stable fallback label for unnamed module-like gates.
fn fallback_if_empty(label: &str, default_label: &str) -> String {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        default_label.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Build a compact label and style marker for structured classical control flow.
fn classify_control_flow(flow: &ClassicalControlOp) -> (VisualOpStyle, String) {
    match flow {
        ClassicalControlOp::If(gate) => {
            let condition = VisualCondition {
                label: classical_expr_label(gate.condition()),
            };
            let has_false_branch = gate.else_body().is_some();
            (
                VisualOpStyle::ControlFlow {
                    kind: VisualControlFlowKind::IfElseBlock {
                        has_false_branch,
                        condition: condition.clone(),
                    },
                },
                format!("IF {}", condition.label),
            )
        }
        ClassicalControlOp::While(gate) => {
            let condition = VisualCondition {
                label: classical_expr_label(gate.condition()),
            };
            (
                VisualOpStyle::ControlFlow {
                    kind: VisualControlFlowKind::WhileBlock {
                        condition: condition.clone(),
                    },
                },
                format!("WH {}", condition.label),
            )
        }
        ClassicalControlOp::For(gate) => {
            let range = VisualCondition {
                label: format!(
                    "range({},{})",
                    classical_expr_label(gate.start()),
                    classical_expr_label(gate.stop()),
                ),
            };
            (
                VisualOpStyle::ControlFlow {
                    kind: VisualControlFlowKind::ForBlock {
                        range: range.clone(),
                    },
                },
                format!("FOR {}", range.label),
            )
        }
        ClassicalControlOp::Switch(gate) => {
            let target = VisualCondition {
                label: classical_expr_label(gate.target()),
            };
            (
                VisualOpStyle::ControlFlow {
                    kind: VisualControlFlowKind::SwitchBlock {
                        target: target.clone(),
                    },
                },
                format!("SW {}", target.label),
            )
        }
        ClassicalControlOp::Break => (
            VisualOpStyle::ControlFlow {
                kind: VisualControlFlowKind::Break,
            },
            "Break".to_string(),
        ),
        ClassicalControlOp::Continue => (
            VisualOpStyle::ControlFlow {
                kind: VisualControlFlowKind::Continue,
            },
            "Continue".to_string(),
        ),
    }
}

/// Format classical expressions for compact display in control-flow labels.
///
/// Unsupported expression shapes fall back to `Debug` so visualization remains available
/// while preserving enough detail for inspection.
fn classical_expr_label(expr: &ClassicalExpr) -> String {
    match expr.kind() {
        ClassicalExprKind::BoolLiteral(value) => value.to_string(),
        ClassicalExprKind::BitLiteral(value) => format!("bit({value})"),
        ClassicalExprKind::UIntLiteral { value, .. } => value.to_string(),
        ClassicalExprKind::BitVecLiteral { value, .. } => format!("bits({value})"),
        _ => format!("{expr:?}"),
    }
}

/// Choose the target label for controlled standard gates.
fn controlled_target_label_for_standard(gate: StandardGate) -> String {
    match gate {
        StandardGate::CX | StandardGate::CCX => "X".to_string(),
        StandardGate::CY => "Y".to_string(),
        StandardGate::CRX => "RX".to_string(),
        StandardGate::CRY => "RY".to_string(),
        StandardGate::CRZ => "RZ".to_string(),
        _ => strip_control_prefix(&standard_gate_label(gate)),
    }
}

/// Determine which lanes must be reserved for column scheduling.
///
/// Multi-qubit gates usually reserve their full vertical span so wires and boxes do not
/// collide with unrelated operations placed between their operands. Control-flow blocks
/// reserve every lane because their child region spans the full circuit display.
fn compute_covered_lanes(
    lanes: &[usize],
    style: &VisualOpStyle,
    num_qubits: usize,
    reserve_full_span: bool,
) -> Vec<usize> {
    if num_qubits == 0 {
        return Vec::new();
    }

    if matches!(style, VisualOpStyle::Barrier) {
        if lanes.is_empty() {
            return (0..num_qubits).collect();
        }
        if reserve_full_span && lanes.len() > 1 {
            let min_lane = lanes.iter().copied().min().unwrap_or(0);
            let max_lane = lanes.iter().copied().max().unwrap_or(min_lane);
            return (min_lane..=max_lane).collect();
        }
        return lanes.to_vec();
    }

    if matches!(style, VisualOpStyle::ControlFlow { .. }) {
        return (0..num_qubits).collect();
    }

    if lanes.is_empty() {
        return vec![0];
    }

    if reserve_full_span && lanes.len() > 1 {
        let min_lane = lanes.iter().copied().min().unwrap_or(0);
        let max_lane = lanes.iter().copied().max().unwrap_or(min_lane);
        return (min_lane..=max_lane).collect();
    }

    lanes.to_vec()
}

/// Find the earliest column that is free on every covered lane.
fn compute_column(covered_lanes: &[usize], next_free: &[usize]) -> usize {
    let mut column = 0usize;
    for lane in covered_lanes {
        column = column.max(next_free[*lane]);
    }
    column
}

/// Public-facing gate label used by visualization backends.
fn standard_gate_label(gate: StandardGate) -> String {
    match gate {
        StandardGate::SDG => "SD".to_string(),
        StandardGate::TDG => "TD".to_string(),
        StandardGate::Phase | StandardGate::GPhase => "P".to_string(),
        _ => gate.to_string(),
    }
}

/// Remove leading control markers from labels such as `CRX` when drawing the target box.
fn strip_control_prefix(label: &str) -> String {
    let mut out = label.to_string();
    while out.starts_with('C') && out.len() > 1 {
        out.remove(0);
    }
    out
}
