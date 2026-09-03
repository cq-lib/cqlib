# 线路结构与构造

`Circuit` 是 `cqlib.circuit` 模块中的核心线路容器，用于表示一段完整的量子程序。它不仅记录量子比特和按顺序排列的线路操作，还负责维护参数表达式、全局相位、经典变量、测量结果以及动态线路所需的经典句柄命名空间。因此，`Circuit` 既是用户构造量子线路的主要入口，也是后续 IR 转换、编译优化、设备映射和结果分析等流程的重要数据基础。

从结构上看，一条 `Circuit` 通常包含以下信息：

* 量子比特列表，用于描述线路中可操作的逻辑量子比特；
* 按追加顺序排列的操作列表，用于记录量子门、指令和控制流结构；
* 线路中的自由参数，用于支持参数化线路、参数绑定和变分算法；
* 全局相位，用于保留线路整体相位信息；
* 动态线路中使用的经典变量和经典值，用于描述测量结果和经典控制逻辑；
* 由 `CircuitId` 标识的经典句柄命名空间，用于保证经典变量和测量值在线路内部的一致性。

上述结构共同构成了 Cqlib 量子线路的基础表示，使线路能够在构造、组合、参数绑定、矩阵转换、IR 导出、编译优化和动态控制流分析等不同阶段保持统一的数据语义。


---

## 创建线路

`Circuit` 支持多种线路创建方式。您既可以直接指定量子比特数量，也可以显式传入量子比特索引列表或 `Qubit` 对象。不同方式适用于不同的建模需求：当量子比特编号连续时，直接传入整数最为简洁；当需要保留特定逻辑编号或与外部系统中的比特编号对齐时，可以使用整数索引列表或显式的 `Qubit` 对象。

### 1. 使用量子比特数量

最常见的方式是向 `Circuit` 传入一个整数 `n`。此时，Cqlib 会自动创建 `n` 个逻辑量子比特，并按照 `0..n-1` 的顺序分配索引。

```python
from cqlib import Circuit

c = Circuit(3)
print(c.num_qubits)             # 3
print([q.index for q in c.qubits])  # [0, 1, 2]
```

在上述示例中，`Circuit(3)` 创建了一条包含 3 个量子比特的线路，对应的逻辑量子比特索引分别为 `0`、`1` 和 `2`。后续添加门操作时，可以直接使用这些整数索引指定门的作用对象，例如 `c.h(0)` 或 `c.cx(0, 1)`。

此外，`Circuit(0)` 也是合法的构造方式，表示一条不包含量子比特和操作的零比特线路。零比特线路通常用于边界测试、递归构造、程序化生成线路时的初始占位，或验证接口在空输入下的行为。

```python
from cqlib import Circuit

empty = Circuit(0)
print(empty.width)      # 0
print(len(empty))       # 0
print(empty.to_matrix())
```

### 2. 使用整数索引列表

当线路中的逻辑量子比特编号不是连续的 `0..n-1`，或者需要与外部数据、设备映射、算法模型中的编号保持一致时，可以直接向 `Circuit` 传入整数索引列表。

```python
from cqlib import Circuit

c = Circuit([2, 0, 5])
print([q.index for q in c.qubits])  # [2, 0, 5]

c.h(2)
c.cx(2, 5)
```

需要注意的是，Cqlib 会保留传入列表中的量子比特顺序。该顺序不仅影响 `c.qubits` 的返回结果，也会影响默认矩阵构造时的量子比特排列方式。除非在矩阵转换时显式指定 `qubits_order`，否则 Cqlib 会按照线路内部保存的 `c.qubits` 顺序解释各个量子比特。


### 3. 使用 `Qubit` 对象

除整数索引外，Cqlib 同时支持显式创建 `Qubit` 对象以用于线路构造。`Qubit` 是 `Cqlib` 中表示逻辑量子比特的轻量句柄，主要用于封装非负整数索引。

```python
from cqlib import Circuit, Qubit

q0 = Qubit(10)
q1 = Qubit(11)

c = Circuit([q0, q1])
c.h(q0)
c.cx(q0, q1)

print(q0.index)
print(q0 == Qubit(10))
```

---

## 添加量子比特

在某些场景下，用户可能需要先构造一条较小的线路，再根据后续算法逻辑、子线路组合或程序化生成结果继续扩展线路宽度。此时可以使用 `add_qubits()` 在线路中添加新的量子比特。

`add_qubits()` 会在保留已有操作序列的基础上，将新的量子比特加入当前线路。新增量子比特不会影响已经存在的门操作，但会扩展线路的量子比特列表，并影响后续可添加操作的作用范围以及线路矩阵的维度。

```python
from cqlib import Circuit

c = Circuit(1)
c.h(0)

c.add_qubits([2, 4])
print(c.num_qubits)                 # 3
print([q.index for q in c.qubits])  # [0, 2, 4]
print(len(c.operations))            # 1

c.cx(0, 2)
```

