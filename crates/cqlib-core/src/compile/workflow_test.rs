// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::{
    CompilerWorkflow, DeviceSynthesisPlacement, PreparedTargetBasis, RewritePhase,
    WorkflowResynthesisSession, WorkflowState, pareto_exploration_error_is_recoverable,
    sabre_config_for_mode,
};
use crate::circuit::gate::FrozenCircuit;
use crate::circuit::{
    Circuit, CircuitGate, CircuitParam, Instruction, MCGate, Parameter, ParameterValue, Qubit,
    StandardGate, UnitaryGate,
};
use crate::compile::knowledge::library::RuleKind;
use crate::compile::resource::ResourcePolicy;
use crate::compile::sabre::RoutingTarget;
use crate::compile::test_utils::{
    assert_compiled_circuit_equivalent, contains_high_level_gate, standard_ops, two_qubit_device,
};
use crate::compile::transform::analysis::WorkflowCircuitAnalysis;
use crate::compile::transform::decompose::unitary::TwoQubitSynthesisTarget;
use crate::compile::transform::layout::PhysicalLayoutGraph;
use crate::compile::transform::{
    CanonicalizeConfig, Canonicalizer, KnowledgeRewriteDiagnostics, KnowledgeRewriteSession,
    KnowledgeRewriter, OperationReplacement, OptimizeOneQubitRuns, RewriteConfig, RewriteEdits,
    TransformOutcome, route_with_layout_tracked_on_topology,
};
use crate::compile::{
    CompileConfig, CompileMode, CompileTarget, CompilerError, DeviceCompileTarget,
    SabreRoutingFailure, compile,
};
use crate::device::{Device, EdgeProp, InstructionProp, Layout, LogicalQubit, PhysicalQubit};
use ndarray::array;
use num_complex::Complex64;
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::sync::{Arc, Barrier};

fn compile_config(mode: CompileMode) -> CompileConfig {
    CompileConfig {
        mode,
        target: CompileTarget::Logical,
        resource_policy: ResourcePolicy::default(),
    }
}

fn workflow_analysis(circuit: &Circuit) -> Option<WorkflowCircuitAnalysis> {
    Some(WorkflowCircuitAnalysis::analyze(circuit))
}

fn run_workflow(circuit: &Circuit, mode: CompileMode) -> super::CompileResult {
    CompilerWorkflow::new(compile_config(mode))
        .run(circuit)
        .unwrap()
}

fn workflow_state_with_target_basis(target_basis: Vec<Instruction>) -> WorkflowState {
    let current = Circuit::new(1);
    let prepared_target_basis = PreparedTargetBasis::new(target_basis).unwrap();
    let two_qubit_target =
        TwoQubitSynthesisTarget::from_cost_model(Arc::clone(&prepared_target_basis.cost_model));
    let one_qubit_optimizer = Some(OptimizeOneQubitRuns::basis_with_cost_model(Arc::clone(
        &prepared_target_basis.cost_model,
    )));
    WorkflowState {
        analysis: workflow_analysis(&current),
        current,
        circuit_revision: 0,
        canonical_proof: None,
        rewrite_session: KnowledgeRewriteSession::default(),
        rewrite_diagnostics: KnowledgeRewriteDiagnostics::default(),
        collect_rewrite_diagnostics: true,
        changed: false,
        steps: Vec::new(),
        prepared_target_basis: Some(Arc::new(prepared_target_basis)),
        two_qubit_target,
        virtual_permutation: None,
        device_metadata: None,
        one_qubit_optimizer,
        pending_one_qubit_resynthesis: false,
        resynthesis_session: WorkflowResynthesisSession::default(),
        routing_physical: None,
        topology_routing_target: None,
        planning_session: None,
        native_quality_checkpoint: None,
    }
}

fn workflow_state_without_target_basis() -> WorkflowState {
    let current = Circuit::new(1);
    WorkflowState {
        analysis: workflow_analysis(&current),
        current,
        circuit_revision: 0,
        canonical_proof: None,
        rewrite_session: KnowledgeRewriteSession::default(),
        rewrite_diagnostics: KnowledgeRewriteDiagnostics::default(),
        collect_rewrite_diagnostics: true,
        changed: false,
        steps: Vec::new(),
        prepared_target_basis: None,
        two_qubit_target: TwoQubitSynthesisTarget::unconstrained(),
        virtual_permutation: None,
        device_metadata: None,
        one_qubit_optimizer: Some(OptimizeOneQubitRuns::logical()),
        pending_one_qubit_resynthesis: false,
        resynthesis_session: WorkflowResynthesisSession::default(),
        routing_physical: None,
        topology_routing_target: None,
        planning_session: None,
        native_quality_checkpoint: None,
    }
}

#[test]
fn canonical_proof_is_config_exact_and_revision_scoped() {
    let q0 = Qubit::new(0);
    let canonicalizer = Canonicalizer::production();
    let mut state = workflow_state_without_target_basis();

    assert!(!state.has_canonical_proof(canonicalizer.config()));
    state
        .apply_canonicalize("test", "canonicalize.first", &canonicalizer)
        .unwrap();
    assert!(state.has_canonical_proof(canonicalizer.config()));
    assert!(!state.has_canonical_proof(&CanonicalizeConfig::new().fold_gphase(false)));

    state
        .apply_canonicalize("test", "canonicalize.reused", &canonicalizer)
        .unwrap();
    assert_eq!(state.steps.len(), 2);
    assert!(!state.steps[1].changed);

    state
        .apply_transform("test", "mutate", |circuit, _analysis| {
            let mut changed = circuit.clone();
            changed.x(q0)?;
            Ok(TransformOutcome::Changed(changed))
        })
        .unwrap();
    assert!(!state.has_canonical_proof(canonicalizer.config()));
}

#[test]
fn workflow_rewrite_diagnostics_are_opt_in_and_semantics_neutral() {
    let mut circuit = Circuit::new(1);
    circuit.rz(Qubit::new(0), 0.25).unwrap();
    let workflow = CompilerWorkflow::new(compile_config(CompileMode::Normal));

    let normal = workflow.run(&circuit).unwrap();
    let (observed, diagnostics) = workflow.run_with_rewrite_diagnostics(&circuit).unwrap();
    let (owned, owned_diagnostics) = workflow
        .run_owned_with_rewrite_diagnostics(circuit)
        .unwrap();

    assert_eq!(normal, observed);
    assert_eq!(normal, owned);
    assert_eq!(diagnostics, owned_diagnostics);
    assert!(diagnostics.direct_reuses > 0);
}

#[test]
fn apply_transform_preserves_current_storage_on_unchanged() {
    let mut state = workflow_state_without_target_basis();
    state.current.h(Qubit::new(0)).unwrap();
    state.analysis = workflow_analysis(&state.current);
    let operation_ptr = state.current.operations().as_ptr();
    let analysis = state.analysis.clone();

    let changed = state
        .apply_transform("test", "unchanged", |_circuit, _analysis| {
            Ok(TransformOutcome::Unchanged)
        })
        .unwrap();

    assert!(!changed);
    assert_eq!(state.current.operations().as_ptr(), operation_ptr);
    assert_eq!(state.analysis, analysis);
    assert!(!state.changed);
    assert_eq!(state.steps.len(), 1);
    assert!(!state.steps[0].changed);
}

#[test]
fn apply_transform_adopts_changed_circuit_and_invalidates_analysis() {
    let mut state = workflow_state_without_target_basis();
    let mut replacement = Circuit::new(1);
    replacement.measure_bits([Qubit::new(0)]).unwrap();

    let changed = state
        .apply_transform("test", "changed", |_circuit, _analysis| {
            Ok(TransformOutcome::Changed(replacement))
        })
        .unwrap();

    assert!(changed);
    assert!(state.changed);
    assert!(state.analysis.is_none());
    assert!(state.analysis().public().has_measurement);
    assert!(state.steps[0].changed);
}

#[test]
fn apply_transform_error_leaves_workflow_state_untouched() {
    let mut state = workflow_state_without_target_basis();
    state.current.h(Qubit::new(0)).unwrap();
    state.analysis = workflow_analysis(&state.current);
    let operation_ptr = state.current.operations().as_ptr();
    let analysis = state.analysis.clone();

    let error = state
        .apply_transform("test", "error", |_circuit, _analysis| {
            Err(CompilerError::InvariantViolation(
                "expected failure".to_string(),
            ))
        })
        .unwrap_err();

    assert!(matches!(error, CompilerError::InvariantViolation(_)));
    assert_eq!(state.current.operations().as_ptr(), operation_ptr);
    assert_eq!(state.analysis, analysis);
    assert!(!state.changed);
    assert!(state.steps.is_empty());
}

#[test]
fn rewrite_session_reuses_fixpoint_after_only_unchanged_steps() {
    let q0 = Qubit::new(0);
    let config = RewriteConfig::production();
    let mut state = workflow_state_without_target_basis();
    state.current.h(q0).unwrap();
    state.current.h(q0).unwrap();
    state.analysis = workflow_analysis(&state.current);

    assert!(
        state
            .apply_knowledge_rewrite("test", "rewrite.first", config.clone())
            .unwrap()
    );
    let revision_after_rewrite = state.circuit_revision;
    state
        .apply_transform("test", "stable", |_circuit, _analysis| {
            Ok(TransformOutcome::Unchanged)
        })
        .unwrap();
    assert_eq!(state.circuit_revision, revision_after_rewrite);

    let forced_full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(&state.current)
        .unwrap();
    assert!(
        !state
            .apply_knowledge_rewrite("test", "rewrite.second", config)
            .unwrap()
    );

    assert_eq!(state.current, forced_full.circuit);
    let report = state.steps.last().unwrap();
    assert!(!report.changed);
    assert!(!report.skipped);
}

#[test]
fn workflow_post_decomposition_reuses_pre_decomposition_fixpoint() {
    let q0 = Qubit::new(0);
    let workflow = CompilerWorkflow::new(compile_config(CompileMode::Enhanced));
    let mut state = workflow_state_without_target_basis();
    state.current.h(q0).unwrap();
    state.current.h(q0).unwrap();
    state.analysis = workflow_analysis(&state.current);

    workflow.lower_init(&mut state).unwrap();
    workflow.lower_decompose(&mut state).unwrap();
    workflow.lower_optimize(&mut state).unwrap();

    let post = state
        .steps
        .iter()
        .find(|step| step.name == "optimize.post_decomposition")
        .unwrap();
    assert!(!post.changed);
    assert!(!post.skipped);
}

#[test]
fn target_cleanup_reuses_target_unaware_proof_when_circuit_is_already_physical() {
    let target_basis = vec![
        Instruction::Standard(StandardGate::RZ),
        Instruction::Standard(StandardGate::X2P),
        Instruction::Standard(StandardGate::CZ),
    ];
    let workflow = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::Basis(target_basis.clone()),
        resource_policy: ResourcePolicy::default(),
    });
    let mut state = workflow_state_with_target_basis(target_basis.clone());
    state.current.rz(Qubit::new(0), 0.25).unwrap();
    state.analysis = workflow_analysis(&state.current);
    let proof_config = workflow
        .rewrite_config(RewritePhase::PostDecomposition)
        .unwrap();
    state
        .apply_knowledge_rewrite("test", "rewrite.proof", proof_config.clone())
        .unwrap();
    let cleanup_config = workflow.target_cleanup_config(&state).unwrap().unwrap();

    assert!(
        !state
            .apply_knowledge_rewrite("test", "optimize.target_cleanup", cleanup_config)
            .unwrap()
    );
    assert!(
        state
            .rewrite_session
            .reusable_proof(state.circuit_revision, &proof_config)
            .is_some()
    );
    assert_eq!(state.rewrite_diagnostics.direct_reuses, 1);
}

