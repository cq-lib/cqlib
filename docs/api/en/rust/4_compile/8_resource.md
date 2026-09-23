# Resource

`cqlib_core::compile::resource` provides request, planning, leasing and recycling management for ancilla qubits.

Clean resources and dirty resources are restoration contracts, not state simulation results. A clean resource must be in `|0>` on entry to and exit from a transform; a dirty resource may be in an unknown or entangled state, and the transform must fully restore its input state before releasing the lease.

## Import

```rust
use cqlib_core::compile::resource::{
    AncillaRequirement, ResourceError, ResourceLease, ResourceLimits, ResourceManager, ResourcePlan,
    ResourcePolicy, ResourceRequest,
};
```

---

## Resource contract objects

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

- `requirement`: the state restoration contract required by the consuming algorithm.
- `count`: the number of ancilla qubits required by the algorithm.
- `excluded`: qubits that the algorithm must not consume as ancilla resources, typically including data, control and target qubits. Every excluded qubit must already be registered in the manager.

A request does not occupy resources; it is used only for planning by `ResourceManager::preview`.

### ResourcePlan

A side-effect-free planning snapshot produced by `preview`. It can be inspected and compared, but it does not reserve qubits and does not modify the circuit; the selected plan must be passed to `commit` before the qubits in it can be used. A plan is a manager-specific snapshot, and any successful resource pool change invalidates old plans.

The fields are private and read through methods:

- `qubits(&self) -> &[Qubit]`: the planned qubits.
- `num_new_qubits(&self) -> usize`: the number of new qubits introduced by the plan.

### ResourceLease

An effective lease produced by `commit`.

- `qubits(&self) -> &[Qubit]`: the leased qubits.

---

## ResourcePolicy / ResourceLimits

```rust
pub struct ResourcePolicy {
    pub max_pre_layout_clean_ancillas: usize,
    pub allow_dirty_borrowing: bool,
}
```

- `max_pre_layout_clean_ancillas`: the total number of logical clean ancillas the compiler may create before layout. Released qubits still count toward the total and can be reused.
- `allow_dirty_borrowing`: whether input qubits may be borrowed under the dirty resource contract. It is only allowed when the consuming algorithm accepts an unknown input state and restores it exactly; it never lets an input qubit satisfy a clean-zero request.

Implements `Default` (creates no clean ancillas and borrows no input qubits).

```rust
pub struct ResourceLimits {
    pub max_total_qubits: Option<usize>,
}
```

- `max_total_qubits`: the upper limit on the total number of qubits of the complete logical circuit (including input qubits and compiler-allocated clean ancillas); `None` means no total qubit limit is enforced.

Implements `Default` and `Copy`.

---

## ResourceManager

The resource manager, which separates side-effect-free planning from actual changes.

- `ResourceManager::from_circuit(circuit: &Circuit, policy: ResourcePolicy, limits: ResourceLimits) -> Result<Self, ResourceError>`: construct a manager from a circuit. The manager must evolve in sync with the circuit.
- `preview(&self, request: &ResourceRequest) -> Result<ResourcePlan, ResourceError>`: preview a request without occupying resources or changing the circuit.
- `commit(&mut self, circuit: &mut Circuit, plan: ResourcePlan) -> Result<ResourceLease, ResourceError>`: actually occupy resources according to the plan and modify the circuit. `plan` is passed by value.
- `release(&mut self, lease: &ResourceLease) -> Result<(), ResourceError>`: release a lease.
- `enter_post_layout(&mut self, circuit: &Circuit) -> Result<(), ResourceError>`: enter the post-layout phase.
- `verify_consistency(&self, circuit: &Circuit) -> Result<(), ResourceError>`: validate that the manager and the circuit are consistent.
- `verify_idle(&self, circuit: &Circuit) -> Result<(), ResourceError>`: validate that all leases have been released.

### Usage conventions

A resource-consuming transform uses `lease.qubits()` while the lease is in effect and must complete restoration before release (a clean ancilla returns to `|0>`, a dirty ancilla restores its original state). Ordinary compilation only needs the `ResourcePolicy` given in `CompileConfig`; the manager lifecycle is maintained internally by the compiler.

---

## Example

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

## Errors

`ResourceError` is the unified error type of resource management; insufficient resources, invalid lease state and manager-circuit inconsistency are all returned through it.
