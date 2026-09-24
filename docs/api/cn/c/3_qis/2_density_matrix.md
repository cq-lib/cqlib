# DensityMatrix（密度矩阵）

`CDensityMatrix*` 是混合态模拟器句柄：量子态表示为 `2^N × 2^N` 的密度矩阵（行主序 `4^N` 个矩阵元），可以描述纯态、混合态以及量子信道作用后的结果。本页覆盖构造与线路对接、矩阵数据与物理性校验、门操作、量子信道与偏迹、概率与测量以及期望值。错误码与内存约定见 [Overview](../0_overview.md)，模块总览见 [QIS 概览](0_overview.md)。

---

## 构造与释放

### density_matrix_new(num_qubits)

```c
struct CDensityMatrix *density_matrix_new(uintptr_t num_qubits);
```

创建混合态模拟器，初始态为纯态 `|0...0><0...0|`。

- `num_qubits` (`uintptr_t`)：量子比特数量。

返回：新分配的 `CDensityMatrix*`，用 `density_matrix_free` 释放。

### density_matrix_maximally_mixed(num_qubits)

```c
struct CDensityMatrix *density_matrix_maximally_mixed(uintptr_t num_qubits);
```

构造最大混合态 `I / 2^N`：各计算基态等概率且无相干项。

- `num_qubits` (`uintptr_t`)：量子比特数量。

返回：新分配的 `CDensityMatrix*`，用 `density_matrix_free` 释放。

### density_matrix_zeros(num_qubits)

```c
struct CDensityMatrix *density_matrix_zeros(uintptr_t num_qubits);
```

创建全零的密度矩阵。它不是合法的物理态（迹为 0），适合作为 Kraus 信道等运算的累加起点。

- `num_qubits` (`uintptr_t`)：量子比特数量。

返回：新分配的 `CDensityMatrix*`，用 `density_matrix_free` 释放；矩阵尺寸超出可寻址范围时返回 `NULL`。

### density_matrix_from_circuit(circuit)

```c
struct CDensityMatrix *density_matrix_from_circuit(const struct CCircuit *circuit);
```

执行线路并返回演化后的密度矩阵，输入线路不被修改。线路先被分解为基础门再逐条执行；测量声明不参与状态演化。

- `circuit` (`const CCircuit*`)：线路句柄，见 [Circuit](../0_circuit/1_circuit.md)。

返回：新分配的 `CDensityMatrix*`；`circuit` 为 NULL 或执行失败返回 `NULL`。

### density_matrix_from_state(num_qubits, initial_state, len)

```c
struct CDensityMatrix *density_matrix_from_state(uintptr_t num_qubits,
                                                 const Complex64 *initial_state,
                                                 uintptr_t len);
```

由归一化纯态振幅做外积 `ρ = |ψ><ψ|` 构造密度矩阵；要求 `len` 等于 `2^num_qubits`。

- `num_qubits` (`uintptr_t`)：量子比特数量。
- `initial_state` (`const Complex64*`)：长度为 `2^num_qubits` 的归一化振幅数组。
- `len` (`uintptr_t`)：数组长度。

返回：新分配的 `CDensityMatrix*`；输入为 NULL、长度不符或未归一化时返回 `NULL`。

### density_matrix_free(ptr)

```c
void density_matrix_free(struct CDensityMatrix *ptr);
```

释放句柄。允许传 `NULL`。

- `ptr` (`CDensityMatrix*`)：密度矩阵句柄。

### density_matrix_num_qubits(ptr)

```c
uintptr_t density_matrix_num_qubits(const struct CDensityMatrix *ptr);
```

返回量子比特数量。

- `ptr` (`const CDensityMatrix*`)：密度矩阵句柄。

返回：比特数量；`NULL` 时返回 0。

### density_matrix_apply_circuit(ptr, circuit)

```c
int32_t density_matrix_apply_circuit(struct CDensityMatrix *ptr, const struct CCircuit *circuit);
```

