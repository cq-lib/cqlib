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

//! Shared infrastructure for classical-safe circuit rebuilds.
//!
//! # The problem
//!
//! Compile passes that rebuild a [`Circuit`] must not create an empty target
//! circuit with pre-copied classical tables and then append operations
//! incrementally. That pattern breaks three invariants of the runtime classical
//! IR:
//!
//! 1. Every entry in the `classical_values` table must be defined by a
//!    `MeasureBit` / `MeasureBits` operation in the circuit operation stream.
//! 2. Every [`ClassicalVar`] and [`ClassicalValue`] handle must belong to the
//!    circuit that contains it (`circuit_id` must match).
//! 3. Definition and control-flow bodies carry their own classical state with
//!    instance boundaries — expanding the same definition twice must produce
//!    two independent sets of handles.
//!
//! The most common symptom of a violation is
//! `CircuitError::UndefinedClassicalValue`: the value table declares a value
//! but no defining measurement exists in the operation stream.
//!
//! # The fix
//!
//! All rebuild passes follow the same pattern:
//!
//! 1. Accept storage-level [`Operation`]s from the source circuit.
//! 2. Produce a flat list of [`ValueOperation`]s, remapping every classical
//!    handle through a [`ClassicalRemap`].
//! 3. Call [`Circuit::from_operations`] exactly once via
//!    [`CircuitRebuildContext::finish`], which owns parameter interning and
//!    validation.
//!
//! # Quick start
//!
//! ```ignore
//! use crate::compile::transform::rebuild::CircuitRebuildContext;
//!
//! let rebuild = CircuitRebuildContext::new(&source);
//! let root_remap = rebuild.root_classical();
//! let mut operations = Vec::new();
//!
//! for op in source.operations() {
//!     if needs_transform(op) {
//!         // produce new ValueOperations with handles from root_remap
//!     } else {
//!         operations.push(rebuild.remap_preserved_operation(
//!             &source, op, root_remap,
//!         )?);
//!     }
//! }
//!
//! let circuit = rebuild.finish(source.qubits(), operations, source.global_phase())?;
//! ```

use crate::circuit::{
    Circuit, CircuitId, CircuitParam, ClassicalControlOp, ClassicalDataOp, ClassicalExpr,
    ClassicalType, ClassicalValue, ClassicalVar, Instruction, Operation, Parameter, ParameterValue,
    Qubit, ValueClassicalControlOp, ValueControlBody, ValueInstruction, ValueOperation,
    ValueSwitchCase,
};
use crate::compile::CompilerError;
use smallvec::SmallVec;
use std::collections::HashMap;

/// Mapping from source-circuit runtime classical handles to rebuilt handles.
///
/// Values of this type are produced by [`CircuitRebuildContext`]. The mapping
/// internals are intentionally private so callers cannot create partial remaps
/// that violate the rebuilt circuit's classical ownership invariants.
///
/// # Standalone use
///
/// A `ClassicalRemap` is self-contained once obtained from
/// [`CircuitRebuildContext::root_classical`] or
/// [`CircuitRebuildContext::allocate_classical_instance`]. All remap methods
/// on this type work without holding a reference to the rebuild context, so
/// callers can pass the remap into helper functions freely.
#[derive(Debug, Clone)]
pub struct ClassicalRemap {
    vars: HashMap<ClassicalVar, ClassicalVar>,
    values: HashMap<ClassicalValue, ClassicalValue>,
}

impl ClassicalRemap {
    /// Returns the rebuilt handle for `var`, if this remap covers it.
    pub fn map_var(&self, var: ClassicalVar) -> Option<ClassicalVar> {
        self.vars.get(&var).copied()
    }

    /// Returns the rebuilt handle for `value`, if this remap covers it.
    pub fn map_value(&self, value: ClassicalValue) -> Option<ClassicalValue> {
        self.values.get(&value).copied()
    }

