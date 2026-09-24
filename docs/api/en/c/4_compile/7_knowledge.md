# Knowledge (C)

`CKnowledgeLibrary` is an opaque handle to a validated compiler rule library: a set of knowledge rules registered by kind and indexed by name. `CKnowledgeRule` is a standalone handle to a single rule that can be parsed, validated, and inserted into any library. A knowledge rule describes an adjacent operation pattern (match), optional symbolic parameter conditions, and a replacement sequence (rewrite); rules are the source for knowledge rewriting and gate decomposition. A match produces a `CKnowledgeBindings*` that records the bound qubits and parameters; the rewrite target instantiated under those bindings is returned as a `CKnowledgeReplacements*`. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## Constants

Rule-kind tags (the `kind` parameter and the return value of `knowledge_library_rule_kind`):

| Constant | Value | Kind |
| --- | --- | --- |
| `RULE_KIND_SIMPLIFY` | 0 | Simplification rules. |
| `RULE_KIND_CANCEL` | 1 | Cancellation rules (adjacent inverse operations removed). |
| `RULE_KIND_MERGE` | 2 | Merge rules. |
| `RULE_KIND_COMMUTE` | 3 | Explicit commutation rules (`A; B -> B; A`). |
| `RULE_KIND_DECOMPOSE` | 4 | Decomposition or lowering rules. |
| `RULE_KIND_CANONICALIZE` | 5 | Canonicalization rules. |
| `RULE_KIND_HARDWARE_NATIVE` | 6 | Hardware-native rules. |
| `RULE_KIND_OTHER` | 7 | Other rules. |

---

## DSL Rules

`knowledge_library_from_dsl_str` / `knowledge_library_from_dsl_file` / `knowledge_rule_from_dsl` accept `.rule` DSL text:

```text
rule merge_rz {
    match {
        RZ(a) 0
        RZ(b) 0
    }
    rewrite {
        RZ(a + b) 0
    }
}
```

- `rule <name>`: the rule name, unique within a library.
- `match { ... }`: the adjacent operation pattern to match, at least one item.
- `rewrite { ... }`: the replacement sequence; may be empty (an empty rewrite cancels and deletes).
- Each operation is `gate_name (params) qubit_labels...`; the parentheses are omitted for parameterless gates (for example `H 0`).
- Qubit labels are non-negative integer placeholders that must start at zero and be contiguous; matching binds them to concrete qubits.
- Parameters may be numeric values, symbols (`a`, `b`), or expressions (`a + b`).

---

## Library Construction and Release

### knowledge_library_new()

```c
struct CKnowledgeLibrary *knowledge_library_new(void);
```

Creates an empty rule library.

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A new empty-library handle; free with `knowledge_library_free`. |

### knowledge_library_builtin()

```c
struct CKnowledgeLibrary *knowledge_library_builtin(void);
```

Creates a library containing the builtin compiler rules. The rules are registered under their own compiler kinds and can be queried per kind with `knowledge_library_rules_by_kind_len`.

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A new builtin-library handle; free with `knowledge_library_free`. |
| NULL | The builtin rule sources failed to validate. |

### knowledge_library_from_dsl_str(source, kind, out)

```c
int32_t knowledge_library_from_dsl_str(const char *source,
                                       uint8_t kind,
                                       struct CKnowledgeLibrary **out);
```

Parses and validates a DSL source string to build a rule library; `kind` (one of `RULE_KIND_*`) classifies every parsed rule.

Parameters:

- `source` (`const char*`): DSL source text.
- `kind` (`uint8_t`): rule-kind tag, one of `RULE_KIND_*`.
- `out` (`struct CKnowledgeLibrary**`): output pointer receiving the new library on success.

Returns: 0 on success with a new `CKnowledgeLibrary*` (free with `knowledge_library_free`) written to `*out`.

Error codes:

| Value | Scenario |
| --- | --- |
| 0 | Success. |
| -1 | `source` or `out` is NULL. |
| -4 | `source` is invalid UTF-8, or DSL parsing, lowering, or validation failed. |
| -8 | `kind` is not a valid `RULE_KIND_*` tag. |

### knowledge_library_from_dsl_file(path, kind, out)

```c
int32_t knowledge_library_from_dsl_file(const char *path,
                                        uint8_t kind,
                                        struct CKnowledgeLibrary **out);
```

Loads, parses, and validates rules from a DSL file to build a rule library; `kind` classifies every rule.

Parameters:

- `path` (`const char*`): path to the DSL file.
- `kind` (`uint8_t`): rule-kind tag, one of `RULE_KIND_*`.
- `out` (`struct CKnowledgeLibrary**`): output pointer receiving the new library on success.

Returns: 0 on success with a new `CKnowledgeLibrary*` written to `*out`.

Error codes:

| Value | Scenario |
| --- | --- |
| 0 | Success. |
| -1 | `path` or `out` is NULL. |
| -4 | `path` is invalid UTF-8, or DSL parsing, lowering, or validation failed. |
| -5 | The file could not be read. |
| -8 | `kind` is not a valid `RULE_KIND_*` tag. |

### knowledge_library_free(ptr)

```c
void knowledge_library_free(struct CKnowledgeLibrary *ptr);
```

Frees a rule library handle; NULL is allowed.

---

## Library Queries

### knowledge_library_len(ptr)

