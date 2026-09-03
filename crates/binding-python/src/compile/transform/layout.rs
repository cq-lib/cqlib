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

use crate::circuit::{PyCircuit, PyStandardGate};
use crate::compile::error::compiler_error_to_py_err;
use crate::compile::sabre::PySabreConfig;
use crate::device::device_impl::PyDevice;
use crate::device::layout::PyLayout;
use crate::device::qubit::{PyLogicalQubit, PyPhysicalQubit, PyPhysicalQubitLike};
use crate::utils::hash_value;
use cqlib_core::compile::sabre::SabreConfig;
use cqlib_core::compile::transform::layout::{DistanceTable, PhysicalLayoutGraph};
use cqlib_core::compile::transform::{
    CircuitLayoutAnalysis, Interaction, InteractionGraph, LayoutDiagnostics, LayoutObjective,
    LayoutResult, LayoutScore, PreparedSabreCircuit, PreparedSabreTarget, Vf2EdgeRequirement,
    Vf2LayoutConfig, analyze_circuit_for_layout, greedy_layout, greedy_layout_prepared,
    prepare_sabre_circuit, prepare_sabre_device_target, prepare_sabre_topology_target,
    sabre_layout, sabre_layout_prepared, trivial_layout, trivial_layout_prepared,
    vf2_perfect_layout, vf2_perfect_layout_prepared,
};
use pyo3::prelude::*;

/// Registers layout bindings as `_native.compile.transform.layout`.
pub(crate) fn register_layout_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "layout")?;

    m.add_class::<PyLayoutObjective>()?;
    m.add_class::<PyLayoutScore>()?;
    m.add_class::<PyLayoutDiagnostics>()?;
    m.add_class::<PyLayoutResult>()?;
    m.add_class::<PyVf2EdgeRequirement>()?;
    m.add_class::<PyVf2LayoutConfig>()?;
    m.add_class::<PyInteraction>()?;
    m.add_class::<PyInteractionGraph>()?;
    m.add_class::<PyCircuitLayoutAnalysis>()?;
    m.add_class::<PyDistanceTable>()?;
    m.add_class::<PyPhysicalLayoutGraph>()?;
    m.add_class::<PyPreparedSabreCircuit>()?;
    m.add_class::<PyPreparedSabreTarget>()?;
    m.add_function(pyo3::wrap_pyfunction!(py_trivial_layout, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_greedy_layout, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_vf2_perfect_layout, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_sabre_layout, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_analyze_circuit_for_layout, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_prepare_sabre_circuit, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        py_prepare_sabre_topology_target,
        &m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_prepare_sabre_device_target, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_sabre_layout_prepared, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_trivial_layout_prepared, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_greedy_layout_prepared, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_vf2_perfect_layout_prepared, &m)?)?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile.transform.layout", &m)?;

    Ok(())
}

