// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Density Matrix quantum simulation.
//!
//! Provides a high-performance density matrix simulator capable of simulating mixed
//! quantum states and quantum channels (e.g., via Kraus operators).
//!
//! # Architecture
//! The simulator utilizes a $2N$-qubit isomorphism (a variant of the Choi-Jamiołkowski isomorphism)
//! to map the $2^N \times 2^N$ density matrix into a statevector of size $4^N$:
//! - **Data Representation**: The density matrix is flattened into a 1D vector of length $4^N$.
//! - **Ket Side (Left Multiply)**: Applying a unitary $U$ acts on the upper half of the conceptual $2N$ qubits (indices $N$ to $2N-1$).
//! - **Bra Side (Right Multiply)**: Applying $U^\dagger$ acts on the lower half of the conceptual $2N$ qubits (indices 0 to $N-1$), applying the conjugate matrix $U^*$.
//!
//! This design allows the simulator to directly reuse highly-optimized statevector and bitwise memory kernels,
//! achieving exceptional parallel performance and cache locality.

use crate::circuit::Measurement;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::circuit_param::CircuitParam;
use crate::circuit::error::CircuitError;
use crate::circuit::gate::StandardGate;
use crate::circuit::gate::directive::Directive;
use crate::circuit::gate::instruction::Instruction;
use crate::device::{ExecutionResult, Outcome};
use crate::qis::Observable;
use crate::qis::error::QisError;
use num_complex::Complex64;
use rayon::prelude::*;
use smallvec::{SmallVec, smallvec};
use std::collections::{HashMap, HashSet};
use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, FRAC_PI_4};

const PARALLEL_THRESHOLD: usize = 7;

macro_rules! with_maybe_par {
    ($num_qubits:expr, $data:expr, $chunk_size:expr, $body:expr) => {{
        if $num_qubits < PARALLEL_THRESHOLD {
            $data.chunks_exact_mut($chunk_size).for_each($body);
        } else {
            $data.par_chunks_exact_mut($chunk_size).for_each($body);
        }
    }};
}

#[inline(always)]
fn interleave_bits(keep: &[usize], k_val: usize, trace: &[usize], t_val: usize) -> usize {
    let mut res = 0;
    for (idx, &q) in keep.iter().enumerate() {
        if (k_val >> idx) & 1 == 1 {
            res |= 1 << q;
        }
    }
    for (idx, &q) in trace.iter().enumerate() {
        if (t_val >> idx) & 1 == 1 {
            res |= 1 << q;
        }
    }
    res
}

/// Quantum density matrix representing mixed or pure quantum states.
///
/// A density matrix describes the statistical state of an N-qubit quantum system.
/// Unlike a statevector which can only represent pure states, a density matrix
/// can represent mixed states (ensembles of pure states).
///
/// # Memory Layout
/// The `data` vector uses a contiguous memory layout representing a flattened
/// $2^N \times 2^N$ matrix. To optimize simulation performance, the simulator
/// employs a $2N$-qubit isomorphism:
/// - The matrix is treated as a statevector of $2N$ qubits.
/// - The "ket" side (Left U) acts on the upper $N$ qubits (indices $N$ to $2N-1$).
/// - The "bra" side (Right $U^\dagger$) acts on the lower $N$ qubits (indices 0 to $N-1$).
///
/// # Example
/// ```rust
/// use cqlib_core::qis::DensityMatrix;
///
/// // Create a 1-qubit density matrix in state |0><0|
/// let mut dm = DensityMatrix::new(1);
/// assert_eq!(dm.num_qubits, 1);
///
/// // Apply Hadamard gate -> |+><+|
/// dm.apply_h(0);
///
/// // Probabilities should be 0.5 for both |0> and |1>
/// let probs = dm.probabilities();
/// assert!((probs[0] - 0.5).abs() < 1e-10);
/// assert!((probs[1] - 0.5).abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub struct DensityMatrix {
    /// Flattened matrix elements. Length is $4^N$.
    data: Vec<Complex64>,
    /// Number of qubits in the system ($N$).
    pub num_qubits: usize,
}

impl std::ops::AddAssign for DensityMatrix {
    fn add_assign(&mut self, rhs: Self) {
        assert_eq!(self.num_qubits, rhs.num_qubits);
        if self.num_qubits < PARALLEL_THRESHOLD {
            self.data
                .iter_mut()
                .zip(rhs.data)
                .for_each(|(a, b)| *a += b);
        } else {
            self.data
                .par_iter_mut()
                .zip(rhs.data.into_par_iter())
                .for_each(|(a, b)| *a += b);
        }
    }
}

impl DensityMatrix {
    /// Returns the flattened row-major density matrix data.
    pub fn data(&self) -> &[Complex64] {
        &self.data
    }

    /// Validates that a single qubit index is within bounds.
    #[inline]
    fn validate_qubit(&self, qubit: usize) -> Result<(), QisError> {
        if self.num_qubits == 0 {
            return Err(QisError::InvalidParameterValue(
                "no valid qubit indices in a zero-qubit system".to_string(),
            ));
        }
        if qubit >= self.num_qubits {
            return Err(QisError::IndexOutOfBounds {
                index: qubit,
                max: self.num_qubits.saturating_sub(1),
            });
        }
        Ok(())
    }

    /// Validates that two qubit indices are within bounds.
    #[inline]
    fn validate_two_qubits(&self, q0: usize, q1: usize) -> Result<(), QisError> {
        if self.num_qubits == 0 {
            return Err(QisError::InvalidParameterValue(
                "no valid qubit indices in a zero-qubit system".to_string(),
            ));
        }
        if q0 >= self.num_qubits {
            return Err(QisError::IndexOutOfBounds {
                index: q0,
                max: self.num_qubits.saturating_sub(1),
            });
        }
        if q1 >= self.num_qubits {
            return Err(QisError::IndexOutOfBounds {
                index: q1,
                max: self.num_qubits.saturating_sub(1),
            });
        }
        Ok(())
    }

    /// Validates that all qubit indices in a slice are within bounds.
    #[inline]
    fn validate_qubits(&self, qubits: &[usize]) -> Result<(), QisError> {
        if self.num_qubits == 0 {
            return Err(QisError::InvalidParameterValue(
                "no valid qubit indices in a zero-qubit system".to_string(),
            ));
        }
        for &q in qubits {
            if q >= self.num_qubits {
                return Err(QisError::IndexOutOfBounds {
                    index: q,
                    max: self.num_qubits.saturating_sub(1),
                });
            }
        }
        Ok(())
    }

    /// Creates a new density matrix initialized to the pure state $|0\dots 0\rangle\langle 0\dots 0|$.
    ///
    /// # Arguments
    /// * `num_qubits` - Number of qubits in the system.
    pub fn new(num_qubits: usize) -> Self {
        let size = 1 << (2 * num_qubits);
        let mut data = vec![Complex64::new(0.0, 0.0); size];
        if size > 0 {
            data[0] = Complex64::new(1.0, 0.0);
        }
        Self { data, num_qubits }
    }

    /// Creates the maximally mixed state $I / 2^N$.
    ///
    /// The returned density matrix has equal probability on every
    /// computational-basis state and zero off-diagonal entries. A zero-qubit
    /// system is represented by the one-dimensional density matrix `[1]`.
    ///
    /// Like every dense density-matrix constructor, this allocates $O(4^N)$
    /// complex values.
    pub fn maximally_mixed(num_qubits: usize) -> Self {
        let dim = 1 << num_qubits;
        let mut data = vec![Complex64::new(0.0, 0.0); dim * dim];
        let probability = Complex64::new(1.0 / dim as f64, 0.0);
        for index in 0..dim {
            data[index * dim + index] = probability;
        }
        Self { data, num_qubits }
    }

