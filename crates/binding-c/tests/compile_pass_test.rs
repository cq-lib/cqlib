//! Integration tests for the compile pass C ABI: layout, routing, transform,
//! and low-level SABRE entry points.

#![allow(dead_code)]

use binding_c::circuit::{
    circuit_cx, circuit_free, circuit_h, circuit_new, circuit_num_operations, circuit_s,
    circuit_sdg, circuit_t, circuit_tdg,
};
use binding_c::compile::*;
use binding_c::cqlib_string_free;
use binding_c::device::*;
use std::ffi::CString;

fn name(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn zeroed<T>() -> T {
    unsafe { std::mem::zeroed() }
}

/// Directed line 0 -> 1 -> 2 built from an edge list.
fn line3_device(dev_name: &CString) -> *mut CDevice {
    let edges: [u32; 4] = [0, 1, 1, 2];
    device_from_edges(dev_name.as_ptr(), 3, edges.as_ptr(), 2)
}

fn identity_layout(n: usize) -> *mut CLayout {
    let qubits: Vec<u32> = (0..n as u32).collect();
    layout_new(qubits.as_ptr(), n, qubits.as_ptr(), n)
}

// =====  Layout entry points  =====

#[test]
fn test_layout_entry_points() {
    let dev_name = name("line-3");
    let device = line3_device(&dev_name);
    assert!(!device.is_null());

    let circuit = circuit_new(3);
    assert_eq!(circuit_cx(circuit, 0, 2), 0);

    // Configuration constructors are callable and match the documented
    // defaults.
    assert_eq!(vf2_layout_config_default().candidate_limit, 10);
    assert_eq!(sabre_config_default().routing_trials, 1);

    // Trivial layout maps logical i to physical i.
    let trivial = trivial_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY);
    assert!(!trivial.is_null());
    let layout = layout_result_layout(trivial);
    assert!(!layout.is_null());
    assert_eq!(layout_num_logical(layout), 3);
    assert_eq!(layout_get(layout, 0), 0);
    assert_eq!(layout_get(layout, 2), 2);
    assert!(layout_result_candidates_evaluated(trivial) >= 1);
    // Every note is a valid C string.
    for i in 0..layout_result_notes_len(trivial) {
        let note = layout_result_note(trivial, i);
        assert!(!note.is_null());
        cqlib_string_free(note);
    }
    layout_free(layout);
    layout_result_free(trivial);

    // Greedy layout succeeds on the same circuit.
    let greedy = greedy_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY);
    assert!(!greedy.is_null());
    layout_result_free(greedy);

    // SABRE layout with the default config.
    let sabre = sabre_layout(
        circuit,
        device,
        LAYOUT_OBJECTIVE_TOPOLOGY_ONLY,
        std::ptr::null(),
    );
    assert!(!sabre.is_null());
    assert!(layout_result_candidates_evaluated(sabre) >= 1);
    layout_result_free(sabre);

    // Unknown objective tag is rejected.
    assert!(trivial_layout(circuit, device, 99).is_null());

    circuit_free(circuit);
    device_free(device);
}

#[test]
fn test_vf2_perfect_layout() {
    let dev_name = name("line-3");
    let device = line3_device(&dev_name);
    assert!(!device.is_null());

    // Interactions (0, 1) and (1, 2) fit the line perfectly.
    let circuit = circuit_new(3);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    assert_eq!(circuit_cx(circuit, 1, 2), 0);

    let result = vf2_perfect_layout(
        circuit,
        device,
        LAYOUT_OBJECTIVE_TOPOLOGY_ONLY,
        std::ptr::null(),
    );
    assert!(!result.is_null());
    assert_eq!(layout_result_is_perfect(result), 1);
    layout_result_free(result);

    circuit_free(circuit);
    device_free(device);
}

