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

//! # Circuit Operation Module
//!
//! This module defines the [`Operation`] struct, which represents a single, concrete execution step
//! within a quantum circuit.
//!
//! ## Role in Architecture
//!
//! While [`Instruction`] defines *what* operation to perform (e.g., "apply a Hadamard gate"),
//! [`Operation`] defines *where* and *how* to apply it. It binds an abstract instruction to:
//! - Specific qubits (Topology).
//! - Specific parameters (Context).
//!
//! ## Memory Optimization
//!
//! Since a circuit may contain millions of operations, this struct is heavily optimized for memory compactness:
//! - **SmallVec**: Uses `SmallVec` for qubits and parameters to store data inline on the stack for common cases
//!   (e.g., 1-2 qubit gates, 0-1 parameters), avoiding heap allocation overhead.
//! - **CircuitParam**: Uses a lightweight enum to store parameters, supporting both immediate float values
//!   and references to the circuit's global parameter table (interning).

use crate::circuit::bit::Qubit;
use crate::circuit::circuit_param::{CircuitParam, ParameterValue};
use crate::circuit::error::CircuitError;
use crate::circuit::gate::{StandardGate, instruction::Instruction};
use crate::circuit::value_instruction::ValueInstruction;
use alloc::borrow::Cow;
use ndarray::Array2;
use num_complex::Complex64;
use smallvec::{SmallVec, smallvec};
use std::fmt;

/// A circuit-local operation in the compact storage IR.
///
/// An `Operation` combines a gate (instruction) with the specific qubits it acts upon and its
/// parameters. Fixed parameters are stored inline, while symbolic parameters may be represented
/// by indices into the owning [`Circuit`](crate::circuit::Circuit)'s parameter table. An
/// `Operation` therefore cannot always be interpreted without its owning circuit.
///
/// # Fields
///
/// * `instruction` - The type of operation (e.g., `StandardGate::H`, `Directive::Measure`).
/// * `qubits` - The ordered list of qubits involved in this operation.
///   - For controlled gates, control qubits usually come first, followed by target qubits.
///   - Implementation uses `SmallVec<[Qubit; 3]>` to optimize for gates acting on ≤3 qubits (covering almost all standard gates).
/// * `params` - The parameters for the operation.
///   - Implementation uses `SmallVec<[CircuitParam; 1]>` to optimize for single-parameter gates (e.g., `RX`, `RZ`).
/// * `label` - An optional human-readable label or tag for this specific operation instance.
#[derive(Debug, Clone)]
pub struct Operation {
    /// The abstract instruction definition (what to do).
    pub instruction: Instruction,
    /// The specific qubits this operation applies to (where to do it).
    pub qubits: SmallVec<[Qubit; 3]>,
    /// The concrete or symbolic parameters for this operation (how to do it).
    pub params: SmallVec<[CircuitParam; 1]>,
    /// Optional metadata label.
    pub label: Option<Box<str>>,
}

impl PartialEq for Operation {
    /// Compares the storage-level structure of two operations.
    ///
    /// Parameters compare in their circuit-local storage form (see
    /// [`CircuitParam`]), and classical handles embedded in
    /// [`Instruction::ClassicalData`]/[`Instruction::ClassicalControl`]
    /// carry the owning circuit's process-local identity, so operations
    /// originating from different circuits are generally unequal unless
    /// remapped first (as [`Circuit`](crate::circuit::Circuit)'s own
    /// equality does internally).
    fn eq(&self, other: &Self) -> bool {
        self.instruction == other.instruction
            && self.qubits == other.qubits
            && self.params == other.params
            && self.label == other.label
    }
}

impl Operation {
    /// Returns the standard gate represented directly by this operation.
    pub fn standard_gate(&self) -> Option<StandardGate> {
        self.instruction.standard_gate()
    }

