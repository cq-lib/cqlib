# Visual IR

`cqlib_core::visualization`

This page covers the intermediate representation layer of circuit drawing: the build entry point `build_visual_circuit`, the build options `VisualBuildOptions`, and the IR data types `VisualCircuit`, `VisualOperation`, `VisualOpStyle`, `VisualChildren`, `VisualControlFlowKind` and `VisualCondition`.

## Import

```rust
use cqlib_core::visualization::{
    VisualBuildOptions, VisualChildren, VisualCircuit, VisualCondition, VisualControlFlowKind,
    VisualOpStyle, VisualOperation, build_visual_circuit,
};
```

---

## Position in the pipeline

Circuit drawing has two steps: first build the circuit into a visual IR independent of the drawing backend, then draw it with a concrete backend.

```text
Circuit ──build_visual_circuit──▶ VisualCircuit ──┬── circuit_to_text / draw_text_from_visual ──▶ 文本图
                                                  └── circuit_to_figure / draw_figure_from_visual ──▶ SVG 图
```

The build stage carries the work related to circuit semantics: mapping qubits to indices, formatting parameters into label text, classifying instructions into drawing styles, and placing operations into non-overlapping columns. The drawing stage only consumes these results and does not read the circuit back. Control flow blocks are recursively collected as sub-circuits during the build stage and expanded into individual markers by the drawing backend.

The intermediate layer decouples building from drawing, with three consequences:

- The text backend and the figure backend consume the same IR, so the gate order, column partitioning and labels of a given circuit are identical under the two backends.
- A custom drawing only needs to consume `VisualCircuit` and does not have to reimplement qubit mapping, parameter formatting and column scheduling.
- Build options can be adjusted independently of drawing options. Entering the text backend through `circuit_to_text` uses the default parameter formatting options; to have the text diagram adopt another parameter display mode, build the IR first and hand it to `draw_text_from_visual`.

---

## `build_visual_circuit(circuit, options) -> Result<VisualCircuit, VisualizationError>`

Build a circuit into the visual IR.

Parameters:

- `circuit` (`&Circuit`): the circuit to build.
- `options` (`&VisualBuildOptions`): the build options.

Returns:

- `Result<VisualCircuit, VisualizationError>`: on success, the visual IR, whose number of qubits equals the number of qubits of the circuit.

Raises:

- `VisualizationError::CircuitBuild`: expanding circuit gate definitions fails when `decompose_circuit_gates` is `true`.
- `VisualizationError::UnknownQubit`: an operation references a qubit that is not in the qubit list of the circuit.
- `VisualizationError::ParameterIndexOutOfBounds`: the index of a symbolic parameter exceeds the length of the parameter table of the circuit.

Example:

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, build_visual_circuit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
assert_eq!(visual.num_qubits(), 2);
assert_eq!(visual.operations.len(), 2);
assert_eq!(visual.operations[0].label, "H");
assert_eq!(visual.operations[1].lanes, vec![0, 1]);
assert_eq!(visual.num_columns, 2);
```

---

## VisualBuildOptions

Build options of the visual IR. These options affect only the build stage and are shared by the text backend and the figure backend.

```rust
pub struct VisualBuildOptions {
    pub decompose_circuit_gates: bool,
    pub reserve_full_span_for_multi_qubit: bool,
    pub parameter_format: ParameterFormatOptions,
}
```

Fields:

- `decompose_circuit_gates` (`bool`): whether to expand circuit gate definitions before building. The default is `false`.
- `reserve_full_span_for_multi_qubit` (`bool`): reserve the whole index span (`min..=max`) for operations involving multiple qubits instead of reserving only the indices of the operation itself. The default is `true`.
- `parameter_format` (`ParameterFormatOptions`): the formatting options of parameter label text. The default is `ParameterFormatOptions::default()`, that is, `ParameterDisplayMode::Numeric`.

The defaults are provided by `VisualBuildOptions::default()`, and the struct update syntax can be used to override only the fields that need to change:

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, build_visual_circuit};

let mut sub = Circuit::new(2);
sub.h(Qubit::new(0)).unwrap();
sub.cx(Qubit::new(0), Qubit::new(1)).unwrap();
let sub_gate = sub.to_gate("SUB_BELL").unwrap();

let mut circuit = Circuit::new(2);
circuit
    .append(sub_gate, vec![Qubit::new(0), Qubit::new(1)], Vec::new(), None)
    .unwrap();

let visual = build_visual_circuit(
    &circuit,
    &VisualBuildOptions {
        decompose_circuit_gates: true,
        ..VisualBuildOptions::default()
    },
)
.unwrap();
assert_eq!(visual.operations[0].label, "H");
```

