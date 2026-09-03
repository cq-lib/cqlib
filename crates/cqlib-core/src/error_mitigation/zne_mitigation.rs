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

use crate::circuit::{Circuit, CircuitError, Instruction, Operation, Parameter};
use crate::error_mitigation::ErrorMitigationError;
use crate::error_mitigation::Estimator;
use crate::qis::Hamiltonian;
use std::collections::HashSet;

/// Extrapolation methods supported by [`ZNEMitigation::extrapolate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtrapolateMethod {
    Polynomial,
    Exponential,
}

/// Zero-noise extrapolation (ZNE) mitigation helper.
///
/// This mirrors the Python `ZNEMitigation` data model and currently implements
/// only circuit folding.
///
/// # Example
///
/// ```rust
/// use cqlib_core::circuit::gate::{Instruction, StandardGate};
/// use cqlib_core::circuit::{Circuit, Qubit};
/// use cqlib_core::error_mitigation::ZNEMitigation;
///
/// let q0 = Qubit::new(0);
/// let q1 = Qubit::new(1);
///
/// let mut circuit = Circuit::new(2);
/// circuit.h(q0).unwrap();
/// circuit.cx(q0, q1).unwrap();
///
/// // Build ZNE with fold levels [0, 1, 2], noise factors [1, 3, 5].
/// let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]);
///
/// // Global folding for each level.
/// let folded_all = zne.fold_circuits(None).unwrap();
/// assert_eq!(folded_all.len(), 3);
///
/// // Selective folding for H only.
/// let folded_h_only = zne
///     .fold_circuits(Some(&[Instruction::Standard(StandardGate::H)]))
///     .unwrap();
/// assert_eq!(folded_h_only.len(), 3);
/// ```
#[derive(Debug, Clone)]
pub struct ZNEMitigation {
    circuit: Circuit,
    fold_levels: Vec<i32>,
    noise_factors: Vec<i32>,
}

impl ZNEMitigation {
    /// Creates a new ZNE mitigation helper.
    ///
    /// `noise_factors` follow the Python implementation: `2 * level + 1`.
    pub fn new(circuit: Circuit, fold_levels: Vec<i32>) -> Self {
        let noise_factors = fold_levels.iter().map(|level| 2 * level + 1).collect();
        Self {
            circuit,
            fold_levels,
            noise_factors,
        }
    }

    /// Returns the original (unfolded) circuit.
    pub fn circuit(&self) -> &Circuit {
        &self.circuit
    }

    /// Returns configured fold levels.
    pub fn fold_levels(&self) -> &[i32] {
        &self.fold_levels
    }

    /// Returns noise factors corresponding to fold levels.
    ///
    /// Each factor is `2 * level + 1` due to unitary folding.
    pub fn noise_factors(&self) -> &[i32] {
        &self.noise_factors
    }

    /// Fold the circuit for each configured level using unitary folding.
    ///
    /// If `gate_set` is `None`, this performs global folding:
    /// `U -> U (U^† U)^level`.
    ///
    /// If `gate_set` is provided, only operations whose instruction name matches
    /// one of the instruction names in `gate_set` are folded.
    pub fn fold_circuits(
        &self,
        gate_set: Option<&[Instruction]>,
    ) -> Result<Vec<Circuit>, CircuitError> {
        self.fold_levels
            .iter()
            .map(|level| self.fold_to_level(*level, gate_set))
            .collect()
    }

    /// Run the error-mitigation sequence and return expectation-value estimates.
    ///
    /// This method folds the circuit at each configured level and computes one
    /// expectation value per folded circuit.
    ///
    /// - `gate_set`: optional selective gate set for folding. `None` means global folding.
    /// - `hamiltonian`: a `qis::Hamiltonian` describing the observable to estimate.
    /// - `estimator`: a shared estimator that receives the folded circuit,
    ///   `Some(hamiltonian)`, and an optional shot count.
    ///   `run_em_sequence` currently passes `None` for the shot count.
    pub fn run_em_sequence(
        &self,
        gate_set: Option<&[Instruction]>,
        hamiltonian: &Hamiltonian,
        estimator: &Estimator<'_>,
    ) -> Result<Vec<f64>, ErrorMitigationError> {
        self.run_em_sequence_with_shots(gate_set, hamiltonian, None, estimator)
    }

    /// Run the error-mitigation sequence with an optional shot count.
    pub fn run_em_sequence_with_shots(
        &self,
        gate_set: Option<&[Instruction]>,
        hamiltonian: &Hamiltonian,
        shots: Option<usize>,
        estimator: &Estimator<'_>,
    ) -> Result<Vec<f64>, ErrorMitigationError> {
        if hamiltonian.num_qubits != self.circuit.width() {
            return Err(ErrorMitigationError::HamiltonianQubitCountMismatch {
                expected: self.circuit.width(),
                actual: hamiltonian.num_qubits,
            });
        }

        let mut hexp_seq = Vec::new();
        for level in self.fold_levels() {
            let circuit = self.fold_to_level(*level, gate_set)?;
            let (expectation, _variance) = estimator(&circuit, Some(hamiltonian), shots);
            hexp_seq.push(expectation);
        }
        Ok(hexp_seq)
    }

