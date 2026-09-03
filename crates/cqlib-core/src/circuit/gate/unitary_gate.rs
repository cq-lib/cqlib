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

//! Custom Unitary Gate Definitions
//!
//! This module provides [`UnitaryGate`], a type for defining custom quantum gates
//! via their unitary matrix representation. Unlike [`StandardGate`](crate::circuit::gate::StandardGate),
//! which represents predefined gates, `UnitaryGate` allows users to specify arbitrary
//! unitary operations.

use crate::circuit::circuit_to_matrix;
use crate::circuit::error::CircuitError;
use crate::circuit::gate::circuit_gate::FrozenCircuit;
use crate::circuit::symbolic_matrix::{SymbolicMatrix, evaluate_symbolic_matrix};
use alloc::borrow::Cow;
use ndarray::Array2;
use num_complex::Complex;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use uuid::Uuid;

/// Matrix representation attached to a [`UnitaryGate`].
#[derive(Clone, Debug, PartialEq)]
pub enum UnitaryMatrix {
    /// A concrete numeric matrix. This is only valid for non-parameterized
    /// unitary gate definitions.
    Numeric(Array2<Complex<f64>>),
    /// A symbolic matrix whose free symbols are mapped positionally by the
    /// gate's `matrix_params`.
    Symbolic(SymbolicMatrix),
}

/// A user-defined unitary quantum gate.
///
/// `UnitaryGate` represents a custom quantum gate defined by its unitary matrix
/// or by an internal circuit representation. Each gate has a unique identifier
/// for equality comparisons and hashing.
///
/// # Examples
///
/// ```
/// use cqlib_core::circuit::gate::UnitaryGate;
/// use ndarray::array;
/// use num_complex::Complex;
///
/// // Create a custom 1-qubit gate
/// let mut gate = UnitaryGate::new("MyGate", 1, 0);
///
/// // Define the unitary matrix (Pauli-X as example)
/// let matrix = array![
///     [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
///     [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
/// ];
///
/// // Attach the matrix
/// let gate = gate.with_matrix(matrix).unwrap();
///
/// assert_eq!(gate.label(), "MyGate");
/// assert_eq!(gate.num_qubits(), 1);
/// ```
#[derive(Clone)]
pub struct UnitaryGate
where
    Self: Send + Sync,
{
    /// Unique identifier for this gate definition.
    ///
    /// Used for equality comparisons and hashing. Each `UnitaryGate::new`
    /// call generates a fresh UUID.
    id: Uuid,
    /// A human-readable label for the gate (e.g., "QFT", "Oracle").
    label: Arc<String>,
    /// The matrix representation of the gate, wrapped in `Arc` for cheap cloning.
    ///
    /// Can be `None` if the gate is purely symbolic (defined by circuit only).
    matrix: Option<Arc<UnitaryMatrix>>,
    /// Ordered formal parameter names for a symbolic matrix representation.
    matrix_params: Option<Arc<[String]>>,
    /// The number of qubits this gate acts on.
    num_qubits: u16,
    /// The number of parameters each application of this gate requires.
    num_params: u16,
    /// Optional internal circuit representation.
    circuit: Option<Arc<FrozenCircuit>>,
}

impl UnitaryGate {
    /// Creates a new unitary gate definition without a matrix.
    ///
    /// The gate is assigned a unique ID and can later be configured with
    /// a matrix using [`with_matrix`](Self::with_matrix) or with a circuit
    /// using [`with_circuit`](Self::with_circuit).
    ///
    /// # Arguments
    ///
    /// * `label` - A descriptive name for the gate.
    /// * `num_qubits` - The number of qubits the gate operates on.
    ///
    /// # Returns
    ///
    /// A new `UnitaryGate` with no matrix attached.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::gate::UnitaryGate;
    ///
    /// let gate = UnitaryGate::new("QFT_3", 3, 0);
    /// assert_eq!(gate.label(), "QFT_3");
    /// assert_eq!(gate.num_qubits(), 3);
    /// assert_eq!(gate.num_params(), 0);
    /// assert!(gate.matrix().is_none());
    /// ```
    pub fn new(label: &str, num_qubits: u16, num_params: u16) -> Self {
        Self {
            id: Uuid::new_v4(),
            label: Arc::new(label.to_string()),
            matrix: None,
            matrix_params: None,
            num_qubits,
            num_params,
            circuit: None,
        }
    }

