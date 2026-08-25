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
use super::*;
use crate::circuit::{ClassicalExpr, Qubit};
use crate::compile::test_utils::assert_compiled_circuit_equivalent;
use crate::compile::transform::decompose::unitary::{
    DeviceSynthesisPlacement, DeviceTwoQubitSynthesisContext, TwoQubitSynthesisTarget,
};
use crate::compile::transform::resynthesis::NativeResynthesisPolicy;
use crate::compile::transform::{ResolvedTransform, resolve_transform_for_test};
use crate::device::Device;

fn resynthesize_two_qubit_blocks(
    circuit: &Circuit,
    config: TwoQubitBlockResynthesisConfig,
) -> Result<ResolvedTransform, CompilerError> {
    super::resynthesize_two_qubit_blocks(circuit, config)
        .map(|outcome| resolve_transform_for_test(outcome, circuit))
}

fn resynthesize_two_qubit_blocks_with_cache_budget(
    circuit: &Circuit,
    config: TwoQubitBlockResynthesisConfig,
    budget: usize,
) -> Result<(ResolvedTransform, TwoQubitSynthesisCacheStats), CompilerError> {
    let mut synthesis_cache = TwoQubitSynthesisCache::new(budget);
    synthesis_cache.ensure_namespace(&config, None);
    let pass = ResynthesisPass {
        source: circuit,
        rebuild: CircuitRebuildContext::new(circuit),
        config,
        device_context: None,
        synthesis_cache: &mut synthesis_cache,
        incremental: None,
    };
    pass.run_with_stats()
        .map(|(outcome, stats)| (resolve_transform_for_test(outcome, circuit), stats))
}

fn resynthesize_two_qubit_blocks_with_device_cache_budget(
    circuit: &Circuit,
    config: TwoQubitBlockResynthesisConfig,
    device_context: DeviceTwoQubitSynthesisContext,
    budget: usize,
) -> Result<(ResolvedTransform, TwoQubitSynthesisCacheStats), CompilerError> {
    let mut synthesis_cache = TwoQubitSynthesisCache::new(budget);
    synthesis_cache.ensure_namespace(&config, Some(&device_context));
    ResynthesisPass {
        source: circuit,
        rebuild: CircuitRebuildContext::new(circuit),
        config,
        device_context: Some(device_context),
        synthesis_cache: &mut synthesis_cache,
        incremental: None,
    }
    .run_with_stats()
    .map(|(outcome, stats)| (resolve_transform_for_test(outcome, circuit), stats))
}

fn cx_config() -> TwoQubitBlockResynthesisConfig {
    config_for_native_2q(StandardGate::CX)
}

fn config_for_native_2q(gate: StandardGate) -> TwoQubitBlockResynthesisConfig {
    TwoQubitBlockResynthesisConfig::normal(
        TwoQubitSynthesisTarget::from_standard_gates(
            vec![
                StandardGate::U,
                StandardGate::H,
                StandardGate::RX,
                StandardGate::RY,
                StandardGate::RZ,
                StandardGate::S,
                StandardGate::SDG,
            ],
            vec![gate],
            true,
        )
        .unwrap(),
    )
}

fn standard_ops(circuit: &Circuit) -> Vec<StandardGate> {
    circuit
        .operations()
        .iter()
        .filter_map(|operation| match operation.instruction {
            Instruction::Standard(gate) => Some(gate),
            _ => None,
        })
        .collect()
}

fn two_qubit_op_count(circuit: &Circuit) -> usize {
    circuit
        .operations()
        .iter()
        .filter(|operation| operation.qubits.len() == 2)
        .count()
}

