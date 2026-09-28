# 可视化 IR

`cqlib_core::visualization`

本页覆盖线路绘制的中间表示层：构建入口 `build_visual_circuit`、构建选项 `VisualBuildOptions`，以及 IR 数据类型 `VisualCircuit`、`VisualOperation`、`VisualOpStyle`、`VisualChildren`、`VisualControlFlowKind`、`VisualCondition`。

## 导入

```rust
use cqlib_core::visualization::{
    VisualBuildOptions, VisualChildren, VisualCircuit, VisualCondition, VisualControlFlowKind,
    VisualOpStyle, VisualOperation, build_visual_circuit,
};
```

---

## 在管线中的位置

线路绘制分两步：先把线路构建为与绘制后端无关的可视化 IR，再由具体后端绘制。

```text
Circuit ──build_visual_circuit──▶ VisualCircuit ──┬── circuit_to_text / draw_text_from_visual ──▶ 文本图
                                                  └── circuit_to_figure / draw_figure_from_visual ──▶ SVG 图
```

构建阶段承担与线路语义相关的工作：把量子比特映射为下标、把参数格式化为标签文本、把指令归类为绘制样式，并把操作排入互不重叠的列。绘制阶段只消费这些结果，不再读回线路。控制流块在构建阶段被递归收集为子线路，由绘制后端展开为逐条标记。

中间层把构建与绘制解耦，带来三点结果：

- 文本后端与图形后端消费同一份 IR，因此同一条线路在两种后端下的门顺序、列划分与标签一致。
- 自定义绘制只需消费 `VisualCircuit`，无需重复实现量子比特映射、参数格式化与列调度。
- 构建选项可以独立于绘制选项调整。文本后端经 `circuit_to_text` 进入时使用默认的参数格式化选项，若要让文本图采用别的参数显示模式，先自行构建 IR 再交给 `draw_text_from_visual`。

---

## `build_visual_circuit(circuit, options) -> Result<VisualCircuit, VisualizationError>`

把线路构建为可视化 IR。

参数：

- `circuit` (`&Circuit`)：待构建的线路。
- `options` (`&VisualBuildOptions`)：构建选项。

返回：

- `Result<VisualCircuit, VisualizationError>`：成功时为可视化 IR，其中量子比特数等于线路的量子比特数。

异常情况：

- `VisualizationError::CircuitBuild`：`decompose_circuit_gates` 为 `true` 时展开线路门定义失败。
- `VisualizationError::UnknownQubit`：操作引用了不在线路量子比特列表中的量子比特。
- `VisualizationError::ParameterIndexOutOfBounds`：符号参数的索引超出线路参数表的长度。

示例：

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, build_visual_circuit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
assert_eq!(visual.num_qubits(), 2);
assert_eq!(visual.operations.len(), 2);
assert_eq!(visual.operations[0].label, "H");
assert_eq!(visual.operations[1].lanes, vec![0, 1]);
assert_eq!(visual.num_columns, 2);
```

---

## VisualBuildOptions

可视化 IR 的构建选项。这些选项只影响构建阶段，由文本后端与图形后端共享。

```rust
pub struct VisualBuildOptions {
    pub decompose_circuit_gates: bool,
    pub reserve_full_span_for_multi_qubit: bool,
    pub parameter_format: ParameterFormatOptions,
}
```

字段：

- `decompose_circuit_gates` (`bool`)：构建前是否展开线路门定义。默认 `false`。
- `reserve_full_span_for_multi_qubit` (`bool`)：为涉及多个量子比特的操作保留整段下标（`min..=max`）而非只保留操作自身的下标。默认 `true`。
- `parameter_format` (`ParameterFormatOptions`)：参数标签文本的格式化选项。默认 `ParameterFormatOptions::default()`，即 `ParameterDisplayMode::Numeric`。

默认值由 `VisualBuildOptions::default()` 提供，也可用结构体更新语法只覆盖需要改动的字段：

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, build_visual_circuit};

let mut sub = Circuit::new(2);
sub.h(Qubit::new(0)).unwrap();
sub.cx(Qubit::new(0), Qubit::new(1)).unwrap();
let sub_gate = sub.to_gate("SUB_BELL").unwrap();

let mut circuit = Circuit::new(2);
circuit
    .append(sub_gate, vec![Qubit::new(0), Qubit::new(1)], Vec::new(), None)
    .unwrap();

let visual = build_visual_circuit(
    &circuit,
    &VisualBuildOptions {
        decompose_circuit_gates: true,
        ..VisualBuildOptions::default()
    },
)
.unwrap();
assert_eq!(visual.operations[0].label, "H");
```

