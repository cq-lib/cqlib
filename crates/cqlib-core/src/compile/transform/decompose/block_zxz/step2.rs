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
//! Block-ZXZ 第二步：解复用（Demultiplexing）。
//!
//! 逐函数对应 Python `decompose/block_ZXZ/step2.py`。

use super::matrix::{adjoint, eye, mat_mul, max_abs_diff, Cm};
use super::step1::{assert_square_unitary, is_matrix_diagonal, is_matrix_unitary};

/// 对幺酉矩阵做复 Schur 风格对角化，返回 `(Q, D)`。
///
/// 对应 Python `schur_unitary_diagonalize`。
pub fn schur_unitary_diagonalize(matrix: &Cm, atol: f64) -> Result<(Cm, Cm), String> {
    assert_square_unitary(matrix, atol)?;
    Ok(super::matrix::schur_unitary_diagonalize(matrix))
}

/// 由单位模本征值构造对角幺酉平方根矩阵 `D`。
///
/// 对应 Python `build_unitary_diagonal_sqrt`。
pub fn build_unitary_diagonal_sqrt(eigenvalues: &Cm) -> Cm {
    super::matrix::build_unitary_diagonal_sqrt(eigenvalues)
}

/// 校验解复用结果是否满足定义方程。
fn validate_demux_result(
    u0: &Cm,
    u1: &Cm,
    v: &Cm,
    d: &Cm,
    w: &Cm,
    atol: f64,
) -> Result<(), String> {
    if !is_matrix_unitary(v, atol) {
        return Err("Demux factor V is not unitary.".into());
    }
    if !is_matrix_unitary(w, atol) {
        return Err("Demux factor W is not unitary.".into());
    }
    if !is_matrix_diagonal(d, atol) {
        return Err("Demux factor D is not diagonal.".into());
    }
    if !is_matrix_unitary(d, atol) {
        return Err("Demux factor D is not unitary.".into());
    }
    let left = mat_mul(&mat_mul(v, d), w);
    let right = mat_mul(&mat_mul(v, &adjoint(d)), w);
    if max_abs_diff(&left, u0) > atol {
        return Err("Demux reconstruction of U0 failed.".into());
    }
    if max_abs_diff(&right, u1) > atol {
        return Err("Demux reconstruction of U1 failed.".into());
    }
    Ok(())
}

/// 把 `diag(U0, U1) = diag(V, V) @ diag(D, D†) @ diag(W, W)` 解复用为 `(V, D, W)`。
///
/// 对应 Python `demux_pair`。
pub fn demux_pair(u0: &Cm, u1: &Cm, atol: f64) -> Result<(Cm, Cm, Cm), String> {
    assert_square_unitary(u0, atol)?;
    assert_square_unitary(u1, atol)?;
    if u0.dim() != u1.dim() {
        return Err("U0 and U1 must have the same shape.".into());
    }

    let m_matrix = mat_mul(u0, &adjoint(u1));
    let (v_factor, eigenvalues) = schur_unitary_diagonalize(&m_matrix, atol)?;
    let d_factor = build_unitary_diagonal_sqrt(&eigenvalues);
    let w_factor = mat_mul(&mat_mul(&d_factor, &adjoint(&v_factor)), u1);

    validate_demux_result(u0, u1, &v_factor, &d_factor, &w_factor, atol)?;
    Ok((v_factor, d_factor, w_factor))
}

/// 解复用 A 分支 `(A1, A2)`。
pub fn demux_a_branch(a1: &Cm, a2: &Cm, atol: f64) -> Result<(Cm, Cm, Cm), String> {
    demux_pair(a1, a2, atol)
}

/// 解复用 C 分支 `(I, C)`。
pub fn demux_c_branch(c: &Cm, atol: f64) -> Result<(Cm, Cm, Cm), String> {
    assert_square_unitary(c, atol)?;
    let identity = eye(c.nrows());
    demux_pair(&identity, c, atol)
}

