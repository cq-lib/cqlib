# 测量结果可视化（C）

本页覆盖测量结果的可视化：直方图 `plot_histogram` 与概率分布图 `plot_distribution`，以及绘图选项 `CResultPlotOptions`。两个入口都接受 `CExecutionResult` 句柄，返回堆上 SVG 标记字符串，用 `cqlib_string_free` 释放。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

`CExecutionResult` 句柄由 `execution_result_new` 创建、`execution_result_finish` 填入测量计数，构造方式见 [Result](../2_device/5_result.md)。

---

## plot_histogram(result, options)

把执行结果的测量计数绘制为直方图。计数按结果的量子比特数格式化为比特串标签，作为横轴刻度；纵轴从零起算，不绘制负值区间。

```c
char *plot_histogram(const struct CExecutionResult *result,
                     const struct CResultPlotOptions *options);
```

参数：

- `result` (`const CExecutionResult*`)：包含测量计数的执行结果。
- `options` (`const CResultPlotOptions*`)：绘图选项；NULL 时使用库默认值。

返回：堆上 C 字符串（SVG 标记），用 `cqlib_string_free` 释放；失败返回 NULL。

失败场景：

- `result` 为 NULL，或执行结果不含计数。
- `options` 中出现非法 UTF-8 字符串、`color` / `legend` 数组内含 NULL 元素，或绘图选项不一致。

---

## plot_distribution(result, options)

把执行结果的测量计数归一化为概率后绘制。每一条计数除以计数总和得到概率，柱高与纵轴刻度按概率标注。

```c
char *plot_distribution(const struct CExecutionResult *result,
                        const struct CResultPlotOptions *options);
```

参数：

- `result` (`const CExecutionResult*`)：包含测量计数的执行结果。
- `options` (`const CResultPlotOptions*`)：绘图选项；NULL 时使用库默认值。

返回：堆上 C 字符串（SVG 标记），用 `cqlib_string_free` 释放；失败返回 NULL。

失败场景：在 [plot_histogram](#plothistogramresult-options) 的基础上，计数求和为零时同样返回 NULL。

---

## CResultPlotOptions

结果图选项，按值填充；`options` 传 NULL 时使用库默认值。选项中的字符串须为有效 UTF-8，否则绘图入口返回 NULL。

| 字段 | 类型 | 含义 | 取值行为 |
| --- | --- | --- | --- |
| `has_figsize` | `uint8_t` | `fig_width` / `fig_height` 是否携带自定义图尺寸。 | `0` 忽略宽高，非 `0` 生效。 |
| `fig_width` | `double` | 图宽，单位为类英寸单位，按每单位 `100` 像素换算为 SVG 像素。 | `has_figsize == 0` 时忽略。给定尺寸时宽、高的下限均为 `2`。 |
| `fig_height` | `double` | 图高，单位与换算同 `fig_width`。 | `has_figsize == 0` 时忽略；下限规则同 `fig_width`。 |
| `color` | `const char* const*` | 每个数据集的颜色，长度由 `color_len` 给出。 | NULL 或 `color_len == 0` 使用内置的定性调色板；数组内含 NULL 元素时返回 NULL。 |
| `color_len` | `uintptr_t` | `color` 的条目数。 | `0` 视为未提供配色。 |
| `number_to_keep` | `intptr_t` | 保留取值最大的 `k` 条，其余聚合为标签 `rest` 的条目。 | 负值禁用聚合；`k` 为 `0` 或不小于条目数时不聚合。取值并列时按键的标签排序，保证输出跨平台一致。 |
| `sort` | `const char*` | 排序策略。 | NULL 默认 `asc`；取值见下表。非法 UTF-8 时返回 NULL。 |
| `target_string` | `const char*` | `sort = "hamming"` 使用的目标比特串。 | NULL 表示未设置。 |
| `legend` | `const char* const*` | 图例条目，每个数据集一条，长度由 `legend_len` 给出。 | NULL 不绘制图例。结果绘制入口只产生一个数据集，因此给出图例时其长度必须为 `1`。 |
| `legend_len` | `uintptr_t` | `legend` 的条目数。 | 与 `legend` 配对使用。 |
| `bar_labels` | `uint8_t` | 是否在柱顶绘制数值标签。 | `0` 不绘制，`1` 绘制；库默认绘制（仅当 `options` 为 NULL 时应用）。 |
| `title` | `const char*` | 图标题。 | NULL 不绘制标题。 |

### 排序策略

| `sort` 取值 | 说明 |
| --- | --- |
| `asc` | 按比特串标签的字典序升序。 |
| `desc` | 按比特串标签的字典序降序。 |
| `value` | 按标签在各数据集中的峰值升序；峰值并列时按标签升序。 |
| `value_desc` | 按标签在各数据集中的峰值降序；峰值并列时按标签升序。 |
| `hamming` | 按与 `target_string` 的汉明距离升序；距离并列时按标签升序。标签 `rest` 始终排在末尾。 |

---

## 示例

```c
#include <stdio.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build an execution result with measurement counts */
    const char *task_id = "task-visualization";
    uint32_t qubits[2] = {0, 1};
    struct CExecutionResult *result =
        execution_result_new(task_id, qubits, 2, 7, NULL);
    if (result == NULL) {
        return 1;
    }
    const char *bitstrings[2] = {"00", "11"};
    uint64_t counts[2] = {2, 5};
    if (execution_result_finish(result, bitstrings, counts, 2) != 0) {
        execution_result_free(result);
        return 1;
    }

    /* 2. Histogram with default options */
    char *svg = plot_histogram(result, NULL);
    if (svg != NULL) {
        printf("%s\n", svg);
        cqlib_string_free(svg);
    }

    /* 3. Distribution with explicit options: custom canvas and title */
    struct CResultPlotOptions options;
    memset(&options, 0, sizeof(options));
    options.has_figsize = 1;
    options.fig_width = 3.2;
    options.fig_height = 2.4;
    options.number_to_keep = -1;  /* negative disables aggregation */
    options.bar_labels = 1;
    options.title = "Measurement counts";
    svg = plot_distribution(result, &options);
    if (svg != NULL) {
        printf("%s\n", svg);
        cqlib_string_free(svg);
    }

    execution_result_free(result);
    return 0;
}
```

按与目标比特串的汉明距离排序并聚合其余条目：把 `sort` 设为 `"hamming"`、`target_string` 设为目标比特串、`number_to_keep` 设为保留条数即可，其余条目聚合为 `rest`。把绘制结果写出为文件见 [落盘与输出](5_render_to_file.md)。
