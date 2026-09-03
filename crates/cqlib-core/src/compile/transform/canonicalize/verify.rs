// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Input and output validation for the canonicalization contract.
//!
//! Validation is split into two modes. [`VerifyMode::Input`] checks that the
//! source circuit is structurally safe to rebuild: qubit references must be
//! known, operation arity must match the instruction, parameter references must
//! resolve, fixed numeric parameters must be finite, and non-barrier operations
//! must not repeat qubits.
//!
//! [`VerifyMode::Output`] applies the same structural checks and then enforces
//! the postconditions promised by the active [`CanonicalizeConfig`]. Production
//! output must not contain top-level `GPhase` operations, must keep any
//! control-flow-local `GPhase` as a single leading nonzero marker, must not
//! retain strict no-ops, must contain canonical barrier scopes, and must have a
//! parameter table with no unused entries.
//!
//! The verifier proves only the representation invariants owned by
//! canonicalization. It does not prove matrix equivalence, target-basis
//! validity, routing legality, or hardware direction constraints.

use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, ClassicalDataOp, Directive, Instruction, Operation,
    Parameter, StandardGate,
};
use crate::compile::CompilerError;
use smallvec::SmallVec;
use std::collections::{BTreeSet, HashSet};

use super::config::CanonicalizeConfig;
use super::ops::{BarrierRelation, barrier_relation, is_strict_noop, parameter_is_exact_zero};
use std::fmt;

#[derive(Clone, Copy)]
struct OperationScope<'a> {
    parent: &'a str,
    index: usize,
}

impl fmt::Display for OperationScope<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}[{}]", self.parent, self.index)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VerifyMode<'a> {
    Input,
    Output { config: &'a CanonicalizeConfig },
}

pub fn verify_circuit(circuit: &Circuit, mode: VerifyMode<'_>) -> Result<(), CompilerError> {
    if let CircuitParam::Index(index) = circuit.global_phase_param()
        && circuit.parameters().get_index(*index as usize).is_none()
    {
        return Err(CompilerError::InvalidInput(format!(
            "global phase references missing parameter index {}",
            index
        )));
    }
    let global_phase = circuit.global_phase();
    verify_parameter_finite(&global_phase, "global phase")?;
    if matches!(mode, VerifyMode::Output { .. }) {
        verify_parameter_canonical(&global_phase, circuit.global_phase_param(), "global phase")?;
        // A canonicalization round accumulates a zero phase contribution for
        // every retained root operation. The expression engine can normalize
        // a tiny numeric residue while constructing that sum even when the
        // residue alone is already a canonical numeric parameter. Reject that
        // representation here so the verified fast path is a true fixed point.
        if !circuit.operations().is_empty() && !global_phase.is_canonical_positive_zero() {
            let accumulated = (global_phase.clone() + Parameter::from(0.0))
                .canonicalized()
                .map_err(|error| {
                    CompilerError::Circuit(crate::circuit::CircuitError::InvalidParameter(error))
                })?;
            if accumulated != global_phase {
                return Err(CompilerError::InvariantViolation(
                    "global phase is not in accumulator-normal form".to_string(),
                ));
            }
        }
    }
    verify_operations(circuit, circuit.operations(), "root", mode)?;

    if matches!(mode, VerifyMode::Output { .. }) {
        verify_output_parameter_table(circuit)?;
    }

    Ok(())
}

