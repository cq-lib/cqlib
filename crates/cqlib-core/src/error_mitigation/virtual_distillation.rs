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

use std::collections::HashMap;

use num_complex::Complex64;

use crate::circuit::{Circuit, CircuitError, CircuitParam, ParameterValue, Qubit, StandardGate};
use crate::error_mitigation::ErrorMitigationError;
use crate::error_mitigation::Estimator;
use crate::qis::{Hamiltonian, Pauli, PauliString};

/// Virtual distillation mitigation based on the moment ratio
/// `Tr(O rho^M) / Tr(rho^M)`.
/// Based on: [1] W. J. Huggins et al., “Virtual Distillation for Quantum Error Mitigation,”
///     Phys. Rev. X, vol. 11, no. 4, p. 041036, Nov. 2021, doi: 10.1103/PhysRevX.11.041036.
///
/// # Example
///
/// ```rust
/// use cqlib_core::circuit::{Circuit, Qubit};
/// use cqlib_core::error_mitigation::VirtualDistillation;
///
/// let q0 = Qubit::new(0);
/// let mut circuit = Circuit::new(1);
/// circuit.x(q0).unwrap();
///
/// let _vd = VirtualDistillation::new(circuit, 2).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct VirtualDistillation {
    circuit: Circuit,
    copies: usize,
}

impl VirtualDistillation {
    /// Creates a new virtual distillation helper.
    pub fn new(circuit: Circuit, copies: usize) -> Result<Self, ErrorMitigationError> {
        if copies < 2 {
            return Err(ErrorMitigationError::InvalidCopies(copies));
        }

        Ok(Self { circuit, copies })
    }

    /// Returns the configured number of copies.
    pub fn copies(&self) -> usize {
        self.copies
    }

    /// Updates the configured number of copies.
    pub fn set_copies(&mut self, copies: usize) -> Result<(), ErrorMitigationError> {
        if copies < 2 {
            return Err(ErrorMitigationError::InvalidCopies(copies));
        }

        self.copies = copies;
        Ok(())
    }

    /// Builds the virtual distillation circuit from the configured base circuit.
    ///
    /// The returned circuit contains:
    /// - `copies` disjoint copies of the base circuit preparation,
    /// - one ancillary qubit (the last qubit, index `copies * base_width`),
    ///   prepared in `|+>` by a Hadamard gate,
    /// - a derangement of the copies implemented as a ladder of swap gates
    ///   between adjacent copies, each controlled by the ancillary qubit
    ///   (CSWAP).
    ///
    /// This is the Hadamard-test circuit of the virtual distillation protocol:
    /// measuring `X` on the ancillary qubit (together with an observable on the
    /// first copy) yields `Tr(O rho^M)` / `Tr(rho^M)`. Applying uncontrolled
    /// swaps instead would leave the permutation-symmetric state `rho^M`
    /// invariant and measure nothing related to the distilled moments.
    pub fn build_copy_swap_circuit(&self) -> Result<Circuit, CircuitError> {
        let base_circuit = self.circuit.decompose()?;
        let base_width = base_circuit.width();
        let ancilla = Qubit::new((self.copies * base_width) as u32);
        let mut copy_swap_circuit = Circuit::new(self.copies * base_width + 1);

        for copy_index in 0..self.copies {
            let copy_offset = copy_index * base_width;
            Self::append_circuit_with_offset(&mut copy_swap_circuit, &base_circuit, copy_offset)?;
        }

        copy_swap_circuit.h(ancilla)?;

        for lower_copy in 0..(self.copies - 1) {
            let lower_offset = lower_copy * base_width;
            let upper_offset = (lower_copy + 1) * base_width;
            for qubit_index in 0..base_width {
                let lower = Qubit::new((lower_offset + qubit_index) as u32);
                let upper = Qubit::new((upper_offset + qubit_index) as u32);
                copy_swap_circuit.multi_control(
                    StandardGate::SWAP,
                    [ancilla],
                    [lower, upper],
                    [],
                )?;
            }
        }

        Ok(copy_swap_circuit)
    }

