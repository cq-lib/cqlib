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

//! Python bindings for deterministic target-basis lowering and exact costs.

use crate::circuit::bit::PyIntOrQubit;
use crate::circuit::{PyCircuit, PyInstruction, PyStandardGate, PyValueOperation};
use crate::compile::error::compiler_error_to_py_err;
use crate::compile::target_basis_item::PyTargetBasisItem;
use crate::compile::transform::PyTransformResult;
use crate::utils::{hash_value, python_string_literal};
use cqlib_core::compile::transform::{
    TargetBasisCost, TargetBasisCostModel, TargetBasisLowerer, TargetBasisSignature, Transformer,
};
use pyo3::prelude::*;

/// Deterministic lowerer for one explicit standard-instruction basis.
#[pyclass(
    name = "TargetBasisLowerer",
    module = "cqlib.compile.transform.target_basis",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTargetBasisLowerer {
    inner: TargetBasisLowerer,
}

#[pymethods]
impl PyTargetBasisLowerer {
    /// Basis entries are either case-insensitive standard-gate names (e.g.
    /// `'H'`, `'X2P'`) or `Instruction` objects for multi-controlled gates.
    #[new]
    fn new(py: Python<'_>, target_basis: Vec<PyTargetBasisItem>) -> PyResult<Self> {
        let target_basis = target_basis
            .into_iter()
            .map(PyTargetBasisItem::into_instruction)
            .collect::<PyResult<Vec<_>>>()?;
        py.detach(move || TargetBasisLowerer::new(target_basis))
            .map(|inner| Self { inner })
            .map_err(compiler_error_to_py_err)
    }

    #[getter]
    fn target_basis(&self) -> Vec<PyInstruction> {
        self.inner
            .target_basis()
            .iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    /// Lowers a circuit without modifying the input.
    fn run(&self, py: Python<'_>, circuit: PyRef<'_, PyCircuit>) -> PyResult<PyTransformResult> {
        let lowerer = self.inner.clone();
        let circuit = circuit.inner.clone();
        py.detach(move || {
            let outcome = lowerer.transform(&circuit, None)?;
            Ok(PyTransformResult::from_outcome(circuit, outcome))
        })
        .map_err(compiler_error_to_py_err)
    }

    fn __repr__(&self) -> String {
        format!(
            "TargetBasisLowerer(target_basis=[{}])",
            self.inner
                .target_basis()
                .iter()
                .map(|instruction| python_string_literal(&instruction.name()))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Canonical, order- and duplicate-insensitive target-basis identity.
#[pyclass(
    name = "TargetBasisSignature",
    module = "cqlib.compile.transform.target_basis",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTargetBasisSignature {
    inner: TargetBasisSignature,
}

impl From<TargetBasisSignature> for PyTargetBasisSignature {
    fn from(inner: TargetBasisSignature) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTargetBasisSignature {
    #[staticmethod]
    fn from_standard_gates(gates: Vec<PyStandardGate>) -> Self {
        let gates = gates.into_iter().map(|gate| gate.inner).collect::<Vec<_>>();
        TargetBasisSignature::from_standard_gates(&gates).into()
    }

    fn __repr__(&self) -> String {
        format!("TargetBasisSignature(hash={})", self.__hash__())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Exact operation and depth cost after target-basis lowering.
#[pyclass(
    name = "TargetBasisCost",
    module = "cqlib.compile.transform.target_basis",
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyTargetBasisCost {
    inner: TargetBasisCost,
}

impl From<TargetBasisCost> for PyTargetBasisCost {
    fn from(inner: TargetBasisCost) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTargetBasisCost {
    #[getter]
    fn two_qubit_ops(&self) -> usize {
        self.inner.two_qubit_ops
    }

    #[getter]
    fn depth(&self) -> usize {
        self.inner.depth
    }

    #[getter]
    fn total_ops(&self) -> usize {
        self.inner.total_ops
    }

    #[getter]
    fn parameterized_ops(&self) -> usize {
        self.inner.parameterized_ops
    }

    fn __repr__(&self) -> String {
        format!(
            "TargetBasisCost(two_qubit_ops={}, depth={}, total_ops={}, parameterized_ops={})",
            self.inner.two_qubit_ops,
            self.inner.depth,
            self.inner.total_ops,
            self.inner.parameterized_ops
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

/// Reusable exact cost evaluator for one target basis.
#[pyclass(
    name = "TargetBasisCostModel",
    module = "cqlib.compile.transform.target_basis",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTargetBasisCostModel {
    inner: TargetBasisCostModel,
}

#[pymethods]
impl PyTargetBasisCostModel {
    /// Basis entries are either case-insensitive standard-gate names (e.g.
    /// `'H'`, `'X2P'`) or `Instruction` objects; the basis must contain only
    /// standard instructions.
    #[new]
    fn new(py: Python<'_>, target_basis: Vec<PyTargetBasisItem>) -> PyResult<Self> {
        let target_basis = target_basis
            .into_iter()
            .map(PyTargetBasisItem::into_instruction)
            .collect::<PyResult<Vec<_>>>()?;
        py.detach(move || TargetBasisCostModel::new(target_basis))
            .map(|inner| Self { inner })
            .map_err(compiler_error_to_py_err)
    }

    #[getter]
    fn signature(&self) -> PyTargetBasisSignature {
        self.inner.signature().clone().into()
    }

    #[getter]
    fn target_basis(&self) -> Vec<PyInstruction> {
        self.inner
            .target_basis()
            .iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "TargetBasisCostModel(target_basis=[{}])",
            self.inner
                .target_basis()
                .iter()
                .map(|instruction| python_string_literal(&instruction.name()))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn cost_of_fixed_operations(
        &self,
        py: Python<'_>,
        qubits: Vec<PyIntOrQubit>,
        operations: Vec<PyValueOperation>,
    ) -> PyResult<PyTargetBasisCost> {
        let model = self.inner.clone();
        let qubits = qubits.into_iter().map(Into::into).collect();
        let operations = operations
            .into_iter()
            .map(|operation| operation.inner)
            .collect();
        py.detach(move || model.cost_of_fixed_operations(qubits, operations))
            .map(Into::into)
            .map_err(compiler_error_to_py_err)
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

pub(crate) fn register_target_basis_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(parent.py(), "target_basis")?;
    module.add_class::<PyTargetBasisLowerer>()?;
    module.add_class::<PyTargetBasisSignature>()?;
    module.add_class::<PyTargetBasisCost>()?;
    module.add_class::<PyTargetBasisCostModel>()?;
    parent.add_submodule(&module)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile.transform.target_basis", &module)?;
    Ok(())
}
