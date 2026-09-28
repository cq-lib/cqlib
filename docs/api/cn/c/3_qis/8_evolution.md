# Pauli 演化（C）

本页介绍把单条 Pauli 串 `P` 指数化为演化算符 `e^(-i·θ/2·P)` 并追加到线路的接口。整条哈密顿量的时间演化线路（Trotter 分解，以及精确分解与 Trotter 近似间的自动选择）由 [Hamiltonian](7_hamiltonian.md) 页的 `hamiltonian_to_trotter_circuit` 与 `hamiltonian_to_evolution_circuit` 提供。错误码、句柄与字符串的释放约定遵循 [Overview](../0_overview.md)。

---

## circuit_pauli_evolution(circuit, pauli, angle, qubits, qubits_len)

把演化算符 `e^(-i·θ/2·P)` 追加到线路末尾，`θ` 即 `angle`。

```c
int32_t circuit_pauli_evolution(struct CCircuit *circuit,
                                const struct CPauliString *pauli,
                                double angle,
                                const uint32_t *qubits,
                                uintptr_t qubits_len);
```

生成的门序列：

1. **相位校验**：Pauli 串的相位必须为 ±1（Hermitian），相位为 ±i 时无法作为酉演化的生成元，返回 `-3`。
2. **相位吸收**：把 ±1 相位吸收进旋转角，得到有效角 `θ_eff = θ · phase`。
3. **基变换**：对每个非恒等比特施加把分量转到 Z 基的门——`X` 用 `H`，`Y` 先用 `S†` 再用 `H`，`Z` 不需要变换。
4. **CNOT 链**：按非恒等比特下标递增的顺序施加 `CX`，逐级累积奇偶性。
5. **核心旋转**：在最后一个非恒等比特上施加 `RZ(θ_eff)`。
6. **逆序 CNOT 链与逆基变换**，把比特恢复到原来的基。

当 Pauli 串全为恒等算子时，演化退化为全局相位 `e^(-i·θ/2)`，不产生任何门，相位记在线路的全局相位上。

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `circuit` | `CCircuit*` | 目标线路句柄，演化门追加到其末尾。 |
| `pauli` | `const CPauliString*` | 要指数化的 Pauli 串 `P`，构造方式见 [Pauli](6_pauli.md)。 |
| `angle` | `double` | 旋转角 `θ`（固定浮点值）。 |
| `qubits` | `const uint32_t*` | 作用位置数组，按 Pauli 串比特顺序排列。 |
| `qubits_len` | `uintptr_t` | 作用位置数量，必须等于 `pauli_string_num_qubits(pauli)`。 |

返回值（错误码）：

| 返回值 | 场景 |
| --- | --- |
| `0` | 成功。 |
| `-1` | `circuit` 或 `pauli` 为 NULL；或 `qubits_len > 0` 而 `qubits` 为 NULL。 |
| `-2` | `qubits` 中存在越界的比特编号。 |
| `-3` | 位置数量与 Pauli 串比特数不一致，或 Pauli 串相位为 ±i。 |
| `-8` | `angle` 为非有限值（NaN 或无穷）。 |

---

## 示例

```c
#include <assert.h>
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* Append e^(-i * pi/4 * XZ): basis changes, a CNOT ladder,
       one RZ, and the inverse ladder. */
    CCircuit* circuit = circuit_new(2);
    CPauliString* pauli = pauli_string_parse("XZ");
    assert(pauli != NULL);

    const uint32_t qubits[2] = {0, 1};
    assert(circuit_pauli_evolution(circuit, pauli, 1.5707963267948966, qubits, 2) == 0);
    assert(circuit_num_operations(circuit) > 0);

    /* NULL arguments are rejected. */
    assert(circuit_pauli_evolution(NULL, pauli, 1.0, qubits, 2) == -1);

    /* Out-of-bounds qubit index. */
    const uint32_t bad[2] = {0, 5};
    assert(circuit_pauli_evolution(circuit, pauli, 1.0, bad, 2) == -2);

    /* Position-count mismatch: a 3-qubit Pauli string on 2 positions. */
    CPauliString* big = pauli_string_parse("XXX");
    assert(circuit_pauli_evolution(circuit, big, 1.0, qubits, 2) == -3);

    pauli_string_free(big);
    pauli_string_free(pauli);
    circuit_free(circuit);
    return 0;
}
```

---

## 相关页面

- [QIS 概览](0_overview.md)：模块总览与共享约定。
- [Pauli](6_pauli.md)：Pauli 串的字符串解析与相位取值。
- [Hamiltonian](7_hamiltonian.md)：整条哈密顿量的 Trotter 演化与精确演化入口。
