# FigureDrawer

Renders a circuit into an SVG vector diagram that can be written to a file, suitable for illustrations in documents and reports.

---

## Drawing options FigureDrawerOptionsC

```c
typedef struct FigureDrawerOptionsC {
  uint8_t show_params;
  uint8_t decompose_circuit_gates;
  double width_per_column;   // 每逻辑列的宽度比例
  double height_per_qubit;   // 每比特的高度比例
  uint32_t dpi;              // PNG 光栅化 DPI（须 > 0，默认 160）
  int32_t fold;              // 每行最大列数；<=0 不折行
  uint8_t initial_state;
  uint8_t reverse_bits;
} FigureDrawerOptionsC;
```

| Field | Type | Description |
| --- | --- | --- |
| `show_params` | `0/1` | Whether a parameterized gate displays its parameter value. |
| `decompose_circuit_gates` | `0/1` | Whether composite gates are decomposed before drawing. |
| `width_per_column` | `double` | The width scaling factor of each logical column. |
| `height_per_qubit` | `double` | The height scaling factor of each qubit. |
| `dpi` | `uint32_t` | The rasterization resolution when exporting PNG; must be `> 0`. |
| `fold` | `int32_t` | The maximum number of logical columns per row, folding beyond it; `<= 0` disables folding. |
| `initial_state` | `0/1` | Whether the qubit labels display `\|0⟩`. |
| `reverse_bits` | `0/1` | Whether the display order of the qubits is reversed. |

The options structure is passed by pointer; when **NULL** is passed for `options` of either rendering function, the library defaults are used as a whole. With zero-initialization (`= {0}`) `dpi` is 0, so `opts.dpi` must be set explicitly before exporting PNG.

---

## Functions

### circuit_to_figure(circuit, options)

Render the circuit into an SVG markup string.

Parameters:

- `circuit` (`const CCircuit *`): the circuit.
- `options` (`const FigureDrawerOptionsC *`): the options; NULL uses the default values.

Returns:

- `char *`: SVG markup (containing the `<svg` root element) that must be released with `cqlib_string_free`; NULL is returned on failure.

### render_figure_to_file(circuit, output_path, options)

Render according to the file extension and write the file: `.svg` for vector output, `.png` for rasterized output.

Parameters:

- `output_path` (`const char *`): the output path; the extension determines the output format.

Returns:

- `int32_t`; `0` on success, `-5` when rendering or IO fails, `-1` when a parameter is NULL.

Error situations: an unsupported extension, an unwritable path, an invalid `dpi` (when exporting PNG), and so on.

---

## Example

### Rendering SVG in memory

```c
FigureDrawerOptionsC opts = {0};
opts.show_params = 1;
opts.fold = 18;

char *svg = circuit_to_figure(qc, &opts);
if (svg) {
    printf("%s\n", svg);        // SVG 标记，可写入 .svg 文件或嵌入 HTML
    cqlib_string_free(svg);
}
```

### Exporting to a file

```c
if (render_figure_to_file(qc, "bell.svg", &opts) != 0) {
    fprintf(stderr, "render failed\n");
}

FigureDrawerOptionsC hi = opts;
hi.dpi = 300;                              // 高分辨率 PNG
render_figure_to_file(qc, "bell.png", &hi);
```

Text drawing is covered by [TextDrawer](1_text_drawer.md); unitary matrix export is provided by the Circuit module ([Circuit To Matrix](../0_circuit/3_circuit_to_matrix.md)) and is not provided again by the visualization module.