#[pyfunction(name = "trivial_layout")]
#[pyo3(signature = (circuit, device, objective=None))]
fn py_trivial_layout(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    device: PyRef<'_, PyDevice>,
    objective: Option<PyLayoutObjective>,
) -> PyResult<PyLayoutResult> {
    let circuit = circuit.inner.clone();
    let device = device.inner.clone();
    let objective = objective.map_or_else(LayoutObjective::topology_only, |value| value.inner);
    py.detach(move || trivial_layout(&circuit, &device, &objective))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

#[pyfunction(name = "greedy_layout")]
#[pyo3(signature = (circuit, device, objective=None))]
fn py_greedy_layout(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    device: PyRef<'_, PyDevice>,
    objective: Option<PyLayoutObjective>,
) -> PyResult<PyLayoutResult> {
    let circuit = circuit.inner.clone();
    let device = device.inner.clone();
    let objective = objective.map_or_else(LayoutObjective::topology_only, |value| value.inner);
    py.detach(move || greedy_layout(&circuit, &device, &objective))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

#[pyfunction(name = "vf2_perfect_layout")]
#[pyo3(signature = (circuit, device, objective=None, config=None))]
fn py_vf2_perfect_layout(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    device: PyRef<'_, PyDevice>,
    objective: Option<PyLayoutObjective>,
    config: Option<PyVf2LayoutConfig>,
) -> PyResult<PyLayoutResult> {
    let circuit = circuit.inner.clone();
    let device = device.inner.clone();
    let objective = objective.map_or_else(LayoutObjective::topology_only, |value| value.inner);
    let config = config.map_or_else(Vf2LayoutConfig::default, |value| value.inner);
    py.detach(move || vf2_perfect_layout(&circuit, &device, &objective, &config))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

#[pyfunction(name = "sabre_layout")]
#[pyo3(signature = (circuit, device, objective=None, config=None))]
fn py_sabre_layout(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    device: PyRef<'_, PyDevice>,
    objective: Option<PyLayoutObjective>,
    config: Option<PySabreConfig>,
) -> PyResult<PyLayoutResult> {
    let circuit = circuit.inner.clone();
    let device = device.inner.clone();
    let objective = objective.map_or_else(LayoutObjective::topology_only, |value| value.inner);
    let config = config.map_or_else(SabreConfig::default, |value| value.inner);
    py.detach(move || sabre_layout(&circuit, &device, &objective, &config))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

/// Weighted objective used to rank candidate initial layouts.
#[pyclass(
    name = "LayoutObjective",
    module = "cqlib.compile.transform.layout",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyLayoutObjective {
    pub(crate) inner: LayoutObjective,
}

impl From<LayoutObjective> for PyLayoutObjective {
    fn from(inner: LayoutObjective) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyLayoutObjective {
    /// Creates a layout objective from explicit non-negative weights.
    #[new]
    #[pyo3(signature = (*, distance_weight=1.0, direction_weight=1.0, two_qubit_error_weight=0.0, readout_error_weight=0.0))]
    fn new(
        distance_weight: f64,
        direction_weight: f64,
        two_qubit_error_weight: f64,
        readout_error_weight: f64,
    ) -> Self {
        LayoutObjective {
            distance_weight,
            direction_weight,
            two_qubit_error_weight,
            readout_error_weight,
        }
        .into()
    }

    /// Returns the topology-only objective.
    #[staticmethod]
    fn topology_only() -> Self {
        LayoutObjective::topology_only().into()
    }

    /// Returns the default fidelity-aware objective.
    #[staticmethod]
    fn fidelity_aware() -> Self {
        LayoutObjective::fidelity_aware().into()
    }

    /// Selects a fidelity-aware objective when the device has usable calibration data.
    #[staticmethod]
    fn auto_from_device(py: Python<'_>, device: PyRef<'_, PyDevice>) -> PyResult<Self> {
        let device = device.inner.clone();
        py.detach(move || {
            let physical = PhysicalLayoutGraph::from_device(&device)?;
            Ok(LayoutObjective::auto_from_physical(&physical))
        })
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
    }

    /// Selects a fidelity-aware objective when the prepared graph has calibration data.
    #[staticmethod]
    fn auto_from_physical(physical: PyRef<'_, PyPhysicalLayoutGraph>) -> Self {
        LayoutObjective::auto_from_physical(&physical.inner).into()
    }

    /// Returns a fidelity-aware objective, rejecting devices without usable calibration data.
    #[staticmethod]
    fn fidelity_required(py: Python<'_>, device: PyRef<'_, PyDevice>) -> PyResult<Self> {
        let device = device.inner.clone();
        py.detach(move || {
            let physical = PhysicalLayoutGraph::from_device(&device)?;
            LayoutObjective::fidelity_required(&physical)
        })
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
    }

    /// Returns a fidelity-aware objective, requiring calibration in a prepared graph.
    #[staticmethod]
    fn fidelity_required_from_physical(
        physical: PyRef<'_, PyPhysicalLayoutGraph>,
    ) -> PyResult<Self> {
        LayoutObjective::fidelity_required(&physical.inner)
            .map(Into::into)
            .map_err(compiler_error_to_py_err)
    }

    /// Scores a complete layout against prepared circuit and physical data.
    fn score_layout(
        &self,
        py: Python<'_>,
        analysis: PyRef<'_, PyCircuitLayoutAnalysis>,
        physical: PyRef<'_, PyPhysicalLayoutGraph>,
        layout: PyRef<'_, PyLayout>,
    ) -> PyResult<PyLayoutScore> {
        let objective = self.inner.clone();
        let analysis = analysis.inner.clone();
        let physical = physical.inner.clone();
        let layout = layout.inner.clone();
        py.detach(move || objective.score_layout(&analysis, &physical, &layout))
            .map(Into::into)
            .map_err(compiler_error_to_py_err)
    }

    #[getter]
    fn distance_weight(&self) -> f64 {
        self.inner.distance_weight
    }

    #[getter]
    fn direction_weight(&self) -> f64 {
        self.inner.direction_weight
    }

    #[getter]
    fn two_qubit_error_weight(&self) -> f64 {
        self.inner.two_qubit_error_weight
    }

    #[getter]
    fn readout_error_weight(&self) -> f64 {
        self.inner.readout_error_weight
    }

    #[getter]
    fn uses_fidelity(&self) -> bool {
        self.inner.uses_fidelity()
    }

    fn __repr__(&self) -> String {
        format!(
            "LayoutObjective(distance_weight={}, direction_weight={}, two_qubit_error_weight={}, readout_error_weight={})",
            self.inner.distance_weight,
            self.inner.direction_weight,
            self.inner.two_qubit_error_weight,
            self.inner.readout_error_weight,
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

/// Breakdown of a layout objective score.
#[pyclass(
    name = "LayoutScore",
    module = "cqlib.compile.transform.layout",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyLayoutScore {
    inner: LayoutScore,
}

impl From<LayoutScore> for PyLayoutScore {
    fn from(inner: LayoutScore) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyLayoutScore {
    #[getter]
    fn total(&self) -> f64 {
        self.inner.total
    }

    #[getter]
    fn distance(&self) -> f64 {
        self.inner.distance
    }

    #[getter]
    fn direction(&self) -> f64 {
        self.inner.direction
    }

    #[getter]
    fn two_qubit_error(&self) -> f64 {
        self.inner.two_qubit_error
    }

    #[getter]
    fn readout_error(&self) -> f64 {
        self.inner.readout_error
    }

    #[getter]
    fn used_fidelity(&self) -> bool {
        self.inner.used_fidelity
    }

    fn __repr__(&self) -> String {
        format!(
            "LayoutScore(total={}, distance={}, direction={}, two_qubit_error={}, readout_error={}, used_fidelity={})",
            self.inner.total,
            self.inner.distance,
            self.inner.direction,
            self.inner.two_qubit_error,
            self.inner.readout_error,
            python_bool(self.inner.used_fidelity),
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

/// Diagnostics emitted by an initial-layout algorithm.
#[pyclass(
    name = "LayoutDiagnostics",
    module = "cqlib.compile.transform.layout",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyLayoutDiagnostics {
    inner: LayoutDiagnostics,
}

impl From<LayoutDiagnostics> for PyLayoutDiagnostics {
    fn from(inner: LayoutDiagnostics) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyLayoutDiagnostics {
    #[getter]
    fn is_perfect(&self) -> bool {
        self.inner.is_perfect
    }

    #[getter]
    fn candidates_evaluated(&self) -> usize {
        self.inner.candidates_evaluated
    }

    #[getter]
    fn used_fidelity(&self) -> bool {
        self.inner.used_fidelity
    }

    #[getter]
    fn notes(&self) -> Vec<String> {
        self.inner.notes.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "LayoutDiagnostics(is_perfect={}, candidates_evaluated={}, used_fidelity={}, notes={:?})",
            python_bool(self.inner.is_perfect),
            self.inner.candidates_evaluated,
            python_bool(self.inner.used_fidelity),
            self.inner.notes,
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

/// Selected initial layout, observed score, and diagnostics.
///
/// The score is the observed objective value of the selected layout.
/// Individual algorithms may use a different selection key; in particular,
/// SABRE selects its winner by predicted route quality under the prepared
/// routing cost model and reports this score for diagnostics.
#[pyclass(
    name = "LayoutResult",
    module = "cqlib.compile.transform.layout",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyLayoutResult {
    inner: LayoutResult,
}

impl From<LayoutResult> for PyLayoutResult {
    fn from(inner: LayoutResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyLayoutResult {
    #[getter]
    fn layout(&self) -> PyLayout {
        self.inner.layout.clone().into()
    }

    /// Observed score of this layout under the requested objective, when
    /// available. This score is diagnostic and is not necessarily the
    /// algorithm's selection key.
    #[getter]
    fn score(&self) -> Option<PyLayoutScore> {
        self.inner.score.clone().map(Into::into)
    }

    #[getter]
    fn diagnostics(&self) -> PyLayoutDiagnostics {
        self.inner.diagnostics.clone().into()
    }

    fn __repr__(&self) -> String {
        format!(
            "LayoutResult(layout={:?}, score={:?}, diagnostics={:?})",
            self.inner.layout, self.inner.score, self.inner.diagnostics,
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

/// Selects which logical interactions are hard topology constraints for VF2.
#[pyclass(
    name = "Vf2EdgeRequirement",
    module = "cqlib.compile.transform.layout",
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyVf2EdgeRequirement {
    pub(crate) inner: Vf2EdgeRequirement,
}

impl From<Vf2EdgeRequirement> for PyVf2EdgeRequirement {
    fn from(inner: Vf2EdgeRequirement) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVf2EdgeRequirement {
    #[staticmethod]
    fn positive_interactions() -> Self {
        Vf2EdgeRequirement::PositiveInteractions.into()
    }

    #[staticmethod]
    fn all_interactions() -> Self {
        Vf2EdgeRequirement::AllInteractions.into()
    }

    fn __repr__(&self) -> &'static str {
        match self.inner {
            Vf2EdgeRequirement::PositiveInteractions => {
                "Vf2EdgeRequirement.positive_interactions()"
            }
            Vf2EdgeRequirement::AllInteractions => "Vf2EdgeRequirement.all_interactions()",
        }
    }

    fn __str__(&self) -> &'static str {
        match self.inner {
            Vf2EdgeRequirement::PositiveInteractions => "positive_interactions",
            Vf2EdgeRequirement::AllInteractions => "all_interactions",
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        let discriminant = match self.inner {
            Vf2EdgeRequirement::PositiveInteractions => 0_u8,
            Vf2EdgeRequirement::AllInteractions => 1,
        };
        hash_value(&discriminant)
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }
}

/// Configuration for VF2 perfect-layout search.
#[pyclass(
    name = "Vf2LayoutConfig",
    module = "cqlib.compile.transform.layout",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVf2LayoutConfig {
    pub(crate) inner: Vf2LayoutConfig,
}

impl From<Vf2LayoutConfig> for PyVf2LayoutConfig {
    fn from(inner: Vf2LayoutConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVf2LayoutConfig {
    #[new]
    #[pyo3(signature = (*, candidate_limit=10, call_limit=None, edge_requirement=None))]
    fn new(
        candidate_limit: usize,
        call_limit: Option<usize>,
        edge_requirement: Option<PyVf2EdgeRequirement>,
    ) -> Self {
        Self {
            inner: Vf2LayoutConfig {
                candidate_limit,
                call_limit,
                edge_requirement: edge_requirement
                    .map_or(Vf2EdgeRequirement::PositiveInteractions, |requirement| {
                        requirement.inner
                    }),
            },
        }
    }

    #[getter]
    fn candidate_limit(&self) -> usize {
        self.inner.candidate_limit
    }

    #[getter]
    fn call_limit(&self) -> Option<usize> {
        self.inner.call_limit
    }

    #[getter]
    fn edge_requirement(&self) -> PyVf2EdgeRequirement {
        self.inner.edge_requirement.into()
    }

    fn __repr__(&self) -> String {
        format!(
            "Vf2LayoutConfig(candidate_limit={}, call_limit={:?}, edge_requirement={})",
            self.inner.candidate_limit,
            self.inner.call_limit,
            PyVf2EdgeRequirement::from(self.inner.edge_requirement).__repr__(),
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

/// One weighted logical interaction used by layout algorithms.
#[pyclass(
    name = "Interaction",
    module = "cqlib.compile.transform.layout",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyInteraction {
    inner: Interaction,
}

impl From<Interaction> for PyInteraction {
    fn from(inner: Interaction) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInteraction {
    #[getter]
    fn left(&self) -> PyLogicalQubit {
        self.inner.left.into()
    }

    #[getter]
    fn right(&self) -> PyLogicalQubit {
        self.inner.right.into()
    }

    #[getter]
    fn weight(&self) -> f64 {
        self.inner.weight
    }

    #[getter]
    fn directed_weight_left_to_right(&self) -> f64 {
        self.inner.directed_weight_left_to_right
    }

    #[getter]
    fn directed_weight_right_to_left(&self) -> f64 {
        self.inner.directed_weight_right_to_left
    }

    #[getter]
    fn first_seen_order(&self) -> usize {
        self.inner.first_seen_order
    }

    fn __repr__(&self) -> String {
        format!(
            "Interaction(left={:?}, right={:?}, weight={}, directed_weight_left_to_right={}, directed_weight_right_to_left={}, first_seen_order={})",
            self.inner.left,
            self.inner.right,
            self.inner.weight,
            self.inner.directed_weight_left_to_right,
            self.inner.directed_weight_right_to_left,
            self.inner.first_seen_order,
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

/// Deterministically ordered weighted logical interaction graph.
#[pyclass(
    name = "InteractionGraph",
    module = "cqlib.compile.transform.layout",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyInteractionGraph {
    inner: InteractionGraph,
}

impl From<InteractionGraph> for PyInteractionGraph {
    fn from(inner: InteractionGraph) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInteractionGraph {
    #[new]
    fn new() -> Self {
        InteractionGraph::new().into()
    }

    #[getter]
    fn interactions(&self) -> Vec<PyInteraction> {
        self.inner
            .interactions()
            .iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn logical_activity(&self) -> Vec<(PyLogicalQubit, f64)> {
        self.inner
            .logical_activity()
            .into_iter()
            .map(|(qubit, weight)| (qubit.into(), weight))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("InteractionGraph(interactions={})", self.inner.len())
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

/// Reusable circuit-side summary for layout selection.
#[pyclass(
    name = "CircuitLayoutAnalysis",
    module = "cqlib.compile.transform.layout",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCircuitLayoutAnalysis {
    inner: CircuitLayoutAnalysis,
}

impl From<CircuitLayoutAnalysis> for PyCircuitLayoutAnalysis {
    fn from(inner: CircuitLayoutAnalysis) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCircuitLayoutAnalysis {
    #[getter]
    fn logical_qubits(&self) -> Vec<PyLogicalQubit> {
        self.inner
            .logical_qubits
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    #[getter]
    fn interactions(&self) -> PyInteractionGraph {
        self.inner.interactions.clone().into()
    }

    fn __repr__(&self) -> String {
        format!(
            "CircuitLayoutAnalysis(logical_qubits={}, interactions={})",
            self.inner.logical_qubits.len(),
            self.inner.interactions.len()
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

/// All-pairs undirected distances over usable physical qubits.
#[pyclass(
    name = "DistanceTable",
    module = "cqlib.compile.transform.layout",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDistanceTable {
    inner: DistanceTable,
}

impl From<DistanceTable> for PyDistanceTable {
    fn from(inner: DistanceTable) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDistanceTable {
    #[getter]
    fn qubits(&self) -> Vec<PyPhysicalQubit> {
        self.inner
            .qubits()
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    fn distance(&self, a: PyPhysicalQubitLike, b: PyPhysicalQubitLike) -> Option<u32> {
        self.inner.distance(a.into(), b.into())
    }

    fn __repr__(&self) -> String {
        format!("DistanceTable(qubits={})", self.inner.qubits().len())
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

/// Compiler-local physical topology and calibration view.
#[pyclass(
    name = "PhysicalLayoutGraph",
    module = "cqlib.compile.transform.layout",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPhysicalLayoutGraph {
    inner: PhysicalLayoutGraph,
}

impl From<PhysicalLayoutGraph> for PyPhysicalLayoutGraph {
    fn from(inner: PhysicalLayoutGraph) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPhysicalLayoutGraph {
    #[staticmethod]
    fn from_device(py: Python<'_>, device: PyRef<'_, PyDevice>) -> PyResult<Self> {
        let device = device.inner.clone();
        py.detach(move || PhysicalLayoutGraph::from_device(&device))
            .map(Into::into)
            .map_err(compiler_error_to_py_err)
    }

    #[getter]
    fn physical_qubits(&self) -> Vec<PyPhysicalQubit> {
        self.inner
            .physical_qubits()
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    #[getter]
    fn distances(&self) -> PyDistanceTable {
        self.inner.distances().clone().into()
    }

    fn distance(&self, a: PyPhysicalQubitLike, b: PyPhysicalQubitLike) -> Option<u32> {
        self.inner.distance(a.into(), b.into())
    }

    fn is_adjacent_undirected(&self, a: PyPhysicalQubitLike, b: PyPhysicalQubitLike) -> bool {
        self.inner.is_adjacent_undirected(a.into(), b.into())
    }

    fn readout_error(&self, qubit: PyPhysicalQubitLike) -> Option<f64> {
        self.inner.readout_error(qubit.into())
    }

    fn supports_two_qubit_gate_directed(
        &self,
        control: PyPhysicalQubitLike,
        target: PyPhysicalQubitLike,
        gate: PyRef<'_, PyStandardGate>,
    ) -> bool {
        self.inner
            .supports_two_qubit_gate_directed(control.into(), target.into(), gate.inner)
    }

    fn two_qubit_gate_error_directed(
        &self,
        control: PyPhysicalQubitLike,
        target: PyPhysicalQubitLike,
        gate: PyRef<'_, PyStandardGate>,
    ) -> Option<f64> {
        self.inner
            .two_qubit_gate_error_directed(control.into(), target.into(), gate.inner)
    }

    fn supports_directed_coupling(
        &self,
        control: PyPhysicalQubitLike,
        target: PyPhysicalQubitLike,
    ) -> bool {
        self.inner
            .supports_directed_coupling(control.into(), target.into())
    }

    #[getter]
    fn has_fidelity_data(&self) -> bool {
        self.inner.has_fidelity_data()
    }

    #[getter]
    fn has_readout_error_data(&self) -> bool {
        self.inner.has_readout_error_data()
    }

    #[getter]
    fn has_two_qubit_error_data(&self) -> bool {
        self.inner.has_two_qubit_error_data()
    }

    fn __repr__(&self) -> String {
        format!(
            "PhysicalLayoutGraph(physical_qubits={}, has_fidelity_data={})",
            self.inner.physical_qubits().len(),
            python_bool(self.inner.has_fidelity_data())
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

/// Circuit-side data prepared once for repeated SABRE layout selection.
#[pyclass(
    name = "PreparedSabreCircuit",
    module = "cqlib.compile.transform.layout",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPreparedSabreCircuit {
    inner: PreparedSabreCircuit,
}

impl From<PreparedSabreCircuit> for PyPreparedSabreCircuit {
    fn from(inner: PreparedSabreCircuit) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPreparedSabreCircuit {
    #[getter]
    fn analysis(&self) -> PyCircuitLayoutAnalysis {
        self.inner.analysis().clone().into()
    }

    #[getter]
    fn logical_qubits(&self) -> Vec<PyLogicalQubit> {
        self.inner
            .logical_qubits()
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "PreparedSabreCircuit(logical_qubits={})",
            self.inner.logical_qubits().len()
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Device-side SABRE data prepared for one circuit's requirements.
#[pyclass(
    name = "PreparedSabreTarget",
    module = "cqlib.compile.transform.layout",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPreparedSabreTarget {
    inner: PreparedSabreTarget,
}

impl From<PreparedSabreTarget> for PyPreparedSabreTarget {
    fn from(inner: PreparedSabreTarget) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPreparedSabreTarget {
    #[getter]
    fn physical(&self) -> PyPhysicalLayoutGraph {
        self.inner.physical().clone().into()
    }

    fn __repr__(&self) -> String {
        format!(
            "PreparedSabreTarget(physical_qubits={})",
            self.inner.physical().physical_qubits().len()
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

#[pyfunction(name = "analyze_circuit_for_layout")]
fn py_analyze_circuit_for_layout(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
) -> PyResult<PyCircuitLayoutAnalysis> {
    let circuit = circuit.inner.clone();
    py.detach(move || analyze_circuit_for_layout(&circuit))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

#[pyfunction(name = "prepare_sabre_circuit")]
fn py_prepare_sabre_circuit(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
) -> PyResult<PyPreparedSabreCircuit> {
    let circuit = circuit.inner.clone();
    py.detach(move || prepare_sabre_circuit(&circuit))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

#[pyfunction(name = "prepare_sabre_topology_target")]
fn py_prepare_sabre_topology_target(
    py: Python<'_>,
    prepared: PyRef<'_, PyPreparedSabreCircuit>,
    device: PyRef<'_, PyDevice>,
) -> PyResult<PyPreparedSabreTarget> {
    let prepared = prepared.inner.clone();
    let device = device.inner.clone();
    py.detach(move || prepare_sabre_topology_target(&prepared, &device))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

#[pyfunction(name = "prepare_sabre_device_target")]
fn py_prepare_sabre_device_target(
    py: Python<'_>,
    prepared: PyRef<'_, PyPreparedSabreCircuit>,
    device: PyRef<'_, PyDevice>,
) -> PyResult<PyPreparedSabreTarget> {
    let prepared = prepared.inner.clone();
    let device = device.inner.clone();
    py.detach(move || prepare_sabre_device_target(&prepared, &device))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

#[pyfunction(name = "sabre_layout_prepared")]
#[pyo3(signature = (prepared, prepared_target, objective=None, config=None))]
fn py_sabre_layout_prepared(
    py: Python<'_>,
    prepared: PyRef<'_, PyPreparedSabreCircuit>,
    prepared_target: PyRef<'_, PyPreparedSabreTarget>,
    objective: Option<PyLayoutObjective>,
    config: Option<PySabreConfig>,
) -> PyResult<PyLayoutResult> {
    let prepared = prepared.inner.clone();
    let prepared_target = prepared_target.inner.clone();
    let objective = objective.map_or_else(LayoutObjective::topology_only, |value| value.inner);
    let config = config.map_or_else(SabreConfig::default, |value| value.inner);
    py.detach(move || sabre_layout_prepared(&prepared, &prepared_target, &objective, &config))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

#[pyfunction(name = "trivial_layout_prepared")]
#[pyo3(signature = (analysis, physical, objective=None))]
fn py_trivial_layout_prepared(
    py: Python<'_>,
    analysis: PyRef<'_, PyCircuitLayoutAnalysis>,
    physical: PyRef<'_, PyPhysicalLayoutGraph>,
    objective: Option<PyLayoutObjective>,
) -> PyResult<PyLayoutResult> {
    let analysis = analysis.inner.clone();
    let physical = physical.inner.clone();
    let objective = objective.map_or_else(LayoutObjective::topology_only, |value| value.inner);
    py.detach(move || trivial_layout_prepared(&analysis, &physical, &objective))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

#[pyfunction(name = "greedy_layout_prepared")]
#[pyo3(signature = (analysis, physical, objective=None))]
fn py_greedy_layout_prepared(
    py: Python<'_>,
    analysis: PyRef<'_, PyCircuitLayoutAnalysis>,
    physical: PyRef<'_, PyPhysicalLayoutGraph>,
    objective: Option<PyLayoutObjective>,
) -> PyResult<PyLayoutResult> {
    let analysis = analysis.inner.clone();
    let physical = physical.inner.clone();
    let objective = objective.map_or_else(LayoutObjective::topology_only, |value| value.inner);
    py.detach(move || greedy_layout_prepared(&analysis, &physical, &objective))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

#[pyfunction(name = "vf2_perfect_layout_prepared")]
#[pyo3(signature = (analysis, physical, objective=None, config=None))]
fn py_vf2_perfect_layout_prepared(
    py: Python<'_>,
    analysis: PyRef<'_, PyCircuitLayoutAnalysis>,
    physical: PyRef<'_, PyPhysicalLayoutGraph>,
    objective: Option<PyLayoutObjective>,
    config: Option<PyVf2LayoutConfig>,
) -> PyResult<PyLayoutResult> {
    let analysis = analysis.inner.clone();
    let physical = physical.inner.clone();
    let objective = objective.map_or_else(LayoutObjective::topology_only, |value| value.inner);
    let config = config.map_or_else(Vf2LayoutConfig::default, |value| value.inner);
    py.detach(move || vf2_perfect_layout_prepared(&analysis, &physical, &objective, &config))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

const fn python_bool(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}
