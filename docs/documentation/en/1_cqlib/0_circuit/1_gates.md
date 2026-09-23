# Quantum gates and instructions

`cqlib.circuit` provides a rich set of quantum gates and circuit instructions for constructing various quantum programs, from basic quantum circuits to parameterized algorithm modules. This page describes in detail the basic usage of quantum gates and instructions in Cqlib, including standard gates, parameterized gates, multi-qubit gates, multi-controlled gates, custom unitary gates, sub-circuit gates and non-unitary instructions such as measurement, reset and barrier.

This page covers how to select the appropriate gate type according to algorithm requirements and add it to a quantum circuit correctly. Common gates can be added quickly through the convenience methods provided by `Circuit`, and more complex gate operations can also be described explicitly with objects such as `StandardGate`, `MCGate`, `UnitaryGate` and `CircuitGate`.

---

## Standard gates

Standard gates are the basic gate set built into Cqlib, covering common single-qubit gates, multi-qubit gates, parameterized rotation gates and some hardware-related gates. Standard gates can be applied in the following two ways:

1. Call the shortcut methods provided by `Circuit` directly;
2. Construct a `StandardGate` object explicitly and append it to the circuit through `append_gate()`.

```python
from cqlib import Circuit
from cqlib.circuit import StandardGate

c = Circuit(1)
c.h(0)
c.append_gate(StandardGate.X(), [0])
c.append_gate(StandardGate.RZ(0.5), [0], label="phase-correction")
```

`StandardGate` objects support operations such as attribute queries, parameter binding, matrix computation, inversion and conversion into a multi-controlled gate.

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

The commonly used standard gates are introduced below by category, according to the number of qubits a gate acts on and its parameter form.


### 1. Single-qubit non-parametric gates

Single-qubit non-parametric gates are the most basic class of standard gates for constructing quantum circuits. They act on a single qubit and require no angle parameter, and the matrix form and effect of the gate are already fixed at definition time.

These gates perform basic transformations of quantum states, for example using the `H` gate to construct a superposition state, the `X` gate to flip a bit, the `Z`, `S` and `T` gates to adjust the phase, or the Clifford gate set to construct basic circuits that are convenient for analysis and compilation and optimization.

In actual use, single-qubit non-parametric gates usually serve as the basic building blocks of more complex circuits. They can be used alone for state preparation, phase correction and circuit debugging, or combined with two-qubit gates and parameterized rotation gates to construct Bell states, GHZ states, variational circuits, error-checking circuits and hardware native gate decomposition results.

| `Circuit` method | `StandardGate` | Description |
|---|---|---|
| `i(q)` | `I` | Identity gate |
| `h(q)` | `H` | Hadamard gate |
| `x(q)` | `X` | Pauli-X / NOT gate |
| `y(q)` | `Y` | Pauli-Y gate |
| `z(q)` | `Z` | Pauli-Z gate |
| `s(q)` | `S` | Phase gate |
| `sdg(q)` | `SDG` | Inverse of the `S` gate |
| `t(q)` | `T` | `pi/4` phase gate |
| `tdg(q)` | `TDG` | Inverse of the `T` gate |
| `x2p(q)` | `X2P` | `sqrt(X)`, by convention `X^(+1/2)` |
| `x2m(q)` | `X2M` | Inverse of `sqrt(X)` |
| `y2p(q)` | `Y2P` | `sqrt(Y)` |
| `y2m(q)` | `Y2M` | Inverse of `sqrt(Y)` |

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

Note that `i(q)` and `delay(q, duration)` have different semantics. `i(q)` denotes the standard identity gate in a quantum circuit; `delay(q, duration)` denotes idle waiting on the hardware timeline, and is usually used to preserve scheduling or timing semantics.

### 2. Single-qubit parameterized gates

Single-qubit parameterized gates are parameterized quantum gates applied on a single qubit, usually controlling the rotation axis and rotation magnitude of a quantum state on the Bloch sphere through one or more angle parameters. Unlike fixed gates such as `H` and `X`, the actual effect of a parameterized gate is determined by the numeric or symbolic parameters passed in, so the circuit behavior can be changed by adjusting the parameters while the circuit structure stays unchanged.

In Cqlib, a single-qubit parameterized gate accepts either an ordinary numeric parameter or a Parameter expression. The former is suitable for constructing circuits whose angles are already determined; the latter is suitable for constructing parameterized circuit templates and, in a later algorithm flow, for parameter binding, parameter sweeps, optimization iterations or symbolic matrix validation.

