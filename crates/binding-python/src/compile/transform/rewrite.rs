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

//! Python bindings for knowledge-based local circuit rewrite.

use crate::circuit::{PyCircuit, PyInstruction};
use crate::compile::error::compiler_error_to_py_err;
use crate::compile::knowledge::library::PyRuleKind;
use crate::utils::hash_value;
use cqlib_core::compile::knowledge::library::RuleKind;
use cqlib_core::compile::transform::{
    KnowledgeRewriteResult, KnowledgeRewriteStats, KnowledgeRewriter, RewriteConfig, RewriteMode,
    rewrite_circuit,
};
use pyo3::prelude::*;

/// High-level knowledge-rule application mode.
#[pyclass(
    name = "RewriteMode",
    module = "cqlib.compile.transform",
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyRewriteMode {
    inner: RewriteMode,
}

impl From<RewriteMode> for PyRewriteMode {
    fn from(inner: RewriteMode) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRewriteMode {
    /// Returns conservative optimization mode.
    #[staticmethod]
    fn optimize() -> Self {
        RewriteMode::Optimize.into()
    }

    /// Returns explicit lowering mode.
    #[staticmethod]
    fn lowering() -> Self {
        RewriteMode::Lowering.into()
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self.inner {
            RewriteMode::Optimize => "optimize",
            RewriteMode::Lowering => "lowering",
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }

    fn __repr__(&self) -> String {
        format!("RewriteMode.{}()", self.name())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }
}

/// Configuration for knowledge-based local circuit rewrite.
#[pyclass(
    name = "RewriteConfig",
    module = "cqlib.compile.transform",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRewriteConfig {
    pub(crate) inner: RewriteConfig,
}

impl From<RewriteConfig> for PyRewriteConfig {
    fn from(inner: RewriteConfig) -> Self {
        Self { inner }
    }
}

impl PyRewriteConfig {
    fn repr(&self) -> String {
        let enabled_kinds = self
            .inner
            .enabled_kinds()
            .iter()
            .map(|kind| format!("RuleKind.{}()", rule_kind_name(*kind)))
            .collect::<Vec<_>>()
            .join(", ");
        let target_instructions = self.inner.target_instruction_basis().map_or_else(
            || "None".to_string(),
            |instructions| {
                format!(
                    "[{}]",
                    instructions
                        .iter()
                        .map(|instruction| format!("Instruction({instruction})"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
        );
        format!(
            "RewriteConfig(max_rounds={}, max_window_ops={}, max_pattern_len={}, recurse_control_flow={}, skip_labeled_ops={}, preserve_two_qubit_connectivity={}, enabled_kinds=[{}], mode={}, target_instructions={})",
            self.inner.max_rounds(),
            self.inner.max_window_ops(),
            self.inner.max_pattern_len(),
            python_bool(self.inner.recurses_control_flow()),
            python_bool(self.inner.skips_labeled_ops()),
            python_bool(self.inner.preserve_two_qubit_connectivity()),
            enabled_kinds,
            PyRewriteMode::from(self.inner.mode()).__repr__(),
            target_instructions,
        )
    }
}

#[pymethods]
impl PyRewriteConfig {
    /// Creates a rewrite configuration from a production or lowering preset.
    #[new]
    #[pyo3(signature = (*, max_rounds=8, max_window_ops=16, max_pattern_len=8, recurse_control_flow=true, skip_labeled_ops=true, preserve_two_qubit_connectivity=false, enabled_kinds=None, mode=None, target_instructions=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        max_rounds: u8,
        max_window_ops: usize,
        max_pattern_len: usize,
        recurse_control_flow: bool,
        skip_labeled_ops: bool,
        preserve_two_qubit_connectivity: bool,
        enabled_kinds: Option<Vec<PyRuleKind>>,
        mode: Option<PyRewriteMode>,
        target_instructions: Option<Vec<PyInstruction>>,
    ) -> PyResult<Self> {
        let mode = mode.map_or(RewriteMode::Optimize, |mode| mode.inner);
        let mut config = match mode {
            RewriteMode::Optimize => RewriteConfig::production(),
            RewriteMode::Lowering => RewriteConfig::lowering(),
        }
        .with_max_rounds(max_rounds)
        .with_max_window_ops(max_window_ops)
        .with_max_pattern_len(max_pattern_len)
        .recurse_control_flow(recurse_control_flow)
        .skip_labeled_ops(skip_labeled_ops)
        .with_preserve_two_qubit_connectivity(preserve_two_qubit_connectivity)
        .with_mode(mode);

        if let Some(kinds) = enabled_kinds {
            config = config.with_enabled_kinds(kinds.into_iter().map(|kind| kind.inner).collect());
        }
        if let Some(instructions) = target_instructions {
            config = config
                .with_target_instructions(
                    instructions
                        .into_iter()
                        .map(|instruction| instruction.inner)
                        .collect(),
                )
                .map_err(compiler_error_to_py_err)?;
        }

        Ok(config.into())
    }

    /// Returns conservative production defaults.
    #[staticmethod]
    fn production() -> Self {
        RewriteConfig::production().into()
    }

    /// Returns explicit lowering defaults.
    #[staticmethod]
    fn lowering() -> Self {
        RewriteConfig::lowering().into()
    }

    #[getter]
    fn max_rounds(&self) -> u8 {
        self.inner.max_rounds()
    }

    #[getter]
    fn max_window_ops(&self) -> usize {
        self.inner.max_window_ops()
    }

    #[getter]
    fn max_pattern_len(&self) -> usize {
        self.inner.max_pattern_len()
    }

    #[getter]
    fn recurse_control_flow(&self) -> bool {
        self.inner.recurses_control_flow()
    }

    #[getter]
    fn skip_labeled_ops(&self) -> bool {
        self.inner.skips_labeled_ops()
    }

    #[getter]
    fn preserve_two_qubit_connectivity(&self) -> bool {
        self.inner.preserve_two_qubit_connectivity()
    }

    #[getter]
    fn enabled_kinds(&self) -> Vec<PyRuleKind> {
        self.inner
            .enabled_kinds()
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    #[getter]
    fn mode(&self) -> PyRewriteMode {
        self.inner.mode().into()
    }

    #[getter]
    fn target_instructions(&self) -> Option<Vec<PyInstruction>> {
        self.inner
            .target_instruction_basis()
            .map(|instructions| instructions.into_iter().map(Into::into).collect())
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

/// Aggregate statistics produced by one knowledge rewrite run.
#[pyclass(
    name = "KnowledgeRewriteStats",
    module = "cqlib.compile.transform",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyKnowledgeRewriteStats {
    inner: KnowledgeRewriteStats,
}

impl From<KnowledgeRewriteStats> for PyKnowledgeRewriteStats {
    fn from(inner: KnowledgeRewriteStats) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyKnowledgeRewriteStats {
    #[getter]
    fn rounds_executed(&self) -> u8 {
        self.inner.rounds_executed
    }

    #[getter]
    fn rules_applied(&self) -> usize {
        self.inner.rules_applied
    }

    #[getter]
    fn changed_sequences(&self) -> usize {
        self.inner.changed_sequences
    }

    #[getter]
    fn reached_fixpoint(&self) -> bool {
        self.inner.reached_fixpoint
    }

    fn __repr__(&self) -> String {
        format!(
            "KnowledgeRewriteStats(rounds_executed={}, rules_applied={}, changed_sequences={}, reached_fixpoint={})",
            self.inner.rounds_executed,
            self.inner.rules_applied,
            self.inner.changed_sequences,
            python_bool(self.inner.reached_fixpoint),
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

/// Rewritten circuit and fixed-point run metadata.
#[pyclass(
    name = "KnowledgeRewriteResult",
    module = "cqlib.compile.transform",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyKnowledgeRewriteResult {
    inner: KnowledgeRewriteResult,
}

impl From<KnowledgeRewriteResult> for PyKnowledgeRewriteResult {
    fn from(inner: KnowledgeRewriteResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyKnowledgeRewriteResult {
    #[getter]
    fn circuit(&self) -> PyCircuit {
        self.inner.circuit.clone().into()
    }

    #[getter]
    fn changed(&self) -> bool {
        self.inner.changed
    }

    #[getter]
    fn stats(&self) -> PyKnowledgeRewriteStats {
        self.inner.stats.clone().into()
    }

    fn __repr__(&self) -> String {
        format!(
            "KnowledgeRewriteResult(changed={}, stats={})",
            python_bool(self.inner.changed),
            PyKnowledgeRewriteStats::from(self.inner.stats.clone()).__repr__(),
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Configurable knowledge-based local circuit rewriter.
#[pyclass(
    name = "KnowledgeRewriter",
    module = "cqlib.compile.transform",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyKnowledgeRewriter {
    inner: KnowledgeRewriter,
}

#[pymethods]
impl PyKnowledgeRewriter {
    /// Creates a rewriter, using production defaults when omitted.
    #[new]
    #[pyo3(signature = (config=None))]
    fn new(config: Option<PyRewriteConfig>) -> Self {
        Self {
            inner: config.map_or_else(KnowledgeRewriter::production, |config| {
                KnowledgeRewriter::new(config.inner)
            }),
        }
    }

    /// Returns a rewriter using conservative production defaults.
    #[staticmethod]
    fn production() -> Self {
        Self {
            inner: KnowledgeRewriter::production(),
        }
    }

    /// Returns a rewriter using explicit lowering defaults.
    #[staticmethod]
    fn lowering() -> Self {
        Self {
            inner: KnowledgeRewriter::lowering(),
        }
    }

    #[getter]
    fn config(&self) -> PyRewriteConfig {
        self.inner.config().clone().into()
    }

    /// Rewrites a circuit without modifying the input.
    fn run(
        &self,
        py: Python<'_>,
        circuit: PyRef<'_, PyCircuit>,
    ) -> PyResult<PyKnowledgeRewriteResult> {
        let rewriter = self.inner.clone();
        let circuit = circuit.inner.clone();
        py.detach(move || rewriter.run(&circuit))
            .map(Into::into)
            .map_err(compiler_error_to_py_err)
    }

    fn __repr__(&self) -> String {
        format!("KnowledgeRewriter(config={})", self.config().repr())
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Rewrites a circuit with production defaults or an explicit configuration.
#[pyfunction(name = "rewrite_circuit")]
#[pyo3(signature = (circuit, config=None))]
pub fn py_rewrite_circuit(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    config: Option<PyRewriteConfig>,
) -> PyResult<PyKnowledgeRewriteResult> {
    let circuit = circuit.inner.clone();
    let config = config.map_or_else(RewriteConfig::production, |config| config.inner);
    py.detach(move || rewrite_circuit(&circuit, config))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

const fn python_bool(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

const fn rule_kind_name(kind: RuleKind) -> &'static str {
    match kind {
        RuleKind::Simplify => "simplify",
        RuleKind::Cancel => "cancel",
        RuleKind::Merge => "merge",
        RuleKind::Commute => "commute",
        RuleKind::Decompose => "decompose",
        RuleKind::Canonicalize => "canonicalize",
        RuleKind::HardwareNative => "hardware_native",
        RuleKind::Other => "other",
    }
}
