# QCIS support

QCIS is a quantum circuit text format oriented to hardware instructions and engineering delivery. It describes quantum operations in the form of "one instruction per line", and is suitable for exporting a Cqlib circuit to hardware-related toolchains, or for restoring a `Circuit` from an existing QCIS file for simulation, visualization and recompilation.

Corresponding Python module:

```python
from cqlib.ir import qcis
```

## 1. API overview

| Function | Use | Example |
|---|---|---|
| `qcis.loads(text)` | Parse a `Circuit` from a QCIS string | `circuit = qcis.loads("H Q0\nM Q0\n")` |
| `qcis.load(path)` | Parse a `Circuit` from a QCIS file | `circuit = qcis.load("input.qcis")` |
| `qcis.dumps(circuit)` | Export a `Circuit` as a QCIS string | `text = qcis.dumps(circuit)` |
| `qcis.dump(circuit, path)` | Write a `Circuit` to a QCIS file | `qcis.dump(circuit, "output.qcis")` |

## 2. QCIS text structure

QCIS is a line-based format. Each line usually consists of three parts:

```text
OPCODE QUBIT_LIST [PARAMETER_LIST]
```

Example:

```text
H Q0
CZ Q0 Q1
RZ Q0 pi/2
M Q0 Q1
```

Meaning:

- `H Q0`: applies a Hadamard gate on `Q0`.
- `CZ Q0 Q1`: applies a CZ gate on `Q0` and `Q1`.
- `RZ Q0 pi/2`: applies an RZ gate with parameter `pi/2` on `Q0`.
- `M Q0 Q1`: measures `Q0` and `Q1`.

QCIS supports comments starting with `//`, and also supports inline comments.

```text
// prepare Bell state
H Q0
CZ Q0 Q1  // entangle Q0 and Q1
```

## 3. Loading from a QCIS string

```python
from cqlib.ir import qcis

qcis_code = """
H Q0
CZ Q0 Q1
RZ Q0 pi/2
M Q0 Q1
"""

circuit = qcis.loads(qcis_code)
print(circuit.num_qubits)
print(len(circuit.operations))
```

The loaded object is a standard Cqlib `Circuit`, which can continue to be used for visualization, simulation, compilation and optimization or export again.

## 4. Loading from a QCIS file

```python
from cqlib.ir import qcis

# first write the QCIS text above to a file to be read
qcis.dump(qcis.loads(qcis_code), "input.qcis")

circuit = qcis.load("input.qcis")
```

If the file does not exist, an I/O-related exception is raised; if the QCIS content has a syntax error or the gate parameters do not match, a `ValueError` is raised.

## 5. Exporting a QCIS string

```python
from cqlib import Circuit
from cqlib.ir import qcis

circuit = Circuit(2)
circuit.h(0)
circuit.cz(0, 1)
circuit.rz(0, 3.141592653589793 / 2)
circuit.measure(0)
circuit.measure(1)

text = qcis.dumps(circuit)
print(text)
```

Typical output:

```text
H Q0
CZ Q0 Q1
RZ Q0 pi/2
M Q0
M Q1
```

## 6. Exporting a QCIS file

```python
from cqlib.ir import qcis

qcis.dump(circuit, "output.qcis")
```

An I/O exception is raised when writing the file fails; if the circuit contains an instruction that QCIS cannot represent, a `ValueError` is raised.

## 7. Supported instruction types

The current QCIS module covers the standard gates and instructions in Cqlib that can be represented as QCIS text.

| Type | Supported content |
|---|---|
| Single-qubit gates | `H`, `S`, `SD`, `T`, `TD`, `X`, `X2P`, `X2M`, `Y`, `Y2P`, `Y2M`, `Z` |
| Parameterized single-qubit gates | `RX`, `RY`, `RZ`, `RXY`, `U`, `XY`, `XY2P`, `XY2M`, `PHASE` |
| Multi-qubit gates | `CX`, `CY`, `CZ`, `SWAP`, `CCX`, `CRX`, `CRY`, `CRZ`, `RXX`, `RYY`, `RZZ`, `RZX`, `FSIM` |
| Instructions | `M` measurement, `B`/`Barrier` barrier |
| Delay | `I Qn t`, meaning a delay of `t` ticks on `Qn` |

