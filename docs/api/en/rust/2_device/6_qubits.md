# Qubit Identifiers

`cqlib_core::device`

`LogicalQubit` and `PhysicalQubit` are the strongly typed qubit identifiers used by the device-side interfaces: the former identifies a logical qubit in a circuit, and the latter identifies a physical qubit on a device. Both types are newtype wrappers around `Qubit` with the same internal representation, but they are distinguished from each other at the type level.

## Import

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};
```

---

## LogicalQubit

The logical qubit identifier, used for logical qubits in a circuit.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[repr(transparent)]
pub struct LogicalQubit(Qubit);
```

Fields:

- The inner `Qubit` field is private and is not accessed directly through a public interface.

### Construction

- `LogicalQubit::new(id: u32) -> LogicalQubit`: construct from a numeric identifier; `const fn`.
- `LogicalQubit::from_qubit(qubit: Qubit) -> LogicalQubit`: wrap an existing circuit qubit; `const fn`.
- Implements `From<Qubit>`, equivalent to `from_qubit`.

### Methods

- `fn qubit(self) -> Qubit`: return the underlying circuit qubit; `const fn`.
- `fn id(self) -> u32`: return the numeric identifier; `const fn`.

### Trait implementations

- `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Ord`, `PartialOrd`: derived implementations; ordering and hashing are based on the numeric identifier.
- `Display`: outputs `L{id}`, for example `L0`.
- `From<Qubit> for LogicalQubit`: equivalent to `LogicalQubit::from_qubit`.
- `From<LogicalQubit> for Qubit`: equivalent to `LogicalQubit::qubit`.

Example:

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::LogicalQubit;

let qubit = Qubit::new(3);
let logical = LogicalQubit::from_qubit(qubit);

assert_eq!(logical.id(), 3);
assert_eq!(logical.qubit(), qubit);
assert_eq!(Qubit::from(logical), qubit);
assert_eq!(logical.to_string(), "L3");

let ordered = [LogicalQubit::new(2), LogicalQubit::new(0)];
assert!(ordered[1] < ordered[0]);
```

---

## PhysicalQubit

The physical qubit identifier, used for qubit positions on a device.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[repr(transparent)]
pub struct PhysicalQubit(Qubit);
```

Fields:

- The inner `Qubit` field is private and is not accessed directly through a public interface.

### Construction

- `PhysicalQubit::new(id: u32) -> PhysicalQubit`: construct from a numeric identifier; `const fn`.
- `PhysicalQubit::from_qubit(qubit: Qubit) -> PhysicalQubit`: wrap an identifier in qubit form; `const fn`.
- Implements `From<Qubit>`, equivalent to `from_qubit`.

### Methods

- `fn qubit(self) -> Qubit`: return the underlying qubit identifier; `const fn`.
- `fn id(self) -> u32`: return the numeric identifier; `const fn`.

### Trait implementations

- `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Ord`, `PartialOrd`: derived implementations; ordering and hashing are based on the numeric identifier.
- `Display`: outputs `P{id}`, for example `P100`.
- `From<Qubit> for PhysicalQubit`: equivalent to `PhysicalQubit::from_qubit`.
- `From<PhysicalQubit> for Qubit`: equivalent to `PhysicalQubit::qubit`.

Example:

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::PhysicalQubit;

let qubit = Qubit::new(11);
let physical = PhysicalQubit::new(qubit.id());

assert_eq!(physical.id(), 11);
assert_eq!(physical.qubit(), qubit);
assert_eq!(physical.to_string(), "P11");

assert!(PhysicalQubit::new(0) < PhysicalQubit::new(2));
```

---

## Role in layout

A layout maintains the mapping from logical qubits to physical qubits, which is the main scenario in which the two types are used together: logical qubits come from a circuit, physical qubits come from a device, and the mapping result is recorded with strongly typed identifiers. There is no conversion between the two types, nor can they be compared directly; the same numeric identifier does not mean that the two are the same identifier.

Example:

```rust
use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};
use std::collections::BTreeMap;

let logical = vec![LogicalQubit::new(0), LogicalQubit::new(1)];
let physical = vec![
    PhysicalQubit::new(100),
    PhysicalQubit::new(101),
    PhysicalQubit::new(102),
];

let init_map = [(LogicalQubit::new(1), PhysicalQubit::new(102))]
    .into_iter()
    .collect::<BTreeMap<_, _>>();

let layout = Layout::new(logical, physical, Some(init_map)).unwrap();

assert_eq!(layout.num_logical(), 2);
assert_eq!(layout.num_physical(), 3);
assert_eq!(layout.num_vacant_physical(), 1);
assert_eq!(
    layout.get_physical(LogicalQubit::new(0)),
    Some(PhysicalQubit::new(100))
);
assert_eq!(
    layout.get_physical(LogicalQubit::new(1)),
    Some(PhysicalQubit::new(102))
);
assert_eq!(
    layout.vacant_physical_qubits().collect::<Vec<_>>(),
    vec![PhysicalQubit::new(101)]
);
```
