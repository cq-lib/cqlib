# 统一流水线（C）

统一流水线 `CErrorMitigation` 把零噪声外推与虚拟蒸馏封装为「采样 → 后处理」两个阶段：`error_mitigation_run` 折叠/派生线路并经估计器回调采样，`error_mitigation_get_mitigated` 在采样完成后产出缓解结果。回调契约与常量见 [模块概览](0_overview.md)，错误码与句柄释放约定见 [Overview](../0_overview.md)。

`method` 参数的取值：

| 常量 | 值 | 方法 | 附加参数 |
| --- | --- | --- | --- |
| `MITIGATION_ZNE` | 0 | 零噪声外推。 | `fold_levels` 折叠层数组。 |
| `MITIGATION_VIRTUAL_DISTILLATION` | 1 | 虚拟蒸馏。 | `copies` 副本数。 |

`process_method` 参数的取值：

| 常量 | 值 | 流程 |
| --- | --- | --- |
| `PROCESS_ZNE_POLYNOMIAL` | 0 | ZNE + 多项式外推。 |
| `PROCESS_ZNE_EXPONENTIAL` | 1 | ZNE + 指数外推。 |
| `PROCESS_VIRTUAL_DISTILLATION` | 2 | 虚拟蒸馏。 |

---

## error_mitigation_new(circuit, method, fold_levels, num_levels, copies)

基于基底线路与缓解方法创建统一流水线句柄。

- `circuit` (`const struct CCircuit *`)：基底线路。
- `method` (`uint8_t`)：缓解方法标签，见上表。
- `fold_levels` (`const int32_t *`)：折叠层数组，`method == MITIGATION_ZNE` 时使用（如 `{0, 1, 2}`）。
- `num_levels` (`uintptr_t`)：折叠层数组的元素个数，`method == MITIGATION_ZNE` 时必须大于 0。
- `copies` (`uintptr_t`)：副本数，`method == MITIGATION_VIRTUAL_DISTILLATION` 时使用，最少为 2。

返回 `struct CErrorMitigation *`：新建句柄，用 `error_mitigation_free` 释放；`circuit` 为 NULL、`method` 不是合法标签、参数与方法不匹配（ZNE 缺折叠层、VD 副本数小于 2）时返回 NULL。

---

## error_mitigation_free(ptr)

释放统一流水线句柄。

- `ptr` (`struct CErrorMitigation *`)：待释放句柄，允许传 NULL。

---

## error_mitigation_run(ptr, method, hamiltonian, shots_a, shots_b, estimator)

执行采样阶段：按配置的方法折叠/派生线路，对每条待执行线路调用一次估计器回调。`method` 必须与 `error_mitigation_new` 配置的方法一致。

- `ptr` (`struct CErrorMitigation *`)：统一流水线句柄。
- `method` (`uint8_t`)：缓解方法标签，必须与创建时一致。
- `hamiltonian` (`const struct CHamiltonian *`)：待估计的可观测量，构造方式见 [哈密顿量](../3_qis/7_hamiltonian.md)。
- `shots_a` (`uintptr_t`)：ZNE 方法下的 shot 数（0 = 未指定）；虚拟蒸馏方法下为分子线路的 shot 数。
- `shots_b` (`uintptr_t`)：ZNE 方法下被忽略；虚拟蒸馏方法下为分母线路的 shot 数。
- `estimator` (`CEstimatorFn`)：估计器回调，契约见 [模块概览](0_overview.md)。

返回 `int32_t`：0 成功；-1（`ptr` 或 `hamiltonian` 为 NULL）；-8（`method` 不是合法标签）；-7（`method` 与配置的方法不一致或其他执行错误）。

---

## error_mitigation_get_mitigated(ptr, process_method, degree, out_expectation, out_variance)

执行后处理阶段：在 `error_mitigation_run` 之后产出缓解结果。

- `ptr` (`struct CErrorMitigation *`)：统一流水线句柄。
- `process_method` (`uint8_t`)：后处理方法标签，见上表。
- `degree` (`uintptr_t`)：ZNE 多项式外推的次数，0 表示自动选择。
- `out_expectation` (`double *`)：写出缓解后的期望值。
- `out_variance` (`double *`)：写出缓解后的方差，不可用时写 NaN。

返回 `int32_t`：0 成功；-1（`ptr`、`out_expectation` 或 `out_variance` 为 NULL）；-8（`process_method` 不是合法标签）；-7（`process_method` 与配置的方法不一致或其他后处理错误）。

---

## 示例

虚拟蒸馏方法的完整流程（分子线路带可观测量、分母线路无可观测量的回调分支，见 [模块概览](0_overview.md)）：

```c
#include <stdio.h>
#include "cqlib_c.h"

/* 估计器回调：分子线路带可观测量（精确模拟），分母线路无可观测量 */
static void estimate(const struct CCircuit *circuit,
                     const struct CHamiltonian *hamiltonian,
                     uintptr_t shots,
                     double *expectation,
                     double *variance) {
    if (hamiltonian != NULL) {
        struct CStatevector *sv = statevector_from_circuit(circuit);
        double value = 0.0;
        statevector_expectation(sv, hamiltonian, &value);
        statevector_free(sv);
        *expectation = value;
        *variance = 0.0;
    } else {
        /* 分母 Tr(rho^M)：由采样型估计器从测量结果估计，此处返回示例值 */
        *expectation = 1.0;
        *variance = 0.0;
    }
    (void)shots;
}

int main(void) {
    struct CCircuit *qc = circuit_new(1);
    circuit_x(qc, 0);

    /* 统一流水线：虚拟蒸馏，3 份拷贝 */
    struct CErrorMitigation *em =
        error_mitigation_new(qc, MITIGATION_VIRTUAL_DISTILLATION, NULL, 0, 3);

    struct CPauliString *z = pauli_string_parse("Z");
    struct CHamiltonian *obs = hamiltonian_from_pauli(z);  /* takes ownership of z */

    /* 采样阶段：分子 shots=512、分母 shots=256 */
    error_mitigation_run(em, MITIGATION_VIRTUAL_DISTILLATION, obs, 512, 256, estimate);

    /* 后处理阶段：直接得到缓解后的期望值与方差 */
    double value = 0.0, variance = 0.0;
    error_mitigation_get_mitigated(em, PROCESS_VIRTUAL_DISTILLATION, 0,
                                   &value, &variance);

    error_mitigation_free(em);
    hamiltonian_free(obs);
    circuit_free(qc);
    return 0;
}
```

ZNE 方法的 `new → run → get_mitigated` 流程示例见 [模块概览](0_overview.md)；对折叠与外推的细粒度控制见 [零噪声外推](1_zne.md)，虚拟蒸馏分子/分母的分离执行见 [虚拟蒸馏](2_virtual_distillation.md)。
