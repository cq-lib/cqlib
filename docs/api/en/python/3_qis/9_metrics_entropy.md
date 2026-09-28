# Metrics and Entropy

`cqlib.qis.metrics`

`cqlib.qis.entropy`

`cqlib.qis.metrics` provides measures of quantum states: purity, fidelity, trace distance, von Neumann entropy, and partial transpose and logarithmic negativity. `cqlib.qis.entropy` provides entropy and entanglement measures: linear entropy, Rényi entropy, entanglement entropy, negativity, concurrence and entanglement of formation. Both submodules consist of functions, whose inputs are a `Statevector` or a `DensityMatrix`.

## Import

```python
from cqlib.qis import entropy, metrics
```

---

## `cqlib.qis.metrics`

### `purity_pure(sv) -> float`

Compute the purity of a pure state.

Parameters:

- `sv` (`Statevector`): the statevector.

Returns:

- `float`: the purity. A normalized pure state gives `1.0`.

Raises:

- None.

Example:

```python
import math

from cqlib.qis import Statevector, metrics

sv = Statevector(2)
assert math.isclose(metrics.purity_pure(sv), 1.0, abs_tol=1e-10)
```

### `purity_mixed(dm) -> float`

Compute the purity of a mixed state.

Parameters:

- `dm` (`DensityMatrix`): the density matrix.

Returns:

- `float`: the purity $\mathrm{Tr}(\rho^2)$, with a range of $[1/2^N, 1.0]$; the maximally mixed state reaches the lower bound.

Raises:

- None.

Example:

```python
import math

from cqlib.qis import DensityMatrix, metrics

dm_pure = DensityMatrix(2)
assert math.isclose(metrics.purity_mixed(dm_pure), 1.0, abs_tol=1e-10)
```

### `state_fidelity_pure(sv1, sv2) -> float`

Compute the fidelity between two pure states.

Parameters:

- `sv1` (`Statevector`): the first statevector.
- `sv2` (`Statevector`): the second statevector.

Returns:

- `float`: the fidelity $F(\psi, \phi) = |\langle \psi | \phi \rangle|^2$, with a range of `[0.0, 1.0]`.

Raises:

- `ValueError`: the two statevectors have inconsistent qubit counts.

Example:

```python
import math

from cqlib.qis import Statevector, metrics

sv1 = Statevector(1)  # |0>
sv2 = Statevector(1)
sv2.apply_x(0)  # |1>

assert math.isclose(metrics.state_fidelity_pure(sv1, sv1), 1.0, abs_tol=1e-10)
assert math.isclose(metrics.state_fidelity_pure(sv1, sv2), 0.0, abs_tol=1e-10)
```

### `trace_distance_pure(sv1, sv2) -> float`

Compute the trace distance between two pure states.

Parameters:

- `sv1` (`Statevector`): the first statevector.
- `sv2` (`Statevector`): the second statevector.

Returns:

- `float`: the trace distance $D(\psi, \phi) = \sqrt{1 - |\langle \psi | \phi \rangle|^2}$, with a range of `[0.0, 1.0]`.

Raises:

- `ValueError`: the two statevectors have inconsistent qubit counts.

Example:

```python
import math

from cqlib.qis import Statevector, metrics

sv1 = Statevector(1)  # |0>
sv2 = Statevector(1)
sv2.apply_x(0)  # |1>

assert math.isclose(metrics.trace_distance_pure(sv1, sv1), 0.0, abs_tol=1e-10)
assert math.isclose(metrics.trace_distance_pure(sv1, sv2), 1.0, abs_tol=1e-10)
```

### `state_fidelity_pure_mixed(sv, dm) -> float`

Compute the fidelity between a pure state and a mixed state.

Parameters:

- `sv` (`Statevector`): the pure state.
- `dm` (`DensityMatrix`): the mixed state.

Returns:

- `float`: the fidelity $F(\psi, \rho) = \langle \psi | \rho | \psi \rangle$, with a range of `[0.0, 1.0]`.

Raises:

- `ValueError`: the statevector and the density matrix have inconsistent qubit counts.

Example:

```python
import math

import numpy as np

from cqlib.qis import DensityMatrix, Statevector, metrics

sv = Statevector(1)  # |0>
dm = DensityMatrix(1)  # |0><0|
assert math.isclose(metrics.state_fidelity_pure_mixed(sv, dm), 1.0, abs_tol=1e-10)

# |0><0| 与 I/2 的保真度为 0.5
mixed_data = np.array([0.5, 0, 0, 0.5], dtype=complex)
dm_mixed = DensityMatrix.from_density_matrix(1, mixed_data)
assert math.isclose(metrics.state_fidelity_pure_mixed(sv, dm_mixed), 0.5, abs_tol=1e-10)
```

### `entropy(dm) -> float`

Compute the von Neumann entropy of a mixed state.

Parameters:

- `dm` (`DensityMatrix`): the density matrix.

Returns:

- `float`: the entropy, in bits (logarithm base 2).

Raises:

- `ValueError`: eigendecomposition failure.

Example:

