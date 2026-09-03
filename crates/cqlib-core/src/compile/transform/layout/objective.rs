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

//! Layout scoring objective shared by layout algorithms.
//!
//! A layout objective assigns lower scores to better candidate mappings. The
//! score combines topology distance, directed-coupling mismatch, optional
//! two-qubit error rates, and optional readout error rates. Algorithms may use
//! this objective either as a final ranking function or as a local tie-breaker
//! while constructing a candidate.

use super::analysis::CircuitLayoutAnalysis;
use super::scoring::score_adjacent_interaction;
use crate::compile::CompilerError;
use crate::compile::physical_target::PhysicalLayoutGraph;
use crate::device::Layout;
use std::collections::BTreeMap;

/// Weighted scoring objective used to rank candidate layouts.
///
/// All weights must be finite and non-negative. Setting a weight to `0.0`
/// removes that component from the total score. The raw components are still
/// reported in [`LayoutScore`] when they can be computed.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutObjective {
    /// Weight for weighted logical interaction distance.
    ///
    /// Each interaction contributes `interaction.weight * physical_distance`.
    pub distance_weight: f64,
    /// Weight for directly adjacent interactions whose observed operation
    /// direction is unsupported by the directed coupling graph.
    pub direction_weight: f64,
    /// Weight for effective two-qubit fidelity cost.
    ///
    /// Known gate-specific calibrations contribute their error probability.
    /// A gate that is native elsewhere on the device but unsupported or
    /// uncalibrated on the candidate edge contributes the conservative upper
    /// bound `1.0`. This is a placement-ranking penalty, not a measured error
    /// rate or circuit-success estimate.
    pub two_qubit_error_weight: f64,
    /// Weight for known readout error rates.
    ///
    /// Readout error is scaled by logical activity. Interaction-free circuits
    /// assign activity `1.0` to each logical qubit so idle layouts can still be
    /// ranked by readout fidelity.
    pub readout_error_weight: f64,
}

impl LayoutObjective {
    /// Creates a topology-only objective.
    ///
    /// This ranks by interaction distance and directed-coupling mismatch only,
    /// ignoring all calibration data even if the target device provides it.
    pub fn topology_only() -> Self {
        Self {
            distance_weight: 1.0,
            direction_weight: 1.0,
            two_qubit_error_weight: 0.0,
            readout_error_weight: 0.0,
        }
    }

    /// Creates a fidelity-aware objective when useful calibration data exists.
    ///
    /// If the physical graph has no usable error-rate data, this falls back to
    /// [`LayoutObjective::topology_only`] instead of failing.
    pub fn auto_from_physical(physical: &PhysicalLayoutGraph) -> Self {
        let mut objective = Self::topology_only();
        if physical.has_two_qubit_error_data() {
            objective.two_qubit_error_weight = 10.0;
        }
        if physical.has_readout_error_data() {
            objective.readout_error_weight = 1.0;
        }
        objective
    }

    /// Creates a fidelity-aware objective and requires calibration data.
    ///
    /// Use this for workflows where silently falling back to topology-only
    /// scoring would hide a device-data configuration problem.
    pub fn fidelity_required(physical: &PhysicalLayoutGraph) -> Result<Self, CompilerError> {
        if physical.has_fidelity_data() {
            Ok(Self::auto_from_physical(physical))
        } else {
            Err(CompilerError::InvalidInput(
                "layout fidelity scoring was requested but the device has no usable fidelity data"
                    .to_string(),
            ))
        }
    }

    /// Creates the default fidelity-aware objective.
    ///
    /// The default weights keep topology distance and direction mismatch active
    /// while making known two-qubit error rates a stronger tie-breaker than
    /// readout error.
    pub fn fidelity_aware() -> Self {
        Self {
            distance_weight: 1.0,
            direction_weight: 1.0,
            two_qubit_error_weight: 10.0,
            readout_error_weight: 1.0,
        }
    }

    /// Returns whether any fidelity/error term can affect scoring.
    pub fn uses_fidelity(&self) -> bool {
        self.two_qubit_error_weight != 0.0 || self.readout_error_weight != 0.0
    }