    /// Creates a density matrix from an initial statevector (pure state).
    ///
    /// Internally computes the outer product $\rho = |\psi\rangle\langle\psi|$.
    ///
    /// # Arguments
    /// * `num_qubits` - Number of qubits in the system.
    /// * `initial_state` - Vector of $2^N$ complex amplitudes representing a normalized pure state.
    ///
    /// # Errors
    /// Returns [`QisError::InvalidStateDimension`] if the `initial_state` length is
    /// incorrect, or [`QisError::NotNormalized`] if it is not normalized.
    pub fn from_state(num_qubits: usize, initial_state: Vec<Complex64>) -> Result<Self, QisError> {
        let dim = 1 << num_qubits;
        if initial_state.len() != dim {
            return Err(QisError::InvalidStateDimension(initial_state.len()));
        }
        let norm: f64 = initial_state.iter().map(|c| c.norm_sqr()).sum();
        if (norm - 1.0).abs() >= 1e-10 {
            return Err(QisError::NotNormalized);
        }

        let size = 1 << (2 * num_qubits);
        let mut data = vec![Complex64::new(0.0, 0.0); size];

        let kernel = |(i, chunk): (usize, &mut [Complex64])| {
            let alpha_i = initial_state[i];
            for j in 0..dim {
                chunk[j] = alpha_i * initial_state[j].conj();
            }
        };

        if num_qubits < PARALLEL_THRESHOLD {
            data.chunks_exact_mut(dim).enumerate().for_each(kernel);
        } else {
            data.par_chunks_exact_mut(dim).enumerate().for_each(kernel);
        }

        Ok(Self { data, num_qubits })
    }

    /// Creates a density matrix directly from a flattened $2^N \times 2^N$ matrix.
    ///
    /// Validates all physical constraints: Hermiticity, positive semidefiniteness, and unit trace.
    ///
    /// # Arguments
    /// * `num_qubits` - Number of qubits in the system.
    /// * `dm_state` - Vector of $4^N$ complex values representing the density matrix.
    ///
    /// # Errors
    /// Returns `QisError::InvalidStateDimension` if the matrix length is incorrect.
    /// Returns `QisError::NotHermitian` if the matrix is not Hermitian.
    /// Returns `QisError::NotPositiveSemidefinite` if the matrix has negative eigenvalues.
    /// Returns `QisError::NotNormalized` if the trace is not equal to 1.
    pub fn from_density_matrix_state(
        num_qubits: usize,
        dm_state: Vec<Complex64>,
    ) -> Result<Self, QisError> {
        let size = 1 << (2 * num_qubits);
        if dm_state.len() != size {
            return Err(QisError::InvalidStateDimension(dm_state.len()));
        }

        let dm = Self {
            data: dm_state,
            num_qubits,
        };
        dm.validate_physical(1e-10)?;
        Ok(dm)
    }

    /// Computes the measurement probability distribution over all computational basis states.
    ///
    /// Extracts the diagonal elements of the density matrix, which represent
    /// the probabilities $P(|i\rangle) = \rho_{ii}$.
    ///
    /// # Returns
    /// Vector of probabilities summing to 1.0.
    pub fn probabilities(&self) -> Vec<f64> {
        let dim = 1 << self.num_qubits;
        if self.num_qubits < PARALLEL_THRESHOLD {
            (0..dim).map(|i| self.data[i * dim + i].re).collect()
        } else {
            (0..dim)
                .into_par_iter()
                .map(|i| self.data[i * dim + i].re)
                .collect()
        }
    }

    /// Constructs a density matrix by simulating a quantum circuit.
    ///
    /// Executes the circuit gates sequentially to evolve the initial $|0\dots 0\rangle\langle 0\dots 0|$ state.
    ///
    /// # Arguments
    /// * `circuit` - The quantum circuit to simulate.
    ///
    /// # Returns
    /// * `Ok(DensityMatrix)` - The resulting density matrix after execution.
    /// * `Err(QisError)` - If the circuit contains unsupported operations.
    pub fn from_circuit(circuit: &Circuit) -> Result<Self, QisError> {
        let mut dm = DensityMatrix::new(circuit.num_qubits());
        dm.apply_circuit(circuit)?;
        Ok(dm)
    }

    /// Applies a quantum circuit to this density matrix in-place.
    ///
    /// The circuit is first decomposed into basic gates via [`Circuit::decompose`],
    /// then each operation is applied sequentially to the current density matrix.
    ///
    /// # Arguments
    /// * `circuit` - The circuit to apply. Must have the same number of qubits
    ///   as this density matrix.
    ///
    /// # Returns
    /// * `Ok(())` - If all operations were applied successfully
    /// * `Err(QisError)` - If the circuit cannot be simulated
    ///
    /// # Errors
    /// - [`QisError::InvalidStateDimension`] if `circuit.num_qubits() != self.num_qubits`
    /// - [`QisError::UnsupportedOperation`] if the circuit contains classical control flow
    /// - [`QisError::CircuitError`] if a gate lacks a matrix representation, contains
    ///   unresolved symbolic parameters, or a referenced qubit is not found
    ///
    /// # Supported Instructions
    /// - Standard single and multi-qubit gates
    /// - Controlled gates (CX, CY, CZ, CRX, CRY, CRZ)
    /// - Multi-controlled gates (CCX / Toffoli)
    /// - Unitary gates with an explicit matrix representation
    /// - `Reset`
    /// - Barriers and delays (ignored)
    /// - Measurement output declarations from `Circuit::measure*` (ignored)
    pub fn apply_circuit(&mut self, circuit: &Circuit) -> Result<(), QisError> {
        if self.num_qubits != circuit.num_qubits() {
            return Err(QisError::InvalidStateDimension(circuit.num_qubits()));
        }
        if circuit
            .operations()
            .iter()
            .any(|op| matches!(op.instruction, Instruction::ClassicalControl(_)))
        {
            return Err(QisError::UnsupportedOperation(
                "classical control flow is not supported in density matrix simulation".to_string(),
            ));
        }
        let circuit = circuit.decompose()?;
        let dm = self;

        let qubits = circuit.qubits();
        let qubit_map: std::collections::HashMap<_, _> = qubits
            .iter()
            .enumerate()
            .map(|(idx, q)| (*q, idx))
            .collect();

        let parameter_values: Vec<Option<f64>> = circuit
            .parameters()
            .iter()
            .map(|p| p.evaluate(&None).ok())
            .collect();

        for op in circuit.operations() {
            let params: Vec<f64> = op
                .params
                .iter()
                .map(|p| match p {
                    CircuitParam::Fixed(v) => Ok(*v),
                    CircuitParam::Index(idx) => parameter_values
                        .get(*idx as usize)
                        .copied()
                        .flatten()
                        .ok_or(QisError::CircuitError(CircuitError::SymbolicParameterError)),
                })
                .collect::<Result<Vec<_>, QisError>>()?;

            let qubit_indices: Result<Vec<usize>, QisError> =
                op.qubits
                    .iter()
                    .map(|q| {
                        qubit_map.get(q).copied().ok_or_else(|| {
                            QisError::CircuitError(CircuitError::QubitNotFound(q.id()))
                        })
                    })
                    .collect();
            let qubit_indices = qubit_indices?;

            match &op.instruction {
                Instruction::Standard(gate) => {
                    dm.apply_standard_gate(*gate, &qubit_indices, &params)?;
                }
                Instruction::McGate(mc_gate) => {
                    let num_controls = mc_gate.num_ctrl_qubits();
                    let base_gate = mc_gate.base_gate();

                    if num_controls == 1 {
                        let control = qubit_indices[0];
                        let target = qubit_indices[1];
                        match base_gate {
                            StandardGate::X => dm.apply_cx(control, target)?,
                            StandardGate::Y => dm.apply_cy(control, target)?,
                            StandardGate::Z => dm.apply_cz(control, target)?,
                            StandardGate::RX => dm.apply_crx(control, target, params[0])?,
                            StandardGate::RY => dm.apply_cry(control, target, params[0])?,
                            StandardGate::RZ => dm.apply_crz(control, target, params[0])?,
                            _ => {
                                let matrix = mc_gate.matrix(&params).map_err(|_| {
                                    QisError::CircuitError(CircuitError::NoMatrixRepresentation)
                                })?;
                                dm.apply_unitary_gate(&qubit_indices, &matrix)?;
                            }
                        }
                    } else if num_controls == 2 && *base_gate == StandardGate::X {
                        dm.apply_ccx(qubit_indices[0], qubit_indices[1], qubit_indices[2])?;
                    } else {
                        let matrix = mc_gate.matrix(&params).map_err(|_| {
                            QisError::CircuitError(CircuitError::NoMatrixRepresentation)
                        })?;
                        dm.apply_unitary_gate(&qubit_indices, &matrix)?;
                    }
                }
                Instruction::UnitaryGate(u_gate) => {
                    if let Some(matrix) = u_gate.matrix() {
                        dm.apply_unitary_gate(&qubit_indices, matrix)?;
                    } else {
                        return Err(QisError::CircuitError(CircuitError::NoMatrixRepresentation));
                    }
                }
                Instruction::CircuitGate(_) => {
                    return Err(QisError::CircuitError(CircuitError::InvalidOperation(
                        "CircuitGate should have been decomposed".to_string(),
                    )));
                }
                Instruction::Directive(directive) => match directive {
                    Directive::Barrier => {}
                    Directive::Measure => {
                        return Err(QisError::UnsupportedOperation(
                            "legacy Measure directive is not supported by density matrix circuit evolution; use Circuit::measure as an output declaration".to_string(),
                        ));
                    }
                    Directive::Reset => {
                        let qubit = qubit_indices[0];
                        dm.reset(qubit)?;
                    }
                },
                Instruction::Delay => continue,
                Instruction::ClassicalControl(_) => {
                    return Err(QisError::UnsupportedOperation(
                        "classical control flow is not supported in density matrix simulation"
                            .to_string(),
                    ));
                }
                Instruction::ClassicalData(_) => {}
            }
        }
        Ok(())
    }

