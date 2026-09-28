# Decompose / Resynthesis (C)

This page covers the circuit-level decomposition passes (circuit-gate definition expansion `decompose_expand_definitions`, matrix-backed unitary synthesis `decompose_unitaries`, multi-controlled gate rewriting `decompose_mc_gates`), two-qubit block resynthesis `resynthesize_two_qubit_blocks`, the numeric unitary-matrix synthesis primitives (one-qubit / two-qubit), unitary synthesis targets `decompose_unitary_target_*`, KAK (Weyl) decomposition, target-basis lowering `target_basis_lowerer_*`, and device lowering `decompose_lower_to_device`. Decomposition is layered by the representation each operation carries: definition expansion runs before numeric synthesis, so unitary gates that ship their own implementation circuit are expanded before entering matrix synthesis. Matrix inputs use `Complex64` (`{double re; double im}` interleaved pairs, row-major); the layout is described in [Unitary Gates](../0_circuit/6_gate_unitary.md). Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## Constants

Two-qubit synthesis basis tags (the `basis` parameter of `synthesize_numeric_2q_unitary`):

| Constant | Value | Meaning |
| --- | --- | --- |
| `TWO_QUBIT_BASIS_PAULI_ROTATIONS` | 0 | Local `U` + `RXX`/`RYY`/`RZZ` Pauli rotations. |
| `TWO_QUBIT_BASIS_CX` | 1 | Local `U` + `CX` templates. |
| `TWO_QUBIT_BASIS_CY` | 2 | Local `U` + `CY` templates. |
| `TWO_QUBIT_BASIS_CZ` | 3 | Local `U` + `CZ` templates. |
| `TWO_QUBIT_BASIS_RZZ` | 4 | Local `U` + `RZZ`. |

KAK local-factor tags (the `factor` parameter of `kak_decomposition_local_factor`):

| Constant | Value | Meaning |
| --- | --- | --- |
| `KAK_FACTOR_K1L` | 0 | `k1l` (left local factor, applied after the interaction). |
| `KAK_FACTOR_K1R` | 1 | `k1r` (right local factor, applied after the interaction). |
| `KAK_FACTOR_K2L` | 2 | `k2l` (left local factor, applied before the interaction). |
| `KAK_FACTOR_K2R` | 3 | `k2r` (right local factor, applied before the interaction). |

---

## Configuration Construction

### decompose_unitary_config_default()

Returns the default unitary-decompose configuration by value: `recurse_control_flow=1` (recurse into control-flow bodies).

```c
struct CUnitaryDecomposeConfig decompose_unitary_config_default(void);
```

Returns: the default configuration struct (by value, nothing to free).

### mc_gate_config_default()

Returns the default MC-gate decompose configuration by value: no pre-layout clean ancillas (`max_pre_layout_clean_ancillas=0`), no dirty borrowing (`allow_dirty_borrowing=0`), no hard limit on total qubits (`has_max_total_qubits=0`).

```c
struct CMcGateDecomposeConfig mc_gate_config_default(void);
```

Returns: the default configuration struct (by value, nothing to free).

### resynthesis_config_normal()

Returns the normal-budget two-qubit block resynthesis configuration by value: `max_block_ops=16`, `max_crossed_ops=4`, `max_scan_span=32`, `skip_labeled_ops=1`, `recurse_control_flow=1`, knowledge-rule commutation oracle enabled (`enable_rule_oracle=1`), matrix fallback disabled (`enable_matrix_fallback=0`), `max_matrix_qubits=4`.

```c
struct CResynthesisConfig resynthesis_config_normal(void);
```

Returns: the normal-budget configuration struct (by value, nothing to free).

### resynthesis_config_enhanced()

Returns the enhanced-budget two-qubit block resynthesis configuration by value: widens the block-collection budgets on top of the normal budget (`max_block_ops=32`, `max_crossed_ops=8`, `max_scan_span=64`); the remaining fields match the normal budget.

```c
struct CResynthesisConfig resynthesis_config_enhanced(void);
```

Returns: the enhanced-budget configuration struct (by value, nothing to free).

---

## Circuit-Level Decomposition Passes

### decompose_expand_definitions(circuit)

Expands the circuit-gate definitions of the circuit (gates that ship their own implementation circuit), replacing gate calls with the plain operations of their implementation circuits, and returns the rebuilt circuit.

```c
struct CCircuit *decompose_expand_definitions(const struct CCircuit *circuit);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | Input circuit. |

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL when `circuit` is NULL or expansion fails.

Construction of circuit gates is covered in [Circuit Gates](../0_circuit/8_gate_circuit_gate.md).

### decompose_unitaries(circuit, config)

Synthesizes the matrix-backed unitary gates of the circuit (gates added by `circuit_unitary`), replacing each matrix with a sequence of standard-gate operations, and returns the rebuilt circuit. Equivalent to `decompose_unitaries_with_rule_stats` with a NULL `stats`.

```c
struct CCircuit *decompose_unitaries(const struct CCircuit *circuit,
                                     struct CUnitaryDecomposeConfig config);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | Input circuit. |
| `config` | `struct CUnitaryDecomposeConfig` | Decompose configuration (by value). |

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL when `circuit` is NULL or synthesis fails.

