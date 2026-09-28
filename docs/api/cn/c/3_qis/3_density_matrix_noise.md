# DensityMatrixNoise（噪声密度矩阵）

`CDensityMatrixNoise*` 是带噪声模型的混合态模拟器句柄：在密度矩阵模拟器之上叠加 `CNoiseModel`，每个门施加后按噪声模型查询同一门、同一比特上的噪声条目并施加相应信道；读出误差只改变报告出来的概率分布。本页覆盖构造与线路对接、门操作、概率与读出噪声、测量与采样以及期望值。错误码与内存约定见 [Overview](../0_overview.md)，模块总览见 [QIS 概览](0_overview.md)。

---

## 构造与释放

### density_matrix_noise_new(num_qubits, noise_model)

```c
struct CDensityMatrixNoise *density_matrix_noise_new(uintptr_t num_qubits,
                                                     const struct CNoiseModel *noise_model);
```

创建噪声模拟器，初始态为 `|0...0><0...0|`；`noise_model` 传 NULL 进行理想（无噪声）仿真。

- `num_qubits` (`uintptr_t`)：量子比特数量。
- `noise_model` (`const CNoiseModel*`)：噪声模型句柄，可为 NULL；构造时被克隆，之后模型句柄可独立释放。

返回：新分配的 `CDensityMatrixNoise*`，用 `density_matrix_noise_free` 释放。

### density_matrix_noise_from_circuit(circuit, noise_model)

```c
struct CDensityMatrixNoise *density_matrix_noise_from_circuit(const struct CCircuit *circuit,
                                                              const struct CNoiseModel *noise_model);
```

执行线路并返回噪声模拟器：线路先被分解为基础门，噪声在每个门之后施加；`noise_model` 传 NULL 进行理想仿真。

- `circuit` (`const CCircuit*`)：线路句柄，见 [Circuit](../0_circuit/1_circuit.md)。
- `noise_model` (`const CNoiseModel*`)：噪声模型句柄，可为 NULL。

返回：新分配的 `CDensityMatrixNoise*`；`circuit` 为 NULL 或执行失败返回 `NULL`。

### density_matrix_noise_free(ptr)

```c
void density_matrix_noise_free(struct CDensityMatrixNoise *ptr);
```

释放句柄。允许传 `NULL`。

- `ptr` (`CDensityMatrixNoise*`)：噪声模拟器句柄。

### density_matrix_noise_num_qubits(ptr)

```c
uintptr_t density_matrix_noise_num_qubits(const struct CDensityMatrixNoise *ptr);
```

返回量子比特数量。

- `ptr` (`const CDensityMatrixNoise*`)：噪声模拟器句柄。

返回：比特数量；`NULL` 时返回 0。

### density_matrix_noise_apply_circuit(ptr, circuit)

```c
int32_t density_matrix_noise_apply_circuit(struct CDensityMatrixNoise *ptr,
                                           const struct CCircuit *circuit);
```

把线路原地作用到当前模拟器上：线路先被分解，每个门施加后注入噪声模型中匹配的门噪声。

- `ptr` (`CDensityMatrixNoise*`)：噪声模拟器句柄。
- `circuit` (`const CCircuit*`)：线路句柄。

返回：`0` 成功；`-1` NULL 指针；`-3` 线路错误；`-7` 线路含不支持的操作；`-8` 线路比特数与模拟器不一致。

---

## 门操作

所有门函数返回 `int32_t`：`0` 成功；`-1` 句柄为 NULL；`-2` 比特编号越界；参数化门的角度取值为 NaN 或 Inf 时返回 `-8`。每个门施加后按噪声模型注入匹配的门噪声；未配置噪声模型或没有匹配条目时只施加理想门。单比特门统一签名为：

```c
int32_t density_matrix_noise_apply_h(struct CDensityMatrixNoise *ptr, uint32_t qubit);
```

