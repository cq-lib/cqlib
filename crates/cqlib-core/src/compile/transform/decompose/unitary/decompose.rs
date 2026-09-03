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

//! Numeric matrix-backed `UnitaryGate` synthesis.
//!
//! This is the circuit-facing entry point for [`super`]. It rebuilds a source
//! circuit while replacing remaining matrix-backed one- and two-qubit
//! `UnitaryGate` operations with standard-gate sequences. Definition expansion
//! is expected to run first: a unitary gate without a matrix representation is
//! rejected with an error directing the caller to expand definitions.
//!
//! Preserved operations are copied into a fresh circuit instead of retaining
//! source [`CircuitParam`] indices. Their parameters are resolved against the
//! source parameter table and interned into the target table. This applies
//! recursively to preserved control-flow bodies as well.
//!
//! Synthesized global phases are accumulated into the rebuilt circuit. For a
//! decomposition nested inside a control-flow body, the phase is represented by
//! a leading [`StandardGate::GPhase`] operation in that body, because it is
//! conditional on executing the body and cannot be lifted to the circuit-level
//! phase.
//!
//! This pass deliberately leaves ordinary standard gates such as `RX` and `RZ`
//! unchanged. Target-basis rewrites own that behavior. Its supported input
//! contract requires finite numeric one- or two-qubit unitary matrices. It
//! rejects unresolved or non-finite unitary parameters, missing matrices,
//! invalid matrix shapes, and unitary gates acting on three or more qubits.

use super::DeviceTwoQubitSynthesisContext;
use super::two_qubit_kak::kak_decompose;
use super::unitary_1q::{OneQubitUnitaryDecomposition, synthesize_numeric_1q_unitary};
use super::unitary_2q::{
    TwoQubitSynthesisRequest, TwoQubitSynthesisTarget, plan_numeric_2q_unitary_for_device_from_kak,
    plan_numeric_2q_unitary_from_kak, select_validated_device_unitary_candidate_from_kak,
    select_validated_numeric_2q_candidate_from_kak,
};
use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, Instruction, Operation, Parameter, ParameterValue,
    Qubit, StandardGate, UnitaryGate, ValueClassicalControlOp, ValueControlBody, ValueInstruction,
    ValueOperation, ValueSwitchCase,
};
use crate::compile::CompilerError;
use crate::compile::transform::analysis::WorkflowCircuitAnalysis;
use crate::compile::transform::decompose::rule::{
    DecompositionRuleCache, DecompositionRuleStats, NumericUnitaryRuleRequest,
    NumericUnitarySynthesisKey,
};
use crate::compile::transform::rebuild::{CircuitRebuildContext, ClassicalRemap};
use crate::compile::transform::transformer::{PassApplicability, WorkflowPass};
use crate::compile::transform::{CircuitAnalysis, TransformOutcome, Transformer};
use ndarray::Array2;
use num_complex::Complex64;
use smallvec::smallvec;
use std::borrow::Cow;
use std::collections::HashMap;

const SYNTHESIS_NAME: &str = "decompose.unitary";
const ANGLE_EPS: f64 = 1e-12;
const PHASE_EPS: f64 = 1e-12;

/// Configuration for circuit-level matrix-backed `UnitaryGate` synthesis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitaryDecomposeConfig {
    /// Target capability used by exact two-qubit numerical synthesis.
    pub two_qubit_target: TwoQubitSynthesisTarget,
    /// Whether matrix-backed `UnitaryGate` operations inside control-flow
    /// bodies should be synthesized recursively.
    pub recurse_control_flow: bool,
}

impl Default for UnitaryDecomposeConfig {
    fn default() -> Self {
        Self {
            two_qubit_target: TwoQubitSynthesisTarget::default(),
            recurse_control_flow: true,
        }
    }
}

/// [`Transformer`] adapter for [`decompose_unitaries`].
///
/// Configuration is bound at construction time.
#[derive(Debug, Clone)]
pub struct DecomposeUnitaries {
    config: UnitaryDecomposeConfig,
    device_context: Option<DeviceTwoQubitSynthesisContext>,
}

impl DecomposeUnitaries {
    pub fn new(config: UnitaryDecomposeConfig) -> Self {
        Self {
            config,
            device_context: None,
        }
    }

    pub(crate) fn new_device_aware(
        config: UnitaryDecomposeConfig,
        device_context: DeviceTwoQubitSynthesisContext,
    ) -> Self {
        Self {
            config,
            device_context: Some(device_context),
        }
    }
}

impl Default for DecomposeUnitaries {
    fn default() -> Self {
        Self::new(UnitaryDecomposeConfig::default())
    }
}

impl WorkflowPass for DecomposeUnitaries {
    fn applicability(&self, analysis: &WorkflowCircuitAnalysis) -> PassApplicability {
        if analysis.has_unitary_gates() {
            PassApplicability::Run
        } else {
            PassApplicability::ProvenNoOp("circuit contains no unitary operations")
        }
    }
}

impl Transformer for DecomposeUnitaries {
    fn name(&self) -> &'static str {
        SYNTHESIS_NAME
    }

    fn transform(
        &self,
        circuit: &Circuit,
        analysis: Option<&CircuitAnalysis>,
    ) -> Result<TransformOutcome, CompilerError> {
        let local_analysis;
        let analysis = match analysis {
            Some(analysis) => analysis,
            None => {
                local_analysis = CircuitAnalysis::analyze(circuit);
                &local_analysis
            }
        };
        if !analysis.has_unitary_gates {
            return Ok(TransformOutcome::Unchanged);
        }
        decompose_unitaries_transform_with_device(
            circuit,
            self.config.clone(),
            self.device_context.clone(),
        )
    }
}

