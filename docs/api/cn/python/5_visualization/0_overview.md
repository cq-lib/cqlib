# 可视化

`cqlib.visualization`

`cqlib.visualization` 负责把量子线路、量子态和测量结果渲染为可视化产物。它以 SVG 为首要输出格式：每个渲染入口都在内存中生成 SVG 文本并作为返回值，并可在给出输出路径时把同一份 SVG 落盘；对需要在终端或纯文本环境中查看的场景，另提供文本线路图。

`cqlib.visualization` 主要覆盖以下能力：

- **线路渲染**：把线路绘制成 SVG 线路图或文本线路图，用于核对门序、比特连接、参数取值与折叠效果。
- **量子态渲染**：以单比特 Bloch 向量、每比特约化 Bloch 向量、态矩阵和 Pauli 期望值等形式呈现量子态信息。
- **测量结果渲染**：把执行结果的计数渲染成计数直方图或归一化概率分布图。
- **统一输出契约**：全部渲染入口共享 SVG 文本返回值与输出路径落盘约定。

## 导入

```python
from cqlib.visualization import (
    draw_text,
    draw_figure,
    plot_histogram,
    plot_distribution,
    plot_bloch_vector,
    plot_bloch_multivector,
    plot_state_city,
    plot_state_paulivec,
)
```

---

## Overview

渲染入口按输入对象分为三组，各组的输入类型与产物如下：

| 输入 | 渲染入口 | 产物 |
| --- | --- | --- |
| 线路（`Circuit`） | `draw_text`、`draw_figure` | 文本线路图、SVG 线路图 |
| 量子态（`Statevector` 或 `DensityMatrix`） | `plot_bloch_vector`、`plot_bloch_multivector`、`plot_state_city`、`plot_state_paulivec` | SVG 状态图 |
| 执行结果（`ExecutionResult`） | `plot_histogram`、`plot_distribution` | SVG 计数直方图、SVG 概率分布图 |

### 统一的输出契约

所有渲染入口遵循同一套输出流程：

1. 调用渲染入口得到返回值，即渲染产物的文本内容。除 `draw_text` 返回文本线路图外，其余入口返回 SVG 文本，类型是 `str` 的子类 `_InlineSvg`。
2. 在 notebook 前端中，把返回值用作单元格的最后一个表达式即可内联显示；`_InlineSvg` 通过 SVG 富显示协议完成这一步。详见 [渲染输出与落盘](5_render_to_file.md)。
3. 传入 `output_path` 时，同一份 SVG 同时写入该路径。扩展名为 `.png` 时按同一份 SVG 光栅化为 PNG，其余扩展名直接写入 SVG 文本。

### 选项的分组

三组入口的选项集各自独立，签名中位于 `*` 之后的参数全部为关键字参数：

- **线路选项**：`initial_state`、`reverse_bits`、`show_params`、`decompose_circuit_gates` 为两个线路入口共有；`fold` 仅 `draw_figure` 提供，`line_width` 仅 `draw_text` 提供。
- **状态选项**：`title`、`color`、`alpha`、`reverse_bits`、`figsize`，四个状态渲染入口签名一致。
- **结果选项**：`figsize`、`color`、`number_to_keep`、`sort`、`target_string`、`legend`、`bar_labels`、`title`，两个结果渲染入口签名一致。

同名选项在三组之间含义一致：`reverse_bits` 控制是否反转计算基比特序的显示，`figsize` 控制绘图尺寸，`title` 控制标题文本，`output_path` 控制是否额外落盘。

---

## 常用入口

```python
from cqlib import Circuit
from cqlib.qis.state import Statevector
from cqlib.visualization import draw_text, plot_bloch_multivector

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

print(draw_text(circuit))

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

svg = plot_bloch_multivector(state)
assert svg.startswith("<svg")
```

---

## 核心概念与术语

| 术语 | 说明 |
| --- | --- |
| **SVG 线路图** | 由 `draw_figure` 渲染的矢量线路图，用图形符号表示量子门、连接线与测量等操作，可缩放后直接嵌入文档或网页。 |
| **文本线路图** | 由 `draw_text` 渲染的纯文本图形，用框线字符绘制量子比特线与门框，适合终端与日志场景。 |
| **内联显示** | 渲染返回值在 notebook 前端中直接作为富输出渲染的行为，由 `_InlineSvg` 的 SVG 富显示协议提供。 |
| **绘图尺寸** | 由 `figsize` 给出的 `(宽, 高)` 尺寸，单位为英寸，渲染时换算为像素；不传时使用该绘图族的默认尺寸。 |
| **折叠** | 由 `fold` 控制的绘制切分：把较长的线路按列切成多行绘制，每行不超过给定列数。 |
| **计数直方图** | 以测量结果中各比特串的出现次数为柱高的图，由 `plot_histogram` 渲染。 |
| **概率分布图** | 把测量结果计数归一化为概率后渲染的图，由 `plot_distribution` 渲染。 |
| **状态图** | 面向量子态的图，包含单个 Bloch 向量、每比特约化 Bloch 向量、态矩阵与 Pauli 期望值四类。 |
| **Pauli 期望值** | 以密度矩阵为输入计算出的 Pauli 基系数，正负值分别用不同颜色呈现。 |
| **Hamming 距离** | 两个等长比特串之间对应位取值不同的位数，用于 `plot_histogram` 与 `plot_distribution` 的 `hamming` 排序策略。 |

