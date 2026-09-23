# Text Circuit Diagrams

`cqlib_core::visualization::circuit::text`

This page covers the text backend for circuits: the drawing entry point `circuit_to_text`, the entry point based on the visual IR `draw_text_from_visual`, and the drawing options `TextDrawerOptions`. The text backend renders a circuit into a Unicode box-drawing character diagram, with operations in the same column displayed aligned.

## Import

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{TextDrawerOptions, circuit_to_text};
```

---

## `circuit_to_text(circuit, options) -> Result<String, VisualizationError>`

Draw a circuit as a Unicode box-drawing character diagram. A visual IR is built internally first and then handed to the text backend for drawing; control flow blocks are expanded into block markers arranged in columns.

Parameters:

- `circuit` (`&Circuit`): the circuit to draw.
- `options` (`&TextDrawerOptions`): the text drawing options.

Returns:

- `Result<String, VisualizationError>`: on success, a UTF-8 text diagram string; returns `"empty circuit"` when the number of qubits in the IR is zero.

Raises:

- `VisualizationError::CircuitBuild`: expanding circuit gate definitions fails when `decompose_circuit_gates` is `true`.
- `VisualizationError::UnknownQubit`: an operation references a qubit that is not in the qubit list of the circuit.
- `VisualizationError::ParameterIndexOutOfBounds`: the index of a symbolic parameter exceeds the length of the parameter table of the circuit.

Example:

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{TextDrawerOptions, circuit_to_text};

let mut circuit = Circuit::new(3);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
circuit.measure(Qubit::new(0)).unwrap();
circuit.measure(Qubit::new(1)).unwrap();
circuit.measure(Qubit::new(2)).unwrap();

let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
assert!(text.contains("Q0:"));
```

---

## TextDrawerOptions

Text drawing options.

```rust
pub struct TextDrawerOptions {
    pub cell_width: usize,
    pub show_params: bool,
    pub decompose_circuit_gates: bool,
    pub line_width: isize,
    pub initial_state: bool,
    pub reverse_bits: bool,
}
```

Fields:

- `cell_width` (`usize`): a reserved field; the rendering width is currently computed dynamically from the content. The default is `9`.
- `show_params` (`bool`): whether to append parameter text after the gate label. The default is `true`.
- `decompose_circuit_gates` (`bool`): whether to expand circuit gate definitions before drawing. The default is `false`.
- `line_width` (`isize`): the maximum fold width. A value greater than `10` folds at that value; a negative value disables folding; a value between `0` and `10` falls back to the default width. The default is `80`.
- `initial_state` (`bool`): whether to display `|0>` at the start of each qubit wire. The default is `false`.
- `reverse_bits` (`bool`): whether to reverse the display order of the qubits. The default is `false`.

The defaults are provided by `TextDrawerOptions::default()`, and the struct update syntax can be used to override only the fields that need to change:

```rust
use cqlib_core::visualization::TextDrawerOptions;

let options = TextDrawerOptions {
    line_width: 12,
    ..TextDrawerOptions::default()
};
```

---

## `draw_text_from_visual(visual, options) -> Result<String, VisualizationError>`

Draw a text diagram from an already built visual IR. Use this entry point when a cached IR is available, to avoid rebuilding it.

Parameters:

- `visual` (`&VisualCircuit`): the visual IR built by `build_visual_circuit`.
- `options` (`&TextDrawerOptions`): the text drawing options.

Returns:

- `Result<String, VisualizationError>`: on success, a UTF-8 text diagram string; returns `"empty circuit"` when the number of qubits in the IR is zero.

Raises:

- The current implementation only returns `Ok`; the `Result` return type is kept for symmetry with `circuit_to_text`.

Example:

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{
    TextDrawerOptions, VisualBuildOptions, build_visual_circuit, draw_text_from_visual,
};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let options = TextDrawerOptions::default();
let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
let text = draw_text_from_visual(&visual, &options).unwrap();
assert!(text.contains("H"));
```

---

## Output forms

### Basic circuit

```rust
let mut circuit = Circuit::new(3);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
circuit.measure(Qubit::new(0)).unwrap();
circuit.measure(Qubit::new(1)).unwrap();
circuit.measure(Qubit::new(2)).unwrap();
```

Output under the default options:

```text
                
 Q0: ───H──■──M─
           │
 Q1: ──────┼──M─
           │
 Q2: ──────X──M─
                
```

Each wire corresponds to one qubit, and between two qubit wires there is a connection column: a controlled gate marks the control position with `■` and connects the target position with `│` and `X`. Instructions such as measurement, reset and delay appear in the corresponding column as an operation name or symbol.

### Fold

When `line_width` is smaller than the natural width of the diagram, the diagram is split into multiple segments:

```rust
let mut circuit = Circuit::new(1);
for _ in 0..10 {
    circuit.h(Qubit::new(0)).unwrap();
}
let options = TextDrawerOptions {
    line_width: 12,
    ..TextDrawerOptions::default()
};
let text = circuit_to_text(&circuit, &options).unwrap();
```

In the output, segments are separated by blank lines, and the end and the start of a segment are marked with `»` and `«` respectively to indicate the continuation direction:

```text
                   »
 Q0: ───H──H──H──H─»
                   »

«                   »
« Q0: ───H──H──H──H─»
«                   »

«
« Q0: ───H──H─
«
```

When `line_width` is negative, no folding occurs and neither `«` nor `»` appears in the output.

### Displaying the initial state and reversing the bit order

When `initial_state` is `true`, `|0>` is displayed at the start of each qubit wire:

```text
             
 Q0: |0>───H─
             
```

When `reverse_bits` is `true`, qubits are displayed in reverse order:

```rust
let mut circuit = Circuit::new(2);
circuit.x(Qubit::new(0)).unwrap();
let options = TextDrawerOptions {
    reverse_bits: true,
    ..TextDrawerOptions::default()
};
let text = circuit_to_text(&circuit, &options).unwrap();
```

```text
          
 Q1: ─────
          
 Q0: ───X─
          
```

### Parameters and circuit gates

When `show_params` is `false`, the gate label shows only the gate name, without parameter text:

```text
          
 Q0: ───RX─
          
```

When `decompose_circuit_gates` is `false`, circuit gates are displayed as a whole as a bordered block; when set to `true`, the gate definitions are expanded first and drawing follows the expanded operations.

---

## Related pages

- For the drawing entry points and options of the figure backend, see [SVG circuit diagrams](2_draw_figure.md).
- For the formatting rules of parameter text, see the parameter formatting section in [SVG circuit diagrams](2_draw_figure.md).
