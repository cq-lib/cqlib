# Device overview

cqlib.device is the data module in Cqlib used to describe the hardware capabilities, noise characteristics and execution results of a quantum backend, and it is located under the cqlib.device package.

In a quantum computing workflow, Circuit expresses "how the algorithm is done", while device expresses "what the hardware allows". It provides unified underlying support for the following key engineering aspects:

- **Physical constraint awareness**: defines the hardware topology (which qubits support two-qubit gate coupling and the direction restrictions)
- **High-fidelity modeling**: refined management of qubit coherence time (T1/T2), readout error and gate fidelity
- **Dynamic layout tracking**: maintains the mapping between logical qubits and physical qubits in real time during the compilation and routing stage
- **Noise simulation**: builds channel models usable for quantum noise simulation, with Kraus operator representation
- **Full task lifecycle management**: tracks the complete closed loop of a task from submission, queuing and running to result return

---

## Core objects at a glance

| Object | Use | Key capabilities |
|---|---|---|
| Topology | Hardware topology graph | Directed coupling modeling, dynamic addition and removal of nodes and edges, adjacency queries, the line factory method |
| Device | Complete hardware description | Calibration parameters with global defaults plus local overrides, factory methods for quick construction (line/bidirectional_line/ring/star/grid/from_edges) |
| QubitProp / EdgeProp / InstructionProp | Calibration property containers | Store the T1/T2, readout error, gate fidelity and duration of qubits and edges |
| Layout | Logical-physical mapping | Bidirectional mapping queries, SWAP routing operations, dynamic binding and unbinding |
| NoiseModel / SingleQubitNoise / TwoQubitNoise / ReadoutError | Noise channels | Bit flip, phase flip, depolarizing, amplitude/phase damping, etc., with Kraus operator export |
| Outcome / Status / ExecutionResult | Execution results | Measurement result encapsulation, task state machine (queued/running/completed/failed/cancelled), probability calculation |

---

## Quick example: from topology to result

The following example demonstrates the complete workflow of the device module, covering topology definition, device creation, calibration injection, layout mapping, noise configuration and result management.

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import (
    Device, EdgeProp, ExecutionResult, InstructionProp, Layout,
    NoiseModel, OperationKey, QubitProp, ReadoutError,
    SingleQubitNoise, Topology, TwoQubitNoise,
)

# 1) define the hardware topology: qubit list + (control, target, gate name) triples
topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])

# 2) create the device and set the global default calibration parameters
device = Device("demo_backend", [0, 1, 2], topo)
device.default_t1 = 50.0
device.default_t2 = 35.0
device.default_readout_error = 0.05
device.default_single_qubit_error = 0.001
device.default_two_qubit_error = 0.01

# 3) inject local calibration parameters (they override the global defaults)
q0_prop = QubitProp(readout_error=0.02)
q0_prop.t1 = 80.0
q0_prop.t2 = 70.0
device.add_qubit_properties(0, q0_prop)

cx_prop = InstructionProp(
    Instruction.from_standard_gate(StandardGate.CX),
    error_rate=0.015,
)
cx_prop.length = 220.0
edge_prop = EdgeProp()
edge_prop.add_native_instruction(cx_prop)
device.add_edge_properties(0, 1, edge_prop)

# 4) layout mapping: logical qubits -> physical qubits
layout = Layout.from_pairs([(0, 11), (1, 10)], physical_count=13)
layout.swap_physical(11, 12)

# 5) noise model configuration
noise = NoiseModel()
noise.add_readout_error(0, ReadoutError(0.02, 0.01))
noise.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(0.005))
noise.add_two_qubit_error(StandardGate.CX, 0, 1, TwoQubitNoise.depolarizing(0.02))

# 6) task execution result
result = ExecutionResult("task-1", [0, 1], 100, 2, "demo_backend")
result.start()
result.finish({"00": 60, "11": 40})
result.calc_probabilities()

# 7) query and verification output
print("device name:", device.name)
print("T1 of qubit 0 (local value):", device.get_t1(0))
print("T1 of qubit 2 (falls back to the global default):", device.get_t1(2))
print("number of usable qubits:", device.num_usable_qubits)
print("logical -> physical mapping:", layout.l2p_map)
print("readout error of qubit 0:", noise.get_readout_error(0))
print("task status:", result.status.kind)
print("probability distribution:", result.probabilities)

# query noise channels through OperationKey
skey = OperationKey.new_single(StandardGate.X, 0)
qubit_noises = noise.get_single_qubit_errors(skey)
if qubit_noises:
    print("number of X gate noise channels:", len(qubit_noises))
```

**Notes**:

- get_t1(), get_t2() and get_readout_error() follow the query policy of **local value first, falling back to the global default when there is no local value**
- l2p_map returns a mapping dictionary from LogicalQubit to PhysicalQubit
- probabilities is obtained by calc_probabilities() normalizing the raw counts, and the result is dict[str, float]
- All NoiseModel.add_*() methods return None, and raise ValueError when parameter validation fails
- A method involving an instruction parameter (such as Device.single_qubit_error) requires an Instruction object (constructed through Instruction.from_standard_gate()) rather than a StandardGate enum value
- The init_map parameter of Layout requires dict[Qubit, Qubit], so passing {0: 11} directly reports a type error. Layout.from_pairs() is recommended as an alternative

## Next steps

- [Topology modeling](1_topology.md): understand the directed graph model of Topology and its instantiation, query and dynamic modification operations
- [Device modeling](2_device.md): master the global default plus local override calibration strategy of Device
- [Layout mapping](3_layout.md): learn the bidirectional logical-physical qubit mapping and SWAP routing operations of Layout
- [Noise model](4_noise.md): understand the use of noise channels such as NoiseModel, SingleQubitNoise, TwoQubitNoise and ReadoutError
- [Execution result and status](5_result.md): become familiar with the complete lifecycle and error handling of Outcome, Status and ExecutionResult
