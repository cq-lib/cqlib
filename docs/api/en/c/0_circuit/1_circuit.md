# Circuit

`CCircuit` is the core circuit container in the C API. It holds the qubit set, the sequence of operations arranged in order, symbolic parameters and the global phase. All state is kept in the opaque handle, and C code operates on circuits through free functions.

```c
#include <cqlib_c.h>
```

---

## Construction and release

### circuit_new(num_qubits)

Create an empty circuit containing `num_qubits` logical qubits (indices `0..num_qubits-1`).

Parameters:

- `num_qubits` (`uintptr_t`): the number of qubits.

Returns:

- `CCircuit *`: a heap-allocated circuit object, to be released with `circuit_free`; returns NULL on failure.

### circuit_free(ptr)

Release the circuit object. NULL is allowed.

```c
CCircuit *qc = circuit_new(3);
// ... 构建与使用 ...
circuit_free(qc);
```

---

## Gate operations

All gate functions return an `int32_t` status code:

| Return value | Meaning |
| --- | --- |
| `0` | Success |
| `-1` | The circuit handle is NULL |
| `-2` | The qubit index is out of range |
| `-3` | Other circuit errors |

Parameterized gates provide both a numeric variant and a symbolic parameter variant (`*_param`): a `_param` variant replaces the corresponding `double` parameters with one or more `const CParameter *`, and parameter objects are created by [param_parse](2_parameter.md).

### 1. Single-qubit gates without parameters

| Function | Gate | Description |
| --- | --- | --- |
| `circuit_i(qc, qubit)` | `I` | Identity gate |
| `circuit_h(qc, qubit)` | `H` | Hadamard gate |
| `circuit_x(qc, qubit)` | `X` | Pauli-X gate |
| `circuit_y(qc, qubit)` | `Y` | Pauli-Y gate |
| `circuit_z(qc, qubit)` | `Z` | Pauli-Z gate |
| `circuit_s(qc, qubit)` | `S` | S gate (√Z) |
| `circuit_sdg(qc, qubit)` | `S†` | S dagger gate |
| `circuit_t(qc, qubit)` | `T` | T gate (√S) |
| `circuit_tdg(qc, qubit)` | `T†` | T dagger gate |
| `circuit_x2p(qc, qubit)` | `X2P` | Rotation around the X axis by +π/2 (√X) |
| `circuit_x2m(qc, qubit)` | `X2M` | Rotation around the X axis by −π/2 |
| `circuit_y2p(qc, qubit)` | `Y2P` | Rotation around the Y axis by +π/2 (√Y) |
| `circuit_y2m(qc, qubit)` | `Y2M` | Rotation around the Y axis by −π/2 |

### 2. Parameterized single-qubit gates

| Function | Gate | Parameter | Description |
| --- | --- | --- | --- |
| `circuit_rx(qc, qubit, theta)` | `RX` | θ | Rotation around the X axis |
| `circuit_ry(qc, qubit, theta)` | `RY` | θ | Rotation around the Y axis |
| `circuit_rz(qc, qubit, theta)` | `RZ` | θ | Rotation around the Z axis |
| `circuit_phase(qc, qubit, lambda)` | `P` | λ | Phase gate |
| `circuit_u(qc, qubit, theta, phi, lambda)` | `U` | θ, φ, λ | Generic single-qubit gate |
| `circuit_xy(qc, qubit, theta)` | `XY` | θ | XY interaction gate |
| `circuit_xy2p(qc, qubit, theta)` | `XY2P` | θ | Positive half-angle XY gate |
| `circuit_xy2m(qc, qubit, theta)` | `XY2M` | θ | Negative half-angle XY gate |
| `circuit_rxy(qc, qubit, theta, phi)` | `RXY` | θ, φ | Rotation around an arbitrary axis in the XY plane |

Each gate has a corresponding `_param` variant: `circuit_rx_param`, `circuit_ry_param`, `circuit_rz_param`, `circuit_phase_param`, `circuit_xy_param`, `circuit_xy2p_param` and `circuit_xy2m_param` each take one `CParameter *`; `circuit_u_param` takes three parameter objects, θ, φ and λ; `circuit_rxy_param` takes two parameter objects, θ and φ.

