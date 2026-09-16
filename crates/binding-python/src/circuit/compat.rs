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

//! Binding-only metadata and validation for legacy construction adapters.

use super::bit::PyIntOrQubit;
use super::error::{CircuitError, ParameterError};
use super::{PyCircuit, PyParameter};
use cqlib_core::circuit::{ClassicalDataOp, Directive, Instruction, Qubit};
use pyo3::prelude::*;
use std::collections::HashSet;

#[derive(FromPyObject)]
pub(super) enum MeasureInput {
    Single(PyIntOrQubit),
    Many(Vec<PyIntOrQubit>),
}

pub(super) fn warn(py: Python<'_>, old: &str, replacement: &str) -> PyResult<()> {
    py.import("cqlib._compat.deprecation")?
        .getattr("warn")?
        .call1((old, replacement))?;
    Ok(())
}

impl PyCircuit {
    pub(super) fn declare_legacy_parameters(
        &mut self,
        parameters: Vec<PyParameter>,
    ) -> PyResult<()> {
        let mut order = self.legacy_parameter_order.clone().unwrap_or_default();
        // Validate the complete declaration before interning any parameter.
        for parameter in &parameters {
            let name = parameter.inner.as_symbol().ok_or_else(|| {
                ParameterError::new_err("declare only single-symbol Parameters, not expressions")
            })?;
            if order.contains(&name) {
                return Err(ParameterError::new_err(format!(
                    "duplicate parameter declaration: {name}"
                )));
            }
            order.push(name);
        }
        for parameter in parameters {
            self.inner.add_parameter(parameter.inner);
        }
        self.legacy_parameter_order = Some(order);
        Ok(())
    }

    pub(super) fn merged_legacy_order(&self, other: &Self) -> Option<Vec<String>> {
        if self.legacy_parameter_order.is_none() && other.legacy_parameter_order.is_none() {
            return None;
        }
        let mut merged = Vec::new();
        for circuit in [self, other] {
            let order = circuit
                .legacy_parameter_order
                .as_deref()
                .unwrap_or_default();
            // Unregistered live symbols make positional binding ambiguous.
            if circuit
                .inner
                .used_symbols()
                .iter()
                .any(|name| !order.contains(name))
            {
                return None;
            }
            for name in order {
                if !merged.contains(name) {
                    merged.push(name.clone());
                }
            }
        }
        Some(merged)
    }

    pub(super) fn legacy_terminal_measurements(&self) -> PyResult<HashSet<Qubit>> {
        let mut measured = HashSet::new();
        for operation in self.inner.operations() {
            match &operation.instruction {
                Instruction::ClassicalData(
                    ClassicalDataOp::MeasureBit { .. } | ClassicalDataOp::MeasureBits { .. },
                )
                | Instruction::Directive(Directive::Measure) => {
                    measured.extend(operation.qubits.iter().copied());
                }
                Instruction::ClassicalControl(_) | Instruction::ClassicalData(_) => {
                    return Err(CircuitError::new_err(
                        "legacy bulk measurement supports static circuits only; use measure or measure_bits",
                    ));
                }
                Instruction::Directive(Directive::Barrier) => {}
                _ if operation
                    .qubits
                    .iter()
                    .any(|qubit| measured.contains(qubit)) =>
                {
                    return Err(CircuitError::new_err(
                        "legacy bulk measurement requires terminal measurements; use measure or measure_bits",
                    ));
                }
                _ => {}
            }
        }
        Ok(measured)
    }

    pub(super) fn validate_legacy_measurements(&self, qubits: &[Qubit]) -> PyResult<()> {
        self.legacy_terminal_measurements()?;
        let available = self.inner.qubits();
        for qubit in qubits {
            if !available.contains(qubit) {
                return Err(CircuitError::new_err(format!(
                    "qubit {} is not in this circuit",
                    qubit.id()
                )));
            }
        }
        Ok(())
    }
}
