# Transform (C)

This page covers the C ABI of the compile transform stage: the structural-analysis snapshot `CCircuitAnalysis`, the handle-based configurations `CTransformConfig` and `CCanonicalizeConfig`, canonicalization with `canonicalize_circuit` / `canonicalize_circuit_with_config`, knowledge-rule rewriting with `rewrite_circuit`, and the five standalone pass wrappers `transform_*`. No entry point modifies its input circuit: analysis snapshots are written into caller-provided structs, and transforms return new handles or new circuits. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## Constants

Rewrite mode tags (the `CRewriteConfig.mode` field):

| Constant | Value | Mode |
| --- | --- | --- |
| `REWRITE_MODE_OPTIMIZE` | 0 | Conservative optimization (production default): accepted rewrites must strictly improve the local cost. |
| `REWRITE_MODE_LOWERING` | 1 | Explicit lowering: decomposition and hardware-native rules may expand locally. |

Rewrite rule-category tags (used by `transform_config_with_enabled_kinds` and `transform_config_enabled_kinds`):

| Constant | Value | Kind |
| --- | --- | --- |
| `REWRITE_KIND_SIMPLIFY` | 0 | Algebraic simplification. |
| `REWRITE_KIND_CANCEL` | 1 | Inverse / repeat cancellation. |
| `REWRITE_KIND_MERGE` | 2 | Neighbor-gate merge. |
| `REWRITE_KIND_COMMUTE` | 3 | Explicit commutation. |
| `REWRITE_KIND_DECOMPOSE` | 4 | Decomposition / lowering. |
| `REWRITE_KIND_CANONICALIZE` | 5 | Canonical representation. |
| `REWRITE_KIND_HARDWARE_NATIVE` | 6 | Hardware-native rewriting. |
| `REWRITE_KIND_OTHER` | 7 | Other. |

---

## Rewrite Configuration

`CRewriteConfig` is the input configuration of the knowledge rewriter, passed by value:

```c
typedef struct CRewriteConfig {
  uint8_t mode;                          /* One of REWRITE_MODE_*. */
  uint8_t max_rounds;                    /* Fixpoint round limit. */
  uint8_t recurse_control_flow;          /* 1 recurses into control-flow bodies. */
  uint8_t skip_labeled_ops;             /* 1 skips labeled operations. */
  uint8_t preserve_two_qubit_connectivity; /* 1 preserves undirected 2q connectivity. */
  uint8_t _pad[3];                       /* Reserved padding. */
  uintptr_t max_window_ops;              /* Matching-window operation limit. */
  uintptr_t max_pattern_len;             /* Maximum matched pattern length. */
  const char *const *target_gate_names;  /* Borrowed target-basis gate names (NULL = none). */
  uintptr_t target_gate_names_len;       /* Number of entries in target_gate_names. */
} CRewriteConfig;
```

| Field | Type | Meaning |
| --- | --- | --- |
| `mode` | `uint8_t` | Rule-set baseline, one of `REWRITE_MODE_*`: `OPTIMIZE` uses the production defaults, `LOWERING` adds decomposition and hardware-native rules. |
| `max_rounds` | `uint8_t` | Fixpoint round limit. |
| `recurse_control_flow` | `uint8_t` | 1 recurses into control-flow bodies, 0 does not. |
| `skip_labeled_ops` | `uint8_t` | 1 skips labeled operations, 0 does not. |
| `preserve_two_qubit_connectivity` | `uint8_t` | 1 preserves undirected two-qubit connectivity (rewrites may not split adjacent two-qubit pairs), 0 does not require it. |
| `max_window_ops` | `uintptr_t` | Operation limit of the rule matching window. |
| `max_pattern_len` | `uintptr_t` | Maximum length of a single matched pattern. |
| `target_gate_names` | `const char *const *` | Borrowed target-basis gate names (each a NUL-terminated string); NULL when no target basis is configured. Set with `rewrite_config_with_target_instructions`. |
| `target_gate_names_len` | `uintptr_t` | Number of entries in `target_gate_names`. |
| `_pad[3]` | — | Padding that keeps the struct layout stable; do not write non-zero values. |

