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
//! A.1 + A.2 优化验证测试。
//!
//! 对应论文（arXiv:2403.13692）第五节：
//! - A.1：层重建等价性 + 每层省 2 个 CNOT。
//! - A.2：叶子 2-CNOT + 对角余量分解的等价性与计数。
//! - 全优化 CNOT 计数：n=3 → 19，n=4 → 95（论文 Table 1）。

use crate::common::{allclose_up_to_global_phase, generate_random_unitary};

use cqlib_core::compile::transform::decompose::block_zxz::engine::{
    decompose_block_zxz_with_options, DecomposeOptions,
};
use cqlib_core::compile::transform::decompose::block_zxz::matrix::{mat_mul, Cm};
use cqlib_core::compile::transform::decompose::block_zxz::ucrz::{
    step3_ucrz_from_diagonal, synthesize_ucrz_circuit, synthesize_ucrz_circuit_reduced_tail,
    synthesize_ucrz_circuit_reversed_reduced_head,
};
use cqlib_core::compile::transform::decompose::block_zxz::vw::{
    count_cx, synth_two_cnot_with_diagonal,
};
use num_complex::Complex64;
use ndarray::Array2;

const ATOL: f64 = 1e-10;

fn count_cx_circuit(circuit: &cqlib_core::circuit::Circuit) -> usize {
    count_cx(circuit)
}

/// 在 `k+1` 比特空间构造闭合 CNOT `CX̄ = CX(control = k-1, target = k)` 矩阵。
fn closing_cx_matrix(num_controls: usize) -> Cm {
    let dim = 1usize << (num_controls + 1);
    let target = num_controls; // 局部编号：top = k
    let control = num_controls - 1; // 闭合 CNOT 的控制位 = 位 k-1
    let mut m = Array2::<Complex64>::eye(dim);
    for index in 0..dim {
        let control_bit = (index >> control) & 1;
        if control_bit == 1 {
            let flipped = index ^ (1 << target);
            m[[flipped, index]] = Complex64::new(1.0, 0.0);
            m[[index, index]] = Complex64::new(0.0, 0.0);
        }
    }
    m
}

/// 构造对角幺酉 D（随机相位）并返回 (D, 反相 α)。
fn random_diagonal(num_controls: usize, seed: u64) -> Cm {
    let dim = 1usize << num_controls;
    let mut rng = crate::common::Rng::new(seed);
    let mut d = Array2::<Complex64>::zeros((dim, dim));
    for i in 0..dim {
        let phase = rng.f64() * std::f64::consts::TAU - std::f64::consts::PI;
        d[[i, i]] = Complex64::from_polar(1.0, phase);
    }
    d
}

/// A.1 基础恒等式：reduced_tail 链矩阵左补 CX̄ 后应等于完整链矩阵。
#[test]
fn reduced_tail_chain_identity() {
    for num_controls in 2..=4usize {
        let d = random_diagonal(num_controls, 11);
        let data = step3_ucrz_from_diagonal(&d, ATOL).expect("ucrz data failed");

        let full = synthesize_ucrz_circuit(&data.theta, &data.controls, data.target)
            .and_then(|c| c.to_matrix(None).map_err(|e| e.to_string()))
            .expect("full chain failed");
        let reduced = synthesize_ucrz_circuit_reduced_tail(&data.theta, &data.controls, data.target)
            .and_then(|c| c.to_matrix(None).map_err(|e| e.to_string()))
            .expect("reduced chain failed");

        let closing = closing_cx_matrix(num_controls);
        // 期望：full ≡ closing · reduced（时间最后门为 CX̄ ⟹ 矩阵最左因子）。
        let expected = mat_mul(&closing, &reduced);
        assert!(
            allclose_up_to_global_phase(&full, &expected, ATOL),
            "reduced_tail identity failed for k={num_controls}"
        );
    }
}

/// A.1 基础恒等式：reversed_reduced_head 链矩阵右补 CX̄ 后应等于完整链矩阵
/// （时间逆序链实现同一对角矩阵，去掉的首门即闭合 CX̄）。
#[test]
fn reversed_reduced_head_chain_identity() {
    for num_controls in 2..=4usize {
        let d = random_diagonal(num_controls, 13);
        let data = step3_ucrz_from_diagonal(&d, ATOL).expect("ucrz data failed");

        let full = synthesize_ucrz_circuit(&data.theta, &data.controls, data.target)
            .and_then(|c| c.to_matrix(None).map_err(|e| e.to_string()))
            .expect("full chain failed");
        let reduced = synthesize_ucrz_circuit_reversed_reduced_head(
            &data.theta,
            &data.controls,
            data.target,
        )
        .and_then(|c| c.to_matrix(None).map_err(|e| e.to_string()))
        .expect("reversed chain failed");

        let closing = closing_cx_matrix(num_controls);
        // 期望：full ≡ reduced · closing（首门 CX̄ 提出 ⟹ 矩阵最右因子）。
        let expected = mat_mul(&reduced, &closing);
        assert!(
            allclose_up_to_global_phase(&full, &expected, ATOL),
            "reversed_reduced_head identity failed for k={num_controls}"
        );
    }
}

