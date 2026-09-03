// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Python bindings for pre-routing basis legalization.

use crate::circuit::{PyCircuit, PyInstruction};
use crate::compile::error::compiler_error_to_py_err;
use crate::compile::transform::PyTransformResult;
use cqlib_core::compile::transform::{LowerToRoutingBasis, Transformer};
use pyo3::prelude::*;

/// Lowers gate-like operations to SABRE's 0/1/2-qubit input contract.
///
/// `preferred_basis` is a hint for the 2-qubit family used when lowering CCX:
/// CZ is preferred only when the basis contains CZ and does not contain CX.
#[pyclass(
    name = "LowerToRoutingBasis",
    module = "cqlib.compile.transform",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyLowerToRoutingBasis {
    inner: LowerToRoutingBasis,
    preferred_basis: Option<Vec<PyInstruction>>,
}

#[pymethods]
impl PyLowerToRoutingBasis {
    /// Creates a routing-basis lowering transform.
    ///
    /// The optional `preferred_basis` is not an exact output basis; final
    /// native-basis translation remains a separate compiler stage.
    #[new]
    #[pyo3(signature = (preferred_basis=None))]
    fn new(preferred_basis: Option<Vec<PyInstruction>>) -> Self {
        Self {
            inner: LowerToRoutingBasis::new(preferred_basis.as_ref().map(|basis| {
                basis
                    .iter()
                    .map(|instruction| instruction.inner.clone())
                    .collect()
            })),
            preferred_basis,
        }
    }

    #[getter]
    fn preferred_basis(&self) -> Option<Vec<PyInstruction>> {
        self.preferred_basis.clone()
    }

    /// Applies routing-basis lowering without modifying the input circuit.
    ///
    /// Raises a compiler error when gate-like operations with more than two
    /// qubits remain after lowering.
    fn run(&self, py: Python<'_>, circuit: PyRef<'_, PyCircuit>) -> PyResult<PyTransformResult> {
        let transform = self.inner.clone();
        let circuit = circuit.inner.clone();
        py.detach(move || {
            let outcome = transform.transform(&circuit, None)?;
            Ok(PyTransformResult::from_outcome(circuit, outcome))
        })
        .map_err(compiler_error_to_py_err)
    }

    fn __repr__(&self) -> String {
        let basis = self.preferred_basis.as_ref().map_or_else(
            || "None".to_string(),
            |basis| {
                format!(
                    "[{}]",
                    basis
                        .iter()
                        .map(|instruction| format!("Instruction({})", instruction.inner))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
        );
        format!("LowerToRoutingBasis(preferred_basis={basis})")
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Lowers gate-like operations to SABRE's 0/1/2-qubit input contract.
///
/// `preferred_basis` is a CCX-lowering hint, not an exact final output basis.
#[pyfunction(name = "lower_to_routing_basis")]
#[pyo3(signature = (circuit, preferred_basis=None))]
pub fn py_lower_to_routing_basis(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    preferred_basis: Option<Vec<PyInstruction>>,
) -> PyResult<PyTransformResult> {
    let circuit = circuit.inner.clone();
    let preferred_basis = preferred_basis.map(|basis| {
        basis
            .into_iter()
            .map(|instruction| instruction.inner)
            .collect()
    });
    py.detach(move || {
        let outcome = LowerToRoutingBasis::new(preferred_basis).transform(&circuit, None)?;
        Ok(PyTransformResult::from_outcome(circuit, outcome))
    })
    .map_err(compiler_error_to_py_err)
}
