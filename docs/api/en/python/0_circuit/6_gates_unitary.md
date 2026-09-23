# UnitaryGate

`cqlib.circuit.gates.UnitaryGate`  

```python
from cqlib.circuit.gates import UnitaryGate
```

`UnitaryGate` is used to define user-defined unitary gates outside the Cqlib standard gate set. Unlike `StandardGate`, the behavior of a `UnitaryGate` is usually described by a matrix, a symbolic matrix or a frozen circuit provided by the user, so it is better suited to representing special oracles in an algorithm, hardware-related calibration gates, problem-specific black-box transformations, or other custom quantum operations that cannot be expressed directly with built-in standard gates.

---

## Constructor

```python
UnitaryGate(label: str, num_qubits: int, num_params: int = 0)
```

| Parameter | Description |
| --- | --- |
| `label` | The readable name of the gate, commonly used for printing, debugging, export and visualization. |
| `num_qubits` | The number of qubits the gate acts on. |
| `num_params` | The number of positional parameters to pass in each time the gate is applied; the default is `0`. |

```python
from cqlib.circuit.gates import UnitaryGate

oracle = UnitaryGate("Oracle", num_qubits=2)
```

The constructor only creates a custom gate object with a name, a qubit count and a parameter count. After creation, a concrete definition can be attached to the gate through `with_matrix()`, `with_symbolic_matrix()` or `with_circuit()`.

---

## Numeric matrix definition

```python
with_matrix(matrix: ArrayLike) -> UnitaryGate
```

`with_matrix()` defines a custom gate through a numeric matrix. The matrix passed in must be a two-dimensional square matrix with shape `2**num_qubits × 2**num_qubits`. For example, a single-qubit gate corresponds to a `(2, 2)` matrix, a two-qubit gate to a `(4, 4)` matrix, and a three-qubit gate to an `(8, 8)` matrix.

```python
import numpy as np
from cqlib import Circuit
from cqlib.circuit.gates import UnitaryGate

z_like = np.array([[1, 0], [0, -1]], dtype=np.complex128)
gate = UnitaryGate("ZLike", 1).with_matrix(z_like)

circuit = Circuit(1)
circuit.append_unitary_gate(gate, [0])
```

---

## Symbolic matrix definition

```python
with_symbolic_matrix(matrix: SymbolicMatrix, params: list[str]) -> UnitaryGate
```

`with_symbolic_matrix()` defines a parameterized custom unitary gate; the symbolic matrix can keep `Parameter` expressions in its matrix elements, so that the same gate definition can bind different parameter values at different circuit positions.

Here, `matrix` is the symbolic matrix definition of the gate and `params` specifies the order of positional parameters when the gate is applied. Parameters passed in through `Circuit.append_unitary_gate(..., params=[...])` when appending the gate are bound into the symbolic matrix in the order of the names given in `params`.

```python
from cqlib import Parameter
from cqlib.circuit import SymbolicComplex, SymbolicMatrix
from cqlib.circuit.gates import UnitaryGate

theta = Parameter("theta")
matrix = SymbolicMatrix([
    [SymbolicComplex.one(), SymbolicComplex.zero()],
    [SymbolicComplex.zero(), SymbolicComplex.exp_i(theta)],
])

phase_like = UnitaryGate("PhaseLike", 1, num_params=1).with_symbolic_matrix(
    matrix,
    ["theta"],
)
```

In the example above, `PhaseLike` is a single-qubit phase gate with one positional parameter. The symbolic matrix uses `theta` to represent the phase parameter, and `with_symbolic_matrix(..., ["theta"])` means that the first parameter is bound to the symbol `theta` when the gate is appended.

When the gate is applied, either a numeric parameter or a new `Parameter` expression can be passed in:

```python
from cqlib import Circuit, Parameter

circuit = Circuit(1)
circuit.append_unitary_gate(phase_like, [0], params=[Parameter("phi")])
```

