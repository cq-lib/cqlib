# Resource

`cqlib.compile.resource` 提供 ancilla 辅助比特的申请、规划、租借与回收管理。

干净资源与脏资源是恢复契约，不是态模拟结果。干净资源进出变换时必须处于 `|0>`；脏资源可能处于未知或纠缠态，变换释放租约前必须完整还原其输入状态。

## 导入

```python
from cqlib.compile.resource import (
    AncillaRequirement,
    ResourcePolicy,
    ResourceLimits,
    ResourceRequest,
    ResourcePlan,
    ResourceLease,
    ResourceManager,
    ResourceError,
    ResourceUnavailableError,
)
```

---

## 资源契约对象

### AncillaRequirement

资源种类：

- `AncillaRequirement.clean_zero()`：干净 ancilla，进出变换时处于 `|0>`。
- `AncillaRequirement.dirty()`：脏 ancilla，释放前必须完整还原原状态。

支持 `==`、`hash`、`copy`、`deepcopy`。

### ResourceRequest(requirement, count, *, excluded=None)

资源申请：

- `requirement` (`AncillaRequirement`)：资源种类。
- `count` (`int`)：申请数量。
- `excluded` (`list[Qubit] | None`)：不可使用的比特。

属性：

- `requirement`、`count`、`excluded`，与构造参数一一对应。

### ResourcePlan

申请预览结果，由 `ResourceManager.preview()` 返回，不实际占用资源。

属性：

- `qubits -> list[Qubit]`：规划到的比特。
- `requirement -> AncillaRequirement`：资源种类。
- `num_new_qubits -> int`：规划引入的新比特数。

### ResourceLease

已生效的租约，由 `ResourceManager.commit()` 返回。

属性：

- `id -> int`：租约 ID。
- `qubits -> list[Qubit]`：租借的比特。
- `requirement -> AncillaRequirement`：资源种类。

---

## ResourcePolicy / ResourceLimits

### ResourcePolicy(*, max_pre_layout_clean_ancillas=0, allow_dirty_borrowing=false)

预布局分解阶段的辅助比特权限：

- `max_pre_layout_clean_ancillas` (`int`)：允许分配的逻辑干净 ancilla 数量上限。
- `allow_dirty_borrowing` (`bool`)：是否允许借用脏输入比特。

设备目标会根据可用物理比特推导硬容量，不受此策略限制。

### ResourceLimits(*, max_total_qubits=None)

资源上限：

- `max_total_qubits` (`int | None`)：总比特数上限。

---

## ResourceManager

资源管理器，把无副作用的规划与实际变更分离。

### ResourceManager.from_circuit(circuit, *, policy=None, limits=None)

静态方法，从线路构造管理器。管理器须与线路保持同步演进。

参数：

- `circuit` (`Circuit`)：管理器跟踪的线路。
- `policy` (`ResourcePolicy | None`)：默认 `ResourcePolicy()`。
- `limits` (`ResourceLimits | None`)：资源上限。

### 方法

- `preview(request) -> ResourcePlan`：预览申请。不占用资源、不改变线路。
- `commit(circuit, plan) -> ResourceLease`：按规划实际占用资源并修改线路。
- `release(lease)`：释放租约。
- `enter_post_layout(circuit)`：进入布局后阶段。
- `verify_consistency(circuit)`：校验管理器与线路一致。
- `verify_idle(circuit)`：校验所有租约已释放。

### 用法约定

消耗资源的变换在租约生效期间使用 `lease.qubits`，并须在释放前完成恢复（干净 ancilla 回到 `|0>`，脏 ancilla 还原原状态）。普通编译只需给 `compile()` 传一个 `ResourcePolicy`，管理器生命周期由编译器内部维护。

示例：

```python
from cqlib import Circuit
from cqlib.compile.resource import (
    AncillaRequirement,
    ResourceManager,
    ResourcePolicy,
    ResourceRequest,
)

circuit = Circuit(2)
manager = ResourceManager.from_circuit(
    circuit,
    policy=ResourcePolicy(max_pre_layout_clean_ancillas=1),
)

request = ResourceRequest(AncillaRequirement.clean_zero(), 1)

# 预览不占用资源、不改变线路
plan = manager.preview(request)
lease = manager.commit(circuit, plan)

# 消耗资源的变换在此使用 lease.qubits，之后须恢复 |0>
manager.release(lease)
manager.verify_idle(circuit)
```

---

## 异常情况

- `ResourceError`：资源管理错误基类。
- `ResourceUnavailableError`：资源不足或租约状态非法。