fn verify_operations(
    circuit: &Circuit,
    operations: &[Operation],
    scope: &str,
    mode: VerifyMode<'_>,
) -> Result<(), CompilerError> {
    for (index, operation) in operations.iter().enumerate() {
        let op_scope = OperationScope {
            parent: scope,
            index,
        };

        for qubit in &operation.qubits {
            if !circuit.contains_qubit(qubit) {
                return Err(CompilerError::InvalidInput(format!(
                    "{op_scope} references unknown qubit {qubit}"
                )));
            }
        }

        verify_no_duplicate_qubits(operation, &op_scope)?;
        verify_instruction_arity(operation, &op_scope)?;
        verify_operation_params(circuit, operation, mode, &op_scope)?;

        if matches!(mode, VerifyMode::Output { config } if config.canonicalizes_instruction_form())
            && matches!(operation.instruction, Instruction::McGate(_))
            && operation.instruction.canonicalize_form() != operation.instruction
        {
            return Err(CompilerError::InvariantViolation(format!(
                "{op_scope} instruction form is not canonical"
            )));
        }

        match &operation.instruction {
            Instruction::ClassicalControl(control) => {
                verify_classical_control(circuit, control, mode, &op_scope)?;
                verify_control_flow_qubits(operation, mode, &op_scope)?;
            }
            Instruction::ClassicalData(ClassicalDataOp::Store { value, .. })
                if matches!(mode, VerifyMode::Output { .. }) && value.simplified() != *value =>
            {
                return Err(CompilerError::InvariantViolation(format!(
                    "{op_scope} classical expression is not simplified"
                )));
            }
            Instruction::Standard(StandardGate::GPhase) => match mode {
                VerifyMode::Input => {}
                VerifyMode::Output { config } if config.folds_gphase() => {
                    if scope == "root" {
                        return Err(CompilerError::InvariantViolation(format!(
                            "{op_scope} contains top-level GPhase after canonicalization"
                        )));
                    }
                    if index != 0 {
                        return Err(CompilerError::InvariantViolation(format!(
                            "{op_scope} contains non-leading control-flow body GPhase"
                        )));
                    }
                    if operation.params.len() != 1 {
                        return Err(CompilerError::InvariantViolation(format!(
                            "{op_scope} has malformed GPhase parameters"
                        )));
                    }
                    if parameter_is_exact_zero(&circuit.resolve_parameter(&operation.params[0])?)? {
                        return Err(CompilerError::InvariantViolation(format!(
                            "{op_scope} contains zero GPhase"
                        )));
                    }
                }
                VerifyMode::Output { .. } => {}
            },
            Instruction::Directive(Directive::Barrier) if matches!(mode, VerifyMode::Output { config } if config.canonicalizes_barriers()) =>
            {
                verify_output_barrier(operation, operations.get(index + 1), &op_scope)?;
            }
            _ => {}
        }

        if matches!(mode, VerifyMode::Output { config } if config.drops_noops()) {
            let params = operation
                .params
                .iter()
                .map(|param| circuit.resolve_parameter(param))
                .collect::<Result<Vec<_>, _>>()?;
            if is_strict_noop(&operation.instruction, &params)? {
                return Err(CompilerError::InvariantViolation(format!(
                    "{op_scope} contains a removable no-op"
                )));
            }
        }
    }

    Ok(())
}

fn verify_no_duplicate_qubits(
    operation: &Operation,
    scope: &OperationScope<'_>,
) -> Result<(), CompilerError> {
    if matches!(
        operation.instruction,
        Instruction::Directive(Directive::Barrier)
    ) {
        return Ok(());
    }

    if let Some((_, qubit)) = operation
        .qubits
        .iter()
        .enumerate()
        .find(|(index, qubit)| operation.qubits[..*index].contains(qubit))
    {
        return Err(CompilerError::InvalidInput(format!(
            "{scope} contains duplicate qubit {qubit}"
        )));
    }

    Ok(())
}

fn verify_instruction_arity(
    operation: &Operation,
    scope: &OperationScope<'_>,
) -> Result<(), CompilerError> {
    if let Some((expected_qubits, expected_params)) = operation.instruction.gate_arity() {
        return verify_fixed_arity(
            expected_qubits,
            operation.qubits.len(),
            expected_params,
            operation.params.len(),
            scope,
        );
    }

    match &operation.instruction {
        Instruction::Directive(Directive::Barrier) => {
            if !operation.params.is_empty() {
                return Err(CompilerError::InvalidInput(format!(
                    "{scope} barrier expects 0 parameters, got {}",
                    operation.params.len()
                )));
            }
        }
        Instruction::ClassicalControl(_) => {
            if !operation.params.is_empty() {
                return Err(CompilerError::InvalidInput(format!(
                    "{scope} control-flow operation expects 0 parameters, got {}",
                    operation.params.len()
                )));
            }
        }
        Instruction::ClassicalData(ClassicalDataOp::MeasureBits { result }) => {
            if !operation.params.is_empty() {
                return Err(CompilerError::InvalidInput(format!(
                    "{scope} measure_bits expects 0 parameters, got {}",
                    operation.params.len()
                )));
            }
            let expected = result.ty().width() as usize;
            if operation.qubits.len() != expected {
                return Err(CompilerError::InvalidInput(format!(
                    "{scope} qubit count mismatch: expected {expected}, got {}",
                    operation.qubits.len()
                )));
            }
        }
        _ => unreachable!("fixed-arity instructions are handled by Instruction::gate_arity"),
    }
    Ok(())
}

fn verify_fixed_arity(
    expected_qubits: usize,
    actual_qubits: usize,
    expected_params: usize,
    actual_params: usize,
    scope: &OperationScope<'_>,
) -> Result<(), CompilerError> {
    if expected_qubits != actual_qubits {
        return Err(CompilerError::InvalidInput(format!(
            "{scope} qubit count mismatch: expected {expected_qubits}, got {actual_qubits}"
        )));
    }
    if expected_params != actual_params {
        return Err(CompilerError::InvalidInput(format!(
            "{scope} parameter count mismatch: expected {expected_params}, got {actual_params}"
        )));
    }
    Ok(())
}

