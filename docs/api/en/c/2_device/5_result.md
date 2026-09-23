# ExecutionResult

`CExecutionResult` records task sampling data. The current C API provides an entry point that constructs it from measurement counts (the device-side task execution entry point will be provided in a later version).

The bitstring is in big-endian format: the MSB is on the left, qubit `N-1` comes first and qubit `0` comes last, in the same direction as the output of [measure_all](../3_qis/1_statevector.md#probabilities-and-measurement).

---

## Functions

### execution_result_from_counts(task_id, num_qubits, shots, bitstrings, counts, num_entries)

Construct an execution result from measurement counts.

Parameters:

- `task_id` (`const char *`): the task identifier string.
- `num_qubits` (`uintptr_t`): the number of qubits.
- `shots` (`uintptr_t`): the total number of shots.
- `bitstrings` (`const char *const *`): the array of bitstrings.
- `counts` (`const uint64_t *`): the array of counts, parallel to `bitstrings`.
- `num_entries` (`uintptr_t`): the number of entries.

Constraints:

- Each bitstring must have length equal to `num_qubits`; otherwise NULL is returned.

Returns:

- `CExecutionResult *`: a heap-allocated result; release it with `execution_result_free`; NULL on failure.

### execution_result_shots(result)

Return the total number of shots.

Returns:

- `uintptr_t`; a NULL handle returns `0`.

### execution_result_num_qubits(result)

Return the number of qubits.

Returns:

- `uintptr_t`; a NULL handle returns `0`.

### execution_result_counts(result)

Return the list of measurement counts.

Returns:

- `CCountsList *`: a new **independently owned** object; release it with `counts_list_free`; the list values are counts (integer weights carried as `f64`); NULL on failure.

### execution_result_probabilities(result)

Compute (deriving from the counts when necessary) and return the probability distribution list.

Returns:

- `CCountsList *`: a new **independently owned** object; release it with `counts_list_free`; the list values are probabilities (`f64` in `0..1`); NULL on failure.

### execution_result_free(ptr)

Release an execution result. Passing NULL is allowed.

---

## CountsList traversal

`execution_result_counts` and `execution_result_probabilities` share `CCountsList`:

```c
uintptr_t counts_list_len(const struct CCountsList *ptr);
char *counts_list_get_key(const struct CCountsList *ptr, uintptr_t index);
double counts_list_get_value(const struct CCountsList *ptr, uintptr_t index);
void counts_list_free(struct CCountsList *ptr);
```

| Function | Returns | Description |
| --- | --- | --- |
| `counts_list_len` | Number of entries | A NULL handle returns `0` |
| `counts_list_get_key(index)` | `char *` | A heap-allocated bitstring; release it with `cqlib_string_free`; NULL when out of range |
| `counts_list_get_value(index)` | `double` | The count or the probability; NaN on error |
| `counts_list_free` | `void` | Release the list; passing NULL is allowed |

---

## Example

### Construction and traversal

```c
const char *bits[2] = {"00", "11"};
const uint64_t cnt[2] = {480, 520};

CExecutionResult *res = execution_result_from_counts("task-1", 2, 1000, bits, cnt, 2);
if (!res) { return 1; }

printf("shots=%zu qubits=%zu\n",
       (size_t)execution_result_shots(res),
       (size_t)execution_result_num_qubits(res));

CCountsList *counts = execution_result_counts(res);
for (uintptr_t i = 0; i < counts_list_len(counts); i++) {
    char *k = counts_list_get_key(counts, i);
    printf("count(%s) = %.0f\n", k, counts_list_get_value(counts, i));
    cqlib_string_free(k);
}
counts_list_free(counts);

CCountsList *probs = execution_result_probabilities(res);
for (uintptr_t i = 0; i < counts_list_len(probs); i++) {
    char *k = counts_list_get_key(probs, i);
    printf("P(%s) = %.3f\n", k, counts_list_get_value(probs, i));
    cqlib_string_free(k);
}
counts_list_free(probs);

execution_result_free(res);
```
