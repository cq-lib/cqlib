# StandardGate

`cqlib_core::circuit::StandardGate`

```rust
use cqlib_core::circuit::StandardGate;
```

`StandardGate` is the enum type in the Rust core used to represent the native standard quantum gates of Cqlib. It covers a basic gate set including common single-qubit gates, parameterized rotation gates, multi-qubit gates, controlled gates, two-body Pauli rotation gates, the fSim gate and the global phase marker.

---

## Enum variants

```rust
pub enum StandardGate {
    I,
    H,
    RX,
    RXX,
    RXY,
    RY,
    RYY,
    RZ,
    RZX,
    RZZ,
    S,
    SDG,
    SWAP,
    T,
    TDG,
    U,
    X,
    XY,
    X2P,
    X2M,
    XY2P,
    XY2M,
    Y,
    Y2P,
    Y2M,
    Z,
    Phase,
    GPhase,
    CX,
    CCX,
    CY,
    CZ,
    CRX,
    CRY,
    CRZ,
    FSIM,
}
```

---

## Gate metadata

`StandardGate` provides a set of metadata methods for querying information such as the number of qubits a gate acts on, the number of built-in control qubits, the number of parameters and whether the gate is diagonal.

| Method | Returns | Description |
| --- | --- | --- |
| `name()` | `&'static str` | Return the stable operation name of the gate, for example `"H"`, `"RX"`, `"CX"`. |
| `all()` | `&'static [StandardGate]` | Return all standard gate enum values, commonly used for generating gate tables, tests or target gate set checks. |
| `num_qubits()` | `usize` | Return the total number of qubits the gate acts on. |
| `num_ctrl_qubits()` | `usize` | Return the number of control qubits built into the gate. |
| `num_params()` | `usize` | Return the number of parameters the gate requires. |
| `is_diagonal()` | `bool` | Determine whether the gate is diagonal in the computational basis. |

```rust
use cqlib_core::circuit::StandardGate;

assert_eq!(StandardGate::H.num_qubits(), 1);
assert_eq!(StandardGate::CX.num_ctrl_qubits(), 1);
assert_eq!(StandardGate::RZ.num_params(), 1);
assert!(StandardGate::RZ.is_diagonal());
```

---

## Gate categories

### 1. Single-qubit fixed gates

Single-qubit fixed gates require no extra parameters and are commonly used for state preparation, basis change, phase correction and Clifford-related optimization.

| Gate | Description |
| --- | --- |
| `I` | Identity gate, which leaves the quantum state unchanged. |
| `H` | Hadamard gate, used to convert between the computational basis and the superposition basis. |
| `X` / `Y` / `Z` | Pauli gates, corresponding to bit flip, bit flip with a phase and phase flip respectively. |
| `S` / `SDG` | The `S` gate and its inverse gate, both belonging to the Clifford phase gates. |
| `T` / `TDG` | The `T` gate and its inverse gate, commonly used to extend a gate set beyond Clifford gates. |
| `X2P` / `X2M` | Positive/negative half-angle rotation gates in the X direction. |
| `Y2P` / `Y2M` | Positive/negative half-angle rotation gates in the Y direction. |

### 2. Parameterized single-qubit gates

Parameterized single-qubit gates control rotation or phase change through angle parameters, and are commonly used in variational circuits, parameter sweeps and hardware calibration.

| Gate | Number of parameters | Description |
| --- | --- | --- |
| `RX` | 1 | X-axis rotation, usually adopting the `exp(-i θ X / 2)` convention. |
| `RY` | 1 | Y-axis rotation, usually adopting the `exp(-i θ Y / 2)` convention. |
| `RZ` | 1 | Z-axis rotation, usually adopting the `exp(-i θ Z / 2)` convention. |
| `RXY` | 2 | Rotation around an arbitrary axis in the XY plane. |
| `U` | 3 | General single-qubit gate. |
| `Phase` | 1 | Phase gate, which applies a phase to the `\|1>` component. |
| `XY` | 1 | Single-qubit parameterized gate of the XY interaction family. |
| `XY2P` / `XY2M` | 1 | Positive/negative half-angle XY gates. |

### 3. Multi-qubit gates

Multi-qubit gates establish correlations between qubits and are core operations in entangled state preparation, quantum algorithms and compilation mapping.

| Gate | Number of parameters | Description |
| --- | --- | --- |
| `CX` | 0 | controlled-X, also known as CNOT. |
| `CY` | 0 | controlled-Y. |
| `CZ` | 0 | controlled-Z. |
| `CCX` | 0 | Toffoli gate, with two control qubits and one target qubit. |
| `SWAP` | 0 | Exchange the states of two qubits. |
| `RXX` / `RYY` / `RZZ` / `RZX` | 1 | Two-qubit Pauli rotation gates. |
| `CRX` / `CRY` / `CRZ` | 1 | Singly controlled parameterized rotation gates. |
| `FSIM` | 2 | fSim gate, commonly used to describe two-qubit interactions in specific hardware. |

### 4. Global phase gate

`GPhase` is a zero-qubit global phase marker with a parameter count of 1. It represents a global phase factor and usually does not act on any concrete qubit.

---

## Matrix interface

