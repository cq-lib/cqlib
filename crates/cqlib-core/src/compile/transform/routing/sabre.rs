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

//! SABRE routing transform.
//!
//! This module adapts the compiler SABRE core into transform-level routing
//! entry points. The algorithm implementation remains in
//! [`crate::compile::sabre`]; this layer owns only the public result types
//! and the high-level `route_sabre` / `route_with_layout` APIs.
//!
//! # Two entry points
//!
//! - [`route_with_layout`] — route with a caller-supplied initial layout.
//!   Returns [`RoutedCircuit`], a pure routing result without layout metadata.
//! - [`route_sabre`] — run SABRE layout refinement then routing in one call.
//!   Returns [`SabreRouteResult`], which wraps a [`RoutedCircuit`] and adds
//!   the layout score.

use crate::circuit::{Circuit, CircuitParam, Parameter, Qubit};
use crate::compile::CompilerError;
use crate::compile::device_planning::DevicePlanningSession;
use crate::compile::sabre::{
    RouteOperationProvenance, SabreConfig, SabreRoutingDiagnostics, SabreRoutingResult,
    finish_sabre_route, sabre_route as sabre_route_core, sabre_route_with_provenance,
    sabre_route_with_provenance_and_session,
};
use crate::compile::transform::layout::{
    LayoutDiagnostics, LayoutObjective, LayoutScore, prepare_sabre_circuit,
    prepare_sabre_device_target, prepare_sabre_device_target_with_session,
    sabre_route_selection_prepared,
};
use crate::compile::transform::{QubitBijection, RewriteEdits};
use crate::device::{Device, Layout, LogicalQubit, PhysicalQubit};
use std::collections::HashMap;
use std::ops::Range;

/// A physical circuit produced by routing, plus routing metadata.
///
/// This is the result of routing from a caller-supplied layout. It does not
/// carry layout-selection metadata; callers that also need the layout score
/// should use [`route_sabre`] which returns [`SabreRouteResult`].
#[derive(Debug, Clone, PartialEq)]
pub struct RoutedCircuit {
    circuit: Circuit,
    initial_layout: Layout,
    final_layout: Layout,
    swap_count: usize,
    diagnostics: SabreRoutingDiagnostics,
}

impl RoutedCircuit {
    /// The routed physical circuit.
    pub fn circuit(&self) -> &Circuit {
        &self.circuit
    }

    /// Consumes `self` and returns the owned physical circuit.
    pub fn into_circuit(self) -> Circuit {
        self.circuit
    }

    /// The initial logical-to-physical layout used for routing.
    pub fn initial_layout(&self) -> &Layout {
        &self.initial_layout
    }

    /// The final logical-to-physical layout after all routed operations.
    pub fn final_layout(&self) -> &Layout {
        &self.final_layout
    }

    /// Number of inserted SWAP operations.
    pub fn swap_count(&self) -> usize {
        self.swap_count
    }

    /// Routing diagnostics (trials evaluated, fallback count, etc.).
    pub fn diagnostics(&self) -> &SabreRoutingDiagnostics {
        &self.diagnostics
    }

    /// Whether routing observably changed the original circuit.
    ///
    /// A route is considered changed when any of the following holds:
    /// - SWAP operations were inserted,
    /// - the physical qubit set differs from the input qubit set,
    /// - the global phase changed, or
    /// - a non-identity initial layout was selected.
    pub fn changed(&self, original: &Circuit) -> bool {
        if self.swap_count > 0
            || original.qubits() != self.circuit.qubits()
            || original.global_phase() != self.circuit.global_phase()
        {
            return true;
        }

        if !original.qubits().into_iter().all(|qubit| {
            self.initial_layout
                .get_physical(LogicalQubit::from_qubit(qubit))
                == Some(PhysicalQubit::from_qubit(qubit))
        }) {
            return true;
        }

        // Structural comparison of the operation streams. Short-circuits on
        // the first difference and avoids the O(circuit size) temporary
        // strings a `Debug`-format comparison would allocate.
        !original.operations_structurally_equal(&self.circuit)
    }
}

/// Routing result with source provenance retained only for compiler-workflow
/// proof transfer. Public routing results intentionally do not carry this
/// potentially circuit-sized metadata.
pub(crate) struct TrackedRoutedCircuit {
    routed: RoutedCircuit,
    provenance: Vec<RouteOperationProvenance>,
}

impl TrackedRoutedCircuit {
    pub(crate) fn routed(&self) -> &RoutedCircuit {
        &self.routed
    }

