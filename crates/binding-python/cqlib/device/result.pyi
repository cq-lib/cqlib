# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
#
# Any modifications or derivative works of this code must retain this
# copyright notice, and modified files need to carry a notice indicating
# that they have been altered from the originals.

from cqlib.circuit import Qubit

class Outcome:
    """
    Measurement outcome as a compact bitstring.

    Represents a quantum measurement result as a bit vector. The outcome
    is stored efficiently using 64-bit chunks and supports arbitrary
    numbers of qubits.

    # Bit Ordering

    Uses little-endian bit ordering: the rightmost bit in the string
    corresponds to qubit 0, and the leftmost to qubit N-1.

    # Example

    ```python
    from cqlib.device import Outcome

    # Create from bitstring
    outcome = Outcome("101")  # Qubit 0 = 1, Qubit 1 = 0, Qubit 2 = 1

    # Check individual bits
    assert outcome.is_one(0)  # True
    assert not outcome.is_one(1)  # False

    # Convert back to string
    bitstring = outcome.to_bitstring(3)  # "101"
    ```
    """

    def __init__(self, bitstring: str) -> None:
        """
        Creates an outcome from a bitstring.

        Args:
            bitstring: Binary string of '0's and '1's

        Raises:
            ValueError: If the string contains characters other than '0' or '1'.
        """
        ...

    @staticmethod
    def from_bitstring(bitstring: str) -> "Outcome":
        """Alternative constructor from bitstring (same as `Outcome()`)."""
        ...

    @staticmethod
    def from_indices(width: int, indices: list[int]) -> "Outcome":
        """Create an outcome with the given bit indices set to one."""
        ...

    def is_one(self, index: int) -> bool:
        """
        Returns True if the bit at the given index is 1.

        Args:
            index: Bit index (0 = least significant = rightmost in string)
        """
        ...

    def to_bitstring(self, num_qubits: int) -> str:
        """
        Formats the outcome as a binary string.

        Args:
            num_qubits: Width of the output bitstring in bits (pads or truncates to this length)

        Returns:
            Binary string of length `num_qubits`
        """
        ...

    @property
    def chunks(self) -> list[int]:
        """
        Returns the raw storage chunks.
        For advanced use only. Returns the internal 64-bit chunks storing the bit values.
        """
        ...

    def __hash__(self) -> int: ...
    def __eq__(self, value: object) -> bool: ...
    def __copy__(self) -> "Outcome": ...
    def __deepcopy__(self, memo: dict) -> "Outcome": ...

class Status:
    """
    Execution status of a quantum job.

    Represents the state of a quantum computation job through its lifecycle
    from submission to completion or failure.

    # States

    - **Queued**: Job is waiting in the queue
    - **Running**: Job is currently executing on the backend
    - **Completed**: Job finished successfully
    - **Failed**: Job encountered an error
    - **Cancelled**: Job was cancelled by the user
    """

    @staticmethod
    def queued() -> "Status":
        """Creates a "queued" status."""
        ...

    @staticmethod
    def running() -> "Status":
        """Creates a "running" status."""
        ...

    @staticmethod
    def completed() -> "Status":
        """Creates a "completed" status."""
        ...

    @staticmethod
    def failed(error_msg: str, error_code: int) -> "Status":
        """
        Creates a "failed" status.

        Args:
            error_msg: Human-readable error description
            error_code: Numeric error code
        """
        ...

    @staticmethod
    def cancelled() -> "Status":
        """Creates a "cancelled" status."""
        ...

    @property
    def kind(self) -> str:
        """
        Returns the status kind as a string.
        One of: "queued", "running", "completed", "failed", "cancelled"
        """
        ...

    @property
    def error_msg(self) -> str | None:
        """Returns the error message if status is "failed", None otherwise."""
        ...

    @property
    def error_code(self) -> int | None:
        """Returns the error code if status is "failed", None otherwise."""
        ...

    def is_terminal(self) -> bool:
        """
        Returns True if the job has reached a terminal state.
        Terminal states are: completed, failed, cancelled.
        """
        ...

    def is_success(self) -> bool:
        """Returns True if the job completed successfully."""
        ...

    def __copy__(self) -> "Status": ...
    def __deepcopy__(self, memo: dict) -> "Status": ...

