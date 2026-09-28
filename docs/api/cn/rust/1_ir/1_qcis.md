# QCIS API

`cqlib_core::ir` 提供 QCIS 的导入与导出接口，用于在 `Circuit` 与 QCIS 文本/文件之间转换。

## 导入

```rust
use cqlib_core::ir::{qcis_load, qcis_loads, qcis_dump, qcis_dumps};
```

## 接口一览

- `qcis_load(path) -> Result<Circuit, QcisParseError>`
- `qcis_loads(qcis: &str) -> Result<Circuit, QcisParseError>`
- `qcis_dump(circuit: &Circuit, path) -> Result<(), QcisDumpError>`
- `qcis_dumps(circuit: &Circuit) -> Result<String, QcisDumpError>`

文件入口的路径参数是 `P: AsRef<Path>` 泛型，`&str`、`PathBuf`、`&Path` 都可以直接传入。

### 格式模块内的入口

`cqlib_core::ir::qcis` 下的 `load` 与 `dump` 子模块转发同名函数，并在模块级别重导出一组 Rust 风格的等价入口：

- `cqlib_core::ir::qcis::loads` / `cqlib_core::ir::qcis::from_str`
- `cqlib_core::ir::qcis::load` / `cqlib_core::ir::qcis::from_path`
- `cqlib_core::ir::qcis::dumps` / `cqlib_core::ir::qcis::to_string`
- `cqlib_core::ir::qcis::dump` / `cqlib_core::ir::qcis::to_path`

## 解析（load / loads）

### `qcis_loads`

从 QCIS 字符串解析 `Circuit`，失败时返回 `QcisParseError`。

错误类型：

- `IoError`：文件系统或 I/O 失败，字符串入口不会产生。
- `InvalidQubitFormat`：比特写法不是 `Q<id>`。
- `InvalidQubitId`：`Q` 后的编号无法解析。
- `QubitCountMismatch`：门的比特数与规格不符。
- `ParameterCountMismatch`：门的参数个数与规格不符。
- `MissingParameter`：参数表达式为空或无法解析。
- `InvalidParameter`：延迟刻度不是固定的非负整数。
- `UnknownGate`：门名不在 QCIS 指令集内。
- `EmptyLine`：空行或没有有效内容。

### `qcis_load`

从文件读取并解析，返回 `Result<Circuit, QcisParseError>`：文件读取失败为 `IoError`，内容解析失败为对应变体。

## 导出（dump / dumps）

### `qcis_dumps`

将电路导出为 QCIS 字符串，失败返回 `QcisDumpError`。

错误类型：

- `UnsupportedGate`：线路中含 QCIS 原生门集之外的门，例如标准恒等门、全局相位、多控制门、自定义门与酉门。
- `UnsupportedClassicalData`：经典存储（`store`）无法表达。
- `UnsupportedClassicalControl`：经典控制流无法表达。
- `SymbolicParameter`：参数下标取不到对应参数，或延迟刻度无法求值。
- `InvalidDelayParameter`：延迟指令的刻度不是非负整数。
- `IoError`：写入缓冲失败。

`M`（测量）与 `B`（屏障）可以导出，`Reset` 不能。

### `qcis_dump`

导出并写入文件，返回 `Result<(), QcisDumpError>`；文件写入失败为 `IoError`。

## QCIS 原生门限制

QCIS 解析与导出仅支持原生门集：

- 单比特门：`H`, `S`, `SD`, `T`, `TD`, `X`, `X2P`, `X2M`, `Y`, `Y2P`, `Y2M`, `Z`
- 含参单比特门：`RX`, `RXY`, `RY`, `RZ`, `U`, `XY`, `XY2P`, `XY2M`, `PHASE`
- 多比特门：`RXX`, `RYY`, `RZX`, `RZZ`, `SWAP`, `CX`, `CCX`, `CY`, `CZ`, `CRX`, `CRY`, `CRZ`, `FSIM`

除门之外，QCIS 文本还表示测量（`M`）、屏障（`B`）与延迟（`I Qn t`，`t` 为延迟刻度数，单位 0.5 ns，必须是非负整数）。

解析时 `SDG`、`TDG`、`Barrier` 分别作为 `SD`、`TD`、`B` 的等价写法接受；导出时统一写成 `SD`、`TD`、`B`。标准恒等门与全局相位不在原生门集内，QCIS 的 `I` 一律表示延迟。遇到无法表达的门时，应先分解到 QCIS 门集后再导出。

## 最小示例

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::ir::{qcis_dumps, qcis_loads};

let mut c = Circuit::new(2);
c.h(Qubit::new(0)).unwrap();
c.cz(Qubit::new(0), Qubit::new(1)).unwrap();

let text = qcis_dumps(&c).unwrap();
let c2 = qcis_loads(&text).unwrap();
assert_eq!(c2.num_qubits(), 2);
```
