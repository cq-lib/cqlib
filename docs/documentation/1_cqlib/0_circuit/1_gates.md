# 量子门与指令

`cqlib.circuit` 提供了丰富的量子门与线路指令，用于构造从基础量子线路到参数化算法模块的各类量子程序。本篇将详细介绍 Cqlib 中量子门与指令的基本用法，包括标准门、参数门、多比特门、多控制门、自定义酉门、子线路门以及测量、重置、屏障等非幺正指令。

通过本篇内容，您可以根据算法需求选择合适的门类型，并将其正确添加到量子线路中。您可以通过 `Circuit` 提供的便捷方法快速添加常用门，也可以使用 `StandardGate`、`MCGate`、`UnitaryGate` 和 `CircuitGate` 等对象显式描述更复杂的门操作。

---

## 标准门

标准门是 Cqlib 内置的基础门集合，覆盖常用单比特门、多比特门、参数化旋转门以及部分硬件相关门。您可以通过以下两种方式执行标准门：

1. 直接调用 `Circuit` 提供的快捷方法；
2. 显式构造 `StandardGate` 对象，并通过 `append_gate()` 追加到线路中。

```python
from cqlib import Circuit
from cqlib.circuit import StandardGate

c = Circuit(1)
c.h(0)
c.append_gate(StandardGate.X(), [0])
c.append_gate(StandardGate.RZ(0.5), [0], label="phase-correction")
```

`StandardGate` 对象支持属性查询、参数绑定、矩阵计算、求逆以及转换为多控制门等操作。

```python
from cqlib.circuit import StandardGate

gate = StandardGate.RX(0.25)

print(gate.num_qubits)       # 1
print(gate.num_params)       # 1
print(gate.num_ctrl_qubits)  # 0
print(gate.params)           # [Parameter(0.25)]
print(gate.matrix().shape)   # (2, 2)

inverse_gate = gate.inverse()
controlled = StandardGate.X().control(2)
```

下面将按照门的作用比特数量和参数形式，对常用标准门进行分类介绍。


### 1. 单比特非参数门

单比特非参数门是构造量子线路最基础的一类标准门,其作用于单个量子比特，使用时不需要提供角度参数，门的矩阵形式和作用效果在定义时已经固定。

您可以通过这些门完成量子态的基本变换，例如利用 `H` 门构造叠加态，利用 `X` 门实现比特翻转，利用 `Z`、`S`、`T` 等门调整相位，或结合 Clifford 门集构造便于分析和编译优化的基础线路。

在实际使用中，单比特非参数门通常作为更复杂线路的基础组成单元，既可以单独用于状态制备、相位修正和线路调试，也可以与双比特门、参数化旋转门组合，用于构造 Bell 态、GHZ 态、变分线路、误差校验线路和硬件原生门分解结果。

| `Circuit` 方法 | `StandardGate` | 说明 |
|---|---|---|
| `i(q)` | `I` | 恒等门 |
| `h(q)` | `H` | Hadamard 门 |
| `x(q)` | `X` | Pauli-X / NOT 门 |
| `y(q)` | `Y` | Pauli-Y 门 |
| `z(q)` | `Z` | Pauli-Z 门|
| `s(q)` | `S` | 相位门 |
| `sdg(q)` | `SDG` | `S` 门的逆 |
| `t(q)` | `T` | `pi/4` 相位门 |
| `tdg(q)` | `TDG` | `T` 门的逆 |
| `x2p(q)` | `X2P` | `sqrt(X)`，约定为 `X^(+1/2)` |
| `x2m(q)` | `X2M` | `sqrt(X)` 的逆 |
| `y2p(q)` | `Y2P` | `sqrt(Y)` |
| `y2m(q)` | `Y2M` | `sqrt(Y)` 的逆 |

```python
from cqlib import Circuit

c = Circuit(1)
c.h(0)
c.x(0)
c.sdg(0)
c.t(0)
c.x2p(0)
c.y2m(0)

print([op.instruction.instruction.name for op in c.operations])
```

需要注意的是，`i(q)` 与 `delay(q, duration)` 的语义不同。`i(q)` 表示量子线路中的标准恒等门；`delay(q, duration)` 表示硬件时间线上的空闲等待，通常用于保留调度或时序语义。

### 2. 单比特参数门

