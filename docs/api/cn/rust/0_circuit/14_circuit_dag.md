# 线路 DAG

`cqlib_core::circuit`

```rust
use cqlib_core::circuit::{CircuitDag, DagControlFlow, DagNode, DagSwitchCase, DagWire};
```

`CircuitDag` 是线路在操作依赖层面的算法视图：节点表示一条操作，边表示两条操作因共享同一份资源而存在的先后顺序。可共享的资源包括量子比特、可变经典变量、不可变经典值，以及用于承载没有具体数据资源之操作的全局序。该视图不改变线路本身的存储结构，主要用于依赖分析、分层调度和子图替换。

本页覆盖 `CircuitDag`、`DagWire`、`DagNode`、`DagControlFlow` 与 `DagSwitchCase`。

---

## 核心概念

| 概念 | 说明 |
| --- | --- |
| `CircuitDag` | 线路的操作级依赖图，操作节点之外每条线还带输入与输出哨兵节点。 |
| `DagWire` | 边承载的资源，取值为量子比特、经典变量、经典值或全局序。 |
| `DagNode` | 节点种类，分为输入哨兵、输出哨兵与操作三类。 |
| `DagControlFlow` | 结构化控制流操作携带的递归 DAG 载荷。 |
| `DagSwitchCase` | `switch` 中的单个精确整数匹配分支。 |

---

## CircuitDag

### 构造

```rust
pub fn from_circuit(circuit: &Circuit) -> Result<Self, CircuitError>
```

`CircuitDag::from_circuit()` 从一条已有线路构造依赖图，同时复制线路的量子比特、参数表、符号名、经典变量与经典值类型表以及全局相位。构造结束后会自动调用 `validate()`。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;

assert_eq!(dag.num_qubits(), 2);
assert_eq!(dag.num_ops(), 2);
assert_eq!(dag.depth()?, 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

```rust
pub fn from_operations(
    qubits: impl IntoIterator<Item = Qubit>,
    operations: &[Operation],
) -> Result<Self, CircuitError>
```

`CircuitDag::from_operations()` 从一段自包含的操作切片构造依赖图，是该视图面向 pass 局部分析的窄入口。它不接受参数表、经典声明与全局相位：每条操作都必须恰好对给定的量子比特集合有效，并且不依赖任何线路级元数据。当操作流中包含索引参数、经典变量、经典值或控制流体时，应改用 `from_circuit()`；有意忽略这些资源的分析 pass 应先将其归一化，例如解析参数、或把非量子操作替换为依赖屏障。

`qubits` 中出现重复量子比特、操作引用了集合之外的量子比特、或操作切片并非自包含时返回错误。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let dag = CircuitDag::from_operations(circuit.qubits(), circuit.operations())?;

assert_eq!(dag.num_ops(), 2);
assert_eq!(dag.parameters().len(), 0);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### 导出回线路

```rust
pub fn to_circuit(&self) -> Result<Circuit, CircuitError>
```

`to_circuit()` 按 DAG 的确定性拓扑序重建线路，并恢复量子比特集合、经典类型表与全局相位。图中含环、引用了非法参数索引、或重建出的操作序列被 `Circuit::from_operations()` 拒绝时返回错误。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, ParameterValue, Qubit};

let mut circuit = Circuit::new(1);
circuit.rx(Qubit::new(0), ParameterValue::from("theta"))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let recovered = dag.to_circuit()?;

assert_eq!(recovered.operations().len(), 1);
assert!(dag.symbols().contains("theta"));

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### 校验

```rust
pub fn validate(&self) -> Result<(), CircuitError>
```

`validate()` 检查图与元数据的一致性，包括：图是否含环、每条线的输入输出哨兵是否合法、操作节点序号是否唯一且与顺序索引一致、操作引用的量子比特与参数索引是否有效、经典变量与经典值是否属于当前线路、控制流载荷是否与所属操作匹配、边的资源是否合法。

