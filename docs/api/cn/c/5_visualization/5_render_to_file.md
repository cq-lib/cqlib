# 落盘与输出（C）

`render_figure_to_file` 把线路直接渲染到输出文件，按扩展名选择格式：`.svg` 输出矢量图，`.png` 按 `FigureDrawerOptionsC.dpi` 栅格化输出。`render_state_plot_to_file` 与 `render_result_plot_to_file` 把状态图与结果图的 SVG 标记字符串写入文件，扩展名选择同样的两种格式。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## render_figure_to_file(circuit, output_path, options)

把线路渲染到输出文件。

参数：

- `circuit` (`const CCircuit*`)：待绘制的线路。
- `output_path` (`const char*`)：输出文件路径，扩展名 `.svg` 或 `.png` 决定输出格式。
- `options` (`const FigureDrawerOptionsC*`)：绘制选项（字段表见 [SVG 线路图](2_draw_figure.md)）；NULL 时使用库默认值。`.png` 输出时 `dpi` 必须大于 0。

返回：0 成功；-5 文件读写或渲染失败；-1 空指针。

---

## render_state_plot_to_file(svg, output_path)

把状态图的 SVG 标记字符串写入输出文件，`.svg` 输出矢量图，`.png` 输出栅格图。SVG 标记串来自 [状态图](3_state_plots.md) 的 `plot_*` 接口。

参数：

- `svg` (`const char*`)：`plot_bloch_vector`、`plot_state_city` 等接口返回的 SVG 标记字符串（该字符串本身仍由调用方用 `cqlib_string_free` 释放）。
- `output_path` (`const char*`)：输出文件路径，扩展名 `.svg` 或 `.png` 决定输出格式。

返回：0 成功；-1 空指针；-4 字符串不是有效的 UTF-8；-5 文件读写或渲染失败。

---

## render_result_plot_to_file(svg, output_path)

把结果图的 SVG 标记字符串写入输出文件，`.svg` 输出矢量图，`.png` 输出栅格图。SVG 标记串来自 [结果图](4_result_plots.md) 的 `plot_histogram`、`plot_distribution`。

参数：

- `svg` (`const char*`)：结果图接口返回的 SVG 标记字符串（该字符串本身仍由调用方用 `cqlib_string_free` 释放）。
- `output_path` (`const char*`)：输出文件路径，扩展名 `.svg` 或 `.png` 决定输出格式。

返回：0 成功；-1 空指针；-4 字符串不是有效的 UTF-8；-5 文件读写或渲染失败。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    /* 1. 默认选项输出 SVG */
    int32_t rc = render_figure_to_file(qc, "bell.svg", NULL);
    if (rc != 0) {
        printf("render failed: %d\n", rc);  /* -5: I/O or rendering failure */
    }

    /* 2. 指定 DPI 输出 PNG */
    struct FigureDrawerOptionsC options = {0};
    options.dpi = 300;
    options.fold = 8;
    rc = render_figure_to_file(qc, "bell.png", &options);

    /* 3. 把状态图的 SVG 标记串写入文件 */
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

在内存中取得 SVG 标记字符串见 [SVG 线路图](2_draw_figure.md)。
