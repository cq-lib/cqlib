# 量子线路

`cqlib.circuit` 是 Cqlib 中描述量子程序的基础模块。它负责表达量子比特、门操作、参数表达式、复合线路、非幺正指令、测量结果以及由经典表达式驱动的动态控制流。后续的 [IR 转换](../1_ir/0_overview.md)、[QIS 模拟](../3_qis/0_overview.md)、[编译优化](../4_compiler/0_overview.md)、[设备映射](../2_device/0_overview.md)和[可视化](../5_visualization/0_overview.md)模块，通常都以 `Circuit` 作为输入或中间表示。

---

## 核心抽象

`cqlib.circuit` 围绕 `Circuit` 构建了一组相互配合的对象，用于描述从基础量子门到动态线路控制流的完整线路结构。

| 抽象 | 作用 | 常用入口 |
|---|---|---|
| `Qubit` | 量子比特标识符，只保存非负索引 | `Qubit(0)`|
| `Circuit` | 量子线路容器，保存量子比特、参数、经典值和操作序列 | `Circuit(...)` |
| `StandardGate` | Cqlib 内置标准门集合，包括 Pauli、Clifford、旋转门、双比特门等 | `StandardGate.H`, `StandardGate.RX(theta)` |
| `MCGate` | 多控制门，把一个标准门提升为带多个控制比特的门 | `MCGate(2, StandardGate.X())` |
| `UnitaryGate` | 用户自定义幺正门，可通过数值矩阵、符号矩阵或不可变子线路定义 | `UnitaryGate("Oracle", 2)` |
| `CircuitGate` | 由子线路转换得到的复合门 | `sub.to_gate("Block")` |
| `Directive` | 非幺正指令，如 barrier、measure、reset | `circuit.measure(0)` |
| `Parameter` | 符号参数表达式 | `Parameter("theta")` |
| `ClassicalType` / `ClassicalExpr` | 动态线路中的经典类型与经典表达式 | `ClassicalType.bit()`, `m.expr()` |
| `ValueOperation` | 完整操作表示：指令 + 比特 + 参数 + 可选标签 | `circuit.operations[0]` |

---

## 核心功能

### 1. 静态线路

静态线路是最基础的使用方式。在线路宽度固定的情况下，您可以按照执行顺序依次追加量子门和指令。

```python
from cqlib import Circuit

c = Circuit(3)
c.h(0)
c.cx(0, 1)
c.rzz(1, 2, 0.25)
c.barrier([0, 1, 2])
c.reset(2)
```

静态线路适用于算法原型验证、编译优化、QIS 模拟和 IR 导出等场景。对于不包含测量、重置和控制流的纯酉线路，还可以进一步转换为矩阵表示，用于小规模线路验证。

### 2. 参数化线路

参数化线路使用 `Parameter` 作为角度或表达式占位符，常用于 VQE、QAOA、量子机器学习、参数扫描和梯度计算等场景。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
phi = Parameter("phi")

c = Circuit(2)
c.rx(0, theta)
c.ry(1, phi)
c.cx(0, 1)

print(c.symbols)  # ['theta', 'phi']

bound = c.assign_parameters({"theta": 0.1, "phi": 0.2})
print(bound.to_matrix())
```

值得注意的是，`assign_parameters()` 会返回一条新的已绑定线路，不会修改原始参数化线路模板。因此，同一个参数化线路可以被多次复用，从而用于不同参数点的扫描和优化。

### 3. 子线路与复合门

实际量子算法通常由若干可复用的线路模块组成。Cqlib 支持将一条 `Circuit` 转换为 `CircuitGate`，再像普通量子门一样追加到其他线路中。

```python
from cqlib import Circuit

bell = Circuit(2)
bell.h(0)
bell.cx(0, 1)

bell_gate = bell.to_gate("Bell")

main = Circuit(4)
main.append_circuit_gate(bell_gate, [0, 1])
main.append_circuit_gate(bell_gate, [2, 3])

flat = main.decompose()
```

`CircuitGate` 适合表达算法中的可复用模块，例如纠缠块、特征映射块、oracle 子程序等。此外，可以使用 `decompose()` 将复合门展开为基础操作，便于后续矩阵验证、编译优化或 IR 导出。

### 4. 自定义门与多控制门

当内置门集合不足以描述某个算法单元时，可以使用 `UnitaryGate` 定义自定义酉门，也可以使用 `MCGate` 构造多控制门。


```python
import numpy as np
from cqlib import Circuit
from cqlib.circuit import MCGate, StandardGate, UnitaryGate

c = Circuit(3)

controlled_h = MCGate(2, StandardGate.H())
c.append_mc_gate(controlled_h, [0, 1, 2])

custom_x = UnitaryGate("CustomX", 1).with_matrix([[0, 1], [1, 0]])
c.append_unitary_gate(custom_x, [2])
```

多控制门和自定义门常用于算法库、oracle 构造、受控子程序以及硬件专用指令建模。请注意，使用自定义酉门时，应确保矩阵维度与作用量子比特数量一致，并且矩阵满足酉性要求。

### 5. 动态线路

动态线路允许在线路执行过程中进行路中测量，并根据测量结果或经典变量控制后续操作。这里，Cqlib 通过 `ClassicalType`、`ClassicalExpr` 和结构化控制流接口描述这类线路。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr, ClassicalType

c = Circuit(1)
measurement = c.measure(0)

condition = measurement.expr().to_bool()
c.if_(condition, lambda body: body.x(0))

flag = c.var(ClassicalType.bool())
c.store(flag, ClassicalExpr.bool_literal(True))
c.while_(flag.expr(), lambda body: body.break_loop())

c.validate()
```

请注意，动态线路通常无法表示为单一酉矩阵，因此不能直接使用 `to_matrix()` 进行整体矩阵转换。对于动态线路，应优先使用结构化验证、IR 转换或支持动态执行语义的后端流程。

### 6. 线路分析与转换

线路构造完成后，可以进一步执行参数绑定、反演、分解、数值矩阵转换或符号矩阵转换等操作。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
c = Circuit(1)
c.rx(0, theta)

inverse = c.inverse()
symbolic = c.to_symbolic_matrix()
numeric = c.assign_parameters({"theta": 0.3}).to_matrix()
```
---

## 下一步

- [量子门与指令](1_gates.md)：了解内置门、自定义门、复合门和非酉指令。
- [线路结构与构造](2_structures.md)：掌握 `Circuit` 的生命周期、索引、组合和操作表示。
- [参数系统](3_parameters.md)：学习参数表达式、参数绑定、表达式化简、符号求导和符号矩阵。