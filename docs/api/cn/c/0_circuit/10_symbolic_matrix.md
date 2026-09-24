# 符号矩阵

本页介绍 C API 中的符号矩阵函数：符号复数、符号矩阵的构造、形状与元素查询、化简、符号替换、参数绑定求值、就地门应用（`apply_*` 家族），以及线路级和矩阵级的全局相位等价性检查。错误码与内存管理约定见 [Overview](../0_overview.md)。

---

## 核心概念

符号矩阵（`CSymbolicMatrix*` 不透明句柄）的每个元素都是符号复数：实部和虚部为参数表达式，可包含未绑定的符号参数。与数值矩阵转换不同，符号矩阵转换保留线路中的参数表达式，后续可通过绑定字符串代入具体数值。

数值求值结果以 `Complex64` 结构输出：

```c
typedef struct Complex64 {
  double re;  /* 实部 */
  double im;  /* 虚部 */
} Complex64;
```

所有返回 `CSymbolicMatrix*` 的函数都分配新句柄，用 `symbolic_matrix_free` 释放；返回 `CSymbolicComplex*` 的函数分配新的符号复数句柄，用 `symbolic_complex_free` 释放。

---

## 符号复数

单个矩阵元素是符号复数（`CSymbolicComplex*` 不透明句柄）：实部和虚部各为一个参数表达式。本节函数是元素级 API，也用于给 `symbolic_matrix_new` 和各 `apply_*` 置换/对角门提供元素。

### symbolic_complex_new(re, im)

由实部和虚部参数表达式构造符号复数。

- `re` (`const CParameter*`)：实部表达式句柄。
- `im` (`const CParameter*`)：虚部表达式句柄。

返回新分配的 `CSymbolicComplex*`；任一输入为 NULL 时返回 NULL。

### symbolic_complex_zero() / symbolic_complex_one() / symbolic_complex_i()

返回加法单位元 `0 + 0i`、乘法单位元 `1 + 0i` 和虚数单位 `0 + 1i`。

无参数，均返回新分配的 `CSymbolicComplex*`。

### symbolic_complex_from_real(re)

由实部表达式构造虚部为零的符号复数。

- `re` (`const CParameter*`)：实部表达式句柄。

返回新分配的 `CSymbolicComplex*`；输入为 NULL 时返回 NULL。

### symbolic_complex_from_complex(re, im)

由一对浮点常量构造符号复数常量。

- `re` (`double`)：实部。
- `im` (`double`)：虚部。

返回新分配的 `CSymbolicComplex*`；任一分量非有限（NaN/Inf）时返回 NULL。

### symbolic_complex_exp_i(theta)

构造 `exp(i*theta)`，即 `cos(theta) + i*sin(theta)`。

- `theta` (`const CParameter*`)：相位表达式句柄。

返回新分配的 `CSymbolicComplex*`；输入为 NULL 时返回 NULL。

### symbolic_complex_re(ptr) / symbolic_complex_im(ptr)

返回实部/虚部表达式的克隆句柄（用 `param_free` 释放）。

- `ptr` (`const CSymbolicComplex*`)：符号复数句柄。

返回新分配的 `CParameter*`；句柄为 NULL 时返回 NULL。

### symbolic_complex_evaluate(ptr, bindings, out)

在参数绑定下把符号复数求值为数值。

- `ptr` (`const CSymbolicComplex*`)：符号复数句柄。
- `bindings` (`const char*`)：绑定字符串，格式为 `"name:value,..."`（传 NULL 表示无绑定）。
- `out` (`Complex64*`)：接收结果。

返回 0 表示成功；-1 表示输入为 NULL；-3 表示存在无法求值的符号；-4 表示绑定字符串格式非法。

### symbolic_complex_simplify(ptr)

对实部和虚部分别执行符号化简，返回化简后的新符号复数。

- `ptr` (`const CSymbolicComplex*`)：原句柄。

返回新分配的 `CSymbolicComplex*`；输入为 NULL 或化简失败时返回 NULL。

### symbolic_complex_replace(ptr, symbol, replacement)

把实部和虚部中出现的符号 `symbol` 全部替换为 `replacement` 表达式，返回替换后的新符号复数。

- `ptr` (`const CSymbolicComplex*`)：原句柄。
- `symbol` (`const char*`)：符号名。
- `replacement` (`const CParameter*`)：替换表达式句柄。

