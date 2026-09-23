# SVG Circuit Diagrams

`cqlib_core::visualization::circuit::figure`

This page covers the figure backend for circuits: the drawing entry point `circuit_to_figure`, the entry point based on the visual IR `draw_figure_from_visual`, the drawing options `FigureDrawerOptions`, and the style and parameter formatting types referenced by those options. The figure backend renders a circuit into an SVG markup string, with a white canvas background.

## Import

```rust
use cqlib_core::visualization::{FigureDrawerOptions, circuit_to_figure};
```

---

## `circuit_to_figure(circuit, options) -> Result<String, VisualizationError>`

Generate the SVG markup of a circuit. A visual IR is built internally first and then handed to the figure backend for drawing.

Parameters:

- `circuit` (`&Circuit`): the circuit to draw.
- `options` (`&FigureDrawerOptions`): the figure drawing options.

Returns:

- `Result<String, VisualizationError>`: on success, an SVG markup string.

Raises:

- `VisualizationError::CircuitBuild`: expanding circuit gate definitions fails when `decompose_circuit_gates` is `true`.
- `VisualizationError::UnknownQubit`: an operation references a qubit that is not in the qubit list of the circuit.
- `VisualizationError::ParameterIndexOutOfBounds`: the index of a symbolic parameter exceeds the length of the parameter table of the circuit.

Example:

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{FigureDrawerOptions, circuit_to_figure};

let mut circuit = Circuit::new(1);
circuit.h(Qubit::new(0)).unwrap();

