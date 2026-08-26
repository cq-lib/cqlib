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

//! SABRE routing core.
//!
//! SABRE is a SWAP-based bidirectional heuristic search for mapping logical
//! qubits onto a device with limited connectivity and local instruction
//! capabilities. The algorithm incrementally routes unary and two-qubit
//! requirements, scores candidate SWAPs with current and lookahead distances
//! plus exact native-plan lower bounds, and selects the best routed circuit
//! from deterministic or seeded routing trials.
//!
//! This implementation follows the published SABRE structure and extends it
//! with cqlib-specific mechanisms: interaction-graph layout seeds, dense routing
//! state, incremental novelty-aware lookahead scoring, exact
//! movement/lowerability feasibility, native-2Q-aware local decisions,
//! multiplicative congestion control, release-valve fallback, compact trial
//! plans, parallel streaming selection, and control-flow layout restoration.
//! These mechanisms are implemented independently around cqlib's device planner
//! and routing contracts.
//!
//! This module is intentionally independent from compiler workflow selection.
//! It exposes reusable SABRE building blocks, but it does not decide whether a
//! workflow should prefer trivial, greedy, VF2, SABRE, or another layout and
//! routing strategy.
//!
//! # Reference
//!
//! Gushu Li, Yufei Ding, and Yuan Xie, "Tackling the Qubit Mapping Problem for
//! NISQ-Era Quantum Devices", ASPLOS 2019. DOI: 10.1145/3297858.3304023.
//! arXiv: 1809.02573.
//!
//! Shaohan Zou, Matthew Treinish, Kevin Hartman, Davide Ivrii, and John Lishman,
//! "LightSABRE: A Lightweight and Enhanced SABRE Algorithm", arXiv: 2409.08368,
//! 2024.
//!
//! # Entry Points
//!
//! - [`sabre_route`] routes a circuit from a supplied initial layout and returns
//!   a physical circuit with inserted SWAP operations, the final layout, and
//!   diagnostics.
//! - [`normalize_initial_layout`] validates and expands a caller-supplied
//!   layout against a device's usable physical qubits.
//! - [`validate_reachable_interactions`] performs the same device-aware
//!   movement and terminal preflight used before routing.
//!
//! # Example
//!
//! ```
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::compile::sabre::{SabreConfig, sabre_route};
//! use cqlib_core::device::{Device, Layout, LogicalQubit, PhysicalQubit, Topology};
//! use std::collections::{BTreeMap, HashSet};
//!
//! let physical = vec![
//!     PhysicalQubit::new(0),
//!     PhysicalQubit::new(1),
//!     PhysicalQubit::new(2),
//! ];
//! let topology = Topology::new(
//!     physical.clone(),
//!     vec![
//!         (PhysicalQubit::new(0), PhysicalQubit::new(1), "cx".to_string()),
//!         (PhysicalQubit::new(1), PhysicalQubit::new(2), "cx".to_string()),
//!     ],
//! )?;
//! let device = Device::new(
//!     "line3",
//!     physical.iter().copied().collect::<HashSet<_>>(),
//!     topology,
//! )?;
//!
//! let logical = vec![LogicalQubit::new(0), LogicalQubit::new(1)];
//! let mapping = BTreeMap::from([
//!     (LogicalQubit::new(0), PhysicalQubit::new(0)),
//!     (LogicalQubit::new(1), PhysicalQubit::new(2)),
//! ]);
//! let layout = Layout::new(logical, physical, Some(mapping))?;
//!
//! let mut circuit = Circuit::new(2);
//! circuit.cx(Qubit::new(0), Qubit::new(1))?;
//!
//! let config = SabreConfig {
//!     routing_trials: 1,
//!     seed: Some(7),
//!     ..SabreConfig::default()
//! };
//! let routed = sabre_route(&circuit, &device, &layout, &config)?;
//!
//! assert_eq!(routed.diagnostics.trials_evaluated, 1);
//! assert!(routed.swap_count <= 1);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod cost;
mod dag;
mod dense_layout;
mod heuristic;
mod layer;
mod routing;

pub(crate) use cost::MetricAvailability;
pub(crate) use dag::SabreDag;
pub use heuristic::{SabreConfig, SabreHeuristicConfig, SabreVf2PrepassConfig};
pub(crate) use routing::{
    ComponentAssignmentSearch, InteractionReachability, PreparedRouteMetadata, RankedTrial,
    RequirementReachabilityFailure, RouteOperationProvenance, RoutingTarget,
    TrackedSabreRoutingResult, TrialResult, compare_ranked_trials, finish_sabre_route,
    interaction_reachability_for_target_with_metadata, movement_component_assignment,
    normalize_initial_layout_for_target, refine_layout_with_metadata,
    route_ranked_trial_with_metadata, sabre_route_with_provenance_and_session_on_physical,
    sabre_route_with_provenance_on_target,
};
pub use routing::{SabreRoutingDiagnostics, SabreRoutingResult, sabre_route};
pub use routing::{normalize_initial_layout, validate_reachable_interactions};

#[cfg(test)]
#[path = "./sabre_test.rs"]
mod sabre_test;
