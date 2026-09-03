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

//! Python bindings for cqlib-core Hamiltonian module.

use crate::circuit::circuit_impl::PyCircuit;
use crate::qis::evolution::PyTrotterMode;
use crate::qis::pauli::PyPauliString;
use crate::qis::qis_error_to_py_err;
use crate::qis::state::density_matrix::PyDensityMatrix;
use crate::qis::state::statevector::PyStatevector;
use cqlib_core::qis::Observable;
use cqlib_core::qis::hamiltonian::Hamiltonian;
use num_complex::Complex64;
use numpy::{PyArray2, ToPyArray};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyComplex, PyDict, PyList, PyTuple};
use std::fmt;

/// A quantum Hamiltonian represented as a sum of Pauli strings.
///
/// A `Hamiltonian` is essentially a sparse representation of a $2^N \times 2^N$
/// matrix, expressed as $H = \sum_k c_k P_k$, where $c_k$ is a complex coefficient
/// and $P_k$ is an $N$-qubit Pauli string.
///
/// This is commonly used for defining system energies, observables for expectation
/// value calculations, and operators for time evolution.
///
/// Examples:
///     >>> from cqlib.qis import Hamiltonian, PauliString
///     >>> # Create a 2-qubit Hamiltonian
///     >>> h = Hamiltonian(2)
///     >>> # Add terms: H = 0.5 * ZZ + 0.3 * XX
///     >>> h.add_term(PauliString.from_str("ZZ"), 0.5)
///     >>> h.add_term(PauliString.from_str("XX"), 0.3)
///     >>> # Simplify to merge duplicate terms
///     >>> h.simplify()
#[pyclass(name = "Hamiltonian", module = "cqlib.qis", from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyHamiltonian {
    pub(crate) inner: Hamiltonian,
}

impl From<Hamiltonian> for PyHamiltonian {
    fn from(inner: Hamiltonian) -> Self {
        Self { inner }
    }
}

impl From<PyHamiltonian> for Hamiltonian {
    fn from(value: PyHamiltonian) -> Self {
        value.inner
    }
}

impl fmt::Display for PyHamiltonian {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// Extract a complex coefficient from a Python object.
///
/// Supports:
/// - float/int -> real number
/// - complex -> complex number
/// - tuple (real, imag) -> complex number
fn extract_complex(value: &Bound<'_, PyAny>) -> PyResult<Complex64> {
    // Try to extract as float first
    if let Ok(val) = value.extract::<f64>() {
        return Ok(Complex64::new(val, 0.0));
    }
    // Try to extract as Python complex
    if let Ok(py_complex) = value.cast::<PyComplex>() {
        return Ok(Complex64::new(py_complex.real(), py_complex.imag()));
    }

    // Try to extract as tuple (real, imag)
    if let Ok(tuple) = value.cast::<PyTuple>()
        && tuple.len() == 2
    {
        let real: f64 = tuple.get_item(0)?.extract()?;
        let imag: f64 = tuple.get_item(1)?.extract()?;
        return Ok(Complex64::new(real, imag));
    }

    Err(PyValueError::new_err(
        "Coefficient must be a float, int, complex, or tuple (real, imag)",
    ))
}

/// Convert a Complex64 to a Python complex number.
fn to_py_complex<'py>(py: Python<'py>, c: Complex64) -> Bound<'py, PyComplex> {
    PyComplex::from_doubles(py, c.re, c.im)
}

#[pymethods]
impl PyHamiltonian {
    /// Creates a new empty Hamiltonian.
    ///
    /// The resulting Hamiltonian represents the zero operator for the given
    /// number of qubits.
    ///
    /// Args:
    ///     num_qubits: The number of qubits this operator acts on.
    ///
    /// Examples:
    ///     >>> from cqlib.qis import Hamiltonian
    ///     >>> h = Hamiltonian(3)  # 3-qubit Hamiltonian
    #[new]
    fn new(num_qubits: usize) -> Self {
        Self {
            inner: Hamiltonian::new(num_qubits),
        }
    }

