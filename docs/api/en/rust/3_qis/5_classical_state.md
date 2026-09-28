# Classical State

`cqlib_core::qis::state`

This page introduces the runtime classical data in `cqlib_core::qis`: `RuntimeValue` is a typed runtime value, and `ClassicalState` stores and indexes these values by circuit handle.

The circuit IR stores classical handles rather than concrete values. When a circuit containing measurement or classical storage is executed, the runtime classical data fills these handles with actual values: immutable classical values are produced by measurement, and mutable classical variables are written by store statements. The identity of a handle is determined jointly by the circuit `CircuitId` and the index within the table, so handles from different circuits are not interchangeable.

## Import

```rust
use cqlib_core::qis::{ClassicalState, RuntimeValue};
```

Both types are also located in `cqlib_core::qis::state`; the two paths are equivalent.

---

## RuntimeValue

```rust
pub enum RuntimeValue {
    Bit(bool),
    Bool(bool),
    UInt { width: u32, value: u128 },
    BitVec { width: u32, bits: Outcome },
}
```

Variants:

| Variant | Payload | Description |
| --- | --- | --- |
| `RuntimeValue::Bit(bool)` | `bool` | A single bit, typically representing a single-qubit measurement result. |
| `RuntimeValue::Bool(bool)` | `bool` | A logical boolean, used for control flow conditions. |
| `RuntimeValue::UInt { width, value }` | `width: u32`, `value: u128` | An unsigned integer of the given bit width; the value is held in a `u128`. |
| `RuntimeValue::BitVec { width, bits }` | `width: u32`, `bits: Outcome` | A bit vector of the given bit width; the bits are carried in the device result type `Outcome`. |

The bit order of `BitVec` is consistent with the device result layer: bit index `0` is the least significant bit, and when formatted as a string the most significant bit appears leftmost.

### Methods

- `fn ty(&self) -> ClassicalType`: return the static classical type corresponding to this runtime value.
- `fn to_bitstring(&self) -> Option<String>`: return the bitstring with the most significant bit on the left; `Bit` and `BitVec` return `Some`, while `Bool` and `UInt` return `None`.

### Other behavior

- Supports `Debug` and `Clone`.
- Comparison and hashing are by value: two values are equal only when the type, bit width and payload are all the same, so `RuntimeValue::Bit(true)` and `RuntimeValue::Bool(true)` are not equal.

Example:

```rust
use cqlib_core::device::Outcome;
use cqlib_core::qis::RuntimeValue;

let bit = RuntimeValue::Bit(true);
assert_eq!(bit.to_bitstring().as_deref(), Some("1"));

let uint = RuntimeValue::UInt { width: 8, value: 42 };
assert_eq!(uint, RuntimeValue::UInt { width: 8, value: 42 });
assert_eq!(uint.to_bitstring(), None);

// 类型不同则值不相等，即使载荷相同
assert_ne!(RuntimeValue::Bit(true), RuntimeValue::Bool(true));

let bit_vec = RuntimeValue::BitVec {
    width: 3,
    bits: Outcome::from_bitstring("101").unwrap(),
};
assert_eq!(bit_vec.to_bitstring().as_deref(), Some("101"));
```

---

## ClassicalState

```rust
pub struct ClassicalState {
    // 字段不公开
}
```

Stores the runtime classical data of one circuit: a table of immutable classical values, a table of mutable classical variables, and their respective static type tables. The indices of the two correspond one-to-one with the handle indices in the circuit.

### Methods

- `fn value(&self, value: ClassicalValue) -> Option<&RuntimeValue>`: return the runtime result of a given immutable classical value.
- `fn var(&self, var: ClassicalVar) -> Option<&RuntimeValue>`: return the current runtime value of a given mutable classical variable.

Both methods return `None` when the handle belongs to another circuit, the type does not match, the index is beyond the corresponding table, or the corresponding value has not yet been produced (not measured) or not yet stored.

### How it is obtained

`ClassicalState` is not used as an ordinary constructed object; it is returned by the circuit execution entry point of the stabilizer simulator:

```rust
pub fn run_circuit(circuit: &Circuit) -> Result<CircuitExecutionResult, QisError>
```

`CircuitExecutionResult` contains the final quantum state and the classical data of this execution:

```rust
pub struct CircuitExecutionResult {
    pub state: StabilizerState,
    pub classical: ClassicalState,
}
```

### Other behavior

- Supports `Debug` and `Clone`.

Example:

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::{RuntimeValue, StabilizerState};

let mut c = Circuit::new(2);
c.h(Qubit::new(0)).unwrap();
c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
let left = c.measure(Qubit::new(0)).unwrap();
let right = c.measure(Qubit::new(1)).unwrap();

let result = StabilizerState::run_circuit(&c).unwrap();
let Some(RuntimeValue::Bit(left_bit)) = result.classical.value(left.value()) else {
    panic!("expected first Bell measurement to produce a bit");
};
let Some(RuntimeValue::Bit(right_bit)) = result.classical.value(right.value()) else {
    panic!("expected second Bell measurement to produce a bit");
};

// Bell 态上的两次测量结果必须保持关联
assert_eq!(left_bit, right_bit);
```

The result of a multi-qubit measurement is packed into a `BitVec` in the little-endian input order of `Circuit::measure_bits()`:

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::{RuntimeValue, StabilizerState};

let mut c = Circuit::new(3);
c.x(Qubit::new(0)).unwrap();
c.x(Qubit::new(2)).unwrap();
let measured = c
    .measure_bits([Qubit::new(2), Qubit::new(1), Qubit::new(0)])
    .unwrap();

let result = StabilizerState::run_circuit(&c).unwrap();
let Some(RuntimeValue::BitVec { width, bits }) = result.classical.value(measured.value()) else {
    panic!("expected BitVec measurement result");
};
assert_eq!(*width, 3);
assert_eq!(bits.to_bitstring(*width as usize), "101");
```

---

## Validation and error handling

`value()` and `var()` express a missing lookup with `Option` and do not return errors themselves. Validation on the write side happens during circuit execution; a type mismatch or a handle that does not belong to the current circuit is returned as a `QisError`:

| Error | When it occurs |
| --- | --- |
| `QisError::CircuitError` | A classical handle that does not belong to the circuit is used during circuit execution, corresponding to `CircuitError::ForeignClassicalHandle`. |
| `QisError::UnsupportedOperation` | The type of a classical value or classical variable is inconsistent with the type registered in the circuit IR, or the type of a runtime value cannot take part in that expression operation. |

Because lookups use the `CircuitId` and index carried by the handle, a cross-circuit read only yields `None` and never reads the data of another circuit.

---

## Related pages

- [QIS Overview](0_overview.md): module overview and glossary.
- [Pauli Operators](6_pauli.md): the bit order convention used when computing Pauli string expectation values from measurement probabilities.
