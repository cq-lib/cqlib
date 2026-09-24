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

use super::{ControlBody, ForOp, IfOp, SwitchCase, SwitchOp, WhileOp};
use crate::circuit::{ClassicalValue, ClassicalVar, Qubit};
use std::collections::BTreeSet;
use std::fmt;

/// Stable category of a classical control-flow operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClassicalControlKind {
    If,
    While,
    For,
    Switch,
    Break,
    Continue,
}

impl ClassicalControlKind {
    /// Returns the stable snake-case operation name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::If => "if",
            Self::While => "while",
            Self::For => "for",
            Self::Switch => "switch",
            Self::Break => "break",
            Self::Continue => "continue",
        }
    }
}

/// Expression-based classical control-flow operation.
#[derive(Debug, Clone, PartialEq)]
pub enum ClassicalControlOp {
    /// Conditional branch.
    If(IfOp),
    /// Condition-controlled loop.
    While(WhileOp),
    /// Unsigned range loop.
    For(ForOp),
    /// Exact-value multi-way branch.
    Switch(SwitchOp),
    /// Exit the nearest enclosing loop or switch.
    Break,
    /// Advance the nearest enclosing loop.
    Continue,
}

impl ClassicalControlOp {
    /// Borrows direct child bodies without allocating or recursing.
    ///
    /// Visits then/else for `if`, the single body for loops, and cases in source
    /// order followed by default for `switch`. Absent bodies are skipped, but
    /// present empty bodies are retained. Break and continue have no bodies.
    /// Conditions and iteration counts do not affect this structural order.
    pub(crate) fn bodies(&self) -> impl Iterator<Item = &ControlBody> + '_ {
        let (first, cases, last): (Option<&ControlBody>, &[SwitchCase], Option<&ControlBody>) =
            match self {
                Self::If(op) => (Some(op.then_body()), &[], op.else_body()),
                Self::While(op) => (Some(op.body()), &[], None),
                Self::For(op) => (Some(op.body()), &[], None),
                Self::Switch(op) => (None, op.cases(), op.default()),
                Self::Break | Self::Continue => (None, &[], None),
            };
        first
            .into_iter()
            .chain(cases.iter().map(SwitchCase::body))
            .chain(last)
    }

    /// Returns the stable category of this control-flow operation.
    pub const fn kind(&self) -> ClassicalControlKind {
        match self {
            Self::If(_) => ClassicalControlKind::If,
            Self::While(_) => ClassicalControlKind::While,
            Self::For(_) => ClassicalControlKind::For,
            Self::Switch(_) => ClassicalControlKind::Switch,
            Self::Break => ClassicalControlKind::Break,
            Self::Continue => ClassicalControlKind::Continue,
        }
    }

    /// Returns the stable snake-case operation name.
    pub const fn name(&self) -> &'static str {
        self.kind().name()
    }

    /// Returns classical variables read by the operation's controlling expressions.
    pub fn classical_var_reads(&self) -> BTreeSet<ClassicalVar> {
        match self {
            Self::If(op) => op.classical_var_reads(),
            Self::While(op) => op.classical_var_reads(),
            Self::For(op) => op.classical_var_reads(),
            Self::Switch(op) => op.classical_var_reads(),
            Self::Break | Self::Continue => BTreeSet::new(),
        }
    }

    /// Returns immutable classical values read by the operation's controlling expressions.
    pub fn classical_value_reads(&self) -> BTreeSet<ClassicalValue> {
        match self {
            Self::If(op) => op.classical_value_reads(),
            Self::While(op) => op.classical_value_reads(),
            Self::For(op) => op.classical_value_reads(),
            Self::Switch(op) => op.classical_value_reads(),
            Self::Break | Self::Continue => BTreeSet::new(),
        }
    }

    /// Returns classical variables written directly by this operation.
    pub fn classical_writes(&self) -> BTreeSet<ClassicalVar> {
        match self {
            Self::For(op) => op.classical_writes(),
            Self::If(_) | Self::While(_) | Self::Switch(_) | Self::Break | Self::Continue => {
                BTreeSet::new()
            }
        }
    }

    /// Returns qubits used by the operation's structured bodies.
    pub fn used_qubits(&self) -> BTreeSet<Qubit> {
        match self {
            Self::If(op) => op.used_qubits(),
            Self::While(op) => op.used_qubits(),
            Self::For(op) => op.used_qubits(),
            Self::Switch(op) => op.used_qubits(),
            Self::Break | Self::Continue => BTreeSet::new(),
        }
    }

    /// Returns true when any structured body contains a measurement operation.
    pub fn has_measurement(&self) -> bool {
        self.bodies().any(ControlBody::has_measurement)
    }

    /// Returns true when this operation's controlling expressions or structured
    /// bodies read `value`.
    pub fn reads_value(&self, value: ClassicalValue) -> bool {
        if self.classical_value_reads().contains(&value) {
            return true;
        }

        self.bodies().any(|body| body.reads_value(value))
    }
}

