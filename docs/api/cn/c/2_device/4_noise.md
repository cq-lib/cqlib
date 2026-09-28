# NoiseModel（C）

`CNoiseModel` 是噪声模型的不透明句柄，按「标准门 + 比特」组合登记噪声信道，按比特登记读出误差；同一个门与比特组合可以登记多条信道。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 通道常量

单比特通道标签（用于 `noise_model_add_single_qubit` 的 `noise_type` 与 `CSingleQubitNoise.tag`）：

| 常量 | 值 | 通道 | 参数含义 |
| --- | --- | --- | --- |
| `NOISE_BIT_FLIP` | 0 | 比特翻转 | 翻转概率 p |
| `NOISE_PHASE_FLIP` | 1 | 相位翻转 | 翻转概率 p |
| `NOISE_DEPOLARIZING` | 2 | 去极化 | 去极化参数 p |
| `NOISE_AMPLITUDE_DAMPING` | 3 | 振幅阻尼 | 阻尼参数 gamma |
| `NOISE_PHASE_DAMPING` | 4 | 相位阻尼 | 散射概率 |
| `NOISE_PAULI` | 5 | Pauli 信道 | 概率 `px`/`py`/`pz`（和 ≤ 1） |

双比特通道标签（`NOISE_TWO_DEPOLARIZING` 用于 `noise_model_add_two_qubit`，全部用于 `CTwoQubitNoise.tag`）：

| 常量 | 值 | 通道 | 参数含义 |
| --- | --- | --- | --- |
| `NOISE_TWO_DEPOLARIZING` | 0 | 去极化 | 去极化参数 p |
| `NOISE_TWO_INDEPENDENT` | 1 | 独立信道组合 | q0/q1 各自的单比特信道 |
| `NOISE_TWO_CORRELATED_PAULI` | 2 | 关联 Pauli | 概率 p + `NOISE_PAULI_OP_*` 算子 |

关联 Pauli 通道的算子标签（`CTwoQubitNoise.op_q0` / `op_q1`）：`NOISE_PAULI_OP_I` = 0、`NOISE_PAULI_OP_X` = 1、`NOISE_PAULI_OP_Y` = 2、`NOISE_PAULI_OP_Z` = 3。

---

## 快照结构体

### CSingleQubitNoise

单比特噪声信道快照，由 `noise_model_get_single_qubit_errors` 写出。`tag` 选取信道；`p` 承载标签 0–4 的参数，`px`/`py`/`pz` 承载标签 5 的 Pauli 概率；不适用的字段为零。

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `tag` | `uint8_t` | 单比特 `NOISE_*` 标签（0–5）。 |
| `p` | `double` | 标签 0–4 的信道概率。 |
| `px` | `double` | 标签 5 的 Pauli-X 概率。 |
| `py` | `double` | 标签 5 的 Pauli-Y 概率。 |
| `pz` | `double` | 标签 5 的 Pauli-Z 概率。 |

### CTwoQubitNoise

双比特噪声信道快照，由 `noise_model_get_two_qubit_errors` 写出。不适用的字段为零。

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `tag` | `uint8_t` | `NOISE_TWO_*` 标签。 |
| `p` | `double` | 去极化与关联 Pauli 标签的信道概率。 |
| `q0_noise` | `struct CSingleQubitNoise` | 独立标签下作用于 q0 的信道。 |
| `q1_noise` | `struct CSingleQubitNoise` | 独立标签下作用于 q1 的信道。 |
| `op_q0` | `uint8_t` | 关联标签下作用于 q0 的 Pauli 算子（`NOISE_PAULI_OP_*`）。 |
| `op_q1` | `uint8_t` | 关联标签下作用于 q1 的 Pauli 算子（`NOISE_PAULI_OP_*`）。 |

---

## 构造与释放

### noise_model_new()

创建一个空的噪声模型。

返回：新建的 `CNoiseModel*`。

### noise_model_free(ptr)

释放 `CNoiseModel`，允许传 NULL。

---

## 登记信道

### noise_model_add_single_qubit(ptr, gate_name, qubit, noise_type, p)

向标准门 `gate_name` 在 `qubit` 上登记一条单比特噪声信道。

参数：

