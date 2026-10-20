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
//! 对应 Python `test/test_block_ZXZ/test_step3.py`。

use crate::common::{allclose_up_to_global_phase, generate_diagonal_d};

use cqlib_core::compile::transform::decompose::block_zxz::matrix::{adjoint, block_diag};
use cqlib_core::compile::transform::decompose::block_zxz::ucrz::step3_ucrz_to_circuit;

const ATOL: f64 = 1e-10;

/// 用 k 个控制位生成对角幺酉 D，验证 UCRz 展开电路等价于 `diag(D, D†)`。
#[test]
fn step3_ucrz_reconstructs_diag() {
    for num_controls in 0..4usize {
        let d = generate_diagonal_d(num_controls, 7);
        let result = step3_ucrz_to_circuit(&d, ATOL).expect("step3 failed");

        let direct = block_diag(&d, &adjoint(&d));
        let decomposed = result.circuit.to_matrix(None).expect("to_matrix failed");

        let ok = allclose_up_to_global_phase(&direct, &decomposed, ATOL);
        assert!(ok, "UCRz circuit not equivalent to diag(D,D†) for k={num_controls}");
    }
}