#[test]
fn target_translation_keeps_cleanup_rewrite_local() {
    let target_basis = vec![
        Instruction::Standard(StandardGate::RZ),
        Instruction::Standard(StandardGate::X2P),
        Instruction::Standard(StandardGate::X),
        Instruction::Standard(StandardGate::CZ),
    ];
    let workflow = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::Basis(target_basis.clone()),
        resource_policy: ResourcePolicy::default(),
    });
    let mut state = workflow_state_with_target_basis(target_basis.clone());
    state.current = Circuit::new(512);
    for index in 0..512 {
        if index == 256 {
            state.current.h(Qubit::new(index)).unwrap();
        } else {
            state.current.x(Qubit::new(index)).unwrap();
        }
    }
    state.analysis = workflow_analysis(&state.current);
    let proof_config = workflow
        .rewrite_config(RewritePhase::PostDecomposition)
        .unwrap();
    state
        .apply_knowledge_rewrite("test", "rewrite.proof", proof_config)
        .unwrap();

    workflow.apply_target_translation(&mut state).unwrap();
    let cleanup_config = workflow.target_cleanup_config(&state).unwrap().unwrap();
    state
        .apply_knowledge_rewrite("test", "optimize.target_cleanup", cleanup_config)
        .unwrap();

    assert!(state.current.operations().iter().all(|operation| {
        target_basis.contains(&operation.instruction)
            || matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::GPhase)
            )
    }));
}

#[test]
fn target_cleanup_full_scans_when_target_cost_could_admit_new_candidates() {
    let q0 = Qubit::new(0);
    let proof_config = RewriteConfig::production();
    let cleanup_config = proof_config
        .clone()
        .with_target_instructions(vec![Instruction::Standard(StandardGate::X)])
        .unwrap();
    let mut state = workflow_state_without_target_basis();
    state.current.h(q0).unwrap();
    state.analysis = workflow_analysis(&state.current);
    state
        .apply_knowledge_rewrite("test", "rewrite.proof", proof_config)
        .unwrap();
    let forced_full = KnowledgeRewriter::new(cleanup_config.clone())
        .force_full_scan()
        .run(&state.current)
        .unwrap();

    state
        .apply_knowledge_rewrite("test", "optimize.target_cleanup", cleanup_config)
        .unwrap();

    assert_eq!(state.current, forced_full.circuit);
}

#[test]
fn rewrite_session_retains_proof_configuration_reach_after_config_shrink() {
    let proof_config = RewriteConfig::production().with_max_window_ops(16);
    let requested_config = proof_config.clone().with_max_window_ops(4);
    let proof_reach = KnowledgeRewriter::new(proof_config.clone())
        .proof_reach()
        .unwrap();
    let requested_reach = KnowledgeRewriter::new(requested_config.clone())
        .proof_reach()
        .unwrap();
    assert!(proof_reach > requested_reach);

    let mut state = workflow_state_without_target_basis();
    state
        .apply_knowledge_rewrite("test", "rewrite.proof", proof_config)
        .unwrap();
    state
        .apply_knowledge_rewrite("test", "rewrite.shrunk", requested_config.clone())
        .unwrap();

    assert_eq!(
        state
            .rewrite_session
            .reusable_proof(state.circuit_revision, &requested_config),
        Some(proof_reach)
    );
}

#[test]
fn changed_transform_invalidates_direct_rewrite_reuse() {
    let q0 = Qubit::new(0);
    let config = RewriteConfig::production();
    let mut state = workflow_state_without_target_basis();
    state
        .apply_knowledge_rewrite("test", "rewrite.proof", config.clone())
        .unwrap();
    let revision = state.circuit_revision;

    state
        .apply_transform_with_edits("test", "changed", |circuit, _analysis| {
            let mut changed = circuit.clone();
            changed.x(q0)?;
            Ok((
                TransformOutcome::Changed(changed),
                RewriteEdits::linear(
                    0,
                    1,
                    vec![OperationReplacement {
                        old: 0..0,
                        new: 0..1,
                    }],
                ),
            ))
        })
        .unwrap();
    assert_eq!(state.circuit_revision, revision + 1);
    let forced_full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(&state.current)
        .unwrap();
    state
        .apply_knowledge_rewrite("test", "rewrite.after_change", config)
        .unwrap();

    assert_eq!(state.current, forced_full.circuit);
}

fn large_flat_rewrite_circuit(change_middle: bool) -> Circuit {
    large_flat_rewrite_circuit_with_changes(if change_middle { &[2_101] } else { &[] })
}

fn large_flat_rewrite_circuit_with_changes(changes: &[usize]) -> Circuit {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    for index in 0..4_200 {
        match index % 4 {
            0 => circuit.t(q0).unwrap(),
            1 if changes.contains(&index) => circuit.t(q0).unwrap(),
            1 | 3 => circuit.h(q0).unwrap(),
            2 => circuit.tdg(q0).unwrap(),
            _ => unreachable!(),
        }
    }
    circuit
}

#[test]
fn rewrite_session_edit_reconciliation_matches_full_scan() {
    let proof_config = RewriteConfig::production()
        .with_enabled_kinds(vec![RuleKind::Cancel])
        .with_max_window_ops(16);
    let requested_config = proof_config
        .clone()
        .with_max_window_ops(4)
        .with_target_instructions(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::T),
            Instruction::Standard(StandardGate::TDG),
        ])
        .unwrap();
    let proof_reach = KnowledgeRewriter::new(proof_config.clone())
        .proof_reach()
        .unwrap();
    let requested_reach = KnowledgeRewriter::new(requested_config.clone())
        .proof_reach()
        .unwrap();
    assert!(proof_reach > requested_reach);
    let mut state = workflow_state_without_target_basis();
    state.current = large_flat_rewrite_circuit(false);
    state.analysis = workflow_analysis(&state.current);
    assert!(
        !state
            .apply_knowledge_rewrite("test", "rewrite.proof", proof_config)
            .unwrap()
    );

    let changed = large_flat_rewrite_circuit(true);
    let forced_full = KnowledgeRewriter::new(requested_config.clone())
        .force_full_scan()
        .run(&changed)
        .unwrap();
    state
        .apply_transform_with_edits("test", "sparse-change", |_circuit, _analysis| {
            Ok((
                TransformOutcome::Changed(changed),
                RewriteEdits::linear(
                    4_200,
                    4_200,
                    vec![OperationReplacement {
                        old: 2_101..2_102,
                        new: 2_101..2_102,
                    }],
                ),
            ))
        })
        .unwrap();
    assert_eq!(state.current.operations().len(), 4_200);
    assert_eq!(
        state
            .apply_knowledge_rewrite("test", "rewrite.incremental", requested_config)
            .unwrap(),
        forced_full.changed
    );

    assert_eq!(state.current, forced_full.circuit);
    assert!(state.rewrite_diagnostics.dirty_anchors > 0);
    assert!(state.rewrite_diagnostics.dirty_anchors < state.current.operations().len() / 2);
    assert_eq!(state.rewrite_diagnostics.full_scan_fallbacks, 0);
}

#[test]
fn one_qubit_optimization_edits_preserve_incremental_rewrite_results() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let config = RewriteConfig::production()
        .with_enabled_kinds(vec![RuleKind::Cancel])
        .with_max_window_ops(16);
    let workflow = CompilerWorkflow::new(compile_config(CompileMode::Normal));
    let mut state = workflow_state_without_target_basis();
    state.current = Circuit::new(2);
    for _ in 0..128 {
        state
            .current
            .append(
                Instruction::Standard(StandardGate::H),
                [q1],
                std::iter::empty(),
                Some("keep"),
            )
            .unwrap();
    }
    state.current.rx(q0, 0.2).unwrap();
    state.current.h(q1).unwrap();
    state.current.rz(q0, -0.3).unwrap();
    for _ in 0..128 {
        state
            .current
            .append(
                Instruction::Standard(StandardGate::H),
                [q1],
                std::iter::empty(),
                Some("keep"),
            )
            .unwrap();
    }
    state.analysis = workflow_analysis(&state.current);
    assert!(
        !state
            .apply_knowledge_rewrite("test", "rewrite.proof", config.clone())
            .unwrap()
    );

    assert!(
        workflow
            .apply_one_qubit_optimization(&mut state, "optimize.one_qubit_runs")
            .unwrap()
    );
    let forced_full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(&state.current)
        .unwrap();
    state
        .apply_knowledge_rewrite("test", "rewrite.incremental", config)
        .unwrap();

    assert_eq!(state.current, forced_full.circuit);
    assert_eq!(state.rewrite_diagnostics.full_scan_fallbacks, 0);
}

#[test]
fn rewrite_session_unknown_edits_force_a_full_scan() {
    let config = RewriteConfig::production()
        .with_enabled_kinds(vec![RuleKind::Cancel])
        .with_max_window_ops(16);
    let mut state = workflow_state_without_target_basis();
    state.current = large_flat_rewrite_circuit(false);
    state.analysis = workflow_analysis(&state.current);
    state
        .apply_knowledge_rewrite("test", "rewrite.proof", config.clone())
        .unwrap();

    let changed = large_flat_rewrite_circuit(true);
    let forced_full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(&changed)
        .unwrap();
    state
        .apply_transform_with_edits("test", "opaque-change", |_circuit, _analysis| {
            Ok((TransformOutcome::Changed(changed), RewriteEdits::Unknown))
        })
        .unwrap();
    state
        .apply_knowledge_rewrite("test", "rewrite.incremental", config)
        .unwrap();

    assert_eq!(state.current, forced_full.circuit);
    assert_eq!(state.rewrite_diagnostics.full_scan_fallbacks, 1);
}

#[test]
fn rewrite_session_composes_multiple_exact_transform_edits() {
    let config = RewriteConfig::production()
        .with_enabled_kinds(vec![RuleKind::Cancel])
        .with_max_window_ops(16);
    let mut state = workflow_state_without_target_basis();
    state.current = large_flat_rewrite_circuit(false);
    state.analysis = workflow_analysis(&state.current);
    state
        .apply_knowledge_rewrite("test", "rewrite.proof", config.clone())
        .unwrap();

    for (index, position) in [1_001usize, 3_001].into_iter().enumerate() {
        let changed = large_flat_rewrite_circuit_with_changes(&[1_001, 3_001][..=index]);
        state
            .apply_transform_with_edits("test", "exact-edit", |_circuit, _analysis| {
                Ok((
                    TransformOutcome::Changed(changed),
                    RewriteEdits::linear(
                        4_200,
                        4_200,
                        vec![OperationReplacement {
                            old: position..position + 1,
                            new: position..position + 1,
                        }],
                    ),
                ))
            })
            .unwrap();
    }
    let forced_full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(&state.current)
        .unwrap();
    state
        .apply_knowledge_rewrite("test", "rewrite.incremental", config)
        .unwrap();

    assert_eq!(state.current, forced_full.circuit);
    assert_eq!(state.rewrite_diagnostics.direct_reuses, 0);
    assert!(state.rewrite_diagnostics.dirty_anchors > 0);
    assert!(state.rewrite_diagnostics.dirty_anchors < state.current.operations().len() / 2);
}

#[test]
fn sabre_route_provenance_limits_post_routing_rewrite_work() {
    let q0 = Qubit::new(0);
    let q2 = Qubit::new(2);
    let mut source = Circuit::new(3);
    for index in 0..4_200 {
        if index == 2_101 {
            source.cx(q0, q2).unwrap();
        } else {
            match index % 4 {
                0 => source.t(q0).unwrap(),
                1 | 3 => source.h(q0).unwrap(),
                2 => source.tdg(q0).unwrap(),
                _ => unreachable!(),
            }
        }
    }
    let config = RewriteConfig::production()
        .with_enabled_kinds(vec![RuleKind::Cancel])
        .with_max_window_ops(16);
    let mut state = workflow_state_without_target_basis();
    state.current = source;
    state.analysis = workflow_analysis(&state.current);
    state
        .apply_knowledge_rewrite("test", "rewrite.pre_route", config.clone())
        .unwrap();

    let device = Device::line("rewrite-session-route", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::T),
            Instruction::Standard(StandardGate::TDG),
            Instruction::Standard(StandardGate::CX),
            Instruction::Standard(StandardGate::SWAP),
        ])
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1), (2, 2)], 3).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let routing_target = RoutingTarget::from_physical(&physical).unwrap();
    let routed = route_with_layout_tracked_on_topology(
        &state.current,
        &routing_target,
        &layout,
        &sabre_config_for_mode(CompileMode::Enhanced, Some(7)),
    )
    .unwrap();
    assert!(routed.routed().swap_count() > 0);
    let edits = routed.rewrite_edits(&state.current);
    let routed_circuit = routed.into_routed().into_circuit();
    let forced_full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(&routed_circuit)
        .unwrap();
    state
        .apply_transform_with_edits("test", "route", |_circuit, _analysis| {
            Ok((TransformOutcome::Changed(routed_circuit), edits))
        })
        .unwrap();
    state
        .apply_knowledge_rewrite("test", "rewrite.post_route", config)
        .unwrap();

    assert_eq!(state.current, forced_full.circuit);
}

