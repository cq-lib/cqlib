# 对易检查（C）

判断两个具体门应用是否对易。每个门应用由标准门名（如 `"H"`、`"RZ"`、`"CX"`）、比特 id 数组与数值参数数组描述；证明为相差全局相位时把相位写入 `phase` 出参。证明结果是保守的：`COMMUTATION_RESULT_UNPROVEN` 只表示配置的证明源无法建立对易性，不表示两个操作不对易。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 常量

对易证明标签（`commutation_check` 系列的返回值）：

| 常量 | 值 | 证明 |
| --- | --- | --- |
| `COMMUTATION_RESULT_UNPROVEN` | 0 | 配置的证明源无法建立对易性。 |
| `COMMUTATION_RESULT_EXACT` | 1 | 精确对易。 |
| `COMMUTATION_RESULT_UP_TO_GLOBAL_PHASE` | 2 | 相差全局相位对易（`lhs·rhs = exp(i·phase)·rhs·lhs`），相位写入 `phase`。 |

---

## 配置结构体

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

`commutation_config_default` 按值返回默认配置：启用规则预言机与矩阵回退，`max_matrix_qubits` 为 4。

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `enable_rule_oracle` | `uint8_t` | 1 启用知识库显式对易规则（`A; B -> B; A`）匹配。 |
| `enable_matrix_fallback` | `uint8_t` | 1 启用小规模局部矩阵比较回退。 |
| `_pad[6]` | `uint8_t[6]` | 填充字段，保持结构体布局稳定，勿写入非零值。 |
| `max_matrix_qubits` | `uintptr_t` | 矩阵回退允许的最大支撑并集比特数（作用于两操作支撑的排序并集）。 |
| `_reserved[2]` | `uint64_t[2]` | 填充字段，勿写入非零值。 |

---

## 一次性检查

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

用共享内置检查器（知识规则与矩阵回退启用）判断两个门应用是否对易。证明为相差全局相位时把相位写入 `phase`（无法数值评估的符号相位以 NaN 写出）；精确对易写入 0.0；无法证明时不写入。`phase` 可为 NULL。

参数：

- `lhs_gate` (`const char*`)：左侧门名。
- `lhs_qubits` (`const uint32_t*`)：左侧比特 id 数组。
- `lhs_qubits_len` (`uintptr_t`)：左侧比特数。
- `lhs_params` (`const double*`)：左侧数值参数数组；长度为 0 时可为 NULL。
- `lhs_params_len` (`uintptr_t`)：左侧参数数。
- `rhs_gate` (`const char*`)：右侧门名。
- `rhs_qubits` (`const uint32_t*`)：右侧比特 id 数组。
- `rhs_qubits_len` (`uintptr_t`)：右侧比特数。
- `rhs_params` (`const double*`)：右侧数值参数数组；长度为 0 时可为 NULL。
- `rhs_params_len` (`uintptr_t`)：右侧参数数。
- `phase` (`double*`)：全局相位出参，可为 NULL。

返回值：

| 值 | 场景 |
| --- | --- |
| 0 / 1 / 2 | `COMMUTATION_RESULT_*` 证明标签。 |
| -1 | 门名为 NULL，或长度大于 0 的比特/参数数组为 NULL。 |
| -4 | 门名非法 UTF-8。 |
| -8 | 门名不是已知的标准门。 |

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

只用符号代数证明源（结构事实、对角/轴代数、受控轴与对称双比特门族）判断对易。参数与返回值同 `commutation_check`；无法证明时返回 `COMMUTATION_RESULT_UNPROVEN`（0），保持结果保守。

---

## 可复用检查器

### commutation_checker_new(config)

```c
struct CCommutationChecker *commutation_checker_new(struct CCommutationConfig config);
```

按显式配置创建可复用检查器（使用内置对易规则），用 `commutation_checker_free` 释放。

参数：

- `config` (`struct CCommutationConfig`)：检查器配置，按值传递。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 新建检查器句柄，用 `commutation_checker_free` 释放。 |

### commutation_checker_from_library(library, config)

```c
struct CCommutationChecker *commutation_checker_from_library(const struct CKnowledgeLibrary *library,
                                                             struct CCommutationConfig config);
```

从已加载的规则库创建检查器：库中对易类别的规则参与证明。规则库句柄（构造方式见 [知识库](7_knowledge.md)）不被接管，调用方继续负责释放。

参数：

- `library` (`const CKnowledgeLibrary*`)：规则库句柄。
- `config` (`struct CCommutationConfig`)：检查器配置，按值传递。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 新建检查器句柄，用 `commutation_checker_free` 释放。 |
| NULL | `library` 为 NULL。 |

### commutation_checker_free(ptr)

```c
void commutation_checker_free(struct CCommutationChecker *ptr);
```

释放检查器句柄，允许传 NULL。

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

用该检查器判断两个门应用是否对易；检查器配置决定参与证明的证明源。除首参数外，参数与返回值同 `commutation_check`，另在 -1 场景中增加 `ptr` 为 NULL。

参数：

- `ptr` (`const CCommutationChecker*`)：检查器句柄。
- 其余参数同 `commutation_check`。

返回值：

| 值 | 场景 |
| --- | --- |
| 0 / 1 / 2 | `COMMUTATION_RESULT_*` 证明标签。 |
| -1 | `ptr` 为 NULL，或 `commutation_check` 的 -1 场景。 |
| -4 | 门名非法 UTF-8。 |
| -8 | 门名不是已知的标准门。 |

---

## 示例

一次性检查：

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

可复用检查器：

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

检查器证明顺序为：结构事实 → 支持门族的符号代数 → 显式知识库对易规则（启用时）→ 具体局部矩阵比较（启用且规模足够小时）；参数比较是符号且带容差的，可证明相等的表达式被视为同一应用。
