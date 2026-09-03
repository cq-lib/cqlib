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

//! Python bindings for cqlib-core Pauli module.

use crate::qis::qis_error_to_py_err;
use crate::qis::state::density_matrix::PyDensityMatrix;
use crate::qis::state::statevector::PyStatevector;
use crate::utils::hash_value;
use cqlib_core::qis::Observable;
use cqlib_core::qis::pauli::{Pauli, PauliString, Phase};
use numpy::{PyArray2, ToPyArray};
use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyComplex, PyDict, PyTuple};
use std::collections::HashMap;
use std::fmt;

/// Phase factor in the Pauli group, isomorphic to Z4 (the cyclic group of order 4).
///
/// Represents powers of the imaginary unit: $i^n$ where $n \\in \\{0, 1, 2, 3\\}$.
///
/// Variants:
///     Plus (1): $i^0 = 1$
///     I (1): $i^1 = i$
///     Minus (-1): $i^2 = -1$
///     MinusI (-i): $i^3 = -i$
#[pyclass(name = "Phase", module = "cqlib.qis", skip_from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyPhase {
    pub(crate) inner: Phase,
}

impl From<Phase> for PyPhase {
    fn from(inner: Phase) -> Self {
        Self { inner }
    }
}

impl From<PyPhase> for Phase {
    fn from(value: PyPhase) -> Self {
        value.inner
    }
}

impl fmt::Display for PyPhase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[pymethods]
impl PyPhase {
    /// Creates a Phase from an integer (mod 4).
    #[new]
    fn new(val: u8) -> Self {
        Self {
            inner: Phase::from(val),
        }
    }

    /// Returns the +1 phase.
    #[staticmethod]
    fn plus() -> Self {
        Self { inner: Phase::Plus }
    }

    /// Returns the +i phase.
    #[staticmethod]
    fn i() -> Self {
        Self { inner: Phase::I }
    }

    /// Returns the -1 phase.
    #[staticmethod]
    fn minus() -> Self {
        Self {
            inner: Phase::Minus,
        }
    }

    /// Returns the -i phase.
    #[staticmethod]
    fn minus_i() -> Self {
        Self {
            inner: Phase::MinusI,
        }
    }

    /// Converts the phase to a Python complex number.
    #[allow(clippy::wrong_self_convention)]
    fn to_complex<'py>(&self, py: Python<'py>) -> Bound<'py, PyComplex> {
        let c = self.inner.to_complex();
        PyComplex::from_doubles(py, c.re, c.im)
    }

    /// Returns the phase as an integer exponent (0-3).
    #[getter]
    fn exponent(&self) -> u8 {
        self.inner as u8
    }

    /// Adds two phases (multiplication in the group).
    fn __add__(&self, other: &PyPhase) -> Self {
        Self {
            inner: self.inner + other.inner,
        }
    }

    /// Multiplies two phases (same as addition in Z4).
    fn __mul__(&self, other: &PyPhase) -> Self {
        Self {
            inner: self.inner * other.inner,
        }
    }

    fn __eq__(&self, other: &PyPhase) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!("Phase({})", self.inner as u8)
    }

    fn __str__(&self) -> String {
        match self.inner {
            Phase::Plus => "1".to_string(),
            Phase::I => "i".to_string(),
            Phase::Minus => "-1".to_string(),
            Phase::MinusI => "-i".to_string(),
        }
    }
}

/// Single-qubit Pauli operators.
///
/// The four Pauli matrices form the basis of single-qubit quantum operations.
///
/// Variants:
///     X: Pauli-X (bit-flip) operator
///     Y: Pauli-Y operator
///     Z: Pauli-Z (phase-flip) operator
///     I: Identity operator
#[pyclass(name = "Pauli", module = "cqlib.qis", from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyPauli {
    pub(crate) inner: Pauli,
}

impl From<Pauli> for PyPauli {
    fn from(inner: Pauli) -> Self {
        Self { inner }
    }
}