    /// Expands a Hamiltonian to the numerator observable on the full virtual
    /// distillation circuit width.
    ///
    /// The numerator of virtual distillation estimates `Tr(O rho^M)`, measured
    /// as the expectation value of `X` on the ancillary qubit tensored with the
    /// observable on the first copy and identities on the remaining copies.
    /// This helper keeps the original Pauli operators on their current qubit
    /// indices, pads identities on the additional copies, and appends an `X`
    /// on the ancillary qubit (the last qubit).
    pub(crate) fn expand_hamiltonian(
        hamiltonian: &Hamiltonian,
        copies: usize,
    ) -> Result<Hamiltonian, ErrorMitigationError> {
        let width = hamiltonian.num_qubits * copies + 1;
        let ancilla = width - 1;
        if hamiltonian.terms.is_empty() {
            return Ok(Hamiltonian::new(width));
        }

        let expanded_terms = hamiltonian
            .terms
            .iter()
            .map(|(term, coeff)| {
                let mut expanded_term = PauliString::new(width);
                expanded_term.phase = term.phase;

                for qubit in 0..hamiltonian.num_qubits {
                    let pauli = match (term.x[qubit], term.z[qubit]) {
                        (false, false) => Pauli::I,
                        (true, false) => Pauli::X,
                        (false, true) => Pauli::Z,
                        (true, true) => Pauli::Y,
                    };
                    expanded_term.set_pauli(qubit, pauli);
                }

                expanded_term.set_pauli(ancilla, Pauli::X);

                (expanded_term, *coeff)
            })
            .collect();

        Ok(Hamiltonian::from_list(expanded_terms)?)
    }

    /// Returns the denominator observable of virtual distillation.
    ///
    /// The denominator estimates `Tr(rho^M)`, measured as the expectation
    /// value of `X` on the ancillary qubit (the last qubit) with identities
    /// everywhere else.
    fn denominator_hamiltonian(&self) -> Result<Hamiltonian, ErrorMitigationError> {
        let width = self.copies * self.circuit.width() + 1;
        let mut term = PauliString::new(width);
        term.set_pauli(width - 1, Pauli::X);
        Ok(Hamiltonian::from_list(vec![(
            term,
            Complex64::new(1.0, 0.0),
        )])?)
    }

    /// Runs the denominator circuit and returns the estimated mean and variance.
    ///
    /// - `shots`: the number of shots to run the circuit.
    /// - `estimator`: an estimator that evaluates the virtual distillation
    ///   circuit with the denominator observable (`X` on the ancillary qubit,
    ///   identities elsewhere) and the provided shot count.
    ///
    /// # Returns
    ///
    /// - `(mu, var)`: the estimated mean and variance of `Tr(rho^M)`.
    pub fn run_denominator_circuit(
        &self,
        shots: usize,
        estimator: &Estimator<'_>,
    ) -> Result<(f64, f64), ErrorMitigationError> {
        let denominator_circuit = self.build_copy_swap_circuit()?;
        let denominator_hamiltonian = self.denominator_hamiltonian()?;

        Ok(estimator(
            &denominator_circuit,
            Some(&denominator_hamiltonian),
            Some(shots),
        ))
    }

    /// Runs the numerator circuit and returns the estimated mean and variance.
    ///
    /// - `hamiltonian`: the Hamiltonian to estimate on the virtual distillation
    ///   circuit. It is expanded to the full circuit width with `X` on the
    ///   ancillary qubit (see [`Self::expand_hamiltonian`]).
    /// - `shots`: the number of shots to run the circuit.
    /// - `estimator`: an estimator that evaluates the virtual distillation
    ///   circuit, the given expanded Hamiltonian, and the provided shot count.
    ///
    /// # Returns
    ///
    /// - `(mu, var)`: the estimated mean and variance of `Tr(H rho^M)`.
    pub fn run_numerator_circuit(
        &self,
        hamiltonian: &Hamiltonian,
        shots: usize,
        estimator: &Estimator<'_>,
    ) -> Result<(f64, f64), ErrorMitigationError> {
        let numerator_circuit = self.build_copy_swap_circuit()?;
        let expanded_hamiltonian = Self::expand_hamiltonian(hamiltonian, self.copies)?;
        if expanded_hamiltonian.num_qubits != numerator_circuit.width() {
            return Err(CircuitError::QubitCountMismatch {
                expected: numerator_circuit.width(),
                actual: expanded_hamiltonian.num_qubits,
            }
            .into());
        }

        Ok(estimator(
            &numerator_circuit,
            Some(&expanded_hamiltonian),
            Some(shots),
        ))
    }