/// 构造中路块对（吸收 `W_A` 与 `V_C`）：`(W_A @ V_C, W_A @ B @ V_C)`。
pub fn build_middle_pair(w_a: &Cm, b: &Cm, v_c: &Cm) -> (Cm, Cm) {
    (mat_mul(w_a, v_c), mat_mul(&mat_mul(w_a, b), v_c))
}

/// 解复用中路块对。
pub fn demux_middle_branch(w_a: &Cm, b: &Cm, v_c: &Cm, atol: f64) -> Result<(Cm, Cm, Cm), String> {
    let (u0, u1) = build_middle_pair(w_a, b, v_c);
    demux_pair(&u0, &u1, atol)
}

/// 构造作用于 (n-1) 比特子块 MSB 的 `Z ⊗ I` 对角矩阵。
///
/// 维度为 `2^{n-1} x 2^{n-1}`，前半对角 +1、后半对角 -1。
fn z_on_subblock_msb(sub_qubits: usize) -> Cm {
    let dim = 1usize << sub_qubits;
    let half = dim / 2;
    let mut z = super::matrix::eye(dim);
    for i in half..dim {
        z[[i, i]] = num_complex::Complex64::new(-1.0, 0.0);
    }
    z
}

/// 构造 A.1 优化版的中路块对（论文 Eq. 11）。
///
/// `B̃ = diag(W_A V_C, (Z⊗I)·W_A·B·V_C·(Z⊗I))`，其中 `Z⊗I` 作用于
/// (n-1) 比特子块的 MSB。该 `B̃` 与两支 UCRz 中被去除的 CNOT 通过
/// `H·CX̄ = CZ·H` / `CX̄·H = H·CZ` 恒等式等价吸收。
pub fn build_middle_pair_a1(w_a: &Cm, b: &Cm, v_c: &Cm) -> (Cm, Cm) {
    let sub_qubits = (w_a.nrows() as f64).log2().round() as usize;
    let z = z_on_subblock_msb(sub_qubits);
    let u0 = mat_mul(w_a, v_c);
    let inner = mat_mul(&mat_mul(w_a, b), v_c);
    let u1 = mat_mul(&mat_mul(&z, &inner), &z);
    (u0, u1)
}

/// 解复用 A.1 优化版的中路块对。
pub fn demux_middle_branch_a1(
    w_a: &Cm,
    b: &Cm,
    v_c: &Cm,
    atol: f64,
) -> Result<(Cm, Cm, Cm), String> {
    let (u0, u1) = build_middle_pair_a1(w_a, b, v_c);
    demux_pair(&u0, &u1, atol)
}

/// Block-ZXZ 第二步的结果（对应 Python 返回的 `dict`）。
#[derive(Debug, Clone)]
pub struct Step2Result {
    /// `(V, D, W)` for the A branch.
    pub a_demux: (Cm, Cm, Cm),
    /// `(V, D, W)` for the C branch.
    pub c_demux: (Cm, Cm, Cm),
    /// Middle pair `(U0_mid, U1_mid)`.
    pub middle_pair: (Cm, Cm),
    /// `(V, D, W)` for the middle branch.
    pub middle_demux: (Cm, Cm, Cm),
}

/// 执行 Block-ZXZ 第二步。
///
/// 对应 Python `block_zxz_step2`。
pub fn block_zxz_step2(
    a1: &Cm,
    a2: &Cm,
    b: &Cm,
    c: &Cm,
    atol: f64,
) -> Result<Step2Result, String> {
    let a_demux = demux_a_branch(a1, a2, atol)?;
    let c_demux = demux_c_branch(c, atol)?;
    let middle_pair = build_middle_pair(&a_demux.2, b, &c_demux.0);
    let middle_demux = demux_pair(&middle_pair.0, &middle_pair.1, atol)?;

    Ok(Step2Result {
        a_demux,
        c_demux,
        middle_pair,
        middle_demux,
    })
}