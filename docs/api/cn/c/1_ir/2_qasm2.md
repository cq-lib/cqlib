# OpenQASM 2.0（C）

C 绑定提供 OpenQASM 2.0 文本与 `CCircuit` 之间的四个转换入口。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 接口一览

| 函数 | 方向 |
| --- | --- |
| `qasm2_loads` | OpenQASM 2.0 字符串 → `CCircuit` |
| `qasm2_load` | OpenQASM 2.0 文件 → `CCircuit` |
| `qasm2_dumps` | `CCircuit` → OpenQASM 2.0 字符串 |
| `qasm2_dump` | `CCircuit` → OpenQASM 2.0 文件 |

---

## 解析

### qasm2_loads(source)

从 OpenQASM 2.0 字符串解析线路，得到新的 `CCircuit*`。

参数：

- `source` (`const char*`)：OpenQASM 2.0 文本，NUL 结尾。

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；`source` 为 NULL、不是合法 UTF-8 或解析失败时返回 NULL。

解析失败的原因包括：语法错误、AST 到线路的转换失败、引用未定义的量子或经典寄存器、引用未定义的门、重定义 qelib1 声明的门名、参数个数或比特数不符、门展开超深或相互依赖成环、参数表达式求值失败、版本声明不是 2.0、重复声明、`include` 成环，以及被调用的 `opaque` 门。

`include` 引入的外部文件不会被解析；文件入口 `qasm2_load` 才按文件所在目录解析相对 `include`。

### qasm2_load(path)

从文件读取 OpenQASM 2.0 文本并解析，得到新的 `CCircuit*`。

参数：

- `path` (`const char*`)：文件路径，NUL 结尾。

返回：成功返回新建的 `CCircuit*`；`path` 为 NULL、不是合法 UTF-8、读取失败或解析失败时返回 NULL。

---

## 导出

### qasm2_dumps(circuit)

把线路导出为 OpenQASM 2.0 字符串。输出含自动生成注释、`OPENQASM 2.0;`、`include "qelib1.inc";` 头部与 `qreg`/`creg` 声明。

参数：

- `circuit` (`const struct CCircuit*`)：待导出的线路。

返回：成功返回堆上 C 字符串（用 `cqlib_string_free` 释放）；`circuit` 为 NULL 或导出失败时返回 NULL。

导出失败的原因包括：内容生成失败、`gate` 定义体内出现测量或复位、比特下标非法、无法表示为 `creg` 的经典类型、经典数据操作或经典控制无法表达，以及不同的自定义门使用了同一个 OpenQASM 门名。

### qasm2_dump(circuit, path)

把线路导出为 OpenQASM 2.0 字符串并写入文件。

参数：

- `circuit` (`const struct CCircuit*`)：待导出的线路。
- `path` (`const char*`)：目标文件路径。

返回：0 成功；-1 任一指针为 NULL；-8 路径不是合法 UTF-8；-5 导出或写入失败。

---

## 支持的 OpenQASM 2.0 子集

- 标准门与参数门，含 `ch`/`cu1`/`cu3` 与 `sx`/`sxdg`/`crx`/`cry`/`crz`/`rxx`/`ryy`/`rzz`/`fsim`
- 测量、屏障、复位等指令
- `qreg`、`creg`、`opaque` 声明，以及 `if (creg == 整数) qop;` 条件语句
- `CircuitGate` 导出为门定义；只带矩阵的酉门导出为 `opaque` 声明
- 未直接覆盖的扩展门以 `gate` 定义形式输出（如 `crx`/`cry`/`rzz`/`rxx`/`ryy`/`rzx`）
- `include "qelib1.inc";` 按内建引入处理，不要求本地存在该文件

导出会按 OpenQASM 2.0 的表达能力取舍，下列结构会失败：

- 经典控制中的 `else` 分支、循环、`switch` 与嵌套控制流
- 无法表示为 `creg` 的经典类型，例如 `bool` 与无符号整数
- `gate` 定义体内的测量与复位
- 逐位条件（如 `if (c[0] == 1)`）与条件屏障

`rzx` 只出现在导出侧的扩展门定义中，加载不接受；全局相位不写成语句，只作为注释输出，重新加载时不会恢复。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

const char *src =
    "OPENQASM 2.0;\n"
    "include \"qelib1.inc\";\n"
    "qreg q[2];\n"
    "h q[0];\n"
    "cx q[0],q[1];\n";

struct CCircuit *c = qasm2_loads(src);
if (c == NULL) {
    /* 解析失败 */
}

char *out = qasm2_dumps(c);
if (out != NULL) {
    printf("%s", out);  /* 含 "OPENQASM 2.0;" */
    cqlib_string_free(out);
}

int32_t rc = qasm2_dump(c, "out.qasm");  /* 0 成功 */

circuit_free(c);
```