    /// Runs the virtual distillation circuit and returns the mean and variance.
    ///
    /// - `hamiltonian`: a `qis::Hamiltonian` describing the observable to estimate for the original circuit.
    /// - `shots_numerator`: the number of shots to run the numerator circuit.
    /// - `shots_denominator`: the number of shots to run the denominator circuit.
    /// - `estimator`: an estimator that evaluates circuits and returns `(mu, var)`.
    ///
    /// # Returns
    ///
    /// - `mu_vd`: the mitigated result of the observable on the original circuit.
    /// - `var_vd`: the variance of the mitigated result.
    pub fn run_vd(
        &self,
        hamiltonian: &Hamiltonian,
        shots_numerator: usize,
        shots_denominator: usize,
        estimator: &Estimator<'_>,
    ) -> Result<(f64, f64), ErrorMitigationError> {
        if hamiltonian.num_qubits != self.circuit.width() {
            return Err(ErrorMitigationError::HamiltonianQubitCountMismatch {
                expected: self.circuit.width(),
                actual: hamiltonian.num_qubits,
            });
        }

        let (mu_numerator, var_numerator) =
            self.run_numerator_circuit(hamiltonian, shots_numerator, estimator)?;
        let (mu_denominator, var_denominator) =
            self.run_denominator_circuit(shots_denominator, estimator)?;
        let (mu_vd, var_vd) = Self::mitigate_from_statistics(
            mu_numerator,
            var_numerator,
            mu_denominator,
            var_denominator,
        )?;

        Ok((mu_vd, var_vd))
    }

    pub(crate) fn mitigate_from_statistics(
        mu_numerator: f64,
        var_numerator: f64,
        mu_denominator: f64,
        var_denominator: f64,
    ) -> Result<(f64, f64), ErrorMitigationError> {
        if mu_denominator == 0.0 {
            return Err(ErrorMitigationError::ZeroDenominatorMean);
        }

        let mu_vd = mu_numerator / mu_denominator;

        // D11 from the paper, using Taylor approximation and assuming independence
        // of numerator and denominator.
        let var_vd = var_numerator / mu_denominator.powi(2)
            + mu_numerator.powi(2) * var_denominator / mu_denominator.powi(4);
        Ok((mu_vd, var_vd))
    }

    fn append_circuit_with_offset(
        target_circuit: &mut Circuit,
        source_circuit: &Circuit,
        qubit_offset: usize,
    ) -> Result<(), CircuitError> {
        let source_qubits = source_circuit.qubits();
        let qubit_positions: HashMap<_, _> = source_qubits
            .iter()
            .enumerate()
            .map(|(position, qubit)| (*qubit, position))
            .collect();

        for op in source_circuit.operations() {
            let mapped_qubits: Vec<Qubit> = op
                .qubits
                .iter()
                .map(|qubit| {
                    let position = qubit_positions[qubit];
                    Qubit::new((qubit_offset + position) as u32)
                })
                .collect();
            let mapped_params: Vec<ParameterValue> = op
                .params
                .iter()
                .map(|param| match param {
                    CircuitParam::Fixed(value) => ParameterValue::Fixed(*value),
                    CircuitParam::Index(index) => {
                        source_circuit.parameters()[*index as usize].clone().into()
                    }
                })
                .collect();

            target_circuit.append(
                op.instruction.clone(),
                mapped_qubits,
                mapped_params,
                op.label.as_deref(),
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "./virtual_distillation_test.rs"]
mod virtual_distillation_test;