`parameter_format` 是嵌套结构，按字段更新：

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{
    ParameterDisplayMode, ParameterFormatOptions, VisualBuildOptions, build_visual_circuit,
};
use std::f64::consts::PI;

let mut circuit = Circuit::new(1);
circuit.rx(Qubit::new(0), PI / 2.0).unwrap();

let options = VisualBuildOptions {
    parameter_format: ParameterFormatOptions {
        mode: ParameterDisplayMode::PiFractionPreferred,
        ..ParameterFormatOptions::default()
    },
    ..VisualBuildOptions::default()
};
let visual = build_visual_circuit(&circuit, &options).unwrap();
assert_eq!(visual.operations[0].params, vec!["π/2".to_string()]);
```

---

## VisualCircuit

布局完成后的、与绘制后端无关的可视化 IR。

```rust
pub struct VisualCircuit {
    pub qubits: Vec<Qubit>,
    pub operations: Vec<VisualOperation>,
    pub num_columns: usize,
}
```

字段：

- `qubits` (`Vec<Qubit>`)：按显示顺序排列的量子比特，操作的 `lanes` 即该列表中的下标。
- `operations` (`Vec<VisualOperation>`)：按线路顺序排列、已排入列的操作。
- `num_columns` (`usize`)：被占用的列数。

方法：

- `fn num_qubits(&self) -> usize`：返回量子比特数。

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::visualization::{VisualCircuit, VisualOpStyle, VisualOperation};

let visual = VisualCircuit {
    qubits: vec![Qubit::new(0)],
    operations: vec![VisualOperation {
        column: 0,
        lanes: vec![0],
        covered_lanes: vec![0],
        label: "H".to_string(),
        params: vec![],
        style: VisualOpStyle::Gate,
        span_box: false,
        children: None,
        span_cols: 1,
    }],
    num_columns: 1,
};
assert_eq!(visual.num_qubits(), 1);
```

---

## VisualOperation

绘制后端消费的原子 IR 节点。一个操作对应一条列上的一段绘制内容。

```rust
pub struct VisualOperation {
    pub column: usize,
    pub lanes: Vec<usize>,
    pub covered_lanes: Vec<usize>,
    pub label: String,
    pub params: Vec<String>,
    pub style: VisualOpStyle,
    pub span_box: bool,
    pub children: Option<VisualChildren>,
    pub span_cols: usize,
}
```

字段：