#[test]
fn test_layout_analysis_and_scoring() {
    let dev_name = name("line-3");
    let device = line3_device(&dev_name);
    assert!(!device.is_null());

    let circuit = circuit_new(3);
    assert_eq!(circuit_cx(circuit, 0, 2), 0);
    assert_eq!(circuit_cx(circuit, 0, 2), 0);

    let analysis = analyze_circuit_for_layout(circuit);
    assert!(!analysis.is_null());
    assert_eq!(circuit_layout_analysis_num_logical(analysis), 3);
    let mut qubits = [0u32; 3];
    assert_eq!(
        circuit_layout_analysis_logical_qubits(analysis, qubits.as_mut_ptr(), 3),
        3
    );
    assert_eq!(qubits, [0, 1, 2]);
    // Both operations collapse into one weighted interaction pair.
    assert_eq!(circuit_layout_analysis_interactions_len(analysis), 1);
    let mut interaction: CInteraction = zeroed();
    assert_eq!(
        circuit_layout_analysis_interaction(analysis, 0, &mut interaction),
        0
    );
    assert_eq!(interaction.left, 0);
    assert_eq!(interaction.right, 2);
    assert_eq!(interaction.weight, 2.0);
    assert_eq!(
        circuit_layout_analysis_interaction(analysis, 5, &mut interaction),
        -8
    );

    let graph = physical_layout_graph_from_device(device);
    assert!(!graph.is_null());
    assert_eq!(physical_layout_graph_num_physical(graph), 3);
    assert_eq!(physical_layout_graph_distance(graph, 0, 2), 2);
    assert_eq!(physical_layout_graph_distance(graph, 0, 1), 1);
    assert_eq!(physical_layout_graph_is_adjacent_undirected(graph, 0, 2), 0);
    assert_eq!(physical_layout_graph_is_adjacent_undirected(graph, 1, 0), 1);
    assert_eq!(
        physical_layout_graph_supports_directed_coupling(graph, 0, 1),
        1
    );
    assert_eq!(
        physical_layout_graph_supports_directed_coupling(graph, 1, 0),
        0
    );
    assert_eq!(physical_layout_graph_has_fidelity_data(graph), 0);

    let trivial = trivial_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY);
    assert!(!trivial.is_null());
    let layout = layout_result_layout(trivial);
    assert!(!layout.is_null());

    let mut score: CLayoutScore = zeroed();
    assert_eq!(
        score_layout(
            LAYOUT_OBJECTIVE_TOPOLOGY_ONLY,
            analysis,
            graph,
            layout,
            &mut score
        ),
        0
    );
    assert!(score.total > 0.0);
    assert_eq!(score_layout(99, analysis, graph, layout, &mut score), -8);

    layout_free(layout);
    layout_result_free(trivial);
    physical_layout_graph_free(graph);
    circuit_layout_analysis_free(analysis);
    circuit_free(circuit);
    device_free(device);
}

#[test]
fn test_prepared_layout_pipeline() {
    let dev_name = name("line-3");
    let device = line3_device(&dev_name);
    assert!(!device.is_null());
    // Exact device-native SABRE requires the device to declare routing
    // capability: native gates, or per-qubit/edge native instructions.
    let gates = name("H,CX,SWAP");
    assert_eq!(device_with_native_gates(device, gates.as_ptr()), 0);

    let circuit = circuit_new(3);
    assert_eq!(circuit_cx(circuit, 0, 2), 0);

    let analysis = analyze_circuit_for_layout(circuit);
    assert!(!analysis.is_null());
    let graph = physical_layout_graph_from_device(device);
    assert!(!graph.is_null());

    let trivial = trivial_layout_prepared(analysis, graph, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY);
    assert!(!trivial.is_null());
    layout_result_free(trivial);

    // Prepared SABRE circuit/target pipeline.
    let prepared = prepare_sabre_circuit(circuit);
    assert!(!prepared.is_null());
    assert_eq!(prepared_sabre_circuit_logical_qubits_len(prepared), 3);
    let mut logical = [0u32; 3];
    assert_eq!(
        prepared_sabre_circuit_logical_qubits(prepared, logical.as_mut_ptr(), 3),
        3
    );
    assert_eq!(logical, [0, 1, 2]);

    let target = prepare_sabre_device_target(prepared, device);
    assert!(!target.is_null());
    let target_graph = prepared_sabre_target_physical(target);
    assert!(!target_graph.is_null());
    assert_eq!(physical_layout_graph_num_physical(target_graph), 3);
    physical_layout_graph_free(target_graph);

    let sabre = sabre_layout_prepared(
        prepared,
        target,
        LAYOUT_OBJECTIVE_TOPOLOGY_ONLY,
        std::ptr::null(),
    );
    assert!(!sabre.is_null());
    layout_result_free(sabre);

    prepared_sabre_target_free(target);
    prepared_sabre_circuit_free(prepared);
    physical_layout_graph_free(graph);
    circuit_layout_analysis_free(analysis);
    circuit_free(circuit);
    device_free(device);
}

// =====  Routing  =====

