# Task submission and result retrieval

The basic unit of circuit execution on the Tianyan platform is the task. In `cqlib-tianyan`, a `TaskHandle` is returned after task submission, through which the task status can be queried or results can be awaited in a blocking manner.

## 1. Submitting a QCIS circuit

The task submission interface in the Python bindings accepts a list of QCIS strings:

The `backend` in the examples below is a `TianyanBackend` instance; see [Backend and device configuration](2_backend_device.md) for how to obtain one.

```python
qcis = "H Q1\nM Q1"
task = backend.run([qcis], shots=1000)
```

Parameter description:

| Parameter | Description |
|---|---|
| `circuits` | List of QCIS strings |
| `shots` | Number of shots for each circuit |

The return value is a `TaskHandle`.

## 2. Submitting through a backend

```python
import os
from cqlib_tianyan import TianyanPlatform

platform = TianyanPlatform.login(os.environ["TIANYAN_API_KEY"])
backend = platform.get_backend("tianyan-287")

circuits = [
    "H Q1\nM Q1",
    "X Q1\nM Q1",
]

task = backend.run(circuits, shots=1000)
print(task.task_ids)
```

## 3. Submitting directly through the platform

If the target device name is already known, `get_backend` can be skipped:

```python
task = platform.submit(
    circuits=["H Q1\nM Q1"],
    shots=1000,
    device_name="tianyan-287",
)
```

This approach suits server-side or automation scripts, because it removes one backend list query.

## 4. TaskHandle fields

| Field | Description |
|---|---|
| `task_ids` | List of task IDs returned by the platform |
| `device_name` | Name of the backend used for submission |
| `shots` | Shots requested for each circuit |
| `submitted_at` | Submission time, an ISO 8601 string |

Example:

```python
print(task.task_ids)
print(task.device_name)
print(task.shots)
print(task.submitted_at)
```

## 5. Non-blocking query

`status()` queries only once and returns the results that have already completed. Circuits that have not yet completed do not appear in the returned list.

```python
partial_results = task.status()

print(f"completed {len(partial_results)} / {len(task.task_ids)}")
for result in partial_results:
    print(result.task_id, result.counts)
```

Suitable for implementing polling logic manually, or for refreshing task status periodically in a UI.

## 6. Blocking wait

`wait()` polls until all circuits are complete, or until a timeout occurs.

```python
results = task.wait(
    timeout=120.0,
    poll_interval=5.0,
)

for result in results:
    print(result.task_id)
    print(result.counts)
    print(result.probabilities)
```

Parameter description:

| Parameter | Description |
|---|---|
| `timeout` | Maximum number of seconds to wait |
| `poll_interval` | Polling interval in seconds, default `5.0` |

`wait()` releases the Python GIL while blocking, so it does not block other Python threads from running.

## 7. Getting raw results

By default, `wait()` decides whether to apply readout error correction according to the `CalibrationMode` at submission time. To obtain raw counts only, use:

```python
raw_results = task.wait_raw(
    timeout=120.0,
    poll_interval=5.0,
)
```

## 8. Result object

`wait()`, `wait_raw()` and `status()` return a list of `cqlib.device.ExecutionResult`.

Common fields:

| Field | Description |
|---|---|
| `task_id` | Platform task ID |
| `qubits` | Measured qubits |
| `shots` | Actual number of shots |
| `num_qubits` | Number of qubits in the result object |
| `backend` | Backend name |
| `counts` | Measurement counts dictionary |
| `probabilities` | Probabilities normalized from counts |
| `status` | Execution status |

Example:

```python
result = results[0]

print(result.task_id)
print(result.backend)
print(result.qubits)
print(result.counts)
print(result.probabilities)
```

## 9. Batch submission

A single request on the Tianyan platform accepts at most 50 circuits. `cqlib-tianyan` splits larger inputs automatically internally:

```python
circuits = ["H Q1\nM Q1"] * 120

task = backend.run(circuits, shots=1000)
print(len(task.task_ids))  # 120
```

Internally, submission is batched as `50 + 50 + 20`, and all task IDs are merged into the same `TaskHandle`.

## 10. Pre-submission checks

It is recommended to check the following before submission:

- Whether the backend is `running`.
- Whether the QCIS text can be parsed correctly by `cqlib.ir.qcis.loads`.
- Whether the circuit contains gates not supported by the target device.
- Whether measurement has been added.
- Whether shots meet the experimental requirements and platform limits.
- Whether a batch task needs a small-scale trial run first.

## 11. Common issues

| Symptom | Common cause | Handling |
|---|---|---|
| Submission fails | Backend unavailable, QCIS format error, invalid API Key or network problem | Check `backend.status`, the QCIS text and the authentication status |
| `wait` times out | Long queue wait time or the task has not completed | Increase `timeout`, or use `status()` to query in batches |
| Fewer results returned than submitted circuits | `status()` was used, which returns only completed results | Use `wait()` to wait for all of them to complete |
| counts is empty | The platform result has not completed or parsing failed | Check the task status and the raw error message |
| Probabilities and counts are inconsistent | Probabilities are normalized from counts, and the correction mode may change counts | Compare `wait()` with `wait_raw()` |

## Next steps

- [QCIS and IR integration](4_qcis_ir_workflow.md): export a Cqlib `Circuit` to QCIS and connect it to the Tianyan task submission flow.
- [Readout error correction](5_readout_mitigation.md): learn about `CalibrationMode` and the difference between raw results and corrected results.
