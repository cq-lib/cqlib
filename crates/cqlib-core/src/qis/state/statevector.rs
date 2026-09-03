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

//! Statevector quantum simulation.
//!
//! This module provides a high-performance statevector simulator for quantum circuits.
//! It supports:
//! - Single-qubit and multi-qubit gate operations
//! - Controlled and multi-controlled gates
//! - Parameterized rotations (RX, RY, RZ, etc.)
//! - Native gates for superconducting qubits (fSim, XY)
//! - Parallel execution for large statevectors using Rayon
//!
//! # Performance Features
//! - Automatic parallelization threshold (14+ qubits)
//! - Specialized kernels for common gates (avoiding generic matrix multiplication)
//! - Memory-efficient in-place operations
//! - Contiguous memory layout compatible with C/NumPy
//!
//! # Example
//! ```rust
//! use cqlib_core::qis::Statevector;
//!
//! // Create Bell state |Φ+⟩ = (|00⟩ + |11⟩)/√2
//! let mut sv = Statevector::new(2);
//! sv.apply_h(0);
//! sv.apply_cx(0, 1);
//!
//! let probs = sv.probabilities();
//! assert!((probs[0] - 0.5).abs() < 1e-10);
//! assert!((probs[3] - 0.5).abs() < 1e-10);
//! ```

use super::aligned_buffer::AlignedBuffer;
use crate::circuit::Measurement;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::circuit_param::CircuitParam;
use crate::circuit::error::CircuitError;
use crate::circuit::gate::StandardGate;
use crate::circuit::gate::directive::Directive;
use crate::circuit::gate::instruction::Instruction;
use crate::device::{ExecutionResult, Outcome};
use crate::qis::error::QisError;
use crate::qis::observable::Observable;
use crate::qis::pauli::{PauliString, Phase};
use num_complex::Complex64;
use rand::Rng;
use rayon::prelude::*;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::f64::consts::FRAC_1_SQRT_2;

// Statevector gates reduce to: for each (α, β) pair separated by `dist`,
//     new_α = u00·α + u01·β
//     new_β = u10·α + u11·β
// where u_ij are complex scalars fixed for the entire gate.
//
// LLVM auto-vectorizes *real* multiplications well (H gate, Z gate), but
// struggles with complex×complex: `(ac−bd, ad+bc)` requires a cross-term
// subtraction that breaks the sequential dependency pattern. The AVX2 path
// below uses `_mm256_addsub_pd` to express this in 5 SIMD ops per 2 complex
// numbers, delivering ~4× throughput vs. scalar on this hot loop.
//
// Memory layout (interleaved): [re₀, im₀, re₁, im₁, re₂, im₂, …]
//
// The trick (Goto–van de Geijn complex GEMV):
//   Given scalar u = (u_re, u_im), vector of 2 complexes a = [a0, a1]:
//     a_perm = permute(a, 0101b)     → [a0.im, a0.re, a1.im, a1.re]  (swap re/im)
//     term1  = a  * broadcast(u_re)  → [a0.re·u_re, a0.im·u_re, …]
//     term2  = a_perm * broadcast(u_im) → [a0.im·u_im, a0.re·u_im, …]
//     result = addsub(term1, term2)  → even indices subtract, odd add:
//              [a0.re·u_re − a0.im·u_im,   a0.im·u_re + a0.re·u_im, …]
//              = [re(u·a0), im(u·a0), re(u·a1), im(u·a1)] ✓

/// Computes `u·a` for two consecutive Complex64 values using AVX2.
///
/// # Safety
/// Caller must ensure:
/// - `avx2` + `fma` features are enabled at runtime.
/// - `a_ptr` points to at least 4 aligned `f64` words (= 2 Complex64, = 32 bytes).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]
#[inline]
unsafe fn cmul2_avx2(
    a_ptr: *const f64,
    u_re: std::arch::x86_64::__m256d,
    u_im: std::arch::x86_64::__m256d,
) -> std::arch::x86_64::__m256d {
    use std::arch::x86_64::*;
    // SAFETY: caller guarantees alignment and valid pointer.
    unsafe {
        let a = _mm256_load_pd(a_ptr); // [a0.re, a0.im, a1.re, a1.im]
        let a_perm = _mm256_permute_pd(a, 0b0101); // [a0.im, a0.re, a1.im, a1.re]
        let term1 = _mm256_mul_pd(a, u_re); // [a.re·u_re, a.im·u_re, …]
        // addsub: even=subtract, odd=add → [re(u·a), im(u·a), …]
        _mm256_addsub_pd(term1, _mm256_mul_pd(a_perm, u_im))
    }
}

/// AVX2 kernel for a single-qubit gate on one chunk `[lower | upper]`.
///
/// Processes pairs (α, β) where lower[i] = α, upper[i] = β, applying:
///   new_α = u00·α + u01·β
///   new_β = u10·α + u11·β
///
/// Requires `dist` ≥ 2 (handled by the caller which falls back to scalar).
///
/// # Safety
/// - `avx2` + `fma` features enabled.
/// - `lower.as_ptr()` and `upper.as_ptr()` are 32-byte aligned (guaranteed by
///   our `AlignedBuffer<Complex64>` and the chunk layout: `dist` is always a
///   power of 2 ≥ 2, so the upper pointer is also 32-byte aligned).
/// - `lower.len() == upper.len()` and both are multiples of 2.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]
unsafe fn sqg_kernel_avx2(
    lower: &mut [Complex64],
    upper: &mut [Complex64],
    u00: Complex64,
    u01: Complex64,
    u10: Complex64,
    u11: Complex64,
) {
    use std::arch::x86_64::*;
    // SAFETY: avx2+fma enabled; caller guarantees alignment and length.
    unsafe {
        // Broadcast scalar components.
        let v00_re = _mm256_set1_pd(u00.re);
        let v00_im = _mm256_set1_pd(u00.im);
        let v01_re = _mm256_set1_pd(u01.re);
        let v01_im = _mm256_set1_pd(u01.im);
        let v10_re = _mm256_set1_pd(u10.re);
        let v10_im = _mm256_set1_pd(u10.im);
        let v11_re = _mm256_set1_pd(u11.re);
        let v11_im = _mm256_set1_pd(u11.im);

        let n = lower.len(); // number of Complex64 elements
        let mut i = 0;

        // Main loop: 2 Complex64 per iteration (= 4 f64 = 1 × __m256d).
        while i + 2 <= n {
            let a_ptr = lower.as_ptr().add(i) as *const f64;
            let b_ptr = upper.as_ptr().add(i) as *const f64;

            // u00·α + u01·β
            let ua = cmul2_avx2(a_ptr, v00_re, v00_im);
            let ub = cmul2_avx2(b_ptr, v01_re, v01_im);
            let new_alpha = _mm256_add_pd(ua, ub);

            // u10·α + u11·β
            let wa = cmul2_avx2(a_ptr, v10_re, v10_im);
            let wb = cmul2_avx2(b_ptr, v11_re, v11_im);
            let new_beta = _mm256_add_pd(wa, wb);

            _mm256_store_pd(lower.as_mut_ptr().add(i) as *mut f64, new_alpha);
            _mm256_store_pd(upper.as_mut_ptr().add(i) as *mut f64, new_beta);

            i += 2;
        }

        // Scalar tail (handles dist == 1, i.e., qubit 0).
        while i < n {
            let a = lower[i];
            let b = upper[i];
            lower[i] = u00 * a + u01 * b;
            upper[i] = u10 * a + u11 * b;
            i += 1;
        }
    }
}

/// Scalar fallback for one chunk of a single-qubit gate.
#[inline(always)]
fn sqg_kernel_scalar(
    lower: &mut [Complex64],
    upper: &mut [Complex64],
    u00: Complex64,
    u01: Complex64,
    u10: Complex64,
    u11: Complex64,
) {
    lower
        .iter_mut()
        .zip(upper.iter_mut())
        .for_each(|(alpha, beta)| {
            let a = *alpha;
            let b = *beta;
            *alpha = u00 * a + u01 * b;
            *beta = u10 * a + u11 * b;
        });
}

/// Runtime-dispatched single-qubit gate kernel.
///
/// Dispatches to AVX2 (x86_64) or scalar. On AArch64, LLVM auto-vectorizes
/// the scalar path using NEON, which is sufficient for interleaved Complex64.
///
/// # Safety
/// `lower.len() == upper.len()` must hold; `lower.as_ptr()` must be 32-byte
/// aligned (guaranteed by `AlignedBuffer` + power-of-2 `dist` offset).
#[inline]
#[allow(unreachable_code)] // scalar path unreachable when AVX2 always taken
fn sqg_kernel(
    lower: &mut [Complex64],
    upper: &mut [Complex64],
    u00: Complex64,
    u01: Complex64,
    u10: Complex64,
    u11: Complex64,
) {
    #[cfg(target_arch = "x86_64")]
    if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
        // SAFETY: avx2+fma confirmed; AlignedBuffer guarantees 64-byte base
        // alignment; dist is always a power of 2, so upper ptr is also aligned.
        return unsafe { sqg_kernel_avx2(lower, upper, u00, u01, u10, u11) };
    }
    sqg_kernel_scalar(lower, upper, u00, u01, u10, u11);
}

/// Threshold for parallel execution (in number of qubits).
///
/// For small systems, the overhead of thread pool scheduling exceeds
/// the performance gains from parallelization. Systems with fewer than
/// 2^14 = 16384 amplitudes use serial execution.
const PARALLEL_THRESHOLD: usize = 14;

/// Conditionally executes serial or parallel iteration based on qubit count.
///
/// Switches between Rayon parallel iteration and standard serial iteration
/// to avoid thread pool overhead for small quantum states.
///
/// # Arguments
/// * `$num_qubits` - Number of qubits (determines execution mode)
/// * `$data` - Mutable slice to process
/// * `$chunk_size` - Size of each chunk for processing
/// * `$body` - Closure to apply to each chunk
///
/// # Example
/// ```rust,ignore
/// with_maybe_par!(num_qubits, data, chunk_size, |chunk| {
///     for elem in chunk { *elem *= 2.0; }
/// });
/// ```
macro_rules! with_maybe_par {
    ($num_qubits:expr, $data:expr, $chunk_size:expr, $body:expr) => {{
        use rayon::prelude::*;
        use std::iter::Iterator;

        if $num_qubits < PARALLEL_THRESHOLD {
            $data.chunks_exact_mut($chunk_size).for_each($body);
        } else {
            $data.par_chunks_exact_mut($chunk_size).for_each($body);
        }
    }};
}

/// Quantum statevector representing a pure quantum state.
///
/// A statevector describes the quantum state of an N-qubit system as a vector
/// of 2^N complex amplitudes. The state |ψ⟩ = Σᵢ αᵢ|i⟩ is stored with αᵢ
/// as the amplitude for basis state |i⟩ (i in binary representation).
///
/// # Memory Layout
/// The `data` vector uses contiguous memory layout (compatible with C/NumPy),
/// where the amplitude at index `i` corresponds to basis state |i⟩ with
/// qubit indices mapping to bits from least significant (qubit 0) to most.
///
/// # Memory Layout
///
/// The `data` buffer uses a 64-byte aligned allocation so that SIMD kernels
/// (AVX2, SSE2) can safely use aligned load/store instructions.
///
/// # Example
/// ```rust
/// use cqlib_core::qis::Statevector;
///
/// // Create a 2-qubit state in |00⟩
/// let sv = Statevector::new(2);
/// assert_eq!(sv.num_qubits, 2);
/// assert_eq!(sv.data().len(), 4); // 2^2 amplitudes
///
/// // |00⟩ state: amplitude 1.0 at index 0, 0.0 elsewhere
/// assert_eq!(sv.data()[0], num_complex::Complex64::new(1.0, 0.0));
/// ```
#[derive(Debug, Clone)]
pub struct Statevector {
    /// Complex amplitudes for each basis state. Length is 2^N where N = num_qubits.
    /// 64-byte aligned for SIMD compatibility (see [`AlignedBuffer`]).
    data: AlignedBuffer<Complex64>,
    /// Number of qubits in the system.
    pub num_qubits: usize,
}