#[test]
fn test_route_with_layout() {
    let dev_name = name("line-3");
    let device = line3_device(&dev_name);
    assert!(!device.is_null());

    let circuit = circuit_new(3);
    assert_eq!(circuit_cx(circuit, 0, 2), 0);
    let layout = identity_layout(3);
    assert!(!layout.is_null());

    // Identity layout needs at least one SWAP for cx(0,2) on the line.
    let routed = route_with_layout(circuit, device, layout, std::ptr::null());
    assert!(!routed.is_null());
    assert!(routed_circuit_swap_count(routed) >= 1);
    assert_eq!(routed_circuit_changed(routed, circuit), 1);

    let final_layout = routed_circuit_final_layout(routed);
    assert!(!final_layout.is_null());
    assert_eq!(layout_num_logical(final_layout), 3);
    layout_free(final_layout);

    let initial_layout = routed_circuit_initial_layout(routed);
    assert!(!initial_layout.is_null());
    layout_free(initial_layout);

    let physical = routed_circuit_circuit(routed);
    assert!(!physical.is_null());
    assert!(circuit_num_operations(physical) >= 1);
    circuit_free(physical);

    let mut diagnostics: CSabreRoutingDiagnostics = zeroed();
    assert_eq!(routed_circuit_diagnostics(routed, &mut diagnostics), 0);
    assert!(diagnostics.trials_evaluated >= 1);

    routed_circuit_free(routed);
    layout_free(layout);
    circuit_free(circuit);
    device_free(device);
}

#[test]
fn test_route_sabre() {
    let dev_name = name("line-3");
    let device = line3_device(&dev_name);
    assert!(!device.is_null());

    let circuit = circuit_new(3);
    assert_eq!(circuit_cx(circuit, 0, 2), 0);

    let result = route_sabre(
        circuit,
        device,
        LAYOUT_OBJECTIVE_TOPOLOGY_ONLY,
        std::ptr::null(),
    );
    assert!(!result.is_null());
    assert!(sabre_route_result_layout_candidates_evaluated(result) >= 1);
    for i in 0..sabre_route_result_layout_notes_len(result) {
        let note = sabre_route_result_layout_note(result, i);
        assert!(!note.is_null());
        cqlib_string_free(note);
    }

    // The layout score is recorded for the selected initial layout.
    let mut score: CLayoutScore = zeroed();
    assert_eq!(sabre_route_result_layout_score(result, &mut score), 0);

    // The nested routed circuit exposes the same metadata.
    let routed = sabre_route_result_routed(result);
    assert!(!routed.is_null());
    assert_eq!(
        sabre_route_result_swap_count(result),
        routed_circuit_swap_count(routed)
    );
    routed_circuit_free(routed);

    let physical = sabre_route_result_circuit(result);
    assert!(!physical.is_null());
    circuit_free(physical);

    let mut diagnostics: CSabreRoutingDiagnostics = zeroed();
    assert_eq!(sabre_route_result_diagnostics(result, &mut diagnostics), 0);

    let final_layout = sabre_route_result_final_layout(result);
    assert!(!final_layout.is_null());
    layout_free(final_layout);

    sabre_route_result_free(result);
    circuit_free(circuit);
    device_free(device);
}

// =====  Low-level SABRE  =====

#[test]
fn test_sabre_route_low_level() {
    let dev_name = name("line-3");
    let device = line3_device(&dev_name);
    assert!(!device.is_null());

    let circuit = circuit_new(3);
    assert_eq!(circuit_cx(circuit, 0, 2), 0);
    let layout = identity_layout(3);
    assert!(!layout.is_null());

    let result = sabre_route(circuit, device, layout, std::ptr::null());
    assert!(!result.is_null());
    assert!(sabre_routing_result_swap_count(result) >= 1);

    let physical = sabre_routing_result_circuit(result);
    assert!(!physical.is_null());
    assert!(circuit_num_operations(physical) >= 1);
    circuit_free(physical);

    let initial = sabre_routing_result_initial_layout(result);
    assert!(!initial.is_null());
    assert_eq!(layout_num_logical(initial), 3);
    layout_free(initial);

    let final_layout = sabre_routing_result_final_layout(result);
    assert!(!final_layout.is_null());
    layout_free(final_layout);

    let mut diagnostics: CSabreRoutingDiagnostics = zeroed();
    assert_eq!(
        sabre_routing_result_diagnostics(result, &mut diagnostics),
        0
    );
    assert!(diagnostics.trials_evaluated >= 1);

    sabre_routing_result_free(result);
    layout_free(layout);
    circuit_free(circuit);
    device_free(device);
}

