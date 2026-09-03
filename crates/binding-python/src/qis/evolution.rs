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

//! Python bindings for cqlib-core evolution module.

use crate::utils::hash_value;
use cqlib_core::qis::evolution::TrotterMode;
use pyo3::prelude::*;

/// Trotter-Suzuki decomposition modes for Hamiltonian time evolution.
///
/// These modes determine how the time evolution operator U(t) = e^(-iHt) is
/// approximated as a product of Pauli rotations.
///
/// Variants:
///     FirstOrder: First-order Lie-Trotter decomposition. Error scales as O(t^2/n).
///     SecondOrder: Second-order Strang splitting (symmetric). Error scales as O(t^3/n^2).
///     Randomized: Randomized first-order Trotter with specified random seed.
///
/// Examples:
///     >>> from cqlib.qis import TrotterMode
///     >>> mode1 = TrotterMode.first_order()
///     >>> mode2 = TrotterMode.second_order()
///     >>> mode3 = TrotterMode.randomized(42)  # with seed 42
#[pyclass(name = "TrotterMode", module = "cqlib.qis", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyTrotterMode {
    pub(crate) inner: TrotterMode,
}

impl From<TrotterMode> for PyTrotterMode {
    fn from(inner: TrotterMode) -> Self {
        Self { inner }
    }
}

impl From<PyTrotterMode> for TrotterMode {
    fn from(value: PyTrotterMode) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyTrotterMode {
    /// Returns the first-order Trotter mode.
    ///
    /// First-order Lie-Trotter decomposition:
    /// U(t) ≈ [Π_k e^(-i c_k t/n · P_k)]^n
    ///
    /// Error scales as O(t^2/n).
    #[staticmethod]
    fn first_order() -> Self {
        Self {
            inner: TrotterMode::FirstOrder,
        }
    }

    /// Returns the second-order Trotter mode.
    ///
    /// Second-order Strang splitting (symmetric decomposition):
    /// U(t) ≈ [e^(-i c_1 t/2n · P_1) ... e^(-i c_m t/2n · P_m) ·
    ///         e^(-i c_m t/2n · P_m) ... e^(-i c_1 t/2n · P_1)]^n
    ///
    /// Error scales as O(t^3/n^2).
    #[staticmethod]
    fn second_order() -> Self {
        Self {
            inner: TrotterMode::SecondOrder,
        }
    }

    /// Returns a randomized first-order Trotter mode with the given seed.
    ///
    /// In each Trotter step, the order of Pauli terms is randomly shuffled.
    /// This can help reduce systematic errors and improve convergence.
    ///
    /// Args:
    ///     seed: The random seed for reproducibility.
    ///
    /// Examples:
    ///     >>> mode = TrotterMode.randomized(42)
    #[staticmethod]
    fn randomized(seed: u64) -> Self {
        Self {
            inner: TrotterMode::Randomized(seed),
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            TrotterMode::FirstOrder => "TrotterMode.FirstOrder".to_string(),
            TrotterMode::SecondOrder => "TrotterMode.SecondOrder".to_string(),
            TrotterMode::Randomized(seed) => format!("TrotterMode.Randomized(seed={})", seed),
        }
    }

    fn __str__(&self) -> String {
        match self.inner {
            TrotterMode::FirstOrder => "first-order".to_string(),
            TrotterMode::SecondOrder => "second-order".to_string(),
            TrotterMode::Randomized(seed) => format!("randomized (seed={})", seed),
        }
    }

    fn __eq__(&self, other: &PyTrotterMode) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }
}
