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

//! Configuration and local cost model for knowledge-based rewrite.

use crate::circuit::{Instruction, MCGate, StandardGate};
use crate::compile::error::CompilerError;
use crate::compile::knowledge::library::RuleKind;

/// High-level rule application mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RewriteMode {
    /// Conservative optimization. Accepted rewrites must strictly improve local cost.
    Optimize,
    /// Explicit lowering. Decomposition and hardware-native rules may expand locally.
    Lowering,
}

/// Stable configuration for the knowledge-based rewrite transformer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteConfig {
    max_rounds: u8,
    max_window_ops: usize,
    max_pattern_len: usize,
    recurse_control_flow: bool,
    skip_labeled_ops: bool,
    preserve_two_qubit_connectivity: bool,
    enabled_kinds: Vec<RuleKind>,
    mode: RewriteMode,
    target_instructions: Option<Vec<TargetInstruction>>,
}

/// Gate-like instruction accepted by target-basis lowering.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TargetInstruction {
    Standard(StandardGate),
    McGate(MCGate),
}

impl Default for RewriteConfig {
    fn default() -> Self {
        Self::production()
    }
}

impl RewriteConfig {
    /// Production defaults for conservative logical optimization.
    pub fn production() -> Self {
        Self {
            max_rounds: 8,
            max_window_ops: 16,
            max_pattern_len: 8,
            recurse_control_flow: true,
            skip_labeled_ops: true,
            preserve_two_qubit_connectivity: false,
            enabled_kinds: vec![
                RuleKind::Simplify,
                RuleKind::Cancel,
                RuleKind::Merge,
                RuleKind::Canonicalize,
            ],
            mode: RewriteMode::Optimize,
            target_instructions: None,
        }
    }

    /// Defaults for explicit knowledge-based lowering.
    pub fn lowering() -> Self {
        Self {
            enabled_kinds: vec![
                RuleKind::Simplify,
                RuleKind::Cancel,
                RuleKind::Merge,
                RuleKind::Canonicalize,
                RuleKind::Decompose,
                RuleKind::HardwareNative,
            ],
            mode: RewriteMode::Lowering,
            ..Self::production()
        }
    }

    /// Returns a new production configuration.
    pub fn new() -> Self {
        Self::production()
    }

    pub fn with_max_rounds(mut self, max_rounds: u8) -> Self {
        self.max_rounds = max_rounds;
        self
    }

    pub fn with_max_window_ops(mut self, max_window_ops: usize) -> Self {
        self.max_window_ops = max_window_ops;
        self
    }

    pub fn with_max_pattern_len(mut self, max_pattern_len: usize) -> Self {
        self.max_pattern_len = max_pattern_len;
        self
    }

    pub fn recurse_control_flow(mut self, enabled: bool) -> Self {
        self.recurse_control_flow = enabled;
        self
    }

    pub fn skip_labeled_ops(mut self, enabled: bool) -> Self {
        self.skip_labeled_ops = enabled;
        self
    }

    /// Restricts rewrites to those that cannot introduce new undirected
    /// two-qubit connectivity.
    ///
    /// This is the post-routing safety policy: once a circuit has been
    /// physically mapped, every two-qubit operation acts on a device coupling
    /// edge, and a rewrite rule whose replacement uses a qubit pair absent
    /// from its match (e.g. a CX bridge collapsing `CX 0 1, CX 1 2, CX 0 1,
    /// CX 1 2` to `CX 0 2`) would break that mapping. When enabled, rules
    /// are statically filtered before matching so replacements only act on
    /// pairs already present in the matched operations.
    ///
    /// The guarantee is undirected topology connectivity, not ordered native
    /// legality: direction flips such as `CX 0 1 -> H 0, H 1, CX 1 0, H 0,
    /// H 1` remain allowed, since gate direction is the device-lowering
    /// stage's responsibility.
    pub fn with_preserve_two_qubit_connectivity(mut self, enabled: bool) -> Self {
        self.preserve_two_qubit_connectivity = enabled;
        self
    }

    pub fn with_enabled_kinds(mut self, kinds: Vec<RuleKind>) -> Self {
        self.enabled_kinds = kinds;
        self
    }

    pub fn with_mode(mut self, mode: RewriteMode) -> Self {
        self.mode = mode;
        self
    }

    /// Restricts lowering to an explicit gate-like target instruction basis.
    pub fn with_target_instructions(
        self,
        target_instructions: Vec<Instruction>,
    ) -> Result<Self, CompilerError> {
        self.try_with_target_instructions(target_instructions)
    }

