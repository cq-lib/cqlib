# Text Circuit Diagrams

`cqlib.visualization.draw_text`

`draw_text` renders a circuit into a text circuit diagram made of box-drawing characters; the output is plain text that can be printed directly to a terminal or written to a log, for quickly checking gate order, qubit connections, parameter values and the row structure after folding.

## Import

```python
from cqlib.visualization import draw_text
```

---

## Functions

### draw_text(circuit, *, line_width=None, initial_state=False, reverse_bits=False, show_params=True, decompose_circuit_gates=False)

Render a text circuit diagram of the circuit.

Parameters:

- `circuit` (`Circuit`): the circuit to render.
- `line_width` (`int | None`): the maximum width of each text row, counted in display columns, folding beyond it; a value not greater than 10 uses the default width of 80, and a negative value means no folding.
- `initial_state` (`bool`): whether to display `|0>` at the start of each qubit line.
- `reverse_bits` (`bool`): whether to reverse the display order of the qubits.
- `show_params` (`bool`): whether to append the gate parameters after the gate label.
- `decompose_circuit_gates` (`bool`): whether to first expand circuit gates into their internal operations before drawing.

All parameters after `line_width` are keyword arguments.

Returns:

- `str`: the text circuit diagram. This entry point forwards directly to the underlying implementation, and the return value is an ordinary string that does not provide inline display; use [SVG circuit diagrams](2_draw_figure.md) when the rendering result needs to be displayed inline.

Raises:

- `TypeError`: `circuit` is not a circuit object.
- `ValueError`: circuit preprocessing fails, or an operation in the circuit references a qubit that does not exist.

---

## Rendering behavior

- Each qubit line is drawn with box-drawing characters (such as `─`, `│`), and gates are annotated on the line in the form of character boxes.
- With `initial_state=True`, `|0>` is displayed at the start of the line, to emphasize the initial state assumption of the circuit.
- With `show_params=True`, rotation gates that carry parameters include the parameter values in their label; when disabled, only the gate name is displayed.
- With `reverse_bits=True`, the highest-numbered qubit is displayed at the top, making it easier to read in alignment with bitstrings written in bit order.
- With `decompose_circuit_gates=True`, circuit gates are first expanded into their internal operations, and the layer of circuit gates no longer appears in the text drawing.

---

## Examples

### 1. Render a text drawing of a circuit

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

### 2. Control folding and displayed content

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

This example only renders a text circuit diagram and produces no file; to render the same circuit as an SVG that can be written to file, use [draw_figure](2_draw_figure.md).
