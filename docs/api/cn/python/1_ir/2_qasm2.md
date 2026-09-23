# qasm2

`cqlib.ir.qasm2` 提供 OpenQASM 2.0 与 `Circuit` 的双向转换接口。

## 导入

```python
from cqlib.ir import qasm2
```

---

## 函数

### qasm2.loads(qasm)

从 OpenQASM 2.0 字符串解析电路。

参数：

- `qasm` (`str`)：QASM 文本。

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

- `OSError`：文件读取失败（`QASM load error: ...`）。
- `ValueError`：文件内容解析失败（`QASM load error: ...`）。

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

- `OSError`：写文件失败（`QASM dump error: ...`）。
- `ValueError`：导出失败（`QASM dump error: ...`）。

## 支持能力

当前`qasm2` 接口支持：

- 常见标准门：`h/x/y/z/s/sdg/t/tdg/id`、`cx/cy/cz/swap/ccx` 等
- 参数门：`rx/ry/rz/u1/u2/u3/p`，以及 `sx/sxdg/crx/cry/crz/rxx/ryy/rzz/fsim` 与 `ch/cu1/cu3`
- 参数表达式：如 `pi`, `pi/2`, `3*pi/4`，加载还接受 `1e-5` 与 `asin`、`acos`、`atan`
- 指令：`measure`, `barrier`, `reset`
- 声明与定义：`qreg`、`creg`、`opaque`，以及 `gate` 自定义门定义与 `if (creg == 整数) qop;` 条件语句
- 自定义门（`CircuitGate`）导出与加载；只带矩阵的酉门导出为 `opaque` 声明

`include "qelib1.inc";` 按内建引入处理，不要求本地存在该文件；qelib1 的门名在未写 `include` 时同样可用，且不能被重新定义。

## 限制

导出会按 OpenQASM 2.0 的表达能力取舍，下列结构会报错：

- 经典控制中的 `else` 分支、循环、`switch` 与嵌套控制流
- 无法表示为 `creg` 的经典类型，例如 `bool` 与无符号整数
- `gate` 定义体内的测量与复位
- 逐位条件（如 `if (c[0] == 1)`）与条件屏障

`rzx` 只出现在导出侧的扩展门定义中，加载不接受；全局相位不写成语句，只作为注释输出，重新加载时不会恢复。

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