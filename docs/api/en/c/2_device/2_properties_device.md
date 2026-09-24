# Device / Properties (C)

`CDevice` is the opaque handle to a device: on top of a topology it aggregates the device name, native gate set, qubit registry, calibration time, and property defaults, and provides validation and error queries. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md).

Qubit-property queries prefer local values: `device_get_t1` and friends first read the properties recorded for that qubit and fall back to the device-wide default. Once a qubit or edge records its own native-instruction list, that list fully replaces the device-level capability at that location and never writes back to the default gate set.

---

## Property structs

### CQubitProp

Snapshot of the properties recorded for one physical qubit, written by `device_qubit_properties`:

| Field | Type | Meaning |
| --- | --- | --- |
| `readout_error` | `double` | Readout error rate in [0, 1]. |
| `t1` | `double` | T1 relaxation time in microseconds; NaN when unset. |
| `t2` | `double` | T2 dephasing time in microseconds; NaN when unset. |
| `prob_meas0_prep1` | `double` | P(measure 0 \| prepared 1); NaN when unset. |
| `prob_meas1_prep0` | `double` | P(measure 1 \| prepared 0); NaN when unset. |
| `frequency` | `double` | Frequency in GHz; NaN when unset. |
| `num_native_instructions` | `uintptr_t` | Number of native instructions recorded for the qubit; the index bound for `device_qubit_prop_native_instruction`. |

### CNativeInstruction

Snapshot of one native instruction capability, written by `device_qubit_prop_native_instruction` / `device_edge_prop_native_instruction`:

| Field | Type | Meaning |
| --- | --- | --- |
| `name` | `char*` | Heap-allocated standard-gate name; free with `cqlib_string_free`. |
| `error_rate` | `double` | Error rate in [0, 1]. |
| `length` | `double` | Duration in nanoseconds; NaN when unset. |

### CEdgeProp

Snapshot of the properties recorded for one directed coupling, written by `device_edge_properties`:

| Field | Type | Meaning |
| --- | --- | --- |
| `num_native_instructions` | `uintptr_t` | Number of native instructions recorded for the edge; the index bound for `device_edge_prop_native_instruction`. |

### CQubitPropInput

Input for `device_add_qubit_properties`. Optional `double` fields use NaN to leave the value unset; native instructions are supplied as parallel arrays of length `num_native_instructions` (gate names, error rates, durations). `native_lengths` may be NULL to leave all durations unset:

| Field | Type | Meaning |
| --- | --- | --- |
| `readout_error` | `double` | Readout error rate in [0, 1]. |
| `t1` | `double` | T1 relaxation time in microseconds; NaN = unset. |
| `t2` | `double` | T2 dephasing time in microseconds; NaN = unset. |
| `prob_meas0_prep1` | `double` | P(measure 0 \| prepared 1); NaN = unset. |
| `prob_meas1_prep0` | `double` | P(measure 1 \| prepared 0); NaN = unset. |
| `frequency` | `double` | Frequency in GHz; NaN = unset. |
| `native_gate_names` | `const char *const*` | Array of standard-gate names, or NULL when none. |
| `native_error_rates` | `const double*` | Array of error rates, or NULL when none. |
| `native_lengths` | `const double*` | Array of durations in ns (NaN = unset), or NULL. |
| `num_native_instructions` | `uintptr_t` | Number of native instructions. |

### CEdgePropInput

Input for `device_add_edge_properties`; fields mirror the native-instruction part of `CQubitPropInput`, and only two-qubit standard gates are accepted:

| Field | Type | Meaning |
| --- | --- | --- |
| `native_gate_names` | `const char *const*` | Array of standard-gate names, or NULL when none. |
| `native_error_rates` | `const double*` | Array of error rates, or NULL when none. |
| `native_lengths` | `const double*` | Array of durations in ns (NaN = unset), or NULL. |
| `num_native_instructions` | `uintptr_t` | Number of native instructions. |

---

## Construction

### device_new(name, num_qubits)

Creates a device with `num_qubits` isolated physical qubits and no couplings; use `device_from_edges` for custom topologies.

Parameters:

