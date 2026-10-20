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
//! Block-ZXZ 第一步：分块 + 极分解 + 求解因子 `A1, A2, B, C`。
//!
//! 逐函数对应 Python `decompose/block_ZXZ/step1.py`。

use super::matrix::{adjoint, eye, is_unitary, mat_mul, Cm};
use num_complex::Complex64;
use ndarray::s;

/// `value` 是否为正的 2 的幂。
pub fn is_power_of_two(value: usize) -> bool {
    value > 0 && (value & (value - 1)) == 0
}

/// `matrix` 是否在 `atol` 内为幺酉矩阵。
pub fn is_matrix_unitary(matrix: &Cm, atol: f64) -> bool {
    is_unitary(matrix, atol)
}

/// `matrix` 是否在 `atol` 内为对角矩阵。
pub fn is_matrix_diagonal(matrix: &Cm, atol: f64) -> bool {
    let (r, c) = matrix.dim();
    for i in 0..r {
        for j in 0..c {
            if i != j && matrix[[i, j]].norm() > atol {
                return false;
            }
        }
    }
    true
}

/// 校验输入为 `2^n x 2^n` 幺酉矩阵。
///
/// 对应 Python `assert_square_unitary`，失败返回 `Err(String)`。
pub fn assert_square_unitary(matrix: &Cm, atol: f64) -> Result<(), String> {
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
    if !is_unitary(matrix, atol) {
        return Err("Input matrix must be unitary.".into());
    }
    Ok(())
}

/// 按最高位把 `unitary` 切成四个等长子块。
///
/// 对应 Python `split_unitary_blocks`，返回 `(X, Y, U21, U22)`。
pub fn split_unitary_blocks(unitary: &Cm) -> (Cm, Cm, Cm, Cm) {
    assert_square_unitary(unitary, 1e-10).expect("invalid unitary");
    let dim = unitary.nrows();
    let half = dim / 2;
    (
        unitary.slice(s![..half, ..half]).to_owned(),
        unitary.slice(s![..half, half..]).to_owned(),
        unitary.slice(s![half.., ..half]).to_owned(),
        unitary.slice(s![half.., half..]).to_owned(),
    )
}

/// 通过 SVD 做极分解，返回 `(S, U_polar)`。
pub fn polar_from_svd(matrix: &Cm) -> (Cm, Cm) {
    super::matrix::polar_from_svd(matrix)
}

/// 计算 Block-ZXZ 因子 `C`：`C = -i · Ux^H @ Uy`。
pub fn compute_c_from_polar(u_x: &Cm, u_y: &Cm) -> Cm {
    let mut c = mat_mul(&adjoint(u_x), u_y);
    c *= Complex64::new(0.0, -1.0);
    c
}

/// 计算 Block-ZXZ 因子 `A1 = (Sx + i Sy) @ Ux`。
pub fn compute_a1(s_x: &Cm, s_y: &Cm, u_x: &Cm) -> Cm {
    let mut inner = s_x.clone();
    let iy = s_y.mapv(|z| Complex64::new(0.0, 1.0) * z);
    inner = inner + &iy;
    mat_mul(&inner, u_x)
}

/// 计算 Block-ZXZ 因子 `A2 = U21 + U22 @ (i Uy^H Ux)`。
pub fn compute_a2(u21: &Cm, u22: &Cm, u_y: &Cm, u_x: &Cm) -> Cm {
    let t = mat_mul(&adjoint(u_y), u_x).mapv(|z| Complex64::new(0.0, 1.0) * z);
    let t = mat_mul(u22, &t);
    u21 + &t
}

/// 计算 Block-ZXZ 因子 `B = 2 A1^H @ X - I`。
pub fn compute_b(a1: &Cm, x: &Cm) -> Cm {
    let identity = eye(x.nrows());
    let t = mat_mul(&adjoint(a1), x).mapv(|z| z * 2.0);
    t - &identity
}

/// 校验第一步输出的 `A1/A2/B/C` 是否均为幺酉。
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

/// 执行 Block-ZXZ 第一步，返回 `(A1, A2, B, C)`。
pub fn block_zxz_step1(unitary: &Cm, atol: f64) -> Result<(Cm, Cm, Cm, Cm), String> {
    assert_square_unitary(unitary, atol)?;
    let n = unitary.nrows();
    let num_qubits = (n as f64).log2().round() as usize;
    if num_qubits < 2 {
        return Err("Block-ZXZ Step 1 expects at least a 2-qubit unitary.".into());
    }

    let (x, y, u21, u22) = split_unitary_blocks(unitary);
    let (s_x, u_x) = polar_from_svd(&x);
    let (s_y, u_y) = polar_from_svd(&y);

    let c = compute_c_from_polar(&u_x, &u_y);
    let a1 = compute_a1(&s_x, &s_y, &u_x);
    let a2 = compute_a2(&u21, &u22, &u_y, &u_x);
    let b = compute_b(&a1, &x);

    assert_step1_outputs_unitary(&a1, &a2, &b, &c, atol)?;
    Ok((a1, a2, b, c))
}