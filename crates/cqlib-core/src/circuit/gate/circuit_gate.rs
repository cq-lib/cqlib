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
use crate::circuit::gate::{UnitaryGate, gate_matrix};
use crate::circuit::symbolic_matrix::{
    SymbolicComplex, SymbolicMatrix, circuit_to_symbolic_matrix,
};
use indexmap::IndexSet;
use ndarray::Array2;
use num_complex::Complex64;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

/// Maximum total qubit count for dense numeric matrix-backed controlled custom gates.
///
/// Capping numeric controlled unitaries at 12 qubits (`4096 x 4096`, ~256 MiB)
/// keeps dense matrix allocation bounded.
pub(crate) const MAX_CONTROLLED_UNITARY_QUBITS: usize = 12;

/// Maximum total qubit count for dense symbolic matrix-backed controlled custom gates.
///
/// Symbolic matrices allocate `4^n` [`SymbolicComplex`] entries (each holding two
/// expression trees), so they use a tighter cap (`256 x 256` = 65,536 entries).
pub(crate) const MAX_CONTROLLED_SYMBOLIC_UNITARY_QUBITS: usize = 8;

/// Validates the target qubit count, `u16` arity, and byte allocation layout
/// before building a controlled dense numeric or symbolic unitary matrix.
pub(crate) fn validate_controlled_unitary_qubits(
    base_qubits: usize,
    num_controls: usize,
    symbolic: bool,
) -> Result<u16, CircuitError> {
    let invalid = CircuitError::InvalidOperation;

    let total_qubits = base_qubits
        .checked_add(num_controls)
        .ok_or_else(|| invalid("controlled gate qubit count overflows usize".to_string()))?;
    let total_qubits_u16 = u16::try_from(total_qubits).map_err(|_| {
        invalid(format!(
            "controlled gate qubit count {total_qubits} exceeds u16::MAX"
        ))
    })?;
    let max_qubits = if symbolic {
        MAX_CONTROLLED_SYMBOLIC_UNITARY_QUBITS
    } else {
        MAX_CONTROLLED_UNITARY_QUBITS
    };
    if total_qubits > max_qubits {
        return Err(invalid(format!(
            "controlled gate requires {total_qubits} qubits, exceeding maximum dense unitary limit of {max_qubits}"
        )));
    }
    let exponent = u32::try_from(total_qubits).map_err(|_| {
        invalid(format!(
            "cannot build matrix for {total_qubits} qubits: exponent overflows u32"
        ))
    })?;
    let dim = 1usize.checked_shl(exponent).ok_or_else(|| {
        invalid(format!(
            "cannot build matrix for {total_qubits} qubits: dimension overflows usize"
        ))
    })?;
    let elems = dim.checked_mul(dim).ok_or_else(|| {
        invalid(format!(
            "cannot build matrix for {total_qubits} qubits: element count overflows usize"
        ))
    })?;
    if symbolic {
        std::alloc::Layout::array::<SymbolicComplex>(elems).map_err(|_| {
            invalid(format!(
                "cannot allocate symbolic matrix for {total_qubits} qubits"
            ))
        })?;
    } else {
        std::alloc::Layout::array::<Complex64>(elems).map_err(|_| {
            invalid(format!(
                "cannot allocate numeric matrix for {total_qubits} qubits"
            ))
        })?;
    }
    Ok(total_qubits_u16)
}

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
        if self.signature_params.is_empty() {
            // circuit_to_matrix takes LSB-first ordering; gate arguments are MSB-first.
            let order: Vec<usize> = self
                .circuit
                .circuit
                .qubits()
                .iter()
                .rev()
                .map(|q| q.index())
                .collect();
            return circuit_to_matrix(&self.circuit.circuit, Some(&order));
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
            .map_err(|err| match err {
                CircuitError::InvalidOperation(_) => err,
                _ => CircuitError::SymbolicParameterError,
            })?;
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

    /// Wraps this composite gate as a controlled [`UnitaryGate`] with `num_controls`
    /// active-high (`|1⟩`) control qubits prepended before the target qubits.
    ///
    /// When applied to a circuit, pass the `num_controls` control qubits first,
    /// followed by the `self.num_qubits()` target qubits (mapped positionally in
    /// the inner circuit's storage order). All declared positional parameters
    /// (`signature_params`) and the inner circuit's global phase (converted into
    /// a relative phase on the all-ones control subspace) are preserved.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::InvalidOperation`] when `num_controls == 0` or the
    /// resulting dense matrix exceeds resource limits, or
    /// [`CircuitError::NoMatrixRepresentation`] when the inner circuit contains
    /// non-unitary operations.
    pub fn control(&self, num_controls: usize) -> Result<UnitaryGate, CircuitError> {
        if num_controls == 0 {
            return Err(CircuitError::InvalidOperation(
                "num_controls must be at least 1".to_string(),
            ));
        }
        let num_params = u16::try_from(self.num_params).map_err(|_| {
            CircuitError::InvalidOperation(format!(
                "circuit gate parameter count {} exceeds u16::MAX",
                self.num_params
            ))
        })?;
        let symbolic = self.num_params > 0;
        let total_qubits =
            validate_controlled_unitary_qubits(self.num_qubits, num_controls, symbolic)?;
        let label = format!("ctl_{}_{}", num_controls, self.name);

        if !symbolic {
            let base = self.matrix_for_params(&[])?;
            UnitaryGate::validate_numeric_unitary_matrix(&base)?;
            let controlled = gate_matrix::control_matrix(&base, num_controls);
            UnitaryGate::new(&label, total_qubits, 0).with_validated_matrix(controlled)
        } else {
            validate_no_reserved_constant_params(self)?;
            // UnitaryGate symbolic matrices put the first positional argument at MSB,
            // whereas circuit_to_symbolic_matrix takes LSB-first qubit ordering.
            let msb_order: Vec<usize> = self
                .circuit
                .circuit()
                .qubits()
                .iter()
                .rev()
                .map(|q| q.index())
                .collect();
            let base = circuit_to_symbolic_matrix(self.circuit.circuit(), Some(&msb_order))?;
            let controlled =
                crate::circuit::symbolic_matrix::gate::control_matrix(&base, num_controls);
            UnitaryGate::new(&label, total_qubits, num_params)
                .with_symbolic_matrix(self.signature_params.iter().cloned(), controlled)
        }
    }
}

