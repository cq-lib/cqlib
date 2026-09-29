# 虚拟蒸馏（C）

虚拟蒸馏通过多份密度矩阵拷贝与 copy-swap 线路估计比值 `Tr(O ρ^M) / Tr(ρ^M)`，其中 `M` 为副本数，`O` 为哈密顿量表示的可观测量。本页覆盖 `CVirtualDistillation` 句柄的创建与配置、copy-swap 线路构造、分子/分母线路的分离执行与完整协议。回调契约见 [模块概览](0_overview.md)，错误码与句柄释放约定见 [Overview](../0_overview.md)。

---

## 句柄与配置

### virtual_distillation_new(circuit, copies)

基于基底线路与副本数创建虚拟蒸馏句柄。后续所有派生线路都由基底线路构造。

- `circuit` (`const struct CCircuit *`)：基底线路。
- `copies` (`uintptr_t`)：副本数，最少为 2。

返回 `struct CVirtualDistillation *`：新建句柄，用 `virtual_distillation_free` 释放；`circuit` 为 NULL 或 `copies` 小于 2 时返回 NULL。

### virtual_distillation_free(ptr)

释放虚拟蒸馏句柄。

- `ptr` (`struct CVirtualDistillation *`)：待释放句柄，允许传 NULL。

### virtual_distillation_copies(ptr)

返回当前配置的副本数。

- `ptr` (`const struct CVirtualDistillation *`)：虚拟蒸馏句柄。

返回 `uintptr_t`：副本数；`ptr` 为 NULL 时返回 0。

### virtual_distillation_set_copies(ptr, copies)

更新副本数。更新失败时保持原值不变。

- `ptr` (`struct CVirtualDistillation *`)：虚拟蒸馏句柄。
- `copies` (`uintptr_t`)：新副本数，最少为 2。

返回 `int32_t`：0 成功；-1（`ptr` 为 NULL）；-8（`copies` 小于 2）。

---

## copy-swap 线路

### virtual_distillation_build_circuit(ptr)

构造虚拟蒸馏协议使用的 copy-swap 线路（Hadamard-test 线路）：先对基底线路做门分解，再把基底线路并排准备 `copies` 份——第 `i` 份整体平移到从 `i × 基底宽度` 开始的比特区间，各份之间互不重叠——然后在末尾的辅助比特（最后一个比特）上施加 `H` 门制备到 `|+⟩`，最后在相邻副本之间施加由辅助比特控制的 `SWAP` 梯（CSWAP），实现各副本的轮换（derangement）。

- `ptr` (`const struct CVirtualDistillation *`)：虚拟蒸馏句柄。

线路宽度为 `copies × 基底宽度 + 1`（基底宽度取分解后线路的宽度，最后一个比特是辅助比特）；操作数为 `copies` 份基底操作加上 1 个 `H` 与 `(copies - 1) × 基底宽度` 个受控 `SWAP`。

返回 `struct CCircuit *`：copy-swap 线路的 owned 句柄，用 `circuit_free` 释放；`ptr` 为 NULL 或构造失败时返回 NULL。

---

## 分子与分母

估计器回调在两种线路上都会收到非 NULL 的 `hamiltonian`，按内容区分：分子可观测量作用在第 0 份副本的比特上并在辅助比特上携带 `X`；分母可观测量只在辅助比特上携带 `X`（其余为恒等）。

### virtual_distillation_run_numerator_circuit(ptr, hamiltonian, shots, estimator, out_mean, out_variance)

通过估计器回调执行分子线路，估计 `Tr(O ρ^M)`。估计器收到 copy-swap 线路、扩展到 copy-swap 全宽的哈密顿量与 `shots`。扩展规则：原 Pauli 项保持比特索引（即作用在第 0 份副本上）、相位与系数不变，其余副本补恒等 `I`，并在辅助比特（最后一个比特）上追加 `X`。

- `ptr` (`const struct CVirtualDistillation *`)：虚拟蒸馏句柄。
- `hamiltonian` (`const struct CHamiltonian *`)：可观测量，比特数须与基底线路宽度一致，构造方式见 [哈密顿量](../3_qis/7_hamiltonian.md)。
- `shots` (`uintptr_t`)：转发给估计器的 shot 数。
- `estimator` (`CEstimatorFn`)：估计器回调，契约见 [模块概览](0_overview.md)。
- `out_mean` (`double *`)：写出估计均值。
- `out_variance` (`double *`)：写出估计方差。

返回 `int32_t`：0 成功；-1（`ptr`、`hamiltonian`、`out_mean` 或 `out_variance` 为 NULL）；-3（哈密顿量比特数与基底线路宽度不符，或 copy-swap 线路构造失败）；-7（其他执行错误）。

### virtual_distillation_run_denominator_circuit(ptr, shots, estimator, out_mean, out_variance)

