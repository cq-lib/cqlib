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

//! # Block-ZXZ decomposition
//!
//! Rust implementation of the Block-ZXZ quantum unitary decomposition
//! (including the paper's A.1/A.2 optimizations).
//!
//! This mirrors the Python project `decompose/block_ZXZ` and the paper
//! *Krol & Al-Ars, "Beyond Quantum Shannon: Circuit Construction for
//! General n-Qubit Gates Based on Block ZXZ-Decomposition"
//! (arXiv:2403.13692)*:
//!
//! - [`step1`]: block splitting + polar decomposition + solving `A1/A2/B/C`
//! - [`step2`]: Schur-style diagonalization + demultiplexing; includes the
//!   A.1-optimized `B-tilde` middle pair
//! - [`ucrz`]: Gray-code expansion of uniformly controlled `Rz` gates;
//!   includes the A.1 CNOT-reduced chain variants
//! - [`vw`]: A.2 two-qubit 2-CNOT + diagonal-remainder synthesis
//!   (Vatan-Williams style construction)
//! - [`engine`]: recursive tree construction, circuit assembly, and the
//!   A.1/A.2 optimization passes
//! - [`matrix`]: complex matrix operations and numeric decomposition helpers
//!
//! With both optimizations enabled (the default) the CNOT count satisfies
//! the paper bound `c_n <= 22/48 * 4^n - 3/2 * 2^n + 5/3`
//! (n = 3 -> 19, n = 4 -> 95).
//!
//! Numerically degenerate inputs may exceed that count by up to two CNOTs:
//! when folding the accumulated A.2 diagonal remainder into a leaf's
//! synthesis target is not numerically feasible (see
//! [`engine::decompose_block_zxz_with_options`]), the remainder is emitted
//! as an explicit diagonal gate instead of aborting the decomposition.
//! Correctness is preserved in both paths.
//!
//! Leaf (1q/2q) synthesis reuses the crate's [`super::unitary`] module, the
//! same primitives the Python project reaches through `cqlib`.

pub mod engine;
pub mod matrix;
pub mod step1;
pub mod step2;
pub mod ucrz;
pub mod vw;

pub use engine::{
    DecomposeOptions, DecomposeResult, Node, decompose_block_zxz, decompose_block_zxz_with_options,
};
pub use step1::block_zxz_step1;
pub use step2::block_zxz_step2;
pub use ucrz::step3_ucrz_to_circuit;