`parameter_format` is a nested struct, updated field by field:

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{
    ParameterDisplayMode, ParameterFormatOptions, VisualBuildOptions, build_visual_circuit,
};
use std::f64::consts::PI;

let mut circuit = Circuit::new(1);
circuit.rx(Qubit::new(0), PI / 2.0).unwrap();

let options = VisualBuildOptions {
    parameter_format: ParameterFormatOptions {
        mode: ParameterDisplayMode::PiFractionPreferred,
        ..ParameterFormatOptions::default()
    },
    ..VisualBuildOptions::default()
};
let visual = build_visual_circuit(&circuit, &options).unwrap();
assert_eq!(visual.operations[0].params, vec!["π/2".to_string()]);
```

---

## VisualCircuit

The visual IR after layout is complete, independent of the drawing backend.

```rust
pub struct VisualCircuit {
    pub qubits: Vec<Qubit>,
    pub operations: Vec<VisualOperation>,
    pub num_columns: usize,
}
```

Fields:

- `qubits` (`Vec<Qubit>`): the qubits in display order; the `lanes` of an operation are indices into this list.
- `operations` (`Vec<VisualOperation>`): the operations in circuit order, already placed into columns.
- `num_columns` (`usize`): the number of occupied columns.

Methods:

- `fn num_qubits(&self) -> usize`: return the number of qubits.

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::visualization::{VisualCircuit, VisualOpStyle, VisualOperation};

let visual = VisualCircuit {
    qubits: vec![Qubit::new(0)],
    operations: vec![VisualOperation {
        column: 0,
        lanes: vec![0],
        covered_lanes: vec![0],
        label: "H".to_string(),
        params: vec![],
        style: VisualOpStyle::Gate,
        span_box: false,
        children: None,
        span_cols: 1,
    }],
    num_columns: 1,
};
assert_eq!(visual.num_qubits(), 1);
```

---

## VisualOperation

The atomic IR node consumed by the drawing backend. One operation corresponds to one piece of drawing content in a column.

```rust
pub struct VisualOperation {
    pub column: usize,
    pub lanes: Vec<usize>,
    pub covered_lanes: Vec<usize>,
    pub label: String,
    pub params: Vec<String>,
    pub style: VisualOpStyle,
    pub span_box: bool,
    pub children: Option<VisualChildren>,
    pub span_cols: usize,
}
```

Fields:

- `column` (`usize`): the column index it is placed into.
- `lanes` (`Vec<usize>)`: the indices of the qubits involved in the operation within `VisualCircuit::qubits`, ordered by the operands of the operation.
- `covered_lanes` (`Vec<usize>`): the qubit indices reserved in this column to avoid overlap. A control flow block reserves all indices; a barrier reserves all indices when `lanes` is empty; any other operation reserves `[0]` when `lanes` is empty. When more than one index is involved, if `reserve_full_span_for_multi_qubit` is `true` the whole `min..=max` span of `lanes` is reserved, otherwise `lanes` itself is reserved. It is empty when the number of qubits is zero.
- `label` (`String`): the main display label.
- `params` (`Vec<String>`): the formatted parameter labels.
- `style` (`VisualOpStyle`): the drawing style.
- `span_box` (`bool`): when `true`, the operation is drawn as a whole block spanning multiple qubit wires. Custom unitary gates, circuit gates and store instructions take `true`.
- `children` (`Option<VisualChildren>`): the sub-circuit of a control flow operation; `None` for non-control-flow operations.
- `span_cols` (`usize`): the number of logical columns reserved by the operation. It is `1` for ordinary operations; for a control flow block it is computed from the number of columns of the sub-circuit plus separators and margins.

```rust
use cqlib_core::visualization::{VisualOpStyle, VisualOperation};

let op = VisualOperation {
    column: 1,
    lanes: vec![0, 1],
    covered_lanes: vec![0, 1],
    label: "X".to_string(),
    params: vec!["π/2".to_string()],
    style: VisualOpStyle::Controlled { num_controls: 1 },
    span_box: false,
    children: None,
    span_cols: 1,
};
assert_eq!(op.label, "X");
```

---

## VisualOpStyle

The drawing style of a single operation. Each variant determines how the backend draws the operation.

```rust
pub enum VisualOpStyle {
    /// 普通门块。
    Gate,
    /// 受控操作，操作数开头的 `num_controls` 个量子比特为控制位。
    Controlled {
        num_controls: usize,
    },
    /// 受控 Z 标记，按两个以竖线相连的圆点绘制。
    Cz,
    /// 两个量子比特间的交换标记。
    Swap,
    /// 屏障标记。
    Barrier,
    /// 测量标记。
    Measure,
    /// 复位标记。
    Reset,
    /// 延时标记。
    Delay,
    /// 控制流标记。
    ControlFlow {
        kind: VisualControlFlowKind,
    },
}
```

### Classification rules of styles and labels

The build stage classifies instructions into styles and labels according to the following table:

| Instruction | `style` | `label` |
| --- | --- | --- |
| Standard gate `SWAP` | `Swap` | `SWAP` |
| Standard gate `CZ` | `Cz` | `CZ` |
| Standard gate with more than 0 controls | `Controlled { num_controls }` | The target gate name |
| Other standard gates | `Gate` | The gate name |
| Multi-controlled gate with `1` control and base gate `Z` | `Cz` | `CZ` |
| Other multi-controlled gates | `Controlled { num_controls }` | The base gate name |
| Custom unitary gate | `Gate`, `span_box` is `true` | `UnitaryGate::label()`, or `Unitary` when empty |
| Circuit gate | `Gate`, `span_box` is `true` | `CircuitGate::name()`, or `Gate` when empty |
| Barrier | `Barrier` | `B` |
| Measurement | `Measure` | `M` |
| Reset | `Reset` | `R` |
| Delay | `Delay` | `D` |
| Classical control flow | `ControlFlow { kind }` | `IF`, `WH`, `FOR`, `SW`, `Break` or `Continue` |
| Measure-to-bit instruction | `Measure` | `M` |
| Store instruction | `Gate`, `span_box` is `true` | `STORE` |

Gate names are abbreviated according to the standard gate convention: `SDG` is displayed as `SD`, `TDG` as `TD`, and `Phase` and `GPhase` as `P`. The label of a controlled gate takes its target gate name: `CX` and `CCX` are `X`, `CY` is `Y`, `CRX`, `CRY` and `CRZ` are `RX`, `RY` and `RZ` respectively, and in all other cases the control prefix at the start of the gate name is removed.

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, VisualOpStyle, build_visual_circuit};

let mut circuit = Circuit::new(3);
circuit.ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2)).unwrap();
circuit.swap(Qubit::new(0), Qubit::new(1)).unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
assert_eq!(visual.operations[0].label, "X");
assert!(matches!(
    visual.operations[0].style,
    VisualOpStyle::Controlled { num_controls: 2 }
));
assert!(matches!(visual.operations[1].style, VisualOpStyle::Swap));
```

---

## VisualChildren

The sub-circuits attached to a control flow operation. A sub-circuit keeps the qubit list of its parent, so when a branch body or loop body touches only some of the qubits, the drawing still aligns with the surrounding circuit.

```rust
pub enum VisualChildren {
    IfElse {
        then_circuit: Box<VisualCircuit>,
        else_circuit: Option<Box<VisualCircuit>>,
    },
    While {
        body_circuit: Box<VisualCircuit>,
    },
    For {
        body_circuit: Box<VisualCircuit>,
    },
    Switch {
        case_circuits: Vec<(String, Box<VisualCircuit>)>,
        default_circuit: Option<Box<VisualCircuit>>,
    },
}
```

Variants:

- `IfElse`: conditional branching. `then_circuit` is the sub-circuit executed when the condition holds, and `else_circuit` is the sub-circuit executed when the condition does not hold, being `None` when there is no false branch.
- `While`: the body of a `while` loop. `body_circuit` is the sub-circuit executed inside the loop.
- `For`: the body of a `for` loop. `body_circuit` is the sub-circuit executed inside the loop.
- `Switch`: multi-way selection. `case_circuits` gives the label and sub-circuit of each branch in source order, and `default_circuit` is the sub-circuit of the default branch, being `None` when there is no default branch.

```rust
use cqlib_core::circuit::{Circuit, ClassicalExpr, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, VisualChildren, build_visual_circuit};