- `name` (`const char*`): device name.
- `num_qubits` (`uint32_t`): number of physical qubits.

Returns: a newly allocated `CDevice*` on success; NULL on error.

### device_line(name, num_qubits)

Creates a device with a directed line topology `0 -> 1 -> ... -> n-1`.

### device_bidirectional_line(name, num_qubits)

Creates a device with a bidirectional line topology.

### device_ring(name, num_qubits)

Creates a device with a bidirectional ring topology.

### device_grid(name, rows, cols)

Creates a device with a bidirectional grid topology; qubit IDs are row-major.

Parameters:

- `rows` (`uint32_t`): number of rows.
- `cols` (`uint32_t`): number of columns.

### device_star(name, num_qubits, center)

Creates a device with a bidirectional star topology around `center`.

Parameters:

- `num_qubits` (`uint32_t`): total number of qubits.
- `center` (`uint32_t`): center qubit ID.

### device_from_edges(name, num_qubits, edges, num_edges)

Creates a device from explicit directed edges. `edges` points to `2 * num_edges` u32 values laid out as consecutive `(control, target)` pairs.

Parameters:

- `num_qubits` (`uint32_t`): number of physical qubits.
- `edges` (`const uint32_t*`): coupling-edge array.
- `num_edges` (`uintptr_t`): number of edges.

Returns: a newly allocated `CDevice*` on success; NULL on error.

### device_line_from_qubits(name, qubits, num_qubits)

Creates a device whose physical qubits (given by ID array) are connected as a directed line in the supplied order.

Parameters:

- `qubits` (`const uint32_t*`): array of physical qubit IDs.
- `num_qubits` (`uintptr_t`): array length.

Returns: a newly allocated `CDevice*` on success; NULL on error.

### device_free(ptr)

Frees a `CDevice`; NULL is allowed.

Parameters:

- `ptr` (`struct CDevice*`): handle to free.

---

## Basic information and native gates

### device_name(ptr)

Returns the device name.

Returns: a heap-allocated C string on success (free with `cqlib_string_free`); NULL on error.

### device_num_qubits(ptr)

Returns the number of usable physical qubits; 0 for NULL.

### device_native_gates(ptr)

Returns the native gate names.

Returns: a comma-separated heap-allocated C string on success (e.g. `"H,CX"`), freed with `cqlib_string_free`; NULL on error or when the gate set is empty.

### device_with_native_gates(ptr, gate_names)

Replaces the device's native gate set with the gates named in `gate_names` (comma separated, e.g. `"H,CX,RZ"`).

Parameters:

- `gate_names` (`const char*`): gate-name list.

Returns: 0 on success; -4 when any name is unknown; other negative values on error.

### device_topology(ptr)

Returns a clone of the device topology; free it with `topology_free`.

Returns: a newly allocated `CTopology*` on success; NULL on error.

---

## Validation

### device_validate_circuit(ptr, circuit)

Validates that `circuit` is compatible with the device.

Parameters:

- `circuit` (`const struct CCircuit*`): circuit to validate.

Returns: 0 when valid; -3 when validation fails; other negative values on error.

### device_validate_operation(ptr, operation)

Validates one resolved operation snapshot (from `circuit_index`) against the device in the physical-qubit ID space.

Parameters:

- `operation` (`const struct CValueOperation*`): the operation snapshot.

Returns: 0 when valid; -3 when the device rejects the operation; -1 on NULL.

### device_validate_value_operation(ptr, operation)

Validates one resolved value-level operation snapshot (from `circuit_index`) against the device in the physical-qubit ID space; control-flow bodies are checked recursively.

Parameters:

- `operation` (`const struct CValueOperation*`): the operation snapshot.

Returns: 0 when valid; -3 when the device rejects the operation; -1 on NULL.

### device_supports_native_instruction(ptr, gate_name, qargs, num_qargs)

Checks whether the standard gate `gate_name` can execute natively on the exact ordered physical `qargs`.

Parameters:

- `gate_name` (`const char*`): standard-gate name.
- `qargs` (`const uint32_t*`): array of physical qubit IDs; an empty list is valid for `GPhase`.
- `num_qargs` (`uintptr_t`): array length.

