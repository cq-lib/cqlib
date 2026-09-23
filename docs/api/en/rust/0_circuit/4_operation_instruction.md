# Operation / Instruction

Module paths:

- `cqlib_core::circuit::Instruction`
- `cqlib_core::circuit::Operation`
- `cqlib_core::circuit::ValueInstruction`
- `cqlib_core::circuit::ValueOperation`
- `cqlib_core::circuit::Directive`
- `cqlib_core::circuit::ClassicalDataOp`

Together these types form the operation-level intermediate representation in the `cqlib_core::circuit` module. They describe every operation in a circuit, including quantum gates, non-unitary instructions, classical data operations, structured control flow and special instructions such as delay.

Semantically, these types fall into two layers:

- Storage-layer IR: used for efficient internal storage in `Circuit`, and dependent on the circuit's internal parameter table;
- Construction-layer IR: used for import, export, serialization, compiler pass output and testing, where operations are as self-contained as possible.

---

## IR layers

| Type | Layer | Description |
| --- | --- | --- |
| `Instruction` | Storage-layer IR | Describes "which instruction is executed"; it can be a standard gate, a multi-controlled gate, a custom unitary gate, a sub-circuit gate, a directive, a classical data operation, classical control flow or `Delay`. |
| `Operation` | Storage-layer IR | Represents one operation inside a circuit, composed of `Instruction + qubits + CircuitParam + label`. Its parameters may reference the parameter table of the owning `Circuit`. |
| `ValueInstruction` | Construction-layer IR | Can wrap an ordinary `Instruction`, or a construction-layer classical control flow operation. |
| `ValueOperation` | Construction-layer IR | Represents a self-contained operation, composed of `ValueInstruction + qubits + ParameterValue + label`, suitable for passing between circuits, serialization and import. |

`Circuit::operations()` returns the internal storage-layer operation sequence:

```rust
pub fn operations(&self) -> &[Operation]
```

To read self-contained operations that can be detached from the circuit's internal parameter table, use:

```rust
pub fn index(&self, i: usize) -> Result<ValueOperation, CircuitError>
```

---

## `Instruction`

`Instruction` describes what an operation "does". It describes only the instruction type and the gate definition itself; it does not include which qubits the instruction acts on, nor the parameter values carried by a particular operation instance.

```rust
pub enum Instruction {
    Standard(StandardGate),
    McGate(Box<MCGate>),
    UnitaryGate(Box<UnitaryGate>),
    CircuitGate(Box<CircuitGate>),
    Directive(Directive),
    ClassicalData(ClassicalDataOp),
    ClassicalControl(ClassicalControlOp),
    Delay,
}
```

### 1. Variant descriptions

| Variant | Description |
| --- | --- |
| `Standard(StandardGate)` | A built-in Cqlib standard quantum gate, for example `H`, `CX`, `RZ`. |
| `McGate(Box<MCGate>)` | A multi-controlled standard gate, obtained by adding several control qubits to a base standard gate. |
| `UnitaryGate(Box<UnitaryGate>)` | A user-defined unitary gate, which can be defined by a matrix, a symbolic matrix or a circuit. |
| `CircuitGate(Box<CircuitGate>)` | A composite gate defined by a frozen circuit. |
| `Directive(Directive)` | A non-unitary instruction, for example `Barrier`, `Measure`, `Reset`. |
| `ClassicalData(ClassicalDataOp)` | A classical data operation, for example `store` or writing a measurement result. |
| `ClassicalControl(ClassicalControlOp)` | Structured control flow, for example `if`, `while`, `for`, `switch`. |
| `Delay` | A delay or idle-time instruction, usually used for scheduling or hardware timing semantics. |

In ordinary hand-written circuits, users usually do not need to construct `Instruction` directly. However, when writing compiler passes, IR importers, serialization tools or low-level tests, working with `Instruction` explicitly gives more precise control over the circuit structure.

### 2. Stable metadata

`Instruction` provides a set of stable identifiers that do not depend on `Debug` output and do not change with the display format, suitable for use in compiler passes, serialization and cache keys.

