# Pauli（C）

`CPauliString` 是 Pauli 串的不透明句柄：N 个比特上各带一个 `I`/`X`/`Y`/`Z` 算子并附一个相位。单比特算子用 `PAULI_*` 常量表示，串的文本格式为 `[+|-][i|j]<算子序列>`。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 常量与格式约定

单比特 Pauli 算子标签：

| 常量 | 值 | 算子 |
| --- | --- | --- |
| `PAULI_I` | 0 | 单位算符 I。 |
| `PAULI_X` | 1 | Pauli X。 |
| `PAULI_Y` | 2 | Pauli Y。 |
| `PAULI_Z` | 3 | Pauli Z。 |

文本格式与比特序：

- 格式为 `[+|-][i|j]<算子序列>`，算子为 `I`、`X`、`Y`、`Z`；相位可取 `+`、`-`、`+i`（可写作 `+j`）、`-i`（可写作 `-j`），省略相位时默认 `+`。
- 比特序与阅读顺序相反：**字符串的第一个字符对应最高比特下标**。例如 `"XZI"` 解析后比特 2 为 `X`、比特 1 为 `Z`、比特 0 为 `I`；`pauli_string_to_string` 输出时沿用同一约定（如 `"+XZI"`）。

---

## 单比特算子乘法

### pauli_mul_with_phase(left, right, out_pauli, out_phase)

```c
int32_t pauli_mul_with_phase(uint8_t left, uint8_t right, uint8_t *out_pauli, Complex64 *out_phase);
```

将两个单比特 Pauli 算子按 `left * right` 相乘，把结果算子标签（`PAULI_I/X/Y/Z` 之一）写入 `*out_pauli`，把 Pauli 群相位因子（`1`、`i`、`-1`、`-i` 之一）写入 `*out_phase`；例如 `X * Z = -iY`，`Z * X = iY`。

返回：`0` 成功；`-1` 输出指针为 NULL；`-8` 输入标签不是 `PAULI_I/X/Y/Z` 之一。

---

## 构造与释放

### pauli_string_new(num_qubits)

构造全 `I` 的 Pauli 串。

参数：

- `num_qubits` (`uintptr_t`)：量子比特数。

返回：成功返回新建的 `CPauliString*`；`num_qubits` 为 0 时返回 NULL。

### pauli_string_free(ptr)

释放 Pauli 串句柄，允许传 NULL。

### pauli_string_parse(source)

按 `[+|-][i|j]<算子序列>` 格式解析 C 字符串（如 `"XYZ"`、`"+ZZI"`、`"-iZII"`）。

返回：成功返回新建的 `CPauliString*`；NULL 输入、非法 UTF-8 或解析失败时返回 NULL。

---

## 算子读写

### pauli_string_set_pauli(ptr, idx, pauli)

把比特 `idx` 上的算子设为 `pauli`（`PAULI_*` 之一）。

返回：0 成功；-1 空指针；-2 `idx` 越界；-8 非法算子标签。

### pauli_string_get_pauli(ptr, idx)

读取比特 `idx` 上的算子。

返回：`PAULI_I`/`PAULI_X`/`PAULI_Y`/`PAULI_Z` 之一；NULL 输入或越界返回 255（`0xFF`）。

### pauli_string_try_get_pauli(ptr, idx, out)

非 panic 版本的算子读取：把算子标签写到 `*out`。

返回：0 成功；-1 空指针；-2 越界。

### pauli_string_try_set_pauli(ptr, idx, pauli)

非 panic 版本的算子写入：`pauli` 必须是 `PAULI_*` 之一。

返回：0 成功；-1 空指针；-2 越界；-8 非法算子标签。

---

## 属性查询

### pauli_string_num_qubits(ptr)

返回 Pauli 串作用的比特数；NULL 返回 0。

### pauli_string_to_string(ptr)

把 Pauli 串格式化为 `"+XYZ"` 形式的 C 字符串（首个字符对应最高比特）。

返回：堆上字符串，用 `cqlib_string_free` 释放；NULL 输入返回 NULL。

### pauli_string_x_mask(ptr)

返回 X 分量位掩码：比特 `i` 带 `X` 或 `Y` 算子时位 `i` 置 1；NULL 返回 0。

### pauli_string_z_mask(ptr)

返回 Z 分量位掩码：比特 `i` 带 `Z` 或 `Y` 算子时位 `i` 置 1；NULL 返回 0。

### pauli_string_y_phase(ptr, out)

