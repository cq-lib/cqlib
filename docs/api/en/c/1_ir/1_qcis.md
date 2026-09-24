# QCIS (C)

The C binding provides four entry points converting between QCIS text and `CCircuit`. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md).

---

## Interface overview

| Function | Direction |
| --- | --- |
| `qcis_loads` | QCIS string → `CCircuit` |
| `qcis_load` | QCIS file → `CCircuit` |
| `qcis_dumps` | `CCircuit` → QCIS string |
| `qcis_dump` | `CCircuit` → QCIS file |

---

## Parsing

### qcis_loads(source)

Parses a QCIS string into a new `CCircuit*`.

Parameters:

- `source` (`const char*`): the QCIS text, NUL-terminated.

Returns: a newly allocated `CCircuit*` on success (free with `circuit_free`); NULL when `source` is NULL, not valid UTF-8, or parsing fails.

Parse failures include: a qubit not written as `Q<id>`, an unparseable id after `Q`, a gate whose qubit count or parameter count does not match its specification, an empty parameter expression, a gate name outside the QCIS instruction set, or an empty line / no valid content.

### qcis_load(path)

Reads a QCIS file and parses it into a new `CCircuit*`.

Parameters:

- `path` (`const char*`): the file path, NUL-terminated.

Returns: a newly allocated `CCircuit*` on success; NULL when `path` is NULL, not valid UTF-8, reading fails, or parsing fails.

---

## Dumping

### qcis_dumps(circuit)

Dumps a circuit to a QCIS string.

Parameters:

- `circuit` (`const struct CCircuit*`): the circuit to dump.

Returns: a heap-allocated C string on success (free with `cqlib_string_free`); NULL when `circuit` is NULL or dumping fails.

Dump failures include: a gate outside the QCIS native gate set (the standard identity gate, global phase, multi-controlled gates, custom gates, and unitary gates), classical storage, classical control flow, an unevaluable symbolic parameter, or a delay whose tick count is not a non-negative integer. Measurement (`M`) and barrier (`B`) can be dumped.

### qcis_dump(circuit, path)

Dumps a circuit to a QCIS string and writes it to a file.

Parameters:

- `circuit` (`const struct CCircuit*`): the circuit to dump.
- `path` (`const char*`): the target file path.

Returns: 0 on success; -1 when either pointer is NULL; -8 when the path is not valid UTF-8; -5 when dumping or writing fails.

---

## QCIS instruction set

QCIS parsing and dumping support only the native gate set:

- Single-qubit gates: `H`, `S`, `SD`, `T`, `TD`, `X`, `X2P`, `X2M`, `Y`, `Y2P`, `Y2M`, `Z`
- Parameterized single-qubit gates: `RX`, `RXY`, `RY`, `RZ`, `U`, `XY`, `XY2P`, `XY2M`, `PHASE`
- Multi-qubit gates: `RXX`, `RYY`, `RZX`, `RZZ`, `SWAP`, `CX`, `CCX`, `CY`, `CZ`, `CRX`, `CRY`, `CRZ`, `FSIM`

Beyond gates, QCIS text also expresses measurement (`M`), barrier (`B`), and delay (`I Qn t`, where `t` is the number of delay ticks, in units of 0.5 ns, and must be a non-negative integer).

When parsing, `SDG`, `TDG`, and `Barrier` are accepted as alternative spellings of `SD`, `TD`, and `B`; dumping always writes `SD`, `TD`, and `B`. The standard identity gate and global phase are not part of the native gate set, and QCIS `I` always denotes a delay. Decompose gates that cannot be expressed into the QCIS gate set before dumping.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

/* Parsing: qubits are written Q<id>; parameters accept expressions */
const char *src =
    "RX Q0 1.0\n"
    "RY Q1 pi/2\n"
    "X Q0\n"
    "CZ Q0 Q1\n"
    "M Q0 Q1\n";

struct CCircuit *c = qcis_loads(src);
if (c == NULL) {
    /* parse failure */
}

uintptr_t n = circuit_num_qubits(c);  /* 2 */

/* Dump back to text */
char *text = qcis_dumps(c);
if (text != NULL) {
    printf("%s", text);
    cqlib_string_free(text);
}

/* Write to a file */
int32_t rc = qcis_dump(c, "out.qcis");  /* 0 on success, -5 on write failure */

circuit_free(c);
```
