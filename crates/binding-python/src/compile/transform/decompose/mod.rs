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

pub(crate) mod config;
mod mc_gate;
mod pass;
mod unitary;

use super::result::PyDecompositionRuleStats;
use config::{PyMcGateDecomposeConfig, PyTwoQubitUnitaryDecomposeBasis, PyUnitaryDecomposeConfig};
use pass::{
    py_decompose_mc_gates, py_decompose_mc_gates_for_device, py_decompose_mc_gates_with_rule_stats,
    py_decompose_unitaries, py_decompose_unitaries_with_rule_stats, py_expand_definitions,
};
use pyo3::prelude::*;

/// Registers decomposition bindings as `_native.compile.transform.decompose`.
pub(crate) fn register_decompose_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "decompose")?;

    m.add_class::<PyTwoQubitUnitaryDecomposeBasis>()?;
    m.add_class::<PyUnitaryDecomposeConfig>()?;
    m.add_class::<PyMcGateDecomposeConfig>()?;
    m.add_class::<PyDecompositionRuleStats>()?;
    m.add_function(pyo3::wrap_pyfunction!(py_expand_definitions, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_decompose_unitaries, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        py_decompose_unitaries_with_rule_stats,
        &m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_decompose_mc_gates, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        py_decompose_mc_gates_with_rule_stats,
        &m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        py_decompose_mc_gates_for_device,
        &m
    )?)?;

    unitary::register_unitary_module(&m)?;
    mc_gate::register_mc_gate_module(&m)?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile.transform.decompose", &m)?;

    Ok(())
}
