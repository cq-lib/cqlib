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

//! # Circuit to Matrix Conversion
//!
//! This module provides functionality to simulate a quantum circuit and compute its final unitary matrix representation.
//!
//! ## Core Logic
//!
//! The simulation assumes a state vector simulation model where the full unitary matrix $U$ of the circuit is computed
//! by sequentially applying gate matrices to an initial Identity matrix.
//!
//! $$ U = U_n \cdot U_{n-1} \cdots U_1 \cdot I $$
//!
//! The returned matrix includes the circuit global phase, i.e. a circuit with
//! global phase `θ` returns `e^{iθ} U`.
//!
//! ## Memory Layout & Convention
//!
//! - **State Vector Ordering**: Little-Endian (similar to Qiskit). Qubit 0 corresponds to the Least Significant Bit (LSB).
//!   State $|q_{n-1} \dots q_1 q_0\rangle$.
//! - **Parallelization**: Large matrix multiplications (large state spaces) are automatically parallelized using `rayon`.

use crate::circuit::Circuit;
use crate::circuit::circuit_param::CircuitParam;
use crate::circuit::error::CircuitError;
use crate::circuit::gate::Instruction;
#[cfg(any(test, debug_assertions))]
use crate::circuit::{ParameterValue, Qubit, ValueOperation};
use ndarray::Array2;
use ndarray::parallel::prelude::*;
use num_complex::Complex64;
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

// Matrix element scale threshold for parallelizing row-block updates.
// This keeps small circuits sequential to avoid rayon scheduling overhead.
const PARALLEL_THRESHOLD_OPS: usize = 1 << 20;

