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

//! Runtime classical storage for circuits.
//!
//! This module defines the first layer of the dynamic control-flow model:
//! typed classical values and storage locations that exist only while a
//! circuit executes. A [`ClassicalValue`] is an immutable runtime result,
//! similar to an SSA value. A [`ClassicalVar`] is a circuit-local handle to a
//! mutable runtime storage location. Control-flow expressions may read both,
//! while store operations may only write variables.

use std::fmt;
use std::num::{NonZeroU32, NonZeroU64};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::circuit::bit::Qubit;
use crate::circuit::classical_expr::ClassicalExpr;
use crate::device::Outcome;
use crate::qis::error::QisError;
use smallvec::SmallVec;

static NEXT_CIRCUIT_ID: AtomicU64 = AtomicU64::new(1);

/// Process-local identity for classical handles owned by a circuit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CircuitId(NonZeroU64);

impl fmt::Display for CircuitId {
    /// Formats the process-local identity as `CircuitId(<id>)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CircuitId({})", self.0)
    }
}

impl CircuitId {
    /// Allocates a new process-local circuit identity.
    pub fn new() -> Self {
        let id = NEXT_CIRCUIT_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1).filter(|next| *next != u64::MAX)
            })
            .unwrap_or_else(|_| panic!("circuit identity space exhausted"));
        Self(NonZeroU64::new(id).expect("circuit identity counter must remain non-zero"))
    }
}

impl Default for CircuitId {
    fn default() -> Self {
        Self::new()
    }
}

/// Static type of a runtime classical variable.
///
/// `Bit` and `BitVec` are the direct targets of measurement operations.
/// `Bool` is kept distinct from `Bit` so control-flow predicates can require
/// explicit boolean expressions. `UInt` represents an unsigned integer with a
/// fixed bit width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ClassicalType {
    /// A single measured or assigned bit.
    Bit,
    /// A logical boolean value.
    Bool,
    /// An unsigned integer with the given non-zero bit width.
    UInt(NonZeroU32),
    /// An ordered bit-vector with the given non-zero bit width.
    BitVec(NonZeroU32),
}

impl ClassicalType {
    /// Creates an unsigned integer type with `width` bits.
    ///
    /// Returns `None` when `width` is zero.
    pub fn uint(width: u32) -> Option<Self> {
        NonZeroU32::new(width).map(Self::UInt)
    }

    /// Creates a bit-vector type with `width` bits.
    ///
    /// Returns `None` when `width` is zero.
    pub fn bit_vec(width: u32) -> Option<Self> {
        NonZeroU32::new(width).map(Self::BitVec)
    }

    /// Returns the number of bits used to represent values of this type.
    pub fn width(self) -> u32 {
        match self {
            Self::Bit | Self::Bool => 1,
            Self::UInt(width) | Self::BitVec(width) => width.get(),
        }
    }

    /// Returns the zero literal for this type.
    ///
    /// Width-bearing literals currently support widths up to 128 bits.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError`](crate::circuit::CircuitError) when this is a
    /// `UInt` or `BitVec` wider than the 128-bit literal representation.
    pub fn zero_literal(self) -> Result<ClassicalExpr, crate::circuit::CircuitError> {
        match self {
            Self::Bool => Ok(ClassicalExpr::bool_literal(false)),
            Self::Bit => Ok(ClassicalExpr::bit_literal(false)),
            Self::UInt(width) => ClassicalExpr::uint_literal(width.get(), 0),
            Self::BitVec(width) => ClassicalExpr::bit_vec_literal(width.get(), 0),
        }
    }

    /// Returns the one literal for this type.
    ///
    /// Width-bearing literals currently support widths up to 128 bits.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError`](crate::circuit::CircuitError) when this is a
    /// `UInt` or `BitVec` wider than the 128-bit literal representation.
    pub fn one_literal(self) -> Result<ClassicalExpr, crate::circuit::CircuitError> {
        match self {
            Self::Bool => Ok(ClassicalExpr::bool_literal(true)),
            Self::Bit => Ok(ClassicalExpr::bit_literal(true)),
            Self::UInt(width) => ClassicalExpr::uint_literal(width.get(), 1),
            Self::BitVec(width) => ClassicalExpr::bit_vec_literal(width.get(), 1),
        }
    }

    /// Returns the number of measured bits accepted by this type.
    ///
    /// Only `Bit` and `BitVec` are valid direct measurement targets. `Bool`
    /// and `UInt` require explicit expression-level conversion.
    pub fn measurement_width(self) -> Option<u32> {
        match self {
            Self::Bit => Some(1),
            Self::BitVec(width) => Some(width.get()),
            Self::Bool | Self::UInt(_) => None,
        }
    }
}