#[test]
fn cancels_adjacent_cx_pair() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn reports_exact_resynthesis_replacement_provenance() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();

    let (outcome, edits) = ResynthesizeTwoQubitBlocks::new(cx_config())
        .transform_with_rewrite_edits(&circuit)
        .unwrap();
    assert!(matches!(outcome, TransformOutcome::Changed(_)));
    let RewriteEdits::Linear {
        old_len,
        new_len,
        replacements,
    } = edits
    else {
        panic!("expected exact linear edits");
    };
    assert_eq!((old_len, new_len), (2, 0));
    assert_eq!(replacements.len(), 1);
    assert_eq!(replacements[0].old, 0..2);
    assert_eq!(replacements[0].new, 0..0);
}

#[test]
fn single_two_qubit_gate_is_not_resynthesized_without_improvement() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(!result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::CX]);
}

#[test]
fn symbolic_operation_in_block_window_is_preserved() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.rz(q0, Parameter::symbol("theta")).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(!result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::RZ, StandardGate::CX]
    );
}

#[test]
fn labeled_two_qubit_gates_are_boundaries() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit
        .append(
            Instruction::Standard(StandardGate::CX),
            [q0, q1],
            std::iter::empty::<ParameterValue>(),
            Some("keep-a"),
        )
        .unwrap();
    circuit
        .append(
            Instruction::Standard(StandardGate::CX),
            [q0, q1],
            std::iter::empty::<ParameterValue>(),
            Some("keep-b"),
        )
        .unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(!result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::CX, StandardGate::CX]
    );
}

#[test]
fn control_flow_without_profitable_body_block_remains_noop() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| body.cx(q0, q1),
            |_| Ok(()),
        )
        .unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(!result.changed);
    assert_eq!(result.circuit.operations().len(), 1);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::ClassicalControl(_)
    ));
}

#[test]
fn control_flow_body_is_resynthesized_recursively() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.cx(q0, q1)?;
                body.cx(q0, q1)
            },
            |_| Ok(()),
        )
        .unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(result.changed);
    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected if operation");
    };
    assert!(if_op.then_body().operations().is_empty());
}

#[test]
fn recurse_control_flow_false_preserves_profitable_body_block() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.cx(q0, q1)?;
                body.cx(q0, q1)
            },
            |_| Ok(()),
        )
        .unwrap();
    let mut config = cx_config();
    config.recurse_control_flow = false;

    let result = resynthesize_two_qubit_blocks(&circuit, config).unwrap();

    assert!(!result.changed);
}

#[test]
fn barrier_prevents_across_boundary_resynthesis() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.barrier(vec![q0, q1]).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(!result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::CX, StandardGate::CX]
    );
}

#[test]
fn reset_prevents_across_boundary_resynthesis() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.reset(q0).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(!result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::CX, StandardGate::CX]
    );
}

#[test]
fn symbolic_two_qubit_gate_is_skipped() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.rzz(q0, q1, Parameter::symbol("theta")).unwrap();
    circuit.rzz(q0, q1, Parameter::symbol("theta")).unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(!result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::RZZ, StandardGate::RZZ]
    );
}

#[test]
fn disjoint_crossed_operation_is_preserved_while_block_is_removed() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q1).unwrap();
    circuit.h(q2).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::H]);
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn swap_pair_is_resynthesized_when_cost_improves() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.swap(q0, q1).unwrap();
    circuit.swap(q0, q1).unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn interleaved_swap_dependency_is_not_moved_by_left_absorption() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.h(q0).unwrap();
    circuit.rx(q1, 0.17).unwrap();
    circuit.ry(q2, -0.19).unwrap();
    circuit.swap(q0, q2).unwrap();
    circuit.swap(q1, q2).unwrap();
    let config = TwoQubitBlockResynthesisConfig::normal(
        TwoQubitSynthesisTarget::from_standard_gates(
            vec![StandardGate::H, StandardGate::RX, StandardGate::RY],
            vec![StandardGate::RXX, StandardGate::RYY, StandardGate::RZZ],
            true,
        )
        .unwrap(),
    );

    let result = resynthesize_two_qubit_blocks(&circuit, config).unwrap();

    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn adjacent_inverse_pair_resynthesizes_across_supported_backends() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let configs = [
        ("cx", config_for_native_2q(StandardGate::CX)),
        ("cz", config_for_native_2q(StandardGate::CZ)),
        ("rzz", config_for_native_2q(StandardGate::RZZ)),
        ("pauli-fallback", TwoQubitBlockResynthesisConfig::default()),
    ];

    for (name, config) in configs {
        let mut circuit = Circuit::new(2);
        circuit.cx(q0, q1).unwrap();
        circuit.cx(q0, q1).unwrap();

        let result = resynthesize_two_qubit_blocks(&circuit, config).unwrap();

        assert!(result.changed, "target {name} should accept identity block");
        assert_eq!(two_qubit_op_count(&result.circuit), 0);
        assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    }
}

