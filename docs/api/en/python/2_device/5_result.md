# Outcome / Status / ExecutionResult

This page covers the APIs related to quantum task results:

- `Outcome`
- `Status`
- `ExecutionResult`

## Import

```python
from cqlib.device import ExecutionResult, Outcome, Status
```

---

## Outcome

### `Outcome(bitstring)`

Construct a measurement result object from a binary string.

Raises:

- `ValueError`: the string contains an illegal character (not `0/1`).

### Static methods

- `Outcome.from_bitstring(bitstring) -> Outcome`: construct from a binary string.
- `Outcome.from_indices(width, indices) -> Outcome`: construct a result of width `width` whose value is `1` only at the positions listed in `indices`.

### Methods and attributes

- `is_one(index) -> bool`: whether the given position is `1`.
- `to_bitstring(num_qubits) -> str`: output the bitstring with the given width.
- `chunks -> list[int]`: the chunks of the internal storage.

Description:

- The bitstring is always written in standard binary form (most significant bit on the left), while the internal storage uses little-endian chunks.
- The bitstring width is not stored by `Outcome`: parsing `"001"` and parsing `"1"` yield the same `Outcome`. The output width must therefore be specified by the caller through `to_bitstring`.
- `Outcome` supports `==` and `hash(...)`.

### Other behavior

- Supports `==`, `hash(...)`, `copy` and `deepcopy`.
- `repr(Outcome("101")) == "Outcome(chunks=[5])"`.

## Status

### Static constructors

- `Status.queued()`
- `Status.running()`
- `Status.completed()`
- `Status.failed(error_msg, error_code)`
- `Status.cancelled()`

### Attributes and methods

- `kind -> str` (`queued/running/completed/failed/cancelled`)
- `error_msg -> str | None`
- `error_code -> int | None`
- `is_terminal() -> bool`
- `is_success() -> bool`

### Other behavior

- Supports `==` and `hash(...)`: two statuses are equal only when the status is the same and the error message and error code are the same; comparison with an object that is not a `Status` returns `False`.
- `repr(Status.completed()) == "Status(completed)"`, `str(Status.completed()) == "Completed"`.

## ExecutionResult

### `ExecutionResult(task_id, qubits, shots, num_qubits, backend=None)`

Parameters:

- `task_id` (`str`)
- `qubits` (`list[int | Qubit]`): the list of measured qubits.
- `shots` (`int`)
- `num_qubits` (`int`)
- `backend` (`str | None`)

The constructed result is in the `queued` state.

### `ExecutionResult.from_counts(task_id, qubits, shots, num_qubits, counts, backend=None)`

Static method that constructs a completed result directly from measurement counts; it is equivalent to calling the constructor, `start()`, `finish(counts)` and `calc_probabilities()` in order.

`counts` is a `dict[str, int]`, and each key must be a valid binary bitstring whose length equals `num_qubits`.

Raises:

- `ValueError`: the bitstring of `counts` is invalid, or its length does not equal `num_qubits`.

### Lifecycle methods

- `start() -> None`
- `finish(counts) -> None`
- `fail(msg, code) -> None`
- `cancel() -> None`
- `calc_probabilities() -> None`

In `finish(counts)`, `counts` has type `dict[str, int]`, and each key must be a valid binary bitstring whose length equals `num_qubits`; writing counts clears the computed probabilities, so `calc_probabilities()` must be called again. `calc_probabilities()` produces no probabilities when the total count is 0.

Raises:

- `ValueError`: the bitstring of `counts` is invalid, or its length does not equal `num_qubits`.

### Attributes

- `task_id -> str`
- `shots -> int`
- `num_qubits -> int`
- `qubits -> list[Qubit]`: the list of measured qubits.
- `status -> Status`
- `created_at -> str`: the creation time, an ISO 8601 string.
- `started_at -> str | None`: the start time, an ISO 8601 string.
- `finished_at -> str | None`: the finish time, an ISO 8601 string.
- `backend -> str | None`
- `counts -> dict[str, int]`
- `probabilities -> dict[str, float] | None`: available only after `calc_probabilities()` is called.

Description:

- `qubits[i]` corresponds to bit `i` of the bitstring (weight `2**i`); the bitstring is written with the most significant bit on the left, so the string order is the reverse of the `qubits` order.

### Other behavior

- Supports `copy` and `deepcopy`.
- `repr(result) == "ExecutionResult(task_id='task-1', status='queued', shots=100, num_qubits=2)"`.

## Example

```python
import pytest
from cqlib import Qubit
from cqlib.device import ExecutionResult, Outcome, Status

outcome = Outcome("101")
assert outcome.is_one(0) is True
assert outcome.is_one(1) is False
assert outcome.to_bitstring(3) == "101"
assert Outcome.from_bitstring("101") == outcome
assert Outcome.from_indices(3, [0, 2]) == outcome

failed = Status.failed("backend down", 42)
assert failed.kind == "failed"
assert failed.error_code == 42
assert failed.is_terminal() is True

result = ExecutionResult("task-1", [Qubit(0), Qubit(1)], 100, 2, "sim")
print(result.status.kind)  # queued

result.start()
result.finish({"00": 60, "11": 40})
result.calc_probabilities()

print(result.status.kind)   # completed
print(result.counts)        # {'00': 60, '11': 40}
print(result.probabilities) # {'00': 0.6, '11': 0.4}

done = ExecutionResult.from_counts("task-2", [Qubit(0), Qubit(1)], 100, 2, {"01": 60, "10": 40})
print(done.status.kind)  # completed
assert done.counts == {"01": 60, "10": 40}

with pytest.raises(ValueError):
    done.finish({"100": 1})
```
