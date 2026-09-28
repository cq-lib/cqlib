# 线路 DAG

`cqlib.circuit`

`CircuitDag` 是线路在操作依赖层面的算法视图：节点表示一条操作，边表示两条操作因共享同一份资源而存在的先后顺序。可共享的资源包括量子比特、可变经典变量、不可变经典值，以及用于承载没有具体数据资源之操作的全局序。该视图不改变线路本身的存储结构，主要用于依赖分析、分层调度和子图替换。

本页覆盖 `CircuitDag`、`DagWire`、`DagControlFlow` 与 `DagSwitchCase`。

---

## 导入

```python
from cqlib.circuit import CircuitDag, DagControlFlow, DagSwitchCase, DagWire
```

---

## CircuitDag

`CircuitDag` 是线路的操作级依赖图。图中的每一条操作对应一个操作节点，此外每条线还带有输入哨兵节点和输出哨兵节点。

### 构造线路 DAG

```python
CircuitDag.from_circuit(circuit: Circuit) -> CircuitDag
circuit.dag() -> CircuitDag
```

`CircuitDag.from_circuit()` 是静态方法，用于从一条已有线路构造依赖图。`Circuit.dag()` 是等价入口，直接在当前线路上构造依赖图。

构造过程结束后会自动校验 DAG 不变量。如果线路本身不满足约束，或者操作引用了线路之外的资源，构造会抛出 `CircuitError`。

示例：

```python
from cqlib import Circuit, CircuitDag

circuit = Circuit(2)
circuit.h(0)
circuit.x(1)
circuit.cx(0, 1)

dag = circuit.dag()
same = CircuitDag.from_circuit(circuit)
```

### 导出回线路

```python
dag.to_circuit() -> Circuit
```

`to_circuit()` 按 DAG 的确定性拓扑序重建线路。返回的线路会带上 DAG 中记录的量子比特、参数表、经典变量、经典值和全局相位。如果 DAG 中含环、引用了非法参数，或重建出的操作序列无法通过线路构造校验，则抛出 `CircuitError`。

```python
recovered = dag.to_circuit()
print([operation.name for operation in recovered.operations])
```

### 校验

```python
dag.validate() -> None
```

`validate()` 检查图与元数据的一致性，包括：图是否含环、每条线的输入输出哨兵是否合法、操作节点序号是否唯一、操作引用的量子比特与参数索引是否有效、经典变量与经典值是否属于当前线路、控制流载荷是否与所属操作匹配。

从外部 IR 导入或手动构造 DAG 之后，建议显式调用一次：

```python
dag.validate()
```

### 属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `num_qubits` | `int` | 量子比特数量。 |
| `num_ops` | `int` | 操作节点数量。 |
| `is_empty` | `bool` | 是否没有操作节点。 |
| `qubits` | `list[Qubit]` | 按插入顺序返回量子比特。 |
| `parameters` | `list[Parameter]` | 参数表中的参数表达式。 |
| `symbols` | `list[str]` | 线路中出现的自由符号名。 |
| `classical_vars` | `list[ClassicalType]` | 已分配的可变经典变量类型。 |
| `classical_values` | `list[ClassicalType]` | 不可变经典值类型，通常由测量产生。 |

```python
assert dag.num_qubits == 2
assert dag.num_ops == 3
assert len(dag) == 3
assert dag.is_empty is False
assert dag.qubits == circuit.qubits
```

### 其他行为

- `len(dag)` 返回操作节点数量，与 `num_ops` 一致。
- `repr` 形如 `CircuitDag(num_qubits=2, num_ops=3)`。
- 支持 `copy`、`deepcopy`，复制后的 DAG 与原 DAG 拓扑序一致。

---

## 节点标识

Python 侧用 `int` 表示一个节点。节点标识由以下入口取得：

| 取得方式 | 说明 |
| --- | --- |
| `op_nodes()` | 按线路源顺序返回全部操作节点。 |
| `topological_op_nodes()` | 按确定性拓扑序返回全部操作节点。 |
| `front_layer()`、`layers()`、`node_layers()` | 分层结果中的节点。 |
| `wire_in(wire)` / `wire_out(wire)` | 某条线的输入与输出哨兵节点。 |
| `apply_operation_front()` / `apply_operation_back()` | 追加操作后返回新节点标识。 |

节点不一定是操作节点。可以用 `is_operation()` 与 `node_kind()` 判断节点种类：

```python
node = dag.topological_op_nodes()[0]

assert dag.is_operation(node)
assert dag.node_kind(node) == "operation"
assert dag.node_kind(dag.wire_in(DagWire.qubit(0))) == "wire_in"
```

`node_kind()` 的取值为 `"wire_in"`、`"wire_out"` 或 `"operation"`；节点不存在时返回 `None`。

