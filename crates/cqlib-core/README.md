# cqlib-core

`cqlib-core` is the Rust implementation at the heart of the Cqlib quantum
computing SDK. It provides circuit construction, symbolic parameters,
compilation and optimization, device modeling, simulation, error mitigation,
intermediate-representation support, and visualization.

> **Beta release:** `cqlib-core` is under active development. Public APIs may
> change before the first stable release.

## Installation

Add the crate to your project:

```toml
[dependencies]
cqlib-core = "0.1.0-beta.1"
```

Or use Cargo:

```shell
cargo add cqlib-core@0.1.0-beta.1
```

## Quick start

Create a Bell-state circuit:

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

assert_eq!(circuit.num_qubits(), 2);
assert_eq!(circuit.operations().len(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

Create and bind a parameterized circuit:

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
use std::collections::HashMap;

let theta = Parameter::symbol("theta");

let mut circuit = Circuit::new(2);
circuit.rx(Qubit::new(0), theta.clone())?;
circuit.ry(Qubit::new(1), theta)?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let mut bindings = HashMap::new();
bindings.insert("theta", 0.5);
let bound = circuit.assign_parameters(&Some(bindings))?;

assert!(bound.parameters().is_empty());

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

## Capabilities

- **Circuit construction** — Static, parameterized, and dynamic circuits with
  measurement-driven classical control flow
- **Gate library** — Standard, controlled, multi-controlled, composite, and
  custom unitary gates
- **Symbolic parameters** — Expression parsing, arithmetic, simplification,
  differentiation, and parameter binding
- **Compilation** — Decomposition, optimization, layout, routing, scheduling,
  and rule-based rewriting
- **Device modeling** — Qubit topology, layouts, calibration properties, and
  noise models
- **Simulation** — Statevector, density-matrix, noisy density-matrix, and
  stabilizer simulation
- **Quantum information** — Pauli operators, Hamiltonians, observables,
  evolution, metrics, and entropy functions
- **Error mitigation** — Zero-noise extrapolation and virtual distillation
- **Interchange formats** — OpenQASM 2, OpenQASM 3, and QCIS
- **Visualization** — Text and SVG circuit rendering plus state and result plots

## Main modules

| Module | Purpose |
| --- | --- |
| `circuit` | Circuits, gates, parameters, classical data, and control flow |
| `compile` | Compiler workflows, transformations, layout, and routing |
| `device` | Device topology, properties, noise, and execution results |
| `error_mitigation` | Error-mitigation methods and configuration |
| `ir` | OpenQASM and QCIS import/export |
| `qis` | Quantum-information types and simulators |
| `visualization` | Circuit, state, and result visualization |

## Documentation and support

- [Rust API documentation](https://docs.rs/cqlib-core)
- [Cqlib documentation](https://qc.zdxlz.com/learn/#/resource/informationSpace?lang=zh&cId=/mkdocs/zh/cqlib/01-overview.html)
- [Source repository](https://gitee.com/cq-lib/cqlib)
- [Issue tracker](https://gitee.com/cq-lib/cqlib/issues)

## Development

From the repository root:

```shell
cargo check -p cqlib-core --locked
cargo test -p cqlib-core --locked
cargo doc -p cqlib-core --no-deps
```

Before publishing:

```shell
cargo publish --dry-run --registry crates-io -p cqlib-core --locked
```

## License

Cqlib is distributed under the Apache License 2.0.
