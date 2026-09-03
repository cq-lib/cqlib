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

//! Python wrappers for compiler knowledge rules and equivalence results.

use crate::circuit::{PyInstruction, PyMcGate, PyParameter, PyStandardGate};
use cqlib_core::circuit::{Parameter, ParameterValue};
use cqlib_core::compile::knowledge::rule::{Condition, Rule, RuleItem};
use cqlib_core::compile::knowledge::rule_equivalence::VerifyResult;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn parameter_values(values: &[ParameterValue]) -> Vec<PyParameter> {
    values.iter().map(Parameter::from).map(Into::into).collect()
}

/// One gate-like operation in a knowledge rule's match or rewrite block.
#[pyclass(name = "RuleItem", module = "cqlib.compile.knowledge", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyRuleItem {
    pub(crate) inner: RuleItem,
}

impl From<RuleItem> for PyRuleItem {
    fn from(inner: RuleItem) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRuleItem {
    /// Creates a standard-gate rule item, preserving parameters bound to the gate.
    #[staticmethod]
    fn standard(gate: PyStandardGate, qubits: Vec<u32>) -> Self {
        Self {
            inner: RuleItem::standard(
                gate.inner,
                &qubits,
                gate.params.into_iter().map(ParameterValue::from).collect(),
            ),
        }
    }

    /// Creates a multi-controlled-gate rule item, preserving bound parameters.
    #[staticmethod]
    fn mc_gate(gate: PyMcGate, qubits: Vec<u32>) -> Self {
        Self {
            inner: RuleItem::mc_gate(
                gate.inner,
                &qubits,
                gate.params.into_iter().map(ParameterValue::from).collect(),
            ),
        }
    }

    #[getter]
    fn instruction(&self) -> PyInstruction {
        self.inner.instruction.clone().into()
    }

    #[getter]
    fn qubits(&self) -> Vec<u32> {
        self.inner.qubits.to_vec()
    }

    #[getter]
    fn params(&self) -> Vec<PyParameter> {
        parameter_values(self.inner.params.as_deref().unwrap_or(&[]))
    }

    fn symbols(&self) -> Vec<String> {
        let mut symbols: Vec<_> = self.inner.symbols().into_iter().collect();
        symbols.sort();
        symbols
    }

    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }

    fn equivalent_to(&self, other: &Self) -> bool {
        self.inner.equivalent_to(&other.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "RuleItem(instruction={}, qubits={:?}, params={:?})",
            self.inner.instruction,
            self.inner.qubits,
            self.params()
                .iter()
                .map(|param| param.inner.to_string())
                .collect::<Vec<_>>()
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// A symbolic condition required for a knowledge rule to match.
#[pyclass(name = "Condition", module = "cqlib.compile.knowledge", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyCondition {
    pub(crate) inner: Condition,
}

impl From<Condition> for PyCondition {
    fn from(inner: Condition) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCondition {
    #[staticmethod]
    fn equal(lhs: PyParameter, rhs: PyParameter) -> Self {
        Self {
            inner: Condition::Eq(lhs.inner, rhs.inner),
        }
    }

    #[staticmethod]
    fn equal_mod(lhs: PyParameter, rhs: PyParameter, modulus: PyParameter) -> Self {
        Self {
            inner: Condition::EqMod(lhs.inner, rhs.inner, modulus.inner),
        }
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner {
            Condition::Eq(_, _) => "equal",
            Condition::EqMod(_, _, _) => "equal_mod",
        }
    }

    #[getter]
    fn lhs(&self) -> PyParameter {
        match &self.inner {
            Condition::Eq(lhs, _) | Condition::EqMod(lhs, _, _) => lhs.clone().into(),
        }
    }

    #[getter]
    fn rhs(&self) -> PyParameter {
        match &self.inner {
            Condition::Eq(_, rhs) | Condition::EqMod(_, rhs, _) => rhs.clone().into(),
        }
    }

    #[getter]
    fn modulus(&self) -> Option<PyParameter> {
        match &self.inner {
            Condition::Eq(_, _) => None,
            Condition::EqMod(_, _, modulus) => Some(modulus.clone().into()),
        }
    }

    fn symbols(&self) -> Vec<String> {
        let mut symbols: Vec<_> = self.inner.symbols().into_iter().collect();
        symbols.sort();
        symbols
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            Condition::Eq(lhs, rhs) => format!("Condition.equal({lhs}, {rhs})"),
            Condition::EqMod(lhs, rhs, modulus) => {
                format!("Condition.equal_mod({lhs}, {rhs}, {modulus})")
            }
        }
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Diagnostic value returned by knowledge-rule equivalence verification.
#[pyclass(
    name = "VerifyResult",
    module = "cqlib.compile.knowledge",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVerifyResult {
    status: &'static str,
    num_bindings: Option<usize>,
    reason: Option<String>,
}

impl From<VerifyResult> for PyVerifyResult {
    fn from(result: VerifyResult) -> Self {
        match result {
            VerifyResult::Equivalent => Self {
                status: "equivalent",
                num_bindings: None,
                reason: None,
            },
            VerifyResult::SampledEqual { num_bindings } => Self {
                status: "sampled_equal",
                num_bindings: Some(num_bindings),
                reason: None,
            },
            VerifyResult::NotEquivalent => Self {
                status: "not_equivalent",
                num_bindings: None,
                reason: None,
            },
            VerifyResult::Inconclusive { reason } => Self {
                status: "inconclusive",
                num_bindings: None,
                reason: Some(reason),
            },
        }
    }
}

#[pymethods]
impl PyVerifyResult {
    #[getter]
    fn status(&self) -> &'static str {
        self.status
    }

    #[getter]
    fn passed(&self) -> bool {
        matches!(self.status, "equivalent" | "sampled_equal")
    }

    /// Returns whether equivalence verification succeeded.
    fn is_verified(&self) -> bool {
        self.passed()
    }

    #[getter]
    fn num_bindings(&self) -> Option<usize> {
        self.num_bindings
    }

    #[getter]
    fn reason(&self) -> Option<String> {
        self.reason.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "VerifyResult(status={:?}, num_bindings={:?}, reason={:?})",
            self.status, self.num_bindings, self.reason
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// A validated compiler knowledge rewrite rule.
#[pyclass(name = "Rule", module = "cqlib.compile.knowledge", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyRule {
    pub(crate) inner: Rule,
}

impl From<Rule> for PyRule {
    fn from(inner: Rule) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRule {
    #[new]
    #[pyo3(signature = (name, operations, target, conditions=None))]
    fn new(
        name: &str,
        operations: Vec<PyRuleItem>,
        target: Vec<PyRuleItem>,
        conditions: Option<Vec<PyCondition>>,
    ) -> Self {
        let mut rule = Rule::new(
            name,
            operations.into_iter().map(|item| item.inner).collect(),
            target.into_iter().map(|item| item.inner).collect(),
        );
        rule.conditions = conditions.map(|conditions| {
            conditions
                .into_iter()
                .map(|condition| condition.inner)
                .collect()
        });
        Self { inner: rule }
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn operations(&self) -> Vec<PyRuleItem> {
        self.inner
            .operations
            .iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    #[getter]
    fn conditions(&self) -> Vec<PyCondition> {
        self.inner
            .conditions
            .iter()
            .flatten()
            .cloned()
            .map(Into::into)
            .collect()
    }

    #[getter]
    fn target(&self) -> Vec<PyRuleItem> {
        self.inner.target.iter().cloned().map(Into::into).collect()
    }

    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }

    fn verify(&self) -> PyResult<PyVerifyResult> {
        self.inner
            .verify()
            .map(Into::into)
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }

    fn verify_by_sampling(&self, num_bindings: usize, tolerance: f64) -> PyResult<PyVerifyResult> {
        if num_bindings == 0 {
            return Err(PyValueError::new_err(
                "num_bindings must be greater than zero",
            ));
        }
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(PyValueError::new_err(
                "tolerance must be finite and greater than zero",
            ));
        }
        self.inner
            .verify_by_sampling(num_bindings, tolerance)
            .map(Into::into)
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }

    fn needs_sampling_fallback(&self) -> bool {
        self.inner.needs_sampling_fallback()
    }

    fn free_symbols(&self) -> Vec<String> {
        let mut symbols: Vec<_> = self.inner.collect_free_symbols().into_iter().collect();
        symbols.sort();
        symbols
    }

    fn operation_qubits(&self) -> Vec<u32> {
        self.inner.operation_qubits().into_iter().collect()
    }

    fn target_qubits(&self) -> Vec<u32> {
        self.inner.target_qubits().into_iter().collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Rule(name={:?}, operations={}, conditions={}, target={})",
            self.inner.name,
            self.inner.operations.len(),
            self.inner
                .conditions
                .as_ref()
                .map_or(0, |items| items.len()),
            self.inner.target.len()
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}
