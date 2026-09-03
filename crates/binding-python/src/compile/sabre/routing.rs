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
use crate::device::device_impl::PyDevice;
use crate::device::layout::PyLayout;
use crate::device::qubit::PyLogicalQubitList;
use cqlib_core::compile::sabre::{
    SabreConfig, SabreHeuristicConfig, SabreRoutingDiagnostics, SabreRoutingResult,
    SabreVf2PrepassConfig, normalize_initial_layout, sabre_route, validate_reachable_interactions,
};
use pyo3::prelude::*;

/// Bounded VF2 prepass used to seed SABRE layout candidates.
#[pyclass(
    name = "SabreVf2PrepassConfig",
    module = "cqlib.compile.sabre",
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PySabreVf2PrepassConfig {
    inner: SabreVf2PrepassConfig,
}

impl From<SabreVf2PrepassConfig> for PySabreVf2PrepassConfig {
    fn from(inner: SabreVf2PrepassConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySabreVf2PrepassConfig {
    #[new]
    #[pyo3(signature = (*, candidate_limit=10, call_limit=1_000_000))]
    fn new(candidate_limit: usize, call_limit: usize) -> Self {
        Self {
            inner: SabreVf2PrepassConfig {
                candidate_limit,
                call_limit,
            },
        }
    }

    #[getter]
    fn candidate_limit(&self) -> usize {
        self.inner.candidate_limit
    }

    #[getter]
    fn call_limit(&self) -> usize {
        self.inner.call_limit
    }

    fn __repr__(&self) -> String {
        format!(
            "SabreVf2PrepassConfig(candidate_limit={}, call_limit={})",
            self.inner.candidate_limit, self.inner.call_limit
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }
}

/// Swap-selection weights and fallback limits used by SABRE.
///
/// Structural distance uses active-layer-normalized lookahead and
/// multiplicative congestion control. Exact native 2Q cost resolves
/// candidates within a narrow structural window.
#[pyclass(
    name = "SabreHeuristicConfig",
    module = "cqlib.compile.sabre",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySabreHeuristicConfig {
    inner: SabreHeuristicConfig,
}

impl From<SabreHeuristicConfig> for PySabreHeuristicConfig {
    fn from(inner: SabreHeuristicConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySabreHeuristicConfig {
    /// Creates a SABRE swap-selection configuration.
    #[new]
    #[pyo3(signature = (*, basic_weight=1.0, lookahead_weights=None, decay_increment=Some(0.002), decay_reset=10, attempt_limit=1000, best_epsilon=1e-10))]
    fn new(
        basic_weight: f64,
        lookahead_weights: Option<Vec<f64>>,
        decay_increment: Option<f64>,
        decay_reset: usize,
        attempt_limit: usize,
        best_epsilon: f64,
    ) -> Self {
        Self {
            inner: SabreHeuristicConfig {
                basic_weight,
                lookahead_weights: lookahead_weights
                    .unwrap_or_else(|| vec![0.5, 0.25, 0.125, 0.0625, 0.03125]),
                decay_increment,
                decay_reset,
                attempt_limit,
                best_epsilon,
            },
        }
    }

    #[getter]
    fn basic_weight(&self) -> f64 {
        self.inner.basic_weight
    }

    #[getter]
    fn lookahead_weights(&self) -> Vec<f64> {
        self.inner.lookahead_weights.clone()
    }

    #[getter]
    fn decay_increment(&self) -> Option<f64> {
        self.inner.decay_increment
    }

    #[getter]
    fn decay_reset(&self) -> usize {
        self.inner.decay_reset
    }

    #[getter]
    fn attempt_limit(&self) -> usize {
        self.inner.attempt_limit
    }

    #[getter]
    fn best_epsilon(&self) -> f64 {
        self.inner.best_epsilon
    }

    fn __repr__(&self) -> String {
        format!(
            "SabreHeuristicConfig(basic_weight={}, lookahead_weights={:?}, decay_increment={:?}, decay_reset={}, attempt_limit={}, best_epsilon={})",
            self.inner.basic_weight,
            self.inner.lookahead_weights,
            self.inner.decay_increment,
            self.inner.decay_reset,
            self.inner.attempt_limit,
            self.inner.best_epsilon,
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

/// Configuration shared by SABRE layout refinement and routing.
#[pyclass(name = "SabreConfig", module = "cqlib.compile.sabre", from_py_object)]
#[derive(Clone, Debug)]
pub struct PySabreConfig {
    pub(crate) inner: SabreConfig,
}

impl From<SabreConfig> for PySabreConfig {
    fn from(inner: SabreConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySabreConfig {
    /// Creates a SABRE configuration using core defaults for omitted objects.
    ///
    /// ``routing_trials`` counts the random routing trials run from each final
    /// refined layout. Automatic layout-and-route search directly returns the
    /// best of these routes; it does not run a separate layout-scoring phase.
    #[new]
    #[pyo3(signature = (*, layout_trials=10, layout_assignment_budget=1_000_000, vf2_prepass=Some(PySabreVf2PrepassConfig::from(SabreVf2PrepassConfig { candidate_limit: 10, call_limit: 1_000_000 })), refinement_iterations=1, routing_trials=1, seed=None, heuristic=None))]
    fn new(
        layout_trials: usize,
        layout_assignment_budget: usize,
        vf2_prepass: Option<PySabreVf2PrepassConfig>,
        refinement_iterations: usize,
        routing_trials: usize,
        seed: Option<u64>,
        heuristic: Option<PySabreHeuristicConfig>,
    ) -> Self {
        Self {
            inner: SabreConfig {
                layout_trials,
                layout_assignment_budget,
                vf2_prepass: vf2_prepass.map(|value| value.inner),
                refinement_iterations,
                routing_trials,
                seed,
                heuristic: heuristic
                    .map_or_else(SabreHeuristicConfig::default, |value| value.inner),
            },
        }
    }

    /// Returns a compact deterministic configuration for tests and examples.
    #[staticmethod]
    fn deterministic_seeded(seed: u64) -> Self {
        SabreConfig::deterministic_seeded(seed).into()
    }

    #[getter]
    fn layout_trials(&self) -> usize {
        self.inner.layout_trials
    }

    #[getter]
    fn layout_assignment_budget(&self) -> usize {
        self.inner.layout_assignment_budget
    }

    #[getter]
    fn vf2_prepass(&self) -> Option<PySabreVf2PrepassConfig> {
        self.inner.vf2_prepass.map(Into::into)
    }

    #[getter]
    fn refinement_iterations(&self) -> usize {
        self.inner.refinement_iterations
    }

    #[getter]
    fn routing_trials(&self) -> usize {
        self.inner.routing_trials
    }

    #[getter]
    fn seed(&self) -> Option<u64> {
        self.inner.seed
    }

    #[getter]
    fn heuristic(&self) -> PySabreHeuristicConfig {
        self.inner.heuristic.clone().into()
    }

    /// Validates routing-specific configuration fields.
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(compiler_error_to_py_err)
    }

    fn __repr__(&self) -> String {
        format!(
            "SabreConfig(layout_trials={}, layout_assignment_budget={}, vf2_prepass={:?}, refinement_iterations={}, routing_trials={}, seed={:?}, heuristic={:?})",
            self.inner.layout_trials,
            self.inner.layout_assignment_budget,
            self.inner.vf2_prepass,
            self.inner.refinement_iterations,
            self.inner.routing_trials,
            self.inner.seed,
            self.inner.heuristic,
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

/// Diagnostics emitted by a completed SABRE routing run.
#[pyclass(
    name = "SabreRoutingDiagnostics",
    module = "cqlib.compile.sabre",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySabreRoutingDiagnostics {
    inner: SabreRoutingDiagnostics,
}

impl From<SabreRoutingDiagnostics> for PySabreRoutingDiagnostics {
    fn from(inner: SabreRoutingDiagnostics) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySabreRoutingDiagnostics {
    #[getter]
    fn trials_evaluated(&self) -> usize {
        self.inner.trials_evaluated
    }

    #[getter]
    fn selected_trial_index(&self) -> usize {
        self.inner.selected_trial_index
    }

    #[getter]
    fn fallback_count(&self) -> usize {
        self.inner.fallback_count
    }

    #[getter]
    fn control_flow_blocks_routed(&self) -> usize {
        self.inner.control_flow_blocks_routed
    }

    #[getter]
    fn two_qubit_depth(&self) -> usize {
        self.inner.two_qubit_depth
    }

    #[getter]
    fn operation_count(&self) -> usize {
        self.inner.operation_count
    }

    #[getter]
    fn native_two_qubit_count(&self) -> usize {
        self.inner.native_two_qubit_count
    }

    #[getter]
    fn native_two_qubit_depth(&self) -> usize {
        self.inner.native_two_qubit_depth
    }

    #[getter]
    fn native_total_depth(&self) -> usize {
        self.inner.native_total_depth
    }

    #[getter]
    fn native_operation_count(&self) -> usize {
        self.inner.native_operation_count
    }

    #[getter]
    fn unknown_loop_count(&self) -> usize {
        self.inner.unknown_loop_count
    }

    #[getter]
    fn requirement_signature_count(&self) -> usize {
        self.inner.requirement_signature_count
    }

    #[getter]
    fn eager_pair_state_count(&self) -> usize {
        self.inner.eager_pair_state_count
    }

    #[getter]
    fn lazy_pair_l1_lookup_count(&self) -> usize {
        self.inner.lazy_pair_l1_lookup_count
    }

    #[getter]
    fn lazy_pair_l1_hit_count(&self) -> usize {
        self.inner.lazy_pair_l1_hit_count
    }

    #[getter]
    fn lazy_pair_l1_cached_count(&self) -> usize {
        self.inner.lazy_pair_l1_cached_count
    }

    fn __repr__(&self) -> String {
        format!(
            "SabreRoutingDiagnostics(trials_evaluated={}, selected_trial_index={}, fallback_count={}, control_flow_blocks_routed={}, two_qubit_depth={}, operation_count={})",
            self.inner.trials_evaluated,
            self.inner.selected_trial_index,
            self.inner.fallback_count,
            self.inner.control_flow_blocks_routed,
            self.inner.two_qubit_depth,
            self.inner.operation_count,
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

/// Normalizes a complete logical-to-physical layout against usable device qubits.
#[pyfunction(name = "normalize_initial_layout")]
pub fn py_normalize_initial_layout(
    py: Python<'_>,
    logical_qubits: PyLogicalQubitList,
    device: PyRef<'_, PyDevice>,
    initial_layout: PyRef<'_, PyLayout>,
) -> PyResult<PyLayout> {
    let logical_qubits = Vec::from(logical_qubits);
    let device = device.inner.clone();
    let initial_layout = initial_layout.inner.clone();
    py.detach(move || normalize_initial_layout(&logical_qubits, &device, &initial_layout))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

/// Validates native movement reachability without performing routing.
#[pyfunction(name = "validate_reachable_interactions")]
pub fn py_validate_reachable_interactions(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    device: PyRef<'_, PyDevice>,
    initial_layout: PyRef<'_, PyLayout>,
) -> PyResult<()> {
    let circuit = circuit.inner.clone();
    let device = device.inner.clone();
    let initial_layout = initial_layout.inner.clone();
    py.detach(move || validate_reachable_interactions(&circuit, &device, &initial_layout))
        .map_err(compiler_error_to_py_err)
}

/// Routed circuit, selected layouts, and routing diagnostics.
#[pyclass(
    name = "SabreRoutingResult",
    module = "cqlib.compile.sabre",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySabreRoutingResult {
    inner: SabreRoutingResult,
}

impl From<SabreRoutingResult> for PySabreRoutingResult {
    fn from(inner: SabreRoutingResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySabreRoutingResult {
    #[getter]
    fn circuit(&self) -> PyCircuit {
        self.inner.circuit.clone().into()
    }

    #[getter]
    fn initial_layout(&self) -> PyLayout {
        self.inner.initial_layout.clone().into()
    }

    #[getter]
    fn final_layout(&self) -> PyLayout {
        self.inner.final_layout.clone().into()
    }

    #[getter]
    fn swap_count(&self) -> usize {
        self.inner.swap_count
    }

    #[getter]
    fn diagnostics(&self) -> PySabreRoutingDiagnostics {
        self.inner.diagnostics.clone().into()
    }

    fn __repr__(&self) -> String {
        format!(
            "SabreRoutingResult(swap_count={}, diagnostics={:?})",
            self.inner.swap_count, self.inner.diagnostics
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Routes a circuit onto a device topology with the SABRE heuristic.
#[pyfunction(name = "sabre_route")]
#[pyo3(signature = (circuit, device, initial_layout, config=None))]
pub fn py_sabre_route(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    device: PyRef<'_, PyDevice>,
    initial_layout: PyRef<'_, PyLayout>,
    config: Option<PySabreConfig>,
) -> PyResult<PySabreRoutingResult> {
    let circuit = circuit.inner.clone();
    let device = device.inner.clone();
    let initial_layout = initial_layout.inner.clone();
    let config = config.map_or_else(SabreConfig::default, |value| value.inner);

    py.detach(move || sabre_route(&circuit, &device, &initial_layout, &config))
        .map(PySabreRoutingResult::from)
        .map_err(compiler_error_to_py_err)
}