`operation(node)` 返回该节点对应的 `ValueOperation`，其中包含指令、作用量子比特、参数和标签；节点不是操作节点时返回 `None`。节点标识不属于当前 DAG 时，遍历类方法会抛出 `CircuitError`。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(1)
circuit.rx(0, theta)

dag = circuit.dag()
operation = dag.operation(dag.topological_op_nodes()[0])

assert operation.name == "RX"
assert operation.params[0] == theta
assert dag.parameters == [theta]
assert dag.symbols == ["theta"]
```

---

## DagWire

`DagWire` 表示 DAG 边所承载的资源，共有四种形态：

| 构造 | 说明 |
| --- | --- |
| `DagWire.qubit(qubit)` | 量子比特线，参数为逻辑量子比特编号 `int` 或 `Qubit` 对象。 |
| `DagWire.classical_var(var)` | 可变经典变量线。 |
| `DagWire.classical_value(value)` | 不可变经典值线，通常为测量结果。 |
| `DagWire.global_order()` | 全局序，用于承载没有具体数据资源的操作之间的稳定顺序。 |

以上均为静态方法。

### 属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `kind` | `str` | 线的种类，取值为 `"qubit"`、`"classical_var"`、`"classical_value"` 或 `"global_order"`。 |
| `qubit_value` | `Qubit \| None` | 量子比特线携带的量子比特，其余形态为 `None`。 |
| `classical_var_value` | `ClassicalVar \| None` | 经典变量线携带的变量，其余形态为 `None`。 |
| `classical_value_value` | `ClassicalValue \| None` | 经典值线携带的值，其余形态为 `None`。 |

### 其他行为

- 支持 `==` 与 `hash`，可作为字典键或集合元素。
- 支持 `copy`、`deepcopy`。
- `repr` 形如 `DagWire.qubit(0)`。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalType, DagWire

circuit = Circuit(1)
measured = circuit.measure(0)
flag = circuit.var(ClassicalType.bool())
circuit.store(flag, measured.expr().bit_to_bool())

dag = circuit.dag()

qubit_wire = DagWire.qubit(0)
value_wire = DagWire.classical_value(measured.value)
var_wire = DagWire.classical_var(flag)

assert qubit_wire.kind == "qubit"
assert qubit_wire.qubit_value == circuit.qubits[0]
assert value_wire.classical_value_value == measured.value
assert var_wire.classical_var_value == flag
assert DagWire.global_order().kind == "global_order"
```

---

## 线查询

| 方法 | 说明 |
| --- | --- |
| `wires()` | 返回当前 DAG 中已物化的全部线。 |
| `has_wire(wire)` | 判断某条线是否已在当前 DAG 中物化。 |
| `is_wire_idle(wire)` | 判断某条线上是否没有任何操作。 |
| `wire_in(wire)` | 返回该线的输入哨兵节点标识，不存在时为 `None`。 |
| `wire_out(wire)` | 返回该线的输出哨兵节点标识，不存在时为 `None`。 |
| `nodes_on_wire(wire)` | 按线上顺序返回该线上的操作节点。 |

线不是合法资源、或线上时间线断裂或分叉时，`is_wire_idle()` 与 `nodes_on_wire()` 抛出 `CircuitError`。

```python
nodes = dag.topological_op_nodes()
measure_node, store_node = nodes

assert dag.has_wire(value_wire)
assert dag.nodes_on_wire(value_wire) == [measure_node, store_node]
assert dag.wire_in(qubit_wire) in dag.predecessors(measure_node)
```

---

## 图遍历

前驱与后继的查询分三个层次：不限资源的全部邻居、限定在某一条线上的邻居、以及按量子或经典资源分类的邻居。

| 方法 | 说明 |
| --- | --- |
| `op_nodes()` | 按源顺序返回全部操作节点。 |
| `topological_op_nodes()` | 按拓扑序返回全部操作节点。 |
| `predecessors(node)` | 返回全部前驱节点，含哨兵节点。 |
| `successors(node)` | 返回全部后继节点，含哨兵节点。 |
| `predecessors_on_wire(node, wire)` | 返回仅通过指定线相连的操作前驱。 |
| `successors_on_wire(node, wire)` | 返回仅通过指定线相连的操作后继。 |
| `quantum_predecessors(node)` | 返回通过量子比特线相连的操作前驱。 |
| `quantum_successors(node)` | 返回通过量子比特线相连的操作后继。 |
| `classical_predecessors(node)` | 返回通过经典变量线或经典值线相连的操作前驱。 |
| `classical_successors(node)` | 返回通过经典变量线或经典值线相连的操作后继。 |

`predecessors()` 与 `successors()` 返回的邻居可能包含线的哨兵节点；其余方法只返回操作节点。带 `wire` 参数的方法在该线不属于当前 DAG 时抛出 `CircuitError`；节点标识不存在时同样抛出 `CircuitError`。

