// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Quantum Gate Matrix Definitions
//!
//! This module provides the unitary matrix representations for all supported quantum gates.
//! Matrices are provided as lazily-initialized static constants for fixed gates, and
//! as functions for parametric gates.
//!
//! # Gate Categories
//!
//! - **Single-Qubit Gates**: H, X, Y, Z, S, T, and their variants
//! - **Rotation Gates**: RX, RY, RZ, RXX, RYY, RZZ, RZX, RXY
//! - **Two-Qubit Gates**: CX, CY, CZ, SWAP, iSWAP, fSim
//! - **Multi-Qubit Gates**: CCX (Toffoli)
//! - **Controlled Gates**: CRX, CRY, CRZ
//!
//! # Usage
//!
//! Static gates can be accessed directly:
//! ```
//! use cqlib_core::circuit::gate::gate_matrix::H_GATE;
//!
//! let h_matrix = &*H_GATE;
//! ```
//!
//! Parametric gates are constructed via functions:
//! ```
//! use cqlib_core::circuit::gate::gate_matrix::rx_gate;
//!
//! let rx_pi_2 = rx_gate(std::f64::consts::PI / 2.0);
//! ```

use ndarray::prelude::*;
use num_complex::Complex;
use std::f64::consts::FRAC_1_SQRT_2;
use std::sync::LazyLock;

/// The complex number $0 + 0i$.
const ZERO: Complex<f64> = Complex::new(0., 0.);

/// The complex number $1 + 0i$.
const ONE: Complex<f64> = Complex::new(1., 0.);

/// The imaginary unit $0 + 1i$.
const I: Complex<f64> = Complex::new(0., 1.);

/// $e^{i\pi/4} = \frac{1 + i}{\sqrt{2}}$.
const EXP_I_PI_4: Complex<f64> = Complex::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2);

/// $e^{-i\pi/4} = \frac{1 - i}{\sqrt{2}}$.
const EXP_NEG_I_PI_4: Complex<f64> = Complex::new(FRAC_1_SQRT_2, -FRAC_1_SQRT_2);

/// The Hadamard Gate.
///
/// Defined as:
/// ```text
/// 1/sqrt(2) * [
///   [1,  1],
///   [1, -1]
/// ]
/// ```
pub static H_GATE: LazyLock<Array2<Complex<f64>>> =
    LazyLock::new(|| array![[ONE, ONE], [ONE, -ONE]] * FRAC_1_SQRT_2);

/// The Identity Gate.
///
/// Defined as:
/// ```text
/// [
///   [1, 0],
///   [0, 1]
/// ]
/// ```
pub static I_GATE: LazyLock<Array2<Complex<f64>>> =
    LazyLock::new(|| array![[ONE, ZERO], [ZERO, ONE]]);

/// The iSWAP Gate.
///
/// Defined as:
/// ```text
/// [
///   [1, 0, 0, 0],
///   [0, 0, i, 0],
///   [0, i, 0, 0],
///   [0, 0, 0, 1]
/// ]
/// ```
pub static ISWAP_GATE: LazyLock<Array2<Complex<f64>>> = LazyLock::new(|| {
    array![
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ZERO, I, ZERO],
        [ZERO, I, ZERO, ZERO],
        [ZERO, ZERO, ZERO, ONE]
    ]
});

/// The S Gate (Phase Gate).
///
/// Defined as:
/// ```text
/// [
///   [1, 0],
///   [0, i]
/// ]
/// ```
pub static S_GATE: LazyLock<Array2<Complex<f64>>> =
    LazyLock::new(|| array![[ONE, ZERO], [ZERO, I]]);

/// The S-dagger Gate (S†).
///
/// Defined as:
/// ```text
/// [
///   [1,  0],
///   [0, -i]
/// ]
/// ```
pub static SDG_GATE: LazyLock<Array2<Complex<f64>>> =
    LazyLock::new(|| array![[ONE, ZERO], [ZERO, -I]]);

