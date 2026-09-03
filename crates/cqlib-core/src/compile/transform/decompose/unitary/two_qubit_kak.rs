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

//! Numeric two-qubit KAK / Weyl decomposition primitive.
//!
//! This module is deliberately circuit-agnostic. It decomposes a concrete
//! 4x4 unitary matrix into local single-qubit factors and a Cartan interaction:
//!
//! ```text
//! U = exp(i*g)
//!   * (K1l ⊗ K1r)
//!   * exp(i*(a*XX + b*YY + c*ZZ))
//!   * (K2l ⊗ K2r)
//! ```
//!
//! The returned coordinates satisfy the Weyl chamber convention
//! `pi/4 >= a >= b >= |c|`, with the additional boundary convention that
//! `c >= 0` when `a = pi/4`. The compiler-facing lowering lives in
//! `unitary_2q.rs`; this file owns only the numerical decomposition contract.
//!
//! The implementation validates finite 4x4 input, the intermediate symmetric
//! unitary used by the Autonne factorization, the local SU(2) factors, the Weyl
//! chamber invariants, and final matrix reconstruction. Detected numerical
//! failures are returned as compiler errors rather than accepted as approximate
//! success. This primitive emits no circuit operations and makes no
//! target-basis or hardware-topology decisions.

use super::matrix::{c, det_2x2};
use crate::circuit::StandardGate;
use crate::compile::CompilerError;
use faer::Mat;
use faer::Side::Lower;
use lazy_static::lazy_static;
use ndarray::{Array1, Array2, array};
use num_complex::Complex64;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

const SYNTHESIS_NAME: &str = "decompose.two_qubit_kak";
const UNITARY_EPS: f64 = 1e-8;
const RECONSTRUCTION_EPS: f64 = 1e-7;
const AUTONNE_EPS: f64 = 10.0 * UNITARY_EPS;
const KRON_RANK_EPS: f64 = 1e-8;

static B_RAW: [[Complex64; 4]; 4] = [
    [c(1.0, 0.0), c(0.0, 1.0), c(0.0, 0.0), c(0.0, 0.0)],
    [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 1.0), c(1.0, 0.0)],
    [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 1.0), c(-1.0, 0.0)],
    [c(1.0, 0.0), c(0.0, -1.0), c(0.0, 0.0), c(0.0, 0.0)],
];

static B_DAG_RAW: [[Complex64; 4]; 4] = [
    [c(0.5, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.5, 0.0)],
    [c(0.0, -0.5), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.5)],
    [c(0.0, 0.0), c(0.0, -0.5), c(0.0, -0.5), c(0.0, 0.0)],
    [c(0.0, 0.0), c(0.5, 0.0), c(-0.5, 0.0), c(0.0, 0.0)],
];

const IPX: [[Complex64; 2]; 2] = [[c(0.0, 0.0), c(0.0, 1.0)], [c(0.0, 1.0), c(0.0, 0.0)]];
const IPY: [[Complex64; 2]; 2] = [[c(0.0, 0.0), c(1.0, 0.0)], [c(-1.0, 0.0), c(0.0, 0.0)]];
const IPZ: [[Complex64; 2]; 2] = [[c(0.0, 1.0), c(0.0, 0.0)], [c(0.0, 0.0), c(0.0, -1.0)]];

lazy_static! {
    static ref B: Array2<Complex64> = Array2::from_shape_fn((4, 4), |(row, col)| B_RAW[row][col]);
    static ref B_DAG: Array2<Complex64> =
        Array2::from_shape_fn((4, 4), |(row, col)| B_DAG_RAW[row][col]);
    static ref IPX_MATRIX: Array2<Complex64> =
        Array2::from_shape_fn((2, 2), |(row, col)| IPX[row][col]);
    static ref IPY_MATRIX: Array2<Complex64> =
        Array2::from_shape_fn((2, 2), |(row, col)| IPY[row][col]);
    static ref IPZ_MATRIX: Array2<Complex64> =
        Array2::from_shape_fn((2, 2), |(row, col)| IPZ[row][col]);
}

fn kak_failed(reason: impl Into<String>) -> CompilerError {
    CompilerError::TransformFailed {
        name: SYNTHESIS_NAME,
        reason: reason.into(),
    }
}

fn kak_invalid(reason: impl Into<String>) -> CompilerError {
    CompilerError::InvalidInput(format!("{SYNTHESIS_NAME}: {}", reason.into()))
}

