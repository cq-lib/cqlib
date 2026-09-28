# CircuitCFG

`cqlib_core::circuit::CircuitCFG`

```rust
use cqlib_core::circuit::CircuitCFG;
use cqlib_core::circuit::cfg::{
    BasicBlock,
    ControlFlowRegion,
    FlowEdge,
    OperationMetadata,
    SwitchRegionCase,
    Terminator,
};
```

`CircuitCFG` 是 Rust core 中用于表示量子线路控制流图（Control Flow Graph, CFG）的分析视图。与 `Circuit` 面向用户和结构化线路构造不同，`CircuitCFG` 更偏向编译器内部使用，用于分析、重写和验证包含结构化控制流的线路。

---

## 核心概念

| 类型 | 说明 |
| --- | --- |
| `CircuitCFG` | 线路的控制流图视图，包含基本块、控制流边、入口块、量子比特、经典数据表、符号表、参数表和全局相位。 |
| `BasicBlock` | 基本块，保存一段顺序执行的 `Operation`，在合法 CFG 中必须恰好带有一个终结符。 |
| `Terminator` | 基本块末尾的控制转移描述，例如顺序跳转、分支、循环退出等。 |
| `FlowEdge` | 控制流图中的边类型，用于描述块与块之间的执行流关系。 |
| `ControlFlowRegion` | 结构化控制流区域元数据，用于记录某个分支或循环对应的结构化区域。 |
| `OperationMetadata` | 操作级元数据，记录操作作用的量子比特、参数和标签。 |
| `SwitchRegionCase` | `switch` 控制流区域中的 case 元数据，记录 case 值与对应入口块。 |

在这些类型中，只有 `CircuitCFG` 可以从 `cqlib_core::circuit` 模块根导入，其余类型位于 `cqlib_core::circuit::cfg` 子模块。符号表、参数表和全局相位由 CFG 内部保存，没有公开访问接口，但会在 `to_circuit()` 时一并还原。

---

## `BasicBlock`

`BasicBlock` 表示一段顺序执行的操作序列。

常用方法如下：

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建一个空基本块。 |
| `with_label(label)` | 消费自身并返回带标签的新基本块，需要链式调用或重新绑定。 |
| `push_operation(op)` | 向基本块末尾添加一条操作。 |
| `extend_operations(ops)` | 批量追加操作。 |
| `set_terminator(terminator)` | 设置基本块的终结符。 |
| `is_empty()` | 判断基本块是否既无操作也无终结符。 |
| `has_terminator()` | 判断基本块是否已经设置终结符。 |
| `len()` | 返回基本块中的操作数量。 |
| `label()` | 读取基本块标签。 |
| `terminator()` | 读取基本块终结符。 |
| `operations()` | 读取基本块中的操作切片。 |

此外，`BasicBlock` 实现了 `Default`，其行为等价于 `BasicBlock::new()`。

---

## `Terminator`

```rust
pub enum Terminator {
    Branch(ClassicalExpr),
    ForLoop {
        var: ClassicalVar,
        start: ClassicalExpr,
        stop: ClassicalExpr,
        step: ClassicalExpr,
    },
    Switch(ClassicalExpr),
    Jump(NodeIndex),
    Break(NodeIndex),
    Continue(NodeIndex),
    Return,
}
```

| 变体 | 说明 |
| --- | --- |
| `Branch(condition)` | 条件分支，条件必须是 `Bool` 表达式，出边类型分别为 `FlowEdge::TrueBranch` 与 `FlowEdge::FalseBranch`。 |
| `ForLoop { var, start, stop, step }` | 半开区间循环，四个表达式必须是相同位宽的 `UInt`。 |
| `Switch(target)` | 多分支选择，目标必须是 `UInt` 表达式，出边数量必须等于 case 数量加一。 |
| `Jump(target)` | 无条件跳转到目标基本块，出边类型为 `FlowEdge::Unconditional`。 |
| `Break(target)` | 跳出循环或 `switch`，出边类型为 `FlowEdge::Break`。 |
| `Continue(target)` | 进入下一轮循环，出边类型为 `FlowEdge::Continue`。 |
| `Return` | 结束当前线路分支，不允许有出边。 |