通过估计器回调执行分母线路，估计 `Tr(ρ^M)`。估计器收到 copy-swap 线路、辅助比特（最后一个比特）上为 `X` 且其余比特为恒等的哈密顿量，以及 `shots`。

- `ptr` (`const struct CVirtualDistillation *`)：虚拟蒸馏句柄。
- `shots` (`uintptr_t`)：转发给估计器的 shot 数。
- `estimator` (`CEstimatorFn`)：估计器回调。
- `out_mean` (`double *`)：写出估计均值。
- `out_variance` (`double *`)：写出估计方差。

返回 `int32_t`：0 成功；-1（`ptr`、`out_mean` 或 `out_variance` 为 NULL）；-3（执行失败）。

### virtual_distillation_build_copy_swap_circuit(ptr)

从基底线路构造 copy-swap 线路：先对基底线路做门分解，再把基底线路并排准备 `copies` 份——第 `i` 份整体平移到从 `i × 基底宽度` 开始的比特区间，各份之间互不重叠——然后在末尾的辅助比特（最后一个比特）上施加 `H` 门制备到 `|+⟩`，最后在相邻副本之间施加由辅助比特控制的 `SWAP` 梯，实现各副本的轮换。线路宽度为 `copies × 基底宽度 + 1`（基底宽度取分解后线路的宽度）；操作数为 `copies` 份基底操作加上 1 个 `H` 与 `(copies - 1) × 基底宽度` 个受控 `SWAP`。

- `ptr` (`const struct CVirtualDistillation *`)：虚拟蒸馏句柄。

返回 `struct CCircuit *`：copy-swap 线路的 owned 句柄，用 `circuit_free` 释放；`ptr` 为 NULL 或构造失败时返回 NULL。

---

## 完整协议

### virtual_distillation_run_vd(ptr, hamiltonian, shots_numerator, shots_denominator, estimator, out_mean, out_variance)

执行完整虚拟蒸馏协议：先校验哈密顿量的比特数等于基底线路宽度，再依次执行分子线路与分母线路，最后组合为缓解结果。分子与分母使用各自的 shot 数。

- `ptr` (`const struct CVirtualDistillation *`)：虚拟蒸馏句柄。
- `hamiltonian` (`const struct CHamiltonian *`)：可观测量，比特数须与基底线路宽度一致。
- `shots_numerator` (`uintptr_t`)：分子线路的 shot 数。
- `shots_denominator` (`uintptr_t`)：分母线路的 shot 数。
- `estimator` (`CEstimatorFn`)：估计器回调。
- `out_mean` (`double *`)：写出缓解后的期望值 `mu_num / mu_den`。
- `out_variance` (`double *`)：写出缓解后的方差。

返回 `int32_t`：0 成功；-1（任一指针为 NULL）；-8（哈密顿量比特数与基底线路宽度不符）；-7（分母均值为零或其他执行错误）。

方差按一阶泰勒近似、假设分子与分母相互独立组合：`var_num / mu_den^2 + mu_num^2 * var_den / mu_den^4`。采样随机性由估计器决定（核心层不接收随机种子）。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

/* 分子与分母都携带哈密顿量——分子作用在第 0 份副本的比特上并在辅助
   比特上携带 X，分母只在辅助比特上携带 X；精确模拟线路并求可观测量期望 */
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
    /* 两份单比特副本 */
    struct CCircuit *qc = circuit_new(1);
    circuit_x(qc, 0);
    struct CVirtualDistillation *vd = virtual_distillation_new(qc, 2);

    /* copy-swap 线路：宽度 = 2 × 1 + 1 = 3 */
    struct CCircuit *copy_swap = virtual_distillation_build_circuit(vd);

    struct CPauliString *z = pauli_string_parse("Z");
    struct CHamiltonian *obs = hamiltonian_from_pauli(z);  /* takes ownership of z */

    /* 对 rho = |1><1|：分子 Tr(Z rho^2) = -1，分母 Tr(rho^2) = 1 */
    double mean = 0.0, variance = 0.0;
    virtual_distillation_run_vd(vd, obs, 3, 2, estimate, &mean, &variance);
    printf("VD estimate: %f\n", mean);

    circuit_free(copy_swap);
    virtual_distillation_free(vd);
    hamiltonian_free(obs);
    circuit_free(qc);
    return 0;
}
```

分子与分母也可以分离执行，各自取得均值与方差：

```c
double num_mean = 0.0, num_var = 0.0;
virtual_distillation_run_numerator_circuit(vd, obs, 512, estimate,
                                           &num_mean, &num_var);
double den_mean = 0.0, den_var = 0.0;
virtual_distillation_run_denominator_circuit(vd, 512, estimate,
                                             &den_mean, &den_var);
```
