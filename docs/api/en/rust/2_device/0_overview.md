# Device

`cqlib_core::device`

`cqlib_core::device` describes the static information of a quantum device and the state of task execution: the topology formed by qubits and couplings, the calibration properties of qubits and edges, the layout from logical qubits to physical qubits, the noise model, and execution results and status.

## Overview

The device module splits hardware-related data into several mutually independent groups of objects: topology describes only connectivity, properties describe only calibration data, layout describes only the mapping, and the noise model describes only channels. Compilation and execution each take what they need, without being coupled to one another.

### Topology

`Topology` consists of a physical qubit list and a coupling list and maintains a directed graph internally; `graph()` retrieves a reference to the underlying graph. Couplings are directed: a coupling on `(control, target)` does not mean the reverse direction has one too, so connectivity queries fall into two kinds, `successors` / `predecessors` (directed) and `neighbors_undirected` (undirected), and degree queries are likewise split into `out_degree` / `in_degree`. A coupling may carry a name label, used for identification only.

### Device and properties

`Device` layers device-level defaults and local overrides on top of a topology, and records the device name, registered qubits, unusable qubits, native gate set and calibration time. Most device-level configuration items have two forms: `with_*` consumes and returns `self`, for chained construction, while `set_*` modifies in place. Qubit-level properties are described by `QubitProp` (`t1`, `t2`, `frequency`, readout error and readout confusion probabilities), and edge-level properties by `EdgeProp`; both hold a set of `InstructionProp` (instruction, error rate and optional duration).

When querying the properties of a qubit, the local value takes precedence and the device-level default is used as a fallback when nothing is set: `get_t1` first looks at the `t1` of that qubit and returns `default_t1` if none is available. The device also provides factory methods such as `line`, `line_from_qubits`, `bidirectional_line`, `ring`, `star`, `grid` and `from_edges`, which directly generate a topology and a qubit set in common shapes.

### Native instruction capability

The device-level `native_gates` is the global default capability; if a qubit or a directed edge has configured its own instruction list through `with_native_instruction` / `set_native_instruction`, that list is a complete override of the capability there and does not write back to the default. Calibration data only attaches error rates and durations to supported instructions; it does not turn an unsupported instruction into a supported one. Native capability is validated before being written, and a rejected update leaves the original capability unchanged. `validate_operation`, `validate_value_operation` and `validate_circuit` use the same set of rules to check whether a circuit can be executed on the device.

### Layout

`Layout` maintains the mapping from logical qubits to physical qubits; a physical qubit that carries no logical qubit is a vacant physical qubit. The mapping can be read as a whole through `l2p_map` / `p2l_map` or queried for a single qubit; `swap_physical` swaps the logical qubits carried at two physical positions, which is the basic operation for moving logical qubits during routing.

### Noise model

`NoiseModel` registers noise channels by qubit and operation: readout error is represented by `ReadoutError`, single-qubit channels are given by variants of `SingleQubitNoise` (bit flip, phase flip, Pauli, depolarizing, amplitude damping, phase damping), and two-qubit channels are given by `TwoQubitNoise` (depolarizing, independent channel combination, correlated Pauli). A channel can be converted into Kraus operators with `to_kraus`, for use in density matrix simulation. Queries specify the gate and qubit combination with `OperationKey`.

### Execution result

`ExecutionResult` records the lifecycle of one task: after construction with `new` it is in `Status::Queued`, after `start` it enters the running state, `finish` / `fail` / `cancel` end the task, and `from_counts` can construct an already completed result directly from measurement counts. A measurement result is represented by `Outcome`, which packs the bitstring into 64-bit blocks and supports both position-wise bit access and restoration of the bitstring; `counts` and `probabilities` are the measurement count and the normalized probability of each outcome respectively.

---

## Common entry points

```rust
use std::collections::HashSet;

use cqlib_core::device::{Device, PhysicalQubit, QubitProp, Topology};

let q0 = PhysicalQubit::new(0);
let q1 = PhysicalQubit::new(1);
let topology = Topology::new(vec![q0, q1], vec![(q0, q1, "CX".to_string())]).unwrap();

let mut device = Device::new("demo".to_string(), HashSet::from_iter([q0, q1]), topology)
    .unwrap()
    .with_default_t1(40.0);

device.add_qubit_properties(q0, QubitProp::new(0.02).with_t1(60.0)).unwrap();

assert_eq!(device.get_t1(q0), Some(60.0)); // 局部属性
assert_eq!(device.get_t1(q1), Some(40.0)); // 回退到设备级默认值
```

---

## Core concepts and terms

