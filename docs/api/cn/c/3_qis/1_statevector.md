# Statevector（态矢量）

`CStatevector*` 是纯态模拟器句柄：量子态表示为 `2^N` 个复振幅，全部操作通过 `statevector_*` 自由函数完成。本页覆盖构造与线路对接、振幅访问、门操作、概率与测量以及期望值。错误码与内存约定见 [Overview](../0_overview.md)，模块总览见 [QIS 概览](0_overview.md)。

---

## 构造与释放

### statevector_new(num_qubits)

```c
struct CStatevector *statevector_new(uintptr_t num_qubits);
```

创建纯态模拟器，初始态为 `|0...0>`：首分量为 1，其余为 0。

- `num_qubits` (`uintptr_t`)：量子比特数量。

返回：新分配的 `CStatevector*`，用 `statevector_free` 释放。

### statevector_from_circuit(circuit)

```c
struct CStatevector *statevector_from_circuit(const struct CCircuit *circuit);
```

执行线路并返回演化后的态，输入线路不被修改。线路先被分解为基础门再逐条执行；测量声明不参与状态演化。

- `circuit` (`const CCircuit*`)：线路句柄，见 [Circuit](../0_circuit/1_circuit.md)。

返回：新分配的 `CStatevector*`；`circuit` 为 NULL 或执行失败返回 `NULL`。

### statevector_from_state(num_qubits, initial_state, len)

```c
struct CStatevector *statevector_from_state(uintptr_t num_qubits,
                                            const Complex64 *initial_state,
                                            uintptr_t len);
```

用给定振幅数组构造态，要求 `len` 等于 `2^num_qubits` 且已归一化；下标 `i` 的分量对应计算基态 `|i>`，比特 0 对应最低位。

- `num_qubits` (`uintptr_t`)：量子比特数量。
- `initial_state` (`const Complex64*`)：长度为 `2^num_qubits` 的振幅数组。
- `len` (`uintptr_t`)：数组长度。

返回：新分配的 `CStatevector*`；输入为 NULL、长度不符或未归一化时返回 `NULL`。

### statevector_free(ptr)

```c
void statevector_free(struct CStatevector *ptr);
```

释放句柄。允许传 `NULL`。

- `ptr` (`CStatevector*`)：态矢量句柄。

### statevector_num_qubits(ptr)

```c
uintptr_t statevector_num_qubits(const struct CStatevector *ptr);
```

返回量子比特数量。

- `ptr` (`const CStatevector*`)：态矢量句柄。

返回：比特数量；`NULL` 时返回 0。

### statevector_apply_circuit(ptr, circuit)

```c
int32_t statevector_apply_circuit(struct CStatevector *ptr, const struct CCircuit *circuit);
```

把线路原地作用到当前态上，线路的比特数必须与态一致。

- `ptr` (`CStatevector*`)：态矢量句柄。
- `circuit` (`const CCircuit*`)：线路句柄。

返回：`0` 成功；`-1` NULL 指针；`-3` 线路错误（门没有矩阵表示、符号参数未解析等）；`-7` 线路含不支持的操作；`-8` 线路比特数与态不一致。

---

## 振幅访问

### statevector_data_len(ptr)

```c
uintptr_t statevector_data_len(const struct CStatevector *ptr);
```

返回复振幅数量（`2^N`），用于为 `statevector_data` 分配缓冲。

- `ptr` (`const CStatevector*`)：态矢量句柄。

返回：振幅数量；`NULL` 时返回 0。

### statevector_data(ptr, buffer, len)

```c
int32_t statevector_data(const struct CStatevector *ptr, Complex64 *buffer, uintptr_t len);
```

把全部振幅按行主序拷贝进 `buffer`，共 `2^N` 个 `Complex64` 值。

- `ptr` (`const CStatevector*`)：态矢量句柄。
- `buffer` (`Complex64*`)：输出缓冲。
- `len` (`uintptr_t`)：缓冲长度，必须等于 `statevector_data_len`。

返回：`0` 成功；`-1` NULL 指针；`-8` `len` 与实际数量不符。

```c
uintptr_t n = statevector_data_len(sv);
Complex64 *amps = malloc(n * sizeof(Complex64));
if (statevector_data(sv, amps, n) == 0) {
    /* amps[i] is the amplitude of |i> */
}
free(amps);
```

### statevector_data_mut(ptr, buffer, len)

```c
int32_t statevector_data_mut(struct CStatevector *ptr, const Complex64 *buffer, uintptr_t len);
```

用 `buffer` 中的 `len` 个 `Complex64` 值覆盖态矢量的全部振幅，是 `statevector_data` 的可写对应版本。

