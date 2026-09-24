# 稳定器（C）

`CStabilizerState` 是稳定子态模拟器的不透明句柄：用一组相互对易的 Pauli 算子（稳定子生成元）描述量子态，只能表示 Clifford 线路可达的稳定子态，可描述的比特规模远大于稠密表示；遇到任意角旋转、`T` 门等非 Clifford 操作时返回错误。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 构造与释放

### stabilizer_new(num_qubits)

构造 `|0...0>` 稳定子态，初始生成元为每个比特的 destabilizer `X_i` 与 stabilizer `Z_i`，相位均为 `+1`。

参数：

- `num_qubits` (`uintptr_t`)：量子比特数。

返回：成功返回新建的 `CStabilizerState*`；`num_qubits` 为 0 时返回 NULL。

### stabilizer_free(ptr)

释放稳定子态句柄，允许传 NULL。

### stabilizer_from_circuit(circuit)

执行 Clifford 线路并返回最终稳定子态。线路末尾的测量声明被当作输出声明忽略，不使态坍缩；屏障与延迟无副作用。

参数：

- `circuit` (`const CCircuit*`)：Clifford 线路，可含 `I`、`H`、`X`、`Y`、`Z`、`S`、`SDG`、`X2P`、`X2M`、`Y2P`、`Y2M`、`CX`、`CY`、`CZ`、`SWAP` 与 `Reset`。

返回：成功返回新建的 `CStabilizerState*`；NULL 输入、线路含非 Clifford 门或执行失败时返回 NULL。

### stabilizer_run_circuit(circuit)

从 `|0...0>` 出发执行 Clifford 线路并返回最终态。与 `stabilizer_from_circuit` 不同，该入口会执行 `MeasureBit` / `MeasureBits` 操作，返回的态已经坍缩。

参数：

- `circuit` (`const CCircuit*`)：Clifford 线路。

返回：成功返回已坍缩的 `CStabilizerState*`；NULL 输入或执行失败时返回 NULL。

---

## 线路作用

### stabilizer_apply_circuit(ptr, circuit)

把 Clifford 线路原地作用到当前态上。

参数：

- `ptr` (`CStabilizerState*`)：稳定子态句柄。
- `circuit` (`const CCircuit*`)：Clifford 线路，比特数必须与态一致。

返回：0 成功；-1 空指针；-3 线路含无法执行的门或未解析的符号参数；-7 含控制流等不支持的操作；-8 线路比特数与态不一致。

---

## 单比特 Clifford 门

以下入口把对应门原地作用到 `qubit` 上，门名列给出每个入口的语义：

| 门名 | 函数 | 说明 |
| --- | --- | --- |
| `H` | `stabilizer_apply_h(ptr, qubit)` | Hadamard 门。 |
| `X` | `stabilizer_apply_x(ptr, qubit)` | Pauli-X。 |
| `Y` | `stabilizer_apply_y(ptr, qubit)` | Pauli-Y。 |
| `Z` | `stabilizer_apply_z(ptr, qubit)` | Pauli-Z。 |
| `S` | `stabilizer_apply_s(ptr, qubit)` | S 门。 |
| `SDG` | `stabilizer_apply_sdg(ptr, qubit)` | S† 门。 |
| `X2P` | `stabilizer_apply_x2p(ptr, qubit)` | √X，即 Rx(π/2)。 |
| `X2M` | `stabilizer_apply_x2m(ptr, qubit)` | √X†，即 Rx(-π/2)。 |
| `Y2P` | `stabilizer_apply_y2p(ptr, qubit)` | √Y，即 Ry(π/2)。 |
| `Y2M` | `stabilizer_apply_y2m(ptr, qubit)` | √Y†，即 Ry(-π/2)。 |

参数：

- `ptr` (`CStabilizerState*`)：稳定子态句柄。
- `qubit` (`uint32_t`)：目标比特下标。

返回：0 成功；-1 空指针；-2 比特下标越界；-3 门无法执行。

---

## 双比特 Clifford 门

| 门名 | 函数 | 说明 |
| --- | --- | --- |
| `CX` | `stabilizer_apply_cx(ptr, control, target)` | 受控 X 门。 |
| `CY` | `stabilizer_apply_cy(ptr, control, target)` | 受控 Y 门。 |
| `CZ` | `stabilizer_apply_cz(ptr, q0, q1)` | 受控 Z 门。 |
| `SWAP` | `stabilizer_apply_swap(ptr, q0, q1)` | 交换两个比特。 |

参数：

- `ptr` (`CStabilizerState*`)：稳定子态句柄。
- `control` / `target` / `q0` / `q1` (`uint32_t`)：目标比特下标。

返回：0 成功；-1 空指针；-2 比特下标越界；-8 两个比特相同。

### stabilizer_apply_standard_gate(ptr, gate_name, qubits, num_target_qubits, params, num_params)

按名称作用 Clifford 标准门（如 `"H"`、`"CX"`、`"SWAP"`）。

参数：

- `ptr` (`CStabilizerState*`)：稳定子态句柄。
- `gate_name` (`const char*`)：标准门名称。
- `qubits` (`const uint32_t*`)：目标比特下标数组，共 `num_target_qubits` 个。
- `num_target_qubits` (`uintptr_t`)：目标比特数。
- `params` (`const double*`)：门参数数组，共 `num_params` 个；`num_params` 为 0 时允许传 NULL。
- `num_params` (`uintptr_t`)：门参数个数。

返回：0 成功；-1 空指针；-2 比特越界；-8 未知门名或参数个数不符；-7 非 Clifford 门。

