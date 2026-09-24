# Metrics and Entropy (C)

This page covers quantum information metrics (purity, fidelity, trace distance) and entropy/entanglement measures (Von Neumann entropy, Renyi entropy, entanglement entropy, negativity, concurrence, entanglement of formation), taking `CStatevector` or `CDensityMatrix` handles as input. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## Shared Conventions

- Every function returns `int32_t`: 0 on success (value written to `out`), -1 for NULL pointers, other negative values for simulator errors (typically -8: dimension mismatch, non-normalized input, non-2-qubit input).
- Subsystem parameters (`qubits`/`subsys_a`/`sys_a` + `len`): `len` 0 denotes an empty subsystem and is valid; a NULL pointer with `len > 0` returns -1.
- `density_matrix_partial_transpose` returns a new handle; all other functions write values through out pointers.

---

## Purity and Fidelity

### statevector_purity(sv, out)

Computes the purity norm of a statevector (1.0 for a normalized pure state).

Returns: 0 on success; -1 for NULL pointers.

### density_matrix_purity(dm, out)

Computes the purity `Tr(rho^2)` of a density matrix.

Returns: 0 on success; -1 for NULL pointers.

### statevector_fidelity(sv1, sv2, out)

Computes the state fidelity `|<psi|phi>|^2` between two statevectors.

Returns: 0 on success; -1 for NULL pointers.

### statevector_fidelity_pure_mixed(sv, dm, out)

Computes the fidelity `<psi|rho|psi>` between a pure statevector and a density matrix.

Returns: 0 on success; -1 for NULL pointers.

### density_matrix_fidelity(dm1, dm2, out)

Computes the state fidelity `(Tr sqrt(sqrt(rho) sigma sqrt(rho)))^2` between two density matrices.

Returns: 0 on success; -1 for NULL pointers.

---

## Trace Distance

### statevector_trace_distance(sv1, sv2, out)

Computes the trace distance `sqrt(1 - |<psi|phi>|^2)` between two pure states.

Returns: 0 on success; -1 for NULL pointers.

### density_matrix_trace_distance(dm1, dm2, out)

Computes the trace distance `1/2 Tr|rho - sigma|` between two density matrices.

Returns: 0 on success; -1 for NULL pointers.

---

## Entropy

### density_matrix_entropy(dm, out)

Computes the Von Neumann entropy `-Tr(rho log2 rho)` of a density matrix (in bits).

Returns: 0 on success; -1 for NULL pointers.

### density_matrix_linear_entropy(dm, out)

Computes the linear entropy `S_L = 1 - Tr(rho^2)`.

Returns: 0 on success; -1 for NULL pointers.

### density_matrix_renyi_entropy(dm, alpha, out)

Computes the Renyi entropy of order `alpha` (base-2 logarithm). `alpha` must be a positive finite value; values close to 1 fall back to the Von Neumann entropy.

Parameters:

- `alpha` (`double`): Renyi order.

Returns: 0 on success; -1 for NULL pointers; -8 for a non-finite or non-positive `alpha`.

---

## Entanglement Measures

### statevector_entanglement_entropy(sv, subsys_a, len, out)

Computes the bipartite entanglement entropy (in bits) of a pure state with respect to subsystem A: `subsys_a` holds `len` qubit indices.

Returns: 0 on success; -1 for NULL pointers; -8 for invalid qubit indices or dimensions.

### statevector_entanglement_entropy_pure(sv, subsys_a, len, out)

Computes the pure-state bipartite entanglement entropy (in bits, log2 base) of a statevector with respect to subsystem A: `subsys_a` holds `len` qubit indices; for a Bell state with subsystem `{0}` the value is `1.0`.

Returns: 0 on success; -1 for NULL pointers; -8 for invalid qubit indices or dimensions.

### density_matrix_negativity(dm, subsys_a, len, out)

Computes the negativity entanglement measure of a bipartite density matrix: transposes subsystem A and evaluates the trace norm.

Returns: 0 on success; -1 for NULL pointers; -8 for invalid qubit indices or dimensions.

### density_matrix_logarithmic_negativity(dm, sys_a, len, out)

Computes the logarithmic negativity `log2 ||rho^{T_A}||_1` with respect to subsystem A (`sys_a` holds `len` qubit indices).

Returns: 0 on success; -1 for NULL pointers; -8 for invalid qubit indices or dimensions.

### density_matrix_concurrence(dm, out)

Computes the concurrence of a 2-qubit density matrix: 0 for separable states, 1 for maximally entangled states.

Returns: 0 on success; -1 for NULL pointers; -8 when the input is not a 2-qubit system.

### density_matrix_entanglement_of_formation(dm, out)

Computes the entanglement of formation (in bits) of a 2-qubit density matrix, derived from the concurrence.

Returns: 0 on success; -1 for NULL pointers; -8 when the input is not a 2-qubit system.

---

## Partial Transpose

### density_matrix_partial_transpose(dm, qubits, len)

Performs the partial transpose on `qubits` (`len` indices) and returns a new density matrix.

Returns: a new `CDensityMatrix*` on success (free with `density_matrix_free`); NULL on NULL pointers (including a NULL array with `len > 0`) or failure.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Bell state and its density matrix */
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);
    struct CStatevector *sv = statevector_from_circuit(qc);
    struct CDensityMatrix *dm = density_matrix_from_circuit(qc);

    /* 2. Purity and fidelity */
    double purity = 0.0;
    density_matrix_purity(dm, &purity);        /* 1.0: pure state */

    struct CCircuit *qc2 = circuit_new(2);
    circuit_h(qc2, 0);
    circuit_cx(qc2, 0, 1);
    struct CStatevector *sv2 = statevector_from_circuit(qc2);
    double fid = 0.0;
    statevector_fidelity(sv, sv2, &fid);       /* 1.0: same state */

    /* 3. Von Neumann entropy and entanglement entropy */
    double entropy = 0.0;
    density_matrix_entropy(dm, &entropy);     /* 0.0: pure state */

    uint32_t subsystem[1] = {0};
    double entanglement = 0.0;
    statevector_entanglement_entropy(sv, subsystem, 1, &entanglement);  /* 1.0 bit */

    /* 4. Partial transpose and negativity */
    double negativity = 0.0;
    density_matrix_negativity(dm, subsystem, 1, &negativity);  /* 0.5 */

    struct CDensityMatrix *pt = density_matrix_partial_transpose(dm, subsystem, 1);
    if (pt != NULL) {
        density_matrix_free(pt);
    }

    statevector_free(sv2);
    circuit_free(qc2);
    density_matrix_free(dm);
    statevector_free(sv);
    circuit_free(qc);
    return 0;
}
```
