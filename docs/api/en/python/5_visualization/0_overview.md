# Visualization

`cqlib.visualization`

`cqlib.visualization` renders quantum circuits, quantum states and measurement results into visualization output. It takes SVG as the primary output format: every rendering entry point generates SVG text in memory as its return value, and writes the same SVG to file when an output path is given; for scenarios that need to inspect a circuit in a terminal or a plain-text environment, text circuit diagrams are provided as well.

`cqlib.visualization` mainly covers the following capabilities:

- **Circuit rendering**: draw a circuit as an SVG circuit diagram or a text circuit diagram, for checking gate order, qubit connections, parameter values and folding effects.
- **Quantum state rendering**: present quantum state information as a single Bloch vector, a reduced Bloch vector per qubit, a state matrix and Pauli expectation values.
- **Measurement result rendering**: render the counts of an execution result as a counts histogram or a normalized probability distribution plot.
- **Unified output contract**: all rendering entry points share the SVG text return value and the output path writing convention.

## Import

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

The rendering entry points fall into three groups by input object; the input types and output of each group are as follows:

| Input | Rendering entry points | Output |
| --- | --- | --- |
| Circuit (`Circuit`) | `draw_text`, `draw_figure` | Text circuit diagram, SVG circuit diagram |
| Quantum state (`Statevector` or `DensityMatrix`) | `plot_bloch_vector`, `plot_bloch_multivector`, `plot_state_city`, `plot_state_paulivec` | SVG state plot |
| Execution result (`ExecutionResult`) | `plot_histogram`, `plot_distribution` | SVG counts histogram, SVG probability distribution plot |

### Unified output contract

All rendering entry points follow the same output flow:

1. Call the rendering entry point to get the return value, which is the text content of the rendering output. Except for `draw_text`, which returns a text circuit diagram, the other entry points return SVG text, whose type is the `str` subclass `_InlineSvg`.
2. In a notebook frontend, using the return value as the last expression of a cell displays it inline; `_InlineSvg` accomplishes this through the SVG rich display protocol. See [Rendering output and file writing](5_render_to_file.md).
3. When `output_path` is passed, the same SVG is written to that path at the same time. When the extension is `.png` the same SVG is rasterized to PNG, and for other extensions the SVG text is written directly.

### Grouping of options

The option sets of the three groups of entry points are independent of each other, and all parameters after `*` in the signatures are keyword arguments:

- **Circuit options**: `initial_state`, `reverse_bits`, `show_params` and `decompose_circuit_gates` are shared by the two circuit entry points; `fold` is provided only by `draw_figure`, and `line_width` only by `draw_text`.
- **State options**: `title`, `color`, `alpha`, `reverse_bits`, `figsize`; the four state rendering entry points have identical signatures.
- **Result options**: `figsize`, `color`, `number_to_keep`, `sort`, `target_string`, `legend`, `bar_labels`, `title`; the two result rendering entry points have identical signatures.

An option of the same name has the same meaning across the three groups: `reverse_bits` controls whether the display of the computational basis qubit order is reversed, `figsize` controls the plot size, `title` controls the title text, and `output_path` controls whether the output is additionally written to file.

---

## Common entry points

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

## Core concepts and terms

