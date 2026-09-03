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

//! Validation of circuit classical-data and structured control-flow invariants.
//!
//! The verifier checks that classical handles belong to the circuit, immutable
//! values have one dominating definition, expressions only read available
//! values, branch-local values do not escape their region, and `break` and
//! `continue` appear only in valid terminal positions. Quantum gate arity and
//! qubit membership are validated when operations are appended, not here.

use crate::circuit::gate::{ClassicalDataOp, Instruction};
use crate::circuit::{
    Circuit, CircuitError, ClassicalControlOp, ClassicalExpr, ClassicalType, ClassicalValue,
    ControlBody, Operation,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Default)]
struct ControlContext {
    break_allowed: bool,
    continue_allowed: bool,
}

impl ControlContext {
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

struct ClassicalVerifier<'a> {
    circuit: &'a Circuit,
    definitions: HashMap<ClassicalValue, String>,
}

impl Circuit {
    /// Validates classical handle ownership, measurement definitions, SSA use
    /// availability, and structured control-flow value scope.
    ///
    /// Validation does not mutate the circuit and may be called after loading,
    /// transforming, or manually constructing circuit IR.
    ///
    /// # Errors
    ///
    /// Returns a [`CircuitError`] describing the first ownership, definition,
    /// type, scope, or control-flow placement violation encountered.
    pub fn validate(&self) -> Result<(), CircuitError> {
        self.validate_with_context(ControlContext::default(), true)
    }

    pub(super) fn validate_builder_state(&self) -> Result<(), CircuitError> {
        let context = ControlContext {
            break_allowed: self.control_scope_stack.iter().rev().any(|scope| {
                matches!(
                    scope,
                    crate::circuit::circuit_classical::ControlScopeKind::Loop
                        | crate::circuit::circuit_classical::ControlScopeKind::Switch
                )
            }),
            continue_allowed: self.control_scope_stack.iter().rev().any(|scope| {
                matches!(
                    scope,
                    crate::circuit::circuit_classical::ControlScopeKind::Loop
                )
            }),
        };
        self.validate_with_context(context, false)
    }

    fn validate_with_context(
        &self,
        context: ControlContext,
        require_all_values_defined: bool,
    ) -> Result<(), CircuitError> {
        let mut verifier = ClassicalVerifier {
            circuit: self,
            definitions: HashMap::new(),
        };
        verifier.verify_operations(self.operations(), HashSet::new(), context, "circuit")?;

        if require_all_values_defined {
            for (index, ty) in self.classical_values.iter().copied().enumerate() {
                let value = ClassicalValue::new(self.circuit_id, index as u32, ty);
                if !verifier.definitions.contains_key(&value) {
                    return Err(CircuitError::UndefinedClassicalValue {
                        index: index as u32,
                        context: "classical value table".to_string(),
                    });
                }
            }
        }
        Ok(())
    }
}

