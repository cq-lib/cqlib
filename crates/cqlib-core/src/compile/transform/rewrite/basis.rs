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

//! Target-basis policy for knowledge-based rewrite.

use crate::circuit::{Circuit, ClassicalControlOp, Instruction, Operation, StandardGate};
use crate::compile::error::CompilerError;
use crate::compile::knowledge::library::RuleKind;
use crate::compile::knowledge::matcher::KnowledgeInstructionKey as RewriteInstructionKey;
use crate::compile::transform::rewrite::config::{RewriteConfig, RewriteMode, TargetInstruction};
use crate::compile::transform::rewrite::matcher::CompiledRuleSet;
use std::collections::{HashMap, HashSet};

const NON_LOWERABLE_DISTANCE: usize = 1_000_000;
const MAX_FINAL_TARGET_EXAMPLES: usize = 3;

/// Target-basis lookup used by matcher filtering and local cost.
pub(super) struct TargetContext {
    physical_keys: HashSet<RewriteInstructionKey>,
    lowerable_ranks: HashMap<RewriteInstructionKey, usize>,
}

impl TargetContext {
    pub(super) fn from_config(
        config: &RewriteConfig,
        rules: &CompiledRuleSet,
    ) -> Result<Option<Self>, CompilerError> {
        let Some(physical_keys) = physical_keys_from_config(config)? else {
            return Ok(None);
        };

        let mut lowerable_ranks = physical_keys
            .iter()
            .cloned()
            .map(|key| (key, 0))
            .collect::<HashMap<_, _>>();

        loop {
            let mut changed = false;
            for (kind, source_keys, rewrite_keys, has_conditions) in rules.lowerable_rules() {
                if kind == RuleKind::Commute
                    || !config.allows_kind(kind)
                    || source_keys.len() != 1
                    || has_conditions
                {
                    continue;
                }

                let mut max_rewrite_rank = 0usize;
                let mut rewrite_is_lowerable = true;
                for key in rewrite_keys {
                    if matches!(key, RewriteInstructionKey::Standard(StandardGate::GPhase)) {
                        continue;
                    }
                    if let Some(rank) = lowerable_ranks.get(key) {
                        max_rewrite_rank = max_rewrite_rank.max(*rank);
                    } else {
                        rewrite_is_lowerable = false;
                        break;
                    }
                }
                if !rewrite_is_lowerable {
                    continue;
                }

                let candidate_rank = max_rewrite_rank.saturating_add(1);
                let source_key = source_keys[0].clone();
                match lowerable_ranks.get_mut(&source_key) {
                    Some(existing_rank) if candidate_rank < *existing_rank => {
                        *existing_rank = candidate_rank;
                        changed = true;
                    }
                    Some(_) => {}
                    None => {
                        lowerable_ranks.insert(source_key, candidate_rank);
                        changed = true;
                    }
                }
            }

            if !changed {
                break;
            }
        }

        Ok(Some(Self {
            physical_keys,
            lowerable_ranks,
        }))
    }

    pub(super) fn allows_rewrite_key(&self, key: &RewriteInstructionKey) -> bool {
        matches!(key, RewriteInstructionKey::Standard(StandardGate::GPhase))
            || self.lowerable_ranks.contains_key(key)
    }

    pub(super) fn physically_supports(&self, key: &RewriteInstructionKey) -> bool {
        self.physical_keys.contains(key)
    }

    pub(super) fn lowering_distance(&self, key: &RewriteInstructionKey) -> usize {
        if self.physically_supports(key) {
            return 0;
        }
        self.lowerable_ranks
            .get(key)
            .copied()
            .unwrap_or(NON_LOWERABLE_DISTANCE)
    }
}

