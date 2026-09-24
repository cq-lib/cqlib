# QCIS（C）

C 绑定提供 QCIS 文本与 `CCircuit` 之间的四个转换入口。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 接口一览

| 函数 | 方向 |
| --- | --- |
| `qcis_loads` | QCIS 字符串 → `CCircuit` |
| `qcis_load` | QCIS 文件 → `CCircuit` |
| `qcis_dumps` | `CCircuit` → QCIS 字符串 |
| `qcis_dump` | `CCircuit` → QCIS 文件 |

---

## 解析

### qcis_loads(source)

从 QCIS 字符串解析线路，得到新的 `CCircuit*`。

参数：

- `source` (`const char*`)：QCIS 文本，NUL 结尾。

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；`source` 为 NULL、不是合法 UTF-8 或解析失败时返回 NULL。

解析失败的原因包括：比特写法不是 `Q<id>`、`Q` 后编号无法解析、门的比特数或参数个数与规格不符、参数表达式为空、门名不在 QCIS 指令集内、空行或没有有效内容。

### qcis_load(path)

从文件读取 QCIS 文本并解析，得到新的 `CCircuit*`。

参数：

- `path` (`const char*`)：文件路径，NUL 结尾。

返回：成功返回新建的 `CCircuit*`；`path` 为 NULL、不是合法 UTF-8、读取失败或解析失败时返回 NULL。

---

## 导出

### qcis_dumps(circuit)

把线路导出为 QCIS 字符串。

参数：

- `circuit` (`const struct CCircuit*`)：待导出的线路。

返回：成功返回堆上 C 字符串（用 `cqlib_string_free` 释放）；`circuit` 为 NULL 或导出失败时返回 NULL。

导出失败的原因包括：线路中含有 QCIS 原生门集之外的门（标准恒等门、全局相位、多控制门、自定义门与酉门）、经典存储、经典控制流、无法求值的符号参数，或延迟刻度不是非负整数。测量（`M`）与屏障（`B`）可以导出。

### qcis_dump(circuit, path)

把线路导出为 QCIS 字符串并写入文件。

参数：

- `circuit` (`const struct CCircuit*`)：待导出的线路。
- `path` (`const char*`)：目标文件路径。

返回：0 成功；-1 任一指针为 NULL；-8 路径不是合法 UTF-8；-5 导出或写入失败。

---

## QCIS 指令集

QCIS 解析与导出仅支持原生门集：

- 单比特门：`H`, `S`, `SD`, `T`, `TD`, `X`, `X2P`, `X2M`, `Y`, `Y2P`, `Y2M`, `Z`
- 含参单比特门：`RX`, `RXY`, `RY`, `RZ`, `U`, `XY`, `XY2P`, `XY2M`, `PHASE`
- 多比特门：`RXX`, `RYY`, `RZX`, `RZZ`, `SWAP`, `CX`, `CCX`, `CY`, `CZ`, `CRX`, `CRY`, `CRZ`, `FSIM`

除门之外，QCIS 文本还表示测量（`M`）、屏障（`B`）与延迟（`I Qn t`，`t` 为延迟刻度数，单位 0.5 ns，必须是非负整数）。

解析时 `SDG`、`TDG`、`Barrier` 分别作为 `SD`、`TD`、`B` 的等价写法接受；导出时统一写成 `SD`、`TD`、`B`。标准恒等门与全局相位不在原生门集内，QCIS 的 `I` 一律表示延迟。遇到无法表达的门时，应先分解到 QCIS 门集后再导出。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

/* 解析：比特写作 Q<id>，参数支持表达式 */
const char *src =
    "RX Q0 1.0\n"
    "RY Q1 pi/2\n"
    "X Q0\n"
    "CZ Q0 Q1\n"
    "M Q0 Q1\n";

struct CCircuit *c = qcis_loads(src);
if (c == NULL) {
    /* 解析失败 */
}

uintptr_t n = circuit_num_qubits(c);  /* 2 */

/* 回写 */
char *text = qcis_dumps(c);
if (text != NULL) {
    printf("%s", text);
    cqlib_string_free(text);
}

/* 写入文件 */
int32_t rc = qcis_dump(c, "out.qcis");  /* 0 成功，-5 写入失败 */

circuit_free(c);
```
