# Trotter evolution

`cqlib.qis`

`TrotterMode` describes how the time evolution operator $U(t) = e^{-iHt}$ is approximated by a sequence of Pauli rotations, and is the decomposition mode that must be specified when converting a `Hamiltonian` into an evolution circuit. This page covers `TrotterMode` itself, together with the two evolution entry points that consume it.

## Import

```python
from cqlib.qis import Hamiltonian, PauliString, TrotterMode
```

---

## TrotterMode

A Trotter-Suzuki decomposition mode. There is no public constructor; the three modes are created by static methods. Equality is determined by mode and random seed.

### Static methods

- `TrotterMode.first_order()`: first-order Lie-Trotter decomposition. For $H = \sum_k c_k P_k$, a single step applies the terms one by one in order
  $U(t) \approx \left[ \prod_k e^{-i c_k (t/n) P_k} \right]^n$,
  with an error of order $O(t^2/n)$.
- `TrotterMode.second_order()`: second-order Strang symmetric decomposition. A single step applies the terms once in order and once in reverse order
  $U(t) \approx \left[ \prod_k e^{-i c_k (t/2n) P_k} \prod_k e^{-i c_k (t/2n) P_k} \right]^n$,
  with an error of order $O(t^3/n^2)$.
- `TrotterMode.randomized(seed)`: randomized first-order decomposition. The order of the Pauli terms is shuffled randomly within each Trotter step, which weakens systematic errors.

Parameters:

- `seed` (`int`): the random seed, an unsigned integer; the same seed gives the same result.

Returns:

- `TrotterMode`: the corresponding decomposition mode object.

### Other behavior

- Supports `==` and `hash`. Two `TrotterMode.randomized(s)` are equal only when the seeds are the same; `first_order()` and `second_order()` are each equal to their own new instances.
- `str` is `first-order`, `second-order` or `randomized (seed=N)`.
- `repr` is `TrotterMode.FirstOrder`, `TrotterMode.SecondOrder` or `TrotterMode.Randomized(seed=N)`.

---

## Evolution entry points

`TrotterMode` is consumed by the two evolution methods of `Hamiltonian`. Both return an abstract circuit and their parameters have the same meaning.

### Hamiltonian.to_trotter_circuit(time, steps, mode) -> Circuit

Generate a Trotterized time evolution circuit with the given mode and number of steps.

Parameters:

- `time` (`float`): the total evolution time $t$.
- `steps` (`int`): the number of Trotter steps $n$, which must be greater than `0`.
- `mode` (`TrotterMode`): the decomposition mode.

Returns:

- `Circuit`: a circuit approximating $U(t) = e^{-iHt}$.

### Hamiltonian.to_evolution_circuit(time, steps, mode) -> Circuit

Generate a time evolution circuit. When all terms commute, an exact single-pass decomposition $\prod_k e^{-i c_k t P_k}$ is used; when non-commuting terms exist, a Trotter approximation is made with the specified mode and number of steps. `steps` and `mode` take effect only in the non-commuting case, but `steps` must still be greater than `0`.

Parameters:

- `time` (`float`): the total evolution time $t$.
- `steps` (`int`): the number of Trotter steps, which must be greater than `0`. It takes part in the decomposition only when commutation does not hold.
- `mode` (`TrotterMode`): the decomposition mode. It takes part in the decomposition only when commutation does not hold.

Returns:

- `Circuit`: the exact or approximate evolution circuit.

### Raises

Both entry points raise `ValueError` in the following cases:

- `steps` is `0`.
- The `Hamiltonian` is empty.
- A coefficient is not Hermitian (the imaginary part is outside the tolerance).
- A Pauli string phase is not Hermitian, that is, the phase is $\pm i$ rather than $\pm 1$.

### Example

```python
from cqlib.qis import TrotterMode

# 三种模式
mode = TrotterMode.first_order()
assert str(mode) == "first-order"
assert "FirstOrder" in repr(mode)

assert str(TrotterMode.second_order()) == "second-order"

mode_random = TrotterMode.randomized(42)
assert "seed=42" in repr(mode_random)
assert mode_random != TrotterMode.randomized(123)
assert mode_random == TrotterMode.randomized(42)
```

```python
from cqlib.qis import Hamiltonian, PauliString, TrotterMode

h = Hamiltonian(2)
h.add_term(PauliString.from_str("ZZ"), 0.5)
h.add_term(PauliString.from_str("XX"), 0.3)

first = h.to_trotter_circuit(1.0, 10, TrotterMode.first_order())
second = h.to_trotter_circuit(1.0, 10, TrotterMode.second_order())
randomized = h.to_trotter_circuit(1.0, 10, TrotterMode.randomized(42))

assert first.num_qubits == 2
assert second.num_qubits == 2
assert randomized.num_qubits == 2
```

```python
from cqlib.qis import Hamiltonian, PauliString, TrotterMode

# 对易项走精确分解，步数与模式不参与分解
h_commuting = Hamiltonian(2)
h_commuting.add_term(PauliString.from_str("ZZ"), 0.5)
h_commuting.add_term(PauliString.from_str("IZ"), 0.3)

exact = h_commuting.to_evolution_circuit(1.0, 1, TrotterMode.first_order())
assert exact.num_qubits == 2

# 非对易项回退到 Trotter
h_non_commuting = Hamiltonian(1)
h_non_commuting.add_term(PauliString.from_str("X"), 1.0)
h_non_commuting.add_term(PauliString.from_str("Z"), 1.0)

trotterized = h_non_commuting.to_evolution_circuit(0.5, 4, TrotterMode.second_order())
assert trotterized.num_qubits == 1
```

---

## Validation and error handling

| Exception | When it occurs |
| --- | --- |
| `ValueError` | `steps` of an evolution entry point is `0`, the `Hamiltonian` is empty, or a coefficient or Pauli string phase is not Hermitian. |

When a decomposition mode must be specified in the construction of a parameterized evolution circuit, use the circuit-layer evolution strategy interface, see [Ansatz](../0_circuit/11_ansatz.md). The complete interface of `Hamiltonian` is given in [Hamiltonian observable](7_hamiltonian.md).
