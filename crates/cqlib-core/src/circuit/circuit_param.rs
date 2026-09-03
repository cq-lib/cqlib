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

//! # Circuit Parameters Module
//!
//! This module defines the types used to represent parameters within a circuit and its operations.
//! It bridges the gap between high-level symbolic expressions ([`Parameter`]) and low-level
//! storage ([`CircuitParam`]).
//!
//! ## Two Representations
//!
//! | Type | IR layer | Parameter form | Where used |
//! |------|----------|---------------|------------|
//! | [`ParameterValue`] | Construction (pre-insertion) | Symbolic or fixed | [`ValueOperation`](super::ValueOperation), gate builders |
//! | [`CircuitParam`] | Storage (post-insertion) | Index into parameter table or fixed | [`Operation`](super::Operation), circuit internals |
//!
//! [`ParameterValue`] is the user-facing type accepted by gate convenience methods
//! (`Circuit::rx`, `Circuit::u`, etc.) and value-level IR builders. Fixed `f64`
//! values and symbolic [`Parameter`] objects are both accepted interchangeably.
//!
//! On insertion via [`Circuit::append`](super::circuit_impl::Circuit::append),
//! symbolic parameters are interned into the circuit's parameter table and
//! replaced with [`CircuitParam::Index`]. Fixed parameters are validated to be
//! finite and stored directly as [`CircuitParam::Fixed`].

use super::parameter::Parameter;

/// Represents a resolved parameter stored efficiently within a [`Circuit`](crate::circuit::Circuit).
///
/// This enum is designed for memory efficiency in the circuit's operation list.
/// Instead of storing full symbolic expressions in every operation, we either store a raw `f64`
/// (for fixed values) or an index into the circuit's centralized parameter table.
#[derive(Debug, Clone)]
pub enum CircuitParam {
    /// An index pointing to the `parameters` list in the parent [`Circuit`](crate::circuit::Circuit).
    /// This represents a symbolic or shared parameter.
    Index(u32),
    /// A concrete, fixed floating-point value.
    Fixed(f64),
}

impl PartialEq for CircuitParam {
    /// Compares the circuit-local storage form: two parameters are equal only
    /// when both are fixed with the same value, or both are indices into the
    /// same parameter-table position. Indices from different circuits compare
    /// by table position, not by the parameter they resolve to.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Index(lhs), Self::Index(rhs)) => lhs == rhs,
            (Self::Fixed(lhs), Self::Fixed(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

impl From<f64> for CircuitParam {
    fn from(v: f64) -> Self {
        Self::Fixed(v)
    }
}

/// A flexible input type for specifying parameters when constructing circuit operations.
///
/// This enum allows users to pass either raw numbers or symbolic `Parameter` objects
/// to gate builders (like `rx`, `ry`, `u`).
///
/// # Examples
///
/// ```rust
/// use cqlib_core::circuit::circuit_param::ParameterValue;
/// use cqlib_core::circuit::Parameter;
///
/// // Create from float
/// let val1: ParameterValue = 1.5.into();
///
/// // Create from integer
/// let val2: ParameterValue = 42.into();
///
/// // Create from symbolic parameter
/// let param = Parameter::symbol("theta");
/// let val3: ParameterValue = param.into();
/// ```
#[derive(Debug, Clone)]
pub enum ParameterValue {
    /// A symbolic parameter (which may contain variables like "theta").
    Param(Parameter),
    /// A fixed floating-point value.
    Fixed(f64),
}

impl PartialEq for ParameterValue {
    /// Compares the construction form: two values are equal only when both
    /// are symbolic with structurally equal expressions, or both are fixed
    /// with the same value. A symbolic parameter and its evaluated fixed
    /// value compare unequal even when numerically identical.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Param(lhs), Self::Param(rhs)) => lhs == rhs,
            (Self::Fixed(lhs), Self::Fixed(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

impl From<f64> for ParameterValue {
    fn from(v: f64) -> Self {
        Self::Fixed(v)
    }
}

impl From<i64> for ParameterValue {
    fn from(v: i64) -> Self {
        Self::Fixed(v as f64)
    }
}

impl From<Parameter> for ParameterValue {
    /// Converts a `Parameter` into a `ParameterValue`.
    ///
    /// It attempts to eagerly evaluate the parameter if it represents a known constant
    /// (like `Pi`, `E`, or a simple number) to optimize storage.
    fn from(para: Parameter) -> Self {
        if let Ok(p) = para.evaluate(&None) {
            Self::Fixed(p)
        } else {
            Self::Param(para)
        }
    }
}

impl From<&Parameter> for ParameterValue {
    fn from(para: &Parameter) -> Self {
        if let Ok(p) = para.evaluate(&None) {
            Self::Fixed(p)
        } else {
            Self::Param(para.clone())
        }
    }
}

impl From<ParameterValue> for Parameter {
    fn from(para: ParameterValue) -> Parameter {
        match para {
            ParameterValue::Param(p) => p,
            ParameterValue::Fixed(f) => Parameter::from(f),
        }
    }
}

impl From<&ParameterValue> for Parameter {
    fn from(para: &ParameterValue) -> Parameter {
        match para {
            ParameterValue::Param(p) => p.clone(),
            ParameterValue::Fixed(f) => Parameter::from(*f),
        }
    }
}

impl From<&str> for ParameterValue {
    /// Converts a string literal into a symbolic parameter.
    ///
    /// ```rust
    /// use cqlib_core::circuit::circuit_param::ParameterValue;
    ///
    /// let theta: ParameterValue = "theta".into();
    /// ```
    fn from(para: &str) -> Self {
        Parameter::symbol(para).into()
    }
}
