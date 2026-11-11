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

//! Complex-matrix utilities for the Block-ZXZ decomposition.
//!
//! Mirrors the `numpy`/`scipy` matrix operations of the Python project.
//! `ndarray::Array2<Complex64>` is the primary matrix type (consistent with
//! the rest of the crate); `faer` is only borrowed for diagonalization and
//! singular-value decomposition.

use ndarray::{Array2, s};
use num_complex::Complex64;

/// Complex matrix alias.
pub type Cm = Array2<Complex64>;

/// Builds the `n x n` identity matrix.
pub fn eye(n: usize) -> Cm {
    Array2::eye(n)
}

/// Matrix product `A @ B`.
#[inline]
pub fn mat_mul(a: &Cm, b: &Cm) -> Cm {
    a.dot(b)
}

/// Conjugate transpose `A^H`.
#[inline]
pub fn adjoint(a: &Cm) -> Cm {
    a.t().mapv(|z| z.conj())
}

/// Builds a diagonal matrix from a list of diagonal values.
pub fn diagonal_from_values(values: &[Complex64]) -> Cm {
    let n = values.len();
    let mut out = Array2::<Complex64>::zeros((n, n));
    for (i, &v) in values.iter().enumerate() {
        out[[i, i]] = v;
    }
    out
}

/// Builds the block-diagonal matrix `diag(top_left, bottom_right)`.
pub fn block_diag(top_left: &Cm, bottom_right: &Cm) -> Cm {
    let td = top_left.dim().0;
    let bd = bottom_right.dim().0;
    let n = td + bd;
    let mut out = Array2::<Complex64>::zeros((n, n));
    out.slice_mut(s![..td, ..td]).assign(top_left);
    out.slice_mut(s![td.., td..]).assign(bottom_right);
    out
}

/// Maximum elementwise absolute difference (used for allclose checks).
///
/// A non-finite difference (caused by NaN or infinite inputs, or by a
/// numerically failed intermediate) is reported as positive infinity so
/// every allclose-style consumer rejects the matrix instead of silently
/// accepting it (`NaN > max` is always false, which would otherwise make a
/// NaN matrix look "equal").
pub fn max_abs_diff(a: &Cm, b: &Cm) -> f64 {
    let mut m = 0.0f64;
    for ((i, j), &av) in a.indexed_iter() {
        let d = (av - b[[i, j]]).norm();
        if !d.is_finite() {
            return f64::INFINITY;
        }
        if d > m {
            m = d;
        }
    }
    m
}

/// Returns whether `A` is unitary within the `atol` tolerance.
pub fn is_unitary(a: &Cm, atol: f64) -> bool {
    if a.nrows() != a.ncols() {
        return false;
    }
    let n = a.nrows();
    max_abs_diff(&mat_mul(&adjoint(a), a), &eye(n)) <= atol
}

/// Returns whether `A` is diagonal within the `atol` tolerance.
///
/// A non-finite off-diagonal element counts as "not diagonal".
pub fn is_diagonal(a: &Cm, atol: f64) -> bool {
    let (r, c) = a.dim();
    for i in 0..r {
        for j in 0..c {
            if i != j {
                let m = a[[i, j]].norm();
                if !m.is_finite() || m > atol {
                    return false;
                }
            }
        }
    }
    true
}

// ---- faer conversions ----

/// Copies an ndarray matrix into a faer matrix.
pub fn to_faer(a: &Cm) -> faer::Mat<Complex64> {
    let (r, c) = a.dim();
    faer::Mat::<Complex64>::from_fn(r, c, |i, j| a[[i, j]])
}

/// Copies a faer matrix reference into an ndarray matrix.
pub fn from_faer(m: &faer::MatRef<'_, Complex64>) -> Cm {
    Array2::from_shape_fn((m.nrows(), m.ncols()), |(i, j)| m[(i, j)])
}

// ---- Numeric decompositions (mirroring numpy.linalg.svd / scipy.linalg.schur) ----

/// Polar decomposition through the SVD: returns `(S, U_polar)` with
/// `matrix ~ S @ U_polar`.
///
/// Mirrors the Python `polar_from_svd`: `X = U Sigma V^H`,
/// `S = U Sigma U^H`, `U_polar = U V^H`. Numeric failures of the
/// underlying SVD (for example on non-finite inputs) are returned as
/// errors instead of panicking.
pub fn polar_from_svd(matrix: &Cm) -> Result<(Cm, Cm), String> {
    let fm = to_faer(matrix);
    let svd = fm.svd().map_err(|_| {
        format!(
            "SVD failed on a {}x{} matrix",
            matrix.nrows(),
            matrix.ncols()
        )
    })?;
    let u = from_faer(&svd.U());
    let v = from_faer(&svd.V());
    let s_vals: Vec<f64> = fm.singular_values().map_err(|_| {
        format!(
            "singular values failed on a {}x{} matrix",
            matrix.nrows(),
            matrix.ncols()
        )
    })?;
    let sigma = diagonal_from_real(&s_vals);
    let s_factor = mat_mul(&mat_mul(&u, &sigma), &adjoint(&u));
    let u_polar = mat_mul(&u, &adjoint(&v));
    Ok((s_factor, u_polar))
}

