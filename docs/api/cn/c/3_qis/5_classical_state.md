# 经典态（C）

经典态页面覆盖三类句柄：测量声明 `CMeasurement`（把测量比特与结果比特序绑定在一起）、测量产生的不可变经典值 `CClassicalValue`，以及采样结果列表 `COutcomeList`。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 测量声明

### measurement_new(value_index, ty_tag, ty_width, qubits, len)

由原始组件创建独立测量声明：`qubits[0]` 成为结果比特 0（最低有效位），`qubits[1]` 成为结果比特 1，依此类推。

该构造面向自包含的比特序工具（`measurement_project`、`measurement_project_basis`、`measurement_check_qubits`）：其内部经典值使用全新的线路身份构建，不属于任何线路，由此派生的表达式会被 `circuit_validate_classical_expr` 拒绝；线路集成的测量用 `circuit_measure` / `circuit_measure_bits` 等构造（见 [经典数据与控制流](../0_circuit/9_classical_control_flow.md)）。

参数：

- `value_index` (`uint32_t`)：值表下标。
- `ty_tag` (`uint32_t`)：结果类型标签，取 `CQLIB_CLASSICAL_TYPE_*` 之一（`Bit` 与 `BitVec` 是直接的测量目标类型）。
- `ty_width` (`uint32_t`)：结果类型宽度。
- `qubits` (`const uint32_t*`)：被测比特下标数组，按结果比特序排列。
- `len` (`uintptr_t`)：被测比特数。

返回：成功返回新建的 `CMeasurement*`；`qubits` 为 NULL、列表为空或类型标签非法时返回 NULL。

### measurement_free(ptr)

释放测量声明句柄，允许传 NULL。

### measurement_value(ptr)

返回该测量产生的不可变经典值。

返回：新建的 `CClassicalValue*`（用 `classical_value_free` 释放）；NULL 输入返回 NULL。

### measurement_expr(ptr)

创建读取该测量不可变值的表达式。

返回：新建的 `CClassicalExpr*`（用 `classical_expr_free` 释放）；NULL 输入返回 NULL。

### measurement_qubits_len(ptr) / measurement_qubits(ptr, buffer, len)

两步式读取被测比特下标（按结果比特序）：`*_len` 返回个数（NULL 返回 0），填充接口把下标拷入 `buffer`。

返回（填充接口）：0 成功；-1 空指针；-8 `len` 与 `measurement_qubits_len` 不符。

### measurement_width(ptr)

返回被测比特数（即结果位数）；NULL 返回 0。

### measurement_ty(ptr, tag, width)

把结果类型写到出参。

返回：0 成功（`*tag` 为 `CQLIB_CLASSICAL_TYPE_*` 之一，`*width` 为类型宽度）；-1 空指针。

### measurement_check_qubits(ptr, num_qubits)

校验全部被测比特对 `num_qubits` 比特的态均有效。

返回：0 成功；-1 空指针；-2 存在越界比特下标。

### measurement_project(ptr, bitstring)

把全寄存器测量结果投影到该测量的比特序上：`bitstring` 是覆盖整个寄存器的大端 `'0'`/`'1'` 串（最左字符为最高比特，与 `statevector_measure_all` 输出一致）；若 `qubits[i]` 在结果中为 1，则结果比特 `i` 置 1。

返回：`measurement_width` 位的大端位串，用 `cqlib_string_free` 释放；NULL 输入、非法 UTF-8 或非法位串返回 NULL。

### measurement_project_basis(ptr, basis)

把计算基下标投影到该测量的比特序上：若 `basis` 的比特 `qubits[i]` 为 1，则结果比特 `i` 置 1。

返回：`measurement_width` 位的大端位串，用 `cqlib_string_free` 释放；NULL 输入返回 NULL。

---

## 经典值

### classical_value_free(ptr)

释放经典值句柄，允许传 NULL。

### classical_value_index(ptr)

返回线路局部值表下标；NULL 返回 `UINT32_MAX`。

### classical_value_ty(ptr, tag, width)

把值类型写到出参。

返回：0 成功（`*tag` 为 `CQLIB_CLASSICAL_TYPE_*` 之一，`*width` 为类型宽度）；-1 空指针。

### classical_value_expr(ptr)

创建读取该不可变运行期值的表达式。

返回：新建的 `CClassicalExpr*`（用 `classical_expr_free` 释放）；NULL 输入返回 NULL。

---

## 采样结果列表

`*_sample_shots` 类接口（见 [态矢量](1_statevector.md)、[稳定器](4_stabilizer.md) 等）返回 `COutcomeList*`：每个元素是大端二进制位串，最左字符对应比特 N-1、最右字符对应比特 0。

### outcome_list_free(ptr)

释放采样结果列表，允许传 NULL。

### outcome_list_len(ptr)

返回位串个数；NULL 返回 0。

### outcome_list_get(ptr, index)

返回下标 `index` 处的位串。

返回：堆上 C 字符串，用 `cqlib_string_free` 释放；越界返回 NULL。

---

## 示例

测量声明 → 模拟采样 → 遍历 OutcomeList 的完整流程：

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. 构造 Bell 线路并声明测量 */
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);
    if (circuit_measure(qc, 0) != 0) {
        circuit_free(qc);
        return 1;
    }

    /* 2. 精确模拟后采样 100 次 */
    struct CStatevector *sv = statevector_from_circuit(qc);
    struct COutcomeList *shots = statevector_sample_shots(sv, 100);

    /* 3. 遍历 OutcomeList：位串为大端（最左字符是比特 N-1） */
    uintptr_t n = outcome_list_len(shots);  /* 100 */
    for (uintptr_t i = 0; i < n; i++) {
        char *bs = outcome_list_get(shots, i);  /* "00" 或 "11" */
        printf("%s\n", bs);
        cqlib_string_free(bs);
    }

    outcome_list_free(shots);
    statevector_free(sv);
    circuit_free(qc);

    /* 4. 独立测量声明：qubits[0] 为结果比特 0，全寄存器投影 */
    uint32_t qubits[2] = {1, 0};
    struct CMeasurement *m =
        measurement_new(0, CQLIB_CLASSICAL_TYPE_BIT_VEC, 2, qubits, 2);
    char *proj = measurement_project(m, "01");  /* "10" */
    cqlib_string_free(proj);
    measurement_free(m);
    return 0;
}
```
