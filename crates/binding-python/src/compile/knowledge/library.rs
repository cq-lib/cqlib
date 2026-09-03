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

//! Python wrappers for knowledge-rule libraries and DSL I/O.

use super::rule::PyRule;
use crate::circuit::PyInstruction;
use crate::utils::hash_value;
use cqlib_core::compile::knowledge::library::{
    RuleId, RuleKind, RuleLibrary, RuleLibraryError, RuleMetadata,
};
use cqlib_core::compile::knowledge::rule_dsl::dump::{
    dump_rule_to_file, dump_rule_to_string, dump_rules_to_file,
};
use cqlib_core::compile::knowledge::rule_dsl::load::{
    LoadError, load_rules_from_file, load_rules_from_str,
};
use pyo3::exceptions::{PyIOError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use std::path::PathBuf;

fn library_error(error: RuleLibraryError) -> PyErr {
    match error {
        RuleLibraryError::Load(error) => load_error(error),
        other => PyValueError::new_err(other.to_string()),
    }
}

fn load_error(error: LoadError) -> PyErr {
    match error {
        LoadError::Io(message) => PyIOError::new_err(message),
        other => PyValueError::new_err(other.to_string()),
    }
}

/// Stable library-local identifier for a knowledge rule.
#[pyclass(name = "RuleId", module = "cqlib.compile.knowledge", from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyRuleId {
    pub(crate) inner: RuleId,
}

