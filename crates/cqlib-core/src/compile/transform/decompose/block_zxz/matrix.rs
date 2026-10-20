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
//! 复数矩阵工具层。
//!
//! 对应 Python 项目中基于 `numpy`/`scipy` 的矩阵运算。这里以 `ndarray::Array2<Complex64>`
//! 作为主矩阵类型（与 cqlib-core 交互一致），仅在对角化/奇异值分解时临时借用 `faer`。

use num_complex::Complex64;
use ndarray::{Array2, s};

/// 复矩阵别名。
pub type Cm = Array2<Complex64>;

/// 构造 `n x n` 单位矩阵。
pub fn eye(n: usize) -> Cm {
    Array2::eye(n)
}

/// 矩阵乘积 `A @ B`。
#[inline]
pub fn mat_mul(a: &Cm, b: &Cm) -> Cm {
    a.dot(b)
}

/// 共轭转置 `A^H`。
#[inline]
pub fn adjoint(a: &Cm) -> Cm {
    a.t().mapv(|z| z.conj())
}

/// 由对角值列表构造对角矩阵。
pub fn diagonal_from_values(values: &[Complex64]) -> Cm {
    let n = values.len();
    let mut out = Array2::<Complex64>::zeros((n, n));
    for (i, &v) in values.iter().enumerate() {
        out[[i, i]] = v;
    }
    out
}

/// 构造分块对角矩阵 `diag(top_left, bottom_right)`。
pub fn block_diag(top_left: &Cm, bottom_right: &Cm) -> Cm {
    let td = top_left.dim().0;
    let bd = bottom_right.dim().0;
    let n = td + bd;
    let mut out = Array2::<Complex64>::zeros((n, n));
    out.slice_mut(s![..td, ..td]).assign(top_left);
    out.slice_mut(s![td.., td..]).assign(bottom_right);
    out
}

/// 逐元素最大绝对差（用于 allclose 判断）。
pub fn max_abs_diff(a: &Cm, b: &Cm) -> f64 {
    let mut m = 0.0f64;
    for ((i, j), &av) in a.indexed_iter() {
        let d = (av - b[[i, j]]).norm();
        if d > m {
            m = d;
        }
    }
    m
}

/// `A` 是否为 `atol` 容忍度下的幺酉矩阵。
pub fn is_unitary(a: &Cm, atol: f64) -> bool {
    if a.nrows() != a.ncols() {
        return false;
    }
    let n = a.nrows();
    max_abs_diff(&mat_mul(&adjoint(a), a), &eye(n)) <= atol
}

/// `A` 是否为 `atol` 容忍度下的对角矩阵。
pub fn is_diagonal(a: &Cm, atol: f64) -> bool {
    let (r, c) = a.dim();
    for i in 0..r {
        for j in 0..c {
            if i != j && a[[i, j]].norm() > atol {
                return false;
            }
        }
    }
    true
}

// ---- faer 互转 ----

/// 将 ndarray 矩阵复制为 faer 矩阵。
pub fn to_faer(a: &Cm) -> faer::Mat<Complex64> {
    let (r, c) = a.dim();
    faer::Mat::<Complex64>::from_fn(r, c, |i, j| a[[i, j]])
}

/// 将 faer 矩阵引用复制为 ndarray 矩阵。
pub fn from_faer(m: &faer::MatRef<'_, Complex64>) -> Cm {
    Array2::from_shape_fn((m.nrows(), m.ncols()), |(i, j)| m[(i, j)])
}

// ---- 数值分解（对应 numpy.linalg.svd / scipy.linalg.schur）----

/// 通过 SVD 做极分解：返回 `(S, U_polar)`，满足 `matrix ≈ S @ U_polar`。
///
/// 对应 Python `polar_from_svd`：`X = U Σ V^H`，`S = U Σ U^H`，`U_polar = U V^H`。
pub fn polar_from_svd(matrix: &Cm) -> (Cm, Cm) {
    let fm = to_faer(matrix);
    let svd = fm.svd().expect("SVD failed");
    let u = from_faer(&svd.U());
    let v = from_faer(&svd.V());
    let s_vals: Vec<f64> = fm.singular_values().expect("singular values failed");
    let sigma = diagonal_from_real(&s_vals);
    let s_factor = mat_mul(&mat_mul(&u, &sigma), &adjoint(&u));
    let u_polar = mat_mul(&u, &adjoint(&v));
    (s_factor, u_polar)
}