| Method | Returns | Description |
| --- | --- | --- |
| `name()` | `String` | The concrete operation name or the user-defined gate label. Unlike `instruction_type()`, it can distinguish different gates within the same category. |
| `instruction_type()` | `&'static str` | The instruction category name: `"standard"`, `"mcgate"`, `"unitary"`, `"circuit"`, `"directive"`, `"classical_data"`, `"classical_control"`, `"delay"`. |

The value of `name()` for each variant: standard gates take the gate name, multi-controlled gates take the multi-controlled gate name, unitary gates take the label, composite gates take the gate name, directives and classical operations take their own operation names, and `Delay` is always `"delay"`.

Category checks use a set of constant predicates, all evaluated at compile time:

| Predicate | Description |
| --- | --- |
| `is_standard()` | Whether the instruction is a standard gate instruction. |
| `is_mcgate()` | Whether the instruction is a multi-controlled gate instruction. |
| `is_unitary()` | Whether the instruction is a user-defined unitary gate instruction. |
| `is_circuit_gate()` | Whether the instruction is a composite gate instruction. |
| `is_directive()` | Whether the instruction is a non-unitary instruction. |
| `is_classical_data()` | Whether the instruction is a classical data operation. |
| `is_classical_control()` | Whether the instruction is structured control flow. |
| `is_delay()` | Whether the instruction is a delay instruction. |
| `is_quantum_gate()` | Whether the instruction is a unitary quantum gate, that is, one of the first four categories. |

### 3. Structural equality

`Instruction` implements `PartialEq`. Two instructions are equal if and only if their variants and internal definitions agree structurally; the comparison does not involve qubit binding or parameter values.

### 4. Common methods

| Method | Description |
| --- | --- |
| `has_measurement()` | Determine whether the current instruction or its recursive control flow body contains a measurement. |
| `reads_value(value)` | Determine whether the current instruction reads the given `ClassicalValue`. |
| `gate_arity()` | Return the number of qubits and the number of parameters the gate requires; non-gate instructions usually return `None`. |
| `matrix(params)` | Try to return the numeric matrix; non-unitary instructions or instructions without a supported matrix representation return `None`. |
| `inverse(params)` | Try to return the inverse instruction and inverse parameters; non-invertible instructions return `None`. |
| `control(num_new_ctrls)` | Try to promote the current instruction to a controlled instruction. |

### 5. `From` conversions

`Instruction` supports conversion from several underlying types, which makes it convenient to create instructions quickly when building IR or in compiler passes.

Common conversions include:

- `StandardGate`
- `Directive`
- `ClassicalControlOp`
- `ClassicalDataOp`

```rust
use cqlib_core::circuit::{Instruction, StandardGate};

let inst: Instruction = StandardGate::H.into();
```

---

## `Directive`

`Directive` represents a special circuit instruction that is not an ordinary unitary gate.

```rust
pub enum Directive {
    Barrier,
    Measure,
    Reset,
}
```

| Variant | Description |
| --- | --- |
| `Barrier` | A compilation or scheduling barrier that prevents operations on the affected qubits from being reordered across this boundary. |
| `Measure` | Computational basis measurement; it produces a classical result and is a non-unitary operation. |
| `Reset` | Reset the qubit to `\|0>`, which usually destroys the coherence of the original quantum state. |

`Directive` also provides a stable operation name interface:

```rust
pub const fn name(self) -> &'static str
```

It returns `"barrier"`, `"measure"` and `"reset"` respectively.

The public inversion interface of `Directive` is as follows:

```rust
pub fn inverse(&self) -> Option<Self>
```

`Barrier` can be regarded as its own inverse; `Measure` and `Reset` are not invertible, so `None` is returned.

```rust
use cqlib_core::circuit::Directive;

assert_eq!(Directive::Barrier.inverse(), Some(Directive::Barrier));
assert_eq!(Directive::Measure.inverse(), None);
assert_eq!(Directive::Reset.inverse(), None);
```

Note that `Directive` usually has no ordinary unitary matrix representation. A circuit containing `Measure` or `Reset` cannot be treated as a purely quantum gate circuit and passed directly to the matrix conversion interface.

---

