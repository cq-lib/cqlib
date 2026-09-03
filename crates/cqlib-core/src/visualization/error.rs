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

//! Visualization error types.
//!
//! Error types shared by visualization IR building and rendering backends.

use crate::circuit::error::CircuitError;
use thiserror::Error;

/// Comprehensive error type for the visualization module.
///
/// This enum captures errors that can occur during IR construction, input validation,
/// SVG rendering, and file output across circuit, state, and result visualization.
#[derive(Debug, Error)]
pub enum VisualizationError {
    /// Thrown when circuit preprocessing fails before visualization IR construction.
    #[error("circuit preprocessing failed: {0}")]
    CircuitBuild(#[from] CircuitError),

    /// Thrown when an operation references a qubit that is not present in the circuit qubit list.
    #[error("operation references unknown qubit Q{0}")]
    UnknownQubit(u32),

    /// Thrown when a symbolic parameter index points outside the circuit parameter table.
    #[error("parameter index {index} out of bounds (len={len})")]
    ParameterIndexOutOfBounds {
        /// Invalid parameter index referenced by the operation.
        index: u32,
        /// Number of parameters currently registered on the circuit.
        len: usize,
    },

    /// Thrown when SVG parsing or rasterization fails while converting SVG-first outputs.
    #[error("svg rendering failed: {0}")]
    SvgRenderFailed(String),

    /// Thrown when input data for a visualization routine is invalid or inconsistent.
    #[error("invalid visualization input: {0}")]
    InvalidInput(String),

    /// Thrown when writing visualization output to disk fails.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
