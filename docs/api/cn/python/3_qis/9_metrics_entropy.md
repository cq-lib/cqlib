# 度量与熵

`cqlib.qis.metrics`

`cqlib.qis.entropy`

`cqlib.qis.metrics` 提供量子态的度量：纯度、保真度、迹距离、冯·诺依曼熵，以及部分转置与对数负度。`cqlib.qis.entropy` 提供熵与纠缠度量：线性熵、Rényi 熵、纠缠熵、负度、concurrence 与纠缠生成熵。两个子模块都由函数组成，输入为 `Statevector` 或 `DensityMatrix`。

## 导入

```python
from cqlib.qis import entropy, metrics
```

---

## `cqlib.qis.metrics`

### `purity_pure(sv) -> float`

计算纯态的纯度。

参数：

- `sv` (`Statevector`)：态矢量。

返回：

- `float`：纯度。归一化的纯态为 `1.0`。

异常情况：

- 无。

示例：

```python
import math

from cqlib.qis import Statevector, metrics

sv = Statevector(2)
assert math.isclose(metrics.purity_pure(sv), 1.0, abs_tol=1e-10)
```

### `purity_mixed(dm) -> float`

计算混合态的纯度。

参数：

- `dm` (`DensityMatrix`)：密度矩阵。

返回：

- `float`：纯度 $\mathrm{Tr}(\rho^2)$，取值区间为 $[1/2^N, 1.0]$，最大混合态取到下界。

异常情况：

- 无。

示例：

```python
import math

from cqlib.qis import DensityMatrix, metrics

dm_pure = DensityMatrix(2)
assert math.isclose(metrics.purity_mixed(dm_pure), 1.0, abs_tol=1e-10)
```

### `state_fidelity_pure(sv1, sv2) -> float`

计算两个纯态之间的保真度。

参数：

- `sv1` (`Statevector`)：第一个态矢量。
- `sv2` (`Statevector`)：第二个态矢量。

返回：

- `float`：保真度 $F(\psi, \phi) = |\langle \psi | \phi \rangle|^2$，取值区间为 `[0.0, 1.0]`。

异常情况：

- `ValueError`：两个态矢量比特数不一致。

示例：

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

计算两个纯态之间的迹距离。

参数：

- `sv1` (`Statevector`)：第一个态矢量。
- `sv2` (`Statevector`)：第二个态矢量。

返回：

- `float`：迹距离 $D(\psi, \phi) = \sqrt{1 - |\langle \psi | \phi \rangle|^2}$，取值区间为 `[0.0, 1.0]`。

异常情况：

- `ValueError`：两个态矢量比特数不一致。

示例：

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

计算纯态与混合态之间的保真度。

参数：

- `sv` (`Statevector`)：纯态。
- `dm` (`DensityMatrix`)：混合态。

返回：

- `float`：保真度 $F(\psi, \rho) = \langle \psi | \rho | \psi \rangle$，取值区间为 `[0.0, 1.0]`。

异常情况：

- `ValueError`：态矢量与密度矩阵比特数不一致。

示例：

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

计算混合态的冯·诺依曼熵。

参数：

- `dm` (`DensityMatrix`)：密度矩阵。

返回：

- `float`：熵，以比特为单位（底为 2 的对数）。

异常情况：

- `ValueError`：特征分解失败。

示例：

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

计算两个混合态之间的迹距离。

参数：

- `dm1` (`DensityMatrix`)：第一个密度矩阵。
- `dm2` (`DensityMatrix`)：第二个密度矩阵。

返回：

- `float`：迹距离 $D(\rho, \sigma) = \frac{1}{2}\mathrm{Tr}|\rho - \sigma|$。

异常情况：

- `ValueError`：两个密度矩阵比特数不一致，或特征分解失败。

示例：

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

计算两个混合态之间的保真度。

参数：

- `dm1` (`DensityMatrix`)：第一个密度矩阵。
- `dm2` (`DensityMatrix`)：第二个密度矩阵。

返回：

- `float`：保真度 $F(\rho, \sigma) = \left( \mathrm{Tr}\sqrt{\sqrt{\rho}\,\sigma\sqrt{\rho}} \right)^2$。

异常情况：

- `ValueError`：两个密度矩阵比特数不一致，或特征分解失败。

示例：

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

对密度矩阵的指定子系统做部分转置。

参数：

- `dm` (`DensityMatrix`)：输入的密度矩阵。
- `target_qubits` (`list[int]`)：要转置的子系统比特下标。

返回：

- `DensityMatrix`：部分转置后的密度矩阵。转置保持迹不变，但可能不再半正定。

异常情况：

- `IndexError`：`target_qubits` 中有比特下标越界。

