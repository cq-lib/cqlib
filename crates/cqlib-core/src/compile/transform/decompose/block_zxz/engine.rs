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
//! Block-ZXZ 主流程：递归树构建与电路组装（第四步）+ A.1/A.2 优化。
//!
//! 逐函数对应 Python `decompose/block_ZXZ/main.py`，并实现论文第五节的两项优化：
//!
//! - **A.1**（`opt_a1`，论文 5.2 / Eq. 11）：中路块对改为
//!   `B̃ = diag(W_A·V_C, (Z⊗I)·W_A·B·V_C·(Z⊗I))`，同时 C 支 UCRz 去尾部
//!   闭合 CNOT、A 支 UCRz 用时间逆序链去首部闭合 CNOT。由恒等式
//!   `H·CX̄ = CZ·H` 与 `CX̄·H = H·CZ`，两个被去除的 CNOT 恰好被吸收进
//!   `B̃` 的 demux，每层节省 2 个 CNOT。
//! - **A.2**（`opt_a2`，论文 5.1 / Shende-Markov-Bullock「up to diagonal」）：
//!   除时间上最后一个两比特叶子外，每个叶子分解为 2-CNOT 电路 + 纯对角
//!   余量（SMB Prop. III.3 构造，见 `vw` 模块文档），余量沿电路时间序传
//!   递并与后续叶子合成目标合并（对角门与所有非叶门严格可交换，可无需
//!   重排地迁移）。
//!
//! 叶子（1q/2q）合成复用 cqlib-core，与 Python 项目调用 cqlib 保持一致。

use crate::circuit::{Circuit, Parameter, Qubit};
use crate::compile::transform::decompose::unitary::synthesize_numeric_1q_unitary;
use num_complex::Complex64;
use ndarray::Array2;

use super::matrix::Cm;
use super::step1::assert_square_unitary;
use super::step1::block_zxz_step1;
use super::step2::{block_zxz_step2, demux_middle_branch_a1, demux_pair};
use super::ucrz::{
    step3_ucrz_from_diagonal, synthesize_ucrz_circuit, synthesize_ucrz_circuit_reduced_tail,
    synthesize_ucrz_circuit_reversed_reduced_head,
};
use super::vw::{synth_leaf_full, synth_two_cnot_with_diagonal};

/// 分解选项（对应论文第五节的优化开关）。
#[derive(Debug, Clone, Copy)]
pub struct DecomposeOptions {
    /// A.1：每层把两个 UCRz 的 CNOT 合并进中路多路复用器（省 2 CNOT/层）。
    pub opt_a1: bool,
    /// A.2：两比特叶子 2-CNOT 化 + 对角余量滚动（省 4^{n-2}-1 CNOT）。
    pub opt_a2: bool,
    /// 强制在块对角（受控门）输入上也应用 A.2。
    /// 论文 5.1 指出受控门输入会因滚动破坏结构而禁用 A.2。
    pub opt_a2_force: bool,
}

impl Default for DecomposeOptions {
    fn default() -> Self {
        DecomposeOptions {
            opt_a1: true,
            opt_a2: true,
            opt_a2_force: false,
        }
    }
}

impl DecomposeOptions {
    /// 全部优化关闭（与 Python 原实现行为一致）。
    pub fn none() -> Self {
        DecomposeOptions {
            opt_a1: false,
            opt_a2: false,
            opt_a2_force: false,
        }
    }
}

/// 返回 `nrows` 对应的量子比特数 `log2(nrows)`。
fn num_qubits_of(unitary: &Array2<Complex64>) -> usize {
    (unitary.nrows() as f64).log2().round() as usize
}

/// 递归树的节点。
#[derive(Debug)]
pub enum Node {
    /// 1/2 比特叶子。
    Base(BaseNode),
    /// 递归层节点。
    Recursive(RecursiveNode),
}

/// 叶子节点（构建阶段缓存矩阵，A.2 合成延迟到链式传递完成后写入电路）。
#[derive(Debug)]
pub struct BaseNode {
    pub num_qubits: usize,
    /// 叶子输入矩阵（A.2 传递 pass 使用）。
    pub matrix: Cm,
    /// 合成电路（1q 在树构建时立即合成；2q 由 `apply_leaf_synthesis` 填充）。
    pub circuit: Circuit,
    /// 合成方法标记。
    pub method: &'static str,
}

