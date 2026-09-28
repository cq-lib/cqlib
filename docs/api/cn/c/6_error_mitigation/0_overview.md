# 错误缓解（C）

错误缓解模块通过三种互补的流水线抑制噪声对期望值估计的影响：零噪声外推（ZNE）在多个噪声因子下折叠线路并外推回零噪声极限，虚拟蒸馏（VD）利用多份拷贝与 copy-swap 线路估计比值 `Tr(O ρ^M) / Tr(ρ^M)`，统一流水线 `CErrorMitigation` 把两者封装为「采样 → 后处理」两个阶段。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

三个子模块共享同一个估计器回调类型 `CEstimatorFn`：缓解流程把待估计的线路交给回调，由回调给出期望值与方差。

---

## 页面导航

| 页面 | 内容 |
| --- | --- |
| [零噪声外推](1_zne.md) | `CZneMitigation`：折叠线路、噪声因子、外推与缓解序列；`CCircuitList` 遍历。 |
| [虚拟蒸馏](2_virtual_distillation.md) | `CVirtualDistillation`：copy-swap 线路、分子/分母电路与完整协议。 |
| [统一流水线](3_unified.md) | `CErrorMitigation`：`new → run → get_mitigated` 两阶段流程。 |

---

## 常量

缓解方法标签（`error_mitigation_new` 的 `method` 参数）：

| 常量 | 值 | 方法 | 附加参数 |
| --- | --- | --- | --- |
| `MITIGATION_ZNE` | 0 | 零噪声外推。 | `fold_levels` 折叠层数组。 |
| `MITIGATION_VIRTUAL_DISTILLATION` | 1 | 虚拟蒸馏。 | `copies` 副本数。 |

后处理方法标签（`error_mitigation_get_mitigated` 的 `process_method` 参数）：

| 常量 | 值 | 流程 |
| --- | --- | --- |
| `PROCESS_ZNE_POLYNOMIAL` | 0 | ZNE + 多项式外推。 |
| `PROCESS_ZNE_EXPONENTIAL` | 1 | ZNE + 指数外推。 |
| `PROCESS_VIRTUAL_DISTILLATION` | 2 | 虚拟蒸馏。 |

ZNE 外推方法标签（`zne_extrapolate` 的 `method` 参数）：

| 常量 | 值 | 方法 |
| --- | --- | --- |
| `ZNE_EXTRAPOLATE_POLYNOMIAL` | 0 | 多项式拟合。 |
| `ZNE_EXTRAPOLATE_EXPONENTIAL` | 1 | 指数衰减拟合。 |

---

## CEstimatorFn 回调契约

```c
typedef void (*CEstimatorFn)(const struct CCircuit *circuit,
                             const struct CHamiltonian *hamiltonian,
                             uintptr_t shots,
                             double *expectation,
                             double *variance);
```

缓解流程通过该回调取得期望值估计，契约如下：

- **指针有效期**：`circuit`、`hamiltonian` 及全部出参指针仅在回调执行期间有效；回调返回后不得继续使用，也不得保存到回调之外。
- **hamiltonian 所有权**：`hamiltonian` 句柄归库所有，回调不得释放它。回调收到的哈密顿量可能已按流程需要扩展到折叠/拷贝线路的宽度。
- **出参写入**：回调必须通过 `expectation` 写入期望值；方差已知时写入 `variance`，未知时写入 NaN。虚拟蒸馏的分母电路估计中 `hamiltonian` 为 NULL，回调按无可观测量处理。
- **shots 语义**：`shots` 为 0 表示「未指定」，回调自行决定采样规模；非 0 时为调用方建议的采样数。

回调内部可自由调用模拟器接口（如 [态矢量](../3_qis/1_statevector.md) 的 `statevector_from_circuit` + `statevector_expectation`）完成估计。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

/* 估计器回调：精确模拟折叠线路并计算 <H> */
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
    *variance = 0.0;  /* unknown variance: NaN is also valid */
    (void)shots;
}

int main(void) {
    struct CCircuit *qc = circuit_new(1);
    circuit_x(qc, 0);

    /* 统一流水线：ZNE，折叠层 {0, 1, 2} */
    int32_t levels[3] = {0, 1, 2};
    struct CErrorMitigation *em =
        error_mitigation_new(qc, MITIGATION_ZNE, levels, 3, 0);

    struct CPauliString *z = pauli_string_parse("Z");
    struct CHamiltonian *obs = hamiltonian_from_pauli(z);

    /* 采样阶段 → 后处理阶段 */
    error_mitigation_run(em, MITIGATION_ZNE, obs, 0, 0, estimate);
    double value = 0.0, variance = 0.0;
    error_mitigation_get_mitigated(em, PROCESS_ZNE_POLYNOMIAL, 0,
                                   &value, &variance);

    error_mitigation_free(em);
    hamiltonian_free(obs);
    circuit_free(qc);
    return 0;
}
```

低层入口（ZNE 折叠/外推的细粒度控制、虚拟蒸馏分子/分母分离执行）见 [零噪声外推](1_zne.md) 与 [虚拟蒸馏](2_virtual_distillation.md)。