从外部导入、自动生成或手动修改过的 DAG，在进入后续阶段前应显式校验：

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;
circuit.measure(Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;
dag.validate()?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### 类型转换

DAG 与线路之间还提供 `TryFrom` 转换，语义与 `from_circuit()`、`to_circuit()` 一致。

| 实现 | 说明 |
| --- | --- |
| `impl TryFrom<&Circuit> for CircuitDag` | 等价于 `CircuitDag::from_circuit()`。 |
| `impl TryFrom<&CircuitDag> for Circuit` | 等价于 `CircuitDag::to_circuit()`。 |

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let dag = CircuitDag::try_from(&circuit)?;
let recovered = Circuit::try_from(&dag)?;
assert_eq!(recovered, circuit);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### 元数据与统计

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `num_qubits()` | `usize` | 量子比特数量。 |
| `num_ops()` | `usize` | 操作节点数量。 |
| `is_empty()` | `bool` | 是否没有操作节点。 |
| `qubits()` | `Vec<Qubit>` | 按插入顺序返回量子比特。 |
| `parameters()` | `&IndexSet<Parameter>` | 参数表。 |
| `symbols()` | `&IndexSet<String>` | 自由符号名集合。 |
| `classical_vars()` | `&[ClassicalType]` | 可变经典变量类型表。 |
| `classical_values()` | `&[ClassicalType]` | 不可变经典值类型表。 |
| `add_parameter(param)` | `(usize, bool)` | 将参数驻留进参数表，返回表内索引与是否新插入。 |

`add_parameter()` 的行为与 `Circuit::add_parameter()` 一致：参数不存在时插入，并登记其全部符号名。

```rust
use cqlib_core::circuit::{Circuit, ClassicalType, CircuitDag, Qubit};

let mut circuit = Circuit::new(1);
let _flag = circuit.var(ClassicalType::Bool);
let _value = circuit.measure(Qubit::new(0))?;

let dag = CircuitDag::from_circuit(&circuit)?;

assert_eq!(dag.classical_vars(), circuit.classical_vars());
assert_eq!(dag.classical_values(), circuit.classical_values());

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### 属性

- 实现 `Debug` 与 `Clone`。

---

## DagWire

```rust
pub enum DagWire {
    Qubit(Qubit),
    ClassicalVar(ClassicalVar),
    ClassicalValue(ClassicalValue),
    GlobalOrder,
}
```

`DagWire` 是边承载的资源。它同时也是线的身份：每条线对应图中一个输入哨兵节点和一个输出哨兵节点。

| 变体 | 说明 |
| --- | --- |
| `DagWire::Qubit(Qubit)` | 量子比特线。 |
| `DagWire::ClassicalVar(ClassicalVar)` | 可变经典变量线，由 `store` 写入。 |
| `DagWire::ClassicalValue(ClassicalValue)` | 不可变经典值线，通常由测量产生。 |
| `DagWire::GlobalOrder` | 全局序，用于承载没有具体数据资源的操作之间的稳定顺序。 |

`DagWire` 实现 `Debug`、`Clone`、`Copy`、`PartialEq`、`Eq` 与 `Hash`。

### 线查询

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `wires()` | `impl Iterator<Item = DagWire>` | 遍历当前 DAG 中已物化的全部线。 |
| `has_wire(wire)` | `bool` | 判断某条线是否已在当前 DAG 中物化。 |
| `is_wire_idle(wire)` | `Result<bool, CircuitError>` | 判断某条线上是否没有任何操作。 |
| `wire_in(wire)` | `Option<NodeIndex>` | 返回该线的输入哨兵节点。 |
| `wire_out(wire)` | `Option<NodeIndex>` | 返回该线的输出哨兵节点。 |
| `nodes_on_wire(wire)` | `Result<Vec<NodeIndex>, CircuitError>` | 按线上顺序返回该线上的操作节点。 |

线不是当前 DAG 的合法资源、或线上时间线断裂或分叉时，`is_wire_idle()` 与 `nodes_on_wire()` 返回错误。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, DagWire, Qubit};

let mut circuit = Circuit::new(1);
circuit.h(Qubit::new(0))?;
circuit.x(Qubit::new(0))?;
circuit.z(Qubit::new(0))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let nodes = dag.nodes_on_wire(DagWire::Qubit(Qubit::new(0)))?;

assert_eq!(nodes.len(), 3);
assert_eq!(dag.num_ops(), 3);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

量子比特线在构造时全部物化，而经典变量线与经典值线只在被操作引用时才物化。下面这条未写入的经典变量线既不在 `wires()` 中，也不参与任何依赖；`DagWire::GlobalOrder` 则始终存在，用于承载不涉及任何具体数据资源的操作。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, ClassicalType, DagWire, Qubit};

let mut circuit = Circuit::new(2);
let unused_var = circuit.var(ClassicalType::Bool);
circuit.h(Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;

assert!(dag.has_wire(DagWire::Qubit(Qubit::new(0))));
assert!(!dag.has_wire(DagWire::ClassicalVar(unused_var)));
assert!(dag.is_wire_idle(DagWire::Qubit(Qubit::new(0)))?);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## DagNode

```rust
pub enum DagNode {
    WireIn(DagWire),
    WireOut(DagWire),
    Operation {
        operation: Operation,
        order: usize,
    },
}
```

`DagNode` 是图中的节点载荷。每个操作节点保存存储层操作与一个源顺序位置，该位置用于确定性遍历。

| 变体 | 说明 |
| --- | --- |
| `DagNode::WireIn(DagWire)` | 某条线的输入哨兵。 |
| `DagNode::WireOut(DagWire)` | 某条线的输出哨兵。 |
| `DagNode::Operation { operation, order }` | 一条操作，`order` 为其源顺序位置。 |

节点种类可以用 `node_kind()` 查询，它返回 `"wire_in"`、`"wire_out"` 或 `"operation"`，节点不存在时返回 `None`。

---

## 节点标识

节点标识类型为 `NodeIndex`，由 `rustworkx_core::petgraph::prelude::NodeIndex` 提供，可按 `node.index()` 取回其 `usize` 值。节点标识由以下入口取得：

| 取得方式 | 说明 |
| --- | --- |
| `op_nodes()` | 按源顺序遍历全部操作节点。 |
| `topological_op_nodes()` | 按拓扑序返回全部操作节点。 |
| `front_layer()`、`layers()`、`node_layers()` | 分层结果中的节点。 |
| `wire_in()` / `wire_out()` | 某条线的输入与输出哨兵节点。 |
| `apply_operation_back()` 等变更方法 | 返回新节点标识。 |

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `operation(node)` | `Option<&Operation>` | 返回节点对应的存储层操作；节点不是操作节点时为 `None`。 |
| `is_operation(node)` | `bool` | 判断节点是否为操作节点。 |
| `node_kind(node)` | `Option<&'static str>` | 返回节点种类。 |
| `control_flow(node)` | `Option<&DagControlFlow>` | 返回该节点携带的控制流载荷，没有时为 `None`。 |
| `op_nodes()` | `impl Iterator<Item = NodeIndex>` | 按源顺序遍历操作节点。 |
| `topological_op_nodes()` | `Result<Vec<NodeIndex>, CircuitError>` | 按确定性拓扑序返回操作节点。 |

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, DagWire, Instruction, Qubit, StandardGate};

