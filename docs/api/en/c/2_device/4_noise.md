# NoiseModel (C)

`CNoiseModel` is an opaque handle to a noise model that records noise channels per standard-gate + qubit combination and readout errors per qubit; one gate + qubit combination can carry multiple channels. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md).

---

## Channel constants

Single-qubit channel tags (used for the `noise_type` argument of `noise_model_add_single_qubit` and for `CSingleQubitNoise.tag`):

| Constant | Value | Channel | Parameter meaning |
| --- | --- | --- | --- |
| `NOISE_BIT_FLIP` | 0 | BitFlip | flip probability p |
| `NOISE_PHASE_FLIP` | 1 | PhaseFlip | flip probability p |
| `NOISE_DEPOLARIZING` | 2 | Depolarizing | depolarizing parameter p |
| `NOISE_AMPLITUDE_DAMPING` | 3 | AmplitudeDamping | damping parameter gamma |
| `NOISE_PHASE_DAMPING` | 4 | PhaseDamping | scattering probability |
| `NOISE_PAULI` | 5 | Pauli channel | probabilities `px`/`py`/`pz` (sum <= 1) |

Two-qubit channel tags (`NOISE_TWO_DEPOLARIZING` for `noise_model_add_two_qubit`; all of them for `CTwoQubitNoise.tag`):

| Constant | Value | Channel | Parameter meaning |
| --- | --- | --- | --- |
| `NOISE_TWO_DEPOLARIZING` | 0 | Depolarizing | depolarizing parameter p |
| `NOISE_TWO_INDEPENDENT` | 1 | Independent | per-qubit channels in q0/q1_noise |
| `NOISE_TWO_CORRELATED_PAULI` | 2 | CorrelatedPauli | probability p + `NOISE_PAULI_OP_*` operators |

Pauli operator tags for the correlated-Pauli channel (`CTwoQubitNoise.op_q0` / `op_q1`): `NOISE_PAULI_OP_I` = 0, `NOISE_PAULI_OP_X` = 1, `NOISE_PAULI_OP_Y` = 2, `NOISE_PAULI_OP_Z` = 3.

---

## Snapshot structs

### CSingleQubitNoise

Snapshot of a single-qubit noise channel, written by `noise_model_get_single_qubit_errors`. `tag` selects the channel; `p` carries the parameter for tags 0-4 and `px`/`py`/`pz` the Pauli probabilities for tag 5; fields that do not apply to the tagged channel are zero.

| Field | Type | Meaning |
| --- | --- | --- |
| `tag` | `uint8_t` | One of the single-qubit `NOISE_*` tags (0-5). |
| `p` | `double` | Channel probability for tags 0-4. |
| `px` | `double` | Pauli-X probability for tag 5. |
| `py` | `double` | Pauli-Y probability for tag 5. |
| `pz` | `double` | Pauli-Z probability for tag 5. |

### CTwoQubitNoise

Snapshot of a two-qubit noise channel, written by `noise_model_get_two_qubit_errors`. Fields that do not apply to the tagged channel are zero.

| Field | Type | Meaning |
| --- | --- | --- |
| `tag` | `uint8_t` | One of the `NOISE_TWO_*` tags. |
| `p` | `double` | Channel probability for the depolarizing and correlated-Pauli tags. |
| `q0_noise` | `struct CSingleQubitNoise` | Channel applied to q0 for the independent tag. |
| `q1_noise` | `struct CSingleQubitNoise` | Channel applied to q1 for the independent tag. |
| `op_q0` | `uint8_t` | Pauli operator applied to q0 for the correlated tag (`NOISE_PAULI_OP_*`). |
| `op_q1` | `uint8_t` | Pauli operator applied to q1 for the correlated tag (`NOISE_PAULI_OP_*`). |

---

## Construction and release

### noise_model_new()

Creates a new empty noise model.

Returns: a newly allocated `CNoiseModel*`.

### noise_model_free(ptr)

Frees a `CNoiseModel`; NULL is allowed.

---

## Adding channels

### noise_model_add_single_qubit(ptr, gate_name, qubit, noise_type, p)

Adds a single-qubit noise channel to the standard gate `gate_name` acting on `qubit`.

