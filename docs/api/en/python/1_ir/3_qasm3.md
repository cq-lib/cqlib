# qasm3

The `qasm3` submodule under `cqlib.ir` provides a bidirectional conversion interface between OpenQASM 3.0 and `Circuit`.

## Import

```python
from cqlib.ir import qasm3
```

---

## Functions

### qasm3.loads(qasm_text)

Parse a circuit from an OpenQASM 3.0 string.

Parameters:

- `qasm_text` (`str`): the QASM text.

Returns:

- `Circuit`

Raises:

- `ValueError`: a syntax error, a semantic error, or use of a feature the current implementation does not support (`QASM3 parse error: ...`).

Example:

```python
from cqlib.ir import qasm3

code = """OPENQASM 3;
include "stdgates.inc";
qubit[2] q;
h q[0];
cx q[0], q[1];
"""

circuit = qasm3.loads(code)
```

### qasm3.load(path)

Read and parse a circuit from a QASM file.

Parameters:

- `path` (`str`): the file path.

Returns:

- `Circuit`

Raises:

- `OSError`: file read failure (nonexistent file, insufficient permissions, and so on).
- `ValueError`: parsing of the file content failed (`QASM3 load error: ...`).

Description:

- `load` reads a "file path", and `loads` reads "string content".
- An `include` with a relative path is resolved from the directory of the input file.

### qasm3.dumps(circuit)

Export a circuit as an OpenQASM 3.0 string.

Parameters:

- `circuit` (`Circuit`)

Returns:

- `str`

Raises:

- `ValueError`: export failure (`QASM3 dump error: ...`).

Description:

- The output is canonicalized text; whitespace, comments and variable names of the original input are not preserved.
- The content is, in order: the header, the standard library inclusion, extension gate definitions, custom gate definitions, quantum and classical declarations, and the main circuit operations.

### qasm3.dump(circuit, path)

Export a circuit as a QASM file.

Parameters:

- `circuit` (`Circuit`)
- `path` (`str`): the output path.

Returns:

- `None`

Raises:

- `OSError`: file write failure.
- `ValueError`: export failure (`QASM3 dump error: ...`).

## Supported features

The `qasm3` interface currently supports:

- The headers `OPENQASM 3;` and `OPENQASM 3.0;`
- The standard library inclusion `include "stdgates.inc";`; parsing uses a built-in standard library and does not require the file to exist in the directory
- Quantum declarations: the scalar `qubit q;` and the one-dimensional register `qubit[2] q;`
- Classical declarations: `bit`, `bit[n]`, `bool`, `uint[n]`
- Angle input declarations such as `input angle[64] theta;`, taken as symbolic parameters of the circuit
- Standard gates mapped to `StandardGate`: `id`, `i`, `x`, `y`, `z`, `h`, `s`, `sdg`, `t`, `tdg`, `rx`, `ry`, `rz`, `p`, `phase`, `u1`, `u`, `U`, `u3`, `u2`, `cx`, `cy`, `cz`, `swap`, `ccx`, `crx`, `cry`, `crz`, plus `sx`, `sxdg` (corresponding to X2P and X2M respectively)
- Cqlib extension gates: `x2p`, `x2m`, `y2p`, `y2m`, `xy2p`, `xy2m`, `rxx`, `ryy`, `rzz`, `rzx`, `fsim`, `gphase`
- Definition and invocation of custom `gate`s, including custom gates with parameters
- Measurement: `c = measure q;`, `v[0] = measure q[0];`, and the standalone `measure q;`
- The instructions `reset` and `barrier`
- Global phase `gphase(theta);`
- Control flow: `if` / `else`, statically expandable `for`, and `switch` with constant branches

## Differences from the OpenQASM 2.0 interface

On top of the same `Circuit` representation, the main differences between this interface and the OpenQASM 2.0 interface:

| Aspect | OpenQASM 2.0 interface | OpenQASM 3.0 interface |
|---|---|---|
| Header and standard library | `OPENQASM 2.0;`, `include "qelib1.inc";` | `OPENQASM 3;` or `OPENQASM 3.0;`, `include "stdgates.inc";` |
| Quantum declarations | `qreg q[2];` | `qubit[2] q;`, and also supports the scalar `qubit q;` |
| Classical declarations | `creg c[2];` | `bit`, `bit[n]`, `bool`, `uint[n]` |
| Measurement | `measure q[0] -> c[0];` | `c[0] = measure q[0];`, `c = measure q;` |
| Control flow | `if (c == 1) ...` followed by a single statement | `if` / `else` blocks, statically expandable `for`, and `switch` with constant branches |
| Global phase | Given only as a comment when exporting; cannot be loaded | `gphase(theta);`, loadable and exportable |
| Input declarations | None | Angle and floating-point `input` declarations |

## Limitations

The following OpenQASM 3.0 features are not supported in the current implementation; a clear error is given when they are encountered, with no partial fallback:

- Timing and pulses: `delay`, `box`, `cal`, `defcal`
- Subroutines and external calls: `def` subroutines, `extern`
- Hardware-related: hardware qubits, `output` declarations, and `input` declarations other than angle and floating-point
- Other statements: `alias`, `pragma`, annotations, legacy declarations
- Gate modifiers: `negctrl`, `pow`; `ctrl` and `inv` are available only when a corresponding implementation exists for the target gate
- Some standard gates: `ch`, `cp`, `cu`, `cswap`
- Control flow: a lowering path for `while` exists in the implementation, but loops with a runtime condition are currently rejected by the semantic check; `for` supports only the statically expanded form whose range is constant
- Complex classical arithmetic, complex lvalue slicing, multi-dimensional indexing
- Qubit declarations inside a scope

The export side has further constraints:

- `delay`, matrix-only `UnitaryGate`, plain assignments writing to a scalar `bit` variable, and measurement inside a gate definition body are not exported
- Control flow such as `while`, `for`, `break`, `continue` is not exported
- The two standard gates `XY` and `RXY` are not exported

## Round-trip behavior

The export result can be loaded again by this interface. Canonicalization rules:

- The header is fixed to `OPENQASM 3.0;` and `include "stdgates.inc";`, regardless of how the header was written in the input.
- Top-level qubits are uniformly exported as `qubit[n] q`; the single-qubit case is likewise written as `qubit[1] q`.
- User classical variables are named `c0`, `c1`, ..., and immutable measurement values are named `v0`, `v1`, ....
- A measurement immediately followed by a compatible store is folded into assignment form, such as `c0 = measure q;`.
- A standalone measurement is written into the automatically generated `bit[k] meas` register, such as `meas[0] = measure q[0];`.
- A non-contiguous or permuted register measurement is split into per-bit assignments, such as `c0[0] = measure q[2];`.
- Gates already provided by `stdgates.inc` are called directly; an extension gate generates a `gate` definition once before the main circuit, and its name is still used at the call site.
- Global phase is exported as a `gphase(...);` statement and enters the global phase of the circuit when loaded again.
- Automatically generated names avoid the gate names and register names already present in the circuit.

Example:

```python
from cqlib.ir import qasm3

code = """OPENQASM 3;
include "stdgates.inc";
qubit[2] q;
h q[0];
cx q[0], q[1];
"""

circuit = qasm3.loads(code)
print(qasm3.dumps(circuit))
```

Output:

```text
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;

h q[0];
cx q[0],q[1];
```

## Common workflow

```python
from cqlib.ir import qasm3

code = """OPENQASM 3;
include "stdgates.inc";
qubit[2] q;
h q[0];
cx q[0], q[1];
"""

circuit = qasm3.loads(code)
print(qasm3.dumps(circuit))

qasm3.dump(circuit, "input.qasm")

# 1) 读取 QASM 文件
c = qasm3.load("input.qasm")

# 2) 处理电路（示例）
c2 = c.decompose()

# 3) 导出为字符串或文件
text = qasm3.dumps(c2)
qasm3.dump(c2, "output.qasm")
```
