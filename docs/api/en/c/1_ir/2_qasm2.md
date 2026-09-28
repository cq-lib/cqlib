# OpenQASM 2.0 (C)

The C binding provides four entry points converting between OpenQASM 2.0 text and `CCircuit`. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md).

---

## Interface overview

| Function | Direction |
| --- | --- |
| `qasm2_loads` | OpenQASM 2.0 string → `CCircuit` |
| `qasm2_load` | OpenQASM 2.0 file → `CCircuit` |
| `qasm2_dumps` | `CCircuit` → OpenQASM 2.0 string |
| `qasm2_dump` | `CCircuit` → OpenQASM 2.0 file |

---

## Parsing

### qasm2_loads(source)

Parses an OpenQASM 2.0 string into a new `CCircuit*`.

Parameters:

- `source` (`const char*`): the OpenQASM 2.0 text, NUL-terminated.

Returns: a newly allocated `CCircuit*` on success (free with `circuit_free`); NULL when `source` is NULL, not valid UTF-8, or parsing fails.

Parse failures include: syntax errors, AST-to-circuit conversion failures, references to undefined quantum or classical registers, references to undefined gates, redefinition of a gate name declared by qelib1, mismatched qubit or parameter counts, gate expansion exceeding the recursion limit or forming a cycle, parameter expression evaluation failures, a version declaration other than 2.0, duplicate declarations, `include` cycles, and an `opaque` gate being called.

External files brought in via `include` are not resolved by `qasm2_loads`; the file entry point `qasm2_load` resolves relative includes against the input file's directory.

### qasm2_load(path)

Reads an OpenQASM 2.0 file and parses it into a new `CCircuit*`.

Parameters:

- `path` (`const char*`): the file path, NUL-terminated.

Returns: a newly allocated `CCircuit*` on success; NULL when `path` is NULL, not valid UTF-8, reading fails, or parsing fails.

---

## Dumping

### qasm2_dumps(circuit)

Dumps a circuit to an OpenQASM 2.0 string. The output contains an auto-generated comment, the `OPENQASM 2.0;` header, `include "qelib1.inc";`, and `qreg`/`creg` declarations.

Parameters:

- `circuit` (`const struct CCircuit*`): the circuit to dump.

Returns: a heap-allocated C string on success (free with `cqlib_string_free`); NULL when `circuit` is NULL or dumping fails.

Dump failures include: content generation failures, a measurement or reset inside a `gate` definition body, an invalid qubit index, a classical type that cannot be represented as `creg`, classical data operations or classical control that cannot be expressed, and distinct custom gates sharing one OpenQASM gate name.

### qasm2_dump(circuit, path)

Dumps a circuit to an OpenQASM 2.0 string and writes it to a file.

Parameters:

- `circuit` (`const struct CCircuit*`): the circuit to dump.
- `path` (`const char*`): the target file path.

Returns: 0 on success; -1 when either pointer is NULL; -8 when the path is not valid UTF-8; -5 when dumping or writing fails.

---

## Supported OpenQASM 2.0 subset

- Standard and parameterized gates, including `ch`/`cu1`/`cu3` and `sx`/`sxdg`/`crx`/`cry`/`crz`/`rxx`/`ryy`/`rzz`/`fsim`
- Measurement, barrier, and reset statements
- `qreg`, `creg`, and `opaque` declarations, plus `if (creg == integer) qop;` conditionals
- `CircuitGate` dumped as a gate definition; matrix-only unitary gates dumped as `opaque` declarations
- Extension gates not covered directly are emitted as `gate` definitions (e.g. `crx`/`cry`/`rzz`/`rxx`/`ryy`/`rzx`)
- `include "qelib1.inc";` is handled as a built-in include; the file need not exist locally

Dumping trades against the expressiveness of OpenQASM 2.0; the following structures fail:

- `else` branches in classical control, loops, `switch`, and nested control flow
- Classical types that cannot be represented as `creg`, such as `bool` and unsigned integers
- Measurement and reset inside `gate` definition bodies
- Bit-wise conditions (e.g. `if (c[0] == 1)`) and conditional barriers

`rzx` appears only in the dumped extension gate definitions and is not accepted when loading; global phase is not written as a statement but only as a comment, and is not restored when reloading.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

const char *src =
    "OPENQASM 2.0;\n"
    "include \"qelib1.inc\";\n"
    "qreg q[2];\n"
    "h q[0];\n"
    "cx q[0],q[1];\n";

struct CCircuit *c = qasm2_loads(src);
if (c == NULL) {
    /* parse failure */
}

char *out = qasm2_dumps(c);
if (out != NULL) {
    printf("%s", out);  /* contains "OPENQASM 2.0;" */
    cqlib_string_free(out);
}

int32_t rc = qasm2_dump(c, "out.qasm");  /* 0 on success */

circuit_free(c);
```
