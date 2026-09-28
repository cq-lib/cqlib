# File Output (C)

`render_figure_to_file` renders a circuit directly to an output file, selecting the format by extension: `.svg` writes a vector figure, `.png` rasterizes with `FigureDrawerOptionsC.dpi`. `render_state_plot_to_file` and `render_result_plot_to_file` write state-plot and result-plot SVG markup strings to files, with the same two extensions. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## render_figure_to_file(circuit, output_path, options)

Renders the circuit to an output file.

Parameters:

- `circuit` (`const CCircuit*`): circuit to draw.
- `output_path` (`const char*`): output file path; the `.svg` or `.png` extension selects the format.
- `options` (`const FigureDrawerOptionsC*`): drawing options (field table in [SVG Figure](2_draw_figure.md)); NULL uses library defaults. For `.png` output, `dpi` must be greater than 0.

Returns: 0 on success; -5 on I/O or rendering failure; -1 for NULL pointers.

---

## render_state_plot_to_file(svg, output_path)

Writes state-plot SVG markup to an output file: `.svg` writes vector output, `.png` writes raster output. The markup comes from the `plot_*` entries on the [State Plots](3_state_plots.md) page.

Parameters:

- `svg` (`const char*`): SVG markup string returned by `plot_bloch_vector`, `plot_state_city`, and their siblings (the string itself is still released by the caller with `cqlib_string_free`).
- `output_path` (`const char*`): output file path; the `.svg` or `.png` extension selects the format.

Returns: 0 on success; -1 for NULL pointers; -4 for strings that are not valid UTF-8; -5 on I/O or rendering failure.

---

## render_result_plot_to_file(svg, output_path)

Writes result-plot SVG markup to an output file: `.svg` writes vector output, `.png` writes raster output. The markup comes from `plot_histogram` and `plot_distribution` on the [Result Plots](4_result_plots.md) page.

Parameters:

- `svg` (`const char*`): SVG markup string returned by the result-plot entries (the string itself is still released by the caller with `cqlib_string_free`).
- `output_path` (`const char*`): output file path; the `.svg` or `.png` extension selects the format.

Returns: 0 on success; -1 for NULL pointers; -4 for strings that are not valid UTF-8; -5 on I/O or rendering failure.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    /* 1. SVG output with default options */
    int32_t rc = render_figure_to_file(qc, "bell.svg", NULL);
    if (rc != 0) {
        printf("render failed: %d\n", rc);  /* -5: I/O or rendering failure */
    }

    /* 2. PNG output with an explicit DPI */
    struct FigureDrawerOptionsC options = {0};
    options.dpi = 300;
    options.fold = 8;
    rc = render_figure_to_file(qc, "bell.png", &options);

    /* 3. Write a state-plot SVG markup string to a file */
    struct CStatevector *sv = statevector_new(2);
    statevector_apply_h(sv, 0);
    statevector_apply_cx(sv, 0, 1);
    char *svg = plot_bloch_vector(sv, NULL);
    if (svg != NULL) {
        render_state_plot_to_file(svg, "bell_bloch.svg");
        cqlib_string_free(svg);
    }

    statevector_free(sv);
    circuit_free(qc);
    return 0;
}
```

To obtain the SVG markup string in memory, see [SVG Figure](2_draw_figure.md).
