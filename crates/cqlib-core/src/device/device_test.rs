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
use crate::circuit::gate::standard_gate::StandardGate;
use crate::circuit::{
    ClassicalExpr, ClassicalType, MCGate, Operation, ParameterValue, Qubit, UnitaryGate,
    ValueClassicalControlOp, ValueControlBody, ValueInstruction, ValueOperation,
};

#[test]
fn test_instruction_prop() {
    let instr = Instruction::Standard(StandardGate::H);
    let prop = InstructionProp::new(instr.clone(), 0.01).with_length(10.0);

    assert!(matches!(
        prop.instruction(),
        Instruction::Standard(StandardGate::H)
    ));
    assert_eq!(prop.error_rate(), 0.01);
    assert_eq!(prop.length(), Some(10.0));
}

#[test]
fn test_qubit_prop() {
    let instr = Instruction::Standard(StandardGate::H);
    let instr_prop = InstructionProp::new(instr, 0.01).with_length(10.0);

    let prop = QubitProp::new(0.05)
        .with_prob_meas0_prep1(0.06)
        .with_prob_meas1_prep0(0.04)
        .with_t1(50.0)
        .with_t2(30.0)
        .with_frequency(5.0)
        .with_native_instruction(instr_prop)
        .unwrap();

    assert_eq!(prop.readout_error(), 0.05);
    assert_eq!(prop.t1(), Some(50.0));
    assert_eq!(prop.t2(), Some(30.0));
    assert_eq!(prop.frequency(), Some(5.0));
    assert_eq!(prop.native_instructions().len(), 1);
    assert_eq!(prop.native_instructions()[0].error_rate(), 0.01);
}

#[test]
fn test_edge_prop() {
    let instr = Instruction::Standard(StandardGate::CX);
    let instr_prop = InstructionProp::new(instr, 0.02).with_length(200.0);

    let prop = EdgeProp::new().with_native_instruction(instr_prop).unwrap();
    assert_eq!(prop.native_instructions().len(), 1);
    assert_eq!(prop.native_instructions()[0].error_rate(), 0.02);
}

