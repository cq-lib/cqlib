# Cqlib

<div align="center">

**High-Performance Quantum Computing SDK — Built in Rust, for Every Language**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE.txt)
[![Rust](https://img.shields.io/badge/rust-1.89%2B-orange.svg)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/python-3.10%2B-blue.svg)](https://www.python.org)
[![C](https://img.shields.io/badge/c-11%2B-lightgrey.svg)](https://en.cppreference.com/w/c/11)

[中文文档](README.CN.md)

</div>

## Overview

Cqlib is a high-performance quantum computing SDK that provides a unified foundation for building, optimizing, and
executing quantum programs. Its core is written in Rust for maximum safety and speed, with first-class language bindings
for Python 3.10+ and C 11+.

Designed from the ground up for both research and production use, Cqlib covers the full quantum computing workflow: from
circuit construction and parameterization, through multi-level IR transformations, rule-based compilation and
optimization passes, to realistic device simulation with built-in noise models and error mitigation techniques such as
Zero-Noise Extrapolation (ZNE) and Virtual Distillation. Its module design allows you to use individual components
independently or compose them into a complete pipeline, making it equally suitable for quantum algorithm prototyping,
education, and integration into larger software stacks.

Learn more at **[qc.zdxlz.com/cqlib](https://qc.zdxlz.com/cqlib)**.

## Features

- **Circuit Construction** — Intuitive APIs to build, compose, and parameterize quantum circuits
- **Intermediate Representation (IR)** — A principled IR for quantum programs enabling multi-level optimization
- **Compiler Passes** — Gate decomposition, layout mapping, routing, and scheduling
- **Device Abstraction** — Realistic device models with calibration data, noise models, and topology
- **Error Mitigation** — Built-in support for Zero-Noise Extrapolation (ZNE) and Virtual Distillation
- **Visualization** — Rich circuit diagrams and result plots (SVG/text)
- **Multi-language Support** — Native Rust, Python (PyO3), and C (cbindgen) interfaces

## Quick Start

### Python

```bash
pip install cqlib
```

```python
# Bell state circuit
from cqlib import Circuit

qc = Circuit(2)
qc.h(0)
qc.cx(0, 1)

# Inspect the circuit
print(qc.num_qubits)  # 2
print(len(qc.operations))  # 2

# Get the unitary matrix
matrix = qc.to_matrix()  # 4x4 numpy complex128 ndarray
```

Parameterized circuit:

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

qc = Circuit(2)
qc.rx(0, theta)
qc.ry(1, theta)
qc.cx(0, 1)

# Bind symbols to numeric values
bound = qc.assign_parameters({"theta": 0.5})
```

### Rust

```toml
[dependencies]
cqlib-core = "0.1"
```

```rust
use cqlib_core::circuit::{Circuit, Qubit};

// Bell state circuit
let mut qc = Circuit::new(2);
qc.h(Qubit::new(0)).unwrap();
qc.cx(Qubit::new(0), Qubit::new(1)).unwrap();

assert_eq!(qc.num_qubits(), 2);
assert_eq!(qc.operations().len(), 2);
```

Parameterized circuit:

```rust
use cqlib_core::circuit::{Circuit, Qubit, Parameter};
use std::collections::HashMap;

let theta = Parameter::symbol("θ");

let mut qc = Circuit::new(2);
qc.rx(Qubit::new(0), theta.clone()).unwrap();
qc.ry(Qubit::new(1), theta).unwrap();
qc.cx(Qubit::new(0), Qubit::new(1)).unwrap();

// Bind parameters
let mut bindings = HashMap::new();
bindings.insert("θ", std::f64::consts::PI);
let evaluated = qc.assign_parameters( & Some(bindings)).unwrap();
```

### C

```c
#include <cqlib/circuit.h>

int main(void) {
    // Create a 2-qubit Bell state circuit
    cqlib_circuit_t *qc = cqlib_circuit_new(2);
    cqlib_circuit_h(qc, 0);
    cqlib_circuit_cx(qc, 0, 1);

    // Inspect
    uint32_t n = cqlib_circuit_num_qubits(qc);   // 2
    size_t ops = cqlib_circuit_operation_count(qc); // 2

    cqlib_circuit_free(qc);
    return 0;
}
```

## Language Support

| Language | Minimum Version | Binding Technology |
|----------|-----------------|--------------------|
| Rust     | 1.89            | Native             |
| Python   | 3.10            | PyO3 (abi3-py310)  |
| C        | 11              | cbindgen           |

## Building from Source

**Prerequisites:** Rust 1.89+, Python 3.10+ (for Python bindings)

```bash
git clone https://gitee.com/cq-lib/cqlib.git
cd cqlib

# Build core library
cargo build --release -p cqlib-core

# Build Python bindings
pip install maturin
maturin develop --release -m crates/binding-python/Cargo.toml

# Build C bindings
cargo build --release -p binding-c
```

Run the test suite:

```bash
cargo test --all
pytest crates/binding-python/tests/
```

## Documentation

| Resource               | Link                                                     |
|------------------------|----------------------------------------------------------|
| API Reference (Rust)   | [docs.rs/cqlib-core](https://docs.rs/cqlib-core)         |
| API Reference (Python) | [qc.zdxlz.com/docs](https://qc.zdxlz.com/)               |
| Source Repository (Gitee) | [gitee.com/cq-lib/cqlib](https://gitee.com/cq-lib/cqlib) |
| Source Repository (GitHub) | [github.com/cq-lib/cqlib](https://github.com/cq-lib/cqlib) |

## Contributing

Contributions are welcome. Please read the [Contributing Guide](CONTRIBUTING.md) before submitting changes. All
participants are expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md). If you need to report a security issue,
please follow the [Security Policy](SECURITY.md).

Please use the repository issue tracker for discussion. When contributing, open a pull request against the `main` branch
and ensure all relevant tests pass.

## License

Cqlib is licensed under the [Apache License, Version 2.0](LICENSE.txt).