#[test]
fn three_cx_block_is_compressed_and_equivalent() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(result.changed);
    assert!(two_qubit_op_count(&result.circuit) < 3);
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn maximal_run_bypasses_bounded_budgets_and_is_synthesized_once() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.set_global_phase(Parameter::from(0.271));
    for index in 0..42 {
        let angle = index as f64 + 1.0;
        circuit.rx(q0, 0.013 * angle).unwrap();
        if index % 2 == 0 {
            circuit.cx(q0, q1).unwrap();
        } else {
            circuit.cx(q1, q0).unwrap();
        }
        circuit.rz(q1, -0.017 * angle).unwrap();
    }
    let mut config = cx_config();
    config.max_block_ops = 1;
    config.max_crossed_ops = 0;
    config.max_scan_span = 0;

    let (first, first_stats) =
        resynthesize_two_qubit_blocks_with_cache_budget(&circuit, config.clone(), 4096).unwrap();
    let (second, second_stats) =
        resynthesize_two_qubit_blocks_with_cache_budget(&circuit, config, 4096).unwrap();

    assert!(first.changed);
    assert!(two_qubit_op_count(&first.circuit) <= 3);
    assert_compiled_circuit_equivalent(&first.circuit, &circuit);
    assert_eq!(first.circuit, second.circuit);
    assert_eq!(first_stats.generic_lookups, 1);
    assert_eq!(first_stats.generic_misses, 1);
    assert_eq!(first_stats.generic_hits, 0);
    assert_eq!(first_stats, second_stats);
}

#[test]
fn maximal_run_with_equal_cost_is_preserved() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();

    let (result, stats) =
        resynthesize_two_qubit_blocks_with_cache_budget(&circuit, cx_config(), 4096).unwrap();

    assert!(!result.changed);
    assert_eq!(result.circuit, circuit);
    assert_eq!(stats.generic_lookups, 1);
}

#[test]
fn maximal_run_is_applied_inside_control_flow_with_tiny_bounded_budget() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                for index in 0..36 {
                    body.ry(q0, 0.019 * (index as f64 + 1.0))?;
                    if index % 2 == 0 {
                        body.cx(q0, q1)?;
                    } else {
                        body.cx(q1, q0)?;
                    }
                }
                Ok(())
            },
            |_| Ok(()),
        )
        .unwrap();
    let mut config = cx_config();
    config.max_block_ops = 1;
    config.max_crossed_ops = 0;
    config.max_scan_span = 0;

    let result = resynthesize_two_qubit_blocks(&circuit, config).unwrap();

    assert!(result.changed);
    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected if operation");
    };
    assert!(
        if_op
            .then_body()
            .operations()
            .iter()
            .filter(|operation| operation.qubits.len() == 2)
            .count()
            <= 3
    );
}

