# Commutation (C)

Checks whether two concrete gate applications commute. Each gate application is described by a standard-gate name (for example `"H"`, `"RZ"`, `"CX"`), a qubit-id array, and a numeric parameter array; when the proof is up to a global phase, the phase is written to the `phase` out pointer. Proof results are conservative: `COMMUTATION_RESULT_UNPROVEN` only means the configured proof sources could not establish commutation, not that the two operations fail to commute. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## Constants

Commutation proof tags (the return value of the `commutation_check` family):

| Constant | Value | Proof |
| --- | --- | --- |
| `COMMUTATION_RESULT_UNPROVEN` | 0 | Commutation could not be established with the configured proof sources. |
| `COMMUTATION_RESULT_EXACT` | 1 | Exact commutation. |
| `COMMUTATION_RESULT_UP_TO_GLOBAL_PHASE` | 2 | Commutation up to a global phase (`lhs·rhs = exp(i·phase)·rhs·lhs`); the phase is written to `phase`. |

---

## Configuration Struct

### CCommutationConfig / commutation_config_default()

```c
typedef struct CCommutationConfig {
  uint8_t enable_rule_oracle;
  uint8_t enable_matrix_fallback;
  uint8_t _pad[6];
  uintptr_t max_matrix_qubits;
  uint64_t _reserved[2];
} CCommutationConfig;

struct CCommutationConfig commutation_config_default(void);
```

`commutation_config_default` returns the default configuration by value: the rule oracle and the matrix fallback are enabled and `max_matrix_qubits` is 4.