返回新分配的 `CSymbolicComplex*`；任一输入为 NULL 或符号名 UTF-8 非法时返回 NULL。

### symbolic_complex_is_zero_exact(ptr) / symbolic_complex_is_one_exact(ptr)

判断实部和虚部是否**恰好**为零/一（不做化简，仅检查常数结构）。

- `ptr` (`const CSymbolicComplex*`)：符号复数句柄。

返回 1/0；句柄为 NULL 时返回 -1。

### symbolic_complex_simplifies_to_zero(ptr)

判断符号复数化简后是否恰好为零。

- `ptr` (`const CSymbolicComplex*`)：符号复数句柄。

返回 1 表示化简后为零，0 表示不为零；-1 表示句柄为 NULL；-3 表示化简失败。

### symbolic_complex_free(ptr)

释放符号复数句柄。

- `ptr` (`CSymbolicComplex*`)：待释放句柄，允许传 NULL。

无返回值。

---

## 矩阵构造与释放

### symbolic_eye(dim)

创建 `dim x dim` 的符号单位矩阵。

- `dim` (`uintptr_t`)：矩阵维度。

返回新分配的 `CSymbolicMatrix*`；仅在内存分配失败时返回 NULL。

### symbolic_matrix_new(rows, cols, elements, len)

由元素句柄数组构造 `rows x cols` 符号矩阵，元素按行主序排列。

- `rows` (`uintptr_t`)：行数，必须大于 0。
- `cols` (`uintptr_t`)：列数，必须大于 0。
- `elements` (`const CSymbolicComplex* const*`)：`rows * cols` 个符号复数句柄。
- `len` (`uintptr_t`)：数组长度，必须等于 `rows * cols`。

返回新分配的 `CSymbolicMatrix*`；`elements` 为 NULL、长度不匹配、维度为零或任一元素句柄为 NULL 时返回 NULL。

### standard_gate_symbolic_matrix(gate_name, params, len)

按名称构造标准门的符号酉矩阵（例如 `"RX"`、`"RZZ"`；名称集合与 `circuit_multi_control` 相同）。

- `gate_name` (`const char*`)：标准门名称。
- `params` (`const CParameter* const*`)：门参数句柄数组。
- `len` (`uintptr_t`)：参数数量。

返回新分配的 `CSymbolicMatrix*`；输入为 NULL、UTF-8 非法、门名未知或参数数量不匹配时返回 NULL。

### circuit_to_symbolic_matrix(ptr, order, order_len)

计算线路的符号酉矩阵。

- `ptr` (`const CCircuit*`)：目标线路句柄。
- `order` (`const uint32_t*`)：量子比特 id 数组，从最低有效位到最高有效位排列；传 NULL 时按量子比特编号排序。
- `order_len` (`uintptr_t`)：数组长度，0 表示使用默认排序。

返回新分配的 `CSymbolicMatrix*`；输入为 NULL、顺序与量子比特集合不匹配或线路含非酉操作时返回 NULL。

### symbolic_matrix_controlled(base, num_ctrls)

把 `base` 嵌入到更大单位矩阵的右下角分块，为其添加 `num_ctrls` 个控制量子比特。

- `base` (`const CSymbolicMatrix*`)：基础矩阵。
- `num_ctrls` (`uintptr_t`)：控制量子比特数量。

返回新分配的 `CSymbolicMatrix*`；`base` 为 NULL 时返回 NULL。

### symbolic_matrix_free(ptr)

释放符号矩阵句柄。

- `ptr` (`CSymbolicMatrix*`)：待释放句柄，允许传 NULL。

无返回值。

---

## 形状与元素查询

### symbolic_matrix_rows(ptr)

返回矩阵行数。

- `ptr` (`const CSymbolicMatrix*`)：符号矩阵句柄。

返回行数；句柄为 NULL 时返回 0。

### symbolic_matrix_cols(ptr)

返回矩阵列数。

- `ptr` (`const CSymbolicMatrix*`)：符号矩阵句柄。

返回列数；句柄为 NULL 时返回 0。

### symbolic_matrix_element_str(ptr, row, col)

把单个矩阵元素格式化为可读的参数表达式字符串（例如 `"cos(theta*0.5)"`）。

- `ptr` (`const CSymbolicMatrix*`)：符号矩阵句柄。
- `row` (`uintptr_t`)：行下标。
- `col` (`uintptr_t`)：列下标。