Parameterized gates are widely used in scenarios such as VQE, QAOA, quantum machine learning, variational circuit design and hardware calibration. For example, in variational quantum algorithms the circuit structure usually stays unchanged, while the optimizer keeps updating the parameter values in rotation gates to search for a better quantum state or objective function value.

| `Circuit` method | `StandardGate` | Parameters | Description |
|---|---|---|---|
| `rx(q, theta)` | `RX` | 1 | Rotation around the X axis |
| `ry(q, theta)` | `RY` | 1 | Rotation around the Y axis |
| `rz(q, theta)` | `RZ` | 1 | Rotation around the Z axis |
| `phase(q, lambda_)` | `Phase` | 1 | Phase gate |
| `u(q, theta, phi, lambda_)` | `U` | 3 | General single-qubit gate |
| `xy(q, theta)` | `XY` | 1 | Single-qubit gate of the XY family |
| `xy2p(q, theta)` | `XY2P` | 1 | Positive half-angle variant of `XY` |
| `xy2m(q, theta)` | `XY2M` | 1 | Negative half-angle variant of `XY` |
| `rxy(q, theta, phi)` | `RXY` | 2 | Specifies the rotation axis in the XY plane |

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

If the circuit still contains unbound symbolic parameters, a numeric matrix cannot be obtained directly with `to_matrix()`. In this case, bind the parameters first through `assign_parameters()`, or use `to_symbolic_matrix()` to keep the symbolic expressions.

`StandardGate.GPhase` is used to represent the global phase. At the circuit level, a more commonly used approach is to set the global phase of `Circuit`:

```python
from cqlib import Circuit, Parameter

c = Circuit(1)
c.set_global_phase(Parameter("alpha"))
print(c.global_phase)
```

### 3. Two-qubit and three-qubit gates

Two-qubit and three-qubit gates describe the interaction between multiple qubits, and are core components for constructing entangled states, controlled logic and the core structures of quantum algorithms. Unlike single-qubit gates, which only change the state of a single qubit, multi-qubit gates can establish correlations between different qubits: for example, `CX`, `CZ` and other controlled gates construct Bell states, GHZ states and various entangled circuits, and `SWAP` can adjust the logical positions between qubits.

In actual quantum algorithms, multi-qubit gates usually play the role of "connecting" the information of different qubits. For example, QAOA commonly uses two-qubit parameterized gates such as `RZZ` to express the interaction terms in the problem Hamiltonian; quantum Fourier transform and phase estimation algorithms use controlled-phase operations; and error-correction and verification circuits often use gates such as `CX` and `CCX` to implement conditional flips and ancilla control.

| `Circuit` method | `StandardGate` | Parameters | Description |
|---|---|---|---|
| `cx(control, target)` | `CX` | 0 | Controlled X gate, CNOT |
| `cy(control, target)` | `CY` | 0 | Controlled Y gate |
| `cz(control, target)` | `CZ` | 0 | Controlled Z gate |
| `swap(a, b)` | `SWAP` | 0 | Swaps the states of two qubits |
| `ccx(c1, c2, target)` | `CCX` | 0 | Toffoli gate |
| `rxx(a, b, theta)` | `RXX` | 1 | `XX` Pauli rotation |
| `ryy(a, b, theta)` | `RYY` | 1 | `YY` Pauli rotation |
| `rzz(a, b, theta)` | `RZZ` | 1 | `ZZ` Pauli rotation |
| `rzx(a, b, theta)` | `RZX` | 1 | `ZX` Pauli rotation |
| `crx(control, target, theta)` | `CRX` | 1 | Controlled `RX` gate|
| `cry(control, target, theta)` | `CRY` | 1 | Controlled `RY` gate|
| `crz(control, target, theta)` | `CRZ` | 1 | Controlled `RZ` gate|
| `fsim(a, b, theta, phi)` | `FSIM` | 2 | fSim two-qubit gate |

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

When adding a multi-qubit gate, Cqlib performs validity checks on the operands, including whether the target qubits have already been registered in the circuit, whether the same operation references the same qubit more than once, and whether the number of parameters passed in is consistent with the gate definition. If a check fails, the system raises `CircuitError` or `ParameterError` to prompt the user to correct the qubit index, the operand or the parameter configuration.

---

## Gate matrices and gate inversion

In quantum circuit analysis, algorithm validation and compilation conversion, the matrix representation of a gate is an important basis for understanding its mathematical effect. Cqlib supports computing matrices directly for standard gates and multi-controlled gates, so that the dimension of a gate can be checked, the unitarity of a gate verified, and the specific effect of a gate on a quantum state analyzed.

