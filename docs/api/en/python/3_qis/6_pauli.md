# Pauli operators

`cqlib.qis`

`Phase`, `Pauli` and `PauliString` are the three levels of the Pauli group: `Phase` represents the phase factor produced when group elements are multiplied, `Pauli` is a single-qubit Pauli operator, and `PauliString` is a tensor product of multi-qubit Pauli operators. `PauliString` is stored in the symplectic representation, and provides conversions between the character form, the bit masks and the matrix form.

## Import

```python
from cqlib.qis import Phase, Pauli, PauliString
```

---

## Phase

A phase factor in the Pauli group, isomorphic to the cyclic group of order 4, representing $i^n$ with $n \in \{0, 1, 2, 3\}$.

### Phase(val)

Parameters:

- `val` (`int`): the phase exponent, taken modulo 4, with an acceptable range of `0`–`255`.

### Static methods

- `Phase.plus()`: return $+1$, that is, $i^0$.
- `Phase.i()`: return $+i$, that is, $i^1$.
- `Phase.minus()`: return $-1$, that is, $i^2$.
- `Phase.minus_i()`: return $-i$, that is, $i^3$.

### Attributes

- `exponent -> int`: the phase exponent, with values `0`–`3`.

### Methods

- `to_complex() -> complex`: convert to a Python complex number.

### Other behavior

- Both `+` and `*` denote group multiplication, and the result is taken modulo 4.
- Supports `==` and `hash`.
- `str` is `1`, `i`, `-1` or `-i`; `repr` has the form `Phase(1)`.

### Raises

- `OverflowError`: raised when `val` is outside `0`–`255`.

---

## Pauli

A single-qubit Pauli operator.

### Static methods

- `Pauli.x()`, `Pauli.y()`, `Pauli.z()`, `Pauli.i()`: the only construction entry points of the four Pauli operators.

### Methods

- `to_symplectic() -> tuple[int, int]`: return the symplectic representation $(x, z)$. `I` is `(0, 0)`, `X` is `(1, 0)`, `Y` is `(1, 1)` and `Z` is `(0, 1)`.
- `to_matrix() -> numpy.ndarray`: return the $2 \times 2$ complex matrix.
- `mul_with_phase(other) -> tuple[Pauli, Phase]`: multiply with another single-qubit Pauli operator and return the resulting operator together with the phase factor.

### Other behavior

- No public constructor.
- `*` denotes multiplication but does not return the phase factor; use `mul_with_phase` when the phase is needed.
- Supports `==` and `hash`.
- `str` is the operator name; `repr` has the form `Pauli.X`.

---

## PauliString

A multi-qubit Pauli operator, that is, $P = \bigotimes_{i=0}^{N-1} P_i$, where $P_i \in \{I, X, Y, Z\}$.

### PauliString(num_qubits)

Parameters:

- `num_qubits` (`int`): the number of qubits. The result is an all-`I` string with phase $+1$.

### PauliString.from_str(s)

Construct from the character form. The format is `[+|-][i|j]<operator sequence>`, where the operator sequence consists of `I`, `X`, `Y` and `Z`; qubits are ordered by decreasing index, so the first character corresponds to the highest qubit index. `j` is equivalent to `i`, and is normalized to `i` internally.

Parameters:

- `s` (`str`): the character form, for example `"XZI"`, `"-iZII"`, `"+XYZ"`.

Returns:

- `PauliString`: the new Pauli string.

Raises:

- `ValueError`: raised when the string is empty, contains an illegal character, or gives only a phase without operators.

### Attributes

- `num_qubits -> int`: the number of qubits.
- `phase -> Phase`: the global phase, readable and writable.
- `x_bits -> list[bool]`: the bit vector of the $X$ component, indexed by qubit index.
- `z_bits -> list[bool]`: the bit vector of the $Z$ component.
- `x_mask -> int`: the integer mask of the $X$ component.
- `z_mask -> int`: the integer mask of the $Z$ component.

### Methods

- `get_pauli(idx) -> Pauli`: read the operator on the given qubit.
- `set_pauli(idx, pauli) -> None`: write the operator on the given qubit.
- `y_phase() -> complex`: return the phase factor contributed by $Y$ operators. Since $Y = iXZ$, $n$ occurrences of $Y$ contribute $i^n$.
- `commutes_with(other) -> bool`: determine whether two Pauli strings commute, based on whether the symplectic inner product modulo 2 is zero.
- `support() -> list[int]`: return the qubit indices carrying an operator other than `I`, in ascending order.
- `to_matrix() -> numpy.ndarray`: return the dense matrix, expanded in little-endian tensor order (qubit `0` is the least significant tensor factor, that is, $P_{N-1} \otimes \cdots \otimes P_0$), including the global phase. The shape is $(2^N, 2^N)$ and the type is `complex128`. The space complexity is $O(4^N)$, which suits analysis and validation of small-scale systems.
- `copy() -> PauliString`: return a copy.
- `expectation(probs) -> float`: compute the expectation value $\langle P \rangle = \sum_s p(s)\langle s | P | s \rangle$ from a probability distribution. If the string contains non-diagonal operators such as `X` or `Y`, the expectation value is `0` for any computational basis probability distribution.
- `expectation_statevector(sv) -> float`: compute the expectation value from a statevector.
- `expectation_density_matrix(dm) -> float`: compute the expectation value from a density matrix.
- `expectation_probs(measurements) -> float`: compute the expectation value from measurement probabilities.
- `variance_statevector(sv) -> float`: compute the variance from a statevector.