| Field | Type | Meaning |
| --- | --- | --- |
| `enable_rule_oracle` | `uint8_t` | 1 enables matching explicit knowledge-base commutation rules (`A; B -> B; A`). |
| `enable_matrix_fallback` | `uint8_t` | 1 enables small local matrix comparison as a fallback. |
| `_pad[6]` | `uint8_t[6]` | Reserved padding to keep the struct layout stable; do not write non-zero values. |
| `max_matrix_qubits` | `uintptr_t` | Maximum union-support size for the matrix fallback (computed on the sorted union of both operations' supports). |
| `_reserved[2]` | `uint64_t[2]` | Reserved padding; do not write non-zero values. |

---

## One-Shot Checks

### commutation_check(lhs_gate, lhs_qubits, lhs_qubits_len, lhs_params, lhs_params_len, rhs_gate, rhs_qubits, rhs_qubits_len, rhs_params, rhs_params_len, phase)

```c
int32_t commutation_check(const char *lhs_gate,
                          const uint32_t *lhs_qubits,
                          uintptr_t lhs_qubits_len,
                          const double *lhs_params,
                          uintptr_t lhs_params_len,
                          const char *rhs_gate,
                          const uint32_t *rhs_qubits,
                          uintptr_t rhs_qubits_len,
                          const double *rhs_params,
                          uintptr_t rhs_params_len,
                          double *phase);
```

Checks whether two gate applications commute using the shared builtin checker (knowledge rules and matrix fallback enabled). When the proof is up to a global phase, the phase is written to `phase` (a symbolic phase that cannot be evaluated numerically surfaces as NaN); exact proofs write 0.0; nothing is written when commutation cannot be proven. `phase` may be NULL.

Parameters:

- `lhs_gate` (`const char*`): left-hand gate name.
- `lhs_qubits` (`const uint32_t*`): left-hand qubit-id array.
- `lhs_qubits_len` (`uintptr_t`): left-hand qubit count.
- `lhs_params` (`const double*`): left-hand numeric parameter array; may be NULL when its length is 0.
- `lhs_params_len` (`uintptr_t`): left-hand parameter count.
- `rhs_gate` (`const char*`): right-hand gate name.
- `rhs_qubits` (`const uint32_t*`): right-hand qubit-id array.
- `rhs_qubits_len` (`uintptr_t`): right-hand qubit count.
- `rhs_params` (`const double*`): right-hand numeric parameter array; may be NULL when its length is 0.
- `rhs_params_len` (`uintptr_t`): right-hand parameter count.
- `phase` (`double*`): global-phase output pointer; may be NULL.

Return value:

| Value | Scenario |
| --- | --- |
| 0 / 1 / 2 | A `COMMUTATION_RESULT_*` proof tag. |
| -1 | A gate name is NULL, or a qubit/parameter array with length greater than 0 is NULL. |
| -4 | A gate name is invalid UTF-8. |
| -8 | A gate name is not a known standard gate. |

### commutation_check_algebraic(lhs_gate, lhs_qubits, lhs_qubits_len, lhs_params, lhs_params_len, rhs_gate, rhs_qubits, rhs_qubits_len, rhs_params, rhs_params_len, phase)

```c
int32_t commutation_check_algebraic(const char *lhs_gate,
                                    const uint32_t *lhs_qubits,
                                    uintptr_t lhs_qubits_len,
                                    const double *lhs_params,
                                    uintptr_t lhs_params_len,
                                    const char *rhs_gate,
                                    const uint32_t *rhs_qubits,
                                    uintptr_t rhs_qubits_len,
                                    const double *rhs_params,
                                    uintptr_t rhs_params_len,
                                    double *phase);
```

Checks commutation using only the symbolic algebraic proof sources (structural facts, diagonal/axis algebra, and the controlled-axis and symmetric two-qubit families). Parameters and return values match `commutation_check`; when commutation cannot be proven it returns `COMMUTATION_RESULT_UNPROVEN` (0), keeping the result conservative.

---

## Reusable Checkers

### commutation_checker_new(config)

```c
struct CCommutationChecker *commutation_checker_new(struct CCommutationConfig config);
```

Creates a reusable checker with builtin commutation rules and an explicit configuration; free with `commutation_checker_free`.

Parameters:

- `config` (`struct CCommutationConfig`): checker configuration, passed by value.

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A new checker handle; free with `commutation_checker_free`. |

### commutation_checker_from_library(library, config)

```c
struct CCommutationChecker *commutation_checker_from_library(const struct CKnowledgeLibrary *library,
                                                             struct CCommutationConfig config);
```

Creates a checker that draws its commutation rules from an already loaded rule library: the library's commutation-kind rules participate in proofs. The library handle (construction in [Knowledge](7_knowledge.md)) is not taken over and remains owned by the caller.

Parameters:

- `library` (`const CKnowledgeLibrary*`): rule library handle.
- `config` (`struct CCommutationConfig`): checker configuration, passed by value.

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A new checker handle; free with `commutation_checker_free`. |
| NULL | `library` is NULL. |

### commutation_checker_free(ptr)

```c
void commutation_checker_free(struct CCommutationChecker *ptr);
```

Frees a checker handle; NULL is allowed.

### commutation_checker_check(ptr, lhs_gate, lhs_qubits, lhs_qubits_len, lhs_params, lhs_params_len, rhs_gate, rhs_qubits, rhs_qubits_len, rhs_params, rhs_params_len, phase)

```c
int32_t commutation_checker_check(const struct CCommutationChecker *ptr,
                                  const char *lhs_gate,
                                  const uint32_t *lhs_qubits,
                                  uintptr_t lhs_qubits_len,
                                  const double *lhs_params,
                                  uintptr_t lhs_params_len,
                                  const char *rhs_gate,
                                  const uint32_t *rhs_qubits,
                                  uintptr_t rhs_qubits_len,
                                  const double *rhs_params,
                                  uintptr_t rhs_params_len,
                                  double *phase);
```

Checks whether two gate applications commute using this checker; the checker's configuration selects which proof sources participate. Apart from the leading handle parameter, parameters and return values match `commutation_check`, with the -1 scenario additionally covering a NULL `ptr`.

Parameters:

- `ptr` (`const CCommutationChecker*`): checker handle.
- Remaining parameters as in `commutation_check`.

Return value:

| Value | Scenario |
| --- | --- |
| 0 / 1 / 2 | A `COMMUTATION_RESULT_*` proof tag. |
| -1 | `ptr` is NULL, or one of the -1 scenarios of `commutation_check`. |
| -4 | A gate name is invalid UTF-8. |
| -8 | A gate name is not a known standard gate. |

---

## Examples

One-shot checks:

```c
#include <math.h>
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    uint32_t q0[1] = {0};
    uint32_t q1[1] = {1};
    double angle_a[1] = {0.3};
    double angle_b[1] = {0.5};
    double phase = 0.0;

    /* 1. Disjoint supports commute exactly. */
    int32_t r = commutation_check("X", q0, 1, NULL, 0, "X", q1, 1, NULL, 0, &phase);
    printf("disjoint: %d (phase %f)\n", r, phase);   /* 1, 0.0 */

    /* 2. X and Z on the same qubit commute up to a global phase of pi. */
    phase = 0.0;
    r = commutation_check("X", q0, 1, NULL, 0, "Z", q0, 1, NULL, 0, &phase);
    printf("x/z: %d (phase %f)\n", r, phase);        /* 2, pi */

    /* 3. Diagonal rotations with numeric parameters commute exactly. */
    r = commutation_check("RZ", q0, 1, angle_a, 1, "RZ", q0, 1, angle_b, 1, &phase);
    printf("rz/rz: %d\n", r);                        /* 1 */

    /* 4. The algebraic-only variant proves the same structural facts. */
    r = commutation_check_algebraic("X", q0, 1, NULL, 0, "X", q1, 1, NULL, 0, &phase);
    printf("algebraic disjoint: %d\n", r);           /* 1 */
    return 0;
}
```

Reusable checkers:

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    uint32_t q0[1] = {0};
    uint32_t q1[1] = {1};
    double phase = 0.0;

    /* 1. A reusable checker with the default configuration. */
    struct CCommutationConfig config = commutation_config_default();
    struct CCommutationChecker *checker = commutation_checker_new(config);
    if (checker == NULL) {
        return 1;
    }
    int32_t r = commutation_checker_check(checker, "X", q0, 1, NULL, 0,
                                          "X", q1, 1, NULL, 0, &phase);
    printf("checker disjoint: %d\n", r);             /* 1 */
    commutation_checker_free(checker);

    /* 2. A checker drawing commutation rules from a knowledge library. */
    struct CKnowledgeLibrary *lib = knowledge_library_builtin();
    if (lib != NULL) {
        struct CCommutationChecker *from_lib = commutation_checker_from_library(lib, config);
        if (from_lib != NULL) {
            r = commutation_checker_check(from_lib, "X", q0, 1, NULL, 0,
                                           "X", q1, 1, NULL, 0, &phase);
            printf("library checker disjoint: %d\n", r);   /* 1 */
            commutation_checker_free(from_lib);
        }
        knowledge_library_free(lib);
    }
    return 0;
}
```

A checker proves commutation in the order structural facts → symbolic algebra for supported gate families → explicit knowledge-base commutation rules (when enabled) → concrete local matrix comparison (when enabled and small enough); parameter comparison is symbolic and tolerant, so provably equal expressions are treated as the same application.