#[test]
fn device_native_gates_accept_only_standard_gates_with_at_most_two_qubits() {
    let mut device = Device::line("native-validation", 2).unwrap();
    device
        .set_native_gates(vec![
            Instruction::Standard(StandardGate::GPhase),
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();

    assert_eq!(device.native_gates().len(), 3);
    assert_eq!(
        device
            .set_native_gates(vec![Instruction::Standard(StandardGate::CCX)])
            .unwrap_err(),
        DeviceError::InvalidNativeInstructionArity {
            instruction: "CCX".to_string(),
            expected: 0..=2,
            actual: 3,
        }
    );
    assert_eq!(
        device
            .set_native_gates(vec![Instruction::Delay])
            .unwrap_err(),
        DeviceError::NonStandardNativeInstruction {
            instruction: "delay".to_string(),
        }
    );
    assert!(matches!(
        Device::line("invalid-builder", 2)
            .unwrap()
            .with_native_gates(vec![Instruction::Delay]),
        Err(DeviceError::NonStandardNativeInstruction { .. })
    ));
}

#[test]
fn set_native_gates_preserves_existing_defaults_when_validation_fails() {
    let mut device = Device::line("atomic-native-validation", 2)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();

    let error = device
        .set_native_gates(vec![
            Instruction::Standard(StandardGate::X),
            Instruction::Delay,
        ])
        .unwrap_err();

    assert!(matches!(
        error,
        DeviceError::NonStandardNativeInstruction { .. }
    ));
    assert!(matches!(
        device.native_gates(),
        [Instruction::Standard(StandardGate::H)]
    ));
}

#[test]
fn qubit_native_instructions_require_single_qubit_standard_gates() {
    let mut prop = QubitProp::new(0.01);
    prop.set_native_instruction(InstructionProp::new(
        Instruction::Standard(StandardGate::H),
        0.001,
    ))
    .unwrap();

    assert!(matches!(
        prop.set_native_instruction(InstructionProp::new(
            Instruction::Standard(StandardGate::GPhase),
            0.0,
        )),
        Err(DeviceError::InvalidNativeInstructionArity {
            expected,
            actual: 0,
            ..
        }) if expected == (1..=1)
    ));
    assert!(matches!(
        QubitProp::new(0.01).with_native_instruction(InstructionProp::new(
            Instruction::Standard(StandardGate::CX),
            0.01,
        )),
        Err(DeviceError::InvalidNativeInstructionArity {
            expected,
            actual: 2,
            ..
        }) if expected == (1..=1)
    ));
    assert!(matches!(
        QubitProp::new(0.01).with_native_instruction(InstructionProp::new(
            Instruction::Standard(StandardGate::CCX),
            0.01,
        )),
        Err(DeviceError::InvalidNativeInstructionArity {
            expected,
            actual: 3,
            ..
        }) if expected == (1..=1)
    ));
    assert!(matches!(
        QubitProp::new(0.01).with_native_instruction(InstructionProp::new(Instruction::Delay, 0.0)),
        Err(DeviceError::NonStandardNativeInstruction { .. })
    ));
    assert_eq!(prop.native_instructions().len(), 1);
}

#[test]
fn edge_native_instructions_require_two_qubit_standard_gates() {
    let mut prop = EdgeProp::new();
    prop.set_native_instruction(InstructionProp::new(
        Instruction::Standard(StandardGate::CX),
        0.01,
    ))
    .unwrap();

    for gate in [StandardGate::GPhase, StandardGate::H, StandardGate::CCX] {
        assert!(matches!(
            prop.set_native_instruction(InstructionProp::new(Instruction::Standard(gate), 0.01,)),
            Err(DeviceError::InvalidNativeInstructionArity {
                expected,
                ..
            }) if expected == (2..=2)
        ));
    }
    assert!(matches!(
        EdgeProp::new().with_native_instruction(InstructionProp::new(Instruction::Delay, 0.0)),
        Err(DeviceError::NonStandardNativeInstruction { .. })
    ));
    assert_eq!(prop.native_instructions().len(), 1);
}

#[test]
fn test_device_creation_and_defaults() {
    let q0 = PhysicalQubit::new(0);
    let q1 = PhysicalQubit::new(1);
    let topo = Topology::new(vec![q0, q1], vec![(q0, q1, "cx".to_string())]);

    let mut device = Device::new(
        "test_device".to_string(),
        HashSet::from_iter([q0, q1]),
        topo.unwrap(),
    )
    .unwrap()
    .with_default_t1(40.0)
    .with_default_t2(20.0)
    .with_default_readout_error(0.03)
    .with_default_single_qubit_error(0.001)
    .with_default_two_qubit_error(0.01);

    assert_eq!(device.name(), "test_device");
    assert_eq!(device.default_t1(), Some(40.0));
    assert_eq!(device.default_t2(), Some(20.0));
    assert_eq!(device.default_readout_error(), Some(0.03));
    assert_eq!(device.default_single_qubit_error(), Some(0.001));
    assert_eq!(device.default_two_qubit_error(), Some(0.01));

    // Fallbacks for q0 (no specific properties set yet)
    assert_eq!(device.get_t1(q0), Some(40.0));
    assert_eq!(device.get_t2(q0), Some(20.0));
    assert_eq!(device.get_readout_error(q0), Some(0.03));

    // Add specific properties for q0
    let q0_prop = QubitProp::new(0.02).with_t1(60.0).with_t2(30.0);
    assert!(device.add_qubit_properties(q0, q0_prop).is_ok());

    assert_eq!(device.get_t1(q0), Some(60.0));
    assert_eq!(device.get_t2(q0), Some(30.0));
    assert_eq!(device.get_readout_error(q0), Some(0.02));

    // Fallbacks still apply to q1
    assert_eq!(device.get_t1(q1), Some(40.0));
}

#[test]
fn line_device_creates_online_qubits_and_directed_topology() {
    let device = Device::line("line", 3).unwrap();

    assert_eq!(device.name(), "line");
    assert_eq!(device.qubits().count(), 3);
    assert_eq!(device.num_usable_qubits(), 3);
    assert!(
        device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(0), PhysicalQubit::new(1))
    );
    assert!(
        device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(1), PhysicalQubit::new(2))
    );
    assert!(
        !device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(1), PhysicalQubit::new(0))
    );
}

