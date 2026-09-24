# Quantum State Visualization (C)

This page covers quantum-state visualization: the Bloch-sphere plot `plot_bloch_vector`, the reduced-Bloch-vector plot `plot_bloch_multivector`, the density-matrix plot `plot_state_city`, the Pauli-basis expectation plot `plot_state_paulivec`, the options struct `CStatePlotOptions`, and the numeric state-inspection helpers `local_bloch_vectors` and `state_to_density_matrix`. The plot entries return a heap-allocated SVG markup string freed with `cqlib_string_free`; the numeric helpers write into caller-allocated buffers following the two-step array pattern. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md).

State-based entries come in two variants: the base name accepts a `CStatevector` (a pure state is expanded to a density matrix via `ρ = |ψ⟩⟨ψ|`), while the `_density_matrix` suffix accepts a `CDensityMatrix` (its matrix data is used directly); both share the same `CStatePlotOptions`. For constructing the state handles see [Statevector](../3_qis/1_statevector.md) and [DensityMatrix](../3_qis/2_density_matrix.md).

---

## plot_bloch_vector(x, y, z, options)

Plots a single Bloch vector.

```c
char *plot_bloch_vector(double x, double y, double z,
                       const struct CStatePlotOptions *options);
```

Parameters:

- `x`, `y`, `z` (`double`): the Bloch coordinates `(x, y, z)`, typically within `[-1, 1]`. Vectors longer than `1` are projected back onto the unit sphere; vectors inside the sphere are kept unchanged.
- `options` (`const CStatePlotOptions*`): plot options; NULL uses library defaults.

Returns: a heap-allocated C string (SVG markup), freed with `cqlib_string_free`; NULL on error. The vector label is fixed to `q0`.

---

## plot_bloch_multivector(state, options) / plot_bloch_multivector_density_matrix(state, options)

Plots one reduced Bloch vector per qubit. Each vector is computed from the qubit's reduced density matrix, and the labels follow the qubit index as `q0`, `q1`, and so on; the spheres are arranged in a grid with at most 4 per row.

```c
char *plot_bloch_multivector(const struct CStatevector *state,
                             const struct CStatePlotOptions *options);
char *plot_bloch_multivector_density_matrix(const struct CDensityMatrix *state,
                                            const struct CStatePlotOptions *options);
```

Parameters:

- `state`: the input state handle (`const CStatevector*` for the first, `const CDensityMatrix*` for the second).
- `options` (`const CStatePlotOptions*`): plot options; NULL uses library defaults.

Returns: a heap-allocated C string (SVG markup), freed with `cqlib_string_free`; NULL on error.

Failure scenarios:

- `state` is NULL.
- `options` contains a string that is not valid UTF-8, or the `color` array contains a NULL entry.
- The state buffer length does not match the qubit count, or the qubit count is large enough to overflow the matrix dimension.

---

## plot_state_city(state, options) / plot_state_city_density_matrix(state, options)

Plots the real and imaginary parts of the density matrix, one panel each, titled `Re[rho]` and `Im[rho]`. When every element has zero imaginary part, no imaginary panel is generated. Each cell's area scales with `sqrt(|value| / max_abs)` so that small-magnitude elements stay visible; `max_abs` is the largest element magnitude, lower-bounded by `1e-12`. The bar fill opacity is controlled by `CStatePlotOptions.alpha`.

```c
char *plot_state_city(const struct CStatevector *state,
                      const struct CStatePlotOptions *options);
char *plot_state_city_density_matrix(const struct CDensityMatrix *state,
                                     const struct CStatePlotOptions *options);
```

Parameters:

- `state`: the input state handle (`const CStatevector*` for the first, `const CDensityMatrix*` for the second).
- `options` (`const CStatePlotOptions*`): plot options; NULL uses library defaults.

Returns: a heap-allocated C string (SVG markup), freed with `cqlib_string_free`; NULL on error.