/// The SWAP Gate.
///
/// Defined as:
/// ```text
/// [
///   [1, 0, 0, 0],
///   [0, 0, 1, 0],
///   [0, 1, 0, 0],
///   [0, 0, 0, 1]
/// ]
/// ```
pub static SWAP_GATE: LazyLock<Array2<Complex<f64>>> = LazyLock::new(|| {
    array![
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ZERO, ONE, ZERO],
        [ZERO, ONE, ZERO, ZERO],
        [ZERO, ZERO, ZERO, ONE]
    ]
});

/// The T Gate (π/4 phase shift).
///
/// Defined as:
/// ```text
/// [
///   [1,           0],
///   [0, exp(i*pi/4)]
/// ]
/// ```
pub static T_GATE: LazyLock<Array2<Complex<f64>>> =
    LazyLock::new(|| array![[ONE, ZERO], [ZERO, EXP_I_PI_4]]);

/// The T-dagger Gate (T†).
///
/// Defined as:
/// ```text
/// [
///   [1,        0       ],
///   [0, exp(-i*pi/4)   ]
/// ]
/// ```
pub static TDG_GATE: LazyLock<Array2<Complex<f64>>> =
    LazyLock::new(|| array![[ONE, ZERO], [ZERO, EXP_NEG_I_PI_4]]);

/// The Pauli-X Gate (NOT Gate).
///
/// Defined as:
/// ```text
/// [
///   [0, 1],
///   [1, 0]
/// ]
/// ```
pub static X_GATE: LazyLock<Array2<Complex<f64>>> =
    LazyLock::new(|| array![[ZERO, ONE], [ONE, ZERO]]);

/// The Sqrt(X) Gate (X +90° rotation).
///
/// Defined as:
/// ```text
/// 1/sqrt(2) * [
///   [ 1, -i],
///   [-i,  1]
/// ]
/// ```
pub static X2P_GATE: LazyLock<Array2<Complex<f64>>> =
    LazyLock::new(|| array![[ONE, -I], [-I, ONE]] * FRAC_1_SQRT_2);

/// The Inverse Sqrt(X) Gate (X -90° rotation, or Sqrt(X)†).
///
/// Defined as:
/// ```text
/// 1/sqrt(2) * [
///   [1, i],
///   [i, 1]
/// ]
/// ```
pub static X2M_GATE: LazyLock<Array2<Complex<f64>>> =
    LazyLock::new(|| array![[ONE, I], [I, ONE]] * FRAC_1_SQRT_2);

/// The Pauli-Y Gate.
///
/// Defined as:
/// ```text
/// [
///   [0, -i],
///   [i,  0]
/// ]
/// ```
pub static Y_GATE: LazyLock<Array2<Complex<f64>>> = LazyLock::new(|| array![[ZERO, -I], [I, ZERO]]);

/// The Sqrt(Y) Gate (Y +90° rotation).
///
/// Defined as:
/// ```text
/// 1/sqrt(2) * [
///   [1, -1],
///   [1,  1]
/// ]
/// ```
pub static Y2P_GATE: LazyLock<Array2<Complex<f64>>> =
    LazyLock::new(|| array![[ONE, -ONE], [ONE, ONE]] * FRAC_1_SQRT_2);

/// The Inverse Sqrt(Y) Gate (Y -90° rotation, or Sqrt(Y)†).
///
/// Defined as:
/// ```text
/// 1/sqrt(2) * [
///   [ 1, 1],
///   [-1, 1]
/// ]
/// ```
pub static Y2M_GATE: LazyLock<Array2<Complex<f64>>> =
    LazyLock::new(|| array![[ONE, ONE], [-ONE, ONE]] * FRAC_1_SQRT_2);

/// The Pauli-Z Gate.
///
/// Defined as:
/// ```text
/// [
///   [1,  0],
///   [0, -1]
/// ]
/// ```
pub static Z_GATE: LazyLock<Array2<Complex<f64>>> =
    LazyLock::new(|| array![[ONE, ZERO], [ZERO, -ONE]]);

