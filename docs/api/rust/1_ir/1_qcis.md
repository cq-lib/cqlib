# QCIS API

`cqlib_core::ir` 提供 QCIS 的导入与导出接口，用于在 `Circuit` 与 QCIS 文本/文件之间转换。

## 导入

```rust
use cqlib_core::ir::{qcis_load, qcis_loads, qcis_dump, qcis_dumps};
```

## 接口一览

- `qcis_load(path: PathBuf) -> Circuit`
- `qcis_loads(qcis: &str) -> Result<Circuit, QcisParseError>`
- `qcis_dump(circuit: &Circuit, path: &PathBuf) -> Result<(), QcisDumpError>`
- `qcis_dumps(circuit: &Circuit) -> Result<String, QcisDumpError>`

## 解析（load / loads）

### `qcis_loads`

从 QCIS 字符串解析 `Circuit`，失败时返回 `QcisParseError`。

常见错误类型：

- `InvalidQubitFormat`
- `InvalidQubitId`
- `QubitCountMismatch`
- `ParameterCountMismatch`
- `MissingParameter`
- `InvalidParameter`
- `UnknownGate`

### `qcis_load`

从文件读取并解析。当前实现内部使用 `expect`，文件读取/解析失败会 panic；稳健调用建议优先使用 `qcis_loads` 自行处理错误。

## 导出（dump / dumps）

### `qcis_dumps`

将电路导出为 QCIS 字符串，失败返回 `QcisDumpError`。

常见错误类型：

- `UnsupportedGate`：包含非 QCIS 原生门
- `SymbolicParameter`：参数不能解析为数值
- `IoError`

### `qcis_dump`

导出并写入文件，返回 `Result<(), QcisDumpError>`。

## QCIS 原生门限制

QCIS 导出仅支持原生门集。错误信息中当前列出的门包括：

- `X2P`, `X2M`, `Y2P`, `Y2M`, `XY2P`, `XY2M`
- `CZ`, `RZ`, `I`, `X`, `Y`, `Z`, `H`, `S`, `SD`, `T`, `TD`
- `RX`, `RY`, `RXY`

遇到非原生门时，应先分解/编译到 QCIS 门集后再导出。

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
