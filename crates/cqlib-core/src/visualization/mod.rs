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

//! Visualization module for circuits, quantum states, and measurement results.
//!
//! This module provides SVG-first rendering pipelines for:
//! - [`circuit`]: circuit diagrams (Unicode text and figure backends)
//! - [`state`]: Bloch vectors, state-city, and Pauli expectation plots
//! - [`result`]: histograms and probability distributions
//!
//! # Module Structure
//!
//! - [`circuit`]: IR builder, text drawer, and figure drawer
//! - [`state`]: quantum-state visualization
//! - [`result`]: result/statistics visualization
//! - [`error`]: shared visualization error types
//!
//! # Examples
//!
//! ```rust
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::visualization::{FigureDrawerOptions, circuit_to_figure};
//!
//! let mut circuit = Circuit::new(2);
//! circuit.h(Qubit::new(0)).unwrap();
//! circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
//!
//! let svg = circuit_to_figure(&circuit, &FigureDrawerOptions::default()).unwrap();
//! assert!(svg.contains("<svg"));
//! ```

pub mod circuit;
pub mod error;
pub mod result;
pub mod state;
pub(crate) mod svg;

// Re-export circuit visualization APIs at `visualization::*` for ergonomic use.
pub use circuit::*;
pub use error::VisualizationError;
pub use result::*;
pub use state::*;

#[cfg(test)]
mod test_utils;

#[cfg(test)]
#[path = "error_test.rs"]
mod error_test;
