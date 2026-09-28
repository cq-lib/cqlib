# C API Overview

## Welcome to the Cqlib C API reference!

The Cqlib C binding (`binding-c`) exposes quantum circuit construction, symbolic parameters, IR conversion, device modeling, quantum information, compilation, visualization and error mitigation through a pure C ABI. Every entry point follows the same style: free functions + opaque handles + `int32_t` error codes, declared in the auto-generated header [crates/binding-c/include/cqlib_c.h](../../../../crates/binding-c/include/cqlib_c.h).

---

## Documentation navigation

### Quantum Circuit (circuit)

- [Overview](0_circuit/0_overview.md)
- [Circuit](0_circuit/1_circuit.md)
- [Qubit](0_circuit/2_qubit.md)
- [Parameters](0_circuit/3_parameter.md)
- [Operations / Instructions](0_circuit/4_operation_instruction.md)
- [Standard Gates](0_circuit/5_gate_standard.md)
- [Unitary Gates](0_circuit/6_gate_unitary.md)
- [Multi-Controlled Gates](0_circuit/7_gate_mc_gate.md)
- [Circuit Gates](0_circuit/8_gate_circuit_gate.md)
- [Classical / Control Flow](0_circuit/9_classical_control_flow.md)
- [Symbolic Matrix](0_circuit/10_symbolic_matrix.md)
- [Ansatz](0_circuit/11_ansatz.md)
- [CFG](0_circuit/12_cfg.md)
- [Circuit to Matrix](0_circuit/13_circuit_to_matrix.md)
- [Circuit DAG](0_circuit/14_circuit_dag.md)

### Intermediate Representation (ir)

- [Overview](1_ir/0_overview.md)
- [QCIS](1_ir/1_qcis.md)
- [OpenQASM 2.0](1_ir/2_qasm2.md)
- [OpenQASM 3.0](1_ir/3_qasm3.md)

### Device (device)

- [Overview](2_device/0_overview.md)
- [Topology](2_device/1_topology.md)
- [Device and noise properties](2_device/2_properties_device.md)
- [Layout](2_device/3_layout.md)
- [Noise](2_device/4_noise.md)
- [Result](2_device/5_result.md)
- [Qubit identifiers](2_device/6_qubits.md)

### Quantum Information (qis)

- [Overview](3_qis/0_overview.md)
- [Statevector](3_qis/1_statevector.md)
- [DensityMatrix](3_qis/2_density_matrix.md)
- [DensityMatrixNoise](3_qis/3_density_matrix_noise.md)
- [Stabilizer](3_qis/4_stabilizer.md)
- [ClassicalState](3_qis/5_classical_state.md)
- [Pauli](3_qis/6_pauli.md)
- [Hamiltonian](3_qis/7_hamiltonian.md)
- [Evolution](3_qis/8_evolution.md)
- [Metrics / Entropy](3_qis/9_metrics_entropy.md)

### Compilation (compile)

- [Overview](4_compile/0_overview.md)
- [Compiler](4_compile/1_compiler.md)
- [Transform](4_compile/2_transform.md)
- [Layout](4_compile/3_layout.md)
- [Routing](4_compile/4_routing.md)
- [SABRE](4_compile/5_sabre.md)
- [Decompose & Resynthesis](4_compile/6_decompose_resynthesis.md)
- [Knowledge Rules](4_compile/7_knowledge.md)
- [Resource](4_compile/8_resource.md)
- [Commutation](4_compile/9_commutation.md)

### Visualization (visualization)

- [Overview](5_visualization/0_overview.md)
- [Text circuit diagrams](5_visualization/1_draw_text.md)
- [SVG circuit diagrams](5_visualization/2_draw_figure.md)
- [State plots](5_visualization/3_state_plots.md)
- [Result plots](5_visualization/4_result_plots.md)
- [Write to file and inline display](5_visualization/5_render_to_file.md)
- [Visual IR](5_visualization/6_visual_ir.md)

### Error Mitigation (error_mitigation)