示例：

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

计算双分划量子态的对数负度。

参数：

- `dm` (`DensityMatrix`)：密度矩阵。
- `sys_a` (`list[int]`)：子系统 A 的比特下标。

返回：

- `float`：对数负度 $\log_2 \|\rho^{T_A}\|_1$，取值不小于 `0`。

异常情况：

- `IndexError`：`sys_a` 中有比特下标越界。
- `ValueError`：特征分解失败。

示例：

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

计算线性熵。线性熵是冯·诺依曼熵的计算上更省事的近似，用于刻画混合程度。

参数：

- `dm` (`DensityMatrix`)：密度矩阵。

返回：

- `float`：线性熵 $1 - \mathrm{Tr}(\rho^2)$，取值区间为 `[0, 1)`。

异常情况：

- 无。

示例：

```python
from cqlib.qis import DensityMatrix, entropy

dm_pure = DensityMatrix(2)
assert abs(entropy.linear_entropy(dm_pure) - 0.0) < 1e-10
```

### `renyi_entropy(dm, alpha) -> float`

计算阶数为 `alpha` 的 Rényi 熵。

参数：

- `dm` (`DensityMatrix`)：密度矩阵。
- `alpha` (`float`)：阶数，必须为正。取 `1.0` 时回退到冯·诺依曼熵。

返回：

- `float`：Rényi 熵，以比特为单位（底为 2 的对数）。

异常情况：

- `ValueError`：`alpha` 不大于零，或特征分解失败。

示例：

```python
from cqlib.qis import DensityMatrix, entropy

dm = DensityMatrix(2)  # 纯态
assert abs(entropy.renyi_entropy(dm, 2.0) - 0.0) < 1e-10
assert abs(entropy.renyi_entropy(dm, 1.0) - 0.0) < 1e-10  # 回退到冯·诺依曼熵
```

### `entanglement_entropy_pure(sv, subsys_a) -> float`

计算双分划纯态的纠缠熵，即子系统 A 约化密度矩阵的冯·诺依曼熵。

参数：

- `sv` (`Statevector`)：纯态。
- `subsys_a` (`list[int]`)：子系统 A 的比特下标。其余比特构成子系统 B。

返回：

- `float`：纠缠熵，以比特为单位。

异常情况：

- `ValueError`：`subsys_a` 为空、包含全部比特，或含重复下标。
- `IndexError`：`subsys_a` 中有比特下标越界。

示例：

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

计算负度，依据部分转置判据。

参数：

- `dm` (`DensityMatrix`)：双分划态的密度矩阵。
- `subsys_a` (`list[int]`)：子系统 A 的比特下标。

返回：

- `float`：负度，即部分转置后负特征值绝对值之和，取值不小于 `0`。

异常情况：

- `IndexError`：`subsys_a` 中有比特下标越界。
- `ValueError`：特征分解失败。

示例：

```python
from cqlib.qis import DensityMatrix, entropy

dm = DensityMatrix(2)
dm.apply_h(0)
dm.apply_cx(0, 1)

# Bell 态的负度为 0.5
assert abs(entropy.negativity(dm, [0]) - 0.5) < 1e-10
```

### `concurrence(dm) -> float`

计算 2 比特量子态的 concurrence。

参数：

- `dm` (`DensityMatrix`)：2 比特态的密度矩阵。

返回：

- `float`：concurrence，取值区间为 `[0, 1]`。可分态为 `0`，最大纠缠态为 `1`。

异常情况：

- `ValueError`：态的比特数不是 2，或计算失败。

示例：

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

计算 2 比特态的纠缠生成熵。

参数：

- `dm` (`DensityMatrix`)：2 比特态的密度矩阵。

返回：

- `float`：纠缠生成熵，以比特为单位。

异常情况：

- `ValueError`：态的比特数不是 2。

示例：

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

## 校验与错误处理

| 异常 | 触发场景 |
| --- | --- |
| `ValueError` | 参与比较的两个量子态比特数不一致；`renyi_entropy` 的 `alpha` 不大于零；`entanglement_entropy_pure` 的子系统为空、包含全部比特或含重复下标；`concurrence` 与 `entanglement_of_formation` 的态不是 2 比特；特征分解失败。 |
| `IndexError` | `partial_transpose`、`logarithmic_negativity`、`negativity`、`entanglement_entropy_pure` 的比特下标越界。 |

注意 `cqlib.qis.entropy` 是子模块名，`cqlib.qis.metrics` 中另有一个同名函数 `entropy`，二者互不相干。

本页各函数的输入类型见 [Statevector](1_statevector.md) 与 [DensityMatrix](2_density_matrix.md)。