在上述示例中，线路最初只包含量子比特 `0`，并已经添加了一个 `H` 门。调用 `add_qubits([2, 4])` 后，线路中新增了索引为 `2` 和 `4` 的量子比特，原有的 `H` 门仍然保留，后续可以继续在新增量子比特上添加操作。

使用 `add_qubits()` 时需要注意以下几点：

- 新增量子比特不能与线路中已有量子比特重复，否则会触发线路结构错误；
- 已有操作不会被重写或重新映射，新增量子比特只影响后续操作；
- 线路的量子比特顺序会影响默认矩阵构造时的比特排列；
- 新增量子比特会扩大线路宽度，因此在线路矩阵转换时，矩阵维度也会随之增加。

---

## 查询线路信息

在构造或调试量子线路时，Cqlib 支持查看线路的基本结构信息，如量子比特数量、已添加的操作、自由参数以及经典句柄命名空间等。`Circuit` 提供了一组常用属性，用于帮助您快速了解当前线路的状态，并为后续参数绑定、线路组合、矩阵转换或 IR 导出提供必要信息。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
c = Circuit(2)
c.rx(0, theta)
c.cx(0, 1)

print(c.id)              # 当前线路的 CircuitId 句柄标识
print(c.num_qubits)      # 2
print(c.width)           # 2
print(c.qubits)          # [Qubit(0), Qubit(1)]
print(c.parameters)      # [theta]
print(c.symbols)         # ['theta']
print(c.operations)      # [ValueOperation(...), ...]
print(len(c))            # 2
```

其中，`num_qubits` 和 `width` 均表示线路包含的量子比特数量；`qubits` 返回线路中注册的量子比特列表；`operations` 返回按追加顺序排列的操作列表；`len(c)` 返回当前线路中的操作数量。对于包含动态线路结构的场景，`id` 表示当前线路的经典句柄命名空间，用于区分不同线路中的经典变量和测量结果。

注意，`parameters` 和 `symbols` 的含义并不完全相同。`parameters` 返回线路中登记的参数表达式对象，而 `symbols` 返回这些表达式中包含的自由符号名称。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
phi = Parameter("phi")

c = Circuit(1)
c.rz(0, 2 * theta + phi)

print([str(p) for p in c.parameters])  # ['phi + 2*theta']
print(c.symbols)                       # ['theta', 'phi']
```

---

## 操作序列与索引

`Circuit` 会按照操作被追加到线路中的先后顺序维护一条有序操作序列。您可以通过 `operations` 属性查看完整操作列表，也可以使用 `operation(index)` 或 `circuit[index]` 访问指定位置的操作。该机制适用于线路检查、调试、转换器开发以及编译前后的结构对比。

```python
from cqlib import Circuit

c = Circuit(2)
c.h(0)
c.cx(0, 1)

first = c[0]
second = c.operation(1)

print(first.instruction.instruction.name)   # H
print(second.instruction.instruction.name)  # CX
print([q.index for q in second.qubits])     # [0, 1]
```

`Circuit` 中的每一项操作通常表示为 `ValueOperation`，它是线路中的完整操作对象，主要包含以下字段：
- `instruction`：操作对应的 `ValueInstruction`，可以表示普通量子门、非幺正指令或经典控制流结构；
- `qubits`：该操作作用的量子比特列表；
- `params`：该次操作携带的参数列表，常用于参数化旋转门、自定义门或复合门；
- `label`：可选的标签信息，通常用于调试、标记线路层、记录转换来源或辅助后续分析。

---

## 添加操作

在线路创建完成后，您可以向 `Circuit` 中持续追加量子门、复合门、非幺正指令或低层操作对象。Cqlib 提供了多种追加操作的方式，以适应不同的使用需求。

### 1. 使用便捷门方法

使用 `Circuit` 提供的便捷门方法是最常见的线路构造方式。此类方法直接以量子比特索引和必要参数作为输入，语义清晰，代码简洁，适合算法原型和日常量子线路开发。

```python
from cqlib import Circuit

c = Circuit(2)
c.h(0)
c.cx(0, 1)
c.rzz(0, 1, 0.25)
```

### 2. 使用显式 gate 对象

当需要保存门对象、检查门属性、添加标签，或通过程序逻辑批量生成门操作时，可以先显式构造 `gate` 对象，再通过相应的 `append_*` 方法追加到线路中。

```python
from cqlib import Circuit
from cqlib.circuit import MCGate, StandardGate

c = Circuit(3)
c.append_gate(StandardGate.H(), [0])
c.append_gate(StandardGate.RZ(0.25), [1], label="rz-calibrated")

mcx = MCGate(2, StandardGate.X())
c.append_mc_gate(mcx, [0, 1, 2])
```

常用的显式追加方法如下：