把线路原地作用到当前密度矩阵上，线路的比特数必须与态一致。

- `ptr` (`CDensityMatrix*`)：密度矩阵句柄。
- `circuit` (`const CCircuit*`)：线路句柄。

返回：`0` 成功；`-1` NULL 指针；`-3` 线路错误；`-7` 线路含不支持的操作；`-8` 线路比特数与态不一致。

---

## 矩阵数据与物理性

### density_matrix_data_len(ptr)

```c
uintptr_t density_matrix_data_len(const struct CDensityMatrix *ptr);
```

返回矩阵元数量（`4^N`），用于为 `density_matrix_data` 分配缓冲。

- `ptr` (`const CDensityMatrix*`)：密度矩阵句柄。

返回：矩阵元数量；`NULL` 时返回 0。

### density_matrix_data(ptr, buffer, len)

```c
int32_t density_matrix_data(const struct CDensityMatrix *ptr, Complex64 *buffer, uintptr_t len);
```

把展平的行主序矩阵元拷贝进 `buffer`，共 `4^N` 个 `Complex64` 值；第 `r·2^N + c` 个元素是矩阵第 `r` 行第 `c` 列。

- `ptr` (`const CDensityMatrix*`)：密度矩阵句柄。
- `buffer` (`Complex64*`)：输出缓冲。
- `len` (`uintptr_t`)：缓冲长度，必须等于 `density_matrix_data_len`。

返回：`0` 成功；`-1` NULL 指针；`-8` `len` 与实际数量不符。

### density_matrix_trace(ptr, out_re, out_im)

```c
int32_t density_matrix_trace(const struct CDensityMatrix *ptr, double *out_re, double *out_im);
```

计算迹并把实部、虚部分别写入 `out_re` / `out_im`；合法物理态的迹等于 `1.0`。

- `ptr` (`const CDensityMatrix*`)：密度矩阵句柄。
- `out_re` (`double*`)：迹的实部输出。
- `out_im` (`double*`)：迹的虚部输出。

返回：`0` 成功；`-1` 任一指针为 NULL。

### density_matrix_is_hermitian(ptr, tol, out)

```c
int32_t density_matrix_is_hermitian(const struct CDensityMatrix *ptr, double tol, bool *out);
```

在容差 `tol` 内判断矩阵是否满足 `ρ = ρ†`，结果写入 `out`。

- `ptr` (`const CDensityMatrix*`)：密度矩阵句柄。
- `tol` (`double`)：容差，必须为有限值。
- `out` (`bool*`)：结果输出。

返回：`0` 成功；`-1` NULL 指针；`-8` `tol` 非有限。

### density_matrix_is_positive_semidefinite(ptr, tol, out)

```c
int32_t density_matrix_is_positive_semidefinite(const struct CDensityMatrix *ptr, double tol, bool *out);
```

在容差 `tol` 内判断矩阵是否半正定（全部特征值满足 `λ >= -tol`），结果写入 `out`。

- `ptr` (`const CDensityMatrix*`)：密度矩阵句柄。
- `tol` (`double`)：容差，必须为有限值。
- `out` (`bool*`)：结果输出。

返回：`0` 成功；`-1` NULL 指针；`-8` `tol` 非有限。

### density_matrix_validate_physical(ptr, tol)

```c
int32_t density_matrix_validate_physical(const struct CDensityMatrix *ptr, double tol);
```

在容差 `tol` 内依次校验密度矩阵的物理性约束：Hermitian（`ρ = ρ†`）、半正定、迹为 1。矩阵只被读取，不做修改。

- `ptr` (`const CDensityMatrix*`)：密度矩阵句柄。
- `tol` (`double`)：容差，必须为有限值。

返回：`0` 态为物理态；`-1` 句柄为 NULL；`-7` 非 Hermitian 或非半正定；`-8` `tol` 非有限或迹不等于 `1`。

