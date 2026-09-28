# Rendering Output and File Writing

`cqlib.visualization`

This page explains the output convention shared by the rendering entry points of `cqlib.visualization`: the relationship between the SVG text return value and writing to file through `output_path`, the meaning of the two type aliases `FigureSize` and `QuantumState`, and the inline display behavior of the return value in a notebook.

## Import

```python
from cqlib.visualization import FigureSize, QuantumState
```

---

## Output path and return value

Every rendering entry point first generates the output text in memory and then writes it to file as needed:

- **Return value**: the text content of the rendering output. Except for `draw_text`, which returns a text circuit diagram, the other entry points return SVG text.
- **`output_path`**: an optional keyword argument. When given, the same SVG is written to that path. The write does not change the content of the return value; both come from the same rendering result.
- **Extension rules**: when the extension is `.png` (case-insensitive), the same SVG is rasterized into a PNG file; for other extensions (including `.svg` and no extension) the SVG text is written directly.

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

Raises:

- `IOError`: writing to the path pointed at by `output_path` fails; `IOError` is `OSError`.
- `ValueError`: PNG rasterization fails, for example the SVG cannot be parsed or the target size exceeds the representable range.

---

## FigureSize

```python
FigureSize = tuple[float, float]
```

Plot size type alias, used by the `figsize` parameter of the rendering entry points. The two elements are the width and the height in inches.

- When `figsize` is not passed, the default size of that plot family is used.
- At render time the inches are converted into the pixel size of the SVG canvas; the default sizes and conversion lower bounds differ between plot families, so the same values may yield different canvas sizes in different plot families.
- A too-small value is handled according to the lower bound of that plot family, so a degenerate canvas is never produced.

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

The state type alias accepted by the state rendering entry points, taking the value `cqlib.qis.Statevector` or `cqlib.qis.DensityMatrix`. The two state types are interchangeable, and rendering the same quantum state with either input yields the same result; see [Quantum state visualization](3_state_plots.md) for details.

When an object that is neither a `Statevector` nor a `DensityMatrix` is passed, the state rendering entry points report it as a `ValueError`.

---

## Inline display

The SVG return value of the rendering entry points is not an ordinary string but the `str` subclass `_InlineSvg`. It keeps all the behavior of a string while implementing the SVG rich display protocol:

- `_repr_svg_()` returns the SVG text itself, on the basis of which a notebook frontend renders the return value directly as a graphic.
- Inline display is triggered only when the return value is used as the last expression of a cell; assigning the return value to a variable or storing it in a container does not display it automatically, and the variable needs to be evaluated explicitly.
- Since the return value is a subclass of `str`, it can take part directly in string comparison, concatenation and file writing.

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

The return value of `draw_text` is an ordinary `str`, that is, the text circuit diagram, and does not implement the SVG rich display protocol; use [draw_figure](2_draw_figure.md) when inline display is needed.
