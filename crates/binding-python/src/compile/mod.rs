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

//! Python bindings for the cqlib compiler pipeline.

pub mod commutation;
pub mod compiler;
pub mod error;
pub mod knowledge;
pub mod resource;
pub mod sabre;
pub mod target_basis_item;
pub mod transform;

pub use compiler::{
    PyCompileConfig, PyCompileMode, PyCompileResult, PyCompileTarget, PyCompilerWorkflow,
    PyDeviceCompilationMetadata, PyDeviceCompileTarget, PyWorkflowStepReport, py_compile,
};

use pyo3::prelude::*;

/// Registers compiler bindings as `_native.compile`.
pub(crate) fn register_compile_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "compile")?;

    error::register_errors(&m)?;
    m.add_class::<PyCompileMode>()?;
    m.add_class::<PyDeviceCompileTarget>()?;
    m.add_class::<PyCompileTarget>()?;
    m.add_class::<PyCompileConfig>()?;
    m.add_class::<PyWorkflowStepReport>()?;
    m.add_class::<PyDeviceCompilationMetadata>()?;
    m.add_class::<PyCompileResult>()?;
    m.add_class::<PyCompilerWorkflow>()?;
    m.add_function(pyo3::wrap_pyfunction!(py_compile, &m)?)?;

    commutation::register_commutation_module(&m)?;
    knowledge::register_knowledge_module(&m)?;
    resource::register_resource_module(&m)?;
    sabre::register_sabre_module(&m)?;
    transform::register_transform_module(&m)?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile", &m)?;

    Ok(())
}
