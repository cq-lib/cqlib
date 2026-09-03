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

//! # QAOA Ansatz Module
//!
//! This module implements the Quantum Approximate Optimization Algorithm (QAOA)
//! ansatz. QAOA is a variational quantum algorithm designed for solving
//! combinatorial optimization problems.
//!
//! ## Algorithm Overview
//!
//! The QAOA circuit alternates between:
//! - **Cost Layer**: Applies $e^{-i\gamma H_C}$ where $H_C$ is the problem Hamiltonian
//! - **Mixer Layer**: Applies $e^{-i\beta H_B}$ where $H_B$ is typically $\sum X_i$
//!
//! By optimizing the parameters $(\gamma, \beta)$, QAOA approximates the ground
//! state of the cost Hamiltonian.

use super::hamiltonian_evolution::EvolutionStrategy;
use super::traits::Ansatz;
use crate::circuit::Parameter;
use crate::circuit::bit::Qubit;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::circuit_param::ParameterValue;
use crate::circuit::error::CircuitError;
use crate::qis::evolution::{
    PauliEvolution, TrotterMode, multiply_angle_by_factor, trotter_first_order_core,
    trotter_second_order_core,
};
use crate::qis::hamiltonian::Hamiltonian;
use crate::qis::pauli::{Pauli, PauliString};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// The QAOA (Quantum Approximate Optimization Algorithm) Ansatz.
///
/// QAOA is a widely used variational quantum algorithm for solving combinatorial
/// optimization problems. The ansatz consists of alternating layers of a cost
/// Hamiltonian $H_C$ and a mixer Hamiltonian $H_B$.
///
/// For a given number of layers $p$, the circuit applies:
/// $$ U(\boldsymbol{\gamma}, \boldsymbol{\beta}) = \prod_{j=1}^{p} e^{-i \beta_j H_B} e^{-i \gamma_j H_C} $$
///
/// By default, the mixer Hamiltonian $H_B$ is the sum of Pauli X operators
/// on all qubits: $H_B = \sum_i X_i$.
#[derive(Debug, Clone)]
pub struct QAOAAnsatz {
    cost_operator: Hamiltonian,
    mixer_operator: Hamiltonian,
    reps: usize,
    initial_state: Option<Circuit>,
    evolution_strategy: EvolutionStrategy,
}

impl QAOAAnsatz {
    /// Creates a new QAOA Ansatz from a given cost operator (Hamiltonian).
    ///
    /// By default, it uses the standard X-mixer ($\sum X_i$) and `reps = 1`.
    /// The number of qubits is inferred from the cost operator.
    ///
    /// # Errors
    ///
    /// Returns `CircuitError::InvalidOperation` if there's an internal error building
    /// the default mixer Hamiltonian.
    pub fn new(mut cost_operator: Hamiltonian) -> Result<Self, CircuitError> {
        // Normalize Pauli phases into the coefficients before Hermiticity validation.
        // For example, 1.0 * (+iX) becomes i * X and is correctly rejected later,
        // while i * (-iX) becomes 1.0 * X and remains a valid Hermitian term.
        cost_operator.simplify();
        validate_qaoa_hamiltonian_structure(&cost_operator, "Cost")?;
        let num_qubits = cost_operator.num_qubits;

        // Build the default X-mixer: H_B = \sum_{i=0}^{n-1} X_i
        // Directly set Pauli X at each qubit index to avoid relying on
        // the string parsing convention (highest-index-first).
        let mut mixer_operator = Hamiltonian::new(num_qubits);
        for i in 0..num_qubits {
            let mut pauli = PauliString::new(num_qubits);
            pauli.set_pauli(i, Pauli::X);
            mixer_operator.add_term(pauli, 1.0.into()).map_err(|e| {
                CircuitError::InvalidOperation(format!(
                    "Failed to add term to mixer Hamiltonian: {:?}",
                    e
                ))
            })?;
        }
        mixer_operator.simplify();
        validate_qaoa_hamiltonian_structure(&mixer_operator, "Mixer")?;

        Ok(Self {
            cost_operator,
            mixer_operator,
            reps: 1,
            initial_state: None,
            evolution_strategy: EvolutionStrategy::Auto { steps: 1 },
        })
    }

    /// Sets the number of alternating layers (depth $p$).
    pub fn reps(mut self, reps: usize) -> Self {
        self.reps = reps;
        self
    }