    /// Returns the label of the gate.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::gate::UnitaryGate;
    ///
    /// let gate = UnitaryGate::new("Oracle", 2, 0);
    /// assert_eq!(gate.label(), "Oracle");
    /// ```
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the number of qubits this gate acts on.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::gate::UnitaryGate;
    ///
    /// let gate = UnitaryGate::new("TwoQubitGate", 2, 0);
    /// assert_eq!(gate.num_qubits(), 2);
    /// ```
    pub fn num_qubits(&self) -> u16 {
        self.num_qubits
    }

    /// Returns the number of parameters this gate expects per application.
    pub fn num_params(&self) -> u16 {
        self.num_params
    }

    /// Returns the matrix representation if available.
    ///
    /// # Returns
    ///
    /// - `Some(&Array2)`: The unitary matrix if it has been attached.
    /// - `None`: If no matrix was provided.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::gate::UnitaryGate;
    ///
    /// let gate = UnitaryGate::new("SymbolicGate", 1, 0);
    /// assert!(gate.matrix().is_none());
    /// ```
    pub fn matrix(&self) -> Option<&Array2<Complex<f64>>> {
        match self.matrix.as_deref() {
            Some(UnitaryMatrix::Numeric(matrix)) => Some(matrix),
            _ => None,
        }
    }

    /// Returns the symbolic matrix representation if available.
    pub fn symbolic_matrix(&self) -> Option<&SymbolicMatrix> {
        match self.matrix.as_deref() {
            Some(UnitaryMatrix::Symbolic(matrix)) => Some(matrix),
            _ => None,
        }
    }

    /// Returns the ordered formal parameter names for the symbolic matrix.
    pub fn matrix_params(&self) -> Option<&[String]> {
        self.matrix_params.as_deref()
    }

    /// Returns the matrix representation if one has been attached.
    pub fn matrix_repr(&self) -> Option<&UnitaryMatrix> {
        self.matrix.as_deref()
    }

    /// Returns the internal circuit representation if available.
    ///
    /// Some unitary gates are defined by their circuit decomposition
    /// rather than an explicit matrix.
    pub fn circuit(&self) -> &Option<Arc<FrozenCircuit>> {
        &self.circuit
    }

    /// Attaches a matrix to the unitary definition.
    ///
    /// Consumes the gate and returns a new one with the matrix attached.
    /// The matrix dimensions must match the expected size for the gate's
    /// qubit count: $2^n \times 2^n$ where $n$ is `num_qubits`.
    ///
    /// # Arguments
    ///
    /// * `mat` - A square matrix of size $2^N \times 2^N$.
    ///
    /// # Returns
    ///
    /// - `Ok(Self)`: The gate with matrix attached.
    /// - `Err(String)`: Error message if dimensions are incorrect.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::gate::UnitaryGate;
    /// use ndarray::array;
    /// use num_complex::Complex;
    ///
    /// let gate = UnitaryGate::new("Hadamard", 1, 0);
    ///
    /// // Correct 2x2 matrix for 1 qubit
    /// let h = Complex::new(1.0 / f64::sqrt(2.0), 0.0);
    /// let matrix = array![
    ///     [h, h],
    ///     [h, -h],
    /// ];
    ///
    /// let gate = gate.with_matrix(matrix).unwrap();
    /// assert!(gate.matrix().is_some());
    /// ```
    pub fn with_matrix(mut self, mat: Array2<Complex<f64>>) -> Result<Self, CircuitError> {
        if self.num_params != 0 {
            return Err(CircuitError::ParameterCountMismatch {
                expected: 0,
                actual: self.num_params as usize,
            });
        }
        self.validate_matrix_shape(&mat)?;

        self.matrix = Some(Arc::new(UnitaryMatrix::Numeric(mat)));
        self.matrix_params = None;
        Ok(self)
    }

