# Visualization

`cqlib_core::visualization`

`cqlib_core::visualization` is the visualization module of Cqlib; it renders circuits, quantum states and measurement results into graphical artifacts that can be viewed and saved. The module uses SVG as its unified output form: each drawing entry point first returns an SVG markup string, and an output entry point then writes it to a `.svg` file or rasterizes it into a `.png` file.

## Overview

Visualization answers three kinds of questions: what a circuit looks like, how a quantum state is distributed on the Bloch sphere and in the density matrix, and how the counts of one measurement are distributed. The module is divided by these three kinds of objects into the `circuit`, `state` and `result` submodules, plus a shared error type `VisualizationError`.

### Rendering pipeline

Circuit drawing has two steps: first convert the circuit into a backend-independent visual IR, then draw it with a concrete backend.

1. **IR construction** — `build_visual_circuit` places operations into non-overlapping columns in circuit order, resolves the style, label and parameter text of each operation, and recursively collects sub-circuits for control flow blocks.
2. **Backend drawing** — the text backend draws the IR into a Unicode box-drawing character diagram, and the figure backend draws the IR into SVG markup. Both backends consume the same IR, so the gate order, column partitioning and labels of a given circuit are identical under the two backends.
3. **Output** — the SVG string can be embedded directly or written to a file; a path with the `.png` extension is rasterized first.

Quantum state and measurement result drawing is single-step: the drawing entry points of `state` and `result` perform data validation, normalization and layout internally and return the SVG string directly.

### Two kinds of circuit backend

- **Text backend** (`circuit_to_text`): produces a monospaced character diagram that can be folded by width, suitable for viewing in terminals and logs.
- **Figure backend** (`circuit_to_figure`): produces SVG markup, and supports per-gate style overrides, per-column folding and PNG rasterization.

### Display bit order, folding and column folding

`reverse_bits` controls the display bit order: when set to `true`, qubits are displayed in reverse order, and the basis labels of quantum states and the rows and columns of density matrices are likewise arranged in reverse order, which makes it easier to align with an externally agreed qubit order.

The text backend uses `line_width` to control folding and the figure backend uses `fold` to control folding: when the width or the number of columns exceeds the limit, the diagram is split into segments, and segments are joined by direction markers.

### Inputs for quantum states and measurement results

Quantum state drawing accepts implementors of `StateVisualizationSource`, which both `Statevector` and `DensityMatrix` implement; a pure state is expanded into a density matrix before drawing. Measurement result drawing accepts `ExecutionResult` directly and formats the measurement results into bitstring labels according to the number of qubits recorded in the result.

---

