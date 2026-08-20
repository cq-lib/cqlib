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

//! Configuration for numeric two-qubit block resynthesis.

use crate::compile::commutation::CommutationConfig;
use crate::compile::transform::decompose::unitary::TwoQubitSynthesisTarget;

/// Configuration for ordinary standard-gate two-qubit block resynthesis.
///
/// `normal` is intended for the default compilation pipeline. `enhanced`
/// increases bounded search budgets and is used where compile-time budget is
/// explicitly traded for better post-routing cleanup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoQubitBlockResynthesisConfig {
    /// Target capability used by exact two-qubit numerical synthesis.
    pub two_qubit_target: TwoQubitSynthesisTarget,
    /// Maximum number of source operations allowed in one bounded
    /// commutation-aware candidate. Maximal uninterrupted pair runs are not
    /// limited by this supplemental search budget.
    pub max_block_ops: usize,
    /// Maximum number of non-block operations that may be crossed while
    /// collecting a block. Crossed operations are preserved at their original
    /// source positions and must commute with the synthesized replacement.
    pub max_crossed_ops: usize,
    /// Maximum bounded collection budget per side of a two-qubit anchor.
    ///
    /// For source-order collection this is the number of source positions
    /// scanned per side. For DAG collection this is the number of operation
    /// frontier nodes visited per direction.
    pub max_scan_span: usize,
    /// Treat labeled operations as hard boundaries.
    pub skip_labeled_ops: bool,
    /// Recursively run the pass inside structured classical-control bodies.
    pub recurse_control_flow: bool,
    /// Semantic commutation engine configuration used by the local collector.
    pub commutation: CommutationConfig,
}

impl TwoQubitBlockResynthesisConfig {
    /// Returns the default-budget configuration for the selected two-qubit
    /// synthesis basis.
    pub fn normal(two_qubit_target: TwoQubitSynthesisTarget) -> Self {
        Self {
            two_qubit_target,
            max_block_ops: 16,
            max_crossed_ops: 4,
            max_scan_span: 32,
            skip_labeled_ops: true,
            recurse_control_flow: true,
            commutation: CommutationConfig {
                enable_rule_oracle: true,
                enable_matrix_fallback: false,
                max_matrix_qubits: 4,
            },
        }
    }

    /// Returns a higher-budget configuration for enhanced optimization modes.
    pub fn enhanced(two_qubit_target: TwoQubitSynthesisTarget) -> Self {
        Self {
            max_block_ops: 32,
            max_crossed_ops: 8,
            max_scan_span: 64,
            ..Self::normal(two_qubit_target)
        }
    }
}

impl Default for TwoQubitBlockResynthesisConfig {
    fn default() -> Self {
        Self::normal(TwoQubitSynthesisTarget::default())
    }
}

#[cfg(test)]
#[path = "config_test.rs"]
mod config_test;
