# Device / Properties

This page covers the core types in `cqlib.device` used for device calibration modeling:

- `InstructionProp`
- `QubitProp`
- `EdgeProp`
- `Device`

## Import

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import Device, EdgeProp, InstructionProp, QubitProp, Topology
```

---

## InstructionProp

Instruction properties: an instruction, its error rate and an optional duration.

### `InstructionProp(instruction, error_rate)`

Parameters:

- `instruction` (`Instruction`): the instruction object.
- `error_rate` (`float`): the error rate, which must be within `[0, 1]`.

Raises:

- `ValueError`: `error_rate` is outside `[0, 1]` or is not a finite value.

### Attributes

- `instruction -> Instruction`: the instruction object, readable and writable.
- `error_rate -> float`: the error rate, readable and writable, validated against `[0, 1]` when written.
- `length -> float | None`: the instruction duration, readable and writable, required to be a non-negative finite value when written.

### Other behavior

- Supports `copy` and `deepcopy`.
- `repr(prop) == "InstructionProp(instruction=X, error_rate=0.01)"`, with `length=...` appended after `length` is set.

## QubitProp

Qubit properties: readout error, readout confusion probabilities, `t1` / `t2`, frequency and native instruction properties.

### `QubitProp(readout_error)`

Parameters:

- `readout_error` (`float`): the readout error, which must be within `[0, 1]`.

Raises:

- `ValueError`: `readout_error` is outside `[0, 1]` or is not a finite value.

### Attributes

- `readout_error -> float`: read-only.
- `prob_meas0_prep1 -> float | None`: readable and writable, the value must be within `[0, 1]`.
- `prob_meas1_prep0 -> float | None`: readable and writable, the value must be within `[0, 1]`.
- `t1 -> float | None`: readable and writable, a finite positive number is required.
- `t2 -> float | None`: readable and writable, a finite positive number is required.
- `frequency -> float | None`: readable and writable, a finite positive number is required.
- `native_instructions -> list[InstructionProp]`: read-only, the native instruction properties of this qubit.

### Methods

- `add_native_instruction(prop) -> None`: append a native instruction property.

Raises:

- `ValueError`: an invalid probability-like parameter, `t1` / `t2` / `frequency` is not a finite positive number, or the appended instruction is not a one-qubit standard gate.
- When a write fails, the previous value stays unchanged.

## EdgeProp

Edge properties: the native instruction properties on a directed edge.

### `EdgeProp()`

Constructor with no arguments.

### Methods

- `add_native_instruction(prop) -> None`: append a native instruction property.

Raises:

- `ValueError`: the appended instruction is not a two-qubit standard gate, or its error rate or duration is invalid.

### Attributes

- `native_instructions -> list[InstructionProp]`: read-only.

## Device

The device object: topology, registered qubit set, native gate set, calibration time and device-level defaults.

### `Device(name, qubits, topology)`

Parameters:

- `name` (`str`): the device name.
- `qubits` (`list[int | Qubit | PhysicalQubit]`): the registered physical qubits.
- `topology` (`Topology`): the device topology.

Raises:

- `ValueError`: the topology contains a qubit that is not registered in `qubits`.

### Static constructors

- `Device.line(name, num_qubits) -> Device`: a directed straight line formed by qubits `0..num_qubits`.
- `Device.line_from_qubits(name, physical_qubits) -> Device`: a directed straight line connected in the given physical qubit order.
- `Device.bidirectional_line(name, num_qubits) -> Device`: a bidirectional straight line.
- `Device.ring(name, num_qubits) -> Device`: a bidirectional ring.
- `Device.star(name, num_qubits, center) -> Device`: a bidirectional star centered on `center`.
- `Device.grid(name, rows, cols) -> Device`: a `rows × cols` bidirectional grid, with qubit indices arranged in row-major order.
- `Device.from_edges(name, num_qubits, edges) -> Device`: qubits `0..num_qubits` plus explicit directed edges `list[tuple[int, int]]`.

### Attributes

- `name -> str`: read-only.
- `qubits -> list[PhysicalQubit]`: read-only, the registered physical qubits.
- `invalid_qubits -> list[PhysicalQubit]`: readable and writable; the written value must already be registered on the device, otherwise it is rejected and the previous value is kept.
- `topology -> Topology`: read-only.
- `native_gates -> list[Instruction]`: readable and writable; writing accepts only standard gates of at most two qubits.
- `usable_qubits -> list[PhysicalQubit]`: read-only, the registered qubits that are not marked invalid.
- `num_usable_qubits -> int`: read-only.
- `default_t1 -> float | None`: readable and writable, the device-level default `t1`.
- `default_t2 -> float | None`: readable and writable, the device-level default `t2`.
- `default_readout_error -> float | None`: readable and writable, the device-level default readout error.
- `default_single_qubit_error -> float | None`: readable and writable, the device-level default single-qubit error rate.
- `default_two_qubit_error -> float | None`: readable and writable, the device-level default two-qubit error rate.
- `calibration_time -> datetime | None`: read-only, the calibration timestamp with nanosecond precision.

### Write methods

- `set_calibration_time(datetime) -> None`: write the calibration timestamp.
- `add_qubit_properties(qubit, props) -> None`: write the properties of a qubit.
- `add_edge_properties(control, target, props) -> None`: write the properties of a directed edge.

Raises:

- `ValueError`: the qubit is not in the device or not in the topology, the directed edge is not in the topology, a default value is invalid, or the time is outside the representable range.
- When any of the three kinds of writes fails, the device keeps its previous values.

### Query methods

- `qubit_properties(qubit) -> QubitProp | None`
- `edge_properties(control, target) -> EdgeProp | None`
- `get_t1(qubit) -> float | None`
- `get_t2(qubit) -> float | None`
- `get_readout_error(qubit) -> float | None`
- `single_qubit_error(qubit, instruction) -> float | None`
- `two_qubit_error(control, target, instruction) -> float | None`
- `edge_error(control, target) -> float | None`
- `is_usable_qubit(qubit) -> bool`
- `supports_native_instruction(instruction, qargs) -> bool`
- `validate_operation(operation) -> None`
- `validate_circuit(circuit) -> None`

Description:

- `get_t1` / `get_t2` / `get_readout_error` return local properties first and fall back to the device-level default when they are missing.
- `single_qubit_error` / `two_qubit_error` take the local native instruction property first and fall back to the default single-qubit / two-qubit error rate when it is missing; they return `None` when the qubit is unusable or the direction does not exist.
- `edge_error` reads calibration data only and takes the minimum error rate of the native instructions on that directed edge.
- `validate_operation` / `validate_circuit` raise `ValueError` when validation fails.

### Other behavior

- Supports `copy` and `deepcopy`.
- `repr(device) == "Device(name='mock_backend')"`.

## Example

```python
from cqlib import Qubit
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import Device, EdgeProp, InstructionProp, QubitProp, Topology