let svg = circuit_to_figure(&circuit, &FigureDrawerOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## FigureDrawerOptions

Figure drawing options.

```rust
pub struct FigureDrawerOptions {
    pub show_params: bool,
    pub decompose_circuit_gates: bool,
    pub parameter_format: ParameterFormatOptions,
    pub width_per_column: f64,
    pub height_per_qubit: f64,
    pub dpi: u32,
    pub gate_width: f64,
    pub gate_height: f64,
    pub moment_spacing: f64,
    pub connect_height: f64,
    pub fold: i32,
    pub style: FigureDrawStyle,
    pub gate_styles: HashMap<String, GateStyle>,
    pub initial_state: bool,
    pub reverse_bits: bool,
}
```

Fields:

- `show_params` (`bool`): whether to append parameter text after the gate label. The default is `true`.
- `decompose_circuit_gates` (`bool`): whether to expand circuit gate definitions before drawing. The default is `false`.
- `parameter_format` (`ParameterFormatOptions`): the formatting options of gate parameter text. The default is `ParameterFormatOptions::default()`.
- `width_per_column` (`f64`): the figure width scaling factor per logical column. The default is `1.2`.
- `height_per_qubit` (`f64`): the figure height scaling factor per qubit. The default is `0.9`.
- `dpi` (`u32`): the rasterization DPI when exporting PNG. The default is `160`, at which the PNG scale is `1.0` and the size matches the native SVG pixels; larger values scale the PNG size proportionally, and the SVG output is unaffected. Drawing fails when the value is `0`.
- `gate_width` (`f64`): the base gate width (in data units), which also serves as the lower bound of the column width. The default is `1.1`.
- `gate_height` (`f64`): the base gate height (in data units). The default is `1.5`.
- `moment_spacing` (`f64`): the horizontal spacing between adjacent columns. The default is `0.3`.
- `connect_height` (`f64`): the vertical spacing between rows after column folding. The default is `2.0`.
- `fold` (`i32`): the maximum number of columns per row. The default is `18`; a negative value or `0` disables column folding.
- `style` (`FigureDrawStyle`): the style preset. The default is `FigureDrawStyle::Cqlib`.
- `gate_styles` (`HashMap<String, GateStyle>`): style overrides by gate name, merged on top of the base style table. The default is an empty map.
- `initial_state` (`bool`): whether to display `|0>` in the qubit labels. The default is `false`.
- `reverse_bits` (`bool`): whether to reverse the display order of the qubits. The default is `false`.

The defaults are provided by `FigureDrawerOptions::default()`:

```rust
use cqlib_core::visualization::FigureDrawerOptions;

let options = FigureDrawerOptions {
    fold: 8,
    ..FigureDrawerOptions::default()
};
```

---

## FigureDrawStyle

Figure style preset.

```rust
pub enum FigureDrawStyle {
    /// Cqlib 默认样式。
    Cqlib,
}
```

`FigureDrawStyle::Cqlib` determines the default colors and line widths of circuit wires, connection lines, gate borders, gate fills and labels.

---

## GateStyle

The style override item of a single gate. All fields are optional; when a field is `None`, the base style applies.

```rust
pub struct GateStyle {
    pub border_color: Option<String>,
    pub background_color: Option<String>,
    pub font_size: Option<f64>,
    pub text_color: Option<String>,
    pub line_color: Option<String>,
    pub line_width: Option<f64>,
}
```

Fields:

- `border_color` (`Option<String>`): the gate border color. The default is `None`.
- `background_color` (`Option<String>`): the gate fill color. The default is `None`.
- `font_size` (`Option<f64>`): the gate label font size, which also participates in gate width measurement. The default is `None`.
- `text_color` (`Option<String>`): the gate label color. The default is `None`.
- `line_color` (`Option<String>`): the connection line color of the gate. The default is `None`.
- `line_width` (`Option<f64>`): the line width of the gate. The default is `None`.

Writing an entry keyed by gate name into `FigureDrawerOptions::gate_styles` overrides the style of that gate:

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
use cqlib_core::visualization::{FigureDrawerOptions, GateStyle, circuit_to_figure};
use std::collections::HashMap;

let mut circuit = Circuit::new(1);
let long_param = Parameter::symbol("styledlongparametername");
circuit.rz(Qubit::new(0), long_param).unwrap();

let mut gate_styles = HashMap::new();
gate_styles.insert(
    "RZ".to_string(),
    GateStyle {
        font_size: Some(16.0),
        ..GateStyle::default()
    },
);

let svg = circuit_to_figure(
    &circuit,
    &FigureDrawerOptions {
        gate_styles,
        ..FigureDrawerOptions::default()
    },
)
.unwrap();
assert!(svg.contains("<svg"));
```

---

## Parameter formatting

`FigureDrawerOptions::parameter_format` controls the textual form of gate parameters in labels.

### ParameterDisplayMode

```rust
pub enum ParameterDisplayMode {
    /// 可求值时优先显示数值。
    Numeric,
    /// 优先显示符号表达式。
    Symbolic,
    /// 显示符号表达式，并在可求值时追加数值。
    SymbolicWithValue,
    /// 对接近常见角度的取值优先使用 `kπ/n` 形式。
    PiFractionPreferred,
}
```

### ParameterFormatOptions

```rust
pub struct ParameterFormatOptions {
    pub mode: ParameterDisplayMode,
    pub decimal_precision: usize,
    pub scientific_lower_bound: f64,
    pub scientific_upper_bound: f64,
    pub pi_tolerance: f64,
    pub pi_max_denominator: i64,
}
```

Fields:

- `mode` (`ParameterDisplayMode`): the display mode. The default is `ParameterDisplayMode::Numeric`.
- `decimal_precision` (`usize`): the decimal precision of fixed-point and scientific notation. The default is `2`.
- `scientific_lower_bound` (`f64`): values whose absolute value falls within `(0, scientific_lower_bound)` use scientific notation. The default is `1e-3`.
- `scientific_upper_bound` (`f64`): values whose absolute value is not less than this use scientific notation. The default is `1e4`.
- `pi_tolerance` (`f64`): the tolerance when matching the ratio of a value to π to a rational fraction. The default is `1e-3`.
- `pi_max_denominator` (`i64`): the maximum denominator used for π fraction matching. The default is `16`.

In numeric mode, `1.2345` is displayed as `1.23`; values beyond the scientific notation thresholds are displayed in forms such as `4e-4` and `1.5e4`. In `PiFractionPreferred` mode, `π/2` is displayed as `π/2`.

```rust
use cqlib_core::visualization::{FigureDrawerOptions, ParameterDisplayMode, ParameterFormatOptions};

let options = FigureDrawerOptions {
    parameter_format: ParameterFormatOptions {
        mode: ParameterDisplayMode::PiFractionPreferred,
        ..ParameterFormatOptions::default()
    },
    ..FigureDrawerOptions::default()
};
```

### ParameterFormatter

A formatter that formats a single parameter entry into a display label; the visual IR build uses it to write parameters into the label of each operation.

```rust
pub struct ParameterFormatter {
    options: ParameterFormatOptions,
}
```

Methods:

- `ParameterFormatter::new(options: ParameterFormatOptions) -> Self`: construct a formatter with the given options.
- `fn format_circuit_param(&self, circuit: &Circuit, param: &CircuitParam) -> Result<String, VisualizationError>`: format one parameter entry. `CircuitParam::Fixed` uses its numeric value directly; `CircuitParam::Index` is resolved into a symbolic expression through the parameter table of the circuit.

Raises:

- `VisualizationError::ParameterIndexOutOfBounds`: the index of `CircuitParam::Index` exceeds the length of the parameter table of the circuit.

```rust
use cqlib_core::circuit::{Circuit, CircuitParam};
use cqlib_core::visualization::{ParameterFormatOptions, ParameterFormatter};

let formatter = ParameterFormatter::new(ParameterFormatOptions::default());
let circuit = Circuit::new(0);

assert_eq!(
    formatter
        .format_circuit_param(&circuit, &CircuitParam::Fixed(1.2345))
        .unwrap(),
    "1.23"
);
```

---

## `draw_figure_from_visual(visual, options, output_path) -> String`

Draw SVG from an already built visual IR. Use this entry point when a cached IR is available, to avoid rebuilding it.

Parameters:

- `visual` (`&VisualCircuit`): the visual IR built by `build_visual_circuit`.
- `options` (`&FigureDrawerOptions`): the figure drawing options.
- `output_path` (`Option<&str>`): a reserved parameter, currently ignored.

Returns:

- `String`: an SVG markup string.

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{
    FigureDrawerOptions, VisualBuildOptions, build_visual_circuit, draw_figure_from_visual,
};

let mut circuit = Circuit::new(1);
circuit.h(Qubit::new(0)).unwrap();
let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
let svg = draw_figure_from_visual(&visual, &FigureDrawerOptions::default(), None);
assert!(svg.contains("<svg"));
```

---

## Output forms

### Column folding

`fold` limits the number of columns per row, and columns beyond the limit are moved to subsequent rows. Each row after column folding keeps complete qubit wires, and the spacing between rows is controlled by `connect_height`:

```rust
let svg = circuit_to_figure(
    &circuit,
    &FigureDrawerOptions {
        fold: 4,
        ..FigureDrawerOptions::default()
    },
)
.unwrap();
```

### Expanding circuit gates and displaying the initial state

When `decompose_circuit_gates` is `false`, a circuit gate is drawn as a single labeled gate block; when set to `true`, it is first expanded into its definition circuit and then drawn. When `initial_state` is `true`, the initial state is marked next to the qubit labels.

---

## Related pages

- For the drawing entry points and options of the text backend, see [Text circuit diagrams](1_draw_text.md).
- To write SVG out to a file, see [Writing to file and output](5_render_to_file.md).
