# 中间表达（C）

C 绑定的 IR 接口在 `CCircuit` 与三种线路文本格式（QCIS、OpenQASM 2.0、OpenQASM 3.0）之间双向转换。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 四函数约定

每种格式提供同一组四个入口，前缀 `<prefix>` 分别为 `qcis`、`qasm2`、`qasm3`：

| 动作 | 函数 | 返回 |
| --- | --- | --- |
| 字符串 → `CCircuit` | `<prefix>_loads(source)` | 成功返回新建的 `CCircuit*`，失败返回 NULL |
| 文件 → `CCircuit` | `<prefix>_load(path)` | 成功返回新建的 `CCircuit*`，失败返回 NULL |
| `CCircuit` → 字符串 | `<prefix>_dumps(circuit)` | 成功返回堆上 C 字符串，失败返回 NULL |
| `CCircuit` → 文件 | `<prefix>_dump(circuit, path)` | 成功返回 0，失败返回负错误码 |

- `loads` / `load` 得到的线路用 `circuit_free` 释放（见 [Circuit](../0_circuit/1_circuit.md)）。
- `dumps` 返回的字符串用 `cqlib_string_free` 释放。
- `dump` 返回 `int32_t` 错误码：空指针为 -1，路径不是合法 UTF-8 为 -8，导出或写入失败为 -5。
- 解析方向不修改任何输入；`loads(dumps(circuit))` 得到的线路与原线路在操作序列上等价。
- 也可以由一种格式读入、另一种格式写出，完成格式转换，例如把 OpenQASM 2.0 文本写成 QCIS 文本。

---

## 页面导航

| 页面 | 内容 |
| --- | --- |
| [QCIS](1_qcis.md) | `qcis_loads` / `qcis_load` / `qcis_dumps` / `qcis_dump` 与 QCIS 原生门集。 |
| [OpenQASM 2.0](2_qasm2.md) | `qasm2_loads` / `qasm2_load` / `qasm2_dumps` / `qasm2_dump` 与支持的 2.0 子集。 |
| [OpenQASM 3.0](3_qasm3.md) | `qasm3_loads` / `qasm3_load` / `qasm3_dumps` / `qasm3_dump` 与支持的 3.0 子集。 |

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

/* 字符串 -> 线路 -> 字符串 */
struct CCircuit *c = qcis_loads("H Q0\nCZ Q0 Q1\n");
if (c == NULL) {
    /* 解析失败 */
}

char *text = qcis_dumps(c);
if (text != NULL) {
    printf("%s", text);       /* "H Q0\nCZ Q0 Q1\n" */
    cqlib_string_free(text);
}

circuit_free(c);

/* 格式转换：QCIS 文本写出为 OpenQASM 2.0 文本 */
struct CCircuit *c2 = qcis_loads("X Q0\nY Q1\n");
char *qasm = qasm2_dumps(c2);  /* 含 "OPENQASM 2.0;" 头 */
cqlib_string_free(qasm);
circuit_free(c2);
```
