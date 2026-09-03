// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::{DeviceLowerer, fusion_candidate_is_admissible, has_fusible_one_qubit_run};
use crate::circuit::{
    Circuit, ClassicalControlOp, ClassicalExpr, ClassicalType, Instruction, Parameter,
    ParameterValue, Qubit, StandardGate,
};
use crate::compile::CompilerError;
use crate::compile::device_planning::DevicePhysicalCost;
use crate::compile::device_planning::cost::{
    MetricAvailability, RobustDurationKey, RobustErrorKey,
};
use crate::compile::test_utils::{assert_compiled_circuit_equivalent, standard_ops};
use crate::compile::transform::TransformerTestExt;
use crate::device::{Device, EdgeProp, InstructionProp, PhysicalQubit, QubitProp};
use std::collections::HashMap;
use std::f64::consts::PI;

#[test]
fn exact_native_circuit_is_unchanged() {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let device =
        Device::line_from_qubits("native", vec![PhysicalQubit::new(0), PhysicalQubit::new(1)])
            .unwrap()
            .with_native_gates(vec![
                Instruction::Standard(StandardGate::H),
                Instruction::Standard(StandardGate::CX),
            ])
            .unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    assert!(!result.changed);
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn lowerer_prefers_lower_error_reverse_cx_realization() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let mut device = Device::bidirectional_line("calibrated-direction", 2).unwrap();
    for physical in [p0, p1] {
        device
            .add_qubit_properties(
                physical,
                QubitProp::new(0.0)
                    .with_native_instruction(InstructionProp::new(
                        Instruction::Standard(StandardGate::H),
                        0.001,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }
    for (control, target, error) in [(p0, p1, 0.20), (p1, p0, 0.01)] {
        device
            .add_edge_properties(
                control,
                target,
                EdgeProp::new()
                    .with_native_instruction(InstructionProp::new(
                        Instruction::Standard(StandardGate::CX),
                        error,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    assert!(result.changed);
    let gates = standard_ops(&result.circuit);
    assert_eq!(
        gates,
        vec![
            StandardGate::H,
            StandardGate::H,
            StandardGate::CX,
            StandardGate::H,
            StandardGate::H,
        ]
    );
    assert_eq!(
        result.circuit.operations()[2].qubits.as_slice(),
        &[Qubit::new(1), Qubit::new(0)]
    );
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn reverse_native_swap_uses_symmetric_direction_template() {
    let mut circuit = Circuit::new(2);
    circuit.swap(Qubit::new(0), Qubit::new(1)).unwrap();
    let device = Device::line_from_qubits(
        "reverse-swap",
        vec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
    )
    .unwrap()
    .with_native_gates(vec![Instruction::Standard(StandardGate::SWAP)])
    .unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    assert!(result.changed);
    assert_eq!(
        result.circuit.operations()[0].qubits.as_slice(),
        &[Qubit::new(1), Qubit::new(0)]
    );
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn missing_reverse_cx_support_returns_structured_failure() {
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let device = Device::line_from_qubits(
        "reverse-cx-without-h",
        vec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
    )
    .unwrap()
    .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
    .unwrap();

    let error = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap_err();
    let CompilerError::DeviceLoweringFailed(failure) = error else {
        panic!("expected device lowering failure");
    };
    assert_eq!(
        failure.qargs,
        vec![PhysicalQubit::new(0), PhysicalQubit::new(1)]
    );
    assert!(
        failure
            .attempted_candidates
            .iter()
            .any(|candidate| candidate.template == "direction_reverse_CX")
    );
}

#[test]
fn reverse_cx_direction_template_preserves_unitary() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut source = Circuit::new(2);
    source.cx(q0, q1).unwrap();
    let device = Device::line_from_qubits(
        "reverse-cx-equivalence",
        vec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
    )
    .unwrap()
    .with_native_gates(vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::CX),
    ])
    .unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&source, None)
        .unwrap();

    assert_eq!(
        standard_ops(&result.circuit),
        vec![
            StandardGate::H,
            StandardGate::H,
            StandardGate::CX,
            StandardGate::H,
            StandardGate::H,
        ]
    );
    assert_eq!(result.circuit.operations()[2].qubits.as_slice(), &[q1, q0]);
    assert_compiled_circuit_equivalent(&result.circuit, &source);
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn reverse_rzx_direction_template_preserves_symbolic_unitary() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let theta = Parameter::symbol("theta");
    let mut source = Circuit::new(2);
    source.rzx(q0, q1, theta).unwrap();
    let device = Device::line_from_qubits(
        "reverse-rzx-equivalence",
        vec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
    )
    .unwrap()
    .with_native_gates(vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::RZX),
    ])
    .unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&source, None)
        .unwrap();

    assert_eq!(
        standard_ops(&result.circuit),
        vec![
            StandardGate::H,
            StandardGate::H,
            StandardGate::RZX,
            StandardGate::H,
            StandardGate::H,
        ]
    );
    assert_eq!(result.circuit.operations()[2].qubits.as_slice(), &[q1, q0]);
    assert!(result.circuit.symbols().contains("theta"));
    for value in [0.0, 0.37, -PI / 2.0, PI] {
        let bindings = Some(HashMap::from([("theta", value)]));
        let bound_source = source.assign_parameters(&bindings).unwrap();
        let bound_result = result.circuit.assign_parameters(&bindings).unwrap();
        assert_compiled_circuit_equivalent(&bound_result, &bound_source);
    }
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn symmetric_direction_templates_preserve_unitary() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let cases = [
        (StandardGate::CZ, vec![]),
        (StandardGate::SWAP, vec![]),
        (StandardGate::RXX, vec![ParameterValue::Fixed(0.37)]),
        (StandardGate::RYY, vec![ParameterValue::Fixed(-0.41)]),
        (StandardGate::RZZ, vec![ParameterValue::Fixed(0.23)]),
        (
            StandardGate::FSIM,
            vec![ParameterValue::Fixed(0.31), ParameterValue::Fixed(-0.27)],
        ),
    ];

    for (gate, params) in cases {
        let mut source = Circuit::new(2);
        source
            .append(Instruction::Standard(gate), [q0, q1], params, None)
            .unwrap();
        let device = Device::line_from_qubits(
            format!("reverse-{gate:?}-equivalence"),
            vec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
        )
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(gate)])
        .unwrap();

        let result = DeviceLowerer::new(&device)
            .transform_resolved(&source, None)
            .unwrap();

        assert_eq!(standard_ops(&result.circuit), vec![gate], "gate={gate:?}");
        assert_eq!(
            result.circuit.operations()[0].qubits.as_slice(),
            &[q1, q0],
            "gate={gate:?}"
        );
        assert_compiled_circuit_equivalent(&result.circuit, &source);
        device.validate_circuit(&result.circuit).unwrap();
    }
}

