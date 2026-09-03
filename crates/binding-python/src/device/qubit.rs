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

//! Python bindings for strongly typed qubit identifiers used in
//! device-facing APIs.
//!
//! Circuit operations use [`Qubit`] as logical wire identifiers.
//! Device-facing code must distinguish those logical identifiers
//! from physical hardware positions. [`PyLogicalQubit`] and
//! [`PyPhysicalQubit`] provide that distinction in Python without
//! changing the compact representation.
//!
//! # Example
//!
//! ```python
//! from cqlib.device import LogicalQubit, PhysicalQubit
//!
//! lq = LogicalQubit(0)     # logical qubit for circuit wire 0
//! pq = PhysicalQubit(100)  # physical qubit at hardware position 100
//!
//! assert lq.id == 0
//! assert pq.id == 100
//! ```

use crate::circuit::PyQubit;
use crate::utils::hash_value;
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{LogicalQubit, PhysicalQubit};
use pyo3::{Borrowed, Bound, FromPyObject, PyAny, PyErr, pyclass, pymethods};

/// Python wrapper for [`LogicalQubit`].
///
/// A logical qubit identifies a circuit wire. It is distinct from a
/// [`PyPhysicalQubit`], even when both carry the same numeric identifier.
/// The compiler resource manager is responsible for allocating logical
/// qubits; the layout maps them to physical qubits.
///
/// # Display
///
/// Logical qubits are displayed as `L{id}`, e.g. `L0`, `L1`.
///
/// # Python Example
///
/// ```python
/// from cqlib.device import LogicalQubit
///
/// lq = LogicalQubit(0)
/// print(lq)        # L0
/// print(lq.id)     # 0
/// print(lq.qubit)  # Qubit(0)
/// ```
#[pyclass(name = "LogicalQubit", module = "cqlib.device", skip_from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyLogicalQubit {
    pub(crate) inner: LogicalQubit,
}

impl From<LogicalQubit> for PyLogicalQubit {
    fn from(inner: LogicalQubit) -> Self {
        Self { inner }
    }
}

impl From<PyLogicalQubit> for LogicalQubit {
    fn from(value: PyLogicalQubit) -> Self {
        value.inner
    }
}

impl From<PyLogicalQubit> for Qubit {
    fn from(value: PyLogicalQubit) -> Self {
        value.inner.qubit()
    }
}

/// Python input accepted where a logical device qubit is required.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyLogicalQubitLike {
    inner: LogicalQubit,
}

impl<'py> FromPyObject<'_, 'py> for PyLogicalQubitLike {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(value) = obj.cast::<PyLogicalQubit>() {
            return Ok(Self {
                inner: value.borrow().inner,
            });
        }
        if let Ok(value) = obj.cast::<PyQubit>() {
            return Ok(Self {
                inner: LogicalQubit::from_qubit(value.borrow().inner),
            });
        }
        obj.extract::<u32>().map(|id| Self {
            inner: LogicalQubit::new(id),
        })
    }
}

impl From<PyLogicalQubitLike> for LogicalQubit {
    fn from(value: PyLogicalQubitLike) -> Self {
        value.inner
    }
}

/// Python sequence accepted for logical device qubits.
pub struct PyLogicalQubitList {
    inner: Vec<LogicalQubit>,
}

impl<'py> FromPyObject<'_, 'py> for PyLogicalQubitList {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        obj.extract::<Vec<PyLogicalQubitLike>>().map(|values| Self {
            inner: values.into_iter().map(Into::into).collect(),
        })
    }
}

impl From<PyLogicalQubitList> for Vec<LogicalQubit> {
    fn from(value: PyLogicalQubitList) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyLogicalQubit {
    /// Creates a logical qubit identifier from its numeric ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The numeric qubit identifier (non-negative integer).
    ///
    /// # Returns
    ///
    /// A new `LogicalQubit` instance.
    #[new]
    fn new(id: u32) -> Self {
        Self {
            inner: LogicalQubit::new(id),
        }
    }

    /// Returns the underlying circuit [`Qubit`].
    ///
    /// The returned qubit is the same as the circuit qubit with the
    /// same numeric identifier.
    #[getter]
    fn qubit(&self) -> PyQubit {
        PyQubit {
            inner: self.inner.qubit(),
        }
    }

    /// Returns the numeric qubit identifier.
    #[getter]
    fn id(&self) -> u32 {
        self.inner.id()
    }

    /// Returns the qubit identifier as an index (for parity with
    /// `Qubit.index`).
    #[getter]
    fn index(&self) -> usize {
        self.inner.id() as usize
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        other
            .cast::<PyLogicalQubit>()
            .is_ok_and(|other| self.inner == other.borrow().inner)
    }

    fn __lt__(&self, other: &Bound<'_, PyAny>) -> bool {
        other
            .cast::<PyLogicalQubit>()
            .is_ok_and(|other| self.inner < other.borrow().inner)
    }

    fn __le__(&self, other: &Bound<'_, PyAny>) -> bool {
        other
            .cast::<PyLogicalQubit>()
            .is_ok_and(|other| self.inner <= other.borrow().inner)
    }

    fn __gt__(&self, other: &Bound<'_, PyAny>) -> bool {
        other
            .cast::<PyLogicalQubit>()
            .is_ok_and(|other| self.inner > other.borrow().inner)
    }

    fn __ge__(&self, other: &Bound<'_, PyAny>) -> bool {
        other
            .cast::<PyLogicalQubit>()
            .is_ok_and(|other| self.inner >= other.borrow().inner)
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    /// Returns a string representation for debugging.
    ///
    /// Example: `LogicalQubit(0)`
    fn __repr__(&self) -> String {
        format!("LogicalQubit({})", self.inner.id())
    }

    /// Returns a human-readable string representation.
    ///
    /// Example: `L0`
    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }
}

/// Python wrapper for [`PhysicalQubit`].
///
/// A physical qubit represents a hardware position on a quantum device.
/// It is distinct from a [`PyLogicalQubit`], even when both carry the
/// same numeric identifier. Layout code is responsible for mapping
/// logical qubits to physical qubits.
///
/// # Display
///
/// Physical qubits are displayed as `P{id}`, e.g. `P100`, `P101`.
///
/// # Python Example
///
/// ```python
/// from cqlib.device import PhysicalQubit
///
/// pq = PhysicalQubit(100)
/// print(pq)        # P100
/// print(pq.id)     # 100
/// print(pq.qubit)  # Qubit(100)
/// ```
#[pyclass(name = "PhysicalQubit", module = "cqlib.device", skip_from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyPhysicalQubit {
    pub(crate) inner: PhysicalQubit,
}

