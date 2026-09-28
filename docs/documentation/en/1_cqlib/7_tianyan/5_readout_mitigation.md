# Readout error correction

Measurements on real quantum devices have readout errors. When obtaining results, `cqlib-tianyan` can use device calibration data to correct measurement counts for readout error. This capability is controlled by `CalibrationMode`.

The `backend` in the examples below is a `TianyanBackend` instance; see [Backend and device configuration](2_backend_device.md) for how to obtain one.

Corresponding import:

```python
from cqlib_tianyan import CalibrationMode
```

## 1. Why readout error correction is needed

Ideally, if a qubit is truly in `|0>`, the measurement result should always be `0`; if it is truly in `|1>`, the measurement result should always be `1`. But hardware readout has errors:

```text
true 0 -> may be read as 1
true 1 -> may be read as 0
```

Platform calibration data usually gives the readout fidelity of each qubit:

| Symbol | Meaning |
|---|---|
| `f00` | Probability of reading 0 when prepared as 0 |
| `f11` | Probability of reading 1 when prepared as 1 |

`cqlib-tianyan` builds a confusion matrix from these data and performs an approximate inversion on the observed counts.

## 2. CalibrationMode

`CalibrationMode` has three modes:

| Mode | Behavior |
|---|---|
| `auto` | Default mode. Correction is applied automatically when calibration data exists and the number of measured qubits does not exceed the threshold; otherwise it falls back to raw counts |
| `enabled` | Forced correction. An error is returned when there is no calibration data or resources are insufficient |
| `disabled` | No correction; raw counts are always returned |

Creation:

```python
from cqlib_tianyan import CalibrationMode

mode = CalibrationMode("auto")
assert mode == "auto"
```

In most cases a string can be passed directly:

```python
task = backend.run_with_mode(
    circuits=["H Q1\nM Q1"],
    shots=1000,
    mode="disabled",
)
```

## 3. Default behavior

`backend.run()` uses the `auto` mode by default:

```python
task = backend.run(["H Q1\nM Q1"], shots=1000)
results = task.wait(timeout_secs=120.0)
```

The `auto` mode applies readout error correction automatically when the conditions are met.

## 4. Getting raw results

To obtain raw counts only without any correction, use `run_raw`:

```python
task = backend.run_raw(["H Q1\nM Q1"], shots=1000)
raw_results = task.wait(timeout_secs=120.0)
```

Or use `wait_raw` on an existing task:

```python
task = backend.run(["H Q1\nM Q1"], shots=1000)
raw_results = task.wait_raw(timeout_secs=120.0)
```

## 5. Specifying the correction mode explicitly

```python
# automatic correction
task = backend.run_with_mode(["H Q1\nM Q1"], shots=1000, mode="auto")

# forced correction
task = backend.run_with_mode(["H Q1\nM Q1"], shots=1000, mode="enabled")

# no correction
task = backend.run_with_mode(["H Q1\nM Q1"], shots=1000, mode="disabled")
```

A `CalibrationMode` object can also be passed:

```python
mode = CalibrationMode("disabled")
task = backend.run_with_mode(["H Q1\nM Q1"], shots=1000, mode=mode)
```

## 6. Qubit count threshold of the auto mode

Readout error correction requires building a confusion matrix. If the number of measured qubits is `n`, the full matrix size is:

```text
2^n x 2^n
```

The memory complexity is approximately:

```text
O(4^n)
```

Therefore the `auto` mode applies correction automatically only when the number of measured qubits does not exceed 14 by default. Above the threshold it falls back to raw counts, avoiding excessive memory overhead.

If forced correction is genuinely required, use:

```python
task = backend.run_with_mode(circuits, shots=1000, mode="enabled")
```

However, this requires the caller to confirm that memory resources are sufficient.

## 7. Comparing results before and after correction

```python
qcis = "H Q1\nM Q1"

task = backend.run([qcis], shots=1000)

calibrated = task.wait(timeout_secs=120.0)
raw = task.wait_raw(timeout_secs=120.0)

print("corrected:", calibrated[0].counts, calibrated[0].probabilities)
print("raw:", raw[0].counts, raw[0].probabilities)
```

Note: the corrected counts are still returned in `ExecutionResult`, in the `counts` field, and `probabilities` are the probabilities normalized from counts.

## 8. Relationship with the device configuration

Readout error correction depends on device calibration data. The device configuration can be checked first:

```python
device = backend.device_config()
```

If there is no usable calibration data:

- `auto` falls back to raw counts.
- `enabled` raises an error.
- `disabled` is unaffected.

## 9. Selection recommendations

| Scenario | Recommended mode |
|---|---|
| Ordinary experiments | `auto` |
| Raw hardware output only | `disabled` or `wait_raw()` |
| Validating an error correction algorithm | `enabled` |
| A large number of measured qubits | `disabled`, or `enabled` after evaluating memory manually |
| Comparing hardware output and correction effects | Use `wait()` and `wait_raw()` together |

## 10. Common issues

| Symptom | Cause | Handling |
|---|---|---|
| `auto` does not change the result noticeably | No calibration data, the number of measured qubits exceeds the threshold, or the readout error itself is small | Verify with `enabled`, or check the device calibration data |
| `enabled` raises an error | Missing calibration data or insufficient resources | Switch to `auto` or `disabled` |
| Very small probability entries appear in the result | Inverse matrix correction redistributes probabilities | Set a post-processing threshold according to the experimental requirements |
| Large-qubit-count tasks are slow | The correction matrix size grows exponentially with the qubit count | Disable correction or split the experiment |

## Next steps

- [Cqlib tutorial entry point](../../0_get_started/0_introduction.md): return to the start of the tutorials and continue with other Cqlib modules.
- [QCIS format description](../1_ir/1_qcis.md): learn about the QCIS text format, the import and export interfaces and the support boundaries.
- [Device module](../2_device/0_overview.md): learn about device topology, device configuration, noise models and the execution result object.
