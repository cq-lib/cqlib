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

//! Target-neutral and target-basis-aware one-qubit optimization.
//!
//! Candidate generation is shared with the exact-physical native optimizer:
//! fixed numeric one-qubit runs are fused through exact matrix synthesis and
//! proven Z/Pauli frames are propagated through a closed identity table.
//! Logical and basis workflows differ only in the cost used to accept the
//! resulting candidate.

use crate::circuit::{Circuit, Instruction};
use crate::compile::CompilerError;
use crate::compile::transform::native_optimization::{
    LocalOptimizationPolicy, optimize_one_qubit_runs_with_policy,
    optimize_one_qubit_runs_with_policy_and_edits,
};
use crate::compile::transform::target_basis::TargetBasisCostModel;
use crate::compile::transform::{CircuitAnalysis, RewriteEdits, TransformOutcome, Transformer};
use std::sync::Arc;

/// Exact one-qubit optimization for logical or explicit-basis workflows.
#[derive(Debug, Clone)]
pub struct OptimizeOneQubitRuns {
    policy: LocalOptimizationPolicy,
}

impl OptimizeOneQubitRuns {
    /// Builds a target-neutral optimizer that accepts only strict logical-cost
    /// improvements.
    pub const fn logical() -> Self {
        Self {
            policy: LocalOptimizationPolicy::Logical,
        }
    }

    /// Builds an optimizer whose candidates are costed after exact lowering to
    /// `target_basis`.
    pub fn basis(target_basis: Vec<Instruction>) -> Result<Self, CompilerError> {
        Ok(Self::basis_with_cost_model(Arc::new(
            TargetBasisCostModel::new(target_basis)?,
        )))
    }

    pub(crate) fn basis_with_cost_model(cost_model: Arc<TargetBasisCostModel>) -> Self {
        Self {
            policy: LocalOptimizationPolicy::Basis(cost_model),
        }
    }

    /// Runs the optimizer and returns exact top-level operation provenance for
    /// incremental rewrite-session invalidation.
    pub(crate) fn transform_with_rewrite_edits(
        &self,
        circuit: &Circuit,
    ) -> Result<(TransformOutcome, RewriteEdits), CompilerError> {
        optimize_one_qubit_runs_with_policy_and_edits(circuit, &self.policy)
    }
}

impl Transformer for OptimizeOneQubitRuns {
    fn name(&self) -> &'static str {
        "optimize.one_qubit_runs"
    }

    fn transform(
        &self,
        circuit: &Circuit,
        _analysis: Option<&CircuitAnalysis>,
    ) -> Result<TransformOutcome, CompilerError> {
        optimize_one_qubit_runs_with_policy(circuit, &self.policy)
    }
}

#[cfg(test)]
#[path = "one_qubit_optimization_test.rs"]
mod one_qubit_optimization_test;