#[test]
fn top_level_gphase_is_folded_without_a_device_capability() {
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            Instruction::Standard(StandardGate::GPhase),
            std::iter::empty::<Qubit>(),
            [ParameterValue::Fixed(0.25)],
            None,
        )
        .unwrap();
    circuit.h(Qubit::new(0)).unwrap();
    let device = Device::line("phase", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    assert_eq!(result.circuit.operations().len(), 1);
    assert!((result.circuit.global_phase().evaluate(&None).unwrap() - 0.25).abs() < 1e-12);
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn body_local_gphase_remains_a_semantic_marker_accepted_by_verifier() {
    let mut circuit = Circuit::new(1);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.append(
                Instruction::Standard(StandardGate::GPhase),
                std::iter::empty::<Qubit>(),
                [ParameterValue::Fixed(0.375)],
                None,
            )?;
            body.h(Qubit::new(0))?;
            Ok(())
        })
        .unwrap();
    let device = Device::line("body-phase", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    let Instruction::ClassicalControl(ClassicalControlOp::If(control)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected if control");
    };
    assert!(matches!(
        control.then_body().operations()[0].instruction,
        Instruction::Standard(StandardGate::GPhase)
    ));
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn selected_rule_binds_symbolic_parameters_when_emitted() {
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(2);
    circuit
        .crz(Qubit::new(0), Qubit::new(1), theta.clone())
        .unwrap();
    let device = Device::line("parameter-rule", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::RZ),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    assert!(result.changed);
    assert!(result.circuit.symbols().contains("theta"));
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn while_for_and_switch_bodies_are_lowered_recursively() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| body.cx(q0, q1))
        .unwrap();
    let loop_var = circuit.var(ClassicalType::uint(2).unwrap());
    circuit
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(2, 0).unwrap(),
            ClassicalExpr::uint_literal(2, 1).unwrap(),
            ClassicalExpr::uint_literal(2, 1).unwrap(),
            |body, _| body.cx(q0, q1),
        )
        .unwrap();
    circuit
        .switch(ClassicalExpr::uint_literal(2, 0).unwrap(), |switch| {
            switch.value(0, |body| body.cx(q0, q1))?;
            switch.default(|body| body.cx(q0, q1))?;
            Ok(())
        })
        .unwrap();
    let device = Device::line_from_qubits(
        "control-flow",
        vec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
    )
    .unwrap()
    .with_native_gates(vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::CX),
    ])
    .unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    assert!(result.changed);
    device.validate_circuit(&result.circuit).unwrap();
}

