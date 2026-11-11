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

//! Block-ZXZ main flow: recursive tree construction and circuit assembly
//! (step 4) plus the A.1/A.2 optimization passes.
//!
//! Function-by-function mirror of the Python `decompose/block_ZXZ/main.py`,
//! implementing the two optimizations of the paper's Section 5:
//!
//! - **A.1** (`opt_a1`, paper 5.2 / Eq. 11): the middle block pair becomes
//!   `B-tilde = diag(W_A V_C, (Z (x) I) W_A B V_C (Z (x) I))`, while the
//!   C-side UCRz chain drops its trailing closing CNOT and the A-side chain
//!   is time-reversed and drops its leading closing CNOT. By the identities
//!   `H CXc = CZ H` and `CXc H = H CZ` the two removed CNOTs are exactly
//!   absorbed into the B-tilde demux, saving 2 CNOTs per level.
//! - **A.2** (`opt_a2`, paper 5.1 / Shende-Markov-Bullock "up to diagonal"):
//!   every two-qubit leaf but the temporally last one is decomposed into a
//!   2-CNOT circuit plus a pure diagonal remainder (SMB Prop. III.3
//!   construction, see the `vw` module docs); the remainder rolls forward
//!   along the circuit's temporal order and merges into the next leaf's
//!   synthesis target (the diagonal commutes strictly with every non-leaf
//!   gate, so it migrates without any reordering).
//!
//! Leaf (1q/2q) synthesis reuses this crate's `unitary` module, matching the
//! Python project's use of `cqlib`.

use crate::circuit::{Circuit, Parameter, Qubit};
use crate::compile::transform::decompose::unitary::synthesize_numeric_1q_unitary;
use ndarray::Array2;
use num_complex::Complex64;

use super::matrix::Cm;
use super::step1::{assert_square_unitary, block_zxz_step1};
use super::step2::{
    Step2Result, block_zxz_step2, build_middle_pair, demux_middle_branch_a1, demux_pair,
};
use super::ucrz::{
    step3_ucrz_from_diagonal, synthesize_ucrz_circuit, synthesize_ucrz_circuit_reduced_tail,
    synthesize_ucrz_circuit_reversed_reduced_head,
};
use super::vw::{diagonal_gate_circuit, synth_leaf_full, synth_two_cnot_with_diagonal};

/// Decomposition options (the optimization switches of the paper's
/// Section 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecomposeOptions {
    /// A.1: fold each recursion level's two UCRz side CNOTs into the middle
    /// multiplexer (saves 2 CNOTs per level).
    pub opt_a1: bool,
    /// A.2: two-qubit leaf 2-CNOT synthesis + diagonal-remainder rolling
    /// (saves `4^(n-2) - 1` CNOTs).
    pub opt_a2: bool,
    /// Re-enables A.2 on block-diagonal (controlled) inputs, where paper
    /// Sec. 5.1 disables it by default because the rolling would break the
    /// block structure.
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
    /// All optimizations off (matches the original unoptimized pipeline).
    pub fn none() -> Self {
        DecomposeOptions {
            opt_a1: false,
            opt_a2: false,
            opt_a2_force: false,
        }
    }
}

/// Returns the qubit count `log2(nrows)` of a unitary.
fn num_qubits_of(unitary: &Array2<Complex64>) -> usize {
    (unitary.nrows() as f64).log2().round() as usize
}

/// A node of the recursive decomposition tree.
///
/// `RecursiveNode` is boxed because a flat inline layout would inflate every
/// `Node` to ~1.1 KB, including the far more numerous leaves (there are
/// `4^(n-2)` of them). `BaseNode` is deliberately kept inline: leaves are the
/// most common nodes in the tree, so giving each one a heap allocation would
/// cost more than the extra `Node` width it saves.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Node {
    /// A 1/2-qubit leaf.
    Base(BaseNode),
    /// A recursion-layer node.
    Recursive(Box<RecursiveNode>),
}

/// A leaf node (the matrix is cached during construction; under A.2 the
/// circuit is filled later by the chain pass).
#[derive(Debug)]
pub struct BaseNode {
    pub num_qubits: usize,
    /// The leaf's input matrix (used by the A.2 transfer pass).
    pub matrix: Cm,
    /// The synthesized circuit (1q leaves are synthesized eagerly during
    /// the tree build; 2q circuits are filled by `apply_leaf_synthesis`).
    pub circuit: Circuit,
    /// Synthesis-method marker.
    pub method: &'static str,
}

