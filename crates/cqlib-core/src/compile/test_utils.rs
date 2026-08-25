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

//! Shared helpers for compiler tests.

use crate::circuit::test_utils::assert_circuits_equivalent_up_to_global_phase;
use crate::circuit::{Circuit, Instruction, ParameterValue, Qubit, StandardGate};
use crate::compile::CompilerError;
use crate::compile::device_planning::DevicePlanningSession;
use crate::compile::transform::decompose::unitary::{
    DeviceSynthesisPlacement, DeviceTwoQubitSynthesisContext,
};
use crate::device::{Device, PhysicalQubit, Topology};
use proptest::prelude::*;
use std::sync::Arc;

const EPSILON: f64 = 1e-9;

pub(super) fn build_device_synthesis_context(
    device: &Device,
    circuit: &Circuit,
    placement: DeviceSynthesisPlacement,
) -> Result<DeviceTwoQubitSynthesisContext, CompilerError> {
    DeviceTwoQubitSynthesisContext::build_with_session(
        device,
        circuit,
        placement,
        Arc::new(DevicePlanningSession::new(device)),
    )
}

/// Asserts that a compiled circuit preserves a source circuit's unitary.
pub(super) fn assert_compiled_circuit_equivalent(actual: &Circuit, expected: &Circuit) {
    assert_circuits_equivalent_up_to_global_phase(actual, expected, EPSILON);
}

/// Extracts standard-gate operations from a circuit.
pub(super) fn standard_ops(circuit: &Circuit) -> Vec<StandardGate> {
    circuit
        .operations()
        .iter()
        .filter_map(|operation| match operation.instruction {
            Instruction::Standard(gate) => Some(gate),
            _ => None,
        })
        .collect()
}

/// Builds a Bell-state preparation circuit.
pub(super) fn bell_circuit() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit
}

/// Builds an n-qubit GHZ-state preparation circuit.
pub(super) fn ghz_circuit(num_qubits: usize) -> Circuit {
    assert!(num_qubits >= 2);
    let mut circuit = Circuit::new(num_qubits);
    circuit.h(Qubit::new(0)).unwrap();
    for index in 0..num_qubits - 1 {
        circuit
            .cx(Qubit::new(index as u32), Qubit::new(index as u32 + 1))
            .unwrap();
    }
    circuit
}

/// Builds a three-qubit QFT circuit using controlled rotations and a final SWAP.
pub(super) fn qft3_circuit() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.h(q2).unwrap();
    circuit.crz(q1, q2, std::f64::consts::FRAC_PI_2).unwrap();
    circuit.h(q1).unwrap();
    circuit.crz(q0, q2, std::f64::consts::FRAC_PI_4).unwrap();
    circuit.crz(q0, q1, std::f64::consts::FRAC_PI_2).unwrap();
    circuit.h(q0).unwrap();
    circuit.swap(q0, q2).unwrap();
    circuit
}

/// Builds a two-qubit line device with the given native gates.
pub(super) fn two_qubit_device(native_gates: Vec<Instruction>) -> Device {
    Device::line("test-device", 2)
        .unwrap()
        .with_native_gates(native_gates)
        .unwrap()
}

/// Asserts every two-qubit operation is supported by a topology edge.
pub(super) fn assert_two_qubit_operations_supported_by_topology(
    circuit: &Circuit,
    topology: &Topology,
) {
    for operation in circuit.operations() {
        if operation.qubits.len() == 2 {
            let first = PhysicalQubit::new(operation.qubits[0].id());
            let second = PhysicalQubit::new(operation.qubits[1].id());
            assert!(
                topology.supports_coupling_either_direction(first, second),
                "operation {operation:?} is not supported by topology"
            );
        }
    }
}

/// Asserts all operations use only the requested standard-gate basis.
pub(super) fn assert_only_standard_gates(circuit: &Circuit, allowed: &[StandardGate]) {
    for operation in circuit.operations() {
        assert!(
            matches!(operation.instruction, Instruction::Standard(gate) if allowed.contains(&gate)),
            "unexpected operation in circuit: {operation:?}"
        );
    }
}

#[derive(Debug, Clone)]
enum GeneratedMatrixGate {
    Single(StandardGate, usize),
    Parametric(StandardGate, usize, f64),
    Two(StandardGate, usize, usize),
    TwoParametric(StandardGate, usize, usize, f64),
}

/// Generates small circuits that are cheap to convert to matrices.
pub(super) fn generated_small_matrix_circuit() -> impl Strategy<Value = Circuit> {
    (0usize..=4).prop_flat_map(|num_qubits| {
        prop::collection::vec(generated_matrix_gate(num_qubits), 0usize..=24).prop_map(
            move |operations| {
                let mut circuit = Circuit::new(num_qubits);
                for operation in operations {
                    append_generated_matrix_gate(&mut circuit, operation);
                }
                circuit
            },
        )
    })
}

pub(super) fn generated_small_routable_circuit() -> impl Strategy<Value = Circuit> {
    prop::collection::vec(generated_line5_long_range_gate(), 1usize..=12).prop_map(|operations| {
        let mut circuit = Circuit::new(5);
        for operation in operations {
            append_generated_matrix_gate(&mut circuit, operation);
        }
        circuit
    })
}

