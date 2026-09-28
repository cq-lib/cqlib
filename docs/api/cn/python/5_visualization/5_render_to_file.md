# 渲染输出与落盘

`cqlib.visualization`

本页说明 `cqlib.visualization` 各渲染入口共享的输出约定：SVG 文本返回值与 `output_path` 落盘的关系、`FigureSize` 与 `QuantumState` 两个类型别名的含义，以及返回值在 notebook 中的内联显示行为。

## 导入

```python
from cqlib.visualization import FigureSize, QuantumState
```

---

## 输出路径与返回值

每个渲染入口都在内存中先生成产物文本，再按需写盘：

- **返回值**：渲染产物的文本内容。除 `draw_text` 返回文本线路图外，其余入口返回 SVG 文本。
- **`output_path`**：可选的关键字参数。给出时，把同一份 SVG 写入该路径。落盘动作不改变返回值的内容，两者来自同一份渲染结果。
- **扩展名规则**：扩展名为 `.png`（不区分大小写）时，按同一份 SVG 光栅化为 PNG 文件；其余扩展名（含 `.svg` 与无扩展名）直接写入 SVG 文本。

```python
from cqlib import Circuit
from cqlib.visualization import draw_figure

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

# 只取返回值：SVG 文本留在内存中，不产生文件
svg = draw_figure(circuit)

# 同时落盘：返回值不变，同一份 SVG 另写入 "bell.svg"
svg = draw_figure(circuit, output_path="bell.svg")

# 扩展名为 .png：返回值不变，同一份 SVG 另光栅化为 "bell.png"
svg = draw_figure(circuit, output_path="bell.png")
```

异常情况：

- `IOError`：`output_path` 指向的路径写入失败；`IOError` 即 `OSError`。
- `ValueError`：PNG 光栅化失败，例如 SVG 无法解析或目标尺寸超出可表示范围。

---

## FigureSize

```python
FigureSize = tuple[float, float]
```

绘图尺寸类型别名，用于各渲染入口的 `figsize` 参数。两个元素依次为宽与高，单位为英寸。

- 不传 `figsize` 时，使用该绘图族的默认尺寸。
- 渲染时把英寸换算为 SVG 画布的像素尺寸；各绘图族的默认尺寸与换算下限不同，因此同一组取值在不同绘图族中得到的画布尺寸可能不同。
- 取值过小时按该绘图族的下限处理，不会得到退化的画布。

```python
from cqlib.device import ExecutionResult
from cqlib.visualization import plot_distribution

result = ExecutionResult.from_counts("vis-test", [0], 100, 1, {"0": 25, "1": 75})

svg = plot_distribution(result, figsize=(3.2, 2.4))
assert 'width="320"' in svg
assert 'height="240"' in svg
```

---

## QuantumState

```python
QuantumState = Statevector | DensityMatrix
```

状态渲染入口接受的状态类型别名，取值为 `cqlib.qis.Statevector` 或 `cqlib.qis.DensityMatrix`。两个状态类型可互换使用，同一量子态用两种输入渲染得到相同结果，详见 [量子态可视化](3_state_plots.md)。

传入既不是 `Statevector` 也不是 `DensityMatrix` 的对象时，状态渲染入口以 `ValueError` 报出。

---

## 内联显示

各渲染入口的 SVG 返回值不是普通字符串，而是 `str` 的子类 `_InlineSvg`。它保持字符串的全部行为，同时实现 SVG 富显示协议：

- `_repr_svg_()` 返回 SVG 文本本身，notebook 前端据此把返回值作为图形直接渲染。
- 内联显示只在返回值被用作单元格的最后一个表达式时触发；把返回值赋给变量或存入容器时不会自动显示，需要显式求值该变量。
- 由于返回值是 `str` 的子类，可以直接参与字符串比较、拼接与写文件。

```python
from cqlib import Circuit
from cqlib.visualization import draw_figure

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
circuit.measure(0)
circuit.measure(1)

svg = draw_figure(circuit, output_path="bell.svg")
assert svg._repr_svg_() == str(svg)
assert "<svg" in svg
```

`draw_text` 的返回值是普通 `str`，即文本线路图，不实现 SVG 富显示协议；需要内联显示时改用 [draw_figure](2_draw_figure.md)。
