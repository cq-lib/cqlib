# CircuitGate / FrozenCircuit

- `cqlib.circuit.gates.FrozenCircuit`
- `cqlib.circuit.gates.CircuitGate`

```python
from cqlib.circuit.gates import FrozenCircuit, CircuitGate
```

`FrozenCircuit` and `CircuitGate` are used to wrap an already constructed circuit as a reusable composite gate. The two are usually used together: `FrozenCircuit` holds an immutable circuit definition, and `CircuitGate` wraps that definition into a gate object that can be appended to other circuits.

---

## `Circuit`, `FrozenCircuit` and `CircuitGate`

`Circuit`, `FrozenCircuit` and `CircuitGate` can be understood as three different levels:

| Type | Role | Typical use |
|---|---|---|
| `Circuit` | A mutable circuit container, to which gates, measurement, control flow and other operations can still be appended | Constructing and editing quantum circuits |
| `FrozenCircuit` | An immutable circuit snapshot, holding the structural definition of a sub-circuit | Serving as a stable definition of a composite gate or of low-level IR |
| `CircuitGate` | A composite gate defined by a `FrozenCircuit` | Reusing a sub-circuit as a single gate |

A typical flow is as follows:

```text
Circuit  ──to_gate(name)──>  CircuitGate  ──append_circuit_gate()──>  Circuit
                                │
                                └──内部持有 FrozenCircuit 定义
```

---

## Construction

A `CircuitGate` can be constructed directly by calling `Circuit.to_gate(name)`:

```python
Circuit.to_gate(name: str) -> CircuitGate
```

Example: wrap a Bell state preparation circuit as a composite gate and reuse it in a larger circuit.

```python
from cqlib import Circuit

bell = Circuit(2)
bell.h(0)
bell.cx(0, 1)

bell_gate = bell.to_gate("Bell")

circuit = Circuit(4)
circuit.append_circuit_gate(bell_gate, [0, 1])
circuit.append_circuit_gate(bell_gate, [2, 3])
```

In the example above, `bell.to_gate("Bell")` wraps the two-qubit Bell sub-circuit as a `CircuitGate` named `Bell`. The composite gate can then be appended to other circuits like an ordinary gate and act on the given qubits.

---

## FrozenCircuit

`FrozenCircuit` represents an immutable circuit snapshot. It holds the qubit order, the operation sequence and optional classical type information of a sub-circuit.

```python
FrozenCircuit(
    qubits: list[Qubit],
    operations: list[ValueOperation],
    classical_vars: list[ClassicalType] | None = None,
    classical_values: list[ClassicalType] | None = None,
)
```

| Parameter | Description |
|---|---|
| `qubits` | The storage order of qubits in the sub-circuit. This order affects the qubit mapping when the composite gate is applied. |
| `operations` | The self-contained operation sequence in the sub-circuit. |
| `classical_vars` | An optional table of classical variable types, usually used in lower-level IR scenarios. |
| `classical_values` | An optional table of classical value types, usually used in lower-level IR scenarios. |

Example:

```python
from cqlib import Circuit
from cqlib.circuit.gates import FrozenCircuit

sub = Circuit(2)
sub.h(0)
sub.cx(0, 1)

frozen = FrozenCircuit(sub.qubits, sub.operations)
```

| Attribute | Type | Description |
|---|---|---|
| `qubits` | `list[Qubit]` | The qubit order held in the sub-circuit definition. |
| `num_operations` | `int` | The number of operations the sub-circuit contains. |
| `operations` | `list[ValueOperation]` | The self-contained operation list in the sub-circuit. |
| `symbols` | `list[str]` | The symbol names registered inside the sub-circuit in insertion order, which may include symbols that are no longer referenced. |
| `used_symbols` | `list[str]` | The symbol names actually referenced after freezing. |

`FrozenCircuit.used_symbols` can be used to find out whether the sub-circuit is a parameterized circuit. Note the difference between `symbols` and `used_symbols`: the former is a stable symbol registry, in which symbols that are no longer referenced may remain after operations are removed or replaced; the latter contains only the symbols actually referenced at present. A `CircuitGate` constructed with the parameter signature omitted uses `used_symbols` as its positional parameter signature, so a composite gate generated from it also requires positional parameters to be passed in the order of `used_symbols` when it is applied.

---

## CircuitGate

`CircuitGate` is a composite gate defined by a `FrozenCircuit`. It wraps a sub-circuit as a gate object, so that it can be added to other circuits through `Circuit.append_circuit_gate()`.

```python
CircuitGate(
    name: str,
    circuit: FrozenCircuit,
    signature_params: list[str] | None = None,
) -> CircuitGate
```

| Parameter | Description |
|---|---|
| `name` | The composite gate name, mainly used for display, debugging and IR expression. |
| `circuit` | The immutable circuit snapshot used to define the composite gate. |
| `signature_params` | The positional parameter signature. When omitted, `circuit.used_symbols` is taken; when given explicitly, parameters are bound in the order of that list. An explicit signature may contain parameter names that the sub-circuit does not reference, and those parameters still count toward the number of parameters of the gate. |

Example:

```python
from cqlib.circuit.gates import CircuitGate

gate = CircuitGate("Bell", frozen)
```

