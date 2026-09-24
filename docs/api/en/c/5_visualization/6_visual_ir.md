# Visual IR (C)

This page covers the visualization intermediate representation: the build entry point `build_visual_circuit`, the build options `CVisualBuildOptions` / `CParameterFormatOptions`, and the query and drawing functions of the IR handle `CVisualCircuit`. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md).

Circuit drawing is a two-step pipeline: first build the backend-agnostic visualization IR from the circuit, then render it with a concrete backend:

```text
CCircuit ──build_visual_circuit──▶ CVisualCircuit ──┬── draw_text_from_visual ──▶ text diagram
                                                    └── draw_figure_from_visual ──▶ SVG figure
```

The build stage handles the circuit-semantic work: mapping qubits to lane indices, formatting parameters into label text, classifying instructions into drawing styles, and scheduling operations into non-overlapping columns. The drawing stages only consume these results and never read the circuit back. Consequently both backends agree on gate order, column layout, and labels for the same circuit; build options can also be tuned independently of drawing options.

---

## Constants

### Parameter display modes (`CParameterFormatOptions.mode`)

| Constant | Value | Mode |
| --- | --- | --- |
| `PARAM_MODE_NUMERIC` | 0 | Prefer numeric display when evaluable. |
| `PARAM_MODE_SYMBOLIC` | 1 | Prefer symbolic expression display. |
| `PARAM_MODE_SYMBOLIC_WITH_VALUE` | 2 | Symbolic expression with numeric value. |
| `PARAM_MODE_PI_FRACTION_PREFERRED` | 3 | Prefer `k*pi/n` fraction display. |

Unknown values fall back to numeric mode (`PARAM_MODE_NUMERIC`).

### Operation drawing styles (returned by `visual_circuit_operation_style`)

| Constant | Value | Style |
| --- | --- | --- |
| `VISUAL_OP_STYLE_GATE` | 0 | Generic gate-like box. |
| `VISUAL_OP_STYLE_CONTROLLED` | 1 | Controlled operation; query the control count with `visual_circuit_operation_num_controls`. |
| `VISUAL_OP_STYLE_CZ` | 2 | Controlled-Z marker, drawn as two dots joined by a vertical line. |
| `VISUAL_OP_STYLE_SWAP` | 3 | Swap marker. |
| `VISUAL_OP_STYLE_BARRIER` | 4 | Barrier marker. |
| `VISUAL_OP_STYLE_MEASURE` | 5 | Measurement marker. |
| `VISUAL_OP_STYLE_RESET` | 6 | Reset marker. |
| `VISUAL_OP_STYLE_DELAY` | 7 | Delay marker. |
| `VISUAL_OP_STYLE_CONTROL_FLOW` | 8 | Control-flow marker; query the kind with `visual_circuit_operation_control_flow_kind`. |

### Control-flow kinds (returned by `visual_circuit_operation_control_flow_kind`)

| Constant | Value | Kind |
| --- | --- | --- |
| `VISUAL_CF_KIND_IF_ELSE_BLOCK` | 0 | If/else block. |
| `VISUAL_CF_KIND_WHILE_BLOCK` | 1 | `while` loop block. |
| `VISUAL_CF_KIND_FOR_BLOCK` | 2 | `for` loop block. |
| `VISUAL_CF_KIND_SWITCH_BLOCK` | 3 | Switch block. |
| `VISUAL_CF_KIND_BREAK` | 4 | Structured `break` marker. |
| `VISUAL_CF_KIND_CONTINUE` | 5 | Structured `continue` marker. |
| `VISUAL_CF_KIND_IF_START` | 6 | Flattened if-start marker. |
| `VISUAL_CF_KIND_ELSE_START` | 7 | Flattened else-start marker. |
| `VISUAL_CF_KIND_WHILE_START` | 8 | Flattened while-start marker. |
| `VISUAL_CF_KIND_FOR_START` | 9 | Flattened for-start marker. |
| `VISUAL_CF_KIND_SWITCH_START` | 10 | Flattened switch-start marker. |
| `VISUAL_CF_KIND_CASE_START` | 11 | Flattened case-start marker. |
| `VISUAL_CF_KIND_DEFAULT_START` | 12 | Flattened default-start marker. |
| `VISUAL_CF_KIND_END` | 13 | Flattened block-end marker. |