Returns: 1 when supported; 0 when not; -1 on NULL; -4 for an unknown gate name. Calibration values do not affect this capability query.

---

## Error and coherence properties

### device_get_t1(ptr, qubit, out)

Returns the T1 relaxation time (μs) for `qubit`, preferring the local property and falling back to the device default.

Parameters:

- `qubit` (`uint32_t`): physical qubit ID.
- `out` (`double*`): receives the value.

Returns: 0 with the value in `*out`; -8 when no T1 data is available for the qubit.

### device_get_t2(ptr, qubit, out)

Returns the T2 dephasing time (μs) for `qubit`, with the same fallback rule. Returns: 0 with the value in `*out`; -8 when unset.

### device_get_readout_error(ptr, qubit, out)

Returns the readout error rate for `qubit`, with the same fallback rule. Returns: 0 with the value in `*out`; -8 when unset.

### device_single_qubit_error(ptr, gate_name, qubit, out)

Returns the error rate of the single-qubit standard gate `gate_name` on `qubit`, preferring the local native-instruction property and falling back to the device default single-qubit error rate.

Parameters:

- `gate_name` (`const char*`): standard-gate name.
- `qubit` (`uint32_t`): physical qubit ID.
- `out` (`double*`): receives the value.

Returns: 0 with the value in `*out`; -4 for an unknown gate name; -8 when the qubit does not support the gate.

### device_two_qubit_error(ptr, gate_name, control, target, out)

Returns the error rate of the two-qubit standard gate `gate_name` on the directed coupling `control -> target`, with the same fallback rule.

Returns: 0 with the value in `*out`; -4 for an unknown gate name; -8 when the coupling does not support the gate.

### device_edge_error(ptr, control, target, out)

Returns the direction-specific coupling error for `control -> target`: the best calibrated native two-qubit error on the edge, or the device default.

Returns: 0 with the value in `*out`; -8 when the directed coupling does not exist.

### device_calibration_time(ptr, out_unix_ms)

Returns the system calibration timestamp as milliseconds since the Unix epoch.

Parameters:

- `out_unix_ms` (`int64_t*`): receives the value.

Returns: 0 with the value in `*out`; -8 when unset.

---

## Per-qubit property setters and getters

The getters below read the values recorded for one qubit; unlike `device_get_t1`, the frequency and preparation/measurement-probability getters have no device-default fallback. The setters edit a single field of the properties already recorded for the qubit (for example via `device_add_qubit_properties`) and preserve every other recorded property; they do not create the record. The indexed variants address the native-instruction list, bounded by `num_native_instructions` from `device_qubit_properties`. An unset optional value is reported as -8 with NaN written to `*out`.

### device_get_frequency(ptr, qubit, out)

Returns the qubit frequency (GHz) recorded for `qubit` (no default fallback).

Parameters:

- `qubit` (`uint32_t`): physical qubit ID.
- `out` (`double*`): receives the value.

Returns: 0 with the value in `*out`; -8 when the qubit has no recorded properties or no frequency is set.

### device_get_prob_meas0_prep1(ptr, qubit, out)

Returns P(measure 0 \| prepared 1) recorded for `qubit` (no default fallback).

Returns: 0 with the value in `*out`; -8 when the qubit has no recorded properties or the probability is unset.

### device_get_prob_meas1_prep0(ptr, qubit, out)

Returns P(measure 1 \| prepared 0) recorded for `qubit` (no default fallback).

Returns: 0 with the value in `*out`; -8 when the qubit has no recorded properties or the probability is unset.

### device_get_error_rate(ptr, qubit, index, out)

Returns the error rate of the native instruction at `index` recorded for `qubit`.

Parameters:

- `index` (`uintptr_t`): native-instruction index.
- `out` (`double*`): receives the value.

Returns: 0 with the value in `*out`; -8 when the qubit has no recorded properties or `index` is out of bounds.

### device_get_length(ptr, qubit, index, out)

Returns the duration (ns) of the native instruction at `index` recorded for `qubit`.

Returns: 0 with the value in `*out`; -8 when the qubit has no recorded properties, `index` is out of bounds, or the duration is unset.