Parameters:

- `gate_name` (`const char*`): standard-gate name.
- `qubit` (`uint32_t`): qubit ID.
- `noise_type` (`uint8_t`): one of the `NOISE_*` single-qubit tags (0-4).
- `p` (`double`): channel parameter.

Returns: 0 on success; -1 on NULL; -4 for an unknown gate name or invalid UTF-8; -8 for an unknown noise tag or invalid probabilities.

### noise_model_add_single_qubit_pauli(ptr, gate_name, qubit, px, py, pz)

Adds a general single-qubit Pauli channel to the standard gate `gate_name` acting on `qubit`, with X/Y/Z probabilities `px`, `py`, `pz` (each >= 0, sum <= 1).

Parameters:

- `px` / `py` / `pz` (`double`): Pauli-X/Y/Z probabilities.

Returns: 0 on success; -1 on NULL; -4 for an unknown gate name; -8 for invalid probabilities.

### noise_model_add_two_qubit(ptr, gate_name, q0, q1, noise_type, p)

Adds a two-qubit depolarizing channel to the standard gate `gate_name` acting on the ordered pair `(q0, q1)`. `noise_type` accepts only `NOISE_TWO_DEPOLARIZING` (0).

Parameters:

- `q0` / `q1` (`uint32_t`): qubit IDs.
- `noise_type` (`uint8_t`): `NOISE_TWO_DEPOLARIZING`.
- `p` (`double`): depolarizing parameter.

Returns: 0 on success; -1 on NULL; -4 for an unknown gate name; -8 when the qubits collide or the probability is invalid.

### noise_model_add_readout(ptr, qubit, p_0_given_1, p_1_given_0)

Adds an asymmetric readout error for `qubit`.

Parameters:

- `qubit` (`uint32_t`): qubit ID.
- `p_0_given_1` (`double`): P(measure 0 | true 1).
- `p_1_given_0` (`double`): P(measure 1 | true 0).

Returns: 0 on success; -8 for invalid probabilities.

---

## Querying channels

### noise_model_get_single_qubit_errors_len(ptr, gate_name, qubit)

Returns the number of single-qubit noise channels recorded for the standard gate `gate_name` on `qubit`.

Returns: the total count; 0 for NULL, an unknown gate name, or absent entries.

### noise_model_get_single_qubit_errors(ptr, gate_name, qubit, out, len)

Two-step output of those channels, paired with `noise_model_get_single_qubit_errors_len`; each entry is a `CSingleQubitNoise` snapshot whose valid fields depend on its `tag`. Returns the total count.

### noise_model_get_two_qubit_errors_len(ptr, gate_name, q0, q1)

Returns the number of two-qubit noise channels recorded for the standard gate `gate_name` on the ordered pair `(q0, q1)`.

Returns: the total count; 0 for NULL, an unknown gate name, colliding qubits, or absent entries.

### noise_model_get_two_qubit_errors(ptr, gate_name, q0, q1, out, len)

Two-step output of those channels, paired with `noise_model_get_two_qubit_errors_len`; each entry is a `CTwoQubitNoise` snapshot whose valid fields depend on its `tag`. Returns the total count.

### noise_model_get_readout_error(ptr, qubit, out_p_0_given_1, out_p_1_given_0)

Writes the readout error recorded for `qubit` to the out parameters.

Parameters:

- `out_p_0_given_1` (`double*`): receives P(measure 0 | true 1).
- `out_p_1_given_0` (`double*`): receives P(measure 1 | true 0).

Returns: 0 on success; -1 on NULL; -8 when no readout error is recorded for the qubit.

---

## Channel validity and Kraus operators

The entry points below address one recorded single-qubit channel by its `(gate_name, qubit, index)` key; `index` is bounded by `noise_model_get_single_qubit_errors_len`. Kraus operators are transferred as `Complex64` re/im pairs (a `struct Complex64` with `re` / `im` fields, defined in `cqlib_c.h`).

### noise_model_is_valid(ptr, gate_name, qubit, index)

Reports whether the single-qubit channel recorded for the standard gate `gate_name` on `qubit` at `index` carries valid noise parameters.

Parameters:

- `gate_name` (`const char*`): standard-gate name.
- `qubit` (`uint32_t`): qubit ID.
- `index` (`uintptr_t`): channel index.

Returns: 1 when valid; 0 when not; -1 on NULL; -4 for an unknown gate name; -8 when no channel exists at that key/index.

### noise_model_to_kraus_len(ptr, gate_name, qubit, index)

Returns the total number of `Complex64` elements across all Kraus operators of the channel; each operator contributes `dim * dim` elements (a single-qubit operator is `2 * 2 = 4`).

Returns: the total element count; 0 for NULL, an unknown gate name, or an absent key/index.

### noise_model_to_kraus(ptr, gate_name, qubit, index, out, len)

Two-step output of the Kraus operators, paired with `noise_model_to_kraus_len`: the operators are written one after another, each in row-major order as `dim * dim` `Complex64` re/im pairs.

Parameters:

- `out` (`Complex64*`): receives the elements; NULL only queries the length.
- `len` (`uintptr_t`): buffer capacity in elements; a smaller value copies only the first `len` elements.

Returns: the total element count; 0 for NULL, an unknown gate name, or an absent key/index.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CNoiseModel *nm = noise_model_new();

    /* single-qubit channel: bit flip on the X gate */
    int32_t rc = noise_model_add_single_qubit(nm, "X", 0, NOISE_BIT_FLIP, 0.005);

    /* single-qubit Pauli channel */
    rc = noise_model_add_single_qubit_pauli(nm, "X", 0, 0.001, 0.002, 0.003);

    /* two-qubit depolarizing channel: CX gate on (0, 1) */
    rc = noise_model_add_two_qubit(nm, "CX", 0, 1, NOISE_TWO_DEPOLARIZING, 0.02);

    /* readout error */
    rc = noise_model_add_readout(nm, 0, 0.02, 0.01);

    /* query the single-qubit channels */
    uintptr_t n = noise_model_get_single_qubit_errors_len(nm, "X", 0);  /* 2 */
    struct CSingleQubitNoise chans[2];
    noise_model_get_single_qubit_errors(nm, "X", 0, chans, n);
    /* chans[0]: tag == NOISE_BIT_FLIP, p == 0.005
       chans[1]: tag == NOISE_PAULI, px/py/pz == 0.001/0.002/0.003 */

    /* query the two-qubit channels */
    uintptr_t m = noise_model_get_two_qubit_errors_len(nm, "CX", 0, 1);  /* 1 */
    struct CTwoQubitNoise tchans[1];
    noise_model_get_two_qubit_errors(nm, "CX", 0, 1, tchans, m);
    /* tchans[0]: tag == NOISE_TWO_DEPOLARIZING, p == 0.02 */

    /* query the readout error */
    double p01 = 0.0, p10 = 0.0;
    rc = noise_model_get_readout_error(nm, 0, &p01, &p10);  /* 0, 0.02 / 0.01 */

    /* channel validity and Kraus operators */
    rc = noise_model_add_single_qubit(nm, "H", 0, NOISE_DEPOLARIZING, 0.1);
    int32_t valid = noise_model_is_valid(nm, "H", 0, 0);       /* 1 */
    uintptr_t klen = noise_model_to_kraus_len(nm, "H", 0, 0);  /* 16: four 2x2 operators */
    Complex64 kraus[16];
    noise_model_to_kraus(nm, "H", 0, 0, kraus, klen);
    /* kraus[0..4]: I*sqrt(0.9) row-major (kraus[0].re == sqrt(0.9))
       kraus[4..16]: X, Y, Z scaled by sqrt(0.1/3) */

    /* amplitude damping on X@0 at index 2 (after bit flip and Pauli) */
    rc = noise_model_add_single_qubit(nm, "X", 0, NOISE_AMPLITUDE_DAMPING, 0.25);
    klen = noise_model_to_kraus_len(nm, "X", 0, 2);            /* 8: two 2x2 operators */

    /* error codes */
    int32_t absent = noise_model_is_valid(nm, "H", 1, 0);      /* -8: no channel at that key */
    int32_t unknown = noise_model_is_valid(nm, "nope", 0, 0);  /* -4: unknown gate */

    noise_model_free(nm);
    return 0;
}
```
