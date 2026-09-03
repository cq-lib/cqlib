# 任务提交与结果获取

天衍平台执行线路的基本单位是任务。`cqlib-tianyan` 中，任务提交后会返回 `TaskHandle`，用户可以通过它查询任务状态或阻塞等待结果。

## 1. 提交 QCIS 线路

Python 绑定中的任务提交接口接收 QCIS 字符串列表：

```python
qcis = "H Q1\nM Q1"
task = backend.run([qcis], shots=1000)
```

参数说明：

| 参数 | 说明 |
|---|---|
| `circuits` | QCIS 字符串列表 |
| `shots` | 每条线路的采样次数 |

返回值是 `TaskHandle`。

## 2. 通过后端提交

```python
import os
from cqlib_tianyan import TianyanPlatform

platform = TianyanPlatform.login(os.environ["TIANYAN_API_KEY"])
backend = platform.get_backend("tianyan-287")

circuits = [
    "H Q1\nM Q1",
    "X Q1\nM Q1",
]

task = backend.run(circuits, shots=1000)
print(task.task_ids)
```

## 3. 通过平台直接提交

如果已经知道目标设备名，也可以跳过 `get_backend`：

```python
task = platform.submit(
    circuits=["H Q1\nM Q1"],
    shots=1000,
    device_name="tianyan-287",
)
```

这种方式适合服务端或自动化脚本，因为它减少一次后端列表查询。

## 4. TaskHandle 字段

| 字段 | 说明 |
|---|---|
| `task_ids` | 平台返回的任务 ID 列表 |
| `device_name` | 提交使用的后端名称 |
| `shots` | 每条线路请求的 shots |
| `submitted_at` | 提交时间，ISO 8601 字符串 |

示例：

```python
print(task.task_ids)
print(task.device_name)
print(task.shots)
print(task.submitted_at)
```

## 5. 非阻塞查询

`status()` 只查询一次，返回已经完成的结果。尚未完成的线路不会出现在返回列表中。

```python
partial_results = task.status()

print(f"已完成 {len(partial_results)} / {len(task.task_ids)}")
for result in partial_results:
    print(result.task_id, result.counts)
```

适合自己实现轮询逻辑，或者在 UI 中定期刷新任务状态。

## 6. 阻塞等待

`wait()` 会轮询直到所有线路完成，或直到超时。

```python
results = task.wait(
    timeout_secs=120.0,
    poll_interval_secs=5.0,
)

for result in results:
    print(result.task_id)
    print(result.counts)
    print(result.probabilities)
```

参数说明：

| 参数 | 说明 |
|---|---|
| `timeout_secs` | 最大等待秒数 |
| `poll_interval_secs` | 轮询间隔秒数，默认 `5.0` |

`wait()` 在阻塞等待时会释放 Python GIL，因此不会阻塞其他 Python 线程运行。

## 7. 获取原始结果

默认 `wait()` 会按照提交时的 `CalibrationMode` 决定是否应用读取误差矫正。若只想拿原始计数，使用：

```python
raw_results = task.wait_raw(
    timeout_secs=120.0,
    poll_interval_secs=5.0,
)
```

## 8. 结果对象

`wait()`、`wait_raw()` 和 `status()` 返回的是 `cqlib.device.ExecutionResult` 列表。

常用字段：

| 字段 | 说明 |
|---|---|
| `task_id` | 平台任务 ID |
| `qubits` | 被测量的量子比特 |
| `shots` | 实际 shots 数 |
| `num_qubits` | 结果对象中的量子比特数 |
| `backend` | 后端名称 |
| `counts` | 测量计数字典 |
| `probabilities` | 按 counts 归一化后的概率 |
| `status` | 执行状态 |

示例：

```python
result = results[0]

print(result.task_id)
print(result.backend)
print(result.qubits)
print(result.counts)
print(result.probabilities)
```

## 9. 批量提交

天衍平台单次请求最多接受 50 条线路。`cqlib-tianyan` 会在内部自动拆分更大的输入：

```python
circuits = ["H Q1\nM Q1"] * 120

task = backend.run(circuits, shots=1000)
print(len(task.task_ids))  # 120
```

内部会按 `50 + 50 + 20` 分批提交，并把所有任务 ID 合并到同一个 `TaskHandle` 中。

## 10. 提交前检查

建议提交前检查以下内容：

- 后端是否 `running`。
- QCIS 文本是否能被 `cqlib.ir.qcis.loads` 正确解析。
- 线路中是否包含目标设备不支持的门。
- 是否已经添加测量。
- shots 是否符合实验需求和平台限制。
- 批量任务是否有必要先小规模试跑。

## 11. 常见问题

| 现象 | 常见原因 | 处理方式 |
|---|---|---|
| 提交失败 | 后端不可用、QCIS 格式错误、API Key 无效或网络问题 | 检查 `backend.status`、QCIS 文本和认证状态 |
| `wait` 超时 | 队列等待时间长或任务未完成 | 增大 `timeout_secs`，或改用 `status()` 分批查询 |
| 返回结果少于提交线路数 | 使用了 `status()`，它只返回已完成结果 | 使用 `wait()` 等待全部完成 |
| counts 为空 | 平台结果尚未完成或解析失败 | 检查任务状态和原始错误信息 |
| 概率和 counts 不一致 | 概率由 counts 归一化得到，矫正模式可能改变 counts | 对比 `wait()` 和 `wait_raw()` |

## 下一步

- [QCIS 与 IR 联动](4_qcis_ir_workflow.md)：把 Cqlib `Circuit` 导出为 QCIS，并接入天衍任务提交流程。
- [读取误差矫正](5_readout_mitigation.md)：了解 `CalibrationMode`、原始结果和矫正结果的区别。
