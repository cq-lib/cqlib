# 中间表达

`cqlib.ir`

`cqlib.ir` 是 Cqlib 的中间表达模块，提供 `Circuit` 与三种线路文本格式之间的双向转换：QCIS、OpenQASM 2.0 与 OpenQASM 3.0。每种格式是一个子模块，接口形态完全一致。

## Overview

中间表达解决的是线路的文本化问题：线路在文件、工具与服务之间传递时是文本，而 Cqlib 内部统一用 `Circuit` 表示。`cqlib.ir` 的每个子模块都提供「文本进、`Circuit` 出」与「`Circuit` 进、文本出」两个方向，转换过程不修改传入的 `Circuit`。

### 统一接口：解析与导出

三个子模块使用同一套四个函数：

- 解析方向：`loads(text)` 从字符串解析，`load(path)` 从文件读取后解析，返回 `Circuit`。
- 导出方向：`dumps(circuit)` 返回文本字符串，`dump(circuit, path)` 把文本写入文件并返回 `None`。

`loads` / `dumps` 不接触文件系统，`load` / `dump` 则要读写文件，因此「文件读写失败」与「内容解析失败」是两类不同的异常。

### 三个格式

| 格式 | 子模块 | 文本特点 |
| --- | --- | --- |
| QCIS | `cqlib.ir.qcis` | 由操作行组成，导出只接受 QCIS 原生门集。 |
| OpenQASM 2.0 | `cqlib.ir.qasm2` | 带 `OPENQASM 2.0;` 与 `include "qelib1.inc";` 头部及 `qreg` 声明，支持自定义门。 |
| OpenQASM 3.0 | `cqlib.ir.qasm3` | 见 [qasm3](3_qasm3.md)。 |

### 双向转换与边界

`loads(dumps(circuit))` 得到的线路与输入线路在操作序列上等价，这就是双向转换。但文本格式的表达能力有限：

- QCIS 只接受原生门集内的门，线路中若含其他门，需先分解到该门集再导出。
- QCIS 文本不记录比特数，只有比特而没有操作的线路导出为空串。

### 与编译的关系

导出是编译产物的可选落盘形式：线路降到目标门集之后即可用对应格式写出。格式差异只体现在门集与文本头上，`Circuit` 一侧的接口保持不变，因此也可以由一种格式读入、另一种格式写出，完成格式转换。

---

## 常用入口

```python
from cqlib import Circuit
from cqlib.ir import qcis

c1 = Circuit(2)
c1.h(0)
c1.cz(0, 1)

qcis_text = qcis.dumps(c1)
c2 = qcis.loads(qcis_text)

assert c2.num_qubits == 2
assert len(c2) == 2
```

---

## 核心概念与术语

| 术语 | 说明 |
| --- | --- |
| **中间表达** | `Circuit` 与线路文本格式之间的表示层，由 `cqlib.ir` 下的格式子模块提供。 |
| **解析** | 文本到 `Circuit` 的方向，对应 `loads`（字符串）与 `load`（文件）。 |
| **导出** | `Circuit` 到文本的方向，对应 `dumps`（字符串）与 `dump`（文件）。 |
| **双向转换** | 先导出再解析回到等价线路，即 `loads(dumps(circuit))`。 |
| **格式转换** | 一种格式读入、另一种格式写出，例如把 OpenQASM 2.0 文本写成 QCIS 文本。 |
| **原生门集** | 文本格式自身定义的门集合；导出时线路中的门必须都能落在该集合内。 |

---

## `cqlib.ir` API 概览

### 解析入口

| 名字 | 简介 |
| --- | --- |
| [`qcis.loads`](1_qcis.md) / [`qcis.load`](1_qcis.md) | 从 QCIS 字符串或文件解析 `Circuit`。 |
| [`qasm2.loads`](2_qasm2.md) / [`qasm2.load`](2_qasm2.md) | 从 OpenQASM 2.0 字符串或文件解析 `Circuit`。 |
| [`qasm3.loads`](3_qasm3.md) / [`qasm3.load`](3_qasm3.md) | 从 OpenQASM 3.0 字符串或文件解析 `Circuit`。 |