返回新分配的 C 字符串（用 `cqlib_string_free` 释放）；输入为 NULL 或下标越界时返回 NULL。

### symbolic_matrix_element(ptr, row, col)

返回 `[row, col]` 元素的克隆句柄（用 `symbolic_complex_free` 释放），可用于逐元素读取符号矩阵。

- `ptr` (`const CSymbolicMatrix*`)：符号矩阵句柄。
- `row` (`uintptr_t`)：行下标。
- `col` (`uintptr_t`)：列下标。

返回新分配的 `CSymbolicComplex*`；句柄为 NULL 或下标越界时返回 NULL。

### symbolic_matrix_evaluate_len(ptr)

返回展平后的元素数量（`rows * cols`），即数值求值所需的缓冲区长度。

- `ptr` (`const CSymbolicMatrix*`)：符号矩阵句柄。

返回元素数量；句柄为 NULL 时返回 0。

---

## 化简与替换

### symbolic_matrix_simplify(ptr)

对矩阵的每个元素执行符号化简，返回化简后的新矩阵。

- `ptr` (`const CSymbolicMatrix*`)：原矩阵句柄。

返回新分配的 `CSymbolicMatrix*`；输入为 NULL 或化简失败时返回 NULL。

### symbolic_matrix_substitute(ptr, names, params, len)

对矩阵中的符号执行同时替换：`names[i]` 是符号名，`params[i]` 是替换表达式。

- `ptr` (`const CSymbolicMatrix*`)：原矩阵句柄。
- `names` (`const char* const*`)：符号名数组。
- `params` (`const CParameter* const*`)：替换表达式句柄数组。
- `len` (`uintptr_t`)：替换数量。

返回新分配的 `CSymbolicMatrix*`；输入为 NULL、UTF-8 非法或替换出错（例如保留前缀）时返回 NULL。

---

## 数值求值

### symbolic_matrix_evaluate(ptr, bindings, buffer, len)

在参数绑定下把矩阵的每个元素求值为数值复数，按行主序写入 `buffer`。

- `ptr` (`const CSymbolicMatrix*`)：符号矩阵句柄。
- `bindings` (`const char*`)：绑定字符串，格式为 `"name:value,name:value"`（传 NULL 表示无绑定）。
- `buffer` (`Complex64*`)：接收结果的数组。
- `len` (`uintptr_t`)：数组长度，必须不小于 `symbolic_matrix_evaluate_len`。

返回 0 表示成功；-1 表示输入为 NULL；-3 表示存在无法求值的符号；-4 表示绑定字符串格式非法；-8 表示 `len` 过小。

---

## 就地门应用（apply_* 家族）

`apply_*` 系列把量子门**原地**作用到符号矩阵上：等价于左乘门算子在目标比特张成的子空间上的表示，形状保持不变。目标比特约束（各函数通用）：

- 矩阵行数必须是 2 的幂，`bits` 中每个比特都必须小于 `log2(rows)`；
- `bits` 数组不能为空、不能有重复；多比特门的门矩阵维度必须等于 `1 << bits_len`。

### 目标比特顺序

`apply_*` 的 `bits` 参数使用系统的 **Little-Endian（最低有效位在前）** 约定：`bits[k]` 对应门矩阵局部行下标的第 `k` 位。对照线路 API：受控门的量子比特按 `[control, target]` 顺序传入（最高有效位在前），`circuit_to_symbolic_matrix` 内部会先反转该序列再应用。因此用 `apply_standard_gate` 复现线路效果时需要**自行传入反转后的顺序**：`circuit_cx(c, 0, 1)` 等价于以 `bits = [1, 0]` 应用 `"CX"`。

### symbolic_matrix_apply_standard_gate(ptr, gate_name, bits, bits_len, params, params_len)

按名称应用标准门（名称集合与 `circuit_multi_control` 相同，如 `"H"`、`"CX"`、`"RX"`），门参数为符号表达式。

- `ptr` (`CSymbolicMatrix*`)：目标矩阵（就地修改）。
- `gate_name` (`const char*`)：标准门名称。
- `bits` (`const uintptr_t*`)：目标比特数组。
- `bits_len` (`uintptr_t`)：目标比特数量。
- `params` (`const CParameter* const*`)：门参数句柄数组，无参门可传 NULL。
- `params_len` (`uintptr_t`)：门参数数量。