/// Result of a two-qubit KAK decomposition.
///
/// The represented matrix is:
///
/// ```text
/// exp(i*global_phase)
///   * (k1l ⊗ k1r)
///   * RXX(-2a) * RYY(-2b) * RZZ(-2c)
///   * (k2l ⊗ k2r)
/// ```
#[derive(Debug, Clone)]
pub struct KakDecomposition {
    /// Scalar phase multiplying the complete decomposition.
    pub global_phase: f64,
    /// Left local factor applied after the Cartan interaction.
    pub k1l: Array2<Complex64>,
    /// Right local factor applied after the Cartan interaction.
    pub k1r: Array2<Complex64>,
    /// Left local factor applied before the Cartan interaction.
    pub k2l: Array2<Complex64>,
    /// Right local factor applied before the Cartan interaction.
    pub k2r: Array2<Complex64>,
    /// Canonical Pauli-XX interaction coordinate.
    pub a: f64,
    /// Canonical Pauli-YY interaction coordinate.
    pub b: f64,
    /// Canonical Pauli-ZZ interaction coordinate.
    pub c: f64,
}

fn max_entry_diff(a: &Array2<Complex64>, b: &Array2<Complex64>) -> f64 {
    let mut max_diff = 0.0_f64;
    for i in 0..a.nrows() {
        for j in 0..a.ncols() {
            max_diff = max_diff.max((a[[i, j]] - b[[i, j]]).norm());
        }
    }
    max_diff
}

