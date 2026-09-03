# Circuit

`cqlib.circuit.Circuit`

```python
from cqlib import Circuit
```

`Circuit` 是 Cqlib Python API 中最核心的线路容器，用于表示一段完整的量子程序。它负责保存量子比特集合、按顺序排列的操作序列、符号参数、全局相位、经典变量与经典值，以及结构化控制流等信息。

---

## 构造线路

```python
Circuit(qubits: int | list[int] | list[Qubit])
```

`Circuit` 支持多种构造方式，既可以直接指定量子比特数量，也可以显式指定逻辑量子比特编号或传入已有的 `Qubit` 对象。

| 写法 | 含义 |
| --- | --- |
| `Circuit(3)` | 创建包含 `Qubit(0)`、`Qubit(1)`、`Qubit(2)` 的三比特线路。 |
| `Circuit([0, 2, 4])` | 使用稀疏逻辑量子比特编号创建线路。 |
| `Circuit([Qubit(5), Qubit(7)])` | 使用已有 `Qubit` 对象创建线路。 |

示例：

```python
from cqlib import Circuit, Qubit

a = Circuit(3)
b = Circuit([0, 2, 4])
c = Circuit([Qubit(10), Qubit(11)])
```

---

## 低层构造

```python
Circuit.from_operations(
    qubits: list[Qubit],
    operations: list[ValueOperation],
    classical_vars: list[ClassicalType] | None = None,
    classical_values: list[ClassicalType] | None = None,
) -> Circuit
```

`Circuit.from_operations()` 用于根据已有的量子比特列表和操作序列重建线路。该接口属于较底层的构造入口，通常用于反序列化、IR 导入、编译器 pass 输出恢复，或在测试中构造精确的底层线路对象。

```python
from cqlib import Circuit, Qubit
from cqlib.circuit import StandardGate, ValueOperation

op = ValueOperation.from_standard_gate(StandardGate.H(), [Qubit(0)])
circuit = Circuit.from_operations([Qubit(0)], [op])
```

---

## 常用电路属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `id` | `CircuitId` | 当前线路的唯一标识，用于经典句柄、测量值和控制流作用域管理。 |
| `num_qubits` | `int` | 线路中包含的量子比特数量。 |
| `width` | `int` | `num_qubits` 的别名。 |
| `qubits` | `list[Qubit]` | 按线路内部顺序保存的量子比特列表。 |
| `parameters` | `list[Parameter]` | 线路中记录的参数表达式列表。 |
| `symbols` | `list[str]` | 线路中所有自由符号名称。 |
| `global_phase` | `Parameter` | 线路整体全局相位。 |
| `classical_vars` | `list[ClassicalType]` | 已分配的可变经典变量类型列表。 |
| `classical_values` | `list[ClassicalType]` | 测量等操作产生的不可变经典值类型列表。 |
| `operations` | `list[ValueOperation]` | 按追加顺序返回的自包含操作列表。 |

`operations` 返回的是可以在 Python 层检查和重建的 `ValueOperation` 列表。其中，每个操作都包含指令类型、作用量子比特、参数和标签等信息。该属性常用于线路调试、结构分析、测试断言和编译 pass 前后的对比。

```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

print(circuit.num_qubits)
print(circuit.operations)
```

---

## 基础电路操作

| 方法 | 说明 |
| --- | --- |
| `add_qubits(qubits)` | 向当前线路追加新的量子比特。 |
| `append(operation)` | 向线路追加一个自包含的 `ValueOperation`。 |
| `operation(index)` | 返回指定位置的 `ValueOperation`。 |
| `__len__()` | 返回线路中的操作数量。 |
| `__getitem__(index)` | 支持使用正向或负向索引读取操作。 |
| `validate()` | 校验线路结构、经典句柄、作用域和控制流不变量。 |
| `depth(recurse=False)` | 计算线路深度。 |
| `set_global_phase(phase)` | 设置线路全局相位。 |


`depth()` 用于估算线路深度。在线性量子线路中，深度通常表示按尽早调度（ASAP）方式排列后，最长量子比特路径上的操作层数。普通门和普通指令通常贡献一层；`barrier` 会约束其覆盖量子比特上的重排；`CircuitGate` 默认可视为一个不透明操作。当线路中包含 `if`、`while`、`for`、`switch`、`break` 或 `continue` 等控制流结构时，深度计算需要额外处理分支和循环语义。


