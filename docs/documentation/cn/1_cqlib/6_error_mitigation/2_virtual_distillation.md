# 虚拟蒸馏（Virtual Distillation）

虚拟蒸馏（Virtual Distillation, VD）通过多份密度矩阵拷贝与 **copy-swap** 线路（即 Huggins 等人 PRX 11, 041036 的 Hadamard-test 线路），估计：

```text
Tr(O ρ^M) / Tr(ρ^M)
```

其中 `M` 为拷贝数（`copies`），`O` 为 `Hamiltonian` 表示的观测量。

---

## Python 入口

```python
import cqlib.error_mitigation as em
from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian, PauliString
```

---

## 快速示例：copy-swap 与 run_vd

```python
circuit = Circuit(1)
hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

vd = em.VirtualDistillation(circuit, copies=2)

copy_swap = vd.build_copy_swap_circuit()
print(copy_swap.width)  # 3 = copies * 基础宽度 + 1（最后一个比特是辅助比特）

def estimator(run_circuit, observable, shots):
    # 分子与分母都携带 Hamiltonian；分子在第 0 份副本的比特上有非恒等
    # Pauli，分母只在辅助比特（最后一个比特）上携带 X。
    is_numerator = any(
        qubit < observable.num_qubits - 1
        for term, _ in observable.terms
        for qubit in term.support()
    )
    if is_numerator:
        return (1.5, 0.25)      # numerator: Tr(O ρ^M)
    return (2.0, 1.0)           # denominator: Tr(ρ^M)

mean, var = vd.run_vd(hamiltonian, shots_numerator=3, shots_denominator=2, estimator=estimator)
print("expectation:", mean)   # 0.75
print("variance:", var)
```

`copies` 必须 **≥ 2**；否则抛出 `ErrorMitigationError`。

---

## 构造 copy-swap 线路

```python
vd = em.VirtualDistillation(circuit, copies=2)
copy_swap = vd.build_copy_swap_circuit()
```

- 输出线路宽度为 `copies` × 基础线路宽度 **+ 1**，最后一个比特是辅助比特；
- 辅助比特先经 `H` 门制备到 `|+⟩`，然后在相邻副本之间施加**由辅助比特控制**的 `SWAP` 梯（CSWAP），实现各副本的轮换（derangement）；
- 原始 `circuit` 不会被修改。

可通过 `set_copies(copies)` 更新拷贝数，再重新构造线路。

---

## 分步运行

| API | 说明 |
|-----|------|
| `run_denominator_circuit(shots, estimator)` | 估计 `Tr(ρ^M)`；观测量为辅助比特上的 `X` |
| `run_numerator_circuit(hamiltonian, shots, estimator)` | 估计 `Tr(O ρ^M)`；观测量为第 0 份副本上的 `O` ⊗ 辅助比特上的 `X` |
| `run_vd(hamiltonian, shots_numerator, shots_denominator, estimator)` | 一次完成分子/分母并返回比值 |

`estimator` 在分子与分母调用中都收到具体 `Hamiltonian`：分子观测量在第 0 份副本的比特上非平凡，并在辅助比特上携带 `X`；分母观测量只在辅助比特上携带 `X`（其余为恒等）。

---

## copies 与资源开销

| `copies` | 线路宽度 | 典型效果 |
|----------|----------|----------|
| `2` | 3 比特（2 份副本 + 1 个辅助比特） | 最低配置，开销较小 |
| `3` | 4 比特（3 份副本 + 1 个辅助比特） | 更强蒸馏，开销更高 |

`M` 越大，越突出主导本征态成分，但制备与测量成本显著增加。

---

## 与统一流水线集成

```python
mitigation = em.ErrorMitigation(
    circuit,
    em.MitigationMethod.virtual_distillation(em.VirtualDistillationConfig(2)),
)

def estimator(run_circuit, observable, shots):
    is_numerator = any(
        qubit < observable.num_qubits - 1
        for term, _ in observable.terms
        for qubit in term.support()
    )
    if is_numerator:
        return (1.5, 0.25)
    return (2.0, 1.0)

mitigation.run(hamiltonian, em.RunArgs.virtual_distillation(3, 2), estimator)
result = mitigation.get_mitigated(em.ProcessArgs.virtual_distillation())

print(result.expectation)
print(result.variance)
```

VD 的 `MitigatedResult.variance` 通常有值。

---

## 说明

- VD 适用于希望放大主导本征态、降低混合噪声影响的场景；
- 分子观测量会在内部扩展为「第 0 份副本上的 `O` ⊗ 其余副本恒等 ⊗ 辅助比特上的 `X`」，分母观测量为辅助比特上的 `X`，无需手动扩展 Pauli 项；
- 正式实验应分别记录 numerator 与 denominator 的 shot 预算。

---

## 下一步

- [统一流水线与 Estimator](3_unified_api.md)：了解 `ErrorMitigation` 如何封装 VD 流程。
