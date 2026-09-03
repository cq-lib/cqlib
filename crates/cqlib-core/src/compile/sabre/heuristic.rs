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

//! SABRE trial and SWAP-selection configuration.
//!
//! The router first evaluates candidate SWAPs with a structural distance score:
//!
//! ```text
//! score = basic_weight * sum(front_layer_distance)
//!       + sum_i lookahead_weights[i] * mean(lookahead_layer_i_distance)
//! score *= max(congestion[swap_left], congestion[swap_right])
//! ```
//!
//! Lower scores are preferred. Active-layer scaling keeps a broad lookahead
//! layer from overwhelming nearer routing requirements. Candidates in
//! a narrow structural window are compared by exact-lowerability-aware
//! predicted native two-qubit cost when the target provides native plans.
//! Exact score ties are sampled from the seeded trial generator.
//!
//! The front layer contains unary and two-qubit routing requirements blocked by
//! their current placement. Lookahead layers bias decisions toward requirements
//! that become relevant soon. Repeated interactions remain represented in their
//! dependency layers because they express future placement locality. Keeping
//! the front as a sum preserves its weight as the front grows.
//!
//! Congestion control is optional. Reusing either endpoint of a candidate SWAP
//! multiplies its entire structural score, discouraging serial movement through
//! the same physical region and allowing independent movement to proceed in
//! parallel.
//!
use crate::compile::CompilerError;

/// Bounded VF2 prepass used to seed SABRE layout candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SabreVf2PrepassConfig {
    /// Maximum number of complete perfect mappings scored by VF2.
    pub candidate_limit: usize,
    /// Maximum number of partial mapping extensions attempted by VF2.
    pub call_limit: usize,
}

/// Configuration shared by SABRE layout refinement and routing.
#[derive(Debug, Clone, PartialEq)]
pub struct SabreConfig {
    /// Number of randomized starting layouts added to deterministic and
    /// interaction-aware layout candidates.
    pub layout_trials: usize,
    /// Maximum component-assignment states explored before layout reports
    /// budget exhaustion. Exhaustion is distinct from proven infeasibility.
    pub layout_assignment_budget: usize,
    /// Optional bounded VF2 prepass used to add a topology-perfect candidate.
    /// `None` disables the prepass.
    pub vf2_prepass: Option<SabreVf2PrepassConfig>,
    /// Number of forward/backward refinement iterations per layout trial.
    pub refinement_iterations: usize,
    /// Number of complete routing trials run for each fully refined layout.
    ///
    /// The search directly returns the best route and does not route
    /// intermediate refinement states or run a separate layout-scoring phase.
    pub routing_trials: usize,
    /// Optional deterministic seed.  Equal seeds produce equal cqlib results.
    pub seed: Option<u64>,
    /// Swap-selection heuristic configuration.
    pub heuristic: SabreHeuristicConfig,
}

/// Swap-selection heuristic used by SABRE.
///
/// The primary score combines the current front-layer sum, active-layer-scaled
/// lookahead, and multiplicative congestion. Exact native 2Q cost resolves
/// candidates in a narrow structural window when native plans are available.
#[derive(Debug, Clone, PartialEq)]
pub struct SabreHeuristicConfig {
    /// Weight of the current front-layer total distance.
    pub basic_weight: f64,
    /// Weights of each active-layer-scaled lookahead-layer total.
    pub lookahead_weights: Vec<f64>,
    /// Amount added to a physical qubit's congestion multiplier after using it
    /// in a heuristic SWAP. `None` disables congestion control.
    pub decay_increment: Option<f64>,
    /// Number of heuristic SWAP attempts before decay values reset.
    pub decay_reset: usize,
    /// Number of heuristic SWAPs allowed without routing a front-layer node
    /// before SABRE falls back to a shortest-path escape.
    pub attempt_limit: usize,
    /// Floating-point tolerance for treating candidate SWAP scores as tied.
    pub best_epsilon: f64,
}

impl Default for SabreHeuristicConfig {
    fn default() -> Self {
        Self {
            basic_weight: 1.0,
            lookahead_weights: vec![0.5, 0.25, 0.125, 0.0625, 0.03125],
            decay_increment: Some(0.002),
            decay_reset: 10,
            attempt_limit: 1000,
            best_epsilon: 1e-10,
        }
    }
}

impl Default for SabreConfig {
    fn default() -> Self {
        Self {
            layout_trials: 10,
            layout_assignment_budget: 1_000_000,
            vf2_prepass: Some(SabreVf2PrepassConfig {
                candidate_limit: 10,
                call_limit: 1_000_000,
            }),
            refinement_iterations: 1,
            routing_trials: 1,
            seed: None,
            heuristic: SabreHeuristicConfig::default(),
        }
    }
}

impl SabreConfig {
    /// Returns a compact deterministic SABRE configuration for reproducible tests and examples.
    ///
    /// This keeps all trial counts small, fixes the random seed, and uses a
    /// bounded swap-attempt limit so small fixtures run quickly while still
    /// exercising the SABRE routing path.
    pub fn deterministic_seeded(seed: u64) -> Self {
        Self {
            layout_trials: 2,
            layout_assignment_budget: 100_000,
            vf2_prepass: Some(SabreVf2PrepassConfig {
                candidate_limit: 10,
                call_limit: 100_000,
            }),
            refinement_iterations: 1,
            routing_trials: 1,
            seed: Some(seed),
            heuristic: SabreHeuristicConfig {
                lookahead_weights: vec![0.5, 0.25, 0.125, 0.0625, 0.03125],
                attempt_limit: 20,
                ..SabreHeuristicConfig::default()
            },
        }
    }

    /// Validates the routing fields used by SABRE.
    ///
    /// This check intentionally ignores layout-refinement fields such as
    /// [`SabreConfig::layout_trials`].
    /// Routing starts from a concrete initial layout and does not depend on
    /// those layout-only knobs.
    pub fn validate(&self) -> Result<(), CompilerError> {
        if self.routing_trials == 0 {
            return Err(CompilerError::InvalidInput(
                "sabre routing_trials must be greater than zero".to_string(),
            ));
        }
        self.heuristic.validate()
    }
}

impl SabreHeuristicConfig {
    pub(crate) fn validate(&self) -> Result<(), CompilerError> {
        validate_weight(self.basic_weight, "sabre basic_weight")?;
        for (index, weight) in self.lookahead_weights.iter().copied().enumerate() {
            validate_weight(weight, &format!("sabre lookahead_weights[{index}]"))?;
        }
        if let Some(increment) = self.decay_increment {
            validate_weight(increment, "sabre decay_increment")?;
            if self.decay_reset == 0 {
                return Err(CompilerError::InvalidInput(
                    "sabre decay_reset must be greater than zero when decay is enabled".to_string(),
                ));
            }
        }
        if !(self.best_epsilon.is_finite() && self.best_epsilon >= 0.0) {
            return Err(CompilerError::InvalidInput(
                "sabre best_epsilon must be finite and non-negative".to_string(),
            ));
        }
        Ok(())
    }
}

fn validate_weight(value: f64, name: &str) -> Result<(), CompilerError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(CompilerError::InvalidInput(format!(
            "{name} must be finite and non-negative"
        )))
    }
}

#[cfg(test)]
#[path = "heuristic_test.rs"]
mod heuristic_test;
