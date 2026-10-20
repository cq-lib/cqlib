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
//! Block-ZXZ 第三步：均匀控制 Rz（UCRz）Gray 码展开。
//!
//! 逐函数对应 Python `decompose/block_ZXZ/ucrz.py`。

use crate::circuit::{Circuit, Qubit};
use num_complex::Complex64;
use super::matrix::Cm;
use super::step1::{is_matrix_diagonal, is_matrix_unitary};

/// 返回 `index` 的二进制反射 Gray 码。
pub fn gray_code(index: usize) -> usize {
    index ^ (index >> 1)
}

/// 返回长度 `2^k` 的 Gray 码序列。
pub fn gray_code_sequence(num_controls: usize) -> Vec<usize> {
    (0..(1usize << num_controls)).map(gray_code).collect()
}

/// 返回局部 CNOT 控制位切换顺序。
///
/// 使用局部编号约定 `0..num_controls-1`，并包含闭合 CNOT（环状首尾切换）。
pub fn gray_control_positions(num_controls: usize) -> Vec<usize> {
    if num_controls == 0 {
        return Vec::new();
    }
    let sequence = gray_code_sequence(num_controls);
    let len = sequence.len();
    let mut controls = Vec::with_capacity(len);
    for index in 0..len {
        let current = sequence[index];
        let next = sequence[(index + 1) % len];
        let diff = current ^ next;
        controls.push(diff.trailing_zeros() as usize);
    }
    controls
}

/// 位内积的奇偶性（`(left & right)` 中 1 的个数取模 2）。
fn binary_dot_parity(left: usize, right: usize) -> usize {
    (left & right).count_ones() as usize % 2
}

/// 构造 Gray 矩阵 `M^k`（实矩阵，`M[i,j] = (-1)^{bit(i)·gray(j)}`）。
pub fn gray_matrix(num_controls: usize) -> Cm {
    let dim = 1usize << num_controls;
    let gray_codes = gray_code_sequence(num_controls);
    let mut out = Cm::zeros((dim, dim));
    for row in 0..dim {
        for (col, &gray) in gray_codes.iter().enumerate() {
            let sign = if binary_dot_parity(row, gray) == 0 { 1.0 } else { -1.0 };
            out[[row, col]] = Complex64::new(sign, 0.0);
        }
    }
    out
}

/// 从对角幺酉矩阵 `D` 提取 UCRz 目标角 `alpha = -2 * arg(D)`。
pub fn extract_ucrz_alpha_from_diagonal(d_matrix: &Cm, atol: f64) -> Result<Vec<f64>, String> {
    let (r, c) = d_matrix.dim();
    if r != c {
        return Err("D must be a square matrix.".into());
    }
    if !is_matrix_diagonal(d_matrix, atol) {
        return Err("D must be diagonal.".into());
    }
    if !is_matrix_unitary(d_matrix, atol) {
        return Err("D must be unitary.".into());
    }
    Ok((0..r).map(|i| -2.0 * d_matrix[[i, i]].arg()).collect())
}

/// 求解 Gray 码线性系统 `M^k θ = α`，返回 `(θ, M^k)`。
pub fn solve_ucrz_theta_from_alpha(alpha: &[f64]) -> Result<(Vec<f64>, Cm), String> {
    if !is_power_of_two(alpha.len()) {
        return Err("alpha length must be a power of two.".into());
    }
    let num_controls = (alpha.len() as f64).log2().round() as usize;
    let m = gray_matrix(num_controls);
    // θ = (M^T @ α) / 2^k
    let dim = alpha.len();
    let mut theta = vec![0.0f64; dim];
    for row in 0..dim {
        let mut sum = 0.0;
        for (col, &a) in alpha.iter().enumerate() {
            sum += m[[col, row]].re * a;
        }
        theta[row] = sum / (dim as f64);
    }
    Ok((theta, m))
}

fn is_power_of_two(value: usize) -> bool {
    value > 0 && (value & (value - 1)) == 0
}

/// 由 `D` 生成 UCRz 数值描述（对应 Python `step3_ucrz_from_diagonal`）。
pub fn step3_ucrz_from_diagonal(d_matrix: &Cm, atol: f64) -> Result<UcrzData, String> {
    let alpha = extract_ucrz_alpha_from_diagonal(d_matrix, atol)?;
    let (theta, gray_matrix) = solve_ucrz_theta_from_alpha(&alpha)?;
    let num_controls = (alpha.len() as f64).log2().round() as usize;
    Ok(UcrzData {
        alpha,
        theta,
        gray_codes: gray_code_sequence(num_controls),
        controls: gray_control_positions(num_controls),
        gray_matrix,
        num_controls,
        target: num_controls,
    })
}

/// 由 Gray 展开角 `θ` 与局部控制序列合成 UCRz 电路。
///
/// 对应 Python `synthesize_ucrz_circuit`。
pub fn synthesize_ucrz_circuit(
    theta: &[f64],
    controls: &[usize],
    target: usize,
) -> Result<Circuit, String> {
    if theta.is_empty() {
        return Err("theta must contain at least one rotation angle.".into());
    }
    if !is_power_of_two(theta.len()) {
        return Err("theta length must be a power of two.".into());
    }
    let num_controls = (theta.len() as f64).log2().round() as usize;

    if num_controls == 0 {
        if !controls.is_empty() {
            return Err("For zero controls, controls must be empty.".into());
        }
        if target != 0 {
            return Err("For zero controls, the local target must be 0.".into());
        }
        let mut circuit = Circuit::new(1);
        circuit
            .rz(Qubit::new(0), theta[0])
            .map_err(|e| e.to_string())?;
        return Ok(circuit);
    }

    if controls.len() != theta.len() {
        return Err("controls and theta must have the same length.".into());
    }
    let expected_controls = gray_control_positions(num_controls);
    if controls != &expected_controls[..] {
        return Err("controls must follow the local Gray-code convention.".into());
    }
    if target != num_controls {
        return Err("Under local numbering, target must equal num_controls.".into());
    }

    let mut circuit = Circuit::new(num_controls + 1);
    for (&angle, &control) in theta.iter().zip(controls.iter()) {
        circuit
            .rz(Qubit::new(target as u32), angle)
            .map_err(|e| e.to_string())?;
        circuit
            .cx(Qubit::new(control as u32), Qubit::new(target as u32))
            .map_err(|e| e.to_string())?;
    }
    Ok(circuit)
}

