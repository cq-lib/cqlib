# qasm3

`cqlib.ir` 下的 `qasm3` 子模块提供 OpenQASM 3.0 与 `Circuit` 的双向转换接口。

## 导入

```python
from cqlib.ir import qasm3
```

---

## 函数

### qasm3.loads(qasm_text)

从 OpenQASM 3.0 字符串解析电路。

参数：

- `qasm_text` (`str`)：QASM 文本。

返回：

- `Circuit`

异常情况：

- `ValueError`：语法错误、语义错误，或使用了当前实现不支持的特性（`QASM3 parse error: ...`）。

示例：

```python
from cqlib.ir import qasm3

code = """OPENQASM 3;
include "stdgates.inc";
qubit[2] q;
h q[0];
cx q[0], q[1];
"""

circuit = qasm3.loads(code)
```

### qasm3.load(path)

从 QASM 文件读取并解析电路。

参数：

- `path` (`str`)：文件路径。

返回：

- `Circuit`

异常情况：

- `OSError`：文件读取失败（不存在、权限不足等）。
- `ValueError`：文件内容解析失败（`QASM3 load error: ...`）。

说明：

- `load` 读取“文件路径”，`loads` 读取“字符串内容”。
- 相对路径的 `include` 从输入文件所在目录解析。

### qasm3.dumps(circuit)

将电路导出为 OpenQASM 3.0 字符串。

参数：

- `circuit` (`Circuit`)

返回：

- `str`

异常情况：

- `ValueError`：导出失败（`QASM3 dump error: ...`）。

说明：

- 输出为规范化文本，不保留原始输入的空格、注释与变量名。
- 内容依次为头部、标准库引入、扩展门定义、自定义门定义、量子与经典声明、主线路操作。

### qasm3.dump(circuit, path)

将电路导出为 QASM 文件。

参数：

- `circuit` (`Circuit`)
- `path` (`str`)：输出路径。

返回：

- `None`

异常情况：

- `OSError`：写文件失败。
- `ValueError`：导出失败（`QASM3 dump error: ...`）。

## 支持能力

当前 `qasm3` 接口支持：

- 头部 `OPENQASM 3;` 与 `OPENQASM 3.0;`
- 标准库引入 `include "stdgates.inc";`，解析时使用内置标准库，不要求所在目录存在该文件
- 量子声明：标量 `qubit q;` 与一维寄存器 `qubit[2] q;`
- 经典声明：`bit`、`bit[n]`、`bool`、`uint[n]`
- 角度输入声明，如 `input angle[64] theta;`，作为线路的符号参数
- 映射到 `StandardGate` 的标准门：`id`、`i`、`x`、`y`、`z`、`h`、`s`、`sdg`、`t`、`tdg`、`rx`、`ry`、`rz`、`p`、`phase`、`u1`、`u`、`U`、`u3`、`u2`、`cx`、`cy`、`cz`、`swap`、`ccx`、`crx`、`cry`、`crz`，以及 `sx`、`sxdg`（分别对应 X2P、X2M）
- Cqlib 扩展门：`x2p`、`x2m`、`y2p`、`y2m`、`xy2p`、`xy2m`、`rxx`、`ryy`、`rzz`、`rzx`、`fsim`、`gphase`
- 自定义门 `gate` 的定义与调用，含带参数的自定义门
- 测量：`c = measure q;`、`v[0] = measure q[0];`，以及独立的 `measure q;`
- 指令 `reset`、`barrier`
- 全局相位 `gphase(theta);`
- 控制流：`if` / `else`、可静态展开的 `for`、常量分支的 `switch`

## 与 OpenQASM 2.0 接口的差异

在同一 `Circuit` 表示之上，本接口与 OpenQASM 2.0 接口的主要差异：

