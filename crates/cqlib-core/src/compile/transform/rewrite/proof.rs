// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of the License in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Rewrite-proof policy and its incrementally maintained gate summary.

use super::RewriteConfig;
use super::edit::OperationReplacement;
use crate::circuit::{Circuit, ClassicalControlOp, Instruction, Operation};

#[derive(Debug, Clone)]
pub(super) struct GateInstructionHistogram {
    top_level: Vec<(Instruction, usize)>,
    recursive: Vec<(Instruction, usize)>,
}

impl GateInstructionHistogram {
    pub(super) fn from_circuit(circuit: &Circuit) -> Self {
        Self {
            top_level: gate_instruction_counts(circuit.operations(), false),
            recursive: gate_instruction_counts(circuit.operations(), true),
        }
    }

    fn is_within_target(&self, target: &[Instruction], recursive: bool) -> bool {
        let entries = if recursive {
            &self.recursive
        } else {
            &self.top_level
        };
        entries
            .iter()
            .all(|(instruction, _)| target.contains(instruction))
    }

    pub(super) fn after_linear_replacements(
        &self,
        before: &Circuit,
        after: &Circuit,
        replacements: &[OperationReplacement],
    ) -> Option<Self> {
        let mut counts = self.top_level.clone();
        for replacement in replacements {
            for operation in &before.operations()[replacement.old.clone()] {
                if !adjust_gate_instruction_count(&mut counts, &operation.instruction, false) {
                    return None;
                }
            }
            for operation in &after.operations()[replacement.new.clone()] {
                adjust_gate_instruction_count(&mut counts, &operation.instruction, true);
            }
        }
        counts.retain(|(_, count)| *count > 0);
        Some(Self {
            top_level: counts.clone(),
            recursive: counts,
        })
    }
}

fn gate_instruction_counts(
    operations: &[Operation],
    recurse_control_flow: bool,
) -> Vec<(Instruction, usize)> {
    let mut counts = Vec::new();
    collect_gate_instruction_counts(operations, recurse_control_flow, &mut counts);
    counts
}

fn collect_gate_instruction_counts(
    operations: &[Operation],
    recurse_control_flow: bool,
    counts: &mut Vec<(Instruction, usize)>,
) {
    for operation in operations {
        match &operation.instruction {
            Instruction::Standard(_) | Instruction::McGate(_) => {
                adjust_gate_instruction_count(counts, &operation.instruction, true);
            }
            Instruction::ClassicalControl(control) if recurse_control_flow => {
                collect_control_flow_gate_counts(control, counts);
            }
            Instruction::ClassicalControl(_)
            | Instruction::ClassicalData(_)
            | Instruction::Directive(_)
            | Instruction::Delay
            | Instruction::UnitaryGate(_)
            | Instruction::CircuitGate(_) => {}
        }
    }
}

fn collect_control_flow_gate_counts(
    control: &ClassicalControlOp,
    counts: &mut Vec<(Instruction, usize)>,
) {
    match control {
        ClassicalControlOp::If(operation) => {
            collect_gate_instruction_counts(operation.then_body().operations(), true, counts);
            if let Some(body) = operation.else_body() {
                collect_gate_instruction_counts(body.operations(), true, counts);
            }
        }
        ClassicalControlOp::While(operation) => {
            collect_gate_instruction_counts(operation.body().operations(), true, counts);
        }
        ClassicalControlOp::For(operation) => {
            collect_gate_instruction_counts(operation.body().operations(), true, counts);
        }
        ClassicalControlOp::Switch(operation) => {
            for case in operation.cases() {
                collect_gate_instruction_counts(case.body().operations(), true, counts);
            }
            if let Some(body) = operation.default() {
                collect_gate_instruction_counts(body.operations(), true, counts);
            }
        }
        ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
    }
}

fn adjust_gate_instruction_count(
    counts: &mut Vec<(Instruction, usize)>,
    instruction: &Instruction,
    increment: bool,
) -> bool {
    if !matches!(
        instruction,
        Instruction::Standard(_) | Instruction::McGate(_)
    ) {
        return true;
    }
    if let Some((_, count)) = counts
        .iter_mut()
        .find(|(existing, _)| existing == instruction)
    {
        if increment {
            *count = count.saturating_add(1);
            true
        } else if *count > 0 {
            *count -= 1;
            true
        } else {
            false
        }
    } else if increment {
        counts.push((instruction.clone(), 1));
        true
    } else {
        false
    }
}

/// Returns whether every rewrite candidate admitted by `requested` was also
/// admitted while establishing `proof`.
pub(super) fn config_is_proven_subset(
    requested: &RewriteConfig,
    proof: &RewriteConfig,
    gate_histogram: Option<&GateInstructionHistogram>,
) -> bool {
    requested.mode() == proof.mode()
        && target_policy_is_proven_subset(requested, proof, gate_histogram)
        && requested.max_window_ops() <= proof.max_window_ops()
        && requested.max_pattern_len() <= proof.max_pattern_len()
        && (!requested.recurses_control_flow() || proof.recurses_control_flow())
        && (!proof.skips_labeled_ops() || requested.skips_labeled_ops())
        && (!proof.preserve_two_qubit_connectivity() || requested.preserve_two_qubit_connectivity())
        && requested
            .enabled_kinds()
            .iter()
            .all(|kind| proof.enabled_kinds().contains(kind))
}

fn target_policy_is_proven_subset(
    requested: &RewriteConfig,
    proof: &RewriteConfig,
    gate_histogram: Option<&GateInstructionHistogram>,
) -> bool {
    let requested_target = requested.target_instruction_basis();
    let proof_target = proof.target_instruction_basis();
    if requested_target == proof_target {
        return true;
    }
    let (Some(requested_target), None) = (requested_target.as_deref(), proof_target.as_deref())
    else {
        return false;
    };
    gate_histogram.is_some_and(|histogram| {
        histogram.is_within_target(requested_target, requested.recurses_control_flow())
    })
}