#[test]
fn line_from_qubits_preserves_supplied_physical_ids_and_order() {
    let p2 = PhysicalQubit::new(2);
    let p5 = PhysicalQubit::new(5);
    let p9 = PhysicalQubit::new(9);

    let device = Device::line_from_qubits("line", vec![p5, p2, p9]).unwrap();

    assert_eq!(device.qubits().collect::<Vec<_>>(), vec![p2, p5, p9]);
    assert!(device.topology().supports_directed_coupling(p5, p2));
    assert!(device.topology().supports_directed_coupling(p2, p9));
    assert!(!device.topology().supports_directed_coupling(p2, p5));
}

#[test]
fn bidirectional_line_device_adds_both_coupling_directions() {
    let device = Device::bidirectional_line("line", 3).unwrap();

    assert_eq!(device.topology().num_couplings(), 4);
    assert!(
        device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(0), PhysicalQubit::new(1))
    );
    assert!(
        device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(1), PhysicalQubit::new(0))
    );
}

#[test]
fn ring_device_adds_unique_bidirectional_cycle_edges() {
    let device = Device::ring("ring", 4).unwrap();

    assert_eq!(device.topology().num_couplings(), 8);
    assert!(
        device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(3), PhysicalQubit::new(0))
    );
    assert!(
        device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(0), PhysicalQubit::new(3))
    );
}

#[test]
fn star_device_connects_center_bidirectionally() {
    let device = Device::star("star", 4, 2).unwrap();

    assert_eq!(device.topology().num_couplings(), 6);
    for leaf in [0, 1, 3] {
        assert!(
            device
                .topology()
                .supports_directed_coupling(PhysicalQubit::new(2), PhysicalQubit::new(leaf))
        );
        assert!(
            device
                .topology()
                .supports_directed_coupling(PhysicalQubit::new(leaf), PhysicalQubit::new(2))
        );
    }
}

#[test]
fn grid_device_uses_row_major_bidirectional_nearest_neighbors() {
    let device = Device::grid("grid", 2, 3).unwrap();

    assert_eq!(device.qubits().collect::<Vec<_>>().len(), 6);
    assert_eq!(device.topology().num_couplings(), 14);
    assert!(
        device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(0), PhysicalQubit::new(1))
    );
    assert!(
        device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(1), PhysicalQubit::new(0))
    );
    assert!(
        device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(1), PhysicalQubit::new(4))
    );
    assert!(
        !device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(0), PhysicalQubit::new(2))
    );
}

#[test]
fn from_edges_creates_explicit_directed_couplings() {
    let device = Device::from_edges("custom", 3, &[(0, 2), (2, 1)]).unwrap();

    assert_eq!(device.topology().num_couplings(), 2);
    assert!(
        device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(0), PhysicalQubit::new(2))
    );
    assert!(
        !device
            .topology()
            .supports_directed_coupling(PhysicalQubit::new(2), PhysicalQubit::new(0))
    );
}

#[test]
fn test_device_errors() {
    let q0 = PhysicalQubit::new(0);
    let q1 = PhysicalQubit::new(1);
    let q2 = PhysicalQubit::new(2); // Not in topology
    let topo = Topology::new(vec![q0, q1], vec![(q0, q1, "0-1".to_string())]);

    let mut device = Device::new(
        "test_device".to_string(),
        HashSet::from_iter([q0, q1]),
        topo.unwrap(),
    )
    .unwrap();

    let prop = QubitProp::new(0.05);
    let err = device.add_qubit_properties(q2, prop);
    assert_eq!(err.unwrap_err(), DeviceError::QubitNotInDevice(q2));

    let edge_prop = EdgeProp::new();
    let err = device.add_edge_properties(q1, q0, edge_prop).unwrap_err();
    assert_eq!(err, DeviceError::EdgeNotInTopology(q1, q0));
}

#[test]
fn qubit_properties_distinguish_device_membership_from_topology_membership() {
    let q0 = PhysicalQubit::new(0);
    let q1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![q0], vec![]).unwrap();
    let mut device = Device::new("partial", HashSet::from([q0, q1]), topology).unwrap();

    assert_eq!(
        device
            .add_qubit_properties(q1, QubitProp::new(0.01))
            .unwrap_err(),
        DeviceError::QubitNotInTopology(q1)
    );
}

