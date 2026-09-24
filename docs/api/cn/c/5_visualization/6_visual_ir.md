# 可视化 IR（C）

本页覆盖线路绘制的中间表示层：构建入口 `build_visual_circuit`、构建选项 `CVisualBuildOptions` / `CParameterFormatOptions`，以及 IR 句柄 `CVisualCircuit` 的查询与绘制函数。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

线路绘制分两步：先把线路构建为与绘制后端无关的可视化 IR，再由具体后端绘制：

```text
CCircuit ──build_visual_circuit──▶ CVisualCircuit ──┬── draw_text_from_visual ──▶ 文本图
                                                    └── draw_figure_from_visual ──▶ SVG 图
```

构建阶段承担与线路语义相关的工作：把量子比特映射为下标（lane）、把参数格式化为标签文本、把指令归类为绘制样式，并把操作排入互不重叠的列。绘制阶段只消费这些结果，不再读回线路。因此同一条线路在两种后端下的门顺序、列划分与标签一致；构建选项也可以独立于绘制选项调整。

---

## 常量

### 参数显示模式（`CParameterFormatOptions.mode`）

| 常量 | 值 | 模式 |
| --- | --- | --- |
| `PARAM_MODE_NUMERIC` | 0 | 可求值时优先数值显示。 |
| `PARAM_MODE_SYMBOLIC` | 1 | 优先符号表达式显示。 |
| `PARAM_MODE_SYMBOLIC_WITH_VALUE` | 2 | 符号表达式附数值。 |
| `PARAM_MODE_PI_FRACTION_PREFERRED` | 3 | 优先 `k*pi/n` 分数显示。 |

未知取值回退为数值模式（`PARAM_MODE_NUMERIC`）。

### 操作绘制样式（`visual_circuit_operation_style` 返回值）

| 常量 | 值 | 样式 |
| --- | --- | --- |
| `VISUAL_OP_STYLE_GATE` | 0 | 普通门块。 |
| `VISUAL_OP_STYLE_CONTROLLED` | 1 | 受控操作；控制位数用 `visual_circuit_operation_num_controls` 查询。 |
| `VISUAL_OP_STYLE_CZ` | 2 | 受控 Z 标记，按两个以竖线相连的圆点绘制。 |
| `VISUAL_OP_STYLE_SWAP` | 3 | 交换标记。 |
| `VISUAL_OP_STYLE_BARRIER` | 4 | 屏障标记。 |
| `VISUAL_OP_STYLE_MEASURE` | 5 | 测量标记。 |
| `VISUAL_OP_STYLE_RESET` | 6 | 复位标记。 |
| `VISUAL_OP_STYLE_DELAY` | 7 | 延时标记。 |
| `VISUAL_OP_STYLE_CONTROL_FLOW` | 8 | 控制流标记；具体类别用 `visual_circuit_operation_control_flow_kind` 查询。 |

### 控制流类别（`visual_circuit_operation_control_flow_kind` 返回值）

| 常量 | 值 | 类别 |
| --- | --- | --- |
| `VISUAL_CF_KIND_IF_ELSE_BLOCK` | 0 | 条件分支块。 |
| `VISUAL_CF_KIND_WHILE_BLOCK` | 1 | `while` 循环块。 |
| `VISUAL_CF_KIND_FOR_BLOCK` | 2 | `for` 循环块。 |
| `VISUAL_CF_KIND_SWITCH_BLOCK` | 3 | 多分支选择块。 |
| `VISUAL_CF_KIND_BREAK` | 4 | 结构化 `break` 标记。 |
| `VISUAL_CF_KIND_CONTINUE` | 5 | 结构化 `continue` 标记。 |
| `VISUAL_CF_KIND_IF_START` | 6 | 展开后的条件分支起始标记。 |
| `VISUAL_CF_KIND_ELSE_START` | 7 | 展开后的假分支起始标记。 |
| `VISUAL_CF_KIND_WHILE_START` | 8 | 展开后的循环起始标记。 |
| `VISUAL_CF_KIND_FOR_START` | 9 | 展开后的 `for` 起始标记。 |
| `VISUAL_CF_KIND_SWITCH_START` | 10 | 展开后的选择起始标记。 |
| `VISUAL_CF_KIND_CASE_START` | 11 | 展开后的分支起始标记。 |
| `VISUAL_CF_KIND_DEFAULT_START` | 12 | 展开后的默认分支起始标记。 |
| `VISUAL_CF_KIND_END` | 13 | 展开后的块结束标记。 |

