# Hamiltonian 观测量

`cqlib.qis`

`Hamiltonian` 用 Pauli 字符串的线性组合表示算子 $H = \sum_k c_k P_k$，其中 $c_k$ 为复系数、$P_k$ 为多比特 Pauli 字符串。它是 $2^N \times 2^N$ 矩阵的稀疏表示，用于描述系统能量、作为观测量计算期望值，以及生成时间演化线路。

本页同时覆盖 `Observable` 协议，它规定了观测量需要提供哪些成员。

## 导入

```python
from cqlib.qis import Hamiltonian, Observable
```

---

## Observable

观测量协议。它是由 `cqlib.qis` 定义的纯 Python `typing.Protocol`，不是原生类，也不提供运行时检查。

定义：

```python
from typing import Dict, List, Protocol, Tuple


class Observable(Protocol):
    def expectation_statevector(self, sv: Statevector) -> float: ...
    def expectation_density_matrix(self, dm: DensityMatrix) -> float: ...
    def expectation_probs(
        self, measurements: List[Tuple[PauliString, Dict[str, float]]]
    ) -> float: ...
    def variance_statevector(self, sv: Statevector) -> float: ...
    @property
    def num_qubits(self) -> int: ...
```

实现该协议需要提供以下成员：

- `expectation_statevector(sv) -> float`：由态矢量计算期望值 $\langle \psi | O | \psi \rangle$。
- `expectation_density_matrix(dm) -> float`：由密度矩阵计算期望值 $\mathrm{Tr}(\rho O)$。
- `expectation_probs(measurements) -> float`：由测量概率计算期望值，`measurements` 是若干 `(PauliString, dict)` 元组。
- `variance_statevector(sv) -> float`：由态矢量计算方差。
- `num_qubits -> int`：观测量作用的量子比特数，用于计算前的维数校验。

`Hamiltonian` 与 `PauliString` 都具备上述全部成员，可直接作为观测量传给接受 `Observable` 的接口。

---

## Hamiltonian

### Hamiltonian(num_qubits)

参数：

- `num_qubits` (`int`)：算符作用的量子比特数。构造出的是该比特数上的零算子。

### Hamiltonian.from_pauli(pauli)

用单个 Pauli 字符串构造，系数为 `1.0`。

参数：

- `pauli` (`PauliString`)：作为唯一项的 Pauli 字符串。

返回：

- `Hamiltonian`：表示 $H = 1.0 \cdot P$ 的算符。

### Hamiltonian.from_list(terms)

由 `(PauliString, 系数)` 元组列表构造。

参数：

- `terms` (`list[tuple[PauliString, float | int | complex | tuple[float, float]]]`)：项列表。系数可以是 `float`、`int`、`complex`，或 `(实部, 虚部)` 二元组。

返回：

- `Hamiltonian`：新的算符。

异常情况：

- `ValueError`：存在项的 Pauli 字符串比特数不一致；或列表元素不是二元组。

### 属性

- `num_qubits -> int`：量子比特数。
- `num_terms -> int`：当前项数，即 `terms` 列表在 `simplify()` 之前的长度。
- `terms -> list[tuple[PauliString, complex]]`：项列表，系数以 Python 复数给出。

### 方法

- `add_term(op, coeff) -> None`：追加一个 Pauli 项及其系数。系数取值形式与 `from_list` 相同。
- `simplify() -> None`：化简。先把 Pauli 字符串的内部相位吸收进复系数，再合并 Pauli 字符串相同的项、删除系数接近零的项。
- `scale(factor) -> None`：用复系数整体缩放所有项。系数取值形式与 `from_list` 相同。
- `all_terms_commute() -> bool`：判断所有 Pauli 项是否两两对易。
- `to_matrix() -> numpy.ndarray`：返回稠密矩阵 $H = \sum_k c_k P_k$，按小端比特序展开并计入 Pauli 相位。形状为 $(2^N, 2^N)$，`complex128` 类型；空算符得到零矩阵。空间复杂度为 $O(4^N)$，适用于小规模系统的分析校验。
- `to_trotter_circuit(time, steps, mode) -> Circuit`：生成 Trotter-Suzuki 分解给出的时间演化线路，用一串 Pauli 旋转逼近 $U(t) = e^{-iHt}$。
- `to_evolution_circuit(time, steps, mode) -> Circuit`：生成时间演化线路。所有项对易时走精确的单遍分解；存在非对易项时回退到指定的 Trotter 模式与步数。
- `copy() -> Hamiltonian`：返回副本。
- 标准库 `copy.copy(h)` 和 `copy.deepcopy(h)` 同样返回项数据独立的副本。
- `expectation_statevector(sv) -> float`：由态矢量计算期望值。
- `expectation_density_matrix(dm) -> float`：由密度矩阵计算期望值。
- `expectation_probs(measurements) -> float`：由测量概率计算期望值。
- `variance_statevector(sv) -> float`：由态矢量计算方差。