Failure scenarios: same as [plot_bloch_multivector](#plot_bloch_multivectorstate-options--plot_bloch_multivector_density_matrixstate-options).

---

## plot_state_paulivec(state, options) / plot_state_paulivec_density_matrix(state, options)

Plots Pauli-basis expectation values as a bar chart. Labels are tensor products of `I`, `X`, `Y`, `Z`, for `4^n` bars in total where `n` is the qubit count; non-negative and negative bars use different colors. Rotated x-axis labels are drawn only when the bar count does not exceed `64`.

```c
char *plot_state_paulivec(const struct CStatevector *state,
                          const struct CStatePlotOptions *options);
char *plot_state_paulivec_density_matrix(const struct CDensityMatrix *state,
                                         const struct CStatePlotOptions *options);
```

Parameters:

- `state`: the input state handle (`const CStatevector*` for the first, `const CDensityMatrix*` for the second).
- `options` (`const CStatePlotOptions*`): plot options; NULL uses library defaults.

Returns: a heap-allocated C string (SVG markup), freed with `cqlib_string_free`; NULL on error.

Failure scenarios: same as [plot_bloch_multivector](#plot_bloch_multivectorstate-options--plot_bloch_multivector_density_matrixstate-options).

---

## local_bloch_vectors_len(state) / local_bloch_vectors(state, out, len)

Computes one reduced single-qubit Bloch vector per qubit of a statevector — the numeric counterpart of the [plot_bloch_multivector](#plot_bloch_multivectorstate-options--plot_bloch_multivector_density_matrixstate-options) plot, returned as data instead of drawn.

```c
uintptr_t local_bloch_vectors_len(const struct CStatevector *state);
int32_t local_bloch_vectors(const struct CStatevector *state, double *out, uintptr_t len);
```

Two-step array output: `local_bloch_vectors_len` returns the required buffer length `3 * num_qubits` (0 for NULL); allocate that many `double` values and pass the buffer together with the same `len` to `local_bloch_vectors`. The output is laid out flat as `[x0, y0, z0, x1, y1, z1, ...]`: entries `q * 3 .. q * 3 + 3` hold the `<X>`, `<Y>`, `<Z>` expectation values of qubit `q`'s reduced state.

Parameters:

- `state` (`const CStatevector*`): input statevector.
- `out` (`double*`): output buffer of `len` doubles.
- `len` (`uintptr_t`): buffer length; must equal `local_bloch_vectors_len(state)`.

Error codes (fill function):

| Value | Scenario |
| --- | --- |
| `0` | Success. |
| `-1` | `state` or `out` is NULL. |
| `-8` | `len` does not match `local_bloch_vectors_len(state)`, or the state payload is invalid. |

For the Bell state every local Bloch vector vanishes, so all six values are `0`; for `|+>` on qubit 0 and `|0>` on qubit 1 the buffer is `[1, 0, 0, 0, 0, 1]`.

---

## state_to_density_matrix_len(state) / state_to_density_matrix(state, out, len)

Converts a statevector into its density matrix `ρ = |ψ⟩⟨ψ|` — the numeric counterpart of the [plot_state_city](#plot_state_citystate-options--plot_state_city_density_matrixstate-options) plot, returned as data instead of drawn.

```c
uintptr_t state_to_density_matrix_len(const struct CStatevector *state);
int32_t state_to_density_matrix(const struct CStatevector *state, Complex64 *out, uintptr_t len);
```

Two-step array output: `state_to_density_matrix_len` returns the required buffer length `4^num_qubits` (0 for NULL); `state_to_density_matrix` copies that many `Complex64` values into `out`, laid out row-major as an `N × N` matrix with `N = 2^num_qubits`: element `(row, col)` is stored at index `row * N + col`.

Parameters:

- `state` (`const CStatevector*`): input statevector.
- `out` (`Complex64*`): output buffer of `len` values.
- `len` (`uintptr_t`): buffer length; must equal `state_to_density_matrix_len(state)`.

Error codes (fill function):

| Value | Scenario |
| --- | --- |
| `0` | Success. |
| `-1` | `state` or `out` is NULL. |
| `-8` | `len` does not match `state_to_density_matrix_len(state)`, or the state payload is invalid. |

For the two-qubit Bell state the `4 × 4` matrix carries `0.5` on the diagonal corners and on the `(0,3)`/`(3,0)` coherences; every other element is `0`.

---

## CStatePlotOptions

Quantum-state plot options, filled by value; passing NULL for `options` uses library defaults. Strings inside the options must be valid UTF-8, otherwise the plot entry returns NULL.

| Field | Type | Meaning | Value behavior |
| --- | --- | --- | --- |
| `title` | `const char*` | Chart title. | NULL draws no title. |
| `color` | `const char* const*` | Color array; length given by `color_len`. | NULL or `color_len == 0` uses the plot-family defaults; a NULL entry inside the array makes the entry return NULL. |
| `color_len` | `uintptr_t` | Number of entries in `color`. | `0` is treated as no colors provided. |
| `alpha` | `double` | Fill opacity of `plot_state_city` bars. | Takes the given value as-is; the library default is `1.0` (applied only when `options` is NULL — an explicitly passed struct with zero makes the bars fully transparent). |
| `reverse_bits` | `uint8_t` | Whether to reverse the displayed computational-basis bit order. | `0` original order, `1` reversed. When set to `1`, the density-matrix plot's row/column labels are ordered in reverse and matrix elements are mapped back to storage positions by bit order; Bloch and Pauli plots reverse their label order accordingly. |
| `has_figsize` | `uint8_t` | Whether `fig_width` / `fig_height` carry a custom figure size. | `0` ignores width/height (each plot family sizes the canvas by its own layout); non-zero enables them. |
| `fig_width` | `double` | Figure width in inch-like units, converted to SVG pixels at `100` pixels per unit. | Ignored when `has_figsize == 0`. When given, the density-matrix and Pauli plots lower-bound width/height at `4`/`3`, and the Bloch plots lower-bound both at `3`. |
| `fig_height` | `double` | Figure height, same units and conversion as `fig_width`. | Ignored when `has_figsize == 0`; same lower bounds as `fig_width`. |

How `color` is consumed depends on the plot family: `plot_state_paulivec` takes the first entry as the non-negative bar color (default `#4569d4`) and the second entry as the negative-bar color (default `#d64b5f`); the other plot families use fixed colors.

---

## Example

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build a Bell state */
    double inv = 0.7071067811865476;
    Complex64 amplitudes[4] = {
        {inv, 0.0}, {0.0, 0.0}, {0.0, 0.0}, {inv, 0.0},
    };
    struct CStatevector *state = statevector_from_state(2, amplitudes, 4);
    if (state == NULL) {
        return 1;
    }

    /* 2. Plot with default options; every entry returns SVG markup */
    char *svg = plot_bloch_multivector(state, NULL);
    if (svg != NULL) {
        printf("%s\n", svg);
        cqlib_string_free(svg);
    }

    /* 3. Plot with explicit options */
    struct CStatePlotOptions options;
    memset(&options, 0, sizeof(options));
    options.title = "Bell state";
    options.alpha = 0.75;
    options.reverse_bits = 1;
    options.has_figsize = 1;
    options.fig_width = 4.0;
    options.fig_height = 3.0;
    svg = plot_state_city(state, &options);
    if (svg != NULL) {
        printf("%s\n", svg);
        cqlib_string_free(svg);
    }

    /* 4. Density-matrix input works through the _density_matrix variants */
    struct CDensityMatrix *rho = density_matrix_from_state(2, amplitudes, 4);
    if (rho != NULL) {
        svg = plot_state_paulivec_density_matrix(rho, NULL);
        if (svg != NULL) {
            cqlib_string_free(svg);
        }
        density_matrix_free(rho);
    }

    /* 5. Single Bloch vector: vectors longer than the unit
       sphere are clamped, not rejected */
    svg = plot_bloch_vector(0.0, 0.0, 1.0, NULL);
    if (svg != NULL) {
        cqlib_string_free(svg);
    }

    /* 6. Numeric state inspection via the two-step array pattern */
    uintptr_t bloch_len = local_bloch_vectors_len(state);      /* 6 = 3 * 2 */
    double *bloch = malloc(bloch_len * sizeof(double));
    local_bloch_vectors(state, bloch, bloch_len);              /* all zeros for the Bell state */
    free(bloch);

    uintptr_t rho_len = state_to_density_matrix_len(state);    /* 16 = 4^2 */
    Complex64 *rho_buf = malloc(rho_len * sizeof(Complex64));
    state_to_density_matrix(state, rho_buf, rho_len);          /* 4x4 matrix, corners 0.5 */
    free(rho_buf);

    statevector_free(state);
    return 0;
}
```

Plotting the same pure state through `plot_state_paulivec` (statevector variant) and `plot_state_paulivec_density_matrix` (density-matrix variant) yields identical SVG markup. To write the plots out to files, see [File Output](5_render_to_file.md).
