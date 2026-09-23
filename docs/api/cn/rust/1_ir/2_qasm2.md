# OpenQASM 2.0 API

`cqlib_core::ir` 提供 OpenQASM 2.0 的导入与导出接口。

## 导入

```rust
use cqlib_core::ir::{qasm2_load, qasm2_loads, qasm2_dump, qasm2_dumps};
```

## 接口一览

- `qasm2_load(path) -> Result<Circuit, QasmParseError>`
- `qasm2_loads(source: &str) -> Result<Circuit, QasmParseError>`
- `qasm2_dump(circuit: &Circuit, path) -> Result<(), QasmDumpError>`
- `qasm2_dumps(circuit: &Circuit) -> Result<String, QasmDumpError>`

文件入口的路径参数是 `P: AsRef<Path>` 泛型，`&str`、`PathBuf`、`&Path` 都可以直接传入。

### 格式模块内的入口

`cqlib_core::ir::qasm2` 下的 `load` 与 `dump` 子模块转发同名函数，并在模块级别重导出一组 Rust 风格的等价入口：

- `cqlib_core::ir::qasm2::loads` / `cqlib_core::ir::qasm2::from_str`
- `cqlib_core::ir::qasm2::load` / `cqlib_core::ir::qasm2::from_path`
- `cqlib_core::ir::qasm2::dumps` / `cqlib_core::ir::qasm2::to_string`
- `cqlib_core::ir::qasm2::dump` / `cqlib_core::ir::qasm2::to_path`

`cqlib_core::ir::qasm2::ast` 是公开的抽象语法树模块，解析产物的语句、表达式与参数引用类型定义在其中。

## 解析（load / loads）

### `qasm2_loads`

从 QASM 字符串解析电路。

### `qasm2_load`

从文件读取并解析电路。

两者失败时返回 `QasmParseError`，变体包括：

- `IoError`：文件系统或 I/O 失败。
- `ParseError`：语法错误。
- `ConversionError`：AST 到 `Circuit` 的转换失败。
- `UndefinedQubit`：引用了未定义的量子寄存器或比特。
- `UndefinedRegister`：引用了未定义的经典寄存器。
- `UndefinedGate`：引用了未定义的门。
- `ReservedGateName`：试图重定义 qelib1 声明的门名。
- `InvalidArgument`：参数格式或用法不合法。
- `MismatchedQubitCount` / `MismatchedParameterCount`：门调用的比特数或参数个数不符。
- `RecursionLimitExceeded` / `CircularGateDependency`：门展开超深或相互依赖成环。
- `EvaluationError`：参数表达式求值失败。
- `UnsupportedVersion`：版本声明不是 2.0。
- `DuplicateDeclaration`：重复声明或命名冲突。
- `IncludeCycle`：`include` 成环。
- `UnsupportedOpaqueGate`：`opaque` 门被调用，无法在 `Circuit` 中表示。

## 导出（dump / dumps）

### `qasm2_dumps`

将电路导出为 QASM 字符串，失败返回 `QasmDumpError`。输出含自动生成注释、`OPENQASM 2.0;`、`include "qelib1.inc";` 头部与 `qreg/creg` 声明。

错误类型：

- `FormatError`：内容生成失败。
- `MeasureInGateNotAllowed` / `ResetInGateNotAllowed`：`gate` 定义体内出现测量或复位。
- `InvalidQubitIndex`：比特下标非法。
- `UnsupportedClassicalType`：经典类型无法表示为 `creg`。
- `UnsupportedClassicalData` / `UnsupportedClassicalControl`：经典数据操作或经典控制无法表达。
- `ConflictingGateDefinition`：不同的自定义门使用了同一个 OpenQASM 门名。
- `IoError`：写入失败。

### `qasm2_dump`

将导出结果写入文件，返回 `Result<(), QasmDumpError>`；文件写入失败为 `IoError`。

## 支持能力（概览）

根据当前实现，QASM2 导入导出支持：

- 标准门与参数门，含 `ch/cu1/cu3` 与 `sx/sxdg/crx/cry/crz/rxx/ryy/rzz/fsim`
- 测量、屏障、复位等指令
- `qreg`、`creg`、`opaque` 声明，以及 `if (creg == 整数) qop;` 条件语句
- `CircuitGate` 导出定义；只带矩阵的酉门导出为 `opaque` 声明
- 未直接覆盖的扩展门以 `gate` 定义形式输出（如 `crx/cry/rzz/rxx/ryy/rzx`）
- `include "qelib1.inc";` 按内建引入处理，不要求本地存在该文件

## 限制

导出会按 OpenQASM 2.0 的表达能力取舍，下列结构会报错：

- 经典控制中的 `else` 分支、循环、`switch` 与嵌套控制流
- 无法表示为 `creg` 的经典类型，例如 `bool` 与无符号整数
- `gate` 定义体内的测量与复位
- 逐位条件（如 `if (c[0] == 1)`）与条件屏障

加载侧的 `loads` 不解析 `include` 引入的外部文件，`load` 才按文件所在目录解析。`rzx` 只出现在导出侧的扩展门定义中，加载不接受；全局相位不写成语句，只作为注释输出，重新加载时不会恢复。

## 最小示例

```rust
use cqlib_core::ir::{qasm2_dumps, qasm2_loads};

let qasm = r#"
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
cx q[0],q[1];
"#;

let c = qasm2_loads(qasm).unwrap();
let out = qasm2_dumps(&c).unwrap();
assert!(out.contains("OPENQASM 2.0;"));
```
