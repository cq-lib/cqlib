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

//! Pre-routing legalization for SABRE's gate-arity contract.
//!
//! This pass is intentionally narrower than final target-basis lowering. It
//! only removes gate-like operations that SABRE cannot route because they act
//! on more than two qubits, while preserving existing 2-qubit standard gates
//! that are already routable.

use crate::circuit::{
    Circuit, ClassicalControlOp, Instruction, Operation, StandardGate, ValueOperation,
};
use crate::compile::CompilerError;
use crate::compile::transform::analysis::WorkflowCircuitAnalysis;
use crate::compile::transform::rebuild::CircuitRebuildContext;
use crate::compile::transform::transformer::{PassApplicability, WorkflowPass};
use crate::compile::transform::{
    CircuitAnalysis, KnowledgeRewriter, RewriteConfig, TransformOutcome, Transformer,
};

/// Lowers gate-like operations to SABRE's 0/1/2-qubit input contract.
///
/// `preferred_basis` is a hint used only to choose the 2-qubit family for CCX
/// lowering. When it contains CZ and does not contain CX, CCX is lowered through
/// CZ rules; otherwise CX is preferred. Final native-basis translation remains a
/// separate post-routing workflow stage.
#[derive(Debug, Clone, Default)]
pub struct LowerToRoutingBasis {
    preferred_basis: Option<Vec<Instruction>>,
}

impl LowerToRoutingBasis {
    /// Creates a routing-basis lowering transform.
    ///
    /// The optional `preferred_basis` should come from the explicit compile
    /// target basis or from device native gates. It is not used as an exact
    /// output basis for this transform.
    pub fn new(preferred_basis: Option<Vec<Instruction>>) -> Self {
        Self { preferred_basis }
    }

    fn routing_pre_basis(&self, circuit: &Circuit) -> Result<Vec<Instruction>, CompilerError> {
        let mut basis = Vec::new();

        for gate in StandardGate::all()
            .iter()
            .copied()
            .filter(|gate| gate.num_qubits() <= 1)
        {
            if !basis.contains(&gate) {
                basis.push(gate);
            }
        }

        collect_routable_two_qubit_gates(circuit.operations(), &mut basis);

        let ccx_gate = self.preferred_ccx_two_qubit_gate();
        if !basis.contains(&ccx_gate) {
            basis.push(ccx_gate);
        }

        Ok(basis.into_iter().map(Instruction::Standard).collect())
    }

    fn preferred_ccx_two_qubit_gate(&self) -> StandardGate {
        if let Some(basis) = self.preferred_basis.as_deref() {
            let has_cz = basis
                .iter()
                .any(|instruction| matches!(instruction, Instruction::Standard(StandardGate::CZ)));
            let has_cx = basis
                .iter()
                .any(|instruction| matches!(instruction, Instruction::Standard(StandardGate::CX)));
            if has_cz && !has_cx {
                return StandardGate::CZ;
            }
        }

        StandardGate::CX
    }
}

impl WorkflowPass for LowerToRoutingBasis {
    fn applicability(&self, analysis: &WorkflowCircuitAnalysis) -> PassApplicability {
        if analysis.has_gate_like_operation_over_two_qubits() {
            PassApplicability::Run
        } else {
            PassApplicability::ProvenNoOp(
                "circuit contains no gate-like operation over more than two qubits",
            )
        }
    }
}