impl Statevector {
    /// Returns the complex amplitudes as a shared slice.
    ///
    /// Length is `2^num_qubits`. The amplitude at index `i` is the coefficient of basis
    /// state |i⟩ in the computational basis.
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::qis::Statevector;
    ///
    /// let sv = Statevector::new(2); // |00⟩
    /// assert_eq!(sv.data().len(), 4);
    /// assert_eq!(sv.data()[0], num_complex::Complex64::new(1.0, 0.0));
    /// ```
    pub fn data(&self) -> &[Complex64] {
        &self.data
    }

    /// Returns the complex amplitudes as a mutable slice.
    ///
    /// Mutating the slice can break normalization; callers are responsible for
    /// preserving a valid statevector.
    pub fn data_mut(&mut self) -> &mut [Complex64] {
        &mut self.data
    }

    /// Creates a new statevector initialized to the |0...0⟩ state.
    ///
    /// The statevector represents the quantum state as a vector of 2^N complex amplitudes,
    /// where N is the number of qubits. All amplitudes are initialized to zero except
    /// the first element (|0...0⟩) which is set to 1.0.
    ///
    /// # Arguments
    /// * `num_qubits` - Number of qubits in the system
    ///
    /// # Returns
    /// A new `Statevector` instance in the ground state
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::qis::Statevector;
    ///
    /// let sv = Statevector::new(2); // |00⟩ state
    /// assert_eq!(sv.num_qubits, 2);
    /// assert_eq!(sv.data().len(), 4); // 2^2 amplitudes
    /// ```
    pub fn new(num_qubits: usize) -> Self {
        let size = 1 << num_qubits;
        // 64-byte aligned allocation — zero-initialised (all amplitudes = 0+0i).
        let mut data = AlignedBuffer::<Complex64>::new_zeroed(size);
        data[0] = Complex64::new(1.0, 0.0); // |0...0⟩
        Statevector { data, num_qubits }
    }

    /// Creates a statevector from initial amplitudes with normalization check.
    ///
    /// # Arguments
    /// * `num_qubits` - Number of qubits in the system
    /// * `initial_state` - Vector of 2^N complex amplitudes
    ///
    /// # Errors
    /// Returns an error if:
    /// - `initial_state` length doesn't match 2^num_qubits
    /// - State is not normalized (sum of probabilities ≠ 1.0 within a dimension-scaled tolerance)
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::qis::Statevector;
    /// use num_complex::Complex64;
    ///
    /// // Create |+0⟩ = (|00⟩ + |10⟩)/√2
    /// let amps = vec![
    ///     Complex64::new(std::f64::consts::FRAC_1_SQRT_2, 0.0), // |00⟩
    ///     Complex64::new(0.0, 0.0),                             // |01⟩
    ///     Complex64::new(std::f64::consts::FRAC_1_SQRT_2, 0.0), // |10⟩
    ///     Complex64::new(0.0, 0.0),                             // |11⟩
    /// ];
    /// let sv = Statevector::from_state(2, amps);
    /// ```
    pub fn from_state(num_qubits: usize, initial_state: Vec<Complex64>) -> Result<Self, QisError> {
        let size = 1 << num_qubits;
        if initial_state.len() != size {
            return Err(QisError::InvalidStateDimension(initial_state.len()));
        }
        // Verify normalization. The tolerance scales with vector length because
        // summing many amplitudes accumulates floating-point roundoff.
        let norm: f64 = initial_state.iter().map(|c| c.norm_sqr()).sum();
        let tolerance = 1e-10 * (size.max(1) as f64);
        if (norm - 1.0).abs() >= tolerance {
            return Err(QisError::NotNormalized);
        }

        // Copy caller-provided Vec into 64-byte aligned buffer.
        let mut data = AlignedBuffer::<Complex64>::new_zeroed(size);
        data.as_mut_slice().copy_from_slice(&initial_state);
        Ok(Statevector { data, num_qubits })
    }

    /// Constructs a statevector by simulating a quantum circuit.
    ///
    /// Executes the circuit gates sequentially to evolve the initial |0...0⟩ state.
    /// Complex gates are decomposed before execution.
    ///
    /// # Arguments
    /// * `circuit` - The quantum circuit to simulate
    ///
    /// # Returns
    /// * `Ok(Statevector)` - The resulting state after circuit execution
    /// * `Err(QisError)` - If the circuit contains unsupported operations
    ///   (control flow gates, symbolic parameters) or decomposition fails
    ///
    /// # Supported Instructions
    /// - Standard single and multi-qubit gates
    /// - Controlled gates (CX, CY, CZ, CRX, CRY, CRZ)
    /// - Multi-controlled gates (CCX/Toffoli)
    /// - Unitary gates with matrix representation
    /// - Barriers and delays (ignored)
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::circuit::{Circuit, Qubit};
    /// use cqlib_core::qis::Statevector;
    ///
    /// // Create Bell state: |Φ+⟩ = (|00⟩ + |11⟩)/√2
    /// let mut circuit = Circuit::new(2);
    /// circuit.h(Qubit::new(0)).unwrap();
    /// circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    ///
    /// let sv = Statevector::from_circuit(&circuit).unwrap();
    /// ```
    pub fn from_circuit(circuit: &Circuit) -> Result<Self, QisError> {
        let mut sv = Statevector::new(circuit.num_qubits());
        sv.apply_circuit(circuit)?;
        Ok(sv)
    }

    /// Applies a quantum circuit to this statevector in-place.
    ///
    /// The circuit is first decomposed into basic gates via [`Circuit::decompose`],
    /// then each operation is applied sequentially to the current state.
    ///
    /// # Arguments
    /// * `circuit` - The circuit to apply. Must have the same number of qubits
    ///   as this statevector.
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
        if circuit.num_qubits() != self.num_qubits {
            return Err(QisError::InvalidStateDimension(circuit.num_qubits()));
        }
        if circuit
            .operations()
            .iter()
            .any(|op| matches!(op.instruction, Instruction::ClassicalControl(_)))
        {
            return Err(QisError::UnsupportedOperation(
                "classical control flow is not supported in statevector simulation".to_string(),
            ));
        }
        // Decompose circuit to basic gates
        let circuit = circuit.decompose()?;
        let sv = self;

        // Build qubit index mapping: Qubit -> physical index
        let qubits = circuit.qubits();
        let qubit_map: std::collections::HashMap<_, _> = qubits
            .iter()
            .enumerate()
            .map(|(idx, q)| (*q, idx))
            .collect();

        // Precompute all parameter values
        let parameter_values: Vec<Option<f64>> = circuit
            .parameters()
            .iter()
            .map(|p| p.evaluate(&None).ok())
            .collect();

