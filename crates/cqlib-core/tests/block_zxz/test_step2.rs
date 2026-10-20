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
//! 对应 Python `test/test_block_ZXZ/test_step2.py`。

use crate::common::generate_random_unitary;

use cqlib_core::compile::transform::decompose::block_zxz::matrix::{
    adjoint, block_diag, eye, mat_mul, max_abs_diff, Cm,
};
use cqlib_core::compile::transform::decompose::block_zxz::step1::block_zxz_step1;
use cqlib_core::compile::transform::decompose::block_zxz::step2::block_zxz_step2;

const ATOL: f64 = 1e-10;

/// `(V D W, V D† W)` 重建 demux 对。
fn reconstruct_demux_pair(v: &Cm, d: &Cm, w: &Cm) -> (Cm, Cm) {
    (mat_mul(&mat_mul(v, d), w), mat_mul(&mat_mul(v, &adjoint(d)), w))
}

/// 验证 A 分支 `diag(A1, A2)` 重建。
fn validate_a_branch(a1: &Cm, a2: &Cm, a_demux: &(Cm, Cm, Cm)) -> f64 {
    let (u0, u1) = reconstruct_demux_pair(&a_demux.0, &a_demux.1, &a_demux.2);
    let original = block_diag(a1, a2);
    let reconstructed = block_diag(&u0, &u1);
    max_abs_diff(&reconstructed, &original)
}

/// 验证 C 分支 `diag(I, C)` 重建。
fn validate_c_branch(c: &Cm, c_demux: &(Cm, Cm, Cm)) -> f64 {
    let (u0, u1) = reconstruct_demux_pair(&c_demux.0, &c_demux.1, &c_demux.2);
    let original = block_diag(&eye(c.nrows()), c);
    let reconstructed = block_diag(&u0, &u1);
    max_abs_diff(&reconstructed, &original)
}

/// 验证中路分支重建。
fn validate_b_branch(middle_pair: &(Cm, Cm), middle_demux: &(Cm, Cm, Cm)) -> f64 {
    let (u0, u1) = reconstruct_demux_pair(&middle_demux.0, &middle_demux.1, &middle_demux.2);
    let original = block_diag(&middle_pair.0, &middle_pair.1);
    let reconstructed = block_diag(&u0, &u1);
    max_abs_diff(&reconstructed, &original)
}

/// n=3..6：验证 A/C/B 三个 demux 分支都能正确重建。
#[test]
fn step2_branches_restored() {
    for num_qubits in 3..=6usize {
        let unitary = generate_random_unitary(num_qubits, 42);
        let (a1, a2, b, c) = block_zxz_step1(&unitary, ATOL).expect("step1 failed");
        let step2 = block_zxz_step2(&a1, &a2, &b, &c, ATOL).expect("step2 failed");

        let a_err = validate_a_branch(&a1, &a2, &step2.a_demux);
        let c_err = validate_c_branch(&c, &step2.c_demux);
        let b_err = validate_b_branch(&step2.middle_pair, &step2.middle_demux);

        assert!(a_err <= ATOL, "A branch error too large: {a_err}");
        assert!(c_err <= ATOL, "C branch error too large: {c_err}");
        assert!(b_err <= ATOL, "B branch error too large: {b_err}");
    }
}