### Other behavior

- `len(ps)` equals `num_qubits`; a zero-qubit string is falsy.
- A string can be iterated directly, yielding `Pauli` one by one in ascending qubit index, including `I`. The iterator has snapshot semantics: modifications to the original string after the iterator is created do not affect this iteration. The iterator provides `__length_hint__`, giving the number of remaining elements.
- `ps[i]` reads the operator on the given qubit and supports negative indices, where `-1` means the highest qubit index.
- `*` and `*=` denote multiplication, and the phase of the result accumulates by group multiplication.
- Supports `==`; as a mutable type it does not define `hash`, so it is unhashable and cannot be used as a dictionary key.
- `str` has the form `+XYZ` or `-iZII`; `repr` has the form `PauliString(num_qubits=3, phase=1, x_bits=[...], z_bits=[...])`.

### Raises

- `ValueError`: `from_str` parsing failure; the two operands of `commutes_with` or `*` have inconsistent qubit counts.
- `IndexError`: index out of range in `get_pauli` or `set_pauli`; index out of range in `ps[i]`.

### Expectation value parameters

The `probs` of `expectation(probs)` is a dictionary from state strings to probabilities. State strings are written in the little-endian convention: the rightmost character corresponds to qubit `0`, so for example `"01"` means qubit `0` is `1` and qubit `1` is `0`. The length of a state string must equal `num_qubits`.

The `measurements` of `expectation_probs(measurements)` is a list of `(PauliString, dict)` tuples, each giving a measurement basis together with the corresponding probability distribution. If the target operator cannot be derived from the given measurement bases, an error is raised.

### Example

```python
from cqlib.qis import Pauli, PauliString, Phase

# 乘法与相位
x, y, z, i = Pauli.x(), Pauli.y(), Pauli.z(), Pauli.i()
res, phase = x.mul_with_phase(y)  # XY = iZ
assert res == z
assert phase == Phase.i()
assert x * y == z
assert x.to_symplectic() == (1, 0)

# 字符形式与属性
ps = PauliString.from_str("-iZII")
assert ps.num_qubits == 3
assert str(ps) == "-iZII"
assert ps.phase == Phase.minus_i()
assert PauliString.from_str("+jZZ").phase == Phase.i()

# 掩码、支撑集与迭代
ps = PauliString.from_str("XIZ")
assert ps.x_mask == 0b100
assert ps.z_mask == 0b001
assert ps.support() == [0, 2]
assert len(ps) == 3
assert list(ps) == [Pauli.z(), Pauli.i(), Pauli.x()]  # 按比特下标升序，含 I
```

```python
from cqlib.qis import Pauli, PauliString

# 构造与逐比特读写
ps = PauliString(3)
assert str(ps) == "+III"
ps.set_pauli(0, Pauli.x())
ps.set_pauli(1, Pauli.z())
ps.set_pauli(2, Pauli.y())
assert str(ps) == "+YZX"
assert ps.get_pauli(0) == Pauli.x()
assert ps[0] == Pauli.x()
assert ps[-1] == Pauli.y()
```

```python
from cqlib.qis import PauliString

# 乘法与对易性
ps1 = PauliString.from_str("X")
ps2 = PauliString.from_str("Z")
assert str(ps1 * ps2) == "-iY"

ps3 = PauliString.from_str("XZ")
ps4 = PauliString.from_str("YX")
assert str(ps3 * ps4) == "-ZY"

ps5 = PauliString.from_str("XZI")
assert ps5.commutes_with(PauliString.from_str("ZXI")) is True
assert ps5.commutes_with(PauliString.from_str("YII")) is False
```

```python
import math

from cqlib.qis import PauliString, Statevector

# 期望值与方差
sv = Statevector(1)
ps_z = PauliString.from_str("Z")
assert math.isclose(ps_z.expectation_statevector(sv), 1.0)

sv.apply_h(0)
ps_x = PauliString.from_str("X")
assert math.isclose(ps_x.expectation_statevector(sv), 1.0)
assert math.isclose(ps_z.variance_statevector(sv), 1.0, abs_tol=1e-10)

# 由概率分布求期望值
assert math.isclose(ps_z.expectation({"0": 0.5, "1": 0.5}), 0.0)
```

```python
import numpy as np

from cqlib.qis import PauliString

# 矩阵形式，小端张量序：比特 0 是最低有效张量因子
ps = PauliString.from_str("ZX")
matrix = ps.to_matrix()
assert matrix.shape == (4, 4)
assert matrix.dtype == np.complex128
```

---

## Validation and error handling

| Exception | When it occurs |
| --- | --- |
| `ValueError` | The string of `PauliString.from_str` is empty, contains an illegal character or lacks operators; the two operands of `commutes_with`, `*` or `*=` have inconsistent qubit counts. |
| `IndexError` | Qubit index out of range in `get_pauli`, `set_pauli` or `ps[i]`. |
| `OverflowError` | The exponent of `Phase(val)` is outside the acceptable range. |

The input types of `expectation_statevector`, `expectation_density_matrix` and `variance_statevector` are given in [Statevector](1_statevector.md) and [DensityMatrix](2_density_matrix.md).