/// The Controlled-NOT Gate (CX).
///
/// Defined as:
/// ```text
/// [
///   [1, 0, 0, 0],
///   [0, 1, 0, 0],
///   [0, 0, 0, 1],
///   [0, 0, 1, 0]
/// ]
/// ```
pub static CX_GATE: LazyLock<Array2<Complex<f64>>> = LazyLock::new(|| {
    array![
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ONE, ZERO, ZERO],
        [ZERO, ZERO, ZERO, ONE],
        [ZERO, ZERO, ONE, ZERO]
    ]
});

/// The Toffoli Gate (Controlled-Controlled-NOT).
///
/// Defined as an 8x8 matrix where the last 2x2 block is swapped (X gate)
/// while the rest acts as identity:
/// ```text
/// [
///   [1, 0, 0, 0, 0, 0, 0, 0],
///   [0, 1, 0, 0, 0, 0, 0, 0],
///   [0, 0, 1, 0, 0, 0, 0, 0],
///   [0, 0, 0, 1, 0, 0, 0, 0],
///   [0, 0, 0, 0, 1, 0, 0, 0],
///   [0, 0, 0, 0, 0, 1, 0, 0],
///   [0, 0, 0, 0, 0, 0, 0, 1],
///   [0, 0, 0, 0, 0, 0, 1, 0]
/// ]
/// ```
pub static CCX_GATE: LazyLock<Array2<Complex<f64>>> = LazyLock::new(|| {
    let mut m = Array2::eye(8);
    // Swap last two diagonal elements to implement X on target if 11
    // Basis: 000, 001, 010, 011, 100, 101, 110, 111
    // 110 -> 111, 111 -> 110
    // Indices: 6 and 7
    m[[6, 6]] = ZERO;
    m[[6, 7]] = ONE;
    m[[7, 6]] = ONE;
    m[[7, 7]] = ZERO;
    m
});

/// The Controlled-Y Gate.
///
/// Defined as:
/// ```text
/// [
///   [1, 0, 0,  0],
///   [0, 1, 0,  0],
///   [0, 0, 0, -i],
///   [0, 0, i,  0]
/// ]
/// ```
pub static CY_GATE: LazyLock<Array2<Complex<f64>>> = LazyLock::new(|| {
    array![
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ONE, ZERO, ZERO],
        [ZERO, ZERO, ZERO, -I],
        [ZERO, ZERO, I, ZERO]
    ]
});

/// The Controlled-Z Gate.
///
/// Defined as:
/// ```text
/// [
///   [1, 0, 0,  0],
///   [0, 1, 0,  0],
///   [0, 0, 1,  0],
///   [0, 0, 0, -1]
/// ]
/// ```
pub static CZ_GATE: LazyLock<Array2<Complex<f64>>> = LazyLock::new(|| {
    array![
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ONE, ZERO, ZERO],
        [ZERO, ZERO, ONE, ZERO],
        [ZERO, ZERO, ZERO, -ONE]
    ]
});

/// Returns the single-qubit rotation gate around the X-axis (RX).
///
/// Defined as:
/// ```text
/// [
///   [cos(theta/2), -i*sin(theta/2)],
///   [-i*sin(theta/2), cos(theta/2)]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians.
#[inline]
pub fn rx_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = (theta / 2.0).sin_cos();
    let cos = Complex::new(cos_val, 0.0);
    let neg_i_sin = Complex::new(0.0, -sin_val);
    array![[cos, neg_i_sin], [neg_i_sin, cos]]
}

/// Returns the two-qubit rotation gate around the XX-axis (RXX).
///
/// Defined as:
/// ```text
/// [
///   [cos(theta/2),       0,              0,        -i*sin(theta/2)],
///   [      0,       cos(theta/2), -i*sin(theta/2),       0        ],
///   [      0,      -i*sin(theta/2), cos(theta/2),        0        ],
///   [-i*sin(theta/2),    0,              0,         cos(theta/2)]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians.
#[inline]
pub fn rxx_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = (theta / 2.0).sin_cos();
    let cos = Complex::new(cos_val, 0.0);
    let neg_i_sin = Complex::new(0.0, -sin_val);
    array![
        [cos, ZERO, ZERO, neg_i_sin],
        [ZERO, cos, neg_i_sin, ZERO],
        [ZERO, neg_i_sin, cos, ZERO],
        [neg_i_sin, ZERO, ZERO, cos]
    ]
}