        for op in circuit.operations() {
            // Resolve parameters using precomputed values
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

            // Get physical qubit indices
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
                    sv.apply_standard_gate(*gate, &qubit_indices, &params)?;
                }
                Instruction::McGate(mc_gate) => {
                    let num_controls = mc_gate.num_ctrl_qubits();
                    let base_gate = mc_gate.base_gate();

                    if num_controls == 1 {
                        // Single-controlled gate
                        let control = qubit_indices[0];
                        let target = qubit_indices[1];
                        match base_gate {
                            StandardGate::X => sv.apply_cx(control, target)?,
                            StandardGate::Y => sv.apply_cy(control, target)?,
                            StandardGate::Z => sv.apply_cz(control, target)?,
                            StandardGate::RX => {
                                let theta = params[0];
                                sv.apply_crx(control, target, theta)?;
                            }
                            StandardGate::RY => {
                                let theta = params[0];
                                sv.apply_cry(control, target, theta)?;
                            }
                            StandardGate::RZ => {
                                let theta = params[0];
                                sv.apply_crz(control, target, theta)?;
                            }
                            _ => {
                                let matrix = mc_gate.matrix(&params).map_err(|_| {
                                    QisError::CircuitError(CircuitError::NoMatrixRepresentation)
                                })?;
                                sv.apply_unitary_gate(&qubit_indices, &matrix)?;
                            }
                        }
                    } else if num_controls == 2 && *base_gate == StandardGate::X {
                        // Toffoli gate
                        let c0 = qubit_indices[0];
                        let c1 = qubit_indices[1];
                        let target = qubit_indices[2];
                        sv.apply_ccx(c0, c1, target)?;
                    } else {
                        let matrix = mc_gate.matrix(&params).map_err(|_| {
                            QisError::CircuitError(CircuitError::NoMatrixRepresentation)
                        })?;
                        sv.apply_unitary_gate(&qubit_indices, &matrix)?;
                    }
                }
                Instruction::UnitaryGate(u_gate) => {
                    if let Some(matrix) = u_gate.matrix() {
                        sv.apply_unitary_gate(&qubit_indices, matrix)?;
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
                            "legacy Measure directive is not supported by statevector circuit evolution; use Circuit::measure as an output declaration".to_string(),
                        ));
                    }
                    Directive::Reset => {
                        let qubit = qubit_indices[0];
                        sv.reset(qubit)?;
                    }
                },
                Instruction::Delay => {
                    // Ignore delays
                    continue;
                }
                Instruction::ClassicalControl(_) => {
                    return Err(QisError::UnsupportedOperation(
                        "classical control flow is not supported in statevector simulation"
                            .to_string(),
                    ));
                }
                Instruction::ClassicalData(_) => {}
            }
        }

        Ok(())
    }

    /// Applies a standard gate to the statevector.
    ///
    /// Internal dispatcher that routes to specialized implementations
    /// based on gate type.
    ///
    /// # Arguments
    /// * `gate` - The standard gate to apply
    /// * `qubits` - Target qubit indices
    /// * `params` - Gate parameters (for parameterized gates)
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
            // Single-qubit gates
            StandardGate::I => { /* no-op */ }
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
            StandardGate::U => {
                self.apply_u(qubits[0], params[0], params[1], params[2])?;
            }
            StandardGate::GPhase => self.apply_gphase(params[0])?,

            // Two-qubit gates
            StandardGate::CX => self.apply_cx(qubits[0], qubits[1])?,
            StandardGate::CY => self.apply_cy(qubits[0], qubits[1])?,
            StandardGate::CZ => self.apply_cz(qubits[0], qubits[1])?,
            StandardGate::SWAP => self.apply_swap(qubits[0], qubits[1])?,
            StandardGate::RXX => self.apply_rxx(qubits[0], qubits[1], params[0])?,
            StandardGate::RYY => self.apply_ryy(qubits[0], qubits[1], params[0])?,
            StandardGate::RZZ => self.apply_rzz(qubits[0], qubits[1], params[0])?,
            StandardGate::RZX => self.apply_rzx(qubits[0], qubits[1], params[0])?,

            // Controlled rotation gates
            StandardGate::CRX => self.apply_crx(qubits[0], qubits[1], params[0])?,
            StandardGate::CRY => self.apply_cry(qubits[0], qubits[1], params[0])?,
            StandardGate::CRZ => self.apply_crz(qubits[0], qubits[1], params[0])?,

            // Three-qubit gates
            StandardGate::CCX => self.apply_ccx(qubits[0], qubits[1], qubits[2])?,

            // Simulator gates
            StandardGate::FSIM => self.apply_fsim(qubits[0], qubits[1], params[0], params[1])?,
        }
        Ok(())
    }

    /// Computes the measurement probability distribution over all basis states.
    ///
    /// Returns P(|i⟩) = |αᵢ|² for each basis state |i⟩.
    /// Uses parallel iteration for large statevectors.
    ///
    /// # Returns
    /// Vector of probabilities summing to 1.0 (within floating-point precision).
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::qis::Statevector;
    ///
    /// let mut sv = Statevector::new(1);
    /// sv.apply_h(0);
    /// let probs = sv.probabilities();
    /// // |+⟩ state: P(|0⟩) = P(|1⟩) = 0.5
    /// assert!((probs[0] - 0.5).abs() < 1e-10);
    /// assert!((probs[1] - 0.5).abs() < 1e-10);
    /// ```
    pub fn probabilities(&self) -> Vec<f64> {
        if self.num_qubits < PARALLEL_THRESHOLD {
            self.data.iter().map(|c| c.norm_sqr()).collect()
        } else {
            self.data.par_iter().map(|c| c.norm_sqr()).collect()
        }
    }

    /// Validates that a qubit index is within bounds.
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

    /// Validates that two qubit indices are within bounds and distinct.
    fn validate_two_qubits(&self, q0: usize, q1: usize) -> Result<(), QisError> {
        self.validate_qubit(q0)?;
        self.validate_qubit(q1)?;
        if q0 == q1 {
            return Err(QisError::InvalidParameterValue(
                "Qubit indices must be distinct".to_string(),
            ));
        }
        Ok(())
    }

    /// Validates that all qubit indices are within bounds and distinct.
    fn validate_qubits(&self, qubits: &[usize]) -> Result<(), QisError> {
        for (i, &q) in qubits.iter().enumerate() {
            self.validate_qubit(q)?;
            for &other in &qubits[..i] {
                if q == other {
                    return Err(QisError::InvalidParameterValue(format!(
                        "Duplicate qubit index {} in gate operation",
                        q
                    )));
                }
            }
        }
        Ok(())
    }

    /// Applies an arbitrary single-qubit gate.
    ///
    /// General-purpose method for applying any 2x2 unitary matrix.
    /// For standard gates, prefer specialized methods (apply_h, apply_x, etc.)
    /// which have optimized implementations.
    ///
    /// # Arguments
    /// * `qubit` - Target qubit index
    /// * `matrix` - 2x2 unitary matrix [[u00, u01], [u10, u11]]
    ///
    /// # Errors
    /// Returns [`QisError::IndexOutOfBounds`] if `qubit` is out of bounds.
    ///
    /// # Example
    /// ```rust,ignore
    /// use cqlib_core::qis::Statevector;
    /// use num_complex::Complex64;
    ///
    /// let mut sv = Statevector::new(1);
    /// // Apply a custom rotation
    /// let matrix = [
    ///     [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
    ///     [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
    /// ];
    /// sv.apply_single_qubit_gate(0, matrix); // Equivalent to X gate
    /// ```
    pub fn apply_single_qubit_gate(
        &mut self,
        qubit: usize,
        matrix: [[Complex64; 2]; 2],
    ) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;

        // Preload matrix elements to registers, reducing memory access in loop
        let u00 = matrix[0][0];
        let u01 = matrix[0][1];
        let u10 = matrix[1][0];
        let u11 = matrix[1][1];

        // Dispatch: AVX2 SIMD on x86_64 (2 Complex64/cycle), scalar elsewhere.
        // The SIMD path uses `_mm256_addsub_pd` to compute (ac−bd, ad+bc) for
        // two complex pairs per 256-bit register, bypassing LLVM's inability
        // to auto-vectorize interleaved re/im complex multiplication.
        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            sqg_kernel(lower, upper, u00, u01, u10, u11);
        };

        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply a general two-qubit gate.
    ///
    /// # Arguments
    /// * `q0` - First qubit index (logical)
    /// * `q1` - Second qubit index (logical)
    /// * `matrix` - 4x4 unitary matrix in the standard computational basis
    ///   ordered as |00⟩, |01⟩, |10⟩, |11⟩ where the bits are (q0, q1).
    ///   That is, matrix[i][j] is the amplitude of |i⟩ when input is |j⟩,
    ///   with i = q0*2 + q1 (binary: q0 is MSB, q1 is LSB).
    ///
    /// # Example - CNOT with q0 as control and q1 as target:
    /// The CNOT gate flips the target when control is |1⟩:
    /// |00⟩ -> |00⟩, |01⟩ -> |01⟩, |10⟩ -> |11⟩, |11⟩ -> |10⟩
    ///
    /// This corresponds to the matrix (rows/cols ordered as |00⟩, |01⟩, |10⟩, |11⟩):
    /// ```text
    /// [[1, 0, 0, 0],
    ///  [0, 1, 0, 0],
    ///  [0, 0, 0, 1],
    ///  [0, 0, 1, 0]]
    /// ```
    pub fn apply_two_qubit_gate(
        &mut self,
        q0: usize,
        q1: usize,
        matrix: [[Complex64; 4]; 4],
    ) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;

        // 1. Identify physical high/low bit positions in memory
        // q_max determines the outer parallel chunk size
        // q_min determines the inner serial stride
        let (q_min, q_max) = if q0 < q1 { (q0, q1) } else { (q1, q0) };
        let dist_max = 1 << q_max;
        let dist_min = 1 << q_min;

        // 2. Preload matrix elements to registers, avoiding array access in hot loop
        // Naming: m_row_col corresponds to logical state |q0 q1>
        let m00 = matrix[0][0];
        let m01 = matrix[0][1];
        let m02 = matrix[0][2];
        let m03 = matrix[0][3];
        let m10 = matrix[1][0];
        let m11 = matrix[1][1];
        let m12 = matrix[1][2];
        let m13 = matrix[1][3];
        let m20 = matrix[2][0];
        let m21 = matrix[2][1];
        let m22 = matrix[2][2];
        let m23 = matrix[2][3];
        let m30 = matrix[3][0];
        let m31 = matrix[3][1];
        let m32 = matrix[3][2];
        let m33 = matrix[3][3];

        // 3. Choose serial or parallel based on qubit count
        // Split statevector into chunks by max_qubit
        // Each chunk contains 2 * dist_max elements
        // Layout: [ ...dist_max (q_max=0)... | ...dist_max (q_max=1)... ]
        let kernel = |chunk: &mut [Complex64]| {
            // Split chunk into q_max=0 and q_max=1 parts
            let (part0, part1) = chunk.split_at_mut(dist_max);

            // 4. Inner serial traversal
            // Further split by q_min within each part
            // Use zip to traverse both parts simultaneously for optimal bounds check elision
            part0
                .chunks_exact_mut(2 * dist_min)
                .zip(part1.chunks_exact_mut(2 * dist_min))
                .for_each(|(sub_chunk0, sub_chunk1)| {
                    // sub_chunk0: q_max=0 region
                    // sub_chunk1: q_max=1 region

                    // Further split by q_min
                    let (v00_slice, v01_slice) = sub_chunk0.split_at_mut(dist_min);
                    let (v10_slice, v11_slice) = sub_chunk1.split_at_mut(dist_min);

                    // Now we have four slices corresponding to physical bit states:
                    // v00_slice: q_max=0, q_min=0
                    // v01_slice: q_max=0, q_min=1
                    // v10_slice: q_max=1, q_min=0
                    // v11_slice: q_max=1, q_min=1

                    // 5. Core computation loop
                    // Use iterator zip to avoid index calculations, enabling SIMD
                    for (((c00, c01), c10), c11) in v00_slice
                        .iter_mut()
                        .zip(v01_slice.iter_mut())
                        .zip(v10_slice.iter_mut())
                        .zip(v11_slice.iter_mut())
                    {
                        let a = *c00; // Phys: 00
                        let b = *c01; // Phys: 01
                        let c = *c10; // Phys: 10
                        let d = *c11; // Phys: 11

                        // 6. Logical mapping: determine input vector positions based on q0 vs q1
                        // logical_amp_00 corresponds to |q0=0, q1=0>
                        // logical_amp_01 corresponds to |q0=0, q1=1> ...
                        let (in0, in1, in2, in3) = if q0 < q1 {
                            // q0=min, q1=max
                            // Phys 00 (min=0, max=0) -> Log 00
                            // Phys 01 (min=1, max=0) -> Log 10 (q0=1, q1=0)
                            // Phys 10 (min=0, max=1) -> Log 01 (q0=0, q1=1)
                            // Phys 11 (min=1, max=1) -> Log 11
                            (a, c, b, d)
                        } else {
                            // q0=max, q1=min
                            // Phys 00 (min=0, max=0) -> Log 00
                            // Phys 01 (min=1, max=0) -> Log 01 (q0=0, q1=1)
                            // Phys 10 (min=0, max=1) -> Log 10 (q0=1, q1=0)
                            // Phys 11 (min=1, max=1) -> Log 11
                            (a, b, c, d)
                        };

                        // Matrix multiplication
                        let out0 = m00 * in0 + m01 * in1 + m02 * in2 + m03 * in3;
                        let out1 = m10 * in0 + m11 * in1 + m12 * in2 + m13 * in3;
                        let out2 = m20 * in0 + m21 * in1 + m22 * in2 + m23 * in3;
                        let out3 = m30 * in0 + m31 * in1 + m32 * in2 + m33 * in3;

                        // Write results (inverse mapping)
                        if q0 < q1 {
                            *c00 = out0;
                            *c10 = out1; // Map Log 01 to Phys 10 (Log 01: q0=0, q1=1 => min=0, max=1 => Phys 10)
                            *c01 = out2; // Map Log 10 to Phys 01 (q0=1, q1=0 => min=1, max=0 => Phys 01)
                            *c11 = out3;
                        } else {
                            *c00 = out0;
                            *c01 = out1;
                            *c10 = out2;
                            *c11 = out3;
                        }
                    }
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist_max, kernel);
        Ok(())
    }

    /// Applies an arbitrary n-qubit unitary gate.
    ///
    /// General-purpose method for applying any 2^n × 2^n unitary matrix.
    /// Uses block-iteration with bit-manipulation for efficient index calculation.
    /// Supports parallel execution for large statevectors.
    ///
    /// For standard gates, prefer specialized methods which are more optimized.
    ///
    /// # Arguments
    /// * `qubits` - Slice of qubit indices the gate acts on (logical order)
    /// * `matrix` - The unitary matrix as a 2^n × 2^n ndarray
    ///
    /// # Errors
    /// Returns an error if:
    /// - Qubit indices are out of bounds
    /// - Qubit indices contain duplicates
    /// - Matrix dimensions don't match 2^n × 2^n
    ///
    /// # Example
    /// ```rust,ignore
    /// use cqlib_core::qis::Statevector;
    /// use ndarray::Array2;
    /// use num_complex::Complex64;
    ///
    /// let mut sv = Statevector::new(2);
    /// // Create SWAP gate matrix
    /// let swap = Array2::from_shape_vec(
    ///     (4, 4),
    ///     vec![
    ///         Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0),
    ///         Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0),
    ///         Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0),
    ///         Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0),
    ///         Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0),
    ///         Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0),
    ///         Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0),
    ///         Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0),
    ///     ]
    /// ).unwrap();
    /// sv.apply_unitary_gate(&[0, 1], &swap);
    /// ```
    pub fn apply_unitary_gate(
        &mut self,
        qubits: &[usize],
        matrix: &ndarray::Array2<Complex64>,
    ) -> Result<(), QisError> {
        let num_target_qubits = qubits.len();
        let gate_dim = 1 << num_target_qubits;

        // Validate inputs
        if num_target_qubits == 0 || num_target_qubits > self.num_qubits {
            return Err(QisError::InvalidParameterValue(format!(
                "Invalid number of target qubits: {} (must be 1 to {})",
                num_target_qubits, self.num_qubits
            )));
        }

        for (i, &q) in qubits.iter().enumerate() {
            self.validate_qubit(q)?;
            for &other in &qubits[..i] {
                if q == other {
                    return Err(QisError::InvalidParameterValue(format!(
                        "Duplicate qubit index {} in apply_unitary",
                        q
                    )));
                }
            }
        }

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

        // Precompute gate offset mapping: gate_index -> physical_offset
        let mut gate_offsets: Vec<usize> = vec![0; gate_dim];
        #[allow(clippy::needless_range_loop)]
        for gate_idx in 0..gate_dim {
            let mut offset = 0;
            for (i, &qubit) in qubits.iter().enumerate() {
                // Big-endian mapping: qubits[0] corresponds to the MSB of the gate index,
                // qubits[last] corresponds to the LSB. This aligns with standard tensor
                // product order where the first qubit in the list is the most significant.
                let bit_pos = num_target_qubits - 1 - i;
                if (gate_idx >> bit_pos) & 1 == 1 {
                    offset |= 1 << qubit;
                }
            }
            gate_offsets[gate_idx] = offset;
        }

        // Sort qubits for efficient index calculation
        let mut sorted_qubits: Vec<usize> = qubits.to_vec();
        sorted_qubits.sort_unstable();

        // Calculate chunk size based on max qubit
        let max_qubit = *sorted_qubits.last().unwrap();
        let chunk_size = 1 << (max_qubit + 1);

        // Create matrix view for cache efficiency
        let matrix_rows: Vec<Vec<Complex64>> = (0..gate_dim)
            .map(|r| (0..gate_dim).map(|c| matrix[[r, c]]).collect())
            .collect();

        // Process chunks
        let kernel = |chunk: &mut [Complex64]| {
            let mut input_buf = vec![Complex64::default(); gate_dim];
            let mut output_buf = vec![Complex64::default(); gate_dim];

            // Number of blocks in this chunk
            let num_blocks = chunk.len() >> num_target_qubits;

            for block_idx in 0..num_blocks {
                // Expand compressed index to physical address
                let mut base = block_idx;
                for &q in &sorted_qubits {
                    let mask = (1 << q) - 1;
                    let left = (base & !mask) << 1;
                    let right = base & mask;
                    base = left | right;
                }

                // Load input amplitudes
                for (gate_idx, &offset) in gate_offsets.iter().enumerate() {
                    input_buf[gate_idx] = chunk[base + offset];
                }

                // Apply matrix multiplication
                for row in 0..gate_dim {
                    let mut sum = Complex64::default();
                    for (matrix_value, input_value) in matrix_rows[row].iter().zip(input_buf.iter())
                    {
                        sum += matrix_value * input_value;
                    }
                    output_buf[row] = sum;
                }

                // Store output amplitudes
                for (gate_idx, &offset) in gate_offsets.iter().enumerate() {
                    chunk[base + offset] = output_buf[gate_idx];
                }
            }
        };

        with_maybe_par!(self.num_qubits, self.data, chunk_size, kernel);
        Ok(())
    }

    /// Applies a native Pauli-string rotation in place.
    ///
    /// The operation is
    ///
    /// `exp(-i * theta / 2 * P)`
    ///
    /// where `P` is a Hermitian [`PauliString`]. This fused kernel avoids
    /// decomposing a Pauli exponential into basis changes, a CNOT ladder, and
    /// an RZ gate. It is intended for high-frequency variational workloads.
    ///
    /// # Errors
    ///
    /// Returns an error if the Pauli string has a different number of qubits or
    /// carries an imaginary global phase (and is therefore not Hermitian).
    pub fn apply_pauli_rotation(
        &mut self,
        pauli: &PauliString,
        theta: f64,
    ) -> Result<(), QisError> {
        if pauli.num_qubits != self.num_qubits {
            return Err(QisError::QubitMismatch {
                expected: self.num_qubits,
                actual: pauli.num_qubits,
            });
        }
        if matches!(pauli.phase, Phase::I | Phase::MinusI) {
            return Err(QisError::NotHermitian);
        }
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        let x_mask = pauli.x_mask();
        let z_mask = pauli.z_mask();
        let base_factor = pauli.phase.to_complex() * pauli.y_phase();
        let half = theta * 0.5;
        let cosine = half.cos();
        let minus_i_sine = Complex64::new(0.0, -half.sin());

        #[inline(always)]
        fn source_phase(index: usize, z_mask: usize, base: Complex64) -> Complex64 {
            if (index & z_mask).count_ones() & 1 == 0 {
                base
            } else {
                -base
            }
        }

        if x_mask == 0 {
            if self.num_qubits < PARALLEL_THRESHOLD {
                self.data
                    .iter_mut()
                    .enumerate()
                    .for_each(|(index, amplitude)| {
                        let phase = source_phase(index, z_mask, base_factor);
                        *amplitude *= Complex64::new(cosine, 0.0) + minus_i_sine * phase;
                    });
            } else {
                self.data
                    .par_iter_mut()
                    .enumerate()
                    .for_each(|(index, amplitude)| {
                        let phase = source_phase(index, z_mask, base_factor);
                        *amplitude *= Complex64::new(cosine, 0.0) + minus_i_sine * phase;
                    });
            }
            return Ok(());
        }

        // Split on the highest flipped bit. Every Pauli pair then contains one
        // amplitude in the lower half and one in the upper half of a chunk, so
        // disjoint pairs can be updated safely without an auxiliary statevector.
        let highest_bit = usize::BITS as usize - 1 - x_mask.leading_zeros() as usize;
        let dist = 1usize << highest_bit;
        let chunk_size = 2 * dist;
        let low_flip_mask = x_mask ^ dist;

        let apply_chunk = |chunk_index: usize, chunk: &mut [Complex64]| {
            let base_index = chunk_index * chunk_size;
            for lower_index in 0..dist {
                let upper_index = dist + (lower_index ^ low_flip_mask);
                let source_lower = base_index + lower_index;
                let source_upper = base_index + upper_index;
                let lower_phase = source_phase(source_lower, z_mask, base_factor);
                let upper_phase = source_phase(source_upper, z_mask, base_factor);
                let lower = chunk[lower_index];
                let upper = chunk[upper_index];
                chunk[lower_index] = cosine * lower + minus_i_sine * upper_phase * upper;
                chunk[upper_index] = cosine * upper + minus_i_sine * lower_phase * lower;
            }
        };

        if self.num_qubits < PARALLEL_THRESHOLD {
            self.data
                .chunks_mut(chunk_size)
                .enumerate()
                .for_each(|(chunk_index, chunk)| apply_chunk(chunk_index, chunk));
        } else {
            self.data
                .par_chunks_mut(chunk_size)
                .enumerate()
                .for_each(|(chunk_index, chunk)| apply_chunk(chunk_index, chunk));
        }
        Ok(())
    }

    /// Apply Pauli-X gate: bit flip
    /// Matrix: [[0, 1], [1, 0]]
    /// Cost: 0 muls, 0 adds, just memory swap.
    pub fn apply_x(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower.iter_mut().zip(upper.iter_mut()).for_each(|(a, b)| {
                std::mem::swap(a, b);
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply Pauli-Y gate
    /// Matrix: [[0, -i], [i, 0]]
    /// Cost: Manual component swapping (faster than complex mul).
    pub fn apply_y(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;
                    *alpha = Complex64::new(b.im, -b.re);
                    *beta = Complex64::new(-a.im, a.re);
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply Pauli-Z gate: phase flip
    /// Matrix: [[1, 0], [0, -1]]
    /// Cost: Only processes half the array. 1 negation.
    pub fn apply_z(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let kernel = |chunk: &mut [Complex64]| {
            let (_lower, upper) = chunk.split_at_mut(dist);
            upper.iter_mut().for_each(|val| {
                *val = -*val;
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply Hadamard gate
    /// Matrix: 1/sqrt(2) * [[1, 1], [1, -1]]
    /// Creates superposition: H|0⟩ = (|0⟩ + |1⟩)/√2, H|1⟩ = (|0⟩ - |1⟩)/√2
    pub fn apply_h(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let k = FRAC_1_SQRT_2;

        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;
                    *alpha = k * (a + b);
                    *beta = k * (a - b);
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply U gate (generic single-qubit rotation)
    /// Matrix: [[cos(θ/2), -e^(iλ)sin(θ/2)], [e^(iφ)sin(θ/2), e^(i(φ+λ))cos(θ/2)]]
    ///
    /// This is the most general single-qubit unitary (up to global phase).
    /// Parameters:
    ///   theta: rotation angle
    ///   phi: phase of the rotation axis
    ///   lambda: second phase parameter
    pub fn apply_u(
        &mut self,
        qubit: usize,
        theta: f64,
        phi: f64,
        lambda: f64,
    ) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;

        // Fast path: if theta is 0, U simplifies to a Z-rotation-like operation
        if theta.abs() < 1e-15 {
            // U(0, φ, λ) = e^(i(φ+λ)/2) * [[1, 0], [0, e^(i(λ+φ))]]
            // This is essentially a phase gate plus global phase
            return self.apply_phase(qubit, lambda + phi);
        }

        let dist = 1 << qubit;

        let half_theta = theta * 0.5;
        let c = half_theta.cos();
        let s = half_theta.sin();

        // Compute phase factors
        let phase_phi = Complex64::from_polar(1.0, phi); // e^(iφ)
        let phase_lambda = Complex64::from_polar(1.0, lambda); // e^(iλ)
        let phase_phi_lambda = Complex64::from_polar(1.0, phi + lambda); // e^(i(φ+λ))

        // Matrix elements:
        // u00 = cos(θ/2)
        // u01 = -e^(iλ) * sin(θ/2)
        // u10 = e^(iφ) * sin(θ/2)
        // u11 = e^(i(φ+λ)) * cos(θ/2)
        let u00 = Complex64::new(c, 0.0);
        let u01 = -phase_lambda * s;
        let u10 = phase_phi * s;
        let u11 = phase_phi_lambda * c;

        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;
                    *alpha = u00 * a + u01 * b;
                    *beta = u10 * a + u11 * b;
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply S gate (Phase gate, Z^1/2)
    /// Matrix: [[1, 0], [0, i]]
    pub fn apply_s(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let kernel = |chunk: &mut [Complex64]| {
            let (_lower, upper) = chunk.split_at_mut(dist);
            upper.iter_mut().for_each(|val| {
                *val = Complex64::new(-val.im, val.re);
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply S† (S-dagger) gate
    /// Matrix: [[1, 0], [0, -i]]
    pub fn apply_sdg(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let kernel = |chunk: &mut [Complex64]| {
            let (_lower, upper) = chunk.split_at_mut(dist);
            upper.iter_mut().for_each(|val| {
                *val = Complex64::new(val.im, -val.re);
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply T gate (Z^1/4)
    /// Matrix: [[1, 0], [0, e^(i*pi/4)]]
    pub fn apply_t(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let phase = Complex64::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2);
        let kernel = |chunk: &mut [Complex64]| {
            let (_lower, upper) = chunk.split_at_mut(dist);
            upper.iter_mut().for_each(|val| {
                *val *= phase;
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply T† (T-dagger) gate
    /// Matrix: [[1, 0], [0, e^(-i*pi/4)]]
    pub fn apply_tdg(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let phase = Complex64::new(FRAC_1_SQRT_2, -FRAC_1_SQRT_2);
        let kernel = |chunk: &mut [Complex64]| {
            let (_lower, upper) = chunk.split_at_mut(dist);
            upper.iter_mut().for_each(|val| {
                *val *= phase;
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply P (Phase) gate
    /// Matrix: [[1, 0], [0, e^(i*theta)]]
    pub fn apply_phase(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        let dist = 1 << qubit;
        let phase = Complex64::from_polar(1.0, theta);
        let kernel = |chunk: &mut [Complex64]| {
            let (_lower, upper) = chunk.split_at_mut(dist);
            upper.iter_mut().for_each(|val| {
                *val *= phase;
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply Rx(theta) gate
    /// Matrix: [[cos(t/2), -i*sin(t/2)], [-i*sin(t/2), cos(t/2)]]
    pub fn apply_rx(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        let dist = 1 << qubit;
        let half_theta = theta * 0.5;
        let c = half_theta.cos();
        let s = half_theta.sin();

        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;
                    let bs_im = b.im * s;
                    let bs_re = b.re * s;
                    let as_im = a.im * s;
                    let as_re = a.re * s;

                    *alpha = Complex64::new(
                        a.re * c + bs_im, // Real part
                        a.im * c - bs_re, // Imag part
                    );

                    *beta = Complex64::new(
                        b.re * c + as_im, // Real part
                        b.im * c - as_re, // Imag part
                    );
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply Ry(theta) gate
    /// Matrix: [[cos(t/2), -sin(t/2)], [sin(t/2), cos(t/2)]]
    pub fn apply_ry(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        let dist = 1 << qubit;

        let half_theta = theta * 0.5;
        let c = half_theta.cos();
        let s = half_theta.sin();

        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;
                    *alpha = a * c - b * s;
                    *beta = a * s + b * c;
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply Rz(theta) gate
    /// Matrix: [[e^(-i*theta/2), 0], [0, e^(i*theta/2)]]
    pub fn apply_rz(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        let dist = 1 << qubit;

        // Precompute phase factors
        let half_theta = theta * 0.5;
        let phase_0 = Complex64::from_polar(1.0, -half_theta); // e^(-i*t/2)
        let phase_1 = Complex64::from_polar(1.0, half_theta); // e^(i*t/2)

        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower.iter_mut().zip(upper.iter_mut()).for_each(|(a, b)| {
                *a *= phase_0;
                *b *= phase_1;
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Applies X2P gate: Rx(π/2), also known as √X.
    ///
    /// Matrix: 1/√2 * [[1, -i], [-i, 1]]
    ///
    /// # Optimization
    /// Uses constant factors only, no trigonometric functions.
    pub fn apply_x2p(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let k = FRAC_1_SQRT_2;
        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;
                    *alpha = Complex64::new(k * (a.re + b.im), k * (a.im - b.re));
                    *beta = Complex64::new(k * (b.re + a.im), k * (b.im - a.re));
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Applies X2M gate: Rx(-π/2), inverse of √X.
    ///
    /// Matrix: 1/√2 * [[1, i], [i, 1]]
    pub fn apply_x2m(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let k = FRAC_1_SQRT_2;
        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;
                    *alpha = Complex64::new(k * (a.re - b.im), k * (a.im + b.re));
                    *beta = Complex64::new(k * (b.re - a.im), k * (b.im + a.re));
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Applies Y2P gate: Ry(π/2).
    ///
    /// Matrix: 1/√2 * [[1, -1], [1, 1]]
    pub fn apply_y2p(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let k = FRAC_1_SQRT_2;
        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;
                    *alpha = k * (a - b);
                    *beta = k * (a + b);
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Applies Y2M gate: Ry(-π/2).
    ///
    /// Matrix: 1/√2 * [[1, 1], [-1, 1]]
    pub fn apply_y2m(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let k = FRAC_1_SQRT_2;
        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;
                    *alpha = k * (a + b);
                    *beta = k * (b - a);
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Applies XY2P gate: Rz(θ - π/2) Ry(π/2) Rz(π/2 - θ).
    ///
    /// Matrix: 1/√2 * [[1, -i·e^(-iθ)], [-i·e^(iθ), 1]]
    ///
    /// # Arguments
    /// * `qubit` - Target qubit index
    /// * `theta` - Rotation angle
    pub fn apply_xy2p(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let k = FRAC_1_SQRT_2;

        // Precompute coefficients, fuse k to reduce multiplications in loop
        // e^(i*theta)
        let phase = Complex64::from_polar(1.0, theta);
        let phase_conj = phase.conj(); // e^(-i*theta)
        let neg_i = Complex64::new(0.0, -1.0);

        // off_diag_01 = k * (-i * e^(-i*theta))
        let coef_01 = neg_i * phase_conj * k;

        // off_diag_10 = k * (-i * e^(i*theta))
        let coef_10 = neg_i * phase * k;

        // Explicitly convert to Complex64 for multiplication with complex numbers
        let k_complex = Complex64::new(k, 0.0);
        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;
                    *alpha = k_complex * a + coef_01 * b;
                    *beta = coef_10 * a + k_complex * b;
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Applies XY2M gate: Rz(-θ + π/2) Ry(-π/2) Rz(-π/2 + θ).
    ///
    /// Matrix: 1/√2 * [[1, i·e^(-iθ)], [i·e^(iθ), 1]]
    ///
    /// # Arguments
    /// * `qubit` - Target qubit index
    /// * `theta` - Rotation angle
    pub fn apply_xy2m(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let dist = 1 << qubit;
        let k = FRAC_1_SQRT_2;
        let phase = Complex64::from_polar(1.0, theta);
        let phase_conj = phase.conj();
        let pos_i = Complex64::new(0.0, 1.0);
        let coef_01 = pos_i * phase_conj * k;
        let coef_10 = pos_i * phase * k;
        let k_complex = Complex64::new(k, 0.0);
        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;
                    *alpha = k_complex * a + coef_01 * b;
                    *beta = coef_10 * a + k_complex * b;
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Applies XY gate.
    ///
    /// Matrix: [[0, -i·e^(-iθ)], [-i·e^(iθ), 0]]
    ///
    /// This is a single-qubit gate distinct from XY2P/XY2M.
    ///
    /// # Arguments
    /// * `qubit` - Target qubit index
    /// * `theta` - Rotation angle
    pub fn apply_xy(&mut self, qubit: usize, theta: f64) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;

        let dist = 1 << qubit;
        let phase = Complex64::from_polar(1.0, theta);
        let phase_conj = phase.conj();
        let neg_i = Complex64::new(0.0, -1.0);

        // u01 = -i * e^(-i*theta)
        let u01 = neg_i * phase_conj;
        // u10 = -i * e^(i*theta)
        let u10 = neg_i * phase;

        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;
                    *alpha = u01 * b;
                    *beta = u10 * a;
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Apply general rotation R(theta, phi).
    ///
    /// Rotates around axis cos(phi)X + sin(phi)Y by angle theta.
    /// Matrix: [[cos(t/2), -i*e^(-i*phi)*sin(t/2)], [-i*e^(i*phi)*sin(t/2), cos(t/2)]]
    ///
    /// # Arguments
    /// * `qubit` - Target qubit index
    /// * `theta` - Rotation angle
    /// * `phi` - Rotation axis angle in XY plane
    pub fn apply_rxy(&mut self, qubit: usize, theta: f64, phi: f64) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;

        // Fast path: treat as Identity if theta is negligible
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        // Precompute all coefficients
        let dist = 1 << qubit;
        let half_theta = theta * 0.5;
        let c = half_theta.cos();
        let s = half_theta.sin();

        // Precompute phi trig functions
        let (sin_phi, cos_phi) = phi.sin_cos();

        // Construct off-diagonal coefficients
        // u01 = -i * (cos_phi - i*sin_phi) * s
        //     = (-i*cos_phi - sin_phi) * s
        //     = s * (-sin_phi - i*cos_phi)
        let u01 = Complex64::new(-s * sin_phi, -s * cos_phi);

        // u10 = -i * (cos_phi + i*sin_phi) * s
        //     = (-i*cos_phi + sin_phi) * s
        //     = s * (sin_phi - i*cos_phi)
        let u10 = Complex64::new(s * sin_phi, -s * cos_phi);

        // Apply (serial or parallel based on qubit count)
        let kernel = |chunk: &mut [Complex64]| {
            let (lower, upper) = chunk.split_at_mut(dist);
            lower
                .iter_mut()
                .zip(upper.iter_mut())
                .for_each(|(alpha, beta)| {
                    let a = *alpha;
                    let b = *beta;

                    // Core computation:
                    // alpha' = c * a + u01 * b
                    // beta'  = u10 * a + c * b

                    // num_complex supports Complex * f64, faster than Complex(c, 0)
                    *alpha = a * c + b * u01;
                    *beta = a * u10 + b * c;
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist, kernel);
        Ok(())
    }

    /// Applies SWAP gate to exchange two qubits.
    ///
    /// Matrix: [[1,0,0,0], [0,0,1,0], [0,1,0,0], [0,0,0,1]]
    ///
    /// # Arguments
    /// * `q0` - First qubit index
    /// * `q1` - Second qubit index
    ///
    /// # Errors
    /// Returns [`QisError::IndexOutOfBounds`] if either qubit is out of bounds,
    /// or [`QisError::QubitMismatch`] if `q0 == q1`.
    ///
    /// # Optimization
    /// Uses memory swap without floating-point operations.
    pub fn apply_swap(&mut self, q0: usize, q1: usize) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;

        // 1. Determine physical memory strides
        let (q_min, q_max) = if q0 < q1 { (q0, q1) } else { (q1, q0) };
        let dist_max = 1 << q_max;
        let dist_min = 1 << q_min;

        // 2. Choose serial or parallel based on qubit count
        let kernel = |chunk: &mut [Complex64]| {
            let (part0, part1) = chunk.split_at_mut(dist_max);

            part0
                .chunks_exact_mut(2 * dist_min)
                .zip(part1.chunks_exact_mut(2 * dist_min))
                .for_each(|(sub_chunk0, sub_chunk1)| {
                    let (_, v01_slice) = sub_chunk0.split_at_mut(dist_min);
                    let (v10_slice, _) = sub_chunk1.split_at_mut(dist_min);

                    // v01_slice: physical (max=0, min=1) -> logical |01> or |10>
                    // v10_slice: physical (max=1, min=0) -> logical |10> or |01>
                    // SWAP always exchanges these two memory blocks
                    v01_slice
                        .iter_mut()
                        .zip(v10_slice.iter_mut())
                        .for_each(|(a, b)| {
                            // Direct memory swap, no floating-point overhead
                            std::mem::swap(a, b);
                        });
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist_max, kernel);
        Ok(())
    }

    /// Applies CNOT (CX) gate: controlled-X operation.
    ///
    /// Flips the target qubit when the control qubit is |1⟩.
    /// Matrix: [[1,0,0,0], [0,1,0,0], [0,0,0,1], [0,0,1,0]]
    ///
    /// # Arguments
    /// * `control` - Control qubit index
    /// * `target` - Target qubit index
    ///
    /// # Errors
    /// Returns [`QisError::IndexOutOfBounds`] if either qubit is out of bounds,
    /// or [`QisError::QubitMismatch`] if `control == target`.
    pub fn apply_cx(&mut self, control: usize, target: usize) -> Result<(), QisError> {
        self.validate_two_qubits(control, target)?;

        let (q_min, q_max) = if control < target {
            (control, target)
        } else {
            (target, control)
        };
        let dist_max = 1 << q_max;
        let dist_min = 1 << q_min;

        let kernel = |chunk: &mut [Complex64]| {
            let (part0, part1) = chunk.split_at_mut(dist_max);

            part0
                .chunks_exact_mut(2 * dist_min)
                .zip(part1.chunks_exact_mut(2 * dist_min))
                .for_each(|(sub_chunk0, sub_chunk1)| {
                    // Get references to four blocks; which to operate on depends on control/target
                    let (_v00, v01) = sub_chunk0.split_at_mut(dist_min);
                    let (v10, v11) = sub_chunk1.split_at_mut(dist_min);

                    if control < target {
                        // Case A: Control is low (q_min), Target is high (q_max)
                        // Control=1 physical positions:
                        //   1. max=0, min=1 (v01) -> Control=1, Target=0
                        //   2. max=1, min=1 (v11) -> Control=1, Target=1
                        // Action: swap v01 and v11 (flip Target)
                        v01.iter_mut()
                            .zip(v11.iter_mut())
                            .for_each(|(a, b)| std::mem::swap(a, b));
                    } else {
                        // Case B: Control is high (q_max), Target is low (q_min)
                        // Control=1 physical positions:
                        //   1. max=1, min=0 (v10) -> Control=1, Target=0
                        //   2. max=1, min=1 (v11) -> Control=1, Target=1
                        // Action: swap v10 and v11 (flip Target)
                        v10.iter_mut()
                            .zip(v11.iter_mut())
                            .for_each(|(a, b)| std::mem::swap(a, b));
                    }
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist_max, kernel);
        Ok(())
    }

    /// Applies controlled-Y (CY) gate.
    ///
    /// Applies Y gate to target when control qubit is |1⟩.
    /// Y matrix: [[0, -i], [i, 0]]
    ///
    /// # Arguments
    /// * `control` - Control qubit index
    /// * `target` - Target qubit index
    ///
    /// # Errors
    /// Returns [`QisError::IndexOutOfBounds`] if either qubit is out of bounds,
    /// or [`QisError::QubitMismatch`] if `control == target`.
    pub fn apply_cy(&mut self, control: usize, target: usize) -> Result<(), QisError> {
        self.validate_two_qubits(control, target)?;

        // 1. Determine physical memory strides
        let (q_min, q_max) = if control < target {
            (control, target)
        } else {
            (target, control)
        };
        let dist_max = 1 << q_max;
        let dist_min = 1 << q_min;

        // 2. Choose serial or parallel based on qubit count
        let kernel = |chunk: &mut [Complex64]| {
            let (part0, part1) = chunk.split_at_mut(dist_max);

            part0
                .chunks_exact_mut(2 * dist_min)
                .zip(part1.chunks_exact_mut(2 * dist_min))
                .for_each(|(sub_chunk0, sub_chunk1)| {
                    // Locate memory blocks where control=1
                    // block_t0: amplitude where Control=1, Target=0
                    // block_t1: amplitude where Control=1, Target=1

                    let (block_t0, block_t1) = if control < target {
                        // Case A: Control is low (min), Target is high (max)
                        // part0: q_max=0 (Target=0), part1: q_max=1 (Target=1)
                        // Need Control=1 states (q_min=1)
                        // |10⟩ (q_min=1, q_max=0) and |11⟩ (q_min=1, q_max=1)
                        // Corresponds to sub_chunk0[dist_min..] and sub_chunk1[dist_min..]
                        let (_, c1_t0) = sub_chunk0.split_at_mut(dist_min);
                        let (_, c1_t1) = sub_chunk1.split_at_mut(dist_min);
                        (c1_t0, c1_t1)
                    } else {
                        // Case B: Control is high (max), Target is low (min)
                        // part0 is Control=0 (skip), part1 is Control=1
                        // Within part1, lower half is Target=0, upper half is Target=1
                        let (t0, t1) = sub_chunk1.split_at_mut(dist_min);
                        (t0, t1)
                    };

                    // 3. Apply Y gate transformation
                    block_t0
                        .iter_mut()
                        .zip(block_t1.iter_mut())
                        .for_each(|(alpha, beta)| {
                            let a = *alpha; // Target=0
                            let b = *beta; // Target=1

                            // Y matrix: [[0, -i], [i, 0]]
                            // new_a = -i * b
                            // new_b =  i * a

                            // -i * (x + it) = y - ix
                            *alpha = Complex64::new(b.im, -b.re);

                            // i * (x + it) = -y + ix
                            *beta = Complex64::new(-a.im, a.re);
                        });
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist_max, kernel);
        Ok(())
    }

    /// Applies controlled-Z (CZ) gate.
    ///
    /// Applies phase flip (-1) to |11⟩ component.
    /// Leaves |00⟩, |01⟩, |10⟩ unchanged.
    ///
    /// # Arguments
    /// * `q0` - First qubit index (acts as control)
    /// * `q1` - Second qubit index (acts as control)
    ///
    /// # Errors
    /// Returns [`QisError::IndexOutOfBounds`] if either qubit is out of bounds,
    /// or [`QisError::QubitMismatch`] if `q0 == q1`.
    pub fn apply_cz(&mut self, q0: usize, q1: usize) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;

        let (q_min, q_max) = if q0 < q1 { (q0, q1) } else { (q1, q0) };
        let dist_max = 1 << q_max;
        let dist_min = 1 << q_min;

        let kernel = |chunk: &mut [Complex64]| {
            let (_, part1) = chunk.split_at_mut(dist_max); // Only care about max=1 part

            part1.chunks_exact_mut(2 * dist_min).for_each(|sub_chunk1| {
                let (_, v11) = sub_chunk1.split_at_mut(dist_min); // Only care about min=1 part

                // v11 now corresponds to max=1, min=1, i.e., |11⟩ state
                // Apply sign flip only to this part
                for val in v11.iter_mut() {
                    *val = -*val;
                }
            });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * dist_max, kernel);
        Ok(())
    }

    /// Apply Toffoli gate (CCX - Controlled-Controlled-X)
    /// Two control qubits and one target qubit.
    /// Flips the target if and only if both controls are |1⟩.
    ///
    /// # Arguments
    /// * `c0` - First control qubit index
    /// * `c1` - Second control qubit index
    /// * `target` - Target qubit index
    pub fn apply_ccx(&mut self, c0: usize, c1: usize, target: usize) -> Result<(), QisError> {
        self.validate_qubits(&[c0, c1, target])?;

        // 1. Sort: determine physical min, mid, max positions and their roles
        // kind: 0 = Control, 1 = Target
        let mut qubits = [(c0, 0), (c1, 0), (target, 1)];
        qubits.sort_by_key(|(q, _)| *q);

        let (q0, k0) = qubits[0]; // Min position
        let (q1, k1) = qubits[1]; // Mid position
        let (q2, k2) = qubits[2]; // Max position

        let d0 = 1 << q0;
        let d1 = 1 << q1;
        let d2 = 1 << q2;

        match (k0, k1, k2) {
            // Case A: Target is at max position -> (Control, Control, Target)
            (0, 0, 1) => {
                let kernel = |chunk: &mut [Complex64]| {
                    let (part_t0, part_t1) = chunk.split_at_mut(d2);

                    // Iterate over Mid layer
                    part_t0
                        .chunks_exact_mut(2 * d1)
                        .zip(part_t1.chunks_exact_mut(2 * d1))
                        .for_each(|(sub_t0, sub_t1)| {
                            // Take Mid=1 part (control is active)
                            let (_, high_m_t0) = sub_t0.split_at_mut(d1);
                            let (_, high_m_t1) = sub_t1.split_at_mut(d1);

                            // Iterate over Min layer (using chunks to handle non-contiguous gaps)
                            high_m_t0
                                .chunks_exact_mut(2 * d0)
                                .zip(high_m_t1.chunks_exact_mut(2 * d0))
                                .for_each(|(leaf_t0, leaf_t1)| {
                                    // Take Min=1 part (control is active)
                                    let (_, valid_t0) = leaf_t0.split_at_mut(d0);
                                    let (_, valid_t1) = leaf_t1.split_at_mut(d0);

                                    // Now Min=1, Mid=1, execute Target flip
                                    valid_t0
                                        .iter_mut()
                                        .zip(valid_t1.iter_mut())
                                        .for_each(|(a, b)| std::mem::swap(a, b));
                                });
                        });
                };
                with_maybe_par!(self.num_qubits, self.data, 2 * d2, kernel);
            }

            // Case B: Target is at mid position -> (Control, Target, Control)
            // Min=Control0, Mid=Target, Max=Control1
            // Need to swap amplitudes where Control0=1, Control1=1, Target=0 with Target=1
            //
            // Memory layout note:
            // We use zip to pair elements from low_t0 (Target=0) and high_t1 (Target=1).
            // Although low_t0 and high_t1 are not contiguous in memory, they are "symmetric"
            // in logical structure:
            // - low_t0 contains Min=0 and Min=1 parts
            // - high_t1 also contains Min=0 and Min=1 parts
            // Since both slices come from the same d1 split, their internal structures match.
            // Therefore, using zip on low_t0.chunks_exact_mut(2*d0) and
            // high_t1.chunks_exact_mut(2*d0) correctly pairs elements at the same Min index.
            (0, 1, 0) => {
                let kernel = |chunk: &mut [Complex64]| {
                    // Max (Control1) is a control, take Max=1 part
                    let (_, part_c1) = chunk.split_at_mut(d2);

                    part_c1.chunks_exact_mut(2 * d1).for_each(|sub| {
                        // Mid is Target, split into Target=0 and Target=1
                        let (low_t0, high_t1) = sub.split_at_mut(d1);

                        // Iterate over Min (Control0) layer
                        // Use memory structure symmetry to zip pair Target=0 and Target=1 blocks
                        low_t0
                            .chunks_exact_mut(2 * d0)
                            .zip(high_t1.chunks_exact_mut(2 * d0))
                            .for_each(|(leaf_t0, leaf_t1)| {
                                // Min is Control0, take Min=1 part (both controls are 1)
                                let (_, valid_t0) = leaf_t0.split_at_mut(d0);
                                let (_, valid_t1) = leaf_t1.split_at_mut(d0);

                                // Execute SWAP: exchange Target=0 and Target=1 amplitudes
                                valid_t0
                                    .iter_mut()
                                    .zip(valid_t1.iter_mut())
                                    .for_each(|(a, b)| std::mem::swap(a, b));
                            });
                    });
                };
                with_maybe_par!(self.num_qubits, self.data, 2 * d2, kernel);
            }

            // Case C: Target is at min position -> (Target, Control, Control)
            (1, 0, 0) => {
                let kernel = |chunk: &mut [Complex64]| {
                    let (_, part_max1) = chunk.split_at_mut(d2); // Max=1

                    part_max1.chunks_exact_mut(2 * d1).for_each(|sub| {
                        let (_, part_mid1) = sub.split_at_mut(d1); // Mid=1

                        part_mid1.chunks_exact_mut(2 * d0).for_each(|leaf| {
                            let (target0, target1) = leaf.split_at_mut(d0); // Min=0 vs Min=1

                            // No further split needed since Min is Target
                            target0
                                .iter_mut()
                                .zip(target1.iter_mut())
                                .for_each(|(a, b)| std::mem::swap(a, b));
                        });
                    });
                };
                with_maybe_par!(self.num_qubits, self.data, 2 * d2, kernel);
            }

            _ => unreachable!("Should cover all permutations"),
        }
        Ok(())
    }

    /// Applies RXX gate (Ising XX interaction).
    ///
    /// Matrix: exp(-i·θ/2 · X⊗X) = cos(θ/2)I - i·sin(θ/2)·X⊗X
    ///
    /// # Arguments
    /// * `q0` - First qubit index
    /// * `q1` - Second qubit index
    /// * `theta` - Interaction angle
    pub fn apply_rxx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        let (q_min, q_max) = if q0 < q1 { (q0, q1) } else { (q1, q0) };
        let d_max = 1 << q_max;
        let d_min = 1 << q_min;

        let half_theta = theta * 0.5;
        let c = half_theta.cos();
        let s = half_theta.sin();

        let kernel = |chunk: &mut [Complex64]| {
            let (part0, part1) = chunk.split_at_mut(d_max);
            part0
                .chunks_exact_mut(2 * d_min)
                .zip(part1.chunks_exact_mut(2 * d_min))
                .for_each(|(sub0, sub1)| {
                    let (v00, v01) = sub0.split_at_mut(d_min);
                    let (v10, v11) = sub1.split_at_mut(d_min);
                    for (((a, b), c_amp), d) in v00
                        .iter_mut()
                        .zip(v01.iter_mut())
                        .zip(v10.iter_mut())
                        .zip(v11.iter_mut())
                    {
                        let a_in = *a;
                        let b_in = *b;
                        let c_in = *c_amp;
                        let d_in = *d;
                        *a = c * a_in + Complex64::new(0.0, -s) * d_in;
                        *b = c * b_in + Complex64::new(0.0, -s) * c_in;
                        *c_amp = Complex64::new(0.0, -s) * b_in + c * c_in;
                        *d = Complex64::new(0.0, -s) * a_in + c * d_in;
                    }
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d_max, kernel);
        Ok(())
    }

    /// Applies RYY gate (Ising YY interaction).
    ///
    /// Matrix: exp(-i·θ/2 · Y⊗Y) = cos(θ/2)I - i·sin(θ/2)·Y⊗Y
    ///
    /// # Arguments
    /// * `q0` - First qubit index
    /// * `q1` - Second qubit index
    /// * `theta` - Interaction angle
    pub fn apply_ryy(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        let (q_min, q_max) = if q0 < q1 { (q0, q1) } else { (q1, q0) };
        let d_max = 1 << q_max;
        let d_min = 1 << q_min;

        let half_theta = theta * 0.5;
        let c = half_theta.cos();
        let s = half_theta.sin();

        let kernel = |chunk: &mut [Complex64]| {
            let (part0, part1) = chunk.split_at_mut(d_max);
            part0
                .chunks_exact_mut(2 * d_min)
                .zip(part1.chunks_exact_mut(2 * d_min))
                .for_each(|(sub0, sub1)| {
                    let (v00, v01) = sub0.split_at_mut(d_min);
                    let (v10, v11) = sub1.split_at_mut(d_min);
                    for (((a, b), c_amp), d) in v00
                        .iter_mut()
                        .zip(v01.iter_mut())
                        .zip(v10.iter_mut())
                        .zip(v11.iter_mut())
                    {
                        let a_in = *a;
                        let b_in = *b;
                        let c_in = *c_amp;
                        let d_in = *d;
                        *a = c * a_in + Complex64::new(0.0, s) * d_in;
                        *b = c * b_in + Complex64::new(0.0, -s) * c_in;
                        *c_amp = Complex64::new(0.0, -s) * b_in + c * c_in;
                        *d = Complex64::new(0.0, s) * a_in + c * d_in;
                    }
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d_max, kernel);
        Ok(())
    }

    /// Applies RZZ gate (Ising ZZ interaction).
    ///
    /// Matrix: exp(-i·θ/2 · Z⊗Z) - applies phase based on qubit parity
    ///
    /// # Arguments
    /// * `q0` - First qubit index
    /// * `q1` - Second qubit index
    /// * `theta` - Interaction angle
    pub fn apply_rzz(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        let (q_min, q_max) = if q0 < q1 { (q0, q1) } else { (q1, q0) };
        let d_max = 1 << q_max;
        let d_min = 1 << q_min;

        let half_theta = theta * 0.5;
        let phase_even = Complex64::from_polar(1.0, -half_theta);
        let phase_odd = Complex64::from_polar(1.0, half_theta);

        let kernel = |chunk: &mut [Complex64]| {
            let (part0, part1) = chunk.split_at_mut(d_max);
            part0
                .chunks_exact_mut(2 * d_min)
                .zip(part1.chunks_exact_mut(2 * d_min))
                .for_each(|(sub0, sub1)| {
                    let (v00, v01) = sub0.split_at_mut(d_min);
                    let (v10, v11) = sub1.split_at_mut(d_min);
                    v00.iter_mut().for_each(|a| *a *= phase_even);
                    v11.iter_mut().for_each(|d| *d *= phase_even);
                    v01.iter_mut().for_each(|b| *b *= phase_odd);
                    v10.iter_mut().for_each(|c| *c *= phase_odd);
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d_max, kernel);
        Ok(())
    }

    /// Applies RZX gate (Ising ZX interaction).
    ///
    /// Matrix: exp(-i·θ/2 · Z⊗X)
    ///
    /// # Arguments
    /// * `q0` - First qubit index (Z rotation)
    /// * `q1` - Second qubit index (X rotation)
    /// * `theta` - Interaction angle
    pub fn apply_rzx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        let (q_min, q_max) = if q0 < q1 { (q0, q1) } else { (q1, q0) };
        let d_max = 1 << q_max;
        let d_min = 1 << q_min;

        let half_theta = theta * 0.5;
        let c = half_theta.cos();
        let s = half_theta.sin();

        let kernel = |chunk: &mut [Complex64]| {
            let (part0, part1) = chunk.split_at_mut(d_max);
            part0
                .chunks_exact_mut(2 * d_min)
                .zip(part1.chunks_exact_mut(2 * d_min))
                .for_each(|(sub0, sub1)| {
                    let (v00, v01) = sub0.split_at_mut(d_min);
                    let (v10, v11) = sub1.split_at_mut(d_min);
                    for (((a, b), c_amp), d) in v00
                        .iter_mut()
                        .zip(v01.iter_mut())
                        .zip(v10.iter_mut())
                        .zip(v11.iter_mut())
                    {
                        let a_in = *a;
                        let b_in = *b;
                        let c_in = *c_amp;
                        let d_in = *d;
                        if q0 < q1 {
                            *a = c * a_in + Complex64::new(0.0, -s) * c_in;
                            *b = c * b_in + Complex64::new(0.0, s) * d_in;
                            *c_amp = Complex64::new(0.0, -s) * a_in + c * c_in;
                            *d = Complex64::new(0.0, s) * b_in + c * d_in;
                        } else {
                            *a = c * a_in + Complex64::new(0.0, -s) * b_in;
                            *b = Complex64::new(0.0, -s) * a_in + c * b_in;
                            *c_amp = c * c_in + Complex64::new(0.0, s) * d_in;
                            *d = Complex64::new(0.0, s) * c_in + c * d_in;
                        }
                    }
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d_max, kernel);
        Ok(())
    }

    /// Applies controlled-RX gate.
    ///
    /// Applies RX(θ) to target when control is |1⟩.
    ///
    /// # Arguments
    /// * `control` - Control qubit index
    /// * `target` - Target qubit index
    /// * `theta` - Rotation angle
    pub fn apply_crx(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(control, target)?;
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        let (q_min, q_max) = if control < target {
            (control, target)
        } else {
            (target, control)
        };
        let d_max = 1 << q_max;
        let d_min = 1 << q_min;

        let half_theta = theta * 0.5;
        let c = half_theta.cos();
        let s = half_theta.sin();

        // CRX matrix on target (when control=1): [[c, -i*s], [-i*s, c]]

        let kernel = |chunk: &mut [Complex64]| {
            let (part0, part1) = chunk.split_at_mut(d_max);

            if control < target {
                // Control is q_min, target is q_max
                // Need to pair |Target=0, Control=1⟩ (part0, sub upper) with |Target=1, Control=1⟩ (part1, sub upper)
                part0
                    .chunks_exact_mut(2 * d_min)
                    .zip(part1.chunks_exact_mut(2 * d_min))
                    .for_each(|(sub0, sub1)| {
                        // sub0: target=0, sub1: target=1
                        // In each sub, lower half is control=0, upper half is control=1
                        let (_, target0_c1) = sub0.split_at_mut(d_min);
                        let (_, target1_c1) = sub1.split_at_mut(d_min);

                        // Apply RX rotation to target when control=1
                        for (a, b) in target0_c1.iter_mut().zip(target1_c1.iter_mut()) {
                            let a_in = *a;
                            let b_in = *b;
                            // RX: [[c, -i*s], [-i*s, c]]
                            // -i*s*(x+iy) = s*y - i*s*x
                            *a = Complex64::new(
                                c * a_in.re + s * b_in.im,
                                c * a_in.im - s * b_in.re,
                            );
                            *b = Complex64::new(
                                s * a_in.im + c * b_in.re,
                                -s * a_in.re + c * b_in.im,
                            );
                        }
                    });
            } else {
                // Control is q_max, target is q_min
                // Process pairs where control=1
                part0
                    .chunks_exact_mut(2 * d_min)
                    .zip(part1.chunks_exact_mut(2 * d_min))
                    .for_each(|(_, sub1)| {
                        // sub0: control=0, sub1: control=1
                        // In sub1, split by target
                        let (target0_ctl1, target1_ctl1) = sub1.split_at_mut(d_min);

                        // Apply RX rotation to target when control=1
                        for (a, b) in target0_ctl1.iter_mut().zip(target1_ctl1.iter_mut()) {
                            let a_in = *a;
                            let b_in = *b;
                            // RX: [[c, -i*s], [-i*s, c]]
                            *a = Complex64::new(
                                c * a_in.re + s * b_in.im,
                                c * a_in.im - s * b_in.re,
                            );
                            *b = Complex64::new(
                                s * a_in.im + c * b_in.re,
                                -s * a_in.re + c * b_in.im,
                            );
                        }
                    });
            }
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d_max, kernel);
        Ok(())
    }

    /// Applies controlled-RY gate.
    ///
    /// Applies RY(θ) to target when control is |1⟩.
    ///
    /// # Arguments
    /// * `control` - Control qubit index
    /// * `target` - Target qubit index
    /// * `theta` - Rotation angle
    pub fn apply_cry(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(control, target)?;
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        let (q_min, q_max) = if control < target {
            (control, target)
        } else {
            (target, control)
        };
        let d_max = 1 << q_max;
        let d_min = 1 << q_min;

        let half_theta = theta * 0.5;
        let c = half_theta.cos();
        let s = half_theta.sin();

        let kernel = |chunk: &mut [Complex64]| {
            let (part0, part1) = chunk.split_at_mut(d_max);

            if control < target {
                // Control is q_min, target is q_max
                // Need to pair |Target=0, Control=1> (part0, sub upper) with |Target=1, Control=1> (part1, sub upper)
                part0
                    .chunks_exact_mut(2 * d_min)
                    .zip(part1.chunks_exact_mut(2 * d_min))
                    .for_each(|(sub0, sub1)| {
                        // sub0: target=0, sub1: target=1
                        // In each sub, lower half is control=0, upper half is control=1
                        let (_, target0_c1) = sub0.split_at_mut(d_min);
                        let (_, target1_c1) = sub1.split_at_mut(d_min);

                        // Apply RY rotation to target when control=1
                        for (a, b) in target0_c1.iter_mut().zip(target1_c1.iter_mut()) {
                            let a_in = *a;
                            let b_in = *b;
                            *a = c * a_in - s * b_in;
                            *b = s * a_in + c * b_in;
                        }
                    });
            } else {
                // Control is q_max, target is q_min
                // Process pairs where control=1
                part0
                    .chunks_exact_mut(2 * d_min)
                    .zip(part1.chunks_exact_mut(2 * d_min))
                    .for_each(|(_, sub1)| {
                        // sub0: control=0, sub1: control=1
                        // In sub1, split by target
                        let (target0_ctl1, target1_ctl1) = sub1.split_at_mut(d_min);

                        // Apply RY rotation to target when control=1
                        for (a, b) in target0_ctl1.iter_mut().zip(target1_ctl1.iter_mut()) {
                            let a_in = *a;
                            let b_in = *b;
                            *a = c * a_in - s * b_in;
                            *b = s * a_in + c * b_in;
                        }
                    });
            }
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d_max, kernel);
        Ok(())
    }

    /// Applies controlled-RZ gate.
    ///
    /// Applies RZ(θ) to target when control is |1⟩.
    ///
    /// # Arguments
    /// * `control` - Control qubit index
    /// * `target` - Target qubit index
    /// * `theta` - Rotation angle
    pub fn apply_crz(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError> {
        self.validate_two_qubits(control, target)?;
        if theta.abs() < 1e-15 {
            return Ok(());
        }

        let (q_min, q_max) = if control < target {
            (control, target)
        } else {
            (target, control)
        };
        let d_max = 1 << q_max;
        let d_min = 1 << q_min;

        let half_theta = theta * 0.5;
        let phase_0 = Complex64::from_polar(1.0, -half_theta);
        let phase_1 = Complex64::from_polar(1.0, half_theta);

        let kernel = |chunk: &mut [Complex64]| {
            let (part0, part1) = chunk.split_at_mut(d_max);

            if control < target {
                // Control is q_min, target is q_max
                // Need to pair |Target=0, Control=1> (part0, sub upper) with |Target=1, Control=1> (part1, sub upper)
                part0
                    .chunks_exact_mut(2 * d_min)
                    .zip(part1.chunks_exact_mut(2 * d_min))
                    .for_each(|(sub0, sub1)| {
                        // sub0: target=0, sub1: target=1
                        // In each sub, lower half is control=0, upper half is control=1
                        let (_, target0_c1) = sub0.split_at_mut(d_min);
                        let (_, target1_c1) = sub1.split_at_mut(d_min);

                        // Apply RZ rotation to target when control=1
                        for (a, b) in target0_c1.iter_mut().zip(target1_c1.iter_mut()) {
                            *a *= phase_0;
                            *b *= phase_1;
                        }
                    });
            } else {
                // Control is q_max, target is q_min
                // Process pairs where control=1
                part0
                    .chunks_exact_mut(2 * d_min)
                    .zip(part1.chunks_exact_mut(2 * d_min))
                    .for_each(|(_, sub1)| {
                        // sub0: control=0, sub1: control=1
                        // In sub1, split by target
                        let (target0_ctl1, target1_ctl1) = sub1.split_at_mut(d_min);

                        // When control=1, apply RZ to target
                        for (a, b) in target0_ctl1.iter_mut().zip(target1_ctl1.iter_mut()) {
                            *a *= phase_0;
                            *b *= phase_1;
                        }
                    });
            }
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d_max, kernel);
        Ok(())
    }

    /// Applies Fermionic Simulation (fSim) gate.
    ///
    /// Native gate for superconducting qubits. Combines iSWAP and controlled-phase.
    ///
    /// Matrix: [[1, 0, 0, 0], [0, c, -i·s, 0], [0, -i·s, c, 0], [0, 0, 0, e^(-iφ)]]
    /// where c = cos(θ), s = sin(θ)
    ///
    /// # Arguments
    /// * `q0` - First qubit index
    /// * `q1` - Second qubit index
    /// * `theta` - iSWAP angle
    /// * `phi` - Controlled-phase angle
    pub fn apply_fsim(
        &mut self,
        q0: usize,
        q1: usize,
        theta: f64,
        phi: f64,
    ) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;

        let (q_min, q_max) = if q0 < q1 { (q0, q1) } else { (q1, q0) };
        let d_max = 1 << q_max;
        let d_min = 1 << q_min;

        let c = theta.cos();
        let s = theta.sin();
        let phase_11 = Complex64::from_polar(1.0, -phi);

        let kernel = |chunk: &mut [Complex64]| {
            let (part0, part1) = chunk.split_at_mut(d_max);
            part0
                .chunks_exact_mut(2 * d_min)
                .zip(part1.chunks_exact_mut(2 * d_min))
                .for_each(|(sub0, sub1)| {
                    let (_, v01) = sub0.split_at_mut(d_min);
                    let (v10, v11) = sub1.split_at_mut(d_min);

                    // |11⟩ gets phase
                    v11.iter_mut().for_each(|d| *d *= phase_11);

                    // |01⟩ and |10⟩ get mixed
                    for (b, c_amp) in v01.iter_mut().zip(v10.iter_mut()) {
                        let b_in = *b;
                        let c_in = *c_amp;
                        *b = c * b_in + Complex64::new(0.0, -s) * c_in;
                        *c_amp = Complex64::new(0.0, -s) * b_in + c * c_in;
                    }
                });
        };
        with_maybe_par!(self.num_qubits, self.data, 2 * d_max, kernel);
        Ok(())
    }

    /// Applies global phase.
    ///
    /// Multiplies the entire statevector by e^(iφ).
    /// Physically undetectable but mathematically relevant for circuit composition.
    ///
    /// # Arguments
    /// * `phi` - Phase angle
    pub fn apply_gphase(&mut self, phi: f64) -> Result<(), QisError> {
        if phi.abs() < 1e-15 {
            return Ok(());
        }
        let phase = Complex64::from_polar(1.0, phi);
        if self.num_qubits < PARALLEL_THRESHOLD {
            self.data.iter_mut().for_each(|a| *a *= phase);
        } else {
            self.data.par_iter_mut().for_each(|a| *a *= phase);
        }
        Ok(())
    }

    /// Computes the expectation value of a Hamiltonian observable.
    ///
    /// Calculates ⟨ψ|H|ψ⟩ for the current statevector |ψ⟩ and a given Hamiltonian H.
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
    /// number of qubits than the statevector.
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::qis::{Statevector, Hamiltonian};
    /// use cqlib_core::qis::pauli::{Pauli, PauliString};
    /// use num_complex::Complex64;
    ///
    /// let mut sv = Statevector::new(1);
    /// sv.apply_x(0);
    ///
    /// // Create Hamiltonian H = Z (Pauli-Z observable)
    /// let mut ps = PauliString::new(1);
    /// ps.set_pauli(0, Pauli::Z);
    /// let h = Hamiltonian::from_pauli(ps);
    ///
    /// let exp = sv.expectation(&h).unwrap();
    /// // For |1⟩ state, ⟨Z⟩ = -1
    /// ```
    pub fn expectation(&self, h: &dyn Observable) -> Result<f64, QisError> {
        h.expectation_statevector(self)
    }

    /// Measures qubit `qubit` in the Z basis, collapsing the statevector.
    ///
    /// Returns `false` for outcome |0⟩ and `true` for outcome |1⟩.
    /// After measurement the statevector is normalised in the post-measurement subspace.
    ///
    /// This is a destructive operation — clone the statevector first if you need
    /// to preserve the pre-measurement state.
    pub fn measure(&mut self, qubit: usize) -> Result<bool, QisError> {
        let mut rng = rand::rng();
        self.measure_with_rng(qubit, &mut rng)
    }

    /// Measures qubit `qubit` using a caller-provided random number generator.
    ///
    /// This is useful for reproducible simulations. For one-off non-deterministic
    /// measurements, use [`measure`](Self::measure).
    pub fn measure_with_rng(&mut self, qubit: usize, rng: &mut impl Rng) -> Result<bool, QisError> {
        self.validate_qubit(qubit)?;
        let n = self.num_qubits;
        let size = 1 << n;
        let mask = 1 << qubit;

        // Compute probability of outcome |1⟩.
        let p1: f64 = (0..size)
            .filter(|i| i & mask != 0)
            .map(|i| self.data[i].norm_sqr())
            .sum();

        let outcome = rng.random::<f64>() < p1;

        // Collapse and renormalise.
        let norm = if outcome {
            p1.sqrt()
        } else {
            (1.0 - p1).sqrt()
        };
        for i in 0..size {
            let keep = if outcome {
                (i & mask) != 0
            } else {
                (i & mask) == 0
            };
            if keep {
                self.data[i] /= norm;
            } else {
                self.data[i] = Complex64::new(0.0, 0.0);
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

    /// Copies amplitude data from `source` into `self` without allocating.
    ///
    /// Both statevectors must have the same `num_qubits`. Used by
    /// [`sample_shots`](Self::sample_shots) to reuse each thread's working copy.
    fn reset_from(&mut self, source: &Statevector) {
        debug_assert_eq!(self.num_qubits, source.num_qubits);
        self.data
            .as_mut_slice()
            .copy_from_slice(source.data.as_slice());
    }

    /// Measures all qubits sequentially, returning a bit-packed [`Outcome`].
    ///
    /// Equivalent to calling `measure(q)` for each qubit `q` in order `0..num_qubits`.
    /// The statevector is fully collapsed after this call.
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
    /// statevector (via [`reset_from`](Self::reset_from)), avoiding per-shot
    /// heap allocation. Returns a [`Vec<Outcome>`] of bit-packed results.
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::qis::Statevector;
    ///
    /// let mut sv = Statevector::new(2);
    /// sv.apply_h(0).unwrap();
    /// sv.apply_cx(0, 1).unwrap(); // |Φ⁺⟩ Bell state
    ///
    /// let shots = sv.sample_shots(200);
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
            "statevector-sample".to_string(),
            measurement.qubits().to_vec(),
            shots,
            measurement.width(),
            Some("statevector".to_string()),
            counts,
        ))
    }

    /// Returns the marginal probability distribution selected by a circuit [`Measurement`].
    ///
    /// This method does not execute circuit IR. It treats the [`Measurement`] as
    /// a self-contained output contract and aggregates the full statevector
    /// computational-basis probabilities over unmeasured qubits.
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

#[cfg(test)]
#[path = "./statevector_test.rs"]
mod statevector_test;