块类别（`*_BLOCK`）由构建阶段产生；绘制后端在绘制前把块展开为逐条标记（`*_START`、`VISUAL_CF_KIND_END`），标记标签形如 `If-0 true`、`End-0`，序号在同一张图内递增。

---

## 构建选项

### CParameterFormatOptions

参数标签文本的格式化选项：

| 字段 | 类型 | 含义 | 取值行为 |
| --- | --- | --- | --- |
| `mode` | `uint32_t` | 参数显示模式。 | 取 `PARAM_MODE_*` 常量之一；未知值回退为数值模式。 |
| `decimal_precision` | `uintptr_t` | 定点与科学计数法的小数位数。 | 按给定值生效。 |
| `scientific_lower_bound` | `double` | 科学计数法下界。 | 落在 `(0, scientific_lower_bound)` 的值使用科学计数法。 |
| `scientific_upper_bound` | `double` | 科学计数法上界。 | `>= scientific_upper_bound` 的值使用科学计数法。 |
| `pi_tolerance` | `double` | π 分数匹配容差。 | `value / pi` 与有理分数的距离在该容差内时按分数显示。 |
| `pi_max_denominator` | `int64_t` | π 分数匹配的最大分母。 | 分母超过该值的分数不参与匹配。 |

### CVisualBuildOptions

可视化 IR 的构建选项，只影响构建阶段，由文本与图形后端共享；`options` 传 NULL 时使用库默认值：

| 字段 | 类型 | 含义 | 取值行为 |
| --- | --- | --- | --- |
| `decompose_circuit_gates` | `uint8_t` | 构建前是否展开线路门定义。 | `0` 不展开（默认），`1` 展开。展开失败时构建返回 NULL。 |
| `reserve_full_span_for_multi_qubit` | `uint8_t` | 是否为多比特操作保留整段下标（`min..=max`）。 | `0` 只保留操作自身的下标，`1` 保留整段（默认）。 |
| `parameter_format` | `struct CParameterFormatOptions` | 参数标签格式化选项，按值嵌入。 | 字段表见上节。 |

---

## 构建与释放

### build_visual_circuit(circuit, options)

把线路构建为可视化 IR。

```c
struct CVisualCircuit *build_visual_circuit(const struct CCircuit *circuit,
                                           const struct CVisualBuildOptions *options);
```

参数：

- `circuit` (`const CCircuit*`)：待构建的线路。
- `options` (`const CVisualBuildOptions*`)：构建选项；NULL 时使用库默认值。

返回：成功返回新建的 `CVisualCircuit*`（用 `visual_circuit_free` 释放）；NULL 输入或构建失败（展开线路门失败、操作引用未知量子比特、符号参数索引越界）时返回 NULL。IR 的量子比特数等于线路的量子比特数。

### visual_circuit_free(ptr)

释放 `CVisualCircuit` 句柄，允许传 NULL。

---

## 线路级查询

### visual_circuit_num_qubits(ptr) / visual_circuit_num_operations(ptr) / visual_circuit_num_columns(ptr)

分别返回量子比特数、操作数与被占用的列数；NULL 句柄返回 0。

### visual_circuit_qubits_len(ptr) / visual_circuit_qubits(ptr, buffer, len)

两步式读取按显示顺序排列的量子比特编号：`*_len` 返回条目数（NULL 返回 0），填充接口把编号拷入 `buffer`。

```c
uintptr_t visual_circuit_qubits_len(const struct CVisualCircuit *ptr);
int32_t visual_circuit_qubits(const struct CVisualCircuit *ptr,
                              uint32_t *buffer,
                              uintptr_t len);
```

