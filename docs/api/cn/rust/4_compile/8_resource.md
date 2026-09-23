# Resource

`cqlib_core::compile::resource` 提供 ancilla 辅助比特的申请、规划、租借与回收管理。

干净资源与脏资源是恢复契约，不是态模拟结果。干净资源进出变换时必须处于 `|0>`；脏资源可能处于未知或纠缠态，变换释放租约前必须完整还原其输入状态。

## 导入

```rust
use cqlib_core::compile::resource::{
    AncillaRequirement, ResourceError, ResourceLease, ResourceLimits, ResourceManager, ResourcePlan,
    ResourcePolicy, ResourceRequest,
};
```

---

## 资源契约对象

### AncillaRequirement

```rust
pub enum AncillaRequirement {
    CleanZero,  // 进出变换时处于 |0>
    Dirty,      // 可进入未知态，必须精确还原
}
```

### ResourceRequest

```rust
pub struct ResourceRequest {
    pub requirement: AncillaRequirement,
    pub count: usize,
    pub excluded: BTreeSet<Qubit>,
}
```

- `requirement`：消耗算法要求的状态恢复契约。
- `count`：算法要求的辅助比特数量。
- `excluded`：算法不得消耗为辅助资源的比特，通常包含数据、控制与目标比特。每个排除比特必须已在管理器中注册。

申请不占用资源，仅供 `ResourceManager::preview` 规划。

### ResourcePlan

由 `preview` 产生的无副作用规划快照。可检查、可比较，但不保留比特、不修改线路；必须把选中的规划传给 `commit` 才能使用其中的比特。规划是管理器专属快照，任何成功的资源池变更都会使旧规划失效。

字段私有，通过方法读取：

- `qubits(&self) -> &[Qubit]`：规划到的比特。
- `num_new_qubits(&self) -> usize`：规划引入的新比特数。

### ResourceLease

由 `commit` 产生的已生效租约。

- `qubits(&self) -> &[Qubit]`：租借的比特。

---

## ResourcePolicy / ResourceLimits

```rust
pub struct ResourcePolicy {
    pub max_pre_layout_clean_ancillas: usize,
    pub allow_dirty_borrowing: bool,
}
```

- `max_pre_layout_clean_ancillas`：布局前编译器可创建的逻辑干净 ancilla 总数。已释放比特仍计入总数，可复用。
- `allow_dirty_borrowing`：是否允许在脏资源契约下借用输入比特。只允许消耗算法接受未知输入态并精确还原的情况；永远不会让输入比特满足 clean-zero 申请。

实现 `Default`（不创建干净 ancilla、不借用输入比特）。

```rust
pub struct ResourceLimits {
    pub max_total_qubits: Option<usize>,
}
```

- `max_total_qubits`：完整逻辑线路的最大总比特数上限（含输入比特与编译器分配的干净 ancilla）；`None` 表示不强制总比特上限。

实现 `Default`、`Copy`。

---

## ResourceManager

资源管理器，把无副作用的规划与实际变更分离。

- `ResourceManager::from_circuit(circuit: &Circuit, policy: ResourcePolicy, limits: ResourceLimits) -> Result<Self, ResourceError>`：从线路构造管理器。管理器须与线路保持同步演进。
- `preview(&self, request: &ResourceRequest) -> Result<ResourcePlan, ResourceError>`：预览申请，不占用资源、不改变线路。
- `commit(&mut self, circuit: &mut Circuit, plan: ResourcePlan) -> Result<ResourceLease, ResourceError>`：按规划实际占用资源并修改线路。`plan` 按值传入。
- `release(&mut self, lease: &ResourceLease) -> Result<(), ResourceError>`：释放租约。
- `enter_post_layout(&mut self, circuit: &Circuit) -> Result<(), ResourceError>`：进入布局后阶段。
- `verify_consistency(&self, circuit: &Circuit) -> Result<(), ResourceError>`：校验管理器与线路一致。
- `verify_idle(&self, circuit: &Circuit) -> Result<(), ResourceError>`：校验所有租约已释放。

### 用法约定

消耗资源的变换在租约生效期间使用 `lease.qubits()`，并须在释放前完成恢复（干净 ancilla 回到 `|0>`，脏 ancilla 还原原状态）。普通编译只需在 `CompileConfig` 里给出 `ResourcePolicy`，管理器生命周期由编译器内部维护。

---

## 示例

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::compile::resource::{
    AncillaRequirement, ResourceLimits, ResourceManager, ResourcePolicy, ResourceRequest,
};

let mut circuit = Circuit::new(2);
let mut manager = ResourceManager::from_circuit(
    &circuit,
    ResourcePolicy { max_pre_layout_clean_ancillas: 1, allow_dirty_borrowing: false },
    ResourceLimits::default(),
)
.unwrap();

let request = ResourceRequest {
    requirement: AncillaRequirement::CleanZero,
    count: 1,
    excluded: Default::default(),
};

// 预览不占用资源、不改变线路
let plan = manager.preview(&request).unwrap();
let lease = manager.commit(&mut circuit, plan).unwrap();

// 消耗资源的变换在此使用 lease.qubits()，之后须恢复 |0>
manager.release(&lease).unwrap();
manager.verify_idle(&circuit).unwrap();
```

---

## 错误

`ResourceError` 是资源管理的统一错误类型，资源不足、租约状态非法、管理器与线路不一致等场景均通过它返回。
