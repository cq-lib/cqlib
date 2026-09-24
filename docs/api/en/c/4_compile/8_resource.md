# Resource (C)

`CResourceManager` manages the request, planning, leasing, and recycling of ancilla qubits: previewing produces a side-effect-free plan snapshot, committing actually reserves the qubits and extends the manager's working circuit, a lease holds the reserved qubits, and releasing returns them to the resource pool. Clean and dirty ancillas are restoration contracts: a clean ancilla must be in `|0>` when entering and leaving the consuming transform; a dirty ancilla may be in an unknown or entangled state, but the transform must fully restore its input state before the lease is released. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md); circuit handle construction is covered in [Circuit](../0_circuit/1_circuit.md).

---

## Constants

Ancilla-requirement tags (the `requirement` parameter of `resource_manager_preview` and the return value of `resource_plan_requirement` / `resource_lease_requirement`):

| Constant | Value | Requirement |
| --- | --- | --- |
| `RESOURCE_REQUIREMENT_CLEAN_ZERO` | 0 | Clean ancilla: enters and leaves the transform in `\|0>`. |
| `RESOURCE_REQUIREMENT_DIRTY` | 1 | Dirty ancilla: may enter in an unknown state, must be restored exactly. |

---

## Configuration Structs

### CResourcePolicy / resource_policy_default()

```c
typedef struct CResourcePolicy {
  uintptr_t max_pre_layout_clean_ancillas;
  uint8_t allow_dirty_borrowing;
  uint8_t _reserved[7];
} CResourcePolicy;

struct CResourcePolicy resource_policy_default(void);
```

`resource_policy_default` returns the default policy by value: no clean ancillas, no dirty borrowing.

| Field | Type | Meaning |
| --- | --- | --- |
| `max_pre_layout_clean_ancillas` | `uintptr_t` | Total clean logical ancillas the compiler may create before layout; released qubits still count toward the total and are reusable. |
| `allow_dirty_borrowing` | `uint8_t` | 1 allows borrowing input qubits under the dirty contract; 0 forbids it. |
| `_reserved[7]` | `uint8_t[7]` | Reserved padding to keep the struct layout stable; do not write non-zero values. |

### CResourceLimits / resource_limits_default()

```c
typedef struct CResourceLimits {
  uint8_t has_max_total_qubits;
  uint8_t _pad[7];
  uintptr_t max_total_qubits;
  uint64_t _reserved[2];
} CResourceLimits;

struct CResourceLimits resource_limits_default(void);
```

`resource_limits_default` returns the default limits by value: no total-qubit bound.

| Field | Type | Meaning |
| --- | --- | --- |
| `has_max_total_qubits` | `uint8_t` | 1 enforces the `max_total_qubits` limit; 0 leaves it unset. |
| `_pad[7]` | `uint8_t[7]` | Reserved padding to keep the struct layout stable; do not write non-zero values. |
| `max_total_qubits` | `uintptr_t` | Hard limit on total logical qubits of the complete circuit (input qubits plus compiler-allocated clean ancillas); used only when `has_max_total_qubits != 0`. |
| `_reserved[2]` | `uint64_t[2]` | Reserved padding; do not write non-zero values. |

---

## Manager Lifecycle

### resource_manager_from_circuit(circuit, policy, limits)

```c
struct CResourceManager *resource_manager_from_circuit(const struct CCircuit *circuit,
                                                       const struct CResourcePolicy *policy,
                                                       const struct CResourceLimits *limits);
```

Creates a resource manager synchronized with a circuit. The manager owns a private working copy of the circuit (extended when plans are committed) and does not take over the input circuit. `policy` and `limits` may be NULL to select the defaults.

Parameters:

- `circuit` (`const CCircuit*`): source circuit.
- `policy` (`const CResourcePolicy*`): resource policy; NULL selects the default.
- `limits` (`const CResourceLimits*`): resource limits; NULL selects the default.

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A new manager handle; free with `resource_manager_free`. |
| NULL | `circuit` is NULL, or the circuit already exceeds the configured limits. |

### resource_manager_free(ptr)

```c
void resource_manager_free(struct CResourceManager *ptr);
```

Frees a manager handle; NULL is allowed. The manager's internal working circuit is released with it; copies taken out by `resource_manager_circuit` are unaffected.

### resource_manager_circuit(ptr)

```c
struct CCircuit *resource_manager_circuit(const struct CResourceManager *ptr);
```

Takes out an owned copy of the manager's current working circuit (committing a plan extends it).

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A new `CCircuit*`; free with `circuit_free`. Independent of `ptr`. |
| NULL | `ptr` is NULL. |

### resource_manager_enter_post_layout(ptr)

