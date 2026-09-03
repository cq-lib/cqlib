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

//! Python-facing adapters for structural knowledge-rule matching.

use super::rule::{PyCondition, PyRule, PyRuleItem};
use crate::circuit::{PyParameter, PyQubit, PyValueOperation};
use cqlib_core::circuit::{Parameter, ValueInstruction, ValueOperation};
use cqlib_core::compile::knowledge::matcher::{
    ConcreteOperationView, MatchBindings, conditions_hold, instantiate_target, match_rule_item,
    rule_matches_operations,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;

fn operation_parts(
    operation: &ValueOperation,
) -> Option<(&cqlib_core::circuit::Instruction, Vec<Parameter>)> {
    let instruction = operation.instruction.as_instruction()?;
    let params = operation.params.iter().map(Parameter::from).collect();
    Some((instruction, params))
}

/// Symbolic parameter and qubit bindings produced by a structural rule match.
#[pyclass(
    name = "MatchBindings",
    module = "cqlib.compile.knowledge",
    skip_from_py_object
)]
#[derive(Clone, Debug, Default)]
pub struct PyMatchBindings {
    pub(crate) inner: MatchBindings,
}

#[pymethods]
impl PyMatchBindings {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    #[getter]
    fn qubits(&self) -> HashMap<u32, PyQubit> {
        self.inner
            .qubits()
            .iter()
            .map(|(&label, &qubit)| (label, qubit.into()))
            .collect()
    }

    fn qubit(&self, rule_qubit: u32) -> Option<PyQubit> {
        self.inner.qubit(rule_qubit).map(Into::into)
    }

    #[getter]
    fn params(&self) -> HashMap<String, PyParameter> {
        self.inner
            .params()
            .iter()
            .map(|(symbol, parameter)| (symbol.clone(), parameter.clone().into()))
            .collect()
    }

    fn param(&self, symbol: &str) -> Option<PyParameter> {
        self.inner.param(symbol).cloned().map(Into::into)
    }

    fn __repr__(&self) -> String {
        let mut qubits: Vec<_> = self
            .inner
            .qubits()
            .iter()
            .map(|(label, qubit)| (*label, qubit.index()))
            .collect();
        qubits.sort_unstable();
        let mut params: Vec<_> = self
            .inner
            .params()
            .iter()
            .map(|(symbol, value)| (symbol.clone(), value.to_string()))
            .collect();
        params.sort();
        format!("MatchBindings(qubits={qubits:?}, params={params:?})")
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Matches one rule item and transactionally updates `bindings` on success.
#[pyfunction(name = "match_rule_item")]
pub fn py_match_rule_item(
    item: PyRef<'_, PyRuleItem>,
    operation: PyRef<'_, PyValueOperation>,
    mut bindings: PyRefMut<'_, PyMatchBindings>,
) -> PyResult<bool> {
    let Some((instruction, params)) = operation_parts(&operation.inner) else {
        return Ok(false);
    };
    let view = ConcreteOperationView::new(instruction, &operation.inner.qubits, &params);
    match_rule_item(&item.inner, view, &mut bindings.inner)
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

/// Returns whether all supplied conditions hold under existing bindings.
#[pyfunction(name = "conditions_hold")]
pub fn py_conditions_hold(
    conditions: Vec<PyCondition>,
    bindings: PyRef<'_, PyMatchBindings>,
) -> bool {
    let conditions: Vec<_> = conditions
        .into_iter()
        .map(|condition| condition.inner)
        .collect();
    conditions_hold(Some(&conditions), &bindings.inner)
}

/// Instantiates rule target items and returns self-contained value operations.
#[pyfunction(name = "instantiate_target")]
pub fn py_instantiate_target(
    target: Vec<PyRuleItem>,
    bindings: PyRef<'_, PyMatchBindings>,
) -> PyResult<Vec<PyValueOperation>> {
    let target: Vec<_> = target.into_iter().map(|item| item.inner).collect();
    instantiate_target(&target, &bindings.inner)
        .map(|replacements| {
            replacements
                .into_iter()
                .map(|replacement| {
                    ValueOperation {
                        instruction: ValueInstruction::from_instruction(replacement.instruction),
                        qubits: replacement.qubits,
                        params: replacement.params.into_iter().collect(),
                        label: None,
                    }
                    .into()
                })
                .collect()
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

/// Matches a rule against an adjacent sequence of self-contained operations.
#[pyfunction(name = "rule_matches_operations")]
pub fn py_rule_matches_operations(
    py: Python<'_>,
    rule: PyRef<'_, PyRule>,
    operations: Vec<Py<PyValueOperation>>,
) -> PyResult<Option<PyMatchBindings>> {
    let operations: Vec<_> = operations
        .into_iter()
        .map(|operation| operation.borrow(py).inner.clone())
        .collect();
    let params: Vec<Vec<Parameter>> = operations
        .iter()
        .map(|operation| operation.params.iter().map(Parameter::from).collect())
        .collect();

    if operations
        .iter()
        .any(|operation| operation.instruction.as_instruction().is_none())
    {
        return Ok(None);
    }

    let views: Vec<_> = operations
        .iter()
        .zip(&params)
        .filter_map(|(operation, params)| {
            operation.instruction.as_instruction().map(|instruction| {
                ConcreteOperationView::new(instruction, &operation.qubits, params)
            })
        })
        .collect();

    rule_matches_operations(&rule.inner, &views)
        .map(|bindings| bindings.map(|inner| PyMatchBindings { inner }))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}
