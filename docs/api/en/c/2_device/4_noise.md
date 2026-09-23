# NoiseModel

`CNoiseModel` describes gate noise and readout error models, and can be used together with error mitigation and noisy simulation.

---

## Functions

### noise_model_new()

Create an empty noise model.

Returns:

- `CNoiseModel *`: a heap-allocated model; release it with `noise_model_free`; NULL on failure.

### noise_model_add_single_qubit(model, gate_name, qubit, noise_type, p)

Add a noise channel for a single-qubit gate.

Parameters:

- `gate_name` (`const char *`): the gate name, for example `"H"`.
- `qubit` (`uint32_t`): the target qubit.
- `noise_type` (`uint8_t`): the noise channel label; see the table below for the values (`NOISE_BIT_FLIP` through `NOISE_PHASE_DAMPING`).
- `p` (`double`): the channel parameter; its meaning depends on the channel.

Returns:

- `int32_t`; `0` on success, `-4` for an unknown gate name or channel label, and `-8` for an invalid probability.

### noise_model_add_two_qubit(model, gate_name, q0, q1, noise_type, p)

Add a noise channel for a two-qubit gate.

Parameters:

- `gate_name` (`const char *`): the gate name, for example `"CX"`.
- `q0`, `q1` (`uint32_t`): the two qubits the gate acts on.
- `noise_type` (`uint8_t`): currently only `NOISE_TWO_DEPOLARIZING` is supported.
- `p` (`double`): the depolarizing parameter.

Returns:

- `int32_t`; `0` on success, `-4` for an unknown gate name, and `-8` for a qubit conflict (`q0 == q1`) or an invalid probability.

### noise_model_add_readout(model, qubit, p_0_given_1, p_1_given_0)

Add an asymmetric readout error for a qubit.

Parameters:

- `qubit` (`uint32_t`): the target qubit.
- `p_0_given_1` (`double`): `P(measured 0 | true 1)`.
- `p_1_given_0` (`double`): `P(measured 1 | true 0)`.

Returns:

- `int32_t`; `0` on success, `-8` when a probability is invalid (outside `[0, 1]`).

### noise_model_free(ptr)

Release a noise model. Passing NULL is allowed.

---

## Noise channel labels

Single-qubit channels:

| Constant | Value | Channel | Meaning of the parameter `p` |
| --- | --- | --- | --- |
| `NOISE_BIT_FLIP` | 0 | Bit flip | Flip probability |
| `NOISE_PHASE_FLIP` | 1 | Phase flip | Flip probability |
| `NOISE_DEPOLARIZING` | 2 | Depolarizing | Depolarizing parameter |
| `NOISE_AMPLITUDE_DAMPING` | 3 | Amplitude damping | Damping parameter γ |
| `NOISE_PHASE_DAMPING` | 4 | Phase damping | Scattering probability |

Two-qubit channels:

| Constant | Value | Channel | Meaning of the parameter `p` |
| --- | --- | --- | --- |
| `NOISE_TWO_DEPOLARIZING` | 0 | Two-qubit depolarizing | Depolarizing parameter |

---

## Example

```c
CNoiseModel *noise = noise_model_new();
if (!noise) { return 1; }

/* 单比特去极化：所有 H(0) 之后以 p=0.001 去极化 */
int32_t rc = noise_model_add_single_qubit(noise, "H", 0, NOISE_DEPOLARIZING, 0.001);

/* 双比特去极化 */
rc |= noise_model_add_two_qubit(noise, "CX", 0, 1, NOISE_TWO_DEPOLARIZING, 0.01);

/* 读出误差：真实 1 被测成 0 的概率 2%，真实 0 被测成 1 的概率 1% */
rc |= noise_model_add_readout(noise, 0, 0.02, 0.01);

if (rc != 0) {
    fprintf(stderr, "add noise failed: %d\n", rc);
}

noise_model_free(noise);
```
