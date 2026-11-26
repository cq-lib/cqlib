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

//! Circuit-Based Gate Definitions
//!
//! This module provides [`CircuitGate`] and [`FrozenCircuit`], allowing
//! quantum circuits to be used as reusable gate components within other circuits.
//! This enables hierarchical circuit construction and custom composite gates.

use crate::circuit::Qubit;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::circuit_to_matrix;
use crate::circuit::error::CircuitError;
use crate::circuit::symbolic_matrix::{SymbolicMatrix, circuit_to_symbolic_matrix};
use indexmap::IndexSet;
use ndarray::Array2;
use num_complex::Complex64;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

/// An immutable, frozen circuit for use in gate definitions.
///
/// `FrozenCircuit` wraps a [`Circuit`] in an immutable container, ensuring
/// that the circuit definition cannot be modified after creation. This is
/// essential for maintaining consistency when circuits are used as gate
/// definitions.
///
/// # Examples
///
/// ```
/// use cqlib_core::circuit::Circuit;
/// use cqlib_core::circuit::gate::FrozenCircuit;
///
/// // Create a circuit and freeze it
/// let circuit = Circuit::new(2);
/// let frozen = FrozenCircuit::new(circuit);
///
/// assert_eq!(frozen.circuit().qubits().len(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct FrozenCircuit {
    pub(crate) circuit: Circuit,
    used_symbols: Arc<IndexSet<String>>,
    symbolic_matrix_cache: Arc<OnceLock<Arc<SymbolicMatrix>>>,
    storage_order_symbolic_matrix_cache: Arc<OnceLock<Arc<SymbolicMatrix>>>,
}

impl FrozenCircuit {
    /// Creates a new frozen circuit from a [`Circuit`].
    ///
    /// The circuit is moved into the frozen container and cannot be
    /// modified afterwards.
    ///
    /// # Arguments
    ///
    /// * `circuit` - The circuit to freeze.
    ///
    /// # Returns
    ///
    /// A new `FrozenCircuit` wrapping the provided circuit.
    pub fn new(circuit: Circuit) -> Self {
        let used_symbols = Arc::new(circuit.used_symbols());
        Self {
            circuit,
            used_symbols,
            symbolic_matrix_cache: Arc::new(OnceLock::new()),
            storage_order_symbolic_matrix_cache: Arc::new(OnceLock::new()),
        }
    }

    /// Returns a reference to the inner circuit.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::Circuit;
    /// use cqlib_core::circuit::gate::FrozenCircuit;
    ///
    /// let circuit = Circuit::new(3);
    /// let frozen = FrozenCircuit::new(circuit);
    ///
    /// assert_eq!(frozen.circuit().qubits().len(), 3);
    /// ```
    pub fn circuit(&self) -> &Circuit {
        &self.circuit
    }

    /// Returns symbols referenced by the frozen circuit's executable IR.
    ///
    /// The set is computed when the circuit is frozen and preserves the inner
    /// circuit's stable symbol-registry order.
    pub fn used_symbols(&self) -> &IndexSet<String> {
        &self.used_symbols
    }

    /// The default matrix order, from least to most significant bit.
    ///
    /// Circuit-backed UnitaryGate definitions map this order positionally to
    /// the caller's qubits. CircuitGate instead uses circuit storage order.
    pub(crate) fn qubits_in_numeric_order(&self) -> Vec<Qubit> {
        let mut qubits = self.circuit.qubits();
        qubits.sort_unstable();
        qubits
    }

    /// Returns the cached symbolic matrix for this frozen circuit.
    ///
    /// The matrix is computed in the circuit's default qubit order on the
    /// first successful call and reused afterwards by composite-gate matrix
    /// construction.
    pub fn symbolic_matrix(&self) -> Result<Arc<SymbolicMatrix>, CircuitError> {
        if let Some(matrix) = self.symbolic_matrix_cache.get() {
            return Ok(matrix.clone());
        }

        let order: Vec<usize> = self
            .qubits_in_numeric_order()
            .iter()
            .map(|q| q.index())
            .collect();
        let matrix = Arc::new(circuit_to_symbolic_matrix(&self.circuit, Some(&order))?);
        let _ = self.symbolic_matrix_cache.set(matrix.clone());
        Ok(self.symbolic_matrix_cache.get().cloned().unwrap_or(matrix))
    }