| 方法 | 用途 |
|---|---|
| `append_gate(gate, qubits, label=None)` | 追加 `StandardGate` |
| `append_mc_gate(gate, qubits, label=None)` | 追加 `MCGate` |
| `append_unitary_gate(gate, qubits, params=None)` | 追加 `UnitaryGate` |
| `append_circuit_gate(gate, qubits, params=None)` | 追加 `CircuitGate` |

### 3. 使用低层 `ValueOperation`

在更底层的开发场景中，也可以直接构造并追加完整的 `ValueOperation`。

```python
from cqlib import Circuit, Qubit
from cqlib.circuit import StandardGate, ValueOperation

operation = ValueOperation.from_standard_gate(
    StandardGate.RX(0.5),
    [Qubit(0)],
    label="manual-rx",
)

c = Circuit(1)
c.append(operation)
print(c[0].label)
```

---

## 从操作重建线路

除常规的逐步构造方式外，Cqlib 还提供了 `Circuit.from_operations()` 接口，用于根据已有的量子比特列表和操作序列重新构造线路。该接口属于较底层的线路构造入口，通常用于从序列化数据、IR 转换结果、编译器中间结果或测试用例中恢复一条完整线路。

与直接调用 `h()`、`cx()`、`append_gate()` 等方法逐步追加操作不同，`from_operations()` 适用于已准备好线路所需的量子比特和 `ValueOperation` 操作列表的场景。Cqlib 会基于这些信息重新生成 `Circuit` 对象，并保留原始操作顺序。

```python
from cqlib import Circuit

source = Circuit(2)
source.h(0)
source.cx(0, 1)

restored = Circuit.from_operations(source.qubits, source.operations)

print(restored.num_qubits)
print([op.instruction.instruction.name for op in restored.operations])
```
---

## 组合线路

在构造复杂量子程序时，通常需要将多条较小的线路组合成一条完整线路。`compose()` 用于将另一条线路中的操作追加到当前线路末尾，从而实现线路模块之间的顺序组合。

默认情况下，`compose()` 会按照两条线路中的量子比特编号进行组合；如果需要将子线路映射到主线路中的特定量子比特位置，可以通过 `qubits` 参数显式指定位置重映射关系。

```python
from cqlib import Circuit

main = Circuit(3)
main.h(0)

sub = Circuit(2)
sub.cx(0, 1)

main.compose(sub, [1, 2])

print([op.instruction.instruction.name for op in main.operations])
print([q.index for q in main[1].qubits])  # [1, 2]
```

请注意，使用 `compose()` 时需要遵循以下规则：

- 如果传入 qubits 参数，其长度必须与 other.num_qubits 一致；
- other 中第 i 个量子比特会被映射到 qubits[i] 指定的主线路量子比特；
- 组合后的操作顺序保持为“当前线路已有操作在前，other 的操作在后”；
  
---

## 子线路封装

在构造复杂量子算法时，许多线路片段会被多次复用，例如旋转层、纠缠层、oracle、ansatz block 或特定的算法子模块。对于这类可复用结构，Cqlib支持使用 `to_gate(name)` 将当前线路封装为 `CircuitGate`，再将生成的复合门追加到其他线路中。

该复合门可以在主线路的不同量子比特位置多次调用，并且可以在需要时通过 `decompose()` 展开为原始线路操作。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

block = Circuit(1)
block.rx(0, theta)
block.rz(0, theta / 2)

gate = block.to_gate("RotationBlock")

main = Circuit(2)
main.append_circuit_gate(gate, [0], [0.2])
main.append_circuit_gate(gate, [1], [0.4])

flat = main.decompose()
print([op.instruction.instruction.name for op in flat.operations])
```

---

## 全局相位

每条 `Circuit` 都包含一个 `global_phase` 属性，用于记录线路整体的全局相位。全局相位不会改变单次测量得到各个计算基态的概率分布，但它是量子线路数学表示的一部分，在矩阵对比、线路等价性判断、编译重写以及某些相位敏感的算法分析中具有重要意义。

在 Cqlib 中，`global_phase` 使用 `Parameter` 表示，因此既可以设置为普通数值，也可以设置为符号参数或参数表达式。

```python
from cqlib import Circuit, Parameter

c = Circuit(1)
print(c.global_phase.is_zero())

c.set_global_phase(0.25)
print(c.global_phase.evaluate({}))  # 0.25

alpha = Parameter("alpha")
c.set_global_phase(alpha)
print(c.global_phase)
print(c.symbols)
```

---

## 下一步

- [参数系统](3_parameters.md)：学习参数表达式、参数绑定、表达式化简、符号求导和符号矩阵。
- [线路分析与转换](4_circuit_analysis.md)：使用反演、分解、矩阵转换和操作检查等工具。
- [控制流](5_control_flow.md)：使用测量结果、经典变量和结构化控制流构造动态线路。
