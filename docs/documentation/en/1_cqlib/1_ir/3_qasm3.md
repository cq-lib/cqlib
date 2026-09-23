# OpenQASM 3.0 support

OpenQASM 3.0 targets modern quantum program design, and is better suited than OpenQASM 2.0 to expressing measurement assignment, classical variables, control flow and more complete dynamic circuit semantics. The `qasm3` module of Cqlib is responsible for bidirectional conversion between OpenQASM 3.0 text and a Cqlib `Circuit`.

Corresponding Python module:

```python
from cqlib.ir import qasm3
```

## 1. API overview

| Function | Use | Example |
|---|---|---|
| `qasm3.loads(text)` | Parse a `Circuit` from an OpenQASM 3.0 string | `circuit = qasm3.loads(qasm_text)` |
| `qasm3.load(path)` | Parse a `Circuit` from an OpenQASM 3.0 file | `circuit = qasm3.load("input.qasm")` |
| `qasm3.dumps(circuit)` | Export a `Circuit` as an OpenQASM 3.0 string | `text = qasm3.dumps(circuit)` |
| `qasm3.dump(circuit, path)` | Write a `Circuit` to an OpenQASM 3.0 file | `qasm3.dump(circuit, "output.qasm")` |

## 2. Minimal OpenQASM 3.0 program

```text
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;

h q[0];
cx q[0],q[1];
```

Loading into Cqlib:

```python
from cqlib.ir import qasm3

qasm_code = """
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;

h q[0];
cx q[0],q[1];
"""

circuit = qasm3.loads(qasm_code)
print(circuit.num_qubits)
```

Both `OPENQASM 3;` and `OPENQASM 3.0;` can be loaded.

## 3. Exporting OpenQASM 3.0 from Cqlib

```python
from cqlib import Circuit
from cqlib.ir import qasm3

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

text = qasm3.dumps(circuit)
print(text)
```

Typical output:

```text
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;

h q[0];
cx q[0],q[1];
```

The exporter generates normalized text, and does not preserve the spaces, comments and variable names of the original input.

## 4. File reading and writing

```python
from cqlib.ir import qasm3

# first write the QASM text above to a file to be read
qasm3.dump(qasm3.loads(qasm_code), "input.qasm")

circuit = qasm3.load("input.qasm")
qasm3.dump(circuit, "output.qasm")
```

A file reading failure raises an I/O exception; a syntax error, a semantic error or an unsupported feature raises a `ValueError`.

## 5. `stdgates.inc`

OpenQASM 3.0 standard gates are usually introduced through:

```text
include "stdgates.inc";
```

Cqlib relies on the OpenQASM 3 frontend to recognize standard library gates. On export, `include "stdgates.inc";` is written out as well, which makes it convenient for external OpenQASM 3 tools to read.

For extension gates that Cqlib has but `stdgates.inc` does not necessarily provide directly, the exporter generates a gate definition before the main circuit. For example, `x2p`, `rxx`, `rzz`, `rzx`, `fsim` and others may be written as custom gates.

## 6. Measurement assignment

Measurement in OpenQASM 3.0 is more natural than in OpenQASM 2.0, because it supports assignment expressions:

```text
bit c;
c = measure q[0];
```

Multi-bit measurement:

```text
bit[2] c;
c = measure q;
```

Partial bit-vector assignment:

```text
bit[2] c;
c[0] = measure q[2];
c[1] = measure q[0];
```

Cqlib currently supports the measurement forms above, and generates a canonical form that can be read back as far as possible on export.

## 7. Mapping rules from Cqlib measurement to QASM3

Cqlib divides measurement into two layers internally:

- `ClassicalValue`: the immutable result produced by measurement.
- `ClassicalVar`: the mutable classical variable created by users.

On export to QASM3, the following rules are applied:

| Cqlib operation | QASM3 output | Description |
|---|---|---|
| `measure_into(q, bit)` | `c0 = measure q[0];` | Single-bit measurement written into a user variable |
| `measure_bits_into([0,1], bitvec)` | `c0 = measure q;` | The whole register is merged into one output when the order is consistent |
| `measure_bits_into([2,0], bitvec)` | `c0[0] = measure q[2];` and `c0[1] = measure q[0];` | Non-contiguous or reordered measurement is split into indexed assignment |
| `measure(q)` | `bit[n] meas; meas[i] = measure q[j];` | A bare measurement generates an explicit register, ensuring that the QASM3 can be read back |

