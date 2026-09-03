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

//! Python bindings for quantum execution results.
//!
//! This module provides types for representing quantum measurement outcomes,
//! job execution status, and complete execution results with histogram data.
//!
//! # Types
//!
//! - [`PyOutcome`]: Compact bitstring representation of measurement outcomes
//! - [`PyStatus`]: Job execution state machine (queued, running, completed, etc.)
//! - [`PyExecutionResult`]: Complete results with counts, timestamps, and metadata
//!
//! # Example
//!
//! ```python
//! from cqlib.device import ExecutionResult, Status, Outcome
//!
//! # Create a new execution
//! result = ExecutionResult(
//!     task_id="task-001",
//!     qubits=[0, 1],
//!     shots=1000,
//!     num_qubits=2,
//!     backend="simulator"
//! )
//!
//! # Mark as running
//! result.start()
//!
//! # Finish with measurement counts
//! result.finish({"00": 520, "11": 480})
//!
//! # Calculate probabilities
//! result.calc_probabilities()
//! print(result.probabilities)  # {"00": 0.52, "11": 0.48}
//! ```

use crate::circuit::PyQubit;
use crate::circuit::bit::PyIntListOrQubitList;
use crate::utils::hash_value;
use cqlib_core::device::{ExecutionResult, Outcome, Status};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;

/// Measurement outcome as a compact bitstring.
///
/// Represents a quantum measurement result as a bit vector. The outcome
/// is stored efficiently using 64-bit chunks and supports arbitrary
/// numbers of qubits.
///
/// # Bit Ordering
///
/// Uses little-endian bit ordering: the rightmost bit in the string
/// corresponds to qubit 0, and the leftmost to qubit N-1.
///
/// # Example
///
/// ```python
/// from cqlib.device import Outcome
///
/// # Create from bitstring
/// outcome = Outcome("101")  # Qubit 0 = 1, Qubit 1 = 0, Qubit 2 = 1
///
/// # Check individual bits
/// assert outcome.is_one(0)  # True
/// assert not outcome.is_one(1)  # False
///
/// # Convert back to string
/// bitstring = outcome.to_bitstring(3)  # "101"
/// ```
#[pyclass(name = "Outcome", module = "cqlib.device", from_py_object)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyOutcome {
    pub(crate) inner: Outcome,
}

impl From<Outcome> for PyOutcome {
    fn from(inner: Outcome) -> Self {
        Self { inner }
    }
}