/// A recursion-layer node.
#[derive(Debug)]
pub struct RecursiveNode {
    pub num_qubits: usize,
    pub input_unitary: Cm,
    pub layer: Layer,
    pub children: Vec<(&'static str, Node)>,
}

/// Result of decomposing a single layer.
#[derive(Debug)]
pub struct Layer {
    /// The three UCRz circuits (named `c`/`b`/`a`).
    pub step3: LayerStep3,
    /// The four sub-unitaries that continue the recursion (named
    /// `va`/`vb`/`wb`/`wc`).
    pub subunitaries: Vec<(&'static str, Cm)>,
    /// The assembly order.
    pub assembly: Vec<AssemblyItem>,
    /// Whether A.1 was applied in this layer.
    pub opt_a1: bool,
}

/// Container of the three UCRz circuits.
#[derive(Debug)]
pub struct LayerStep3 {
    pub c: Circuit,
    pub b: Circuit,
    pub a: Circuit,
}

/// One assembly item.
#[derive(Debug, Clone)]
pub enum AssemblyItem {
    /// A recursive child circuit (named `wc`/`wb`/`vb`/`va`).
    Child(&'static str),
    /// A UCRz circuit (named `c`/`b`/`a`).
    Ucrz(&'static str),
    /// The top-qubit Hadamard.
    H,
}

/// Returns the fixed assembly order of one Block-ZXZ layer (mirrors the
/// Python `get_layer_assembly_plan`).
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

/// Returns whether `unitary` is block-diagonal (a controlled-gate shape,
/// the disable condition for A.2). A non-finite off-diagonal element
/// counts as "not block-diagonal" (NaN comparisons are always false and
/// would otherwise make a broken matrix pass the `> atol` test).
pub fn is_block_diagonal(unitary: &Cm, atol: f64) -> bool {
    let n = unitary.nrows();
    let half = n / 2;
    for i in 0..n {
        for j in 0..n {
            let in_offdiag = (i < half) != (j < half);
            if in_offdiag {
                let m = unitary[[i, j]].norm();
                if !m.is_finite() || m > atol {
                    return false;
                }
            }
        }
    }
    true
}

/// Collects the four sub-unitaries that continue the recursion from a
/// plain [`block_zxz_step2`](super::step2::block_zxz_step2) result
/// (mirrors the Python `collect_recursive_subunitaries`).
///
/// Note: this uses the *unoptimized* middle demux of the step-2 result.
/// [`build_block_zxz_layer`] deliberately does not call this helper: when
/// A.1 is enabled the recursion must continue from the B-tilde demux
/// (paper Eq. 11) that also produced the layer's B-side UCRz circuit.
pub fn collect_recursive_subunitaries(step2: &Step2Result) -> Vec<(&'static str, Cm)> {
    vec![
        ("va", step2.a_demux.0.clone()),
        ("vb", step2.middle_demux.0.clone()),
        ("wb", step2.middle_demux.2.clone()),
        ("wc", step2.c_demux.2.clone()),
    ]
}

/// Returns whether `unitary` is a 1/2-qubit recursion leaf (mirrors the
/// Python `is_base_case_unitary`).
pub fn is_base_case_unitary(unitary: &Cm, atol: f64) -> Result<bool, String> {
    assert_square_unitary(unitary, atol)?;
    Ok(unitary.nrows() == 2 || unitary.nrows() == 4)
}

/// One-qubit leaf synthesis (ZYZ path via the crate's `unitary` module).
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

/// Builds a leaf node (deferred synthesis: the matrix is recorded first and
/// the circuit is filled by `apply_leaf_synthesis` in execution order).
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
            // Deferred synthesis: cache the matrix; the circuit is filled
            // by the A.2 pass in execution order. With A.2 off, synthesize
            // immediately.
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

/// Builds one recursion layer (mirrors the Python `build_block_zxz_layer`,
/// with the A.1 switch).
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
        // A.1: the C-side chain drops its trailing closing CNOT and the
        // A-side chain is time-reversed without its leading closing CNOT;
        // each saves one CNOT.
        let c_data = step3_ucrz_from_diagonal(&c_demux.1, atol)?;
        let a_data = step3_ucrz_from_diagonal(&a_demux.1, atol)?;
        let c_circuit =
            synthesize_ucrz_circuit_reduced_tail(&c_data.theta, &c_data.controls, c_data.target)?;
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

    // Middle demux: under A.1 use the B-tilde pair (Eq. 11), otherwise the
    // regular pair.
    let middle_demux = if use_a1 {
        demux_middle_branch_a1(&a_demux.2, &b, &c_demux.0, atol)?
    } else {
        let (u0, u1) = build_middle_pair(&a_demux.2, &b, &c_demux.0);
        demux_pair(&u0, &u1, atol)?
    };
    let b_ucrz = {
        let data = step3_ucrz_from_diagonal(&middle_demux.1, atol)?;
        synthesize_ucrz_circuit(&data.theta, &data.controls, data.target)?
    };

    // The recursion sub-unitaries must come from the SAME middle demux that
    // produced the layer's B-side UCRz circuit: under A.1 this is the
    // B-tilde demux (paper Eq. 11), not the unoptimized demux of
    // `block_zxz_step2`.
    let subunitaries = vec![
        ("va", a_demux.0.clone()),
        ("vb", middle_demux.0.clone()),
        ("wb", middle_demux.2.clone()),
        ("wc", c_demux.2.clone()),
    ];

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

/// Builds the full recursion tree (mirrors the Python `build_block_zxz_tree`).
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
        children.push((*name, build_block_zxz_tree(sub, atol, options)?));
    }
    Ok(Node::Recursive(Box::new(RecursiveNode {
        num_qubits: num_qubits_of(unitary),
        input_unitary: unitary.clone(),
        layer,
        children,
    })))
}

/// Snapshots all two-qubit leaf matrices in circuit execution order
/// (depth-first, `wc -> wb -> vb -> va` at each level).
fn collect_2q_leaf_matrices(node: &Node, out: &mut Vec<Cm>) {
    match node {
        Node::Base(b) => {
            if b.num_qubits == 2 {
                out.push(b.matrix.clone());
            }
        }
        Node::Recursive(r) => {
            for name in ["wc", "wb", "vb", "va"] {
                if let Some((_, child)) = r.children.iter().find(|(n, _)| *n == name) {
                    collect_2q_leaf_matrices(child, out);
                }
            }
        }
    }
}

/// Pure-function stage: chain-synthesizes the leaf matrices in execution
/// order with the A.2 diagonal-remainder transfer.
///
/// The `i`-th leaf's synthesis target is `U_i @ D_in` (`D_in` is the
/// upstream remainder, initially the identity); the synthesis obeys
/// `D_out @ circuit.to_matrix() == U_i @ D_in` exactly - the leaf circuits
/// carry their synthesis global phases internally, so no phase is folded
/// into the rolling remainder. The remainder
/// `D_out` is a pure diagonal matrix (the inverse of the SMB
/// `two_cnot_phase_diagonal` construction) and commutes
/// with every non-leaf gate (Rz/CX/H on higher qubits), so it transfers to
/// the next leaf's input unchanged. The temporally last leaf uses the full
/// 0-3 CX adaptive synthesis to absorb the whole accumulated remainder
/// chain.
///
/// Both absorptions can be numerically infeasible: folding a diagonal into a
/// near-degenerate leaf routinely places the target on a Weyl-chamber
/// boundary (`c -> 0`), where the shared two-qubit backend's KAK
/// canonicalization is accurate to about `1e-10` only - while that backend
/// validates candidates at `1e-10` (`RECONSTRUCTION_EPS` of the KAK module
/// itself is `1e-7`). Rather than failing the whole decomposition, the chain
/// then degrades: the leaf is synthesized unchanged and the pending
/// remainder is emitted as an explicit diagonal gate
/// ([`diagonal_gate_circuit`]) at the same position, which is exact by
/// construction at the cost of two extra CX.
fn run_a2_chain(matrices: &[Cm]) -> Result<Vec<Circuit>, String> {
    use super::matrix::mat_mul;

    let mut circuits = Vec::with_capacity(matrices.len());
    // Upstream diagonal remainder (initially the identity). While
    // synthesizing the i-th leaf it is fully absorbed into the target
    // `U_i @ D_pending` and replaced by the new synthesis remainder
    // `D_out`.
    let mut pending = identity_diagonal();

    for (index, unitary) in matrices.iter().enumerate() {
        let is_last = index + 1 == matrices.len();
        let has_pending = !is_identity_diagonal(&pending);
        let target = if has_pending {
            mat_mul(unitary, &pending)
        } else {
            unitary.clone()
        };

        if is_last {
            match synth_leaf_full(&target) {
                Ok(circuit) => circuits.push(circuit),
                Err(_) if has_pending => {
                    // Degraded path: emit the remainder explicitly before the
                    // leaf (`U_last @ D_pending` applies the remainder first)
                    // and synthesize the leaf unchanged.
                    let mut circuit = diagonal_gate_circuit(&pending)?;
                    let leaf = synth_leaf_full(unitary)?;
                    circuit.compose(&leaf, None).map_err(|e| e.to_string())?;
                    circuits.push(circuit);
                }
                Err(error) => return Err(error),
            }
            continue;
        }

        let synthesis = match synth_two_cnot_with_diagonal(&target) {
            Ok(synthesis) => TwoCnotChainStep {
                circuit: synthesis.circuit,
                diagonal_out: synthesis.diagonal_out,
            },
            Err(_) if has_pending => {
                // Degraded path: emit the remainder explicitly before the
                // leaf and synthesize the leaf unchanged.
                let mut circuit = diagonal_gate_circuit(&pending)?;
                let synthesis = synth_two_cnot_with_diagonal(unitary)?;
                circuit
                    .compose(&synthesis.circuit, None)
                    .map_err(|e| e.to_string())?;
                TwoCnotChainStep {
                    circuit,
                    diagonal_out: synthesis.diagonal_out,
                }
            }
            Err(error) => return Err(error),
        };
        circuits.push(synthesis.circuit);
        pending = synthesis.diagonal_out;
    }
    Ok(circuits)
}

/// One normalized chain step: the emitted circuit plus the diagonal that
/// must be transferred to the next leaf.
struct TwoCnotChainStep {
    circuit: Circuit,
    diagonal_out: Cm,
}

/// The 4x4 identity diagonal.
fn identity_diagonal() -> Cm {
    Array2::eye(4)
}

/// Returns whether the diagonal gate `d` is numerically the identity.
fn is_identity_diagonal(d: &Cm) -> bool {
    for i in 0..4 {
        let delta = d[[i, i]] - Complex64::new(1.0, 0.0);
        if delta.norm() > 1e-12 {
            return false;
        }
    }
    true
}

/// Write-back stage: writes the synthesized circuits into the two-qubit
/// leaves' `circuit` fields in execution order.
fn write_back_leaf_circuits(
    node: &mut Node,
    circuits: &mut std::vec::IntoIter<Circuit>,
) -> Result<(), String> {
    match node {
        Node::Base(b) => {
            if b.num_qubits == 2
                && let Some(circuit) = circuits.next()
            {
                b.circuit = circuit;
                b.method = "2q";
            }
            Ok(())
        }
        Node::Recursive(r) => {
            for name in ["wc", "wb", "vb", "va"] {
                if let Some((_, child)) = r.children.iter_mut().find(|(n, _)| *n == name) {
                    write_back_leaf_circuits(child, circuits)?;
                }
            }
            Ok(())
        }
    }
}

/// A.2 transfer pass: chain-synthesizes the leaves with 2-CNOT +
/// diagonal-remainder and writes the circuits back in execution order.
fn apply_leaf_synthesis(tree: &mut Node, options: &DecomposeOptions) -> Result<(), String> {
    if !options.opt_a2 {
        return Ok(()); // Leaves were synthesized eagerly during the build.
    }

    let mut matrices = Vec::new();
    collect_2q_leaf_matrices(tree, &mut matrices);
    if matrices.len() <= 1 {
        // Single leaf (e.g. a 2-qubit input): synthesize fully directly.
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
    node.children
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| c)
}

fn step3_circuit<'a>(layer: &'a Layer, name: &str) -> Option<&'a Circuit> {
    match name {
        "c" => Some(&layer.step3.c),
        "b" => Some(&layer.step3.b),
        "a" => Some(&layer.step3.a),
        _ => None,
    }
}

