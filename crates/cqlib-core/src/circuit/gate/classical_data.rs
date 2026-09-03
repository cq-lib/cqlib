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

//! Side-effecting runtime classical data instructions.
//!
//! These instructions connect quantum measurements and typed classical
//! storage. Pure computations remain represented by
//! [`ClassicalExpr`].

use crate::circuit::{ClassicalExpr, ClassicalValue, ClassicalVar};
use std::fmt;

/// Runtime classical data operation.
///
/// These operations are side-effecting classical updates in the circuit
/// schedule. They produce or overwrite runtime classical storage, while
/// [`ClassicalExpr`] remains side-effect-free and only reads runtime classical
/// variables or immutable values.
#[derive(Debug, Clone, PartialEq)]
pub enum ClassicalDataOp {
    /// Stores an expression value into a mutable classical variable.
    Store {
        /// Mutable storage location being updated.
        target: ClassicalVar,
        /// Type-compatible value expression to store.
        value: ClassicalExpr,
    },
    /// Measures one qubit and produces an immutable `Bit` value.
    MeasureBit {
        /// Immutable value receiving the measurement result.
        result: ClassicalValue,
    },
    /// Measures several qubits and produces an immutable `BitVec(width)` value.
    ///
    /// The operation qubit order is the bit-vector order, with qubit index 0
    /// mapped to the least-significant bit.
    MeasureBits {
        /// Immutable bit-vector value receiving the packed result.
        result: ClassicalValue,
    },
}

impl ClassicalDataOp {
    /// Returns the stable snake-case operation name.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Store { .. } => "store",
            Self::MeasureBit { .. } => "measure_bit",
            Self::MeasureBits { .. } => "measure_bits",
        }
    }

    /// Returns the mutable store target, if this is a store operation.
    pub fn target(&self) -> Option<ClassicalVar> {
        match self {
            Self::Store { target, .. } => Some(*target),
            Self::MeasureBit { .. } | Self::MeasureBits { .. } => None,
        }
    }

    /// Returns the immutable measurement result, if this is a measurement.
    pub fn result(&self) -> Option<ClassicalValue> {
        match self {
            Self::Store { .. } => None,
            Self::MeasureBit { result } | Self::MeasureBits { result } => Some(*result),
        }
    }

    /// Returns the stored expression, if this is a store operation.
    pub fn value(&self) -> Option<&ClassicalExpr> {
        match self {
            Self::Store { value, .. } => Some(value),
            Self::MeasureBit { .. } | Self::MeasureBits { .. } => None,
        }
    }
}

impl fmt::Display for ClassicalDataOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
