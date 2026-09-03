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

//! Python bindings for exact ordered-qargs device instruction lowering.

use crate::circuit::PyCircuit;
use crate::compile::error::compiler_error_to_py_err;
use crate::compile::transform::PyTransformResult;
use crate::device::device_impl::PyDevice;
use cqlib_core::compile::transform::{DeviceLowerer, Transformer};
use pyo3::prelude::*;

/// Lowers a routed physical circuit to exact native device capabilities.
#[pyclass(
    name = "DeviceLowerer",
    module = "cqlib.compile.transform.device_lowering",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDeviceLowerer {
    device: PyDevice,
}

#[pymethods]
impl PyDeviceLowerer {
    #[new]
    fn new(device: PyRef<'_, PyDevice>) -> Self {
        Self {
            device: device.clone(),
        }
    }

    #[getter]
    fn device(&self) -> PyDevice {
        self.device.clone()
    }

    /// Applies exact native lowering without modifying the input circuit.
    fn run(&self, py: Python<'_>, circuit: PyRef<'_, PyCircuit>) -> PyResult<PyTransformResult> {
        let device = self.device.inner.clone();
        let circuit = circuit.inner.clone();
        py.detach(move || {
            let outcome = DeviceLowerer::new(&device).transform(&circuit, None)?;
            Ok(PyTransformResult::from_outcome(circuit, outcome))
        })
        .map_err(compiler_error_to_py_err)
    }

    fn __repr__(&self) -> String {
        format!(
            "DeviceLowerer(device=Device(name={:?}))",
            self.device.inner.name()
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

pub(crate) fn register_device_lowering_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(parent.py(), "device_lowering")?;
    module.add_class::<PyDeviceLowerer>()?;
    parent.add_submodule(&module)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile.transform.device_lowering", &module)?;
    Ok(())
}