---

## 测量、重置与采样

### stabilizer_measure(ptr, qubit)

在 Z 基下测量 `qubit` 并使态坍缩。

返回：测量结果 `0` 或 `1`；-1 空指针；-2 比特下标越界。

### stabilizer_measure_all(ptr)

按顺序测量所有比特并使态坍缩。

返回：大端位串（最左字符是比特 N-1，最右字符是比特 0），用 `cqlib_string_free` 释放；NULL 输入返回 NULL。

### stabilizer_reset(ptr, qubit)

把 `qubit` 重置为 `|0>`（一次 Z 基测量，结果为 `|1>` 时补一个 X 修正）。

返回：0 成功；-1 空指针；-2 比特下标越界。

### stabilizer_sample_shots(ptr, shots)

并行采样 `shots` 次独立测量结果；同一初始态总是产生相同的样本集合。

参数：

- `ptr` (`const CStabilizerState*`)：稳定子态句柄。
- `shots` (`uintptr_t`)：采样次数。

返回：`COutcomeList*`，用 `outcome_list_free` 释放；NULL 输入返回 NULL。位串遍历见 [经典态](5_classical_state.md)。

---

## 概率分布

### stabilizer_probabilities_len(ptr) / stabilizer_probabilities(ptr, buffer, len)

两步式读取全部计算基态上的概率分布：`*_len` 返回长度 `2^N`（NULL 返回 0），填充接口把分布拷入 `buffer`，下标 `i` 对应二进制表示为 `i` 的基态（比特 0 为最低位）。该接口只适用于小规模系统。

返回（填充接口）：0 成功；-1 空指针；-8 `len` 与 `2^N` 不符或系统规模超限。

### stabilizer_probability_of(ptr, bits, len, out)

返回指定计算基态的测量概率：与稳定子态相容的位串返回 `1/2^k`（`k` 为测量结果随机的比特数），不相容时返回 0。该调用不修改态。

参数：

- `ptr` (`const CStabilizerState*`)：稳定子态句柄。
- `bits` (`const bool*`)：`len` 个布尔值，`bits[0]` 是比特 0。
- `len` (`uintptr_t`)：必须等于比特数。
- `out` (`double*`)：概率输出。

返回：0 成功；-1 空指针；-8 长度不符。

---

## 生成元与 Pauli 期望值

### stabilizer_num_qubits(ptr)

返回比特数；NULL 返回 0。

### stabilizer_get_stabilizers_len(ptr) / stabilizer_get_stabilizer(ptr, index)

读取稳定子生成元：`*_len` 返回生成元个数（恒为 N），`get_stabilizer` 返回第 `index` 个生成元的 `"+XYZ"` 形式 Pauli 标签（比特序约定见 [Pauli](6_pauli.md)），用 `cqlib_string_free` 释放；越界返回 NULL。

### stabilizer_get_destabilizers_len(ptr) / stabilizer_get_destabilizer(ptr, index)

读取 destabilizer 生成元：`*_len` 返回生成元个数（恒为 N），`get_destabilizer` 返回第 `index` 个生成元的 Pauli 标签，格式与 `stabilizer_get_stabilizer` 相同，用 `cqlib_string_free` 释放；越界返回 NULL。

### stabilizer_pauli_expectation(ptr, pauli, out)

计算 Pauli 期望值 ⟨P⟩：`1` 表示 `P` 属于稳定子群、`-1` 表示属于取负的稳定子群、`0` 表示两者皆非。该判定覆盖生成元之积，不只针对单个生成元。

参数：

- `ptr` (`const CStabilizerState*`)：稳定子态句柄。
- `pauli` (`const CPauliString*`)：Pauli 串，比特数必须与态一致。
- `out` (`int32_t*`)：期望值输出（-1、0 或 +1）。

返回：0 成功；-1 空指针；-8 Pauli 串比特数与态不一致。

### stabilizer_to_stim_format(ptr)

按 Stim 兼容文本格式导出生成元表（每行一个 `"+XYZ"` Pauli 标签）。

返回：C 字符串，用 `cqlib_string_free` 释放；NULL 输入返回 NULL。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Bell 态：H(0) + CX(0, 1) */
    struct CStabilizerState *s = stabilizer_new(2);
    if (s == NULL) {
        return 1;
    }
    stabilizer_apply_h(s, 0);
    stabilizer_apply_cx(s, 0, 1);

    /* 2. 读取生成元标签（"+XY" 形式）与 Pauli 期望值 */
    char *stab = stabilizer_get_stabilizer(s, 0);
    struct CPauliString *zz = pauli_string_parse("ZZ");
    int32_t e_zz = 0;
    stabilizer_pauli_expectation(s, zz, &e_zz);  /* 1：ZZ 是 Bell 态稳定子 */
    cqlib_string_free(stab);
    pauli_string_free(zz);

    /* 3. 指定基态概率：|00⟩ 与 |11⟩ 各占一半 */
    bool bits[2] = {false, false};
    double p00 = 0.0;
    stabilizer_probability_of(s, bits, 2, &p00);  /* 0.5 */

    /* 4. 采样 500 次并遍历 OutcomeList */
    struct COutcomeList *shots = stabilizer_sample_shots(s, 500);
    uintptr_t n = outcome_list_len(shots);  /* 500 */
    for (uintptr_t i = 0; i < n; i++) {
        char *bs = outcome_list_get(shots, i);  /* "00" 或 "11" */
        cqlib_string_free(bs);
    }
    outcome_list_free(shots);

    stabilizer_free(s);
    return 0;
}
```