- [Overview](6_error_mitigation/0_overview.md)
- [Zero-noise extrapolation (ZNE)](6_error_mitigation/1_zne.md)
- [Virtual distillation](6_error_mitigation/2_virtual_distillation.md)
- [Unified pipeline](6_error_mitigation/3_unified.md)

---

## Quick start: Bell state

The following complete example builds a two-qubit Bell circuit, reads the probability distribution with the two-step pattern after exact simulation, and releases every resource:

```c
#include <cqlib_c.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    /* 1. Build the Bell circuit: H(0) + CX(0, 1) */
    CCircuit *qc = circuit_new(2);
    if (qc == NULL) {
        return 1;
    }
    if (circuit_h(qc, 0) != 0 || circuit_cx(qc, 0, 1) != 0) {
        circuit_free(qc);
        return 1;
    }

    /* 2. Exact simulation */
    CStatevector *sv = statevector_from_circuit(qc);
    if (sv == NULL) {
        circuit_free(qc);
        return 1;
    }

    /* 3. Two-step output: query the length first, then fill the buffer */
    uintptr_t len = statevector_probabilities_len(sv);
    double *probs = malloc(len * sizeof(double));
    if (probs == NULL || statevector_probabilities(sv, probs, len) != 0) {
        free(probs);
        statevector_free(sv);
        circuit_free(qc);
        return 1;
    }
    /* probs = {0.5, 0.0, 0.0, 0.5} */
    printf("P(00) = %f, P(11) = %f\n", probs[0], probs[3]);

    /* 4. Release (in reverse order of acquisition) */
    free(probs);
    statevector_free(sv);
    circuit_free(qc);
    return 0;
}
```

### Build and link

Windows (MinGW gcc, gnu target):

```bash
cargo build --release -p binding-c --target x86_64-pc-windows-gnu

gcc -Icrates/binding-c/include main.c -Ltarget/x86_64-pc-windows-gnu/release -lbinding_c -lntdll -lws2_32 -lbcrypt -luserenv -ladvapi32 -o bell
```

At runtime the executable must find `binding_c.dll`: add the absolute path of `target/x86_64-pc-windows-gnu/release` to `PATH`, or copy the DLL next to the executable. See [crates/binding-c/README.md](../../../../crates/binding-c/README.md) for macOS and Linux linking, test projects and further examples.

---

## Error codes

Functions returning `int32_t` use `0` for success and negative values for errors:

| Value | Name | Typical scenario |
| --- | --- | --- |
| `0` | Ok | Success. |
| `-1` | NullPtr | NULL handle, NULL array or invalid C string passed in. |
| `-2` | QubitOutOfBounds | Qubit index outside the circuit or device range. |
| `-3` | CircuitError | Circuit structure error: validation failure, gate arity or parameter count mismatch, the same qubit reused within one operation, non-invertible operations, etc. |
| `-4` | ParseError | Parse error: unknown gate name, invalid expression syntax, invalid UTF-8, malformed IR text. |
| `-5` | IoError | File read/write failure. |
| `-6` | CompilerError | Compilation workflow error. |
| `-7` | SimulationError | Simulation error: a non-Clifford gate entering stabilizer simulation, not run yet, etc. |
| `-8` | InvalidParam | Invalid parameter: out-of-range probability, buffer too small, invalid tag value, etc. |

Pointer-returning functions use `NULL` for errors; `param_evaluate` returns `0.0` when evaluation fails (including unbound symbols); `circuit_depth` returns a negative value on error. Boolean-style queries return `1`/`0`, with `-1` usually meaning NULL input.

---

## Memory ownership and freeing