/// Performs the KAK decomposition of a 4x4 unitary matrix.
///
/// # Errors
///
/// Returns [`CompilerError`] when the input is not a finite 4x4 unitary or a
/// numerically valid canonical decomposition cannot be reconstructed.
pub fn kak_decompose(matrix: &Array2<Complex64>) -> Result<KakDecomposition, CompilerError> {
    validate_input(matrix)?;

    let det_u_raw = Mat::from_fn(matrix.nrows(), matrix.ncols(), |row, col| {
        let value = matrix[[row, col]];
        faer::c64::new(value.re, value.im)
    })
    .as_ref()
    .determinant();
    let det_u = Complex64::new(det_u_raw.re, det_u_raw.im);
    let det_pow = det_u.powf(-0.25);
    let u_su4 = matrix.mapv(|value| det_pow * value);
    let mut global_phase = det_u.arg() / 4.0;

    let u_p = B_DAG.dot(&u_su4.dot(&*B));

    let m2_raw = u_p.t().to_owned().dot(&u_p);
    let max_diff = max_entry_diff(&m2_raw, &m2_raw.t().to_owned());

    if max_diff > AUTONNE_EPS {
        return Err(kak_failed(format!(
            "U_p^T U_p is not complex symmetric: max transpose difference {max_diff}"
        )));
    }

    let m2 = Array2::from_shape_fn(m2_raw.dim(), |(row, col)| {
        (m2_raw[[row, col]] + m2_raw[[col, row]]) * 0.5
    });
    validate_unitary_4x4("U_p^T U_p", &m2, AUTONNE_EPS)?;
    let (p, d) = autonne_decompose(&m2)?;

    let mut d_angles = [0.0f64; 4];
    for i in 0..4 {
        d_angles[i] = -d[i].arg() / 2.0;
    }
    d_angles[3] = -(d_angles[0] + d_angles[1] + d_angles[2]);

    let mut cs = [0.0f64; 3];
    for i in 0..3 {
        cs[i] = ((d_angles[i] + d_angles[3]) / 2.0).rem_euclid(TAU);
    }

    let cstemp: [f64; 3] = cs.map(|x| {
        let y = x.rem_euclid(FRAC_PI_2);
        y.min(FRAC_PI_2 - y)
    });
    let mut sort_order = [0usize, 1, 2];
    sort_order.sort_by(|&a, &b| cstemp[a].partial_cmp(&cstemp[b]).unwrap());
    let order = [sort_order[1], sort_order[2], sort_order[0]];

    let cs_orig = cs;
    let d_orig = d_angles;
    let p_orig = p.clone();
    for i in 0..3 {
        cs[i] = cs_orig[order[i]];
        d_angles[i] = d_orig[order[i]];
    }
    let mut p = p;
    for i in 0..3 {
        for row in 0..4 {
            p[[row, i]] = p_orig[[row, order[i]]];
        }
    }

    let det_p = Mat::from_fn(p.nrows(), p.ncols(), |row, col| p[[row, col]].re)
        .as_ref()
        .determinant();
    if (det_p.abs() - 1.0).abs() > AUTONNE_EPS {
        return Err(kak_failed(format!(
            "Autonne P determinant {det_p}, expected +/-1"
        )));
    }
    if det_p < 0.0 {
        for row in 0..4 {
            p[[row, 3]] = -p[[row, 3]];
        }
    }

    let diag_d_inv: Array2<Complex64> = Array2::from_diag(&Array1::from_vec(
        d_angles.map(|x| Complex64::from_polar(1.0, x)).to_vec(),
    ));
    let u_p_p_d_inv = u_p.dot(&p.dot(&diag_d_inv));
    let p_t = p.t().to_owned();
    let k1_magic = B.dot(&u_p_p_d_inv.dot(&*B_DAG));
    let k2_magic = B.dot(&p_t.dot(&*B_DAG));

    let (mut k1l, mut k1r, phase_l) = decompose_two_qubit_product_gate(&k1_magic)?;
    let (k2l, mut k2r, phase_r) = decompose_two_qubit_product_gate(&k2_magic)?;
    global_phase += phase_l + phase_r;

    // The raw Cartan coordinates are periodic modulo pi/2 under local Pauli
    // conjugations. If the first coordinate lies beyond the pi/2 boundary,
    // translate it back by 3pi/2 and absorb the equivalent Y conjugation into
    // the left local factors. The scalar introduced by the local matrices is
    // tracked in the global phase.
    if cs[0] > FRAC_PI_2 {
        cs[0] -= 3.0 * FRAC_PI_2;
        k1l = k1l.dot(&*IPY_MATRIX);
        k1r = k1r.dot(&*IPY_MATRIX);
        global_phase += FRAC_PI_2;
    }
    // Same periodic-boundary correction for the second coordinate, using the
    // X local conjugation associated with that Weyl-axis permutation.
    if cs[1] > FRAC_PI_2 {
        cs[1] -= 3.0 * FRAC_PI_2;
        k1l = k1l.dot(&*IPX_MATRIX);
        k1r = k1r.dot(&*IPX_MATRIX);
        global_phase += FRAC_PI_2;
    }

    // Reflections across the pi/4 chamber walls move the first two
    // coordinates into the canonical Weyl chamber. `conjs` records whether an
    // odd number of these reflections occurred; that parity determines the
    // later correction needed for the third coordinate.
    let mut conjs = 0u32;
    if cs[0] > FRAC_PI_4 {
        cs[0] = FRAC_PI_2 - cs[0];
        k1l = k1l.dot(&*IPY_MATRIX);
        k2r = IPY_MATRIX.dot(&k2r);
        conjs += 1;
        global_phase -= FRAC_PI_2;
    }
    if cs[1] > FRAC_PI_4 {
        cs[1] = FRAC_PI_2 - cs[1];
        k1l = k1l.dot(&*IPX_MATRIX);
        k2r = IPX_MATRIX.dot(&k2r);
        conjs += 1;
        global_phase += FRAC_PI_2;
        if conjs == 1 {
            global_phase -= PI;
        }
    }

    // The third coordinate first gets the same pi/2 periodic-boundary
    // correction. When exactly one earlier reflection occurred, the local
    // conjugation parity contributes an additional scalar sign, represented as
    // a pi shift in the accumulated global phase.
    if cs[2] > FRAC_PI_2 {
        cs[2] -= 3.0 * FRAC_PI_2;
        k1l = k1l.dot(&*IPZ_MATRIX);
        k1r = k1r.dot(&*IPZ_MATRIX);
        global_phase += FRAC_PI_2;
        if conjs == 1 {
            global_phase -= PI;
        }
    }
    // An odd number of earlier chamber reflections flips the orientation of
    // the third axis. Reflect it through the pi/4 wall and absorb the matching
    // Z conjugations into the local factors.
    if conjs == 1 {
        cs[2] = FRAC_PI_2 - cs[2];
        k1l = k1l.dot(&*IPZ_MATRIX);
        k2r = IPZ_MATRIX.dot(&k2r);
        global_phase += FRAC_PI_2;
    }
    // Finally fold the third coordinate from the outer half of the chamber
    // into the signed interval used by the emitted XX/YY/ZZ interaction block.
    if cs[2] > FRAC_PI_4 {
        cs[2] -= FRAC_PI_2;
        k1l = k1l.dot(&*IPZ_MATRIX);
        k1r = k1r.dot(&*IPZ_MATRIX);
        global_phase -= FRAC_PI_2;
    }

    let decomp = KakDecomposition {
        global_phase,
        k1l,
        k1r,
        k2l,
        k2r,
        a: cs[1],
        b: cs[0],
        c: cs[2],
    };

    validate_weyl_chamber(decomp.a, decomp.b, decomp.c)?;
    validate_local_su2(&decomp)?;
    validate_reconstruction(matrix, &decomp)?;
    Ok(decomp)
}

