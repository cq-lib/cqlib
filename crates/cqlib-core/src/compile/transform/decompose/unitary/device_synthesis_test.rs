// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of the License in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::*;
use crate::circuit::{MCGate, ParameterValue, UnitaryGate};
use crate::compile::device_planning::cost::MetricAvailability;
use crate::compile::transform::decompose::unitary::TwoQubitUnitaryDecomposeBasis;
use crate::compile::transform::decompose::unitary::unitary_2q::{
    plan_numeric_2q_unitary_for_device, select_device_unitary_candidate,
};
use crate::device::{EdgeProp, InstructionProp};

#[test]
fn pre_layout_prefers_broad_family_over_single_calibrated_edge() {
    let p1 = PhysicalQubit::new(1);
    let p2 = PhysicalQubit::new(2);
    let mut device = Device::bidirectional_line("coverage", 4)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
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
                        0.0001,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }
    let matrix = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
    let gate = UnitaryGate::new("SWAP", 2, 0)
        .with_matrix(matrix.clone())
        .unwrap();
    let mut circuit = Circuit::new(4);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(3)])
        .unwrap();
    let context = DeviceTwoQubitSynthesisContext::build(
        &device,
        &circuit,
        DeviceSynthesisPlacement::PreLayoutEnvelope,
    )
    .unwrap();
    let qubits = [Qubit::new(0), Qubit::new(3)];
    let candidates = plan_numeric_2q_unitary_for_device(&matrix, qubits, &context).unwrap();
    let selected = select_device_unitary_candidate(candidates, qubits, &context).unwrap();

    assert_eq!(selected.backend, TwoQubitUnitaryDecomposeBasis::Cx);
    assert!(selected.operations.iter().all(|operation| {
        !matches!(
            operation.instruction,
            ValueInstruction::Instruction(Instruction::Standard(StandardGate::CZ))
        )
    }));
    assert!(selected.operations.iter().all(|operation| {
        operation
            .params
            .iter()
            .all(|param| matches!(param, ParameterValue::Fixed(_)))
    }));
}

#[test]
fn pre_layout_evaluation_derives_all_fields_from_one_pair_scan() {
    let device = Device::bidirectional_line("single-pair-scan", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q1).unwrap();
    let context = DeviceTwoQubitSynthesisContext::build(
        &device,
        &circuit,
        DeviceSynthesisPlacement::PreLayoutEnvelope,
    )
    .unwrap();
    let operations = vec![ValueOperation::from_standard(
        StandardGate::CX,
        [q0, q1],
        [],
    )];

    let evaluation = context.evaluate_pre_layout(&operations, [q0, q1]).unwrap();
    let stats = context.physical_cost_cache_stats();
    assert_eq!(stats.lookups, context.data.eligible_pairs.len());
    assert_eq!(stats.misses, context.data.eligible_pairs.len());
    assert_eq!(stats.hits, 0);

    let expected_pair_costs = context
        .data
        .eligible_pairs
        .iter()
        .filter_map(|pair| {
            context
                .cost_on_pair(&operations, [q0, q1], *pair)
                .map(|cost| (*pair, cost))
        })
        .collect::<BTreeMap<_, _>>();
    let expected_domain = expected_pair_costs
        .keys()
        .copied()
        .collect::<OrderedPairDomain>();
    let expected_worst = expected_pair_costs
        .values()
        .copied()
        .max_by(|left, right| left.compare(*right))
        .unwrap();
    assert_eq!(evaluation.domain, expected_domain);
    assert_eq!(evaluation.coverage, context.coverage_key(&expected_domain));
    assert_eq!(evaluation.worst_cost, expected_worst);
    assert_eq!(
        evaluation.worst_cost_on_domain(&expected_domain),
        Some(expected_worst)
    );
}

#[test]
fn parameter_independent_physical_cost_cache_reuses_distinct_angles() {
    let device = Device::bidirectional_line("parameter-independent-cost", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::RZ),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap()
        .with_default_single_qubit_error(0.001)
        .with_default_two_qubit_error(0.01);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.rz(q0, 0.1).unwrap();
    circuit.cx(q0, q1).unwrap();
    let context = DeviceTwoQubitSynthesisContext::build(
        &device,
        &circuit,
        DeviceSynthesisPlacement::ExactPhysical,
    )
    .unwrap();
    let sequence = |angle| {
        vec![
            ValueOperation::from_standard(StandardGate::RZ, [q0], [ParameterValue::Fixed(angle)]),
            ValueOperation::from_standard(StandardGate::CX, [q0, q1], []),
        ]
    };

    let first = context
        .exact_cost_diagnostic(&sequence(0.1), [q0, q1])
        .unwrap();
    let second = context
        .exact_cost_diagnostic(&sequence(-2.7), [q0, q1])
        .unwrap();
    assert_eq!(first, second);
    let stats = context.physical_cost_cache_stats();
    assert_eq!(stats.lookups, 2);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.entries, 1);
}

