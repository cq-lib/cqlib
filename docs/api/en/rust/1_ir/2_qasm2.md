# OpenQASM 2.0 API

`cqlib_core::ir` provides OpenQASM 2.0 import and export interfaces.

## Import

```rust
use cqlib_core::ir::{qasm2_load, qasm2_loads, qasm2_dump, qasm2_dumps};
```

## Interface overview

- `qasm2_load(path) -> Result<Circuit, QasmParseError>`
- `qasm2_loads(source: &str) -> Result<Circuit, QasmParseError>`
- `qasm2_dump(circuit: &Circuit, path) -> Result<(), QasmDumpError>`
- `qasm2_dumps(circuit: &Circuit) -> Result<String, QasmDumpError>`

The path parameter of the file entry points is generic over `P: AsRef<Path>`; `&str`, `PathBuf` and `&Path` can all be passed directly.

### Entry points inside the format module

The `load` and `dump` submodules under `cqlib_core::ir::qasm2` forward the functions of the same name and re-export a set of equivalent Rust-style entry points at module level:

- `cqlib_core::ir::qasm2::loads` / `cqlib_core::ir::qasm2::from_str`
- `cqlib_core::ir::qasm2::load` / `cqlib_core::ir::qasm2::from_path`
- `cqlib_core::ir::qasm2::dumps` / `cqlib_core::ir::qasm2::to_string`
- `cqlib_core::ir::qasm2::dump` / `cqlib_core::ir::qasm2::to_path`

`cqlib_core::ir::qasm2::ast` is a public abstract syntax tree module, where the statement, expression and parameter-reference types of the parse output are defined.

## Parsing (load / loads)

### `qasm2_loads`

Parse a circuit from a QASM string.

### `qasm2_load`

Read from a file and parse a circuit.

Both return `QasmParseError` on failure; the variants include:

- `IoError`: filesystem or I/O failure.
- `ParseError`: syntax error.
- `ConversionError`: conversion from the AST to `Circuit` failed.
- `UndefinedQubit`: an undefined quantum register or qubit is referenced.
- `UndefinedRegister`: an undefined classical register is referenced.
- `UndefinedGate`: an undefined gate is referenced.
- `ReservedGateName`: an attempt to redefine a gate name declared by qelib1.
- `InvalidArgument`: the parameter format or usage is invalid.
- `MismatchedQubitCount` / `MismatchedParameterCount`: the number of qubits or parameters of a gate call does not match.
- `RecursionLimitExceeded` / `CircularGateDependency`: gate expansion is too deep or gates depend on each other in a cycle.
- `EvaluationError`: evaluation of a parameter expression failed.
- `UnsupportedVersion`: the version declaration is not 2.0.
- `DuplicateDeclaration`: duplicate declaration or naming conflict.
- `IncludeCycle`: `include` forms a cycle.
- `UnsupportedOpaqueGate`: an `opaque` gate is called and cannot be represented in `Circuit`.

## Dumping (dump / dumps)

### `qasm2_dumps`

Dump a circuit to a QASM string; returns `QasmDumpError` on failure. The output contains an auto-generated comment, the `OPENQASM 2.0;` and `include "qelib1.inc";` header, and `qreg` / `creg` declarations.

Error types:

- `FormatError`: content generation failed.
- `MeasureInGateNotAllowed` / `ResetInGateNotAllowed`: measurement or reset appears inside a `gate` definition body.
- `InvalidQubitIndex`: the qubit index is invalid.
- `UnsupportedClassicalType`: the classical type cannot be represented as a `creg`.
- `UnsupportedClassicalData` / `UnsupportedClassicalControl`: a classical data operation or classical control cannot be expressed.
- `ConflictingGateDefinition`: different custom gates use the same OpenQASM gate name.
- `IoError`: write failure.

### `qasm2_dump`

Write the dump result to a file, returning `Result<(), QasmDumpError>`; a file write failure is `IoError`.

## Supported features (overview)

According to the current implementation, QASM2 import and export support:

- Standard gates and parameterized gates, including `ch/cu1/cu3` and `sx/sxdg/crx/cry/crz/rxx/ryy/rzz/fsim`
- Instructions such as measurement, barrier and reset
- `qreg`, `creg` and `opaque` declarations, and the conditional statement `if (creg == integer) qop;`
- Export of `CircuitGate` definitions; a unitary gate that carries only a matrix is dumped as an `opaque` declaration
- Extension gates not covered directly are emitted as `gate` definitions (for example `crx/cry/rzz/rxx/ryy/rzx`)
- `include "qelib1.inc";` is treated as a built-in include; the file need not exist locally

## Limitations

Dumping makes trade-offs according to the expressive power of OpenQASM 2.0; the following constructs raise an error:

- `else` branches, loops, `switch` and nested control flow in classical control
- Classical types that cannot be represented as `creg`, for example `bool` and unsigned integers
- Measurement and reset inside a `gate` definition body
- Bit-wise conditions (for example `if (c[0] == 1)`) and conditional barriers

On the loading side, `loads` does not resolve external files brought in by `include`; only `load` resolves them relative to the directory of the file. `rzx` appears only in the extension gate definitions on the dumping side and is not accepted on loading; global phase is not written as a statement but emitted only as a comment, and is not restored on reload.

## Minimal example

```rust
use cqlib_core::ir::{qasm2_dumps, qasm2_loads};

let qasm = r#"
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
cx q[0],q[1];
"#;

let c = qasm2_loads(qasm).unwrap();
let out = qasm2_dumps(&c).unwrap();
assert!(out.contains("OPENQASM 2.0;"));
```