    /// Creates a Hamiltonian from a single Pauli string with a coefficient of 1.0.
    ///
    /// Args:
    ///     pauli: The Pauli string to wrap into a Hamiltonian.
    ///
    /// Returns:
    ///     A new Hamiltonian representing H = 1.0 * P.
    ///
    /// Examples:
    ///     >>> from cqlib.qis import Hamiltonian, PauliString
    ///     >>> h = Hamiltonian.from_pauli(PauliString.from_str("ZZ"))
    #[staticmethod]
    fn from_pauli(pauli: &PyPauliString) -> Self {
        Self {
            inner: Hamiltonian::from_pauli(pauli.inner.clone()),
        }
    }

    /// Creates a Hamiltonian from a list of (PauliString, coefficient) tuples.
    ///
    /// Args:
    ///     terms: A list of tuples, each containing a PauliString and a coefficient.
    ///            Coefficients can be float, int, complex, or tuple (real, imag).
    ///
    /// Returns:
    ///     A new Hamiltonian instance.
    ///
    /// Raises:
    ///     ValueError: If not all Pauli strings have the same number of qubits.
    ///
    /// Examples:
    ///     >>> from cqlib.qis import Hamiltonian, PauliString
    ///     >>> terms = [
    ///     ...     (PauliString.from_str("ZZ"), 0.5),
    ///     ...     (PauliString.from_str("XX"), (0.0, 0.3)),  # complex 0.3j
    ///     ... ]
    ///     >>> h = Hamiltonian.from_list(terms)
    #[staticmethod]
    fn from_list(terms: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut rust_terms: Vec<(cqlib_core::qis::pauli::PauliString, Complex64)> = Vec::new();

        for item in terms.try_iter()? {
            let item = item?;
            let tuple = item.cast::<PyTuple>().map_err(|_| {
                PyValueError::new_err("Each term must be a tuple (PauliString, coefficient)")
            })?;

            if tuple.len() != 2 {
                return Err(PyValueError::new_err(
                    "Each term must be a tuple (PauliString, coefficient)",
                ));
            }

            let pauli: PyPauliString = tuple.get_item(0)?.extract()?;
            let coeff = extract_complex(&tuple.get_item(1)?)?;

            rust_terms.push((pauli.inner, coeff));
        }

        let inner = Hamiltonian::from_list(rust_terms).map_err(qis_error_to_py_err)?;

        Ok(Self { inner })
    }

