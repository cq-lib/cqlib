# Device modeling

The Device module is responsible for aggregating the full hardware characteristics of a backend. It combines the abstract physical topology (Topology) with concrete calibration parameters, providing data support for noise-aware compilation and high-fidelity simulation.

Cqlib adopts a **"global default plus local override"** calibration strategy: when a parameter is queried, the local value is returned first, and the query falls back to the global default when no local value is configured.

---

## Core objects

| Object | Use |
|---|---|
| InstructionProp | The physical behavior of a specific gate instruction (error rate, execution duration); an Instruction object (created through Instruction.from_standard_gate()) must be passed to the constructor |
| QubitProp | Single-qubit properties (T1/T2, readout error, frequency, native gate list) |
| EdgeProp | Coupling edge properties (native two-qubit instruction set) |
| Device | Top-level entity that integrates topology and properties, providing global defaults and query interfaces |

---

## Building a device baseline

```python
from cqlib.circuit import StandardGate
from cqlib.device import Device, Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])
device = Device("demo_backend", [0, 1, 2], topo)
device.default_t1 = 50.0
device.default_t2 = 35.0
device.default_readout_error = 0.05
device.default_single_qubit_error = 0.001
device.default_two_qubit_error = 0.01

print("device name:", device.name)
print("register qubit count:", len(device.qubits))
```

**Note**: global defaults are used only for subsequent fallback queries. If no local value is set for a qubit, calling get_t1(q) returns default_t1.

### Factory methods

Device provides factory methods for quickly constructing common topology structures, saving the step of creating a Topology manually:

```python
from cqlib.device import Device

d1 = Device.line("line_dev", num_qubits=5)                    # unidirectional line
d2 = Device.bidirectional_line("bi_line", num_qubits=5)      # bidirectional line
d3 = Device.ring("ring_dev", num_qubits=4)                    # bidirectional ring
d4 = Device.star("star_dev", num_qubits=5, center=0)         # bidirectional star
d5 = Device.grid("grid_dev", rows=3, cols=4)                  # bidirectional grid (row-major order)
d6 = Device.from_edges("custom", num_qubits=4, edges=[(0, 1), (1, 2)])  # custom directed edges

print("line:", d1.num_usable_qubits)
print("bidirectional line:", d2.num_usable_qubits)
print("ring:", d3.num_usable_qubits)
print("star:", d4.num_usable_qubits)
print("grid:", d5.num_usable_qubits)
print("custom:", d6.num_usable_qubits)
```

---

## Injecting local calibration data

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import Device, EdgeProp, InstructionProp, QubitProp, Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])
device = Device("dev", [0, 1, 2], topo)

# ---- single-qubit calibration ----
q0 = QubitProp(readout_error=0.02)
q0.t1 = 80.0              # T1 relaxation time (microseconds)
q0.t2 = 70.0              # T2 decoherence time (microseconds)
q0.frequency = 5.1        # frequency (GHz)

# set the measurement discrimination error
q0.prob_meas0_prep1 = 0.02  # P(measure 0 | prepare 1)
q0.prob_meas1_prep0 = 0.01  # P(measure 1 | prepare 0)

# Add a native single-qubit gate (note: use add_native_instruction(); appending to native_instructions has no effect)
x_prop = InstructionProp(
    Instruction.from_standard_gate(StandardGate.X),
    error_rate=0.001,
)
x_prop.length = 20.0  # gate duration (nanoseconds)
q0.add_native_instruction(x_prop)

device.add_qubit_properties(0, q0)

# ---- coupling edge calibration ----
cx_prop = InstructionProp(
    Instruction.from_standard_gate(StandardGate.CX),
    error_rate=0.015,
)
cx_prop.length = 200.0

edge = EdgeProp()
edge.add_native_instruction(cx_prop)
device.add_edge_properties(0, 1, edge)

print("local properties of qubit 0 injected")
```

**Note**:
- The first parameter of the InstructionProp constructor must be an Instruction object, created with Instruction.from_standard_gate(StandardGate.X). Passing StandardGate.X directly is not allowed
- QubitProp.native_instructions is a read-only property that returns a new list on every read: it cannot be assigned directly (q0.native_instructions = [...] raises AttributeError), and calling .append() on it has no effect (only a temporary copy is modified); entries must be added through add_native_instruction()
- EdgeProp.native_instructions is also a read-only property, and entries must be added through the add_native_instruction() method

---

## Parameter queries and the fallback mechanism

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import Device, Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])
device = Device("dev", [0, 1, 2], topo)
device.default_t1 = 50.0
device.default_single_qubit_error = 0.001

# falls back to the default when no local value is set
print("T1 of qubit 1 (default fallback):", device.get_t1(1))

# after setting local calibration for qubit 0, the local value takes priority
q0_prop = QubitProp(readout_error=0.02)
q0_prop.t1 = 80.0
device.add_qubit_properties(0, q0_prop)
print("T1 of qubit 0 (local value):", device.get_t1(0))

# query the single-qubit gate error rate (note: the third parameter must be an Instruction object)
x_inst = Instruction.from_standard_gate(StandardGate.X)
print("X gate error of qubit 0 (default fallback):", device.single_qubit_error(0, x_inst))

# query the readout error (global default)
device.default_readout_error = 0.05
print("readout error of qubit 0 (default fallback):", device.get_readout_error(0))
```

**Fallback chain**:
- get_t1(q): returns QubitProp(q).t1 first, and returns device.default_t1 if it is not set
- get_readout_error(q): returns QubitProp(q).readout_error first, and returns device.default_readout_error if it is not set
- single_qubit_error(q, inst): looks up in order → ① native gate error ② single-qubit default error ③ device global default
- If a qubit is unavailable (not registered or marked as invalid), all the queries above return None

---

## Invalid qubit management

```python
from cqlib.device import Device, Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])
device = Device("dev", [0, 1, 2], topo)

# mark qubit 2 as invalid (offline/faulty)
device.invalid_qubits = [2]   # note: use a list, not a set

print("number of usable qubits:", device.num_usable_qubits)  # 2
print("usable qubit list:", device.usable_qubits)     # [PhysicalQubit(0), PhysicalQubit(1)]
print("is qubit 2 usable:", device.is_usable_qubit(2))  # False
```

---

## Robustness validation

```python
from cqlib.device import Device, QubitProp, Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])
device = Device("dev", [0, 1, 2], topo)

try:
    device.add_qubit_properties(99, QubitProp(0.01))
except ValueError as e:
    print("adding properties for a nonexistent qubit:", e)

try:
    device.add_edge_properties(0, 9, EdgeProp())
except ValueError as e:
    print("adding properties for a nonexistent edge:", e)
```

---

## Next steps

- [Layout mapping](3_layout.md): learn the bidirectional logical-physical qubit mapping and SWAP routing operations of Layout
- [Noise model](4_noise.md): understand the use of noise channels such as NoiseModel, SingleQubitNoise, TwoQubitNoise and ReadoutError
- [Execution result and status](5_result.md): become familiar with the complete lifecycle and error handling of Outcome, Status and ExecutionResult