When computing a matrix, note the following:

- For a gate that contains no symbolic parameter, `matrix()` can be called directly to obtain its numeric matrix.
- For a gate that contains symbolic parameters, concrete numeric parameters need to be provided when computing the matrix, or the gate object needs to be made numeric first through parameter binding before the matrix is computed.

This preserves the flexibility of parameterized gates in algorithm templates and also gives a definite matrix result when numeric validation is needed.

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

In addition, Cqlib also supports performing inversion on invertible gates.

- For common self-inverse gates, the inversion result is the same as the original gate.
- For parameterized gates such as rotation gates, inversion usually amounts to negating the rotation angle.

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

The inversion rules of common gates are as follows:

- `H`, `X`, `Y`, `Z`, `CX`, `CY`, `CZ`, `SWAP` and `CCX` are self-inverse gates.
- `S` and `SDG` are mutual inverses, and `T` and `TDG` are mutual inverses.
- The inverse of a rotation gate is usually obtained by negating the angle, for example `RX(theta)^† = RX(-theta)`.
- The inverse of `U(theta, phi, lambda)` transforms the parameters according to the matrix definition.
- `Barrier` does not change the quantum state and can be regarded as its own inverse; `Measure` and `Reset` have no ordinary unitary inverse operation.

---

## Multi-controlled gates

`MCGate` is used to extend an existing `StandardGate` into a multi-controlled gate, that is, to add one or more control qubits on top of the original gate. The target gate acts on the corresponding target qubit only when all control qubits satisfy the specified control condition. This mechanism is commonly used to construct Toffoli gates, multi-controlled phase gates, oracles, conditional flip operations and controlled subroutines in quantum algorithms.

When appending an `MCGate` to a circuit, the qubit order needs to be passed in according to the convention: **all control qubits first, then the target qubits that the target gate acts on**.

For example, for a three-control `X` gate, the first three qubits are control qubits and the last qubit is the target qubit. This keeps the semantics of the multi-controlled gate clear and also facilitates later gate decomposition, compilation and optimization and hardware mapping.


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

Besides constructing an `MCGate` explicitly, the `control()` method can also be called directly on a standard gate object to generate a multi-controlled gate.

```python
from cqlib.circuit import StandardGate

ccx = StandardGate.X().control(2)
controlled_cx = StandardGate.CX().control(1)
```

`StandardGate.CX().control(1)` means adding one more control qubit on top of the existing `CX` gate, so its total number of control qubits is 2, which is equivalent to the three-qubit `CCX` in control structure.

---

## Custom unitary gates

When an algorithm needs a unitary operation outside the built-in standard gates, `UnitaryGate` can be used to define a custom unitary gate. Custom unitary gates are suitable for representing special oracles in an algorithm, problem-related transforms, hardware-specific gates and quantum operations with a known matrix form.

When defining a `UnitaryGate`, the gate name and the number of qubits it acts on need to be specified explicitly. If the gate is defined with a matrix, the matrix dimension must be `2^n × 2^n`, where `n` denotes the number of qubits the gate acts on, that is, `num_qubits`. Cqlib checks whether the matrix dimension matches the number of qubits according to the gate definition.

```python
import numpy as np
from cqlib import Circuit
from cqlib.circuit import UnitaryGate

h_matrix = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)
custom_h = UnitaryGate("CustomH", 1).with_matrix(h_matrix)

c = Circuit(2)
c.append_unitary_gate(custom_h, [0])
```

Besides a numeric matrix, a custom unitary gate can also be defined with a symbolic matrix. A symbolic matrix is suitable for describing a gate family with parameters: the symbolic form of the gate can be defined first, and concrete parameter values passed in later from different circuits or different positions. This approach is commonly used in parameterized oracles, tunable phase gates, algorithm templates and symbolic validation scenarios.

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

Note that `UnitaryGate` is better suited to quantum operations that already have a clear matrix definition. If the goal is only to add an existing sub-circuit to other circuits as a reusable module, `CircuitGate` is usually more recommended. `CircuitGate` can be obtained by encapsulating a sub-circuit directly, which both avoids writing a matrix by hand and facilitates later decomposition, parameter binding and circuit structure analysis.

---

## Sub-circuit gate `CircuitGate`

`CircuitGate` is used to encapsulate an existing sub-circuit as a reusable composite gate. Unlike `UnitaryGate`, which describes gate behavior through a matrix, `CircuitGate` keeps the structural information of the sub-circuit, so it is better suited to expressing algorithm modules, oracles, ansatz blocks, repeated circuit structures and other quantum program fragments that need to be reused in multiple positions.