/// 对复幺酉矩阵做特征（Schur 风格）对角化。
///
/// 返回 `(Q, D)`，满足 `matrix ≈ Q D Q^H`，`D` 为特征值对角阵。
/// 对应 Python `schur_unitary_diagonalize`（`scipy.linalg.schur` 输出 `complex`）。
///
/// 退化谱修复：faer 的复 EVD 对重复特征值不保证特征向量互相正交
/// （正规矩阵理论上可幺正对角化）。这里按特征值聚类分组、组内做修正
/// Gram-Schmidt：简并子空间内的正交基仍张成同一特征子空间，对角化
/// 性质不变；不同特征值的子空间由正规性自然（数值上）正交。
pub fn schur_unitary_diagonalize(matrix: &Cm) -> (Cm, Cm) {
    let fm = to_faer(matrix);
    let eig = fm.eigen().expect("EVD failed");
    let q_raw = from_faer(&eig.U());
    let evals: Vec<Complex64> = fm.eigenvalues().expect("eigenvalues failed");
    let q = reorthogonalize_degenerate_eigenvectors(&q_raw, &evals);
    let d = diagonal_from_values(&evals);
    (q, d)
}

/// 把特征向量矩阵在简并子空间内重新正交化（见
/// `schur_unitary_diagonalize` 的文档）。
///
/// 同一特征值聚类内的特征向量做修正 Gram-Schmidt；不同聚类之间的
/// 正交性由正规矩阵谱的性质自然保持。单位圆上精确简并在浮点上呈
/// `~1e-15` 量级抖动，非简并特征值间隔通常为 `O(0.1)` 以上，聚类
/// 容差取 `1e-7` 安全。本函数不改变各特征向量的所属子空间，因此
/// 对角化关系 `M = Q D Q^H` 仍然成立。
fn reorthogonalize_degenerate_eigenvectors(q: &Cm, evals: &[Complex64]) -> Cm {
    let n = q.ncols();
    debug_assert_eq!(q.nrows(), n);
    debug_assert_eq!(evals.len(), n);

    const CLUSTER_TOL: f64 = 1e-7;

    // 聚类：同一特征值（单位圆上精确简并 + 浮点抖动）的列归入同组。
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
        // 组内修正 Gram-Schmidt（复内积），子空间不变。
        for (pos, &col) in group.iter().enumerate() {
            let mut v: Vec<Complex64> = (0..q.nrows()).map(|r| out[[r, col]]).collect();
            for &prev in group.iter().take(pos) {
                let mut proj = Complex64::new(0.0, 0.0);
                for r in 0..q.nrows() {
                    proj += out[[r, prev]].conj() * v[r];
                }
                for r in 0..q.nrows() {
                    v[r] -= proj * out[[r, prev]];
                }
            }
            let norm: f64 = v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
            if norm > 1e-12 {
                for r in 0..q.nrows() {
                    out[[r, col]] = v[r] / norm;
                }
            }
            // norm 极小时保留原向量（faer 已保证其为特征向量且非零；
            // 该情形意味着组内原向量近线性相关，GS 无法正交化，但不
            // 会比修复前更差）。
        }
    }
    out
}

/// 由实数值列表构造复对角阵（虚部为零）。
fn diagonal_from_real(values: &[f64]) -> Cm {
    let v: Vec<Complex64> = values.iter().map(|&x| Complex64::new(x, 0.0)).collect();
    diagonal_from_values(&v)
}

/// 从单位模本征值构造对角幺酉平方根矩阵。
///
/// 对应 Python `build_unitary_diagonal_sqrt`：
/// `normalized = e / |e|`，`phase = exp(0.5j * arg(normalized))`。
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
        // sqrt(e^{iθ}) = e^{iθ/2}
        d[[i, i]] = normalized.sqrt();
    }
    d
}