    /// CircuitGate's positional basis is distinct from the numeric-sorted
    /// default used by the public FrozenCircuit matrix and UnitaryGate.
    fn symbolic_matrix_in_storage_order(&self) -> Result<Arc<SymbolicMatrix>, CircuitError> {
        if let Some(matrix) = self.storage_order_symbolic_matrix_cache.get() {
            return Ok(matrix.clone());
        }

        let order: Vec<usize> = self.circuit.qubits().iter().map(|q| q.index()).collect();
        let matrix = Arc::new(circuit_to_symbolic_matrix(&self.circuit, Some(&order))?);
        let _ = self.storage_order_symbolic_matrix_cache.set(matrix.clone());
        Ok(self
            .storage_order_symbolic_matrix_cache
            .get()
            .cloned()
            .unwrap_or(matrix))
    }
}

impl PartialEq for FrozenCircuit {
    /// Compares two frozen circuits by their defining circuit and used-symbol
    /// set.
    ///
    /// The defining circuits compare with [`Circuit`]'s structural equality,
    /// which ignores process-local circuit identity. The derived symbolic
    /// matrix cache never participates: freezing equal circuits and then
    /// materializing the matrix on only one side keeps them equal.
    fn eq(&self, other: &Self) -> bool {
        self.circuit == other.circuit && self.used_symbols == other.used_symbols
    }
}

/// A composite gate defined by a quantum circuit.
///
/// `CircuitGate` encapsulates a frozen circuit as a reusable gate operation.
/// When used in a circuit, its parameters are mapped positionally to the
/// symbolic parameters of the inner circuit.
///
/// # Parameter Resolution Logic
///
/// It is important to distinguish between the symbols defined inside the `CircuitGate`
/// and the arguments passed to it from the outside.
///
/// **Example Flow:**
///
/// 1. **Internal Expression**: The inner circuit contains a gate with a parameter expression, e.g., `theta + 1`.
/// 2. **Binding**: The `CircuitGate` usage defines a mapping from an external argument to the internal symbol, e.g., `theta` $\leftarrow$ `2 * x`.
/// 3. **Evaluation**:
///    If the external argument `x` is `0.5`:
///    - First, the binding is resolved: `theta` becomes `2 * 0.5 = 1.0`.
///    - Then, the internal circuit evaluates its expression using this value: `1.0 + 1 = 2.0`.
///
/// # Examples
///
/// ```
/// use cqlib_core::circuit::Circuit;
/// use cqlib_core::circuit::gate::{CircuitGate, FrozenCircuit};
///
/// // Create a simple circuit and wrap as a CircuitGate
/// let circuit = Circuit::new(2);
/// let frozen = FrozenCircuit::new(circuit);
/// let gate = CircuitGate::new("Empty2Qubit", frozen).unwrap();
///
/// assert_eq!(gate.num_qubits(), 2);
/// assert_eq!(gate.name(), "Empty2Qubit");
/// ```
#[derive(Debug, Clone)]
pub struct CircuitGate {
    /// The name/label of this composite gate.
    pub name: Arc<String>,
    pub(crate) num_qubits: usize,
    pub(crate) num_params: usize,
    pub(crate) signature_params: IndexSet<String>,
    pub(crate) circuit: Arc<FrozenCircuit>,
}

impl PartialEq for CircuitGate {
    /// Compares two circuit gates by name, ordered signature, and defining circuit.
    ///
    /// The defining circuits compare with [`Circuit`]'s
    /// structural equality, which ignores process-local circuit identity. The
    /// frozen matrix cache never participates. Signature order is significant
    /// because call parameters are bound positionally.
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.num_qubits == other.num_qubits
            && self.num_params == other.num_params
            && self
                .signature_params
                .iter()
                .eq(other.signature_params.iter())
            && self.circuit.circuit() == other.circuit.circuit()
    }
}

impl CircuitGate {
    /// Creates a new circuit-based gate.
    ///
    /// Extracts the qubit count and parameter count from the frozen circuit.
    ///
    /// # Arguments
    ///
    /// * `name` - A descriptive name for the gate.
    /// * `circuit` - The frozen circuit defining the gate operation.
    ///
    /// # Returns
    ///
    /// - `Ok(CircuitGate)`: The new gate if successful.
    /// - `Err(CircuitError)`: If the circuit cannot be used as a gate.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::Circuit;
    /// use cqlib_core::circuit::gate::{CircuitGate, FrozenCircuit};
    ///
    /// let circuit = Circuit::new(2);
    /// let frozen = FrozenCircuit::new(circuit);
    /// let gate = CircuitGate::new("Bell", frozen).unwrap();
    ///
    /// assert_eq!(gate.num_qubits(), 2);
    /// ```
    pub fn new(name: impl Into<String>, circuit: FrozenCircuit) -> Result<Self, CircuitError> {
        let num_qubits = circuit.circuit.qubits().len();
        let signature_params = circuit.used_symbols().clone();
        let num_params = signature_params.len();

        Ok(Self {
            name: Arc::new(name.into()),
            num_qubits,
            num_params,
            signature_params,
            circuit: Arc::new(circuit),
        })
    }