### 3. Two-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `circuit_cx(qc, control, target)` | `CX` | CNOT gate |
| `circuit_cy(qc, control, target)` | `CY` | controlled-Y gate |
| `circuit_cz(qc, control, target)` | `CZ` | controlled-Z gate |
| `circuit_swap(qc, a, b)` | `SWAP` | Exchange the states of two qubits |
| `circuit_rxx(qc, a, b, theta)` | `RXX` | `exp(-i θ XX / 2)` |
| `circuit_ryy(qc, a, b, theta)` | `RYY` | `exp(-i θ YY / 2)` |
| `circuit_rzz(qc, a, b, theta)` | `RZZ` | `exp(-i θ ZZ / 2)` |
| `circuit_rzx(qc, a, b, theta)` | `RZX` | `exp(-i θ ZX / 2)` |
| `circuit_crx(qc, control, target, theta)` | `CRX` | controlled-RX gate |
| `circuit_cry(qc, control, target, theta)` | `CRY` | controlled-RY gate |
| `circuit_crz(qc, control, target, theta)` | `CRZ` | controlled-RZ gate |
| `circuit_fsim(qc, a, b, theta, phi)` | `fSim` | fSim(theta, phi) gate |

Corresponding `*_param` variants are also provided: the single-parameter gates (`rxx/ryy/rzz/rzx/crx/cry/crz`) take one parameter object, and `circuit_fsim_param` takes two parameter objects, θ and φ.

### 4. Three-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `circuit_ccx(qc, control1, control2, target)` | `CCX` | Toffoli gate |

### 5. Non-unitary instructions

| Function | Instruction | Description |
| --- | --- | --- |
| `circuit_measure(qc, qubit)` | `measure` | Measure a single qubit in the Z basis; the result is recorded as a circuit classical value |
| `circuit_reset(qc, qubit)` | `reset` | Reset the qubit to `\|0⟩` |
| `circuit_barrier(qc, qubits, count)` | `barrier` | Insert a barrier on the specified qubits |

The behavior of `circuit_barrier`:

- `qubits` is NULL or `count` is 0: create a global barrier covering **all** qubits;
- Otherwise: create a barrier only on the qubits specified by the `qubits` array.

A barrier is used to prevent the compiler from reordering operations on the affected qubits across that boundary; it does not change the quantum state itself.

### Example

```c
CCircuit *qc = circuit_new(3);
circuit_h(qc, 0);                 // H(0)
circuit_cx(qc, 0, 1);             // CX(0, 1)
circuit_rzz(qc, 1, 2, 0.25);      // RZZ(1, 2, 0.25)

/* 符号参数版本 */
CParameter *theta = param_parse("theta/2");
circuit_rx_param(qc, 2, theta);   // RX(2, theta/2)
param_free(theta);

/* 非酉指令 */
uint32_t target[2] = {0, 1};
circuit_barrier(qc, target, 2);   // 局部 barrier
circuit_measure(qc, 0);
circuit_reset(qc, 2);

circuit_free(qc);
```

---

## Circuit properties

```c
uintptr_t circuit_num_qubits(const struct CCircuit *ptr);
uintptr_t circuit_num_operations(const struct CCircuit *ptr);
uintptr_t circuit_num_parameters(const struct CCircuit *ptr);
int32_t   circuit_validate(const struct CCircuit *ptr);
intptr_t  circuit_depth(const struct CCircuit *ptr, bool recurse);
uintptr_t circuit_width(const struct CCircuit *ptr);
uintptr_t circuit_qubits_len(const struct CCircuit *ptr);
uintptr_t circuit_qubits(const struct CCircuit *ptr, uint32_t *out, uintptr_t len);
```

