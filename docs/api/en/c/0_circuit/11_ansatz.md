# Ansatz

This page covers the variational circuit templates in the C API: TwoLocal, BasicEntanglerLayers, QAOAAnsatz, the feature-map family (ZFeatureMap, ZZFeatureMap, IQPFeatureMap, PauliFeatureMap), StronglyEntanglingLayers, and the Pauli evolution ansatz, plus the facade builders and entanglement topology helpers. Each template follows the flow "create handle → configure → validate → build circuit", and the generated circuit names its symbolic parameters with a `{prefix}` prefix. See [Overview](../0_overview.md) for the error-code and memory-management conventions.

---

## Tag constants

Entanglement topology tags (used by `two_local_entanglement`, the feature maps, the facade builders, and the topology helpers):

| Topology tag | Value | Meaning |
| --- | --- | --- |
| `ENTANGLEMENT_LINEAR` | 0 | Linear nearest-neighbor chain. |
| `ENTANGLEMENT_CIRCULAR` | 1 | Linear chain plus wrap-around edge. |
| `ENTANGLEMENT_FULL` | 2 | All-to-all pairs. |
| `ENTANGLEMENT_CUSTOM` | 3 | Custom explicit pairs. |

Evolution strategy tags (used by `qaoa_ansatz_evolution_strategy`):

| Strategy tag | Value | Meaning |
| --- | --- | --- |
| `EVOLUTION_STRATEGY_EXACT` | 0 | Exact term-wise Pauli evolution. |
| `EVOLUTION_STRATEGY_AUTO` | 1 | Automatic exact/Trotter selection (uses `steps`). |
| `EVOLUTION_STRATEGY_TROTTER` | 2 | Explicit Trotter product formula (uses `mode` / `steps` / `seed`). |

Trotter mode tags (`mode` values accepted by `EVOLUTION_STRATEGY_TROTTER`, plus the `trotter_mode` value reported by `pauli_evolution_ansatz_evolution_info`):

| Mode tag | Value | Meaning |
| --- | --- | --- |
| `TROTTER_FIRST_ORDER` | 0 | First-order product formula. |
| `TROTTER_SECOND_ORDER` | 1 | Second-order product formula. |
| `TROTTER_RANDOMIZED` | 2 | Randomized first-order formula (`seed` is the random seed). |
| `TROTTER_MODE_NONE` | -1 | No Trotter decomposition was emitted (single-pass exact path); only reported in `CPauliEvolutionInfo.trotter_mode`, never accepted as a `mode` input. |

---

## TwoLocal

TwoLocal alternates single-qubit rotation layers with multi-qubit entanglement layers, suitable for VQE, quantum machine learning, and general variational circuit construction. `two_local_new` defaults to: `reps = 3`, rotation `[RY]`, entangler `CX`, linear topology, final rotation layer kept.

### two_local_new(num_qubits)

Creates a TwoLocal template handle.

- `num_qubits` (`uintptr_t`): number of qubits.

Returns a newly allocated `CTwoLocal*`.

### two_local_free(ptr)

Frees a TwoLocal handle.

- `ptr` (`CTwoLocal*`): handle to free; NULL is allowed.

No return value.

### two_local_reps(ptr, reps)

Sets the number of repetition layers.

- `ptr` (`CTwoLocal*`): template handle.
- `reps` (`uintptr_t`): number of repetitions.

Returns 0 on success, -1 on NULL input.

### two_local_rotation_gates(ptr, names, len)

Sets the per-layer single-qubit parameterized rotation gates by name (must be RX, RY, or RZ; validated on build).

- `ptr` (`CTwoLocal*`): template handle.
- `names` (`const char* const*`): array of gate names.
- `len` (`uintptr_t`): array length.

Returns 0 on success, -1 on NULL input, -4 on invalid UTF-8, -8 on an unknown gate name.

### two_local_entanglement_gate(ptr, name)

Sets the two-qubit entanglement gate by name (CX, CY, or CZ; validated on build).

- `ptr` (`CTwoLocal*`): template handle.
- `name` (`const char*`): gate name.

Returns 0 on success, -1 on NULL input, -4 on invalid UTF-8, -8 on an unknown name.

### two_local_entanglement(ptr, topology, pairs, pairs_len)

Sets the entanglement topology. For `ENTANGLEMENT_CUSTOM`, `pairs` is a flat array of `pairs_len / 2` control/target qubit-id pairs.

- `ptr` (`CTwoLocal*`): template handle.
- `topology` (`int32_t`): an `ENTANGLEMENT_*` topology tag.
- `pairs` (`const uint32_t*`): flat array of custom pairs (NULL for other topologies).
- `pairs_len` (`uintptr_t`): length of the flat array.

Returns 0 on success, -1 on NULL input (including `pairs` NULL while `pairs_len > 0`), -8 on an unknown topology or an odd pair count.

### two_local_skip_final_rotation_layer(ptr, skip)

Sets whether to skip the final rotation layer.

- `ptr` (`CTwoLocal*`): template handle.
- `skip` (`bool`): true to skip.

Returns 0 on success, -1 on NULL input.

### two_local_validate(ptr)

Validates the template configuration.

- `ptr` (`const CTwoLocal*`): template handle.

Returns 0 when the configuration is valid, -1 on NULL input, -3 on an invalid configuration.

### two_local_num_parameters(ptr)

Returns the number of independent parameters required by the generated circuit.

- `ptr` (`const CTwoLocal*`): template handle.

Returns the parameter count, or 0 for NULL.

### two_local_num_qubits(ptr)

Returns the number of qubits the template acts on.

- `ptr` (`const CTwoLocal*`): template handle.

Returns the qubit count, or 0 for NULL.

### two_local_build_circuit(ptr, prefix)

Builds the parameterized circuit from the current configuration; parameters are named `"{prefix}_{index}"`.

- `ptr` (`const CTwoLocal*`): template handle.
- `prefix` (`const char*`): parameter name prefix.

Returns a newly allocated `CCircuit*` (free with `circuit_free`); NULL on NULL input, invalid UTF-8, or build failure.