#[test]
fn equal_physical_cost_is_not_a_strict_improvement() {
    let cost = DevicePhysicalCost {
        native_two_qubit_ops: 3,
        native_two_qubit_depth: 3,
        error: MetricAvailability::Disabled,
        total_native_depth: 7,
        native_total_ops: 11,
        duration: MetricAvailability::Disabled,
        makespan: MetricAvailability::Disabled,
    };

    assert!(!cost.strictly_better_than(cost));
}

#[test]
fn exact_sequence_cost_supports_one_qubit_only_circuits() {
    let device = Device::line("one-qubit-sequence", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::U)])
        .unwrap()
        .with_default_single_qubit_error(0.001);
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.u(q0, 0.3, -0.2, 0.7).unwrap();
    let context = DeviceTwoQubitSynthesisContext::build(
        &device,
        &circuit,
        DeviceSynthesisPlacement::ExactPhysical,
    )
    .unwrap();
    let operations = vec![ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::U)),
        qubits: smallvec![q0],
        params: smallvec![
            ParameterValue::Fixed(0.3),
            ParameterValue::Fixed(-0.2),
            ParameterValue::Fixed(0.7),
        ],
        label: None,
    }];

    let cost = context.exact_sequence_cost(&operations).unwrap();

    assert_eq!(cost.native_two_qubit_ops, 0);
    assert_eq!(cost.native_total_ops, 1);
    assert_eq!(cost.total_native_depth, 1);
}

#[test]
fn exact_sequence_cost_distinguishes_unprepared_and_unsupported() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let device = Device::line("diagnostic", 2)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::FSIM)])
        .unwrap();
    let empty = Circuit::new(2);
    let context = DeviceTwoQubitSynthesisContext::build(
        &device,
        &empty,
        DeviceSynthesisPlacement::ExactPhysical,
    )
    .unwrap();
    let fsim = ValueOperation::from_standard(
        StandardGate::FSIM,
        [q0, q1],
        [ParameterValue::Fixed(0.2), ParameterValue::Fixed(-0.3)],
    );
    assert!(matches!(
        context.exact_sequence_cost_diagnostic(&[fsim]),
        Err(DeviceContextCostFailure::Unprepared(_))
    ));

    let unsupported_device = Device::line("unsupported", 2).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    let unsupported_context = DeviceTwoQubitSynthesisContext::build(
        &unsupported_device,
        &circuit,
        DeviceSynthesisPlacement::ExactPhysical,
    )
    .unwrap();
    let cx = ValueOperation::from_standard(StandardGate::CX, [q0, q1], []);
    let error = unsupported_context
        .exact_sequence_cost_diagnostic(&[cx])
        .unwrap_err();
    let DeviceContextCostFailure::Unsupported(failure) = error else {
        panic!("prepared but infeasible CX must report Unsupported, got {error:?}");
    };
    assert!(matches!(
        failure.instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    assert_eq!(
        failure.qargs,
        vec![PhysicalQubit::new(0), PhysicalQubit::new(1)]
    );
}

#[test]
fn exact_sequence_cost_reports_wrong_placement_and_invalid_operations() {
    let device = Device::line("diagnostic-shape", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::U)])
        .unwrap();
    let circuit = Circuit::new(1);
    let pre_layout = DeviceTwoQubitSynthesisContext::build(
        &device,
        &circuit,
        DeviceSynthesisPlacement::PreLayoutEnvelope,
    )
    .unwrap();
    assert!(matches!(
        pre_layout.exact_sequence_cost_diagnostic(&[]),
        Err(DeviceContextCostFailure::WrongPlacement)
    ));

    let exact = DeviceTwoQubitSynthesisContext::build(
        &device,
        &circuit,
        DeviceSynthesisPlacement::ExactPhysical,
    )
    .unwrap();
    let measurement = ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Directive(
            crate::circuit::Directive::Measure,
        )),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![],
        label: None,
    };
    assert!(matches!(
        exact.exact_sequence_cost_diagnostic(&[measurement]),
        Err(DeviceContextCostFailure::InvalidOperation(_))
    ));
}

#[test]
fn exact_context_prepares_mc_gate_source_root() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let instruction = Instruction::McGate(Box::new(MCGate::new(1, StandardGate::X)));
    let mut circuit = Circuit::new(2);
    circuit
        .append(
            instruction.clone(),
            [q0, q1],
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();
    let device = Device::line("mc-root", 2).unwrap();
    let context = DeviceTwoQubitSynthesisContext::build(
        &device,
        &circuit,
        DeviceSynthesisPlacement::ExactPhysical,
    )
    .unwrap();
    let operation = ValueOperation {
        instruction: ValueInstruction::from_instruction(instruction),
        qubits: smallvec![q0, q1],
        params: smallvec![],
        label: None,
    };

    match context.exact_sequence_cost_diagnostic(&[operation]) {
        Ok(_) | Err(DeviceContextCostFailure::Unsupported(_)) => {}
        other => panic!("exact McGate source root was not prepared: {other:?}"),
    }
}
