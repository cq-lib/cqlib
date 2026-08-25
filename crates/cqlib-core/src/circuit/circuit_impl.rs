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

//! # Quantum Circuit Module
//!
//! This module defines the [`Circuit`] struct, which is the primary container for quantum programs
//! in the `Cqlib` ecosystem. It acts as an intermediate representation (IR) that captures the sequence
//! of quantum operations, qubit management, and symbolic parameters.
//!
//! ## Core Features
//!
//! - **Instruction Scheduling**: Stores a sequence of operations ([`Operation`]) including gates, measurements, and barriers.
//! - **Qubit Management**: Efficiently handles qubit allocation using topological ordering.
//! - **Parametric Circuits**: Native support for variational quantum algorithms (VQA) via symbolic parameters.
//!   Parameters are "interned" to minimize memory usage and accelerate bulk evaluation.
//! - **Extensibility**: Supports standard gates, custom unitary matrices, and arbitrary control structures.
//!
//! ## Example
//!
//! ```rust
//! use cqlib_core::circuit::circuit_impl::Circuit;
//! use cqlib_core::circuit::Qubit;
//!
//! // Create a circuit with 2 qubits
//! let mut circuit = Circuit::new(2);
//!
//! let q0 = Qubit::new(0);
//! let q1 = Qubit::new(1);
//!
//! // Apply Hadamard gate to q0
//! circuit.h(q0);
//!
//! // Apply Controlled-NOT gate (q0 controls q1)
//! circuit.cx(q0, q1);
//!
//! // Measure q0 and keep its immutable runtime result
//! let measured = circuit.measure(q0).unwrap();
//! assert_eq!(measured.ty().width(), 1);
//! ```

use crate::circuit::bit::Qubit;
use crate::circuit::circuit_classical::ControlScopeKind;
use crate::circuit::circuit_param::{CircuitParam, ParameterValue};
use crate::circuit::classical::CircuitId;
use crate::circuit::classical_expr::{ClassicalExpr, ClassicalExprKind};
use crate::circuit::error::CircuitError;
use crate::circuit::gate::circuit_gate::{CircuitGate, FrozenCircuit};
use crate::circuit::gate::instruction::Instruction;
use crate::circuit::gate::{Directive, StandardGate, UnitaryGate};
use crate::circuit::operation::{Operation, ValueOperation};
use crate::circuit::parameter::Parameter;
use crate::circuit::value_instruction::{
    ValueControlBody, ValueInstruction, storage_operation_to_value,
};
use crate::circuit::{
    ClassicalControlOp, ClassicalType, ClassicalValue, ClassicalVar, ControlBody, ForOp, IfOp,
    SwitchCase, SwitchOp, ValueClassicalControlOp, WhileOp,
};
use crate::circuit::{ClassicalDataOp, circuit_to_matrix};
use indexmap::IndexSet;
use ndarray::Array2;
use num_complex::Complex64;
use smallvec::{SmallVec, smallvec};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A quantum circuit representation serving as the core IR for quantum programs.
///
/// The `Circuit` struct is designed to be a high-performance, memory-efficient container for quantum
/// operations. It supports both static circuits (fixed angles) and parameterized circuits (symbolic angles),
/// making it suitable for a wide range of applications from error correction to variational quantum algorithms.
///
/// # Internal Architecture
///
/// - **Qubit Storage**: Uses `IndexSet<Qubit>` to maintain deterministic ordering of qubits while allowing $O(1)$ lookups.
/// - **Parameter Interning**: Symbolic parameters are stored in a centralized `IndexSet`. Instructions reference these parameters
///   by index rather than owning them. This "interning" strategy significantly reduces memory footprint for deep parameterized
///   circuits and enables vectorized parameter updates.
#[derive(Debug)]
pub struct Circuit {
    pub(super) circuit_id: CircuitId,
    /// The set of quantum bits (qubits) managed by this circuit.
    ///
    /// # Implementation Note
    /// Used `IndexSet` to maintain the strict insertion order of qubits (which defines the logical
    /// qubit indices 0, 1, 2...) while allowing $O(1)$ membership testing (`contains`).
    pub(super) qubits: IndexSet<Qubit>,
    /// A registry of all unique symbolic variables (e.g., "theta", "phi") used within the circuit.
    /// This field serves as a cache to quickly identify which free parameters need to be bound
    /// before simulation, avoiding the need to traverse the entire instruction list.
    pub(super) symbols: IndexSet<String>,
    /// The centralized storage for symbolic parameters.
    ///
    /// This table implements the **Interning** pattern. Instructions in the `data` vector do not
    /// own their `Parameter` objects; instead, they store lightweight indices pointing to this set.
    /// This design allows for:
    /// 1. **Deduplication**: Identical expressions are stored only once.
    /// 2. **Batch Evaluation**: All parameters can be resolved to `f64` values in a single linear pass.
    pub(super) parameters: IndexSet<Parameter>,
    /// The ordered sequence of operations (quantum gates, measurements, etc.) in the circuit.
    ///
    /// This vector represents the circuit schedule.
    pub(super) data: Arc<Vec<Operation>>,
    /// Static types of circuit-local runtime classical variables.
    ///
    /// A [`ClassicalVar`] ID is an index into this table. Keeping ownership in
    /// the circuit prevents expressions from silently referencing variables
    /// allocated by another circuit.
    pub(super) classical_vars: Vec<ClassicalType>,
    /// Static types of immutable circuit-local runtime classical values.
    ///
    /// A [`ClassicalValue`] ID is an index into this table. This verifies that
    /// measurement-produced values consumed by expressions belong to this circuit.
    pub(super) classical_values: Vec<ClassicalType>,
    /// Active structured-control scopes while closure bodies are being built.
    ///
    /// This transient builder stack validates placement of `break` and `continue`.
    pub(super) control_scope_stack: Vec<ControlScopeKind>,
    ///  The global phase of the circuit, representing a scalar factor $e^{i\theta}$.
    ///
    /// While the global phase is unobservable in isolated systems, it is critical for:
    /// - **Controlled Operations**: When this circuit is controlled by another qubit.
    /// - **Sub-circuit Composition**: Correctly merging phases when combining circuits.
    pub(super) global_phase: CircuitParam,
}

impl Clone for Circuit {
    fn clone(&self) -> Self {
        let circuit_id = CircuitId::new();
        if self.classical_vars.is_empty() && self.classical_values.is_empty() {
            return Self {
                circuit_id,
                qubits: self.qubits.clone(),
                symbols: self.symbols.clone(),
                parameters: self.parameters.clone(),
                data: self.data.clone(),
                classical_vars: Vec::new(),
                classical_values: Vec::new(),
                control_scope_stack: self.control_scope_stack.clone(),
                global_phase: self.global_phase.clone(),
            };
        }
        let var_map = self
            .classical_vars
            .iter()
            .enumerate()
            .map(|(index, ty)| {
                (
                    ClassicalVar::new(self.circuit_id, index as u32, *ty),
                    ClassicalVar::new(circuit_id, index as u32, *ty),
                )
            })
            .collect::<HashMap<_, _>>();
        let value_map = self
            .classical_values
            .iter()
            .enumerate()
            .map(|(index, ty)| {
                (
                    ClassicalValue::new(self.circuit_id, index as u32, *ty),
                    ClassicalValue::new(circuit_id, index as u32, *ty),
                )
            })
            .collect::<HashMap<_, _>>();
        let data = self
            .data
            .iter()
            .map(|operation| {
                Self::remap_operation(
                    operation,
                    &|qubit| Ok(qubit),
                    &|param| Ok(param.clone()),
                    &var_map,
                    &value_map,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .expect("a valid circuit must be cloneable with complete handle mappings");

        Self {
            circuit_id,
            qubits: self.qubits.clone(),
            symbols: self.symbols.clone(),
            parameters: self.parameters.clone(),
            data: Arc::new(data),
            classical_vars: self.classical_vars.clone(),
            classical_values: self.classical_values.clone(),
            control_scope_stack: self.control_scope_stack.clone(),
            global_phase: self.global_phase.clone(),
        }
    }
}

/// Compares exact compiler-IR structure while ignoring process-local circuit identity.
impl PartialEq for Circuit {
    fn eq(&self, other: &Self) -> bool {
        if self.qubits != other.qubits
            || self.symbols != other.symbols
            || self.parameters.len() != other.parameters.len()
            || !self
                .parameters
                .iter()
                .all(|parameter| other.parameters.contains(parameter))
            || self.classical_vars != other.classical_vars
            || self.classical_values != other.classical_values
            || self.control_scope_stack != other.control_scope_stack
            || self.data.len() != other.data.len()
        {
            return false;
        }
        circuit_params_equal(
            &self.global_phase,
            &other.global_phase,
            &self.parameters,
            &other.parameters,
        ) && operations_structurally_equal(
            self.data.as_slice(),
            other.data.as_slice(),
            &self.parameters,
            &other.parameters,
        )
    }
}

impl Circuit {
    /// Compares operation semantics without allocating remapped operation trees.
    pub(crate) fn operations_structurally_equal(&self, other: &Self) -> bool {
        operations_structurally_equal(
            self.data.as_slice(),
            other.data.as_slice(),
            &self.parameters,
            &other.parameters,
        )
    }
}

fn circuit_params_equal(
    left: &CircuitParam,
    right: &CircuitParam,
    left_parameters: &IndexSet<Parameter>,
    right_parameters: &IndexSet<Parameter>,
) -> bool {
    match (left, right) {
        (CircuitParam::Fixed(left), CircuitParam::Fixed(right)) => left == right,
        (CircuitParam::Index(left), CircuitParam::Index(right)) => left_parameters
            .get_index(*left as usize)
            .zip(right_parameters.get_index(*right as usize))
            .is_some_and(|(left, right)| left == right),
        (CircuitParam::Fixed(value), CircuitParam::Index(index)) => right_parameters
            .get_index(*index as usize)
            .is_some_and(|right| Parameter::from(*value) == *right),
        (CircuitParam::Index(index), CircuitParam::Fixed(value)) => left_parameters
            .get_index(*index as usize)
            .is_some_and(|left| *left == Parameter::from(*value)),
    }
}

fn classical_var_equal(left: ClassicalVar, right: ClassicalVar) -> bool {
    left.index() == right.index() && left.ty() == right.ty()
}

fn classical_value_equal(left: ClassicalValue, right: ClassicalValue) -> bool {
    left.index() == right.index() && left.ty() == right.ty()
}

fn classical_expr_equal(left: &ClassicalExpr, right: &ClassicalExpr) -> bool {
    if left.ty() != right.ty() {
        return false;
    }
    match (left.kind(), right.kind()) {
        (ClassicalExprKind::Var(left), ClassicalExprKind::Var(right)) => {
            classical_var_equal(*left, *right)
        }
        (ClassicalExprKind::Value(left), ClassicalExprKind::Value(right)) => {
            classical_value_equal(*left, *right)
        }
        (
            ClassicalExprKind::Unary {
                op: left_op,
                expr: left,
            },
            ClassicalExprKind::Unary {
                op: right_op,
                expr: right,
            },
        ) => left_op == right_op && classical_expr_equal(left, right),
        (
            ClassicalExprKind::Binary {
                op: left_op,
                lhs: left_lhs,
                rhs: left_rhs,
            },
            ClassicalExprKind::Binary {
                op: right_op,
                lhs: right_lhs,
                rhs: right_rhs,
            },
        ) => {
            left_op == right_op
                && classical_expr_equal(left_lhs, right_lhs)
                && classical_expr_equal(left_rhs, right_rhs)
        }
        (
            ClassicalExprKind::Compare {
                op: left_op,
                lhs: left_lhs,
                rhs: left_rhs,
            },
            ClassicalExprKind::Compare {
                op: right_op,
                lhs: right_lhs,
                rhs: right_rhs,
            },
        ) => {
            left_op == right_op
                && classical_expr_equal(left_lhs, right_lhs)
                && classical_expr_equal(left_rhs, right_rhs)
        }
        (
            ClassicalExprKind::Cast {
                cast: left_cast,
                expr: left,
            },
            ClassicalExprKind::Cast {
                cast: right_cast,
                expr: right,
            },
        ) => left_cast == right_cast && classical_expr_equal(left, right),
        (
            ClassicalExprKind::Select {
                condition: left_condition,
                then_expr: left_then,
                else_expr: left_else,
            },
            ClassicalExprKind::Select {
                condition: right_condition,
                then_expr: right_then,
                else_expr: right_else,
            },
        ) => {
            classical_expr_equal(left_condition, right_condition)
                && classical_expr_equal(left_then, right_then)
                && classical_expr_equal(left_else, right_else)
        }
        (
            ClassicalExprKind::ExtractBit {
                value: left,
                index: left_index,
            },
            ClassicalExprKind::ExtractBit {
                value: right,
                index: right_index,
            },
        ) => left_index == right_index && classical_expr_equal(left, right),
        (
            ClassicalExprKind::ExtractBits {
                value: left,
                offset: left_offset,
                width: left_width,
            },
            ClassicalExprKind::ExtractBits {
                value: right,
                offset: right_offset,
                width: right_width,
            },
        ) => {
            left_offset == right_offset
                && left_width == right_width
                && classical_expr_equal(left, right)
        }
        (ClassicalExprKind::Concat { parts: left }, ClassicalExprKind::Concat { parts: right })
        | (
            ClassicalExprKind::PackBits { bits: left },
            ClassicalExprKind::PackBits { bits: right },
        ) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| classical_expr_equal(left, right))
        }
        (left, right) => left == right,
    }
}

fn classical_data_equal(left: &ClassicalDataOp, right: &ClassicalDataOp) -> bool {
    match (left, right) {
        (
            ClassicalDataOp::Store {
                target: left_target,
                value: left_value,
            },
            ClassicalDataOp::Store {
                target: right_target,
                value: right_value,
            },
        ) => {
            classical_var_equal(*left_target, *right_target)
                && classical_expr_equal(left_value, right_value)
        }
        (
            ClassicalDataOp::MeasureBit { result: left },
            ClassicalDataOp::MeasureBit { result: right },
        )
        | (
            ClassicalDataOp::MeasureBits { result: left },
            ClassicalDataOp::MeasureBits { result: right },
        ) => classical_value_equal(*left, *right),
        _ => false,
    }
}

fn operation_instruction_equal(
    left: &Instruction,
    right: &Instruction,
    left_parameters: &IndexSet<Parameter>,
    right_parameters: &IndexSet<Parameter>,
) -> bool {
    match (left, right) {
        (Instruction::ClassicalData(left), Instruction::ClassicalData(right)) => {
            classical_data_equal(left, right)
        }
        (Instruction::ClassicalControl(left), Instruction::ClassicalControl(right)) => {
            classical_control_equal(left, right, left_parameters, right_parameters)
        }
        _ => left == right,
    }
}

fn control_body_equal(
    left: &ControlBody,
    right: &ControlBody,
    left_parameters: &IndexSet<Parameter>,
    right_parameters: &IndexSet<Parameter>,
) -> bool {
    operations_structurally_equal(
        left.operations(),
        right.operations(),
        left_parameters,
        right_parameters,
    )
}

fn classical_control_equal(
    left: &ClassicalControlOp,
    right: &ClassicalControlOp,
    left_parameters: &IndexSet<Parameter>,
    right_parameters: &IndexSet<Parameter>,
) -> bool {
    match (left, right) {
        (ClassicalControlOp::If(left), ClassicalControlOp::If(right)) => {
            classical_expr_equal(left.condition(), right.condition())
                && control_body_equal(
                    left.then_body(),
                    right.then_body(),
                    left_parameters,
                    right_parameters,
                )
                && match (left.else_body(), right.else_body()) {
                    (Some(left), Some(right)) => {
                        control_body_equal(left, right, left_parameters, right_parameters)
                    }
                    (None, None) => true,
                    _ => false,
                }
        }
        (ClassicalControlOp::While(left), ClassicalControlOp::While(right)) => {
            classical_expr_equal(left.condition(), right.condition())
                && control_body_equal(left.body(), right.body(), left_parameters, right_parameters)
        }
        (ClassicalControlOp::For(left), ClassicalControlOp::For(right)) => {
            classical_var_equal(left.var(), right.var())
                && classical_expr_equal(left.start(), right.start())
                && classical_expr_equal(left.stop(), right.stop())
                && classical_expr_equal(left.step(), right.step())
                && control_body_equal(left.body(), right.body(), left_parameters, right_parameters)
        }
        (ClassicalControlOp::Switch(left), ClassicalControlOp::Switch(right)) => {
            classical_expr_equal(left.target(), right.target())
                && left.cases().len() == right.cases().len()
                && left.cases().iter().zip(right.cases()).all(|(left, right)| {
                    left.value() == right.value()
                        && control_body_equal(
                            left.body(),
                            right.body(),
                            left_parameters,
                            right_parameters,
                        )
                })
                && match (left.default(), right.default()) {
                    (Some(left), Some(right)) => {
                        control_body_equal(left, right, left_parameters, right_parameters)
                    }
                    (None, None) => true,
                    _ => false,
                }
        }
        (ClassicalControlOp::Break, ClassicalControlOp::Break)
        | (ClassicalControlOp::Continue, ClassicalControlOp::Continue) => true,
        _ => false,
    }
}

fn operations_structurally_equal(
    left: &[Operation],
    right: &[Operation],
    left_parameters: &IndexSet<Parameter>,
    right_parameters: &IndexSet<Parameter>,
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.qubits == right.qubits
                && left.label == right.label
                && left.params.len() == right.params.len()
                && left.params.iter().zip(&right.params).all(|(left, right)| {
                    circuit_params_equal(left, right, left_parameters, right_parameters)
                })
                && operation_instruction_equal(
                    &left.instruction,
                    &right.instruction,
                    left_parameters,
                    right_parameters,
                )
        })
}