    /// Remaps a classical variable, failing if this remap does not cover it.
    pub fn remap_var(&self, var: ClassicalVar) -> Result<ClassicalVar, CompilerError> {
        self.map_var(var).ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "missing classical variable remap for id {}",
                var.id()
            ))
        })
    }

    /// Remaps a classical value, failing if this remap does not cover it.
    pub fn remap_value(&self, value: ClassicalValue) -> Result<ClassicalValue, CompilerError> {
        self.map_value(value).ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "missing classical value remap for id {}",
                value.index()
            ))
        })
    }

    /// Remaps runtime classical variable and value reads inside an expression.
    ///
    /// Every [`ClassicalVar`] and [`ClassicalValue`] read by `expr` must be
    /// covered by this remap, otherwise an error is returned.
    pub fn remap_expr(&self, expr: &ClassicalExpr) -> Result<ClassicalExpr, CompilerError> {
        expr.remap_classical_ids(&self.vars, &self.values)
            .map_err(CompilerError::Circuit)
    }

    /// Remaps classical handles inside a runtime classical data operation.
    ///
    /// Covers [`ClassicalDataOp::Store`] (target variable and value
    /// expression), [`ClassicalDataOp::MeasureBit`], and
    /// [`ClassicalDataOp::MeasureBits`] (result value).
    pub fn remap_data_op(&self, op: &ClassicalDataOp) -> Result<ClassicalDataOp, CompilerError> {
        Ok(match op {
            ClassicalDataOp::Store { target, value } => ClassicalDataOp::Store {
                target: self.remap_var(*target)?,
                value: self.remap_expr(value)?,
            },
            ClassicalDataOp::MeasureBit { result } => ClassicalDataOp::MeasureBit {
                result: self.remap_value(*result)?,
            },
            ClassicalDataOp::MeasureBits { result } => ClassicalDataOp::MeasureBits {
                result: self.remap_value(*result)?,
            },
        })
    }
}

/// Context for rebuilding a circuit through value-level operations.
///
/// The context owns the new circuit identity and the accumulated classical
/// variable / value tables that will be supplied to
/// [`Circuit::from_operations`] in [`finish`](Self::finish).
///
/// # Lifecycle
///
/// ```ignore
/// let rebuild = CircuitRebuildContext::new(&source);
/// let root_remap = rebuild.root_classical();
///
/// // For definition expansion, allocate per-instance remaps:
/// let instance_remap = rebuild.allocate_classical_instance(&definition);
///
/// // Produce ValueOperations, then commit:
/// let circuit = rebuild.finish(qubits, operations, global_phase)?;
/// ```
///
/// # Design
///
/// `CircuitRebuildContext` delegates per-handle and per-expression remapping
/// to [`ClassicalRemap`] so that callers can pass the remap into helpers
/// without threading the full context. The context itself handles higher-level
/// concerns: accumulating classical tables across `allocate_classical_instance`
/// calls, resolving source parameters, and constructing the final `Circuit`.
#[derive(Debug, Clone)]
pub struct CircuitRebuildContext {
    target_circuit_id: CircuitId,
    classical_vars: Vec<ClassicalType>,
    classical_values: Vec<ClassicalType>,
    root_classical: ClassicalRemap,
}

impl CircuitRebuildContext {
    /// Creates a rebuild context for a new circuit with the same root-level
    /// classical tables as `source`.
    ///
    /// The root [`ClassicalRemap`] maps every classical handle in `source` to
    /// a corresponding handle with the new `circuit_id`. Access it via
    /// [`root_classical`](Self::root_classical).
    pub fn new(source: &Circuit) -> Self {
        let target_circuit_id = CircuitId::new();
        let classical_vars = source.classical_vars().to_vec();
        let classical_values = source.classical_values().to_vec();

        let vars = classical_vars
            .iter()
            .copied()
            .enumerate()
            .map(|(index, ty)| {
                (
                    ClassicalVar::new(source.id(), index as u32, ty),
                    ClassicalVar::new(target_circuit_id, index as u32, ty),
                )
            })
            .collect();
        let values = classical_values
            .iter()
            .copied()
            .enumerate()
            .map(|(index, ty)| {
                (
                    ClassicalValue::new(source.id(), index as u32, ty),
                    ClassicalValue::new(target_circuit_id, index as u32, ty),
                )
            })
            .collect();

        Self {
            target_circuit_id,
            classical_vars,
            classical_values,
            root_classical: ClassicalRemap { vars, values },
        }
    }

    /// Returns the remap for root-level classical handles from the source
    /// circuit.
    ///
    /// Use this remap when preserving operations from the top-level operation
    /// stream. For definition bodies or other nested circuits, use
    /// [`allocate_classical_instance`](Self::allocate_classical_instance)
    /// instead.
    pub fn root_classical(&self) -> &ClassicalRemap {
        &self.root_classical
    }