/// Circuit-local handle to a mutable runtime classical storage location.
///
/// The handle carries its static type so expression and operation builders can
/// validate uses without maintaining parallel typed ID families. The `id` is
/// meaningful only inside the circuit that allocated it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClassicalVar {
    circuit_id: CircuitId,
    index: u32,
    ty: ClassicalType,
}

impl ClassicalVar {
    /// Creates a new classical variable handle.
    ///
    /// Prefer [`Circuit::var`](super::Circuit::var) for normal circuit
    /// construction so the handle is registered in the circuit's type table.
    pub fn new(circuit_id: CircuitId, index: u32, ty: ClassicalType) -> Self {
        Self {
            circuit_id,
            index,
            ty,
        }
    }

    /// Returns the circuit-local variable identifier.
    pub fn id(self) -> u32 {
        self.index
    }

    /// Returns the circuit-local variable table index.
    pub fn index(self) -> u32 {
        self.index
    }

    /// Returns the identity of the circuit that owns this variable.
    pub fn circuit_id(self) -> CircuitId {
        self.circuit_id
    }

    /// Returns the static type of this variable.
    pub fn ty(self) -> ClassicalType {
        self.ty
    }

    /// Creates an expression that reads this variable's current runtime value.
    pub fn expr(self) -> ClassicalExpr {
        ClassicalExpr::var(self)
    }
}

/// Circuit-local handle to an immutable runtime classical value.
///
/// Values are produced by operations such as measurement. Unlike
/// [`ClassicalVar`], a value is never overwritten. If a runtime result must be
/// preserved as loop-carried mutable state, store it into a [`ClassicalVar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClassicalValue {
    circuit_id: CircuitId,
    index: u32,
    ty: ClassicalType,
}

impl ClassicalValue {
    /// Creates a new classical value handle.
    ///
    /// Prefer circuit measurement builders for normal circuit construction so
    /// the value has a dominating producer in the circuit IR.
    pub fn new(circuit_id: CircuitId, index: u32, ty: ClassicalType) -> Self {
        Self {
            circuit_id,
            index,
            ty,
        }
    }

    /// Returns the circuit-local value table index.
    pub fn index(self) -> u32 {
        self.index
    }

    /// Returns the identity of the circuit that owns this value.
    pub fn circuit_id(self) -> CircuitId {
        self.circuit_id
    }

    /// Returns the static type of this value.
    pub fn ty(self) -> ClassicalType {
        self.ty
    }

    /// Creates an expression that reads this immutable runtime value.
    pub fn expr(self) -> ClassicalExpr {
        ClassicalExpr::value(self)
    }
}

/// Self-contained handle returned by circuit measurement builders.
///
/// [`ClassicalValue`] is the IR value used by expressions and control-flow.
/// `Measurement` adds the measured qubits and their bit order, so state-level
/// sampling APIs can consume a measurement without searching the circuit IR.
///
/// Bit order follows the `measure_bits` input order: `qubits()[0]` is bit index
/// `0`, the least-significant bit in packed results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measurement {
    value: ClassicalValue,
    qubits: SmallVec<[Qubit; 3]>,
}

impl Measurement {
    /// Creates a measurement handle from its IR value and measured qubits.
    pub fn new(value: ClassicalValue, qubits: SmallVec<[Qubit; 3]>) -> Self {
        Self { value, qubits }
    }

    /// Returns the immutable circuit value produced by this measurement.
    pub fn value(&self) -> ClassicalValue {
        self.value
    }

    /// Creates an expression that reads this measurement's immutable value.
    pub fn expr(&self) -> ClassicalExpr {
        self.value.expr()
    }

    /// Returns the measured qubits in result bit order.
    pub fn qubits(&self) -> &[Qubit] {
        &self.qubits
    }

    /// Returns the number of measured bits.
    pub fn width(&self) -> usize {
        self.qubits.len()
    }

    /// Returns the static type of the measurement result.
    pub fn ty(&self) -> ClassicalType {
        self.value.ty()
    }

    /// Checks that all measured qubits are valid for a state with `num_qubits`.
    ///
    /// Returns [`QisError::IndexOutOfBounds`] for the first qubit index outside
    /// `0..num_qubits`.
    pub fn check_qubits(&self, num_qubits: usize) -> Result<(), QisError> {
        for qubit in self.qubits() {
            let index = qubit.index();
            if index >= num_qubits {
                return Err(QisError::IndexOutOfBounds {
                    index,
                    max: num_qubits.saturating_sub(1),
                });
            }
        }
        Ok(())
    }