#[test]
fn test_normalize_and_validate_reachable() {
    let dev_name = name("line-3");
    let device = line3_device(&dev_name);
    assert!(!device.is_null());

    let logical: [u32; 3] = [0, 1, 2];
    let layout = identity_layout(3);
    assert!(!layout.is_null());

    // normalize_initial_layout accepts an already-normal identity layout.
    let normalized = normalize_initial_layout(logical.as_ptr(), 3, device, layout);
    assert!(!normalized.is_null());
    assert_eq!(layout_num_logical(normalized), 3);
    layout_free(normalized);

    // Adjacent interactions are reachable from the identity layout.
    let adjacent = circuit_new(3);
    assert_eq!(circuit_cx(adjacent, 0, 1), 0);
    assert_eq!(validate_reachable_interactions(adjacent, device, layout), 0);
    // Distant but connected interactions remain routable.
    assert_eq!(circuit_cx(adjacent, 0, 2), 0);
    assert_eq!(validate_reachable_interactions(adjacent, device, layout), 0);
    circuit_free(adjacent);

    layout_free(layout);
    device_free(device);

    // A qubit with no couplings cannot host a two-qubit interaction.
    let iso_name = name("iso-4");
    let edges: [u32; 4] = [0, 1, 1, 2];
    let iso_device = device_from_edges(iso_name.as_ptr(), 4, edges.as_ptr(), 2);
    assert!(!iso_device.is_null());
    let iso_layout = identity_layout(4);
    let split = circuit_new(4);
    assert_eq!(circuit_cx(split, 0, 3), 0);
    assert_eq!(
        validate_reachable_interactions(split, iso_device, iso_layout),
        -6
    );
    circuit_free(split);
    layout_free(iso_layout);
    device_free(iso_device);
}

// =====  Transforms  =====

#[test]
fn test_canonicalize() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let result = canonicalize_circuit(circuit);
    assert!(!result.is_null());
    assert!(canonicalize_result_rounds(result) >= 1);
    let out = canonicalize_result_circuit(result);
    assert!(!out.is_null());
    assert_eq!(circuit_num_operations(out), 2);
    assert!(canonicalize_result_changed(result) == 0 || canonicalize_result_changed(result) == 1);
    circuit_free(out);
    canonicalize_result_free(result);

    // The transform wrapper returns a new circuit handle.
    let wrapped = transform_canonicalize(circuit);
    assert!(!wrapped.is_null());
    assert_eq!(circuit_num_operations(wrapped), 2);
    circuit_free(wrapped);

    circuit_free(circuit);
}

#[test]
fn test_rewrite_circuit() {
    let circuit = circuit_new(1);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_h(circuit, 0), 0);

    let config = rewrite_config_default();
    assert_eq!(config.mode, REWRITE_MODE_OPTIMIZE);
    let result = rewrite_circuit(circuit, config);
    assert!(!result.is_null());

    let changed = knowledge_rewrite_result_changed(result);
    assert!(changed == 0 || changed == 1);
    let out = knowledge_rewrite_result_circuit(result);
    assert!(!out.is_null());
    if changed == 1 {
        assert!(circuit_num_operations(out) <= 2);
    }

    let mut stats: CKnowledgeRewriteStats = zeroed();
    assert_eq!(knowledge_rewrite_result_stats(result, &mut stats), 0);
    let mut diagnostics: CKnowledgeRewriteDiagnostics = zeroed();
    assert_eq!(
        knowledge_rewrite_result_diagnostics(result, &mut diagnostics),
        0
    );
    if changed == 1 {
        assert!(stats.rules_applied >= 1);
    }

    circuit_free(out);
    knowledge_rewrite_result_free(result);

    // Lowering-mode config and the transform wrapper are callable.
    let lowering = rewrite_config_lowering();
    assert_eq!(lowering.mode, REWRITE_MODE_LOWERING);
    let wrapped = transform_knowledge_rewrite(circuit, rewrite_config_default());
    assert!(!wrapped.is_null());
    circuit_free(wrapped);

    circuit_free(circuit);
}

