# 线路分析与转换

完成 Circuit 构造后，线路通常还需要进入进一步的分析、验证与转换流程。您可以根据实际需求检查操作序列、统计门类型、绑定符号参数、展开复合门、生成逆线路、转换为矩阵表示，或对线路结构进行一致性校验。这些能力是连接线路构造、算法验证、IR 转换、编译优化和后端执行的重要基础。

本节将系统介绍 `cqlib.circuit` 中与线路分析和结构转换相关的常用接口，帮助您理解一条线路在构造完成后如何被检查、复用、变换和验证。

---

## 检查操作序列

`Circuit.operations` 用于返回线路中的有序 `ValueOperation` 列表。该列表按照操作被追加到线路中以先后顺序保存。

每个 `ValueOperation` 都可以进一步拆分为：
- `instruction` 用于描述该操作对应的门、指令或控制流结构；
- `qubits` 表示该操作作用的量子比特；
- `params` 表示本次操作携带的参数；
- `label` 用于记录调试标签或转换来源。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

c = Circuit(2)
c.h(0)
c.cx(0, 1)
c.rz(1, theta)

for index, op in enumerate(c.operations):
    if op.instruction.is_instruction:
        instruction = op.instruction.instruction
        name = instruction.name
    else:
        name = op.instruction.classical_control.kind

    qubits = [q.index for q in op.qubits]
    print(index, name, qubits, op.params, op.label)
```

除遍历完整操作列表外，您也可以通过索引访问单个操作。`circuit[i]` 与 `circuit.operation(i)` 都可以用于获取指定位置的 `ValueOperation`。

```python
first = c[0]
second = c.operation(1)

print(first.instruction.instruction.name)   # H
print(second.instruction.instruction.name)  # CX
```

---

## 统计线路信息

在线路分析和编译优化过程中，通常需要统计线路中的门类型、操作数量或特定指令的出现次数。这类统计信息可用于评估线路规模、比较优化前后的变化，或在测试中验证编译 pass 是否产生了预期结果。

在Cqlib中，您可以通过 `Circuit.operations` 实现所需的统计逻辑。由于 `operations` 保存了线路中的完整操作序列，因此可以通过遍历操作列表提取指令名称、作用比特、参数和控制流类型等信息。

```python
from collections import Counter
from cqlib import Circuit

c = Circuit(3)
c.h(0)
c.cx(0, 1)
c.cx(1, 2)
c.rzz(0, 2, 0.5)

names = []
for op in c.operations:
    if op.instruction.is_instruction:
        names.append(op.instruction.instruction.name)
    else:
        names.append(op.instruction.classical_control.kind)

print(Counter(names))
```

---

## 生成逆线路

`inverse()` 用于生成当前线路对应的逆线路。对于一条仅包含可逆量子门的线路，其逆线路表示与原线路相反的量子演化过程。具体而言，Cqlib 会反转原线路中的操作顺序，并将每个操作替换为对应的逆操作，从而得到一条新的 `Circuit`。

```python
import numpy as np
from cqlib import Circuit

c = Circuit(2)
c.h(0)
c.cx(0, 1)
c.rz(1, 0.25)

inv = c.inverse()

product = inv.to_matrix() @ c.to_matrix()
print(np.allclose(product, np.eye(4), atol=1e-10))

print([op.instruction.instruction.name for op in c.operations])
print([op.instruction.instruction.name for op in inv.operations])
```

需要注意的是：

- `inverse()` 不会修改原始线路，而是返回一条新的 `Circuit`；
- `Barrier` 不改变量子态，可视为自身的逆，因此会被保留；
- `Measure`、`Reset`、经典控制流等非可逆结构无法生成普通逆线路。

```python
from cqlib import Circuit
from cqlib.circuit import CircuitError

c = Circuit(1)
c.measure(0)

try:
    c.inverse()
except CircuitError:
    print("measurement is not invertible")