返回（填充接口）：0 成功；-1 句柄为 NULL（或 `buffer` 为 NULL 而条目数非零）；-8 `len` 小于条目数。

---

## 操作级查询

以下接口均按操作下标 `index` 访问 `VisualCircuit` 的操作列表（构建时的线路顺序）。`index` 越界或句柄为 NULL 时的返回值见各接口说明。

### visual_circuit_operation_label(ptr, index)

返回操作的主显示标签，堆上 C 字符串，用 `cqlib_string_free` 释放；NULL 输入或越界返回 NULL。

```c
char *visual_circuit_operation_label(const struct CVisualCircuit *ptr,
                                     uintptr_t index);
```

### visual_circuit_operation_column(ptr, index)

返回操作排入的列序号；NULL 输入或越界返回 `UINTPTR_MAX`。

### visual_circuit_operation_span_cols(ptr, index)

返回操作保留的逻辑列数；NULL 输入或越界返回 `UINTPTR_MAX`。普通操作为 `1`，控制流块按子线路的列数加上分隔与边距计算。

### visual_circuit_operation_is_span_box(ptr, index)

返回操作是否按跨多条量子比特线的整体块绘制：`1` 是、`0` 否；NULL 输入或越界返回 `-1`。自定义酉门、线路门与存储指令为 `1`。

### visual_circuit_operation_lanes_len(ptr, index) / visual_circuit_operation_lanes(ptr, index, buffer, len)

两步式读取操作的操作数下标（`lanes`）：按操作的操作数顺序给出所涉量子比特在显示序列中的下标。共享同一量子比特的操作必须排在相邻列，落在不同量子比特上的操作可以共用一列。

```c
uintptr_t visual_circuit_operation_lanes_len(const struct CVisualCircuit *ptr,
                                             uintptr_t index);
int32_t visual_circuit_operation_lanes(const struct CVisualCircuit *ptr,
                                       uintptr_t index,
                                       uintptr_t *buffer,
                                       uintptr_t len);
```

返回（填充接口）：0 成功；-1 句柄为 NULL（或 `buffer` 为 NULL 而条目数非零）；-8 `len` 小于条目数。

### visual_circuit_operation_covered_lanes_len(ptr, index) / visual_circuit_operation_covered_lanes(ptr, index, buffer, len)

两步式读取操作在本列保留的下标（`covered_lanes`）：为避免重叠而保留的量子比特下标。控制流块保留全部下标；屏障在 `lanes` 为空时保留全部下标；其余操作在 `lanes` 为空时保留 `[0]`。涉及多于一个下标时，若 `reserve_full_span_for_multi_qubit` 为 `1` 则保留 `lanes` 的 `min..=max` 整段，否则保留 `lanes` 本身。

```c
uintptr_t visual_circuit_operation_covered_lanes_len(const struct CVisualCircuit *ptr,
                                                     uintptr_t index);
int32_t visual_circuit_operation_covered_lanes(const struct CVisualCircuit *ptr,
                                               uintptr_t index,
                                               uintptr_t *buffer,
                                               uintptr_t len);
```

返回（填充接口）：0 成功；-1 句柄为 NULL（或 `buffer` 为 NULL 而条目数非零）；-8 `len` 小于条目数。

### visual_circuit_operation_params_len(ptr, index) / visual_circuit_operation_params(ptr, index, out, len)

两步式读取操作已格式化的参数标签：`*_len` 返回条目数（NULL 输入或越界返回 0），填充接口把每个标签写为新建的堆上 C 字符串，逐个用 `cqlib_string_free` 释放。

```c
uintptr_t visual_circuit_operation_params_len(const struct CVisualCircuit *ptr,
                                               uintptr_t index);
int32_t visual_circuit_operation_params(const struct CVisualCircuit *ptr,
                                        uintptr_t index,
                                        char **out,
                                        uintptr_t len);
```

返回（填充接口）：0 成功；-1 句柄为 NULL（或 `out` 为 NULL 而条目数非零）；-8 `len` 小于条目数；-3 标签含内嵌 NUL、无法构造 C 字符串。

