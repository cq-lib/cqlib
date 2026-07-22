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

//! Canonicalizer entry point and round orchestration.

use crate::circuit::value_instruction::storage_operation_to_value;
use crate::circuit::{
    Circuit, CircuitError, CircuitParam, ClassicalControlOp, ClassicalDataOp, ControlBody,
    Directive, ForOp, IfOp, Instruction, Operation, Parameter, StandardGate, SwitchCase, SwitchOp,
    ValueOperation, WhileOp,
};
use crate::compile::CompilerError;
use crate::compile::transform::transformer::{TransformResult, Transformer};
use smallvec::{SmallVec, smallvec};

use super::config::CanonicalizeConfig;
use super::equivalence::circuits_equivalent_for_canonicalize;
use super::ops::{canonicalize_operation_qubits, is_strict_noop, push_operation};
use super::verify::{VerifyMode, verify_circuit};

/// Result of a canonicalization run.
#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalizeResult {
    /// Canonicalized circuit.
    pub circuit: Circuit,
    /// Whether the output differs from the input representation.
    pub changed: bool,
    /// Number of canonicalization rounds executed.
    pub rounds: u8,
}

/// Canonicalizer entry point.
#[derive(Debug, Clone)]
pub struct Canonicalizer {
    config: CanonicalizeConfig,
}

impl Default for Canonicalizer {
    fn default() -> Self {
        Self::production()
    }
}

impl Canonicalizer {
    /// Creates a canonicalizer with the supplied configuration.
    pub const fn new(config: CanonicalizeConfig) -> Self {
        Self { config }
    }

    /// Creates a canonicalizer using production defaults.
    pub const fn production() -> Self {
        Self::new(CanonicalizeConfig::production())
    }

    /// Returns the active configuration.
    pub const fn config(&self) -> &CanonicalizeConfig {
        &self.config
    }

    /// Canonicalizes `circuit` and returns the rebuilt circuit plus run metadata.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::{Circuit, Qubit};
    /// use cqlib_core::compile::transform::Canonicalizer;
    ///
    /// let mut circuit = Circuit::new(1);
    /// circuit.h(Qubit::new(0)).unwrap();
    ///
    /// let result = Canonicalizer::production().run(&circuit).unwrap();
    /// assert_eq!(result.circuit.qubits(), circuit.qubits());
    /// assert!(result.rounds >= 1);
    /// let _changed = result.changed;
    /// ```
    pub fn run(&self, circuit: &Circuit) -> Result<CanonicalizeResult, CompilerError> {
        verify_circuit(circuit, VerifyMode::Input)?;

        if self.config.round_limit() == 0 {
            return Err(CompilerError::InvalidInput(
                "canonicalize round_limit must be greater than zero".to_string(),
            ));
        }

        // A single pass can expose new canonicalization opportunities. For
        // example, parameter simplification can turn `theta - theta` into a
        // fixed zero, which then lets the next pass remove a rotation. The loop
        // therefore proves a stable representation before reporting success.
        let mut current = rebuild_circuit_from_value_operations(
            circuit,
            value_operations_from(circuit)?,
            circuit.global_phase(),
        )?;
        for round in 1..=self.config.round_limit() {
            let next = CanonicalizeRound::new(&current, &self.config).run()?;
            verify_circuit(
                &next,
                VerifyMode::Output {
                    config: &self.config,
                },
            )?;

            if circuits_equivalent_for_canonicalize(&current, &next) {
                return Ok(CanonicalizeResult {
                    circuit: next,
                    changed: !circuits_equivalent_for_canonicalize(circuit, &current),
                    rounds: round,
                });
            }

            current = next;
        }

        Err(CompilerError::InvariantViolation(format!(
            "canonicalization did not reach a fixed point within {} rounds",
            self.config.round_limit()
        )))
    }
}

// Transformer integration keeps only the common circuit/changed shape; callers
// that need canonicalization rounds should use `Canonicalizer::run` directly.
impl Transformer for Canonicalizer {
    fn name(&self) -> &'static str {
        "canonicalize"
    }

    fn transform(
        &self,
        circuit: &Circuit,
        _analysis: Option<&crate::compile::transform::CircuitAnalysis>,
    ) -> Result<TransformResult, CompilerError> {
        let result = self.run(circuit)?;
        Ok(TransformResult {
            circuit: result.circuit,
            changed: result.changed,
        })
    }
}

/// Canonicalizes a circuit using production defaults.
pub fn canonicalize_circuit(circuit: &Circuit) -> Result<CanonicalizeResult, CompilerError> {
    Canonicalizer::production().run(circuit)
}

struct CanonicalizeRound<'a> {
    source: &'a Circuit,
    config: &'a CanonicalizeConfig,
    target: Circuit,
    top_phase: Parameter,
}

impl<'a> CanonicalizeRound<'a> {
    fn new(source: &'a Circuit, config: &'a CanonicalizeConfig) -> Self {
        Self {
            source,
            config,
            target: Circuit::from_qubits(source.qubits()).expect("source qubits are unique"),
            top_phase: source.global_phase(),
        }
    }