fn validate_no_reserved_constant_params(gate: &CircuitGate) -> Result<(), CircuitError> {
    for param in &gate.signature_params {
        if matches!(param.as_str(), "e" | "pi" | "π") {
            return Err(CircuitError::InvalidOperation(format!(
                "controlled circuit gate parameter '{param}' conflicts with built-in mathematical constant"
            )));
        }
    }
    for op in gate.circuit.circuit().operations() {
        if let crate::circuit::Instruction::CircuitGate(inner) = &op.instruction {
            validate_no_reserved_constant_params(inner)?;
        }
    }
    Ok(())
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

    #[test]
    fn control_bell_single_and_double_controls_match_block_unitary_and_targets() {
        let mut bell = Circuit::from_qubits(vec![Qubit::new(7), Qubit::new(2)]).unwrap();
        bell.h(Qubit::new(7)).unwrap();
        bell.cx(Qubit::new(7), Qubit::new(2)).unwrap();
        bell.set_global_phase(Parameter::from(0.31));
        let bell_gate = CircuitGate::new("Bell", FrozenCircuit::new(bell)).unwrap();

        let s2 = 1.0 / 2.0_f64.sqrt();
        let phase = Complex64::from_polar(1.0, 0.31);
        // MSB-first Bell unitary on |t0 t1>: H(t0) followed by CX(t0, t1).
        let bell_mat = ndarray::array![
            [
                phase * s2,
                Complex64::default(),
                phase * s2,
                Complex64::default()
            ],
            [
                Complex64::default(),
                phase * s2,
                Complex64::default(),
                phase * s2
            ],
            [
                Complex64::default(),
                phase * s2,
                Complex64::default(),
                -phase * s2
            ],
            [
                phase * s2,
                Complex64::default(),
                -phase * s2,
                Complex64::default()
            ],
        ];

        for num_ctrls in [1usize, 2] {
            let controlled = bell_gate.control(num_ctrls).unwrap();
            assert_eq!(controlled.label(), format!("ctl_{num_ctrls}_Bell"));
            assert_eq!(controlled.num_qubits(), (2 + num_ctrls) as u16);
            assert_eq!(controlled.num_params(), 0);
            assert!(controlled.circuit().is_none());

            let mat = controlled.matrix().unwrap();
            let dim = 1usize << (2 + num_ctrls);
            assert_eq!(mat.shape(), &[dim, dim]);
            let active_start = dim - 4;
            for r in 0..dim {
                for c in 0..dim {
                    let expected = if r >= active_start && c >= active_start {
                        bell_mat[[r - active_start, c - active_start]]
                    } else if r == c {
                        Complex64::new(1.0, 0.0)
                    } else {
                        Complex64::default()
                    };
                    assert!((mat[[r, c]] - expected).norm() < 1e-12);
                }
            }
        }

        // Apply doubly-controlled Bell on out-of-order, non-contiguous qubits [3, 1, 4, 0] in a 5-qubit circuit.
        let cc_bell = bell_gate.control(2).unwrap();
        let mut circuit = Circuit::new(5);
        circuit
            .unitary(
                cc_bell,
                vec![Qubit::new(3), Qubit::new(1), Qubit::new(4), Qubit::new(0)],
            )
            .unwrap();
        let full = circuit_to_matrix(&circuit, None).unwrap();

        for col in 0..32usize {
            let c0 = (col >> 3) & 1;
            let c1 = (col >> 1) & 1;
            let t0 = (col >> 4) & 1;
            let t1 = col & 1;
            let local_col = (t0 << 1) | t1;

            let mut expected_col = vec![Complex64::default(); 32];
            if c0 == 1 && c1 == 1 {
                for local_row in 0..4usize {
                    let nr_t0 = (local_row >> 1) & 1;
                    let nr_t1 = local_row & 1;
                    let row = (col & !((1 << 4) | 1)) | (nr_t0 << 4) | nr_t1;
                    expected_col[row] = bell_mat[[local_row, local_col]];
                }
            } else {
                expected_col[col] = Complex64::new(1.0, 0.0);
            }
            for row in 0..32usize {
                assert!(
                    (full[[row, col]] - expected_col[row]).norm() < 1e-12,
                    "mismatch at ({row}, {col}): got {:?}, expected {:?}",
                    full[[row, col]],
                    expected_col[row]
                );
            }
        }
    }

    #[test]
    fn control_parameterized_preserves_signature_nested_swaps_and_internal_prefix() {
        use crate::circuit::ParameterValue;
        use crate::circuit::symbolic_matrix::evaluate_symbolic_matrix;
        use crate::circuit::test_utils::assert_matrix_approx_eq;

        let alpha = Parameter::symbol("alpha");
        let beta = Parameter::symbol("__cqlib_internal_sub_beta");
        let mut inner = Circuit::from_qubits(vec![Qubit::new(9), Qubit::new(3)]).unwrap();
        inner
            .rx(Qubit::new(9), alpha.clone() + beta.clone())
            .unwrap();
        inner.cx(Qubit::new(9), Qubit::new(3)).unwrap();
        inner.set_global_phase(alpha * 0.5 - beta);

        let gate = CircuitGate::with_signature(
            "ParamOp",
            FrozenCircuit::new(inner),
            [
                "unused".to_string(),
                "__cqlib_internal_sub_beta".to_string(),
                "alpha".to_string(),
            ],
        )
        .unwrap();

        let ctrl_gate = gate.control(1).unwrap();
        assert_eq!(ctrl_gate.num_qubits(), 3);
        assert_eq!(ctrl_gate.num_params(), 3);
        assert_eq!(
            ctrl_gate.matrix_params().unwrap(),
            ["unused", "__cqlib_internal_sub_beta", "alpha"]
        );

        // Nest inside an outer circuit with swapped/same-name/expression symbolic arguments.
        let mut outer = Circuit::new(3);
        outer
            .unitary_with_params(
                ctrl_gate,
                vec![Qubit::new(2), Qubit::new(0), Qubit::new(1)],
                [
                    ParameterValue::from(Parameter::symbol("alpha") + Parameter::from(5.0)),
                    ParameterValue::from(Parameter::symbol("alpha")),
                    ParameterValue::from(
                        Parameter::symbol("__cqlib_internal_sub_beta") * Parameter::from(2.0),
                    ),
                ],
            )
            .unwrap();

        let symbolic = circuit_to_symbolic_matrix(&outer, None).unwrap();
        let bindings = HashMap::from([
            ("alpha", 0.41_f64),
            ("__cqlib_internal_sub_beta", -0.27_f64),
        ]);
        let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();
        let bound = outer.assign_parameters(&Some(bindings)).unwrap();
        let numeric = circuit_to_matrix(&bound, None).unwrap();
        assert_matrix_approx_eq(&evaluated, &numeric, 1e-12);

        // Compare against explicit controlled reference: alpha_inner = 2*(-0.27) = -0.54, beta_inner = 0.41.
        let a_val = -0.54_f64;
        let b_val = 0.41_f64;
        let angle = a_val + b_val;
        let rel_phase = Complex64::from_polar(1.0, a_val * 0.5 - b_val);
        let c = Complex64::new((angle / 2.0).cos(), 0.0);
        let s = Complex64::new(0.0, -(angle / 2.0).sin());
        // Control is q2 (bit 2). When bit 2 == 0 (indices 0..4), must be identity;
        // when bit 2 == 1 (indices 4..8): RX(angle) on q0 (bit 0) then CX(q0, q1) * rel_phase.
        for col in 0..4usize {
            for row in 0..8usize {
                let expected = if row == col {
                    Complex64::new(1.0, 0.0)
                } else {
                    Complex64::default()
                };
                assert!((numeric[[row, col]] - expected).norm() < 1e-12);
            }
        }
        // Col 4 (|q2=1, q1=0, q0=0>): RX(q0) -> c|100> + s|101>, CX(q0->q1) -> c|100> + s|111>
        assert!((numeric[[4, 4]] - rel_phase * c).norm() < 1e-12);
        assert!((numeric[[7, 4]] - rel_phase * s).norm() < 1e-12);
    }

    #[test]
    fn control_zero_qubit_phase_circuit_produces_controlled_phase_gate() {
        let mut phase_only = Circuit::new(0);
        phase_only.set_global_phase(Parameter::symbol("theta"));
        let gate = CircuitGate::new("PhaseOnly", FrozenCircuit::new(phase_only)).unwrap();

        let ctrl = gate.control(2).unwrap();
        assert_eq!(ctrl.num_qubits(), 2);
        assert_eq!(ctrl.num_params(), 1);
        let mat = ctrl.matrix_for_params(&[0.73]).unwrap();
        for r in 0..4 {
            for c in 0..4 {
                let expected = if r != c {
                    Complex64::default()
                } else if r == 3 {
                    Complex64::from_polar(1.0, 0.73)
                } else {
                    Complex64::new(1.0, 0.0)
                };
                assert!((mat[[r, c]] - expected).norm() < 1e-12);
            }
        }
    }

    #[test]
    fn control_rejects_zero_controls_oversized_limits_and_nonunitary_bodies() {
        let simple = CircuitGate::new("OneQ", FrozenCircuit::new(Circuit::new(1))).unwrap();
        assert!(matches!(
            simple.control(0),
            Err(CircuitError::InvalidOperation(_))
        ));
        assert!(matches!(
            simple.control(MAX_CONTROLLED_UNITARY_QUBITS),
            Err(CircuitError::InvalidOperation(_))
        ));
        assert!(matches!(
            simple.control(30),
            Err(CircuitError::InvalidOperation(_))
        ));
        assert!(matches!(
            simple.control(usize::MAX),
            Err(CircuitError::InvalidOperation(_))
        ));

        let mut meas = Circuit::new(1);
        meas.append(
            crate::circuit::Instruction::Directive(crate::circuit::Directive::Measure),
            [Qubit::new(0)],
            [],
            None,
        )
        .unwrap();
        let nonunitary = CircuitGate::new("Meas", FrozenCircuit::new(meas)).unwrap();
        assert!(matches!(
            nonunitary.control(1),
            Err(CircuitError::NoMatrixRepresentation)
        ));

        // Symbolic controlled gates enforce the tighter symbolic qubit cap (8 qubits).
        let mut sym_one_q = Circuit::new(1);
        sym_one_q
            .rx(Qubit::new(0), Parameter::symbol("theta"))
            .unwrap();
        let sym_gate = CircuitGate::new("SymOneQ", FrozenCircuit::new(sym_one_q)).unwrap();
        assert!(matches!(
            sym_gate.control(MAX_CONTROLLED_SYMBOLIC_UNITARY_QUBITS),
            Err(CircuitError::InvalidOperation(_))
        ));

        // Classical control flow reports InvalidOperation consistently for both numeric and symbolic gates.
        let mut cond_num = Circuit::new(1);
        cond_num
            .if_else(
                crate::circuit::ClassicalExpr::bool_literal(false),
                |body| {
                    body.x(Qubit::new(0))?;
                    Ok(())
                },
                |body| {
                    body.z(Qubit::new(0))?;
                    Ok(())
                },
            )
            .unwrap();
        let cond_num_gate = CircuitGate::new("CondNum", FrozenCircuit::new(cond_num)).unwrap();
        match cond_num_gate.control(1) {
            Err(CircuitError::InvalidOperation(msg)) => {
                assert!(msg.contains("control-flow"), "unexpected message: {msg}");
            }
            other => panic!("expected InvalidOperation for numeric if_else, got {other:?}"),
        }
    }

    #[test]
    fn control_rejects_constant_named_formal_parameters() {
        use crate::circuit::gate::StandardGate;
        use crate::circuit::{Instruction, ParameterValue};

        for name in ["e", "pi", "π"] {
            let mut sub = Circuit::new(1);
            sub.append(
                Instruction::Standard(StandardGate::RX),
                [Qubit::new(0)],
                [ParameterValue::Param(Parameter::symbol(name))],
                None,
            )
            .unwrap();
            let gate = CircuitGate::with_signature(
                "ConstNamed",
                FrozenCircuit::new(sub),
                vec![name.to_string()],
            )
            .unwrap();
            assert!(
                matches!(gate.control(1), Err(CircuitError::InvalidOperation(_))),
                "expected InvalidOperation for reserved constant parameter name '{name}'"
            );

            // Nested inner gate declaring a reserved constant parameter must also be rejected.
            let mut outer = Circuit::new(1);
            outer
                .append(
                    Instruction::CircuitGate(Box::new(gate)),
                    [Qubit::new(0)],
                    [ParameterValue::Param(Parameter::symbol("theta"))],
                    None,
                )
                .unwrap();
            let outer_gate = CircuitGate::new("OuterWrap", FrozenCircuit::new(outer)).unwrap();
            assert!(
                matches!(
                    outer_gate.control(1),
                    Err(CircuitError::InvalidOperation(_))
                ),
                "expected InvalidOperation for nested reserved constant parameter name '{name}'"
            );
        }

        // Degenerate numeric base matrix (U(pi/2, 1e16, 1.0)) must still be rejected.
        let mut degenerate = Circuit::new(1);
        degenerate
            .u(Qubit::new(0), std::f64::consts::FRAC_PI_2, 1e16, 1.0)
            .unwrap();
        let degenerate_gate =
            CircuitGate::new("DegenerateU", FrozenCircuit::new(degenerate)).unwrap();
        assert!(matches!(
            degenerate_gate.control(1),
            Err(CircuitError::InvalidOperation(_))
        ));
    }

    #[test]
    fn control_preserves_column_major_symbolic_unitary_inside_composite_gate() {
        use crate::circuit::ParameterValue;
        use ndarray::ShapeBuilder;

        let theta = Parameter::symbol("theta");
        // Column-major 1-qubit unitary M = [[0, 1], [exp(i*theta), 0]].
        let col_major = Array2::from_shape_vec(
            (2, 2).f(),
            vec![
                SymbolicComplex::zero(),
                SymbolicComplex::exp_i(theta.clone()),
                SymbolicComplex::one(),
                SymbolicComplex::zero(),
            ],
        )
        .unwrap();
        assert!(!col_major.is_standard_layout());

        let inner_u = UnitaryGate::new("ColMajorPhaseFlip", 1, 1)
            .with_symbolic_matrix(["theta"], col_major)
            .unwrap();
        let mut sub = Circuit::new(1);
        sub.unitary_with_params(inner_u, vec![Qubit::new(0)], [ParameterValue::from(theta)])
            .unwrap();

        let composite = CircuitGate::new("WrapColMajor", FrozenCircuit::new(sub)).unwrap();
        let controlled = composite.control(1).unwrap();
        let mat = controlled.matrix_for_params(&[0.5]).unwrap();

        // Active 2x2 block at rows/cols 2..4 must be M, NOT M^T:
        // mat[2, 3] == 1, mat[3, 2] == exp(0.5i).
        assert!((mat[[0, 0]] - Complex64::new(1.0, 0.0)).norm() < 1e-12);
        assert!((mat[[1, 1]] - Complex64::new(1.0, 0.0)).norm() < 1e-12);
        assert!((mat[[2, 2]] - Complex64::default()).norm() < 1e-12);
        assert!((mat[[2, 3]] - Complex64::new(1.0, 0.0)).norm() < 1e-12);
        assert!((mat[[3, 2]] - Complex64::from_polar(1.0, 0.5)).norm() < 1e-12);
        assert!((mat[[3, 3]] - Complex64::default()).norm() < 1e-12);
    }
}