impl From<RuleId> for PyRuleId {
    fn from(inner: RuleId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRuleId {
    #[getter]
    fn index(&self) -> usize {
        self.inner.as_usize()
    }

    fn __index__(&self) -> usize {
        self.inner.as_usize()
    }

    fn __repr__(&self) -> String {
        format!("RuleId({})", self.inner.as_usize())
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

/// Coarse compiler use-case assigned to a knowledge rule.
#[pyclass(name = "RuleKind", module = "cqlib.compile.knowledge", from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyRuleKind {
    pub(crate) inner: RuleKind,
}

impl PyRuleKind {
    fn label(self) -> &'static str {
        match self.inner {
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
}

impl From<RuleKind> for PyRuleKind {
    fn from(inner: RuleKind) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRuleKind {
    #[staticmethod]
    fn simplify() -> Self {
        Self {
            inner: RuleKind::Simplify,
        }
    }

    #[staticmethod]
    fn cancel() -> Self {
        Self {
            inner: RuleKind::Cancel,
        }
    }

    #[staticmethod]
    fn merge() -> Self {
        Self {
            inner: RuleKind::Merge,
        }
    }

    #[staticmethod]
    fn commute() -> Self {
        Self {
            inner: RuleKind::Commute,
        }
    }

    #[staticmethod]
    fn decompose() -> Self {
        Self {
            inner: RuleKind::Decompose,
        }
    }

    #[staticmethod]
    fn canonicalize() -> Self {
        Self {
            inner: RuleKind::Canonicalize,
        }
    }

    #[staticmethod]
    fn hardware_native() -> Self {
        Self {
            inner: RuleKind::HardwareNative,
        }
    }

    #[staticmethod]
    fn other() -> Self {
        Self {
            inner: RuleKind::Other,
        }
    }

    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("RuleKind.{}()", self.label())
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

/// Precomputed selection metadata for a rule in a library.
#[pyclass(
    name = "RuleMetadata",
    module = "cqlib.compile.knowledge",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRuleMetadata {
    inner: RuleMetadata,
}

impl From<RuleMetadata> for PyRuleMetadata {
    fn from(inner: RuleMetadata) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRuleMetadata {
    #[getter]
    fn id(&self) -> PyRuleId {
        self.inner.id.into()
    }

    #[getter]
    fn kind(&self) -> PyRuleKind {
        self.inner.kind.into()
    }

    #[getter]
    fn pattern_len(&self) -> usize {
        self.inner.pattern_len
    }

    #[getter]
    fn rewrite_len(&self) -> usize {
        self.inner.rewrite_len
    }

    #[getter]
    fn qubit_count(&self) -> usize {
        self.inner.qubit_count
    }

    #[getter]
    fn first_instruction(&self) -> PyInstruction {
        self.inner.first_instruction.clone().into()
    }

    #[getter]
    fn cost_delta(&self) -> isize {
        self.inner.cost_delta
    }

    #[getter]
    fn has_conditions(&self) -> bool {
        self.inner.has_conditions
    }

    fn __repr__(&self) -> String {
        format!(
            "RuleMetadata(id={}, kind={}, pattern_len={}, rewrite_len={}, qubit_count={}, cost_delta={}, has_conditions={})",
            self.inner.id.as_usize(),
            PyRuleKind::from(self.inner.kind).label(),
            self.inner.pattern_len,
            self.inner.rewrite_len,
            self.inner.qubit_count,
            self.inner.cost_delta,
            self.inner.has_conditions
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Validated knowledge-rule collection with compiler selection indexes.
#[pyclass(
    name = "RuleLibrary",
    module = "cqlib.compile.knowledge",
    skip_from_py_object
)]
#[derive(Clone, Debug, Default)]
pub struct PyRuleLibrary {
    pub(crate) inner: RuleLibrary,
}

#[pymethods]
impl PyRuleLibrary {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    #[staticmethod]
    fn builtin() -> PyResult<Self> {
        RuleLibrary::builtin_rules()
            .cloned()
            .map(|inner| Self { inner })
            .map_err(library_error)
    }

    #[staticmethod]
    fn from_rules(rules: Vec<PyRule>, kind: PyRuleKind) -> PyResult<Self> {
        RuleLibrary::from_rules(
            rules.into_iter().map(|rule| rule.inner).collect(),
            kind.inner,
        )
        .map(|inner| Self { inner })
        .map_err(library_error)
    }

    #[staticmethod]
    fn from_dsl(source: &str, kind: PyRuleKind) -> PyResult<Self> {
        RuleLibrary::from_dsl_str(source, kind.inner)
            .map(|inner| Self { inner })
            .map_err(library_error)
    }

    #[staticmethod]
    fn from_dsl_file(path: PathBuf, kind: PyRuleKind) -> PyResult<Self> {
        RuleLibrary::from_dsl_file(path, kind.inner)
            .map(|inner| Self { inner })
            .map_err(library_error)
    }

    fn add_rule(&mut self, rule: PyRule, kind: PyRuleKind) -> PyResult<PyRuleId> {
        self.inner
            .add_rule(rule.inner, kind.inner, true)
            .map(Into::into)
            .map_err(library_error)
    }

    fn extend_rules(&mut self, rules: Vec<PyRule>, kind: PyRuleKind) -> PyResult<Vec<PyRuleId>> {
        self.inner
            .extend_rules(
                rules.into_iter().map(|rule| rule.inner).collect(),
                kind.inner,
            )
            .map(|ids| ids.into_iter().map(Into::into).collect())
            .map_err(library_error)
    }

    fn rules(&self) -> Vec<PyRule> {
        self.inner.rules().iter().cloned().map(Into::into).collect()
    }

    fn get(&self, id: PyRuleId) -> Option<PyRule> {
        self.inner.get(id.inner).cloned().map(Into::into)
    }

    fn metadata(&self, id: PyRuleId) -> Option<PyRuleMetadata> {
        self.inner.metadata(id.inner).cloned().map(Into::into)
    }

    fn id_by_name(&self, name: &str) -> Option<PyRuleId> {
        self.inner.id_by_name(name).map(Into::into)
    }

    fn get_by_name(&self, name: &str) -> Option<PyRule> {
        self.inner.get_by_name(name).cloned().map(Into::into)
    }

    fn __contains__(&self, name: &str) -> bool {
        self.inner.contains(name)
    }

    fn candidates_for_first_instruction(
        &self,
        instruction: PyInstruction,
    ) -> PyResult<Vec<PyRuleId>> {
        self.inner
            .candidates_for_first_instruction(&instruction.inner)
            .map(|ids| ids.iter().copied().map(Into::into).collect())
            .map_err(library_error)
    }

    fn rules_by_kind(&self, kind: PyRuleKind) -> Vec<PyRuleId> {
        self.inner
            .rules_by_kind(kind.inner)
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    fn filter_rule_ids_by_instruction_keys(
        &self,
        op_instructions: Vec<PyInstruction>,
        target_instructions: Vec<PyInstruction>,
    ) -> PyResult<Vec<PyRuleId>> {
        let op_instructions: Vec<_> = op_instructions
            .into_iter()
            .map(|instruction| instruction.inner)
            .collect();
        let target_instructions: Vec<_> = target_instructions
            .into_iter()
            .map(|instruction| instruction.inner)
            .collect();
        self.inner
            .filter_rule_ids_by_instruction_keys(&op_instructions, &target_instructions)
            .map(|ids| ids.into_iter().map(Into::into).collect())
            .map_err(library_error)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __bool__(&self) -> bool {
        !self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("RuleLibrary(len={})", self.inner.len())
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Parses runtime knowledge rules from a DSL source string.
#[pyfunction(name = "loads")]
pub fn py_loads(source: &str) -> PyResult<Vec<PyRule>> {
    load_rules_from_str(source)
        .map(|rules| rules.into_iter().map(Into::into).collect())
        .map_err(load_error)
}

/// Loads runtime knowledge rules from a DSL file.
#[pyfunction(name = "load")]
pub fn py_load(path: PathBuf) -> PyResult<Vec<PyRule>> {
    load_rules_from_file(path)
        .map(|rules| rules.into_iter().map(Into::into).collect())
        .map_err(load_error)
}

/// Serializes one runtime knowledge rule to DSL text.
#[pyfunction(name = "dumps")]
pub fn py_dumps(rule: PyRef<'_, PyRule>) -> String {
    dump_rule_to_string(&rule.inner)
}

/// Writes one rule or a sequence of rules to a DSL file.
#[pyfunction(name = "dump")]
pub fn py_dump(rule_or_rules: &Bound<'_, PyAny>, path: PathBuf) -> PyResult<()> {
    if let Ok(rule) = rule_or_rules.extract::<PyRef<'_, PyRule>>() {
        return dump_rule_to_file(&rule.inner, path)
            .map_err(|error| PyIOError::new_err(error.to_string()));
    }
    if let Ok(rules) = rule_or_rules.extract::<Vec<PyRule>>() {
        let rules: Vec<_> = rules.into_iter().map(|rule| rule.inner).collect();
        return dump_rules_to_file(&rules, path)
            .map_err(|error| PyIOError::new_err(error.to_string()));
    }
    Err(PyTypeError::new_err(
        "dump expects a Rule or a sequence of Rule objects",
    ))
}
