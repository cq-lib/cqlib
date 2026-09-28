// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// use this file except in compliance with the License. You may obtain
// a copy of the License in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "cqlib_c.h"

// Directed line 0 -> 1 -> 2 built from an edge list.
static CDevice* build_line3_device(const char* name) {
    uint32_t edges[4] = {0, 1, 1, 2};
    return device_from_edges(name, 3, edges, 2);
}

// Circuit with one cx(0, 2) interaction that does not fit the line.
static CCircuit* build_line3_circuit(void) {
    CCircuit* circuit = circuit_new(3);
    assert(circuit != NULL);
    assert(circuit_cx(circuit, 0, 2) == 0);
    return circuit;
}

static void test_layout_passes(void) {
    CDevice* device = build_line3_device("line-3");
    CCircuit* circuit = build_line3_circuit();
    assert(device != NULL);

    // Trivial layout maps logical i to physical i.
    CLayoutResult* trivial = trivial_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY);
    assert(trivial != NULL);
    CLayout* layout = layout_result_layout(trivial);
    assert(layout != NULL);
    assert(layout_num_logical(layout) == 3);
    assert(layout_get(layout, 0) == 0);
    assert(layout_get(layout, 2) == 2);
    layout_free(layout);
    layout_result_free(trivial);

    // Greedy layout runs on the same circuit.
    CLayoutResult* greedy = greedy_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY);
    assert(greedy != NULL);
    layout_result_free(greedy);

    // SABRE layout with the default config reports candidates.
    CLayoutResult* sabre = sabre_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL);
    assert(sabre != NULL);
    assert(layout_result_candidates_evaluated(sabre) >= 1);
    layout_result_free(sabre);

    // Unknown objective tag is rejected.
    assert(trivial_layout(circuit, device, 99) == NULL);

    circuit_free(circuit);
    device_free(device);
}

static void test_vf2_layout(void) {
    CDevice* device = build_line3_device("line-3");
    assert(device != NULL);

    // Interactions (0, 1) and (1, 2) fit the line perfectly.
    CCircuit* circuit = circuit_new(3);
    assert(circuit != NULL);
    assert(circuit_cx(circuit, 0, 1) == 0);
    assert(circuit_cx(circuit, 1, 2) == 0);

    CLayoutResult* result =
        vf2_perfect_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL);
    assert(result != NULL);
    assert(layout_result_is_perfect(result) == 1);
    layout_result_free(result);

    circuit_free(circuit);
    device_free(device);
}

static void test_analysis_and_prepared(void) {
    CDevice* device = build_line3_device("line-3");
    CCircuit* circuit = build_line3_circuit();
    assert(device != NULL);
    // Exact device-native SABRE requires declared routing capability.
    assert(device_with_native_gates(device, "H,CX,SWAP") == 0);

    // Layout analysis collapses both operations into one interaction pair.
    CCircuitLayoutAnalysis* analysis = analyze_circuit_for_layout(circuit);
    assert(analysis != NULL);
    assert(circuit_layout_analysis_num_logical(analysis) == 3);
    assert(circuit_layout_analysis_interactions_len(analysis) == 1);
    CInteraction interaction;
    memset(&interaction, 0, sizeof(interaction));
    assert(circuit_layout_analysis_interaction(analysis, 0, &interaction) == 0);
    assert(interaction.left == 0);
    assert(interaction.right == 2);
    assert(interaction.weight == 1.0);

    // Physical graph view of the device.
    CPhysicalLayoutGraph* graph = physical_layout_graph_from_device(device);
    assert(graph != NULL);
    assert(physical_layout_graph_num_physical(graph) == 3);
    assert(physical_layout_graph_distance(graph, 0, 2) == 2);
    assert(physical_layout_graph_is_adjacent_undirected(graph, 0, 2) == 0);
    assert(physical_layout_graph_has_fidelity_data(graph) == 0);

    // Trivial layout over the prepared analysis and graph.
    CLayoutResult* trivial =
        trivial_layout_prepared(analysis, graph, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY);
    assert(trivial != NULL);
    layout_result_free(trivial);

    // Prepared SABRE circuit/target pipeline.
    CPreparedSabreCircuit* prepared = prepare_sabre_circuit(circuit);
    assert(prepared != NULL);
    CPreparedSabreTarget* target = prepare_sabre_device_target(prepared, device);
    assert(target != NULL);
    CPhysicalLayoutGraph* target_graph = prepared_sabre_target_physical(target);
    assert(target_graph != NULL);
    assert(physical_layout_graph_num_physical(target_graph) == 3);
    physical_layout_graph_free(target_graph);

    CLayoutResult* sabre_prepared =
        sabre_layout_prepared(prepared, target, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL);
    assert(sabre_prepared != NULL);
    layout_result_free(sabre_prepared);
    prepared_sabre_target_free(target);
    prepared_sabre_circuit_free(prepared);

    physical_layout_graph_free(graph);
    circuit_layout_analysis_free(analysis);
    circuit_free(circuit);
    device_free(device);
}

