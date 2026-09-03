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

use crate::circuit::{ClassicalValue, Operation, Qubit};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Structured control-flow body.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlBody {
    operations: Arc<Vec<Operation>>,
}

impl ControlBody {
    /// Creates a body from a sequence of operations.
    pub fn new(operations: Vec<Operation>) -> Self {
        Self {
            operations: Arc::new(operations),
        }
    }

    /// Returns the body operations.
    pub fn operations(&self) -> &[Operation] {
        self.operations.as_slice()
    }

    /// Returns all qubits referenced directly by this body.
    pub fn used_qubits(&self) -> BTreeSet<Qubit> {
        let mut qubits = BTreeSet::new();
        for operation in self.operations() {
            qubits.extend(operation.qubits.iter().copied());
        }
        qubits
    }

    /// Returns true when this body contains a measurement operation.
    pub fn has_measurement(&self) -> bool {
        self.operations()
            .iter()
            .any(|operation| operation.instruction.has_measurement())
    }

    /// Returns true when this body reads `value`.
    pub fn reads_value(&self, value: ClassicalValue) -> bool {
        self.operations()
            .iter()
            .any(|operation| operation.instruction.reads_value(value))
    }
}

impl From<Vec<Operation>> for ControlBody {
    fn from(operations: Vec<Operation>) -> Self {
        Self::new(operations)
    }
}

#[cfg(test)]
mod tests {
    use super::ControlBody;
    use crate::circuit::{
        CircuitId, ClassicalDataOp, ClassicalExpr, ClassicalType, ClassicalValue,
        ControlBody as Body, IfOp, Instruction, Operation, Qubit, StandardGate,
    };
    use smallvec::smallvec;

    #[test]
    fn body_reports_directly_used_qubits() {
        let body = ControlBody::new(vec![
            Operation {
                instruction: Instruction::Standard(StandardGate::CX),
                qubits: smallvec![Qubit::new(0), Qubit::new(2)],
                params: smallvec![],
                label: None,
            },
            Operation {
                instruction: Instruction::Standard(StandardGate::H),
                qubits: smallvec![Qubit::new(1)],
                params: smallvec![],
                label: None,
            },
        ]);

        let qubits = body.used_qubits();
        assert_eq!(qubits.len(), 3);
        assert!(qubits.contains(&Qubit::new(0)));
        assert!(qubits.contains(&Qubit::new(1)));
        assert!(qubits.contains(&Qubit::new(2)));
    }

    #[test]
    fn body_reports_nested_measurements() {
        let value = ClassicalValue::new(CircuitId::new(), 0, ClassicalType::Bit);
        let inner = Body::new(vec![Operation {
            instruction: Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result: value }),
            qubits: smallvec![Qubit::new(0)],
            params: smallvec![],
            label: None,
        }]);
        let op = IfOp::new(ClassicalExpr::bool_literal(true), inner, None).unwrap();
        let body = ControlBody::new(vec![Operation {
            instruction: Instruction::ClassicalControl(super::super::ClassicalControlOp::If(op)),
            qubits: smallvec![],
            params: smallvec![],
            label: None,
        }]);

        assert!(body.has_measurement());
    }

    #[test]
    fn body_reports_nested_value_reads() {
        let circuit_id = CircuitId::new();
        let value = ClassicalValue::new(circuit_id, 0, ClassicalType::Bit);
        let target = crate::circuit::ClassicalVar::new(circuit_id, 1, ClassicalType::Bit);
        let inner = Body::new(vec![Operation {
            instruction: Instruction::ClassicalData(ClassicalDataOp::Store {
                target,
                value: value.expr(),
            }),
            qubits: smallvec![],
            params: smallvec![],
            label: None,
        }]);
        let op = IfOp::new(ClassicalExpr::bool_literal(true), inner, None).unwrap();
        let body = ControlBody::new(vec![Operation {
            instruction: Instruction::ClassicalControl(super::super::ClassicalControlOp::If(op)),
            qubits: smallvec![],
            params: smallvec![],
            label: None,
        }]);

        assert!(body.reads_value(value));
    }
}