In actual use, a sub-circuit can be constructed first and then converted into a `CircuitGate` through `to_gate(name)`. The resulting composite gate can be appended to other circuits like an ordinary gate, and can be expanded back into the original sub-circuit operations through `decompose()` when needed, which facilitates later circuit analysis, parameter binding, compilation and optimization or IR export.

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

For a `CircuitGate` with parameters, concrete parameter values can be passed in when it is appended to a circuit. The parameters are bound positionally in the order of the symbols recorded in `gate.symbols`; if `params` is not passed, the original symbolic parameters in the sub-circuit are kept, so the composite gate remains in parameterized form.

```python
print(block.name)
print(block.num_qubits)
print(block.num_params)
print(block.symbols)
```

Besides `to_gate(name)`, a composite gate can also be constructed explicitly with `FrozenCircuit` and `CircuitGate`.

```python
from cqlib import Circuit
from cqlib.circuit import CircuitGate, FrozenCircuit

sub = Circuit(1)
sub.h(0)

frozen = FrozenCircuit(sub.qubits, sub.operations)
gate = CircuitGate("HadamardBlock", frozen)
```

---

## Directive: non-unitary instructions

Besides ordinary quantum gates, a quantum circuit may also contain special instructions such as measurement, reset, barrier and delay. Such instructions usually cannot correspond to an ordinary unitary matrix, so they are represented uniformly by `Directive` in Cqlib.

`Directive` is mainly used to describe auxiliary semantics during circuit execution. For example, `barrier` constrains gate reordering during compilation and optimization, `measure` reads out quantum state information as a classical result, `reset` reinitializes a qubit to |0>, and `delay` preserves the idle-wait semantics in hardware time scheduling.

Common non-unitary instructions can be added to a circuit directly through the interfaces provided by `Circuit`:

| `Circuit` method | Instruction name | Description |
|---|---|---|
| `barrier(qubits)` | `Barrier` | Prevents the compiler from reordering gates across the specified qubits |
| `measure(qubit)` | `measure_bit` | Measures a single qubit and returns a `Measurement` |
| `measure_bits(qubits)` | `measure_bits` | Measures multiple qubits and returns a bit-vector measurement value |
| `reset(qubit)` | `Reset` | Resets a qubit |
| `delay(qubit, duration)` | `delay` | Keeps idle on the hardware timeline |

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

Note that non-unitary instructions affect the mathematical representation of a circuit and the way it is processed later:

- `barrier` itself does not change the quantum state and is only used to express a compilation constraint. Therefore, when a circuit contains no other non-unitary operation, a circuit with `barrier` can still be used for matrix conversion.
- `measure`, `measure_into` and `measure_bits` introduce quantum measurement and classical results, so the circuit can no longer be represented completely by a single unitary matrix.
- `reset` reinitializes a qubit to a fixed state; it is a non-unitary operation and likewise cannot be described by an ordinary unitary matrix.
- `delay` is mainly used to preserve time semantics at the hardware execution or scheduling level.

Therefore, before performing `to_matrix()`, circuit equivalence validation or gate-level optimization, first confirm whether the circuit contains special instructions such as measurement, reset and delay.

---

## Low-level instructions and ValueOperation

In some lower-level or more automated development scenarios, Cqlib supports constructing `Instruction` and `ValueOperation` explicitly, which provides a more flexible development approach. For example, when writing a circuit deserializer, an IR converter, compilation and optimization tests, an automated circuit generation tool or a custom front-end interface, this approach can be used to describe directly the instruction type, acting qubits, parameter list and label information of a certain operation.

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

Semantically, `Instruction` is used to describe "what type of instruction is executed", while `ValueOperation` is used to describe one concrete application of this instruction in a circuit, including which qubits it acts on, which parameters it uses, and whether it carries an extra label.

This distinction allows Cqlib to reuse a unified data model across high-level circuit construction, low-level operation representation, IR conversion, compilation and optimization and dynamic control flow handling.

---

## Next steps

- [Circuit structure and construction](2_structures.md): understand the lifecycle, indexing, composition and operation representation of `Circuit`.
- [Parameter system](3_parameters.md): learn about parameter expressions, parameter binding, expression simplification, symbolic differentiation and symbolic matrices.
- [Circuit analysis and transformation](4_circuit_analysis.md): use tools such as inversion, decomposition, matrix conversion and operation inspection.