fn validate_weyl_chamber(a: f64, b: f64, c: f64) -> Result<(), CompilerError> {
    let eps = UNITARY_EPS;
    if a < -eps || b < -eps {
        return Err(kak_failed(format!(
            "Weyl chamber violation: a={a}, b={b} must be non-negative"
        )));
    }
    if a > FRAC_PI_4 + eps {
        return Err(kak_failed(format!("Weyl chamber violation: a={a} > pi/4")));
    }
    if a + eps < b {
        return Err(kak_failed(format!("Weyl chamber violation: a={a} < b={b}")));
    }
    if b + eps < c.abs() {
        return Err(kak_failed(format!(
            "Weyl chamber violation: b={b} < |c|={}",
            c.abs()
        )));
    }
    if (a - FRAC_PI_4).abs() < eps && c < -eps {
        return Err(kak_failed(format!(
            "Weyl chamber violation: a=pi/4 but c={c} < 0"
        )));
    }
    Ok(())
}

fn validate_local_su2(decomp: &KakDecomposition) -> Result<(), CompilerError> {
    let eps = UNITARY_EPS;
    for (name, m) in [
        ("K1l", &decomp.k1l),
        ("K1r", &decomp.k1r),
        ("K2l", &decomp.k2l),
        ("K2r", &decomp.k2r),
    ] {
        if m.nrows() != 2 || m.ncols() != 2 {
            return Err(kak_failed(format!("{name} is not 2x2")));
        }
        let product = m.t().mapv(|value| value.conj()).dot(m);
        for i in 0..2 {
            for j in 0..2 {
                let expected = if i == j {
                    Complex64::new(1.0, 0.0)
                } else {
                    Complex64::new(0.0, 0.0)
                };
                let diff = (product[[i, j]] - expected).norm();
                if diff > eps {
                    return Err(kak_failed(format!(
                        "{name} is not unitary: (Mdag M)[{i},{j}] differs by {diff}"
                    )));
                }
            }
        }
        let det = det_2x2(m);
        if (det - Complex64::new(1.0, 0.0)).norm() > eps {
            return Err(kak_failed(format!("{name} det={det}, expected 1")));
        }
    }
    Ok(())
}

fn validate_input(matrix: &Array2<Complex64>) -> Result<(), CompilerError> {
    if matrix.nrows() != 4 || matrix.ncols() != 4 {
        return Err(kak_invalid(format!(
            "expected 4x4 matrix, got {}x{}",
            matrix.nrows(),
            matrix.ncols()
        )));
    }

    for ((row, col), value) in matrix.indexed_iter() {
        if !value.re.is_finite() || !value.im.is_finite() {
            return Err(kak_invalid(format!(
                "matrix contains non-finite element at ({row}, {col}): {value}"
            )));
        }
    }
    validate_unitary_4x4("input matrix", matrix, UNITARY_EPS)?;
    Ok(())
}

fn validate_unitary_4x4(
    matrix_name: &'static str,
    matrix: &Array2<Complex64>,
    eps: f64,
) -> Result<(), CompilerError> {
    let product = matrix.t().mapv(|value| value.conj()).dot(matrix);
    for row in 0..4 {
        for col in 0..4 {
            let expected = if row == col {
                Complex64::new(1.0, 0.0)
            } else {
                Complex64::new(0.0, 0.0)
            };
            let diff = (product[[row, col]] - expected).norm();
            if diff > eps {
                return Err(kak_failed(format!(
                    "{matrix_name} is not unitary: (Mdag M)[{row},{col}] differs by {diff}"
                )));
            }
        }
    }

    Ok(())
}

