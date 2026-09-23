# Intermediate Representation

`cqlib.ir`

`cqlib.ir` is the intermediate representation module of Cqlib, providing bidirectional conversion between `Circuit` and three circuit text formats: QCIS, OpenQASM 2.0 and OpenQASM 3.0. Each format is a submodule, and the interface shape is exactly the same across all of them.

## Overview

The intermediate representation solves the problem of textualizing circuits: a circuit is text when it travels between files, tools and services, whereas Cqlib represents it uniformly with `Circuit` internally. Each submodule of `cqlib.ir` provides two directions, "text in, `Circuit` out" and "`Circuit` in, text out", and the conversion does not modify the `Circuit` passed in.

### Unified interface: parsing and export

The three submodules use the same set of four functions:

- Parsing direction: `loads(text)` parses from a string, `load(path)` reads a file and then parses it; both return a `Circuit`.
- Export direction: `dumps(circuit)` returns a text string, `dump(circuit, path)` writes the text to a file and returns `None`.

`loads` / `dumps` do not touch the file system, whereas `load` / `dump` do read and write files, so "file read/write failure" and "content parse failure" are two different kinds of exceptions.

### The three formats

| Format | Submodule | Text characteristics |
| --- | --- | --- |
| QCIS | `cqlib.ir.qcis` | Composed of operation lines; export only accepts the QCIS native gate set. |
| OpenQASM 2.0 | `cqlib.ir.qasm2` | Carries the `OPENQASM 2.0;` and `include "qelib1.inc";` header and `qreg` declarations; supports custom gates. |
| OpenQASM 3.0 | `cqlib.ir.qasm3` | See [qasm3](3_qasm3.md). |

### Round-trip conversion and boundaries

The circuit obtained from `loads(dumps(circuit))` is equivalent to the input circuit in its operation sequence; this is round-trip conversion. The expressive power of the text formats is limited, however:

- QCIS only accepts gates inside the native gate set; if a circuit contains other gates, they must first be decomposed into that gate set before exporting.
- QCIS text does not record the number of qubits, so a circuit that has qubits but no operations exports as an empty string.

### Relationship to compilation

Export is an optional form of writing compilation output to file: once a circuit is lowered to the target gate set, it can be written out in the corresponding format. Format differences show up only in the gate set and the text header, while the interface on the `Circuit` side stays unchanged, so a circuit can also be read in one format and written out in another, which completes format conversion.

---

## Common entry points

```python
from cqlib import Circuit
from cqlib.ir import qcis

c1 = Circuit(2)
c1.h(0)
c1.cz(0, 1)

qcis_text = qcis.dumps(c1)
c2 = qcis.loads(qcis_text)

assert c2.num_qubits == 2
assert len(c2) == 2
```

---

## Core concepts and terms

| Term | Description |
| --- | --- |
| **Intermediate Representation** | The representation layer between `Circuit` and the circuit text formats, provided by the format submodules under `cqlib.ir`. |
| **Parsing** | The text-to-`Circuit` direction, corresponding to `loads` (string) and `load` (file). |
| **Export** | The `Circuit`-to-text direction, corresponding to `dumps` (string) and `dump` (file). |
| **Round-trip conversion** | Exporting and then parsing back to an equivalent circuit, that is `loads(dumps(circuit))`. |
| **Format conversion** | Reading in one format and writing out another, for example writing OpenQASM 2.0 text as QCIS text. |
| **Native gate set** | The gate set defined by the text format itself; when exporting, every gate in the circuit must fall inside that set. |

---

## `cqlib.ir` API overview

### Parsing entry points

| Name | Description |
| --- | --- |
| [`qcis.loads`](1_qcis.md) / [`qcis.load`](1_qcis.md) | Parse a `Circuit` from a QCIS string or file. |
| [`qasm2.loads`](2_qasm2.md) / [`qasm2.load`](2_qasm2.md) | Parse a `Circuit` from an OpenQASM 2.0 string or file. |
| [`qasm3.loads`](3_qasm3.md) / [`qasm3.load`](3_qasm3.md) | Parse a `Circuit` from an OpenQASM 3.0 string or file. |

### Export entry points

| Name | Description |
| --- | --- |
| [`qcis.dumps`](1_qcis.md) / [`qcis.dump`](1_qcis.md) | Export a `Circuit` as a QCIS string or file. |
| [`qasm2.dumps`](2_qasm2.md) / [`qasm2.dump`](2_qasm2.md) | Export a `Circuit` as an OpenQASM 2.0 string or file. |
| [`qasm3.dumps`](3_qasm3.md) / [`qasm3.dump`](3_qasm3.md) | Export a `Circuit` as an OpenQASM 3.0 string or file. |

### Format submodules

| Name | Description |
| --- | --- |
| [`cqlib.ir.qcis`](1_qcis.md) | The QCIS text format; export is constrained by the QCIS native gate set. |
| [`cqlib.ir.qasm2`](2_qasm2.md) | The OpenQASM 2.0 text format, supporting standard gates, parameter expressions, measurement and custom gates. |
| [`cqlib.ir.qasm3`](3_qasm3.md) | The OpenQASM 3.0 text format. |

---

## Quick examples

### 1. Round-trip conversion of QCIS text

```python
from cqlib.circuit import Circuit
from cqlib.ir.qcis import dumps, loads

c1 = Circuit(2)
c1.h(0)
c1.cz(0, 1)

qcis = dumps(c1)
c2 = loads(qcis)

assert c2.num_qubits == 2
assert len(c2) == 2
```

The exported text is one operation per line, so the content can be asserted precisely:

```python
from cqlib.circuit import Circuit
from cqlib.ir.qcis import dumps

c = Circuit(2)
c.cx(0, 1)

assert dumps(c) == "CX Q0 Q1\n"
```

### 2. Parsing and exporting OpenQASM 2.0 text

```python
from cqlib.ir import qasm2

code = """OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
h q[0];
"""

circuit = qasm2.loads(code)

text = qasm2.dumps(circuit)
lines = text.strip().split("\n")
assert lines[1] == "OPENQASM 2.0;"
assert lines[2] == 'include "qelib1.inc";'
assert "h q[0];" in text
```

### 3. Reading and writing QCIS files

```python
import os
import tempfile

from cqlib.circuit import Circuit
from cqlib.ir import qcis

c = Circuit(2)
c.h(0)
c.cz(0, 1)

with tempfile.NamedTemporaryFile(mode="w", suffix=".qcis", delete=False) as f:
    path = f.name

try:
    qcis.dump(c, path)

    with open(path) as f:
        assert "H Q0" in f.read()

    c2 = qcis.load(path)
    assert c2.num_qubits == 2
finally:
    os.unlink(path)
```

---

## Validation and error handling

Content problems raise `ValueError`, file problems raise `OSError` (`IOError` is its alias), and the message prefix of each distinguishes the format and the action.

| Exception | When it occurs |
| --- | --- |
| `ValueError` | The text syntax is invalid or parsing failed; the message prefix is `QCIS parse error:`, `QASM parse error:`, `QASM3 parse error:`. |
| `ValueError` | Export failed, for example the circuit contains a gate the target format cannot express; the message prefix is `QCIS dump error:`, `QASM dump error:`, `QASM3 dump error:`. |
| `OSError` | `load` failed to read the file; the message prefix is `<format> load error:`. |
| `OSError` | `dump` failed to write the file; the message prefix is `<format> dump error:`. |

With `load` / `dump`, the same prefix may come from a file read/write or from content processing; to distinguish the two, judge by the exception type. Before exporting, the circuit must already fall inside the target format's gate set, otherwise decompose the gates first and then export.
