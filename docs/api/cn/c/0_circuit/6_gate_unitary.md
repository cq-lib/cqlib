# 酉门

本页介绍 C API 中以矩阵定义的自定义酉门追加接口；错误码与内存管理的全局约定见 [Overview](../0_overview.md)。

---

## circuit_unitary(ptr, label, num_qubits, matrix, qubits, qubits_len)

向线路追加一个由数值矩阵定义的自定义酉门。

参数：

- `ptr` (`struct CCircuit *`)：目标线路句柄。
- `label` (`const char *`)：门的可读名称，用于显示、可视化与 IR 输出。
- `num_qubits` (`uintptr_t`)：门作用的量子比特数量 n，最大为 20。
- `matrix` (`const Complex64 *`)：行主序展平的 2^n × 2^n 复数矩阵，共 2^n·2^n 个 `Complex64` 元素；每个元素为 `{double re; double im}` 结构，实虚成对交错，布局与 `circuit_to_matrix` 输出一致。
- `qubits` (`const uint32_t *`)：作用比特索引数组。
- `qubits_len` (`uintptr_t`)：`qubits` 的长度，须等于 `num_qubits`。

返回：`int32_t`，`0` 成功；`-1` NULL 输入（`ptr`、`label` 或 `matrix` 为 NULL）；`-2` 比特越界；`-3` 矩阵非酉、形状不符或应用失败；`-4` `label` 含无效 UTF-8；`-8` `num_qubits` 超过 20。

矩阵在追加时做酉性校验：U†U 须在容差范围内为单位矩阵，否则返回 `-3`。数值矩阵定义的门不带应用参数，`qubits` 按数组顺序映射到门定义的比特。

```c
#include <math.h>
#include "cqlib_c.h"

/* 自定义 2×2 酉门：sqrt(X) */
CCircuit *c = circuit_new(1);

double s = 1.0 / sqrt(2.0);
Complex64 sqrt_x[4] = {
    {s,  0.0}, {0.0, -s},
    {0.0, -s}, {s,  0.0},
};

uint32_t qubits[1] = {0};
if (circuit_unitary(c, "sqrt_x", 1, sqrt_x, qubits, 1) != 0) {
    /* 处理错误 */
}

circuit_free(c);
```

---

## circuit_unitary_with_params(ptr, label, num_qubits, re_exprs, im_exprs, param_names, param_names_len, qubits, qubits_len, params, params_len)

向线路追加一个由符号矩阵定义的参数化自定义酉门。

参数：

- `ptr` (`struct CCircuit *`)：目标线路句柄。
- `label` (`const char *`)：门的可读名称。
- `num_qubits` (`uintptr_t`)：门作用的量子比特数量 n，最大为 20。
- `re_exprs` (`const char *const *`)：行主序展平的 2^n·2^n 个实部表达式字符串，如 `"cos(theta)"`、`"1"`。
- `im_exprs` (`const char *const *`)：同布局的虚部表达式字符串数组；可为 NULL，此时所有虚部为 0。
- `param_names` (`const char *const *`)：位置参数名列表，声明应用参数与矩阵中符号名的绑定顺序。
- `param_names_len` (`uintptr_t`)：`param_names` 的长度。
- `qubits` (`const uint32_t *`)：作用比特索引数组。
- `qubits_len` (`uintptr_t`)：`qubits` 的长度，须等于 `num_qubits`。
- `params` (`const struct CParameter *const *`)：应用时的参数句柄数组，第 i 个参数绑定到 `param_names[i]`。
- `params_len` (`uintptr_t`)：`params` 的长度，须等于 `param_names_len`。

返回：`int32_t`，`0` 成功；`-1` NULL 输入；`-2` 比特越界；`-3` 形状、符号声明或应用失败；`-4` 无效 UTF-8 或表达式语法错误；`-8` 尺寸无效（`num_qubits` 超过 20 或 `param_names_len` 超过 65535）。

表达式使用参数表达式语法（与 `param_parse` 一致）；矩阵中出现的自由符号必须已在 `param_names` 中声明。数值角度以表达式形式书写，例如 `"pi/2"`。

```c
#include "cqlib_c.h"

/* 参数化对角相位门 diag(1, e^(i·theta))，应用时绑定 theta = pi/2 */
CCircuit *c = circuit_new(1);

const char *re[4] = {"1", "0", "0", "cos(theta)"};
const char *im[4] = {"0", "0", "0", "sin(theta)"};
const char *names[1] = {"theta"};

CParameter *angle = param_parse("pi/2");
const struct CParameter *params[1] = {angle};

uint32_t qubits[1] = {0};
if (circuit_unitary_with_params(c, "phase_e", 1,
                                re, im, names, 1,
                                qubits, 1, params, 1) != 0) {
    /* 处理错误 */
}

param_free(angle);
circuit_free(c);
```