#[test]
fn sabre_pure_layout_relabel_reuses_rewrite_proof_in_constant_time() {
    let config = RewriteConfig::production()
        .with_enabled_kinds(vec![RuleKind::Cancel])
        .with_max_window_ops(16);
    let mut state = workflow_state_without_target_basis();
    state.current = large_flat_rewrite_circuit(false);
    state.analysis = workflow_analysis(&state.current);
    state
        .apply_knowledge_rewrite("test", "rewrite.pre_route", config.clone())
        .unwrap();

    let device = Device::line("rewrite-session-layout", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::T),
            Instruction::Standard(StandardGate::TDG),
        ])
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 2)], 3).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let routing_target = RoutingTarget::from_physical(&physical).unwrap();
    let routed = route_with_layout_tracked_on_topology(
        &state.current,
        &routing_target,
        &layout,
        &sabre_config_for_mode(CompileMode::Enhanced, Some(11)),
    )
    .unwrap();
    assert_eq!(routed.routed().swap_count(), 0);
    assert!(routed.routed().changed(&state.current));
    let edits = routed.rewrite_edits(&state.current);
    let routed_circuit = routed.into_routed().into_circuit();
    let forced_full = KnowledgeRewriter::new(config.clone())
        .force_full_scan()
        .run(&routed_circuit)
        .unwrap();
    state
        .apply_transform_with_edits("test", "route", |_circuit, _analysis| {
            Ok((TransformOutcome::Changed(routed_circuit), edits))
        })
        .unwrap();
    state
        .apply_knowledge_rewrite("test", "rewrite.post_route", config)
        .unwrap();

    assert_eq!(state.current, forced_full.circuit);
    assert_eq!(state.rewrite_diagnostics.direct_reuses, 1);
    assert_eq!(state.rewrite_diagnostics.dirty_anchors, 0);
}

#[test]
fn rewrite_session_does_not_certify_a_round_limited_mutation() {
    let q0 = Qubit::new(0);
    let config = RewriteConfig::production().with_max_rounds(1);
    let mut state = workflow_state_without_target_basis();
    state.current.h(q0).unwrap();
    state.current.h(q0).unwrap();
    state.analysis = workflow_analysis(&state.current);

    assert!(
        state
            .apply_knowledge_rewrite("test", "rewrite.limited", config.clone())
            .unwrap()
    );
    assert!(
        state
            .rewrite_session
            .reusable_proof(state.circuit_revision, &config)
            .is_none()
    );
    assert!(
        !state
            .apply_knowledge_rewrite("test", "rewrite.retry", config.clone())
            .unwrap()
    );

    assert!(
        state
            .rewrite_session
            .reusable_proof(state.circuit_revision, &config)
            .is_some()
    );
    assert_eq!(state.rewrite_diagnostics.full_scan_fallbacks, 1);
}

fn run_edit_session_in_pool(threads: usize) -> Circuit {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap()
        .install(|| {
            let config = RewriteConfig::production()
                .with_enabled_kinds(vec![RuleKind::Cancel])
                .with_max_window_ops(16);
            let mut state = workflow_state_without_target_basis();
            state.current = large_flat_rewrite_circuit(false);
            state.analysis = workflow_analysis(&state.current);
            state
                .apply_knowledge_rewrite("test", "rewrite.proof", config.clone())
                .unwrap();
            let changed = large_flat_rewrite_circuit(true);
            state
                .apply_transform_with_edits("test", "sparse-change", |_circuit, _analysis| {
                    Ok((
                        TransformOutcome::Changed(changed),
                        RewriteEdits::linear(
                            4_200,
                            4_200,
                            vec![OperationReplacement {
                                old: 2_101..2_102,
                                new: 2_101..2_102,
                            }],
                        ),
                    ))
                })
                .unwrap();
            state
                .apply_knowledge_rewrite("test", "rewrite.incremental", config)
                .unwrap();
            state.current
        })
}

#[test]
fn rewrite_session_edit_reconciliation_is_thread_deterministic() {
    let single = run_edit_session_in_pool(1);
    let four = run_edit_session_in_pool(4);
    assert_eq!(single, four);
}

#[test]
fn target_translation_skips_circuit_already_in_explicit_basis() {
    let workflow = CompilerWorkflow::new(compile_config(CompileMode::Normal));
    let q0 = Qubit::new(0);

    for name in [
        "translate.target_basis",
        "translate.target_basis.after_one_qubit",
    ] {
        let mut state =
            workflow_state_with_target_basis(vec![Instruction::Standard(StandardGate::X)]);
        state.current.x(q0).unwrap();
        state.analysis = workflow_analysis(&state.current);
        let operations = state.current.operations().as_ptr();

        workflow
            .apply_target_translation_named(&mut state, name)
            .unwrap();

        assert_eq!(state.current.operations().as_ptr(), operations);
        assert!(!state.changed);
        let report = state.steps.last().unwrap();
        assert_eq!(report.name, name);
        assert!(report.skipped);
        assert!(!report.changed);
        assert_eq!(
            report.reason.as_deref(),
            Some("circuit already satisfies the explicit target basis")
        );
    }
}

#[test]
fn target_translation_still_runs_for_gate_outside_explicit_basis() {
    let workflow = CompilerWorkflow::new(compile_config(CompileMode::Normal));
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut state = workflow_state_with_target_basis(vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::CZ),
    ]);
    state.current = Circuit::new(2);
    state.current.cx(q0, q1).unwrap();
    state.analysis = workflow_analysis(&state.current);

    workflow.apply_target_translation(&mut state).unwrap();

    let report = state.steps.last().unwrap();
    assert_eq!(report.name, "translate.target_basis");
    assert!(!report.skipped);
    assert!(report.changed);
    assert_eq!(
        standard_ops(&state.current),
        vec![StandardGate::H, StandardGate::CZ, StandardGate::H]
    );
}

#[test]
fn target_translation_does_not_skip_explicit_gphase() {
    let workflow = CompilerWorkflow::new(compile_config(CompileMode::Normal));
    let mut state = workflow_state_with_target_basis(vec![
        Instruction::Standard(StandardGate::X),
        Instruction::Standard(StandardGate::GPhase),
    ]);
    state
        .current
        .append(
            Instruction::Standard(StandardGate::GPhase),
            Vec::<Qubit>::new(),
            [ParameterValue::Fixed(0.25)],
            None,
        )
        .unwrap();
    state.analysis = workflow_analysis(&state.current);

    workflow.apply_target_translation(&mut state).unwrap();

    let report = state.steps.last().unwrap();
    assert!(!report.skipped);
    assert!(report.changed);
    assert!(state.current.operations().is_empty());
    assert_eq!(state.current.global_phase(), Parameter::from(0.25));
}

#[test]
fn two_qubit_resynthesis_returns_the_recorded_changed_status() {
    let workflow = CompilerWorkflow::new(compile_config(CompileMode::Normal));
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    let mut stable_state = workflow_state_without_target_basis();
    stable_state.current = Circuit::new(2);
    stable_state.current.h(q0).unwrap();
    stable_state.analysis = workflow_analysis(&stable_state.current);

    let stable_changed = workflow
        .apply_two_qubit_resynthesis(
            &mut stable_state,
            "test",
            "resynthesis.stable",
            DeviceSynthesisPlacement::PreLayoutEnvelope,
        )
        .unwrap();

    assert!(!stable_changed);
    assert!(!stable_state.steps.last().unwrap().changed);

    let mut changed_state = workflow_state_with_target_basis(vec![
        Instruction::Standard(StandardGate::U),
        Instruction::Standard(StandardGate::CX),
    ]);
    changed_state.current = Circuit::new(2);
    changed_state.current.cx(q0, q1).unwrap();
    changed_state.current.cx(q0, q1).unwrap();
    changed_state.analysis = workflow_analysis(&changed_state.current);

    let resynthesis_changed = workflow
        .apply_two_qubit_resynthesis(
            &mut changed_state,
            "test",
            "resynthesis.changed",
            DeviceSynthesisPlacement::PreLayoutEnvelope,
        )
        .unwrap();

    assert!(resynthesis_changed);
    assert!(changed_state.steps.last().unwrap().changed);
    assert!(changed_state.current.operations().is_empty());
}

fn binding_case(bindings: &[(&'static str, f64)]) -> Option<HashMap<&'static str, f64>> {
    Some(bindings.iter().copied().collect())
}

fn assert_bindings_preserve_semantics(
    source: &Circuit,
    compiled: &Circuit,
    binding_cases: &[Option<HashMap<&'static str, f64>>],
) {
    for bindings in binding_cases {
        let bound_source = source.assign_parameters(bindings).unwrap();
        let bound_compiled = compiled.assign_parameters(bindings).unwrap();
        assert_compiled_circuit_equivalent(&bound_compiled, &bound_source);
    }
}

fn operation_parameter(circuit: &Circuit, param: &CircuitParam) -> Parameter {
    match param {
        CircuitParam::Fixed(value) => Parameter::from(*value),
        CircuitParam::Index(index) => circuit
            .parameters()
            .get_index(*index as usize)
            .cloned()
            .expect("parameter index should exist in rebuilt workflow circuit"),
    }
}

#[test]
fn normal_workflow_cancels_adjacent_self_inverse_gates() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();
    circuit.h(q0).unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);

    assert!(result.changed);
    assert_eq!(result.mode, CompileMode::Normal);
    assert!(result.circuit.operations().is_empty());
}

#[test]
fn normal_workflow_reports_no_change_for_stable_circuit() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);

    assert!(!result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::H]);
}

#[test]
fn logical_workflow_keeps_swap_when_no_layout_metadata_can_represent_it() {
    let mut circuit = Circuit::new(2);
    circuit.swap(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);

    assert!(standard_ops(&result.circuit).contains(&StandardGate::SWAP));
    let step = result.step("optimize.virtual_permutation").unwrap();
    assert!(step.skipped);
    assert_eq!(step.reason.as_deref(), Some("no target device configured"));
    assert!(result.device_metadata.is_none());
}

#[test]
fn normal_workflow_keeps_measurement_circuit_through_definition_stage() {
    let mut circuit = Circuit::new(2);
    circuit
        .measure_bits([Qubit::new(0), Qubit::new(1)])
        .unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);

    assert_eq!(result.circuit.operations().len(), 1);
    assert_eq!(
        result.circuit.classical_values(),
        circuit.classical_values()
    );
    let definitions = result.step("decompose.definitions").unwrap();
    assert!(definitions.skipped);
    assert!(!definitions.changed);
    assert_eq!(
        definitions.reason.as_deref(),
        Some("circuit contains no unexpanded gate definitions")
    );
}

#[test]
fn normal_workflow_reports_staged_order() {
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();

    let result = CompilerWorkflow::new(compile_config(CompileMode::Normal))
        .run(&circuit)
        .unwrap();

    assert_eq!(
        result
            .steps
            .iter()
            .map(|step| step.name)
            .collect::<Vec<_>>(),
        vec![
            "resolve.target",
            "validate.resources",
            "canonicalize.input",
            "decompose.definitions",
            "optimize.pre_decomposition",
            "decompose.unitary",
            "decompose.mc_gates",
            "canonicalize.after_decomposition",
            "optimize.commutative_cancellation",
            "optimize.virtual_permutation",
            "resynthesize.two_qubit_blocks",
            "optimize.one_qubit.post_decomposition",
            "optimize.post_decomposition",
            "optimize.commutative_cancellation.after_rewrite",
            "optimize.one_qubit.after_rewrite",
            "optimize.one_qubit_fixed_point",
            "decompose.routing_basis",
            "route.sabre",
            "translate.target_basis",
            "optimize.one_qubit.post_translation",
            "translate.target_basis.after_one_qubit",
            "canonicalize.output",
            "lower.device_instructions",
            "canonicalize.native_input",
            "optimize.native_fixed_point",
            "validate.device",
        ]
    );
    for name in [
        "decompose.definitions",
        "decompose.unitary",
        "decompose.mc_gates",
        "decompose.routing_basis",
        "optimize.virtual_permutation",
        "route.sabre",
        "translate.target_basis",
        "optimize.one_qubit.post_translation",
        "translate.target_basis.after_one_qubit",
        "lower.device_instructions",
        "canonicalize.native_input",
        "optimize.native_fixed_point",
        "validate.device",
    ] {
        assert!(result.step(name).unwrap().skipped, "{name}");
    }
    assert!(
        result
            .step("optimize.one_qubit_fixed_point")
            .unwrap()
            .reason
            .as_deref()
            .unwrap()
            .contains("max_rounds=2")
    );
}