/// 递归层节点。
#[derive(Debug)]
pub struct RecursiveNode {
    pub num_qubits: usize,
    pub input_unitary: Cm,
    pub layer: Layer,
    pub children: Vec<(String, Node)>,
}

/// 单层分解结果。
#[derive(Debug)]
pub struct Layer {
    /// 三个 UCRz 电路（按 `c/b/a` 命名）。
    pub step3: LayerStep3,
    /// 四个继续递归的子幺正（按 `va/vb/wb/wc` 命名）。
    pub subunitaries: Vec<(String, Cm)>,
    /// 组装顺序。
    pub assembly: Vec<AssemblyItem>,
    /// 本层是否应用了 A.1。
    pub opt_a1: bool,
}

/// 三个 UCRz 电路容器。
#[derive(Debug)]
pub struct LayerStep3 {
    pub c: Circuit,
    pub b: Circuit,
    pub a: Circuit,
}

/// 组装条目。
#[derive(Debug, Clone)]
pub enum AssemblyItem {
    /// 递归子电路（名称为 `wc/wb/vb/va`）。
    Child(&'static str),
    /// UCRz 电路（名称为 `c/b/a`）。
    Ucrz(&'static str),
    /// 顶层 Hadamard。
    H,
}

/// 返回固定的一层组装顺序（对应 Python `get_layer_assembly_plan`）。
pub fn get_layer_assembly_plan() -> Vec<AssemblyItem> {
    vec![
        AssemblyItem::Child("wc"),
        AssemblyItem::Ucrz("c"),
        AssemblyItem::H,
        AssemblyItem::Child("wb"),
        AssemblyItem::Ucrz("b"),
        AssemblyItem::Child("vb"),
        AssemblyItem::H,
        AssemblyItem::Ucrz("a"),
        AssemblyItem::Child("va"),
    ]
}

/// 是否为块对角矩阵（受控门结构，A.2 的禁用条件）。
pub fn is_block_diagonal(unitary: &Cm, atol: f64) -> bool {
    let n = unitary.nrows();
    let half = n / 2;
    for i in 0..n {
        for j in 0..n {
            let in_offdiag = (i < half) != (j < half);
            if in_offdiag && unitary[[i, j]].norm() > atol {
                return false;
            }
        }
    }
    true
}

/// 收集四个继续递归的子幺正（对应 Python `collect_recursive_subunitaries`）。
pub fn collect_recursive_subunitaries(
    step2: &super::step2::Step2Result,
    middle_pair: &(Cm, Cm),
) -> Vec<(String, Cm)> {
    let _ = middle_pair;
    vec![
        ("va".to_string(), step2.a_demux.0.clone()),
        ("vb".to_string(), step2.middle_demux.0.clone()),
        ("wb".to_string(), step2.middle_demux.2.clone()),
        ("wc".to_string(), step2.c_demux.2.clone()),
    ]
}

/// 是否为 1/2 比特的递归叶子（对应 Python `is_base_case_unitary`）。
pub fn is_base_case_unitary(unitary: &Cm, atol: f64) -> Result<bool, String> {
    assert_square_unitary(unitary, atol)?;
    Ok(unitary.nrows() == 2 || unitary.nrows() == 4)
}

/// 1 比特叶子合成（ZYZ，对应 Python 的 `synthesize_numeric_1q_unitary` 路径）。
fn synth_1q(unitary: &Cm) -> Result<(Circuit, &'static str), String> {
    let decomposition = synthesize_numeric_1q_unitary(unitary).map_err(|e| e.to_string())?;
    let mut circuit = Circuit::new(1);
    circuit
        .u(
            Qubit::new(0),
            Parameter::from(decomposition.theta),
            Parameter::from(decomposition.phi),
            Parameter::from(decomposition.lambda),
        )
        .map_err(|e| e.to_string())?;
    circuit.set_global_phase(Parameter::from(decomposition.global_phase));
    Ok((circuit, "1q"))
}

/// 构建叶子节点（延迟合成：先记录矩阵，电路由 `apply_leaf_synthesis` 填充）。
///
/// `pending_diagonal` 用于记录该叶子尚未合并的 A.2 对角余量输入。
fn build_base_case_node(unitary: &Cm, options: &DecomposeOptions) -> Result<BaseNode, String> {
    let num_qubits = num_qubits_of(unitary);
    match num_qubits {
        1 => {
            let (circuit, method) = synth_1q(unitary)?;
            Ok(BaseNode {
                num_qubits,
                matrix: unitary.clone(),
                circuit,
                method,
            })
        }
        2 => {
            // 延迟合成：缓存矩阵，电路由 A.2 pass 按执行序填充；
            // A.2 关闭时立即完整合成。
            if !options.opt_a2 {
                let circuit = synth_leaf_full(unitary)?;
                Ok(BaseNode {
                    num_qubits,
                    matrix: unitary.clone(),
                    circuit,
                    method: "2q",
                })
            } else {
                Ok(BaseNode {
                    num_qubits,
                    matrix: unitary.clone(),
                    circuit: Circuit::new(2),
                    method: "2q-pending",
                })
            }
        }
        _ => Err("Base case circuit synthesis only supports 1 or 2 qubits.".into()),
    }
}

/// 构建单层（对应 Python `build_block_zxz_layer`，带 A.1 开关）。
pub fn build_block_zxz_layer(
    unitary: &Cm,
    atol: f64,
    options: &DecomposeOptions,
) -> Result<Layer, String> {
    assert_square_unitary(unitary, atol)?;

    let (a1, a2, b, c) = block_zxz_step1(unitary, atol)?;
    let step2 = block_zxz_step2(&a1, &a2, &b, &c, atol)?;
    let a_demux = &step2.a_demux;
    let c_demux = &step2.c_demux;

    let (c_ucrz, a_ucrz, use_a1) = if options.opt_a1 {
        // A.1：C 支去尾闭合 CNOT，A 支时间逆序去首闭合 CNOT，均省 1 个 CNOT。
        let c_data = step3_ucrz_from_diagonal(&c_demux.1, atol)?;
        let a_data = step3_ucrz_from_diagonal(&a_demux.1, atol)?;
        let c_circuit = synthesize_ucrz_circuit_reduced_tail(
            &c_data.theta,
            &c_data.controls,
            c_data.target,
        )?;
        let a_circuit = synthesize_ucrz_circuit_reversed_reduced_head(
            &a_data.theta,
            &a_data.controls,
            a_data.target,
        )?;
        (c_circuit, a_circuit, true)
    } else {
        let c_data = step3_ucrz_from_diagonal(&c_demux.1, atol)?;
        let a_data = step3_ucrz_from_diagonal(&a_demux.1, atol)?;
        let c_circuit = synthesize_ucrz_circuit(&c_data.theta, &c_data.controls, c_data.target)?;
        let a_circuit = synthesize_ucrz_circuit(&a_data.theta, &a_data.controls, a_data.target)?;
        (c_circuit, a_circuit, false)
    };

    // 中路 demux：A.1 时使用 B̃ 对（Eq. 11），否则使用常规对。
    let middle_demux = if use_a1 {
        demux_middle_branch_a1(&a_demux.2, &b, &c_demux.0, atol)?
    } else {
        demux_pair(
            &super::step2::build_middle_pair(&a_demux.2, &b, &c_demux.0).0,
            &super::step2::build_middle_pair(&a_demux.2, &b, &c_demux.0).1,
            atol,
        )?
    };
    let b_ucrz = {
        let data = step3_ucrz_from_diagonal(&middle_demux.1, atol)?;
        synthesize_ucrz_circuit(&data.theta, &data.controls, data.target)?
    };

    let mut subunitaries = vec![
        ("va".to_string(), a_demux.0.clone()),
        ("vb".to_string(), middle_demux.0.clone()),
        ("wb".to_string(), middle_demux.2.clone()),
        ("wc".to_string(), c_demux.2.clone()),
    ];
    subunitaries.shrink_to_fit();

    Ok(Layer {
        step3: LayerStep3 {
            c: c_ucrz,
            b: b_ucrz,
            a: a_ucrz,
        },
        subunitaries,
        assembly: get_layer_assembly_plan(),
        opt_a1: use_a1,
    })
}

/// 构建完整递归树（对应 Python `build_block_zxz_tree`）。
pub fn build_block_zxz_tree(
    unitary: &Cm,
    atol: f64,
    options: &DecomposeOptions,
) -> Result<Node, String> {
    assert_square_unitary(unitary, atol)?;

    if is_base_case_unitary(unitary, atol)? {
        return Ok(Node::Base(build_base_case_node(unitary, options)?));
    }

    let layer = build_block_zxz_layer(unitary, atol, options)?;
    let mut children = Vec::new();
    for (name, sub) in &layer.subunitaries {
        children.push((name.clone(), build_block_zxz_tree(sub, atol, options)?));
    }
    Ok(Node::Recursive(RecursiveNode {
        num_qubits: num_qubits_of(unitary),
        input_unitary: unitary.clone(),
        layer,
        children,
    }))
}

/// 按电路执行序快照全部两比特叶子的输入矩阵（DFS：wc → wb → vb → va 递归）。
fn collect_2q_leaf_matrices(node: &Node, out: &mut Vec<Cm>) {
    match node {
        Node::Base(b) => {
            if b.num_qubits == 2 {
                out.push(b.matrix.clone());
            }
        }
        Node::Recursive(r) => {
            for name in ["wc", "wb", "vb", "va"] {
                if let Some((_, child)) = r.children.iter().find(|(n, _)| n == name) {
                    collect_2q_leaf_matrices(child, out);
                }
            }
        }
    }
}

/// 纯函数阶段：按执行序对叶子矩阵链式合成（A.2 对角余量传递）。
///
/// 第 i 个叶子的合成目标为 `U_i · D_in`（`D_in` 为上游余量，初始为单位阵），
/// 分解结果满足 `D_out · circuit ≡ U_i · D_in`：余量 `D_out` 是纯对角矩阵
/// （SMB `two_cnot_phase_diagonal` 构造的逆相位 + 全局相位），因与所有非叶门
/// 可交换而无需重排地传给下一个叶子的输入（右乘 `U_{i+1} · D_out`）。
/// 最后一个叶子用完整合成（0~3 CNOT 自适应）吸收整条余量链。
fn run_a2_chain(matrices: &[Cm]) -> Result<Vec<Circuit>, String> {
    use super::matrix::mat_mul;

    let mut circuits = Vec::with_capacity(matrices.len());
    // 上游对角余量（初始为单位阵）。合成第 i 个叶子时被完全吸收进目标
    // `U_i · D_pending`，随后由新的合成余量 `D_out` 接替传递。
    let mut pending = crz_identity();

    for (index, unitary) in matrices.iter().enumerate() {
        let is_last = index + 1 == matrices.len();
        let target = if is_identity_diagonal(&pending) {
            unitary.clone()
        } else {
            mat_mul(unitary, &pending)
        };

        if is_last {
            circuits.push(synth_leaf_full(&target)?);
        } else {
            let synthesis = synth_two_cnot_with_diagonal(&target)?;
            circuits.push(synthesis.circuit);
            pending = synthesis.diagonal_out;
        }
    }
    Ok(circuits)
}

/// 单位对角门。
fn crz_identity() -> Cm {
    Array2::eye(4)
}

/// 判断对角门是否（数值上）为单位阵。
fn is_identity_diagonal(d: &Cm) -> bool {
    for i in 0..4 {
        let delta = d[[i, i]] - Complex64::new(1.0, 0.0);
        if delta.norm() > 1e-12 {
            return false;
        }
    }
    true
}

/// 写回阶段：按执行序把合成电路写入两比特叶子的 `circuit` 字段。
fn write_back_leaf_circuits(node: &mut Node, circuits: &mut std::vec::IntoIter<Circuit>) -> Result<(), String> {
    match node {
        Node::Base(b) => {
            if b.num_qubits == 2 {
                if let Some(circuit) = circuits.next() {
                    b.circuit = circuit;
                    b.method = "2q";
                }
            }
            Ok(())
        }
        Node::Recursive(r) => {
            for name in ["wc", "wb", "vb", "va"] {
                if let Some((_, child)) = r.children.iter_mut().find(|(n, _)| n == name) {
                    write_back_leaf_circuits(child, circuits)?;
                }
            }
            Ok(())
        }
    }
}

/// A.2 传递 pass：按执行序对叶子做 2-CNOT + 对角余量合成并写回电路。
fn apply_leaf_synthesis(tree: &mut Node, options: &DecomposeOptions) -> Result<(), String> {
    if !options.opt_a2 {
        return Ok(()); // A.2 关闭时叶子已在树构建阶段立即合成。
    }

    let mut matrices = Vec::new();
    collect_2q_leaf_matrices(tree, &mut matrices);
    if matrices.len() <= 1 {
        // 单叶子（n=2 输入）：直接完整合成。
        let circuits: Vec<Circuit> = matrices
            .iter()
            .map(synth_leaf_full)
            .collect::<Result<_, _>>()?;
        let mut iter = circuits.into_iter();
        write_back_leaf_circuits(tree, &mut iter)?;
        return Ok(());
    }

    let circuits = run_a2_chain(&matrices)?;
    let mut iter = circuits.into_iter();
    write_back_leaf_circuits(tree, &mut iter)?;
    Ok(())
}

fn find_child<'a>(node: &'a RecursiveNode, name: &str) -> Option<&'a Node> {
    node.children.iter().find(|(n, _)| n == name).map(|(_, c)| c)
}

