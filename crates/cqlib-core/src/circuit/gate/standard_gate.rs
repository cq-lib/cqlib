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

//! Standard Quantum Gate Definitions
//!
//! This module defines the `StandardGate` enum, which enumerates all quantum logic gates
//! natively supported by the Cqlib core. It acts as the "Instruction Set Architecture" (ISA)
//! for the quantum virtual machine.
//!
//! # Key Components
//!
//! - [`StandardGate`]: The central enum representing gates like `H`, `CX`, `RX`, etc.
//! - Gate Properties: Metadata such as qubit count, parameter count, and matrix representations.

use crate::circuit::gate::gate_matrix;
use crate::circuit::{CircuitError, Parameter};
use ndarray::prelude::*;
use num::complex::Complex;
use smallvec::{SmallVec, smallvec};
use std::borrow::Cow;
use std::f64::consts::PI;
use std::fmt;
use std::hash::Hash;

/// Represents the set of standard quantum logic gates supported natively by Cqlib.
///
/// This enum serves as the fundamental identifier for quantum operations that have
/// a defined behavior and matrix representation within the core library. It includes:
///
/// - **Pauli Gates**: $I, X, Y, Z$
/// - **Clifford Gates**: $H, S, T, S^\dagger, T^\dagger$
/// - **Parametric Rotations**: $RX(\theta), RY(\theta), RZ(\theta), U(\theta, \phi, \lambda)$
/// - **Two-Qubit Gates**: $CX, CZ, SWAP, FSim$
/// - **Multi-Controlled Gates**: $CCX$ (Toffoli)
///
/// # Design Note
/// `StandardGate` is designed to be lightweight (`Copy`, `repr(u8)`) for efficient
/// storage and transmission.
#[repr(u8)]
#[derive(Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Default, Clone, Copy)]
pub enum StandardGate {
    /// Identity gate (No-operation).
    #[default]
    I = 0,
    /// Hadamard gate. Superposition creator.
    H = 1,

    // --- Single Qubit Rotations ---
    /// Rotation around X-axis: $R_x(\theta) = e^{-i\theta X/2}$.
    RX,
    /// Ising XX interaction gate (two-qubit parametric).
    RXX,
    /// Rotation around an axis in the X-Y plane.
    RXY,
    /// Rotation around Y-axis: $R_y(\theta) = e^{-i\theta Y/2}$.
    RY,
    /// Ising YY interaction gate.
    RYY,
    /// Rotation around Z-axis: $R_z(\theta) = e^{-i\theta Z/2}$.
    RZ,
    /// Ising ZX interaction gate.
    RZX,
    /// Ising ZZ interaction gate.
    RZZ,

    // --- Clifford & Phase Gates ---
    /// S gate (Phase gate, $\sqrt{Z}$).
    S,
    /// S-dagger gate ($S^\dagger$).
    SDG,
    /// SWAP gate (exchanges states of two qubits).
    SWAP,
    /// T gate ($\\sqrt{S}$).
    T,
    /// T-dagger gate ($T^\dagger$).
    TDG,
    /// Generic single-qubit unitary $U(\theta, \phi, \lambda)$.
    U,

    // --- Pauli Gates ---
    /// Pauli-X gate (Bit-flip, NOT).
    X,
    /// Single-qubit pi rotation about an axis in the XY plane.
    XY,
    /// $\\sqrt{X}$ gate (SX).
    X2P,
    /// $\\sqrt{X}^\dagger$ gate (SXdg).
    X2M,
    /// Positive half-pi single-qubit rotation about an axis in the XY plane.
    XY2P,
    /// Negative half-pi single-qubit rotation about an axis in the XY plane.
    XY2M,
    /// Pauli-Y gate (Bit-phase-flip).
    Y,
    /// $\\sqrt{Y}$ gate.
    Y2P,
    /// $\\sqrt{Y}^\dagger$ gate.
    Y2M,
    /// Pauli-Z gate (Phase-flip).
    Z,

    // --- Global/Relative Phase ---
    /// Phase shift gate $P(\lambda)$.
    Phase,
    /// Zero-qubit global phase marker.
    ///
    /// `GPhase` is not applied to any qubit. Whole-circuit global phase should
    /// normally be represented by [`crate::circuit::Circuit::global_phase`];
    /// `GPhase` operations are mainly useful as import or intermediate IR
    /// markers, including body-local phase inside control-flow operations.
    GPhase,