Construction of matrix-backed unitary gates is covered in [Unitary Gates](../0_circuit/6_gate_unitary.md).

### decompose_unitaries_with_rule_stats(circuit, config, stats)

Diagnostic form of `decompose_unitaries`: when `stats` is non-NULL, writes a snapshot of the pass-local decomposition-rule cache statistics of this run into `*stats`.

```c
struct CCircuit *decompose_unitaries_with_rule_stats(const struct CCircuit *circuit,
                                                     struct CUnitaryDecomposeConfig config,
                                                     struct CDecompositionRuleStats *stats);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | Input circuit. |
| `config` | `struct CUnitaryDecomposeConfig` | Decompose configuration (by value). |
| `stats` | `struct CDecompositionRuleStats*` | Rule-cache statistics output; NULL skips statistics. |

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL when `circuit` is NULL or synthesis fails (`*stats` is then left untouched).

### decompose_mc_gates(circuit, config)

Rewrites the multi-controlled gates of the circuit (gates added by `circuit_multi_control`), replacing each with a sequence of singly-controlled / uncontrolled operations, and returns the rebuilt circuit. Equivalent to `decompose_mc_gates_with_rule_stats` with a NULL `stats`.

```c
struct CCircuit *decompose_mc_gates(const struct CCircuit *circuit,
                                    struct CMcGateDecomposeConfig config);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | Input circuit. |
| `config` | `struct CMcGateDecomposeConfig` | Resource and limit configuration (by value). |

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL when `circuit` is NULL or rewriting fails.

Construction of multi-controlled gates is covered in [Multi-Controlled Gates](../0_circuit/7_gate_mc_gate.md).

### decompose_mc_gates_with_rule_stats(circuit, config, stats)

Diagnostic form of `decompose_mc_gates`: when `stats` is non-NULL, writes a snapshot of the pass-local decomposition-rule cache statistics of this run into `*stats`.

```c
struct CCircuit *decompose_mc_gates_with_rule_stats(const struct CCircuit *circuit,
                                                    struct CMcGateDecomposeConfig config,
                                                    struct CDecompositionRuleStats *stats);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | Input circuit. |
| `config` | `struct CMcGateDecomposeConfig` | Resource and limit configuration (by value). |
| `stats` | `struct CDecompositionRuleStats*` | Rule-cache statistics output; NULL skips statistics. |

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL when `circuit` is NULL or rewriting fails (`*stats` is then left untouched).

### decompose_mc_gates_for_device(circuit, device, policy)

Rewrites the multi-controlled gates of the circuit within the usable-qubit capacity of the device (a pre-layout logical transform): the rewriting plan is constrained by the device's usable physical qubit count. `policy` may be NULL to select the default resource policy.

```c
struct CCircuit *decompose_mc_gates_for_device(const struct CCircuit *circuit,
                                               const struct CDevice *device,
                                               const struct CResourcePolicy *policy);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | Input circuit. |
| `device` | `const CDevice*` | Target device (construction in [Device / Properties](../2_device/2_properties_device.md)). |
| `policy` | `const CResourcePolicy*` | Resource policy; NULL selects the default (see [Resource Policy](8_resource.md)). |

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL when any handle is NULL or rewriting fails.

---

## Two-Qubit Block Resynthesis

### resynthesize_two_qubit_blocks(circuit, config)

Resynthesizes the two-qubit blocks of the circuit: candidate blocks are collected within bounded budgets around two-qubit anchors, reordered through the commutation engine, and re-synthesized; returns the rebuilt circuit.

```c
struct CCircuit *resynthesize_two_qubit_blocks(const struct CCircuit *circuit,
                                               struct CResynthesisConfig config);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | Input circuit. |
| `config` | `struct CResynthesisConfig` | Resynthesis budget configuration (by value). |

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL when `circuit` is NULL or resynthesis fails.

---

## Numeric Unitary Synthesis Primitives

### synthesize_numeric_1q_unitary(matrix, out)

Synthesizes a 2x2 unitary matrix into Cqlib's `U` convention: the represented matrix is `exp(i * global_phase) * U(theta, phi, lambda)`. `matrix` points at 4 row-major `Complex64` elements.

```c
int32_t synthesize_numeric_1q_unitary(const Complex64 *matrix,
                                      struct COneQubitUnitaryDecomposition *out);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `matrix` | `const Complex64*` | 4 row-major complex elements (2x2 matrix). |
| `out` | `struct COneQubitUnitaryDecomposition*` | Synthesis result output. |

Returns: 0 on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `matrix` or `out` is NULL. |
| `-6` | CompilerError | The matrix is non-finite, non-unitary, or otherwise rejected by the synthesizer. |

### synthesize_numeric_2q_unitary(matrix, first, second, basis)

Synthesizes a 4x4 unitary matrix into a sequence of standard-gate operations, delivered as a `CTwoQubitUnitarySynthesis` handle. `matrix` points at 16 row-major `Complex64` elements; `first` and `second` are the target qubits; `basis` is one of `TWO_QUBIT_BASIS_*`.

