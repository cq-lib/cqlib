// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! SVG-first quantum-state visualization.
//!
//! This module provides plotting routines for pure and mixed quantum states:
//!
//! - [`plot_bloch_vector`]: render one Bloch vector
//! - [`plot_bloch_multivector`]: render one reduced Bloch vector per qubit
//! - [`plot_state_city`]: render real/imaginary density-matrix components
//! - [`plot_state_paulivec`]: render Pauli-basis expectation values
//!
//! # Module Structure
//!
//! - [`StateVisualizationSource`]: adapter trait for core QIS state types
//! - [`StatePlotOptions`]: styling and layout options
//! - [`state_to_density_matrix`]: normalize QIS states into density matrices
//! - [`local_bloch_vectors`]: compute reduced single-qubit Bloch vectors
//!
//! # Examples
//!
//! ```rust
//! use cqlib_core::qis::Statevector;
//! use cqlib_core::visualization::{StatePlotOptions, plot_bloch_multivector};
//! use num_complex::Complex64;
//!
//! let state = Statevector::from_state(
//!     1,
//!     vec![
//!         Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
//!         Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
//!     ],
//! ).unwrap();
//! let svg = plot_bloch_multivector(&state, &StatePlotOptions::default()).unwrap();
//! assert!(svg.contains("<svg"));
//! ```

mod data;
mod math;
mod options;
mod plot;

pub use data::{StateVisualizationSource, state_to_density_matrix};
pub use math::local_bloch_vectors;
pub use options::StatePlotOptions;
pub use plot::{
    plot_bloch_multivector, plot_bloch_vector, plot_state_city, plot_state_paulivec,
    render_state_plot_to_file,
};

#[cfg(test)]
#[path = "data_test.rs"]
mod data_test;

#[cfg(test)]
#[path = "math_test.rs"]
mod math_test;

#[cfg(test)]
#[path = "options_test.rs"]
mod options_test;

#[cfg(test)]
#[path = "plot_test.rs"]
mod plot_test;
