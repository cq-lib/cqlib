# 零噪声外推（C）

零噪声外推（ZNE）把线路折叠到多个噪声因子（`2 × 折叠层 + 1`）下，逐个估计期望值，再外推回零噪声极限。本页覆盖 `CZneMitigation` 句柄的创建、折叠线路生成、配置读取、外推与缓解序列，以及折叠结果列表 `CCircuitList` 的遍历。回调契约与模块常量见 [模块概览](0_overview.md)，错误码与句柄释放约定见 [Overview](../0_overview.md)。

---

## 句柄生命周期

### zne_mitigation_new(circuit, fold_levels, num_levels)

基于基底线路与折叠层数组创建 ZNE 缓解句柄。噪声因子由折叠层推导：`2 × level + 1`。

- `circuit` (`const struct CCircuit *`)：基底线路，后续折叠均作用于它的副本。
- `fold_levels` (`const int32_t *`)：折叠层数组，如 `{0, 1, 2}`。
- `num_levels` (`uintptr_t`)：折叠层数组的元素个数。

返回 `struct CZneMitigation *`：新建句柄，用 `zne_mitigation_free` 释放；`circuit` 为 NULL、`num_levels` 为 0 或 `fold_levels` 为 NULL 时返回 NULL。

### zne_mitigation_free(ptr)

释放 ZNE 缓解句柄。

- `ptr` (`struct CZneMitigation *`)：待释放句柄，允许传 NULL。

---

## 折叠与线路列表

### zne_mitigation_fold_circuit(ptr, gate_names)

在每个配置的折叠层上折叠线路，返回折叠线路列表（每个折叠层一条线路）。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。
- `gate_names` (`const char *`)：可选的逗号分隔门名列表（如 `"H,CX"`），只折叠指定门；传 NULL 时全局折叠整条线路。

返回 `struct CCircuitList *`：折叠线路列表，用 `circuit_list_free` 释放；`ptr` 为 NULL、`gate_names` 含未知门名或非法 UTF-8、或折叠失败时返回 NULL。

### circuit_list_len(ptr)

返回折叠线路列表中的线路数量。

- `ptr` (`const struct CCircuitList *`)：折叠线路列表。

返回 `uintptr_t`：线路数量；`ptr` 为 NULL 时返回 0。

### circuit_list_get(ptr, index)

读取列表中指定下标的线路，返回独立副本。

- `ptr` (`const struct CCircuitList *`)：折叠线路列表。
- `index` (`uintptr_t`)：线路下标。

返回 `struct CCircuit *`：线路的 owned 副本，用 `circuit_free` 释放；`ptr` 为 NULL 或 `index` 越界时返回 NULL。

### circuit_list_free(ptr)

释放折叠线路列表。`circuit_list_get` 取出的线路副本不随之释放，需逐个 `circuit_free`。

- `ptr` (`struct CCircuitList *`)：待释放列表，允许传 NULL。

---

## 配置读取

### zne_circuit(ptr)

返回原始（未折叠）线路的独立副本。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。

返回 `struct CCircuit *`：线路副本，用 `circuit_free` 释放；`ptr` 为 NULL 时返回 NULL。

### zne_mitigation_noise_factor(ptr, index)

返回指定折叠层对应的噪声因子（`2 × level + 1`）。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。
- `index` (`uintptr_t`)：折叠层下标。

返回 `int32_t`：噪声因子；`ptr` 为 NULL 或 `index` 越界时返回 0。

### zne_fold_levels_len(ptr) / zne_fold_levels(ptr, buffer, len)

两步式读取配置的折叠层数组：`zne_fold_levels_len` 返回元素个数，`zne_fold_levels` 把数组拷入调用方缓冲区。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。
- `buffer` (`int32_t *`)：接收折叠层的缓冲区，长度不小于 `zne_fold_levels_len(ptr)`。
- `len` (`uintptr_t`)：缓冲区长度，必须等于 `zne_fold_levels_len(ptr)`。

`zne_fold_levels_len` 返回 `uintptr_t`：元素个数，`ptr` 为 NULL 时返回 0。`zne_fold_levels` 返回 `int32_t`：0 成功；-1（`ptr` 或 `buffer` 为 NULL）；-8（`len` 与元素个数不符）。

### zne_noise_factors_len(ptr) / zne_noise_factors(ptr, buffer, len)

两步式读取噪声因子数组（每个折叠层 `2 × level + 1`）：`zne_noise_factors_len` 返回元素个数，`zne_noise_factors` 把数组拷入调用方缓冲区。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。
- `buffer` (`int32_t *`)：接收噪声因子的缓冲区，长度不小于 `zne_noise_factors_len(ptr)`。
- `len` (`uintptr_t`)：缓冲区长度，必须等于 `zne_noise_factors_len(ptr)`。

