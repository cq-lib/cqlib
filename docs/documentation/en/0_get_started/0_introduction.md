# Introduction to Cqlib

## Welcome to Cqlib!

Cqlib is a quantum computing software development kit (SDK) independently developed by China Telecom Quantum Group and built for engineering practice. It provides a unified, stable and high-performance programming interface and representation capability for quantum circuits.

This documentation set covers the [installation tutorial](1_installation.md), the [quickstart](2_quickstart.md), [scenario-based guides](../1_cqlib/0_circuit/0_overview.md) and the [API reference](../../../api/en/python/0_overview.md) for multiple programming languages, aiming to make the construction, execution and visualization of quantum computing tasks smoother and to improve development efficiency and the overall experience.

---


## Why choose Cqlib?

- **Extreme performance, Rust-driven**  
  The core layer is built in Rust, ensuring high-concurrency processing capability and memory safety for [circuit compilation](../1_cqlib/4_compiler/0_overview.md) and [intermediate representation](../1_cqlib/1_ir/0_overview.md) conversion, and providing a solid foundation for large-scale quantum circuit simulation and complex logic processing.

- **Native ecosystem, QCIS integration**  
  Deep integration with the independently developed quantum instruction set [QCIS](../1_cqlib/1_ir/1_qcis.md), supporting bidirectional import and export between circuits and QCIS text.

- **Multi-language coordination, developer-friendly**  
  Through the jointly designed, highly abstracted [Python interface](../../../api/en/python/0_overview.md) and low-level [C interface](../../../api/en/c/0_overview.md), algorithm development efficiency and system integration stability are both addressed, supporting cross-platform and cross-language mixed programming scenarios.

- **Unified architecture, flexible extension**  
  A unified [intermediate representation](../1_cqlib/1_ir/0_overview.md) supports the construction of maintainable and extensible quantum software systems, covering the full path from prototype validation to enterprise application development.

---

## What programming languages does Cqlib use?

Cqlib uses three languages, Rust, Python and C, and follows the design idea of "a unified core with layered interfaces", ensuring consistency of capability while accommodating the usage habits of different development scenarios:

- **[Rust](../../../api/en/rust/0_overview.md)**  
  As the core implementation layer, it carries key capabilities such as quantum circuits, gates and instructions, the parameter system, intermediate representation, device modeling, quantum information, error mitigation and compilation and optimization.

- **[Python](../../../api/en/python/0_overview.md)**  
  Aimed at algorithm development and rapid prototyping, providing a more efficient and intuitive Python interface, convenient for interactive use and fast iteration.

- **[C](../../../api/en/c/0_overview.md)**  
  Aimed at system integration and cross-language invocation, providing a stable C interface, convenient for deployment in heterogeneous projects.

---

## What can Cqlib do?

The core capabilities of Cqlib consist of the following eight modules:

- [Quantum Circuit](../1_cqlib/0_circuit/0_overview.md)
  For creating and managing quantum circuits; supports basic functionality such as adding quantum gates, parameterized circuits, control flow, circuit composition, matrix conversion and structural analysis. It is the main entry point for building quantum programs with Cqlib.
- [Intermediate Representation](../1_cqlib/1_ir/0_overview.md)
  For converting between circuit objects and text formats; supports the QCIS, OpenQASM 2.0 and OpenQASM 3.0 formats, facilitating circuit storage, exchange across toolchains and subsequent compilation.
- [Device](../1_cqlib/2_device/0_overview.md)
  For describing information related to quantum devices, including device topology, the mapping from logical qubits to physical qubits, calibration properties, noise models and execution results, providing the data foundation for backend adaptation and noise-aware compilation.
- [Quantum Information](../1_cqlib/3_qis/0_overview.md)
  Provides tools related to quantum states, operators and quantum information processing, including statevector, density matrix, stabilizer simulation, Pauli operators, Hamiltonian, quantum evolution and information metrics.
- [Compilation and optimization](../1_cqlib/4_compiler/0_overview.md)
  For transforming high-level quantum circuits into circuit forms better suited to execution on a target device; supports gate decomposition, topological layout, routing mapping, template matching, swap optimization and Clifford-related optimization.
- [Visualization](../1_cqlib/5_visualization/0_overview.md)
  For displaying the structure of quantum circuits; supports Unicode text diagrams and SVG rendering, making it easy to inspect circuit hierarchy, gate operation order and overall structure.
- [Error Mitigation](../1_cqlib/6_error_mitigation/0_overview.md)
  For reducing the impact of noise on quantum computation results; provides error mitigation capabilities such as zero-noise extrapolation (ZNE) and virtual distillation, suitable for noisy simulation or for processing results from real device experiments.
- [Tianyan quantum cloud platform client](../1_cqlib/7_tianyan/0_overview.md)
  For connecting to the Tianyan quantum cloud platform; supports platform authentication, backend queries, device configuration retrieval, QCIS task submission, task status polling, execution result parsing and readout error correction, suitable for submitting locally built circuits to cloud real devices or platform backends.

---


## Next steps

To start using Cqlib, the following reading order is recommended:

- [Installation guide](1_installation.md): installation and setup in a single step
- [Quickstart](2_quickstart.md): the first quantum circuit, from "0" to "1"
- [Quantum Circuit](../1_cqlib/0_circuit/0_overview.md): learn about the basic module for describing quantum programs in Cqlib.
- [Tianyan quantum cloud platform client](../1_cqlib/7_tianyan/0_overview.md): learn how to log in to the platform, select a backend, submit QCIS tasks and obtain cloud execution results.