    pub fn apply_standard_gate(
        &mut self,
        gate: StandardGate,
        qubits: &[usize],
        params: &[f64],
    ) -> Result<(), QisError> {
        if qubits.len() != gate.num_qubits() {
            return Err(QisError::InvalidParameterValue(format!(
                "Gate {:?} requires {} qubits, got {}",
                gate,
                gate.num_qubits(),
                qubits.len()
            )));
        }
        if params.len() != gate.num_params() {
            return Err(QisError::InvalidParameterValue(format!(
                "Gate {:?} requires {} parameters, got {}",
                gate,
                gate.num_params(),
                params.len()
            )));
        }
        match gate {
            StandardGate::I => {}
            StandardGate::X => self.apply_x(qubits[0])?,
            StandardGate::Y => self.apply_y(qubits[0])?,
            StandardGate::Z => self.apply_z(qubits[0])?,
            StandardGate::H => self.apply_h(qubits[0])?,
            StandardGate::S => self.apply_s(qubits[0])?,
            StandardGate::SDG => self.apply_sdg(qubits[0])?,
            StandardGate::T => self.apply_t(qubits[0])?,
            StandardGate::TDG => self.apply_tdg(qubits[0])?,
            StandardGate::RX => self.apply_rx(qubits[0], params[0])?,
            StandardGate::RY => self.apply_ry(qubits[0], params[0])?,
            StandardGate::RZ => self.apply_rz(qubits[0], params[0])?,
            StandardGate::Phase => self.apply_phase(qubits[0], params[0])?,
            StandardGate::X2P => self.apply_x2p(qubits[0])?,
            StandardGate::X2M => self.apply_x2m(qubits[0])?,
            StandardGate::Y2P => self.apply_y2p(qubits[0])?,
            StandardGate::Y2M => self.apply_y2m(qubits[0])?,
            StandardGate::RXY => self.apply_rxy(qubits[0], params[0], params[1])?,
            StandardGate::XY => self.apply_xy(qubits[0], params[0])?,
            StandardGate::XY2P => self.apply_xy2p(qubits[0], params[0])?,
            StandardGate::XY2M => self.apply_xy2m(qubits[0], params[0])?,
            StandardGate::U => self.apply_u(qubits[0], params[0], params[1], params[2])?,
            StandardGate::GPhase => self.apply_gphase(params[0]),

            StandardGate::CX => self.apply_cx(qubits[0], qubits[1])?,
            StandardGate::CY => self.apply_cy(qubits[0], qubits[1])?,
            StandardGate::CZ => self.apply_cz(qubits[0], qubits[1])?,
            StandardGate::SWAP => self.apply_swap(qubits[0], qubits[1])?,
            StandardGate::RXX => self.apply_rxx(qubits[0], qubits[1], params[0])?,
            StandardGate::RYY => self.apply_ryy(qubits[0], qubits[1], params[0])?,
            StandardGate::RZZ => self.apply_rzz(qubits[0], qubits[1], params[0])?,
            StandardGate::RZX => self.apply_rzx(qubits[0], qubits[1], params[0])?,

            StandardGate::CRX => self.apply_crx(qubits[0], qubits[1], params[0])?,
            StandardGate::CRY => self.apply_cry(qubits[0], qubits[1], params[0])?,
            StandardGate::CRZ => self.apply_crz(qubits[0], qubits[1], params[0])?,

            StandardGate::CCX => self.apply_ccx(qubits[0], qubits[1], qubits[2])?,

            StandardGate::FSIM => self.apply_fsim(qubits[0], qubits[1], params[0], params[1])?,
        }
        Ok(())
    }

    /// Creates a new density matrix filled entirely with zeros.
    ///
    /// This is not a valid physical state (trace = 0) but is useful as an accumulator
    /// during operations like Kraus channel application.
    ///
    /// # Arguments
    /// * `num_qubits` - Number of qubits in the system.
    pub fn zeros(num_qubits: usize) -> Self {
        let size = 1 << (2 * num_qubits);
        Self {
            data: vec![Complex64::new(0.0, 0.0); size],
            num_qubits,
        }
    }

    /// Computes the trace of the density matrix.
    ///
    /// For any valid physical state, the trace must equal 1.0.
    ///
    /// # Returns
    /// The trace (sum of diagonal elements) as a complex number.
    pub fn trace(&self) -> Complex64 {
        let n = self.num_qubits;
        let dim = 1 << n;
        if n < PARALLEL_THRESHOLD {
            (0..dim).map(|i| self.data[i * dim + i]).sum()
        } else {
            (0..dim)
                .into_par_iter()
                .map(|i| self.data[i * dim + i])
                .sum()
        }
    }