    /// Overrides the default mixer Hamiltonian.
    ///
    /// The custom mixer must act on the same number of qubits as the cost operator.
    pub fn mixer(mut self, mut mixer_operator: Hamiltonian) -> Result<Self, CircuitError> {
        if mixer_operator.num_qubits != self.cost_operator.num_qubits {
            return Err(CircuitError::QubitCountMismatch {
                expected: self.cost_operator.num_qubits,
                actual: mixer_operator.num_qubits,
            });
        }
        mixer_operator.simplify();
        validate_qaoa_hamiltonian_structure(&mixer_operator, "Mixer")?;
        self.mixer_operator = mixer_operator;
        Ok(self)
    }

    /// Sets an initial state circuit to be prepended before the QAOA layers.
    ///
    /// By default, QAOA starts in the uniform superposition state $|+\rangle^{\otimes n}$.
    /// If an initial state is provided, it replaces the default Hadamard layer.
    pub fn initial_state(mut self, circuit: Circuit) -> Result<Self, CircuitError> {
        if circuit.num_qubits() != self.cost_operator.num_qubits {
            return Err(CircuitError::QubitCountMismatch {
                expected: self.cost_operator.num_qubits,
                actual: circuit.num_qubits(),
            });
        }
        self.initial_state = Some(circuit);
        Ok(self)
    }

    /// Sets the Hamiltonian evolution strategy used for both cost and mixer layers.
    ///
    /// The default is `EvolutionStrategy::Auto { steps: 1 }`: commuting Hamiltonians
    /// are decomposed exactly, while non-commuting Hamiltonians use first-order
    /// Trotterization with one step. Use `EvolutionStrategy::Exact` to require
    /// exact term-wise evolution, or `EvolutionStrategy::Trotter` to control the
    /// product-formula mode and step count explicitly.
    pub fn evolution_strategy(mut self, strategy: EvolutionStrategy) -> Self {
        self.evolution_strategy = strategy;
        self
    }
}

impl Ansatz for QAOAAnsatz {
    fn validate(&self) -> Result<(), CircuitError> {
        validate_qaoa_hamiltonian(&self.cost_operator, "Cost")?;
        validate_qaoa_hamiltonian(&self.mixer_operator, "Mixer")?;
        validate_qaoa_evolution_strategy(&self.evolution_strategy, &self.cost_operator, "Cost")?;
        validate_qaoa_evolution_strategy(&self.evolution_strategy, &self.mixer_operator, "Mixer")?;

        // Validate mixer operator has same number of qubits as cost operator
        if self.mixer_operator.num_qubits != self.cost_operator.num_qubits {
            return Err(CircuitError::QubitCountMismatch {
                expected: self.cost_operator.num_qubits,
                actual: self.mixer_operator.num_qubits,
            });
        }

        // Validate initial state has same number of qubits if provided
        if let Some(initial_circuit) = &self.initial_state
            && initial_circuit.num_qubits() != self.cost_operator.num_qubits
        {
            return Err(CircuitError::QubitCountMismatch {
                expected: self.cost_operator.num_qubits,
                actual: initial_circuit.num_qubits(),
            });
        }

        Ok(())
    }

    /// Builds the parameterized QAOA circuit.
    ///
    /// Parameters are generated using the provided prefix. For example, if `prefix` is "p",
    /// the parameters will be named "p_gamma_0", "p_beta_0", "p_gamma_1", "p_beta_1", etc.
    fn build_circuit(&self, prefix: &str) -> Result<Circuit, CircuitError> {
        self.validate()?;
        // 1. Prepare initial state
        let mut circuit = if let Some(initial_circuit) = &self.initial_state {
            initial_circuit.clone()
        } else {
            let n = self.num_qubits();
            let mut c = Circuit::new(n);
            let qubits = c.qubits();
            for q in &qubits {
                c.h(*q)?;
            }
            c
        };

        let qubits = circuit.qubits();

        // 2. Apply alternating layers of Cost and Mixer
        for layer in 0..self.reps {
            // Create parameters for this layer
            let gamma_name = format!("{}_gamma_{}", prefix, layer);
            let beta_name = format!("{}_beta_{}", prefix, layer);

            let gamma_param = Parameter::try_from(gamma_name.as_str())
                .map_err(|_| CircuitError::InvalidParameterValue(layer * 2, f64::NAN))?;
            let beta_param = Parameter::try_from(beta_name.as_str())
                .map_err(|_| CircuitError::InvalidParameterValue(layer * 2 + 1, f64::NAN))?;

            append_hamiltonian_evolution(
                &mut circuit,
                &self.cost_operator,
                ParameterValue::Param(gamma_param),
                &self.evolution_strategy,
                &qubits,
            )?;

            append_hamiltonian_evolution(
                &mut circuit,
                &self.mixer_operator,
                ParameterValue::Param(beta_param),
                &self.evolution_strategy,
                &qubits,
            )?;
        }

        Ok(circuit)
    }