```

---

## 分解复合门

`decompose()` 用于展开线路中的复合门，并返回一条新的 `Circuit`。当线路中包含由 `CircuitGate` 表示的复合门时，`decompose()` 会将其替换为该复合门内部定义的原始操作序列。该接口常用于矩阵验证、IR 导出、编译优化以及不支持复合门的后端适配流程。

```python
from cqlib import Circuit

sub = Circuit(2)
sub.h(0)
sub.cx(0, 1)
bell = sub.to_gate("Bell")

main = Circuit(2)
main.append_circuit_gate(bell, [0, 1])

print(len(main))  # 1

flat = main.decompose()
print(len(flat))  # 2
print([op.instruction.instruction.name for op in flat.operations])
```

对于参数化复合门，`decompose()` 会按照追加复合门时传入的位置参数进行绑定和展开。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

sub = Circuit(1)
sub.rx(0, theta)
block = sub.to_gate("RxBlock")

main = Circuit(1)
main.append_circuit_gate(block, [0], [0.75])

flat = main.decompose()
print(flat[0].params)  # [0.75]
```

---

## 单个操作的矩阵

Cqlib 支持对单个 `ValueOperation` 计算矩阵。`ValueOperation.matrix()` 用于获取某一次具体操作对应的矩阵表示，适合在门级测试、编译规则验证、门分解结果检查和局部操作分析中使用。

```python
import numpy as np
from cqlib import Qubit
from cqlib.circuit import StandardGate, ValueOperation

op = ValueOperation.from_standard_gate(StandardGate.X(), [Qubit(0)])
matrix = op.matrix()

print(np.allclose(matrix, np.array([[0, 1], [1, 0]], dtype=complex)))
```

需要注意的是，该接口仅适用于普通幺正操作。

---

## 转换为门

在量子算法开发中，通常需要将一段已经构造好的线路作为可复用模块，在其他线路中多次调用。`to_gate(name)` 用于将当前线路封装为 `CircuitGate`，从而把一段子线路转换为一个具有名称的复合门。

这种方式适合表达结构化算法模块，例如状态制备模块、oracle、ansatz block、纠缠层或重复使用的线路片段。与直接使用 `compose()` 追加线路不同，`to_gate()` 会保留模块边界，使主线路在结构上更加清晰，也便于后续进行复合门分解、参数绑定、IR 导出和编译优化。

```python
from cqlib import Circuit

sub = Circuit(1)
sub.h(0)

gate = sub.to_gate("HadamardBlock")

main = Circuit(1)
main.append_circuit_gate(gate, [0])
```

当后续分析流程需要查看复合门内部结构时，可以调用 `decompose()` 将复合门展开为原始操作序列。

```python
flat = main.decompose()
print(flat.to_matrix())
```

---

## 线路组合与重映射

`compose()` 用于将一条线路中的操作追加到另一条线路之后。通过该接口，您可以将多个线路片段按顺序拼接成一条完整线路，也可以在追加过程中对量子比特进行重映射，使子线路中的逻辑比特作用到主线路中的指定比特上。

```python
from cqlib import Circuit

prefix = Circuit(3)
prefix.h(0)

block = Circuit(2)
block.cx(0, 1)

prefix.compose(block, [1, 2])

print([op.instruction.instruction.name for op in prefix.operations])
print([q.index for q in prefix[1].qubits])
```

---

## 结构校验

`validate()` 用于检查线路对象的内部一致性，帮助您在后续分析、转换、编译或执行之前尽早发现潜在结构问题：
- 对于普通静态线路，校验通常关注量子比特引用、操作对象和参数结构是否一致；
- 对于包含测量、经典变量和控制流的动态线路，校验还会进一步检查经典句柄、测量结果引用以及控制流结构是否满足线路语义要求。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

c = Circuit(1)
c.measure(0)
c.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))

c.validate()
```

---



## 下一步

- [控制流](5_control_flow.md)：使用测量结果、经典变量和结构化控制流构造动态线路。
- [中间表示](../1_ir/0_overview.md)：掌握 Circuit 与 IR 之间的双向转换流程。
- [QCIS 支持](../1_ir/1_qcis.md)：将 Cqlib 线路导出为 QCIS 指令或从 QCIS 文件加载线路。