    /// Adds a new Pauli string term with a given coefficient to the Hamiltonian.
    ///
    /// Args:
    ///     op: The Pauli string operator to add.
    ///     coeff: The coefficient for this term. Can be float, int, complex, or tuple (real, imag).
    ///
    /// Returns:
    ///     self (for method chaining)
    ///
    /// Raises:
    ///     ValueError: If the number of qubits in the operator does not match the Hamiltonian.
    ///
    /// Examples:
    ///     >>> from cqlib.qis import Hamiltonian, PauliString
    ///     >>> h = Hamiltonian(2)
    ///     >>> h.add_term(PauliString.from_str("ZZ"), 0.5)
    ///     >>> h.add_term(PauliString.from_str("XX"), (0.0, 0.3))  # complex 0.3j
    fn add_term<'py>(&mut self, op: &PyPauliString, coeff: &Bound<'py, PyAny>) -> PyResult<()> {
        let coeff = extract_complex(coeff)?;
        self.inner
            .add_term(op.inner.clone(), coeff)
            .map_err(qis_error_to_py_err)?;
        Ok(())
    }

    /// Simplifies the Hamiltonian by combining terms with the same Pauli string.
    ///
    /// This method performs two optimizations:
    /// 1. **Phase Normalization**: Absorbs any internal phases from the PauliString
    ///    into the complex coefficient.
    /// 2. **Term Aggregation**: Groups terms with identical Pauli strings and sums
    ///    their coefficients. Terms with near-zero coefficients are removed.
    ///
    /// This is important for optimizing performance before quantum simulations.
    ///
    /// Examples:
    ///     >>> from cqlib.qis import Hamiltonian, PauliString
    ///     >>> h = Hamiltonian(2)
    ///     >>> h.add_term(PauliString.from_str("ZZ"), 0.5)
    ///     >>> h.add_term(PauliString.from_str("ZZ"), 0.3)  # duplicate
    ///     >>> h.simplify()  # Now H = 0.8 * ZZ
    fn simplify(&mut self) {
        self.inner.simplify();
    }

    /// Scales all terms in the Hamiltonian by a complex factor.
    ///
    /// Args:
    ///     factor: The scaling factor. Can be float, int, complex, or tuple (real, imag).
    ///
    /// Examples:
    ///     >>> from cqlib.qis import Hamiltonian, PauliString
    ///     >>> h = Hamiltonian(2)
    ///     >>> h.add_term(PauliString.from_str("ZZ"), 1.0)
    ///     >>> h.scale(2.0)  # H = 2.0 * ZZ
    fn scale<'py>(&mut self, factor: &Bound<'py, PyAny>) -> PyResult<()> {
        let factor = extract_complex(factor)?;
        self.inner.scale(factor);
        Ok(())
    }

    /// Returns the number of qubits this Hamiltonian acts on.
    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits
    }

    /// Returns the list of terms as (PauliString, coefficient) tuples.
    ///
    /// Coefficients are returned as Python complex numbers.
    ///
    /// Returns:
    ///     A list of tuples, each containing a PauliString and its complex coefficient.
    #[getter]
    fn terms<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        use pyo3::IntoPyObject;
        let list = PyList::empty(py);
        for (pauli, coeff) in &self.inner.terms {
            let py_pauli = PyPauliString {
                inner: pauli.clone(),
            };
            let py_coeff = to_py_complex(py, *coeff);
            let tuple = PyTuple::new(
                py,
                vec![py_pauli.into_pyobject(py)?.into_any(), py_coeff.into_any()],
            )?;
            list.append(tuple)?;
        }
        Ok(list)
    }

    /// Returns the number of terms in the Hamiltonian.
    ///
    /// This is the length of the terms list before simplification.
    #[getter]
    fn num_terms(&self) -> usize {
        self.inner.terms.len()
    }

    /// Returns true if all Pauli terms mutually commute.
    fn all_terms_commute(&self) -> bool {
        self.inner.all_terms_commute()
    }

    /// Computes the dense matrix representation as a NumPy array.
    ///
    /// Evaluates H = Σ_k c_k P_k by summing each term's Pauli string matrix
    /// (in little-endian qubit order, including Pauli phases) multiplied by
    /// its coefficient. An empty Hamiltonian yields the zero matrix.
    ///
    /// Memory usage is O(4^N); intended for small-system analysis and verification.
    ///
    /// Returns:
    ///     A NumPy array of shape (2^num_qubits, 2^num_qubits) with complex128 dtype.
    ///
    /// Raises:
    ///     ValueError: If a term's qubit count does not match the Hamiltonian's
    ///         (defensive; the public constructors already reject such terms).
    ///
    /// Examples:
    ///     >>> from cqlib.qis import Hamiltonian, PauliString
    ///     >>> h = Hamiltonian.from_pauli(PauliString.from_str("Z"))
    ///     >>> h.to_matrix()
    ///     array([[ 1.+0.j,  0.+0.j],
    ///            [ 0.+0.j, -1.+0.j]])
    #[allow(clippy::wrong_self_convention)]
    fn to_matrix<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray2<Complex64>>> {
        let matrix = py
            .detach(|| self.inner.to_matrix())
            .map_err(qis_error_to_py_err)?;
        Ok(matrix.to_pyarray(py))
    }

    /// Adds two Hamiltonians together.
    ///
    /// Note: This performs a simple lazy concatenation of the term lists.
    /// It does not automatically merge identical terms. Call `simplify()` after
    /// addition to optimize the result.
    ///
    /// Args:
    ///     other: The Hamiltonian to add.
    ///
    /// Returns:
    ///     A new Hamiltonian containing all terms from both.
    ///
    /// Raises:
    ///     ValueError: If the Hamiltonians have different numbers of qubits.
    ///
    /// Examples:
    ///     >>> from cqlib.qis import Hamiltonian, PauliString
    ///     >>> h1 = Hamiltonian(2)
    ///     >>> h1.add_term(PauliString.from_str("ZZ"), 0.5)
    ///     >>> h2 = Hamiltonian(2)
    ///     >>> h2.add_term(PauliString.from_str("XX"), 0.3)
    ///     >>> h3 = h1 + h2  # Contains both ZZ and XX terms
    fn __add__(&self, other: &PyHamiltonian) -> PyResult<Self> {
        if self.inner.num_qubits != other.inner.num_qubits {
            return Err(PyValueError::new_err(format!(
                "Cannot add Hamiltonians with different numbers of qubits: {} vs {}",
                self.inner.num_qubits, other.inner.num_qubits
            )));
        }
        let result = self.inner.clone() + other.inner.clone();
        Ok(Self { inner: result })
    }

    /// In-place addition of another Hamiltonian.
    ///
    /// Raises:
    ///     ValueError: If the Hamiltonians have different numbers of qubits.
    fn __iadd__(&mut self, other: &PyHamiltonian) -> PyResult<()> {
        if self.inner.num_qubits != other.inner.num_qubits {
            return Err(PyValueError::new_err(format!(
                "Cannot add Hamiltonians with different numbers of qubits: {} vs {}",
                self.inner.num_qubits, other.inner.num_qubits
            )));
        }
        self.inner.terms.extend(other.inner.terms.clone());
        Ok(())
    }

    fn __eq__(&self, other: &PyHamiltonian) -> bool {
        self.inner == other.inner
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "Hamiltonian(num_qubits={}, num_terms={})",
            self.inner.num_qubits,
            self.inner.terms.len()
        )
    }

    /// Converts the Hamiltonian to a Trotterized time evolution circuit.
    ///
    /// Implements Trotter-Suzuki decomposition to approximate the time evolution
    /// operator U(t) = e^(-iHt) as a sequence of Pauli rotations.
    ///
    /// Args:
    ///     time: The total evolution time t.
    ///     steps: The number of Trotter steps n (must be > 0).
    ///     mode: The Trotter decomposition mode (FirstOrder, SecondOrder, Randomized).
    ///
    /// Returns:
    ///     The approximated time evolution circuit.
    ///
    /// Raises:
    ///     ValueError: If steps is 0, Hamiltonian is empty, or other error occurs.
    ///
    /// Mathematical Formulation:
    ///
    /// For H = Σ_k c_k P_k, the decomposition approximates:
    ///
    /// ```text
    /// First Order:
    /// U(t) ≈ Π_{s=1}^{n} Π_k e^(-i c_k Δt · P_k), where Δt = t/n
    ///
    /// Second Order:
    /// U(t) ≈ Π_{s=1}^{n} [Π_k e^(-i c_k Δt/2 · P_k) · Π_k e^(-i c_k Δt/2 · P_k)]
    /// (with reverse order in second product)
    /// ```
    ///
    /// Examples:
    /// ```python
    /// >>> from cqlib.qis import Hamiltonian, PauliString, TrotterMode
    /// >>> h = Hamiltonian(2)
    /// >>> h.add_term(PauliString.from_str("ZZ"), 0.5)
    /// >>> h.add_term(PauliString.from_str("XX"), 0.3)
    /// >>> circuit = h.to_trotter_circuit(1.0, 10, TrotterMode.first_order())
    /// ```
    #[allow(clippy::wrong_self_convention)]
    fn to_trotter_circuit(
        &self,
        time: f64,
        steps: usize,
        mode: &PyTrotterMode,
    ) -> PyResult<PyCircuit> {
        let circuit = self
            .inner
            .to_trotter_circuit(time, steps, mode.inner)
            .map_err(qis_error_to_py_err)?;

        Ok(PyCircuit { inner: circuit })
    }

    /// Converts the Hamiltonian to a time evolution circuit.
    ///
    /// Commuting terms use the exact single-pass decomposition. Non-commuting
    /// terms fall back to the requested Trotter mode and number of steps.
    fn to_evolution_circuit(
        &self,
        time: f64,
        steps: usize,
        mode: &PyTrotterMode,
    ) -> PyResult<PyCircuit> {
        let circuit = self
            .inner
            .to_evolution_circuit(time, steps, mode.inner)
            .map_err(qis_error_to_py_err)?;

        Ok(PyCircuit { inner: circuit })
    }

    /// Returns a copy of this Hamiltonian.
    ///
    /// Returns:
    ///     A new Hamiltonian instance with the same terms.
    fn copy(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
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
}
