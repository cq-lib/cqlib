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
//! A.2 优化：两比特门的 2-CNOT + 对角余量分解（论文 5.1 节）。
//!
//! 论文（arXiv:2403.13692 §5.1）引用 Shende-Markov-Bullock 的「up to
//! diagonal」技法（PRL 版 quant-ph/0406176；判据为 quant-ph/0308045
//! Prop. III.3）：任意两比特幺正 `v` 存在（计算基）对角 `d`，使 `d·v`
//! 与 `v·d` 均可用 2 个 CNOT 实现；对角余量沿电路迁移并在下一个两比特
//! 门处合并，直到最后一个门完整合成。
//!
//! 数学判据（SMB Prop. III.3）：`u ∈ SU(4)` 可用 2 CNOT + 单比特门实现
//! ⟺ `tr[γ(u)]` 为实数，其中 `γ(u) = u·S·uᵀ·S`，`S = σy⊗σy`。
//!
//! 显式构造（本模块推导）：对 4×4 幺正 `U`，令 `M = U·S·Uᵀ`（复对称），
//! `A = M[1][2]`、`B = M[0][3]`（由对称性配对项相等），
//! `α = arg A`、`β = arg B`、`δ = arg det U`、`c₀ = α + β − δ`，取
//! ```text
//! u* = atan2(|B|·sin c₀, |A| + |B|·cos c₀),   v* = c₀ − u*,
//! D  = diag(1, 1, e^{i(u*−α)}, e^{i(v*−β)}),
//! ```
//! 则 `T = D·U` 的 SU(4) 归一满足 `tr[γ(T̃)] = 2(|A| sin u* − |B| sin v*)`
//! 为实数（推导要点：`tr[γ(DU)] = 2(d₁d₂·m₁₂ − d₀d₃·m₀₃)`，而全局归一
//! 因子 `det(DU)^{-1/2}` 的辐角恰好把 `δ` 并入两个配平相位），故 `T`
//! 落在 2-CNOT 类。
//!
//! 合成契约（见 `engine::run_a2_chain`）：`D_out · circuit ≡ target`。
//! circuit 实现的是 `T = D·target`（cqlib 返回全局相位 `φ_T`），余量取
//! `D_out = e^{iφ_T}·D⁻¹`：纯对角相位矩阵，作用于位 `(q1, q0)`。
//!
//! 余量传递的合法性：Block-ZXZ/A.1 电路的非叶门只有 `Rz(层 top)`、
//! `CX(位 < top → 层 top)` 与 `H(层 top)`，其中 X/H 只作用于 `top ≥ q2`；
//! 对角余量的相位仅依赖 `(q1,q0)` 的取值（与 CX 控制投影、Rz 相位均
//! 可交换），且作用位与 `top` 不相交，因此余量与全部非叶门严格可交换，
//! 无需重排即可迁移到下一个两比特叶子的输入（`U_next · D_out`），最终
//! 被时间上最后一个叶子的完整合成（0~3 CX 自适应）吸收。

use crate::circuit::{Circuit, Parameter, Qubit};
use crate::compile::transform::decompose::unitary::{
    synthesize_numeric_2q_unitary, TwoQubitUnitaryDecomposeBasis,
};
use num_complex::Complex64;
use ndarray::Array2;

use super::matrix::{adjoint, eye, mat_mul, to_faer, Cm};

/// `S = σy⊗σy`（基序 `(q1,q0)`，张量第一因子为高位 q1）。
///
/// 由 `σy|0⟩ = i|1⟩`、`σy|1⟩ = −i|0⟩`：
/// `S|00⟩ = −|11⟩`、`S|01⟩ = |10⟩`、`S|10⟩ = |01⟩`、`S|11⟩ = −|00⟩`，
/// 故 `S = [[0,0,0,−1],[0,0,1,0],[0,1,0,0],[−1,0,0,0]]`（实、对称、对合、酉）。
fn syy_matrix() -> Cm {
    let mut s = Array2::<Complex64>::zeros((4, 4));
    let one = Complex64::new(1.0, 0.0);
    s[[0, 3]] = -one;
    s[[1, 2]] = one;
    s[[2, 1]] = one;
    s[[3, 0]] = -one;
    s
}

/// 构造左乘对角相位 `D`，使 `T = D·U` 的 SU(4) 归一落入 2-CNOT 类
/// （SMB Prop. III.3 的显式解，推导见模块文档）。
pub fn two_cnot_phase_diagonal(u: &Cm) -> Cm {
    let s = syy_matrix();
    let u_t = u.t().to_owned(); // 纯转置（无共轭）
    let m = mat_mul(&mat_mul(u, &s), &u_t); // M = U·S·Uᵀ，复对称

    let a = m[[1, 2]]; // 由对称性 m[1][2] = m[2][1]
    let b = m[[0, 3]]; // 由对称性 m[0][3] = m[3][0]
    let alpha = a.arg();
    let beta = b.arg();
    let r1 = a.norm();
    let r0 = b.norm();

    // δ = arg det U。
    let fm = to_faer(u);
    let det = fm.determinant();
    let delta = Complex64::new(det.re, det.im).arg();

    // 配平方程（恒有解）：(r1 + r0·cos c₀)·sin u* = r0·sin c₀·cos u*。
    let c0 = alpha + beta - delta;
    let u_star = (r0 * c0.sin()).atan2(r1 + r0 * c0.cos());
    let v_star = c0 - u_star;

    // 约束 d1·d2 = e^{i(u*−α+…)} 的最简分配：d0 = d1 = 1。
    let phi_p = u_star - alpha;
    let phi_q = v_star - beta;
    let mut d = eye(4);
    d[[2, 2]] = Complex64::from_polar(1.0, phi_p);
    d[[3, 3]] = Complex64::from_polar(1.0, phi_q);
    d
}

