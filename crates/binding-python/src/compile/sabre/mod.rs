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

//! Python bindings for SABRE qubit routing.

pub mod routing;

pub use routing::{
    PySabreConfig, PySabreHeuristicConfig, PySabreRoutingDiagnostics, PySabreRoutingResult,
    PySabreVf2PrepassConfig, py_normalize_initial_layout, py_sabre_route,
    py_validate_reachable_interactions,
};

use pyo3::prelude::*;

/// Registers SABRE bindings as `_native.compile.sabre`.
pub(crate) fn register_sabre_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "sabre")?;

    m.add_class::<PySabreHeuristicConfig>()?;
    m.add_class::<PySabreVf2PrepassConfig>()?;
    m.add_class::<PySabreConfig>()?;
    m.add_class::<PySabreRoutingDiagnostics>()?;
    m.add_class::<PySabreRoutingResult>()?;
    m.add_function(pyo3::wrap_pyfunction!(py_sabre_route, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_normalize_initial_layout, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        py_validate_reachable_interactions,
        &m
    )?)?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile.sabre", &m)?;

    Ok(())
}
