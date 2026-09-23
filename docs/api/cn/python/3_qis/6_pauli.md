# Pauli 算子

`cqlib.qis`

`Phase`、`Pauli` 与 `PauliString` 是 Pauli 群的三个层次：`Phase` 表示群元素相乘时产生的相位因子，`Pauli` 是单比特 Pauli 算子，`PauliString` 是多比特 Pauli 算子的张量积。`PauliString` 采用辛表示存储，同时提供字符形式、比特掩码与矩阵形式之间的转换。

## 导入

```python
from cqlib.qis import Phase, Pauli, PauliString
```

---

## Phase

Pauli 群中的相位因子，同构于 4 阶循环群，表示 $i^n$ 且 $n \in \{0, 1, 2, 3\}$。

### Phase(val)

参数：

- `val` (`int`)：相位指数，按 4 取模，可取值范围为 `0`–`255`。

### 静态方法

- `Phase.plus()`：返回 $+1$，即 $i^0$。
- `Phase.i()`：返回 $+i$，即 $i^1$。
- `Phase.minus()`：返回 $-1$，即 $i^2$。
- `Phase.minus_i()`：返回 $-i$，即 $i^3$。

### 属性

- `exponent -> int`：相位指数，取值 `0`–`3`。

### 方法

- `to_complex() -> complex`：转换为 Python 复数。

### 其他行为

- `+` 与 `*` 都表示群乘法，结果按 4 取模。
- 支持 `==`、`hash`。
- `str` 为 `1`、`i`、`-1`、`-i`；`repr` 形如 `Phase(1)`。

### 异常情况

- `OverflowError`：`val` 超出 `0`–`255` 时抛出。

---

## Pauli

单比特 Pauli 算子。

### 静态方法

- `Pauli.x()`、`Pauli.y()`、`Pauli.z()`、`Pauli.i()`：四个 Pauli 算子的唯一构造入口。

### 方法

- `to_symplectic() -> tuple[int, int]`：返回辛表示 $(x, z)$。`I` 为 `(0, 0)`，`X` 为 `(1, 0)`，`Y` 为 `(1, 1)`，`Z` 为 `(0, 1)`。
- `to_matrix() -> numpy.ndarray`：返回 $2 \times 2$ 复数矩阵。
- `mul_with_phase(other) -> tuple[Pauli, Phase]`：与另一个单比特 Pauli 算子相乘，返回结果算子与相位因子。

### 其他行为

- 没有公开构造方法。
- `*` 表示相乘，但不返回相位因子；需要相位时用 `mul_with_phase`。
- 支持 `==`、`hash`。
- `str` 为算符名；`repr` 形如 `Pauli.X`。

---

## PauliString

多比特 Pauli 算子，即 $P = \bigotimes_{i=0}^{N-1} P_i$，其中 $P_i \in \{I, X, Y, Z\}$。

### PauliString(num_qubits)

参数：

- `num_qubits` (`int`)：量子比特数。构造出的是全 `I` 字符串，相位为 $+1$。

### PauliString.from_str(s)

从字符形式构造。格式为 `[+|-][i|j]<算符序列>`，算符序列由 `I`、`X`、`Y`、`Z` 组成；量子比特按下标逆序排列，首个字符对应最高比特下标。`j` 与 `i` 等价，内部统一为 `i`。

参数：

- `s` (`str`)：字符形式，例如 `"XZI"`、`"-iZII"`、`"+XYZ"`。

返回：

- `PauliString`：新的 Pauli 字符串。

异常情况：

- `ValueError`：字符串为空、含有非法字符、或只给出相位而没有算符时抛出。

### 属性

- `num_qubits -> int`：量子比特数。
- `phase -> Phase`：全局相位，可读可写。
- `x_bits -> list[bool]`：$X$ 分量比特向量，下标为比特下标。
- `z_bits -> list[bool]`：$Z$ 分量比特向量。
- `x_mask -> int`：$X$ 分量的整数掩码。
- `z_mask -> int`：$Z$ 分量的整数掩码。

### 方法

- `get_pauli(idx) -> Pauli`：读取指定比特上的算符。
- `set_pauli(idx, pauli) -> None`：写入指定比特上的算符。
- `y_phase() -> complex`：返回 $Y$ 算符贡献的相位因子。由 $Y = iXZ$，$n$ 个 $Y$ 贡献 $i^n$。
- `commutes_with(other) -> bool`：判断两个 Pauli 字符串是否对易，依据是辛内积模 2 是否为零。
- `support() -> list[int]`：返回取了非 `I` 算符的比特下标，升序排列。
- `to_matrix() -> numpy.ndarray`：返回稠密矩阵，按小端张量序展开（比特 `0` 是最低有效张量因子，即 $P_{N-1} \otimes \cdots \otimes P_0$），并计入全局相位。形状为 $(2^N, 2^N)$，`complex128` 类型。空间复杂度为 $O(4^N)$，适用于小规模系统的分析校验。
- `copy() -> PauliString`：返回副本。
- `expectation(probs) -> float`：由概率分布计算期望值 $\langle P \rangle = \sum_s p(s)\langle s | P | s \rangle$。若字符串含 `X` 或 `Y` 等非对角算符，对任意计算基概率分布的期望值均为 `0`。
- `expectation_statevector(sv) -> float`：由态矢量计算期望值。
- `expectation_density_matrix(dm) -> float`：由密度矩阵计算期望值。
- `expectation_probs(measurements) -> float`：由测量概率计算期望值。
- `variance_statevector(sv) -> float`：由态矢量计算方差。