    /// Unified extrapolation API.
    ///
    /// - `method = ExtrapolateMethod::Polynomial`: uses `degree` and delegates
    ///   to [`ZNEMitigation::poly_extrapolate`].
    /// - `method = ExtrapolateMethod::Exponential`: ignores `degree` and
    ///   delegates to [`ZNEMitigation::exp_extrapolate`].
    pub fn extrapolate(
        &self,
        noisy_results: &[f64],
        method: ExtrapolateMethod,
        degree: usize,
    ) -> Result<f64, ErrorMitigationError> {
        match method {
            ExtrapolateMethod::Polynomial => self.poly_extrapolate(noisy_results, degree),
            ExtrapolateMethod::Exponential => self.exp_extrapolate(noisy_results),
        }
    }

    /// Given the noisy results, extrapolate the expectation value using a polynomial fit.
    ///
    /// - `noisy_results`: the noisy results to extrapolate.
    /// - `degree`: the degree of the polynomial to use for extrapolation.
    ///
    /// Returns the extrapolated expectation value.
    pub fn poly_extrapolate(
        &self,
        noisy_results: &[f64],
        degree: usize,
    ) -> Result<f64, ErrorMitigationError> {
        let n = self.noise_factors.len();
        self.validate_noisy_results(noisy_results)?;
        if degree >= n {
            return Err(ErrorMitigationError::InvalidPolynomialDegree {
                degree,
                num_points: n,
            });
        }

        let d = degree + 1;
        let x: Vec<f64> = self.noise_factors.iter().map(|&v| v as f64).collect();
        let y = noisy_results;

        // Build normal equations: (V^T V) c = V^T y, where
        // V[i, j] = x_i^j and c stores coefficients in ascending order.
        let mut a = vec![vec![0.0; d]; d];
        let mut b = vec![0.0; d];
        for row in 0..n {
            let mut x_pows = vec![1.0; d];
            for j in 1..d {
                x_pows[j] = x_pows[j - 1] * x[row];
            }

            for j in 0..d {
                b[j] += y[row] * x_pows[j];
                for k in 0..d {
                    a[j][k] += x_pows[j] * x_pows[k];
                }
            }
        }

        let coeffs = Self::solve_linear_system(a, b)?;
        Ok(coeffs[0])
    }

    /// Given the noisy results, extrapolate the expectation value using an
    /// exponential-decay model:
    ///
    /// `y(x) = A * exp(-x / tau)`.
    ///
    /// The fit is performed in log-space by linear regression on:
    ///
    /// `ln(y) = ln(A) + m * x`, where `m = -1 / tau`.
    ///
    /// Returns `A`, which is the extrapolated value at `x = 0`.
    pub fn exp_extrapolate(&self, noisy_results: &[f64]) -> Result<f64, ErrorMitigationError> {
        let n = self.noise_factors.len();
        self.validate_noisy_results(noisy_results)?;
        if !noisy_results.iter().all(|&v| v > 0.0) {
            return Err(ErrorMitigationError::NonPositiveNoisyResults);
        }

        let x: Vec<f64> = self.noise_factors.iter().map(|&v| v as f64).collect();
        let y_log: Vec<f64> = noisy_results.iter().map(|&v| v.ln()).collect();

        let n_f = n as f64;
        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = y_log.iter().sum();
        let sum_xx: f64 = x.iter().map(|v| v * v).sum();
        let sum_xy: f64 = x.iter().zip(y_log.iter()).map(|(xi, yi)| xi * yi).sum();

        let denom = n_f * sum_xx - sum_x * sum_x;
        if denom.abs() <= 1e-14 {
            return Err(ErrorMitigationError::SingularExponentialFit);
        }

        let slope = (n_f * sum_xy - sum_x * sum_y) / denom;
        let intercept = (sum_y - slope * sum_x) / n_f;
        Ok(intercept.exp())
    }

    fn solve_linear_system(
        mut a: Vec<Vec<f64>>,
        mut b: Vec<f64>,
    ) -> Result<Vec<f64>, ErrorMitigationError> {
        let n = a.len();
        debug_assert!(n > 0, "Coefficient matrix must not be empty.");
        debug_assert_eq!(b.len(), n, "Right-hand side length must match matrix size.");
        for row in &a {
            debug_assert_eq!(row.len(), n, "Coefficient matrix must be square.");
        }

        let eps = 1e-14_f64;

        for i in 0..n {
            // Partial pivoting for numerical stability.
            let mut pivot_row = i;
            let mut pivot_abs = a[i][i].abs();

            #[allow(clippy::needless_range_loop)]
            for r in (i + 1)..n {
                let cand = a[r][i].abs();
                if cand > pivot_abs {
                    pivot_abs = cand;
                    pivot_row = r;
                }
            }

            if pivot_abs <= eps {
                return Err(ErrorMitigationError::SingularPolynomialFit);
            }

            if pivot_row != i {
                a.swap(i, pivot_row);
                b.swap(i, pivot_row);
            }

            for r in (i + 1)..n {
                let factor = a[r][i] / a[i][i];
                a[r][i] = 0.0;
                #[allow(clippy::needless_range_loop)]
                for c in (i + 1)..n {
                    a[r][c] -= factor * a[i][c];
                }
                b[r] -= factor * b[i];
            }
        }

        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let mut sum = b[i];
            for (j, xj) in x.iter().enumerate().skip(i + 1) {
                sum -= a[i][j] * *xj;
            }
            x[i] = sum / a[i][i];
        }

