# 落盘与输出

`cqlib_core::visualization`

本页覆盖可视化的文件输出入口：线路图的 `render_figure_to_file`，量子态图与结果图的 `render_state_plot_to_file`、`render_result_plot_to_file`。三个入口共用同一套输出契约：扩展名决定写出格式，内容为 SVG 标记。

## 导入

```rust
use cqlib_core::visualization::{
    render_figure_to_file, render_result_plot_to_file, render_state_plot_to_file,
};
```

---

## `render_figure_to_file(circuit, output_path, options) -> Result<(), VisualizationError>`

把线路直接渲染到输出文件。内部先构建可视化 IR 并生成 SVG 标记，再写出文件；不需要先调用 `circuit_to_figure`。

参数：

- `circuit` (`&Circuit`)：待渲染的线路。
- `output_path` (`&str`)：目标文件路径。扩展名为 `.png` 时写出栅格图，其余扩展名写出 SVG 向量图。
- `options` (`&FigureDrawerOptions`)：图形绘制选项。`dpi` 决定 PNG 的栅格化缩放：默认 `160` 对应缩放 `1.0`，更大的取值按比例放大 PNG 尺寸。

返回：

- `Result<(), VisualizationError>`：成功时不返回数据。

异常情况：

- `VisualizationError::CircuitBuild`：`decompose_circuit_gates` 为 `true` 时展开线路门定义失败。
- `VisualizationError::UnknownQubit`：操作引用了不在线路量子比特列表中的量子比特。
- `VisualizationError::ParameterIndexOutOfBounds`：符号参数的索引超出线路参数表的长度。
- `VisualizationError::InvalidInput`：`dpi` 为 `0`，或缩放后的画布尺寸超出可表示范围。
- `VisualizationError::SvgRenderFailed`：SVG 解析或栅格化失败。
- `VisualizationError::Io`：文件写入失败。

示例：

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{FigureDrawerOptions, render_figure_to_file};

let mut circuit = Circuit::new(1);
circuit.h(Qubit::new(0)).unwrap();

render_figure_to_file(&circuit, "circuit.svg", &FigureDrawerOptions::default()).unwrap();
render_figure_to_file(&circuit, "circuit.png", &FigureDrawerOptions::default()).unwrap();
```

---

## `render_state_plot_to_file(svg, output_path) -> Result<(), VisualizationError>`

把量子态绘图入口产出的 SVG 标记写出为文件。

参数：

- `svg` (`&str`)：由量子态绘图入口返回的 SVG 标记。
- `output_path` (`&str`)：目标文件路径。扩展名为 `.png` 时写出栅格图，其余扩展名写出 SVG 向量图。

返回：

- `Result<(), VisualizationError>`：成功时不返回数据。

异常情况：

- `VisualizationError::InvalidInput`：PNG 缩放后的画布尺寸超出可表示范围。
- `VisualizationError::SvgRenderFailed`：SVG 解析或栅格化失败。
- `VisualizationError::Io`：文件写入失败。

示例：

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::{
    StatePlotOptions, plot_bloch_multivector, render_state_plot_to_file,
};
use num_complex::Complex64;

let state = Statevector::from_state(1, vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)])
    .unwrap();
let svg = plot_bloch_multivector(&state, &StatePlotOptions::default()).unwrap();
render_state_plot_to_file(&svg, "bloch.svg").unwrap();
```

---

## `render_result_plot_to_file(svg, output_path) -> Result<(), VisualizationError>`

把结果绘图入口产出的 SVG 标记写出为文件。

参数：

- `svg` (`&str`)：由 `plot_histogram` 或 `plot_distribution` 返回的 SVG 标记。
- `output_path` (`&str`)：目标文件路径。扩展名为 `.png` 时写出栅格图，其余扩展名写出 SVG 向量图。

返回：

- `Result<(), VisualizationError>`：成功时不返回数据。

异常情况：

- `VisualizationError::InvalidInput`：PNG 缩放后的画布尺寸超出可表示范围。
- `VisualizationError::SvgRenderFailed`：SVG 解析或栅格化失败。
- `VisualizationError::Io`：文件写入失败。

示例：

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{ExecutionResult, Outcome};
use cqlib_core::visualization::{
    ResultPlotOptions, plot_histogram, render_result_plot_to_file,
};
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
render_result_plot_to_file(&svg, "histogram.svg").unwrap();
```

---

## 输出格式

| 扩展名 | 写出内容 |
| --- | --- |
| `.png`（不区分大小写） | 栅格图。SVG 先按缩放系数确定位图尺寸，再编码为 PNG。 |
| 其他扩展名 | 原样写出 SVG 标记文本。 |

栅格化缩放：线路图按 `dpi / 160` 缩放，默认 `dpi` 取值下缩放为 `1.0`；量子态图与结果图固定按 `1.0` 写出。缩放只改变位图尺寸，SVG 的坐标系统保持不变。

---

## 相关页面

- 线路绘制选项见 [SVG 线路图](2_draw_figure.md)。
- 量子态绘图入口见 [量子态可视化](3_state_plots.md)。
- 结果绘图入口见 [测量结果可视化](4_result_plots.md)。
