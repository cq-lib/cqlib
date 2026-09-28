# SVG Circuit Diagrams

`cqlib.visualization.draw_figure`

`draw_figure` renders a circuit into an SVG circuit diagram. Compared with a text circuit diagram, it represents operations such as quantum gates, connection lines, barriers and measurements with graphical symbols, is suitable for presentation in documents, web pages and notebooks, and can be written to file directly through an output path.

## Import

```python
from cqlib.visualization import draw_figure
```

---

## Functions

### draw_figure(circuit, *, fold=None, initial_state=False, reverse_bits=False, show_params=True, decompose_circuit_gates=False, output_path=None)

Render an SVG circuit diagram of the circuit.

Parameters:

- `circuit` (`Circuit`): the circuit to render.
- `fold` (`int | None`): the maximum number of columns drawn per row; a negative value means no folding; when not passed, the default value of 18 is used.
- `initial_state` (`bool`): whether to display `|0>` in the qubit labels.
- `reverse_bits` (`bool`): whether to reverse the display order of the qubits.
- `show_params` (`bool`): whether to append the gate parameters after the gate label.
- `decompose_circuit_gates` (`bool`): whether to first expand circuit gates into their internal operations before drawing.
- `output_path` (`str | None`): the output path. When given, the same SVG is written to that path; when the extension is `.png` the SVG is rasterized to PNG, and for other extensions the SVG text is written directly.

All parameters after `circuit` are keyword arguments.

Returns:

- `_InlineSvg`: SVG text, whose type is a subclass of `str`; it is displayed inline when used as the last expression of a cell in a notebook frontend. Whether or not `output_path` is given does not affect the content of the return value.

Raises:

- `TypeError`: `circuit` is not a circuit object.
- `ValueError`: circuit preprocessing fails, an operation references a qubit that does not exist, or PNG rasterization fails.
- `IOError`: writing to the path pointed at by `output_path` fails; `IOError` is `OSError`.

---

## Rendering behavior

- `fold` controls folding: the circuit is split by columns into multiple rows, each row not exceeding the given number of columns; a negative value means no folding, and the whole circuit is drawn on one row; when not passed, the default number of columns is used.
- With `initial_state=True`, qubit labels take the form `q2 |0>`; with `reverse_bits=True` the display order is reversed, and the highest-numbered qubit is displayed at the top.
- With `show_params=False`, parameter values do not appear in the gate labels.
- With `decompose_circuit_gates=True`, circuit gates are first expanded into their internal operations, and the layer of circuit gates no longer appears in the drawing.

---

## Examples

### 1. Render a circuit diagram with the default style

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

### 2. Fold, reverse the qubit order and hide parameters

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

Circuit folding and qubit order reversal only affect the drawing result; `initial_state` is merely a label annotation and does not rewrite the circuit itself. Use [draw_text](1_draw_text.md) when a plain-text output is needed.
