# C API Overview

## Welcome to the Cqlib C API Reference!



---

## Documentation map

### Quantum circuit (circuit)

- [Overview](0_circuit/0_overview.md)
- [Circuit](0_circuit/1_circuit.md)
- [Parameter](0_circuit/2_parameter.md)
- [Circuit To Matrix](0_circuit/3_circuit_to_matrix.md)

### Intermediate representation (ir)

- [QCIS](1_ir/1_qcis.md)
- [OpenQASM 2.0](1_ir/2_qasm2.md)
- [OpenQASM 3.0](1_ir/3_qasm3.md)

### Device (device)

- [Topology](2_device/1_topology.md)
- [Device](2_device/2_properties_device.md)
- [Layout](2_device/3_layout.md)
- [NoiseModel](2_device/4_noise.md)
- [ExecutionResult](2_device/5_result.md)

### Quantum information (qis)

- [Overview](3_qis/0_overview.md)
- [Statevector](3_qis/1_statevector.md)
- [DensityMatrix](3_qis/2_density_matrix.md)
- [StabilizerState](3_qis/3_stabilizer_state.md)
- [PauliString](3_qis/4_pauli_string.md)
- [Hamiltonian](3_qis/5_hamiltonian.md)

### Compilation (compile)

- [Compile](4_compile/1_compile.md)

### Visualization (visualization)

- [TextDrawer](5_visualization/1_text_drawer.md)
- [FigureDrawer](5_visualization/2_figure_drawer.md)

### Error mitigation (error_mitigation)

- [ErrorMitigation](6_error_mitigation/1_error_mitigation.md)
- [ZNEMitigation](6_error_mitigation/2_zne_mitigation.md)
- [VirtualDistillation](6_error_mitigation/3_virtual_distillation.md)

---

## Quick start

```c
#include <stdio.h>
#include <cqlib_c.h>

int main(void) {
    // 1. 构建两比特线路：H(0), CX(0,1)
    CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    // 2. 用 Statevector 模拟
    CStatevector *sv = statevector_from_circuit(qc);
    double probs[4];
    statevector_probabilities(sv, probs, 4);
    printf("P(00)=%.2f P(11)=%.2f\n", probs[0], probs[3]);

    // 3. 释放
    statevector_free(sv);
    circuit_free(qc);
    return 0;
}
```

Compiling and linking (MinGW gcc example; the C interface must first be built with the gnu target, see [crates/binding-c/README.md](../../../../crates/binding-c/README.md) for the rules):

```bash
cargo build --release -p binding-c --target x86_64-pc-windows-gnu
gcc main.c -Icrates/binding-c/include -Ltarget/x86_64-pc-windows-gnu/release -lbinding_c -lntdll -lws2_32 -lbcrypt -luserenv -ladvapi32 -o demo
```

---

## Global conventions

### Status codes

Every interface returning `int32_t` follows one status code scheme (positive values and zero mean success or a query result):

| Status code | Name | Typical situation |
| ---- | ---------------- | ---------------------------- |
| `0`  | Ok               | Success (or the value returned by a query interface) |
| `-1` | NullPtr          | A required handle pointer is NULL |
| `-2` | QubitOutOfBounds | The qubit index is outside the circuit |
| `-3` | CircuitError     | A circuit operation failed (invalid structure, wrong operand, and so on) |
| `-4` | ParseError       | A string or format failed to parse (QASM, parameter expressions, unknown gate names, and so on) |
| `-5` | IoError          | A file read or write failed |
| `-6` | CompilerError    | A compilation step failed |
| `-7` | SimulationError  | A simulation step failed |
| `-8` | InvalidParam     | An invalid parameter (probability out of range, length mismatch, and so on) |

### Memory ownership

1. **Constructors return heap pointers**: handle objects returned by `circuit_new`, `statevector_from_circuit` and the like are owned by the caller and released with the matching `*_free`; every `*_free` accepts NULL.
2. **Strings are released with `cqlib_string_free`**: interfaces returning `char *` (`qasm2_dumps`, `device_name` and so on) allocate on the heap, and must be released with `cqlib_string_free` rather than the C standard library `free` (the allocators differ).
3. **Arrays use two-step output**: large arrays (probabilities, unitary matrices, qubit lists) follow the `_len` plus reader pattern — first call `*_len` to obtain the element count and allocate a buffer, then call the matching reader (such as `statevector_probabilities`) to fill it.
4. **Nested lists use their matching free**: `outcome_list_free`, `counts_list_free`, `circuit_list_free` and `topology_free` release the corresponding lists or cloned objects.
5. **Explicit ownership transfer**: a few interfaces take ownership of an argument (for example `hamiltonian_from_pauli` takes over and releases the Pauli string); the header comments mark each one.

### Constant labels

Enumeration constants are provided in the header as `#define` macros; always use the macro rather than the numeric literal:

| Constant group | Value |
| ------------------------------------------------------------------------ | ------------ |
| `PAULI_I/X/Y/Z`                                                          | Pauli operator 0–3 |
| `COMPILE_MODE_NORMAL/ENHANCED`                                           | Compilation mode 0/1 |
| `COMPILE_TARGET_LOGICAL/BASIS/DEVICE/TOPOLOGY_BASIS`                     | Compilation target 0–3 |
| `NOISE_BIT_FLIP/PHASE_FLIP/DEPOLARIZING/AMPLITUDE_DAMPING/PHASE_DAMPING` | Single-qubit noise channel 0–4 |
| `NOISE_TWO_DEPOLARIZING`                                                 | Two-qubit depolarizing channel 0 |
| `MITIGATION_ZNE/VIRTUAL_DISTILLATION`                                    | Mitigation method 0/1 |
| `PROCESS_ZNE_POLYNOMIAL/ZNE_EXPONENTIAL/VIRTUAL_DISTILLATION`            | Post-processing method 0–2 |

### Threading and concurrency

Handle objects are not thread-safe: concurrent access to the same object (including a `_free` release) must be locked by the caller; different objects are independent of one another and may be used in parallel.

### Complex layout

Complex numbers use the `Complex64` structure (`double re; double im;`); matrices are stored in row-major order with the real and imaginary parts interleaved.
