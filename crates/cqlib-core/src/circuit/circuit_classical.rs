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

#![allow(rustdoc::private_doc_tests)]

//! Circuit builder support for runtime classical data and structured control flow.
//!
//! This module is the circuit-level API layer for dynamic circuits. It connects
//! three lower-level concepts:
//!
//! - [`ClassicalValue`]: immutable runtime results produced by operations such
//!   as measurement. Values are SSA-like: once produced, they are never
//!   overwritten.
//! - [`Measurement`]: self-contained receipt returned by measurement builders.
//!   It contains both the immutable [`ClassicalValue`] and the measured qubits
//!   in result bit order.
//! - [`ClassicalVar`]: mutable runtime storage slots allocated by a circuit.
//!   Variables are used when a classical result must be stored, reused, or
//!   updated across loop iterations.
//! - [`ClassicalExpr`]: typed, side-effect-free expressions that read values
//!   and variables. Control-flow operations consume expressions; they do not
//!   read mutable storage directly.
//!
//! The public API intentionally keeps measurement, storage, and control flow as
//! separate concepts:
//!
//! - [`Circuit::measure`] and [`Circuit::measure_bits`] append measurement
//!   operations and return [`Measurement`] handles.
//! - [`Circuit::store`] copies an expression result into mutable
//!   [`ClassicalVar`] storage.
//! - Structured control-flow operations consume [`ClassicalExpr`] values.
//!
//! This separation allows a measurement result to drive control flow directly
//! without allocating mutable storage. A variable is needed only when a value
//! must be overwritten, carried across loop iterations, or retained as named
//! runtime state.
//!
//! # Measurement model
//!
//! Every measurement remains an operation in the circuit schedule. Its qubits,
//! position, result type, and immutable result handle are therefore preserved
//! by the circuit IR. The returned [`Measurement`] identifies that specific
//! measurement operation and exposes the measured qubit order; measuring the
//! same qubit again creates a different value.
//!
//! [`Circuit::measure`] produces a [`ClassicalType::Bit`].
//! [`Circuit::measure_bits`] produces a [`ClassicalType::BitVec`] whose width is
//! the number of measured qubits. For a multi-qubit measurement, input order is
//! bit-vector order: the first qubit maps to bit index `0`, the least-significant
//! bit.
//!
//! Use [`Measurement::value`] or [`Measurement::expr`] when the IR value is
//! needed for control flow or storage. Use [`Measurement::qubits`] when a state
//! sampler needs to know which qubits to sample and how to order the result.
//!
//! Measurement results are runtime values, not compile-time booleans. Convert a
//! measured `Bit` explicitly with [`ClassicalExpr::bit_to_bool`] before using it
//! as an `if` or `while` condition.
//!
//! The circuit IR records measurements and their data-flow uses, but it does not
//! define a result presentation policy. Selecting result fields, grouping them
//! into user-visible registers, and formatting bit strings belong to the
//! execution/result layer rather than these builder APIs.
//!
//! # Measurement value used directly by `if`
//!
//! ```rust
//! use cqlib_core::circuit::{
//!     Circuit, CircuitError, ClassicalExpr, Qubit,
//! };
//!
//! fn build() -> Result<Circuit, CircuitError> {
//!     let mut circuit = Circuit::new(2);
//!     let q0 = Qubit::new(0);
//!     let q1 = Qubit::new(1);
//!
//!     let measured = circuit.measure(q0)?;
//!     let condition = ClassicalExpr::bit_to_bool(measured.expr())?;
//!
//!     circuit.if_(condition, |body| {
//!         body.x(q1)?;
//!         Ok(())
//!     })?;
//!
//!     Ok(circuit)
//! }
//! ```
//!
//! Expected circuit shape:
//!
//! ```text
//! measure_bit q[0] -> v0: Bit
//! if bit_to_bool(v0) {
//!     x q[1]
//! }
//! ```
//!
//! The measurement value is immutable and belongs to this circuit. It can be
//! read by any later expression for which it is available, but it cannot be
//! overwritten.
//!
//! # Multi-qubit measurement and bit order
//!
//! ```rust
//! use cqlib_core::circuit::{Circuit, CircuitError, Qubit};
//!
//! fn build() -> Result<Circuit, CircuitError> {
//!     let mut circuit = Circuit::new(3);
//!     let measured = circuit.measure_bits([
//!         Qubit::new(2),
//!         Qubit::new(0),
//!         Qubit::new(1),
//!     ])?;
//!
//!     assert_eq!(measured.ty().width(), 3);
//!     Ok(circuit)
//! }
//! ```
//!
//! The result layout is:
//!
//! ```text
//! result[0] = measure(q[2])  // least-significant bit
//! result[1] = measure(q[0])
//! result[2] = measure(q[1])  // most-significant bit
//! ```
//!
//! # Measuring into mutable storage
//!
//! [`Circuit::measure_into`] and [`Circuit::measure_bits_into`] are convenience
//! operations. They append the same measurement as [`Circuit::measure`] or
//! [`Circuit::measure_bits`], then append a [`Circuit::store`] into the supplied
//! variable. They still return the same [`Measurement`] handle.
//!
//! ```rust
//! use cqlib_core::circuit::{
//!     Circuit, CircuitError, ClassicalType, Qubit,
//! };
//!
//! fn build() -> Result<Circuit, CircuitError> {
//!     let mut circuit = Circuit::new(2);
//!     let latest = circuit.var(ClassicalType::bit_vec(2).unwrap());
//!
//!     let measured = circuit.measure_bits_into(
//!         [Qubit::new(0), Qubit::new(1)],
//!         latest,
//!     )?;
//!
//!     assert_eq!(measured.ty(), latest.ty());
//!     Ok(circuit)
//! }
//! ```
//!
//! Conceptually, this appends:
//!
//! ```text
//! measure_bits q[0], q[1] -> v0: BitVec(2)
//! store s0 <- v0
//! ```
//!
//! The measurement handle's immutable value `v0` always denotes this
//! measurement occurrence. The variable `s0` denotes mutable storage and may be
//! overwritten later, which makes it suitable for loop-carried or reusable
//! runtime state. Measuring into a variable does not by itself declare how an
//! executor presents results.
//!
//! # Storing reusable classical state
//!
//! Use [`Circuit::var`] and [`Circuit::store`] when a classical expression must
//! be preserved or updated later.
//!
//! ```rust
//! use cqlib_core::circuit::{
//!     Circuit, CircuitError, ClassicalExpr, ClassicalType, Qubit,
//! };
//!
//! fn build() -> Result<Circuit, CircuitError> {
//!     let q0 = Qubit::new(0);
//!     let q1 = Qubit::new(1);
//!     let mut circuit = Circuit::from_qubits(vec![q0, q1])?;
//!     circuit.h(q0)?;
//!
//!     let flag = circuit.var(ClassicalType::Bool);
//!     let measured = circuit.measure(q0)?;
//!     let condition = ClassicalExpr::bit_to_bool(measured.expr())?;
//!     circuit.store(flag, condition)?;
//!
//!     circuit.if_(flag.expr(), |body| {
//!         body.z(q1)?;
//!         Ok(())
//!     })?;
//!
//!     Ok(circuit)
//! }
//! ```
//!
//! Expected circuit shape:
//!
//! ```text
//! var s0: Bool
//! measure_bit q[0] -> v0: Bit
//! store s0 <- bit_to_bool(v0)
//! if s0 {
//!     z q[1]
//! }
//! ```
//!
//! # Loop-carried classical state
//!
//! A loop condition should read a variable when the condition is updated inside
//! the loop body.
//!
//! ```rust,no_run
//! use cqlib_core::circuit::{
//!     Circuit, CircuitError, ClassicalExpr, ClassicalType, Qubit,
//! };
//!
//! fn build() -> Result<Circuit, CircuitError> {
//!     let mut circuit = Circuit::new(1);
//!     let q0 = Qubit::new(0);
//!
//!     let keep_running = circuit.var(ClassicalType::Bool);
//!     circuit.store(keep_running, ClassicalExpr::bool_literal(true))?;
//!
//!     circuit.while_(keep_running.expr(), |body| {
//!         let measured = body.measure(q0)?;
//!         let next = ClassicalExpr::bit_to_bool(measured.expr())?;
//!         body.store(keep_running, next)?;
//!         Ok(())
//!     })?;
//!
//!     Ok(circuit)
//! }
//! ```
//!
//! Expected circuit shape:
//!
//! ```text
//! var s0: Bool
//! store s0 <- true
//! while s0 {
//!     measure_bit q[0] -> v0: Bit
//!     store s0 <- bit_to_bool(v0)
//! }
//! ```
//!
//! # `if_else`
//!
//! ```rust,no_run
//! use cqlib_core::circuit::{
//!     Circuit, CircuitError, ClassicalExpr, Qubit,
//! };
//!
//! fn build() -> Result<Circuit, CircuitError> {
//!     let mut circuit = Circuit::new(2);
//!     let q0 = Qubit::new(0);
//!     let q1 = Qubit::new(1);
//!     let measured = circuit.measure(q0)?;
//!     let condition = ClassicalExpr::bit_to_bool(measured.expr())?;
//!
//!     circuit.if_else(
//!         condition,
//!         |then_body| {
//!             then_body.x(q1)?;
//!             Ok(())
//!         },
//!         |else_body| {
//!             else_body.z(q1)?;
//!             Ok(())
//!         },
//!     )?;
//!
//!     Ok(circuit)
//! }
//! ```
//!
//! Expected circuit shape:
//!
//! ```text
//! measure_bit q[0] -> v0: Bit
//! if bit_to_bool(v0) {
//!     x q[1]
//! } else {
//!     z q[1]
//! }
//! ```
//!
//! # Unsigned `for` loop
//!
//! `for_uint` models a runtime unsigned range loop. The loop variable is a
//! mutable [`ClassicalVar`], and the closure receives its read expression.
//!
//! ```rust,no_run
//! use cqlib_core::circuit::{
//!     Circuit, CircuitError, ClassicalExpr, ClassicalType, Qubit,
//! };
//!
//! fn build() -> Result<Circuit, CircuitError> {
//!     let mut circuit = Circuit::new(1);
//!     let q0 = Qubit::new(0);
//!     let i = circuit.var(ClassicalType::uint(8).unwrap());
//!
//!     circuit.for_uint(
//!         i,
//!         ClassicalExpr::uint_literal(8, 0)?,
//!         ClassicalExpr::uint_literal(8, 4)?,
//!         ClassicalExpr::uint_literal(8, 1)?,
//!         |body, _i_expr| {
//!             body.h(q0)?;
//!             Ok(())
//!         },
//!     )?;
//!
//!     Ok(circuit)
//! }
//! ```
//!
//! Expected circuit shape:
//!
//! ```text
//! var s0: UInt(8)
//! for s0 in 0_u8..4_u8 step 1_u8 {
//!     h q[0]
//! }
//! ```
//!
//! # `switch`
//!
//! [`SwitchBuilder`] is the only builder object kept in this module. It is a
//! temporary case collector for [`Circuit::switch`]; it does not represent a
//! general scoped circuit.
//!
//! ```rust,no_run
//! use cqlib_core::circuit::{
//!     Circuit, CircuitError, ClassicalExpr, ClassicalType, Qubit,
//! };
//!
//! fn build() -> Result<Circuit, CircuitError> {
//!     let mut circuit = Circuit::new(2);
//!     let state = circuit.var(ClassicalType::uint(2).unwrap());
//!     circuit.store(state, ClassicalExpr::uint_literal(2, 1)?)?;
//!
//!     circuit.switch(state.expr(), |case| {
//!         case.value(0, |body| {
//!             body.x(Qubit::new(0))?;
//!             Ok(())
//!         })?;
//!         case.value(1, |body| {
//!             body.h(Qubit::new(1))?;
//!             Ok(())
//!         })?;
//!         case.value(2, |body| {
//!             body.h(Qubit::new(1))?;
//!             Ok(())
//!         })?;
//!         case.default(|body| {
//!             body.z(Qubit::new(0))?;
//!             Ok(())
//!         })?;
//!         Ok(())
//!     })?;
//!
//!     Ok(circuit)
//! }
//! ```
//!
//! Expected circuit shape:
//!
//! ```text
//! var s0: UInt(2)
//! store s0 <- 1_u2
//! switch s0 {
//!     case 0:
//!         x q[0]
//!     case 1:
//!         h q[1]
//!     case 2:
//!         h q[1]
//!     default:
//!         z q[0]
//! }
//! ```
//!
//! Each case stores exactly one integer label and one body. Sharing body
//! construction across multiple labels should be done by the caller if needed,
//! rather than by adding another builder abstraction here.
//!
//! # Internal validation
//!
//! `Circuit` keeps compact circuit-local type tables for classical variables
//! and values. These tables are not user-facing program objects; they validate
//! that expression handles belong to the circuit that consumes them and that
//! their static types match. The `control_scope_stack` is also builder-only
//! state: it validates that `break` and `continue` appear only in legal
//! structured-control bodies.