`zne_noise_factors_len` 返回 `uintptr_t`：元素个数，`ptr` 为 NULL 时返回 0。`zne_noise_factors` 返回 `int32_t`：0 成功；-1（`ptr` 或 `buffer` 为 NULL）；-8（`len` 与元素个数不符）。

---

## 两步式折叠输出

### zne_fold_circuits_len(ptr, gate_names)

返回两步式折叠输出的线路数量（每个折叠层一条）。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。
- `gate_names` (`const char *`)：可选的逗号分隔门名列表，语义同 `zne_mitigation_fold_circuit`。

返回 `uintptr_t`：线路数量；`ptr` 为 NULL 或 `gate_names` 含未知门名/非法 UTF-8 时返回 0。

### zne_fold_circuits(ptr, gate_names, buffer, len)

在每个配置的折叠层上折叠线路，把每条折叠线路作为 owned 句柄写入调用方缓冲区。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。
- `gate_names` (`const char *`)：可选的逗号分隔门名列表，传 NULL 时全局折叠。
- `buffer` (`struct CCircuit **`)：接收线路句柄的缓冲区，长度为 `zne_fold_circuits_len(ptr, gate_names)`；每个元素都是 owned 的 `CCircuit *`，须逐个 `circuit_free`。
- `len` (`uintptr_t`)：缓冲区长度，必须等于 `zne_fold_circuits_len(ptr, gate_names)`。

返回 `int32_t`：0 成功；-1（`ptr` 或 `buffer` 为 NULL）；-4（`gate_names` 含未知门名或非法 UTF-8）；-8（`len` 与折叠层数不符）；-3（折叠失败）。

---

## 外推

`zne_extrapolate` 的 `method` 参数取以下标签：

| 常量 | 值 | 方法 |
| --- | --- | --- |
| `ZNE_EXTRAPOLATE_POLYNOMIAL` | 0 | 多项式拟合。 |
| `ZNE_EXTRAPOLATE_EXPONENTIAL` | 1 | 指数衰减拟合。 |

### zne_extrapolate(ptr, noisy_results, len, method, degree, out_value)

从各噪声因子下的含噪期望值外推零噪声期望值。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。
- `noisy_results` (`const double *`)：每个噪声因子一个期望值，按折叠层顺序排列。
- `len` (`uintptr_t`)：期望值数量。
- `method` (`uint8_t`)：外推方法标签，见上表。多项式方法使用 `degree`；指数方法要求全部值为正。
- `degree` (`uintptr_t`)：多项式次数，必须小于数据点数。
- `out_value` (`double *`)：写出外推结果。

返回 `int32_t`：0 成功；-1（`ptr`、`noisy_results` 或 `out_value` 为 NULL）；-8（`method` 非法、结果为空、`len` 与噪声因子数不符、多项式次数不合法或指数拟合遇到非正值）；-7（其他拟合错误）。

### zne_poly_extrapolate(ptr, noisy_results, len, degree, out_value)

用给定次数的多项式拟合外推零噪声期望值，等价于 `zne_extrapolate` 的多项式分支。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。
- `noisy_results` (`const double *`)：每个噪声因子一个期望值，按折叠层顺序排列。
- `len` (`uintptr_t`)：期望值数量。
- `degree` (`uintptr_t`)：多项式次数，必须小于噪声因子数。
- `out_value` (`double *`)：写出外推结果。

返回 `int32_t`：0 成功；-1（`ptr`、`noisy_results` 或 `out_value` 为 NULL）；-8（结果为空、`len` 与噪声因子数不符或次数不合法）；-7（其他拟合错误）。

### zne_exp_extrapolate(ptr, noisy_results, len, out_value)

用指数衰减模型 `y(x) = A · exp(-x / τ)` 拟合外推零噪声期望值，写出 `x = 0` 处的拟合值 `A`。要求全部输入值为正。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。
- `noisy_results` (`const double *`)：每个噪声因子一个期望值，按折叠层顺序排列。
- `len` (`uintptr_t`)：期望值数量。
- `out_value` (`double *`)：写出拟合值 `A`。

返回 `int32_t`：0 成功；-1（`ptr`、`noisy_results` 或 `out_value` 为 NULL）；-8（结果为空、`len` 与噪声因子数不符或存在非正值）；-7（其他拟合错误）。

---

## 缓解序列

### zne_run_em_sequence_len(ptr)

返回缓解序列产出的期望值数量（每个折叠层一个）。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。

返回 `uintptr_t`：期望值数量；`ptr` 为 NULL 时返回 0。

### zne_run_em_sequence_with_shots_len(ptr)

返回带显式 shot 数的缓解序列产出的期望值数量（每个折叠层一个），与 `zne_run_em_sequence_len` 相同。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。

返回 `uintptr_t`：期望值数量；`ptr` 为 NULL 时返回 0。

### zne_run_em_sequence(ptr, gate_names, hamiltonian, estimator, buffer, len)

