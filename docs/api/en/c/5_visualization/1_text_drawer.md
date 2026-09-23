# TextDrawer

Renders a circuit into a UTF-8 text diagram (box-drawing characters), suitable for log output and viewing in a terminal.

---

## Drawing options TextDrawerOptionsC

```c
typedef struct TextDrawerOptionsC {
  uint8_t show_params;              // 是否在门标签后附加参数文本
  uint8_t decompose_circuit_gates;  // 绘制前是否分解复合门
  intptr_t line_width;              // 折行宽度；<=0 不折行
  uint8_t initial_state;            // 比特线首是否显示 |0>
  uint8_t reverse_bits;             // 是否反转比特显示顺序
} TextDrawerOptionsC;
```

| Field | Type | Description |
| --- | --- | --- |
| `show_params` | `0/1` | Whether a parameterized gate displays its parameter value (such as `RZ(0.5)`). |
| `decompose_circuit_gates` | `0/1` | Whether composite gates are decomposed before drawing; used when the circuit comes from IR parsing or a compilation result and contains composite gates. |
| `line_width` | integer | The maximum width of a single folded segment; `<= 0` disables folding. |
| `initial_state` | `0/1` | Whether `\|0⟩` is displayed at the start of each qubit line. |
| `reverse_bits` | `0/1` | Whether the display order of the qubits is reversed (qubit 0 is displayed at the bottom). |

The options structure is passed by pointer; when **NULL** is passed for `options` of `circuit_to_text`, the library defaults are used as a whole. Zero-initialization (`= {0}`) means every switch is off and folding is disabled.

---

## Functions

### circuit_to_text(circuit, options)

Parameters:

- `circuit` (`const CCircuit *`): the circuit, which is not modified.
- `options` (`const TextDrawerOptionsC *`): the drawing options; NULL uses the default values.

Returns:

- `char *`: a UTF-8 text diagram that must be released with `cqlib_string_free`; NULL is returned on failure (a NULL circuit and so on).

---

## Example

### Default options

```c
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

char *text = circuit_to_text(qc, NULL);
printf("%s\n", text);
cqlib_string_free(text);
```

The output looks like:

```text
     ┌───┐
q_0: ┤ H ├──■──
     └───┘┌─┴─┐
q_1: ─────┤ X ├
          └───┘
```

### Displaying parameters and folding

```c
TextDrawerOptionsC opts = {0};
opts.show_params = 1;      // 显示门参数
opts.line_width = 40;      // 超宽时折行

char *text = circuit_to_text(qc, &opts);
printf("%s\n", text);
cqlib_string_free(text);
```

SVG figure rendering is covered by [FigureDrawer](2_figure_drawer.md).
