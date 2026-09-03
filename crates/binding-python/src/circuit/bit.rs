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

//! Python bindings for quantum bits.
//!
//! This module provides Python-compatible types for working with qubits in quantum circuits.
//!
//! # Types
//!
//! - [`PyQubit`]: A Python wrapper around the core [`Qubit`] type. Represents a single
//!   qubit identified by its index in a quantum register.
//!
//! - [`PyIntQubitList`]: Flexible input type for circuit constructors. Accepts:
//!   - An integer `n` → creates qubits `[0, 1, ..., n-1]`
//!   - A list of integers `[0, 2, 4]` → creates qubits with specific indices
//!   - A list of `Qubit` objects
//!
//! - [`PyIntOrQubit`]: Flexible input type for single-qubit gate methods. Accepts either
//!   an integer index or a `Qubit` object, allowing both `circuit.h(0)` and `circuit.h(Qubit(0))`.
//!
//! - [`PyIntListOrQubitList`]: Flexible input type for multi-qubit operations. Accepts
//!   either a list of integer indices or a list of `Qubit` objects.
//!
//! # Examples
//!
//! ```python
//! from cqlib import Circuit, Qubit
//!
//! # Create qubits directly
//! q0 = Qubit(0)
//! q1 = Qubit(1)
//!
//! # Initialize circuits with different formats
//! c1 = Circuit(3)                      # 3 qubits: 0, 1, 2
//! c2 = Circuit([0, 2, 4])              # 3 qubits: 0, 2, 4
//! c3 = Circuit([Qubit(0), Qubit(1)])   # 2 qubits: 0, 1
//!
//! # Gate methods accept flexible arguments
//! c1.h(0)           # integer index
//! c1.h(Qubit(0))    # Qubit object
//! ```

use crate::circuit::error::QubitError as PyQubitError;
use crate::utils::hash_value;
use cqlib_core::circuit::Qubit;
use pyo3::prelude::*;

/// A qubit in a quantum register.
///
/// In quantum computing, a qubit (quantum bit) is the basic unit of quantum information.
/// This type represents a qubit identified by its index in a quantum register.
///
/// # Python API
///
/// ```python
/// from cqlib import Qubit
///
/// # Create a qubit by its index
/// q0 = Qubit(0)
/// q1 = Qubit(1)
///
/// # Access properties
/// print(q0.index)  # 0
/// print(q0.id)     # 0 (raw identifier)
///
/// # Compare qubits
/// q0 < q1          # True
/// q0 == Qubit(0)   # True
/// ```
///
/// # Notes
///
/// - Qubits with the same index are considered equal.
/// - Qubits can be used as dictionary keys (hashable).
/// - Comparison operators (`<`, `<=`, `>`, `>=`) compare by index.
#[pyclass(name = "Qubit", module = "cqlib.circuit", from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyQubit {
    /// The underlying core `Qubit` type.
    pub inner: Qubit,
}

#[pymethods]
impl PyQubit {
    /// Creates a qubit with the given index.
    ///
    /// # Arguments
    ///
    /// * `index` - The qubit index in the quantum register (non-negative integer).
    ///
    /// # Python Example
    ///
    /// ```python
    /// from cqlib import Qubit
    ///
    /// q0 = Qubit(0)
    /// q1 = Qubit(1)
    /// ```
    #[new]
    fn new(index: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(index) = index.extract::<i128>() {
            if index < 0 {
                return Err(PyQubitError::new_err(
                    cqlib_core::circuit::QubitError::NegativeIndex(index).to_string(),
                ));
            }
            if index as u128 > u32::MAX as u128 {
                return Err(PyQubitError::new_err(
                    cqlib_core::circuit::QubitError::IndexTooLarge(index as u128).to_string(),
                ));
            }
            return Ok(Self {
                inner: Qubit::new(index as u32),
            });
        }

        if let Ok(index) = index.extract::<u128>() {
            if index > u32::MAX as u128 {
                return Err(PyQubitError::new_err(
                    cqlib_core::circuit::QubitError::IndexTooLarge(index).to_string(),
                ));
            }
            return Ok(Self {
                inner: Qubit::new(index as u32),
            });
        }

        Err(pyo3::exceptions::PyTypeError::new_err(
            "qubit index must be an integer",
        ))
    }

    /// Returns the qubit index.
    ///
    /// The index identifies the qubit's position in the quantum register.
    #[getter]
    fn index(&self) -> usize {
        self.inner.index()
    }

    /// Returns the raw identifier.
    ///
    /// This is the underlying storage value (same as `index` but as `u32`).
    #[getter]
    fn id(&self) -> u32 {
        self.inner.id()
    }

    /// Returns a string representation for debugging.
    ///
    /// Example: `Qubit(0)`
    fn __repr__(&self) -> String {
        format!("Qubit({})", self.inner.index())
    }

    /// Returns a human-readable string representation.
    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }

    /// Compares two qubits for equality.
    ///
    /// Returns `True` if both qubits have the same index.
    /// Returns `False` if `other` is not a `Qubit`.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyQubit>() {
            return Ok(false);
        }
        let other_qubit = other.extract::<PyQubit>()?;
        Ok(self.inner == other_qubit.inner)
    }

    /// Computes a hash for use in dictionaries and sets.
    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    /// Less-than comparison (by index).
    fn __lt__(&self, other: &PyQubit) -> bool {
        self.inner < other.inner
    }

    /// Less-than-or-equal comparison (by index).
    fn __le__(&self, other: &PyQubit) -> bool {
        self.inner <= other.inner
    }

    /// Greater-than comparison (by index).
    fn __gt__(&self, other: &PyQubit) -> bool {
        self.inner > other.inner
    }

    /// Greater-than-or-equal comparison (by index).
    fn __ge__(&self, other: &PyQubit) -> bool {
        self.inner >= other.inner
    }
}

