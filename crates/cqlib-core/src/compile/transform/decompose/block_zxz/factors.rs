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

//! Equations (5)–(11) of Krol & Al-Ars. All block indices split on the MSB.
//! Demultiplexing takes a unitary Schur basis of U0 U1†, so the same W
//! reconstructs both branches, including their relative and common phases.

use super::synthesis::{Matrix, failed};
use crate::compile::CompilerError;
use faer::{Mat, Side};
use ndarray::{Array1, Array2, ArrayView2, Axis, s};
use num_complex::Complex64 as C;

pub(super) fn to_faer(m: ArrayView2<'_, C>) -> Mat<C> {
    Mat::from_fn(m.nrows(), m.ncols(), |i, j| m[[i, j]])
}

fn from_faer(m: faer::MatRef<'_, C>) -> Matrix {
    Array2::from_shape_fn((m.nrows(), m.ncols()), |(i, j)| m[(i, j)])
}

pub(super) fn adjoint(m: &Matrix) -> Matrix {
    m.t().mapv(|z| z.conj())
}

pub(super) fn residual(a: &Matrix, b: &Matrix) -> f64 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).norm())
        .fold(0.0, |max, x| {
            if x.is_finite() {
                x.max(max)
            } else {
                f64::INFINITY
            }
        })
}

pub(super) fn validate_input(matrix: &Matrix, atol: f64) -> Result<f64, CompilerError> {
    let n = matrix.nrows();
    if !atol.is_finite() || atol < 0.0 {
        return Err(CompilerError::InvalidInput(
            "Block-ZXZ requires finite nonnegative atol".into(),
        ));
    }
    if matrix.ncols() != n || n < 2 || !n.is_power_of_two() {
        return Err(CompilerError::InvalidInput(format!(
            "expected square power-of-two dimension ≥ 2, got {}×{}",
            n,
            matrix.ncols()
        )));
    }
    if !matrix.iter().all(|z| z.is_finite()) {
        return Err(CompilerError::InvalidInput(
            "Block-ZXZ matrix contains non-finite entries".into(),
        ));
    }
    let error = residual(&adjoint(matrix).dot(matrix), &Matrix::eye(n));
    if error > atol {
        return Err(CompilerError::InvalidInput(format!(
            "unitarity residual {error:e} exceeds {atol:e}"
        )));
    }
    Ok(error)
}

pub(super) fn is_block_diagonal(m: &Matrix, atol: f64) -> bool {
    let k = m.nrows() / 2;
    (0..k).all(|i| (0..k).all(|j| m[[i, k + j]].norm() <= atol && m[[k + i, j]].norm() <= atol))
}

/// Returns the closest unitary matrix in every unitarily invariant norm via polar SVD.
pub(super) fn nearest_unitary(x: &Matrix) -> Result<Matrix, CompilerError> {
    let svd = to_faer(x.view())
        .svd()
        .map_err(|e| failed(format!("unitary projection SVD (dim {}): {e:?}", x.nrows())))?;
    Ok(from_faer((svd.U() * svd.V().adjoint()).as_ref()))
}

/// Full SVD supplies an orthonormal completion even when X has a null space.
fn polar(x: ArrayView2<'_, C>) -> Result<(Matrix, Matrix), CompilerError> {
    let svd = to_faer(x)
        .svd()
        .map_err(|e| failed(format!("polar SVD (dim {}): {e:?}", x.nrows())))?;
    let u = svd.U();
    let positive = u * svd.S() * u.adjoint();
    Ok((
        from_faer(positive.as_ref()),
        from_faer((u * svd.V().adjoint()).as_ref()),
    ))
}

pub(super) struct Blocks {
    pub a1: Matrix,
    pub a2: Matrix,
    pub b: Matrix,
    pub c: Matrix,
}

pub(super) fn split(u: &Matrix) -> Result<Blocks, CompilerError> {
    let k = u.nrows() / 2;
    let x = u.slice(s![..k, ..k]);
    let (sx, ux) = polar(x)?;
    let (sy, uy) = polar(u.slice(s![..k, k..]))?;
    let c = adjoint(&ux).dot(&uy) * -C::i();
    let a1 = (sx + sy * C::i()).dot(&ux);
    let a2 = &u.slice(s![k.., ..k]) + &u.slice(s![k.., k..]).dot(&adjoint(&c));
    let b = adjoint(&a1).dot(&x) * C::new(2.0, 0.0) - Matrix::eye(k);
    Ok(Blocks { a1, a2, b, c })
}

