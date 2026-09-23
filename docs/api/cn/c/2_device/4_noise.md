# NoiseModel

`CNoiseModel` 描述门噪声与读出误差模型，可配合误差缓解与含噪模拟使用。

---

## 函数

### noise_model_new()

创建空噪声模型。

返回：

- `CNoiseModel *`：堆分配模型，需 `noise_model_free` 释放；失败返回 NULL。

### noise_model_add_single_qubit(model, gate_name, qubit, noise_type, p)

为单比特门添加噪声通道。

参数：

- `gate_name` (`const char *`)：门名，如 `"H"`。
- `qubit` (`uint32_t`)：目标比特。
- `noise_type` (`uint8_t`)：噪声通道标签，取值见下表（`NOISE_BIT_FLIP` 到 `NOISE_PHASE_DAMPING`）。
- `p` (`double`)：通道参数，含义随通道而定。

返回：

- `int32_t`；成功 `0`，未知门名或未知通道标签 `-4`，概率非法 `-8`。

### noise_model_add_two_qubit(model, gate_name, q0, q1, noise_type, p)

为双比特门添加噪声通道。

参数：

- `gate_name` (`const char *`)：门名，如 `"CX"`。
- `q0`, `q1` (`uint32_t`)：作用的两个比特。
- `noise_type` (`uint8_t`)：当前仅支持 `NOISE_TWO_DEPOLARIZING`。
- `p` (`double`)：去极化参数。

返回：

- `int32_t`；成功 `0`，未知门名 `-4`，比特冲突（`q0 == q1`）或概率非法 `-8`。

### noise_model_add_readout(model, qubit, p_0_given_1, p_1_given_0)

为比特添加非对称读出误差。

参数：

- `qubit` (`uint32_t`)：目标比特。
- `p_0_given_1` (`double`)：`P(测得 0 | 真实 1)`。
- `p_1_given_0` (`double`)：`P(测得 1 | 真实 0)`。

返回：

- `int32_t`；成功 `0`，概率非法（不在 `[0, 1]`）返回 `-8`。

### noise_model_free(ptr)

释放噪声模型。允许传 NULL。

---

## 噪声通道标签

单比特通道：

| 常量 | 值 | 通道 | 参数 `p` 含义 |
| --- | --- | --- | --- |
| `NOISE_BIT_FLIP` | 0 | 比特翻转 | 翻转概率 |
| `NOISE_PHASE_FLIP` | 1 | 相位翻转 | 翻转概率 |
| `NOISE_DEPOLARIZING` | 2 | 去极化 | 去极化参数 |
| `NOISE_AMPLITUDE_DAMPING` | 3 | 振幅阻尼 | 阻尼参数 γ |
| `NOISE_PHASE_DAMPING` | 4 | 相位阻尼 | 散射概率 |

双比特通道：

| 常量 | 值 | 通道 | 参数 `p` 含义 |
| --- | --- | --- | --- |
| `NOISE_TWO_DEPOLARIZING` | 0 | 双比特去极化 | 去极化参数 |

---

## 示例

```c
CNoiseModel *noise = noise_model_new();
if (!noise) { return 1; }

/* 单比特去极化：所有 H(0) 之后以 p=0.001 去极化 */
int32_t rc = noise_model_add_single_qubit(noise, "H", 0, NOISE_DEPOLARIZING, 0.001);

/* 双比特去极化 */
rc |= noise_model_add_two_qubit(noise, "CX", 0, 1, NOISE_TWO_DEPOLARIZING, 0.01);

/* 读出误差：真实 1 被测成 0 的概率 2%，真实 0 被测成 1 的概率 1% */
rc |= noise_model_add_readout(noise, 0, 0.02, 0.01);

if (rc != 0) {
    fprintf(stderr, "add noise failed: %d\n", rc);
}

noise_model_free(noise);
```
