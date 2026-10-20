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
//! 对照 `paper_md/Block-ZXZ.md` 的优化方法检查电路结构。
//!
//! 论文写了三个优化相关构造：
//! - A.1（第五步）：UCRz 边侧 CNOT 经 H 变为 CZ，吸收进 B̃ = diag(W_A V_C,
//!   Z·W_A·B·V_C·Z)，每层递归省 2 个 CNOT；
//! - A.2（第六步）：2q 叶子用「2-CNOT + 对角余量」分解并对角滚动；
//! - 基线要求：n=2 叶子用「最优 3-CNOT 构造」。
//!
//! 本测试统计分解电路的实际门构成，量化确认 A.1/A.2 已实现。

use crate::common::generate_random_unitary;

use cqlib_core::circuit::Circuit;
use cqlib_core::compile::transform::decompose::block_zxz::engine::{
    decompose_block_zxz_with_options, DecomposeOptions,
};
use cqlib_core::compile::transform::decompose::block_zxz::vw::count_cx;
use std::collections::BTreeMap;

/// 递归展开（含 CircuitGate 嵌套）统计各标准门出现次数。
fn count_gates(circuit: &Circuit, counts: &mut BTreeMap<String, usize>) {
    for op in circuit.operations() {
        match &op.instruction {
            cqlib_core::circuit::gate::Instruction::Standard(gate) => {
                *counts.entry(gate.name().to_string()).or_insert(0) += 1;
            }
            cqlib_core::circuit::gate::Instruction::CircuitGate(cg) => {
                count_gates(cg.circuit().as_ref().circuit(), counts);
            }
            _ => {}
        }
    }
}

fn describe(num_qubits: usize, options: &DecomposeOptions) -> usize {
    let unitary = generate_random_unitary(num_qubits, 7);
    let result = decompose_block_zxz_with_options(&unitary, 1e-10, options).expect("decompose failed");
    let mut counts = BTreeMap::new();
    count_gates(&result.circuit, &mut counts);
    let cx = count_cx(&result.circuit);

    println!("--- n = {num_qubits} (a1={}, a2={}) 门构成 ---", options.opt_a1, options.opt_a2);
    let mut total = 0;
    for (name, n) in &counts {
        println!("  {name:>6} x {n}");
        total += n;
    }
    println!("  CX = {cx}, total ops = {total}");
    cx
}

#[test]
fn report_gate_composition() {
    // 未优化基线：n=3 → 层 12 + 叶 12 = 24（论文 4.2 节递推）。
    let baseline = describe(
        3,
        &DecomposeOptions {
            opt_a1: false,
            opt_a2: false,
            opt_a2_force: false,
        },
    );
    assert_eq!(baseline, 24, "unoptimized baseline should be 24 CX for n=3");

    // 仅 A.1：每层省 2 → 24 - 2 = 22。
    let a1_only = describe(
        3,
        &DecomposeOptions {
            opt_a1: true,
            opt_a2: false,
            opt_a2_force: false,
        },
    );
    assert_eq!(a1_only, 22, "A.1-only should be 22 CX for n=3");

    // 全优化：A.1 省 2 + A.2 省 3 → 19（论文 Table 1）。
    let full = describe(3, &DecomposeOptions::default());
    assert_eq!(full, 19, "fully optimized should be 19 CX for n=3");

    // n=4 全优化 → 95（论文 Table 1）。
    let n4 = describe(4, &DecomposeOptions::default());
    assert_eq!(n4, 95, "fully optimized should be 95 CX for n=4");
}