# 测量结果可视化

`cqlib.visualization`

测量结果可视化入口把执行结果中的测量计数渲染成 SVG 图。两者共享同一组选项：`plot_histogram` 以原始计数为柱高，`plot_distribution` 先把计数归一化为概率再绘制。

## 导入

```python
from cqlib.visualization import plot_distribution, plot_histogram
```

---

## 输入与结果类型

两个入口的输入都是 `cqlib.device.ExecutionResult`。可直接由计数构造一个已完成的结果对象：

```python
from cqlib.device import ExecutionResult

result = ExecutionResult.from_counts(
    "vis-test",
    [0, 1],
    7,
    2,
    {"00": 2, "11": 5},
)
```

计数键为比特串，长度需与量子比特数一致，否则构造结果对象时即报错。图中的比特串标签即结果对象中的计数键。

---

## 共同参数

两个入口除首个位置参数外，其余参数完全相同，且全部为关键字参数：

| 参数 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `figsize` | [`FigureSize`](5_render_to_file.md) `\| None` | `None` | 绘图尺寸，单位为英寸；不传时使用该绘图族的默认尺寸。 |
| `color` | `list[str] \| None` | `None` | 每个数据集一个颜色；不传时使用内置调色板。 |
| `number_to_keep` | `int \| None` | `None` | 只保留取值最大的 k 个柱，其余计数聚合为一个柱，标签为 `rest`；取值为 0 或不小于柱数时不聚合。 |
| `sort` | `str` | `"asc"` | 柱的排序策略，取值见[排序策略](#排序策略)。 |
| `target_string` | `str \| None` | `None` | `sort="hamming"` 使用的目标比特串。 |
| `legend` | `list[str] \| None` | `None` | 图例条目，每个数据集一条。由执行结果的计数渲染时只有一个数据集，给出一条即可；条目数与数据集数量不一致时报错。 |
| `bar_labels` | `bool` | `True` | 是否在柱上标注数值。 |
| `title` | `str \| None` | `None` | 图表标题，不传时不绘制标题。 |
| `output_path` | `str \| None` | `None` | 输出路径，给出时同一份 SVG 写入该路径，见 [渲染输出与落盘](5_render_to_file.md)。 |

---

## 函数

### plot_histogram(result, *, figsize=None, color=None, number_to_keep=None, sort="asc", target_string=None, legend=None, bar_labels=True, title=None, output_path=None)

把执行结果的测量计数渲染成计数直方图，纵轴为计数。

参数：

- `result` (`ExecutionResult`)：待渲染的执行结果。结果中没有计数时以 `ValueError` 报出。
- 其余参数见 [共同参数](#共同参数)。

返回：

- `_InlineSvg`：SVG 文本，类型为 `str` 的子类，在 notebook 前端中作为单元格的最后一个表达式时内联显示。

异常情况：

- `TypeError`：`result` 不是 `ExecutionResult` 对象。
- `ValueError`：结果中没有计数，或参数取值非法，例如 `sort` 不在允许的取值集合内、`sort="hamming"` 未提供 `target_string`、`legend` 条目数与数据集数量不一致。
- `IOError`：`output_path` 指向的路径写入失败。

---

### plot_distribution(result, *, figsize=None, color=None, number_to_keep=None, sort="asc", target_string=None, legend=None, bar_labels=True, title=None, output_path=None)

把执行结果的测量计数归一化为概率后渲染成概率分布图，纵轴为概率。

参数：

- `result` (`ExecutionResult`)：待渲染的执行结果。结果中没有计数、或计数之和为零时以 `ValueError` 报出。
- 其余参数见 [共同参数](#共同参数)。归一化在聚合 `rest` 柱之前完成，因此图中各柱高度之和为 1。

返回：

- `_InlineSvg`：SVG 文本。

异常情况：

- `TypeError`：`result` 不是 `ExecutionResult` 对象。
- `ValueError`：结果中没有计数、计数之和为零，或参数取值非法，例如 `sort` 不在允许的取值集合内、`sort="hamming"` 未提供 `target_string`、`legend` 条目数与数据集数量不一致。
- `IOError`：`output_path` 指向的路径写入失败。

---

## 排序策略

`sort` 的取值及其含义：

| 取值 | 说明 |
| --- | --- |
| `asc` | 按比特串标签升序排列，为默认取值。 |
| `desc` | 按比特串标签降序排列。 |
| `value` | 按各柱取值升序排列。 |
| `value_desc` | 按各柱取值降序排列。 |
| `hamming` | 按与 `target_string` 的 [Hamming 距离](0_overview.md)升序排列，聚合出的 `rest` 柱排在最后。 |

`hamming` 排序要求所有比特串与 `target_string` 等长，且必须提供 `target_string`；不满足时以 `ValueError` 报出。其他取值不使用 `target_string`。

---

## 示例

### 1. 计数直方图

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

svg = plot_histogram(result, output_path="histogram.svg")
assert "<svg" in svg
assert "Count" in svg
```

### 2. 按 Hamming 距离排序并聚合尾部计数

```python
from cqlib.device import ExecutionResult
from cqlib.visualization import plot_histogram

result = ExecutionResult.from_counts(
    "vis-test",
    [0, 1, 2],
    10,
    3,
    {"100": 4, "101": 3, "010": 2, "001": 1},
)

svg = plot_histogram(
    result,
    sort="hamming",
    target_string="100",
    number_to_keep=3,
    color=["#123456"],
    legend=["sim"],
    bar_labels=False,
    title="Hamming order",
    output_path="histogram_hamming.svg",
)
assert "Hamming order" in svg
assert "#123456" in svg
assert ">sim</text>" in svg
assert ">rest</text>" in svg
```

### 3. 概率分布图

```python
from cqlib.device import ExecutionResult
from cqlib.visualization import plot_distribution

result = ExecutionResult.from_counts(
    "vis-test",
    [0],
    100,
    1,
    {"0": 25, "1": 75},
)

svg = plot_distribution(
    result,
    figsize=(3.2, 2.4),
    color=["#0f766e"],
    legend=["probability"],
    bar_labels=False,
    title="Distribution options",
    output_path="distribution.svg",
)
assert 'width="320"' in svg
assert 'height="240"' in svg
assert "#0f766e" in svg
assert ">probability</text>" in svg
```