Every field except `mode` overrides the baseline: start from `rewrite_config_default()` / `rewrite_config_lowering()` and adjust the fields you need. The `target_gate_names` array and its strings are borrowed, not copied: they must stay valid until the `rewrite_circuit` / `transform_knowledge_rewrite` call that consumes the configuration.

### rewrite_config_default()

```c
struct CRewriteConfig rewrite_config_default(void);
```

Returns the production optimization configuration by value (`REWRITE_MODE_OPTIMIZE`): `max_rounds = 8`, `recurse_control_flow = 1`, `skip_labeled_ops = 1`, `preserve_two_qubit_connectivity = 0`, `max_window_ops = 16`, `max_pattern_len = 8`.

### rewrite_config_lowering()

```c
struct CRewriteConfig rewrite_config_lowering(void);
```

Returns the explicit lowering configuration by value: identical field values to `rewrite_config_default()` except `mode = REWRITE_MODE_LOWERING`, which adds decomposition and hardware-native rules to the rule set.

### rewrite_config_with_target_instructions(config, gate_names, len)

```c
int32_t rewrite_config_with_target_instructions(struct CRewriteConfig *config,
                                                const char *const *gate_names,
                                                uintptr_t len);
```

Restricts a by-value rewrite configuration to an explicit standard-gate target-instruction basis, parsed from `len` gate names (e.g. `"H"`, `"CX"`). The name array is borrowed (see the field table above) and validated eagerly; an empty basis is rejected and the configuration is left unchanged. The target basis only takes effect when `config` uses `REWRITE_MODE_LOWERING`.

- `config` (`CRewriteConfig*`): rewrite configuration, modified in place.
- `gate_names` (`const char *const *`): array of `len` NUL-terminated standard-gate names.
- `len` (`uintptr_t`): number of entries in `gate_names`.

Returns: `0` on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `config` is NULL, or `len` is greater than 0 but `gate_names` is NULL or contains a NULL entry. |
| `-4` | ParseError | An entry is invalid UTF-8 or an unknown gate name. |
| `-6` | CompilerError | The core rejected the basis. |
| `-8` | InvalidParam | The basis is empty (`len` is 0); the configuration is left unchanged. |

---

## Transform Configuration Handle

