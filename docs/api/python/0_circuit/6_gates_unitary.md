# UnitaryGate

`cqlib.circuit.gates.UnitaryGate`  

```python
from cqlib.circuit.gates import UnitaryGate
```

`UnitaryGate` 用于定义 Cqlib 标准门集合之外的用户自定义酉门。与 `StandardGate` 不同，`UnitaryGate` 的行为通常由用户提供的矩阵、符号矩阵或冻结线路来描述，因此更适合表示算法中的特殊 oracle、硬件相关校准门、问题相关黑盒变换，或其他无法直接用内置标准门表达的自定义量子操作。

---

## 构造函数

```python
UnitaryGate(label: str, num_qubits: int, num_params: int = 0)
```

| 参数 | 说明 |
| --- | --- |
| `label` | 门的可读名称，常用于打印、调试、导出和可视化。 |
| `num_qubits` | 该门作用的量子比特数量。 |
| `num_params` | 每次应用该门时需要传入的位置参数数量，默认为 `0`。 |

```python
from cqlib.circuit.gates import UnitaryGate

oracle = UnitaryGate("Oracle", num_qubits=2)
```

构造函数只创建一个带有名称、作用比特数和参数个数的自定义门对象。创建后，可以通过 `with_matrix()`、`with_symbolic_matrix()` 或 `with_circuit()` 为该门附加具体定义。

---

## 数值矩阵定义

```python
with_matrix(matrix: ArrayLike) -> UnitaryGate
```

`with_matrix()` 用于通过数值矩阵定义自定义门。传入矩阵必须是二维方阵，形状应为 `2**num_qubits × 2**num_qubits`。例如，单比特门对应 `(2, 2)` 矩阵，双比特门对应 `(4, 4)` 矩阵，三比特门对应 `(8, 8)` 矩阵。

```python
import numpy as np
from cqlib import Circuit
from cqlib.circuit.gates import UnitaryGate

z_like = np.array([[1, 0], [0, -1]], dtype=np.complex128)
gate = UnitaryGate("ZLike", 1).with_matrix(z_like)

circuit = Circuit(1)
circuit.append_unitary_gate(gate, [0])
```

---

## 符号矩阵定义

```python
with_symbolic_matrix(matrix: SymbolicMatrix, params: list[str]) -> UnitaryGate
```

`with_symbolic_matrix()` 用于定义带参数的自定义酉门，符号矩阵可以在矩阵元素中保留 `Parameter` 表达式，使同一个门定义能够在不同线路位置绑定不同参数值。

其中，`matrix` 是门的符号矩阵定义，`params` 用于指定该门应用时的位置参数顺序。追加门时通过 `Circuit.append_unitary_gate(..., params=[...])` 传入的参数，会按照 `params` 中给出的名称顺序绑定到符号矩阵中。

```python
from cqlib import Parameter
from cqlib.circuit import SymbolicComplex, SymbolicMatrix
from cqlib.circuit.gates import UnitaryGate

theta = Parameter("theta")
matrix = SymbolicMatrix([
    [SymbolicComplex.one(), SymbolicComplex.zero()],
    [SymbolicComplex.zero(), SymbolicComplex.exp_i(theta)],
])

phase_like = UnitaryGate("PhaseLike", 1, num_params=1).with_symbolic_matrix(
    matrix,
    ["theta"],
)
```

在上述示例中，`PhaseLike` 是一个带有一个位置参数的单比特相位门。符号矩阵中使用 `theta` 表示相位参数，而 `with_symbolic_matrix(..., ["theta"])` 表示追加该门时第一个参数会绑定到符号 `theta`。

应用该门时，可以传入数值参数，也可以传入新的 `Parameter` 表达式：

```python
from cqlib import Circuit, Parameter

circuit = Circuit(1)
circuit.append_unitary_gate(phase_like, [0], params=[Parameter("phi")])
```

这里的 `phi` 是门应用时传入的位置参数，会替换符号矩阵定义中的 `theta`。如果传入的是数值，例如 `params=[0.25]`，则表示本次应用使用具体数值相位。

---

## 冻结线路定义

```python
with_circuit(circuit: FrozenCircuit) -> UnitaryGate
```

当自定义门可以由已有线路结构描述时，可以使用 `FrozenCircuit` 作为门定义。这种方式适合在 `UnitaryGate` 形式下保留线路级结构信息，使后续在需要时仍然能够参考或展开其内部操作。

```python
from cqlib import Circuit
from cqlib.circuit.gates import FrozenCircuit, UnitaryGate

sub = Circuit(2)
sub.h(0)
sub.cx(0, 1)

frozen = FrozenCircuit(sub.qubits, sub.operations)
gate = UnitaryGate("BellPrep", 2).with_circuit(frozen)
```

如果希望将一段线路作为可复用复合门添加到其他线路中，可以使用：

```python
bell_gate = sub.to_gate("BellPrep")
```


| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `label` | `str` | 门的可读名称。 |
| `num_qubits` | `int` | 门作用的量子比特数量。 |
| `num_params` | `int` | 每次应用该门时需要传入的位置参数数量。 |
| `symbolic_matrix` | `SymbolicMatrix / None` | 符号矩阵定义；如果未使用符号矩阵定义，则通常为 `None`。 |
| `matrix_params` | `list[str] / None` | 符号矩阵中的参数名顺序。 |
| `circuit` | `FrozenCircuit / None` | 冻结线路定义；如果未使用线路定义，则通常为 `None`。 |

---

## 追加到 `Circuit`

```python
Circuit.append_unitary_gate(
    gate: UnitaryGate,
    qubits: list[int | Qubit],
    params: list[float | Parameter] | None = None,
) -> None
```

定义完成后的 `UnitaryGate` 可以通过 `Circuit.append_unitary_gate()` 追加到线路中。追加时需要提供该门作用的量子比特列表；如果该门是参数化门，还需要提供本次应用的位置参数。

追加时需要满足以下条件：

- `qubits` 的数量必须等于 `gate.num_qubits`；
- 如果 `gate.num_params > 0`，则 `params` 的长度应与 `gate.num_params` 一致；
- 作用量子比特必须已经存在于目标线路中。

---

## 定义方式对比

| 定义方式 | 是否保留结构 | 是否支持符号参数 | 典型用途 |
| --- | --- | --- | --- |
| `with_matrix()` | 否 | 否 | 固定黑盒矩阵、oracle、校准门、小规模测试门。 |
| `with_symbolic_matrix()` | 否 | 是 | 参数化黑盒矩阵、可调相位门、符号验证。 |
| `with_circuit()` | 是 | 取决于内部线路 | 需要保留分解信息，同时以 `UnitaryGate` 形式应用的场景。 |