Example:

```python
from cqlib import Circuit
from cqlib.ir import qasm3

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
circuit.measure(0)
circuit.measure(1)

print(qasm3.dumps(circuit))
```

Output:

```text
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;
bit[2] meas;

h q[0];
cx q[0],q[1];
meas[0] = measure q[0];
meas[1] = measure q[1];
```

The reason for this is: a bare measurement can produce only a temporary value inside Cqlib, but if QASM3 text is to be stored and read back reliably across tools, an explicit classical destination is preferable. The current implementation generates a `meas` register instead of leaking the internal temporary name `v0/v1`.

## 8. Scalar measurement assignment compatibility

Common forms in OpenQASM 3.0 can also be loaded:

```text
OPENQASM 3.0;
qubit[1] q;
bit v;
v = measure q[0];
```

And:

```text
OPENQASM 3.0;
qubit q;
bit[2] c;
c[0] = measure q;
```

Cqlib lowers them into an internal measurement operation plus a classical store operation.

## 9. reset, barrier and global phase

Example:

```text
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;
reset q[0];
barrier q[0],q[1];
gphase(0.25);
```

Cqlib supports loading these statements as corresponding `Circuit` operations. On export, equivalent semantics are preserved as far as possible.

## 10. Custom gates

OpenQASM 3.0 custom gate example:

```text
OPENQASM 3.0;
include "stdgates.inc";

gate bell a, b {
    h a;
    cx a, b;
}

qubit[2] q;
bell q[0], q[1];
```

After loading, Cqlib treats `bell` as a circuit definition gate. On export, if the `Circuit` contains a `CircuitGate` that can be represented as a QASM3 gate body, the corresponding definition is output as well.

## 11. Control flow support scope

The QASM3 loader of Cqlib supports part of the control flow that can be mapped to the current `Circuit`:

| OpenQASM 3 feature | Current support | Description |
|---|---|---|
| `if/else` | Supported | The condition must be convertible into a Cqlib classical expression |
| Static `for` | Supported | For example a fixed range `[0:2]`, which can be expanded statically |
| `switch` | Partial support for exact-value cases | Suitable for simple unsigned integer cases |
| `while` | Limited by the frontend and the expressive power of the IR | A complex runtime loop may be rejected |
| `break/continue` | Restricted | Depends on whether it can be mapped to the current control flow model |

Recommended principle: if a circuit is mainly used for cross-toolchain exchange, use simple, static and expandable control flow as far as possible; for a complex dynamic program, check whether the target backend supports it first.

## 12. Current support and limitations

| Type | Support status |
|---|---|
| `qubit`, `qubit[n]` | Supported |
| `bit`, `bit[n]`, `bool`, `uint[n]` | Common forms supported |
| Standard gates | Supported for gates that can be mapped to a Cqlib `StandardGate` |
| Cqlib extension gates | A gate definition is generated on export |
| Measurement assignment | Common forms of scalar, bit-vector and indexed assignment supported |
| `reset`, `barrier`, `gphase` | Supported |
| Custom gate | Supported for a mappable gate body |
| calibration, pulse, extern, hardware qubit, alias | Currently not supported |
| Arbitrary complex classical arithmetic | Lossless lowering is currently not supported |
| Complex lvalue slicing, multi-dimensional indexing | Currently not supported, or only the necessary subset is supported |

## 13. Troubleshooting common errors

| Symptom | Common cause | Handling |
|---|---|---|
| `QASM3 parse error` | A syntax error, a frontend semantic check failure or use of a feature that Cqlib does not support | Verify with a minimal QASM3 program first, then add features step by step |
| `unsupported feature` | Use of calibration, extern, a complex lvalue or an unsupported gate modifier | Rewrite as basic gates, or expand into a basic circuit in an external tool first |
| Export fails | The `Circuit` contains a Delay, Unitary or complex store that QASM3 cannot represent | Call `decompose()` first, or switch to a more suitable format |
| An external tool cannot read the QASM3 exported by Cqlib | The external tool does not yet support some extension gates or OpenQASM 3 subsets | Decompose the extension gates first, or switch to the syntax subset supported by the target tool |
| The measurement register name becomes `meas` | Cqlib generates an explicit target register for a bare measurement | Normal behavior, used to ensure that the text can be read back |

## Next steps

- [Format conversion workflow](4_conversion_workflow.md)