/// Eigen (Schur-style) diagonalization of a complex unitary matrix.
///
/// Returns `(Q, D)` with `matrix ~ Q D Q^H` and `D` the diagonal matrix of
/// eigenvalues. Mirrors the Python `schur_unitary_diagonalize`
/// (`scipy.linalg.schur` with `output="complex"`).
///
/// Degenerate-spectrum repair: faer's complex EVD does not guarantee that
/// eigenvectors of a repeated eigenvalue are mutually orthogonal (a normal
/// matrix is theoretically unitarily diagonalizable). Columns are grouped
/// by eigenvalue clusters and re-orthonormalized within each group: a
/// modified Gram-Schmidt pass first, and when the raw eigenvectors turn out
/// numerically linearly dependent (a maximally degenerate cluster), an
/// orthonormal basis of the cluster's approximate eigenspace is recovered
/// from the SVD null space of `matrix - lambda_bar I`. Both repairs keep
/// the diagonalization error bounded by the cluster/gap tolerance.
///
/// A `matrix` that is numerically a scalar multiple of the identity is
/// diagonalized directly by the identity basis (any orthonormal basis
/// diagonalizes it); this fast path avoids running an EVD on a maximally
/// degenerate spectrum and is exact to within `atol`.
///
/// When the EVD basis is not orthogonal enough for `atol` (LAPACK-style
/// EVD eigenvector error scales with `eps / spectral_gap`, which exceeds a
/// tight `atol` for nearly degenerate spectra), the EVD result is discarded
/// and a Jacobi (rotation) diagonalization takes over: its accumulated
/// basis is unitary by construction, independent of the spectral gap.
///
/// Numeric failures of the underlying EVD/SVD (for example on non-finite
/// inputs) are returned as errors instead of panicking.
pub fn schur_unitary_diagonalize(matrix: &Cm, atol: f64) -> Result<(Cm, Cm), String> {
    let n = matrix.nrows();
    if n > 0 && matrix.ncols() == n {
        // Fast path: numerically scalar multiple of the identity.
        let mu = (0..n).map(|i| matrix[[i, i]]).sum::<Complex64>() / n as f64;
        let mut scalar = eye(n);
        for i in 0..n {
            scalar[[i, i]] = mu;
        }
        if max_abs_diff(matrix, &scalar) <= atol {
            return Ok((eye(n), scalar));
        }
    }

    let fm = to_faer(matrix);
    let eig = fm.eigen().map_err(|_| {
        format!(
            "EVD failed on a {}x{} matrix",
            matrix.nrows(),
            matrix.ncols()
        )
    })?;
    let q_raw = from_faer(&eig.U());
    let evals: Vec<Complex64> = fm.eigenvalues().map_err(|_| {
        format!(
            "eigenvalues failed on a {}x{} matrix",
            matrix.nrows(),
            matrix.ncols()
        )
    })?;
    let q = reorthogonalize_degenerate_eigenvectors(matrix, &q_raw, &evals)?;

    // EVD eigenvector orthogonality error grows like eps / spectral_gap;
    // for nearly degenerate spectra it can exceed a tight atol. In that
    // case the basis is unusable (the demux factor check would reject it),
    // so restart from the Jacobi iteration, which stays unitary by
    // construction.
    let orthogonality_error = max_abs_diff(&mat_mul(&adjoint(&q), &q), &eye(n));
    if orthogonality_error <= atol * 0.5 {
        let d = diagonal_from_values(&evals);
        return Ok((q, d));
    }
    jacobi_diagonalize(matrix, atol)
}