    /// Attaches a symbolic matrix to the unitary definition.
    ///
    /// `params` defines the positional mapping between each gate application's
    /// arguments and the free symbols inside `matrix`.
    pub fn with_symbolic_matrix<I, S>(
        mut self,
        params: I,
        matrix: SymbolicMatrix,
    ) -> Result<Self, CircuitError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.validate_symbolic_matrix_shape(&matrix)?;
        let params: Vec<String> = params.into_iter().map(Into::into).collect();
        self.validate_symbolic_matrix_params(&params, &matrix)?;

        self.matrix = Some(Arc::new(UnitaryMatrix::Symbolic(matrix)));
        self.matrix_params = Some(Arc::from(params.into_boxed_slice()));
        Ok(self)
    }

    /// Resolves this gate into a concrete matrix for the supplied parameter values.
    pub fn matrix_for_params(
        &self,
        params: &[f64],
    ) -> Result<Cow<'_, Array2<Complex<f64>>>, CircuitError> {
        self.validate_param_values(params)?;

        if let Some(matrix) = self.matrix.as_deref() {
            match matrix {
                UnitaryMatrix::Numeric(matrix) => return Ok(Cow::Borrowed(matrix)),
                UnitaryMatrix::Symbolic(matrix) => {
                    let matrix_params = self
                        .matrix_params
                        .as_deref()
                        .ok_or(CircuitError::SymbolicParameterError)?;
                    let bindings: HashMap<&str, f64> = matrix_params
                        .iter()
                        .map(String::as_str)
                        .zip(params.iter().copied())
                        .collect();
                    let matrix = evaluate_symbolic_matrix(matrix, &Some(bindings))
                        .map_err(|_| CircuitError::SymbolicParameterError)?;
                    self.validate_matrix_shape(&matrix)?;
                    return Ok(Cow::Owned(matrix));
                }
            }
        }

        if let Some(circuit) = self.circuit.as_ref() {
            let inner = circuit.circuit();
            let symbols = circuit.used_symbols();
            let mut bindings = HashMap::new();
            for (symbol, value) in symbols.iter().zip(params.iter()) {
                bindings.insert(symbol.as_str(), *value);
            }
            let assigned = inner
                .assign_parameters(&Some(bindings))
                .map_err(|_| CircuitError::SymbolicParameterError)?;
            return circuit_to_matrix(&assigned, None).map(Cow::Owned);
        }

        Err(CircuitError::NoMatrixRepresentation)
    }

    fn validate_param_values(&self, params: &[f64]) -> Result<(), CircuitError> {
        if params.len() != self.num_params as usize {
            return Err(CircuitError::ParameterCountMismatch {
                expected: self.num_params as usize,
                actual: params.len(),
            });
        }
        for (idx, value) in params.iter().copied().enumerate() {
            if !value.is_finite() {
                return Err(CircuitError::InvalidParameterValue(idx, value));
            }
        }
        Ok(())
    }

    fn validate_symbolic_matrix_params(
        &self,
        params: &[String],
        matrix: &SymbolicMatrix,
    ) -> Result<(), CircuitError> {
        if params.len() != self.num_params as usize {
            return Err(CircuitError::ParameterCountMismatch {
                expected: self.num_params as usize,
                actual: params.len(),
            });
        }

        let mut seen = HashSet::new();
        for param in params {
            if !seen.insert(param.as_str()) {
                return Err(CircuitError::InvalidOperation(format!(
                    "duplicate symbolic matrix parameter '{param}'"
                )));
            }
        }

        for value in matrix.iter() {
            for symbol in value
                .re
                .get_symbols()
                .into_iter()
                .chain(value.im.get_symbols())
            {
                if !seen.contains(symbol.as_str()) {
                    return Err(CircuitError::InvalidOperation(format!(
                        "symbolic matrix contains undeclared parameter '{symbol}'"
                    )));
                }
            }
        }

        Ok(())
    }

    fn validate_circuit_signature(&self, circuit: &FrozenCircuit) -> Result<(), CircuitError> {
        let actual_qubits = circuit.circuit().qubits().len();
        if actual_qubits != self.num_qubits as usize {
            return Err(CircuitError::QubitCountMismatch {
                expected: self.num_qubits as usize,
                actual: actual_qubits,
            });
        }

        let actual_params = circuit.used_symbols().len();
        if actual_params != self.num_params as usize {
            return Err(CircuitError::ParameterCountMismatch {
                expected: self.num_params as usize,
                actual: actual_params,
            });
        }

        Ok(())
    }

    /// Attaches a circuit representation to the unitary definition.
    ///
    /// This allows the gate to be defined by its circuit decomposition,
    /// which is useful for inverse operations and optimization.
    ///
    /// # Arguments
    ///
    /// * `circuit` - The frozen circuit representing this gate.
    pub fn with_circuit(mut self, circuit: Arc<FrozenCircuit>) -> Result<Self, CircuitError> {
        self.validate_circuit_signature(&circuit)?;
        self.circuit = Some(circuit);
        Ok(self)
    }

    fn validate_matrix_shape(&self, mat: &Array2<Complex<f64>>) -> Result<(), CircuitError> {
        self.validate_symbolic_matrix_shape(mat)?;

        // Reject matrices containing NaN or infinite elements.
        for ((row, col), val) in mat.indexed_iter() {
            if !val.re.is_finite() || !val.im.is_finite() {
                return Err(CircuitError::InvalidOperation(format!(
                    "Matrix contains non-finite element at ({row}, {col}): {val}"
                )));
            }
        }

        // Reject non-unitary matrices: U†U must be approximately I.
        const UNITARITY_EPS: f64 = 1e-10;
        let conj_t = mat.t().mapv(|x| x.conj());
        let product = conj_t.dot(mat);
        for (i, val) in product.diag().iter().enumerate() {
            let diff = (val - Complex::new(1.0, 0.0)).norm();
            if diff > UNITARITY_EPS {
                return Err(CircuitError::InvalidOperation(format!(
                    "Matrix is not unitary: (U†U)[{i},{i}] = {val}, expected 1.0"
                )));
            }
        }
        for ((i, j), val) in product.indexed_iter() {
            if i != j {
                let diff = val.norm();
                if diff > UNITARITY_EPS {
                    return Err(CircuitError::InvalidOperation(format!(
                        "Matrix is not unitary: (U†U)[{i},{j}] = {val}, expected 0.0"
                    )));
                }
            }
        }

        Ok(())
    }

    fn validate_symbolic_matrix_shape<T>(&self, mat: &Array2<T>) -> Result<(), CircuitError> {
        let expected_dim = 1usize.checked_shl(self.num_qubits as u32).ok_or_else(|| {
            CircuitError::InvalidOperation(format!(
                "cannot build matrix for {} qubits: dimension overflows usize",
                self.num_qubits
            ))
        })?;
        if mat.shape() != [expected_dim, expected_dim] {
            return Err(CircuitError::InvalidOperation(format!(
                "Matrix dimension mismatch. Expected {}x{}, got {}x{}",
                expected_dim,
                expected_dim,
                mat.nrows(),
                mat.ncols()
            )));
        }

        Ok(())
    }
}

impl fmt::Debug for UnitaryGate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnitaryGate")
            .field("id", &self.id)
            .field("label", &self.label)
            .field("matrix", &self.matrix)
            .field("matrix_params", &self.matrix_params)
            .field("num_qubits", &self.num_qubits)
            .field("num_params", &self.num_params)
            .field("circuit", &self.circuit)
            .finish()
    }
}

impl Eq for UnitaryGate {}

impl PartialEq for UnitaryGate {
    /// Equality is based solely on the unique ID.
    ///
    /// Two `UnitaryGate` instances are considered equal only if they
    /// were created by the same constructor call (share the same UUID).
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for UnitaryGate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Display for UnitaryGate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.label().fmt(f)
    }
}

#[cfg(test)]
#[path = "./unitary_gate_test.rs"]
mod unitary_gate_test;