impl Transformer for LowerToRoutingBasis {
    fn name(&self) -> &'static str {
        "lower_to_routing_basis"
    }

    fn transform(
        &self,
        circuit: &Circuit,
        analysis: Option<&CircuitAnalysis>,
    ) -> Result<TransformOutcome, CompilerError> {
        if !has_unroutable_gate_like_operation(circuit.operations()) {
            return Ok(TransformOutcome::Unchanged);
        }

        if can_directly_lower_top_level_ccx(circuit.operations()) {
            let circuit =
                directly_lower_top_level_ccx(circuit, self.preferred_ccx_two_qubit_gate())?;
            validate_routing_basis_contract(circuit.operations())?;
            return Ok(TransformOutcome::Changed(circuit));
        }

        let config =
            RewriteConfig::lowering().with_target_instructions(self.routing_pre_basis(circuit)?)?;
        let result = match KnowledgeRewriter::new(config).transform(circuit, analysis) {
            Ok(result) => result,
            Err(err) => {
                validate_routing_basis_contract(circuit.operations())?;
                return Err(err);
            }
        };

        match result {
            TransformOutcome::Unchanged => {
                validate_routing_basis_contract(circuit.operations())?;
                Ok(TransformOutcome::Unchanged)
            }
            TransformOutcome::Changed(lowered) => {
                validate_routing_basis_contract(lowered.operations())?;
                Ok(TransformOutcome::Changed(lowered))
            }
        }
    }
}

fn can_directly_lower_top_level_ccx(operations: &[Operation]) -> bool {
    let mut found = false;
    for operation in operations {
        match &operation.instruction {
            Instruction::Standard(StandardGate::CCX)
                if operation.qubits.len() == 3
                    && operation.params.is_empty()
                    && operation.label.is_none() =>
            {
                found = true;
            }
            Instruction::Standard(_)
            | Instruction::McGate(_)
            | Instruction::UnitaryGate(_)
            | Instruction::CircuitGate(_)
                if operation.qubits.len() > 2 =>
            {
                return false;
            }
            Instruction::ClassicalControl(op) => {
                let mut nested_unroutable = false;
                for_each_control_body(op, |body| {
                    nested_unroutable |= has_unroutable_gate_like_operation(body.operations());
                });
                if nested_unroutable {
                    return false;
                }
            }
            _ => {}
        }
    }
    found
}

fn directly_lower_top_level_ccx(
    source: &Circuit,
    two_qubit_gate: StandardGate,
) -> Result<Circuit, CompilerError> {
    let rebuild = CircuitRebuildContext::new(source);
    let mut operations = Vec::with_capacity(source.operations().len());

    for operation in source.operations() {
        if matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::CCX)
        ) {
            let [control_0, control_1, target] = operation.qubits.as_slice() else {
                return Err(CompilerError::InvariantViolation(
                    "CCX operation must have exactly three qubits".to_string(),
                ));
            };
            append_direct_ccx_template(
                &mut operations,
                *control_0,
                *control_1,
                *target,
                two_qubit_gate,
            );
        } else {
            operations.push(rebuild.remap_preserved_operation(
                source,
                operation,
                rebuild.root_classical(),
            )?);
        }
    }

    rebuild.finish(source.qubits(), operations, source.global_phase())
}

fn append_direct_ccx_template(
    operations: &mut Vec<ValueOperation>,
    control_0: crate::circuit::Qubit,
    control_1: crate::circuit::Qubit,
    target: crate::circuit::Qubit,
    two_qubit_gate: StandardGate,
) {
    if two_qubit_gate == StandardGate::CZ {
        let gates = [
            (StandardGate::CZ, control_1, Some(target)),
            (StandardGate::H, target, None),
            (StandardGate::TDG, target, None),
            (StandardGate::H, target, None),
            (StandardGate::CZ, control_0, Some(target)),
            (StandardGate::H, target, None),
            (StandardGate::T, target, None),
            (StandardGate::H, target, None),
            (StandardGate::CZ, control_1, Some(target)),
            (StandardGate::H, target, None),
            (StandardGate::TDG, target, None),
            (StandardGate::H, target, None),
            (StandardGate::CZ, control_0, Some(target)),
            (StandardGate::H, target, None),
            (StandardGate::T, control_1, None),
            (StandardGate::T, target, None),
            (StandardGate::H, target, None),
            (StandardGate::H, control_1, None),
            (StandardGate::CZ, control_0, Some(control_1)),
            (StandardGate::H, control_1, None),
            (StandardGate::T, control_0, None),
            (StandardGate::TDG, control_1, None),
            (StandardGate::H, control_1, None),
            (StandardGate::CZ, control_0, Some(control_1)),
            (StandardGate::H, control_1, None),
        ];
        append_fixed_gate_sequence(operations, gates);
        return;
    }

    let gates = [
        (StandardGate::H, target, None),
        (StandardGate::CX, control_1, Some(target)),
        (StandardGate::TDG, target, None),
        (StandardGate::CX, control_0, Some(target)),
        (StandardGate::T, target, None),
        (StandardGate::CX, control_1, Some(target)),
        (StandardGate::TDG, target, None),
        (StandardGate::CX, control_0, Some(target)),
        (StandardGate::T, control_1, None),
        (StandardGate::T, target, None),
        (StandardGate::H, target, None),
        (StandardGate::CX, control_0, Some(control_1)),
        (StandardGate::T, control_0, None),
        (StandardGate::TDG, control_1, None),
        (StandardGate::CX, control_0, Some(control_1)),
    ];
    append_fixed_gate_sequence(operations, gates);
}