    /// Checks if the density matrix is Hermitian (self-adjoint) within a tolerance.
    ///
    /// A valid density matrix must satisfy ρ = ρ†, i.e., ρ_ij = ρ_ji*.
    ///
    /// # Arguments
    /// * `tol` - Tolerance for floating-point comparison (e.g., 1e-10).
    ///
    /// # Returns
    /// `true` if the matrix is Hermitian within the specified tolerance.
    pub fn is_hermitian(&self, tol: f64) -> bool {
        let dim = 1 << self.num_qubits;
        for i in 0..dim {
            for j in 0..i {
                let ij_idx = i * dim + j;
                let ji_idx = j * dim + i;
                let ij = self.data[ij_idx];
                let ji_conj = self.data[ji_idx].conj();
                if (ij.re - ji_conj.re).abs() > tol || (ij.im - ji_conj.im).abs() > tol {
                    return false;
                }
            }
        }
        // Check diagonal is real
        for i in 0..dim {
            if self.data[i * dim + i].im.abs() > tol {
                return false;
            }
        }
        true
    }

    /// Checks if the density matrix is positive semidefinite (approximately).
    ///
    /// Uses faer's self-adjoint eigenvalue decomposition to check that
    /// all eigenvalues satisfy λᵢ ≥ -tol.
    ///
    /// For Hermitian matrices the eigenvalue check is definitive.
    /// For non-Hermitian inputs the decomposition may fail or produce
    /// meaningless results, in which case `false` is returned.
    ///
    /// # Edge cases
    /// - Matrices containing NaN or Inf return `false` without panicking.
    /// - Negative eigenvalues smaller in magnitude than `tol` are treated
    ///   as numerical noise and pass the check.
    ///
    /// # Arguments
    /// * `tol` - Tolerance for floating-point comparison.
    ///
    /// # Returns
    /// `true` if all eigenvalues are ≥ -tol, `false` otherwise.
    pub fn is_positive_semidefinite_approx(&self, tol: f64) -> bool {
        let dim = 1 << self.num_qubits;

        // Reject NaN/Inf immediately.
        if self
            .data
            .iter()
            .any(|c| c.re.is_nan() || c.re.is_infinite() || c.im.is_nan() || c.im.is_infinite())
        {
            return false;
        }

        if !self.is_hermitian(tol) {
            return false;
        }

        // Quick rejection: negative diagonal (necessary condition).
        for i in 0..dim {
            if self.data[i * dim + i].re < -tol {
                return false;
            }
        }

        // Build faer matrix and compute self-adjoint eigenvalues.
        let mat = faer::Mat::from_fn(dim, dim, |row, col| {
            let c = self.data[row * dim + col];
            faer::c64::new(c.re, c.im)
        });

        match mat.self_adjoint_eigenvalues(faer::Side::Lower) {
            Ok(eigenvalues) => eigenvalues.iter().all(|&lambda| lambda >= -tol),
            Err(_) => false,
        }
    }

    /// Validates all physical constraints of the density matrix.
    ///
    /// Checks:
    /// 1. Hermiticity: ρ = ρ†
    /// 2. Positive semidefiniteness: All eigenvalues >= 0
    /// 3. Unit trace: Tr(ρ) = 1
    ///
    /// # Arguments
    /// * `tol` - Tolerance for floating-point comparisons (e.g., 1e-10).
    ///
    /// # Returns
    /// * `Ok(())` if all constraints are satisfied.
    /// * `Err(QisError::NotHermitian)` if not Hermitian.
    /// * `Err(QisError::NotPositiveSemidefinite)` if not positive semidefinite.
    /// * `Err(QisError::NotNormalized)` if trace is not 1.
    pub fn validate_physical(&self, tol: f64) -> Result<(), QisError> {
        // Check Hermiticity
        if !self.is_hermitian(tol) {
            return Err(QisError::NotHermitian);
        }

        // Check positive semidefiniteness
        if !self.is_positive_semidefinite_approx(tol) {
            return Err(QisError::NotPositiveSemidefinite);
        }

        // Check unit trace
        let tr = self.trace();
        if (tr.re - 1.0).abs() >= tol || tr.im.abs() >= tol {
            return Err(QisError::NotNormalized);
        }

        Ok(())
    }

    fn apply_x_kernel(&mut self, q: usize, off: usize) {
        let d = 1 << (q + off);
        let kernel = |chunk: &mut [Complex64]| {
            let (l, u) = chunk.split_at_mut(d);
            l.iter_mut()
                .zip(u.iter_mut())
                .for_each(|(a, b)| std::mem::swap(a, b));
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d, kernel);
    }