- `column` (`usize`)：排入的列序号。
- `lanes` (`Vec<usize>)`：操作涉及的量子比特在 `VisualCircuit::qubits` 中的下标，按操作的操作数顺序排列。
- `covered_lanes` (`Vec<usize>`)：为避免重叠而在本列上保留的量子比特下标。控制流块保留全部下标；屏障在 `lanes` 为空时保留全部下标；其余操作在 `lanes` 为空时保留 `[0]`。涉及多于一个下标时，若 `reserve_full_span_for_multi_qubit` 为 `true`，则保留 `lanes` 的 `min..=max` 整段，否则保留 `lanes` 本身。量子比特数为零时为空。
- `label` (`String`)：主显示标签。
- `params` (`Vec<String>`)：已格式化的参数标签。
- `style` (`VisualOpStyle`)：绘制样式。
- `span_box` (`bool`)：为 `true` 时该操作按跨多条量子比特线的整体块绘制。自定义酉门、线路门与存储指令取 `true`。
- `children` (`Option<VisualChildren>`)：控制流操作的子线路，非控制流操作为 `None`。
- `span_cols` (`usize`)：该操作保留的逻辑列数。普通操作为 `1`；控制流块按子线路的列数加上分隔与边距计算。

```rust
use cqlib_core::visualization::{VisualOpStyle, VisualOperation};

let op = VisualOperation {
    column: 1,
    lanes: vec![0, 1],
    covered_lanes: vec![0, 1],
    label: "X".to_string(),
    params: vec!["π/2".to_string()],
    style: VisualOpStyle::Controlled { num_controls: 1 },
    span_box: false,
    children: None,
    span_cols: 1,
};
assert_eq!(op.label, "X");
```

---

## VisualOpStyle

单个操作的绘制样式。每个变体决定该操作由后端如何绘制。

```rust
pub enum VisualOpStyle {
    /// 普通门块。
    Gate,
    /// 受控操作，操作数开头的 `num_controls` 个量子比特为控制位。
    Controlled {
        num_controls: usize,
    },
    /// 受控 Z 标记，按两个以竖线相连的圆点绘制。
    Cz,
    /// 两个量子比特间的交换标记。
    Swap,
    /// 屏障标记。
    Barrier,
    /// 测量标记。
    Measure,
    /// 复位标记。
    Reset,
    /// 延时标记。
    Delay,
    /// 控制流标记。
    ControlFlow {
        kind: VisualControlFlowKind,
    },
}
```

### 样式与标签的归类规则

构建阶段按下表把指令归类为样式与标签：

| 指令 | `style` | `label` |
| --- | --- | --- |
| 标准门 `SWAP` | `Swap` | `SWAP` |
| 标准门 `CZ` | `Cz` | `CZ` |
| 标准门且控制位数大于 0 | `Controlled { num_controls }` | 目标门名 |
| 其余标准门 | `Gate` | 门名 |
| 多控门，控制位为 `1` 且基门为 `Z` | `Cz` | `CZ` |
| 其余多控门 | `Controlled { num_controls }` | 基门门名 |
| 自定义酉门 | `Gate`，`span_box` 为 `true` | `UnitaryGate::label()`，为空时 `Unitary` |
| 线路门 | `Gate`，`span_box` 为 `true` | `CircuitGate::name()`，为空时 `Gate` |
| 屏障 | `Barrier` | `B` |
| 测量 | `Measure` | `M` |
| 复位 | `Reset` | `R` |
| 延时 | `Delay` | `D` |
| 经典控制流 | `ControlFlow { kind }` | `IF`、`WH`、`FOR`、`SW`、`Break` 或 `Continue` |
| 测量到位指令 | `Measure` | `M` |
| 存储指令 | `Gate`，`span_box` 为 `true` | `STORE` |

门名按标准门口径缩写：`SDG` 显示为 `SD`，`TDG` 显示为 `TD`，`Phase` 与 `GPhase` 显示为 `P`。受控门的标签取其目标门名：`CX` 与 `CCX` 为 `X`，`CY` 为 `Y`，`CRX`、`CRY`、`CRZ` 分别为 `RX`、`RY`、`RZ`，其余情形去掉门名开头的控制前缀。

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, VisualOpStyle, build_visual_circuit};

let mut circuit = Circuit::new(3);
circuit.ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2)).unwrap();
circuit.swap(Qubit::new(0), Qubit::new(1)).unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
assert_eq!(visual.operations[0].label, "X");
assert!(matches!(
    visual.operations[0].style,
    VisualOpStyle::Controlled { num_controls: 2 }
));
assert!(matches!(visual.operations[1].style, VisualOpStyle::Swap));
```

---

## VisualChildren

附着在控制流操作上的子线路。子线路保留父级的量子比特列表，因此分支体与循环体只有触及部分量子比特时，绘制仍与外围线路对齐。

```rust
pub enum VisualChildren {
    IfElse {
        then_circuit: Box<VisualCircuit>,
        else_circuit: Option<Box<VisualCircuit>>,
    },
    While {
        body_circuit: Box<VisualCircuit>,
    },
    For {
        body_circuit: Box<VisualCircuit>,
    },
    Switch {
        case_circuits: Vec<(String, Box<VisualCircuit>)>,
        default_circuit: Option<Box<VisualCircuit>>,
    },
}
```

变体：

- `IfElse`：条件分支。`then_circuit` 为条件成立时执行的子线路，`else_circuit` 为条件不成立时执行的子线路，无假分支时为 `None`。
- `While`：`while` 循环体。`body_circuit` 为循环体内执行的子线路。
- `For`：`for` 循环体。`body_circuit` 为循环体内执行的子线路。
- `Switch`：多分支选择。`case_circuits` 按源码顺序给出各分支的标签与子线路，`default_circuit` 为默认分支的子线路，无默认分支时为 `None`。

```rust
use cqlib_core::circuit::{Circuit, ClassicalExpr, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, VisualChildren, build_visual_circuit};