---

## 在参数取值处求值酉矩阵

C ABI 没有独立的酉门句柄，因此在线路之外求值一个门定义时复用 `circuit_unitary_with_params` 的输入方式：符号矩阵加上按位置绑定的形式参数名。

### unitary_gate_matrix_for_params_len(matrix)

```c
uintptr_t unitary_gate_matrix_for_params_len(const struct CSymbolicMatrix *matrix);
```

返回 `unitary_gate_matrix_for_params` 为 `matrix` 写出的 `Complex64` 元素个数（`rows * cols`）；NULL 或非方阵时返回 0。

### unitary_gate_matrix_for_params(matrix, param_names, param_names_len, params, params_len, buffer, buffer_len)

```c
int32_t unitary_gate_matrix_for_params(const struct CSymbolicMatrix *matrix,
                                       const char *const *param_names,
                                       uintptr_t param_names_len,
                                       const double *params,
                                       uintptr_t params_len,
                                       Complex64 *buffer,
                                       uintptr_t buffer_len);
```

在给定的数值参数取值处求值自定义酉门的符号矩阵，并把得到的数值矩阵按行主序写入 `buffer`。门的构造与校验和 `circuit_unitary_with_params` 完全一致（形状、参数个数、数值有限性、酉性）。

参数：

- `matrix` (`const struct CSymbolicMatrix *`)：`2^n × 2^n` 符号矩阵（构造见[符号矩阵](10_symbolic_matrix.md)）。
- `param_names` (`const char *const *`)：形式参数名列表，按顺序把应用参数绑定到矩阵符号；`param_names_len` 为 0 时可为 NULL。
- `param_names_len` (`uintptr_t`)：`param_names` 的长度，不超过 65535。
- `params` (`const double *`)：数值参数取值；`params_len` 为 0 时可为 NULL。
- `params_len` (`uintptr_t`)：`params` 的长度。
- `buffer` (`Complex64 *`)：接收求值结果的缓冲，按行主序。
- `buffer_len` (`uintptr_t`)：缓冲长度，至少为 `unitary_gate_matrix_for_params_len(matrix)`。

返回：`int32_t`，`0` 成功；`-1` NULL 输入；`-3` 形状、参数个数、非有限数值或酉性校验失败；`-4` 参数名不是合法 UTF-8；`-8` 尺寸非法（空矩阵或非 2 的幂、超过 20 比特、缓冲过小）。

```c
#include <math.h>
#include "cqlib_c.h"

/* 在 theta = pi/2 处求值 U(theta) = cos(theta)*I + i*sin(theta)*X（结果为 iX） */
struct CParameter *zero = param_parse("0");
struct CParameter *cos_t = param_parse("cos(theta)");
struct CParameter *sin_t = param_parse("sin(theta)");
struct CSymbolicComplex *e00 = symbolic_complex_new(cos_t, zero);
struct CSymbolicComplex *e01 = symbolic_complex_new(zero, sin_t);
struct CSymbolicComplex *e10 = symbolic_complex_new(zero, sin_t);
struct CSymbolicComplex *e11 = symbolic_complex_new(cos_t, zero);
const struct CSymbolicComplex *elements[4] = {e00, e01, e10, e11};
struct CSymbolicMatrix *matrix = symbolic_matrix_new(2, 2, elements, 4);

const char *names[1] = {"theta"};
double params[1] = {3.141592653589793 / 2.0};

uintptr_t len = unitary_gate_matrix_for_params_len(matrix);   /* 4 */
Complex64 out[4];
if (unitary_gate_matrix_for_params(matrix, names, 1, params, 1, out, len) == 0) {
    /* out = {{0,0},{0,1},{0,1},{0,0}}，即矩阵 iX */
}

symbolic_matrix_free(matrix);
symbolic_complex_free(e00);
symbolic_complex_free(e01);
symbolic_complex_free(e10);
symbolic_complex_free(e11);
param_free(zero);
param_free(cos_t);
param_free(sin_t);
```

---

## 两种定义方式对比

| 函数 | 矩阵定义 | 应用参数 | 典型用途 |
| --- | --- | --- | --- |
| `circuit_unitary` | 数值 `Complex64` 数组 | 无 | 固定黑盒矩阵、校准门、测试 oracle。 |
| `circuit_unitary_with_params` | 实虚部表达式字符串 | `CParameter` 数组按 `param_names` 位置绑定 | 参数化自定义矩阵门。 |

---

## 相关页面

- [标准门](5_gate_standard.md)：内置标准门集合。
- [参数](3_parameter.md)：`CParameter` 的创建与表达式语法。
- [线路门](8_gate_circuit_gate.md)：以子线路定义的复合门。
- [线路转矩阵](13_circuit_to_matrix.md)：与 `circuit_unitary` 输入一致的矩阵布局输出。
