# qasm3

Provides bidirectional conversion between OpenQASM 3.0 text and `Circuit`; the interface shape is exactly the same as [qasm2](2_qasm2.md).

---

## Functions

### qasm3_loads(source)

Parse a circuit from an OpenQASM 3.0 string.

Parameters:

- `source` (`const char *`): the QASM3 text, usually starting with the `OPENQASM 3.0;` header, declaring qubits in the `qubit[2] q;` form.

Returns:

- `CCircuit *`: a heap-allocated circuit, released with `circuit_free`; returns NULL on failure.

Status code:

- `-4`: invalid QASM3 syntax or parse failure.

### qasm3_load(path)

Parse a circuit from a QASM3 file.

Parameters:

- `path` (`const char *`): the QASM3 file path.

Returns:

- `CCircuit *`: returns NULL on failure.

Status code:

- `-5`: file read failure (nonexistent file, insufficient permissions, and so on).
- `-4`: parsing of the file content failed.

### qasm3_dumps(circuit)

Export a circuit to a QASM3 string.

Parameters:

- `circuit` (`const CCircuit *`): the circuit.

Returns:

- `char *`: the heap-allocated QASM3 text, released with `cqlib_string_free`; returns NULL on failure.

### qasm3_dump(circuit, path)

Write a circuit to a QASM3 file.

Parameters:

- `circuit` (`const CCircuit *`): the circuit.
- `path` (`const char *`): the output file path.

Returns:

- `int32_t`; `0` on success, `-5` on IO failure.

---

## Notes

The current export focuses on the core gate set, and the export fidelity of complex control flow is subject to the actual output of `qasm3_dumps`. The remaining conventions (qubit index, round-trip verification, cross-format conversion) are the same as [qasm2](2_qasm2.md).

---

## Example

### Parsing and export

```c
const char *src =
    "OPENQASM 3.0;\n"
    "include \"stdgates.inc\";\n"
    "qubit[2] q;\n"
    "h q[0];\n"
    "cx q[0], q[1];\n";

CCircuit *qc = qasm3_loads(src);
if (!qc) { return 1; }

char *text = qasm3_dumps(qc);
printf("%s\n", text);
cqlib_string_free(text);

qasm3_dump(qc, "bell.qasm");                     // 写文件
CCircuit *qc2 = qasm3_load("bell.qasm");   // 从文件读回
circuit_free(qc2);
circuit_free(qc);
```

### Cross-format conversion

```c
CCircuit *qc = qasm3_loads(qasm3_src);
char *qasm2_text = qasm2_dumps(qc);      // QASM3 -> QASM2
cqlib_string_free(qasm2_text);
circuit_free(qc);
```