```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

assert len(circuit) == 2
first = circuit[0]
last = circuit[-1]
depth = circuit.depth()
```

---

## 标准门便捷方法

`Circuit` 提供了一组常用量子门的便捷使用方法：

### 1. 单量子比特固定门

| 方法 | 门 |
| --- | --- |
| `i(qubit)` | Identity |
| `h(qubit)` | Hadamard |
| `x(qubit)` | Pauli-X |
| `y(qubit)` | Pauli-Y |
| `z(qubit)` | Pauli-Z |
| `s(qubit)` | S |
| `sdg(qubit)` | S dagger |
| `t(qubit)` | T |
| `tdg(qubit)` | T dagger |
| `x2p(qubit)` | X half-pi positive |
| `x2m(qubit)` | X half-pi negative |
| `y2p(qubit)` | Y half-pi positive |
| `y2m(qubit)` | Y half-pi negative |

### 2. 参数化单量子比特门

| 方法 | 参数 | 说明 |
| --- | --- | --- |
| `rx(qubit, theta)` | `float | Parameter` | 绕 X 轴旋转。 |
| `ry(qubit, theta)` | `float | Parameter` | 绕 Y 轴旋转。 |
| `rz(qubit, theta)` | `float | Parameter` | 绕 Z 轴旋转。 |
| `phase(qubit, lambda_)` | `float | Parameter` | 相位门 `P(lambda)`。 |
| `u(qubit, theta, phi, lambda_)` | `float | Parameter` | 通用单量子比特门。 |
| `xy(qubit, theta)` | `float | Parameter` | XY 交互族。 |
| `xy2p(qubit, theta)` | `float | Parameter` | 正半角 XY 门。 |
| `xy2m(qubit, theta)` | `float | Parameter` | 负半角 XY 门。 |
| `rxy(qubit, theta, phi)` | `float | Parameter` | XY 平面任意轴旋转。 |

### 3. 多量子比特门

| 方法 | 说明 |
| --- | --- |
| `cx(control, target)` | CNOT 门。 |
| `cy(control, target)` | controlled-Y 门。 |
| `cz(control, target)` | controlled-Z 门。 |
| `swap(a, b)` | 交换两个量子比特的状态。 |
| `ccx(control1, control2, target)` | Toffoli 门。 |
| `rxx(a, b, theta)` | `exp(-i theta XX / 2)`。 |
| `ryy(a, b, theta)` | `exp(-i theta YY / 2)`。 |
| `rzz(a, b, theta)` | `exp(-i theta ZZ / 2)`。 |
| `rzx(a, b, theta)` | `exp(-i theta ZX / 2)`。 |
| `crx(control, target, theta)` | controlled-RX 门。 |
| `cry(control, target, theta)` | controlled-RY 门。 |
| `crz(control, target, theta)` | controlled-RZ 门。 |
| `fsim(a, b, theta, phi)` | fSim(theta, phi) 门。 |

示例：

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(3)
circuit.h(0)
circuit.cx(0, 1)
circuit.rzz(1, 2, theta)
circuit.crx(0, 2, 0.25)
```

---

## 新增操作接口

除便捷门方法外，`Circuit` 还提供了更通用的新增操作接口，用于添加显式构造的门对象、复合门或底层操作对象。

| 方法 | 说明 |
| --- | --- |
| `append_gate(gate, qubits, label=None)` | 追加 `StandardGate`。 |
| `append_mc_gate(gate, qubits, label=None)` | 追加 `MCGate`。 |
| `append_unitary_gate(gate, qubits, params=None)` | 追加 `UnitaryGate`。 |
| `append_circuit_gate(gate, qubits, params=None)` | 追加 `CircuitGate`。 |
| `append(operation)` | 追加底层 `ValueOperation`。 |

```python
from cqlib import Circuit, Parameter
from cqlib.circuit import StandardGate

theta = Parameter("theta")