返回 0 表示成功；-1 表示矩阵、门名或参数句柄为 NULL；-3 表示门的比特数/参数数量与矩阵不匹配；-4 表示门名 UTF-8 非法；-8 表示门名未知或 `bits` 非法。

### symbolic_matrix_apply_gate(ptr, gate, bits, bits_len)

把符号门矩阵（维度须为 `1 << bits_len`）作用到目标比特。

- `ptr` (`CSymbolicMatrix*`)：目标矩阵（就地修改）。
- `gate` (`const CSymbolicMatrix*`)：门矩阵句柄。
- `bits` (`const uintptr_t*`)：目标比特数组。
- `bits_len` (`uintptr_t`)：目标比特数量。

返回 0 表示成功；-1 表示输入为 NULL；-3 表示门矩阵维度与目标比特不匹配；-8 表示 `bits` 非法。

### symbolic_matrix_apply_gate_num(ptr, gate, gate_dim, bits, bits_len)

`symbolic_matrix_apply_gate` 的数值变体：`gate` 指向 `gate_dim * gate_dim` 个行主序 `Complex64`，`gate_dim` 必须等于 `1 << bits_len`。

- `ptr` (`CSymbolicMatrix*`)：目标矩阵（就地修改）。
- `gate` (`const Complex64*`)：门矩阵数值数组。
- `gate_dim` (`uintptr_t`)：门矩阵维度。
- `bits` (`const uintptr_t*`)：目标比特数组。
- `bits_len` (`uintptr_t`)：目标比特数量。

返回 0 表示成功；-1 表示矩阵或 `gate` 为 NULL；-3 表示门矩阵与目标比特不匹配；-8 表示 `gate_dim` 为 0、不等于 `1 << bits_len`，或 `bits` 非法。

### symbolic_matrix_apply_single_qubit_gate(ptr, gate, bit)

把 `2x2` 符号门作用到单个目标比特。

- `ptr` (`CSymbolicMatrix*`)：目标矩阵（就地修改）。
- `gate` (`const CSymbolicMatrix*`)：`2x2` 门矩阵句柄。
- `bit` (`uintptr_t`)：目标比特。

返回 0 表示成功；-1 表示输入为 NULL；-8 表示门矩阵不是 `2x2`、矩阵行数非 2 的幂或 `bit` 越界。

### symbolic_matrix_apply_single_qubit_gate_num(ptr, gate, bit)

`symbolic_matrix_apply_single_qubit_gate` 的数值变体：`gate` 指向 4 个行主序 `Complex64`。

- `ptr` (`CSymbolicMatrix*`)：目标矩阵（就地修改）。
- `gate` (`const Complex64*`)：4 个门矩阵元素。
- `bit` (`uintptr_t`)：目标比特。

返回 0 表示成功；-1 表示输入为 NULL；-8 表示矩阵行数非 2 的幂或 `bit` 越界。

### symbolic_matrix_apply_two_qubit_gate(ptr, gate, b0, b1)

把 `4x4` 符号门作用到两个目标比特。

- `ptr` (`CSymbolicMatrix*`)：目标矩阵（就地修改）。
- `gate` (`const CSymbolicMatrix*`)：`4x4` 门矩阵句柄。
- `b0`, `b1` (`uintptr_t`)：两个目标比特。

返回 0 表示成功；-1 表示输入为 NULL；-8 表示门矩阵不是 `4x4`、矩阵行数非 2 的幂、比特重复或越界。

### symbolic_matrix_apply_two_qubit_gate_num(ptr, gate, b0, b1)

`symbolic_matrix_apply_two_qubit_gate` 的数值变体：`gate` 指向 16 个行主序 `Complex64`。错误码同上。

### symbolic_matrix_apply_general_gate(ptr, gate, bits, bits_len)

把任意元数的符号门（维度 `1 << bits_len` 的方阵）作用到目标比特。

- `ptr` (`CSymbolicMatrix*`)：目标矩阵（就地修改）。
- `gate` (`const CSymbolicMatrix*`)：门矩阵句柄（维度为 2 的幂的方阵）。
- `bits` (`const uintptr_t*`)：目标比特数组。
- `bits_len` (`uintptr_t`)：目标比特数量。

返回 0 表示成功；-1 表示输入为 NULL；-8 表示门矩阵形状与 `1 << bits_len` 不符或 `bits` 非法。