1. **Constructors and transforms return heap handles**: `circuit_new`, `circuit_from_qubits`, `circuit_inverse`, `circuit_decompose`, `circuit_assign_params`, `param_parse`, `circuit_to_gate` and friends return newly allocated opaque handles; the caller releases them with the matching `*_free` (`circuit_free`, `param_free`, `circuit_gate_free`, ...). Every `*_free` accepts `NULL`.
2. **Strings are freed with `cqlib_string_free`**: every interface returning `char*` (`operation_name`, `value_operation_name`, `circuit_gate_name`, each element written by `circuit_symbols`, ...) is released with `cqlib_string_free`.
3. **Arrays use the two-step output pattern**: first call a `*_len` function to query the count and allocate a buffer, then call the fill function. Behavior on a too-small buffer falls into two classes: most fill functions return a negative error code (usually `-8`); some functions (such as `circuit_qubits`) only return the total count without copying.
4. **Nested lists come with matching frees**: each element written by `circuit_parameters` (a newly allocated `CParameter*`, released individually with `param_free`), `circuit_cfg_block_operations` (released with `operation_free`) and similar functions is an independent handle that must be freed one by one.
5. **Interfaces that explicitly hand over ownership**: `circuit_to_gate` clones the input circuit into a composite-gate handle; the `CValueOperation*` returned by `circuit_index` is a read-only snapshot independent of later circuit mutations (release with `value_operation_free`); when `CParameterValue.tag == PARAMETER_VALUE_TAG_PARAM` the embedded `param` field is a newly allocated `CParameter*` (release with `param_free`). Ownership of these objects belongs entirely to the caller.

---

## Constant tags

The header exports the following `#define` tag constants; all are part of the stable ABI.

### Parameter storage tags

| Constant | Value | Description |
| --- | --- | --- |
| `CIRCUIT_PARAM_TAG_FIXED` | 0 | `CCircuitParam.tag`: fixed number stored in `value`. |
| `CIRCUIT_PARAM_TAG_INDEX` | 1 | `CCircuitParam.tag`: interned expression in the circuit's parameter table; `index` holds the table index. |
| `PARAMETER_VALUE_TAG_FIXED` | 0 | `CParameterValue.tag`: fixed number stored in `value`; `param` is NULL. |
| `PARAMETER_VALUE_TAG_PARAM` | 1 | `CParameterValue.tag`: symbolic parameter; `param` points to a newly allocated `CParameter*`. |
| `OPERATION_PARAM_FIXED` | 0 | Parameter tag written by `operation_params`: fixed value. |
| `OPERATION_PARAM_INDEX` | 1 | Parameter tag written by `operation_params`: index into the owning circuit's parameter table; resolve it against the circuit. |

### Ansatz topology and evolution strategy

| Constant | Value | Description |
| --- | --- | --- |
| `ENTANGLEMENT_LINEAR` | 0 | TwoLocal entanglement topology: linear nearest-neighbor chain. |
| `ENTANGLEMENT_CIRCULAR` | 1 | Linear chain plus wrap-around edge. |
| `ENTANGLEMENT_FULL` | 2 | All-to-all pairs. |
| `ENTANGLEMENT_CUSTOM` | 3 | Custom explicit pairs of control/target ids. |
| `EVOLUTION_STRATEGY_EXACT` | 0 | QAOA evolution strategy: exact term-wise evolution. |
| `EVOLUTION_STRATEGY_AUTO` | 1 | Automatic exact/Trotter selection. |
| `EVOLUTION_STRATEGY_TROTTER` | 2 | Explicit Trotter product formula. |
| `TROTTER_FIRST_ORDER` | 0 | First-order product formula. |
| `TROTTER_SECOND_ORDER` | 1 | Second-order product formula. |
| `TROTTER_RANDOMIZED` | 2 | Randomized first-order formula. |
| `TROTTER_MODE_FIRST_ORDER` | 0 | Trotter-Suzuki decomposition mode: first-order Lie-Trotter. |
| `TROTTER_MODE_SECOND_ORDER` | 1 | Trotter-Suzuki decomposition mode: second-order Strang splitting. |

### Control flow graph tags

