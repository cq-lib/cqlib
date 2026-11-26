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

//! Python bindings for cqlib-core entropy module.
//!
//! This module exposes quantum entropy and entanglement measures to Python.

use cqlib_core::qis::entropy as core_entropy;
use pyo3::prelude::*;

use crate::qis::qis_error_to_py_err;
use crate::qis::state::density_matrix::PyDensityMatrix;
use crate::qis::state::statevector::PyStatevector;

/// Calculates the linear entropy of a quantum state.
///
/// The linear entropy is a computationally efficient approximation of the
/// Von Neumann entropy. It serves as a measure of mixedness.
///
/// Args:
///     dm (DensityMatrix): The density matrix representing the quantum state.
///
/// Returns:
///     float: The linear entropy value in [0, 1).
///
/// Raises:
///     ValueError: If the calculation fails.
#[pyfunction]
pub fn linear_entropy(dm: &PyDensityMatrix) -> PyResult<f64> {
    core_entropy::linear_entropy(&dm.inner).map_err(qis_error_to_py_err)
}

/// Calculates the Rényi entropy of order alpha.
///
/// Args:
///     dm (DensityMatrix): The density matrix representing the quantum state.
///     alpha (float): Positive order, including 1 (Von Neumann entropy)
///         and positive infinity (min-entropy).
///
/// Returns:
///     float: The Rényi entropy in bits (base-2 logarithm).
///
/// Raises:
///     ValueError: If alpha is NaN or <= 0, eigendecomposition fails,
///         or no positive spectrum remains.
///
/// Notes:
///     Non-diagonal spectra discard eigenvalues at or below
///     dimension * machine epsilon * largest eigenvalue as numerical zeros.
///     Diagonal spectra retain all positive entries.
#[pyfunction]
pub fn renyi_entropy(dm: &PyDensityMatrix, alpha: f64) -> PyResult<f64> {
    core_entropy::renyi_entropy(&dm.inner, alpha).map_err(qis_error_to_py_err)
}

/// Calculates the entanglement entropy for a bipartite pure state.
///
/// Computes the Von Neumann entropy of the reduced density matrix of subsystem A.
///
/// Args:
///     sv (Statevector): The pure quantum state.
///     subsys_a (list[int]): Indices of qubits belonging to subsystem A.
///
/// Returns:
///     float: The entanglement entropy in bits.
///
/// Raises:
///     ValueError: If subsystem indices are invalid or out of bounds.
#[pyfunction]
pub fn entanglement_entropy_pure(sv: &PyStatevector, subsys_a: Vec<usize>) -> PyResult<f64> {
    core_entropy::entanglement_entropy_pure(&sv.inner, &subsys_a).map_err(qis_error_to_py_err)
}

/// Calculates the negativity entanglement measure.
///
/// Computes the negativity based on the partial transpose criterion.
///
/// Args:
///     dm (DensityMatrix): The density matrix of the bipartite state.
///     subsys_a (list[int]): Qubit indices comprising subsystem A.
///
/// Returns:
///     float: The negativity value (>= 0).
///
/// Raises:
///     ValueError: If subsystem indices are invalid or eigendecomposition fails.
#[pyfunction]
pub fn negativity(dm: &PyDensityMatrix, subsys_a: Vec<usize>) -> PyResult<f64> {
    core_entropy::negativity(&dm.inner, &subsys_a).map_err(qis_error_to_py_err)
}

/// Calculates the concurrence for a 2-qubit quantum state.
///
/// Args:
///     dm (DensityMatrix): The density matrix of a 2-qubit state.
///
/// Returns:
///     float: The concurrence value in [0, 1].
///
/// Raises:
///     ValueError: If the state does not have exactly 2 qubits or calculation fails.
#[pyfunction]
pub fn concurrence(dm: &PyDensityMatrix) -> PyResult<f64> {
    core_entropy::concurrence(&dm.inner).map_err(qis_error_to_py_err)
}

/// Calculates the entanglement of formation for a 2-qubit state.
///
/// Args:
///     dm (DensityMatrix): The density matrix of a 2-qubit state.
///
/// Returns:
///     float: The entanglement of formation in bits.
///
/// Raises:
///     ValueError: If the state does not have exactly 2 qubits.
#[pyfunction]
pub fn entanglement_of_formation(dm: &PyDensityMatrix) -> PyResult<f64> {
    core_entropy::entanglement_of_formation(&dm.inner).map_err(qis_error_to_py_err)
}

/// Register the entropy module with Python.
pub fn register_entropy_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let entropy_module = PyModule::new(parent.py(), "entropy")?;

    entropy_module.add_function(wrap_pyfunction!(linear_entropy, &entropy_module)?)?;
    entropy_module.add_function(wrap_pyfunction!(renyi_entropy, &entropy_module)?)?;
    entropy_module.add_function(wrap_pyfunction!(
        entanglement_entropy_pure,
        &entropy_module
    )?)?;
    entropy_module.add_function(wrap_pyfunction!(negativity, &entropy_module)?)?;
    entropy_module.add_function(wrap_pyfunction!(concurrence, &entropy_module)?)?;
    entropy_module.add_function(wrap_pyfunction!(
        entanglement_of_formation,
        &entropy_module
    )?)?;

    parent.add_submodule(&entropy_module)?;
    Ok(())
}