### symbolic_matrix_apply_general_gate_num(ptr, gate, bits, bits_len)

`symbolic_matrix_apply_general_gate` 的数值变体：`gate` 指向 `(1 << bits_len)` 平方个行主序 `Complex64`。

- `ptr` (`CSymbolicMatrix*`)：目标矩阵（就地修改）。
- `gate` (`const Complex64*`)：门矩阵数值数组。
- `bits` (`const uintptr_t*`)：目标比特数组。
- `bits_len` (`uintptr_t`)：目标比特数量。

返回 0 表示成功；-1 表示矩阵或 `gate` 为 NULL；-8 表示 `bits_len` 溢出或 `bits` 非法。

### symbolic_matrix_apply_permutation_gate(ptr, indices, values, len, bits, bits_len)

就地应用符号置换门：`permutation[i] = (indices[i], values[i])` 表示输出的第 `i` 个局部行等于 `values[i]` 乘以输入的第 `indices[i]` 个局部行。

- `ptr` (`CSymbolicMatrix*`)：目标矩阵（就地修改）。
- `indices` (`const uintptr_t*`)：源局部行下标数组。
- `values` (`const CSymbolicComplex* const*`)：与 `indices` 配对的符号乘子句柄数组。
- `len` (`uintptr_t`)：置换长度，必须等于 `1 << bits_len`。
- `bits` (`const uintptr_t*`)：目标比特数组。
- `bits_len` (`uintptr_t`)：目标比特数量。

返回 0 表示成功；-1 表示矩阵为 NULL、`indices[i]` 越界或 `values[i]` 为 NULL；-8 表示 `len` 不等于 `1 << bits_len`、`indices`/`values` 为 NULL 或 `bits` 非法。

### symbolic_matrix_apply_permutation_gate_num(ptr, indices, values, len, bits, bits_len)

`symbolic_matrix_apply_permutation_gate` 的数值变体：`values` 指向 `len` 个 `Complex64` 乘子。

- `ptr` (`CSymbolicMatrix*`)：目标矩阵（就地修改）。
- `indices` (`const uintptr_t*`)：源局部行下标数组。
- `values` (`const Complex64*`)：数值乘子数组。
- `len` (`uintptr_t`)：置换长度，必须等于 `1 << bits_len`。
- `bits` (`const uintptr_t*`)：目标比特数组。
- `bits_len` (`uintptr_t`)：目标比特数量。

返回 0 表示成功；-1 表示矩阵或 `values` 为 NULL；-8 表示 `indices[i]` 越界、`len` 不等于 `1 << bits_len`、`indices` 为 NULL 或 `bits` 非法。

### symbolic_matrix_apply_diagonal_gate(ptr, diagonal, len, bits, bits_len)

就地应用符号对角门：`diagonal[i]` 缩放第 `i` 个局部行。

- `ptr` (`CSymbolicMatrix*`)：目标矩阵（就地修改）。
- `diagonal` (`const CSymbolicComplex* const*`)：对角元素句柄数组。
- `len` (`uintptr_t`)：对角长度，必须等于 `1 << bits_len`。
- `bits` (`const uintptr_t*`)：目标比特数组。
- `bits_len` (`uintptr_t`)：目标比特数量。

返回 0 表示成功；-1 表示矩阵为 NULL 或任一对角元素句柄为 NULL；-8 表示 `len` 不等于 `1 << bits_len`、`diagonal` 为 NULL 或 `bits` 非法。

### symbolic_matrix_apply_diagonal_gate_num(ptr, diagonal, len, bits, bits_len)

`symbolic_matrix_apply_diagonal_gate` 的数值变体：`diagonal` 指向 `len` 个 `Complex64`。

- `ptr` (`CSymbolicMatrix*`)：目标矩阵（就地修改）。
- `diagonal` (`const Complex64*`)：对角数值数组。
- `len` (`uintptr_t`)：对角长度，必须等于 `1 << bits_len`。
- `bits` (`const uintptr_t*`)：目标比特数组。
- `bits_len` (`uintptr_t`)：目标比特数量。

返回 0 表示成功；-1 表示矩阵或 `diagonal` 为 NULL；-8 表示 `len` 不等于 `1 << bits_len` 或 `bits` 非法。

### 示例：就地复现线路矩阵