/// Rewrites supported matrix-backed `UnitaryGate` operations in `circuit`.
///
/// The output circuit owns a fresh parameter table. Every preserved operation
/// parameter is resolved against the source circuit and rebuilt through the
/// target circuit instead of copying `CircuitParam::Index` values.
///
/// One-qubit matrices are emitted as [`StandardGate::U`] operations when a
/// non-trivial local gate remains. Two-qubit matrices are emitted according to
/// [`UnitaryDecomposeConfig::two_qubit_target`]. Synthesized scalar phases are
/// preserved either as circuit global phase or, within recursively processed
/// control-flow bodies, as explicit [`StandardGate::GPhase`] operations.
///
/// # Errors
///
/// Returns [`CompilerError`] if a matrix-backed `UnitaryGate` has non-fixed
/// parameters, no matrix representation, unsupported arity, invalid qubit
/// arity, or if the numeric 1q/2q synthesis primitive rejects the matrix.
pub fn decompose_unitaries(
    circuit: &Circuit,
    config: UnitaryDecomposeConfig,
) -> Result<Circuit, CompilerError> {
    Ok(decompose_unitaries_transform(circuit, config)?.into_circuit(circuit))
}

fn decompose_unitaries_transform(
    circuit: &Circuit,
    config: UnitaryDecomposeConfig,
) -> Result<TransformOutcome, CompilerError> {
    decompose_unitaries_transform_with_rule_stats(circuit, config).map(|(result, _)| result)
}

fn decompose_unitaries_transform_with_device(
    circuit: &Circuit,
    config: UnitaryDecomposeConfig,
    device_context: Option<DeviceTwoQubitSynthesisContext>,
) -> Result<TransformOutcome, CompilerError> {
    decompose_unitaries_transform_with_context(circuit, config, device_context)
        .map(|(result, _)| result)
}

/// Rewrites supported matrix-backed unitary operations and returns runtime rule stats.
///
/// This diagnostic entry point exposes pass-local decomposition-rule reuse for
/// numeric unitary synthesis. Matrix cache keys are bit-exact.
pub fn decompose_unitaries_with_rule_stats(
    circuit: &Circuit,
    config: UnitaryDecomposeConfig,
) -> Result<(TransformOutcome, DecompositionRuleStats), CompilerError> {
    decompose_unitaries_transform_with_rule_stats(circuit, config)
}

fn decompose_unitaries_transform_with_rule_stats(
    circuit: &Circuit,
    config: UnitaryDecomposeConfig,
) -> Result<(TransformOutcome, DecompositionRuleStats), CompilerError> {
    decompose_unitaries_transform_with_context(circuit, config, None)
}

fn decompose_unitaries_transform_with_context(
    circuit: &Circuit,
    config: UnitaryDecomposeConfig,
    device_context: Option<DeviceTwoQubitSynthesisContext>,
) -> Result<(TransformOutcome, DecompositionRuleStats), CompilerError> {
    let decomposer = UnitaryDecomposer {
        source: circuit,
        rebuild: CircuitRebuildContext::new(circuit),
        top_phase: circuit.global_phase(),
        config,
        device_context,
        device_cache: HashMap::new(),
        rule_cache: DecompositionRuleCache::default(),
        changed: false,
    };
    decomposer.run()
}

struct UnitaryDecomposer<'a> {
    source: &'a Circuit,
    rebuild: CircuitRebuildContext,
    top_phase: Parameter,
    config: UnitaryDecomposeConfig,
    device_context: Option<DeviceTwoQubitSynthesisContext>,
    device_cache: HashMap<DeviceUnitaryCacheKey, Decomposition>,
    rule_cache: DecompositionRuleCache,
    changed: bool,
}

#[derive(Clone)]
struct Decomposition {
    operations: Vec<ValueOperation>,
    phase_delta: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct DeviceUnitaryCacheKey {
    matrix_bits: Vec<(u64, u64)>,
    ordered_qargs: [Qubit; 2],
}

impl<'a> UnitaryDecomposer<'a> {
    fn run(mut self) -> Result<(TransformOutcome, DecompositionRuleStats), CompilerError> {
        let root_classical = self.rebuild.root_classical().clone();
        let mut operations = Vec::with_capacity(self.source.operations().len());
        let phase_delta =
            self.apply_sequence(self.source.operations(), &root_classical, &mut operations)?;
        if phase_delta.abs() > PHASE_EPS {
            self.top_phase = self.top_phase + Parameter::from(phase_delta);
        }
        let circuit = self
            .rebuild
            .finish(self.source.qubits(), operations, self.top_phase)?;
        let stats = self.rule_cache.stats();
        // Even when no unitary was synthesized, rebuilding can compact and
        // remap the parameter table. In that case the source IR cannot be
        // retained exactly, so the rebuilt representation is still Changed.
        let changed = self.changed || circuit != *self.source;
        let outcome = if changed {
            TransformOutcome::Changed(circuit)
        } else {
            TransformOutcome::Unchanged
        };
        Ok((outcome, stats))
    }

    fn apply_sequence(
        &mut self,
        source_operations: &[Operation],
        classical_remap: &ClassicalRemap,
        output: &mut Vec<ValueOperation>,
    ) -> Result<f64, CompilerError> {
        let mut phase_delta = 0.0;

        for operation in source_operations {
            let decomposition = self.decompose_operation(operation, classical_remap)?;
            phase_delta += decomposition.phase_delta;
            output.extend(decomposition.operations);
        }

        Ok(phase_delta)
    }

    fn decompose_operation(
        &mut self,
        operation: &Operation,
        classical_remap: &ClassicalRemap,
    ) -> Result<Decomposition, CompilerError> {
        match &operation.instruction {
            Instruction::UnitaryGate(gate) => self.decompose_unitary_gate(gate, operation),
            Instruction::ClassicalControl(control) => {
                self.decompose_control_flow(operation, control, classical_remap)
            }
            _ => Ok(Decomposition {
                operations: vec![self.rebuild.remap_preserved_operation(
                    self.source,
                    operation,
                    classical_remap,
                )?],
                phase_delta: 0.0,
            }),
        }
    }

