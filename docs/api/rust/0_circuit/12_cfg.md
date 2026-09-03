# CircuitCFG

`cqlib_core::circuit::CircuitCFG`

```rust
use cqlib_core::circuit::CircuitCFG;
```

`CircuitCFG` 是 Rust core 中用于表示量子线路控制流图（Control Flow Graph, CFG）的分析视图。与 `Circuit` 面向用户和结构化线路构造不同，`CircuitCFG` 更偏向编译器内部使用，用于分析、重写和验证包含结构化控制流的线路。

---

## 核心概念

| 类型 | 说明 |
| --- | --- |
| `CircuitCFG` | 线路的控制流图视图，包含基本块、控制流边、入口块、量子比特和经典数据表等信息。 |
| `BasicBlock` | 基本块，保存一段顺序执行的 `Operation`，并可带有一个终结符。 |
| `Terminator` | 基本块末尾的控制转移描述，例如顺序跳转、分支、循环退出等。 |
| `FlowEdge` | 控制流图中的边类型，用于描述块与块之间的执行流关系。 |
| `ControlFlowRegion` | 结构化控制流区域元数据，用于记录某个分支或循环对应的结构化区域。 |
| `OperationMetadata` | 操作级元数据，可用于记录 pass 分析或转换过程中需要保留的信息。 |
| `SwitchRegionCase` | `switch` 控制流区域中的 case 元数据。 |

---

## `BasicBlock`

`BasicBlock` 表示一段顺序执行的操作序列。

常用方法如下：

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建一个空基本块。 |
| `with_label(label)` | 创建或设置带标签的基本块。 |
| `push_operation(op)` | 向基本块末尾添加一条操作。 |
| `extend_operations(ops)` | 批量追加操作。 |
| `set_terminator(terminator)` | 设置基本块的终结符。 |
| `is_empty()` | 判断基本块是否既无操作也无终结符。 |
| `has_terminator()` | 判断基本块是否已经设置终结符。 |
| `len()` | 返回基本块中的操作数量。 |
| `label()` | 读取基本块标签。 |
| `terminator()` | 读取基本块终结符。 |
| `operations()` | 读取基本块中的操作切片。 |

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
| `CircuitCFG::new(num_qubits)` | 创建包含连续逻辑量子比特的空 CFG。 |
| `CircuitCFG::from_qubits(qubits)` | 根据指定逻辑量子比特集合创建空 CFG，适合稀疏逻辑编号。 |
| `CircuitCFG::from_circuit(circuit)` | 从已有结构化 `Circuit` 构造 CFG，是最常用的入口。 |

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
| `add_edge(from, to, edge)` | 在两个基本块之间添加控制流边。 |
| `entry_block()` | 读取当前 CFG 的入口基本块。 |
| `set_entry_block(index)` | 设置 CFG 的入口基本块。 |
| `set_control_flow_region(branch_block, region)` | 为某个分支或控制流入口块设置结构化区域元数据。 |
| `control_flow_region(branch_block)` | 读取某个基本块关联的结构化控制流区域元数据。 |
| `is_loop_header(block)` | 判断某个基本块是否为循环头。 |
| `block_mut(index)` | 可变读取指定基本块。 |
| `outgoing_edges(index)` | 读取指定基本块的出边。 |

---

## 查询接口

`CircuitCFG` 提供以下查询接口，用于遍历图结构和读取线路基础信息。

| 方法 | 说明 |
| --- | --- |
| `blocks()` | 遍历 CFG 中的基本块。 |
| `num_blocks()` | 返回基本块数量。 |
| `num_qubits()` | 返回量子比特数量。 |
| `qubits()` | 返回 CFG 中的逻辑量子比特列表。 |
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
- 基本块终结符与出边数量、出边类型是否匹配；
- 控制流区域元数据是否与图结构一致；
- 循环头、循环回边和退出边是否满足约束；
- 经典变量和值的作用域和依赖关系是否可被正确解释；
- `break` / `continue` 等跳转是否处于合法区域内。

### `to_circuit()`

`to_circuit()` 用于将 CFG 重新还原为结构化 `Circuit`。它要求 CFG 不仅是一个合法图，还必须能够对应回 Cqlib 支持的结构化控制流形式。

如果 CFG 的图结构已经被破坏，或某些控制流区域无法映射回结构化 `if`、`while`、`for`、`switch` 等结构，`to_circuit()` 会返回错误。

建议工作流如下：

```rust
let mut cfg = CircuitCFG::from_circuit(&circuit)?;

// 在这里执行 CFG 分析或转换 pass

cfg.validate()?;
let new_circuit = cfg.to_circuit()?;
```

---

## `ControlFlowRegion` 元数据

`ControlFlowRegion` 用于记录结构化控制流区域的边界和语义。对于 `if`、`while`、`for`、`switch` 等结构，仅有 CFG 边并不足以完整恢复原始结构化语义，还需要区域元数据描述哪些基本块属于同一个控制流结构、入口和出口在哪里、switch case 如何对应等。

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
for block in cfg.blocks() {
    for op in block.operations() {
        // 分析每条 Operation
    }

    if block.has_terminator() {
        // 分析控制转移
    }
}
```

### 3. 修改后验证并重建

```rust
cfg.validate()?;
let circuit = cfg.to_circuit()?;
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