let mut circuit = Circuit::new(1);
circuit.h(Qubit::new(0))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let node = dag.topological_op_nodes()?[0];
let operation = dag.operation(node).expect("operation node");

assert!(matches!(
    operation.instruction,
    Instruction::Standard(StandardGate::H)
));
assert_eq!(operation.qubits[0], Qubit::new(0));

let wire_in = dag.wire_in(DagWire::Qubit(Qubit::new(0))).expect("qubit wire");
assert!(dag.operation(wire_in).is_none());

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 图遍历

前驱与后继的查询分三个层次：不限资源的全部邻居、限定在某一条线上的邻居、以及按量子或经典资源分类的邻居。

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `predecessors(node)` | `impl Iterator<Item = NodeIndex>` | 遍历全部前驱节点，可能包含哨兵节点。 |
| `successors(node)` | `impl Iterator<Item = NodeIndex>` | 遍历全部后继节点，可能包含哨兵节点。 |
| `predecessors_on_wire(node, wire)` | `Result<Vec<NodeIndex>, CircuitError>` | 仅通过指定线相连的操作前驱。 |
| `successors_on_wire(node, wire)` | `Result<Vec<NodeIndex>, CircuitError>` | 仅通过指定线相连的操作后继。 |
| `quantum_predecessors(node)` | `Result<Vec<NodeIndex>, CircuitError>` | 通过量子比特线相连的操作前驱。 |
| `quantum_successors(node)` | `Result<Vec<NodeIndex>, CircuitError>` | 通过量子比特线相连的操作后继。 |
| `classical_predecessors(node)` | `Result<Vec<NodeIndex>, CircuitError>` | 通过经典变量线或经典值线相连的操作前驱。 |
| `classical_successors(node)` | `Result<Vec<NodeIndex>, CircuitError>` | 通过经典变量线或经典值线相连的操作后继。 |

