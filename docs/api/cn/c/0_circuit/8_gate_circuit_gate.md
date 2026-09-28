# 线路门

本页介绍 C API 中的线路门：把一段已构造的线路封装为可命名、可复用的复合门；错误码与内存管理的全局约定见 [Overview](../0_overview.md)。

---

## circuit_to_gate(ptr, name)

把线路的一份冻结副本封装为命名复合门。传入线路被克隆，封装后仍可继续使用或释放。

参数：

- `ptr` (`const struct CCircuit *`)：源线路句柄。
- `name` (`const char *`)：复合门名称。

返回：新分配的 `struct CCircuitGate *`，用 `circuit_gate_free` 释放；输入为 NULL、名称含无效 UTF-8 或封装失败时返回 NULL。

封装时，源线路中的自由符号按其出现顺序构成复合门的调用签名，可通过 `circuit_gate_signature_params` 查询。

---

## circuit_gate_free(ptr)

释放复合门句柄。允许传 NULL，无返回值。

---

## circuit_gate_name(ptr)

返回复合门名称的新分配 C 字符串。

参数：

- `ptr` (`const struct CCircuitGate *`)：复合门句柄。

返回：`char *`，用 `cqlib_string_free` 释放；输入为 NULL 或出错时返回 NULL。

---

## circuit_gate_num_qubits(ptr)

返回复合门应用时需要的量子比特数量。

参数：

- `ptr` (`const struct CCircuitGate *`)：复合门句柄。

返回：`uintptr_t` 比特数；输入为 NULL 时返回 0。

---

## circuit_gate_num_params(ptr)

返回复合门调用签名中的位置参数个数。

参数：

- `ptr` (`const struct CCircuitGate *`)：复合门句柄。

返回：`uintptr_t` 参数个数；输入为 NULL 时返回 0。

---

## circuit_gate_signature_params_len(ptr)

返回调用签名中位置参数名的个数，即两步式输出的第一步。

参数：

- `ptr` (`const struct CCircuitGate *`)：复合门句柄。

返回：`uintptr_t` 签名参数名个数；输入为 NULL 时返回 0。

---

## circuit_gate_signature_params(ptr, out, len)

把调用签名的参数名拷贝到 `out`（两步式输出；先调用 `circuit_gate_signature_params_len` 查询长度）。

参数：

- `ptr` (`const struct CCircuitGate *`)：复合门句柄。
- `out` (`char **`)：输出缓冲区，调用者按长度分配；传 NULL 仅查询长度。
- `len` (`uintptr_t`)：`out` 可容纳的元素数。

返回：`uintptr_t` 签名参数名总数；每个写出的字符串由调用者用 `cqlib_string_free` 释放。

---

## circuit_circuit_gate(ptr, gate, qubits, qubits_len, params, params_len)

向线路追加复合门，`params` 按位置绑定到门的调用签名：第 i 个参数替换签名中第 i 个符号名，替换同时进行。

参数：

- `ptr` (`struct CCircuit *`)：目标线路句柄。
- `gate` (`const struct CCircuitGate *`)：复合门句柄。
- `qubits` (`const uint32_t *`)：复合门在外层线路中作用的比特索引数组，按数组顺序映射到门定义的比特。
- `qubits_len` (`uintptr_t`)：`qubits` 的长度，须等于 `circuit_gate_num_qubits(gate)`。
- `params` (`const struct CParameter *const *`)：位置参数句柄数组；无参数门传 `NULL, 0`。
- `params_len` (`uintptr_t`)：`params` 的长度，须等于 `circuit_gate_num_params(gate)`。

返回：`int32_t`，`0` 成功；`-1` NULL 输入（长度非 0 时数组为 NULL）；`-2` 比特越界；`-3` 比特或参数数量不匹配、应用失败。

---

## 示例：封装 Bell 线路为门并复用

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. 构造 Bell 子线路 */
    CCircuit *inner = circuit_new(2);
    circuit_h(inner, 0);
    circuit_cx(inner, 0, 1);

    /* 2. 封装为命名复合门；源线路被克隆，随后即可释放 */
    struct CCircuitGate *bell = circuit_to_gate(inner, "Bell");
    circuit_free(inner);

    if (bell == NULL) {
        return 1;
    }

    /* 3. 查询门元数据：num_qubits == 2，num_params == 0 */
    char *name = circuit_gate_name(bell);
    printf("gate: %s, qubits: %zu, params: %zu\n",
           name,
           circuit_gate_num_qubits(bell),
           circuit_gate_num_params(bell));
    cqlib_string_free(name);

    /* 4. 在外层线路中把 Bell 门复用两次 */
    CCircuit *outer = circuit_new(4);
    uint32_t first[2] = {0, 1};
    uint32_t second[2] = {2, 3};

    if (circuit_circuit_gate(outer, bell, first, 2, NULL, 0) != 0) {
        /* 处理错误 */
    }
    if (circuit_circuit_gate(outer, bell, second, 2, NULL, 0) != 0) {
        /* 处理错误 */
    }

    circuit_gate_free(bell);
    circuit_free(outer);
    return 0;
}
```

带参数的复合门按签名绑定：下面的子线路使用符号 `theta`，封装后签名为 `["theta"]`，外层调用以 `alpha` 替换。

```c
#include <stdlib.h>
#include "cqlib_c.h"

CCircuit *inner = circuit_new(1);
CParameter *theta = param_parse("theta");

circuit_rz_param(inner, 0, theta);
param_free(theta);

struct CCircuitGate *rot = circuit_to_gate(inner, "RzBlock");
circuit_free(inner);

/* 签名查询：circuit_gate_signature_params_len(rot) == 1 */
uintptr_t n = circuit_gate_signature_params_len(rot);
char **names = malloc(n * sizeof(char *));
circuit_gate_signature_params(rot, names, n);   /* names[0] == "theta" */
for (uintptr_t i = 0; i < n; ++i) {
    cqlib_string_free(names[i]);
}
free(names);

/* 外层调用：以 alpha 绑定 theta */
CCircuit *outer = circuit_new(1);
CParameter *alpha = param_parse("alpha");
const struct CParameter *bind[1] = {alpha};
uint32_t q[1] = {0};

circuit_circuit_gate(outer, rot, q, 1, bind, 1);

param_free(alpha);
circuit_gate_free(rot);
circuit_free(outer);
```

---

## 相关页面

- [线路](1_circuit.md)：线路的创建、追加与组合。
- [参数](3_parameter.md)：`CParameter` 的创建与释放。
- [酉门](6_gate_unitary.md)：矩阵定义的自定义酉门。
- [线路转矩阵](13_circuit_to_matrix.md)：复合门展开后整线路矩阵的计算。