- `ptr` (`CStatevector*`)：态矢量句柄。
- `buffer` (`const Complex64*`)：`2^N` 个振幅的输入数组。
- `len` (`uintptr_t`)：数组长度，必须等于 `statevector_data_len`。

返回：`0` 成功；`-1` NULL 指针；`-8` `len` 与实际数量不符。

直接写入振幅可能破坏归一化；调用方有责任保证写入的是合法态矢量——模拟器会按写入的数据原样继续演化。

---

## 门操作

所有门函数返回 `int32_t`：`0` 成功；`-1` 句柄为 NULL；`-2` 比特编号越界；参数化门的角度取值为 NaN 或 Inf 时返回 `-8`。单比特门统一签名为：

```c
int32_t statevector_apply_h(struct CStatevector *ptr, uint32_t qubit);
```

### 无参数单比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `statevector_apply_h(ptr, qubit)` | H | Hadamard 门。 |
| `statevector_apply_x(ptr, qubit)` | X | Pauli-X，比特翻转。 |
| `statevector_apply_y(ptr, qubit)` | Y | Pauli-Y。 |
| `statevector_apply_z(ptr, qubit)` | Z | Pauli-Z，相位翻转。 |
| `statevector_apply_s(ptr, qubit)` | S | S 门（√Z）。 |
| `statevector_apply_sdg(ptr, qubit)` | S† | S 的逆。 |
| `statevector_apply_t(ptr, qubit)` | T | T 门（√S）。 |
| `statevector_apply_tdg(ptr, qubit)` | T† | T 的逆。 |
| `statevector_apply_x2p(ptr, qubit)` | X2P | `Rx(π/2)`。 |
| `statevector_apply_x2m(ptr, qubit)` | X2M | `Rx(-π/2)`。 |
| `statevector_apply_y2p(ptr, qubit)` | Y2P | `Ry(π/2)`。 |
| `statevector_apply_y2m(ptr, qubit)` | Y2M | `Ry(-π/2)`。 |

### 参数化单比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `statevector_apply_rx(ptr, qubit, theta)` | Rx(θ) | 绕 X 轴旋转 `θ`。 |
| `statevector_apply_ry(ptr, qubit, theta)` | Ry(θ) | 绕 Y 轴旋转 `θ`。 |
| `statevector_apply_rz(ptr, qubit, theta)` | Rz(θ) | 绕 Z 轴旋转 `θ`。 |
| `statevector_apply_phase(ptr, qubit, theta)` | P(θ) | 相位门，给 `\|1>` 分量乘 `e^(iθ)`。 |
| `statevector_apply_u(ptr, qubit, theta, phi, lambda)` | U(θ, φ, λ) | 通用单比特门。 |
| `statevector_apply_xy(ptr, qubit, theta)` | XY(θ) | XY 门。 |
| `statevector_apply_xy2p(ptr, qubit, theta)` | XY2P(θ) | XY2P 门。 |
| `statevector_apply_xy2m(ptr, qubit, theta)` | XY2M(θ) | XY2M 门。 |
| `statevector_apply_rxy(ptr, qubit, theta, phi)` | RXY(θ, φ) | 绕 XY 平面内方位角为 `φ` 的轴旋转 `θ`。 |
| `statevector_apply_gphase(ptr, phi)` | GPhase(φ) | 给整个态乘全局相位 `e^(iφ)`；签名无 qubit 参数。 |

GPhase 签名：

```c
int32_t statevector_apply_gphase(struct CStatevector *ptr, double phi);
```

### 双比特门

统一签名 `int32_t statevector_apply_<gate>(struct CStatevector *ptr, uint32_t q0, uint32_t q1, ...)`，受控门的两个比特参数名为 `control` / `target`。

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `statevector_apply_cx(ptr, control, target)` | CX | 受控 X。 |
| `statevector_apply_cy(ptr, control, target)` | CY | 受控 Y。 |
| `statevector_apply_cz(ptr, q0, q1)` | CZ | 受控 Z，两参数地位对称。 |
| `statevector_apply_swap(ptr, q0, q1)` | SWAP | 交换两个比特。 |
| `statevector_apply_rxx(ptr, q0, q1, theta)` | RXX(θ) | `exp(-i·θ/2·X⊗X)`。 |
| `statevector_apply_ryy(ptr, q0, q1, theta)` | RYY(θ) | `exp(-i·θ/2·Y⊗Y)`。 |
| `statevector_apply_rzz(ptr, q0, q1, theta)` | RZZ(θ) | `exp(-i·θ/2·Z⊗Z)`。 |
| `statevector_apply_rzx(ptr, q0, q1, theta)` | RZX(θ) | `exp(-i·θ/2·Z⊗X)`。 |
| `statevector_apply_crx(ptr, control, target, theta)` | CrX(θ) | 受控 `Rx(θ)`。 |
| `statevector_apply_cry(ptr, control, target, theta)` | CrY(θ) | 受控 `Ry(θ)`。 |
| `statevector_apply_crz(ptr, control, target, theta)` | CrZ(θ) | 受控 `Rz(θ)`。 |
| `statevector_apply_fsim(ptr, q0, q1, theta, phi)` | FSIM | 费米子模拟门，`theta` 为 iSWAP 角，`phi` 为受控相位角。 |