Block kinds (`*_BLOCK`) are produced by the build stage; before drawing, the backends flatten blocks into per-marker kinds (`*_START`, `VISUAL_CF_KIND_END`). Marker labels look like `If-0 true`, `End-0`, with an index that increases within a figure.

---

## Build Options

### CParameterFormatOptions

Formatting options for parameter label text:

| Field | Type | Meaning | Value behavior |
| --- | --- | --- | --- |
| `mode` | `uint32_t` | Parameter display mode. | One of the `PARAM_MODE_*` constants; unknown values fall back to numeric mode. |
| `decimal_precision` | `uintptr_t` | Decimal precision for fixed-point/scientific formatting. | Takes the given value as-is. |
| `scientific_lower_bound` | `double` | Scientific-notation lower bound. | Values in `(0, scientific_lower_bound)` use scientific notation. |
| `scientific_upper_bound` | `double` | Scientific-notation upper bound. | Values `>= scientific_upper_bound` use scientific notation. |
| `pi_tolerance` | `double` | Tolerance for pi-fraction matching. | When `value / pi` is within this tolerance of a rational fraction, the fraction display applies. |
| `pi_max_denominator` | `int64_t` | Maximum denominator for pi-fraction matching. | Fractions with a larger denominator do not participate in matching. |

### CVisualBuildOptions

Visualization IR build options; they only affect the build stage and are shared by the text and figure backends. Passing NULL for `options` uses library defaults:

| Field | Type | Meaning | Value behavior |
| --- | --- | --- | --- |
| `decompose_circuit_gates` | `uint8_t` | Whether to expand circuit-gate definitions before building. | `0` no expansion (default), `1` expand. A failed expansion makes the build return NULL. |
| `reserve_full_span_for_multi_qubit` | `uint8_t` | Whether to reserve the full lane span (`min..=max`) for multi-qubit operations. | `0` keeps only the operation's own lanes, `1` keeps the full span (default). |
| `parameter_format` | `struct CParameterFormatOptions` | Parameter label formatting options, embedded by value. | Field table above. |

---

## Build and Free

### build_visual_circuit(circuit, options)

Builds the visualization IR from a circuit.

```c
struct CVisualCircuit *build_visual_circuit(const struct CCircuit *circuit,
                                           const struct CVisualBuildOptions *options);
```

Parameters:

- `circuit` (`const CCircuit*`): the circuit to build.
- `options` (`const CVisualBuildOptions*`): build options; NULL uses library defaults.

Returns: a newly allocated `CVisualCircuit*` on success (free with `visual_circuit_free`); NULL on NULL input or build failure (circuit-gate expansion failure, an operation referencing an unknown qubit, or a symbolic-parameter index out of range). The IR's qubit count equals the circuit's qubit count.

### visual_circuit_free(ptr)

Frees the `CVisualCircuit` handle; NULL is allowed.

---

## Circuit-Level Queries

### visual_circuit_num_qubits(ptr) / visual_circuit_num_operations(ptr) / visual_circuit_num_columns(ptr)

Return the qubit count, the operation count, and the number of occupied columns, respectively; a NULL handle returns 0.

### visual_circuit_qubits_len(ptr) / visual_circuit_qubits(ptr, buffer, len)

Two-step read of the qubit ids in display order: `*_len` returns the entry count (0 for NULL), and the fill function copies the ids into `buffer`.

```c
uintptr_t visual_circuit_qubits_len(const struct CVisualCircuit *ptr);
int32_t visual_circuit_qubits(const struct CVisualCircuit *ptr,
                              uint32_t *buffer,
                              uintptr_t len);
```