    /// Computes the numerical unitary matrix for this specific operation.
    ///
    /// This method accepts only parameters already stored as fixed values and delegates
    /// matrix generation to the underlying [`Instruction`]. It cannot resolve
    /// circuit-local parameter indices without access to the owning circuit.
    ///
    /// # Returns
    ///
    /// * `Ok(Cow<Array2>)` - The unitary matrix. It may be borrowed (static) or owned (computed).
    /// * `Err(CircuitError)` - If the operation is non-unitary or contains an
    ///   unresolved symbolic parameter.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::SymbolicParameterError`] when the operation
    /// contains an indexed symbolic parameter. Resolve parameters through the
    /// owning [`Circuit`](crate::circuit::Circuit) before requesting a numeric
    /// matrix. Returns [`CircuitError::NoMatrixRepresentation`] for non-unitary
    /// instructions such as measurement, reset, and barriers.
    pub fn matrix(&self) -> Result<Cow<'_, Array2<Complex64>>, CircuitError> {
        let mut ps: SmallVec<[f64; 4]> = smallvec![];
        for p in self.params.iter() {
            match p {
                CircuitParam::Fixed(val) => {
                    ps.push(*val);
                }
                CircuitParam::Index(_index) => {
                    return Err(CircuitError::SymbolicParameterError);
                }
            }
        }
        self.instruction
            .matrix(&ps)
            .ok_or(CircuitError::NoMatrixRepresentation)
    }
}

/// A value-level operation independent of a circuit parameter table.
///
/// Unlike [`Operation`], symbolic parameters are stored directly as
/// [`ParameterValue`] values rather than circuit-local table indices. This form
/// is used before insertion into a circuit and when returning a resolved
/// operation from [`Circuit::index`](crate::circuit::Circuit::index).
#[derive(Debug, Clone)]
pub struct ValueOperation {
    /// The abstract instruction definition (what to do).
    pub instruction: ValueInstruction,
    /// The specific qubits this operation applies to (where to do it).
    pub qubits: SmallVec<[Qubit; 3]>,
    /// The concrete or symbolic parameters for this operation (how to do it).
    pub params: SmallVec<[ParameterValue; 1]>,
    /// Optional metadata label.
    pub label: Option<Box<str>>,
}

impl PartialEq for ValueOperation {
    /// Compares the construction-level structure of two operations.
    ///
    /// Parameters compare in their construction form (see
    /// [`ParameterValue`]), and classical handles embedded in
    /// [`Instruction::ClassicalData`]/[`Instruction::ClassicalControl`]
    /// carry the owning circuit's process-local identity, so operations
    /// originating from different circuits are generally unequal unless
    /// remapped first (as [`Circuit`](crate::circuit::Circuit)'s own
    /// equality does internally).
    fn eq(&self, other: &Self) -> bool {
        self.instruction == other.instruction
            && self.qubits == other.qubits
            && self.params == other.params
            && self.label == other.label
    }
}

impl ValueOperation {
    /// Creates a standard gate operation.
    pub fn from_standard(
        gate: StandardGate,
        qubits: impl IntoIterator<Item = Qubit>,
        params: impl IntoIterator<Item = ParameterValue>,
    ) -> Self {
        Self {
            instruction: ValueInstruction::from_instruction(Instruction::Standard(gate)),
            qubits: qubits.into_iter().collect(),
            params: params.into_iter().collect(),
            label: None,
        }
    }

    /// Returns the human-readable instruction name.
    pub fn name(&self) -> String {
        self.instruction.name()
    }

    /// Returns the number of qubits used by this operation instance.
    pub fn num_qubits(&self) -> usize {
        self.qubits.len()
    }

    /// Returns the number of parameters carried by this operation instance.
    pub fn num_params(&self) -> usize {
        self.params.len()
    }