### 无参数单比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `density_matrix_noise_apply_h(ptr, qubit)` | H | Hadamard 门。 |
| `density_matrix_noise_apply_x(ptr, qubit)` | X | Pauli-X，比特翻转。 |
| `density_matrix_noise_apply_y(ptr, qubit)` | Y | Pauli-Y。 |
| `density_matrix_noise_apply_z(ptr, qubit)` | Z | Pauli-Z，相位翻转。 |
| `density_matrix_noise_apply_s(ptr, qubit)` | S | S 门（√Z）。 |
| `density_matrix_noise_apply_sdg(ptr, qubit)` | S† | S 的逆。 |
| `density_matrix_noise_apply_t(ptr, qubit)` | T | T 门（√S）。 |
| `density_matrix_noise_apply_tdg(ptr, qubit)` | T† | T 的逆。 |
| `density_matrix_noise_apply_x2p(ptr, qubit)` | X2P | `Rx(π/2)`。 |
| `density_matrix_noise_apply_x2m(ptr, qubit)` | X2M | `Rx(-π/2)`。 |
| `density_matrix_noise_apply_y2p(ptr, qubit)` | Y2P | `Ry(π/2)`。 |
| `density_matrix_noise_apply_y2m(ptr, qubit)` | Y2M | `Ry(-π/2)`。 |

### 参数化单比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `density_matrix_noise_apply_rx(ptr, qubit, theta)` | Rx(θ) | 绕 X 轴旋转 `θ`。 |
| `density_matrix_noise_apply_ry(ptr, qubit, theta)` | Ry(θ) | 绕 Y 轴旋转 `θ`。 |
| `density_matrix_noise_apply_rz(ptr, qubit, theta)` | Rz(θ) | 绕 Z 轴旋转 `θ`。 |
| `density_matrix_noise_apply_phase(ptr, qubit, theta)` | P(θ) | 相位门，给 `\|1>` 分量乘 `e^(iθ)`。 |
| `density_matrix_noise_apply_u(ptr, qubit, theta, phi, lambda)` | U(θ, φ, λ) | 通用单比特门。 |
| `density_matrix_noise_apply_xy(ptr, qubit, theta)` | XY(θ) | XY 门。 |
| `density_matrix_noise_apply_xy2p(ptr, qubit, theta)` | XY2P(θ) | XY2P 门。 |
| `density_matrix_noise_apply_xy2m(ptr, qubit, theta)` | XY2M(θ) | XY2M 门。 |
| `density_matrix_noise_apply_rxy(ptr, qubit, theta, phi)` | RXY(θ, φ) | 绕 XY 平面内方位角为 `φ` 的轴旋转 `θ`。 |
| `density_matrix_noise_apply_gphase(ptr, phi)` | GPhase(φ) | 给整个态乘全局相位 `e^(iφ)`；签名无 qubit 参数。 |

GPhase 签名：

```c
int32_t density_matrix_noise_apply_gphase(struct CDensityMatrixNoise *ptr, double phi);
```

### 双比特门

统一签名 `int32_t density_matrix_noise_apply_<gate>(struct CDensityMatrixNoise *ptr, uint32_t q0, uint32_t q1, ...)`，受控门的两个比特参数名为 `control` / `target`。

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `density_matrix_noise_apply_cx(ptr, control, target)` | CX | 受控 X。 |
| `density_matrix_noise_apply_cy(ptr, control, target)` | CY | 受控 Y。 |
| `density_matrix_noise_apply_cz(ptr, q0, q1)` | CZ | 受控 Z，两参数地位对称。 |
| `density_matrix_noise_apply_swap(ptr, q0, q1)` | SWAP | 交换两个比特。 |
| `density_matrix_noise_apply_rxx(ptr, q0, q1, theta)` | RXX(θ) | `exp(-i·θ/2·X⊗X)`。 |
| `density_matrix_noise_apply_ryy(ptr, q0, q1, theta)` | RYY(θ) | `exp(-i·θ/2·Y⊗Y)`。 |
| `density_matrix_noise_apply_rzz(ptr, q0, q1, theta)` | RZZ(θ) | `exp(-i·θ/2·Z⊗Z)`。 |
| `density_matrix_noise_apply_rzx(ptr, q0, q1, theta)` | RZX(θ) | `exp(-i·θ/2·Z⊗X)`。 |
| `density_matrix_noise_apply_crx(ptr, control, target, theta)` | CrX(θ) | 受控 `Rx(θ)`。 |
| `density_matrix_noise_apply_cry(ptr, control, target, theta)` | CrY(θ) | 受控 `Ry(θ)`。 |
| `density_matrix_noise_apply_crz(ptr, control, target, theta)` | CrZ(θ) | 受控 `Rz(θ)`。 |
| `density_matrix_noise_apply_fsim(ptr, q0, q1, theta, phi)` | FSIM | 费米子模拟门，`theta` 为 iSWAP 角，`phi` 为受控相位角。 |