```python
assert dag.quantum_successors(measure_node) == []
assert dag.classical_predecessors(store_node) == [measure_node]
assert dag.predecessors_on_wire(store_node, value_wire) == [measure_node]
```

---

## 分层与深度

| 方法 | 说明 |
| --- | --- |
| `front_layer()` | 返回没有操作前驱的操作节点，即最早可执行的一层。 |
| `layers()` | 按尽早调度方式返回分层结果，每层是一个操作节点列表。 |
| `node_layers()` | 返回每个操作节点到所在层号的映射。 |
| `depth()` | 返回依赖深度，即层数。 |

`layers()` 与 `node_layers()` 使用同一套层号，`depth()` 等于最大层号加一。图中含环，或操作前驱的层号与拓扑序不一致时，抛出 `CircuitError`。

```python
circuit = Circuit(2)
circuit.h(0)
circuit.x(1)
circuit.cx(0, 1)
dag = circuit.dag()

assert len(dag.front_layer()) == 2
assert [len(layer) for layer in dag.layers()] == [2, 1]
assert dag.depth() == 2
```

`node_layers()` 返回的字典按拓扑序排列，键为节点标识，值为层号。

```python
assert list(dag.node_layers().keys()) == dag.topological_op_nodes()
```

---

## 控制流

当线路中包含结构化控制流时，控制流操作节点会额外携带一份递归的 DAG 载荷。控制流本身的构造接口与低层 IR 见 [Classical Data / Control Flow](9_classical_control_flow.md)。

| 方法 | 说明 |
| --- | --- |
| `control_flow(node)` | 返回该节点携带的 `DagControlFlow`，没有时为 `None`。 |
| `has_control_flow()` | 判断顶层操作中是否存在结构化控制流。 |
| `has_nested_control_flow()` | 判断控制流体内部是否还包含结构化控制流。 |
| `has_measurement()` | 判断顶层操作中是否直接或递归包含测量。 |

### DagControlFlow

`kind` 属性给出控制流形态，取值为 `"if"`、`"while"`、`"for"`、`"switch"`、`"break"` 或 `"continue"`。各形态暴露的属性如下，不适用于该形态的属性返回 `None` 或空列表。

| 属性 | 适用形态 | 类型 | 说明 |
| --- | --- | --- | --- |
| `kind` | 全部 | `str` | 控制流形态。 |
| `then_body` | `if` | `CircuitDag \| None` | 条件成立时执行的分支体。 |
| `else_body` | `if` | `CircuitDag \| None` | `else` 分支体；没有该分支时为 `None`。 |
| `body` | `while`、`for` | `CircuitDag \| None` | 循环体。 |
| `cases` | `switch` | `list[DagSwitchCase]` | 各精确整数匹配分支。 |
| `default_body` | `switch` | `CircuitDag \| None` | `default` 分支体；没有该分支时为 `None`。 |

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalType

circuit = Circuit(1)
flag = circuit.var(ClassicalType.bool())
circuit.if_else(
    flag.expr(),
    lambda body: body.x(0),
    lambda body: body.z(0),
)

dag = circuit.dag()
control = dag.control_flow(dag.topological_op_nodes()[0])

assert control.kind == "if"
assert [op.name for op in control.then_body.to_circuit().operations] == ["X"]
assert [op.name for op in control.else_body.to_circuit().operations] == ["Z"]
```

### DagSwitchCase

`DagSwitchCase` 表示 `switch` 中的一个精确整数匹配分支。

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `value` | `int` | 该分支匹配的整数值。 |
| `body` | `CircuitDag` | 命中该分支时执行的控制流体。 |

```python
circuit = Circuit(1)
target = circuit.var(ClassicalType.uint(8))

circuit.switch(
    target.expr(),
    lambda case: (
        case.value(1, lambda body: body.x(0)),
        case.value(2, lambda body: body.y(0)),
        case.default(lambda body: body.z(0)),
    ),
)

dag = circuit.dag()
control = dag.control_flow(dag.topological_op_nodes()[0])