| Constant | Value | Description |
| --- | --- | --- |
| `CFG_FLOW_TRUE_BRANCH` | 1 | True edge out of `if`/`while`/`for` headers. |
| `CFG_FLOW_FALSE_BRANCH` | 2 | False edge out of the headers. |
| `CFG_FLOW_UNCONDITIONAL` | 3 | Fallthrough jump / structured merge edge. |
| `CFG_FLOW_CASE` | 4 | Exact-value switch case edge (value in the case halves). |
| `CFG_FLOW_DEFAULT_CASE` | 5 | Switch default edge. |
| `CFG_FLOW_BREAK` | 6 | Structured `break` edge. |
| `CFG_FLOW_CONTINUE` | 7 | Structured `continue` edge. |
| `CFG_TERMINATOR_BRANCH` | 1 | Boolean branch header terminator. |
| `CFG_TERMINATOR_FOR_LOOP` | 2 | Unsigned range loop header terminator. |
| `CFG_TERMINATOR_SWITCH` | 3 | Exact-value multi-way branch header terminator. |
| `CFG_TERMINATOR_JUMP` | 4 | Unconditional jump terminator. |
| `CFG_TERMINATOR_BREAK` | 5 | Structured `break` terminator. |
| `CFG_TERMINATOR_CONTINUE` | 6 | Structured `continue` terminator. |
| `CFG_TERMINATOR_RETURN` | 7 | End-of-execution terminator. |
| `CFG_REGION_IF` | 1 | Structured conditional region. |
| `CFG_REGION_WHILE` | 2 | Structured while-loop region. |
| `CFG_REGION_FOR` | 3 | Structured range-loop region. |
| `CFG_REGION_SWITCH` | 4 | Structured exact-value switch region. |

### Classical types and expression nodes

| Constant | Value | Description |
| --- | --- | --- |
| `CQLIB_CLASSICAL_TYPE_BIT` | 0 | Classical type `Bit`. |
| `CQLIB_CLASSICAL_TYPE_BOOL` | 1 | Classical type `Bool`. |
| `CQLIB_CLASSICAL_TYPE_UINT` | 2 | Classical type `UInt`. |
| `CQLIB_CLASSICAL_TYPE_BIT_VEC` | 3 | Classical type `BitVec`. |

Node-kind tags written by `classical_expr_kind`: `CQLIB_CLASSICAL_EXPR_VAR` (0), `_VALUE` (1), `_BOOL_LITERAL` (2), `_BIT_LITERAL` (3), `_UINT_LITERAL` (4), `_BIT_VEC_LITERAL` (5), `_UNARY` (6), `_BINARY` (7), `_COMPARE` (8), `_CAST` (9), `_SELECT` (10), `_EXTRACT_BIT` (11), `_EXTRACT_BITS` (12), `_CONCAT` (13), `_PACK_BITS` (14).

### DAG wire tags

| Constant | Value | Description |
| --- | --- | --- |
| `DAG_WIRE_QUBIT` | 0 | Qubit timeline wire carrying a qubit id. |
| `DAG_WIRE_CLASSICAL_VAR` | 1 | Mutable classical storage wire carrying a variable id. |
| `DAG_WIRE_CLASSICAL_VALUE` | 2 | Immutable classical value wire carrying a value id. |
| `DAG_WIRE_GLOBAL_ORDER` | 3 | Global ordering resource wire with no payload. |

### Compilation configuration tags

| Constant | Value | Description |
| --- | --- | --- |
| `COMPILE_MODE_NORMAL` | 0 | Normal compilation mode. |
| `COMPILE_MODE_ENHANCED` | 1 | Enhanced compilation mode. |
| `COMPILE_TARGET_LOGICAL` | 0 | Logical target. |
| `COMPILE_TARGET_BASIS` | 1 | Basis gate-set target. |
| `COMPILE_TARGET_DEVICE` | 2 | Device target. |
| `COMPILE_TARGET_TOPOLOGY_BASIS` | 3 | Topology basis target. |

### Compilation pass tags