    /// Allocates a fresh copy of `circuit`'s classical tables in the rebuilt
    /// circuit and returns the corresponding handle remap.
    ///
    /// This is intended for definition expansion and other transforms that may
    /// instantiate the same source circuit more than once. Each call appends
    /// new entries to the accumulated classical tables and returns a
    /// [`ClassicalRemap`] with distinct rebuilt handles, so two expansions of
    /// the same definition never share classical state.
    pub fn allocate_classical_instance(&mut self, circuit: &Circuit) -> ClassicalRemap {
        let var_base = self.classical_vars.len();
        let mut vars = HashMap::with_capacity(circuit.classical_vars().len());
        for (index, ty) in circuit.classical_vars().iter().copied().enumerate() {
            self.classical_vars.push(ty);
            vars.insert(
                ClassicalVar::new(circuit.id(), index as u32, ty),
                ClassicalVar::new(self.target_circuit_id, (var_base + index) as u32, ty),
            );
        }

        let value_base = self.classical_values.len();
        let mut values = HashMap::with_capacity(circuit.classical_values().len());
        for (index, ty) in circuit.classical_values().iter().copied().enumerate() {
            self.classical_values.push(ty);
            values.insert(
                ClassicalValue::new(circuit.id(), index as u32, ty),
                ClassicalValue::new(self.target_circuit_id, (value_base + index) as u32, ty),
            );
        }

        ClassicalRemap { vars, values }
    }

    /// Builds and validates the final [`Circuit`] from rebuilt value-level
    /// operations.
    ///
    /// This is the only entry point for constructing the output circuit. It
    /// calls [`Circuit::from_operations`] which infers the classical circuit
    /// id, interns parameters, and runs full validation. Returns an error if
    /// any classical handle in `operations` is inconsistent with the
    /// accumulated classical tables.
    pub fn finish(
        self,
        qubits: Vec<Qubit>,
        operations: Vec<ValueOperation>,
        global_phase: Parameter,
    ) -> Result<Circuit, CompilerError> {
        let mut circuit = Circuit::from_operations(
            qubits,
            operations,
            Some(self.classical_vars),
            Some(self.classical_values),
        )?;
        circuit.set_global_phase(global_phase);
        Ok(circuit)
    }

    /// Resolves a storage-level parameter from `source` into a value-level
    /// parameter suitable for a rebuilt operation.
    ///
    /// [`CircuitParam::Index`] is resolved against `source`'s parameter table.
    /// [`CircuitParam::Fixed`] values are constant-folded when possible.
    pub fn resolve_source_param(
        source: &Circuit,
        param: &CircuitParam,
    ) -> Result<ParameterValue, CompilerError> {
        Ok(ParameterValue::from(source.resolve_parameter(param)?))
    }

    /// Resolves storage-level parameters from `source` into value-level
    /// parameters suitable for a rebuilt operation.
    pub fn resolve_source_params(
        source: &Circuit,
        params: &[CircuitParam],
    ) -> Result<SmallVec<[ParameterValue; 1]>, CompilerError> {
        params
            .iter()
            .map(|param| Self::resolve_source_param(source, param))
            .collect()
    }

    /// Converts an unchanged source operation into a rebuilt value-level
    /// operation, resolving parameters and remapping runtime classical handles.
    ///
    /// This is the primary entry point for preserved (non-transformed)
    /// operations. It calls [`remap_instruction`](Self::remap_instruction)
    /// internally, so classical-control operations are recursively remapped.
    /// Passes that apply their own transform to control-flow bodies should
    /// use the lower-level remap methods instead.
    pub fn remap_preserved_operation(
        &self,
        source: &Circuit,
        operation: &Operation,
        classical_remap: &ClassicalRemap,
    ) -> Result<ValueOperation, CompilerError> {
        Ok(ValueOperation {
            instruction: self.remap_instruction(source, &operation.instruction, classical_remap)?,
            qubits: operation.qubits.clone(),
            params: Self::resolve_source_params(source, &operation.params)?,
            label: operation.label.clone(),
        })
    }

