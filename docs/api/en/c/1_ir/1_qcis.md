# qcis

Provides bidirectional conversion between QCIS text and `Circuit`. QCIS is the native instruction format of China Telecom quantum computers, based on the native gate set (`H`, `X2P`, `RZ`, `CZ` and so on).

---

## Functions

### qcis_loads(source)

Parse a circuit from a QCIS string.

Parameters:

- `source` (`const char *`): the QCIS text.

Returns:

- `CCircuit *`: a heap-allocated circuit, released with `circuit_free`; returns NULL on failure.

Status code:

- `-4`: invalid QCIS syntax or parse failure.

Example:

```c
const char *src = "H Q0\nX2P Q1\nCZ Q0 Q1\n";
CCircuit *qc = qcis_loads(src);
circuit_free(qc);
```

### qcis_load(path)

Parse a circuit from a QCIS file.

Parameters:

- `path` (`const char *`): the QCIS file path.

Returns:

- `CCircuit *`: returns NULL on failure.

Status code:

- `-5`: file read failure (nonexistent file, insufficient permissions, and so on).
- `-4`: QCIS parsing of the file content failed.

### qcis_dumps(circuit)

Export a circuit to a QCIS string.

Parameters:

- `circuit` (`const CCircuit *`): the circuit.

Returns:

- `char *`: the heap-allocated QCIS text, released with `cqlib_string_free`; returns NULL on failure.

### qcis_dump(circuit, path)

Write a circuit to a QCIS file.

Parameters:

- `circuit` (`const CCircuit *`): the circuit.
- `path` (`const char *`): the output file path.

Returns:

- `int32_t`; `0` on success, `-5` on IO failure.

---

## Notes

QCIS export only supports the QCIS native gate set. A non-native gate fails the export, and the circuit must first be decomposed into the native gate set with [compile](../4_compile/1_compile.md) or `circuit_decompose` before exporting.

The current QCIS native gate set is:

- `X2P`, `X2M`, `Y2P`, `Y2M`, `XY2P`, `XY2M`
- `CZ`, `RZ`, `I`, `X`, `Y`, `Z`, `H`, `S`, `SD`, `T`, `TD`
- `RX`, `RY`, `RXY`

`load` reads a "file path" and `loads` reads "string content"; `dump` writes a file and `dumps` returns a string; all four share the same parsing/serialization implementation.

---

## Example

### Parsing and export

```c
const char *src = "H Q0\nX2P Q1\nCZ Q0 Q1\n";
CCircuit *qc = qcis_loads(src);
if (!qc) { return 1; }

printf("qubits=%zu ops=%zu\n",
       (size_t)circuit_num_qubits(qc),
       (size_t)circuit_num_operations(qc));

char *out = qcis_dumps(qc);
printf("%s\n", out);
cqlib_string_free(out);

circuit_free(qc);
```

### File read/write and decomposition

```c
qcis_dump(qc, "input.qcis");              // 写文件

CCircuit *c = qcis_load("input.qcis");   // 读文件
CCircuit *c2 = circuit_decompose(c);     // 分解复合门

char *text = qcis_dumps(c2);
cqlib_string_free(text);
qcis_dump(c2, "output.qcis");

circuit_free(c2);
circuit_free(c);
```