#[test]
fn test_transform_wrappers() {
    // A one-qubit run collapses: the optimized stream never grows.
    let runs = circuit_new(1);
    assert_eq!(circuit_h(runs, 0), 0);
    assert_eq!(circuit_t(runs, 0), 0);
    assert_eq!(circuit_tdg(runs, 0), 0);
    assert_eq!(circuit_s(runs, 0), 0);
    assert_eq!(circuit_sdg(runs, 0), 0);
    assert_eq!(circuit_num_operations(runs), 5);

    let optimized = transform_optimize_one_qubit_runs(runs);
    assert!(!optimized.is_null());
    assert!(circuit_num_operations(optimized) <= 5);
    assert!(circuit_num_operations(optimized) >= 1);
    circuit_free(optimized);
    circuit_free(runs);

    // Commutative cancellation and routing-basis lowering smoke runs.
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let cancelled = transform_commutative_cancellation(circuit);
    assert!(!cancelled.is_null());
    assert!(circuit_num_operations(cancelled) >= 1);
    circuit_free(cancelled);

    let lowered = transform_lower_to_routing_basis(circuit);
    assert!(!lowered.is_null());
    assert!(circuit_num_operations(lowered) >= 1);
    circuit_free(lowered);

    circuit_free(circuit);
}

#[test]
fn test_circuit_analyze() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let mut analysis: CCircuitAnalysis = zeroed();
    assert_eq!(circuit_analyze(circuit, &mut analysis), 0);
    assert_eq!(analysis.has_classical_data, 0);
    assert_eq!(analysis.has_classical_control, 0);
    assert_eq!(analysis.has_measurement, 0);
    assert_eq!(analysis.has_classical_values, 0);
    assert_eq!(analysis.has_classical_vars, 0);
    assert_eq!(analysis.has_runtime_classical, 0);
    assert_eq!(analysis.needs_classical_handle_preservation, 0);
    assert_eq!(analysis.has_circuit_gate_definitions, 0);
    assert_eq!(analysis.has_unitary_circuit_definitions, 0);
    assert_eq!(analysis.has_unitary_gates, 0);
    assert_eq!(analysis.has_mc_gates, 0);

    circuit_free(circuit);
}

// =====  NULL guards  =====

