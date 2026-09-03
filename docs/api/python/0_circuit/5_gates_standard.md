# StandardGate

`cqlib.circuit.gates.StandardGate`  

```python
from cqlib import Parameter
from cqlib.circuit.gates import StandardGate
```

`StandardGate` 表示 Cqlib 内置的标准量子门集合，用于描述常见的单量子比特门、多量子比特门、参数化旋转门、受控门、二体 Pauli 相互作用门以及部分硬件相关门。它是 `Circuit` 线路构造、门级分析、矩阵计算、编译转换和 IR 导出过程中的基础门类型之一。

---

## 基本操作

标准门可以通过 `Circuit.append_gate()` 显式追加到线路中。

```python
from cqlib import Circuit, Parameter
from cqlib.circuit.gates import StandardGate

theta = Parameter("theta")

circuit = Circuit(2)
circuit.append_gate(StandardGate.H, [0])
circuit.append_gate(StandardGate.RX(theta), [0])
circuit.append_gate(StandardGate.CX, [0, 1])
```

在上述示例中，`StandardGate.H` 和 `StandardGate.CX` 是不带参数的固定门；`StandardGate.RX(theta)` 是带有参数 `theta` 的参数化门实例。追加门时，`qubits` 参数用于指定门作用的量子比特，量子比特数量必须与该门的 `num_qubits` 保持一致。

---

## 标准门分类

Cqlib 内置标准门可以按照作用量子比特数量和参数形式进行分类：

### 1. 单量子比特固定门

单量子比特固定门作用于一个量子比特，使用时不需要提供角度参数。

| 门 | 说明 |
| --- | --- |
| `StandardGate.I` | 恒等门，不改变量子态。 |
| `StandardGate.H` | Hadamard 门，用于构造叠加态。 |
| `StandardGate.X` | Pauli-X 门，等价于比特翻转。 |
| `StandardGate.Y` | Pauli-Y 门。 |
| `StandardGate.Z` | Pauli-Z 门，用于相位翻转。 |
| `StandardGate.S` | S 相位门，等价于 `sqrt(Z)`。 |
| `StandardGate.SDG` | `S` 的逆门。 |
| `StandardGate.T` | T 相位门，对应 `pi/4` 相位。 |
| `StandardGate.TDG` | `T` 的逆门。 |
| `StandardGate.X2P` | X 方向正半角门。 |
| `StandardGate.X2M` | X 方向负半角门。 |
| `StandardGate.Y2P` | Y 方向正半角门。 |
| `StandardGate.Y2M` | Y 方向负半角门。 |

### 2. 参数化单量子比特门

参数化单量子比特门通过一个或多个角度参数控制门的作用效果。

| 门 | 参数数量 | 说明 |
| --- | --- | --- |
| `StandardGate.RX(theta)` | 1 | X 轴旋转门，通常表示 `exp(-i theta X / 2)`。 |
| `StandardGate.RY(theta)` | 1 | Y 轴旋转门，通常表示 `exp(-i theta Y / 2)`。 |
| `StandardGate.RZ(theta)` | 1 | Z 轴旋转门，通常表示 `exp(-i theta Z / 2)`。 |
| `StandardGate.RXY(theta, phi)` | 2 | XY 平面内任意轴旋转门。 |
| `StandardGate.U(theta, phi, lambda_)` | 3 | 通用单量子比特门。 |
| `StandardGate.Phase(lambda_)` | 1 | 相位门 `P(lambda)`。 |
| `StandardGate.XY(theta)` | 1 | XY 交互相关门族。 |
| `StandardGate.XY2P(theta)` | 1 | 正半角 XY 门。 |
| `StandardGate.XY2M(theta)` | 1 | 负半角 XY 门。 |

参数可以是普通数值，也可以是 `Parameter` 表达式。使用符号参数时，该门可以作为参数化线路的一部分，在后续通过 `Circuit.assign_parameters()` 进行绑定。

### 3. 多量子比特门

多量子比特门用于描述量子比特之间的相互作用，是构造纠缠态、受控逻辑和量子算法核心线路的重要基础。

| 门 | 参数数量 | 说明 |
| --- | --- | --- |
| `StandardGate.CX` | 0 | controlled-X，即 CNOT 门。 |
| `StandardGate.CY` | 0 | controlled-Y 门。 |
| `StandardGate.CZ` | 0 | controlled-Z 门。 |
| `StandardGate.SWAP` | 0 | 交换两个量子比特的状态。 |
| `StandardGate.CCX` | 0 | Toffoli 门，即双控制 X 门。 |
| `StandardGate.RXX(theta)` | 1 | XX 旋转门。 |
| `StandardGate.RYY(theta)` | 1 | YY 旋转门。 |
| `StandardGate.RZZ(theta)` | 1 | ZZ 旋转门。 |
| `StandardGate.RZX(theta)` | 1 | ZX 旋转门。 |
| `StandardGate.CRX(theta)` | 1 | controlled-RX 门。 |
| `StandardGate.CRY(theta)` | 1 | controlled-RY 门。 |
| `StandardGate.CRZ(theta)` | 1 | controlled-RZ 门。 |
| `StandardGate.FSIM(theta, phi)` | 2 | fSim 门，常见于部分超导量子计算模型。 |