```c
int32_t resource_manager_enter_post_layout(struct CResourceManager *ptr);
```

Moves the manager into the post-layout phase: no fresh logical clean ancillas are allocated afterwards.

Error codes:

| Value | Scenario |
| --- | --- |
| 0 | Success. |
| -1 | `ptr` is NULL. |
| -6 | A consistency check between the manager and its working circuit failed. |

---

## Planning (Preview)

### resource_manager_preview(ptr, requirement, count, excluded, excluded_len)

```c
struct CResourcePlan *resource_manager_preview(const struct CResourceManager *ptr,
                                               uint8_t requirement,
                                               uintptr_t count,
                                               const uint32_t *excluded,
                                               uintptr_t excluded_len);
```

Previews an ancilla request against the manager without reserving resources or mutating the working circuit. `excluded` lists qubits that must not be consumed as ancillary resources (typically data, control, and target qubits); it may be NULL when `excluded_len` is 0. A plan is a manager-private snapshot: any successful change to the resource pool (for example committing another plan) invalidates older plans.

Parameters:

- `ptr` (`const CResourceManager*`): resource manager.
- `requirement` (`uint8_t`): ancilla-requirement tag, one of `RESOURCE_REQUIREMENT_*`.
- `count` (`uintptr_t`): number of ancillas requested.
- `excluded` (`const uint32_t*`): array of excluded qubit ids.
- `excluded_len` (`uintptr_t`): length of the excluded array.

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A new plan handle; free with `resource_plan_free`. |
| NULL | `ptr` is NULL, `requirement` is invalid, `excluded_len` is greater than 0 while `excluded` is NULL, or the request cannot be satisfied. |

### resource_plan_free(ptr)

```c
void resource_plan_free(struct CResourcePlan *ptr);
```

Frees a plan handle; NULL is allowed. When the plan is committed by `resource_manager_commit`, the handle is consumed by that call and must not be freed again.

### resource_plan_qubits_len(ptr) / resource_plan_qubits(ptr, out, len)

```c
uintptr_t resource_plan_qubits_len(const struct CResourcePlan *ptr);
uintptr_t resource_plan_qubits(const struct CResourcePlan *ptr, uint32_t *out, uintptr_t len);
```

Two-step read of the planned qubits: `*_len` returns the number of planned qubits; the fill function copies the qubit ids into `out` (writing `min(total, len)` entries) and returns the total. When `out` is NULL it only returns the total without copying.

Parameters:

- `ptr` (`const CResourcePlan*`): plan handle.
- `out` (`uint32_t*`): buffer receiving the qubit ids.
- `len` (`uintptr_t`): buffer length.

Return value (both functions):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of planned qubits. |
| 0 | `ptr` is NULL. |

### resource_plan_num_new_qubits(ptr)

```c
uintptr_t resource_plan_num_new_qubits(const struct CResourcePlan *ptr);
```

Returns the number of new logical qubits the plan introduces when committed (the part beyond the input circuit width).

Return value:

| Return | Scenario |
| --- | --- |
| Non-negative | The number of new qubits. |
| 0 | `ptr` is NULL. |

### resource_plan_requirement(ptr)

```c
int32_t resource_plan_requirement(const struct CResourcePlan *ptr);
```

Returns the plan's ancilla-requirement tag.

Return value:

| Value | Scenario |
| --- | --- |
| 0 / 1 | A `RESOURCE_REQUIREMENT_*` tag. |
| -1 | `ptr` is NULL. |

---

## Commit and Lease

### resource_manager_commit(ptr, plan)

```c
struct CResourceLease *resource_manager_commit(struct CResourceManager *ptr,
                                               struct CResourcePlan *plan);
```

Commits a previewed plan: reserves its qubits, extends the manager's working circuit, and returns the active lease.

**Ownership**: the `plan` handle is consumed (freed) by this call, whether the commit succeeds or not; after the call, do not use or free the original handle again.

Parameters:

- `ptr` (`CResourceManager*`): resource manager.
- `plan` (`CResourcePlan*`): plan to commit whose ownership is taken.

Return value:

| Return | Scenario |
| --- | --- |
| Non-NULL | A new lease handle; return the qubits with `resource_manager_release` first, then free it with `resource_lease_free`. |
| NULL | `ptr` or `plan` is NULL, or the plan is stale. |

### resource_lease_free(ptr)

```c
void resource_lease_free(struct CResourceLease *ptr);
```

Frees a lease handle; NULL is allowed. `resource_manager_release` should have been called first to return the qubits.

### resource_lease_qubits_len(ptr) / resource_lease_qubits(ptr, out, len)

```c
uintptr_t resource_lease_qubits_len(const struct CResourceLease *ptr);
uintptr_t resource_lease_qubits(const struct CResourceLease *ptr, uint32_t *out, uintptr_t len);
```