## Common entry points

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{FigureDrawerOptions, circuit_to_figure};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let svg = circuit_to_figure(&circuit, &FigureDrawerOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## Core concepts and terms

| Term | Description |
| --- | --- |
| **Visual IR** | The circuit intermediate representation `VisualCircuit`, independent of the drawing backend, built by `build_visual_circuit` and drawn by both the text backend and the figure backend. |
| **Text backend** | The backend that draws the visual IR into a Unicode box-drawing character diagram; the entry point is `circuit_to_text`. |
| **Figure backend** | The backend that draws the visual IR into SVG markup; the entry point is `circuit_to_figure`. |
| **Fold** | Splitting the circuit into multiple segments for display when the text diagram width exceeds `line_width`, with segments joined by the `«` and `»` markers. |
| **Column folding** | Splitting the circuit into multiple rows for display when the number of columns of the figure diagram exceeds `fold`. |
| **Display bit order** | The qubit display order determined by `reverse_bits`; when set to `true`, the reverse order is used. |
| **Dataset** | A numeric series in a result plot. The result drawing entry point reduces the counts of one measurement into a single dataset. |
| **Rest bucket** | Aggregating the entries beyond `number_to_keep` in a result plot into an entry labeled `rest`. |

---

## `cqlib_core::visualization` API overview

### Circuit drawing

| Name | Description |
| --- | --- |
| [`circuit_to_text`](1_draw_text.md) / [`TextDrawerOptions`](1_draw_text.md) | The text drawing entry point and drawing options for circuits. |
| [`draw_text_from_visual`](1_draw_text.md) | Draw a text diagram from an already built visual IR. |
| [`circuit_to_figure`](2_draw_figure.md) / [`FigureDrawerOptions`](2_draw_figure.md) | The SVG drawing entry point and drawing options for circuits. |
| [`draw_figure_from_visual`](2_draw_figure.md) | Draw SVG from an already built visual IR. |
| [`FigureDrawStyle`](2_draw_figure.md) / [`GateStyle`](2_draw_figure.md) | Figure style presets and single-gate style overrides. |
| [`ParameterFormatOptions`](2_draw_figure.md) / [`ParameterDisplayMode`](2_draw_figure.md) / [`ParameterFormatter`](2_draw_figure.md) | Formatting options, display modes and formatters for gate parameter text. |

### Visual IR

| Name | Description |
| --- | --- |
| [`build_visual_circuit`](6_visual_ir.md) / [`VisualBuildOptions`](6_visual_ir.md) | The entry point and build options for constructing a circuit into the visual IR. |
| [`VisualCircuit`](6_visual_ir.md) / [`VisualOperation`](6_visual_ir.md) | The circuit and single-operation representation at the visual layer. |
| [`VisualOpStyle`](6_visual_ir.md) / [`VisualChildren`](6_visual_ir.md) | The style classification of an operation and its control flow child nodes. |
| [`VisualControlFlowKind`](6_visual_ir.md) / [`VisualCondition`](6_visual_ir.md) | Control flow forms and their condition representation. |

### Quantum state drawing

| Name | Description |
| --- | --- |
| [`plot_bloch_vector`](3_state_plots.md) | Draw a single Bloch vector. |
| [`plot_bloch_multivector`](3_state_plots.md) | Draw one reduced Bloch vector per qubit. |
| [`plot_state_city`](3_state_plots.md) | Draw the real and imaginary parts of a density matrix. |
| [`plot_state_paulivec`](3_state_plots.md) | Draw a bar chart of Pauli basis expectation values. |
| [`StatePlotOptions`](3_state_plots.md) | Title, color scheme, opacity and bit order options for quantum state drawing. |
| [`StateVisualizationSource`](3_state_plots.md) | The input trait for quantum state drawing, adapting core state objects into density matrix data. |
| [`state_to_density_matrix`](3_state_plots.md) / [`local_bloch_vectors`](3_state_plots.md) | Density matrix normalization and reduced Bloch vector computation. |

### Measurement result drawing

| Name | Description |
| --- | --- |
| [`plot_histogram`](4_result_plots.md) | Draw measurement counts as a histogram. |
| [`plot_distribution`](4_result_plots.md) | Normalize measurement counts into probabilities and draw them. |
| [`ResultPlotOptions`](4_result_plots.md) | Ordering, color scheme, retained entries and layout options for result plots. |

### File output

| Name | Description |
| --- | --- |
| [`render_figure_to_file`](5_render_to_file.md) | Render a circuit directly to a `.svg` or `.png` file. |
| [`render_state_plot_to_file`](5_render_to_file.md) | Write the SVG of a quantum state plot to a file. |
| [`render_result_plot_to_file`](5_render_to_file.md) | Write the SVG of a result plot to a file. |
| [`VisualizationError`](0_overview.md) | The error type shared by all visualization stages. |

---

## Quick examples

### 1. Circuit text diagram

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{TextDrawerOptions, circuit_to_text};

let mut circuit = Circuit::new(3);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
circuit.measure(Qubit::new(0)).unwrap();

let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
assert!(text.contains("Q0:"));
assert!(text.contains("H"));
```

### 2. Circuit SVG diagram

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{FigureDrawerOptions, circuit_to_figure};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let svg = circuit_to_figure(
    &circuit,
    &FigureDrawerOptions {
        initial_state: true,
        ..FigureDrawerOptions::default()
    },
)
.unwrap();
assert!(svg.contains("<svg"));
```

### 3. Measurement result histogram

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{ExecutionResult, Outcome};
use cqlib_core::visualization::{ResultPlotOptions, plot_histogram};
use std::collections::HashMap;

let mut result = ExecutionResult::new(
    "task-visualization".to_string(),
    (0..2).map(|idx| Qubit::new(idx as u32)).collect::<Vec<_>>(),
    7,
    2,
    Some("simulator".to_string()),
    None,
);
let counts = [("00", 2usize), ("11", 5)]
    .into_iter()
    .map(|(bits, count)| (Outcome::from_bitstring(bits).unwrap(), count))
    .collect::<HashMap<_, _>>();
result.finish(counts, None);

let svg = plot_histogram(&result, &ResultPlotOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## Validation and error handling

The entry points of visualization uniformly return `VisualizationError`:

| Error | When it occurs |
| --- | --- |
| `VisualizationError::CircuitBuild` | Preprocessing fails while expanding circuit gate definitions. |
| `VisualizationError::UnknownQubit` | An operation references a qubit that is not in the qubit list of the circuit. |
| `VisualizationError::ParameterIndexOutOfBounds` | The index of a symbolic parameter exceeds the length of the parameter table of the circuit. |
| `VisualizationError::InvalidInput` | Invalid input data: the state buffer length does not match the number of qubits, the result contains no counts, the probability distribution sums to zero, `sort = "hamming"` is missing `target_string`, the legend length does not match the number of datasets, `dpi` is zero, and so on. |
| `VisualizationError::SvgRenderFailed` | SVG parsing or rasterization failure. |
| `VisualizationError::Io` | Writing the output file fails. |

The specific conditions for each entry point are described on the corresponding page.