pub(super) struct Demux {
    pub v: Matrix,
    pub d: Array1<C>,
    pub w: Matrix,
}

/// Target single-linkage threshold for Hermitian eigenvalue clustering. Keeping
/// distinct clusters separated by at least 1e-2 bounds cross-cluster
/// eigenvector mixing to O(eps / 1e-2) ≈ 2e-14 even for opposite-sine pairs.
/// For dense wide spectra whose gaps are all below 1e-2, the threshold adapts
/// to half of the largest spectral gap so clusters always split cleanly.
const SCHUR_CLUSTER_TOL: f64 = 1e-2;

fn hermitian_part(m: &Matrix) -> Mat<C> {
    let n = m.nrows();
    Mat::from_fn(n, n, |i, j| (m[[i, j]] + m[[j, i]].conj()) * 0.5)
}

fn skew_hermitian_imag_part(m: &Matrix) -> Mat<C> {
    let n = m.nrows();
    Mat::from_fn(n, n, |i, j| {
        (m[[i, j]] - m[[j, i]].conj()) * C::new(0.0, -0.5)
    })
}

fn max_eigen_gap(s: faer::diag::DiagRef<'_, C>) -> f64 {
    let p = s.dim();
    let mut best = 0.0_f64;
    for i in 1..p {
        let gap = (s[i] - s[i - 1]).norm();
        if gap > best {
            best = gap;
        }
    }
    best
}

fn is_already_diagonal(m: &Matrix) -> bool {
    let tol = 4.0 * f64::EPSILON;
    m.indexed_iter()
        .all(|((i, j), z)| i == j || z.norm() <= tol)
}

/// Recursively computes an orthonormal Schur basis for a unitary (normal) block.
///
/// If all eigenvalues of the block lie strictly inside an open arc of width
/// `2*pi/3` around their mean (verified via the exact eigenvalues of the aligned
/// Hermitian real part, not merely diagonal entries), then `sin(phi)` is
/// strictly monotonic with slope `> 0.5` and a single Hermitian eigendecomposition
/// of the aligned imaginary part diagonalizes the block to `O(p * eps)`.
///
/// Otherwise the spectrum has spread `>= 0.5` along either the real or imaginary
/// axis, so picking the better-separated Hermitian axis and splitting at gaps
/// above `min(SCHUR_CLUSTER_TOL, 0.5 * max_gap)` strictly reduces every cluster
/// dimension until every subcluster is 1D or fits inside a narrow arc.
fn diagonalize_block(sub: &Matrix, rotation: C) -> Result<Matrix, CompilerError> {
    let p = sub.nrows();
    if p <= 1 || is_already_diagonal(sub) {
        return Ok(Matrix::eye(p));
    }

    let use_arc_alignment = rotation == C::new(1.0, 0.0);
    let working = if use_arc_alignment {
        let mu = (0..p).map(|i| sub[[i, i]]).sum::<C>() / p as f64;
        let mu_norm = mu.norm();
        if mu_norm > 0.5 {
            sub * (mu.conj() / mu_norm)
        } else {
            sub.clone()
        }
    } else {
        sub * rotation
    };

    let eig_re = hermitian_part(&working)
        .self_adjoint_eigen(Side::Lower)
        .map_err(|e| failed(format!("Schur real basis (dim {p}): {e:?}")))?;

    if use_arc_alignment && (0..p).all(|i| eig_re.S()[i].re > 0.5) {
        let eig_tan = skew_hermitian_imag_part(&working)
            .self_adjoint_eigen(Side::Lower)
            .map_err(|e| failed(format!("Schur arc basis (dim {p}): {e:?}")))?;
        return Ok(from_faer(eig_tan.U()));
    }

    let max_gap_re = max_eigen_gap(eig_re.S());
    let (chosen, max_gap) = if max_gap_re >= SCHUR_CLUSTER_TOL {
        (eig_re, max_gap_re)
    } else {
        let eig_im = skew_hermitian_imag_part(&working)
            .self_adjoint_eigen(Side::Lower)
            .map_err(|e| failed(format!("Schur imag basis (dim {p}): {e:?}")))?;
        let max_gap_im = max_eigen_gap(eig_im.S());
        if max_gap_im > max_gap_re {
            (eig_im, max_gap_im)
        } else {
            (eig_re, max_gap_re)
        }
    };

    let mut v = from_faer(chosen.U());
    if max_gap <= f64::EPSILON {
        return Ok(v);
    }

    let tol = SCHUR_CLUSTER_TOL.min(0.5 * max_gap);
    let s_vals = chosen.S();
    let mut s0 = 0;
    while s0 < p {
        let mut s1 = s0 + 1;
        while s1 < p && (s_vals[s1] - s_vals[s1 - 1]).norm() <= tol {
            s1 += 1;
        }
        if s1 - s0 > 1 && s1 - s0 < p {
            let basis = v.slice(s![.., s0..s1]);
            let child_sub = basis.t().mapv(|z| z.conj()).dot(sub).dot(&basis);
            let child_u = diagonalize_block(&child_sub, C::new(1.0, 0.0))?;
            let rotated = basis.dot(&child_u);
            v.slice_mut(s![.., s0..s1]).assign(&rotated);
        }
        s0 = s1;
    }
    Ok(v)
}

