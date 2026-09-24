# SVG Figure (C)

`circuit_to_figure` draws a circuit as an SVG figure and returns the SVG markup string. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## circuit_to_figure(circuit, options)

Draws the circuit as an SVG figure.

Parameters:

- `circuit` (`const CCircuit*`): circuit to draw.
- `options` (`const FigureDrawerOptionsC*`): drawing options; NULL uses library defaults.

Returns: a heap-allocated C string containing SVG markup; free with `cqlib_string_free`. NULL on error.

---

## FigureDrawerOptionsC

Figure drawing options, filled by value; unset fields use library defaults:

| Field | Type | Meaning | Value behavior |
| --- | --- | --- | --- |
| `show_params` | `uint8_t` | Whether to append parameter text to gate labels. | `0` off, `1` on. |
| `decompose_circuit_gates` | `uint8_t` | Whether to decompose circuit-gates before drawing. | `0` no decomposition, `1` decompose. |
| `width_per_column` | `double` | Figure width scale per logical column. | Positive values apply. |
| `height_per_qubit` | `double` | Figure height scale per qubit. | Positive values apply. |
| `dpi` | `uint32_t` | Rasterization DPI when exporting PNG. | Must be `> 0`. |
| `fold` | `int32_t` | Maximum columns per row (excess columns fold onto new rows). | `<= 0` disables folding. |
| `initial_state` | `uint8_t` | Whether to show `\|0>` in qubit labels. | `0` hidden, `1` shown. |
| `reverse_bits` | `uint8_t` | Whether to reverse display order of qubits. | `0` original order, `1` reversed. |

`dpi` participates only in PNG rasterization (see [File Output](5_render_to_file.md)); `fold` controls folding of wide circuits onto multiple rows, with `<= 0` keeping a single unfolded row.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    /* 1. Default options: returns the SVG markup string */
    char *svg = circuit_to_figure(qc, NULL);
    if (svg != NULL) {
        printf("%s\n", svg);  /* "<?xml ...><svg ...>...</svg>" */
        cqlib_string_free(svg);
    }

    /* 2. Explicit options: show initial state, fold at 8 columns, 300 DPI for PNG export */
    struct FigureDrawerOptionsC options = {0};
    options.initial_state = 1;
    options.fold = 8;  /* <= 0 disables folding */
    options.dpi = 300;
    char *svg2 = circuit_to_figure(qc, &options);
    if (svg2 != NULL) {
        cqlib_string_free(svg2);
    }

    circuit_free(qc);
    return 0;
}
```

For text drawing and file output see [Text Drawing](1_draw_text.md) and [File Output](5_render_to_file.md).
