# Quantum Circuit (circuit)

The quantum circuit module of the C API centers on the opaque handle `CCircuit*` and provides circuit construction and inspection, symbolic parameters, gate operations, classical data and control flow, symbolic matrices, parameterized templates (ansatz), control flow graphs and circuit DAGs.

See [Overview](../0_overview.md) for the global conventions on error codes, memory ownership and the two-step array output pattern.

---

## Module navigation

| Category | Page | Main objects | Description |
| --- | --- | --- | --- |
| Circuit container | [Circuit](1_circuit.md) | `CCircuit*` | Circuit construction, property queries, composition, inversion, decomposition, global phase and parameter binding. |
| Qubit | [Qubit](2_qubit.md) | `CQubit` | Logical qubit identifier: creation, ID access, checked conversions, comparison and string representation. |
| Parameters | [Parameters](3_parameter.md) | `CParameter*`, `CCircuitParam`, `CParameterValue` | Expression parsing, evaluation and interning into the circuit parameter table. |
| Operation IR | [Operations / Instructions](4_operation_instruction.md) | `COperation*`, `CValueOperation*` | Storage-layer operations, circuit operation snapshots and delay instructions. |
| Standard gates | [Standard Gates](5_gate_standard.md) | `circuit_h` and friends | Built-in standard gates with numeric-angle and symbolic-angle (`*_param`) variants. |
| Unitary gates | [Unitary Gates](6_gate_unitary.md) | `circuit_unitary` and friends | Numeric matrix gates and symbolic matrix gates. |
| Multi-controlled gates | [Multi-Controlled Gates](7_gate_mc_gate.md) | `circuit_multi_control` | Add control qubits in front of a standard gate. |
| Circuit gates | [Circuit Gates](8_gate_circuit_gate.md) | `CCircuitGate*`, `circuit_to_gate` | Freeze a circuit into a reusable composite gate. |
| Classical data and control flow | [Classical / Control Flow](9_classical_control_flow.md) | `CClassicalVar*`, `CClassicalExpr*`, `circuit_if`, ... | Measurement, classical expressions and structured control flow. |
| Symbolic matrix | [Symbolic Matrix](10_symbolic_matrix.md) | `CSymbolicMatrix*` | Dense symbolic matrices that retain parameters, and equivalence checking. |
| Ansatz | [Ansatz](11_ansatz.md) | `CTwoLocal`, `CQAOAAnsatz`, ... | Parameterized circuit templates. |
| Control flow graph | [CFG](12_cfg.md) | `CCircuitCFG*` | Circuit control flow graph view, queries and reconstruction. |
| Matrix conversion | [Circuit to Matrix](13_circuit_to_matrix.md) | `circuit_to_matrix` and friends | Numeric matrices of purely unitary circuits and global phase. |
| Circuit DAG | [Circuit DAG](14_circuit_dag.md) | `CCircuitDag*` | Dependency DAG view and circuit reconstruction. |

---

## Core concepts

| Concept | Description |
| --- | --- |
| **Opaque handle** | Types such as `struct CCircuit` are only forward-declared in the header with no exported fields; all access goes through functions, and handles are released with the matching `*_free`. |
| **Qubit numbering** | Logical qubits are `uint32_t` ids. `circuit_new(n)` creates consecutive ids `0..n-1`; `circuit_from_qubits` accepts an explicit (including sparse) id list. |
| **Gate function prefix** | Gate operations all start with `circuit_`: fixed gates such as `circuit_h`, `circuit_cx`; angle-carrying gates provide a numeric variant (`circuit_rx`) and a symbolic variant (`circuit_rx_param`, taking a `CParameter*`). |
| **Symbolic parameters** | `param_parse` parses an expression from a string; once interned into the circuit's parameter table, a parameter is represented as a `CCircuitParam` (fixed value or table index). |
| **Measurement and control flow** | Measurement produces immutable classical values owned by the circuit; classical expressions and `circuit_if`/`circuit_while`/`circuit_for_uint`/`circuit_switch` describe runtime control flow. |

Controlled gates follow the order "control qubits first, target qubit last"; for example, in `circuit_cx(qc, control, target)` the `control` argument is the control qubit.

---

## Minimal example

```c
#include <cqlib_c.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);       /* 0 on success, negative on error */
    circuit_cx(qc, 0, 1);

    printf("qubits    = %zu\n", circuit_num_qubits(qc));
    printf("depth     = %zd\n", circuit_depth(qc, true));
    printf("operations= %zu\n", circuit_num_operations(qc));

    /* Two-step read of the qubit list */
    uintptr_t n = circuit_qubits_len(qc);
    uint32_t *ids = malloc(n * sizeof(uint32_t));
    circuit_qubits(qc, ids, n);   /* ids = {0, 1} */

    free(ids);
    circuit_free(qc);
    return 0;
}
```

After construction, a circuit is usually executed by the [Statevector](../3_qis/1_statevector.md) or [DensityMatrix](../3_qis/2_density_matrix.md) simulators; a circuit containing measurement or control flow no longer has a single fixed unitary matrix representation, and the matrix conversion interfaces apply only to purely quantum sub-circuits (see [Circuit to Matrix](13_circuit_to_matrix.md)).
