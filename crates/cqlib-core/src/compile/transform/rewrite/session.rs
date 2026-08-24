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

//! Workflow-lifetime proof state for knowledge rewrite.

use super::RewriteConfig;
use super::edit_validation::{
    map_and_invalidate_ranges, valid_clean_gap_bijections, valid_exact_clean_gaps,
    valid_replacements,
};
use super::proof::{GateInstructionHistogram, config_is_proven_subset};
use super::rewriter::{LinearRewriteWorkspace, is_linear_workspace_eligible};
use crate::circuit::Circuit;
use crate::compile::transform::RewriteEdits;
use std::ops::Range;

#[derive(Debug, Clone)]
struct RewriteCertificate {
    /// Workflow circuit revision for which every eligible anchor was proven
    /// candidate-free.
    circuit_revision: u64,
    /// Configuration under which the clean-anchor proof was established.
    proof_config: RewriteConfig,
    /// Maximum forward observation reach used by that proof. Mutation-aware
    /// reuse must invalidate against this value, never silently recompute it
    /// from a later, narrower configuration.
    proof_reach: usize,
    /// Dirty anchor coordinates accumulated since the certificate was proved.
    /// The ranges always live in `circuit_revision` coordinates.
    pending_ranges: Vec<Range<usize>>,
    /// Whether the rule model used for this proof observes runtime qubits only
    /// through equality and is therefore invariant under bijective renaming.
    qubit_bijection_invariant: bool,
    /// Gate-like instruction counts for target-policy subset checks. Exact
    /// linear edits update this from replacement ranges only, preserving
    /// the unchanged-revision O(1) reuse path.
    gate_histogram: GateInstructionHistogram,
    /// Matcher inputs for the single flat block, when the circuit shape
    /// supports cross-pass workspace reuse.
    workspace: Option<LinearRewriteWorkspace>,
}

/// Cross-pass rewrite state owned by one compiler workflow run.
#[derive(Debug, Default)]
pub(crate) struct KnowledgeRewriteSession {
    certificate: Option<RewriteCertificate>,
    /// Set only after a previously usable proof is invalidated. The next
    /// rewrite execution consumes this marker for production diagnostics;
    /// an initial rewrite with no prior proof is not a fallback.
    pending_full_scan_fallback: bool,
}

pub(crate) struct RewriteExecutionRecord {
    pub(crate) config: RewriteConfig,
    pub(crate) proof_reach: usize,
    pub(crate) qubit_bijection_invariant: bool,
    pub(crate) reached_fixpoint: bool,
    pub(crate) workspace: Option<LinearRewriteWorkspace>,
}

pub(crate) struct LinearRewriteReconciliation {
    pub(crate) state: Option<(Vec<Range<usize>>, Option<LinearRewriteWorkspace>)>,
    pub(crate) full_scan_fallback: bool,
}

impl KnowledgeRewriteSession {
    pub(crate) fn invalidate(&mut self) {
        self.invalidate_for_full_scan();
    }

    pub(crate) fn reusable_proof(
        &self,
        circuit_revision: u64,
        requested: &RewriteConfig,
    ) -> Option<usize> {
        if requested.max_rounds() == 0 {
            return None;
        }
        let certificate = self.certificate.as_ref()?;
        (certificate.circuit_revision == circuit_revision
            && certificate.pending_ranges.is_empty()
            && config_is_proven_subset(
                requested,
                &certificate.proof_config,
                Some(&certificate.gate_histogram),
            ))
        .then_some(certificate.proof_reach)
    }

    pub(crate) fn record_execution(
        &mut self,
        circuit: &Circuit,
        circuit_revision: u64,
        record: RewriteExecutionRecord,
    ) {
        if record.reached_fixpoint {
            self.pending_full_scan_fallback = false;
            self.certificate = Some(RewriteCertificate {
                circuit_revision,
                proof_config: record.config,
                proof_reach: record.proof_reach,
                pending_ranges: Vec::new(),
                qubit_bijection_invariant: record.qubit_bijection_invariant,
                gate_histogram: GateInstructionHistogram::from_circuit(circuit),
                workspace: record.workspace,
            });
        } else {
            self.certificate = None;
            // A round-limited execution did not establish clean-anchor
            // proofs, so a later retry must conservatively start with a full
            // scan even when no earlier certificate existed.
            self.pending_full_scan_fallback = true;
        }
    }