Returns (fill function): 0 on success; -1 for a NULL handle (or a NULL `buffer` with a non-zero entry count); -8 when `len` is smaller than the entry count.

---

## Operation-Level Queries

The following functions access the operation list (in circuit order at build time) by index. Out-of-range `index` or NULL-handle return values are described per function.

### visual_circuit_operation_label(ptr, index)

Returns the operation's primary display label as a heap-allocated C string, freed with `cqlib_string_free`; NULL input or out-of-bounds returns NULL.

```c
char *visual_circuit_operation_label(const struct CVisualCircuit *ptr,
                                     uintptr_t index);
```

### visual_circuit_operation_column(ptr, index)

Returns the column the operation was scheduled into; NULL input or out-of-bounds returns `UINTPTR_MAX`.

### visual_circuit_operation_span_cols(ptr, index)

Returns the number of logical columns the operation reserves; NULL input or out-of-bounds returns `UINTPTR_MAX`. Ordinary operations take `1`; control-flow blocks are computed from the child circuit's column count plus separators and margins.

### visual_circuit_operation_is_span_box(ptr, index)

Returns whether the operation is drawn as a single box spanning multiple qubit wires: `1` yes, `0` no; NULL input or out-of-bounds returns `-1`. Custom unitary gates, circuit gates, and store instructions take `1`.

### visual_circuit_operation_lanes_len(ptr, index) / visual_circuit_operation_lanes(ptr, index, buffer, len)

Two-step read of the operation's operand lanes: the display-order indices of the qubits involved, arranged in operand order. Operations sharing a qubit must occupy adjacent columns; operations on disjoint qubits may share a column.

```c
uintptr_t visual_circuit_operation_lanes_len(const struct CVisualCircuit *ptr,
                                             uintptr_t index);
int32_t visual_circuit_operation_lanes(const struct CVisualCircuit *ptr,
                                       uintptr_t index,
                                       uintptr_t *buffer,
                                       uintptr_t len);
```

Returns (fill function): 0 on success; -1 for a NULL handle (or a NULL `buffer` with a non-zero entry count); -8 when `len` is smaller than the entry count.

### visual_circuit_operation_covered_lanes_len(ptr, index) / visual_circuit_operation_covered_lanes(ptr, index, buffer, len)

Two-step read of the lanes the operation reserves in its column: qubit indices kept free of overlap. Control-flow blocks reserve all lanes; a barrier with empty `lanes` reserves all lanes; other operations with empty `lanes` reserve `[0]`. When more than one lane is involved and `reserve_full_span_for_multi_qubit` is `1`, the full `min..=max` span of `lanes` is reserved; otherwise `lanes` itself is reserved.

```c
uintptr_t visual_circuit_operation_covered_lanes_len(const struct CVisualCircuit *ptr,
                                                     uintptr_t index);
int32_t visual_circuit_operation_covered_lanes(const struct CVisualCircuit *ptr,
                                               uintptr_t index,
                                               uintptr_t *buffer,
                                               uintptr_t len);
```

Returns (fill function): 0 on success; -1 for a NULL handle (or a NULL `buffer` with a non-zero entry count); -8 when `len` is smaller than the entry count.

### visual_circuit_operation_params_len(ptr, index) / visual_circuit_operation_params(ptr, index, out, len)

Two-step read of the operation's formatted parameter labels: `*_len` returns the entry count (0 for NULL input or out-of-bounds), and the fill function writes each label as a freshly allocated C string, each freed individually with `cqlib_string_free`.

```c
uintptr_t visual_circuit_operation_params_len(const struct CVisualCircuit *ptr,
                                               uintptr_t index);
int32_t visual_circuit_operation_params(const struct CVisualCircuit *ptr,
                                        uintptr_t index,
                                        char **out,
                                        uintptr_t len);
```

Returns (fill function): 0 on success; -1 for a NULL handle (or a NULL `out` with a non-zero entry count); -8 when `len` is smaller than the entry count; -3 when a label contains an embedded NUL and cannot form a C string.