fn validate_reconstruction(
    original: &Array2<Complex64>,
    decomp: &KakDecomposition,
) -> Result<(), CompilerError> {
    let k1 = ndarray::linalg::kron(&decomp.k1l, &decomp.k1r);
    let k2 = ndarray::linalg::kron(&decomp.k2l, &decomp.k2r);
    let rxx = StandardGate::RXX
        .matrix(&[-2.0 * decomp.a])
        .map_err(|e| kak_failed(format!("failed to get RXX matrix: {e}")))?;
    let ryy = StandardGate::RYY
        .matrix(&[-2.0 * decomp.b])
        .map_err(|e| kak_failed(format!("failed to get RYY matrix: {e}")))?;
    let rzz = StandardGate::RZZ
        .matrix(&[-2.0 * decomp.c])
        .map_err(|e| kak_failed(format!("failed to get RZZ matrix: {e}")))?;

    let reconstructed = k1
        .dot(&rxx.dot(&ryy.dot(&rzz.dot(&k2))))
        .mapv(|value| Complex64::from_polar(1.0, decomp.global_phase) * value);
    let error = reconstruction_error(original, &reconstructed);

    if error.max_entry > RECONSTRUCTION_EPS {
        return Err(kak_failed(format!(
            "KAK reconstruction failed: max_entry={}, frobenius={}, phase_invariant_frobenius={}, global_phase={}, cartan=({}, {}, {})",
            error.max_entry,
            error.frobenius,
            error.phase_invariant_frobenius,
            decomp.global_phase,
            decomp.a,
            decomp.b,
            decomp.c
        )));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct ReconstructionError {
    max_entry: f64,
    frobenius: f64,
    phase_invariant_frobenius: f64,
}

fn reconstruction_error(
    expected: &Array2<Complex64>,
    actual: &Array2<Complex64>,
) -> ReconstructionError {
    let mut max_entry = 0.0_f64;
    let mut frobenius_sq = 0.0_f64;
    let mut expected_norm_sq = 0.0_f64;
    let mut actual_norm_sq = 0.0_f64;
    let mut inner = Complex64::new(0.0, 0.0);

    for row in 0..expected.nrows() {
        for col in 0..expected.ncols() {
            let expected_value = expected[[row, col]];
            let actual_value = actual[[row, col]];
            let diff = actual_value - expected_value;
            max_entry = max_entry.max(diff.norm());
            frobenius_sq += diff.norm_sqr();
            expected_norm_sq += expected_value.norm_sqr();
            actual_norm_sq += actual_value.norm_sqr();
            inner += expected_value.conj() * actual_value;
        }
    }

    ReconstructionError {
        max_entry,
        frobenius: frobenius_sq.sqrt(),
        phase_invariant_frobenius: (expected_norm_sq + actual_norm_sq - 2.0 * inner.norm())
            .max(0.0)
            .sqrt(),
    }
}

fn autonne_decompose(
    m2: &Array2<Complex64>,
) -> Result<(Array2<Complex64>, [Complex64; 4]), CompilerError> {
    validate_autonne_input(m2)?;

    let real_part = Mat::from_fn(4, 4, |i, j| m2[[i, j]].re);
    let real_decomp = real_part
        .self_adjoint_eigen(Lower)
        .map_err(|e| kak_failed(format!("failed to diagonalize Re(M2): {e:?}")))?;

    let real_basis = real_decomp.U();
    let real_eigenvalues = real_decomp.S();
    let mut p_real = [[0.0_f64; 4]; 4];
    let mut start = 0usize;

    while start < 4 {
        let mut end = start + 1;
        while end < 4 && (real_eigenvalues[end] - real_eigenvalues[start]).abs() <= AUTONNE_EPS {
            end += 1;
        }

        let width = end - start;
        if width == 1 {
            // A non-degenerate real eigenvalue already fixes a real Autonne
            // basis vector, so it can be copied directly from the real-part
            // eigensystem.
            for row in 0..4 {
                p_real[row][start] = real_basis[(row, start)];
            }
        } else {
            // Inside a degenerate eigenspace of Re(M2), any orthonormal basis
            // diagonalizes the real part. We must diagonalize Im(M2) restricted
            // to that subspace as well; otherwise P^T M2 P need not be
            // diagonal even though the real part is. The resulting real basis
            // simultaneously diagonalizes the commuting real and imaginary
            // parts of the complex symmetric unitary.
            let imag_block = Mat::from_fn(width, width, |row, col| {
                let mut value = 0.0_f64;
                for i in 0..4 {
                    for j in 0..4 {
                        value += real_basis[(i, start + row)]
                            * m2[[i, j]].im
                            * real_basis[(j, start + col)];
                    }
                }
                value
            });
            let imag_decomp = imag_block.self_adjoint_eigen(Lower).map_err(|e| {
                kak_failed(format!(
                    "failed to diagonalize Im(M2) degeneracy block: {e:?}"
                ))
            })?;
            let block_basis = imag_decomp.U();

            for local_col in 0..width {
                for row in 0..4 {
                    let mut value = 0.0_f64;
                    for basis_col in 0..width {
                        value += real_basis[(row, start + basis_col)]
                            * block_basis[(basis_col, local_col)];
                    }
                    p_real[row][start + local_col] = value;
                }
            }
        }

        start = end;
    }

    let p = Array2::from_shape_fn((4, 4), |(row, col)| Complex64::new(p_real[row][col], 0.0));
    validate_real_orthogonal("Autonne P", &p)?;

    let p_t = p.t().to_owned();
    let p_t_m2_p = p_t.dot(m2).dot(&p);
    let d: [Complex64; 4] = [
        p_t_m2_p[[0, 0]],
        p_t_m2_p[[1, 1]],
        p_t_m2_p[[2, 2]],
        p_t_m2_p[[3, 3]],
    ];

    let mut max_off_diag = 0.0_f64;
    for row in 0..4 {
        for col in 0..4 {
            if row != col {
                max_off_diag = max_off_diag.max(p_t_m2_p[[row, col]].norm());
            }
        }
    }
    if max_off_diag > AUTONNE_EPS {
        return Err(kak_failed(format!(
            "Autonne diagonalization left off-diagonal residual {max_off_diag}"
        )));
    }

    let diag_d = Array2::from_diag(&Array1::from_vec(d.to_vec()));
    let compare = p.dot(&diag_d.dot(&p_t));
    let max_err = max_entry_diff(&compare, m2);
    if max_err > AUTONNE_EPS {
        return Err(kak_failed(format!(
            "Autonne reconstruction failed: max entry difference {max_err}"
        )));
    }

    Ok((p, d))
}

fn validate_autonne_input(m2: &Array2<Complex64>) -> Result<(), CompilerError> {
    let mut max_commutator = 0.0_f64;
    for row in 0..4 {
        for col in 0..4 {
            let mut ab = 0.0_f64;
            let mut ba = 0.0_f64;
            for k in 0..4 {
                ab += m2[[row, k]].re * m2[[k, col]].im;
                ba += m2[[row, k]].im * m2[[k, col]].re;
            }
            max_commutator = max_commutator.max((ab - ba).abs());
        }
    }

    if max_commutator > AUTONNE_EPS {
        return Err(kak_failed(format!(
            "Autonne input Re(M2) and Im(M2) do not commute: max commutator {max_commutator}"
        )));
    }

    Ok(())
}

fn validate_real_orthogonal(
    name: &'static str,
    m: &Array2<Complex64>,
) -> Result<(), CompilerError> {
    for row in 0..4 {
        for col in 0..4 {
            if m[[row, col]].im.abs() > AUTONNE_EPS {
                return Err(kak_failed(format!(
                    "{name} has non-real entry at ({row}, {col})"
                )));
            }
        }
    }

    let product = m.t().dot(m);
    for row in 0..4 {
        for col in 0..4 {
            let expected = if row == col {
                Complex64::new(1.0, 0.0)
            } else {
                Complex64::new(0.0, 0.0)
            };
            let diff = (product[[row, col]] - expected).norm();
            if diff > AUTONNE_EPS {
                return Err(kak_failed(format!(
                    "{name} is not orthogonal: (P^T P)[{row},{col}] differs by {diff}"
                )));
            }
        }
    }

    Ok(())
}

fn decompose_two_qubit_product_gate(
    m: &Array2<Complex64>,
) -> Result<(Array2<Complex64>, Array2<Complex64>, f64), CompilerError> {
    let rearranged = Array2::from_shape_fn((4, 4), |(row, col)| {
        let left_row = row / 2;
        let left_col = row % 2;
        let right_row = col / 2;
        let right_col = col % 2;
        m[[2 * left_row + right_row, 2 * left_col + right_col]]
    });

    let svd = Mat::from_fn(rearranged.nrows(), rearranged.ncols(), |row, col| {
        let value = rearranged[[row, col]];
        faer::c64::new(value.re, value.im)
    })
    .thin_svd()
    .map_err(|e| kak_failed(format!("Kronecker rank-1 SVD failed: {e:?}")))?;
    let singular_values = svd.S();
    let leading = singular_values[0].re;
    let trailing = if singular_values.dim() > 1 {
        singular_values[1].re.abs()
    } else {
        0.0
    };

    if leading <= KRON_RANK_EPS {
        return Err(kak_failed(
            "Kronecker factorization has zero leading singular value",
        ));
    }
    if trailing > KRON_RANK_EPS * leading.max(1.0) {
        return Err(kak_failed(format!(
            "Kronecker factorization is not rank-1: second singular value {trailing}"
        )));
    }

    let scale = leading.sqrt();
    let u = svd.U();
    let v = svd.V();
    let l_vec: [Complex64; 4] = std::array::from_fn(|i| {
        let value = u[(i, 0)];
        scale * Complex64::new(value.re, value.im)
    });
    let r_vec: [Complex64; 4] = std::array::from_fn(|i| {
        let value = v[(i, 0)];
        scale * Complex64::new(value.re, value.im).conj()
    });

    let l_raw = array![[l_vec[0], l_vec[1]], [l_vec[2], l_vec[3]]];
    let r_raw = array![[r_vec[0], r_vec[1]], [r_vec[2], r_vec[3]]];
    let det_l = det_2x2(&l_raw);
    let det_r = det_2x2(&r_raw);
    if det_l.norm() < KRON_RANK_EPS || det_r.norm() < KRON_RANK_EPS {
        return Err(kak_failed(format!(
            "Kronecker factors have near-zero determinants: det_l={det_l}, det_r={det_r}"
        )));
    }

    let det_l_sqrt = det_l.sqrt();
    let det_r_sqrt = det_r.sqrt();
    let one = Complex64::new(1.0, 0.0);
    let minus_one = Complex64::new(-1.0, 0.0);

    let mut best: Option<(Array2<Complex64>, Array2<Complex64>, f64, f64)> = None;
    for sign_l in [one, minus_one] {
        for sign_r in [one, minus_one] {
            let sqrt_l = sign_l * det_l_sqrt;
            let sqrt_r = sign_r * det_r_sqrt;
            let l = l_raw.mapv(|value| value / sqrt_l);
            let r = r_raw.mapv(|value| value / sqrt_r);
            let phase = (sqrt_l * sqrt_r).arg();
            let reconstructed = ndarray::linalg::kron(&l, &r)
                .mapv(|value| Complex64::from_polar(1.0, phase) * value);
            let error = max_entry_diff(&reconstructed, m);

            let is_better = match best.as_ref() {
                Some((_, _, _, best_error)) => error < *best_error,
                None => true,
            };
            if is_better {
                best = Some((l, r, phase, error));
            }
        }
    }

    let Some((l, r, phase, max_diff)) = best else {
        return Err(kak_failed(
            "Kronecker product decomposition produced no phase branch",
        ));
    };

    if max_diff > KRON_RANK_EPS {
        return Err(kak_failed(format!(
            "Kronecker product decomposition failed: max difference {max_diff}"
        )));
    }

    Ok((l, r, phase))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use ndarray::linalg::kron;

    const CARTAN_EPS: f64 = 1e-7;
    const MATRIX_EPS: f64 = 1e-7;

    fn reconstructed_from_kak(decomp: &KakDecomposition) -> Array2<Complex64> {
        let k1 = kron(&decomp.k1l, &decomp.k1r);
        let k2 = kron(&decomp.k2l, &decomp.k2r);
        let rxx = StandardGate::RXX
            .matrix(&[-2.0 * decomp.a])
            .unwrap()
            .into_owned();
        let ryy = StandardGate::RYY
            .matrix(&[-2.0 * decomp.b])
            .unwrap()
            .into_owned();
        let rzz = StandardGate::RZZ
            .matrix(&[-2.0 * decomp.c])
            .unwrap()
            .into_owned();
        let phase = Complex64::from_polar(1.0, decomp.global_phase);

        k1.dot(&rxx.dot(&ryy.dot(&rzz.dot(&k2))))
            .mapv(|value| phase * value)
    }

    fn assert_weyl_chamber(decomp: &KakDecomposition) {
        assert!(
            decomp.a >= -CARTAN_EPS && decomp.a <= FRAC_PI_4 + CARTAN_EPS,
            "a={} not in Weyl chamber",
            decomp.a
        );
        assert!(
            decomp.b >= -CARTAN_EPS && decomp.b <= decomp.a + CARTAN_EPS,
            "b={} not in Weyl chamber for a={}",
            decomp.b,
            decomp.a
        );
        assert!(
            decomp.c.abs() <= decomp.b + CARTAN_EPS,
            "c={} not in Weyl chamber for b={}",
            decomp.c,
            decomp.b
        );
    }

    #[test]
    fn reconstructs_common_standard_gates() {
        let cases = [
            Array2::eye(4),
            StandardGate::CX.matrix(&[]).unwrap().into_owned(),
            StandardGate::CZ.matrix(&[]).unwrap().into_owned(),
            StandardGate::SWAP.matrix(&[]).unwrap().into_owned(),
            StandardGate::FSIM
                .matrix(&[0.2, -0.3])
                .unwrap()
                .into_owned(),
        ];

        for matrix in cases {
            let decomp = kak_decompose(&matrix).unwrap();
            assert_weyl_chamber(&decomp);
            assert_abs_diff_eq!(
                matrix,
                reconstructed_from_kak(&decomp),
                epsilon = MATRIX_EPS
            );
        }
    }

    #[test]
    fn recovers_cartan_core_coordinates() {
        let expected_a = 0.31;
        let expected_b = 0.17;
        let expected_c = -0.08;
        let rxx = StandardGate::RXX
            .matrix(&[-2.0 * expected_a])
            .unwrap()
            .into_owned();
        let ryy = StandardGate::RYY
            .matrix(&[-2.0 * expected_b])
            .unwrap()
            .into_owned();
        let rzz = StandardGate::RZZ
            .matrix(&[-2.0 * expected_c])
            .unwrap()
            .into_owned();
        let matrix = rxx.dot(&ryy.dot(&rzz));
        let decomp = kak_decompose(&matrix).unwrap();

        assert!((decomp.a - expected_a).abs() < CARTAN_EPS);
        assert!((decomp.b - expected_b).abs() < CARTAN_EPS);
        assert!((decomp.c - expected_c).abs() < CARTAN_EPS);
        assert_abs_diff_eq!(
            matrix,
            reconstructed_from_kak(&decomp),
            epsilon = MATRIX_EPS
        );
    }

    #[test]
    fn reconstructs_constructed_local_product_with_global_phase() {
        let k1l = StandardGate::U
            .matrix(&[0.4, -0.2, 0.7])
            .unwrap()
            .into_owned();
        let k1r = StandardGate::U
            .matrix(&[1.1, 0.3, -0.5])
            .unwrap()
            .into_owned();
        let k2l = StandardGate::U
            .matrix(&[0.8, -0.9, 0.2])
            .unwrap()
            .into_owned();
        let k2r = StandardGate::U
            .matrix(&[0.6, 1.0, -0.4])
            .unwrap()
            .into_owned();
        let rxx = StandardGate::RXX.matrix(&[-0.42]).unwrap().into_owned();
        let ryy = StandardGate::RYY.matrix(&[-0.28]).unwrap().into_owned();
        let rzz = StandardGate::RZZ.matrix(&[0.12]).unwrap().into_owned();
        let local_left = kron(&k1l, &k1r);
        let local_right = kron(&k2l, &k2r);
        let phase = Complex64::from_polar(1.0, 0.37);
        let matrix = local_left
            .dot(&rxx.dot(&ryy.dot(&rzz.dot(&local_right))))
            .mapv(|value| phase * value);

        let decomp = kak_decompose(&matrix).unwrap();
        assert_weyl_chamber(&decomp);
        assert_abs_diff_eq!(
            matrix,
            reconstructed_from_kak(&decomp),
            epsilon = MATRIX_EPS
        );
    }

    #[test]
    fn reconstructs_weyl_boundary_cases_with_global_phase() {
        let k1l = StandardGate::U
            .matrix(&[0.2, -0.4, 0.9])
            .unwrap()
            .into_owned();
        let k1r = StandardGate::U
            .matrix(&[1.0, 0.8, -0.7])
            .unwrap()
            .into_owned();
        let k2l = StandardGate::U
            .matrix(&[0.7, -0.5, 0.1])
            .unwrap()
            .into_owned();
        let k2r = StandardGate::U
            .matrix(&[0.3, 0.6, -0.2])
            .unwrap()
            .into_owned();
        let local_left = kron(&k1l, &k1r);
        let local_right = kron(&k2l, &k2r);
        let cases = [
            (FRAC_PI_4, FRAC_PI_4, 0.0, -0.73),
            (FRAC_PI_4, 0.19, 0.19, 1.1),
            (0.33, 0.21, -0.21, -1.4),
            (0.0, 0.0, 0.0, 0.92),
        ];

        for (a, b, c, phase) in cases {
            let rxx = StandardGate::RXX.matrix(&[-2.0 * a]).unwrap().into_owned();
            let ryy = StandardGate::RYY.matrix(&[-2.0 * b]).unwrap().into_owned();
            let rzz = StandardGate::RZZ.matrix(&[-2.0 * c]).unwrap().into_owned();
            let matrix = local_left
                .dot(&rxx.dot(&ryy.dot(&rzz.dot(&local_right))))
                .mapv(|value| Complex64::from_polar(1.0, phase) * value);

            let decomp = kak_decompose(&matrix).unwrap();
            assert_weyl_chamber(&decomp);
            assert_abs_diff_eq!(
                matrix,
                reconstructed_from_kak(&decomp),
                epsilon = MATRIX_EPS
            );
        }
    }

    #[test]
    fn rejects_non_4x4_input() {
        let matrix = Array2::eye(2);
        let err = kak_decompose(&matrix).unwrap_err();
        assert!(err.to_string().contains("expected 4x4"));
    }

    #[test]
    fn rejects_non_unitary_4x4_input() {
        let mut matrix = Array2::eye(4);
        matrix[[3, 3]] = Complex64::new(2.0, 0.0);

        let err = kak_decompose(&matrix).unwrap_err();
        assert!(err.to_string().contains("not unitary"));
    }

    #[test]
    fn rejects_zero_matrix() {
        let matrix = Array2::zeros((4, 4));
        let err = kak_decompose(&matrix).unwrap_err();
        assert!(err.to_string().contains("not unitary"));
    }
}
