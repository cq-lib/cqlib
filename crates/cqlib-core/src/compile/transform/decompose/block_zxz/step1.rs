// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Block-ZXZ step 1: block splitting + polar decomposition + solving the
//! factors `A1, A2, B, C`.
//!
//! Function-by-function mirror of the Python `decompose/block_ZXZ/step1.py`.

use super::matrix::{Cm, adjoint, eye, is_unitary, mat_mul};
use ndarray::s;
use num_complex::Complex64;

/// Returns whether `value` is a positive power of two.
pub fn is_power_of_two(value: usize) -> bool {
    value > 0 && (value & (value - 1)) == 0
}

/// Returns whether `matrix` is unitary within `atol`.
pub fn is_matrix_unitary(matrix: &Cm, atol: f64) -> bool {
    is_unitary(matrix, atol)
}

/// Returns whether `matrix` is diagonal within `atol`.
///
/// A non-finite off-diagonal element counts as "not diagonal".
pub fn is_matrix_diagonal(matrix: &Cm, atol: f64) -> bool {
    let (r, c) = matrix.dim();
    for i in 0..r {
        for j in 0..c {
            if i != j {
                let m = matrix[[i, j]].norm();
                if !m.is_finite() || m > atol {
                    return false;
                }
            }
        }
    }
    true
}

/// Returns whether every element of `matrix` is finite (neither NaN nor
/// infinite).
pub fn has_finite_elements(matrix: &Cm) -> bool {
    matrix.iter().all(|z| z.re.is_finite() && z.im.is_finite())
}

/// Validates that the input is a `2^n x 2^n` unitary matrix.
///
/// Mirrors the Python `assert_square_unitary`; returns `Err(String)` on
/// failure. Rejects `atol` that is not a finite non-negative number, and
/// rejects matrices containing NaN or infinite elements before any
/// numerical decomposition is attempted (a NaN matrix would otherwise
/// slip past the `U^H U ~ I` comparison because NaN comparisons are always
/// false, and then poison the SVD/EVD pipeline).
pub fn assert_square_unitary(matrix: &Cm, atol: f64) -> Result<(), String> {
    if !atol.is_finite() || atol < 0.0 {
        return Err(format!(
            "Tolerance atol must be a finite non-negative number, got {atol}."
        ));
    }
    if matrix.ndim() != 2 {
        return Err("Input matrix must be two-dimensional.".into());
    }
    let (rows, cols) = matrix.dim();
    if rows != cols {
        return Err("Input matrix must be square.".into());
    }
    if !is_power_of_two(rows) {
        return Err("Input matrix dimension must be a power of two.".into());
    }
    if !has_finite_elements(matrix) {
        return Err("Input matrix contains NaN or infinite elements.".into());
    }
    if !is_unitary(matrix, atol) {
        return Err("Input matrix must be unitary.".into());
    }
    Ok(())
}

/// Splits `unitary` into four equal blocks along the most significant qubit.
///
/// Mirrors the Python `split_unitary_blocks`; returns `(X, Y, U21, U22)`.
/// The matrix is re-validated with the caller's `atol` (never a hidden
/// fixed tolerance, which would silently disagree with the public one) and
/// validation failures are returned as errors instead of panicking.
pub fn split_unitary_blocks(unitary: &Cm, atol: f64) -> Result<(Cm, Cm, Cm, Cm), String> {
    assert_square_unitary(unitary, atol)?;
    let dim = unitary.nrows();
    let half = dim / 2;
    Ok((
        unitary.slice(s![..half, ..half]).to_owned(),
        unitary.slice(s![..half, half..]).to_owned(),
        unitary.slice(s![half.., ..half]).to_owned(),
        unitary.slice(s![half.., half..]).to_owned(),
    ))
}

/// Polar decomposition through the SVD; returns `(S, U_polar)`.
///
/// Numeric failures are returned as errors instead of panicking.
pub fn polar_from_svd(matrix: &Cm) -> Result<(Cm, Cm), String> {
    super::matrix::polar_from_svd(matrix)
}

/// Computes the Block-ZXZ factor `C`: `C = -i * Ux^H @ Uy`.
pub fn compute_c_from_polar(u_x: &Cm, u_y: &Cm) -> Cm {
    let mut c = mat_mul(&adjoint(u_x), u_y);
    c *= Complex64::new(0.0, -1.0);
    c
}

/// Computes the Block-ZXZ factor `A1 = (Sx + i Sy) @ Ux`.
pub fn compute_a1(s_x: &Cm, s_y: &Cm, u_x: &Cm) -> Cm {
    let mut inner = s_x.clone();
    let iy = s_y.mapv(|z| Complex64::new(0.0, 1.0) * z);
    inner += &iy;
    mat_mul(&inner, u_x)
}

/// Computes the Block-ZXZ factor `A2 = U21 + U22 @ (i Uy^H Ux)`.
pub fn compute_a2(u21: &Cm, u22: &Cm, u_y: &Cm, u_x: &Cm) -> Cm {
    let t = mat_mul(&adjoint(u_y), u_x).mapv(|z| Complex64::new(0.0, 1.0) * z);
    let t = mat_mul(u22, &t);
    u21 + &t
}

/// Computes the Block-ZXZ factor `B = 2 A1^H @ X - I`.
pub fn compute_b(a1: &Cm, x: &Cm) -> Cm {
    let identity = eye(x.nrows());
    let t = mat_mul(&adjoint(a1), x).mapv(|z| z * 2.0);
    t - &identity
}

/// Validates that the step-1 outputs `A1/A2/B/C` are all unitary.
pub fn assert_step1_outputs_unitary(
    a1: &Cm,
    a2: &Cm,
    b: &Cm,
    c: &Cm,
    atol: f64,
) -> Result<(), String> {
    for (name, m) in [("A1", a1), ("A2", a2), ("B", b), ("C", c)] {
        if !is_unitary(m, atol) {
            return Err(format!("{name} is not unitary within tolerance {atol}."));
        }
    }
    Ok(())
}

/// Runs Block-ZXZ step 1 and returns `(A1, A2, B, C)`.
pub fn block_zxz_step1(unitary: &Cm, atol: f64) -> Result<(Cm, Cm, Cm, Cm), String> {
    assert_square_unitary(unitary, atol)?;
    let n = unitary.nrows();
    let num_qubits = (n as f64).log2().round() as usize;
    if num_qubits < 2 {
        return Err("Block-ZXZ Step 1 expects at least a 2-qubit unitary.".into());
    }

    let (x, y, u21, u22) = split_unitary_blocks(unitary, atol)?;
    let (s_x, u_x) = polar_from_svd(&x)?;
    let (s_y, u_y) = polar_from_svd(&y)?;

    let c = compute_c_from_polar(&u_x, &u_y);
    let a1 = compute_a1(&s_x, &s_y, &u_x);
    let a2 = compute_a2(&u21, &u22, &u_y, &u_x);
    let b = compute_b(&a1, &x);

    assert_step1_outputs_unitary(&a1, &a2, &b, &c, atol)?;
    Ok((a1, a2, b, c))
}