circuit = Circuit(1)
circuit.append_gate(StandardGate.RX(theta), [0], label="first-rx")
```

---

## 非酉指令

量子线路中不仅可以包含普通量子门，也可能包含测量、重置、屏障和延迟等非普通酉门指令。

| 方法 | 说明 |
| --- | --- |
| `barrier(qubits)` | 插入 barrier，用于阻止编译器跨该边界重排相关量子比特上的操作。 |
| `reset(qubit)` | 复位量子比特。 |
| `delay(qubit, duration)` | 在指定量子比特上插入空闲等待时间。 |
| `measure(qubit)` | 测量单个量子比特，返回 `Measurement`。 |
| `measure_bits(qubits)` | 测量多个量子比特，返回多 bit 结果。 |
| `measure_into(qubit, target)` | 测量单个量子比特并写入已有经典变量。 |
| `measure_bits_into(qubits, target)` | 测量多个量子比特并写入已有经典变量。 |

---

## 参数绑定

```python
assign_parameters(bindings: dict[str, float] | None = None) -> Circuit
```

`assign_parameters()` 用于将线路中的符号参数绑定为具体数值，并返回绑定后的新线路。该方法不会修改原始线路，因此适用于将一条参数化线路作为模板，在不同参数值下重复生成具体线路。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(1)
circuit.rx(0, theta)

bound = circuit.assign_parameters({"theta": 3.141592653589793})
assert bound.symbols == []
```

需要注意的是，绑定值必须是有限数值。如果绑定值为 `nan`、`inf` 或其他非法数值，通常会触发 `ParameterError`。如果只绑定部分符号，未绑定符号会保留在返回的新线路中。

---

## 反演、分解与组合

| 方法 | 说明 |
| --- | --- |
| `inverse()` | 返回一条新线路：操作顺序反转，并对每个可逆操作取逆。 |
| `decompose()` | 返回一条新线路，递归展开由 `CircuitGate` 表示的复合门。 |
| `to_gate(name)` | 将当前线路封装为可复用的 `CircuitGate`。 |
| `compose(other, qubits=None)` | 将另一条线路追加到当前线路中，可指定量子比特映射。 |

```python
from cqlib import Circuit

bell = Circuit(2)
bell.h(0)
bell.cx(0, 1)

bell_gate = bell.to_gate("Bell")

larger = Circuit(4)
larger.append_circuit_gate(bell_gate, [0, 1])
larger.append_circuit_gate(bell_gate, [2, 3])
```

`compose(other, qubits=None)` 的行为如下：

- `qubits=None`：按 `other` 的逻辑量子比特编号将其合并到当前线路中，必要时向当前线路加入新的量子比特；
- `qubits=[...]`：将 `other.qubits` 按顺序映射到当前线路中的目标量子比特上。

需要区分的是，`compose()` 会把另一条线路的操作直接追加到当前线路中，而`to_gate()` 会保留子线路的模块边界。

---

## 矩阵转换

| 方法 | 说明 |
| --- | --- |
| `to_matrix(qubits_order=None)` | 返回数值矩阵，类型通常为 `numpy.ndarray[np.complex128]`。 |
| `to_symbolic_matrix(qubits_order=None)` | 返回 `SymbolicMatrix`，并保留符号参数。 |

```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

matrix = circuit.to_matrix()
matrix_reversed = circuit.to_matrix([1, 0])
```
---

## 结构化控制流入口

`Circuit` 提供了两类控制流构造方式：

- `if_()`、`if_else()`、`while_()`、`for_uint()`、`switch()`；
- 手动构造 `ClassicalControlOp` 后调用 `append_control()`。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

circuit = Circuit(2)
circuit.h(0)

circuit.if_(
    ClassicalExpr.bool_literal(True),
    lambda body: body.x(1),
)
```

---

## 完整示例：参数化子线路复用

下面的示例展示如何先构造一个参数化子线路，将其封装为 `CircuitGate`，再在更大的线路中重复使用，并分别绑定不同参数值。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

layer = Circuit(2)
layer.rx(0, theta)
layer.cx(0, 1)
layer.rz(1, theta / 2)

layer_gate = layer.to_gate("ParamLayer")

model = Circuit(4)
model.append_circuit_gate(layer_gate, [0, 1], params=[Parameter("a")])
model.append_circuit_gate(layer_gate, [2, 3], params=[Parameter("b")])

bound = model.assign_parameters({"a": 0.1, "b": 0.2})
```

该示例体现了 `Circuit` 的典型复用方式：先构造可参数化的线路模块，再通过 `to_gate()` 封装为复合门，最后在主线路中多次调用并分别绑定参数。