    /// Converts an instruction into its value-level rebuilt form.
    ///
    /// Classical data operations are remapped via
    /// [`ClassicalRemap::remap_data_op`]. Classical control is recursively
    /// remapped via [`remap_control_flow`](Self::remap_control_flow). Other
    /// instruction kinds are cloned unchanged.
    ///
    /// Passes that apply their own transform to control-flow bodies should use
    /// [`remap_non_control_instruction`](Self::remap_non_control_instruction)
    /// instead, which rejects classical control to prevent accidental
    /// double-processing.
    pub fn remap_instruction(
        &self,
        source: &Circuit,
        instruction: &Instruction,
        classical_remap: &ClassicalRemap,
    ) -> Result<ValueInstruction, CompilerError> {
        match instruction {
            Instruction::ClassicalControl(control) => Ok(ValueInstruction::ClassicalControl(
                self.remap_control_flow(source, control, classical_remap)?,
            )),
            Instruction::ClassicalData(op) => Ok(ValueInstruction::from_instruction(
                Instruction::ClassicalData(classical_remap.remap_data_op(op)?),
            )),
            _ => Ok(ValueInstruction::from_instruction(instruction.clone())),
        }
    }

    /// Converts a non-control instruction into its value-level rebuilt form.
    ///
    /// This helper rejects structured classical control so transforms that
    /// rewrite control-flow bodies themselves cannot accidentally preserve an
    /// unprocessed nested control operation. Use
    /// [`remap_instruction`](Self::remap_instruction) when the transform does
    /// not recurse into control flow.
    pub fn remap_non_control_instruction(
        &self,
        instruction: &Instruction,
        classical_remap: &ClassicalRemap,
    ) -> Result<ValueInstruction, CompilerError> {
        match instruction {
            Instruction::ClassicalControl(_) => Err(CompilerError::InvariantViolation(
                "classical control must be rebuilt by the transform".to_string(),
            )),
            Instruction::ClassicalData(op) => Ok(ValueInstruction::from_instruction(
                Instruction::ClassicalData(classical_remap.remap_data_op(op)?),
            )),
            _ => Ok(ValueInstruction::from_instruction(instruction.clone())),
        }
    }

