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

//! Gate matrix construction and circuit-level symbolic matrix computation.
//!
//! This module provides:
//!
//! - [`standard_gate_symbolic_matrix`] — per-gate symbolic unitary matrices,
//! - [`circuit_to_symbolic_matrix`] — whole-circuit symbolic matrix with
//!   parameter substitution for [`CircuitGate`](crate::circuit::CircuitGate)
//!   and circuit-backed [`UnitaryGate`](crate::circuit::UnitaryGate),
//! - [`apply_gate_to_matrix`] and [`apply_gate_to_matrix_num`] — gate
//!   application dispatchers that select the fast path (diagonal /
//!   permutation / single-qubit / two-qubit / general) automatically,
//! - [`control_matrix`] — controlled-symbolic matrix construction for
//!   multi-controlled gates.

use crate::circuit::circuit_to_matrix::{
    apply_gate_to_matrix as apply_numeric_gate_to_numeric_matrix, circuit_to_matrix,
};
use crate::circuit::symbolic_matrix::PARALLEL_THRESHOLD_OPS;
use crate::circuit::symbolic_matrix::matrix::{
    SymbolicComplex, SymbolicMatrix, UnsafeSymbolicSlice, apply_numeric_diagonal_gate,
    apply_numeric_permutation_gate, apply_symbolic_diagonal_gate, apply_symbolic_permutation_gate,
    numeric_is_zero, substitute_symbolic_matrix, symbolic_eye,
};
use crate::circuit::{Circuit, CircuitError, CircuitParam, Instruction, Parameter, StandardGate};
use ndarray::Array2;
use ndarray::parallel::prelude::*;
use num_complex::Complex64;
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};

enum SymbolicGateKind {
    Diagonal(Vec<SymbolicComplex>),
    Permutation(Vec<(usize, SymbolicComplex)>),
    Dense,
}

enum NumericGateKind {
    Diagonal(Vec<Complex64>),
    Permutation(Vec<(usize, Complex64)>),
    Dense,
}

const NUMERIC_RUN_DENSE_FLUSH_MAX_DIM: usize = 64;

struct NumericRun {
    matrix: Array2<Complex64>,
    len: usize,
}

impl NumericRun {
    fn new(dim: usize) -> Self {
        Self {
            matrix: Array2::eye(dim),
            len: 0,
        }
    }

    fn push(&mut self, gate: &Array2<Complex64>, bits: &[usize]) -> Result<(), CircuitError> {
        validate_target_bits(self.matrix.nrows(), bits)?;
        apply_numeric_gate_to_numeric_matrix(&mut self.matrix, gate, bits)?;
        self.len += 1;
        Ok(())
    }