| Function | Returns | Description |
| --- | --- | --- |
| `circuit_num_qubits` | Qubit count | A NULL handle returns `0` |
| `circuit_width` | Qubit count | An alias for `num_qubits` |
| `circuit_num_operations` | Operation count | The total number of operations, counted in append order |
| `circuit_num_parameters` | Symbolic parameter count | The number of symbolic parameters recorded in the circuit; `0` after all of them are bound |
| `circuit_validate` | Status code | Validate circuit consistency; `0` on success, `-3` on failure |
| `circuit_depth(recurse)` | Depth | Returns a negative value on error (NULL) |
| `circuit_qubits_len` | Qubit count | Used to allocate the `circuit_qubits` buffer |

`circuit_depth(ptr, recurse)` estimates the circuit depth. The depth is the number of operation layers on the longest qubit path after the operations are arranged by as-soon-as-possible (ASAP) scheduling: ordinary gates and non-unitary instructions usually contribute one layer, and a `barrier` constrains reordering on the qubits it covers. When `recurse` is `true`, composite sub-circuits are recursively expanded in the count.

`circuit_qubits` copies the qubit indices into `out` and returns the total number of qubits in the circuit. When `out` is NULL or `len` is smaller than the total qubit count, no copy is performed and only the total is returned — it can be called first to probe the required capacity:

```c
CCircuit *qc = circuit_new(3);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

printf("qubits=%zu ops=%zu depth=%zu\n",
       (size_t)circuit_num_qubits(qc),
       (size_t)circuit_num_operations(qc),
       (size_t)circuit_depth(qc, false));

uint32_t ids[3];
circuit_qubits(qc, ids, 3);   // ids = {0, 1, 2}

circuit_free(qc);
```

---

## Structural operations

### circuit_remove_operation(ptr, index)

Delete the operation at the specified position; subsequent operations shift forward.

Parameters:

- `index` (`uintptr_t`): the operation index, counted from 0 in append order.

Returns:

- `int32_t`; `0` on success, and a negative status code for errors such as an index out of range.

### circuit_inverse(ptr)

Return the inverse circuit: the operation order is reversed, and each invertible operation is inverted.

Returns:

- `CCircuit *`: a new, **independently owned** circuit, to be released with `circuit_free`; returns NULL on failure.

### circuit_decompose(ptr)

Return a copy with composite gates recursively expanded.

Circuits that carry composite gates from QASM/QCIS parsing or from compilation results are recursively expanded.

Returns:

- `CCircuit *`: the new circuit; returns NULL on failure.

### circuit_compose(ptr, other, qubits_map, map_len)

Append the operations of `other` to the current circuit in order; the behavior is determined by `qubits_map`:

- `qubits_map` is NULL (`map_len` passed as 0): merge aligned by qubit index, and qubits are added automatically when `other` uses indices that do not exist in the current circuit;
- `qubits_map` is not NULL: map the `i`-th qubit of `other` onto `qubits_map[i]` of the current circuit.

Returns:

- `int32_t`; `0` on success, and `-2` when a mapping target is out of range.

Combining it with an inverse circuit is a typical usage:

```c
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

CCircuit *inv = circuit_inverse(qc);
circuit_compose(qc, inv, NULL, 0);     // qc 现在等价于恒等操作

circuit_free(inv);
circuit_free(qc);
```

### circuit_add_qubits(ptr, qubits, count)

Append new qubits to the circuit.

Parameters:

- `qubits` (`const uint32_t *`): the array of indices of the new qubits;
- `count` (`uintptr_t`): the length of the array.

Returns:

- `int32_t`; `0` on success, and a negative status code for a duplicate or invalid index.

### circuit_set_global_phase(ptr, phase) / circuit_set_global_phase_param(ptr, param)

Set the circuit global phase, as a numeric variant and a symbolic parameter variant respectively.

Returns:

- `int32_t`; `0` on success, and a negative status code for a NULL handle or an invalid parameter.

```c
CCircuit *qc = circuit_new(1);
circuit_x(qc, 0);
circuit_set_global_phase(qc, 3.141592653589793 / 4.0);
circuit_free(qc);
```

---

For parsing, evaluation and whole-circuit binding of symbolic parameters, see [Parameter](2_parameter.md); for unitary matrix export, see [Circuit To Matrix](3_circuit_to_matrix.md).