单比特参数门是在单个量子比特上执行的参数化量子门，通常通过一个或多个角度参数控制量子态在 Bloch 球上的旋转方向和旋转幅度。与 `H`、`X` 等固定门不同，参数门的实际作用由传入的数值或符号参数决定，因此可以在保持线路结构不变的情况下，通过调整参数改变线路行为。

在 Cqlib 中，单比特参数门既可以接受普通数值参数，也可以接受 Parameter 表达式。前者适合构造已经确定角度的线路；后者适合构造参数化线路模板，并在后续算法流程中进行参数绑定、参数扫描、优化迭代或符号矩阵验证。

参数化门广泛用于 VQE、QAOA、量子机器学习、变分线路设计和硬件校准等场景。例如，在变分量子算法中，线路结构通常保持不变，而优化器会不断更新旋转门中的参数值，从而搜索更优的量子态或目标函数值。

| `Circuit` 方法 | `StandardGate` | 参数 | 说明 |
|---|---|---|---|
| `rx(q, theta)` | `RX` | 1 | 绕 X 轴旋转 |
| `ry(q, theta)` | `RY` | 1 | 绕 Y 轴旋转 |
| `rz(q, theta)` | `RZ` | 1 | 绕 Z 轴旋转 |
| `phase(q, lambda_)` | `Phase` | 1 | 相位门 |
| `u(q, theta, phi, lambda_)` | `U` | 3 | 通用单比特门 |
| `xy(q, theta)` | `XY` | 1 | XY 系列单比特门 |
| `xy2p(q, theta)` | `XY2P` | 1 | `XY` 的正半角变体 |
| `xy2m(q, theta)` | `XY2M` | 1 | `XY` 的负半角变体 |
| `rxy(q, theta, phi)` | `RXY` | 2 | 在 XY 平面中指定旋转轴 |

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
phi = Parameter("phi")

c = Circuit(1)
c.rx(0, theta)
c.ry(0, 0.25)
c.rz(0, 2 * theta + phi)
c.phase(0, phi)
c.u(0, theta, 0.1, phi)
c.rxy(0, theta, phi)

print(c.symbols)
bound = c.assign_parameters({"theta": 0.3, "phi": 0.5})
print(bound.to_matrix())
```

如果线路中仍包含未绑定的符号参数，则无法直接使用 `to_matrix()` 得到数值矩阵。此时可以先通过 `assign_parameters()` 绑定参数，或使用 `to_symbolic_matrix()` 保留符号表达式。

`StandardGate.GPhase` 用于表示全局相位。在线路层面，更常用的方式是设置 `Circuit` 的全局相位：

```python
from cqlib import Circuit, Parameter

c = Circuit(1)
c.set_global_phase(Parameter("alpha"))
print(c.global_phase)
```

### 3. 双比特门与三比特门

双比特门与三比特门用于描述多个量子比特之间的相互作用，是构造纠缠态、受控逻辑和量子算法核心结构的核心组件。与单比特门只改变单个量子比特状态不同，多比特门可以在不同量子比特之间建立关联关系，例如通过 `CX`、`CZ` 等受控门构造 Bell 态、GHZ 态和各类纠缠线路，也可以通过 `SWAP` 调整量子比特之间的逻辑位置。

在实际量子算法中，多比特门通常承担“连接”不同量子比特信息的作用。例如，QAOA 中常用 `RZZ` 等双比特参数门表达问题哈密顿量中的相互作用项，量子傅里叶变换和相位估计算法中会使用受控相位类操作，纠错和验证线路中也经常使用 `CX`、`CCX` 等门实现条件翻转和辅助比特控制。

| `Circuit` 方法 | `StandardGate` | 参数 | 说明 |
|---|---|---|---|
| `cx(control, target)` | `CX` | 0 | 受控 X 门，CNOT |
| `cy(control, target)` | `CY` | 0 | 受控 Y 门 |
| `cz(control, target)` | `CZ` | 0 | 受控 Z 门 |
| `swap(a, b)` | `SWAP` | 0 | 交换两个量子比特状态 |
| `ccx(c1, c2, target)` | `CCX` | 0 | Toffoli 门 |
| `rxx(a, b, theta)` | `RXX` | 1 | `XX` Pauli 旋转 |
| `ryy(a, b, theta)` | `RYY` | 1 | `YY` Pauli 旋转 |
| `rzz(a, b, theta)` | `RZZ` | 1 | `ZZ` Pauli 旋转 |
| `rzx(a, b, theta)` | `RZX` | 1 | `ZX` Pauli 旋转 |
| `crx(control, target, theta)` | `CRX` | 1 | 受控 `RX` 门|
| `cry(control, target, theta)` | `CRY` | 1 | 受控 `RY` 门|
| `crz(control, target, theta)` | `CRZ` | 1 | 受控 `RZ` 门|
| `fsim(a, b, theta, phi)` | `FSIM` | 2 | fSim 双比特门 |

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
phi = Parameter("phi")

c = Circuit(3)
c.cx(0, 1)
c.swap(1, 2)
c.ccx(0, 1, 2)
c.rzz(0, 2, theta)
c.crx(1, 2, 0.25)
c.fsim(0, 1, theta, phi)
```