    pub(crate) fn into_routed(self) -> RoutedCircuit {
        self.routed
    }

    pub(crate) fn rewrite_edits(&self, original: &Circuit) -> RewriteEdits {
        let provenance = self
            .provenance
            .iter()
            .map(|entry| match entry {
                RouteOperationProvenance::Source(order) => Some(*order),
                RouteOperationProvenance::Inserted => None,
            })
            .collect::<Vec<_>>();
        let edits = RewriteEdits::from_route_provenance(original.operations().len(), &provenance);
        let RewriteEdits::Linear {
            old_len,
            new_len,
            replacements,
        } = edits
        else {
            return RewriteEdits::Unknown;
        };
        let Some(clean_gap_bijections) = routed_clean_gap_bijections(
            original,
            self.routed.circuit(),
            &provenance,
            &replacements,
        ) else {
            return RewriteEdits::Unknown;
        };
        RewriteEdits::linear_modulo_qubit_bijection(
            old_len,
            new_len,
            replacements,
            clean_gap_bijections,
        )
    }
}

fn routed_clean_gap_bijections(
    original: &Circuit,
    routed: &Circuit,
    provenance: &[Option<usize>],
    replacements: &[crate::compile::transform::OperationReplacement],
) -> Option<Vec<QubitBijection>> {
    if provenance.len() != routed.operations().len() {
        return None;
    }
    let mut bijections = Vec::with_capacity(replacements.len().saturating_add(1));
    let mut old_cursor = 0usize;
    let mut new_cursor = 0usize;
    for replacement in replacements {
        bijections.push(routed_gap_bijection(
            original,
            routed,
            provenance,
            old_cursor..replacement.old.start,
            new_cursor..replacement.new.start,
        )?);
        old_cursor = replacement.old.end;
        new_cursor = replacement.new.end;
    }
    bijections.push(routed_gap_bijection(
        original,
        routed,
        provenance,
        old_cursor..original.operations().len(),
        new_cursor..routed.operations().len(),
    )?);
    Some(bijections)
}

fn routed_gap_bijection(
    original: &Circuit,
    routed: &Circuit,
    provenance: &[Option<usize>],
    old: Range<usize>,
    new: Range<usize>,
) -> Option<QubitBijection> {
    if old.len() != new.len() {
        return None;
    }
    let mut forward = HashMap::<Qubit, Qubit>::new();
    let mut reverse = HashMap::<Qubit, Qubit>::new();
    for (old_order, new_order) in old.zip(new) {
        if provenance.get(new_order).copied().flatten() != Some(old_order) {
            return None;
        }
        let source = &original.operations()[old_order];
        let target = &routed.operations()[new_order];
        if source.instruction != target.instruction
            || source.label != target.label
            || source.qubits.len() != target.qubits.len()
            || source.params.len() != target.params.len()
            || !source
                .params
                .iter()
                .zip(&target.params)
                .all(|(left, right)| resolve_param(original, left) == resolve_param(routed, right))
        {
            return None;
        }
        for (&logical, &physical) in source.qubits.iter().zip(&target.qubits) {
            if forward
                .insert(logical, physical)
                .is_some_and(|old| old != physical)
                || reverse
                    .insert(physical, logical)
                    .is_some_and(|old| old != logical)
            {
                return None;
            }
        }
    }
    let mut pairs = forward.into_iter().collect::<Vec<_>>();
    pairs.sort_by_key(|(source, target)| (*source, *target));
    Some(QubitBijection { pairs })
}

fn resolve_param(circuit: &Circuit, parameter: &CircuitParam) -> Option<Parameter> {
    match parameter {
        CircuitParam::Fixed(value) => Some(Parameter::from(*value)),
        CircuitParam::Index(index) => circuit.parameters().get_index(*index as usize).cloned(),
    }
}

/// Full SABRE pipeline result: layout selection + routing.
///
/// Returned by [`route_sabre`]. Wraps a [`RoutedCircuit`] and adds the layout
/// observed objective score so callers can inspect the winning layout.
#[derive(Debug, Clone, PartialEq)]
pub struct SabreRouteResult {
    routed: RoutedCircuit,
    layout_score: Option<LayoutScore>,
    layout_diagnostics: LayoutDiagnostics,
}

impl SabreRouteResult {
    /// The routed physical circuit and routing metadata.
    pub fn routed(&self) -> &RoutedCircuit {
        &self.routed
    }

