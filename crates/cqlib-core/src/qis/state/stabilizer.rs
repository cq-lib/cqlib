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

//! Stabilizer state simulator using the Aaronson-Gottesman tableau algorithm.
//!
//! Implements efficient simulation of Clifford circuits using the symplectic tableau
//! representation from:
//!
//! > Aaronson & Gottesman, "Improved Simulation of Stabilizer Circuits",
//! > Phys. Rev. A 70, 052328 (2004).
//!
//! Non-Clifford gates (T, TDG, RX, RZ, U, …) are rejected immediately with
//! [`QisError::NonCliffordGate`] — use [`crate::qis::Statevector`] for universal circuits.
//!
//! # Supported Gates
//!
//! | Gate | Symbol | Kind |
//! |------|--------|------|
//! | Hadamard | H | single-qubit |
//! | Phase | S | single-qubit |
//! | Phase-dagger | S† / SDG | single-qubit |
//! | Pauli-X | X | single-qubit |
//! | Pauli-Y | Y | single-qubit |
//! | Pauli-Z | Z | single-qubit |
//! | √X | X2P / SX | single-qubit |
//! | √X† | X2M / SXdg | single-qubit |
//! | √Y | Y2P / SY | single-qubit |
//! | √Y† | Y2M / SYdg | single-qubit |
//! | Controlled-NOT | CX / CNOT | two-qubit |
//! | Controlled-Y | CY | two-qubit |
//! | Controlled-Z | CZ | two-qubit |
//! | SWAP | SWAP | two-qubit |
//!
//! # Performance
//!
//! | Operation | Complexity | Notes |
//! |-----------|-----------|-------|
//! | Single-qubit gate | O(n) | one pass over 2n rows |
//! | Two-qubit gate | O(n) | one pass over 2n rows |
//! | Measurement (random) | O(n²/w) | SIMD-accelerated rowsum, w = 64 |
//! | Measurement (deterministic) | O(n²/w) | scratch-row accumulation via rowsum |
//! | Memory | O(n²/w) | ~50 MB for n = 10 000 |
//!
//! Multi-shot sampling uses Rayon for parallel execution; each shot gets an
//! independent clone of the state.
//!
//! # Memory Layout
//!
//! The tableau uses a single flat `Box<[u64]>` for cache efficiency:
//! - `2n + 1` rows total: rows `0..n` = destabilizers, rows `n..2n` = stabilizers,
//!   row `2n` = pre-allocated scratch for deterministic measurement.
//! - Each row: `row_len` u64 words for the X-block, then `row_len` words for the Z-block:
//!   `[x₀..xₙ | z₀..zₙ]` packed into `row_len` words each.
//! - `row_len` is padded to a multiple of 8 (512 bits) for AVX-512 alignment.
//! - Phases stored separately in `phases: Box<[u8]>` as `0` (= +1), `1` (= +i), `2` (= −1), `3` (= −i).
//!
//! # Bit Addressing
//!
//! Qubit `q` maps to word `q / 64` and bit `q % 64` within both the X and Z blocks
//! of every row.
//!
//! # Examples
//!
//! **Bell state preparation and measurement:**
//! ```rust
//! use cqlib_core::qis::StabilizerState;
//!
//! let mut s = StabilizerState::new(2);
//! s.apply_h(0).unwrap();
//! s.apply_cx(0, 1).unwrap(); // |Φ⁺⟩ = (|00⟩ + |11⟩)/√2
//!
//! // Outcomes are always correlated
//! let shots = s.sample_shots(500);
//! assert!(shots.iter().all(|v| v.is_one(0) == v.is_one(1)));
//! ```
//!
//! **GHZ state — 1000 qubits:**
//! ```rust
//! use cqlib_core::qis::StabilizerState;
//!
//! let n = 1000;
//! let mut s = StabilizerState::new(n);
//! s.apply_h(0).unwrap();
//! for q in 0..n - 1 {
//!     s.apply_cx(q, q + 1).unwrap();
//! }
//! // All qubits measure identically
//! let mut copy = s.clone();
//! let result = copy.measure_all();
//! let first = result.is_one(0);
//! assert!((0..n).all(|q| result.is_one(q) == first));
//! ```
//!
//! **Deterministic measurement on |0⟩:**
//! ```rust
//! use cqlib_core::qis::StabilizerState;
//!
//! let mut s = StabilizerState::new(3);
//! // |000⟩ state — all measurements deterministically 0
//! assert_eq!(s.measure(0).unwrap(), false);
//! assert_eq!(s.measure(1).unwrap(), false);
//! assert_eq!(s.measure(2).unwrap(), false);
//! ```
//!
//! **From a Clifford circuit:**
//! ```rust
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::qis::StabilizerState;
//!
//! let mut c = Circuit::new(2);
//! c.h(Qubit::new(0)).unwrap();
//! c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
//! let stab = StabilizerState::from_circuit(&c).unwrap();
//! let stabilizers = stab.get_stabilizers();
//! // Bell state is stabilized by +XX and +ZZ
//! assert_eq!(stabilizers.len(), 2);
//! ```
//!
//! **Circuit measurement results:**
//! ```rust
//! use cqlib_core::circuit::{Circuit, ClassicalType, Qubit};
//! use cqlib_core::qis::{RuntimeValue, StabilizerState};
//!
//! let mut c = Circuit::new(3);
//! c.x(Qubit::new(0)).unwrap();
//! c.x(Qubit::new(2)).unwrap();
//!
//! // The returned measurement carries both the IR value and measured qubit order.
//! let bit = c.measure(Qubit::new(0)).unwrap();
//!
//! // `measure_bits_into` also writes a mutable variable copy for later classical use.
//! let latest = c.var(ClassicalType::bit_vec(3).unwrap());
//! let bits = c
//!     .measure_bits_into([Qubit::new(0), Qubit::new(1), Qubit::new(2)], latest)
//!     .unwrap();
//!
//! let execution = StabilizerState::run_circuit(&c).unwrap();
//! assert_eq!(execution.classical.value(bit.value()), Some(&RuntimeValue::Bit(true)));
//! assert_eq!(
//!     execution.classical.value(bits.value()).and_then(RuntimeValue::to_bitstring).as_deref(),
//!     Some("101")
//! );
//! assert_eq!(execution.classical.var(latest), execution.classical.value(bits.value()));
//! ```
//!
//! **Non-Clifford gate returns an error:**
//! ```rust
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::qis::{StabilizerState, QisError};
//!
//! let mut c = Circuit::new(1);
//! c.t(Qubit::new(0)).unwrap(); // T gate is not Clifford
//! let result = StabilizerState::from_circuit(&c);
//! assert!(matches!(result, Err(QisError::NonCliffordGate(_))));
//! ```

use super::aligned_buffer::AlignedBuffer;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::circuit_param::CircuitParam;
use crate::circuit::error::CircuitError;
use crate::circuit::gate::directive::Directive;
use crate::circuit::gate::{ClassicalDataOp, Instruction, StandardGate};
use crate::circuit::operation::Operation;
use crate::circuit::{Measurement, Qubit};
use crate::device::{ExecutionResult, Outcome};
use crate::qis::error::QisError;
use crate::qis::pauli::{Pauli, PauliString, Phase};
use crate::qis::state::{ClassicalState, RuntimeValue};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use smallvec::SmallVec;
use std::collections::HashMap;

/// Result returned by [`StabilizerState::run_circuit`].
///
/// Contains both the final quantum state and runtime classical data produced
/// while executing measurement and store operations.
#[derive(Debug)]
pub struct CircuitExecutionResult {
    /// Final stabilizer state after all circuit operations.
    pub state: StabilizerState,
    /// Runtime classical values and variables indexed by circuit-local handles.
    pub classical: ClassicalState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitClassicalMode {
    Ignore,
    Execute,
}

/// Stabilizer state simulator based on the Aaronson-Gottesman symplectic tableau.
///
/// Represents an n-qubit stabilizer state using 2n generator rows, each composed
/// of an X-block and Z-block of packed `u64` words, plus a phase per row.
#[derive(Debug)]
pub struct StabilizerState {
    /// Number of qubits.
    pub num_qubits: usize,
    /// Number of `u64` words per block (X-block or Z-block) per row.
    /// Padded to a multiple of 8 for 512-bit SIMD alignment.
    row_len: usize,
    /// Flat tableau storage: `(2n+1)` rows × `2 * row_len` words, 64-byte aligned.
    /// Row `i` occupies `tableau[i * 2 * row_len .. (i+1) * 2 * row_len]`.
    /// Within a row: first `row_len` words = X-block, next `row_len` words = Z-block.
    /// Row `2n` is the pre-allocated scratch row for deterministic measurement.
    tableau: AlignedBuffer<u64>,
    /// Phase for each of the `2n+1` rows: `0`=+1, `1`=+i, `2`=−1, `3`=−i.
    phases: Box<[u8]>,
    /// RNG for random measurement outcomes.
    rng: SmallRng,
}

// `xor_rows_dispatch` XORs `count` u64 words from `src_base` into `dst_base`
// within `tableau`, dispatching to the best available SIMD tier at runtime.
// `count` is always a multiple of 16 (= 2 × row_len, row_len ≥ 8), so no tail
// handling is required for any SIMD width up to 512 bits.
//
// SAFETY contract shared by all variants:
//   • `dst_base + count ≤ tableau.len()` and `src_base + count ≤ tableau.len()`
//   • `[dst_base, dst_base+count)` and `[src_base, src_base+count)` do not overlap
//     (guaranteed by the caller via distinct row indices h ≠ i).

/// AVX2 path: 256-bit XOR, processing 4 u64 words per iteration.
///
/// Requires 32-byte alignment. Our `AlignedBuffer` provides 64-byte alignment,
/// so aligned loads (`_mm256_load_si256`) are safe for every row start.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn xor_rows_avx2(tableau: &mut [u64], dst_base: usize, src_base: usize, count: usize) {
    use std::arch::x86_64::{__m256i, _mm256_load_si256, _mm256_store_si256, _mm256_xor_si256};
    // SAFETY: avx2 feature enabled; AlignedBuffer guarantees 64-byte alignment
    // (≥ required 32-byte AVX2 alignment); caller guarantees bounds + non-overlap.
    unsafe {
        let ptr = tableau.as_mut_ptr();
        let dst = ptr.add(dst_base) as *mut __m256i;
        let src = ptr.add(src_base) as *const __m256i;
        let chunks = count / 4; // 4 u64 per __m256i (256 bits)
        for i in 0..chunks {
            let a = _mm256_load_si256(dst.add(i));
            let b = _mm256_load_si256(src.add(i));
            _mm256_store_si256(dst.add(i), _mm256_xor_si256(a, b));
        }
    }
}

