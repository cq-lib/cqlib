# StabilizerState

稳定子模拟器，以稳定子表表示量子态，仅支持 Clifford 门集，可模拟大规模（数千比特）电路。

---

## 构造与释放

### stabilizer_new(num_qubits)

创建 `\|0...0⟩` 初态的稳定子态。

返回：

- `CStabilizerState *`：堆分配对象，需 `stabilizer_free` 释放；失败返回 NULL。

### stabilizer_from_circuit(circuit)

模拟 Clifford 线路并返回末态。

返回：

- `CStabilizerState *`；线路含非 Clifford 门时模拟失败返回 NULL（逐门应用时返回 `-7`）。

### stabilizer_free(ptr)

释放对象。允许传 NULL。

### stabilizer_num_qubits(ptr)

返回比特数；NULL 句柄返回 `0`。

---

## 门操作

前缀 `stabilizer_apply_`，返回 `int32_t` 错误码：`0` 成功、`-2` 比特越界、`-7` 非 Clifford 门、`-1` NULL 句柄。

| 分组 | 函数 | 门 |
| --- | --- | --- |
| 无参单比特 | `apply_h/x/y/z/s/sdg/x2p/x2m/y2p/y2m(st, qubit)` | Clifford 集内 |
| 双比特 | `apply_cx/cy/cz/swap(st, q0, q1)` | Clifford 集内 |
| 整线路 | `apply_circuit(st, circuit)` | 线路必须全为 Clifford 门 |

**仅支持 Clifford 门集**：`H`、`S`、`S†`、`X`、`Y`、`Z`、`X2P/X2M/Y2P/Y2M`（±π/2 旋转）及 `CX`、`CY`、`CZ`、`SWAP`。非 Clifford 门（如 `T`、`T†`、`RX`、任意角旋转）不支持，施加时返回 `-7`。

---

## 概率与测量

### stabilizer_probabilities_len(ptr) / stabilizer_probabilities(ptr, buffer, len)

两步式读取 `2^N` 个基态概率分布，语义与 [Statevector](1_statevector.md#概率与测量) 一致。

### stabilizer_measure(ptr, qubit)

Z 基测量 `qubit` 并更新稳定子表。

返回：

- `int32_t`：测量结果 `0`/\|0⟩ 或 `1`/\|1⟩；失败返回负错误码。

### stabilizer_measure_all(ptr)

测量全部比特并坍缩，返回堆分配大端比特串，需 `cqlib_string_free` 释放；失败返回 NULL。

### stabilizer_sample_shots(ptr, shots)

独立采样 `shots` 次，返回 `COutcomeList *`，用法见[概览](0_overview.md#采样结果-outcomelist)。

### stabilizer_reset(ptr, qubit)

将 `qubit` 重置到 `\|0⟩`。成功 `0`，失败负错误码。

---

## 稳定子读取

### stabilizer_pauli_expectation(ptr, pauli, out)

计算 Pauli 串期望值 `⟨P⟩` 写入 `out`。

参数：

- `pauli` (`const CPauliString *`)：见 [PauliString](4_pauli_string.md)。

返回：

- `int32_t`；成功 `0`，失败负错误码。

取值恒为 `-1`、`0` 或 `+1`（稳定子态对 Pauli 算符的本征值只能是 ±1，Pauli 串与稳定子群不对易时为 `0`）。

### stabilizer_get_stabilizers_len(ptr)

返回稳定子生成元数量（恒等于比特数 `N`）；NULL 句柄返回 `0`。

### stabilizer_get_stabilizer(ptr, index)

返回第 `index` 个生成元的字符串形式（如 `"+XX"`：符号位 + 各比特 Pauli 字母）。

返回：

- `char *`：堆分配字符串，需 `cqlib_string_free` 释放；越界返回 NULL。

---

## 示例

```c
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);        // Bell 态制备（全 Clifford）

CStabilizerState *st = stabilizer_from_circuit(qc);

/* Pauli 期望：Bell 态 ⟨ZZ⟩ = +1，⟨XX⟩ = +1 */
CPauliString *zz = pauli_string_parse("ZZ");
CPauliString *xx = pauli_string_parse("XX");

int32_t vzz, vxx;
stabilizer_pauli_expectation(st, zz, &vzz);   // +1
stabilizer_pauli_expectation(st, xx, &vxx);   // +1
printf("<ZZ>=%d <XX>=%d\n", vzz, vxx);

pauli_string_free(zz);
pauli_string_free(xx);

/* 生成元列表 */
uintptr_t n = stabilizer_get_stabilizers_len(st);   // 2
for (uintptr_t i = 0; i < n; i++) {
    char *gen = stabilizer_get_stabilizer(st, i);
    printf("g%zu = %s\n", (size_t)i, gen);          // 如 "+XX"、"+ZZ"
    cqlib_string_free(gen);
}

stabilizer_free(st);
circuit_free(qc);
```
