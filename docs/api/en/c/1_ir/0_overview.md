# Intermediate Representation (C)

The C binding's IR interface converts between `CCircuit` and three circuit text formats: QCIS, OpenQASM 2.0, and OpenQASM 3.0. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md).

---

## The four-function convention

Each format exposes the same four entry points, with the `<prefix>` being `qcis`, `qasm2`, or `qasm3`:

| Action | Function | Returns |
| --- | --- | --- |
| string → `CCircuit` | `<prefix>_loads(source)` | a newly allocated `CCircuit*` on success, NULL on error |
| file → `CCircuit` | `<prefix>_load(path)` | a newly allocated `CCircuit*` on success, NULL on error |
| `CCircuit` → string | `<prefix>_dumps(circuit)` | a heap-allocated C string on success, NULL on error |
| `CCircuit` → file | `<prefix>_dump(circuit, path)` | 0 on success, a negative error code on failure |

- Circuits returned by `loads` / `load` are released with `circuit_free` (see [Circuit](../0_circuit/1_circuit.md)).
- Strings returned by `dumps` are released with `cqlib_string_free`.
- `dump` returns an `int32_t` error code: -1 for NULL pointers, -8 when the path is not valid UTF-8, and -5 when dumping or writing fails.
- Parsing never modifies its input; `loads(dumps(circuit))` yields a circuit whose operation sequence is equivalent to the original.
- A circuit can be read with one format and written with another to convert between formats, for example reading OpenQASM 2.0 text and writing QCIS text.

---

## Pages

| Page | Contents |
| --- | --- |
| [QCIS](1_qcis.md) | `qcis_loads` / `qcis_load` / `qcis_dumps` / `qcis_dump` and the QCIS native gate set. |
| [OpenQASM 2.0](2_qasm2.md) | `qasm2_loads` / `qasm2_load` / `qasm2_dumps` / `qasm2_dump` and the supported 2.0 subset. |
| [OpenQASM 3.0](3_qasm3.md) | `qasm3_loads` / `qasm3_load` / `qasm3_dumps` / `qasm3_dump` and the supported 3.0 subset. |

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

/* string -> circuit -> string */
struct CCircuit *c = qcis_loads("H Q0\nCZ Q0 Q1\n");
if (c == NULL) {
    /* parse failure */
}

char *text = qcis_dumps(c);
if (text != NULL) {
    printf("%s", text);       /* "H Q0\nCZ Q0 Q1\n" */
    cqlib_string_free(text);
}

circuit_free(c);

/* format conversion: QCIS text written as OpenQASM 2.0 text */
struct CCircuit *c2 = qcis_loads("X Q0\nY Q1\n");
char *qasm = qasm2_dumps(c2);  /* contains the "OPENQASM 2.0;" header */
cqlib_string_free(qasm);
circuit_free(c2);
```
