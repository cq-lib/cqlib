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

//! Definition-based decomposition.
//!
//! This module expands operations whose implementation is already represented
//! by another circuit. It intentionally does not synthesize matrix-only
//! unitaries or lower standard gates to a target basis.

use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, Instruction, Operation, Parameter, ParameterValue,
    Qubit, StandardGate, ValueClassicalControlOp, ValueControlBody, ValueInstruction,
    ValueOperation, ValueSwitchCase,
};
use crate::compile::CompilerError;
use crate::compile::transform::analysis::WorkflowCircuitAnalysis;
use crate::compile::transform::rebuild::{CircuitRebuildContext, ClassicalRemap};
use crate::compile::transform::transformer::{PassApplicability, WorkflowPass};
use crate::compile::transform::{CircuitAnalysis, TransformOutcome, Transformer};
use smallvec::{SmallVec, smallvec};
use std::collections::{HashMap, HashSet};

const MAX_DEFINITION_DEPTH: usize = 64;

/// [`Transformer`] adapter for [`expand_definitions`].
///
/// Parameter-free — definition expansion never needs configuration.
#[derive(Debug, Clone, Copy, Default)]
pub struct DecomposeDefinitions;

impl WorkflowPass for DecomposeDefinitions {
    fn applicability(&self, analysis: &WorkflowCircuitAnalysis) -> PassApplicability {
        if analysis.has_unexpanded_definitions() {
            PassApplicability::Run
        } else {
            PassApplicability::ProvenNoOp("circuit contains no unexpanded gate definitions")
        }
    }
}

impl Transformer for DecomposeDefinitions {
    fn name(&self) -> &'static str {
        "decompose_definitions"
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
        if !analysis.has_circuit_gate_definitions && !analysis.has_unitary_circuit_definitions {
            return Ok(TransformOutcome::Unchanged);
        }
        expand_definitions_transform(circuit)
    }
}

/// Expands all circuit-backed definitions in `circuit`.
///
/// The expansion handles [`Instruction::CircuitGate`] and circuit-backed
/// [`Instruction::UnitaryGate`] operations. Matrix-only unitary gates are left
/// unchanged for a later synthesis stage.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::circuit::{Circuit, Instruction, Qubit, StandardGate};
/// use cqlib_core::compile::transform::decompose::expand_definitions;
///
/// let mut definition = Circuit::new(1);
/// definition.h(Qubit::new(0)).unwrap();
/// let h_definition = definition.to_gate("h_definition").unwrap();
///
/// let mut circuit = Circuit::new(1);
/// circuit
///     .append(h_definition, [Qubit::new(0)], [], None)
///     .unwrap();
///
/// let expanded = expand_definitions(&circuit).unwrap();
/// assert!(matches!(
///     expanded.operations()[0].instruction,
///     Instruction::Standard(StandardGate::H),
/// ));
/// ```
pub fn expand_definitions(circuit: &Circuit) -> Result<Circuit, CompilerError> {
    Ok(expand_definitions_transform(circuit)?.into_circuit(circuit))
}

fn expand_definitions_transform(circuit: &Circuit) -> Result<TransformOutcome, CompilerError> {
    DefinitionExpander::new(circuit)?.run()
}

struct DefinitionExpander<'a> {
    source: &'a Circuit,
    rebuild: CircuitRebuildContext,
    top_phase: Parameter,
    changed: bool,
}

impl<'a> DefinitionExpander<'a> {
    fn new(source: &'a Circuit) -> Result<Self, CompilerError> {
        let rebuild = CircuitRebuildContext::new(source);
        Ok(Self {
            source,
            rebuild,
            top_phase: source.global_phase(),
            changed: false,
        })
    }