static void test_routing(void) {
    CDevice* device = build_line3_device("line-3");
    CCircuit* circuit = build_line3_circuit();
    assert(device != NULL);

    uint32_t logical[3] = {0, 1, 2};
    uint32_t physical[3] = {0, 1, 2};
    CLayout* layout = layout_new(logical, 3, physical, 3);
    assert(layout != NULL);

    // Identity layout needs at least one SWAP for cx(0, 2) on the line.
    CRoutedCircuit* routed = route_with_layout(circuit, device, layout, NULL);
    assert(routed != NULL);
    assert(routed_circuit_swap_count(routed) >= 1);
    assert(routed_circuit_changed(routed, circuit) == 1);

    CLayout* final_layout = routed_circuit_final_layout(routed);
    assert(final_layout != NULL);
    assert(layout_num_logical(final_layout) == 3);
    layout_free(final_layout);

    CCircuit* physical_circuit = routed_circuit_circuit(routed);
    assert(physical_circuit != NULL);
    assert(circuit_num_operations(physical_circuit) >= 1);
    circuit_free(physical_circuit);

    CSabreRoutingDiagnostics diagnostics;
    memset(&diagnostics, 0, sizeof(diagnostics));
    assert(routed_circuit_diagnostics(routed, &diagnostics) == 0);
    assert(diagnostics.trials_evaluated >= 1);

    routed_circuit_free(routed);
    layout_free(layout);

    // route_sabre selects a layout and routes in one call.
    CSabreRouteResult* result = route_sabre(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL);
    assert(result != NULL);
    assert(sabre_route_result_layout_candidates_evaluated(result) >= 1);

    CLayoutScore score;
    memset(&score, 0, sizeof(score));
    assert(sabre_route_result_layout_score(result, &score) == 0);

    CRoutedCircuit* inner = sabre_route_result_routed(result);
    assert(inner != NULL);
    assert(sabre_route_result_swap_count(result) == routed_circuit_swap_count(inner));
    routed_circuit_free(inner);
    sabre_route_result_free(result);

    circuit_free(circuit);
    device_free(device);
}

