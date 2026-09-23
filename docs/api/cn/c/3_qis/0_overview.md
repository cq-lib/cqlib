# 量子信息

QIS 模块提供三种量子态模拟器与可观测量工具。

---

## 模拟器对比

| 模拟器 | 页面 | 状态表示 | 门集 | 适用场景 |
| --- | --- | --- | --- | --- |
| Statevector | [Statevector](1_statevector.md) | `2^N` 复振幅 | 全部标准门 | N ≤ 30 比特精确模拟 |
| DensityMatrix | [DensityMatrix](2_density_matrix.md) | `2^N × 2^N` 密度矩阵 | 全部标准门 + Kraus 通道 | 混合态、噪声、子系统 |
| StabilizerState | [StabilizerState](3_stabilizer_state.md) | 稳定子表 | 仅 Clifford 门集 | 大规模（数千比特）Clifford 线路 |

三种模拟器共享同构的接口族：

- **构造**：`<sim>_new(n)` 创建 `\|0…0⟩` 初态；`<sim>_from_circuit(circuit)` 直接模拟线路得到末态。
- **门操作**：前缀 `apply_`，分组与 [Circuit 门操作](../0_circuit/1_circuit.md#门操作)一一对应；不支持的门返回 `-7`。
- **测量与采样**：`measure(qubit)` 单比特坍缩测量、`measure_all()` 全测量（返回大端比特串）、`sample_shots(shots)` 独立采样、`reset(qubit)` 重置、`probabilities_len/probabilities` 两步式读概率。
- **可观测量**：`expectation(hamiltonian, out)` 计算 `⟨H⟩`；StabilizerState 使用 `pauli_expectation`（结果恒为 `-1/0/+1`）。

可观测量工具：[PauliString](4_pauli_string.md)（Pauli 串）、[Hamiltonian](5_hamiltonian.md)（厄米算量）。

---

## 采样结果 OutcomeList

`statevector_sample_shots`、`density_matrix_sample_shots`、`stabilizer_sample_shots` 共用的返回类型：

```c
void outcome_list_free(struct COutcomeList *ptr);
uintptr_t outcome_list_len(const struct COutcomeList *ptr);
char *outcome_list_get(const struct COutcomeList *ptr, uintptr_t index);
```

| 函数 | 说明 |
| --- | --- |
| `outcome_list_len` | 采样结果数量 |
| `outcome_list_get(index)` | 返回大端二进制比特串（MSB 为最高位 qubit），堆分配需 `cqlib_string_free`，越界返回 NULL |
| `outcome_list_free` | 释放列表，允许传 NULL |

```c
COutcomeList *samples = statevector_sample_shots(sv, 1000);
for (uintptr_t i = 0; i < outcome_list_len(samples); i++) {
    char *s = outcome_list_get(samples, i);        // "00" / "11" / ...
    printf("%s\n", s);
    cqlib_string_free(s);
}
outcome_list_free(samples);
```