Here `phi` is the positional parameter passed in when the gate is applied, and it replaces `theta` in the symbolic matrix definition. If a number is passed in, for example `params=[0.25]`, it means this application uses a concrete numeric phase.

---

## Frozen circuit definition

```python
with_circuit(circuit: FrozenCircuit) -> UnitaryGate
```

When a custom gate can be described by an existing circuit structure, a `FrozenCircuit` can be used as the gate definition. This form suits keeping circuit-level structural information in `UnitaryGate` form, so that the inner operations can still be referenced or expanded later when needed.

```python
from cqlib import Circuit
from cqlib.circuit.gates import FrozenCircuit, UnitaryGate

sub = Circuit(2)
sub.h(0)
sub.cx(0, 1)

frozen = FrozenCircuit(sub.qubits, sub.operations)
gate = UnitaryGate("BellPrep", 2).with_circuit(frozen)
```

To add a circuit to other circuits as a reusable composite gate, use:

```python
bell_gate = sub.to_gate("BellPrep")
```


| Attribute | Type | Description |
| --- | --- | --- |
| `label` | `str` | The readable name of the gate. |
| `num_qubits` | `int` | The number of qubits the gate acts on. |
| `num_params` | `int` | The number of positional parameters to pass in each time the gate is applied. |
| `symbolic_matrix` | `SymbolicMatrix / None` | The symbolic matrix definition; usually `None` if no symbolic matrix definition was used. |
| `matrix_params` | `list[str] / None` | The order of parameter names in the symbolic matrix. |
| `circuit` | `FrozenCircuit / None` | The frozen circuit definition; usually `None` if no circuit definition was used. |

---

## Matrix evaluation

```python
matrix() -> np.ndarray
matrix_for_params(params: list[float]) -> np.ndarray
```

`matrix()` returns the numeric matrix attached by `with_matrix()`. If the gate has not been defined yet, or was defined with `with_symbolic_matrix()` or `with_circuit()` instead, there is no numeric matrix that can be returned directly, and `CircuitError` is raised.

`matrix_for_params(params)` computes the numeric matrix of the gate under the given parameters, and supports all three definition forms: numeric matrix, symbolic matrix and frozen circuit; `params` are provided in the order given by `matrix_params`.

```python
import numpy as np
from cqlib.circuit.gates import UnitaryGate

z_like = np.array([[1, 0], [0, -1]], dtype=np.complex128)
gate = UnitaryGate("ZLike", 1).with_matrix(z_like)

assert gate.matrix().shape == (2, 2)
assert np.array(gate).shape == (2, 2)
```

`UnitaryGate` also implements the array protocol, so the matrix representation can be obtained directly through `np.array(gate)` or `np.asarray(gate)`.

---

## Appending to `Circuit`

```python
Circuit.append_unitary_gate(
    gate: UnitaryGate,
    qubits: list[int | Qubit],
    params: list[float | Parameter] | None = None,
) -> None
```

Once defined, a `UnitaryGate` can be appended to a circuit through `Circuit.append_unitary_gate()`. Appending requires the list of qubits the gate acts on; if the gate is parameterized, the positional parameters of this application are also required.

Appending must satisfy the following conditions:

- the number of `qubits` must equal `gate.num_qubits`;
- if `gate.num_params > 0`, the length of `params` should agree with `gate.num_params`;
- the qubits acted on must already exist in the target circuit.

---

## Comparison of definition forms

| Definition form | Preserves structure | Supports symbolic parameters | Typical use |
| --- | --- | --- | --- |
| `with_matrix()` | No | No | Fixed black-box matrices, oracles, calibration gates, small-scale test gates. |
| `with_symbolic_matrix()` | No | Yes | Parameterized black-box matrices, tunable phase gates, symbolic verification. |
| `with_circuit()` | Yes | Depends on the inner circuit | Scenarios that need to preserve decomposition information while applying the gate in `UnitaryGate` form. |