### 三比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `statevector_apply_ccx(ptr, c0, c1, target)` | CCX | Toffoli 门，两个控制位同时为 `\|1>` 时翻转目标位。 |

### 通用门入口

#### statevector_apply_standard_gate(ptr, gate_name, qubits, num_target_qubits, params, num_params)

```c
int32_t statevector_apply_standard_gate(struct CStatevector *ptr,
                                        const char *gate_name,
                                        const uint32_t *qubits,
                                        uintptr_t num_target_qubits,
                                        const double *params,
                                        uintptr_t num_params);
```

按名称施加标准门（如 `"H"`、`"CX"`、`"RZZ"`、`"FSIM"`），分派到对应的专用实现。

- `ptr` (`CStatevector*`)：态矢量句柄。
- `gate_name` (`const char*`)：NUL 结尾的标准门名。
- `qubits` (`const uint32_t*`)：目标比特数组，长度为 `num_target_qubits`。
- `num_target_qubits` (`uintptr_t`)：目标比特数量，必须与门的要求一致。
- `params` (`const double*`)：门参数数组；`num_params` 为 0 时可为 NULL。
- `num_params` (`uintptr_t`)：参数数量，必须与门的要求一致。

返回：`0` 成功；`-1` NULL 指针；`-2` 比特越界；`-8` 未知门名、门名不是合法 UTF-8、比特数或参数个数与门不符、参数值非有限。

#### statevector_apply_unitary_gate(ptr, qubits, num_target_qubits, matrix, dim)

```c
int32_t statevector_apply_unitary_gate(struct CStatevector *ptr,
                                       const uint32_t *qubits,
                                       uintptr_t num_target_qubits,
                                       const Complex64 *matrix,
                                       uintptr_t dim);
```

把任意酉矩阵作用到指定比特上。

- `ptr` (`CStatevector*`)：态矢量句柄。
- `qubits` (`const uint32_t*`)：目标比特数组，不得重复。
- `num_target_qubits` (`uintptr_t`)：目标比特数量。
- `matrix` (`const Complex64*`)：行主序 `dim × dim` 矩阵。
- `dim` (`uintptr_t`)：矩阵维度，必须等于 `2^num_target_qubits`。

返回：`0` 成功；`-1` NULL 指针；`-2` 比特越界；`-8` `num_target_qubits` 或 `dim` 为 0、维度不匹配或比特重复。

#### statevector_apply_pauli_rotation(ptr, pauli, theta)

```c
int32_t statevector_apply_pauli_rotation(struct CStatevector *ptr,
                                         const struct CPauliString *pauli,
                                         double theta);
```

原地作用 Pauli 串旋转 `exp(-i·θ/2·P)`，不做门分解。Pauli 串必须覆盖整个寄存器且为 Hermitian，构造方式见 [Pauli](6_pauli.md)。

- `ptr` (`CStatevector*`)：态矢量句柄。
- `pauli` (`const CPauliString*`)：Pauli 串句柄。
- `theta` (`double`)：旋转角。

返回：`0` 成功；`-1` NULL 指针；`-8` `theta` 非有限或 Pauli 串比特数与态不一致。

---

## 概率与测量

### statevector_probabilities_len(ptr)

```c
uintptr_t statevector_probabilities_len(const struct CStatevector *ptr);
```

返回计算基态概率分布的长度（`2^N`）。

- `ptr` (`const CStatevector*`)：态矢量句柄。

返回：概率数量；`NULL` 时返回 0。

### statevector_probabilities(ptr, buffer, len)

```c
int32_t statevector_probabilities(const struct CStatevector *ptr, double *buffer, uintptr_t len);
```

把全部计算基态上的测量概率（各振幅模的平方）拷贝进 `buffer`。

- `ptr` (`const CStatevector*`)：态矢量句柄。
- `buffer` (`double*`)：输出缓冲。
- `len` (`uintptr_t`)：缓冲长度，必须等于 `statevector_probabilities_len`。

返回：`0` 成功；`-1` NULL 指针；`-8` `len` 与实际数量不符。

### statevector_measure(ptr, qubit)

```c
int32_t statevector_measure(struct CStatevector *ptr, uint32_t qubit);
```

