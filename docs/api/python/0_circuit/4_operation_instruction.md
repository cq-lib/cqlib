# Operation / Instruction

- `cqlib.circuit.Instruction`
- `cqlib.circuit.ValueInstruction`
- `cqlib.circuit.ValueOperation`
- `cqlib.circuit.Directive`

```python
from cqlib.circuit import Instruction, ValueInstruction, ValueOperation, Directive
```

`Instruction`、`ValueInstruction` 和 `ValueOperation` 是 `cqlib.circuit` 中用于描述线路操作的基础表示。从语义上看，`Instruction` 描述“执行什么指令”，例如一个标准门、复合门、自定义酉门或非酉指令；`ValueOperation` 描述“这条指令在线路中的一次具体应用”，包括作用在哪些量子比特上、携带哪些参数，以及是否带有标签信息。

---

## 操作 IR 层次

Cqlib 将线路操作拆分为多个层次，以便同时支持普通量子门、非酉指令和结构化控制流。

| 类型 | 作用 |
| --- | --- |
| `Instruction` | 存储层指令，表示标准门、多控制门、自定义酉门、子线路门、`Directive`、`delay` 等具体指令类型。 |
| `ValueInstruction` | 构造层指令，可以包裹普通 `Instruction`，也可以包裹结构化控制流对象 `ClassicalControlOp`。 |
| `ValueOperation` | 自包含操作，组合了 `ValueInstruction`、作用量子比特、参数列表和可选标签。 |
| `Directive` | 非酉指令类型，用于表示 `barrier`、`measure`、`reset` 等特殊线路语义。 |

这种分层设计可以将“指令定义”和“指令在线路中的应用”区分开来。

---

## `Instruction`

`Instruction` 用于描述一条操作所对应的指令类型。它只关心“要执行什么”，不包含该指令具体作用在哪些量子比特上，也不表示它在线路中的位置。

### 1. 创建接口

常用的静态构造方法如下：

| 静态方法 | 说明 |
| --- | --- |
| `Instruction.from_standard_gate(gate)` | 从 `StandardGate` 创建标准门指令。 |
| `Instruction.from_mc_gate(gate)` | 从 `MCGate` 创建多控制门指令。 |
| `Instruction.from_unitary_gate(gate)` | 从 `UnitaryGate` 创建自定义酉门指令。 |
| `Instruction.from_circuit_gate(gate)` | 从 `CircuitGate` 创建子线路复合门指令。 |
| `Instruction.from_directive(directive)` | 从 `Directive` 创建非酉指令。 |
| `Instruction.delay()` | 创建 `delay` 指令。 |

示例：

```python
from cqlib.circuit import Instruction, Directive, StandardGate

h_inst = Instruction.from_standard_gate(StandardGate.H())
barrier_inst = Instruction.from_directive(Directive.barrier())
delay_inst = Instruction.delay()
```

### 2. 属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `name` | `str` | 指令的可读名称，例如 `"h"`、`"cx"`、`"measure"`。 |
| `instruction_type` | `str` | 指令类别，例如 `"standard"`、`"mcgate"`、`"unitary"`、`"circuit"`、`"directive"`、`"classical_data"`、`"classical_control"`、`"delay"`。 |
| `is_standard` | `bool` | 是否为标准门指令。 |
| `is_mcgate` | `bool` | 是否为多控制门指令。 |
| `is_unitary` | `bool` | 是否为用户自定义酉门指令。 |
| `is_circuit_gate` | `bool` | 是否为子线路复合门指令。 |
| `is_directive` | `bool` | 是否为非酉 `Directive` 指令。 |
| `is_classical_control` | `bool` | 是否为结构化控制流指令。 |
| `is_classical_data` | `bool` | 是否为经典数据相关指令。 |
| `is_delay` | `bool` | 是否为延迟指令。 |
| `standard_gate` | `StandardGate / None` | 当指令为标准门时，返回内部标准门对象。 |
| `directive` | `Directive / None` | 当指令为 `Directive` 时，返回内部 directive 对象。 |

---

## `ValueInstruction`

`ValueInstruction` 是构造层指令，用于统一表示普通指令和结构化控制流，其为 `ValueOperation` 提供统一入口，使线路操作既可以表示普通量子门和非酉指令，也可以表示 `if`、`while`、`for`、`switch` 等控制流结构。

### 1. 创建接口

| 静态方法 | 说明 |
| --- | --- |
| `ValueInstruction.from_instruction(instruction)` | 将普通 `Instruction` 包装为构造层指令。 |
| `ValueInstruction.from_classical_control(control)` | 将 `ClassicalControlOp` 包装为构造层控制流指令。 |