#[test]
fn incremental_device_resynthesis_recollects_current_maximal_runs() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    for index in 0..40 {
        circuit
            .u(q0, 0.011 * (index as f64 + 1.0), 0.0, 0.0)
            .unwrap();
        circuit.cx(q0, q1).unwrap();
    }
    let device = Device::line("incremental-maximal-run", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let context = DeviceTwoQubitSynthesisContext::build(
        &device,
        &circuit,
        DeviceSynthesisPlacement::ExactPhysical,
    )
    .unwrap();
    let mut config = cx_config();
    config.max_block_ops = 1;
    config.max_crossed_ops = 0;
    config.max_scan_span = 0;
    let (expected, _) = resynthesize_two_qubit_blocks_with_device_cache_budget(
        &circuit,
        config.clone(),
        context.clone(),
        4096,
    )
    .unwrap();
    let mut session = NativeResynthesisSession::new(NativeResynthesisPolicy::Incremental);

    let first = resynthesize_two_qubit_blocks_incremental(
        &circuit,
        config.clone(),
        context.clone(),
        &mut session,
    )
    .map(|outcome| resolve_transform_for_test(outcome, &circuit))
    .unwrap();
    let second =
        resynthesize_two_qubit_blocks_incremental(&first.circuit, config, context, &mut session)
            .map(|outcome| resolve_transform_for_test(outcome, &first.circuit))
            .unwrap();

    assert!(first.changed);
    assert_eq!(first.circuit, expected.circuit);
    assert!(two_qubit_op_count(&first.circuit) <= 3);
    assert!(!second.changed);
    assert!(two_qubit_op_count(&second.circuit) <= 3);
    assert_compiled_circuit_equivalent(&second.circuit, &circuit);
}

#[test]
fn mixed_one_and_two_qubit_block_remains_semantically_equivalent() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.x(q1).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert!(result.changed);
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn numeric_rotation_mixed_block_preserves_semantics() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.rx(q0, 0.3).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.rz(q1, 0.7).unwrap();
    circuit.cz(q0, q1).unwrap();
    circuit.rx(q0, 1.2).unwrap();

    let result = resynthesize_two_qubit_blocks(&circuit, cx_config()).unwrap();

    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
}

#[test]
fn repeated_blocks_hit_pass_local_plan_cache_without_block_fact_overhead() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.barrier(vec![q0, q1]).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();

    let (result, stats) =
        resynthesize_two_qubit_blocks_with_cache_budget(&circuit, cx_config(), 4096).unwrap();

    assert!(result.changed);
    assert!(stats.generic_misses > 0);
    assert!(stats.generic_hits > 0);
    assert_eq!(stats.block_fact_lookups, 0);
    assert_eq!(stats.block_fact_hits, 0);
    assert_eq!(stats.block_fact_misses, 0);
    assert_eq!(stats.block_fact_entries, 0);
    assert_eq!(
        stats.generic_lookups,
        stats.generic_hits + stats.generic_misses
    );
}

#[test]
fn root_and_control_flow_body_share_synthesis_cache() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.cx(q0, q1)?;
                body.cx(q0, q1)
            },
            |_| Ok(()),
        )
        .unwrap();

    let (_, stats) =
        resynthesize_two_qubit_blocks_with_cache_budget(&circuit, cx_config(), 4096).unwrap();

    assert!(stats.generic_hits > 0);
    assert_eq!(stats.generic_entries, 1);
}

#[test]
fn sibling_control_flow_bodies_share_synthesis_cache() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.cx(q0, q1)?;
                body.cx(q0, q1)
            },
            |body| {
                body.cx(q0, q1)?;
                body.cx(q0, q1)
            },
        )
        .unwrap();

    let (_, stats) =
        resynthesize_two_qubit_blocks_with_cache_budget(&circuit, cx_config(), 4096).unwrap();

    assert_eq!(stats.generic_misses, 1);
    assert_eq!(stats.generic_hits, 1);
    assert_eq!(stats.generic_entries, 1);
}