/// Computes the unitary matrix representation of a quantum circuit.
///
/// This function applies each operation to an initially identity matrix. The
/// result is a $2^N \times 2^N$ dense matrix, where $N$ is the number of
/// circuit qubits, and includes the circuit's global phase.
///
/// The first entry in `qubits_order` is the least-significant basis-state bit.
/// When no order is supplied, qubits are sorted by numeric identifier. Matrix
/// storage grows as $4^N$, so this function is intended for small circuits.
///
/// # Arguments
///
/// * `circuit` - The quantum circuit to simulate.
/// * `qubits_order` - Optional complete ordering of the circuit's numeric qubit
///   identifiers, from least-significant to most-significant basis-state bit.
///
/// # Errors
///
/// Returns an error if `qubits_order` is not an exact permutation of the
/// circuit qubits, the circuit contains unresolved parameters or non-unitary
/// operations, a gate definition is invalid, or the dense matrix dimensions
/// cannot be represented.
///
/// # Example
///
/// ```rust
/// use cqlib_core::circuit::circuit_impl::Circuit;
/// use cqlib_core::circuit::Qubit;
/// use cqlib_core::circuit::circuit_to_matrix;
///
/// let mut circuit = Circuit::new(2);
/// circuit.h(Qubit::new(0)).unwrap();
/// circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
///
/// let matrix = circuit_to_matrix(&circuit, None).unwrap();
/// // matrix is now the 4x4 unitary of the Bell state preparation.
/// ```
pub fn circuit_to_matrix(
    circuit: &Circuit,
    qubits_order: Option<&[usize]>,
) -> Result<Array2<Complex64>, CircuitError> {
    let circuit_qubits: Vec<usize> = circuit.qubits().iter().map(|q| q.index()).collect();
    let num_qubits = circuit_qubits.len();
    let dim = 1usize.checked_shl(num_qubits as u32).ok_or_else(|| {
        CircuitError::InvalidOperation(format!(
            "cannot build matrix for {num_qubits} qubits: dimension overflows usize"
        ))
    })?;
    let _matrix_len = dim.checked_mul(dim).ok_or_else(|| {
        CircuitError::InvalidOperation(format!(
            "cannot build matrix for {num_qubits} qubits: matrix element count overflows usize"
        ))
    })?;

    let target_order: Vec<usize> = match qubits_order {
        Some(order) => {
            let circuit_set: HashSet<usize> = circuit_qubits.iter().cloned().collect();
            let order_set: HashSet<usize> = order.iter().cloned().collect();
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

    // Map logical qubit index to physical bit position (0 to N-1).
    // Little-Endian mapping: first qubit in target_order corresponds to LSB (bit 0)
    // q0 -> bit 0, q1 -> bit 1, ...
    // This matches Qiskit's convention.
    let qubit_bit_map: HashMap<usize, usize> = target_order
        .iter()
        .enumerate()
        .map(|(i, &q_id)| (q_id, i))
        .collect();

    // Start with Identity
    let mut matrix: Array2<Complex64> = Array2::eye(dim);

    // Iterate over operations and update the matrix in-place
    for op in circuit.operations().iter() {
        // Map operation qubits to physical bit positions
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

        let params: SmallVec<[f64; 4]> = op
            .params
            .iter()
            .enumerate()
            .map(|(param_index, p)| match p {
                CircuitParam::Fixed(value) => {
                    if !value.is_finite() {
                        return Err(CircuitError::InvalidParameterValue(param_index, *value));
                    }
                    Ok(*value)
                }
                CircuitParam::Index(_) => Err(CircuitError::SymbolicParameterError),
            })
            .collect::<Result<_, _>>()?;

        match &op.instruction {
            Instruction::Standard(std_gate) => {
                let gate_matrix = std_gate
                    .matrix(params.as_slice())
                    .map_err(|_| CircuitError::NoMatrixRepresentation)?;
                // StandardGate matrices are Big-Endian (Controls/First Args are MSB).
                // System is Little-Endian. Reverse bits to align.
                let reversed_bits: SmallVec<[usize; 3]> = bits.iter().cloned().rev().collect();
                apply_gate_to_matrix(&mut matrix, gate_matrix.as_ref(), &reversed_bits)?;
            }
            Instruction::McGate(mc_gate) => {
                let gate_matrix = mc_gate
                    .matrix(params.as_slice())
                    .map_err(|_| CircuitError::NoMatrixRepresentation)?;
                // McGate matrices are Big-Endian (Controls MSB).
                let reversed_bits: SmallVec<[usize; 3]> = bits.iter().cloned().rev().collect();
                apply_gate_to_matrix(&mut matrix, gate_matrix.as_ref(), &reversed_bits)?;
            }
            Instruction::UnitaryGate(u_gate) => {
                if u_gate.circuit().is_some()
                    && u_gate.matrix().is_none()
                    && u_gate.symbolic_matrix().is_none()
                {
                    let gate_matrix = u_gate.matrix_for_params(params.as_slice())?;
                    // Sub-circuit matrices are already built in the system's Little-Endian basis,
                    // so we apply them directly without reversing bits.
                    apply_gate_to_matrix(&mut matrix, gate_matrix.as_ref(), &bits)?;
                } else {
                    let gate_matrix = u_gate.matrix_for_params(params.as_slice())?;
                    // UnitaryGate matrices are assumed to follow standard Big-Endian convention.
                    // System is Little-Endian. Reverse bits to align.
                    let reversed_bits: SmallVec<[usize; 3]> = bits.iter().cloned().rev().collect();
                    apply_gate_to_matrix(&mut matrix, gate_matrix.as_ref(), &reversed_bits)?;
                }
            }
            Instruction::CircuitGate(circuit_gate) => {
                let symbols = circuit_gate.signature_params();
                let expected = symbols.len();
                let actual = params.len();
                if actual != expected {
                    return Err(CircuitError::ParameterCountMismatch { expected, actual });
                }
                let mut bindings = HashMap::new();
                for (sym, val) in symbols.iter().zip(params.iter()) {
                    bindings.insert(sym.as_str(), *val);
                }
                let sub_circuit = circuit_gate
                    .circuit
                    .circuit
                    .assign_parameters(&Some(bindings))
                    .map_err(|_| CircuitError::SymbolicParameterError)?;
                let sub_matrix = circuit_to_matrix(&sub_circuit, None)?;
                apply_gate_to_matrix(&mut matrix, &sub_matrix, &bits)?;
            }
            Instruction::ClassicalControl(_) => {
                return Err(CircuitError::InvalidOperation(
                    "control-flow operations do not have an unconditional matrix representation"
                        .to_string(),
                ));
            }
            Instruction::ClassicalData(_) => {
                return Err(CircuitError::NoMatrixRepresentation);
            }
            Instruction::Directive(directive) => match directive {
                crate::circuit::Directive::Barrier => continue,
                crate::circuit::Directive::Measure | crate::circuit::Directive::Reset => {
                    return Err(CircuitError::NoMatrixRepresentation);
                }
            },
            Instruction::Delay => continue,
        }
    }

    let global_phase = circuit
        .global_phase()
        .evaluate(&None)
        .map_err(|_| CircuitError::SymbolicParameterError)?;
    if global_phase != 0.0 {
        let phase = Complex64::from_polar(1.0, global_phase);
        matrix.mapv_inplace(|value| phase * value);
    }

    Ok(matrix)
}

/// Builds a dense unitary directly from resolved value operations.
///
/// This internal path avoids constructing and validating a temporary `Circuit`
/// in compiler passes that already hold finite, standard-gate operations.
#[cfg(any(test, debug_assertions))]
pub(crate) fn value_operations_to_matrix(
    qubits: &[Qubit],
    operations: &[ValueOperation],
    global_phase: f64,
) -> Result<Array2<Complex64>, CircuitError> {
    let num_qubits = qubits.len();
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
    let mut qubit_bit_map = HashMap::with_capacity(num_qubits);
    for (bit, qubit) in qubits.iter().copied().enumerate() {
        if qubit_bit_map.insert(qubit.index(), bit).is_some() {
            return Err(CircuitError::InvalidOperation(
                "matrix qubit order contains duplicates".to_string(),
            ));
        }
    }

    let mut matrix = Array2::eye(dim);
    for operation in operations {
        let Some(Instruction::Standard(gate)) = operation.instruction.as_instruction() else {
            return Err(CircuitError::NoMatrixRepresentation);
        };
        let bits = operation
            .qubits
            .iter()
            .map(|qubit| {
                qubit_bit_map
                    .get(&qubit.index())
                    .copied()
                    .ok_or(CircuitError::QubitNotFound(qubit.id()))
            })
            .collect::<Result<SmallVec<[usize; 3]>, _>>()?;
        let params = operation
            .params
            .iter()
            .enumerate()
            .map(|(index, param)| match param {
                ParameterValue::Fixed(value) if value.is_finite() => Ok(*value),
                ParameterValue::Fixed(value) => {
                    Err(CircuitError::InvalidParameterValue(index, *value))
                }
                ParameterValue::Param(_) => Err(CircuitError::SymbolicParameterError),
            })
            .collect::<Result<SmallVec<[f64; 4]>, _>>()?;
        let gate_matrix = gate
            .matrix(params.as_slice())
            .map_err(|_| CircuitError::NoMatrixRepresentation)?;
        let reversed_bits = bits.iter().copied().rev().collect::<SmallVec<[usize; 3]>>();
        apply_gate_to_matrix(&mut matrix, gate_matrix.as_ref(), &reversed_bits)?;
    }

    if global_phase != 0.0 {
        if !global_phase.is_finite() {
            return Err(CircuitError::InvalidParameterValue(0, global_phase));
        }
        let phase = Complex64::from_polar(1.0, global_phase);
        matrix.mapv_inplace(|value| phase * value);
    }
    Ok(matrix)
}

/// Applies a gate (given as a matrix) to the full state matrix.
pub(crate) fn apply_gate_to_matrix(
    matrix: &mut Array2<Complex64>,
    gate_matrix: &Array2<Complex64>,
    bits: &[usize],
) -> Result<(), CircuitError> {
    validate_target_bits(matrix.nrows(), bits)?;
    let expected_dim = 1usize << bits.len();
    if gate_matrix.nrows() != expected_dim || gate_matrix.ncols() != expected_dim {
        return Err(CircuitError::QubitCountMismatch {
            expected: gate_matrix.nrows().trailing_zeros() as usize,
            actual: bits.len(),
        });
    }
    match bits.len() {
        1 => apply_single_qubit_gate(matrix, gate_matrix, bits[0])?,
        2 => apply_two_qubit_gate(matrix, gate_matrix, bits[0], bits[1])?,
        _ => apply_general_gate(matrix, gate_matrix, bits)?,
    }
    Ok(())
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

// Helper struct for raw pointer access to split mutable borrow
struct UnsafeSlice<'a> {
    ptr: *mut Complex64,
    _marker: PhantomData<&'a mut [Complex64]>,
}
unsafe impl<'a> Sync for UnsafeSlice<'a> {}
unsafe impl<'a> Send for UnsafeSlice<'a> {}

impl<'a> UnsafeSlice<'a> {
    fn new(slice: &'a mut [Complex64]) -> Self {
        Self {
            ptr: slice.as_mut_ptr(),
            _marker: PhantomData,
        }
    }

    // SAFETY: Caller must ensure that concurrent accesses do not alias.
    unsafe fn row_ptr(&self, row_idx: usize, cols: usize) -> *mut Complex64 {
        unsafe { self.ptr.add(row_idx * cols) }
    }
}

fn apply_single_qubit_gate(
    matrix: &mut Array2<Complex64>,
    gate: &Array2<Complex64>,
    bit: usize,
) -> Result<(), CircuitError> {
    let dim = matrix.nrows(); // 2^N
    let cols = matrix.ncols(); // 2^N, same as dim

    let u00 = gate[[0, 0]];
    let u01 = gate[[0, 1]];
    let u10 = gate[[1, 0]];
    let u11 = gate[[1, 1]];

    let step = 1 << bit;
    let total_ops = dim * cols;
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let slice = matrix.as_slice_mut().ok_or_else(|| {
        CircuitError::InvalidOperation(
            "target matrix must use contiguous standard layout".to_string(),
        )
    })?;
    let unsafe_slice = UnsafeSlice::new(slice);

    let process_block = |i: usize| unsafe {
        for j in 0..step {
            let r0_idx = i + j;
            let r1_idx = r0_idx + step;

            let r0_ptr = unsafe_slice.row_ptr(r0_idx, cols);
            let r1_ptr = unsafe_slice.row_ptr(r1_idx, cols);

            for k in 0..cols {
                let v0 = *r0_ptr.add(k);
                let v1 = *r1_ptr.add(k);

                *r0_ptr.add(k) = u00 * v0 + u01 * v1;
                *r1_ptr.add(k) = u10 * v0 + u11 * v1;
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
    Ok(())
}

fn apply_two_qubit_gate(
    matrix: &mut Array2<Complex64>,
    gate: &Array2<Complex64>,
    b0: usize,
    b1: usize,
) -> Result<(), CircuitError> {
    let dim = matrix.nrows();
    let cols = matrix.ncols();

    let (low, high) = if b0 < b1 { (b0, b1) } else { (b1, b0) };
    let mask_low = (1 << low) - 1;
    let mask_high = (1 << high) - 1;
    let off0 = 1 << b0;
    let off1 = 1 << b1;

    let slice = matrix.as_slice_mut().ok_or_else(|| {
        CircuitError::InvalidOperation(
            "target matrix must use contiguous standard layout".to_string(),
        )
    })?;
    let unsafe_slice = UnsafeSlice::new(slice);

    let loop_limit = dim >> 2;
    let total_ops = dim * cols;
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let process_idx = |i: usize| {
        let left_part = (i & !mask_low) << 1;
        let right_part = i & mask_low;
        let tmp = left_part | right_part;

        let left_final = (tmp & !mask_high) << 1;
        let right_final = tmp & mask_high;

        let base = left_final | right_final;

        // Correct Little-Endian Mapping:
        // bits[0] (b0) corresponds to LSB (Gate Index Bit 0) -> off0
        // bits[1] (b1) corresponds to MSB (Gate Index Bit 1) -> off1
        // Gate Indices:
        // 00 -> base
        // 01 -> base | off0
        // 10 -> base | off1
        // 11 -> base | off0 | off1

        let r0_idx = base;
        let r1_idx = base | off0;
        let r2_idx = base | off1;
        let r3_idx = base | off0 | off1;

        unsafe {
            let p0 = unsafe_slice.row_ptr(r0_idx, cols);
            let p1 = unsafe_slice.row_ptr(r1_idx, cols);
            let p2 = unsafe_slice.row_ptr(r2_idx, cols);
            let p3 = unsafe_slice.row_ptr(r3_idx, cols);

            for k in 0..cols {
                let v0 = *p0.add(k);
                let v1 = *p1.add(k);
                let v2 = *p2.add(k);
                let v3 = *p3.add(k);

                let mut sum0 = Complex64::default();
                let mut sum1 = Complex64::default();
                let mut sum2 = Complex64::default();
                let mut sum3 = Complex64::default();

                sum0 +=
                    gate[[0, 0]] * v0 + gate[[0, 1]] * v1 + gate[[0, 2]] * v2 + gate[[0, 3]] * v3;
                sum1 +=
                    gate[[1, 0]] * v0 + gate[[1, 1]] * v1 + gate[[1, 2]] * v2 + gate[[1, 3]] * v3;
                sum2 +=
                    gate[[2, 0]] * v0 + gate[[2, 1]] * v1 + gate[[2, 2]] * v2 + gate[[2, 3]] * v3;
                sum3 +=
                    gate[[3, 0]] * v0 + gate[[3, 1]] * v1 + gate[[3, 2]] * v2 + gate[[3, 3]] * v3;

                *p0.add(k) = sum0;
                *p1.add(k) = sum1;
                *p2.add(k) = sum2;
                *p3.add(k) = sum3;
            }
        }
    };

    if parallel {
        (0..loop_limit).into_par_iter().for_each(process_idx);
    } else {
        (0..loop_limit).for_each(process_idx);
    }
    Ok(())
}

fn apply_general_gate(
    matrix: &mut Array2<Complex64>,
    gate: &Array2<Complex64>,
    bits: &[usize],
) -> Result<(), CircuitError> {
    let dim = matrix.nrows();
    let cols = matrix.ncols();
    let num_targets = bits.len();
    let gate_dim = 1 << num_targets;

    let mut sorted_bits: SmallVec<[usize; 8]> = bits.iter().copied().collect();
    sorted_bits.sort();

    let mut gate_offsets = vec![0usize; gate_dim];
    for (k, gate_offset_ref) in gate_offsets.iter_mut().enumerate().take(gate_dim) {
        let mut offset = 0;
        for (j, &phy_bit) in bits.iter().enumerate() {
            // Correct Little-Endian Mapping:
            // bits[j] corresponds to the j-th bit of the gate index k.
            if (k >> j) & 1 == 1 {
                offset |= 1 << phy_bit;
            }
        }
        *gate_offset_ref = offset;
    }

    let slice = matrix.as_slice_mut().ok_or_else(|| {
        CircuitError::InvalidOperation(
            "target matrix must use contiguous standard layout".to_string(),
        )
    })?;
    let unsafe_slice = UnsafeSlice::new(slice);

    let loop_limit = dim >> num_targets;
    let total_ops = dim * cols;
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let process_idx =
        |i: usize, row_ptrs: &mut Vec<*mut Complex64>, input_vec: &mut Vec<Complex64>| {
            let mut base = i;
            for &q in &sorted_bits {
                let mask = (1 << q) - 1;
                let left = (base & !mask) << 1;
                let right = base & mask;
                base = left | right;
            }

            unsafe {
                row_ptrs.clear();
                for item in gate_offsets.iter().take(gate_dim) {
                    row_ptrs.push(unsafe_slice.row_ptr(base | item, cols));
                }

                for k_col in 0..cols {
                    for g in 0..gate_dim {
                        input_vec[g] = *row_ptrs[g].add(k_col);
                    }

                    for r in 0..gate_dim {
                        let mut sum = Complex64::default();
                        for c in 0..gate_dim {
                            sum += gate[[r, c]] * input_vec[c];
                        }
                        *row_ptrs[r].add(k_col) = sum;
                    }
                }
            }
        };

    if parallel {
        (0..loop_limit).into_par_iter().for_each_init(
            || {
                (
                    Vec::<*mut Complex64>::with_capacity(gate_dim),
                    vec![Complex64::default(); gate_dim],
                )
            },
            |(row_ptrs, input_vec), i| process_idx(i, row_ptrs, input_vec),
        );
    } else {
        let mut row_ptrs = Vec::<*mut Complex64>::with_capacity(gate_dim);
        let mut input_vec = vec![Complex64::default(); gate_dim];
        for i in 0..loop_limit {
            process_idx(i, &mut row_ptrs, &mut input_vec);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "./circuit_to_matrix_test.rs"]
mod circuit_to_matrix_test;