在 Z 基下测量指定比特并使态坍缩，坍缩后态在对应子空间内重新归一化。该操作是破坏性的。

- `ptr` (`CStatevector*`)：态矢量句柄。
- `qubit` (`uint32_t`)：被测比特。

返回：`0` 表示结果 `|0>`，`1` 表示结果 `|1>`；`-1` NULL；`-2` 比特越界。

### statevector_measure_all(ptr)

```c
char *statevector_measure_all(struct CStatevector *ptr);
```

按 `0..num_qubits` 顺序逐比特测量并使态坍缩，返回大端比特串（MSB = 比特 `N-1`，LSB = 比特 `0`）。

- `ptr` (`CStatevector*`)：态矢量句柄。

返回：堆分配的 C 字符串，用 `cqlib_string_free` 释放；失败返回 `NULL`。

### statevector_sample_shots(ptr, shots)

```c
struct COutcomeList *statevector_sample_shots(const struct CStatevector *ptr, uintptr_t shots);
```

按当前概率分布并行采样 `shots` 次独立测量结果，不修改态。返回的列表用 `outcome_list_len` 取数量、`outcome_list_get` 读取每个大端比特串（用 `cqlib_string_free` 释放）、`outcome_list_free` 释放列表，详见 [ClassicalState](5_classical_state.md)。

- `ptr` (`const CStatevector*`)：态矢量句柄。
- `shots` (`uintptr_t`)：采样次数。

返回：新分配的 `COutcomeList*`；失败返回 `NULL`。

### statevector_reset(ptr, qubit)

```c
int32_t statevector_reset(struct CStatevector *ptr, uint32_t qubit);
```

把指定比特重置为 `|0>`：先做一次 Z 基测量，结果为 `|1>` 时补一个 X 修正。

- `ptr` (`CStatevector*`)：态矢量句柄。
- `qubit` (`uint32_t`)：目标比特。

返回：`0` 成功；`-1` NULL；`-2` 比特越界。

---

## 期望值

### statevector_expectation(ptr, observable, out)

```c
int32_t statevector_expectation(const struct CStatevector *ptr,
                                const struct CHamiltonian *observable,
                                double *out);
```

计算 `⟨ψ|H|ψ⟩` 并写入 `out`。

- `ptr` (`const CStatevector*`)：态矢量句柄。
- `observable` (`const CHamiltonian*`)：可观测量句柄，见 [Hamiltonian](7_hamiltonian.md)。
- `out` (`double*`)：输出指针。

返回：`0` 成功；`-1` NULL 指针；`-8` 观测量的比特数与态不一致。

```c
/* <ZZ> of the Bell state equals 1 */
struct CPauliString *zz = pauli_string_parse("ZZ");
struct CHamiltonian *h = hamiltonian_from_pauli(zz);  /* takes ownership of zz */
double exp = 0.0;
statevector_expectation(sv, h, &exp);   /* exp ≈ 1.0 */
hamiltonian_free(h);
```

---

## 示例

完整流程：制备 Bell 态、读概率、测量、采样、求期望值。

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    /* Bell state (|00> + |11>)/sqrt(2) */
    struct CStatevector *sv = statevector_new(2);
    if (sv == NULL) return 1;
    statevector_apply_h(sv, 0);
    statevector_apply_cx(sv, 0, 1);

    /* Probabilities: P(00) = P(11) = 0.5 */
    uintptr_t n = statevector_probabilities_len(sv);   /* 4 */
    double *probs = malloc(n * sizeof(double));
    statevector_probabilities(sv, probs, n);
    printf("P(00)=%.3f P(11)=%.3f\n", probs[0], probs[3]);
    free(probs);

    /* Sample 100 shots: only "00" and "11" appear */
    struct COutcomeList *shots = statevector_sample_shots(sv, 100);
    for (uintptr_t i = 0; i < outcome_list_len(shots); i++) {
        char *bits = outcome_list_get(shots, i);
        printf("%s ", bits);
        cqlib_string_free(bits);
    }
    printf("\n");
    outcome_list_free(shots);

    /* Expectation <ZZ> = 1 */
    struct CPauliString *zz = pauli_string_parse("ZZ");
    struct CHamiltonian *h = hamiltonian_from_pauli(zz);
    double exp = 0.0;
    statevector_expectation(sv, h, &exp);
    printf("<ZZ>=%.3f\n", exp);
    hamiltonian_free(h);

    /* Destructive full measurement collapses the state */
    char *bits = statevector_measure_all(sv);
    printf("measured: %s\n", bits);   /* "00" or "11" */
    cqlib_string_free(bits);

    statevector_free(sv);
    return 0;
}
```