impl From<PyPauli> for Pauli {
    fn from(value: PyPauli) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyPauli {
    /// Returns the X Pauli operator.
    #[staticmethod]
    fn x() -> Self {
        Self { inner: Pauli::X }
    }

    /// Returns the Y Pauli operator.
    #[staticmethod]
    fn y() -> Self {
        Self { inner: Pauli::Y }
    }

    /// Returns the Z Pauli operator.
    #[staticmethod]
    fn z() -> Self {
        Self { inner: Pauli::Z }
    }

    /// Returns the Identity operator.
    #[staticmethod]
    fn i() -> Self {
        Self { inner: Pauli::I }
    }

    /// Returns the symplectic representation (x, z) as a tuple.
    ///
    /// The symplectic encoding maps Pauli operators to binary pairs:
    /// - I = (0, 0)
    /// - X = (1, 0)
    /// - Y = (1, 1)
    /// - Z = (0, 1)
    #[allow(clippy::wrong_self_convention)]
    fn to_symplectic(&self) -> (u8, u8) {
        self.inner.to_symplectic()
    }

    /// Returns the 2x2 complex matrix representation as a NumPy array.
    #[allow(clippy::wrong_self_convention)]
    fn to_matrix<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<num_complex::Complex64>> {
        let mat = self.inner.to_matrix();
        let data: Vec<Vec<num_complex::Complex64>> = vec![
            vec![mat[[0, 0]], mat[[0, 1]]],
            vec![mat[[1, 0]], mat[[1, 1]]],
        ];
        PyArray2::from_vec2(py, &data).unwrap()
    }

    /// Multiplies two Pauli operators, returning the result and phase factor.
    ///
    /// Returns:
    ///     tuple: (result_pauli, phase) where phase is a Phase object
    fn mul_with_phase(&self, other: &PyPauli) -> (Self, PyPhase) {
        let (p, ph) = self.inner.mul_with_phase(other.inner);
        (Self { inner: p }, PyPhase { inner: ph })
    }

    /// Multiplication operator (without explicit phase tracking).
    fn __mul__(&self, other: &PyPauli) -> Self {
        let (p, _) = self.inner.mul_with_phase(other.inner);
        Self { inner: p }
    }

    fn __eq__(&self, other: &PyPauli) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!("Pauli.{}", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// Multi-qubit Pauli string operator in symplectic representation.
///
/// A Pauli string is a tensor product of single-qubit Pauli operators across
/// multiple qubits: $P = \\bigotimes_{i=0}^{N-1} P_i$ where $P_i \\in \\{I, X, Y, Z\\}$.
#[pyclass(name = "PauliString", module = "cqlib.qis", from_py_object)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyPauliString {
    pub(crate) inner: PauliString,
}

impl From<PauliString> for PyPauliString {
    fn from(inner: PauliString) -> Self {
        Self { inner }
    }
}

impl From<PyPauliString> for PauliString {
    fn from(value: PyPauliString) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyPauliString {
    /// Creates a new identity Pauli string with the specified number of qubits.
    #[new]
    fn new(num_qubits: usize) -> Self {
        Self {
            inner: PauliString::new(num_qubits),
        }
    }

    /// Creates a PauliString from a string representation.
    ///
    /// The format is: `[+|-][i|j]<pauli operators>` where pauli operators are I, X, Y, or Z.
    ///
    /// Qubits are in reverse order: the first character corresponds to the highest qubit index.
    ///
    /// Args:
    ///     s: String representation like "XZI", "-iXYZ", "+ZII"
    ///
    /// Returns:
    ///     A new PauliString instance
    ///
    /// Raises:
    ///     ValueError: If the string format is invalid
    ///
    /// Examples:
    ///     >>> PauliString.from_str("XZI")      # +XZI
    ///     >>> PauliString.from_str("-iZII")    # -iZII
    ///     >>> PauliString.from_str("+XYZ")     # +XYZ
    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        let inner = s
            .parse::<PauliString>()
            .map_err(|e| PyValueError::new_err(format!("Invalid PauliString format: {}", e)))?;
        Ok(Self { inner })
    }

    /// Sets the Pauli operator at the specified qubit index.
    ///
    /// Args:
    ///     idx: Qubit index (0 to num_qubits-1)
    ///     pauli: The Pauli operator to set
    ///
    /// Raises:
    ///     IndexError: If idx >= num_qubits
    fn set_pauli(&mut self, idx: usize, pauli: &PyPauli) -> PyResult<()> {
        self.inner
            .try_set_pauli(idx, pauli.inner)
            .map_err(crate::qis::qis_error_to_py_err)
    }

    /// Gets the Pauli operator at the specified qubit index.
    ///
    /// Args:
    ///     idx: Qubit index (0 to num_qubits-1)
    ///
    /// Returns:
    ///     The Pauli operator at the specified index
    fn get_pauli(&self, idx: usize) -> PyResult<PyPauli> {
        self.inner
            .try_get_pauli(idx)
            .map(|pauli| PyPauli { inner: pauli })
            .map_err(crate::qis::qis_error_to_py_err)
    }

    /// Returns the number of qubits in the Pauli string.
    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits
    }

    /// Returns the global phase factor.
    #[getter]
    fn phase(&self) -> PyPhase {
        PyPhase {
            inner: self.inner.phase,
        }
    }

    /// Sets the global phase factor.
    #[setter]
    fn set_phase(&mut self, phase: &PyPhase) {
        self.inner.phase = phase.inner;
    }

    /// Returns the X-component bit vector as a list of booleans.
    #[getter]
    fn x_bits(&self) -> Vec<bool> {
        self.inner.x.iter().map(|b| *b).collect()
    }

    /// Returns the Z-component bit vector as a list of booleans.
    #[getter]
    fn z_bits(&self) -> Vec<bool> {
        self.inner.z.iter().map(|b| *b).collect()
    }

    /// Returns the X-component as an integer mask.
    #[getter]
    fn x_mask(&self) -> usize {
        self.inner.x_mask()
    }

    /// Returns the Z-component as an integer mask.
    #[getter]
    fn z_mask(&self) -> usize {
        self.inner.z_mask()
    }

    /// Computes the phase factor contributed by Y operators.
    ///
    /// Y = iXZ, so n Y operators contribute i^n phase.
    ///
    /// Returns:
    ///     A Python complex number (1, i, -1, or -i)
    fn y_phase<'py>(&self, py: Python<'py>) -> Bound<'py, PyComplex> {
        let c = self.inner.y_phase();
        PyComplex::from_doubles(py, c.re, c.im)
    }

    /// Checks if this Pauli string commutes with another.
    ///
    /// Two Pauli strings commute if their symplectic inner product is 0 (mod 2).
    fn commutes_with(&self, other: &PyPauliString) -> PyResult<bool> {
        if self.inner.num_qubits != other.inner.num_qubits {
            return Err(PyValueError::new_err(
                "Pauli strings must have the same number of qubits",
            ));
        }
        Ok(self.inner.commutes_with(&other.inner))
    }

    /// Returns the dense matrix representation as a NumPy array.
    ///
    /// The matrix is the tensor product of the single-qubit Pauli operators
    /// in little-endian order: qubit 0 is the least-significant tensor factor,
    /// i.e. the string expands as P_{N-1} ⊗ ... ⊗ P_0. The global phase is included.
    ///
    /// Memory usage is O(4^N); intended for small-system analysis and verification.
    ///
    /// Returns:
    ///     A NumPy array of shape (2^num_qubits, 2^num_qubits) with complex128 dtype.
    ///
    /// Examples:
    ///     >>> from cqlib.qis import PauliString
    ///     >>> PauliString.from_str("Z").to_matrix()
    ///     array([[ 1.+0.j,  0.+0.j],
    ///            [ 0.+0.j, -1.+0.j]])
    #[allow(clippy::wrong_self_convention)]
    fn to_matrix<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<num_complex::Complex64>> {
        self.inner.to_matrix().to_pyarray(py)
    }

    /// Returns the qubit indices where the Pauli is not identity, in ascending order.
    ///
    /// Returns:
    ///     A sorted list of qubit indices supporting a non-identity Pauli (X, Y, or Z).
    ///
    /// Examples:
    ///     >>> from cqlib.qis import PauliString
    ///     >>> PauliString.from_str("XZI").support()  # X on qubit 2, Z on qubit 1
    ///     [1, 2]
    fn support(&self) -> Vec<usize> {
        self.inner.support()
    }

    /// Returns the number of qubits (the number of single-qubit Pauli operators).
    fn __len__(&self) -> usize {
        self.inner.num_qubits
    }

    /// Returns an iterator over single-qubit Pauli operators in ascending qubit index.
    ///
    /// Identity operators are included. Iteration uses snapshot semantics: modifications
    /// to the PauliString after creating the iterator do not affect the iteration.
    ///
    /// Equivalent to calling `get_pauli(i)` for i in range(num_qubits), but lazy.
    fn __iter__(slf: PyRef<'_, Self>) -> PyPauliStringIter {
        PyPauliStringIter {
            inner: slf.inner.clone(),
            cursor: 0,
            parent: slf.into(),
        }
    }

    /// Returns the Pauli operator at the given qubit index.
    ///
    /// Supports negative indexing: index -1 refers to the highest qubit index.
    ///
    /// Args:
    ///     index: Qubit index (negative values count from the end)
    ///
    /// Returns:
    ///     The Pauli operator at the specified index
    ///
    /// Raises:
    ///     IndexError: If index is out of bounds
    fn __getitem__(&self, index: isize) -> PyResult<PyPauli> {
        let len = self.inner.num_qubits;
        let resolved_index = if index < 0 {
            len.checked_add_signed(index)
        } else {
            usize::try_from(index).ok()
        };
        match resolved_index {
            Some(index) if index < len => Ok(PyPauli {
                inner: self.inner.get_pauli(index),
            }),
            _ => Err(PyIndexError::new_err(format!(
                "qubit index {index} out of bounds for PauliString with {len} qubits"
            ))),
        }
    }

    /// Computes the expectation value given a probability distribution.
    ///
    /// This calculates ⟨P⟩ = Σ_s p(s) ⟨s|P|s⟩, where p(s) is the probability of basis state |s⟩.
    ///
    /// Important: The state keys use little-endian convention: the rightmost character
    /// corresponds to qubit 0. For example, "01" means qubit 0 = 1, qubit 1 = 0.
    ///
    /// If this Pauli string contains X or Y operators (non-diagonal), the expectation
    /// value is 0 for any probability distribution over computational basis states.
    ///
    /// Args:
    ///     probs: A dict mapping state strings (e.g., "00", "01") to their probabilities.
    ///            The string uses little-endian: index 0 (leftmost) is qubit n-1.
    ///
    /// Returns:
    ///     The expectation value as a float.
    ///
    /// Raises:
    ///     ValueError: If state string length doesn't match num_qubits or contains invalid chars.
    ///
    /// Examples:
    ///     >>> ps = PauliString.from_str("IZ")  # Z on qubit 0
    ///     >>> probs = {"00": 0.5, "01": 0.5}    # 50% |00⟩, 50% |01⟩
    ///     >>> exp = ps.expectation(probs)       # ⟨Z⟩ = 0.5*1 + 0.5*(-1) = 0
    fn expectation(&self, probs: &Bound<'_, PyDict>) -> PyResult<f64> {
        let mut rust_probs: HashMap<String, f64> = HashMap::new();
        for (key, value) in probs.iter() {
            let key_str: String = key.extract()?;
            let val_f64: f64 = value.extract()?;
            rust_probs.insert(key_str, val_f64);
        }

        self.inner
            .expectation(&rust_probs)
            .map_err(qis_error_to_py_err)
    }

    /// Computes the expectation value for a statevector.
    fn expectation_statevector(&self, sv: &PyStatevector) -> PyResult<f64> {
        self.inner
            .expectation_statevector(&sv.inner)
            .map_err(qis_error_to_py_err)
    }

    /// Computes the expectation value for a density matrix.
    fn expectation_density_matrix(&self, dm: &PyDensityMatrix) -> PyResult<f64> {
        self.inner
            .expectation_density_matrix(&dm.inner)
            .map_err(qis_error_to_py_err)
    }

    /// Computes the expectation value from measurement probabilities.
    fn expectation_probs(&self, measurements: &Bound<'_, PyAny>) -> PyResult<f64> {
        let mut rust_measurements = Vec::new();
        for item in measurements.try_iter()? {
            let item = item?;
            let tuple = item.cast::<PyTuple>().map_err(|_| {
                PyValueError::new_err("Each measurement must be a tuple (PauliString, dict)")
            })?;
            if tuple.len() != 2 {
                return Err(PyValueError::new_err(
                    "Each measurement must be a tuple (PauliString, dict)",
                ));
            }
            let pauli: PyPauliString = tuple.get_item(0)?.extract()?;
            let probs_dict: Bound<'_, PyDict> = tuple.get_item(1)?.cast_into()?;

            let mut rust_probs = std::collections::HashMap::new();
            for (k, v) in probs_dict.iter() {
                let k_str: String = k.extract()?;
                let v_f64: f64 = v.extract()?;
                rust_probs.insert(k_str, v_f64);
            }
            rust_measurements.push((pauli.inner, rust_probs));
        }

        self.inner
            .expectation_probs(&rust_measurements)
            .map_err(qis_error_to_py_err)
    }

    /// Computes the variance for a statevector.
    fn variance_statevector(&self, sv: &PyStatevector) -> PyResult<f64> {
        self.inner
            .variance_statevector(&sv.inner)
            .map_err(qis_error_to_py_err)
    }

    /// Returns a new Pauli string that is the product of this and another.
    ///
    /// Raises:
    ///     ValueError: If Pauli strings have different number of qubits
    fn __mul__(&self, other: &PyPauliString) -> PyResult<Self> {
        if self.inner.num_qubits != other.inner.num_qubits {
            return Err(PyValueError::new_err(
                "Pauli strings must have the same number of qubits",
            ));
        }
        let result = &self.inner * &other.inner;
        Ok(Self { inner: result })
    }

    /// In-place multiplication with another Pauli string.
    ///
    /// Raises:
    ///     ValueError: If Pauli strings have different number of qubits
    fn __imul__(&mut self, other: &PyPauliString) -> PyResult<()> {
        if self.inner.num_qubits != other.inner.num_qubits {
            return Err(PyValueError::new_err(
                "Pauli strings must have the same number of qubits",
            ));
        }
        self.inner *= &other.inner;
        Ok(())
    }

    fn __eq__(&self, other: &PyPauliString) -> bool {
        self.inner == other.inner
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "PauliString(num_qubits={}, phase={}, x_bits={:?}, z_bits={:?})",
            self.inner.num_qubits,
            self.inner.phase,
            self.x_bits(),
            self.z_bits()
        )
    }

    /// Returns a copy of this Pauli string.
    fn copy(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// Lazy iterator over the single-qubit Pauli operators of a PauliString.
///
/// Created by `PauliString.__iter__`. Yields `Pauli` objects in ascending
/// qubit index order (identities included), raising StopIteration when exhausted.
///
/// Iteration uses snapshot semantics: modifications to the parent PauliString
/// after creating the iterator do not affect the iteration.
#[pyclass(name = "PauliStringIter", module = "cqlib.qis")]
pub struct PyPauliStringIter {
    /// Snapshot of the iterated PauliString.
    inner: PauliString,
    /// Next qubit index to yield.
    cursor: usize,
    /// Reference to the parent PauliString, kept alive for the iterator's lifetime.
    /// Never read directly: dropping the iterator releases the parent.
    #[allow(dead_code)]
    parent: Py<PyPauliString>,
}

#[pymethods]
impl PyPauliStringIter {
    /// Returns the next Pauli operator, or raises StopIteration when exhausted.
    fn __next__(&mut self) -> Option<PyPauli> {
        if self.cursor < self.inner.num_qubits {
            let pauli = PyPauli {
                inner: self.inner.get_pauli(self.cursor),
            };
            self.cursor += 1;
            Some(pauli)
        } else {
            None
        }
    }

    /// Returns the iterator itself (iterators are iterable).
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Returns the number of remaining items.
    fn __length_hint__(&self) -> usize {
        self.inner.num_qubits.saturating_sub(self.cursor)
    }

    fn __repr__(&self) -> String {
        format!(
            "<PauliStringIter object at {:p}>",
            std::ptr::addr_of!(*self)
        )
    }
}