### TwoLocal example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CTwoLocal *ansatz = two_local_new(3);

    two_local_reps(ansatz, 2);
    const char *rotations[] = {"RY", "RZ"};
    two_local_rotation_gates(ansatz, rotations, 2);
    two_local_entanglement_gate(ansatz, "CX");
    two_local_entanglement(ansatz, ENTANGLEMENT_LINEAR, NULL, 0);

    if (two_local_validate(ansatz) != 0) {
        two_local_free(ansatz);
        return 1;
    }

    /* Parameters are named "theta_0", "theta_1", ... */
    struct CCircuit *circuit = two_local_build_circuit(ansatz, "theta");
    if (circuit == NULL) {
        two_local_free(ansatz);
        return 1;
    }
    printf("parameters: %zu, qubits: %zu\n",
           two_local_num_parameters(ansatz),
           two_local_num_qubits(ansatz));

    circuit_free(circuit);
    two_local_free(ansatz);
    return 0;
}
```

---

## BasicEntanglerLayers

BasicEntanglerLayers consists of single-qubit rotation layers and a fixed-pattern entanglement layer, suitable for training circuits with a simple structure and a controlled parameter count. `basic_entangler_layers_new` defaults to: `reps = 1`, rotation `RX`, entangler `CX`.

### basic_entangler_layers_new(num_qubits)

Creates a BasicEntanglerLayers template handle.

- `num_qubits` (`uintptr_t`): number of qubits.

Returns a newly allocated `CBasicEntanglerLayers*`.

### basic_entangler_layers_free(ptr)

Frees a BasicEntanglerLayers handle.

- `ptr` (`CBasicEntanglerLayers*`): handle to free; NULL is allowed.

No return value.

### basic_entangler_layers_reps(ptr, reps)

Sets the number of layers.

- `ptr` (`CBasicEntanglerLayers*`): template handle.
- `reps` (`uintptr_t`): number of layers.

Returns 0 on success, -1 on NULL input.

### basic_entangler_layers_rotation_gate(ptr, name)

Sets the single-qubit rotation gate by name (RX, RY, RZ, or Phase).

- `ptr` (`CBasicEntanglerLayers*`): template handle.
- `name` (`const char*`): gate name.

Returns 0 on success, -1 on NULL input, -4 on invalid UTF-8, -8 on an unknown gate name.

### basic_entangler_layers_entanglement_gate(ptr, name)

Sets the entanglement gate by name (CX, CY, or CZ).

- `ptr` (`CBasicEntanglerLayers*`): template handle.
- `name` (`const char*`): gate name.

Returns 0 on success, -1 on NULL input, -4 on invalid UTF-8, -8 on an unknown gate name.

### basic_entangler_layers_num_parameters(ptr)

Returns the number of independent parameters required by the generated circuit.

- `ptr` (`const CBasicEntanglerLayers*`): template handle.

Returns the parameter count, or 0 for NULL.

### basic_entangler_layers_num_qubits(ptr)

Returns the number of qubits the template acts on.

- `ptr` (`const CBasicEntanglerLayers*`): template handle.

Returns the qubit count, or 0 for NULL.

### basic_entangler_layers_build_circuit(ptr, prefix)

Builds the parameterized circuit from the current configuration; parameters are named `"{prefix}_{index}"`.

- `ptr` (`const CBasicEntanglerLayers*`): template handle.
- `prefix` (`const char*`): parameter name prefix.

Returns a newly allocated `CCircuit*` (free with `circuit_free`); NULL on NULL input, invalid UTF-8, or build failure.

---

## QAOAAnsatz

QAOAAnsatz builds parameterized circuits for the Quantum Approximate Optimization Algorithm (QAOA), alternating time evolution under the cost Hamiltonian and the mixer Hamiltonian:

```text
U(β, γ) = ∏_l exp(-i β_l H_M) exp(-i γ_l H_C)
```

`qaoa_ansatz_new` infers the qubit count from the cost Hamiltonian, defaults the mixer to the standard X-mixer, and sets `reps = 1`. Each layer contributes one `γ_l` and one `β_l` parameter, so the total parameter count is `2 * reps`, with parameters named `"{prefix}_gamma_{layer}"` / `"{prefix}_beta_{layer}"`.

### qaoa_ansatz_new(cost)

Creates a QAOA template from a cost Hamiltonian (the Hamiltonian is cloned).

- `cost` (`const CHamiltonian*`): cost Hamiltonian handle.

Returns a newly allocated `CQAOAAnsatz*`; NULL on NULL input or an invalid cost operator.

### qaoa_ansatz_free(ptr)

Frees a QAOA handle.

- `ptr` (`CQAOAAnsatz*`): handle to free; NULL is allowed.

No return value.

### qaoa_ansatz_reps(ptr, reps)

Sets the number of alternating layers (QAOA depth p).

- `ptr` (`CQAOAAnsatz*`): template handle.
- `reps` (`uintptr_t`): number of layers.

Returns 0 on success, -1 on NULL input.

### qaoa_ansatz_mixer(ptr, mixer)

Overrides the mixer Hamiltonian (cloned; must match the cost operator's qubit count).

- `ptr` (`CQAOAAnsatz*`): template handle.
- `mixer` (`const CHamiltonian*`): mixer Hamiltonian handle.

Returns 0 on success, -1 on NULL input, -3 on a qubit-count mismatch or an invalid mixer.

### qaoa_ansatz_initial_state(ptr, circuit)

Prepends an initial-state circuit (cloned; must match the qubit count).

- `ptr` (`CQAOAAnsatz*`): template handle.
- `circuit` (`const CCircuit*`): initial-state circuit handle.

Returns 0 on success, -1 on NULL input, -3 on a qubit-count mismatch.

### qaoa_ansatz_evolution_strategy(ptr, strategy, mode, steps, seed)

Sets the Hamiltonian evolution strategy. `EVOLUTION_STRATEGY_EXACT` ignores the remaining arguments; `EVOLUTION_STRATEGY_AUTO` uses `steps`; `EVOLUTION_STRATEGY_TROTTER` uses `mode` (`TROTTER_FIRST_ORDER`, `TROTTER_SECOND_ORDER`, or `TROTTER_RANDOMIZED`), `steps`, and `seed`.

- `ptr` (`CQAOAAnsatz*`): template handle.
- `strategy` (`int32_t`): an `EVOLUTION_STRATEGY_*` strategy tag.
- `mode` (`int32_t`): Trotter mode tag, used only by the TROTTER strategy.
- `steps` (`uintptr_t`): Trotter step count, ignored by the EXACT strategy.
- `seed` (`uint64_t`): random seed, used only by `TROTTER_RANDOMIZED`.

Returns 0 on success, -1 on NULL input, -8 on an unknown tag.

### qaoa_ansatz_validate(ptr)

Validates the template configuration.

- `ptr` (`const CQAOAAnsatz*`): template handle.

Returns 0 when the configuration is valid, -1 on NULL input, -3 on an invalid configuration.

### qaoa_ansatz_num_parameters(ptr)

Returns the number of parameters (2 per layer).

- `ptr` (`const CQAOAAnsatz*`): template handle.

Returns the parameter count, or 0 for NULL.

### qaoa_ansatz_num_qubits(ptr)

Returns the number of qubits the template acts on.

- `ptr` (`const CQAOAAnsatz*`): template handle.

Returns the qubit count, or 0 for NULL.

### qaoa_ansatz_build_circuit(ptr, prefix)

Builds the parameterized circuit from the current configuration; parameters are named `"{prefix}_gamma_{layer}"` / `"{prefix}_beta_{layer}"`.

- `ptr` (`const CQAOAAnsatz*`): template handle.
- `prefix` (`const char*`): parameter name prefix.

Returns a newly allocated `CCircuit*` (free with `circuit_free`); NULL on NULL input, invalid UTF-8, or build failure.

### QAOA example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* Cost Hamiltonian: H_C = Z0 * Z1 */
    struct CPauliString *zz = pauli_string_parse("ZZ");
    struct CHamiltonian *cost = hamiltonian_from_pauli(zz);  /* takes ownership of zz */

    struct CQAOAAnsatz *ansatz = qaoa_ansatz_new(cost);
    qaoa_ansatz_reps(ansatz, 2);
    qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_AUTO,
                                   TROTTER_FIRST_ORDER, 1, 0);

    if (qaoa_ansatz_validate(ansatz) != 0) {
        qaoa_ansatz_free(ansatz);
        hamiltonian_free(cost);
        return 1;
    }

    /* Parameters are named "q_gamma_0", "q_beta_0", "q_gamma_1", "q_beta_1" */
    struct CCircuit *circuit = qaoa_ansatz_build_circuit(ansatz, "q");
    if (circuit == NULL) {
        qaoa_ansatz_free(ansatz);
        hamiltonian_free(cost);
        return 1;
    }
    printf("parameters: %zu, qubits: %zu\n",
           qaoa_ansatz_num_parameters(ansatz),
           qaoa_ansatz_num_qubits(ansatz));

    circuit_free(circuit);
    qaoa_ansatz_free(ansatz);
    hamiltonian_free(cost);
    return 0;
}
```

