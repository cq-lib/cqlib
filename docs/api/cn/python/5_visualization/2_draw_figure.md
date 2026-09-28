# SVG 线路图

`cqlib.visualization.draw_figure`

`draw_figure` 把线路渲染成 SVG 线路图。与文本线路图相比，它以图形符号表示量子门、连接线、屏障与测量等操作，适合在文档、网页与 notebook 中展示，并可通过输出路径直接落盘。

## 导入

```python
from cqlib.visualization import draw_figure
```

---

## 函数

### draw_figure(circuit, *, fold=None, initial_state=False, reverse_bits=False, show_params=True, decompose_circuit_gates=False, output_path=None)

渲染线路的 SVG 线路图。

参数：

- `circuit` (`Circuit`)：待渲染的线路。
- `fold` (`int | None`)：每行最多绘制的列数，负值表示不折叠；不传时使用默认值 18。
- `initial_state` (`bool`)：是否在比特标签中显示 `|0>`。
- `reverse_bits` (`bool`)：是否反转量子比特的显示顺序。
- `show_params` (`bool`)：是否在门标签后附上门的参数。
- `decompose_circuit_gates` (`bool`)：是否先把线路门展开为内部操作后再绘制。
- `output_path` (`str | None`)：输出路径。给出时，同一份 SVG 写入该路径；扩展名为 `.png` 时按该 SVG 光栅化为 PNG，其余扩展名直接写入 SVG 文本。

`circuit` 之后的参数全部为关键字参数。

返回：

- `_InlineSvg`：SVG 文本，类型为 `str` 的子类，在 notebook 前端中作为单元格的最后一个表达式时内联显示。是否给出 `output_path` 都不影响返回值内容。

异常情况：

- `TypeError`：`circuit` 不是线路对象。
- `ValueError`：线路预处理失败、操作引用了不存在的量子比特，或 PNG 光栅化失败。
- `IOError`：`output_path` 指向的路径写入失败；`IOError` 即 `OSError`。

---

## 渲染行为

- `fold` 控制折叠：按列把线路切成多行，每行不超过给定列数；负值表示不折叠，整条线路绘制在一行；不传时使用默认列数。
- `initial_state=True` 时，比特标签形如 `q2 |0>`；`reverse_bits=True` 时显示顺序反转，编号最大的量子比特显示在最上方。
- `show_params=False` 时，门标签中不出现参数取值。
- `decompose_circuit_gates=True` 时，线路门先展开为内部操作，图中不再出现线路门这一层。

---

## 示例

### 1. 渲染默认样式的线路图

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

### 2. 折叠、反转比特序并隐藏参数

```python
from cqlib import Circuit
from cqlib.visualization import draw_figure

circuit = Circuit(3)
circuit.h(0)
circuit.rx(1, 0.25)
circuit.cx(0, 2)
circuit.barrier([0, 1, 2])
circuit.swap(1, 2)
circuit.measure(0)
circuit.measure(1)
circuit.measure(2)

svg = draw_figure(
    circuit,
    fold=2,
    initial_state=True,
    reverse_bits=True,
    show_params=False,
    output_path="folded.svg",
)
assert "q2 |0" in svg
assert "q0 |0" in svg
assert "0.25" not in svg
```

线路折叠与比特序反转只影响绘制结果；`initial_state` 只是标签标注，不改写线路本身。需要纯文本产物时使用 [draw_text](1_draw_text.md)。
