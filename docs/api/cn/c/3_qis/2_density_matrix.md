# DensityMatrix

密度矩阵模拟器，以 `2^N × 2^N` 密度矩阵表示量子态，支持混合态、噪声通道与子系统操作。

---

## 构造与释放

### density_matrix_new(num_qubits)

创建初始化为 `\|0...0⟩`（纯态）的密度矩阵。

返回：

- `CDensityMatrix *`：堆分配对象，需 `density_matrix_free` 释放；失败返回 NULL。

### density_matrix_from_circuit(circuit)

模拟 `circuit` 并返回末态密度矩阵。

返回：

- `CDensityMatrix *`；失败返回 NULL。

### density_matrix_free(ptr)

释放对象。允许传 NULL。

### density_matrix_num_qubits(ptr)

返回比特数；NULL 句柄返回 `0`。

---

## 门操作

所有门函数前缀 `density_matrix_apply_`，返回 `int32_t` 错误码：`0` 成功、`-2` 比特越界、`-7` 模拟失败、`-1` NULL 句柄。门集与 [Statevector](1_statevector.md#门操作) 一一对应，仅将前缀替换为 `density_matrix_apply_`：

| 分组 | 函数 |
| --- | --- |
| 无参单比特 | `apply_i/h/x/y/z/s/sdg/t/tdg/x2p/x2m/y2p/y2m(dm, qubit)` |
| 参数单比特 | `apply_rx/ry/rz/phase/xy/xy2p/xy2m(dm, qubit, theta)`、`apply_u(dm, qubit, theta, phi, lambda)`、`apply_rxy(dm, qubit, theta, phi)`、`apply_gphase(dm, phi)` |
| 双比特 | `apply_cx/cy/cz/swap(dm, q0, q1)`、`apply_rxx/ryy/rzz/rzx(dm, q0, q1, theta)`、`apply_crx/cry/crz(dm, control, target, theta)`、`apply_fsim(dm, q0, q1, theta, phi)` |
| 三比特 | `apply_ccx(dm, c0, c1, target)` |
| 整线路 | `apply_circuit(dm, circuit)`（原地执行线路） |

---

## 概率与测量

### density_matrix_probabilities_len(ptr) / density_matrix_probabilities(ptr, buffer, len)

两步式读取 `2^N` 个基态概率分布，语义与 [Statevector](1_statevector.md#概率与测量) 对应接口一致；`density_matrix_probabilities` 成功返回 `0`，失败返回负错误码。

### density_matrix_expectation(ptr, observable, out)

计算 `⟨H⟩` 写入 `out`；成功 `0`，失败负错误码。混合态下按 `Tr(Hρ)` 计算。

### density_matrix_measure(ptr, qubit)

Z 基测量 `qubit` 并坍缩。与 Statevector 不同，密度矩阵版本**不返回**测量结果（返回 `int32_t` 错误码，成功 `0`）。

### density_matrix_measure_all(ptr)

测量全部比特并坍缩，返回堆分配大端比特串，需 `cqlib_string_free` 释放；失败返回 NULL。

### density_matrix_sample_shots(ptr, shots)

独立采样 `shots` 次，返回 `COutcomeList *`，用法见[概览](0_overview.md#采样结果-outcomelist)。

### density_matrix_reset(ptr, qubit)

将 `qubit` 重置到 `\|0⟩`。成功 `0`，失败负错误码。

---

## Kraus 通道与部分迹

### density_matrix_apply_kraus(ptr, ops, num_ops, op_dim, qubits, num_target_qubits)

对目标比特施加 Kraus 噪声通道 `ρ -> Σ_i K_i ρ K_i†`。

参数：

- `ops` (`const Complex64 *`)：`num_ops × op_dim × op_dim` 个**行主序、实虚交错**的扁平数组，按 `K0, K1, ...` 顺序排列。
- `num_ops` (`uintptr_t`)：Kraus 算符个数。
- `op_dim` (`uintptr_t`)：单个算符的维度，必须等于 `2^num_target_qubits`。
- `qubits` (`const uint32_t *`)：目标比特数组。
- `num_target_qubits` (`uintptr_t`)：目标比特数。

返回：

- `int32_t`；成功 `0`，结构非法 `-3`，参数不匹配（`op_dim` 与比特数不符等）`-8`。

### density_matrix_partial_trace(ptr, keep_qubits, num_keep)

对 `keep_qubits` 之外的比特求偏迹，返回仅保留指定比特的降维密度矩阵。

参数：

- `keep_qubits` (`const uint32_t *`)：保留比特数组。
- `num_keep` (`uintptr_t`)：保留比特数。

返回：

- `CDensityMatrix *`：**独立拥有**的新对象，需 `density_matrix_free` 释放；失败返回 NULL。

---

## 示例

### 振幅阻尼通道

对 1 个目标比特施加振幅阻尼（`op_dim = 2`）：

```c
double gamma = 0.1;
Complex64 k0[2] = {{1.0, 0.0}, {0.0, 0.0},
                   {0.0, 0.0}, {sqrt(1.0 - gamma), 0.0}};   // diag(1, √(1-γ))
Complex64 k1[2] = {{0.0, 0.0}, {0.0, 0.0},
                   {sqrt(gamma), 0.0}, {0.0, 0.0}};         // √γ |0><1|

Complex64 ops[4];
memcpy(ops, k0, sizeof(k0));
memcpy(ops + 2, k1, sizeof(k1));

uint32_t target[1] = {0};
density_matrix_apply_kraus(dm, ops, 2, 2, target, 1);
```

### 部分迹与期望

```c
CDensityMatrix *dm = density_matrix_from_circuit(qc);

/* 纠缠后保留 q0，得到 q0 的约化密度矩阵 */
uint32_t keep[1] = {0};
CDensityMatrix *reduced = density_matrix_partial_trace(dm, keep, 1);

double probs[2];
density_matrix_probabilities(reduced, probs, 2);   // 各 0.5（最大混合）
density_matrix_free(reduced);

density_matrix_free(dm);
```