#[test]
fn independent_transforms_do_not_share_synthesis_cache() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();

    let (_, first) =
        resynthesize_two_qubit_blocks_with_cache_budget(&circuit, cx_config(), 4096).unwrap();
    let (_, second) =
        resynthesize_two_qubit_blocks_with_cache_budget(&circuit, cx_config(), 4096).unwrap();

    assert_eq!(first.generic_misses, 1);
    assert_eq!(second.generic_misses, 1);
    assert_eq!(first.generic_hits, second.generic_hits);
}

#[test]
fn cached_and_uncached_resynthesis_are_bit_exact() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    for _ in 0..3 {
        circuit.h(q0).unwrap();
        circuit.cx(q0, q1).unwrap();
        circuit.rz(q1, 0.37).unwrap();
        circuit.cx(q0, q1).unwrap();
        circuit.barrier(vec![q0, q1]).unwrap();
    }

    let configs = [
        ("cx", config_for_native_2q(StandardGate::CX)),
        ("cz", config_for_native_2q(StandardGate::CZ)),
        ("rzz", config_for_native_2q(StandardGate::RZZ)),
        ("pauli-fallback", TwoQubitBlockResynthesisConfig::default()),
    ];
    for (name, config) in configs {
        let (cached, cached_stats) =
            resynthesize_two_qubit_blocks_with_cache_budget(&circuit, config.clone(), 4096)
                .unwrap();
        let (uncached, uncached_stats) =
            resynthesize_two_qubit_blocks_with_cache_budget(&circuit, config, 0).unwrap();

        assert_eq!(cached.changed, uncached.changed, "target={name}");
        assert_eq!(cached.circuit, uncached.circuit, "target={name}");
        assert!(
            cached_stats
                .generic_hits
                .saturating_add(cached_stats.block_fact_hits)
                .saturating_add(cached_stats.terminal_decision_hits)
                > 0,
            "target={name}"
        );
        assert_eq!(uncached_stats.generic_hits, 0, "target={name}");
        assert_eq!(uncached_stats.block_fact_hits, 0, "target={name}");
        assert!(
            uncached_stats.capacity_rejections
                >= uncached_stats
                    .generic_misses
                    .saturating_add(uncached_stats.block_fact_misses),
            "target={name}: {uncached_stats:?}"
        );
        assert_eq!(uncached_stats.terminal_decision_hits, 0, "target={name}");
    }
}

#[test]
fn device_cached_and_uncached_resynthesis_are_bit_exact() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    for _ in 0..3 {
        circuit.cx(q0, q1).unwrap();
        circuit.cx(q0, q1).unwrap();
        circuit.barrier(vec![q0, q1]).unwrap();
    }
    let device = Device::line("resynthesis-device-cache", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();

    for placement in [
        DeviceSynthesisPlacement::ExactPhysical,
        DeviceSynthesisPlacement::PreLayoutEnvelope,
    ] {
        let context = DeviceTwoQubitSynthesisContext::build(&device, &circuit, placement).unwrap();
        let (cached, cached_stats) = resynthesize_two_qubit_blocks_with_device_cache_budget(
            &circuit,
            cx_config(),
            context.clone(),
            4096,
        )
        .unwrap();
        let (uncached, uncached_stats) = resynthesize_two_qubit_blocks_with_device_cache_budget(
            &circuit,
            cx_config(),
            context,
            0,
        )
        .unwrap();

        assert_eq!(cached.changed, uncached.changed, "placement={placement:?}");
        assert_eq!(cached.circuit, uncached.circuit, "placement={placement:?}");
        assert!(cached_stats.device_hits > 0, "placement={placement:?}");
        assert_eq!(uncached_stats.device_hits, 0, "placement={placement:?}");
        assert!(
            uncached_stats.capacity_rejections
                >= uncached_stats
                    .device_misses
                    .saturating_add(uncached_stats.block_fact_misses),
            "placement={placement:?}: {uncached_stats:?}"
        );
        assert_eq!(
            uncached_stats.terminal_decision_hits, 0,
            "placement={placement:?}"
        );
    }
}
