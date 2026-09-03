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

//! Python bindings for cqlib-core quantum information (qis) module.

use cqlib_core::qis::error::QisError;
use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::prelude::*;

pub mod entropy;
pub mod evolution;
pub mod hamiltonian;
pub mod metrics;
pub mod pauli;
pub mod state;

/// Converts a QisError to the appropriate Python exception.
pub(crate) fn qis_error_to_py_err(err: QisError) -> PyErr {
    match err {
        QisError::IndexOutOfBounds { .. } => PyIndexError::new_err(err.to_string()),
        _ => PyValueError::new_err(err.to_string()),
    }
}

/// Register the qis submodule.
pub fn register_qis_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let qis_module = PyModule::new(parent.py(), "qis")?;

    // Register evolution types
    qis_module.add_class::<evolution::PyTrotterMode>()?;

    // Register Hamiltonian and Pauli types
    qis_module.add_class::<hamiltonian::PyHamiltonian>()?;
    qis_module.add_class::<pauli::PyPhase>()?;
    qis_module.add_class::<pauli::PyPauli>()?;
    qis_module.add_class::<pauli::PyPauliString>()?;
    qis_module.add_class::<pauli::PyPauliStringIter>()?;

    // Register state submodule
    state::register_state_module(&qis_module)?;

    // Register entropy and metrics submodules
    entropy::register_entropy_module(&qis_module)?;
    metrics::register_metrics_module(&qis_module)?;

    parent.add_submodule(&qis_module)?;
    Ok(())
}