### 三比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `density_matrix_noise_apply_ccx(ptr, c0, c1, target)` | CCX | Toffoli 门，两个控制位同时为 `\|1>` 时翻转目标位。 |

### 通用门入口

#### density_matrix_noise_apply_standard_gate_noise(ptr, gate_name, qubits, qubit_len, params, param_len)

```c
int32_t density_matrix_noise_apply_standard_gate_noise(struct CDensityMatrixNoise *ptr,
                                                       const char *gate_name,
                                                       const uint32_t *qubits,
                                                       uintptr_t qubit_len,
                                                       const double *params,
                                                       uintptr_t param_len);
```

按名称施加标准门（如 `"H"`、`"CX"`、`"RZZ"`、`"GPhase"`、`"FSIM"`）：先施加理想门，再施加噪声模型中匹配的门噪声。

- `ptr` (`CDensityMatrixNoise*`)：噪声模拟器句柄。
- `gate_name` (`const char*`)：NUL 结尾的标准门名。
- `qubits` (`const uint32_t*`)：目标比特数组，长度为 `qubit_len`；`qubit_len` 为 0 时可为 NULL。
- `qubit_len` (`uintptr_t`)：目标比特数量，必须与门的要求一致。
- `params` (`const double*`)：门参数数组；`param_len` 为 0 时可为 NULL。
- `param_len` (`uintptr_t`)：参数数量，必须与门的要求一致。

返回：`0` 成功；`-1` NULL 指针；`-2` 比特越界；`-4` 门名不是合法 UTF-8；`-8` 未知门名、比特数或参数个数与门不符、参数值非有限。

#### density_matrix_noise_apply_unitary_gate(ptr, qubits, qubit_len, matrix, matrix_dim)

```c
int32_t density_matrix_noise_apply_unitary_gate(struct CDensityMatrixNoise *ptr,
                                                const uint32_t *qubits,
                                                uintptr_t qubit_len,
                                                const Complex64 *matrix,
                                                uintptr_t matrix_dim);
```

按 `ρ → U ρ U†` 作用任意酉矩阵。通用酉门没有对应的标准门类型，施加时不注入门噪声；需要噪声建模时使用具体的门函数。

- `ptr` (`CDensityMatrixNoise*`)：噪声模拟器句柄。
- `qubits` (`const uint32_t*`)：目标比特数组，不得重复。
- `qubit_len` (`uintptr_t`)：目标比特数量。
- `matrix` (`const Complex64*`)：行主序 `matrix_dim × matrix_dim` 矩阵。
- `matrix_dim` (`uintptr_t`)：矩阵维度，必须等于 `2^qubit_len`。

返回：`0` 成功；`-1` NULL 指针；`-2` 比特越界；`-8` `qubit_len` 或 `matrix_dim` 为 0、维度不匹配或比特重复。

---

## 概率与读出噪声

模拟器把「量子态的噪声」与「读出的噪声」分开表达：门噪声作用于密度矩阵本身；读出误差只改变报告出来的概率分布。

### density_matrix_noise_probabilities_len(ptr)

```c
uintptr_t density_matrix_noise_probabilities_len(const struct CDensityMatrixNoise *ptr);
```

返回计算基态概率分布的长度（`2^N`）。

- `ptr` (`const CDensityMatrixNoise*`)：噪声模拟器句柄。

返回：概率数量；`NULL` 时返回 0。

### density_matrix_noise_probabilities(ptr, buffer, len)

