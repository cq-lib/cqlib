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

//! # block_zxz
//!
//! 用 Rust 严格重写的 Block-ZXZ 量子酉矩阵分解算法（含论文 A.1/A.2 优化）。
//!
//! 对应 Python 项目 `decompose/block_ZXZ` 与论文
//! *Krol & Al-Ars, "Beyond Quantum Shannon: Circuit Construction for General
//! n-Qubit Gates Based on Block ZXZ-Decomposition" (arXiv:2403.13692)*：
//!
//! - [`step1`]：分块 + 极分解 + 求解 `A1/A2/B/C`
//! - [`step2`]：Schur 对角化 + 解复用（demux）；含 A.1 的 `B̃` 中路对
//! - [`ucrz`]：均匀控制 Rz 的 Gray 码展开；含 A.1 的去 CNOT 链变体
//! - [`vw`]：A.2 的两比特 2-CNOT + 对角余量分解（SMB Prop. III.3 构造）
//! - [`engine`]：递归树构建与电路组装 + A.1/A.2 优化 pass
//! - [`matrix`]：复数矩阵运算与数值分解工具
//!
//! 全优化（默认开启）时 CNOT 计数满足论文公式
//! `c_n ≤ 22/48·4^n − 3/2·2^n + 5/3`（n=3 → 19，n=4 → 95）。
//!
//! 数值底座为 `ndarray` 与 `faer`；单/双比特叶子合成复用本 crate
//! `decompose::unitary` 的数值合成接口（与 Python 项目调用 cqlib 一致）。

pub mod engine;
pub mod matrix;
pub mod step1;
pub mod step2;
pub mod ucrz;
pub mod vw;

pub use engine::{
    decompose_block_zxz, decompose_block_zxz_with_options, DecomposeOptions, DecomposeResult, Node,
};
pub use step1::block_zxz_step1;
pub use step2::block_zxz_step2;
pub use ucrz::step3_ucrz_to_circuit;