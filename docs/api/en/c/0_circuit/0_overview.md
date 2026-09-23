# Quantum circuit

The circuit module is the core entry point of the Cqlib C API, used to build, represent and transform quantum circuits. All circuit state is kept in the opaque handle `CCircuit`, and C code operates on circuits through free functions.

The circuit module mainly provides the following capabilities:

- **Basic circuit construction**: create a set of logical qubits, and append quantum gates, measurement, reset, barrier and other operations in order.
- **Gate system**: the complete set of free functions for single-qubit gates without parameters, parameterized single-qubit gates, two-qubit gates and three-qubit gates; parameterized gates provide both a numeric variant and a symbolic parameter variant.
- **Symbolic parameters**: represent adjustable parameters such as gate angles with `CParameter`, with support for expression parsing, evaluation and whole-circuit parameter binding.
- **Circuit properties and structure**: query the qubit count, operation count and depth, with support for inverse circuits, composite gate decomposition, circuit composition and appending qubits.
- **Matrix conversion**: export small purely quantum gate circuits as a dense unitary matrix (two-step API).

---

## Core conventions

| Concept | Description |
| --- | --- |
| **Opaque handle** | Handle types such as `CCircuit` have only a forward declaration in the header and are managed internally by Rust; they can only be created and destroyed through the API. |
| **Qubit index** | `Circuit` uses consecutive logical indices `0..num_qubits-1`; an index out of range returns status code `-2`. |
| **Status code** | Gate and structural operations return `int32_t`, with `0` for success; see [Overview](../0_overview.md#status-codes) for details. |
| **Returns a new object** | `circuit_inverse` / `circuit_decompose` / `circuit_assign_params` and the like return a new, independently owned circuit, which must be released with `circuit_free`. |

---

## Quick examples

### Bell state preparation

```c
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);
circuit_free(qc);
```

### Parameterized rotation

```c
CParameter *theta = param_parse("theta/2");
circuit_rx_param(qc, 1, theta);
param_free(theta);

CCircuit *bound = circuit_assign_params(qc, "theta:3.1415926");
circuit_free(bound);
```

---

## API overview

| Object | Page | Description |
| --- | --- | --- |
| `CCircuit` | [Circuit](1_circuit.md) | The main circuit container: construction and release, gate operations, circuit properties and structural operations. |
| `CParameter` | [Parameter](2_parameter.md) | Symbolic parameter expressions: parsing, evaluation and whole-circuit parameter binding. |
| Unitary matrix export | [Circuit To Matrix](3_circuit_to_matrix.md) | Two-step export of a circuit's dense unitary matrix. |
