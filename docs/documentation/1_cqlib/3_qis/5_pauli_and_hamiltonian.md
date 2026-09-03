# Pauli、PauliString 与 Hamiltonian

Pauli 算符和 Pauli 哈密顿量是 VQE、QAOA、量子模拟、稳定子分析和测量后处理的基础。`cqlib.qis` 提供单比特 `Pauli`、多比特 `PauliString`、全局相位 `Phase` 和 Pauli 项和式 `Hamiltonian`。

本节重点不是线路构造，而是如何描述可观测量、判断对易性、计算期望值，并把哈密顿量转换为演化线路。

---

## 任务：理解单比特 Pauli 与相位

```python
from cqlib.qis import Pauli

x = Pauli.x()
y = Pauli.y()
z = Pauli.z()

result, phase = x.mul_with_phase(z)

print(result)
print(phase)
print(phase.to_complex())
print(y.to_symplectic())
print(z.to_matrix())
```

Pauli 乘法会产生全局相位。需要保留相位时使用 `mul_with_phase()`；只关心 Pauli 类型时可以使用普通乘法，但不要用它解释完整的相位关系。

---

## 构造 PauliString

`PauliString` 表示多比特 Pauli 算符。可以从字符串创建，也可以逐位设置。

```python
from cqlib.qis import Pauli, PauliString

string = PauliString.from_str("XZI")
print(string.num_qubits)
print(string.x_bits)
print(string.z_bits)
print(string.x_mask)
print(string.z_mask)

manual = PauliString(3)
manual.set_pauli(0, Pauli.x())
manual.set_pauli(1, Pauli.z())
manual.set_pauli(2, Pauli.i())

print(manual)
```

使用字符串和 bitstring 概率一起做后处理时，需要统一 qubit 顺序约定。特别是从概率字典计算期望值时，应确认结果字符串的每一位对应哪个 qubit。

---

## 判断对易性和相乘

```python
from cqlib.qis import PauliString

xx = PauliString.from_str("XX")
zz = PauliString.from_str("ZZ")
zi = PauliString.from_str("ZI")
iz = PauliString.from_str("IZ")

print(xx.commutes_with(zz))
print(zi.commutes_with(iz))
print(xx * zz)
```

对易性决定了 Pauli 项是否可以在同一测量基中合并处理，也影响 Hamiltonian 的演化分解。构建 VQE 或 QAOA 测量流程时，应尽早检查 Pauli 项的对易结构。

---

## 从状态或概率计算 Pauli 期望值

`PauliString` 可以直接对 `Statevector` 或 `DensityMatrix` 计算期望值，也可以从测量概率字典估计期望值。

```python
from cqlib.qis import PauliString, Statevector

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

zz = PauliString.from_str("ZZ")
print(zz.expectation_statevector(state))

probs = {
    "00": 0.5,
    "11": 0.5,
}
print(zz.expectation(probs))
```

从概率字典计算期望值时，只适合已经在对应 Pauli 测量基下得到的概率。对于含 `X` 或 `Y` 的 PauliString，不能直接把计算基采样概率当作对应测量结果，除非线路中已经完成了正确的换基测量。

---

## 构造 Hamiltonian

`Hamiltonian` 是 PauliString 与系数的稀疏和式，形式为 `H = Σ c_k P_k`。

```python
from cqlib.qis import Hamiltonian, PauliString

hamiltonian = Hamiltonian(2)
hamiltonian.add_term(PauliString.from_str("ZZ"), 1.0)
hamiltonian.add_term(PauliString.from_str("XI"), 0.5)
hamiltonian.add_term(PauliString.from_str("ZZ"), -0.2)

print(hamiltonian.num_terms)
hamiltonian.simplify()
print(hamiltonian.terms)
```

也可以从列表一次性创建：

```python
hamiltonian = Hamiltonian.from_list([
    (PauliString.from_str("ZI"), -1.0),
    (PauliString.from_str("IZ"), -1.0),
    (PauliString.from_str("ZZ"), 0.5),
])
```

`simplify()` 会合并重复 PauliString，并把 PauliString 内部相位吸收到系数中。构造大哈密顿量后，建议先 `simplify()` 再进入模拟、测量分组或演化分解。

---

## 计算 Hamiltonian 期望值和方差

```python
from cqlib.qis import Hamiltonian, PauliString, Statevector

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

hamiltonian = Hamiltonian.from_list([
    (PauliString.from_str("ZZ"), 1.0),
    (PauliString.from_str("XX"), 0.5),
])

print(hamiltonian.expectation_statevector(state))
print(hamiltonian.variance_statevector(state))
```

如果输入是密度矩阵，可以使用 `expectation_density_matrix()`。如果输入来自真实采样结果，可以使用 `expectation_probs()` 汇总每个 Pauli 测量基下的概率。

---

## 构造 Trotter 演化线路

`Hamiltonian` 可以转换为近似实现 `e^{-iHt}` 的量子线路。`TrotterMode` 提供一阶、二阶和随机化模式。

```python
from cqlib.qis import Hamiltonian, PauliString, TrotterMode

hamiltonian = Hamiltonian.from_list([
    (PauliString.from_str("ZZ"), 0.5),
    (PauliString.from_str("XI"), 0.3),
])

circuit = hamiltonian.to_trotter_circuit(
    time=1.0,
    steps=4,
    mode=TrotterMode.first_order(),
)

print(circuit)
```

对于对易项，`to_evolution_circuit()` 可以使用更直接的演化分解；对于非对易项，会回退到指定 Trotter 模式。

```python
circuit = hamiltonian.to_evolution_circuit(
    time=1.0,
    steps=4,
    mode=TrotterMode.second_order(),
)
```

时间演化线路的精度取决于哈密顿量项是否对易、总演化时间、Trotter 步数和分解阶数。正式实验前应先在小规模系统上做误差对比。

---

## 下一步

·[Statevector 纯态模拟](1_statevector.md):把 PauliString 和 Hamiltonian 作用到理想纯态上，验证期望值和能量函数。  
·[DensityMatrix 混态模拟](2_density_matrix.md):把同一组可观测量用于混态和含噪态分析。  
·[量子态指标与熵](6_metrics_entropy.md):在能量之外，继续比较保真度、迹距离、纯度和纠缠指标。