static void test_sabre_route_and_validate(void) {
    CDevice* device = build_line3_device("line-3");
    CCircuit* circuit = build_line3_circuit();
    assert(device != NULL);

    uint32_t logical[3] = {0, 1, 2};
    uint32_t physical[3] = {0, 1, 2};
    CLayout* layout = layout_new(logical, 3, physical, 3);
    assert(layout != NULL);

    // Low-level SABRE route from the identity layout.
    CSabreRoutingResult* result = sabre_route(circuit, device, layout, NULL);
    assert(result != NULL);
    assert(sabre_routing_result_swap_count(result) >= 1);

    CCircuit* physical_circuit = sabre_routing_result_circuit(result);
    assert(physical_circuit != NULL);
    circuit_free(physical_circuit);
    sabre_routing_result_free(result);

    // normalize_initial_layout accepts an already-normal identity layout.
    CLayout* normalized = normalize_initial_layout(logical, 3, device, layout);
    assert(normalized != NULL);
    assert(layout_num_logical(normalized) == 3);
    layout_free(normalized);

    // Connected interactions are reachable from the identity layout.
    CCircuit* adjacent = circuit_new(3);
    assert(circuit_cx(adjacent, 0, 1) == 0);
    assert(circuit_cx(adjacent, 0, 2) == 0);
    assert(validate_reachable_interactions(adjacent, device, layout) == 0);
    circuit_free(adjacent);

    layout_free(layout);
    circuit_free(circuit);
    device_free(device);

    // A qubit with no couplings cannot host a two-qubit interaction.
    uint32_t iso_edges[4] = {0, 1, 1, 2};
    CDevice* iso_device = device_from_edges("iso-4", 4, iso_edges, 2);
    assert(iso_device != NULL);
    uint32_t logical4[4] = {0, 1, 2, 3};
    uint32_t physical4[4] = {0, 1, 2, 3};
    CLayout* iso_layout = layout_new(logical4, 4, physical4, 4);
    assert(iso_layout != NULL);
    CCircuit* split = circuit_new(4);
    assert(circuit_cx(split, 0, 3) == 0);
    assert(validate_reachable_interactions(split, iso_device, iso_layout) == -6);
    circuit_free(split);
    layout_free(iso_layout);
    device_free(iso_device);
}

static void test_transforms(void) {
    // Canonicalize runs and reports its rounds.
    CCircuit* bell = circuit_new(2);
    assert(circuit_h(bell, 0) == 0);
    assert(circuit_cx(bell, 0, 1) == 0);

    CCanonicalizeResult* canon = canonicalize_circuit(bell);
    assert(canon != NULL);
    assert(canonicalize_result_rounds(canon) >= 1);
    CCircuit* canonical = canonicalize_result_circuit(canon);
    assert(canonical != NULL);
    assert(circuit_num_operations(canonical) == 2);
    circuit_free(canonical);
    canonicalize_result_free(canon);

    // Standalone canonicalize wrapper returns a new circuit.
    CCircuit* wrapped = transform_canonicalize(bell);
    assert(wrapped != NULL);
    assert(circuit_num_operations(wrapped) == 2);
    circuit_free(wrapped);
    circuit_free(bell);

    // One-qubit runs fuse: the optimized stream never grows.
    CCircuit* runs = circuit_new(1);
    assert(circuit_h(runs, 0) == 0);
    assert(circuit_t(runs, 0) == 0);
    assert(circuit_tdg(runs, 0) == 0);
    assert(circuit_s(runs, 0) == 0);
    assert(circuit_sdg(runs, 0) == 0);
    assert(circuit_num_operations(runs) == 5);
    CCircuit* optimized = transform_optimize_one_qubit_runs(runs);
    assert(optimized != NULL);
    assert(circuit_num_operations(optimized) <= 5);
    assert(circuit_num_operations(optimized) >= 1);
    circuit_free(optimized);
    circuit_free(runs);

    // Knowledge rewrite with the default configuration.
    CCircuit* twice_h = circuit_new(1);
    assert(circuit_h(twice_h, 0) == 0);
    assert(circuit_h(twice_h, 0) == 0);
    CKnowledgeRewriteResult* rewritten = rewrite_circuit(twice_h, rewrite_config_default());
    assert(rewritten != NULL);
    CKnowledgeRewriteStats stats;
    memset(&stats, 0, sizeof(stats));
    assert(knowledge_rewrite_result_stats(rewritten, &stats) == 0);
    CKnowledgeRewriteDiagnostics diagnostics;
    memset(&diagnostics, 0, sizeof(diagnostics));
    assert(knowledge_rewrite_result_diagnostics(rewritten, &diagnostics) == 0);
    knowledge_rewrite_result_free(rewritten);
    circuit_free(twice_h);

    // Commutative cancellation and routing-basis lowering smoke runs.
    CCircuit* simple = circuit_new(2);
    assert(circuit_h(simple, 0) == 0);
    assert(circuit_cx(simple, 0, 1) == 0);
    CCircuit* cancelled = transform_commutative_cancellation(simple);
    assert(cancelled != NULL);
    circuit_free(cancelled);
    CCircuit* lowered = transform_lower_to_routing_basis(simple);
    assert(lowered != NULL);
    circuit_free(lowered);
    circuit_free(simple);

    // Structural analysis snapshot.
    CCircuit* probe = circuit_new(2);
    assert(circuit_h(probe, 0) == 0);
    CCircuitAnalysis analysis;
    memset(&analysis, 0, sizeof(analysis));
    assert(circuit_analyze(probe, &analysis) == 0);
    assert(analysis.has_classical_control == 0);
    assert(analysis.has_unitary_gates == 0);
    circuit_free(probe);
}