fn verify_operation_params(
    circuit: &Circuit,
    operation: &Operation,
    mode: VerifyMode<'_>,
    scope: &OperationScope<'_>,
) -> Result<(), CompilerError> {
    for (param_index, param) in operation.params.iter().enumerate() {
        match param {
            CircuitParam::Fixed(value) => {
                if !value.is_finite() {
                    return Err(CompilerError::InvalidInput(format!(
                        "{scope} parameter {param_index} is non-finite: {value}"
                    )));
                }
            }
            CircuitParam::Index(index) => {
                let Some(param) = circuit.parameters().get_index(*index as usize) else {
                    return Err(CompilerError::InvalidInput(format!(
                        "{scope} references missing parameter index {index}"
                    )));
                };
                verify_parameter_finite(param, format_args!("{scope} parameter {param_index}"))?;
            }
        }
        if matches!(mode, VerifyMode::Output { .. }) {
            let resolved = circuit.resolve_parameter(param)?;
            verify_parameter_canonical(
                &resolved,
                param,
                format_args!("{scope} parameter {param_index}"),
            )?;
        }
    }
    Ok(())
}

fn verify_parameter_canonical(
    param: &Parameter,
    stored: &CircuitParam,
    scope: impl fmt::Display,
) -> Result<(), CompilerError> {
    let canonical = param.canonicalized().map_err(|error| {
        CompilerError::Circuit(crate::circuit::CircuitError::InvalidParameter(error))
    })?;
    if canonical != *param
        || (canonical.get_symbols().is_empty() && !matches!(stored, CircuitParam::Fixed(_)))
    {
        return Err(CompilerError::InvariantViolation(format!(
            "{scope} is not stored in canonical form"
        )));
    }
    Ok(())
}

fn verify_parameter_finite(
    param: &Parameter,
    scope: impl fmt::Display,
) -> Result<(), CompilerError> {
    if param.get_symbols().is_empty() {
        let value = param.evaluate(&None).map_err(|error| {
            CompilerError::InvalidInput(format!("{scope} cannot be evaluated: {error}"))
        })?;
        if !value.is_finite() {
            return Err(CompilerError::InvalidInput(format!(
                "{scope} evaluates to non-finite value {value}"
            )));
        }
    }
    Ok(())
}

fn verify_control_flow_qubits(
    operation: &Operation,
    mode: VerifyMode<'_>,
    scope: &OperationScope<'_>,
) -> Result<(), CompilerError> {
    let expected: SmallVec<[_; 3]> = match &operation.instruction {
        Instruction::ClassicalControl(control) => control.used_qubits().into_iter().collect(),
        _ => SmallVec::new(),
    };
    match mode {
        VerifyMode::Input => {
            for qubit in &expected {
                if !operation.qubits.contains(qubit) {
                    return Err(CompilerError::InvalidInput(format!(
                        "{scope} outer qubit list is missing required qubit {qubit}"
                    )));
                }
            }
        }
        VerifyMode::Output { .. } => {
            if operation.qubits != expected {
                return Err(CompilerError::InvariantViolation(format!(
                    "{scope} control-flow qubits are not canonical: expected {:?}, got {:?}",
                    expected, operation.qubits
                )));
            }
        }
    }
    Ok(())
}

fn verify_classical_control(
    circuit: &Circuit,
    control: &ClassicalControlOp,
    mode: VerifyMode<'_>,
    scope: &OperationScope<'_>,
) -> Result<(), CompilerError> {
    let body_mode = match mode {
        VerifyMode::Output { config } if !config.recurses_control_flow() => VerifyMode::Input,
        _ => mode,
    };

    if matches!(mode, VerifyMode::Output { config } if config.recurses_control_flow()) {
        let expressions_are_canonical = match control {
            ClassicalControlOp::If(op) => op.condition().simplified() == *op.condition(),
            ClassicalControlOp::While(op) => op.condition().simplified() == *op.condition(),
            ClassicalControlOp::For(op) => {
                op.start().simplified() == *op.start()
                    && op.stop().simplified() == *op.stop()
                    && op.step().simplified() == *op.step()
            }
            ClassicalControlOp::Switch(op) => op.target().simplified() == *op.target(),
            ClassicalControlOp::Break | ClassicalControlOp::Continue => true,
        };
        if !expressions_are_canonical {
            return Err(CompilerError::InvariantViolation(format!(
                "{scope} control-flow expression is not simplified"
            )));
        }
    }

    match control {
        ClassicalControlOp::If(op) => {
            verify_operations(
                circuit,
                op.then_body().operations(),
                &format!("{scope}.then"),
                body_mode,
            )?;
            if let Some(body) = op.else_body() {
                verify_operations(
                    circuit,
                    body.operations(),
                    &format!("{scope}.else"),
                    body_mode,
                )?;
            }
        }
        ClassicalControlOp::While(op) => {
            verify_operations(
                circuit,
                op.body().operations(),
                &format!("{scope}.body"),
                body_mode,
            )?;
        }
        ClassicalControlOp::For(op) => {
            verify_operations(
                circuit,
                op.body().operations(),
                &format!("{scope}.body"),
                body_mode,
            )?;
        }
        ClassicalControlOp::Switch(op) => {
            for case in op.cases() {
                verify_operations(
                    circuit,
                    case.body().operations(),
                    &format!("{scope}.case({})", case.value()),
                    body_mode,
                )?;
            }
            if let Some(body) = op.default() {
                verify_operations(
                    circuit,
                    body.operations(),
                    &format!("{scope}.default"),
                    body_mode,
                )?;
            }
        }
        ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
    }
    Ok(())
}

