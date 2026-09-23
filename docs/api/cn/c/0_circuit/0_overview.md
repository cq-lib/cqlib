# 量子电路

线路模块是 Cqlib C API 的核心入口，用于构建、表示和变换量子线路。所有线路状态保存在不透明句柄 `CCircuit` 中，C 代码通过自由函数操作线路。

线路模块主要覆盖以下能力：

- **基础线路构造**：创建逻辑量子比特集合，并按顺序追加量子门、测量、重置、屏障等操作。
- **门体系**：无参单比特门、参数化单比特门、双比特门与三比特门的全量自由函数；参数化门同时提供数值版本与符号参数版本。
- **符号参数**：使用 `CParameter` 表示门角等可调参数，支持表达式解析、求值与整线路参数绑定。
- **线路属性与结构**：查询量子比特数、操作数、深度，支持逆线路、复合门分解、线路拼接与追加量子比特。
- **矩阵转换**：将小规模纯量子门线路导出为稠密酉矩阵（两步式 API）。

---

## 核心约定

| 概念         | 说明                                                                                                  |
| ---------- | --------------------------------------------------------------------------------------------------- |
| **不透明句柄**  | `CCircuit` 等句柄类型在头文件中仅有前置声明，内部由 Rust 管理，只能通过 API 创建与销毁。                                      |
| **量子比特编号** | `Circuit` 使用连续逻辑编号 `0..num_qubits-1`；越界返回错误码 `-2`。                                                  |
| **错误码**    | 门与结构操作返回 `int32_t`，成功为 `0`，详见[概览](../0_overview.md#错误码)。                                            |
| **返回新对象**  | `circuit_inverse` / `circuit_decompose` / `circuit_assign_params` 等返回独立拥有的新线路，需用 `circuit_free` 释放。 |

---

## 快速示例

### Bell 态制备

```c
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);
circuit_free(qc);
```

### 参数化旋转

```c
CParameter *theta = param_parse("theta/2");
circuit_rx_param(qc, 1, theta);
param_free(theta);

CCircuit *bound = circuit_assign_params(qc, "theta:3.1415926");
circuit_free(bound);
```

---

## API 概览

| 对象           | 页面                                          | 简介                         |
| ------------ | ------------------------------------------- | -------------------------- |
| `CCircuit`   | [Circuit](1_circuit.md)                     | 线路主容器：构造与释放、门操作、线路属性、结构操作。 |
| `CParameter` | [Parameter](2_parameter.md)                 | 符号参数表达式：解析、求值与整线路参数绑定。     |
| 酉矩阵导出        | [Circuit To Matrix](3_circuit_to_matrix.md) | 两步式导出线路的稠密酉矩阵。             |