/// Returns the single-qubit rotation gate around an arbitrary axis in the XY plane.
///
/// The axis of rotation is defined by the angle `phi`. When `phi = 0`, this is equivalent to `RX(theta)`.
///
/// Defined as:
/// ```text
/// [
///   [        cos(theta/2),         -i*exp(-i*phi)*sin(theta/2)],
///   [-i*exp(i*phi)*sin(theta/2),          cos(theta/2)        ]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians.
/// * `phi` - The azimuthal angle in radians defining the axis of rotation in the XY plane.
#[inline]
pub fn rxy_gate(theta: f64, phi: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = (theta / 2.0).sin_cos();
    let cos = Complex::new(cos_val, 0.0);
    let sin = Complex::new(sin_val, 0.0);

    let (phi_sin, phi_cos) = phi.sin_cos();
    let exp_neg_i_phi = Complex::new(phi_cos, -phi_sin);
    let exp_i_phi = Complex::new(phi_cos, phi_sin);

    // -i * exp(-i*phi) * sin(theta/2)
    // = -i * (cos(phi) - i*sin(phi)) * sin(theta/2)
    // = (-i*cos(phi) - sin(phi)) * sin(theta/2)
    let term1 = -I * exp_neg_i_phi * sin;

    // -i * exp(i*phi) * sin(theta/2)
    // = -i * (cos(phi) + i*sin(phi)) * sin(theta/2)
    // = (-i*cos(phi) + sin(phi)) * sin(theta/2)
    let term2 = -I * exp_i_phi * sin;

    array![[cos, term1], [term2, cos]]
}

/// Returns the single-qubit rotation gate around the Y-axis (RY).
///
/// Defined as:
/// ```text
/// [
///   [cos(theta/2), -sin(theta/2)],
///   [sin(theta/2),  cos(theta/2)]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians.
#[inline]
pub fn ry_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = (theta / 2.0).sin_cos();
    let cos = Complex::new(cos_val, 0.0);
    let sin = Complex::new(sin_val, 0.0);
    let neg_sin = -sin;
    array![[cos, neg_sin], [sin, cos]]
}

/// Returns the two-qubit rotation gate around the YY-axis (RYY).
///
/// Defined as:
/// ```text
/// [
///   [cos(theta/2),       0,              0,        i*sin(theta/2)],
///   [      0,       cos(theta/2), -i*sin(theta/2),       0       ],
///   [      0,      -i*sin(theta/2), cos(theta/2),        0       ],
///   [i*sin(theta/2),     0,              0,        cos(theta/2)]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians.
#[inline]
pub fn ryy_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = (theta / 2.0).sin_cos();
    let cos = Complex::new(cos_val, 0.0);
    let i_sin = Complex::new(0.0, sin_val);
    let neg_i_sin = -i_sin;
    array![
        [cos, ZERO, ZERO, i_sin],
        [ZERO, cos, neg_i_sin, ZERO],
        [ZERO, neg_i_sin, cos, ZERO],
        [i_sin, ZERO, ZERO, cos]
    ]
}

/// Returns the single-qubit rotation gate around the Z-axis (RZ).
///
/// Defined as:
/// ```text
/// [
///   [exp(-i*theta/2),       0      ],
///   [       0,       exp(i*theta/2)]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians.
#[inline]
pub fn rz_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = (theta / 2.0).sin_cos();
    let exp_neg = Complex::new(cos_val, -sin_val);
    let exp_pos = Complex::new(cos_val, sin_val);
    array![[exp_neg, ZERO], [ZERO, exp_pos]]
}