    /// Creates a new circuit-based gate with an explicit call signature.
    ///
    /// This is useful for frontends whose gate signatures are declared separately
    /// from the symbols that happen to appear in the implementation. Declared but
    /// unused parameters remain part of the gate arity.
    pub fn with_signature(
        name: impl Into<String>,
        circuit: FrozenCircuit,
        params: impl IntoIterator<Item = String>,
    ) -> Result<Self, CircuitError> {
        let mut signature_params = IndexSet::new();
        for param in params {
            if !signature_params.insert(param.clone()) {
                return Err(CircuitError::InvalidOperation(format!(
                    "duplicate circuit gate parameter '{param}'"
                )));
            }
        }

        for symbol in circuit.used_symbols() {
            if !signature_params.contains(symbol) {
                return Err(CircuitError::InvalidOperation(format!(
                    "circuit gate implementation references undeclared parameter '{symbol}'"
                )));
            }
        }

        let num_qubits = circuit.circuit.qubits().len();
        let num_params = signature_params.len();

        Ok(Self {
            name: Arc::new(name.into()),
            num_qubits,
            num_params,
            signature_params,
            circuit: Arc::new(circuit),
        })
    }

    /// Returns the positional parameter signature used when invoking this gate.
    pub fn signature_params(&self) -> &IndexSet<String> {
        &self.signature_params
    }

    /// Returns the symbols actually referenced by this gate's backing circuit.
    pub fn used_symbols(&self) -> &IndexSet<String> {
        self.circuit.used_symbols()
    }

    /// Returns the set of symbolic parameter names used in the circuit.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::Circuit;
    /// use cqlib_core::circuit::gate::{CircuitGate, FrozenCircuit};
    ///
    /// let circuit = Circuit::new(1);
    /// let frozen = FrozenCircuit::new(circuit);
    /// let gate = CircuitGate::new("NoParams", frozen).unwrap();
    ///
    /// let symbols = gate.symbols();
    /// assert!(symbols.is_empty());
    /// ```
    pub fn symbols(&self) -> IndexSet<String> {
        self.used_symbols().clone()
    }

    /// Returns the number of qubits this gate acts on.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::Circuit;
    /// use cqlib_core::circuit::gate::{CircuitGate, FrozenCircuit};
    ///
    /// let circuit = Circuit::new(3);
    /// let frozen = FrozenCircuit::new(circuit);
    /// let gate = CircuitGate::new("ThreeQubitOp", frozen).unwrap();
    /// assert_eq!(gate.num_qubits(), 3);
    /// ```
    pub fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    /// Returns the number of parameters this gate accepts.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::Circuit;
    /// use cqlib_core::circuit::gate::{CircuitGate, FrozenCircuit};
    ///
    /// let circuit = Circuit::new(1);
    /// let frozen = FrozenCircuit::new(circuit);
    /// let gate = CircuitGate::new("NoParams", frozen).unwrap();
    ///
    /// assert_eq!(gate.num_params(), 0);
    /// ```
    pub fn num_params(&self) -> usize {
        self.num_params
    }

    /// Resolves a numeric gate-local matrix, with the first positional
    /// argument as the most-significant bit, like a standard gate matrix.
    /// This differs from `symbolic_matrix`'s storage-order, little-endian basis.
    pub(crate) fn matrix_for_params(
        &self,
        params: &[f64],
    ) -> Result<Array2<Complex64>, CircuitError> {
        if params.len() != self.signature_params.len() {
            return Err(CircuitError::ParameterCountMismatch {
                expected: self.signature_params.len(),
                actual: params.len(),
            });
        }
        for (index, &value) in params.iter().enumerate() {
            if !value.is_finite() {
                return Err(CircuitError::InvalidParameterValue(index, value));
            }
        }
        let bindings: HashMap<&str, f64> = self
            .signature_params
            .iter()
            .map(String::as_str)
            .zip(params.iter().copied())
            .collect();
        let assigned = self
            .circuit
            .circuit
            .assign_parameters(&Some(bindings))
            .map_err(|_| CircuitError::SymbolicParameterError)?;
        // circuit_to_matrix takes LSB-first ordering; gate arguments are MSB-first.
        let order: Vec<usize> = assigned.qubits().iter().rev().map(|q| q.index()).collect();
        circuit_to_matrix(&assigned, Some(&order))
    }

