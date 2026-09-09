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

use crate::circuit::gate::{ClassicalDataOp, Directive};
use crate::circuit::{Circuit, Instruction};
use crate::qis::QisError;
use std::collections::HashMap;

// Validate a decomposed circuit before state evolution starts. State-level APIs
// treat measurements as output declarations, so a measured qubit cannot be used
// by a later quantum operation. Independent qubits may still evolve.
pub(super) fn validate_terminal_measurements(circuit: &Circuit) -> Result<(), QisError> {
    let mut measured = HashMap::new();
    for (index, op) in circuit.operations().iter().enumerate() {
        match &op.instruction {
            Instruction::ClassicalData(
                ClassicalDataOp::MeasureBit { .. } | ClassicalDataOp::MeasureBits { .. },
            ) => {
                for qubit in &op.qubits {
                    measured.entry(*qubit).or_insert(index);
                }
            }
            Instruction::ClassicalData(ClassicalDataOp::Store { .. })
            | Instruction::Directive(Directive::Barrier)
            | Instruction::Delay => {}
            _ => {
                for qubit in &op.qubits {
                    if let Some(measurement_index) = measured.get(qubit) {
                        return Err(QisError::UnsupportedOperation(format!(
                            "mid-circuit measurement on qubit {} at operation {}: later '{}' at \
                             operation {} uses the measured qubit (indices refer to the decomposed \
                             circuit, starting at 0); from_circuit/apply_circuit only support \
                             terminal measurement declarations",
                            qubit.id(),
                            measurement_index,
                            op.instruction.name(),
                            index,
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}