执行缓解序列：在每个配置的折叠层上折叠线路，对每条折叠线路经 `estimator` 回调估计一个期望值。回调收到的 shot 数为 0（「未指定」）。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。
- `gate_names` (`const char *`)：可选的逗号分隔门名列表，传 NULL 时全局折叠。
- `hamiltonian` (`const struct CHamiltonian *`)：待估计的可观测量，构造方式见 [哈密顿量](../3_qis/7_hamiltonian.md)。
- `estimator` (`CEstimatorFn`)：估计器回调，契约见 [模块概览](0_overview.md)。
- `buffer` (`double *`)：接收期望值的缓冲区，长度为 `zne_run_em_sequence_len(ptr)`。
- `len` (`uintptr_t`)：缓冲区长度，必须等于 `zne_run_em_sequence_len(ptr)`。

返回 `int32_t`：0 成功；-1（`ptr`、`hamiltonian` 或 `buffer` 为 NULL）；-4（`gate_names` 含未知门名或非法 UTF-8）；-8（`len` 与噪声因子数不符）；-3（折叠失败）；-7（其他执行错误）。

### zne_run_em_sequence_with_shots(ptr, gate_names, hamiltonian, shots, estimator, buffer, len)

与 `zne_run_em_sequence` 相同，但把 `shots` 转发给估计器回调作为建议采样数。

- `ptr` (`const struct CZneMitigation *`)：ZNE 缓解句柄。
- `gate_names` (`const char *`)：可选的逗号分隔门名列表，传 NULL 时全局折叠。
- `hamiltonian` (`const struct CHamiltonian *`)：待估计的可观测量。
- `shots` (`uintptr_t`)：转发给估计器的 shot 数。
- `estimator` (`CEstimatorFn`)：估计器回调。
- `buffer` (`double *`)：接收期望值的缓冲区，长度为 `zne_run_em_sequence_with_shots_len(ptr)`。
- `len` (`uintptr_t`)：缓冲区长度，必须等于 `zne_run_em_sequence_with_shots_len(ptr)`。

返回 `int32_t`：错误码同 `zne_run_em_sequence`。

---

## 示例

折叠 → 逐线路估计 → 多项式外推的完整流程（估计器用态矢量精确模拟，见 [态矢量](../3_qis/1_statevector.md)）：

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

/* 估计器回调：精确模拟折叠线路并计算 <ZZ> */
static void estimate(const struct CCircuit *circuit,
                     const struct CHamiltonian *hamiltonian,
                     uintptr_t shots,
                     double *expectation,
                     double *variance) {
    struct CStatevector *sv = statevector_from_circuit(circuit);
    double value = 0.0;
    statevector_expectation(sv, hamiltonian, &value);
    statevector_free(sv);
    *expectation = value;
    *variance = 0.0;
    (void)shots;
}

int main(void) {
    /* Bell 线路：H(0) + CX(0, 1) */
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    /* 折叠层 {0, 1, 2} → 噪声因子 {1, 3, 5} */
    int32_t levels[3] = {0, 1, 2};
    struct CZneMitigation *zne = zne_mitigation_new(qc, levels, 3);

    struct CPauliString *zz = pauli_string_parse("ZZ");
    struct CHamiltonian *obs = hamiltonian_from_pauli(zz);  /* takes ownership of zz */

    /* 折叠 + 逐线路估计 → 每个噪声因子一个期望值 */
    uintptr_t len = zne_run_em_sequence_len(zne);
    double noisy_results[3];
    zne_run_em_sequence(zne, NULL, obs, estimate, noisy_results, len);

    /* 多项式外推回零噪声极限 */
    double value = 0.0;
    zne_extrapolate(zne, noisy_results, len, ZNE_EXTRAPOLATE_POLYNOMIAL, 2, &value);
    printf("ZNE estimate: %f\n", value);

    zne_mitigation_free(zne);
    hamiltonian_free(obs);
    circuit_free(qc);
    return 0;
}
```

细粒度路径：先折叠成 `CCircuitList` 遍历，或用两步式接口直接取得 owned 线路数组：

```c
/* 折叠成列表后逐个读取（每次 get 返回 owned 副本） */
struct CCircuitList *folded = zne_mitigation_fold_circuit(zne, "H,CX");
for (uintptr_t i = 0; i < circuit_list_len(folded); i++) {
    struct CCircuit *c = circuit_list_get(folded, i);
    int32_t factor = zne_mitigation_noise_factor(zne, i);  /* noise factor of level i */
    /* 在此估计线路 c 上的期望值 */
    circuit_free(c);
}
circuit_list_free(folded);

/* 两步式：缓冲区接收 owned 线路句柄，逐个释放 */
uintptr_t m = zne_fold_circuits_len(zne, NULL);
struct CCircuit **buf = malloc(m * sizeof(struct CCircuit *));
zne_fold_circuits(zne, NULL, buf, m);
for (uintptr_t i = 0; i < m; i++) {
    circuit_free(buf[i]);
}
free(buf);
```