    // --- Controlled Gates ---
    /// Controlled-X gate (CNOT).
    CX,
    /// Toffoli gate (Controlled-Controlled-X).
    CCX,
    /// Controlled-Y gate.
    CY,
    /// Controlled-Z gate.
    CZ,
    /// Controlled-RX rotation.
    CRX,
    /// Controlled-RY rotation.
    CRY,
    /// Controlled-RZ rotation.
    CRZ,

    // --- Simulation Gates ---
    /// Fermionic Simulation gate (fSim).
    FSIM,
}

// Gate metadata lookup table.
//
// Maps each StandardGate variant to
// (num_controls, num_targets, num_params). The table index must exactly match
// the StandardGate discriminant values; do not reorder entries without
// updating the enum definition.
const GATE_INFO_TABLE: [(u8, u8, u8); 36] = [
    (0, 1, 0), // I
    (0, 1, 0), // H
    (0, 1, 1), // RX
    (0, 2, 1), // RXX
    (0, 1, 2), // RXY
    (0, 1, 1), // RY
    (0, 2, 1), // RYY
    (0, 1, 1), // RZ
    (0, 2, 1), // RZX
    (0, 2, 1), // RZZ
    (0, 1, 0), // S
    (0, 1, 0), // SDG
    (0, 2, 0), // SWAP
    (0, 1, 0), // T
    (0, 1, 0), // TDG
    (0, 1, 3), // U
    (0, 1, 0), // X
    (0, 1, 1), // XY
    (0, 1, 0), // X2P
    (0, 1, 0), // X2M
    (0, 1, 1), // XY2P
    (0, 1, 1), // XY2M
    (0, 1, 0), // Y
    (0, 1, 0), // Y2P
    (0, 1, 0), // Y2M
    (0, 1, 0), // Z
    (0, 1, 1), // Phase
    (0, 0, 1), // GPhase
    (1, 1, 0), // CX
    (2, 1, 0), // CCX
    (1, 1, 0), // CY
    (1, 1, 0), // CZ
    (1, 1, 1), // CRX
    (1, 1, 1), // CRY
    (1, 1, 1), // CRZ
    (0, 2, 2), // FSIM
];

const ALL_STANDARD_GATES: [StandardGate; 36] = [
    StandardGate::I,
    StandardGate::H,
    StandardGate::RX,
    StandardGate::RXX,
    StandardGate::RXY,
    StandardGate::RY,
    StandardGate::RYY,
    StandardGate::RZ,
    StandardGate::RZX,
    StandardGate::RZZ,
    StandardGate::S,
    StandardGate::SDG,
    StandardGate::SWAP,
    StandardGate::T,
    StandardGate::TDG,
    StandardGate::U,
    StandardGate::X,
    StandardGate::XY,
    StandardGate::X2P,
    StandardGate::X2M,
    StandardGate::XY2P,
    StandardGate::XY2M,
    StandardGate::Y,
    StandardGate::Y2P,
    StandardGate::Y2M,
    StandardGate::Z,
    StandardGate::Phase,
    StandardGate::GPhase,
    StandardGate::CX,
    StandardGate::CCX,
    StandardGate::CY,
    StandardGate::CZ,
    StandardGate::CRX,
    StandardGate::CRY,
    StandardGate::CRZ,
    StandardGate::FSIM,
];