#[test]
fn enhanced_sabre_uses_the_same_heuristic_with_larger_search_budgets() {
    let normal = sabre_config_for_mode(CompileMode::Normal, Some(7));
    let enhanced = sabre_config_for_mode(CompileMode::Enhanced, Some(7));

    assert_eq!(
        enhanced.heuristic.basic_weight,
        normal.heuristic.basic_weight
    );
    assert_eq!(
        enhanced.heuristic.decay_increment,
        normal.heuristic.decay_increment
    );
    assert_eq!(enhanced.heuristic.decay_reset, normal.heuristic.decay_reset);
    assert_eq!(
        enhanced.heuristic.attempt_limit,
        normal.heuristic.attempt_limit
    );
    assert_eq!(
        enhanced.heuristic.best_epsilon,
        normal.heuristic.best_epsilon
    );
    assert_eq!(
        enhanced.heuristic.lookahead_weights,
        normal.heuristic.lookahead_weights
    );
    assert!(enhanced.layout_trials > normal.layout_trials);
    assert!(enhanced.layout_assignment_budget > normal.layout_assignment_budget);
    assert!(enhanced.refinement_iterations > normal.refinement_iterations);
    assert!(enhanced.routing_trials > normal.routing_trials);
    assert_eq!(normal.routing_trials, 1);
    assert_eq!(enhanced.routing_trials, 2);
}

#[test]
fn enhanced_workflow_uses_richer_stage_sequence() {
    let mut circuit = Circuit::new(1);
    circuit.rz(Qubit::new(0), 0.25).unwrap();
    circuit.rz(Qubit::new(0), 0.5).unwrap();
    circuit.rz(Qubit::new(0), -0.75).unwrap();

    let normal = run_workflow(&circuit, CompileMode::Normal);
    let enhanced = run_workflow(&circuit, CompileMode::Enhanced);

    assert!(enhanced.changed);
    assert!(enhanced.steps.len() > normal.steps.len());
    assert_eq!(
        enhanced
            .steps
            .iter()
            .map(|step| step.name)
            .collect::<Vec<_>>(),
        vec![
            "resolve.target",
            "validate.resources",
            "canonicalize.input",
            "decompose.definitions",
            "optimize.pre_decomposition",
            "decompose.unitary",
            "decompose.mc_gates",
            "canonicalize.after_decomposition",
            "optimize.commutative_cancellation",
            "optimize.virtual_permutation",
            "resynthesize.two_qubit_blocks",
            "optimize.one_qubit.post_decomposition",
            "optimize.post_decomposition",
            "optimize.commutative_cancellation.after_rewrite",
            "optimize.one_qubit.after_rewrite",
            "optimize.one_qubit_fixed_point",
            "decompose.routing_basis",
            "route.sabre",
            "resynthesize.two_qubit_blocks.post_routing",
            "optimize.post_routing",
            "translate.target_basis",
            "optimize.target_cleanup",
            "optimize.one_qubit.post_translation",
            "translate.target_basis.after_one_qubit",
            "canonicalize.output",
            "lower.device_instructions",
            "canonicalize.native_input",
            "optimize.native_fixed_point",
            "validate.device",
        ]
    );
    for name in [
        "decompose.routing_basis",
        "optimize.virtual_permutation",
        "route.sabre",
        "resynthesize.two_qubit_blocks.post_routing",
        "optimize.post_routing",
        "translate.target_basis",
        "optimize.target_cleanup",
    ] {
        assert!(enhanced.step(name).unwrap().skipped, "{name}");
    }
    assert_eq!(
        enhanced
            .step("optimize.target_cleanup")
            .unwrap()
            .reason
            .as_deref(),
        Some("no explicit target basis configured")
    );
    assert!(
        enhanced
            .step("optimize.one_qubit_fixed_point")
            .unwrap()
            .reason
            .as_deref()
            .unwrap()
            .contains("max_rounds=4")
    );
    assert!(enhanced.circuit.operations().is_empty());
}

#[test]
fn enhanced_explicit_basis_runs_target_cleanup() {
    let mut circuit = Circuit::new(1);
    circuit.rz(Qubit::new(0), 0.25).unwrap();
    let target_gates = [StandardGate::RZ, StandardGate::X2P, StandardGate::CZ];
    let target_basis = target_gates
        .iter()
        .copied()
        .map(Instruction::Standard)
        .collect::<Vec<_>>();
    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::Basis(target_basis.clone()),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    let cleanup = result.step("optimize.target_cleanup").unwrap();
    assert!(!cleanup.skipped);
    assert!(
        standard_ops(&result.circuit)
            .iter()
            .all(|gate| target_gates.contains(gate))
    );
}

#[test]
fn workflow_expands_circuit_gate_definitions_before_optimization() {
    let q0 = Qubit::new(0);
    let mut definition = Circuit::new(1);
    definition.h(q0).unwrap();
    let gate = CircuitGate::new("H_DEF", FrozenCircuit::new(definition)).unwrap();

    let mut circuit = Circuit::new(1);
    circuit
        .circuit_gate(gate, vec![q0], Vec::<ParameterValue>::new())
        .unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);

    assert!(result.step_changed("decompose.definitions"));
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::H]);
    assert!(!contains_high_level_gate(&result.circuit));
}

