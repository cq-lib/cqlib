# Backend and device configuration

After authentication is complete, a quantum backend must be selected. A backend on the Tianyan platform is represented by `TianyanBackend`, which provides attributes such as the backend name, display name, status and billing type, as well as interfaces for querying the physical qubit count and downloading the device configuration.

## 1. Listing backends

```python
import os
from cqlib_tianyan import TianyanPlatform

platform = TianyanPlatform.login(os.environ["TIANYAN_API_KEY"])
backends = platform.list_backends()

for backend in backends:
    print(backend.name, backend.display_name, backend.status, backend.toll)
```

`list_backends()` returns a list of `TianyanBackend`.

## 2. Backend object attributes and methods

| Member | Form | Return type | Description |
|---|---|---|---|
| `name` | Attribute | `str` | Backend device identifier, used to submit tasks, for example `tianyan-287` |
| `display_name` | Attribute | `str` | User-friendly display name |
| `status` | Attribute | `DeviceStatus` | Device running status |
| `toll` | Attribute | `DeviceToll` | Billing type |
| `num_qubits()` | Method | `int` | Returns the total number of physical qubits in the device configuration, including disabled qubits; the first call downloads the device configuration, and subsequent calls reuse the cache |

Example:

```python
for backend in platform.list_backends():
    print(f"name={backend.name}")
    print(f"display={backend.display_name}")
    print(f"status={backend.status}")
    print(f"toll={backend.toll}")

# after the backend is selected, query the total physical qubit count in its device configuration
backend = platform.get_backend("tianyan-287")
print(f"qubits={backend.num_qubits()}")
```

`num_qubits()` is a method rather than an attribute. The first call to this method may issue a network request to obtain the device configuration; to determine the currently available physical qubits, further check the device topology and disabled qubit information returned by `device_config()`.

## 3. Backend status

`DeviceStatus` can be compared with strings:

| Value | Meaning |
|---|---|
| `running` | The device is online and can accept tasks |
| `calibration` | The device is being calibrated; tasks may be queued |
| `under_maintenance` | Under maintenance; temporarily unavailable |
| `offline` | Offline |
| `unknown` | The platform returned an unrecognized status |

It is recommended to check before submission:

```python
backend = platform.get_backend("tianyan-287")

if not backend.is_available():
    raise RuntimeError(f"backend unavailable: {backend.status}")
```

A direct comparison is also possible:

```python
if backend.status == "running":
    print("backend can accept submissions")
```

## 4. Billing type

`DeviceToll` represents the backend billing type:

| Value | Meaning |
|---|---|
| `free` | Free task |
| `paid` | Consumes quota or is billed |
| `unknown` | The platform returned an unrecognized billing type |

```python
if backend.toll == "paid":
    print("this backend may consume quota, confirm before submitting")
```

## 5. Getting a specific backend

```python
backend = platform.get_backend("tianyan-287")
```

If the backend does not exist, an exception is raised.

## 6. Getting the device configuration

`device_config()` downloads device topology, calibration and error information and returns a `cqlib.device.Device` object:

```python
device = backend.device_config()

print(device.name)
print(device.num_usable_qubits)
print(device.topology)
```

The result is cached inside the backend object; repeated calls on the same `TianyanBackend` do not download it again each time.

## 7. Uses of the device configuration

The device configuration can be used to:

- Inspect available physical qubits.
- Inspect the coupling topology.
- Assist the compiler in layout mapping and gate routing.
- Obtain readout fidelity for readout error correction.
- Analyze single-qubit gate, two-qubit gate and measurement errors.

Typical flow:

```python
backend = platform.get_backend("tianyan-287")
device = backend.device_config()

# the device can later be passed to the compilation or analysis module
```

## 8. Backend selection recommendations

| Scenario | Recommendation |
|---|---|
| Only testing the API chain | Choose a `running` backend that is free or low-cost |
| A specific physical qubit is required | Check the device topology and available qubits first |
| Better result quality is required | Pay attention to readout error, two-qubit gate error and calibration time |
| Batch submission | Test with a small number of circuits first, then expand the batch |
| Error correction is required | Confirm that the device configuration has usable calibration data |

## Next steps

- [Task submission and result retrieval](3_task_result.md): use the selected backend to submit QCIS circuits, query task status and obtain results.
- [QCIS and IR integration](4_qcis_ir_workflow.md): learn how to export QCIS from a Cqlib `Circuit` and then submit it to the Tianyan platform.