    /// Consumes `self` and returns the owned [`RoutedCircuit`].
    pub fn into_routed(self) -> RoutedCircuit {
        self.routed
    }

    /// Observed score of the selected initial layout, when available.
    ///
    /// SABRE selects the winner by predicted native route quality; this score
    /// is diagnostic and is not the route-selection key.
    pub fn layout_score(&self) -> Option<&LayoutScore> {
        self.layout_score.as_ref()
    }

    /// Diagnostics produced while selecting the initial layout.
    pub fn layout_diagnostics(&self) -> &LayoutDiagnostics {
        &self.layout_diagnostics
    }

    // ── Transparent accessors for common routed fields ──

    /// The routed physical circuit.
    pub fn circuit(&self) -> &Circuit {
        self.routed.circuit()
    }

    /// Number of inserted SWAP operations.
    pub fn swap_count(&self) -> usize {
        self.routed.swap_count()
    }

    /// The initial layout used for routing.
    pub fn initial_layout(&self) -> &Layout {
        self.routed.initial_layout()
    }

    /// The final layout after routing.
    pub fn final_layout(&self) -> &Layout {
        self.routed.final_layout()
    }

    /// Routing diagnostics.
    pub fn diagnostics(&self) -> &SabreRoutingDiagnostics {
        self.routed.diagnostics()
    }

    /// Whether routing changed the original circuit.
    pub fn changed(&self, original: &Circuit) -> bool {
        self.routed.changed(original)
    }
}

struct SabreRoutingResultWithScore {
    routing: SabreRoutingResult,
    provenance: Vec<RouteOperationProvenance>,
    layout_score: Option<LayoutScore>,
    layout_diagnostics: LayoutDiagnostics,
}

fn sabre_layout_and_route(
    circuit: &Circuit,
    device: &Device,
    objective: &LayoutObjective,
    config: &SabreConfig,
    retain_provenance: bool,
) -> Result<SabreRoutingResultWithScore, CompilerError> {
    sabre_layout_and_route_with_session(circuit, device, objective, config, retain_provenance, None)
}

fn sabre_layout_and_route_with_session(
    circuit: &Circuit,
    device: &Device,
    objective: &LayoutObjective,
    config: &SabreConfig,
    retain_provenance: bool,
    planning_session: Option<&DevicePlanningSession>,
) -> Result<SabreRoutingResultWithScore, CompilerError> {
    let prepared = prepare_sabre_circuit(circuit)?;
    let prepared_target = if let Some(session) = planning_session {
        prepare_sabre_device_target_with_session(&prepared, device, session)?
    } else {
        prepare_sabre_device_target(&prepared, device)?
    };
    let selection = sabre_route_selection_prepared(
        &prepared,
        &prepared_target,
        objective,
        config,
        retain_provenance,
    )?;
    let routed = finish_sabre_route(
        circuit,
        prepared_target.routing_target(),
        selection.initial_layout,
        selection.trial,
        selection.selected_trial_index,
        selection.trials_evaluated,
    )?;
    Ok(SabreRoutingResultWithScore {
        routing: routed.routing,
        provenance: routed.provenance,
        layout_score: Some(selection.score),
        layout_diagnostics: selection.diagnostics,
    })
}

/// Routes `circuit` from a caller-supplied initial layout.
///
/// This is the low-level entry point for callers that already have a layout
/// (e.g. from VF2, greedy, or a previous SABRE run). It does not perform
/// layout refinement or scoring.
///
/// Use [`route_sabre`] if you need automatic layout selection.
pub fn route_with_layout(
    circuit: &Circuit,
    device: &Device,
    initial_layout: &Layout,
    config: &SabreConfig,
) -> Result<RoutedCircuit, CompilerError> {
    let routing = sabre_route_core(circuit, device, initial_layout, config)?;
    Ok(RoutedCircuit {
        circuit: routing.circuit,
        initial_layout: routing.initial_layout,
        final_layout: routing.final_layout,
        swap_count: routing.swap_count,
        diagnostics: routing.diagnostics,
    })
}

pub(crate) fn route_with_layout_tracked(
    circuit: &Circuit,
    device: &Device,
    initial_layout: &Layout,
    config: &SabreConfig,
) -> Result<TrackedRoutedCircuit, CompilerError> {
    let result = sabre_route_with_provenance(circuit, device, initial_layout, config)?;
    let routing = result.routing;
    Ok(TrackedRoutedCircuit {
        routed: RoutedCircuit {
            circuit: routing.circuit,
            initial_layout: routing.initial_layout,
            final_layout: routing.final_layout,
            swap_count: routing.swap_count,
            diagnostics: routing.diagnostics,
        },
        provenance: result.provenance,
    })
}