- `gate_name` (`const char*`)：标准门名。
- `qubit` (`uint32_t`)：比特 ID。
- `noise_type` (`uint8_t`)：`NOISE_*` 单比特标签（0–4）。
- `p` (`double`)：信道参数。

返回：0 成功；-1 空指针；-4 门名未知或非法 UTF-8；-8 噪声标签未知或概率非法。

### noise_model_add_single_qubit_pauli(ptr, gate_name, qubit, px, py, pz)

向标准门 `gate_name` 在 `qubit` 上登记一条一般单比特 Pauli 信道，X/Y/Z 概率为 `px`、`py`、`pz`（各自 ≥ 0，和 ≤ 1）。

参数：

- `px` / `py` / `pz` (`double`)：Pauli-X/Y/Z 概率。

返回：0 成功；-1 空指针；-4 门名未知；-8 概率非法。

### noise_model_add_two_qubit(ptr, gate_name, q0, q1, noise_type, p)

向标准门 `gate_name` 在有序比特对 `(q0, q1)` 上登记一条双比特去极化信道。`noise_type` 只接受 `NOISE_TWO_DEPOLARIZING`（0）。

参数：

- `q0` / `q1` (`uint32_t`)：比特 ID。
- `noise_type` (`uint8_t`)：`NOISE_TWO_DEPOLARIZING`。
- `p` (`double`)：去极化参数。

返回：0 成功；-1 空指针；-4 门名未知；-8 比特相同或概率非法。

### noise_model_add_readout(ptr, qubit, p_0_given_1, p_1_given_0)

为 `qubit` 登记非对称读出误差。

参数：

- `qubit` (`uint32_t`)：比特 ID。
- `p_0_given_1` (`double`)：P(真值为 1 测得 0)。
- `p_1_given_0` (`double`)：P(真值为 0 测得 1)。

返回：0 成功；-8 概率非法。

---

## 查询信道

### noise_model_get_single_qubit_errors_len(ptr, gate_name, qubit)

返回标准门 `gate_name` 在 `qubit` 上登记的单比特信道数。

返回：总数；`ptr` 为 NULL、门名未知或无记录时为 0。

### noise_model_get_single_qubit_errors(ptr, gate_name, qubit, out, len)

两步式输出上述信道，与 `noise_model_get_single_qubit_errors_len` 配对；每个条目是一个 `CSingleQubitNoise` 快照，有效字段取决于其 `tag`。返回总数。

### noise_model_get_two_qubit_errors_len(ptr, gate_name, q0, q1)

返回标准门 `gate_name` 在有序比特对 `(q0, q1)` 上登记的双比特信道数。

返回：总数；`ptr` 为 NULL、门名未知、比特相同或无记录时为 0。

### noise_model_get_two_qubit_errors(ptr, gate_name, q0, q1, out, len)

两步式输出上述信道，与 `noise_model_get_two_qubit_errors_len` 配对；每个条目是一个 `CTwoQubitNoise` 快照，有效字段取决于其 `tag`。返回总数。

### noise_model_get_readout_error(ptr, qubit, out_p_0_given_1, out_p_1_given_0)

写出 `qubit` 登记的读出误差。

参数：

- `out_p_0_given_1` (`double*`)：写出 P(真值为 1 测得 0)。
- `out_p_1_given_0` (`double*`)：写出 P(真值为 0 测得 1)。

返回：0 成功；-1 空指针；-8 该比特没有登记读出误差。

---

## 信道有效性与 Kraus 算子

以下接口按 `(gate_name, qubit, index)` 键寻址一条已登记的单比特信道；`index` 的上界为 `noise_model_get_single_qubit_errors_len` 的返回值。Kraus 算子以 `Complex64` 的 re/im 对传输（`cqlib_c.h` 中定义的 `struct Complex64`，含 `re` / `im` 字段）。

### noise_model_is_valid(ptr, gate_name, qubit, index)

报告标准门 `gate_name` 在 `qubit` 上、下标为 `index` 的单比特信道是否携带合法的噪声参数。

参数：

- `gate_name` (`const char*`)：标准门名。
- `qubit` (`uint32_t`)：比特 ID。
- `index` (`uintptr_t`)：信道下标。