#[test]
fn definition_expansion_invalidates_analysis_before_unitary_decomposition() {
    let q0 = Qubit::new(0);
    let matrix = array![
        [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
        [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
    ];
    let unitary = UnitaryGate::new("DEFINED_X", 1, 0)
        .with_matrix(matrix)
        .unwrap();
    let mut definition = Circuit::new(1);
    definition.unitary(unitary, vec![q0]).unwrap();
    let gate = CircuitGate::new("UNITARY_DEF", FrozenCircuit::new(definition)).unwrap();

    let mut circuit = Circuit::new(1);
    circuit
        .circuit_gate(gate, vec![q0], Vec::<ParameterValue>::new())
        .unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);

    assert!(result.step_changed("decompose.definitions"));
    let unitary_step = result.step("decompose.unitary").unwrap();
    assert!(!unitary_step.skipped);
    assert!(unitary_step.changed);
    assert!(!contains_high_level_gate(&result.circuit));
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn definition_expansion_invalidates_analysis_before_mc_decomposition() {
    let qubits = (0..4).map(Qubit::new).collect::<Vec<_>>();
    let mut definition = Circuit::new(4);
    definition
        .append(
            Instruction::McGate(Box::new(MCGate::new(3, StandardGate::X))),
            qubits.clone(),
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();
    let gate = CircuitGate::new("MCX_DEF", FrozenCircuit::new(definition)).unwrap();

    let mut circuit = Circuit::new(4);
    circuit
        .circuit_gate(gate, qubits, Vec::<ParameterValue>::new())
        .unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);

    assert!(result.step_changed("decompose.definitions"));
    let mc_step = result.step("decompose.mc_gates").unwrap();
    assert!(!mc_step.skipped);
    assert!(mc_step.changed);
    assert!(!contains_high_level_gate(&result.circuit));
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn workflow_synthesizes_matrix_backed_unitary_gates() {
    let q0 = Qubit::new(0);
    let matrix = array![
        [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
        [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
    ];
    let gate = UnitaryGate::new("X_MATRIX", 1, 0)
        .with_matrix(matrix)
        .unwrap();
    let mut circuit = Circuit::new(1);
    circuit.unitary(gate, vec![q0]).unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);

    assert!(result.step_changed("decompose.unitary"));
    assert!(!contains_high_level_gate(&result.circuit));
    assert!(!result.circuit.operations().is_empty());
}

#[test]
fn workflow_does_not_skip_unitary_validation_when_matrix_is_missing() {
    let mut circuit = Circuit::new(1);
    circuit
        .unitary(
            UnitaryGate::new("MISSING_MATRIX", 1, 0),
            vec![Qubit::new(0)],
        )
        .unwrap();

    let error = CompilerWorkflow::new(compile_config(CompileMode::Normal))
        .run(&circuit)
        .unwrap_err();

    assert!(error.to_string().contains("no matrix representation"));
}

#[test]
fn workflow_uses_cx_kak_for_cx_target() {
    let gate = UnitaryGate::new("SWAP_MATRIX", 2, 0)
        .with_matrix(StandardGate::SWAP.matrix(&[]).unwrap().into_owned())
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    let config = CompileConfig {
        target: CompileTarget::Basis(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ]),
        ..compile_config(CompileMode::Normal)
    };

    let result = CompilerWorkflow::new(config).run(&circuit).unwrap();
    let ops = standard_ops(&result.circuit);

    assert!(result.step_changed("decompose.unitary"));
    assert_eq!(
        ops.iter().filter(|gate| **gate == StandardGate::CX).count(),
        3
    );
    assert!(!ops.contains(&StandardGate::CZ));
    assert!(!ops.contains(&StandardGate::RXX));
    assert!(!ops.contains(&StandardGate::RYY));
    assert!(!ops.contains(&StandardGate::RZZ));
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn workflow_uses_three_native_cx_for_device_targeted_swap_unitary() {
    let gate = UnitaryGate::new("DEVICE_CX_SWAP_MATRIX", 2, 0)
        .with_matrix(StandardGate::SWAP.matrix(&[]).unwrap().into_owned())
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    let device = Device::bidirectional_line("device-cx-kak", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: None,
            seed: Some(41),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();
    let ops = standard_ops(&result.circuit);

    assert_eq!(
        ops.iter().filter(|gate| **gate == StandardGate::CX).count(),
        3
    );
    assert!(
        ops.iter()
            .all(|gate| matches!(gate, StandardGate::U | StandardGate::CX))
    );
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    device.validate_circuit(&result.circuit).unwrap();
    assert!(result.step("select.sabre_pareto_beam").is_none());
}

#[test]
fn strict_device_exposes_exact_device_planning_target() {
    for mode in [CompileMode::Normal, CompileMode::Enhanced] {
        let workflow = CompilerWorkflow::new(CompileConfig {
            mode,
            target: CompileTarget::Device(DeviceCompileTarget {
                device: Device::line("borrowed-routing-device", 2).unwrap(),
                initial_layout: None,
                seed: Some(46),
            }),
            resource_policy: ResourcePolicy::default(),
        });
        assert!(workflow.strict_device_target().is_some());
    }
}

#[test]
fn topology_basis_selects_topology_only_without_mutating_device_capabilities() {
    let offline = PhysicalQubit::new(1);
    let device = Device::line("owned-routing-device", 2)
        .unwrap()
        .with_invalid_qubits(HashSet::from([offline]))
        .unwrap();
    let workflow = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::TopologyBasis {
            device_target: DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: Some(47),
            },
            basis: vec![
                Instruction::Standard(StandardGate::H),
                Instruction::Standard(StandardGate::CZ),
            ],
        },
        resource_policy: ResourcePolicy::default(),
    });
    let target = workflow.routing_device_target().unwrap();

    assert!(workflow.strict_device_target().is_none());
    assert_eq!(target.device.qubits().count(), 2);
    assert_eq!(
        target.device.invalid_qubits().collect::<Vec<_>>(),
        vec![offline]
    );
    assert_eq!(target.device.topology().undirected_edges().count(), 1);
    assert!(target.device.native_gates().is_empty());
}

#[test]
fn topology_basis_routes_on_device_but_uses_explicit_output_basis() {
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let device = Device::bidirectional_line("topology-basis", 2)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap();
    let basis = vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::CZ),
    ];

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::TopologyBasis {
            device_target: DeviceCompileTarget {
                device: device.clone(),
                initial_layout: None,
                seed: Some(47),
            },
            basis,
        },
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::H, StandardGate::CZ, StandardGate::H]
    );
    assert!(result.device_metadata.is_some());
    assert!(result.step("lower.device_instructions").unwrap().skipped);
    assert!(result.step("canonicalize.native_input").unwrap().skipped);
    assert!(result.step("optimize.native_fixed_point").unwrap().skipped);
    assert!(result.step("validate.device").unwrap().skipped);
    assert!(device.validate_circuit(&result.circuit).is_err());
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn workflow_uses_three_native_cz_for_device_targeted_swap_unitary() {
    let gate = UnitaryGate::new("DEVICE_CZ_SWAP_MATRIX", 2, 0)
        .with_matrix(StandardGate::SWAP.matrix(&[]).unwrap().into_owned())
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    let device = Device::bidirectional_line("device-cz-kak", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CZ),
        ])
        .unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: None,
            seed: Some(43),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();
    let ops = standard_ops(&result.circuit);

    assert_eq!(
        ops.iter().filter(|gate| **gate == StandardGate::CZ).count(),
        3
    );
    assert!(
        ops.iter()
            .all(|gate| matches!(gate, StandardGate::U | StandardGate::CZ))
    );
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn workflow_lowers_to_exact_pair_and_runs_native_fixed_point_in_both_modes() {
    // Pre-layout prefers the broad CX family when CZ is only native on one edge
    // (see device_synthesis pre_layout coverage test). Force the routed pair onto
    // that CZ-only edge. Both modes must emit its exact native basis and run
    // the shared native fixed-point closure, even when earlier device lowering
    // has already produced a stable circuit.
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let mut device = Device::bidirectional_line("heterogeneous-exact-pair", 4)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    for (control, target) in [(p1, p2), (p2, p1)] {
        device
            .add_edge_properties(
                control,
                target,
                EdgeProp::new()
                    .with_native_instruction(InstructionProp::new(
                        Instruction::Standard(StandardGate::CZ),
                        0.005,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }
    let gate = UnitaryGate::new("LOCAL_CZ_SWAP_MATRIX", 2, 0)
        .with_matrix(StandardGate::SWAP.matrix(&[]).unwrap().into_owned())
        .unwrap();
    // The four-qubit device gives CX wider physical-pair coverage than the
    // single CZ-only edge; logical qubit indices do not affect that envelope.
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    let initial_layout = Layout::from_pairs(&[(0, 1), (1, 2)], 4).unwrap();
    let compile_for_mode = |mode| {
        CompilerWorkflow::new(CompileConfig {
            mode,
            target: CompileTarget::Device(DeviceCompileTarget {
                device: device.clone(),
                initial_layout: Some(initial_layout.clone()),
                seed: Some(47),
            }),
            resource_policy: ResourcePolicy::default(),
        })
        .run(&circuit)
        .unwrap()
    };

    let normal = compile_for_mode(CompileMode::Normal);
    let enhanced = compile_for_mode(CompileMode::Enhanced);
    let enhanced_ops = standard_ops(&enhanced.circuit);
    let normal_ops = standard_ops(&normal.circuit);

    assert_eq!(
        enhanced_ops
            .iter()
            .filter(|gate| **gate == StandardGate::CZ)
            .count(),
        3
    );
    assert!(
        enhanced_ops
            .iter()
            .all(|gate| matches!(gate, StandardGate::U | StandardGate::CZ))
    );
    assert_eq!(
        normal_ops
            .iter()
            .filter(|gate| **gate == StandardGate::CZ)
            .count(),
        3
    );
    assert!(
        normal_ops
            .iter()
            .all(|gate| matches!(gate, StandardGate::U | StandardGate::CZ))
    );
    let post_routing = "resynthesize.two_qubit_blocks.post_routing";
    assert!(normal.step(post_routing).is_none());
    let resynthesis = enhanced.step(post_routing).unwrap();
    assert!(!resynthesis.skipped, "{resynthesis:?}");
    // A resynthesis candidate need not improve on an earlier decomposition.
    // Floating-point tie-breaking can affect acceptance on different platforms;
    // the contract is that enhanced mode runs this stage and the final circuit
    // uses the exact physical basis, not that this particular stage must change it.
    // `changed` can reflect only canonicalization of a tiny global-phase
    // residual, which varies across architectures. Require execution and the
    // configured quality policy; correctness is checked on the final circuits.
    for result in [&normal, &enhanced] {
        let native = result.step("optimize.native_fixed_point").unwrap();
        assert!(!native.skipped, "{native:?}");
        let reason = native.reason.as_deref().unwrap();
        assert!(
            reason.contains("quality_policy=balanced_depth"),
            "{native:?}"
        );
        assert!(reason.contains("quality_rejections="), "{native:?}");
    }
    let mut physical_expected = Circuit::new(4);
    physical_expected
        .swap(Qubit::new(1), Qubit::new(2))
        .unwrap();
    assert_compiled_circuit_equivalent(&enhanced.circuit, &physical_expected);
    assert_compiled_circuit_equivalent(&normal.circuit, &physical_expected);
    device.validate_circuit(&enhanced.circuit).unwrap();
    device.validate_circuit(&normal.circuit).unwrap();
}

#[test]
fn workflow_runs_two_qubit_resynthesis_after_decomposition() {
    let gate = UnitaryGate::new("SWAP_MATRIX", 2, 0)
        .with_matrix(StandardGate::SWAP.matrix(&[]).unwrap().into_owned())
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate.clone(), vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    let config = CompileConfig {
        target: CompileTarget::Basis(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ]),
        ..compile_config(CompileMode::Normal)
    };

    let result = CompilerWorkflow::new(config).run(&circuit).unwrap();
    let ops = standard_ops(&result.circuit);

    assert!(result.step_changed("resynthesize.two_qubit_blocks"));
    assert!(
        ops.iter().filter(|gate| **gate == StandardGate::CX).count() < 6,
        "resynthesis should reduce the six-CX decomposition of two SWAPs"
    );
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn workflow_uses_cz_kak_for_cz_target() {
    let gate = UnitaryGate::new("SWAP_MATRIX", 2, 0)
        .with_matrix(StandardGate::SWAP.matrix(&[]).unwrap().into_owned())
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    let config = CompileConfig {
        target: CompileTarget::Basis(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CZ),
        ]),
        ..compile_config(CompileMode::Normal)
    };

    let result = CompilerWorkflow::new(config).run(&circuit).unwrap();
    let ops = standard_ops(&result.circuit);

    assert!(result.step_changed("decompose.unitary"));
    assert_eq!(
        ops.iter().filter(|gate| **gate == StandardGate::CZ).count(),
        3
    );
    assert!(!ops.contains(&StandardGate::CX));
    assert!(!ops.contains(&StandardGate::RXX));
    assert!(!ops.contains(&StandardGate::RYY));
    assert!(!ops.contains(&StandardGate::RZZ));
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn workflow_uses_cy_kak_for_cy_target() {
    let gate = UnitaryGate::new("SWAP_MATRIX", 2, 0)
        .with_matrix(StandardGate::SWAP.matrix(&[]).unwrap().into_owned())
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    let config = CompileConfig {
        target: CompileTarget::Basis(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CY),
        ]),
        ..compile_config(CompileMode::Normal)
    };

    let result = CompilerWorkflow::new(config).run(&circuit).unwrap();
    let ops = standard_ops(&result.circuit);

    assert!(result.step_changed("decompose.unitary"));
    assert_eq!(
        ops.iter().filter(|gate| **gate == StandardGate::CY).count(),
        3
    );
    assert!(!ops.contains(&StandardGate::CX));
    assert!(!ops.contains(&StandardGate::CZ));
    assert!(!ops.contains(&StandardGate::RXX));
    assert!(!ops.contains(&StandardGate::RYY));
    assert!(!ops.contains(&StandardGate::RZZ));
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn workflow_uses_rzz_kak_for_rzz_only_entangler_target() {
    let gate = UnitaryGate::new("SWAP_MATRIX", 2, 0)
        .with_matrix(StandardGate::SWAP.matrix(&[]).unwrap().into_owned())
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    let config = CompileConfig {
        target: CompileTarget::Basis(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::RX),
            Instruction::Standard(StandardGate::RZZ),
        ]),
        ..compile_config(CompileMode::Normal)
    };

    let result = CompilerWorkflow::new(config).run(&circuit).unwrap();
    let ops = standard_ops(&result.circuit);

    assert!(result.step_changed("decompose.unitary"));
    assert_eq!(
        ops.iter()
            .filter(|gate| **gate == StandardGate::RZZ)
            .count(),
        3
    );
    assert!(!ops.contains(&StandardGate::CX));
    assert!(!ops.contains(&StandardGate::CY));
    assert!(!ops.contains(&StandardGate::CZ));
    assert!(!ops.contains(&StandardGate::RXX));
    assert!(!ops.contains(&StandardGate::RYY));
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn workflow_keeps_pauli_kak_for_full_ising_target() {
    let gate = UnitaryGate::new("SWAP_MATRIX", 2, 0)
        .with_matrix(StandardGate::SWAP.matrix(&[]).unwrap().into_owned())
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    let config = CompileConfig {
        target: CompileTarget::Basis(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::RXX),
            Instruction::Standard(StandardGate::RYY),
            Instruction::Standard(StandardGate::RZZ),
        ]),
        ..compile_config(CompileMode::Normal)
    };

    let result = CompilerWorkflow::new(config).run(&circuit).unwrap();
    let ops = standard_ops(&result.circuit);

    assert!(result.step_changed("decompose.unitary"));
    assert!(!ops.contains(&StandardGate::CX));
    assert!(!ops.contains(&StandardGate::CY));
    assert!(!ops.contains(&StandardGate::CZ));
    assert!(ops.contains(&StandardGate::RXX));
    assert!(ops.contains(&StandardGate::RYY));
    assert!(ops.contains(&StandardGate::RZZ));
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn workflow_keeps_pauli_kak_for_partial_ising_target() {
    let rxx = StandardGate::RXX.matrix(&[0.7]).unwrap().into_owned();
    let ryy = StandardGate::RYY.matrix(&[-0.4]).unwrap().into_owned();
    let matrix = rxx.dot(&ryy);
    let gate = UnitaryGate::new("PARTIAL_ISING_MATRIX", 2, 0)
        .with_matrix(matrix)
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    let config = CompileConfig {
        target: CompileTarget::Basis(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::RX),
            Instruction::Standard(StandardGate::RXX),
            Instruction::Standard(StandardGate::RZZ),
        ]),
        ..compile_config(CompileMode::Normal)
    };

    let result = CompilerWorkflow::new(config).run(&circuit).unwrap();
    let ops = standard_ops(&result.circuit);

    assert!(result.step_changed("decompose.unitary"));
    assert!(ops.contains(&StandardGate::RZZ));
    assert!(!ops.contains(&StandardGate::RXX));
    assert!(!ops.contains(&StandardGate::RYY));
    assert!(!ops.contains(&StandardGate::CX));
    assert!(!ops.contains(&StandardGate::CY));
    assert!(!ops.contains(&StandardGate::CZ));
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn workflow_keeps_pauli_kak_without_target() {
    let gate = UnitaryGate::new("SWAP_MATRIX", 2, 0)
        .with_matrix(StandardGate::SWAP.matrix(&[]).unwrap().into_owned())
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);
    let ops = standard_ops(&result.circuit);

    assert!(result.step_changed("decompose.unitary"));
    assert!(!ops.contains(&StandardGate::CX));
    assert!(!ops.contains(&StandardGate::CY));
    assert!(!ops.contains(&StandardGate::CZ));
    assert!(
        ops.contains(&StandardGate::RXX)
            || ops.contains(&StandardGate::RYY)
            || ops.contains(&StandardGate::RZZ)
    );
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn workflow_decomposes_multi_controlled_gates() {
    let qubits = (0..4).map(Qubit::new).collect::<Vec<_>>();
    let mut circuit = Circuit::new(4);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(3, StandardGate::X))),
            qubits.clone(),
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);

    assert!(result.step_changed("decompose.mc_gates"));
    assert!(!contains_high_level_gate(&result.circuit));
}

#[test]
fn workflow_reports_rewrite_change_for_symbolic_merge() {
    let q0 = Qubit::new(0);
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(1);
    circuit.rz(q0, theta.clone()).unwrap();
    circuit.rz(q0, 0.5).unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);

    assert!(
        result.step_changed("optimize.pre_decomposition")
            || result.step_changed("optimize.post_decomposition")
    );
    assert_eq!(result.circuit.operations().len(), 1);
    let merged = operation_parameter(&result.circuit, &result.circuit.operations()[0].params[0]);
    assert!(merged.provably_equal(&(theta.clone() + Parameter::from(0.5)), 1e-12));
    assert_bindings_preserve_semantics(
        &circuit,
        &result.circuit,
        &[
            binding_case(&[("theta", 0.0)]),
            binding_case(&[("theta", 0.25)]),
            binding_case(&[("theta", -PI / 4.0)]),
        ],
    );
}