| 方面 | OpenQASM 2.0 接口 | OpenQASM 3.0 接口 |
|---|---|---|
| 头部与标准库 | `OPENQASM 2.0;`、`include "qelib1.inc";` | `OPENQASM 3;` 或 `OPENQASM 3.0;`、`include "stdgates.inc";` |
| 量子声明 | `qreg q[2];` | `qubit[2] q;`，并支持标量 `qubit q;` |
| 经典声明 | `creg c[2];` | `bit`、`bit[n]`、`bool`、`uint[n]` |
| 测量 | `measure q[0] -> c[0];` | `c[0] = measure q[0];`、`c = measure q;` |
| 控制流 | `if (c == 1) ...` 后接单条语句 | `if` / `else` 块、可静态展开的 `for`、常量分支的 `switch` |
| 全局相位 | 导出时仅以注释形式给出，不可加载 | `gphase(theta);`，可加载、可导出 |
| 输入声明 | 无 | 角度与浮点 `input` 声明 |

## 限制

以下 OpenQASM 3.0 特性在当前实现中不支持，遇到时给出明确错误，不做部分降级：

- 定时与脉冲：`delay`、`box`、`cal`、`defcal`
- 子程序与外部调用：`def` 子程序、`extern`
- 硬件相关：硬件量子比特、`output` 声明，以及除角度与浮点以外的 `input` 声明
- 其它语句：`alias`、`pragma`、注解、旧式声明
- 门修饰符：`negctrl`、`pow`；`ctrl` 与 `inv` 仅在目标门存在对应实现时可用
- 部分标准门：`ch`、`cp`、`cu`、`cswap`
- 控制流：`while` 在实现中存在下降路径，但运行时条件的循环当前会被语义检查拒绝；`for` 只支持范围为常量的静态展开形式
- 复杂经典算术、复杂左值切片、多维索引
- 作用域内的量子比特声明

导出侧另有约束：

- 不导出 `delay`、仅矩阵形式的 `UnitaryGate`、写入标量 `bit` 变量的普通赋值，以及门定义体内的测量
- 不导出 `while`、`for`、`break`、`continue` 等控制流
- 不导出 `XY`、`RXY` 两种标准门

## 往返行为

导出结果可被本接口重新加载。规范化规则：

- 头部固定为 `OPENQASM 3.0;` 与 `include "stdgates.inc";`，与输入中的头部写法无关。
- 顶层量子比特统一导出为 `qubit[n] q`，单比特情形同样写作 `qubit[1] q`。
- 用户经典变量按 `c0`、`c1`… 命名，不可变的测量值按 `v0`、`v1`… 命名。
- 测量紧接兼容的存储时折叠为赋值形式，如 `c0 = measure q;`。
- 独立测量写入自动生成的 `bit[k] meas` 寄存器，如 `meas[0] = measure q[0];`。
- 非连续或重排的寄存器测量拆分为逐位赋值，如 `c0[0] = measure q[2];`。
- `stdgates.inc` 已提供的门直接调用；扩展门在主线路前生成一次 `gate` 定义，调用处仍使用其名称。
- 全局相位以 `gphase(...);` 语句导出，重新加载后进入电路的全局相位。
- 自动生成的名称会避开电路中已有的门名与寄存器名。

示例：

```python
from cqlib.ir import qasm3

code = """OPENQASM 3;
include "stdgates.inc";
qubit[2] q;
h q[0];
cx q[0], q[1];
"""

circuit = qasm3.loads(code)
print(qasm3.dumps(circuit))
```

输出：

```text
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;

h q[0];
cx q[0],q[1];
```

## 常见流程

```python
from cqlib.ir import qasm3

code = """OPENQASM 3;
include "stdgates.inc";
qubit[2] q;
h q[0];
cx q[0], q[1];
"""

circuit = qasm3.loads(code)
print(qasm3.dumps(circuit))

qasm3.dump(circuit, "input.qasm")

# 1) 读取 QASM 文件
c = qasm3.load("input.qasm")

# 2) 处理电路（示例）
c2 = c.decompose()

# 3) 导出为字符串或文件
text = qasm3.dumps(c2)
qasm3.dump(c2, "output.qasm")
```
