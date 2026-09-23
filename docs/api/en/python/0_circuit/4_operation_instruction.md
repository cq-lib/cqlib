# Operation / Instruction

- `cqlib.circuit.Instruction`
- `cqlib.circuit.ValueInstruction`
- `cqlib.circuit.ValueOperation`
- `cqlib.circuit.Directive`

```python
from cqlib.circuit import Instruction, ValueInstruction, ValueOperation, Directive
```

`Instruction`, `ValueInstruction` and `ValueOperation` are the basic representations in `cqlib.circuit` for describing circuit operations. Semantically, `Instruction` describes "which instruction is executed", for example a standard gate, a composite gate, a custom unitary gate or a non-unitary instruction; `ValueOperation` describes "one concrete application of this instruction in a circuit", including which qubits it acts on, which parameters it carries, and whether it carries label information.

---

## Operation IR levels

Cqlib splits circuit operations into several levels, so that ordinary quantum gates, non-unitary instructions and structured control flow can all be supported.

| Type | Role |
| --- | --- |
| `Instruction` | A storage-layer instruction, representing concrete instruction types such as standard gates, multi-controlled gates, custom unitary gates, sub-circuit gates, `Directive` and `delay`. |
| `ValueInstruction` | A construction-layer instruction, which can wrap an ordinary `Instruction` or a structured control flow object `ClassicalControlOp`. |
| `ValueOperation` | A self-contained operation, combining a `ValueInstruction`, the qubits acted on, the parameter list and an optional label. |
| `Directive` | A non-unitary instruction type, used to represent special circuit semantics such as `barrier`, `measure` and `reset`. |

This layered design separates "the definition of an instruction" from "the application of that instruction in a circuit".

---

## `Instruction`

`Instruction` describes the instruction type corresponding to an operation. It only cares about "what to execute"; it does not contain the qubits that the instruction concretely acts on, nor does it represent its position in a circuit.

### 1. Creation interfaces

The commonly used static construction methods are as follows:

| Static method | Description |
| --- | --- |
| `Instruction.from_name(name)` | Create a standard gate instruction by standard gate name; name matching is case-insensitive; raises `ValueError` when the name matches no standard gate. |
| `Instruction.from_standard_gate(gate)` | Create a standard gate instruction from a `StandardGate`; `gate` must be an unbound definition without parameters, otherwise raises `CircuitError`. |
| `Instruction.from_mc_gate(gate)` | Create a multi-controlled gate instruction from an `MCGate`; its base gate must likewise be an unbound definition without parameters, otherwise raises `CircuitError`. |
| `Instruction.from_unitary_gate(gate)` | Create a custom unitary gate instruction from a `UnitaryGate`. |
| `Instruction.from_circuit_gate(gate)` | Create a sub-circuit composite gate instruction from a `CircuitGate`. |
| `Instruction.from_directive(directive)` | Create a non-unitary instruction from a `Directive`. |
| `Instruction.delay()` | Create a `delay` instruction. |

Example:

```python
from cqlib.circuit import Instruction, Directive, StandardGate

h_inst = Instruction.from_standard_gate(StandardGate.H())
barrier_inst = Instruction.from_directive(Directive.barrier())
delay_inst = Instruction.delay()
```

### 2. Attributes

| Attribute | Type | Description |
| --- | --- | --- |
| `name` | `str` | The readable name of the instruction. Standard gate instructions use canonical uppercase names, for example `"H"` and `"CX"`; `Directive` and `delay` instructions use lowercase names, for example `"barrier"`, `"measure"` and `"delay"`. |
| `instruction_type` | `str` | The instruction category, for example `"standard"`, `"mcgate"`, `"unitary"`, `"circuit"`, `"directive"`, `"classical_data"`, `"classical_control"` and `"delay"`. |
| `is_standard` | `bool` | Whether it is a standard gate instruction. |
| `is_mcgate` | `bool` | Whether it is a multi-controlled gate instruction. |
| `is_unitary` | `bool` | Whether it is a user-defined unitary gate instruction. |
| `is_circuit_gate` | `bool` | Whether it is a sub-circuit composite gate instruction. |
| `is_directive` | `bool` | Whether it is a non-unitary `Directive` instruction. |
| `is_classical_control` | `bool` | Whether it is a structured control flow instruction. |
| `is_classical_data` | `bool` | Whether it is an instruction related to classical data. |
| `is_delay` | `bool` | Whether it is a delay instruction. |
| `standard_gate` | `StandardGate / None` | When the instruction is a standard gate, return the internal standard gate object. |
| `directive` | `Directive / None` | When the instruction is a `Directive`, return the internal directive object. |

---

## `ValueInstruction`

`ValueInstruction` is a construction-layer instruction, used to represent ordinary instructions and structured control flow uniformly. It provides a unified entry for `ValueOperation`, so that a circuit operation can represent ordinary quantum gates and non-unitary instructions as well as control flow structures such as `if`, `while`, `for` and `switch`.

### 1. Creation interfaces

| Static method | Description |
| --- | --- |
| `ValueInstruction.from_instruction(instruction)` | Wrap an ordinary `Instruction` as a construction-layer instruction. |
| `ValueInstruction.from_classical_control(control)` | Wrap a `ClassicalControlOp` as a construction-layer control flow instruction. |

### 2. Attributes