```c
int32_t density_matrix_noise_probabilities(const struct CDensityMatrixNoise *ptr,
                                           double *buffer,
                                           uintptr_t len);
```

把理想概率分布（密度矩阵对角元，含已施加的门噪声、不含读出误差）拷贝进 `buffer`。

- `ptr` (`const CDensityMatrixNoise*`)：噪声模拟器句柄。
- `buffer` (`double*`)：输出缓冲。
- `len` (`uintptr_t`)：缓冲长度，必须等于 `density_matrix_noise_probabilities_len`。

返回：`0` 成功；`-1` NULL 指针；`-8` `len` 与实际数量不符。

### density_matrix_noise_probabilities_with_readout_len(ptr)

```c
uintptr_t density_matrix_noise_probabilities_with_readout_len(const struct CDensityMatrixNoise *ptr);
```

返回含读出误差概率分布的缓冲长度（`2^N`）；缓冲大小与传给 `density_matrix_noise_probabilities_with_readout` 的比特选择无关。

- `ptr` (`const CDensityMatrixNoise*`)：噪声模拟器句柄。

返回：概率数量；`NULL` 时返回 0。

### density_matrix_noise_probabilities_with_readout(ptr, buffer, len, qubits, qubit_len)

```c
int32_t density_matrix_noise_probabilities_with_readout(const struct CDensityMatrixNoise *ptr,
                                                        double *buffer,
                                                        uintptr_t len,
                                                        const uint32_t *qubits,
                                                        uintptr_t qubit_len);
```

在理想概率分布的基础上，对 `qubits` 列出的比特施加噪声模型中登记的读出误差，结果拷贝进 `buffer`。

- `ptr` (`const CDensityMatrixNoise*`)：噪声模拟器句柄。
- `buffer` (`double*`)：输出缓冲。
- `len` (`uintptr_t`)：缓冲长度，必须等于 `density_matrix_noise_probabilities_with_readout_len`。
- `qubits` (`const uint32_t*`)：施加读出误差的比特数组；`qubit_len` 为 0 时可为 NULL，此时结果与 `density_matrix_noise_probabilities` 一致。
- `qubit_len` (`uintptr_t`)：比特数量。

返回：`0` 成功；`-1` NULL 指针；`-2` 比特越界；`-8` `len` 与实际数量不符。

---

## 测量、重置与采样

### density_matrix_noise_measure(ptr, qubit)

```c
int32_t density_matrix_noise_measure(struct CDensityMatrixNoise *ptr, uint32_t qubit);
```

在 Z 基下测量指定比特并使底层密度矩阵坍缩；测量操作本身不施加噪声。

- `ptr` (`CDensityMatrixNoise*`)：噪声模拟器句柄。
- `qubit` (`uint32_t`)：被测比特。

返回：`0` 表示结果 `|0>`，`1` 表示结果 `|1>`；`-1` NULL；`-2` 比特越界。

### density_matrix_noise_measure_all(ptr)

```c
char *density_matrix_noise_measure_all(struct CDensityMatrixNoise *ptr);
```

按 `0..num_qubits` 顺序逐比特测量并使态坍缩，返回大端比特串（MSB = 比特 `N-1`，LSB = 比特 `0`）。

- `ptr` (`CDensityMatrixNoise*`)：噪声模拟器句柄。

返回：堆分配的 C 字符串，用 `cqlib_string_free` 释放；失败返回 `NULL`。

### density_matrix_noise_reset(ptr, qubit)

```c
int32_t density_matrix_noise_reset(struct CDensityMatrixNoise *ptr, uint32_t qubit);
```

把指定比特重置为 `|0>`；重置不施加读出误差。

- `ptr` (`CDensityMatrixNoise*`)：噪声模拟器句柄。
- `qubit` (`uint32_t`)：目标比特。

返回：`0` 成功；`-1` NULL；`-2` 比特越界。

### density_matrix_noise_sample_shots(ptr, shots)

```c
struct COutcomeList *density_matrix_noise_sample_shots(const struct CDensityMatrixNoise *ptr,
                                                       uintptr_t shots);
```