#[test]
fn qubit_properties_can_be_recorded_for_offline_qubits() {
    let q0 = PhysicalQubit::new(0);
    let mut device = Device::line("offline-calibration", 1).unwrap();
    device.set_invalid_qubits(HashSet::from([q0])).unwrap();

    device
        .add_qubit_properties(q0, QubitProp::new(0.02).with_t1(50.0))
        .unwrap();

    assert_eq!(device.qubit_properties(q0).unwrap().t1(), Some(50.0));
    assert!(!device.is_usable_qubit(q0));
}

#[test]
fn invalid_qubits_must_be_registered_with_device() {
    let q0 = PhysicalQubit::new(0);
    let q1 = PhysicalQubit::new(1);
    let q2 = PhysicalQubit::new(2);
    let topology = Topology::new(vec![q0, q1], vec![(q0, q1, "CX".to_string())]).unwrap();
    let device = Device::new("test-device", HashSet::from([q0, q1]), topology).unwrap();

    assert_eq!(
        device
            .clone()
            .with_invalid_qubits(HashSet::from([q2]))
            .unwrap_err(),
        DeviceError::QubitNotInDevice(q2)
    );

    let mut device = device.with_invalid_qubits(HashSet::from([q1])).unwrap();
    assert_eq!(
        device.set_invalid_qubits(HashSet::from([q2])).unwrap_err(),
        DeviceError::QubitNotInDevice(q2)
    );
    assert_eq!(device.invalid_qubits().collect::<Vec<_>>(), vec![q1]);
}

#[test]
fn usable_qubits_exclude_invalid_qubits_in_stable_order() {
    let q0 = PhysicalQubit::new(0);
    let q1 = PhysicalQubit::new(1);
    let q2 = PhysicalQubit::new(2);
    let topology = Topology::new(
        vec![q0, q1, q2],
        vec![(q0, q1, "CX".to_string()), (q1, q2, "CX".to_string())],
    )
    .unwrap();
    let device = Device::new("test-device", HashSet::from([q2, q0, q1]), topology)
        .unwrap()
        .with_invalid_qubits(HashSet::from([q1]))
        .unwrap();

    assert_eq!(device.qubits().collect::<Vec<_>>(), vec![q0, q1, q2]);
    assert_eq!(device.invalid_qubits().collect::<Vec<_>>(), vec![q1]);
    assert_eq!(device.usable_qubits().collect::<Vec<_>>(), vec![q0, q2]);
    assert!(device.is_usable_qubit(q0));
    assert!(!device.is_usable_qubit(q1));
    assert!(!device.is_usable_qubit(PhysicalQubit::new(99)));
    assert_eq!(device.num_usable_qubits(), 2);
}

#[test]
fn single_qubit_error_uses_native_instruction_then_default() {
    let q0 = PhysicalQubit::new(0);
    let q1 = PhysicalQubit::new(1);
    let q2 = PhysicalQubit::new(2);
    let topology = Topology::new(vec![q0, q1, q2], vec![]).unwrap();
    let mut device = Device::new("test-device", HashSet::from([q0, q1, q2]), topology)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::X),
        ])
        .unwrap()
        .with_default_single_qubit_error(0.01)
        .with_invalid_qubits(HashSet::from([q2]))
        .unwrap();
    device
        .add_qubit_properties(
            q0,
            QubitProp::new(0.02)
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::H),
                    0.001,
                ))
                .unwrap(),
        )
        .unwrap();
    device
        .add_qubit_properties(q1, QubitProp::new(0.03))
        .unwrap();

    assert_eq!(
        device.single_qubit_error(q0, &Instruction::Standard(StandardGate::H)),
        Some(0.001)
    );
    assert_eq!(
        device.single_qubit_error(q0, &Instruction::Standard(StandardGate::X)),
        None
    );
    assert_eq!(
        device.single_qubit_error(q1, &Instruction::Standard(StandardGate::H)),
        Some(0.01)
    );
    assert_eq!(
        device.single_qubit_error(q1, &Instruction::Standard(StandardGate::Y)),
        None
    );
    assert_eq!(
        device.single_qubit_error(q2, &Instruction::Standard(StandardGate::H)),
        None
    );
    assert_eq!(
        device.single_qubit_error(
            PhysicalQubit::new(99),
            &Instruction::Standard(StandardGate::H)
        ),
        None
    );
}