| Constant | Value | Description |
| --- | --- | --- |
| `REWRITE_MODE_OPTIMIZE` | 0 | Knowledge rewrite mode: optimization. |
| `REWRITE_MODE_LOWERING` | 1 | Knowledge rewrite mode: lowering toward the target basis. |
| `LAYOUT_OBJECTIVE_TOPOLOGY_ONLY` | 0 | Layout objective: topology only (distance + direction mismatch). |
| `LAYOUT_OBJECTIVE_FIDELITY_AWARE` | 1 | Layout objective: fidelity aware (adds error-rate terms). |
| `LAYOUT_OBJECTIVE_AUTO` | 2 | Layout objective: chosen automatically from device calibration data. |
| `VF2_EDGE_REQUIREMENT_POSITIVE_INTERACTIONS` | 0 | VF2 edge requirement: positive-weight interactions only. |
| `VF2_EDGE_REQUIREMENT_ALL_INTERACTIONS` | 1 | VF2 edge requirement: all interactions. |
| `SABRE_MAX_LOOKAHEAD_WEIGHTS` | 8 | Maximum length of the `CSabreConfigC.lookahead_weights` array. |
| `TWO_QUBIT_BASIS_PAULI_ROTATIONS` | 0 | Two-qubit synthesis target basis: Pauli rotations. |
| `TWO_QUBIT_BASIS_CX` | 1 | Two-qubit synthesis target basis: CX. |
| `TWO_QUBIT_BASIS_CY` | 2 | Two-qubit synthesis target basis: CY. |
| `TWO_QUBIT_BASIS_CZ` | 3 | Two-qubit synthesis target basis: CZ. |
| `TWO_QUBIT_BASIS_RZZ` | 4 | Two-qubit synthesis target basis: RZZ. |
| `KAK_FACTOR_K1L` | 0 | KAK local factor: first local gate (left). |
| `KAK_FACTOR_K1R` | 1 | KAK local factor: first local gate (right). |
| `KAK_FACTOR_K2L` | 2 | KAK local factor: second local gate (left). |
| `KAK_FACTOR_K2R` | 3 | KAK local factor: second local gate (right). |

### Knowledge rule and resource tags

| Constant | Value | Description |
| --- | --- | --- |
| `RULE_KIND_SIMPLIFY` | 0 | Knowledge rule kind: simplification. |
| `RULE_KIND_CANCEL` | 1 | Knowledge rule kind: cancellation. |
| `RULE_KIND_MERGE` | 2 | Knowledge rule kind: merging. |
| `RULE_KIND_COMMUTE` | 3 | Knowledge rule kind: commutation. |
| `RULE_KIND_DECOMPOSE` | 4 | Knowledge rule kind: decomposition. |
| `RULE_KIND_CANONICALIZE` | 5 | Knowledge rule kind: canonicalization. |
| `RULE_KIND_HARDWARE_NATIVE` | 6 | Knowledge rule kind: hardware-native. |
| `RULE_KIND_OTHER` | 7 | Knowledge rule kind: other. |
| `RESOURCE_REQUIREMENT_CLEAN_ZERO` | 0 | Resource requirement: a \|0⟩ (clean) ancilla qubit. |
| `RESOURCE_REQUIREMENT_DIRTY` | 1 | Resource requirement: a dirty ancilla qubit in any state. |

### Commutation result tags

| Constant | Value | Description |
| --- | --- | --- |
| `COMMUTATION_RESULT_UNPROVEN` | 0 | Commutation could not be proven. |
| `COMMUTATION_RESULT_EXACT` | 1 | The operations commute exactly. |
| `COMMUTATION_RESULT_UP_TO_GLOBAL_PHASE` | 2 | The operations commute up to a global phase. |

### Noise channel tags