`CTransformConfig` is an opaque handle around the same core rewrite / transform configuration whose by-value C form is `CRewriteConfig`. Unlike the by-value struct it lives on the heap and is modified in place: every `transform_config_with_*` setter rewrites the configuration inside the handle and returns an `int32_t` status code — setters are not chainable. Rule-category fields use the `REWRITE_KIND_*` tags from the [Constants](#constants) section.

### transform_config_default()

```c
struct CTransformConfig *transform_config_default(void);
```

Creates a transform configuration with production (conservative optimization) defaults: `mode = REWRITE_MODE_OPTIMIZE`, `max_rounds = 8`, `max_window_ops = 16`, `max_pattern_len = 8`, control-flow recursion and labeled-operation skipping enabled, the enabled rule categories `SIMPLIFY`, `CANCEL`, `MERGE`, `CANONICALIZE`, and no target-instruction basis.

Returns: a new `CTransformConfig*` on success (free with `transform_config_free`); NULL on allocation failure.

### transform_config_lowering()

```c
struct CTransformConfig *transform_config_lowering(void);
```

Creates a transform configuration with explicit knowledge-based lowering defaults: identical to `transform_config_default()` except `mode = REWRITE_MODE_LOWERING`, which widens the enabled rule set to six categories (adding `DECOMPOSE` and `HARDWARE_NATIVE`).

Returns: a new `CTransformConfig*` on success (free with `transform_config_free`); NULL on allocation failure.

### transform_config_free(ptr)

```c
void transform_config_free(struct CTransformConfig *ptr);
```

Frees a transform configuration; NULL is allowed.

### transform_config_with_mode(ptr, mode)

```c
int32_t transform_config_with_mode(struct CTransformConfig *ptr, uint8_t mode);
```

Sets the rewrite mode of `config` in place; `mode` must be one of `REWRITE_MODE_*`.

Returns: `0` on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` is NULL. |
| `-8` | InvalidParam | `mode` is not a valid `REWRITE_MODE_*` tag. |

### transform_config_with_max_rounds(ptr, max_rounds)

```c
int32_t transform_config_with_max_rounds(struct CTransformConfig *ptr, uint8_t max_rounds);
```

Sets the fixpoint round limit of the configuration in place.

Returns: `0` on success; `-1` (`NullPtr`) when `ptr` is NULL.

### transform_config_with_max_window_ops(ptr, max_window_ops)

```c
int32_t transform_config_with_max_window_ops(struct CTransformConfig *ptr,
                                             uintptr_t max_window_ops);
```

Sets the matching-window operation limit of the configuration in place.

Returns: `0` on success; `-1` (`NullPtr`) when `ptr` is NULL.

### transform_config_with_max_pattern_len(ptr, max_pattern_len)

```c
int32_t transform_config_with_max_pattern_len(struct CTransformConfig *ptr,
                                              uintptr_t max_pattern_len);
```

Sets the maximum matched pattern length of the configuration in place.

Returns: `0` on success; `-1` (`NullPtr`) when `ptr` is NULL.

### transform_config_with_enabled_kinds(ptr, kinds, len)

```c
int32_t transform_config_with_enabled_kinds(struct CTransformConfig *ptr,
                                            const uint8_t *kinds,
                                            uintptr_t len);
```

Replaces the enabled rule categories of the configuration in place: `kinds` points at `len` `REWRITE_KIND_*` tags (NULL when `len` is 0, which disables every rule category). When the call is rejected the configuration is left unchanged.

Returns: `0` on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` is NULL, or `len` is greater than 0 but `kinds` is NULL. |
| `-8` | InvalidParam | A tag in `kinds` is not a valid `REWRITE_KIND_*` tag. |

### transform_config_with_target_instructions(ptr, gate_names, len)

```c
int32_t transform_config_with_target_instructions(struct CTransformConfig *ptr,
                                                  const char *const *gate_names,
                                                  uintptr_t len);
```

Restricts the configuration to an explicit standard-gate target-instruction basis, parsed from `len` gate names (e.g. `"H"`, `"CX"`). An empty basis is rejected; when the call is rejected the configuration is left unchanged.

Returns: `0` on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` is NULL, or `len` is greater than 0 but `gate_names` is NULL or contains a NULL entry. |
| `-4` | ParseError | An entry is invalid UTF-8 or an unknown gate name. |
| `-6` | CompilerError | The core rejected the basis. |
| `-8` | InvalidParam | The basis is empty (`len` is 0). |

### transform_config_mode(ptr)

```c
int32_t transform_config_mode(const struct CTransformConfig *ptr);
```

Returns the `REWRITE_MODE_*` mode tag of the configuration.

Returns: the mode tag; `-1` for NULL input.

### transform_config_max_rounds(ptr)

```c
uint8_t transform_config_max_rounds(const struct CTransformConfig *ptr);
```

Returns the fixpoint round limit of the configuration.

Returns: the round limit; 0 for NULL input.

### transform_config_max_window_ops(ptr)

```c
uintptr_t transform_config_max_window_ops(const struct CTransformConfig *ptr);
```

Returns the matching-window operation limit of the configuration.

Returns: the operation limit; 0 for NULL input.

### transform_config_max_pattern_len(ptr)

```c
uintptr_t transform_config_max_pattern_len(const struct CTransformConfig *ptr);
```

Returns the maximum matched pattern length of the configuration.

Returns: the pattern-length limit; 0 for NULL input.

### transform_config_recurse_control_flow(ptr)

```c
int32_t transform_config_recurse_control_flow(const struct CTransformConfig *ptr);
```

Tests whether the configuration recurses into control-flow bodies.

Returns: `1` when enabled; `0` otherwise; `-1` for NULL input.

### transform_config_skip_labeled_ops(ptr)

```c
int32_t transform_config_skip_labeled_ops(const struct CTransformConfig *ptr);
```

Tests whether the configuration skips labeled operations.

Returns: `1` when enabled; `0` otherwise; `-1` for NULL input.

### transform_config_enabled_kinds_len(ptr) / transform_config_enabled_kinds(ptr, out, len)

```c
uintptr_t transform_config_enabled_kinds_len(const struct CTransformConfig *ptr);
uintptr_t transform_config_enabled_kinds(const struct CTransformConfig *ptr,
                                         uint8_t *out,
                                         uintptr_t len);
```

Two-step read of the enabled rule categories (step one: `*_len` returns the total; step two: the fill function copies `REWRITE_KIND_*` tags into `out`, writing `min(total, len)` entries). When `out` is NULL the fill function only returns the total without copying.

Returns (both functions): the total number of enabled categories; 0 for NULL input.

### transform_config_target_instruction_basis_len(ptr) / transform_config_target_instruction_basis(ptr, out, len)

```c
uintptr_t transform_config_target_instruction_basis_len(const struct CTransformConfig *ptr);
uintptr_t transform_config_target_instruction_basis(const struct CTransformConfig *ptr,
                                                    char **out,
                                                    uintptr_t len);
```

Two-step read of the configured target-basis instruction names (0 when no target basis is set). The fill function copies the gate names into `out`, writing `min(total, len)` entries; each written entry is a freshly allocated C string freed with `cqlib_string_free`. When `out` is NULL it only returns the total without copying.

Returns (both functions): the total number of basis instructions; 0 for NULL input or when no target basis is set.

---

## Structural Analysis

`CCircuitAnalysis` is a snapshot of circuit facts; every flag is 0 or 1:

```c
typedef struct CCircuitAnalysis {
  uint8_t has_classical_data;
  uint8_t has_classical_control;
  uint8_t has_measurement;
  uint8_t has_classical_values;
  uint8_t has_classical_vars;
  uint8_t has_runtime_classical;
  uint8_t needs_classical_handle_preservation;
  uint8_t has_circuit_gate_definitions;
  uint8_t has_unitary_circuit_definitions;
  uint8_t has_unitary_gates;
  uint8_t has_mc_gates;
} CCircuitAnalysis;
```

| Field | Meaning |
| --- | --- |
| `has_classical_data` | The circuit contains classical-data operations. |
| `has_classical_control` | The circuit contains classical control flow. |
| `has_measurement` | The circuit contains measurement operations. |
| `has_classical_values` | The circuit registers classical values. |
| `has_classical_vars` | The circuit registers classical variables. |
| `has_runtime_classical` | The circuit uses any runtime classical construct. |
| `needs_classical_handle_preservation` | Rewrites must preserve classical handles for this circuit. |
| `has_circuit_gate_definitions` | The circuit contains unexpanded circuit-backed gate definitions. |
| `has_unitary_circuit_definitions` | The circuit contains unitary definitions with circuit bodies. |
| `has_unitary_gates` | The circuit contains matrix-backed unitary gates. |
| `has_mc_gates` | The circuit contains multi-controlled gates. |

### circuit_analyze(circuit, out)

```c
int32_t circuit_analyze(const struct CCircuit *circuit, struct CCircuitAnalysis *out);
```

Analyzes the circuit and writes the structural snapshot to `*out`.

- `circuit` (`const CCircuit*`): input circuit.
- `out` (`CCircuitAnalysis*`): snapshot output.

Returns: `0` on success; `-1` when either argument is NULL.

---

## Canonicalization

The canonicalizer unifies instruction forms, folds global phases, and removes no-ops; its outcome is carried by a `CCanonicalizeResult` handle. The pass runs with production defaults under `canonicalize_circuit`, or with a caller-built `CCanonicalizeConfig` handle under `canonicalize_circuit_with_config`.

### canonicalize_circuit(circuit)

```c
struct CCanonicalizeResult *canonicalize_circuit(const struct CCircuit *circuit);
```

Canonicalizes the circuit with production defaults; the input circuit is not modified.

- `circuit` (`const CCircuit*`): input circuit.

Returns: a new `CCanonicalizeResult*` on success (free with `canonicalize_result_free`); NULL on NULL input or execution failure.

### canonicalize_circuit_with_config(circuit, config)

```c
struct CCanonicalizeResult *canonicalize_circuit_with_config(const struct CCircuit *circuit,
                                                             const struct CCanonicalizeConfig *config);
```

Canonicalizes the circuit using the supplied configuration; the input circuit is not modified and the config is not consumed — it remains owned by the caller and can be reused.

- `circuit` (`const CCircuit*`): input circuit.
- `config` (`const CCanonicalizeConfig*`): canonicalize configuration handle (see below).

Returns: a new `CCanonicalizeResult*` on success (free with `canonicalize_result_free`); NULL on NULL input or execution failure.

### canonicalize_result_free(ptr)

```c
void canonicalize_result_free(struct CCanonicalizeResult *ptr);
```

Frees a canonicalize result handle; NULL is allowed.

### canonicalize_result_circuit(ptr)

```c
struct CCircuit *canonicalize_result_circuit(const struct CCanonicalizeResult *ptr);
```

Retrieves the canonicalized circuit (a deep clone, independent of `ptr`).

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL on NULL input or failure.

### canonicalize_result_changed(ptr)

```c
int32_t canonicalize_result_changed(const struct CCanonicalizeResult *ptr);
```

Tests whether canonicalization changed the circuit representation.

Returns: `1` when changed; `0` otherwise; `-1` for NULL.

### canonicalize_result_rounds(ptr)

```c
uint8_t canonicalize_result_rounds(const struct CCanonicalizeResult *ptr);
```

Returns the number of canonicalization rounds executed.

Returns: the round count; 0 for NULL input.

### canonicalize_config_default()

```c
struct CCanonicalizeConfig *canonicalize_config_default(void);
```

Creates a canonicalize configuration with production defaults: round limit 8 and every optional behavior enabled (control-flow recursion, `GPhase` folding, instruction-form canonicalization, strict no-op removal, barrier canonicalization).

Returns: a new `CCanonicalizeConfig*` on success (free with `canonicalize_config_free`); NULL on allocation failure.

### canonicalize_config_free(ptr)

```c
void canonicalize_config_free(struct CCanonicalizeConfig *ptr);
```

Frees a canonicalize configuration; NULL is allowed.

### canonicalize_config_with_round_limit(ptr, round_limit)

```c
int32_t canonicalize_config_with_round_limit(struct CCanonicalizeConfig *ptr, uint8_t round_limit);
```

Sets the maximum number of canonicalization rounds in place.

Returns: `0` on success; `-1` (`NullPtr`) when `ptr` is NULL.

### canonicalize_config_with_recurse_control_flow(ptr, enabled)

```c
int32_t canonicalize_config_with_recurse_control_flow(struct CCanonicalizeConfig *ptr,
                                                      int32_t enabled);
```

Sets whether control-flow bodies are recursively canonicalized (non-zero `enabled`) or not (`0`) in place.

Returns: `0` on success; `-1` (`NullPtr`) when `ptr` is NULL.

### canonicalize_config_with_fold_gphase(ptr, enabled)

```c
int32_t canonicalize_config_with_fold_gphase(struct CCanonicalizeConfig *ptr, int32_t enabled);
```

Sets whether `GPhase` operations are folded into scope-local phase, in place.

Returns: `0` on success; `-1` (`NullPtr`) when `ptr` is NULL.

### canonicalize_config_with_canonicalize_instruction_form(ptr, enabled)

```c
int32_t canonicalize_config_with_canonicalize_instruction_form(struct CCanonicalizeConfig *ptr,
                                                               int32_t enabled);
```

Sets whether `McGate` forms are collapsed into existing standard gates, in place.

Returns: `0` on success; `-1` (`NullPtr`) when `ptr` is NULL.

### canonicalize_config_with_drop_noops(ptr, enabled)

```c
int32_t canonicalize_config_with_drop_noops(struct CCanonicalizeConfig *ptr, int32_t enabled);
```

Sets whether strict no-op removal is performed, in place.

Returns: `0` on success; `-1` (`NullPtr`) when `ptr` is NULL.

### canonicalize_config_with_canonicalize_barriers(ptr, enabled)

```c
int32_t canonicalize_config_with_canonicalize_barriers(struct CCanonicalizeConfig *ptr,
                                                       int32_t enabled);
```

Sets whether barrier scopes are canonicalized and adjacent barriers merged, in place.

Returns: `0` on success; `-1` (`NullPtr`) when `ptr` is NULL.

### canonicalize_config_round_limit(ptr)

```c
uint8_t canonicalize_config_round_limit(const struct CCanonicalizeConfig *ptr);
```

Returns the configured round limit.

Returns: the round limit; 0 for NULL input.

### canonicalize_config_recurse_control_flow(ptr)

```c
int32_t canonicalize_config_recurse_control_flow(const struct CCanonicalizeConfig *ptr);
```

Tests whether control-flow recursion is enabled.

Returns: `1` when enabled; `0` otherwise; `-1` for NULL input.

### canonicalize_config_fold_gphase(ptr)

```c
int32_t canonicalize_config_fold_gphase(const struct CCanonicalizeConfig *ptr);
```

Tests whether `GPhase` folding is enabled.

Returns: `1` when enabled; `0` otherwise; `-1` for NULL input.

### canonicalize_config_canonicalize_instruction_form(ptr)

```c
int32_t canonicalize_config_canonicalize_instruction_form(const struct CCanonicalizeConfig *ptr);
```

Tests whether instruction-form canonicalization is enabled.

Returns: `1` when enabled; `0` otherwise; `-1` for NULL input.

### canonicalize_config_drop_noops(ptr)

```c
int32_t canonicalize_config_drop_noops(const struct CCanonicalizeConfig *ptr);
```

Tests whether strict no-op removal is enabled.

Returns: `1` when enabled; `0` otherwise; `-1` for NULL input.

### canonicalize_config_canonicalize_barriers(ptr)

```c
int32_t canonicalize_config_canonicalize_barriers(const struct CCanonicalizeConfig *ptr);
```

Tests whether barrier canonicalization is enabled.

Returns: `1` when enabled; `0` otherwise; `-1` for NULL input.

---

## Knowledge Rewrite

The knowledge rewriter performs pattern-based simplification and cancellation through knowledge rules; it is the core of the pipeline's logical optimization. Its outcome is carried by a `CKnowledgeRewriteResult` handle.

### rewrite_circuit(circuit, config)

```c
struct CKnowledgeRewriteResult *rewrite_circuit(const struct CCircuit *circuit,
                                                struct CRewriteConfig config);
```

Applies the knowledge rewriter to the circuit with the supplied configuration; the input circuit is not modified.

- `circuit` (`const CCircuit*`): input circuit.
- `config` (`struct CRewriteConfig`): rewrite configuration, passed by value.

Returns: a new `CKnowledgeRewriteResult*` on success (free with `knowledge_rewrite_result_free`); NULL on NULL input, an invalid `config.mode` or target-instruction basis, or execution failure.

### knowledge_rewrite_result_free(ptr)

```c
void knowledge_rewrite_result_free(struct CKnowledgeRewriteResult *ptr);
```

Frees a knowledge-rewrite result handle; NULL is allowed.

### knowledge_rewrite_result_circuit(ptr)

```c
struct CCircuit *knowledge_rewrite_result_circuit(const struct CKnowledgeRewriteResult *ptr);
```

Retrieves the rewritten circuit (a deep clone, independent of `ptr`).

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL on NULL input or failure.

### knowledge_rewrite_result_changed(ptr)

```c
int32_t knowledge_rewrite_result_changed(const struct CKnowledgeRewriteResult *ptr);
```

Tests whether rewriting changed the circuit representation.

Returns: `1` when changed; `0` otherwise; `-1` for NULL.

### knowledge_rewrite_result_stats(ptr, out)

```c
int32_t knowledge_rewrite_result_stats(const struct CKnowledgeRewriteResult *ptr,
                                       struct CKnowledgeRewriteStats *out);
```

Writes the rewrite run-statistics snapshot to `*out`. The `CKnowledgeRewriteStats` layout:

| Field | Type | Meaning |
| --- | --- | --- |
| `rounds_executed` | `uint8_t` | Fixpoint rounds actually executed. |
| `reached_fixpoint` | `uint8_t` | 1 when a stable round was observed before `max_rounds`. |
| `rules_applied` | `uintptr_t` | Rule patches emitted into rebuilt sequences. |
| `changed_sequences` | `uintptr_t` | Operation sequences whose patch set was non-empty. |

- `ptr` (`const CKnowledgeRewriteResult*`): rewrite result handle.
- `out` (`CKnowledgeRewriteStats*`): statistics snapshot output.

Returns: `0` on success; `-1` when either argument is NULL.

### knowledge_rewrite_result_diagnostics(ptr, out)

```c
int32_t knowledge_rewrite_result_diagnostics(const struct CKnowledgeRewriteResult *ptr,
                                             struct CKnowledgeRewriteDiagnostics *out);
```

Writes the rewrite performance-diagnostics snapshot to `*out`. The `CKnowledgeRewriteDiagnostics` layout:

| Field | Type | Meaning |
| --- | --- | --- |
| `direct_reuses` | `uintptr_t` | Rewrite phases skipped by a reusable session fixpoint proof. |
| `dirty_anchors` | `uintptr_t` | Anchors presented by verified dirty ranges before the density fallback. |
| `full_scan_fallbacks` | `uintptr_t` | Incremental executions conservatively promoted to full scans. |
| `condition_cache_hits` | `uintptr_t` | Cached rule-condition results reused. |
| `condition_cache_misses` | `uintptr_t` | Cacheable rule-condition bindings not found in the local cache. |
| `symbolic_fallbacks` | `uintptr_t` | Conditions evaluated by the conservative symbolic path. |

- `ptr` (`const CKnowledgeRewriteResult*`): rewrite result handle.
- `out` (`CKnowledgeRewriteDiagnostics*`): diagnostics snapshot output.

Returns: `0` on success; `-1` when either argument is NULL.

---

## Standalone Pass Wrappers

The `transform_*` family wraps a single transform pass: it returns the rebuilt circuit directly and skips the intermediate result handle. None of them modifies the input circuit; free outputs with `circuit_free`.

### transform_canonicalize(circuit)

```c
struct CCircuit *transform_canonicalize(const struct CCircuit *circuit);
```

Canonicalizes the circuit with production defaults and returns the rebuilt circuit (equivalent to running `canonicalize_circuit` and then retrieving the circuit).

Returns: a new `CCircuit*` on success; NULL on NULL input or execution failure.

### transform_knowledge_rewrite(circuit, config)

```c
struct CCircuit *transform_knowledge_rewrite(const struct CCircuit *circuit,
                                             struct CRewriteConfig config);
```

Applies the knowledge rewriter with the supplied configuration and returns the rebuilt circuit.

- `circuit` (`const CCircuit*`): input circuit.
- `config` (`struct CRewriteConfig`): rewrite configuration, passed by value.

Returns: a new `CCircuit*` on success; NULL on NULL input, an invalid `config.mode` or target-instruction basis, or execution failure.

### transform_optimize_one_qubit_runs(circuit)

```c
struct CCircuit *transform_optimize_one_qubit_runs(const struct CCircuit *circuit);
```

Fuses fixed numeric one-qubit runs through exact synthesis (target-neutral logical cost); the output never has more operations than the input.

Returns: a new `CCircuit*` on success; NULL on NULL input or execution failure.

### transform_commutative_cancellation(circuit)

```c
struct CCircuit *transform_commutative_cancellation(const struct CCircuit *circuit);
```

Cancels self-inverse gate pairs (such as adjacent `H·H`, `CX·CX`) through global commutation analysis.

Returns: a new `CCircuit*` on success; NULL on NULL input or execution failure.

### transform_lower_to_routing_basis(circuit)

```c
struct CCircuit *transform_lower_to_routing_basis(const struct CCircuit *circuit);
```

Lowers the circuit to the routing basis: gates exceeding the SABRE gate-arity constraints (such as `CCX`) are expanded into operations of at most two qubits, preparing the circuit for routing.

Returns: a new `CCircuit*` on success; NULL on NULL input or execution failure.

---

## Example

Structural analysis, canonicalization, and knowledge rewriting:

```c
#include <stdio.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Structural analysis of a Bell-pair circuit */
    struct CCircuit *bell = circuit_new(2);
    circuit_h(bell, 0);
    circuit_cx(bell, 0, 1);

    struct CCircuitAnalysis analysis;
    memset(&analysis, 0, sizeof(analysis));
    if (circuit_analyze(bell, &analysis) != 0) {
        circuit_free(bell);
        return 1;
    }
    /* analysis.has_classical_control == 0, analysis.has_unitary_gates == 0 */

    /* 2. Canonicalize and read the result handle */
    struct CCanonicalizeResult *canon = canonicalize_circuit(bell);
    if (canon == NULL) {
        circuit_free(bell);
        return 1;
    }
    uint8_t rounds = canonicalize_result_rounds(canon);   /* >= 1 */
    struct CCircuit *canonical = canonicalize_result_circuit(canon);
    if (canonical != NULL) {
        circuit_free(canonical);  /* deep clone, independent of canon */
    }
    canonicalize_result_free(canon);

    /* 3. Knowledge rewrite with the production configuration */
    struct CRewriteConfig config = rewrite_config_default();
    struct CKnowledgeRewriteResult *rewritten = rewrite_circuit(bell, config);
    if (rewritten != NULL) {
        int32_t changed = knowledge_rewrite_result_changed(rewritten);
        struct CKnowledgeRewriteStats stats;
        struct CKnowledgeRewriteDiagnostics diagnostics;
        memset(&stats, 0, sizeof(stats));
        memset(&diagnostics, 0, sizeof(diagnostics));
        if (knowledge_rewrite_result_stats(rewritten, &stats) == 0) {
            printf("rewrite: rounds=%u rules=%llu\n",
                   stats.rounds_executed, (unsigned long long)stats.rules_applied);
        }
        knowledge_rewrite_result_diagnostics(rewritten, &diagnostics);
        knowledge_rewrite_result_free(rewritten);
    }
    circuit_free(bell);

    /* 4. Standalone pass wrappers return rebuilt circuits directly */
    struct CCircuit *runs = circuit_new(1);
    circuit_h(runs, 0);
    circuit_t(runs, 0);
    circuit_tdg(runs, 0);
    struct CCircuit *fused = transform_optimize_one_qubit_runs(runs);
    if (fused != NULL) {
        circuit_free(fused);
    }

    struct CCircuit *lowered = transform_lower_to_routing_basis(runs);
    if (lowered != NULL) {
        circuit_free(lowered);
    }
    circuit_free(runs);
    return 0;
}
```

Configured canonicalization and the transform configuration handle:

```c
#include <stdio.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build a canonicalize config and tighten the round limit */
    struct CCanonicalizeConfig *config = canonicalize_config_default();
    if (config == NULL) {
        return 1;
    }
    /* canonicalize_config_round_limit(config) == 8, all optional behaviors on */
    canonicalize_config_with_round_limit(config, 2);

    struct CCircuit *circuit = circuit_new(2);
    circuit_h(circuit, 0);
    circuit_cx(circuit, 0, 1);
    circuit_t(circuit, 1);

    /* 2. Canonicalize under the config; the config stays owned by the caller */
    struct CCanonicalizeResult *result = canonicalize_circuit_with_config(circuit, config);
    if (result != NULL) {
        struct CCircuit *canonical = canonicalize_result_circuit(result);
        /* canonicalize_result_rounds(result) <= 2 */
        circuit_free(canonical);
        canonicalize_result_free(result);
    }

    /* 3. Reuse the same config with a tighter limit */
    canonicalize_config_with_round_limit(config, 1);
    struct CCanonicalizeResult *limited = canonicalize_circuit_with_config(circuit, config);
    /* canonicalize_result_rounds(limited) <= 1 */
    canonicalize_result_free(limited);
    canonicalize_config_free(config);
    circuit_free(circuit);

    /* 4. Transform config: inspect defaults, then restrict in place */
    struct CTransformConfig *tconfig = transform_config_default();
    if (tconfig == NULL) {
        return 1;
    }
    /* transform_config_mode(tconfig) == REWRITE_MODE_OPTIMIZE,
       transform_config_enabled_kinds_len(tconfig) == 4 */
    const uint8_t kinds[2] = {REWRITE_KIND_CANCEL, REWRITE_KIND_DECOMPOSE};
    transform_config_with_enabled_kinds(tconfig, kinds, 2);

    const char *const gates[2] = {"H", "CX"};
    if (transform_config_with_target_instructions(tconfig, gates, 2) == 0) {
        char *basis[2] = {NULL, NULL};
        uintptr_t total = transform_config_target_instruction_basis(tconfig, basis, 2);
        for (uintptr_t i = 0; i < total && i < 2; i++) {
            cqlib_string_free(basis[i]);  /* each entry: newly allocated */
        }
    }
    transform_config_with_mode(tconfig, 99);  /* -8, config unchanged */
    transform_config_free(tconfig);
    return 0;
}
```

The fused one-qubit run never grows after `transform_optimize_one_qubit_runs`; the compile entry point that orchestrates these passes end-to-end is documented in [Compiler](1_compiler.md), and layout/routing in [Initial Layout](3_layout.md) and [Routing](4_routing.md).