#[test]
fn two_qubit_and_edge_errors_respect_direction_and_usability() {
    let q0 = PhysicalQubit::new(0);
    let q1 = PhysicalQubit::new(1);
    let q2 = PhysicalQubit::new(2);
    let topology = Topology::new(
        vec![q0, q1, q2],
        vec![
            (q0, q1, "CX".to_string()),
            (q1, q0, "CX".to_string()),
            (q1, q2, "CX".to_string()),
        ],
    )
    .unwrap();
    let mut device = Device::new("test-device", HashSet::from([q0, q1, q2]), topology)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::CX),
            Instruction::Standard(StandardGate::SWAP),
        ])
        .unwrap()
        .with_default_two_qubit_error(0.07);
    device
        .add_edge_properties(
            q0,
            q1,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CZ),
                    0.03,
                ))
                .unwrap()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.02,
                ))
                .unwrap(),
        )
        .unwrap();
    device.add_edge_properties(q1, q0, EdgeProp::new()).unwrap();

    assert_eq!(
        device.two_qubit_error(q0, q1, &Instruction::Standard(StandardGate::CX)),
        Some(0.02)
    );
    assert_eq!(
        device.two_qubit_error(q0, q1, &Instruction::Standard(StandardGate::SWAP)),
        None
    );
    assert_eq!(
        device.two_qubit_error(q1, q0, &Instruction::Standard(StandardGate::CX)),
        Some(0.07)
    );
    assert_eq!(
        device.two_qubit_error(q1, q0, &Instruction::Standard(StandardGate::CZ)),
        None
    );
    assert_eq!(
        device.two_qubit_error(q0, q2, &Instruction::Standard(StandardGate::CX)),
        None
    );
    assert_eq!(device.edge_error(q0, q1), Some(0.02));
    assert_eq!(device.edge_error(q1, q0), Some(0.07));

    device.set_invalid_qubits(HashSet::from([q2])).unwrap();
    assert_eq!(
        device.two_qubit_error(q1, q2, &Instruction::Standard(StandardGate::CX)),
        None
    );
    assert_eq!(device.edge_error(q1, q2), None);
}

#[test]
fn local_native_instructions_do_not_synchronize_device_defaults() {
    let q0 = PhysicalQubit::new(0);
    let q1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![q0, q1], vec![(q0, q1, "q0-q1".to_string())]).unwrap();
    let mut device = Device::new("test-device", HashSet::from([q0, q1]), topology)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap()
        .with_default_single_qubit_error(0.01)
        .with_default_two_qubit_error(0.02);

    device
        .add_edge_properties(
            q0,
            q1,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.003,
                ))
                .unwrap(),
        )
        .unwrap();

    assert!(matches!(
        device.native_gates(),
        [Instruction::Standard(StandardGate::H)]
    ));
    assert_eq!(
        device.two_qubit_error(q0, q1, &Instruction::Standard(StandardGate::CX)),
        Some(0.003)
    );

    device
        .set_native_gates(vec![Instruction::Standard(StandardGate::X)])
        .unwrap();

    assert_eq!(
        device.two_qubit_error(q0, q1, &Instruction::Standard(StandardGate::CX)),
        Some(0.003)
    );
    assert_eq!(
        device.single_qubit_error(q0, &Instruction::Standard(StandardGate::H)),
        None
    );
    assert_eq!(
        device.single_qubit_error(q0, &Instruction::Standard(StandardGate::X)),
        Some(0.01)
    );
}

#[test]
fn supports_native_instruction_uses_capabilities_without_calibration() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p1, p0, "p1-p0".to_string())]).unwrap();
    let mut device = Device::new("directed", HashSet::from([p0, p1]), topology)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();

    assert!(device.supports_native_instruction(&Instruction::Standard(StandardGate::H), &[p0]));
    assert!(
        device.supports_native_instruction(&Instruction::Standard(StandardGate::CX), &[p1, p0])
    );
    assert!(
        !device.supports_native_instruction(&Instruction::Standard(StandardGate::CX), &[p0, p1])
    );
    assert!(!device.supports_native_instruction(&Instruction::Standard(StandardGate::GPhase), &[]));
    assert!(
        !device
            .supports_native_instruction(&Instruction::Standard(StandardGate::CCX), &[p0, p1, p0])
    );
    assert!(
        !device.supports_native_instruction(&Instruction::Directive(Directive::Measure), &[p0])
    );
    assert!(!device.supports_native_instruction(&Instruction::Directive(Directive::Reset), &[p1]));
    assert!(!device.supports_native_instruction(&Instruction::Delay, &[p0]));

    device
        .add_edge_properties(
            p1,
            p0,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CZ),
                    0.01,
                ))
                .unwrap(),
        )
        .unwrap();

    assert!(
        !device.supports_native_instruction(&Instruction::Standard(StandardGate::CX), &[p1, p0])
    );
    assert!(
        device.supports_native_instruction(&Instruction::Standard(StandardGate::CZ), &[p1, p0])
    );
}