    fn reset(&mut self) {
        self.matrix.fill(Complex64::new(0.0, 0.0));
        for idx in 0..self.matrix.nrows() {
            self.matrix[[idx, idx]] = Complex64::new(1.0, 0.0);
        }
        self.len = 0;
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Returns the symbolic unitary matrix for a [`StandardGate`].
///
/// Non-parametric gates (H, X, SWAP, CCX, …) delegate to the numerical
/// [`StandardGate::matrix`] and convert the result via
/// the internal numeric-to-symbolic conversion. Parametric gates (RX, RY, RZ, U, …)
/// build their matrices symbolically so that parameters remain as
/// [`Parameter`] expressions for deferred evaluation.
///
/// # Errors
///
/// Returns [`CircuitError::ParameterCountMismatch`] if the number of
/// parameters does not match the gate's declared count.
pub fn standard_gate_symbolic_matrix(
    gate: StandardGate,
    params: &[Parameter],
) -> Result<SymbolicMatrix, CircuitError> {
    validate_params(gate, params)?;
    let z = SymbolicComplex::zero();
    let o = SymbolicComplex::one();
    let i = SymbolicComplex::i();
    let neg_i = -i.clone();
    let h = SymbolicComplex::from_real(1.0 / std::f64::consts::SQRT_2);

    Ok(match gate {
        StandardGate::H
        | StandardGate::I
        | StandardGate::S
        | StandardGate::SDG
        | StandardGate::T
        | StandardGate::TDG
        | StandardGate::X
        | StandardGate::Y
        | StandardGate::Z
        | StandardGate::X2P
        | StandardGate::X2M
        | StandardGate::Y2P
        | StandardGate::Y2M
        | StandardGate::SWAP
        | StandardGate::CX
        | StandardGate::CY
        | StandardGate::CZ
        | StandardGate::CCX => {
            let numeric = gate
                .matrix(&[])
                .map_err(|_| CircuitError::NoMatrixRepresentation)?;
            symbolic_matrix_from_numeric(numeric.as_ref())
        }
        StandardGate::RX => {
            let c = cos_half(&params[0]);
            let s = neg_i_times(sin_half(&params[0]));
            ndarray::array![[c.clone(), s.clone()], [s, c]]
        }
        StandardGate::RY => {
            let c = cos_half(&params[0]);
            let s = SymbolicComplex::from_real(sin_half(&params[0]));
            ndarray::array![[c.clone(), -s.clone()], [s, c]]
        }
        StandardGate::RZ => {
            let h = half(&params[0]);
            ndarray::array![
                [exp_neg_i(h.clone()), z.clone()],
                [z, SymbolicComplex::exp_i(h)]
            ]
        }
        StandardGate::Phase => ndarray::array![
            [o.clone(), z.clone()],
            [z, SymbolicComplex::exp_i(params[0].clone())]
        ],
        StandardGate::GPhase => {
            let phase = SymbolicComplex::exp_i(params[0].clone());
            ndarray::array![[phase.clone(), z.clone()], [z, phase]]
        }
        StandardGate::RXX => {
            let c = cos_half(&params[0]);
            let s = neg_i_times(sin_half(&params[0]));
            ndarray::array![
                [c.clone(), z.clone(), z.clone(), s.clone()],
                [z.clone(), c.clone(), s.clone(), z.clone()],
                [z.clone(), s.clone(), c.clone(), z.clone()],
                [s, z.clone(), z, c]
            ]
        }
        StandardGate::RYY => {
            let c = cos_half(&params[0]);
            let s = i_times(sin_half(&params[0]));
            let ns = -s.clone();
            ndarray::array![
                [c.clone(), z.clone(), z.clone(), s.clone()],
                [z.clone(), c.clone(), ns.clone(), z.clone()],
                [z.clone(), ns, c.clone(), z.clone()],
                [s, z.clone(), z, c]
            ]
        }
        StandardGate::RZZ => {
            let h = half(&params[0]);
            let exp_neg = exp_neg_i(h.clone());
            let exp_pos = SymbolicComplex::exp_i(h);
            ndarray::array![
                [exp_neg.clone(), z.clone(), z.clone(), z.clone()],
                [z.clone(), exp_pos.clone(), z.clone(), z.clone()],
                [z.clone(), z.clone(), exp_pos, z.clone()],
                [z.clone(), z.clone(), z, exp_neg]
            ]
        }
        StandardGate::RZX => {
            let c = cos_half(&params[0]);
            let s = i_times(sin_half(&params[0]));
            let ns = -s.clone();
            ndarray::array![
                [c.clone(), ns.clone(), z.clone(), z.clone()],
                [ns, c.clone(), z.clone(), z.clone()],
                [z.clone(), z.clone(), c.clone(), s.clone()],
                [z.clone(), z.clone(), s, c]
            ]
        }
        StandardGate::CRX => {
            let c = cos_half(&params[0]);
            let s = neg_i_times(sin_half(&params[0]));
            ndarray::array![
                [o.clone(), z.clone(), z.clone(), z.clone()],
                [z.clone(), o.clone(), z.clone(), z.clone()],
                [z.clone(), z.clone(), c.clone(), s.clone()],
                [z.clone(), z.clone(), s, c]
            ]
        }
        StandardGate::CRY => {
            let c = cos_half(&params[0]);
            let s = SymbolicComplex::from_real(sin_half(&params[0]));
            ndarray::array![
                [o.clone(), z.clone(), z.clone(), z.clone()],
                [z.clone(), o.clone(), z.clone(), z.clone()],
                [z.clone(), z.clone(), c.clone(), -s.clone()],
                [z.clone(), z.clone(), s, c]
            ]
        }
        StandardGate::CRZ => {
            let h = half(&params[0]);
            ndarray::array![
                [o.clone(), z.clone(), z.clone(), z.clone()],
                [z.clone(), o.clone(), z.clone(), z.clone()],
                [z.clone(), z.clone(), exp_neg_i(h.clone()), z.clone()],
                [z.clone(), z.clone(), z, SymbolicComplex::exp_i(h)]
            ]
        }
        StandardGate::RXY => {
            let c = cos_half(&params[0]);
            let s = SymbolicComplex::from_real(sin_half(&params[0]));
            let upper = neg_i.clone() * exp_neg_i(params[1].clone()) * s.clone();
            let lower = neg_i * SymbolicComplex::exp_i(params[1].clone()) * s;
            ndarray::array![[c.clone(), upper], [lower, c]]
        }
        StandardGate::U => {
            let c = cos_half(&params[0]);
            let s = SymbolicComplex::from_real(sin_half(&params[0]));
            let exp_phi = SymbolicComplex::exp_i(params[1].clone());
            let exp_lambda = SymbolicComplex::exp_i(params[2].clone());
            let exp_phi_lambda = SymbolicComplex::exp_i(params[1].clone() + params[2].clone());
            ndarray::array![
                [c.clone(), -(exp_lambda * s.clone())],
                [exp_phi * s, exp_phi_lambda * c]
            ]
        }
        StandardGate::XY => {
            let upper = neg_i.clone() * exp_neg_i(params[0].clone());
            let lower = neg_i * SymbolicComplex::exp_i(params[0].clone());
            ndarray::array![[z, upper], [lower, SymbolicComplex::zero()]]
        }
        StandardGate::XY2P => {
            let upper = neg_i.clone() * exp_neg_i(params[0].clone()) * h.clone();
            let lower = neg_i * SymbolicComplex::exp_i(params[0].clone()) * h.clone();
            ndarray::array![[h.clone(), upper], [lower, h]]
        }
        StandardGate::XY2M => {
            let upper = i.clone() * exp_neg_i(params[0].clone()) * h.clone();
            let lower = i * SymbolicComplex::exp_i(params[0].clone()) * h.clone();
            ndarray::array![[h.clone(), upper], [lower, h]]
        }
        StandardGate::FSIM => {
            let c = SymbolicComplex::from_real(params[0].cos());
            let s = neg_i_times(params[0].sin());
            let phase = exp_neg_i(params[1].clone());
            ndarray::array![
                [o.clone(), z.clone(), z.clone(), z.clone()],
                [z.clone(), c.clone(), s.clone(), z.clone()],
                [z.clone(), s, c, z.clone()],
                [z.clone(), z.clone(), z, phase]
            ]
        }
    })
}

/// Constructs a controlled-symbolic matrix by embedding `base` into the
/// bottom-right block of a larger identity matrix.
///
/// For `num_ctrls` control qubits the resulting dimension is
/// `base_dim × 2^num_ctrls`. All control-basis states map to identity
/// rows; only when every control qubit is `|1⟩` does the base gate act.
pub fn control_matrix(base: &SymbolicMatrix, num_ctrls: usize) -> SymbolicMatrix {
    if num_ctrls == 0 {
        return base.clone();
    }

    let base_dim = base.nrows();
    let total_dim = base_dim << num_ctrls;
    let mut matrix = symbolic_eye(total_dim);
    let start = total_dim - base_dim;

    for row in 0..base_dim {
        for col in 0..base_dim {
            matrix[[start + row, start + col]] = base[[row, col]].clone();
        }
    }

    matrix
}

/// Computes the symbolic unitary matrix representation of a quantum circuit.
///
/// This is the symbolic counterpart of
/// [`circuit_to_matrix()`](fn@crate::circuit::circuit_to_matrix): instead
/// of evaluating gate parameters to `f64` values immediately, it preserves
/// them as [`Parameter`] expressions so that the resulting matrix can be
/// evaluated later with different bindings via
/// [`evaluate_symbolic_matrix()`](crate::circuit::symbolic_matrix::evaluate_symbolic_matrix).
///
/// # Qubit ordering
///
/// The `qubits_order` parameter controls which qubit maps to which bit
/// position in the matrix:
///
/// - `None` — qubits are sorted by index in ascending order (qubit 0 → bit 0).
/// - `Some(order)` — the first entry maps to the least-significant bit.
///
/// The order must contain exactly the same set of qubit indices as the
/// circuit, with no duplicates.
///
/// # Errors
///
/// - [`CircuitError::InvalidOperation`] if `qubits_order` does not match
///   the circuit's qubit set, or if the circuit contains control-flow
///   operations.
/// - [`CircuitError::NoMatrixRepresentation`] for non-unitary operations
///   (measure, reset) or gates without a matrix definition.
/// - [`CircuitError::ParameterCountMismatch`] if a gate or circuit gate
///   receives the wrong number of parameters.
/// - [`CircuitError::QubitNotFound`] if an operation references a qubit
///   not present in the circuit.
pub fn circuit_to_symbolic_matrix(
    circuit: &Circuit,
    qubits_order: Option<&[usize]>,
) -> Result<SymbolicMatrix, CircuitError> {
    let circuit_qubits: Vec<usize> = circuit.qubits().iter().map(|q| q.index()).collect();
    let num_qubits = circuit_qubits.len();
    let dim = 1usize.checked_shl(num_qubits as u32).ok_or_else(|| {
        CircuitError::InvalidOperation(format!(
            "cannot build matrix for {num_qubits} qubits: dimension overflows usize"
        ))
    })?;
    dim.checked_mul(dim).ok_or_else(|| {
        CircuitError::InvalidOperation(format!(
            "cannot build matrix for {num_qubits} qubits: matrix element count overflows usize"
        ))
    })?;

    let target_order: Vec<usize> = match qubits_order {
        Some(order) => {
            let circuit_set: HashSet<usize> = circuit_qubits.iter().copied().collect();
            let order_set: HashSet<usize> = order.iter().copied().collect();
            if circuit_set != order_set || circuit_set.len() != order.len() {
                return Err(CircuitError::InvalidOperation(format!(
                    "qubits_order mismatch! Circuit has {:?}, but order provided is {:?}",
                    circuit_qubits, order
                )));
            }
            order.to_vec()
        }
        None => {
            let mut sorted = circuit_qubits.clone();
            sorted.sort();
            sorted
        }
    };

    let qubit_bit_map: HashMap<usize, usize> = target_order
        .iter()
        .enumerate()
        .map(|(i, &q_id)| (q_id, i))
        .collect();

    let full_bits: Vec<usize> = (0..num_qubits).collect();
    let mut matrix: Option<SymbolicMatrix> = None;
    let mut numeric_run = NumericRun::new(dim);

    for op in circuit.operations() {
        let bits: SmallVec<[usize; 3]> = op
            .qubits
            .iter()
            .map(|q| {
                qubit_bit_map
                    .get(&q.index())
                    .copied()
                    .ok_or(CircuitError::QubitNotFound(q.id()))
            })
            .collect::<Result<_, _>>()?;
        let params = resolve_params(circuit, &op.params)?;

        match &op.instruction {
            Instruction::Standard(gate) => {
                let reversed_bits: SmallVec<[usize; 3]> = bits.iter().copied().rev().collect();
                if let Some(numeric_params) = constant_params(&params)? {
                    let gate_matrix = gate
                        .matrix(&numeric_params)
                        .map_err(|_| CircuitError::NoMatrixRepresentation)?;
                    push_numeric_gate(
                        &mut matrix,
                        &mut numeric_run,
                        gate_matrix.as_ref(),
                        &reversed_bits,
                        &full_bits,
                    )?;
                } else {
                    flush_numeric_run(&mut matrix, &mut numeric_run, &full_bits)?;
                    let matrix = matrix.get_or_insert_with(|| symbolic_eye(dim));
                    apply_standard_gate_to_matrix(matrix, *gate, &reversed_bits, &params)?;
                }
            }
            Instruction::McGate(mc_gate) => {
                let reversed_bits: SmallVec<[usize; 3]> = bits.iter().copied().rev().collect();
                if let Some(numeric_params) = constant_params(&params)? {
                    let gate_matrix = mc_gate
                        .matrix(&numeric_params)
                        .map_err(|_| CircuitError::NoMatrixRepresentation)?;
                    push_numeric_gate(
                        &mut matrix,
                        &mut numeric_run,
                        gate_matrix.as_ref(),
                        &reversed_bits,
                        &full_bits,
                    )?;
                } else {
                    flush_numeric_run(&mut matrix, &mut numeric_run, &full_bits)?;
                    let matrix = matrix.get_or_insert_with(|| symbolic_eye(dim));
                    let base = standard_gate_symbolic_matrix(*mc_gate.base_gate(), &params)?;
                    let gate_matrix = control_matrix(&base, mc_gate.num_ctrl_qubits());
                    apply_gate_to_matrix(matrix, &gate_matrix, &reversed_bits)?;
                }
            }
            Instruction::UnitaryGate(u_gate) => {
                let reversed_bits: SmallVec<[usize; 3]> = bits.iter().copied().rev().collect();
                if let Some(unitary_matrix) = u_gate.symbolic_matrix() {
                    if let Some(concrete_params) = constant_params(&params)? {
                        let gate_matrix = u_gate.matrix_for_params(&concrete_params)?;
                        push_numeric_gate(
                            &mut matrix,
                            &mut numeric_run,
                            gate_matrix.as_ref(),
                            &reversed_bits,
                            &full_bits,
                        )?;
                    } else {
                        flush_numeric_run(&mut matrix, &mut numeric_run, &full_bits)?;
                        let matrix_params = u_gate
                            .matrix_params()
                            .ok_or(CircuitError::SymbolicParameterError)?;
                        let expected = matrix_params.len();
                        let actual = params.len();
                        if actual != expected {
                            return Err(CircuitError::ParameterCountMismatch { expected, actual });
                        }
                        let replacements: HashMap<String, Parameter> = matrix_params
                            .iter()
                            .cloned()
                            .zip(params.iter().cloned())
                            .collect();
                        let sub_matrix =
                            substitute_symbolic_matrix(unitary_matrix.clone(), &replacements)?;
                        let matrix = matrix.get_or_insert_with(|| symbolic_eye(dim));
                        apply_gate_to_matrix(matrix, &sub_matrix, &reversed_bits)?;
                    }
                } else if u_gate.circuit().is_some() && u_gate.matrix().is_none() {
                    if let Some(numeric_params) = constant_params(&params)? {
                        let gate_matrix = u_gate.matrix_for_params(&numeric_params)?;
                        push_numeric_gate(
                            &mut matrix,
                            &mut numeric_run,
                            gate_matrix.as_ref(),
                            &bits,
                            &full_bits,
                        )?;
                    } else {
                        flush_numeric_run(&mut matrix, &mut numeric_run, &full_bits)?;
                        let matrix = matrix.get_or_insert_with(|| symbolic_eye(dim));
                        let sub_circuit = u_gate
                            .circuit()
                            .as_ref()
                            .ok_or(CircuitError::NoMatrixRepresentation)?;
                        let symbols = sub_circuit.used_symbols();
                        let expected = symbols.len();
                        let actual = params.len();
                        if actual != expected {
                            return Err(CircuitError::ParameterCountMismatch { expected, actual });
                        }
                        let replacements: HashMap<String, Parameter> = symbols
                            .iter()
                            .cloned()
                            .zip(params.iter().cloned())
                            .collect();
                        let sub_matrix = sub_circuit.symbolic_matrix()?;
                        let sub_matrix =
                            substitute_symbolic_matrix((*sub_matrix).clone(), &replacements)?;
                        apply_gate_to_matrix(matrix, &sub_matrix, &bits)?;
                    }
                } else if let Some(concrete_params) = constant_params(&params)? {
                    let gate_matrix = u_gate.matrix_for_params(&concrete_params)?;
                    push_numeric_gate(
                        &mut matrix,
                        &mut numeric_run,
                        gate_matrix.as_ref(),
                        &reversed_bits,
                        &full_bits,
                    )?;
                } else {
                    flush_numeric_run(&mut matrix, &mut numeric_run, &full_bits)?;
                    let concrete_params: Vec<f64> = params
                        .iter()
                        .map(|param| {
                            param
                                .evaluate(&None)
                                .map_err(|_| CircuitError::SymbolicParameterError)
                        })
                        .collect::<Result<_, _>>()?;
                    let gate_matrix = u_gate.matrix_for_params(&concrete_params)?;
                    let sub_matrix = symbolic_matrix_from_numeric(gate_matrix.as_ref());
                    let matrix = matrix.get_or_insert_with(|| symbolic_eye(dim));
                    apply_gate_to_matrix(matrix, &sub_matrix, &reversed_bits)?;
                }
            }
            Instruction::CircuitGate(circuit_gate) => {
                let symbols = circuit_gate.signature_params();
                let expected = symbols.len();
                let actual = params.len();
                if actual != expected {
                    return Err(CircuitError::ParameterCountMismatch { expected, actual });
                }
                let replacements: HashMap<String, Parameter> = symbols
                    .iter()
                    .cloned()
                    .zip(params.iter().cloned())
                    .collect();
                if let Some(numeric_params) = constant_params(&params)? {
                    let mut bindings = HashMap::new();
                    for (symbol, value) in symbols.iter().zip(numeric_params.iter()) {
                        bindings.insert(symbol.as_str(), *value);
                    }
                    let sub_circuit = circuit_gate
                        .circuit()
                        .circuit()
                        .assign_parameters(&Some(bindings))
                        .map_err(|_| CircuitError::SymbolicParameterError)?;
                    let sub_matrix = circuit_to_matrix(&sub_circuit, None)?;
                    push_numeric_gate(
                        &mut matrix,
                        &mut numeric_run,
                        &sub_matrix,
                        &bits,
                        &full_bits,
                    )?;
                } else {
                    flush_numeric_run(&mut matrix, &mut numeric_run, &full_bits)?;
                    let matrix = matrix.get_or_insert_with(|| symbolic_eye(dim));
                    let sub_matrix = circuit_gate.symbolic_matrix()?;
                    let sub_matrix =
                        substitute_symbolic_matrix((*sub_matrix).clone(), &replacements)?;
                    apply_gate_to_matrix(matrix, &sub_matrix, &bits)?;
                }
            }
            Instruction::ClassicalControl(_) => {
                flush_numeric_run(&mut matrix, &mut numeric_run, &full_bits)?;
                return Err(CircuitError::InvalidOperation(
                    "control-flow operations do not have an unconditional matrix representation"
                        .to_string(),
                ));
            }
            Instruction::ClassicalData(_) => {
                flush_numeric_run(&mut matrix, &mut numeric_run, &full_bits)?;
                return Err(CircuitError::NoMatrixRepresentation);
            }
            Instruction::Directive(directive) => match directive {
                crate::circuit::Directive::Barrier => continue,
                crate::circuit::Directive::Measure | crate::circuit::Directive::Reset => {
                    flush_numeric_run(&mut matrix, &mut numeric_run, &full_bits)?;
                    return Err(CircuitError::NoMatrixRepresentation);
                }
            },
            Instruction::Delay => continue,
        }
    }

    flush_numeric_run(&mut matrix, &mut numeric_run, &full_bits)?;
    let mut matrix = matrix.unwrap_or_else(|| symbolic_eye(dim));

    let global_phase = circuit.global_phase();
    if !global_phase.is_zero() {
        let phase = SymbolicComplex::exp_i(global_phase);
        matrix
            .as_slice_mut()
            .expect("symbolic matrix must be contiguous")
            .par_iter_mut()
            .for_each(|value| {
                let old = std::mem::take(value);
                *value = &phase * old;
            });
    }

    Ok(matrix)
}

fn constant_params(params: &[Parameter]) -> Result<Option<Vec<f64>>, CircuitError> {
    if params.iter().any(|param| !param.is_constant()) {
        return Ok(None);
    }
    params
        .iter()
        .map(|param| {
            param
                .evaluate(&None)
                .map_err(|_| CircuitError::SymbolicParameterError)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn push_numeric_gate(
    matrix: &mut Option<SymbolicMatrix>,
    numeric_run: &mut NumericRun,
    gate: &Array2<Complex64>,
    bits: &[usize],
    full_bits: &[usize],
) -> Result<(), CircuitError> {
    if matrix.is_none() || numeric_run.matrix.nrows() <= NUMERIC_RUN_DENSE_FLUSH_MAX_DIM {
        numeric_run.push(gate, bits)
    } else {
        flush_numeric_run(matrix, numeric_run, full_bits)?;
        apply_gate_to_matrix_num(
            matrix
                .as_mut()
                .expect("symbolic matrix must exist after numeric-run flush"),
            gate,
            bits,
        )
    }
}

fn flush_numeric_run(
    matrix: &mut Option<SymbolicMatrix>,
    numeric_run: &mut NumericRun,
    full_bits: &[usize],
) -> Result<(), CircuitError> {
    if numeric_run.is_empty() {
        return Ok(());
    }

    if let Some(matrix) = matrix.as_mut() {
        apply_gate_to_matrix_num(matrix, &numeric_run.matrix, full_bits)?;
    } else {
        *matrix = Some(symbolic_matrix_from_numeric(&numeric_run.matrix));
    }
    numeric_run.reset();
    Ok(())
}

/// Applies a [`StandardGate`] to `matrix`, automatically choosing the numeric
/// fast path when all supplied parameters are constant (contain no free
/// symbols).
///
/// # Arguments
///
/// * `matrix` — the state matrix to update in-place.
/// * `gate` — the standard gate to apply.
/// * `bits` — target qubit positions in **Little-Endian** order.
/// * `params` — gate parameters.  If any parameter contains free symbols, the
///   symbolic path is used; otherwise the parameters are evaluated to `f64`
///   and the numeric fast path is taken.
///
/// # Errors
///
/// Returns [`CircuitError::ParameterCountMismatch`] if the number of
/// parameters does not match the gate's declared count, or
/// [`CircuitError::NoMatrixRepresentation`] if the gate has no matrix.
pub fn apply_standard_gate_to_matrix(
    matrix: &mut SymbolicMatrix,
    gate: StandardGate,
    bits: &[usize],
    params: &[Parameter],
) -> Result<(), CircuitError> {
    let has_symbols = params.iter().any(|p| !p.is_constant());

    if has_symbols {
        let gate_matrix = standard_gate_symbolic_matrix(gate, params)?;
        apply_gate_to_matrix(matrix, &gate_matrix, bits)?;
    } else {
        let numeric_params: Vec<f64> = params
            .iter()
            .map(|p| {
                p.evaluate(&None)
                    .map_err(|_| CircuitError::SymbolicParameterError)
            })
            .collect::<Result<_, _>>()?;
        let gate_matrix = gate
            .matrix(&numeric_params)
            .map_err(|_| CircuitError::NoMatrixRepresentation)?;
        apply_gate_to_matrix_num(matrix, gate_matrix.as_ref(), bits)?;
    }
    Ok(())
}

/// Applies a gate matrix to the target qubit positions of a state matrix.
///
/// Dispatches to an optimised code path based on the number of target
/// qubits:
///
/// | `bits.len()` | Code path                    |
/// |-------------|------------------------------|
/// | 1           | [`apply_single_qubit_gate`]  |
/// | 2           | [`apply_two_qubit_gate`]     |
/// | 3+          | [`apply_general_gate`]       |
///
/// The `bits` parameter uses the system's **Little-Endian** convention:
/// qubit 0 is the least-significant bit. Callers that receive gate-local
/// Big-Endian bit order (e.g. from [`StandardGate`] matrices) must reverse
/// the bits before calling this function.
pub fn apply_gate_to_matrix(
    matrix: &mut SymbolicMatrix,
    gate: &SymbolicMatrix,
    bits: &[usize],
) -> Result<(), CircuitError> {
    validate_target_bits(matrix.nrows(), bits)?;
    let expected_dim = 1usize << bits.len();
    if gate.nrows() != expected_dim || gate.ncols() != expected_dim {
        return Err(CircuitError::QubitCountMismatch {
            expected: gate.nrows().trailing_zeros() as usize,
            actual: bits.len(),
        });
    }
    match classify_symbolic_gate(gate) {
        SymbolicGateKind::Diagonal(diagonal) => {
            apply_symbolic_diagonal_gate(matrix, &diagonal, bits);
        }
        SymbolicGateKind::Permutation(permutation) => {
            apply_symbolic_permutation_gate(matrix, &permutation, bits);
        }
        SymbolicGateKind::Dense => match bits.len() {
            1 => apply_single_qubit_gate(matrix, gate, bits[0]),
            2 => apply_two_qubit_gate(matrix, gate, bits[0], bits[1]),
            _ => apply_general_gate(matrix, gate, bits),
        },
    }
    Ok(())
}

/// Applies a single-qubit gate to the given bit position of `matrix`.
///
/// For each pair of rows that differ only in the target bit, the gate is
/// multiplied as a 2×2 matrix transformation:
///
/// ```text
/// [v0']   [u00 u01] [v0]
/// [v1'] = [u10 u11] [v1]
/// ```
///
/// Uses an internal disjoint-slice wrapper and rayon parallelism when the
/// matrix exceeds the configured element threshold.
pub fn apply_single_qubit_gate(matrix: &mut SymbolicMatrix, gate: &SymbolicMatrix, bit: usize) {
    let dim = matrix.nrows();
    let cols = matrix.ncols();
    let step = 1usize << bit;
    let total_ops = dim.saturating_mul(cols);
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let u00 = &gate[[0, 0]];
    let u01 = &gate[[0, 1]];
    let u10 = &gate[[1, 0]];
    let u11 = &gate[[1, 1]];

    let slice = matrix
        .as_slice_mut()
        .expect("Symbolic matrix must be contiguous");
    let unsafe_slice = UnsafeSymbolicSlice::new(slice);

    let process_block = |i: usize| {
        // SAFETY: Each worker processes a unique block starting at `i` and
        // touches rows `i + j` and `i + j + step` where `j < step`. Because
        // the outer iterator steps by `step * 2`, no two workers can ever
        // access the same row index, so there is no aliasing.
        unsafe {
            for j in 0..step {
                let r0_idx = i + j;
                let r1_idx = r0_idx + step;

                let r0_ptr = unsafe_slice.row_ptr(r0_idx, cols);
                let r1_ptr = unsafe_slice.row_ptr(r1_idx, cols);

                for col in 0..cols {
                    let v0 = (*r0_ptr.add(col)).clone();
                    let v1 = (*r1_ptr.add(col)).clone();

                    *r0_ptr.add(col) = u00 * &v0 + u01 * &v1;
                    *r1_ptr.add(col) = u10 * &v0 + u11 * &v1;
                }
            }
        }
    };

    if parallel {
        // SAFETY: `into_par_iter().step_by(step * 2)` partitions the row
        // index space into disjoint blocks, each processed by a single
        // worker. The raw pointers derived from `UnsafeSymbolicSlice` are
        // never aliased across workers, satisfying Rust's safety rules.
        (0..dim)
            .into_par_iter()
            .step_by(step * 2)
            .for_each(process_block);
    } else {
        (0..dim).step_by(step * 2).for_each(process_block);
    }
}

/// Applies a two-qubit gate to the given bit positions of `matrix`.
///
/// For each group of four rows that differ only in the two target bits,
/// the gate is multiplied as a 4×4 matrix transformation. The index
/// mapping uses a bit-insertion scheme to compute the base row index
/// from the compact iteration variable `i`.
///
/// Uses an internal disjoint-slice wrapper and rayon parallelism when the
/// matrix exceeds the configured element threshold.
pub fn apply_two_qubit_gate(
    matrix: &mut SymbolicMatrix,
    gate: &SymbolicMatrix,
    b0: usize,
    b1: usize,
) {
    let dim = matrix.nrows();
    let cols = matrix.ncols();
    let (low, high) = if b0 < b1 { (b0, b1) } else { (b1, b0) };
    let mask_low = (1usize << low) - 1;
    let mask_high = (1usize << high) - 1;
    let off0 = 1usize << b0;
    let off1 = 1usize << b1;
    let loop_limit = dim >> 2;
    let total_ops = dim.saturating_mul(cols);
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let g00 = &gate[[0, 0]];
    let g01 = &gate[[0, 1]];
    let g02 = &gate[[0, 2]];
    let g03 = &gate[[0, 3]];
    let g10 = &gate[[1, 0]];
    let g11 = &gate[[1, 1]];
    let g12 = &gate[[1, 2]];
    let g13 = &gate[[1, 3]];
    let g20 = &gate[[2, 0]];
    let g21 = &gate[[2, 1]];
    let g22 = &gate[[2, 2]];
    let g23 = &gate[[2, 3]];
    let g30 = &gate[[3, 0]];
    let g31 = &gate[[3, 1]];
    let g32 = &gate[[3, 2]];
    let g33 = &gate[[3, 3]];

    let slice = matrix
        .as_slice_mut()
        .expect("Symbolic matrix must be contiguous");
    let unsafe_slice = UnsafeSymbolicSlice::new(slice);

    let process_idx = |i: usize| {
        let left_part = (i & !mask_low) << 1;
        let right_part = i & mask_low;
        let tmp = left_part | right_part;

        let left_final = (tmp & !mask_high) << 1;
        let right_final = tmp & mask_high;
        let base = left_final | right_final;

        let r0_idx = base;
        let r1_idx = base | off0;
        let r2_idx = base | off1;
        let r3_idx = base | off0 | off1;

        // SAFETY: Each `i` in the outer iterator maps to a unique `base`,
        // and the four derived row indices (`base`, `base|off0`, `base|off1`,
        // `base|off0|off1`) never overlap with those of any other `i`. Thus
        // no two workers touch the same row index, so there is no aliasing.
        unsafe {
            let p0 = unsafe_slice.row_ptr(r0_idx, cols);
            let p1 = unsafe_slice.row_ptr(r1_idx, cols);
            let p2 = unsafe_slice.row_ptr(r2_idx, cols);
            let p3 = unsafe_slice.row_ptr(r3_idx, cols);

            for col in 0..cols {
                let v0 = (*p0.add(col)).clone();
                let v1 = (*p1.add(col)).clone();
                let v2 = (*p2.add(col)).clone();
                let v3 = (*p3.add(col)).clone();

                *p0.add(col) = g00 * &v0 + g01 * &v1 + g02 * &v2 + g03 * &v3;
                *p1.add(col) = g10 * &v0 + g11 * &v1 + g12 * &v2 + g13 * &v3;
                *p2.add(col) = g20 * &v0 + g21 * &v1 + g22 * &v2 + g23 * &v3;
                *p3.add(col) = g30 * &v0 + g31 * &v1 + g32 * &v2 + g33 * &v3;
            }
        }
    };

    if parallel {
        // SAFETY: `into_par_iter()` distributes distinct `i` values across
        // workers. Because the mapping from `i` to the four touched rows is
        // injective, different workers never access the same row index. The
        // raw pointers derived from `UnsafeSymbolicSlice` are therefore
        // non-aliased across workers.
        (0..loop_limit).into_par_iter().for_each(process_idx);
    } else {
        (0..loop_limit).for_each(process_idx);
    }
}

/// Applies an n-qubit gate to the given bit positions of `matrix`.
///
/// This is the general (unoptimised) code path used when the gate acts on
/// three or more qubits. It iterates over all `2^n` row groups and
/// performs a full matrix-vector multiply for each column.
///
/// # Parallelism
///
/// When the matrix element count exceeds the configured parallel threshold,
/// each worker thread receives its own scratch buffers (`row_ptrs` and
/// `input`) via [`rayon::iter::ParallelIterator::for_each_init`].
pub fn apply_general_gate(matrix: &mut SymbolicMatrix, gate: &SymbolicMatrix, bits: &[usize]) {
    let dim = matrix.nrows();
    let cols = matrix.ncols();
    let num_targets = bits.len();
    let gate_dim = 1usize << num_targets;

    let mut sorted_bits: SmallVec<[usize; 8]> = bits.iter().copied().collect();
    sorted_bits.sort();

    let mut gate_offsets = vec![0usize; gate_dim];
    for (k, offset_ref) in gate_offsets.iter_mut().enumerate() {
        let mut offset = 0usize;
        for (j, &physical_bit) in bits.iter().enumerate() {
            if (k >> j) & 1 == 1 {
                offset |= 1usize << physical_bit;
            }
        }
        *offset_ref = offset;
    }

    let loop_limit = dim >> num_targets;
    let total_ops = dim.saturating_mul(cols);
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let slice = matrix
        .as_slice_mut()
        .expect("Symbolic matrix must be contiguous");
    let unsafe_slice = UnsafeSymbolicSlice::new(slice);

    let process_idx =
        |i: usize, row_ptrs: &mut Vec<*mut SymbolicComplex>, input: &mut Vec<SymbolicComplex>| {
            let mut base = i;
            for &q in &sorted_bits {
                let mask = (1usize << q) - 1;
                let left = (base & !mask) << 1;
                let right = base & mask;
                base = left | right;
            }

            // SAFETY: Each `i` maps to a unique `base`, and `gate_offsets`
            // are fixed non-overlapping bit patterns. Therefore the set of
            // row indices `{base | offset}` for a given `i` is disjoint from
            // the set for any other `i`. No two workers ever write to the
            // same row, so aliasing cannot occur.
            unsafe {
                row_ptrs.clear();
                for offset in gate_offsets.iter().take(gate_dim) {
                    row_ptrs.push(unsafe_slice.row_ptr(base | offset, cols));
                }

                for col in 0..cols {
                    for g in 0..gate_dim {
                        input[g] = (*row_ptrs[g].add(col)).clone();
                    }

                    for row in 0..gate_dim {
                        let mut sum = SymbolicComplex::zero();
                        for col_gate in 0..gate_dim {
                            sum = sum + &gate[[row, col_gate]] * &input[col_gate];
                        }
                        *row_ptrs[row].add(col) = sum;
                    }
                }
            }
        };

    if parallel {
        // SAFETY: `into_par_iter()` partitions the range `0..loop_limit`
        // across workers. Because each `i` produces a disjoint set of row
        // indices, the per-worker `row_ptrs` never alias. `for_each_init`
        // further guarantees that each worker owns its own scratch buffers.
        (0..loop_limit).into_par_iter().for_each_init(
            || {
                (
                    Vec::<*mut SymbolicComplex>::with_capacity(gate_dim),
                    vec![SymbolicComplex::zero(); gate_dim],
                )
            },
            |(row_ptrs, input), i| process_idx(i, row_ptrs, input),
        );
    } else {
        let mut row_ptrs = Vec::<*mut SymbolicComplex>::with_capacity(gate_dim);
        let mut input = vec![SymbolicComplex::zero(); gate_dim];
        for i in 0..loop_limit {
            process_idx(i, &mut row_ptrs, &mut input);
        }
    }
}

/// Dispatches a numeric gate matrix to the appropriate optimised apply
/// function based on the number of target qubits.
pub fn apply_gate_to_matrix_num(
    matrix: &mut SymbolicMatrix,
    gate: &Array2<Complex64>,
    bits: &[usize],
) -> Result<(), CircuitError> {
    validate_target_bits(matrix.nrows(), bits)?;
    let expected_dim = 1usize << bits.len();
    if gate.nrows() != expected_dim || gate.ncols() != expected_dim {
        return Err(CircuitError::QubitCountMismatch {
            expected: gate.nrows().trailing_zeros() as usize,
            actual: bits.len(),
        });
    }
    match classify_numeric_gate(gate) {
        NumericGateKind::Diagonal(diagonal) => {
            apply_numeric_diagonal_gate(matrix, &diagonal, bits);
        }
        NumericGateKind::Permutation(permutation) => {
            apply_numeric_permutation_gate(matrix, &permutation, bits);
        }
        NumericGateKind::Dense => match bits.len() {
            1 => apply_single_qubit_gate_num(matrix, gate, bits[0]),
            2 => apply_two_qubit_gate_num(matrix, gate, bits[0], bits[1]),
            _ => apply_general_gate_num(matrix, gate, bits),
        },
    }
    Ok(())
}

/// Numeric version of [`apply_single_qubit_gate`].
///
/// The gate matrix elements are concrete `Complex64` values, so each
/// multiplication is a simple `Complex64 × SymbolicComplex` instead of a
/// full symbolic expression-tree multiplication.
pub fn apply_single_qubit_gate_num(
    matrix: &mut SymbolicMatrix,
    gate: &Array2<Complex64>,
    bit: usize,
) {
    let dim = matrix.nrows();
    let cols = matrix.ncols();
    let step = 1usize << bit;
    let total_ops = dim.saturating_mul(cols);
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let u00 = gate[[0, 0]];
    let u01 = gate[[0, 1]];
    let u10 = gate[[1, 0]];
    let u11 = gate[[1, 1]];

    let slice = matrix
        .as_slice_mut()
        .expect("Symbolic matrix must be contiguous");
    let unsafe_slice = UnsafeSymbolicSlice::new(slice);

    let process_block = |i: usize| {
        // SAFETY: Same disjoint-row guarantee as the symbolic variant.
        unsafe {
            for j in 0..step {
                let r0_idx = i + j;
                let r1_idx = r0_idx + step;

                let r0_ptr = unsafe_slice.row_ptr(r0_idx, cols);
                let r1_ptr = unsafe_slice.row_ptr(r1_idx, cols);

                for col in 0..cols {
                    let v0 = (*r0_ptr.add(col)).clone();
                    let v1 = (*r1_ptr.add(col)).clone();

                    *r0_ptr.add(col) = u00 * &v0 + u01 * &v1;
                    *r1_ptr.add(col) = u10 * &v0 + u11 * &v1;
                }
            }
        }
    };

    if parallel {
        (0..dim)
            .into_par_iter()
            .step_by(step * 2)
            .for_each(process_block);
    } else {
        (0..dim).step_by(step * 2).for_each(process_block);
    }
}

/// Numeric version of [`apply_two_qubit_gate`].
pub fn apply_two_qubit_gate_num(
    matrix: &mut SymbolicMatrix,
    gate: &Array2<Complex64>,
    b0: usize,
    b1: usize,
) {
    let dim = matrix.nrows();
    let cols = matrix.ncols();
    let (low, high) = if b0 < b1 { (b0, b1) } else { (b1, b0) };
    let mask_low = (1usize << low) - 1;
    let mask_high = (1usize << high) - 1;
    let off0 = 1usize << b0;
    let off1 = 1usize << b1;
    let loop_limit = dim >> 2;
    let total_ops = dim.saturating_mul(cols);
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let g00 = gate[[0, 0]];
    let g01 = gate[[0, 1]];
    let g02 = gate[[0, 2]];
    let g03 = gate[[0, 3]];
    let g10 = gate[[1, 0]];
    let g11 = gate[[1, 1]];
    let g12 = gate[[1, 2]];
    let g13 = gate[[1, 3]];
    let g20 = gate[[2, 0]];
    let g21 = gate[[2, 1]];
    let g22 = gate[[2, 2]];
    let g23 = gate[[2, 3]];
    let g30 = gate[[3, 0]];
    let g31 = gate[[3, 1]];
    let g32 = gate[[3, 2]];
    let g33 = gate[[3, 3]];

    let slice = matrix
        .as_slice_mut()
        .expect("Symbolic matrix must be contiguous");
    let unsafe_slice = UnsafeSymbolicSlice::new(slice);

    let process_idx = |i: usize| {
        let left_part = (i & !mask_low) << 1;
        let right_part = i & mask_low;
        let tmp = left_part | right_part;

        let left_final = (tmp & !mask_high) << 1;
        let right_final = tmp & mask_high;
        let base = left_final | right_final;

        let r0_idx = base;
        let r1_idx = base | off0;
        let r2_idx = base | off1;
        let r3_idx = base | off0 | off1;

        // SAFETY: Same disjoint-row guarantee as the symbolic variant.
        unsafe {
            let p0 = unsafe_slice.row_ptr(r0_idx, cols);
            let p1 = unsafe_slice.row_ptr(r1_idx, cols);
            let p2 = unsafe_slice.row_ptr(r2_idx, cols);
            let p3 = unsafe_slice.row_ptr(r3_idx, cols);

            for col in 0..cols {
                let v0 = (*p0.add(col)).clone();
                let v1 = (*p1.add(col)).clone();
                let v2 = (*p2.add(col)).clone();
                let v3 = (*p3.add(col)).clone();

                *p0.add(col) = g00 * &v0 + g01 * &v1 + g02 * &v2 + g03 * &v3;
                *p1.add(col) = g10 * &v0 + g11 * &v1 + g12 * &v2 + g13 * &v3;
                *p2.add(col) = g20 * &v0 + g21 * &v1 + g22 * &v2 + g23 * &v3;
                *p3.add(col) = g30 * &v0 + g31 * &v1 + g32 * &v2 + g33 * &v3;
            }
        }
    };

    if parallel {
        (0..loop_limit).into_par_iter().for_each(process_idx);
    } else {
        (0..loop_limit).for_each(process_idx);
    }
}

/// Numeric version of [`apply_general_gate`].
pub fn apply_general_gate_num(
    matrix: &mut SymbolicMatrix,
    gate: &Array2<Complex64>,
    bits: &[usize],
) {
    let dim = matrix.nrows();
    let cols = matrix.ncols();
    let num_targets = bits.len();
    let gate_dim = 1usize << num_targets;

    let mut sorted_bits: SmallVec<[usize; 8]> = bits.iter().copied().collect();
    sorted_bits.sort();

    let mut gate_offsets = vec![0usize; gate_dim];
    for (k, offset_ref) in gate_offsets.iter_mut().enumerate() {
        let mut offset = 0usize;
        for (j, &physical_bit) in bits.iter().enumerate() {
            if (k >> j) & 1 == 1 {
                offset |= 1usize << physical_bit;
            }
        }
        *offset_ref = offset;
    }

    let loop_limit = dim >> num_targets;
    let total_ops = dim.saturating_mul(cols);
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let slice = matrix
        .as_slice_mut()
        .expect("Symbolic matrix must be contiguous");
    let unsafe_slice = UnsafeSymbolicSlice::new(slice);

    let process_idx =
        |i: usize, row_ptrs: &mut Vec<*mut SymbolicComplex>, input: &mut Vec<SymbolicComplex>| {
            let mut base = i;
            for &q in &sorted_bits {
                let mask = (1usize << q) - 1;
                let left = (base & !mask) << 1;
                let right = base & mask;
                base = left | right;
            }

            // SAFETY: Same disjoint-row guarantee as the symbolic variant.
            unsafe {
                row_ptrs.clear();
                for offset in gate_offsets.iter().take(gate_dim) {
                    row_ptrs.push(unsafe_slice.row_ptr(base | offset, cols));
                }

                for col in 0..cols {
                    for g in 0..gate_dim {
                        input[g] = (*row_ptrs[g].add(col)).clone();
                    }

                    for row in 0..gate_dim {
                        let mut sum = SymbolicComplex::zero();
                        for col_gate in 0..gate_dim {
                            sum = sum + gate[[row, col_gate]] * &input[col_gate];
                        }
                        *row_ptrs[row].add(col) = sum;
                    }
                }
            }
        };

    if parallel {
        (0..loop_limit).into_par_iter().for_each_init(
            || {
                (
                    Vec::<*mut SymbolicComplex>::with_capacity(gate_dim),
                    vec![SymbolicComplex::zero(); gate_dim],
                )
            },
            |(row_ptrs, input), i| process_idx(i, row_ptrs, input),
        );
    } else {
        let mut row_ptrs = Vec::<*mut SymbolicComplex>::with_capacity(gate_dim);
        let mut input = vec![SymbolicComplex::zero(); gate_dim];
        for i in 0..loop_limit {
            process_idx(i, &mut row_ptrs, &mut input);
        }
    }
}

fn validate_target_bits(matrix_dim: usize, bits: &[usize]) -> Result<(), CircuitError> {
    if !matrix_dim.is_power_of_two() {
        return Err(CircuitError::InvalidOperation(format!(
            "matrix dimension {matrix_dim} is not a power of two"
        )));
    }

    let num_qubits = matrix_dim.trailing_zeros() as usize;
    let mut seen = HashSet::with_capacity(bits.len());
    for &bit in bits {
        if bit >= num_qubits {
            return Err(CircuitError::InvalidOperation(format!(
                "gate bit {bit} is out of range for {num_qubits} qubits"
            )));
        }
        if !seen.insert(bit) {
            return Err(CircuitError::DuplicateQubits);
        }
    }

    Ok(())
}

/// Validates that the number of supplied `params` matches the gate's
/// declared [`StandardGate::num_params`].
///
/// Returns [`CircuitError::ParameterCountMismatch`] on failure.
fn validate_params(gate: StandardGate, params: &[Parameter]) -> Result<(), CircuitError> {
    let expected = gate.num_params();
    if params.len() != expected {
        return Err(CircuitError::ParameterCountMismatch {
            expected,
            actual: params.len(),
        });
    }
    Ok(())
}

/// Converts a numerical complex matrix into a [`SymbolicMatrix`] by wrapping
/// each element with [`SymbolicComplex::from_complex`].
fn symbolic_matrix_from_numeric(matrix: &Array2<Complex64>) -> SymbolicMatrix {
    matrix.mapv(SymbolicComplex::from_complex)
}

/// Returns `θ / 2` as a new [`Parameter`] expression.
fn half(theta: &Parameter) -> Parameter {
    theta.clone() / 2.0
}

/// Returns `cos(θ/2)` as a purely real [`SymbolicComplex`].
///
/// This is the diagonal-element pattern shared by `RX`, `RY`, `RXX`, etc.
fn cos_half(theta: &Parameter) -> SymbolicComplex {
    SymbolicComplex::from_real(half(theta).cos())
}

/// Returns `sin(θ/2)` as a [`Parameter`].
///
/// The caller decides whether to wrap this as a real or imaginary component.
fn sin_half(theta: &Parameter) -> Parameter {
    half(theta).sin()
}

/// Returns `−i · value`, i.e. a purely imaginary [`SymbolicComplex`] with
/// negative imaginary coefficient.
///
/// Used for the off-diagonal elements of `RX`, `RXX`, `CRX`, etc.
fn neg_i_times(value: Parameter) -> SymbolicComplex {
    SymbolicComplex::new(0.0, -1.0 * value)
}

/// Returns `i · value`, i.e. a purely imaginary [`SymbolicComplex`] with
/// positive imaginary coefficient.
///
/// Used for the off-diagonal elements of `RYY`, `RZX`, etc.
fn i_times(value: Parameter) -> SymbolicComplex {
    SymbolicComplex::new(0.0, value)
}

/// Returns `exp(−i·θ)` as `cos(θ) − i·sin(θ)`.
///
/// Used for the diagonal phase factors of `RZ`, `RZZ`, `CRZ`, etc.
fn exp_neg_i(theta: Parameter) -> SymbolicComplex {
    SymbolicComplex::exp_i(-theta)
}

fn classify_symbolic_gate(gate: &SymbolicMatrix) -> SymbolicGateKind {
    let dim = gate.nrows();
    if (0..dim).all(|row| (0..dim).all(|col| row == col || gate[[row, col]].is_zero_exact())) {
        return SymbolicGateKind::Diagonal((0..dim).map(|idx| gate[[idx, idx]].clone()).collect());
    }

    let mut seen_cols = vec![false; dim];
    let mut permutation = Vec::with_capacity(dim);
    for row in 0..dim {
        let mut nonzero = None;
        for col in 0..dim {
            if !gate[[row, col]].is_zero_exact() {
                if nonzero.is_some() || seen_cols[col] {
                    return SymbolicGateKind::Dense;
                }
                nonzero = Some((col, gate[[row, col]].clone()));
            }
        }
        let Some((col, factor)) = nonzero else {
            return SymbolicGateKind::Dense;
        };
        seen_cols[col] = true;
        permutation.push((col, factor));
    }

    SymbolicGateKind::Permutation(permutation)
}

fn classify_numeric_gate(gate: &Array2<Complex64>) -> NumericGateKind {
    let dim = gate.nrows();
    if (0..dim).all(|row| (0..dim).all(|col| row == col || numeric_is_zero(gate[[row, col]]))) {
        return NumericGateKind::Diagonal((0..dim).map(|idx| gate[[idx, idx]]).collect());
    }

    let mut seen_cols = vec![false; dim];
    let mut permutation = Vec::with_capacity(dim);
    for row in 0..dim {
        let mut nonzero = None;
        for col in 0..dim {
            let factor = gate[[row, col]];
            if !numeric_is_zero(factor) {
                if nonzero.is_some() || seen_cols[col] {
                    return NumericGateKind::Dense;
                }
                nonzero = Some((col, factor));
            }
        }
        let Some((col, factor)) = nonzero else {
            return NumericGateKind::Dense;
        };
        seen_cols[col] = true;
        permutation.push((col, factor));
    }

    NumericGateKind::Permutation(permutation)
}

/// Resolves a slice of [`CircuitParam`] values into concrete [`Parameter`]
/// expressions.
///
/// - [`CircuitParam::Fixed`] is converted directly from its `f64` value.
/// - [`CircuitParam::Index`] is looked up in the circuit's parameter table.
///
/// # Errors
///
/// Returns [`CircuitError::InvalidParameterIndex`] if an index is out of
/// bounds for the circuit's parameter set.
fn resolve_params(
    circuit: &Circuit,
    params: &[CircuitParam],
) -> Result<Vec<Parameter>, CircuitError> {
    params
        .iter()
        .map(|param| match param {
            CircuitParam::Fixed(value) => Ok(Parameter::from(*value)),
            CircuitParam::Index(idx) => circuit
                .parameters()
                .get_index(*idx as usize)
                .cloned()
                .ok_or(CircuitError::InvalidParameterIndex(*idx)),
        })
        .collect()
}

#[cfg(test)]
#[path = "./gate_test.rs"]
mod gate_test;