fn step3_circuit<'a>(layer: &'a Layer, name: &str) -> Option<&'a Circuit> {
    match name {
        "c" => Some(&layer.step3.c),
        "b" => Some(&layer.step3.b),
        "a" => Some(&layer.step3.a),
        _ => None,
    }
}

/// 组装 Block-ZXZ 电路（对应 Python `assemble_block_zxz_circuit`）。
pub fn assemble_block_zxz_circuit(node: &Node) -> Result<Circuit, String> {
    match node {
        Node::Base(b) => Ok(b.circuit.clone()),
        Node::Recursive(r) => {
            let num_qubits = r.num_qubits;
            let mut circuit = Circuit::new(num_qubits);
            let child_qubits: Vec<Qubit> = (0..num_qubits - 1).map(|q| Qubit::new(q as u32)).collect();
            let top = Qubit::new((num_qubits - 1) as u32);

            for item in &r.layer.assembly {
                match item {
                    AssemblyItem::Child(name) => {
                        let child = find_child(r, name)
                            .ok_or_else(|| format!("missing child {name}"))?;
                        let child_circuit = assemble_block_zxz_circuit(child)?;
                        circuit
                            .compose(&child_circuit, Some(&child_qubits))
                            .map_err(|e| e.to_string())?;
                    }
                    AssemblyItem::Ucrz(name) => {
                        let c = step3_circuit(&r.layer, name)
                            .ok_or_else(|| format!("missing ucrz {name}"))?;
                        circuit.compose(c, None).map_err(|e| e.to_string())?;
                    }
                    AssemblyItem::H => {
                        circuit.h(top).map_err(|e| e.to_string())?;
                    }
                }
            }
            Ok(circuit)
        }
    }
}

