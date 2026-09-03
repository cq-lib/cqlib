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
use crate::circuit::classical_expr::{ClassicalExpr, ClassicalExprKind, simplify};
use crate::circuit::{CircuitError, ClassicalValue, ClassicalVar, Qubit};
use std::collections::BTreeSet;

/// Unsigned runtime range loop with half-open `[start, stop)` semantics.
#[derive(Debug, Clone, PartialEq)]
pub struct ForOp {
    var: ClassicalVar,
    start: ClassicalExpr,
    stop: ClassicalExpr,
    step: ClassicalExpr,
    body: ControlBody,
}

impl ForOp {
    /// Creates an unsigned runtime range loop.
    pub fn new(
        var: ClassicalVar,
        start: ClassicalExpr,
        stop: ClassicalExpr,
        step: ClassicalExpr,
        body: ControlBody,
    ) -> Result<Self, CircuitError> {
        super::validate_for_types(var, &start, &stop, &step)?;

        Ok(Self {
            var,
            start,
            stop,
            step,
            body,
        })
    }

    /// Returns the mutable unsigned loop variable.
    pub fn var(&self) -> ClassicalVar {
        self.var
    }

    /// Returns the inclusive initial value.
    pub fn start(&self) -> &ClassicalExpr {
        &self.start
    }

    /// Returns the exclusive upper bound.
    pub fn stop(&self) -> &ClassicalExpr {
        &self.stop
    }

    /// Returns the iteration increment.
    pub fn step(&self) -> &ClassicalExpr {
        &self.step
    }

    /// Returns the loop body.
    pub fn body(&self) -> &ControlBody {
        &self.body
    }

    /// Returns the exact iteration count when every range expression reduces
    /// to an unsigned literal. Runtime-dependent ranges and zero steps are
    /// intentionally reported as unknown.
    pub(crate) fn static_iteration_count(&self) -> Option<u128> {
        let literal = |expr: &ClassicalExpr| match simplify(expr).kind() {
            ClassicalExprKind::UIntLiteral { value, .. } => Some(*value),
            _ => None,
        };
        let start = literal(&self.start)?;
        let stop = literal(&self.stop)?;
        let step = literal(&self.step)?;
        if step == 0 {
            return None;
        }
        Some(if start >= stop {
            0
        } else {
            (stop - start).div_ceil(step)
        })
    }

    /// Returns mutable variables read by the range expressions.
    pub fn classical_var_reads(&self) -> BTreeSet<ClassicalVar> {
        let mut vars = self.start.vars();
        vars.extend(self.stop.vars());
        vars.extend(self.step.vars());
        vars
    }

    /// Returns immutable values read by the range expressions.
    pub fn classical_value_reads(&self) -> BTreeSet<ClassicalValue> {
        let mut values = self.start.values();
        values.extend(self.stop.values());
        values.extend(self.step.values());
        values
    }

    /// Returns the loop variable written by the operation.
    pub fn classical_writes(&self) -> BTreeSet<ClassicalVar> {
        let mut vars = BTreeSet::new();
        vars.insert(self.var);
        vars
    }

    /// Returns qubits used by the loop body.
    pub fn used_qubits(&self) -> BTreeSet<Qubit> {
        self.body.used_qubits()
    }
}

#[cfg(test)]
mod tests {
    use super::ForOp;
    use crate::circuit::{
        CircuitId, ClassicalExpr, ClassicalType, ClassicalValue, ClassicalVar, ControlBody,
    };
    use std::sync::OnceLock;

    fn test_circuit_id() -> CircuitId {
        static ID: OnceLock<CircuitId> = OnceLock::new();
        *ID.get_or_init(CircuitId::new)
    }

    #[test]
    fn for_requires_matching_uint_types() {
        let var = ClassicalVar::new(test_circuit_id(), 0, ClassicalType::uint(8).unwrap());
        assert!(
            ForOp::new(
                var,
                ClassicalExpr::uint_literal(8, 0).unwrap(),
                ClassicalExpr::uint_literal(8, 10).unwrap(),
                ClassicalExpr::uint_literal(8, 1).unwrap(),
                ControlBody::new(vec![]),
            )
            .is_ok()
        );

        assert!(
            ForOp::new(
                ClassicalVar::new(test_circuit_id(), 1, ClassicalType::Bit),
                ClassicalExpr::uint_literal(8, 0).unwrap(),
                ClassicalExpr::uint_literal(8, 10).unwrap(),
                ClassicalExpr::uint_literal(8, 1).unwrap(),
                ControlBody::new(vec![]),
            )
            .is_err()
        );

        assert!(
            ForOp::new(
                var,
                ClassicalExpr::uint_literal(4, 0).unwrap(),
                ClassicalExpr::uint_literal(8, 10).unwrap(),
                ClassicalExpr::uint_literal(8, 1).unwrap(),
                ControlBody::new(vec![]),
            )
            .is_err()
        );
    }

    #[test]
    fn static_iteration_count_accepts_simplifiable_uint_constants() {
        let var = ClassicalVar::new(test_circuit_id(), 0, ClassicalType::uint(8).unwrap());
        let bitvec_uint = |value| {
            ClassicalExpr::bit_vec_literal(8, value)
                .unwrap()
                .to_uint()
                .unwrap()
        };
        let op = ForOp::new(
            var,
            bitvec_uint(1),
            bitvec_uint(8),
            bitvec_uint(2),
            ControlBody::new(vec![]),
        )
        .unwrap();

        assert_eq!(op.static_iteration_count(), Some(4));
    }

    #[test]
    fn for_reports_range_reads_and_loop_variable_write() {
        let var = ClassicalVar::new(test_circuit_id(), 0, ClassicalType::uint(8).unwrap());
        let start_var = ClassicalVar::new(test_circuit_id(), 1, ClassicalType::uint(8).unwrap());
        let stop_var = ClassicalVar::new(test_circuit_id(), 2, ClassicalType::uint(8).unwrap());
        let step_var = ClassicalVar::new(test_circuit_id(), 3, ClassicalType::uint(8).unwrap());
        let op = ForOp::new(
            var,
            ClassicalExpr::var(start_var),
            ClassicalExpr::var(stop_var),
            ClassicalExpr::var(step_var),
            ControlBody::new(vec![]),
        )
        .unwrap();

        let reads = op.classical_var_reads();
        assert_eq!(reads.len(), 3);
        assert!(reads.contains(&start_var));
        assert!(reads.contains(&stop_var));
        assert!(reads.contains(&step_var));
        assert_eq!(
            op.classical_writes().into_iter().collect::<Vec<_>>(),
            vec![var]
        );
    }

    #[test]
    fn for_reports_range_value_reads() {
        let ty = ClassicalType::uint(8).unwrap();
        let var = ClassicalVar::new(test_circuit_id(), 0, ty);
        let start = ClassicalValue::new(test_circuit_id(), 1, ty);
        let stop = ClassicalValue::new(test_circuit_id(), 2, ty);
        let step = ClassicalValue::new(test_circuit_id(), 3, ty);
        let op = ForOp::new(
            var,
            start.expr(),
            stop.expr(),
            step.expr(),
            ControlBody::new(vec![]),
        )
        .unwrap();

        let reads = op.classical_value_reads();
        assert_eq!(reads.len(), 3);
        assert!(reads.contains(&start));
        assert!(reads.contains(&stop));
        assert!(reads.contains(&step));
        assert!(op.classical_var_reads().is_empty());
    }
}
