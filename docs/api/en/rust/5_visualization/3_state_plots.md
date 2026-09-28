# Quantum State Visualization

`cqlib_core::visualization::state`

This page covers quantum state visualization: the Bloch vector plot `plot_bloch_vector`, the reduced Bloch vector plot `plot_bloch_multivector`, the density matrix plot `plot_state_city`, the Pauli basis expectation value plot `plot_state_paulivec`, and the plot options `StatePlotOptions`. All drawing entry points accept core state objects directly and return an SVG markup string.

## Import

```rust
use cqlib_core::visualization::{
    StatePlotOptions, plot_bloch_multivector, plot_bloch_vector, plot_state_city,
    plot_state_paulivec,
};
```

---

## `plot_bloch_vector(vector, options) -> Result<String, VisualizationError>`

Draw a single Bloch vector.

Parameters:

- `vector` (`[f64; 3]`): the Bloch coordinates `(x, y, z)`; the components usually fall within `[-1, 1]`. A vector with a norm greater than `1` is projected back onto the unit sphere, and a vector inside the sphere is left unchanged.
- `options` (`&StatePlotOptions`): the plot options.

Returns:

- `Result<String, VisualizationError>`: an SVG markup string. The vector label is fixed to `q0`.

Example:

```rust
use cqlib_core::visualization::{StatePlotOptions, plot_bloch_vector};

let svg = plot_bloch_vector([0.0, 0.0, 1.0], &StatePlotOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## `plot_bloch_multivector(state, options) -> Result<String, VisualizationError>`

Draw one reduced Bloch vector for each qubit. Each vector is computed from the reduced density matrix of that qubit, and labels are written as `q0`, `q1` and so on according to the qubit index.

Parameters:

- `state` (`&S`, `S: StateVisualizationSource + ?Sized`): the input state; both `Statevector` and `DensityMatrix` can be passed directly.
- `options` (`&StatePlotOptions`): the plot options.

Returns:

- `Result<String, VisualizationError>`: an SVG markup string. Multiple spheres are arranged in a grid with at most 4 per row.

Raises:

- `VisualizationError::InvalidInput`: the state buffer length does not match the number of qubits, or the number of qubits is too large and the matrix dimension overflows.

Example:

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::{StatePlotOptions, plot_bloch_multivector};
use num_complex::Complex64;

let state = Statevector::from_state(
    1,
    vec![
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
    ],
)
.unwrap();

let svg = plot_bloch_multivector(&state, &StatePlotOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## `plot_state_city(state, options) -> Result<String, VisualizationError>`

Draw the real and imaginary parts of the density matrix, one panel for each part. Only inputs with a non-zero imaginary part are drawn: when the imaginary parts of all elements are zero, no imaginary part panel is generated in the SVG. The area of each cell scales with `sqrt(|value| / max_abs)`, so that small-magnitude elements remain visible; `max_abs` is the maximum magnitude of the matrix elements, with a lower bound of `1e-12`.

Parameters:

- `state` (`&S`, `S: StateVisualizationSource + ?Sized`): the input state. A pure state is expanded into a density matrix first.
- `options` (`&StatePlotOptions`): the plot options. The fill opacity is controlled by `alpha`.

Returns:

- `Result<String, VisualizationError>`: an SVG markup string, with panel titles `Re[rho]` and `Im[rho]`.

Raises:

- `VisualizationError::InvalidInput`: the state buffer length does not match the number of qubits, or the number of qubits is too large and the matrix dimension overflows.

Example:

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::{StatePlotOptions, plot_state_city};
use num_complex::Complex64;

let state = Statevector::from_state(
    1,
    vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
)
.unwrap();

let svg = plot_state_city(&state, &StatePlotOptions::default()).unwrap();
assert!(svg.contains("Re[rho]"));
```

---

## `plot_state_paulivec(state, options) -> Result<String, VisualizationError>`

Draw Pauli basis expectation values as a bar chart. The labels are tensor products of `I`, `X`, `Y` and `Z`, `4^n` of them in total, where `n` is the number of qubits. Non-negative bars and negative bars use different colors.

Parameters:

- `state` (`&S`, `S: StateVisualizationSource + ?Sized`): the input state.
- `options` (`&StatePlotOptions`): the plot options. The first element of `color` is the color of non-negative bars, and the second element is the color of negative bars.

Returns:

- `Result<String, VisualizationError>`: an SVG markup string. Rotated horizontal axis labels are drawn only when the number of bars does not exceed `64`.

Raises:

- `VisualizationError::InvalidInput`: the state buffer length does not match the number of qubits, or the number of qubits is too large and the matrix dimension overflows.

Example:

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::{StatePlotOptions, plot_state_paulivec};
use num_complex::Complex64;

let state = Statevector::from_state(
    1,
    vec![
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
    ],
)
.unwrap();