/// SSE2 path: 128-bit XOR, processing 2 u64 words per iteration.
///
/// Requires 16-byte alignment. Our `AlignedBuffer` provides 64-byte alignment.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn xor_rows_sse2(tableau: &mut [u64], dst_base: usize, src_base: usize, count: usize) {
    use std::arch::x86_64::{__m128i, _mm_load_si128, _mm_store_si128, _mm_xor_si128};
    // SAFETY: sse2 feature enabled; AlignedBuffer guarantees 64-byte alignment
    // (≥ required 16-byte SSE2 alignment); caller guarantees bounds + non-overlap.
    unsafe {
        let ptr = tableau.as_mut_ptr();
        let dst = ptr.add(dst_base) as *mut __m128i;
        let src = ptr.add(src_base) as *const __m128i;
        let chunks = count / 2; // 2 u64 per __m128i (128 bits)
        for i in 0..chunks {
            let a = _mm_load_si128(dst.add(i));
            let b = _mm_load_si128(src.add(i));
            _mm_store_si128(dst.add(i), _mm_xor_si128(a, b));
        }
    }
}

/// NEON path: 128-bit XOR, processing 2 u64 words per iteration.
/// NEON is mandatory on aarch64; no runtime feature detection required.
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn xor_rows_neon(tableau: &mut [u64], dst_base: usize, src_base: usize, count: usize) {
    use std::arch::aarch64::{veorq_u64, vld1q_u64, vst1q_u64};
    // SAFETY: NEON is mandatory on aarch64; caller guarantees bounds and non-overlap.
    unsafe {
        let ptr = tableau.as_mut_ptr();
        let dst = ptr.add(dst_base);
        let src = ptr.add(src_base) as *const u64;
        let chunks = count / 2; // 2 u64 per uint64x2_t (128 bits)
        for i in 0..chunks {
            let a = vld1q_u64(dst.add(i * 2) as *const u64);
            let b = vld1q_u64(src.add(i * 2));
            vst1q_u64(dst.add(i * 2), veorq_u64(a, b));
        }
    }
}

/// Scalar fallback: word loop that the compiler auto-vectorizes at -O2.
/// Used on unsupported architectures or when SIMD features are unavailable.
#[inline]
fn xor_rows_scalar(tableau: &mut [u64], dst_base: usize, src_base: usize, count: usize) {
    // SAFETY: use raw pointer to read src while dst is mutably borrowed from the
    // same slice.  Non-overlap is guaranteed by the caller.
    let ptr = tableau.as_mut_ptr();
    for w in 0..count {
        // SAFETY: dst_base+w < tableau.len() and src_base+w < tableau.len().
        unsafe { *ptr.add(dst_base + w) ^= *ptr.add(src_base + w) };
    }
}

/// Runtime-dispatched XOR of `count` u64 words from `src_base` into `dst_base`.
///
/// # Safety
/// * `dst_base + count ≤ tableau.len()` and `src_base + count ≤ tableau.len()`
/// * `[dst_base, dst_base+count)` and `[src_base, src_base+count)` must not overlap.
#[inline]
#[allow(unreachable_code)] // scalar tail unreachable on aarch64 (always takes NEON path)
unsafe fn xor_rows_dispatch(tableau: &mut [u64], dst_base: usize, src_base: usize, count: usize) {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            // SAFETY: avx2 confirmed; bounds + non-overlap guaranteed by caller.
            return unsafe { xor_rows_avx2(tableau, dst_base, src_base, count) };
        }
        if is_x86_feature_detected!("sse2") {
            // SAFETY: sse2 confirmed; bounds + non-overlap guaranteed by caller.
            return unsafe { xor_rows_sse2(tableau, dst_base, src_base, count) };
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        // SAFETY: NEON mandatory on aarch64; bounds + non-overlap guaranteed by caller.
        return unsafe { xor_rows_neon(tableau, dst_base, src_base, count) };
    }
    xor_rows_scalar(tableau, dst_base, src_base, count);
}

impl StabilizerState {
    /// Creates a new stabilizer state initialized to |0...0⟩.
    ///
    /// In the Aaronson-Gottesman convention the initial generators are:
    /// - Destabilizer `i` (row `i`):   X on qubit `i`, identity elsewhere, phase +1.
    /// - Stabilizer  `i` (row `n+i`):  Z on qubit `i`, identity elsewhere, phase +1.
    ///
    /// # Arguments
    /// * `n` — number of qubits
    pub fn new(n: usize) -> Self {
        assert!(n > 0, "StabilizerState requires at least 1 qubit");

        // Pad row_len to the next multiple of 8 u64 words (= 512 bits).
        let words_needed = n.div_ceil(64);
        let row_len = ((words_needed + 7) & !7).max(8);

        // Allocate 2n+1 rows: rows 0..n = destabilizers, n..2n = stabilizers,
        // row 2n = persistent scratch (used by deterministic measurement).
        let total_rows = 2 * n + 1;
        let total_words = total_rows * 2 * row_len;
        let mut tableau = AlignedBuffer::new_zeroed(total_words);
        let phases = vec![0u8; total_rows].into_boxed_slice();

        // Initialise destabilizers: row i has X bit set at column i.
        for i in 0..n {
            let word = i / 64;
            let bit = i % 64;
            tableau[Self::x_offset(i, i, row_len, word)] |= 1u64 << bit;
        }

        // Initialise stabilizers: row n+i has Z bit set at column i.
        for i in 0..n {
            let word = i / 64;
            let bit = i % 64;
            tableau[Self::z_offset(n + i, i, row_len, word)] |= 1u64 << bit;
        }

        StabilizerState {
            num_qubits: n,
            row_len,
            tableau,
            phases,
            rng: SmallRng::from_os_rng(),
        }
    }

    /// Copies tableau and phase data from `source` into `self` without allocating.
    ///
    /// The two states must have the same number of qubits (and therefore the same
    /// `row_len`). The RNG is **not** copied — the caller is expected to manage it
    /// separately (e.g., re-seeding for each shot).
    ///
    /// This is used by [`sample_shots`](Self::sample_shots) to reuse a single
    /// pre-allocated working buffer per Rayon worker thread.
    fn reset_from(&mut self, source: &StabilizerState) {
        debug_assert_eq!(self.num_qubits, source.num_qubits);
        debug_assert_eq!(self.row_len, source.row_len);
        self.tableau
            .as_mut_slice()
            .copy_from_slice(source.tableau.as_slice());
        self.phases.copy_from_slice(&source.phases);
    }

    /// Index of the persistent scratch row in the tableau (= `2n`).
    #[inline(always)]
    fn scratch_row(&self) -> usize {
        2 * self.num_qubits
    }

    /// Zeroes the scratch row's bit-columns and phase, ready for a new accumulation.
    fn clear_scratch(&mut self) {
        let s = self.scratch_row();
        let base = Self::row_base(s, self.row_len);
        for w in 0..2 * self.row_len {
            self.tableau[base + w] = 0;
        }
        self.phases[s] = 0;
    }

    /// Base byte offset (in u64 words) of row `row` in the tableau.
    #[inline(always)]
    fn row_base(row: usize, row_len: usize) -> usize {
        row * 2 * row_len
    }

    /// Flat index into `tableau` for the `word`-th u64 in the X-block of `row`,
    /// where `col_qubit` determines which X-block column we're accessing.
    /// (The `word` parameter is `col_qubit / 64`.)
    #[inline(always)]
    fn x_offset(row: usize, _col_qubit: usize, row_len: usize, word: usize) -> usize {
        Self::row_base(row, row_len) + word
    }

    /// Flat index into `tableau` for the `word`-th u64 in the Z-block of `row`.
    #[inline(always)]
    fn z_offset(row: usize, _col_qubit: usize, row_len: usize, word: usize) -> usize {
        Self::row_base(row, row_len) + row_len + word
    }

    /// Returns the X-bit of `row` at qubit column `q`.
    #[inline(always)]
    pub(crate) fn x_bit(&self, row: usize, q: usize) -> bool {
        let base = Self::row_base(row, self.row_len);
        let word = q / 64;
        let bit = q % 64;
        (self.tableau[base + word] >> bit) & 1 == 1
    }

