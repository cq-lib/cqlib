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

//! Shared validation for classical control-flow operations.
//!
//! Both the storage IR ([`ForOp`](super::ForOp), [`SwitchOp`](super::SwitchOp))
//! and the construction IR ([`ValueClassicalControlOp`](crate::circuit::ValueClassicalControlOp))
//! enforce the same invariants. These helpers are the single source of truth for
//! those rules so the two IRs cannot drift apart.

use crate::circuit::classical_expr::ClassicalExpr;
use crate::circuit::{CircuitError, ClassicalType, ClassicalVar};
use std::collections::BTreeSet;

/// Validates that `var` is a `UInt` and that `start`, `stop`, and `step` all
/// match the loop variable's width.
pub(crate) fn validate_for_types(
    var: ClassicalVar,
    start: &ClassicalExpr,
    stop: &ClassicalExpr,
    step: &ClassicalExpr,
) -> Result<(), CircuitError> {
    if !matches!(var.ty(), ClassicalType::UInt(_)) {
        return Err(CircuitError::InvalidOperation(format!(
            "for loop variable must be UInt, got {:?}",
            var.ty()
        )));
    }
    if start.ty() != var.ty() {
        return Err(CircuitError::InvalidOperation(format!(
            "for start type must match loop variable {:?}, got {:?}",
            var.ty(),
            start.ty()
        )));
    }
    if stop.ty() != var.ty() {
        return Err(CircuitError::InvalidOperation(format!(
            "for stop type must match loop variable {:?}, got {:?}",
            var.ty(),
            stop.ty()
        )));
    }
    if step.ty() != var.ty() {
        return Err(CircuitError::InvalidOperation(format!(
            "for step type must match loop variable {:?}, got {:?}",
            var.ty(),
            step.ty()
        )));
    }

    Ok(())
}

/// Validates that `target` is a `UInt` and that every case value fits the
/// target width and appears exactly once.
pub(crate) fn validate_switch(
    target: &ClassicalExpr,
    case_values: impl IntoIterator<Item = u128>,
) -> Result<(), CircuitError> {
    let width = match target.ty() {
        ClassicalType::UInt(width) => width.get(),
        ty => {
            return Err(CircuitError::InvalidOperation(format!(
                "switch target must be UInt, got {ty:?}"
            )));
        }
    };

    let mut values = BTreeSet::new();
    for value in case_values {
        if width < 128 && value >= (1u128 << width) {
            return Err(CircuitError::InvalidOperation(format!(
                "switch case value {} does not fit in target width {width}",
                value
            )));
        }
        if !values.insert(value) {
            return Err(CircuitError::InvalidOperation(format!(
                "duplicate switch case value {}",
                value
            )));
        }
    }

    Ok(())
}