    /// Scores a candidate layout against circuit and physical-graph analysis.
    ///
    /// The layout must map every logical qubit in `analysis`. Interactions
    /// whose mapped physical qubits are disconnected are rejected because the
    /// resulting mapping cannot be routed within the usable topology.
    ///
    /// # Errors
    ///
    /// Returns [`CompilerError::InvalidInput`] for invalid objective weights or
    /// disconnected mapped interactions. Returns
    /// [`CompilerError::InvariantViolation`] if the layout omits a logical
    /// qubit present in the analysis.
    pub fn score_layout(
        &self,
        analysis: &CircuitLayoutAnalysis,
        physical: &PhysicalLayoutGraph,
        layout: &Layout,
    ) -> Result<LayoutScore, CompilerError> {
        validate_objective_weight(self.distance_weight, "distance_weight")?;
        validate_objective_weight(self.direction_weight, "direction_weight")?;
        validate_objective_weight(self.two_qubit_error_weight, "two_qubit_error_weight")?;
        validate_objective_weight(self.readout_error_weight, "readout_error_weight")?;

        let mut distance = 0.0;
        let mut direction = 0.0;
        let mut two_qubit_error = 0.0;

        for (interaction_index, interaction) in
            analysis.interactions.interactions().iter().enumerate()
        {
            let left_physical = layout.get_physical(interaction.left).ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "layout does not map logical qubit {}",
                    interaction.left
                ))
            })?;
            let right_physical = layout.get_physical(interaction.right).ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "layout does not map logical qubit {}",
                    interaction.right
                ))
            })?;
            let Some(pair_distance) = physical.distance(left_physical, right_physical) else {
                return Err(CompilerError::InvalidInput(format!(
                    "physical qubits {} and {} are disconnected in the usable topology",
                    left_physical, right_physical
                )));
            };

            distance += interaction.weight * f64::from(pair_distance);

            if pair_distance == 1 {
                // Direction and two-qubit error are meaningful only for a
                // concrete adjacent hardware edge. Non-adjacent interactions
                // are handled through the distance term and later routing.
                let left_index = physical.physical_index(left_physical).ok_or_else(|| {
                    CompilerError::InvariantViolation(format!(
                        "layout mapped {} outside the physical target",
                        left_physical
                    ))
                })?;
                let right_index = physical.physical_index(right_physical).ok_or_else(|| {
                    CompilerError::InvariantViolation(format!(
                        "layout mapped {} outside the physical target",
                        right_physical
                    ))
                })?;
                let adjacent = score_adjacent_interaction(
                    analysis.interactions.gate_contributions(interaction_index),
                    left_index,
                    right_index,
                    physical,
                );
                direction += adjacent.direction;
                if self.two_qubit_error_weight != 0.0 {
                    two_qubit_error += adjacent.effective_two_qubit_error;
                }
            }
        }

        let readout_error = if self.readout_error_weight == 0.0 {
            0.0
        } else {
            score_readout_error(analysis, physical, layout)?
        };

        let total = self.distance_weight * distance
            + self.direction_weight * direction
            + self.two_qubit_error_weight * two_qubit_error
            + self.readout_error_weight * readout_error;

        Ok(LayoutScore {
            total,
            distance,
            direction,
            two_qubit_error,
            readout_error,
            used_fidelity: self.uses_fidelity(),
        })
    }
}

impl Default for LayoutObjective {
    fn default() -> Self {
        Self::topology_only()
    }
}

/// Breakdown of one layout score.
///
/// Component fields are unweighted by the objective, but include the logical
/// interaction weights already present in the circuit analysis. The
/// [`LayoutScore::total`] field applies the objective weights.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutScore {
    /// Weighted sum according to [`LayoutObjective`].
    pub total: f64,
    /// Raw weighted-distance component.
    pub distance: f64,
    /// Raw direction-mismatch component.
    pub direction: f64,
    /// Unweighted effective two-qubit placement cost.
    ///
    /// In addition to known gate-specific error probabilities, this includes a
    /// conservative `1.0` cost for native gates that are unsupported or lack
    /// calibration on the selected physical edge. The value is accumulated
    /// over weighted logical interactions and may exceed `1.0`; it is not a
    /// measured error rate and must not be used for calibration reporting or
    /// circuit-success estimation.
    pub two_qubit_error: f64,
    /// Raw readout error component.
    pub readout_error: f64,
    /// Whether the objective was configured to use fidelity terms.
    pub used_fidelity: bool,
}

/// Computes the raw readout-error component for a complete layout.
///
/// Logical qubit activity scales the contribution of each mapped physical
/// readout error. Interaction-free circuits use one unit of activity per
/// logical qubit so readout data can still rank idle mappings.
fn score_readout_error(
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    layout: &Layout,
) -> Result<f64, CompilerError> {
    let mut activity = analysis.interactions.logical_activity();
    if activity.is_empty() {
        // Without two-qubit interactions there is no natural activity signal.
        // Assigning one unit to each logical qubit lets readout calibration
        // still rank otherwise equivalent interaction-free layouts.
        activity = analysis
            .logical_qubits
            .iter()
            .copied()
            .map(|logical| (logical, 1.0))
            .collect::<BTreeMap<_, _>>();
    }

    let mut readout_error = 0.0;
    for logical in &analysis.logical_qubits {
        let physical_qubit = layout.get_physical(*logical).ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "layout does not map logical qubit {logical}"
            ))
        })?;
        if let Some(error) = physical.readout_error(physical_qubit) {
            readout_error += activity.get(logical).copied().unwrap_or(1.0) * error;
        }
    }
    Ok(readout_error)
}

/// Validates that one objective weight can participate in total ordering.
fn validate_objective_weight(value: f64, name: &str) -> Result<(), CompilerError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(CompilerError::InvalidInput(format!(
            "layout objective {name} must be finite and non-negative"
        )))
    }
}