topo = Topology([0, 1, 2], [(0, 1, "G1"), (1, 2, "G2")])
device = Device("mock_backend", [Qubit(0), Qubit(1), Qubit(2)], topo)

device.default_t1 = 50.0
device.default_t2 = 35.0
device.default_readout_error = 0.05
device.native_gates = [
    Instruction.from_standard_gate(StandardGate.X),
    Instruction.from_standard_gate(StandardGate.CX),
]

x_inst = Instruction.from_standard_gate(StandardGate.X)
cx_inst = Instruction.from_standard_gate(StandardGate.CX)

qp0 = QubitProp(0.02)
qp0.t1 = 80.0
qp0.t2 = 70.0
qp0.add_native_instruction(InstructionProp(x_inst, 0.001))
device.add_qubit_properties(0, qp0)

ep01 = EdgeProp()
ep01.add_native_instruction(InstructionProp(cx_inst, 0.02))
device.add_edge_properties(0, 1, ep01)

assert device.get_t1(0) == 80.0
assert device.get_t1(2) == 50.0  # 回退到设备级默认值
assert device.single_qubit_error(0, x_inst) == 0.001
assert device.two_qubit_error(0, 1, cx_inst) == 0.02
assert device.edge_error(0, 1) == 0.02
assert device.num_usable_qubits == 3

device.invalid_qubits = [2]
assert device.is_usable_qubit(2) is False
assert device.num_usable_qubits == 2
```
