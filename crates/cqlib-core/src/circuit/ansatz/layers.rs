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

//! Layer-style hardware-efficient ansatze.
//!
//! These templates mirror commonly used layer primitives: one simple entangler
//! layer family and one more expressive strongly-entangling layer family.

use super::traits::Ansatz;
use crate::circuit::bit::Qubit;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::error::CircuitError;
use crate::circuit::gate::StandardGate;
use crate::circuit::{Instruction, Parameter, ParameterValue};

fn validate_rotation_gate(name: &str, gate: StandardGate) -> Result<(), CircuitError> {
    if !matches!(
        gate,
        StandardGate::RX | StandardGate::RY | StandardGate::RZ | StandardGate::Phase
    ) {
        return Err(CircuitError::InvalidOperation(format!(
            "{name} rotation_gate must be RX, RY, RZ, or Phase, got {gate:?}"
        )));
    }
    Ok(())
}

fn validate_entanglement_gate(name: &str, gate: StandardGate) -> Result<(), CircuitError> {
    if !matches!(gate, StandardGate::CX | StandardGate::CY | StandardGate::CZ) {
        return Err(CircuitError::InvalidOperation(format!(
            "{name} entanglement_gate must be CX, CY, or CZ, got {gate:?}"
        )));
    }
    Ok(())
}

fn parameter(prefix: &str, idx: usize) -> Result<ParameterValue, CircuitError> {
    let param_name = format!("{prefix}_{idx}");
    Parameter::try_from(param_name.as_str())
        .map(ParameterValue::Param)
        .map_err(|_| CircuitError::InvalidParameterValue(idx, f64::NAN))
}

/// Basic entangler layers.
///
/// Each layer applies one parameterized single-qubit rotation to every qubit,
/// followed by a nearest-neighbor entangling ring. For two qubits the ring is
/// a single edge `(0, 1)`; for one qubit no entanglement gate is added.
#[derive(Debug, Clone)]
pub struct BasicEntanglerLayers {
    num_qubits: usize,
    reps: usize,
    rotation_gate: StandardGate,
    entanglement_gate: StandardGate,
}

impl BasicEntanglerLayers {
    /// Creates a new BasicEntanglerLayers template.
    ///
    /// Defaults: `reps = 1`, `rotation_gate = RX`, `entanglement_gate = CX`.
    pub fn new(num_qubits: usize) -> Self {
        Self {
            num_qubits,
            reps: 1,
            rotation_gate: StandardGate::RX,
            entanglement_gate: StandardGate::CX,
        }
    }

    /// Sets the number of layers.
    pub fn reps(mut self, reps: usize) -> Self {
        self.reps = reps;
        self
    }

    /// Sets the single-parameter rotation gate used in each layer.
    pub fn rotation_gate(mut self, gate: StandardGate) -> Self {
        self.rotation_gate = gate;
        self
    }

    /// Sets the two-qubit entangling gate used after each rotation layer.
    pub fn entanglement_gate(mut self, gate: StandardGate) -> Self {
        self.entanglement_gate = gate;
        self
    }

    fn entanglement_pairs(&self) -> Vec<(usize, usize)> {
        if self.num_qubits < 2 {
            return Vec::new();
        }
        let mut pairs: Vec<_> = (0..self.num_qubits - 1).map(|i| (i, i + 1)).collect();
        if self.num_qubits > 2 {
            pairs.push((self.num_qubits - 1, 0));
        }
        pairs
    }
}

impl Ansatz for BasicEntanglerLayers {
    fn validate(&self) -> Result<(), CircuitError> {
        if self.num_qubits == 0 {
            return Err(CircuitError::InvalidOperation(
                "BasicEntanglerLayers requires at least 1 qubit".to_string(),
            ));
        }
        validate_rotation_gate("BasicEntanglerLayers", self.rotation_gate)?;
        validate_entanglement_gate("BasicEntanglerLayers", self.entanglement_gate)?;
        Ok(())
    }

    fn build_circuit(&self, prefix: &str) -> Result<Circuit, CircuitError> {
        self.validate()?;

        let mut circuit = Circuit::new(self.num_qubits);
        let mut param_idx = 0;
        let pairs = self.entanglement_pairs();

        for _layer in 0..self.reps {
            for q in 0..self.num_qubits {
                circuit.append(
                    Instruction::Standard(self.rotation_gate),
                    vec![Qubit::new(q as u32)],
                    vec![parameter(prefix, param_idx)?],
                    None,
                )?;
                param_idx += 1;
            }
            for (control, target) in &pairs {
                circuit.append(
                    Instruction::Standard(self.entanglement_gate),
                    vec![Qubit::new(*control as u32), Qubit::new(*target as u32)],
                    vec![],
                    None,
                )?;
            }
        }

        Ok(circuit)
    }

