# Hamiltonian observable

`cqlib.qis`

`Hamiltonian` represents an operator as a linear combination of Pauli strings, $H = \sum_k c_k P_k$, where $c_k$ is a complex coefficient and $P_k$ is a multi-qubit Pauli string. It is a sparse representation of a $2^N \times 2^N$ matrix, used to describe system energy, to compute expectation values as an observable, and to generate time evolution circuits.

This page also covers the `Observable` protocol, which specifies the members an observable must provide.

## Import

```python
from cqlib.qis import Hamiltonian, Observable
```

---

## Observable

The observable protocol. It is a pure Python `typing.Protocol` defined by `cqlib.qis`, not a native class, and it provides no runtime checking.

Definition:

```python
from typing import Dict, List, Protocol, Tuple


class Observable(Protocol):
    def expectation_statevector(self, sv: Statevector) -> float: ...
    def expectation_density_matrix(self, dm: DensityMatrix) -> float: ...
    def expectation_probs(
        self, measurements: List[Tuple[PauliString, Dict[str, float]]]
    ) -> float: ...
    def variance_statevector(self, sv: Statevector) -> float: ...
    @property
    def num_qubits(self) -> int: ...
```

Implementing this protocol requires the following members:

- `expectation_statevector(sv) -> float`: compute the expectation value $\langle \psi | O | \psi \rangle$ from a statevector.
- `expectation_density_matrix(dm) -> float`: compute the expectation value $\mathrm{Tr}(\rho O)$ from a density matrix.
- `expectation_probs(measurements) -> float`: compute the expectation value from measurement probabilities, where `measurements` is several `(PauliString, dict)` tuples.
- `variance_statevector(sv) -> float`: compute the variance from a statevector.
- `num_qubits -> int`: the number of qubits the observable acts on, used for the dimension check before computation.

Both `Hamiltonian` and `PauliString` have all of the members above and can be passed directly as an observable to interfaces accepting `Observable`.

---

## Hamiltonian

### Hamiltonian(num_qubits)

Parameters:

- `num_qubits` (`int`): the number of qubits the operator acts on. The result is the zero operator on that number of qubits.

### Hamiltonian.from_pauli(pauli)

Construct from a single Pauli string, with a coefficient of `1.0`.

Parameters:

- `pauli` (`PauliString`): the Pauli string serving as the only term.

Returns:

- `Hamiltonian`: the operator representing $H = 1.0 \cdot P$.

### Hamiltonian.from_list(terms)

Construct from a list of `(PauliString, coefficient)` tuples.

Parameters:

- `terms` (`list[tuple[PauliString, float | int | complex | tuple[float, float]]]`): the list of terms. A coefficient can be a `float`, an `int`, a `complex`, or a `(real part, imaginary part)` pair.

Returns:

- `Hamiltonian`: the new operator.

Raises:

- `ValueError`: the Pauli strings of the terms have inconsistent qubit counts; or an element of the list is not a pair.

### Attributes

- `num_qubits -> int`: the number of qubits.
- `num_terms -> int`: the current number of terms, that is, the length of the `terms` list before `simplify()`.
- `terms -> list[tuple[PauliString, complex]]`: the list of terms, with coefficients given as Python complex numbers.

### Methods

- `add_term(op, coeff) -> None`: append a Pauli term and its coefficient. Coefficient forms are the same as in `from_list`.
- `simplify() -> None`: simplify. The internal phase of the Pauli strings is first absorbed into the complex coefficients, then terms with the same Pauli string are merged and terms with a coefficient close to zero are removed.
- `scale(factor) -> None`: scale all terms by a complex coefficient. Coefficient forms are the same as in `from_list`.
- `all_terms_commute() -> bool`: determine whether all Pauli terms commute pairwise.
- `to_matrix() -> numpy.ndarray`: return the dense matrix $H = \sum_k c_k P_k$, expanded in little-endian bit order and including the Pauli phases. The shape is $(2^N, 2^N)$ and the type is `complex128`; an empty operator gives a zero matrix. The space complexity is $O(4^N)$, which suits analysis and validation of small-scale systems.
- `to_trotter_circuit(time, steps, mode) -> Circuit`: generate the time evolution circuit given by the Trotter-Suzuki decomposition, approximating $U(t) = e^{-iHt}$ by a sequence of Pauli rotations.
- `to_evolution_circuit(time, steps, mode) -> Circuit`: generate the time evolution circuit. When all terms commute, an exact single-pass decomposition is used; when non-commuting terms exist, it falls back to the specified Trotter mode and number of steps.
- `copy() -> Hamiltonian`: return a copy.
- `expectation_statevector(sv) -> float`: compute the expectation value from a statevector.
- `expectation_density_matrix(dm) -> float`: compute the expectation value from a density matrix.
- `expectation_probs(measurements) -> float`: compute the expectation value from measurement probabilities.
- `variance_statevector(sv) -> float`: compute the variance from a statevector.