/// A unitary is normal. Its Hermitian real and imaginary parts commute, so
/// their joint orthonormal eigenbasis gives a diagonal complex Schur form.
fn schur_basis(m: &Matrix, rotation: C) -> Result<Matrix, CompilerError> {
    diagonalize_block(m, rotation)
}

pub(super) fn demux(u0: &Matrix, u1: &Matrix, atol: f64) -> Result<Demux, CompilerError> {
    let m = u0.dot(&adjoint(u1));
    // When the relative unitary is diagonal, the computational basis is
    // already a Schur basis. Diagonalizing a nearly scalar matrix instead
    // introduces arbitrary dense eigenvectors into otherwise simple leaves.
    let roundoff = atol.min(8.0 * f64::EPSILON);
    if m.indexed_iter()
        .all(|((i, j), z)| i == j || z.norm() <= roundoff)
    {
        let d = m.diag().mapv(|z| C::from_polar(1., 0.5 * z.arg()));
        let rows = d.view().insert_axis(Axis(1));
        let w = u1 * &rows;
        if residual(&(&w * &rows), u0) <= roundoff
            && residual(&(&w * &rows.mapv(|z| z.conj())), u1) <= roundoff
        {
            return Ok(Demux {
                v: Matrix::eye(m.nrows()),
                d,
                w,
            });
        }
    }
    let mut error = f64::INFINITY;
    // Backup rotations separate spectra that remain poorly conditioned at 0.
    for angle in [0.0, std::f64::consts::FRAC_PI_4, 1.0] {
        let v = schur_basis(&m, C::from_polar(1.0, angle))?;
        let vh = adjoint(&v);
        // Only the diagonal of V† M V enters the square root. The row/column
        // inner products avoid a second dense matrix multiplication.
        let left = vh.dot(&m);
        let d = Array1::from_shape_fn(m.nrows(), |i| {
            let eigenvalue = left.row(i).dot(&v.column(i));
            C::from_polar(1.0, 0.5 * eigenvalue.arg())
        });
        let w = (&vh * &d.view().insert_axis(Axis(1))).dot(u1);
        // D is stored as a vector. Broadcasting scales columns of V.
        error = residual(&(&v * &d).dot(&w), u0)
            .max(residual(&(&v * &d.mapv(|z| z.conj())).dot(&w), u1));
        if error <= atol {
            return Ok(Demux { v, d, w });
        }
    }
    Err(failed(format!(
        "demultiplexing (dim {}): maximum residual {error:e} exceeds {atol:e}",
        u0.nrows()
    )))
}

#[cfg(test)]
mod tests {
    use super::super::synthesis::synthesis_test::{assert_matrix_close, seeded_unitary};
    use super::*;

