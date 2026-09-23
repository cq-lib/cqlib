# NoiseModel / Noise Channels

This page covers the APIs in `cqlib.device` related to noise modeling:

- `SingleQubitNoise`
- `TwoQubitNoise`
- `ReadoutError`
- `OperationKey`
- `NoiseModel`

## Import

```python
from cqlib.circuit import StandardGate
from cqlib.device import (
    NoiseModel,
    OperationKey,
    ReadoutError,
    SingleQubitNoise,
    TwoQubitNoise,
)
```

---

## SingleQubitNoise

### Static constructors

- `bit_flip(p) -> SingleQubitNoise`: bit flip channel, applying X with probability `p`.
- `phase_flip(p) -> SingleQubitNoise`: phase flip channel, applying Z with probability `p`.
- `pauli(px, py, pz) -> SingleQubitNoise`: general Pauli channel; the three probabilities must satisfy `px + py + pz <= 1`.
- `depolarizing(p) -> SingleQubitNoise`: depolarizing channel, applying a random Pauli error with probability `p`, with each of the three Paulis taking `p/3`.
- `amplitude_damping(gamma) -> SingleQubitNoise`: amplitude damping channel.
- `phase_damping(lambda_) -> SingleQubitNoise`: phase damping channel.

Raises:

- `ValueError`: a probability is outside `[0, 1]` or is not a finite value; the sum of the three probabilities of `pauli` exceeds 1.

### Methods and attributes

- `is_valid() -> bool`: whether the noise parameters are physically valid.
- `to_kraus() -> list[numpy.ndarray]`: the list of Kraus operators, each a 2×2 complex matrix.

Description:

- The channel kind is determined by the static constructor; there is no `kind` attribute.

### Other behavior

- Supports `==`: two channels are equal when the static constructor and the parameters are the same.
- `repr(SingleQubitNoise.bit_flip(0.01)) == "SingleQubitNoise.bit_flip(0.01)"`.

## TwoQubitNoise

### Static constructors

- `depolarizing(p) -> TwoQubitNoise`: two-qubit depolarizing channel.
- `independent(q0_noise, q1_noise) -> TwoQubitNoise`: the independent composition of two single-qubit channels.
- `correlated_pauli(op_q0, op_q1, p) -> TwoQubitNoise`: correlated Pauli channel; `op_q0` / `op_q1` are `Pauli` objects, for example `Pauli.x()`, `Pauli.z()`.

Raises:

- `ValueError`: `p` is outside `[0, 1]` or is not a finite value.
- `TypeError`: `op_q0` / `op_q1` is not a `Pauli` object.

### Methods and attributes

- `is_valid() -> bool`
- `to_kraus() -> list[numpy.ndarray]`: the list of Kraus operators, each a 4×4 complex matrix.
- `kind -> str`: `depolarizing` / `independent` / `correlated_pauli`.

### Other behavior

- Supports `==`.

## ReadoutError

### `ReadoutError(p_0_given_1, p_1_given_0)`

Parameters:

- `p_0_given_1` (`float`): the probability of reading 0 when the true state is 1.
- `p_1_given_0` (`float`): the probability of reading 1 when the true state is 0.

Raises:

- `ValueError`: a probability is outside `[0, 1]` or is not a finite value.

### Attributes

- `p_0_given_1 -> float`
- `p_1_given_0 -> float`

### Methods

- `is_valid() -> bool`

### Other behavior

- Supports `==`.
- `repr(ReadoutError(0.1, 0.2)) == "ReadoutError(p_0_given_1=0.1, p_1_given_0=0.2)"`.

## OperationKey

### Static constructors

- `new_single(gate, q0) -> OperationKey`
- `new_double(gate, q0, q1) -> OperationKey`
- `new_triple(gate, q0, q1, q2) -> OperationKey`

Raises:

- `ValueError`: a multi-qubit gate has duplicate qubits.

### Attributes

- `gate -> StandardGate`: the gate type. The key records only the gate type; the object returned for a parameterized gate has parameters all equal to 0.
- `qubits -> list[int]`: the qubit indices involved in the operation.

Description:

- `__eq__` and `__hash__` are implemented, so it can be used as a dictionary key; keys are unequal when the gate or the qubit combination differs.
- `repr(OperationKey.new_single(StandardGate.X, 0)) == "OperationKey(gate=X, qubits=[0])"`.

## NoiseModel

### `NoiseModel()`

### Write methods

- `add_readout_error(qubit, error) -> None`: register a readout error for a qubit.
- `add_single_qubit_error(gate, qubit, noise) -> None`: register a noise channel for a single-qubit gate on the given qubit.
- `add_two_qubit_error(gate, q0, q1, noise) -> None`: register a noise channel for a two-qubit gate on the given qubit pair.

Raises:

- `ValueError`: a noise parameter is invalid, or the two qubits of a two-qubit gate are the same.

### Query methods

- `get_readout_error(qubit) -> ReadoutError | None`
- `get_single_qubit_errors(key) -> list[SingleQubitNoise] | None`
- `get_two_qubit_errors(key) -> list[TwoQubitNoise] | None`

Description:

- The same gate and qubit combination can have several registered channels, and a query returns a list; an unregistered combination returns `None`.

## Example

```python
import pytest
from cqlib.circuit import StandardGate
from cqlib.device import (
    NoiseModel,
    OperationKey,
    ReadoutError,
    SingleQubitNoise,
    TwoQubitNoise,
)
from cqlib.qis import Pauli

nm = NoiseModel()
nm.add_readout_error(0, ReadoutError(0.1, 0.2))
nm.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(0.01))
nm.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.phase_flip(0.02))
nm.add_two_qubit_error(StandardGate.CX, 0, 1, TwoQubitNoise.depolarizing(0.02))
nm.add_two_qubit_error(
    StandardGate.CX, 0, 1, TwoQubitNoise.correlated_pauli(Pauli.x(), Pauli.z(), 0.05)
)

skey = OperationKey.new_single(StandardGate.X, 0)
tkey = OperationKey.new_double(StandardGate.CX, 0, 1)

assert len(nm.get_single_qubit_errors(skey)) == 2
assert len(nm.get_two_qubit_errors(tkey)) == 2
assert nm.get_readout_error(0) == ReadoutError(0.1, 0.2)
assert nm.get_readout_error(9) is None

assert TwoQubitNoise.depolarizing(0.02).kind == "depolarizing"
assert TwoQubitNoise.correlated_pauli(Pauli.x(), Pauli.z(), 0.05).kind == "correlated_pauli"
assert SingleQubitNoise.depolarizing(0.1).is_valid() is True

with pytest.raises(ValueError):
    SingleQubitNoise.pauli(0.5, 0.5, 0.5)

with pytest.raises(ValueError):
    TwoQubitNoise.correlated_pauli(Pauli.x(), Pauli.z(), 1.5)

with pytest.raises(TypeError):
    TwoQubitNoise.correlated_pauli("X", Pauli.z(), 0.05)
```