impl fmt::Display for StandardGate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl StandardGate {
    /// Returns the stable operation name used by circuit APIs and statistics.
    pub const fn name(self) -> &'static str {
        match self {
            Self::I => "I",
            Self::H => "H",
            Self::RX => "RX",
            Self::RXX => "RXX",
            Self::RXY => "RXY",
            Self::RY => "RY",
            Self::RYY => "RYY",
            Self::RZ => "RZ",
            Self::RZX => "RZX",
            Self::RZZ => "RZZ",
            Self::S => "S",
            Self::SDG => "SDG",
            Self::SWAP => "SWAP",
            Self::T => "T",
            Self::TDG => "TDG",
            Self::U => "U",
            Self::X => "X",
            Self::XY => "XY",
            Self::X2P => "X2P",
            Self::X2M => "X2M",
            Self::XY2P => "XY2P",
            Self::XY2M => "XY2M",
            Self::Y => "Y",
            Self::Y2P => "Y2P",
            Self::Y2M => "Y2M",
            Self::Z => "Z",
            Self::Phase => "Phase",
            Self::GPhase => "GPhase",
            Self::CX => "CX",
            Self::CCX => "CCX",
            Self::CY => "CY",
            Self::CZ => "CZ",
            Self::CRX => "CRX",
            Self::CRY => "CRY",
            Self::CRZ => "CRZ",
            Self::FSIM => "FSIM",
        }
    }

    /// Returns all standard gates in enum discriminant order.
    pub fn all() -> &'static [Self] {
        &ALL_STANDARD_GATES
    }

    /// Returns the unitary matrix representation of the gate.
    ///
    /// The matrix is returned as a `Cow<Array2<Complex<f64>>>`.
    /// - Returns `Cow::Borrowed` for static gates (like H, X, CX) to avoid allocation.
    /// - Returns `Cow::Owned` for parametric gates (like RX, U) where the matrix is computed on the fly.
    ///
    /// # Arguments
    ///
    /// * `params` - A slice of floating-point parameters associated with the gate.
    ///   For non-parametric gates, this can be empty. For parametric gates, it must
    ///   contain the correct number of values (e.g., 1 for RX, 3 for U).
    ///
    /// # Panics
    ///
    /// Panics if `params` does not contain enough values for the specific gate type.
    pub fn matrix(&self, params: &[f64]) -> Result<Cow<'_, Array2<Complex<f64>>>, CircuitError> {
        // Validate all parameters are finite
        for (i, &p) in params.iter().enumerate() {
            if !p.is_finite() {
                return Err(CircuitError::InvalidParameterValue(i, p));
            }
        }

        Ok(match self {
            Self::H => Cow::Borrowed(&gate_matrix::H_GATE),
            Self::I => Cow::Borrowed(&gate_matrix::I_GATE),
            Self::S => Cow::Borrowed(&gate_matrix::S_GATE),
            Self::SDG => Cow::Borrowed(&gate_matrix::SDG_GATE),
            Self::T => Cow::Borrowed(&gate_matrix::T_GATE),
            Self::TDG => Cow::Borrowed(&gate_matrix::TDG_GATE),
            Self::X => Cow::Borrowed(&gate_matrix::X_GATE),
            Self::Y => Cow::Borrowed(&gate_matrix::Y_GATE),
            Self::Z => Cow::Borrowed(&gate_matrix::Z_GATE),
            Self::X2P => Cow::Borrowed(&gate_matrix::X2P_GATE),
            Self::X2M => Cow::Borrowed(&gate_matrix::X2M_GATE),
            Self::Y2P => Cow::Borrowed(&gate_matrix::Y2P_GATE),
            Self::Y2M => Cow::Borrowed(&gate_matrix::Y2M_GATE),
            Self::SWAP => Cow::Borrowed(&gate_matrix::SWAP_GATE),
            Self::CX => Cow::Borrowed(&gate_matrix::CX_GATE),
            Self::CY => Cow::Borrowed(&gate_matrix::CY_GATE),
            Self::CZ => Cow::Borrowed(&gate_matrix::CZ_GATE),
            Self::CCX => Cow::Borrowed(&gate_matrix::CCX_GATE),

            Self::RX => Cow::Owned(gate_matrix::rx_gate(params[0])),
            Self::RY => Cow::Owned(gate_matrix::ry_gate(params[0])),
            Self::RZ => Cow::Owned(gate_matrix::rz_gate(params[0])),
            Self::Phase => Cow::Owned(gate_matrix::phase_gate(params[0])),
            Self::GPhase => Cow::Owned(gate_matrix::global_phase_gate(params[0])),
            Self::RXX => Cow::Owned(gate_matrix::rxx_gate(params[0])),
            Self::RYY => Cow::Owned(gate_matrix::ryy_gate(params[0])),
            Self::RZZ => Cow::Owned(gate_matrix::rzz_gate(params[0])),
            Self::RZX => Cow::Owned(gate_matrix::rzx_gate(params[0])),
            Self::CRX => Cow::Owned(gate_matrix::crx_gate(params[0])),
            Self::CRY => Cow::Owned(gate_matrix::cry_gate(params[0])),
            Self::CRZ => Cow::Owned(gate_matrix::crz_gate(params[0])),
            Self::XY => Cow::Owned(gate_matrix::xy_gate(params[0])),
            Self::XY2P => Cow::Owned(gate_matrix::xy2p_gate(params[0])),
            Self::XY2M => Cow::Owned(gate_matrix::xy2m_gate(params[0])),

            Self::RXY => Cow::Owned(gate_matrix::rxy_gate(params[0], params[1])),
            Self::FSIM => Cow::Owned(gate_matrix::fsim_gate(params[0], params[1])),

            Self::U => Cow::Owned(gate_matrix::u_gate(params[0], params[1], params[2])),
        })
    }

    /// Computes the inverse (Hermitian conjugate) of the gate.
    ///
    /// # Arguments
    ///
    /// * `params` - The parameters of the gate instance to be inverted.
    ///
    /// # Returns
    ///
    /// Returns an `Option` containing a tuple:
    /// - `StandardGate`: The type of the inverse gate.
    /// - `SmallVec<[Parameter; 3]>`: The transformed parameters for the inverse gate.
    ///
    /// Returns `None` if the inverse cannot be represented as a StandardGate (rare).
    pub fn inverse(
        &self,
        params: &[Parameter],
    ) -> Option<(StandardGate, SmallVec<[Parameter; 3]>)> {
        match self {
            // Self-inverse gates (U = U†)
            Self::H => Some((Self::H, smallvec![])),
            Self::I => Some((Self::I, smallvec![])),
            Self::X => Some((Self::X, smallvec![])),
            Self::Y => Some((Self::Y, smallvec![])),
            Self::Z => Some((Self::Z, smallvec![])),
            Self::CX => Some((Self::CX, smallvec![])),
            Self::CCX => Some((Self::CCX, smallvec![])),
            Self::CY => Some((Self::CY, smallvec![])),
            Self::CZ => Some((Self::CZ, smallvec![])),
            Self::SWAP => Some((Self::SWAP, smallvec![])),

            // Paired gates (A = B†)
            Self::S => Some((Self::SDG, smallvec![])),
            Self::SDG => Some((Self::S, smallvec![])),
            Self::T => Some((Self::TDG, smallvec![])),
            Self::TDG => Some((Self::T, smallvec![])),
            Self::X2P => Some((Self::X2M, smallvec![])),
            Self::X2M => Some((Self::X2P, smallvec![])),
            Self::Y2P => Some((Self::Y2M, smallvec![])),
            Self::Y2M => Some((Self::Y2P, smallvec![])),
            Self::XY2P => Some((Self::XY2M, smallvec![params[0].clone()])),
            Self::XY2M => Some((Self::XY2P, smallvec![params[0].clone()])),

            // Parametric gates: usually negate the parameter
            // RX(theta)† = RX(-theta)
            Self::RX => Some((Self::RX, smallvec![-1.0 * params[0].clone()])),
            Self::RY => Some((Self::RY, smallvec![-1.0 * params[0].clone()])),
            Self::RZ => Some((Self::RZ, smallvec![-1.0 * params[0].clone()])),
            Self::Phase => Some((Self::Phase, smallvec![-1.0 * params[0].clone()])),
            Self::GPhase => Some((Self::GPhase, smallvec![-1.0 * params[0].clone()])),
            Self::RXX => Some((Self::RXX, smallvec![-1.0 * params[0].clone()])),
            Self::RYY => Some((Self::RYY, smallvec![-1.0 * params[0].clone()])),
            Self::RZZ => Some((Self::RZZ, smallvec![-1.0 * params[0].clone()])),
            Self::RZX => Some((Self::RZX, smallvec![-1.0 * params[0].clone()])),

            // Controlled rotations
            Self::CRX => Some((Self::CRX, smallvec![-1.0 * params[0].clone()])),
            Self::CRY => Some((Self::CRY, smallvec![-1.0 * params[0].clone()])),
            Self::CRZ => Some((Self::CRZ, smallvec![-1.0 * params[0].clone()])),

            // Two-parameter gates
            // RXY(theta, phi) -> Rotation axis is defined by phi.
            // Rotation amount is theta. Inverse is RXY(-theta, phi).
            Self::RXY => Some((
                Self::RXY,
                smallvec![-1.0 * params[0].clone(), params[1].clone()],
            )),

            // fSim(theta, phi) -> Inverse is fSim(-theta, -phi)
            Self::FSIM => Some((
                Self::FSIM,
                smallvec![-1.0 * params[0].clone(), -1.0 * params[1].clone()],
            )),

            // U(theta, phi, lambda)† = U(-theta, -lambda, -phi)
            // Note parameter swap for phi/lambda
            Self::U => Some((
                Self::U,
                smallvec![
                    -1.0 * params[0].clone(),
                    -1.0 * params[2].clone(),
                    -1.0 * params[1].clone()
                ],
            )),

            // XY gate (theta) -> Inverse is XY(pi+theta)
            Self::XY => Some((Self::XY, smallvec![PI + params[0].clone()])),
        }
    }

    /// Returns whether this gate is diagonal in the computational basis.
    ///
    /// Diagonal standard gates preserve computational basis states and only
    /// change their phases. This is a structural property of the gate family,
    /// independent of concrete parameter values.
    pub fn is_diagonal(&self) -> bool {
        matches!(
            self,
            Self::I
                | Self::RZ
                | Self::RZZ
                | Self::S
                | Self::SDG
                | Self::T
                | Self::TDG
                | Self::Z
                | Self::Phase
                | Self::GPhase
                | Self::CZ
                | Self::CRZ
        )
    }

    /// Returns whether exchanging the two operands leaves this two-qubit gate unchanged.
    ///
    /// This is an exact semantic property used by physical layout and device
    /// lowering. It is deliberately narrower than being self-inverse or having
    /// no control qubits: callers may reverse the ordered operands only for the
    /// gate families listed here.
    pub(crate) fn is_invariant_under_operand_swap(&self) -> bool {
        matches!(
            self,
            Self::CZ | Self::SWAP | Self::RXX | Self::RYY | Self::RZZ | Self::FSIM
        )
    }

    /// Returns the number of control qubits defined for this gate.
    ///
    /// For example:
    /// - `X` -> 0
    /// - `CX` -> 1
    /// - `CCX` -> 2
    pub fn num_ctrl_qubits(&self) -> usize {
        GATE_INFO_TABLE[*self as usize].0 as usize
    }

    /// Returns the total number of qubits this gate acts on (controls + targets).
    ///
    /// For example:
    /// - `X` -> 1
    /// - `CX` -> 2 (1 control + 1 target)
    /// - `SWAP` -> 2
    pub fn num_qubits(&self) -> usize {
        let idx = *self as usize;
        let (c, t, _) = GATE_INFO_TABLE[idx];
        (c + t) as usize
    }

    /// Returns the number of floating-point parameters this gate accepts.
    ///
    /// For example:
    /// - `H` -> 0
    /// - `RX` -> 1
    /// - `U` -> 3
    pub fn num_params(&self) -> usize {
        GATE_INFO_TABLE[*self as usize].2 as usize
    }
}