---

## 门操作

所有门函数返回 `int32_t`：`0` 成功；`-1` 句柄为 NULL；`-2` 比特编号越界；参数化门的角度取值为 NaN 或 Inf 时返回 `-8`。单比特门统一签名为：

```c
int32_t density_matrix_apply_h(struct CDensityMatrix *ptr, uint32_t qubit);
```

### 无参数单比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `density_matrix_apply_h(ptr, qubit)` | H | Hadamard 门。 |
| `density_matrix_apply_x(ptr, qubit)` | X | Pauli-X，比特翻转。 |
| `density_matrix_apply_y(ptr, qubit)` | Y | Pauli-Y。 |
| `density_matrix_apply_z(ptr, qubit)` | Z | Pauli-Z，相位翻转。 |
| `density_matrix_apply_s(ptr, qubit)` | S | S 门（√Z）。 |
| `density_matrix_apply_sdg(ptr, qubit)` | S† | S 的逆。 |
| `density_matrix_apply_t(ptr, qubit)` | T | T 门（√S）。 |
| `density_matrix_apply_tdg(ptr, qubit)` | T† | T 的逆。 |
| `density_matrix_apply_x2p(ptr, qubit)` | X2P | `Rx(π/2)`。 |
| `density_matrix_apply_x2m(ptr, qubit)` | X2M | `Rx(-π/2)`。 |
| `density_matrix_apply_y2p(ptr, qubit)` | Y2P | `Ry(π/2)`。 |
| `density_matrix_apply_y2m(ptr, qubit)` | Y2M | `Ry(-π/2)`。 |

### 参数化单比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `density_matrix_apply_rx(ptr, qubit, theta)` | Rx(θ) | 绕 X 轴旋转 `θ`。 |
| `density_matrix_apply_ry(ptr, qubit, theta)` | Ry(θ) | 绕 Y 轴旋转 `θ`。 |
| `density_matrix_apply_rz(ptr, qubit, theta)` | Rz(θ) | 绕 Z 轴旋转 `θ`。 |
| `density_matrix_apply_phase(ptr, qubit, theta)` | P(θ) | 相位门，给 `\|1>` 分量乘 `e^(iθ)`。 |
| `density_matrix_apply_u(ptr, qubit, theta, phi, lambda)` | U(θ, φ, λ) | 通用单比特门。 |
| `density_matrix_apply_xy(ptr, qubit, theta)` | XY(θ) | XY 门。 |
| `density_matrix_apply_xy2p(ptr, qubit, theta)` | XY2P(θ) | XY2P 门。 |
| `density_matrix_apply_xy2m(ptr, qubit, theta)` | XY2M(θ) | XY2M 门。 |
| `density_matrix_apply_rxy(ptr, qubit, theta, phi)` | RXY(θ, φ) | 绕 XY 平面内方位角为 `φ` 的轴旋转 `θ`。 |
| `density_matrix_apply_gphase(ptr, phi)` | GPhase(φ) | 全局相位。密度矩阵在全局相位下不变，调用恒返回 `0` 且不改变状态；签名无 qubit 参数。 |

GPhase 签名：

```c
int32_t density_matrix_apply_gphase(struct CDensityMatrix *ptr, double phi);
```

### 双比特门

