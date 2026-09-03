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

//! Python bindings for device-aware circuit routing transforms.

use crate::circuit::PyCircuit;
use crate::compile::error::compiler_error_to_py_err;
use crate::compile::sabre::{PySabreConfig, PySabreRoutingDiagnostics};
use crate::compile::transform::layout::{PyLayoutDiagnostics, PyLayoutObjective, PyLayoutScore};
use crate::device::device_impl::PyDevice;
use crate::device::layout::PyLayout;
use cqlib_core::compile::sabre::SabreConfig;
use cqlib_core::compile::transform::LayoutObjective;
use cqlib_core::compile::transform::routing::{
    RoutedCircuit, SabreRouteResult, route_sabre, route_with_layout,
};
use pyo3::prelude::*;

/// Registers routing bindings as `_native.compile.transform.routing`.
pub(crate) fn register_routing_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "routing")?;

    m.add_class::<PyRoutedCircuit>()?;
    m.add_class::<PySabreRouteResult>()?;
    m.add_function(pyo3::wrap_pyfunction!(py_route_with_layout, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_route_sabre, &m)?)?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile.transform.routing", &m)?;

    Ok(())
}

/// Routes a circuit from a caller-supplied initial layout.
#[pyfunction(name = "route_with_layout")]
#[pyo3(signature = (circuit, device, initial_layout, config=None))]
fn py_route_with_layout(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    device: PyRef<'_, PyDevice>,
    initial_layout: PyRef<'_, PyLayout>,
    config: Option<PySabreConfig>,
) -> PyResult<PyRoutedCircuit> {
    let circuit = circuit.inner.clone();
    let device = device.inner.clone();
    let initial_layout = initial_layout.inner.clone();
    let config = config.map_or_else(SabreConfig::default, |value| value.inner);

    py.detach(move || route_with_layout(&circuit, &device, &initial_layout, &config))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

/// Selects a SABRE initial layout and routes a circuit for a device.
#[pyfunction(name = "route_sabre")]
#[pyo3(signature = (circuit, device, objective=None, config=None))]
fn py_route_sabre(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    device: PyRef<'_, PyDevice>,
    objective: Option<PyLayoutObjective>,
    config: Option<PySabreConfig>,
) -> PyResult<PySabreRouteResult> {
    let circuit = circuit.inner.clone();
    let device = device.inner.clone();
    let objective = objective.map_or_else(LayoutObjective::topology_only, |value| value.inner);
    let config = config.map_or_else(SabreConfig::default, |value| value.inner);

    py.detach(move || route_sabre(&circuit, &device, &objective, &config))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

/// A physical circuit produced by routing, plus routing metadata.
#[pyclass(
    name = "RoutedCircuit",
    module = "cqlib.compile.transform.routing",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRoutedCircuit {
    inner: RoutedCircuit,
}

impl From<RoutedCircuit> for PyRoutedCircuit {
    fn from(inner: RoutedCircuit) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRoutedCircuit {
    #[getter]
    fn circuit(&self) -> PyCircuit {
        self.inner.circuit().clone().into()
    }

    #[getter]
    fn initial_layout(&self) -> PyLayout {
        self.inner.initial_layout().clone().into()
    }

    #[getter]
    fn final_layout(&self) -> PyLayout {
        self.inner.final_layout().clone().into()
    }

    #[getter]
    fn swap_count(&self) -> usize {
        self.inner.swap_count()
    }

    #[getter]
    fn diagnostics(&self) -> PySabreRoutingDiagnostics {
        self.inner.diagnostics().clone().into()
    }

    /// Returns whether routing observably changed `original`.
    fn changed(&self, original: PyRef<'_, PyCircuit>) -> bool {
        self.inner.changed(&original.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "RoutedCircuit(swap_count={}, diagnostics={:?})",
            self.inner.swap_count(),
            self.inner.diagnostics(),
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

/// Full SABRE layout-selection and routing result.
///
/// Wraps the routed circuit and adds the observed objective score of the
/// winning layout. SABRE selects the winner by predicted native route
/// quality; the score is diagnostic and is not the route-selection key.
#[pyclass(
    name = "SabreRouteResult",
    module = "cqlib.compile.transform.routing",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySabreRouteResult {
    inner: SabreRouteResult,
}

impl From<SabreRouteResult> for PySabreRouteResult {
    fn from(inner: SabreRouteResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySabreRouteResult {
    #[getter]
    fn routed(&self) -> PyRoutedCircuit {
        self.inner.routed().clone().into()
    }

    /// Observed score of the selected initial layout, when available.
    ///
    /// SABRE selects the winner by predicted route quality under the prepared
    /// routing cost model; this score is diagnostic and is not the
    /// route-selection key.
    #[getter]
    fn layout_score(&self) -> Option<PyLayoutScore> {
        self.inner.layout_score().cloned().map(Into::into)
    }

    #[getter]
    fn layout_diagnostics(&self) -> PyLayoutDiagnostics {
        self.inner.layout_diagnostics().clone().into()
    }

    #[getter]
    fn circuit(&self) -> PyCircuit {
        self.inner.circuit().clone().into()
    }

    #[getter]
    fn initial_layout(&self) -> PyLayout {
        self.inner.initial_layout().clone().into()
    }

    #[getter]
    fn final_layout(&self) -> PyLayout {
        self.inner.final_layout().clone().into()
    }

    #[getter]
    fn swap_count(&self) -> usize {
        self.inner.swap_count()
    }

    #[getter]
    fn diagnostics(&self) -> PySabreRoutingDiagnostics {
        self.inner.diagnostics().clone().into()
    }

    /// Returns whether routing observably changed `original`.
    fn changed(&self, original: PyRef<'_, PyCircuit>) -> bool {
        self.inner.changed(&original.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "SabreRouteResult(swap_count={}, layout_score={:?}, diagnostics={:?})",
            self.inner.swap_count(),
            self.inner.layout_score(),
            self.inner.diagnostics(),
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