| Attribute | Type | Description |
|---|---|---|
| `name` | `str` | The composite gate name. |
| `num_qubits` | `int` | The number of qubits to provide when applying the gate. |
| `num_params` | `int` | The number of positional parameters to provide when applying the gate, equal to the length of `signature_params`. |
| `signature_params` | `list[str]` | The positional parameter signature, which determines the binding order and the number of positional parameters when the gate is applied. |
| `used_symbols` | `list[str]` | The symbol names actually referenced in the sub-circuit on which the gate is based. |
| `symbols` | `list[str]` | A backward-compatible alias of `used_symbols`; it does not represent the binding order of positional parameters. |
| `circuit` | `FrozenCircuit` | The immutable circuit definition held inside the composite gate. |

---

## Explicit parameter signature

When the positional parameters of a gate need to be decoupled from the symbols actually appearing in the sub-circuit, the static method `CircuitGate.with_signature()` can be used to declare the call signature explicitly:

```python
CircuitGate.with_signature(
    name: str,
    circuit: FrozenCircuit,
    signature_params: list[str],
) -> CircuitGate
```

```python
from cqlib import Circuit, Parameter
from cqlib.circuit.gates import CircuitGate, FrozenCircuit

body = Circuit(1)
body.rx(0, Parameter("used"))

frozen_body = FrozenCircuit(body.qubits, body.operations)

gate = CircuitGate.with_signature("Declared", frozen_body, ["used", "declared_unused"])
```

When a signature is declared explicitly, duplicate parameter names are not allowed in `signature_params`; at the same time, every symbol actually referenced in the sub-circuit must already be declared in `signature_params`. Both cases raise `CircuitError`.

A parameter that is declared but not referenced by the sub-circuit still counts toward the number of parameters of the gate, so `gate.num_params` equals the length of `signature_params` rather than the length of `gate.used_symbols`.

---

## Parameterized sub-circuit gates

If a sub-circuit contains symbolic parameters, `CircuitGate` records those symbolic parameters and binds them positionally in the order of `gate.signature_params` when the gate is applied.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

block = Circuit(1)
block.rx(0, theta)
block.rz(0, theta / 2)

gate = block.to_gate("ParamBlock")

circuit = Circuit(1)
circuit.append_circuit_gate(gate, [0], params=[Parameter("alpha")])
```

In the example above, the sub-circuit `block` contains the symbolic parameter `theta`. After it is wrapped as `ParamBlock`, the `alpha` passed in when the composite gate is applied externally replaces the inner `theta` according to the positional parameter rule.

Note that the parameter binding of a `CircuitGate` is positional. That is, the `i`-th parameter passed in when applying the gate replaces the `i`-th symbol in `gate.signature_params`.

```python
print(gate.signature_params)
print(gate.num_params)
```

---

## Appending to a Circuit

A `CircuitGate` needs to be appended to a circuit through `Circuit.append_circuit_gate()`:

```python
Circuit.append_circuit_gate(
    gate: CircuitGate,
    qubits: list[int | Qubit],
    params: list[float | Parameter] | None = None,
) -> None
```

Example:

```python
from cqlib import Circuit, Parameter

alpha = Parameter("alpha")

sub = Circuit(1)
sub.rx(0, Parameter("theta"))

rx_block = sub.to_gate("RxBlock")

main = Circuit(1)
main.append_circuit_gate(rx_block, [0], params=[alpha])
```

When appending, note that:

- the number of `qubits` must equal `gate.num_qubits`;
- the number of `params` must equal `gate.num_params`;
- the order of `qubits` determines the mapping from the qubits inside the sub-circuit to the qubits of the outer circuit;

---

## Inverse gate

```python
CircuitGate.inverse() -> CircuitGate
```

`inverse()` returns a new `CircuitGate`. The underlying frozen circuit of the new gate is obtained by inverting the original circuit operation by operation and reversing the order, the name of the new gate has the suffix `_dg` appended to the original name, and the positional parameter signature stays unchanged.

```python
inverse_gate = gate.inverse()
```

This interface applies to invertible sub-circuits. If the sub-circuit definition contains measurement, `reset` or other non-invertible operations, for example classical data operations, control flow, and instructions that cannot be inverted, the inversion process raises `CircuitError`. Moreover, `inverse()` does not modify the original `CircuitGate` but returns a new composite gate definition.

---

## Decomposition

`Circuit.decompose()` can expand a `CircuitGate` in a circuit back into the base operation sequence of its inner definition.

```python
outer = Circuit(1)
outer.append_circuit_gate(gate, [0], params=[0.2])

expanded = outer.decompose()
```

After decomposition, the composite gate is replaced by its inner original operations. For a parameterized `CircuitGate`, the positional parameters passed in when it is applied are substituted into the inner operations during decomposition.

---

## Other behavior

- Both `FrozenCircuit` and `CircuitGate` support structural equality comparison with `==`. Comparison of `FrozenCircuit` ignores the circuit identity and the symbolic matrix cache and compares only the definition itself; comparison of `CircuitGate` includes the name, the ordered signature and the underlying definition circuit.
- `repr` has the form `FrozenCircuit(qubits=2, operations=2)` and `CircuitGate("Bell", qubits=2, params=0)`.
- Both support `copy` and `deepcopy`. Since both are immutable objects, the copy equals the original object.