let mut circuit = Circuit::new(2);
let condition = ClassicalExpr::bool_literal(true);
circuit
    .if_else(condition, |body| body.x(Qubit::new(1)), |body| body.z(Qubit::new(1)))
    .unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
let op = &visual.operations[0];
assert_eq!(op.label, "IF true");

match op.children.as_ref() {
    Some(VisualChildren::IfElse {
        then_circuit,
        else_circuit,
    }) => {
        assert_eq!(then_circuit.operations[0].label, "X");
        assert_eq!(else_circuit.as_ref().unwrap().operations[0].label, "Z");
    }
    _ => panic!("expected IfElse children"),
}
```

---

## VisualControlFlowKind

绘制后端使用的控制流标记族。

```rust
pub enum VisualControlFlowKind {
    IfElseBlock {
        has_false_branch: bool,
        condition: VisualCondition,
    },
    WhileBlock {
        condition: VisualCondition,
    },
    ForBlock {
        range: VisualCondition,
    },
    SwitchBlock {
        target: VisualCondition,
    },
    Break,
    Continue,
    IfStart,
    ElseStart,
    WhileStart,
    ForStart,
    SwitchStart,
    CaseStart,
    DefaultStart,
    End,
}
```

变体分两组。

源块变体，由构建阶段产生：

- `IfElseBlock`：条件分支块。`has_false_branch` 表示源块是否含假分支，`condition` 为分支条件的显示元数据。
- `WhileBlock`：`while` 循环块。`condition` 为循环条件的显示元数据。
- `ForBlock`：`for` 循环块。`range` 为循环区间表达式的显示元数据。
- `SwitchBlock`：多分支选择块。`target` 为分支目标表达式的显示元数据。
- `Break` 与 `Continue`：结构化 `break` 与 `continue` 标记。

展开后的标记变体，由绘制后端在绘制前展开源块时产生：

- `IfStart`、`ElseStart`：条件分支的起始与假分支起始标记。
- `WhileStart`、`ForStart`：循环起始标记。
- `SwitchStart`、`CaseStart`、`DefaultStart`：多分支选择的起始、分支与默认分支标记。
- `End`：块结束标记。

绘制前，文本后端与图形后端都会先把源块变体展开为逐条标记，再逐条绘制。标记的标签形如 `If-0 true`、`Else-0`、`End-0`，序号在同一张图内递增。因此展开后的标记变体不会出现在 `build_visual_circuit` 的返回值中，它描述的是后端绘制阶段的中间形态。

```rust
use cqlib_core::circuit::{Circuit, ClassicalExpr, Qubit};
use cqlib_core::visualization::{
    VisualBuildOptions, VisualChildren, VisualControlFlowKind, VisualOpStyle,
    build_visual_circuit,
};

let mut circuit = Circuit::new(2);
let condition = ClassicalExpr::bool_literal(false);
circuit
    .while_(condition, |body| {
        body.h(Qubit::new(0))?;
        body.cx(Qubit::new(0), Qubit::new(1))
    })
    .unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
let op = &visual.operations[0];
assert!(op.label.starts_with("WH false"));
assert!(matches!(
    op.style,
    VisualOpStyle::ControlFlow {
        kind: VisualControlFlowKind::WhileBlock { .. }
    }
));

match op.children.as_ref() {
    Some(VisualChildren::While { body_circuit }) => {
        assert_eq!(body_circuit.operations.len(), 2);
    }
    _ => panic!("expected While children"),
}
```

`for` 循环与多分支选择同样以子线路形式记录，子线路的操作序列与源码顺序一致：

```rust
use cqlib_core::circuit::{Circuit, ClassicalExpr, ClassicalType, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, VisualChildren, build_visual_circuit};

let mut circuit = Circuit::new(2);
let loop_var = circuit.var(ClassicalType::uint(3).unwrap());
circuit
    .for_uint(
        loop_var,
        ClassicalExpr::uint_literal(3, 0).unwrap(),
        ClassicalExpr::uint_literal(3, 4).unwrap(),
        ClassicalExpr::uint_literal(3, 1).unwrap(),
        |body, _| body.cx(Qubit::new(0), Qubit::new(1)),
    )
    .unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
