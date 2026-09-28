# 量子态可视化（C）

本页覆盖量子态的可视化：布洛赫向量图 `plot_bloch_vector`、约化布洛赫向量图 `plot_bloch_multivector`、密度矩阵图 `plot_state_city`、泡利基期望值图 `plot_state_paulivec`，绘图选项 `CStatePlotOptions`，以及数值化的状态检查接口 `local_bloch_vectors` 与 `state_to_density_matrix`。绘图入口返回堆上 SVG 标记字符串，用 `cqlib_string_free` 释放；数值接口按两步式数组约定写入调用方分配的缓冲区。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

态类入口各有两个变体：基础名接受 `CStatevector`（纯态按 `ρ = |ψ⟩⟨ψ|` 展开为密度矩阵），`_density_matrix` 后缀接受 `CDensityMatrix`（直接使用其矩阵数据），两者共用同一套 `CStatePlotOptions`。态句柄的构造见 [Statevector](../3_qis/1_statevector.md) 与 [DensityMatrix](../3_qis/2_density_matrix.md)。

---

## plot_bloch_vector(x, y, z, options)

绘制单个布洛赫向量。

```c
char *plot_bloch_vector(double x, double y, double z,
                       const struct CStatePlotOptions *options);
```

参数：

- `x`、`y`、`z` (`double`)：布洛赫坐标 `(x, y, z)`，分量通常落在 `[-1, 1]`。模长大于 `1` 的向量会被投影回单位球面，落在球内的向量保持不变。
- `options` (`const CStatePlotOptions*`)：绘图选项；NULL 时使用库默认值。

返回：堆上 C 字符串（SVG 标记），用 `cqlib_string_free` 释放；失败返回 NULL。向量标签固定为 `q0`。

---

## plot_bloch_multivector(state, options) / plot_bloch_multivector_density_matrix(state, options)

为每个量子比特绘制一个约化布洛赫向量。每个向量由该比特的约化密度矩阵计算得到，标签按量子比特编号写成 `q0`、`q1` 等；多个球排成网格，每行最多 4 个。

```c
char *plot_bloch_multivector(const struct CStatevector *state,
                             const struct CStatePlotOptions *options);
char *plot_bloch_multivector_density_matrix(const struct CDensityMatrix *state,
                                            const struct CStatePlotOptions *options);
```

参数：

- `state`：输入态句柄（前者 `const CStatevector*`，后者 `const CDensityMatrix*`）。
- `options` (`const CStatePlotOptions*`)：绘图选项；NULL 时使用库默认值。

返回：堆上 C 字符串（SVG 标记），用 `cqlib_string_free` 释放；失败返回 NULL。

失败场景：

- `state` 为 NULL。
- `options` 中出现非法 UTF-8 字符串或 `color` 数组内含 NULL 元素。
- 状态缓冲长度与量子比特数不符，或量子比特数过大导致矩阵维度溢出。

---

## plot_state_city(state, options) / plot_state_city_density_matrix(state, options)

绘制密度矩阵的实部与虚部，每部分一个面板，面板标题为 `Re[rho]` 与 `Im[rho]`。当所有元素的虚部均为零时，SVG 中不生成虚部面板。每个单元的面积随 `sqrt(|value| / max_abs)` 缩放，使小幅度元素仍可见；`max_abs` 取矩阵元素模长的最大值，下限为 `1e-12`。柱体填充不透明度由 `CStatePlotOptions.alpha` 控制。

```c
char *plot_state_city(const struct CStatevector *state,
                      const struct CStatePlotOptions *options);
char *plot_state_city_density_matrix(const struct CDensityMatrix *state,
                                     const struct CStatePlotOptions *options);
```

参数：

- `state`：输入态句柄（前者 `const CStatevector*`，后者 `const CDensityMatrix*`）。
- `options` (`const CStatePlotOptions*`)：绘图选项；NULL 时使用库默认值。

返回：堆上 C 字符串（SVG 标记），用 `cqlib_string_free` 释放；失败返回 NULL。

