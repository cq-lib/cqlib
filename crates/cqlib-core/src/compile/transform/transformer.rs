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

use crate::circuit::Circuit;
use crate::compile::CompilerError;
use crate::compile::transform::analysis::{CircuitAnalysis, WorkflowCircuitAnalysis};

/// Conservative workflow scheduling decision for one compiler pass.
///
/// `ProvenNoOp` is stronger than ordinary inapplicability: executing the pass
/// on the exact analyzed circuit revision must return `TransformOutcome::Unchanged`
/// and must not suppress an error that the pass would otherwise report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PassApplicability {
    Run,
    ProvenNoOp(&'static str),
}

/// Workflow-only precondition declaration.
///
/// This remains separate from the public [`Transformer`] contract so adding
/// scheduling facts does not expand the Rust or Python API surface.
pub(crate) trait WorkflowPass {
    fn applicability(&self, analysis: &WorkflowCircuitAnalysis) -> PassApplicability;
}

/// Outcome of applying a compiler transform to a circuit.
///
/// `Unchanged` means the input compiler IR can be retained exactly as-is.
/// `Changed` carries the replacement circuit that callers must adopt.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)] // Boxing would allocate on every changed pass.
pub enum TransformOutcome {
    /// The input circuit representation can be retained without rebuilding it.
    Unchanged,
    /// The transform produced a replacement circuit representation.
    Changed(Circuit),
}

impl TransformOutcome {
    /// Whether the transform produced a replacement circuit.
    pub const fn changed(&self) -> bool {
        matches!(self, Self::Changed(_))
    }

    /// Resolves the outcome into an owned circuit.
    ///
    /// This helper is intended for APIs that promise an owned circuit. Core
    /// workflows should match on the outcome directly so `Unchanged` remains
    /// zero-copy.
    pub fn into_circuit(self, original: &Circuit) -> Circuit {
        match self {
            Self::Unchanged => original.clone(),
            Self::Changed(circuit) => circuit,
        }
    }
}

/// Common interface for compiler transforms over an immutable circuit.
///
/// # Implementing
///
/// - [`name`](Transformer::name) returns a static human-readable label for logging.
/// - [`transform`](Transformer::transform) applies the pass to a circuit.
///
/// Parameters that differ between pass instances (e.g. config, device) are bound at
/// construction time so `transform` keeps a uniform signature across all passes.
pub trait Transformer {
    /// Human-readable pass name for logging and debugging.
    fn name(&self) -> &'static str;

    /// Applies the transform to `circuit`.
    ///
    /// Callers may provide precomputed [`CircuitAnalysis`] to avoid repeated
    /// structural scans across workflow stages. When `analysis` is `None`, the
    /// transform must derive any required facts itself.
    fn transform(
        &self,
        circuit: &Circuit,
        analysis: Option<&CircuitAnalysis>,
    ) -> Result<TransformOutcome, CompilerError>;
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct ResolvedTransform {
    pub(crate) circuit: Circuit,
    pub(crate) changed: bool,
}

#[cfg(test)]
pub(crate) fn resolve_transform_for_test(
    outcome: TransformOutcome,
    original: &Circuit,
) -> ResolvedTransform {
    let changed = outcome.changed();
    ResolvedTransform {
        circuit: outcome.into_circuit(original),
        changed,
    }
}

#[cfg(test)]
pub(crate) trait TransformerTestExt: Transformer {
    fn transform_resolved(
        &self,
        circuit: &Circuit,
        analysis: Option<&CircuitAnalysis>,
    ) -> Result<ResolvedTransform, CompilerError> {
        self.transform(circuit, analysis)
            .map(|outcome| resolve_transform_for_test(outcome, circuit))
    }
}

#[cfg(test)]
impl<T: Transformer + ?Sized> TransformerTestExt for T {}

#[cfg(test)]
#[path = "./transformer_test.rs"]
mod transformer_test;