impl ClassicalVerifier<'_> {
    fn verify_operations(
        &mut self,
        operations: &[Operation],
        mut available: HashSet<ClassicalValue>,
        context: ControlContext,
        context_name: &str,
    ) -> Result<HashSet<ClassicalValue>, CircuitError> {
        for (index, operation) in operations.iter().enumerate() {
            match &operation.instruction {
                Instruction::ClassicalData(op) => {
                    let operation_context = format!("{context_name} operation {index}");
                    self.verify_data_op(op, operation, &mut available, &operation_context)?;
                }
                Instruction::ClassicalControl(op) => {
                    let operation_context = format!("{context_name} operation {index}");
                    self.verify_control_op(op, &available, context, &operation_context)?;
                    match op {
                        ClassicalControlOp::Break => {
                            if !context.break_allowed {
                                return Err(CircuitError::InvalidOperation(
                                    "break can only be used inside a loop or switch body"
                                        .to_string(),
                                ));
                            }
                            if index + 1 != operations.len() {
                                return Err(CircuitError::NonTerminalControlTransfer {
                                    operation: "break",
                                    context: context_name.to_string(),
                                });
                            }
                        }
                        ClassicalControlOp::Continue => {
                            if !context.continue_allowed {
                                return Err(CircuitError::InvalidOperation(
                                    "continue can only be used inside a loop body".to_string(),
                                ));
                            }
                            if index + 1 != operations.len() {
                                return Err(CircuitError::NonTerminalControlTransfer {
                                    operation: "continue",
                                    context: context_name.to_string(),
                                });
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Ok(available)
    }

    fn verify_data_op(
        &mut self,
        op: &ClassicalDataOp,
        operation: &Operation,
        available: &mut HashSet<ClassicalValue>,
        context: &str,
    ) -> Result<(), CircuitError> {
        match op {
            ClassicalDataOp::Store { target, value } => {
                self.circuit.validate_classical_var(*target)?;
                self.verify_expr(value, available, context)?;
                if target.ty() != value.ty() {
                    return Err(CircuitError::InvalidOperation(format!(
                        "store target type {:?} does not match value type {:?}",
                        target.ty(),
                        value.ty()
                    )));
                }
            }
            ClassicalDataOp::MeasureBit { result } => {
                self.circuit.validate_classical_value(*result)?;
                if result.ty() != ClassicalType::Bit || operation.qubits.len() != 1 {
                    return Err(CircuitError::InvalidOperation(format!(
                        "measure_bit result {} must be Bit with one qubit",
                        result.index()
                    )));
                }
                self.define(*result, available, context)?;
            }
            ClassicalDataOp::MeasureBits { result } => {
                self.circuit.validate_classical_value(*result)?;
                if result.ty().measurement_width() != Some(operation.qubits.len() as u32) {
                    return Err(CircuitError::InvalidOperation(format!(
                        "measure_bits result {} type {:?} does not match {} qubits",
                        result.index(),
                        result.ty(),
                        operation.qubits.len()
                    )));
                }
                self.define(*result, available, context)?;
            }
        }
        Ok(())
    }

    fn define(
        &mut self,
        value: ClassicalValue,
        available: &mut HashSet<ClassicalValue>,
        context: &str,
    ) -> Result<(), CircuitError> {
        if let Some(first) = self.definitions.insert(value, context.to_string()) {
            return Err(CircuitError::DuplicateClassicalValueDefinition {
                index: value.index(),
                first,
                second: context.to_string(),
            });
        }
        available.insert(value);
        Ok(())
    }

    fn verify_expr(
        &self,
        expr: &ClassicalExpr,
        available: &HashSet<ClassicalValue>,
        context: &str,
    ) -> Result<(), CircuitError> {
        for var in expr.vars() {
            self.circuit.validate_classical_var(var)?;
        }
        for value in expr.values() {
            self.circuit.validate_classical_value(value)?;
            if !available.contains(&value) {
                if self.definitions.contains_key(&value) {
                    return Err(CircuitError::ClassicalValueOutOfScope {
                        index: value.index(),
                        context: context.to_string(),
                    });
                }
                return Err(CircuitError::UndefinedClassicalValue {
                    index: value.index(),
                    context: context.to_string(),
                });
            }
        }
        Ok(())
    }

    fn verify_body(
        &mut self,
        body: &ControlBody,
        available: &HashSet<ClassicalValue>,
        context: ControlContext,
        name: &str,
    ) -> Result<(), CircuitError> {
        self.verify_operations(body.operations(), available.clone(), context, name)?;
        Ok(())
    }

    fn verify_control_op(
        &mut self,
        op: &ClassicalControlOp,
        available: &HashSet<ClassicalValue>,
        context: ControlContext,
        operation_context: &str,
    ) -> Result<(), CircuitError> {
        match op {
            ClassicalControlOp::If(op) => {
                self.verify_expr(op.condition(), available, operation_context)?;
                self.verify_body(
                    op.then_body(),
                    available,
                    context,
                    &format!("{operation_context} then body"),
                )?;
                if let Some(body) = op.else_body() {
                    self.verify_body(
                        body,
                        available,
                        context,
                        &format!("{operation_context} else body"),
                    )?;
                }
            }
            ClassicalControlOp::While(op) => {
                self.verify_expr(op.condition(), available, operation_context)?;
                self.verify_body(
                    op.body(),
                    available,
                    context.enter_loop(),
                    &format!("{operation_context} while body"),
                )?;
            }
            ClassicalControlOp::For(op) => {
                self.circuit.validate_classical_var(op.var())?;
                self.verify_expr(op.start(), available, operation_context)?;
                self.verify_expr(op.stop(), available, operation_context)?;
                self.verify_expr(op.step(), available, operation_context)?;
                self.verify_body(
                    op.body(),
                    available,
                    context.enter_loop(),
                    &format!("{operation_context} for body"),
                )?;
            }
            ClassicalControlOp::Switch(op) => {
                self.verify_expr(op.target(), available, operation_context)?;
                let body_context = context.enter_switch();
                for case in op.cases() {
                    self.verify_body(
                        case.body(),
                        available,
                        body_context,
                        &format!("{operation_context} switch case {}", case.value()),
                    )?;
                }
                if let Some(body) = op.default() {
                    self.verify_body(
                        body,
                        available,
                        body_context,
                        &format!("{operation_context} switch default"),
                    )?;
                }
            }
            ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
        }
        Ok(())
    }
}
