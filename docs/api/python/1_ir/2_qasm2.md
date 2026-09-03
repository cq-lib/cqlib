# qasm2

`cqlib.ir.qasm2` 提供 OpenQASM 2.0 与 `Circuit` 的双向转换接口。

## 导入

```python
from cqlib.ir import qasm2
```

---

## 函数

### qasm2.loads(qasm_text)

从 OpenQASM 2.0 字符串解析电路。

参数：

- `qasm_text` (`str`)：QASM 文本。

返回：

- `Circuit`

异常情况：

- `ValueError`：QASM 语法不合法或解析失败（`QASM parse error: ...`）。

示例：

```python
from cqlib.ir import qasm2

code = """OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
cz q[0],q[1];
"""

circuit = qasm2.loads(code)
```

### qasm2.load(path)

从 QASM 文件读取并解析电路。

参数：

- `path` (`str`)：文件路径。

返回：

- `Circuit`

异常情况：

- `ValueError`：加载或解析失败（`QASM load error: ...`）。

### qasm2.dumps(circuit)

将电路导出为 OpenQASM 2.0 字符串。

参数：

- `circuit` (`Circuit`)

返回：

- `str`

异常情况：

- `ValueError`：导出失败（`QASM dump error: ...`）。

说明：

- 输出包含标准头部，例如：
`OPENQASM 2.0;`、`include "qelib1.inc";`、`qreg/creg` 声明。

### qasm2.dump(circuit, path)

将电路导出为 QASM 文件。

参数：

- `circuit` (`Circuit`)
- `path` (`str`)：输出路径。

返回：

- `None`

异常情况：

- `OSError`：写文件失败。

## 支持能力

当前`qasm2` 接口支持：

- 常见标准门：`h/x/y/z/s/sdg/t/tdg/cx/cy/cz/swap/ccx` 等
- 参数门：`rx/ry/rz/u1/u2/u3` 及参数表达式（如 `pi`, `pi/2`, `3*pi/4`）
- 指令：`measure`, `barrier`, `reset`
- 自定义门（`CircuitGate`）导出与加载

## 常见流程

```python
from cqlib.ir import qasm2

code = """OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
cz q[0],q[1];
"""

circuit = qasm2.loads(code)
print(qasm2.dumps(circuit))

qasm2.dump(circuit, "input.qasm")

# 1) 读取 QASM 文件
c = qasm2.load("input.qasm")

# 2) 处理电路（示例）
c2 = c.decompose()

# 3) 导出为字符串或文件
text = qasm2.dumps(c2)
qasm2.dump(c2, "output.qasm")
```