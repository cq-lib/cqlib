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

//! High-level gate commutation checker.
//!
//! [`CommutationChecker`] is the public entry point for asking whether two
//! concrete instruction applications may be swapped.  It is intentionally
//! conservative: `None` is not a proof of non-commutation, only a statement
//! that the configured proof sources did not establish a valid exchange.
//!
//! The checker first applies cheap local facts, then algebraic proofs, then
//! optional knowledge-base commutation rules, and finally optional local-matrix
//! comparison.  This order keeps symbolic queries fast while still supporting
//! exact numeric fallback for small concrete gates.
//!
//! # Examples
//!
//! ```rust
//! use cqlib_core::circuit::{Instruction, Qubit, StandardGate};
//! use cqlib_core::compile::commutation::{Commutation, CommutationChecker};
//!
//! let checker = CommutationChecker::builtin();
//! let result = checker.check(
//!     &Instruction::Standard(StandardGate::H),
//!     &[Qubit::new(0)],
//!     &[],
//!     &Instruction::Standard(StandardGate::X),
//!     &[Qubit::new(1)],
//!     &[],
//! );
//!
//! assert_eq!(result, Some(Commutation::Exact));
//! ```

use crate::circuit::{Instruction, Parameter, Qubit, StandardGate};
use crate::compile::PARAMETER_EQ_TOLERANCE;
use crate::compile::commutation::algebra::algebraic_commutation;
use crate::compile::commutation::matrix::matrix_commutation;
use crate::compile::commutation::rules::RuleCommutationOracle;
use crate::compile::knowledge::library::RuleLibrary;
use std::sync::OnceLock;

/// Proven relationship between two commuting operations.
///
/// [`Commutation::UpToGlobalPhase`] uses the phase angle `phase` in:
///
/// `lhs * rhs = exp(i * phase) * rhs * lhs`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Commutation {
    /// The operations commute exactly.
    Exact,
    /// The operations commute after accounting for a global phase.
    UpToGlobalPhase(Parameter),
}

impl Commutation {
    /// Returns whether this proof is exact and does not introduce a global phase.
    pub fn is_exact(&self) -> bool {
        matches!(self, Self::Exact)
    }

    /// Returns the global phase associated with this proof.
    pub fn phase(&self) -> Parameter {
        match self {
            Self::Exact => Parameter::from(0.0),
            Self::UpToGlobalPhase(phase) => phase.clone(),
        }
    }
}

/// Result of a commutation query.
///
/// `None` means the checker cannot prove that the operations commute.
pub type CommutationResult = Option<Commutation>;

/// Configuration for [`CommutationChecker`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommutationConfig {
    /// Enables matching explicit `A; B -> B; A` rules from the compiler knowledge base.
    pub enable_rule_oracle: bool,
    /// Enables small local matrix comparison as a fallback for concrete gates.
    pub enable_matrix_fallback: bool,
    /// Maximum number of qubits in the union support for matrix fallback.
    ///
    /// This bound applies to the sorted union of the two operations' qubit
    /// supports, not to each operation separately.
    pub max_matrix_qubits: usize,
}

impl Default for CommutationConfig {
    fn default() -> Self {
        Self {
            enable_rule_oracle: true,
            enable_matrix_fallback: true,
            max_matrix_qubits: 4,
        }
    }
}

/// Reusable commutation checker for compiler passes.
#[derive(Debug, Clone)]
pub struct CommutationChecker {
    /// Optional knowledge-base oracle for explicit commutation rules.
    rules: Option<RuleCommutationOracle>,
    /// Runtime knobs controlling slower or less general proof sources.
    config: CommutationConfig,
}

impl CommutationChecker {
    /// Builds a checker with builtin commutation rules and default configuration.
    pub fn builtin() -> Self {
        Self::with_config(CommutationConfig::default())
    }

    /// Builds a checker with builtin commutation rules and an explicit configuration.
    pub fn with_config(config: CommutationConfig) -> Self {
        let rules = if config.enable_rule_oracle {
            Some(RuleCommutationOracle::builtin())
        } else {
            None
        };

        Self { rules, config }
    }

    /// Builds a checker using commutation rules from an already loaded library.
    pub fn from_library(library: &RuleLibrary, config: CommutationConfig) -> Self {
        let rules = if config.enable_rule_oracle {
            Some(RuleCommutationOracle::from_library(library))
        } else {
            None
        };

        Self { rules, config }
    }

    /// Returns the active checker configuration.
    pub const fn config(&self) -> &CommutationConfig {
        &self.config
    }