---

## Feature maps

Feature maps encode a classical feature vector `x_0, x_1, ...` into a quantum state and are the standard input layer for quantum machine learning and quantum kernel methods. All four templates (ZFeatureMap, ZZFeatureMap, IQPFeatureMap, PauliFeatureMap) share the same conventions: each repetition layer starts from a Hadamard layer, the symbolic parameters are named `"{prefix}_{index}"` with one parameter per input feature, and the same parameter objects are reused across repetition layers, so `*_num_parameters` reports `num_qubits` (0 when `reps = 0`, which builds an empty circuit). Note that `circuit_num_parameters` counts distinct angle-expression objects: a constant such as `pi` embedded inside an angle expression is not a separate parameter (see the ZZFeatureMap example below).

### ZFeatureMap

ZFeatureMap is the non-entangling first-order map: each repetition layer applies a Hadamard layer followed by an independent `RZ(2 * x_i)` rotation on every qubit. `z_feature_map_new` defaults to `reps = 2`.

### z_feature_map_new(num_qubits)

Creates a ZFeatureMap template handle.

- `num_qubits` (`uintptr_t`): number of qubits (= number of input features).

Returns a newly allocated `CZFeatureMap*`.

### z_feature_map_free(ptr)

Frees a ZFeatureMap handle.

- `ptr` (`CZFeatureMap*`): handle to free; NULL is allowed.

No return value.

### z_feature_map_reps(ptr, reps)

Sets the number of repetition layers.

- `ptr` (`CZFeatureMap*`): template handle.
- `reps` (`uintptr_t`): number of repetitions.

Returns 0 on success, -1 on NULL input.

### z_feature_map_validate(ptr)

Validates the template configuration (requires at least 1 qubit).

- `ptr` (`const CZFeatureMap*`): template handle.

Returns 0 when the configuration is valid, -1 on NULL input, -3 on an invalid configuration.

### z_feature_map_num_parameters(ptr)

Returns the number of input features (one parameter per qubit; 0 when `reps = 0`).

- `ptr` (`const CZFeatureMap*`): template handle.

Returns the parameter count, or 0 for NULL.

### z_feature_map_num_qubits(ptr)

Returns the number of qubits the template acts on.

- `ptr` (`const CZFeatureMap*`): template handle.

Returns the qubit count, or 0 for NULL.

### z_feature_map_build_circuit(ptr, prefix)

Builds the parameterized circuit from the current configuration; parameters are named `"{prefix}_{index}"`.

- `ptr` (`const CZFeatureMap*`): template handle.
- `prefix` (`const char*`): parameter name prefix.

Returns a newly allocated `CCircuit*` (free with `circuit_free`); NULL on NULL input, invalid UTF-8, or build failure.

### ZZFeatureMap