    /// Returns a clone of the internal frozen circuit.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::Circuit;
    /// use cqlib_core::circuit::gate::{CircuitGate, FrozenCircuit};
    ///
    /// let circuit = Circuit::new(2);
    /// let frozen = FrozenCircuit::new(circuit);
    /// let gate = CircuitGate::new("Test", frozen).unwrap();
    ///
    /// let inner = gate.circuit();
    /// assert_eq!(inner.circuit().qubits().len(), 2);
    /// ```
    pub fn circuit(&self) -> Arc<FrozenCircuit> {
        self.circuit.clone()
    }

    /// Returns the cached symbolic matrix in this gate's positional basis.
    ///
    /// The first qubit in the inner circuit's storage order is the
    /// least-significant matrix bit, matching the first application argument.
    pub fn symbolic_matrix(&self) -> Result<Arc<SymbolicMatrix>, CircuitError> {
        self.circuit.symbolic_matrix_in_storage_order()
    }

    /// Returns the name of this circuit gate.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::Circuit;
    /// use cqlib_core::circuit::gate::{CircuitGate, FrozenCircuit};
    ///
    /// let circuit = Circuit::new(2);
    /// let frozen = FrozenCircuit::new(circuit);
    /// let gate = CircuitGate::new("MyCustomGate", frozen).unwrap();
    /// assert_eq!(gate.name(), "MyCustomGate");
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Computes the inverse of this circuit gate.
    ///
    /// Creates a new `CircuitGate` with the circuit inverted and appends "_dg"
    /// to the name.
    ///
    /// # Returns
    ///
    /// - `Ok(CircuitGate)`: The inverted gate.
    /// - `Err(CircuitError)`: If the circuit cannot be inverted.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::Circuit;
    /// use cqlib_core::circuit::gate::{CircuitGate, FrozenCircuit};
    ///
    /// let circuit = Circuit::new(1);
    /// let frozen = FrozenCircuit::new(circuit);
    /// let gate = CircuitGate::new("Empty", frozen).unwrap();
    ///
    /// // Note: empty circuits are trivially invertible
    /// let inverse = gate.inverse().unwrap();
    /// assert_eq!(inverse.name(), "Empty_dg");
    /// ```
    pub fn inverse(&self) -> Result<Self, CircuitError> {
        let inverted_circuit = self.circuit.circuit.inverse()?;
        let frozen_inverted = FrozenCircuit::new(inverted_circuit);
        CircuitGate::with_signature(
            format!("{}_dg", self.name),
            frozen_inverted,
            self.signature_params.iter().cloned(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::{Circuit, Parameter, Qubit};

    #[test]
    fn numeric_matrix_binds_declared_signature_and_global_phase_without_mutation() {
        let mut circuit = Circuit::from_qubits(vec![Qubit::new(7)]).unwrap();
        let theta = Parameter::symbol("theta");
        let phi = Parameter::symbol("phi");
        circuit.rx(Qubit::new(7), theta.clone()).unwrap();
        circuit.rz(Qubit::new(7), phi.clone()).unwrap();
        circuit.set_global_phase(theta + phi / Parameter::from(2.0));
        let gate = CircuitGate::with_signature(
            "declared",
            FrozenCircuit::new(circuit),
            ["unused".to_string(), "phi".to_string(), "theta".to_string()],
        )
        .unwrap();
        let original = gate.clone();
        for (theta, phi) in [(0.37_f64, -0.21_f64), (-0.61, 0.42), (0.37, -0.21)] {
            let matrix = gate.matrix_for_params(&[12.0, phi, theta]).unwrap();
            for row in 0..2 {
                let phase = theta + phi / 2.0 + if row == 0 { -phi / 2.0 } else { phi / 2.0 };
                for col in 0..2 {
                    let rx = if row == col {
                        Complex64::new((theta / 2.0).cos(), 0.0)
                    } else {
                        Complex64::new(0.0, -(theta / 2.0).sin())
                    };
                    assert!(
                        (matrix[[row, col]] - Complex64::from_polar(1.0, phase) * rx).norm()
                            < 1e-12
                    );
                }
            }
            assert_eq!(gate, original);
            assert_eq!(gate.used_symbols().len(), 2);
        }
    }

    #[test]
    fn numeric_matrix_rejects_invalid_arguments_and_nonunitary_definitions() {
        let gate = CircuitGate::with_signature(
            "unused",
            FrozenCircuit::new(Circuit::new(1)),
            ["unused".to_string()],
        )
        .unwrap();
        for params in [vec![], vec![0.1, 0.2]] {
            assert!(matches!(
                gate.matrix_for_params(&params),
                Err(CircuitError::ParameterCountMismatch { expected: 1, .. })
            ));
        }
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                gate.matrix_for_params(&[value]),
                Err(CircuitError::InvalidParameterValue(0, _))
            ));
        }
        for directive in [
            crate::circuit::Directive::Measure,
            crate::circuit::Directive::Reset,
        ] {
            let mut body = Circuit::new(1);
            body.append(
                crate::circuit::Instruction::Directive(directive),
                [Qubit::new(0)],
                [],
                None,
            )
            .unwrap();
            let gate = CircuitGate::new("nonunitary", FrozenCircuit::new(body)).unwrap();
            assert!(matches!(
                gate.matrix_for_params(&[]),
                Err(CircuitError::NoMatrixRepresentation)
            ));
        }
    }