### Other behavior

- `+` and `+=` concatenate the term lists of two operators lazily and do not merge like terms automatically; call `simplify()` when merging is needed.
- Supports `==`.
- Does not define `hash` and is unhashable.
- `str` gives a readable form of the operator; `repr` has the form `Hamiltonian(num_qubits=2, num_terms=2)`.

### Raises

- `ValueError`: the two operators have inconsistent qubit counts (`+`, `+=`), the qubit count of a term is inconsistent with the operator (`from_list`, `add_term`), the coefficient type is not supported, the observable and the quantum state have inconsistent qubit counts (`expectation_*`, `variance_statevector`), or no usable measurement basis is found in the measurement probabilities (`expectation_probs`).
- `ValueError`: `to_trotter_circuit` and `to_evolution_circuit` raise when `steps` is `0`, the operator is empty, a coefficient is not Hermitian, or a Pauli string phase is not Hermitian.

### Example

```python
import math

from cqlib.qis import Hamiltonian, PauliString, Statevector

# 从空算符逐项构造
h = Hamiltonian(2)
h.add_term(PauliString.from_str("ZZ"), 0.5)
h.add_term(PauliString.from_str("XX"), 0.5)
h.simplify()

# 在 Bell 态上求期望值
sv = Statevector(2)
sv.apply_h(0)
sv.apply_cx(0, 1)
assert math.isclose(h.expectation_statevector(sv), 1.0)
```

```python
import math

from cqlib.qis import Hamiltonian, PauliString

# 由项列表构造，系数支持 float、complex 与 (实部, 虚部) 二元组
terms = [
    (PauliString.from_str("XX"), 0.5),
    (PauliString.from_str("YY"), -0.3 + 0.0j),
    (PauliString.from_str("ZZ"), (0.0, 0.2)),
]
h = Hamiltonian.from_list(terms)
assert h.num_qubits == 2
assert h.num_terms == 3
```

```python
import math

from cqlib.qis import Hamiltonian, PauliString

# 化简：合并同类项、删除近零项、把内部相位并入系数
h = Hamiltonian(2)
h.add_term(PauliString.from_str("XX"), 0.5)
h.add_term(PauliString.from_str("XX"), 0.3)
h.add_term(PauliString.from_str("ZZ"), 1.0)
h.add_term(PauliString.from_str("ZZ"), -1.0)
h.add_term(PauliString.from_str("+iYY"), 0.5)

assert h.num_terms == 5
h.simplify()
assert h.num_terms == 2

xx_term = next(t for t in h.terms if str(t[0]) == "+XX")
assert math.isclose(xx_term[1].real, 0.8)
```

```python
from cqlib.qis import Hamiltonian, PauliString, TrotterMode

# 时间演化线路
h = Hamiltonian(2)
h.add_term(PauliString.from_str("ZZ"), 0.5)
h.add_term(PauliString.from_str("XX"), 0.3)

circuit = h.to_trotter_circuit(1.0, 10, TrotterMode.first_order())
assert circuit.num_qubits == 2
assert len(circuit) > 0
```

```python
from cqlib.qis import Hamiltonian, PauliString, TrotterMode

# 所有项对易时 to_evolution_circuit 走精确分解
h_commuting = Hamiltonian(2)
h_commuting.add_term(PauliString.from_str("ZZ"), 0.5)
h_commuting.add_term(PauliString.from_str("IZ"), 0.3)
assert h_commuting.all_terms_commute() is True

exact = h_commuting.to_evolution_circuit(1.0, 1, TrotterMode.first_order())
assert exact.num_qubits == 2

# 存在非对易项时回退到 Trotter
h_non_commuting = Hamiltonian(1)
h_non_commuting.add_term(PauliString.from_str("X"), 1.0)
h_non_commuting.add_term(PauliString.from_str("Z"), 1.0)
assert h_non_commuting.all_terms_commute() is False

trotterized = h_non_commuting.to_evolution_circuit(0.5, 4, TrotterMode.second_order())
assert trotterized.num_qubits == 1
```

---

## Validation and error handling

| Exception | When it occurs |
| --- | --- |
| `ValueError` | Inconsistent qubit counts: between the terms of `from_list`, between a term and the operator in `add_term`, between the two operators of `+` and `+=`, or between the observable and the quantum state. |
| `ValueError` | The coefficient is not a `float`, an `int`, a `complex`, or a `(real part, imaginary part)` pair. |
| `ValueError` | `to_trotter_circuit` and `to_evolution_circuit`: `steps` is `0`, the operator is empty, or a coefficient or a Pauli string phase is not Hermitian. |
| `ValueError` | `expectation_probs` cannot derive the target operator from the given measurement bases. |

The input types of observable computation are given in [Statevector](1_statevector.md) and [DensityMatrix](2_density_matrix.md); the definition of Trotter modes is given in [Trotter evolution](8_evolution.md).
