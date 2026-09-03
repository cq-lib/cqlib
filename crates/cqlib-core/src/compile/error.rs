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

use crate::circuit::{CircuitError, Instruction};
use crate::device::{DeviceValidationError, LogicalQubit, PhysicalQubit};
use std::fmt;
use thiserror::Error;

/// One exact device-lowering dependency for which no plan exists.
#[derive(Debug, Clone)]
pub struct DeviceLoweringDependency {
    /// Instruction required by the candidate.
    pub instruction: Instruction,
    /// Exact ordered physical arguments required by the candidate.
    pub qargs: Vec<PhysicalQubit>,
}

/// One unsuccessful candidate considered while lowering a device instruction.
#[derive(Debug, Clone)]
pub struct DeviceLoweringCandidateFailure {
    /// Stable direction-template or knowledge-rule name.
    pub template: String,
    /// Unique candidate dependencies for which the planner found no native plan.
    pub unsatisfied_dependencies: Vec<DeviceLoweringDependency>,
}

/// Structured diagnostics for an operation with no device instruction plan.
#[derive(Debug, Clone)]
pub struct DeviceLoweringFailure {
    /// Source instruction that could not be lowered.
    pub instruction: Instruction,
    /// Ordered physical arguments on which lowering was attempted.
    pub qargs: Vec<PhysicalQubit>,
    /// Candidate templates and their unresolved exact-qargs dependencies.
    pub attempted_candidates: Vec<DeviceLoweringCandidateFailure>,
}

impl fmt::Display for DeviceLoweringFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "no device instruction lowering plan for {} on ordered physical qargs {:?}",
            self.instruction, self.qargs
        )
    }
}

impl std::error::Error for DeviceLoweringFailure {}

/// Structured failures produced while selecting or routing a SABRE layout.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SabreRoutingFailure {
    /// No physical qubit supports the complete native plan required by a
    /// logical unary requirement.
    #[error("logical unary requirement on {logical} has no executable native terminal")]
    NoExecutableUnaryTerminal {
        /// Logical qubit carrying the unsupported unary requirement.
        logical: LogicalQubit,
    },
    /// A unary requirement cannot reach any physical qubit on which its
    /// selected native plan is executable.
    #[error(
        "logical unary requirement on {logical} at physical qubit {physical} cannot reach an executable location through lowerable SWAP edges"
    )]
    UnreachableUnaryPlacement {
        /// Logical qubit carrying the unary requirement.
        logical: LogicalQubit,
        /// Physical location at which the logical qubit starts.
        physical: PhysicalQubit,
    },
    /// No ordered physical pair supports the complete native plan required by
    /// a logical interaction.
    #[error("logical interaction {logical:?} has no executable native terminal pair")]
    NoExecutablePairTerminal {
        /// Ordered logical pair carrying the unsupported interaction.
        logical: [LogicalQubit; 2],
    },
    /// An ordered pair requirement cannot reach any physical pair on which its
    /// selected native plan is executable.
    #[error(
        "logical interaction {logical:?} at physical qubits {physical:?} cannot reach an executable terminal pair through lowerable SWAP edges"
    )]
    UnreachablePairPlacement {
        /// Ordered logical pair carrying the interaction.
        logical: [LogicalQubit; 2],
        /// Ordered physical placement at which routing starts.
        physical: [PhysicalQubit; 2],
    },
    /// No assignment of logical qubits to SWAP-feasible movement components
    /// satisfies all unary, ordered-pair, and capacity constraints.
    #[error(
        "no movement-component assignment satisfies all unary, ordered-pair, and capacity constraints"
    )]
    MovementAssignmentInfeasible,
    /// The bounded component-assignment search stopped without either finding
    /// a placement or proving that none exists.
    #[error(
        "movement-component assignment exhausted budget {budget} after {expansions} expansions without proving infeasibility"
    )]
    MovementAssignmentBudgetExhausted {
        /// Maximum number of assignment states permitted by the configuration.
        budget: usize,
        /// Number of assignment states actually expanded.
        expansions: usize,
    },
    /// Candidate generation succeeded, but every candidate failed an exact
    /// movement-reachability or native-lowering requirement.
    #[error(
        "all {evaluated} SABRE layout candidates were infeasible ({missing_terminal} missing a native terminal, {movement_unreachable} movement-unreachable, {unsupported_native} rejected during native lowering)"
    )]
    NoFeasibleLayoutCandidate {
        /// Total number of candidates evaluated.
        evaluated: usize,
        /// Candidates rejected because a requirement has no executable native
        /// terminal anywhere on the target.
        missing_terminal: usize,
        /// Candidates rejected because their starting placement cannot reach
        /// an existing terminal through lowerable SWAPs.
        movement_unreachable: usize,
        /// Candidates rejected because an exact-qargs native plan was absent.
        unsupported_native: usize,
    },
}

/// Errors raised by compiler infrastructure and compiler state validation.
#[derive(Debug, Error)]
pub enum CompilerError {
    /// Conversion or validation of the circuit control-flow graph failed.
    #[error(transparent)]
    Circuit(#[from] CircuitError),
    /// The input compiler state or circuit does not satisfy a pass precondition.
    #[error("invalid compiler input: {0}")]
    InvalidInput(String),
    /// A compiler transform could not complete its declared operation.
    #[error("compiler transform '{name}' failed: {reason}")]
    TransformFailed {
        /// Stable transform or synthesis primitive name.
        name: &'static str,
        /// Human-readable diagnostic describing why the transform failed.
        reason: String,
    },
    /// A compiler pass produced a state that violates its declared contract.
    #[error("compiler invariant violation: {0}")]
    InvariantViolation(String),
    /// No exact-qargs native lowering plan exists for an operation.
    #[error("device instruction lowering failed: {0}")]
    DeviceLoweringFailed(#[source] DeviceLoweringFailure),
    /// SABRE could not find a feasible layout or route under the declared
    /// movement and native-instruction constraints.
    #[error("SABRE routing failed: {0}")]
    SabreRoutingFailed(#[source] SabreRoutingFailure),
    /// The final circuit violates the configured device execution contract.
    #[error("device validation failed: {0}")]
    DeviceValidationFailed(#[from] DeviceValidationError),
}
