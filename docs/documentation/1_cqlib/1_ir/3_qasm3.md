# OpenQASM 3.0 支持

OpenQASM 3.0 面向现代量子程序设计，比 OpenQASM 2.0 更适合表达测量赋值、经典变量、控制流和更完整的动态线路语义。Cqlib 的 `qasm3` 模块负责在 OpenQASM 3.0 文本和 Cqlib `Circuit` 之间双向转换。

对应 Python 模块：

```python
from cqlib.ir import qasm3
```

## 1. API 总览

| 函数 | 用途 | 示例 |
|---|---|---|
| `qasm3.loads(text)` | 从 OpenQASM 3.0 字符串解析 `Circuit` | `circuit = qasm3.loads(qasm_text)` |
| `qasm3.load(path)` | 从 OpenQASM 3.0 文件解析 `Circuit` | `circuit = qasm3.load("input.qasm")` |
| `qasm3.dumps(circuit)` | 将 `Circuit` 导出为 OpenQASM 3.0 字符串 | `text = qasm3.dumps(circuit)` |
| `qasm3.dump(circuit, path)` | 将 `Circuit` 写入 OpenQASM 3.0 文件 | `qasm3.dump(circuit, "output.qasm")` |

## 2. 最小 OpenQASM 3.0 程序

```text
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;

h q[0];
cx q[0],q[1];
```

加载到 Cqlib：

```python
from cqlib.ir import qasm3

qasm_code = """
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;

h q[0];
cx q[0],q[1];
"""

circuit = qasm3.loads(qasm_code)
print(circuit.num_qubits)
```

`OPENQASM 3;` 和 `OPENQASM 3.0;` 都可以被加载。

## 3. 从 Cqlib 导出 OpenQASM 3.0

```python
from cqlib import Circuit
from cqlib.ir import qasm3

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

text = qasm3.dumps(circuit)
print(text)
```

典型输出：

```text
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;

h q[0];
cx q[0],q[1];
```

导出器会生成规范化文本，不保留原始输入的空格、注释和变量名。

## 4. 文件读写

```python
from cqlib.ir import qasm3

circuit = qasm3.load("input.qasm")
qasm3.dump(circuit, "output.qasm")
```

文件读取失败会抛出 I/O 异常；语法错误、语义错误或 unsupported feature 会抛出 `ValueError`。

## 5. `stdgates.inc`

OpenQASM 3.0 标准门通常通过：

```text
include "stdgates.inc";
```

引入。Cqlib 依赖 OpenQASM 3 前端识别标准库门。导出时也会写出 `include "stdgates.inc";`，方便外部 OpenQASM 3 工具读取。

对于 Cqlib 有、但 `stdgates.inc` 不一定直接提供的扩展门，导出器会在主线路前生成 gate definition。例如 `x2p`、`rxx`、`rzz`、`rzx`、`fsim` 等可能被写成自定义 gate。

## 6. 测量赋值

OpenQASM 3.0 的测量比 OpenQASM 2.0 更自然，因为它支持赋值表达式：

```text
bit c;
c = measure q[0];
```

多比特测量：

```text
bit[2] c;
c = measure q;
```

部分 bit-vector 赋值：

```text
bit[2] c;
c[0] = measure q[2];
c[1] = measure q[0];
```

Cqlib 当前支持上述测量写法，并在导出时尽量生成可读回的规范形式。

## 7. Cqlib 测量到 QASM3 的映射规则

Cqlib 内部把测量分为两层：

- `ClassicalValue`：测量产生的不可变结果。
- `ClassicalVar`：用户创建的可变经典变量。

导出到 QASM3 时会按以下规则处理：

| Cqlib 操作 | QASM3 输出 | 说明 |
|---|---|---|
| `measure_into(q, bit)` | `c0 = measure q[0];` | 单比特测量写入用户变量 |
| `measure_bits_into([0,1], bitvec)` | `c0 = measure q;` | 全寄存器顺序一致时合并输出 |
| `measure_bits_into([2,0], bitvec)` | `c0[0] = measure q[2];` 和 `c0[1] = measure q[0];` | 非连续或重排测量时拆成 indexed assignment |
| `measure(q)` | `bit[n] meas; meas[i] = measure q[j];` | 裸测量会生成显式寄存器，保证 QASM3 可读回 |

示例：