ZZFeatureMap adds second-order ZZ interactions: each repetition layer applies a Hadamard layer, `RZ(2 * x_i)` on each qubit, then for every pair `(i, j)` of the entanglement topology a Pauli evolution `e^{-i * 2 * (pi - x_i) * (pi - x_j) * Z_i Z_j}`. Defaults: `reps = 2`, full entanglement. Custom topologies are stored by the setter and checked by validation: out-of-bounds indices, self-loops, and duplicate (undirected) edges are rejected.

### zz_feature_map_new(num_qubits)

Creates a ZZFeatureMap template handle.

- `num_qubits` (`uintptr_t`): number of qubits (= number of input features).

Returns a newly allocated `CZZFeatureMap*`.

### zz_feature_map_free(ptr)

Frees a ZZFeatureMap handle.

- `ptr` (`CZZFeatureMap*`): handle to free; NULL is allowed.

No return value.

### zz_feature_map_reps(ptr, reps)

Sets the number of repetition layers.

- `ptr` (`CZZFeatureMap*`): template handle.
- `reps` (`uintptr_t`): number of repetitions.

Returns 0 on success, -1 on NULL input.

### zz_feature_map_entanglement(ptr, topology, pairs, pairs_len)

Sets the entanglement topology for the ZZ interactions. For `ENTANGLEMENT_CUSTOM`, `pairs` is a flat array of `pairs_len / 2` control/target qubit-id pairs.

- `ptr` (`CZZFeatureMap*`): template handle.
- `topology` (`int32_t`): an `ENTANGLEMENT_*` topology tag.
- `pairs` (`const uint32_t*`): flat array of custom pairs (NULL for other topologies).
- `pairs_len` (`uintptr_t`): length of the flat array.

Returns 0 on success, -1 on NULL input (including `pairs` NULL while `pairs_len > 0`), -8 on an unknown topology or an odd pair count.

### zz_feature_map_validate(ptr)

Validates the template configuration (at least 1 qubit; for custom topologies, in-bounds indices, no self-loops, no duplicate edges).

- `ptr` (`const CZZFeatureMap*`): template handle.

Returns 0 when the configuration is valid, -1 on NULL input, -3 on an invalid configuration.

### zz_feature_map_num_parameters(ptr)

Returns the number of input features (one parameter per qubit; 0 when `reps = 0`).

- `ptr` (`const CZZFeatureMap*`): template handle.

Returns the parameter count, or 0 for NULL.

### zz_feature_map_num_qubits(ptr)

Returns the number of qubits the template acts on.

- `ptr` (`const CZZFeatureMap*`): template handle.

Returns the qubit count, or 0 for NULL.

### zz_feature_map_build_circuit(ptr, prefix)

Builds the parameterized circuit from the current configuration; parameters are named `"{prefix}_{index}"`.

- `ptr` (`const CZZFeatureMap*`): template handle.
- `prefix` (`const char*`): parameter name prefix.

Returns a newly allocated `CCircuit*` (free with `circuit_free`); NULL on NULL input, invalid UTF-8, or build failure.

