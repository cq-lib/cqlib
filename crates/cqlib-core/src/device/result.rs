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

//! Quantum measurement results and execution status.
//!
//! This module provides types for representing quantum measurement outcomes,
//! execution status tracking, and result aggregation with histogram data.
//!
//! # Core Types
//!
//! - [`Outcome`]: Compact bit vector for measurement outcomes
//! - [`Status`]: Job execution state machine
//! - [`ExecutionResult`]: Complete measurement results with metadata
//!
//! # Example
//!
//! ```rust,ignore
//! use cqlib_core::device::{Outcome, ExecutionResult};
//! use std::collections::HashMap;
//!
//! // Create a new execution result
//! let mut result = ExecutionResult::new(
//!     "task-001".to_string(),
//!     vec![],
//!     1000,  // shots
//!     2,     // num_qubits
//!     Some("simulator".to_string()),
//!     None,
//! );
//!
//! // Mark as running and complete with counts
//! result.start(None);
//! let mut counts = HashMap::new();
//! counts.insert(Outcome::from_bitstring("00").unwrap(), 520);
//! counts.insert(Outcome::from_bitstring("11").unwrap(), 480);
//! result.finish(counts, None);
//!
//! // Calculate probabilities
//! result.calc_probabilities();
//! ```

use crate::circuit::Qubit;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Write;
use std::str::FromStr;
use thiserror::Error;
use time::OffsetDateTime;

/// Number of bits per u64 chunk.
const BITS_PER_CHUNK: usize = 64;

/// Error type for outcome parsing failures.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum OutcomeError {
    /// Invalid character in binary string at given index.
    #[error("Invalid character '{1}' in binary string at index {0}")]
    InvalidCharacter(usize, char),
}

/// Measurement outcome as a compact bit vector.
///
/// # Bit Layout & Endianness (Little-Endian)
///
/// This struct uses **Little-Endian** storage:
/// - **Low Index holds Low Bits**: `measured_qubits[0]` is stored in `chunks[0]`, bit 0.
/// - **High Index holds High Bits**: `measured_qubits[64]` is stored in `chunks[1]`, bit 0.
///
/// ## Visual Mapping
///
/// ```text
/// Memory Index:    [     0      ]  [     1      ] ...
/// Data (u64):      [ 63 ... 0   ]  [ 127 ... 64 ]
///                    ^      ^                 ^
///                    |      |                 |
/// Qubit Index:      MSB    LSB               LSB of next chunk
/// ```
///
/// Note: while stored as little-endian chunks, bitstrings are written in standard
/// binary form (MSB left). The original input width is not stored: parsing `"001"`
/// and `"1"` produces the same outcome. Callers must provide the desired output
/// width to [`Outcome::to_bitstring`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Outcome(pub SmallVec<[u64; 4]>);

impl Outcome {
    /// Creates an outcome from raw chunks.
    #[inline(always)]
    pub fn new(chunks: SmallVec<[u64; 4]>) -> Self {
        Self(chunks)
    }

    /// Creates an outcome with the given bit indices set to one.
    ///
    /// `width` is the number of logical bits in the outcome. Indices greater
    /// than or equal to `width` are ignored. Bit index 0 is stored as the least
    /// significant bit, matching [`Outcome`]'s little-endian layout.
    pub fn from_indices(width: usize, indices: impl IntoIterator<Item = usize>) -> Self {
        let mut chunks = SmallVec::from_elem(0u64, width.div_ceil(BITS_PER_CHUNK));
        for index in indices {
            if index < width {
                chunks[index / BITS_PER_CHUNK] |= 1u64 << (index % BITS_PER_CHUNK);
            }
        }
        Self(chunks)
    }

    /// Creates an outcome from bits in little-endian order.
    ///
    /// `bits[0]` becomes bit index 0, the least-significant bit.
    pub(crate) fn from_lsb_bits(bits: &[bool]) -> Self {
        let mut chunks = SmallVec::from_elem(0u64, bits.len().div_ceil(BITS_PER_CHUNK));
        for (index, value) in bits.iter().copied().enumerate() {
            if value {
                chunks[index / BITS_PER_CHUNK] |= 1u64 << (index % BITS_PER_CHUNK);
            }
        }
        Self(chunks)
    }