/// A.1 优化：去掉尾部闭合 CNOT 的 UCRz 链（用于 C 支路）。
///
/// 标准链时间序 `[Rz(θ0), C0, Rz(θ1), C1, ..., Rz(θm), CX̄]` 的矩阵为
/// `M = CX̄ · Rest`，本函数输出 `Rest` 对应的链（少一个 CNOT）。
/// 层组装中配合 `H · CX̄ = CZ · H` 恒等式，缺失的 CNOT 被吸收进 B̃ 的 demux。
pub fn synthesize_ucrz_circuit_reduced_tail(
    theta: &[f64],
    controls: &[usize],
    target: usize,
) -> Result<Circuit, String> {
    let mut circuit = synthesize_ucrz_circuit(theta, controls, target)?;
    // 去掉时间上最后一个门（闭合 CX）。
    let n = circuit.operations().len();
    circuit
        .remove_operation(n - 1)
        .map_err(|e| e.to_string())?;
    Ok(circuit)
}

/// A.1 优化：时间逆序 UCRz 链去掉首个闭合 CNOT（用于 A 支路）。
///
/// 标准链完整反转得到的链实现同一矩阵（Rz/CX 均为对称矩阵，
/// 反转积 = 原积的转置，而 UCRz 矩阵为对角阵，转置不变）：
/// `[CX̄, Rz(θm), C(m-1), Rz(θ(m-1)), ..., C0, Rz(θ0)]`。
/// 本函数去掉其中的首门 `CX̄`（紧邻其时间更早一侧的 Hadamard，
/// 由 `CX̄ · H = H · CZ` 恒等式吸收进 B̃ 的 demux），输出其余部分。
pub fn synthesize_ucrz_circuit_reversed_reduced_head(
    theta: &[f64],
    controls: &[usize],
    target: usize,
) -> Result<Circuit, String> {
    if theta.is_empty() || !is_power_of_two(theta.len()) {
        return Err("theta length must be a positive power of two.".into());
    }
    let num_controls = (theta.len() as f64).log2().round() as usize;

    if num_controls == 0 {
        // 单门情形无 CNOT 可省。
        let mut circuit = Circuit::new(1);
        circuit
            .rz(Qubit::new(0), theta[0])
            .map_err(|e| e.to_string())?;
        return Ok(circuit);
    }

    if controls.len() != theta.len() {
        return Err("controls and theta must have the same length.".into());
    }
    if controls != &gray_control_positions(num_controls)[..] {
        return Err("controls must follow the local Gray-code convention.".into());
    }
    if target != num_controls {
        return Err("Under local numbering, target must equal num_controls.".into());
    }

    // 时间序（首门 CX̄ 已去除）：
    // [Rz(θm), C(m-1), Rz(θ(m-1)), ..., C0, Rz(θ0)]
    let m = theta.len() - 1;
    let mut circuit = Circuit::new(num_controls + 1);
    circuit
        .rz(Qubit::new(target as u32), theta[m])
        .map_err(|e| e.to_string())?;
    for idx in (0..m).rev() {
        circuit
            .cx(
                Qubit::new(controls[idx] as u32),
                Qubit::new(target as u32),
            )
            .map_err(|e| e.to_string())?;
        circuit
            .rz(Qubit::new(target as u32), theta[idx])
            .map_err(|e| e.to_string())?;
    }
    Ok(circuit)
}

/// UCRz 数值描述 + 电路（对应 Python 返回的 `dict`）。
#[derive(Debug, Clone)]
pub struct UcrzData {
    /// 目标 UCRz 角向量。
    pub alpha: Vec<f64>,
    /// Gray 展开的电路旋转角向量。
    pub theta: Vec<f64>,
    /// Gray 码序列。
    pub gray_codes: Vec<usize>,
    /// 局部控制位序列。
    pub controls: Vec<usize>,
    /// Gray 矩阵 `M^k`。
    pub gray_matrix: Cm,
    /// 控制比特数 `k`。
    pub num_controls: usize,
    /// 局部目标位（本约定下等于 `num_controls`）。
    pub target: usize,
}

/// 由 `D` 一步生成 UCRz 数值信息与局部电路。
///
/// 对应 Python `step3_ucrz_to_circuit`。
pub fn step3_ucrz_to_circuit(d: &Cm, atol: f64) -> Result<UcrzCircuitData, String> {
    let data = step3_ucrz_from_diagonal(d, atol)?;
    let circuit = synthesize_ucrz_circuit(&data.theta, &data.controls, data.target)?;
    Ok(UcrzCircuitData {
        data,
        circuit,
    })
}

/// 含电路的 UCRz 结果。
#[derive(Debug)]
pub struct UcrzCircuitData {
    pub data: UcrzData,
    pub circuit: Circuit,
}