    /// Projects a full-register outcome onto this measurement's qubit order.
    ///
    /// If `qubits()[i]` is one in `full`, bit `i` is set in the returned
    /// [`Outcome`].
    pub fn project(&self, full: &Outcome) -> Outcome {
        Outcome::from_indices(
            self.width(),
            self.qubits()
                .iter()
                .enumerate()
                .filter_map(|(bit, qubit)| full.is_one(qubit.index()).then_some(bit)),
        )
    }

    /// Projects a computational-basis index onto this measurement's qubit order.
    ///
    /// If bit `qubits()[i]` is one in `basis`, bit `i` is set in the returned
    /// [`Outcome`].
    pub fn project_basis(&self, basis: usize) -> Outcome {
        Outcome::from_indices(
            self.width(),
            self.qubits()
                .iter()
                .enumerate()
                .filter_map(|(bit, qubit)| (((basis >> qubit.index()) & 1) == 1).then_some(bit)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{CircuitId, ClassicalType, ClassicalValue, ClassicalVar, Measurement};
    use crate::circuit::Qubit;
    use crate::device::Outcome;
    use crate::qis::QisError;
    use smallvec::smallvec;

    #[test]
    fn type_widths_are_reported() {
        assert_eq!(ClassicalType::Bit.width(), 1);
        assert_eq!(ClassicalType::Bool.width(), 1);
        assert_eq!(ClassicalType::uint(7).unwrap().width(), 7);
        assert_eq!(ClassicalType::bit_vec(3).unwrap().width(), 3);
    }

    #[test]
    fn zero_width_integer_and_bit_vector_are_rejected() {
        assert_eq!(ClassicalType::uint(0), None);
        assert_eq!(ClassicalType::bit_vec(0), None);
    }

    #[test]
    fn measurement_width_accepts_only_bits_and_bit_vectors() {
        assert_eq!(ClassicalType::Bit.measurement_width(), Some(1));
        assert_eq!(
            ClassicalType::bit_vec(5).unwrap().measurement_width(),
            Some(5)
        );
        assert_eq!(ClassicalType::Bool.measurement_width(), None);
        assert_eq!(ClassicalType::uint(5).unwrap().measurement_width(), None);
    }

    #[test]
    fn variables_expose_id_and_type() {
        let var = ClassicalVar::new(CircuitId::new(), 12, ClassicalType::bit_vec(4).unwrap());

        assert_eq!(var.id(), 12);
        assert_eq!(var.ty(), ClassicalType::bit_vec(4).unwrap());
    }

    #[test]
    fn variable_identity_includes_id_and_type() {
        let circuit_id = CircuitId::new();
        let bit = ClassicalVar::new(circuit_id, 1, ClassicalType::Bit);
        let same_bit = ClassicalVar::new(circuit_id, 1, ClassicalType::Bit);
        let bool_var = ClassicalVar::new(circuit_id, 1, ClassicalType::Bool);
        let other_bit = ClassicalVar::new(circuit_id, 2, ClassicalType::Bit);

        assert_eq!(bit, same_bit);
        assert_ne!(bit, bool_var);
        assert_ne!(bit, other_bit);
    }

    #[test]
    fn measurement_exposes_value_and_qubit_order() {
        let value = ClassicalValue::new(CircuitId::new(), 3, ClassicalType::bit_vec(2).unwrap());
        let measurement = Measurement::new(value, smallvec![Qubit::new(1), Qubit::new(0)]);

        assert_eq!(measurement.value(), value);
        assert_eq!(measurement.ty(), ClassicalType::bit_vec(2).unwrap());
        assert_eq!(measurement.width(), 2);
        assert_eq!(measurement.qubits(), &[Qubit::new(1), Qubit::new(0)]);
        assert_eq!(measurement.expr().ty(), ClassicalType::bit_vec(2).unwrap());
    }

    #[test]
    fn measurement_projects_outcomes_in_measurement_order() {
        let value = ClassicalValue::new(CircuitId::new(), 3, ClassicalType::bit_vec(3).unwrap());
        let measurement = Measurement::new(
            value,
            smallvec![Qubit::new(2), Qubit::new(0), Qubit::new(1)],
        );
        let full = Outcome::from_indices(3, [0, 2]);

        let projected = measurement.project(&full);
        assert_eq!(projected.to_bitstring(3), "011");
        assert_eq!(measurement.project_basis(0b101).to_bitstring(3), "011");
    }

    #[test]
    fn measurement_checks_qubit_bounds() {
        let value = ClassicalValue::new(CircuitId::new(), 3, ClassicalType::bit_vec(2).unwrap());
        let measurement = Measurement::new(value, smallvec![Qubit::new(0), Qubit::new(2)]);

        assert!(measurement.check_qubits(3).is_ok());
        assert!(matches!(
            measurement.check_qubits(2),
            Err(QisError::IndexOutOfBounds { index: 2, max: 1 })
        ));
    }
}