    /// QAOA requires 2 parameters per layer: one for the cost Hamiltonian ($\gamma$)
    /// and one for the mixer Hamiltonian ($\beta$).
    fn num_parameters(&self) -> usize {
        self.reps * 2
    }

    fn num_qubits(&self) -> usize {
        self.cost_operator.num_qubits
    }
}

fn append_hamiltonian_evolution(
    circuit: &mut Circuit,
    hamiltonian: &Hamiltonian,
    time: ParameterValue,
    strategy: &EvolutionStrategy,
    qubits: &[Qubit],
) -> Result<(), CircuitError> {
    match strategy {
        EvolutionStrategy::Exact => {
            append_exact_hamiltonian_evolution(circuit, hamiltonian, time, qubits)
        }
        EvolutionStrategy::Auto { steps } => {
            if hamiltonian.all_terms_commute() {
                append_exact_hamiltonian_evolution(circuit, hamiltonian, time, qubits)
            } else {
                trotter_first_order_core(circuit, &hamiltonian.terms, time, *steps, qubits, None)
            }
        }
        EvolutionStrategy::Trotter { mode, steps } => match mode {
            TrotterMode::FirstOrder => {
                trotter_first_order_core(circuit, &hamiltonian.terms, time, *steps, qubits, None)
            }
            TrotterMode::SecondOrder => {
                trotter_second_order_core(circuit, &hamiltonian.terms, time, *steps, qubits)
            }
            TrotterMode::Randomized(seed) => {
                let mut rng = StdRng::seed_from_u64(*seed);
                trotter_first_order_core(
                    circuit,
                    &hamiltonian.terms,
                    time,
                    *steps,
                    qubits,
                    Some(&mut rng),
                )
            }
        },
    }
}

fn append_exact_hamiltonian_evolution(
    circuit: &mut Circuit,
    hamiltonian: &Hamiltonian,
    time: ParameterValue,
    qubits: &[Qubit],
) -> Result<(), CircuitError> {
    for (pauli_str, coeff) in &hamiltonian.terms {
        let term_angle = multiply_angle_by_factor(time.clone(), 2.0 * coeff.re);
        circuit.pauli_evolution(pauli_str, term_angle, qubits)?;
    }
    Ok(())
}

fn validate_qaoa_hamiltonian(hamiltonian: &Hamiltonian, name: &str) -> Result<(), CircuitError> {
    validate_qaoa_hamiltonian_structure(hamiltonian, name)?;

    for (pauli_str, coeff) in &hamiltonian.terms {
        if coeff.im.abs() > 1e-10 {
            return Err(CircuitError::InvalidOperation(format!(
                "{name} Hamiltonian coefficient for {} has non-zero imaginary part ({}). QAOA requires Hermitian Hamiltonian with real coefficients.",
                pauli_str, coeff.im
            )));
        }
    }

    Ok(())
}

fn validate_qaoa_evolution_strategy(
    strategy: &EvolutionStrategy,
    hamiltonian: &Hamiltonian,
    name: &str,
) -> Result<(), CircuitError> {
    match strategy {
        EvolutionStrategy::Exact => {
            if !hamiltonian.all_terms_commute() {
                return Err(CircuitError::InvalidOperation(format!(
                    "EvolutionStrategy::Exact requires all {name} Hamiltonian terms to mutually commute"
                )));
            }
        }
        EvolutionStrategy::Auto { steps } => {
            if *steps == 0 {
                return Err(CircuitError::InvalidOperation(
                    "EvolutionStrategy::Auto requires steps >= 1".to_string(),
                ));
            }
        }
        EvolutionStrategy::Trotter { steps, .. } => {
            if *steps == 0 {
                return Err(CircuitError::InvalidOperation(
                    "EvolutionStrategy::Trotter requires steps >= 1".to_string(),
                ));
            }
        }
    }

    Ok(())
}

fn validate_qaoa_hamiltonian_structure(
    hamiltonian: &Hamiltonian,
    name: &str,
) -> Result<(), CircuitError> {
    if hamiltonian.num_qubits == 0 {
        return Err(CircuitError::InvalidOperation(format!(
            "{name} Hamiltonian requires at least one qubit"
        )));
    }

    if hamiltonian.terms.is_empty() {
        return Err(CircuitError::InvalidOperation(format!(
            "{name} Hamiltonian contains no non-zero terms"
        )));
    }

    if hamiltonian
        .terms
        .iter()
        .all(|(pauli, _)| pauli.support().is_empty())
    {
        return Err(CircuitError::InvalidOperation(format!(
            "{name} Hamiltonian contains only identity terms"
        )));
    }

    Ok(())
}

#[cfg(test)]
#[path = "qaoa_test.rs"]
mod qaoa_test;
