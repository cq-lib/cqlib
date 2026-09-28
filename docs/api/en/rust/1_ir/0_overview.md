# Intermediate Representation

`cqlib_core::ir`

`cqlib_core::ir` is the intermediate representation module of Cqlib, providing two-way conversion between `Circuit` and three circuit text formats: QCIS, OpenQASM 2.0 and OpenQASM 3.0. The crate root exports four function aliases for each format; inside each format module the code is organized into separate files for parsing and dumping, and each holds an independent error type.

## Overview

The intermediate representation addresses the text form of circuits: a circuit travels between files, tools and services as text, while inside Cqlib it is uniformly represented by `Circuit`. Every format module under `cqlib_core::ir` provides both directions, "text in, `Circuit` out" and "`Circuit` in, text out", and the conversion does not modify the `Circuit` passed in.

### Crate root aliases

The crate root re-exports the four entry points of each format as `<format>_<action>`; they are the same items as the functions of the same name inside the format modules:

| Action | QCIS | OpenQASM 2.0 | OpenQASM 3.0 |
| --- | --- | --- | --- |
| String → `Circuit` | `qcis_loads` | `qasm2_loads` | `qasm3_loads` |
| File → `Circuit` | `qcis_load` | `qasm2_load` | `qasm3_load` |
| `Circuit` → String | `qcis_dumps` | `qasm2_dumps` | `qasm3_dumps` |
| `Circuit` → File | `qcis_dump` | `qasm2_dump` | `qasm3_dump` |

### Format modules

Each format module (`cqlib_core::ir::qcis`, `cqlib_core::ir::qasm2`, `cqlib_core::ir::qasm3`) consists of two submodules, `load` and `dump`, which forward the functions of the same name and additionally provide `from_str` / `from_path` and `to_string` / `to_path`. `qasm2::ast` is a public abstract syntax tree module, where the parse output can be inspected separately.

### Error types

Parsing and dumping of each format has its own independent error enum; the crate provides no unified wrapper:

| Format | Parse error | Dump error |
| --- | --- | --- |
| QCIS | `qcis::load::QcisParseError` | `qcis::dump::QcisDumpError` |
| OpenQASM 2.0 | `qasm2::load::QasmParseError` | `qasm2::dump::QasmDumpError` |
| OpenQASM 3.0 | `qasm3::load::Qasm3ParseError` | `qasm3::dump::Qasm3DumpError` |

When I/O fails, the file entry points return an `IoError` variant carrying the original `std::io::Error`; the original error can be retrieved layer by layer with `Error::source`.

### Round-trip conversion and boundaries

The circuit obtained from `loads(dumps(circuit))` is equivalent to the input circuit in its operation sequence; this is the round-trip conversion. The expressive power of the text formats is limited, however:

- QCIS accepts only gates within its native gate set; if the circuit contains other gates, they must first be decomposed into that gate set before dumping. Classical storage (`store`) and classical control are outside the expressive scope of QCIS; measurement and barriers can be expressed, and symbolic gate parameters are written out verbatim as symbolic text.
- When dumping fails, the enum indicates the specific reason, for example an unsupported gate, a classical operation that cannot be expressed, or an invalid delay tick.

### Relation to compilation

Dumping is an optional write-to-file form for compilation output: once a circuit has been lowered to the target gate set, it can be written out in the corresponding format. Format differences appear only in the gate set and the text header; the interface on the `Circuit` side stays unchanged, so a circuit can also be read in one format and written out in another, completing a format conversion.

---

## Common entry points

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::ir::{qcis_dumps, qcis_loads};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();

let qcis = qcis_dumps(&circuit).unwrap();
let parsed = qcis_loads(&qcis).unwrap();

assert_eq!(parsed.num_qubits(), 2);
```

---

## Core concepts and terms

| Term | Description |
| --- | --- |
| **Intermediate representation** | The representation layer between `Circuit` and circuit text formats, provided by the format modules under `cqlib_core::ir`. |
| **Parsing** | The direction from text to `Circuit`, corresponding to `loads` (string) and `load` (file). |
| **Dumping** | The direction from `Circuit` to text, corresponding to `dumps` (string) and `dump` (file). |
| **Round-trip conversion** | Dumping and then parsing back to an equivalent circuit, that is `loads(dumps(circuit))`. |
| **Format conversion** | Reading in one format and writing out another, for example writing OpenQASM 2.0 text as QCIS text. |
| **Native gate set** | The gate set defined by the text format itself; on dumping, every gate in the circuit must fall within that set. |

---

## `cqlib_core::ir` API overview

### Crate root aliases

| Name | Description |
| --- | --- |
| [`qcis_loads`](1_qcis.md) / [`qcis_load`](1_qcis.md) | Parse a `Circuit` from a QCIS string or file. |
| [`qcis_dumps`](1_qcis.md) / [`qcis_dump`](1_qcis.md) | Dump a `Circuit` to a QCIS string or file. |
| [`qasm2_loads`](2_qasm2.md) / [`qasm2_load`](2_qasm2.md) | Parse a `Circuit` from an OpenQASM 2.0 string or file. |
| [`qasm2_dumps`](2_qasm2.md) / [`qasm2_dump`](2_qasm2.md) | Dump a `Circuit` to an OpenQASM 2.0 string or file. |
| [`qasm3_loads`](3_qasm3.md) / [`qasm3_load`](3_qasm3.md) | Parse a `Circuit` from an OpenQASM 3.0 string or file. |
| [`qasm3_dumps`](3_qasm3.md) / [`qasm3_dump`](3_qasm3.md) | Dump a `Circuit` to an OpenQASM 3.0 string or file. |

### Format modules

| Name | Description |
| --- | --- |
| [`cqlib_core::ir::qcis`](1_qcis.md) | The QCIS format, with `load` / `dump` submodules and auxiliary entry points such as `from_str` and `to_string`. |
| [`cqlib_core::ir::qasm2`](2_qasm2.md) | The OpenQASM 2.0 format, which additionally has a public `ast` submodule. |
| [`cqlib_core::ir::qasm3`](3_qasm3.md) | The OpenQASM 3.0 format. |

### Error types

| Name | Description |
| --- | --- |
| [`QcisParseError`](1_qcis.md) / [`QcisDumpError`](1_qcis.md) | QCIS parse and dump errors. |
| [`QasmParseError`](2_qasm2.md) / [`QasmDumpError`](2_qasm2.md) | OpenQASM 2.0 parse and dump errors. |
| [`Qasm3ParseError`](3_qasm3.md) / [`Qasm3DumpError`](3_qasm3.md) | OpenQASM 3.0 parse and dump errors. |

---

## Quick examples

### 1. Parse QCIS text and write it back

```rust
use cqlib_core::ir::{qcis_dumps, qcis_loads};