    /// Returns a stable category name for this operation's instruction.
    pub fn instruction_type(&self) -> &'static str {
        self.instruction.instruction_type()
    }

    /// Returns `true` if this operation uses a standard-gate instruction.
    pub fn is_standard(&self) -> bool {
        self.instruction.is_standard()
    }

    /// Returns `true` if this operation uses a multi-controlled-gate instruction.
    pub fn is_mcgate(&self) -> bool {
        self.instruction.is_mcgate()
    }

    /// Returns `true` if this operation uses a user-defined unitary instruction.
    pub fn is_unitary(&self) -> bool {
        self.instruction.is_unitary()
    }

    /// Returns `true` if this operation uses a circuit-backed gate instruction.
    pub fn is_circuit_gate(&self) -> bool {
        self.instruction.is_circuit_gate()
    }

    /// Returns `true` if this operation uses a directive instruction.
    pub fn is_directive(&self) -> bool {
        self.instruction.is_directive()
    }

    /// Returns `true` if this operation uses a classical-data instruction.
    pub fn is_classical_data(&self) -> bool {
        self.instruction.is_classical_data()
    }

    /// Returns `true` if this operation uses a classical-control instruction.
    pub fn is_classical_control(&self) -> bool {
        self.instruction.is_classical_control()
    }

    /// Returns `true` if this operation uses a delay instruction.
    pub fn is_delay(&self) -> bool {
        self.instruction.is_delay()
    }
}

impl fmt::Display for ValueOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.instruction)?;
        if !self.params.is_empty() {
            write!(f, "(")?;
            for (i, param) in self.params.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", crate::circuit::Parameter::from(param))?;
            }
            write!(f, ")")?;
        }
        for qubit in &self.qubits {
            write!(f, " {}", qubit)?;
        }
        if let Some(ref label) = self.label {
            write!(f, " [{}]", label)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_standard_gate_query_delegates_to_instruction() {
        let standard = Operation {
            instruction: Instruction::Standard(StandardGate::H),
            qubits: smallvec![Qubit::new(0)],
            params: smallvec![],
            label: None,
        };
        let directive = Operation {
            instruction: Instruction::Directive(crate::circuit::Directive::Barrier),
            qubits: smallvec![],
            params: smallvec![],
            label: None,
        };

        assert_eq!(standard.standard_gate(), Some(StandardGate::H));
        assert_eq!(directive.standard_gate(), None);
    }

    #[test]
    fn value_operation_exposes_instance_queries() {
        let operation = ValueOperation::from_standard(
            StandardGate::RX,
            [Qubit::new(0)],
            [ParameterValue::Fixed(0.5)],
        );

        assert_eq!(operation.name(), "RX");
        assert_eq!(operation.num_qubits(), 1);
        assert_eq!(operation.num_params(), 1);
        assert_eq!(operation.instruction_type(), "standard");
        assert!(operation.is_standard());
        assert!(!operation.is_directive());
        assert!(!operation.is_classical_control());
    }

    #[test]
    fn value_operation_compares_structurally() {
        let make = || {
            ValueOperation::from_standard(
                StandardGate::RX,
                [Qubit::new(0)],
                [ParameterValue::Fixed(0.5)],
            )
        };
        assert_eq!(make(), make());

        let different_param = ValueOperation::from_standard(
            StandardGate::RX,
            [Qubit::new(0)],
            [ParameterValue::Fixed(1.5)],
        );
        assert_ne!(make(), different_param);

        let different_qubit = ValueOperation::from_standard(
            StandardGate::RX,
            [Qubit::new(1)],
            [ParameterValue::Fixed(0.5)],
        );
        assert_ne!(make(), different_qubit);

        let labeled = ValueOperation {
            label: Some("tagged".into()),
            ..make()
        };
        assert_ne!(make(), labeled);
    }

    #[test]
    fn value_operation_parameter_forms_do_not_cross_compare() {
        let symbolic = ValueOperation::from_standard(
            StandardGate::RX,
            [Qubit::new(0)],
            [ParameterValue::Param(crate::circuit::Parameter::symbol(
                "theta",
            ))],
        );
        let symbolic_again = ValueOperation::from_standard(
            StandardGate::RX,
            [Qubit::new(0)],
            [ParameterValue::Param(crate::circuit::Parameter::symbol(
                "theta",
            ))],
        );
        let fixed = ValueOperation::from_standard(
            StandardGate::RX,
            [Qubit::new(0)],
            [ParameterValue::Fixed(0.5)],
        );

        assert_eq!(symbolic, symbolic_again);
        assert_ne!(symbolic, fixed);
    }
}