在合法 CFG 中，每个基本块都必须有且仅有一个终结符，且其出边数量与类型必须与终结符一致。由 `from_circuit()` 构造的线性线路，其入口块终结符为 `Terminator::Return`。

---

## `FlowEdge`

| 变体 | 说明 |
| --- | --- |
| `TrueBranch` | 条件为真时经过的边。 |
| `FalseBranch` | 条件为假时经过的边。 |
| `Unconditional` | 无条件跳转边。 |
| `Case(value)` | `switch` 中匹配到 `value` 时经过的边。 |
| `DefaultCase` | `switch` 默认分支的边；即使源 `switch` 没有 default 体也会存在。 |
| `Break` | `break` 跳出边。 |
| `Continue` | `continue` 继续边。 |

---

## 创建 `CircuitCFG`

`CircuitCFG` 提供以下创建接口：

```rust
pub fn new(num_qubits: usize) -> Self
pub fn from_qubits(qubits: Vec<Qubit>) -> Self
pub fn from_circuit(circuit: &Circuit) -> Result<Self, CircuitError>
```

| 接口 | 说明 |
| --- | --- |
| `CircuitCFG::new(num_qubits)` | 创建包含连续逻辑量子比特的空 CFG；此时尚未设置入口块，需要编辑后才能通过 `validate()`。 |
| `CircuitCFG::from_qubits(qubits)` | 根据指定逻辑量子比特集合创建空 CFG，适合稀疏逻辑编号；重复的量子比特会被去除，插入顺序会被保留。 |
| `CircuitCFG::from_circuit(circuit)` | 从已有结构化 `Circuit` 构造 CFG，是最常用的入口；构造结束前会在内部调用一次 `validate()`，无法结构化的控制流会直接返回错误。 |

`CircuitCFG` 还实现了 `TryFrom<&Circuit>`，其行为等同于 `from_circuit()`；反向的 `TryFrom<&CircuitCFG> for Circuit` 等同于 `to_circuit()`。两者的错误类型都是 `CircuitError`。

