# Visualization (C)

The visualization module provides four groups of output: `circuit_to_text` outputs a box-drawing-character UTF-8 text diagram, `circuit_to_figure` outputs an SVG markup string, and `render_figure_to_file` renders a circuit directly to a `.svg` / `.png` file; state plots (Bloch, multi-Bloch, city, paulivec) and result plots (histogram, distribution) generate SVG from simulator states and execution results, and the Visual IR compiles a circuit into a backend-agnostic drawing intermediate representation. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## Page Navigation

| Page | Content |
| --- | --- |
| [Text Drawing](1_draw_text.md) | `circuit_to_text` and the `TextDrawerOptionsC` option table. |
| [SVG Figure](2_draw_figure.md) | `circuit_to_figure` and the `FigureDrawerOptionsC` option table. |
| [State Plots](3_state_plots.md) | `plot_bloch_vector`, `plot_bloch_multivector`, `plot_state_city`, `plot_state_paulivec`, and the `CStatePlotOptionsC` option table. |
| [Result Plots](4_result_plots.md) | `plot_histogram`, `plot_distribution`, and the `CResultPlotOptionsC` option table. |
| [File Output](5_render_to_file.md) | `render_figure_to_file`, `render_state_plot_to_file`, `render_result_plot_to_file`: renders to a `.svg` or `.png` file by extension. |
| [Visual IR](6_visual_ir.md) | `build_visual_circuit`, the `CVisualCircuit` handle family, and `draw_text_from_visual`, `draw_figure_from_visual`. |

---

## Shared Conventions

- The `options` parameter of every drawing entry accepts NULL, in which case library defaults apply; for explicit configuration, fill the corresponding options struct (`TextDrawerOptionsC` / `FigureDrawerOptionsC` / `CStatePlotOptionsC` / `CResultPlotOptionsC` / `CVisualBuildOptionsC`) by value — the struct memory stays owned by the caller.
- Functions returning `char*` (`circuit_to_text`, `circuit_to_figure`, the `plot_*` and `draw_*_from_visual` entries) are freed with `cqlib_string_free`.
- `CVisualCircuit*` handles are freed with `visual_circuit_free` (NULL is allowed); the accessor family is covered in [Visual IR](6_visual_ir.md).
- `render_figure_to_file` and the two plot-to-file entries return an `int32_t` error code: 0 on success; -5 on I/O or rendering failure; -1 for NULL pointers (the plot-to-file entries additionally return -4 for strings that are not valid UTF-8).