/// Returns the two-qubit rotation gate for the ZX interaction (RZX).
///
/// Defined as:
/// ```text
/// [
///   [   cos(theta/2), -i*sin(theta/2),        0,              0       ],
///   [-i*sin(theta/2),    cos(theta/2),        0,              0       ],
///   [       0,               0,          cos(theta/2),  i*sin(theta/2)],
///   [       0,               0,          i*sin(theta/2), cos(theta/2) ]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians.
#[inline]
pub fn rzx_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = (theta / 2.0).sin_cos();
    let cos = Complex::new(cos_val, 0.0);
    let i_sin = Complex::new(0.0, sin_val);
    let neg_i_sin = -i_sin;
    array![
        [cos, neg_i_sin, ZERO, ZERO],
        [neg_i_sin, cos, ZERO, ZERO],
        [ZERO, ZERO, cos, i_sin],
        [ZERO, ZERO, i_sin, cos]
    ]
}

/// Returns the two-qubit rotation gate around the ZZ-axis (RZZ).
///
/// Defined as:
/// ```text
/// [
///   [exp(-i*theta/2),       0,              0,              0       ],
///   [       0,       exp(i*theta/2),        0,              0       ],
///   [       0,              0,       exp(i*theta/2),        0       ],
///   [       0,              0,              0,       exp(-i*theta/2)]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians.
#[inline]
pub fn rzz_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = (theta / 2.0).sin_cos();
    let exp_neg = Complex::new(cos_val, -sin_val);
    let exp_pos = Complex::new(cos_val, sin_val);
    array![
        [exp_neg, ZERO, ZERO, ZERO],
        [ZERO, exp_pos, ZERO, ZERO],
        [ZERO, ZERO, exp_pos, ZERO],
        [ZERO, ZERO, ZERO, exp_neg]
    ]
}

/// Returns the Phase gate (P gate).
///
/// Defined as:
/// ```text
/// [
///   [1,       0      ],
///   [0, exp(i*lambda)]
/// ]
/// ```
///
/// # Arguments
///
/// * `lambda` - The phase angle in radians.
#[inline]
pub fn phase_gate(lambda: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = lambda.sin_cos();
    let exp_i_lambda = Complex::new(cos_val, sin_val);
    array![[ONE, ZERO], [ZERO, exp_i_lambda]]
}

/// Returns a global phase gate.
///
/// This applies a global phase factor to the quantum state, physically equivalent to identity
/// but useful for mathematical consistency in simulations.
///
/// Defined as:
/// ```text
/// [
///   [exp(i*theta),      0      ],
///   [     0,       exp(i*theta)]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The global phase angle in radians.
#[inline]
pub fn global_phase_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = theta.sin_cos();
    let exp_i_theta = Complex::new(cos_val, sin_val);
    array![[exp_i_theta, ZERO], [ZERO, exp_i_theta]]
}

/// Returns the general single-qubit unitary gate (U3).
///
/// Defined as:
/// ```text
/// [
///   [      cos(theta/2),          -exp(i*lambda)*sin(theta/2)     ],
///   [exp(i*phi)*sin(theta/2), exp(i*(phi+lambda))*cos(theta/2)]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The rotation angle defining the amplitude mixture.
/// * `phi` - The phase of the lower element.
/// * `lambda` - The phase of the upper-right element.
#[inline]
pub fn u_gate(theta: f64, phi: f64, lambda: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = (theta / 2.0).sin_cos();
    let cos = Complex::new(cos_val, 0.0);
    let sin = Complex::new(sin_val, 0.0);

    let (phi_sin, phi_cos) = phi.sin_cos();
    let exp_i_phi = Complex::new(phi_cos, phi_sin);

    let (lam_sin, lam_cos) = lambda.sin_cos();
    let exp_i_lambda = Complex::new(lam_cos, lam_sin);

    let (pl_sin, pl_cos) = (phi + lambda).sin_cos();
    let exp_i_phi_lambda = Complex::new(pl_cos, pl_sin);

    array![
        [cos, -exp_i_lambda * sin],
        [exp_i_phi * sin, exp_i_phi_lambda * cos]
    ]
}