/// Jacobi (rotation) unitary diagonalization of a normal matrix.
///
/// Sweeps through all `(p, q)` pairs and applies the analytic eigendecomposition
/// of each `2x2` sub-block as a unitary similarity transform. The accumulated
/// basis `Q` is a product of unitary transforms and therefore unitary by
/// construction, to working precision — unlike an EVD basis, whose columns
/// lose orthogonality as `eps / spectral_gap` grows. This is the numerically
/// stable path for nearly degenerate (near-scalar) spectra.
///
/// Convergence is checked on the off-diagonal Frobenius norm; a unitary
/// matrix is normal, so the iteration converges globally for every spectrum.
fn jacobi_diagonalize(matrix: &Cm, atol: f64) -> Result<(Cm, Cm), String> {
    let n = matrix.nrows();
    if n == 0 || matrix.ncols() != n {
        return Err("Jacobi diagonalization requires a non-empty square matrix".into());
    }
    let scale = matrix
        .iter()
        .map(|z| z.norm())
        .fold(0.0f64, f64::max)
        .max(1.0);
    // Numerical floor of the sweep target; the similarity transforms keep
    // the basis unitary, so a tight target is safe.
    let target = (atol * 0.05).clamp(1e-15, 1e-12) * scale;
    let max_sweeps = 80;

    let mut a = matrix.clone();
    let mut q = eye(n);
    for _sweep in 0..max_sweeps {
        // Paardekooper: the squared Frobenius norm of the strict lower
        // triangle is monotonically non-increasing under these 2x2 Schur
        // transforms for a normal matrix, and converges to zero.
        let mut lower_sq = 0.0f64;
        for p in 0..n {
            for r in (p + 1)..n {
                lower_sq += a[[r, p]].norm_sqr();
            }
        }
        if lower_sq.sqrt() <= target {
            break;
        }
        for p in 0..n {
            for r in (p + 1)..n {
                let b = a[[p, r]];
                let c = a[[r, p]];
                if c.norm() <= target * 0.01 {
                    continue;
                }
                let app = a[[p, p]];
                let aqq = a[[r, r]];
                // Analytic eigendecomposition of the 2x2 sub-block. Its
                // first column is an eigenvector, so the unitary
                // S = [v1, orthonormal complement] puts S^H G S into
                // upper-triangular form with an exact zero at (r, p).
                let half_trace = (app + aqq) * 0.5;
                let half_diff = (app - aqq) * 0.5;
                let disc = (half_diff * half_diff + b * c).sqrt();
                let lambda = half_trace + disc;
                // Try the first row equation (a - l) x + b y = 0 first,
                // fall back to the second row when its coefficients vanish
                // (for example b == 0 with c != 0, which would otherwise
                // collapse the vector and leave (r, p) un-zeroed).
                let mut v1 = [b, lambda - app];
                let mut norm = (v1[0].norm_sqr() + v1[1].norm_sqr()).sqrt();
                if norm <= 1e-14 {
                    v1 = [lambda - aqq, c];
                    norm = (v1[0].norm_sqr() + v1[1].norm_sqr()).sqrt();
                }
                if norm <= 1e-14 {
                    continue; // Sub-block is already (numerically) diagonal.
                }
                v1[0] /= norm;
                v1[1] /= norm;
                // Second column of the unitary S: the orthonormal
                // complement, which is the eigenvector of the other
                // eigenvalue for a normal 2x2 block.
                let s00 = v1[0];
                let s10 = v1[1];
                let s01 = -v1[1].conj();
                let s11 = v1[0].conj();
                // a <- S^H a (rows p, r)
                for j in 0..n {
                    let ap = a[[p, j]];
                    let ar = a[[r, j]];
                    a[[p, j]] = s00.conj() * ap + s10.conj() * ar;
                    a[[r, j]] = s01.conj() * ap + s11.conj() * ar;
                }
                // a <- a S (columns p, r)
                for i in 0..n {
                    let bp = a[[i, p]];
                    let br = a[[i, r]];
                    a[[i, p]] = bp * s00 + br * s10;
                    a[[i, r]] = bp * s01 + br * s11;
                }
                // q <- q S
                for i in 0..n {
                    let qp = q[[i, p]];
                    let qr = q[[i, r]];
                    q[[i, p]] = qp * s00 + qr * s10;
                    q[[i, r]] = qp * s01 + qr * s11;
                }
            }
        }
    }

    let evals: Vec<Complex64> = (0..n).map(|i| a[[i, i]]).collect();
    let d = diagonal_from_values(&evals);
    Ok((q, d))
}

