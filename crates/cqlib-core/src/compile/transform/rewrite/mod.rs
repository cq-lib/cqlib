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

//! Knowledge-based local circuit rewrite.
//!
//! This pass consumes validated compiler knowledge rules and applies them as
//! dependency-aware local rewrites. It is a local rewrite engine, not a global
//! optimizer: it searches bounded operation windows, checks rule conditions,
//! applies cost policy, and rebuilds the affected circuit regions.
//!
//! [`RewriteConfig::production`] selects the conservative optimization mode
//! used by normal compiler cleanup. It enables simplification, cancellation,
//! merge, and canonicalization rules and accepts rewrites only under the local
//! cost model. [`RewriteConfig::lowering`] enables decomposition and
//! hardware-native rules as well, allowing local expansion when the caller has
//! requested explicit target-basis lowering.
//!
//! Target-basis translation is expressed by calling
//! [`RewriteConfig::with_target_instructions`]. The rewriter then rejects a
//! lowering result that still contains gate-like instructions outside the
//! configured basis.
//!
//! The pass recurses into supported structured classical-control bodies. It
//! does not synthesize arbitrary matrix-backed unitaries, choose layouts, route
//! through device topology, or perform directed-coupling legalization.

mod basis;
mod config;
mod edit;
mod edit_validation;
mod matcher;
mod proof;
mod rewriter;
mod session;

pub use config::{RewriteConfig, RewriteMode};
pub(crate) use edit::{OperationReplacement, QubitBijection, RewriteEdits};
pub use rewriter::{
    KnowledgeRewriteResult, KnowledgeRewriteStats, KnowledgeRewriter, rewrite_circuit,
};
pub(crate) use session::KnowledgeRewriteSession;
#[cfg(test)]
pub(crate) use session::KnowledgeRewriteWorksetStats;

#[cfg(test)]
#[path = "./rewrite_test.rs"]
mod rewrite_test;
