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

//! Result types for concrete layout algorithms.
//!
//! Layout algorithms return both the selected mapping and enough metadata for
//! compiler pipelines to explain why that mapping was chosen. Diagnostics are
//! intentionally lightweight and human-readable; they are not a stable machine
//! protocol.

use super::objective::LayoutScore;
use crate::device::Layout;

/// Candidate or selected layout produced by a concrete layout algorithm.
///
/// Most production layout entries return a scored result. The score remains
/// optional so simple adapters or future algorithms can report a valid layout
/// before a final objective has been evaluated.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutResult {
    /// Selected logical-to-physical mapping.
    pub layout: Layout,
    /// Optional observed score for this layout under the requested objective.
    ///
    /// Individual algorithms may use a different selection key. In
    /// particular, SABRE selects its winner by predicted route quality under
    /// the prepared routing cost model and reports this score for diagnostics.
    pub score: Option<LayoutScore>,
    /// Diagnostics describing search and scoring behavior.
    pub diagnostics: LayoutDiagnostics,
}

/// Diagnostics emitted by layout planning.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayoutDiagnostics {
    /// Whether this layout directly realizes every two-qubit interaction on an
    /// adjacent hardware edge.
    pub is_perfect: bool,
    /// Number of candidate layouts considered by the method.
    ///
    /// Algorithms count this according to their own search unit: for example,
    /// greedy placement counts local placement candidates, while VF2 and SABRE
    /// count complete layouts that were evaluated.
    pub candidates_evaluated: usize,
    /// Whether fidelity/error data contributed to the selected score.
    pub used_fidelity: bool,
    /// Human-readable notes for debug reports and compiler logs.
    pub notes: Vec<String>,
}

impl LayoutDiagnostics {
    /// Creates empty diagnostics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one diagnostic note and returns the updated diagnostics.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}
