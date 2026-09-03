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

//! Global self-inverse gate cancellation over commutation sets.
//!
//! The knowledge-based rewriter matches bounded windows (patterns of at most
//! eight operations, scanned within sixteen-operation windows), so identical
//! self-inverse gates separated by longer stretches of exactly-commuting
//! operations can never be paired. This transform performs an unbounded
//! commutation-set analysis instead: each wire's operation sequence is
//! partitioned into sets of pairwise exactly-commuting operations and
//! self-inverse gates from `{CX, CY, CZ, H, Y, X, Z}` are cancelled pairwise
//! within a set. Pairwise deletion is an exact identity and introduces no
//! global phase.
//!
//! Labeled operations, non-unitary instructions, and structured classical
//! control are hard barriers; control-flow bodies are analyzed recursively.
//! A single run does not reach a fixed point (deletions can expose new
//! adjacent pairs). Every run only removes operations, and bounded workflow
//! re-runs expose and remove additional pairs, without guaranteeing a full
//! fixed point for arbitrarily nested inputs.

mod pass;
mod sets;

pub use pass::CommutativeCancellation;