Alias rules:

- `SDG` can be loaded and is normalized to `SD` on export.
- `TDG` can be loaded and is normalized to `TD` on export.

## 8. `I` in QCIS is not an ordinary identity gate

QCIS `I Qn t` represents a delay instruction, not the identity gate among the Cqlib standard gates.

```text
I Q0 10
```

The meaning is idling on `Q0` for the specified duration. Cqlib represents it as a `Delay` instruction after loading. On export, if an ordinary identity gate has been placed directly in the circuit, the QCIS dumper refuses to export, avoiding the misinterpretation of an "identity gate with no duration" as a "hardware delay with a duration".

## 9. Measurement semantics

QCIS only describes the measurement instruction itself, and does not describe the explicit classical register assignment of OpenQASM. Therefore:

```text
M Q0 Q1
```

After loading into Cqlib, it becomes a measurement operation in the `Circuit`; if it is subsequently exported to OpenQASM 3, Cqlib automatically supplies a readable classical destination, for example:

```text
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;
bit[2] meas;

meas[0] = measure q[0];
meas[1] = measure q[1];
```

This step is normal behavior at the format conversion boundary: QCIS has no classical assignment syntax while OpenQASM 3 does, so Cqlib completes the target register on export.

## 10. Conversion from QCIS to OpenQASM

```python
from cqlib.ir import qcis, qasm3

qcis_code = """
H Q0
CZ Q0 Q1
M Q0 Q1
"""

circuit = qcis.loads(qcis_code)
qasm3_text = qasm3.dumps(circuit)
print(qasm3_text)
```

This flow is suitable for converting a hardware-side QCIS file into more general OpenQASM 3 text, which is then handed to other tools that support OpenQASM for reading.

## 11. Unsupported cases or cases requiring prior handling

QCIS is a hardware-instruction-style format and is not suitable for expressing all high-level circuit semantics. The following content usually cannot be exported to QCIS directly:

- A `UnitaryGate` in arbitrary matrix form.
- A user-defined `CircuitGate`, unless it is first decomposed into basic gates supported by QCIS.
- Generalized forms of multi-controlled gates, unless they have already been decomposed.
- Complex classical control flow such as `if/else`, `for`, `while` and `switch`.
- Standard identity gates and the global phase `GPhase`.

Recommended handling:

```python
compiled = circuit.decompose()
text = qcis.dumps(compiled)
```

If it still fails, the decomposed circuit still contains instructions that QCIS cannot express, and compilation and optimization or target gate set mapping is required first.

## 12. Troubleshooting common errors

| Symptom | Common cause | Handling |
|---|---|---|
| `ValueError: QCIS parse error` | An unknown gate, an incorrect qubit format or a parameter count mismatch in the text | Check whether the `Q0` form is used, and check the parameter count of each gate |
| An unsupported gate is reported on export | The `Circuit` contains a gate or control flow that QCIS does not support | Call `decompose()` first, then perform target gate set mapping |
| The behavior of the `I` instruction differs from expectation | `I` in QCIS is a delay, not an ordinary identity | If it is only an ordinary identity gate, exporting it to QCIS is not recommended |
| The number of qubits appears to change | Cqlib constructs the circuit according to the maximum qubit index that actually appears | Check whether the QCIS file starts from `Q1` without `Q0` |

## Next steps

- [OpenQASM 2.0 support](2_qasm2.md)
- [OpenQASM 3.0 support](3_qasm3.md)
- [Format conversion workflow](4_conversion_workflow.md)
