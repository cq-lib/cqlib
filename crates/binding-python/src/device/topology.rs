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

//! Python bindings for quantum device topology.
//!
//! This module provides [`PyTopology`], a Python wrapper around the core
//! [`Topology`] type that represents the coupling/connectivity structure
//! of a quantum device.
//!
//! # Directed Couplings
//!
//! **Important:** Couplings in this topology are **directed**
//! (asymmetric). A coupling from qubit `a` to qubit `b` does **not**
//! imply a coupling from `b` to `a`. This models real quantum hardware
//! where two-qubit gates only work in specific control-target directions.
//!
//! Use [`PyTopology::supports_directed_coupling`] to check for a coupling
//! in a specific direction, or [`PyTopology::supports_coupling_either_direction`]
//! to check both directions.
//!
//! # Example
//!
//! ```python
//! from cqlib.device import Topology
//!
//! # Create a topology with directed couplings
//! topology = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CX")])
//!
//! # Check directed connectivity
//! topology.supports_directed_coupling(0, 1)  # True: 0 -> 1
//! topology.supports_directed_coupling(1, 0)  # False: 1 -> 0 not present
//!
//! # Get successors (qubits reachable via outgoing couplings)
//! topology.successors(0)  # [PhysicalQubit(1)]
//!
//! # Get predecessors (qubits with incoming couplings)
//! topology.predecessors(1)  # [PhysicalQubit(0)]
//! ```

use crate::device::qubit::{PyPhysicalQubit, PyPhysicalQubitLike, PyPhysicalQubitList};
use cqlib_core::device::{PhysicalQubit, topology::Topology};
use pyo3::exceptions::PyValueError;
use pyo3::{Bound, PyAny, PyResult, pyclass, pymethods};

/// A directed coupling graph representing quantum hardware connectivity.
///
/// Each node represents a physical qubit, and each directed edge
/// represents a coupling between qubits (e.g., for two-qubit gates like
/// CNOT).
///
/// # Directed vs Undirected
///
/// This topology is **directed**. A coupling `a -> b` does not imply
/// `b -> a`. This models hardware where gates only work in specific
/// directions.
///
/// # Python Example
///
/// ```python
/// from cqlib.device import Topology
///
/// # Create topology with explicit qubit list and directed couplings
/// topology = Topology(
///     qubits=[0, 1, 2, 3],
///     couplings=[(0, 1, "CX"), (1, 2, "CX"), (2, 3, "CX")]
/// )
///
/// # Query properties
/// topology.num_qubits       # 4
/// topology.num_couplings    # 3
/// topology.qubits           # [Qubit(0), Qubit(1), Qubit(2), Qubit(3)]
///
/// # Check connectivity
/// topology.supports_directed_coupling(0, 1)  # True (0 -> 1)
/// topology.supports_directed_coupling(1, 0)  # False (no 1 -> 0)
/// ```
#[pyclass(name = "Topology", module = "cqlib.device", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyTopology {
    /// The underlying core topology.
    pub(crate) inner: Topology,
}