impl From<PhysicalQubit> for PyPhysicalQubit {
    fn from(inner: PhysicalQubit) -> Self {
        Self { inner }
    }
}

impl From<PyPhysicalQubit> for PhysicalQubit {
    fn from(value: PyPhysicalQubit) -> Self {
        value.inner
    }
}

impl From<PyPhysicalQubit> for Qubit {
    fn from(value: PyPhysicalQubit) -> Self {
        value.inner.qubit()
    }
}

/// Python input accepted where a physical device qubit is required.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyPhysicalQubitLike {
    inner: PhysicalQubit,
}

impl<'py> FromPyObject<'_, 'py> for PyPhysicalQubitLike {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(value) = obj.cast::<PyPhysicalQubit>() {
            return Ok(Self {
                inner: value.borrow().inner,
            });
        }
        if let Ok(value) = obj.cast::<PyQubit>() {
            return Ok(Self {
                inner: PhysicalQubit::from_qubit(value.borrow().inner),
            });
        }
        obj.extract::<u32>().map(|id| Self {
            inner: PhysicalQubit::new(id),
        })
    }
}

impl From<PyPhysicalQubitLike> for PhysicalQubit {
    fn from(value: PyPhysicalQubitLike) -> Self {
        value.inner
    }
}

impl From<PyPhysicalQubitLike> for Qubit {
    fn from(value: PyPhysicalQubitLike) -> Self {
        PhysicalQubit::from(value).qubit()
    }
}

/// Python sequence accepted for physical device qubits.
pub struct PyPhysicalQubitList {
    inner: Vec<PhysicalQubit>,
}

impl<'py> FromPyObject<'_, 'py> for PyPhysicalQubitList {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        obj.extract::<Vec<PyPhysicalQubitLike>>()
            .map(|values| Self {
                inner: values.into_iter().map(Into::into).collect(),
            })
    }
}

impl From<PyPhysicalQubitList> for Vec<PhysicalQubit> {
    fn from(value: PyPhysicalQubitList) -> Self {
        value.inner
    }
}

impl From<PyPhysicalQubitList> for Vec<Qubit> {
    fn from(value: PyPhysicalQubitList) -> Self {
        Vec::<PhysicalQubit>::from(value)
            .into_iter()
            .map(PhysicalQubit::qubit)
            .collect()
    }
}

#[pymethods]
impl PyPhysicalQubit {
    /// Creates a physical qubit identifier from its numeric ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The numeric hardware-qubit identifier (non-negative integer).
    ///
    /// # Returns
    ///
    /// A new `PhysicalQubit` instance.
    #[new]
    fn new(id: u32) -> Self {
        Self {
            inner: PhysicalQubit::new(id),
        }
    }

    /// Returns the underlying circuit [`Qubit`].
    ///
    /// The returned qubit is the same as the circuit qubit with the
    /// same numeric identifier.
    #[getter]
    fn qubit(&self) -> PyQubit {
        PyQubit {
            inner: self.inner.qubit(),
        }
    }

    /// Returns the numeric hardware-qubit identifier.
    #[getter]
    fn id(&self) -> u32 {
        self.inner.id()
    }

    /// Returns the hardware-qubit identifier as an index (for parity
    /// with `Qubit.index`).
    #[getter]
    fn index(&self) -> usize {
        self.inner.id() as usize
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        other
            .cast::<PyPhysicalQubit>()
            .is_ok_and(|other| self.inner == other.borrow().inner)
    }

    fn __lt__(&self, other: &Bound<'_, PyAny>) -> bool {
        other
            .cast::<PyPhysicalQubit>()
            .is_ok_and(|other| self.inner < other.borrow().inner)
    }

    fn __le__(&self, other: &Bound<'_, PyAny>) -> bool {
        other
            .cast::<PyPhysicalQubit>()
            .is_ok_and(|other| self.inner <= other.borrow().inner)
    }

    fn __gt__(&self, other: &Bound<'_, PyAny>) -> bool {
        other
            .cast::<PyPhysicalQubit>()
            .is_ok_and(|other| self.inner > other.borrow().inner)
    }

    fn __ge__(&self, other: &Bound<'_, PyAny>) -> bool {
        other
            .cast::<PyPhysicalQubit>()
            .is_ok_and(|other| self.inner >= other.borrow().inner)
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    /// Returns a string representation for debugging.
    ///
    /// Example: `PhysicalQubit(100)`
    fn __repr__(&self) -> String {
        format!("PhysicalQubit({})", self.inner.id())
    }

    /// Returns a human-readable string representation.
    ///
    /// Example: `P100`
    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }
}
