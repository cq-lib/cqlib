# Standard Gates

This page covers all C API functions that append standard quantum gates to a circuit; for global conventions on error codes and memory management, see [Overview](../0_overview.md).

---

## Conventions

- Every function on this page returns an `int32_t` error code: `0` on success, `-1` on NULL input, `-2` on an out-of-bounds qubit index, `-3` on a circuit error (a NaN or infinite numeric angle also returns `-3`).
- Numeric variants pass angles (in radians) as `double`; the matching `_param` variants pass angles as `const CParameter*` symbolic expressions and return `-1` when the parameter handle is NULL. `CParameter` handles are created with `param_parse` and released with `param_free`; see [Parameter](3_parameter.md).
- Each gate acts on a fixed number of qubits: one for single-qubit gates, two for two-qubit gates, three for the three-qubit gate; `qubit`/`control`/`target`/`a`/`b` are qubit indices within the circuit.

---

## Single-qubit gates without parameters

The following 13 functions all share the signature `(struct CCircuit *ptr, uint32_t qubit)` and append a fixed-matrix single-qubit gate.

| Function | Gate | Description |
| --- | --- | --- |
| `circuit_h(ptr, qubit)` | H | Hadamard gate, converting between the computational and superposition bases. |
| `circuit_x(ptr, qubit)` | X | Pauli-X gate, a bit flip. |
| `circuit_y(ptr, qubit)` | Y | Pauli-Y gate, a bit flip with a phase. |
| `circuit_z(ptr, qubit)` | Z | Pauli-Z gate, a phase flip. |
| `circuit_i(ptr, qubit)` | I | Identity gate, leaves the quantum state unchanged. |
| `circuit_s(ptr, qubit)` | S | Clifford phase gate, diag(1, i). |
| `circuit_sdg(ptr, qubit)` | SDG | Inverse of S, diag(1, −i). |
| `circuit_t(ptr, qubit)` | T | π/8 phase gate, diag(1, e^(iπ/4)). |
| `circuit_tdg(ptr, qubit)` | TDG | Inverse of T, diag(1, e^(−iπ/4)). |
| `circuit_x2p(ptr, qubit)` | X2P | √X gate, a +90° rotation about the X axis. |
| `circuit_x2m(ptr, qubit)` | X2M | √X† gate, a −90° rotation about the X axis. |
| `circuit_y2p(ptr, qubit)` | Y2P | √Y gate, a +90° rotation about the Y axis. |
| `circuit_y2m(ptr, qubit)` | Y2M | √Y† gate, a −90° rotation about the Y axis. |

---

## Parameterized single-qubit gates

Numeric variants pass angles as `double`; `_param` variants pass a `const CParameter*`; both append the same standard gate.

| Function | Gate | Description |
| --- | --- | --- |
| `circuit_rx(ptr, qubit, theta)` | RX | Rotation about the X axis by θ, exp(−iθX/2). |
| `circuit_rx_param(ptr, qubit, param_ptr)` | RX | Same gate, with a symbolic angle. |
| `circuit_ry(ptr, qubit, theta)` | RY | Rotation about the Y axis by θ, exp(−iθY/2). |
| `circuit_ry_param(ptr, qubit, param_ptr)` | RY | Same gate, with a symbolic angle. |
| `circuit_rz(ptr, qubit, theta)` | RZ | Rotation about the Z axis by θ, exp(−iθZ/2). |
| `circuit_rz_param(ptr, qubit, param_ptr)` | RZ | Same gate, with a symbolic angle. |
| `circuit_phase(ptr, qubit, lambda)` | Phase | Phase gate diag(1, e^(iλ)), applying a phase to the `1` component. |
| `circuit_phase_param(ptr, qubit, param)` | Phase | Same gate, with a symbolic phase. |
| `circuit_u(ptr, qubit, theta, phi, lambda)` | U | General single-qubit gate U(θ, φ, λ) with three angles. |
| `circuit_u_param(ptr, qubit, theta, phi, lambda)` | U | Same gate, with three symbolic angles. |
| `circuit_xy(ptr, qubit, theta)` | XY | XY-interaction-family gate with off-diagonal entries −i·e^(∓iθ). |
| `circuit_xy_param(ptr, qubit, param)` | XY | Same gate, with a symbolic phase. |
| `circuit_xy2p(ptr, qubit, theta)` | XY2P | +90° rotation about the XY-plane axis selected by θ. |
| `circuit_xy2p_param(ptr, qubit, param)` | XY2P | Same gate, with a symbolic axis angle. |
| `circuit_xy2m(ptr, qubit, theta)` | XY2M | −90° rotation about the XY-plane axis selected by θ. |
| `circuit_xy2m_param(ptr, qubit, param)` | XY2M | Same gate, with a symbolic axis angle. |
| `circuit_rxy(ptr, qubit, theta, phi)` | RXY | Rotation by θ about the XY-plane axis determined by φ. |
| `circuit_rxy_param(ptr, qubit, theta, phi)` | RXY | Same gate, with two symbolic angles. |

---

## Two-qubit gates

`cx`/`cy`/`cz`/`swap` are fixed-matrix gates; the rest are parameterized by angles, with `_param` variants taking a `const CParameter*`.

