# Quantum State Visualization

`cqlib.visualization`

The quantum state visualization entry points render a quantum state into SVG state plots, comprising four kinds: a single Bloch vector, a reduced Bloch vector per qubit, a state matrix (real and imaginary panels) and a bar chart of Pauli basis expectation values. The four entry points have identical signatures; they differ in the plot family rendered and the input form accepted.

## Import

```python
from cqlib.visualization import (
    plot_bloch_multivector,
    plot_bloch_vector,
    plot_state_city,
    plot_state_paulivec,
)
```

---

## Input and state types

The three entry points that take a state as input (`plot_bloch_multivector`, `plot_state_city`, `plot_state_paulivec`) accept `cqlib.qis.Statevector` or `cqlib.qis.DensityMatrix`, that is, the type alias [`QuantumState`](5_render_to_file.md). Rendering the same quantum state with either input yields the same SVG content:

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

The input of `plot_bloch_vector` is not a state object but a vector made of three real numbers.

---

## Common parameters

Apart from the first positional parameter, the four entry points take exactly the same parameters, all of which are keyword arguments:

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `title` | `str \| None` | `None` | The chart title; when not passed, no title is drawn. |
| `color` | `list[str] \| None` | `None` | Plot colors. `plot_state_paulivec` takes the first element as the color of positive-coefficient bars and the second element as the color of negative-coefficient bars; the other entry points accept this parameter but do not use it for coloring. |
| `alpha` | `float` | `1.0` | The fill opacity of the matrix panels; takes effect only in `plot_state_city`, and is clamped between 0.05 and 1.0 at render time. |
| `reverse_bits` | `bool` | `False` | Whether to reverse the display of the computational basis qubit order. |
| `figsize` | [`FigureSize`](5_render_to_file.md) `\| None` | `None` | The plot size in inches; when not passed, the default size of that plot family is used. |
| `output_path` | `str \| None` | `None` | The output path; when given, the same SVG is written to that path, see [Rendering output and file writing](5_render_to_file.md). |

---

## Functions

### plot_bloch_vector(vector, *, title=None, color=None, alpha=1.0, reverse_bits=False, figsize=None, output_path=None)

Render a single Bloch vector.

Parameters:

- `vector` (`Sequence[float]`): a real number sequence of length 3, corresponding in order to the three components `(x, y, z)`.
- For the remaining parameters see [Common parameters](#common-parameters).

Returns:

- `_InlineSvg`: SVG text, whose type is a subclass of `str`; it is displayed inline when used as the last expression of a cell in a notebook frontend.

Raises:

- `ValueError`: the number of elements of `vector` is not 3.
- `IOError`: writing to the path pointed at by `output_path` fails.

---

### plot_bloch_multivector(state, *, title=None, color=None, alpha=1.0, reverse_bits=False, figsize=None, output_path=None)

Render one reduced Bloch vector for each qubit in the quantum state, for observing the local state of each qubit.

Parameters:

- `state` (`QuantumState`): the quantum state to render.
- For the remaining parameters see [Common parameters](#common-parameters).

Returns:

- `_InlineSvg`: SVG text.

Raises:

- `ValueError`: `state` is neither a `Statevector` nor a `DensityMatrix`.
- `IOError`: writing to the path pointed at by `output_path` fails.

---

### plot_state_city(state, *, title=None, color=None, alpha=1.0, reverse_bits=False, figsize=None, output_path=None)

Represent the quantum state as a density matrix and render its real and imaginary panels.

Parameters:

- `state` (`QuantumState`): the quantum state to render.
- For the remaining parameters see [Common parameters](#common-parameters).

Returns:

- `_InlineSvg`: SVG text.

Raises:

- `ValueError`: `state` is neither a `Statevector` nor a `DensityMatrix`.
- `IOError`: writing to the path pointed at by `output_path` fails.

---

### plot_state_paulivec(state, *, title=None, color=None, alpha=1.0, reverse_bits=False, figsize=None, output_path=None)

Compute the expectation values of the quantum state in the Pauli basis and render them as a bar chart.

Parameters:

- `state` (`QuantumState`): the quantum state to render.
- For the remaining parameters see [Common parameters](#common-parameters).

Returns:

- `_InlineSvg`: SVG text.

Raises:

- `ValueError`: `state` is neither a `Statevector` nor a `DensityMatrix`.
- `IOError`: writing to the path pointed at by `output_path` fails.

---

## Rendering behavior

- `plot_bloch_vector` renders one Bloch sphere, with the three components of `vector` corresponding in order to `(x, y, z)`; when the norm exceeds 1 the vector is scaled back to the unit sphere.
- `plot_bloch_multivector` renders one Bloch sphere per qubit, with the spheres labeled by qubit index and arranged in alignment.
- The real panel of `plot_state_city` is titled `Re[rho]` and the imaginary panel `Im[rho]`; when the imaginary part of the density matrix is entirely zero only the real panel is rendered. Matrix elements are represented as squares whose area grows with the absolute value of the element, and positive and negative values are distinguished by different colors.
- `plot_state_paulivec` gives the expectation value of each Pauli basis as a bar chart, with positive and negative coefficients distinguished by different colors.
- For all four entry points, `reverse_bits` only changes the display order of the computational basis labels and does not change the quantum state itself; `title` and `figsize` only affect the canvas and do not affect the computed results.

---

## Examples

### 1. A single Bloch vector

```python
from cqlib.visualization import plot_bloch_vector

svg = plot_bloch_vector([0.0, 0.0, 1.0], output_path="bloch_z.svg")
assert "data-cqlib-bloch-3d" in svg
```

### 2. A reduced Bloch vector per qubit

```python
from cqlib.qis import Statevector
from cqlib.visualization import plot_bloch_multivector

state = Statevector(1)
state.apply_h(0)

svg = plot_bloch_multivector(state, output_path="bloch_multivector.svg")
assert "data-cqlib-bloch-3d" in svg
```

### 3. State matrix

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

### 4. Pauli expectation values

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