`predecessors()` 与 `successors()` 返回的邻居可能包含线的哨兵节点，需要过滤时可以用 `operation(node).is_some()` 判断。其余方法只返回操作节点，结果按拓扑序排列。节点标识不存在、或线不是当前 DAG 的合法资源时返回错误。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, DagWire, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.x(Qubit::new(1))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;
circuit.z(Qubit::new(0))?;
circuit.h(Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let nodes = dag.topological_op_nodes()?;
let (h0, x1, cx, z0, h1) = (nodes[0], nodes[1], nodes[2], nodes[3], nodes[4]);

assert_eq!(
    dag.predecessors_on_wire(cx, DagWire::Qubit(Qubit::new(0)))?,
    vec![h0]
);
assert_eq!(
    dag.successors_on_wire(cx, DagWire::Qubit(Qubit::new(1)))?,
    vec![h1]
);
assert_eq!(dag.quantum_predecessors(cx)?, vec![h0, x1]);
assert_eq!(dag.quantum_successors(cx)?, vec![z0, h1]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

经典资源上的依赖使用同一组接口：

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, ClassicalExpr, ClassicalType, Qubit};

let mut circuit = Circuit::new(1);
let measured = circuit.measure(Qubit::new(0))?;
let dest = circuit.var(ClassicalType::Bool);
circuit.store(dest, ClassicalExpr::bit_to_bool(measured.expr())?)?;

let dag = CircuitDag::from_circuit(&circuit)?;
let nodes = dag.topological_op_nodes()?;
let (measure, store) = (nodes[0], nodes[1]);

assert_eq!(dag.classical_predecessors(store)?, vec![measure]);
assert!(dag.quantum_predecessors(store)?.is_empty());

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 分层与深度

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `front_layer()` | `Result<Vec<NodeIndex>, CircuitError>` | 没有操作前驱的操作节点，即最早可执行的一层。 |
| `layers()` | `Result<Vec<Vec<NodeIndex>>, CircuitError>` | 按尽早调度方式分层的操作节点。 |
| `node_layers()` | `Result<IndexMap<NodeIndex, usize>, CircuitError>` | 每个操作节点到所在层号的映射。 |
| `depth()` | `Result<usize, CircuitError>` | 依赖深度，即层数。 |

`layers()` 与 `node_layers()` 使用同一套层号：某一层的层号为其全部操作前驱层号的最大值加一。`depth()` 等于最大层号加一，空图为 `0`。图中含环，或操作前驱的层号与拓扑序不一致时返回错误。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.x(Qubit::new(1))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let layers = dag.layers()?;

assert_eq!(layers.len(), 2);
assert_eq!(layers[0].len(), 2);
assert_eq!(layers[1].len(), 1);
assert_eq!(dag.front_layer()?.len(), 2);
assert_eq!(dag.depth()?, 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`node_layers()` 返回的映射与 `layers()` 一致，可用于按节点反查层号。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.x(Qubit::new(1))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let node_layers = dag.node_layers()?;

assert_eq!(node_layers.len(), dag.num_ops());
for (layer_index, layer) in dag.layers()?.iter().enumerate() {
    for node in layer {
        assert_eq!(node_layers.get(node).copied(), Some(layer_index));
    }
}

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 控制流与测量

当线路中包含结构化控制流时，控制流操作节点会额外携带一份递归的 DAG 载荷。控制流本身的构造接口与低层 IR 见 [Classical Data / Control Flow](9_classical_control_flow.md)。

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `has_control_flow()` | `bool` | 顶层操作中是否存在结构化控制流。 |
| `has_nested_control_flow()` | `bool` | 控制流体内部是否还包含结构化控制流。 |
| `has_measurement()` | `bool` | 顶层操作中是否直接或递归包含测量。 |
| `control_flow(node)` | `Option<&DagControlFlow>` | 返回该节点携带的控制流载荷。 |

`has_control_flow()` 与 `has_measurement()` 都只遍历顶层操作节点，区别在于 `has_measurement()` 会对每条操作内部递归判断，因此控制流体内的测量同样会被计入。`has_nested_control_flow()` 则深入控制流体，判断其中是否还有控制流。

### DagControlFlow

```rust
pub enum DagControlFlow {
    If {
        then_body: Box<CircuitDag>,
        else_body: Option<Box<CircuitDag>>,
    },
    While { body: Box<CircuitDag> },
    For { body: Box<CircuitDag> },
    Switch {
        cases: Vec<DagSwitchCase>,
        default: Option<Box<CircuitDag>>,
    },
    Break,
    Continue,
}
```

| 变体 | 说明 |
| --- | --- |
| `If { then_body, else_body }` | `if` 结构，`else_body` 在缺少该分支时为 `None`。 |
| `While { body }` | `while` 循环。 |
| `For { body }` | 基于 `UInt` 变量的半开区间循环。 |
| `Switch { cases, default }` | `switch` 多分支选择。 |
| `Break` | 跳出跳转，没有子图。 |
| `Continue` | 继续跳转，没有子图。 |

各分支体本身就是 `CircuitDag`，可以继续调用 `num_ops()`、`topological_op_nodes()`、`control_flow()` 等接口递归分析。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, ClassicalExpr, DagControlFlow, Qubit};

let mut circuit = Circuit::new(2);
let measured = circuit.measure(Qubit::new(0))?;
let condition = ClassicalExpr::bit_to_bool(measured.expr())?;
circuit.if_(condition, |body| {
    body.x(Qubit::new(1))?;
    Ok(())
})?;

let dag = CircuitDag::from_circuit(&circuit)?;
let control_node = dag.topological_op_nodes()?[1];

match dag.control_flow(control_node).expect("if payload") {
    DagControlFlow::If {
        then_body,
        else_body,
    } => {
        assert_eq!(then_body.num_ops(), 1);
        assert!(else_body.is_none());
    }
    _ => panic!("expected if control-flow payload"),
}

assert!(dag.has_control_flow());
assert!(!dag.has_nested_control_flow());

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

循环体的取法一致。`For` 变体只保存循环体，不保存循环变量、起止与步长，这些信息保留在所属操作的 `ClassicalControlOp::For` 中。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, ClassicalExpr, ClassicalType, DagControlFlow, Qubit};