| Function | Gate | Description |
| --- | --- | --- |
| `circuit_cx(ptr, control, target)` | CX | Controlled-X (CNOT); flips the target when the control is `1`. |
| `circuit_cy(ptr, control, target)` | CY | Controlled-Y; applies Y to the target when the control is `1`. |
| `circuit_cz(ptr, control, target)` | CZ | Controlled-Z; applies Z to the target when the control is `1`. |
| `circuit_swap(ptr, a, b)` | SWAP | Exchanges the states of two qubits. |
| `circuit_rxx(ptr, a, b, theta)` | RXX | Two-body Pauli rotation exp(−iθ X⊗X/2). |
| `circuit_rxx_param(ptr, a, b, param)` | RXX | Same gate, with a symbolic angle. |
| `circuit_ryy(ptr, a, b, theta)` | RYY | Two-body Pauli rotation exp(−iθ Y⊗Y/2). |
| `circuit_ryy_param(ptr, a, b, param)` | RYY | Same gate, with a symbolic angle. |
| `circuit_rzz(ptr, a, b, theta)` | RZZ | Two-body Pauli rotation exp(−iθ Z⊗Z/2). |
| `circuit_rzz_param(ptr, a, b, param)` | RZZ | Same gate, with a symbolic angle. |
| `circuit_rzx(ptr, a, b, theta)` | RZX | Two-body Pauli rotation exp(−iθ Z⊗X/2), with Z on `a` and X on `b`. |
| `circuit_rzx_param(ptr, a, b, param)` | RZX | Same gate, with a symbolic angle. |
| `circuit_crx(ptr, control, target, theta)` | CRX | Controlled RX(θ); rotates the target when the control is `1`. |
| `circuit_crx_param(ptr, control, target, param)` | CRX | Same gate, with a symbolic angle. |
| `circuit_cry(ptr, control, target, theta)` | CRY | Controlled RY(θ); rotates the target when the control is `1`. |
| `circuit_cry_param(ptr, control, target, param)` | CRY | Same gate, with a symbolic angle. |
| `circuit_crz(ptr, control, target, theta)` | CRZ | Controlled RZ(θ); rotates the target when the control is `1`. |
| `circuit_crz_param(ptr, control, target, param)` | CRZ | Same gate, with a symbolic angle. |
| `circuit_fsim(ptr, a, b, theta, phi)` | fSim | fSim gate: rotates the `01`/`10` block by θ and applies an e^(−iφ) phase to the `11` component. |
| `circuit_fsim_param(ptr, a, b, theta, phi)` | fSim | Same gate, with two symbolic angles. |

---

## Three-qubit gate

| Function | Gate | Description |
| --- | --- | --- |
| `circuit_ccx(ptr, control1, control2, target)` | CCX | Toffoli gate; flips the target when both controls are `1`. |

---

## Mathematical conventions

Rotation gates use the half-angle exponential definition:

```text
RX(θ) = exp(-i θ X / 2)
RY(θ) = exp(-i θ Y / 2)
RZ(θ) = exp(-i θ Z / 2)
RXY(θ, φ) = rotation by θ about the XY-plane axis at azimuth φ (half-angle convention)
```

Two-body Pauli rotation gates use the same convention:

```text
RXX(θ) = exp(-i θ X⊗X / 2)
RYY(θ) = exp(-i θ Y⊗Y / 2)
RZZ(θ) = exp(-i θ Z⊗Z / 2)
RZX(θ) = exp(-i θ Z⊗X / 2)
```

Matrix forms of the remaining gates:

```text
U(θ, φ, λ) = [[cos(θ/2),          -e^(-iλ)·sin(θ/2)],
              [e^(iφ)·sin(θ/2),    e^(i(φ+λ))·cos(θ/2)]]

fSim(θ, φ) = [[1,      0,           0,          0      ],
               [0,      cos(θ),     -i·sin(θ),   0      ],
               [0,     -i·sin(θ),   cos(θ),      0      ],
               [0,      0,           0,          e^(-iφ)]]
```

---

## Example

```c
#include "cqlib_c.h"

/* Bell circuit: H(0) + CX(0, 1) */
CCircuit *c = circuit_new(2);

if (circuit_h(c, 0) != 0) {
    /* handle error */
}
if (circuit_cx(c, 0, 1) != 0) {
    /* handle error */
}

/* Symbolic parameter: RZ(0, theta), where theta is a symbolic expression */
CParameter *theta = param_parse("theta");

if (circuit_rz_param(c, 0, theta) != 0) {
    /* handle error */
}

param_free(theta);
circuit_free(c);
```

---

## See also

- [Parameter](3_parameter.md): creating, evaluating and freeing `CParameter`.
- [Circuit](1_circuit.md) and [Operation Instructions](4_operation_instruction.md): non-unitary instructions such as `circuit_measure`, `circuit_reset`, `circuit_barrier` and `circuit_delay`.
- [Unitary Gates](6_gate_unitary.md): matrix-defined custom unitary gates.
- [Multi-Controlled Gates](7_gate_mc_gate.md): adding control qubits to any standard gate.
- [Circuit to Matrix](13_circuit_to_matrix.md): numeric output of a whole circuit's unitary matrix.
