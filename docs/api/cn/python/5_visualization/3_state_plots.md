# 量子态可视化

`cqlib.visualization`

量子态可视化入口把量子态渲染成 SVG 状态图，包括单个 Bloch 向量、每比特约化 Bloch 向量、态矩阵（实部与虚部面板）以及 Pauli 基期望值柱状图四类。四个入口的签名一致，区别在于渲染的图族与接受的输入形态。

## 导入

```python
from cqlib.visualization import (
    plot_bloch_multivector,
    plot_bloch_vector,
    plot_state_city,
    plot_state_paulivec,
)
```

---

## 输入与状态类型

三个以状态为输入的入口（`plot_bloch_multivector`、`plot_state_city`、`plot_state_paulivec`）接受 `cqlib.qis.Statevector` 或 `cqlib.qis.DensityMatrix`，即类型别名 [`QuantumState`](5_render_to_file.md)。同一个量子态用两种输入渲染，得到的 SVG 内容一致：

```python
from cqlib.qis import DensityMatrix, Statevector
from cqlib.visualization import plot_state_city, plot_state_paulivec

state = Statevector(1)
state.apply_h(0)

density = DensityMatrix(1)
density.apply_h(0)

assert plot_state_city(state) == plot_state_city(density)
assert plot_state_paulivec(state) == plot_state_paulivec(density)
```

`plot_bloch_vector` 的输入不是状态对象，而是三个实数组成的向量。

---

## 共同参数

四个入口除首个位置参数外，其余参数完全相同，且全部为关键字参数：

| 参数 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `title` | `str \| None` | `None` | 图表标题，不传时不绘制标题。 |
| `color` | `list[str] \| None` | `None` | 绘图颜色。`plot_state_paulivec` 取第一个元素作为正系数柱颜色、第二个元素作为负系数柱颜色；其余入口接受该参数但不参与配色。 |
| `alpha` | `float` | `1.0` | 矩阵面板的填充不透明度，仅在 `plot_state_city` 中生效，渲染时限制在 0.05 至 1.0 之间。 |
| `reverse_bits` | `bool` | `False` | 是否反转计算基比特序的显示。 |
| `figsize` | [`FigureSize`](5_render_to_file.md) `\| None` | `None` | 绘图尺寸，单位为英寸；不传时使用该绘图族的默认尺寸。 |
| `output_path` | `str \| None` | `None` | 输出路径，给出时同一份 SVG 写入该路径，见 [渲染输出与落盘](5_render_to_file.md)。 |

---

## 函数

### plot_bloch_vector(vector, *, title=None, color=None, alpha=1.0, reverse_bits=False, figsize=None, output_path=None)

渲染单个 Bloch 向量。

参数：

- `vector` (`Sequence[float]`)：长度为 3 的实数序列，依次对应 `(x, y, z)` 三个分量。
- 其余参数见 [共同参数](#共同参数)。

返回：

- `_InlineSvg`：SVG 文本，类型为 `str` 的子类，在 notebook 前端中作为单元格的最后一个表达式时内联显示。

异常情况：

- `ValueError`：`vector` 的元素个数不是 3。
- `IOError`：`output_path` 指向的路径写入失败。

---

### plot_bloch_multivector(state, *, title=None, color=None, alpha=1.0, reverse_bits=False, figsize=None, output_path=None)

对量子态中的每个量子比特各渲染一个约化 Bloch 向量，用于观察各比特的局部态。

参数：

- `state` (`QuantumState`)：待渲染的量子态。
- 其余参数见 [共同参数](#共同参数)。

返回：

- `_InlineSvg`：SVG 文本。

异常情况：

- `ValueError`：`state` 既不是 `Statevector` 也不是 `DensityMatrix`。
- `IOError`：`output_path` 指向的路径写入失败。

---

### plot_state_city(state, *, title=None, color=None, alpha=1.0, reverse_bits=False, figsize=None, output_path=None)

把量子态表示为密度矩阵，渲染其实部与虚部两个面板。

参数：

- `state` (`QuantumState`)：待渲染的量子态。
- 其余参数见 [共同参数](#共同参数)。

返回：

- `_InlineSvg`：SVG 文本。

异常情况：

- `ValueError`：`state` 既不是 `Statevector` 也不是 `DensityMatrix`。
- `IOError`：`output_path` 指向的路径写入失败。

---

### plot_state_paulivec(state, *, title=None, color=None, alpha=1.0, reverse_bits=False, figsize=None, output_path=None)

计算量子态在 Pauli 基下的期望值，并渲染成柱状图。

参数：

- `state` (`QuantumState`)：待渲染的量子态。
- 其余参数见 [共同参数](#共同参数)。

返回：

- `_InlineSvg`：SVG 文本。

异常情况：

- `ValueError`：`state` 既不是 `Statevector` 也不是 `DensityMatrix`。
- `IOError`：`output_path` 指向的路径写入失败。

---

## 渲染行为

- `plot_bloch_vector` 渲染一个 Bloch 球，`vector` 的三个分量依次对应 `(x, y, z)`；模长超过 1 时按比例缩放回单位球面。
- `plot_bloch_multivector` 为每个量子比特渲染一个 Bloch 球，球按量子比特编号标注并对齐排列。
- `plot_state_city` 的实部面板标题为 `Re[rho]`，虚部面板标题为 `Im[rho]`；密度矩阵虚部全为零时只渲染实部面板。矩阵元素以方块表示，方块面积随元素绝对值增大，取值正负用不同颜色区分。
- `plot_state_paulivec` 以柱状图给出各 Pauli 基的期望值，正负系数用不同颜色区分。
- 四个入口的 `reverse_bits` 都只改变计算基标签的显示顺序，不改变量子态本身；`title` 与 `figsize` 只影响画布，不影响计算结果。

---

## 示例

### 1. 单个 Bloch 向量

```python
from cqlib.visualization import plot_bloch_vector

svg = plot_bloch_vector([0.0, 0.0, 1.0], output_path="bloch_z.svg")
assert "data-cqlib-bloch-3d" in svg
```

### 2. 每比特约化 Bloch 向量

```python
from cqlib.qis import Statevector
from cqlib.visualization import plot_bloch_multivector

state = Statevector(1)
state.apply_h(0)

svg = plot_bloch_multivector(state, output_path="bloch_multivector.svg")
assert "data-cqlib-bloch-3d" in svg
```

### 3. 态矩阵

```python
from cqlib.qis import DensityMatrix
from cqlib.visualization import plot_state_city

density = DensityMatrix(2)
density.apply_h(0)

svg = plot_state_city(
    density,
    reverse_bits=True,
    alpha=0.7,
    figsize=(5.0, 4.0),
    title="Reverse state city",
    output_path="state_city.svg",
)
assert "Reverse state city" in svg
assert 'fill-opacity="0.700"' in svg
```

### 4. Pauli 期望值

```python
from cqlib.qis import Statevector
from cqlib.visualization import plot_state_paulivec

state = Statevector(1)
state.apply_h(0)

svg = plot_state_paulivec(
    state,
    color=["#00aa00", "#aa0000"],
    title="Pauli colors",
    figsize=(5.0, 3.0),
    output_path="paulivec.svg",
)
assert "Pauli colors" in svg
assert "#00aa00" in svg
```
