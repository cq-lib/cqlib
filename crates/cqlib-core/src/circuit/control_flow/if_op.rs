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

use super::ControlBody;
use crate::circuit::classical_expr::ClassicalExpr;
use crate::circuit::{CircuitError, ClassicalType, ClassicalValue, ClassicalVar, Qubit};
use std::collections::BTreeSet;

/// Conditional execution controlled by a boolean classical expression.
#[derive(Debug, Clone, PartialEq)]
pub struct IfOp {
    condition: ClassicalExpr,
    then_body: ControlBody,
    else_body: Option<ControlBody>,
}

impl IfOp {
    /// Creates an expression-based `if` operation.
    pub fn new(
        condition: ClassicalExpr,
        then_body: ControlBody,
        else_body: Option<ControlBody>,
    ) -> Result<Self, CircuitError> {
        if condition.ty() != ClassicalType::Bool {
            return Err(CircuitError::InvalidOperation(format!(
                "if condition must be Bool, got {:?}",
                condition.ty()
            )));
        }
        Ok(Self {
            condition,
            then_body,
            else_body,
        })
    }

    /// Returns the boolean branch condition.
    pub fn condition(&self) -> &ClassicalExpr {
        &self.condition
    }

    /// Returns the body executed when the condition is true.
    pub fn then_body(&self) -> &ControlBody {
        &self.then_body
    }

    /// Returns the optional body executed when the condition is false.
    pub fn else_body(&self) -> Option<&ControlBody> {
        self.else_body.as_ref()
    }

    /// Returns mutable variables read by the condition.
    pub fn classical_var_reads(&self) -> BTreeSet<ClassicalVar> {
        self.condition.vars()
    }

    /// Returns immutable values read by the condition.
    pub fn classical_value_reads(&self) -> BTreeSet<ClassicalValue> {
        self.condition.values()
    }

    /// Returns qubits used by either branch body.
    pub fn used_qubits(&self) -> BTreeSet<Qubit> {
        let mut qubits = self.then_body.used_qubits();
        if let Some(else_body) = &self.else_body {
            qubits.extend(else_body.used_qubits());
        }
        qubits
    }
}

#[cfg(test)]
mod tests {
    use super::IfOp;
    use crate::circuit::{
        CircuitId, ClassicalExpr, ClassicalType, ClassicalValue, ClassicalVar, ControlBody,
    };

    #[test]
    fn if_requires_bool_condition() {
        let condition = ClassicalExpr::bool_literal(true);
        let op = IfOp::new(condition, ControlBody::new(vec![]), None).unwrap();

        assert!(op.else_body().is_none());
        assert!(
            IfOp::new(
                ClassicalExpr::bit_literal(true),
                ControlBody::new(vec![]),
                None
            )
            .is_err()
        );
    }

    #[test]
    fn if_reports_condition_var_reads() {
        let circuit_id = CircuitId::new();
        let bit = ClassicalExpr::var(ClassicalVar::new(circuit_id, 3, ClassicalType::Bit));
        let condition = ClassicalExpr::bit_to_bool(bit).unwrap();
        let op = IfOp::new(condition, ControlBody::new(vec![]), None).unwrap();

        assert!(op.classical_var_reads().contains(&ClassicalVar::new(
            circuit_id,
            3,
            ClassicalType::Bit
        )));
    }

    #[test]
    fn if_reports_condition_value_reads() {
        let value = ClassicalValue::new(CircuitId::new(), 4, ClassicalType::Bit);
        let condition = ClassicalExpr::bit_to_bool(value.expr()).unwrap();
        let op = IfOp::new(condition, ControlBody::new(vec![]), None).unwrap();

        assert!(op.classical_var_reads().is_empty());
        assert!(op.classical_value_reads().contains(&value));
    }
}
