# Topology

`CTopology` describes the relationship between hardware physical qubits and coupling edges. A topology is obtained by cloning a device and is a read-only view.

---

## Obtaining the topology

### device_topology(device)

Clone and return the topology object of the device.

Parameters:

- `device` (`const CDevice *`): the device.

Returns:

- `CTopology *`: a new **independently owned** object whose lifetime is independent of the source device; release it with `topology_free`; NULL on failure.

### topology_free(ptr)

Release a topology object. Passing NULL is allowed.

---

## Properties

### topology_num_qubits(topology)

Return the number of physical qubits in the topology.

Returns:

- `uintptr_t`; a NULL handle returns `0`.

### topology_num_couplings(topology)

Return the number of coupling edges in the topology.

Returns:

- `uintptr_t`; a NULL handle returns `0`.

---

## Notes

The topology is stored with **directed edge** semantics: `(u, v)` denotes a coupling that allows `u` as control and `v` as target. A bidirectional coupling is stored as two directed edges, so the bidirectional line `0 - 1 - 2` has `num_couplings` equal to `4`.

The shape of the topology is determined by how the device was constructed; see the constructor table in [Device](2_properties_device.md).

---

## Example

```c
CDevice *dev = device_bidirectional_line("line-4", 4);

CTopology *topo = device_topology(dev);
printf("qubits=%zu couplings=%zu\n",
       (size_t)topology_num_qubits(topo),
       (size_t)topology_num_couplings(topo));   // 4, 6

topology_free(topo);
device_free(dev);
```

After the device is destroyed, a cloned topology can still be used independently:

```c
device_free(dev);
printf("%zu\n", (size_t)topology_num_qubits(topo));   // 仍然有效
topology_free(topo);
```
