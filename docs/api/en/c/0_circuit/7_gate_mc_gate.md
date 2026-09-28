# Multi-Controlled Gates

This page covers `circuit_multi_control`, the C API interface for adding control qubits to any standard gate; for global conventions on error codes and memory management, see [Overview](../0_overview.md).

---

## circuit_multi_control(ptr, gate_name, controls, controls_len, targets, targets_len, params, params_len)

Appends a multi-controlled standard gate to the circuit: the base gate acts on its target qubits only when every control qubit is in the `1` state.

Parameters:

- `ptr` (`struct CCircuit *`): target circuit handle.
- `gate_name` (`const char *`): name of the base standard gate; see the gate-name list below.
- `controls` (`const uint32_t *`): array of control qubit indices.
- `controls_len` (`uintptr_t`): number of control qubits; may be 0.
- `targets` (`const uint32_t *`): array of qubits the base gate acts on; its length must equal the base gate's own qubit count.
- `targets_len` (`uintptr_t`): length of `targets`.
- `params` (`const struct CParameter *const *`): array of parameter handles for the base gate; its length must equal the base gate's parameter count; pass `NULL, 0` for parameterless gates.
- `params_len` (`uintptr_t`): length of `params`.

The operation's qubit list is `[controls..., targets...]`: the added control qubits come first, followed by the qubits the base gate itself requires; if the base gate carries built-in controls (such as `"CX"`), its internal control qubits keep their original order. For example, with base gate `"X"`, two controls and one target, the list `[c0, c1, t]` means `c0` and `c1` control and `t` is flipped; with base gate `"CX"` and one added control, the three qubits are in order `[new_control, cx_control, cx_target]`.

Returns: `int32_t`, `0` on success; `-1` on NULL input (`ptr` or `gate_name` is NULL, or an array is NULL while its length is non-zero); `-2` on out-of-bounds qubits; `-3` when the base gate cannot be controlled or the qubit/parameter counts mismatch; `-4` on invalid UTF-8 in `gate_name`; `-8` on an unknown gate name.

Parameters are passed as `CParameter` symbolic expressions (numeric angles are written as expressions, e.g. `"pi/2"`) and correspond positionally to the base gate's parameters (for example, `"U"` takes 3, `"RXY"` and `"fSim"` take 2 each, `"RX"` takes 1).

### Supported base gate names

`I`, `H`, `X`, `Y`, `Z`, `S`, `SDG`, `T`, `TDG`, `X2P`, `X2M`, `Y2P`, `Y2M`, `RX`, `RY`, `RZ`, `Phase`, `GPhase`, `U`, `XY`, `XY2P`, `XY2M`, `RXY`, `SWAP`, `RXX`, `RYY`, `RZZ`, `RZX`, `CX`, `CY`, `CZ`, `CCX`, `CRX`, `CRY`, `CRZ`, `fSim` (both `FSIM` and `fSim` are accepted).

### MCX example

```c
#include "cqlib_c.h"

CCircuit *c = circuit_new(4);

/* Doubly controlled X (Toffoli semantics): controls = {0, 1}, target = 2 */
uint32_t ccx_controls[2] = {0, 1};
uint32_t ccx_target[1] = {2};

if (circuit_multi_control(c, "X",
                          ccx_controls, 2,
                          ccx_target, 1,
                          NULL, 0) != 0) {
    /* handle error */
}

/* Triply controlled H */
uint32_t mch_controls[3] = {0, 1, 2};
uint32_t mch_target[1] = {3};

if (circuit_multi_control(c, "H",
                          mch_controls, 3,
                          mch_target, 1,
                          NULL, 0) != 0) {
    /* handle error */
}

/* Singly controlled RZ(theta) with a symbolic angle */
CParameter *theta = param_parse("theta");
const struct CParameter *crz_params[1] = {theta};
uint32_t crz_control[1] = {0};
uint32_t crz_target[1] = {1};

if (circuit_multi_control(c, "RZ",
                          crz_control, 1,
                          crz_target, 1,
                          crz_params, 1) != 0) {
    /* handle error */
}

param_free(theta);
circuit_free(c);
```

---

## Relation to standard controlled gates

Common controlled gates are special cases of `circuit_multi_control` with a specific base gate:

| Standard gate function | Equivalent call |
| --- | --- |
| `circuit_cx(ptr, c, t)` | `circuit_multi_control(ptr, "X", {c}, 1, {t}, 1, NULL, 0)` |
| `circuit_cy(ptr, c, t)` | `circuit_multi_control(ptr, "Y", {c}, 1, {t}, 1, NULL, 0)` |
| `circuit_cz(ptr, c, t)` | `circuit_multi_control(ptr, "Z", {c}, 1, {t}, 1, NULL, 0)` |
| `circuit_ccx(ptr, c1, c2, t)` | `circuit_multi_control(ptr, "X", {c1, c2}, 2, {t}, 1, NULL, 0)` |
| `circuit_crx(ptr, c, t, θ)` | `circuit_multi_control(ptr, "RX", {c}, 1, {t}, 1, {θ}, 1)` |
| `circuit_cry(ptr, c, t, θ)` | `circuit_multi_control(ptr, "RY", {c}, 1, {t}, 1, {θ}, 1)` |
| `circuit_crz(ptr, c, t, θ)` | `circuit_multi_control(ptr, "RZ", {c}, 1, {t}, 1, {θ}, 1)` |

Use `circuit_multi_control` when more control qubits are needed or controls must be added to an arbitrary standard gate; for singly or doubly controlled base gates, call the [standard gate](5_gate_standard.md) functions directly.

---

## See also

- [Standard Gates](5_gate_standard.md): the base gate set and each gate's parameter count.
- [Parameter](3_parameter.md): creating `CParameter` and the expression syntax.