---

## `cqlib.visualization` API 概览

下表按功能分组列出本模块的页面（本页为模块概览）。

### 线路可视化

| 名字 | 简介 |
| --- | --- |
| [`draw_text`](1_draw_text.md) | 渲染文本线路图，支持折行宽度、初始态标注与比特序控制。 |
| [`draw_figure`](2_draw_figure.md) | 渲染 SVG 线路图，支持折叠、初始态标注与参数显示。 |

### 量子态可视化

| 名字 | 简介 |
| --- | --- |
| [`plot_bloch_vector`](3_state_plots.md) | 渲染单个 Bloch 向量。 |
| [`plot_bloch_multivector`](3_state_plots.md) | 每个量子比特渲染一个约化 Bloch 向量。 |
| [`plot_state_city`](3_state_plots.md) | 渲染密度矩阵的实部与虚部面板。 |
| [`plot_state_paulivec`](3_state_plots.md) | 渲染 Pauli 基期望值柱状图。 |

### 测量结果可视化

| 名字 | 简介 |
| --- | --- |
| [`plot_histogram`](4_result_plots.md) | 渲染计数直方图。 |
| [`plot_distribution`](4_result_plots.md) | 渲染归一化概率分布图。 |

### 输出与类型

| 名字 | 简介 |
| --- | --- |
| [`output_path`](5_render_to_file.md) | 把同一份 SVG 落盘的通用参数，扩展名决定写入 SVG 文本还是光栅化为 PNG。 |
| [`FigureSize`](5_render_to_file.md) | 绘图尺寸类型别名 `tuple[float, float]`。 |
| [`QuantumState`](5_render_to_file.md) | 状态渲染入口接受的状态类型别名。 |
| [`_InlineSvg`](5_render_to_file.md) | 返回值包装类型，提供 notebook 内联显示。 |

---

## 快速示例

### 1. 线路的文本图与 SVG 线路图

```python
from cqlib import Circuit
from cqlib.visualization import draw_figure, draw_text

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
circuit.measure(0)
circuit.measure(1)

text = draw_text(circuit)
assert "H" in text

svg = draw_figure(circuit, output_path="bell.svg")
assert "<svg" in svg
```

### 2. 测量结果的计数直方图

```python
from cqlib.device import ExecutionResult
from cqlib.visualization import plot_histogram

result = ExecutionResult.from_counts(
    "vis-test",
    [0, 1],
    7,
    2,
    {"00": 2, "11": 5},
)

histogram = plot_histogram(result)
assert "Count" in histogram
```

### 3. 量子态的状态图

```python
from cqlib.qis import Statevector
from cqlib.visualization import plot_bloch_multivector, plot_state_paulivec

state = Statevector(1)
state.apply_h(0)

bloch = plot_bloch_multivector(state)
assert "data-cqlib-bloch-3d" in bloch

pauli = plot_state_paulivec(state)
assert "<svg" in pauli
```

---

## 校验与错误处理

渲染入口在参数解析、输入校验和输出落盘阶段都可能失败。常见异常包括：

| 异常 | 触发场景 |
| --- | --- |
| `TypeError` | 输入对象类型与入口不匹配，例如把非线路对象传给 `draw_figure`，或把非 `ExecutionResult` 对象传给 `plot_histogram`。 |
| `ValueError` | 参数或输入内容非法，例如 Bloch 向量长度不是 3、`sort` 取值不在允许集合内、`sort="hamming"` 未提供 `target_string`、状态对象既不是 `Statevector` 也不是 `DensityMatrix`、线路预处理失败或操作引用了线路中不存在的量子比特。 |
| `IOError` | `output_path` 指向的路径写入失败；`IOError` 即 `OSError`。 |

PNG 光栅化失败等渲染错误同样以 `ValueError` 报出。各入口的具体触发条件见后续页面。

线路对象与门定义参考量子电路模块文档；执行结果对象参考设备模块文档；线路经编译后的形态与目标约束参考编译优化模块文档。
