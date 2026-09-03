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

use super::PyDecompositionRuleStats;
use super::config::{PyMcGateDecomposeConfig, PyUnitaryDecomposeConfig};
use crate::circuit::PyCircuit;
use crate::compile::error::compiler_error_to_py_err;
use crate::compile::resource::PyResourcePolicy;
use crate::compile::transform::PyTransformResult;
use crate::device::device_impl::PyDevice;
use cqlib_core::compile::transform::Transformer;
use cqlib_core::compile::transform::decompose::mc_gate::decompose_mc_gates_for_device;
use cqlib_core::compile::transform::decompose::{
    DecomposeDefinitions, DecomposeMcGates, DecomposeUnitaries, McGateDecomposeConfig,
    UnitaryDecomposeConfig, decompose_mc_gates_with_rule_stats,
    decompose_unitaries_with_rule_stats,
};
use pyo3::prelude::*;

/// Expands circuit-backed gate definitions without modifying the input.
#[pyfunction(name = "expand_definitions")]
pub fn py_expand_definitions(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
) -> PyResult<PyTransformResult> {
    let circuit = circuit.inner.clone();
    py.detach(move || {
        let outcome = DecomposeDefinitions.transform(&circuit, None)?;
        Ok(PyTransformResult::from_outcome(circuit, outcome))
    })
    .map_err(compiler_error_to_py_err)
}

/// Synthesizes matrix-backed one- and two-qubit unitary gates.
#[pyfunction(name = "decompose_unitaries")]
#[pyo3(signature = (circuit, config=None))]
pub fn py_decompose_unitaries(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    config: Option<PyUnitaryDecomposeConfig>,
) -> PyResult<PyTransformResult> {
    let circuit = circuit.inner.clone();
    let config = config.map_or_else(UnitaryDecomposeConfig::default, |value| value.inner);
    py.detach(move || {
        let outcome = DecomposeUnitaries::new(config).transform(&circuit, None)?;
        Ok(PyTransformResult::from_outcome(circuit, outcome))
    })
    .map_err(compiler_error_to_py_err)
}

/// Synthesizes matrix-backed unitaries and returns pass-local rule-cache stats.
#[pyfunction(name = "decompose_unitaries_with_rule_stats")]
#[pyo3(signature = (circuit, config=None))]
pub fn py_decompose_unitaries_with_rule_stats(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    config: Option<PyUnitaryDecomposeConfig>,
) -> PyResult<(PyTransformResult, PyDecompositionRuleStats)> {
    let circuit = circuit.inner.clone();
    let config = config.map_or_else(UnitaryDecomposeConfig::default, |value| value.inner);
    py.detach(move || {
        let (outcome, stats) = decompose_unitaries_with_rule_stats(&circuit, config)?;
        Ok((
            PyTransformResult::from_outcome(circuit, outcome),
            stats.into(),
        ))
    })
    .map_err(compiler_error_to_py_err)
}

/// Decomposes multi-controlled gates using configured ancillary resources.
///
/// This entry point has no target device and therefore does not enforce device
/// capacity. Use `decompose_mc_gates_for_device` when capacity must be checked.
#[pyfunction(name = "decompose_mc_gates")]
#[pyo3(signature = (circuit, config=None))]
pub fn py_decompose_mc_gates(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    config: Option<PyMcGateDecomposeConfig>,
) -> PyResult<PyTransformResult> {
    let circuit = circuit.inner.clone();
    let config = config.map_or_else(McGateDecomposeConfig::default, |value| value.inner);
    py.detach(move || {
        let outcome = DecomposeMcGates::new(config).transform(&circuit, None)?;
        Ok(PyTransformResult::from_outcome(circuit, outcome))
    })
    .map_err(compiler_error_to_py_err)
}

/// Decomposes multi-controlled gates and returns pass-local rule-cache stats.
///
/// This entry point has no target device and therefore does not enforce device
/// capacity. Use `decompose_mc_gates_for_device` when capacity must be checked.
#[pyfunction(name = "decompose_mc_gates_with_rule_stats")]
#[pyo3(signature = (circuit, config=None))]
pub fn py_decompose_mc_gates_with_rule_stats(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    config: Option<PyMcGateDecomposeConfig>,
) -> PyResult<(PyTransformResult, PyDecompositionRuleStats)> {
    let circuit = circuit.inner.clone();
    let config = config.map_or_else(McGateDecomposeConfig::default, |value| value.inner);
    py.detach(move || {
        let (outcome, stats) = decompose_mc_gates_with_rule_stats(&circuit, config)?;
        Ok((
            PyTransformResult::from_outcome(circuit, outcome),
            stats.into(),
        ))
    })
    .map_err(compiler_error_to_py_err)
}

/// Decomposes multi-controlled gates while enforcing device capacity.
#[pyfunction(name = "decompose_mc_gates_for_device")]
#[pyo3(signature = (circuit, device, resource_policy=None))]
pub fn py_decompose_mc_gates_for_device(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    device: PyRef<'_, PyDevice>,
    resource_policy: Option<PyResourcePolicy>,
) -> PyResult<PyTransformResult> {
    let circuit = circuit.inner.clone();
    let device = device.inner.clone();
    let resource_policy = resource_policy.map_or_else(Default::default, |value| value.inner);
    py.detach(move || {
        let outcome = decompose_mc_gates_for_device(&circuit, &device, resource_policy)?;
        Ok(PyTransformResult::from_outcome(circuit, outcome))
    })
    .map_err(compiler_error_to_py_err)
}
