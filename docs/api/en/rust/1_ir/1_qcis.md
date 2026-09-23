# QCIS API

`cqlib_core::ir` provides QCIS import and export interfaces for converting between `Circuit` and QCIS text or files.

## Import

```rust
use cqlib_core::ir::{qcis_load, qcis_loads, qcis_dump, qcis_dumps};
```

## Interface overview

- `qcis_load(path) -> Result<Circuit, QcisParseError>`
- `qcis_loads(qcis: &str) -> Result<Circuit, QcisParseError>`
- `qcis_dump(circuit: &Circuit, path) -> Result<(), QcisDumpError>`
- `qcis_dumps(circuit: &Circuit) -> Result<String, QcisDumpError>`

The path parameter of the file entry points is generic over `P: AsRef<Path>`; `&str`, `PathBuf` and `&Path` can all be passed directly.

### Entry points inside the format module

The `load` and `dump` submodules under `cqlib_core::ir::qcis` forward the functions of the same name and re-export a set of equivalent Rust-style entry points at module level:

- `cqlib_core::ir::qcis::loads` / `cqlib_core::ir::qcis::from_str`
- `cqlib_core::ir::qcis::load` / `cqlib_core::ir::qcis::from_path`
- `cqlib_core::ir::qcis::dumps` / `cqlib_core::ir::qcis::to_string`
- `cqlib_core::ir::qcis::dump` / `cqlib_core::ir::qcis::to_path`

## Parsing (load / loads)

### `qcis_loads`

Parse a `Circuit` from a QCIS string; returns `QcisParseError` on failure.

Error types:

- `IoError`: filesystem or I/O failure, never produced by the string entry point.
- `InvalidQubitFormat`: the qubit notation is not `Q<id>`.
- `InvalidQubitId`: the number after `Q` cannot be parsed.
- `QubitCountMismatch`: the number of qubits of a gate does not match the specification.
- `ParameterCountMismatch`: the number of parameters of a gate does not match the specification.
- `MissingParameter`: the parameter expression is empty or cannot be parsed.
- `InvalidParameter`: the delay tick is not a fixed non-negative integer.
- `UnknownGate`: the gate name is not in the QCIS instruction set.
- `EmptyLine`: empty line or no valid content.

### `qcis_load`

Read from a file and parse, returning `Result<Circuit, QcisParseError>`: a file read failure is `IoError`, and a content parse failure is the corresponding variant.

## Dumping (dump / dumps)

### `qcis_dumps`

Dump a circuit to a QCIS string; returns `QcisDumpError` on failure.

Error types:

- `UnsupportedGate`: the circuit contains gates outside the QCIS native gate set, for example the standard identity gate, global phase, multi-controlled gates, custom gates and unitary gates.
- `UnsupportedClassicalData`: classical storage (`store`) cannot be expressed.
- `UnsupportedClassicalControl`: classical control flow cannot be expressed.
- `SymbolicParameter`: the parameter index does not resolve to a corresponding parameter, or the delay tick cannot be evaluated.
- `InvalidDelayParameter`: the tick of a delay instruction is not a non-negative integer.
- `IoError`: failed to write to the buffer.

`M` (measurement) and `B` (barrier) can be dumped; `Reset` cannot.

### `qcis_dump`

Dump and write to a file, returning `Result<(), QcisDumpError>`; a file write failure is `IoError`.

## QCIS native gate restrictions

QCIS parsing and dumping support only the native gate set:

- Single-qubit gates: `H`, `S`, `SD`, `T`, `TD`, `X`, `X2P`, `X2M`, `Y`, `Y2P`, `Y2M`, `Z`
- Parameterized single-qubit gates: `RX`, `RXY`, `RY`, `RZ`, `U`, `XY`, `XY2P`, `XY2M`, `PHASE`
- Multi-qubit gates: `RXX`, `RYY`, `RZX`, `RZZ`, `SWAP`, `CX`, `CCX`, `CY`, `CZ`, `CRX`, `CRY`, `CRZ`, `FSIM`

Besides gates, QCIS text also expresses measurement (`M`), barrier (`B`) and delay (`I Qn t`, where `t` is the number of delay ticks in units of 0.5 ns and must be a non-negative integer).

When parsing, `SDG`, `TDG` and `Barrier` are accepted as equivalent spellings of `SD`, `TD` and `B` respectively; when dumping, they are uniformly written as `SD`, `TD` and `B`. The standard identity gate and global phase are not in the native gate set, and `I` in QCIS always means delay. When an inexpressible gate is encountered, decompose it into the QCIS gate set first and then dump.

## Minimal example

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::ir::{qcis_dumps, qcis_loads};

let mut c = Circuit::new(2);
c.h(Qubit::new(0)).unwrap();
c.cz(Qubit::new(0), Qubit::new(1)).unwrap();

let text = qcis_dumps(&c).unwrap();
let c2 = qcis_loads(&text).unwrap();
assert_eq!(c2.num_qubits(), 2);
```