        Ok(x)
    }

    fn validate_noisy_results(&self, noisy_results: &[f64]) -> Result<(), ErrorMitigationError> {
        if noisy_results.is_empty() {
            return Err(ErrorMitigationError::EmptyNoisyResults);
        }

        let expected = self.noise_factors.len();
        let actual = noisy_results.len();
        if actual != expected {
            return Err(ErrorMitigationError::NoisyResultsLengthMismatch { expected, actual });
        }

        Ok(())
    }

    fn fold_to_level(
        &self,
        level: i32,
        gate_set: Option<&[Instruction]>,
    ) -> Result<Circuit, CircuitError> {
        if level < 0 {
            return Err(CircuitError::InvalidControlOperation(
                "Fold level must be non-negative.".to_string(),
            ));
        }

        if level == 0 {
            return Ok(self.circuit.clone());
        }

        match gate_set {
            None => self.fold_all(level as usize),
            Some(gates) => self.fold_selected(level as usize, gates),
        }
    }

    fn fold_all(&self, level: usize) -> Result<Circuit, CircuitError> {
        let mut folded = Circuit::from_qubits(self.circuit.qubits())?;
        let inverse = self.circuit.inverse()?;

        self.append_circuit_ops(&mut folded, &self.circuit)?;
        for _ in 0..level {
            self.append_circuit_ops(&mut folded, &inverse)?;
            self.append_circuit_ops(&mut folded, &self.circuit)?;
        }

        Ok(folded)
    }

    fn fold_selected(
        &self,
        level: usize,
        gate_set: &[Instruction],
    ) -> Result<Circuit, CircuitError> {
        let gate_names: HashSet<String> = gate_set.iter().map(Instruction::name).collect();
        let mut folded = Circuit::from_qubits(self.circuit.qubits())?;

        for op in self.circuit.operations() {
            self.append_operation(&mut folded, &self.circuit, op)?;
            if gate_names.contains(&op.instruction.name()) {
                for _ in 0..level {
                    let inv = self.invert_operation(op)?;
                    self.append_operation(&mut folded, &inv.0, &inv.1)?;
                    self.append_operation(&mut folded, &self.circuit, op)?;
                }
            }
        }

        Ok(folded)
    }

    fn append_circuit_ops(
        &self,
        target: &mut Circuit,
        source: &Circuit,
    ) -> Result<(), CircuitError> {
        for op in source.operations() {
            self.append_operation(target, source, op)?;
        }
        Ok(())
    }

    fn append_operation(
        &self,
        target: &mut Circuit,
        source: &Circuit,
        op: &Operation,
    ) -> Result<(), CircuitError> {
        let params = op.params.iter().map(|param| match param {
            crate::circuit::circuit_param::CircuitParam::Fixed(value) => (*value).into(),
            crate::circuit::circuit_param::CircuitParam::Index(index) => {
                source.parameters()[*index as usize].clone().into()
            }
        });

        target.append(
            op.instruction.clone(),
            op.qubits.iter().copied(),
            params,
            op.label.as_deref(),
        )
    }

    fn invert_operation(&self, op: &Operation) -> Result<(Circuit, Operation), CircuitError> {
        let op_params: Vec<Parameter> = op
            .params
            .iter()
            .map(|param| match param {
                crate::circuit::circuit_param::CircuitParam::Fixed(value) => {
                    Parameter::from(*value)
                }
                crate::circuit::circuit_param::CircuitParam::Index(index) => {
                    self.circuit.parameters()[*index as usize].clone()
                }
            })
            .collect();

        let (inv_inst, inv_params) = op
            .instruction
            .inverse(&op_params)
            .ok_or(CircuitError::IrreversibleOperation)?;

        let mut inv_circuit = Circuit::from_qubits(self.circuit.qubits())?;
        inv_circuit.append(
            inv_inst,
            op.qubits.iter().copied(),
            inv_params.into_iter().map(Into::into),
            op.label.as_deref(),
        )?;
        let inv_op = inv_circuit.operations()[0].clone();

        Ok((inv_circuit, inv_op))
    }
}

#[cfg(test)]
#[path = "./zne_mitigation_test.rs"]
mod zne_mitigation_test;