assert!(visual.operations[0].label.starts_with("FOR"));
match visual.operations[0].children.as_ref() {
    Some(VisualChildren::For { body_circuit }) => {
        assert_eq!(body_circuit.operations[0].label, "X");
    }
    _ => panic!("expected For children"),
}
```

---

## VisualCondition

控制流可视化使用的条件显示元数据。

```rust
pub struct VisualCondition {
    pub label: String,
}
```

字段：

- `label` (`String`)：分支条件的显示标签。

标签文本由条件的经典表达式生成：布尔字面量显示为 `true` 或 `false`，位字面量显示为 `bit(v)`，无符号整数字面量显示其数值，位向量字面量显示为 `bits(v)`，其余表达式形态回退为其 `Debug` 文本。

```rust
use cqlib_core::visualization::VisualCondition;

let condition = VisualCondition {
    label: "true".to_string(),
};
assert_eq!(condition.label, "true");
```

---

## 典型用法

### 检查线路结构与列划分

共享同一量子比特的操作必须排在相邻列，落在不同量子比特上的操作可以共用一列：

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, build_visual_circuit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.x(Qubit::new(1)).unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
let cols: Vec<usize> = visual.operations.iter().map(|op| op.column).collect();
assert_eq!(cols, vec![0, 0]);
```

### 检查操作样式与占位

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, VisualOpStyle, build_visual_circuit};

let mut circuit = Circuit::new(3);
circuit.ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2)).unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
let op = &visual.operations[0];
assert_eq!(op.label, "X");
assert!(matches!(op.style, VisualOpStyle::Controlled { num_controls: 2 }));
assert_eq!(op.covered_lanes, vec![0, 1, 2]);
```

### 手工构造 IR 并渲染

手工构造的 `VisualCircuit` 可以直接交给绘制入口使用。以下 IR 描述一条只含单个 `H` 门的线路：

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::visualization::{
    TextDrawerOptions, VisualCircuit, VisualOpStyle, VisualOperation, draw_text_from_visual,
};

let visual = VisualCircuit {
    qubits: vec![Qubit::new(0)],
    operations: vec![VisualOperation {
        column: 0,
        lanes: vec![0],
        covered_lanes: vec![0],
        label: "H".to_string(),
        params: vec![],
        style: VisualOpStyle::Gate,
        span_box: false,
        children: None,
        span_cols: 1,
    }],
    num_columns: 1,
};

let text = draw_text_from_visual(&visual, &TextDrawerOptions::default()).unwrap();
assert!(text.contains("H"));
```

绘制结果为：

```text
          
 Q0: ───H─
          
```

### 以自定义构建选项渲染

构建选项只作用在构建阶段，因此同一份线路的绘制文本可以随构建选项改变：

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
use cqlib_core::visualization::{
    FigureDrawerOptions, ParameterDisplayMode, TextDrawerOptions, VisualBuildOptions,
    build_visual_circuit, draw_figure_from_visual, draw_text_from_visual,
};

let mut circuit = Circuit::new(1);
let theta = Parameter::symbol("theta");
circuit.rx(Qubit::new(0), theta + 1.0).unwrap();

let mut build_options = VisualBuildOptions::default();
build_options.parameter_format.mode = ParameterDisplayMode::SymbolicWithValue;
let visual = build_visual_circuit(&circuit, &build_options).unwrap();

let text = draw_text_from_visual(&visual, &TextDrawerOptions::default()).unwrap();
assert!(text.contains("1 + theta"));

let svg = draw_figure_from_visual(&visual, &FigureDrawerOptions::default(), None);
assert!(svg.contains("<svg"));
```

---

## 相关页面

- 文本后端的绘制入口与绘制选项见 [文本线路图](1_draw_text.md)。
- 图形后端的绘制入口与绘制选项见 [SVG 线路图](2_draw_figure.md)。
- 参数标签的格式化选项见 [SVG 线路图](2_draw_figure.md) 中的参数格式化一节。
- 本页用到的控制流构造见 [经典数据与控制流](../0_circuit/9_classical_control_flow.md)。
- 模块整体结构与渲染管线见 [可视化](0_overview.md)。
