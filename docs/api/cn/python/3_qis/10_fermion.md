# 费米算子与 Jordan–Wigner 映射

`cqlib.qis.fermion` 提供通用费米代数与稀疏映射。公共类型也可以从 `cqlib.qis` 导入，计算由 Rust Core 完成。

```python
from cqlib.qis import FermionicLadder, FermionicOp, JordanWignerMapper

a = FermionicOp.annihilation(num_modes=4, mode=0)
identity = FermionicOp.identity(4)
assert (a * a.adjoint() + a.adjoint() * a).equiv(identity)

term = [FermionicLadder.create(2), FermionicLadder.annihilate(0)]
op = FermionicOp.from_term(num_modes=4, term=term, coefficient=0.5j)
hamiltonian = JordanWignerMapper().map_operator(op)
```

## 算子与运算

`FermionicLadder.create(mode)` 和 `annihilate(mode)` 创建不可变单算子，提供只读 `mode`、`kind`、`adjoint()`、值比较和哈希。它自身不声明模态空间，越界检查发生在构造 `FermionicOp` 时。

`FermionicOp(num_modes)` 创建零算子；`identity(num_modes)` 创建恒等算子。`from_term(num_modes, term, coefficient=1.0)` 接受一个有序乘积；`from_terms(num_modes, terms)` 接受 `(term, coefficient)` 二元组的可迭代对象，合并重复项。空乘积是恒等项，空项集合是零算子，二者都保留模态数量，包括零模态。

| 接口 | 行为 |
| --- | --- |
| `creation` / `annihilation` / `number` | 参数为 `num_modes, mode`；构造产生、湮灭、模态数算子。 |
| `total_number(num_modes)` | 构造总粒子数算子。 |
| `A + B` / `A - B` / `-A` | 返回新算子；加减要求模态数量一致。 |
| `A * B` | 算子乘积 AB，B 先作用；不自动正规序。 |
| `c * A` / `A * c` / `A.scaled(c)` | 标量乘法，接受有限的整数、浮点数和复数。 |
| `adjoint()` | 共轭系数、反转乘积并交换产生与湮灭。 |
| `normal_order()` | 返回新展开：产生算子在前，每组模态编号升序，保留收缩项与符号。 |
| `add_term(term, coefficient=1.0)` | 原地添加项；失败后对象不变，返回 `None`。 |
| `chop(*, atol=0.0)` | 原地删除复模长不大于容差的项，返回 `None`。 |
| `equiv(other, *, atol=0.0)` | 正规序后判断代数等价，模态数量不一致报错。 |
| `is_hermitian(*, atol=0.0)` | 判断算子与其共轭转置是否代数等价。 |

只读属性 `num_modes`、`num_terms` 描述存储；`terms` 返回独立的 `list[(tuple[FermionicLadder, ...], complex)]` 快照。`==` 比较存储结构，`is_zero()` 检查存储为空；数学等价与代数零使用 `equiv()` 判断。

默认只删除精确零，不隐式截断微小系数。容差必须有限且非负。`copy()`、`copy.copy()`、`copy.deepcopy()` 创建独立副本。算子可变且不可哈希；`A += B` 重新绑定结果，不修改 A 的别名。求和使用 `sum(operators, start=FermionicOp(num_modes))`。

## 激发生成元与演化

`excitation_generator(num_modes, sources, targets)` 构造 G = i(T − T†)。T 的产生算子按 targets 原顺序排列，湮灭算子按 sources 逆序排列。例如 `[0, 1] -> [2, 3]` 表示 T = a†₂ a†₃ a₁ a₀。列表必须非空、等长、各自无重复且互不重叠；不自动排序，也不引入阶乘归一化。

```python
from cqlib.circuit.ansatz import BasisEncoding, EvolutionStrategy, PauliEvolutionAnsatz

mapper = JordanWignerMapper()
circuit = BasisEncoding(mapper.map_occupation([1, 1, 0, 0])).build_circuit("hf")
for name, sources, targets in [
    ("theta_double", [0, 1], [2, 3]),
    ("theta_single", [0], [2]),
]:
    generator = FermionicOp.excitation_generator(4, sources, targets)
    block = (PauliEvolutionAnsatz(mapper.map_operator(generator))
             .with_strategy(EvolutionStrategy.exact())
             .with_time_param_name(name).build_circuit("excitation"))
    circuit.compose(block)
bound = circuit.assign_parameters({"theta_double": 0.2, "theta_single": -0.3})
```

每块演化为 exp(−iθG) = exp(θ(T − T†))。多个独立参数应分别构造、命名并组合演化块；把生成元相加后构造一块演化会共享一个参数。

## 映射与扩展

`JordanWignerMapper()` 提供 `map_operator(op) -> Hamiltonian`、`num_qubits(num_modes) -> int` 和 `map_occupation(occupation) -> list[bool]`。算子映射支持非厄米、恒等和零算子；线路与期望值消费者自行要求厄米性。

模态从 0 编号，模态 p 对应比特 p，Z 链覆盖小于 p 的编号，比特 0 是最低有效位。占据输入必须为整数 0/1 或布尔值；返回 `bits[p]` 对应比特 p，空输入返回空列表。Core 不规定 α/β 或化学轨道顺序。

`FermionToQubitMapper` 与 `OccupationStateMapper` 是独立的、可运行时检查的 `typing.Protocol`。开发者可以实现同签名的方法，不必继承协议；运行时检查只确认成员存在，不验证签名和计算正确性。Python 映射器由算法代码直接调用，不自动桥接到 Rust trait。

类型错误抛出 `TypeError`；负数或越界模态抛出 `IndexError`；负模态数量、空间不匹配、非有限系数、非法占据值或容差抛出 `ValueError`；模态整数超出 Rust 可表示范围抛出 `OverflowError`。
