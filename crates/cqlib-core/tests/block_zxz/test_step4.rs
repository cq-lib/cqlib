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
//! 对应 Python `test/test_block_ZXZ/test_step4.py`。

use crate::common::{allclose_up_to_global_phase, generate_random_unitary};

use cqlib_core::compile::transform::decompose::block_zxz::engine::decompose_block_zxz;

const ATOL: f64 = 1e-10;

/// 完整重建：`decompose_block_zxz` 组装电路应与输入幺酉矩阵在全局相位意义下一致。
#[test]
fn step4_matrix_reconstruction() {
    for num_qubits in 3..=4usize {
        let unitary = generate_random_unitary(num_qubits, 7);
        let result = decompose_block_zxz(&unitary, ATOL).expect("decompose failed");
        let reconstructed = result
            .circuit
            .to_matrix(None)
            .expect("to_matrix failed");

        let ok = allclose_up_to_global_phase(&unitary, &reconstructed, ATOL);
        assert!(
            ok,
            "full reconstruction failed for {num_qubits} qubits (error exceeded {ATOL})"
        );
    }
}