let mut circuit = Circuit::new(2);
let condition = ClassicalExpr::bool_literal(true);
circuit
    .if_else(condition, |body| body.x(Qubit::new(1)), |body| body.z(Qubit::new(1)))
    .unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
let op = &visual.operations[0];
assert_eq!(op.label, "IF true");

match op.children.as_ref() {
    Some(VisualChildren::IfElse {
        then_circuit,
        else_circuit,
    }) => {
        assert_eq!(then_circuit.operations[0].label, "X");
        assert_eq!(else_circuit.as_ref().unwrap().operations[0].label, "Z");
    }
    _ => panic!("expected IfElse children"),
}
```

---

## VisualControlFlowKind

The control flow marker family used by the drawing backend.

```rust
pub enum VisualControlFlowKind {
    IfElseBlock {
        has_false_branch: bool,
        condition: VisualCondition,
    },
    WhileBlock {
        condition: VisualCondition,
    },
    ForBlock {
        range: VisualCondition,
    },
    SwitchBlock {
        target: VisualCondition,
    },
    Break,
    Continue,
    IfStart,
    ElseStart,
    WhileStart,
    ForStart,
    SwitchStart,
    CaseStart,
    DefaultStart,
    End,
}
```

The variants fall into two groups.

Source block variants, produced by the build stage:

- `IfElseBlock`: the conditional branch block. `has_false_branch` indicates whether the source block has a false branch, and `condition` is the display metadata of the branch condition.
- `WhileBlock`: the `while` loop block. `condition` is the display metadata of the loop condition.
- `ForBlock`: the `for` loop block. `range` is the display metadata of the loop range expression.
- `SwitchBlock`: the multi-way selection block. `target` is the display metadata of the branch target expression.
- `Break` and `Continue`: structured `break` and `continue` markers.

Expanded marker variants, produced by the drawing backend when it expands a source block before drawing:

- `IfStart`, `ElseStart`: the marker for the start of a conditional branch and for the start of its false branch.
- `WhileStart`, `ForStart`: loop start markers.
- `SwitchStart`, `CaseStart`, `DefaultStart`: the start, branch and default branch markers of a multi-way selection.
- `End`: the block end marker.

Before drawing, both the text backend and the figure backend first expand source block variants into individual markers and then draw them one by one. Marker labels take forms such as `If-0 true`, `Else-0` and `End-0`, with the index incrementing within the same diagram. The expanded marker variants therefore do not appear in the return value of `build_visual_circuit`; they describe an intermediate form of the backend drawing stage.

```rust
use cqlib_core::circuit::{Circuit, ClassicalExpr, Qubit};
use cqlib_core::visualization::{
    VisualBuildOptions, VisualChildren, VisualControlFlowKind, VisualOpStyle,
    build_visual_circuit,
};

let mut circuit = Circuit::new(2);
let condition = ClassicalExpr::bool_literal(false);
circuit
    .while_(condition, |body| {
        body.h(Qubit::new(0))?;
        body.cx(Qubit::new(0), Qubit::new(1))
    })
    .unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
let op = &visual.operations[0];
assert!(op.label.starts_with("WH false"));
assert!(matches!(
    op.style,
    VisualOpStyle::ControlFlow {
        kind: VisualControlFlowKind::WhileBlock { .. }
    }
));

match op.children.as_ref() {
    Some(VisualChildren::While { body_circuit }) => {
        assert_eq!(body_circuit.operations.len(), 2);
    }
    _ => panic!("expected While children"),
}
```

A `for` loop and a multi-way selection are likewise recorded as sub-circuits, and the operation sequence of a sub-circuit follows the source order:

```rust
use cqlib_core::circuit::{Circuit, ClassicalExpr, ClassicalType, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, VisualChildren, build_visual_circuit};

let mut circuit = Circuit::new(2);
let loop_var = circuit.var(ClassicalType::uint(3).unwrap());
circuit
    .for_uint(
        loop_var,
        ClassicalExpr::uint_literal(3, 0).unwrap(),
        ClassicalExpr::uint_literal(3, 4).unwrap(),
        ClassicalExpr::uint_literal(3, 1).unwrap(),
        |body, _| body.cx(Qubit::new(0), Qubit::new(1)),
    )
    .unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