### visual_circuit_operation_style(ptr, index)

Returns the operation's drawing style as one of the `VISUAL_OP_STYLE_*` constants; NULL input or out-of-bounds returns `UINT32_MAX`.

```c
uint32_t visual_circuit_operation_style(const struct CVisualCircuit *ptr,
                                        uintptr_t index);
```

### visual_circuit_operation_num_controls(ptr, index)

Returns the number of control qubits; meaningful only when the style is `VISUAL_OP_STYLE_CONTROLLED`. NULL input, out-of-bounds, or non-controlled styles return 0.

### visual_circuit_operation_control_flow_kind(ptr, index)

Returns the control-flow kind as one of the `VISUAL_CF_KIND_*` constants; meaningful only when the style is `VISUAL_OP_STYLE_CONTROL_FLOW`. NULL input, out-of-bounds, or non-control-flow styles return `UINT32_MAX`.

```c
uint32_t visual_circuit_operation_control_flow_kind(const struct CVisualCircuit *ptr,
                                                     uintptr_t index);
```

### Style and label classification rules

The build stage classifies instructions into styles and labels per the following table:

| Instruction | Style | Label |
| --- | --- | --- |
| Standard gate `SWAP` | `VISUAL_OP_STYLE_SWAP` | `SWAP` |
| Standard gate `CZ` | `VISUAL_OP_STYLE_CZ` | `CZ` |
| Standard gate with control count > 0 | `VISUAL_OP_STYLE_CONTROLLED` | Target gate name |
| Other standard gates | `VISUAL_OP_STYLE_GATE` | Gate name |
| Multi-controlled gate with control value `1` and base gate `Z` | `VISUAL_OP_STYLE_CZ` | `CZ` |
| Other multi-controlled gates | `VISUAL_OP_STYLE_CONTROLLED` | Base gate name |
| Custom unitary gate | `VISUAL_OP_STYLE_GATE`, span box | Unitary label, or `Unitary` when empty |
| Circuit gate | `VISUAL_OP_STYLE_GATE`, span box | Circuit-gate name, or `Gate` when empty |
| Barrier | `VISUAL_OP_STYLE_BARRIER` | `B` |
| Measurement | `VISUAL_OP_STYLE_MEASURE` | `M` |
| Reset | `VISUAL_OP_STYLE_RESET` | `R` |
| Delay | `VISUAL_OP_STYLE_DELAY` | `D` |
| Classical control flow | `VISUAL_OP_STYLE_CONTROL_FLOW` | `IF`, `WH`, `FOR`, `SW`, `Break`, or `Continue` |
| Measure-into-bit instruction | `VISUAL_OP_STYLE_MEASURE` | `M` |
| Store instruction | `VISUAL_OP_STYLE_GATE`, span box | `STORE` |

Gate names are abbreviated by the standard-gate convention: `SDG` displays as `SD`, `TDG` as `TD`, `Phase` and `GPhase` as `P`. Controlled-gate labels take the target gate name: `CX` and `CCX` become `X`, `CY` becomes `Y`, `CRX`/`CRY`/`CRZ` become `RX`/`RY`/`RZ`; otherwise the leading control prefix is dropped from the gate name.

---

## Drawing

### draw_text_from_visual(visual, options)

Draws pre-built visualization IR as a UTF-8 text diagram (box-drawing characters).

```c
char *draw_text_from_visual(const struct CVisualCircuit *visual,
                            const struct TextDrawerOptionsC *options);
```

Parameters:

- `visual` (`const CVisualCircuit*`): the visualization IR handle.
- `options` (`const TextDrawerOptionsC*`): drawing options (field table in [Text Drawing](1_draw_text.md)); NULL uses library defaults.

Returns: a heap-allocated C string, freed with `cqlib_string_free`; NULL on error.

### draw_figure_from_visual(visual, options)

Draws pre-built visualization IR as an SVG figure.