在添加多比特门时，Cqlib 会对操作对象进行合法性检查，包括目标量子比特是否已经在线路中注册、同一次操作是否重复引用同一个量子比特，以及传入参数的数量是否与门定义一致。若检查未通过，系统会抛出 `CircuitError` 或 `ParameterError`，以提示用户修正量子比特索引、作用对象或参数配置。

---

## 门矩阵与门求逆

在量子线路分析、算法验证和编译转换过程中，门的矩阵表示是理解其数学作用的重要依据。Cqlib 支持对标准门和多控制门直接计算矩阵，以便您检查门的维度、验证门的幺正性，以及分析门对量子态的具体作用。

在计算矩阵时，需要注意：

- 对于不含符号参数的门，可以直接调用 `matrix()` 获取其数值矩阵。
- 对于包含符号参数的门，需要在计算矩阵时提供具体的数值参数，或先通过参数绑定得到数值化后的门对象，再进行矩阵计算。

这样既可以保留参数化门在算法模板中的灵活性，也可以在需要数值验证时获得明确的矩阵结果。

```python
import numpy as np
from cqlib import Parameter
from cqlib.circuit import StandardGate

h = StandardGate.H()
h_matrix = h.matrix()
print(h_matrix)

theta = Parameter("theta")
rx = StandardGate.RX(theta)
rx_matrix = rx.matrix([np.pi / 2])

print(rx_matrix)
```

此外，Cqlib 还支持对可逆门执行求逆操作。

- 对于常见自反门，求逆结果与原门相同。
- 对于旋转门等参数化门，求逆通常表现为旋转角度取相反数。

```python
from cqlib.circuit import StandardGate

h_inverse = StandardGate.H.inverse()
print(h_inverse)  # H

rx_inverse = StandardGate.RX(0.5).inverse()
print(rx_inverse.params[0].evaluate())  # -0.5

s_inverse = StandardGate.S.inverse()
t_inverse = StandardGate.T.inverse()

print(s_inverse)  # SDG
print(t_inverse)  # TDG
```

常见门的求逆规则如下：

- `H`、`X`、`Y`、`Z`、`CX`、`CY`、`CZ`、`SWAP` 和 `CCX` 为自反门。
- `S` 与 `SDG` 互为逆门，`T` 与 `TDG` 互为逆门。
- 旋转门的逆门通常通过角度取负得到，例如 `RX(theta)^† = RX(-theta)`。
- `U(theta, phi, lambda)` 的逆门会根据矩阵定义转换参数。
- `Barrier` 不改变量子态，可视为自身的逆；`Measure` 和 `Reset` 不具备普通幺正逆操作。

---

## 多控制门

`MCGate` 用于将已有的 `StandardGate` 扩展为多控制门，即在原始门的基础上增加一个或多个控制量子比特。只有当所有控制比特满足指定控制条件时，目标门才会作用于对应的目标比特。该机制常用于构造 Toffoli 门、多控制相位门、oracle、条件翻转操作以及量子算法中的受控子程序。

在向线路中追加 `MCGate` 时，需要按照约定传入量子比特顺序：**先给出所有控制比特，再给出目标门所作用的目标比特**。

例如，对于一个三控制 `X` 门，前三个量子比特为控制比特，最后一个量子比特为目标比特。这样可以保证多控制门的语义清晰，也便于后续进行门分解、编译优化和硬件映射。


```python
from cqlib import Circuit, Parameter
from cqlib.circuit import MCGate, StandardGate

c = Circuit(4)

mcx = MCGate(3, StandardGate.X())
c.append_mc_gate(mcx, [0, 1, 2, 3])

theta = Parameter("theta")
mcrz = MCGate(2, StandardGate.RZ(theta))
c.append_mc_gate(mcrz, [0, 1, 2])

print(c[0].instruction.instruction.name)  # C3-X
print(c[1].params)                        # [theta]
```

