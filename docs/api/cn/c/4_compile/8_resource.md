# 资源管理（C）

`CResourceManager` 管理 ancilla 辅助比特的申请、规划、租借与回收：预览（preview）产生无副作用的规划快照，提交（commit）才真正占用比特并扩展管理器的工作线路，租约（lease）持有已占用的比特，释放（release）把它们归还资源池。干净资源与脏资源是恢复契约：干净资源进出变换时必须处于 `|0>`；脏资源可处于未知或纠缠态，但消耗资源的变换必须在释放租约前完整还原其输入状态。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)；线路句柄的构造见 [Circuit](../0_circuit/1_circuit.md)。

---

## 常量

ancilla 需求标签（`resource_manager_preview` 的 `requirement` 参数与 `resource_plan_requirement` / `resource_lease_requirement` 的返回值）：

| 常量 | 值 | 需求 |
| --- | --- | --- |
| `RESOURCE_REQUIREMENT_CLEAN_ZERO` | 0 | 干净资源：进出变换时处于 `\|0>`。 |
| `RESOURCE_REQUIREMENT_DIRTY` | 1 | 脏资源：可进入未知态，必须精确还原。 |

---

## 配置结构体

### CResourcePolicy / resource_policy_default()

```c
typedef struct CResourcePolicy {
  uintptr_t max_pre_layout_clean_ancillas;
  uint8_t allow_dirty_borrowing;
  uint8_t _reserved[7];
} CResourcePolicy;

struct CResourcePolicy resource_policy_default(void);
```

`resource_policy_default` 按值返回默认策略：不创建干净 ancilla、不允许脏借用。

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `max_pre_layout_clean_ancillas` | `uintptr_t` | 布局前编译器可创建的逻辑干净 ancilla 总数；已释放比特仍计入总数、可复用。 |
| `allow_dirty_borrowing` | `uint8_t` | 1 允许在脏资源契约下借用输入比特；0 禁止。 |
| `_reserved[7]` | `uint8_t[7]` | 填充字段，保持结构体布局稳定，勿写入非零值。 |

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

`resource_limits_default` 按值返回默认限制：不强制总比特上限。

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `has_max_total_qubits` | `uint8_t` | 1 启用 `max_total_qubits` 上限；0 不设上限。 |
| `_pad[7]` | `uint8_t[7]` | 填充字段，保持结构体布局稳定，勿写入非零值。 |
| `max_total_qubits` | `uintptr_t` | 完整逻辑线路的最大总比特数（含输入比特与编译器分配的干净 ancilla），仅在 `has_max_total_qubits != 0` 时生效。 |
| `_reserved[2]` | `uint64_t[2]` | 填充字段，勿写入非零值。 |

---

## 管理器生命周期

### resource_manager_from_circuit(circuit, policy, limits)

```c
struct CResourceManager *resource_manager_from_circuit(const struct CCircuit *circuit,
                                                       const struct CResourcePolicy *policy,
                                                       const struct CResourceLimits *limits);
```

创建与线路同步的资源管理器。管理器持有线路的私有工作副本（提交规划时被修改），不接管入参线路。`policy` 与 `limits` 可传 NULL，选用默认值。

参数：

- `circuit` (`const CCircuit*`)：源线路。
- `policy` (`const CResourcePolicy*`)：资源策略；NULL 选用默认值。
- `limits` (`const CResourceLimits*`)：资源上限；NULL 选用默认值。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 新建管理器句柄，用 `resource_manager_free` 释放。 |
| NULL | `circuit` 为 NULL，或线路已超出配置上限。 |

### resource_manager_free(ptr)

```c
void resource_manager_free(struct CResourceManager *ptr);
```

释放管理器句柄，允许传 NULL。管理器内部的工作线路随之释放；`resource_manager_circuit` 取出的副本不受影响。

### resource_manager_circuit(ptr)

```c
struct CCircuit *resource_manager_circuit(const struct CResourceManager *ptr);
```

取出管理器当前工作线路的独立副本（提交规划会扩展该线路）。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 新建 `CCircuit*`，用 `circuit_free` 释放；与 `ptr` 互相独立。 |
| NULL | `ptr` 为 NULL。 |

### resource_manager_enter_post_layout(ptr)

```c
int32_t resource_manager_enter_post_layout(struct CResourceManager *ptr);
```

把管理器切换到布局后阶段：此后不再分配新的逻辑干净 ancilla。

错误码：

| 值 | 场景 |
| --- | --- |
| 0 | 成功。 |
| -1 | `ptr` 为 NULL。 |
| -6 | 管理器与工作线路一致性校验失败。 |

---

## 规划（预览）

### resource_manager_preview(ptr, requirement, count, excluded, excluded_len)

```c
struct CResourcePlan *resource_manager_preview(const struct CResourceManager *ptr,
                                               uint8_t requirement,
                                               uintptr_t count,
                                               const uint32_t *excluded,
                                               uintptr_t excluded_len);
```

对照管理器预演一次 ancilla 申请：不占用资源、不修改工作线路。`excluded` 列出不得作为辅助资源消耗的比特（通常包含数据、控制与目标比特）；`excluded_len` 为 0 时 `excluded` 可为 NULL。规划是管理器专属快照：任何成功的资源池变更（如提交另一个规划）都会使旧规划失效。

参数：

- `ptr` (`const CResourceManager*`)：资源管理器。
- `requirement` (`uint8_t`)：ancilla 需求标签，取 `RESOURCE_REQUIREMENT_*` 之一。
- `count` (`uintptr_t`)：申请的辅助比特数。
- `excluded` (`const uint32_t*`)：排除比特 id 数组。
- `excluded_len` (`uintptr_t`)：排除数组长度。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 新建规划句柄，用 `resource_plan_free` 释放。 |
| NULL | `ptr` 为 NULL、`requirement` 非法、`excluded_len` 大于 0 而 `excluded` 为 NULL，或申请无法满足。 |