```python
import math

import numpy as np

from cqlib.qis import DensityMatrix, metrics

dm = DensityMatrix(2)
assert math.isclose(metrics.entropy(dm), 0.0, abs_tol=1e-10)

# 最大混合单比特态 I/2 的熵为 1 比特
mixed_data = np.array([0.5, 0, 0, 0.5], dtype=complex)
dm_mixed = DensityMatrix.from_density_matrix(1, mixed_data)
assert math.isclose(metrics.entropy(dm_mixed), 1.0, abs_tol=1e-10)
```

### `trace_distance_mixed(dm1, dm2) -> float`

Compute the trace distance between two mixed states.

Parameters:

- `dm1` (`DensityMatrix`): the first density matrix.
- `dm2` (`DensityMatrix`): the second density matrix.

Returns:

- `float`: the trace distance $D(\rho, \sigma) = \frac{1}{2}\mathrm{Tr}|\rho - \sigma|$.

Raises:

- `ValueError`: the two density matrices have inconsistent qubit counts, or eigendecomposition failure.

Example:

```python
import math

from cqlib.qis import DensityMatrix, metrics

dm1 = DensityMatrix(1)
dm2 = DensityMatrix(1)
dm2.apply_x(0)

assert math.isclose(metrics.trace_distance_mixed(dm1, dm1), 0.0, abs_tol=1e-10)
assert math.isclose(metrics.trace_distance_mixed(dm1, dm2), 1.0, abs_tol=1e-10)
```

### `state_fidelity_mixed(dm1, dm2) -> float`

Compute the fidelity between two mixed states.

Parameters:

- `dm1` (`DensityMatrix`): the first density matrix.
- `dm2` (`DensityMatrix`): the second density matrix.

Returns:

- `float`: the fidelity $F(\rho, \sigma) = \left( \mathrm{Tr}\sqrt{\sqrt{\rho}\,\sigma\sqrt{\rho}} \right)^2$.

Raises:

- `ValueError`: the two density matrices have inconsistent qubit counts, or eigendecomposition failure.

Example:

```python
import math

from cqlib.qis import DensityMatrix, metrics

dm1 = DensityMatrix(1)
dm2 = DensityMatrix(1)
dm2.apply_x(0)

assert math.isclose(metrics.state_fidelity_mixed(dm1, dm1), 1.0, abs_tol=1e-10)
assert math.isclose(metrics.state_fidelity_mixed(dm1, dm2), 0.0, abs_tol=1e-10)
```

### `partial_transpose(dm, target_qubits) -> DensityMatrix`

Take the partial transpose of the given subsystem of a density matrix.

Parameters:

- `dm` (`DensityMatrix`): the input density matrix.
- `target_qubits` (`list[int]`): the qubit indices of the subsystem to transpose.

Returns:

- `DensityMatrix`: the density matrix after the partial transpose. The transpose preserves the trace but may no longer be positive semidefinite.

Raises:

- `IndexError`: a qubit index in `target_qubits` is out of range.

Example:

```python
import math

from cqlib.qis import DensityMatrix, metrics

# Bell 态 |Φ+> = (|00> + |11>) / √2
dm = DensityMatrix(2)
dm.apply_h(0)
dm.apply_cx(0, 1)

pt_dm = metrics.partial_transpose(dm, [0])
data = pt_dm.data
assert math.isclose(data[0, 0].real, 0.5, abs_tol=1e-10)
assert math.isclose(data[3, 3].real, 0.5, abs_tol=1e-10)
```

### `logarithmic_negativity(dm, sys_a) -> float`

Compute the logarithmic negativity of a bipartite quantum state.

Parameters:

- `dm` (`DensityMatrix`): the density matrix.
- `sys_a` (`list[int]`): the qubit indices of subsystem A.

Returns:

- `float`: the logarithmic negativity $\log_2 \|\rho^{T_A}\|_1$, which is not less than `0`.

Raises:

- `IndexError`: a qubit index in `sys_a` is out of range.
- `ValueError`: eigendecomposition failure.

Example:

```python
import math

from cqlib.qis import DensityMatrix, metrics

# Bell 态的对数负度为 1.0
dm = DensityMatrix(2)
dm.apply_h(0)
dm.apply_cx(0, 1)
assert math.isclose(metrics.logarithmic_negativity(dm, [0]), 1.0, abs_tol=1e-10)

# 可分态 |00> 的对数负度为 0.0
dm_sep = DensityMatrix(2)
assert math.isclose(metrics.logarithmic_negativity(dm_sep, [0]), 0.0, abs_tol=1e-10)
```

---

## `cqlib.qis.entropy`

### `linear_entropy(dm) -> float`

Compute the linear entropy. The linear entropy is a computationally cheaper approximation of the von Neumann entropy, used to characterize the degree of mixing.

Parameters:

- `dm` (`DensityMatrix`): the density matrix.

Returns:

- `float`: the linear entropy $1 - \mathrm{Tr}(\rho^2)$, with a range of `[0, 1)`.

Raises:

- None.

Example:

```python
from cqlib.qis import DensityMatrix, entropy

dm_pure = DensityMatrix(2)
assert abs(entropy.linear_entropy(dm_pure) - 0.0) < 1e-10
```