### device_set_t1(ptr, qubit, t1)

Sets the T1 relaxation time (μs) recorded for `qubit`.

Returns: 0 on success; -1 on NULL; -8 when the qubit has no recorded properties.

### device_set_t2(ptr, qubit, t2)

Sets the T2 dephasing time (μs) recorded for `qubit`.

Returns: 0 on success; -1 on NULL; -8 when the qubit has no recorded properties.

### device_set_frequency(ptr, qubit, frequency)

Sets the qubit frequency (GHz) recorded for `qubit`.

Returns: 0 on success; -1 on NULL; -8 when the qubit has no recorded properties.

### device_set_prob_meas0_prep1(ptr, qubit, prob)

Sets P(measure 0 \| prepared 1) recorded for `qubit`.

Returns: 0 on success; -1 on NULL; -8 when the qubit has no recorded properties.

### device_set_prob_meas1_prep0(ptr, qubit, prob)

Sets P(measure 1 \| prepared 0) recorded for `qubit`.

Returns: 0 on success; -1 on NULL; -8 when the qubit has no recorded properties.

### device_set_error_rate(ptr, qubit, index, error_rate)

Sets the error rate of the native instruction at `index` recorded for `qubit`.

Returns: 0 on success; -1 on NULL; -8 when the qubit has no recorded properties or `index` is out of bounds.

### device_set_length(ptr, qubit, index, length)

Sets the duration (ns) of the native instruction at `index` recorded for `qubit`.

Returns: 0 on success; -1 on NULL; -8 when the qubit has no recorded properties or `index` is out of bounds.

### device_set_instruction(ptr, qubit, index, gate_name)

Replaces the standard gate carried by the native instruction at `index` recorded for `qubit`. The edited list is revalidated, so changing a single-qubit gate to a two-qubit gate (or vice versa) is rejected.

Parameters:

- `index` (`uintptr_t`): native-instruction index.
- `gate_name` (`const char*`): replacement standard-gate name.

Returns: 0 on success; -1 on NULL; -4 for an unknown gate name; -8 when the qubit has no recorded properties, `index` is out of bounds, or the edited list fails validation.

### device_set_native_instruction(ptr, qubit, gate_name, error_rate, length)

Appends one native single-qubit instruction (standard gate `gate_name` with `error_rate` and duration `length` in ns; NaN `length` = unset) to the recorded properties of `qubit`.

Parameters:

- `gate_name` (`const char*`): standard-gate name.
- `error_rate` (`double`): error rate in [0, 1].
- `length` (`double`): duration in ns; NaN = unset.

Returns: 0 on success; -1 on NULL; -4 for an unknown gate name; -8 when the qubit has no recorded properties or the instruction is rejected (wrong arity or invalid calibration values).

---

## Defaults

### device_default_t1(ptr, out)

Returns the device-wide default T1 time (μs). Returns: 0 with the value in `*out`; -8 when no default is set.

### device_default_t2(ptr, out)

Returns the device-wide default T2 time (μs). Returns: 0 with the value in `*out`; -8 when unset.

### device_default_readout_error(ptr, out)

Returns the device-wide default readout error rate. Returns: 0 with the value in `*out`; -8 when unset.

### device_default_single_qubit_error(ptr, out)

Returns the device-wide default single-qubit gate error rate. Returns: 0 with the value in `*out`; -8 when unset.

### device_default_two_qubit_error(ptr, out)

Returns the device-wide default two-qubit gate error rate. Returns: 0 with the value in `*out`; -8 when unset.

### device_set_default_t1(ptr, t1)

Sets the device-wide default T1 time (μs). Returns: 0 on success; a negative code on error.

### device_set_default_t2(ptr, t2)

Sets the device-wide default T2 time (μs). Returns: 0 on success; a negative code on error.

### device_set_default_readout_error(ptr, error)

Sets the device-wide default readout error rate. Returns: 0 on success; a negative code on error.

### device_set_default_single_qubit_error(ptr, error)

Sets the device-wide default single-qubit gate error rate. Returns: 0 on success; a negative code on error.

### device_set_default_two_qubit_error(ptr, error)

