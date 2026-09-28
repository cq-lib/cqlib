# Resource

`cqlib.compile.resource` provides the request, planning, borrowing and reclamation management of ancilla qubits.

Clean resources and dirty resources are restoration contracts, not state simulation results. Clean resources must be in `|0>` when entering and leaving the transform; dirty resources may be in an unknown or entangled state, and their input state must be fully restored before the transform releases the lease.

## Import

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

## Resource contract objects

### AncillaRequirement

Resource kinds:

- `AncillaRequirement.clean_zero()`: clean ancilla, in `|0>` when entering and leaving the transform.
- `AncillaRequirement.dirty()`: dirty ancilla, whose original state must be fully restored before release.

Supports `==`, `hash`, `copy` and `deepcopy`.

### ResourceRequest(requirement, count, *, excluded=None)

Resource request:

- `requirement` (`AncillaRequirement`): the resource kind.
- `count` (`int`): the requested count.
- `excluded` (`list[Qubit] | None`): the qubits that cannot be used.

Attributes:

- `requirement`, `count`, `excluded`, corresponding one-to-one with the constructor parameters.

### ResourcePlan

Request preview result, returned by `ResourceManager.preview()`; it does not actually occupy resources.

Attributes:

- `qubits -> list[Qubit]`: the planned qubits.
- `requirement -> AncillaRequirement`: the resource kind.
- `num_new_qubits -> int`: the number of new qubits the plan introduces.

### ResourceLease

An effective lease, returned by `ResourceManager.commit()`.

Attributes:

- `id -> int`: the lease ID.
- `qubits -> list[Qubit]`: the borrowed qubits.
- `requirement -> AncillaRequirement`: the resource kind.

---

## ResourcePolicy / ResourceLimits

### ResourcePolicy(*, max_pre_layout_clean_ancillas=0, allow_dirty_borrowing=false)

Ancilla permissions of the pre-layout decomposition stage:

- `max_pre_layout_clean_ancillas` (`int`): the upper limit on the logical clean ancillas allowed to be allocated.
- `allow_dirty_borrowing` (`bool`): whether borrowing dirty input qubits is allowed.

A device target derives a hard capacity from the available physical qubits and is not restricted by this policy.

### ResourceLimits(*, max_total_qubits=None)

Resource limits:

- `max_total_qubits` (`int | None`): the upper limit on the total number of qubits.

---

## ResourceManager

Resource manager, separating side-effect-free planning from actual modification.

### ResourceManager.from_circuit(circuit, *, policy=None, limits=None)

Static method that constructs a manager from a circuit. The manager must evolve in sync with the circuit.

Parameters:

- `circuit` (`Circuit`): the circuit tracked by the manager.
- `policy` (`ResourcePolicy | None`): defaults to `ResourcePolicy()`.
- `limits` (`ResourceLimits | None`): the resource limits.

### Methods

- `preview(request) -> ResourcePlan`: preview a request. Does not occupy resources and does not modify the circuit.
- `commit(circuit, plan) -> ResourceLease`: actually occupy resources according to the plan and modify the circuit.
- `release(lease)`: release a lease.
- `enter_post_layout(circuit)`: enter the post-layout stage.
- `verify_consistency(circuit)`: verify that the manager and the circuit are consistent.
- `verify_idle(circuit)`: verify that all leases have been released.

### Usage conventions

A transform that consumes resources uses `lease.qubits` while the lease is effective and must complete restoration before release (a clean ancilla returns to `|0>`, a dirty ancilla has its original state restored). Ordinary compilation only needs to pass a `ResourcePolicy` to `compile()`; the manager lifecycle is maintained internally by the compiler.

Example:

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

## Raises

- `ResourceError`: the resource management error base class.
- `ResourceUnavailableError`: insufficient resources or an invalid lease state.
