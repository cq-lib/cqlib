// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! # Quantum Circuit Module
//!
//! Core data structures for representing, constructing, and manipulating quantum
//! circuits. The [`Circuit`] type is the primary IR container, supporting:
//!
//! - **Static circuits**: fixed gate sequences with concrete numeric parameters.
//! - **Parameterized circuits**: symbolic gate angles for variational algorithms
//!   (VQE, QAOA, quantum machine learning).
//! - **Dynamic circuits**: runtime classical control flow driven by mid-circuit
//!   measurements — conditionals, loops, and switches over typed classical
//!   expressions.
//!
//! ## Module Map
//!
//! | Submodule | Purpose | Key types |
//! |-----------|---------|-----------|
//! | [`circuit_impl`] | Circuit container and builder API | [`Circuit`] |
//! | [`bit`] | Qubit identifier and conversions | [`Qubit`], [`QubitError`] |
//! | [`gate`] | Gate definitions, matrix generation, arity | [`StandardGate`], [`Instruction`], [`UnitaryGate`], [`MCGate`], [`Directive`], [`ClassicalDataOp`] |
//! | [`operation`] | Storage-IR operation (instruction + qubits + params) | [`Operation`] |
//! | [`value_instruction`] | Construction-IR operation tree, pre-insertion | [`ValueOperation`], [`ValueInstruction`], [`ValueClassicalControlOp`] |
//! | [`circuit_param`] | Parameter representation in both IR layers | [`ParameterValue`] (construction), [`CircuitParam`] (storage) |
//! | [`parameter`] | Symbolic/numeric parameter expressions | [`Parameter`] |
//! | [`classical`] | Runtime classical storage and type system | [`ClassicalVar`], [`ClassicalValue`], [`ClassicalType`], [`Measurement`] |
//! | [`classical_expr`] | Side-effect-free typed classical expression AST | [`ClassicalExpr`], [`ClassicalBinaryOp`], [`ClassicalCompareOp`] |
//! | [`control_flow`] | Structured classical control-flow IR | [`IfOp`], [`WhileOp`], [`ForOp`], [`SwitchOp`], [`ClassicalControlOp`] |
//! | [`cfg`] | Structured control-flow graph view | [`CircuitCFG`] |
//! | [`circuit_verify`] | Classical-data and control-flow validation | [`Circuit::validate`] |
//! | [`circuit_to_matrix`] | Dense unitary matrix computation | [`circuit_to_matrix()`] |
//! | [`ansatz`] | Variational circuit templates | `Ansatz` trait, `TwoLocal`, `QAOAAnsatz` |
//! | [`symbolic_matrix`] | Dense symbolic unitary for small subcircuits | Symbolic gate and matrix types |
//! | [`error`] | Unified error type catalog | [`CircuitError`] |
//!
//! ## Quick Start
//!
//! ### Static circuit: Bell state → unitary matrix
//!
//! ```
//! use cqlib_core::circuit::{Circuit, Qubit};
//!
//! let mut c = Circuit::new(2);
//! c.h(Qubit::new(0)).unwrap();
//! c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
//! assert_eq!(c.num_qubits(), 2);
//! assert_eq!(c.operations().len(), 2);
//! ```
//!
//! ### Parameterized circuit: bind symbolic angles
//!
//! ```
//! use cqlib_core::circuit::{Circuit, Qubit, Parameter};
//! use std::collections::HashMap;
//!
//! let theta = Parameter::symbol("θ");
//! let mut c = Circuit::new(1);
//! c.rx(Qubit::new(0), theta.clone()).unwrap();
//! c.rz(Qubit::new(0), theta).unwrap();
//!
//! // Resolve symbols to concrete values
//! let mut bindings = HashMap::new();
//! bindings.insert("θ", std::f64::consts::PI);
//! let evaluated = c.assign_parameters(&Some(bindings)).unwrap();
//! ```
//!
//! ### Dynamic circuit: mid-circuit measurement with conditional gate
//!
//! ```
//! use cqlib_core::circuit::{Circuit, Qubit};
//!
//! let mut c = Circuit::new(2);
//! let q0 = Qubit::new(0);
//! let q1 = Qubit::new(1);
//!
//! c.h(q0).unwrap();
//! let m = c.measure(q0).unwrap();        // → Bit
//!
//! // Apply X on q1 only when q0 measured as |1⟩
//! c.if_(m.expr().to_bool().unwrap(), |body| {
//!     body.x(q1)?;
//!     Ok(())
//! }).unwrap();
//! ```
//!
//! ## Architecture
//!
//! The circuit IR has two layers designed to separate construction ergonomics
//! from storage efficiency:
//!
//! | Layer | Operation type | Parameter type | Purpose |
//! |-------|---------------|----------------|---------|
//! | **Construction IR** | [`ValueOperation`] / [`ValueInstruction`] | [`ParameterValue`] | Self-contained, pre-insertion builder |
//! | **Storage IR** | [`Operation`] / [`Instruction`] | [`CircuitParam`] | Compact, interned, post-insertion storage |
//!
//! [`Circuit::from_operations`] is the sole bridge: it recursively interns
//! symbolic [`Parameter`] values into the circuit's [`IndexSet`] parameter
//! table and replaces them with stable [`CircuitParam::Index`] references.
//! Indexed parameters are never exposed to construction-IR callers, preventing
//! dangling references.
//!
//! ### Validation
//!
//! Use [`Circuit::validate`] after construction or after loading external
//! circuit IR. It checks:
//!
//! - Classical handle ownership (no foreign circuit handles)
//! - Immutable [`ClassicalValue`] dominates all use sites (SSA)
//! - Scoped values do not escape their defining control-flow regions
//! - [`break`](ClassicalControlOp::Break) and [`continue`](ClassicalControlOp::Continue)
//!   appear only in valid terminal positions
//!
//! Gate arity and qubit membership are validated eagerly during [`Circuit::append`],
//! not deferred to post-hoc validation.
//!
//! ## Error Handling
//!
//! All construction methods that can fail return [`Result<_, CircuitError>`].
//! Common error variants:
//!
//! | Error variant | When |
//! |--------------|------|
//! | [`QubitNotFound`] | Qubit not registered in the circuit |
//! | [`QubitCountMismatch`] / [`ParameterCountMismatch`] | Wrong arity for fixed-arity instruction |
//! | [`InvalidParameterValue`] | Non-finite (NaN/Inf) fixed parameter |
//! | [`DuplicateQubits`] | Same qubit used twice in one operation |
//! | [`ForeignClassicalHandle`] | Handle from another circuit |
//! | [`UndefinedClassicalValue`] | Read of uninitialized classical value |
//!
//! See the [`error`] module for the full catalog with per-variant documentation.
//!
//! [`QubitNotFound`]: CircuitError::QubitNotFound
//! [`QubitCountMismatch`]: CircuitError::QubitCountMismatch
//! [`ParameterCountMismatch`]: CircuitError::ParameterCountMismatch
//! [`InvalidParameterValue`]: CircuitError::InvalidParameterValue
//! [`DuplicateQubits`]: CircuitError::DuplicateQubits
//! [`ForeignClassicalHandle`]: CircuitError::ForeignClassicalHandle
//! [`UndefinedClassicalValue`]: CircuitError::UndefinedClassicalValue