Sets the device-wide default two-qubit gate error rate. Returns: 0 on success; a negative code on error.

---

## Usable qubits

### device_invalid_qubits_len(ptr)

Returns the number of invalid (offline/faulty) qubits; 0 for NULL.

### device_invalid_qubits(ptr, out, len)

Two-step output of the invalid qubit IDs, paired with `device_invalid_qubits_len`; the fill function returns the total count.

### device_set_invalid_qubits(ptr, qubits, len)

Replaces the set of invalid qubits.

Parameters:

- `qubits` (`const uint32_t*`): array of qubit IDs.
- `len` (`uintptr_t`): array length.

Returns: 0 on success; -1 on NULL; -2 when any qubit is not registered with the device (the existing set is then preserved).

### device_is_usable_qubit(ptr, qubit)

Checks whether `qubit` is registered with the device and not marked invalid.

Returns: 1 when usable; 0 otherwise; -1 on NULL.

### device_num_usable_qubits(ptr)

Returns the number of usable qubits; 0 for NULL.

### device_usable_qubits_len(ptr) / device_usable_qubits(ptr, out, len)

Two-step output of the usable qubit IDs; the fill function returns the total count.

### device_qubits_len(ptr) / device_qubits(ptr, out, len)

Two-step output of all registered physical qubit IDs (including invalid ones); the fill function returns the total count.

---

## Qubit and edge properties

### device_qubit_properties(ptr, qubit, out)

Writes the per-qubit property snapshot for `qubit` to `*out`.

Parameters:

- `qubit` (`uint32_t`): physical qubit ID.
- `out` (`struct CQubitProp*`): receives the snapshot.

Returns: 0 on success; -8 when no properties have been recorded for the qubit (optional fields in the snapshot are NaN when unset).

### device_qubit_prop_native_instruction(ptr, qubit, index, out)

Writes the native instruction at `index` for `qubit` to `*out`.

Parameters:

- `qubit` (`uint32_t`): physical qubit ID.
- `index` (`uintptr_t`): instruction index, bounded by `num_native_instructions` from the snapshot.
- `out` (`struct CNativeInstruction*`): receives the snapshot; `name` must be freed with `cqlib_string_free`.

Returns: 0 on success; -8 when the qubit has no recorded properties or `index` is out of bounds.

### device_edge_properties(ptr, control, target, out)

Writes the per-edge property snapshot for the directed coupling `control -> target` to `*out`.

Returns: 0 on success; -8 when no properties have been recorded for the edge.

### device_edge_prop_native_instruction(ptr, control, target, index, out)

Writes the native instruction at `index` for the directed coupling `control -> target` to `*out` (two-qubit standard gates).

Returns: 0 on success; -8 when the edge has no recorded properties or `index` is out of bounds; `name` must be freed with `cqlib_string_free`.

### device_add_qubit_properties(ptr, qubit, input)

Records (or replaces) the properties of `qubit`.

Parameters:

- `qubit` (`uint32_t`): physical qubit ID.
- `input` (`const struct CQubitPropInput*`): property input.

Returns: 0 on success; -1 on NULL; -2 when the qubit is not registered with the device or absent from its topology; -4 for an unknown native gate name; -8 for invalid native-instruction calibration values.

### device_add_edge_properties(ptr, control, target, input)

Records (or replaces) the properties of the directed coupling `control -> target`.

Parameters:

- `input` (`const struct CEdgePropInput*`): property input.