pub(super) fn validate_final_target(
    circuit: &Circuit,
    config: &RewriteConfig,
) -> Result<(), CompilerError> {
    if config.mode() != RewriteMode::Lowering || config.target_instructions().is_none() {
        return Ok(());
    }

    let Some(physical_keys) = physical_keys_from_config(config)? else {
        return Ok(());
    };

    let mut scan = FinalTargetScan::default();
    scan_operations(
        circuit.operations(),
        &physical_keys,
        config.recurses_control_flow(),
        &mut scan,
    );

    if scan.control_flow_ops > 0 {
        return Err(CompilerError::InvalidInput(
            "rewrite cannot prove final target instruction basis while recurse_control_flow is disabled and control-flow operations are present".to_string(),
        ));
    }
    if scan.unsupported_gate_like_ops == 0 {
        return Ok(());
    }

    let target_basis = config
        .target_instruction_basis()
        .unwrap_or_default()
        .into_iter()
        .map(|instruction| instruction.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let mut reason = format!(
        "lowering incomplete: {} gate-like operations remain outside target basis {{{}}}",
        scan.unsupported_gate_like_ops, target_basis
    );
    if !scan.examples.is_empty() {
        reason.push_str(&format!(" (examples: {})", scan.examples.join(", ")));
    }
    reason.push_str("; this may indicate missing decomposition rules or insufficient max_rounds");
    Err(CompilerError::InvalidInput(reason))
}

#[derive(Default)]
struct FinalTargetScan {
    unsupported_gate_like_ops: usize,
    control_flow_ops: usize,
    examples: Vec<String>,
}

fn scan_operations(
    operations: &[Operation],
    physical_keys: &HashSet<RewriteInstructionKey>,
    recurse_control_flow: bool,
    scan: &mut FinalTargetScan,
) {
    for operation in operations {
        match &operation.instruction {
            Instruction::Standard(_) | Instruction::McGate(_) => {
                let key = RewriteInstructionKey::from_instruction(&operation.instruction)
                    .expect("standard and multi-controlled gates are rewrite instruction keys");
                if !physical_keys.contains(&key) {
                    scan.add_unsupported(&operation.instruction);
                }
            }
            Instruction::UnitaryGate(_) | Instruction::CircuitGate(_) => {
                scan.add_unsupported(&operation.instruction);
            }
            Instruction::ClassicalControl(cc) => {
                if recurse_control_flow {
                    match cc {
                        ClassicalControlOp::If(op) => {
                            scan_operations(
                                op.then_body().operations(),
                                physical_keys,
                                recurse_control_flow,
                                scan,
                            );
                            if let Some(else_body) = op.else_body() {
                                scan_operations(
                                    else_body.operations(),
                                    physical_keys,
                                    recurse_control_flow,
                                    scan,
                                );
                            }
                        }
                        ClassicalControlOp::While(op) => {
                            scan_operations(
                                op.body().operations(),
                                physical_keys,
                                recurse_control_flow,
                                scan,
                            );
                        }
                        ClassicalControlOp::For(op) => {
                            scan_operations(
                                op.body().operations(),
                                physical_keys,
                                recurse_control_flow,
                                scan,
                            );
                        }
                        ClassicalControlOp::Switch(op) => {
                            for case in op.cases() {
                                scan_operations(
                                    case.body().operations(),
                                    physical_keys,
                                    recurse_control_flow,
                                    scan,
                                );
                            }
                            if let Some(default) = op.default() {
                                scan_operations(
                                    default.operations(),
                                    physical_keys,
                                    recurse_control_flow,
                                    scan,
                                );
                            }
                        }
                        ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
                    }
                } else {
                    scan.control_flow_ops += 1;
                }
            }
            Instruction::ClassicalData(_) | Instruction::Directive(_) | Instruction::Delay => {}
        }
    }
}

fn physical_keys_from_config(
    config: &RewriteConfig,
) -> Result<Option<HashSet<RewriteInstructionKey>>, CompilerError> {
    let Some(target_instructions) = config.target_instructions() else {
        return Ok(None);
    };
    if target_instructions.is_empty() {
        return Err(CompilerError::InvalidInput(
            "rewrite target instruction basis must not be empty".to_string(),
        ));
    }

    let mut keys = HashSet::with_capacity(target_instructions.len());
    for instruction in target_instructions {
        match instruction {
            TargetInstruction::Standard(gate) => {
                keys.insert(RewriteInstructionKey::Standard(*gate));
            }
            TargetInstruction::McGate(gate) => {
                keys.insert(RewriteInstructionKey::McGate(gate.clone()));
            }
        }
    }

    Ok(Some(keys))
}

impl FinalTargetScan {
    fn add_unsupported(&mut self, instruction: &Instruction) {
        self.unsupported_gate_like_ops += 1;
        if self.examples.len() < MAX_FINAL_TARGET_EXAMPLES {
            self.examples.push(instruction.to_string());
        }
    }
}