```c
struct CTwoQubitUnitarySynthesis *synthesize_numeric_2q_unitary(const Complex64 *matrix,
                                                                uint32_t first,
                                                                uint32_t second,
                                                                uint8_t basis);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `matrix` | `const Complex64*` | 16 row-major complex elements (4x4 matrix). |
| `first` | `uint32_t` | First target qubit ID. |
| `second` | `uint32_t` | Second target qubit ID. |
| `basis` | `uint8_t` | Synthesis basis, one of `TWO_QUBIT_BASIS_*`. |

Returns: a new `CTwoQubitUnitarySynthesis*` on success (free with `two_qubit_synthesis_free`); NULL when `matrix` is NULL, `basis` is not a valid tag, or synthesis fails (for example the matrix is non-finite or non-unitary).

---

## Unitary Synthesis Targets

`CDecomposeUnitaryTarget` describes the native gate set a unitary synthesis step may emit: a native one-qubit basis, a native two-qubit basis, and a Pauli-rotation fallback flag. It also evaluates the exact target-basis cost of a fixed operation sequence.

### decompose_unitary_target_unconstrained()

```c
struct CDecomposeUnitaryTarget *decompose_unitary_target_unconstrained(void);
```

Creates an unconstrained synthesis target: no native-basis restrictions and the neutral exact Pauli-rotation fallback enabled.

Returns: a new `CDecomposeUnitaryTarget*` (free with `decompose_unitary_target_free`).

### decompose_unitary_target_from_instructions(gate_names, len)

Creates a synthesis target from a workflow-style target-basis list of standard-gate names (e.g. `"H"`, `"RZ"`, `"CX"`): one-qubit names become the native one-qubit basis, two-qubit names the native two-qubit basis, and the exact Pauli-rotation fallback stays enabled.

```c
struct CDecomposeUnitaryTarget *decompose_unitary_target_from_instructions(
    const char *const *gate_names,
    uintptr_t len);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `gate_names` | `const char* const*` | Array of standard-gate names; may be NULL when `len` is 0. |
| `len` | `uintptr_t` | Number of gate names. |

Returns: a new `CDecomposeUnitaryTarget*` on success (free with `decompose_unitary_target_free`); NULL when `len` is greater than 0 but `gate_names` is NULL, an entry is invalid UTF-8 or an unknown gate name, or the core rejects the basis (for example an empty basis).

### decompose_unitary_target_from_standard_gates(native_1q_names, num_1q, native_2q_names, num_2q, fallback_pauli)

Creates a synthesis target from explicit native one-qubit and two-qubit standard-gate name lists and a Pauli-rotation fallback flag. Every name in `native_1q_names` must be a one-qubit standard gate and every name in `native_2q_names` a two-qubit standard gate.