let svg = plot_state_paulivec(&state, &StatePlotOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## StatePlotOptions

Quantum state plot options.

```rust
pub struct StatePlotOptions {
    pub title: Option<String>,
    pub color: Vec<String>,
    pub alpha: f64,
    pub reverse_bits: bool,
    pub figsize: Option<(f64, f64)>,
}
```

Fields:

- `title` (`Option<String>`): the plot title. The default is `None`, and no title is drawn.
- `color` (`Vec<String>`): the color scheme used by some plot families. `plot_state_paulivec` takes the first element as the color of non-negative bars, defaulting to `#4569d4`, and the second element as the color of negative bars, defaulting to `#d64b5f`. The remaining plot families use fixed color schemes. The default is an empty vector.
- `alpha` (`f64`): the fill opacity of the `plot_state_city` bars. The default is `1.0`.
- `reverse_bits` (`bool`): whether to reverse the display bit order of the computational basis. The default is `false`. When set to `true`, the row and column labels of the density matrix plot are arranged in reverse order and the matrix elements are mapped back to storage positions by bit order; the label order of the Bloch plots and the Pauli plot is reversed accordingly.
- `figsize` (`Option<(f64, f64)>`): the figure size, in inch-like units, converted into SVG pixels at `100` pixels per unit. The default is `None`, and each plot family computes the canvas size from its own layout. When a size is given, the lower bounds for the width and height of the density matrix plot and the Pauli plot are `4` and `3` respectively, and the lower bounds for the width and height of the Bloch plots are both `3`.

```rust
use cqlib_core::visualization::StatePlotOptions;

let options = StatePlotOptions {
    title: Some("State city".to_string()),
    reverse_bits: true,
    ..StatePlotOptions::default()
};
```

---

## Auxiliary API

### StateVisualizationSource

The input trait accepted by the quantum state drawing entry points. It adapts core state objects into density matrix data rather than introducing another data container.

```rust
pub trait StateVisualizationSource {
    fn num_qubits(&self) -> usize;
    fn density_matrix_data(&self) -> Result<Vec<Complex64>, VisualizationError>;
}
```

Implementors:

- `Statevector`: expanded into a density matrix as `ρ = |ψ⟩⟨ψ|`.
- `DensityMatrix`: returns its data directly.

Implementors must return row-major density matrix data of length `4^num_qubits`; an invalid dimension raises `VisualizationError::InvalidInput`.

### `state_to_density_matrix(state) -> Result<(usize, Vec<Complex64>), VisualizationError>`

Convert a core state object into a row-major density matrix. Returns the number of qubits and data of length `4^num_qubits`.

Raises:

- `VisualizationError::InvalidInput`: the number of qubits is inconsistent with the buffer length, or the number of qubits is too large and the matrix dimension overflows.

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::state_to_density_matrix;
use num_complex::Complex64;

let state = Statevector::from_state(1, vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)])
    .unwrap();
let (num_qubits, rho) = state_to_density_matrix(&state).unwrap();
assert_eq!(num_qubits, 1);
assert_eq!(rho.len(), 4);
```

### `local_bloch_vectors(state) -> Result<Vec<(usize, [f64; 3])>, VisualizationError>`

Compute the reduced Bloch vector of each qubit, returning a list of `(qubit index, [x, y, z])` sorted by qubit index in ascending order.

Raises:

- `VisualizationError::InvalidInput`: the state buffer length does not match the number of qubits.

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::local_bloch_vectors;
use num_complex::Complex64;

let state = Statevector::from_state(1, vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)])
    .unwrap();
let vectors = local_bloch_vectors(&state).unwrap();
assert_eq!(vectors.len(), 1);
assert!((vectors[0].1[2] - 1.0).abs() < 1e-10);
```

---

## Example

### Pure states and mixed states use the same entry points

Both `Statevector` and `DensityMatrix` implement `StateVisualizationSource`, so the same drawing entry points work for both:

```rust
use cqlib_core::qis::{DensityMatrix, Statevector};
use cqlib_core::visualization::{StatePlotOptions, plot_state_city, plot_state_paulivec};
use num_complex::Complex64;

let statevector = Statevector::from_state(
    1,
    vec![
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
    ],
)
.unwrap();
let density_matrix =
    DensityMatrix::from_state(statevector.num_qubits, statevector.data().to_vec()).unwrap();

let sv_svg = plot_state_paulivec(&statevector, &StatePlotOptions::default()).unwrap();
let dm_svg = plot_state_paulivec(&density_matrix, &StatePlotOptions::default()).unwrap();
assert_eq!(sv_svg, dm_svg);
```

### Bloch plot of a multi-qubit state

For a multi-qubit state, one reduced Bloch vector is drawn per qubit, which can be used to observe the local information of each qubit in an entangled state:

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::{StatePlotOptions, plot_bloch_multivector};
use num_complex::Complex64;

let bell = Statevector::from_state(
    2,
    vec![
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
    ],
)
.unwrap();

let svg = plot_bloch_multivector(&bell, &StatePlotOptions::default()).unwrap();
assert!(svg.contains("q0"));
assert!(svg.contains("q1"));
```

---

## Related pages

- To write drawing results out to a file, see [Writing to file and output](5_render_to_file.md).