    /// Returns the Z-bit of `row` at qubit column `q`.
    #[inline(always)]
    pub(crate) fn z_bit(&self, row: usize, q: usize) -> bool {
        let base = Self::row_base(row, self.row_len);
        let word = q / 64;
        let bit = q % 64;
        (self.tableau[base + self.row_len + word] >> bit) & 1 == 1
    }

    /// Sets or clears the X-bit of `row` at qubit column `q`.
    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) fn set_x_bit(&mut self, row: usize, q: usize, val: bool) {
        let base = Self::row_base(row, self.row_len);
        let word = q / 64;
        let bit = q % 64;
        if val {
            self.tableau[base + word] |= 1u64 << bit;
        } else {
            self.tableau[base + word] &= !(1u64 << bit);
        }
    }

    /// Sets or clears the Z-bit of `row` at qubit column `q`.
    #[inline(always)]
    pub(crate) fn set_z_bit(&mut self, row: usize, q: usize, val: bool) {
        let base = Self::row_base(row, self.row_len);
        let word = q / 64;
        let bit = q % 64;
        if val {
            self.tableau[base + self.row_len + word] |= 1u64 << bit;
        } else {
            self.tableau[base + self.row_len + word] &= !(1u64 << bit);
        }
    }

    /// Returns the encoded phase of row `row`.
    #[inline(always)]
    pub(crate) fn phase(&self, row: usize) -> u8 {
        self.phases[row]
    }

    /// Sets the phase for `row` to `val` (0–3 for +1, +i, −1, −i respectively).
    #[inline(always)]
    pub(crate) fn set_phase(&mut self, row: usize, val: u8) {
        self.phases[row] = val & 3;
    }

    pub(crate) fn validate_qubit(&self, q: usize) -> Result<(), QisError> {
        if self.num_qubits == 0 {
            return Err(QisError::InvalidParameterValue(
                "no valid qubit indices in a zero-qubit system".to_string(),
            ));
        }
        if q >= self.num_qubits {
            return Err(QisError::IndexOutOfBounds {
                index: q,
                max: self.num_qubits.saturating_sub(1),
            });
        }
        Ok(())
    }

    pub(crate) fn validate_two_qubits(&self, q0: usize, q1: usize) -> Result<(), QisError> {
        self.validate_qubit(q0)?;
        self.validate_qubit(q1)?;
        if q0 == q1 {
            return Err(QisError::InvalidParameterValue(
                "Two-qubit gate requires distinct qubit indices".to_string(),
            ));
        }
        Ok(())
    }

    /// Applies the CNOT (CX) gate with `control` as control and `target` as target.
    ///
    /// Aaronson-Gottesman rule (Eq. 11) for each row `i`:
    /// - `phase[i] ^= 2 * x[i][c] * z[i][t] * (x[i][t] ^ z[i][c] ^ 1)`
    /// - `x[i][t] ^= x[i][c]`
    /// - `z[i][c] ^= z[i][t]`
    pub fn apply_cx(&mut self, control: usize, target: usize) -> Result<(), QisError> {
        self.validate_two_qubits(control, target)?;
        let c_word = control / 64;
        let c_mask = 1u64 << (control % 64);
        let t_word = target / 64;
        let t_mask = 1u64 << (target % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            let xc = (self.tableau[base + c_word] & c_mask) != 0;
            let xt = (self.tableau[base + t_word] & t_mask) != 0;
            let zc = (self.tableau[base + rl + c_word] & c_mask) != 0;
            let zt = (self.tableau[base + rl + t_word] & t_mask) != 0;
            if xc && zt && (xt ^ zc ^ true) {
                self.phases[row] ^= 2;
            }
            if xc {
                self.tableau[base + t_word] ^= t_mask;
            }
            if zt {
                self.tableau[base + rl + c_word] ^= c_mask;
            }
        }
        Ok(())
    }

    /// Applies the SWAP gate between qubits `q0` and `q1`.
    ///
    /// Native rule: for each row `i`, swap (x[i][q0], z[i][q0]) with (x[i][q1], z[i][q1]).
    /// No phase change — SWAP maps Paulis without sign.
    pub fn apply_swap(&mut self, q0: usize, q1: usize) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        let w0 = q0 / 64;
        let m0 = 1u64 << (q0 % 64);
        let w1 = q1 / 64;
        let m1 = 1u64 << (q1 % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            // Read bits
            let x0 = (self.tableau[base + w0] & m0) != 0;
            let z0 = (self.tableau[base + rl + w0] & m0) != 0;
            let x1 = (self.tableau[base + w1] & m1) != 0;
            let z1 = (self.tableau[base + rl + w1] & m1) != 0;
            // Write swapped bits
            if x1 {
                self.tableau[base + w0] |= m0;
            } else {
                self.tableau[base + w0] &= !m0;
            }
            if z1 {
                self.tableau[base + rl + w0] |= m0;
            } else {
                self.tableau[base + rl + w0] &= !m0;
            }
            if x0 {
                self.tableau[base + w1] |= m1;
            } else {
                self.tableau[base + w1] &= !m1;
            }
            if z0 {
                self.tableau[base + rl + w1] |= m1;
            } else {
                self.tableau[base + rl + w1] &= !m1;
            }
        }
        Ok(())
    }

    /// Applies the CZ gate (controlled-Z) between `q0` and `q1`.
    ///
    /// Implemented as (I⊗H)·CNOT·(I⊗H) inlined per-row — no heap allocations.
    pub fn apply_cz(&mut self, q0: usize, q1: usize) -> Result<(), QisError> {
        self.validate_two_qubits(q0, q1)?;
        let w0 = q0 / 64;
        let m0 = 1u64 << (q0 % 64);
        let w1 = q1 / 64;
        let m1 = 1u64 << (q1 % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            // H on q1: swap x1 ↔ z1, phase if both set
            let x1 = (self.tableau[base + w1] & m1) != 0;
            let z1 = (self.tableau[base + rl + w1] & m1) != 0;
            if x1 && z1 {
                self.phases[row] ^= 2;
            }
            if z1 {
                self.tableau[base + w1] |= m1;
            } else {
                self.tableau[base + w1] &= !m1;
            }
            if x1 {
                self.tableau[base + rl + w1] |= m1;
            } else {
                self.tableau[base + rl + w1] &= !m1;
            }

            // CNOT(q0, q1)
            let xc = (self.tableau[base + w0] & m0) != 0;
            let xt = (self.tableau[base + w1] & m1) != 0;
            let zc = (self.tableau[base + rl + w0] & m0) != 0;
            let zt = (self.tableau[base + rl + w1] & m1) != 0;
            if xc && zt && (xt ^ zc ^ true) {
                self.phases[row] ^= 2;
            }
            if xc {
                self.tableau[base + w1] ^= m1;
            }
            if zt {
                self.tableau[base + rl + w0] ^= m0;
            }

            // H on q1 again
            let x1 = (self.tableau[base + w1] & m1) != 0;
            let z1 = (self.tableau[base + rl + w1] & m1) != 0;
            if x1 && z1 {
                self.phases[row] ^= 2;
            }
            if z1 {
                self.tableau[base + w1] |= m1;
            } else {
                self.tableau[base + w1] &= !m1;
            }
            if x1 {
                self.tableau[base + rl + w1] |= m1;
            } else {
                self.tableau[base + rl + w1] &= !m1;
            }
        }
        Ok(())
    }

    /// Applies the CY gate (controlled-Y) between `control` and `target`.
    ///
    /// Implemented as (I⊗S)·CX·(I⊗S†) inlined per-row, i.e. in time order:
    /// S†(target) → CX(control, target) → S(target).
    ///
    /// Derivation: Y = SXS†, so CY = (I⊗S)·CX·(I⊗S†).
    pub fn apply_cy(&mut self, control: usize, target: usize) -> Result<(), QisError> {
        self.validate_two_qubits(control, target)?;
        let cw = control / 64;
        let cm = 1u64 << (control % 64);
        let tw = target / 64;
        let tm = 1u64 << (target % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            // S† on target: phase flip when x=1 and z=0 (i.e. X → -Y), then z ^= x
            let x = (self.tableau[base + tw] & tm) != 0;
            let z = (self.tableau[base + rl + tw] & tm) != 0;
            if x && !z {
                self.phases[row] ^= 2;
            }
            if x {
                self.tableau[base + rl + tw] ^= tm;
            }

            // CNOT(control, target)
            let xc = (self.tableau[base + cw] & cm) != 0;
            let xt = (self.tableau[base + tw] & tm) != 0;
            let zc = (self.tableau[base + rl + cw] & cm) != 0;
            let zt = (self.tableau[base + rl + tw] & tm) != 0;
            if xc && zt && (xt ^ zc ^ true) {
                self.phases[row] ^= 2;
            }
            if xc {
                self.tableau[base + tw] ^= tm;
            }
            if zt {
                self.tableau[base + rl + cw] ^= cm;
            }

            // S on target: phase flip when x=1 and z=1 (i.e. Y → -X), then z ^= x
            let x = (self.tableau[base + tw] & tm) != 0;
            let z = (self.tableau[base + rl + tw] & tm) != 0;
            if x && z {
                self.phases[row] ^= 2;
            }
            if x {
                self.tableau[base + rl + tw] ^= tm;
            }
        }
        Ok(())
    }

    /// Returns the number of qubits.
    pub fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    /// Applies the Hadamard gate to qubit `qubit`.
    ///
    /// Aaronson-Gottesman update rule for each row `i` (0..2n):
    /// - `phase[i] ^= 2 * x[i][qubit] & z[i][qubit]`  (adds 2 mod 4 when Y, i.e. sign flip)
    /// - swap `x[i][qubit]` and `z[i][qubit]`
    pub fn apply_h(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let word = qubit / 64;
        let mask = 1u64 << (qubit % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            let x = (self.tableau[base + word] & mask) != 0;
            let z = (self.tableau[base + rl + word] & mask) != 0;
            if x && z {
                self.phases[row] ^= 2;
            }
            // Swap x and z bits.
            if x {
                self.tableau[base + rl + word] |= mask;
            } else {
                self.tableau[base + rl + word] &= !mask;
            }
            if z {
                self.tableau[base + word] |= mask;
            } else {
                self.tableau[base + word] &= !mask;
            }
        }
        Ok(())
    }

    /// Applies the S gate (phase gate, √Z) to qubit `qubit`.
    ///
    /// Aaronson-Gottesman rule for each row `i`:
    /// - `phase[i] ^= 2 * x[i][qubit] * z[i][qubit]`
    /// - `z[i][qubit] ^= x[i][qubit]`
    ///
    /// Effect: X→Y, Y→-X, Z→Z.
    pub fn apply_s(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let word = qubit / 64;
        let mask = 1u64 << (qubit % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            let x = (self.tableau[base + word] & mask) != 0;
            let z = (self.tableau[base + rl + word] & mask) != 0;
            if x && z {
                self.phases[row] ^= 2;
            }
            if x {
                // z ^= x (x=1): flip z bit
                self.tableau[base + rl + word] ^= mask;
            }
        }
        Ok(())
    }

    /// Applies the S† gate (S-dagger, inverse phase gate) to qubit `qubit`.
    ///
    /// Native rule for each row `i` (derived from S†: X→-Y, Y→X, Z→Z):
    /// - `phase[i] ^= 2 * x[i][qubit] * (1 - z[i][qubit])`  (flip sign when X but not yet Y)
    /// - `z[i][qubit] ^= x[i][qubit]`
    pub fn apply_sdg(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let word = qubit / 64;
        let mask = 1u64 << (qubit % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            let x = (self.tableau[base + word] & mask) != 0;
            let z = (self.tableau[base + rl + word] & mask) != 0;
            if x && !z {
                self.phases[row] ^= 2;
            }
            if x {
                self.tableau[base + rl + word] ^= mask;
            }
        }
        Ok(())
    }

    /// Applies the Pauli-X (bit-flip) gate to qubit `qubit`.
    ///
    /// Effect: X→X, Y→-Y, Z→-Z.
    /// Rule per row `i`: `phase[i] ^= 2 * z[i][qubit]`. No bit changes.
    pub fn apply_x(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let word = qubit / 64;
        let mask = 1u64 << (qubit % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            if (self.tableau[base + rl + word] & mask) != 0 {
                self.phases[row] ^= 2;
            }
        }
        Ok(())
    }

    /// Applies the Pauli-Z (phase-flip) gate to qubit `qubit`.
    ///
    /// Effect: X→-X, Y→-Y, Z→Z.
    /// Rule per row `i`: `phase[i] ^= 2 * x[i][qubit]`. No bit changes.
    pub fn apply_z(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let word = qubit / 64;
        let mask = 1u64 << (qubit % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            if (self.tableau[base + word] & mask) != 0 {
                self.phases[row] ^= 2;
            }
        }
        Ok(())
    }

    /// Applies the Pauli-Y gate to qubit `qubit`.
    ///
    /// Effect: X→-X, Y→Y, Z→-Z.
    /// Rule per row `i`: `phase[i] ^= 2 * (x[i][qubit] XOR z[i][qubit])`. No bit changes.
    pub fn apply_y(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let word = qubit / 64;
        let mask = 1u64 << (qubit % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            let x = (self.tableau[base + word] & mask) != 0;
            let z = (self.tableau[base + rl + word] & mask) != 0;
            if x ^ z {
                self.phases[row] ^= 2;
            }
        }
        Ok(())
    }

    /// Applies the √X gate (X2P, SX, Rx(π/2)) to qubit `qubit`.
    ///
    /// Conjugation: X→X, Y→Z, Z→−Y.
    /// Rule per row `i`:
    /// - `phase[i] ^= 2 * (!x[i][qubit] & z[i][qubit])` (flip for Z→−Y)
    /// - `x[i][qubit] ^= z[i][qubit]`
    pub fn apply_x2p(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let word = qubit / 64;
        let mask = 1u64 << (qubit % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            let x = (self.tableau[base + word] & mask) != 0;
            let z = (self.tableau[base + rl + word] & mask) != 0;
            if !x && z {
                self.phases[row] ^= 2;
            }
            // x ^= z
            if z {
                self.tableau[base + word] ^= mask;
            }
        }
        Ok(())
    }

    /// Applies the √X† gate (X2M, SXdg, Rx(−π/2)) to qubit `qubit`.
    ///
    /// Conjugation: X→X, Y→−Z, Z→Y.
    /// Rule per row `i`:
    /// - `phase[i] ^= 2 * (x[i][qubit] & z[i][qubit])` (flip for Y→−Z)
    /// - `x[i][qubit] ^= z[i][qubit]`
    pub fn apply_x2m(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let word = qubit / 64;
        let mask = 1u64 << (qubit % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            let x = (self.tableau[base + word] & mask) != 0;
            let z = (self.tableau[base + rl + word] & mask) != 0;
            if x && z {
                self.phases[row] ^= 2;
            }
            if z {
                self.tableau[base + word] ^= mask;
            }
        }
        Ok(())
    }

    /// Applies the √Y gate (Y2P, SY, Ry(π/2)) to qubit `qubit`.
    ///
    /// Conjugation: X→Z, Y→Y, Z→−X.
    /// Rule per row `i`:
    /// - `phase[i] ^= 2 * (!x[i][qubit] & z[i][qubit])` (flip for Z→−X)
    /// - swap `x[i][qubit]` ↔ `z[i][qubit]`
    pub fn apply_y2p(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let word = qubit / 64;
        let mask = 1u64 << (qubit % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            let x = (self.tableau[base + word] & mask) != 0;
            let z = (self.tableau[base + rl + word] & mask) != 0;
            if !x && z {
                self.phases[row] ^= 2;
            }
            // swap x ↔ z
            if x {
                self.tableau[base + rl + word] |= mask;
            } else {
                self.tableau[base + rl + word] &= !mask;
            }
            if z {
                self.tableau[base + word] |= mask;
            } else {
                self.tableau[base + word] &= !mask;
            }
        }
        Ok(())
    }

    /// Applies the √Y† gate (Y2M, SYdg, Ry(−π/2)) to qubit `qubit`.
    ///
    /// Conjugation: X→−Z, Y→Y, Z→X.
    /// Rule per row `i`:
    /// - `phase[i] ^= 2 * (x[i][qubit] & !z[i][qubit])` (flip for X→−Z)
    /// - swap `x[i][qubit]` ↔ `z[i][qubit]`
    pub fn apply_y2m(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        let word = qubit / 64;
        let mask = 1u64 << (qubit % 64);
        let rl = self.row_len;
        for row in 0..(2 * self.num_qubits) {
            let base = row * 2 * rl;
            let x = (self.tableau[base + word] & mask) != 0;
            let z = (self.tableau[base + rl + word] & mask) != 0;
            if x && !z {
                self.phases[row] ^= 2;
            }
            // swap x ↔ z
            if x {
                self.tableau[base + rl + word] |= mask;
            } else {
                self.tableau[base + rl + word] &= !mask;
            }
            if z {
                self.tableau[base + word] |= mask;
            } else {
                self.tableau[base + word] &= !mask;
            }
        }
        Ok(())
    }

    /// Phase contribution from multiplying Pauli `P_h` (from h-row) by `P_i` (from i-row).
    ///
    /// Returns an integer exponent `e` such that `P_h * P_i = i^e * P_result`.
    /// Kept for reference/testing; the hot path uses `g_phase_word` instead.
    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) fn g_phase(x1: bool, z1: bool, x2: bool, z2: bool) -> i32 {
        match (x1, z1) {
            (false, false) => 0,
            (true, true) => z2 as i32 - x2 as i32,
            (true, false) => z2 as i32 * (2 * x2 as i32 - 1),
            (false, true) => x2 as i32 * (1 - 2 * z2 as i32),
        }
    }

    /// Accumulates the g_phase contribution of a single 64-bit word pair using
    /// word-level popcount — **2 `count_ones()` calls** per 64-bit word.
    ///
    /// Algebraic derivation:
    /// - `anti_comm = (xh & zi) XOR (xi & zh)` — positions where the two Paulis
    ///   anti-commute (anti-commuting ↔ g ≠ 0).
    /// - `pos = (xh & zi & (zh XOR xi)) | (!xh & zh & xi & !zi)` — positions
    ///   where g = +1 (the "forward cyclic" Pauli pairs X→Y, Y→Z, Z→X).
    /// - `sum = 2 · popcount(pos) − popcount(anti_comm)`
    ///   (since popcount(neg) = popcount(anti_comm) − popcount(pos),
    ///   and popcount(pos) − popcount(neg) = 2·popcount(pos) − popcount(anti_comm)).
    ///
    /// Verified against the scalar `g_phase` for all 9 single-qubit Pauli pairs.
    #[inline(always)]
    pub(crate) fn g_phase_word(xh: u64, zh: u64, xi: u64, zi: u64) -> i32 {
        // Positions where the Paulis anti-commute (g ≠ 0).
        let anti_comm = (xh & zi) ^ (xi & zh);
        // Positions where g = +1 (cyclic order X→Y, Y→Z, Z→X).
        let pos = (xh & zi & (zh ^ xi)) | (!xh & zh & xi & !zi);
        2 * pos.count_ones() as i32 - anti_comm.count_ones() as i32
    }

    /// Multiplies row `h` by row `i` in-place: `row_h := row_h * row_i`.
    ///
    /// Phase is computed via `g_phase_word` (word-level popcount, O(n/64) vs O(n)).
    /// Bit XOR is dispatched to the best available SIMD instruction set at runtime.
    fn rowsum(&mut self, h: usize, i: usize) {
        debug_assert_ne!(h, i, "rowsum requires distinct rows");

        let h_base = Self::row_base(h, self.row_len);
        let i_base = Self::row_base(i, self.row_len);
        let rl = self.row_len;

        let mut sum = self.phases[h] as i32 + self.phases[i] as i32;
        // Only the active words (0..n_words) contain qubit data; the padding words
        // in the range n_words..row_len are always zero and contribute 0 to the sum.
        let n_words = self.num_qubits.div_ceil(64);
        for w in 0..n_words {
            let xh = self.tableau[h_base + w];
            let zh = self.tableau[h_base + rl + w];
            let xi = self.tableau[i_base + w];
            let zi = self.tableau[i_base + rl + w];
            sum += Self::g_phase_word(xh, zh, xi, zi);
        }
        // New phase is sum mod 4, normalised to {0,1,2,3}.
        self.phases[h] = (((sum % 4) + 4) % 4) as u8;

        // SAFETY: h ≠ i guarantees [h_base, h_base+2*rl) and [i_base, i_base+2*rl)
        // are non-overlapping regions within the same 64-byte aligned allocation.
        unsafe {
            xor_rows_dispatch(self.tableau.as_mut_slice(), h_base, i_base, 2 * rl);
        }
    }

    /// Measures qubit `qubit` in the Z basis, collapsing the state.
    ///
    /// Returns `false` for outcome `0` (+1 eigenvalue) and `true` for outcome `1` (-1 eigenvalue).
    ///
    /// # Algorithm (Aaronson-Gottesman §IV)
    ///
    /// **Random case** — some stabilizer generator has X or Y on qubit `qubit`:
    /// 1. Find any stabilizer row `p` (index `n..2n`) with `x[p][qubit] = 1`.
    /// 2. XOR row `p` into every other row that also has `x[.][qubit] = 1`.
    /// 3. Move row `p` to its paired destabilizer row `p - n`.
    /// 4. Replace stabilizer row `p` with `±Z_qubit`, where the sign is random.
    ///
    /// **Deterministic case** — all stabilizers commute with `Z_qubit`:
    /// 1. Compute the product of destabilizer rows `i` (for which the paired
    ///    stabilizer `n+i` has `x[n+i][qubit] = 1`) into a scratch accumulator.
    /// 2. The measurement outcome is the phase of the scratch row.
    pub fn measure(&mut self, qubit: usize) -> Result<bool, QisError> {
        self.validate_qubit(qubit)?;

        // Find first stabilizer row with x[.][qubit] = 1.
        let maybe_p = (self.num_qubits..2 * self.num_qubits).find(|&row| self.x_bit(row, qubit));

        match maybe_p {
            Some(p) => {
                // Make p the unique row with x[.][qubit] = 1 by XOR-ing p into all others.
                for i in 0..2 * self.num_qubits {
                    if i != p && self.x_bit(i, qubit) {
                        self.rowsum(i, p);
                    }
                }

                // Copy stabilizer p → destabilizer p-n.
                let dest = p - self.num_qubits;
                let p_base = Self::row_base(p, self.row_len);
                let d_base = Self::row_base(dest, self.row_len);
                // SAFETY: dest ≠ p (since dest < n ≤ p < 2n).
                unsafe {
                    let src = self.tableau.as_slice()[p_base..p_base + 2 * self.row_len].as_ptr();
                    let dst =
                        self.tableau.as_mut_slice()[d_base..d_base + 2 * self.row_len].as_mut_ptr();
                    std::ptr::copy_nonoverlapping(src, dst, 2 * self.row_len);
                }
                self.phases[dest] = self.phases[p];

                // Reset stabilizer p to Z_qubit with random phase.
                let p_base = Self::row_base(p, self.row_len);
                for w in 0..2 * self.row_len {
                    self.tableau[p_base + w] = 0;
                }
                self.set_z_bit(p, qubit, true);
                let b: bool = self.rng.random();
                self.set_phase(p, if b { 2 } else { 0 });
                Ok(b)
            }
            None => {
                // Accumulate the product of paired stabilizer rows into the
                // pre-allocated scratch row (row 2n) using the SIMD-accelerated
                // rowsum, giving O(n²/w) complexity instead of O(n²) scalar work.
                let scratch = self.scratch_row();
                self.clear_scratch();

                for i in 0..self.num_qubits {
                    // Include destabilizer i only if it has an X-component at qubit `qubit`.
                    // Then rowsum the paired stabilizer n+i into scratch.
                    if self.x_bit(i, qubit) {
                        self.rowsum(scratch, self.num_qubits + i);
                    }
                }

                // Phase 0 = +1 eigenvalue (outcome 0); phase 2 = -1 eigenvalue (outcome 1).
                Ok(self.phases[scratch] == 2)
            }
        }
    }

    /// Measures all qubits in order, returning a bit-packed [`Outcome`].
    ///
    /// Qubit `q`'s result is stored at bit `q` inside the `Outcome`.
    /// Use [`Outcome::is_one(q)`](crate::device::Outcome::is_one) to read it.
    pub fn measure_all(&mut self) -> Outcome {
        let num_chunks = self.num_qubits.div_ceil(64);
        let mut chunks = SmallVec::from_elem(0u64, num_chunks);
        for qubit in 0..self.num_qubits {
            if self.measure(qubit).unwrap() {
                chunks[qubit / 64] |= 1u64 << (qubit % 64);
            }
        }
        Outcome::new(chunks)
    }

    /// Resets `qubit` to the |0⟩ state.
    ///
    /// Implemented by measuring in the Z basis and applying X if the outcome is |1⟩.
    /// This correctly handles both deterministic and random measurement cases.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cqlib_core::qis::StabilizerState;
    ///
    /// let mut s = StabilizerState::new(2);
    /// s.apply_x(0).unwrap();  // |0⟩ → |1⟩
    /// s.reset(0).unwrap();    // back to |0⟩
    /// assert_eq!(s.probabilities().unwrap()[0], 1.0); // P(|00⟩) = 1
    /// ```
    pub fn reset(&mut self, qubit: usize) -> Result<(), QisError> {
        self.validate_qubit(qubit)?;
        if self.measure(qubit)? {
            self.apply_x(qubit)?;
        }
        Ok(())
    }

    /// Performs a forced (post-selected) Z-basis measurement on `qubit`.
    ///
    /// Like `measure`, but the outcome is forced to `outcome` rather than random.
    /// Returns `true` if the forced outcome has non-zero probability (measurement is
    /// consistent with the state), `false` if probability is zero (outcome is impossible).
    ///
    /// Used internally by `probability_of`.
    fn force_measure(&mut self, qubit: usize, outcome: bool) -> bool {
        // Find first stabilizer row with x[.][qubit] = 1.
        let maybe_p = (self.num_qubits..2 * self.num_qubits).find(|&row| self.x_bit(row, qubit));

        match maybe_p {
            Some(p) => {
                // Random measurement: force to `outcome`.
                for i in 0..2 * self.num_qubits {
                    if i != p && self.x_bit(i, qubit) {
                        self.rowsum(i, p);
                    }
                }
                let dest = p - self.num_qubits;
                let p_base = Self::row_base(p, self.row_len);
                let d_base = Self::row_base(dest, self.row_len);
                // SAFETY: dest < n ≤ p.
                unsafe {
                    let src = self.tableau.as_slice()[p_base..p_base + 2 * self.row_len].as_ptr();
                    let dst =
                        self.tableau.as_mut_slice()[d_base..d_base + 2 * self.row_len].as_mut_ptr();
                    std::ptr::copy_nonoverlapping(src, dst, 2 * self.row_len);
                }
                self.phases[dest] = self.phases[p];
                let p_base = Self::row_base(p, self.row_len);
                for w in 0..2 * self.row_len {
                    self.tableau[p_base + w] = 0;
                }
                self.set_z_bit(p, qubit, true);
                self.set_phase(p, if outcome { 2 } else { 0 });
                true // random case: forced outcome is always possible
            }
            None => {
                // Deterministic case: check whether outcome matches.
                let scratch = self.scratch_row();
                self.clear_scratch();
                for i in 0..self.num_qubits {
                    if self.x_bit(i, qubit) {
                        self.rowsum(scratch, self.num_qubits + i);
                    }
                }
                let actual = self.phases[scratch] == 2;
                actual == outcome // false → this outcome is impossible
            }
        }
    }

    /// Returns the probability of measuring the given computational basis state.
    ///
    /// Performs O(n²) work per qubit, O(n³) total, on a cloned (non-destructive) copy.
    ///
    /// # Returns
    /// - `Ok(0.0)` if the bitstring is inconsistent with the stabilizer state.
    /// - `Ok(1.0 / 2.0^k)` where `k` is the number of qubits with random outcomes,
    ///   if the bitstring is in the support.
    /// - `Err(QubitMismatch)` if `bits.len() != n`.
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::qis::StabilizerState;
    ///
    /// let mut s = StabilizerState::new(2);
    /// s.apply_h(0).unwrap();
    /// s.apply_cx(0, 1).unwrap(); // Bell state |Φ⁺⟩
    ///
    /// // Only |00⟩ and |11⟩ are possible, each with probability 0.5
    /// assert!((s.probability_of(&[false, false]).unwrap() - 0.5).abs() < 1e-10);
    /// assert!((s.probability_of(&[true,  true ]).unwrap() - 0.5).abs() < 1e-10);
    /// assert_eq!(s.probability_of(&[false, true]).unwrap(), 0.0);
    /// assert_eq!(s.probability_of(&[true,  false]).unwrap(), 0.0);
    /// ```
    pub fn probability_of(&self, bits: &[bool]) -> Result<f64, QisError> {
        if bits.len() != self.num_qubits {
            return Err(QisError::QubitMismatch {
                expected: self.num_qubits,
                actual: bits.len(),
            });
        }
        let mut s = self.clone();
        let mut prob = 1.0_f64;
        for (qubit, &desired) in bits.iter().enumerate() {
            // Check if measurement is random (some stabilizer has X on this qubit).
            let is_random = (s.num_qubits..2 * s.num_qubits).any(|row| s.x_bit(row, qubit));
            if is_random {
                prob *= 0.5;
                s.force_measure(qubit, desired);
            } else {
                // Deterministic: force_measure returns false if outcome is impossible.
                if !s.force_measure(qubit, desired) {
                    return Ok(0.0);
                }
            }
        }
        Ok(prob)
    }

    /// Returns the full probability distribution over all computational basis states.
    ///
    /// Returns a `Vec<f64>` of length `2^n` where entry `i` is the probability of
    /// measuring the state whose binary representation is `i` (qubit 0 = LSB).
    ///
    /// # Limitations
    /// Only feasible for small systems. Returns `Err(InvalidParameterValue)` if `n > 20`
    /// (which would require a 8 MB vector and O(2^20 × n³) computation).
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::qis::StabilizerState;
    ///
    /// let mut s = StabilizerState::new(2);
    /// s.apply_h(0).unwrap();
    /// s.apply_cx(0, 1).unwrap(); // Bell state
    ///
    /// let probs = s.probabilities().unwrap();
    /// assert_eq!(probs.len(), 4);
    /// assert!((probs[0b00] - 0.5).abs() < 1e-10); // |00⟩
    /// assert_eq!(probs[0b01], 0.0);                // |10⟩ (qubit-0 is LSB)
    /// assert_eq!(probs[0b10], 0.0);                // |01⟩
    /// assert!((probs[0b11] - 0.5).abs() < 1e-10); // |11⟩
    /// ```
    pub fn probabilities(&self) -> Result<Vec<f64>, QisError> {
        const MAX_QUBITS: usize = 20;
        if self.num_qubits > MAX_QUBITS {
            return Err(QisError::InvalidParameterValue(format!(
                "probabilities() is only supported for n ≤ {MAX_QUBITS} qubits \
                 (requested n = {}); use sample_shots() for large systems",
                self.num_qubits
            )));
        }
        let size = 1usize << self.num_qubits;
        let mut probs = vec![0.0_f64; size];
        for (i, prob) in probs.iter_mut().enumerate() {
            let bits: Vec<bool> = (0..self.num_qubits).map(|q| (i >> q) & 1 == 1).collect();
            *prob = self.probability_of(&bits)?;
        }
        Ok(probs)
    }

    /// Runs `shots` independent measurements in parallel using Rayon.
    ///
    /// Returns a [`Vec<Outcome>`] of bit-packed measurement results, one per shot.
    /// Each Rayon worker thread reuses a single pre-allocated working copy of the
    /// state (via [`reset_from`](Self::reset_from)), avoiding per-shot heap allocation.
    ///
    /// Seeds for each shot are derived sequentially from the primary RNG (ensuring
    /// reproducibility), then each shot runs on an independently-seeded clone in
    /// parallel. This produces the correct marginal distribution while being
    /// deterministic: the same initial state always yields the same joint sample set.
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::qis::StabilizerState;
    /// let mut s = StabilizerState::new(2);
    /// s.apply_h(0).unwrap();
    /// s.apply_cx(0, 1).unwrap(); // Bell state
    /// let results = s.sample_shots(1000);
    /// assert_eq!(results.len(), 1000);
    /// for shot in &results { assert_eq!(shot.is_one(0), shot.is_one(1)); }
    /// ```
    pub fn sample_shots(&self, shots: usize) -> Vec<Outcome> {
        // Derive independent seeds sequentially so the overall sequence is
        // deterministic and reproducible from the same initial RNG state.
        let seeds: Vec<u64> = {
            let mut rng = self.rng.clone();
            (0..shots).map(|_| rng.random()).collect()
        };
        seeds
            .into_par_iter()
            .map_with(self.clone(), |work, seed| {
                work.reset_from(self);
                work.rng = SmallRng::seed_from_u64(seed);
                work.measure_all()
            })
            .collect()
    }

    /// Samples the state using a circuit [`Measurement`] as the output contract.
    ///
    /// Unlike [`apply_circuit`](Self::apply_circuit), this method does not read
    /// or execute the circuit IR. The [`Measurement`] already carries the qubits
    /// and their result bit order:
    /// - `measurement.qubits()[i]` becomes bit `i` in each [`Outcome`].
    /// - [`Outcome::to_bitstring`] displays the most-significant result bit first,
    ///   so string order is the reverse of `measurement.qubits()`.
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::circuit::{Circuit, Qubit};
    /// use cqlib_core::qis::StabilizerState;
    ///
    /// let mut c = Circuit::new(2);
    /// c.h(Qubit::new(0)).unwrap();
    /// c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    /// let out = c.measure_bits([Qubit::new(1), Qubit::new(0)]).unwrap();
    ///
    /// // `from_circuit` ignores terminal measurements and keeps the Bell state.
    /// let state = StabilizerState::from_circuit(&c).unwrap();
    /// let result = state.sample(&out, 1000).unwrap();
    ///
    /// assert_eq!(result.shots(), 1000);
    /// assert!(result
    ///     .counts()
    ///     .keys()
    ///     .all(|bits| bits.to_bitstring(out.width()) == "00"
    ///         || bits.to_bitstring(out.width()) == "11"));
    /// ```
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
            "stabilizer-sample".to_string(),
            measurement.qubits().to_vec(),
            shots,
            measurement.width(),
            Some("stabilizer".to_string()),
            counts,
        ))
    }

    /// Returns the probability distribution selected by a circuit [`Measurement`].
    ///
    /// This is a marginal distribution over `measurement.qubits()`, not the full
    /// `2^n` state distribution. The returned [`Outcome`] keys use the same bit
    /// order as [`sample`](Self::sample).
    ///
    /// v1 intentionally reuses [`probabilities`](Self::probabilities), so it has
    /// the same `n <= 20` limit. Use [`sample`](Self::sample) for larger states.
    pub fn probs(&self, measurement: &Measurement) -> Result<HashMap<Outcome, f64>, QisError> {
        measurement.check_qubits(self.num_qubits)?;

        let mut marginal = HashMap::new();
        for (basis, prob) in self.probabilities()?.into_iter().enumerate() {
            if prob == 0.0 {
                continue;
            }
            *marginal
                .entry(measurement.project_basis(basis))
                .or_insert(0.0) += prob;
        }
        Ok(marginal)
    }

    /// Constructs a `StabilizerState` by simulating a Clifford circuit.
    ///
    /// This state-level entry point only performs quantum state evolution.
    /// Measurement and store operations produced by `Circuit::measure*` are
    /// treated as output declarations and ignored here: they do not collapse
    /// the state and do not populate runtime classical data.
    ///
    /// Use [`sample`](Self::sample) or [`probs`](Self::probs) with the returned
    /// [`Measurement`] to query measurement distributions from the final state.
    /// Use [`apply_circuit`](Self::apply_circuit) when you need execution
    /// semantics where measurements collapse the state and write classical data.
    ///
    /// # Supported instructions
    /// - Gates: `I, H, X, Y, Z, S, SDG, X2P, X2M, Y2P, Y2M, CX, CY, CZ, SWAP`
    /// - Classical data: `MeasureBit`, `MeasureBits`, `Store` are ignored
    /// - Directives: `Reset` (returns qubit to |0⟩), `Barrier`/`Delay` (no-op)
    ///
    /// [`apply_circuit`]: StabilizerState::apply_circuit
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::circuit::{Circuit, Qubit};
    /// use cqlib_core::qis::StabilizerState;
    ///
    /// let mut c = Circuit::new(2);
    /// c.h(Qubit::new(0)).unwrap();
    /// c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    /// let out = c
    ///     .measure_bits([Qubit::new(1), Qubit::new(0)])
    ///     .unwrap();
    ///
    /// // The measurement above is an output declaration for state sampling.
    /// // It does not collapse the Bell state during `from_circuit`.
    /// let stab = StabilizerState::from_circuit(&c).unwrap();
    /// let probs = stab.probs(&out).unwrap();
    /// assert!((probs.values().sum::<f64>() - 1.0).abs() < 1e-10);
    /// ```
    pub fn from_circuit(circuit: &Circuit) -> Result<Self, QisError> {
        let mut state = StabilizerState::new(circuit.num_qubits());
        state.apply_circuit(circuit)?;
        Ok(state)
    }

    /// Applies a Clifford circuit to this stabilizer state in-place.
    ///
    /// This state-level entry point ignores terminal measurement declarations,
    /// matching [`from_circuit`](Self::from_circuit). Use
    /// [`run_circuit`](Self::run_circuit) when runtime classical measurement
    /// values are required.
    pub fn apply_circuit(&mut self, input_circuit: &Circuit) -> Result<(), QisError> {
        if input_circuit.num_qubits() != self.num_qubits {
            return Err(QisError::InvalidStateDimension(input_circuit.num_qubits()));
        }
        if input_circuit
            .operations()
            .iter()
            .any(|op| matches!(op.instruction, Instruction::ClassicalControl(_)))
        {
            return Err(QisError::UnsupportedOperation(
                "classical control flow is not supported in stabilizer simulation".to_string(),
            ));
        }

        let circuit = input_circuit.decompose()?;
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

        let mut classical = ClassicalState::for_circuit(&circuit);
        StabilizerState::execute_operations(
            self,
            circuit.operations(),
            &qubit_map,
            &parameter_values,
            &mut classical,
            CircuitClassicalMode::Ignore,
        )
    }

    /// Executes a Clifford circuit and returns both the final state and runtime
    /// classical data.
    ///
    /// Returns a [`CircuitExecutionResult`] containing:
    /// - `state`: the final [`StabilizerState`] after all operations
    /// - `classical`: measurement results and mutable variables indexed by
    ///   [`crate::circuit::ClassicalValue`] and [`crate::circuit::ClassicalVar`]
    ///
    /// # Supported instructions
    /// Same quantum gates as [`from_circuit`]. Unlike `from_circuit`, this
    /// method executes `MeasureBit`, `MeasureBits`, and `Store`: measurements
    /// collapse the state immediately and write runtime classical data.
    /// Control-flow gates are not supported.
    ///
    /// [`from_circuit`]: StabilizerState::from_circuit
    ///
    /// # Example
    /// ```rust
    /// use cqlib_core::circuit::{Circuit, ClassicalExpr, ClassicalType, Qubit};
    /// use cqlib_core::qis::{RuntimeValue, StabilizerState};
    ///
    /// let mut c = Circuit::new(1);
    /// c.x(Qubit::new(0)).unwrap();
    ///
    /// // `measure` creates an immutable runtime value.
    /// let measured = c.measure(Qubit::new(0)).unwrap();
    ///
    /// // `store` can copy or transform measured values into mutable variables.
    /// let flag = c.var(ClassicalType::Bool);
    /// c.store(flag, ClassicalExpr::bit_to_bool(measured.expr()).unwrap()).unwrap();
    ///
    /// let result = StabilizerState::run_circuit(&c).unwrap();
    /// assert_eq!(result.classical.value(measured.value()), Some(&RuntimeValue::Bit(true)));
    /// assert_eq!(result.classical.var(flag), Some(&RuntimeValue::Bool(true)));
    /// ```
    pub fn run_circuit(circuit: &Circuit) -> Result<CircuitExecutionResult, QisError> {
        StabilizerState::run_circuit_with_mode(circuit, CircuitClassicalMode::Execute)
    }

    fn run_circuit_with_mode(
        input_circuit: &Circuit,
        classical_mode: CircuitClassicalMode,
    ) -> Result<CircuitExecutionResult, QisError> {
        if input_circuit
            .operations()
            .iter()
            .any(|op| matches!(op.instruction, Instruction::ClassicalControl(_)))
        {
            return Err(QisError::UnsupportedOperation(
                "classical control flow is not supported in stabilizer simulation".to_string(),
            ));
        }

        let circuit = input_circuit.decompose()?;
        let mut state = StabilizerState::new(circuit.num_qubits());

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

        let mut classical = ClassicalState::for_circuit(&circuit);

        StabilizerState::execute_operations(
            &mut state,
            circuit.operations(),
            &qubit_map,
            &parameter_values,
            &mut classical,
            classical_mode,
        )?;
        classical.rebind_to_circuit(input_circuit)?;

        Ok(CircuitExecutionResult { state, classical })
    }

    /// Recursive operation executor used by [`apply_circuit`].
    ///
    /// Processes a slice of operations, updating the quantum state and runtime
    /// classical state in place.
    ///
    /// [`apply_circuit`]: StabilizerState::apply_circuit
    fn execute_operations(
        state: &mut StabilizerState,
        ops: &[Operation],
        qubit_map: &std::collections::HashMap<Qubit, usize>,
        parameter_values: &[Option<f64>],
        classical: &mut ClassicalState,
        classical_mode: CircuitClassicalMode,
    ) -> Result<(), QisError> {
        for op in ops {
            let qubit_indices: Vec<usize> =
                op.qubits
                    .iter()
                    .map(|q| {
                        qubit_map.get(q).copied().ok_or_else(|| {
                            QisError::CircuitError(CircuitError::QubitNotFound(q.id()))
                        })
                    })
                    .collect::<Result<_, _>>()?;

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
                .collect::<Result<_, _>>()?;

            match &op.instruction {
                Instruction::Standard(gate) => {
                    state.apply_clifford_gate(*gate, &qubit_indices, &params)?;
                }
                Instruction::McGate(mc) if mc.num_ctrl_qubits() == 1 => {
                    let ctrl = qubit_indices[0];
                    let tgt = qubit_indices[1];
                    match mc.base_gate() {
                        StandardGate::X => state.apply_cx(ctrl, tgt)?,
                        StandardGate::Y => state.apply_cy(ctrl, tgt)?,
                        StandardGate::Z => state.apply_cz(ctrl, tgt)?,
                        g => {
                            return Err(QisError::UnsupportedOperation(format!(
                                "Controlled-{g} is not a Clifford gate"
                            )));
                        }
                    }
                }
                Instruction::McGate(mc) => {
                    return Err(QisError::UnsupportedOperation(format!(
                        "{}-controlled gates are not supported in stabilizer simulation",
                        mc.num_ctrl_qubits()
                    )));
                }
                Instruction::Directive(directive) => match directive {
                    Directive::Barrier => {} // no-op
                    Directive::Measure => {
                        return Err(QisError::UnsupportedOperation(
                            "legacy Measure directive is not supported by stabilizer circuit execution; use Circuit::measure".to_string(),
                        ));
                    }
                    Directive::Reset => {
                        let q = qubit_indices[0];
                        state.reset(q)?;
                    }
                },
                Instruction::Delay => {} // no-op
                Instruction::UnitaryGate(_) => {
                    return Err(QisError::UnsupportedOperation(
                        "Arbitrary unitary gates are not Clifford".to_string(),
                    ));
                }
                Instruction::CircuitGate(_) => {
                    return Err(QisError::CircuitError(CircuitError::InvalidOperation(
                        "CircuitGate should have been decomposed".to_string(),
                    )));
                }
                Instruction::ClassicalControl(_) => {
                    return Err(QisError::UnsupportedOperation(
                        "classical control flow is not supported in stabilizer simulation"
                            .to_string(),
                    ));
                }
                Instruction::ClassicalData(data) => {
                    if classical_mode == CircuitClassicalMode::Ignore {
                        continue;
                    }
                    match data {
                        ClassicalDataOp::MeasureBit { result } => {
                            let bit = state.measure(qubit_indices[0])?;
                            classical.set_value(*result, RuntimeValue::Bit(bit))?;
                        }
                        ClassicalDataOp::MeasureBits { result } => {
                            let bits = qubit_indices
                                .iter()
                                .copied()
                                .map(|q| state.measure(q))
                                .collect::<Result<Vec<_>, _>>()?;
                            classical.set_value(*result, RuntimeValue::bit_vec_from_lsb(&bits))?;
                        }
                        ClassicalDataOp::Store { target, value } => {
                            classical.store(*target, value)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Applies a standard gate to the stabilizer tableau.
    ///
    /// Returns [`QisError::NonCliffordGate`] for gates outside the Clifford set.
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
        self.apply_clifford_gate(gate, qubits, params)
    }

    /// Dispatches a [`StandardGate`] to the appropriate Clifford tableau method.
    /// Returns `Err` for any non-Clifford gate.
    fn apply_clifford_gate(
        &mut self,
        gate: StandardGate,
        qubits: &[usize],
        _params: &[f64],
    ) -> Result<(), QisError> {
        match gate {
            StandardGate::I => {}
            StandardGate::H => self.apply_h(qubits[0])?,
            StandardGate::X => self.apply_x(qubits[0])?,
            StandardGate::Y => self.apply_y(qubits[0])?,
            StandardGate::Z => self.apply_z(qubits[0])?,
            StandardGate::S => self.apply_s(qubits[0])?,
            StandardGate::SDG => self.apply_sdg(qubits[0])?,
            StandardGate::X2P => self.apply_x2p(qubits[0])?,
            StandardGate::X2M => self.apply_x2m(qubits[0])?,
            StandardGate::Y2P => self.apply_y2p(qubits[0])?,
            StandardGate::Y2M => self.apply_y2m(qubits[0])?,
            StandardGate::CX => self.apply_cx(qubits[0], qubits[1])?,
            StandardGate::CY => self.apply_cy(qubits[0], qubits[1])?,
            StandardGate::CZ => self.apply_cz(qubits[0], qubits[1])?,
            StandardGate::SWAP => self.apply_swap(qubits[0], qubits[1])?,
            g => {
                return Err(QisError::NonCliffordGate(format!(
                    "Gate '{g}' is not a Clifford gate — use Statevector for universal simulation"
                )));
            }
        }
        Ok(())
    }

    /// There are `n` stabilizer generators (rows `n..2n`).
    pub fn get_stabilizers(&self) -> Vec<PauliString> {
        (0..self.num_qubits)
            .map(|i| self.row_to_pauli_string(self.num_qubits + i))
            .collect()
    }

    /// Returns the destabilizer generators as [`PauliString`]s.
    pub fn get_destabilizers(&self) -> Vec<PauliString> {
        (0..self.num_qubits)
            .map(|i| self.row_to_pauli_string(i))
            .collect()
    }

    /// Converts tableau row `row` to a [`PauliString`].
    fn row_to_pauli_string(&self, row: usize) -> PauliString {
        let mut ps = PauliString::new(self.num_qubits);
        ps.phase = Phase::from(self.phase(row));
        for q in 0..self.num_qubits {
            let x = self.x_bit(row, q);
            let z = self.z_bit(row, q);
            let pauli = match (x, z) {
                (false, false) => Pauli::I,
                (true, false) => Pauli::X,
                (true, true) => Pauli::Y,
                (false, true) => Pauli::Z,
            };
            ps.set_pauli(q, pauli);
        }
        ps
    }

    /// Returns the expectation value ⟨ψ|P|ψ⟩ for a Pauli string `pauli`.
    ///
    /// Uses GF(2) Gaussian elimination over the symplectic representation of the
    /// stabilizer generators to determine whether `pauli` is in the stabilizer
    /// group (expectation +1), the negative stabilizer group (expectation -1),
    /// or neither (expectation 0).
    ///
    /// This correctly handles products of generators, not just individual generators.
    pub fn pauli_expectation(&self, pauli: &PauliString) -> Result<i32, QisError> {
        if pauli.num_qubits != self.num_qubits {
            return Err(QisError::QubitMismatch {
                expected: self.num_qubits,
                actual: pauli.num_qubits,
            });
        }

        let n = self.num_qubits;
        let n_words = n.div_ceil(64);
        let rl = self.row_len;

        // combo_words: one bit per generator (row), tracking which generators are
        // XOR'd together to match the query. n generators → n bits → n_words words.
        let combo_words = n_words;

        // Build working matrix: each entry is [x-block (n_words) | z-block (n_words) | combo (combo_words)].
        // Row i corresponds to stabilizer generator i (tableau row n+i).
        let row_stride = 2 * n_words + combo_words;
        let mut mat: Vec<u64> = vec![0u64; n * row_stride];
        let mut mat_phase: Vec<i32> = vec![0i32; n];

        for i in 0..n {
            let src = n + i; // stabilizer row index
            let src_base = Self::row_base(src, rl);
            let dst = &mut mat[i * row_stride..(i + 1) * row_stride];
            // Copy x-block
            dst[..n_words].copy_from_slice(&self.tableau[src_base..src_base + n_words]);
            // Copy z-block
            dst[n_words..2 * n_words]
                .copy_from_slice(&self.tableau[src_base + rl..src_base + rl + n_words]);
            // Set combo bit i → 1
            dst[2 * n_words + i / 64] |= 1u64 << (i % 64);
            // Phase as i32 in {0,1,2,3}
            mat_phase[i] = self.phases[src] as i32;
        }

        // Build the query symplectic vector from the PauliString.
        let mut qx = vec![0u64; n_words];
        let mut qz = vec![0u64; n_words];
        for (q, p) in pauli.iter().enumerate() {
            match p {
                Pauli::X | Pauli::Y => qx[q / 64] |= 1u64 << (q % 64),
                _ => {}
            }
            match p {
                Pauli::Z | Pauli::Y => qz[q / 64] |= 1u64 << (q % 64),
                _ => {}
            }
        }
        let mut q_combo = vec![0u64; combo_words];
        let mut q_phase: i32 = pauli.phase as i32; // 0=+1,1=+i,2=-1,3=-i

        // Reduced row echelon form (RREF) over GF(2), iterating over 2n columns:
        // columns 0..n → x-block (qubit q = col, word offset 0)
        // columns n..2n → z-block (qubit q = col-n, word offset n_words)
        let mut pivot_row = 0usize;
        for col in 0..(2 * n) {
            let (word_off, bit_idx) = if col < n {
                (0usize, col % 64)
            } else {
                (n_words, (col - n) % 64)
            };
            let word_col = if col < n { col / 64 } else { (col - n) / 64 };
            let word_abs = word_off + word_col;
            let mask = 1u64 << bit_idx;

            // Find a pivot in rows pivot_row..n
            let found = (pivot_row..n).find(|&r| (mat[r * row_stride + word_abs] & mask) != 0);
            let p = match found {
                Some(p) => p,
                None => continue, // no pivot in this column → skip
            };

            // Swap pivot row to position pivot_row
            if p != pivot_row {
                let (lo, hi) = if p < pivot_row {
                    (p, pivot_row)
                } else {
                    (pivot_row, p)
                };
                let (left, right) = mat.split_at_mut(hi * row_stride);
                left[lo * row_stride..(lo + 1) * row_stride]
                    .swap_with_slice(&mut right[..row_stride]);
                mat_phase.swap(p, pivot_row);
            }

            // Eliminate all other rows (full RREF, not just lower triangular)
            for r in 0..n {
                if r == pivot_row {
                    continue;
                }
                if (mat[r * row_stride + word_abs] & mask) == 0 {
                    continue;
                }
                // XOR row pivot_row into row r
                // phase update: same formula as rowsum but non-mutating for pivot_row
                let mut sum = mat_phase[r] + mat_phase[pivot_row];
                for w in 0..n_words {
                    let xh = mat[r * row_stride + w];
                    let zh = mat[r * row_stride + n_words + w];
                    let xi = mat[pivot_row * row_stride + w];
                    let zi = mat[pivot_row * row_stride + n_words + w];
                    sum += Self::g_phase_word(xh, zh, xi, zi);
                }
                mat_phase[r] = ((sum % 4) + 4) % 4;
                for w in 0..row_stride {
                    mat[r * row_stride + w] ^= mat[pivot_row * row_stride + w];
                }
            }

            // Eliminate the query vector at this column
            let q_word = if col < n { &qx } else { &qz };
            if (q_word[word_col] & mask) != 0 {
                // XOR pivot_row into query
                let mut sum = q_phase + mat_phase[pivot_row];
                for w in 0..n_words {
                    let xh = qx[w];
                    let zh = qz[w];
                    let xi = mat[pivot_row * row_stride + w];
                    let zi = mat[pivot_row * row_stride + n_words + w];
                    sum += Self::g_phase_word(xh, zh, xi, zi);
                }
                q_phase = ((sum % 4) + 4) % 4;
                for w in 0..n_words {
                    qx[w] ^= mat[pivot_row * row_stride + w];
                    qz[w] ^= mat[pivot_row * row_stride + n_words + w];
                }
                for w in 0..combo_words {
                    q_combo[w] ^= mat[pivot_row * row_stride + 2 * n_words + w];
                }
            }

            pivot_row += 1;
        }

        // If any x or z bit remains in the query, the Pauli is not in the stabilizer group.
        let in_group = qx.iter().chain(qz.iter()).all(|&w| w == 0);
        if !in_group {
            return Ok(0);
        }

        // q_phase tracks the phase of the running product:
        //   Q_orig · G_combo  (where G_combo is the generators eliminated so far)
        // After elimination q_phase = phase(Q_orig · G_combo) = phase(λI), so
        // Q_orig = λ · G_combo^{-1} and ⟨Q_orig⟩ = i^{q_phase}:
        //   q_phase = 0 (+1) → Q_orig is the stabilizer  → expectation +1
        //   q_phase = 2 (-1) → Q_orig is −(stabilizer)   → expectation −1
        //   q_phase = 1 or 3 → imaginary, not a Hermitian stabilizer element
        Ok(match q_phase {
            0 => 1,
            2 => -1,
            _ => 0,
        })
    }

    /// Exports the stabilizer tableau in Stim-compatible text format.
    pub fn to_stim_format(&self) -> String {
        let mut out = String::new();
        for i in 0..self.num_qubits {
            let stab = self.row_to_pauli_string(self.num_qubits + i);
            out.push_str(&stab.to_string());
            out.push('\n');
        }
        out
    }
}

impl Clone for StabilizerState {
    /// O(n²) deep copy — clones the flat tableau, phases, and RNG state.
    ///
    /// The RNG is cloned (not re-seeded) so that parallel samples produced from
    /// independent clones diverge deterministically from the same seed, which is
    /// required for reproducible multi-shot sampling with Rayon.
    fn clone(&self) -> Self {
        StabilizerState {
            num_qubits: self.num_qubits,
            row_len: self.row_len,
            tableau: self.tableau.clone(),
            phases: self.phases.clone(),
            rng: self.rng.clone(),
        }
    }
}

#[cfg(test)]
#[path = "./stabilizer_test.rs"]
mod stabilizer_test;
