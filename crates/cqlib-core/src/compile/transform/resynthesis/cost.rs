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

//! Local cost model for numeric block resynthesis.

use super::commutation::OperationView;
use crate::circuit::{Instruction, ParameterValue, ValueInstruction, ValueOperation};
use crate::compile::CompilerError;
use crate::compile::transform::decompose::unitary::{
    TargetAwareSynthesisCost, TwoQubitSynthesisTarget, TwoQubitUnitaryDecomposeBasis,
    target_aware_cost_of_value_operations,
};

/// Local cost used to accept only strictly improving resynthesis patches.
pub(crate) type ResynthesisCost = TargetAwareSynthesisCost;

pub(crate) fn cost_of_source_value_operations(
    operations: &[ValueOperation],
    target: &TwoQubitSynthesisTarget,
) -> Result<ResynthesisCost, CompilerError> {
    let mut cost = target_aware_cost_of_value_operations(
        operations,
        target,
        TwoQubitUnitaryDecomposeBasis::PauliRotations,
    )?;
    // Backend preference orders equivalent synthesis candidates only. Source
    // operations must not receive a synthetic backend advantage.
    cost.backend_order = 0;
    Ok(cost)
}

pub(crate) fn value_operations_of_source_ops(
    ops: &[&OperationView<'_>],
) -> Result<Vec<ValueOperation>, CompilerError> {
    ops.iter()
        .map(|view| {
            let Instruction::Standard(gate) = view.operation.instruction else {
                return Err(CompilerError::InvariantViolation(
                    "resynthesis cost requested for non-standard operation".to_string(),
                ));
            };
            let params = view
                .params
                .iter()
                .map(|parameter| parameter.evaluate(&None))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| {
                    CompilerError::InvariantViolation(
                        "resynthesis cost requested for symbolic operation".to_string(),
                    )
                })?;
            Ok(ValueOperation {
                instruction: ValueInstruction::from_instruction(Instruction::Standard(gate)),
                qubits: view.operation.qubits.clone(),
                params: params.into_iter().map(ParameterValue::Fixed).collect(),
                label: view.operation.label.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "cost_test.rs"]
mod cost_test;