impl From<PyOutcome> for Outcome {
    fn from(value: PyOutcome) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyOutcome {
    /// Creates an outcome from a bitstring.
    ///
    /// # Arguments
    ///
    /// * `bitstring` - Binary string of '0's and '1's
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if the string contains characters other than '0' or '1'.
    ///
    /// # Bit Ordering
    ///
    /// The rightmost character corresponds to qubit 0 (least significant bit).
    #[new]
    fn new(bitstring: String) -> PyResult<Self> {
        Outcome::from_bitstring(&bitstring)
            .map(Self::from)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Alternative constructor from bitstring (same as `Outcome()`).
    #[staticmethod]
    fn from_bitstring(bitstring: String) -> PyResult<Self> {
        Self::new(bitstring)
    }

    /// Creates an outcome with the given bit indices set to one.
    #[staticmethod]
    fn from_indices(width: usize, indices: Vec<usize>) -> Self {
        Self::from(Outcome::from_indices(width, indices))
    }

    /// Returns True if the bit at the given index is 1.
    ///
    /// # Arguments
    ///
    /// * `index` - Bit index (0 = least significant = rightmost in string)
    fn is_one(&self, index: usize) -> bool {
        self.inner.is_one(index)
    }

    /// Formats the outcome as a binary string.
    ///
    /// # Arguments
    ///
    /// * `num_qubits` - Total number of qubits (pads with leading zeros if needed)
    ///
    /// # Returns
    ///
    /// Binary string of length `num_qubits`
    fn to_bitstring(&self, num_qubits: usize) -> String {
        self.inner.to_bitstring(num_qubits)
    }

    /// Returns the raw storage chunks.
    ///
    /// For advanced use only. Returns the internal 64-bit chunks storing
    /// the bit values.
    #[getter]
    fn chunks(&self) -> Vec<u64> {
        self.inner.0.to_vec()
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyOutcome>() {
            return Ok(false);
        }
        let other = other.extract::<PyOutcome>()?;
        Ok(self.inner == other.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __repr__(&self) -> String {
        format!("Outcome(chunks={:?})", self.chunks())
    }
}

/// Execution status of a quantum job.
///
/// Represents the state of a quantum computation job through its lifecycle
/// from submission to completion or failure.
///
/// # States
///
/// - **Queued**: Job is waiting in the queue
/// - **Running**: Job is currently executing on the backend
/// - **Completed**: Job finished successfully
/// - **Failed**: Job encountered an error
/// - **Cancelled**: Job was cancelled by the user
///
/// # Example
///
/// ```python
/// from cqlib.device import Status
///
/// # Create different statuses
/// status = Status.queued()
/// status = Status.running()
/// status = Status.completed()
/// status = Status.failed("Timeout", 500)
/// status = Status.cancelled()
///
/// # Check state
/// if status.is_terminal():
///     print(f"Job finished with status: {status}")
/// ```
#[pyclass(name = "Status", module = "cqlib.device", skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyStatus {
    pub(crate) inner: Status,
}

impl From<Status> for PyStatus {
    fn from(inner: Status) -> Self {
        Self { inner }
    }
}

impl From<PyStatus> for Status {
    fn from(value: PyStatus) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyStatus {
    /// Creates a "queued" status.
    #[staticmethod]
    fn queued() -> Self {
        Self {
            inner: Status::Queued,
        }
    }

    /// Creates a "running" status.
    #[staticmethod]
    fn running() -> Self {
        Self {
            inner: Status::Running,
        }
    }

    /// Creates a "completed" status.
    #[staticmethod]
    fn completed() -> Self {
        Self {
            inner: Status::Completed,
        }
    }

    /// Creates a "failed" status.
    ///
    /// # Arguments
    ///
    /// * `error_msg` - Human-readable error description
    /// * `error_code` - Numeric error code
    #[staticmethod]
    fn failed(error_msg: String, error_code: i32) -> Self {
        Self {
            inner: Status::Failed {
                error_msg,
                error_code,
            },
        }
    }

    /// Creates a "cancelled" status.
    #[staticmethod]
    fn cancelled() -> Self {
        Self {
            inner: Status::Cancelled,
        }
    }

    /// Returns the status kind as a string.
    ///
    /// One of: "queued", "running", "completed", "failed", "cancelled"
    #[getter]
    fn kind(&self) -> &'static str {
        status_kind(&self.inner)
    }

    /// Returns the error message if status is "failed", None otherwise.
    #[getter]
    fn error_msg(&self) -> Option<String> {
        match &self.inner {
            Status::Failed { error_msg, .. } => Some(error_msg.clone()),
            _ => None,
        }
    }

    /// Returns the error code if status is "failed", None otherwise.
    #[getter]
    fn error_code(&self) -> Option<i32> {
        match self.inner {
            Status::Failed { error_code, .. } => Some(error_code),
            _ => None,
        }
    }

    /// Returns True if the job has reached a terminal state.
    ///
    /// Terminal states are: completed, failed, cancelled.
    fn is_terminal(&self) -> bool {
        self.inner.is_terminal()
    }

    /// Returns True if the job completed successfully.
    fn is_success(&self) -> bool {
        self.inner.is_success()
    }

    /// Compares two statuses by value: same state and, for failures, the
    /// same error message and code.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyStatus>() {
            return Ok(false);
        }
        let other = other.extract::<PyRef<'_, PyStatus>>()?;
        Ok(self.inner == other.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __repr__(&self) -> String {
        format!("Status({})", self.kind())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// Complete execution results for a quantum job.
///
/// Contains all information about a quantum computation: measurement counts,
/// timestamps, backend information, and calculated probabilities.
///
/// # Example
///
/// ```python
/// from cqlib.device import ExecutionResult
///
/// # Create result object
/// result = ExecutionResult(
///     task_id="task-001",
///     qubits=[0, 1],
///     shots=1000,
///     num_qubits=2,
///     backend="ibmq_manila"
/// )
///
/// # Lifecycle
/// result.start()  # Mark as running
/// result.finish({"00": 512, "11": 488})  # Set counts
/// result.calc_probabilities()  # Calculate probabilities
///
/// # Access results
/// print(result.counts)  # {"00": 512, "11": 488}
/// print(result.probabilities)  # {"00": 0.512, "11": 0.488}
/// ```
#[pyclass(name = "ExecutionResult", module = "cqlib.device", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyExecutionResult {
    pub(crate) inner: ExecutionResult,
}

impl From<ExecutionResult> for PyExecutionResult {
    fn from(inner: ExecutionResult) -> Self {
        Self { inner }
    }
}

impl From<PyExecutionResult> for ExecutionResult {
    fn from(value: PyExecutionResult) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyExecutionResult {
    /// Creates a new execution result in "queued" status.
    ///
    /// # Arguments
    ///
    /// * `task_id` - Unique job identifier
    /// * `qubits` - List of measured qubits
    /// * `shots` - Number of measurement shots
    /// * `num_qubits` - Number of measured qubits (width of the measurement bitstrings)
    /// * `backend` - Optional backend name
    #[new]
    #[pyo3(signature = (task_id, qubits, shots, num_qubits, backend=None))]
    fn new(
        task_id: String,
        qubits: PyIntListOrQubitList,
        shots: usize,
        num_qubits: usize,
        backend: Option<String>,
    ) -> PyResult<Self> {
        let qubits = qubits.into();
        Ok(Self {
            inner: ExecutionResult::new(task_id, qubits, shots, num_qubits, backend, None),
        })
    }

    /// Creates a completed execution result from measurement counts.
    #[staticmethod]
    #[pyo3(signature = (task_id, qubits, shots, num_qubits, counts, backend=None))]
    fn from_counts(
        task_id: String,
        qubits: PyIntListOrQubitList,
        shots: usize,
        num_qubits: usize,
        counts: HashMap<String, usize>,
        backend: Option<String>,
    ) -> PyResult<Self> {
        let qubits = qubits.into();
        let counts = parse_counts(num_qubits, counts)?;
        Ok(Self {
            inner: ExecutionResult::from_counts(
                task_id, qubits, shots, num_qubits, backend, counts,
            ),
        })
    }

    /// Marks the job as running.
    ///
    /// Sets status to "running" and records the start timestamp.
    fn start(&mut self) {
        self.inner.start(None);
    }

    /// Marks the job as completed with measurement counts.
    ///
    /// # Arguments
    ///
    /// * `counts` - Dictionary mapping bitstrings to occurrence counts
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if any bitstring contains invalid characters or
    /// has a length that does not match `num_qubits`.
    fn finish(&mut self, counts: HashMap<String, usize>) -> PyResult<()> {
        let counts = parse_counts(self.num_qubits(), counts)?;
        self.inner.finish(counts, None);
        Ok(())
    }

    /// Marks the job as failed.
    ///
    /// # Arguments
    ///
    /// * `msg` - Error message
    /// * `code` - Error code
    fn fail(&mut self, msg: String, code: i32) {
        self.inner.fail(msg, code);
    }

    /// Marks the job as cancelled.
    fn cancel(&mut self) {
        self.inner.cancel();
    }

    /// Calculates probabilities from measurement counts.
    ///
    /// Populates the `probabilities` property with normalized frequencies.
    fn calc_probabilities(&mut self) {
        self.inner.calc_probabilities();
    }

    /// Returns the task ID.
    #[getter]
    fn task_id(&self) -> String {
        self.inner.task_id().to_string()
    }

    /// Returns the number of shots.
    #[getter]
    fn shots(&self) -> usize {
        self.inner.shots()
    }

    /// Returns the number of qubits.
    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    /// Returns the list of measured qubits.
    #[getter]
    fn qubits(&self) -> Vec<PyQubit> {
        self.inner
            .qubits()
            .iter()
            .copied()
            .map(PyQubit::from)
            .collect()
    }

    /// Returns the current execution status.
    #[getter]
    fn status(&self) -> PyStatus {
        PyStatus {
            inner: self.inner.status().clone(),
        }
    }

    /// Returns the creation timestamp as an ISO 8601 string.
    #[getter]
    fn created_at(&self) -> String {
        self.inner.created_at().to_string()
    }

    /// Returns the start timestamp, if the job has started.
    #[getter]
    fn started_at(&self) -> Option<String> {
        self.inner.started_at().as_ref().map(ToString::to_string)
    }

    /// Returns the finish timestamp, if the job has finished.
    #[getter]
    fn finished_at(&self) -> Option<String> {
        self.inner.finished_at().as_ref().map(ToString::to_string)
    }

    /// Returns the backend name, if set.
    #[getter]
    fn backend(&self) -> Option<String> {
        self.inner.backend().cloned()
    }

    /// Returns the measurement counts as a dictionary.
    ///
    /// Maps bitstrings (e.g., "00101") to occurrence counts.
    #[getter]
    fn counts(&self) -> HashMap<String, usize> {
        counts_to_bitstring_map(self.inner.counts(), self.num_qubits())
    }

    /// Returns the calculated probabilities, if available.
    ///
    /// Maps bitstrings to probabilities (0.0 to 1.0).
    /// Requires `calc_probabilities()` to be called first.
    #[getter]
    fn probabilities(&self) -> Option<HashMap<String, f64>> {
        self.inner
            .probabilities()
            .as_ref()
            .map(|probs| probabilities_to_bitstring_map(probs, self.num_qubits()))
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ExecutionResult(task_id='{}', status='{}', shots={}, num_qubits={})",
            self.task_id(),
            self.status().kind(),
            self.shots(),
            self.num_qubits()
        )
    }
}

/// Converts Status to its string representation.
fn status_kind(status: &Status) -> &'static str {
    match status {
        Status::Queued => "queued",
        Status::Running => "running",
        Status::Completed => "completed",
        Status::Failed { .. } => "failed",
        Status::Cancelled => "cancelled",
    }
}

/// Parses string-keyed measurement counts into `Outcome`-keyed counts.
///
/// Validates that every key has exactly `num_qubits` bits before parsing. This
/// is required because [`Outcome::from_bitstring`] does not preserve the input
/// width: without this check, distinct keys such as `"1"` and `"01"` would
/// collapse to the same `Outcome` (silently dropping data), and a key wider
/// than `num_qubits` would be truncated when formatted back to a bitstring.
fn parse_counts(
    num_qubits: usize,
    counts: HashMap<String, usize>,
) -> PyResult<HashMap<Outcome, usize>> {
    let mut parsed = HashMap::with_capacity(counts.len());
    for (bitstring, count) in counts {
        if bitstring.len() != num_qubits {
            return Err(PyValueError::new_err(format!(
                "bitstring '{bitstring}' has length {} but num_qubits={num_qubits}; \
                 every counts key must have exactly num_qubits bits",
                bitstring.len()
            )));
        }
        let outcome = Outcome::from_bitstring(&bitstring)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        parsed.insert(outcome, count);
    }
    Ok(parsed)
}

/// Converts internal counts to string-keyed dictionary.
fn counts_to_bitstring_map(
    counts: &HashMap<Outcome, usize>,
    num_qubits: usize,
) -> HashMap<String, usize> {
    counts
        .iter()
        .map(|(outcome, count)| (outcome.to_bitstring(num_qubits), *count))
        .collect()
}

/// Converts internal probabilities to string-keyed dictionary.
fn probabilities_to_bitstring_map(
    probabilities: &HashMap<Outcome, f64>,
    num_qubits: usize,
) -> HashMap<String, f64> {
    probabilities
        .iter()
        .map(|(outcome, prob)| (outcome.to_bitstring(num_qubits), *prob))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{PyExecutionResult, PyOutcome};
    use crate::circuit::bit::PyIntListOrQubitList;
    use std::collections::HashMap;

    #[test]
    fn outcome_from_indices_sets_little_endian_bits() {
        let outcome = PyOutcome::from_indices(3, vec![0, 2, 9]);

        assert_eq!(outcome.to_bitstring(3), "101");
    }

    #[test]
    fn execution_result_from_counts_returns_completed_result() {
        let result = PyExecutionResult::from_counts(
            "task".to_string(),
            PyIntListOrQubitList::IntList(vec![0]),
            2,
            1,
            HashMap::from([("0".to_string(), 1), ("1".to_string(), 1)]),
            Some("sim".to_string()),
        )
        .unwrap();

        assert_eq!(result.status().kind(), "completed");
        assert_eq!(result.backend(), Some("sim".to_string()));
        assert_eq!(result.counts().get("0"), Some(&1));
        assert_eq!(result.probabilities().unwrap().get("1"), Some(&0.5));
    }
}
