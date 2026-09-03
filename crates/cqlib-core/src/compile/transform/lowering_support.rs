// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Shared output and traversal support for circuit lowering passes.

use crate::circuit::{Operation, Parameter, ValueOperation};
use crate::compile::CompilerError;
use crate::compile::transform::rebuild::ClassicalRemap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoweringScope {
    TopLevel,
    ControlFlowBody,
}

/// Mutable output state for one independently lowered operation sequence.
///
/// The target only owns common emission state. Pass-specific finalization,
/// such as flushing delayed device leaves or materializing a body-local
/// `GPhase`, remains the responsibility of the lowering pass.
pub(crate) struct LoweringTarget<'a> {
    output: &'a mut Vec<ValueOperation>,
    phase_delta: &'a mut Parameter,
    scope: LoweringScope,
}

impl<'a> LoweringTarget<'a> {
    /// Creates a target for a circuit's root operation sequence.
    pub(crate) fn top_level(
        output: &'a mut Vec<ValueOperation>,
        phase_delta: &'a mut Parameter,
    ) -> Self {
        Self {
            output,
            phase_delta,
            scope: LoweringScope::TopLevel,
        }
    }

    /// Creates a target for one structured-control body.
    pub(crate) fn control_flow_body(
        output: &'a mut Vec<ValueOperation>,
        phase_delta: &'a mut Parameter,
    ) -> Self {
        Self {
            output,
            phase_delta,
            scope: LoweringScope::ControlFlowBody,
        }
    }

    /// Returns whether this target emits the circuit's root sequence.
    pub(crate) const fn is_top_level(&self) -> bool {
        matches!(self.scope, LoweringScope::TopLevel)
    }

    /// Appends one value-level operation to this sequence.
    pub(crate) fn push(&mut self, operation: ValueOperation) {
        self.output.push(operation);
    }

    /// Adds an exact global-phase contribution to this sequence.
    pub(crate) fn accumulate_phase(&mut self, phase: Parameter) {
        *self.phase_delta = &*self.phase_delta + &phase;
    }
}

/// Common linear sequence driver for lowering passes whose unit of work is one
/// source operation.
pub(crate) trait OperationSequenceLowerer {
    /// Lowers one operation into `target`.
    fn lower_one_operation(
        &mut self,
        operation: &Operation,
        classical_remap: &ClassicalRemap,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError>;

    /// Lowers one operation sequence in source order.
    fn lower_sequence(
        &mut self,
        operations: &[Operation],
        classical_remap: &ClassicalRemap,
        mut target: LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        for operation in operations {
            self.lower_one_operation(operation, classical_remap, &mut target)?;
        }
        Ok(())
    }
}