let mut circuit = Circuit::new(1);
let counter = circuit.var(ClassicalType::uint(8).expect("valid width"));
circuit.for_uint(
    counter,
    ClassicalExpr::uint_literal(8, 0)?,
    ClassicalExpr::uint_literal(8, 3)?,
    ClassicalExpr::uint_literal(8, 1)?,
    |body, _| {
        body.h(Qubit::new(0))?;
        Ok(())
    },
)?;

let dag = CircuitDag::from_circuit(&circuit)?;
let control_node = dag
    .topological_op_nodes()?
    .into_iter()
    .find(|node| dag.control_flow(*node).is_some())
    .expect("control-flow node");

match dag.control_flow(control_node).expect("for payload") {
    DagControlFlow::For { body } => {
        assert_eq!(body.num_ops(), 1);
    }
    _ => panic!("expected for control-flow payload"),
}

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### DagSwitchCase

```rust
pub struct DagSwitchCase {
    pub value: u128,
    pub body: Box<CircuitDag>,
}
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `value` | `u128` | 该分支匹配的整数值。 |
| `body` | `Box<CircuitDag>` | 命中该分支时执行的控制流体。 |

```rust
use cqlib_core::circuit::{Circuit, ClassicalType, CircuitDag, DagControlFlow, Qubit};

