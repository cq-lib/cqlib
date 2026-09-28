# OpenQASM 3.0 (C)

The C binding provides four entry points converting between OpenQASM 3.0 text and `CCircuit`. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md).

---

## Interface overview

| Function | Direction |
| --- | --- |
| `qasm3_loads` | OpenQASM 3.0 string → `CCircuit` |
| `qasm3_load` | OpenQASM 3.0 file → `CCircuit` |
| `qasm3_dumps` | `CCircuit` → OpenQASM 3.0 string |
| `qasm3_dump` | `CCircuit` → OpenQASM 3.0 file |

---

## Parsing

### qasm3_loads(source)

Parses an OpenQASM 3.0 string into a new `CCircuit*`.

Parameters:

- `source` (`const char*`): the OpenQASM 3.0 text, NUL-terminated.

Returns: a newly allocated `CCircuit*` on success (free with `circuit_free`); NULL when `source` is NULL, not valid UTF-8, or parsing fails.

Parse failures include: syntax errors, semantic errors, conversion failures, unsupported features, undefined symbols or gates, type errors, mismatched parameter or qubit counts, and gate expansion exceeding the recursion limit or forming a cycle.

### qasm3_load(path)

Reads an OpenQASM 3.0 file and parses it into a new `CCircuit*`. Relative `include` paths are resolved against the input file's directory.

Parameters:

- `path` (`const char*`): the file path, NUL-terminated.

Returns: a newly allocated `CCircuit*` on success; NULL when `path` is NULL, not valid UTF-8, reading fails, or parsing fails.

---

## Dumping

### qasm3_dumps(circuit)

Dumps a circuit to an OpenQASM 3.0 string. The output is, in order: `OPENQASM 3.0;`, `include "stdgates.inc";`, extension gate definitions, custom gate definitions, quantum and classical declarations, and the main circuit body.

Parameters:

- `circuit` (`const struct CCircuit*`): the circuit to dump.

Returns: a heap-allocated C string on success (free with `cqlib_string_free`); NULL when `circuit` is NULL or dumping fails.

Dump failures include: instructions that cannot be represented (such as `delay`, `XY`, `RXY`), classical storage that cannot be represented, control flow that cannot be represented (such as `while`, `for`, `break` / `continue`), a measurement inside a gate definition body, and signature conflicts between same-named gate definitions.

### qasm3_dump(circuit, path)

Dumps a circuit to an OpenQASM 3.0 string and writes it to a file.

Parameters:

- `circuit` (`const struct CCircuit*`): the circuit to dump.
- `path` (`const char*`): the target file path.

Returns: 0 on success; -1 when either pointer is NULL; -8 when the path is not valid UTF-8; -5 when dumping or writing fails.

---

## Supported OpenQASM 3.0 subset

- The `OPENQASM 3;` and `OPENQASM 3.0;` headers, plus `include "stdgates.inc";` (a built-in standard library; no file on disk is required)
- Scalar qubit and one-dimensional quantum register declarations
- Classical declarations `bit`, `bit[n]`, `bool`, `uint[n]`, plus angle and float `input` declarations
- Standard and extension gates that map onto the standard gates
- Custom gate definitions and calls
- Measurement, `reset`, `barrier`, and global phase
- Control flow: `if` / `else`, static `for`, constant-branch `switch`

The following features produce an explicit error rather than partial degradation:

- Timing and pulses: `delay`, `box`, `cal`, `defcal`
- Subprograms and externals: `def` subprograms, `extern`
- Hardware-related: hardware qubits, `output` declarations, and `input` declarations other than angle and float
- Other statements: `alias`, `pragma`, annotations, legacy declarations
- Gate modifiers: `negctrl`, `pow`; `ctrl` and `inv` only work when the target gate has a matching implementation
- Some standard gates: `ch`, `cp`, `cu`, `cswap`
- Control flow: `while` over a runtime condition is rejected by semantic checks; `for` only supports static expansion of constant ranges
- Complex classical arithmetic, complex lvalue slices, multi-dimensional indexing, and in-scope qubit declarations

Dumping has further constraints: `delay`, matrix-only unitary gates, plain assignments to scalar `bit` variables, measurements inside gate definition bodies, `while` / `for` / `break` / `continue` control flow, and the `XY` and `RXY` standard gates are not dumped.

---

## Round-trip behavior

Dump output can be loaded again. Normalization rules:

- The header is always `OPENQASM 3.0;` and `include "stdgates.inc";`, regardless of the input spelling.
- Top-level qubits are dumped uniformly as `qubit[n] q`; the single-qubit case is also written `qubit[1] q`.
- User classical variables are named `c0`, `c1`, ...; immutable measured values are named `v0`, `v1`, ...
- A measurement immediately feeding a compatible store collapses into an assignment such as `c0 = measure q;`.
- Stand-alone measurements go into an auto-generated `bit[k] meas` register, e.g. `meas[0] = measure q[0];`.
- Non-contiguous or reordered register measurements split into per-bit assignments such as `c0[0] = measure q[2];`.
- Gates already provided by `stdgates.inc` are called directly; extension gates get one `gate` definition before the main body and keep their names at call sites.
- Global phase is dumped as a `gphase(...);` statement and re-enters the circuit's global phase on reload.
- Auto-generated names avoid gate and register names already present in the circuit.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

const char *src =
    "OPENQASM 3;\n"
    "include \"stdgates.inc\";\n"
    "qubit[2] q;\n"
    "h q[0];\n"
    "cx q[0], q[1];\n";

struct CCircuit *c = qasm3_loads(src);
if (c == NULL) {
    /* parse failure */
}

char *out = qasm3_dumps(c);
if (out != NULL) {
    printf("%s", out);  /* header normalized to "OPENQASM 3.0;" */
    cqlib_string_free(out);
}

int32_t rc = qasm3_dump(c, "out.qasm");  /* 0 on success */

circuit_free(c);
```