    fn run(mut self) -> Result<Circuit, CompilerError> {
        self.top_phase = compiler_parameter(self.top_phase.canonicalized())?;
        let mut top_level = Vec::with_capacity(self.source.operations().len());

        // Top-level `GPhase` operations are not retained as operations. Their
        // phase contribution is accumulated here and materialized into
        // `Circuit::global_phase` after all operations have been rebuilt.
        for operation in self.source.operations() {
            let rewritten = self.rewrite_operation(operation, ScopeKind::TopLevel)?;
            self.top_phase =
                compiler_parameter((self.top_phase + rewritten.phase).canonicalized())?;
            for operation in rewritten.operations {
                push_operation(&mut top_level, operation, self.config);
            }
        }

        let phase = compiler_parameter(self.top_phase.canonicalized())?;
        let operations = top_level
            .into_iter()
            .map(|operation| self.value_operation(operation))
            .collect::<Result<Vec<_>, _>>()?;
        rebuild_circuit_from_value_operations(self.source, operations, phase)
    }

    fn canonicalize_body(&mut self, body: &[Operation]) -> Result<Vec<Operation>, CompilerError> {
        let mut out = Vec::with_capacity(body.len());
        let mut body_phase = Parameter::from(0.0);

        // Body-local phase is conditional on the enclosing control-flow branch
        // or loop iteration, so it cannot be lifted to circuit global phase.
        // The canonical body representation keeps it as one leading `GPhase`.
        for operation in body {
            let rewritten = self.rewrite_operation(operation, ScopeKind::ControlFlowBody)?;
            body_phase = compiler_parameter((body_phase + rewritten.phase).canonicalized())?;
            for operation in rewritten.operations {
                push_operation(&mut out, operation, self.config);
            }
        }

        body_phase = compiler_parameter(body_phase.canonicalized())?;
        if !compiler_parameter(body_phase.is_exact_zero())? {
            let param = self.intern_parameter(body_phase)?;
            out.insert(
                0,
                Operation {
                    instruction: Instruction::Standard(StandardGate::GPhase),
                    qubits: smallvec![],
                    params: smallvec![param],
                    label: None,
                },
            );
        }

        Ok(out)
    }

    fn rewrite_operation(
        &mut self,
        operation: &Operation,
        scope: ScopeKind,
    ) -> Result<RewriteResult, CompilerError> {
        let mut instruction = operation.instruction.clone();
        if self.config.canonicalizes_instruction_form() {
            instruction = instruction.canonicalize_form();
        }

        if self.config.recurses_control_flow() {
            instruction = self.rewrite_control_flow_instruction(instruction)?;
        }

        // Simplify runtime classical expressions embedded in data ops.
        if let Instruction::ClassicalData(data) = &instruction {
            instruction = Instruction::ClassicalData(simplify_classical_data_op(data));
        }

        let qubits = canonicalize_operation_qubits(&instruction, &operation.qubits, self.config);
        let semantic_params = operation
            .params
            .iter()
            .map(|param| self.source.resolve_parameter(param))
            .collect::<Result<Vec<_>, _>>()?;

        if self.config.folds_gphase()
            && matches!(instruction, Instruction::Standard(StandardGate::GPhase))
        {
            let phase = semantic_params
                .first()
                .cloned()
                .unwrap_or_else(|| Parameter::from(0.0));
            return Ok(RewriteResult::phase(phase));
        }

        if self.config.drops_noops() && is_strict_noop(&instruction, &semantic_params, &qubits)? {
            return Ok(RewriteResult::drop());
        }

        let params = semantic_params
            .into_iter()
            .map(|param| self.intern_parameter(param))
            .collect::<Result<SmallVec<[CircuitParam; 1]>, _>>()?;

        let mut label = operation.label.clone();
        if self.config.canonicalizes_barriers()
            && matches!(instruction, Instruction::Directive(Directive::Barrier))
        {
            label = None;
        }

        let mut operation = Operation {
            instruction,
            qubits,
            params,
            label,
        };

        // The operation-level qubit list is derived from the canonicalized body,
        // not preserved from input. This prevents deleted body no-ops from
        // keeping dead qubits visible to later analysis passes.
        if matches!(scope, ScopeKind::TopLevel | ScopeKind::ControlFlowBody)
            && let Instruction::ClassicalControl(control) = &operation.instruction
        {
            operation.qubits = control.used_qubits().into_iter().collect();
        }

        Ok(RewriteResult::keep(operation))
    }

    fn rewrite_control_flow_instruction(
        &mut self,
        instruction: Instruction,
    ) -> Result<Instruction, CompilerError> {
        match instruction {
            Instruction::ClassicalControl(control) => Ok(Instruction::ClassicalControl(
                self.rewrite_control_flow(control)?,
            )),
            _ => Ok(instruction),
        }
    }