### 其他行为

- `len(ps)` 等于 `num_qubits`；零比特字符串的布尔值为假。
- 可直接迭代，按比特下标升序逐个给出 `Pauli`，包含 `I`。迭代器采用快照语义：创建迭代器之后对原字符串的修改不影响本次迭代。迭代器提供 `__length_hint__`，给出剩余元素个数。
- `ps[i]` 读取指定比特上的算符，支持负下标，`-1` 指最高比特下标。
- `*` 与 `*=` 表示相乘，结果的相位按群乘法累加。
- 支持 `==`；作为可变类型不定义 `hash`，不可哈希，也不能用作字典键。
- `str` 形如 `+XYZ`、`-iZII`；`repr` 形如 `PauliString(num_qubits=3, phase=1, x_bits=[...], z_bits=[...])`。

### 异常情况

- `ValueError`：`from_str` 解析失败；`commutes_with` 或 `*` 的两个操作数比特数不一致。
- `IndexError`：`get_pauli`、`set_pauli` 的下标越界；`ps[i]` 的下标越界。

### 期望值参数说明

`expectation(probs)` 的 `probs` 是状态串到概率的字典。状态串按小端约定书写：最右字符对应比特 `0`，例如 `"01"` 表示比特 `0` 为 `1`、比特 `1` 为 `0`。状态串长度必须等于 `num_qubits`。

`expectation_probs(measurements)` 的 `measurements` 是若干 `(PauliString, dict)` 元组的列表，每个元组给出一组测量基与对应的概率分布。若待求算符无法由给出的测量基推出，则报错。

### 示例

```python
from cqlib.qis import Pauli, PauliString, Phase

# 乘法与相位
x, y, z, i = Pauli.x(), Pauli.y(), Pauli.z(), Pauli.i()
res, phase = x.mul_with_phase(y)  # XY = iZ
assert res == z
assert phase == Phase.i()
assert x * y == z
assert x.to_symplectic() == (1, 0)

# 字符形式与属性
ps = PauliString.from_str("-iZII")
assert ps.num_qubits == 3
assert str(ps) == "-iZII"
assert ps.phase == Phase.minus_i()
assert PauliString.from_str("+jZZ").phase == Phase.i()

# 掩码、支撑集与迭代
ps = PauliString.from_str("XIZ")
assert ps.x_mask == 0b100
assert ps.z_mask == 0b001
assert ps.support() == [0, 2]
assert len(ps) == 3
assert list(ps) == [Pauli.z(), Pauli.i(), Pauli.x()]  # 按比特下标升序，含 I
```

```python
from cqlib.qis import Pauli, PauliString

# 构造与逐比特读写
ps = PauliString(3)
assert str(ps) == "+III"
ps.set_pauli(0, Pauli.x())
ps.set_pauli(1, Pauli.z())
ps.set_pauli(2, Pauli.y())
assert str(ps) == "+YZX"
assert ps.get_pauli(0) == Pauli.x()
assert ps[0] == Pauli.x()
assert ps[-1] == Pauli.y()
```

```python
from cqlib.qis import PauliString

# 乘法与对易性
ps1 = PauliString.from_str("X")
ps2 = PauliString.from_str("Z")
assert str(ps1 * ps2) == "-iY"

ps3 = PauliString.from_str("XZ")
ps4 = PauliString.from_str("YX")
assert str(ps3 * ps4) == "-ZY"

ps5 = PauliString.from_str("XZI")
assert ps5.commutes_with(PauliString.from_str("ZXI")) is True
assert ps5.commutes_with(PauliString.from_str("YII")) is False
```

```python
import math

from cqlib.qis import PauliString, Statevector

# 期望值与方差
sv = Statevector(1)
ps_z = PauliString.from_str("Z")
assert math.isclose(ps_z.expectation_statevector(sv), 1.0)

sv.apply_h(0)
ps_x = PauliString.from_str("X")
assert math.isclose(ps_x.expectation_statevector(sv), 1.0)
assert math.isclose(ps_z.variance_statevector(sv), 1.0, abs_tol=1e-10)

# 由概率分布求期望值
assert math.isclose(ps_z.expectation({"0": 0.5, "1": 0.5}), 0.0)
```

```python
import numpy as np

from cqlib.qis import PauliString

# 矩阵形式，小端张量序：比特 0 是最低有效张量因子
ps = PauliString.from_str("ZX")
matrix = ps.to_matrix()
assert matrix.shape == (4, 4)
assert matrix.dtype == np.complex128
```

---

## 校验与错误处理

| 异常 | 触发场景 |
| --- | --- |
| `ValueError` | `PauliString.from_str` 的字符串为空、含非法字符或缺少算符；`commutes_with`、`*`、`*=` 的两个操作数比特数不一致。 |
| `IndexError` | `get_pauli`、`set_pauli` 或 `ps[i]` 的比特下标越界。 |
| `OverflowError` | `Phase(val)` 的指数超出可取值范围。 |

`expectation_statevector`、`expectation_density_matrix` 与 `variance_statevector` 的输入类型见 [Statevector](1_statevector.md) 与 [DensityMatrix](2_density_matrix.md)。
