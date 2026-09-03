# Cqlib

Cqlib is a high-performance quantum computing SDK with a Rust core and
Python bindings. It provides APIs for building and parameterizing quantum
circuits, compilation and optimization, device modeling, simulation, error
mitigation, and visualization.

> **Beta release:** Cqlib is under active development. APIs may change before
> the stable release.

## Installation

Cqlib requires Python 3.10 or later.

Install the latest pre-release from PyPI:

```shell
python -m pip install --pre cqlib
```

## Quick start

Create a Bell-state circuit:

```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

print(circuit.num_qubits)
print(circuit.operations)
```

Create and bind a parameterized circuit:

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(2)
circuit.rx(0, theta)
circuit.ry(1, theta)
circuit.cx(0, 1)

bound = circuit.assign_parameters({"theta": 0.5})
```

## Supported environments

- Python 3.10 or later on CPython
- Linux, macOS, and Windows
- Type information is included for static type checkers

Binary wheel availability depends on the platform and architecture. When a
compatible wheel is unavailable, installation from the source distribution
requires a Rust toolchain.

## Documentation and support

- [Documentation](https://qc.zdxlz.com/learn/#/resource/informationSpace?lang=zh&cId=/mkdocs/zh/cqlib/01-overview.html)
- [Source repository](https://gitee.com/cq-lib/cqlib)
- [Issue tracker](https://gitee.com/cq-lib/cqlib/issues)

## Development

From the repository root:

```shell
maturin develop -m crates/binding-python/Cargo.toml
pytest crates/binding-python/tests/
```

## License

Cqlib is distributed under the Apache License 2.0.
