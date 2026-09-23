# Circuit

`CCircuit` 是 C API 中最核心的线路容器。它负责保存量子比特集合、按顺序排列的操作序列、符号参数与全局相位。所有状态保存在不透明句柄中，C 代码通过自由函数操作线路。

```c
#include <cqlib_c.h>
```

---

## 构造与释放

### circuit_new(num_qubits)

创建包含 `num_qubits` 个逻辑量子比特（编号 `0..num_qubits-1`）的空线路。

参数：

- `num_qubits` (`uintptr_t`)：量子比特数量。

返回：

- `CCircuit *`：堆分配线路对象，需用 `circuit_free` 释放；失败返回 NULL。

### circuit_free(ptr)

释放线路对象。允许传 NULL。

```c
CCircuit *qc = circuit_new(3);
// ... 构建与使用 ...
circuit_free(qc);
```

---

## 门操作

所有门函数返回 `int32_t` 错误码：

| 返回值 | 含义 |
| --- | --- |
| `0` | 成功 |
| `-1` | 线路句柄为 NULL |
| `-2` | 量子比特编号越界 |
| `-3` | 其他线路错误 |

参数化门同时提供数值版本与符号参数版本（`*_param`）：`_param` 变体把对应 `double` 参数替换为一个或多个 `const CParameter *`，参数对象由 [param_parse](2_parameter.md) 创建。

### 1. 无参数单比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `circuit_i(qc, qubit)` | `I` | 恒等门 |
| `circuit_h(qc, qubit)` | `H` | Hadamard 门 |
| `circuit_x(qc, qubit)` | `X` | Pauli-X 门 |
| `circuit_y(qc, qubit)` | `Y` | Pauli-Y 门 |
| `circuit_z(qc, qubit)` | `Z` | Pauli-Z 门 |
| `circuit_s(qc, qubit)` | `S` | S 门（√Z） |
| `circuit_sdg(qc, qubit)` | `S†` | S 伴随门 |
| `circuit_t(qc, qubit)` | `T` | T 门（√S） |
| `circuit_tdg(qc, qubit)` | `T†` | T 伴随门 |
| `circuit_x2p(qc, qubit)` | `X2P` | 绕 X 轴 +π/2 旋转（√X） |
| `circuit_x2m(qc, qubit)` | `X2M` | 绕 X 轴 −π/2 旋转 |
| `circuit_y2p(qc, qubit)` | `Y2P` | 绕 Y 轴 +π/2 旋转（√Y） |
| `circuit_y2m(qc, qubit)` | `Y2M` | 绕 Y 轴 −π/2 旋转 |

### 2. 参数化单比特门

| 函数 | 门 | 参数 | 说明 |
| --- | --- | --- | --- |
| `circuit_rx(qc, qubit, theta)` | `RX` | θ | 绕 X 轴旋转 |
| `circuit_ry(qc, qubit, theta)` | `RY` | θ | 绕 Y 轴旋转 |
| `circuit_rz(qc, qubit, theta)` | `RZ` | θ | 绕 Z 轴旋转 |
| `circuit_phase(qc, qubit, lambda)` | `P` | λ | 相位门 |
| `circuit_u(qc, qubit, theta, phi, lambda)` | `U` | θ, φ, λ | 通用单比特门 |
| `circuit_xy(qc, qubit, theta)` | `XY` | θ | XY 交互门 |
| `circuit_xy2p(qc, qubit, theta)` | `XY2P` | θ | 正半角 XY 门 |
| `circuit_xy2m(qc, qubit, theta)` | `XY2M` | θ | 负半角 XY 门 |
| `circuit_rxy(qc, qubit, theta, phi)` | `RXY` | θ, φ | XY 平面任意轴旋转 |

每个门都有对应的 `_param` 变体：`circuit_rx_param`、`circuit_ry_param`、`circuit_rz_param`、`circuit_phase_param`、`circuit_xy_param`、`circuit_xy2p_param`、`circuit_xy2m_param` 各接受一个 `CParameter *`；`circuit_u_param` 接受 θ、φ、λ 三个参数对象；`circuit_rxy_param` 接受 θ、φ 两个参数对象。