#[test]
fn workflow_decomposes_parameterized_mc_gate() {
    let qubits = (0..3).map(Qubit::new).collect::<Vec<_>>();
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::RZ))),
            qubits,
            vec![ParameterValue::Param(theta.clone())],
            None,
        )
        .unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);

    assert!(result.step_changed("decompose.mc_gates"));
    assert!(!contains_high_level_gate(&result.circuit));
    assert!(result.circuit.uses_symbol("theta"));
    assert_bindings_preserve_semantics(
        &circuit,
        &result.circuit,
        &[
            binding_case(&[("theta", 0.0)]),
            binding_case(&[("theta", 0.31)]),
            binding_case(&[("theta", PI / 3.0)]),
        ],
    );
}

#[test]
fn workflow_routes_parameterized_circuit_when_device_present() {
    let q0 = Qubit::new(0);
    let q2 = Qubit::new(2);
    let theta = Parameter::symbol("theta");
    let phi = Parameter::symbol("phi");
    let mut circuit = Circuit::new(3);
    circuit.rx(q0, theta.clone()).unwrap();
    circuit.rz(q2, phi.clone()).unwrap();
    circuit.cx(q0, q2).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: Device::line("workflow-param-line", 3)
                .unwrap()
                .with_native_gates(vec![
                    Instruction::Standard(StandardGate::H),
                    Instruction::Standard(StandardGate::RX),
                    Instruction::Standard(StandardGate::RZ),
                    Instruction::Standard(StandardGate::CX),
                ])
                .unwrap(),
            initial_layout: None,
            seed: Some(11),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    assert!(result.step_changed("route.sabre"));
    assert!(result.circuit.uses_symbol("theta"));
    assert!(result.circuit.uses_symbol("phi"));
    assert!(result.circuit.operations().iter().any(|operation| {
        matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::RX)
        ) && matches!(operation.params.as_slice(), [CircuitParam::Index(_)])
    }));
    assert!(result.circuit.operations().iter().any(|operation| {
        matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::RZ)
        ) && matches!(operation.params.as_slice(), [CircuitParam::Index(_)])
    }));
}

#[test]
fn workflow_routes_from_supplied_initial_layout() {
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();
    let layout = Layout::from_pairs(&[(0, 2)], 3).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: Device::line("layout-line", 3)
                .unwrap()
                .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
                .unwrap(),
            initial_layout: Some(layout),
            seed: Some(17),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    let route = result
        .steps
        .iter()
        .find(|step| step.name == "route.sabre")
        .unwrap();
    assert!(!route.skipped);
    assert!(route.changed);
    assert!(
        route
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("supplied initial layout"))
    );
    assert_eq!(
        result.circuit.operations()[0].qubits.as_slice(),
        &[Qubit::new(2)]
    );
}

#[test]
fn supplied_initial_layout_stays_input_oriented_while_final_layout_composes_output_permutation() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.swap(q0, q1).unwrap();
    circuit.x(q0).unwrap();
    let supplied = Layout::from_pairs(&[(0, 2), (1, 0)], 3).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: Device::line("permuted-layout-line", 3)
                .unwrap()
                .with_native_gates(vec![
                    Instruction::Standard(StandardGate::H),
                    Instruction::Standard(StandardGate::X),
                    Instruction::Standard(StandardGate::CX),
                ])
                .unwrap(),
            initial_layout: Some(supplied),
            seed: Some(19),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    assert!(result.step_changed("optimize.virtual_permutation"));
    let metadata = result.device_metadata.unwrap();
    assert_eq!(
        metadata.initial_layout.get_physical(LogicalQubit::new(0)),
        Some(PhysicalQubit::new(2))
    );
    assert_eq!(
        metadata.initial_layout.get_physical(LogicalQubit::new(1)),
        Some(PhysicalQubit::new(0))
    );
    assert_eq!(
        metadata
            .virtual_permutation
            .rewritten_output(LogicalQubit::new(0)),
        Some(LogicalQubit::new(1))
    );
    assert_eq!(
        metadata.final_layout.get_physical(LogicalQubit::new(0)),
        Some(PhysicalQubit::new(0))
    );
    assert_eq!(
        metadata.final_layout.get_physical(LogicalQubit::new(1)),
        Some(PhysicalQubit::new(2))
    );
}

#[test]
fn automatic_layout_with_virtual_permutation_remains_device_valid() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q1).unwrap();
    circuit.swap(q0, q2).unwrap();
    let device = Device::line("automatic-permutation-line", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: None,
            seed: Some(23),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    assert!(result.step_changed("optimize.virtual_permutation"));
    device.validate_circuit(&result.circuit).unwrap();
    let metadata = result.device_metadata.unwrap();
    assert_eq!(
        metadata
            .virtual_permutation
            .rewritten_output(LogicalQubit::new(0)),
        Some(LogicalQubit::new(2))
    );
    assert_eq!(
        metadata
            .virtual_permutation
            .rewritten_output(LogicalQubit::new(2)),
        Some(LogicalQubit::new(0))
    );
}

#[test]
fn virtual_permutation_precedes_two_qubit_resynthesis() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.swap(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    let device = Device::bidirectional_line("permutation-before-resynthesis", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: None,
            seed: Some(29),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    let virtual_index = result
        .steps
        .iter()
        .position(|step| step.name == "optimize.virtual_permutation")
        .unwrap();
    let resynthesis_index = result
        .steps
        .iter()
        .position(|step| step.name == "resynthesize.two_qubit_blocks")
        .unwrap();
    assert!(virtual_index < resynthesis_index);
    assert!(result.step_changed("optimize.virtual_permutation"));
    assert_eq!(
        result
            .circuit
            .operations()
            .iter()
            .filter(|operation| operation.qubits.len() == 2)
            .count(),
        1
    );
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn device_workflow_legalizes_reverse_cx_to_directed_native_gates() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();

    let device = Device::line_from_qubits(
        "reverse-cx",
        vec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
    )
    .unwrap()
    .with_native_gates(vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::CX),
    ])
    .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: Some(layout),
            seed: Some(7),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn device_workflow_legalizes_swap_through_reverse_cx_templates() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit
        .append(
            Instruction::Standard(StandardGate::SWAP),
            [q0, q1],
            [],
            Some("preserve-native-realization"),
        )
        .unwrap();

    let device = Device::line_from_qubits(
        "reverse-swap",
        vec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
    )
    .unwrap()
    .with_native_gates(vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::CX),
    ])
    .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: Some(layout),
            seed: Some(11),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn device_workflow_selects_lowest_cost_native_swap_realization() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit
        .append(
            Instruction::Standard(StandardGate::SWAP),
            [q0, q1],
            [],
            Some("preserve-native-realization"),
        )
        .unwrap();

    let device = Device::line_from_qubits(
        "reverse-swap-cz",
        vec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
    )
    .unwrap()
    .with_native_gates(vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::CX),
        Instruction::Standard(StandardGate::CZ),
    ])
    .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: Some(layout),
            seed: Some(13),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    device.validate_circuit(&result.circuit).unwrap();
    let gates = standard_ops(&result.circuit);
    assert_eq!(
        gates
            .iter()
            .filter(|gate| matches!(gate, StandardGate::CX | StandardGate::CZ))
            .count(),
        3
    );
    assert_eq!(gates.len(), 5);
}

#[test]
fn device_workflow_reports_reversed_native_gate_as_changed() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cz(q0, q1).unwrap();

    let device = Device::line_from_qubits(
        "reverse-cz",
        vec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
    )
    .unwrap()
    .with_native_gates(vec![Instruction::Standard(StandardGate::CZ)])
    .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: Some(layout),
            seed: Some(17),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    device.validate_circuit(&result.circuit).unwrap();
    assert!(result.step_changed("lower.device_instructions"));
    assert_eq!(result.circuit.operations()[0].qubits.as_slice(), &[q1, q0]);
}

#[test]
fn device_workflow_legalizes_global_cx_against_local_cz_override() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let mut device = Device::line("local-cz", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    device
        .add_edge_properties(
            p0,
            p1,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CZ),
                    0.01,
                ))
                .unwrap(),
        )
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: Some(layout),
            seed: Some(19),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    device.validate_circuit(&result.circuit).unwrap();
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::H, StandardGate::CZ, StandardGate::H]
    );
}

#[test]
fn device_workflow_uses_local_native_capabilities_without_global_defaults() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let mut device = Device::line("local-only", 2).unwrap();
    for qubit in [p0, p1] {
        device
            .add_qubit_properties(
                qubit,
                crate::device::QubitProp::new(0.01)
                    .with_native_instruction(InstructionProp::new(
                        Instruction::Standard(StandardGate::H),
                        0.01,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }
    device
        .add_edge_properties(
            p0,
            p1,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.01,
                ))
                .unwrap(),
        )
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: Some(layout),
            seed: Some(23),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    assert!(!result.step_changed("lower.device_instructions"));
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn device_workflow_rejects_unroutable_native_interaction_during_sabre_preflight() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    let device = Device::line_from_qubits(
        "no-reverse-cx-plan",
        vec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
    )
    .unwrap()
    .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
    .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();

    let err = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device,
            initial_layout: Some(layout),
            seed: Some(29),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap_err();

    assert!(matches!(
        err,
        CompilerError::SabreRoutingFailed(SabreRoutingFailure::UnreachablePairPlacement {
            logical,
            physical,
        }) if logical == [Qubit::new(0).into(), Qubit::new(1).into()]
            && physical == [PhysicalQubit::new(0), PhysicalQubit::new(1)]
    ));
}

#[test]
fn device_workflow_legalizes_control_flow_bodies_and_returns_layout_metadata() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    let measured = circuit.measure(q0).unwrap();
    circuit
        .if_(measured.expr().to_bool().unwrap(), |body| {
            body.cx(q0, q1)?;
            Ok(())
        })
        .unwrap();
    let device = Device::line_from_qubits(
        "controlled-reverse-cx",
        vec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
    )
    .unwrap()
    .with_native_gates(vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::CX),
    ])
    .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: Some(layout.clone()),
            seed: Some(31),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    device.validate_circuit(&result.circuit).unwrap();
    assert!(result.step_changed("lower.device_instructions"));
    let metadata = result.device_metadata.expect("device compilation metadata");
    assert_eq!(metadata.initial_layout, layout);
    assert_eq!(metadata.final_layout, layout);
}

#[test]
fn workflow_target_translation_keeps_parameterized_semantics() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.crz(q0, q1, theta.clone()).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Basis(vec![
            Instruction::Standard(StandardGate::RZ),
            Instruction::Standard(StandardGate::X2P),
            Instruction::Standard(StandardGate::X2M),
            Instruction::Standard(StandardGate::Y2P),
            Instruction::Standard(StandardGate::Y2M),
            Instruction::Standard(StandardGate::CZ),
            Instruction::Standard(StandardGate::GPhase),
        ]),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    assert!(result.step_changed("translate.target_basis"));
    assert!(result.circuit.uses_symbol("theta"));
    assert!(standard_ops(&result.circuit).iter().all(|gate| matches!(
        gate,
        StandardGate::RZ
            | StandardGate::X2P
            | StandardGate::X2M
            | StandardGate::Y2P
            | StandardGate::Y2M
            | StandardGate::CZ
            | StandardGate::GPhase
    )));
    assert_bindings_preserve_semantics(
        &circuit,
        &result.circuit,
        &[
            binding_case(&[("theta", 0.0)]),
            binding_case(&[("theta", 0.21)]),
            binding_case(&[("theta", -PI / 5.0)]),
        ],
    );
}

#[test]
fn target_basis_translation_runs_after_definition_decomposition() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut definition = Circuit::new(2);
    definition.cx(q0, q1).unwrap();
    let gate = CircuitGate::new("CX_DEF", FrozenCircuit::new(definition)).unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .circuit_gate(gate, vec![q0, q1], Vec::<ParameterValue>::new())
        .unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Basis(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CZ),
        ]),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    assert!(result.step_changed("decompose.definitions"));
    assert!(result.step_changed("translate.target_basis"));
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::H, StandardGate::CZ, StandardGate::H]
    );
}

#[test]
fn explicit_target_basis_runs_lowering() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Basis(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CZ),
        ]),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    assert!(result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::H, StandardGate::CZ, StandardGate::H]
    );
    assert_eq!(result.circuit.operations()[0].qubits.as_slice(), &[q1]);
    assert_eq!(result.circuit.operations()[1].qubits.as_slice(), &[q0, q1]);
    assert_eq!(result.circuit.operations()[2].qubits.as_slice(), &[q1]);
    assert!(
        result
            .steps
            .iter()
            .any(|step| step.name == "lower.device_instructions" && step.skipped)
    );
    assert!(
        result
            .steps
            .iter()
            .any(|step| step.name == "validate.device" && step.skipped)
    );
}