统一签名 `int32_t density_matrix_apply_<gate>(struct CDensityMatrix *ptr, uint32_t q0, uint32_t q1, ...)`，受控门的两个比特参数名为 `control` / `target`。

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `density_matrix_apply_cx(ptr, control, target)` | CX | 受控 X。 |
| `density_matrix_apply_cy(ptr, control, target)` | CY | 受控 Y。 |
| `density_matrix_apply_cz(ptr, q0, q1)` | CZ | 受控 Z，两参数地位对称。 |
| `density_matrix_apply_swap(ptr, q0, q1)` | SWAP | 交换两个比特。 |
| `density_matrix_apply_rxx(ptr, q0, q1, theta)` | RXX(θ) | `exp(-i·θ/2·X⊗X)`。 |
| `density_matrix_apply_ryy(ptr, q0, q1, theta)` | RYY(θ) | `exp(-i·θ/2·Y⊗Y)`。 |
| `density_matrix_apply_rzz(ptr, q0, q1, theta)` | RZZ(θ) | `exp(-i·θ/2·Z⊗Z)`。 |
| `density_matrix_apply_rzx(ptr, q0, q1, theta)` | RZX(θ) | `exp(-i·θ/2·Z⊗X)`。 |
| `density_matrix_apply_crx(ptr, control, target, theta)` | CrX(θ) | 受控 `Rx(θ)`。 |
| `density_matrix_apply_cry(ptr, control, target, theta)` | CrY(θ) | 受控 `Ry(θ)`。 |
| `density_matrix_apply_crz(ptr, control, target, theta)` | CrZ(θ) | 受控 `Rz(θ)`。 |
| `density_matrix_apply_fsim(ptr, q0, q1, theta, phi)` | FSIM | 费米子模拟门，`theta` 为 iSWAP 角，`phi` 为受控相位角。 |

### 三比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `density_matrix_apply_ccx(ptr, c0, c1, target)` | CCX | Toffoli 门，两个控制位同时为 `\|1>` 时翻转目标位。 |

### 通用门入口

#### density_matrix_apply_standard_gate(ptr, gate_name, qubits, num_target_qubits, params, num_params)

```c
int32_t density_matrix_apply_standard_gate(struct CDensityMatrix *ptr,
                                           const char *gate_name,
                                           const uint32_t *qubits,
                                           uintptr_t num_target_qubits,
                                           const double *params,
                                           uintptr_t num_params);
```

按名称施加标准门（如 `"H"`、`"CX"`、`"RZZ"`、`"FSIM"`），分派到对应的专用实现。

- `ptr` (`CDensityMatrix*`)：密度矩阵句柄。
- `gate_name` (`const char*`)：NUL 结尾的标准门名。
- `qubits` (`const uint32_t*`)：目标比特数组，长度为 `num_target_qubits`。
- `num_target_qubits` (`uintptr_t`)：目标比特数量，必须与门的要求一致。
- `params` (`const double*`)：门参数数组；`num_params` 为 0 时可为 NULL。
- `num_params` (`uintptr_t`)：参数数量，必须与门的要求一致。

返回：`0` 成功；`-1` NULL 指针；`-2` 比特越界；`-8` 未知门名、门名不是合法 UTF-8、比特数或参数个数与门不符、参数值非有限。

---

## 量子信道与偏迹

### density_matrix_apply_kraus(ptr, ops, num_ops, op_dim, qubits, num_target_qubits)

```c
int32_t density_matrix_apply_kraus(struct CDensityMatrix *ptr,
                                   const Complex64 *ops,
                                   uintptr_t num_ops,
                                   uintptr_t op_dim,
                                   const uint32_t *qubits,
                                   uintptr_t num_target_qubits);
```

作用由 Kraus 算符给出的量子信道，演化为 `ρ → Σ_k K_k ρ K_k†`。

- `ptr` (`CDensityMatrix*`)：密度矩阵句柄。
- `ops` (`const Complex64*`)：展平的算符数组，共 `num_ops × op_dim × op_dim` 个行主序 `Complex64` 值，每个算符依次存放。
- `num_ops` (`uintptr_t`)：算符数量。
- `op_dim` (`uintptr_t`)：单个算符的维度，等于 `2^num_target_qubits`。
- `qubits` (`const uint32_t*`)：目标比特数组。
- `num_target_qubits` (`uintptr_t`)：目标比特数量。

返回：`0` 成功；`-1` NULL 指针；`-2` 比特越界；`-8` `num_ops`、`op_dim` 或 `num_target_qubits` 为 0，或算符尺寸与比特数不符。