#[test]
fn test_null_guards() {
    // Layout entry points.
    assert!(trivial_layout(std::ptr::null(), std::ptr::null(), 0).is_null());
    assert!(greedy_layout(std::ptr::null(), std::ptr::null(), 0).is_null());
    assert!(vf2_perfect_layout(std::ptr::null(), std::ptr::null(), 0, std::ptr::null()).is_null());
    assert!(sabre_layout(std::ptr::null(), std::ptr::null(), 0, std::ptr::null()).is_null());
    assert!(analyze_circuit_for_layout(std::ptr::null()).is_null());
    assert!(physical_layout_graph_from_device(std::ptr::null()).is_null());
    assert!(prepare_sabre_circuit(std::ptr::null()).is_null());
    assert!(prepare_sabre_device_target(std::ptr::null(), std::ptr::null()).is_null());
    assert!(trivial_layout_prepared(std::ptr::null(), std::ptr::null(), 0).is_null());
    assert!(
        sabre_layout_prepared(std::ptr::null(), std::ptr::null(), 0, std::ptr::null()).is_null()
    );
    assert_eq!(
        score_layout(
            0,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut()
        ),
        -1
    );

    // Routing entry points.
    assert!(
        route_with_layout(
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null()
        )
        .is_null()
    );
    assert!(route_sabre(std::ptr::null(), std::ptr::null(), 0, std::ptr::null()).is_null());
    assert!(
        sabre_route(
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null()
        )
        .is_null()
    );
    assert!(
        normalize_initial_layout(std::ptr::null(), 0, std::ptr::null(), std::ptr::null()).is_null()
    );
    assert_eq!(
        validate_reachable_interactions(std::ptr::null(), std::ptr::null(), std::ptr::null()),
        -1
    );

    // Transform entry points.
    assert!(canonicalize_circuit(std::ptr::null()).is_null());
    assert!(rewrite_circuit(std::ptr::null(), rewrite_config_default()).is_null());
    assert!(transform_canonicalize(std::ptr::null()).is_null());
    assert!(transform_knowledge_rewrite(std::ptr::null(), rewrite_config_default()).is_null());
    assert!(transform_optimize_one_qubit_runs(std::ptr::null()).is_null());
    assert!(transform_commutative_cancellation(std::ptr::null()).is_null());
    assert!(transform_lower_to_routing_basis(std::ptr::null()).is_null());
    assert_eq!(circuit_analyze(std::ptr::null(), std::ptr::null_mut()), -1);

    // Accessors on NULL handles.
    assert!(layout_result_layout(std::ptr::null()).is_null());
    assert_eq!(layout_result_has_score(std::ptr::null()), -1);
    assert_eq!(layout_result_is_perfect(std::ptr::null()), -1);
    assert_eq!(layout_result_candidates_evaluated(std::ptr::null()), 0);
    assert_eq!(layout_result_notes_len(std::ptr::null()), 0);
    assert!(layout_result_note(std::ptr::null(), 0).is_null());
    assert!(routed_circuit_circuit(std::ptr::null()).is_null());
    assert!(routed_circuit_initial_layout(std::ptr::null()).is_null());
    assert!(routed_circuit_final_layout(std::ptr::null()).is_null());
    assert_eq!(routed_circuit_swap_count(std::ptr::null()), 0);
    assert_eq!(
        routed_circuit_diagnostics(std::ptr::null(), std::ptr::null_mut()),
        -1
    );
    assert_eq!(
        routed_circuit_changed(std::ptr::null(), std::ptr::null()),
        -1
    );
    assert!(sabre_route_result_routed(std::ptr::null()).is_null());
    assert!(sabre_route_result_circuit(std::ptr::null()).is_null());
    assert!(sabre_route_result_initial_layout(std::ptr::null()).is_null());
    assert!(sabre_route_result_final_layout(std::ptr::null()).is_null());
    assert_eq!(sabre_route_result_swap_count(std::ptr::null()), 0);
    assert_eq!(
        sabre_route_result_changed(std::ptr::null(), std::ptr::null()),
        -1
    );
    assert_eq!(
        sabre_route_result_layout_score(std::ptr::null(), std::ptr::null_mut()),
        -1
    );
    assert_eq!(sabre_route_result_layout_is_perfect(std::ptr::null()), -1);
    assert_eq!(
        sabre_route_result_layout_candidates_evaluated(std::ptr::null()),
        0
    );
    assert_eq!(
        sabre_route_result_layout_used_fidelity(std::ptr::null()),
        -1
    );
    assert_eq!(sabre_route_result_layout_notes_len(std::ptr::null()), 0);
    assert!(sabre_route_result_layout_note(std::ptr::null(), 0).is_null());
    assert!(sabre_routing_result_circuit(std::ptr::null()).is_null());
    assert_eq!(sabre_routing_result_swap_count(std::ptr::null()), 0);
    assert_eq!(
        sabre_routing_result_diagnostics(std::ptr::null(), std::ptr::null_mut()),
        -1
    );
    assert!(canonicalize_result_circuit(std::ptr::null()).is_null());
    assert_eq!(canonicalize_result_changed(std::ptr::null()), -1);
    assert_eq!(canonicalize_result_rounds(std::ptr::null()), 0);
    assert!(knowledge_rewrite_result_circuit(std::ptr::null()).is_null());
    assert_eq!(knowledge_rewrite_result_changed(std::ptr::null()), -1);
    assert_eq!(
        knowledge_rewrite_result_stats(std::ptr::null(), std::ptr::null_mut()),
        -1
    );
    assert_eq!(
        knowledge_rewrite_result_diagnostics(std::ptr::null(), std::ptr::null_mut()),
        -1
    );
    assert_eq!(circuit_layout_analysis_num_logical(std::ptr::null()), 0);
    assert_eq!(
        circuit_layout_analysis_interactions_len(std::ptr::null()),
        0
    );
    assert_eq!(
        circuit_layout_analysis_interaction(std::ptr::null(), 0, std::ptr::null_mut()),
        -1
    );
    assert_eq!(physical_layout_graph_num_physical(std::ptr::null()), 0);
    assert_eq!(
        physical_layout_graph_distance(std::ptr::null(), 0, 1),
        u32::MAX
    );
    assert_eq!(
        prepared_sabre_circuit_logical_qubits_len(std::ptr::null()),
        0
    );
    assert!(prepared_sabre_target_physical(std::ptr::null()).is_null());
    assert_eq!(sabre_config_validate(std::ptr::null()), -1);

    // Free functions accept NULL.
    layout_result_free(std::ptr::null_mut());
    circuit_layout_analysis_free(std::ptr::null_mut());
    physical_layout_graph_free(std::ptr::null_mut());
    prepared_sabre_circuit_free(std::ptr::null_mut());
    prepared_sabre_target_free(std::ptr::null_mut());
    routed_circuit_free(std::ptr::null_mut());
    sabre_route_result_free(std::ptr::null_mut());
    sabre_routing_result_free(std::ptr::null_mut());
    canonicalize_result_free(std::ptr::null_mut());
    knowledge_rewrite_result_free(std::ptr::null_mut());
}