### ZZFeatureMap example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CZZFeatureMap *map = zz_feature_map_new(2);

    /* Reports the 2 input features x_0, x_1. */
    printf("features: %zu\n", zz_feature_map_num_parameters(map));

    /* circuit_num_parameters counts distinct angle expressions: two RZ
       angles plus the shared ZZ expression 4 * (pi - x_i) * (pi - x_j);
       the pi constant inside the expression is not a separate parameter. */
    struct CCircuit *circuit = zz_feature_map_build_circuit(map, "x");
    if (circuit == NULL) {
        zz_feature_map_free(map);
        return 1;
    }
    printf("circuit parameters: %zu\n", circuit_num_parameters(circuit));

    circuit_free(circuit);
    zz_feature_map_free(map);
    return 0;
}
```

### IQPFeatureMap

IQPFeatureMap is the IQP-style diagonal encoding. It applies the same circuit structure as ZZFeatureMap — Hadamard layer, first-order Z rotations, and second-order ZZ interactions following the entanglement topology — under a dedicated handle. Defaults: `reps = 2`, full entanglement.

### iqp_feature_map_new(num_qubits)

Creates an IQPFeatureMap template handle.

- `num_qubits` (`uintptr_t`): number of qubits (= number of input features).

Returns a newly allocated `CIQPFeatureMap*`.

### iqp_feature_map_free(ptr)

Frees an IQPFeatureMap handle.

- `ptr` (`CIQPFeatureMap*`): handle to free; NULL is allowed.

No return value.

### iqp_feature_map_reps(ptr, reps)

Sets the number of repetition layers.

- `ptr` (`CIQPFeatureMap*`): template handle.
- `reps` (`uintptr_t`): number of repetitions.

Returns 0 on success, -1 on NULL input.

### iqp_feature_map_entanglement(ptr, topology, pairs, pairs_len)

Sets the entanglement topology for the two-qubit diagonal interactions. For `ENTANGLEMENT_CUSTOM`, `pairs` is a flat array of `pairs_len / 2` control/target qubit-id pairs.

- `ptr` (`CIQPFeatureMap*`): template handle.
- `topology` (`int32_t`): an `ENTANGLEMENT_*` topology tag.
- `pairs` (`const uint32_t*`): flat array of custom pairs (NULL for other topologies).
- `pairs_len` (`uintptr_t`): length of the flat array.

Returns 0 on success, -1 on NULL input (including `pairs` NULL while `pairs_len > 0`), -8 on an unknown topology or an odd pair count.

### iqp_feature_map_validate(ptr)

Validates the template configuration (at least 1 qubit; for custom topologies, in-bounds indices, no self-loops, no duplicate edges).

- `ptr` (`const CIQPFeatureMap*`): template handle.

Returns 0 when the configuration is valid, -1 on NULL input, -3 on an invalid configuration.

### iqp_feature_map_num_parameters(ptr)

Returns the number of input features (one parameter per qubit; 0 when `reps = 0`).

- `ptr` (`const CIQPFeatureMap*`): template handle.

Returns the parameter count, or 0 for NULL.

### iqp_feature_map_num_qubits(ptr)

Returns the number of qubits the template acts on.

- `ptr` (`const CIQPFeatureMap*`): template handle.

Returns the qubit count, or 0 for NULL.

### iqp_feature_map_build_circuit(ptr, prefix)

Builds the parameterized circuit from the current configuration; parameters are named `"{prefix}_{index}"`.

- `ptr` (`const CIQPFeatureMap*`): template handle.
- `prefix` (`const char*`): parameter name prefix.

Returns a newly allocated `CCircuit*` (free with `circuit_free`); NULL on NULL input, invalid UTF-8, or build failure.

### PauliFeatureMap

PauliFeatureMap generalizes the Z and ZZ maps to arbitrary Pauli templates. Each repetition layer applies a Hadamard layer followed, for every configured Pauli template with support size `k` and every `k`-tuple of qubits drawn from the entanglement topology, by a Pauli evolution:

- `k = 1`: `e^{-i * x_i * P}` with angle `2 * x_i`;
- `k >= 2`: `e^{-i * 2 * prod_j (pi - x_j) * P}` with angle `4 * prod_j (pi - x_j)`.

Templates whose support is empty contribute only a global phase and are skipped. Defaults: `reps = 2`, Pauli templates `["Z", "ZZ"]`, full entanglement, parameter prefix `"x"`. Passing an empty prefix to `pauli_feature_map_build_circuit` falls back to the configured parameter prefix.

### pauli_feature_map_new(num_qubits)

Creates a PauliFeatureMap template handle.

- `num_qubits` (`uintptr_t`): number of qubits (= number of input features).

Returns a newly allocated `CPauliFeatureMap*`.

### pauli_feature_map_free(ptr)

Frees a PauliFeatureMap handle.

- `ptr` (`CPauliFeatureMap*`): handle to free; NULL is allowed.

No return value.

### pauli_feature_map_reps(ptr, reps)

Sets the number of repetition layers.

- `ptr` (`CPauliFeatureMap*`): template handle.
- `reps` (`uintptr_t`): number of repetitions.

Returns 0 on success, -1 on NULL input.

### pauli_feature_map_paulis(ptr, texts, labels, len)

Sets the Pauli evolution templates. `texts` and `labels` are parallel arrays of `len` NUL-terminated C strings; each text is parsed as a PauliString (for example `"Z"`, `"ZZ"`, `"XY"`) and each label names the template. A template longer than the qubit count is stored but fails validation. A failing call leaves the previous configuration in place.

- `ptr` (`CPauliFeatureMap*`): template handle.
- `texts` (`const char* const*`): array of Pauli string texts.
- `labels` (`const char* const*`): array of template labels.
- `len` (`uintptr_t`): array length.

Returns 0 on success, -1 on NULL input (handle, arrays while `len > 0`, or an individual element), -4 on invalid UTF-8, -8 on a Pauli string that fails to parse.

### pauli_feature_map_entanglement(ptr, topology, pairs, pairs_len)

Sets the entanglement topology for the multi-qubit interactions. For `ENTANGLEMENT_CUSTOM`, `pairs` is a flat array of `pairs_len / 2` control/target qubit-id pairs.

- `ptr` (`CPauliFeatureMap*`): template handle.
- `topology` (`int32_t`): an `ENTANGLEMENT_*` topology tag.
- `pairs` (`const uint32_t*`): flat array of custom pairs (NULL for other topologies).
- `pairs_len` (`uintptr_t`): length of the flat array.

Returns 0 on success, -1 on NULL input (including `pairs` NULL while `pairs_len > 0`), -8 on an unknown topology or an odd pair count.

### pauli_feature_map_parameter_prefix(ptr, prefix)

Sets the fallback parameter prefix used when `pauli_feature_map_build_circuit` receives an empty prefix.

- `ptr` (`CPauliFeatureMap*`): template handle.
- `prefix` (`const char*`): parameter name prefix.

Returns 0 on success, -1 on NULL input, -4 on invalid UTF-8.

### pauli_feature_map_validate(ptr)

Validates the template configuration (at least 1 qubit; no Pauli template longer than the qubit count; for custom topologies, in-bounds indices, no self-loops, no duplicate edges).

- `ptr` (`const CPauliFeatureMap*`): template handle.

Returns 0 when the configuration is valid, -1 on NULL input, -3 on an invalid configuration.

### pauli_feature_map_num_parameters(ptr)

Returns the number of input features (one parameter per qubit; 0 when `reps = 0`).

- `ptr` (`const CPauliFeatureMap*`): template handle.

Returns the parameter count, or 0 for NULL.

### pauli_feature_map_num_qubits(ptr)

Returns the number of qubits the template acts on.

- `ptr` (`const CPauliFeatureMap*`): template handle.

Returns the qubit count, or 0 for NULL.

### pauli_feature_map_build_circuit(ptr, prefix)

Builds the parameterized circuit from the current configuration; parameters are named `"{prefix}_{index}"`. An empty prefix falls back to the configured parameter prefix.

- `ptr` (`const CPauliFeatureMap*`): template handle.
- `prefix` (`const char*`): parameter name prefix (empty string uses the configured fallback).

Returns a newly allocated `CCircuit*` (free with `circuit_free`); NULL on NULL input, invalid UTF-8, or build failure.

### PauliFeatureMap example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CPauliFeatureMap *map = pauli_feature_map_new(2);

    const char *texts[] = {"Z", "ZZ"};
    const char *labels[] = {"Z", "ZZ"};
    pauli_feature_map_paulis(map, texts, labels, 2);
    pauli_feature_map_parameter_prefix(map, "p");

    if (pauli_feature_map_validate(map) != 0) {
        pauli_feature_map_free(map);
        return 1;
    }

    /* An empty prefix falls back to "p": parameters are named "p_0", "p_1". */
    struct CCircuit *circuit = pauli_feature_map_build_circuit(map, "");
    if (circuit == NULL) {
        pauli_feature_map_free(map);
        return 1;
    }
    printf("features: %zu\n", pauli_feature_map_num_parameters(map));

    circuit_free(circuit);
    pauli_feature_map_free(map);
    return 0;
}
```

---