use crate::circuit::bit::Qubit;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::classical_expr::ClassicalExpr;
use crate::circuit::control_flow::{
    ClassicalControlOp, ControlBody, ForOp, IfOp, SwitchCase, SwitchOp, WhileOp,
};
use crate::circuit::error::CircuitError;
use crate::circuit::gate::instruction::Instruction;
use crate::circuit::{ClassicalDataOp, ClassicalType, ClassicalValue, ClassicalVar, Measurement};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ControlScopeKind {
    Loop,
    Switch,
}

/// Scope kind used by foreign-language bindings while building callback bodies.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalControlScope {
    Branch,
    Loop,
    Switch,
}

/// Temporary builder for the ordered cases of a structured `switch` operation.
pub struct SwitchBuilder<'a> {
    circuit: &'a mut Circuit,
    cases: Vec<SwitchCase>,
    default: Option<ControlBody>,
}

impl SwitchBuilder<'_> {
    /// Adds one exact-value case.
    pub fn value<F>(&mut self, value: u128, body: F) -> Result<(), CircuitError>
    where
        F: FnOnce(&mut Circuit) -> Result<(), CircuitError>,
    {
        let body = self
            .circuit
            .build_control_body(Some(ControlScopeKind::Switch), body)?;
        self.cases.push(SwitchCase::new(value, body));
        Ok(())
    }

    /// Adds the optional default case.
    pub fn default<F>(&mut self, body: F) -> Result<(), CircuitError>
    where
        F: FnOnce(&mut Circuit) -> Result<(), CircuitError>,
    {
        if self.default.is_some() {
            return Err(CircuitError::InvalidOperation(
                "switch default case is already defined".to_string(),
            ));
        }
        self.default = Some(
            self.circuit
                .build_control_body(Some(ControlScopeKind::Switch), body)?,
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CircuitCheckpoint {
    data_len: usize,
    parameter_len: usize,
    symbol_len: usize,
    classical_var_len: usize,
    classical_value_len: usize,
    control_scope_len: usize,
}

/// Opaque rollback token for callback-driven control-flow construction.
#[doc(hidden)]
pub struct ControlBodyTransaction {
    checkpoint: CircuitCheckpoint,
}

#[derive(Debug, Clone, Copy)]
struct ControlValidationContext {
    break_allowed: bool,
    continue_allowed: bool,
}

impl ControlValidationContext {
    fn from_scope_stack(scopes: &[ControlScopeKind]) -> Self {
        Self {
            break_allowed: scopes
                .iter()
                .rev()
                .any(|scope| matches!(scope, ControlScopeKind::Loop | ControlScopeKind::Switch)),
            continue_allowed: scopes
                .iter()
                .rev()
                .any(|scope| matches!(scope, ControlScopeKind::Loop)),
        }
    }

    fn enter_loop(self) -> Self {
        Self {
            break_allowed: true,
            continue_allowed: true,
        }
    }

    fn enter_switch(self) -> Self {
        Self {
            break_allowed: true,
            continue_allowed: self.continue_allowed,
        }
    }
}

impl Circuit {
    /// Allocates a mutable runtime classical variable.
    pub fn var(&mut self, ty: ClassicalType) -> ClassicalVar {
        self.allocate_classical_var(ty)
    }

    fn allocate_classical_var(&mut self, ty: ClassicalType) -> ClassicalVar {
        let id = self.classical_vars.len() as u32;
        self.classical_vars.push(ty);
        ClassicalVar::new(self.circuit_id, id, ty)
    }

    fn allocate_classical_value(&mut self, ty: ClassicalType) -> ClassicalValue {
        let id = self.classical_values.len() as u32;
        self.classical_values.push(ty);
        ClassicalValue::new(self.circuit_id, id, ty)
    }

    pub(super) fn checkpoint(&self) -> CircuitCheckpoint {
        CircuitCheckpoint {
            data_len: self.data.len(),
            parameter_len: self.parameters.len(),
            symbol_len: self.symbols.len(),
            classical_var_len: self.classical_vars.len(),
            classical_value_len: self.classical_values.len(),
            control_scope_len: self.control_scope_stack.len(),
        }
    }

    pub(super) fn rollback_to(&mut self, checkpoint: CircuitCheckpoint) {
        Arc::make_mut(&mut self.data).truncate(checkpoint.data_len);
        self.parameters.truncate(checkpoint.parameter_len);
        self.symbols.truncate(checkpoint.symbol_len);
        self.classical_vars.truncate(checkpoint.classical_var_len);
        self.classical_values
            .truncate(checkpoint.classical_value_len);
        self.control_scope_stack
            .truncate(checkpoint.control_scope_len);
    }

    #[doc(hidden)]
    pub fn validate_classical_var(&self, var: ClassicalVar) -> Result<(), CircuitError> {
        if var.circuit_id() != self.circuit_id {
            return Err(CircuitError::ForeignClassicalHandle {
                kind: "classical variable",
                index: var.index(),
            });
        }
        match self.classical_vars.get(var.id() as usize) {
            Some(ty) if *ty == var.ty() => Ok(()),
            Some(ty) => Err(CircuitError::InvalidOperation(format!(
                "classical variable {} has type {:?}, got {:?}",
                var.id(),
                ty,
                var.ty()
            ))),
            None => Err(CircuitError::InvalidOperation(format!(
                "classical variable {} is not allocated by this circuit",
                var.id()
            ))),
        }
    }

    pub(super) fn validate_classical_value(
        &self,
        value: ClassicalValue,
    ) -> Result<(), CircuitError> {
        if value.circuit_id() != self.circuit_id {
            return Err(CircuitError::ForeignClassicalHandle {
                kind: "classical value",
                index: value.index(),
            });
        }
        match self.classical_values.get(value.index() as usize) {
            Some(ty) if *ty == value.ty() => Ok(()),
            Some(ty) => Err(CircuitError::InvalidOperation(format!(
                "classical value {} has type {:?}, got {:?}",
                value.index(),
                ty,
                value.ty()
            ))),
            None => Err(CircuitError::InvalidOperation(format!(
                "classical value {} is not produced by this circuit",
                value.index()
            ))),
        }
    }

    #[doc(hidden)]
    pub fn validate_classical_expr(&self, expr: &ClassicalExpr) -> Result<(), CircuitError> {
        for var in expr.vars() {
            self.validate_classical_var(var)?;
        }
        for value in expr.values() {
            self.validate_classical_value(value)?;
        }
        Ok(())
    }

    fn validate_control_body_in(
        &self,
        body: &ControlBody,
        context: ControlValidationContext,
    ) -> Result<(), CircuitError> {
        for operation in body.operations() {
            for qubit in &operation.qubits {
                if !self.qubits.contains(qubit) {
                    return Err(CircuitError::QubitNotFound(qubit.id()));
                }
            }
            if let Instruction::ClassicalControl(op) = &operation.instruction {
                self.validate_control_op_in(op, context)?;
            }
            if let Instruction::ClassicalData(op) = &operation.instruction {
                self.validate_classical_data_op(op, operation.qubits.len())?;
            }
        }
        Ok(())
    }

    pub(super) fn validate_classical_data_op(
        &self,
        op: &ClassicalDataOp,
        qubit_count: usize,
    ) -> Result<(), CircuitError> {
        match op {
            ClassicalDataOp::Store { target, value } => {
                self.validate_classical_var(*target)?;
                if qubit_count != 0 {
                    return Err(CircuitError::QubitCountMismatch {
                        expected: 0,
                        actual: qubit_count,
                    });
                }
                self.validate_classical_expr(value)?;
                if value.ty() != target.ty() {
                    return Err(CircuitError::InvalidOperation(format!(
                        "store target type {:?} does not match value type {:?}",
                        target.ty(),
                        value.ty()
                    )));
                }
            }
            ClassicalDataOp::MeasureBit { result } => {
                self.validate_classical_value(*result)?;
                if qubit_count != 1 {
                    return Err(CircuitError::QubitCountMismatch {
                        expected: 1,
                        actual: qubit_count,
                    });
                }
                if result.ty() != ClassicalType::Bit {
                    return Err(CircuitError::InvalidOperation(format!(
                        "single-qubit measurement result must be Bit, got {:?}",
                        result.ty()
                    )));
                }
            }
            ClassicalDataOp::MeasureBits { result } => {
                self.validate_classical_value(*result)?;
                let expected = match result.ty() {
                    ClassicalType::BitVec(width) => width.get() as usize,
                    ty => {
                        return Err(CircuitError::InvalidOperation(format!(
                            "multi-qubit measurement result must be BitVec, got {ty:?}"
                        )));
                    }
                };
                if qubit_count != expected {
                    return Err(CircuitError::QubitCountMismatch {
                        expected,
                        actual: qubit_count,
                    });
                }
            }
        }
        Ok(())
    }

    pub(super) fn validate_control_op(&self, op: &ClassicalControlOp) -> Result<(), CircuitError> {
        self.validate_control_op_in(
            op,
            ControlValidationContext::from_scope_stack(&self.control_scope_stack),
        )
    }

    fn validate_control_op_in(
        &self,
        op: &ClassicalControlOp,
        context: ControlValidationContext,
    ) -> Result<(), CircuitError> {
        match op {
            ClassicalControlOp::If(op) => {
                self.validate_classical_expr(op.condition())?;
                self.validate_control_body_in(op.then_body(), context)?;
                if let Some(body) = op.else_body() {
                    self.validate_control_body_in(body, context)?;
                }
            }
            ClassicalControlOp::While(op) => {
                self.validate_classical_expr(op.condition())?;
                self.validate_control_body_in(op.body(), context.enter_loop())?;
            }
            ClassicalControlOp::For(op) => {
                self.validate_classical_var(op.var())?;
                self.validate_classical_expr(op.start())?;
                self.validate_classical_expr(op.stop())?;
                self.validate_classical_expr(op.step())?;
                self.validate_control_body_in(op.body(), context.enter_loop())?;
            }
            ClassicalControlOp::Switch(op) => {
                self.validate_classical_expr(op.target())?;
                let body_context = context.enter_switch();
                for case in op.cases() {
                    self.validate_control_body_in(case.body(), body_context)?;
                }
                if let Some(body) = op.default() {
                    self.validate_control_body_in(body, body_context)?;
                }
            }
            ClassicalControlOp::Break => {
                if !context.break_allowed {
                    return Err(CircuitError::InvalidOperation(
                        "break can only be used inside a loop or switch body".to_string(),
                    ));
                }
            }
            ClassicalControlOp::Continue => {
                if !context.continue_allowed {
                    return Err(CircuitError::InvalidOperation(
                        "continue can only be used inside a loop body".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn build_control_body<F>(
        &mut self,
        scope: Option<ControlScopeKind>,
        body: F,
    ) -> Result<ControlBody, CircuitError>
    where
        F: FnOnce(&mut Circuit) -> Result<(), CircuitError>,
    {
        let checkpoint = self.checkpoint();

        if let Some(scope) = scope {
            self.control_scope_stack.push(scope);
        }
        let result = body(self);
        self.control_scope_stack
            .truncate(checkpoint.control_scope_len);

        match result {
            Ok(()) => Ok(ControlBody::new(
                Arc::make_mut(&mut self.data).split_off(checkpoint.data_len),
            )),
            Err(error) => {
                self.rollback_to(checkpoint);
                Err(error)
            }
        }
    }

    /// Starts an externally driven control-flow construction transaction.
    #[doc(hidden)]
    pub fn begin_control_body_transaction(&self) -> ControlBodyTransaction {
        ControlBodyTransaction {
            checkpoint: self.checkpoint(),
        }
    }

    /// Enters one callback body within an external construction transaction.
    #[doc(hidden)]
    pub fn enter_external_control_body(
        &mut self,
        transaction: &ControlBodyTransaction,
        scope: ExternalControlScope,
    ) {
        self.control_scope_stack
            .truncate(transaction.checkpoint.control_scope_len);
        match scope {
            ExternalControlScope::Branch => {}
            ExternalControlScope::Loop => self.control_scope_stack.push(ControlScopeKind::Loop),
            ExternalControlScope::Switch => self.control_scope_stack.push(ControlScopeKind::Switch),
        }
    }

    /// Captures operations appended by the current callback body.
    #[doc(hidden)]
    pub fn finish_external_control_body(
        &mut self,
        transaction: &ControlBodyTransaction,
    ) -> ControlBody {
        self.control_scope_stack
            .truncate(transaction.checkpoint.control_scope_len);
        ControlBody::new(Arc::make_mut(&mut self.data).split_off(transaction.checkpoint.data_len))
    }

    /// Commits state allocated while constructing callback bodies.
    #[doc(hidden)]
    pub fn commit_control_body_transaction(&mut self, transaction: ControlBodyTransaction) {
        self.control_scope_stack
            .truncate(transaction.checkpoint.control_scope_len);
    }

    /// Rolls back all state allocated while constructing callback bodies.
    #[doc(hidden)]
    pub fn rollback_control_body_transaction(&mut self, transaction: ControlBodyTransaction) {
        self.rollback_to(transaction.checkpoint);
    }

    /// Stores a runtime classical expression into a mutable classical variable.
    pub fn store(
        &mut self,
        target: ClassicalVar,
        value: ClassicalExpr,
    ) -> Result<(), CircuitError> {
        self.validate_classical_var(target)?;
        self.validate_classical_expr(&value)?;
        self.append(
            Instruction::ClassicalData(ClassicalDataOp::Store { target, value }),
            std::iter::empty::<Qubit>(),
            std::iter::empty(),
            None,
        )
    }

    /// Measures one qubit and produces a self-contained measurement handle.
    ///
    /// The returned [`Measurement`] contains the immutable [`ClassicalValue`]
    /// for this measurement and the measured qubit. The value may be read by
    /// later classical expressions or structured control flow through
    /// [`Measurement::expr`].
    /// This method does not write mutable classical storage; use
    /// [`Circuit::measure_into`] when the result must also be stored in a
    /// [`ClassicalVar`].
    ///
    /// If appending the measurement fails, the allocated classical value is
    /// rolled back and the circuit is left unchanged.
    pub fn measure(&mut self, qubit: Qubit) -> Result<Measurement, CircuitError> {
        let checkpoint = self.checkpoint();
        let result = self.allocate_classical_value(ClassicalType::Bit);
        if let Err(error) = self.append(
            Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }),
            [qubit],
            std::iter::empty(),
            None,
        ) {
            self.rollback_to(checkpoint);
            return Err(error);
        }
        Ok(Measurement::new(result, smallvec::smallvec![qubit]))
    }

    /// Measures qubits and produces a self-contained `BitVec` measurement handle.
    ///
    /// The result width equals the number of qubits. Input order defines bit
    /// order: the first qubit maps to bit index `0`, the least-significant bit.
    /// The returned [`Measurement`] keeps that order and contains the immutable
    /// [`ClassicalValue`] that may be read by later classical expressions or
    /// structured control flow.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::InvalidOperation`] when `qubits` is empty. Any
    /// error while appending the operation rolls back the allocated classical
    /// value and leaves the circuit unchanged.
    pub fn measure_bits<I>(&mut self, qubits: I) -> Result<Measurement, CircuitError>
    where
        I: IntoIterator<Item = Qubit>,
    {
        let qubits: smallvec::SmallVec<[Qubit; 3]> = qubits.into_iter().collect();
        let ty = ClassicalType::bit_vec(qubits.len() as u32).ok_or_else(|| {
            CircuitError::InvalidOperation(
                "multi-qubit measurement requires at least one qubit".to_string(),
            )
        })?;
        let checkpoint = self.checkpoint();
        let result = self.allocate_classical_value(ty);
        if let Err(error) = self.append(
            Instruction::ClassicalData(ClassicalDataOp::MeasureBits { result }),
            qubits.iter().copied(),
            std::iter::empty(),
            None,
        ) {
            self.rollback_to(checkpoint);
            return Err(error);
        }
        Ok(Measurement::new(result, qubits))
    }

    /// Measures one qubit and stores the result into `target`.
    ///
    /// This is a convenience operation equivalent to calling
    /// [`Circuit::measure`] followed by [`Circuit::store`]. It returns the
    /// self-contained measurement handle in addition to updating `target`, so
    /// later expressions may read either the original value or the mutable
    /// variable.
    ///
    /// `target` must be a [`ClassicalType::Bit`] variable owned by this circuit.
    /// The measurement and store are appended atomically: if either operation
    /// fails, both operations and the allocated measurement value are rolled
    /// back.
    pub fn measure_into(
        &mut self,
        qubit: Qubit,
        target: ClassicalVar,
    ) -> Result<Measurement, CircuitError> {
        self.validate_classical_var(target)?;
        let checkpoint = self.checkpoint();
        let result = self.measure(qubit)?;
        if let Err(error) = self.store(target, result.expr()) {
            self.rollback_to(checkpoint);
            return Err(error);
        }
        Ok(result)
    }

    /// Measures qubits and stores the resulting `BitVec` into `target`.
    ///
    /// This is a convenience operation equivalent to calling
    /// [`Circuit::measure_bits`] followed by [`Circuit::store`]. The returned
    /// [`Measurement`] contains the immutable measurement result; `target` is
    /// the mutable copy written by the generated store operation.
    ///
    /// `target` must be a [`ClassicalType::BitVec`] variable owned by this
    /// circuit, and its width must equal the number of measured qubits. Input
    /// order defines bit order: the first qubit maps to bit index `0`, the
    /// least-significant bit. The measurement and store are appended atomically
    /// and are both rolled back if either operation fails.
    pub fn measure_bits_into<I>(
        &mut self,
        qubits: I,
        target: ClassicalVar,
    ) -> Result<Measurement, CircuitError>
    where
        I: IntoIterator<Item = Qubit>,
    {
        self.validate_classical_var(target)?;
        let qubits: smallvec::SmallVec<[Qubit; 3]> = qubits.into_iter().collect();

        let checkpoint = self.checkpoint();
        let result = self.measure_bits(qubits)?;
        if let Err(error) = self.store(target, result.expr()) {
            self.rollback_to(checkpoint);
            return Err(error);
        }
        Ok(result)
    }

    /// Appends a structured `if` operation controlled by a boolean classical expression.
    pub fn if_<F>(&mut self, condition: ClassicalExpr, then_body: F) -> Result<(), CircuitError>
    where
        F: FnOnce(&mut Circuit) -> Result<(), CircuitError>,
    {
        self.validate_classical_expr(&condition)?;
        let checkpoint = self.checkpoint();

        let result = (|| {
            let then_body = self.build_control_body(None, then_body)?;
            let op = IfOp::new(condition, then_body, None)?;
            self.append_control(ClassicalControlOp::If(op))
        })();

        if result.is_err() {
            self.rollback_to(checkpoint);
        }
        result
    }

    /// Appends a structured `if`/`else` operation controlled by a boolean classical expression.
    pub fn if_else<T, E>(
        &mut self,
        condition: ClassicalExpr,
        then_body: T,
        else_body: E,
    ) -> Result<(), CircuitError>
    where
        T: FnOnce(&mut Circuit) -> Result<(), CircuitError>,
        E: FnOnce(&mut Circuit) -> Result<(), CircuitError>,
    {
        self.validate_classical_expr(&condition)?;
        let checkpoint = self.checkpoint();

        let result = (|| {
            let then_body = self.build_control_body(None, then_body)?;
            let else_body = self.build_control_body(None, else_body)?;
            let op = IfOp::new(condition, then_body, Some(else_body))?;
            self.append_control(ClassicalControlOp::If(op))
        })();

        if result.is_err() {
            self.rollback_to(checkpoint);
        }
        result
    }

    /// Appends a structured `while` loop controlled by a boolean classical expression.
    pub fn while_<F>(&mut self, condition: ClassicalExpr, body: F) -> Result<(), CircuitError>
    where
        F: FnOnce(&mut Circuit) -> Result<(), CircuitError>,
    {
        self.validate_classical_expr(&condition)?;
        let checkpoint = self.checkpoint();

        let result = (|| {
            let body = self.build_control_body(Some(ControlScopeKind::Loop), body)?;
            let op = WhileOp::new(condition, body)?;
            self.append_control(ClassicalControlOp::While(op))
        })();

        if result.is_err() {
            self.rollback_to(checkpoint);
        }
        result
    }

    /// Appends an unsigned runtime range loop with half-open `[start, stop)` semantics.
    pub fn for_uint<F>(
        &mut self,
        var: ClassicalVar,
        start: ClassicalExpr,
        stop: ClassicalExpr,
        step: ClassicalExpr,
        body: F,
    ) -> Result<(), CircuitError>
    where
        F: FnOnce(&mut Circuit, ClassicalExpr) -> Result<(), CircuitError>,
    {
        self.validate_classical_var(var)?;
        self.validate_classical_expr(&start)?;
        self.validate_classical_expr(&stop)?;
        self.validate_classical_expr(&step)?;
        let checkpoint = self.checkpoint();
        let loop_expr = var.expr();

        let result = (|| {
            let body = self
                .build_control_body(Some(ControlScopeKind::Loop), |scope| body(scope, loop_expr))?;
            let op = ForOp::new(var, start, stop, step, body)?;
            self.append_control(ClassicalControlOp::For(op))
        })();

        if result.is_err() {
            self.rollback_to(checkpoint);
        }
        result
    }

    /// Appends a structured exact-value `switch` operation over a `UInt` expression.
    pub fn switch<F>(&mut self, target: ClassicalExpr, build: F) -> Result<(), CircuitError>
    where
        F: FnOnce(&mut SwitchBuilder<'_>) -> Result<(), CircuitError>,
    {
        self.validate_classical_expr(&target)?;
        let checkpoint = self.checkpoint();

        let result = (|| {
            let mut builder = SwitchBuilder {
                circuit: self,
                cases: Vec::new(),
                default: None,
            };
            build(&mut builder)?;
            let op = SwitchOp::new(target, builder.cases, builder.default)?;
            builder
                .circuit
                .append_control(ClassicalControlOp::Switch(op))
        })();

        if result.is_err() {
            self.rollback_to(checkpoint);
        }
        result
    }

    /// Appends a raw structured classical-control operation.
    pub fn append_control(&mut self, op: ClassicalControlOp) -> Result<(), CircuitError> {
        let qubits: Vec<Qubit> = op.used_qubits().into_iter().collect();
        self.append(
            Instruction::ClassicalControl(op),
            qubits,
            std::iter::empty(),
            None,
        )
    }

    /// Appends a `break` to the nearest enclosing loop or switch body.
    pub fn break_loop(&mut self) -> Result<(), CircuitError> {
        self.append_control(ClassicalControlOp::Break)
    }

    /// Appends a `continue` to the nearest enclosing loop body.
    pub fn continue_loop(&mut self) -> Result<(), CircuitError> {
        self.append_control(ClassicalControlOp::Continue)
    }
}
