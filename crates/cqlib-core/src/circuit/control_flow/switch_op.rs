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
use crate::circuit::{CircuitError, ClassicalValue, ClassicalVar, Qubit};
use std::collections::BTreeSet;

/// Exact-value switch case.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    value: u128,
    body: ControlBody,
}

impl SwitchCase {
    /// Creates an exact-value case with its body.
    pub fn new(value: u128, body: ControlBody) -> Self {
        Self { value, body }
    }

    /// Returns the exact value matched by this case.
    pub fn value(&self) -> u128 {
        self.value
    }

    /// Returns the body executed when this case matches.
    pub fn body(&self) -> &ControlBody {
        &self.body
    }
}

/// Exact-value switch over an unsigned classical expression.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchOp {
    target: ClassicalExpr,
    cases: Box<[SwitchCase]>,
    default: Option<ControlBody>,
}

impl SwitchOp {
    /// Creates a switch without fallthrough.
    pub fn new(
        target: ClassicalExpr,
        cases: Vec<SwitchCase>,
        default: Option<ControlBody>,
    ) -> Result<Self, CircuitError> {
        super::validate_switch(&target, cases.iter().map(|case| case.value))?;

        Ok(Self {
            target,
            cases: cases.into_boxed_slice(),
            default,
        })
    }

    /// Returns the unsigned expression being matched.
    pub fn target(&self) -> &ClassicalExpr {
        &self.target
    }

    /// Returns exact-value cases in source order.
    pub fn cases(&self) -> &[SwitchCase] {
        &self.cases
    }

    /// Returns the optional default body.
    pub fn default(&self) -> Option<&ControlBody> {
        self.default.as_ref()
    }

    /// Returns mutable variables read by the target expression.
    pub fn classical_var_reads(&self) -> BTreeSet<ClassicalVar> {
        self.target.vars()
    }

    /// Returns immutable values read by the target expression.
    pub fn classical_value_reads(&self) -> BTreeSet<ClassicalValue> {
        self.target.values()
    }

    /// Returns qubits used by all case and default bodies.
    pub fn used_qubits(&self) -> BTreeSet<Qubit> {
        let mut qubits = BTreeSet::new();
        for case in self.cases.iter() {
            qubits.extend(case.body.used_qubits());
        }
        if let Some(default) = &self.default {
            qubits.extend(default.used_qubits());
        }
        qubits
    }
}

#[cfg(test)]
mod tests {
    use super::{SwitchCase, SwitchOp};
    use crate::circuit::{CircuitId, ClassicalExpr, ClassicalType, ClassicalValue, ControlBody};

    #[test]
    fn switch_requires_uint_target_and_valid_unique_cases() {
        assert!(
            SwitchOp::new(
                ClassicalExpr::uint_literal(3, 0).unwrap(),
                vec![
                    SwitchCase::new(1, ControlBody::new(vec![])),
                    SwitchCase::new(7, ControlBody::new(vec![])),
                ],
                None,
            )
            .is_ok()
        );

        assert!(
            SwitchOp::new(ClassicalExpr::bit_vec_literal(3, 0).unwrap(), vec![], None,).is_err()
        );
        assert!(
            SwitchOp::new(
                ClassicalExpr::uint_literal(3, 0).unwrap(),
                vec![SwitchCase::new(8, ControlBody::new(vec![]))],
                None,
            )
            .is_err()
        );
        assert!(
            SwitchOp::new(
                ClassicalExpr::uint_literal(3, 0).unwrap(),
                vec![
                    SwitchCase::new(1, ControlBody::new(vec![])),
                    SwitchCase::new(1, ControlBody::new(vec![])),
                ],
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn switch_reports_target_value_reads() {
        let value = ClassicalValue::new(CircuitId::new(), 5, ClassicalType::uint(3).unwrap());
        let op = SwitchOp::new(value.expr(), vec![], None).unwrap();

        assert!(op.classical_var_reads().is_empty());
        assert!(op.classical_value_reads().contains(&value));
    }
}
