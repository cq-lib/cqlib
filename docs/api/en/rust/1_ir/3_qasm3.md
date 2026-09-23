# OpenQASM 3.0 API

`cqlib_core::ir::qasm3` provides OpenQASM 3.0 import and export interfaces for converting between `Circuit` and OpenQASM 3.0 text or files.

## Import

```rust
use cqlib_core::ir::{qasm3_load, qasm3_loads, qasm3_dump, qasm3_dumps};
```

## Interface overview

- `qasm3_load(path: impl AsRef<Path>) -> Result<Circuit, Qasm3ParseError>`
- `qasm3_loads(source: &str) -> Result<Circuit, Qasm3ParseError>`
- `qasm3_dump(circuit: &Circuit, path: impl AsRef<Path>) -> Result<(), Qasm3DumpError>`
- `qasm3_dumps(circuit: &Circuit) -> Result<String, Qasm3DumpError>`

The `qasm3` module additionally provides aliases with identical behavior: `from_path`, `from_str`, `to_path`, `to_string`.

## Parsing (load / loads)

### `qasm3_loads`

Parse a circuit from an OpenQASM 3.0 string.

### `qasm3_load`

Read from a file and parse a circuit. A relative-path `include` is resolved from the directory of the input file.

Both return `Qasm3ParseError` on failure; the common variants:

- `IoError`
- `ParseError`
- `SemanticError`
- `ConversionError`
- `UnsupportedFeature`
- `UndefinedSymbol`
- `UndefinedGate`
- `TypeError`
- `InvalidArgument`
- `MismatchedQubitCount`
- `MismatchedParameterCount`
- `RecursionLimitExceeded`
- `CircularGateDependency`

Among them, `IoError` retains the underlying I/O error, which can be read through `source()`.

## Dumping (dump / dumps)

### `qasm3_dumps`

Dump a circuit to an OpenQASM 3.0 string. The output consists, in order, of `OPENQASM 3.0;`, `include "stdgates.inc";`, extension gate definitions, custom gate definitions, quantum and classical declarations, and the operations of the main circuit.

### `qasm3_dump`

Write the dump result to a file, returning `Result<(), Qasm3DumpError>`.

Both return `Qasm3DumpError` on failure; the common variants:

- `IoError`: file read or write failure
- `FormatError`: formatting failure
- `UnsupportedInstruction`: instructions that cannot be represented, such as `delay`, `XY`, `RXY`
- `UnsupportedClassicalData`: classical storage that cannot be represented
- `UnsupportedClassicalControl`: control flow that cannot be represented, such as `while`, `for`, `break` / `continue`
- `MeasureInGateNotAllowed`: measurement is contained in a gate definition body
- `ConflictingGateDefinition`: conflicting signatures of gate definitions with the same name

## Supported features (overview)

According to the current implementation, OpenQASM 3.0 import and export support:

- The header `OPENQASM 3;` and `OPENQASM 3.0;`, and `include "stdgates.inc";` (a built-in standard library is used; no file on disk is required)
- Scalar qubit and one-dimensional qubit register declarations
- Classical declarations `bit`, `bit[n]`, `bool`, `uint[n]`, and angle and float input declarations
- Standard gates mapped to `StandardGate` and Cqlib extension gates
- Custom gate definitions and calls
- Measurement, `reset`, `barrier`, global phase
- Control flow `if` / `else`, static `for`, constant-branch `switch`

## Differences from the OpenQASM 2.0 interface

Built on the same `Circuit` representation, the main differences between this interface and the OpenQASM 2.0 interface:

| Aspect | OpenQASM 2.0 interface | OpenQASM 3.0 interface |
|---|---|---|
| Header and standard library | `OPENQASM 2.0;`, `include "qelib1.inc";` | `OPENQASM 3;` or `OPENQASM 3.0;`, `include "stdgates.inc";` |
| Quantum declarations | `qreg q[2];` | `qubit[2] q;`, and a scalar `qubit q;` is supported |
| Classical declarations | `creg c[2];` | `bit`, `bit[n]`, `bool`, `uint[n]` |
| Measurement | `measure q[0] -> c[0];` | `c[0] = measure q[0];`, `c = measure q;` |
| Control flow | `if (c == 1) ...` followed by a single statement | `if` / `else` blocks, static `for`, constant-branch `switch` |
| Global phase | Given only as a comment when dumping; cannot be loaded | `gphase(theta);`, can be loaded and dumped |
| Input declarations | None | Angle and float `input` declarations |

## Limitations

The following OpenQASM 3.0 features are not supported in the current implementation; a clear error is given when they are encountered, with no partial lowering:

- Timing and pulses: `delay`, `box`, `cal`, `defcal`
- Subroutines and external calls: `def` subroutines, `extern`
- Hardware-related: hardware qubits, `output` declarations, and `input` declarations other than angle and float
- Other statements: `alias`, `pragma`, annotations, legacy declarations
- Gate modifiers: `negctrl`, `pow`; `ctrl` and `inv` are available only when a corresponding implementation exists for the target gate
- Some standard gates: `ch`, `cp`, `cu`, `cswap`
- Control flow: a lowering path for `while` exists in the implementation, but a loop with a runtime condition is currently rejected by the semantic check; `for` supports only the statically expanded form with a constant range
- Complex classical arithmetic, complex lvalue slicing, multi-dimensional indexing
- Qubit declarations inside a scope

The dumping side has additional constraints:

- `delay`, matrix-only `UnitaryGate`, an ordinary assignment that writes to a scalar `bit` variable, and measurement inside a gate definition body are not dumped
- Control flow such as `while`, `for`, `break` and `continue` is not dumped
- The two standard gates `XY` and `RXY` are not dumped

## Round-trip behavior

The dump result can be loaded again by this interface. Canonicalization rules:

- The header is fixed to `OPENQASM 3.0;` and `include "stdgates.inc";`, regardless of how the header was written in the input.
- Top-level qubits are uniformly dumped as `qubit[n] q`; the single-qubit case is likewise written as `qubit[1] q`.
- User classical variables are named `c0`, `c1`, … and immutable measurement values are named `v0`, `v1`, …
- When a measurement is immediately followed by a compatible store, it is folded into the assignment form, for example `c0 = measure q;`.
- A standalone measurement is written into an auto-generated `bit[k] meas` register, for example `meas[0] = measure q[0];`.
- A non-contiguous or reordered register measurement is split into bit-wise assignments, for example `c0[0] = measure q[2];`.
- Gates already provided by `stdgates.inc` are called directly; an extension gate generates a `gate` definition once before the main circuit, and its name is still used at the call site.
- Global phase is dumped as a `gphase(...);` statement and enters the global phase of the circuit after reloading.
- Auto-generated names avoid the gate names and register names already present in the circuit.

## Minimal example

```rust
use cqlib_core::ir::{qasm3_dumps, qasm3_loads};

let qasm = r#"
OPENQASM 3;
include "stdgates.inc";
qubit[2] q;
h q[0];
cx q[0], q[1];
"#;

let c = qasm3_loads(qasm).unwrap();
let out = qasm3_dumps(&c).unwrap();
assert!(out.contains("OPENQASM 3.0;"));
```

Extension gate definitions and round trip:

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::ir::{qasm3_dumps, qasm3_loads};

let q0 = Qubit::new(0);
let q1 = Qubit::new(1);
let mut circuit = Circuit::new(2);
circuit.x2p(q0).unwrap();
circuit.rzz(q0, q1, 0.25).unwrap();

let qasm = qasm3_dumps(&circuit).unwrap();
assert!(qasm.contains("gate x2p q { rx(pi/2) q; }"));

let round_trip = qasm3_loads(&qasm).unwrap();
assert_eq!(round_trip.operations().len(), 2);
```