let mut circuit = Circuit::new(1);
let state = circuit.var(ClassicalType::uint(2).expect("valid width"));
circuit.switch(state.expr(), |cases| {
    cases.value(0, |body| {
        body.x(Qubit::new(0))?;
        Ok(())
    })?;
    cases.value(1, |body| {
        body.z(Qubit::new(0))?;
        Ok(())
    })?;
    cases.default(|body| {
        body.h(Qubit::new(0))?;
        Ok(())
    })?;
    Ok(())
})?;

let dag = CircuitDag::from_circuit(&circuit)?;
let control_node = dag
    .topological_op_nodes()?
    .into_iter()
    .find(|node| dag.control_flow(*node).is_some())
    .expect("switch node");

match dag.control_flow(control_node).expect("switch payload") {
    DagControlFlow::Switch { cases, default } => {
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].value, 0);
        assert_eq!(cases[0].body.num_ops(), 1);
        assert!(default.is_some());
    }
    _ => panic!("expected switch control-flow payload"),
}

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 操作计数与连续段

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `operation_count_by_name()` | `IndexMap<String, usize>` | 按指令名统计顶层操作数量。 |
| `operation_count_by_name_recursive()` | `IndexMap<String, usize>` | 按指令名统计顶层与各控制流体内部的操作数量。 |
| `collect_runs_on_wire(wire, predicate)` | `Result<Vec<Vec<NodeIndex>>, CircuitError>` | 在指定线上收集由 `predicate` 接受的连续操作段。 |
| `collect_1q_runs()` | `Result<Vec<Vec<NodeIndex>>, CircuitError>` | 在每条量子比特线上收集连续的单量子比特门段。 |
| `collect_2q_runs()` | `Result<Vec<Vec<NodeIndex>>, CircuitError>` | 在每条量子比特线上收集连续的双量子比特门段。 |

计数结果保持插入顺序，键为指令名。连续段以源码顺序在线上收集：`predicate` 返回 `false` 的操作会打断当前段。

`collect_runs_on_wire()` 面向具体的量子比特线与经典线；`DagWire::GlobalOrder` 虽然也被接受，但它只对没有具体数据资源的操作排序，通常不适合作为优化段的划分依据。