/// Block-ZXZ 分解结果。
#[derive(Debug)]
pub struct DecomposeResult {
    pub tree: Node,
    pub circuit: Circuit,
}

/// 返回递归树与组装电路（对应 Python `decompose_block_zxz`，默认全优化）。
pub fn decompose_block_zxz(unitary: &Cm, atol: f64) -> Result<DecomposeResult, String> {
    decompose_block_zxz_with_options(unitary, atol, &DecomposeOptions::default())
}

/// 带优化开关的分解入口。
pub fn decompose_block_zxz_with_options(
    unitary: &Cm,
    atol: f64,
    options: &DecomposeOptions,
) -> Result<DecomposeResult, String> {
    assert_square_unitary(unitary, atol)?;

    // A.2 禁用条件：受控门（块对角）输入（论文 5.1 约束）。
    let effective = if options.opt_a2
        && !options.opt_a2_force
        && is_block_diagonal(unitary, atol)
    {
        let mut o = *options;
        o.opt_a2 = false;
        o
    } else {
        *options
    };

    let mut tree = build_block_zxz_tree(unitary, atol, &effective)?;
    apply_leaf_synthesis(&mut tree, &effective)?;
    let circuit = assemble_block_zxz_circuit(&tree)?;
    Ok(DecomposeResult { tree, circuit })
}