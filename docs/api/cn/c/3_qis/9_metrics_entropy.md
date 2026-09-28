# 度量与熵（C）

本页覆盖量子信息度量（纯度、保真度、迹距离）与熵及纠缠度量（Von Neumann 熵、Renyi 熵、纠缠熵、负性、并发度、生成纠缠度），输入为态矢量 `CStatevector` 或密度矩阵 `CDensityMatrix` 句柄。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 共同约定

- 所有函数返回 `int32_t`：0 成功（数值写入 `out`），-1 空指针，其余负值为模拟层错误（典型 -8：维度不符、非归一化输入、非双比特输入）。
- 子系统参数（`qubits`/`subsys_a`/`sys_a` + `len`）：`len` 为 0 表示空子系统，合法；`len > 0` 且指针为 NULL 返回 -1。
- `density_matrix_partial_transpose` 返回新句柄；其余函数通过 out 指针写出数值。

---

## 纯度与保真度

### statevector_purity(sv, out)

计算态矢量的纯度范数（归一化纯态为 1.0）。

返回：0 成功；-1 空指针。

### density_matrix_purity(dm, out)

计算密度矩阵的纯度 `Tr(rho^2)`。

返回：0 成功；-1 空指针。

### statevector_fidelity(sv1, sv2, out)

计算两个态矢量之间的态保真度 `|<psi|phi>|^2`。

返回：0 成功；-1 空指针。

### statevector_fidelity_pure_mixed(sv, dm, out)

计算纯态与混合态之间的保真度 `<psi|rho|psi>`。

返回：0 成功；-1 空指针。

### density_matrix_fidelity(dm1, dm2, out)

计算两个密度矩阵之间的态保真度 `(Tr sqrt(sqrt(rho) sigma sqrt(rho)))^2`。

返回：0 成功；-1 空指针。

---

## 迹距离

### statevector_trace_distance(sv1, sv2, out)

计算两个纯态之间的迹距离 `sqrt(1 - |<psi|phi>|^2)`。

返回：0 成功；-1 空指针。

### density_matrix_trace_distance(dm1, dm2, out)

计算两个密度矩阵之间的迹距离 `1/2 Tr|rho - sigma|`。

返回：0 成功；-1 空指针。

---

## 熵

### density_matrix_entropy(dm, out)

计算密度矩阵的 Von Neumann 熵 `-Tr(rho log2 rho)`（单位比特）。

返回：0 成功；-1 空指针。

### density_matrix_linear_entropy(dm, out)

计算线性熵 `S_L = 1 - Tr(rho^2)`。

返回：0 成功；-1 空指针。

### density_matrix_renyi_entropy(dm, alpha, out)

计算阶数 `alpha` 的 Renyi 熵（以 2 为底）。`alpha` 必须为正的有限值；接近 1 时回退到 Von Neumann 熵。

参数：

- `alpha` (`double`)：Renyi 阶数。

返回：0 成功；-1 空指针；-8 `alpha` 非有限值或非正。

---

## 纠缠度量

### statevector_entanglement_entropy(sv, subsys_a, len, out)

计算纯态相对子系统 A 的二分纠缠熵（单位比特）：`subsys_a` 为 `len` 个比特下标。

返回：0 成功；-1 空指针；-8 比特下标或维度非法。

### statevector_entanglement_entropy_pure(sv, subsys_a, len, out)

计算态矢量相对子系统 A 的纯态二分纠缠熵（以 2 为底，单位比特）：`subsys_a` 为 `len` 个比特下标；Bell 态取子系统 `{0}` 时结果为 `1.0`。

返回：0 成功；-1 空指针；-8 比特下标或维度非法。

### density_matrix_negativity(dm, subsys_a, len, out)

计算二分密度矩阵的负性纠缠度量：对子系统 A 转置后求迹范数。

返回：0 成功；-1 空指针；-8 比特下标或维度非法。

### density_matrix_logarithmic_negativity(dm, sys_a, len, out)

计算对数负性 `log2 ||rho^{T_A}||_1`（子系统 A 由 `sys_a` 指定）。

返回：0 成功；-1 空指针；-8 比特下标或维度非法。

### density_matrix_concurrence(dm, out)

计算双比特密度矩阵的并发度：可分离态为 0，最大纠缠态为 1。

返回：0 成功；-1 空指针；-8 输入不是双比特系统。

### density_matrix_entanglement_of_formation(dm, out)

计算双比特密度矩阵的生成纠缠度（单位比特），由并发度导出。

返回：0 成功；-1 空指针；-8 输入不是双比特系统。

---

## 部分转置

### density_matrix_partial_transpose(dm, qubits, len)

对 `qubits`（`len` 个下标）执行部分转置，返回新密度矩阵。

返回：成功返回新建的 `CDensityMatrix*`（用 `density_matrix_free` 释放）；空指针（含 `len > 0` 且数组为 NULL）或失败返回 NULL。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Bell 态与其密度矩阵 */
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);
    struct CStatevector *sv = statevector_from_circuit(qc);
    struct CDensityMatrix *dm = density_matrix_from_circuit(qc);

    /* 2. 纯度与保真度 */
    double purity = 0.0;
    density_matrix_purity(dm, &purity);        /* 1.0：纯态 */

    struct CCircuit *qc2 = circuit_new(2);
    circuit_h(qc2, 0);
    circuit_cx(qc2, 0, 1);
    struct CStatevector *sv2 = statevector_from_circuit(qc2);
    double fid = 0.0;
    statevector_fidelity(sv, sv2, &fid);       /* 1.0：同一状态 */

    /* 3. Von Neumann 熵与纠缠熵 */
    double entropy = 0.0;
    density_matrix_entropy(dm, &entropy);     /* 0.0：纯态 */

    uint32_t subsystem[1] = {0};
    double entanglement = 0.0;
    statevector_entanglement_entropy(sv, subsystem, 1, &entanglement);  /* 1.0 比特 */

    /* 4. 部分转置与负性 */
    double negativity = 0.0;
    density_matrix_negativity(dm, subsystem, 1, &negativity);  /* 0.5 */

    struct CDensityMatrix *pt = density_matrix_partial_transpose(dm, subsystem, 1);
    if (pt != NULL) {
        density_matrix_free(pt);
    }

    statevector_free(sv2);
    circuit_free(qc2);
    density_matrix_free(dm);
    statevector_free(sv);
    circuit_free(qc);
    return 0;
}
```