### `renyi_entropy(dm, alpha) -> float`

Compute the Rényi entropy of order `alpha`.

Parameters:

- `dm` (`DensityMatrix`): the density matrix.
- `alpha` (`float`): the order, which must be positive. A value of `1.0` falls back to the von Neumann entropy.

Returns:

- `float`: the Rényi entropy, in bits (logarithm base 2).

Raises:

- `ValueError`: `alpha` is not greater than zero, or eigendecomposition failure.

Example:

```python
from cqlib.qis import DensityMatrix, entropy

dm = DensityMatrix(2)  # 纯态
assert abs(entropy.renyi_entropy(dm, 2.0) - 0.0) < 1e-10
assert abs(entropy.renyi_entropy(dm, 1.0) - 0.0) < 1e-10  # 回退到冯·诺依曼熵
```

### `entanglement_entropy_pure(sv, subsys_a) -> float`

Compute the entanglement entropy of a bipartite pure state, that is, the von Neumann entropy of the reduced density matrix of subsystem A.

Parameters:

- `sv` (`Statevector`): the pure state.
- `subsys_a` (`list[int]`): the qubit indices of subsystem A. The remaining qubits form subsystem B.

Returns:

- `float`: the entanglement entropy, in bits.

Raises:

- `ValueError`: `subsys_a` is empty, contains all qubits, or contains duplicate indices.
- `IndexError`: a qubit index in `subsys_a` is out of range.

Example:

```python
from cqlib.qis import Statevector, entropy

# Bell 态 |Φ+> = (|00> + |11>) / √2
sv = Statevector(2)
sv.apply_h(0)
sv.apply_cx(0, 1)
assert abs(entropy.entanglement_entropy_pure(sv, [0]) - 1.0) < 1e-10

# 直积态 |00> 的纠缠熵为 0
sv_prod = Statevector(2)
assert abs(entropy.entanglement_entropy_pure(sv_prod, [0]) - 0.0) < 1e-10
```

### `negativity(dm, subsys_a) -> float`

Compute the negativity, based on the positive partial transpose criterion.

Parameters:

- `dm` (`DensityMatrix`): the density matrix of a bipartite state.
- `subsys_a` (`list[int]`): the qubit indices of subsystem A.

Returns:

- `float`: the negativity, that is, the sum of the absolute values of the negative eigenvalues after the partial transpose, which is not less than `0`.

Raises:

- `IndexError`: a qubit index in `subsys_a` is out of range.
- `ValueError`: eigendecomposition failure.

Example:

```python
from cqlib.qis import DensityMatrix, entropy

dm = DensityMatrix(2)
dm.apply_h(0)
dm.apply_cx(0, 1)

# Bell 态的负度为 0.5
assert abs(entropy.negativity(dm, [0]) - 0.5) < 1e-10
```

### `concurrence(dm) -> float`

Compute the concurrence of a two-qubit quantum state.

Parameters:

- `dm` (`DensityMatrix`): the density matrix of a two-qubit state.

Returns:

- `float`: the concurrence, with a range of `[0, 1]`. A separable state gives `0` and a maximally entangled state gives `1`.

Raises:

- `ValueError`: the state does not have 2 qubits, or the computation fails.

Example:

```python
from cqlib.qis import DensityMatrix, entropy

dm = DensityMatrix(2)
dm.apply_h(0)
dm.apply_cx(0, 1)

assert abs(entropy.concurrence(dm) - 1.0) < 1e-10

dm_sep = DensityMatrix(2)
assert abs(entropy.concurrence(dm_sep) - 0.0) < 1e-10
```

### `entanglement_of_formation(dm) -> float`

Compute the entanglement of formation of a two-qubit state.

Parameters:

- `dm` (`DensityMatrix`): the density matrix of a two-qubit state.

Returns:

- `float`: the entanglement of formation, in bits.

Raises:

- `ValueError`: the state does not have 2 qubits.

Example:

```python
from cqlib.qis import DensityMatrix, entropy

dm = DensityMatrix(2)
dm.apply_h(0)
dm.apply_cx(0, 1)

assert abs(entropy.entanglement_of_formation(dm) - 1.0) < 1e-10

dm_sep = DensityMatrix(2)
assert abs(entropy.entanglement_of_formation(dm_sep) - 0.0) < 1e-10
```

---

## Validation and error handling

| Exception | When it occurs |
| --- | --- |
| `ValueError` | The two quantum states being compared have inconsistent qubit counts; `alpha` of `renyi_entropy` is not greater than zero; the subsystem of `entanglement_entropy_pure` is empty, contains all qubits or contains duplicate indices; the state of `concurrence` or `entanglement_of_formation` is not a two-qubit state; eigendecomposition failure. |
| `IndexError` | Qubit index out of range in `partial_transpose`, `logarithmic_negativity`, `negativity` or `entanglement_entropy_pure`. |

Note that `cqlib.qis.entropy` is a submodule name, while `cqlib.qis.metrics` contains a function of the same name `entropy`; the two are unrelated.

The input types of the functions on this page are given in [Statevector](1_statevector.md) and [DensityMatrix](2_density_matrix.md).
