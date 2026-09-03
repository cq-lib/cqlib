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

use crate::circuit::PyCircuit;
use crate::compile::error::compiler_error_to_py_err;
use cqlib_core::compile::transform::{
    CanonicalizeConfig, CanonicalizeResult, Canonicalizer, canonicalize_circuit,
};
use pyo3::prelude::*;

/// Configuration for circuit canonicalization.
#[pyclass(
    name = "CanonicalizeConfig",
    module = "cqlib.compile.transform",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCanonicalizeConfig {
    pub(crate) inner: CanonicalizeConfig,
}

impl From<CanonicalizeConfig> for PyCanonicalizeConfig {
    fn from(inner: CanonicalizeConfig) -> Self {
        Self { inner }
    }
}

impl From<PyCanonicalizeConfig> for CanonicalizeConfig {
    fn from(value: PyCanonicalizeConfig) -> Self {
        value.inner
    }
}

impl PyCanonicalizeConfig {
    fn repr(&self) -> String {
        format!(
            "CanonicalizeConfig(round_limit={}, recurse_control_flow={}, fold_gphase={}, canonicalize_instruction_form={}, drop_noops={}, canonicalize_barriers={})",
            self.inner.round_limit(),
            python_bool(self.inner.recurses_control_flow()),
            python_bool(self.inner.folds_gphase()),
            python_bool(self.inner.canonicalizes_instruction_form()),
            python_bool(self.inner.drops_noops()),
            python_bool(self.inner.canonicalizes_barriers()),
        )
    }
}

#[pymethods]
impl PyCanonicalizeConfig {
    /// Creates a canonicalization configuration.
    #[new]
    #[pyo3(signature = (*, round_limit=8, recurse_control_flow=true, fold_gphase=true, canonicalize_instruction_form=true, drop_noops=true, canonicalize_barriers=true))]
    fn new(
        round_limit: u8,
        recurse_control_flow: bool,
        fold_gphase: bool,
        canonicalize_instruction_form: bool,
        drop_noops: bool,
        canonicalize_barriers: bool,
    ) -> Self {
        CanonicalizeConfig::new()
            .with_round_limit(round_limit)
            .recurse_control_flow(recurse_control_flow)
            .fold_gphase(fold_gphase)
            .canonicalize_instruction_form(canonicalize_instruction_form)
            .drop_noops(drop_noops)
            .canonicalize_barriers(canonicalize_barriers)
            .into()
    }

    /// Returns the production canonicalization configuration.
    #[staticmethod]
    fn production() -> Self {
        CanonicalizeConfig::production().into()
    }

    #[getter]
    fn round_limit(&self) -> u8 {
        self.inner.round_limit()
    }

    #[getter]
    fn recurse_control_flow(&self) -> bool {
        self.inner.recurses_control_flow()
    }

    #[getter]
    fn fold_gphase(&self) -> bool {
        self.inner.folds_gphase()
    }

    #[getter]
    fn canonicalize_instruction_form(&self) -> bool {
        self.inner.canonicalizes_instruction_form()
    }

    #[getter]
    fn drop_noops(&self) -> bool {
        self.inner.drops_noops()
    }

    #[getter]
    fn canonicalize_barriers(&self) -> bool {
        self.inner.canonicalizes_barriers()
    }

    fn __repr__(&self) -> String {
        self.repr()
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

/// Result of a canonicalization run.
#[pyclass(
    name = "CanonicalizeResult",
    module = "cqlib.compile.transform",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCanonicalizeResult {
    inner: CanonicalizeResult,
}

impl From<CanonicalizeResult> for PyCanonicalizeResult {
    fn from(inner: CanonicalizeResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCanonicalizeResult {
    #[getter]
    fn circuit(&self) -> PyCircuit {
        self.inner.circuit.clone().into()
    }

    #[getter]
    fn changed(&self) -> bool {
        self.inner.changed
    }

    #[getter]
    fn rounds(&self) -> u8 {
        self.inner.rounds
    }

    fn __repr__(&self) -> String {
        format!(
            "CanonicalizeResult(changed={}, rounds={})",
            python_bool(self.inner.changed),
            self.inner.rounds,
        )
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

/// Configurable circuit canonicalizer.
#[pyclass(
    name = "Canonicalizer",
    module = "cqlib.compile.transform",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCanonicalizer {
    inner: Canonicalizer,
}

#[pymethods]
impl PyCanonicalizer {
    /// Creates a canonicalizer, using production defaults when omitted.
    #[new]
    #[pyo3(signature = (config=None))]
    fn new(config: Option<PyCanonicalizeConfig>) -> Self {
        Self {
            inner: config.map_or_else(Canonicalizer::production, |config| {
                Canonicalizer::new(config.inner)
            }),
        }
    }

    /// Returns a canonicalizer using production defaults.
    #[staticmethod]
    fn production() -> Self {
        Self {
            inner: Canonicalizer::production(),
        }
    }

    #[getter]
    fn config(&self) -> PyCanonicalizeConfig {
        self.inner.config().clone().into()
    }

    /// Canonicalizes a circuit without modifying the input.
    fn run(&self, py: Python<'_>, circuit: PyRef<'_, PyCircuit>) -> PyResult<PyCanonicalizeResult> {
        let canonicalizer = self.inner.clone();
        let circuit = circuit.inner.clone();
        py.detach(move || canonicalizer.run(&circuit))
            .map(PyCanonicalizeResult::from)
            .map_err(compiler_error_to_py_err)
    }

    fn __repr__(&self) -> String {
        format!("Canonicalizer(config={})", self.config().repr())
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Canonicalizes a circuit using production defaults.
#[pyfunction(name = "canonicalize_circuit")]
pub fn py_canonicalize_circuit(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
) -> PyResult<PyCanonicalizeResult> {
    let circuit = circuit.inner.clone();
    py.detach(move || canonicalize_circuit(&circuit))
        .map(PyCanonicalizeResult::from)
        .map_err(compiler_error_to_py_err)
}

const fn python_bool(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}
