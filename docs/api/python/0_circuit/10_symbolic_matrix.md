# SymbolicMatrix

- `cqlib.circuit.SymbolicComplex`
- `cqlib.circuit.SymbolicMatrix`

```python
from cqlib.circuit import SymbolicComplex, SymbolicMatrix
```

`SymbolicMatrix` 是 Cqlib 中用于表示符号矩阵的基础类型。它由多个 `SymbolicComplex` 元素组成，而每个 `SymbolicComplex` 的实部和虚部都可以保存 `Parameter` 表达式。通过这种设计，Cqlib 可以在矩阵元素中保留符号参数，而不是在构造矩阵时立即将其求值为普通复数。

## 核心概念

在 Cqlib 中，符号矩阵由两层结构组成：

| 类型 | 作用 |
| --- | --- |
| `Parameter` | 表示数值、符号变量或参数表达式。 |
| `SymbolicComplex` | 表示一个复数元素，实部和虚部均为 `Parameter`。 |
| `SymbolicMatrix` | 表示由 `SymbolicComplex` 组成的密集矩阵。 |

普通数值矩阵中的元素通常是 `complex`，而 `SymbolicMatrix` 中的元素可以包含符号参数。例如，矩阵元素可以是 `exp(i * theta)`、`cos(theta)` 或由多个参数组合得到的表达式。这使得同一个矩阵结构能够在不同参数取值下重复求值，也便于在编译和验证阶段保留参数关系。

## `SymbolicComplex`

`SymbolicComplex` 用于表示符号复数。它的实部和虚部均为 `Parameter`，因此既可以表示普通复数，也可以表示包含符号参数的复数表达式。

```python
SymbolicComplex(real: Parameter, imag: Parameter)
```

例如，一个符号复数可以写成：

```text
real + i * imag
```

其中 `real` 和 `imag` 都可以是常量、单个符号或更复杂的参数表达式。

### 1. 构造方法

`SymbolicComplex` 提供了一组常用静态方法，用于快速创建零、单位虚数、纯实数以及相位因子等常见符号复数。

| 静态方法 | 说明 |
| --- | --- |
| `zero()` | 创建 `0 + 0i`。 |
| `one()` | 创建 `1 + 0i`。 |
| `i()` | 创建 `0 + 1i`。 |
| `from_real(value)` | 根据实数或参数表达式创建纯实数复数。 |
| `exp_i(theta)` | 创建 `exp(i * theta)`，即 `cos(theta) + i sin(theta)`。 |

```python
from cqlib import Parameter
from cqlib.circuit import SymbolicComplex

theta = Parameter("theta")

zero = SymbolicComplex.zero()
one = SymbolicComplex.one()
imag = SymbolicComplex.i()
phase = SymbolicComplex.exp_i(theta)
```

### 2. 属性和方法

| 接口 | 返回 | 说明 |
| --- | --- | --- |
| `real` | `Parameter` | 符号复数的实部。 |
| `imag` | `Parameter` | 符号复数的虚部。 |
| `symbols` | `list[str]` | 实部和虚部中包含的自由符号名。 |
| `evaluate(bindings=None)` | `complex` | 根据参数绑定将符号复数求值为 `complex`。 |
| `simplify()` | `SymbolicComplex` | 化简实部和虚部表达式。 |
| `replace(symbol, value)` | `SymbolicComplex` | 将指定符号替换为新的参数表达式。 |
| `is_zero_exact()` | `bool` | 判断是否在语法上精确等于 `0`。 |
| `is_one_exact()` | `bool` | 判断是否在语法上精确等于 `1`。 |
| `simplifies_to_zero()` | `bool` | 判断化简后是否等于 `0`。 |

```python
from cqlib import Parameter
from cqlib.circuit import SymbolicComplex

theta = Parameter("theta")
value = SymbolicComplex.exp_i(theta)

numeric = value.evaluate({"theta": 0.0})
print(numeric)  # (1+0j)
```

## `SymbolicMatrix`

`SymbolicMatrix` 用于表示由 `SymbolicComplex` 元素组成的密集矩阵。矩阵按行传入，每一行都应具有相同长度。

```python
SymbolicMatrix(rows: list[list[SymbolicComplex]])
```

下面的示例构造了一个简单的 `2 × 2` 符号矩阵：

```python
from cqlib.circuit import SymbolicComplex, SymbolicMatrix

i = SymbolicComplex.i()
zero = SymbolicComplex.zero()

matrix = SymbolicMatrix([
    [zero, i],
    [i, zero],
])
```

