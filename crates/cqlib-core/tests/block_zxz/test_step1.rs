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
//! 对应 Python `test/test_block_ZXZ/test_step1.py`。

use crate::common::generate_random_unitary;

use cqlib_core::compile::transform::decompose::block_zxz::matrix::{
    block_diag, eye, mat_mul, max_abs_diff, Cm,
};
use cqlib_core::compile::transform::decompose::block_zxz::step1::{
    block_zxz_step1, is_matrix_unitary,
};

const ATOL: f64 = 1e-10;

/// 由第一步因子按矩阵方程重建酉矩阵。
fn reconstruct_step1(a1: &Cm, a2: &Cm, b: &Cm, c: &Cm) -> Cm {
    let block_a = block_diag(a1, a2);
    let identity = eye(b.nrows());
    let mut middle = Cm::zeros((2 * b.nrows(), 2 * b.nrows()));
    let n = b.nrows();
    for i in 0..n {
        for j in 0..n {
            middle[[i, j]] = 0.5 * (identity[[i, j]] + b[[i, j]]);
            middle[[i, n + j]] = 0.5 * (identity[[i, j]] - b[[i, j]]);
            middle[[n + i, j]] = 0.5 * (identity[[i, j]] - b[[i, j]]);
            middle[[n + i, n + j]] = 0.5 * (identity[[i, j]] + b[[i, j]]);
        }
    }
    let block_c = block_diag(&eye(c.nrows()), c);
    mat_mul(&mat_mul(&block_a, &middle), &block_c)
}

/// 对 n=3..6 验证 Step 1 因子幺酉性与矩阵方程重建。
#[test]
fn step1_factors_unitary_and_reconstructs() {
    for num_qubits in 3..=6usize {
        let unitary = generate_random_unitary(num_qubits, 42);
        let (a1, a2, b, c) = block_zxz_step1(&unitary, ATOL).expect("step1 failed");

        assert!(is_matrix_unitary(&a1, ATOL), "A1 not unitary");
        assert!(is_matrix_unitary(&a2, ATOL), "A2 not unitary");
        assert!(is_matrix_unitary(&b, ATOL), "B not unitary");
        assert!(is_matrix_unitary(&c, ATOL), "C not unitary");

        let reconstructed = reconstruct_step1(&a1, &a2, &b, &c);
        let err = max_abs_diff(&reconstructed, &unitary);
        assert!(
            err <= ATOL,
            "n={num_qubits}: step1 equation error too large: {err}"
        );
    }
}

/// 无效输入应报错（对应 Python 的 ValueError）。
#[test]
fn step1_rejects_invalid_input() {
    // 3x3 非 2 幂维度
    let bad = Cm::eye(3);
    assert!(block_zxz_step1(&bad, ATOL).is_err(), "non power-of-two accepted");
}