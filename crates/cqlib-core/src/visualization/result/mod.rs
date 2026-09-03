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

//! SVG-first result/statistics visualization.
//!
//! This module provides histogram and probability-distribution plots for measurement
//! outcomes and other count-based statistics.
//!
//! # Module Structure
//!
//! - [`ResultPlotOptions`]: sorting, coloring, and layout options
//! - [`plot_histogram`]: render an [`crate::device::ExecutionResult`] as raw measured counts
//! - [`plot_distribution`]: render an [`crate::device::ExecutionResult`] as normalized probabilities

mod data;
mod options;
mod plot;

pub use data::{plot_distribution, plot_histogram};
pub use options::ResultPlotOptions;
pub use plot::render_result_plot_to_file;

#[cfg(test)]
#[path = "data_test.rs"]
mod data_test;

#[cfg(test)]
#[path = "options_test.rs"]
mod options_test;

#[cfg(test)]
#[path = "plot_test.rs"]
mod plot_test;