```c
uintptr_t knowledge_library_len(const struct CKnowledgeLibrary *ptr);
```

Returns the number of rules in the library.

Return value:

| Return | Scenario |
| --- | --- |
| Non-negative | The rule count. |
| 0 | `ptr` is NULL or the library is empty. |

### knowledge_library_is_empty(ptr)

```c
int32_t knowledge_library_is_empty(const struct CKnowledgeLibrary *ptr);
```

Tests whether the library contains no rules.

Return value:

| Value | Scenario |
| --- | --- |
| 1 | The library is empty. |
| 0 | The library is not empty. |
| -1 | `ptr` is NULL. |

### knowledge_library_contains(ptr, name)

```c
int32_t knowledge_library_contains(const struct CKnowledgeLibrary *ptr, const char *name);
```

Tests whether a rule named `name` exists in the library.

Parameters:

- `ptr` (`const CKnowledgeLibrary*`): rule library handle.
- `name` (`const char*`): rule name.

Return value:

| Value | Scenario |
| --- | --- |
| 1 | A rule with that name exists. |
| 0 | No rule with that name exists. |
| -1 | `ptr` or `name` is NULL. |
| -4 | `name` is invalid UTF-8. |

### knowledge_library_id_by_name(ptr, name)

```c
int64_t knowledge_library_id_by_name(const struct CKnowledgeLibrary *ptr, const char *name);
```