### 其他行为

- `+` 与 `+=` 把两个算符的项列表惰性拼接，不自动合并同类项；需要合并时调用 `simplify()`。
- 支持 `==`。
- 不定义 `hash`，不可哈希。
- `str` 给出算符的可读形式；`repr` 形如 `Hamiltonian(num_qubits=2, num_terms=2)`。

### 异常情况

实数期望值和方差接口要求吸收 Pauli 相位、合并重复项后的算符为厄米算符；原始及合并后的系数必须有限，否则抛出 `ValueError`。计算保留非零小系数。

- `ValueError`：两个算符比特数不一致（`+`、`+=`）、项的比特数与算符不一致（`from_list`、`add_term`）、系数类型不受支持、观测量与量子态比特数不一致（`expectation_*`、`variance_statevector`）、测量概率中找不到可用的测量基（`expectation_probs`）。
- `ValueError`：`to_trotter_circuit` 与 `to_evolution_circuit` 在 `steps` 为 `0`、算符为空、系数非 Hermitian、或 Pauli 字符串相位非 Hermitian 时抛出。

### 示例

```python
import math

from cqlib.qis import Hamiltonian, PauliString, Statevector

# 从空算符逐项构造
h = Hamiltonian(2)
h.add_term(PauliString.from_str("ZZ"), 0.5)
h.add_term(PauliString.from_str("XX"), 0.5)
h.simplify()

# 在 Bell 态上求期望值
sv = Statevector(2)
sv.apply_h(0)
sv.apply_cx(0, 1)
assert math.isclose(h.expectation_statevector(sv), 1.0)
```

```python
import math

from cqlib.qis import Hamiltonian, PauliString

# 由项列表构造，系数支持 float、complex 与 (实部, 虚部) 二元组
terms = [
    (PauliString.from_str("XX"), 0.5),
    (PauliString.from_str("YY"), -0.3 + 0.0j),
    (PauliString.from_str("ZZ"), (0.0, 0.2)),
]
h = Hamiltonian.from_list(terms)
assert h.num_qubits == 2
assert h.num_terms == 3
```

```python
import math

from cqlib.qis import Hamiltonian, PauliString

# 化简：合并同类项、删除近零项、把内部相位并入系数
h = Hamiltonian(2)
h.add_term(PauliString.from_str("XX"), 0.5)
h.add_term(PauliString.from_str("XX"), 0.3)
h.add_term(PauliString.from_str("ZZ"), 1.0)
h.add_term(PauliString.from_str("ZZ"), -1.0)
h.add_term(PauliString.from_str("+iYY"), 0.5)

assert h.num_terms == 5
h.simplify()
assert h.num_terms == 2

xx_term = next(t for t in h.terms if str(t[0]) == "+XX")
assert math.isclose(xx_term[1].real, 0.8)
```

```python
from cqlib.qis import Hamiltonian, PauliString, TrotterMode

# 时间演化线路
h = Hamiltonian(2)
h.add_term(PauliString.from_str("ZZ"), 0.5)
h.add_term(PauliString.from_str("XX"), 0.3)

circuit = h.to_trotter_circuit(1.0, 10, TrotterMode.first_order())
assert circuit.num_qubits == 2
assert len(circuit) > 0
```