计算串中 n 个 `Y` 算子贡献的相位因子 `i^n`。

返回：0 成功（复数值写入 `*out`）；-1 空指针。

### pauli_string_support_len(ptr) / pauli_string_support(ptr, buffer, len)

两步式读取支集：`*_len` 返回非单位算子的比特个数（NULL 返回 0），填充接口把支集下标（升序）作为 `uint32_t` 拷入 `buffer`。

返回（填充接口）：0 成功；-1 空指针；-8 `len` 与 `pauli_string_support_len` 不符。

---

## 对易与矩阵

### pauli_string_commutes_with(ptr, other)

判断两个 Pauli 串是否对易。

返回：1 对易；0 不对易；-1 空指针；-8 两者比特数不同。

### pauli_string_matrix_len(ptr)

返回稠密矩阵的边长 `2^N`；NULL 返回 0。

### pauli_string_matrix(ptr, buffer, len)

把 `2^N × 2^N` 稠密矩阵按行主序拷入 `buffer`（`Complex64` 值）。`len` 必须等于 `pauli_string_matrix_len(ptr) * pauli_string_matrix_len(ptr)`。

返回：0 成功；-1 空指针；-8 `len` 与矩阵元素数不符。

---

## 期望值

### pauli_string_expectation(ptr, bitstrings, probs, len, out)

在计算基概率分布上计算 Pauli 串的期望值 `⟨P⟩ = Σ_s p(s)·⟨s|P|s⟩`，分布以并行数组给出：`bitstrings[i]` 与 `probs[i]` 一一对应。

约定：

- 每个比特串采用小端约定（最右字符对应比特 0），且必须恰好为 `num_qubits` 个 `'0'`/`'1'` 字符。
- 串中含 `X` 或 `Y` 算子时结果恒为 `0.0`。
- 串中仅含 `I` 与 `Z` 时按公式 `phase × Σ_s p(s) × (-1)^(Σ_i z[i]·s[i])` 求值，其中 `phase` 为串的全局相位，`z[i]` 为比特 `i` 的 Z 分量。

参数：

- `ptr` (`const struct CPauliString*`)：Pauli 串句柄。
- `bitstrings` (`const char* const*`)：NUL 结尾比特串数组；`len` 为 0 时可为 NULL。
- `probs` (`const double*`)：与 `bitstrings` 平行的概率数组；`len` 为 0 时可为 NULL。
- `len` (`uintptr_t`)：分布条目数量。
- `out` (`double*`)：输出指针。

返回：0 成功；-1 `ptr` 或 `out` 为 NULL，或 `len > 0` 时任一数组为 NULL；-4 比特串不是合法 UTF-8；-7 比特串长度不符或含 `'0'`/`'1'` 之外的字符；-8 比特串数组含 NULL 元素。

---

## 示例

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. 解析：首字符对应最高比特，"XZI" -> 比特2=X、比特1=Z、比特0=I */
    struct CPauliString *ps = pauli_string_parse("XZI");
    uintptr_t n = pauli_string_num_qubits(ps);      /* 3 */
    uint8_t p2 = pauli_string_get_pauli(ps, 2);     /* PAULI_X (1) */
    uint8_t p0 = pauli_string_get_pauli(ps, 0);     /* PAULI_I (0) */

    /* 2. 掩码与支集 */
    uintptr_t x_mask = pauli_string_x_mask(ps);     /* bit 2 -> 0b100 */
    uintptr_t z_mask = pauli_string_z_mask(ps);     /* bit 1 -> 0b010 */
    uintptr_t support_len = pauli_string_support_len(ps);  /* 2 */
    uint32_t support[2];
    pauli_string_support(ps, support, support_len); /* {1, 2} */

    /* 3. 文本输出与对易判断 */
    char *text = pauli_string_to_string(ps);        /* "+XZI" */
    struct CPauliString *zz = pauli_string_parse("ZZI");
    int32_t commutes = pauli_string_commutes_with(ps, zz);  /* 0（比特 2 上 X 与 Z 反对易） */

    /* 4. 两步式读取 8x8 稠密矩阵 */
    uintptr_t dim = pauli_string_matrix_len(ps);    /* 8 */
    Complex64 *matrix = malloc(dim * dim * sizeof(Complex64));
    pauli_string_matrix(ps, matrix, dim * dim);
    free(matrix);

    cqlib_string_free(text);
    pauli_string_free(zz);
    pauli_string_free(ps);
    return 0;
}
```