/// Returns the parameterized XY gate.
///
/// This gate represents a rotation defined by the phase `theta` in the complex plane off-diagonals.
///
/// Defined as:
/// ```text
/// [
///   [       0,        -i*exp(-i*theta)],
///   [-i*exp(i*theta),        0        ]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The phase angle in radians.
#[inline]
pub fn xy_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = theta.sin_cos();
    let exp_i_theta = Complex::new(cos_val, sin_val);
    let exp_neg_i_theta = Complex::new(cos_val, -sin_val);

    let term1 = -I * exp_neg_i_theta;
    let term2 = -I * exp_i_theta;

    array![[ZERO, term1], [term2, ZERO]]
}

/// Returns the XY2P gate (Plus Half-Pi XY).
///
/// This is a +90° single-qubit rotation about the XY-plane axis selected by `theta`.
///
/// Defined as:
/// ```text
/// 1/sqrt(2) * [
///   [       1,        -i*exp(-i*theta)],
///   [-i*exp(i*theta),        1        ]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The phase angle in radians.
#[inline]
pub fn xy2p_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = theta.sin_cos();
    let exp_i_theta = Complex::new(cos_val, sin_val);
    let exp_neg_i_theta = Complex::new(cos_val, -sin_val);

    let term1 = -I * exp_neg_i_theta * FRAC_1_SQRT_2;
    let term2 = -I * exp_i_theta * FRAC_1_SQRT_2;
    let diag = Complex::new(FRAC_1_SQRT_2, 0.0);

    array![[diag, term1], [term2, diag]]
}

/// Returns the XY2M gate (Minus Half-Pi XY).
///
/// This is a -90° single-qubit rotation about the XY-plane axis selected by `theta`.
///
/// Defined as:
/// ```text
/// 1/sqrt(2) * [
///   [      1,         i*exp(-i*theta)],
///   [i*exp(i*theta),         1       ]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The phase angle in radians.
#[inline]
pub fn xy2m_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = theta.sin_cos();
    let exp_i_theta = Complex::new(cos_val, sin_val);
    let exp_neg_i_theta = Complex::new(cos_val, -sin_val);

    let term1 = I * exp_neg_i_theta * FRAC_1_SQRT_2;
    let term2 = I * exp_i_theta * FRAC_1_SQRT_2;
    let diag = Complex::new(FRAC_1_SQRT_2, 0.0);

    array![[diag, term1], [term2, diag]]
}

/// Returns the Controlled-RX gate (CRX).
///
/// Applies an X-rotation to the target qubit if the control qubit is |1⟩.
///
/// Defined as:
/// ```text
/// [
///   [1, 0,               0,              0],
///   [0, 1,               0,              0],
///   [0, 0,   cos(theta/2), -i*sin(theta/2)],
///   [0, 0, -i*sin(theta/2),   cos(theta/2)]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians.
#[inline]
pub fn crx_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = (theta / 2.0).sin_cos();
    let cos = Complex::new(cos_val, 0.0);
    let neg_i_sin = Complex::new(0.0, -sin_val);

    array![
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ONE, ZERO, ZERO],
        [ZERO, ZERO, cos, neg_i_sin],
        [ZERO, ZERO, neg_i_sin, cos]
    ]
}

/// Returns the Controlled-RY gate (CRY).
///
/// Applies a Y-rotation to the target qubit if the control qubit is |1⟩.
///
/// Defined as:
/// ```text
/// [
///   [1, 0,            0,            0],
///   [0, 1,            0,            0],
///   [0, 0, cos(theta/2), -sin(theta/2)],
///   [0, 0, sin(theta/2),  cos(theta/2)]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians.
#[inline]
pub fn cry_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = (theta / 2.0).sin_cos();
    let cos = Complex::new(cos_val, 0.0);
    let sin = Complex::new(sin_val, 0.0);
    let neg_sin = -sin;

    array![
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ONE, ZERO, ZERO],
        [ZERO, ZERO, cos, neg_sin],
        [ZERO, ZERO, sin, cos]
    ]
}