    #[test]
    fn polar_factors_complete_the_null_space_at_each_rank() {
        let left = seeded_unitary(4, 81);
        let right = seeded_unitary(4, 82);
        for rank in 0..=4 {
            let values = Array1::from_shape_fn(4, |i| {
                C::from(if i < rank { 1. / (i + 1) as f64 } else { 0. })
            });
            let x = (&left * &values).dot(&right);
            let (positive, unitary) = polar(x.view()).unwrap();
            assert_matrix_close(&positive, &(&left * &values).dot(&adjoint(&left)), 1e-13);
            assert_matrix_close(&unitary.dot(&adjoint(&unitary)), &Matrix::eye(4), 1e-13);
            assert_matrix_close(&positive.dot(&unitary), &x, 1e-13);
        }
    }

    #[test]
    fn both_multiplexer_branches_reconstruct_with_degenerate_or_conjugate_spectra() {
        let v = seeded_unitary(4, 21);
        let u1 = seeded_unitary(4, 22);
        for phases in [[0.9; 4], [0.7, -0.7, 1.3, -1.3], [0., 3e-9, 9e-9, 1.7]] {
            let eigenvalues = Array1::from_vec(phases.to_vec()).mapv(|p| C::from_polar(1., p));
            let u0 = (&v * &eigenvalues).dot(&adjoint(&v)).dot(&u1);
            let result = demux(&u0, &u1, 1e-12).unwrap();
            assert_matrix_close(&(&result.v * &result.d).dot(&result.w), &u0, 1e-12);
            assert_matrix_close(
                &(&result.v * &result.d.mapv(|z| z.conj())).dot(&result.w),
                &u1,
                1e-12,
            );
            assert_matrix_close(&adjoint(&result.v).dot(&result.v), &Matrix::eye(4), 1e-13);
        }
    }

    #[test]
    fn failed_demultiplexing_reports_the_dimension_and_residual() {
        let id = Matrix::eye(4);
        let err = demux(&(&id * 2.), &id, 1e-12).err().unwrap().to_string();
        assert!(
            err.contains("demultiplexing (dim 4)") && err.contains("residual"),
            "{err}"
        );
    }

    #[test]
    fn finite_elements_that_overflow_the_gram_matrix_are_rejected() {
        let matrix = Matrix::eye(4) * C::new(f64::MAX, 0.);
        let err = validate_input(&matrix, f64::MAX).unwrap_err();
        assert!(matches!(err, CompilerError::InvalidInput(_)));
    }

    #[test]
    fn demux_resolves_dual_projection_near_conjugate_spectra() {
        let v = seeded_unitary(4, 91);
        let u1 = seeded_unitary(4, 92);
        let half_pi = std::f64::consts::FRAC_PI_2;
        let quarter_pi = std::f64::consts::FRAC_PI_4;
        for gap in [
            1e-12, 1e-9, 1.5e-8, 2e-8, 5e-8, 1e-7, 1e-6, 1e-4, 5e-3, 1.5e-2,
        ] {
            let phases = [half_pi, -half_pi + gap, quarter_pi, -3.0 * quarter_pi + gap];
            let eigenvalues = Array1::from_vec(phases.to_vec()).mapv(|p| C::from_polar(1., p));
            let u0 = (&v * &eigenvalues).dot(&adjoint(&v)).dot(&u1);
            let result = demux(&u0, &u1, 1e-12).unwrap();
            assert_matrix_close(&(&result.v * &result.d).dot(&result.w), &u0, 1e-12);
            assert_matrix_close(
                &(&result.v * &result.d.mapv(|z| z.conj())).dot(&result.w),
                &u1,
                1e-12,
            );
        }
    }

