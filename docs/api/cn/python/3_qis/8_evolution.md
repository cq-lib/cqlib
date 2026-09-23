# Trotter 演化

`cqlib.qis`

`TrotterMode` 描述如何把时间演化算子 $U(t) = e^{-iHt}$ 近似为一串 Pauli 旋转，是把 `Hamiltonian` 转成演化线路时必须指定的分解方式。本页覆盖 `TrotterMode` 本身，以及消费它的两个演化入口。

## 导入

```python
from cqlib.qis import Hamiltonian, PauliString, TrotterMode
```

---

## TrotterMode

Trotter-Suzuki 分解模式。没有公开构造方法，三种模式分别由静态方法创建；相等性按模式与随机种子判定。

### 静态方法

- `TrotterMode.first_order()`：一阶 Lie-Trotter 分解。对 $H = \sum_k c_k P_k$，单步按项顺序逐个作用
  $U(t) \approx \left[ \prod_k e^{-i c_k (t/n) P_k} \right]^n$，
  误差量级为 $O(t^2/n)$。
- `TrotterMode.second_order()`：二阶 Strang 对称分解。单步先按项顺序、再按相反顺序各作用一次
  $U(t) \approx \left[ \prod_k e^{-i c_k (t/2n) P_k} \prod_k e^{-i c_k (t/2n) P_k} \right]^n$，
  误差量级为 $O(t^3/n^2)$。
- `TrotterMode.randomized(seed)`：随机化一阶分解。每个 Trotter 步内随机打乱 Pauli 项的顺序，用于削弱系统性误差。

参数：

- `seed` (`int`)：随机种子，无符号整数，相同种子给出相同结果。

返回：

- `TrotterMode`：对应的分解模式对象。

### 其他行为

- 支持 `==` 与 `hash`。`TrotterMode.randomized(s)` 之间只有种子相同才相等；`first_order()` 与 `second_order()` 分别与各自的新实例相等。
- `str` 为 `first-order`、`second-order`、`randomized (seed=N)`。
- `repr` 为 `TrotterMode.FirstOrder`、`TrotterMode.SecondOrder`、`TrotterMode.Randomized(seed=N)`。

---

## 演化入口

`TrotterMode` 由 `Hamiltonian` 的两个演化方法消费。二者都返回抽象线路，参数含义一致。

### Hamiltonian.to_trotter_circuit(time, steps, mode) -> Circuit

按指定模式与步数生成 Trotter 化的时间演化线路。

参数：

- `time` (`float`)：总演化时间 $t$。
- `steps` (`int`)：Trotter 步数 $n$，必须大于 `0`。
- `mode` (`TrotterMode`)：分解模式。

返回：

- `Circuit`：逼近 $U(t) = e^{-iHt}$ 的线路。

### Hamiltonian.to_evolution_circuit(time, steps, mode) -> Circuit

生成时间演化线路。所有项对易时走精确的单遍分解 $\prod_k e^{-i c_k t P_k}$；存在非对易项时按指定的模式与步数做 Trotter 近似。`steps` 与 `mode` 只在非对易时起作用，但 `steps` 仍须大于 `0`。

参数：

- `time` (`float`)：总演化时间 $t$。
- `steps` (`int`)：Trotter 步数，必须大于 `0`。仅在对易性不成立时参与分解。
- `mode` (`TrotterMode`)：分解模式。仅在对易性不成立时参与分解。

返回：

- `Circuit`：精确或近似的演化线路。

### 异常情况

两个入口在以下情况抛出 `ValueError`：

- `steps` 为 `0`。
- `Hamiltonian` 为空。
- 系数不是 Hermitian 的（虚部超出容差）。
- Pauli 字符串的相位不是 Hermitian 的，即相位为 $\pm i$ 而非 $\pm 1$。

### 示例

```python
from cqlib.qis import TrotterMode

# 三种模式
mode = TrotterMode.first_order()
assert str(mode) == "first-order"
assert "FirstOrder" in repr(mode)

assert str(TrotterMode.second_order()) == "second-order"

mode_random = TrotterMode.randomized(42)
assert "seed=42" in repr(mode_random)
assert mode_random != TrotterMode.randomized(123)
assert mode_random == TrotterMode.randomized(42)
```

```python
from cqlib.qis import Hamiltonian, PauliString, TrotterMode

h = Hamiltonian(2)
h.add_term(PauliString.from_str("ZZ"), 0.5)
h.add_term(PauliString.from_str("XX"), 0.3)

first = h.to_trotter_circuit(1.0, 10, TrotterMode.first_order())
second = h.to_trotter_circuit(1.0, 10, TrotterMode.second_order())
randomized = h.to_trotter_circuit(1.0, 10, TrotterMode.randomized(42))

assert first.num_qubits == 2
assert second.num_qubits == 2
assert randomized.num_qubits == 2
```

```python
from cqlib.qis import Hamiltonian, PauliString, TrotterMode

# 对易项走精确分解，步数与模式不参与分解
h_commuting = Hamiltonian(2)
h_commuting.add_term(PauliString.from_str("ZZ"), 0.5)
h_commuting.add_term(PauliString.from_str("IZ"), 0.3)

exact = h_commuting.to_evolution_circuit(1.0, 1, TrotterMode.first_order())
assert exact.num_qubits == 2

# 非对易项回退到 Trotter
h_non_commuting = Hamiltonian(1)
h_non_commuting.add_term(PauliString.from_str("X"), 1.0)
h_non_commuting.add_term(PauliString.from_str("Z"), 1.0)

trotterized = h_non_commuting.to_evolution_circuit(0.5, 4, TrotterMode.second_order())
assert trotterized.num_qubits == 1
```

---

## 校验与错误处理

| 异常 | 触发场景 |
| --- | --- |
| `ValueError` | 演化入口的 `steps` 为 `0`、`Hamiltonian` 为空、系数或 Pauli 字符串相位非 Hermitian。 |

需要在参数化的演化线路构造中指定分解方式时，使用线路层的演化策略接口，见 [Ansatz](../0_circuit/11_ansatz.md)。`Hamiltonian` 的完整接口见 [Hamiltonian 观测量](7_hamiltonian.md)。
