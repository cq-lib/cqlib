# qasm2

`cqlib.ir.qasm2` provides a bidirectional conversion interface between OpenQASM 2.0 and `Circuit`.

## Import

```python
from cqlib.ir import qasm2
```

---

## Functions

### qasm2.loads(qasm)

Parse a circuit from an OpenQASM 2.0 string.

Parameters:

- `qasm` (`str`): the QASM text.

Returns:

- `Circuit`

Raises:

- `ValueError`: invalid QASM syntax or parse failure (`QASM parse error: ...`).

Example:

```python
from cqlib.ir import qasm2

code = """OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
cz q[0],q[1];
"""

circuit = qasm2.loads(code)
```

### qasm2.load(path)

Read and parse a circuit from a QASM file.

Parameters:

- `path` (`str`): the file path.

Returns:

- `Circuit`

Raises:

- `OSError`: file read failure (`QASM load error: ...`).
- `ValueError`: parsing of the file content failed (`QASM load error: ...`).

### qasm2.dumps(circuit)

Export a circuit as an OpenQASM 2.0 string.

Parameters:

- `circuit` (`Circuit`)

Returns:

- `str`

Raises:

- `ValueError`: export failure (`QASM dump error: ...`).

Description:

- The output contains the standard header, for example:
`OPENQASM 2.0;`, `include "qelib1.inc";`, `qreg`/`creg` declarations.

### qasm2.dump(circuit, path)

Export a circuit as a QASM file.

Parameters:

- `circuit` (`Circuit`)
- `path` (`str`): the output path.

Returns:

- `None`

Raises:

- `OSError`: file write failure (`QASM dump error: ...`).
- `ValueError`: export failure (`QASM dump error: ...`).

## Supported features

The `qasm2` interface currently supports:

- Common standard gates: `h/x/y/z/s/sdg/t/tdg/id`, `cx/cy/cz/swap/ccx`, and so on
- Parameterized gates: `rx/ry/rz/u1/u2/u3/p`, plus `sx/sxdg/crx/cry/crz/rxx/ryy/rzz/fsim` and `ch/cu1/cu3`
- Parameter expressions: such as `pi`, `pi/2`, `3*pi/4`; loading also accepts `1e-5` and `asin`, `acos`, `atan`
- Instructions: `measure`, `barrier`, `reset`
- Declarations and definitions: `qreg`, `creg`, `opaque`, plus `gate` custom gate definitions and `if (creg == integer) qop;` conditional statements
- Export and loading of custom gates (`CircuitGate`); a unitary gate carrying only a matrix is exported as an `opaque` declaration

`include "qelib1.inc";` is treated as a built-in inclusion and does not require the file to exist locally; the qelib1 gate names are likewise available when `include` is not written, and cannot be redefined.

## Limitations

Export makes trade-offs according to the expressive power of OpenQASM 2.0; the following constructs raise an error:

- `else` branches in classical control, loops, `switch` and nested control flow
- Classical types that cannot be represented as `creg`, such as `bool` and unsigned integers
- Measurement and reset inside a `gate` definition body
- Bitwise conditions (such as `if (c[0] == 1)`) and conditional barriers

`rzx` appears only in the extension gate definitions on the export side and is not accepted when loading; global phase is not written as a statement but output only as a comment, and is not restored when the text is loaded again.

## Common workflow

```python
from cqlib.ir import qasm2

code = """OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
cz q[0],q[1];
"""

circuit = qasm2.loads(code)
print(qasm2.dumps(circuit))

qasm2.dump(circuit, "input.qasm")

# 1) 读取 QASM 文件
c = qasm2.load("input.qasm")

# 2) 处理电路（示例）
c2 = c.decompose()

# 3) 导出为字符串或文件
text = qasm2.dumps(c2)
qasm2.dump(c2, "output.qasm")
```
