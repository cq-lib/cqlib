# 线路转矩阵

`circuit_to_matrix_len` 与 `circuit_to_matrix` 把数值线路转换为其酉矩阵：`n` 个量子比特对应 `2^n × 2^n` 维矩阵，行主序、交错 `(re, im)` 双精度输出，并包含线路的全局相位。矩阵规模随量子比特数按 `4^n` 增长，适合小规模线路。保留符号表达式的转换见 [符号矩阵](10_symbolic_matrix.md)；错误码、句柄释放与两步式数组输出等通用约定见 [Overview](../0_overview.md)。

---

## 两步式输出

### circuit_to_matrix_len(ptr, qubits_order, order_len)

返回酉矩阵的复数元素个数（行数 × 列数），用于分配输出缓冲区。传 `qubits_order` = NULL 使用默认排序。

- `ptr` (`const struct CCircuit*`)：线路句柄。
- `qubits_order` (`const uintptr_t*`)：量子比特顺序数组，可为 NULL。
- `order_len` (`uintptr_t`)：顺序数组长度。

返回：复数元素个数；`ptr` 为 NULL 或转换失败（含未解析符号参数、非酉操作、量子比特顺序非法等）时返回 0。

### circuit_to_matrix(ptr, qubits_order, order_len, out, buffer_len)

把酉矩阵按交错 `(实部, 虚部)` 双精度拷贝进 `out`，返回写入的复数元素个数。

- `ptr` (`const struct CCircuit*`)：线路句柄。
- `qubits_order` (`const uintptr_t*`)：量子比特顺序数组，可为 NULL。
- `order_len` (`uintptr_t`)：顺序数组长度。
- `out` (`double*`)：输出缓冲区，须容纳至少 `2 × circuit_to_matrix_len(...)` 个 double。
- `buffer_len` (`uintptr_t`)：缓冲区容量（double 数量）。

返回：写入的复数元素个数；`ptr` 或 `out` 为 NULL、`buffer_len` 不足（写入前拒绝）或转换失败时返回 0。

第 `i` 个复数元素为 `{ out[2*i], out[2*i+1] }`，位于矩阵第 `i / 2^n` 行、第 `i % 2^n` 列（行主序）。

---

## 量子比特顺序

`qubits_order` 指定矩阵基态索引中量子比特的排列，必须与线路的量子比特集合完全一致——不能遗漏、重复或包含未知量子比特，否则视为转换失败（两个接口均返回 0）。

- `NULL` / `order_len == 0`：默认按量子比特编号升序排列；
- 顺序中第一个量子比特对应基态索引的最低有效位（little-endian），默认排序下即编号最小的量子比特（如 qubit 0）为最低位。

```c
/* 显式指定：qubit 1 为最低位 */
uintptr_t order[2] = {1, 0};
uintptr_t len = circuit_to_matrix_len(circuit, order, 2);
```

---

## 参数化线路

数值矩阵要求线路中的符号参数均已解析为具体数值。线路含未解析符号参数时，两个接口均返回 0；先用 `circuit_assign_params` 绑定参数（见 [线路](1_circuit.md)）再转换：

```c
struct CParameter* theta = param_parse("theta");

struct CCircuit* circuit = circuit_new(1);
circuit_rx_param(circuit, 0, theta);

/* theta 未绑定：返回 0 */
uintptr_t unbound = circuit_to_matrix_len(circuit, NULL, 0);

/* 绑定 theta = 0.5 后转换：1 个量子比特 → 2 × 2 = 4 */
struct CCircuit* bound = circuit_assign_params(circuit, "theta:0.5");
uintptr_t len = circuit_to_matrix_len(bound, NULL, 0);
```

---

## 全局相位

矩阵包含线路的全局相位 `theta`：设不含全局相位时线路矩阵为 `U`，则转换结果为 `exp(i * theta) * U`。

---

## 示例

两步式取回 2 量子比特线路（H + CX）的 4 × 4 酉矩阵：

```c
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit* circuit = circuit_new(2);
    circuit_h(circuit, 0);
    circuit_cx(circuit, 0, 1);

    /* 第一步：查询复数元素个数（2^2 × 2^2 = 16） */
    uintptr_t len = circuit_to_matrix_len(circuit, NULL, 0);
    if (len == 0) {
        circuit_free(circuit);
        return 1;
    }

    /* 第二步：分配至少 2 * len 个 double 并拷贝 */
    double* buffer = (double*)malloc(len * 2 * sizeof(double));
    uintptr_t written = circuit_to_matrix(circuit, NULL, 0, buffer, len * 2);
    if (written != len) {
        free(buffer);
        circuit_free(circuit);
        return 1;
    }

    /* 例如读取第 0 行第 3 列元素：位于 buffer[6]（实部）与 buffer[7]（虚部） */
    printf("U[0][3] = %g + %gi\n", buffer[6], buffer[7]);

    free(buffer);
    circuit_free(circuit);
    return 0;
}
```

线路构造与参数绑定接口（`circuit_new`、`circuit_assign_params` 等）见 [线路](1_circuit.md)。
