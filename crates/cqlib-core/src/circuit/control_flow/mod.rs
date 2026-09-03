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

//! Expression-based classical control-flow IR.
//!
//! This module defines the third layer of cqlib's dynamic-circuit classical
//! control model: structured control-flow operations that consume typed,
//! side-effect-free classical expressions.
//!
//! The model is intentionally split into three layers:
//!
//! 1. Runtime classical values and storage are defined by
//!    [`crate::circuit::classical`]. Those types identify circuit-local
//!    classical storage and value types.
//! 2. Classical expressions are defined by
//!    [`ClassicalExpr`](crate::circuit::ClassicalExpr).
//!    Expressions are strongly typed and have no side effects. They only read
//!    runtime classical variables, combine literals, cast explicitly, compare
//!    values, select between values, or extract/pack bits.
//! 3. Control-flow operations are defined in this module by
//!    [`ClassicalControlOp`] and the concrete operation structs. They consume
//!    [`ClassicalExpr`](crate::circuit::ClassicalExpr) values to decide which
//!    structured body executes.
//!
//! This module is IR only. It does not allocate classical storage, execute a
//! circuit, or lower control flow to backend instructions.
//!
//! # Core Concepts
//!
//! [`ClassicalExpr`](crate::circuit::ClassicalExpr) represents a runtime classical computation. It has a
//! static [`crate::circuit::ClassicalType`] and can report the classical
//! variables and immutable values it reads. The expression layer does not write
//! storage, measure qubits, branch, or loop.
//!
//! [`ControlBody`] owns the operations inside a structured control-flow region.
//! A body is a sequence of ordinary circuit [`crate::circuit::Operation`] values
//! that belongs to an operation such as [`IfOp`], [`WhileOp`], [`ForOp`], or
//! [`SwitchOp`].
//!
//! [`ClassicalControlOp`] is the sum type for structured classical control
//! operations:
//!
//! - [`IfOp`] executes a `then` body, and optionally an `else` body, when a
//!   boolean condition is evaluated at runtime.
//! - [`WhileOp`] repeats a body while a boolean condition remains true.
//! - [`ForOp`] models an unsigned half-open runtime range loop,
//!   `[start, stop)`, with an explicit unsigned loop variable and step.
//! - [`SwitchOp`] selects one body by matching an unsigned expression against
//!   exact case values. Cases do not fall through.
//! - [`ClassicalControlOp::Break`] exits the nearest enclosing loop or switch.
//! - [`ClassicalControlOp::Continue`] advances the nearest enclosing loop.
//!
//! Conditions for [`IfOp`] and [`WhileOp`] must have type `Bool`; measured
//! `Bit` values must be explicitly cast with
//! [`ClassicalExpr::bit_to_bool`](crate::circuit::ClassicalExpr::bit_to_bool).
//! Ordered comparisons are restricted to `UInt` expressions. Bit-vector to
//! integer interpretation is explicit through
//! [`ClassicalExpr::bit_vec_to_uint`](crate::circuit::ClassicalExpr::bit_vec_to_uint),
//! preserving the rule that bit index `0` is the least-significant bit.
//!
//! # Resource Queries
//!
//! Control-flow IR nodes expose lightweight dependency queries:
//!
//! - `classical_var_reads()` returns classical variables read by controlling
//!   expressions.
//! - `classical_value_reads()` returns immutable classical values read by
//!   controlling expressions.
//! - `classical_writes()` currently reports direct writes introduced by the
//!   control operation itself, such as a [`ForOp`] loop variable.
//! - `used_qubits()` reports qubits referenced directly by structured bodies.
//!
//! These methods are structural summaries for validation, remapping, and future
//! lowering passes. They are not a complete data-flow analysis for nested
//! dynamic-circuit execution.
//!
//! Classical handles are circuit-local. When circuits are composed, classical
//! variables and values from the source circuit are copied into the destination
//! circuit's classical tables and every expression/control body reference is
//! remapped to the new local ID.
//!
//! # Examples
//!
//! Build a boolean condition from a measured bit-like variable:
//!
//! ```text
//! bit_var: ClassicalVar<Bit>
//!
//! bit_expr  = ClassicalExpr::var(bit_var)
//! condition = ClassicalExpr::bit_to_bool(bit_expr)
//! if_op     = IfOp::new(condition, then_body, else_body)
//! ```
//!
//! Build an unsigned comparison for a loop or branch condition:
//!
//! ```text
//! counter: ClassicalVar<UInt(8)>
//!
//! lhs       = ClassicalExpr::var(counter)
//! rhs       = ClassicalExpr::uint_literal(8, 10)
//! condition = ClassicalExpr::lt(lhs, rhs)
//! while_op  = WhileOp::new(condition, body)
//! ```
//!
//! Build a switch over an unsigned runtime expression:
//!
//! ```text
//! state: ClassicalVar<UInt(2)>
//!
//! target = ClassicalExpr::var(state)
//! cases  = [
//!   SwitchCase::new(0, zero_body),
//!   SwitchCase::new(1, one_body),
//!   SwitchCase::new(2, two_body),
//! ]
//! switch = SwitchOp::new(target, cases, default_body)
//! ```
//!
//! Build a runtime `for` loop:
//!
//! ```text
//! i: ClassicalVar<UInt(8)>
//!
//! start = ClassicalExpr::uint_literal(8, 0)
//! stop  = ClassicalExpr::uint_literal(8, 16)
//! step  = ClassicalExpr::uint_literal(8, 1)
//! for_op = ForOp::new(i, start, stop, step, body)
//! ```
//!
//! # Design Boundaries
//!
//! This module deliberately does not define post-selection or runtime
//! assertions as first-class control-flow operations. Post-selection belongs to
//! execution/result filtering semantics, while assertions belong to diagnostics
//! or verification. They can be added later as separate IR families once their
//! backend behavior and failure model are specified.

mod body;
mod control_op;
mod for_op;
mod if_op;
mod switch_op;
mod validation;
mod while_op;

pub use body::ControlBody;
pub use control_op::{ClassicalControlKind, ClassicalControlOp};
pub use for_op::ForOp;
pub use if_op::IfOp;
pub use switch_op::{SwitchCase, SwitchOp};
pub(crate) use validation::{validate_for_types, validate_switch};
pub use while_op::WhileOp;