    /// Restricts lowering to an explicit gate-like target instruction basis.
    pub fn try_with_target_instructions(
        mut self,
        target_instructions: Vec<Instruction>,
    ) -> Result<Self, CompilerError> {
        if target_instructions.is_empty() {
            return Err(CompilerError::InvalidInput(
                "rewrite target instruction basis must not be empty".to_string(),
            ));
        }

        let mut deduped = Vec::with_capacity(target_instructions.len());
        for instruction in target_instructions {
            let instruction = match instruction {
                Instruction::Standard(gate) => TargetInstruction::Standard(gate),
                Instruction::McGate(gate) => TargetInstruction::McGate(*gate),
                other => {
                    return Err(CompilerError::InvalidInput(format!(
                        "unsupported rewrite target instruction {other:?}"
                    )));
                }
            };

            if !deduped.contains(&instruction) {
                deduped.push(instruction);
            }
        }

        self.target_instructions = Some(deduped);
        Ok(self)
    }

    pub const fn max_rounds(&self) -> u8 {
        self.max_rounds
    }

    pub const fn max_window_ops(&self) -> usize {
        self.max_window_ops
    }

    pub const fn max_pattern_len(&self) -> usize {
        self.max_pattern_len
    }

    pub const fn recurses_control_flow(&self) -> bool {
        self.recurse_control_flow
    }

    pub const fn skips_labeled_ops(&self) -> bool {
        self.skip_labeled_ops
    }

    /// Returns whether rewrites must preserve undirected two-qubit
    /// connectivity, rejecting rules that may act on new qubit pairs.
    pub const fn preserve_two_qubit_connectivity(&self) -> bool {
        self.preserve_two_qubit_connectivity
    }

    pub const fn mode(&self) -> RewriteMode {
        self.mode
    }

    /// Returns the rule categories enabled by this configuration.
    pub fn enabled_kinds(&self) -> &[RuleKind] {
        &self.enabled_kinds
    }

    /// Returns the configured target instruction basis in insertion order.
    pub fn target_instruction_basis(&self) -> Option<Vec<Instruction>> {
        self.target_instructions.as_ref().map(|instructions| {
            instructions
                .iter()
                .map(|instruction| match instruction {
                    TargetInstruction::Standard(gate) => Instruction::Standard(*gate),
                    TargetInstruction::McGate(gate) => Instruction::McGate(Box::new(gate.clone())),
                })
                .collect()
        })
    }

    pub(super) fn target_instructions(&self) -> Option<&[TargetInstruction]> {
        self.target_instructions.as_deref()
    }

    pub(super) fn enables_kind(&self, kind: RuleKind) -> bool {
        self.enabled_kinds.contains(&kind)
    }

    pub(super) fn allows_kind(&self, kind: RuleKind) -> bool {
        if !self.enables_kind(kind) {
            return false;
        }

        !matches!(kind, RuleKind::Decompose | RuleKind::HardwareNative)
            || self.mode == RewriteMode::Lowering
    }

    pub(super) fn allows_rewrite(
        &self,
        kind: RuleKind,
        before: LocalRewriteCost,
        after: LocalRewriteCost,
    ) -> bool {
        if after.unsupported_ops > before.unsupported_ops {
            return false;
        }
        if after < before {
            return true;
        }
        if self.target_instructions.is_some() {
            return false;
        }

        self.mode == RewriteMode::Lowering
            && matches!(kind, RuleKind::Decompose | RuleKind::HardwareNative)
    }
}

/// Local rewrite cost used to enforce termination in logical rewrite mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LocalRewriteCost {
    pub(super) unsupported_ops: usize,
    pub(super) lowering_distance: usize,
    pub(super) two_qubit_ops: usize,
    pub(super) multi_qubit_ops: usize,
    pub(super) depth_estimate: usize,
    pub(super) total_ops: usize,
    pub(super) parameterized_ops: usize,
}

/// Policy for counting `GPhase` operations in local rewrite cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GPhaseCost {
    ExplicitOperation,
    ImplicitReplacement,
}

impl LocalRewriteCost {
    pub(super) fn add_gate_like(
        &mut self,
        standard_gate: Option<StandardGate>,
        target_supported: bool,
        qubit_count: usize,
        param_count: usize,
        gphase_cost: GPhaseCost,
    ) -> bool {
        if standard_gate == Some(StandardGate::GPhase)
            && gphase_cost == GPhaseCost::ImplicitReplacement
        {
            return false;
        }

        self.total_ops += 1;
        if !target_supported {
            self.unsupported_ops += 1;
        }
        match qubit_count {
            0 | 1 => {}
            2 => self.two_qubit_ops += 1,
            _ => self.multi_qubit_ops += 1,
        }
        if param_count > 0 {
            self.parameterized_ops += 1;
        }
        true
    }
}