assert!(visual.operations[0].label.starts_with("FOR"));
match visual.operations[0].children.as_ref() {
    Some(VisualChildren::For { body_circuit }) => {
        assert_eq!(body_circuit.operations[0].label, "X");
    }
    _ => panic!("expected For children"),
}
```

---

## VisualCondition

The condition display metadata used by control flow visualization.

```rust
pub struct VisualCondition {
    pub label: String,
}
```

Fields:

- `label` (`String`): the display label of the branch condition.

The label text is generated from the classical expression of the condition: a boolean literal is displayed as `true` or `false`, a bit literal as `bit(v)`, an unsigned integer literal shows its numeric value, a bit vector literal as `bits(v)`, and any other expression form falls back to its `Debug` text.

```rust
use cqlib_core::visualization::VisualCondition;

let condition = VisualCondition {
    label: "true".to_string(),
};
assert_eq!(condition.label, "true");
```

---

## Typical usage

### Inspecting circuit structure and column partitioning

Operations sharing the same qubit must be placed in adjacent columns, while operations on different qubits can share one column:

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, build_visual_circuit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.x(Qubit::new(1)).unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
let cols: Vec<usize> = visual.operations.iter().map(|op| op.column).collect();
assert_eq!(cols, vec![0, 0]);
```

### Inspecting operation styles and lane reservation

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{VisualBuildOptions, VisualOpStyle, build_visual_circuit};

let mut circuit = Circuit::new(3);
circuit.ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2)).unwrap();

let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
let op = &visual.operations[0];
assert_eq!(op.label, "X");
assert!(matches!(op.style, VisualOpStyle::Controlled { num_controls: 2 }));
assert_eq!(op.covered_lanes, vec![0, 1, 2]);
```

### Constructing an IR by hand and rendering it

A hand-constructed `VisualCircuit` can be handed directly to a drawing entry point. The following IR describes a circuit containing only a single `H` gate:

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::visualization::{
    TextDrawerOptions, VisualCircuit, VisualOpStyle, VisualOperation, draw_text_from_visual,
};

let visual = VisualCircuit {
    qubits: vec![Qubit::new(0)],
    operations: vec![VisualOperation {
        column: 0,
        lanes: vec![0],
        covered_lanes: vec![0],
        label: "H".to_string(),
        params: vec![],
        style: VisualOpStyle::Gate,
        span_box: false,
        children: None,
        span_cols: 1,
    }],
    num_columns: 1,
};

let text = draw_text_from_visual(&visual, &TextDrawerOptions::default()).unwrap();
assert!(text.contains("H"));
```

The drawing result is:

```text
          
 Q0: ───H─
          
```

### Rendering with custom build options

Build options act only in the build stage, so the drawing text of the same circuit can change with the build options:

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
use cqlib_core::visualization::{
    FigureDrawerOptions, ParameterDisplayMode, TextDrawerOptions, VisualBuildOptions,
    build_visual_circuit, draw_figure_from_visual, draw_text_from_visual,
};

let mut circuit = Circuit::new(1);
let theta = Parameter::symbol("theta");
circuit.rx(Qubit::new(0), theta + 1.0).unwrap();

let mut build_options = VisualBuildOptions::default();
build_options.parameter_format.mode = ParameterDisplayMode::SymbolicWithValue;
let visual = build_visual_circuit(&circuit, &build_options).unwrap();

let text = draw_text_from_visual(&visual, &TextDrawerOptions::default()).unwrap();
assert!(text.contains("1 + theta"));

let svg = draw_figure_from_visual(&visual, &FigureDrawerOptions::default(), None);
assert!(svg.contains("<svg"));
```

---

## Related pages

- For the drawing entry points and drawing options of the text backend, see [Text circuit diagrams](1_draw_text.md).
- For the drawing entry points and drawing options of the figure backend, see [SVG circuit diagrams](2_draw_figure.md).
- For the formatting options of parameter labels, see the parameter formatting section in [SVG circuit diagrams](2_draw_figure.md).
- For the control flow constructs used on this page, see [Classical data and control flow](../0_circuit/9_classical_control_flow.md).
- For the overall module structure and the rendering pipeline, see [Visualization](0_overview.md).