| Term | Description |
| --- | --- |
| **Physical qubit** | A physical position on the device, represented by `PhysicalQubit`. |
| **Logical qubit** | A wire in a circuit, represented by `LogicalQubit`; even with the same index it is distinguished from a physical qubit. |
| **Topology** | The directed graph formed by physical qubits and the couplings between them, represented by `Topology`. |
| **Coupling** | A connection between two physical qubits over which a two-qubit gate can be executed. |
| **Directed coupling** | A coupling with a direction: a coupling on `(control, target)` does not imply that the reverse direction has one too. |
| **Native instruction** | An instruction that can be executed directly on the device; the device-level set is the default capability, and qubits and edges can each override it. |
| **Local override** | The native instruction list of a qubit or edge; once non-empty it completely replaces the device-level default capability and does not write back to the default. |
| **Default fallback** | When querying a qubit property, the device-level default is returned if that qubit has no local value. |
| **Initial layout** | The initial mapping from logical qubits to physical qubits, represented by `Layout`. |
| **Vacant physical qubit** | A physical qubit that currently carries no logical qubit. |
| **Noise channel** | A set of operators describing one kind of noise action, convertible into Kraus operators with `to_kraus`. |
| **Readout error** | The probability of reading `\|0>` as 1 or `\|1>` as 0 during measurement, given by the two fields `p_0_given_1` and `p_1_given_0` of `ReadoutError`. |
| **Measurement outcome** | The bitstring obtained from one sampling, represented by `Outcome`. |

---

## `cqlib_core::device` API overview

### Topology

| Name | Description |
| --- | --- |
| [`Topology`](1_topology.md) | A directed graph formed by physical qubits and a coupling list, with add, remove and connectivity queries. |
| [`Topology::line`](1_topology.md) | A constructor that builds a topology in a line shape. |

### Device and properties

| Name | Description |
| --- | --- |
| [`Device`](2_properties_device.md) | The device object: topology, qubit set, native gate set, calibration time and default properties. |
| [`Device::line`](2_properties_device.md) / [`Device::ring`](2_properties_device.md) / [`Device::star`](2_properties_device.md) / [`Device::grid`](2_properties_device.md) | Factory methods that build a device in common shapes. |
| [`QubitProp`](2_properties_device.md) | Qubit properties: readout error, confusion probability, `t1` / `t2`, frequency and native instruction properties. |
| [`EdgeProp`](2_properties_device.md) | Edge properties: the native instruction properties on that directed edge. |
| [`InstructionProp`](2_properties_device.md) | Instruction properties: the instruction object, error rate and optional duration. |
| [`Device::validate_circuit`](2_properties_device.md) / [`Device::validate_operation`](2_properties_device.md) | Validate a circuit or a single operation against the native capability and the topology. |

### Layout

| Name | Description |
| --- | --- |
| [`Layout`](3_layout.md) | The mapping from logical qubits to physical qubits, with binding, unbinding and physical qubit swap. |
| [`Layout::from_pairs`](3_layout.md) | Build a layout from `(logical index, physical index)` pairs and the total number of physical qubits. |
| [`LogicalQubit`](3_layout.md) / [`PhysicalQubit`](3_layout.md) | The logical qubit and physical qubit types, whose `Display` forms are `L0` and `P11` respectively. |

### Noise model

| Name | Description |
| --- | --- |
| [`NoiseModel`](4_noise.md) | A container that registers noise channels by qubit and operation. |
| [`SingleQubitNoise`](4_noise.md) | The single-qubit noise channel enum, with bit flip, phase flip, Pauli, depolarizing and two kinds of damping. |
| [`TwoQubitNoise`](4_noise.md) | The two-qubit noise channel enum, with depolarizing, independent combination and correlated Pauli. |
| [`ReadoutError`](4_noise.md) | Readout error, recording `P(0\|1)` and `P(1\|0)` separately. |
| [`OperationKey`](4_noise.md) | The key for noise queries, formed by combining a gate and qubit indices. |

### Execution result

| Name | Description |
| --- | --- |
| [`ExecutionResult`](5_result.md) | The result of a task execution, with counts, probabilities and timestamps. |
| [`Status`](5_result.md) | Task status: `Queued`, `Running`, `Completed`, `Failed` and `Cancelled`. |
| [`Outcome`](5_result.md) | A compact measurement result bitstring, supporting bit-wise access and restoration. |

---

## Quick examples

### 1. Build a device and configure properties

```rust
use std::collections::HashSet;

use cqlib_core::device::{Device, PhysicalQubit, QubitProp, Topology};

let device = Device::line("line", 3).unwrap();
assert_eq!(device.name(), "line");
assert_eq!(device.qubits().count(), 3);
assert_eq!(device.num_usable_qubits(), 3);
assert!(device
    .topology()
    .supports_directed_coupling(PhysicalQubit::new(0), PhysicalQubit::new(1)));
assert!(!device
    .topology()
    .supports_directed_coupling(PhysicalQubit::new(1), PhysicalQubit::new(0)));

let q0 = PhysicalQubit::new(0);
let q1 = PhysicalQubit::new(1);
let topology = Topology::new(vec![q0, q1], vec![(q0, q1, "CX".to_string())]).unwrap();
let mut device = Device::new("demo".to_string(), HashSet::from_iter([q0, q1]), topology)
    .unwrap()
    .with_default_t1(40.0)
    .with_default_t2(20.0);

device
    .add_qubit_properties(q0, QubitProp::new(0.02).with_t1(60.0).with_t2(30.0))
    .unwrap();

assert_eq!(device.get_t1(q0), Some(60.0));
assert_eq!(device.get_readout_error(q0), Some(0.02));
```

