# 执行结果与状态

cqlib.device 的执行结果模块提供测量结果封装、任务状态追踪与概率计算。

---

## Outcome：测量结果

Outcome 使用小端序（little-endian）比特顺序：字符串最右侧对应 Qubit 0，最左侧对应 Qubit N-1。

```python
from cqlib.device import Outcome

# 从比特字符串创建（小端序：最右 = Qubit 0）
o = Outcome("101")
print("比特 0 是否为 1:", o.is_one(0))   # True
print("比特 1 是否为 1:", o.is_one(1))   # False
print("完整比特串 (3):", o.to_bitstring(3))  # "101"

# 从比特字符串构造（与直接构造等价）
o2 = Outcome.from_bitstring("101")
print("o == o2:", o == o2)  # True

# 从索引列表构造（指定哪些位置的比特为 1）
o3 = Outcome.from_indices(width=3, indices=[0, 2])
print("索引构造:", o3.to_bitstring(3))  # "101"
```

**说明**：
- 字符串只能包含 '0' 和 '1'，包含其他字符会抛出 ValueError
- 	o_bitstring(num_qubits) 如果 
um_qubits 大于实际宽度，高位补零

---

## Status：任务状态

Status 表示量子任务的执行阶段，支持以下五种状态：

| 状态 | 构造方法 | 是否为终态 |
|---|---|---|
| Queued | Status.queued() | 否 |
| Running | Status.running() | 否 |
| Completed | Status.completed() | 是 |
| Failed | Status.failed(msg, code) | 是 |
| Cancelled | Status.cancelled() | 是 |

```python
from cqlib.device import Status

q = Status.queued()
c = Status.completed()
f = Status.failed("backend down", 500)
x = Status.cancelled()

print("queued:", q.kind, "终态?", q.is_terminal())
print("completed:", c.kind, "成功?", c.is_success())
print("failed:", f.kind, f.error_msg, f.error_code)
print("cancelled:", x.kind, "终态?", x.is_terminal())
```

**说明**：
- Status.kind 是属性（不是方法），返回字符串，如 "completed"、"failed"
- is_terminal() 对 completed、ailed、cancelled 返回 True
- is_success() 仅对 completed 返回 True
- error_msg 和 error_code 仅在 kind == "failed" 时有值，否则返回 None

---

## ExecutionResult：完整执行结果

```python
from cqlib.device import ExecutionResult

result = ExecutionResult("q-task-001", [0, 1], 1000, 2, "Tianyan-176-2")
print("创建时状态:", result.status.kind)  # queued

# 标记为运行中
result.start()
print("启动后状态:", result.status.kind)  # running

# 完成并填入测量计数
result.finish({"00": 600, "11": 400})
result.calc_probabilities()  # 计算概率分布

print("任务 ID:", result.task_id)
print("测量次数:", result.shots)
print("计数结果:", result.counts)
print("概率分布:", result.probabilities)
print("后端名称:", result.backend)
```

### 从计数直接构造

如果已有测量结果，可使用 rom_counts 一步完成创建和填充：

```python
from cqlib.device import ExecutionResult

result = ExecutionResult.from_counts(
    task_id="q-task-002", qubits=[0, 1],
    shots=1024, num_qubits=2,
    counts={"00": 512, "11": 512},
    backend="simulator",
)
print("状态:", result.status.kind)        # completed（已自动完成）
print("概率分布:", result.probabilities)  # {"00": 0.5, "11": 0.5}
```

---

## 异常流程处理

```python
from cqlib.device import ExecutionResult

# 失败场景
f = ExecutionResult("task-fail", [0], 10, 1, None)
f.fail("timeout", 408)
print("失败状态:", f.status.kind)              # failed
print("错误消息:", f.status.error_msg)          # timeout
print("错误码:", f.status.error_code)           # 408

# 取消场景
c = ExecutionResult("task-cancel", [0], 10, 1, None)
c.cancel()
print("取消状态:", c.status.kind)               # cancelled
```

---

## 输入校验

```python
from cqlib.device import ExecutionResult

r = ExecutionResult("bad", [0], 10, 1, None)
try:
    r.finish({"2": 1})  # "2" 不是有效的二进制字符串
except ValueError as e:
    print("无效计数:", e)
```

---

## 下一步

- [量子信息](../3_qis/0_overview.md)：掌握 Statevector、DensityMatrix、Pauli 等基础
- [编译优化](../4_compiler/0_overview.md)：了解编译管线的布局、路由与优化
- [可视化](../5_visualization/0_overview.md)：学习电路绘制与结果可视化