    fn num_parameters(&self) -> usize {
        self.reps * self.num_qubits
    }

    fn num_qubits(&self) -> usize {
        self.num_qubits
    }
}

/// Strongly entangling layers.
///
/// Each layer applies a general single-qubit `U(theta, phi, lambda)` on every
/// qubit, then entangles every qubit `i` with `(i + range) mod num_qubits`.
/// The default range cycles through `1..num_qubits` across layers.
///
/// `CX` is the default entangling gate because the layer uses a directed
/// `(control, target)` connection pattern, and a controlled-X gate makes that
/// direction explicit. Users can still select `CX`, `CY`, or `CZ` manually to
/// match backend-native gates or experiment-specific circuit conventions.
#[derive(Debug, Clone)]
pub struct StronglyEntanglingLayers {
    num_qubits: usize,
    reps: usize,
    entanglement_gate: StandardGate,
    ranges: Option<Vec<usize>>,
}

impl StronglyEntanglingLayers {
    /// Creates a new StronglyEntanglingLayers template.
    ///
    /// Defaults: `reps = 1`, `entanglement_gate = CX`, ranges cycle by layer.
    /// The entanglement gate remains configurable so users can align the
    /// template with hardware-native or experiment-specific two-qubit gates.
    pub fn new(num_qubits: usize) -> Self {
        Self {
            num_qubits,
            reps: 1,
            entanglement_gate: StandardGate::CX,
            ranges: None,
        }
    }

    /// Sets the number of layers.
    pub fn reps(mut self, reps: usize) -> Self {
        self.reps = reps;
        self
    }

    /// Sets the two-qubit entangling gate used after each U layer.
    ///
    /// Valid choices are `CX`, `CY`, and `CZ`. `CX` is the default because this
    /// layer's range pattern is directed; custom gates are accepted when users
    /// intentionally want a different controlled or symmetric entangler.
    pub fn entanglement_gate(mut self, gate: StandardGate) -> Self {
        self.entanglement_gate = gate;
        self
    }

    /// Sets explicit entanglement ranges. Ranges are reused cyclically by layer.
    pub fn ranges(mut self, ranges: Vec<usize>) -> Self {
        self.ranges = Some(ranges);
        self
    }

    fn range_for_layer(&self, layer: usize) -> Option<usize> {
        if self.num_qubits < 2 {
            return None;
        }
        match &self.ranges {
            Some(ranges) => Some(ranges[layer % ranges.len()]),
            None => Some((layer % (self.num_qubits - 1)) + 1),
        }
    }
}

impl Ansatz for StronglyEntanglingLayers {
    fn validate(&self) -> Result<(), CircuitError> {
        if self.num_qubits == 0 {
            return Err(CircuitError::InvalidOperation(
                "StronglyEntanglingLayers requires at least 1 qubit".to_string(),
            ));
        }
        validate_entanglement_gate("StronglyEntanglingLayers", self.entanglement_gate)?;
        if self.num_qubits > 1
            && let Some(ranges) = &self.ranges
        {
            if ranges.is_empty() {
                return Err(CircuitError::InvalidOperation(
                    "StronglyEntanglingLayers ranges must not be empty".to_string(),
                ));
            }
            for &range in ranges {
                if range == 0 || range >= self.num_qubits {
                    return Err(CircuitError::InvalidOperation(format!(
                        "StronglyEntanglingLayers range must be in 1..{}, got {}",
                        self.num_qubits, range
                    )));
                }
            }
        }
        Ok(())
    }

    fn build_circuit(&self, prefix: &str) -> Result<Circuit, CircuitError> {
        self.validate()?;

        let mut circuit = Circuit::new(self.num_qubits);
        let mut param_idx = 0;

        for layer in 0..self.reps {
            for q in 0..self.num_qubits {
                circuit.u(
                    Qubit::new(q as u32),
                    parameter(prefix, param_idx)?,
                    parameter(prefix, param_idx + 1)?,
                    parameter(prefix, param_idx + 2)?,
                )?;
                param_idx += 3;
            }

            if let Some(range) = self.range_for_layer(layer) {
                for control in 0..self.num_qubits {
                    let target = (control + range) % self.num_qubits;
                    circuit.append(
                        Instruction::Standard(self.entanglement_gate),
                        vec![Qubit::new(control as u32), Qubit::new(target as u32)],
                        vec![],
                        None,
                    )?;
                }
            }
        }

        Ok(circuit)
    }

    fn num_parameters(&self) -> usize {
        self.reps * self.num_qubits * 3
    }

    fn num_qubits(&self) -> usize {
        self.num_qubits
    }
}

#[cfg(test)]
#[path = "layers_test.rs"]
mod layers_test;