class ExecutionResult:
    """
    Complete execution results for a quantum job.

    Contains all information about a quantum computation: measurement counts,
    timestamps, backend information, and calculated probabilities.

    # Example

    ```python
    from cqlib.device import ExecutionResult

    # Create result object
    result = ExecutionResult(
        task_id="task-001",
        qubits=[0, 1],
        shots=1000,
        num_qubits=2,
        backend="ibmq_manila"
    )

    # Lifecycle
    result.start()  # Mark as running
    result.finish({"00": 512, "11": 488})  # Set counts
    result.calc_probabilities()  # Calculate probabilities

    # Access results
    print(result.counts)  # {"00": 512, "11": 488}
    print(result.probabilities)  # {"00": 0.512, "11": 0.488}
    ```
    """

    def __init__(
        self,
        task_id: str,
        qubits: list[int] | list[Qubit],
        shots: int,
        num_qubits: int,
        backend: str | None = None,
    ) -> None:
        """
        Creates a new execution result in "queued" status.

        Args:
            task_id: Unique job identifier
            qubits: List of measured qubits (either all ints or all Qubits)
            shots: Number of measurement shots
            num_qubits: Number of measured qubits (width of the measurement bitstrings)
            backend: Optional backend name
        """
        ...

    @staticmethod
    def from_counts(
        task_id: str,
        qubits: list[int] | list[Qubit],
        shots: int,
        num_qubits: int,
        counts: dict[str, int],
        backend: str | None = None,
    ) -> "ExecutionResult":
        """
        Creates a completed execution result from measurement counts.

        Args:
            task_id: Unique job identifier
            qubits: List of measured qubits.
            shots: Number of measurement shots
            num_qubits: Number of measured qubits (width of the measurement bitstrings)
            counts: Dictionary mapping bitstrings to occurrence counts
            backend: Optional backend name

        Raises:
            ValueError: If any bitstring contains invalid characters or its
                length is not equal to num_qubits.
        """
        ...

    def start(self) -> None:
        """
        Marks the job as running.
        Sets status to "running" and records the start timestamp.
        """
        ...

    def finish(self, counts: dict[str, int]) -> None:
        """
        Marks the job as completed with measurement counts.

        Args:
            counts: Dictionary mapping bitstrings to occurrence counts

        Raises:
            ValueError: If any bitstring contains invalid characters or its
                length is not equal to num_qubits.
        """
        ...

    def fail(self, msg: str, code: int) -> None:
        """
        Marks the job as failed.

        Args:
            msg: Error message
            code: Error code
        """
        ...

    def cancel(self) -> None:
        """Marks the job as cancelled."""
        ...

    def calc_probabilities(self) -> None:
        """
        Calculates probabilities from measurement counts.
        Populates the `probabilities` property with normalized frequencies.
        """
        ...

    @property
    def task_id(self) -> str:
        """Returns the task ID."""
        ...

    @property
    def shots(self) -> int:
        """Returns the number of shots."""
        ...

    @property
    def num_qubits(self) -> int:
        """Returns the number of qubits."""
        ...

    @property
    def qubits(self) -> list[Qubit]:
        """Returns the list of measured qubits."""
        ...

    @property
    def status(self) -> Status:
        """Returns the current execution status."""
        ...

    @property
    def created_at(self) -> str:
        """Returns the creation timestamp as an ISO 8601 string."""
        ...

    @property
    def started_at(self) -> str | None:
        """Returns the start timestamp, if the job has started."""
        ...

    @property
    def finished_at(self) -> str | None:
        """Returns the finish timestamp, if the job has finished."""
        ...

    @property
    def backend(self) -> str | None:
        """Returns the backend name, if set."""
        ...

    @property
    def counts(self) -> dict[str, int]:
        """
        Returns the measurement counts as a dictionary.
        Maps bitstrings (e.g., "00101") to occurrence counts.
        """
        ...

    @property
    def probabilities(self) -> dict[str, float] | None:
        """
        Returns the calculated probabilities, if available.
        Maps bitstrings to probabilities (0.0 to 1.0).
        Requires `calc_probabilities()` to be called first.
        """
        ...

    def __copy__(self) -> "ExecutionResult": ...
    def __deepcopy__(self, memo: dict) -> "ExecutionResult": ...
