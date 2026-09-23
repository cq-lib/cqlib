# Writing to file and output

`cqlib_core::visualization`

This page covers the file output entry points of visualization: `render_figure_to_file` for circuit diagrams, and `render_state_plot_to_file` and `render_result_plot_to_file` for quantum state plots and result plots. The three entry points share the same output contract: the extension determines the written format, and the content is SVG markup.

## Import

```rust
use cqlib_core::visualization::{
    render_figure_to_file, render_result_plot_to_file, render_state_plot_to_file,
};
```

---

## `render_figure_to_file(circuit, output_path, options) -> Result<(), VisualizationError>`

Render a circuit directly to an output file. A visual IR is built and SVG markup generated internally, and then the file is written; calling `circuit_to_figure` first is not necessary.

Parameters:

- `circuit` (`&Circuit`): the circuit to render.
- `output_path` (`&str`): the target file path. A `.png` extension writes a raster image, and any other extension writes an SVG vector image.
- `options` (`&FigureDrawerOptions`): the figure drawing options. `dpi` determines the rasterization scale of the PNG: the default `160` corresponds to a scale of `1.0`, and larger values scale the PNG size proportionally.

Returns:

- `Result<(), VisualizationError>`: no data is returned on success.

Raises:

- `VisualizationError::CircuitBuild`: expanding circuit gate definitions fails when `decompose_circuit_gates` is `true`.
- `VisualizationError::UnknownQubit`: an operation references a qubit that is not in the qubit list of the circuit.
- `VisualizationError::ParameterIndexOutOfBounds`: the index of a symbolic parameter exceeds the length of the parameter table of the circuit.
- `VisualizationError::InvalidInput`: `dpi` is `0`, or the scaled canvas size exceeds the representable range.
- `VisualizationError::SvgRenderFailed`: SVG parsing or rasterization failure.
- `VisualizationError::Io`: writing the file fails.

Example:

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{FigureDrawerOptions, render_figure_to_file};

let mut circuit = Circuit::new(1);
circuit.h(Qubit::new(0)).unwrap();

render_figure_to_file(&circuit, "circuit.svg", &FigureDrawerOptions::default()).unwrap();
render_figure_to_file(&circuit, "circuit.png", &FigureDrawerOptions::default()).unwrap();
```

---

## `render_state_plot_to_file(svg, output_path) -> Result<(), VisualizationError>`

Write the SVG markup produced by the quantum state drawing entry points out to a file.

Parameters:

- `svg` (`&str`): the SVG markup returned by a quantum state drawing entry point.
- `output_path` (`&str`): the target file path. A `.png` extension writes a raster image, and any other extension writes an SVG vector image.

Returns:

- `Result<(), VisualizationError>`: no data is returned on success.

Raises:

- `VisualizationError::InvalidInput`: the canvas size after PNG scaling exceeds the representable range.
- `VisualizationError::SvgRenderFailed`: SVG parsing or rasterization failure.
- `VisualizationError::Io`: writing the file fails.

Example:

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::{
    StatePlotOptions, plot_bloch_multivector, render_state_plot_to_file,
};
use num_complex::Complex64;

let state = Statevector::from_state(1, vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)])
    .unwrap();
let svg = plot_bloch_multivector(&state, &StatePlotOptions::default()).unwrap();
render_state_plot_to_file(&svg, "bloch.svg").unwrap();
```

---

## `render_result_plot_to_file(svg, output_path) -> Result<(), VisualizationError>`

Write the SVG markup produced by the result drawing entry points out to a file.

Parameters:

- `svg` (`&str`): the SVG markup returned by `plot_histogram` or `plot_distribution`.
- `output_path` (`&str`): the target file path. A `.png` extension writes a raster image, and any other extension writes an SVG vector image.

Returns:

- `Result<(), VisualizationError>`: no data is returned on success.

Raises:

- `VisualizationError::InvalidInput`: the canvas size after PNG scaling exceeds the representable range.
- `VisualizationError::SvgRenderFailed`: SVG parsing or rasterization failure.
- `VisualizationError::Io`: writing the file fails.

Example:

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{ExecutionResult, Outcome};
use cqlib_core::visualization::{
    ResultPlotOptions, plot_histogram, render_result_plot_to_file,
};
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
render_result_plot_to_file(&svg, "histogram.svg").unwrap();
```

---

## Output formats

| Extension | Written content |
| --- | --- |
| `.png` (case-insensitive) | A raster image. The SVG is first sized into a bitmap by the scale factor and then encoded as PNG. |
| Other extensions | The SVG markup text is written as-is. |

Rasterization scale: circuit diagrams are scaled by `dpi / 160`, giving a scale of `1.0` at the default `dpi`; quantum state plots and result plots are always written at a scale of `1.0`. Scaling changes only the bitmap size, and the SVG coordinate system stays unchanged.

---

## Related pages

- For circuit drawing options, see [SVG circuit diagrams](2_draw_figure.md).
- For quantum state drawing entry points, see [Quantum state visualization](3_state_plots.md).
- For result drawing entry points, see [Measurement result visualization](4_result_plots.md).