## Facade builders

The facade constructors create pre-configured handles for common templates in one call. They return the same handle types as the corresponding `*_new` constructors, so the resulting objects work with all of the `two_local_*`, `zz_feature_map_*`, and `pauli_feature_map_*` functions. All four return NULL when the topology arguments are invalid (unknown tag, an odd custom pair count, or `pairs` NULL while `pairs_len > 0`); `pauli_feature_map_build` also returns NULL when a Pauli string fails to parse.

### efficient_su2(num_qubits, reps, topology, pairs, pairs_len)

Creates an EfficientSU2 ansatz: `[RY, RZ]` rotation layers with `CX` entanglement and the final rotation layer kept, giving `(reps + 1) * num_qubits * 2` parameters.

- `num_qubits` (`uintptr_t`): number of qubits.
- `reps` (`uintptr_t`): number of repetition layers.
- `topology` (`int32_t`): an `ENTANGLEMENT_*` topology tag.
- `pairs` (`const uint32_t*`): flat array of custom pairs (NULL for other topologies).
- `pairs_len` (`uintptr_t`): length of the flat array.

Returns a newly allocated `CTwoLocal*`, or NULL on invalid topology arguments.

### real_amplitudes(num_qubits, reps, topology, pairs, pairs_len)

Creates a RealAmplitudes ansatz: `[RY]` rotation layers with `CX` entanglement and the final rotation layer kept, giving `(reps + 1) * num_qubits` parameters.

- `num_qubits` (`uintptr_t`): number of qubits.
- `reps` (`uintptr_t`): number of repetition layers.
- `topology` (`int32_t`): an `ENTANGLEMENT_*` topology tag.
- `pairs` (`const uint32_t*`): flat array of custom pairs (NULL for other topologies).
- `pairs_len` (`uintptr_t`): length of the flat array.

Returns a newly allocated `CTwoLocal*`, or NULL on invalid topology arguments.

### zz_feature_map_build(num_qubits, reps, topology, pairs, pairs_len)

Creates a ZZFeatureMap with the given repetition count and topology.

- `num_qubits` (`uintptr_t`): number of qubits (= number of input features).
- `reps` (`uintptr_t`): number of repetition layers.
- `topology` (`int32_t`): an `ENTANGLEMENT_*` topology tag.
- `pairs` (`const uint32_t*`): flat array of custom pairs (NULL for other topologies).
- `pairs_len` (`uintptr_t`): length of the flat array.

Returns a newly allocated `CZZFeatureMap*`, or NULL on invalid topology arguments.

### pauli_feature_map_build(num_qubits, reps, texts, labels, paulis_len, topology, pairs, pairs_len)

Creates a PauliFeatureMap with explicit Pauli templates. `texts` and `labels` are parallel arrays of `paulis_len` NUL-terminated C strings; each text is parsed as a PauliString.

- `num_qubits` (`uintptr_t`): number of qubits (= number of input features).
- `reps` (`uintptr_t`): number of repetition layers.
- `texts` (`const char* const*`): array of Pauli string texts.
- `labels` (`const char* const*`): array of template labels.
- `paulis_len` (`uintptr_t`): length of the two arrays.
- `topology` (`int32_t`): an `ENTANGLEMENT_*` topology tag.
- `pairs` (`const uint32_t*`): flat array of custom pairs (NULL for other topologies).
- `pairs_len` (`uintptr_t`): length of the flat array.

Returns a newly allocated `CPauliFeatureMap*`, or NULL on invalid topology arguments or an unparsable Pauli string.

