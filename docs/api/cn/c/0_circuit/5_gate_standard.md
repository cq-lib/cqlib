# 标准门

本页汇总 C API 中向线路追加标准量子门的全部函数；错误码与内存管理的全局约定见 [Overview](../0_overview.md)。

---

## 约定

- 本页所有函数返回 `int32_t` 错误码：`0` 成功，`-1` 传入 NULL 指针，`-2` 比特索引越界，`-3` 线路错误（数值角度为 NaN 或无穷时同样返回 `-3`）。
- 数值变体以 `double` 传入角度（弧度）；对应的 `_param` 变体以 `const CParameter*` 符号表达式传入角度，参数句柄为 NULL 时返回 `-1`。`CParameter` 句柄由 `param_parse` 创建、`param_free` 释放，详见[参数](3_parameter.md)。
- 每个门的作用比特数量固定：单比特门 1 个，双比特门 2 个，三比特门 3 个；`qubit`/`control`/`target`/`a`/`b` 均为线路内的比特索引。

---

## 无参数单比特门

以下 13 个函数签名均为 `(struct CCircuit *ptr, uint32_t qubit)`，直接追加固定矩阵的单比特门。

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `circuit_h(ptr, qubit)` | H | Hadamard 门，在计算基与叠加基之间变换。 |
| `circuit_x(ptr, qubit)` | X | Pauli-X 门，比特翻转。 |
| `circuit_y(ptr, qubit)` | Y | Pauli-Y 门，比特翻转并附加相位。 |
| `circuit_z(ptr, qubit)` | Z | Pauli-Z 门，相位翻转。 |
| `circuit_i(ptr, qubit)` | I | 恒等门，不改变量子态。 |
| `circuit_s(ptr, qubit)` | S | Clifford 相位门，diag(1, i)。 |
| `circuit_sdg(ptr, qubit)` | SDG | S 的逆门，diag(1, −i)。 |
| `circuit_t(ptr, qubit)` | T | π/8 相位门，diag(1, e^(iπ/4))。 |
| `circuit_tdg(ptr, qubit)` | TDG | T 的逆门，diag(1, e^(−iπ/4))。 |
| `circuit_x2p(ptr, qubit)` | X2P | √X 门，绕 X 轴 +90° 旋转。 |
| `circuit_x2m(ptr, qubit)` | X2M | √X† 门，绕 X 轴 −90° 旋转。 |
| `circuit_y2p(ptr, qubit)` | Y2P | √Y 门，绕 Y 轴 +90° 旋转。 |
| `circuit_y2m(ptr, qubit)` | Y2M | √Y† 门，绕 Y 轴 −90° 旋转。 |

---

## 参数化单比特门

数值变体以 `double` 传角；`_param` 变体以 `const CParameter*` 传角，二者追加同一标准门。

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `circuit_rx(ptr, qubit, theta)` | RX | 绕 X 轴旋转 θ，exp(−iθX/2)。 |
| `circuit_rx_param(ptr, qubit, param_ptr)` | RX | 同上，角度为符号表达式。 |
| `circuit_ry(ptr, qubit, theta)` | RY | 绕 Y 轴旋转 θ，exp(−iθY/2)。 |
| `circuit_ry_param(ptr, qubit, param_ptr)` | RY | 同上，角度为符号表达式。 |
| `circuit_rz(ptr, qubit, theta)` | RZ | 绕 Z 轴旋转 θ，exp(−iθZ/2)。 |
| `circuit_rz_param(ptr, qubit, param_ptr)` | RZ | 同上，角度为符号表达式。 |
| `circuit_phase(ptr, qubit, lambda)` | Phase | 相位门 diag(1, e^(iλ))，对 `1` 态分量施加相位。 |
| `circuit_phase_param(ptr, qubit, param)` | Phase | 同上，相位为符号表达式。 |
| `circuit_u(ptr, qubit, theta, phi, lambda)` | U | 通用单比特门 U(θ, φ, λ)，3 个角度参数。 |
| `circuit_u_param(ptr, qubit, theta, phi, lambda)` | U | 同上，三个角度均为符号表达式。 |
| `circuit_xy(ptr, qubit, theta)` | XY | XY 交互族门，非对角元为 −i·e^(∓iθ)。 |
| `circuit_xy_param(ptr, qubit, param)` | XY | 同上，相位为符号表达式。 |
| `circuit_xy2p(ptr, qubit, theta)` | XY2P | 绕 θ 选取的 XY 平面轴 +90° 旋转。 |
| `circuit_xy2p_param(ptr, qubit, param)` | XY2P | 同上，轴角为符号表达式。 |
| `circuit_xy2m(ptr, qubit, theta)` | XY2M | 绕 θ 选取的 XY 平面轴 −90° 旋转。 |
| `circuit_xy2m_param(ptr, qubit, param)` | XY2M | 同上，轴角为符号表达式。 |
| `circuit_rxy(ptr, qubit, theta, phi)` | RXY | 绕 XY 平面内由 φ 确定的轴旋转 θ。 |
| `circuit_rxy_param(ptr, qubit, theta, phi)` | RXY | 同上，两个角度均为符号表达式。 |

---

## 双比特门

