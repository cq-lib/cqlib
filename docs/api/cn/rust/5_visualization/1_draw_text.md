# 文本线路图

`cqlib_core::visualization::circuit::text`

本页覆盖线路的文本后端：绘制入口 `circuit_to_text`、基于可视化 IR 的入口 `draw_text_from_visual`，以及绘制选项 `TextDrawerOptions`。文本后端把线路渲染为 Unicode 框线字符图，同一列上的操作对齐显示。

## 导入

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{TextDrawerOptions, circuit_to_text};
```

---

## `circuit_to_text(circuit, options) -> Result<String, VisualizationError>`

把线路绘制为 Unicode 框线字符图。内部先构建可视化 IR，再交由文本后端绘制；控制流块会被展开为按列排布的块标记。

参数：

- `circuit` (`&Circuit`)：待绘制的线路。
- `options` (`&TextDrawerOptions`)：文本绘制选项。

返回：

- `Result<String, VisualizationError>`：成功时为 UTF-8 文本图字符串；IR 中量子比特数为零时返回 `"empty circuit"`。

异常情况：

- `VisualizationError::CircuitBuild`：`decompose_circuit_gates` 为 `true` 时展开线路门定义失败。
- `VisualizationError::UnknownQubit`：操作引用了不在线路量子比特列表中的量子比特。
- `VisualizationError::ParameterIndexOutOfBounds`：符号参数的索引超出线路参数表的长度。

示例：

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{TextDrawerOptions, circuit_to_text};

let mut circuit = Circuit::new(3);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
circuit.measure(Qubit::new(0)).unwrap();
circuit.measure(Qubit::new(1)).unwrap();
circuit.measure(Qubit::new(2)).unwrap();

let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
assert!(text.contains("Q0:"));
```

---

## TextDrawerOptions

文本绘制选项。

```rust
pub struct TextDrawerOptions {
    pub cell_width: usize,
    pub show_params: bool,
    pub decompose_circuit_gates: bool,
    pub line_width: isize,
    pub initial_state: bool,
    pub reverse_bits: bool,
}
```

字段：

- `cell_width` (`usize`)：保留字段，当前渲染宽度按内容动态计算。默认 `9`。
- `show_params` (`bool`)：是否在门标签后附加参数文本。默认 `true`。
- `decompose_circuit_gates` (`bool`)：绘制前是否展开线路门定义。默认 `false`。
- `line_width` (`isize`)：折行的最大宽度。取值大于 `10` 时按该值折行；取负值时禁用折行；取值在 `0` 到 `10` 之间时回退为默认宽度。默认 `80`。
- `initial_state` (`bool`)：是否在每个量子比特线开头显示 `|0>`。默认 `false`。
- `reverse_bits` (`bool`)：是否反转量子比特的显示顺序。默认 `false`。

默认值由 `TextDrawerOptions::default()` 提供，也可用结构体更新语法只覆盖需要改动的字段：

```rust
use cqlib_core::visualization::TextDrawerOptions;

let options = TextDrawerOptions {
    line_width: 12,
    ..TextDrawerOptions::default()
};
```

---

## `draw_text_from_visual(visual, options) -> Result<String, VisualizationError>`

从已构建的可视化 IR 绘制文本图。已有缓存 IR 时使用本入口，可避免重复构建。

参数：

- `visual` (`&VisualCircuit`)：由 `build_visual_circuit` 构建的可视化 IR。
- `options` (`&TextDrawerOptions`)：文本绘制选项。

返回：

- `Result<String, VisualizationError>`：成功时为 UTF-8 文本图字符串；IR 中量子比特数为零时返回 `"empty circuit"`。

异常情况：

- 当前实现只返回 `Ok`；保留 `Result` 返回类型是为了与 `circuit_to_text` 保持对称。

示例：

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{
    TextDrawerOptions, VisualBuildOptions, build_visual_circuit, draw_text_from_visual,
};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let options = TextDrawerOptions::default();
let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
let text = draw_text_from_visual(&visual, &options).unwrap();
assert!(text.contains("H"));
```

---

## 输出形态

### 基本线路

```rust
let mut circuit = Circuit::new(3);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
circuit.measure(Qubit::new(0)).unwrap();
circuit.measure(Qubit::new(1)).unwrap();
circuit.measure(Qubit::new(2)).unwrap();
```

在默认选项下的输出：

```text
                
 Q0: ───H──■──M─
           │
 Q1: ──────┼──M─
           │
 Q2: ──────X──M─
                
```

每条线对应一个量子比特，两个量子比特线之间是连接列：受控门用 `■` 标记控制位、用 `│` 与 `X` 连接目标位。测量、复位与延时等指令以操作名或符号出现在对应列上。

### 折行

`line_width` 小于图的自然宽度时，图被拆成多段：

```rust
let mut circuit = Circuit::new(1);
for _ in 0..10 {
    circuit.h(Qubit::new(0)).unwrap();
}
let options = TextDrawerOptions {
    line_width: 12,
    ..TextDrawerOptions::default()
};
let text = circuit_to_text(&circuit, &options).unwrap();
```

输出中每段之间以空行分隔，段末与段首分别以 `»` 与 `«` 标记接续方向：

```text
                   »
 Q0: ───H──H──H──H─»
                   »

«                   »
« Q0: ───H──H──H──H─»
«                   »

«
« Q0: ───H──H─
«
```

`line_width` 取负值时不折行，输出中不出现 `«` 与 `»`。

### 显示初始态与反转位序

`initial_state` 为 `true` 时，每条量子比特线的开头显示 `|0>`：

```text
             
 Q0: |0>───H─
             
```

`reverse_bits` 为 `true` 时按相反顺序显示量子比特：

```rust
let mut circuit = Circuit::new(2);
circuit.x(Qubit::new(0)).unwrap();
let options = TextDrawerOptions {
    reverse_bits: true,
    ..TextDrawerOptions::default()
};
let text = circuit_to_text(&circuit, &options).unwrap();
```

```text
          
 Q1: ─────
          
 Q0: ───X─
          
```

### 参数与线路门

`show_params` 为 `false` 时，门标签只显示门名，不附加参数文本：

```text
          
 Q0: ───RX─
          
```

`decompose_circuit_gates` 为 `false` 时，线路门按整体显示为带边框的块；置为 `true` 时先展开门定义，再按展开后的操作绘制。

---

## 相关页面

- 图形后端的绘制入口与选项见 [SVG 线路图](2_draw_figure.md)。
- 参数文本的格式化规则见 [SVG 线路图](2_draw_figure.md) 中的参数格式化一节。