### Facade example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* EfficientSU2: [RY, RZ] + CX -> (1 + 1) * 2 * 2 = 8 parameters */
    struct CTwoLocal *su2 = efficient_su2(2, 1, ENTANGLEMENT_FULL, NULL, 0);
    printf("efficient_su2 parameters: %zu\n", two_local_num_parameters(su2));
    two_local_free(su2);

    /* RealAmplitudes: [RY] + CX -> (2 + 1) * 3 = 9 parameters */
    struct CTwoLocal *ra = real_amplitudes(3, 2, ENTANGLEMENT_LINEAR, NULL, 0);
    printf("real_amplitudes parameters: %zu\n", two_local_num_parameters(ra));
    two_local_free(ra);

    /* ZZ feature map with a circular topology */
    struct CZZFeatureMap *zz = zz_feature_map_build(3, 2, ENTANGLEMENT_CIRCULAR, NULL, 0);
    printf("zz_feature_map features: %zu\n", zz_feature_map_num_parameters(zz));
    zz_feature_map_free(zz);
    return 0;
}
```

---

## Entanglement topology helpers

These helpers expand an `ENTANGLEMENT_*` topology tag into explicit qubit groups with the same semantics the templates use. They use a two-step output pattern: call the `*_len` function first to obtain the required flat length, allocate the buffer, then call the copy function.

### entanglement_generate_pairs_len(topology, pairs, pairs_len, num_qubits)

Returns the flattened length produced by `entanglement_generate_pairs` (two entries per pair), or 0 when the arguments are invalid (unknown topology tag, odd custom pair count, or `pairs` NULL while `pairs_len > 0`). For `ENTANGLEMENT_CUSTOM`, the custom pairs are echoed verbatim and `num_qubits` is ignored.

- `topology` (`int32_t`): an `ENTANGLEMENT_*` topology tag.
- `pairs` (`const uint32_t*`): flat array of custom pairs (NULL for other topologies).
- `pairs_len` (`uintptr_t`): length of the flat array.
- `num_qubits` (`uintptr_t`): qubit count used by the predefined topologies.

Returns the required length of the `out` buffer for `entanglement_generate_pairs`, or 0 on invalid arguments.

### entanglement_generate_pairs(topology, pairs, pairs_len, num_qubits, out, len)

Copies the flattened pairs `[c0, t0, c1, t1, ...]` into `out`. Predefined topologies generate: linear `(0,1), (1,2), ...`; circular adds the wrap-around edge `(n-1, 0)`; full all-to-all pairs in lexicographic order.

- `topology` (`int32_t`): an `ENTANGLEMENT_*` topology tag.
- `pairs` (`const uint32_t*`): flat array of custom pairs (NULL for other topologies).
- `pairs_len` (`uintptr_t`): length of the flat array.
- `num_qubits` (`uintptr_t`): qubit count used by the predefined topologies.
- `out` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer length returned by `entanglement_generate_pairs_len`.

Returns 0 on success, -1 on NULL input (`out` NULL while `len > 0`), -8 when `len` does not match the required length or an argument is invalid.

### entanglement_generate_k_tuples_len(topology, pairs, pairs_len, k, num_qubits)

Returns the flattened length produced by `entanglement_generate_k_tuples` (`k` entries per tuple), or 0 on an invalid argument combination: unknown topology or malformed custom pairs, `k = 0`, `k > num_qubits`, or `num_qubits = 0`.

- `topology` (`int32_t`): an `ENTANGLEMENT_*` topology tag.
- `pairs` (`const uint32_t*`): flat array of custom pairs (NULL for other topologies).
- `pairs_len` (`uintptr_t`): length of the flat array.
- `k` (`uintptr_t`): tuple size.
- `num_qubits` (`uintptr_t`): qubit count.

Returns the required length of the `out` buffer for `entanglement_generate_k_tuples`, or 0 on invalid arguments.

### entanglement_generate_k_tuples(topology, pairs, pairs_len, k, num_qubits, out, len)

Copies the flattened k-tuples `[t0_0, t0_1, ..., t1_0, ...]` into `out`. `k = 1` yields each qubit on its own; `k = 2` yields the topology's pairs (custom pairs included); for `k >= 3`, linear yields consecutive windows `(i, ..., i+k-1)`, circular yields rotational windows mod n, and full yields lexicographic k-combinations — custom topologies only define pairs and fall back to full.

- `topology` (`int32_t`): an `ENTANGLEMENT_*` topology tag.
- `pairs` (`const uint32_t*`): flat array of custom pairs (NULL for other topologies).
- `pairs_len` (`uintptr_t`): length of the flat array.
- `k` (`uintptr_t`): tuple size.
- `num_qubits` (`uintptr_t`): qubit count.
- `out` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer length returned by `entanglement_generate_k_tuples_len`.

Returns 0 on success, -1 on NULL input (`out` NULL while `len > 0`), -8 when `len` does not match the required length or an argument is invalid.

### Entanglement helpers example

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    /* Two-step pattern: query the length first, then copy. */
    uintptr_t len = entanglement_generate_pairs_len(ENTANGLEMENT_CIRCULAR, NULL, 0, 4);
    uint32_t *pairs = malloc(len * sizeof(uint32_t));

    if (entanglement_generate_pairs(ENTANGLEMENT_CIRCULAR, NULL, 0, 4, pairs, len) == 0) {
        /* prints: 0 1 1 2 2 3 3 0 */
        for (uintptr_t i = 0; i < len; i++) {
            printf("%u ", pairs[i]);
        }
        printf("\n");
    }
    free(pairs);
    return 0;
}
```

---

## StronglyEntanglingLayers

StronglyEntanglingLayers applies, in each layer, a U gate (three parameters) on every qubit followed by an entangler connecting qubit `i` to qubit `(i + r) mod num_qubits`, where `r` is the layer's range. Defaults: `reps = 1`, entangler `CX`, ranges cycling through `1..num_qubits` (layer `l` uses `(l mod (num_qubits - 1)) + 1`). Parameters are named `"{prefix}_{index}"` and numbered consecutively; the total count is `reps * num_qubits * 3`.

### strongly_entangling_layers_new(num_qubits)

Creates a StronglyEntanglingLayers template handle.

- `num_qubits` (`uintptr_t`): number of qubits.

Returns a newly allocated `CStronglyEntanglingLayers*`.

### strongly_entangling_layers_free(ptr)

Frees a StronglyEntanglingLayers handle.

- `ptr` (`CStronglyEntanglingLayers*`): handle to free; NULL is allowed.

No return value.

### strongly_entangling_layers_reps(ptr, reps)

Sets the number of layers.

- `ptr` (`CStronglyEntanglingLayers*`): template handle.
- `reps` (`uintptr_t`): number of layers.

Returns 0 on success, -1 on NULL input.

### strongly_entangling_layers_entanglement_gate(ptr, name)

Sets the two-qubit entangling gate by name (CX, CY, or CZ).

- `ptr` (`CStronglyEntanglingLayers*`): template handle.
- `name` (`const char*`): gate name.

Returns 0 on success, -1 on NULL input, -4 on invalid UTF-8, -8 on an unknown gate name.

### strongly_entangling_layers_ranges(ptr, ranges, len)

Sets explicit entanglement ranges reused cyclically across layers (layer `l` uses `ranges[l mod len]`). Each range must be in `1..num_qubits` at validation time. An empty list (`len == 0`) stores the empty list, which fails validation until replaced.

- `ptr` (`CStronglyEntanglingLayers*`): template handle.
- `ranges` (`const uintptr_t*`): array of ranges.
- `len` (`uintptr_t`): array length.

Returns 0 on success, -1 on NULL input (handle, or `ranges` NULL while `len > 0`).

### strongly_entangling_layers_validate(ptr)

Validates the template configuration (at least 1 qubit, a known entangler, and — when set — a non-empty ranges list with every range in `1..num_qubits`).

- `ptr` (`const CStronglyEntanglingLayers*`): template handle.

Returns 0 when the configuration is valid, -1 on NULL input, -3 on an invalid configuration.

### strongly_entangling_layers_num_parameters(ptr)

Returns the number of independent parameters (`reps * num_qubits * 3`).

- `ptr` (`const CStronglyEntanglingLayers*`): template handle.

Returns the parameter count, or 0 for NULL.

### strongly_entangling_layers_num_qubits(ptr)

Returns the number of qubits the template acts on.

- `ptr` (`const CStronglyEntanglingLayers*`): template handle.

Returns the qubit count, or 0 for NULL.

### strongly_entangling_layers_build_circuit(ptr, prefix)