#[test]
fn target_basis_failure_is_reported() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();

    let err = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Basis(vec![Instruction::Standard(StandardGate::CZ)]),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap_err();

    assert!(matches!(err, CompilerError::InvalidInput(_)));
}

#[test]
fn mc_gate_target_basis_is_rejected_by_workflow_contract() {
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let err = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Basis(vec![Instruction::McGate(Box::new(MCGate::new(
            1,
            StandardGate::X,
        )))]),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap_err();

    assert!(matches!(err, CompilerError::InvalidInput(_)));
}

#[test]
fn device_native_gates_are_legalized_against_ordered_capabilities() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    let device = two_qubit_device(vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::CZ),
    ]);

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: None,
            seed: None,
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    assert!(result.step_changed("lower.device_instructions"));
    device.validate_circuit(&result.circuit).unwrap();
    assert!(standard_ops(&result.circuit).contains(&StandardGate::CZ));
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::CX));
}

#[test]
fn device_workflow_routes_circuit_before_native_legalization() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q1, q2).unwrap();
    circuit.cx(q0, q2).unwrap();
    let device = Device::line("test-device", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: None,
            seed: Some(7),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    assert!(result.step_changed("route.sabre"));
    assert!(result.step_changed("lower.device_instructions"));
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::SWAP));
    device.validate_circuit(&result.circuit).unwrap();
    assert!(
        result
            .steps
            .iter()
            .find(|step| step.name == "route.sabre")
            .is_some_and(|step| !step.skipped)
    );
}

#[test]
fn routed_swaps_are_lowered_to_device_native_basis() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q1, q2).unwrap();
    circuit.cx(q0, q2).unwrap();
    let device = Device::line("test-device", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CZ),
        ])
        .unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device,
            initial_layout: None,
            seed: Some(7),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    assert!(result.step_changed("route.sabre"));
    assert!(result.step_changed("lower.device_instructions"));
    assert!(
        standard_ops(&result.circuit)
            .iter()
            .all(|gate| matches!(gate, StandardGate::H | StandardGate::CZ))
    );
}

#[test]
fn routed_swaps_are_lowered_to_qcis_native_subset() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q1, q2).unwrap();
    circuit.cx(q0, q2).unwrap();
    let device = Device::line("test-device", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::RZ),
            Instruction::Standard(StandardGate::X2P),
            Instruction::Standard(StandardGate::CZ),
        ])
        .unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device,
            initial_layout: None,
            seed: Some(7),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    let routing_basis_step = result
        .steps
        .iter()
        .find(|step| step.name == "decompose.routing_basis")
        .expect("workflow should report routing-basis decomposition");
    assert!(routing_basis_step.skipped);
    assert!(!routing_basis_step.changed);
    assert_eq!(
        routing_basis_step.reason.as_deref(),
        Some("circuit contains no gate-like operation over more than two qubits")
    );
    assert!(result.step_changed("route.sabre"));
    assert!(result.step_changed("lower.device_instructions"));
    assert!(standard_ops(&result.circuit).iter().all(|gate| matches!(
        gate,
        StandardGate::RZ | StandardGate::X2P | StandardGate::CZ
    )));
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::SWAP));
}

#[test]
fn enhanced_device_workflow_runs_post_routing_cleanup() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q1, q2).unwrap();
    circuit.cx(q0, q2).unwrap();
    let device = Device::line("test-device", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::Device(DeviceCompileTarget {
            device,
            initial_layout: None,
            seed: Some(7),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    let post_routing = result
        .steps
        .iter()
        .find(|step| step.name == "optimize.post_routing")
        .unwrap();
    assert!(!post_routing.skipped);
    let route_index = result
        .steps
        .iter()
        .position(|step| step.name == "route.sabre")
        .unwrap();
    assert_eq!(
        result.steps[route_index..]
            .iter()
            .map(|step| step.name)
            .collect::<Vec<_>>(),
        vec![
            "route.sabre",
            "resynthesize.two_qubit_blocks.post_routing",
            "optimize.post_routing",
            "translate.target_basis",
            "optimize.target_cleanup",
            "optimize.one_qubit.post_translation",
            "translate.target_basis.after_one_qubit",
            "canonicalize.output",
            "lower.device_instructions",
            "canonicalize.native_input",
            "optimize.native_fixed_point",
            "validate.device",
            "select.sabre_pareto_beam",
        ]
    );
    let selection = result.step("select.sabre_pareto_beam").unwrap();
    assert!(!selection.skipped);
    assert!(
        selection
            .reason
            .as_deref()
            .unwrap()
            .contains("contract=exact_scope_pareto_with_2q_gain")
    );
    let reason = selection.reason.as_deref().unwrap();
    assert!(reason.contains("tiers_evaluated=2"));
    assert!(reason.contains("native_2q_ops=") || reason.contains("candidate0_native_2q_ops="));
}

#[test]
fn optional_sabre_beam_discards_recoverable_candidate_errors_only() {
    assert!(pareto_exploration_error_is_recoverable(
        &CompilerError::InvalidInput("candidate-local input".to_string())
    ));
    assert!(pareto_exploration_error_is_recoverable(
        &CompilerError::TransformFailed {
            name: "candidate-transform",
            reason: "candidate-local failure".to_string(),
        }
    ));
    assert!(!pareto_exploration_error_is_recoverable(
        &CompilerError::InvariantViolation("internal contract".to_string())
    ));
}

#[test]
fn device_capacity_blocks_clean_ancilla_allocation_but_allows_no_aux_fallback() {
    let qubits = (0..4).map(Qubit::new).collect::<Vec<_>>();
    let mut circuit = Circuit::new(4);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(3, StandardGate::X))),
            qubits,
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();
    let device = Device::line("test-device", 4)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::S),
            Instruction::Standard(StandardGate::SDG),
            Instruction::Standard(StandardGate::T),
            Instruction::Standard(StandardGate::TDG),
            Instruction::Standard(StandardGate::X),
            Instruction::Standard(StandardGate::Z),
            Instruction::Standard(StandardGate::RX),
            Instruction::Standard(StandardGate::RY),
            Instruction::Standard(StandardGate::RZ),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device,
            initial_layout: None,
            seed: None,
        }),
        resource_policy: ResourcePolicy {
            max_pre_layout_clean_ancillas: 2,
            allow_dirty_borrowing: false,
        },
    })
    .run(&circuit)
    .unwrap();

    assert!(result.step_changed("decompose.mc_gates"));
    assert_eq!(result.circuit.qubits().len(), 4);
    assert!(!contains_high_level_gate(&result.circuit));
}

#[test]
fn device_capacity_rejects_source_circuit_that_is_too_wide() {
    let mut circuit = Circuit::new(3);
    circuit.h(Qubit::new(0)).unwrap();
    let device = Device::line("test-device", 2).unwrap();

    let err = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device,
            initial_layout: None,
            seed: None,
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap_err();

    assert!(matches!(
        err,
        CompilerError::InvalidInput(reason) if reason.contains("source circuit uses 3 logical qubits")
    ));
}

#[test]
fn device_capacity_rejects_too_wide_source_before_mc_decomposition() {
    let qubits = (0..3).map(Qubit::new).collect::<Vec<_>>();
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
            qubits,
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();
    let device = Device::line("test-device", 2).unwrap();

    let err = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device,
            initial_layout: None,
            seed: None,
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap_err();

    assert!(matches!(
        err,
        CompilerError::InvalidInput(reason) if reason.contains("source circuit uses 3 logical qubits")
    ));
}

#[test]
fn compile_matches_built_workflow() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.x(q1).unwrap();
    circuit.h(q0).unwrap();

    let direct = compile(&circuit, compile_config(CompileMode::Normal)).unwrap();
    let built = CompilerWorkflow::new(compile_config(CompileMode::Normal))
        .run(&circuit)
        .unwrap();

    assert_eq!(direct.changed, built.changed);
    assert_eq!(standard_ops(&direct.circuit), standard_ops(&built.circuit));
    assert_eq!(direct.steps, built.steps);
}

#[test]
fn workflow_config_can_build_enhanced_workflow() {
    let workflow = CompilerWorkflow::new(compile_config(CompileMode::Enhanced));

    assert_eq!(workflow.config().mode, CompileMode::Enhanced);
}

#[test]
fn target_cleanup_config_only_available_for_explicit_basis() {
    let workflow = CompilerWorkflow::new(compile_config(CompileMode::Normal));
    let target_basis = vec![
        Instruction::Standard(StandardGate::RZ),
        Instruction::Standard(StandardGate::X2P),
        Instruction::Standard(StandardGate::CZ),
    ];
    let state = workflow_state_with_target_basis(target_basis.clone());

    for phase in [
        RewritePhase::PreDecomposition,
        RewritePhase::PostDecomposition,
        RewritePhase::PostRouting,
    ] {
        let config = workflow.rewrite_config(phase).unwrap();
        assert!(config.target_instruction_basis().is_none());
    }

    let cleanup_config = workflow.target_cleanup_config(&state).unwrap();
    let cleanup_config = cleanup_config.unwrap();
    let cleanup_basis = cleanup_config.target_instruction_basis().unwrap();
    assert_eq!(cleanup_basis.len(), target_basis.len());
    assert!(matches!(
        cleanup_basis.as_slice(),
        [
            Instruction::Standard(StandardGate::RZ),
            Instruction::Standard(StandardGate::X2P),
            Instruction::Standard(StandardGate::CZ)
        ]
    ));

    assert!(
        workflow
            .target_cleanup_config(&workflow_state_without_target_basis())
            .unwrap()
            .is_none()
    );
}

