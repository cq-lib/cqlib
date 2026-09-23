# OpenQASM 3.0 API

`cqlib_core::ir::qasm3` 提供 OpenQASM 3.0 的导入与导出接口，用于在 `Circuit` 与 OpenQASM 3.0 文本/文件之间转换。

## 导入

```rust
use cqlib_core::ir::{qasm3_load, qasm3_loads, qasm3_dump, qasm3_dumps};
```

## 接口一览

- `qasm3_load(path: impl AsRef<Path>) -> Result<Circuit, Qasm3ParseError>`
- `qasm3_loads(source: &str) -> Result<Circuit, Qasm3ParseError>`
- `qasm3_dump(circuit: &Circuit, path: impl AsRef<Path>) -> Result<(), Qasm3DumpError>`
- `qasm3_dumps(circuit: &Circuit) -> Result<String, Qasm3DumpError>`

`qasm3` 模块内另提供行为一致的别名：`from_path`、`from_str`、`to_path`、`to_string`。

## 解析（load / loads）

### `qasm3_loads`

从 OpenQASM 3.0 字符串解析电路。

### `qasm3_load`

从文件读取并解析电路。相对路径的 `include` 从输入文件所在目录解析。

两者失败时返回 `Qasm3ParseError`，常见变体：

- `IoError`
- `ParseError`
- `SemanticError`
- `ConversionError`
- `UnsupportedFeature`
- `UndefinedSymbol`
- `UndefinedGate`
- `TypeError`
- `InvalidArgument`
- `MismatchedQubitCount`
- `MismatchedParameterCount`
- `RecursionLimitExceeded`
- `CircularGateDependency`

其中 `IoError` 保留底层 I/O 错误，可通过 `source()` 读取。

## 导出（dump / dumps）

### `qasm3_dumps`

将电路导出为 OpenQASM 3.0 字符串。输出依次为 `OPENQASM 3.0;`、`include "stdgates.inc";`、扩展门定义、自定义门定义、量子与经典声明、主线路操作。

### `qasm3_dump`

将导出结果写入文件，返回 `Result<(), Qasm3DumpError>`。

两者失败时返回 `Qasm3DumpError`，常见变体：

- `IoError`：文件读写失败
- `FormatError`：格式化失败
- `UnsupportedInstruction`：无法表示的指令，如 `delay`、`XY`、`RXY`
- `UnsupportedClassicalData`：无法表示的经典存储
- `UnsupportedClassicalControl`：无法表示的控制流，如 `while`、`for`、`break` / `continue`
- `MeasureInGateNotAllowed`：门定义体内包含测量
- `ConflictingGateDefinition`：同名门定义的签名冲突

## 支持能力（概览）

根据当前实现，OpenQASM 3.0 导入导出支持：

- 头部 `OPENQASM 3;` 与 `OPENQASM 3.0;`，以及 `include "stdgates.inc";`（使用内置标准库，不要求磁盘文件）
- 标量量子比特与一维量子寄存器声明
- 经典声明 `bit`、`bit[n]`、`bool`、`uint[n]`，以及角度与浮点输入声明
- 映射到 `StandardGate` 的标准门与 Cqlib 扩展门
- 自定义门定义与调用
- 测量、`reset`、`barrier`、全局相位
- 控制流 `if` / `else`、静态 `for`、常量分支 `switch`

## 与 OpenQASM 2.0 接口的差异

在同一 `Circuit` 表示之上，本接口与 OpenQASM 2.0 接口的主要差异：

| 方面 | OpenQASM 2.0 接口 | OpenQASM 3.0 接口 |
|---|---|---|
| 头部与标准库 | `OPENQASM 2.0;`、`include "qelib1.inc";` | `OPENQASM 3;` 或 `OPENQASM 3.0;`、`include "stdgates.inc";` |
| 量子声明 | `qreg q[2];` | `qubit[2] q;`，并支持标量 `qubit q;` |
| 经典声明 | `creg c[2];` | `bit`、`bit[n]`、`bool`、`uint[n]` |
| 测量 | `measure q[0] -> c[0];` | `c[0] = measure q[0];`、`c = measure q;` |
| 控制流 | `if (c == 1) ...` 后接单条语句 | `if` / `else` 块、静态 `for`、常量分支 `switch` |
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

## 最小示例

```rust
use cqlib_core::ir::{qasm3_dumps, qasm3_loads};

let qasm = r#"
OPENQASM 3;
include "stdgates.inc";
qubit[2] q;
h q[0];
cx q[0], q[1];
"#;

let c = qasm3_loads(qasm).unwrap();
let out = qasm3_dumps(&c).unwrap();
assert!(out.contains("OPENQASM 3.0;"));
```

扩展门定义与往返：

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::ir::{qasm3_dumps, qasm3_loads};

let q0 = Qubit::new(0);
let q1 = Qubit::new(1);
let mut circuit = Circuit::new(2);
circuit.x2p(q0).unwrap();
circuit.rzz(q0, q1, 0.25).unwrap();

let qasm = qasm3_dumps(&circuit).unwrap();
assert!(qasm.contains("gate x2p q { rx(pi/2) q; }"));

let round_trip = qasm3_loads(&qasm).unwrap();
assert_eq!(round_trip.operations().len(), 2);
```