    /// Create an Outcome from a binary string (e.g., "101").
    ///
    /// # Convention
    /// - The string is interpreted as **Big-Endian visual order**.
    /// - The **last character** (rightmost) corresponds to **Qubit 0** (LSB).
    /// - The **first character** (leftmost) corresponds to **Qubit N-1** (MSB).
    ///
    /// Example: `from_bitstring("100")` -> Qubit 2=1, Qubit 1=0, Qubit 0=0.
    pub fn from_bitstring(s: &str) -> Result<Self, OutcomeError> {
        let len = s.len();
        if len == 0 {
            return Ok(Self(SmallVec::new()));
        }

        // Calculate required chunks (ceil(len / 64))
        let num_chunks = len.div_ceil(BITS_PER_CHUNK);

        // Pre-allocate and initialize to 0
        let mut chunks = smallvec::smallvec![0u64; num_chunks];

        // Iterate from rightmost character (qubit 0)
        // enumerate() gives the qubit index directly (0, 1, 2...)
        for (qubit_index, c) in s.chars().rev().enumerate() {
            match c {
                '1' => {
                    let chunk_idx = qubit_index / BITS_PER_CHUNK;
                    let bit_idx = qubit_index % BITS_PER_CHUNK;

                    // Safety: chunk size is calculated from len, always valid
                    chunks[chunk_idx] |= 1u64 << bit_idx;
                }
                '0' => {
                    // Default is 0, no operation needed
                }
                _ => {
                    // Calculate original string index for accurate error message
                    let original_index = len - 1 - qubit_index;
                    return Err(OutcomeError::InvalidCharacter(original_index, c));
                }
            }
        }

        Ok(Self(chunks))
    }

    /// Returns true if the bit at given index is 1.
    #[inline]
    pub fn is_one(&self, index: usize) -> bool {
        let chunk_idx = index / BITS_PER_CHUNK;
        let bit_idx = index % BITS_PER_CHUNK;

        // Safety: SmallVec indexing is fast, usually bounds check is elided in loops
        // if the compiler can prove bounds, but explicit check is safe.
        if let Some(chunk) = self.0.get(chunk_idx) {
            (chunk >> bit_idx) & 1 == 1
        } else {
            false
        }
    }

    /// Formats the outcome as a binary string with the requested width.
    ///
    /// Output is big-endian (MSB left), padded with leading zeros. If the
    /// requested width is smaller than the stored value, high bits are truncated.
    pub fn to_bitstring(&self, num_qubits: usize) -> String {
        if num_qubits == 0 {
            return String::new();
        }

        let mut s = String::with_capacity(num_qubits);
        let full_chunks = num_qubits / BITS_PER_CHUNK;
        let remainder_bits = num_qubits % BITS_PER_CHUNK;
        // Little-endian storage, but usually printed Big-endian (Q_n...Q_0)
        if remainder_bits > 0 {
            let chunk = self.0.get(full_chunks).copied().unwrap_or(0);
            let mask = if remainder_bits == 64 {
                u64::MAX
            } else {
                (1u64 << remainder_bits) - 1
            };
            let val = chunk & mask;
            write!(&mut s, "{:0width$b}", val, width = remainder_bits).unwrap();
        }

        // Iterate from high index chunk to low index chunk
        for i in (0..full_chunks).rev() {
            let chunk = self.0.get(i).copied().unwrap_or(0);
            // Print full 64 bits
            // e.g., Chunk 0: bit 63 ... bit 0 prints as "0...01"
            write!(&mut s, "{:064b}", chunk).unwrap();
        }
        s
    }
}

impl FromStr for Outcome {
    type Err = OutcomeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_bitstring(s)
    }
}