除显式构造 `MCGate` 外，也可以直接从标准门对象调用  `control()` 方法生成多控制门。

```python
from cqlib.circuit import StandardGate

ccx = StandardGate.X().control(2)
controlled_cx = StandardGate.CX().control(1)
```

`StandardGate.CX().control(1)` 表示在已有 `CX` 门的基础上再增加一个控制比特，因此其总控制比特数为 2，与三比特 `CCX` 在控制结构上等价。

---

## 自定义幺正门

当算法需要使用内置标准门以外的幺正操作时，可以通过 `UnitaryGate` 定义自定义幺正门。自定义幺正门适用于表示算法中的特殊 oracle、问题相关变换、硬件专用门、已知矩阵形式的量子操作。

定义 `UnitaryGate` 时，需要明确指定门名称和作用量子比特数量。若使用矩阵定义该门，则矩阵维度必须为 `2^n × 2^n`，其中 `n` 表示该门作用的量子比特数量，即 `num_qubits`。Cqlib 会根据门定义检查矩阵维度与作用比特数量是否匹配。

```python
import numpy as np
from cqlib import Circuit
from cqlib.circuit import UnitaryGate

h_matrix = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)
custom_h = UnitaryGate("CustomH", 1).with_matrix(h_matrix)

c = Circuit(2)
c.append_unitary_gate(custom_h, [0])
```

除数值矩阵外，自定义幺正门也可以通过符号矩阵定义。符号矩阵适合描述带参数的门族，可以先定义门的符号形式，再从不同线路或不同位置传入具体参数值。这种方式常用于参数化 oracle、可调相位门、算法模板和符号验证场景。

```python
from cqlib import Circuit, Parameter
from cqlib.circuit import SymbolicComplex, SymbolicMatrix, UnitaryGate

theta = Parameter("theta")
phase = SymbolicComplex.exp_i(theta)

matrix = SymbolicMatrix(
    [
        [SymbolicComplex.one(), SymbolicComplex.zero()],
        [SymbolicComplex.zero(), phase],
    ]
)

gate = UnitaryGate("SymbolicPhase", 1, num_params=1).with_symbolic_matrix(
    matrix,
    ["theta"],
)

c = Circuit(1)
c.append_unitary_gate(gate, [0], [0.25])
```

需要注意的是，`UnitaryGate` 更适合用于已经具有明确矩阵定义的量子操作。如果目标只是将已有子线路作为一个可复用模块添加到其他线路中，通常更推荐使用 `CircuitGate`。`CircuitGate` 可以直接由子线路封装得到，既避免手动编写矩阵，也更便于后续进行分解、参数绑定和线路结构分析。

---

## 子线路门 `CircuitGate`

`CircuitGate` 用于将一段已有子线路封装为可复用的复合门。与 `UnitaryGate` 通过矩阵描述门的行为不同，`CircuitGate` 保留了子线路的结构信息，因此更适合表达算法模块、oracle、ansatz block、重复线路结构以及其他需要在多个位置复用的量子程序片段。

在实际使用中，您可以先构造一段子线路，然后通过 `to_gate(name)` 将其转换为 `CircuitGate`。转换后的复合门可以像普通门一样追加到其他线路中，并且可以在需要时通过 `decompose()` 展开为原始子线路操作，便于后续进行线路分析、参数绑定、编译优化或 IR 导出。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

sub = Circuit(1)
sub.rx(0, theta)
sub.rz(0, theta / 2)

block = sub.to_gate("ParamBlock")

main = Circuit(2)
main.append_circuit_gate(block, [0], [0.3])
main.append_circuit_gate(block, [1], [0.7])

decomposed = main.decompose()
print([op.instruction.instruction.name for op in decomposed.operations])
```

对于带参数的 `CircuitGate`，追加到线路时可以传入具体参数值。参数会按照 `gate.symbols` 中记录的符号顺序进行位置绑定；如果不传入 `params`，则会保留子线路中的原始符号参数，使该复合门仍然保持参数化形式。

```python
print(block.name)
print(block.num_qubits)
print(block.num_params)
print(block.symbols)
```

除使用 `to_gate(name)` 外，也可以显式使用 `FrozenCircuit` 和 `CircuitGate` 构造复合门。

```python
from cqlib import Circuit
from cqlib.circuit import CircuitGate, FrozenCircuit

