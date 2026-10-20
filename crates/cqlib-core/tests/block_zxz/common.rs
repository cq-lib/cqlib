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
//! 测试共用工具：确定性随机幺酉生成、相位对齐、矩阵比较。
//!
//! 对应 Python 测试中的 `generate_random_unitary` / `_align_global_phase` / `_allclose`。
#![allow(dead_code)]

use ndarray::Array2;
use num_complex::Complex64;
use cqlib_core::compile::transform::decompose::block_zxz::matrix::{
    from_faer, mat_mul, max_abs_diff, to_faer, Cm,
};

/// 确定性 xorshift64* 伪随机数生成器。
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// 返回 `[0, 1)` 均匀分布。
    pub fn f64(&mut self) -> f64 {
        (self.next() >> 11) as f64 / ((1u64 << 53) as f64)
    }
    /// 返回标准正态分布（Box-Muller）。
    pub fn gaussian(&mut self) -> f64 {
        let u1 = self.f64().max(1e-300);
        let u2 = self.f64();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

/// 生成确定性的随机 `num_qubits` 比特幺酉矩阵。
///
/// 使用标准正态复数矩阵 + QR 分解并规约相位（与 Python 测试构造一致）。
pub fn generate_random_unitary(num_qubits: usize, seed: u64) -> Cm {
    assert!(num_qubits >= 1, "num_qubits must be at least 1");
    let dim = 1usize << num_qubits;
    let mut rng = Rng::new(seed);
    let mut gaussian = Array2::<Complex64>::zeros((dim, dim));
    for i in 0..dim {
        for j in 0..dim {
            gaussian[[i, j]] = Complex64::new(rng.gaussian(), rng.gaussian());
        }
    }

    let fm = to_faer(&gaussian);
    let qr = fm.qr();
    let q = from_faer(&qr.compute_Q().as_ref());
    let r = from_faer(&qr.R());

    // 相位规约：phases = r_diag / |r_diag|，U = Q @ diag(conj(phases))
    let mut phases = vec![Complex64::new(1.0, 0.0); dim];
    for i in 0..dim {
        let d = r[[i, i]];
        let mag = d.norm();
        if mag > 0.0 {
            phases[i] = d / mag;
        }
    }
    let phase_mat: Vec<Complex64> = phases.iter().map(|z| z.conj()).collect();
    let phase_diag = cqlib_core::compile::transform::decompose::block_zxz::matrix::diagonal_from_values(&phase_mat);
    mat_mul(&q, &phase_diag)
}

/// 生成合法的对角幺酉矩阵 `D`（随机相位）。
pub fn generate_diagonal_d(num_controls: usize, seed: u64) -> Cm {
    let dim = 1usize << num_controls;
    let mut rng = Rng::new(seed);
    let mut d = Array2::<Complex64>::zeros((dim, dim));
    for i in 0..dim {
        let phase = rng.f64() * std::f64::consts::TAU - std::f64::consts::PI;
        d[[i, i]] = Complex64::from_polar(1.0, phase);
    }
    d
}

/// 把 `candidate` 对齐到 `reference`（消除全局相位差）。
pub fn align_up_to(reference: &Cm, candidate: &Cm) -> Cm {
    let mut overlap = Complex64::new(0.0, 0.0);
    for i in 0..reference.nrows() {
        for j in 0..reference.ncols() {
            overlap += reference[[i, j]].conj() * candidate[[i, j]];
        }
    }
    if overlap.norm() > 0.0 {
        candidate.mapv(|z| z * Complex64::from_polar(1.0, -overlap.arg()))
    } else {
        candidate.clone()
    }
}

/// 判断两个矩阵在全局相位意义下相等。
pub fn allclose_up_to_global_phase(reference: &Cm, candidate: &Cm, atol: f64) -> bool {
    max_abs_diff(reference, &align_up_to(reference, candidate)) <= atol
}