    fn decompose_control_flow(
        &mut self,
        operation: &Operation,
        control: &ClassicalControlOp,
        classical_remap: &ClassicalRemap,
    ) -> Result<Decomposition, CompilerError> {
        let instruction = match control {
            ClassicalControlOp::If(op) => {
                let then_body = self.rebuild_body(op.then_body().operations(), classical_remap)?;
                let else_body = op
                    .else_body()
                    .map(|body| self.rebuild_body(body.operations(), classical_remap))
                    .transpose()?;
                ValueClassicalControlOp::If {
                    condition: classical_remap.remap_expr(op.condition())?,
                    then_body: ValueControlBody::new(then_body),
                    else_body: else_body.map(ValueControlBody::new),
                }
            }
            ClassicalControlOp::While(op) => {
                let body = self.rebuild_body(op.body().operations(), classical_remap)?;
                ValueClassicalControlOp::While {
                    condition: classical_remap.remap_expr(op.condition())?,
                    body: ValueControlBody::new(body),
                }
            }
            ClassicalControlOp::For(op) => {
                let body = self.rebuild_body(op.body().operations(), classical_remap)?;
                ValueClassicalControlOp::For {
                    var: classical_remap.remap_var(op.var())?,
                    start: classical_remap.remap_expr(op.start())?,
                    stop: classical_remap.remap_expr(op.stop())?,
                    step: classical_remap.remap_expr(op.step())?,
                    body: ValueControlBody::new(body),
                }
            }
            ClassicalControlOp::Switch(op) => {
                let cases = op
                    .cases()
                    .iter()
                    .map(|case| {
                        Ok(ValueSwitchCase::new(
                            case.value(),
                            ValueControlBody::new(
                                self.rebuild_body(case.body().operations(), classical_remap)?,
                            ),
                        ))
                    })
                    .collect::<Result<Vec<_>, CompilerError>>()?;
                let default = op
                    .default()
                    .map(|body| self.rebuild_body(body.operations(), classical_remap))
                    .transpose()?
                    .map(ValueControlBody::new);
                ValueClassicalControlOp::Switch {
                    target: classical_remap.remap_expr(op.target())?,
                    cases,
                    default,
                }
            }
            ClassicalControlOp::Break => ValueClassicalControlOp::Break,
            ClassicalControlOp::Continue => ValueClassicalControlOp::Continue,
        };
        let qubits = instruction.used_qubits().into_iter().collect();

        Ok(Decomposition {
            operations: vec![ValueOperation {
                instruction: ValueInstruction::ClassicalControl(instruction),
                qubits,
                params: CircuitRebuildContext::resolve_source_params(
                    self.source,
                    &operation.params,
                )?,
                label: operation.label.clone(),
            }],
            phase_delta: 0.0,
        })
    }

    fn rebuild_body(
        &mut self,
        source_body: &[Operation],
        classical_remap: &ClassicalRemap,
    ) -> Result<Vec<ValueOperation>, CompilerError> {
        let mut body = Vec::with_capacity(source_body.len());
        if self.config.recurse_control_flow {
            let phase_delta = self.apply_sequence(source_body, classical_remap, &mut body)?;
            if phase_delta.abs() > PHASE_EPS {
                body.insert(
                    0,
                    ValueOperation {
                        instruction: ValueInstruction::from_instruction(Instruction::Standard(
                            StandardGate::GPhase,
                        )),
                        qubits: smallvec![],
                        params: smallvec![ParameterValue::Fixed(phase_delta)],
                        label: None,
                    },
                );
            }
        } else {
            for operation in source_body {
                body.push(self.rebuild.remap_preserved_operation(
                    self.source,
                    operation,
                    classical_remap,
                )?);
            }
        }
        Ok(body)
    }

    fn decompose_unitary_gate(
        &mut self,
        gate: &UnitaryGate,
        operation: &Operation,
    ) -> Result<Decomposition, CompilerError> {
        self.changed = true;
        if operation.qubits.len() != gate.num_qubits() as usize {
            return Err(CompilerError::TransformFailed {
                name: SYNTHESIS_NAME,
                reason: format!(
                    "operation qubit count mismatch for UnitaryGate '{}': expected {}, got {}",
                    gate.label(),
                    gate.num_qubits(),
                    operation.qubits.len()
                ),
            });
        }
        if gate.matrix_repr().is_none() {
            return Err(CompilerError::TransformFailed {
                name: SYNTHESIS_NAME,
                reason: format!(
                    "UnitaryGate '{}' has no matrix representation; run definition expansion before unitary synthesis",
                    gate.label()
                ),
            });
        }

        let matrix = self.numeric_matrix_for_gate(gate, operation)?;
        match gate.num_qubits() {
            1 => {
                let request = NumericUnitaryRuleRequest {
                    num_qubits: gate.num_qubits(),
                    matrix: matrix.as_ref(),
                    synthesis_key: NumericUnitarySynthesisKey::one_qubit(),
                };
                if let Some((operations, phase_delta)) = self
                    .rule_cache
                    .instantiate_numeric_unitary(request.clone(), &operation.qubits)?
                {
                    return Ok(Decomposition {
                        operations,
                        phase_delta,
                    });
                }
                let OneQubitUnitaryDecomposition {
                    theta,
                    phi,
                    lambda,
                    global_phase,
                } = synthesize_numeric_1q_unitary(matrix.as_ref()).map_err(|source| {
                    CompilerError::TransformFailed {
                        name: SYNTHESIS_NAME,
                        reason: format!(
                            "one-qubit synthesis failed for UnitaryGate '{}': {source}",
                            gate.label()
                        ),
                    }
                })?;
                let mut operations = Vec::new();
                if theta.abs() > ANGLE_EPS || phi.abs() > ANGLE_EPS || lambda.abs() > ANGLE_EPS {
                    operations.push(ValueOperation {
                        instruction: ValueInstruction::from_instruction(Instruction::Standard(
                            StandardGate::U,
                        )),
                        qubits: operation.qubits.clone(),
                        params: smallvec![
                            ParameterValue::Fixed(theta),
                            ParameterValue::Fixed(phi),
                            ParameterValue::Fixed(lambda)
                        ],
                        label: None,
                    });
                }
                let decomposition = Decomposition {
                    operations,
                    phase_delta: global_phase,
                };
                self.rule_cache.insert_numeric_unitary(
                    request,
                    &operation.qubits,
                    &decomposition.operations,
                    decomposition.phase_delta,
                )?;
                Ok(decomposition)
            }
            2 => {
                let qubits = [operation.qubits[0], operation.qubits[1]];
                if let Some(context) = self.device_context.as_ref() {
                    let cache_key = DeviceUnitaryCacheKey {
                        matrix_bits: matrix
                            .iter()
                            .map(|value| (value.re.to_bits(), value.im.to_bits()))
                            .collect(),
                        ordered_qargs: qubits,
                    };
                    if let Some(decomposition) = self.device_cache.get(&cache_key) {
                        return Ok(decomposition.clone());
                    }
                    let decomp = kak_decompose(matrix.as_ref())?;
                    let candidates = plan_numeric_2q_unitary_for_device_from_kak(
                        matrix.as_ref(),
                        qubits,
                        context,
                        &decomp,
                    )?;
                    let candidate = select_validated_device_unitary_candidate_from_kak(
                        matrix.as_ref(),
                        qubits,
                        context,
                        &decomp,
                        candidates,
                    )?
                    .ok_or_else(|| CompilerError::TransformFailed {
                            name: SYNTHESIS_NAME,
                            reason: format!(
                                "no exact device-aware two-qubit synthesis candidate for UnitaryGate '{}'",
                                gate.label()
                            ),
                        })?;
                    let decomposition = Decomposition {
                        operations: candidate.operations,
                        phase_delta: candidate.global_phase,
                    };
                    self.device_cache.insert(cache_key, decomposition.clone());
                    return Ok(decomposition);
                }
                let request = NumericUnitaryRuleRequest {
                    num_qubits: gate.num_qubits(),
                    matrix: matrix.as_ref(),
                    synthesis_key: NumericUnitarySynthesisKey::two_qubit(
                        &self.config.two_qubit_target,
                    ),
                };
                if let Some((operations, phase_delta)) = self
                    .rule_cache
                    .instantiate_numeric_unitary(request.clone(), &operation.qubits)?
                {
                    return Ok(Decomposition {
                        operations,
                        phase_delta,
                    });
                }
                let decomp = kak_decompose(matrix.as_ref())?;
                let candidates = plan_numeric_2q_unitary_from_kak(
                    TwoQubitSynthesisRequest {
                        matrix: matrix.as_ref(),
                        qubits,
                        target: self.config.two_qubit_target.clone(),
                    },
                    &decomp,
                )?;
                let candidate = select_validated_numeric_2q_candidate_from_kak(
                    matrix.as_ref(),
                    qubits,
                    &self.config.two_qubit_target,
                    &decomp,
                    candidates,
                )?
                .ok_or_else(|| CompilerError::TransformFailed {
                    name: SYNTHESIS_NAME,
                    reason: format!(
                        "no exact two-qubit synthesis candidate for UnitaryGate '{}'",
                        gate.label()
                    ),
                })?;
                let crate::compile::transform::decompose::unitary::TwoQubitSynthesisCandidate {
                    operations,
                    global_phase,
                    ..
                } = candidate;
                let decomposition = Decomposition {
                    operations,
                    phase_delta: global_phase,
                };
                self.rule_cache.insert_numeric_unitary(
                    request,
                    &operation.qubits,
                    &decomposition.operations,
                    decomposition.phase_delta,
                )?;
                Ok(decomposition)
            }
            other => Err(CompilerError::TransformFailed {
                name: SYNTHESIS_NAME,
                reason: format!(
                    "3q+ UnitaryGate synthesis is not supported yet; gate '{}' has {other} qubits",
                    gate.label()
                ),
            }),
        }
    }