pub mod ansatz;
pub mod bit;
pub mod cfg;
mod circuit_classical;
pub mod circuit_impl;
pub mod circuit_param;
pub mod circuit_to_matrix;
pub mod circuit_verify;
pub mod classical;
pub mod classical_expr;
pub mod control_flow;
pub mod dag;
pub mod depth;
pub mod error;
pub mod gate;
pub mod operation;
pub mod parameter;
pub mod symbolic_matrix;
pub mod value_instruction;

pub use bit::{Qubit, QubitError};
pub use cfg::CircuitCFG;
pub use circuit_classical::SwitchBuilder;
#[doc(hidden)]
pub use circuit_classical::{ControlBodyTransaction, ExternalControlScope};
pub use circuit_impl::Circuit;
pub use circuit_param::{CircuitParam, ParameterValue};
pub use circuit_to_matrix::circuit_to_matrix;
pub use classical::{CircuitId, ClassicalType, ClassicalValue, ClassicalVar, Measurement};
pub use classical_expr::{
    ClassicalBinaryOp, ClassicalCast, ClassicalCompareOp, ClassicalExpr, ClassicalExprKind,
    ClassicalExprNode, ClassicalUnaryOp,
};
pub use control_flow::{
    ClassicalControlOp, ControlBody, ForOp, IfOp, SwitchCase, SwitchOp, WhileOp,
};
pub use dag::{CircuitDag, DagControlFlow, DagNode, DagSwitchCase, DagWire};
pub use error::CircuitError;
pub use gate::circuit_gate::CircuitGate;
pub use gate::classical_data::ClassicalDataOp;
pub use gate::directive::Directive;
pub use gate::instruction::Instruction;
pub use gate::mc_gate::MCGate;
pub use gate::standard_gate::StandardGate;
pub use gate::unitary_gate::UnitaryGate;
pub use operation::{Operation, ValueOperation};
pub use parameter::Parameter;
pub use value_instruction::{
    ValueClassicalControlOp, ValueControlBody, ValueInstruction, ValueSwitchCase,
    storage_operation_to_value,
};