| Constant | Value | Description |
| --- | --- | --- |
| `NOISE_BIT_FLIP` | 0 | Single-qubit bit-flip channel with flip probability p. |
| `NOISE_PHASE_FLIP` | 1 | Phase-flip channel with flip probability p. |
| `NOISE_DEPOLARIZING` | 2 | Depolarizing channel with depolarizing parameter p. |
| `NOISE_AMPLITUDE_DAMPING` | 3 | Amplitude damping channel with damping parameter gamma. |
| `NOISE_PHASE_DAMPING` | 4 | Phase damping channel with scattering probability. |
| `NOISE_PAULI` | 5 | General single-qubit Pauli channel with probabilities px/py/pz (each non-negative, sum <= 1). |
| `NOISE_TWO_DEPOLARIZING` | 0 | Two-qubit depolarizing channel. |
| `NOISE_TWO_INDEPENDENT` | 1 | Two-qubit independent channel (q0/q1 each carry a single-qubit channel). |
| `NOISE_TWO_CORRELATED_PAULI` | 2 | Correlated Pauli channel (probability p plus `NOISE_PAULI_OP_*` operators). |
| `NOISE_PAULI_OP_I` / `_X` / `_Y` / `_Z` | 0 / 1 / 2 / 3 | Pauli operators applied to q0/q1 by the correlated channel. |

### Execution status tags

| Constant | Value | Description |
| --- | --- | --- |
| `EXECUTION_STATUS_QUEUED` | 0 | Task submitted and queued. |
| `EXECUTION_STATUS_RUNNING` | 1 | Task currently running. |
| `EXECUTION_STATUS_COMPLETED` | 2 | Task completed successfully. |
| `EXECUTION_STATUS_FAILED` | 3 | Task failed (query the error code and message). |
| `EXECUTION_STATUS_CANCELLED` | 4 | Task cancelled by the user. |

### Error mitigation tags

| Constant | Value | Description |
| --- | --- | --- |
| `MITIGATION_ZNE` | 0 | Mitigation method: zero-noise extrapolation (extra parameter: fold-level array). |
| `MITIGATION_VIRTUAL_DISTILLATION` | 1 | Mitigation method: virtual distillation (extra parameter: copies). |
| `PROCESS_ZNE_POLYNOMIAL` | 0 | Mitigation process: ZNE + polynomial extrapolation. |
| `PROCESS_ZNE_EXPONENTIAL` | 1 | Mitigation process: ZNE + exponential extrapolation. |
| `PROCESS_VIRTUAL_DISTILLATION` | 2 | Mitigation process: virtual distillation. |
| `ZNE_EXTRAPOLATE_POLYNOMIAL` | 0 | `zne_extrapolate` method: polynomial. |
| `ZNE_EXTRAPOLATE_EXPONENTIAL` | 1 | `zne_extrapolate` method: exponential. |

### Pauli operator tags

| Constant | Value | Description |
| --- | --- | --- |
| `PAULI_I` | 0 | Identity operator I. |
| `PAULI_X` | 1 | Pauli X. |
| `PAULI_Y` | 2 | Pauli Y. |
| `PAULI_Z` | 3 | Pauli Z. |

### Parameter display mode tags

| Constant | Value | Description |
| --- | --- | --- |
| `PARAM_MODE_NUMERIC` | 0 | Numeric display mode. |
| `PARAM_MODE_SYMBOLIC` | 1 | Symbolic display mode. |
| `PARAM_MODE_SYMBOLIC_WITH_VALUE` | 2 | Symbolic display with the numeric value. |
| `PARAM_MODE_PI_FRACTION_PREFERRED` | 3 | Pi-fraction-preferred display mode. |

---

## Threading and Complex64 layout

### Threading and concurrency

- There is no global initialization or teardown call: load the shared library (`binding_c.dll` on Windows, `libbinding_c.so` on Linux, `libbinding_c.dylib` on macOS) and call the entry points directly.
- Handles are independent heap objects owned by the caller; when several threads share one handle, the caller is responsible for serializing access. Distinct handles are independent of each other and can be used separately.

### Complex64 layout

```c
typedef struct Complex64 {
  double re;
  double im;
} Complex64;
```

Complex numbers are represented as consecutive interleaved `(re, im)` double pairs. Matrix interfaces (`circuit_to_matrix`, `operation_matrix`, ...) emit row-major interleaved double arrays; the buffer must hold `2 × element count` doubles.
