# Statevector

态矢量模拟器，以 `2^N` 个复振幅精确表示纯态，支持全部标准门，适合 N ≤ 30 比特的精确模拟。

---

## 构造与释放

### statevector_new(num_qubits)

创建初始化为 `\|0...0⟩` 的态矢量。

参数：

- `num_qubits` (`uintptr_t`)：量子比特数量。

返回：

- `CStatevector *`：堆分配对象，需 `statevector_free` 释放；失败返回 NULL。

### statevector_from_circuit(circuit)

模拟 `circuit` 并返回末态。

参数：

- `circuit` (`const CCircuit *`)：输入线路。

返回：

- `CStatevector *`；失败（NULL、模拟失败等）返回 NULL。

### statevector_free(ptr)

释放态矢量。允许传 NULL。

### statevector_num_qubits(ptr)

返回比特数；NULL 句柄返回 `0`。

---

## 门操作

所有门函数前缀 `statevector_apply_`，返回 `int32_t` 错误码：`0` 成功、`-2` 比特越界、`-7` 模拟失败、`-1` NULL 句柄。分组与 [Circuit 门操作](../0_circuit/1_circuit.md#门操作)一一对应。

### 1. 无参数单比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `statevector_apply_i(sv, qubit)` | `I` | 恒等门 |
| `statevector_apply_h(sv, qubit)` | `H` | Hadamard 门 |
| `statevector_apply_x(sv, qubit)` | `X` | Pauli-X 门 |
| `statevector_apply_y(sv, qubit)` | `Y` | Pauli-Y 门 |
| `statevector_apply_z(sv, qubit)` | `Z` | Pauli-Z 门 |
| `statevector_apply_s(sv, qubit)` | `S` | S 门 |
| `statevector_apply_sdg(sv, qubit)` | `S†` | S 伴随门 |
| `statevector_apply_t(sv, qubit)` | `T` | T 门 |
| `statevector_apply_tdg(sv, qubit)` | `T†` | T 伴随门 |
| `statevector_apply_x2p(sv, qubit)` | `X2P` | 绕 X 轴 +π/2 旋转 |
| `statevector_apply_x2m(sv, qubit)` | `X2M` | 绕 X 轴 −π/2 旋转 |
| `statevector_apply_y2p(sv, qubit)` | `Y2P` | 绕 Y 轴 +π/2 旋转 |
| `statevector_apply_y2m(sv, qubit)` | `Y2M` | 绕 Y 轴 −π/2 旋转 |

### 2. 参数化单比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `statevector_apply_rx(sv, qubit, theta)` | `RX` | 绕 X 轴旋转 |
| `statevector_apply_ry(sv, qubit, theta)` | `RY` | 绕 Y 轴旋转 |
| `statevector_apply_rz(sv, qubit, theta)` | `RZ` | 绕 Z 轴旋转 |
| `statevector_apply_phase(sv, qubit, theta)` | `P` | 相位门 |
| `statevector_apply_u(sv, qubit, theta, phi, lambda)` | `U` | 通用单比特门 |
| `statevector_apply_xy(sv, qubit, theta)` | `XY` | XY 交互门 |
| `statevector_apply_xy2p(sv, qubit, theta)` | `XY2P` | 正半角 XY 门 |
| `statevector_apply_xy2m(sv, qubit, theta)` | `XY2M` | 负半角 XY 门 |
| `statevector_apply_rxy(sv, qubit, theta, phi)` | `RXY` | XY 平面任意轴旋转 |
| `statevector_apply_gphase(sv, phi)` | `gphase` | 全局相位 |

### 3. 双比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `statevector_apply_cx(sv, control, target)` | `CX` | CNOT 门 |
| `statevector_apply_cy(sv, control, target)` | `CY` | controlled-Y 门 |
| `statevector_apply_cz(sv, q0, q1)` | `CZ` | controlled-Z 门 |
| `statevector_apply_swap(sv, q0, q1)` | `SWAP` | 交换两比特状态 |
| `statevector_apply_rxx(sv, q0, q1, theta)` | `RXX` | `exp(-i θ XX / 2)` |
| `statevector_apply_ryy(sv, q0, q1, theta)` | `RYY` | `exp(-i θ YY / 2)` |
| `statevector_apply_rzz(sv, q0, q1, theta)` | `RZZ` | `exp(-i θ ZZ / 2)` |
| `statevector_apply_rzx(sv, q0, q1, theta)` | `RZX` | `exp(-i θ ZX / 2)` |
| `statevector_apply_crx(sv, control, target, theta)` | `CRX` | controlled-RX 门 |
| `statevector_apply_cry(sv, control, target, theta)` | `CRY` | controlled-RY 门 |
| `statevector_apply_crz(sv, control, target, theta)` | `CRZ` | controlled-RZ 门 |
| `statevector_apply_fsim(sv, q0, q1, theta, phi)` | `fSim` | fSim 门 |

### 4. 三比特门与整线路

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `statevector_apply_ccx(sv, c0, c1, target)` | `CCX` | Toffoli 门 |
| `statevector_apply_circuit(sv, circuit)` | — | **原地**执行整条线路（含测量/重置时同样生效） |

---

## 概率与测量

### statevector_probabilities_len(ptr)

返回基态概率数量 `2^N`，NULL 句柄返回 `0`。用于分配 `statevector_probabilities` 的 `buffer` 缓冲区。

### statevector_probabilities(ptr, buffer, len)

将 `2^N` 个基态概率（计算基 `|q_{N-1}...q_0⟩`，下标按大端二进制序）拷入 `buffer`。

返回：

- `int32_t`；成功 `0`，缓冲区不足等错误返回负错误码。

### statevector_expectation(ptr, observable, out)

计算可观测量期望值 `⟨H⟩` 写入 `out`。

参数：

- `observable` (`const CHamiltonian *`)：见 [Hamiltonian](5_hamiltonian.md)。

返回：

- `int32_t`；成功 `0`，失败返回负错误码。

### statevector_measure(ptr, qubit)

在 Z 基测量 `qubit` 并**坍缩**态矢量。

返回：

- `int32_t`：测量结果 `0`（\|0⟩）或 `1`（\|1⟩）；失败返回负错误码。

### statevector_measure_all(ptr)

测量全部比特并坍缩。

返回：

- `char *`：堆分配的大端比特串（MSB 为 qubit `N-1`，LSB 为 qubit `0`），需 `cqlib_string_free` 释放；失败返回 NULL。

### statevector_sample_shots(ptr, shots)

独立采样 `shots` 次测量结果（不坍缩原态）。

返回：

- `COutcomeList *`：堆分配采样列表，需 `outcome_list_free` 释放；用法见[概览](0_overview.md#采样结果-outcomelist)；失败返回 NULL。

### statevector_reset(ptr, qubit)

将 `qubit` 重置到 `\|0⟩`（测量后条件翻转）。

返回：

- `int32_t`；成功 `0`，失败返回负错误码。

---

## 完整示例

```c
/* 构造 Bell 线路并模拟 */
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

CStatevector *sv = statevector_from_circuit(qc);

/* 概率分布 */
double probs[4];
statevector_probabilities(sv, probs, 4);
/* probs = [0.5, 0, 0, 0.5] */

/* 采样 */
COutcomeList *samples = statevector_sample_shots(sv, 1000);
for (uintptr_t i = 0; i < outcome_list_len(samples); i++) {
    char *s = outcome_list_get(samples, i);
    printf("%s\n", s);
    cqlib_string_free(s);
}
outcome_list_free(samples);

/* 坍缩测量 */
int32_t bit = statevector_measure(sv, 0);      // 0 或 1
char *all = statevector_measure_all(sv);       // 如 "11"
cqlib_string_free(all);

statevector_free(sv);
circuit_free(qc);
```

### 逐门构建与哈密顿量期望

```c
CStatevector *sv = statevector_new(1);
statevector_apply_h(sv, 0);
statevector_apply_rz(sv, 0, 0.5);

/* ⟨Z⟩ */
CPauliString *z = pauli_string_parse("Z");
CHamiltonian *h = hamiltonian_from_pauli(z);   // 接管 z 所有权

double ez;
statevector_expectation(sv, h, &ez);
printf("<Z> = %f\n", ez);

hamiltonian_free(h);
statevector_free(sv);
```
