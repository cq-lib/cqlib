# MCGate

`cqlib_core::circuit::MCGate`

```rust
use cqlib_core::circuit::{MCGate, StandardGate};
```

`MCGate` represents a multi-controlled gate obtained by "adding several control qubits in front of a standard gate". It takes a `StandardGate` as the base gate and adds new control qubits in front of the qubits the base gate originally acts on. The base gate acts on the target qubits only when all control qubits satisfy the control condition.

---

## Constructor

```rust
pub fn new(num_controls: u8, gate: StandardGate) -> Self
```

`MCGate::new(num_controls, gate)` adds `num_controls` control qubits in front of the base standard gate `gate`.

| Parameter | Description |
| --- | --- |
| `num_controls` | The number of added control qubits. |
| `gate` | The base standard gate being controlled. |

```rust
use cqlib_core::circuit::{MCGate, StandardGate};

let ccx = MCGate::new(2, StandardGate::X);
let mch = MCGate::new(3, StandardGate::H);
```

In the example above, `MCGate::new(2, StandardGate::X)` represents a doubly controlled `X` gate whose semantics are equivalent to the common Toffoli gate; `MCGate::new(3, StandardGate::H)` represents a triply controlled `H` gate.

---

## Attributes and methods

`MCGate` provides a set of metadata methods for querying the number of control qubits, the total number of qubits it acts on, the number of parameters and the base gate type.

| Method | Returns | Description |
| --- | --- | --- |
| `num_ctrl_qubits()` | `usize` | Return the total number of control qubits, including the added control qubits and the control qubits built into the base gate. |
| `num_qubits()` | `usize` | Return the total number of qubits the multi-controlled gate acts on. |
| `num_params()` | `usize` | Return the number of parameters the base gate requires. |
| `base_gate()` | `&StandardGate` | Return the base standard gate being controlled. |
| `name()` | `String` | Return the stable operation name: the base gate name when no control qubit is added, otherwise `C<number of added control qubits>-<base gate name>`. |

```rust
use cqlib_core::circuit::{MCGate, StandardGate};

let gate = MCGate::new(1, StandardGate::CX);

assert_eq!(gate.num_ctrl_qubits(), 2); // 新增 1 个控制位 + CX 自带 1 个控制位
assert_eq!(gate.num_qubits(), 3);
assert_eq!(gate.num_params(), 0);
assert_eq!(*gate.base_gate(), StandardGate::CX);
```

---

## Order of control and target qubits

When an `MCGate` is applied, the qubit order has clear semantics:

```text
[new_control_0, new_control_1, ..., base_gate_qubit_0, base_gate_qubit_1, ...]
```

That is, the added control qubits always come first, followed by the qubits the base gate itself requires. If the base gate itself already carries control qubits, the control qubits inside the base gate keep the original order of that standard gate.

For example, `MCGate::new(2, StandardGate::X)` acts on three qubits:

```text
[control_0, control_1, target]
```

The first two are the added control qubits, and the last one is the target qubit of the `X` gate.

And `MCGate::new(1, StandardGate::CX)` acts on three qubits:

```text
[new_control, cx_control, cx_target]
```

The first one is the added control qubit, and the second and third are the control qubit and the target qubit of the original `CX` gate respectively.

---

## Matrix

```rust
pub fn matrix(
    &self,
    params: &[f64],
) -> Result<Cow<'_, Array2<Complex<f64>>>, CircuitError>
```

`matrix(params)` returns the local numeric matrix of the multi-controlled gate. The parameter list `params` corresponds to the parameters of the base gate, and its length must agree with `self.num_params()`.

```rust
use cqlib_core::circuit::{MCGate, StandardGate};

let gate = MCGate::new(1, StandardGate::RZ);

let matrix = gate.matrix(&[std::f64::consts::PI / 2.0])?;
assert_eq!(matrix.shape(), &[4, 4]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

In the example above, the base gate is the single-qubit parameterized gate `RZ`; after one control qubit is added, the result is a two-qubit controlled `RZ` gate, so the matrix shape is `(4, 4)`.

In general, if an `MCGate` acts on `n` qubits, its matrix shape is:

```text
2^n × 2^n
```

---

## Inverse gate

```rust
pub fn inverse(&self, params: &[Parameter]) -> Option<(MCGate, SmallVec<[Parameter; 3]>)>
```

`inverse(params)` returns the inverse gate of the current multi-controlled gate and the transformed parameter list. The inverse of a multi-controlled gate is equivalent to "taking the inverse of the base gate and then adding the same control qubits".

```rust
use cqlib_core::circuit::{MCGate, StandardGate};

let gate = MCGate::new(1, StandardGate::S);

let (inverse, params) = gate.inverse(&[]).unwrap();

assert_eq!(*inverse.base_gate(), StandardGate::SDG);
assert!(params.is_empty());
```

---

## `MCGate` and `StandardGate`

Some common controlled gates already exist as standard enum values of `StandardGate`. They can be regarded as special cases of `MCGate`.

| Standard gate | Equivalent form |
| --- | --- |
| `StandardGate::CX` | `MCGate::new(1, StandardGate::X)` |
| `StandardGate::CY` | `MCGate::new(1, StandardGate::Y)` |
| `StandardGate::CZ` | `MCGate::new(1, StandardGate::Z)` |
| `StandardGate::CCX` | `MCGate::new(2, StandardGate::X)` |
| `StandardGate::CRX` | `MCGate::new(1, StandardGate::RX)` |
| `StandardGate::CRY` | `MCGate::new(1, StandardGate::RY)` |
| `StandardGate::CRZ` | `MCGate::new(1, StandardGate::RZ)` |