失败场景：与 [plot_bloch_multivector](#plot_bloch_multivectorstate-options--plot_bloch_multivector_density_matrixstate-options) 相同。

---

## plot_state_paulivec(state, options) / plot_state_paulivec_density_matrix(state, options)

把泡利基期望值绘制为柱状图。标签为 `I`、`X`、`Y`、`Z` 的张量积，共 `4^n` 条，其中 `n` 为量子比特数；非负柱与负值柱使用不同颜色。柱数不超过 `64` 时才绘制旋转的横轴标签。

```c
char *plot_state_paulivec(const struct CStatevector *state,
                          const struct CStatePlotOptions *options);
char *plot_state_paulivec_density_matrix(const struct CDensityMatrix *state,
                                         const struct CStatePlotOptions *options);
```

参数：

- `state`：输入态句柄（前者 `const CStatevector*`，后者 `const CDensityMatrix*`）。
- `options` (`const CStatePlotOptions*`)：绘图选项；NULL 时使用库默认值。

返回：堆上 C 字符串（SVG 标记），用 `cqlib_string_free` 释放；失败返回 NULL。

失败场景：与 [plot_bloch_multivector](#plot_bloch_multivectorstate-options--plot_bloch_multivector_density_matrixstate-options) 相同。

---

## local_bloch_vectors_len(state) / local_bloch_vectors(state, out, len)

计算态矢量每个量子比特的约化布洛赫向量，是 [plot_bloch_multivector](#plot_bloch_multivectorstate-options--plot_bloch_multivector_density_matrixstate-options) 图的数值版本，结果以数据形式返回而非绘图。

```c
uintptr_t local_bloch_vectors_len(const struct CStatevector *state);
int32_t local_bloch_vectors(const struct CStatevector *state, double *out, uintptr_t len);
```

两步式数组输出：`local_bloch_vectors_len` 返回所需缓冲区长度 `3 * num_qubits`（NULL 返回 0）；分配相应数量的 `double` 后，把缓冲区连同相同的 `len` 传给 `local_bloch_vectors`。输出按 `[x0, y0, z0, x1, y1, z1, ...]` 扁平排列：第 `q * 3 .. q * 3 + 3` 项为量子比特 `q` 约化态的 `<X>`、`<Y>`、`<Z>` 期望值。

参数：

- `state` (`const CStatevector*`)：输入态矢量。
- `out` (`double*`)：长度为 `len` 的输出缓冲区。
- `len` (`uintptr_t`)：缓冲区长度，必须等于 `local_bloch_vectors_len(state)`。

错误码（填充接口）：

| 值 | 场景 |
| --- | --- |
| `0` | 成功。 |
| `-1` | `state` 或 `out` 为 NULL。 |
| `-8` | `len` 与 `local_bloch_vectors_len(state)` 不符，或状态数据非法。 |

贝尔态的每个约化布洛赫向量都为零，六个值全为 `0`；比特 0 处于 `|+⟩`、比特 1 处于 `|0⟩` 时，缓冲区为 `[1, 0, 0, 0, 0, 1]`。

---

## state_to_density_matrix_len(state) / state_to_density_matrix(state, out, len)

把态矢量转换为密度矩阵 `ρ = |ψ⟩⟨ψ|`，是 [plot_state_city](#plot_state_citystate-options--plot_state_city_density_matrixstate-options) 图的数值版本，结果以数据形式返回。

```c
uintptr_t state_to_density_matrix_len(const struct CStatevector *state);
int32_t state_to_density_matrix(const struct CStatevector *state, Complex64 *out, uintptr_t len);
```

两步式数组输出：`state_to_density_matrix_len` 返回所需缓冲区长度 `4^num_qubits`（NULL 返回 0）；`state_to_density_matrix` 把相应数量的 `Complex64` 值拷入 `out`，按行主序排列为 `N × N` 矩阵（`N = 2^num_qubits`）：元素 `(row, col)` 存储在下标 `row * N + col` 处。

参数：

- `state` (`const CStatevector*`)：输入态矢量。
- `out` (`Complex64*`)：长度为 `len` 的输出缓冲区。
- `len` (`uintptr_t`)：缓冲区长度，必须等于 `state_to_density_matrix_len(state)`。

错误码（填充接口）：

| 值 | 场景 |
| --- | --- |
| `0` | 成功。 |
| `-1` | `state` 或 `out` 为 NULL。 |
| `-8` | `len` 与 `state_to_density_matrix_len(state)` 不符，或状态数据非法。 |

两比特贝尔态的 `4 × 4` 矩阵在对角角落与 `(0,3)`、`(3,0)` 相干项处为 `0.5`，其余元素均为 `0`。

---

## CStatePlotOptions

量子态绘图选项，按值填充；`options` 传 NULL 时使用库默认值。选项中的字符串须为有效 UTF-8，否则绘图入口返回 NULL。

| 字段 | 类型 | 含义 | 取值行为 |
| --- | --- | --- | --- |
| `title` | `const char*` | 图标题。 | NULL 不绘制标题。 |
| `color` | `const char* const*` | 配色数组，长度由 `color_len` 给出。 | NULL 或 `color_len == 0` 使用各绘图族的默认配色；数组内含 NULL 元素时返回 NULL。 |
| `color_len` | `uintptr_t` | `color` 的条目数。 | `0` 视为未提供配色。 |
| `alpha` | `double` | `plot_state_city` 柱体的填充不透明度。 | 按给定值生效；库默认值为 `1.0`（仅当 `options` 为 NULL 时应用，显式传入结构体时零值即完全透明）。 |
| `reverse_bits` | `uint8_t` | 是否反转显示的计算基位序。 | `0` 原序，`1` 反序。置为 `1` 时，密度矩阵图的行列标签按相反顺序排列，矩阵元素按位序映射回存储位置；布洛赫图与泡利图的标签顺序随之反转。 |
| `has_figsize` | `uint8_t` | `fig_width` / `fig_height` 是否携带自定义图尺寸。 | `0` 忽略宽高（各绘图族按自身布局计算画布尺寸），非 `0` 生效。 |
| `fig_width` | `double` | 图宽，单位为类英寸单位，按每单位 `100` 像素换算为 SVG 像素。 | `has_figsize == 0` 时忽略。给定尺寸时，密度矩阵图与泡利图的宽、高下限分别为 `4` 与 `3`，布洛赫图的宽、高下限均为 `3`。 |
| `fig_height` | `double` | 图高，单位与换算同 `fig_width`。 | `has_figsize == 0` 时忽略；下限规则同 `fig_width`。 |

`color` 的用法因绘图族而异：`plot_state_paulivec` 取第一个元素作为非负柱颜色（缺省 `#4569d4`）、第二个元素作为负值柱颜色（缺省 `#d64b5f`）；其余绘图族使用固定配色。

---

## 示例

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build a Bell state */
    double inv = 0.7071067811865476;
    Complex64 amplitudes[4] = {
        {inv, 0.0}, {0.0, 0.0}, {0.0, 0.0}, {inv, 0.0},
    };
    struct CStatevector *state = statevector_from_state(2, amplitudes, 4);
    if (state == NULL) {
        return 1;
    }

    /* 2. Plot with default options; every entry returns SVG markup */
    char *svg = plot_bloch_multivector(state, NULL);
    if (svg != NULL) {
        printf("%s\n", svg);
        cqlib_string_free(svg);
    }

    /* 3. Plot with explicit options */
    struct CStatePlotOptions options;
    memset(&options, 0, sizeof(options));
    options.title = "Bell state";
    options.alpha = 0.75;
    options.reverse_bits = 1;
    options.has_figsize = 1;
    options.fig_width = 4.0;
    options.fig_height = 3.0;
    svg = plot_state_city(state, &options);
    if (svg != NULL) {
        printf("%s\n", svg);
        cqlib_string_free(svg);
    }

    /* 4. Density-matrix input works through the _density_matrix variants */
    struct CDensityMatrix *rho = density_matrix_from_state(2, amplitudes, 4);
    if (rho != NULL) {
        svg = plot_state_paulivec_density_matrix(rho, NULL);
        if (svg != NULL) {
            cqlib_string_free(svg);
        }
        density_matrix_free(rho);
    }

    /* 5. Single Bloch vector: vectors longer than the unit
       sphere are clamped, not rejected */
    svg = plot_bloch_vector(0.0, 0.0, 1.0, NULL);
    if (svg != NULL) {
        cqlib_string_free(svg);
    }

    /* 6. Numeric state inspection via the two-step array pattern */
    uintptr_t bloch_len = local_bloch_vectors_len(state);      /* 6 = 3 * 2 */
    double *bloch = malloc(bloch_len * sizeof(double));
    local_bloch_vectors(state, bloch, bloch_len);              /* all zeros for the Bell state */
    free(bloch);

    uintptr_t rho_len = state_to_density_matrix_len(state);    /* 16 = 4^2 */
    Complex64 *rho_buf = malloc(rho_len * sizeof(Complex64));
    state_to_density_matrix(state, rho_buf, rho_len);          /* 4x4 matrix, corners 0.5 */
    free(rho_buf);

    statevector_free(state);
    return 0;
}
```

同一纯态分别经 `plot_state_paulivec`（态矢径）与 `plot_state_paulivec_density_matrix`（密度矩阵径）绘制时，两者输出的 SVG 标记一致。把绘制结果写出为文件见 [落盘与输出](5_render_to_file.md)。