    fn numeric_matrix_for_gate<'gate>(
        &self,
        gate: &'gate UnitaryGate,
        operation: &Operation,
    ) -> Result<Cow<'gate, Array2<Complex64>>, CompilerError> {
        let mut fixed_params = Vec::with_capacity(operation.params.len());
        for (position, param) in operation.params.iter().enumerate() {
            let parameter = self.resolve_source_param(param)?;
            let value = parameter.evaluate(&None).map_err(|_| {
                let mut symbols = parameter.get_symbols().into_iter().collect::<Vec<_>>();
                symbols.sort();
                let detail = if symbols.is_empty() {
                    parameter.to_string()
                } else {
                    symbols.join(", ")
                };
                CompilerError::TransformFailed {
                    name: SYNTHESIS_NAME,
                    reason: format!(
                        "UnitaryGate '{}' parameter {position} must be fixed numeric before synthesis; unresolved symbols: {detail}",
                        gate.label()
                    ),
                }
            })?;
            if !value.is_finite() {
                return Err(CompilerError::InvalidInput(format!(
                    "non-finite unitary parameter {value} at position {position} for UnitaryGate '{}'",
                    gate.label()
                )));
            }
            fixed_params.push(value);
        }

        gate.matrix_for_params(&fixed_params)
            .map_err(|source| CompilerError::TransformFailed {
                name: SYNTHESIS_NAME,
                reason: format!(
                    "failed to resolve numeric matrix for UnitaryGate '{}': {source}",
                    gate.label()
                ),
            })
    }

    fn resolve_source_param(&self, param: &CircuitParam) -> Result<Parameter, CompilerError> {
        match param {
            CircuitParam::Fixed(value) => {
                if !value.is_finite() {
                    return Err(CompilerError::InvalidInput(format!(
                        "non-finite fixed parameter {value}"
                    )));
                }
                Ok(Parameter::from(*value))
            }
            CircuitParam::Index(index) => self
                .source
                .parameters()
                .get_index(*index as usize)
                .cloned()
                .ok_or_else(|| {
                    CompilerError::InvalidInput(format!("missing parameter index {index}"))
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::Qubit;
    use crate::circuit::gate::gate_matrix;
    use crate::circuit::symbolic_matrix::{SymbolicComplex, SymbolicMatrix};
    use crate::circuit::{CircuitError, ClassicalExpr, circuit_to_matrix};
    use crate::compile::transform::{
        ResolvedTransform, TransformerTestExt, resolve_transform_for_test,
    };
    use approx::assert_abs_diff_eq;
    use ndarray::array;

    fn decompose_unitaries_with_rule_stats(
        circuit: &Circuit,
        config: UnitaryDecomposeConfig,
    ) -> Result<(ResolvedTransform, DecompositionRuleStats), CompilerError> {
        super::decompose_unitaries_with_rule_stats(circuit, config)
            .map(|(outcome, stats)| (resolve_transform_for_test(outcome, circuit), stats))
    }

    fn config_for_native_2q(gate: StandardGate) -> UnitaryDecomposeConfig {
        UnitaryDecomposeConfig {
            two_qubit_target: TwoQubitSynthesisTarget::from_standard_gates(
                vec![
                    StandardGate::U,
                    StandardGate::H,
                    StandardGate::RX,
                    StandardGate::RY,
                    StandardGate::RZ,
                    StandardGate::S,
                    StandardGate::SDG,
                ],
                vec![gate],
                true,
            )
            .unwrap(),
            ..Default::default()
        }
    }

    fn operation_parameter(circuit: &Circuit, operation: &Operation, position: usize) -> Parameter {
        match operation.params.get(position) {
            Some(CircuitParam::Fixed(value)) => Parameter::from(*value),
            Some(CircuitParam::Index(index)) => circuit.parameters()[*index as usize].clone(),
            None => panic!("operation has no parameter at position {position}"),
        }
    }

    #[test]
    fn transform_reports_unchanged_without_unitary_gates() {
        let mut circuit = Circuit::new(1);
        circuit.h(Qubit::new(0)).unwrap();

        let result = DecomposeUnitaries::default()
            .transform_resolved(&circuit, None)
            .unwrap();

        assert!(!result.changed);
        assert_eq!(result.circuit.operations().len(), 1);
        assert!(matches!(
            result.circuit.operations()[0].instruction,
            Instruction::Standard(StandardGate::H)
        ));
    }

    #[test]
    fn decomposes_numeric_1q_unitary_gate() {
        let gamma = 0.37;
        let matrix = gate_matrix::u_gate(0.3, 0.4, 0.5) * Complex64::from_polar(1.0, gamma);
        let gate = UnitaryGate::new("custom_u", 1, 0)
            .with_matrix(matrix)
            .unwrap();
        let mut circuit = Circuit::new(1);
        circuit.unitary(gate, vec![Qubit::new(0)]).unwrap();

        let before = circuit_to_matrix(&circuit, None).unwrap();
        let decomposed = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap();
        let after = circuit_to_matrix(&decomposed, None).unwrap();

        assert!(decomposed.operations().iter().all(|operation| matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::U)
        )));
        assert_abs_diff_eq!(before, after, epsilon = 1e-8);
    }

    #[test]
    fn repeated_numeric_1q_unitary_reuses_runtime_rule() {
        let gamma = 0.17;
        let matrix = gate_matrix::u_gate(0.2, -0.3, 0.4) * Complex64::from_polar(1.0, gamma);
        let first_gate = UnitaryGate::new("first_u", 1, 0)
            .with_matrix(matrix.clone())
            .unwrap();
        let second_gate = UnitaryGate::new("second_u", 1, 0)
            .with_matrix(matrix)
            .unwrap();
        let mut circuit = Circuit::new(2);
        circuit.unitary(first_gate, vec![Qubit::new(0)]).unwrap();
        circuit.unitary(second_gate, vec![Qubit::new(1)]).unwrap();
        let before = circuit_to_matrix(&circuit, None).unwrap();

        let (result, stats) =
            decompose_unitaries_with_rule_stats(&circuit, UnitaryDecomposeConfig::default())
                .unwrap();
        let after = circuit_to_matrix(&result.circuit, None).unwrap();

        assert_eq!(stats.misses, 1);
        assert_eq!(stats.inserts, 1);
        assert_eq!(stats.hits, 1);
        assert!(result.circuit.operations().iter().all(|operation| matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::U)
        )));
        assert_abs_diff_eq!(before, after, epsilon = 1e-8);
    }

    #[test]
    fn decomposes_unitary_inside_runtime_classical_control() {
        let gate = UnitaryGate::new("runtime_u", 1, 0)
            .with_matrix(gate_matrix::u_gate(0.3, 0.4, 0.5))
            .unwrap();
        let mut circuit = Circuit::new(1);
        let measured = circuit.measure(Qubit::new(0)).unwrap();
        circuit
            .if_(
                ClassicalExpr::bit_to_bool(measured.expr()).unwrap(),
                |body| {
                    body.append(
                        Instruction::UnitaryGate(Box::new(gate.clone())),
                        [Qubit::new(0)],
                        [],
                        None,
                    )
                },
            )
            .unwrap();

        let result = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap();

        assert_eq!(result.classical_values().len(), 1);
        assert!(result.validate().is_ok());
        let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
            &result.operations()[1].instruction
        else {
            panic!("expected runtime classical if operation");
        };
        assert!(
            op.then_body()
                .operations()
                .iter()
                .all(|operation| { !matches!(operation.instruction, Instruction::UnitaryGate(_)) })
        );
    }

    #[test]
    fn accumulates_top_level_synthesized_global_phase() {
        let phase = 0.62;
        let matrix = StandardGate::X
            .matrix(&[])
            .unwrap()
            .into_owned()
            .mapv(|value| Complex64::from_polar(1.0, phase) * value);
        let gate = UnitaryGate::new("phase_x", 1, 0)
            .with_matrix(matrix)
            .unwrap();
        let mut circuit = Circuit::new(1);
        circuit.set_global_phase(Parameter::from(0.13));
        circuit.unitary(gate, vec![Qubit::new(0)]).unwrap();

        let before = circuit_to_matrix(&circuit, None).unwrap();
        let decomposed = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap();
        let after = circuit_to_matrix(&decomposed, None).unwrap();

        assert!(decomposed.operations().iter().all(|operation| matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::U)
        )));
        assert!(decomposed.global_phase().evaluate(&None).unwrap().abs() > 0.1);
        assert_abs_diff_eq!(before, after, epsilon = 1e-8);
    }

    #[test]
    fn decomposes_numeric_2q_unitary_gate_with_pauli_backend() {
        let matrix = StandardGate::FSIM
            .matrix(&[0.2, -0.3])
            .unwrap()
            .into_owned();
        let gate = UnitaryGate::new("custom_2q", 2, 0)
            .with_matrix(matrix)
            .unwrap();
        let mut circuit = Circuit::new(2);
        circuit
            .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
            .unwrap();

        let before = circuit_to_matrix(&circuit, None).unwrap();
        let decomposed = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap();
        let after = circuit_to_matrix(&decomposed, None).unwrap();

        assert!(decomposed.operations().iter().all(|operation| matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::U)
                | Instruction::Standard(StandardGate::RXX)
                | Instruction::Standard(StandardGate::RYY)
                | Instruction::Standard(StandardGate::RZZ)
        )));
        assert_abs_diff_eq!(before, after, epsilon = 1e-8);
    }

    #[test]
    fn repeated_numeric_2q_unitary_reuses_target_aware_runtime_rule() {
        let matrix = StandardGate::FSIM
            .matrix(&[0.23, -0.31])
            .unwrap()
            .into_owned();
        let first_gate = UnitaryGate::new("first_2q", 2, 0)
            .with_matrix(matrix.clone())
            .unwrap();
        let second_gate = UnitaryGate::new("second_2q", 2, 0)
            .with_matrix(matrix)
            .unwrap();
        let mut circuit = Circuit::new(4);
        circuit
            .unitary(first_gate, vec![Qubit::new(0), Qubit::new(1)])
            .unwrap();
        circuit
            .unitary(second_gate, vec![Qubit::new(2), Qubit::new(3)])
            .unwrap();
        let before = circuit_to_matrix(&circuit, None).unwrap();

        let (result, stats) =
            decompose_unitaries_with_rule_stats(&circuit, UnitaryDecomposeConfig::default())
                .unwrap();
        let after = circuit_to_matrix(&result.circuit, None).unwrap();

        assert_eq!(stats.misses, 1);
        assert_eq!(stats.inserts, 1);
        assert_eq!(stats.hits, 1);
        assert!(result.circuit.operations().iter().all(|operation| matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::U)
                | Instruction::Standard(StandardGate::RXX)
                | Instruction::Standard(StandardGate::RYY)
                | Instruction::Standard(StandardGate::RZZ)
        )));
        assert_abs_diff_eq!(before, after, epsilon = 1e-8);
    }

    #[test]
    fn decomposes_numeric_2q_unitary_gate_with_cx_backend() {
        let matrix = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
        let gate = UnitaryGate::new("custom_2q", 2, 0)
            .with_matrix(matrix)
            .unwrap();
        let mut circuit = Circuit::new(2);
        circuit
            .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
            .unwrap();
        let config = config_for_native_2q(StandardGate::CX);

        let before = circuit_to_matrix(&circuit, None).unwrap();
        let decomposed = decompose_unitaries(&circuit, config).unwrap();
        let after = circuit_to_matrix(&decomposed, None).unwrap();

        assert!(decomposed.operations().iter().all(|operation| matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::U) | Instruction::Standard(StandardGate::CX)
        )));
        assert_abs_diff_eq!(before, after, epsilon = 1e-8);
    }

    #[test]
    fn decomposes_numeric_2q_unitary_gate_with_cz_backend() {
        let matrix = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
        let gate = UnitaryGate::new("custom_2q", 2, 0)
            .with_matrix(matrix)
            .unwrap();
        let mut circuit = Circuit::new(2);
        circuit
            .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
            .unwrap();
        let config = config_for_native_2q(StandardGate::CZ);

        let before = circuit_to_matrix(&circuit, None).unwrap();
        let decomposed = decompose_unitaries(&circuit, config).unwrap();
        let after = circuit_to_matrix(&decomposed, None).unwrap();

        assert!(decomposed.operations().iter().all(|operation| matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::U) | Instruction::Standard(StandardGate::CZ)
        )));
        assert_abs_diff_eq!(before, after, epsilon = 1e-8);
    }

    #[test]
    fn decomposes_numeric_2q_unitary_gate_with_cy_backend() {
        let matrix = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
        let gate = UnitaryGate::new("custom_2q", 2, 0)
            .with_matrix(matrix)
            .unwrap();
        let mut circuit = Circuit::new(2);
        circuit
            .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
            .unwrap();
        let config = config_for_native_2q(StandardGate::CY);

        let before = circuit_to_matrix(&circuit, None).unwrap();
        let decomposed = decompose_unitaries(&circuit, config).unwrap();
        let after = circuit_to_matrix(&decomposed, None).unwrap();

        assert!(decomposed.operations().iter().all(|operation| matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::U) | Instruction::Standard(StandardGate::CY)
        )));
        assert_abs_diff_eq!(before, after, epsilon = 1e-8);
    }

    #[test]
    fn decomposes_numeric_2q_unitary_gate_with_rzz_backend() {
        let matrix = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
        let gate = UnitaryGate::new("custom_2q", 2, 0)
            .with_matrix(matrix)
            .unwrap();
        let mut circuit = Circuit::new(2);
        circuit
            .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
            .unwrap();
        let config = config_for_native_2q(StandardGate::RZZ);

        let before = circuit_to_matrix(&circuit, None).unwrap();
        let decomposed = decompose_unitaries(&circuit, config).unwrap();
        let after = circuit_to_matrix(&decomposed, None).unwrap();

        assert!(decomposed.operations().iter().all(|operation| matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::U)
                | Instruction::Standard(StandardGate::H)
                | Instruction::Standard(StandardGate::RX)
                | Instruction::Standard(StandardGate::RZZ)
        )));
        assert_eq!(
            decomposed
                .operations()
                .iter()
                .filter(|operation| matches!(
                    operation.instruction,
                    Instruction::Standard(StandardGate::RZZ)
                ))
                .count(),
            3
        );
        assert_abs_diff_eq!(before, after, epsilon = 1e-8);
    }

    #[test]
    fn remaps_preserved_top_level_operation_parameters() {
        let mut circuit = Circuit::new(2);
        circuit.add_parameter(Parameter::symbol("unused"));
        let theta = Parameter::symbol("theta");
        circuit
            .append(
                Instruction::Standard(StandardGate::RXX),
                [Qubit::new(0), Qubit::new(1)],
                [ParameterValue::Param(theta)],
                None,
            )
            .unwrap();

        let decomposed = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap();
        let operation = &decomposed.operations()[0];
        let CircuitParam::Index(index) = operation.params[0] else {
            panic!("expected symbolic parameter index");
        };

        assert_eq!(index, 0);
        assert_eq!(decomposed.parameters().len(), 1);
        assert!(
            operation_parameter(&decomposed, operation, 0)
                .get_symbols()
                .contains("theta")
        );
    }

    #[test]
    fn remaps_preserved_control_flow_body_parameters() {
        let mut circuit = Circuit::new(2);
        circuit.add_parameter(Parameter::symbol("unused"));
        let (theta_index, _) = circuit.add_parameter(Parameter::symbol("theta"));
        let body_ops = vec![Operation {
            instruction: Instruction::Standard(StandardGate::RZ),
            qubits: smallvec![Qubit::new(1)],
            params: smallvec![CircuitParam::Index(theta_index as u32)],
            label: None,
        }];
        let body_param_values: Vec<Vec<ParameterValue>> = body_ops
            .iter()
            .map(|op| {
                op.params
                    .iter()
                    .map(|p| circuit.parameter_value(p))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        circuit
            .if_else(
                ClassicalExpr::bool_literal(true),
                move |body| {
                    for (op, params) in body_ops.into_iter().zip(body_param_values) {
                        body.append(op.instruction, op.qubits, params, op.label.as_deref())?;
                    }
                    Ok(())
                },
                |_| Ok(()),
            )
            .unwrap();

        let decomposed = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap();
        let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
            &decomposed.operations()[0].instruction
        else {
            panic!("expected if operation");
        };
        let operation = &op.then_body().operations()[0];
        let CircuitParam::Index(index) = operation.params[0] else {
            panic!("expected remapped symbolic parameter index");
        };

        assert_eq!(index, 0);
        assert_eq!(decomposed.parameters().len(), 1);
        assert!(
            operation_parameter(&decomposed, operation, 0)
                .get_symbols()
                .contains("theta")
        );
    }

    #[test]
    fn remaps_preserved_false_body_parameters() {
        let mut circuit = Circuit::new(2);
        circuit.add_parameter(Parameter::symbol("unused"));
        let (phi_index, _) = circuit.add_parameter(Parameter::symbol("phi"));
        let true_body_ops = vec![Operation {
            instruction: Instruction::Standard(StandardGate::I),
            qubits: smallvec![Qubit::new(1)],
            params: smallvec![],
            label: None,
        }];
        let false_body_ops = vec![Operation {
            instruction: Instruction::Standard(StandardGate::RZ),
            qubits: smallvec![Qubit::new(1)],
            params: smallvec![CircuitParam::Index(phi_index as u32)],
            label: None,
        }];
        let true_body_param_values: Vec<Vec<ParameterValue>> = true_body_ops
            .iter()
            .map(|op| {
                op.params
                    .iter()
                    .map(|p| circuit.parameter_value(p))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let false_body_param_values: Vec<Vec<ParameterValue>> = false_body_ops
            .iter()
            .map(|op| {
                op.params
                    .iter()
                    .map(|p| circuit.parameter_value(p))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        circuit
            .if_else(
                ClassicalExpr::bool_literal(true),
                move |body| {
                    for (op, params) in true_body_ops.into_iter().zip(true_body_param_values) {
                        body.append(op.instruction, op.qubits, params, op.label.as_deref())?;
                    }
                    Ok(())
                },
                move |body| {
                    for (op, params) in false_body_ops.into_iter().zip(false_body_param_values) {
                        body.append(op.instruction, op.qubits, params, op.label.as_deref())?;
                    }
                    Ok(())
                },
            )
            .unwrap();

        let decomposed = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap();
        let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
            &decomposed.operations()[0].instruction
        else {
            panic!("expected if operation");
        };
        let else_body = op.else_body().expect("expected else body");
        let CircuitParam::Index(index) = else_body.operations()[0].params[0] else {
            panic!("expected remapped false-body parameter index");
        };

        assert_eq!(index, 0);
        assert_eq!(decomposed.parameters().len(), 1);
        assert!(
            operation_parameter(&decomposed, &else_body.operations()[0], 0)
                .get_symbols()
                .contains("phi")
        );
    }

    #[test]
    fn remaps_preserved_while_body_parameters() {
        let mut circuit = Circuit::new(2);
        circuit.add_parameter(Parameter::symbol("unused"));
        let (beta_index, _) = circuit.add_parameter(Parameter::symbol("beta"));
        let body_ops = vec![Operation {
            instruction: Instruction::Standard(StandardGate::RY),
            qubits: smallvec![Qubit::new(1)],
            params: smallvec![CircuitParam::Index(beta_index as u32)],
            label: None,
        }];
        let body_param_values: Vec<Vec<ParameterValue>> = body_ops
            .iter()
            .map(|op| {
                op.params
                    .iter()
                    .map(|p| circuit.parameter_value(p))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        circuit
            .while_(ClassicalExpr::bool_literal(true), move |body| {
                for (op, params) in body_ops.into_iter().zip(body_param_values) {
                    body.append(op.instruction, op.qubits, params, op.label.as_deref())?;
                }
                Ok(())
            })
            .unwrap();

        let decomposed = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap();
        let Instruction::ClassicalControl(ClassicalControlOp::While(op)) =
            &decomposed.operations()[0].instruction
        else {
            panic!("expected while operation");
        };
        let CircuitParam::Index(index) = op.body().operations()[0].params[0] else {
            panic!("expected remapped while-body parameter index");
        };

        assert_eq!(index, 0);
        assert_eq!(decomposed.parameters().len(), 1);
        assert!(
            operation_parameter(&decomposed, &op.body().operations()[0], 0)
                .get_symbols()
                .contains("beta")
        );
    }

    #[test]
    fn keeps_body_local_phase_inside_control_flow() {
        let phase = Complex64::from_polar(1.0, 0.25);
        let gate = UnitaryGate::new("phase_cx", 2, 0)
            .with_matrix(
                StandardGate::CX
                    .matrix(&[])
                    .unwrap()
                    .into_owned()
                    .mapv(|value| phase * value),
            )
            .unwrap();
        let mut circuit = Circuit::new(3);
        circuit
            .if_else(
                ClassicalExpr::bool_literal(true),
                |body| {
                    body.append(
                        Instruction::UnitaryGate(Box::new(gate)),
                        [Qubit::new(1), Qubit::new(2)],
                        Vec::<ParameterValue>::new(),
                        None,
                    )
                },
                |_| Ok(()),
            )
            .unwrap();

        let decomposed = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap();
        assert!(decomposed.global_phase().is_zero());
        let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
            &decomposed.operations()[0].instruction
        else {
            panic!("expected if operation");
        };
        assert!(matches!(
            op.then_body().operations()[0].instruction,
            Instruction::Standard(StandardGate::GPhase)
        ));
        assert!(matches!(
            op.then_body().operations()[0].params[0],
            CircuitParam::Fixed(_)
        ));
    }

    #[test]
    fn recurses_nested_control_flow_bodies() {
        let phase = Complex64::from_polar(1.0, 0.41);
        let gate = UnitaryGate::new("phase_x", 1, 0)
            .with_matrix(
                StandardGate::X
                    .matrix(&[])
                    .unwrap()
                    .into_owned()
                    .mapv(|value| phase * value),
            )
            .unwrap();
        let mut circuit = Circuit::new(2);
        circuit
            .if_else(
                ClassicalExpr::bool_literal(true),
                |body| {
                    body.while_(ClassicalExpr::bool_literal(true), |inner| {
                        inner.append(
                            Instruction::UnitaryGate(Box::new(gate)),
                            [Qubit::new(1)],
                            Vec::<ParameterValue>::new(),
                            None,
                        )
                    })
                },
                |_| Ok(()),
            )
            .unwrap();

        let decomposed = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap();
        let Instruction::ClassicalControl(ClassicalControlOp::If(outer)) =
            &decomposed.operations()[0].instruction
        else {
            panic!("expected outer if operation");
        };
        let Instruction::ClassicalControl(ClassicalControlOp::While(inner)) =
            &outer.then_body().operations()[0].instruction
        else {
            panic!("expected nested while operation");
        };

        assert!(matches!(
            inner.body().operations()[0].instruction,
            Instruction::Standard(StandardGate::GPhase)
        ));
        assert!(inner.body().operations().iter().any(|operation| matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::U)
        )));
    }

    #[test]
    fn can_preserve_control_flow_body_unitaries_without_recursing() {
        let mut circuit = Circuit::new(2);
        circuit.add_parameter(Parameter::symbol("unused"));
        let (theta_index, _) = circuit.add_parameter(Parameter::symbol("theta"));
        let gate = UnitaryGate::new("x_body", 1, 0)
            .with_matrix(StandardGate::X.matrix(&[]).unwrap().into_owned())
            .unwrap();
        let body_ops = vec![
            Operation {
                instruction: Instruction::UnitaryGate(Box::new(gate)),
                qubits: smallvec![Qubit::new(1)],
                params: smallvec![],
                label: None,
            },
            Operation {
                instruction: Instruction::Standard(StandardGate::RZ),
                qubits: smallvec![Qubit::new(1)],
                params: smallvec![CircuitParam::Index(theta_index as u32)],
                label: None,
            },
        ];
        let body_param_values: Vec<Vec<ParameterValue>> = body_ops
            .iter()
            .map(|op| {
                op.params
                    .iter()
                    .map(|p| circuit.parameter_value(p))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        circuit
            .if_else(
                ClassicalExpr::bool_literal(true),
                move |body| {
                    for (op, params) in body_ops.into_iter().zip(body_param_values) {
                        body.append(op.instruction, op.qubits, params, op.label.as_deref())?;
                    }
                    Ok(())
                },
                |_| Ok(()),
            )
            .unwrap();
        let config = UnitaryDecomposeConfig {
            recurse_control_flow: false,
            ..Default::default()
        };

        let decomposed = decompose_unitaries(&circuit, config).unwrap();
        let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
            &decomposed.operations()[0].instruction
        else {
            panic!("expected if operation");
        };
        let body = op.then_body().operations();
        let CircuitParam::Index(index) = body[1].params[0] else {
            panic!("expected remapped preserved body parameter index");
        };

        assert!(matches!(body[0].instruction, Instruction::UnitaryGate(_)));
        assert_eq!(index, 0);
        assert_eq!(decomposed.parameters().len(), 1);
    }

    #[test]
    fn rejects_unbound_symbolic_unitary_gate() {
        let theta = Parameter::symbol("theta");
        let matrix: SymbolicMatrix = array![
            [SymbolicComplex::one(), SymbolicComplex::zero()],
            [SymbolicComplex::zero(), SymbolicComplex::exp_i(theta)]
        ];
        let gate = UnitaryGate::new("symbolic_1q", 1, 1)
            .with_symbolic_matrix(["theta"], matrix)
            .unwrap();
        let mut circuit = Circuit::new(1);
        circuit
            .unitary_with_params(
                gate,
                vec![Qubit::new(0)],
                vec![Parameter::symbol("alpha").into()],
            )
            .unwrap();

        let err = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap_err();
        assert!(
            err.to_string()
                .contains("parameter 0 must be fixed numeric")
        );
        assert!(err.to_string().contains("unresolved symbols: alpha"));
    }

    #[test]
    fn rejects_unitary_gate_without_matrix_representation() {
        let gate = UnitaryGate::new("opaque", 1, 0);
        let mut circuit = Circuit::new(1);
        circuit.unitary(gate, vec![Qubit::new(0)]).unwrap();

        let err = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap_err();

        assert!(matches!(err, CompilerError::TransformFailed { .. }));
        assert!(err.to_string().contains("no matrix representation"));
    }

    #[test]
    fn rejects_non_finite_unitary_parameter() {
        let theta = Parameter::symbol("theta");
        let matrix: SymbolicMatrix = array![
            [SymbolicComplex::one(), SymbolicComplex::zero()],
            [SymbolicComplex::zero(), SymbolicComplex::exp_i(theta)]
        ];
        let gate = UnitaryGate::new("parameterized_phase", 1, 1)
            .with_symbolic_matrix(["theta"], matrix)
            .unwrap();
        let mut circuit = Circuit::new(1);
        let err = circuit
            .unitary_with_params(gate, vec![Qubit::new(0)], vec![f64::NAN.into()])
            .unwrap_err();

        assert!(matches!(
            err,
            CircuitError::InvalidParameterValue(0, value) if value.is_nan()
        ));
    }

    #[test]
    fn rejects_three_qubit_unitary_gate() {
        let gate = UnitaryGate::new("custom_3q", 3, 0)
            .with_matrix(Array2::eye(8))
            .unwrap();
        let mut circuit = Circuit::new(3);
        circuit
            .unitary(gate, vec![Qubit::new(0), Qubit::new(1), Qubit::new(2)])
            .unwrap();

        let err = decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap_err();
        assert!(err.to_string().contains("not supported yet"));
    }
}