#[test]
fn gphase_requires_an_explicit_device_default_capability() {
    let mut device = Device::line("gphase", 1).unwrap();
    let gphase = Instruction::Standard(StandardGate::GPhase);

    assert!(!device.supports_native_instruction(&gphase, &[]));
    device.set_native_gates(vec![gphase.clone()]).unwrap();
    assert!(device.supports_native_instruction(&gphase, &[]));
    assert!(!device.supports_native_instruction(&gphase, &[PhysicalQubit::new(0)]));
}

#[test]
fn validate_circuit_reports_missing_directed_coupling() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p1, p0, "p1-p0".to_string())]).unwrap();
    let device = Device::new("directed", HashSet::from([p0, p1]), topology)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let error = device.validate_circuit(&circuit).unwrap_err();

    assert_eq!(
        error,
        DeviceValidationError::MissingDirectedCoupling {
            device: "directed".to_string(),
            instruction: "CX".to_string(),
            control: p0,
            target: p1,
        }
    );
    let message = error.to_string();
    assert!(message.contains("directed"));
    assert!(message.contains("CX"));
    assert!(message.contains("P0"));
    assert!(message.contains("P1"));

    let mut supported = Circuit::new(2);
    supported.cx(Qubit::new(1), Qubit::new(0)).unwrap();
    assert!(device.validate_circuit(&supported).is_ok());
}

#[test]
fn validate_circuit_reports_unsupported_instruction_on_existing_edge() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p0, p1, "p0-p1".to_string())]).unwrap();
    let mut device = Device::new("local-cz", HashSet::from([p0, p1]), topology)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
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

    assert_eq!(
        device.validate_circuit(&circuit).unwrap_err(),
        DeviceValidationError::UnsupportedInstruction {
            device: "local-cz".to_string(),
            instruction: "CX".to_string(),
            qargs: vec![p0, p1],
        }
    );
}

#[test]
fn edge_error_is_calibration_not_a_capability_predicate() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p0, p1, "p0-p1".to_string())]).unwrap();
    let device = Device::new("calibration-only", HashSet::from([p0, p1]), topology)
        .unwrap()
        .with_default_two_qubit_error(0.02);

    assert_eq!(device.edge_error(p0, p1), Some(0.02));
    assert!(
        !device.supports_native_instruction(&Instruction::Standard(StandardGate::CX), &[p0, p1])
    );
    assert_eq!(
        device.two_qubit_error(p0, p1, &Instruction::Standard(StandardGate::CX)),
        None
    );
}

#[test]
fn validate_circuit_rejects_unusable_qubits_and_recurses_control_flow() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p1, p0, "p1-p0".to_string())]).unwrap();
    let device = Device::new("directed", HashSet::from([p0, p1]), topology)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap();
    let mut controlled = Circuit::new(2);
    let measured = controlled.measure(Qubit::new(0)).unwrap();
    controlled
        .if_(measured.expr().to_bool().unwrap(), |body| {
            body.cx(Qubit::new(0), Qubit::new(1))?;
            Ok(())
        })
        .unwrap();

    assert!(matches!(
        device.validate_circuit(&controlled),
        Err(DeviceValidationError::MissingDirectedCoupling { control, target, .. })
            if control == p0 && target == p1
    ));

    let control = &controlled.operations()[1].instruction;
    assert!(matches!(control, Instruction::ClassicalControl(_)));
    assert!(!device.supports_native_instruction(control, &[p0, p1]));

    let offline = device.with_invalid_qubits(HashSet::from([p0])).unwrap();
    let mut measurement = Circuit::new(1);
    measurement.measure(Qubit::new(0)).unwrap();
    assert_eq!(
        offline.validate_circuit(&measurement).unwrap_err(),
        DeviceValidationError::UnusablePhysicalQubit {
            device: "directed".to_string(),
            qubit: p0,
        }
    );
}