fn qcis_rz_x2p_cz_device(name: &str, qubits: u32) -> Device {
    Device::bidirectional_line(name, qubits)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::RZ),
            Instruction::Standard(StandardGate::X2P),
            Instruction::Standard(StandardGate::CZ),
        ])
        .unwrap()
}

#[test]
fn fused_lowering_eliminates_h_pairs_between_same_target_cx() {
    let device = qcis_rz_x2p_cz_device("fused-cx-chain", 3);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q2, q1).unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    // Each CX lowers to (RZ X2P RZ) CZ (RZ X2P RZ); the H pair between the two
    // CZ gates on the shared target fuses to identity and disappears.
    let gates = standard_ops(&result.circuit);
    assert_eq!(
        gates,
        vec![
            StandardGate::RZ,
            StandardGate::X2P,
            StandardGate::RZ,
            StandardGate::CZ,
            StandardGate::CZ,
            StandardGate::RZ,
            StandardGate::X2P,
            StandardGate::RZ,
        ]
    );
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn fused_lowering_merges_rz_pair_with_exact_phase() {
    let device = qcis_rz_x2p_cz_device("fused-rz", 1);
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.t(q0).unwrap();
    circuit.t(q0).unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    // T + T lowers to RZ(π/4) RZ(π/4) plus two GPhase(π/8); the RZ pair fuses
    // into one RZ(π/2) while the phase folds into the circuit global phase.
    let gates = standard_ops(&result.circuit);
    assert_eq!(gates, vec![StandardGate::RZ]);
    let angle = result
        .circuit
        .resolve_parameter(&result.circuit.operations()[0].params[0])
        .unwrap()
        .evaluate(&None)
        .unwrap();
    assert!((angle - PI / 2.0).abs() < 1e-9, "angle={angle}");
    let phase = result.circuit.global_phase().evaluate(&None).unwrap();
    assert!((phase - PI / 4.0).abs() < 1e-9, "phase={phase}");
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn fused_lowering_barrier_blocks_run_fusion() {
    let device = qcis_rz_x2p_cz_device("fused-barrier", 1);
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.t(q0).unwrap();
    circuit.barrier(vec![q0]).unwrap();
    circuit.t(q0).unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    let gates = standard_ops(&result.circuit);
    assert_eq!(gates, vec![StandardGate::RZ, StandardGate::RZ]);
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn fused_lowering_delays_emission_across_unrelated_two_qubit_gate() {
    let device = qcis_rz_x2p_cz_device("fused-delayed", 3);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.t(q0).unwrap();
    circuit.cx(q1, q2).unwrap();
    circuit.t(q0).unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    // The CX on disjoint qubits does not flush q0, so T + T still merges into
    // one RZ even though a lowered CX was emitted in between.
    let gates = standard_ops(&result.circuit);
    assert_eq!(
        gates
            .iter()
            .filter(|gate| **gate == StandardGate::RZ)
            .count(),
        1 + 4, // one merged RZ on q0 plus the two RZ pairs of the CX form
        "{gates:?}"
    );
    assert!(!gates.contains(&StandardGate::X2P) || gates.len() > 1);
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn fused_lowering_falls_back_when_no_one_qubit_synthesis_path() {
    let device = Device::bidirectional_line("no-one-qubit-synthesis", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::X2P)])
        .unwrap();
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.x(q0).unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    // X lowers to X2P X2P; the fused run is an X rotation but the device has
    // no RZ/U synthesis path, so the original leaves are emitted unchanged.
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::X2P, StandardGate::X2P]
    );
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn fused_lowering_is_deterministic() {
    let device = qcis_rz_x2p_cz_device("fused-determinism", 3);
    let build = || {
        let q0 = Qubit::new(0);
        let q1 = Qubit::new(1);
        let q2 = Qubit::new(2);
        let mut circuit = Circuit::new(3);
        circuit.cx(q0, q1).unwrap();
        circuit.cx(q2, q1).unwrap();
        circuit.t(q2).unwrap();
        circuit.cx(q0, q1).unwrap();
        circuit
    };

    let first = DeviceLowerer::new(&device)
        .transform_resolved(&build(), None)
        .unwrap();
    let second = DeviceLowerer::new(&device)
        .transform_resolved(&build(), None)
        .unwrap();

    assert_eq!(first.circuit, second.circuit);
}