```c
/* Bit-flip channel with p = 0.3 on qubit 0 */
double p = 0.3, s0 = sqrt(1.0 - p), s1 = sqrt(p);
Complex64 ops[8] = {
    {s0, 0}, {0, 0}, {0, 0}, {s0, 0},   /* K0 = sqrt(1-p) * I */
    {0, 0}, {s1, 0}, {s1, 0}, {0, 0},   /* K1 = sqrt(p) * X   */
};
uint32_t q = 0;
density_matrix_apply_kraus(dm, ops, 2, 2, &q, 1);
```

### density_matrix_partial_trace(ptr, keep_qubits, num_keep)

```c
struct CDensityMatrix *density_matrix_partial_trace(const struct CDensityMatrix *ptr,
                                                    const uint32_t *keep_qubits,
                                                    uintptr_t num_keep);
```

对 `keep_qubits` 之外的比特求偏迹，返回比特数为 `num_keep` 的新密度矩阵。

- `ptr` (`const CDensityMatrix*`)：密度矩阵句柄。
- `keep_qubits` (`const uint32_t*`)：保留比特数组；`num_keep` 为 0 时可为 NULL。
- `num_keep` (`uintptr_t`)：保留比特数量。

返回：新分配的 `CDensityMatrix*`，用 `density_matrix_free` 释放；失败返回 `NULL`。

---

## 概率与测量

### density_matrix_probabilities_len(ptr)

```c
uintptr_t density_matrix_probabilities_len(const struct CDensityMatrix *ptr);
```

返回计算基态概率分布的长度（`2^N`）。

- `ptr` (`const CDensityMatrix*`)：密度矩阵句柄。

返回：概率数量；`NULL` 时返回 0。

### density_matrix_probabilities(ptr, buffer, len)

```c
int32_t density_matrix_probabilities(const struct CDensityMatrix *ptr,
                                     double *buffer,
                                     uintptr_t len);
```

把密度矩阵对角元（即各计算基态的测量概率）拷贝进 `buffer`。

- `ptr` (`const CDensityMatrix*`)：密度矩阵句柄。
- `buffer` (`double*`)：输出缓冲。
- `len` (`uintptr_t`)：缓冲长度，必须等于 `density_matrix_probabilities_len`。

返回：`0` 成功；`-1` NULL 指针；`-8` `len` 与实际数量不符。

### density_matrix_measure(ptr, qubit)

```c
int32_t density_matrix_measure(struct CDensityMatrix *ptr, uint32_t qubit);
```

在 Z 基下测量指定比特并使密度矩阵坍缩：`ρ' = Π_b ρ Π_b / Tr(Π_b ρ)`。

- `ptr` (`CDensityMatrix*`)：密度矩阵句柄。
- `qubit` (`uint32_t`)：被测比特。

返回：`0` 表示结果 `|0>`，`1` 表示结果 `|1>`；`-1` NULL；`-2` 比特越界。

### density_matrix_measure_all(ptr)

```c
char *density_matrix_measure_all(struct CDensityMatrix *ptr);
```

按 `0..num_qubits` 顺序逐比特测量并使态坍缩，返回大端比特串（MSB = 比特 `N-1`，LSB = 比特 `0`）。

- `ptr` (`CDensityMatrix*`)：密度矩阵句柄。

返回：堆分配的 C 字符串，用 `cqlib_string_free` 释放；失败返回 `NULL`。

### density_matrix_sample_shots(ptr, shots)

```c
struct COutcomeList *density_matrix_sample_shots(const struct CDensityMatrix *ptr, uintptr_t shots);
```

按当前概率分布并行采样 `shots` 次独立测量结果，不修改态。返回的列表用 `outcome_list_len` 取数量、`outcome_list_get` 读取每个大端比特串（用 `cqlib_string_free` 释放）、`outcome_list_free` 释放列表，详见 [ClassicalState](5_classical_state.md)。