impl fmt::Display for ClassicalControlOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::ClassicalControlOp;
    use crate::circuit::{
        Circuit, CircuitId, ClassicalExpr, ClassicalType, ClassicalValue, ClassicalVar,
        ControlBody, ForOp, IfOp, Instruction, Qubit, SwitchCase, SwitchOp, WhileOp,
    };

    fn assert_bodies(control: &ClassicalControlOp, expected: &[&ControlBody]) {
        let actual = control.bodies().collect::<Vec<_>>();
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(std::ptr::eq(actual, *expected));
        }
    }

    #[test]
    fn bodies_preserve_optional_empty_branches_without_recursing() {
        let mut nested = Circuit::new(1);
        nested
            .while_(ClassicalExpr::bool_literal(false), |body| {
                body.x(Qubit::new(0))
            })
            .unwrap();
        for else_body in [None, Some(ControlBody::new(vec![]))] {
            let control = ClassicalControlOp::If(
                IfOp::new(
                    ClassicalExpr::bool_literal(false),
                    ControlBody::new(nested.operations().to_vec()),
                    else_body,
                )
                .unwrap(),
            );
            let ClassicalControlOp::If(op) = &control else {
                unreachable!()
            };
            if let Some(else_body) = op.else_body() {
                assert_bodies(&control, &[op.then_body(), else_body]);
            } else {
                assert_bodies(&control, &[op.then_body()]);
            }
            assert!(matches!(
                control.bodies().next().unwrap().operations()[0].instruction,
                Instruction::ClassicalControl(ClassicalControlOp::While(_))
            ));
        }
    }

    #[test]
    fn bodies_preserve_switch_source_order_and_optional_default() {
        let empty = ControlBody::new(vec![]);
        for cases in [
            vec![],
            vec![
                SwitchCase::new(2, empty.clone()),
                SwitchCase::new(0, empty.clone()),
            ],
        ] {
            for default in [None, Some(empty.clone())] {
                let control = ClassicalControlOp::Switch(
                    SwitchOp::new(
                        ClassicalExpr::uint_literal(2, 0).unwrap(),
                        cases.clone(),
                        default,
                    )
                    .unwrap(),
                );
                let ClassicalControlOp::Switch(op) = &control else {
                    unreachable!()
                };
                let mut expected = Vec::new();
                if !op.cases().is_empty() {
                    // The source order is 2, 0, even though the target is 0.
                    expected.push(op.cases()[0].body());
                    expected.push(op.cases()[1].body());
                }
                if let Some(default) = op.default() {
                    expected.push(default);
                }
                assert_bodies(&control, &expected);
            }
        }
    }

    #[test]
    fn bodies_enumerate_loop_structure_independently_of_iteration_count() {
        let var = ClassicalVar::new(CircuitId::new(), 0, ClassicalType::uint(8).unwrap());
        for stop in [0, 3] {
            let control = ClassicalControlOp::For(
                ForOp::new(
                    var,
                    ClassicalExpr::uint_literal(8, 0).unwrap(),
                    ClassicalExpr::uint_literal(8, stop).unwrap(),
                    ClassicalExpr::uint_literal(8, 1).unwrap(),
                    ControlBody::new(vec![]),
                )
                .unwrap(),
            );
            let ClassicalControlOp::For(op) = &control else {
                unreachable!()
            };
            assert_bodies(&control, &[op.body()]);
        }
        let control = ClassicalControlOp::While(
            WhileOp::new(ClassicalExpr::bool_literal(false), ControlBody::new(vec![])).unwrap(),
        );
        let ClassicalControlOp::While(op) = &control else {
            unreachable!()
        };
        assert_bodies(&control, &[op.body()]);
        assert_bodies(&ClassicalControlOp::Break, &[]);
        assert_bodies(&ClassicalControlOp::Continue, &[]);
    }

    #[test]
    fn break_and_continue_have_no_resource_dependencies() {
        assert!(ClassicalControlOp::Break.classical_var_reads().is_empty());
        assert!(ClassicalControlOp::Break.classical_value_reads().is_empty());
        assert!(ClassicalControlOp::Break.classical_writes().is_empty());
        assert!(ClassicalControlOp::Break.used_qubits().is_empty());
        assert!(
            ClassicalControlOp::Continue
                .classical_var_reads()
                .is_empty()
        );
        assert!(
            ClassicalControlOp::Continue
                .classical_value_reads()
                .is_empty()
        );
        assert!(ClassicalControlOp::Continue.classical_writes().is_empty());
        assert!(ClassicalControlOp::Continue.used_qubits().is_empty());
    }

    #[test]
    fn control_op_forwards_classical_var_and_value_reads() {
        let circuit_id = CircuitId::new();
        let bit = ClassicalVar::new(circuit_id, 1, ClassicalType::Bit);
        let value = ClassicalValue::new(circuit_id, 2, ClassicalType::Bit);
        let condition = ClassicalExpr::try_and(
            ClassicalExpr::bit_to_bool(bit.expr()).unwrap(),
            ClassicalExpr::bit_to_bool(value.expr()).unwrap(),
        )
        .unwrap();
        let op = IfOp::new(condition, ControlBody::new(vec![]), None).unwrap();
        let op = ClassicalControlOp::If(op);

        assert!(op.classical_var_reads().contains(&bit));
        assert!(op.classical_value_reads().contains(&value));
    }

    #[test]
    fn display_reports_operation_kind() {
        let condition = ClassicalExpr::bool_literal(true);
        let body = ControlBody::new(vec![]);
        let if_op = IfOp::new(condition.clone(), body.clone(), None).unwrap();
        let while_op = WhileOp::new(condition, body.clone()).unwrap();
        let loop_var = ClassicalVar::new(CircuitId::new(), 0, ClassicalType::uint(8).unwrap());
        let for_op = ForOp::new(
            loop_var,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 2).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            body.clone(),
        )
        .unwrap();
        let switch_op = SwitchOp::new(
            ClassicalExpr::uint_literal(2, 1).unwrap(),
            vec![SwitchCase::new(1, body)],
            None,
        )
        .unwrap();

        assert_eq!(ClassicalControlOp::If(if_op).to_string(), "if");
        assert_eq!(ClassicalControlOp::While(while_op).to_string(), "while");
        assert_eq!(ClassicalControlOp::For(for_op).to_string(), "for");
        assert_eq!(ClassicalControlOp::Switch(switch_op).to_string(), "switch");
        assert_eq!(ClassicalControlOp::Break.to_string(), "break");
        assert_eq!(ClassicalControlOp::Continue.to_string(), "continue");
    }
}
