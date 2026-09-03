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

//! Python bindings for logical-to-physical qubit layout management.
//!
//! This module provides [`PyLayout`], a Python wrapper for mapping circuit
//! logical qubits to physical qubits on a quantum device. Layouts are
//! essential for circuit routing algorithms like SABRE that track how
//! logical qubits move across hardware.
//!
//! # Concepts
//!
//! - **Logical qubits**: Virtual qubits used in the quantum circuit
//!   (LogicalQubit(0), LogicalQubit(1), ...)
//! - **Physical qubits**: Actual hardware qubits on the device
//!   (PhysicalQubit(100), PhysicalQubit(101), ...)
//! - **Vacant physical qubits**: Physical qubits not currently carrying a
//!   logical qubit — available for routing or binding additional
//!   logical qubits.
//!
//! # Example
//!
//! ```python
//! from cqlib.device import Layout
//!
//! # Create a layout with 2 logical qubits mapped to 4 physical qubits
//! layout = Layout(
//!     logical=[0, 1],
//!     physical=[100, 101, 102, 103]
//! )
//!
//! # Query mappings
//! physical = layout.get_physical(0)  # Physical qubit for logical 0
//! logical = layout.get_logical(101)  # Logical qubit on physical 101
//!
//! # Perform SWAP operation (used in routing)
//! layout.swap_physical(100, 101)
//! ```
//!
//! # Notes
//!
//! - Layout does **not** allocate auxiliary qubits. Algorithm auxiliary
//!   qubits are logical circuit resources and must be managed by the
//!   compiler resource manager before [`PyLayout::bind`]ing them to
//!   physical qubits.
//! - Layout owns the set of physical qubits and tracks which are vacant.

use crate::device::qubit::{
    PyLogicalQubit, PyLogicalQubitLike, PyLogicalQubitList, PyPhysicalQubit, PyPhysicalQubitLike,
    PyPhysicalQubitList,
};
use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};
use pyo3::exceptions::PyValueError;
use pyo3::{Bound, PyAny, PyResult, pyclass, pymethods};
use std::collections::HashMap;

/// Maps circuit logical qubits to physical qubits on a quantum device.
///
/// A layout owns the set of physical qubits available to a placement or
/// routing step. Every logical qubit present in the layout has exactly one
/// physical mapping. A physical qubit may be vacant.
///
/// # Vacant Physical Qubits
///
/// Unlike ancilla qubits (which are logical circuit resources), vacant
/// physical qubits are simply unused hardware positions. They become
/// available when a logical qubit is explicitly [`bind`](PyLayout::bind)ed
/// to them.
///
/// # Python Example
///
/// ```python
/// from cqlib.device import Layout
///
/// # Create layout with initial mapping
/// init_map = {0: 100, 1: 101}  # logical 0 -> physical 100
/// layout = Layout(
///     logical=[0, 1],
///     physical=[100, 101, 102],
///     init_map=init_map
/// )
///
/// # Check vacant count
/// print(layout.num_vacant_physical)  # 1 (physical qubit 102 is vacant)
///
/// # Get all mappings
/// print(layout.l2p_map)  # {LogicalQubit(0): PhysicalQubit(100), ...}
/// ```
#[pyclass(name = "Layout", module = "cqlib.device", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyLayout {
    pub(crate) inner: Layout,
}

impl From<Layout> for PyLayout {
    fn from(inner: Layout) -> Self {
        Self { inner }
    }
}

