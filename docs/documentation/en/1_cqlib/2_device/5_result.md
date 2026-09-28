# Execution result and status

The execution result module of cqlib.device provides measurement result encapsulation, task status tracking and probability calculation.

---

## Outcome: measurement result

Outcome uses little-endian bit order: the rightmost character of the string corresponds to Qubit 0, and the leftmost corresponds to Qubit N-1.

```python
from cqlib.device import Outcome

# create from a bit string (little-endian: rightmost = Qubit 0)
o = Outcome("101")
print("is bit 0 one:", o.is_one(0))   # True
print("is bit 1 one:", o.is_one(1))   # False
print("full bitstring (3):", o.to_bitstring(3))  # "101"

# construct from a bit string (equivalent to direct construction)
o2 = Outcome.from_bitstring("101")
print("o == o2:", o == o2)  # True

# construct from an index list (specifying which bit positions are 1)
o3 = Outcome.from_indices(width=3, indices=[0, 2])
print("constructed from indices:", o3.to_bitstring(3))  # "101"
```

**Notes**:
- The string may contain only '0' and '1'; other characters raise ValueError
- 
to_bitstring(num_qubits) if 
num_qubits is greater than the actual width, the high-order bits are padded with zeros

---

## Status: task status

Status represents the execution stage of a quantum task, and supports the following five states:

| State | Constructor | Terminal |
|---|---|---|
| Queued | Status.queued() | No |
| Running | Status.running() | No |
| Completed | Status.completed() | Yes |
| Failed | Status.failed(msg, code) | Yes |
| Cancelled | Status.cancelled() | Yes |

```python
from cqlib.device import Status

q = Status.queued()
c = Status.completed()
f = Status.failed("backend down", 500)
x = Status.cancelled()

print("queued:", q.kind, "terminal?", q.is_terminal())
print("completed:", c.kind, "success?", c.is_success())
print("failed:", f.kind, f.error_msg, f.error_code)
print("cancelled:", x.kind, "terminal?", x.is_terminal())
```

**Notes**:
- Status.kind is a property (not a method) and returns a string, such as "completed" or "failed"
- is_terminal() returns True for completed, failed and cancelled
- is_success() returns True only for completed
- error_msg and error_code have values only when kind == "failed"; otherwise they return None

---

## ExecutionResult: the complete execution result

```python
from cqlib.device import ExecutionResult

result = ExecutionResult("q-task-001", [0, 1], 1000, 2, "Tianyan-176-2")
print("status on creation:", result.status.kind)  # queued

# mark as running
result.start()
print("status after start:", result.status.kind)  # running

# finish and fill in the measurement counts
result.finish({"00": 600, "11": 400})
result.calc_probabilities()  # compute the probability distribution

print("task ID:", result.task_id)
print("shots:", result.shots)
print("counts:", result.counts)
print("probabilities:", result.probabilities)
print("backend:", result.backend)
```

### Constructing directly from counts

If measurement results are already available, from_counts can be used to complete creation and filling in one step:

```python
from cqlib.device import ExecutionResult

result = ExecutionResult.from_counts(
    task_id="q-task-002", qubits=[0, 1],
    shots=1024, num_qubits=2,
    counts={"00": 512, "11": 512},
    backend="simulator",
)
print("status:", result.status.kind)        # completed (finished automatically)
print("probabilities:", result.probabilities)  # {"00": 0.5, "11": 0.5}
```

---

## Exception flow handling

```python
from cqlib.device import ExecutionResult

# failure scenario
f = ExecutionResult("task-fail", [0], 10, 1, None)
f.fail("timeout", 408)
print("failed status:", f.status.kind)              # failed
print("error message:", f.status.error_msg)          # timeout
print("error code:", f.status.error_code)           # 408

# cancellation scenario
c = ExecutionResult("task-cancel", [0], 10, 1, None)
c.cancel()
print("cancelled status:", c.status.kind)               # cancelled
```

---

## Input validation

```python
from cqlib.device import ExecutionResult

r = ExecutionResult("bad", [0], 10, 1, None)
try:
    r.finish({"2": 1})  # "2" is not a valid binary string
except ValueError as e:
    print("invalid counts:", e)
```

---

## Next steps

- [Quantum Information](../3_qis/0_overview.md): master the basics such as Statevector, DensityMatrix and Pauli
- [Compilation and optimization](../4_compiler/0_overview.md): learn about the layout, routing and optimization of the compilation pipeline
- [Visualization](../5_visualization/0_overview.md): learn about circuit drawing and result visualization