## `ClassicalDataOp`

`ClassicalDataOp` represents classical data operations in a circuit, mainly used to describe writes to classical variables and the production of measurement results.

```rust
pub enum ClassicalDataOp {
    Store { target: ClassicalVar, value: ClassicalExpr },
    MeasureBit { result: ClassicalValue },
    MeasureBits { result: ClassicalValue },
}
```

| Variant | Description |
| --- | --- |
| `Store { target, value }` | Write a classical expression into a mutable classical variable. |
| `MeasureBit { result }` | Single-qubit measurement produces one classical value. |
| `MeasureBits { result }` | Multi-qubit measurement produces a bit vector classical value. |

The common access methods are as follows:

| Method | Returns | Description |
| --- | --- | --- |
| `target()` | `Option<ClassicalVar>` | Return the target variable when the operation is `Store`. |
| `result()` | `Option<ClassicalValue>` | Return the result value when the operation produces a measurement result. |
| `value()` | `Option<&ClassicalExpr>` | Return the written expression when the operation is `Store`. |

Ordinary users usually construct these operations through high-level `Circuit` methods, for example:

- `Circuit::store()`
- `Circuit::measure()`
- `Circuit::measure_bits()`
- `Circuit::measure_into()`
- `Circuit::measure_bits_into()`

---

## `Operation`

`Operation` is the storage-layer operation representation used internally by `Circuit`. It records one concrete application of an instruction in the current circuit.

```rust
pub struct Operation {
    pub instruction: Instruction,
    pub qubits: SmallVec<[Qubit; 3]>,
    pub params: SmallVec<[CircuitParam; 1]>,
    pub label: Option<Box<str>>,
}
```

Field descriptions:

| Field | Description |
| --- | --- |
| `instruction` | The instruction type executed by the current operation. |
| `qubits` | The logical qubits the current operation acts on. |
| `params` | The parameters carried by the current operation, represented with `CircuitParam`. |
| `label` | An optional metadata label that does not change the mathematical semantics of the operation. |

### `Operation::standard_gate`

```rust
pub fn standard_gate(&self) -> Option<StandardGate>
```

`Operation::standard_gate()` returns the corresponding standard gate definition when the operation is a standard gate, and `None` otherwise.

### `Operation::matrix`

```rust
pub fn matrix(&self) -> Result<Cow<'_, Array2<Complex64>>, CircuitError>
```

`Operation::matrix()` computes the numeric matrix of a single operation.

---

## `ValueInstruction`

`ValueInstruction` is the construction-layer instruction representation. It can wrap an ordinary storage-layer `Instruction`, or a construction-layer classical control flow object.

```rust
pub enum ValueInstruction {
    Instruction(Instruction),
    ClassicalControl(ValueClassicalControlOp),
}
```

The common methods are as follows:

| Method | Description |
| --- | --- |
| `from_instruction(inst)` | Wrap an ordinary `Instruction` as a `ValueInstruction`. |
| `is_classical_control()` | Determine whether it is construction-layer control flow. |
| `is_instruction()` | Determine whether it is an ordinary instruction. |
| `name()` | Return the concrete operation name or the user-defined gate label. |
| `instruction_type()` | Return the stable category name; construction-layer control flow is always `"classical_control"`, and the rest is delegated to the inner `Instruction`. |
| `is_standard()` / `is_mcgate()` / `is_unitary()` / `is_circuit_gate()` | Determine whether the instruction is a standard gate, a multi-controlled gate, a custom unitary gate or a composite gate respectively. |
| `is_directive()` / `is_classical_data()` / `is_delay()` | Determine whether the instruction is a non-unitary instruction, a classical data operation or a delay instruction respectively. |
| `standard_gate()` | Return the corresponding `StandardGate` when the instruction is a standard gate. |
| `directive()` | Return the corresponding `Directive` when the instruction is a non-unitary instruction. |
| `as_instruction()` | Read a reference to the inner `Instruction`. |
| `into_instruction()` | Consume the object and take out the inner `Instruction`. |

---

## `ValueOperation`

`ValueOperation` is the construction-layer self-contained operation representation, suitable for passing between circuits and for use as a boundary object for serialization, importers and compiler output.

