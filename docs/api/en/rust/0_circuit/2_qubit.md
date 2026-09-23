# Qubit

`cqlib_core::circuit::Qubit`

```rust
use cqlib_core::circuit::Qubit;
```

`Qubit` is a lightweight handle representing a logical qubit in the Rust core. It stores a `u32` number internally, used to identify a logical qubit stably in quantum circuits, compilation IR, mapping tables and set structures.

---

## Creating a `Qubit`

```rust
pub const fn new(id: u32) -> Self
```

`Qubit::new(id)` creates a logical qubit from a `u32` number. Because the input type is already an unsigned 32-bit integer, the constructor itself cannot fail, and it can be used in a constant context.

```rust
use cqlib_core::circuit::Qubit;

let q0 = Qubit::new(0);
let q5 = Qubit::new(5);
```
---

## Accessing the number

`Qubit` provides two commonly used number access interfaces:

| Method | Returns | Description |
| --- | --- | --- |
| `id()` | `u32` | Return the raw internal number. |
| `index()` | `usize` | Convert the number to `usize`, which is convenient in scenarios that require an array index type. |

```rust
use cqlib_core::circuit::Qubit;

let q = Qubit::new(12);

assert_eq!(q.id(), 12);
assert_eq!(q.index(), 12usize);
assert_eq!(format!("{q}"), "Q12");
```

---

## Integer conversion

`Qubit` supports direct conversion from unsigned integer types that cannot overflow. For example, `u8`, `u16` and `u32` can be converted into `Qubit` through `From`.

```rust
use cqlib_core::circuit::Qubit;

let a: Qubit = 0u8.into();
let b: Qubit = 10u16.into();
let c: Qubit = 20u32.into();

assert_eq!(a, Qubit::new(0));
assert_eq!(b, Qubit::new(10));
assert_eq!(c, Qubit::new(20));
```

For integer types that may be negative or may exceed the `u32` range, use `TryFrom`: the signed types `i8`, `i16`, `i32` and `isize`, and the unsigned type `usize`. On 64-bit systems `usize` and `isize` can represent values beyond `u32`; a failed conversion is not silently truncated, but returns an error.

```rust
use cqlib_core::circuit::{Qubit, QubitError};

assert_eq!(Qubit::try_from(3i32).unwrap(), Qubit::new(3));

assert!(matches!(
    Qubit::try_from(-1i32),
    Err(QubitError::NegativeIndex(-1))
));

assert!(matches!(
    Qubit::try_from(usize::MAX),
    Err(QubitError::IndexTooLarge(_))
));
```

Common conversion errors are as follows:

| Error | Description |
| --- | --- |
| `QubitError::NegativeIndex(i128)` | The input is a signed integer whose value is negative. |
| `QubitError::IndexTooLarge(u128)` | The input value cannot be represented as `u32`. |

---

## Comparison, ordering and hashing

Both comparison and hashing of `Qubit` are based on the internal number. That is, two `Qubit` values with the same number are treated as the same logical qubit handle.

```rust
use std::collections::{BTreeSet, HashMap};
use cqlib_core::circuit::Qubit;

assert_eq!(Qubit::new(0), Qubit::new(0));
assert!(Qubit::new(0) < Qubit::new(1));

let mut map = HashMap::new();
map.insert(Qubit::new(0), "ancilla");
assert_eq!(map.get(&Qubit::new(0)), Some(&"ancilla"));

let mut set = BTreeSet::new();
set.insert(Qubit::new(2));
set.insert(Qubit::new(0));
set.insert(Qubit::new(1));

let ordered: Vec<_> = set.into_iter().collect();
assert_eq!(ordered, vec![Qubit::new(0), Qubit::new(1), Qubit::new(2)]);
```

---

## Logical numbering and storage position

The number of a `Qubit` is a logical identifier. In a circuit, the actual order of qubits is determined by the qubit list stored in `Circuit`; in a matrix or statevector, the axis order is likewise determined by the qubit ordering convention of the relevant conversion interface.

For example, the following circuit contains only two qubits, but their logical numbers are `10` and `20` respectively:

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let circuit = Circuit::from_qubits(vec![Qubit::new(10), Qubit::new(20)])?;

assert_eq!(circuit.num_qubits(), 2);
assert_eq!(circuit.qubits(), vec![Qubit::new(10), Qubit::new(20)]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```