返回：1 合法；0 不合法；-1 空指针；-4 门名未知；-8 该键/下标处没有信道。

### noise_model_to_kraus_len(ptr, gate_name, qubit, index)

返回该信道全部 Kraus 算子的 `Complex64` 元素总数；每个算子贡献 `dim * dim` 个元素（单比特算子为 `2 * 2 = 4`）。

返回：总元素数；`ptr` 为 NULL、门名未知或键/下标无记录时返回 0。

### noise_model_to_kraus(ptr, gate_name, qubit, index, out, len)

两步式输出 Kraus 算子，与 `noise_model_to_kraus_len` 配对：算子按顺序连续写出，每个算子按行优先排列为 `dim * dim` 个 `Complex64` re/im 对。

参数：

- `out` (`Complex64*`)：接收元素；传 NULL 仅查询长度。
- `len` (`uintptr_t`)：缓冲区容量（元素数）；值较小时只复制前 `len` 个元素。

返回：总元素数；`ptr` 为 NULL、门名未知或键/下标无记录时返回 0。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CNoiseModel *nm = noise_model_new();

    /* 单比特信道：X 门上的比特翻转 */
    int32_t rc = noise_model_add_single_qubit(nm, "X", 0, NOISE_BIT_FLIP, 0.005);

    /* 单比特 Pauli 信道 */
    rc = noise_model_add_single_qubit_pauli(nm, "X", 0, 0.001, 0.002, 0.003);

    /* 双比特去极化信道：CX 门 (0, 1) */
    rc = noise_model_add_two_qubit(nm, "CX", 0, 1, NOISE_TWO_DEPOLARIZING, 0.02);

    /* 读出误差 */
    rc = noise_model_add_readout(nm, 0, 0.02, 0.01);

    /* 查询单比特信道 */
    uintptr_t n = noise_model_get_single_qubit_errors_len(nm, "X", 0);  /* 2 */
    struct CSingleQubitNoise chans[2];
    noise_model_get_single_qubit_errors(nm, "X", 0, chans, n);
    /* chans[0]: tag == NOISE_BIT_FLIP, p == 0.005
       chans[1]: tag == NOISE_PAULI, px/py/pz == 0.001/0.002/0.003 */

    /* 查询双比特信道 */
    uintptr_t m = noise_model_get_two_qubit_errors_len(nm, "CX", 0, 1);  /* 1 */
    struct CTwoQubitNoise tchans[1];
    noise_model_get_two_qubit_errors(nm, "CX", 0, 1, tchans, m);
    /* tchans[0]: tag == NOISE_TWO_DEPOLARIZING, p == 0.02 */

    /* 查询读出误差 */
    double p01 = 0.0, p10 = 0.0;
    rc = noise_model_get_readout_error(nm, 0, &p01, &p10);  /* 0，0.02 / 0.01 */

    /* 信道有效性与 Kraus 算子 */
    rc = noise_model_add_single_qubit(nm, "H", 0, NOISE_DEPOLARIZING, 0.1);
    int32_t valid = noise_model_is_valid(nm, "H", 0, 0);       /* 1 */
    uintptr_t klen = noise_model_to_kraus_len(nm, "H", 0, 0);  /* 16：四个 2x2 算子 */
    Complex64 kraus[16];
    noise_model_to_kraus(nm, "H", 0, 0, kraus, klen);
    /* kraus[0..4]：I*sqrt(0.9) 按行优先（kraus[0].re == sqrt(0.9)）
       kraus[4..16]：X、Y、Z，缩放 sqrt(0.1/3) */

    /* X@0 上的振幅阻尼，下标 2（比特翻转与 Pauli 之后） */
    rc = noise_model_add_single_qubit(nm, "X", 0, NOISE_AMPLITUDE_DAMPING, 0.25);
    klen = noise_model_to_kraus_len(nm, "X", 0, 2);            /* 8：两个 2x2 算子 */

    /* 错误码 */
    int32_t absent = noise_model_is_valid(nm, "H", 1, 0);      /* -8：该键处没有信道 */
    int32_t unknown = noise_model_is_valid(nm, "nope", 0, 0);  /* -4：门名未知 */

    noise_model_free(nm);
    return 0;
}
```