### 导出入口

| 名字 | 简介 |
| --- | --- |
| [`qcis.dumps`](1_qcis.md) / [`qcis.dump`](1_qcis.md) | 把 `Circuit` 导出为 QCIS 字符串或文件。 |
| [`qasm2.dumps`](2_qasm2.md) / [`qasm2.dump`](2_qasm2.md) | 把 `Circuit` 导出为 OpenQASM 2.0 字符串或文件。 |
| [`qasm3.dumps`](3_qasm3.md) / [`qasm3.dump`](3_qasm3.md) | 把 `Circuit` 导出为 OpenQASM 3.0 字符串或文件。 |

### 格式子模块

| 名字 | 简介 |
| --- | --- |
| [`cqlib.ir.qcis`](1_qcis.md) | QCIS 文本格式，导出受 QCIS 原生门集约束。 |
| [`cqlib.ir.qasm2`](2_qasm2.md) | OpenQASM 2.0 文本格式，支持标准门、参数表达式、测量与自定义门。 |
| [`cqlib.ir.qasm3`](3_qasm3.md) | OpenQASM 3.0 文本格式。 |

---

## 快速示例

### 1. QCIS 文本的双向转换

```python
from cqlib.circuit import Circuit
from cqlib.ir.qcis import dumps, loads

c1 = Circuit(2)
c1.h(0)
c1.cz(0, 1)

qcis = dumps(c1)
c2 = loads(qcis)

assert c2.num_qubits == 2
assert len(c2) == 2
```

导出的文本是逐行的操作，可以对内容做精确断言：

```python
from cqlib.circuit import Circuit
from cqlib.ir.qcis import dumps

c = Circuit(2)
c.cx(0, 1)

assert dumps(c) == "CX Q0 Q1\n"
```

### 2. OpenQASM 2.0 文本的解析与导出

```python
from cqlib.ir import qasm2

code = """OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
h q[0];
"""

circuit = qasm2.loads(code)

text = qasm2.dumps(circuit)
lines = text.strip().split("\n")
assert lines[1] == "OPENQASM 2.0;"
assert lines[2] == 'include "qelib1.inc";'
assert "h q[0];" in text
```

### 3. QCIS 文件读写

```python
import os
import tempfile

from cqlib.circuit import Circuit
from cqlib.ir import qcis

c = Circuit(2)
c.h(0)
c.cz(0, 1)

with tempfile.NamedTemporaryFile(mode="w", suffix=".qcis", delete=False) as f:
    path = f.name

try:
    qcis.dump(c, path)

    with open(path) as f:
        assert "H Q0" in f.read()

    c2 = qcis.load(path)
    assert c2.num_qubits == 2
finally:
    os.unlink(path)
```

---

## 校验与错误处理

内容问题抛 `ValueError`，文件问题抛 `OSError`（`IOError` 是它的别名），两者的消息前缀区分了格式与动作。

| 异常 | 触发场景 |
| --- | --- |
| `ValueError` | 文本语法不合法或解析失败，消息前缀为 `QCIS parse error:`、`QASM parse error:`、`QASM3 parse error:`。 |
| `ValueError` | 导出失败，例如线路中含目标格式无法表达的门，消息前缀为 `QCIS dump error:`、`QASM dump error:`、`QASM3 dump error:`。 |
| `OSError` | `load` 读取文件失败，消息前缀为 `<格式> load error:`。 |
| `OSError` | `dump` 写入文件失败，消息前缀为 `<格式> dump error:`。 |

使用 `load` / `dump` 时，同一个前缀既可能来自文件读写，也可能来自内容处理；需要区分时按异常类型判断即可。导出前线路必须已经落在目标格式的门集内，否则先做门分解再导出。
