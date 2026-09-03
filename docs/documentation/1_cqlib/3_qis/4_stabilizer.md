# StabilizerState 稳定子模拟

`StabilizerState` 用稳定子表表示 Clifford 线路。它适合模拟只包含 Clifford 门的线路，并可以直接读取稳定子生成元、destabilizer、Pauli 期望值和测量结果。

当线路包含任意角度旋转、`T` 门、`fSim` 或其他非 Clifford 操作时，应改用 `Statevector` 或 `DensityMatrix`。

---

## 任务：用稳定子模拟 Bell 态

```python
from cqlib.qis import StabilizerState

state = StabilizerState(2)
state.apply_h(0)
state.apply_cx(0, 1)

print(state.probabilities())
print(state.get_stabilizers())
```

Bell 态的稳定子生成元可以用来检查纠缠结构。相比只看概率，稳定子生成元能更直接地表达态满足哪些 Pauli 约束。

---

## 从 Circuit 构造稳定子态

如果已有一条 Clifford 线路，可以直接从线路构造稳定子态。

```python
from cqlib import Circuit
from cqlib.qis import StabilizerState

circuit = Circuit(3)
circuit.h(0)
circuit.cx(0, 1)
circuit.cx(1, 2)

state = StabilizerState.from_circuit(circuit)
print(state.probabilities())
```

也可以用 `apply_circuit()` 把线路原地作用到已有稳定子态。

```python
prefix = Circuit(3)
prefix.h(0)

suffix = Circuit(3)
suffix.cx(0, 1)
suffix.cx(1, 2)

state = StabilizerState(3)
state.apply_circuit(prefix)
state.apply_circuit(suffix)

print(state.get_stabilizers())
```

这种分段方式适合检查 Clifford 编码线路、纠错线路或大规模 GHZ 线路的中间稳定子结构。

---

## 执行带经典状态的稳定子线路

`run_circuit()` 返回 `StabilizerCircuitResult`，其中包含最终稳定子态和经典状态。

```python
from cqlib import Circuit
from cqlib.qis import StabilizerState

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
circuit.measure(0)

result = StabilizerState.run_circuit(circuit)

print(result.state)
print(result.classical)
```

当线路包含测量或动态线路中的经典值时，`run_circuit()` 比只读取最终量子态更适合保留经典执行结果。

---

## 测量、重置和采样

稳定子态支持单比特测量、全测量、重置和有限 shot 采样。

```python
from cqlib.qis import StabilizerState

state = StabilizerState(2)
state.apply_h(0)
state.apply_cx(0, 1)

shots = state.sample_shots(32)
counts = {}
for outcome in shots:
    bitstring = outcome.to_bitstring(state.num_qubits)
    counts[bitstring] = counts.get(bitstring, 0) + 1

print(counts)

measured = state.measure_all()
print(measured.to_bitstring(2))
```

`measure()` 和 `measure_all()` 会坍缩当前稳定子态；`sample_shots()` 适合在不手动处理坍缩过程的情况下生成采样结果。

---

## 查看稳定子、destabilizer 和 Pauli 期望值

```python
from cqlib.qis import PauliString, StabilizerState

state = StabilizerState(2)
state.apply_h(0)
state.apply_cx(0, 1)

print("stabilizers:", state.get_stabilizers())
print("destabilizers:", state.get_destabilizers())
print("ZZ:", state.pauli_expectation(PauliString.from_str("ZZ")))
print("XI:", state.pauli_expectation(PauliString.from_str("XI")))
print(state.to_stim_format())
```

`pauli_expectation()` 返回 `-1`、`0` 或 `1`。这适合快速检查某个 Pauli 可观测量是否被当前稳定子态确定。

---

## 支持范围

`StabilizerState` 适合以下操作：

- 单比特 Clifford 门：`H`、`S`、`Sdg`、`X`、`Y`、`Z`、`X2p`、`X2m`、`Y2p`、`Y2m`；
- 双比特 Clifford 门：`CX`、`CY`、`CZ`、`SWAP`；
- 稳定子语义下的测量、重置、采样和概率读取。

如果线路包含非 Clifford 操作，稳定子模拟器无法精确表达完整态。此时应回到 `Statevector` 或 `DensityMatrix`，或者先把非 Clifford 片段单独隔离出来分析。


---

## 下一步

·[Statevector 纯态模拟](1_statevector.md):当线路包含非 Clifford 门时，用纯态模拟器继续验证。  
·[Pauli、PauliString 与 Hamiltonian](5_pauli_and_hamiltonian.md):理解稳定子生成元背后的 PauliString 表示和 Pauli 期望值。  
·[用文本图调试线路](../5_visualization/1_draw_text.md):在稳定子线路较长时，先用文本图检查门序和控制位。
