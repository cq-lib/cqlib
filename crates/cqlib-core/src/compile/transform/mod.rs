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

//! Circuit transforms used by the compiler workflow.
//!
//! This module contains reusable compiler algorithms that transform, analyze,
//! place, or route a [`Circuit`](crate::circuit::Circuit). It is an algorithm
//! boundary rather than a workflow manager: callers may compose individual
//! transforms directly, while the public [`compile`](crate::compile::compile)
//! entry point decides when each completed transform runs in the staged
//! pipeline.
//!
//! # Transform Contract
//!
//! Circuit-to-circuit passes implement [`Transformer`]. A transformer takes an
//! immutable circuit reference and returns a [`TransformOutcome`].
//! [`TransformOutcome::Unchanged`] retains the input IR exactly as-is, while
//! [`TransformOutcome::Changed`] carries its replacement. Callers should not
//! pre-scan a circuit to infer whether a transform should run; the transform
//! itself owns traversal of any operation forms it supports, including
//! structured classical-control bodies.
//!
//! Layout and routing algorithms expose richer result types because they
//! return placement scores, final layouts, SWAP counts, and routing
//! diagnostics. They are still transform-layer algorithms and do not make
//! workflow policy decisions such as target-basis selection.
//!
//! # Module Roles
//!
//! - [`canonicalize`] validates and normalizes the compiler IR without doing
//!   semantic optimization or hardware lowering.
//! - [`commutative_cancellation`] cancels self-inverse gate pairs through a
//!   global, unbounded commutation-set analysis.
//! - [`decompose`] expands circuit-backed definitions, synthesizes
//!   matrix-backed unitaries, and lowers multi-controlled gates.
//! - [`device_lowering`] maps routed physical gates to exact ordered device
//!   instructions, including direction and local capability overrides.
//! - [`rewrite`] applies compiler knowledge rules for conservative
//!   optimization or explicit target-basis lowering.
//! - [`layout`] selects an initial logical-to-physical mapping for a device but
//!   does not insert SWAPs.
//! - [`routing`] rebuilds a physical circuit from an automatic or supplied
//!   initial layout and inserts SWAPs as needed.
//!
//! Direct use of this module is appropriate for tests, diagnostics, and custom
//! pipelines. Most users should start with [`compile`](crate::compile::compile)
//! so target constraints, resources, routing, and output canonicalization stay
//! ordered consistently.

pub mod analysis;
pub mod canonicalize;
pub mod commutative_cancellation;
pub mod decompose;
pub mod device_lowering;
pub mod layout;
pub(crate) mod lowering_support;
pub(crate) mod native_optimization;
pub mod one_qubit_optimization;
pub mod rebuild;
pub mod resynthesis;
pub mod rewrite;
pub mod routing;
pub mod routing_basis;
pub mod target_basis;
pub mod transformer;
pub mod virtual_permutation;

pub use analysis::CircuitAnalysis;
pub use canonicalize::{
    CanonicalizeConfig, CanonicalizeResult, Canonicalizer, canonicalize_circuit,
};
pub use commutative_cancellation::CommutativeCancellation;
pub use device_lowering::DeviceLowerer;
pub use layout::{
    CircuitLayoutAnalysis, Interaction, InteractionGraph, LayoutDiagnostics, LayoutObjective,
    LayoutResult, LayoutScore, PreparedSabreCircuit, PreparedSabreDeviceTarget, Vf2EdgeRequirement,
    Vf2LayoutConfig, analyze_circuit_for_layout, greedy_layout, greedy_layout_prepared,
    prepare_sabre_circuit, prepare_sabre_device_target, sabre_layout, sabre_layout_prepared,
    trivial_layout, trivial_layout_prepared, vf2_perfect_layout, vf2_perfect_layout_prepared,
};
pub use one_qubit_optimization::OptimizeOneQubitRuns;
pub use resynthesis::{
    ResynthesizeTwoQubitBlocks, TwoQubitBlockResynthesisConfig, resynthesize_two_qubit_blocks,
};
#[cfg(test)]
pub(crate) use rewrite::KnowledgeRewriteWorksetStats;
pub use rewrite::{
    KnowledgeRewriteResult, KnowledgeRewriteStats, KnowledgeRewriter, RewriteConfig, RewriteMode,
    rewrite_circuit,
};
pub(crate) use rewrite::{
    KnowledgeRewriteSession, OperationReplacement, QubitBijection, RewriteEdits,
};
pub use routing::{RoutedCircuit, SabreRouteResult, route_sabre, route_with_layout};
pub(crate) use routing::{route_sabre_tracked, route_with_layout_tracked};
pub use routing_basis::LowerToRoutingBasis;
pub use target_basis::{
    TargetBasisCost, TargetBasisCostModel, TargetBasisLowerer, TargetBasisSignature,
};
#[cfg(test)]
pub(crate) use transformer::{ResolvedTransform, TransformerTestExt, resolve_transform_for_test};
pub use transformer::{TransformOutcome, Transformer};
pub use virtual_permutation::{
    VirtualPermutation, VirtualPermutationElisionResult, VirtualPermutationElisionStatus,
    elide_virtual_permutations,
};
