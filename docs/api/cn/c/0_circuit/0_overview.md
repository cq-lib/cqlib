# 量子电路（circuit）

C API 的量子电路模块以不透明句柄 `CCircuit*` 为中心，提供线路构造与查询、符号参数、门操作、经典数据与控制流、符号矩阵、参数化模板（Ansatz）、控制流图与线路 DAG 等能力。

错误码、内存所有权与数组两步式输出等全局约定见 [Overview](../0_overview.md)。

---

## 模块导航

| 分类 | 页面 | 主要对象 | 说明 |
| --- | --- | --- | --- |
| 线路容器 | [Circuit](1_circuit.md) | `CCircuit*` | 线路构造、属性查询、组合、反演、分解、全局相位与参数绑定。 |
| 量子比特 | [量子比特](2_qubit.md) | `CQubit` | 逻辑比特标识：创建、编号访问、受检转换、比较与字符串表示。 |
| 符号参数 | [符号参数](3_parameter.md) | `CParameter*`、`CCircuitParam`、`CParameterValue` | 表达式解析、求值与线路参数表驻留。 |
| 操作与指令 | [操作与指令](4_operation_instruction.md) | `COperation*`、`CValueOperation*` | 存储层操作、线路操作快照与延迟指令。 |
| 标准门 | [标准门](5_gate_standard.md) | `circuit_h` 等便捷函数 | 内置标准门，数值角与符号角（`*_param`）两种变体。 |
| 自定义酉门 | [自定义酉门](6_gate_unitary.md) | `circuit_unitary` 等 | 数值矩阵门与符号矩阵门。 |
| 多控制门 | [多控制门](7_gate_mc_gate.md) | `circuit_multi_control` | 在标准门前追加控制位。 |
| 复合门 | [复合门](8_gate_circuit_gate.md) | `CCircuitGate*`、`circuit_to_gate` | 将线路冻结为可复用复合门。 |
| 经典数据与控制流 | [经典数据与控制流](9_classical_control_flow.md) | `CClassicalVar*`、`CClassicalExpr*`、`circuit_if` 等 | 测量、经典表达式与结构化控制流。 |
| 符号矩阵 | [符号矩阵](10_symbolic_matrix.md) | `CSymbolicMatrix*` | 保留参数的密集符号矩阵与等价检查。 |
| Ansatz | [Ansatz](11_ansatz.md) | `CTwoLocal`、`CQAOAAnsatz` 等 | 参数化线路模板。 |
| 控制流图 | [CFG](12_cfg.md) | `CCircuitCFG*` | 线路控制流图视图、查询与重建。 |
| 矩阵转换 | [线路转矩阵](13_circuit_to_matrix.md) | `circuit_to_matrix` 等 | 纯酉线路的数值矩阵与全局相位。 |
| 线路 DAG | [线路 DAG](14_circuit_dag.md) | `CCircuitDag*` | 依赖有向无环图视图与线路重建。 |

---

## 核心概念

| 概念 | 说明 |
| --- | --- |
| **不透明句柄** | `struct CCircuit` 等类型在头文件中仅前向声明，不导出字段；全部访问都通过函数完成，句柄用对应的 `*_free` 释放。 |
| **量子比特编号** | 逻辑比特用 `uint32_t` id 表示。`circuit_new(n)` 创建连续 id `0..n-1`；`circuit_from_qubits` 允许显式给出（含稀疏的）id 列表。 |
| **门函数前缀** | 门操作统一以 `circuit_` 开头：固定门如 `circuit_h`、`circuit_cx`；带角度的门提供数值版（`circuit_rx`）与符号版（`circuit_rx_param`，接受 `CParameter*`）。 |
| **符号参数** | `param_parse` 从字符串解析表达式；参数驻留进线路参数表后以 `CCircuitParam`（固定值或表下标）表示。 |
| **测量与控制流** | 测量产生线路所有的不可变经典值，经典表达式与 `circuit_if`/`circuit_while`/`circuit_for_uint`/`circuit_switch` 描述运行时控制流。 |

受控门遵循“控制位在前、目标位在后”的顺序；例如 `circuit_cx(qc, control, target)` 中 `control` 为控制位。

---

## 最小示例

```c
#include <cqlib_c.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);       /* 0 成功，负值为错误码 */
    circuit_cx(qc, 0, 1);

    printf("qubits    = %zu\n", circuit_num_qubits(qc));
    printf("depth     = %zd\n", circuit_depth(qc, true));
    printf("operations= %zu\n", circuit_num_operations(qc));

    /* 两步式读取比特列表 */
    uintptr_t n = circuit_qubits_len(qc);
    uint32_t *ids = malloc(n * sizeof(uint32_t));
    circuit_qubits(qc, ids, n);   /* ids = {0, 1} */

    free(ids);
    circuit_free(qc);
    return 0;
}
```

线路构造完成后，通常交由 [态矢量](../3_qis/1_statevector.md) 或 [密度矩阵](../3_qis/2_density_matrix.md) 模拟执行；含测量或控制流的线路不再具有单一固定的酉矩阵表示，矩阵转换接口只适用于纯量子子线路（见 [线路转矩阵](13_circuit_to_matrix.md)）。
