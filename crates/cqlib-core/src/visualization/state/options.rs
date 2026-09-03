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

//! Public options for quantum-state visualization.
//!
//! State plotting APIs accept core QIS state types directly, such as
//! [`crate::qis::Statevector`] and [`crate::qis::DensityMatrix`].

/// Options shared by state-visualization backends.
///
/// Use [`StatePlotOptions::default`] for library defaults, then override only the fields
/// needed for a specific plot family.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::visualization::StatePlotOptions;
///
/// let options = StatePlotOptions {
///     title: Some("State city".to_string()),
///     reverse_bits: true,
///     ..StatePlotOptions::default()
/// };
/// assert!(options.reverse_bits);
/// ```
#[derive(Debug, Clone)]
pub struct StatePlotOptions {
    /// Optional chart title.
    pub title: Option<String>,
    /// Optional colors used by selected plot families.
    pub color: Vec<String>,
    /// Fill opacity for state-city bars.
    pub alpha: f64,
    /// Whether to reverse displayed computational-basis bit order.
    pub reverse_bits: bool,
    /// Optional figure size in inches-like units, scaled to SVG pixels.
    pub figsize: Option<(f64, f64)>,
}

impl Default for StatePlotOptions {
    fn default() -> Self {
        Self {
            title: None,
            color: Vec::new(),
            alpha: 1.0,
            reverse_bits: false,
            figsize: None,
        }
    }
}