    /// Advances a clean-anchor certificate through one verified transform edit
    /// script. Strict linear edits touch only replacements; routing
    /// equivalence edits additionally validate their clean-gap qubit
    /// mappings before the certificate advances.
    pub(crate) fn apply_rewrite_edits(
        &mut self,
        old_revision: u64,
        new_revision: u64,
        before: &Circuit,
        after: &Circuit,
        edits: &RewriteEdits,
    ) {
        let Some(certificate) = self.certificate.as_ref() else {
            return;
        };
        if certificate.circuit_revision != old_revision {
            self.invalidate_for_full_scan();
            return;
        }
        let proof_reach = certificate.proof_reach;
        let pending_ranges = certificate.pending_ranges.clone();
        let qubit_bijection_invariant = certificate.qubit_bijection_invariant;
        let (old_len, new_len, replacements, valid_correspondence, qubits_renamed) = match edits {
            RewriteEdits::Linear {
                old_len,
                new_len,
                replacements,
            } => (
                *old_len,
                *new_len,
                replacements,
                before.qubits() == after.qubits()
                    && valid_exact_clean_gaps(before, after, replacements),
                false,
            ),
            RewriteEdits::LinearModuloQubitBijection {
                old_len,
                new_len,
                replacements,
                clean_gap_bijections,
            } => (
                *old_len,
                *new_len,
                replacements,
                qubit_bijection_invariant
                    && valid_clean_gap_bijections(
                        before,
                        after,
                        replacements,
                        clean_gap_bijections,
                    ),
                true,
            ),
            RewriteEdits::Unknown => {
                self.invalidate_for_full_scan();
                return;
            }
        };
        if before.operations().len() != old_len
            || after.operations().len() != new_len
            || !is_linear_workspace_eligible(before)
            || !is_linear_workspace_eligible(after)
            || !valid_replacements(old_len, new_len, replacements)
            || !valid_correspondence
        {
            self.invalidate_for_full_scan();
            return;
        }

        let Some(gate_histogram) =
            certificate
                .gate_histogram
                .after_linear_replacements(before, after, replacements)
        else {
            self.invalidate_for_full_scan();
            return;
        };

        let pending_ranges =
            map_and_invalidate_ranges(&pending_ranges, replacements, old_len, new_len, proof_reach);
        let workspace = self
            .certificate
            .as_mut()
            .and_then(|certificate| certificate.workspace.take())
            .and_then(|workspace| {
                workspace.after_linear_replacements(after, replacements, qubits_renamed)
            });
        let certificate = self
            .certificate
            .as_mut()
            .expect("certificate was validated above");
        certificate.pending_ranges = pending_ranges;
        certificate.circuit_revision = new_revision;
        certificate.gate_histogram = gate_histogram;
        certificate.workspace = workspace;
    }

    /// Returns edit-derived initial anchor ranges. `None` means the
    /// caller must execute a full scan.
    pub(crate) fn take_reconciled_linear_state(
        &mut self,
        circuit_revision: u64,
        requested: &RewriteConfig,
    ) -> LinearRewriteReconciliation {
        let Some(certificate) = self.certificate.as_mut() else {
            return LinearRewriteReconciliation {
                state: None,
                full_scan_fallback: std::mem::take(&mut self.pending_full_scan_fallback),
            };
        };
        if certificate.circuit_revision != circuit_revision
            || !config_is_proven_subset(
                requested,
                &certificate.proof_config,
                Some(&certificate.gate_histogram),
            )
        {
            return LinearRewriteReconciliation {
                state: None,
                full_scan_fallback: true,
            };
        }
        LinearRewriteReconciliation {
            state: Some((
                certificate.pending_ranges.clone(),
                certificate.workspace.take(),
            )),
            full_scan_fallback: false,
        }
    }

    fn invalidate_for_full_scan(&mut self) {
        if self.certificate.take().is_some() {
            self.pending_full_scan_fallback = true;
        }
    }
}

#[cfg(test)]
#[path = "session_test.rs"]
mod session_test;
