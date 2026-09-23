# qasm2

Provides bidirectional conversion between OpenQASM 2.0 text and `Circuit`.

---

## Functions

### qasm2_loads(source)

Parse a circuit from an OpenQASM 2.0 string.

Parameters:

- `source` (`const char *`): the QASM2 text, usually starting with the `OPENQASM 2.0;` header.

Returns:

- `CCircuit *`: a heap-allocated circuit, released with `circuit_free`; returns NULL on failure.

Status code:

- `-4`: invalid QASM2 syntax or parse failure.

### qasm2_load(path)

Parse a circuit from a QASM2 file.

Parameters:

- `path` (`const char *`): the QASM2 file path.

Returns:

- `CCircuit *`: returns NULL on failure.

Status code:

- `-5`: file read failure (nonexistent file, insufficient permissions, and so on).
- `-4`: parsing of the file content failed.

### qasm2_dumps(circuit)

Export a circuit to a QASM2 string.

Parameters:

- `circuit` (`const CCircuit *`): the circuit.

Returns:

- `char *`: the heap-allocated QASM2 text (with the `OPENQASM 2.0;` header and register declarations), released with `cqlib_string_free`; returns NULL on failure.

### qasm2_dump(circuit, path)

Write a circuit to a QASM2 file.

Parameters:

- `circuit` (`const CCircuit *`): the circuit.
- `path` (`const char *`): the output file path.

Returns:

- `int32_t`; `0` on success, `-5` on IO failure.

---

## Notes

- The qubit index starts from `0` in register declaration order, consistent with the logical index of `Circuit`;
- `load`/`loads` and `dump`/`dumps` share the same parsing/serialization implementation, and `loads(dumps(c))` can be used for round-trip verification;
- Export only covers the supported gate set; when the circuit contains an unsupported operation, the export fails and returns NULL.

---

## Example

### Parsing and export

```c
const char *src =
    "OPENQASM 2.0;\n"
    "include \"qelib1.inc\";\n"
    "qreg q[2];\n"
    "h q[0];\n"
    "cx q[0], q[1];\n";

CCircuit *qc = qasm2_loads(src);
if (!qc) { return 1; }

char *text = qasm2_dumps(qc);      // 导出回 QASM2 字符串
printf("%s\n", text);
cqlib_string_free(text);

circuit_free(qc);
```

### File read/write round trip

```c
qasm2_dump(qc, "bell.qasm");                     // 写文件
CCircuit *qc2 = qasm2_load("bell.qasm");   // 从文件读回
printf("ops=%zu\n", (size_t)circuit_num_operations(qc2));
circuit_free(qc2);
circuit_free(qc);
```

Cross-format conversion is done with `loads` / `dumps` (for example QASM2 → QCIS):

```c
CCircuit *qc = qasm2_loads(qasm_src);
char *qcis_text = qcis_dumps(qc);       // QASM2 -> QCIS
cqlib_string_free(qcis_text);
circuit_free(qc);
```
