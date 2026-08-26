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

//! Initial-layout algorithms for quantum compilation.
//!
//! A layout maps each logical circuit qubit to one usable physical qubit on a
//! target device. This module only chooses that initial mapping. It does not
//! insert SWAP operations, rewrite instructions, or produce a fully routed
//! physical circuit; routing is handled by later compiler stages such as the
//! SABRE routing core.
//!
//! The shared pipeline is:
//!
//! 1. [`analyze_circuit_for_layout`] extracts weighted logical interactions
//!    from a [`Circuit`](crate::circuit::Circuit).
//! 2. [`PhysicalLayoutGraph::from_device`] turns a [`Device`](crate::device::Device)
//!    into a compiler-local physical topology and calibration view.
//! 3. A concrete algorithm, such as [`trivial_layout`], [`greedy_layout`],
//!    [`vf2_perfect_layout`], or [`sabre_layout`], returns a [`LayoutResult`].
//!
//! Mixed quantum/classical circuits are analyzed structurally. Layout analysis
//! recursively visits `if`, `while`, `for`, and `switch` bodies so interactions
//! hidden behind classical control still influence the initial mapping. It does
//! not estimate branch probabilities, loop trip counts, or runtime paths.
//!
//! # Examples
//!
//! ```rust
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::compile::transform::{LayoutObjective, trivial_layout};
//! use cqlib_core::device::Device;
//!
//! let mut circuit = Circuit::new(2);
//! circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
//! let device = Device::line("line-2", 2).unwrap();
//!
//! let result = trivial_layout(&circuit, &device, &LayoutObjective::topology_only()).unwrap();
//! assert!(result.score.is_some());
//! ```

mod analysis;
mod greedy;
mod interaction_seed;
mod objective;
mod result;
mod sabre;
mod scoring;
mod trivial;
mod vf2;
mod vf2_engine;

pub use crate::compile::physical_target::{DistanceTable, PhysicalLayoutGraph};
pub use analysis::{
    CircuitLayoutAnalysis, Interaction, InteractionGraph, analyze_circuit_for_layout,
};
pub(crate) use greedy::{GreedyCandidateOutcome, greedy_layout_candidate_prepared};
pub use greedy::{greedy_layout, greedy_layout_prepared};
pub use objective::{LayoutObjective, LayoutScore};
pub use result::{LayoutDiagnostics, LayoutResult};
pub use sabre::{
    PreparedSabreCircuit, PreparedSabreTarget, prepare_sabre_circuit, prepare_sabre_device_target,
    prepare_sabre_topology_target, sabre_layout, sabre_layout_prepared,
};
pub(crate) use sabre::{prepare_sabre_device_target_with_session, sabre_route_selection_prepared};
pub use trivial::{trivial_layout, trivial_layout_prepared};
pub use vf2::{
    Vf2EdgeRequirement, Vf2LayoutConfig, vf2_perfect_layout, vf2_perfect_layout_prepared,
};
pub(crate) use vf2::{Vf2PreparedOutcome, try_vf2_perfect_layout_prepared};

/// Returns whether a layout realizes all positive-weight interactions directly.
///
/// This shared helper is intentionally topology-only. Direction mismatch and
/// calibration quality are represented by [`LayoutObjective`] rather than by
/// the perfect-layout diagnostic.
fn is_perfect_layout(
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    layout: &crate::device::Layout,
) -> bool {
    // "Perfect" is a topology property: every positive-weight logical
    // interaction is already adjacent on the device. Coupling direction and
    // calibration quality are intentionally scored elsewhere.
    analysis
        .interactions
        .interactions()
        .iter()
        .filter(|interaction| interaction.weight > 0.0)
        .all(|interaction| {
            let Some(left) = layout.get_physical(interaction.left) else {
                return false;
            };
            let Some(right) = layout.get_physical(interaction.right) else {
                return false;
            };
            physical.is_adjacent_undirected(left, right)
        })
}

#[cfg(test)]
mod greedy_test;

#[cfg(test)]
mod gate_aware_test;

#[cfg(test)]
mod layout_test;

#[cfg(test)]
mod sabre_test;

#[cfg(test)]
mod vf2_test;
