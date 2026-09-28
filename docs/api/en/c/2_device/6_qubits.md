# Qubit Identifiers (C)

This page covers the C ABI of the strongly typed device-facing qubit identifiers `CLogicalQubit` and `CPhysicalQubit`: the former marks a circuit wire, the latter a hardware position on a device. Both types share the same numeric representation (a `uint32_t` ID) but stay distinct at the type level; each converts only to and from `CQubit`, the two never convert into each other directly, and equal numeric IDs do not make them the same identifier. Error-code and string-release conventions follow the [Overview](../0_overview.md).

---

## Structs

Both types are passed by value and need no release:

```c
typedef struct CLogicalQubit {
  uint32_t id;
} CLogicalQubit;

typedef struct CPhysicalQubit {
  uint32_t id;
} CPhysicalQubit;
```

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `uint32_t` | Numeric identifier. |

---

## CLogicalQubit

Logical qubit identifier used for circuit wires.

### logical_qubit_new(id)

```c
struct CLogicalQubit logical_qubit_new(uint32_t id);
```

Creates the identifier from its numeric ID, returned by value.

- `id` (`uint32_t`): numeric identifier.

### logical_qubit_from_qubit(qubit)

```c
struct CLogicalQubit logical_qubit_from_qubit(struct CQubit qubit);
```

Wraps an existing circuit qubit as a logical identifier.

- `qubit` (`struct CQubit`): circuit qubit identifier.

### logical_qubit_qubit(qubit)

```c
struct CQubit logical_qubit_qubit(struct CLogicalQubit qubit);
```

Returns the underlying circuit qubit identifier.

- `qubit` (`struct CLogicalQubit`): logical identifier.

### logical_qubit_id(qubit)

```c
uint32_t logical_qubit_id(struct CLogicalQubit qubit);
```

Returns the numeric identifier.

- `qubit` (`struct CLogicalQubit`): logical identifier.

### logical_qubit_equal(a, b) / logical_qubit_compare(a, b)

```c
int32_t logical_qubit_equal(struct CLogicalQubit a, struct CLogicalQubit b);
int32_t logical_qubit_compare(struct CLogicalQubit a, struct CLogicalQubit b);
```

Comparison and ordering use the numeric identifier: `equal` returns `1` (same) or `0` (different); `compare` returns `-1` (`a < b`), `0` (equal) or `1` (`a > b`).

- `a`, `b` (`struct CLogicalQubit`): identifiers to compare.

### logical_qubit_to_string(qubit)

```c
char *logical_qubit_to_string(struct CLogicalQubit qubit);
```

Formats the identifier as `L<id>` (e.g. `L3`). Release the returned string with [cqlib_string_free](../0_overview.md); returns NULL on allocation failure.

- `qubit` (`struct CLogicalQubit`): logical identifier.

---

## CPhysicalQubit

Physical qubit identifier used for hardware positions.

### physical_qubit_new(id)

```c
struct CPhysicalQubit physical_qubit_new(uint32_t id);
```

Creates the identifier from its numeric ID, returned by value.

- `id` (`uint32_t`): numeric identifier.

### physical_qubit_from_qubit(qubit)

```c
struct CPhysicalQubit physical_qubit_from_qubit(struct CQubit qubit);
```

Wraps an existing qubit-shaped identifier as a physical hardware ID.

- `qubit` (`struct CQubit`): qubit-shaped identifier.

### physical_qubit_qubit(qubit)

```c
struct CQubit physical_qubit_qubit(struct CPhysicalQubit qubit);
```

Returns the underlying qubit-shaped identifier.

- `qubit` (`struct CPhysicalQubit`): physical identifier.

### physical_qubit_id(qubit)

```c
uint32_t physical_qubit_id(struct CPhysicalQubit qubit);
```

Returns the numeric identifier.

- `qubit` (`struct CPhysicalQubit`): physical identifier.

### physical_qubit_equal(a, b) / physical_qubit_compare(a, b)

```c
int32_t physical_qubit_equal(struct CPhysicalQubit a, struct CPhysicalQubit b);
int32_t physical_qubit_compare(struct CPhysicalQubit a, struct CPhysicalQubit b);
```

Comparison and ordering use the numeric identifier: `equal` returns `1` (same) or `0` (different); `compare` returns `-1` (`a < b`), `0` (equal) or `1` (`a > b`).

- `a`, `b` (`struct CPhysicalQubit`): identifiers to compare.

### physical_qubit_to_string(qubit)

```c
char *physical_qubit_to_string(struct CPhysicalQubit qubit);
```

Formats the identifier as `P<id>` (e.g. `P11`). Release the returned string with [cqlib_string_free](../0_overview.md); returns NULL on allocation failure.

- `qubit` (`struct CPhysicalQubit`): physical identifier.

---

## Error codes

None of the interfaces on this page return error codes: constructors, accessors and comparisons never fail; `*_to_string` returns NULL only on allocation failure.

---

## Example

```c
#include <cqlib_c.h>
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    /* A circuit qubit wraps as a logical identifier and unwraps back */
    CLogicalQubit logical = logical_qubit_from_qubit(qubit_new(3));
    assert(logical_qubit_id(logical) == 3u);
    assert(qubit_id(logical_qubit_qubit(logical)) == 3u);

    /* Physical identifiers are constructed independently */
    CPhysicalQubit physical = physical_qubit_new(100);
    assert(physical_qubit_id(physical) == 100u);

    /* Comparison */
    assert(logical_qubit_compare(logical_qubit_new(0), logical_qubit_new(2)) == -1);
    assert(physical_qubit_compare(physical, physical_qubit_new(101)) == -1);

    /* String representation */
    char *ltext = logical_qubit_to_string(logical);
    assert(strcmp(ltext, "L3") == 0);
    cqlib_string_free(ltext);

    char *ptext = physical_qubit_to_string(physical);
    assert(strcmp(ptext, "P100") == 0);
    cqlib_string_free(ptext);

    /* Layout scenario: register and query the logical 1 -> physical 102 mapping */
    const uint32_t pairs[2] = {1u, 102u};
    CLayout *layout = layout_from_pairs(pairs, 1, 3);
    assert(layout != NULL);
    assert(layout_get(layout, 1) == physical_qubit_id(physical_qubit_new(102)));
    layout_free(layout);
    return 0;
}
```

---

## Related pages

- [Qubit](../0_circuit/2_qubit.md): the `CQubit` circuit identifier, the conversion hub for both types.
- [Layout](3_layout.md): binding and querying the logical-to-physical mapping.
- [Overview](../0_overview.md): error codes, memory ownership and global conventions.