| Attribute | Type | Description |
| --- | --- | --- |
| `is_instruction` | `bool` | Whether it wraps an ordinary `Instruction`. |
| `is_classical_control` | `bool` | Whether it wraps structured control flow. |
| `instruction` | `Instruction / None` | The ordinary instruction content. |
| `classical_control` | `ClassicalControlOp / None` | The control flow content. |
| `name` | `str` | The instruction name; returns the structure name when control flow is wrapped, for example `"if"` and `"while"`. |
| `instruction_type` | `str` | The instruction category, with the same values as `Instruction.instruction_type`; it is `"classical_control"` when control flow is wrapped. |
| `is_standard` / `is_mcgate` / `is_unitary` / `is_circuit_gate` / `is_directive` / `is_classical_data` / `is_delay` | `bool` | Category check attributes with the same names as on `Instruction`; all are `False` when control flow is wrapped. |
| `standard_gate` | `StandardGate / None` | When a standard gate instruction is wrapped, return the internal standard gate object. |
| `directive` | `Directive / None` | When a `Directive` instruction is wrapped, return the internal directive object. |

---

## `ValueOperation`

`ValueOperation` represents one complete operation in a circuit. It contains the instruction to execute, the qubits that instruction acts on, the parameters applied and an optional label.

The construction signature is as follows:

```python
ValueOperation(
    instruction: ValueInstruction,
    qubits: list[Qubit],
    params: list[float | Parameter] | None = None,
    label: str | None = None,
)
```

Here, `qubits` are the logical qubits this operation acts on; `params` are the numeric or symbolic parameters passed in when it is applied; `label` is optional user metadata, used for debugging, annotating the source or recording compiler information.

### 1. Factory methods

| Static method | Description |
| --- | --- |
| `from_instruction(instruction, qubits, params=None, label=None)` | Create an operation from an ordinary `Instruction`, the qubits acted on and explicit parameters. |
| `from_standard_gate(gate, qubits, label=None)` | Create an operation from a `StandardGate` that already contains parameter information. |
| `from_mc_gate(gate, qubits, label=None)` | Create an operation from an `MCGate` that already contains the base gate parameters. |
| `from_classical_control(control)` | Create a control flow operation from a `ClassicalControlOp`. |

Example:

```python
from cqlib import Qubit
from cqlib.circuit import ValueOperation, StandardGate

op = ValueOperation.from_standard_gate(
    StandardGate.RX(0.25),
    [Qubit(0)],
    label="rx-layer-0",
)
```

### 2. Attributes and methods

| Interface | Type | Description |
| --- | --- | --- |
| `instruction` | `ValueInstruction` | The construction-layer instruction corresponding to the operation. |
| `qubits` | `list[Qubit]` | The list of qubits the operation acts on. |
| `params` | `list[float \| Parameter]` | The parameter list carried by this operation. |
| `label` | `str \| None` | The optional label. |
| `name` | `str` | The operation name; a control flow operation returns the structure name, for example `"if"`. |
| `num_qubits` | `int` | The number of qubits this operation acts on. |
| `num_params` | `int` | The number of parameters carried by this operation. |
| `instruction_type` | `str` | The instruction category, with the same values as `Instruction.instruction_type`. |
| `is_standard` / `is_mcgate` / `is_unitary` / `is_circuit_gate` / `is_directive` / `is_classical_data` / `is_classical_control` / `is_delay` | `bool` | Category check attributes with the same names as on `Instruction`. |
| `matrix()` | `np.ndarray` | Return the unitary matrix corresponding to the operation; raises `ValueError` when it carries symbolic parameters or is a control flow operation. |


---

## `Directive`

`Directive` represents non-unitary circuit instructions.

| Static method | Description |
| --- | --- |
| `Directive.barrier()` | Create a barrier instruction, used to prevent the compiler from reordering operations on the related qubits across the boundary. |
| `Directive.measure()` | Create a computational basis measurement instruction. |
| `Directive.reset()` | Create a reset instruction, which resets a qubit. |

| Method | Description |
| --- | --- |
| `name()` | Return the stable lowercase instruction name: `"barrier"`, `"measure"` or `"reset"`. |
| `is_barrier()` | Determine whether it is a barrier instruction. |
| `is_measure()` | Determine whether it is a measurement instruction. |
| `is_reset()` | Determine whether it is a reset instruction. |
| `inverse()` | Return the inverse of the instruction; `barrier` returns itself, while `measure` and `reset` return `None`. |

Example:

```python
from cqlib.circuit import Directive

barrier = Directive.barrier()
assert barrier.is_barrier()
assert Directive.measure().inverse() is None
assert Directive.reset().inverse() is None
```

---

## `ValueOperation` and `Circuit`

When a circuit is constructed directly through the convenience methods provided by `Circuit`, for example `h()`, `cx()` and `rz()`, Cqlib automatically creates the corresponding `Instruction` and `ValueOperation` internally.

When operation-level IR needs to be constructed by hand, `ValueOperation` can be created explicitly and then restored to a circuit object through `Circuit.from_operations()`.

```python
from cqlib import Circuit, Qubit
from cqlib.circuit import ValueOperation, StandardGate

ops = [
    ValueOperation.from_standard_gate(StandardGate.H(), [Qubit(0)]),
    ValueOperation.from_standard_gate(StandardGate.CX(), [Qubit(0), Qubit(1)]),
]

circuit = Circuit.from_operations([Qubit(0), Qubit(1)], ops)
```

---

## The `label` tag

`label` is readable metadata attached to an operation instance. It does not change the matrix, the quantum semantics or the execution result of the gate, and is mainly used for debugging, visualization, recording the source or identifying operations generated at a compilation stage.

```python
from cqlib import Qubit
from cqlib.circuit import ValueOperation, StandardGate

op = ValueOperation.from_standard_gate(
    StandardGate.RZ(0.1),
    [Qubit(0)],
    label="calibrated-rz",
)
```