### 1. 属性和方法

| 接口 | 返回 | 说明 |
| --- | --- | --- |
| `shape` | `tuple[int, int]` | 矩阵形状，格式为 `(rows, cols)`。 |
| `symbols` | `list[str]` | 矩阵所有元素中包含的自由符号名。 |
| `evaluate(bindings=None)` | `numpy.ndarray` | 将符号矩阵求值为数值复数矩阵。 |
| `simplify()` | `SymbolicMatrix` | 化简矩阵中的所有元素。 |
| `substitute(replacements)` | `SymbolicMatrix` | 同时替换矩阵中的多个符号。 |
| `rows()` | `list[list[SymbolicComplex]]` | 返回矩阵的嵌套行表示。 |
| `__getitem__((row, col))` | `SymbolicComplex` | 读取指定位置元素，支持负索引。 |
| `__len__()` | `int` | 返回矩阵行数。 |

```python
import numpy as np
from cqlib import Parameter
from cqlib.circuit import SymbolicComplex, SymbolicMatrix

theta = Parameter("theta")

m = SymbolicMatrix([
    [SymbolicComplex.one(), SymbolicComplex.zero()],
    [SymbolicComplex.zero(), SymbolicComplex.exp_i(theta)],
])

numeric = m.evaluate({"theta": np.pi})
print(numeric.shape)  # (2, 2)
```

### 2. 化简、替换与求值

`SymbolicMatrix` 支持对矩阵元素中的参数表达式进行化简、符号替换和数值求值。这些操作不会改变原矩阵对象，而是返回新的符号矩阵或数值矩阵结果。

```python
from cqlib import Parameter
from cqlib.circuit import SymbolicComplex, SymbolicMatrix

theta = Parameter("theta")
phi = Parameter("phi")

m = SymbolicMatrix([
    [SymbolicComplex.one(), SymbolicComplex.zero()],
    [SymbolicComplex.zero(), SymbolicComplex.exp_i(theta)],
])

renamed = m.substitute({"theta": phi})
simplified = renamed.simplify()
numeric = simplified.evaluate({"phi": 0.5})
```

### 3. 创建参数化自定义门

`SymbolicMatrix` 的一个典型用途是定义参数化自定义酉门。通过 `UnitaryGate.with_symbolic_matrix()`，用户可以将一个符号矩阵注册为自定义门的矩阵定义，并指定应用该门时的参数顺序。

```python
from cqlib import Circuit, Parameter
from cqlib.circuit import SymbolicComplex, SymbolicMatrix
from cqlib.circuit.gates import UnitaryGate

theta = Parameter("theta")

symbolic = SymbolicMatrix([
    [SymbolicComplex.one(), SymbolicComplex.zero()],
    [SymbolicComplex.zero(), SymbolicComplex.exp_i(theta)],
])

gate = UnitaryGate("DiagPhase", 1, num_params=1).with_symbolic_matrix(
    symbolic,
    ["theta"],
)

circuit = Circuit(1)
circuit.append_unitary_gate(gate, [0], params=[Parameter("phi")])
```

在上述示例中，`with_symbolic_matrix(symbolic, ["theta"])` 表示该自定义门具有一个位置参数，对应符号矩阵中的 `theta`。当通过 `append_unitary_gate()` 将该门追加到线路中时，传入的 `params=[Parameter("phi")]` 会按照参数顺序将门定义中的 `theta` 替换为外部参数 `phi`。

### 4. 从线路获得符号矩阵

除手动构造符号矩阵外，也可以从参数化线路中生成符号矩阵。`Circuit.to_symbolic_matrix(qubits_order=None)` 会将线路转换为保留参数表达式的矩阵表示。

```python
Circuit.to_symbolic_matrix(qubits_order=None) -> SymbolicMatrix
```

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(1)
circuit.rx(0, theta)

matrix = circuit.to_symbolic_matrix()
print(matrix.symbols)
```

当线路中包含符号参数时，`to_symbolic_matrix()` 可以保留这些参数结构，适合用于小规模参数化线路验证、门定义检查和编译规则测试。

### 5. 复杂度与适用边界

对于 `n` 个量子比特的线路，完整矩阵大小为 `2^n × 2^n`，矩阵元素数量为 `4^n`。当矩阵元素中还包含符号表达式时，表达式复杂度也可能随门数量和参数数量增长。