- `ptr` (`const CDensityMatrix*`)：密度矩阵句柄。
- `shots` (`uintptr_t`)：采样次数。

返回：新分配的 `COutcomeList*`；失败返回 `NULL`。

### density_matrix_reset(ptr, qubit)

```c
int32_t density_matrix_reset(struct CDensityMatrix *ptr, uint32_t qubit);
```

把指定比特重置为 `|0>`。

- `ptr` (`CDensityMatrix*`)：密度矩阵句柄。
- `qubit` (`uint32_t`)：目标比特。

返回：`0` 成功；`-1` NULL；`-2` 比特越界。

---

## 期望值

### density_matrix_expectation(ptr, observable, out)

```c
int32_t density_matrix_expectation(const struct CDensityMatrix *ptr,
                                   const struct CHamiltonian *observable,
                                   double *out);
```

计算 `Tr(ρ·H)` 并写入 `out`。

- `ptr` (`const CDensityMatrix*`)：密度矩阵句柄。
- `observable` (`const CHamiltonian*`)：可观测量句柄，见 [Hamiltonian](7_hamiltonian.md)。
- `out` (`double*`)：输出指针。

返回：`0` 成功；`-1` NULL 指针；`-8` 观测量的比特数与态不一致。

---

## 示例

### 1. Bell 态、矩阵数据与概率

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    struct CDensityMatrix *dm = density_matrix_new(2);
    density_matrix_apply_h(dm, 0);
    density_matrix_apply_cx(dm, 0, 1);

    /* Probabilities: P(00) = P(11) = 0.5 */
    double probs[4];
    density_matrix_probabilities(dm, probs, 4);
    printf("P(00)=%.3f P(11)=%.3f\n", probs[0], probs[3]);

    /* Row-major matrix data: 16 Complex64 elements */
    uintptr_t n = density_matrix_data_len(dm);   /* 16 */
    Complex64 *data = malloc(n * sizeof(Complex64));
    density_matrix_data(dm, data, n);
    /* data[0] is rho[0][0], data[3] is rho[0][3], data[15] is rho[3][3] */
    printf("rho[3][3].re=%.3f\n", data[15].re);
    free(data);

    density_matrix_free(dm);
    return 0;
}
```

### 2. 最大混合态、迹与 Hermitian 校验

```c
struct CDensityMatrix *dm = density_matrix_maximally_mixed(2);
/* 4^2 = 16 matrix elements, trace = 1, Hermitian */
double tr_re, tr_im;
density_matrix_trace(dm, &tr_re, &tr_im);        /* tr_re = 1.0, tr_im = 0.0 */
bool herm = false;
density_matrix_is_hermitian(dm, 1e-10, &herm);   /* herm = true */
density_matrix_free(dm);
```

### 3. 偏迹

```c
struct CDensityMatrix *dm = density_matrix_new(2);
density_matrix_apply_h(dm, 0);
density_matrix_apply_cx(dm, 0, 1);

/* Trace out qubit 1: qubit 0 becomes maximally mixed */
uint32_t keep[1] = {0};
struct CDensityMatrix *reduced = density_matrix_partial_trace(dm, keep, 1);
/* density_matrix_num_qubits(reduced) == 1, P(0) = P(1) = 0.5 */
density_matrix_free(reduced);
density_matrix_free(dm);
```

### 4. 期望值

```c
struct CDensityMatrix *dm = density_matrix_new(2);
density_matrix_apply_h(dm, 0);
density_matrix_apply_cx(dm, 0, 1);

struct CPauliString *zz = pauli_string_parse("ZZ");
struct CHamiltonian *h = hamiltonian_from_pauli(zz);  /* takes ownership of zz */
double exp = 0.0;
density_matrix_expectation(dm, h, &exp);   /* exp ≈ 1.0 */
hamiltonian_free(h);
density_matrix_free(dm);
```
