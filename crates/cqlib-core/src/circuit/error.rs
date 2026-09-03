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

//! # Circuit Error Types
//!
//! This module defines error types for quantum circuit operations.
//! It provides comprehensive error handling for parameter evaluation,
//! circuit validation, and symbolic computation errors.

use symb_anafis::DiffError;
use thiserror::Error;

/// A unified error type for operations involving symbolic parameters.
#[derive(Debug, Error)]
pub enum ParameterError {
    /// Wraps symbolic errors from the underlying math engine (parsing, syntax, diff, simplify).
    #[error(transparent)]
    SymbolicError(#[from] DiffError),

    /// Indicates that a symbolic variable required for evaluation was missing.
    #[error("Symbol not found: {0}")]
    UndefinedSymbol(String),

    /// Indicates an arithmetic error where a division or modulo operation by zero occurred.
    #[error("Division by zero")]
    DivisionByZero,

    /// Indicates that a mathematical function was called with an argument outside its defined domain.
    #[error("Domain error: {0}")]
    DomainError(String),

    /// Indicates that a calculation resulted in NaN (Not a Number).
    #[error("Calculation resulted in NaN: {0}")]
    NaN(String),

    /// Indicates an error occurred during parsing.
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Errors that can occur during the numerical evaluation of symbolic parameters.
///
/// This enum captures failure modes when resolving a symbolic [`Parameter`](crate::circuit::Parameter)
/// expression into a concrete floating-point value. These errors typically
/// arise while binding a parameterized quantum circuit.
#[derive(Debug, Error)]
pub enum EvalError {
    /// Indicates that a symbolic variable required for evaluation was missing from the provided bindings.
    ///
    /// # Example
    /// evaluating `theta + phi` with bindings `{ "theta": 1.0 }` will raise `UndefinedSymbol("phi")`.
    #[error("Symbol not found: {0}")]
    UndefinedSymbol(String),

    /// Indicates an arithmetic error where a division or modulo operation by zero occurred.
    ///
    /// This includes both explicit division by zero (e.g., `x / 0.0`) and runtime zeros (e.g., `x / y` where `y = 0.0`).
    #[error("Division by zero")]
    DivisionByZero,

    /// Indicates that a mathematical function was called with an argument outside its defined domain.
    ///
    /// Common causes include:
    /// - `sqrt(x)` where `x < 0`
    /// - `ln(x)` where `x <= 0`
    /// - `asin(x)` or `acos(x)` where `|x| > 1`
    #[error("Domain error: {0}")]
    DomainError(String),

    /// Indicates that an intermediate or final calculation resulted in `NaN` (Not a Number).
    ///
    /// This serves as a catch-all for undefined numerical behaviors not covered by other variants.
    #[error("Calculation resulted in NaN: {0}")]
    NaN(String),

    /// Indicates that the expression tree exceeded the maximum allowed recursion depth during evaluation.
    ///
    /// This typically occurs with deeply nested expressions (e.g., `((((x+1)+1)+1)...)`).
    /// Consider simplifying the expression or increasing the depth limit.
    #[error("Maximum recursion depth exceeded during evaluation (depth limit: {0})")]
    MaxRecursionDepthExceeded(usize),
}

/// Errors that can occur during symbolic differentiation.
#[derive(Debug, Error)]
pub enum DerivativeError {
    /// Indicates that an expression is not differentiable with respect to a given variable.
    #[error("Cannot differentiate expression: {0}")]
    NonDifferentiable(String),
}

/// A comprehensive error type for operations involving Quantum Circuits.
///
/// This enum aggregates errors from circuit construction, validation, and manipulation.
/// It serves as the primary error type for the `Circuit` struct and its related methods.
#[derive(Debug, Error)]
pub enum CircuitError {
    /// Thrown when attempting to add qubits that share the same unique identifier (index).
    ///
    /// Qubit indices within a circuit must be unique to ensure unambiguous addressing.
    #[error("Duplicate qubits found in circuit definition")]
    DuplicateQubits,

    /// Thrown when an operation references a qubit that is not part of the circuit.
    ///
    /// All qubits used in operations (gates, measurements) must be explicitly added to the circuit
    /// via `add_qubits` or `new` before use.
    #[error("Qubit {0} not found in circuit")]
    QubitNotFound(u32),

    /// Thrown when the number of qubits provided for an operation does not match its definition.
    ///
    /// For example, applying a 2-qubit CNOT gate to 3 qubits, or a 1-qubit Custom Unitary to 2 qubits.
    #[error("Qubit count mismatch: expected {expected}, got {actual}")]
    QubitCountMismatch {
        /// Required qubit count.
        expected: usize,
        /// Supplied qubit count.
        actual: usize,
    },

    /// Thrown when the number of parameters supplied to a fixed-arity
    /// instruction does not match its definition.
    #[error("Parameter count mismatch: expected {expected}, got {actual}")]
    ParameterCountMismatch {
        /// Required parameter count.
        expected: usize,
        /// Supplied parameter count.
        actual: usize,
    },

    /// Wraps errors raised while simplifying or evaluating symbolic parameters.
    #[error("Invalid parameter: {0}")]
    InvalidParameter(#[from] ParameterError),

    /// Thrown when an operation is requested to provide a unitary matrix, but none exists.
    ///
    /// This typically happens when calling `.matrix()` on non-unitary instructions such as:
    /// - `Measure`
    /// - `Barrier`
    /// - `Reset`
    #[error("Instruction has no matrix representation")]
    NoMatrixRepresentation,

    /// Wraps errors that occur during the evaluation of circuit parameters.
    ///
    /// This variant propagates [`EvalError`]s up the stack when operations like matrix generation
    /// or gate inversion fail due to invalid parameter bindings.
    #[error("Failed to evaluate parameters: {0}")]
    ParamEvaluationFailed(#[from] EvalError),

    /// Thrown when inversion is requested for a non-unitary operation or circuit.
    #[error("Operation is irreversible")]
    IrreversibleOperation,

    /// Thrown when an instruction cannot be promoted to a controlled operation.
    #[error("Operation cannot be controlled: {0}")]
    InvalidControlOperation(String),

    /// Thrown when a numeric result is requested from an operation whose
    /// circuit-local symbolic parameters have not been resolved.
    #[error("Symbolic parameter cannot be evaluated in this context")]
    SymbolicParameterError,

    /// Thrown when a parameter cannot be resolved during circuit decomposition.
    ///
    /// This occurs when a sub-circuit or control flow body contains symbolic parameters
    /// that are not bound to concrete values and cannot be evaluated.
    #[error("Unresolved parameter in decomposition: {0}")]
    UnresolvedParameter(String),

    /// Thrown when a parameter index is out of range.
    ///
    /// This indicates internal data corruption or an inconsistency between the
    /// circuit's parameter table and the indices stored in operations.
    #[error("Parameter index {0} out of range")]
    InvalidParameterIndex(u32),

    /// Thrown when an operation index is outside the circuit operation list.
    ///
    /// `index` is the requested zero-based index and `len` is the operation
    /// count at the time of the request.
    #[error("Operation index {index} out of bounds for circuit with {len} operations")]
    OperationIndexOutOfBounds {
        /// Requested zero-based operation index.
        index: usize,
        /// Number of operations in the circuit.
        len: usize,
    },

    /// Thrown when deleting an operation would remove a classical value that
    /// is still read by another operation.
    #[error("classical value {index} is still used at {context}")]
    ClassicalValueStillInUse {
        /// Circuit-local value index.
        index: u32,
        /// Description of the remaining use site.
        context: String,
    },

    /// Thrown when a gate parameter has an invalid value (NaN or Infinity).
    ///
    /// This occurs when appending or evaluating a gate with a non-finite fixed
    /// parameter. The tuple stores the parameter position and rejected value.
    #[error("Invalid parameter value at index {0}: {1}")]
    InvalidParameterValue(usize, f64),

    /// Thrown when the control flow graph (CFG) has an invalid or inconsistent structure.
    ///
    /// This error indicates that the DAG representation of the circuit is malformed,
    /// which can occur when:
    /// - A Branch terminator lacks required TrueBranch or FalseBranch edges
    /// - A Jump terminator references a non-existent block
    /// - The entry block is not set
    /// - Edges reference nodes that don't exist in the graph
    ///
    /// # Examples
    ///
    /// This error is returned when converting an invalid `CircuitDag` back to a `Circuit`
    /// if the DAG structure was manually modified and left in an inconsistent state.
    #[error("Invalid control flow graph structure: {0}")]
    InvalidControlFlow(String),

    /// Thrown when the circuit dependency DAG has an invalid or inconsistent structure.
    #[error("Invalid circuit DAG structure: {0}")]
    InvalidDag(String),

    /// Thrown when a classical handle was created by a different circuit.
    #[error("{kind} {index} belongs to another circuit")]
    ForeignClassicalHandle {
        /// Human-readable handle kind.
        kind: &'static str,
        /// Circuit-local handle index.
        index: u32,
    },

    /// Thrown when a classical value is read before it is available at a use site.
    #[error("classical value {index} is not available at {context}")]
    UndefinedClassicalValue {
        /// Circuit-local value index.
        index: u32,
        /// Description of the invalid use site.
        context: String,
    },

    /// Thrown when validation finds multiple producers for one immutable value.
    #[error("classical value {index} has more than one definition: {first} and {second}")]
    DuplicateClassicalValueDefinition {
        /// Circuit-local value index.
        index: u32,
        /// Description of the first definition site.
        first: String,
        /// Description of the conflicting definition site.
        second: String,
    },

    /// Thrown when a classical value is used outside its defining control-flow region.
    #[error("classical value {index} escapes its defining control-flow region at {context}")]
    ClassicalValueOutOfScope {
        /// Circuit-local value index.
        index: u32,
        /// Description of the invalid use site.
        context: String,
    },

    /// Thrown when `break` or `continue` is followed by another operation in the same body.
    #[error("{operation} must be the final operation in {context}")]
    NonTerminalControlTransfer {
        /// Control-transfer operation name.
        operation: &'static str,
        /// Description of the containing body.
        context: String,
    },

    /// Thrown when `Circuit::depth(recurse=false)` is called on a circuit that
    /// contains any classical control-flow operation (`if`/`while`/`for`/
    /// `switch`/`break`/`continue`) at any nesting depth. Pass `recurse = true`
    /// to obtain an estimated depth that unfolds control flow.
    #[error("circuit contains control-flow operations; pass recurse=true for an estimated depth")]
    ControlFlowPresent,

    /// Catch-all for an invalid operation that has no more specific error variant.
    #[error("Invalid Operation: {0}")]
    InvalidOperation(String),
}

/// Legacy compilation error placeholder.
#[derive(Debug, Error)]
pub enum CompileError {
    /// Unclassified compilation failure.
    #[error("error")]
    Error,
}