### resource_plan_free(ptr)

```c
void resource_plan_free(struct CResourcePlan *ptr);
```

释放规划句柄，允许传 NULL。规划被 `resource_manager_commit` 提交时句柄由该调用接管，无需再释放。

### resource_plan_qubits_len(ptr) / resource_plan_qubits(ptr, out, len)

```c
uintptr_t resource_plan_qubits_len(const struct CResourcePlan *ptr);
uintptr_t resource_plan_qubits(const struct CResourcePlan *ptr, uint32_t *out, uintptr_t len);
```

两步式读取规划比特：`*_len` 返回规划比特数；填充接口把比特 id 拷入 `out`（实际拷贝 `min(总数, len)` 个），返回总数。`out` 为 NULL 时只返回总数、不拷贝。

参数：

- `ptr` (`const CResourcePlan*`)：规划句柄。
- `out` (`uint32_t*`)：接收比特 id 的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度。

返回值（两者一致）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 规划比特总数。 |
| 0 | `ptr` 为 NULL。 |

### resource_plan_num_new_qubits(ptr)

```c
uintptr_t resource_plan_num_new_qubits(const struct CResourcePlan *ptr);
```

返回规划提交时引入的新逻辑比特数（超出输入线路宽度的部分）。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 新比特数。 |
| 0 | `ptr` 为 NULL。 |

### resource_plan_requirement(ptr)

```c
int32_t resource_plan_requirement(const struct CResourcePlan *ptr);
```

返回规划的 ancilla 需求标签。

返回值：

| 值 | 场景 |
| --- | --- |
| 0 / 1 | `RESOURCE_REQUIREMENT_*` 标签。 |
| -1 | `ptr` 为 NULL。 |

---

## 提交与租约

### resource_manager_commit(ptr, plan)

```c
struct CResourceLease *resource_manager_commit(struct CResourceManager *ptr,
                                               struct CResourcePlan *plan);
```

提交规划：占用其比特、扩展管理器的工作线路并返回生效的租约。

**所有权约定**：`plan` 句柄被此调用接管（无论提交成功与否），调用之后不得再使用或释放原句柄。

参数：

- `ptr` (`CResourceManager*`)：资源管理器。
- `plan` (`CResourcePlan*`)：待提交规划，所有权被接管。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 新建租约句柄；先用 `resource_manager_release` 归还比特，再用 `resource_lease_free` 释放。 |
| NULL | `ptr` 或 `plan` 为 NULL，或规划已失效。 |

### resource_lease_free(ptr)

```c
void resource_lease_free(struct CResourceLease *ptr);
```

释放租约句柄，允许传 NULL。释放前应已调用 `resource_manager_release` 归还比特。

### resource_lease_qubits_len(ptr) / resource_lease_qubits(ptr, out, len)

```c
uintptr_t resource_lease_qubits_len(const struct CResourceLease *ptr);
uintptr_t resource_lease_qubits(const struct CResourceLease *ptr, uint32_t *out, uintptr_t len);
```

两步式读取租约比特：`*_len` 返回租借比特数；填充接口把比特 id 拷入 `out`（实际拷贝 `min(总数, len)` 个），返回总数。`out` 为 NULL 时只返回总数、不拷贝。

参数：

- `ptr` (`const CResourceLease*`)：租约句柄。
- `out` (`uint32_t*`)：接收比特 id 的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度。

返回值（两者一致）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 租借比特总数。 |
| 0 | `ptr` 为 NULL。 |

### resource_lease_requirement(ptr)

```c
int32_t resource_lease_requirement(const struct CResourceLease *ptr);
```

返回租约的 ancilla 需求标签。

返回值：

| 值 | 场景 |
| --- | --- |
| 0 / 1 | `RESOURCE_REQUIREMENT_*` 标签。 |
| -1 | `ptr` 为 NULL。 |

### resource_manager_release(ptr, lease)

```c
int32_t resource_manager_release(struct CResourceManager *ptr, const struct CResourceLease *lease);
```

释放生效的租约，把其比特归还资源池。消耗资源的变换必须在调用前完成状态恢复：干净 ancilla 回到 `|0>`，脏 ancilla 还原输入状态。租约句柄在释放后仍可读取，直到用 `resource_lease_free` 释放。

错误码：

| 值 | 场景 |
| --- | --- |
| 0 | 成功。 |
| -1 | `ptr` 或 `lease` 为 NULL。 |
| -6 | 租约未知或已释放。 |

---

## 一致性检查

### resource_manager_verify_consistency(ptr)

```c
int32_t resource_manager_verify_consistency(const struct CResourceManager *ptr);
```

校验管理器与其工作线路仍然一致。

错误码：

| 值 | 场景 |
| --- | --- |
| 0 | 一致。 |
| -1 | `ptr` 为 NULL。 |
| -6 | 不一致。 |

### resource_manager_verify_idle(ptr)

```c
int32_t resource_manager_verify_idle(const struct CResourceManager *ptr);
```

校验全部租约已释放、资源池空闲。

错误码：

| 值 | 场景 |
| --- | --- |
| 0 | 空闲。 |
| -1 | `ptr` 为 NULL。 |
| -6 | 仍有活跃租约。 |

---

## 示例

申请、提交与释放一个干净 ancilla 的完整生命周期：

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

预览失败返回 NULL 的常见原因：需求标签非法、申请数超出策略允许的干净 ancilla 数，或排除列表后无法满足。