/// Returns the Controlled-RZ gate (CRZ).
///
/// Applies a Z-rotation to the target qubit if the control qubit is |1⟩.
///
/// Defined as:
/// ```text
/// [
///   [1, 0,               0,              0],
///   [0, 1,               0,              0],
///   [0, 0, exp(-i*theta/2),       0       ],
///   [0, 0,               0, exp(i*theta/2)]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians.
#[inline]
pub fn crz_gate(theta: f64) -> Array2<Complex<f64>> {
    let (sin_val, cos_val) = (theta / 2.0).sin_cos();
    let exp_neg = Complex::new(cos_val, -sin_val);
    let exp_pos = Complex::new(cos_val, sin_val);

    array![
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ONE, ZERO, ZERO],
        [ZERO, ZERO, exp_neg, ZERO],
        [ZERO, ZERO, ZERO, exp_pos]
    ]
}

/// Returns the Fermionic Simulation gate (fSim).
///
/// A useful 2-qubit gate for superconducting quantum processors (e.g., Sycamore).
/// It combines an iSWAP-like interaction (`theta`) and a conditional phase (`phi`).
///
/// Defined as:
/// ```text
/// [
///   [1,      0,           0,              0       ],
///   [0,  cos(theta), -i*sin(theta),       0       ],
///   [0, -i*sin(theta),  cos(theta),       0       ],
///   [0,      0,           0,        exp(-i*phi)   ]
/// ]
/// ```
///
/// # Arguments
///
/// * `theta` - The swapping angle in radians.
/// * `phi` - The conditional phase angle in radians.
#[inline]
pub fn fsim_gate(theta: f64, phi: f64) -> Array2<Complex<f64>> {
    let (sin_theta, cos_theta) = theta.sin_cos();
    let cos_t = Complex::new(cos_theta, 0.0);
    let neg_i_sin_t = Complex::new(0.0, -sin_theta);

    let (sin_phi, cos_phi) = phi.sin_cos();
    let exp_neg_i_phi = Complex::new(cos_phi, -sin_phi);

    array![
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, cos_t, neg_i_sin_t, ZERO],
        [ZERO, neg_i_sin_t, cos_t, ZERO],
        [ZERO, ZERO, ZERO, exp_neg_i_phi]
    ]
}

/// Constructs a controlled version of a given unitary matrix.
///
/// The resulting matrix represents a gate with `num_ctrls` control qubits and
/// the target qubits acted upon by `base_matrix`.
/// The controls are assumed to be "active high" (triggered on state |1>).
///
/// The structure of the matrix is block diagonal:
/// ```text
/// [
///   [ I, 0 ],
///   [ 0, U ]
/// ]
/// ```
/// where `I` is the identity matrix of size `dim - base_dim` and `U` is the `base_matrix`.
/// Total dimension `dim = base_dim * 2^num_ctrls`.
///
/// # Arguments
///
/// * `base_matrix` - The unitary matrix of the target gate.
/// * `num_ctrls` - The number of control qubits to add.
pub fn control_matrix(
    base_matrix: &Array2<Complex<f64>>,
    num_ctrls: usize,
) -> Array2<Complex<f64>> {
    if num_ctrls == 0 {
        return base_matrix.clone();
    }

    let base_dim = base_matrix.nrows();
    let total_dim = base_dim << num_ctrls;

    // Create an identity matrix for the full dimension
    let mut mat = Array2::eye(total_dim);

    // Replace the bottom-right block with the base matrix
    // The top-left (total_dim - base_dim) x (total_dim - base_dim) block remains Identity
    // This corresponds to the condition where at least one control bit is 0.
    // The action U is applied only when all controls are 1 (the last block).

    let start_idx = total_dim - base_dim;

    // Using slice assignment or direct iteration
    for i in 0..base_dim {
        for j in 0..base_dim {
            mat[[start_idx + i, start_idx + j]] = base_matrix[[i, j]];
        }
    }

    mat
}

#[cfg(test)]
#[path = "./gate_matrix_test.rs"]
mod gate_matrix_test;
