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

use super::edit_validation::{
    map_and_invalidate_ranges, valid_clean_gap_bijections, valid_exact_clean_gaps,
    valid_replacements,
};
use super::proof::{GateInstructionHistogram, config_is_proven_subset};
use super::rewriter::is_linear_workspace_eligible;
use super::{KnowledgeRewriteStats, RewriteConfig};
use crate::circuit::Circuit;
use crate::compile::transform::RewriteEdits;
use std::ops::Range;

/// Deterministic work-avoidance counters for a workflow rewrite session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct KnowledgeRewriteWorksetStats {
    /// Number of semantic rewrite calls presented to the session.
    pub(crate) calls: usize,
    /// Calls discharged directly by an unchanged-circuit fixpoint proof.
    pub(crate) direct_reuses: usize,
    /// Calls which executed the rewriter.
    pub(crate) executions: usize,
    /// Executions initialized directly from verified transform edits.
    pub(crate) edit_reconciliations: usize,
    /// Anchor positions admitted by edit-derived dirty ranges.
    pub(crate) anchors_recomputed: usize,
    /// Verified transform edit scripts accepted without matcher-wide scanning.
    pub(crate) edits_accepted: usize,
    /// Supplied edit scripts rejected because their structural contract was not
    /// sufficient to preserve the clean-anchor proof.
    pub(crate) edit_fallbacks: usize,
    /// Reach carried by the proof used or established by the latest call.
    pub(crate) last_proof_reach: usize,
}

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
}

/// Cross-pass rewrite state owned by one compiler workflow run.
#[derive(Debug, Default)]
pub(crate) struct KnowledgeRewriteSession {
    certificate: Option<RewriteCertificate>,
    stats: KnowledgeRewriteWorksetStats,
    last_rewrite_stats: Option<KnowledgeRewriteStats>,
}

impl KnowledgeRewriteSession {
    pub(crate) fn invalidate(&mut self) {
        self.certificate = None;
    }

    pub(crate) fn reusable_stats(
        &self,
        circuit_revision: u64,
        requested: &RewriteConfig,
    ) -> Option<(KnowledgeRewriteStats, usize)> {
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
        .then_some((
            KnowledgeRewriteStats {
                rounds_executed: 1,
                rules_applied: 0,
                changed_sequences: 0,
                reached_fixpoint: true,
            },
            certificate.proof_reach,
        ))
    }

    pub(crate) fn record_reuse(&mut self, stats: KnowledgeRewriteStats, proof_reach: usize) {
        self.stats.calls = self.stats.calls.saturating_add(1);
        self.stats.direct_reuses = self.stats.direct_reuses.saturating_add(1);
        self.stats.last_proof_reach = proof_reach;
        self.last_rewrite_stats = Some(stats);
    }

    pub(crate) fn record_execution(
        &mut self,
        circuit: &Circuit,
        circuit_revision: u64,
        config: RewriteConfig,
        proof_reach: usize,
        qubit_bijection_invariant: bool,
        stats: KnowledgeRewriteStats,
    ) {
        self.stats.calls = self.stats.calls.saturating_add(1);
        self.stats.executions = self.stats.executions.saturating_add(1);
        self.stats.last_proof_reach = proof_reach;
        if stats.reached_fixpoint {
            self.certificate = Some(RewriteCertificate {
                circuit_revision,
                proof_config: config,
                proof_reach,
                pending_ranges: Vec::new(),
                qubit_bijection_invariant,
                gate_histogram: GateInstructionHistogram::from_circuit(circuit),
            });
        } else {
            self.certificate = None;
        }
        self.last_rewrite_stats = Some(stats);
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
            self.stats.edit_fallbacks = self.stats.edit_fallbacks.saturating_add(1);
            self.certificate = None;
            return;
        }
        let proof_reach = certificate.proof_reach;
        let pending_ranges = certificate.pending_ranges.clone();
        let qubit_bijection_invariant = certificate.qubit_bijection_invariant;
        let (old_len, new_len, replacements, valid_correspondence) = match edits {
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
            ),
            RewriteEdits::Unknown => {
                self.stats.edit_fallbacks = self.stats.edit_fallbacks.saturating_add(1);
                self.certificate = None;
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
            self.stats.edit_fallbacks = self.stats.edit_fallbacks.saturating_add(1);
            self.certificate = None;
            return;
        }

        let Some(gate_histogram) =
            certificate
                .gate_histogram
                .after_linear_replacements(before, after, replacements)
        else {
            self.stats.edit_fallbacks = self.stats.edit_fallbacks.saturating_add(1);
            self.certificate = None;
            return;
        };

        let pending_ranges =
            map_and_invalidate_ranges(&pending_ranges, replacements, old_len, new_len, proof_reach);
        let certificate = self
            .certificate
            .as_mut()
            .expect("certificate was validated above");
        certificate.pending_ranges = pending_ranges;
        certificate.circuit_revision = new_revision;
        certificate.gate_histogram = gate_histogram;
        self.stats.edits_accepted = self.stats.edits_accepted.saturating_add(1);
    }

    /// Returns edit-derived initial anchor ranges. `None` means the
    /// caller must execute a full scan.
    pub(crate) fn reconcile_linear_ranges(
        &mut self,
        circuit_revision: u64,
        requested: &RewriteConfig,
    ) -> Option<Vec<Range<usize>>> {
        let certificate = self.certificate.as_ref()?;
        if certificate.circuit_revision != circuit_revision
            || !config_is_proven_subset(
                requested,
                &certificate.proof_config,
                Some(&certificate.gate_histogram),
            )
        {
            return None;
        }
        self.stats.anchors_recomputed = self.stats.anchors_recomputed.saturating_add(
            certificate
                .pending_ranges
                .iter()
                .map(|range| range.end.saturating_sub(range.start))
                .sum::<usize>(),
        );
        self.stats.edit_reconciliations = self.stats.edit_reconciliations.saturating_add(1);
        Some(certificate.pending_ranges.clone())
    }

    #[cfg(test)]
    pub(crate) const fn stats(&self) -> KnowledgeRewriteWorksetStats {
        self.stats
    }

    #[cfg(test)]
    pub(crate) fn last_rewrite_stats(&self) -> Option<&KnowledgeRewriteStats> {
        self.last_rewrite_stats.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn proof_reach(&self) -> Option<usize> {
        self.certificate
            .as_ref()
            .map(|certificate| certificate.proof_reach)
    }
}

#[cfg(test)]
#[path = "session_test.rs"]
mod session_test;