/// Assembles the Block-ZXZ circuit (mirrors the Python
/// `assemble_block_zxz_circuit`).
pub fn assemble_block_zxz_circuit(node: &Node) -> Result<Circuit, String> {
    match node {
        Node::Base(b) => Ok(b.circuit.clone()),
        Node::Recursive(r) => {
            let num_qubits = r.num_qubits;
            let mut circuit = Circuit::new(num_qubits);
            let child_qubits: Vec<Qubit> =
                (0..num_qubits - 1).map(|q| Qubit::new(q as u32)).collect();
            let top = Qubit::new((num_qubits - 1) as u32);

            for item in &r.layer.assembly {
                match item {
                    AssemblyItem::Child(name) => {
                        let child =
                            find_child(r, name).ok_or_else(|| format!("missing child {name}"))?;
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

/// Result of a Block-ZXZ decomposition.
#[derive(Debug)]
pub struct DecomposeResult {
    pub tree: Node,
    pub circuit: Circuit,
}

/// Returns the recursion tree and assembled circuit (mirrors the Python
/// `decompose_block_zxz`); both paper Section 5 optimizations are enabled
/// by default.
pub fn decompose_block_zxz(unitary: &Cm, atol: f64) -> Result<DecomposeResult, String> {
    decompose_block_zxz_with_options(unitary, atol, &DecomposeOptions::default())
}

/// Decomposition entry point with explicit optimization switches.
///
/// A.2 is auto-disabled on block-diagonal (controlled-gate) inputs unless
/// `options.opt_a2_force` is set (paper Sec. 5.1 constraint).
pub fn decompose_block_zxz_with_options(
    unitary: &Cm,
    atol: f64,
    options: &DecomposeOptions,
) -> Result<DecomposeResult, String> {
    assert_square_unitary(unitary, atol)?;

    // A.2 disable condition: block-diagonal (controlled-gate) input
    // (paper 5.1 constraint).
    let effective = if options.opt_a2 && !options.opt_a2_force && is_block_diagonal(unitary, atol) {
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
