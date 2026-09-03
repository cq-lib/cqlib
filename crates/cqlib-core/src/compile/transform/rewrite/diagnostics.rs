// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Rewrite work-avoidance and condition-evaluation diagnostics.

/// Performance diagnostics produced by rewrite execution.
///
/// These counters describe which incremental and condition-evaluation paths
/// were exercised. They never participate in rule selection or correctness
/// decisions and are suitable for benchmark attribution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KnowledgeRewriteDiagnostics {
    /// Rewrite phases skipped by a reusable session fixpoint proof.
    pub direct_reuses: usize,
    /// Anchors presented by verified dirty ranges before any density fallback.
    pub dirty_anchors: usize,
    /// Incremental executions conservatively promoted to full scans.
    ///
    /// This includes rejected or incompatible edit proofs, unsupported
    /// circuit shapes, and dirty ranges exceeding the density threshold.
    pub full_scan_fallbacks: usize,
    /// Cached rule-condition results reused for identical parameter bindings.
    pub condition_cache_hits: usize,
    /// Cacheable rule-condition bindings not found in the pass-local cache.
    pub condition_cache_misses: usize,
    /// Conditions evaluated by the conservative symbolic path.
    pub symbolic_fallbacks: usize,
}

impl KnowledgeRewriteDiagnostics {
    pub(crate) fn merge(&mut self, other: Self) {
        self.direct_reuses = self.direct_reuses.saturating_add(other.direct_reuses);
        self.dirty_anchors = self.dirty_anchors.saturating_add(other.dirty_anchors);
        self.full_scan_fallbacks = self
            .full_scan_fallbacks
            .saturating_add(other.full_scan_fallbacks);
        self.condition_cache_hits = self
            .condition_cache_hits
            .saturating_add(other.condition_cache_hits);
        self.condition_cache_misses = self
            .condition_cache_misses
            .saturating_add(other.condition_cache_misses);
        self.symbolic_fallbacks = self
            .symbolic_fallbacks
            .saturating_add(other.symbolic_fallbacks);
    }

    pub(super) fn merge_matcher(&mut self, other: MatcherDiagnostics) {
        self.merge(Self {
            direct_reuses: 0,
            dirty_anchors: other.dirty_anchors,
            full_scan_fallbacks: other.full_scan_fallbacks,
            condition_cache_hits: other.condition_cache_hits,
            condition_cache_misses: other.condition_cache_misses,
            symbolic_fallbacks: other.symbolic_fallbacks,
        });
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct MatcherDiagnostics {
    pub(super) condition_cache_hits: usize,
    pub(super) condition_cache_misses: usize,
    pub(super) symbolic_fallbacks: usize,
    pub(super) dirty_anchors: usize,
    pub(super) full_scan_fallbacks: usize,
}

impl MatcherDiagnostics {
    pub(super) fn saturating_delta(self, earlier: Self) -> Self {
        Self {
            condition_cache_hits: self
                .condition_cache_hits
                .saturating_sub(earlier.condition_cache_hits),
            condition_cache_misses: self
                .condition_cache_misses
                .saturating_sub(earlier.condition_cache_misses),
            symbolic_fallbacks: self
                .symbolic_fallbacks
                .saturating_sub(earlier.symbolic_fallbacks),
            dirty_anchors: self.dirty_anchors.saturating_sub(earlier.dirty_anchors),
            full_scan_fallbacks: self
                .full_scan_fallbacks
                .saturating_sub(earlier.full_scan_fallbacks),
        }
    }
}
