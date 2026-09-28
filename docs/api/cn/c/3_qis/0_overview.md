# QIS 量子信息（C）

C 绑定的量子信息接口以不透明句柄（`CStatevector`、`CDensityMatrix`、`CDensityMatrixNoise`、`CStabilizerState` 等）加自由函数的形式提供本地量子模拟：态的构造与演化、门操作、概率与测量、期望值以及噪声仿真。错误码、句柄与字符串的释放约定遵循 [Overview](../0_overview.md)。

---

## 三个稠密态模拟器

| 模拟器 | 态表示 | 数据规模 | 噪声支持 | 读出噪声 | 适用场景 |
| --- | --- | --- | --- | --- | --- |
| [Statevector](1_statevector.md) | 纯态 | `2^N` 个复振幅 | 无 | 无 | 理想通用线路，内存占用最小 |
| [DensityMatrix](2_density_matrix.md) | 混合态 | `4^N` 个矩阵元 | Kraus 算符信道 | 无 | 混合态、量子信道与偏迹 |
| [DensityMatrixNoise](3_density_matrix_noise.md) | 混合态 | `4^N` 个矩阵元 | 完整噪声模型（门噪声自动施加） | 有 | 设备级含噪声仿真 |

三个模拟器共享同一组门函数，命名规律为 `<模拟器>_apply_<门名>`：

```c
statevector_apply_h(ptr, qubit);
density_matrix_apply_h(ptr, qubit);
density_matrix_noise_apply_h(ptr, qubit);
```

门集合覆盖：无参数单比特门 H、X、Y、Z、S、S†、T、T†、X2P、X2M、Y2P、Y2M；参数化单比特门 Rx、Ry、Rz、P、U、XY、XY2P、XY2M、RXY、GPhase；双比特门 CX、CY、CZ、SWAP、RXX、RYY、RZZ、RZX、CrX、CrY、CrZ、FSIM；三比特门 CCX。每个模拟器还提供按名称分派的 `apply_standard_gate` 族与整线路入口 `apply_circuit`；Statevector 与 DensityMatrixNoise 另有任意酉矩阵入口 `apply_unitary_gate`。

三个模拟器都与线路句柄对接：`*_from_circuit` 执行一条线路并返回演化后的态；`*_apply_circuit` 把线路原地作用到已有态上，线路的比特数必须与态一致。线路句柄见 [Circuit](../0_circuit/1_circuit.md)。

---

## 共享约定

- **句柄生命周期**：`*_new`、`*_from_circuit`、`*_from_state`、`*_maximally_mixed` 等构造接口返回堆分配句柄，用对应的 `*_free` 释放；所有 `*_free` 允许传 `NULL`；构造失败返回 `NULL`。
- **错误码**：返回 `int32_t` 的接口用 `0` 表示成功、负值表示错误；测量接口 `*_measure` 用 `0`/`1` 表示测量结果 `|0>`/`|1>`，负值仍是错误。QIS 接口的典型错误码：`-1`（NULL 指针）、`-2`（比特编号越界）、`-3`（线路错误）、`-7`（不支持的操作）、`-8`（非法参数：非有限角度、缓冲区长度不符、未归一化的输入态、比特数不匹配、未知门名）。
- **两步式数组输出**：概率与矩阵数据先调用 `*_len` 接口查询元素数量，分配缓冲后调用填充接口（如 `statevector_probabilities_len` + `statevector_probabilities`、`density_matrix_data_len` + `density_matrix_data`）；`len` 与实际数量不符时填充接口返回 `-8`。
- **字符串**：`*_measure_all` 返回大端比特串（最高位是比特 `N-1`，最低位是比特 `0`），用 `cqlib_string_free` 释放。
- **Complex64**：`{ double re; double im; }`，振幅与矩阵元素按行主序、实部虚部交错排列。
- **采样结果**：`*_sample_shots` 返回 `COutcomeList*`，每个元素是一个大端比特串 `char*`；用 `outcome_list_len` 取数量、`outcome_list_get` 取元素（用 `cqlib_string_free` 释放）、`outcome_list_free` 释放列表，详见 [ClassicalState](5_classical_state.md)。
- **观测量**：期望值接口接受 `CHamiltonian*` 句柄，构造方式见 [Hamiltonian](7_hamiltonian.md)，其 Pauli 串成分见 [Pauli](6_pauli.md)。

---

## 页面导航

| 页面 | 内容 |
| --- | --- |
| [Statevector](1_statevector.md) | 纯态模拟器：振幅访问、门操作、概率、测量、采样与期望值。 |
| [DensityMatrix](2_density_matrix.md) | 混合态模拟器：矩阵数据、物理性校验、Kraus 信道、偏迹与期望值。 |
| [DensityMatrixNoise](3_density_matrix_noise.md) | 噪声模拟器：门噪声自动施加、含读出噪声的概率分布。 |
| [StabilizerState](4_stabilizer.md) | 稳定子态模拟器，支持大规模 Clifford 线路。 |
| [ClassicalState](5_classical_state.md) | 运行期经典数据与 `COutcomeList`。 |
| [Pauli](6_pauli.md) | Pauli 算子、Pauli 串及其矩阵表示。 |
| [Hamiltonian](7_hamiltonian.md) | 哈密顿量与可观测量。 |
| [Evolution](8_evolution.md) | Pauli 串演化门 `circuit_pauli_evolution`。 |
| [Metrics / Entropy](9_metrics_entropy.md) | 保真度、纯度、迹距离、熵与纠缠度量。 |

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* Run the same Bell circuit on both simulators */
    struct CStatevector *sv = statevector_new(2);
    struct CDensityMatrix *dm = density_matrix_new(2);

    statevector_apply_h(sv, 0);
    statevector_apply_cx(sv, 0, 1);
    density_matrix_apply_h(dm, 0);
    density_matrix_apply_cx(dm, 0, 1);

    /* Pure and mixed representations agree: P(00) = P(11) = 0.5 */
    double p_sv[4], p_dm[4];
    statevector_probabilities(sv, p_sv, 4);
    density_matrix_probabilities(dm, p_dm, 4);
    printf("P(00): sv=%.3f dm=%.3f\n", p_sv[0], p_dm[0]);
    printf("P(11): sv=%.3f dm=%.3f\n", p_sv[3], p_dm[3]);

    statevector_free(sv);
    density_matrix_free(dm);
    return 0;
}
```

噪声模型句柄 `CNoiseModel` 的构造与噪声登记见 [NoiseModel](../2_device/4_noise.md)。
