# Device

`CDevice` describes the physical properties of a quantum processor: the number of qubits, the coupling topology and the native gate set.

---

## Construction and release

### device_new(name, num_qubits)

Create a device with `num_qubits` **isolated** physical qubits and no couplings.

### device_line(name, num_qubits)

Create a device with a directed line topology `0 -> 1 -> ... -> n-1`.

### device_bidirectional_line(name, num_qubits)

Create a device with a bidirectional line topology.

### device_ring(name, num_qubits)

Create a device with a bidirectional ring topology.

### device_grid(name, rows, cols)

Create a device with a bidirectional grid topology; qubit indices are assigned in row-major order (`row * cols + col`), and the total number of qubits is `rows * cols`.

### device_star(name, num_qubits, center)

Create a device with a bidirectional star topology around the `center` qubit.

### device_from_edges(name, num_qubits, edges, num_edges)

Create a device from explicit directed edges.

Parameters:

- `name` (`const char *`): the device name.
- `num_qubits` (`uint32_t`): the total number of physical qubits.
- `edges` (`const uint32_t *`): `2 * num_edges` u32 values, laid out consecutively as `(control, target)` pairs.
- `num_edges` (`uintptr_t`): the number of edges.

Notes:

- No parity check is performed on `num_edges`; the values are parsed in pairs. Referencing a qubit outside the `num_qubits` range causes the construction to fail.

All the constructors above return:

- `CDevice *`: a heap-allocated device; release it with `device_free`; NULL on failure.

### device_free(ptr)

Release a device object. Passing NULL is allowed.

---

## Properties

### device_name(device)

Return the device name.

Returns:

- `char *`: a heap-allocated string; release it with `cqlib_string_free`; NULL on failure.

### device_num_qubits(device)

Return the number of available physical qubits.

Returns:

- `uintptr_t`; a NULL handle returns `0`.

### device_native_gates(device)

Return a comma-separated list of native gate names.

Returns:

- `char *`: a heap-allocated string; release it with `cqlib_string_free`; NULL when the gate set is empty or on failure.

### device_topology(device)

Return an independent clone of the device topology; see [Topology](1_topology.md) for details.

---

## Native gate set and validation

### device_with_native_gates(device, gate_names)

**Replace** the native gate set of a device with a comma-separated list of gate names.

Parameters:

- `gate_names` (`const char *`): for example `"H,CX,RZ"`.

Returns:

- `int32_t`; `0` on success, `-4` when any gate name is unknown.

### device_validate_circuit(device, circuit)

Validate whether a circuit is compatible with a device: whether all gates used by the circuit are in the native gate set, and whether the two-qubit gates lie on coupling edges.

Parameters:

- `circuit` (`const CCircuit *`): the circuit to validate.

Returns:

- `int32_t`; `0` when compatible, `-3` when incompatible, and a negative status code on an invalid argument.

---

## Example

### Construction and query

```c
CDevice *dev = device_bidirectional_line("line-4", 4);
device_with_native_gates(dev, "H,CX,RZ,X2P");

char *name = device_name(dev);            // "line-4"
char *gates = device_native_gates(dev);   // "H,CX,RZ,X2P"
printf("%s: %s (%zu qubits)\n", name, gates,
       (size_t)device_num_qubits(dev));
cqlib_string_free(gates);
cqlib_string_free(name);

device_free(dev);
```

### Custom topology and circuit validation

```c
/* 0<->1, 1<->2 双向耦合 */
uint32_t edges[4] = {0, 1, 1, 0, 1, 2, 2, 1};
CDevice *dev = device_from_edges("custom", 3, edges, 4);
device_with_native_gates(dev, "H,CZ,RZ");

CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cz(qc, 0, 1);

int32_t ok = device_validate_circuit(dev, qc);
printf("compatible=%d\n", ok == 0);

circuit_free(qc);
device_free(dev);
```

For the qubit layout (logical to physical mapping), see [Layout](3_layout.md); for the noise model, see [NoiseModel](4_noise.md).