    fn run(mut self) -> Result<TransformOutcome, CompilerError> {
        let qubit_map = QubitMap::Identity;
        let symbol_bindings = HashMap::new();
        let root_classical = self.rebuild.root_classical().clone();
        let mut operations = Vec::with_capacity(self.source.operations().len());

        for operation in self.source.operations() {
            let phase = self.expand_operation(
                operation,
                self.source,
                &qubit_map,
                &root_classical,
                &symbol_bindings,
                0,
                &mut operations,
            )?;
            self.top_phase = self.top_phase.clone() + phase;
        }

        let target = self
            .rebuild
            .finish(self.source.qubits(), operations, self.top_phase)?;
        Ok(if self.changed {
            TransformOutcome::Changed(target)
        } else {
            TransformOutcome::Unchanged
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn expand_operation(
        &mut self,
        operation: &Operation,
        context: &Circuit,
        qubit_map: &QubitMap<'_>,
        classical_remap: &ClassicalRemap,
        symbol_bindings: &HashMap<String, Parameter>,
        depth: usize,
        operations: &mut Vec<ValueOperation>,
    ) -> Result<Parameter, CompilerError> {
        match &operation.instruction {
            Instruction::CircuitGate(gate) => {
                self.changed = true;
                let definition = gate.circuit();
                self.expand_definition(
                    gate.name(),
                    definition.circuit(),
                    gate.num_qubits(),
                    gate.num_params(),
                    operation,
                    context,
                    qubit_map,
                    classical_remap,
                    symbol_bindings,
                    depth,
                    operations,
                )
            }
            Instruction::UnitaryGate(gate) => {
                if let Some(definition) = gate.circuit().as_ref() {
                    self.changed = true;
                    self.expand_definition(
                        gate.label(),
                        definition.circuit(),
                        gate.num_qubits() as usize,
                        gate.num_params() as usize,
                        operation,
                        context,
                        qubit_map,
                        classical_remap,
                        symbol_bindings,
                        depth,
                        operations,
                    )
                } else {
                    self.keep_operation(
                        operation,
                        context,
                        qubit_map,
                        classical_remap,
                        symbol_bindings,
                        operations,
                    )
                }
            }
            Instruction::ClassicalControl(control) => self.expand_control_flow(
                operation,
                control,
                context,
                qubit_map,
                classical_remap,
                symbol_bindings,
                depth,
                operations,
            ),
            _ => self.keep_operation(
                operation,
                context,
                qubit_map,
                classical_remap,
                symbol_bindings,
                operations,
            ),
        }
    }

    // Definition expansion carries the recursive source context, qubit map,
    // symbol bindings, depth counter, and output buffer together. Grouping
    // them into a one-off context type would hide the dataflow without reducing
    // the actual contract of this recursive call.
    #[allow(clippy::too_many_arguments)]
    fn expand_definition(
        &mut self,
        name: &str,
        definition: &Circuit,
        expected_qubits: usize,
        expected_params: usize,
        operation: &Operation,
        context: &Circuit,
        qubit_map: &QubitMap<'_>,
        _classical_remap: &ClassicalRemap,
        symbol_bindings: &HashMap<String, Parameter>,
        depth: usize,
        operations: &mut Vec<ValueOperation>,
    ) -> Result<Parameter, CompilerError> {
        if depth >= MAX_DEFINITION_DEPTH {
            return Err(CompilerError::InvalidInput(format!(
                "definition expansion exceeded maximum recursion depth {MAX_DEFINITION_DEPTH} while expanding '{name}'"
            )));
        }

        if operation.qubits.len() != expected_qubits {
            return Err(CompilerError::InvalidInput(format!(
                "definition '{name}' expects {expected_qubits} qubits, got {}",
                operation.qubits.len()
            )));
        }
        if operation.params.len() != expected_params {
            return Err(CompilerError::InvalidInput(format!(
                "definition '{name}' expects {expected_params} parameters, got {}",
                operation.params.len()
            )));
        }

        let definition_symbols = definition.symbols();
        if definition_symbols.len() != expected_params {
            return Err(CompilerError::InvariantViolation(format!(
                "definition '{name}' signature has {expected_params} parameters but backing circuit has {} symbols",
                definition_symbols.len()
            )));
        }

        let resolved_params = self.resolve_params(context, &operation.params, symbol_bindings)?;

        // Each definition owns its formal-symbol scope.  The caller arguments
        // have already been resolved through the current scope above, so the
        // nested definition must receive a fresh binding table instead of a
        // merged parent table. Merging would make equal symbol names in nested
        // definitions accidentally capture outer bindings.
        let mut next_symbol_bindings = HashMap::with_capacity(definition_symbols.len());
        for (symbol, value) in definition_symbols.iter().zip(resolved_params) {
            next_symbol_bindings.insert(symbol.clone(), value);
        }

        let definition_qubits = definition.qubits();
        if definition_qubits.len() != expected_qubits {
            return Err(CompilerError::InvariantViolation(format!(
                "definition '{name}' signature has {expected_qubits} qubits but backing circuit has {} qubits",
                definition_qubits.len()
            )));
        }

        let mut next_qubit_map = HashMap::with_capacity(definition_qubits.len());
        for (inner, callsite) in definition_qubits.iter().zip(operation.qubits.iter()) {
            next_qubit_map.insert(*inner, qubit_map.map_one(*callsite)?);
        }
        let next_qubit_map = QubitMap::Mapped(&next_qubit_map);
        let next_classical_remap = self.rebuild.allocate_classical_instance(definition);

        let mut phase = apply_symbol_bindings(definition.global_phase(), &next_symbol_bindings);
        for inner_operation in definition.operations() {
            let inner_phase = self.expand_operation(
                inner_operation,
                definition,
                &next_qubit_map,
                &next_classical_remap,
                &next_symbol_bindings,
                depth + 1,
                operations,
            )?;
            phase = phase + inner_phase;
        }

        Ok(phase)
    }

    // Control-flow expansion carries the recursive source context, qubit map,
    // symbol bindings, depth counter, and output buffer together.
    #[allow(clippy::too_many_arguments)]
    fn expand_control_flow(
        &mut self,
        operation: &Operation,
        control: &ClassicalControlOp,
        context: &Circuit,
        qubit_map: &QubitMap<'_>,
        classical_remap: &ClassicalRemap,
        symbol_bindings: &HashMap<String, Parameter>,
        depth: usize,
        operations: &mut Vec<ValueOperation>,
    ) -> Result<Parameter, CompilerError> {
        let instruction = match control {
            ClassicalControlOp::If(op) => ValueClassicalControlOp::If {
                condition: classical_remap.remap_expr(op.condition())?,
                then_body: ValueControlBody::new(self.expand_body(
                    op.then_body().operations(),
                    context,
                    qubit_map,
                    classical_remap,
                    symbol_bindings,
                    depth,
                )?),
                else_body: op
                    .else_body()
                    .map(|body| {
                        self.expand_body(
                            body.operations(),
                            context,
                            qubit_map,
                            classical_remap,
                            symbol_bindings,
                            depth,
                        )
                        .map(ValueControlBody::new)
                    })
                    .transpose()?,
            },
            ClassicalControlOp::While(op) => ValueClassicalControlOp::While {
                condition: classical_remap.remap_expr(op.condition())?,
                body: ValueControlBody::new(self.expand_body(
                    op.body().operations(),
                    context,
                    qubit_map,
                    classical_remap,
                    symbol_bindings,
                    depth,
                )?),
            },
            ClassicalControlOp::For(op) => ValueClassicalControlOp::For {
                var: classical_remap.remap_var(op.var())?,
                start: classical_remap.remap_expr(op.start())?,
                stop: classical_remap.remap_expr(op.stop())?,
                step: classical_remap.remap_expr(op.step())?,
                body: ValueControlBody::new(self.expand_body(
                    op.body().operations(),
                    context,
                    qubit_map,
                    classical_remap,
                    symbol_bindings,
                    depth,
                )?),
            },
            ClassicalControlOp::Switch(op) => ValueClassicalControlOp::Switch {
                target: classical_remap.remap_expr(op.target())?,
                cases: op
                    .cases()
                    .iter()
                    .map(|case| {
                        Ok(ValueSwitchCase::new(
                            case.value(),
                            ValueControlBody::new(self.expand_body(
                                case.body().operations(),
                                context,
                                qubit_map,
                                classical_remap,
                                symbol_bindings,
                                depth,
                            )?),
                        ))
                    })
                    .collect::<Result<Vec<_>, CompilerError>>()?,
                default: op
                    .default()
                    .map(|body| {
                        self.expand_body(
                            body.operations(),
                            context,
                            qubit_map,
                            classical_remap,
                            symbol_bindings,
                            depth,
                        )
                        .map(ValueControlBody::new)
                    })
                    .transpose()?,
            },
            ClassicalControlOp::Break => ValueClassicalControlOp::Break,
            ClassicalControlOp::Continue => ValueClassicalControlOp::Continue,
        };

        operations.push(ValueOperation {
            instruction: ValueInstruction::ClassicalControl(instruction),
            qubits: qubit_map.map_all(&operation.qubits)?,
            params: smallvec![],
            label: operation.label.clone(),
        });
        Ok(Parameter::from(0.0))
    }

    fn expand_body(
        &mut self,
        body: &[Operation],
        context: &Circuit,
        qubit_map: &QubitMap<'_>,
        classical_remap: &ClassicalRemap,
        symbol_bindings: &HashMap<String, Parameter>,
        depth: usize,
    ) -> Result<Vec<ValueOperation>, CompilerError> {
        let mut operations = Vec::with_capacity(body.len());
        let mut phase = Parameter::from(0.0);

        for operation in body {
            let operation_phase = self.expand_operation(
                operation,
                context,
                qubit_map,
                classical_remap,
                symbol_bindings,
                depth,
                &mut operations,
            )?;
            phase = phase + operation_phase;
        }

        if !phase.is_zero() {
            operations.insert(
                0,
                ValueOperation {
                    instruction: ValueInstruction::from_instruction(Instruction::Standard(
                        StandardGate::GPhase,
                    )),
                    qubits: smallvec![],
                    params: smallvec![ParameterValue::from(phase)],
                    label: None,
                },
            );
        }

        Ok(operations)
    }

    fn keep_operation(
        &mut self,
        operation: &Operation,
        context: &Circuit,
        qubit_map: &QubitMap<'_>,
        classical_remap: &ClassicalRemap,
        symbol_bindings: &HashMap<String, Parameter>,
        operations: &mut Vec<ValueOperation>,
    ) -> Result<Parameter, CompilerError> {
        let params = self
            .resolve_params(context, &operation.params, symbol_bindings)?
            .into_iter()
            .map(ParameterValue::from)
            .collect::<SmallVec<[ParameterValue; 1]>>();

        operations.push(ValueOperation {
            instruction: self
                .rebuild
                .remap_non_control_instruction(&operation.instruction, classical_remap)?,
            qubits: qubit_map.map_all(&operation.qubits)?,
            params,
            label: operation.label.clone(),
        });
        Ok(Parameter::from(0.0))
    }

    fn resolve_params(
        &self,
        context: &Circuit,
        params: &[CircuitParam],
        symbol_bindings: &HashMap<String, Parameter>,
    ) -> Result<Vec<Parameter>, CompilerError> {
        params
            .iter()
            .map(|param| {
                let resolved = context.resolve_parameter(param)?;
                Ok(apply_symbol_bindings(resolved, symbol_bindings))
            })
            .collect()
    }
}

enum QubitMap<'a> {
    Identity,
    Mapped(&'a HashMap<Qubit, Qubit>),
}

impl QubitMap<'_> {
    fn map_all(&self, qubits: &[Qubit]) -> Result<SmallVec<[Qubit; 3]>, CompilerError> {
        if matches!(self, Self::Identity) {
            return Ok(qubits.iter().copied().collect());
        }

        let mut mapped = SmallVec::with_capacity(qubits.len());
        for qubit in qubits {
            mapped.push(self.map_one(*qubit)?);
        }
        Ok(mapped)
    }

    fn map_one(&self, qubit: Qubit) -> Result<Qubit, CompilerError> {
        match self {
            Self::Identity => Ok(qubit),
            Self::Mapped(qubit_map) => qubit_map.get(&qubit).copied().ok_or_else(|| {
                CompilerError::InvalidInput(format!(
                    "definition expansion references unmapped qubit {qubit}"
                ))
            }),
        }
    }
}

fn apply_symbol_bindings(mut parameter: Parameter, map: &HashMap<String, Parameter>) -> Parameter {
    if map.is_empty() {
        return parameter;
    }

    if !binding_values_reference_bound_symbols(map) {
        for (symbol, value) in map {
            parameter = parameter.replace(symbol, value.clone());
        }
        return parameter;
    }

    let mut occupied = parameter.get_symbols();
    for (symbol, value) in map {
        occupied.insert(symbol.clone());
        occupied.extend(value.get_symbols());
    }

    let mut temp_map = Vec::with_capacity(map.len());
    for (index, (symbol, value)) in map.iter().enumerate() {
        let temp = fresh_temp_symbol(index, symbol, &mut occupied);
        parameter = parameter.replace(symbol, Parameter::symbol(&temp));
        temp_map.push((temp, value.clone()));
    }

    for (temp, value) in temp_map {
        parameter = parameter.replace(&temp, value);
    }

    parameter
}

fn binding_values_reference_bound_symbols(map: &HashMap<String, Parameter>) -> bool {
    map.values()
        .flat_map(Parameter::get_symbols)
        .any(|symbol| map.contains_key(&symbol))
}

fn fresh_temp_symbol(index: usize, symbol: &str, occupied: &mut HashSet<String>) -> String {
    let mut attempt = 0;
    loop {
        let candidate = format!("__CQLIB_DECOMPOSE_TMP_{index}_{attempt}_{symbol}");
        if occupied.insert(candidate.clone()) {
            return candidate;
        }
        attempt += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::ParameterValue;
    use crate::circuit::gate::{FrozenCircuit, UnitaryGate};
    use crate::circuit::{ClassicalDataOp, ClassicalExpr};
    use crate::compile::transform::TransformerTestExt;
    use ndarray::array;
    use num_complex::Complex;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn op_param(circuit: &Circuit, param: &CircuitParam) -> Parameter {
        match param {
            CircuitParam::Fixed(value) => Parameter::from(*value),
            CircuitParam::Index(index) => circuit.parameters()[*index as usize].clone(),
        }
    }

    #[test]
    fn transform_reports_unchanged_without_circuit_backed_definitions() {
        let mut circuit = Circuit::new(1);
        circuit.h(Qubit::new(0)).unwrap();

        let result = DecomposeDefinitions
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
    fn transform_keeps_measurement_circuit_without_definitions() {
        let mut circuit = Circuit::new(2);
        circuit
            .measure_bits([Qubit::new(0), Qubit::new(1)])
            .unwrap();

        let result = DecomposeDefinitions
            .transform_resolved(&circuit, None)
            .unwrap();

        assert!(!result.changed);
        assert_eq!(result.circuit.operations().len(), 1);
        assert_eq!(
            result.circuit.classical_values(),
            circuit.classical_values()
        );
        assert!(matches!(
            result.circuit.operations()[0].instruction,
            Instruction::ClassicalData(ClassicalDataOp::MeasureBits { .. })
        ));
    }

    #[test]
    fn expands_circuit_gate_with_qubit_and_parameter_mapping() {
        let mut inner = Circuit::new(2);
        let theta = Parameter::symbol("theta");
        let beta = Parameter::symbol("beta");
        inner.h(Qubit::new(0)).unwrap();
        inner.rx(Qubit::new(0), theta).unwrap();
        inner.rz(Qubit::new(1), beta + 1.0).unwrap();
        let gate = inner.to_gate("inner").unwrap();

        let mut outer = Circuit::new(2);
        let gamma = Parameter::symbol("gamma");
        let delta = Parameter::symbol("delta");
        outer
            .append(
                gate,
                [Qubit::new(1), Qubit::new(0)],
                [
                    ParameterValue::Param(gamma.clone()),
                    ParameterValue::Param(gamma + delta),
                ],
                None,
            )
            .unwrap();

        let expanded = expand_definitions(&outer).unwrap();
        let operations = expanded.operations();
        assert_eq!(operations.len(), 3);
        assert!(matches!(
            operations[0].instruction,
            Instruction::Standard(StandardGate::H)
        ));
        assert_eq!(operations[0].qubits.as_slice(), &[Qubit::new(1)]);
        assert!(matches!(
            operations[1].instruction,
            Instruction::Standard(StandardGate::RX)
        ));
        assert_eq!(operations[1].qubits.as_slice(), &[Qubit::new(1)]);
        assert!(matches!(
            operations[2].instruction,
            Instruction::Standard(StandardGate::RZ)
        ));
        assert_eq!(operations[2].qubits.as_slice(), &[Qubit::new(0)]);

        let rx_param = op_param(&expanded, &operations[1].params[0]);
        let rz_param = op_param(&expanded, &operations[2].params[0]);
        let bindings = HashMap::from([("gamma", 2.0), ("delta", 3.0)]);
        assert_eq!(rx_param.evaluate(&Some(bindings.clone())).unwrap(), 2.0);
        assert_eq!(rz_param.evaluate(&Some(bindings)).unwrap(), 6.0);
    }

    #[test]
    fn uses_simultaneous_parameter_substitution() {
        let mut inner = Circuit::new(1);
        inner.rx(Qubit::new(0), Parameter::symbol("a")).unwrap();
        inner.rz(Qubit::new(0), Parameter::symbol("b")).unwrap();
        let gate = inner.to_gate("swap_params").unwrap();

        let mut outer = Circuit::new(1);
        outer
            .append(
                gate,
                [Qubit::new(0)],
                [
                    ParameterValue::Param(Parameter::symbol("b")),
                    ParameterValue::Param(Parameter::symbol("a")),
                ],
                None,
            )
            .unwrap();

        let expanded = expand_definitions(&outer).unwrap();
        let operations = expanded.operations();
        let rx_param = op_param(&expanded, &operations[0].params[0]);
        let rz_param = op_param(&expanded, &operations[1].params[0]);
        assert_eq!(rx_param.as_symbol().as_deref(), Some("b"));
        assert_eq!(rz_param.as_symbol().as_deref(), Some("a"));
    }

    #[test]
    fn expands_nested_circuit_gates() {
        let mut leaf = Circuit::new(1);
        leaf.h(Qubit::new(0)).unwrap();
        let leaf_gate = leaf.to_gate("leaf").unwrap();

        let mut middle = Circuit::new(1);
        middle
            .rx(Qubit::new(0), Parameter::symbol("theta"))
            .unwrap();
        middle.append(leaf_gate, [Qubit::new(0)], [], None).unwrap();
        let middle_gate = middle.to_gate("middle").unwrap();

        let mut outer = Circuit::new(1);
        outer
            .append(
                middle_gate,
                [Qubit::new(0)],
                [ParameterValue::Param(Parameter::symbol("phi"))],
                None,
            )
            .unwrap();

        let expanded = expand_definitions(&outer).unwrap();
        let operations = expanded.operations();
        assert_eq!(operations.len(), 2);
        assert!(matches!(
            operations[0].instruction,
            Instruction::Standard(StandardGate::RX)
        ));
        assert!(matches!(
            operations[1].instruction,
            Instruction::Standard(StandardGate::H)
        ));
        assert_eq!(
            op_param(&expanded, &operations[0].params[0])
                .as_symbol()
                .as_deref(),
            Some("phi")
        );
    }

    #[test]
    fn resolves_nested_definition_symbol_scopes_without_capture() {
        let mut leaf = Circuit::new(1);
        leaf.rx(Qubit::new(0), Parameter::symbol("theta")).unwrap();
        let leaf_gate = leaf.to_gate("leaf").unwrap();

        let mut middle = Circuit::new(1);
        middle
            .append(
                leaf_gate,
                [Qubit::new(0)],
                [ParameterValue::Param(Parameter::symbol("theta") + 1.0)],
                None,
            )
            .unwrap();
        let middle_gate = middle.to_gate("middle").unwrap();

        let mut outer = Circuit::new(1);
        outer
            .append(
                middle_gate,
                [Qubit::new(0)],
                [ParameterValue::Param(Parameter::symbol("alpha"))],
                None,
            )
            .unwrap();

        let expanded = expand_definitions(&outer).unwrap();
        let operations = expanded.operations();
        assert_eq!(operations.len(), 1);
        assert!(matches!(
            operations[0].instruction,
            Instruction::Standard(StandardGate::RX)
        ));

        let param = op_param(&expanded, &operations[0].params[0]);
        let bindings = HashMap::from([("alpha", 2.0)]);
        assert_eq!(param.evaluate(&Some(bindings)).unwrap(), 3.0);
        assert!(param.get_symbols().contains("alpha"));
        assert!(!param.get_symbols().contains("theta"));
    }

    #[test]
    fn merges_definition_global_phase_at_top_level() {
        let mut inner = Circuit::new(1);
        inner.set_global_phase(Parameter::from(0.5));
        inner.x(Qubit::new(0)).unwrap();
        let gate = inner.to_gate("phase_x").unwrap();

        let mut outer = Circuit::new(1);
        outer.set_global_phase(Parameter::from(0.25));
        outer.append(gate, [Qubit::new(0)], [], None).unwrap();

        let expanded = expand_definitions(&outer).unwrap();
        assert_eq!(expanded.operations().len(), 1);
        assert!(matches!(
            expanded.operations()[0].instruction,
            Instruction::Standard(StandardGate::X)
        ));
        assert!((expanded.global_phase().evaluate(&None).unwrap() - 0.75).abs() < 1e-12);
    }

    #[test]
    fn expands_circuit_backed_unitary_gate() {
        let mut inner = Circuit::new(1);
        inner.rx(Qubit::new(0), Parameter::symbol("theta")).unwrap();
        let gate = UnitaryGate::new("unitary_rx", 1, 1)
            .with_circuit(Arc::new(FrozenCircuit::new(inner)))
            .unwrap();

        let mut outer = Circuit::new(1);
        outer
            .unitary_with_params(
                gate,
                vec![Qubit::new(0)],
                [ParameterValue::Param(Parameter::symbol("phi"))],
            )
            .unwrap();

        let expanded = expand_definitions(&outer).unwrap();
        let operations = expanded.operations();
        assert_eq!(operations.len(), 1);
        assert!(matches!(
            operations[0].instruction,
            Instruction::Standard(StandardGate::RX)
        ));
        assert_eq!(
            op_param(&expanded, &operations[0].params[0])
                .as_symbol()
                .as_deref(),
            Some("phi")
        );
    }

    #[test]
    fn keeps_matrix_only_unitary_gate() {
        let matrix = array![
            [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
            [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)]
        ];
        let gate = UnitaryGate::new("matrix_x", 1, 0)
            .with_matrix(matrix)
            .unwrap();

        let mut circuit = Circuit::new(1);
        circuit.unitary(gate, vec![Qubit::new(0)]).unwrap();

        let expanded = expand_definitions(&circuit).unwrap();
        assert_eq!(expanded.operations().len(), 1);
        assert!(matches!(
            expanded.operations()[0].instruction,
            Instruction::UnitaryGate(_)
        ));
    }

    #[test]
    fn expands_definitions_in_control_flow_body_with_body_local_phase() {
        let mut inner = Circuit::new(1);
        inner.set_global_phase(Parameter::from(0.5));
        inner.x(Qubit::new(0)).unwrap();
        let gate = inner.to_gate("phase_x").unwrap();

        let mut circuit = Circuit::new(2);
        circuit
            .if_else(
                ClassicalExpr::bool_literal(true),
                |body| body.append(gate, [Qubit::new(1)], [], None),
                |_body| Ok(()),
            )
            .unwrap();

        let expanded = expand_definitions(&circuit).unwrap();
        assert!(expanded.global_phase().is_zero());
        let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
            &expanded.operations()[0].instruction
        else {
            panic!("expected if operation");
        };
        let then_body = op.then_body().operations();
        assert_eq!(then_body.len(), 2);
        assert!(matches!(
            then_body[0].instruction,
            Instruction::Standard(StandardGate::GPhase)
        ));
        assert!(matches!(
            then_body[1].instruction,
            Instruction::Standard(StandardGate::X)
        ));
    }

    #[test]
    fn expands_definition_with_measurement_and_classical_control() {
        let mut inner = Circuit::new(1);
        let measured = inner.measure(Qubit::new(0)).unwrap();
        inner
            .if_(
                ClassicalExpr::bit_to_bool(measured.expr()).unwrap(),
                |body| body.x(Qubit::new(0)),
            )
            .unwrap();
        let gate = inner.to_gate("measure_then_x").unwrap();

        let mut outer = Circuit::new(1);
        outer.append(gate, [Qubit::new(0)], [], None).unwrap();

        let expanded = expand_definitions(&outer).unwrap();
        assert_eq!(expanded.classical_values().len(), 1);
        assert!(expanded.validate().is_ok());
        assert!(matches!(
            expanded.operations()[0].instruction,
            Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
        ));
        let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
            &expanded.operations()[1].instruction
        else {
            panic!("expected expanded if operation");
        };
        assert_eq!(op.then_body().operations().len(), 1);
        assert!(matches!(
            op.then_body().operations()[0].instruction,
            Instruction::Standard(StandardGate::X)
        ));
    }

    #[test]
    fn expands_multiple_definition_calls_with_distinct_classical_handles() {
        let mut inner = Circuit::new(1);
        let measured = inner.measure(Qubit::new(0)).unwrap();
        inner
            .if_(
                ClassicalExpr::bit_to_bool(measured.expr()).unwrap(),
                |body| body.x(Qubit::new(0)),
            )
            .unwrap();
        let gate = inner.to_gate("measure_then_x").unwrap();

        let mut outer = Circuit::new(2);
        outer
            .append(gate.clone(), [Qubit::new(0)], [], None)
            .unwrap();
        outer.append(gate, [Qubit::new(1)], [], None).unwrap();

        let expanded = expand_definitions(&outer).unwrap();
        assert_eq!(expanded.classical_values().len(), 2);
        assert!(expanded.validate().is_ok());

        let Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result: first }) =
            expanded.operations()[0].instruction
        else {
            panic!("expected first measurement");
        };
        let Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result: second }) =
            expanded.operations()[2].instruction
        else {
            panic!("expected second measurement");
        };

        assert_ne!(first.index(), second.index());
        let Instruction::ClassicalControl(ClassicalControlOp::If(first_if)) =
            &expanded.operations()[1].instruction
        else {
            panic!("expected first if");
        };
        let Instruction::ClassicalControl(ClassicalControlOp::If(second_if)) =
            &expanded.operations()[3].instruction
        else {
            panic!("expected second if");
        };
        assert!(first_if.condition().values().contains(&first));
        assert!(second_if.condition().values().contains(&second));
    }

    #[test]
    fn rejects_excessive_definition_depth() {
        let mut circuit = Circuit::new(1);
        circuit.x(Qubit::new(0)).unwrap();
        let mut gate = circuit.to_gate("level_0").unwrap();

        for index in 1..=MAX_DEFINITION_DEPTH {
            let mut next = Circuit::new(1);
            next.append(gate, [Qubit::new(0)], [], None).unwrap();
            gate = next.to_gate(format!("level_{index}")).unwrap();
        }

        let mut top = Circuit::new(1);
        top.append(gate, [Qubit::new(0)], [], None).unwrap();

        let err = expand_definitions(&top).unwrap_err();
        assert!(
            matches!(err, CompilerError::InvalidInput(message) if message.contains("maximum recursion depth"))
        );
    }
}
