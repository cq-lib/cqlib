# Qubit

`cqlib.circuit.Qubit`

```python
from cqlib import Qubit
```


`Qubit` 是 Cqlib 中用于表示逻辑量子比特的基础类型。它通常作为量子线路中量子比特的句柄使用，用一个非负整数标识逻辑量子比特编号。需要注意的是，`Qubit` 只表示一个逻辑编号，并不保存量子态本身，也不绑定具体的物理设备位置。它同样不记录自己属于哪一条 `Circuit`。某个 `Qubit` 是否可以在线路中使用，取决于该量子比特是否已经被当前 `Circuit` 注册。

---

## 构造函数

```python
Qubit(index: int)
```

`Qubit(index)` 用于创建一个逻辑量子比特句柄。`index` 必须是非负整数，内部会以无符号整数形式保存。

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `index` | `int` | 逻辑量子比特编号，需为非负整数。 |

示例：

```python
from cqlib import Qubit

q0 = Qubit(0)
q5 = Qubit(5)

print(q0.index)
print(q5.index)
```

---

## 属性

`Qubit` 提供以下常用属性：

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `index` | `int` | 逻辑量子比特编号。 |
| `id` | `int` | 内部原始编号，默认与 `index` 数值一致。 |

通常情况下，用户直接使用 `index` 即可。`id` 更多用于底层调试、内部表示或与其他底层数据结构对齐的场景。

```python
from cqlib import Qubit

q = Qubit(3)

print(q.index)  # 3
print(q.id)     # 3
```

---

## 比较、排序与哈希

`Qubit` 支持常见的比较、排序和哈希操作，包括：

- `==` / `!=`
- `<` / `<=` / `>` / `>=`
- `hash()`
- `copy.copy()` / `copy.deepcopy()`
- `str()` / `repr()`

这些操作都基于量子比特编号进行判断。也就是说，两个 `Qubit` 对象只要编号相同，就会被视为相同的逻辑量子比特。

```python
from cqlib import Qubit

assert Qubit(0) == Qubit(0)
assert Qubit(0) < Qubit(1)

mapping = {Qubit(0): "ancilla"}
assert mapping[Qubit(0)] == "ancilla"
```

---

## `Circuit` 与 `Qubit`

大多数 `Circuit` 门方法既接受整数索引，也接受 `Qubit` 对象。对于简单线路，直接使用整数索引通常更加简洁；对于需要显式管理逻辑量子比特编号的场景，使用 `Qubit` 对象会更加清晰。

```python
from cqlib import Circuit, Qubit

circuit = Circuit(2)

circuit.h(0)
circuit.cx(Qubit(0), Qubit(1))
```

在上述示例中，`h(0)` 和 `cx(Qubit(0), Qubit(1))` 都表示对当前线路中已注册的逻辑量子比特进行操作。

构造线路时，也可以直接传入 `Qubit` 列表：

```python
from cqlib import Circuit, Qubit

circuit = Circuit([Qubit(10), Qubit(20)])
circuit.cx(Qubit(10), Qubit(20))
```

---

## 稀疏逻辑索引

`Qubit` 的编号是逻辑标识，不要求从 `0` 开始连续排列。例如，`Qubit(10)` 和 `Qubit(20)` 可以共同构成一条两比特线路。

```python
from cqlib import Circuit, Qubit

circuit = Circuit([Qubit(10), Qubit(20)])

assert circuit.num_qubits == 2
assert [q.index for q in circuit.qubits] == [10, 20]
```

需要注意的是，逻辑编号不一定等同于矩阵或状态向量中的轴顺序。对于 `Circuit([Qubit(10), Qubit(20)])`，线路内部只包含两个量子比特，矩阵维度为 `4 × 4`。默认情况下，矩阵转换会按照线路中保存的 `qubits` 顺序解释量子比特。

如果需要明确控制矩阵比特顺序，可以在矩阵转换时传入 `qubits_order`：

```python
matrix = circuit.to_matrix([20, 10])
```

这表示在构造矩阵时，按照逻辑量子比特 `20`、`10` 的顺序解释比特轴。

---

## 语义边界

`Qubit` 是逻辑量子比特的“地址”或“句柄”，不是量子态对象。相同编号的 `Qubit` 可以出现在不同线路中，但这些线路彼此独立。

```python
from cqlib import Circuit, Qubit

q0 = Qubit(0)

left = Circuit([q0])
right = Circuit([q0])

left.x(q0)
right.h(q0)
```

在上述示例中，`left` 和 `right` 都使用了 `Qubit(0)`，但它们是两条不同线路，彼此不会相互影响。

---

## 与物理量子比特的区别

`Qubit.index` 表示逻辑量子比特编号，不等同于真实设备上的物理量子比特编号。在面向硬件执行时，逻辑量子比特通常需要经过布局和映射，才能对应到具体设备上的物理量子比特。例如，逻辑量子比特 `Qubit(0)` 在某个设备上可能被映射到物理量子比特 `Q5`，在另一个设备或编译结果中又可能被映射到其他物理位置。

---

## 完整示例

下面的示例展示了 `Qubit` 的典型用法，包括构造逻辑量子比特、创建稀疏编号线路、添加门操作以及指定矩阵比特顺序。

```python
from cqlib import Circuit, Qubit

q10 = Qubit(10)
q20 = Qubit(20)

circuit = Circuit([q10, q20])
circuit.h(q10)
circuit.cx(q10, q20)

print(circuit.num_qubits)
print([q.index for q in circuit.qubits])

matrix_default = circuit.to_matrix()
matrix_reversed = circuit.to_matrix([20, 10])

print(matrix_default.shape)
print(matrix_reversed.shape)
```