### 3. 双比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `circuit_cx(qc, control, target)` | `CX` | CNOT 门 |
| `circuit_cy(qc, control, target)` | `CY` | controlled-Y 门 |
| `circuit_cz(qc, control, target)` | `CZ` | controlled-Z 门 |
| `circuit_swap(qc, a, b)` | `SWAP` | 交换两个量子比特的状态 |
| `circuit_rxx(qc, a, b, theta)` | `RXX` | `exp(-i θ XX / 2)` |
| `circuit_ryy(qc, a, b, theta)` | `RYY` | `exp(-i θ YY / 2)` |
| `circuit_rzz(qc, a, b, theta)` | `RZZ` | `exp(-i θ ZZ / 2)` |
| `circuit_rzx(qc, a, b, theta)` | `RZX` | `exp(-i θ ZX / 2)` |
| `circuit_crx(qc, control, target, theta)` | `CRX` | controlled-RX 门 |
| `circuit_cry(qc, control, target, theta)` | `CRY` | controlled-RY 门 |
| `circuit_crz(qc, control, target, theta)` | `CRZ` | controlled-RZ 门 |
| `circuit_fsim(qc, a, b, theta, phi)` | `fSim` | fSim(theta, phi) 门 |

同样各提供 `*_param` 变体：单参数门（`rxx/ryy/rzz/rzx/crx/cry/crz`）接受一个参数对象，`circuit_fsim_param` 接受 θ、φ 两个参数对象。

### 4. 三比特门

| 函数 | 门 | 说明 |
| --- | --- | --- |
| `circuit_ccx(qc, control1, control2, target)` | `CCX` | Toffoli 门 |

### 5. 非酉指令

| 函数 | 指令 | 说明 |
| --- | --- | --- |
| `circuit_measure(qc, qubit)` | `measure` | 在 Z 基测量单个量子比特，结果记录为线路经典值 |
| `circuit_reset(qc, qubit)` | `reset` | 将量子比特复位到 `\|0⟩` |
| `circuit_barrier(qc, qubits, count)` | `barrier` | 在指定量子比特上插入屏障 |

`circuit_barrier` 的行为：

- `qubits` 为 NULL 或 `count` 为 0：创建覆盖**全部**量子比特的全局屏障；
- 否则：仅在 `qubits` 数组指定的比特上创建屏障。

barrier 用于阻止编译器跨该边界重排相关量子比特上的操作；它本身不改变量子态。

### 示例

```c
CCircuit *qc = circuit_new(3);
circuit_h(qc, 0);                 // H(0)
circuit_cx(qc, 0, 1);             // CX(0, 1)
circuit_rzz(qc, 1, 2, 0.25);      // RZZ(1, 2, 0.25)

/* 符号参数版本 */
CParameter *theta = param_parse("theta/2");
circuit_rx_param(qc, 2, theta);   // RX(2, theta/2)
param_free(theta);

/* 非酉指令 */
uint32_t target[2] = {0, 1};
circuit_barrier(qc, target, 2);   // 局部 barrier
circuit_measure(qc, 0);
circuit_reset(qc, 2);

circuit_free(qc);
```

---

## 线路属性

```c
uintptr_t circuit_num_qubits(const struct CCircuit *ptr);
uintptr_t circuit_num_operations(const struct CCircuit *ptr);
uintptr_t circuit_num_parameters(const struct CCircuit *ptr);
int32_t   circuit_validate(const struct CCircuit *ptr);
intptr_t  circuit_depth(const struct CCircuit *ptr, bool recurse);
uintptr_t circuit_width(const struct CCircuit *ptr);
uintptr_t circuit_qubits_len(const struct CCircuit *ptr);
uintptr_t circuit_qubits(const struct CCircuit *ptr, uint32_t *out, uintptr_t len);
```