impl From<PyLayout> for Layout {
    fn from(value: PyLayout) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyLayout {
    /// Creates a new layout mapping logical qubits to physical qubits.
    ///
    /// Entries in `init_map` are applied first. Remaining logical qubits
    /// are mapped to remaining physical qubits in the order supplied by
    /// `logical` and `physical`. Extra physical qubits remain vacant.
    ///
    /// # Arguments
    ///
    /// * `logical`: List of logical qubit identifiers (integers or Qubit
    ///   objects).
    /// * `physical`: List of physical qubit identifiers available on the
    ///   device.
    /// * `init_map`: Optional initial mapping from logical to physical
    ///   qubits. If not provided, logical qubits are mapped sequentially
    ///   to physical qubits.
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if:
    /// - The number of logical qubits exceeds physical qubits
    /// - Duplicate logical or physical qubits are in the lists
    /// - `init_map` references a qubit not in the lists
    /// - `init_map` maps multiple logical qubits to the same physical qubit
    ///
    /// # Example
    ///
    /// ```python
    /// from cqlib.device import Layout
    ///
    /// # Automatic sequential mapping
    /// layout = Layout(logical=[0, 1], physical=[100, 101, 102])
    ///
    /// # Custom initial mapping
    /// layout = Layout(
    ///     logical=[0, 1],
    ///     physical=[100, 101, 102],
    ///     init_map={0: 101, 1: 100}
    /// )
    /// ```
    #[new]
    #[pyo3(signature = (logical, physical, init_map=None))]
    fn new(
        logical: PyLogicalQubitList,
        physical: PyPhysicalQubitList,
        init_map: Option<HashMap<PyLogicalQubitLike, PyPhysicalQubitLike>>,
    ) -> PyResult<Self> {
        let logical = Vec::<LogicalQubit>::from(logical);
        let physical = Vec::<PhysicalQubit>::from(physical);
        let init_map = init_map.map(|m| m.into_iter().map(|(l, p)| (l.into(), p.into())).collect());

        let inner = Layout::new(logical, physical, init_map)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Creates a layout from `(logical, physical)` qubit ID pairs.
    ///
    /// Logical qubits are exactly the logical IDs that appear in
    /// `pairs`. Physical qubits are `0..physical_count`; any physical
    /// qubit not referenced by a pair remains vacant.
    ///
    /// # Arguments
    ///
    /// * `pairs`: List of `(logical_id, physical_id)` pairs.
    /// * `physical_count`: Total number of physical qubits (0-indexed).
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if a logical ID appears more than once, a
    /// physical ID appears more than once, or any physical ID is outside
    /// `0..physical_count`.
    ///
    /// # Example
    ///
    /// ```python
    /// from cqlib.device import Layout
    ///
    /// layout = Layout.from_pairs([(0, 2), (1, 0)], physical_count=4)
    /// # Logical 0 maps to physical 2, logical 1 maps to physical 0
    /// # Physical qubits 1 and 3 are vacant
    /// ```
    #[staticmethod]
    fn from_pairs(pairs: Vec<(u32, u32)>, physical_count: u32) -> PyResult<Self> {
        let inner = Layout::from_pairs(&pairs, physical_count)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Returns the number of mapped logical qubits.
    #[getter]
    fn num_logical(&self) -> usize {
        self.inner.num_logical()
    }

    /// Returns the number of physical qubits available to the layout.
    #[getter]
    fn num_physical(&self) -> usize {
        self.inner.num_physical()
    }

    /// Returns the number of physical qubits not currently carrying a
    /// logical qubit.
    #[getter]
    fn num_vacant_physical(&self) -> usize {
        self.inner.num_vacant_physical()
    }

    /// Returns the physical qubit mapped to a logical qubit.
    ///
    /// # Arguments
    ///
    /// * `logical_id`: The logical qubit identifier.
    ///
    /// # Returns
    ///
    /// The mapped physical qubit, or `None` if the logical qubit is not
    /// bound.
    fn get_physical(&self, logical_id: PyLogicalQubitLike) -> Option<PyPhysicalQubit> {
        self.inner.get_physical(logical_id.into()).map(Into::into)
    }

    /// Returns the logical qubit carried by a physical qubit.
    ///
    /// # Arguments
    ///
    /// * `physical_id`: The physical qubit identifier.
    ///
    /// # Returns
    ///
    /// The logical qubit mapped to this physical qubit, or `None` if the
    /// physical qubit is vacant.
    fn get_logical(&self, physical_id: PyPhysicalQubitLike) -> Option<PyLogicalQubit> {
        self.inner.get_logical(physical_id.into()).map(Into::into)
    }

    /// Returns all mapped logical qubits.
    #[getter]
    fn logical_qubits(&self) -> Vec<PyLogicalQubit> {
        self.inner.logical_qubits().map(Into::into).collect()
    }

    /// Returns all physical qubits available to the layout.
    #[getter]
    fn physical_qubits(&self) -> Vec<PyPhysicalQubit> {
        self.inner.physical_qubits().map(Into::into).collect()
    }

    /// Returns all vacant physical qubits.
    ///
    /// Vacant physical qubits are physical positions not currently
    /// carrying a logical qubit.
    #[getter]
    fn vacant_physical_qubits(&self) -> Vec<PyPhysicalQubit> {
        self.inner
            .vacant_physical_qubits()
            .map(Into::into)
            .collect()
    }

    /// Returns whether a physical qubit belongs to the layout and is
    /// vacant.
    ///
    /// # Arguments
    ///
    /// * `physical_id`: The physical qubit to check.
    fn is_physical_vacant(&self, physical_id: PyPhysicalQubitLike) -> bool {
        self.inner.is_physical_vacant(physical_id.into())
    }

    /// Returns the logical-to-physical qubit mapping.
    ///
    /// Maps each logical qubit to its assigned physical qubit.
    #[getter]
    fn l2p_map(&self) -> HashMap<PyLogicalQubit, PyPhysicalQubit> {
        self.inner
            .l2p_map()
            .iter()
            .map(|(l, p)| ((*l).into(), (*p).into()))
            .collect()
    }

    /// Returns the physical-to-logical qubit mapping.
    ///
    /// Maps each physical qubit to its assigned logical qubit (if any).
    /// Vacant physical qubits are not included.
    #[getter]
    fn p2l_map(&self) -> HashMap<PyPhysicalQubit, PyLogicalQubit> {
        self.inner
            .p2l_map()
            .iter()
            .map(|(p, l)| ((*p).into(), (*l).into()))
            .collect()
    }

    /// Binds an unmapped logical qubit to a vacant physical qubit.
    ///
    /// This operation may introduce a new logical qubit to the layout.
    /// The caller must ensure the logical qubit is registered with the
    /// compiler resource manager when required.
    ///
    /// # Arguments
    ///
    /// * `logical_id`: The logical qubit to bind.
    /// * `physical_id`: The vacant physical qubit to bind it to.
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if the physical qubit does not belong to the
    /// layout, or if either qubit already participates in a mapping.
    fn bind(
        &mut self,
        logical_id: PyLogicalQubitLike,
        physical_id: PyPhysicalQubitLike,
    ) -> PyResult<()> {
        self.inner
            .bind(logical_id.into(), physical_id.into())
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Removes the mapping for a logical qubit and returns the released
    /// physical qubit.
    ///
    /// # Arguments
    ///
    /// * `logical_id`: The logical qubit to unbind.
    ///
    /// # Returns
    ///
    /// The physical qubit that was released.
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if the logical qubit is not bound.
    fn unbind(&mut self, logical_id: PyLogicalQubitLike) -> PyResult<PyPhysicalQubit> {
        self.inner
            .unbind(logical_id.into())
            .map(Into::into)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Swaps the logical qubits carried by two physical qubits.
    ///
    /// This is the core operation used by routing algorithms (e.g., SABRE)
    /// when inserting SWAP gates. After a SWAP gate is applied on the
    /// hardware, the logical qubits on those physical positions are
    /// exchanged.
    ///
    /// Either physical qubit may be vacant. Swapping an occupied qubit
    /// with a vacant qubit moves the logical qubit to the vacant position.
    ///
    /// # Arguments
    ///
    /// * `phys_a`: First physical qubit.
    /// * `phys_b`: Second physical qubit.
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if either physical qubit does not belong to
    /// the layout.
    ///
    /// # Example
    ///
    /// ```python
    /// from cqlib.device import Layout
    ///
    /// layout = Layout(logical=[0], physical=[100, 101])
    ///
    /// # Before swap: Qubit 0 is on some physical qubit
    /// phys_before = layout.get_physical(0)
    ///
    /// # Perform SWAP on physical qubits
    /// layout.swap_physical(100, 101)
    ///
    /// # After swap, Qubit 0 is on the other physical qubit
    /// phys_after = layout.get_physical(0)
    /// assert phys_before != phys_after
    /// ```
    fn swap_physical(
        &mut self,
        phys_a: PyPhysicalQubitLike,
        phys_b: PyPhysicalQubitLike,
    ) -> PyResult<()> {
        self.inner
            .swap_physical(phys_a.into(), phys_b.into())
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    /// Returns a string representation for debugging.
    ///
    /// Example: `Layout(num_logical=2, num_vacant_physical=1,
    /// num_physical=3)`
    fn __repr__(&self) -> String {
        format!(
            "Layout(num_logical={}, num_vacant_physical={}, num_physical={})",
            self.num_logical(),
            self.num_vacant_physical(),
            self.num_physical()
        )
    }
}