/// A.2 中单个两比特叶子的优化合成结果。
pub struct TwoCnotSynthesis {
    /// 2-CNOT（或更少）电路，与 `diagonal_out` 联合实现 `target`。
    pub circuit: Circuit,
    /// 对角余量 `D_out = e^{iφ_T}·D⁻¹`（作用于位 `(q1,q0)`，时间晚侧/
    /// 矩阵左端），满足 `D_out · circuit ≡ target`（全局相位意义）。
    pub diagonal_out: Cm,
    /// 本次合成实际使用的 CX 数。
    pub cx_count: usize,
}

/// 把 `target` 分解为 2-CNOT 电路 + 纯对角余量。
///
/// 合成约束：`D_out · circuit ≡ target`（全局相位意义）。
/// 余量为单位模对角矩阵；若 `target` 本身已在 2-CNOT 类内（或数值上
/// 构造失败时兜底），则返回完整合成且余量为单位阵。
pub fn synth_two_cnot_with_diagonal(target: &Cm) -> Result<TwoCnotSynthesis, String> {
    // 基线合成（3-CNOT 或更少），保证任何路径下都有正确结果。
    let baseline = synthesize_for_matrix(target)?;
    let baseline_cx = count_cx(&baseline.0);
    if baseline_cx <= 2 {
        // 目标本身已在 2-CNOT 类内（含局部、受控等特殊结构）。
        return Ok(TwoCnotSynthesis {
            circuit: baseline.0,
            diagonal_out: eye(4),
            cx_count: baseline_cx,
        });
    }

    // SMB 构造：T = D·target 落入 2-CNOT 类，余量 D_out = e^{iφ_T}·D⁻¹ 纯对角。
    let d = two_cnot_phase_diagonal(target);
    let t = mat_mul(&d, target);
    let (circuit, global_phase) = synthesize_for_matrix(&t)?;
    let cx = count_cx(&circuit);
    if cx <= 2 {
        let phase = Complex64::from_polar(1.0, global_phase);
        let mut d_out = adjoint(&d); // D 对角幺正 ⟹ D⁻¹ = D†，仍为对角。
        d_out.mapv_inplace(|v| v * phase);
        // D_out · circuit ≡ e^{iφ_T}·D⁻¹·T(数值) ≡ D⁻¹·D·target ≡ target。
        return Ok(TwoCnotSynthesis {
            circuit,
            diagonal_out: d_out,
            cx_count: cx,
        });
    }

    // 兜底：直接 3-CNOT 合成，余量为单位阵（A.2 链在此叶子处中断，
    // 仍保证数值正确；闭式构造下正常不应触发）。
    Ok(TwoCnotSynthesis {
        circuit: baseline.0,
        diagonal_out: eye(4),
        cx_count: baseline_cx,
    })
}

/// 统计电路（含嵌套 CompositeGate）中 `CX` 门数量。
pub fn count_cx(circuit: &Circuit) -> usize {
    use crate::circuit::gate::Instruction;
    let mut total = 0;
    for op in circuit.operations() {
        match &op.instruction {
            Instruction::Standard(gate) => {
                if gate.name() == "CX" {
                    total += 1;
                }
            }
            Instruction::CircuitGate(cg) => {
                total += count_cx(cg.circuit().as_ref().circuit());
            }
            _ => {}
        }
    }
    total
}

/// 用 cqlib 的 Cx 基合成 4×4 幺正（qubits = [1, 0]，与整体引擎一致），
/// 返回 `(circuit, global_phase)`，满足 `e^{iφ}·circuit ≡ matrix`。
fn synthesize_for_matrix(matrix: &Cm) -> Result<(Circuit, f64), String> {
    let synthesis = synthesize_numeric_2q_unitary(
        matrix,
        [Qubit::new(1), Qubit::new(0)],
        TwoQubitUnitaryDecomposeBasis::Cx,
    )
    .map_err(|e| e.to_string())?;
    let mut circuit = Circuit::from_operations(
        vec![Qubit::new(0), Qubit::new(1)],
        synthesis.operations,
        None,
        None,
    )
    .map_err(|e| e.to_string())?;
    circuit.set_global_phase(Parameter::from(synthesis.global_phase));
    Ok((circuit, synthesis.global_phase))
}

/// 最后一个叶子（或禁用 A.2 的叶子）的完整合成（0~3 CX 自适应）。
pub fn synth_leaf_full(target: &Cm) -> Result<Circuit, String> {
    let (circuit, _) = synthesize_for_matrix(target)?;
    Ok(circuit)
}