fn verify_output_barrier(
    operation: &Operation,
    next: Option<&Operation>,
    scope: &OperationScope<'_>,
) -> Result<(), CompilerError> {
    // An empty scope is the canonical representation of a global barrier.
    // Only local barrier scopes require sorted, deduplicated operands.
    if !operation.qubits.is_empty() {
        let mut sorted = operation.qubits.clone();
        sorted.sort_unstable_by_key(|qubit| qubit.id());
        sorted.dedup();
        if operation.qubits != sorted {
            return Err(CompilerError::InvariantViolation(format!(
                "{scope} barrier qubits are not sorted and deduplicated"
            )));
        }
    }
    if operation.label.is_some() {
        return Err(CompilerError::InvariantViolation(format!(
            "{scope} barrier label was not cleared"
        )));
    }
    if let Some(next) = next {
        let relation = barrier_relation(&operation.qubits, &next.qubits);
        if matches!(next.instruction, Instruction::Directive(Directive::Barrier))
            && matches!(
                relation,
                BarrierRelation::Equal
                    | BarrierRelation::LeftSuperset
                    | BarrierRelation::RightSuperset
            )
        {
            return Err(CompilerError::InvariantViolation(format!(
                "{scope} has mergeable adjacent barrier"
            )));
        }
    }
    Ok(())
}

fn verify_output_parameter_table(circuit: &Circuit) -> Result<(), CompilerError> {
    let mut used = BTreeSet::new();
    if let CircuitParam::Index(index) = circuit.global_phase_param() {
        used.insert(*index);
    }
    collect_used_parameters(circuit.operations(), &mut used);

    for index in 0..circuit.parameters().len() {
        if !used.contains(&(index as u32)) {
            return Err(CompilerError::InvariantViolation(format!(
                "parameter table contains unused parameter index {index}"
            )));
        }
    }

    // Canonicalization rebuilds the parameter table from actual operation and
    // phase usage. The symbol cache must exactly reflect that rebuilt table or
    // downstream symbolic-parameter analysis can become stale.
    let mut expected_symbols = HashSet::new();
    for param in circuit.parameters() {
        expected_symbols.extend(param.get_symbols());
    }
    let actual_symbols: HashSet<_> = circuit.symbols().iter().cloned().collect();
    if expected_symbols != actual_symbols {
        return Err(CompilerError::InvariantViolation(
            "symbol table does not match canonical parameter table".to_string(),
        ));
    }

    Ok(())
}

fn collect_used_parameters(operations: &[Operation], used: &mut BTreeSet<u32>) {
    for operation in operations {
        for param in &operation.params {
            if let CircuitParam::Index(index) = param {
                used.insert(*index);
            }
        }
        if let Instruction::ClassicalControl(control) = &operation.instruction {
            collect_control_parameters(control, used);
        }
    }
}

fn collect_control_parameters(control: &ClassicalControlOp, used: &mut BTreeSet<u32>) {
    match control {
        ClassicalControlOp::If(op) => {
            collect_used_parameters(op.then_body().operations(), used);
            if let Some(body) = op.else_body() {
                collect_used_parameters(body.operations(), used);
            }
        }
        ClassicalControlOp::While(op) => collect_used_parameters(op.body().operations(), used),
        ClassicalControlOp::For(op) => collect_used_parameters(op.body().operations(), used),
        ClassicalControlOp::Switch(op) => {
            for case in op.cases() {
                collect_used_parameters(case.body().operations(), used);
            }
            if let Some(body) = op.default() {
                collect_used_parameters(body.operations(), used);
            }
        }
        ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
    }
}
