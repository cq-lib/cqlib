# qcis

`cqlib.ir.qcis` provides a bidirectional conversion interface between QCIS text and `Circuit`.

## Import

```python
from cqlib.ir import qcis
```

---

## Functions

### qcis.loads(qcis)

Parse a circuit from a QCIS string.

Parameters:

- `qcis` (`str`): the QCIS text.

Returns:

- `Circuit`

Raises:

- `ValueError`: invalid QCIS syntax or parse failure (`QCIS parse error: ...`).

Example:

```python
from cqlib.ir import qcis

qcis_code = """H Q0
CZ Q0 Q1
"""
circuit = qcis.loads(qcis_code)
```

### qcis.load(path)

Parse a circuit from a QCIS file.

Parameters:

- `path` (`str`): the QCIS file path.

Returns:

- `Circuit`

Raises:

- `OSError`: file read failure (nonexistent file, insufficient permissions, and so on).
- `ValueError`: QCIS parsing of the file content failed (`QCIS parse error: ...`).

Description:

- `load` reads a "file path";
- `loads` reads "string content".

### qcis.dumps(circuit)

Serialize a circuit into a QCIS string.

Parameters:

- `circuit` (`Circuit`)

Returns:

- `str`

Raises:

- `ValueError`: export failure (commonly an unsupported gate; error prefix `QCIS dump error: ...`).

### qcis.dump(circuit, path)

Serialize a circuit and write it to a QCIS file.

Parameters:

- `circuit` (`Circuit`)
- `path` (`str`)

Returns:

- `None`

Raises:

- `OSError`: file write failure.
- `ValueError`: export failure (for example the circuit contains a gate QCIS cannot express; error prefix `QCIS dump error: ...`).

## Notes

`qcis.dumps` / `qcis.dump` only support the QCIS native gate set. A non-native gate raises an error, and the circuit must first be decomposed/compiled into the QCIS gate set before exporting.

The current QCIS native gate set is:

- Single-qubit gates: `H`, `S`, `SD`, `T`, `TD`, `X`, `X2P`, `X2M`, `Y`, `Y2P`, `Y2M`, `Z`
- Parameterized single-qubit gates: `RX`, `RXY`, `RY`, `RZ`, `U`, `XY`, `XY2P`, `XY2M`, `PHASE`
- Multi-qubit gates: `RXX`, `RYY`, `RZX`, `RZZ`, `SWAP`, `CX`, `CCX`, `CY`, `CZ`, `CRX`, `CRY`, `CRZ`, `FSIM`

Besides gates, QCIS text also represents measurement (`M`), barrier (`B`) and delay (`I Qn t`, where `t` is the number of delay ticks in units of 0.5 ns and must be a non-negative integer).

`SDG` and `TDG` are accepted during parsing as equivalent spellings of `SD` and `TD`, and are uniformly written as `SD` and `TD` when exporting. Standard identity gates and global phase are not inside the QCIS native gate set and raise an error when exporting; `I` in QCIS always means a delay, not an identity gate.

## Example

```python
from cqlib.ir import qcis

qcis_code = """H Q0
CZ Q0 Q1
"""
circuit = qcis.loads(qcis_code)

print(qcis.dumps(circuit))

qcis.dump(circuit, "input.qcis")

# 1) 读文件
c = qcis.load("input.qcis")

# 2) 处理电路（示例：分解）
c2 = c.decompose()

# 3) 导出文本或文件
text = qcis.dumps(c2)
qcis.dump(c2, "output.qcis")
```
