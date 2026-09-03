# MCGate

`cqlib.circuit.gates.MCGate`  

```python
from cqlib.circuit.gates import MCGate
```

`MCGate` 用于表示多控制量子门，即在一个已有 `StandardGate` 的基础上增加一个或多个控制量子比特。只有当所有控制比特满足控制条件时，基础门才会作用到对应目标比特上。多控制门常用于构造 Toffoli 门、多控制相位门、oracle、条件翻转操作以及量子算法中的受控子程序。

---

## 构造函数

```python
MCGate(num_controls: int, gate: StandardGate)
```

| 参数 | 说明 |
| --- | --- |
| `num_controls` | 需要额外增加的控制位数量。 |
| `gate` | 被控制的基础标准门，即控制条件满足时实际执行的门。 |

下面的示例分别构造了一个两控制 `X` 门和一个三控制 `H` 门：

```python
from cqlib.circuit.gates import MCGate, StandardGate

ccx = MCGate(2, StandardGate.X)
mch = MCGate(3, StandardGate.H)
```

此外，也可以直接从标准门对象调用 `control()` 方法生成多控制门。

```python
from cqlib.circuit.gates import StandardGate

mcx = StandardGate.X.control(4)
```

在上述示例中，`mcx` 表示一个四控制 `X` 门。应用该门时，需要提供 4 个控制量子比特和 1 个目标量子比特。

---

## 属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `num_ctrl_qubits` | `int` | 多控制门的控制位总数。 |
| `num_qubits` | `int` | 门作用的总量子位数量，等于控制位数量加基础门作用位数量。 |
| `num_params` | `int` | 基础门需要的参数数量。 |
| `base_gate` | `StandardGate` | 未增加控制前的基础标准门。 |
| `params` | `list[Parameter]` | 基础门当前已经绑定的参数。 |

```python
from cqlib import Parameter
from cqlib.circuit.gates import MCGate, StandardGate

theta = Parameter("theta")
gate = MCGate(2, StandardGate.RZ(theta))

assert gate.num_ctrl_qubits == 2
assert gate.num_qubits == 3
assert gate.num_params == 1
```

在上述示例中，基础门是一个单量子比特 `RZ(theta)` 门，外层额外增加了 2 个控制位，因此该 `MCGate` 总共作用在 3 个量子比特上：前两个是控制位，最后一个是 `RZ` 的目标位。

---

## 控制位和目标位顺序

在将 `MCGate` 追加到线路中时，量子比特顺序须按照“控制位在前、目标位在后”的规则传入：

```text
[control_0, control_1, ..., control_k, target_0, target_1, ...]
```

例如，`MCGate(2, StandardGate.X)` 等价于一个 Toffoli 形式的两控制 `X` 门。

```python
from cqlib import Circuit
from cqlib.circuit.gates import MCGate, StandardGate

circuit = Circuit(3)

ccx = MCGate(2, StandardGate.X)
circuit.append_mc_gate(ccx, [0, 1, 2])
```

在上述示例中，量子比特 `0` 和 `1` 是控制位，量子比特 `2` 是目标位。只有当控制位满足控制条件时，`X` 门才会作用到目标位上。

---

## 矩阵

```python
matrix(params: list[float] | None = None) -> np.ndarray[np.complex128]
```

`matrix()` 用于返回多控制门的数值酉矩阵。矩阵维度由 `num_qubits` 决定，大小为 `2**num_qubits × 2**num_qubits`。对于参数化基础门，可以通过 `params` 参数传入具体数值，用于临时计算矩阵。

```python
import numpy as np
from cqlib.circuit.gates import MCGate, StandardGate

gate = MCGate(1, StandardGate.RZ)
matrix = gate.matrix([np.pi / 2])

assert matrix.shape == (4, 4)
```

在上述示例中，`gate` 是一个单控制 `RZ` 门，总共作用在 2 个量子比特上，因此矩阵大小为 `4 × 4`。

需要注意的是，`matrix()` 返回的是该门自身的局部矩阵，不包含所在 `Circuit` 的全局相位，也不考虑线路中其他操作。

---

## 反门

```python
inverse() -> MCGate
```

`inverse()` 用于返回当前多控制门的逆门。多控制门的逆可以理解为：保持控制结构不变，仅将基础门替换为其逆门。也就是说，多控制门的逆等价于“控制基础门的逆”。

```python
from cqlib.circuit.gates import MCGate, StandardGate

gate = MCGate(1, StandardGate.S)
inverse = gate.inverse()

assert inverse.base_gate == StandardGate.SDG
```

在上述示例中，`S` 门的逆为 `SDG`，因此单控制 `S` 门的逆为单控制 `SDG` 门。对于自反门，例如 `X`、`Z`、`H` 等，其多控制形式的逆通常与自身相同。

---

## 与标准受控门的关系

Cqlib 中的一些标准门本身已经表示常见受控门。例如，`CX` 是单控制 `X` 门，`CZ` 是单控制 `Z` 门，`CCX` 是两控制 `X` 门。从语义上看，这些标准门都可以看作 `MCGate` 的特例。

| 标准门 | 等价形式 |
| --- | --- |
| `StandardGate.CX` | `MCGate(1, StandardGate.X)` |
| `StandardGate.CY` | `MCGate(1, StandardGate.Y)` |
| `StandardGate.CZ` | `MCGate(1, StandardGate.Z)` |
| `StandardGate.CCX` | `MCGate(2, StandardGate.X)` |
| `StandardGate.CRX(theta)` | `MCGate(1, StandardGate.RX(theta))` |
| `StandardGate.CRY(theta)` | `MCGate(1, StandardGate.RY(theta))` |
| `StandardGate.CRZ(theta)` | `MCGate(1, StandardGate.RZ(theta))` |

---

## 自带控制门的基础门

如果基础门本身已经包含控制位，如 `StandardGate.CX`，再使用 `MCGate` 增加控制位时，新的控制位会叠加到原有控制结构之上。

```python
from cqlib.circuit.gates import MCGate, StandardGate

gate = MCGate(1, StandardGate.CX)

assert gate.num_ctrl_qubits == 2
assert gate.num_qubits == 3
```

在上述示例中，`StandardGate.CX` 本身已经包含 1 个控制位和 1 个目标位。`MCGate(1, StandardGate.CX)` 又在外层增加了 1 个控制位，因此整体上是一个三量子比特门，具有 2 个控制位和 1 个目标位。

---

## `MCGate` 与 `Circuit`

将 `MCGate` 添加到线路中时，可以使用 `Circuit.append_mc_gate()`。该接口会检查传入量子比特数量是否与 `gate.num_qubits` 一致，并在必要时检查参数、量子比特重复引用等合法性。

```python
from cqlib import Circuit
from cqlib.circuit.gates import MCGate, StandardGate

circuit = Circuit(4)

mcx = MCGate(3, StandardGate.X)
circuit.append_mc_gate(mcx, [0, 1, 2, 3])
```