### visual_circuit_operation_style(ptr, index)

返回操作的绘制样式，取 `VISUAL_OP_STYLE_*` 常量之一；NULL 输入或越界返回 `UINT32_MAX`。

```c
uint32_t visual_circuit_operation_style(const struct CVisualCircuit *ptr,
                                        uintptr_t index);
```

### visual_circuit_operation_num_controls(ptr, index)

返回操作的控制量子比特数；仅当样式为 `VISUAL_OP_STYLE_CONTROLLED` 时有意义。NULL 输入、越界或非受控样式返回 0。

### visual_circuit_operation_control_flow_kind(ptr, index)

返回控制流操作的类别，取 `VISUAL_CF_KIND_*` 常量之一；仅当样式为 `VISUAL_OP_STYLE_CONTROL_FLOW` 时有意义。NULL 输入、越界或非控制流样式返回 `UINT32_MAX`。

```c
uint32_t visual_circuit_operation_control_flow_kind(const struct CVisualCircuit *ptr,
                                                     uintptr_t index);
```

### 样式与标签的归类规则

构建阶段按下表把指令归类为样式与标签：

| 指令 | 样式 | 标签 |
| --- | --- | --- |
| 标准门 `SWAP` | `VISUAL_OP_STYLE_SWAP` | `SWAP` |
| 标准门 `CZ` | `VISUAL_OP_STYLE_CZ` | `CZ` |
| 标准门且控制位数大于 0 | `VISUAL_OP_STYLE_CONTROLLED` | 目标门名 |
| 其余标准门 | `VISUAL_OP_STYLE_GATE` | 门名 |
| 多控门，控制位为 `1` 且基门为 `Z` | `VISUAL_OP_STYLE_CZ` | `CZ` |
| 其余多控门 | `VISUAL_OP_STYLE_CONTROLLED` | 基门门名 |
| 自定义酉门 | `VISUAL_OP_STYLE_GATE`，span box | 酉门标签，为空时 `Unitary` |
| 线路门 | `VISUAL_OP_STYLE_GATE`，span box | 线路门名称，为空时 `Gate` |
| 屏障 | `VISUAL_OP_STYLE_BARRIER` | `B` |
| 测量 | `VISUAL_OP_STYLE_MEASURE` | `M` |
| 复位 | `VISUAL_OP_STYLE_RESET` | `R` |
| 延时 | `VISUAL_OP_STYLE_DELAY` | `D` |
| 经典控制流 | `VISUAL_OP_STYLE_CONTROL_FLOW` | `IF`、`WH`、`FOR`、`SW`、`Break` 或 `Continue` |
| 测量到位指令 | `VISUAL_OP_STYLE_MEASURE` | `M` |
| 存储指令 | `VISUAL_OP_STYLE_GATE`，span box | `STORE` |

门名按标准门口径缩写：`SDG` 显示为 `SD`，`TDG` 显示为 `TD`，`Phase` 与 `GPhase` 显示为 `P`。受控门的标签取其目标门名：`CX` 与 `CCX` 为 `X`，`CY` 为 `Y`，`CRX`、`CRY`、`CRZ` 分别为 `RX`、`RY`、`RZ`，其余情形去掉门名开头的控制前缀。

---

## 绘制

### draw_text_from_visual(visual, options)

把已构建的可视化 IR 绘制为 UTF-8 文本线路图（框线字符）。

```c
char *draw_text_from_visual(const struct CVisualCircuit *visual,
                            const struct TextDrawerOptionsC *options);
```

参数：

- `visual` (`const CVisualCircuit*`)：可视化 IR 句柄。
- `options` (`const TextDrawerOptionsC*`)：绘制选项（字段表见 [文本线路图](1_draw_text.md)）；NULL 时使用库默认值。

返回：堆上 C 字符串，用 `cqlib_string_free` 释放；失败返回 NULL。

### draw_figure_from_visual(visual, options)

把已构建的可视化 IR 绘制为 SVG 图形。