#[pymethods]
impl PyTopology {
    /// Creates a new topology with specified qubits and directed
    /// couplings.
    ///
    /// Args:
    ///     qubits: List of qubit identifiers (integers or Qubit objects).
    ///         Duplicate qubits are detected and rejected.
    ///     couplings: List of directed couplings as tuples
    ///         `(control, target, name)`. Each coupling is directed from
    ///         `control` to `target`. Coupling names are informational
    ///         labels (e.g., "CX", "CZ").
    ///
    /// Returns:
    ///     Topology: A new topology instance.
    ///
    /// Example:
    /// ```python
    /// from cqlib.device import Topology
    ///
    /// # Directed line: 0 -> 1 -> 2
    /// topology = Topology(
    ///     qubits=[0, 1, 2],
    ///     couplings=[(0, 1, "CX"), (1, 2, "CX")]
    /// )
    /// ```
    #[new]
    #[pyo3(signature = (qubits, couplings))]
    fn new(
        qubits: PyPhysicalQubitList,
        couplings: Vec<(PyPhysicalQubitLike, PyPhysicalQubitLike, String)>,
    ) -> PyResult<Self> {
        let qubits = Vec::<PhysicalQubit>::from(qubits);
        let couplings: Vec<(PhysicalQubit, PhysicalQubit, String)> = couplings
            .into_iter()
            .map(|(c, t, s)| (c.into(), t.into(), s))
            .collect();
        let inner =
            Topology::new(qubits, couplings).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Creates a directed line topology in the supplied qubit order.
    ///
    /// Constructs a linear chain: `qubits[0] -> qubits[1] -> qubits[2]
    /// -> ...`
    ///
    /// Args:
    ///     qubits: List of qubit identifiers in line order.
    ///
    /// Returns:
    ///     Topology: A new line topology with `len(qubits) - 1`
    ///     couplings.
    ///
    /// Example:
    /// ```python
    /// from cqlib.device import Topology
    ///
    /// # Creates couplings: 0 -> 1, 1 -> 2, 2 -> 3
    /// topology = Topology.line([0, 1, 2, 3])
    /// ```
    #[staticmethod]
    fn line(qubits: PyPhysicalQubitList) -> PyResult<Self> {
        let qubits = Vec::<PhysicalQubit>::from(qubits);

        let inner = Topology::line(qubits).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Returns the number of physical qubits in the topology.
    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    /// Returns the number of directed coupling edges.
    #[getter]
    fn num_couplings(&self) -> usize {
        self.inner.num_couplings()
    }

    /// Returns all physical qubits in the topology.
    #[getter]
    fn qubits(&self) -> Vec<PyPhysicalQubit> {
        self.inner.qubits().map(Into::into).collect()
    }

    /// Adds physical qubits to the topology.
    ///
    /// Args:
    ///     qubits: List of qubit identifiers to add.
    ///
    /// Raises:
    ///     ValueError: If any qubit already exists in the topology.
    fn add_qubits(&mut self, qubits: PyPhysicalQubitList) -> PyResult<()> {
        let qubits = Vec::<PhysicalQubit>::from(qubits);
        self.inner
            .add_qubits(qubits)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Adds directed couplings to the topology.
    ///
    /// Each coupling is directed from the first qubit to the second.
    ///
    /// Args:
    ///     couplings: List of tuples `(control, target, name)` where:
    ///         - `control`: Source qubit of the directed coupling
    ///         - `target`: Destination qubit of the directed coupling
    ///         - `name`: String identifier for the coupling
    ///
    /// Raises:
    ///     ValueError: If either endpoint qubit does not exist in the
    ///     topology, the coupling already exists, or a self-coupling
    ///     is requested.
    fn add_couplings(
        &mut self,
        couplings: Vec<(PyPhysicalQubitLike, PyPhysicalQubitLike, String)>,
    ) -> PyResult<()> {
        let couplings: Vec<(PhysicalQubit, PhysicalQubit, String)> = couplings
            .into_iter()
            .map(|(c, t, s)| (c.into(), t.into(), s))
            .collect();
        self.inner
            .add_couplings(couplings)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Removes physical qubits and all their incident couplings from the
    /// topology.
    ///
    /// When a qubit is removed, all directed couplings where it is either
    /// the source or target are also removed.
    ///
    /// Args:
    ///     qubits: List of qubit identifiers to remove.
    ///
    /// Raises:
    ///     ValueError: If any qubit does not exist in the topology.
    fn remove_qubits(&mut self, qubits: PyPhysicalQubitList) -> PyResult<()> {
        let qubits = Vec::<PhysicalQubit>::from(qubits);
        self.inner
            .remove_qubits(qubits)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Removes directed couplings from the topology.
    ///
    /// Only removes the specific directed coupling. The reverse coupling
    /// (if present) is not affected.
    ///
    /// Args:
    ///     couplings: List of `(control, target)` tuples specifying
    ///         directed couplings to remove.
    ///
    /// Raises:
    ///     ValueError: If a coupling does not exist or endpoint qubits
    ///     are missing.
    fn remove_couplings(
        &mut self,
        couplings: Vec<(PyPhysicalQubitLike, PyPhysicalQubitLike)>,
    ) -> PyResult<()> {
        let couplings: Vec<(PhysicalQubit, PhysicalQubit)> = couplings
            .into_iter()
            .map(|(c, t)| (c.into(), t.into()))
            .collect();
        self.inner
            .remove_couplings(couplings)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Checks whether the directed coupling `control -> target` exists.
    ///
    /// Returns `True` if there is a directed coupling edge from qubit
    /// `control` to qubit `target`. This does **not** check the reverse
    /// direction.
    ///
    /// Args:
    ///     control: Source qubit (control).
    ///     target: Destination qubit (target).
    ///
    /// Returns:
    ///     bool: `True` if `control -> target` coupling exists.
    ///
    /// Example:
    ///     ```python
    ///     topology = Topology([0, 1], [(0, 1, "CX")])
    ///     topology.supports_directed_coupling(0, 1)  # True: 0 -> 1
    ///     topology.supports_directed_coupling(1, 0)  # False: no 1 -> 0
    ///     ```
    fn supports_directed_coupling(
        &self,
        control: PyPhysicalQubitLike,
        target: PyPhysicalQubitLike,
    ) -> bool {
        self.inner
            .supports_directed_coupling(control.into(), target.into())
    }

    /// Checks whether a coupling exists in either direction.
    ///
    /// Returns `True` if there is a directed coupling `a -> b` or
    /// `b -> a` (or both).
    ///
    /// Args:
    ///     a: First qubit.
    ///     b: Second qubit.
    fn supports_coupling_either_direction(
        &self,
        a: PyPhysicalQubitLike,
        b: PyPhysicalQubitLike,
    ) -> bool {
        self.inner
            .supports_coupling_either_direction(a.into(), b.into())
    }

    /// Returns qubits reachable via outgoing couplings from `qubit`.
    ///
    /// All qubits `v` such that a directed coupling `qubit -> v` exists.
    ///
    /// Args:
    ///     qubit: The source qubit.
    ///
    /// Returns:
    ///     List[PhysicalQubit]: List of successors.
    ///
    /// Example:
    ///     ```python
    ///     topology = Topology([0, 1, 2], [(0, 1, "CX"), (0, 2, "CX")])
    ///     topology.successors(0)  # [PhysicalQubit(1), PhysicalQubit(2)]
    ///     ```
    fn successors(&self, qubit: PyPhysicalQubitLike) -> Vec<PyPhysicalQubit> {
        self.inner
            .successors(qubit.into())
            .map(Into::into)
            .collect()
    }

    /// Returns qubits with incoming couplings to `qubit`.
    ///
    /// All qubits `u` such that a directed coupling `u -> qubit` exists.
    ///
    /// Args:
    ///     qubit: The target qubit.
    ///
    /// Returns:
    ///     List[PhysicalQubit]: List of predecessors.
    fn predecessors(&self, qubit: PyPhysicalQubitLike) -> Vec<PyPhysicalQubit> {
        self.inner
            .predecessors(qubit.into())
            .map(Into::into)
            .collect()
    }

    /// Returns all qubits coupled to `qubit` in either direction.
    ///
    /// Bidirectional couplings are returned once (deduplicated).
    ///
    /// Args:
    ///     qubit: The qubit to query.
    ///
    /// Returns:
    ///     List[PhysicalQubit]: List of coupled neighbors.
    fn neighbors_undirected(&self, qubit: PyPhysicalQubitLike) -> Vec<PyPhysicalQubit> {
        self.inner
            .neighbors_undirected(qubit.into())
            .map(Into::into)
            .collect()
    }

    /// Returns all unique coupling edges without direction.
    ///
    /// Each returned pair is ordered by physical qubit ID, and
    /// bidirectional couplings collapse to one pair.
    ///
    /// Returns:
    ///     List[Tuple[PhysicalQubit, PhysicalQubit]]: List of undirected
    ///     edge pairs.
    fn undirected_edges(&self) -> Vec<(PyPhysicalQubit, PyPhysicalQubit)> {
        self.inner
            .undirected_edges()
            .map(|(a, b)| (a.into(), b.into()))
            .collect()
    }

    /// Returns the name of the directed coupling, if it exists.
    ///
    /// Args:
    ///     control: Source qubit.
    ///     target: Target qubit.
    ///
    /// Returns:
    ///     Optional[str]: The coupling name if `control -> target`
    ///     exists, `None` otherwise.
    fn get_coupling_name(
        &self,
        control: PyPhysicalQubitLike,
        target: PyPhysicalQubitLike,
    ) -> Option<String> {
        self.inner.get_coupling_name(control.into(), target.into())
    }

    /// Checks if a qubit exists in the topology.
    ///
    /// Args:
    ///     qubit: Qubit to check.
    ///
    /// Returns:
    ///     bool: `True` if the qubit exists in the topology.
    fn contains_qubit(&self, qubit: PyPhysicalQubitLike) -> bool {
        self.inner.contains_qubit(&qubit.into())
    }

    /// Returns the number of outgoing couplings from a qubit.
    ///
    /// Returns the count of directed couplings where this qubit is the
    /// source.
    ///
    /// Args:
    ///     qubit: The qubit to query.
    ///
    /// Returns:
    ///     int: Number of outgoing couplings. Returns 0 if the qubit
    ///     does not exist.
    fn out_degree(&self, qubit: PyPhysicalQubitLike) -> usize {
        self.inner.out_degree(&qubit.into())
    }

    /// Returns the number of incoming couplings to a qubit.
    ///
    /// Returns the count of directed couplings where this qubit is the
    /// target.
    ///
    /// Args:
    ///     qubit: The qubit to query.
    ///
    /// Returns:
    ///     int: Number of incoming couplings. Returns 0 if the qubit
    ///     does not exist.
    fn in_degree(&self, qubit: PyPhysicalQubitLike) -> usize {
        self.inner.in_degree(&qubit.into())
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    /// Returns a string representation for debugging.
    ///
    /// Example: `Topology(num_qubits=4, num_couplings=3)`
    fn __repr__(&self) -> String {
        format!(
            "Topology(num_qubits={}, num_couplings={})",
            self.inner.num_qubits(),
            self.inner.num_couplings()
        )
    }
}