#[test]
fn fused_lowering_cancels_x_pair_through_x2p_leaves_with_exact_phase() {
    let device = Device::bidirectional_line("fused-x-pair", 1)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::RZ),
            Instruction::Standard(StandardGate::X2P),
            Instruction::Standard(StandardGate::X),
        ])
        .unwrap();
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.x(q0).unwrap();
    circuit.x(q0).unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    // Each X lowers to X2P X2P + GPhase(π/2). The four X2P leaves merge into
    // two X (each contributing -π/2) and the X pair then cancels, leaving an
    // empty circuit with zero net phase.
    assert!(result.circuit.operations().is_empty());
    let phase = result.circuit.global_phase().evaluate(&None).unwrap();
    assert!(phase.abs() < 1e-9, "phase={phase}");
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    device.validate_circuit(&result.circuit).unwrap();
}

fn calibrated_one_qubit_device(
    name: &str,
    gates: &[(StandardGate, f64)],
    two_qubit: &[(StandardGate, f64)],
) -> Device {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let mut device = Device::bidirectional_line(name, 2).unwrap();
    for physical in [p0, p1] {
        let mut prop = QubitProp::new(0.0);
        for (gate, error) in gates {
            prop = prop
                .with_native_instruction(InstructionProp::new(Instruction::Standard(*gate), *error))
                .unwrap();
        }
        device.add_qubit_properties(physical, prop).unwrap();
    }
    for (gate, error) in two_qubit {
        for (control, target) in [(p0, p1), (p1, p0)] {
            device
                .add_edge_properties(
                    control,
                    target,
                    EdgeProp::new()
                        .with_native_instruction(InstructionProp::new(
                            Instruction::Standard(*gate),
                            *error,
                        ))
                        .unwrap(),
                )
                .unwrap();
        }
    }
    device
}

