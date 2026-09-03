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

/// Loop controlled by a boolean classical expression.
#[derive(Debug, Clone, PartialEq)]
pub struct WhileOp {
    condition: ClassicalExpr,
    body: ControlBody,
}

impl WhileOp {
    /// Creates an expression-based `while` operation.
    pub fn new(condition: ClassicalExpr, body: ControlBody) -> Result<Self, CircuitError> {
        if condition.ty() != ClassicalType::Bool {
            return Err(CircuitError::InvalidOperation(format!(
                "while condition must be Bool, got {:?}",
                condition.ty()
            )));
        }
        Ok(Self { condition, body })
    }

    /// Returns the boolean loop condition.
    pub fn condition(&self) -> &ClassicalExpr {
        &self.condition
    }

    /// Returns the loop body.
    pub fn body(&self) -> &ControlBody {
        &self.body
    }

    /// Returns mutable variables read by the condition.
    pub fn classical_var_reads(&self) -> BTreeSet<ClassicalVar> {
        self.condition.vars()
    }

    /// Returns immutable values read by the condition.
    pub fn classical_value_reads(&self) -> BTreeSet<ClassicalValue> {
        self.condition.values()
    }

    /// Returns qubits used by the loop body.
    pub fn used_qubits(&self) -> BTreeSet<Qubit> {
        self.body.used_qubits()
    }
}

#[cfg(test)]
mod tests {
    use super::WhileOp;
    use crate::circuit::{
        CircuitId, ClassicalExpr, ClassicalType, ClassicalValue, ClassicalVar, ControlBody,
    };

    #[test]
    fn while_requires_bool_condition() {
        assert!(WhileOp::new(ClassicalExpr::bool_literal(true), ControlBody::new(vec![])).is_ok());
        assert!(WhileOp::new(ClassicalExpr::bit_literal(true), ControlBody::new(vec![])).is_err());
    }

    #[test]
    fn while_reports_condition_reads() {
        let circuit_id = CircuitId::new();
        let bit = ClassicalVar::new(circuit_id, 3, ClassicalType::Bit);
        let value = ClassicalValue::new(circuit_id, 4, ClassicalType::Bit);
        let condition = ClassicalExpr::try_and(
            ClassicalExpr::bit_to_bool(bit.expr()).unwrap(),
            ClassicalExpr::bit_to_bool(value.expr()).unwrap(),
        )
        .unwrap();
        let op = WhileOp::new(condition, ControlBody::new(vec![])).unwrap();

        assert!(op.classical_var_reads().contains(&bit));
        assert!(op.classical_value_reads().contains(&value));
    }
}