### 2. 属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `is_instruction` | `bool` | 是否包裹普通 `Instruction`。 |
| `is_classical_control` | `bool` | 是否包裹结构化控制流。 |
| `instruction` | `Instruction / None` | 普通指令内容。 |
| `classical_control` | `ClassicalControlOp / None` | 控制流内容。 |

---

## `ValueOperation`

`ValueOperation` 表示线路中的一次完整操作。它包含要执行的指令、该指令作用的量子比特、应用时参数和可选标签。

构造签名如下：

```python
ValueOperation(
    instruction: ValueInstruction,
    qubits: list[Qubit],
    params: list[float | Parameter] | None = None,
    label: str | None = None,
)
```

其中，`qubits` 表示本次操作作用的逻辑量子比特；`params` 表示应用时传入的数值或符号参数；`label` 是用户可选的元数据，用于调试、标注来源或记录编译信息。

### 1. 工厂方法

| 静态方法 | 说明 |
| --- | --- |
| `from_instruction(instruction, qubits, params=None, label=None)` | 从普通 `Instruction`、作用量子比特和显式参数创建操作。 |
| `from_standard_gate(gate, qubits, label=None)` | 从已包含参数信息的 `StandardGate` 创建操作。 |
| `from_mc_gate(gate, qubits, label=None)` | 从已包含基础门参数的 `MCGate` 创建操作。 |
| `from_classical_control(control)` | 从 `ClassicalControlOp` 创建控制流操作。 |

示例：

```python
from cqlib import Qubit
from cqlib.circuit import ValueOperation, StandardGate

op = ValueOperation.from_standard_gate(
    StandardGate.RX(0.25),
    [Qubit(0)],
    label="rx-layer-0",
)
```

### 2. 属性和方法

| 接口 | 类型 | 说明 |
| --- | --- | --- |
| `instruction` | `ValueInstruction` | 操作对应的构造层指令。 |
| `qubits` | `list[Qubit]` | 操作作用的量子比特列表。 |
| `params` | `list[float | Parameter]` | 本次操作携带的参数列表。 |
| `label` | `str | None` | 可选标签。 |
| `matrix()` | `np.ndarray` | 返回该操作对应的酉矩阵。 |


---

## `Directive`

`Directive` 用于表示非酉线路指令。

| 静态方法 | 说明 |
| --- | --- |
| `Directive.barrier()` | 创建屏障指令，用于限制编译器跨边界重排相关量子比特上的操作。 |
| `Directive.measure()` | 创建计算基测量指令。 |
| `Directive.reset()` | 创建复位指令，将量子比特进行复位。 |

| 方法 | 说明 |
| --- | --- |
| `name()` | 返回稳定的小写指令名称：`"barrier"`、`"measure"` 或 `"reset"`。 |
| `is_barrier()` | 判断是否为屏障指令。 |
| `is_measure()` | 判断是否为测量指令。 |
| `is_reset()` | 判断是否为复位指令。 |
| `inverse()` | 返回指令的逆；`barrier` 返回自身，`measure` 和 `reset` 返回 `None`。 |

示例：

```python
from cqlib.circuit import Directive

barrier = Directive.barrier()
assert barrier.is_barrier()
assert Directive.measure().inverse() is None
assert Directive.reset().inverse() is None
```

---

## `ValueOperation` 与 `Circuit`

当用户直接通过 `Circuit` 提供的便捷方法构造线路，例如 `h()`、`cx()`、`rz()` 等时，Cqlib 会在内部自动创建相应的 `Instruction` 和 `ValueOperation`。

当需要手动构造操作级 IR 时，可以显式创建 `ValueOperation`，再通过 `Circuit.from_operations()` 恢复为线路对象。

```python
from cqlib import Circuit, Qubit
from cqlib.circuit import ValueOperation, StandardGate

ops = [
    ValueOperation.from_standard_gate(StandardGate.H(), [Qubit(0)]),
    ValueOperation.from_standard_gate(StandardGate.CX(), [Qubit(0), Qubit(1)]),
]

circuit = Circuit.from_operations([Qubit(0), Qubit(1)], ops)
```

---

## 标签 `label`

`label` 是附加在操作实例上的可读元数据。它不会改变门的矩阵、量子语义或执行结果，主要用于调试、可视化、记录来源或标识编译阶段生成的操作。

```python
from cqlib import Qubit
from cqlib.circuit import ValueOperation, StandardGate

op = ValueOperation.from_standard_gate(
    StandardGate.RZ(0.1),
    [Qubit(0)],
    label="calibrated-rz",
)
```
