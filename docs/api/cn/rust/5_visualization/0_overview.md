# 可视化

`cqlib_core::visualization`

`cqlib_core::visualization` 是 Cqlib 的可视化模块，把线路、量子态与测量结果渲染为可查看、可保存的图形产物。模块以 SVG 为统一输出形态：每个绘制入口先返回 SVG 标记字符串，再由输出入口写入 `.svg` 文件，或栅格化为 `.png` 文件。

## Overview

可视化回答三类问题：一条线路长什么样、一个量子态在布洛赫球与密度矩阵上如何分布、一次测量的计数如何分布。模块按这三类对象划分为 `circuit`、`state`、`result` 三个子模块，另有一个共享的错误类型 `VisualizationError`。

### 渲染管线

线路绘制分两步：先把线路转换为与后端无关的可视化 IR，再由具体后端绘制。

1. **IR 构建** — `build_visual_circuit` 按线路顺序把操作排入互不重叠的列，为每个操作解析样式、标签与参数文本，并对控制流块递归收集子线路。
2. **后端绘制** — 文本后端把 IR 绘制成 Unicode 框线字符图，图形后端把 IR 绘制成 SVG 标记。两个后端消费同一份 IR，因此同一条线路在两种后端下的门顺序、列划分与标签一致。
3. **输出** — SVG 字符串可直接嵌入使用，也可写入文件；扩展名为 `.png` 的路径会先栅格化。

量子态与测量结果绘制是单步的：`state` 与 `result` 的绘制入口内部完成数据校验、归一化与布局，直接返回 SVG 字符串。

### 两类线路后端

- **文本后端**（`circuit_to_text`）：产出可按宽度折行的等宽字符图，适合在终端与日志中查看。
- **图形后端**（`circuit_to_figure`）：产出 SVG 标记，支持按门覆盖样式、按列折叠，以及 PNG 栅格化。

### 显示位序与折行折叠

`reverse_bits` 控制显示位序：置为 `true` 时量子比特按相反顺序显示，量子态的基标签与密度矩阵行列也按相反顺序排列，便于与外部约定的比特顺序对齐。

文本后端用 `line_width` 控制折行，图形后端用 `fold` 控制折叠：宽度或列数超过上限时，图按段拆分显示，段与段之间以方向标记衔接。

### 量子态与测量结果的输入

量子态绘制接受 `StateVisualizationSource` 的实现者，`Statevector` 与 `DensityMatrix` 均已实现该 trait；纯态在绘制前先展开为密度矩阵。测量结果绘制直接接受 `ExecutionResult`，按结果记录的量子比特数把测量结果格式化为比特串标签。

---

