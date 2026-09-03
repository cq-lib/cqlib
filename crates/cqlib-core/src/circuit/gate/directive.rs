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

//! Non-Unitary Circuit Directives
//!
//! This module defines [`Directive`], an enum representing non-unitary operations
//! that can appear in quantum circuits. Unlike unitary gates, these operations
//! are generally irreversible and represent interactions with the classical world
//! (measurement) or hardware-level instructions (barriers, resets).

use std::fmt;

/// Non-unitary operations in a quantum circuit.
///
/// Directives represent operations that cannot be described by unitary matrices
/// and often involve classical outcomes or hardware-specific behavior.
///
/// # Variants
///
/// - `Barrier`: Prevents gate reordering across its boundary during optimization
/// - `Measure`: Collapses qubit state to classical bit
/// - `Reset`: Prepares qubit in |0⟩ state
///
/// # Examples
///
/// ```
/// use cqlib_core::circuit::gate::Directive;
///
/// // Create a barrier directive
/// let barrier = Directive::Barrier;
///
/// // Measurement directive
/// let measure = Directive::Measure;
///
/// // Reset directive
/// let reset = Directive::Reset;
/// ```
#[repr(u8)]
#[derive(Eq, Hash, PartialEq, Debug, Clone, Copy)]
pub enum Directive {
    /// A synchronization barrier preventing gate reordering.
    ///
    /// Ensures that operations before and after the barrier cannot be
    /// reordered during circuit optimization. Useful for timing-critical
    /// sequences or hardware constraints.
    Barrier,
    /// Measurement operation collapsing quantum state to classical information.
    ///
    /// Measures the qubit in the computational basis and stores the
    /// result (0 or 1) in a classical register.
    Measure,
    /// Reset operation preparing qubit in the |0⟩ state.
    ///
    /// Forces the qubit into the ground state regardless of its current state.
    /// This is typically implemented via measurement followed by conditional
    /// X gate or direct initialization.
    Reset,
}

impl fmt::Display for Directive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl Directive {
    /// Returns the stable lowercase operation name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Barrier => "barrier",
            Self::Measure => "measure",
            Self::Reset => "reset",
        }
    }

    /// Returns the inverse of the directive if it exists.
    ///
    /// # Returns
    ///
    /// - `Some(Directive::Barrier)`: Barrier is self-inverse
    /// - `None`: Measure and Reset have no inverse
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::gate::Directive;
    ///
    /// // Barrier is self-inverse
    /// assert_eq!(Directive::Barrier.inverse(), Some(Directive::Barrier));
    ///
    /// // Measurement has no inverse
    /// assert_eq!(Directive::Measure.inverse(), None);
    ///
    /// // Reset has no inverse
    /// assert_eq!(Directive::Reset.inverse(), None);
    /// ```
    pub fn inverse(&self) -> Option<Self> {
        match self {
            Directive::Barrier => Some(Directive::Barrier),
            _ => None,
        }
    }
}
