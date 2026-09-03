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

"""Tests for outcome/status/execution-result bindings."""

import pytest

from cqlib import Qubit
from cqlib.device import ExecutionResult, Outcome, Status


class TestOutcome:
    """Tests compact measurement outcome APIs."""

    def test_outcome_helpers(self):
        """Outcome should parse bitstrings and preserve bit access semantics."""
        outcome = Outcome("101")
        assert outcome.is_one(0) is True
        assert outcome.is_one(1) is False
        assert outcome.is_one(2) is True
        assert outcome.to_bitstring(3) == "101"
        assert outcome == Outcome.from_bitstring("101")
        assert hash(outcome) == hash(Outcome("101"))

        with pytest.raises(ValueError):
            Outcome("10a1")


class TestStatus:
    """Tests status constructors and status flags."""

    def test_status_constructors(self):
        """Status constructors should expose kind and terminal/success flags."""
        queued = Status.queued()
        running = Status.running()
        completed = Status.completed()
        failed = Status.failed("boom", 500)
        cancelled = Status.cancelled()

        assert queued.kind == "queued"
        assert queued.is_terminal() is False
        assert running.kind == "running"
        assert completed.kind == "completed"
        assert completed.is_success() is True
        assert failed.kind == "failed"
        assert failed.error_msg == "boom"
        assert failed.error_code == 500
        assert cancelled.kind == "cancelled"
        assert cancelled.is_terminal() is True

    def test_status_value_equality(self):
        """Statuses compare by value; identical states are equal."""
        assert Status.completed() == Status.completed()
        assert Status.failed("x", 1) == Status.failed("x", 1)
        assert Status.failed("x", 1) != Status.failed("y", 1)
        assert Status.failed("x", 1) != Status.failed("x", 2)
        assert Status.completed() != Status.cancelled()

    def test_status_cross_type_comparison_returns_false(self):
        assert (Status.completed() == 42) is False
        assert (Status.completed() == "completed") is False

    def test_status_is_hashable_and_deduplicates(self):
        assert hash(Status.completed()) == hash(Status.completed())
        assert len({Status.completed(), Status.completed(), Status.cancelled()}) == 2


class TestExecutionResult:
    """Tests execution result lifecycle transitions and accessors."""

    def test_execution_result_lifecycle(self):
        """ExecutionResult should follow queued->running->completed flow."""
        result = ExecutionResult(
            task_id="task-1",
            qubits=[Qubit(0), Qubit(1)],
            shots=100,
            num_qubits=2,
            backend="sim",
        )
        assert result.task_id == "task-1"
        assert result.status.kind == "queued"
        assert result.created_at is not None

        result.start()
        assert result.status.kind == "running"
        assert result.started_at is not None

        result.finish({"00": 60, "11": 40})
        assert result.status.kind == "completed"
        assert result.finished_at is not None
        assert result.counts["00"] == 60
        assert result.counts["11"] == 40

        result.calc_probabilities()
        probs = result.probabilities
        assert probs is not None
        assert probs["00"] == pytest.approx(0.6)
        assert probs["11"] == pytest.approx(0.4)

    def test_execution_result_failure_paths(self):
        """ExecutionResult should support fail/cancel transitions and validation."""
        failed = ExecutionResult("task-fail", [Qubit(0)], 10, 1, None)
        failed.fail("backend down", 42)
        assert failed.status.kind == "failed"
        assert failed.status.error_msg == "backend down"
        assert failed.status.error_code == 42

        cancelled = ExecutionResult("task-cancel", [Qubit(0)], 10, 1, None)
        cancelled.cancel()
        assert cancelled.status.kind == "cancelled"

        invalid = ExecutionResult("task-invalid", [Qubit(0)], 10, 1, None)
        with pytest.raises(ValueError):
            invalid.finish({"2": 1})

    def test_from_counts_rejects_key_collision(self):
        """Distinct keys that collapse to one outcome must raise ValueError."""
        with pytest.raises(ValueError):
            ExecutionResult.from_counts("task", [0, 1], 3, 2, {"1": 1, "01": 2})

    def test_from_counts_rejects_width_mismatch(self):
        """A key wider than num_qubits must raise ValueError, not truncate."""
        with pytest.raises(ValueError):
            ExecutionResult.from_counts("task", [0, 1], 1, 2, {"100": 1})

    def test_from_counts_accepts_valid_width(self):
        """Keys of exactly num_qubits bits are accepted and preserved."""
        result = ExecutionResult.from_counts("task", [0, 1], 3, 2, {"01": 1, "10": 2})
        assert result.counts == {"01": 1, "10": 2}

    def test_finish_applies_same_width_validation(self):
        """finish() must validate width and collisions like from_counts()."""
        result = ExecutionResult("task", [0, 1], 3, 2)
        with pytest.raises(ValueError):
            result.finish({"1": 1, "01": 2})
        with pytest.raises(ValueError):
            result.finish({"100": 1})