## 常用入口

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{FigureDrawerOptions, circuit_to_figure};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let svg = circuit_to_figure(&circuit, &FigureDrawerOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## 核心概念与术语

| 术语 | 说明 |
| --- | --- |
| **可视化 IR** | 与绘制后端无关的线路中间表示 `VisualCircuit`，由 `build_visual_circuit` 构建，文本后端与图形后端都由它绘制。 |
| **文本后端** | 把可视化 IR 绘制为 Unicode 框线字符图的后端，入口为 `circuit_to_text`。 |
| **图形后端** | 把可视化 IR 绘制为 SVG 标记的后端，入口为 `circuit_to_figure`。 |
| **折行** | 文本图宽度超过 `line_width` 时把线路拆成多段显示，段间以 `«`、`»` 标记衔接。 |
| **折叠** | 图形图的列数超过 `fold` 时把线路拆成多行显示。 |
| **显示位序** | 由 `reverse_bits` 决定的量子比特显示顺序；置为 `true` 时按相反顺序显示。 |
| **数据集** | 结果图中的一个数值序列。结果绘制入口把一次测量的计数归约为单个数据集。 |
| **保留桶** | 结果图中把 `number_to_keep` 之外的条目聚合成标签为 `rest` 的条目。 |

---

## `cqlib_core::visualization` API 概览

### 线路绘制

| 名字 | 简介 |
| --- | --- |
| [`circuit_to_text`](1_draw_text.md) / [`TextDrawerOptions`](1_draw_text.md) | 线路的文本绘制入口与绘制选项。 |
| [`draw_text_from_visual`](1_draw_text.md) | 从已构建的可视化 IR 绘制文本图。 |
| [`circuit_to_figure`](2_draw_figure.md) / [`FigureDrawerOptions`](2_draw_figure.md) | 线路的 SVG 绘制入口与绘制选项。 |
| [`draw_figure_from_visual`](2_draw_figure.md) | 从已构建的可视化 IR 绘制 SVG。 |
| [`FigureDrawStyle`](2_draw_figure.md) / [`GateStyle`](2_draw_figure.md) | 图形样式预设与单门样式覆盖。 |
| [`ParameterFormatOptions`](2_draw_figure.md) / [`ParameterDisplayMode`](2_draw_figure.md) / [`ParameterFormatter`](2_draw_figure.md) | 门参数文本的格式化选项、显示模式与格式化器。 |

### 视觉 IR

| 名字 | 简介 |
| --- | --- |
| [`build_visual_circuit`](6_visual_ir.md) / [`VisualBuildOptions`](6_visual_ir.md) | 把线路构建为视觉 IR 的入口与构建选项。 |
| [`VisualCircuit`](6_visual_ir.md) / [`VisualOperation`](6_visual_ir.md) | 视觉层的线路与单条操作表示。 |
| [`VisualOpStyle`](6_visual_ir.md) / [`VisualChildren`](6_visual_ir.md) | 操作的样式归类与控制流子节点。 |
| [`VisualControlFlowKind`](6_visual_ir.md) / [`VisualCondition`](6_visual_ir.md) | 控制流形态与其条件表示。 |

### 量子态绘制

| 名字 | 简介 |
| --- | --- |
| [`plot_bloch_vector`](3_state_plots.md) | 绘制单个布洛赫向量。 |
| [`plot_bloch_multivector`](3_state_plots.md) | 每个量子比特绘制一个约化布洛赫向量。 |
| [`plot_state_city`](3_state_plots.md) | 绘制密度矩阵的实部与虚部。 |
| [`plot_state_paulivec`](3_state_plots.md) | 绘制泡利基期望值柱状图。 |
| [`StatePlotOptions`](3_state_plots.md) | 量子态绘制的标题、配色、不透明度与位序选项。 |
| [`StateVisualizationSource`](3_state_plots.md) | 量子态绘制的输入 trait，把核心态对象适配为密度矩阵数据。 |
| [`state_to_density_matrix`](3_state_plots.md) / [`local_bloch_vectors`](3_state_plots.md) | 密度矩阵归一化与约化布洛赫向量计算。 |

### 测量结果绘制

| 名字 | 简介 |
| --- | --- |
| [`plot_histogram`](4_result_plots.md) | 把测量计数绘制为直方图。 |
| [`plot_distribution`](4_result_plots.md) | 把测量计数归一化为概率后绘制。 |
| [`ResultPlotOptions`](4_result_plots.md) | 结果图的排序、配色、保留条目与布局选项。 |

### 文件输出

| 名字 | 简介 |
| --- | --- |
| [`render_figure_to_file`](5_render_to_file.md) | 直接把线路渲染到 `.svg` 或 `.png` 文件。 |
| [`render_state_plot_to_file`](5_render_to_file.md) | 把量子态图的 SVG 写出为文件。 |
| [`render_result_plot_to_file`](5_render_to_file.md) | 把结果图的 SVG 写出为文件。 |
| [`VisualizationError`](0_overview.md) | 可视化各阶段共用的错误类型。 |

---

## 快速示例

### 1. 线路文本图

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{TextDrawerOptions, circuit_to_text};

let mut circuit = Circuit::new(3);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
circuit.measure(Qubit::new(0)).unwrap();

let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
assert!(text.contains("Q0:"));
assert!(text.contains("H"));
```

### 2. 线路 SVG 图

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{FigureDrawerOptions, circuit_to_figure};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let svg = circuit_to_figure(
    &circuit,
    &FigureDrawerOptions {
        initial_state: true,
        ..FigureDrawerOptions::default()
    },
)
.unwrap();
assert!(svg.contains("<svg"));
```

### 3. 测量结果直方图

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{ExecutionResult, Outcome};
use cqlib_core::visualization::{ResultPlotOptions, plot_histogram};
use std::collections::HashMap;

let mut result = ExecutionResult::new(
    "task-visualization".to_string(),
    (0..2).map(|idx| Qubit::new(idx as u32)).collect::<Vec<_>>(),
    7,
    2,
    Some("simulator".to_string()),
    None,
);
let counts = [("00", 2usize), ("11", 5)]
    .into_iter()
    .map(|(bits, count)| (Outcome::from_bitstring(bits).unwrap(), count))
    .collect::<HashMap<_, _>>();
result.finish(counts, None);

let svg = plot_histogram(&result, &ResultPlotOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## 校验与错误处理

可视化的各个入口统一返回 `VisualizationError`：

| 错误 | 触发场景 |
| --- | --- |
| `VisualizationError::CircuitBuild` | 展开线路门定义时预处理失败。 |
| `VisualizationError::UnknownQubit` | 操作引用了不在线路量子比特列表中的量子比特。 |
| `VisualizationError::ParameterIndexOutOfBounds` | 符号参数的索引超出线路参数表的长度。 |
| `VisualizationError::InvalidInput` | 输入数据非法：状态缓冲长度与量子比特数不符、结果不含计数、概率分布求和为零、`sort = "hamming"` 缺少 `target_string`、图例长度与数据集数不符、`dpi` 为零等。 |
| `VisualizationError::SvgRenderFailed` | SVG 解析或栅格化失败。 |
| `VisualizationError::Io` | 输出文件写入失败。 |

各入口的具体触发条件见对应页面。
