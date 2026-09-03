# 读取误差矫正

真实量子设备的测量存在读取误差。`cqlib-tianyan` 在获取结果时可以利用设备校准数据对测量计数做读取误差矫正。该能力由 `CalibrationMode` 控制。

对应导入：

```python
from cqlib_tianyan import CalibrationMode
```

## 1. 为什么需要读取误差矫正

理想情况下，如果某个量子比特真实处于 `|0>`，测量结果应该总是 `0`；真实处于 `|1>`，测量结果应该总是 `1`。但硬件读出会有错误：

```text
真实 0 -> 可能读成 1
真实 1 -> 可能读成 0
```

平台校准数据通常会给出每个量子比特的读出保真度：

| 符号 | 含义 |
|---|---|
| `f00` | 制备为 0 时读出为 0 的概率 |
| `f11` | 制备为 1 时读出为 1 的概率 |

`cqlib-tianyan` 会基于这些数据构建混淆矩阵，并对观测计数做近似反演。

## 2. CalibrationMode

`CalibrationMode` 有三种模式：

| 模式 | 行为 |
|---|---|
| `auto` | 默认模式。有校准数据且测量比特数不超过阈值时自动矫正，否则回退到原始计数 |
| `enabled` | 强制矫正。没有校准数据或资源不足时返回错误 |
| `disabled` | 不做矫正，始终返回原始计数 |

创建方式：

```python
from cqlib_tianyan import CalibrationMode

mode = CalibrationMode("auto")
assert mode == "auto"
```

大多数情况下可以直接传字符串：

```python
task = backend.run_with_mode(
    circuits=["H Q1\nM Q1"],
    shots=1000,
    mode="disabled",
)
```

## 3. 默认行为

`backend.run()` 默认使用 `auto` 模式：

```python
task = backend.run(["H Q1\nM Q1"], shots=1000)
results = task.wait(timeout_secs=120.0)
```

`auto` 模式会在满足条件时自动应用读取误差矫正。

## 4. 获取原始结果

如果只想获取原始计数，不希望做任何矫正，可以使用 `run_raw`：

```python
task = backend.run_raw(["H Q1\nM Q1"], shots=1000)
raw_results = task.wait(timeout_secs=120.0)
```

或者在已有任务上使用 `wait_raw`：

```python
task = backend.run(["H Q1\nM Q1"], shots=1000)
raw_results = task.wait_raw(timeout_secs=120.0)
```

## 5. 显式指定矫正模式

```python
# 自动矫正
task = backend.run_with_mode(["H Q1\nM Q1"], shots=1000, mode="auto")

# 强制矫正
task = backend.run_with_mode(["H Q1\nM Q1"], shots=1000, mode="enabled")

# 禁用矫正
task = backend.run_with_mode(["H Q1\nM Q1"], shots=1000, mode="disabled")
```

也可以传 `CalibrationMode` 对象：

```python
mode = CalibrationMode("disabled")
task = backend.run_with_mode(["H Q1\nM Q1"], shots=1000, mode=mode)
```

## 6. auto 模式的比特数阈值

读取误差矫正需要构建混淆矩阵。若测量比特数为 `n`，完整矩阵规模是：

```text
2^n x 2^n
```

内存复杂度接近：

```text
O(4^n)
```

因此 `auto` 模式默认只在测量比特数不超过 14 时自动矫正。超过阈值时会回退到原始计数，避免内存开销过大。

如果确实需要强制矫正，可以使用：

```python
task = backend.run_with_mode(circuits, shots=1000, mode="enabled")
```

但这要求调用方自己确认内存资源足够。

## 7. 对比矫正前后结果

```python
qcis = "H Q1\nM Q1"

task = backend.run([qcis], shots=1000)

calibrated = task.wait(timeout_secs=120.0)
raw = task.wait_raw(timeout_secs=120.0)

print("矫正后:", calibrated[0].counts, calibrated[0].probabilities)
print("原始值:", raw[0].counts, raw[0].probabilities)
```

注意：矫正后的 counts 仍会以 `ExecutionResult` 的 `counts` 字段返回，`probabilities` 是基于 counts 归一化得到的概率。

## 8. 与设备配置的关系

读取误差矫正依赖设备校准数据。可以先查看设备配置：

```python
device = backend.device_config()
```

如果没有可用校准数据：

- `auto` 会回退到原始计数。
- `enabled` 会报错。
- `disabled` 不受影响。

## 9. 选择建议

| 场景 | 推荐模式 |
|---|---|
| 普通实验 | `auto` |
| 只想看硬件原始输出 | `disabled` 或 `wait_raw()` |
| 做误差矫正算法验证 | `enabled` |
| 大量测量比特 | `disabled`，或自行评估内存后使用 `enabled` |
| 对比硬件与矫正效果 | 同时使用 `wait()` 和 `wait_raw()` |

## 10. 常见问题

| 现象 | 原因 | 处理方式 |
|---|---|---|
| `auto` 没有明显改变结果 | 无校准数据、测量比特数超过阈值，或读出误差本身较小 | 使用 `enabled` 验证，或查看设备校准数据 |
| `enabled` 报错 | 缺少校准数据或资源不足 | 改用 `auto` 或 `disabled` |
| 结果中出现很小概率项 | 逆矩阵矫正会重新分配概率 | 根据实验需求设置后处理阈值 |
| 大比特数任务很慢 | 矫正矩阵规模随比特数指数增长 | 禁用矫正或拆分实验 |

## 下一步

- [Cqlib 教程总览](../../README.md)：回到教程目录，继续查看其他 Cqlib 模块。
- [QCIS 格式说明](../1_ir/1_qcis.md)：了解 QCIS 文本格式、导入导出接口和支持边界。
- [设备模块](../2_device/0_overview.md)：了解设备拓扑、设备配置、噪声模型和执行结果对象。
