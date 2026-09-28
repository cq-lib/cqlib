# 文本线路图

`cqlib.visualization.draw_text`

`draw_text` 把线路渲染成由框线字符组成的文本线路图，输出为纯文本，可直接打印到终端或写入日志，用于快速核对门序、比特连接、参数取值与折行后的行结构。

## 导入

```python
from cqlib.visualization import draw_text
```

---

## 函数

### draw_text(circuit, *, line_width=None, initial_state=False, reverse_bits=False, show_params=True, decompose_circuit_gates=False)

渲染线路的文本线路图。

参数：

- `circuit` (`Circuit`)：待渲染的线路。
- `line_width` (`int | None`)：每行文本的最大宽度，按显示列数计，超出后折行；取值不大于 10 时使用默认宽度 80，负值表示不折行。
- `initial_state` (`bool`)：是否在每条量子比特线路的起始处显示 `|0>`。
- `reverse_bits` (`bool`)：是否反转量子比特的显示顺序。
- `show_params` (`bool`)：是否在门标签后附上门的参数。
- `decompose_circuit_gates` (`bool`)：是否先把线路门展开为内部操作后再绘制。

`line_width` 之后的参数全部为关键字参数。

返回：

- `str`：文本线路图。该入口直接转发底层实现，返回值是普通字符串，不提供内联显示；需要把渲染结果内联显示时使用 [SVG 线路图](2_draw_figure.md)。

异常情况：

- `TypeError`：`circuit` 不是线路对象。
- `ValueError`：线路预处理失败，或线路中的操作引用了不存在的量子比特。

---

## 渲染行为

- 每条量子比特线路用框线字符（如 `─`、`│`）绘制，门以字符框的形式标注在线路上。
- `initial_state=True` 时，线路起始处显示 `|0>`，用于强调线路的初始态假设。
- `show_params=True` 时，带参数的旋转门在标签中附带参数值；关闭后只显示门名。
- `reverse_bits=True` 时，最高编号的量子比特显示在最上方，便于与按位书写的比特串对齐阅读。
- `decompose_circuit_gates=True` 时，线路门先展开为内部操作，文本图中不再出现线路门这一层。

---

## 示例

### 1. 渲染线路的文本图

```python
from cqlib import Circuit
from cqlib.visualization import draw_text

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
circuit.measure(0)
circuit.measure(1)

text = draw_text(circuit)
assert "H" in text
```

### 2. 控制折行与显示内容

```python
from cqlib import Circuit
from cqlib.visualization import draw_text

circuit = Circuit(3)
circuit.h(0)
circuit.rx(1, 0.25)
circuit.cx(0, 2)
circuit.barrier([0, 1, 2])
circuit.swap(1, 2)
circuit.measure(0)
circuit.measure(1)
circuit.measure(2)

text = draw_text(
    circuit,
    line_width=40,
    initial_state=True,
    reverse_bits=True,
    show_params=False,
)
```

该示例只渲染文本线路图，不产生落盘文件；需要把同一线路渲染成可落盘的 SVG 时使用 [draw_figure](2_draw_figure.md)。
