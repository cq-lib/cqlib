# Runtime classical state

`cqlib.qis.state`

`ClassicalState` and `RuntimeValue` describe the runtime classical data produced during circuit execution. `RuntimeValue` is the typed value of a single classical value in one execution; `ClassicalState` is a snapshot of all runtime classical data after one execution ends, with values queried by handle. Both are given by the simulator after execution completes and have no public constructor.

## Import

```python
from cqlib.qis.state import ClassicalState, RuntimeValue
```

---

## RuntimeValue

The runtime value of a single classical value. The kind of the value is distinguished by `kind`, and the static type is given by `ty`.

### Attributes

- `kind -> str`: the kind of the value, one of `"bit"`, `"bool"`, `"uint"` or `"bit_vec"`.
- `ty -> ClassicalType`: the classical type corresponding to this value.

### Methods

- `to_bitstring() -> str | None`: return a bitstring when the value is of kind `bit` or `bit_vec`, and `None` for the other kinds.
- `as_bit() -> bool`: read the value as a `bit`.
- `as_bool() -> bool`: read the value as a `bool`.
- `as_uint() -> int`: read the value as a `uint`.
- `as_bitvec_outcome() -> Outcome`: read the value as a `bit_vec` and return a measurement receipt.

### Other behavior

- Equality is determined by value: two values are equal only when the kind, bit width and payload are all the same; comparison with other types is always false.
- Equal values have the same hash, so they can be put into a set directly for deduplication.
- Supports `copy` and `deepcopy`.
- `repr` has the form `RuntimeValue.bit(true)`, `RuntimeValue.uint(width=8, value=3)`, `RuntimeValue.bit_vec(width=2, bits='10')`.

### Raises

- `TypeError`: raised when `as_bit`, `as_bool`, `as_uint` or `as_bitvec_outcome` is used with a mismatched value kind.

---

## ClassicalState

A snapshot of the runtime classical state after one circuit execution ends, with values queried by classical value handle or classical variable handle.

### Methods

- `value(value) -> RuntimeValue | None`: query the runtime value produced by this execution for an immutable classical value; returns `None` when there is no value. `value` is a `ClassicalValue`, usually taken from a measurement receipt.
- `var(var) -> RuntimeValue | None`: query the current runtime value of a mutable classical variable; returns `None` when there is no value. `var` is a `ClassicalVar`.

### Other behavior

- No public constructor; can only be obtained from an execution result.
- Supports `copy` and `deepcopy`.
- `repr` is always `ClassicalState()`.

---

## Example

### Query the value produced by a measurement

```python
from cqlib.circuit import Circuit
from cqlib.qis.state import StabilizerState

circuit = Circuit(2)
circuit.x(0)
measurement = circuit.measure(0)
circuit.reset(0)
circuit.h(1)

result = StabilizerState.run_circuit(circuit)
measured = result.classical.value(measurement.value)
assert measured.kind == "bit"
assert measured.as_bit() is True
assert measured.to_bitstring() == "1"
```

### Query a mutable classical variable

```python
from cqlib.circuit import Circuit, ClassicalExpr, ClassicalType
from cqlib.qis.state import StabilizerState

circuit = Circuit(1)
flag = circuit.var(ClassicalType.bool())
circuit.store(flag, ClassicalExpr.bool_literal(True))

result = StabilizerState.run_circuit(circuit)
stored = result.classical.var(flag)
assert stored.kind == "bool"
assert stored.as_bool() is True
assert stored.to_bitstring() is None
```

### Value comparison and deduplication

Values of the same kind and the same payload are equal and have the same hash, so they can be used directly for set deduplication; values of different kinds are not equal even when they are semantically close.

```python
from cqlib.circuit import Circuit
from cqlib.qis.state import StabilizerState


def measured_bit():
    circuit = Circuit(1)
    circuit.x(0)
    measurement = circuit.measure(0)
    return StabilizerState.run_circuit(circuit).classical.value(measurement.value)


first = measured_bit()
second = measured_bit()
assert first == second
assert hash(first) == hash(second)
assert len({first, second}) == 1
```

---

## Validation and error handling

| Exception | When it occurs |
| --- | --- |
| `TypeError` | Reading a `RuntimeValue` with a mismatched accessor, for example calling `as_uint()` on a value of kind `bit`. |

The definitions of classical types, classical variables and classical value handles are given in [Classical / Control Flow](../0_circuit/9_classical_control_flow.md); the definition of `Outcome` is given in [Result](../2_device/5_result.md).