    /// Rebuilds one structured classical-control operation after body
    /// rewriting and expression simplification.
    fn rewrite_control_flow(
        &mut self,
        control: ClassicalControlOp,
    ) -> Result<ClassicalControlOp, CompilerError> {
        match control {
            ClassicalControlOp::If(op) => {
                let then_body = self.canonicalize_control_body(op.then_body())?;
                let else_body = op
                    .else_body()
                    .map(|body| self.canonicalize_control_body(body))
                    .transpose()?;
                Ok(ClassicalControlOp::If(
                    IfOp::new(op.condition().simplified(), then_body, else_body)
                        .map_err(CompilerError::Circuit)?,
                ))
            }
            ClassicalControlOp::While(op) => {
                let body = self.canonicalize_control_body(op.body())?;
                Ok(ClassicalControlOp::While(
                    WhileOp::new(op.condition().simplified(), body)
                        .map_err(CompilerError::Circuit)?,
                ))
            }
            ClassicalControlOp::For(op) => {
                let body = self.canonicalize_control_body(op.body())?;
                Ok(ClassicalControlOp::For(
                    ForOp::new(
                        op.var(),
                        op.start().simplified(),
                        op.stop().simplified(),
                        op.step().simplified(),
                        body,
                    )
                    .map_err(CompilerError::Circuit)?,
                ))
            }
            ClassicalControlOp::Switch(op) => {
                let mut cases = Vec::with_capacity(op.cases().len());
                for case in op.cases() {
                    cases.push(SwitchCase::new(
                        case.value(),
                        self.canonicalize_control_body(case.body())?,
                    ));
                }
                let default = op
                    .default()
                    .map(|body| self.canonicalize_control_body(body))
                    .transpose()?;
                Ok(ClassicalControlOp::Switch(
                    SwitchOp::new(op.target().simplified(), cases, default)
                        .map_err(CompilerError::Circuit)?,
                ))
            }
            ClassicalControlOp::Break => Ok(ClassicalControlOp::Break),
            ClassicalControlOp::Continue => Ok(ClassicalControlOp::Continue),
        }
    }

    /// Canonicalizes a structured control-flow body and wraps it back in `ControlBody`.
    fn canonicalize_control_body(
        &mut self,
        body: &ControlBody,
    ) -> Result<ControlBody, CompilerError> {
        Ok(ControlBody::new(self.canonicalize_body(body.operations())?))
    }

    fn intern_parameter(&mut self, param: Parameter) -> Result<CircuitParam, CompilerError> {
        Ok(self.target.map_param(param)?)
    }

    /// Resolves canonical operation parameters into value-level operations.
    ///
    /// Final circuit construction goes through [`Circuit::from_operations`] so
    /// the rebuilt circuit can inherit the source circuit's classical-handle
    /// identity and validate `ClassicalData`/`ClassicalControl` operations.
    fn value_operation(&self, operation: Operation) -> Result<ValueOperation, CompilerError> {
        storage_operation_to_value(operation, &|param| self.target.parameter_value(param))
            .map_err(CompilerError::Circuit)
    }
}

/// Simplifies the runtime classical expression inside a [`ClassicalDataOp`].
fn simplify_classical_data_op(op: &ClassicalDataOp) -> ClassicalDataOp {
    match op {
        ClassicalDataOp::Store { target, value } => ClassicalDataOp::Store {
            target: *target,
            value: value.simplified(),
        },
        ClassicalDataOp::MeasureBit { result } => ClassicalDataOp::MeasureBit { result: *result },
        ClassicalDataOp::MeasureBits { result } => ClassicalDataOp::MeasureBits { result: *result },
    }
}

fn compiler_parameter<T>(
    result: Result<T, crate::circuit::error::ParameterError>,
) -> Result<T, CompilerError> {
    result.map_err(|error| CompilerError::Circuit(CircuitError::InvalidParameter(error)))
}

/// Converts a circuit operation list into value-level operations without
/// remapping classical handles.
fn value_operations_from(circuit: &Circuit) -> Result<Vec<ValueOperation>, CompilerError> {
    circuit
        .operations()
        .iter()
        .cloned()
        .map(|operation| {
            storage_operation_to_value(operation, &|param| circuit.parameter_value(param))
                .map_err(CompilerError::Circuit)
        })
        .collect()
}

/// Rebuilds a circuit while preserving runtime classical tables and handles.
fn rebuild_circuit_from_value_operations(
    source: &Circuit,
    operations: Vec<ValueOperation>,
    global_phase: Parameter,
) -> Result<Circuit, CompilerError> {
    let mut target = Circuit::from_operations(
        source.qubits(),
        operations,
        Some(source.classical_vars().to_vec()),
        Some(source.classical_values().to_vec()),
    )
    .map_err(CompilerError::Circuit)?;
    target.set_global_phase(global_phase);
    Ok(target)
}

#[derive(Debug, Clone, Copy)]
enum ScopeKind {
    TopLevel,
    ControlFlowBody,
}

#[derive(Debug, Clone)]
struct RewriteResult {
    operations: Vec<Operation>,
    phase: Parameter,
}

impl RewriteResult {
    fn keep(operation: Operation) -> Self {
        Self {
            operations: vec![operation],
            phase: Parameter::from(0.0),
        }
    }

    fn drop() -> Self {
        Self {
            operations: Vec::new(),
            phase: Parameter::from(0.0),
        }
    }

    fn phase(phase: Parameter) -> Self {
        Self {
            operations: Vec::new(),
            phase,
        }
    }
}
