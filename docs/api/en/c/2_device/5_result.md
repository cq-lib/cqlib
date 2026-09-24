# ExecutionResult (C)

`CExecutionResult` is the opaque handle to a task execution result, recording one task's lifecycle, measurement counts, and probability distribution; `CCountsList` is a bitstring-to-weight key/value list. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md).

Entry `i` of the `qubits` array corresponds to bit `i` of the outcome keys; bitstrings are written with the most significant bit on the left, the reverse of the `qubits` order.

---

## Status constants

Tags written by `execution_result_status`:

| Constant | Value | Meaning |
| --- | --- | --- |
| `EXECUTION_STATUS_QUEUED` | 0 | Task submitted and queued. |
| `EXECUTION_STATUS_RUNNING` | 1 | Task currently running. |
| `EXECUTION_STATUS_COMPLETED` | 2 | Task completed successfully. |
| `EXECUTION_STATUS_FAILED` | 3 | Task failed (query the error code and message for details). |
| `EXECUTION_STATUS_CANCELLED` | 4 | Task cancelled by the user. |

---

## Construction and release

### execution_result_new(task_id, qubits, num_qubits, shots, backend)

Creates an execution result in Queued status; the creation timestamp is the current time.

Parameters:

- `task_id` (`const char*`): task ID.
- `qubits` (`const uint32_t*`): array of measured-qubit IDs; entry `i` corresponds to bit `i` of the outcome keys.
- `num_qubits` (`uintptr_t`): number of measured qubits.
- `shots` (`uintptr_t`): shot count.
- `backend` (`const char*`): backend name; may be NULL.

Returns: a newly allocated `CExecutionResult*` on success; NULL on error.

### execution_result_from_counts(task_id, num_qubits, shots, bitstrings, counts, num_entries)

Creates a completed (Completed) result directly from measurement counts; the measured qubits are `0..num_qubits-1`.

Parameters:

- `task_id` (`const char*`): task ID.
- `num_qubits` (`uintptr_t`): number of measured qubits; each bitstring must have exactly `num_qubits` characters.
- `shots` (`uintptr_t`): shot count.
- `bitstrings` (`const char *const*`): array of bitstrings, parallel to `counts`.
- `counts` (`const uint64_t*`): array of counts, parallel to `bitstrings`.
- `num_entries` (`uintptr_t`): number of entries.

Returns: a newly allocated `CExecutionResult*` on success; NULL on error.

### execution_result_free(ptr)

Frees a `CExecutionResult`; NULL is allowed.

---

## Lifecycle

### execution_result_start(ptr)

Marks the job as running and records the current time as the start timestamp.

Returns: 0 on success; -1 on NULL.

### execution_result_finish(ptr, bitstrings, counts, num_entries)

Marks the job as completed and replaces the measurement counts; `bitstrings` and `counts` are parallel arrays, repeated bitstrings accumulate, and the finish timestamp is the current time.

Parameters:

- `bitstrings` (`const char *const*`): array of bitstrings.
- `counts` (`const uint64_t*`): array of counts.
- `num_entries` (`uintptr_t`): number of entries.

Returns: 0 on success; -1 on NULL; -4 for an invalid bitstring (length or characters).

### execution_result_fail(ptr, msg, code)

Marks the job as failed and records an error message and code.

Parameters:

- `msg` (`const char*`): error message.
- `code` (`int32_t`): error code.

Returns: 0 on success; -1 on NULL; -4 when the message is not valid UTF-8.

### execution_result_cancel(ptr)

Marks the job as cancelled.

Returns: 0 on success; -1 on NULL.

---

## Status and timestamps

### execution_result_status(ptr, out_tag, out_error_code)

Writes the execution status tag (one of the `EXECUTION_STATUS_*` constants). When the status is failed, `*out_error_code` receives the stored error code (0 otherwise); `out_error_code` may be NULL when not needed.

Parameters:

- `out_tag` (`uint8_t*`): receives the status tag.
- `out_error_code` (`int32_t*`): receives the error code; may be NULL.

Returns: 0 on success; -1 on NULL.

### execution_result_is_success(ptr)

Returns whether the execution completed successfully.

Returns: 1 when successful; 0 otherwise; -1 on NULL.

### execution_result_is_terminal(ptr)

Returns whether the execution reached a terminal state (completed, failed, or cancelled); Queued and Running are not terminal.

Returns: 1 when terminal; 0 otherwise; -1 on NULL.

### execution_result_error_message(ptr)

Returns the error message stored by the failed status.

Returns: a heap-allocated C string on success (free with `cqlib_string_free`); NULL on NULL input or when the status is not failed.

### execution_result_task_id(ptr)

Returns the task ID.

Returns: a heap-allocated C string on success (free with `cqlib_string_free`); NULL on error.

### execution_result_backend(ptr)

Returns the backend name.

Returns: a heap-allocated C string on success (free with `cqlib_string_free`); NULL on error or when no backend is set.

### execution_result_created_at(ptr, out_ms)

Writes the task creation timestamp in milliseconds since the Unix epoch.

Returns: 0 with the value in `*out_ms`; -1 on NULL.

### execution_result_started_at(ptr, out_ms)

Writes the execution start timestamp in milliseconds since the Unix epoch.

Returns: 0 with the value in `*out_ms`; -8 when the job has not started.