```rust
use cqlib_core::circuit::{Circuit, CircuitCFG, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;

let cfg = CircuitCFG::from_circuit(&circuit)?;

assert_eq!(cfg.num_qubits(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 图编辑接口

`CircuitCFG` 提供基本的图编辑能力，用于添加基本块、添加边、设置入口块和维护结构化控制流区域元数据。

| 方法 | 说明 |
| --- | --- |
| `add_block(block)` | 添加一个 `BasicBlock`，并返回对应的 `NodeIndex`。 |
| `add_edge(source, target, flow)` | 在 `source` 与 `target` 之间添加控制流边，成功时返回边索引；任一端点不存在时不修改图并返回 `None`。 |
| `entry_block()` | 读取当前 CFG 的入口基本块，未设置时返回 `None`。 |
| `set_entry_block(index)` | 设置 CFG 的入口基本块；该接口不校验索引是否有效，合法性由 `validate()` 判定。 |
| `set_control_flow_region(branch_block, region)` | 为某个分支或控制流入口块设置结构化区域元数据。 |
| `control_flow_region(branch_block)` | 读取某个基本块关联的结构化控制流区域元数据，没有记录时返回 `None`。 |
| `is_loop_header(block)` | 判断某个基本块是否为循环头；只有 `While`、`For` 区域的入口块返回 `true`，`Switch` 区域入口不算循环头。 |
| `block_mut(index)` | 可变读取指定基本块，索引不存在时返回 `None`。 |
| `outgoing_edges(source)` | 遍历指定基本块的出边，产出 `(NodeIndex, FlowEdge)` 二元组。 |

---

## 查询接口

`CircuitCFG` 提供以下查询接口，用于遍历图结构和读取线路基础信息。

| 方法 | 说明 |
| --- | --- |
| `blocks()` | 遍历 CFG 中的基本块，产出 `(NodeIndex, &BasicBlock)` 二元组。 |
| `num_blocks()` | 返回基本块数量。 |
| `num_qubits()` | 返回量子比特数量。 |
| `qubits()` | 按插入顺序返回逻辑量子比特列表，返回拥有所有权的 `Vec<Qubit>`。 |
| `classical_vars()` | 返回经典变量类型表。 |
| `classical_values()` | 返回经典值类型表。 |

---

## 验证与重建

`CircuitCFG` 提供两个关键接口用于检查和还原图结构：

```rust
pub fn validate(&self) -> Result<(), CircuitError>
pub fn to_circuit(&self) -> Result<Circuit, CircuitError>
```

### `validate()`

`validate()` 用于检查 CFG 结构是否自洽。典型检查内容包括：

- 是否存在有效入口块；
- 边引用的基本块是否存在；
- 每个基本块是否都有终结符，且终结符与出边数量、出边类型是否匹配，`Return` 是否没有出边；
- 基本块内是否残留未展开的 `Instruction::ClassicalControl`；
- 是否存在不可达或未被消费的基本块，即全部基本块是否都从入口块可达；
- 控制流区域元数据是否与图结构一致；
- 循环头、循环回边和退出边是否满足约束；
- `Branch` 的条件是否为 `Bool` 表达式；
- `Switch` 的目标是否为 `UInt` 表达式，出边数量是否等于 case 数量加一；
- 经典变量和值的作用域和依赖关系是否可被正确解释；
- `break` / `continue` 终结符是否各自恰好有一条同类型出边并指向目标块。

其中，`break` 与 `continue` 是否出现在合法的循环或 `switch` 区域内，由 `from_circuit()` 在构造期判定；`validate()` 只检查终结符与出边的对应关系。

### `to_circuit()`

`to_circuit()` 用于将 CFG 重新还原为结构化 `Circuit`。它要求 CFG 不仅是一个合法图，还必须能够对应回 Cqlib 支持的结构化控制流形式。

如果 CFG 的图结构已经被破坏，或某些控制流区域无法映射回结构化 `if`、`while`、`for`、`switch` 等结构，`to_circuit()` 会返回错误。该方法内部会先调用一次 `validate()`，因此上一节的约束在此处同样生效。

建议工作流如下：

```rust
use cqlib_core::circuit::{Circuit, CircuitCFG, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let cfg = CircuitCFG::from_circuit(&circuit)?;

// 在这里执行 CFG 分析或转换 pass

cfg.validate()?;
let new_circuit = cfg.to_circuit()?;

assert_eq!(new_circuit.num_qubits(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## `ControlFlowRegion` 元数据

`ControlFlowRegion` 用于记录结构化控制流区域的边界和语义。对于 `if`、`while`、`for`、`switch` 等结构，仅有 CFG 边并不足以完整恢复原始结构化语义，还需要区域元数据描述哪些基本块属于同一个控制流结构、入口和出口在哪里、switch case 如何对应等。

```rust
pub enum ControlFlowRegion {
    If {
        then_entry: NodeIndex,
        else_entry: NodeIndex,
        merge_block: NodeIndex,
        has_else: bool,
        outer: OperationMetadata,
    },
    While {
        body_entry: NodeIndex,
        exit_block: NodeIndex,
        outer: OperationMetadata,
    },
    For {
        body_entry: NodeIndex,
        exit_block: NodeIndex,
        outer: OperationMetadata,
    },
    Switch {
        cases: Vec<SwitchRegionCase>,
        default_entry: NodeIndex,
        merge_block: NodeIndex,
        has_default: bool,
        outer: OperationMetadata,
    },
}
```

| 变体 | 字段 | 说明 |
| --- | --- | --- |
| `If` | `then_entry`, `else_entry`, `merge_block`, `has_else`, `outer` | 条件分支区域的入口块、`else` 分支入口块、汇合块，以及源线路是否存在 `else` 分支。 |
| `While` | `body_entry`, `exit_block`, `outer` | `while` 循环体入口块与退出块。 |
| `For` | `body_entry`, `exit_block`, `outer` | `for` 循环体入口块与退出块。 |
| `Switch` | `cases`, `default_entry`, `merge_block`, `has_default`, `outer` | 各 case 的元数据、默认分支入口块、汇合块，以及源 `switch` 是否存在 default 分支。 |

其中 `outer` 是记录该控制流结构本身的操作元数据，用于在重建时还原被展开的控制流操作。相关类型如下：

```rust
pub struct OperationMetadata {
    pub qubits: SmallVec<[Qubit; 3]>,
    pub params: SmallVec<[CircuitParam; 1]>,
    pub label: Option<Box<str>>,
}

pub struct SwitchRegionCase {
    pub value: u128,
    pub entry: NodeIndex,
}
```

| 字段 | 说明 |
| --- | --- |
| `OperationMetadata::qubits` | 该控制流操作作用的量子比特。 |
| `OperationMetadata::params` | 该控制流操作的参数。 |
| `OperationMetadata::label` | 该控制流操作的标签。 |
| `SwitchRegionCase::value` | case 匹配的 `UInt` 取值。 |
| `SwitchRegionCase::entry` | 该 case 对应分支的入口块。 |

这些元数据没有独立的访问接口，pass 在修改图结构时需要自行维护；否则 `to_circuit()` 重建线路时可能丢失参数或标签。

因此，编写 CFG pass 时需要特别注意：

- 如果修改了分支结构，应同步更新对应的 `ControlFlowRegion`；
- 如果删除或合并了基本块，应检查区域元数据中是否仍引用旧 block；
- 如果调整循环边，应检查循环头和区域边界是否仍然正确；
- 如果修改 switch case，应同步维护 `SwitchRegionCase` 等元数据；
- 如果只是对基本块内部操作做局部优化，通常不需要修改区域元数据。

保持区域元数据与图边一致，是 `to_circuit()` 能否成功重建结构化线路的关键。

---

## 典型使用流程

### 1. 从结构化线路进入 CFG

```rust
use cqlib_core::circuit::{Circuit, CircuitCFG, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let cfg = CircuitCFG::from_circuit(&circuit)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### 2. 遍历基本块

```rust
use cqlib_core::circuit::{Circuit, CircuitCFG, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;

let cfg = CircuitCFG::from_circuit(&circuit)?;

for (node, block) in cfg.blocks() {
    for _op in block.operations() {
        // 分析每条 Operation
    }

    if block.has_terminator() {
        // 分析控制转移
    }
}

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### 3. 修改后验证并重建

```rust
use cqlib_core::circuit::cfg::{BasicBlock, Terminator};
use cqlib_core::circuit::{Circuit, CircuitCFG};

let mut cfg = CircuitCFG::new(1);

let mut block = BasicBlock::new().with_label("entry");
block.set_terminator(Terminator::Return);
let entry = cfg.add_block(block);
cfg.set_entry_block(entry);

cfg.validate()?;
let circuit = cfg.to_circuit()?;

assert_eq!(circuit.num_qubits(), 1);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 适用的编译 pass 类型

`CircuitCFG` 适合用于需要显式控制流结构的分析和转换任务，例如：

| pass 类型 | 说明 |
| --- | --- |
| 控制流可达性分析 | 检查不可达基本块、分支路径和循环结构。 |
| 分支内局部门优化 | 在每个基本块内部做门合并、门抵消或局部替换。 |
| 循环体资源估计 | 统计循环体内的门数量、深度或测量使用情况。 |
| 经典数据作用域检查 | 分析 `ClassicalValue` 是否越作用域使用。 |
| 动态线路合法性检查 | 检查控制流、测量和经典数据是否符合后端约束。 |
| 后端控制流降级 | 将结构化控制流转换为目标后端支持的形式。 |
| 分支展开或静态化 | 在条件可静态确定时，将控制流简化为线性操作。 |