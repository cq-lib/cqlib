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

//! Multi-Controlled Gate Definitions
//!
//! This module provides [`MCGate`], a type representing gates with multiple
//! control qubits applied to a base [`StandardGate`]. For example, a
//! multi-controlled X gate with 2 controls is the CCX (Toffoli) gate.

use crate::circuit::gate::{StandardGate, gate_matrix};
use crate::circuit::{CircuitError, Parameter};
use alloc::borrow::Cow;
use ndarray::Array2;
use num_complex::Complex;
use smallvec::SmallVec;
use std::fmt;

/// A multi-controlled quantum gate.
///
/// `MCGate` represents a [`StandardGate`] with additional control qubits.
/// The gate applies the base operation only when all control qubits are
/// in the |1⟩ state.
///
/// # Examples
///
/// ```
/// use cqlib_core::circuit::gate::{MCGate, StandardGate};
///
/// // Create a Toffoli-like gate (CCX) using 2 controls and X gate
/// let ccx = MCGate::new(2, StandardGate::X);
/// assert_eq!(ccx.num_ctrl_qubits(), 2);
/// assert_eq!(ccx.num_qubits(), 3); // 2 controls + 1 target
///
/// // Create a multi-controlled Z gate
/// let mcz = MCGate::new(3, StandardGate::Z);
/// assert_eq!(mcz.num_ctrl_qubits(), 3);
/// assert_eq!(mcz.num_qubits(), 4);
/// ```
#[derive(Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Clone)]
pub struct MCGate {
    /// Number of additional control qubits (beyond any inherent controls).
    num_controls: u8,
    /// The base gate to be conditionally applied.
    gate: StandardGate,
}

impl MCGate {
    /// Creates a new multi-controlled gate.
    ///
    /// # Arguments
    ///
    /// * `num_controls` - The number of control qubits to add.
    /// * `gate` - The base [`StandardGate`] to control.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::gate::{MCGate, StandardGate};
    ///
    /// // 3-controlled Hadamard
    /// let mc_h = MCGate::new(3, StandardGate::H);
    ///
    /// // Controlled-RX (1 control added to RX)
    /// let crx = MCGate::new(1, StandardGate::RX);
    /// ```
    pub fn new(num_controls: u8, gate: StandardGate) -> Self {
        Self { num_controls, gate }
    }

    /// Returns the unitary matrix representation of the multi-controlled gate.
    ///
    /// The matrix is constructed by applying the control structure to the
    /// base gate's matrix. For large numbers of controls, this results in
    /// a sparse block-diagonal matrix.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the base gate (if parametric).
    ///
    /// # Returns
    ///
    /// A `Cow` containing the matrix, borrowed if no controls are present.
    pub fn matrix(&self, params: &[f64]) -> Result<Cow<'_, Array2<Complex<f64>>>, CircuitError> {
        let base_matrix = self.gate.matrix(params)?;
        if self.num_controls == 0 {
            return Ok(base_matrix);
        }
        // Construct controlled matrix
        let controlled = gate_matrix::control_matrix(&base_matrix, self.num_controls as usize);
        Ok(Cow::Owned(controlled))
    }

    /// Computes the inverse of the multi-controlled gate.
    ///
    /// The inverse of a controlled gate C(U) is C(U†).
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the base gate.
    ///
    /// # Returns
    ///
    /// `Some((MCGate, parameters))` if the base gate is invertible, `None` otherwise.
    pub fn inverse(&self, params: &[Parameter]) -> Option<(MCGate, SmallVec<[Parameter; 3]>)> {
        // The inverse of a controlled gate C(U) is C(U†).
        let (inv_gate, inv_params) = self.gate.inverse(params)?;
        Some((
            Self {
                num_controls: self.num_controls,
                gate: inv_gate,
            },
            inv_params,
        ))
    }

    /// Returns the total number of control qubits.
    ///
    /// This includes both the explicitly added controls and any inherent
    /// controls in the base gate (e.g., CX has 1 inherent control).
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::gate::{MCGate, StandardGate};
    ///
    /// // Adding 2 controls to X gate
    /// let gate = MCGate::new(2, StandardGate::X);
    /// assert_eq!(gate.num_ctrl_qubits(), 2);
    ///
    /// // Adding 1 control to CX (which already has 1 control)
    /// let gate = MCGate::new(1, StandardGate::CX);
    /// assert_eq!(gate.num_ctrl_qubits(), 2); // 1 added + 1 inherent
    /// ```
    pub fn num_ctrl_qubits(&self) -> usize {
        self.num_controls as usize + self.gate.num_ctrl_qubits()
    }

    /// Returns the total number of qubits (controls + targets).
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::gate::{MCGate, StandardGate};
    ///
    /// // CCX: 2 controls + 1 target = 3 qubits
    /// let ccx = MCGate::new(2, StandardGate::X);
    /// assert_eq!(ccx.num_qubits(), 3);
    /// ```
    pub fn num_qubits(&self) -> usize {
        self.num_controls as usize + self.gate.num_qubits()
    }

    /// Returns the number of parameters required by the base gate.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::gate::{MCGate, StandardGate};
    ///
    /// // Controlled-RX requires 1 parameter
    /// let crx = MCGate::new(1, StandardGate::RX);
    /// assert_eq!(crx.num_params(), 1);
    ///
    /// // Multi-controlled H requires no parameters
    /// let much = MCGate::new(2, StandardGate::H);
    /// assert_eq!(much.num_params(), 0);
    /// ```
    pub fn num_params(&self) -> usize {
        self.gate.num_params()
    }

    /// Returns a reference to the base gate.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::circuit::gate::{MCGate, StandardGate};
    ///
    /// let gate = MCGate::new(2, StandardGate::Z);
    /// assert_eq!(*gate.base_gate(), StandardGate::Z);
    /// ```
    pub fn base_gate(&self) -> &StandardGate {
        &self.gate
    }

    /// Returns the stable operation name used by circuit APIs and statistics.
    pub fn name(&self) -> String {
        if self.num_controls == 0 {
            self.gate.name().to_string()
        } else {
            format!("C{}-{}", self.num_controls, self.gate.name())
        }
    }
}

impl fmt::Display for MCGate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}