sub = Circuit(1)
sub.h(0)

frozen = FrozenCircuit(sub.qubits, sub.operations)
gate = CircuitGate("HadamardBlock", frozen)
```

---

## Directive：非幺正指令

除普通量子门外，量子线路中还可能包含测量、重置、屏障和延迟等特殊指令。这类指令通常无法对应一个普通的幺正矩阵，因此在 Cqlib 中统一通过 `Directive` 表示。

`Directive` 主要用于描述线路执行过程中的辅助语义。例如，`barrier` 用于约束编译优化过程中的门重排，`measure` 用于将量子态信息读出为经典结果，`reset` 用于将量子比特重新初始化到 |0>，`delay` 则用于保留硬件时间调度中的空闲等待语义。

常用的非幺正指令可以直接通过 `Circuit` 提供的接口添加到线路中：

| `Circuit` 方法 | 指令名 | 说明 |
|---|---|---|
| `barrier(qubits)` | `Barrier` | 阻止编译器跨越指定比特重排门 |
| `measure(qubit)` | `measure_bit` | 测量单个比特，返回 `Measurement` |
| `measure_bits(qubits)` | `measure_bits` | 测量多个比特，返回 bit-vector 测量值 |
| `reset(qubit)` | `Reset` | 重置量子比特 |
| `delay(qubit, duration)` | `delay` | 在硬件时间线上保持空闲 |

```python
from cqlib import Circuit, Parameter
from cqlib.circuit import ClassicalType

c = Circuit(2)
c.h(0)
c.barrier([0, 1])

readout = c.measure(0)
bit_var = c.var(ClassicalType.bit())
c.measure_into(1, bit_var)

c.reset(0)
c.delay(1, Parameter("tau"))

print(readout.width)
print(c.classical_values)
print(c.classical_vars)
```

需要注意的是，非幺正指令会影响线路的数学表示和后续处理方式：

- `barrier` 本身不改变量子态，仅用于表达编译约束。因此，当线路中不包含其他非幺正操作时，带有 `barrier` 的线路仍可用于矩阵转换。
- `measure`、`measure_into` 和 `measure_bits` 会引入量子测量与经典结果，因此线路不再能用单一幺正矩阵完整表示。
- `reset` 会将量子比特重新初始化到固定状态，属于非幺正操作，同样不能通过普通幺正矩阵描述。
- `delay` 主要用于保留硬件执行或调度层面的时间语义。

因此，在进行 `to_matrix()`、线路等价性验证或门级优化时，应先确认线路中是否包含测量、重置、延迟等特殊指令。

---

## 低层指令与 ValueOperation

在一些更底层或更自动化的开发场景中，Cqlib支持显式构造 `Instruction` 和 `ValueOperation`，从而提供了一种更灵活的开发方式。例如，在编写线路反序列化器、IR 转换器、编译优化测试、自动化线路生成工具或自定义前端接口时，您可以通过此方式直接描述某一条操作的指令类型、作用量子比特、参数列表和标签信息。

```python
from cqlib import Circuit, Qubit
from cqlib.circuit import Instruction, StandardGate, ValueOperation

instruction = Instruction.from_standard_gate(StandardGate.H())
operation = ValueOperation.from_standard_gate(
    StandardGate.RX(0.25),
    [Qubit(0)],
    label="rx-layer-0",
)

c = Circuit(1)
c.append(operation)

print(c[0].instruction)
print(c[0].params)
print(c[0].label)
```

从语义上看，`Instruction` 用于描述“执行什么类型的指令”，`ValueOperation` 则用于描述这条指令在线路中的一次具体应用，包括它作用在哪些量子比特上、使用哪些参数，以及是否携带额外标签。

这种区分使 Cqlib 能够在高层线路构造、低层操作表示、IR 转换、编译优化和动态控制流处理之间复用统一的数据模型。

---

## 下一步

- [线路结构与构造](2_structures.md)：掌握 `Circuit` 的生命周期、索引、组合和操作表示。
- [参数系统](3_parameters.md)：学习参数表达式、参数绑定、表达式化简、符号求导和符号矩阵。
- [线路分析与转换](4_circuit_analysis.md)：使用反演、分解、矩阵转换和操作检查等工具。