Builds the parameterized circuit from the current configuration; parameters are named `"{prefix}_{index}"`.

- `ptr` (`const CStronglyEntanglingLayers*`): template handle.
- `prefix` (`const char*`): parameter name prefix.

Returns a newly allocated `CCircuit*` (free with `circuit_free`); NULL on NULL input, invalid UTF-8, or build failure.

### StronglyEntanglingLayers example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CStronglyEntanglingLayers *layers = strongly_entangling_layers_new(3);

    strongly_entangling_layers_reps(layers, 2);
    uintptr_t ranges[] = {1, 2};  /* reused cyclically: layer 0 -> 1, layer 1 -> 2 */
    strongly_entangling_layers_ranges(layers, ranges, 2);
    strongly_entangling_layers_entanglement_gate(layers, "CZ");

    if (strongly_entangling_layers_validate(layers) != 0) {
        strongly_entangling_layers_free(layers);
        return 1;
    }

    /* 2 layers * 3 qubits * 3 U-angles = 18 parameters named "t_0", "t_1", ... */
    struct CCircuit *circuit = strongly_entangling_layers_build_circuit(layers, "t");
    if (circuit == NULL) {
        strongly_entangling_layers_free(layers);
        return 1;
    }
    printf("parameters: %zu\n", strongly_entangling_layers_num_parameters(layers));

    circuit_free(circuit);
    strongly_entangling_layers_free(layers);
    return 0;
}
```

---

## PauliEvolutionAnsatz

PauliEvolutionAnsatz represents time evolution under a Hamiltonian. `pauli_evolution_ansatz_new` clones the input Hamiltonian and simplifies it (duplicate terms merged, phases absorbed into coefficients, near-zero terms removed); the default evolution strategy is automatic exact/Trotter selection with one step.

### pauli_evolution_ansatz_new(hamiltonian)

Creates a Pauli evolution ansatz from a Hamiltonian (cloned and simplified in the process).

- `hamiltonian` (`const CHamiltonian*`): Hamiltonian handle.

Returns a newly allocated `CPauliEvolutionAnsatz*`; NULL on NULL input or an empty operator.

### pauli_evolution_ansatz_free(ptr)

Frees a Pauli evolution ansatz handle.

- `ptr` (`CPauliEvolutionAnsatz*`): handle to free; NULL is allowed.

No return value.

### pauli_evolution_ansatz_with_time_param_name(ptr, name)

Overrides the time-parameter symbol used by the evolution lowering. By default the symbol is derived as `"{prefix}_t"`; setting a name uses that exact name. An empty name resets it to the default.

- `ptr` (`CPauliEvolutionAnsatz*`): template handle.
- `name` (`const char*`): new time-parameter symbol (empty string resets to the default).

Returns 0 on success, -1 on NULL input, -4 on invalid UTF-8.

### pauli_evolution_ansatz_evolution_info(ptr, out)

Reads the evolution strategy info (the C mirror of `PauliEvolutionAnsatz::evolution_info`) into `out`. `out` describes the strategy actually used: the decomposition emitted for the current configuration and the structure of the simplified Hamiltonian, which also reveals what `EVOLUTION_STRATEGY_AUTO` selected.

- `ptr` (`const CPauliEvolutionAnsatz*`): template handle.
- `out` (`CPauliEvolutionInfo*`): output struct receiving the snapshot.

Returns 0 on success, -1 on NULL input (`ptr` or `out`).

`CPauliEvolutionInfo` is a transparent struct with a stable layout:

| Field | Type | Meaning |
| --- | --- | --- |
| `is_exact` | `uint8_t` | 1 when the decomposition is mathematically exact, 0 otherwise. |
| `all_terms_commute` | `uint8_t` | 1 when all Hamiltonian terms mutually commute, 0 otherwise. |
| `_pad` | `uint8_t[6]` | Reserved padding to keep the struct layout stable. |
| `steps` | `uintptr_t` | Number of decomposition repetitions emitted into the circuit. |
| `num_terms` | `uintptr_t` | Number of Pauli terms in the (simplified) Hamiltonian. |
| `trotter_mode` | `int32_t` | `TROTTER_FIRST_ORDER`, `TROTTER_SECOND_ORDER`, or `TROTTER_RANDOMIZED`; `TROTTER_MODE_NONE` when the single-pass exact path was used. |
| `trotter_seed` | `uint64_t` | Seed for `TROTTER_RANDOMIZED`; 0 otherwise. |

### PauliEvolutionAnsatz example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* Evolution under H = X on one qubit. */
    struct CPauliString *px = pauli_string_parse("X");
    struct CHamiltonian *ham = hamiltonian_from_pauli(px);  /* takes ownership of px */

    struct CPauliEvolutionAnsatz *ansatz = pauli_evolution_ansatz_new(ham);
    if (ansatz == NULL) {
        hamiltonian_free(ham);
        return 1;
    }

    /* Rename the time parameter, then reset it to the default. */
    pauli_evolution_ansatz_with_time_param_name(ansatz, "tau");
    pauli_evolution_ansatz_with_time_param_name(ansatz, "");

    /* Read the strategy actually used: the single commuting term takes the
       single-pass exact path (trotter_mode == TROTTER_MODE_NONE). */
    struct CPauliEvolutionInfo info;
    if (pauli_evolution_ansatz_evolution_info(ansatz, &info) == 0) {
        printf("exact=%d commute=%d steps=%zu terms=%zu mode=%d\n",
               info.is_exact, info.all_terms_commute,
               info.steps, info.num_terms, info.trotter_mode);
    }

    pauli_evolution_ansatz_free(ansatz);
    hamiltonian_free(ham);
    return 0;
}
```

---

See [Hamiltonian](../3_qis/7_hamiltonian.md) for the Hamiltonian constructors (`hamiltonian_from_pauli` / `hamiltonian_free`), [Pauli](../3_qis/6_pauli.md) for Pauli string parsing (`pauli_string_parse`), and [Circuit](1_circuit.md) for circuit handle management (`circuit_free`, `circuit_num_parameters`).