```python
from cqlib.qis import Hamiltonian, PauliString, TrotterMode

# 所有项对易时 to_evolution_circuit 走精确分解
h_commuting = Hamiltonian(2)
h_commuting.add_term(PauliString.from_str("ZZ"), 0.5)
h_commuting.add_term(PauliString.from_str("IZ"), 0.3)
assert h_commuting.all_terms_commute() is True

exact = h_commuting.to_evolution_circuit(1.0, 1, TrotterMode.first_order())
assert exact.num_qubits == 2

# 存在非对易项时回退到 Trotter
h_non_commuting = Hamiltonian(1)
h_non_commuting.add_term(PauliString.from_str("X"), 1.0)
h_non_commuting.add_term(PauliString.from_str("Z"), 1.0)
assert h_non_commuting.all_terms_commute() is False

trotterized = h_non_commuting.to_evolution_circuit(0.5, 4, TrotterMode.second_order())
assert trotterized.num_qubits == 1
```

---

## 校验与错误处理

| 异常 | 触发场景 |
| --- | --- |
| `ValueError` | 比特数不一致：`from_list` 的项之间、`add_term` 的项与算符之间、`+` 与 `+=` 的两个算符之间、观测量与量子态之间。 |
| `ValueError` | 系数不是 `float`、`int`、`complex` 或 `(实部, 虚部)` 二元组。 |
| `ValueError` | `to_trotter_circuit` 与 `to_evolution_circuit`：`steps` 为 `0`、算符为空、系数或 Pauli 字符串相位非 Hermitian。 |
| `ValueError` | `expectation_probs` 无法从给定的测量基推出待求算符。 |

观测量计算的输入类型见 [Statevector](1_statevector.md) 与 [DensityMatrix](2_density_matrix.md)；Trotter 模式的定义见 [Trotter 演化](8_evolution.md)。

## 经过校验的稀疏代数

`from_terms(num_qubits, terms)` 明确指定宽度，接受 `(PauliString, 整数/浮点数/复数)` 项。它校验尺寸与有限系数、吸收 Pauli 相位并合并重复项，只删除精确零；空输入保留指定宽度。

| 接口 | 行为 |
| --- | --- |
| `normalized()` | 返回校验与合并后的新对象，保留所有非零系数。 |
| `add(other)` | 同宽度的安全加法，返回合并后的新对象。 |
| `A - B`、`A * B`、`-A` | 安全减法、算子乘积及取负；AB 中 B 先作用。 |
| `c * A`、`A * c`、`scaled(c)` | 校验标量及结果系数，返回新对象。 |
| `adjoint()` | 返回共轭转置，包括 Pauli 相位。 |
| `chop(*, atol=0.0)` | 原地规范化，并删除复模长不大于容差的系数；失败后对象不变。 |
| `equiv(other, *, atol=0.0)` | 比较规范化后的系数差，宽度不一致报错；容差不是矩阵算子范数界。 |
| `is_hermitian(*, atol=0.0)` | 与共轭转置比较，即规范化系数满足 2·abs(imag) ≤ atol。 |
| `normalized_hermitian()` | 将模长不大于 1e-10 的虚部残差投影为零，更大虚部报错；保留任意非零实部。 |
| `tensor(other)` | 返回 self ⊗ other，other 占低编号比特。 |
| `remap_qubits(mapping, target_width)` | mapping[source] = destination；目标必须唯一且在范围内，未使用位置补恒等。 |

这些接口支持非厄米算子，厄米性由演化与期望值消费者校验。所有容差必须有限且非负。比特嵌入是量子比特重标记，不是费米模态置换。

```python
from cqlib.qis import Hamiltonian, PauliString

x = Hamiltonian.from_terms(1, [(PauliString.from_str("X"), 1.0)])
z = Hamiltonian.from_terms(1, [(PauliString.from_str("Z"), 1.0)])
assert (x * z).equiv(Hamiltonian.from_terms(1, [(PauliString.from_str("Y"), -1j)]))
assert x.add(x).equiv(2 * x)
assert x.remap_qubits([2], target_width=3).num_qubits == 3
```

### 旧接口兼容

`from_list`、`add_term`、`scale`、`+`、`+=` 保留现有行为。其中 `+` 与 `+=` 拼接存储项；`scale` 原地缩放。`simplify()` 使用旧的近零截断策略，会删除微小系数。需要完整保留映射结果时使用 `normalized()`，需要明确截断时使用 `chop(atol=...)`。

`add_term()` 和 `scale()` 原地修改并返回 `None`，不支持链式调用。新代数运算遇到宽度不一致、非有限系数或中间系数溢出时抛出 `ValueError`。