| Term | Description |
| --- | --- |
| **SVG circuit diagram** | The vector circuit diagram rendered by `draw_figure`, using graphical symbols for operations such as quantum gates, connection lines and measurements; it can be scaled and embedded directly into documents or web pages. |
| **Text circuit diagram** | The plain-text drawing rendered by `draw_text`, using box-drawing characters to draw qubit lines and gate boxes; suitable for terminal and log scenarios. |
| **Inline display** | The behavior whereby a rendering return value is rendered directly as a rich output in a notebook frontend, provided by the SVG rich display protocol of `_InlineSvg`. |
| **Plot size** | The `(width, height)` size given by `figsize`, in inches, converted to pixels at render time; when not passed, the default size of that plot family is used. |
| **Fold** | The drawing split controlled by `fold`: a longer circuit is split by columns into multiple rows, each row not exceeding the given number of columns. |
| **Counts histogram** | A plot whose bar heights are the occurrence counts of each bitstring in the measurement result, rendered by `plot_histogram`. |
| **Probability distribution plot** | A plot rendered after normalizing the measurement result counts into probabilities, rendered by `plot_distribution`. |
| **State plot** | A plot for quantum states, comprising four kinds: a single Bloch vector, a reduced Bloch vector per qubit, a state matrix and Pauli expectation values. |
| **Pauli expectation value** | The Pauli basis coefficients computed with the density matrix as input; positive and negative values are presented in different colors. |
| **Hamming distance** | The number of positions at which the corresponding bits of two equal-length bitstrings differ; used by the `hamming` sort strategy of `plot_histogram` and `plot_distribution`. |

---

## `cqlib.visualization` API Overview

The table below lists the pages of this module grouped by function (this page is the module overview).

### Circuit visualization

| Name | Description |
| --- | --- |
| [`draw_text`](1_draw_text.md) | Render a text circuit diagram, supporting fold width, initial state annotation and qubit order control. |
| [`draw_figure`](2_draw_figure.md) | Render an SVG circuit diagram, supporting folding, initial state annotation and parameter display. |

### Quantum state visualization

| Name | Description |
| --- | --- |
| [`plot_bloch_vector`](3_state_plots.md) | Render a single Bloch vector. |
| [`plot_bloch_multivector`](3_state_plots.md) | Render one reduced Bloch vector for each quantum qubit. |
| [`plot_state_city`](3_state_plots.md) | Render the real and imaginary panels of the density matrix. |
| [`plot_state_paulivec`](3_state_plots.md) | Render a bar chart of Pauli basis expectation values. |

### Measurement result visualization

| Name | Description |
| --- | --- |
| [`plot_histogram`](4_result_plots.md) | Render a counts histogram. |
| [`plot_distribution`](4_result_plots.md) | Render a normalized probability distribution plot. |

### Output and types

| Name | Description |
| --- | --- |
| [`output_path`](5_render_to_file.md) | The common parameter that writes the same SVG to file; the extension determines whether SVG text is written or PNG is rasterized. |
| [`FigureSize`](5_render_to_file.md) | Plot size type alias `tuple[float, float]`. |
| [`QuantumState`](5_render_to_file.md) | The state type alias accepted by the state rendering entry points. |
| [`_InlineSvg`](5_render_to_file.md) | The return value wrapper type, providing notebook inline display. |

---

## Quick examples

### 1. Text drawing and SVG circuit diagram of a circuit

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

### 2. Counts histogram of a measurement result

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

### 3. State plots of a quantum state

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

## Validation and error handling

The rendering entry points may fail at the parameter parsing, input validation and output writing stages. Common exceptions include:

| Exception | When it occurs |
| --- | --- |
| `TypeError` | The input object type does not match the entry point, for example passing a non-circuit object to `draw_figure`, or a non-`ExecutionResult` object to `plot_histogram`. |
| `ValueError` | A parameter or the input content is invalid, for example the Bloch vector length is not 3, the `sort` value is not in the allowed set, `sort="hamming"` is used without `target_string`, the state object is neither a `Statevector` nor a `DensityMatrix`, circuit preprocessing fails, or an operation references a qubit that does not exist in the circuit. |
| `IOError` | Writing to the path pointed at by `output_path` fails; `IOError` is `OSError`. |

Rendering errors such as a PNG rasterization failure are likewise reported as `ValueError`. For the specific conditions of each entry point see the following pages.

For the circuit object and gate definitions see the quantum circuit module documentation; for the execution result object see the device module documentation; for the compiled form of a circuit and target constraints see the compilation module documentation.
