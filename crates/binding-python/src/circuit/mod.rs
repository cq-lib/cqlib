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

//! Python bindings for the foundational circuit IR types.
//!
//! Module declarations may include work-in-progress wrappers, but
//! [`register_circuit_module`] exposes only types that have completed API,
//! documentation, error-mapping, and test review.

pub mod ansatz;
pub mod bit;
pub mod circuit_impl;
pub mod circuit_to_matrix;
pub mod classical;
pub mod classical_expr;
pub mod control_flow;
pub mod dag;
pub mod error;
pub mod gate;
pub mod instruction;
pub mod operation;
pub mod parameter;
pub mod symbolic_matrix;

pub use ansatz::{
    PyAngleEncoding, PyBasicEntanglerLayers, PyBasisEncoding, PyEvolutionInfo, PyEvolutionStrategy,
    PyIQPFeatureMap, PyPauliEvolutionAnsatz, PyPauliFeatureMap, PyQAOAAnsatz,
    PyStronglyEntanglingLayers, PyTwoLocal, PyZFeatureMap, PyZZFeatureMap,
};
pub use bit::PyQubit;
pub use circuit_impl::PyCircuit;
pub use circuit_to_matrix::py_circuit_to_matrix;
pub use classical::{
    PyCircuitId, PyClassicalType, PyClassicalValue, PyClassicalVar, PyMeasurement,
};
pub use classical_expr::PyClassicalExpr;
pub use control_flow::{
    PyClassicalControlOp, PySwitchBuilder, PyValueControlBody, PyValueSwitchCase,
};
pub use dag::{PyCircuitDag, PyDagControlFlow, PyDagSwitchCase, PyDagWire};
pub use gate::{
    PyCircuitGate, PyDirective, PyFrozenCircuit, PyMcGate, PyStandardGate, PyUnitaryGate,
};
pub use instruction::{PyInstruction, PyValueInstruction};
pub use operation::{PyOperation, PyValueOperation};
pub use parameter::PyParameter;
pub use symbolic_matrix::{PySymbolicComplex, PySymbolicMatrix};

use pyo3::prelude::*;

/// Registers the reviewed foundational circuit API as `_native.circuit`.
///
/// Keep this list explicit. A wrapper must not become public merely because its
/// Rust module compiles; it is added here only after its Python API is confirmed.
pub(crate) fn register_circuit_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "circuit")?;

    error::register_errors(&m)?;
    m.add_class::<PyQubit>()?;
    m.add_class::<PyParameter>()?;
    m.add_class::<PyCircuitId>()?;
    m.add_class::<PyClassicalType>()?;
    m.add_class::<PyClassicalVar>()?;
    m.add_class::<PyClassicalValue>()?;
    m.add_class::<PyMeasurement>()?;
    m.add_class::<PyClassicalExpr>()?;
    m.add_class::<PySymbolicComplex>()?;
    m.add_class::<PySymbolicMatrix>()?;
    m.add_class::<PyInstruction>()?;
    m.add_class::<PyValueInstruction>()?;
    m.add_class::<PyValueOperation>()?;
    m.add_class::<PyValueControlBody>()?;
    m.add_class::<PyValueSwitchCase>()?;
    m.add_class::<PyClassicalControlOp>()?;
    m.add_class::<PySwitchBuilder>()?;
    m.add_class::<PyDagWire>()?;
    m.add_class::<PyDagSwitchCase>()?;
    m.add_class::<PyDagControlFlow>()?;
    m.add_class::<PyCircuitDag>()?;
    m.add_class::<PyCircuit>()?;
    m.add_function(pyo3::wrap_pyfunction!(py_circuit_to_matrix, &m)?)?;
    gate::register_gate_classes(&m)?;
    ansatz::register_ansatz_module(&m)?;

    // PyO3 submodules must also be inserted into sys.modules for direct imports.
    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.circuit", &m)?;

    Ok(())
}