按当前概率分布并行采样 `shots` 次独立测量结果，不修改态；采样的是量子态本身的分布，不含读出误差。返回的列表用 `outcome_list_len` 取数量、`outcome_list_get` 读取每个大端比特串（用 `cqlib_string_free` 释放）、`outcome_list_free` 释放列表，详见 [ClassicalState](5_classical_state.md)。

- `ptr` (`const CDensityMatrixNoise*`)：噪声模拟器句柄。
- `shots` (`uintptr_t`)：采样次数。

返回：新分配的 `COutcomeList*`；失败返回 `NULL`。

---

## 期望值

### density_matrix_noise_expectation(ptr, observable, out)

```c
int32_t density_matrix_noise_expectation(const struct CDensityMatrixNoise *ptr,
                                         const struct CHamiltonian *observable,
                                         double *out);
```

计算 `<H> = Tr(H·ρ)` 并写入 `out`；结果含已施加的门噪声，不含读出误差（读出误差不影响量子态）。

- `ptr` (`const CDensityMatrixNoise*`)：噪声模拟器句柄。
- `observable` (`const CHamiltonian*`)：可观测量句柄，见 [Hamiltonian](7_hamiltonian.md)。
- `out` (`double*`)：输出指针。

返回：`0` 成功；`-1` NULL 指针；`-8` 观测量的比特数与模拟器不一致。

---

## 噪声模型的配置

噪声模型句柄 `CNoiseModel` 由设备模块提供，构造与登记接口包括 `noise_model_new`、`noise_model_add_single_qubit`（按 `NOISE_BIT_FLIP` 等标签选择信道）、`noise_model_add_single_qubit_pauli`、`noise_model_add_two_qubit`（`NOISE_TWO_DEPOLARIZING`）与 `noise_model_add_readout`（非对称读出误差 `p_0_given_1` / `p_1_given_0`），详见 [NoiseModel](../2_device/4_noise.md)。

---

## 示例

### 1. 含门噪声与读出噪声的 Bell 态

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 5% depolarizing on CX(0,1), 10% readout error on qubit 0 */
    struct CNoiseModel *model = noise_model_new();
    noise_model_add_two_qubit(model, "CX", 0, 1, NOISE_TWO_DEPOLARIZING, 0.05);
    noise_model_add_readout(model, 0, 0.0, 0.1);

    struct CDensityMatrixNoise *sim = density_matrix_noise_new(2, model);
    density_matrix_noise_apply_h(sim, 0);
    density_matrix_noise_apply_cx(sim, 0, 1);

    /* Ideal distribution: gate noise only, no readout error */
    double ideal[4];
    density_matrix_noise_probabilities(sim, ideal, 4);
    printf("ideal   P(00)=%.3f P(11)=%.3f\n", ideal[0], ideal[3]);

    /* With readout error applied to qubit 0 */
    double with_ro[4];
    uint32_t measured[1] = {0};
    density_matrix_noise_probabilities_with_readout(sim, with_ro, 4, measured, 1);
    printf("readout P(00)=%.3f P(11)=%.3f\n", with_ro[0], with_ro[3]);

    density_matrix_noise_free(sim);
    noise_model_free(model);
    return 0;
}
```

### 2. 由线路构造并求期望值

```c
struct CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

struct CNoiseModel *model = noise_model_new();
noise_model_add_two_qubit(model, "CX", 0, 1, NOISE_TWO_DEPOLARIZING, 0.05);

struct CDensityMatrixNoise *sim = density_matrix_noise_from_circuit(qc, model);

/* <ZZ> stays close to 1 under weak depolarizing noise */
struct CPauliString *zz = pauli_string_parse("ZZ");
struct CHamiltonian *h = hamiltonian_from_pauli(zz);  /* takes ownership of zz */
double exp = 0.0;
density_matrix_noise_expectation(sim, h, &exp);
printf("<ZZ>=%.3f\n", exp);
hamiltonian_free(h);

density_matrix_noise_free(sim);
noise_model_free(model);
circuit_free(qc);
```
