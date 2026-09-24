# Text Drawing (C)

`circuit_to_text` draws a circuit as a UTF-8 text diagram (box-drawing characters). Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## circuit_to_text(circuit, options)

Draws the circuit as UTF-8 text.

Parameters:

- `circuit` (`const CCircuit*`): circuit to draw.
- `options` (`const TextDrawerOptionsC*`): drawing options; NULL uses library defaults.

Returns: a heap-allocated C string; free with `cqlib_string_free`. NULL on error.

---

## TextDrawerOptionsC

Text drawing options, filled by value; unset fields use library defaults:

| Field | Type | Meaning | Value behavior |
| --- | --- | --- | --- |
| `show_params` | `uint8_t` | Whether to append parameter text to gate labels. | `0` off, `1` on. |
| `decompose_circuit_gates` | `uint8_t` | Whether to decompose circuit-gates before drawing. | `0` no decomposition, `1` decompose. |
| `line_width` | `intptr_t` | Maximum width per wrapped segment. | `<= 0` disables wrapping. |
| `initial_state` | `uint8_t` | Whether to show `\|0>` at the start of each qubit wire. | `0` hidden, `1` shown. |
| `reverse_bits` | `uint8_t` | Whether to reverse the displayed qubit order. | `0` original order, `1` reversed. |

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    /* 1. Default options */
    char *text = circuit_to_text(qc, NULL);
    if (text != NULL) {
        printf("%s\n", text);
        cqlib_string_free(text);
    }

    /* 2. Explicit options: show initial state, disable wrapping */
    struct TextDrawerOptionsC options = {0};
    options.show_params = 1;
    options.initial_state = 1;
    options.line_width = 0;  /* <= 0 disables wrapping */
    char *text2 = circuit_to_text(qc, &options);
    if (text2 != NULL) {
        printf("%s\n", text2);
        cqlib_string_free(text2);
    }

    circuit_free(qc);
    return 0;
}
```

For SVG figures and file output see [SVG Figure](2_draw_figure.md) and [File Output](5_render_to_file.md).
