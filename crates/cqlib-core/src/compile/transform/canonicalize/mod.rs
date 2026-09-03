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

//! Circuit IR canonicalization for the new compiler.
//!
//! Canonicalization is the first logical transform over a [`Circuit`]. It is
//! deliberately narrower than optimization, decomposition, routing, scheduling,
//! target-basis translation, or hardware-aware legalization. The pass rebuilds
//! a circuit into the compiler's stable logical representation and validates the
//! result before it can be used by later passes.
//!
//! [`Circuit`]: crate::circuit::Circuit
//!
//! # Scope
//!
//! The canonicalizer owns representation-level cleanup only:
//!
//! - validates qubit references, parameter references, instruction arity, and
//!   finite fixed numeric parameters;
//! - rebuilds and simplifies the parameter table, folding constant expressions
//!   into `CircuitParam::Fixed` values and removing unused parameters;
//! - normalizes circuit-level global phase;
//! - folds top-level zero-qubit `GPhase` markers into `Circuit::global_phase`;
//! - represents control-flow body-local phase as an optional leading zero-qubit
//!   `GPhase` marker in that body;
//! - rewrites multi-controlled gates into exact existing `StandardGate` forms
//!   when the IR has such a form, without decomposition;
//! - removes strict no-ops, including labeled no-ops and self-stores
//!   (`store v <- v`) on classical variables;
//! - canonicalizes barrier scopes and merges adjacent barriers when their scopes
//!   are equal or one scope is a superset of the other;
//! - preserves `ClassicalData` instructions such as stores and measurements
//!   while validating their qubit width and parameter shape;
//! - simplifies runtime classical expressions embedded in `ClassicalControl`
//!   and `ClassicalData` operations via [`ClassicalExpr::simplified`] (e.g.
//!   `not(not(x)) → x`, `eq(x, x) → true`, `and(x, true) → x`);
//! - recursively canonicalizes structured `ClassicalControl` bodies for `if`,
//!   `while`, `for`, and `switch`, preserving `break`/`continue` markers and
//!   recomputing each outer control-flow operation qubit list in circuit-global
//!   qubit order.
//!
//! [`ClassicalExpr::simplified`]: crate::circuit::classical_expr::ClassicalExpr::simplified
//!
//! # Non-Goals
//!
//! This module must not perform approximate optimization, commutation-based
//! rewrites, target-basis translation, high-level synthesis, KAK/Euler
//! decomposition, routing, layout selection, gate-direction correction, or
//! hardware angle-bound wrapping. Those belong to later compiler stages with
//! different contracts and validation.
//!
//! # Phase Policy
//!
//! `GPhase` is a zero-qubit phase marker, not a qubit-targeted operation. A
//! `GPhase` operation with non-empty operands is invalid by the ordinary
//! instruction arity rules.
//!
//! Top-level `GPhase` markers are not part of production canonical output. They
//! are accumulated into `Circuit::global_phase`, which is the canonical
//! representation for whole-circuit global phase. Inside control-flow bodies,
//! phase is branch- or loop-local and cannot be lifted to the circuit global
//! phase without changing semantics. Canonical output therefore permits a
//! `GPhase` marker only as the first operation of a control-flow body, where it
//! represents body-local phase. Multiple body-local phases are merged into that
//! leading marker; zero phase is removed.
//!
//! # Production Output Contract
//!
//! With [`CanonicalizeConfig::production`], a successful canonicalization run
//! guarantees:
//!
//! - all operation references are valid and arity-correct;
//! - fixed parameters are finite;
//! - top-level operations contain no `GPhase`;
//! - any retained `GPhase` is a zero-qubit marker;
//! - a control-flow body contains `GPhase` only at index `0`, and only when its
//!   phase is nonzero;
//! - no removable strict no-op remains (including self-stores on classical
//!   variables);
//! - local barrier scopes are sorted and deduplicated; an empty scope denotes a
//!   global barrier over every circuit qubit; all barriers are label-free and
//!   no adjacent barrier pair is mergeable;
//! - the parameter and symbol tables are consistent and contain no unused
//!   parameter entries;
//! - classical variable and value tables are preserved with stable handles;
//!   unused entries may remain after expression simplification or no-op removal;
//! - controlling expressions in `if`/`while`/`for`/`switch` and stored
//!   expressions in `ClassicalDataOp::Store` are in simplified form;
//! - running canonicalization a second time is unchanged.
//!
//! The configuration type exposes a small set of behavior switches for focused
//! testing and staged compiler integration. When a switch is disabled, the
//! corresponding production guarantee is intentionally relaxed: disabling
//! `fold_gphase` permits top-level `GPhase`, disabling `drop_noops` permits
//! strict no-ops, disabling `canonicalize_barriers` preserves labels and operand
//! order for barriers that are not removed by the no-op policy, and disabling
//! `recurse_control_flow` leaves control-flow bodies in their input
//! representation. Parameter rebuilding and simplification remain part of the
//! canonicalization contract in all configurations.
//!
//! # Quantum-Classical Example
//!
//! The pass treats structured classical control as first-class circuit IR.
//! Controlling expressions are simplified inline, body-level no-ops are
//! removed, `GPhase` is folded, and the outer qubit list is recomputed from
//! the canonicalized body.
//!
//! ```rust
//! use cqlib_core::circuit::{Circuit, ClassicalExpr, ClassicalType, Qubit};
//! use cqlib_core::compile::transform::canonicalize_circuit;
//!
//! let mut circuit = Circuit::new(2);
//! // eq(x, x) → true after simplification
//! let x = ClassicalExpr::uint_literal(8, 42).unwrap();
//! let tautology = ClassicalExpr::eq(x.clone(), x).unwrap();
//!
//! circuit.if_(tautology, |body| {
//!     body.i(Qubit::new(1))?;    // no-op, removed
//!     body.h(Qubit::new(1))?;
//!     Ok(())
//! }).unwrap();
//!
//! let result = canonicalize_circuit(&circuit).unwrap();
//! // Body: I-gate removed, condition simplified to `true`
//! assert_eq!(result.circuit.operations().len(), 1);
//! // Only the qubit actually used (q[1]) appears in the outer qubit list
//! assert_eq!(
//!     result.circuit.operations()[0].qubits.as_slice(),
//!     &[Qubit::new(1)],
//! );
//! ```

mod canonicalizer;
mod config;
mod ops;
mod verify;

pub use canonicalizer::{CanonicalizeResult, Canonicalizer, canonicalize_circuit};
pub use config::CanonicalizeConfig;

#[cfg(test)]
mod canonicalize_test;
