# MCGate

`cqlib.circuit.gates.MCGate`  

```python
from cqlib.circuit.gates import MCGate
```

`MCGate` represents a multi-controlled quantum gate, that is, one or more control qubits added on top of an existing `StandardGate`. The base gate acts on the corresponding target qubits only when all control qubits satisfy the control condition. Multi-controlled gates are commonly used to construct Toffoli gates, multi-controlled phase gates, oracles, conditional flip operations and controlled subroutines in quantum algorithms.

---

## Constructor

```python
MCGate(num_controls: int, gate: StandardGate)
```

| Parameter | Description |
| --- | --- |
| `num_controls` | The number of additional control qubits to add. |
| `gate` | The base standard gate being controlled, that is, the gate actually executed when the control condition is satisfied. |

The following example constructs a doubly controlled `X` gate and a triply controlled `H` gate:

```python
from cqlib.circuit.gates import MCGate, StandardGate

ccx = MCGate(2, StandardGate.X)
mch = MCGate(3, StandardGate.H)
```

Alternatively, the `control()` method can be called directly on a standard gate object to generate a multi-controlled gate.

```python
from cqlib.circuit.gates import StandardGate

mcx = StandardGate.X.control(4)
```

In the example above, `mcx` represents a quadruply controlled `X` gate. Applying this gate requires 4 control qubits and 1 target qubit.

---

## Attributes

| Attribute | Type | Description |
| --- | --- | --- |
| `num_ctrl_qubits` | `int` | The total number of control qubits of the multi-controlled gate. |
| `num_qubits` | `int` | The total number of qubits the gate acts on, equal to the number of control qubits plus the number of qubits the base gate acts on. |
| `num_params` | `int` | The number of parameters the base gate requires. |
| `base_gate` | `StandardGate` | The base standard gate before controls were added. |
| `params` | `list[Parameter]` | The parameters currently bound on the base gate. |

```python
from cqlib import Parameter
from cqlib.circuit.gates import MCGate, StandardGate

theta = Parameter("theta")
gate = MCGate(2, StandardGate.RZ(theta))

assert gate.num_ctrl_qubits == 2
assert gate.num_qubits == 3
assert gate.num_params == 1
```

In the example above, the base gate is a single-qubit `RZ(theta)` gate and 2 control qubits are added outside it, so this `MCGate` acts on 3 qubits in total: the first two are control qubits and the last is the target qubit of `RZ`.

---

## Order of control and target qubits

When an `MCGate` is appended to a circuit, qubits must be passed in the order of control qubits first and target qubits last:

```text
[control_0, control_1, ..., control_k, target_0, target_1, ...]
```

For example, `MCGate(2, StandardGate.X)` is equivalent to a two-control `X` gate in Toffoli form.

```python
from cqlib import Circuit
from cqlib.circuit.gates import MCGate, StandardGate

circuit = Circuit(3)

ccx = MCGate(2, StandardGate.X)
circuit.append_mc_gate(ccx, [0, 1, 2])
```

In the example above, qubits `0` and `1` are control qubits and qubit `2` is the target qubit. The `X` gate acts on the target qubit only when the control qubits satisfy the control condition.

---

## Matrix

```python
matrix(params: list[float] | None = None) -> np.ndarray[np.complex128]
```

`matrix()` returns the numeric unitary matrix of the multi-controlled gate. The matrix dimension is determined by `num_qubits` and equals `2**num_qubits × 2**num_qubits`. For a parameterized base gate, concrete values can be passed through the `params` argument to compute the matrix temporarily.

```python
import numpy as np
from cqlib.circuit.gates import MCGate, StandardGate

gate = MCGate(1, StandardGate.RZ)
matrix = gate.matrix([np.pi / 2])

assert matrix.shape == (4, 4)
```

In the example above, `gate` is a singly controlled `RZ` gate acting on 2 qubits in total, so the matrix size is `4 × 4`.

Note that `matrix()` returns the local matrix of the gate itself; it does not include the global phase of the enclosing `Circuit` and does not take other operations in the circuit into account.