fn append_fixed_gate_sequence<const N: usize>(
    operations: &mut Vec<ValueOperation>,
    gates: [(
        StandardGate,
        crate::circuit::Qubit,
        Option<crate::circuit::Qubit>,
    ); N],
) {
    for (gate, first, second) in gates {
        let qubits = second.map_or_else(|| vec![first], |second| vec![first, second]);
        operations.push(ValueOperation::from_standard(gate, qubits, []));
    }
}

fn has_unroutable_gate_like_operation(operations: &[Operation]) -> bool {
    for operation in operations {
        match &operation.instruction {
            Instruction::Standard(_)
            | Instruction::McGate(_)
            | Instruction::UnitaryGate(_)
            | Instruction::CircuitGate(_) => {
                if operation.qubits.len() > 2 {
                    return true;
                }
            }
            Instruction::ClassicalControl(op) => {
                let mut found = false;
                for_each_control_body(op, |body| {
                    if !found {
                        found = has_unroutable_gate_like_operation(body.operations());
                    }
                });
                if found {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

fn collect_routable_two_qubit_gates(operations: &[Operation], basis: &mut Vec<StandardGate>) {
    for operation in operations {
        match &operation.instruction {
            Instruction::Standard(gate) if operation.qubits.len() == 2 => {
                if !basis.contains(gate) {
                    basis.push(*gate);
                }
            }
            Instruction::ClassicalControl(op) => {
                for_each_control_body(op, |body| {
                    collect_routable_two_qubit_gates(body.operations(), basis);
                });
            }
            _ => {}
        }
    }
}

fn validate_routing_basis_contract(operations: &[Operation]) -> Result<(), CompilerError> {
    for operation in operations {
        match &operation.instruction {
            Instruction::Standard(_)
            | Instruction::McGate(_)
            | Instruction::UnitaryGate(_)
            | Instruction::CircuitGate(_)
                if operation.qubits.len() > 2 =>
            {
                return Err(CompilerError::InvalidInput(format!(
                    "routing-basis lowering did not satisfy route.sabre input contract: found {}-qubit operation {}",
                    operation.qubits.len(),
                    operation.instruction
                )));
            }
            Instruction::ClassicalControl(op) => {
                let mut result = Ok(());
                for_each_control_body(op, |body| {
                    if result.is_ok() {
                        result = validate_routing_basis_contract(body.operations());
                    }
                });
                result?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn for_each_control_body(
    op: &ClassicalControlOp,
    mut visit: impl FnMut(&crate::circuit::ControlBody),
) {
    match op {
        ClassicalControlOp::If(op) => {
            visit(op.then_body());
            if let Some(body) = op.else_body() {
                visit(body);
            }
        }
        ClassicalControlOp::While(op) => {
            visit(op.body());
        }
        ClassicalControlOp::For(op) => {
            visit(op.body());
        }
        ClassicalControlOp::Switch(op) => {
            for case in op.cases() {
                visit(case.body());
            }
            if let Some(body) = op.default() {
                visit(body);
            }
        }
        ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
    }
}

#[cfg(test)]
#[path = "./routing_basis_test.rs"]
mod routing_basis_test;