```c
struct CDecomposeUnitaryTarget *decompose_unitary_target_from_standard_gates(
    const char *const *native_1q_names,
    uintptr_t num_1q,
    const char *const *native_2q_names,
    uintptr_t num_2q,
    uint8_t fallback_pauli);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `native_1q_names` | `const char* const*` | Native one-qubit standard-gate names; may be NULL when `num_1q` is 0. |
| `num_1q` | `uintptr_t` | Number of one-qubit gate names. |
| `native_2q_names` | `const char* const*` | Native two-qubit standard-gate names; may be NULL when `num_2q` is 0. |
| `num_2q` | `uintptr_t` | Number of two-qubit gate names. |
| `fallback_pauli` | `uint8_t` | Non-zero enables the exact Pauli-rotation fallback. |

Returns: a new `CDecomposeUnitaryTarget*` on success (free with `decompose_unitary_target_free`); NULL on NULL input, invalid UTF-8, an unknown gate name, a wrong-arity gate, or an empty combined basis.

### decompose_unitary_target_free(ptr)

Frees a unitary-decompose synthesis target; NULL is allowed.

```c
void decompose_unitary_target_free(struct CDecomposeUnitaryTarget *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `struct CDecomposeUnitaryTarget*` | Handle to free; may be NULL. |

### decompose_unitary_target_fallback_pauli(ptr)

Tests whether the exact Pauli-rotation fallback is permitted for the target.

```c
int32_t decompose_unitary_target_fallback_pauli(const struct CDecomposeUnitaryTarget *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CDecomposeUnitaryTarget*` | Synthesis target handle. |

Returns: `1` when the fallback is permitted; `0` otherwise; `-1` for NULL input.

### decompose_unitary_target_set_fallback_pauli(ptr, value)

Sets whether the Pauli-rotation fallback is permitted for the target. The target is rebuilt around its native gate lists, which requires a non-empty basis; when the call is rejected the target is left unchanged.

```c
int32_t decompose_unitary_target_set_fallback_pauli(struct CDecomposeUnitaryTarget *ptr,
                                                    uint8_t value);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `struct CDecomposeUnitaryTarget*` | Synthesis target handle. |
| `value` | `uint8_t` | Non-zero permits the fallback. |

Returns: `0` on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` is NULL. |
| `-8` | InvalidParam | The target is unconstrained (no native gates). |

### decompose_unitary_target_native_1q_len(ptr) / decompose_unitary_target_native_1q(ptr, out, len)

```c
uintptr_t decompose_unitary_target_native_1q_len(const struct CDecomposeUnitaryTarget *ptr);
uintptr_t decompose_unitary_target_native_1q(const struct CDecomposeUnitaryTarget *ptr,
                                             char **out,
                                             uintptr_t len);
```

Two-step read of the native one-qubit gate names (step one: `*_len` returns the total; step two: the fill function copies the names into `out`, writing `min(total, len)` entries). Each written entry is a freshly allocated C string freed with `cqlib_string_free`; when `out` is NULL the fill function only returns the total without copying.

Returns (both functions): the total number of native one-qubit gates; 0 for NULL input.

### decompose_unitary_target_native_2q_len(ptr) / decompose_unitary_target_native_2q(ptr, out, len)

```c
uintptr_t decompose_unitary_target_native_2q_len(const struct CDecomposeUnitaryTarget *ptr);
uintptr_t decompose_unitary_target_native_2q(const struct CDecomposeUnitaryTarget *ptr,
                                             char **out,
                                             uintptr_t len);
```

Two-step read of the native two-qubit gate names (same buffer-copy pattern as the one-qubit pair; each written entry is freed with `cqlib_string_free`).

Returns (both functions): the total number of native two-qubit gates; 0 for NULL input.

### decompose_unitary_target_basis_len(ptr) / decompose_unitary_target_basis(ptr, out, len)

```c
uintptr_t decompose_unitary_target_basis_len(const struct CDecomposeUnitaryTarget *ptr);
uintptr_t decompose_unitary_target_basis(const struct CDecomposeUnitaryTarget *ptr,
                                         char **out,
                                         uintptr_t len);
```

Two-step read of the combined native basis (one-qubit gates followed by two-qubit gates; same buffer-copy pattern; each written entry is freed with `cqlib_string_free`).

Returns (both functions): the total number of basis gates; 0 for NULL input.

### decompose_unitary_target_cost_of_fixed_operations(ptr, gate_names, qubit_ids, qubit_counts, num_ops, out)

Computes the exact target-basis cost of a fixed standard-gate operation sequence under the native basis of the target. The sequence is given as `num_ops` gate names, with each operation's qubit IDs flattened into `qubit_ids` and per-operation qubit counts in `qubit_counts`. Parameters are treated as fixed zeros. Requires a constrained target (an unconstrained target has no basis to cost against). The cost snapshot is written to `*out` (the `CTargetBasisCost` layout is documented in [Configuration and Result Structs](#configuration-and-result-structs)).

```c
int32_t decompose_unitary_target_cost_of_fixed_operations(
    const struct CDecomposeUnitaryTarget *ptr,
    const char *const *gate_names,
    const uint32_t *qubit_ids,
    const uint32_t *qubit_counts,
    uintptr_t num_ops,
    struct CTargetBasisCost *out);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CDecomposeUnitaryTarget*` | Synthesis target handle. |
| `gate_names` | `const char* const*` | Gate name of each operation in the sequence. |
| `qubit_ids` | `const uint32_t*` | Qubit IDs of all operations, flattened in sequence order. |
| `qubit_counts` | `const uint32_t*` | Number of qubits each operation acts on. |
| `num_ops` | `uintptr_t` | Number of operations in the sequence. |
| `out` | `struct CTargetBasisCost*` | Cost snapshot output. |

Returns: `0` on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` or `out` is NULL. |
| `-4` | ParseError | A gate name is invalid UTF-8 or an unknown gate name. |
| `-6` | CompilerError | The core rejected the sequence. |
| `-8` | InvalidParam | The target is unconstrained (no native basis). |

---

## CTwoQubitUnitarySynthesis Readers

Result handle of `synthesize_numeric_2q_unitary`: the emitted operation sequence and the global phase.

### two_qubit_synthesis_free(ptr)

Frees a two-qubit synthesis result handle; NULL is allowed.

```c
void two_qubit_synthesis_free(struct CTwoQubitUnitarySynthesis *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `struct CTwoQubitUnitarySynthesis*` | Handle to free; may be NULL. |

### two_qubit_synthesis_num_operations(ptr)

Returns the number of emitted operations.

```c
uintptr_t two_qubit_synthesis_num_operations(const struct CTwoQubitUnitarySynthesis *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CTwoQubitUnitarySynthesis*` | Synthesis result handle. |

Returns: the operation count; 0 on NULL input.

### two_qubit_synthesis_global_phase(ptr)

Returns the global phase multiplying the emitted operation sequence.

```c
double two_qubit_synthesis_global_phase(const struct CTwoQubitUnitarySynthesis *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CTwoQubitUnitarySynthesis*` | Synthesis result handle. |

Returns: the global phase (radians); 0.0 on NULL input.

### two_qubit_synthesis_operation_name(ptr, index)

Returns the instruction name of operation `index`.

```c
char *two_qubit_synthesis_operation_name(const struct CTwoQubitUnitarySynthesis *ptr,
                                         uintptr_t index);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CTwoQubitUnitarySynthesis*` | Synthesis result handle. |
| `index` | `uintptr_t` | Zero-based operation index. |

Returns: a newly allocated C string on success (free with `cqlib_string_free`); NULL on NULL input or an out-of-bounds index.

### two_qubit_synthesis_operation_qubits_len(ptr, index)

Returns the number of qubits operation `index` acts on (step one of the two-step read pattern).

```c
uintptr_t two_qubit_synthesis_operation_qubits_len(const struct CTwoQubitUnitarySynthesis *ptr,
                                                   uintptr_t index);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CTwoQubitUnitarySynthesis*` | Synthesis result handle. |
| `index` | `uintptr_t` | Zero-based operation index. |

Returns: the qubit count; 0 on NULL input or an out-of-bounds index.

### two_qubit_synthesis_operation_qubits(ptr, index, out, len)

Copies the qubit IDs of operation `index` into `out` (step two of the two-step read pattern; call `two_qubit_synthesis_operation_qubits_len` first). When `len` is smaller than the total, only the first `len` entries are copied.

```c
uintptr_t two_qubit_synthesis_operation_qubits(const struct CTwoQubitUnitarySynthesis *ptr,
                                               uintptr_t index,
                                               uint32_t *out,
                                               uintptr_t len);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CTwoQubitUnitarySynthesis*` | Synthesis result handle. |
| `index` | `uintptr_t` | Zero-based operation index. |
| `out` | `uint32_t*` | Qubit ID output array. |
| `len` | `uintptr_t` | Capacity of `out`. |

Returns: the total number of qubits the operation acts on; 0 on NULL input or an out-of-bounds index.

---

## KAK Decomposition

### kak_decompose(matrix)

Decomposes a 4x4 unitary matrix into its KAK (Weyl) coordinates and local factors. `matrix` points at 16 row-major `Complex64` elements.

```c
struct CKakDecomposition *kak_decompose(const Complex64 *matrix);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `matrix` | `const Complex64*` | 16 row-major complex elements (4x4 matrix). |

Returns: a new `CKakDecomposition*` on success (free with `kak_decomposition_free`); NULL when `matrix` is NULL or decomposition fails (for example the matrix is non-finite or non-unitary).

### kak_decomposition_free(ptr)

Frees a KAK decomposition handle; NULL is allowed.

```c
void kak_decomposition_free(struct CKakDecomposition *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `struct CKakDecomposition*` | Handle to free; may be NULL. |

### kak_decomposition_global_phase(ptr)

Returns the scalar phase multiplying the complete decomposition.

```c
double kak_decomposition_global_phase(const struct CKakDecomposition *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CKakDecomposition*` | KAK decomposition handle. |

Returns: the global phase (radians); 0.0 on NULL input.

### kak_decomposition_coordinates(ptr, a, b, c)

Writes the canonical Pauli XX/YY/ZZ interaction coordinates `a`, `b`, `c` to the given out pointers.

```c
int32_t kak_decomposition_coordinates(const struct CKakDecomposition *ptr,
                                      double *a,
                                      double *b,
                                      double *c);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CKakDecomposition*` | KAK decomposition handle. |
| `a` | `double*` | XX interaction coordinate output. |
| `b` | `double*` | YY interaction coordinate output. |
| `c` | `double*` | ZZ interaction coordinate output. |

Returns: 0 on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | Any pointer is NULL. |

### kak_decomposition_local_factor(ptr, factor, out)

Copies one 2x2 local factor (selected by a `KAK_FACTOR_*` tag) into `out` as 4 row-major `Complex64` elements.

```c
int32_t kak_decomposition_local_factor(const struct CKakDecomposition *ptr,
                                       uint8_t factor,
                                       Complex64 *out);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CKakDecomposition*` | KAK decomposition handle. |
| `factor` | `uint8_t` | Local-factor tag, one of `KAK_FACTOR_*`. |
| `out` | `Complex64*` | 4 row-major complex elements output. |

Returns: 0 on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` or `out` is NULL. |
| `-8` | InvalidParam | `factor` is not a valid `KAK_FACTOR_*` tag. |

---

## Target-Basis Lowering

### target_basis_lowerer_new(gate_names, len)

Creates a target-basis lowerer from an array of standard-gate names: each entry of `gate_names` must be a standard-gate name accepted elsewhere by the C ABI (for example `"H"`, `"RZ"`, `"CX"`); the names become the target basis.

```c
struct CTargetBasisLowerer *target_basis_lowerer_new(const char *const *gate_names, uintptr_t len);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `gate_names` | `const char* const*` | Array of standard-gate names; may be NULL when `len` is 0. |
| `len` | `uintptr_t` | Number of gate names. |

Returns: a new `CTargetBasisLowerer*` on success (free with `target_basis_lowerer_free`); NULL when `len` is greater than 0 but `gate_names` is NULL, an entry is NULL, an entry is invalid UTF-8, an entry is an unknown gate name, or the basis is empty (`len` is 0).

### target_basis_lowerer_free(ptr)

Frees a target-basis lowerer handle; NULL is allowed.

```c
void target_basis_lowerer_free(struct CTargetBasisLowerer *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `struct CTargetBasisLowerer*` | Handle to free; may be NULL. |

### target_basis_lowerer_num_gates(ptr)

Returns the number of gates in the target basis.

```c
uintptr_t target_basis_lowerer_num_gates(const struct CTargetBasisLowerer *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CTargetBasisLowerer*` | Lowerer handle. |

Returns: the gate count; 0 on NULL input.

### target_basis_lowerer_requires_lowering(ptr, circuit)

Reports whether translating `circuit` to the target basis can change or reject its gate-like operations.

```c
int32_t target_basis_lowerer_requires_lowering(const struct CTargetBasisLowerer *ptr,
                                               const struct CCircuit *circuit);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CTargetBasisLowerer*` | Lowerer handle. |
| `circuit` | `const CCircuit*` | Input circuit. |

Returns: 1 when lowering is needed (the circuit contains gate-like operations outside the basis); 0 when it is not.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` or `circuit` is NULL. |

### target_basis_lowerer_apply(ptr, circuit)

Lowers `circuit` to the target basis and returns the rebuilt circuit.

```c
struct CCircuit *target_basis_lowerer_apply(const struct CTargetBasisLowerer *ptr,
                                            const struct CCircuit *circuit);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CTargetBasisLowerer*` | Lowerer handle. |
| `circuit` | `const CCircuit*` | Input circuit. |

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL when any handle is NULL or lowering fails.

---

## Device Lowering

### decompose_lower_to_device(circuit, device)

Lowers `circuit` to the native instruction set of the device and returns the rebuilt circuit.

```c
struct CCircuit *decompose_lower_to_device(const struct CCircuit *circuit,
                                           const struct CDevice *device);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | Input circuit. |
| `device` | `const CDevice*` | Target device; its native gate set (set by `device_with_native_gates`, see [Device / Properties](../2_device/2_properties_device.md)) is the lowering target. |

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL when any handle is NULL or lowering fails.

---

## Configuration and Result Structs

### CUnitaryDecomposeConfig

```c
typedef struct CUnitaryDecomposeConfig {
  uint8_t recurse_control_flow;
  uint8_t _reserved[7];
} CUnitaryDecomposeConfig;
```

| Field | Type | Meaning |
| --- | --- | --- |
| `recurse_control_flow` | `uint8_t` | 1 recurses into control-flow bodies, 0 does not. |
| `_reserved[7]` | `uint8_t[7]` | Padding that keeps the struct layout stable. |

### CMcGateDecomposeConfig

```c
typedef struct CMcGateDecomposeConfig {
  uintptr_t max_pre_layout_clean_ancillas;
  uint8_t allow_dirty_borrowing;
  uint8_t has_max_total_qubits;
  uint8_t _pad[6];
  uintptr_t max_total_qubits;
  uint64_t _reserved[2];
} CMcGateDecomposeConfig;
```

| Field | Type | Meaning |
| --- | --- | --- |
| `max_pre_layout_clean_ancillas` | `uintptr_t` | Upper bound on total clean logical ancillas the pass may create before layout. |
| `allow_dirty_borrowing` | `uint8_t` | 1 allows borrowing input qubits under the dirty contract. |
| `has_max_total_qubits` | `uint8_t` | 1 enforces the `max_total_qubits` hard limit, 0 leaves it unset. |
| `_pad[6]` | `uint8_t[6]` | Padding that keeps the struct layout stable. |
| `max_total_qubits` | `uintptr_t` | Hard limit on total logical qubits (used when `has_max_total_qubits != 0`). |
| `_reserved[2]` | `uint64_t[2]` | Padding that keeps the struct layout stable. |

### CResynthesisConfig

```c
typedef struct CResynthesisConfig {
  uintptr_t max_block_ops;
  uintptr_t max_crossed_ops;
  uintptr_t max_scan_span;
  uint8_t skip_labeled_ops;
  uint8_t recurse_control_flow;
  uint8_t enable_rule_oracle;
  uint8_t enable_matrix_fallback;
  uint8_t _pad[4];
  uintptr_t max_matrix_qubits;
  uint64_t _reserved[2];
} CResynthesisConfig;
```

| Field | Type | Meaning |
| --- | --- | --- |
| `max_block_ops` | `uintptr_t` | Maximum source operations in one bounded candidate block. |
| `max_crossed_ops` | `uintptr_t` | Maximum non-block operations crossed while collecting a block. |
| `max_scan_span` | `uintptr_t` | Collection budget per side of a two-qubit anchor. |
| `skip_labeled_ops` | `uint8_t` | 1 treats labeled operations as hard boundaries. |
| `recurse_control_flow` | `uint8_t` | 1 recurses into structured classical-control bodies. |
| `enable_rule_oracle` | `uint8_t` | 1 enables the knowledge-rule commutation oracle. |
| `enable_matrix_fallback` | `uint8_t` | 1 enables the local matrix fallback in the commutation engine. |
| `_pad[4]` | `uint8_t[4]` | Padding that keeps the struct layout stable. |
| `max_matrix_qubits` | `uintptr_t` | Maximum union-support size for the matrix fallback. |
| `_reserved[2]` | `uint64_t[2]` | Padding that keeps the struct layout stable. |

### CDecompositionRuleStats

Snapshot of the pass-local decomposition-rule cache statistics (the output of `decompose_unitaries_with_rule_stats` / `decompose_mc_gates_with_rule_stats`):

| Field | Type | Meaning |
| --- | --- | --- |
| `hits` | `uintptr_t` | Cache hits during the run. |
| `misses` | `uintptr_t` | Cache misses during the run. |
| `inserts` | `uintptr_t` | Cache writes during the run. |

### CTargetBasisCost

Snapshot of a target-basis lowering cost (the output of `decompose_unitary_target_cost_of_fixed_operations`):

| Field | Type | Meaning |
| --- | --- | --- |
| `two_qubit_ops` | `uintptr_t` | Two-qubit operations in the lowered sequence. |
| `depth` | `uintptr_t` | Critical-path depth of the lowered sequence. |
| `total_ops` | `uintptr_t` | Total operation count of the lowered sequence (global phase excluded). |
| `parameterized_ops` | `uintptr_t` | Operations carrying parameters in the lowered sequence. |

### COneQubitUnitaryDecomposition

Snapshot of a one-qubit numeric synthesis result (the output of `synthesize_numeric_1q_unitary`); the represented matrix is `exp(i * global_phase) * U(theta, phi, lambda)`:

| Field | Type | Meaning |
| --- | --- | --- |
| `theta` | `double` | Polar rotation angle. |
| `phi` | `double` | First azimuthal angle. |
| `lambda` | `double` | Second azimuthal angle. |
| `global_phase` | `double` | Scalar phase multiplying the synthesized gate. |

---

## Examples

First a chain of circuit-level decomposition passes (definition expansion -> unitary synthesis -> MC-gate rewriting -> two-qubit block resynthesis), following the call order of the test suite:

```c
#include <math.h>
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. A frozen circuit-gate is expanded back into plain operations */
    struct CCircuit *inner = circuit_new(1);
    circuit_h(inner, 0);
    struct CCircuitGate *gate = circuit_to_gate(inner, "my_h");
    struct CCircuit *outer = circuit_new(2);
    uint32_t gate_qubits[1] = {0};
    circuit_circuit_gate(outer, gate, gate_qubits, 1, NULL, 0);
    struct CCircuit *expanded = decompose_expand_definitions(outer);  /* 1 op */

    /* 2. A matrix-backed H is synthesized into standard operations */
    struct CCircuit *unitary_circuit = circuit_new(2);
    Complex64 h_matrix[4] = {
        {1.0 / sqrt(2.0), 0.0}, {1.0 / sqrt(2.0), 0.0},
        {1.0 / sqrt(2.0), 0.0}, {-1.0 / sqrt(2.0), 0.0},
    };
    uint32_t unitary_targets[1] = {0};
    circuit_unitary(unitary_circuit, "my_u", 1, h_matrix, unitary_targets, 1);
    struct CDecompositionRuleStats stats;
    struct CCircuit *synthesized = decompose_unitaries_with_rule_stats(
        unitary_circuit, decompose_unitary_config_default(), &stats);
    /* stats.hits / stats.misses / stats.inserts reflect the rule cache */

    /* 3. A doubly-controlled X is rewritten within device capacity */
    struct CCircuit *mc_circuit = circuit_new(3);
    uint32_t controls[2] = {0, 1};
    uint32_t targets[1] = {2};
    circuit_multi_control(mc_circuit, "X", controls, 2, targets, 1, NULL, 0);
    uint32_t edges[4] = {0, 1, 1, 2};
    struct CDevice *device = device_from_edges("line-3", 3, edges, 2);
    struct CCircuit *rewritten = decompose_mc_gates_for_device(mc_circuit, device, NULL);

    /* 4. Two-qubit blocks are resynthesized under the enhanced budget */
    struct CCircuit *block_circuit = circuit_new(2);
    circuit_h(block_circuit, 0);
    circuit_cx(block_circuit, 0, 1);
    struct CCircuit *resynthesized =
        resynthesize_two_qubit_blocks(block_circuit, resynthesis_config_enhanced());

    circuit_free(resynthesized);
    circuit_free(block_circuit);
    circuit_free(rewritten);
    device_free(device);
    circuit_free(mc_circuit);
    circuit_free(synthesized);
    circuit_free(unitary_circuit);
    circuit_free(expanded);
    circuit_gate_free(gate);
    circuit_free(outer);
    circuit_free(inner);
    return 0;
}
```

Numeric synthesis, KAK decomposition, and lowering (the 2q synthesis and the KAK coordinates of the identity are all zero):

```c
#include <math.h>
#include <stdio.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. 1q synthesis of the identity yields U(0, 0, 0), zero phase */
    Complex64 identity2[4] = {{1.0, 0.0}, {0.0, 0.0}, {0.0, 0.0}, {1.0, 0.0}};
    struct COneQubitUnitaryDecomposition one_q;
    memset(&one_q, 0, sizeof(one_q));
    if (synthesize_numeric_1q_unitary(identity2, &one_q) != 0) {
        return 1;
    }

    /* 2. 2q synthesis of the identity in the CX basis */
    Complex64 identity4[16];
    memset(identity4, 0, sizeof(identity4));
    for (int i = 0; i < 16; i += 5) {
        identity4[i].re = 1.0;
    }
    struct CTwoQubitUnitarySynthesis *synthesis =
        synthesize_numeric_2q_unitary(identity4, 0, 1, TWO_QUBIT_BASIS_CX);
    if (synthesis != NULL) {
        for (uintptr_t i = 0; i < two_qubit_synthesis_num_operations(synthesis); i++) {
            char *name = two_qubit_synthesis_operation_name(synthesis, i);
            uint32_t qubits[8];
            uintptr_t len = two_qubit_synthesis_operation_qubits_len(synthesis, i);
            uintptr_t copied = two_qubit_synthesis_operation_qubits(synthesis, i, qubits, len);
            printf("op=%s qubits=%lu\n", name, (unsigned long)copied);
            cqlib_string_free(name);
        }
        two_qubit_synthesis_free(synthesis);
    }

    /* 3. KAK decomposition of the identity: zero interaction coordinates */
    struct CKakDecomposition *kak = kak_decompose(identity4);
    if (kak != NULL) {
        double a = 0.0, b = 0.0, c = 0.0;
        Complex64 k1l[4];
        if (kak_decomposition_coordinates(kak, &a, &b, &c) == 0) {
            printf("a=%f b=%f c=%f\n", a, b, c);  /* ~0 */
        }
        kak_decomposition_local_factor(kak, KAK_FACTOR_K1L, k1l);  /* 0 */
        kak_decomposition_free(kak);
    }

    /* 4. Target-basis lowering: T is outside {H, RZ, CX} */
    const char *basis_names[3] = {"H", "RZ", "CX"};
    struct CTargetBasisLowerer *lowerer = target_basis_lowerer_new(basis_names, 3);
    if (lowerer != NULL) {
        struct CCircuit *t_circuit = circuit_new(1);
        circuit_t(t_circuit, 0);
        if (target_basis_lowerer_requires_lowering(lowerer, t_circuit) == 1) {
            struct CCircuit *lowered = target_basis_lowerer_apply(lowerer, t_circuit);
            circuit_free(lowered);
        }
        circuit_free(t_circuit);
        target_basis_lowerer_free(lowerer);
    }

    /* 5. Device lowering: a native circuit passes through unchanged */
    uint32_t edges[2] = {0, 1};
    struct CDevice *device = device_from_edges("line-2", 2, edges, 1);
    device_with_native_gates(device, "H,RZ,CX");
    struct CCircuit *native_circuit = circuit_new(2);
    circuit_h(native_circuit, 0);
    circuit_cx(native_circuit, 0, 1);
    struct CCircuit *device_lowered = decompose_lower_to_device(native_circuit, device);  /* 2 ops */

    circuit_free(device_lowered);
    circuit_free(native_circuit);
    device_free(device);
    return 0;
}
```

A unitary synthesis target: native-basis read-back and cost evaluation:

```c
#include <stdio.h>
#include <string.h>
#include "cqlib_c.h"

static void sort_names(char **names, uintptr_t len) {
    for (uintptr_t i = 0; i + 1 < len; i++) {
        for (uintptr_t j = i + 1; j < len; j++) {
            if (strcmp(names[i], names[j]) > 0) {
                char *tmp = names[i];
                names[i] = names[j];
                names[j] = tmp;
            }
        }
    }
}

int main(void) {
    /* 1. Build a target from a workflow-style basis list */
    const char *const basis_gates[3] = {"H", "RZ", "CX"};
    struct CDecomposeUnitaryTarget *target =
        decompose_unitary_target_from_instructions(basis_gates, 3);
    if (target == NULL) {
        return 1;
    }
    /* decompose_unitary_target_fallback_pauli(target) == 1,
       native_1q_len == 2 (H, RZ), native_2q_len == 1 (CX), basis_len == 3 */

    /* 2. Two-step read of the combined basis (1q gates first) */
    char *basis[3] = {NULL, NULL, NULL};
    uintptr_t total = decompose_unitary_target_basis(target, basis, 3);
    sort_names(basis, total);
    for (uintptr_t i = 0; i < total && i < 3; i++) {
        printf("basis[%llu]=%s\n", (unsigned long long)i, basis[i]);
        cqlib_string_free(basis[i]);
    }

    /* 3. Cost of a fixed operation sequence under the native basis */
    const char *const ops[3] = {"H", "CX", "H"};
    const uint32_t qubit_ids[4] = {0, 0, 1, 1};
    const uint32_t op_counts[3] = {1, 2, 1};
    struct CTargetBasisCost cost;
    memset(&cost, 0, sizeof(cost));
    if (decompose_unitary_target_cost_of_fixed_operations(target, ops, qubit_ids,
                                                          op_counts, 3, &cost) == 0) {
        /* cost.total_ops == 3, cost.two_qubit_ops == 1,
           cost.parameterized_ops == 0, cost.depth == 3 */
        printf("ops=%llu two_qubit=%llu depth=%llu\n",
               (unsigned long long)cost.total_ops,
               (unsigned long long)cost.two_qubit_ops,
               (unsigned long long)cost.depth);
    }

    /* 4. A parameterized gate is counted as such */
    const char *const rz_ops[1] = {"RZ"};
    const uint32_t rz_counts[1] = {1};
    decompose_unitary_target_cost_of_fixed_operations(target, rz_ops, qubit_ids,
                                                      rz_counts, 1, &cost);
    /* cost.total_ops == 1, cost.parameterized_ops == 1 */

    /* 5. Toggling the fallback rebuilds the target around its native lists */
    decompose_unitary_target_set_fallback_pauli(target, 0);
    /* decompose_unitary_target_fallback_pauli(target) == 0 */
    decompose_unitary_target_set_fallback_pauli(target, 1);

    decompose_unitary_target_free(target);
    return 0;
}
```
