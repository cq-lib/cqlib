# 可视化（C）

可视化模块提供四组输出：`circuit_to_text` 输出框线字符的 UTF-8 文本线路图，`circuit_to_figure` 输出 SVG 标记字符串，`render_figure_to_file` 直接把线路渲染到 `.svg` / `.png` 文件；状态图（Bloch、多 Bloch、city、paulivec）与结果图（直方图、分布图）从模拟器态和执行结果生成 SVG，Visual IR 则把线路编译为后端无关的绘图中间表示。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 页面导航

| 页面 | 内容 |
| --- | --- |
| [文本线路图](1_draw_text.md) | `circuit_to_text` 与 `TextDrawerOptionsC` 选项表。 |
| [SVG 线路图](2_draw_figure.md) | `circuit_to_figure` 与 `FigureDrawerOptionsC` 选项表。 |
| [状态图](3_state_plots.md) | `plot_bloch_vector`、`plot_bloch_multivector`、`plot_state_city`、`plot_state_paulivec` 及 `CStatePlotOptionsC` 选项表。 |
| [结果图](4_result_plots.md) | `plot_histogram`、`plot_distribution` 及 `CResultPlotOptionsC` 选项表。 |
| [落盘与输出](5_render_to_file.md) | `render_figure_to_file`、`render_state_plot_to_file`、`render_result_plot_to_file`：按扩展名渲染为 `.svg` 或 `.png` 文件。 |
| [Visual IR](6_visual_ir.md) | `build_visual_circuit`、`CVisualCircuit` 句柄族与 `draw_text_from_visual`、`draw_figure_from_visual`。 |

---

## 共同约定

- 各绘制接口的 `options` 均可传 NULL，此时使用库默认选项；显式配置时按值填充对应的选项结构体（`TextDrawerOptionsC` / `FigureDrawerOptionsC` / `CStatePlotOptionsC` / `CResultPlotOptionsC` / `CVisualBuildOptionsC`），结构体内存归调用方管理。
- 返回 `char*` 的接口（`circuit_to_text`、`circuit_to_figure`、`plot_*`、`draw_*_from_visual`）用 `cqlib_string_free` 释放。
- `CVisualCircuit*` 句柄用 `visual_circuit_free` 释放（允许 NULL）；其访问器族见 [Visual IR](6_visual_ir.md)。
- `render_figure_to_file` 与两个 plot 落盘接口返回 `int32_t` 错误码：0 成功；-5 文件读写或渲染失败；-1 空指针（plot 落盘接口另有 -4：字符串不是有效的 UTF-8）。