/// Re-orthonormalizes eigenvectors inside degenerate subspaces (see the
/// documentation of [`schur_unitary_diagonalize`]).
///
/// Eigenvectors of the same eigenvalue cluster are first processed by a
/// modified Gram-Schmidt pass over the complex inner product. When the raw
/// eigenvectors of a cluster are numerically linearly dependent (faer
/// returns near-collinear vectors for a repeated eigenvalue), the partial
/// Gram-Schmidt basis is discarded and an orthonormal basis of the
/// cluster's approximate eigenspace is recovered from the smallest right
/// singular vectors of `matrix - lambda_bar I`, where `lambda_bar` is the
/// cluster mean. That null-space basis is exactly the eigenspace when the
/// degeneracy is exact, and its residual is bounded by the cluster spread
/// otherwise. Orthogonality across clusters follows from the normality of
/// a unitary matrix.
///
/// The function never produces un-orthonormalized columns, so `Q` stays
/// unitary (a demux factor check would otherwise reject it).
fn reorthogonalize_degenerate_eigenvectors(
    matrix: &Cm,
    q: &Cm,
    evals: &[Complex64],
) -> Result<Cm, String> {
    let n = q.ncols();
    debug_assert_eq!(q.nrows(), n);
    debug_assert_eq!(evals.len(), n);

    const CLUSTER_TOL: f64 = 1e-7;

    // Clustering: columns of the same eigenvalue (exact degeneracy on the
    // unit circle plus floating-point jitter) join the same group.
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for col in 0..n {
        let mut placed = false;
        for group in groups.iter_mut() {
            if (evals[group[0]] - evals[col]).norm() <= CLUSTER_TOL {
                group.push(col);
                placed = true;
                break;
            }
        }
        if !placed {
            groups.push(vec![col]);
        }
    }

    let mut out = q.clone();
    for group in &groups {
        if group.len() <= 1 {
            continue;
        }
        // First attempt: modified Gram-Schmidt within the group.
        let mut basis: Vec<Vec<Complex64>> = Vec::with_capacity(group.len());
        let mut collapsed = false;
        for &col in group {
            let mut v: Vec<Complex64> = (0..n).map(|r| q[[r, col]]).collect();
            for b in &basis {
                let proj: Complex64 = b.iter().zip(v.iter()).map(|(x, y)| x.conj() * y).sum();
                for r in 0..n {
                    v[r] -= proj * b[r];
                }
            }
            let norm: f64 = v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
            if norm > 1e-8 {
                for x in &mut v {
                    *x /= norm;
                }
                basis.push(v);
            } else {
                // Near-linear dependence: Gram-Schmidt cannot repair it.
                collapsed = true;
                break;
            }
        }
        if !collapsed {
            for (pos, &col) in group.iter().enumerate() {
                for r in 0..n {
                    out[[r, col]] = basis[pos][r];
                }
            }
            continue;
        }

        // Fallback: recover an orthonormal basis of the (approximate)
        // eigenspace from the SVD null space of `matrix - lambda_bar I`.
        let m = group.len();
        let lambda_bar = group.iter().map(|&c| evals[c]).sum::<Complex64>() / m as f64;
        let mut shifted = matrix.clone();
        for k in 0..n {
            shifted[[k, k]] -= lambda_bar;
        }
        let fm = to_faer(&shifted);
        let svd = fm
            .svd()
            .map_err(|_| format!("SVD failed on a {n}x{n} degenerate eigenspace"))?;
        let v = from_faer(&svd.V());
        // Rank the right singular vectors by the residual |A v_j| (that is,
        // by increasing singular value) without relying on an internal
        // ordering of the SVD output.
        let mut ranked: Vec<(f64, usize)> = (0..n)
            .map(|j| {
                let mut av = vec![Complex64::new(0.0, 0.0); n];
                for r in 0..n {
                    let mut acc = Complex64::new(0.0, 0.0);
                    for k in 0..n {
                        acc += shifted[[r, k]] * v[[k, j]];
                    }
                    av[r] = acc;
                }
                let residual = av.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
                (residual, j)
            })
            .collect();
        ranked.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));
        for (pos, &col) in group.iter().enumerate() {
            let src = ranked[pos].1;
            for r in 0..n {
                out[[r, col]] = v[[r, src]];
            }
        }
    }
    Ok(out)
}

/// Builds a complex diagonal matrix from real values (zero imaginary parts).
fn diagonal_from_real(values: &[f64]) -> Cm {
    let v: Vec<Complex64> = values.iter().map(|&x| Complex64::new(x, 0.0)).collect();
    diagonal_from_values(&v)
}

/// Builds a diagonal unitary square root from unit-modulus eigenvalues.
///
/// Mirrors the Python `build_unitary_diagonal_sqrt`:
/// `normalized = e / |e|`, `phase = exp(0.5j * arg(normalized))`.
pub fn build_unitary_diagonal_sqrt(eigenvalues: &Cm) -> Cm {
    let n = eigenvalues.nrows();
    let mut d = Array2::<Complex64>::zeros((n, n));
    for i in 0..n {
        let e = eigenvalues[[i, i]];
        let mag = e.norm();
        let normalized = if mag > 0.0 {
            e / mag
        } else {
            Complex64::new(1.0, 0.0)
        };
        // sqrt(e^{i theta}) = e^{i theta / 2}
        d[[i, i]] = normalized.sqrt();
    }
    d
}