static void test_null_guards(void) {
    assert(trivial_layout(NULL, NULL, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY) == NULL);
    assert(greedy_layout(NULL, NULL, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY) == NULL);
    assert(vf2_perfect_layout(NULL, NULL, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL) == NULL);
    assert(sabre_layout(NULL, NULL, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL) == NULL);
    assert(analyze_circuit_for_layout(NULL) == NULL);
    assert(physical_layout_graph_from_device(NULL) == NULL);
    assert(prepare_sabre_circuit(NULL) == NULL);
    assert(trivial_layout_prepared(NULL, NULL, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY) == NULL);
    assert(sabre_layout_prepared(NULL, NULL, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL) == NULL);
    assert(score_layout(LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL, NULL, NULL, NULL) == -1);

    assert(route_with_layout(NULL, NULL, NULL, NULL) == NULL);
    assert(route_sabre(NULL, NULL, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL) == NULL);
    assert(sabre_route(NULL, NULL, NULL, NULL) == NULL);
    assert(normalize_initial_layout(NULL, 0, NULL, NULL) == NULL);
    assert(validate_reachable_interactions(NULL, NULL, NULL) == -1);

    assert(canonicalize_circuit(NULL) == NULL);
    assert(rewrite_circuit(NULL, rewrite_config_default()) == NULL);
    assert(transform_canonicalize(NULL) == NULL);
    assert(transform_knowledge_rewrite(NULL, rewrite_config_default()) == NULL);
    assert(transform_optimize_one_qubit_runs(NULL) == NULL);
    assert(transform_commutative_cancellation(NULL) == NULL);
    assert(transform_lower_to_routing_basis(NULL) == NULL);
    assert(circuit_analyze(NULL, NULL) == -1);

    // Accessors on NULL handles.
    assert(layout_result_layout(NULL) == NULL);
    assert(layout_result_is_perfect(NULL) == -1);
    assert(layout_result_candidates_evaluated(NULL) == 0);
    assert(routed_circuit_circuit(NULL) == NULL);
    assert(routed_circuit_swap_count(NULL) == 0);
    assert(routed_circuit_changed(NULL, NULL) == -1);
    assert(sabre_route_result_routed(NULL) == NULL);
    assert(sabre_route_result_layout_score(NULL, NULL) == -1);
    assert(sabre_routing_result_swap_count(NULL) == 0);
    assert(canonicalize_result_rounds(NULL) == 0);
    assert(knowledge_rewrite_result_stats(NULL, NULL) == -1);
    assert(sabre_config_validate(NULL) == -1);

    // Free functions accept NULL.
    layout_result_free(NULL);
    circuit_layout_analysis_free(NULL);
    physical_layout_graph_free(NULL);
    prepared_sabre_circuit_free(NULL);
    prepared_sabre_target_free(NULL);
    routed_circuit_free(NULL);
    sabre_route_result_free(NULL);
    sabre_routing_result_free(NULL);
    canonicalize_result_free(NULL);
    knowledge_rewrite_result_free(NULL);
}

int main(void) {
    test_layout_passes();
    test_vf2_layout();
    test_analysis_and_prepared();
    test_routing();
    test_sabre_route_and_validate();
    test_transforms();
    test_null_guards();
    printf("binding-c compile pass tests passed\n");
    return 0;
}