    fn apply_y_kernel(&mut self, q: usize, off: usize, conj: bool) {
        let d = 1 << (q + off);
        let kernel = |chunk: &mut [Complex64]| {
            let (l, u) = chunk.split_at_mut(d);
            l.iter_mut().zip(u.iter_mut()).for_each(|(alpha, beta)| {
                let a = *alpha;
                let b = *beta;
                if conj {
                    *alpha = Complex64::new(-b.im, b.re);
                    *beta = Complex64::new(a.im, -a.re);
                } else {
                    *alpha = Complex64::new(b.im, -b.re);
                    *beta = Complex64::new(-a.im, a.re);
                }
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d, kernel);
    }

    fn apply_z_kernel(&mut self, q: usize, off: usize) {
        let d = 1 << (q + off);
        let kernel = |chunk: &mut [Complex64]| {
            u_slice(chunk, d).iter_mut().for_each(|v| *v = -*v);
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d, kernel);
    }

    fn apply_h_kernel(&mut self, q: usize, off: usize) {
        let d = 1 << (q + off);
        let k = FRAC_1_SQRT_2;
        let kernel = |chunk: &mut [Complex64]| {
            let (l, u) = chunk.split_at_mut(d);
            l.iter_mut().zip(u.iter_mut()).for_each(|(a, b)| {
                let (v0, v1) = (*a, *b);
                *a = k * (v0 + v1);
                *b = k * (v0 - v1);
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d, kernel);
    }

    fn apply_rx_kernel(&mut self, q: usize, off: usize, theta: f64) {
        let d = 1 << (q + off);
        let (c, s) = ((theta * 0.5).cos(), (theta * 0.5).sin());
        let kernel = |chunk: &mut [Complex64]| {
            let (l, u) = chunk.split_at_mut(d);
            l.iter_mut().zip(u.iter_mut()).for_each(|(alpha, beta)| {
                let (a, b) = (*alpha, *beta);
                *alpha = Complex64::new(a.re * c + b.im * s, a.im * c - b.re * s);
                *beta = Complex64::new(b.re * c + a.im * s, b.im * c - a.re * s);
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d, kernel);
    }

    fn apply_ry_kernel(&mut self, q: usize, off: usize, theta: f64) {
        let d = 1 << (q + off);
        let (c, s) = ((theta * 0.5).cos(), (theta * 0.5).sin());
        let kernel = |chunk: &mut [Complex64]| {
            let (l, u) = chunk.split_at_mut(d);
            l.iter_mut().zip(u.iter_mut()).for_each(|(a, b)| {
                let (v0, v1) = (*a, *b);
                *a = v0 * c - v1 * s;
                *b = v0 * s + v1 * c;
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d, kernel);
    }

    fn apply_u_kernel(&mut self, q: usize, off: usize, theta: f64, phi: f64, lam: f64, conj: bool) {
        let d = 1 << (q + off);
        let (c, s) = ((theta * 0.5).cos(), (theta * 0.5).sin());
        let (p_phi, p_lam, p_tot) = if conj {
            (
                Complex64::from_polar(1.0, -phi),
                Complex64::from_polar(1.0, -lam),
                Complex64::from_polar(1.0, -(phi + lam)),
            )
        } else {
            (
                Complex64::from_polar(1.0, phi),
                Complex64::from_polar(1.0, lam),
                Complex64::from_polar(1.0, phi + lam),
            )
        };
        let (u00, u01, u10, u11) = (Complex64::new(c, 0.0), -p_lam * s, p_phi * s, p_tot * c);
        let kernel = |chunk: &mut [Complex64]| {
            let (l, u) = chunk.split_at_mut(d);
            l.iter_mut().zip(u.iter_mut()).for_each(|(a, b)| {
                let (v0, v1) = (*a, *b);
                *a = u00 * v0 + u01 * v1;
                *b = u10 * v0 + u11 * v1;
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d, kernel);
    }

    fn apply_cx_kernel(&mut self, ctrl: usize, tgt: usize, off: usize) {
        let (c, t) = (ctrl + off, tgt + off);
        let (q_min, q_max) = if c < t { (c, t) } else { (t, c) };
        let (d_min, d_max) = (1 << q_min, 1 << q_max);
        let kernel = |chunk: &mut [Complex64]| {
            let (p0, p1) = chunk.split_at_mut(d_max);
            p0.chunks_exact_mut(2 * d_min)
                .zip(p1.chunks_exact_mut(2 * d_min))
                .for_each(|(s0, s1)| {
                    if c < t {
                        let (_, v01) = s0.split_at_mut(d_min);
                        let (_, v11) = s1.split_at_mut(d_min);
                        v01.iter_mut()
                            .zip(v11.iter_mut())
                            .for_each(|(a, b)| std::mem::swap(a, b));
                    } else {
                        let (v10, v11) = s1.split_at_mut(d_min);
                        v10.iter_mut()
                            .zip(v11.iter_mut())
                            .for_each(|(a, b)| std::mem::swap(a, b));
                    }
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d_max, kernel);
    }

    fn apply_cz_kernel(&mut self, q0: usize, q1: usize, off: usize) {
        let (c0, c1) = (q0 + off, q1 + off);
        let (q_min, q_max) = if c0 < c1 { (c0, c1) } else { (c1, c0) };
        let (d_min, d_max) = (1 << q_min, 1 << q_max);
        let kernel = |chunk: &mut [Complex64]| {
            let (_, p1) = chunk.split_at_mut(d_max);
            p1.chunks_exact_mut(2 * d_min).for_each(|s| {
                u_slice(s, d_min).iter_mut().for_each(|v| *v = -*v);
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d_max, kernel);
    }

    /// Applies the Pauli-X (NOT) gate to the specified qubit.
    ///
    /// # Errors
    /// Returns `QisError::IndexOutOfBounds` if qubit >= num_qubits.
    pub fn apply_x(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        self.apply_x_kernel(qubit, self.num_qubits);
        self.apply_x_kernel(qubit, 0);
        Ok(())
    }
    /// Applies the Pauli-Y gate to the specified qubit.
    pub fn apply_y(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        self.apply_y_kernel(qubit, self.num_qubits, false);
        self.apply_y_kernel(qubit, 0, true);
        Ok(())
    }
    /// Applies the Pauli-Z gate to the specified qubit.
    pub fn apply_z(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        self.apply_z_kernel(qubit, self.num_qubits);
        self.apply_z_kernel(qubit, 0);
        Ok(())
    }
    /// Applies the Hadamard gate to the specified qubit.
    pub fn apply_h(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        self.apply_h_kernel(qubit, self.num_qubits);
        self.apply_h_kernel(qubit, 0);
        Ok(())
    }
    /// Applies the generic single-qubit U gate with parameters `theta`, `phi`, and `lam`.
    pub fn apply_u(
        &mut self,
        qubit: usize,
        theta: f64,
        phi: f64,
        lambda: f64,
    ) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        self.apply_u_kernel(qubit, self.num_qubits, theta, phi, lambda, false);
        self.apply_u_kernel(qubit, 0, theta, phi, lambda, true);
        Ok(())
    }
    /// Applies the single-qubit rotation about the X-axis by angle `theta`.
    pub fn apply_rx(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        self.apply_rx_kernel(qubit, self.num_qubits, theta);
        self.apply_rx_kernel(qubit, 0, -theta);
        Ok(())
    }
    /// Applies the single-qubit rotation about the Y-axis by angle `theta`.
    pub fn apply_ry(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        self.apply_ry_kernel(qubit, self.num_qubits, theta);
        self.apply_ry_kernel(qubit, 0, theta);
        Ok(())
    }
    /// Applies the single-qubit rotation about the Z-axis by angle `theta`.
    pub fn apply_rz(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.apply_phase(qubit, theta)?; /* RZ is equivalent to Phase gate in density matrix */
        Ok(())
    }
    /// Applies a phase shift of `theta` to the specified qubit.
    pub fn apply_phase(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        self.apply_p_kernel(qubit, self.num_qubits, theta);
        self.apply_p_kernel(qubit, 0, -theta);
        Ok(())
    }

    /// Applies the S (Phase) gate to the specified qubit.
    pub fn apply_s(&mut self, qubit: usize) -> Result<(), QisError> {
        self.apply_phase(qubit, FRAC_PI_2)?;
        Ok(())
    }
    /// Applies the inverse S (SDG) gate to the specified qubit.
    pub fn apply_sdg(&mut self, qubit: usize) -> Result<(), QisError> {
        self.apply_phase(qubit, -FRAC_PI_2)?;
        Ok(())
    }
    /// Applies the T (Pi/8) gate to the specified qubit.
    pub fn apply_t(&mut self, qubit: usize) -> Result<(), QisError> {
        self.apply_phase(qubit, FRAC_PI_4)?;
        Ok(())
    }
    /// Applies the inverse T (TDG) gate to the specified qubit.
    pub fn apply_tdg(&mut self, qubit: usize) -> Result<(), QisError> {
        self.apply_phase(qubit, -FRAC_PI_4)?;
        Ok(())
    }
    /// Applies a +Pi/2 rotation about the X-axis.
    pub fn apply_x2p(&mut self, qubit: usize) -> Result<(), QisError> {
        self.apply_rx(qubit, FRAC_PI_2)?;
        Ok(())
    }
    /// Applies a -Pi/2 rotation about the X-axis.
    pub fn apply_x2m(&mut self, qubit: usize) -> Result<(), QisError> {
        self.apply_rx(qubit, -FRAC_PI_2)?;
        Ok(())
    }
    /// Applies a +Pi/2 rotation about the Y-axis.
    pub fn apply_y2p(&mut self, qubit: usize) -> Result<(), QisError> {
        self.apply_ry(qubit, FRAC_PI_2)?;
        Ok(())
    }
    /// Applies a -Pi/2 rotation about the Y-axis.
    pub fn apply_y2m(&mut self, qubit: usize) -> Result<(), QisError> {
        self.apply_ry(qubit, -FRAC_PI_2)?;
        Ok(())
    }
    /// Applies the parameterized RXY rotation gate.
    pub fn apply_rxy(&mut self, qubit: usize, theta: f64, phi: f64) -> Result<(), QisError> {
        self.apply_u(qubit, theta, phi - FRAC_PI_2, FRAC_PI_2 - phi)?;
        Ok(())
    }
    /// Applies the XY2P gate.
    pub fn apply_xy2p(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.apply_rxy(qubit, FRAC_PI_2, theta)?;
        Ok(())
    }
    /// Applies the XY2M gate.
    pub fn apply_xy2m(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.apply_rxy(qubit, -FRAC_PI_2, theta)?;
        Ok(())
    }
    /// Applies the XY gate.
    ///
    /// Matrix: [[0, -i·e^(-iθ)], [-i·e^(iθ), 0]]
    pub fn apply_xy(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let mat = StandardGate::XY.matrix(&[theta]).unwrap();
        self.apply_single_qubit_gate(
            qubit,
            [[mat[[0, 0]], mat[[0, 1]]], [mat[[1, 0]], mat[[1, 1]]]],
        )?;
        Ok(())
    }
    /// Applies a global phase (has no observable effect on a density matrix).
    pub fn apply_gphase(&mut self, _phi: f64) { /* Global phase has no effect on density matrix */
    }
    fn apply_p_kernel(&mut self, q: usize, off: usize, t: f64) {
        let d = 1 << (q + off);
        let phase = Complex64::from_polar(1.0, t);
        let kernel = |chunk: &mut [Complex64]| {
            u_slice(chunk, d).iter_mut().for_each(|v| *v *= phase);
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d, kernel);
    }

    /// Applies the Controlled-X (CNOT) gate.
    pub fn apply_cx(&mut self, control: usize, target: usize) -> Result<(), QisError> {
        self.validate_two_qubits(control, target)?;
        self.apply_cx_kernel(control, target, self.num_qubits);
        self.apply_cx_kernel(control, target, 0);
        Ok(())
    }
    /// Applies the Controlled-Z gate.
    pub fn apply_cz(&mut self, q0: usize, q1: usize) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        self.apply_cz_kernel(q0, q1, self.num_qubits);
        self.apply_cz_kernel(q0, q1, 0);
        Ok(())
    }

    /// Applies an arbitrary 2x2 unitary matrix to a single qubit.
    pub fn apply_single_qubit_gate(
        &mut self,
        qubit: usize,
        matrix: [[Complex64; 2]; 2],
    ) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let flat = vec![matrix[0][0], matrix[0][1], matrix[1][0], matrix[1][1]];
        self.apply_matrix_kernel(&[qubit], self.num_qubits, &flat, false);
        self.apply_matrix_kernel(&[qubit], 0, &flat, true);
        Ok(())
    }

    /// Applies an arbitrary 4x4 unitary matrix to two qubits.
    pub fn apply_two_qubit_gate(
        &mut self,
        q0: usize,
        q1: usize,
        matrix: [[Complex64; 4]; 4],
    ) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        let mut flat = Vec::with_capacity(16);
        for row in &matrix {
            for &val in row {
                flat.push(val);
            }
        }
        self.apply_matrix_kernel(&[q0, q1], self.num_qubits, &flat, false);
        self.apply_matrix_kernel(&[q0, q1], 0, &flat, true);
        Ok(())
    }

    /// Applies the SWAP gate between two qubits.
    pub fn apply_swap(&mut self, q0: usize, q1: usize) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        let mat = StandardGate::SWAP.matrix(&[]).unwrap();
        self.apply_unitary_gate(&[q0, q1], &mat)?;
        Ok(())
    }

    /// Applies the Controlled-Y gate.
    pub fn apply_cy(&mut self, control: usize, target: usize) -> Result<(), QisError> {
        self.validate_two_qubits(control, target)?;
        let mat = StandardGate::CY.matrix(&[]).unwrap();
        self.apply_unitary_gate(&[control, target], &mat)?;
        Ok(())
    }

    /// Applies the Toffoli (Controlled-Controlled-X) gate.
    pub fn apply_ccx(&mut self, c0: usize, c1: usize, target: usize) -> Result<(), QisError> {
        self.validate_qubits(&[c0, c1, target])?;
        let mat = StandardGate::CCX.matrix(&[]).unwrap();
        self.apply_unitary_gate(&[c0, c1, target], &mat)?;
        Ok(())
    }

    /// Applies the parameterized RXX (Ising XX) gate.
    pub fn apply_rxx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        let mat = StandardGate::RXX.matrix(&[theta]).unwrap();
        self.apply_unitary_gate(&[q0, q1], &mat)?;
        Ok(())
    }

    /// Applies the parameterized RYY (Ising YY) gate.
    pub fn apply_ryy(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        let mat = StandardGate::RYY.matrix(&[theta]).unwrap();
        self.apply_unitary_gate(&[q0, q1], &mat)?;
        Ok(())
    }

    /// Applies the parameterized RZZ (Ising ZZ) gate.
    pub fn apply_rzz(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        let mat = StandardGate::RZZ.matrix(&[theta]).unwrap();
        self.apply_unitary_gate(&[q0, q1], &mat)?;
        Ok(())
    }

    /// Applies the parameterized RZX gate.
    pub fn apply_rzx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        let mat = StandardGate::RZX.matrix(&[theta]).unwrap();
        self.apply_unitary_gate(&[q0, q1], &mat)?;
        Ok(())
    }

    /// Applies the Controlled-RX gate.
    pub fn apply_crx(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(control, target)?;
        let mat = StandardGate::CRX.matrix(&[theta]).unwrap();
        self.apply_unitary_gate(&[control, target], &mat)?;
        Ok(())
    }

    /// Applies the Controlled-RY gate.
    pub fn apply_cry(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(control, target)?;
        let mat = StandardGate::CRY.matrix(&[theta]).unwrap();
        self.apply_unitary_gate(&[control, target], &mat)?;
        Ok(())
    }

    /// Applies the Controlled-RZ gate.
    pub fn apply_crz(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(control, target)?;
        let mat = StandardGate::CRZ.matrix(&[theta]).unwrap();
        self.apply_unitary_gate(&[control, target], &mat)?;
        Ok(())
    }

    /// Applies the Fermionic Simulation (FSIM) gate.
    pub fn apply_fsim(
        &mut self,
        q0: usize,
        q1: usize,
        theta: f64,
        phi: f64,
    ) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        let mat = StandardGate::FSIM.matrix(&[theta, phi]).unwrap();
        self.apply_unitary_gate(&[q0, q1], &mat)?;
        Ok(())
    }

    /// Applies an arbitrary n-qubit unitary gate.
    ///
    /// The evolution is given by $\rho \to U \rho U^\dagger$.
    ///
    /// # Arguments
    /// * `qs` - Slice of qubit indices the gate acts on.
    /// * `mat` - The unitary matrix as a $2^n \times 2^n$ `ndarray`.
    pub fn apply_unitary_gate(
        &mut self,
        qubits: &[usize],
        matrix: &ndarray::Array2<Complex64>,
    ) -> Result<(), QisError> {
        let num_target_qubits = qubits.len();
        let gate_dim = 1 << num_target_qubits;

        // Validate number of target qubits
        if num_target_qubits == 0 || num_target_qubits > self.num_qubits {
            return Err(QisError::InvalidParameterValue(format!(
                "Invalid number of target qubits: {} (must be 1 to {})",
                num_target_qubits, self.num_qubits
            )));
        }

        // Validate qubits and check for duplicates
        for (i, &q) in qubits.iter().enumerate() {
            if q >= self.num_qubits {
                return Err(QisError::IndexOutOfBounds {
                    index: q,
                    max: self.num_qubits.saturating_sub(1),
                });
            }
            for &other in &qubits[..i] {
                if q == other {
                    return Err(QisError::InvalidParameterValue(format!(
                        "Duplicate qubit index {} in apply_unitary_gate",
                        q
                    )));
                }
            }
        }

        // Validate matrix dimensions
        if matrix.shape() != [gate_dim, gate_dim] {
            return Err(QisError::InvalidParameterValue(format!(
                "Matrix dimensions {}x{} don't match expected {}x{} for {} qubits",
                matrix.nrows(),
                matrix.ncols(),
                gate_dim,
                gate_dim,
                num_target_qubits
            )));
        }

        let flat: Vec<Complex64> = matrix.iter().cloned().collect();
        self.apply_matrix_kernel(qubits, self.num_qubits, &flat, false);
        self.apply_matrix_kernel(qubits, 0, &flat, true);
        Ok(())
    }

    fn apply_matrix_kernel(&mut self, qs: &[usize], off: usize, mat: &[Complex64], conj: bool) {
        Self::apply_matrix_kernel_impl(&mut self.data, self.num_qubits, qs, off, mat, conj);
    }

    fn apply_matrix_kernel_impl(
        data: &mut [Complex64],
        num_qubits: usize,
        qs: &[usize],
        off: usize,
        mat: &[Complex64],
        conj: bool,
    ) {
        if qs.is_empty() {
            return;
        }
        let n = qs.len();
        let dim = 1 << n;
        let mut offsets: SmallVec<[usize; 16]> = smallvec![0; dim];
        for i in 0..dim {
            for (j, &q) in qs.iter().enumerate() {
                if (i >> (n - 1 - j)) & 1 == 1 {
                    offsets[i] |= 1 << (q + off);
                }
            }
        }
        let mut sorted: SmallVec<[usize; 16]> = qs.iter().map(|&q| q + off).collect();
        sorted.sort_unstable();
        let chunk_size = 1 << (sorted.last().unwrap() + 1);

        let kernel = |chunk: &mut [Complex64]| {
            let mut i_buf: SmallVec<[Complex64; 16]> = smallvec![Complex64::default(); dim];
            let mut o_buf: SmallVec<[Complex64; 16]> = smallvec![Complex64::default(); dim];
            for b_idx in 0..(chunk.len() >> n) {
                let mut base = b_idx;
                for &q in &sorted {
                    let mask = (1 << q) - 1;
                    base = ((base & !mask) << 1) | (base & mask);
                }
                for i in 0..dim {
                    i_buf[i] = chunk[base + offsets[i]];
                }
                for r in 0..dim {
                    let mut sum = Complex64::default();
                    for c in 0..dim {
                        let val = mat[r * dim + c];
                        let coeff = if conj { val.conj() } else { val };
                        sum += coeff * i_buf[c];
                    }
                    o_buf[r] = sum;
                }
                for i in 0..dim {
                    chunk[base + offsets[i]] = o_buf[i];
                }
            }
        };
        with_maybe_par!(num_qubits, data, chunk_size, kernel);
    }

    /// Applies a general quantum channel specified by Kraus operators.
    ///
    /// The evolution of the density matrix is given by $\rho \to \sum_k K_k \rho K_k^\dagger$,
    /// where $\sum_k K_k^\dagger K_k = I$ for a trace-preserving channel.
    ///
    /// # Arguments
    /// * `ops` - A slice of Kraus operators, where each operator is represented as a flattened vector of `Complex64`.
    /// * `qs` - The target qubit indices the channel acts upon.
    pub fn apply_kraus(&mut self, ops: &[Vec<Complex64>], qs: &[usize]) -> Result<(), QisError> {
        if ops.is_empty() {
            return Err(QisError::InvalidParameterValue(
                "Kraus operator list must not be empty".to_string(),
            ));
        }
        if qs.is_empty() {
            return Err(QisError::InvalidParameterValue(
                "Kraus channel must target at least one qubit".to_string(),
            ));
        }
        self.validate_qubits(qs)?;
        let op_dim = 1usize << qs.len();
        let expected_len = op_dim * op_dim;
        for (idx, op) in ops.iter().enumerate() {
            if op.len() != expected_len {
                return Err(QisError::InvalidParameterValue(format!(
                    "Kraus operator {} has length {}, expected {} for {} target qubits",
                    idx,
                    op.len(),
                    expected_len,
                    qs.len()
                )));
            }
        }
        let source_data = self.data.clone();

        for val in self.data.iter_mut() {
            *val = Complex64::default();
        }

        let mut work_buffer = vec![Complex64::default(); self.data.len()];

        for op in ops {
            work_buffer.copy_from_slice(&source_data);

            Self::apply_matrix_kernel_impl(
                &mut work_buffer,
                self.num_qubits,
                qs,
                self.num_qubits,
                op,
                false,
            );
            Self::apply_matrix_kernel_impl(&mut work_buffer, self.num_qubits, qs, 0, op, true);

            if self.num_qubits < PARALLEL_THRESHOLD {
                for (acc, val) in self.data.iter_mut().zip(&work_buffer) {
                    *acc += val;
                }
            } else {
                use rayon::prelude::*;
                self.data
                    .par_iter_mut()
                    .zip(work_buffer.par_iter())
                    .for_each(|(acc, val)| {
                        *acc += val;
                    });
            }
        }

        #[cfg(debug_assertions)]
        {
            let tr = self.trace();
            debug_assert!((tr.re - 1.0).abs() < 1e-10, "Trace error: {}", tr);
        }
        Ok(())
    }

    /// Computes the partial trace over a set of qubits.
    ///
    /// Reduces the N-qubit system to a smaller subsystem containing only the specified `keep` qubits
    /// by tracing out all other qubits.
    ///
    /// # Arguments
    /// * `keep` - A slice of qubit indices to keep in the resulting reduced density matrix.
    ///
    /// # Returns
    /// A new `DensityMatrix` representing the subsystem, with `num_qubits = keep.len()`.
    pub fn partial_trace(&self, keep: &[usize]) -> Result<Self, QisError> {
        for (idx, &q) in keep.iter().enumerate() {
            if q >= self.num_qubits {
                return Err(QisError::IndexOutOfBounds {
                    index: q,
                    max: self.num_qubits.saturating_sub(1),
                });
            }
            if keep[..idx].contains(&q) {
                return Err(QisError::InvalidParameterValue(format!(
                    "Duplicate qubit index {} in partial_trace",
                    q
                )));
            }
        }
        let mut s_keep = keep.to_vec();
        s_keep.sort_unstable();
        let all: HashSet<_> = (0..self.num_qubits).collect();
        let mut trace: Vec<_> = all
            .difference(&s_keep.iter().cloned().collect())
            .cloned()
            .collect();
        trace.sort_unstable();
        let (n_k, n_t) = (s_keep.len(), trace.len());
        let (dim, r_dim) = (1 << self.num_qubits, 1 << n_k);
        let mut res = Self::zeros(n_k);
        res.data.par_iter_mut().enumerate().for_each(|(idx, val)| {
            let (i, j) = (idx / r_dim, idx % r_dim);
            let mut sum = Complex64::default();
            for t in 0..(1 << n_t) {
                sum += self.data[interleave_bits(&s_keep, i, &trace, t) * dim
                    + interleave_bits(&s_keep, j, &trace, t)];
            }
            *val = sum;
        });
        Ok(res)
    }

    pub(crate) fn partial_transpose(&self, target_qubits: &[usize]) -> Result<Self, QisError> {
        for &q in target_qubits {
            self.validate_qubit(q)?;
        }

        let n = self.num_qubits;
        let mut swap_mask = 0usize;
        for &q in target_qubits {
            swap_mask |= 1 << q;
        }

        let mut res = Self::zeros(n);
        res.data.par_iter_mut().enumerate().for_each(|(idx, val)| {
            let lower_swap_bits = idx & swap_mask;
            let upper_swap_bits = (idx >> n) & swap_mask;

            let mut src_idx = idx & !(swap_mask | (swap_mask << n));
            src_idx |= upper_swap_bits;
            src_idx |= lower_swap_bits << n;

            *val = self.data[src_idx];
        });

        Ok(res)
    }

    /// Computes the expectation value of a Hamiltonian observable.
    ///
    /// Calculates Tr(ρ * H) for the current density matrix ρ and a given Hamiltonian H.
    /// The Hamiltonian is represented as a sum of Pauli strings with coefficients.
    ///
    /// # Arguments
    /// * `h` - The Hamiltonian observable.
    ///
    /// # Returns
    /// The expectation value as a real number (f64), or a [`QisError`] if the
    /// qubit counts do not match.
    ///
    /// # Errors
    /// Returns [`QisError::CircuitError`] if the observable acts on a different
    /// number of qubits than the density matrix.
    pub fn expectation(&self, h: &dyn Observable) -> Result<f64, QisError> {
        h.expectation_density_matrix(self)
    }

    /// Copies density matrix data from `source` into `self` without allocating.
    ///
    /// Both matrices must have the same `num_qubits`. Used by
    /// [`sample_shots`](Self::sample_shots) to reuse each thread's working copy.
    pub(crate) fn reset_from(&mut self, source: &DensityMatrix) {
        debug_assert_eq!(self.num_qubits, source.num_qubits);
        self.data.copy_from_slice(&source.data);
    }

    /// Measures `qubit` in the Z basis, collapsing the density matrix in-place.
    ///
    /// Returns `true` for outcome |1⟩ and `false` for outcome |0⟩.
    /// The density matrix is projected into the subspace consistent with the
    /// outcome and renormalized: `ρ' = Π_b ρ Π_b / Tr(Π_b ρ)`.
    pub fn measure(&mut self, qubit: usize) -> Result<bool, QisError> {
        self.validate_qubit(qubit)?;
        let dim = 1 << self.num_qubits;
        let mask = 1usize << qubit;

        // P(|1⟩) = sum of diagonal elements where bit `qubit` is 1.
        let p1: f64 = (0..dim)
            .filter(|i| i & mask != 0)
            .map(|i| self.data[i * dim + i].re)
            .sum();

        let outcome = {
            use rand::Rng;
            let mut rng = rand::rng();
            rng.random::<f64>() < p1
        };

        // Project and renormalise: zero rows/cols inconsistent with outcome.
        let norm = if outcome { p1 } else { 1.0 - p1 };
        for row in 0..dim {
            let row_match = if outcome {
                (row & mask) != 0
            } else {
                (row & mask) == 0
            };
            for col in 0..dim {
                let col_match = if outcome {
                    (col & mask) != 0
                } else {
                    (col & mask) == 0
                };
                if row_match && col_match {
                    self.data[row * dim + col] /= norm;
                } else {
                    self.data[row * dim + col] = Complex64::new(0.0, 0.0);
                }
            }
        }

        Ok(outcome)
    }

    /// Resets `qubit` to the |0⟩ state.
    ///
    /// Reset is modeled as a Z-basis measurement followed by an X correction
    /// when the measured outcome is |1⟩. This is destructive, matching circuit
    /// reset semantics.
    pub fn reset(&mut self, qubit: usize) -> Result<(), QisError> {
        if self.measure(qubit)? {
            self.apply_x(qubit)?;
        }
        Ok(())
    }

    /// Measures all qubits sequentially, returning a bit-packed [`Outcome`].
    ///
    /// Equivalent to calling `measure(q)` for each qubit `q` in order `0..num_qubits`.
    /// The density matrix is fully collapsed after this call.
    /// Use [`Outcome::is_one(q)`](crate::device::Outcome::is_one) to read qubit `q`'s result.
    pub fn measure_all(&mut self) -> Outcome {
        let num_chunks = self.num_qubits.div_ceil(64);
        let mut chunks = SmallVec::from_elem(0u64, num_chunks);
        for q in 0..self.num_qubits {
            if self.measure(q).unwrap() {
                chunks[q / 64] |= 1u64 << (q % 64);
            }
        }
        Outcome::new(chunks)
    }

    /// Samples `shots` independent measurement outcomes in parallel.
    ///
    /// Each Rayon worker thread reuses a single pre-allocated clone of the
    /// density matrix (via [`reset_from`](Self::reset_from)), avoiding per-shot
    /// heap allocation. Returns a [`Vec<Outcome>`] of bit-packed results.
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::qis::DensityMatrix;
    ///
    /// let mut dm = DensityMatrix::new(2);
    /// dm.apply_h(0).unwrap();
    /// dm.apply_cx(0, 1).unwrap(); // |Φ⁺⟩ Bell state
    ///
    /// let shots = dm.sample_shots(200);
    /// // All outcomes must be |00⟩ or |11⟩
    /// assert!(shots.iter().all(|v| v.is_one(0) == v.is_one(1)));
    /// ```
    pub fn sample_shots(&self, shots: usize) -> Vec<Outcome> {
        (0..shots)
            .into_par_iter()
            .map_with(self.clone(), |work, _| {
                work.reset_from(self);
                work.measure_all()
            })
            .collect()
    }

    /// Samples the state using a circuit [`Measurement`] as the output contract.
    ///
    /// `measurement.qubits()[i]` becomes bit `i` in each [`Outcome`]. The returned
    /// [`ExecutionResult`] reuses the device result shape, with counts projected
    /// to only the qubits named by the measurement.
    pub fn sample(
        &self,
        measurement: &Measurement,
        shots: usize,
    ) -> Result<ExecutionResult, QisError> {
        measurement.check_qubits(self.num_qubits)?;

        let mut counts = HashMap::new();
        for full in self.sample_shots(shots) {
            *counts.entry(measurement.project(&full)).or_insert(0usize) += 1;
        }

        Ok(ExecutionResult::from_counts(
            "density-matrix-sample".to_string(),
            measurement.qubits().to_vec(),
            shots,
            measurement.width(),
            Some("density-matrix".to_string()),
            counts,
        ))
    }

    /// Returns the marginal probability distribution selected by a circuit [`Measurement`].
    ///
    /// This method does not execute circuit IR. It treats the [`Measurement`] as
    /// a self-contained output contract and aggregates computational-basis
    /// probabilities over unmeasured qubits.
    pub fn probs(&self, measurement: &Measurement) -> Result<HashMap<Outcome, f64>, QisError> {
        measurement.check_qubits(self.num_qubits)?;

        let mut marginal = HashMap::new();
        for (basis, prob) in self.probabilities().into_iter().enumerate() {
            if prob == 0.0 {
                continue;
            }
            *marginal
                .entry(measurement.project_basis(basis))
                .or_insert(0.0) += prob;
        }
        Ok(marginal)
    }
}

// Helpers for slice splitting
#[inline(always)]
fn u_slice(s: &mut [Complex64], d: usize) -> &mut [Complex64] {
    s.split_at_mut(d).1
}

#[cfg(test)]
#[path = "density_matrix_test.rs"]
mod density_matrix_test;