`cx`/`cy`/`cz`/`swap` 为固定矩阵门；其余以角度参数化，`_param` 变体以 `const CParameter*` 传角。

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `circuit_cx(ptr, control, target)` | CX | 受控 X（CNOT），控制位为 `1` 时翻转目标位。 |
| `circuit_cy(ptr, control, target)` | CY | 受控 Y，控制位为 `1` 时对目标位作用 Y。 |
| `circuit_cz(ptr, control, target)` | CZ | 受控 Z，控制位为 `1` 时对目标位作用 Z。 |
| `circuit_swap(ptr, a, b)` | SWAP | 交换两个比特的状态。 |
| `circuit_rxx(ptr, a, b, theta)` | RXX | 二体 Pauli 旋转 exp(−iθ X⊗X/2)。 |
| `circuit_rxx_param(ptr, a, b, param)` | RXX | 同上，角度为符号表达式。 |
| `circuit_ryy(ptr, a, b, theta)` | RYY | 二体 Pauli 旋转 exp(−iθ Y⊗Y/2)。 |
| `circuit_ryy_param(ptr, a, b, param)` | RYY | 同上，角度为符号表达式。 |
| `circuit_rzz(ptr, a, b, theta)` | RZZ | 二体 Pauli 旋转 exp(−iθ Z⊗Z/2)。 |
| `circuit_rzz_param(ptr, a, b, param)` | RZZ | 同上，角度为符号表达式。 |
| `circuit_rzx(ptr, a, b, theta)` | RZX | 二体 Pauli 旋转 exp(−iθ Z⊗X/2)，`a` 上为 Z、`b` 上为 X。 |
| `circuit_rzx_param(ptr, a, b, param)` | RZX | 同上，角度为符号表达式。 |
| `circuit_crx(ptr, control, target, theta)` | CRX | 受控 RX(θ)，控制位为 `1` 时旋转目标位。 |
| `circuit_crx_param(ptr, control, target, param)` | CRX | 同上，角度为符号表达式。 |
| `circuit_cry(ptr, control, target, theta)` | CRY | 受控 RY(θ)，控制位为 `1` 时旋转目标位。 |
| `circuit_cry_param(ptr, control, target, param)` | CRY | 同上，角度为符号表达式。 |
| `circuit_crz(ptr, control, target, theta)` | CRZ | 受控 RZ(θ)，控制位为 `1` 时旋转目标位。 |
| `circuit_crz_param(ptr, control, target, param)` | CRZ | 同上，角度为符号表达式。 |
| `circuit_fsim(ptr, a, b, theta, phi)` | fSim | fSim 门：`01`/`10` 子块按 θ 旋转，`11` 分量附加 e^(−iφ) 相位。 |
| `circuit_fsim_param(ptr, a, b, theta, phi)` | fSim | 同上，两个角度均为符号表达式。 |

---

## 三比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `circuit_ccx(ptr, control1, control2, target)` | CCX | Toffoli 门，两个控制位均为 `1` 时翻转目标位。 |

---

## 数学约定

旋转门采用半角指数定义：

```text
RX(θ) = exp(-i θ X / 2)
RY(θ) = exp(-i θ Y / 2)
RZ(θ) = exp(-i θ Z / 2)
RXY(θ, φ) = 绕 XY 平面内方位角为 φ 的轴旋转 θ（半角约定）
```

二体 Pauli 旋转门使用相同约定：

```text
RXX(θ) = exp(-i θ X⊗X / 2)
RYY(θ) = exp(-i θ Y⊗Y / 2)
RZZ(θ) = exp(-i θ Z⊗Z / 2)
RZX(θ) = exp(-i θ Z⊗X / 2)
```

其他门的矩阵形式：

```text
U(θ, φ, λ) = [[cos(θ/2),          -e^(-iλ)·sin(θ/2)],
              [e^(iφ)·sin(θ/2),    e^(i(φ+λ))·cos(θ/2)]]

fSim(θ, φ) = [[1,      0,           0,          0      ],
               [0,      cos(θ),     -i·sin(θ),   0      ],
               [0,     -i·sin(θ),   cos(θ),      0      ],
               [0,      0,           0,          e^(-iφ)]]
```

---

## 示例

```c
#include "cqlib_c.h"

/* Bell 线路：H(0) + CX(0, 1) */
CCircuit *c = circuit_new(2);

if (circuit_h(c, 0) != 0) {
    /* 处理错误 */
}
if (circuit_cx(c, 0, 1) != 0) {
    /* 处理错误 */
}

/* 符号参数：RZ(0, theta)，theta 为符号表达式 */
CParameter *theta = param_parse("theta");

if (circuit_rz_param(c, 0, theta) != 0) {
    /* 处理错误 */
}

param_free(theta);
circuit_free(c);
```

---

## 相关页面

- [参数](3_parameter.md)：`CParameter` 的创建、求值与释放。
- [线路](1_circuit.md) 与 [操作指令](4_operation_instruction.md)：`circuit_measure`、`circuit_reset`、`circuit_barrier`、`circuit_delay` 等非酉指令。
- [酉门](6_gate_unitary.md)：矩阵定义的自定义酉门。
- [多控制门](7_gate_mc_gate.md)：向任意标准门追加控制位。
- [线路转矩阵](13_circuit_to_matrix.md)：整线路酉矩阵的数值输出。