The number of parameters must agree with `num_params` when calling: `CircuitError` is raised when the length of the explicitly passed `params` does not equal `num_params`, or when `params` is not passed and the parameters on the gate have not all been bound to concrete values. Numeric parameters passed in must be finite, otherwise `ParameterError` is raised.

---

## Inverse gate

```python
inverse() -> MCGate
```

`inverse()` returns the inverse gate of the current multi-controlled gate. The inverse of a multi-controlled gate can be understood as: keep the control structure unchanged and replace only the base gate with its inverse. In other words, the inverse of a multi-controlled gate is equivalent to "controlling the inverse of the base gate".

```python
from cqlib.circuit.gates import MCGate, StandardGate

gate = MCGate(1, StandardGate.S)
inverse = gate.inverse()

assert inverse.base_gate == StandardGate.SDG
```

In the example above, the inverse of the `S` gate is `SDG`, so the inverse of a singly controlled `S` gate is a singly controlled `SDG` gate. For self-inverse gates, for example `X`, `Z` and `H`, the inverse of their multi-controlled form is usually the same as the gate itself.

Calling `inverse()` requires the parameters on the gate to be fully bound: `CircuitError` is raised when the number of bound parameters does not equal `num_params`; `RuntimeError` is raised when the base gate itself is not invertible.

---

## Relation to standard controlled gates

Some standard gates in Cqlib already represent common controlled gates. For example, `CX` is a singly controlled `X` gate, `CZ` is a singly controlled `Z` gate, and `CCX` is a doubly controlled `X` gate. Semantically, these standard gates can all be regarded as special cases of `MCGate`.

| Standard gate | Equivalent form |
| --- | --- |
| `StandardGate.CX` | `MCGate(1, StandardGate.X)` |
| `StandardGate.CY` | `MCGate(1, StandardGate.Y)` |
| `StandardGate.CZ` | `MCGate(1, StandardGate.Z)` |
| `StandardGate.CCX` | `MCGate(2, StandardGate.X)` |
| `StandardGate.CRX(theta)` | `MCGate(1, StandardGate.RX(theta))` |
| `StandardGate.CRY(theta)` | `MCGate(1, StandardGate.RY(theta))` |
| `StandardGate.CRZ(theta)` | `MCGate(1, StandardGate.RZ(theta))` |

---

## Base gates that already carry control qubits

If the base gate itself already contains control qubits, such as `StandardGate.CX`, adding control qubits with `MCGate` stacks the new control qubits on top of the existing control structure.

```python
from cqlib.circuit.gates import MCGate, StandardGate

gate = MCGate(1, StandardGate.CX)

assert gate.num_ctrl_qubits == 2
assert gate.num_qubits == 3
```

In the example above, `StandardGate.CX` already contains 1 control qubit and 1 target qubit. `MCGate(1, StandardGate.CX)` adds 1 more control qubit outside it, so the whole thing is a three-qubit gate with 2 control qubits and 1 target qubit.

---

## `MCGate` and `Circuit`

To add an `MCGate` to a circuit, use `Circuit.append_mc_gate()`. This interface checks whether the number of qubits passed in agrees with `gate.num_qubits`, and checks legality such as parameters and duplicate qubit references when necessary.

```python
from cqlib import Circuit
from cqlib.circuit.gates import MCGate, StandardGate

circuit = Circuit(4)

mcx = MCGate(3, StandardGate.X)
circuit.append_mc_gate(mcx, [0, 1, 2, 3])
```

---

## Other behavior

- Supports `==` and `hash`; comparison and hashing take the control structure, the base gate and the bound parameters into account at the same time, so an `MCGate` can be used as a dictionary key or a set element.
- Supports `copy` and `deepcopy`.
- `repr` can be evaluated directly to reconstruct the original gate: without parameters it has the form `MCGate(2, StandardGate.X)`, and with parameters it has the form `MCGate(1, StandardGate.RX(1.5))`.