pub(crate) fn route_with_layout_tracked_with_session(
    circuit: &Circuit,
    device: &Device,
    initial_layout: &Layout,
    config: &SabreConfig,
    planning_session: &DevicePlanningSession,
) -> Result<TrackedRoutedCircuit, CompilerError> {
    let result = sabre_route_with_provenance_and_session(
        circuit,
        device,
        initial_layout,
        config,
        planning_session,
    )?;
    let routing = result.routing;
    Ok(TrackedRoutedCircuit {
        routed: RoutedCircuit {
            circuit: routing.circuit,
            initial_layout: routing.initial_layout,
            final_layout: routing.final_layout,
            swap_count: routing.swap_count,
            diagnostics: routing.diagnostics,
        },
        provenance: result.provenance,
    })
}

/// Selects a SABRE initial layout and routes `circuit` for `device`.
///
/// This function first runs SABRE layout refinement, then routes the original
/// forward circuit from the selected initial layout. The returned circuit is
/// rebuilt over usable physical qubit identifiers and includes inserted
/// [`StandardGate::SWAP`](crate::circuit::StandardGate::SWAP) operations when
/// the selected layout alone cannot satisfy the physical topology.
///
/// If you already have an initial layout, use [`route_with_layout`] to skip
/// the layout-selection step.
///
/// Equal deterministic seeds in [`SabreConfig`] produce equal cqlib routing
/// results for the same circuit and device.
///
/// # Limitations
///
/// This transform checks exact-qargs native-plan feasibility while routing, but
/// it does not emit those native plans or perform target-basis lowering. It also
/// does not select a compiler workflow. Callers should run required
/// decomposition, device lowering, and basis translation passes explicitly.
///
/// # Errors
///
/// Returns [`CompilerError::InvalidInput`] for invalid SABRE configuration,
/// insufficient usable physical qubits, unreachable routing requirements, or
/// unsupported circuit operations such as undecomposed gates that touch more
/// than two qubits. Bounded layout-search and native-feasibility failures are
/// returned as [`CompilerError::SabreRoutingFailed`].
pub fn route_sabre(
    circuit: &Circuit,
    device: &Device,
    objective: &LayoutObjective,
    config: &SabreConfig,
) -> Result<SabreRouteResult, CompilerError> {
    let result = sabre_layout_and_route(circuit, device, objective, config, false)?;

    Ok(SabreRouteResult {
        routed: RoutedCircuit {
            circuit: result.routing.circuit,
            initial_layout: result.routing.initial_layout,
            final_layout: result.routing.final_layout,
            swap_count: result.routing.swap_count,
            diagnostics: result.routing.diagnostics,
        },
        layout_score: result.layout_score,
        layout_diagnostics: result.layout_diagnostics,
    })
}

pub(crate) fn route_sabre_tracked(
    circuit: &Circuit,
    device: &Device,
    objective: &LayoutObjective,
    config: &SabreConfig,
) -> Result<TrackedRoutedCircuit, CompilerError> {
    let result = sabre_layout_and_route(circuit, device, objective, config, true)?;
    Ok(TrackedRoutedCircuit {
        routed: RoutedCircuit {
            circuit: result.routing.circuit,
            initial_layout: result.routing.initial_layout,
            final_layout: result.routing.final_layout,
            swap_count: result.routing.swap_count,
            diagnostics: result.routing.diagnostics,
        },
        provenance: result.provenance,
    })
}

pub(crate) fn route_sabre_tracked_with_session(
    circuit: &Circuit,
    device: &Device,
    objective: &LayoutObjective,
    config: &SabreConfig,
    planning_session: &DevicePlanningSession,
) -> Result<TrackedRoutedCircuit, CompilerError> {
    let result = sabre_layout_and_route_with_session(
        circuit,
        device,
        objective,
        config,
        true,
        Some(planning_session),
    )?;
    Ok(TrackedRoutedCircuit {
        routed: RoutedCircuit {
            circuit: result.routing.circuit,
            initial_layout: result.routing.initial_layout,
            final_layout: result.routing.final_layout,
            swap_count: result.routing.swap_count,
            diagnostics: result.routing.diagnostics,
        },
        provenance: result.provenance,
    })
}

#[cfg(test)]
#[path = "./sabre_test.rs"]
mod routing_test;