```rust
pub fn matrix(
    &self,
    params: &[f64],
) -> Result<Cow<'_, Array2<Complex<f64>>>, CircuitError>
```

`matrix()` returns the local numeric matrix of the standard gate. `params` must provide parameters according to `self.num_params()`; when any one of them is `NaN` or infinite, `InvalidParameterValue` is returned; passing fewer parameters than `num_params()` is a call error.

```rust
use cqlib_core::circuit::StandardGate;

let h = StandardGate::H.matrix(&[])?;
let rx = StandardGate::RX.matrix(&[std::f64::consts::PI / 2.0])?;

assert_eq!(h.shape(), &[2, 2]);
assert_eq!(rx.shape(), &[2, 2]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Mathematical conventions

Rotation gates in Cqlib use the half-angle exponential definition common in quantum computing. For example:

```text
RX(θ) = exp(-i θ X / 2)
RY(θ) = exp(-i θ Y / 2)
RZ(θ) = exp(-i θ Z / 2)
```

Two-body Pauli rotation gates use a similar convention:

```text
RXX(θ) = exp(-i θ X⊗X / 2)
RYY(θ) = exp(-i θ Y⊗Y / 2)
RZZ(θ) = exp(-i θ Z⊗Z / 2)
RZX(θ) = exp(-i θ Z⊗X / 2)
```

`Phase(λ)` applies a phase to the `|1>` component; `GPhase(λ)` represents a global phase factor.

---

## Inverse gate interface

```rust
pub fn inverse(
    &self,
    params: &[Parameter],
) -> Option<(StandardGate, SmallVec<[Parameter; 3]>)>
```

`inverse()` returns the inverse gate enum of the current standard gate together with the transformed parameter list. Every standard gate has a representable inverse gate, so this interface always returns `Some` for a standard gate. `params` must supply enough parameters according to `num_params()`; an insufficient number of parameters is a call error.

Common inverse gate relations are as follows:

| Gate | Inverse |
| --- | --- |
| `H` | `H` |
| `X` / `Y` / `Z` | The gate itself |
| `S` | `SDG` |
| `SDG` | `S` |
| `T` | `TDG` |
| `TDG` | `T` |
| `RX(θ)` | `RX(-θ)` |
| `RY(θ)` | `RY(-θ)` |
| `RZ(θ)` | `RZ(-θ)` |
| `CX` / `CY` / `CZ` | The gate itself |
| `SWAP` | The gate itself |
| `CCX` | The gate itself |
| `X2P` / `X2M` | Mutually inverse |
| `Y2P` / `Y2M` | Mutually inverse |
| `XY2P` / `XY2M` | Mutually inverse, with parameters unchanged |
| `Phase(λ)` / `GPhase(λ)` | Parameters are negated |
| `RXX` / `RYY` / `RZZ` / `RZX` | Parameters are negated |
| `RXY(θ, φ)` | `RXY(-θ, φ)`, with `φ` unchanged |
| `FSIM(θ, φ)` | `FSIM(-θ, -φ)` |
| `XY(θ)` | `XY(π + θ)` |
| `U(θ, φ, λ)` | `U(-θ, -λ, -φ)`, with the last two parameters swapped |

```rust
use cqlib_core::circuit::{Parameter, StandardGate};

let theta = Parameter::symbol("theta");
let inverse = StandardGate::RX.inverse(&[theta.clone()]).unwrap();

assert_eq!(inverse.0, StandardGate::RX);

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```

---

## `StandardGate` and `Circuit`

For ordinary hand-written circuits on the Rust side, the convenience methods provided by `Circuit` are usually preferred, for example `h()`, `cx()` and `rz()`. These methods automatically construct the corresponding standard gate instruction and check the number of qubits, the number of parameters and qubit ownership.

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut c = Circuit::new(1);
c.rx(Qubit::new(0), "theta")?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

When writing importers, compiler passes or low-level tests, a `StandardGate` can also be converted explicitly into an `Instruction` and then appended to a circuit through `Circuit::append()`.

```rust
use cqlib_core::circuit::{Circuit, ParameterValue, Qubit, StandardGate};

let mut c = Circuit::new(1);

c.append(
    StandardGate::RX.into(),
    [Qubit::new(0)],
    [ParameterValue::from("theta")],
    None,
)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## `StandardGate` and `MCGate`

Standard gates such as `CX`, `CY`, `CZ`, `CCX`, `CRX`, `CRY` and `CRZ` can be regarded as built-in enum forms of common controlled gates. For these gates, compilers and matrix implementations can use the optimized standard gate path directly.

When more control qubits are needed, or control qubits are to be added to an arbitrary base standard gate, `MCGate` should be used. For example, multi-controlled X gates, multi-controlled phase gates and controlled custom standard gates are usually better represented through `MCGate`.

| Standard gate | Can be understood as |
| --- | --- |
| `CX` | Singly controlled `X` |
| `CY` | Singly controlled `Y` |
| `CZ` | Singly controlled `Z` |
| `CCX` | Doubly controlled `X` |
| `CRX(θ)` | Singly controlled `RX(θ)` |
| `CRY(θ)` | Singly controlled `RY(θ)` |
| `CRZ(θ)` | Singly controlled `RZ(θ)` |