assert control.kind == "switch"
assert [case.value for case in control.cases] == [1, 2]
assert [op.name for op in control.cases[0].body.to_circuit().operations] == ["X"]
assert [op.name for op in control.default_body.to_circuit().operations] == ["Z"]
```

控制流体本身也是 `CircuitDag`，可以继续调用 `to_circuit()`、`layers()`、`control_flow()` 等接口，用于递归分析嵌套结构。

---

## 操作计数与连续段

| 方法 | 说明 |
| --- | --- |
| `operation_count_by_name()` | 按指令名统计顶层操作数量。 |
| `operation_count_by_name_recursive()` | 按指令名统计顶层与各控制流体内部的操作数量。 |
| `collect_1q_runs()` | 在每条量子比特线上收集连续的单量子比特门段。 |
| `collect_2q_runs()` | 在每条量子比特线上收集连续的双量子比特门段。 |

计数结果返回字典，键为指令名，值为数量。连续段以一个列表的列表返回，其中每个元素是一段连续的门节点标识序列，顺序与线上顺序一致。非门操作、测量、复位、屏障和多量子比特操作会中断单量子比特门段；同一个双量子比特操作可能同时出现在其两个量子比特线的段中。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr, ClassicalType

circuit = Circuit(2)
circuit.h(0)
circuit.x(0)
circuit.cx(0, 1)
circuit.cz(0, 1)
counter = circuit.var(ClassicalType.uint(8))
circuit.for_uint(
    counter,
    ClassicalExpr.uint_literal(8, 0),
    ClassicalExpr.uint_literal(8, 2),
    ClassicalExpr.uint_literal(8, 1),
    lambda body, _: body.z(1),
)

dag = circuit.dag()
top_counts = dag.operation_count_by_name()
recursive_counts = dag.operation_count_by_name_recursive()
oneq_runs = dag.collect_1q_runs()
twoq_runs = dag.collect_2q_runs()

assert dag.has_control_flow()
assert dag.has_nested_control_flow() is False
assert top_counts["H"] == 1
assert top_counts["for"] == 1
assert "Z" not in top_counts
assert recursive_counts["Z"] == 1
assert any(len(run) == 2 for run in oneq_runs)
assert any(len(run) == 2 for run in twoq_runs)
```

---

## 修改图

| 方法 | 说明 |
| --- | --- |
| `add_parameter(parameter)` | 将参数插入 DAG 参数表，返回 `(index, is_new)`。 |
| `apply_operation_front(operation)` | 在 DAG 前端追加一条操作，返回新节点标识。 |
| `apply_operation_back(operation)` | 在 DAG 后端追加一条操作，返回新节点标识。 |
| `remove_op_node(node)` | 删除一个操作节点，返回被删除的 `ValueOperation`。 |
| `substitute_node(node, operation)` | 用一条 `ValueOperation` 替换指定操作节点。 |
| `substitute_node_with_dag(node, replacement)` | 用另一个 `CircuitDag` 替换指定操作节点。 |

追加操作时传入的是自包含的 `ValueOperation`。替换控制流节点时，替换内容必须只包含一条同类型的控制流操作；替换 DAG 读取或写入的资源必须落在被替换节点的资源范围内，否则抛出 `CircuitError`。

```python
from cqlib import Circuit
from cqlib.circuit import StandardGate, ValueOperation

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
dag = circuit.dag()
q0, q1 = dag.qubits

front = ValueOperation.from_standard_gate(StandardGate.X, [q1])
back = ValueOperation.from_standard_gate(StandardGate.Z, [q0])
front_node = dag.apply_operation_front(front)
back_node = dag.apply_operation_back(back)

assert dag.operation(front_node).name == "X"
assert dag.operation(back_node).name == "Z"
assert [op.name for op in dag.to_circuit().operations] == ["X", "H", "CX", "Z"]

nodes = dag.topological_op_nodes()
removed = dag.remove_op_node(nodes[1])
assert removed.name == "H"
assert [op.name for op in dag.to_circuit().operations] == ["X", "CX", "Z"]

nodes = dag.topological_op_nodes()
replacement = ValueOperation.from_standard_gate(StandardGate.Y, [q1])
dag.substitute_node(nodes[0], replacement)
assert [op.name for op in dag.to_circuit().operations] == ["Y", "CX", "Z"]
```

用子图整体替换一个节点：

```python
circuit = Circuit(1)
circuit.h(0)
dag = circuit.dag()

replacement_circuit = Circuit(1)
replacement_circuit.x(0)
replacement_circuit.z(0)
replacement = replacement_circuit.dag()

dag.substitute_node_with_dag(dag.topological_op_nodes()[0], replacement)

assert [op.name for op in dag.to_circuit().operations] == ["X", "Z"]
```

---

## 典型用途

编译 pass 与路由算法通常在 DAG 上完成依赖分析：用 `front_layer()` 与 `node_layers()` 决定哪些操作已经就绪，用 `predecessors()` 与 `successors()` 判断两条操作之间是否存在顺序约束，用 `collect_1q_runs()` 之类的连续段收集定位可以整体处理的门序列，再用 `substitute_node()` 系列接口把变换结果写回图中。

---

## 异常情况

- `CircuitError`：节点标识不属于当前 DAG、线不属于当前 DAG、图含环、线的时间线断裂或分叉、控制流载荷与所属操作不匹配、替换 DAG 的资源范围或控制流形态非法、参数索引越界、经典变量或经典值属于其他线路等。