    /// Checks whether two concrete instruction applications commute.
    ///
    /// The qubit and parameter slices must match the instruction arities, and
    /// each operation must use distinct qubits internally.  Non-gate
    /// instructions such as directives, delays, and control-flow gates are
    /// outside the checker and return `None`.
    ///
    /// Proofs are attempted in a fixed order:
    ///
    /// 1. cheap local facts: identity, global phase, disjoint support, and the
    ///    same application;
    /// 2. symbolic algebra over supported gate families;
    /// 3. explicit knowledge-base commutation rules, when enabled;
    /// 4. concrete local matrix comparison, when enabled and small enough.
    pub fn check(
        &self,
        lhs_inst: &Instruction,
        lhs_qubits: &[Qubit],
        lhs_params: &[Parameter],
        rhs_inst: &Instruction,
        rhs_qubits: &[Qubit],
        rhs_params: &[Parameter],
    ) -> CommutationResult {
        let (lhs_expected_qubits, lhs_expected_params) = lhs_inst.gate_arity()?;
        let (rhs_expected_qubits, rhs_expected_params) = rhs_inst.gate_arity()?;
        if lhs_qubits.len() != lhs_expected_qubits
            || lhs_params.len() != lhs_expected_params
            || rhs_qubits.len() != rhs_expected_qubits
            || rhs_params.len() != rhs_expected_params
            || lhs_qubits
                .iter()
                .enumerate()
                .any(|(index, qubit)| lhs_qubits[index + 1..].contains(qubit))
            || rhs_qubits
                .iter()
                .enumerate()
                .any(|(index, qubit)| rhs_qubits[index + 1..].contains(qubit))
        {
            return None;
        }

        if matches!(lhs_inst, Instruction::Standard(StandardGate::I))
            || matches!(rhs_inst, Instruction::Standard(StandardGate::I))
            || matches!(lhs_inst, Instruction::McGate(gate) if *gate.base_gate() == StandardGate::I)
            || matches!(rhs_inst, Instruction::McGate(gate) if *gate.base_gate() == StandardGate::I)
            || matches!(lhs_inst, Instruction::Standard(StandardGate::GPhase))
            || matches!(rhs_inst, Instruction::Standard(StandardGate::GPhase))
            || !lhs_qubits.iter().any(|qubit| rhs_qubits.contains(qubit))
        {
            return Some(Commutation::Exact);
        }
        if same_application(
            lhs_inst, lhs_qubits, lhs_params, rhs_inst, rhs_qubits, rhs_params,
        ) {
            return Some(Commutation::Exact);
        }

        if let Some(result) = algebraic_commutation(
            lhs_inst, lhs_qubits, lhs_params, rhs_inst, rhs_qubits, rhs_params,
        ) {
            return Some(result);
        }

        if let Some(rules) = self.rules.as_ref()
            && let Some(result) = rules.check(
                lhs_inst, lhs_qubits, lhs_params, rhs_inst, rhs_qubits, rhs_params,
            )
        {
            return Some(result);
        }

        if self.config.enable_matrix_fallback {
            return matrix_commutation(
                lhs_inst,
                lhs_qubits,
                lhs_params,
                rhs_inst,
                rhs_qubits,
                rhs_params,
                self.config.max_matrix_qubits,
            );
        }

        None
    }
}

/// Checks commutation with the shared builtin checker.
///
/// The builtin instance enables both knowledge-base rules and matrix fallback
/// with the default qubit limit.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::circuit::{Instruction, Qubit, StandardGate};
/// use cqlib_core::compile::commutation::{check_commutation, Commutation};
///
/// let result = check_commutation(
///     &Instruction::Standard(StandardGate::X),
///     &[Qubit::new(0)],
///     &[],
///     &Instruction::Standard(StandardGate::X),
///     &[Qubit::new(0)],
///     &[],
/// );
///
/// assert_eq!(result, Some(Commutation::Exact));
/// ```
pub fn check_commutation(
    lhs_inst: &Instruction,
    lhs_qubits: &[Qubit],
    lhs_params: &[Parameter],
    rhs_inst: &Instruction,
    rhs_qubits: &[Qubit],
    rhs_params: &[Parameter],
) -> CommutationResult {
    static CHECKER: OnceLock<CommutationChecker> = OnceLock::new();
    CHECKER.get_or_init(CommutationChecker::builtin).check(
        lhs_inst, lhs_qubits, lhs_params, rhs_inst, rhs_qubits, rhs_params,
    )
}

/// Returns whether both operands are the same gate application.
///
/// Parameter comparison is symbolic and tolerant, so expressions that are
/// provably equal are treated as the same application even when they are not
/// represented by identical syntax.
fn same_application(
    lhs_inst: &Instruction,
    lhs_qubits: &[Qubit],
    lhs_params: &[Parameter],
    rhs_inst: &Instruction,
    rhs_qubits: &[Qubit],
    rhs_params: &[Parameter],
) -> bool {
    let same_instruction =
        lhs_inst.is_quantum_gate() && rhs_inst.is_quantum_gate() && lhs_inst == rhs_inst;

    lhs_qubits == rhs_qubits
        && same_instruction
        && lhs_params.len() == rhs_params.len()
        && lhs_params
            .iter()
            .zip(rhs_params)
            .all(|(lhs, rhs)| lhs.provably_equal(rhs, PARAMETER_EQ_TOLERANCE))
}