    #[test]
    fn demux_resolves_single_linkage_chains_and_narrow_arcs() {
        let v = seeded_unitary(8, 93);
        let u1 = seeded_unitary(8, 94);
        let half_pi = std::f64::consts::FRAC_PI_2;
        let quarter_pi = std::f64::consts::FRAC_PI_4;
        let pi = std::f64::consts::PI;
        let spectra: Vec<[f64; 8]> = vec![
            // 3-way chains near both 0 and pi/4 projections
            [
                half_pi - 1e-8,
                -half_pi,
                half_pi + 1e-8,
                0.1,
                quarter_pi - 1e-8,
                -3.0 * quarter_pi,
                quarter_pi + 1e-8,
                -0.3,
            ],
            // Narrow arc near pi/2 (where sin is stationary)
            [
                half_pi - 3e-4,
                half_pi - 2e-4,
                half_pi - 1e-4,
                half_pi,
                half_pi + 1e-4,
                half_pi + 2e-4,
                -half_pi,
                -half_pi + 1e-5,
            ],
            // Opposite-pole clusters around 0 and pi
            [0.0, 1e-8, -1e-8, 2e-4, pi, pi - 1e-8, -pi + 1e-8, pi - 2e-4],
        ];
        for phases in spectra {
            let eigenvalues = Array1::from_vec(phases.to_vec()).mapv(|p| C::from_polar(1., p));
            let u0 = (&v * &eigenvalues).dot(&adjoint(&v)).dot(&u1);
            let result = demux(&u0, &u1, 1e-12).unwrap();
            assert_matrix_close(&(&result.v * &result.d).dot(&result.w), &u0, 1e-12);
            assert_matrix_close(
                &(&result.v * &result.d.mapv(|z| z.conj())).dot(&result.w),
                &u1,
                1e-12,
            );
            assert_matrix_close(&adjoint(&result.v).dot(&result.v), &Matrix::eye(8), 1e-13);
        }
    }

    #[test]
    fn nearest_unitary_projects_noisy_matrices_within_tolerance() {
        let exact = seeded_unitary(8, 95);
        let noisy = &exact * C::from(1.0 + 2e-7);
        let projected = nearest_unitary(&noisy).unwrap();
        assert_matrix_close(&adjoint(&projected).dot(&projected), &Matrix::eye(8), 1e-14);
        assert_matrix_close(&projected, &exact, 1e-14);
        assert_matrix_close(&projected, &noisy, 1e-6);
    }

    #[test]
    fn demux_resolves_dense_wide_arc_and_full_circle_spectra() {
        let dim = 256;
        let v = seeded_unitary(dim, 96);
        let vh = adjoint(&v);
        let eye = Matrix::eye(dim);
        let pi = std::f64::consts::PI;

        let wide_arc: Vec<f64> = (0..dim)
            .map(|i| -1.2 + 2.4 * (i as f64) / ((dim - 1) as f64))
            .collect();
        let full_circle: Vec<f64> = (0..dim)
            .map(|i| -pi + 2.0 * pi * (i as f64) / (dim as f64) + 0.37)
            .collect();

        for phases in [wide_arc, full_circle] {
            let eigenvalues = Array1::from_vec(phases).mapv(|p| C::from_polar(1.0, p));
            let u0 = (&v * &eigenvalues).dot(&vh);
            let result = demux(&u0, &eye, 1e-10).unwrap();
            assert_matrix_close(&(&result.v * &result.d).dot(&result.w), &u0, 1e-10);
            assert_matrix_close(
                &(&result.v * &result.d.mapv(|z| z.conj())).dot(&result.w),
                &eye,
                1e-10,
            );
            assert_matrix_close(&adjoint(&result.v).dot(&result.v), &eye, 1e-12);
        }
    }

    #[test]
    fn demux_resolves_convex_diagonal_arc_trap() {
        let dim = 16;
        let v = seeded_unitary(dim, 97);
        let u1 = seeded_unitary(dim, 98);
        let mut phases = vec![0.0; dim];
        for (i, phase) in phases.iter_mut().enumerate().take(dim - 1) {
            *phase = (i as f64) * 1e-7;
        }
        phases[dim - 1] = std::f64::consts::PI;
        let eigenvalues = Array1::from_vec(phases).mapv(|p| C::from_polar(1.0, p));
        let u0 = (&v * &eigenvalues).dot(&adjoint(&v)).dot(&u1);
        let result = demux(&u0, &u1, 1e-12).unwrap();
        assert_matrix_close(&(&result.v * &result.d).dot(&result.w), &u0, 1e-12);
        assert_matrix_close(
            &(&result.v * &result.d.mapv(|z| z.conj())).dot(&result.w),
            &u1,
            1e-12,
        );
        assert_matrix_close(&adjoint(&result.v).dot(&result.v), &Matrix::eye(dim), 1e-13);
    }
}
