# Circuit Gates

This page covers circuit gates in the C API: wrapping an already-built circuit into a named, reusable composite gate; for global conventions on error codes and memory management, see [Overview](../0_overview.md).

---

## circuit_to_gate(ptr, name)

Freezes a copy of the circuit into a named composite gate. The source circuit is cloned and stays usable after the call; it may be released afterwards.

Parameters:

- `ptr` (`const struct CCircuit *`): source circuit handle.
- `name` (`const char *`): composite gate name.

Returns: a newly allocated `struct CCircuitGate *`, freed with `circuit_gate_free`; NULL on NULL input, invalid UTF-8 in the name or failure.

When wrapping, the source circuit's free symbols form the composite gate's call signature in order of appearance, queryable through `circuit_gate_signature_params`.

---

## circuit_gate_free(ptr)

Frees a composite gate handle. Passing NULL is allowed; no return value.

---

## circuit_gate_name(ptr)

Returns the gate's name as a newly allocated C string.

Parameters:

- `ptr` (`const struct CCircuitGate *`): composite gate handle.

Returns: `char *`, freed with `cqlib_string_free`; NULL on NULL input or error.

---

## circuit_gate_num_qubits(ptr)

Returns the number of qubits the gate acts on when applied.

Parameters:

- `ptr` (`const struct CCircuitGate *`): composite gate handle.

Returns: `uintptr_t` qubit count; 0 for NULL.

---

## circuit_gate_num_params(ptr)

Returns the number of positional parameters in the gate's call signature.

Parameters:

- `ptr` (`const struct CCircuitGate *`): composite gate handle.

Returns: `uintptr_t` parameter count; 0 for NULL.

---

## circuit_gate_signature_params_len(ptr)

Returns the number of positional parameter names in the gate's call signature; the first step of the two-step output pattern.

Parameters:

- `ptr` (`const struct CCircuitGate *`): composite gate handle.

Returns: `uintptr_t` signature name count; 0 for NULL.

---

## circuit_gate_signature_params(ptr, out, len)

Copies the call signature's parameter names into `out` (two-step output pattern; call `circuit_gate_signature_params_len` first).

Parameters:

- `ptr` (`const struct CCircuitGate *`): composite gate handle.
- `out` (`char **`): output buffer allocated by the caller according to the length; passing NULL only queries the length.
- `len` (`uintptr_t`): number of elements `out` can hold.

Returns: `uintptr_t` total number of signature names; each written string is freed by the caller with `cqlib_string_free`.

---

## circuit_circuit_gate(ptr, gate, qubits, qubits_len, params, params_len)

Appends the composite gate to the circuit, binding `params` positionally to the gate's call signature: the i-th argument replaces the i-th signature symbol name, with all substitutions applied simultaneously.

Parameters:

- `ptr` (`struct CCircuit *`): target circuit handle.
- `gate` (`const struct CCircuitGate *`): composite gate handle.
- `qubits` (`const uint32_t *`): qubit indices in the outer circuit the gate acts on, mapped onto the gate's qubits in array order.
- `qubits_len` (`uintptr_t`): length of `qubits`; must equal `circuit_gate_num_qubits(gate)`.
- `params` (`const struct CParameter *const *`): array of positional parameter handles; pass `NULL, 0` for parameterless gates.
- `params_len` (`uintptr_t`): length of `params`; must equal `circuit_gate_num_params(gate)`.

Returns: `int32_t`, `0` on success; `-1` on NULL input (an array is NULL while its length is non-zero); `-2` on out-of-bounds qubits; `-3` on qubit/parameter count mismatch or application failure.

---

## Example: wrapping a Bell circuit as a gate and reusing it

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build the Bell sub-circuit */
    CCircuit *inner = circuit_new(2);
    circuit_h(inner, 0);
    circuit_cx(inner, 0, 1);

    /* 2. Wrap it into a named composite gate; the source circuit
       is cloned and can be released right away */
    struct CCircuitGate *bell = circuit_to_gate(inner, "Bell");
    circuit_free(inner);

    if (bell == NULL) {
        return 1;
    }

    /* 3. Query gate metadata: num_qubits == 2, num_params == 0 */
    char *name = circuit_gate_name(bell);
    printf("gate: %s, qubits: %zu, params: %zu\n",
           name,
           circuit_gate_num_qubits(bell),
           circuit_gate_num_params(bell));
    cqlib_string_free(name);

    /* 4. Reuse the Bell gate twice in an outer circuit */
    CCircuit *outer = circuit_new(4);
    uint32_t first[2] = {0, 1};
    uint32_t second[2] = {2, 3};

    if (circuit_circuit_gate(outer, bell, first, 2, NULL, 0) != 0) {
        /* handle error */
    }
    if (circuit_circuit_gate(outer, bell, second, 2, NULL, 0) != 0) {
        /* handle error */
    }

    circuit_gate_free(bell);
    circuit_free(outer);
    return 0;
}
```

A parameterized composite gate binds through its signature: the sub-circuit below uses the symbol `theta`, its signature after wrapping is `["theta"]`, and the outer call substitutes `alpha`.

```c
#include <stdlib.h>
#include "cqlib_c.h"

CCircuit *inner = circuit_new(1);
CParameter *theta = param_parse("theta");

circuit_rz_param(inner, 0, theta);
param_free(theta);

struct CCircuitGate *rot = circuit_to_gate(inner, "RzBlock");
circuit_free(inner);

/* Signature query: circuit_gate_signature_params_len(rot) == 1 */
uintptr_t n = circuit_gate_signature_params_len(rot);
char **names = malloc(n * sizeof(char *));
circuit_gate_signature_params(rot, names, n);   /* names[0] == "theta" */
for (uintptr_t i = 0; i < n; ++i) {
    cqlib_string_free(names[i]);
}
free(names);

/* Outer call: bind theta to alpha */
CCircuit *outer = circuit_new(1);
CParameter *alpha = param_parse("alpha");
const struct CParameter *bind[1] = {alpha};
uint32_t q[1] = {0};

circuit_circuit_gate(outer, rot, q, 1, bind, 1);

param_free(alpha);
circuit_gate_free(rot);
circuit_free(outer);
```

---

## See also

- [Circuit](1_circuit.md): circuit creation, appending and composition.
- [Parameter](3_parameter.md): creating and freeing `CParameter`.
- [Unitary Gates](6_gate_unitary.md): matrix-defined custom unitary gates.
- [Circuit to Matrix](13_circuit_to_matrix.md): whole-circuit matrix computation after composite gates are expanded.