Returns: 0 on success; -1 on NULL; -2 when the directed coupling is not in the device topology; -4 for an unknown gate name; -8 when a native instruction is not a two-qubit standard gate or has invalid calibration values.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* line topology: 0 -> 1 -> 2 */
    struct CDevice *dev = device_line("mock_backend", 3);
    if (dev == NULL) {
        return 1;
    }

    /* device-wide defaults */
    device_set_default_t1(dev, 50.0);
    device_set_default_t2(dev, 35.0);
    device_set_default_readout_error(dev, 0.05);

    /* local properties for qubit 0: readout error 0.02, T1 80, native H */
    struct CQubitPropInput input;
    input.readout_error = 0.02;
    input.t1 = 80.0;
    input.t2 = 70.0;
    input.prob_meas0_prep1 = 0.01;
    input.prob_meas1_prep0 = 0.03;
    input.frequency = 5.0;

    const char *names[1] = {"H"};
    const double rates[1] = {0.001};
    input.native_gate_names = names;
    input.native_error_rates = rates;
    input.native_lengths = NULL;
    input.num_native_instructions = 1;

    int32_t rc = device_add_qubit_properties(dev, 0, &input);  /* 0 */

    /* local properties for edge 0 -> 1: native CX */
    struct CEdgePropInput einput;
    const char *enames[1] = {"CX"};
    const double erates[1] = {0.02};
    einput.native_gate_names = enames;
    einput.native_error_rates = erates;
    einput.native_lengths = NULL;
    einput.num_native_instructions = 1;

    rc = device_add_edge_properties(dev, 0, 1, &einput);  /* 0 */

    /* queries: local first, default fallback */
    double t1_0 = 0.0, t1_2 = 0.0;
    device_get_t1(dev, 0, &t1_0);  /* 0, t1_0 == 80.0 (local) */
    device_get_t1(dev, 2, &t1_2);  /* 0, t1_2 == 50.0 (default) */

    double sq_err = 0.0, tq_err = 0.0, edge_err = 0.0;
    device_single_qubit_error(dev, "H", 0, &sq_err);   /* 0, 0.001 */
    device_two_qubit_error(dev, "CX", 0, 1, &tq_err);  /* 0, 0.02 */
    device_edge_error(dev, 0, 1, &edge_err);            /* 0, 0.02 */

    /* per-qubit property getters (no default fallback) */
    double freq = 0.0, pm01 = 0.0, pm10 = 0.0, rate = 0.0, len = 0.0;
    device_get_frequency(dev, 0, &freq);         /* 0, freq == 5.0 */
    device_get_prob_meas0_prep1(dev, 0, &pm01);  /* 0, pm01 == 0.01 */
    device_get_prob_meas1_prep0(dev, 0, &pm10);  /* 0, pm10 == 0.03 */
    device_get_error_rate(dev, 0, 0, &rate);     /* 0, rate == 0.001 */
    device_get_length(dev, 0, 0, &len);          /* -8: duration unset, len == NaN */

    /* per-qubit property setters: one field per call, others preserved */
    device_set_t1(dev, 0, 85.0);                 /* 0 */
    device_set_frequency(dev, 0, 5.1);           /* 0 */
    device_get_frequency(dev, 0, &freq);         /* 0, freq == 5.1 */
    device_set_error_rate(dev, 0, 0, 0.002);     /* 0 */
    device_set_length(dev, 0, 0, 50.0);          /* 0 */
    device_set_t1(dev, 2, 1.0);                  /* -8: qubit 2 has no recorded properties */

    /* native instruction edits */
    device_set_instruction(dev, 0, 0, "X");                   /* 0: H -> X, calibration kept */
    device_set_native_instruction(dev, 0, "Z", 0.003, 80.0);  /* 0: appended */

    /* property snapshot and native instructions */
    struct CQubitProp prop;
    if (device_qubit_properties(dev, 0, &prop) == 0) {
        for (uintptr_t i = 0; i < prop.num_native_instructions; i++) {
            struct CNativeInstruction ni;
            if (device_qubit_prop_native_instruction(dev, 0, i, &ni) == 0) {
                printf("%s rate=%f\n", ni.name, ni.error_rate);
                cqlib_string_free(ni.name);
            }
        }
    }

    /* invalid qubits */
    uint32_t bad[1] = {2};
    device_set_invalid_qubits(dev, bad, 1);
    int32_t usable = device_is_usable_qubit(dev, 2);     /* 0 */
    uintptr_t n_usable = device_num_usable_qubits(dev); /* 2 */

    /* native gate set */
    rc = device_with_native_gates(dev, "H,CX,RZ");  /* 0 */
    char *gates = device_native_gates(dev);          /* "H,CX,RZ" */
    cqlib_string_free(gates);

    device_free(dev);
    return 0;
}
```