Two-step read of the leased qubits: `*_len` returns the number of leased qubits; the fill function copies the qubit ids into `out` (writing `min(total, len)` entries) and returns the total. When `out` is NULL it only returns the total without copying.

Parameters:

- `ptr` (`const CResourceLease*`): lease handle.
- `out` (`uint32_t*`): buffer receiving the qubit ids.
- `len` (`uintptr_t`): buffer length.

Return value (both functions):

| Return | Scenario |
| --- | --- |
| Non-negative | The total number of leased qubits. |
| 0 | `ptr` is NULL. |

### resource_lease_requirement(ptr)

```c
int32_t resource_lease_requirement(const struct CResourceLease *ptr);
```

Returns the lease's ancilla-requirement tag.

Return value:

| Value | Scenario |
| --- | --- |
| 0 / 1 | A `RESOURCE_REQUIREMENT_*` tag. |
| -1 | `ptr` is NULL. |

### resource_manager_release(ptr, lease)

```c
int32_t resource_manager_release(struct CResourceManager *ptr, const struct CResourceLease *lease);
```

Releases an active lease, returning its qubits to the resource pool. The consuming transform must have restored the state contract before this call: clean ancillas back to `|0>`, dirty ancillas restored to their input state. The lease handle stays valid for inspection until freed with `resource_lease_free`.

Error codes:

| Value | Scenario |
| --- | --- |
| 0 | Success. |
| -1 | `ptr` or `lease` is NULL. |
| -6 | The lease is unknown or already released. |

---

## Consistency Checks

### resource_manager_verify_consistency(ptr)

```c
int32_t resource_manager_verify_consistency(const struct CResourceManager *ptr);
```

Checks that the manager still agrees with its working circuit.

Error codes:

| Value | Scenario |
| --- | --- |
| 0 | Consistent. |
| -1 | `ptr` is NULL. |
| -6 | Mismatch. |

### resource_manager_verify_idle(ptr)

```c
int32_t resource_manager_verify_idle(const struct CResourceManager *ptr);
```

Checks that every lease has been released and the pool is idle.

Error codes:

| Value | Scenario |
| --- | --- |
| 0 | Idle. |
| -1 | `ptr` is NULL. |
| -6 | Leases are still active. |

---

## Example

Full lifecycle of requesting, committing, and releasing one clean ancilla:

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Input circuit and a policy allowing two clean ancillas. */
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);

    struct CResourcePolicy policy = resource_policy_default();
    policy.max_pre_layout_clean_ancillas = 2;

    struct CResourceManager *manager = resource_manager_from_circuit(qc, &policy, NULL);
    if (manager == NULL) {
        circuit_free(qc);
        return 1;
    }

    /* 2. Preview one clean ancilla without touching the circuit. */
    struct CResourcePlan *plan =
        resource_manager_preview(manager, RESOURCE_REQUIREMENT_CLEAN_ZERO, 1, NULL, 0);
    if (plan == NULL) {
        resource_manager_free(manager);
        circuit_free(qc);
        return 1;
    }
    uint32_t planned[1];
    resource_plan_qubits(plan, planned, 1);                /* planned[0] == 2 */
    uintptr_t new_qubits = resource_plan_num_new_qubits(plan);  /* 1 */

    /* 3. Commit: the plan handle is consumed and the lease becomes active. */
    struct CResourceLease *lease = resource_manager_commit(manager, plan);
    if (lease == NULL) {
        resource_manager_free(manager);
        circuit_free(qc);
        return 1;
    }
    uint32_t leased[1];
    resource_lease_qubits(lease, leased, 1);              /* leased[0] == planned[0] */
    printf("leased qubit %u, %llu new qubits\n",
           leased[0], (unsigned long long)new_qubits);

    /* 4. The working circuit now spans the borrowed qubit. */
    struct CCircuit *working = resource_manager_circuit(manager);
    printf("working qubits: %llu\n",
           (unsigned long long)circuit_num_qubits(working));  /* 3 */
    circuit_free(working);

    /* 5. Restore the ancilla to |0>, then release the lease. */
    if (resource_manager_release(manager, lease) != 0) {
        resource_lease_free(lease);
        resource_manager_free(manager);
        circuit_free(qc);
        return 1;
    }
    if (resource_manager_verify_idle(manager) == 0) {
        printf("all leases released\n");
    }

    resource_lease_free(lease);
    resource_manager_free(manager);
    circuit_free(qc);
    return 0;
}
```

Common reasons for a NULL preview result: an invalid requirement tag, a count exceeding the clean-ancilla budget of the policy, or an exclusion list that makes the request unsatisfiable.