```c
char *draw_figure_from_visual(const struct CVisualCircuit *visual,
                              const struct FigureDrawerOptionsC *options);
```

Parameters:

- `visual` (`const CVisualCircuit*`): the visualization IR handle.
- `options` (`const FigureDrawerOptionsC*`): drawing options (field table in [SVG Figure](2_draw_figure.md)); NULL uses library defaults.

Returns: a heap-allocated C string (SVG markup), freed with `cqlib_string_free`; NULL on error.

---

## Example

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit *circuit = circuit_new(2);
    circuit_h(circuit, 0);
    circuit_cx(circuit, 0, 1);

    /* 1. Build the IR with default options */
    struct CVisualCircuit *visual = build_visual_circuit(circuit, NULL);
    if (visual == NULL) {
        circuit_free(circuit);
        return 1;
    }

    /* 2. Circuit-level queries */
    uintptr_t num_qubits = visual_circuit_num_qubits(visual);      /* 2 */
    uintptr_t num_ops = visual_circuit_num_operations(visual);     /* 2 */
    uintptr_t num_cols = visual_circuit_num_columns(visual);       /* 2 */

    /* 3. Two-step read of the qubit ids in display order */
    uintptr_t qubits_len = visual_circuit_qubits_len(visual);
    uint32_t *qubits = malloc(qubits_len * sizeof(uint32_t));
    if (visual_circuit_qubits(visual, qubits, qubits_len) == 0) {
        /* qubits[0] == 0, qubits[1] == 1 */
    }
    free(qubits);

    /* 4. Operation-level queries */
    char *label = visual_circuit_operation_label(visual, 0);
    if (label != NULL) {
        printf("label: %s\n", label);  /* "H" */
        cqlib_string_free(label);
    }
    uintptr_t column = visual_circuit_operation_column(visual, 1);  /* 1 */
    uint32_t style = visual_circuit_operation_style(visual, 1);      /* CONTROLLED */
    uintptr_t num_controls = visual_circuit_operation_num_controls(visual, 1);  /* 1 */

    uintptr_t lanes_len = visual_circuit_operation_lanes_len(visual, 1);
    uintptr_t *lanes = malloc(lanes_len * sizeof(uintptr_t));
    if (visual_circuit_operation_lanes(visual, 1, lanes, lanes_len) == 0) {
        /* lanes[0] == 0, lanes[1] == 1 */
    }
    free(lanes);

    /* 5. Out-of-bounds access fails gracefully */
    if (visual_circuit_operation_label(visual, 2) == NULL) {
        printf("index 2 out of range\n");
    }

    /* 6. Draw the pre-built IR with both backends */
    char *text = draw_text_from_visual(visual, NULL);
    if (text != NULL) {
        printf("%s\n", text);
        cqlib_string_free(text);
    }
    char *svg = draw_figure_from_visual(visual, NULL);
    if (svg != NULL) {
        cqlib_string_free(svg);
    }

    /* 7. Explicit build options: pi-fraction parameter labels */
    struct CVisualBuildOptions options;
    memset(&options, 0, sizeof(options));
    options.reserve_full_span_for_multi_qubit = 1;
    options.parameter_format.mode = PARAM_MODE_PI_FRACTION_PREFERRED;
    options.parameter_format.decimal_precision = 2;
    options.parameter_format.scientific_lower_bound = 1e-3;
    options.parameter_format.scientific_upper_bound = 1e4;
    options.parameter_format.pi_tolerance = 1e-3;
    options.parameter_format.pi_max_denominator = 16;
    struct CVisualCircuit *visual2 = build_visual_circuit(circuit, &options);
    if (visual2 != NULL) {
        visual_circuit_free(visual2);
    }

    visual_circuit_free(visual);
    circuit_free(circuit);
    return 0;
}
```

For drawing straight from a circuit (without going through the IR explicitly), see [Text Drawing](1_draw_text.md) and [SVG Figure](2_draw_figure.md); for writing SVG markup out to files, see [File Output](5_render_to_file.md).