### 2. Create a layout

```rust
use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};

let logical = vec![LogicalQubit::new(0), LogicalQubit::new(1)];
let physical = vec![
    PhysicalQubit::new(100),
    PhysicalQubit::new(101),
    PhysicalQubit::new(102),
];

let layout = Layout::new(logical, physical, None).unwrap();

assert_eq!(layout.num_logical(), 2);
assert_eq!(layout.num_physical(), 3);
assert_eq!(layout.num_vacant_physical(), 1);
assert_eq!(
    layout.vacant_physical_qubits().collect::<Vec<_>>(),
    vec![PhysicalQubit::new(102)]
);
```

A layout can also be constructed directly from qubit pairs:

```rust
use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};

let layout = Layout::from_pairs(&[(2, 3), (0, 1)], 5).unwrap();

assert_eq!(layout.num_vacant_physical(), 3);
assert_eq!(layout.get_physical(LogicalQubit::new(2)), Some(PhysicalQubit::new(3)));
assert_eq!(layout.get_physical(LogicalQubit::new(1)), None);
```

### 3. Construct an execution result from measurement counts

```rust
use std::collections::HashMap;

use cqlib_core::circuit::Qubit;
use cqlib_core::device::{ExecutionResult, Outcome, Status};

let zero = Outcome::from_indices(1, []);
let one = Outcome::from_indices(1, [0]);

let mut counts = HashMap::new();
counts.insert(zero.clone(), 3);
counts.insert(one.clone(), 1);

let result = ExecutionResult::from_counts(
    "task".to_string(),
    vec![Qubit::new(0)],
    4,
    1,
    Some("backend".to_string()),
    counts,
);

assert_eq!(result.status(), &Status::Completed);
assert_eq!(result.counts().get(&zero), Some(&3));

let probabilities = result.probabilities().as_ref().unwrap();
assert_eq!(probabilities.get(&zero), Some(&0.75));
```

---

## Validation and error handling

Errors of the device module are split into several enums by domain; the module root exports `DeviceError`, `DeviceValidationError`, `TopologyError` and `LayoutError`, while `NoiseError` and `OutcomeError` live in their own submodules.

| Error | When it occurs |
| --- | --- |
| `TopologyError::{QubitNotFound, CouplingNotFound, QubitAlreadyExists, CouplingAlreadyExists, SelfCoupling, DuplicateQubitRemoval, DuplicateCouplingRemoval}` | Topology construction and modification: operating on unknown or duplicate qubits and couplings, self-coupling, duplicate removal. |
| `DeviceError::{InvalidOnlineQubit, QubitNotInDevice, QubitNotInTopology, EdgeNotInTopology, InvalidTopology}` | During device construction and property writes, a qubit or edge is not in the device or topology, or the underlying topology itself is invalid. |
| `DeviceError::{NonStandardNativeInstruction, InvalidNativeInstructionArity, InvalidNativeInstructionErrorRate, InvalidNativeInstructionDuration}` | An invalid native instruction: a non-standard gate, an arity outside the allowed range (at most two qubits at device level, exactly one qubit at qubit level, exactly two qubits at edge level), or an invalid value for the error rate or the duration. |
| `DeviceValidationError::{UnusablePhysicalQubit, MissingDirectedCoupling, UnsupportedInstruction, UndecomposedInstruction}` | A `validate_operation`, `validate_value_operation` or `validate_circuit` check failed: the qubit is unusable, a directed coupling is missing, the instruction is not in the native gate set, or the instruction has not been decomposed yet. |
| `LayoutError::{TooManyLogicalQubits, DuplicateLogicalQubit, DuplicatePhysicalQubit, InvalidLogicalQubit, InvalidPhysicalQubit, LogicalQubitAlreadyBound, PhysicalQubitAlreadyOccupied, LogicalQubitNotBound}` | Layout construction and binding operations: more logical qubits than physical qubits, duplicate or invalid indices, a target position already occupied, or unbinding a logical qubit that is not bound. |
| `NoiseError::{InvalidProbability, QubitCollision, InconsistentArity, Internal}` | A noise channel parameter outside `[0, 1]`, a repeated qubit within the same gate, or an arity inconsistent with the gate; the path is `cqlib_core::device::noise::NoiseError`. |
| `OutcomeError::InvalidCharacter` | The bitstring passed to `Outcome::from_bitstring` contains a character other than `0` / `1`; the path is `cqlib_core::device::result::OutcomeError`. |

Native capability writes at device level and at qubit/edge level are both validated before taking effect, and a rejected update leaves the original state of the object unchanged.