```c
/* 复现 circuit_h(c, 0) + circuit_cx(c, 0, 1) 的符号矩阵。
 * CX 的 [control, target] 对在 apply_* 的 Little-Endian
 * 约定下需反转为 [target, control]。 */
struct CSymbolicMatrix *matrix = symbolic_eye(4);
const uintptr_t h_bits[1] = {0};
const uintptr_t cx_bits[2] = {1, 0};
symbolic_matrix_apply_standard_gate(matrix, "H", h_bits, 1, NULL, 0);
symbolic_matrix_apply_standard_gate(matrix, "CX", cx_bits, 2, NULL, 0);
/* matrix 现在等价于 circuit_to_symbolic_matrix(c, NULL, 0) */
```

---

## 等价性检查

### symbolic_matrices_equivalent(lhs, rhs)

判断两个符号矩阵是否可证明仅相差全局相位。

- `lhs` (`const CSymbolicMatrix*`)：左侧矩阵。
- `rhs` (`const CSymbolicMatrix*`)：右侧矩阵。

返回 1 表示等价，0 表示不等价；-1 表示输入为 NULL；-3 表示化简失败。

### circuits_equivalent(lhs, rhs, order, order_len)

在指定量子比特顺序下比较两条线路的矩阵是否仅相差全局相位。

- `lhs` (`const CCircuit*`)：左侧线路句柄。
- `rhs` (`const CCircuit*`)：右侧线路句柄。
- `order` (`const uint32_t*`)：量子比特 id 数组，从最低有效位到最高有效位排列；传 NULL（且 `order_len` 为 0）时按量子比特编号排序。
- `order_len` (`uintptr_t`)：数组长度。

返回 1 表示等价，0 表示不等价；-1 表示输入为 NULL；-3 表示矩阵构造失败（含顺序与量子比特集合不匹配）；-8 表示 `order` 为 NULL 而 `order_len` 大于 0。

---

## 量子比特顺序约定

`circuit_to_symbolic_matrix` 和 `circuits_equivalent` 的 `order` 参数确定矩阵基态下标与线路量子比特的对应关系：

- `order` 为 NULL（`order_len` 为 0）：默认按量子比特编号排序；
- `order` 显式给定：数组中第 `i` 个量子比特对应基态下标的第 `i` 位（最低有效位在前）；
- 显式顺序必须与线路中的量子比特集合完全一致，不能遗漏、重复或包含未知量子比特。

例如线路包含量子比特 0 和 2 时，`[0, 2]` 与 `[2, 0]` 都是合法顺序，但会得到不同轴序下的矩阵表示。

---

## 示例：含参线路 → 符号矩阵 → 代入求值

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. 构造含参线路：RX(theta) 作用于 qubit 0 */
    struct CCircuit *circuit = circuit_new(1);
    struct CParameter *theta = param_parse("theta");
    if (circuit_rx_param(circuit, 0, theta) != 0) {
        return 1;
    }

    /* 2. 转换为符号矩阵（默认按量子比特编号排序） */
    struct CSymbolicMatrix *matrix = circuit_to_symbolic_matrix(circuit, NULL, 0);
    if (matrix == NULL) {
        return 1;
    }

    /* 查看元素的符号表达形式，例如 "cos(theta*0.5)" */
    char *element = symbolic_matrix_element_str(matrix, 0, 0);
    if (element != NULL) {
        printf("matrix[0][0] = %s\n", element);
        cqlib_string_free(element);
    }

    /* 3. 绑定 theta = pi/2，代入求值为数值矩阵 */
    uintptr_t len = symbolic_matrix_evaluate_len(matrix);
    Complex64 *values = malloc(len * sizeof(Complex64));
    int32_t status = symbolic_matrix_evaluate(matrix, "theta:1.5707963267948966",
                                              values, len);
    if (status == 0) {
        /* values[0] 即 cos(pi/4)，values 按行主序排列 */
        printf("matrix[0][0] = %f + %fi\n", values[0].re, values[0].im);
    }

    free(values);
    symbolic_matrix_free(matrix);
    param_free(theta);
    circuit_free(circuit);
    return status;
}
```

参数表达式解析（`param_parse` / `param_free`）见 [参数](3_parameter.md)，参数化旋转门（`circuit_rx_param`）见 [标准门](5_gate_standard.md)，线路构造见 [Circuit](1_circuit.md)。