impl From<usize> for Circuit {
    fn from(num_qubits: usize) -> Self {
        Circuit::new(num_qubits)
    }
}

impl Circuit {
    /// Creates a new, empty quantum circuit with a specified number of qubits.
    ///
    /// The qubits will be automatically indexed from `0` to `num_qubits - 1`.
    ///
    /// # Arguments
    ///
    /// * `num_qubits` - The number of qubits to initialize in the circuit.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cqlib_core::circuit::circuit_impl::Circuit;
    ///
    /// let circuit = Circuit::new(5);
    /// assert_eq!(circuit.num_qubits(), 5);
    /// ```
    pub fn new(num_qubits: usize) -> Self {
        let qubits = (0..num_qubits).map(|i| Qubit::new(i as u32)).collect();

        Self {
            circuit_id: CircuitId::new(),
            qubits,
            data: Arc::new(Vec::new()),
            classical_vars: vec![],
            classical_values: vec![],
            control_scope_stack: vec![],
            symbols: IndexSet::default(),
            parameters: IndexSet::default(),
            global_phase: CircuitParam::Fixed(0.0),
        }
    }

    /// Creates a circuit from a specific list of qubits.
    ///
    /// This is useful when you want to define a sub-circuit or use non-contiguous qubit indices.
    ///
    /// # Arguments
    ///
    /// * `qubits` - A vector of `Qubit` identifiers.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::DuplicateQubits`] if the input vector contains duplicate qubits.
    pub fn from_qubits(qubits: Vec<Qubit>) -> Result<Circuit, CircuitError> {
        if !Self::check_qubits_unique(&qubits) {
            return Err(CircuitError::DuplicateQubits);
        }

        Ok(Self {
            circuit_id: CircuitId::new(),
            symbols: IndexSet::new(),
            qubits: qubits.into_iter().collect(),
            data: Arc::new(Vec::new()),
            classical_vars: vec![],
            classical_values: vec![],
            control_scope_stack: vec![],
            parameters: IndexSet::default(),
            global_phase: CircuitParam::Fixed(0.0),
        })
    }

    /// Creates a circuit from qubits, value-level operations, and optional runtime classical tables.
    ///
    /// Operations are appended in order through value-level lowering, so qubit
    /// membership is validated and every [`ParameterValue`] is interned into
    /// this circuit's parameter table. `classical_vars` and
    /// `classical_values` are installed before appending operations, allowing
    /// expression-based control flow and classical data operations to reference
    /// existing circuit-local handles.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::DuplicateQubits`] if `qubits` contains
    /// duplicates. Also returns any error from [`Circuit::append`], including
    /// [`CircuitError::QubitNotFound`] when an operation references a qubit
    /// outside the circuit.
    pub fn from_operations(
        qubits: Vec<Qubit>,
        operations: impl IntoIterator<Item = ValueOperation>,
        classical_vars: Option<Vec<ClassicalType>>,
        classical_values: Option<Vec<ClassicalType>>,
    ) -> Result<Self, CircuitError> {
        let operations = operations.into_iter().collect::<Vec<_>>();
        let mut circuit = Self::from_qubits(qubits)?;
        Arc::make_mut(&mut circuit.data).reserve(operations.len());
        if let Some(circuit_id) = infer_classical_circuit_id(&operations)? {
            circuit.circuit_id = circuit_id;
        }
        circuit.classical_vars = classical_vars.unwrap_or_default();
        circuit.classical_values = classical_values.unwrap_or_default();
        for operation in operations {
            circuit.append_value_operation_deferred_validation(operation)?;
        }
        circuit.validate_operation_parameters(circuit.operations())?;
        circuit.validate()?;
        Ok(circuit)
    }

    /// Lowers and appends a self-contained value-level operation.
    ///
    /// # Errors
    ///
    /// Returns errors from value-instruction lowering or [`Circuit::append`].
    pub fn append_value_operation(
        &mut self,
        operation: ValueOperation,
    ) -> Result<(), CircuitError> {
        self.append_value_operation_with_validation(operation, true)
    }

    fn append_value_operation_deferred_validation(
        &mut self,
        operation: ValueOperation,
    ) -> Result<(), CircuitError> {
        self.append_value_operation_with_validation(operation, false)
    }

    fn append_value_operation_with_validation(
        &mut self,
        operation: ValueOperation,
        validate_builder_state: bool,
    ) -> Result<(), CircuitError> {
        let ValueOperation {
            instruction,
            qubits,
            params,
            label,
        } = operation;
        let instruction = self.lower_instruction(instruction)?;
        self.append_owned_with_validation(
            instruction,
            qubits,
            params,
            label,
            validate_builder_state,
        )
    }

    /// Adds new qubits to the existing circuit.
    ///
    /// # Arguments
    ///
    /// * `new_qubits` - A vector of new `Qubit` identifiers to add.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::DuplicateQubits`] if any of the new qubits already exist in the circuit
    /// or if `new_qubits` contains duplicates.
    pub fn add_qubits(&mut self, new_qubits: Vec<Qubit>) -> Result<(), CircuitError> {
        let mut seen_new = HashSet::with_capacity(new_qubits.len());

        for q in &new_qubits {
            if self.qubits.contains(q) {
                return Err(CircuitError::DuplicateQubits);
            }

            if !seen_new.insert(*q) {
                return Err(CircuitError::DuplicateQubits);
            }
        }
        self.qubits.extend(new_qubits);
        Ok(())
    }

    /// Interns a parameter and records its symbols.
    ///
    /// Returns the stable table index and whether the parameter was newly inserted.
    pub fn add_parameter(&mut self, param: Parameter) -> (usize, bool) {
        let (index, is_new) = self.parameters.insert_full(param.clone());
        if is_new {
            for sym in param.get_symbols() {
                self.symbols.insert(sym);
            }
        }
        (index, is_new)
    }

    /// Resolves an operation parameter reference into a high-level parameter.
    ///
    /// `CircuitParam::Fixed` values are converted directly into numeric
    /// [`Parameter`] values after validating that the stored float is finite.
    /// `CircuitParam::Index` values are resolved through this circuit's
    /// parameter table.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::InvalidParameterValue`] for non-finite fixed
    /// values and [`CircuitError::InvalidParameterIndex`] for missing parameter
    /// table entries.
    pub fn resolve_parameter(&self, param: &CircuitParam) -> Result<Parameter, CircuitError> {
        match param {
            CircuitParam::Fixed(value) => {
                if !value.is_finite() {
                    return Err(CircuitError::InvalidParameterValue(0, *value));
                }
                Ok(Parameter::from(*value))
            }
            CircuitParam::Index(index) => self
                .parameters
                .get_index(*index as usize)
                .cloned()
                .ok_or(CircuitError::InvalidParameterIndex(*index)),
        }
    }