Looks up the rule id of `name` (the insertion index; a rule's id equals the library length at insertion time).

Parameters:

- `ptr` (`const CKnowledgeLibrary*`): rule library handle.
- `name` (`const char*`): rule name.

Return value:

| Return | Scenario |
| --- | --- |
| Non-negative | The rule id. |
| -1 | `ptr` or `name` is NULL, or no rule with that name exists. |
| -4 | `name` is invalid UTF-8. |

### knowledge_library_rules_by_kind_len(ptr, kind) / knowledge_library_rules_by_kind(ptr, kind, out, len)

```c
uintptr_t knowledge_library_rules_by_kind_len(const struct CKnowledgeLibrary *ptr, uint8_t kind);
uintptr_t knowledge_library_rules_by_kind(const struct CKnowledgeLibrary *ptr,
                                          uint8_t kind,
                                          uint32_t *out,
                                          uintptr_t len);
```

Two-step read of the rule ids registered under a kind: `*_len` returns the total number of rules of that kind; the fill function copies the ids into `out` (writing `min(total, len)` entries) and returns the total. When `out` is NULL it only returns the total without copying.

Parameters:

- `ptr` (`const CKnowledgeLibrary*`): rule library handle.
- `kind` (`uint8_t`): rule-kind tag, one of `RULE_KIND_*`.
- `out` (`uint32_t*`): buffer receiving the ids.
- `len` (`uintptr_t`): buffer length.

Return value (both functions):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of rules of that kind. |
| 0 | `ptr` is NULL, or `kind` is not a valid `RULE_KIND_*` tag. |

### knowledge_rules_len(ptr) / knowledge_rules(ptr, out, len)

```c
uintptr_t knowledge_rules_len(const struct CKnowledgeLibrary *ptr);
uintptr_t knowledge_rules(const struct CKnowledgeLibrary *ptr, uint32_t *out, uintptr_t len);
```

Two-step read of the rule ids (insertion indices) in the library: `*_len` returns the total number of rules; the fill function copies the ids into `out` (writing `min(total, len)` entries) and returns the total. When `out` is NULL it only returns the total without copying.

Parameters:

- `ptr` (`const CKnowledgeLibrary*`): rule library handle.
- `out` (`uint32_t*`): buffer receiving the ids.
- `len` (`uintptr_t`): buffer length.

Return value (both functions):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of rules. |
| 0 | `ptr` is NULL. |

### knowledge_get_by_name(ptr, name)

```c
struct CKnowledgeRule *knowledge_get_by_name(const struct CKnowledgeLibrary *ptr, const char *name);
```

Looks up a rule by name and returns a cloned handle.

Parameters:

- `ptr` (`const CKnowledgeLibrary*`): rule library handle.
- `name` (`const char*`): rule name.

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A new `CKnowledgeRule*` clone; free with `knowledge_rule_free`. |
| NULL | `ptr` or `name` is NULL, or no rule with that name exists. |

---

## Rule Metadata

The following functions read the precomputed metadata of the rule at insertion index `index`; valid indices span `[0, knowledge_library_len(ptr))`.

### knowledge_library_rule_name(ptr, index)

```c
char *knowledge_library_rule_name(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

Returns the name of the rule at `index`.

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A heap-allocated C string; free with `cqlib_string_free`. |
| NULL | `ptr` is NULL or `index` is out of bounds. |

### knowledge_library_rule_kind(ptr, index)

```c
int32_t knowledge_library_rule_kind(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

Returns the kind tag of the rule at `index`.

Return value:

| Value | Scenario |
| --- | --- |
| 0–7 | A `RULE_KIND_*` kind tag. |
| -1 | `ptr` is NULL. |
| -8 | `index` is out of bounds. |

### knowledge_library_rule_first_instruction(ptr, index)

```c
char *knowledge_library_rule_first_instruction(const struct CKnowledgeLibrary *ptr,
                                               uintptr_t index);
```

Returns the name of the instruction starting the rule's match pattern (candidate rules can be filtered by their first instruction).

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A heap-allocated C string; free with `cqlib_string_free`. |
| NULL | `ptr` is NULL or `index` is out of bounds. |

### knowledge_library_rule_pattern_len(ptr, index)

```c
uintptr_t knowledge_library_rule_pattern_len(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

Returns the number of operations in the rule's match pattern.

Return value:

| Return | Scenario |
| --- | --- |
| Non-negative | The pattern length. |
| 0 | `ptr` is NULL or `index` is out of bounds. |

### knowledge_library_rule_rewrite_len(ptr, index)

```c
uintptr_t knowledge_library_rule_rewrite_len(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

Returns the number of operations emitted by the rule's rewrite target.

Return value:

| Return | Scenario |
| --- | --- |
| Non-negative | The rewrite length. |
| 0 | `ptr` is NULL or `index` is out of bounds. |

### knowledge_library_rule_qubit_count(ptr, index)

```c
uintptr_t knowledge_library_rule_qubit_count(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

Returns the number of distinct rule-local qubit labels used by the rule.

Return value:

| Return | Scenario |
| --- | --- |
| Non-negative | The qubit-label count. |
| 0 | `ptr` is NULL or `index` is out of bounds. |

### knowledge_library_rule_cost_delta(ptr, index)

```c
int64_t knowledge_library_rule_cost_delta(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

Returns the rule's static operation-count delta `rewrite_len - pattern_len` (may be negative when the rule removes operations).

Return value:

| Return | Scenario |
| --- | --- |
| Integer | `rewrite_len - pattern_len`. |
| 0 | `ptr` is NULL or `index` is out of bounds. |

### knowledge_library_rule_has_conditions(ptr, index)

```c
int32_t knowledge_library_rule_has_conditions(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

Tests whether the rule carries non-empty symbolic parameter conditions.

Return value:

| Value | Scenario |
| --- | --- |
| 1 | The rule carries non-empty conditions. |
| 0 | The rule has no conditions. |
| -1 | `ptr` is NULL. |
| -8 | `index` is out of bounds. |

---

## Single-Rule Parsing and Insertion

### knowledge_rule_from_dsl(source, out)

```c
int32_t knowledge_rule_from_dsl(const char *source, struct CKnowledgeRule **out);
```

Parses a DSL source string that must define exactly one rule and creates a single-rule handle (to be inserted with `knowledge_library_add_rule`).

Parameters:

- `source` (`const char*`): DSL source text (exactly one `rule` definition).
- `out` (`struct CKnowledgeRule**`): output pointer receiving the new handle on success.

Returns: 0 on success with a new `CKnowledgeRule*` (free with `knowledge_rule_free`) written to `*out`.

Error codes:

| Value | Scenario |
| --- | --- |
| 0 | Success. |
| -1 | `source` or `out` is NULL. |
| -4 | `source` is invalid UTF-8, or DSL parsing, lowering, or validation failed. |
| -8 | The source does not define exactly one rule. |

### knowledge_rule_free(ptr)

```c
void knowledge_rule_free(struct CKnowledgeRule *ptr);
```

Frees a single-rule handle; NULL is allowed.

### knowledge_library_add_rule(ptr, rule, kind)

```c
int64_t knowledge_library_add_rule(struct CKnowledgeLibrary *ptr,
                                   const struct CKnowledgeRule *rule,
                                   uint8_t kind);
```

Inserts a parsed rule into the library under kind `kind` and returns its assigned rule id. Structural validation runs before insertion; rule names must be unique within the library.

Parameters:

- `ptr` (`CKnowledgeLibrary*`): target rule library.
- `rule` (`const CKnowledgeRule*`): the rule to insert (cloned; the handle is not consumed and remains owned by the caller).
- `kind` (`uint8_t`): rule-kind tag, one of `RULE_KIND_*`.

Return value:

| Return | Scenario |
| --- | --- |
| Non-negative | The assigned rule id. |
| -1 | `ptr` or `rule` is NULL. |
| -4 | Structural validation failed or the rule name duplicates an existing one. |
| -6 | The rule's first instruction cannot be indexed. |
| -8 | `kind` is not a valid `RULE_KIND_*` tag. |

---

## Single-Rule Accessors

### knowledge_rule_name(ptr)

```c
char *knowledge_rule_name(const struct CKnowledgeRule *ptr);
```

Returns the rule's name.

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A heap-allocated C string; free with `cqlib_string_free`. |
| NULL | `ptr` is NULL. |

### knowledge_rule_num_qubits(ptr)

```c
uintptr_t knowledge_rule_num_qubits(const struct CKnowledgeRule *ptr);
```

Returns the number of qubit labels the rule involves.

Return value:

| Return | Scenario |
| --- | --- |
| Non-negative | The qubit-label count. |
| 0 | `ptr` is NULL. |

### knowledge_rule_pattern_len(ptr)

```c
uintptr_t knowledge_rule_pattern_len(const struct CKnowledgeRule *ptr);
```

Returns the number of operations in the rule's match pattern.

Return value:

| Return | Scenario |
| --- | --- |
| Non-negative | The pattern length. |
| 0 | `ptr` is NULL. |

### knowledge_rule_rewrite_len(ptr)

```c
uintptr_t knowledge_rule_rewrite_len(const struct CKnowledgeRule *ptr);
```

Returns the number of operations emitted by the rule's rewrite target.

Return value:

| Return | Scenario |
| --- | --- |
| Non-negative | The rewrite length. |
| 0 | `ptr` is NULL. |

### knowledge_rule_operation_symbols_len(ptr) / knowledge_rule_operation_symbols(ptr, out, len)

```c
uintptr_t knowledge_rule_operation_symbols_len(const struct CKnowledgeRule *ptr);
int32_t knowledge_rule_operation_symbols(const struct CKnowledgeRule *ptr,
                                         char **out,
                                         uintptr_t len);
```

Two-step read of the distinct symbols bound by the rule's match block: `*_len` returns the total number of symbols; the fill function copies the symbols (sorted, deduplicated) into `out` as newly allocated C strings and returns 0. Each string must be released with `cqlib_string_free`.

Parameters:

- `ptr` (`const CKnowledgeRule*`): rule handle.
- `out` (`char**`): buffer receiving the symbol strings.
- `len` (`uintptr_t`): buffer length, which must equal the count returned by `knowledge_rule_operation_symbols_len`.

Return value (`*_len`):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of symbols. |
| 0 | `ptr` is NULL. |

Error codes (fill function):

| Value | Scenario |
| --- | --- |
| 0 | Success; `out` holds `len` newly allocated strings. |
| -1 | `ptr` is NULL, or `out` is NULL while `len > 0`. |
| -8 | `len` does not match the symbol count. |

### knowledge_rule_target_symbols_len(ptr) / knowledge_rule_target_symbols(ptr, out, len)

```c
uintptr_t knowledge_rule_target_symbols_len(const struct CKnowledgeRule *ptr);
int32_t knowledge_rule_target_symbols(const struct CKnowledgeRule *ptr,
                                      char **out,
                                      uintptr_t len);
```

Two-step read of the distinct symbols referenced by the rule's rewrite target: `*_len` returns the total number of symbols; the fill function copies the symbols (sorted, deduplicated) into `out` as newly allocated C strings and returns 0. Each string must be released with `cqlib_string_free`.

Parameters:

- `ptr` (`const CKnowledgeRule*`): rule handle.
- `out` (`char**`): buffer receiving the symbol strings.
- `len` (`uintptr_t`): buffer length, which must equal the count returned by `knowledge_rule_target_symbols_len`.

Return value (`*_len`):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of symbols. |
| 0 | `ptr` is NULL. |

Error codes (fill function):

| Value | Scenario |
| --- | --- |
| 0 | Success; `out` holds `len` newly allocated strings. |
| -1 | `ptr` is NULL, or `out` is NULL while `len > 0`. |
| -8 | `len` does not match the symbol count. |

### knowledge_rule_condition_symbols_len(ptr) / knowledge_rule_condition_symbols(ptr, out, len)

```c
uintptr_t knowledge_rule_condition_symbols_len(const struct CKnowledgeRule *ptr);
int32_t knowledge_rule_condition_symbols(const struct CKnowledgeRule *ptr,
                                         char **out,
                                         uintptr_t len);
```

Two-step read of the distinct symbols referenced by the rule's parameter conditions: `*_len` returns the total number of symbols; the fill function copies the symbols (sorted, deduplicated) into `out` as newly allocated C strings and returns 0. Each string must be released with `cqlib_string_free`.

Parameters:

- `ptr` (`const CKnowledgeRule*`): rule handle.
- `out` (`char**`): buffer receiving the symbol strings.
- `len` (`uintptr_t`): buffer length, which must equal the count returned by `knowledge_rule_condition_symbols_len`.

Return value (`*_len`):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of symbols. |
| 0 | `ptr` is NULL. |

Error codes (fill function):

| Value | Scenario |
| --- | --- |
| 0 | Success; `out` holds `len` newly allocated strings. |
| -1 | `ptr` is NULL, or `out` is NULL while `len > 0`. |
| -8 | `len` does not match the symbol count. |

### knowledge_rule_has_conditions(ptr)

```c
int32_t knowledge_rule_has_conditions(const struct CKnowledgeRule *ptr);
```

Tests whether the rule carries non-empty symbolic parameter conditions.

Return value:

| Value | Scenario |
| --- | --- |
| 1 | The rule carries non-empty conditions. |
| 0 | The rule has no conditions. |
| -1 | `ptr` is NULL. |

### knowledge_rule_validate(ptr)

```c
int32_t knowledge_rule_validate(const struct CKnowledgeRule *ptr);
```

Runs the rule's structural validation.

Error codes:

| Value | Scenario |
| --- | --- |
| 0 | The rule is well formed. |
| -1 | `ptr` is NULL. |
| -4 | Validation failed. |

---

## Matching Engine

The matching engine drives a rule manually: match the pattern against concrete operations to obtain `CKnowledgeBindings*`, evaluate conditions on the bindings, and instantiate the rewrite target into `CKnowledgeReplacements*`. Operations are passed as `COperation*` handles created with `operation_new` (see [Operation / Instruction](../0_circuit/4_operation_instruction.md)).

### knowledge_bindings_new()

```c
struct CKnowledgeBindings *knowledge_bindings_new(void);
```

Creates an empty match-bindings record for use with `knowledge_match_rule_item`.

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A new bindings handle; free with `knowledge_bindings_free`. |

### knowledge_bindings_free(ptr)

```c
void knowledge_bindings_free(struct CKnowledgeBindings *ptr);
```

Frees match bindings; NULL is allowed.

### knowledge_match_rule_item(ptr, item_index, op, bindings)

```c
int32_t knowledge_match_rule_item(const struct CKnowledgeRule *ptr,
                                  uintptr_t item_index,
                                  const struct COperation *op,
                                  struct CKnowledgeBindings *bindings);
```

Matches the rule's match-pattern item at `item_index` against a concrete operation and records the bound qubits and parameters into `bindings`. The match is transactional: when the item does not match, the bindings are left unchanged, so a failed attempt does not corrupt earlier bindings.

Parameters:

- `ptr` (`const CKnowledgeRule*`): rule handle.
- `item_index` (`uintptr_t`): index into the rule's match pattern.
- `op` (`const COperation*`): the operation to match.
- `bindings` (`CKnowledgeBindings*`): bindings record updated on success.

Error codes:

| Value | Scenario |
| --- | --- |
| 1 | The item matches; `bindings` now records the bound qubits and parameters. |
| 0 | The item does not match; `bindings` unchanged. |
| -1 | `ptr`, `op`, or `bindings` is NULL. |
| -2 | `item_index` is out of bounds. |
| -8 | The operation carries indexed (table-resolved) parameters. |
| -6 | The rule item uses an instruction the matcher cannot represent. |

### knowledge_rule_matches_operations(ptr, operations, operations_len, out)

```c
int32_t knowledge_rule_matches_operations(const struct CKnowledgeRule *ptr,
                                          const struct COperation *const *operations,
                                          uintptr_t operations_len,
                                          struct CKnowledgeBindings **out);
```

Matches the rule's full match pattern against `operations_len` adjacent operations. On success writes a new `CKnowledgeBindings*` to `*out` and returns 1. Parameter conditions are not evaluated here; check them separately with `knowledge_conditions_hold` on the returned bindings.

Parameters:

- `ptr` (`const CKnowledgeRule*`): rule handle.
- `operations` (`const COperation* const*`): array of adjacent operations.
- `operations_len` (`uintptr_t`): number of operations.
- `out` (`struct CKnowledgeBindings**`): output pointer receiving the bindings on success.

Error codes:

| Value | Scenario |
| --- | --- |
| 1 | The pattern matches; a new `CKnowledgeBindings*` (free with `knowledge_bindings_free`) was written to `*out`. |
| 0 | The pattern does not match; `out` left untouched. |
| -1 | `ptr`, `operations`, or `out` is NULL. |
| -8 | `operations_len` differs from the pattern length, or an operation carries indexed parameters. |

### knowledge_conditions_hold(ptr, bindings)

```c
int32_t knowledge_conditions_hold(const struct CKnowledgeRule *ptr,
                                  const struct CKnowledgeBindings *bindings);
```

Evaluates the rule's symbolic parameter conditions under the bindings.

Return value:

| Value | Scenario |
| --- | --- |
| 1 | All parameter conditions hold. |
| 0 | At least one condition fails. |
| -1 | `ptr` or `bindings` is NULL. |

### knowledge_equivalent_to(lhs, lhs_item, rhs, rhs_item)

```c
int32_t knowledge_equivalent_to(const struct CKnowledgeRule *lhs,
                                uintptr_t lhs_item,
                                const struct CKnowledgeRule *rhs,
                                uintptr_t rhs_item);
```

Tests whether the match-pattern item at `lhs_item` of `lhs` is equivalent to the item at `rhs_item` of `rhs`.

Parameters:

- `lhs` (`const CKnowledgeRule*`): first rule handle.
- `lhs_item` (`uintptr_t`): item index in the first rule's pattern.
- `rhs` (`const CKnowledgeRule*`): second rule handle.
- `rhs_item` (`uintptr_t`): item index in the second rule's pattern.

Return value:

| Value | Scenario |
| --- | --- |
| 1 | The two items are equivalent. |
| 0 | The items are not equivalent. |
| -1 | `lhs` or `rhs` is NULL. |
| -2 | An item index is out of bounds. |

### knowledge_candidates_for_first_instruction_len(ptr, name) / knowledge_candidates_for_first_instruction(ptr, name, out, len)

```c
uintptr_t knowledge_candidates_for_first_instruction_len(const struct CKnowledgeLibrary *ptr,
                                                         const char *name);
uintptr_t knowledge_candidates_for_first_instruction(const struct CKnowledgeLibrary *ptr,
                                                     const char *name,
                                                     uint32_t *out,
                                                     uintptr_t len);
```

Two-step read of the rule ids whose first match instruction has the same matcher key as the gate `name`: `*_len` returns the total number of candidates; the fill function copies the ids into `out` (writing `min(total, len)` entries) and returns the total. When `out` is NULL it only returns the total without copying.

Parameters:

- `ptr` (`const CKnowledgeLibrary*`): rule library handle.
- `name` (`const char*`): gate name.
- `out` (`uint32_t*`): buffer receiving the ids.
- `len` (`uintptr_t`): buffer length.

Return value (both functions):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of candidate rules. |
| 0 | `ptr` or `name` is NULL, or `name` is an unknown gate name. |

### knowledge_collect_free_symbols_len(ptr) / knowledge_collect_free_symbols(ptr, out, len)

```c
uintptr_t knowledge_collect_free_symbols_len(const struct CKnowledgeRule *ptr);
uintptr_t knowledge_collect_free_symbols(const struct CKnowledgeRule *ptr,
                                         char **out,
                                         uintptr_t len);
```

Two-step read of the rule's free symbols (sorted): `*_len` returns the total number of symbols; the fill function copies the symbols into `out` as newly allocated C strings (writing `min(total, len)` entries) and returns the total. Each string must be released with `cqlib_string_free`.

Parameters:

- `ptr` (`const CKnowledgeRule*`): rule handle.
- `out` (`char**`): buffer receiving the symbol strings.
- `len` (`uintptr_t`): buffer length.

Return value (both functions):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of free symbols. |
| 0 | `ptr` is NULL. |

### knowledge_filter_rule_ids_by_instruction_keys_len(ptr, ops, ops_len, targets, targets_len) / knowledge_filter_rule_ids_by_instruction_keys(ptr, ops, ops_len, targets, targets_len, out, len)

```c
uintptr_t knowledge_filter_rule_ids_by_instruction_keys_len(const struct CKnowledgeLibrary *ptr,
                                                            const char *const *ops,
                                                            uintptr_t ops_len,
                                                            const char *const *targets,
                                                            uintptr_t targets_len);
uintptr_t knowledge_filter_rule_ids_by_instruction_keys(const struct CKnowledgeLibrary *ptr,
                                                        const char *const *ops,
                                                        uintptr_t ops_len,
                                                        const char *const *targets,
                                                        uintptr_t targets_len,
                                                        uint32_t *out,
                                                        uintptr_t len);
```

Two-step read of the rule ids whose match pattern covers every gate in `ops` and whose rewrite target covers every gate in `targets` (both as arrays of gate names): `*_len` returns the total number of matching rules; the fill function copies the ids into `out` (writing `min(total, len)` entries) and returns the total. When `out` is NULL it only returns the total without copying.

Parameters:

- `ptr` (`const CKnowledgeLibrary*`): rule library handle.
- `ops` (`const char* const*`): match-side gate names.
- `ops_len` (`uintptr_t`): number of match-side names.
- `targets` (`const char* const*`): rewrite-side gate names.
- `targets_len` (`uintptr_t`): number of rewrite-side names.
- `out` (`uint32_t*`): buffer receiving the ids.
- `len` (`uintptr_t`): buffer length.

Return value (both functions):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of matching rules. |
| 0 | `ptr` is NULL, or a gate name is NULL or unknown. |

### knowledge_extend_rules(ptr, rules, rules_len, kind, out, out_len)

```c
int32_t knowledge_extend_rules(struct CKnowledgeLibrary *ptr,
                               const struct CKnowledgeRule *const *rules,
                               uintptr_t rules_len,
                               uint8_t kind,
                               uint32_t *out,
                               uintptr_t out_len);
```

Adds several parsed rules to the library under kind `kind` atomically and writes the assigned rule ids into `out` (which must have room for `rules_len` entries). If any rule fails, the library is left unchanged.

Parameters:

- `ptr` (`CKnowledgeLibrary*`): target rule library.
- `rules` (`const CKnowledgeRule* const*`): array of parsed rule handles (cloned; the handles are not consumed).
- `rules_len` (`uintptr_t`): number of rules.
- `kind` (`uint8_t`): rule-kind tag, one of `RULE_KIND_*`.
- `out` (`uint32_t*`): buffer receiving the assigned rule ids.
- `out_len` (`uintptr_t`): buffer length, at least `rules_len`.

Error codes:

| Value | Scenario |
| --- | --- |
| 0 | Success; `out` holds the assigned ids. |
| -1 | `ptr`, `rules`, or `out` is NULL. |
| -8 | `kind` is not a valid `RULE_KIND_*` tag, or `out_len` is smaller than `rules_len`. |
| -4 | A validation failure or duplicate rule name; the library is unchanged. |

### knowledge_target_qubits_len(ptr) / knowledge_target_qubits(ptr, out, len)

```c
uintptr_t knowledge_target_qubits_len(const struct CKnowledgeRule *ptr);
uintptr_t knowledge_target_qubits(const struct CKnowledgeRule *ptr, uint32_t *out, uintptr_t len);
```

Two-step read of the rule-local qubit labels (sorted) used by the rule's rewrite target: `*_len` returns the total number of labels; the fill function copies the labels into `out` (writing `min(total, len)` entries) and returns the total. When `out` is NULL it only returns the total without copying.

Parameters:

- `ptr` (`const CKnowledgeRule*`): rule handle.
- `out` (`uint32_t*`): buffer receiving the labels.
- `len` (`uintptr_t`): buffer length.

Return value (both functions):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of labels. |
| 0 | `ptr` is NULL. |

### knowledge_instantiate_target(ptr, bindings, out)

```c
int32_t knowledge_instantiate_target(const struct CKnowledgeRule *ptr,
                                     const struct CKnowledgeBindings *bindings,
                                     struct CKnowledgeReplacements **out);
```

Instantiates the rule's rewrite target under the match bindings, evaluating each target parameter expression with the bound values.

Parameters:

- `ptr` (`const CKnowledgeRule*`): rule handle.
- `bindings` (`const CKnowledgeBindings*`): bindings produced by a match.
- `out` (`struct CKnowledgeReplacements**`): output pointer receiving the instantiation result on success.

Error codes:

| Value | Scenario |
| --- | --- |
| 0 | Success; a new `CKnowledgeReplacements*` (free with `knowledge_replacements_free`) was written to `*out`. |
| -1 | `ptr`, `bindings`, or `out` is NULL. |
| -2 | The target references a qubit not bound by the match. |
| -8 | The target references a symbol not bound by the match. |
| -6 | The target uses an instruction the matcher cannot represent. |

### knowledge_replacements_len(ptr)

```c
uintptr_t knowledge_replacements_len(const struct CKnowledgeReplacements *ptr);
```

Returns the number of instantiated replacement items.

Return value:

| Return | Scenario |
| --- | --- |
| Non-negative | The replacement item count. |
| 0 | `ptr` is NULL. |

### knowledge_replacement_instruction(ptr, index)

```c
char *knowledge_replacement_instruction(const struct CKnowledgeReplacements *ptr, uintptr_t index);
```

Returns the instruction name (e.g. `"RZ"`) of the replacement item at `index`.

Parameters:

- `ptr` (`const CKnowledgeReplacements*`): instantiation result handle.
- `index` (`uintptr_t`): replacement item index.

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A heap-allocated C string; free with `cqlib_string_free`. |
| NULL | `ptr` is NULL or `index` is out of bounds. |

### knowledge_replacement_qubits_len(ptr, index) / knowledge_replacement_qubits(ptr, index, out, len)

```c
uintptr_t knowledge_replacement_qubits_len(const struct CKnowledgeReplacements *ptr,
                                           uintptr_t index);
uintptr_t knowledge_replacement_qubits(const struct CKnowledgeReplacements *ptr,
                                       uintptr_t index,
                                       uint32_t *out,
                                       uintptr_t len);
```

Two-step read of the concrete qubits of the replacement item at `index`: `*_len` returns the total number of qubits; the fill function copies the qubits into `out` (writing `min(total, len)` entries) and returns the total. When `out` is NULL it only returns the total without copying.

Parameters:

- `ptr` (`const CKnowledgeReplacements*`): instantiation result handle.
- `index` (`uintptr_t`): replacement item index.
- `out` (`uint32_t*`): buffer receiving the qubits.
- `len` (`uintptr_t`): buffer length.

Return value (both functions):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of qubits. |
| 0 | `ptr` is NULL or `index` is out of bounds. |

### knowledge_replacement_params_len(ptr, index) / knowledge_replacement_params(ptr, index, out, len)

```c
uintptr_t knowledge_replacement_params_len(const struct CKnowledgeReplacements *ptr,
                                           uintptr_t index);
uintptr_t knowledge_replacement_params(const struct CKnowledgeReplacements *ptr,
                                       uintptr_t index,
                                       double *out,
                                       uintptr_t len);
```

Two-step read of the evaluated double parameters of the replacement item at `index`: `*_len` returns the total number of parameters; the fill function copies the parameters into `out` (writing `min(total, len)` entries) and returns the total. When `out` is NULL it only returns the total without copying.

Parameters:

- `ptr` (`const CKnowledgeReplacements*`): instantiation result handle.
- `index` (`uintptr_t`): replacement item index.
- `out` (`double*`): buffer receiving the evaluated parameters.
- `len` (`uintptr_t`): buffer length.

Return value (both functions):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of parameters. |
| 0 | `ptr` is NULL or `index` is out of bounds. |

### knowledge_replacements_free(ptr)

```c
void knowledge_replacements_free(struct CKnowledgeReplacements *ptr);
```

Frees a rewrite-target instantiation result; NULL is allowed.

---

## Examples

Walking the builtin library and querying by kind:

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Load the builtin rule library. */
    struct CKnowledgeLibrary *lib = knowledge_library_builtin();
    if (lib == NULL) {
        return 1;
    }

    /* 2. Walk the rule metadata by insertion index. */
    for (uintptr_t i = 0; i < knowledge_library_len(lib); i++) {
        char *name = knowledge_library_rule_name(lib, i);
        int32_t kind = knowledge_library_rule_kind(lib, i);
        uintptr_t pattern = knowledge_library_rule_pattern_len(lib, i);
        if (name != NULL) {
            printf("rule %s: kind=%d pattern_len=%llu\n",
                   name, kind, (unsigned long long)pattern);
            cqlib_string_free(name);
        }
    }

    /* 3. Two-step read of the decompose-rule ids. */
    uintptr_t n = knowledge_library_rules_by_kind_len(lib, RULE_KIND_DECOMPOSE);
    uint32_t *ids = malloc(n * sizeof(uint32_t));
    if (ids != NULL && n > 0) {
        uintptr_t written = knowledge_library_rules_by_kind(lib, RULE_KIND_DECOMPOSE, ids, n);
        printf("%llu decompose rules (first id %u)\n",
               (unsigned long long)written, ids[0]);
        free(ids);
    }

    knowledge_library_free(lib);
    return 0;
}
```

Building a library from DSL text and querying it:

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build a library from DSL text (merge two adjacent RZ rotations). */
    const char *source =
        "rule merge_rz {\n"
        "    match {\n"
        "        RZ(a) 0\n"
        "        RZ(b) 0\n"
        "    }\n"
        "    rewrite {\n"
        "        RZ(a + b) 0\n"
        "    }\n"
        "}";
    struct CKnowledgeLibrary *lib = NULL;
    if (knowledge_library_from_dsl_str(source, RULE_KIND_MERGE, &lib) != 0) {
        return 1;
    }

    /* 2. Query by name. */
    if (knowledge_library_contains(lib, "merge_rz") == 1) {
        printf("id=%lld\n", (long long)knowledge_library_id_by_name(lib, "merge_rz"));
    }

    /* 3. Read the metadata of rule 0. */
    char *first = knowledge_library_rule_first_instruction(lib, 0);  /* "RZ" */
    printf("first=%s pattern=%llu rewrite=%llu cost_delta=%lld\n",
           first,
           (unsigned long long)knowledge_library_rule_pattern_len(lib, 0),
           (unsigned long long)knowledge_library_rule_rewrite_len(lib, 0),
           (long long)knowledge_library_rule_cost_delta(lib, 0));
    cqlib_string_free(first);

    knowledge_library_free(lib);
    return 0;
}
```

Parsing and inserting a single rule:

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Parse exactly one rule: cancel two adjacent H gates. */
    const char *source =
        "rule cancel_h {\n"
        "    match {\n"
        "        H 0\n"
        "        H 0\n"
        "    }\n"
        "    rewrite {}\n"
        "}";
    struct CKnowledgeRule *rule = NULL;
    if (knowledge_rule_from_dsl(source, &rule) != 0) {
        return 1;
    }

    /* 2. Inspect and validate the standalone rule. */
    if (knowledge_rule_validate(rule) != 0) {
        knowledge_rule_free(rule);
        return 1;
    }
    char *name = knowledge_rule_name(rule);  /* "cancel_h" */
    printf("rule %s: qubits=%llu pattern=%llu rewrite=%llu conditions=%d\n",
           name,
           (unsigned long long)knowledge_rule_num_qubits(rule),
           (unsigned long long)knowledge_rule_pattern_len(rule),
           (unsigned long long)knowledge_rule_rewrite_len(rule),
           knowledge_rule_has_conditions(rule));
    cqlib_string_free(name);

    /* 3. Insert into a fresh library (assigns id 0), then reject duplicates. */
    struct CKnowledgeLibrary *lib = knowledge_library_new();
    if (knowledge_library_add_rule(lib, rule, RULE_KIND_CANCEL) != 0) {
        knowledge_rule_free(rule);
        knowledge_library_free(lib);
        return 1;
    }
    if (knowledge_library_add_rule(lib, rule, RULE_KIND_CANCEL) == -4) {
        printf("duplicate name rejected\n");
    }

    knowledge_rule_free(rule);
    knowledge_library_free(lib);
    return 0;
}
```

Matching a rule against concrete operations and instantiating its rewrite target:

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build the merge_rz rule. */
    const char *source =
        "rule merge_rz {\n"
        "    match {\n"
        "        RZ(a) 0\n"
        "        RZ(b) 0\n"
        "    }\n"
        "    rewrite {\n"
        "        RZ(a + b) 0\n"
        "    }\n"
        "}";
    struct CKnowledgeRule *rule = NULL;
    if (knowledge_rule_from_dsl(source, &rule) != 0) {
        return 1;
    }

    /* 2. Build two adjacent RZ operations. */
    uint32_t q0[1] = {7};
    double p1[1] = {0.25};
    double p2[1] = {0.5};
    struct COperation *a = operation_new("RZ", q0, 1, p1, 1);
    struct COperation *b = operation_new("RZ", q0, 1, p2, 1);
    const struct COperation *ops[2] = {a, b};

    /* 3. Match the full pattern (conditions are checked separately). */
    struct CKnowledgeBindings *bindings = NULL;
    if (knowledge_rule_matches_operations(rule, ops, 2, &bindings) == 1) {
        /* 4. Instantiate the rewrite target: RZ(a + b) with a + b == 0.75. */
        struct CKnowledgeReplacements *repl = NULL;
        if (knowledge_instantiate_target(rule, bindings, &repl) == 0) {
            for (uintptr_t i = 0; i < knowledge_replacements_len(repl); i++) {
                char *name = knowledge_replacement_instruction(repl, i);  /* "RZ" */
                uint32_t qubits[1];
                double params[1];
                knowledge_replacement_qubits(repl, i, qubits, 1);       /* {7} */
                knowledge_replacement_params(repl, i, params, 1);       /* {0.75} */
                printf("repl %s q=%u p=%f\n", name, qubits[0], params[0]);
                cqlib_string_free(name);
            }
            knowledge_replacements_free(repl);
        }
        knowledge_bindings_free(bindings);
    }

    /* 5. Inspect rule internals: free symbols and target qubit labels. */
    uintptr_t nsym = knowledge_collect_free_symbols_len(rule);
    char **symbols = malloc(nsym * sizeof(char *));
    if (symbols != NULL && knowledge_collect_free_symbols(rule, symbols, nsym) == nsym) {
        for (uintptr_t i = 0; i < nsym; i++) {
            printf("symbol %s\n", symbols[i]);
            cqlib_string_free(symbols[i]);
        }
        free(symbols);
    }

    operation_free(a);
    operation_free(b);
    knowledge_rule_free(rule);
    return 0;
}
```

Checkers that draw their commutation rules from a rule library are covered in [Commutation](9_commutation.md).
