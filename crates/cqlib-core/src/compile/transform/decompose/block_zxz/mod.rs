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

//! Recursive Block-ZXZ synthesis of numeric unitary matrices.
//! [Krol & Al-Ars, §§4–5](https://arxiv.org/html/2403.13692v2#S4).

mod factors;
mod leaf;
mod synthesis;
mod ucrz;

pub use synthesis::{
    BlockZxzDecomposeOptions, decompose_block_zxz, decompose_block_zxz_with_options,
};