#[cfg(test)]
mod tests {
    use super::StandardGate;

    #[test]
    fn operand_swap_invariance_is_exhaustive_for_two_qubit_standard_gates() {
        let expected = [
            StandardGate::RXX,
            StandardGate::RYY,
            StandardGate::RZZ,
            StandardGate::SWAP,
            StandardGate::CZ,
            StandardGate::FSIM,
        ];

        for gate in StandardGate::all()
            .iter()
            .copied()
            .filter(|gate| gate.num_qubits() == 2)
        {
            assert_eq!(
                gate.is_invariant_under_operand_swap(),
                expected.contains(&gate),
                "unexpected operand-swap classification for {gate}"
            );
        }
    }

    #[test]
    fn classified_two_qubit_gates_are_matrix_invariant_under_operand_swap() {
        let cases: &[(StandardGate, &[f64])] = &[
            (StandardGate::RXX, &[0.37]),
            (StandardGate::RYY, &[-0.41]),
            (StandardGate::RZZ, &[0.23]),
            (StandardGate::SWAP, &[]),
            (StandardGate::CZ, &[]),
            (StandardGate::FSIM, &[0.31, -0.17]),
        ];
        let swapped_basis = [0, 2, 1, 3];

        for (gate, params) in cases {
            let matrix = gate.matrix(params).unwrap();
            for row in 0..4 {
                for column in 0..4 {
                    let swapped = matrix[[swapped_basis[row], swapped_basis[column]]];
                    assert!(
                        (matrix[[row, column]] - swapped).norm() < 1e-12,
                        "{gate} is not invariant at ({row}, {column})"
                    );
                }
            }
        }
    }

    #[test]
    fn xy_pulse_family_remains_single_qubit() {
        for gate in [StandardGate::XY, StandardGate::XY2P, StandardGate::XY2M] {
            assert_eq!(gate.num_qubits(), 1);
            assert_eq!(gate.matrix(&[0.37]).unwrap().dim(), (2, 2));
            assert!(!gate.is_invariant_under_operand_swap());
        }
    }
}