```python
from cqlib import Circuit
from cqlib.ir import qasm3

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
circuit.measure(0)
circuit.measure(1)

print(qasm3.dumps(circuit))
```

输出：

```text
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;
bit[2] meas;

h q[0];
cx q[0],q[1];
meas[0] = measure q[0];
meas[1] = measure q[1];
```

这样做的原因是：裸测量在 Cqlib 内部可以只产生临时值，但 QASM3 文本如果要稳定地跨工具保存和读回，最好有显式 classical destination。当前实现会生成 `meas` 寄存器，而不是泄漏内部临时名 `v0/v1`。

## 8. 标量测量赋值兼容

OpenQASM 3.0 中常见的写法也可以加载：

```text
OPENQASM 3.0;
qubit[1] q;
bit v;
v = measure q[0];
```

以及：

```text
OPENQASM 3.0;
qubit q;
bit[2] c;
c[0] = measure q;
```

Cqlib 会把它们降低为内部的测量操作加 classical store 操作。

## 9. reset、barrier 和 global phase

示例：

```text
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;
reset q[0];
barrier q[0],q[1];
gphase(0.25);
```

Cqlib 支持将这些语句加载为对应的 `Circuit` 操作。导出时也会尽量保持等价语义。

## 10. 自定义门

OpenQASM 3.0 自定义门示例：

```text
OPENQASM 3.0;
include "stdgates.inc";

gate bell a, b {
    h a;
    cx a, b;
}

qubit[2] q;
bell q[0], q[1];
```

Cqlib 加载后会把 `bell` 视为线路定义门。导出时，如果 `Circuit` 中有可表示为 QASM3 gate body 的 `CircuitGate`，也会输出对应定义。

## 11. 控制流支持范围

Cqlib 的 QASM3 loader 支持部分可映射到当前 `Circuit` 的控制流：

| OpenQASM 3 特性 | 当前支持情况 | 说明 |
|---|---|---|
| `if/else` | 支持 | 条件需能转换为 Cqlib classical expression |
| 静态 `for` | 支持 | 例如固定范围 `[0:2]`，可静态展开 |
| `switch` | 支持部分精确值 case | 适用于简单 unsigned integer case |
| `while` | 受前端与 IR 表达能力限制 | 复杂运行时循环可能被拒绝 |
| `break/continue` | 受限 | 取决于是否能映射到当前控制流模型 |

推荐原则：如果线路主要用于跨框架交换，尽量使用简单、静态、可展开的控制流；复杂动态程序应先验证目标后端是否支持。

## 12. 当前支持与限制

| 类型 | 支持情况 |
|---|---|
| `qubit`、`qubit[n]` | 支持 |
| `bit`、`bit[n]`、`bool`、`uint[n]` | 支持常用形式 |
| 标准门 | 支持可映射到 Cqlib `StandardGate` 的门 |
| Cqlib 扩展门 | 导出时生成 gate definition |
| 测量赋值 | 支持 scalar、bit-vector、indexed assignment 的常用形式 |
| `reset`、`barrier`、`gphase` | 支持 |
| 自定义 gate | 支持可映射的 gate body |
| calibration、pulse、extern、hardware qubit、alias | 当前不支持 |
| 任意复杂 classical arithmetic | 当前不支持无损 lowering |
| 复杂 lvalue slicing、多维索引 | 当前不支持或仅支持必要子集 |

## 13. 常见错误排查

| 现象 | 常见原因 | 处理方式 |
|---|---|---|
| `QASM3 parse error` | 语法错误、前端语义检查失败、使用了 Cqlib 不支持的特性 | 先用最小 QASM3 程序验证，再逐步加入特性 |
| `unsupported feature` | 使用了 calibration、extern、复杂 lvalue 或 unsupported gate modifier | 改写为基础门，或先在外部工具中展开为基础线路 |
| 导出失败 | `Circuit` 中包含 QASM3 无法表示的 Delay、Unitary 或复杂 store | 先 `decompose()` 或改用更合适的格式 |
| 外部工具无法读取 Cqlib 导出的 QASM3 | 外部工具暂不支持某些扩展门或 OpenQASM 3 子集 | 先分解扩展门，或改用目标工具支持的语法子集 |
| 测量寄存器名变成 `meas` | Cqlib 为裸测量生成显式目标寄存器 | 正常行为，用于保证文本可读回 |

## 下一步

- [格式转换工作流](4_conversion_workflow.md)