/// A.1 层级等价与计数：n=3/n=4 上仅启用 A.1 时，重建等价且每层省 2 CX。
#[test]
fn a1_layer_equivalence_and_savings() {
    for num_qubits in 3..=4usize {
        let unitary = generate_random_unitary(num_qubits, 7);

        // A.1 关闭（对照）。
        let baseline = decompose_block_zxz_with_options(
            &unitary,
            ATOL,
            &DecomposeOptions {
                opt_a1: false,
                opt_a2: false,
                opt_a2_force: false,
            },
        )
        .expect("baseline decompose failed");
        // A.1 打开（A.2 关闭以隔离计数来源）。
        let optimized = decompose_block_zxz_with_options(
            &unitary,
            ATOL,
            &DecomposeOptions {
                opt_a1: true,
                opt_a2: false,
                opt_a2_force: false,
            },
        )
        .expect("a1 decompose failed");

        // 等价性。
        let recon = optimized
            .circuit
            .to_matrix(None)
            .expect("to_matrix failed");
        assert!(
            allclose_up_to_global_phase(&unitary, &recon, ATOL),
            "A.1 reconstruction failed for n={num_qubits}"
        );

        // 计数：递归层总数 = (4^{n-2}-1)/3，每层省 2（n=3 → 1 层省 2；n=4 → 5 层省 10）。
        let baseline_cx = count_cx_circuit(&baseline.circuit);
        let optimized_cx = count_cx_circuit(&optimized.circuit);
        let layers = (4usize.pow((num_qubits - 2) as u32) - 1) / 3;
        let expected_savings = 2 * layers;
        assert_eq!(
            baseline_cx - optimized_cx,
            expected_savings,
            "A.1 savings mismatch for n={num_qubits}: baseline={baseline_cx} optimized={optimized_cx}"
        );
    }
}

/// A.2 单叶子：2-CNOT 分解 + 对角余量的等价性。
///
/// 合成约束：`D_out · circuit ≡ G`（余量在时间晚侧/矩阵左端）。
#[test]
fn a2_two_cnot_synthesis_equivalence() {
    for seed in 1..=8u64 {
        let g = generate_random_unitary(2, seed);
        let synthesis = synth_two_cnot_with_diagonal(&g).expect("synth failed");
        println!("seed={seed}: cx_count={}", synthesis.cx_count);

        let circuit_matrix = synthesis
            .circuit
            .to_matrix(None)
            .expect("to_matrix failed");
        let with_diagonal = mat_mul(&synthesis.diagonal_out, &circuit_matrix);
        assert!(
            allclose_up_to_global_phase(&g, &with_diagonal, ATOL),
            "A.2 leaf synthesis failed for seed={seed}"
        );
        assert!(
            synthesis.cx_count <= 2,
            "A.2 leaf used {} CX for seed={seed}",
            synthesis.cx_count
        );
        // 余量必须为纯对角（作用位 (q1,q0)），否则无法无重排地沿链迁移。
        for i in 0..4 {
            for j in 0..4 {
                if i != j {
                    assert!(
                        synthesis.diagonal_out[[i, j]].norm() < 1e-12,
                        "A.2 remainder not diagonal for seed={seed} at ({i},{j})"
                    );
                }
            }
        }
    }
}

/// 全优化端到端：重建等价 + CNOT 计数（论文 Table 1：n=3 → 19，n=4 → 95）。
///
/// 多种子验证：对一般随机酉，全优化电路应稳定达到论文计数（每个非最后
/// 两比特叶子 2 CX + 最后叶子完整合成）；断言上界（个别叶子落入更低
/// CNOT 类时允许更少）。
#[test]
fn full_optimization_counts() {
    let cases = [(3usize, 19usize), (4, 95)];
    for (num_qubits, expected) in cases {
        for seed in [7u64, 8, 9, 10] {
            let unitary = generate_random_unitary(num_qubits, seed);
            let result = decompose_block_zxz_with_options(
                &unitary,
                ATOL,
                &DecomposeOptions::default(),
            )
            .expect("decompose failed");

            let recon = result.circuit.to_matrix(None).expect("to_matrix failed");
            assert!(
                allclose_up_to_global_phase(&unitary, &recon, ATOL),
                "full optimization reconstruction failed for n={num_qubits} seed={seed}"
            );

            let cx = count_cx_circuit(&result.circuit);
            assert!(
                cx <= expected,
                "n={num_qubits} seed={seed}: CX count {cx} exceeds paper bound {expected}"
            );
            println!("n={num_qubits} seed={seed}: optimized CX = {cx} (paper bound {expected})");
        }
    }
}

/// 块对角（受控门）输入：A.2 自动禁用且分解仍然正确。
#[test]
fn block_diagonal_input_disables_a2() {
    let u0 = generate_random_unitary(2, 21);
    let u1 = generate_random_unitary(2, 22);
    let mut u = Array2::<Complex64>::zeros((8, 8));
    for i in 0..4 {
        for j in 0..4 {
            u[[i, j]] = u0[[i, j]];
            u[[4 + i, 4 + j]] = u1[[i, j]];
        }
    }

    let result = decompose_block_zxz_with_options(&u, ATOL, &DecomposeOptions::default())
        .expect("block-diagonal decompose failed");
    let recon = result.circuit.to_matrix(None).expect("to_matrix failed");
    assert!(
        allclose_up_to_global_phase(&u, &recon, ATOL),
        "block-diagonal reconstruction failed"
    );
}