# FigureDrawer

将线路渲染为 SVG 矢量图并可写入文件，适合文档与报告插图。

---

## 绘制选项 FigureDrawerOptionsC

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

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `show_params` | `0/1` | 参数化门是否显示参数值。 |
| `decompose_circuit_gates` | `0/1` | 绘制前是否分解复合门。 |
| `width_per_column` | `double` | 每个逻辑列的宽度缩放系数。 |
| `height_per_qubit` | `double` | 每个比特的高度缩放系数。 |
| `dpi` | `uint32_t` | 导出 PNG 时的光栅化分辨率，必须 `> 0`。 |
| `fold` | `int32_t` | 每行最大逻辑列数，超出折行；`<= 0` 不折行。 |
| `initial_state` | `0/1` | 比特标签是否显示 `\|0⟩`。 |
| `reverse_bits` | `0/1` | 是否反转比特显示顺序。 |

选项结构体按指针传入；两个渲染函数的 `options` 传 **NULL** 时整体使用库默认值。零初始化（`= {0}`）时 `dpi` 为 0，导出 PNG 前需显式设置 `opts.dpi`。

---

## 函数

### circuit_to_figure(circuit, options)

将线路渲染为 SVG 标记字符串。

参数：

- `circuit` (`const CCircuit *`)：线路。
- `options` (`const FigureDrawerOptionsC *`)：选项，NULL 使用默认值。

返回：

- `char *`：SVG 标记（含 `<svg` 根元素），需 `cqlib_string_free` 释放；失败返回 NULL。

### render_figure_to_file(circuit, output_path, options)

按扩展名渲染并写文件：`.svg` 矢量输出、`.png` 光栅化输出。

参数：

- `output_path` (`const char *`)：输出路径，扩展名决定输出格式。

返回：

- `int32_t`；成功 `0`，渲染或 IO 失败 `-5`，参数为 NULL `-1`。

错误场景：不支持的扩展名、路径不可写、`dpi` 非法（导出 PNG 时）等。

---

## 示例

### 内存中渲染 SVG

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

### 导出文件

```c
if (render_figure_to_file(qc, "bell.svg", &opts) != 0) {
    fprintf(stderr, "render failed\n");
}

FigureDrawerOptionsC hi = opts;
hi.dpi = 300;                              // 高分辨率 PNG
render_figure_to_file(qc, "bell.png", &hi);
```

文本绘制见 [TextDrawer](1_text_drawer.md)；酉矩阵导出由 Circuit 模块提供（[Circuit To Matrix](../0_circuit/3_circuit_to_matrix.md)），可视化模块不重复提供。
