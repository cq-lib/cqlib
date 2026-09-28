# OpenQASM 3.0（C）

C 绑定提供 OpenQASM 3.0 文本与 `CCircuit` 之间的四个转换入口。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 接口一览

| 函数 | 方向 |
| --- | --- |
| `qasm3_loads` | OpenQASM 3.0 字符串 → `CCircuit` |
| `qasm3_load` | OpenQASM 3.0 文件 → `CCircuit` |
| `qasm3_dumps` | `CCircuit` → OpenQASM 3.0 字符串 |
| `qasm3_dump` | `CCircuit` → OpenQASM 3.0 文件 |

---

## 解析

### qasm3_loads(source)

从 OpenQASM 3.0 字符串解析线路，得到新的 `CCircuit*`。

参数：

- `source` (`const char*`)：OpenQASM 3.0 文本，NUL 结尾。

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；`source` 为 NULL、不是合法 UTF-8 或解析失败时返回 NULL。

解析失败的原因包括：语法错误、语义错误、转换失败、不支持的特性、未定义的符号或门、类型错误、参数个数或比特数不符、门展开超深或相互依赖成环。

### qasm3_load(path)

从文件读取 OpenQASM 3.0 文本并解析，得到新的 `CCircuit*`。相对路径的 `include` 从输入文件所在目录解析。

参数：

- `path` (`const char*`)：文件路径，NUL 结尾。

返回：成功返回新建的 `CCircuit*`；`path` 为 NULL、不是合法 UTF-8、读取失败或解析失败时返回 NULL。

---

## 导出

### qasm3_dumps(circuit)

把线路导出为 OpenQASM 3.0 字符串。输出依次为 `OPENQASM 3.0;`、`include "stdgates.inc";`、扩展门定义、自定义门定义、量子与经典声明、主线路操作。

参数：

- `circuit` (`const struct CCircuit*`)：待导出的线路。

返回：成功返回堆上 C 字符串（用 `cqlib_string_free` 释放）；`circuit` 为 NULL 或导出失败时返回 NULL。

导出失败的原因包括：无法表示的指令（如 `delay`、`XY`、`RXY`）、无法表示的经典存储、无法表示的控制流（如 `while`、`for`、`break` / `continue`）、门定义体内包含测量，以及同名门定义的签名冲突。

### qasm3_dump(circuit, path)

把线路导出为 OpenQASM 3.0 字符串并写入文件。

参数：

- `circuit` (`const struct CCircuit*`)：待导出的线路。
- `path` (`const char*`)：目标文件路径。

返回：0 成功；-1 任一指针为 NULL；-8 路径不是合法 UTF-8；-5 导出或写入失败。

---

## 支持的 OpenQASM 3.0 子集

- 头部 `OPENQASM 3;` 与 `OPENQASM 3.0;`，以及 `include "stdgates.inc";`（使用内置标准库，不要求磁盘文件）
- 标量量子比特与一维量子寄存器声明
- 经典声明 `bit`、`bit[n]`、`bool`、`uint[n]`，以及角度与浮点输入声明
- 映射到标准门的标准门与扩展门
- 自定义门定义与调用
- 测量、`reset`、`barrier`、全局相位
- 控制流 `if` / `else`、静态 `for`、常量分支 `switch`

以下特性遇到时给出明确错误，不做部分降级：

- 定时与脉冲：`delay`、`box`、`cal`、`defcal`
- 子程序与外部调用：`def` 子程序、`extern`
- 硬件相关：硬件量子比特、`output` 声明，以及除角度与浮点以外的 `input` 声明
- 其它语句：`alias`、`pragma`、注解、旧式声明
- 门修饰符：`negctrl`、`pow`；`ctrl` 与 `inv` 仅在目标门存在对应实现时可用
- 部分标准门：`ch`、`cp`、`cu`、`cswap`
- 控制流：运行时条件的 `while` 会被语义检查拒绝；`for` 只支持范围为常量的静态展开形式
- 复杂经典算术、复杂左值切片、多维索引，以及作用域内的量子比特声明

导出侧另有约束：不导出 `delay`、仅矩阵形式的酉门、写入标量 `bit` 变量的普通赋值、门定义体内的测量，以及 `while`、`for`、`break`、`continue` 等控制流和 `XY`、`RXY` 两种标准门。

---

## 往返行为

导出结果可被重新加载。规范化规则：

- 头部固定为 `OPENQASM 3.0;` 与 `include "stdgates.inc";`，与输入中的头部写法无关。
- 顶层量子比特统一导出为 `qubit[n] q`，单比特情形同样写作 `qubit[1] q`。
- 用户经典变量按 `c0`、`c1`… 命名，不可变的测量值按 `v0`、`v1`… 命名。
- 测量紧接兼容的存储时折叠为赋值形式，如 `c0 = measure q;`。
- 独立测量写入自动生成的 `bit[k] meas` 寄存器，如 `meas[0] = measure q[0];`。
- 非连续或重排的寄存器测量拆分为逐位赋值，如 `c0[0] = measure q[2];`。
- `stdgates.inc` 已提供的门直接调用；扩展门在主线路前生成一次 `gate` 定义，调用处仍使用其名称。
- 全局相位以 `gphase(...);` 语句导出，重新加载后进入线路的全局相位。
- 自动生成的名称会避开线路中已有的门名与寄存器名。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

const char *src =
    "OPENQASM 3;\n"
    "include \"stdgates.inc\";\n"
    "qubit[2] q;\n"
    "h q[0];\n"
    "cx q[0], q[1];\n";

struct CCircuit *c = qasm3_loads(src);
if (c == NULL) {
    /* 解析失败 */
}

char *out = qasm3_dumps(c);
if (out != NULL) {
    printf("%s", out);  /* 头部规范化为 "OPENQASM 3.0;" */
    cqlib_string_free(out);
}

int32_t rc = qasm3_dump(c, "out.qasm");  /* 0 成功 */

circuit_free(c);
```
