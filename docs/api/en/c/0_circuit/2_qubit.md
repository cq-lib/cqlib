# Qubit (C)

This page covers the C ABI of the `CQubit` logical-qubit identifier. `CQubit` is a lightweight handle passed by value; it wraps a `uint32_t` ID used to identify a logical qubit stably across circuits, compilation IR, mapping tables and set structures. Error-code and string-release conventions follow the [Overview](../0_overview.md).

---

## Struct

`CQubit` is passed by value and needs no release:

```c
typedef struct CQubit {
  uint32_t id;
} CQubit;
```

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `uint32_t` | Numeric identifier of the logical qubit. |

The ID is a logical identity: it carries no circuit identity and does not correspond to a storage position in a state vector or matrix; the actual order is determined by the qubit list kept by `Circuit` and the ordering conventions of the conversion interfaces. IDs may be sparse (e.g. 10, 20); callers must not assume the value is a valid index into dense storage.

---

## Creation and access

### qubit_new(id)

```c
struct CQubit qubit_new(uint32_t id);
```

Creates a logical qubit with the supplied numeric identifier, returned by value.

- `id` (`uint32_t`): logical qubit identifier.

### qubit_id(qubit)

```c
uint32_t qubit_id(struct CQubit qubit);
```

Returns the raw underlying identifier (compact 4-byte representation).

- `qubit` (`struct CQubit`): the logical qubit.

### qubit_index(qubit)

```c
uintptr_t qubit_index(struct CQubit qubit);
```

Returns the numeric identifier as a `uintptr_t` for index-style use. The value does not map the qubit to any position in a circuit, matrix or simulator.

---

## Checked integer conversions

### qubit_try_from_i64(value, out)

```c
int32_t qubit_try_from_i64(int64_t value, struct CQubit *out);
```

Checked conversion from a signed integer: negative or out-of-range values return an error instead of being silently truncated.

- `value` (`int64_t`): input integer.
- `out` (`struct CQubit*`): output qubit.

Returns: `0` on success; `-1` when `out` is NULL; `-8` when `value` is negative or outside the `uint32_t` range.

### qubit_try_from_u64(value, out)

```c
int32_t qubit_try_from_u64(uint64_t value, struct CQubit *out);
```

Checked conversion from an unsigned integer: values outside the `uint32_t` range return an error.

- `value` (`uint64_t`): input integer.
- `out` (`struct CQubit*`): output qubit.

Returns: `0` on success; `-1` when `out` is NULL; `-8` when `value` is outside the `uint32_t` range.

---

## Comparison and ordering

Comparison, ordering and hashing are all based on the numeric identifier: two `CQubit` values with the same ID are the same logical qubit.

### qubit_equal(a, b)

```c
int32_t qubit_equal(struct CQubit a, struct CQubit b);
```

Returns `1` when both carry the same ID, `0` otherwise.

- `a`, `b` (`struct CQubit`): qubits to compare.

### qubit_compare(a, b)

```c
int32_t qubit_compare(struct CQubit a, struct CQubit b);
```

Compares by numeric identifier: `-1` when `a < b`, `0` when equal, `1` when `a > b`.

- `a`, `b` (`struct CQubit`): qubits to compare.

---

## String representation

### qubit_to_string(qubit)

```c
char *qubit_to_string(struct CQubit qubit);
```

Formats the identifier as `Q<id>` (e.g. `Q12`). Release the returned string with [cqlib_string_free](../0_overview.md); returns NULL on allocation failure.

- `qubit` (`struct CQubit`): the logical qubit.

---

## Error codes

| Function | `0` | `-1` | `-8` |
| --- | --- | --- | --- |
| `qubit_try_from_i64` | converted | `out` is NULL | value negative or outside the `uint32_t` range |
| `qubit_try_from_u64` | converted | `out` is NULL | value outside the `uint32_t` range |

`qubit_new`, `qubit_id`, `qubit_index`, `qubit_equal` and `qubit_compare` never fail; `qubit_to_string` returns NULL only on allocation failure.

---

## Example

```c
#include <cqlib_c.h>
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    /* Creation and access */
    CQubit q = qubit_new(12);
    assert(q.id == 12);
    assert(qubit_id(q) == 12u);
    assert(qubit_index(q) == 12u);

    /* Checked conversions: negative and out-of-range values error out */
    CQubit out = {0};
    assert(qubit_try_from_i64(-1, &out) == -8);
    assert(qubit_try_from_i64(3, &out) == 0);
    assert(qubit_id(out) == 3u);

    /* Comparison */
    assert(qubit_equal(q, qubit_new(12)) == 1);
    assert(qubit_compare(qubit_new(0), qubit_new(1)) == -1);

    /* String representation */
    char *text = qubit_to_string(q);
    assert(strcmp(text, "Q12") == 0);
    cqlib_string_free(text);

    /* Interop with circuits: sparse logical IDs 10 and 20 */
    const uint32_t ids[2] = {qubit_id(qubit_new(10)), qubit_id(qubit_new(20))};
    CCircuit *qc = circuit_from_qubits(ids, 2);
    assert(circuit_num_qubits(qc) == 2);
    circuit_free(qc);
    return 0;
}
```

---

## Related pages

- [Circuit](1_circuit.md): `circuit_from_qubits` accepts explicit ID lists (sparse IDs included).
- [Qubit identifiers](../2_device/6_qubits.md): device-side `CLogicalQubit` / `CPhysicalQubit`.
- [Overview](../0_overview.md): error codes, memory ownership and global conventions.
