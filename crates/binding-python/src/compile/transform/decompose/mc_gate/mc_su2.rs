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

use super::{compiler_error, into_py_operations};
use crate::circuit::bit::{PyIntListOrQubitList, PyIntOrQubit};
use crate::circuit::operation::extract_parameter_value;
use crate::circuit::{PyStandardGate, PyValueOperation};
use crate::utils::hash_value;
use cqlib_core::circuit::{ParameterValue, Qubit};
use cqlib_core::compile::transform::decompose::mc_gate::{
    Su2RotationAxis, decompose_mc_su2_n_clean, decompose_mc_su2_no_aux,
};
use pyo3::prelude::*;

/// Axis of a multi-controlled special-unitary rotation.
#[pyclass(
    name = "Su2RotationAxis",
    module = "cqlib.compile.transform.decompose.mc_gate",
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PySu2RotationAxis {
    inner: Su2RotationAxis,
}

#[pymethods]
impl PySu2RotationAxis {
    #[staticmethod]
    fn x() -> Self {
        Self {
            inner: Su2RotationAxis::X,
        }
    }

    #[staticmethod]
    fn y() -> Self {
        Self {
            inner: Su2RotationAxis::Y,
        }
    }

    #[staticmethod]
    fn z() -> Self {
        Self {
            inner: Su2RotationAxis::Z,
        }
    }

    #[getter]
    fn rotation_gate(&self) -> PyStandardGate {
        PyStandardGate::from(self.inner.rotation_gate(), Vec::new())
    }

    #[getter]
    fn controlled_rotation_gate(&self) -> PyStandardGate {
        PyStandardGate::from(self.inner.controlled_rotation_gate(), Vec::new())
    }

    fn __repr__(&self) -> &'static str {
        match self.inner {
            Su2RotationAxis::X => "Su2RotationAxis.x()",
            Su2RotationAxis::Y => "Su2RotationAxis.y()",
            Su2RotationAxis::Z => "Su2RotationAxis.z()",
        }
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

#[pyfunction(name = "decompose_mc_su2_no_aux")]
fn py_decompose_mc_su2_no_aux(
    py: Python<'_>,
    axis: PySu2RotationAxis,
    theta: Bound<'_, PyAny>,
    controls: PyIntListOrQubitList,
    target: PyIntOrQubit,
) -> PyResult<Vec<PyValueOperation>> {
    let theta: ParameterValue = extract_parameter_value(&theta)?;
    let controls: Vec<Qubit> = controls.into();
    let target: Qubit = target.into();
    py.detach(move || decompose_mc_su2_no_aux(axis.inner, &theta, &controls, target))
        .map(into_py_operations)
        .map_err(compiler_error)
}

#[pyfunction(name = "decompose_mc_su2_n_clean")]
fn py_decompose_mc_su2_n_clean(
    py: Python<'_>,
    axis: PySu2RotationAxis,
    theta: Bound<'_, PyAny>,
    controls: PyIntListOrQubitList,
    target: PyIntOrQubit,
    clean_ancillas: PyIntListOrQubitList,
) -> PyResult<Vec<PyValueOperation>> {
    let theta: ParameterValue = extract_parameter_value(&theta)?;
    let controls: Vec<Qubit> = controls.into();
    let target: Qubit = target.into();
    let clean_ancillas: Vec<Qubit> = clean_ancillas.into();
    py.detach(move || {
        decompose_mc_su2_n_clean(axis.inner, &theta, &controls, target, &clean_ancillas)
    })
    .map(into_py_operations)
    .map_err(compiler_error)
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PySu2RotationAxis>()?;
    module.add_function(pyo3::wrap_pyfunction!(py_decompose_mc_su2_no_aux, module)?)?;
    module.add_function(pyo3::wrap_pyfunction!(py_decompose_mc_su2_n_clean, module)?)?;
    Ok(())
}