#[test]
fn normal_workflow_fixpoint_closes_nested_cancellation_pairs() {
    // End-to-end pipeline check: three nesting levels on wire q1 (an outer
    // CZ pair blocked by a CX pair, itself blocked by an inner H pair),
    // every pair shielded from the windowed knowledge rewriter (>16 ops)
    // and one-qubit fusion (single gates on distinct wires) by commuting
    // gates on other wires. Global cancellation plus the fixpoint loop must
    // clear all three pairs; note that bounded two-qubit resynthesis also
    // rewrites nearby pair fragments into unitary gates in this pipeline.
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(70);
    circuit.cz(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.h(q1).unwrap();
    for index in 0..34u32 {
        circuit
            .rx(Qubit::new(2 + index), 0.01 * f64::from(index + 1))
            .unwrap();
    }
    circuit.h(q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    for index in 0..34u32 {
        circuit
            .ry(Qubit::new(36 + index), 0.02 * f64::from(index + 1))
            .unwrap();
    }
    circuit.cz(q0, q1).unwrap();

    let result = run_workflow(&circuit, CompileMode::Normal);

    assert!(result.changed);
    let gates = standard_ops(&result.circuit);
    assert!(!gates.contains(&StandardGate::CZ), "gates: {gates:?}");
    assert!(!gates.contains(&StandardGate::CX), "gates: {gates:?}");
    assert!(!gates.contains(&StandardGate::H), "gates: {gates:?}");
}

#[test]
fn fixpoint_loop_reruns_cancellation_after_stable_resynthesis() {
    // Drives close_one_qubit_resynthesis directly with a one-qubit-only
    // synthesis basis, so two-qubit resynthesis can never change the
    // circuit. Round 1 removes the H pair and exposes the CX pair; only a
    // second cancellation run removes it. A loop that exits whenever
    // resynthesis is stable would strand the CX pair.
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.h(q1).unwrap();
    circuit.h(q1).unwrap();
    circuit.cx(q0, q1).unwrap();

    let target_basis = vec![
        Instruction::Standard(StandardGate::RZ),
        Instruction::Standard(StandardGate::X2P),
    ];
    let mut state = workflow_state_with_target_basis(target_basis);
    state.analysis = workflow_analysis(&circuit);
    state.current = circuit;

    let workflow = CompilerWorkflow::new(compile_config(CompileMode::Normal));
    workflow.close_one_qubit_resynthesis(&mut state).unwrap();

    let gates = standard_ops(&state.current);
    assert!(gates.is_empty(), "gates: {gates:?}");
}

/// Asserts that every two-qubit operation in `circuit` acts on an adjacent
/// pair of the 0-1-2 line topology.
fn assert_two_qubit_gates_on_line(circuit: &Circuit) {
    for operation in circuit.operations() {
        if operation.qubits.len() != 2 {
            continue;
        }
        let a = operation.qubits[0].index();
        let b = operation.qubits[1].index();
        let (lo, hi) = (a.min(b), a.max(b));
        assert!(
            matches!((lo, hi), (0, 1) | (1, 2)),
            "two-qubit gate acts on non-adjacent pair ({a}, {b})"
        );
    }
}

#[test]
fn enhanced_device_target_keeps_routed_two_qubit_gates_on_topology() {
    // End-to-end smoke test: a non-adjacent CX forces routing to insert SWAPs,
    // and the full Enhanced pipeline must finish with every two-qubit gate on a
    // line-topology edge. The bridge-collapse regression itself is pinned
    // deterministically by `post_routing_cleanup_does_not_collapse_bridge`.
    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let device = Device::bidirectional_line("device-post-routing-line", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let initial_layout = Layout::from_pairs(&[(0, 0), (1, 1), (2, 2)], 3).unwrap();

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: Some(initial_layout),
            seed: Some(47),
        }),
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    // Strict device validation fails loudly if any two-qubit gate escaped the
    // line topology. (Matrix equivalence against the logical input is omitted:
    // the routed circuit lives on physical qubits whose final layout may be
    // permuted by the inserted SWAPs, so a direct matrix comparison is not the
    // invariant under test here.)
    device.validate_circuit(&result.circuit).unwrap();
    assert_two_qubit_gates_on_line(&result.circuit);
}

#[test]
fn enhanced_topology_basis_target_keeps_routed_two_qubit_gates_on_topology() {
    // End-to-end smoke test for the TopologyBasis path: routing a non-adjacent
    // CX and lowering to the H/CZ basis must keep every two-qubit gate on the
    // line topology, and the validate.topology backstop must actually run.
    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let device = Device::bidirectional_line("topology-basis-post-routing-line", 3)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap();
    let initial_layout = Layout::from_pairs(&[(0, 0), (1, 1), (2, 2)], 3).unwrap();
    let basis = vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::CZ),
    ];

    let result = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::TopologyBasis {
            device_target: DeviceCompileTarget {
                device: device.clone(),
                initial_layout: Some(initial_layout),
                seed: Some(47),
            },
            basis,
        },
        resource_policy: ResourcePolicy::default(),
    })
    .run(&circuit)
    .unwrap();

    // The workflow's validate.topology step already ran internally; assert the
    // invariant directly so a regression in that step still fails here.
    assert!(!result.step("validate.topology").unwrap().skipped);
    assert_two_qubit_gates_on_line(&result.circuit);
    assert!(
        standard_ops(&result.circuit)
            .iter()
            .all(|gate| matches!(gate, StandardGate::H | StandardGate::CZ))
    );
    assert!(result.step("select.sabre_pareto_beam").is_none());
}

#[test]
fn rewrite_connectivity_policy_follows_phase_and_routing() {
    // Logical target never routes.
    let logical = CompilerWorkflow::new(compile_config(CompileMode::Enhanced));
    assert!(
        !logical
            .rewrite_config(RewritePhase::PreDecomposition)
            .unwrap()
            .preserve_two_qubit_connectivity()
    );
    assert!(
        !logical
            .rewrite_config(RewritePhase::PostDecomposition)
            .unwrap()
            .preserve_two_qubit_connectivity()
    );
    assert!(
        logical
            .rewrite_config(RewritePhase::PostRouting)
            .unwrap()
            .preserve_two_qubit_connectivity()
    );
    assert!(
        !logical
            .rewrite_config(RewritePhase::TargetCleanup)
            .unwrap()
            .preserve_two_qubit_connectivity()
    );

    // A device target routes, so TargetCleanup (when it runs) must also
    // preserve connectivity.
    let device_workflow = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: Device::bidirectional_line("wiring-device", 2).unwrap(),
            initial_layout: None,
            seed: Some(47),
        }),
        resource_policy: ResourcePolicy::default(),
    });
    assert!(
        device_workflow
            .rewrite_config(RewritePhase::TargetCleanup)
            .unwrap()
            .preserve_two_qubit_connectivity()
    );
}

#[test]
fn post_routing_cleanup_does_not_collapse_bridge() {
    // Direct regression for the fix: hand-build an already-routed circuit that
    // contains the four-CX bridge pattern on physical qubits, then run the
    // workflow's actual post-routing cleanup. The connectivity policy must keep
    // the pattern intact instead of collapsing it to the non-adjacent CX(0,2).
    let (q0, q1, q2) = (Qubit::new(0), Qubit::new(1), Qubit::new(2));
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q1, q2).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q1, q2).unwrap();

    let workflow = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: Device::bidirectional_line("cleanup-bridge", 3).unwrap(),
            initial_layout: None,
            seed: Some(47),
        }),
        resource_policy: ResourcePolicy::default(),
    });

    let mut state = workflow_state_without_target_basis();
    state.analysis = workflow_analysis(&circuit);
    state.current = circuit;

    workflow.apply_post_routing_cleanup(&mut state).unwrap();

    // Without the policy the four CX collapse to a single CX(0,2); with the
    // policy they must all remain on the {0,1}/{1,2} line.
    assert_eq!(standard_ops(&state.current), vec![StandardGate::CX; 4]);
    assert_two_qubit_gates_on_line(&state.current);
}

#[test]
fn validate_topology_target_rejects_non_topological_two_qubit_gate() {
    // The backstop must catch a topology violation even when the filter
    // fails: CZ is in the loose device's native basis, but {0,2} is not an
    // edge of the 0-1-2 line.
    let mut circuit = Circuit::new(3);
    circuit.cz(Qubit::new(0), Qubit::new(2)).unwrap();

    let workflow = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::TopologyBasis {
            device_target: DeviceCompileTarget {
                device: Device::bidirectional_line("validate-topology", 3).unwrap(),
                initial_layout: None,
                seed: Some(47),
            },
            basis: vec![
                Instruction::Standard(StandardGate::H),
                Instruction::Standard(StandardGate::CZ),
            ],
        },
        resource_policy: ResourcePolicy::default(),
    });

    let mut state = workflow_state_without_target_basis();
    state.analysis = workflow_analysis(&circuit);
    state.current = circuit;

    assert!(workflow.validate_topology_target(&mut state).is_err());
}

#[test]
fn validate_topology_target_accepts_topological_circuit() {
    let mut circuit = Circuit::new(3);
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cz(Qubit::new(1), Qubit::new(2)).unwrap();

    let workflow = CompilerWorkflow::new(CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::TopologyBasis {
            device_target: DeviceCompileTarget {
                device: Device::bidirectional_line("validate-topology-ok", 3).unwrap(),
                initial_layout: None,
                seed: Some(47),
            },
            basis: vec![
                Instruction::Standard(StandardGate::H),
                Instruction::Standard(StandardGate::CZ),
            ],
        },
        resource_policy: ResourcePolicy::default(),
    });

    let mut state = workflow_state_without_target_basis();
    state.analysis = workflow_analysis(&circuit);
    state.current = circuit;

    workflow.validate_topology_target(&mut state).unwrap();
}

#[test]
fn try_new_reports_invalid_target_basis_during_construction() {
    let workflow = CompilerWorkflow::try_new(CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Basis(vec![Instruction::McGate(Box::new(MCGate::new(
            1,
            StandardGate::X,
        )))]),
        resource_policy: ResourcePolicy::default(),
    });

    assert!(matches!(workflow, Err(CompilerError::InvalidInput(_))));
}

#[test]
fn topology_basis_workflow_reuses_prepared_target_across_mixed_runs() {
    let config = CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::TopologyBasis {
            device_target: DeviceCompileTarget {
                device: Device::bidirectional_line("cached-topology-target", 3).unwrap(),
                initial_layout: None,
                seed: Some(101),
            },
            basis: vec![
                Instruction::Standard(StandardGate::H),
                Instruction::Standard(StandardGate::CZ),
            ],
        },
        resource_policy: ResourcePolicy::default(),
    };
    let workflow = CompilerWorkflow::try_new(config.clone()).unwrap();
    let prepared = workflow.prepared_target().unwrap();
    let basis = Arc::clone(prepared.target_basis.as_ref().unwrap());
    let physical = Arc::clone(prepared.routing_physical.as_ref().unwrap());
    let routing = Arc::clone(prepared.topology_routing_target.as_ref().unwrap());

    let mut first_circuit = Circuit::new(3);
    first_circuit.h(Qubit::new(0)).unwrap();
    first_circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
    let mut second_circuit = Circuit::new(3);
    second_circuit.h(Qubit::new(1)).unwrap();
    second_circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();

    let first = workflow.run(&first_circuit).unwrap();
    let repeated = workflow.run(&first_circuit).unwrap();
    let second = workflow.run(&second_circuit).unwrap();
    let fresh_second = CompilerWorkflow::try_new(config)
        .unwrap()
        .run(&second_circuit)
        .unwrap();

    assert_eq!(first, repeated);
    assert_eq!(second, fresh_second);
    let prepared = workflow.prepared_target().unwrap();
    assert!(Arc::ptr_eq(&basis, prepared.target_basis.as_ref().unwrap()));
    assert!(Arc::ptr_eq(
        &physical,
        prepared.routing_physical.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(
        &routing,
        prepared.topology_routing_target.as_ref().unwrap()
    ));
}

#[test]
fn device_workflow_reuses_prepared_target_across_mixed_runs() {
    let device = Device::bidirectional_line("cached-device-target", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let config = CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: device.clone(),
            initial_layout: None,
            seed: Some(103),
        }),
        resource_policy: ResourcePolicy::default(),
    };
    let workflow = CompilerWorkflow::try_new(config.clone()).unwrap();
    let prepared = workflow.prepared_target().unwrap();
    let physical = Arc::clone(prepared.routing_physical.as_ref().unwrap());
    let planning_session = Arc::clone(prepared.planning_session.as_ref().unwrap());

    let mut first_circuit = Circuit::new(3);
    first_circuit.h(Qubit::new(0)).unwrap();
    first_circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
    let mut second_circuit = Circuit::new(3);
    second_circuit.h(Qubit::new(2)).unwrap();
    second_circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();

    let first = workflow.run(&first_circuit).unwrap();
    let repeated = workflow.run(&first_circuit).unwrap();
    let second = workflow.run(&second_circuit).unwrap();
    let fresh_second = CompilerWorkflow::try_new(config)
        .unwrap()
        .run(&second_circuit)
        .unwrap();

    assert_eq!(first, repeated);
    assert_eq!(second, fresh_second);
    device.validate_circuit(&first.circuit).unwrap();
    device.validate_circuit(&second.circuit).unwrap();
    let prepared = workflow.prepared_target().unwrap();
    assert!(Arc::ptr_eq(
        &physical,
        prepared.routing_physical.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(
        &planning_session,
        prepared.planning_session.as_ref().unwrap()
    ));
}

#[test]
fn shared_device_workflow_supports_concurrent_mixed_runs() {
    const WORKERS: usize = 4;
    let device = Device::bidirectional_line("concurrent-cached-device-target", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let config = CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device,
            initial_layout: None,
            seed: Some(107),
        }),
        resource_policy: ResourcePolicy::default(),
    };
    let mut first_circuit = Circuit::new(3);
    first_circuit.h(Qubit::new(0)).unwrap();
    first_circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
    let mut second_circuit = Circuit::new(3);
    second_circuit.h(Qubit::new(2)).unwrap();
    second_circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
    let circuits = Arc::new([first_circuit, second_circuit]);
    let expected = circuits
        .iter()
        .map(|circuit| {
            CompilerWorkflow::try_new(config.clone())
                .unwrap()
                .run(circuit)
                .unwrap()
        })
        .collect::<Vec<_>>();
    let expected = Arc::new(expected);
    let workflow = Arc::new(CompilerWorkflow::try_new(config).unwrap());
    let barrier = Arc::new(Barrier::new(WORKERS));

    let handles = (0..WORKERS)
        .map(|worker| {
            let circuits = Arc::clone(&circuits);
            let expected = Arc::clone(&expected);
            let workflow = Arc::clone(&workflow);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let circuit_index = worker % circuits.len();
                barrier.wait();
                let result = workflow.run(&circuits[circuit_index]).unwrap();
                assert_eq!(result, expected[circuit_index]);
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.join().unwrap();
    }
}