#[test]
fn runtime_operations_are_validator_policy_not_atomic_capabilities() {
    let p0 = PhysicalQubit::new(0);
    let topology = Topology::new(vec![p0], vec![]).unwrap();
    let device = Device::new("runtime", HashSet::from([p0]), topology).unwrap();
    let mut circuit = Circuit::new(1);
    circuit.measure(Qubit::new(0)).unwrap();
    circuit.reset(Qubit::new(0)).unwrap();
    circuit
        .delay(Qubit::new(0), ParameterValue::from(20.0))
        .unwrap();

    for operation in circuit.operations() {
        assert!(!device.supports_native_instruction(&operation.instruction, &[p0]));
    }
    assert!(device.validate_circuit(&circuit).is_ok());
}

#[test]
fn validate_circuit_recurses_while_for_and_switch_bodies() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![(p1, p0, "p1-p0".to_string())]).unwrap();
    let device = Device::new("directed", HashSet::from([p0, p1]), topology)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap();

    let mut while_circuit = Circuit::new(2);
    while_circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.cx(Qubit::new(0), Qubit::new(1))
        })
        .unwrap();
    assert!(matches!(
        device.validate_circuit(&while_circuit),
        Err(DeviceValidationError::MissingDirectedCoupling { .. })
    ));

    let mut for_circuit = Circuit::new(2);
    let loop_var = for_circuit.var(ClassicalType::uint(2).unwrap());
    for_circuit
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(2, 0).unwrap(),
            ClassicalExpr::uint_literal(2, 1).unwrap(),
            ClassicalExpr::uint_literal(2, 1).unwrap(),
            |body, _| body.cx(Qubit::new(0), Qubit::new(1)),
        )
        .unwrap();
    assert!(matches!(
        device.validate_circuit(&for_circuit),
        Err(DeviceValidationError::MissingDirectedCoupling { .. })
    ));

    let mut switch_circuit = Circuit::new(2);
    switch_circuit
        .switch(ClassicalExpr::uint_literal(2, 0).unwrap(), |switch| {
            switch.value(0, |body| body.cx(Qubit::new(0), Qubit::new(1)))?;
            switch.default(|_| Ok(()))?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        device.validate_circuit(&switch_circuit),
        Err(DeviceValidationError::MissingDirectedCoupling { .. })
    ));
}

#[test]
fn validate_value_operation_matches_storage_operation_validation() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let forward = Device::new(
        "forward",
        HashSet::from([p0, p1]),
        Topology::new(vec![p0, p1], vec![(p0, p1, "p0-p1".to_string())]).unwrap(),
    )
    .unwrap()
    .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
    .unwrap();
    let reverse = Device::new(
        "reverse",
        HashSet::from([p0, p1]),
        Topology::new(vec![p0, p1], vec![(p1, p0, "p1-p0".to_string())]).unwrap(),
    )
    .unwrap()
    .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
    .unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let value_operation = circuit.index(0).unwrap();

    assert!(forward.validate_operation(&circuit.operations()[0]).is_ok());
    assert!(forward.validate_value_operation(&value_operation).is_ok());
    assert!(matches!(
        reverse.validate_operation(&circuit.operations()[0]),
        Err(DeviceValidationError::MissingDirectedCoupling { .. })
    ));
    assert!(matches!(
        reverse.validate_value_operation(&value_operation),
        Err(DeviceValidationError::MissingDirectedCoupling { .. })
    ));
}

#[test]
fn validate_value_operation_does_not_require_owning_classical_tables() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let mut circuit = Circuit::new(2);
    let measurement = circuit.measure(Qubit::new(0)).unwrap();
    circuit
        .if_(measurement.expr().to_bool().unwrap(), |body| {
            body.cx(Qubit::new(0), Qubit::new(1))
        })
        .unwrap();
    let operation = circuit.index(1).unwrap();

    let forward = Device::new(
        "forward",
        HashSet::from([p0, p1]),
        Topology::new(vec![p0, p1], vec![(p0, p1, "p0-p1".to_string())]).unwrap(),
    )
    .unwrap()
    .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
    .unwrap();
    let reverse = Device::new(
        "reverse",
        HashSet::from([p0, p1]),
        Topology::new(vec![p0, p1], vec![(p1, p0, "p1-p0".to_string())]).unwrap(),
    )
    .unwrap()
    .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
    .unwrap();

    assert!(forward.validate_value_operation(&operation).is_ok());
    assert!(matches!(
        reverse.validate_value_operation(&operation),
        Err(DeviceValidationError::MissingDirectedCoupling { .. })
    ));
}

