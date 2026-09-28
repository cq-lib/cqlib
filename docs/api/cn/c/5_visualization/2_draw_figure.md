# SVG 线路图（C）

`circuit_to_figure` 把线路绘制为 SVG 图形，返回 SVG 标记字符串。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## circuit_to_figure(circuit, options)

绘制线路的 SVG 图形。

参数：

- `circuit` (`const CCircuit*`)：待绘制的线路。
- `options` (`const FigureDrawerOptionsC*`)：绘制选项；NULL 时使用库默认值。

返回：堆上 C 字符串（SVG 标记），用 `cqlib_string_free` 释放；失败返回 NULL。

---

## FigureDrawerOptionsC

图形绘制选项，按值填充；未设置的字段使用库默认值：

| 字段 | 类型 | 含义 | 取值行为 |
| --- | --- | --- | --- |
| `show_params` | `uint8_t` | 是否在门标签后附加参数文本。 | `0` 关闭，`1` 开启。 |
| `decompose_circuit_gates` | `uint8_t` | 绘制前是否分解复合门。 | `0` 不分解，`1` 分解。 |
| `width_per_column` | `double` | 每个逻辑列的图形宽度缩放。 | 正值生效。 |
| `height_per_qubit` | `double` | 每个量子比特的图形高度缩放。 | 正值生效。 |
| `dpi` | `uint32_t` | 导出 PNG 时的栅格化 DPI。 | 必须 `> 0`。 |
| `fold` | `int32_t` | 每行最大列数（超过则折叠换行）。 | `<= 0` 禁用折叠。 |
| `initial_state` | `uint8_t` | 是否在比特标签中显示 `\|0>`。 | `0` 不显示，`1` 显示。 |
| `reverse_bits` | `uint8_t` | 是否反转显示的比特顺序。 | `0` 原序，`1` 反序。 |

`dpi` 只在 PNG 栅格化时参与计算（见 [落盘与输出](5_render_to_file.md)）；`fold` 控制宽线路的折叠换行，`<= 0` 时整行输出不折叠。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    /* 1. 默认选项：返回 SVG 标记字符串 */
    char *svg = circuit_to_figure(qc, NULL);
    if (svg != NULL) {
        printf("%s\n", svg);  /* "<?xml ...><svg ...>...</svg>" */
        cqlib_string_free(svg);
    }

    /* 2. 显式选项：显示初态、每行最多 8 列、PNG 导出 300 DPI */
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

文本绘制与文件输出见 [文本线路图](1_draw_text.md)、[落盘与输出](5_render_to_file.md)。
