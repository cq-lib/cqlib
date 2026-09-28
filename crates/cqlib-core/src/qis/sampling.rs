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

//! Direct circuit sampling with the existing ideal simulators.

use std::collections::{HashMap, HashSet};

use rand::SeedableRng;
use rand::distr::{Distribution, weighted::WeightedIndex};
use rand::rngs::SmallRng;

use crate::circuit::gate::ClassicalDataOp;
use crate::circuit::{Circuit, Directive, Instruction, Qubit};
use crate::device::{ExecutionResult, Outcome};
use crate::qis::{DensityMatrix, QisError, StabilizerState, Statevector};

/// Ideal simulator used by [`sample`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Simulator {
    /// General pure-state simulation; the default.
    #[default]
    Statevector,
    /// Density-matrix simulation, including deterministic reset channels.
    DensityMatrix,
    /// Simulation of the Clifford gates supported by [`StabilizerState`].
    Stabilizer,
}

/// Samples a circuit from the all-zero initial state without changing it.
///
/// An explicit `qubits` list selects the final Z-basis outputs in bit order,
/// with the first qubit displayed at the right of each result string. Otherwise,
/// terminal measurement declarations select the outputs in first-occurrence
/// order, with duplicates removed. Without declarations, all circuit qubits
/// are sampled in circuit order, including idle qubits.
///
/// The circuit must have bound parameters and only terminal measurements.
/// Classical control flow is unsupported. Barrier and Delay are no-ops; Store
/// does not affect the returned quantum measurement counts. Reset requires
/// [`Simulator::DensityMatrix`]. A seed makes counts reproducible for the same
/// simulator and library version. `shots` and the selected output width must
/// both be positive.
///
/// ```
/// use cqlib_core::circuit::{Circuit, Qubit};
/// use cqlib_core::qis::{sample, Simulator};
///
/// let mut circuit = Circuit::new(2);
/// circuit.h(Qubit::new(0)).unwrap();
/// circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
/// let result = sample(&circuit, 1000, Some(42), Simulator::default(), None).unwrap();
/// assert_eq!(result.counts().values().sum::<usize>(), 1000);
/// ```
pub fn sample(
    circuit: &Circuit,
    shots: usize,
    seed: Option<u64>,
    simulator: Simulator,
    qubits: Option<&[Qubit]>,
) -> Result<ExecutionResult, QisError> {
    if shots == 0 {
        return Err(QisError::InvalidParameterValue(
            "shots must be a positive integer".into(),
        ));
    }

    if circuit
        .operations()
        .iter()
        .any(|op| matches!(op.instruction, Instruction::ClassicalControl(_)))
    {
        return Err(QisError::UnsupportedOperation(
            "sample does not support classical control flow".into(),
        ));
    }
    let circuit = circuit.decompose()?;
    if simulator != Simulator::DensityMatrix
        && circuit
            .operations()
            .iter()
            .any(|op| matches!(op.instruction, Instruction::Directive(Directive::Reset)))
    {
        return Err(QisError::UnsupportedOperation(
            "sample with reset requires the density_matrix simulator".into(),
        ));
    }
    super::state::validate_terminal_measurements(&circuit)?;

    let circuit_qubits = circuit.qubits();
    let mut selected = qubits.map(<[Qubit]>::to_vec).unwrap_or_else(|| {
        let mut seen = HashSet::new();
        circuit
            .operations()
            .iter()
            .filter(|op| {
                matches!(
                    op.instruction,
                    Instruction::ClassicalData(
                        ClassicalDataOp::MeasureBit { .. } | ClassicalDataOp::MeasureBits { .. }
                    )
                )
            })
            .flat_map(|op| op.qubits.iter().copied())
            .filter(|qubit| seen.insert(*qubit))
            .collect()
    });
    if qubits.is_none() && selected.is_empty() {
        selected = circuit_qubits.clone();
    }
    if selected.is_empty() {
        return Err(QisError::InvalidParameterValue(
            "sample requires at least one output qubit".into(),
        ));
    }
    let qubit_positions: HashMap<_, _> = circuit_qubits
        .iter()
        .enumerate()
        .map(|(position, &qubit)| (qubit, position))
        .collect();
    let mut seen = HashSet::new();
    let positions = selected
        .iter()
        .map(|qubit| {
            if !seen.insert(*qubit) {
                return Err(QisError::InvalidParameterValue(
                    "qubits must not contain duplicates".into(),
                ));
            }
            qubit_positions.get(qubit).copied().ok_or_else(|| {
                QisError::InvalidParameterValue(format!(
                    "output qubit {} is not in the circuit",
                    qubit.id()
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut counts = HashMap::new();
    let backend = match simulator {
        Simulator::Statevector | Simulator::DensityMatrix => {
            let (mut probabilities, backend) = match simulator {
                Simulator::Statevector => (
                    Statevector::from_circuit(&circuit)?.probabilities(),
                    "statevector",
                ),
                _ => (
                    DensityMatrix::from_circuit(&circuit)?.probabilities(),
                    "density_matrix",
                ),
            };
            // Density-matrix evolution can leave tiny negative diagonal entries
            // through roundoff, even for a gate followed by its inverse.
            for probability in &mut probabilities {
                if (-1e-12..0.0).contains(probability) {
                    *probability = 0.0;
                }
            }
            let distribution = WeightedIndex::new(probabilities)
                .map_err(|error| QisError::InvalidParameterValue(error.to_string()))?;
            let mut rng = seed.map_or_else(SmallRng::from_os_rng, SmallRng::seed_from_u64);
            for _ in 0..shots {
                let basis = distribution.sample(&mut rng);
                let outcome = project(&positions, |position| basis & (1usize << position) != 0);
                *counts.entry(outcome).or_insert(0) += 1;
            }
            backend
        }
        Simulator::Stabilizer => {
            let mut state = StabilizerState::from_circuit(&circuit)?;
            if let Some(seed) = seed {
                state.seed_rng(seed);
            }
            for full in state.sample_shots(shots) {
                let outcome = project(&positions, |position| full.is_one(position));
                *counts.entry(outcome).or_insert(0) += 1;
            }
            "stabilizer"
        }
    };

    Ok(ExecutionResult::from_counts(
        format!("{backend}-sample"),
        selected,
        shots,
        positions.len(),
        Some(backend.into()),
        counts,
    ))
}

fn project(positions: &[usize], is_one: impl Fn(usize) -> bool) -> Outcome {
    Outcome::from_indices(
        positions.len(),
        positions
            .iter()
            .enumerate()
            .filter_map(|(bit, &position)| is_one(position).then_some(bit)),
    )
}