/// Execution status of a quantum job.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Status {
    /// Task has been submitted and is queued for execution.
    Queued,
    /// Task is currently running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task failed with an error.
    Failed { error_msg: String, error_code: i32 },
    /// Task was cancelled by the user.
    Cancelled,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Queued => write!(f, "Queued"),
            Status::Running => write!(f, "Running"),
            Status::Completed => write!(f, "Completed"),
            Status::Failed {
                error_msg,
                error_code,
            } => {
                write!(f, "Failed (Code {}): {}", error_code, error_msg)
            }
            Status::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl Status {
    /// Returns true if the status is terminal (completed, failed, or cancelled).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Status::Completed | Status::Failed { .. } | Status::Cancelled
        )
    }

    /// Returns true if the job completed successfully.
    pub fn is_success(&self) -> bool {
        matches!(self, Status::Completed)
    }
}

/// Quantum measurement results with histogram data and metadata.
///
/// # Measurement Mapping & Bit Ordering
///
/// There is a strictly **1-to-1 correspondence** between the indices of the
/// [`qubits`](ExecutionResult::qubits) vector and the bits in the [`Outcome`] keys.
///
/// - **Rule**: The qubit at `qubits[i]` corresponds to the **$i$-th bit** (value $2^i$) in the outcome.
/// - **String Representation**: Since binary strings are printed in Big-Endian (MSB left),
///   the string order is the **reverse** of the `qubits` vector order.
///
/// ## Example Mapping
///
/// Suppose we measure two qubits: `qubits = vec![Qubit(2), Qubit(5)]`.
///
/// | Index | Qubit ID | Outcome Bit (Storage) | String Position ("01") |
/// | :---: | :---:    | :---:                 | :---:                  |
/// | **0** | `Qubit(2)` | Bit 0 (LSB, $2^0$)  | **Last** (Rightmost)   |
/// | **1** | `Qubit(5)` | Bit 1 (MSB, $2^1$)  | **First** (Leftmost)   |
///
/// If the result is `Qubit(2) = 1` and `Qubit(5) = 0`:
/// - **Storage**: `...001` (Bit 0 is 1)
/// - **String**: `"01"` (Qubit 5 is '0', Qubit 2 is '1')
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Unique task identifier.
    task_id: String,
    /// Number of shots executed.
    shots: usize,
    /// Number of qubits measured.
    num_qubits: usize,
    /// Measured qubits.
    qubits: Vec<Qubit>,
    /// Current execution status.
    status: Status,
    /// Task creation timestamp.
    created_at: OffsetDateTime,
    /// Execution start timestamp (None if not started).
    started_at: Option<OffsetDateTime>,
    /// Execution finish timestamp (None if not finished).
    finished_at: Option<OffsetDateTime>,
    /// Backend name where executed.
    backend: Option<String>,
    /// Measurement counts per outcome.
    counts: HashMap<Outcome, usize>,
    /// Calculated probabilities (computed on demand).
    probabilities: Option<HashMap<Outcome, f64>>,
}