    #[test]
    fn signature_params_are_distinct_from_used_symbols() {
        let mut circuit = Circuit::new(1);
        circuit
            .rx(Qubit::new(0), Parameter::symbol("used"))
            .unwrap();

        let gate = CircuitGate::with_signature(
            "declared",
            FrozenCircuit::new(circuit),
            ["unused".to_string(), "used".to_string()],
        )
        .unwrap();

        assert_eq!(gate.num_params(), 2);
        assert_eq!(
            gate.signature_params().iter().cloned().collect::<Vec<_>>(),
            vec!["unused".to_string(), "used".to_string()]
        );
        assert!(gate.symbols().contains("used"));
        assert!(!gate.symbols().contains("unused"));
        assert_eq!(gate.used_symbols(), &gate.symbols());
    }

    #[test]
    fn explicit_signature_ignores_interned_but_unreferenced_symbols() {
        let mut circuit = Circuit::new(1);
        circuit.add_parameter(Parameter::symbol("stale"));
        circuit
            .rx(Qubit::new(0), Parameter::symbol("used"))
            .unwrap();

        let gate = CircuitGate::with_signature(
            "declared",
            FrozenCircuit::new(circuit),
            ["declared_unused".to_string(), "used".to_string()],
        )
        .unwrap();

        assert_eq!(
            gate.signature_params()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["declared_unused", "used"]
        );
        assert_eq!(
            gate.used_symbols()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["used"]
        );
    }

    #[test]
    fn explicit_signature_still_rejects_actually_used_undeclared_symbols() {
        let mut circuit = Circuit::new(1);
        circuit
            .rx(Qubit::new(0), Parameter::symbol("used"))
            .unwrap();

        let error = CircuitGate::with_signature(
            "missing",
            FrozenCircuit::new(circuit),
            ["different".to_string()],
        )
        .unwrap_err();

        assert!(
            matches!(error, CircuitError::InvalidOperation(message) if message.contains("used"))
        );
    }

    #[test]
    fn inverse_preserves_explicit_signature_and_live_dependencies() {
        let mut circuit = Circuit::new(1);
        circuit.add_parameter(Parameter::symbol("stale"));
        circuit
            .rx(Qubit::new(0), Parameter::symbol("used"))
            .unwrap();
        let gate = CircuitGate::with_signature(
            "forward",
            FrozenCircuit::new(circuit),
            ["declared_unused".to_string(), "used".to_string()],
        )
        .unwrap();

        let inverse = gate.inverse().unwrap();

        assert_eq!(inverse.signature_params(), gate.signature_params());
        assert_eq!(inverse.used_symbols(), gate.used_symbols());
    }

    #[test]
    fn frozen_circuits_compare_structurally_ignoring_matrix_cache() {
        let make_frozen = || {
            let mut circuit = Circuit::new(1);
            circuit.h(Qubit::new(0)).unwrap();
            FrozenCircuit::new(circuit)
        };

        let a = make_frozen();
        let b = make_frozen();
        assert_eq!(a, b);

        // Materializing the symbolic-matrix cache on one side must not
        // change equality.
        a.symbolic_matrix().unwrap();
        assert_eq!(a, b);

        let mut other = Circuit::new(1);
        other.x(Qubit::new(0)).unwrap();
        assert_ne!(a, FrozenCircuit::new(other));
    }
}