let qcis = r#"
        RX Q0 1.0
        RY Q1 pi/2
        X Q0
        CZ Q0 Q1
    "#;

let circuit = qcis_loads(qcis).unwrap();
assert_eq!(circuit.num_qubits(), 2);
```

### 2. The exact text of a dump result

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::ir::qcis_dumps;

let mut c = Circuit::new(2);
let q0 = Qubit::new(0);
let q1 = Qubit::new(1);

c.x(q0).unwrap();
c.y(q1).unwrap();
c.z(q0).unwrap();
c.h(q1).unwrap();

let qcis = qcis_dumps(&c).unwrap();
assert_eq!(qcis, "X Q0\nY Q1\nZ Q0\nH Q1\n");
```

### 3. Write to a file and inspect the error source

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::ir::qcis::dump::QcisDumpError;
use cqlib_core::ir::qcis_dump;
use std::error::Error;

let mut c = Circuit::new(1);
c.h(Qubit::new(0)).unwrap();

let path = std::env::temp_dir().join("cqlib_ir_overview.qcis");
qcis_dump(&c, &path).unwrap();

let missing = path.join("out.qcis");
let error = qcis_dump(&c, missing).unwrap_err();
assert!(matches!(error, QcisDumpError::IoError(_)));
assert!(error.source().is_some());
```

---

## Validation and error handling

Parse errors and dump errors are defined separately; file failures are represented by the `IoError` variant, and the remaining variants are content-level causes.

| Error | When it occurs |
| --- | --- |
| `QcisParseError::IoError` | `qcis_load` failed to read the file. |
| `QcisParseError::{InvalidQubitFormat, InvalidQubitId, QubitCountMismatch, ParameterCountMismatch, MissingParameter, InvalidParameter, UnknownGate, EmptyLine}` | Qubit notation in the QCIS text, an unknown gate, a mismatch in gate arity or parameter count, and empty lines or no valid content. |
| `QcisDumpError::{UnsupportedGate, UnsupportedClassicalData, UnsupportedClassicalControl, SymbolicParameter, InvalidDelayParameter}` | A gate, classical data, classical control, symbolic parameter or invalid delay parameter that cannot be expressed when dumping QCIS. |
| `QasmParseError::{ParseError, ConversionError, UndefinedQubit, UndefinedRegister, UndefinedGate, ReservedGateName, InvalidArgument, MismatchedQubitCount, MismatchedParameterCount, RecursionLimitExceeded, EvaluationError, CircularGateDependency, UnsupportedVersion, DuplicateDeclaration, IncludeCycle, UnsupportedOpaqueGate}` | OpenQASM 2.0 syntax, semantic, gate definition and version declaration problems. |
| `QasmDumpError::{FormatError, MeasureInGateNotAllowed, ResetInGateNotAllowed, InvalidQubitIndex, UnsupportedClassicalType, UnsupportedClassicalData, UnsupportedClassicalControl, ConflictingGateDefinition}` | Invalid generated content, measurement or reset inside a gate definition, an invalid qubit index, and classical types or duplicate custom gate names that cannot be expressed when dumping OpenQASM 2.0. |
| `Qasm3ParseError::{ParseError, SemanticError, ConversionError, UnsupportedFeature, UndefinedSymbol, UndefinedGate, TypeError, InvalidArgument, MismatchedQubitCount, MismatchedParameterCount, RecursionLimitExceeded, CircularGateDependency}` | OpenQASM 3.0 syntax, semantics, symbol resolution, type checking and lowering failures. |
| `Qasm3DumpError::{FormatError, UnsupportedInstruction, UnsupportedClassicalData, UnsupportedClassicalControl, MeasureInGateNotAllowed, ConflictingGateDefinition}` | Instructions, classical data, classical control and duplicate custom gate names that cannot be expressed when dumping OpenQASM 3.0. |
| `*DumpError::IoError` / `*ParseError::IoError` | File write or read failure; the original I/O error can be retrieved with `Error::source`. |

Before dumping, the circuit must already fall within the gate set of the target format; otherwise perform gate decomposition first and then dump.
