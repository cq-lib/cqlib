# C API 概览

## 欢迎查阅 Cqlib C API 参考手册！



---

## 文档导航

### 量子电路（circuit）

- [Overview](0_circuit/0_overview.md)
- [Circuit](0_circuit/1_circuit.md)
- [Parameter](0_circuit/2_parameter.md)
- [Circuit To Matrix](0_circuit/3_circuit_to_matrix.md)

### 中间表达（ir）

- [QCIS](1_ir/1_qcis.md)
- [OpenQASM 2.0](1_ir/2_qasm2.md)
- [OpenQASM 3.0](1_ir/3_qasm3.md)

### 设备模块（device）

- [Topology](2_device/1_topology.md)
- [Device](2_device/2_properties_device.md)
- [Layout](2_device/3_layout.md)
- [NoiseModel](2_device/4_noise.md)
- [ExecutionResult](2_device/5_result.md)

### 量子信息（qis）

- [Overview](3_qis/0_overview.md)
- [Statevector](3_qis/1_statevector.md)
- [DensityMatrix](3_qis/2_density_matrix.md)
- [StabilizerState](3_qis/3_stabilizer_state.md)
- [PauliString](3_qis/4_pauli_string.md)
- [Hamiltonian](3_qis/5_hamiltonian.md)

### 编译优化（compile）

- [Compile](4_compile/1_compile.md)

### 可视化（visualization）

- [TextDrawer](5_visualization/1_text_drawer.md)
- [FigureDrawer](5_visualization/2_figure_drawer.md)

### 错误缓解（error_mitigation）

- [ErrorMitigation](6_error_mitigation/1_error_mitigation.md)
- [ZNEMitigation](6_error_mitigation/2_zne_mitigation.md)
- [VirtualDistillation](6_error_mitigation/3_virtual_distillation.md)

---

## 快速上手

```c
#include <stdio.h>
#include <cqlib_c.h>

int main(void) {
    // 1. 构建两比特线路：H(0), CX(0,1)
    CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    // 2. 用 Statevector 模拟
    CStatevector *sv = statevector_from_circuit(qc);
    double probs[4];
    statevector_probabilities(sv, probs, 4);
    printf("P(00)=%.2f P(11)=%.2f\n", probs[0], probs[3]);

    // 3. 释放
    statevector_free(sv);
    circuit_free(qc);
    return 0;
}
```

编译链接（MinGW gcc 示例；C 接口需先以 gnu target 构建，规则见 [crates/binding-c/README.md](../../../../crates/binding-c/README.md)）：

```bash
cargo build --release -p binding-c --target x86_64-pc-windows-gnu
gcc main.c -Icrates/binding-c/include -Ltarget/x86_64-pc-windows-gnu/release -lbinding_c -lntdll -lws2_32 -lbcrypt -luserenv -ladvapi32 -o demo
```

---

## 全局约定

### 错误码

所有返回 `int32_t` 的接口遵循统一错误码体系（正数/零为成功或查询结果）：

| 错误码  | 名称               | 典型场景                         |
| ---- | ---------------- | ---------------------------- |
| `0`  | Ok               | 成功（或查询类接口的有效返回值）             |
| `-1` | NullPtr          | 必需的句柄指针为 NULL             |
| `-2` | QubitOutOfBounds | 量子比特编号超出线路范围                 |
| `-3` | CircuitError     | 线路操作失败（结构非法、操作数错等）           |
| `-4` | ParseError       | 字符串/格式解析失败（QASM、参数表达式、未知门名等） |
| `-5` | IoError          | 文件读写失败                       |
| `-6` | CompilerError    | 编译过程失败                       |
| `-7` | SimulationError  | 模拟过程失败                       |
| `-8` | InvalidParam     | 参数非法（概率越界、长度不匹配等）            |

### 内存所有权

1. **构造返回堆指针**：`circuit_new`、`statevector_from_circuit` 等返回的句柄对象由调用方持有，用对应 `*_free` 释放；所有 `*_free` 均允许传 NULL。
2. **字符串用 `cqlib_string_free` 释放**：返回 `char *` 的接口（`qasm2_dumps`、`device_name` 等）都是堆分配，必须用 `cqlib_string_free`，不能用 C 标准库 `free`（分配器不同）。
3. **数组两步式输出**：大数组（概率、酉矩阵、量子比特列表）遵循 `_len` + 读取函数的两步模式——先调用 `*_len` 获得元素数量并分配缓冲区，再调用对应读取函数（如 `statevector_probabilities`）填充。
4. **嵌套列表用配套 free**：`outcome_list_free`、`counts_list_free`、`circuit_list_free`、`topology_free` 分别释放对应的列表/克隆对象。
5. **显式接管所有权**：少数接口会接管入参所有权（如 `hamiltonian_from_pauli` 接管并释放 Pauli 字符串），头文件注释均有标注。

### 常量标签

枚举常量以 `#define` 宏形式在头文件中提供，一律使用宏而非常数字面量：

| 常量组                                                                      | 取值           |
| ------------------------------------------------------------------------ | ------------ |
| `PAULI_I/X/Y/Z`                                                          | Pauli 算符 0–3 |
| `COMPILE_MODE_NORMAL/ENHANCED`                                           | 编译模式 0/1     |
| `COMPILE_TARGET_LOGICAL/BASIS/DEVICE/TOPOLOGY_BASIS`                     | 编译目标 0–3     |
| `NOISE_BIT_FLIP/PHASE_FLIP/DEPOLARIZING/AMPLITUDE_DAMPING/PHASE_DAMPING` | 单比特噪声通道 0–4  |
| `NOISE_TWO_DEPOLARIZING`                                                 | 双比特去极化通道 0   |
| `MITIGATION_ZNE/VIRTUAL_DISTILLATION`                                    | 缓解方法 0/1     |
| `PROCESS_ZNE_POLYNOMIAL/ZNE_EXPONENTIAL/VIRTUAL_DISTILLATION`            | 后处理方法 0–2    |

### 线程与并发

句柄对象非线程安全：同一对象的并发访问（包括 `_free` 释放）需由调用方自行加锁；不同对象相互独立，可并行使用。

### 复数布局

复数使用 `Complex64` 结构（`double re; double im;`），矩阵按行主序、实虚交错存储。