impl ExecutionResult {
    /// Creates a new execution result in Queued status.
    pub fn new(
        task_id: String,
        qubits: Vec<Qubit>,
        shots: usize,
        num_qubits: usize,
        backend: Option<String>,
        created_at: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            task_id,
            status: Status::Queued,
            shots,
            qubits,
            num_qubits,
            created_at: created_at.unwrap_or(OffsetDateTime::now_utc()),
            started_at: None,
            finished_at: None,
            backend,
            counts: HashMap::new(),
            probabilities: None,
        }
    }

    /// Creates a completed execution result from measurement counts.
    ///
    /// This applies the same lifecycle as `new`, `start`, `finish`, and
    /// `calc_probabilities`, but keeps simulator call sites focused on
    /// producing counts.
    pub fn from_counts(
        task_id: String,
        qubits: Vec<Qubit>,
        shots: usize,
        num_qubits: usize,
        backend: Option<String>,
        counts: HashMap<Outcome, usize>,
    ) -> Self {
        let mut result = Self::new(task_id, qubits, shots, num_qubits, backend, None);
        result.start(None).finish(counts, None).calc_probabilities();
        result
    }

    /// Marks the job as running.
    pub fn start(&mut self, t: Option<OffsetDateTime>) -> &mut Self {
        self.started_at = t.or(Some(OffsetDateTime::now_utc()));
        self.status = Status::Running;
        self
    }

    /// Marks the job as completed with measurement counts.
    pub fn finish(
        &mut self,
        counts: HashMap<Outcome, usize>,
        t: Option<OffsetDateTime>,
    ) -> &mut Self {
        self.counts = counts;
        self.probabilities = None;
        self.status = Status::Completed;
        self.finished_at = t.or(Some(OffsetDateTime::now_utc()));
        self
    }

    /// Marks the job as failed with error code and message.
    pub fn fail(&mut self, msg: String, code: i32) {
        self.status = Status::Failed {
            error_msg: msg,
            error_code: code,
        };
    }

    /// Marks the job as cancelled.
    pub fn cancel(&mut self) {
        self.status = Status::Cancelled;
    }

    /// Calculates probabilities from counts.
    pub fn calc_probabilities(&mut self) -> &mut Self {
        let total_observed: usize = self.counts.values().sum();
        if total_observed == 0 {
            self.probabilities = None;
        } else {
            let mut probs = HashMap::with_capacity(self.counts.len());
            let inv_total = 1.0 / (total_observed as f64);
            for (outcome, &count) in &self.counts {
                probs.insert(outcome.clone(), count as f64 * inv_total);
            }
            self.probabilities = Some(probs);
        }
        self
    }

    /// Returns the task ID.
    pub fn task_id(&self) -> &str {
        self.task_id.as_str()
    }

    /// Returns the number of shots.
    pub fn shots(&self) -> usize {
        self.shots
    }

    /// Returns the number of qubits.
    pub fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    /// Returns the measured qubits.
    pub fn qubits(&self) -> &Vec<Qubit> {
        &self.qubits
    }

    /// Returns the execution status.
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Returns the creation timestamp.
    pub fn created_at(&self) -> &OffsetDateTime {
        &self.created_at
    }

    /// Returns the start timestamp (None if not started).
    pub fn started_at(&self) -> &Option<OffsetDateTime> {
        &self.started_at
    }

    /// Returns the finish timestamp (None if not finished).
    pub fn finished_at(&self) -> &Option<OffsetDateTime> {
        &self.finished_at
    }

    /// Returns the backend name.
    pub fn backend(&self) -> Option<&String> {
        self.backend.as_ref()
    }

    /// Returns the measurement counts.
    pub fn counts(&self) -> &HashMap<Outcome, usize> {
        &self.counts
    }

    /// Returns the calculated probabilities (None if not computed).
    pub fn probabilities(&self) -> &Option<HashMap<Outcome, f64>> {
        &self.probabilities
    }
}

#[cfg(test)]
mod api_tests {
    use super::{ExecutionResult, Outcome, Status};
    use crate::circuit::Qubit;
    use std::collections::HashMap;

    #[test]
    fn outcome_from_indices_uses_little_endian_bit_indices() {
        let outcome = Outcome::from_indices(66, [0, 2, 65, 99]);

        assert!(outcome.is_one(0));
        assert!(!outcome.is_one(1));
        assert!(outcome.is_one(2));
        assert!(outcome.is_one(65));
        assert!(!outcome.is_one(99));
    }

    #[test]
    fn execution_result_from_counts_returns_completed_result_with_probabilities() {
        let zero = Outcome::from_indices(1, []);
        let one = Outcome::from_indices(1, [0]);
        let mut counts = HashMap::new();
        counts.insert(zero.clone(), 3);
        counts.insert(one.clone(), 1);

        let result = ExecutionResult::from_counts(
            "task".to_string(),
            vec![Qubit::new(0)],
            4,
            1,
            Some("backend".to_string()),
            counts,
        );

        assert_eq!(result.status(), &Status::Completed);
        assert_eq!(result.counts().get(&zero), Some(&3));
        assert_eq!(result.counts().get(&one), Some(&1));
        let probabilities = result.probabilities().as_ref().unwrap();
        assert_eq!(probabilities.get(&zero), Some(&0.75));
        assert_eq!(probabilities.get(&one), Some(&0.25));
    }
}

#[cfg(test)]
#[path = "./result_test.rs"]
mod result_test;