### execution_result_finished_at(ptr, out_ms)

Writes the execution finish timestamp in milliseconds since the Unix epoch.

Returns: 0 with the value in `*out_ms`; -8 when the job has not finished.

---

## Measurement results

### execution_result_shots(ptr)

Returns the number of shots; 0 for NULL.

### execution_result_num_qubits(ptr)

Returns the number of measured qubits; 0 for NULL.

### execution_result_qubits_len(ptr) / execution_result_qubits(ptr, out, len)

Two-step output of the measured-qubit IDs; entry `i` corresponds to bit `i` of the outcome keys. The fill function returns the total count.

### execution_result_to_bitstring(ptr, outcome)

Formats the raw outcome bitmask as a big-endian bitstring over the result's measured-qubit width: bit `i` of `outcome` is measured qubit `i`, and high bits beyond the width are truncated.

Parameters:

- `outcome` (`uint64_t`): raw outcome bitmask.

Returns: a heap-allocated C string on success (free with `cqlib_string_free`); NULL on NULL input.

### execution_result_counts(ptr)

Returns the measurement counts.

Returns: a newly allocated `CCountsList*` on success (bitstring keys, integer counts as f64 weights), freed with `counts_list_free`; NULL on error.

### execution_result_probabilities(ptr)

Returns the probability distribution (computed first if needed, by normalizing the counts).

Returns: a newly allocated `CCountsList*` on success (bitstring keys, f64 probability weights), freed with `counts_list_free`; NULL on error.

### execution_result_calc_probabilities_len(ptr) / execution_result_calc_probabilities(ptr, out, len)

Two-step probability distribution: `calc_probabilities_len` computes the distribution (cached on the result) and returns the number of entries; `calc_probabilities` recomputes it and copies the probabilities into `out`. Entries are sorted ascending by outcome value, where bit `i` of the outcome value is measured qubit `i` (matching `execution_result_to_bitstring`). Both take a mutable handle; a smaller `len` copies only the first `len` entries. The fill function returns the total number of entries; 0 for NULL.

---

## CCountsList

The return value of `execution_result_counts` and `execution_result_probabilities`: keys are measurement-outcome bitstrings; the former carries counts as weights, the latter probabilities.

### counts_list_len(ptr)

Returns the number of entries in the list; 0 for NULL.

### counts_list_get_key(ptr, index)

Returns the bitstring key at `index`.

Returns: a heap-allocated C string on success (free with `cqlib_string_free`); NULL on out-of-bounds.

### counts_list_get_value(ptr, index)

Returns the weight (count or probability) at `index`; NaN on error.

### counts_list_free(ptr)

Frees a `CCountsList`; NULL is allowed.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* path one: build a completed result directly from counts */
    const char *bits[2] = {"00", "11"};
    uint64_t cnts[2] = {750, 250};

    struct CExecutionResult *r = execution_result_from_counts(
        "task-1", 2, 1000, bits, cnts, 2);
    if (r == NULL) {
        return 1;
    }

    uint8_t status = 255;
    int32_t err_code = 0;
    execution_result_status(r, &status, &err_code);  /* EXECUTION_STATUS_COMPLETED */

    struct CCountsList *counts = execution_result_counts(r);
    if (counts != NULL) {
        for (uintptr_t i = 0; i < counts_list_len(counts); i++) {
            char *key = counts_list_get_key(counts, i);
            double w = counts_list_get_value(counts, i);
            printf("%s: %f\n", key, w);  /* "00": 750, "11": 250 */
            cqlib_string_free(key);
        }
        counts_list_free(counts);
    }

    struct CCountsList *probs = execution_result_probabilities(r);
    /* same traversal: weights are 0.75 / 0.25 */
    counts_list_free(probs);

    /* status flags, bitstrings, and the two-step probability buffer */
    int32_t ok = execution_result_is_success(r);     /* 1 */
    int32_t done = execution_result_is_terminal(r);  /* 1 */

    char *bs = execution_result_to_bitstring(r, 0b01);     /* "01" */
    char *bs2 = execution_result_to_bitstring(r, 0b101);   /* "11": bit 2 is beyond the width, truncated */
    cqlib_string_free(bs);
    cqlib_string_free(bs2);

    uintptr_t n_probs = execution_result_calc_probabilities_len(r);  /* 2 */
    double pvals[2];
    execution_result_calc_probabilities(r, pvals, n_probs);
    /* ascending outcome order: pvals[0] == 0.75 ("00"), pvals[1] == 0.25 ("11") */

    execution_result_free(r);

    /* path two: the full lifecycle */
    uint32_t qubits[2] = {0, 1};
    struct CExecutionResult *r2 = execution_result_new(
        "task-2", qubits, 2, 1000, "sim");
    if (r2 == NULL) {
        return 1;
    }

    execution_result_start(r2);                  /* Queued -> Running */
    execution_result_finish(r2, bits, cnts, 2);  /* Running -> Completed */

    char *task = execution_result_task_id(r2);     /* "task-2" */
    char *backend = execution_result_backend(r2); /* "sim" */
    cqlib_string_free(task);
    cqlib_string_free(backend);

    int64_t created = 0, started = 0, finished = 0;
    execution_result_created_at(r2, &created);
    execution_result_started_at(r2, &started);
    execution_result_finished_at(r2, &finished);

    execution_result_free(r2);
    return 0;
}
```