```rust
pub struct ValueOperation {
    pub instruction: ValueInstruction,
    pub qubits: SmallVec<[Qubit; 3]>,
    pub params: SmallVec<[ParameterValue; 1]>,
    pub label: Option<Box<str>>,
}
```

Field descriptions:

| Field | Description |
| --- | --- |
| `instruction` | The construction-layer instruction, which can represent an ordinary instruction or control flow. |
| `qubits` | The list of acted-on qubits. |
| `params` | Self-contained parameters represented with `ParameterValue`, which can be a fixed value or a complete `Parameter`. |
| `label` | An optional label used only as metadata. |

### `ValueOperation::from_standard`

```rust
pub fn from_standard(
    gate: StandardGate,
    qubits: impl IntoIterator<Item = Qubit>,
    params: impl IntoIterator<Item = ParameterValue>,
) -> Self
```

```rust
use cqlib_core::circuit::{ParameterValue, Qubit, StandardGate, ValueOperation};

let op = ValueOperation::from_standard(
    StandardGate::RX,
    [Qubit::new(0)],
    [ParameterValue::from(0.5_f64)],
);
```

---

## `ParameterValue` and `CircuitParam`

Operation parameters use different representations in the construction layer and the storage layer.

| Type | Description |
| --- | --- |
| `ParameterValue` | The construction-layer parameter representation; it can be a fixed numeric value or a complete `Parameter` expression. |
| `CircuitParam` | The storage-layer parameter representation; it can be a fixed numeric value or an index into the parameter table of the owning `Circuit`. |

Common conversion examples:

```rust
use cqlib_core::circuit::{Parameter, ParameterValue};

let fixed = ParameterValue::from(0.5_f64);
let symbolic = ParameterValue::from(Parameter::symbol("theta"));
let also_symbolic = ParameterValue::from("phi");
```

---

## Building and importing a circuit manually

`ValueOperation` is often used to build circuits from external formats, or as the intermediate result of compiler pass output. The following example shows how to build an operation sequence manually and create a circuit through `Circuit::from_operations()`.

```rust
use cqlib_core::circuit::{
    Circuit, ParameterValue, Qubit, StandardGate, ValueOperation,
};

let ops = vec![
    ValueOperation::from_standard(StandardGate::H, [Qubit::new(0)], []),
    ValueOperation::from_standard(StandardGate::CX, [Qubit::new(0), Qubit::new(1)], []),
    ValueOperation::from_standard(
        StandardGate::RZ,
        [Qubit::new(1)],
        [ParameterValue::from("theta")],
    ),
];

let circuit = Circuit::from_operations(
    vec![Qubit::new(0), Qubit::new(1)],
    ops,
    None,
    None,
)?;

assert!(circuit.symbols().contains("theta"));

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## The `label` tag

`label` is optional metadata attached to an operation instance, used to record debugging information, the import source position, calibration tags, visualization names or compiler pass markers.

```rust
use cqlib_core::circuit::{ParameterValue, Qubit, StandardGate, ValueOperation};

let mut op = ValueOperation::from_standard(
    StandardGate::RZ,
    [Qubit::new(0)],
    [ParameterValue::from(0.25_f64)],
);

op.label = Some("calibrated-rz".into());
```

In compilation, whether to keep, modify or discard a `label` should be treated as a metadata handling policy, not as part of circuit semantics.

---

## Validation boundaries

`Circuit::append()` and `Circuit::from_operations()` check for common structural errors when appending or importing, including:

- whether the number of qubits matches the instruction arity;
- whether the parameter count matches the instruction definition;
- whether the qubits in an operation are duplicated;
- whether the qubits in an operation belong to the current circuit;
- whether fixed parameters are finite values;
- whether classical data handles belong to the current circuit;
- whether scopes, jumps and classical value reads in control flow satisfy the constraints.

For automatically generated IR, call the following before entering later compiler stages or backend execution:

```rust
circuit.validate()?;
```

This detects IR construction problems as early as possible and avoids errors being deferred to matrix conversion, device mapping or actual backend execution.