对于受控门，调用 `append_gate()` 时通常按照“控制比特在前、目标比特在后”的顺序传入量子比特。例如，`StandardGate.CX` 应作用在 `[control, target]` 上。

### 4. 全局相位门

| 门 | 参数数量 | 说明 |
| --- | --- | --- |
| `StandardGate.GPhase(lambda_)` | 1 | 零量子比特全局相位标记。 |

---

## 属性

`StandardGate` 提供了一组属性，用于查询门的基本结构信息。这些属性常用于线路校验、编译规则匹配、门集覆盖检查和文档生成。

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `num_qubits` | `int` | 门作用的总量子比特数量。 |
| `num_ctrl_qubits` | `int` | 门内置控制比特数量。 |
| `num_params` | `int` | 门所需参数数量。 |
| `params` | `list[Parameter]` | 当前门实例已经绑定的参数列表。 |

```python
from cqlib.circuit.gates import StandardGate

assert StandardGate.H.num_qubits == 1
assert StandardGate.CX.num_ctrl_qubits == 1
assert StandardGate.RX.num_params == 1

gate = StandardGate.RZ(0.5)
assert len(gate.params) == 1
```

---

## 参数绑定

参数化标准门可以通过调用门工厂并传入参数来生成带参数的门实例。

```python
gate(*args: float | Parameter) -> StandardGate
```

```python
from cqlib import Parameter
from cqlib.circuit.gates import StandardGate

theta = Parameter("theta")

rx = StandardGate.RX(theta)
u = StandardGate.U(theta, 0.1, 0.2)
```

---

## 矩阵计算

```python
matrix(params: list[float] | None = None) -> np.ndarray[np.complex128]
```

`matrix()` 用于返回标准门的局部数值酉矩阵，即该门自身在其作用量子比特空间上的矩阵。

对于固定门，可以直接调用 `matrix()`。对于参数化门，有两种常见方式：

- 如果门实例已经绑定了可求值的常量参数，可以直接调用 `matrix()`；
- 如果门实例或门定义中包含符号参数，需要在 `matrix(params)` 中传入具体数值参数，或先在线路中完成参数绑定后再进行矩阵转换。

```python
import numpy as np
from cqlib.circuit.gates import StandardGate

h = StandardGate.H.matrix()
rx = StandardGate.RX.matrix([np.pi / 2])

assert h.shape == (2, 2)
assert rx.shape == (2, 2)
```

---

## 反门

```python
inverse() -> StandardGate
```

`inverse()` 返回当前标准门的逆门。对于自反门，例如 `H`、`X`、`Y`、`Z`、`CX`、`CZ` 等，逆门通常就是其自身；对于相位门，`S` 与 `SDG`、`T` 与 `TDG` 互为逆；对于参数化旋转门，逆门通常对应角度取相反数或按门定义返回等价逆门。

```python
from cqlib.circuit.gates import StandardGate

assert StandardGate.H.inverse() == StandardGate.H
assert StandardGate.S.inverse() == StandardGate.SDG
assert StandardGate.T.inverse() == StandardGate.TDG
```

当门实例包含参数时，`inverse()` 会按照该门的逆变换规则处理参数。该能力常用于构造逆线路、uncompute 结构、门级测试和编译优化验证。

---

## 生成多控制门

```python
control(num_controls: int) -> MCGate
```

`control(num_controls)` 用于在已有标准门基础上增加指定数量的控制比特，并返回一个 `MCGate`。该接口适合构造多控制 `X` 门、多控制相位门、多控制旋转门以及 `oracle` 中常见的受控操作。

```python
from cqlib.circuit.gates import StandardGate

mcx = StandardGate.X.control(3)

assert mcx.num_ctrl_qubits == 3
assert mcx.base_gate == StandardGate.X
```

应用多控制门时，量子比特通常按照“控制比特在前、目标比特在后”的顺序传入。例如，三控制 `X` 门共需要四个量子比特，前三个为控制比特，最后一个为目标比特。

```python
circuit.append_mc_gate(mcx, [0, 1, 2, 3])  # controls: 0, 1, 2; target: 3
```

---

## `StandardGate` 与 `Circuit` 

`Circuit` 提供了大量便捷门方法，这些方法本质上会向线路中追加对应的 `StandardGate`：

| `Circuit` 方法 | 对应标准门 |
| --- | --- |
| `h(q)` | `StandardGate.H` |
| `x(q)` | `StandardGate.X` |
| `rx(q, theta)` | `StandardGate.RX(theta)` |
| `cx(control, target)` | `StandardGate.CX` |
| `rzz(a, b, theta)` | `StandardGate.RZZ(theta)` |
| `fsim(a, b, theta, phi)` | `StandardGate.FSIM(theta, phi)` |