```c
char *draw_figure_from_visual(const struct CVisualCircuit *visual,
                              const struct FigureDrawerOptionsC *options);
```

参数：

- `visual` (`const CVisualCircuit*`)：可视化 IR 句柄。
- `options` (`const FigureDrawerOptionsC*`)：绘制选项（字段表见 [SVG 线路图](2_draw_figure.md)）；NULL 时使用库默认值。

返回：堆上 C 字符串（SVG 标记），用 `cqlib_string_free` 释放；失败返回 NULL。

---

## 示例

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit *circuit = circuit_new(2);
    circuit_h(circuit, 0);
    circuit_cx(circuit, 0, 1);

    /* 1. Build the IR with default options */
    struct CVisualCircuit *visual = build_visual_circuit(circuit, NULL);
    if (visual == NULL) {
        circuit_free(circuit);
        return 1;
    }

    /* 2. Circuit-level queries */
    uintptr_t num_qubits = visual_circuit_num_qubits(visual);      /* 2 */
    uintptr_t num_ops = visual_circuit_num_operations(visual);     /* 2 */
    uintptr_t num_cols = visual_circuit_num_columns(visual);       /* 2 */

    /* 3. Two-step read of the qubit ids in display order */
    uintptr_t qubits_len = visual_circuit_qubits_len(visual);
    uint32_t *qubits = malloc(qubits_len * sizeof(uint32_t));
    if (visual_circuit_qubits(visual, qubits, qubits_len) == 0) {
        /* qubits[0] == 0, qubits[1] == 1 */
    }
    free(qubits);

    /* 4. Operation-level queries */
    char *label = visual_circuit_operation_label(visual, 0);
    if (label != NULL) {
        printf("label: %s\n", label);  /* "H" */
        cqlib_string_free(label);
    }
    uintptr_t column = visual_circuit_operation_column(visual, 1);  /* 1 */
    uint32_t style = visual_circuit_operation_style(visual, 1);      /* CONTROLLED */
    uintptr_t num_controls = visual_circuit_operation_num_controls(visual, 1);  /* 1 */

    uintptr_t lanes_len = visual_circuit_operation_lanes_len(visual, 1);
    uintptr_t *lanes = malloc(lanes_len * sizeof(uintptr_t));
    if (visual_circuit_operation_lanes(visual, 1, lanes, lanes_len) == 0) {
        /* lanes[0] == 0, lanes[1] == 1 */
    }
    free(lanes);

    /* 5. Out-of-bounds access fails gracefully */
    if (visual_circuit_operation_label(visual, 2) == NULL) {
        printf("index 2 out of range\n");
    }

    /* 6. Draw the pre-built IR with both backends */
    char *text = draw_text_from_visual(visual, NULL);
    if (text != NULL) {
        printf("%s\n", text);
        cqlib_string_free(text);
    }
    char *svg = draw_figure_from_visual(visual, NULL);
    if (svg != NULL) {
        cqlib_string_free(svg);
    }

    /* 7. Explicit build options: pi-fraction parameter labels */
    struct CVisualBuildOptions options;
    memset(&options, 0, sizeof(options));
    options.reserve_full_span_for_multi_qubit = 1;
    options.parameter_format.mode = PARAM_MODE_PI_FRACTION_PREFERRED;
    options.parameter_format.decimal_precision = 2;
    options.parameter_format.scientific_lower_bound = 1e-3;
    options.parameter_format.scientific_upper_bound = 1e4;
    options.parameter_format.pi_tolerance = 1e-3;
    options.parameter_format.pi_max_denominator = 16;
    struct CVisualCircuit *visual2 = build_visual_circuit(circuit, &options);
    if (visual2 != NULL) {
        visual_circuit_free(visual2);
    }

    visual_circuit_free(visual);
    circuit_free(circuit);
    return 0;
}
```

直接从线路绘制（不经 IR）见 [文本线路图](1_draw_text.md) 与 [SVG 线路图](2_draw_figure.md)；把 SVG 标记字符串写出到文件见 [落盘与输出](5_render_to_file.md)。