#[test]
fn fast_path_does_not_block_peephole_fusion_on_fully_native_device() {
    let device = calibrated_one_qubit_device(
        "fully-native-fusion",
        &[
            (StandardGate::RZ, 0.001),
            (StandardGate::X2P, 0.001),
            (StandardGate::X, 0.001),
            (StandardGate::U, 0.001),
        ],
        &[(StandardGate::CZ, 0.01)],
    );
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rz(q0, 0.25).unwrap();
    circuit.rz(q0, 0.5).unwrap();
    circuit.x(q0).unwrap();
    circuit.x(q0).unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    // Every gate is already native, but RZ + RZ still merges and X + X still
    // cancels: the already-native fast path must not bypass fused emission.
    let gates = standard_ops(&result.circuit);
    assert_eq!(gates, vec![StandardGate::RZ]);
    let angle = result
        .circuit
        .resolve_parameter(&result.circuit.operations()[0].params[0])
        .unwrap()
        .evaluate(&None)
        .unwrap();
    assert!((angle - 0.75).abs() < 1e-9, "angle={angle}");
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn fast_path_does_not_block_general_native_run_synthesis() {
    let device = calibrated_one_qubit_device(
        "fully-native-general-fusion",
        &[(StandardGate::H, 0.001)],
        &[],
    );
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();
    circuit.h(q0).unwrap();

    assert!(has_fusible_one_qubit_run(&circuit));
    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn fast_path_scan_keeps_control_flow_branches_independent() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| body.h(q0),
            |body| body.h(q0),
        )
        .unwrap();

    // A gate at the end of `then` and one at the start of `else` are mutually
    // exclusive, not one two-gate run.
    assert!(!has_fusible_one_qubit_run(&circuit));

    let mut body_run = Circuit::new(1);
    body_run
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.h(q0)?;
            body.h(q0)
        })
        .unwrap();
    assert!(has_fusible_one_qubit_run(&body_run));

    let mut switch = Circuit::new(1);
    switch
        .switch(ClassicalExpr::uint_literal(2, 0).unwrap(), |cases| {
            cases.value(0, |body| body.h(q0))?;
            cases.default(|body| body.h(q0))?;
            Ok(())
        })
        .unwrap();
    assert!(!has_fusible_one_qubit_run(&switch));
}

#[test]
fn general_native_run_fuses_inside_control_flow_body() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.h(q0)?;
            body.h(q0)
        })
        .unwrap();
    let device = calibrated_one_qubit_device(
        "native-body-general-fusion",
        &[(StandardGate::H, 0.001)],
        &[],
    );

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::If(control)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected if control");
    };

    assert!(result.changed);
    assert!(control.then_body().operations().is_empty());
    device.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn fast_path_scan_matches_run_boundaries() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    let mut across_disjoint = Circuit::new(2);
    across_disjoint.h(q0).unwrap();
    across_disjoint.h(q1).unwrap();
    across_disjoint.h(q0).unwrap();
    assert!(has_fusible_one_qubit_run(&across_disjoint));

    let mut delay_boundary = Circuit::new(1);
    delay_boundary.h(q0).unwrap();
    delay_boundary
        .delay(q0, ParameterValue::Fixed(1.0))
        .unwrap();
    delay_boundary.h(q0).unwrap();
    assert!(!has_fusible_one_qubit_run(&delay_boundary));

    let mut global_barrier = Circuit::new(1);
    global_barrier.h(q0).unwrap();
    global_barrier.barrier(Vec::<Qubit>::new()).unwrap();
    global_barrier.h(q0).unwrap();
    assert!(!has_fusible_one_qubit_run(&global_barrier));
}

fn physical_cost(error: f64, total_depth: u32, total_ops: u32) -> DevicePhysicalCost {
    DevicePhysicalCost {
        native_two_qubit_ops: 0,
        native_two_qubit_depth: 0,
        error: MetricAvailability::Available(RobustErrorKey {
            unavailable_count: 0,
            imputed_count: 0,
            log_error: error,
        }),
        total_native_depth: total_depth,
        native_total_ops: total_ops,
        duration: MetricAvailability::<RobustDurationKey>::Disabled,
        makespan: MetricAvailability::<f64>::Disabled,
    }
}

