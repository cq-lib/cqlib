# 文本线路图（C）

`circuit_to_text` 把线路绘制为 UTF-8 文本线路图（框线字符）。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## circuit_to_text(circuit, options)

绘制线路的 UTF-8 文本图。

参数：

- `circuit` (`const CCircuit*`)：待绘制的线路。
- `options` (`const TextDrawerOptionsC*`)：绘制选项；NULL 时使用库默认值。

返回：堆上 C 字符串，用 `cqlib_string_free` 释放；失败返回 NULL。

---

## TextDrawerOptionsC

文本绘制选项，按值填充；未设置的字段使用库默认值：

| 字段 | 类型 | 含义 | 取值行为 |
| --- | --- | --- | --- |
| `show_params` | `uint8_t` | 是否在门标签后附加参数文本。 | `0` 关闭，`1` 开启。 |
| `decompose_circuit_gates` | `uint8_t` | 绘制前是否分解复合门。 | `0` 不分解，`1` 分解。 |
| `line_width` | `intptr_t` | 折行后每段的最大宽度。 | `<= 0` 禁用折行。 |
| `initial_state` | `uint8_t` | 是否在每个量子比特线起始处显示 `\|0>`。 | `0` 不显示，`1` 显示。 |
| `reverse_bits` | `uint8_t` | 是否反转显示的比特顺序。 | `0` 原序，`1` 反序。 |

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    /* 1. 默认选项 */
    char *text = circuit_to_text(qc, NULL);
    if (text != NULL) {
        printf("%s\n", text);
        cqlib_string_free(text);
    }

    /* 2. 显式选项：显示初态、禁用折行 */
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

SVG 图形绘制与文件输出见 [SVG 线路图](2_draw_figure.md)、[落盘与输出](5_render_to_file.md)。