`collect_1q_runs()` 与 `collect_2q_runs()` 的内部判据是：操作作用于一个或两个量子比特，且指令为标准门、多控制门、自定义酉门或复合门。非门操作、测量、复位、屏障和多量子比特操作会中断段。同一个双量子比特操作可能同时出现在其两个量子比特线的段中。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, DagWire, Instruction, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.x(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;
circuit.h(Qubit::new(0))?;
circuit.reset(Qubit::new(0))?;
circuit.z(Qubit::new(0))?;
circuit.x(Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let q0_runs = dag.collect_runs_on_wire(DagWire::Qubit(Qubit::new(0)), |operation| {
    matches!(operation.instruction, Instruction::Standard(_)) && operation.qubits.len() == 1
})?;

assert_eq!(
    q0_runs.iter().map(Vec::len).collect::<Vec<_>>(),
    vec![2, 1, 1]
);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 变更图

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `apply_operation_back(operation)` | `Result<NodeIndex, CircuitError>` | 在 DAG 后端追加一条存储层操作。 |
| `apply_value_operation_back(operation)` | `Result<NodeIndex, CircuitError>` | 先降级再在 DAG 后端追加一条构造层操作。 |
| `apply_operation_front(operation)` | `Result<NodeIndex, CircuitError>` | 在 DAG 前端追加一条存储层操作。 |
| `apply_value_operation_front(operation)` | `Result<NodeIndex, CircuitError>` | 先降级再在 DAG 前端追加一条构造层操作。 |
| `remove_op_node(node)` | `Result<Operation, CircuitError>` | 删除一个操作节点，返回被删除的操作。 |
| `substitute_node(node, operation)` | `Result<(), CircuitError>` | 用一条存储层操作替换指定节点。 |
| `substitute_value_node(node, operation)` | `Result<(), CircuitError>` | 先降级再用一条构造层操作替换指定节点。 |
| `substitute_node_with_dag(node, replacement)` | `Result<(), CircuitError>` | 用另一个 `CircuitDag` 替换指定节点。 |

`apply_value_operation_*` 与 `substitute_value_node()` 会把 `ValueOperation` 中的符号参数驻留进 DAG 参数表，因此调用后 `symbols()` 与 `parameters()` 也会随之更新。降级过程中若出现非有限数值参数或参数索引非法，则返回错误。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Parameter, ParameterValue, Qubit, StandardGate, ValueOperation};

let mut circuit = Circuit::new(1);
circuit.h(Qubit::new(0))?;
let mut dag = CircuitDag::from_circuit(&circuit)?;
let theta = Parameter::symbol("theta");

let rx = ValueOperation::from_standard(
    StandardGate::RX,
    [Qubit::new(0)],
    [ParameterValue::Param(theta.clone())],
);

let front = dag.apply_value_operation_front(rx)?;

assert!(dag.symbols().contains("theta"));
assert_eq!(dag.num_ops(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`substitute_node_with_dag()` 会先校验替换 DAG，再检查替换后读取或写入的资源是否落在被替换节点的资源范围内。替换控制流节点时，替换内容必须只包含一条同类型的控制流操作。任一校验不通过则返回错误。

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(1);
circuit.x(Qubit::new(0))?;
let mut dag = CircuitDag::from_circuit(&circuit)?;

let mut replacement = Circuit::new(1);
replacement.h(Qubit::new(0))?;
let replacement_dag = CircuitDag::from_circuit(&replacement)?;

let node = dag.topological_op_nodes()?[0];
dag.substitute_node_with_dag(node, replacement_dag)?;

assert_eq!(dag.num_ops(), 1);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 典型用途

编译 pass 与路由算法通常在 DAG 上完成依赖分析：用 `front_layer()` 与 `node_layers()` 决定哪些操作已经就绪，用 `predecessors()` 与 `successors()` 判断两条操作之间是否存在顺序约束，用 `collect_1q_runs()` 这类连续段收集接口定位可以整体处理的门序列，再用 `substitute_node()` 系列接口把变换结果写回图中。

`from_operations()` 适合 pass 内部的局部视图：当一段操作已经解析完参数、并且只涉及量子资源时，可以用它构造一个不携带线路级元数据的轻量依赖图。

---

## 错误处理

`CircuitDag` 的多数接口返回 `Result<_, CircuitError>`。常见错误来源如下：

| 错误 | 触发场景 |
| --- | --- |
| `CircuitError::InvalidDag` | 图含环、操作节点序号重复或与顺序索引不一致、线的哨兵非法、线的时间线断裂或分叉、节点标识不存在、控制流载荷与所属操作不匹配、替换 DAG 的资源范围或控制流形态非法。 |
| `CircuitError::QubitNotFound` | 操作引用了不在 DAG 量子比特集合中的量子比特。 |
| `CircuitError::DuplicateQubits` | `from_operations()` 的 `qubits` 中含有重复量子比特。 |
| `CircuitError::ForeignClassicalHandle` | 操作或线引用了属于其他线路的经典变量或经典值。 |
| `CircuitError::InvalidParameterIndex` | 操作引用了超出参数表范围的参数索引，或全局相位参数无法解析。 |
| `CircuitError::InvalidParameterValue` | 降级构造层操作时出现非有限数值参数。 |
| `CircuitError::InvalidOperation` | 构造层指令中嵌套了不合法的控制流表示。 |
| `CircuitError::DuplicateClassicalValueDefinition` | 同一个不可变经典值出现了多个产生者。 |