#[test]
fn fused_candidate_requires_both_fewer_leaves_and_better_physical_cost() {
    let run = physical_cost(0.5, 2, 2);
    let lower_error_but_longer = physical_cost(0.1, 3, 3);
    assert!(lower_error_but_longer.strictly_better_than(run));
    assert!(!fusion_candidate_is_admissible(
        3,
        lower_error_but_longer,
        2,
        run,
    ));

    let shorter_but_worse = physical_cost(0.8, 1, 1);
    assert!(!fusion_candidate_is_admissible(
        1,
        shorter_but_worse,
        2,
        run,
    ));

    let shorter_and_better = physical_cost(0.1, 1, 1);
    assert!(fusion_candidate_is_admissible(
        1,
        shorter_and_better,
        2,
        run,
    ));
}

#[test]
fn peephole_x_merge_respects_planner_cost_choice() {
    // A noisy native X must not replace the cheaper X2P pair the planner
    // already selected as the best realization of X.
    let noisy_x = calibrated_one_qubit_device(
        "noisy-x",
        &[(StandardGate::X2P, 0.001), (StandardGate::X, 0.5)],
        &[(StandardGate::CZ, 0.01)],
    );
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.x(q0).unwrap();

    let result = DeviceLowerer::new(&noisy_x)
        .transform_resolved(&circuit, None)
        .unwrap();
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::X2P, StandardGate::X2P]
    );
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    noisy_x.validate_circuit(&result.circuit).unwrap();

    // A high-fidelity native X is used directly.
    let good_x = calibrated_one_qubit_device(
        "good-x",
        &[(StandardGate::X2P, 0.001), (StandardGate::X, 0.0001)],
        &[(StandardGate::CZ, 0.01)],
    );
    let result = DeviceLowerer::new(&good_x)
        .transform_resolved(&circuit, None)
        .unwrap();
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::X]);
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    good_x.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn fused_run_choice_respects_calibration_error() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();

    // H lowers to RZ X2P RZ (three low-error leaves). A fused single U is
    // shorter, but with a very noisy native U it must not win.
    let noisy_u = calibrated_one_qubit_device(
        "noisy-u",
        &[
            (StandardGate::RZ, 0.001),
            (StandardGate::X2P, 0.001),
            (StandardGate::U, 0.5),
        ],
        &[(StandardGate::CZ, 0.01)],
    );
    let result = DeviceLowerer::new(&noisy_u)
        .transform_resolved(&circuit, None)
        .unwrap();
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::RZ, StandardGate::X2P, StandardGate::RZ]
    );
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    noisy_u.validate_circuit(&result.circuit).unwrap();

    // With a high-fidelity native U the fused form wins.
    let good_u = calibrated_one_qubit_device(
        "good-u",
        &[
            (StandardGate::RZ, 0.001),
            (StandardGate::X2P, 0.001),
            (StandardGate::U, 0.0001),
        ],
        &[(StandardGate::CZ, 0.01)],
    );
    let result = DeviceLowerer::new(&good_u)
        .transform_resolved(&circuit, None)
        .unwrap();
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::U]);
    assert_compiled_circuit_equivalent(&result.circuit, &circuit);
    good_u.validate_circuit(&result.circuit).unwrap();
}

#[test]
fn lowering_is_idempotent_on_qcis_native_output() {
    let device = qcis_rz_x2p_cz_device("lowering-idempotent", 2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();

    let first = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();
    assert!(first.changed);
    let second = DeviceLowerer::new(&device)
        .transform_resolved(&first.circuit, None)
        .unwrap();

    assert!(!second.changed);
    assert_eq!(second.circuit, first.circuit);
    device.validate_circuit(&second.circuit).unwrap();
}

#[test]
fn lowering_wide_sparse_circuit_only_plans_used_qubits() {
    let device = qcis_rz_x2p_cz_device("wide-sparse", 20);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(20);
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = DeviceLowerer::new(&device)
        .transform_resolved(&circuit, None)
        .unwrap();

    // Planning is bounded to the used qubits, so a wide but sparse circuit
    // lowers quickly; the output still validates on the full device.
    assert!(result.changed);
    assert!(result.circuit.operations().len() > circuit.operations().len());
    device.validate_circuit(&result.circuit).unwrap();
}