fn generated_matrix_gate(num_qubits: usize) -> BoxedStrategy<GeneratedMatrixGate> {
    if num_qubits == 0 {
        return Just(GeneratedMatrixGate::Single(StandardGate::I, 0)).boxed();
    }

    let single = (
        prop_oneof![
            Just(StandardGate::I),
            Just(StandardGate::H),
            Just(StandardGate::X),
            Just(StandardGate::Y),
            Just(StandardGate::Z),
            Just(StandardGate::S),
            Just(StandardGate::SDG),
            Just(StandardGate::T),
            Just(StandardGate::TDG),
        ],
        0usize..num_qubits,
    )
        .prop_map(|(gate, qubit)| GeneratedMatrixGate::Single(gate, qubit));
    let angle = -std::f64::consts::TAU..std::f64::consts::TAU;
    let parametric = (
        prop_oneof![
            Just(StandardGate::RX),
            Just(StandardGate::RY),
            Just(StandardGate::RZ),
            Just(StandardGate::Phase),
        ],
        0usize..num_qubits,
        angle.clone(),
    )
        .prop_map(|(gate, qubit, theta)| GeneratedMatrixGate::Parametric(gate, qubit, theta));

    if num_qubits == 1 {
        return prop_oneof![single, parametric].boxed();
    }

    let two_qubits = (0usize..num_qubits, 0usize..num_qubits).prop_filter(
        "two-qubit operation endpoints must be distinct",
        |(first, second)| first != second,
    );
    let two = (
        prop_oneof![
            Just(StandardGate::CX),
            Just(StandardGate::CZ),
            Just(StandardGate::SWAP),
        ],
        two_qubits.clone(),
    )
        .prop_map(|(gate, (first, second))| GeneratedMatrixGate::Two(gate, first, second));
    let two_parametric = (
        prop_oneof![
            Just(StandardGate::CRX),
            Just(StandardGate::CRY),
            Just(StandardGate::CRZ),
            Just(StandardGate::RXX),
            Just(StandardGate::RYY),
            Just(StandardGate::RZZ),
        ],
        two_qubits,
        angle,
    )
        .prop_map(|(gate, (first, second), theta)| {
            GeneratedMatrixGate::TwoParametric(gate, first, second, theta)
        });

    prop_oneof![single, parametric, two, two_parametric].boxed()
}

fn generated_line5_long_range_gate() -> BoxedStrategy<GeneratedMatrixGate> {
    let single = (
        prop_oneof![
            Just(StandardGate::H),
            Just(StandardGate::X),
            Just(StandardGate::Z),
            Just(StandardGate::RZ),
        ],
        0usize..5,
        -std::f64::consts::TAU..std::f64::consts::TAU,
    )
        .prop_map(|(gate, qubit, theta)| match gate {
            StandardGate::RZ => GeneratedMatrixGate::Parametric(gate, qubit, theta),
            _ => GeneratedMatrixGate::Single(gate, qubit),
        });
    let pair = prop_oneof![
        Just((0usize, 2usize)),
        Just((0usize, 3usize)),
        Just((0usize, 4usize)),
        Just((1usize, 3usize)),
        Just((1usize, 4usize)),
        Just((2usize, 4usize)),
    ];
    let two = (
        prop_oneof![Just(StandardGate::CX), Just(StandardGate::CZ)],
        pair.clone(),
    )
        .prop_map(|(gate, (first, second))| GeneratedMatrixGate::Two(gate, first, second));
    let two_parametric = (
        prop_oneof![
            Just(StandardGate::CRX),
            Just(StandardGate::CRY),
            Just(StandardGate::CRZ)
        ],
        pair,
        -std::f64::consts::TAU..std::f64::consts::TAU,
    )
        .prop_map(|(gate, (first, second), theta)| {
            GeneratedMatrixGate::TwoParametric(gate, first, second, theta)
        });

    prop_oneof![single, two, two_parametric].boxed()
}

fn append_generated_matrix_gate(circuit: &mut Circuit, operation: GeneratedMatrixGate) {
    match operation {
        GeneratedMatrixGate::Single(_, qubit) if circuit.qubits().is_empty() => {
            debug_assert_eq!(qubit, 0);
        }
        GeneratedMatrixGate::Single(gate, qubit) => {
            circuit
                .append(
                    Instruction::Standard(gate),
                    [Qubit::new(qubit as u32)],
                    std::iter::empty(),
                    None,
                )
                .unwrap();
        }
        GeneratedMatrixGate::Parametric(gate, qubit, theta) => {
            circuit
                .append(
                    Instruction::Standard(gate),
                    [Qubit::new(qubit as u32)],
                    [ParameterValue::Fixed(theta)],
                    None,
                )
                .unwrap();
        }
        GeneratedMatrixGate::Two(gate, first, second) => {
            circuit
                .append(
                    Instruction::Standard(gate),
                    [Qubit::new(first as u32), Qubit::new(second as u32)],
                    std::iter::empty(),
                    None,
                )
                .unwrap();
        }
        GeneratedMatrixGate::TwoParametric(gate, first, second, theta) => {
            circuit
                .append(
                    Instruction::Standard(gate),
                    [Qubit::new(first as u32), Qubit::new(second as u32)],
                    [ParameterValue::Fixed(theta)],
                    None,
                )
                .unwrap();
        }
    }
}

/// Returns whether a circuit still contains non-lowered gate-like instructions.
pub(super) fn contains_high_level_gate(circuit: &Circuit) -> bool {
    circuit.operations().iter().any(|operation| {
        matches!(
            operation.instruction,
            Instruction::CircuitGate(_) | Instruction::UnitaryGate(_) | Instruction::McGate(_)
        )
    })
}