    /// Resolves an operation parameter reference into an appendable value.
    ///
    /// This is useful when rebuilding operations from an existing circuit:
    /// fixed values remain fixed, while indexed symbolic parameters are looked
    /// up in the circuit parameter table and returned as [`ParameterValue::Param`].
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::InvalidParameterIndex`] when an indexed parameter
    /// does not exist in this circuit's parameter table.
    pub fn parameter_value(&self, param: &CircuitParam) -> Result<ParameterValue, CircuitError> {
        match param {
            CircuitParam::Fixed(value) => Ok(ParameterValue::Fixed(*value)),
            CircuitParam::Index(index) => self
                .parameters
                .get_index(*index as usize)
                .cloned()
                .map(ParameterValue::Param)
                .ok_or(CircuitError::InvalidParameterIndex(*index)),
        }
    }

    /// Maps a high-level parameter into the circuit's operation parameter form.
    ///
    /// This is not the same as [`Circuit::add_parameter`]. `add_parameter`
    /// always inserts a [`Parameter`] into the circuit parameter table and
    /// returns the raw table index. `map_param` first canonicalizes the
    /// parameter and then chooses how operations should store it:
    ///
    /// - constant parameters are simplified, evaluated, finite-checked,
    ///   normalized from `-0.0` to `0.0`, and returned as
    ///   [`CircuitParam::Fixed`] without being inserted into the parameter
    ///   table;
    /// - symbolic parameters are simplified, interned in the parameter table,
    ///   and returned as [`CircuitParam::Index`]. The circuit symbol table is
    ///   updated when a new symbolic parameter is inserted.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::InvalidParameter`] if parameter simplification
    /// or constant evaluation fails.
    pub fn map_param(&mut self, param: Parameter) -> Result<CircuitParam, CircuitError> {
        let param = param.canonicalized()?;
        if param.get_symbols().is_empty() {
            let value = param.evaluate(&None)?;
            let value = if value == 0.0 { 0.0 } else { value };
            Ok(CircuitParam::Fixed(value))
        } else {
            let (index, _) = self.add_parameter(param);
            Ok(CircuitParam::Index(index as u32))
        }
    }

    /// Returns the number of qubits in the circuit.
    ///
    /// Alias for `num_qubits()`.
    pub fn width(&self) -> usize {
        self.qubits.len()
    }

    /// Returns the number of qubits in the circuit.
    pub fn num_qubits(&self) -> usize {
        self.qubits.len()
    }

    /// Returns the parameters of the circuit.
    pub fn parameters(&self) -> &IndexSet<Parameter> {
        &self.parameters
    }

    /// Returns all symbolic variable names present in the parameter registry.
    ///
    /// The registry is stable and is not compacted when operations stop using a
    /// parameter, so this set may include symbols that are no longer referenced
    /// by the executable IR. Use [`Circuit::used_symbols`] when only live
    /// dependencies are needed.
    pub fn symbols(&self) -> &IndexSet<String> {
        &self.symbols
    }

    /// Returns symbolic variable names referenced by the executable IR.
    ///
    /// This includes the global phase and operation parameters in top-level and
    /// nested structured control-flow bodies. The result preserves the stable
    /// insertion order of [`Circuit::symbols`] while filtering out interned but
    /// unreferenced symbols.
    pub fn used_symbols(&self) -> IndexSet<String> {
        let referenced_indices = self.referenced_parameter_indices();
        let mut referenced_symbols = HashSet::new();
        for index in referenced_indices {
            if let Some(parameter) = self.parameters.get_index(index as usize) {
                referenced_symbols.extend(parameter.get_symbols());
            }
        }

        self.symbols
            .iter()
            .filter(|symbol| referenced_symbols.contains(symbol.as_str()))
            .cloned()
            .collect()
    }

    /// Returns whether `symbol` is referenced by the circuit's executable IR.
    ///
    /// This inspects the global phase and operation parameters, including
    /// parameters in nested structured control-flow bodies. It intentionally
    /// does not use the circuit's symbol registry because that registry may
    /// retain symbols that are interned but no longer referenced.
    pub fn uses_symbol(&self, symbol: &str) -> bool {
        self.referenced_parameter_indices()
            .into_iter()
            .any(|index| {
                self.parameters
                    .get_index(index as usize)
                    .is_some_and(|parameter| parameter.get_symbols().contains(symbol))
            })
    }

    fn referenced_parameter_indices(&self) -> HashSet<u32> {
        let mut indices = HashSet::new();
        if let CircuitParam::Index(index) = self.global_phase {
            indices.insert(index);
        }
        Self::collect_operation_parameter_indices(self.data.as_slice(), &mut indices);
        indices
    }

    fn collect_operation_parameter_indices(operations: &[Operation], indices: &mut HashSet<u32>) {
        for operation in operations {
            indices.extend(operation.params.iter().filter_map(|param| match param {
                CircuitParam::Index(index) => Some(*index),
                CircuitParam::Fixed(_) => None,
            }));

            if let Instruction::ClassicalControl(control) = &operation.instruction {
                match control {
                    ClassicalControlOp::If(op) => {
                        Self::collect_operation_parameter_indices(
                            op.then_body().operations(),
                            indices,
                        );
                        if let Some(body) = op.else_body() {
                            Self::collect_operation_parameter_indices(body.operations(), indices);
                        }
                    }
                    ClassicalControlOp::While(op) => {
                        Self::collect_operation_parameter_indices(op.body().operations(), indices);
                    }
                    ClassicalControlOp::For(op) => {
                        Self::collect_operation_parameter_indices(op.body().operations(), indices);
                    }
                    ClassicalControlOp::Switch(op) => {
                        for case in op.cases() {
                            Self::collect_operation_parameter_indices(
                                case.body().operations(),
                                indices,
                            );
                        }
                        if let Some(body) = op.default() {
                            Self::collect_operation_parameter_indices(body.operations(), indices);
                        }
                    }
                    ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
                }
            }
        }
    }

    /// Returns a vector of all qubits in the circuit, preserving their insertion order.
    pub fn qubits(&self) -> Vec<Qubit> {
        self.qubits.iter().cloned().collect()
    }

    /// Returns whether `qubit` belongs to this circuit without materializing
    /// the ordered qubit list.
    pub(crate) fn contains_qubit(&self, qubit: &Qubit) -> bool {
        self.qubits.contains(qubit)
    }

    /// Returns whether two circuits have the same ordered qubit domain without
    /// allocating either public qubit vector.
    pub(crate) fn has_same_qubits(&self, other: &Self) -> bool {
        self.qubits == other.qubits
    }

    /// Returns the global phase of the circuit as a `Parameter`.
    pub fn global_phase(&self) -> Parameter {
        match self.global_phase {
            CircuitParam::Index(index) => self.parameters[index as usize].clone(),
            CircuitParam::Fixed(value) => Parameter::from(value),
        }
    }

    /// Returns the compact storage representation of the global phase.
    pub fn global_phase_param(&self) -> &CircuitParam {
        &self.global_phase
    }

    /// Sets the global phase of the circuit.
    pub fn set_global_phase(&mut self, phase: Parameter) {
        // Try to simplify/evaluate to keep it clean
        if let Ok(val) = phase.evaluate(&None) {
            self.global_phase = CircuitParam::Fixed(val);
        } else {
            let (index, is_new) = self.parameters.insert_full(phase.clone());
            if is_new {
                for sym in phase.get_symbols() {
                    self.symbols.insert(sym);
                }
            }
            self.global_phase = CircuitParam::Index(index as u32);
        }
    }

    /// Returns storage-IR operations in execution order.
    pub fn operations(&self) -> &[Operation] {
        self.data.as_slice()
    }

    /// Removes a top-level operation from the circuit schedule.
    ///
    /// This deletes exactly one operation from the circuit's top-level
    /// operation list. Structured control-flow operations and circuit-backed
    /// gates are removed as whole operations; this API does not delete
    /// individual operations inside a [`ControlBody`] or nested circuit.
    ///
    /// If the removed operation defines measurement-backed classical values,
    /// those values are removed from the circuit's immutable classical value
    /// table and remaining value references are compacted. Deletion fails
    /// atomically when any remaining operation still reads a removed value.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::remove_operations`] for a single index.
    pub fn remove_operation(&mut self, index: usize) -> Result<Operation, CircuitError> {
        let mut removed = self.remove_operations([index])?;
        Ok(removed.remove(0))
    }

    /// Removes multiple top-level operations from the circuit schedule.
    ///
    /// `indices` are interpreted against the original top-level operation
    /// list before deletion. Duplicate indices are ignored, and removed
    /// operations are returned in ascending original-index order. Structured
    /// control-flow operations and circuit-backed gates are removed as whole
    /// operations; this API does not delete individual operations inside a
    /// [`ControlBody`] or nested circuit.
    ///
    /// If the removed operations define measurement-backed classical values,
    /// those values are removed from the circuit's immutable classical value
    /// table and remaining value references are compacted. Deletion fails
    /// atomically when any remaining operation still reads a removed value.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::OperationIndexOutOfBounds`] when any index is
    /// not a valid operation index. Returns
    /// [`CircuitError::ClassicalValueStillInUse`] when a measurement result
    /// defined by the removed operations is still read by an operation that is
    /// not also being removed. Also returns any validation or remapping error
    /// raised by the candidate circuit after deletion.
    pub fn remove_operations<I>(&mut self, indices: I) -> Result<Vec<Operation>, CircuitError>
    where
        I: IntoIterator<Item = usize>,
    {
        let mut indices = indices.into_iter().collect::<Vec<_>>();
        indices.sort_unstable();
        indices.dedup();

        if indices.is_empty() {
            return Ok(Vec::new());
        }

        if let Some(index) = indices
            .iter()
            .copied()
            .find(|&index| index >= self.data.len())
        {
            return Err(CircuitError::OperationIndexOutOfBounds {
                index,
                len: self.data.len(),
            });
        }

        let mut removed = Vec::with_capacity(indices.len());
        let mut removed_values = Vec::new();
        for &index in &indices {
            let operation = self.data[index].clone();
            Self::collect_defined_classical_values(&operation, &mut removed_values);
            removed.push(operation);
        }
        removed_values.sort_unstable();
        removed_values.dedup();

        let remove_set = indices.iter().copied().collect::<HashSet<_>>();

        for (operation_index, operation) in self.data.iter().enumerate() {
            if remove_set.contains(&operation_index) {
                continue;
            }
            for value in &removed_values {
                if operation.instruction.reads_value(*value) {
                    return Err(CircuitError::ClassicalValueStillInUse {
                        index: value.index(),
                        context: format!("operation {operation_index}"),
                    });
                }
            }
        }

        let mut candidate_classical_values = self.classical_values.clone();
        let candidate_data = if removed_values.is_empty() {
            let mut candidate_data = Vec::with_capacity(self.data.len() - indices.len());
            for (operation_index, operation) in self.data.iter().cloned().enumerate() {
                if !remove_set.contains(&operation_index) {
                    candidate_data.push(operation);
                }
            }
            candidate_data
        } else {
            let (compacted_values, value_map) = Self::build_value_compaction_map(
                self.circuit_id,
                &candidate_classical_values,
                &removed_values,
            );
            candidate_classical_values = compacted_values;

            let qubit_mapping = self
                .qubits
                .iter()
                .copied()
                .map(|qubit| (qubit, qubit))
                .collect::<HashMap<_, _>>();
            let param_index_map = (0..self.parameters.len())
                .map(|index| CircuitParam::Index(index as u32))
                .collect::<Vec<_>>();
            let var_map = self
                .classical_vars
                .iter()
                .copied()
                .enumerate()
                .map(|(var_index, ty)| {
                    let var = ClassicalVar::new(self.circuit_id, var_index as u32, ty);
                    (var, var)
                })
                .collect::<HashMap<_, _>>();

            self.data
                .iter()
                .enumerate()
                .filter(|(operation_index, _)| !remove_set.contains(operation_index))
                .map(|(_, operation)| {
                    Self::remap_compose_operation(
                        operation,
                        &qubit_mapping,
                        &param_index_map,
                        &var_map,
                        &value_map,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        let candidate = Self {
            circuit_id: self.circuit_id,
            qubits: self.qubits.clone(),
            symbols: self.symbols.clone(),
            parameters: self.parameters.clone(),
            data: Arc::new(candidate_data),
            classical_vars: self.classical_vars.clone(),
            classical_values: candidate_classical_values,
            control_scope_stack: self.control_scope_stack.clone(),
            global_phase: self.global_phase.clone(),
        };
        candidate.validate()?;

        self.data = candidate.data;
        self.classical_values = candidate.classical_values;

        Ok(removed)
    }

    fn collect_defined_classical_values(operation: &Operation, out: &mut Vec<ClassicalValue>) {
        let collect_from_body = |body: &ControlBody, out: &mut Vec<ClassicalValue>| {
            for operation in body.operations() {
                Circuit::collect_defined_classical_values(operation, out);
            }
        };

        match &operation.instruction {
            Instruction::ClassicalData(op) => {
                if let Some(result) = op.result() {
                    out.push(result);
                }
            }
            Instruction::ClassicalControl(op) => match op {
                ClassicalControlOp::If(op) => {
                    collect_from_body(op.then_body(), out);
                    if let Some(body) = op.else_body() {
                        collect_from_body(body, out);
                    }
                }
                ClassicalControlOp::While(op) => collect_from_body(op.body(), out),
                ClassicalControlOp::For(op) => collect_from_body(op.body(), out),
                ClassicalControlOp::Switch(op) => {
                    for case in op.cases() {
                        collect_from_body(case.body(), out);
                    }
                    if let Some(body) = op.default() {
                        collect_from_body(body, out);
                    }
                }
                ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
            },
            _ => {}
        }
    }

    fn build_value_compaction_map(
        circuit_id: CircuitId,
        values: &[ClassicalType],
        removed: &[ClassicalValue],
    ) -> (Vec<ClassicalType>, HashMap<ClassicalValue, ClassicalValue>) {
        let removed_indices = removed
            .iter()
            .map(|value| value.index())
            .collect::<HashSet<_>>();
        let mut compacted = Vec::with_capacity(values.len().saturating_sub(removed_indices.len()));
        let mut value_map =
            HashMap::with_capacity(values.len().saturating_sub(removed_indices.len()));

        for (old_index, ty) in values.iter().copied().enumerate() {
            if removed_indices.contains(&(old_index as u32)) {
                continue;
            }
            let new_index = compacted.len() as u32;
            compacted.push(ty);
            value_map.insert(
                ClassicalValue::new(circuit_id, old_index as u32, ty),
                ClassicalValue::new(circuit_id, new_index, ty),
            );
        }

        (compacted, value_map)
    }

    /// Returns the circuit depth (longest ASAP schedule path over qubit wires).
    ///
    /// Every instruction node contributes one layer on the qubits it touches;
    /// barriers synchronize their listed qubits (an empty-qubit barrier is
    /// global, synchronizing every circuit qubit). `CircuitGate` is opaque
    /// (depth 1); call [`decompose`](Self::decompose) first to recurse into
    /// nested sub-circuits.
    ///
    /// With `recurse = false` (default), an error is returned if the circuit
    /// contains any classical control-flow operation (`if`/`while`/`for`/
    /// `switch`/`break`/`continue`). With `recurse = true`, control flow is
    /// unfolded into an estimated depth: `if`/`switch` take the max branch,
    /// `while` counts the body once, `for` with a statically-known
    /// unsigned-literal range is fully unrolled (else counted once).
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::ControlFlowPresent`] when `recurse = false` and
    /// the circuit contains control flow.
    pub fn depth(&self, recurse: bool) -> Result<usize, CircuitError> {
        crate::circuit::depth::circuit_depth(
            self.qubits.iter().copied(),
            self.data.as_slice(),
            recurse,
        )
    }

    /// Returns the static types of runtime classical variables owned by this circuit.
    pub fn classical_vars(&self) -> &[ClassicalType] {
        &self.classical_vars
    }

    /// Returns this circuit's process-local identity for constructing explicit
    /// classical IR handles.
    pub fn id(&self) -> CircuitId {
        self.circuit_id
    }

    /// Returns the static types of immutable runtime classical values owned by this circuit.
    pub fn classical_values(&self) -> &[ClassicalType] {
        &self.classical_values
    }

    /// Appends a generic instruction to the circuit.
    ///
    /// This is the low-level method used by all specific gate methods (e.g., `h`, `cx`).
    /// It handles arity validation, parameter interning, and qubit validation.
    ///
    /// # Arguments
    ///
    /// * `instruction` - The instruction to append (Standard, Extended, or Directive).
    /// * `qubits` - The qubits this instruction acts upon.
    /// * `params` - The parameters for the instruction (if any).
    /// * `label` - An optional label for the operation.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::QubitCountMismatch`] or
    /// [`CircuitError::ParameterCountMismatch`] when a fixed-arity instruction
    /// receives the wrong number of operands. Returns [`CircuitError::DuplicateQubits`]
    /// when the same qubit is supplied more than once and
    /// [`CircuitError::QubitNotFound`] if a qubit is not present in the circuit.
    /// Non-finite fixed parameters produce [`CircuitError::InvalidParameterValue`].
    /// Classical data and control-flow instructions may additionally fail their
    /// ownership, type, scope, or structural validation.
    pub fn append<Q, P>(
        &mut self,
        instruction: Instruction,
        qubits: Q,
        params: P,
        label: Option<&str>,
    ) -> Result<(), CircuitError>
    where
        Q: IntoIterator,
        Q::Item: Into<Qubit>,
        P: IntoIterator<Item = ParameterValue>,
    {
        self.append_with_validation(instruction, qubits, params, label, true)
    }

    fn append_with_validation<Q, P>(
        &mut self,
        instruction: Instruction,
        qubits: Q,
        params: P,
        label: Option<&str>,
        validate_builder_state: bool,
    ) -> Result<(), CircuitError>
    where
        Q: IntoIterator,
        Q::Item: Into<Qubit>,
        P: IntoIterator<Item = ParameterValue>,
    {
        self.append_owned_with_validation(
            instruction,
            qubits,
            params,
            label.map(Into::into),
            validate_builder_state,
        )
    }

    fn append_owned_with_validation<Q, P>(
        &mut self,
        instruction: Instruction,
        qubits: Q,
        params: P,
        label: Option<Box<str>>,
        validate_builder_state: bool,
    ) -> Result<(), CircuitError>
    where
        Q: IntoIterator,
        Q::Item: Into<Qubit>,
        P: IntoIterator<Item = ParameterValue>,
    {
        let validate_classical_now = validate_builder_state
            && matches!(
                instruction,
                Instruction::ClassicalData(_) | Instruction::ClassicalControl(_)
            );
        let checkpoint = validate_classical_now.then(|| self.checkpoint());

        if let Instruction::ClassicalControl(op) = &instruction {
            self.validate_control_op(op)?;
        }

        let qubits_sv: SmallVec<[Qubit; 3]> = qubits.into_iter().map(|q| q.into()).collect();
        let params_sv: SmallVec<[ParameterValue; 1]> = params.into_iter().collect();

        if let Some((expected_qubits, expected_params)) = instruction.gate_arity() {
            if qubits_sv.len() != expected_qubits {
                return Err(CircuitError::QubitCountMismatch {
                    expected: expected_qubits,
                    actual: qubits_sv.len(),
                });
            }
            if params_sv.len() != expected_params {
                return Err(CircuitError::ParameterCountMismatch {
                    expected: expected_params,
                    actual: params_sv.len(),
                });
            }
        }

        const INLINE_DUPLICATE_CHECK_LIMIT: usize = 8;
        if qubits_sv.len() <= INLINE_DUPLICATE_CHECK_LIMIT {
            if qubits_sv
                .iter()
                .enumerate()
                .any(|(index, qubit)| qubits_sv[..index].contains(qubit))
            {
                return Err(CircuitError::DuplicateQubits);
            }
        } else {
            let mut seen = HashSet::with_capacity(qubits_sv.len());
            for &qubit in &qubits_sv {
                if !seen.insert(qubit) {
                    return Err(CircuitError::DuplicateQubits);
                }
            }
        }

        for qubit in &qubits_sv {
            if !self.qubits.contains(qubit) {
                return Err(CircuitError::QubitNotFound(qubit.id()));
            }
        }
        if let Instruction::ClassicalData(op) = &instruction {
            self.validate_classical_data_op(op, qubits_sv.len())?;
        }

        let mut circuit_params = smallvec![];
        for (param_index, p) in params_sv.into_iter().enumerate() {
            match p {
                ParameterValue::Param(param) => {
                    let (index, is_new) = self.parameters.insert_full(param.clone());
                    if is_new {
                        for sym in param.get_symbols() {
                            self.symbols.insert(sym);
                        }
                    }
                    circuit_params.push(CircuitParam::Index(index as u32));
                }
                ParameterValue::Fixed(value) => {
                    if !value.is_finite() {
                        return Err(CircuitError::InvalidParameterValue(param_index, value));
                    }
                    circuit_params.push(CircuitParam::Fixed(value));
                }
            }
        }

        Arc::make_mut(&mut self.data).push(Operation {
            instruction,
            qubits: qubits_sv,
            params: circuit_params,
            label,
        });

        if validate_classical_now && let Err(error) = self.validate_builder_state() {
            self.rollback_to(checkpoint.expect("classical append must define a checkpoint"));
            return Err(error);
        }

        Ok(())
    }

    /// Appends a Hadamard (H) gate.
    ///
    /// The H gate creates a superposition state: $H|0\rangle = \frac{|0\rangle + |1\rangle}{\sqrt{2}}$.
    pub fn h(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::H),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    // --- Pauli Gates ---

    /// Appends an Identity (I) gate.
    ///
    /// This is a no-op gate, often used for alignment or waiting.
    pub fn i(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::I),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    /// Appends a Pauli-X (NOT) gate.
    ///
    /// Performs a bit flip: $X|0\rangle = |1\rangle, X|1\rangle = |0\rangle$.
    pub fn x(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::X),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    /// Appends a Pauli-Y gate.
    ///
    /// Performs a bit and phase flip: $Y|0\rangle = i|1\rangle, Y|1\rangle = -i|0\rangle$.
    pub fn y(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::Y),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    /// Appends a Pauli-Z gate.
    ///
    /// Performs a phase flip: $Z|0\rangle = |0\rangle, Z|1\rangle = -|1\rangle$.
    pub fn z(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::Z),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    /// Appends a $\sqrt{X}$ (SX) gate.
    ///
    /// A 90-degree rotation around the X-axis. $SX^2 = X$.
    pub fn x2p(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::X2P),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    /// Appends a $\sqrt{X}^\dagger$ (SXdg) gate.
    ///
    /// The inverse of the SX gate.
    pub fn x2m(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::X2M),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    /// Appends a $\sqrt{Y}$ gate.
    pub fn y2p(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::Y2P),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    /// Appends a $\sqrt{Y}^\dagger$ gate.
    pub fn y2m(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::Y2M),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    /// Appends an XY gate.
    ///
    /// Applies a pi rotation about the axis at angle `theta` in the single-qubit XY plane.
    pub fn xy(
        &mut self,
        qubit: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];
        self.append(
            Instruction::Standard(StandardGate::XY),
            [qubit],
            params,
            None,
        )
    }

    /// Appends a positive half-pi rotation in the single-qubit XY plane.
    pub fn xy2p(
        &mut self,
        qubit: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];
        self.append(
            Instruction::Standard(StandardGate::XY2P),
            [qubit],
            params,
            None,
        )
    }

    /// Appends a negative half-pi rotation in the single-qubit XY plane.
    pub fn xy2m(
        &mut self,
        qubit: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];
        self.append(
            Instruction::Standard(StandardGate::XY2M),
            [qubit],
            params,
            None,
        )
    }

    // --- Clifford & Phase Gates ---

    /// Appends an S (Phase) gate.
    ///
    /// Applies a phase of $i$ to the $|1\rangle$ state ($Z^{1/2}$).
    pub fn s(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::S),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    /// Appends an S-dagger ($S^\dagger$) gate.
    ///
    /// Applies a phase of $-i$ to the $|1\rangle$ state.
    pub fn sdg(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::SDG),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    /// Appends a T gate.
    ///
    /// Applies a phase of $e^{i\pi/4}$ to the $|1\rangle$ state ($Z^{1/4}$).
    pub fn t(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::T),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    /// Appends a T-dagger ($T^\dagger$) gate.
    pub fn tdg(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::TDG),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    // --- Parametric Rotations ---

    /// Appends a rotation around the X-axis by angle `theta`.
    ///
    /// $RX(\theta) = e^{-i\theta X/2}$
    pub fn rx(
        &mut self,
        qubit: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];
        self.append(
            Instruction::Standard(StandardGate::RX),
            [qubit],
            params,
            None,
        )
    }

    /// Appends a rotation around the Y-axis by angle `theta`.
    ///
    /// $RY(\theta) = e^{-i\theta Y/2}$
    pub fn ry(
        &mut self,
        qubit: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];

        self.append(
            Instruction::Standard(StandardGate::RY),
            [qubit],
            params,
            None,
        )
    }

    /// Appends a rotation around the Z-axis by angle `theta`.
    ///
    /// $RZ(\theta) = e^{-i\theta Z/2}$
    pub fn rz(
        &mut self,
        qubit: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];

        self.append(
            Instruction::Standard(StandardGate::RZ),
            [qubit],
            params,
            None,
        )
    }

    /// Appends a Phase gate (P gate).
    ///
    /// Applies a phase of $e^{i\lambda}$ to the $|1\rangle$ state.
    /// Equivalent to $RZ(\lambda)$ up to a global phase.
    pub fn phase(
        &mut self,
        qubit: Qubit,
        lambda: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![lambda.into()];

        self.append(
            Instruction::Standard(StandardGate::Phase),
            [qubit],
            params,
            None,
        )
    }

    /// Appends a generic single-qubit rotation gate $U(\theta, \phi, \lambda)$.
    ///
    /// This is the most general single-qubit unitary gate.
    /// $$
    /// U(\theta, \phi, \lambda) = \begin{pmatrix}
    /// \cos(\theta/2) & -e^{i\lambda}\sin(\theta/2) \\
    /// e^{i\phi}\sin(\theta/2) & e^{i(\phi+\lambda)}\cos(\theta/2)
    /// \end{pmatrix}
    /// $$
    pub fn u(
        &mut self,
        qubit: Qubit,
        theta: impl Into<ParameterValue>,
        phi: impl Into<ParameterValue>,
        lambda: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> =
            smallvec![theta.into(), phi.into(), lambda.into()];

        self.append(
            Instruction::Standard(StandardGate::U),
            [qubit],
            params,
            None,
        )
    }

    /// Appends a Controlled-NOT (CX or CNOT) gate.
    ///
    /// Flips the `target` qubit if and only if the `control` qubit is $|1\rangle$.
    pub fn cx(&mut self, control: Qubit, target: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::CX),
            [control, target],
            std::iter::empty(),
            None,
        )
    }

    /// Appends a Controlled-Y (CY) gate.
    pub fn cy(&mut self, control: Qubit, target: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::CY),
            [control, target],
            std::iter::empty(),
            None,
        )
    }

    /// Appends a Controlled-Z (CZ) gate.
    ///
    /// Adds a phase of -1 only if both qubits are $|1\rangle$.
    pub fn cz(&mut self, control: Qubit, target: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::CZ),
            [control, target],
            std::iter::empty(),
            None,
        )
    }

    /// Appends a SWAP gate.
    ///
    /// Exchange the states of two qubits.
    pub fn swap(&mut self, a: Qubit, b: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::SWAP),
            [a, b],
            std::iter::empty(),
            None,
        )
    }

    /// Appends an Ising XX coupling gate ($R_{XX}(\theta)$).
    ///
    /// $R_{XX}(\theta) = e^{-i\theta X \otimes X / 2}$
    pub fn rxx(
        &mut self,
        a: Qubit,
        b: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];

        self.append(
            Instruction::Standard(StandardGate::RXX),
            [a, b],
            params,
            None,
        )
    }

    /// Appends an Ising YY coupling gate ($R_{YY}(\theta)$).
    ///
    /// $R_{YY}(\theta) = e^{-i\theta Y \otimes Y / 2}$
    pub fn ryy(
        &mut self,
        a: Qubit,
        b: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];

        self.append(
            Instruction::Standard(StandardGate::RYY),
            [a, b],
            params,
            None,
        )
    }

    /// Appends an Ising ZZ coupling gate ($R_{ZZ}(\theta)$).
    ///
    /// $R_{ZZ}(\theta) = e^{-i\theta Z \otimes Z / 2}$
    pub fn rzz(
        &mut self,
        a: Qubit,
        b: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];

        self.append(
            Instruction::Standard(StandardGate::RZZ),
            [a, b],
            params,
            None,
        )
    }

    /// Appends an Ising ZX coupling gate ($R_{ZX}(\theta)$).
    ///
    /// $R_{ZX}(\theta) = e^{-i\theta Z \otimes X / 2}$
    pub fn rzx(
        &mut self,
        a: Qubit,
        b: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];

        self.append(
            Instruction::Standard(StandardGate::RZX),
            [a, b],
            params,
            None,
        )
    }

    /// Appends a Controlled-RX gate (CRX).
    ///
    /// Performs an X-rotation on the target if the control is $|1\rangle$.
    pub fn crx(
        &mut self,
        control: Qubit,
        target: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];

        self.append(
            Instruction::Standard(StandardGate::CRX),
            [control, target],
            params,
            None,
        )
    }

    /// Appends a Controlled-RY gate (CRY).
    pub fn cry(
        &mut self,
        control: Qubit,
        target: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];

        self.append(
            Instruction::Standard(StandardGate::CRY),
            [control, target],
            params,
            None,
        )
    }

    /// Appends a Controlled-RZ gate (CRZ).
    pub fn crz(
        &mut self,
        control: Qubit,
        target: Qubit,
        theta: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into()];

        self.append(
            Instruction::Standard(StandardGate::CRZ),
            [control, target],
            params,
            None,
        )
    }

    /// Appends a Toffoli gate (CCX).
    ///
    /// A 3-qubit gate where the target flips if and only if both controls are $|1\rangle$.
    pub fn ccx(
        &mut self,
        control1: Qubit,
        control2: Qubit,
        target: Qubit,
    ) -> Result<(), CircuitError> {
        self.append(
            Instruction::Standard(StandardGate::CCX),
            [control1, control2, target],
            std::iter::empty(),
            None,
        )
    }

    // --- Advanced / Other Gates ---

    /// Appends a Fermionic Simulation gate (fSim).
    ///
    /// Useful in quantum chemistry simulations.
    ///
    /// $$
    /// \text{fSim}(\theta, \phi) = \begin{pmatrix}
    /// 1 & 0 & 0 & 0 \\
    /// 0 & \cos\theta & -i\sin\theta & 0 \\
    /// 0 & -i\sin\theta & \cos\theta & 0 \\
    /// 0 & 0 & 0 & e^{-i\phi}
    /// \end{pmatrix}
    /// $$
    pub fn fsim(
        &mut self,
        a: Qubit,
        b: Qubit,
        theta: impl Into<ParameterValue>,
        phi: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 1]> = smallvec![theta.into(), phi.into()];

        self.append(
            Instruction::Standard(StandardGate::FSIM),
            [a, b],
            params,
            None,
        )
    }

    /// Appends a rotation in the XY plane.
    pub fn rxy(
        &mut self,
        qubit: Qubit,
        theta: impl Into<ParameterValue>,
        phi: impl Into<ParameterValue>,
    ) -> Result<(), CircuitError> {
        let params: SmallVec<[ParameterValue; 2]> = smallvec![theta.into(), phi.into()];

        self.append(
            Instruction::Standard(StandardGate::RXY),
            [qubit],
            params,
            None,
        )
    }

    /// Inserts a Barrier.
    ///
    /// A barrier forbids the compiler from optimizing or reordering across this line. It has no
    /// physical effect on the quantum state, but is crucial for debugging and manual optimization
    /// control.
    ///
    /// A non-empty `qubits` list creates a barrier over exactly those qubits. An empty list
    /// creates a global barrier over every qubit in this circuit.
    pub fn barrier(&mut self, qubits: Vec<Qubit>) -> Result<(), CircuitError> {
        self.append(
            Instruction::Directive(Directive::Barrier),
            qubits,
            std::iter::empty(),
            None,
        )
    }

    /// Resets a qubit to the $|0\rangle$ state.
    ///
    /// This is a non-unitary operation.
    pub fn reset(&mut self, qubit: Qubit) -> Result<(), CircuitError> {
        self.append(
            Instruction::Directive(Directive::Reset),
            [qubit],
            std::iter::empty(),
            None,
        )
    }

    /// Applies a multi-controlled version of a standard gate.
    ///
    /// This method automatically handles gate promotion. For example, applying `X` with 1 control
    /// becomes `CX`, and with 2 controls becomes `CCX`. For higher numbers of controls, it creates
    /// an [`Instruction::McGate`].
    ///
    /// # Arguments
    ///
    /// * `gate` - The base standard gate to apply (e.g., `X`, `Y`, `RX`).
    /// * `controls` - A list of control qubits.
    /// * `targets` - A list of target qubits.
    /// * `params` - Parameters for the base gate.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cqlib_core::circuit::circuit_impl::Circuit;
    /// use cqlib_core::circuit::Qubit;
    /// use cqlib_core::circuit::gate::StandardGate;
    ///
    /// let mut circuit = Circuit::new(4);
    /// let q0 = Qubit::new(0);
    /// let q1 = Qubit::new(1);
    /// let q2 = Qubit::new(2);
    ///
    /// // Equivalent to CCX(q0, q1, q2)
    /// circuit.multi_control(StandardGate::X, [q0, q1], vec![q2], []).unwrap();
    /// ```
    pub fn multi_control<I, C, T, P>(
        &mut self,
        instruction: I,
        controls: C,
        targets: T,
        params: P,
    ) -> Result<(), CircuitError>
    where
        I: Into<Instruction>,
        C: IntoIterator,
        C::Item: Into<Qubit>,
        T: IntoIterator,
        T::Item: Into<Qubit>,
        P: IntoIterator<Item = ParameterValue>,
    {
        let controls_sv: SmallVec<[Qubit; 3]> = controls.into_iter().map(|q| q.into()).collect();
        let targets_sv: SmallVec<[Qubit; 1]> = targets.into_iter().map(|q| q.into()).collect();
        let num_controls = controls_sv.len();

        let inst: Instruction = instruction.into();

        let controlled_inst = inst
            .control(num_controls)
            .ok_or_else(|| CircuitError::InvalidControlOperation(inst.to_string()))?;

        let mut all_qubits = controls_sv;
        all_qubits.extend(targets_sv);
        self.append(controlled_inst, all_qubits, params, None)
    }

    /// Appends a custom unitary gate to the circuit.
    ///
    /// This allows inserting user-defined gates defined by a specific matrix.
    ///
    /// # Arguments
    /// * `definition` - The definition of the custom gate (matrix, label, etc.).
    /// * `qubits` - The list of qubits to apply the gate to.
    ///
    /// # Example
    /// ```rust
    /// use ndarray::Array2;
    /// use num_complex::Complex64;
    /// use cqlib_core::circuit::circuit_impl::Circuit;
    /// use cqlib_core::circuit::gate::UnitaryGate;
    /// use cqlib_core::circuit::Qubit;
    ///
    /// // Define a custom gate (e.g., Identity)
    /// let mat = Array2::eye(2).mapv(|x| Complex64::new(x, 0.0));
    /// let u_gate = UnitaryGate::new("MyGate", 1, 0)
    ///      .with_matrix(mat)
    ///      .unwrap();
    ///
    /// let mut circuit = Circuit::new(4);
    /// let q0 = Qubit::new(0);
    ///
    /// // Apply the custom gate
    /// circuit.unitary(u_gate, vec![q0]).unwrap();
    /// ```
    pub fn unitary(&mut self, gate: UnitaryGate, qubits: Vec<Qubit>) -> Result<(), CircuitError> {
        self.unitary_with_params(gate, qubits, std::iter::empty())
    }

    /// Appends a parameterized custom unitary gate to the circuit.
    pub fn unitary_with_params<P>(
        &mut self,
        gate: UnitaryGate,
        qubits: Vec<Qubit>,
        params: P,
    ) -> Result<(), CircuitError>
    where
        P: IntoIterator<Item = ParameterValue>,
    {
        let qubits_sv: SmallVec<[Qubit; 3]> = qubits.into();
        let params_vec: Vec<ParameterValue> = params.into_iter().collect();

        // Check if qubit count matches definition.num_qubits
        if qubits_sv.len() != gate.num_qubits() as usize {
            return Err(CircuitError::QubitCountMismatch {
                expected: gate.num_qubits() as usize,
                actual: qubits_sv.len(),
            });
        }
        if params_vec.len() != gate.num_params() as usize {
            return Err(CircuitError::ParameterCountMismatch {
                expected: gate.num_params() as usize,
                actual: params_vec.len(),
            });
        }

        self.append(
            Instruction::UnitaryGate(Box::new(gate)),
            qubits_sv,
            params_vec,
            None,
        )
    }

    /// Appends a Delay instruction to the circuit.
    ///
    /// This instruction represents an idle period on a specific qubit, often used for
    /// dynamical decoupling or timing control in pulse-level scheduling.
    ///
    /// # Arguments
    ///
    /// * `qubit` - The qubit to apply the delay to.
    /// * `delay` - The duration of the delay. The unit depends on the target backend (e.g., seconds, samples, or dt).
    pub fn delay(
        &mut self,
        qubit: impl Into<Qubit>,
        delay: ParameterValue,
    ) -> Result<(), CircuitError> {
        self.append(Instruction::Delay, vec![qubit], vec![delay], None)
    }

    /// Appends a pre-compiled `CircuitGate` to this circuit.
    ///
    /// This allows nesting circuits within circuits.
    ///
    /// # Arguments
    ///
    /// * `gate` - The `CircuitGate` instance to append.
    /// * `qubits` - The qubits in this circuit that the sub-circuit acts upon.
    /// * `params` - The parameter values to bind to the sub-circuit's parameters.
    pub fn circuit_gate(
        &mut self,
        gate: CircuitGate,
        qubits: Vec<Qubit>,
        params: impl IntoIterator<Item = ParameterValue>,
    ) -> Result<(), CircuitError> {
        self.append(
            Instruction::CircuitGate(Box::new(gate)),
            qubits,
            params,
            None,
        )
    }

    /// Creates the inverse (adjoint) of the circuit.
    ///
    /// The inverse circuit represents the unitary $U^\dagger$ such that $U^\dagger U = I$.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::IrreversibleOperation`] if the circuit contains non-unitary
    /// operations (e.g., `Measure`, `Reset`) or gates that cannot be symbolically inverted.
    pub fn inverse(&self) -> Result<Circuit, CircuitError> {
        let mut new_circuit = Circuit::from_qubits(self.qubits())?;
        new_circuit.classical_vars = self.classical_vars.clone();
        new_circuit.classical_values = self.classical_values.clone();
        Arc::make_mut(&mut new_circuit.data).reserve(self.data.len());
        // 1. Invert Global Phase
        let current_phase_param = self.global_phase();
        // New phase = -1.0 * old_phase
        let new_phase_param = Parameter::from(-1.0) * current_phase_param;

        // Try to simplify/evaluate to keep it clean (e.g. Fixed(-0.5))
        if let Ok(val) = new_phase_param.evaluate(&None) {
            new_circuit.global_phase = CircuitParam::Fixed(val);
        } else {
            let (index, is_new) = new_circuit.parameters.insert_full(new_phase_param.clone());
            if is_new {
                for sym in new_phase_param.get_symbols() {
                    new_circuit.symbols.insert(sym);
                }
            }
            new_circuit.global_phase = CircuitParam::Index(index as u32);
        }

        // 2. Iterate backwards
        for op in self.data.iter().rev() {
            // Special handling for Directives
            match &op.instruction {
                Instruction::Directive(directive) => match directive {
                    Directive::Barrier => {
                        new_circuit.append(
                            Instruction::Directive(Directive::Barrier),
                            op.qubits.clone(),
                            std::iter::empty(),
                            op.label.as_deref(),
                        )?;
                        continue;
                    }
                    _ => return Err(CircuitError::IrreversibleOperation),
                },
                Instruction::ClassicalData(_) | Instruction::ClassicalControl(_) => {
                    // Control flow operations cannot be statically inverted
                    return Err(CircuitError::IrreversibleOperation);
                }
                _ => {
                    // Resolve parameters
                    let params: SmallVec<[Parameter; 3]> = op
                        .params
                        .iter()
                        .map(|p| match p {
                            CircuitParam::Fixed(val) => Parameter::from(*val),
                            CircuitParam::Index(idx) => self.parameters[*idx as usize].clone(),
                        })
                        .collect();

                    // Invert instruction
                    if let Some((inv_inst, inv_params)) = op.instruction.inverse(&params) {
                        // Convert back to CircuitParam/ParameterValue
                        let param_values: SmallVec<[ParameterValue; 3]> =
                            inv_params.into_iter().map(ParameterValue::from).collect();

                        new_circuit.append(
                            inv_inst,
                            op.qubits.clone(),
                            param_values,
                            op.label.as_deref(),
                        )?;
                    } else {
                        return Err(CircuitError::IrreversibleOperation);
                    }
                }
            }
        }

        Ok(new_circuit)
    }

    /// Converts the circuit into a `CircuitGate` instruction.
    ///
    /// This method "freezes" the current circuit and wraps it into an instruction that can be
    /// appended to another circuit. Its positional parameter signature is
    /// inferred from symbols currently referenced by the executable IR, in
    /// stable registry order.
    ///
    /// # Arguments
    ///
    /// * `name` - A name for the new gate.
    pub fn to_gate(self, name: impl Into<String>) -> Result<Instruction, CircuitError> {
        let frozen = FrozenCircuit::new(self);
        let gate = CircuitGate::new(name, frozen)?;
        Ok(Instruction::CircuitGate(Box::new(gate)))
    }

    fn check_qubits_unique(qubits: &[Qubit]) -> bool {
        let mut seen = HashSet::with_capacity(qubits.len());
        for q in qubits {
            if !seen.insert(q) {
                return false;
            }
        }
        true
    }

    /// Decomposes the circuit by resolving sub-circuit gates into their fundamental operations.
    ///
    /// This method recursively unpacks any [`Instruction::CircuitGate`] (hierarchical instructions)
    /// found in the circuit. It handles:
    ///
    /// 1. **Parameter Substitution**: Parameters in the sub-circuit are replaced by the arguments
    ///    passed from the parent circuit.
    ///    - Example: If sub-circuit has `Rx(theta+1)` and is called with `theta = beta`,
    ///      the result is `Rx(beta+1)`.
    /// 2. **Qubit Mapping**: Virtual qubits in the sub-circuit definition are mapped to the
    ///    physical qubits in the parent circuit.
    ///
    /// # Returns
    ///
    /// - `Ok(Circuit)`: A new flattened `Circuit` containing only base instructions (Standard, Unitary, Directives).
    /// - `Err(CircuitError)`: If a parameter cannot be resolved during decomposition.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::UnresolvedParameter`] if a symbolic parameter in a sub-circuit
    /// or control flow body cannot be evaluated to a concrete value.
    pub fn decompose(&self) -> Result<Circuit, CircuitError> {
        let mut new_circuit = Circuit::from_qubits(self.qubits()).unwrap();
        new_circuit.classical_vars = self.classical_vars.clone();
        new_circuit.classical_values = self.classical_values.clone();
        // Preserve the order of symbols from the original circuit.
        new_circuit.symbols = self.symbols.clone();

        // Copy global phase
        match &self.global_phase {
            CircuitParam::Fixed(f) => new_circuit.global_phase = CircuitParam::Fixed(*f),
            CircuitParam::Index(i) => {
                let p = self.parameters[*i as usize].clone();
                let (idx, is_new) = new_circuit.parameters.insert_full(p.clone());
                if is_new {
                    for sym in p.get_symbols() {
                        new_circuit.symbols.insert(sym);
                    }
                }
                new_circuit.global_phase = CircuitParam::Index(idx as u32);
            }
        }

        let initial_qubit_map: HashMap<Qubit, Qubit> =
            self.qubits.iter().map(|q| (*q, *q)).collect();
        let initial_param_map: HashMap<String, Parameter> = HashMap::new();
        let var_map = self
            .classical_vars
            .iter()
            .enumerate()
            .map(|(index, ty)| {
                (
                    ClassicalVar::new(self.circuit_id, index as u32, *ty),
                    ClassicalVar::new(new_circuit.circuit_id, index as u32, *ty),
                )
            })
            .collect();
        let value_map = self
            .classical_values
            .iter()
            .enumerate()
            .map(|(index, ty)| {
                (
                    ClassicalValue::new(self.circuit_id, index as u32, *ty),
                    ClassicalValue::new(new_circuit.circuit_id, index as u32, *ty),
                )
            })
            .collect();

        for op in self.data.iter() {
            Self::decompose_recursive(
                op,
                self,
                &initial_qubit_map,
                &initial_param_map,
                &var_map,
                &value_map,
                &mut new_circuit,
            )?;
        }

        Ok(new_circuit)
    }

    fn decompose_recursive(
        op: &Operation,
        context_circuit: &Circuit,
        qubit_map: &HashMap<Qubit, Qubit>,
        param_map: &HashMap<String, Parameter>,
        var_map: &HashMap<ClassicalVar, ClassicalVar>,
        value_map: &HashMap<ClassicalValue, ClassicalValue>,
        target_circuit: &mut Circuit,
    ) -> Result<(), CircuitError> {
        match &op.instruction {
            Instruction::CircuitGate(cg) => {
                // 1. Resolve Parameters in current context
                let mut resolved_params = Vec::with_capacity(op.params.len());
                for p in &op.params {
                    let mut param = match p {
                        CircuitParam::Fixed(v) => Parameter::from(*v),
                        CircuitParam::Index(idx) => {
                            context_circuit.parameters[*idx as usize].clone()
                        }
                    };

                    // Apply substitution from the *parent* scope (if we are deep in recursion)
                    // We need simultaneous substitution here too
                    param = Self::apply_param_map(param, param_map);
                    if param.get_symbols().is_empty() {
                        param.evaluate(&None)?;
                    }
                    resolved_params.push(param);
                }

                // 2. Build maps for the next level
                // Param Map: Inner Symbol -> Resolved Value
                let mut next_param_map = HashMap::new();
                for (i, sym) in cg.signature_params().iter().enumerate() {
                    if i < resolved_params.len() {
                        next_param_map.insert(sym.clone(), resolved_params[i].clone());
                    }
                }

                // Qubit Map: Inner Qubit -> Outer Qubit
                // op.qubits are the qubits in 'context_circuit' that the gate acts on.
                // We need to map them through 'qubit_map' to get 'target_circuit' qubits.
                let mut next_qubit_map = HashMap::new();
                for (i, inner_q) in cg.circuit.circuit.qubits().iter().enumerate() {
                    if i < op.qubits.len() {
                        let local_q = op.qubits[i];
                        let global_q = qubit_map.get(&local_q).unwrap_or(&local_q);
                        next_qubit_map.insert(*inner_q, *global_q);
                    }
                }

                // 3. Recurse
                for sub_op in cg.circuit.circuit.data.iter() {
                    Self::decompose_recursive(
                        sub_op,
                        &cg.circuit.circuit,
                        &next_qubit_map,
                        &next_param_map,
                        var_map,
                        value_map,
                        target_circuit,
                    )?;
                }
                Ok(())
            }
            Instruction::ClassicalControl(_) => Err(CircuitError::InvalidOperation(
                "decomposing circuits with classical control is not yet supported".to_string(),
            )),
            _ => {
                // Base case: Standard/Unitary/Directive
                // Map Qubits
                let mapped_qubits: SmallVec<[Qubit; 3]> = op
                    .qubits
                    .iter()
                    .map(|q| *qubit_map.get(q).unwrap_or(q))
                    .collect();

                // Map Parameters
                let mut mapped_params: SmallVec<[ParameterValue; 3]> = smallvec![];
                for p in &op.params {
                    let mut param = match p {
                        CircuitParam::Fixed(v) => Parameter::from(*v),
                        CircuitParam::Index(idx) => {
                            context_circuit.parameters[*idx as usize].clone()
                        }
                    };

                    param = Self::apply_param_map(param, param_map);
                    if param.get_symbols().is_empty() {
                        param.evaluate(&None)?;
                    }

                    mapped_params.push(ParameterValue::from(param));
                }

                let instruction = match &op.instruction {
                    Instruction::ClassicalData(op) => Instruction::ClassicalData(
                        Self::remap_classical_data_op(op, var_map, value_map)?,
                    ),
                    _ => op.instruction.clone(),
                };

                target_circuit.append(
                    instruction,
                    mapped_qubits,
                    mapped_params,
                    op.label.as_deref(),
                )?;
                Ok(())
            }
        }
    }

    fn apply_param_map(param: Parameter, map: &HashMap<String, Parameter>) -> Parameter {
        param.substitute_many_simultaneous(map)
    }

    /// Computes the dense unitary matrix represented by this circuit.
    ///
    /// The first qubit in `qubits_order` is mapped to the least-significant
    /// basis-state bit. When no order is supplied, qubits are sorted by their
    /// numeric identifiers. The returned matrix includes the circuit's global
    /// phase.
    ///
    /// The matrix dimension is `2^n` and its element count is `4^n` for `n`
    /// qubits. This API is therefore intended only for small circuits.
    ///
    /// # Errors
    ///
    /// Returns an error when the circuit contains a non-unitary operation,
    /// unresolved symbolic parameter, invalid qubit order, unsupported gate
    /// definition, or a matrix dimension that cannot be represented.
    pub fn to_matrix(
        &self,
        qubits_order: Option<&[usize]>,
    ) -> Result<Array2<Complex64>, CircuitError> {
        circuit_to_matrix(self, qubits_order)
    }

    /// Returns a new circuit with the supplied symbols replaced by numeric values.
    ///
    /// Parameters that remain symbolic are simplified and re-interned in the
    /// returned circuit. The original circuit is not modified.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported classical control flow, failed symbolic
    /// simplification, or an invalid stored parameter index.
    pub fn assign_parameters(
        &self,
        bindings: &Option<HashMap<&str, f64>>,
    ) -> Result<Circuit, CircuitError> {
        if self
            .data
            .iter()
            .any(|op| matches!(op.instruction, Instruction::ClassicalControl(_)))
        {
            return Err(CircuitError::InvalidOperation(
                "assigning parameters in circuits with classical control is not yet supported"
                    .to_string(),
            ));
        }

        let mut new_circuit = Circuit::from_qubits(self.qubits())?;
        new_circuit.classical_vars = self.classical_vars.clone();
        new_circuit.classical_values = self.classical_values.clone();

        // Map from old parameter index to new CircuitParam (either Fixed or Index)
        let mut index_map: Vec<CircuitParam> = Vec::with_capacity(self.parameters.len());

        for param in self.parameters.iter() {
            if let Ok(val) = param.evaluate(bindings) {
                index_map.push(CircuitParam::Fixed(val));
                continue;
            }

            let mut tp = param.clone();
            if let Some(bindings) = bindings {
                for (k, v) in bindings.iter() {
                    tp = tp.replace(k, Parameter::from(*v));
                }
                let s = tp.simplify();
                tp = s.map_err(|e| CircuitError::UnresolvedParameter(format!("{:?}", e)))?;
            }

            if tp.get_symbols().is_empty() {
                index_map.push(CircuitParam::Fixed(tp.evaluate(&None)?));
            } else {
                // Intern the new parameter (deduplicates automatically)
                let (idx, is_new) = new_circuit.parameters.insert_full(tp.clone());

                // If it's a new symbolic parameter, track its symbols
                if is_new {
                    for sym in tp.get_symbols() {
                        new_circuit.symbols.insert(sym);
                    }
                }
                index_map.push(CircuitParam::Index(idx as u32));
            }
        }

        // Remap operations to use new parameter indices or fixed values
        let new_data = Arc::make_mut(&mut new_circuit.data);
        new_data.reserve(self.data.len());
        for op in self.data.iter() {
            let mut new_op = op.clone();
            for p in &mut new_op.params {
                if let CircuitParam::Index(old_idx) = p {
                    *p = index_map
                        .get(*old_idx as usize)
                        .cloned()
                        .ok_or(CircuitError::InvalidParameterIndex(*old_idx))?;
                }
            }
            new_data.push(new_op);
        }

        // Remap global phase
        match self.global_phase {
            CircuitParam::Index(old_idx) => {
                new_circuit.global_phase = index_map
                    .get(old_idx as usize)
                    .cloned()
                    .ok_or(CircuitError::InvalidParameterIndex(old_idx))?;
            }
            CircuitParam::Fixed(val) => {
                new_circuit.global_phase = CircuitParam::Fixed(val);
            }
        }

        Ok(new_circuit)
    }

    fn remap_compose_operation(
        op: &Operation,
        qubit_mapping: &HashMap<Qubit, Qubit>,
        param_index_map: &[CircuitParam],
        var_map: &HashMap<ClassicalVar, ClassicalVar>,
        value_map: &HashMap<ClassicalValue, ClassicalValue>,
    ) -> Result<Operation, CircuitError> {
        Self::remap_operation(
            op,
            &|qubit| {
                qubit_mapping
                    .get(&qubit)
                    .copied()
                    .ok_or(CircuitError::QubitNotFound(qubit.id()))
            },
            &|param| match param {
                CircuitParam::Index(old_idx) => param_index_map
                    .get(*old_idx as usize)
                    .cloned()
                    .ok_or(CircuitError::InvalidParameterIndex(*old_idx)),
                CircuitParam::Fixed(value) => Ok(CircuitParam::Fixed(*value)),
            },
            var_map,
            value_map,
        )
    }

    fn remap_operation<QubitMapper, ParamMapper>(
        op: &Operation,
        map_qubit: &QubitMapper,
        map_param: &ParamMapper,
        var_map: &HashMap<ClassicalVar, ClassicalVar>,
        value_map: &HashMap<ClassicalValue, ClassicalValue>,
    ) -> Result<Operation, CircuitError>
    where
        QubitMapper: Fn(Qubit) -> Result<Qubit, CircuitError>,
        ParamMapper: Fn(&CircuitParam) -> Result<CircuitParam, CircuitError>,
    {
        let qubits = op
            .qubits
            .iter()
            .copied()
            .map(map_qubit)
            .collect::<Result<_, _>>()?;
        let params = op.params.iter().map(map_param).collect::<Result<_, _>>()?;
        let instruction = match &op.instruction {
            Instruction::ClassicalData(classical_op) => Instruction::ClassicalData(
                Self::remap_classical_data_op(classical_op, var_map, value_map)?,
            ),
            Instruction::ClassicalControl(op) => Instruction::ClassicalControl(
                Self::remap_control_op(op, map_qubit, map_param, var_map, value_map)?,
            ),
            _ => op.instruction.clone(),
        };

        Ok(Operation {
            instruction,
            qubits,
            params,
            label: op.label.clone(),
        })
    }

    fn remap_classical_data_op(
        op: &ClassicalDataOp,
        var_map: &HashMap<ClassicalVar, ClassicalVar>,
        value_map: &HashMap<ClassicalValue, ClassicalValue>,
    ) -> Result<ClassicalDataOp, CircuitError> {
        match op {
            ClassicalDataOp::Store { target, value } => Ok(ClassicalDataOp::Store {
                target: var_map.get(target).copied().ok_or_else(|| {
                    CircuitError::InvalidOperation(format!(
                        "missing classical variable remap for id {}",
                        target.id()
                    ))
                })?,
                value: value.remap_classical_ids(var_map, value_map)?,
            }),
            ClassicalDataOp::MeasureBit { result } => Ok(ClassicalDataOp::MeasureBit {
                result: value_map.get(result).copied().ok_or_else(|| {
                    CircuitError::InvalidOperation(format!(
                        "missing classical value remap for id {}",
                        result.index()
                    ))
                })?,
            }),
            ClassicalDataOp::MeasureBits { result } => Ok(ClassicalDataOp::MeasureBits {
                result: value_map.get(result).copied().ok_or_else(|| {
                    CircuitError::InvalidOperation(format!(
                        "missing classical value remap for id {}",
                        result.index()
                    ))
                })?,
            }),
        }
    }

    fn remap_control_body<QubitMapper, ParamMapper>(
        body: &ControlBody,
        map_qubit: &QubitMapper,
        map_param: &ParamMapper,
        var_map: &HashMap<ClassicalVar, ClassicalVar>,
        value_map: &HashMap<ClassicalValue, ClassicalValue>,
    ) -> Result<ControlBody, CircuitError>
    where
        QubitMapper: Fn(Qubit) -> Result<Qubit, CircuitError>,
        ParamMapper: Fn(&CircuitParam) -> Result<CircuitParam, CircuitError>,
    {
        body.operations()
            .iter()
            .map(|op| Self::remap_operation(op, map_qubit, map_param, var_map, value_map))
            .collect::<Result<Vec<_>, _>>()
            .map(ControlBody::new)
    }

    fn remap_control_op<QubitMapper, ParamMapper>(
        op: &ClassicalControlOp,
        map_qubit: &QubitMapper,
        map_param: &ParamMapper,
        var_map: &HashMap<ClassicalVar, ClassicalVar>,
        value_map: &HashMap<ClassicalValue, ClassicalValue>,
    ) -> Result<ClassicalControlOp, CircuitError>
    where
        QubitMapper: Fn(Qubit) -> Result<Qubit, CircuitError>,
        ParamMapper: Fn(&CircuitParam) -> Result<CircuitParam, CircuitError>,
    {
        match op {
            ClassicalControlOp::If(op) => {
                let condition = op.condition().remap_classical_ids(var_map, value_map)?;
                let then_body = Self::remap_control_body(
                    op.then_body(),
                    map_qubit,
                    map_param,
                    var_map,
                    value_map,
                )?;
                let else_body = op
                    .else_body()
                    .map(|body| {
                        Self::remap_control_body(body, map_qubit, map_param, var_map, value_map)
                    })
                    .transpose()?;
                IfOp::new(condition, then_body, else_body).map(ClassicalControlOp::If)
            }
            ClassicalControlOp::While(op) => {
                let condition = op.condition().remap_classical_ids(var_map, value_map)?;
                let body =
                    Self::remap_control_body(op.body(), map_qubit, map_param, var_map, value_map)?;
                WhileOp::new(condition, body).map(ClassicalControlOp::While)
            }
            ClassicalControlOp::For(op) => {
                let var = var_map.get(&op.var()).copied().ok_or_else(|| {
                    CircuitError::InvalidOperation(format!(
                        "missing classical variable remap for id {}",
                        op.var().id()
                    ))
                })?;
                let start = op.start().remap_classical_ids(var_map, value_map)?;
                let stop = op.stop().remap_classical_ids(var_map, value_map)?;
                let step = op.step().remap_classical_ids(var_map, value_map)?;
                let body =
                    Self::remap_control_body(op.body(), map_qubit, map_param, var_map, value_map)?;
                ForOp::new(var, start, stop, step, body).map(ClassicalControlOp::For)
            }
            ClassicalControlOp::Switch(op) => {
                let target = op.target().remap_classical_ids(var_map, value_map)?;
                let cases = op
                    .cases()
                    .iter()
                    .map(|case| {
                        Ok(SwitchCase::new(
                            case.value(),
                            Self::remap_control_body(
                                case.body(),
                                map_qubit,
                                map_param,
                                var_map,
                                value_map,
                            )?,
                        ))
                    })
                    .collect::<Result<Vec<_>, CircuitError>>()?;
                let default = op
                    .default()
                    .map(|body| {
                        Self::remap_control_body(body, map_qubit, map_param, var_map, value_map)
                    })
                    .transpose()?;
                SwitchOp::new(target, cases, default).map(ClassicalControlOp::Switch)
            }
            ClassicalControlOp::Break => Ok(ClassicalControlOp::Break),
            ClassicalControlOp::Continue => Ok(ClassicalControlOp::Continue),
        }
    }

    /// Composes another circuit into this circuit.
    ///
    /// This method merges the operations from `other` circuit into `self`. Qubits from `other`
    /// can either be mapped to existing qubits in `self` (via `qubits_map`) or merged by their
    /// [`Qubit`] IDs.
    /// Circuit-local classical variables and values from `other` are appended to `self`'s
    /// classical tables, and all classical data/control references inside copied operations are
    /// remapped to those new local IDs.
    ///
    /// # Arguments
    ///
    /// * `other` - The circuit to compose into this circuit.
    /// * `qubits_map` - An optional slice mapping qubits from `other` to qubits in `self`.
    ///   - If `Some(mapping)` is provided, each qubit in `other` (in their natural iteration order)
    ///     is mapped to the corresponding qubit in `mapping`.
    ///   - If `None` is provided, qubits are mapped by ID. IDs not already present in `self` are
    ///     appended, while matching IDs reuse the existing qubits.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If composition succeeds.
    /// * `Err(CircuitError)` - If the mapping is invalid (wrong length or non-existent qubits).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::circuit_impl::Circuit;
    /// use cqlib_core::circuit::Qubit;
    ///
    /// // Create qc1 with qubits 1, 3, 5
    /// let mut qc1 = Circuit::new(0);
    /// let q1 = Qubit::new(1);
    /// let q3 = Qubit::new(3);
    /// let q5 = Qubit::new(5);
    /// qc1.add_qubits(vec![q1, q3, q5]).unwrap();
    /// qc1.h(q1).unwrap();
    ///
    /// // Create qc2 with qubits 1, 2
    /// let mut qc2 = Circuit::new(0);
    /// let q2 = Qubit::new(2);
    /// qc2.add_qubits(vec![q1, q2]).unwrap();
    /// qc2.x(q1).unwrap();
    ///
    /// // Compose qc2 into qc1, mapping qc2's qubits: q1->q3, q2->q1
    /// qc1.compose(&qc2, Some(&[q3, q1])).unwrap();
    /// ```
    pub fn compose(
        &mut self,
        other: &Circuit,
        qubits_map: Option<&[Qubit]>,
    ) -> Result<(), CircuitError> {
        // Build qubit mapping: other_qubit -> target_qubit
        let qubit_mapping: HashMap<Qubit, Qubit> = if let Some(mapping) = qubits_map {
            // Validate mapping length
            if mapping.len() != other.qubits.len() {
                return Err(CircuitError::QubitCountMismatch {
                    expected: other.qubits.len(),
                    actual: mapping.len(),
                });
            }

            // Build map and validate target qubits exist in self
            let mut map = HashMap::with_capacity(mapping.len());
            for (other_qubit, target_qubit) in other.qubits.iter().zip(mapping.iter()) {
                if !self.qubits.contains(target_qubit) {
                    return Err(CircuitError::QubitNotFound(target_qubit.id()));
                }
                map.insert(*other_qubit, *target_qubit);
            }
            map
        } else {
            // No explicit mapping: merge qubits by ID.
            let mut map = HashMap::with_capacity(other.qubits.len());
            for other_qubit in other.qubits.iter() {
                // Existing IDs are reused; new IDs preserve `other`'s insertion order.
                self.qubits.insert(*other_qubit);
                map.insert(*other_qubit, *other_qubit);
            }
            map
        };

        // Build circuit-local classical ID mappings. Handles from `other` are
        // remapped to the appended table range owned by `self`.
        let var_base = self.classical_vars.len();
        let mut var_map = HashMap::with_capacity(other.classical_vars.len());
        for (idx, ty) in other.classical_vars.iter().enumerate() {
            var_map.insert(
                ClassicalVar::new(other.circuit_id, idx as u32, *ty),
                ClassicalVar::new(self.circuit_id, (var_base + idx) as u32, *ty),
            );
        }

        let value_base = self.classical_values.len();
        let mut value_map = HashMap::with_capacity(other.classical_values.len());
        for (idx, ty) in other.classical_values.iter().enumerate() {
            value_map.insert(
                ClassicalValue::new(other.circuit_id, idx as u32, *ty),
                ClassicalValue::new(self.circuit_id, (value_base + idx) as u32, *ty),
            );
        }

        // Merge parameters and build index mapping
        let mut param_index_map: Vec<CircuitParam> = Vec::with_capacity(other.parameters.len());
        for param in other.parameters.iter() {
            let (idx, _) = self.add_parameter(param.clone());
            param_index_map.push(CircuitParam::Index(idx as u32));
        }

        let remapped_ops = other
            .data
            .iter()
            .map(|op| {
                Self::remap_compose_operation(
                    op,
                    &qubit_mapping,
                    &param_index_map,
                    &var_map,
                    &value_map,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Merge global phase (if both have fixed values, add them; otherwise keep symbolic)
        self.global_phase = match (self.global_phase.clone(), other.global_phase.clone()) {
            (CircuitParam::Fixed(a), CircuitParam::Fixed(b)) => CircuitParam::Fixed(a + b),
            (CircuitParam::Fixed(a), CircuitParam::Index(idx)) => {
                // Add fixed value to symbolic parameter
                let sym_param = other
                    .parameters
                    .get_index(idx as usize)
                    .cloned()
                    .ok_or(CircuitError::InvalidParameterIndex(idx))?;
                let new_expr = Parameter::from(a) + sym_param;
                let (new_idx, _) = self.parameters.insert_full(new_expr);
                CircuitParam::Index(new_idx as u32)
            }
            (CircuitParam::Index(idx), CircuitParam::Fixed(b)) => {
                // Add symbolic parameter to fixed value
                let sym_param = self
                    .parameters
                    .get_index(idx as usize)
                    .cloned()
                    .ok_or(CircuitError::InvalidParameterIndex(idx))?;
                let new_expr = sym_param + Parameter::from(b);
                let (new_idx, _) = self.parameters.insert_full(new_expr);
                CircuitParam::Index(new_idx as u32)
            }
            (CircuitParam::Index(idx_a), CircuitParam::Index(idx_b)) => {
                // Add two symbolic parameters
                let param_a = self
                    .parameters
                    .get_index(idx_a as usize)
                    .cloned()
                    .ok_or(CircuitError::InvalidParameterIndex(idx_a))?;
                let param_b = other
                    .parameters
                    .get_index(idx_b as usize)
                    .cloned()
                    .ok_or(CircuitError::InvalidParameterIndex(idx_b))?;
                // param_b needs to be merged into self's parameter set first
                let (merged_b_idx, _) = self.parameters.insert_full(param_b);
                let merged_b = self
                    .parameters
                    .get_index(merged_b_idx)
                    .cloned()
                    .ok_or(CircuitError::InvalidParameterIndex(merged_b_idx as u32))?;
                let new_expr = param_a + merged_b;
                let (new_idx, _) = self.parameters.insert_full(new_expr);
                CircuitParam::Index(new_idx as u32)
            }
        };

        self.classical_vars
            .extend(other.classical_vars.iter().copied());
        self.classical_values
            .extend(other.classical_values.iter().copied());
        let data = Arc::make_mut(&mut self.data);
        data.reserve(remapped_ops.len());
        data.extend(remapped_ops);

        Ok(())
    }

    /// Returns an operation with its parameters resolved to value-level representations.
    ///
    /// Unlike [`Circuit::operations`], which exposes the compact storage IR,
    /// this method converts indexed parameters back into [`ParameterValue`]s.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::OperationIndexOutOfBounds`] when `i` is not a
    /// valid operation index. Returns [`CircuitError::InvalidParameterIndex`]
    /// if the stored operation references a missing entry in this circuit's
    /// parameter table.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::{Circuit, CircuitError};
    ///
    /// let circuit = Circuit::new(1);
    /// assert!(matches!(
    ///     circuit.index(0),
    ///     Err(CircuitError::OperationIndexOutOfBounds { index: 0, len: 0 })
    /// ));
    /// ```
    pub fn index(&self, i: usize) -> Result<ValueOperation, CircuitError> {
        let operation =
            self.data
                .get(i)
                .cloned()
                .ok_or(CircuitError::OperationIndexOutOfBounds {
                    index: i,
                    len: self.data.len(),
                })?;
        storage_operation_to_value(operation, &|param| self.parameter_value(param))
    }
}

impl Circuit {
    fn lower_instruction(
        &mut self,
        instruction: ValueInstruction,
    ) -> Result<Instruction, CircuitError> {
        let op = match instruction {
            ValueInstruction::Instruction(Instruction::ClassicalControl(_)) => {
                return Err(CircuitError::InvalidOperation(
                    "ValueInstruction::Instruction cannot wrap Instruction::ClassicalControl"
                        .to_string(),
                ));
            }
            ValueInstruction::Instruction(instruction) => return Ok(instruction),
            ValueInstruction::ClassicalControl(op) => op,
        };

        let op = match op {
            ValueClassicalControlOp::If {
                condition,
                then_body,
                else_body,
            } => {
                let then_body = self.lower_body(then_body)?;
                let else_body = else_body.map(|body| self.lower_body(body)).transpose()?;
                IfOp::new(condition, then_body, else_body).map(ClassicalControlOp::If)?
            }
            ValueClassicalControlOp::While { condition, body } => {
                let body = self.lower_body(body)?;
                WhileOp::new(condition, body).map(ClassicalControlOp::While)?
            }
            ValueClassicalControlOp::For {
                var,
                start,
                stop,
                step,
                body,
            } => {
                let body = self.lower_body(body)?;
                ForOp::new(var, start, stop, step, body).map(ClassicalControlOp::For)?
            }
            ValueClassicalControlOp::Switch {
                target,
                cases,
                default,
            } => {
                let cases = cases
                    .into_iter()
                    .map(|case| Ok(SwitchCase::new(case.value, self.lower_body(case.body)?)))
                    .collect::<Result<Vec<_>, CircuitError>>()?;
                let default = default.map(|body| self.lower_body(body)).transpose()?;
                SwitchOp::new(target, cases, default).map(ClassicalControlOp::Switch)?
            }
            ValueClassicalControlOp::Break => ClassicalControlOp::Break,
            ValueClassicalControlOp::Continue => ClassicalControlOp::Continue,
        };
        Ok(Instruction::ClassicalControl(op))
    }

    fn lower_operation(&mut self, operation: ValueOperation) -> Result<Operation, CircuitError> {
        let instruction = self.lower_instruction(operation.instruction)?;
        let params = operation
            .params
            .into_iter()
            .enumerate()
            .map(
                |(param_index, param)| -> Result<CircuitParam, CircuitError> {
                    match param {
                        ParameterValue::Param(param) => {
                            let (index, is_new) = self.parameters.insert_full(param.clone());
                            if is_new {
                                for sym in param.get_symbols() {
                                    self.symbols.insert(sym);
                                }
                            }
                            Ok(CircuitParam::Index(index as u32))
                        }
                        ParameterValue::Fixed(value) => {
                            if !value.is_finite() {
                                return Err(CircuitError::InvalidParameterValue(
                                    param_index,
                                    value,
                                ));
                            }
                            Ok(CircuitParam::Fixed(value))
                        }
                    }
                },
            )
            .collect::<Result<_, _>>()?;
        Ok(Operation {
            instruction,
            qubits: operation.qubits,
            params,
            label: operation.label,
        })
    }

    fn lower_body(&mut self, body: ValueControlBody) -> Result<ControlBody, CircuitError> {
        body.operations()
            .iter()
            .cloned()
            .map(|operation| self.lower_operation(operation))
            .collect::<Result<Vec<_>, _>>()
            .map(ControlBody::new)
    }

    fn validate_operation_parameters(&self, operations: &[Operation]) -> Result<(), CircuitError> {
        for operation in operations {
            for param in &operation.params {
                match param {
                    CircuitParam::Fixed(value) => {
                        if !value.is_finite() {
                            return Err(CircuitError::InvalidParameterValue(0, *value));
                        }
                    }
                    CircuitParam::Index(index) => {
                        if self.parameters.get_index(*index as usize).is_none() {
                            return Err(CircuitError::InvalidParameterIndex(*index));
                        }
                    }
                }
            }
            if let Instruction::ClassicalControl(op) = &operation.instruction {
                match op {
                    ClassicalControlOp::If(op) => {
                        self.validate_operation_parameters(op.then_body().operations())?;
                        if let Some(body) = op.else_body() {
                            self.validate_operation_parameters(body.operations())?;
                        }
                    }
                    ClassicalControlOp::While(op) => {
                        self.validate_operation_parameters(op.body().operations())?;
                    }
                    ClassicalControlOp::For(op) => {
                        self.validate_operation_parameters(op.body().operations())?;
                    }
                    ClassicalControlOp::Switch(op) => {
                        for case in op.cases() {
                            self.validate_operation_parameters(case.body().operations())?;
                        }
                        if let Some(body) = op.default() {
                            self.validate_operation_parameters(body.operations())?;
                        }
                    }
                    ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
                }
            }
        }
        Ok(())
    }
}

fn infer_classical_circuit_id(
    operations: &[ValueOperation],
) -> Result<Option<CircuitId>, CircuitError> {
    fn collect_instruction(instruction: &Instruction, identities: &mut HashSet<CircuitId>) {
        match instruction {
            Instruction::ClassicalData(op) => match op {
                ClassicalDataOp::Store { target, value } => {
                    identities.insert(target.circuit_id());
                    identities.extend(value.vars().into_iter().map(ClassicalVar::circuit_id));
                    identities.extend(value.values().into_iter().map(ClassicalValue::circuit_id));
                }
                ClassicalDataOp::MeasureBit { result }
                | ClassicalDataOp::MeasureBits { result } => {
                    identities.insert(result.circuit_id());
                }
            },
            Instruction::ClassicalControl(op) => {
                identities.extend(
                    op.classical_var_reads()
                        .into_iter()
                        .map(ClassicalVar::circuit_id),
                );
                identities.extend(
                    op.classical_value_reads()
                        .into_iter()
                        .map(ClassicalValue::circuit_id),
                );
                identities.extend(
                    op.classical_writes()
                        .into_iter()
                        .map(ClassicalVar::circuit_id),
                );
                match op {
                    ClassicalControlOp::If(op) => {
                        for operation in op
                            .then_body()
                            .operations()
                            .iter()
                            .chain(op.else_body().into_iter().flat_map(ControlBody::operations))
                        {
                            collect_instruction(&operation.instruction, identities);
                        }
                    }
                    ClassicalControlOp::While(op) => {
                        for operation in op.body().operations() {
                            collect_instruction(&operation.instruction, identities);
                        }
                    }
                    ClassicalControlOp::For(op) => {
                        for operation in op.body().operations() {
                            collect_instruction(&operation.instruction, identities);
                        }
                    }
                    ClassicalControlOp::Switch(op) => {
                        for operation in op
                            .cases()
                            .iter()
                            .flat_map(|case| case.body().operations())
                            .chain(op.default().into_iter().flat_map(ControlBody::operations))
                        {
                            collect_instruction(&operation.instruction, identities);
                        }
                    }
                    ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
                }
            }
            _ => {}
        }
    }

    fn collect_value(instruction: &ValueInstruction, identities: &mut HashSet<CircuitId>) {
        match instruction {
            ValueInstruction::Instruction(instruction) => {
                collect_instruction(instruction, identities);
            }
            ValueInstruction::ClassicalControl(op) => {
                identities.extend(
                    op.classical_var_reads()
                        .into_iter()
                        .map(ClassicalVar::circuit_id),
                );
                identities.extend(
                    op.classical_value_reads()
                        .into_iter()
                        .map(ClassicalValue::circuit_id),
                );
                identities.extend(
                    op.classical_writes()
                        .into_iter()
                        .map(ClassicalVar::circuit_id),
                );
                match op {
                    ValueClassicalControlOp::If {
                        then_body,
                        else_body,
                        ..
                    } => {
                        for operation in then_body
                            .operations()
                            .iter()
                            .chain(else_body.iter().flat_map(|body| body.operations()))
                        {
                            collect_value(&operation.instruction, identities);
                        }
                    }
                    ValueClassicalControlOp::While { body, .. }
                    | ValueClassicalControlOp::For { body, .. } => {
                        for operation in body.operations() {
                            collect_value(&operation.instruction, identities);
                        }
                    }
                    ValueClassicalControlOp::Switch { cases, default, .. } => {
                        for operation in cases
                            .iter()
                            .flat_map(|case| case.body.operations())
                            .chain(default.iter().flat_map(|body| body.operations()))
                        {
                            collect_value(&operation.instruction, identities);
                        }
                    }
                    ValueClassicalControlOp::Break | ValueClassicalControlOp::Continue => {}
                }
            }
        }
    }

    let mut identities = HashSet::new();
    for operation in operations {
        collect_value(&operation.instruction, &mut identities);
    }
    if identities.len() > 1 {
        return Err(CircuitError::InvalidOperation(
            "operations contain classical handles from multiple circuits".to_string(),
        ));
    }
    Ok(identities.into_iter().next())
}

#[cfg(test)]
#[path = "./circuit_test.rs"]
mod circuit_test;
