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

//! Circuit visualization pipeline.
//!
//! This module provides a complete visualization pipeline for
//! [`Circuit`](crate::circuit::Circuit): from backend-agnostic IR construction
//! to concrete text/figure rendering.
//!
//! # Core Components
//!
//! - **Layout builder**: [`build_visual_circuit`] converts circuit operations into layered
//!   [`VisualCircuit`] IR.
//! - **Text drawer**: [`circuit_to_text`] renders Unicode box-drawing circuit diagrams.
//! - **Figure drawer**: [`circuit_to_figure`], [`draw_figure_from_visual`], and
//!   [`render_figure_to_file`] generate SVG/PNG outputs.
//!
//! # Examples
//!
//! ```rust
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::visualization::{TextDrawerOptions, circuit_to_text};
//!
//! let mut circuit = Circuit::new(2);
//! circuit.h(Qubit::new(0)).unwrap();
//! circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
//!
//! let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
//! assert!(text.contains("H"));
//! ```

pub mod figure;
pub mod ir;
pub mod layout;
pub mod params;
pub mod style;
pub mod text;

pub use figure::{
    FigureDrawStyle, FigureDrawerOptions, circuit_to_figure, draw_figure_from_visual,
    render_figure_to_file,
};
pub use ir::{
    VisualChildren, VisualCircuit, VisualCondition, VisualControlFlowKind, VisualOpStyle,
    VisualOperation,
};
pub use layout::{VisualBuildOptions, build_visual_circuit};
pub use params::{ParameterDisplayMode, ParameterFormatOptions, ParameterFormatter};
pub use style::GateStyle;
pub use text::{TextDrawerOptions, circuit_to_text, draw_text_from_visual};

#[cfg(test)]
#[path = "figure_test.rs"]
mod figure_test;

#[cfg(test)]
#[path = "params_test.rs"]
mod params_test;

#[cfg(test)]
#[path = "layout_test.rs"]
mod layout_test;

#[cfg(test)]
#[path = "ir_test.rs"]
mod ir_test;

#[cfg(test)]
#[path = "style_test.rs"]
mod style_test;

#[cfg(test)]
#[path = "text_test.rs"]
mod text_test;