#[test]
fn validate_value_control_body_only_skips_a_leading_global_phase_marker() {
    let p0 = PhysicalQubit::new(0);
    let device = Device::new(
        "phase-marker",
        HashSet::from([p0]),
        Topology::new(vec![p0], vec![]).unwrap(),
    )
    .unwrap()
    .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
    .unwrap();
    let gphase =
        ValueOperation::from_standard(StandardGate::GPhase, [], [ParameterValue::from(0.25)]);
    let h = ValueOperation::from_standard(StandardGate::H, [Qubit::new(0)], []);
    let make_control = |operations| {
        let control = ValueClassicalControlOp::If {
            condition: ClassicalExpr::bool_literal(true),
            then_body: ValueControlBody::new(operations),
            else_body: None,
        };
        ValueOperation {
            qubits: control.used_qubits().into_iter().collect(),
            instruction: ValueInstruction::ClassicalControl(control),
            params: Default::default(),
            label: None,
        }
    };

    assert!(
        device
            .validate_value_operation(&make_control(vec![gphase.clone(), h.clone()]))
            .is_ok()
    );
    assert!(matches!(
        device.validate_value_operation(&make_control(vec![h, gphase])),
        Err(DeviceValidationError::UnsupportedInstruction { .. })
    ));
}

#[test]
fn validate_circuit_reports_undecomposed_standard_gate() {
    let qubits = [
        PhysicalQubit::new(0),
        PhysicalQubit::new(1),
        PhysicalQubit::new(2),
    ];
    let topology = Topology::new(qubits.to_vec(), vec![]).unwrap();
    let device = Device::new("three", HashSet::from(qubits), topology).unwrap();
    let mut circuit = Circuit::new(3);
    circuit
        .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();

    assert!(matches!(
        device.validate_circuit(&circuit),
        Err(DeviceValidationError::UndecomposedInstruction { instruction, .. })
            if instruction == "CCX"
    ));
}

#[test]
fn validate_operation_rejects_undecomposed_composite_gates() {
    let p0 = PhysicalQubit::new(0);
    let topology = Topology::new(vec![p0], vec![]).unwrap();
    let device = Device::new("one", HashSet::from([p0]), topology).unwrap();
    let mut inner = Circuit::new(1);
    inner.x(Qubit::new(0)).unwrap();
    let circuit_gate = inner.to_gate("inner").unwrap();
    let instructions = [
        Instruction::McGate(Box::new(MCGate::new(1, StandardGate::X))),
        Instruction::UnitaryGate(Box::new(UnitaryGate::new("opaque", 1, 0))),
        circuit_gate,
    ];

    for instruction in instructions {
        let operation = Operation {
            instruction,
            qubits: smallvec::smallvec![Qubit::new(0)],
            params: smallvec::SmallVec::new(),
            label: None,
        };
        assert!(matches!(
            device.validate_operation(&operation),
            Err(DeviceValidationError::UndecomposedInstruction { .. })
        ));
    }
}

#[test]
fn native_instruction_properties_reject_invalid_calibration_values() {
    let invalid_error = InstructionProp::new(Instruction::Standard(StandardGate::H), f64::NAN);
    assert!(matches!(
        QubitProp::new(0.0).with_native_instruction(invalid_error),
        Err(DeviceError::InvalidNativeInstructionErrorRate { .. })
    ));

    let invalid_duration =
        InstructionProp::new(Instruction::Standard(StandardGate::CX), 0.01).with_length(-1.0);
    assert!(matches!(
        EdgeProp::new().with_native_instruction(invalid_duration),
        Err(DeviceError::InvalidNativeInstructionDuration { .. })
    ));

    assert!(
        QubitProp::new(0.0)
            .with_native_instruction(
                InstructionProp::new(Instruction::Standard(StandardGate::H), 1.0).with_length(0.0),
            )
            .is_ok()
    );
}