impl From<Qubit> for PyQubit {
    /// Wraps a core `Qubit` into a `PyQubit`.
    fn from(inner: Qubit) -> Self {
        PyQubit { inner }
    }
}

impl From<PyQubit> for Qubit {
    /// Unwraps a `PyQubit` into the core `Qubit`.
    fn from(py_qubit: PyQubit) -> Self {
        py_qubit.inner
    }
}

/// Circuit constructor argument: accepts `int`, `List[int]`, or `List[Qubit]`.
///
/// Used by [`PyCircuit::new`] to support flexible circuit initialization.
///
/// # Variants
///
/// | Input Type | Example | Result |
/// |------------|---------|--------|
/// | `int` | `Circuit(3)` | Qubits `[0, 1, 2]` |
/// | `List[int]` | `Circuit([0, 2, 4])` | Qubits at specified indices |
/// | `List[Qubit]` | `Circuit([Qubit(0), Qubit(1)])` | Given Qubit objects |
#[derive(FromPyObject)]
pub enum PyIntQubitList {
    /// Integer: create N sequential qubits (0 to N-1).
    NumQubits(u32),
    /// List of integers: create qubits at specific indices.
    IndexList(Vec<u32>),
    /// List of Qubit objects: use the provided qubits.
    QubitList(Vec<PyQubit>),
}

impl From<PyIntQubitList> for Vec<PyQubit> {
    /// Converts to a vector of `PyQubit` objects.
    fn from(item: PyIntQubitList) -> Vec<PyQubit> {
        match item {
            PyIntQubitList::NumQubits(n) => (0..n).map(|i| PyQubit::from(Qubit::new(i))).collect(),
            PyIntQubitList::IndexList(indices) => indices
                .iter()
                .map(|&i| PyQubit::from(Qubit::new(i)))
                .collect(),
            PyIntQubitList::QubitList(qubits) => qubits,
        }
    }
}

impl From<PyIntQubitList> for Vec<Qubit> {
    /// Converts to a vector of core `Qubit` objects.
    fn from(item: PyIntQubitList) -> Vec<Qubit> {
        match item {
            PyIntQubitList::NumQubits(n) => (0..n).map(Qubit::new).collect(),
            PyIntQubitList::IndexList(indices) => indices.iter().copied().map(Qubit::new).collect(),
            PyIntQubitList::QubitList(qubits) => qubits.into_iter().map(|q| q.inner).collect(),
        }
    }
}

/// Single-qubit gate argument: accepts `int` or `Qubit`.
///
/// Enables gate methods to accept flexible qubit arguments.
///
/// # Python Usage
///
/// ```python
/// # Both forms are equivalent
/// circuit.h(0)           # integer index
/// circuit.h(Qubit(0))    # Qubit object
/// ```
#[derive(FromPyObject)]
pub enum PyIntOrQubit {
    /// Integer qubit index (e.g., `0`).
    Int(u32),
    /// Qubit object.
    Qubit(PyQubit),
}

impl From<PyIntOrQubit> for PyQubit {
    /// Converts to a `PyQubit`, creating one from the index if needed.
    fn from(item: PyIntOrQubit) -> PyQubit {
        match item {
            PyIntOrQubit::Int(i) => PyQubit::from(Qubit::new(i)),
            PyIntOrQubit::Qubit(q) => q,
        }
    }
}

impl From<PyIntOrQubit> for Qubit {
    /// Converts to a core `Qubit`, creating one from the index if needed.
    fn from(item: PyIntOrQubit) -> Qubit {
        match item {
            PyIntOrQubit::Int(i) => Qubit::new(i),
            PyIntOrQubit::Qubit(q) => q.inner,
        }
    }
}

/// Multi-qubit operation argument: accepts `List[int]` or `List[Qubit]`.
///
/// Enables gate methods to accept flexible qubit list arguments.
///
/// # Python Usage
///
/// ```python
/// # Both forms are equivalent
/// circuit.barrier([0, 1, 2])
/// circuit.barrier([Qubit(0), Qubit(1), Qubit(2)])
/// ```
#[derive(FromPyObject)]
pub enum PyIntListOrQubitList {
    /// List of integer indices (e.g., `[0, 1, 2]`).
    IntList(Vec<u32>),
    /// List of Qubit objects.
    QubitList(Vec<PyQubit>),
}

impl From<PyIntListOrQubitList> for Vec<PyQubit> {
    /// Converts to a vector of `PyQubit` objects, creating from indices if needed.
    fn from(item: PyIntListOrQubitList) -> Vec<PyQubit> {
        match item {
            PyIntListOrQubitList::IntList(indices) => indices
                .into_iter()
                .map(|i| PyQubit::from(Qubit::new(i)))
                .collect(),
            PyIntListOrQubitList::QubitList(qubits) => qubits,
        }
    }
}

impl From<PyIntListOrQubitList> for Vec<Qubit> {
    /// Converts to a vector of core `Qubit` objects, creating from indices if needed.
    fn from(item: PyIntListOrQubitList) -> Vec<Qubit> {
        match item {
            PyIntListOrQubitList::IntList(indices) => indices.into_iter().map(Qubit::new).collect(),
            PyIntListOrQubitList::QubitList(qubits) => {
                qubits.into_iter().map(|q| q.inner).collect()
            }
        }
    }
}
