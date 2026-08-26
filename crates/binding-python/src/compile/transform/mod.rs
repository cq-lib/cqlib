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

//! Python bindings for reusable compiler transforms.

mod analysis;
mod canonicalize;
mod commutative_cancellation;
pub mod decompose;
mod device_lowering;
pub mod layout;
mod native_optimization;
mod one_qubit_optimization;
pub mod result;
mod resynthesis;
mod rewrite;
pub mod routing;
mod routing_basis;
mod target_basis;
mod virtual_permutation;

use pyo3::prelude::*;

use analysis::register_analysis_module;
use canonicalize::{
    PyCanonicalizeConfig, PyCanonicalizeResult, PyCanonicalizer, py_canonicalize_circuit,
};
use device_lowering::register_device_lowering_module;
use native_optimization::register_native_optimization_module;
use one_qubit_optimization::register_one_qubit_optimization_module;
use result::PyTransformResult;
use resynthesis::register_resynthesis_module;
use rewrite::{
    PyKnowledgeRewriteResult, PyKnowledgeRewriteStats, PyKnowledgeRewriter, PyRewriteConfig,
    PyRewriteMode, py_rewrite_circuit,
};
use routing_basis::{PyLowerToRoutingBasis, py_lower_to_routing_basis};
use target_basis::register_target_basis_module;
use virtual_permutation::register_virtual_permutation_module;

/// Registers transform bindings as `_native.compile.transform`.
pub(crate) fn register_transform_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "transform")?;

    m.add_class::<PyCanonicalizeConfig>()?;
    m.add_class::<PyCanonicalizer>()?;
    m.add_class::<PyCanonicalizeResult>()?;
    m.add_class::<PyRewriteMode>()?;
    m.add_class::<PyRewriteConfig>()?;
    m.add_class::<PyKnowledgeRewriter>()?;
    m.add_class::<PyKnowledgeRewriteStats>()?;
    m.add_class::<PyKnowledgeRewriteResult>()?;
    m.add_class::<PyLowerToRoutingBasis>()?;
    m.add_class::<PyTransformResult>()?;
    m.add_function(pyo3::wrap_pyfunction!(py_canonicalize_circuit, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_rewrite_circuit, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_lower_to_routing_basis, &m)?)?;

    decompose::register_decompose_module(&m)?;
    register_analysis_module(&m)?;
    commutative_cancellation::register_commutative_cancellation_module(&m)?;
    register_device_lowering_module(&m)?;
    register_native_optimization_module(&m)?;
    register_one_qubit_optimization_module(&m)?;
    layout::register_layout_module(&m)?;
    routing::register_routing_module(&m)?;
    register_resynthesis_module(&m)?;
    register_target_basis_module(&m)?;
    register_virtual_permutation_module(&m)?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile.transform", &m)?;

    Ok(())
}