    /// Recursively remaps a classical control operation and its bodies.
    ///
    /// Conditions, loop variables, and switch targets are remapped through
    /// `classical_remap`. Every operation inside each control-flow body is
    /// preserved and remapped via
    /// [`remap_preserved_sequence`](Self::remap_preserved_sequence).
    ///
    /// Passes that apply their own transform inside control-flow bodies (such
    /// as the knowledge rewriter or unitary decomposer with
    /// `recurse_control_flow`) should call [`remap_expr`](ClassicalRemap::remap_expr)
    /// and [`remap_var`](ClassicalRemap::remap_var) on the condition or loop
    /// variable, then process the bodies through their own transform pipeline
    /// instead of calling this method.
    pub fn remap_control_flow(
        &self,
        source: &Circuit,
        control: &ClassicalControlOp,
        classical_remap: &ClassicalRemap,
    ) -> Result<ValueClassicalControlOp, CompilerError> {
        Ok(match control {
            ClassicalControlOp::If(op) => ValueClassicalControlOp::If {
                condition: classical_remap.remap_expr(op.condition())?,
                then_body: ValueControlBody::new(self.remap_preserved_sequence(
                    source,
                    op.then_body().operations(),
                    classical_remap,
                )?),
                else_body: op
                    .else_body()
                    .map(|body| {
                        self.remap_preserved_sequence(source, body.operations(), classical_remap)
                            .map(ValueControlBody::new)
                    })
                    .transpose()?,
            },
            ClassicalControlOp::While(op) => ValueClassicalControlOp::While {
                condition: classical_remap.remap_expr(op.condition())?,
                body: ValueControlBody::new(self.remap_preserved_sequence(
                    source,
                    op.body().operations(),
                    classical_remap,
                )?),
            },
            ClassicalControlOp::For(op) => ValueClassicalControlOp::For {
                var: classical_remap.remap_var(op.var())?,
                start: classical_remap.remap_expr(op.start())?,
                stop: classical_remap.remap_expr(op.stop())?,
                step: classical_remap.remap_expr(op.step())?,
                body: ValueControlBody::new(self.remap_preserved_sequence(
                    source,
                    op.body().operations(),
                    classical_remap,
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
                            ValueControlBody::new(self.remap_preserved_sequence(
                                source,
                                case.body().operations(),
                                classical_remap,
                            )?),
                        ))
                    })
                    .collect::<Result<Vec<_>, CompilerError>>()?,
                default: op
                    .default()
                    .map(|body| {
                        self.remap_preserved_sequence(source, body.operations(), classical_remap)
                            .map(ValueControlBody::new)
                    })
                    .transpose()?,
            },
            ClassicalControlOp::Break => ValueClassicalControlOp::Break,
            ClassicalControlOp::Continue => ValueClassicalControlOp::Continue,
        })
    }

    /// Remaps a sequence of preserved operations.
    ///
    /// Each operation is individually remapped via
    /// [`remap_preserved_operation`](Self::remap_preserved_operation). This is
    /// a convenience for remapping control-flow bodies and other operation
    /// slices in bulk.
    pub fn remap_preserved_sequence(
        &self,
        source: &Circuit,
        operations: &[Operation],
        classical_remap: &ClassicalRemap,
    ) -> Result<Vec<ValueOperation>, CompilerError> {
        operations
            .iter()
            .map(|operation| self.remap_preserved_operation(source, operation, classical_remap))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::StandardGate;

    #[test]
    fn preserved_rebuild_remaps_runtime_classical_handles_recursively() {
        let mut source = Circuit::new(3);
        let flag = source.var(ClassicalType::Bool);
        let counter = source.var(ClassicalType::uint(2).unwrap());
        let selector = source.var(ClassicalType::uint(2).unwrap());
        let bits_out = source.var(ClassicalType::bit_vec(2).unwrap());
        let measured = source.measure(Qubit::new(0)).unwrap();
        let measured_bits = source.measure_bits([Qubit::new(1), Qubit::new(2)]).unwrap();
        source
            .store(flag, ClassicalExpr::bit_to_bool(measured.expr()).unwrap())
            .unwrap();
        source
            .if_(flag.expr(), |body| body.x(Qubit::new(0)))
            .unwrap();
        source
            .for_uint(
                counter,
                selector.expr(),
                ClassicalExpr::uint_literal(2, 3).unwrap(),
                ClassicalExpr::uint_literal(2, 1).unwrap(),
                |body, loop_value| {
                    body.store(bits_out, measured_bits.expr())?;
                    body.switch(loop_value, |cases| {
                        cases.value(1, |case_body| case_body.h(Qubit::new(1)))?;
                        cases.default(|case_body| case_body.z(Qubit::new(2)))?;
                        Ok(())
                    })
                },
            )
            .unwrap();

        let rebuild = CircuitRebuildContext::new(&source);
        assert!(rebuild.root_classical().map_var(flag).is_some());
        assert!(
            rebuild
                .root_classical()
                .map_value(measured.value())
                .is_some()
        );
        let operations = source
            .operations()
            .iter()
            .map(|operation| {
                rebuild.remap_preserved_operation(&source, operation, rebuild.root_classical())
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let rebuilt = rebuild
            .finish(source.qubits(), operations, source.global_phase())
            .unwrap();

        assert_ne!(source.id(), rebuilt.id());
        assert_eq!(rebuilt.classical_vars(), source.classical_vars());
        assert_eq!(rebuilt.classical_values(), source.classical_values());
        rebuilt.validate().unwrap();
        for operation in rebuilt.operations() {
            assert_operation_handles_belong_to(operation, rebuilt.id());
        }
    }

    #[test]
    fn allocate_classical_instance_produces_distinct_handles_for_same_source() {
        let mut definition = Circuit::new(1);
        let flag = definition.var(ClassicalType::Bool);
        definition.measure(Qubit::new(0)).unwrap();
        definition
            .if_(flag.expr(), |body| body.x(Qubit::new(0)))
            .unwrap();

        let source = definition.clone(); // treat definition as a standalone circuit
        let mut rebuild = CircuitRebuildContext::new(&source);

        let first = rebuild.allocate_classical_instance(&definition);
        let second = rebuild.allocate_classical_instance(&definition);

        // The two instances must have distinct handles for the same source var
        let source_var = ClassicalVar::new(definition.id(), 0, ClassicalType::Bool);
        let first_mapped = first.map_var(source_var).unwrap();
        let second_mapped = second.map_var(source_var).unwrap();
        assert_ne!(
            first_mapped, second_mapped,
            "two allocations of the same definition must produce independent handles"
        );

        // The source value (from measure) must also differ
        let source_value = ClassicalValue::new(definition.id(), 0, ClassicalType::Bit);
        let first_val = first.map_value(source_value).unwrap();
        let second_val = second.map_value(source_value).unwrap();
        assert_ne!(first_val, second_val);
    }

    #[test]
    fn remap_data_op_is_callable_on_standalone_remap() {
        let mut source = Circuit::new(1);
        let old_var = source.var(ClassicalType::Bool);
        let measured = source.measure(Qubit::new(0)).unwrap();
        let old_value = measured.value();
        let rebuild = CircuitRebuildContext::new(&source);
        let remap = rebuild.root_classical();
        let new_var = remap.map_var(old_var).unwrap();
        let new_value = remap.map_value(old_value).unwrap();

        let op = ClassicalDataOp::MeasureBit { result: old_value };
        let remapped = remap.remap_data_op(&op).unwrap();
        let ClassicalDataOp::MeasureBit { result } = remapped else {
            panic!("expected MeasureBit");
        };
        assert_eq!(result, new_value);

        let op = ClassicalDataOp::Store {
            target: old_var,
            value: old_value.expr(),
        };
        let remapped = remap.remap_data_op(&op).unwrap();
        match remapped {
            ClassicalDataOp::Store { target, value } => {
                assert_eq!(target, new_var);
                assert_eq!(value, new_value.expr());
            }
            _ => panic!("expected Store"),
        }
    }

    fn assert_operation_handles_belong_to(operation: &Operation, circuit_id: CircuitId) {
        match &operation.instruction {
            Instruction::ClassicalData(op) => {
                assert_classical_data_handles_belong_to(op, circuit_id)
            }
            Instruction::ClassicalControl(op) => assert_control_handles_belong_to(op, circuit_id),
            Instruction::Standard(StandardGate::X | StandardGate::H | StandardGate::Z) => {}
            _ => {}
        }
    }

    fn assert_classical_data_handles_belong_to(op: &ClassicalDataOp, circuit_id: CircuitId) {
        match op {
            ClassicalDataOp::Store { target, value } => {
                assert_eq!(target.circuit_id(), circuit_id);
                assert_expr_handles_belong_to(value, circuit_id);
            }
            ClassicalDataOp::MeasureBit { result } | ClassicalDataOp::MeasureBits { result } => {
                assert_eq!(result.circuit_id(), circuit_id);
            }
        }
    }

    fn assert_control_handles_belong_to(op: &ClassicalControlOp, circuit_id: CircuitId) {
        match op {
            ClassicalControlOp::If(op) => {
                assert_expr_handles_belong_to(op.condition(), circuit_id);
                assert_body_handles_belong_to(op.then_body().operations(), circuit_id);
                if let Some(body) = op.else_body() {
                    assert_body_handles_belong_to(body.operations(), circuit_id);
                }
            }
            ClassicalControlOp::While(op) => {
                assert_expr_handles_belong_to(op.condition(), circuit_id);
                assert_body_handles_belong_to(op.body().operations(), circuit_id);
            }
            ClassicalControlOp::For(op) => {
                assert_eq!(op.var().circuit_id(), circuit_id);
                assert_expr_handles_belong_to(op.start(), circuit_id);
                assert_expr_handles_belong_to(op.stop(), circuit_id);
                assert_expr_handles_belong_to(op.step(), circuit_id);
                assert_body_handles_belong_to(op.body().operations(), circuit_id);
            }
            ClassicalControlOp::Switch(op) => {
                assert_expr_handles_belong_to(op.target(), circuit_id);
                for case in op.cases() {
                    assert_body_handles_belong_to(case.body().operations(), circuit_id);
                }
                if let Some(body) = op.default() {
                    assert_body_handles_belong_to(body.operations(), circuit_id);
                }
            }
            ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
        }
    }

    fn assert_body_handles_belong_to(operations: &[Operation], circuit_id: CircuitId) {
        for operation in operations {
            assert_operation_handles_belong_to(operation, circuit_id);
        }
    }

    fn assert_expr_handles_belong_to(expr: &ClassicalExpr, circuit_id: CircuitId) {
        for var in expr.vars() {
            assert_eq!(var.circuit_id(), circuit_id);
        }
        for value in expr.values() {
            assert_eq!(value.circuit_id(), circuit_id);
        }
    }
}