| 函数 | 返回 | 说明 |
| --- | --- | --- |
| `circuit_num_qubits` | 比特数 | NULL 句柄返回 `0` |
| `circuit_width` | 比特数 | `num_qubits` 的别名 |
| `circuit_num_operations` | 操作数 | 按追加顺序计数的操作总数 |
| `circuit_num_parameters` | 符号参数数 | 线路中记录的符号参数数量，全部绑定后为 `0` |
| `circuit_validate` | 错误码 | 校验线路一致性；成功 `0`，失败 `-3` |
| `circuit_depth(recurse)` | 深度 | 错误（NULL）返回负值 |
| `circuit_qubits_len` | 比特数 | 用于分配 `circuit_qubits` 缓冲区 |

`circuit_depth(ptr, recurse)` 用于估算线路深度。深度表示按尽早调度（ASAP）方式排列后，最长量子比特路径上的操作层数：普通门和非酉指令通常贡献一层，`barrier` 会约束其覆盖量子比特上的重排。`recurse` 为 `true` 时递归展开复合子线路统计。

`circuit_qubits` 将量子比特编号拷入 `out`，返回线路总比特数。当 `out` 为 NULL 或 `len` 小于总比特数时不执行拷贝，仅返回总数——可以先调用它探查所需容量：

```c
CCircuit *qc = circuit_new(3);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

printf("qubits=%zu ops=%zu depth=%zu\n",
       (size_t)circuit_num_qubits(qc),
       (size_t)circuit_num_operations(qc),
       (size_t)circuit_depth(qc, false));

uint32_t ids[3];
circuit_qubits(qc, ids, 3);   // ids = {0, 1, 2}

circuit_free(qc);
```

---

## 结构操作

### circuit_remove_operation(ptr, index)

删除指定位置的操作，后续操作前移。

参数：

- `index` (`uintptr_t`)：操作下标，按追加顺序从 0 计。

返回：

- `int32_t`；成功 `0`，越界等错误返回负错误码。

### circuit_inverse(ptr)

返回逆线路：操作顺序反转，并对每个可逆操作取逆。

返回：

- `CCircuit *`：**独立拥有**的新线路，需 `circuit_free` 释放；失败返回 NULL。

### circuit_decompose(ptr)

返回递归展开复合门后的副本。

对从 QASM/QCIS 解析或编译结果中携带复合门的线路递归展开。

返回：

- `CCircuit *`：新线路；失败返回 NULL。

### circuit_compose(ptr, other, qubits_map, map_len)

将 `other` 的操作按顺序追加到当前线路，行为由 `qubits_map` 决定：

- `qubits_map` 为 NULL（`map_len` 传 0）：按量子比特编号对齐合并，`other` 使用了当前线路不存在的编号时自动补齐比特；
- `qubits_map` 非空：将 `other` 的第 `i` 个量子比特映射到当前线路的 `qubits_map[i]` 上。

返回：

- `int32_t`；成功 `0`，映射目标越界返回 `-2`。

与逆线路组合是典型用法：

```c
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

CCircuit *inv = circuit_inverse(qc);
circuit_compose(qc, inv, NULL, 0);     // qc 现在等价于恒等操作

circuit_free(inv);
circuit_free(qc);
```

### circuit_add_qubits(ptr, qubits, count)

向线路追加新的量子比特。

参数：

- `qubits` (`const uint32_t *`)：新比特的编号数组；
- `count` (`uintptr_t`)：数组长度。

返回：

- `int32_t`；成功 `0`，编号重复或非法返回负错误码。

### circuit_set_global_phase(ptr, phase) / circuit_set_global_phase_param(ptr, param)

设置线路全局相位，分别为数值版本与符号参数版本。

返回：

- `int32_t`；成功 `0`，NULL 句柄或非法参数返回负错误码。

```c
CCircuit *qc = circuit_new(1);
circuit_x(qc, 0);
circuit_set_global_phase(qc, 3.141592653589793 / 4.0);
circuit_free(qc);
```

---

符号参数的解析、求值与整线路绑定见 [Parameter](2_parameter.md)；酉矩阵导出见 [Circuit To Matrix](3_circuit_to_matrix.md)。
