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
    CompileConfig, CompileMode, CompileTarget, DeviceCompileTarget, compile, compile_owned,
};
use crate::circuit::{
    Circuit, CircuitParam, Instruction, MCGate, Parameter, ParameterValue, Qubit, StandardGate,
};
use crate::compile::CompilerError;
use crate::compile::resource::ResourcePolicy;
use crate::compile::test_utils::{
    assert_compiled_circuit_equivalent, assert_only_standard_gates,
    assert_two_qubit_operations_supported_by_topology, bell_circuit, contains_high_level_gate,
    generated_small_matrix_circuit, generated_small_routable_circuit, ghz_circuit, qft3_circuit,
    standard_ops,
};
use crate::device::{Device, Layout};
use proptest::prelude::*;
use std::collections::HashMap;
use std::f64::consts::PI;

fn compile_normal(circuit: &Circuit) -> super::CompileResult {
    compile(
        circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Logical,
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap()
}

#[test]
fn owned_compile_entry_matches_borrowed_entry() {
    let circuit = bell_circuit();
    let config = CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Logical,
        resource_policy: ResourcePolicy::default(),
    };

    let borrowed = compile(&circuit, config.clone()).unwrap();
    let owned = compile_owned(circuit.clone(), config).unwrap();

    assert_eq!(owned, borrowed);
    assert_eq!(circuit, bell_circuit());
}

fn assert_compiled_matrix_equivalent(actual: &Circuit, expected: &Circuit) {
    assert_compiled_circuit_equivalent(actual, expected);
}

fn operation_parameter(circuit: &Circuit, param: &CircuitParam) -> Parameter {
    match param {
        CircuitParam::Fixed(value) => Parameter::from(*value),
        CircuitParam::Index(index) => circuit
            .parameters()
            .get_index(*index as usize)
            .cloned()
            .expect("parameter index should exist in compiled circuit"),
    }
}

fn stable_circuit_debug(circuit: &Circuit) -> String {
    let parameters = circuit
        .parameters()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
        circuit.qubits(),
        circuit.symbols(),
        parameters,
        circuit.classical_vars(),
        circuit.classical_values(),
        circuit.operations(),
        circuit.global_phase().to_string(),
    )
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
        assert_compiled_matrix_equivalent(&bound_compiled, &bound_source);
    }
}

fn compile_to_basis(circuit: &Circuit, basis: Vec<StandardGate>) -> super::CompileResult {
    compile(
        circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Basis(
                basis
                    .into_iter()
                    .map(Instruction::Standard)
                    .collect::<Vec<_>>(),
            ),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap()
}

fn compile_to_basis_checked(circuit: &Circuit, basis: &[StandardGate]) -> super::CompileResult {
    let result = compile_to_basis(circuit, basis.to_vec());
    assert!(
        result.step_changed("translate.target_basis"),
        "target-basis translation should change circuit for basis {basis:?}"
    );
    assert_only_standard_gates(&result.circuit, basis);
    assert_compiled_matrix_equivalent(&result.circuit, circuit);
    result
}

fn compile_on_device_checked(
    circuit: &Circuit,
    device: Device,
    seed: u32,
    allowed: &[StandardGate],
) -> super::CompileResult {
    let topology = device.topology().clone();
    let validation_device = device.clone();
    let result = compile(
        circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Device(DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: Some(seed),
            }),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap();

    assert!(
        result
            .steps
            .iter()
            .any(|step| step.name == "route.sabre" && !step.skipped),
        "routing step should run"
    );
    assert_only_standard_gates(&result.circuit, allowed);
    assert_two_qubit_operations_supported_by_topology(&result.circuit, &topology);
    validation_device.validate_circuit(&result.circuit).unwrap();
    assert!(result.circuit.qubits().len() <= topology.num_qubits());
    result
}

fn native_basis(gates: &[StandardGate]) -> Vec<Instruction> {
    gates.iter().copied().map(Instruction::Standard).collect()
}

fn qcis_native_basis() -> Vec<StandardGate> {
    vec![
        StandardGate::I,
        StandardGate::RZ,
        StandardGate::X2P,
        StandardGate::X2M,
        StandardGate::Y2P,
        StandardGate::Y2M,
        StandardGate::XY2P,
        StandardGate::XY2M,
        StandardGate::CZ,
        StandardGate::GPhase,
    ]
}

fn qcis_cz_basis() -> Vec<StandardGate> {
    vec![
        StandardGate::RZ,
        StandardGate::X2P,
        StandardGate::X2M,
        StandardGate::Y2P,
        StandardGate::Y2M,
        StandardGate::CZ,
        StandardGate::GPhase,
    ]
}

fn toffoli_circuit() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
            vec![q0, q1, q2],
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();
    circuit
}

fn single_qubit_gate_suite() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.h(q0).unwrap();
    circuit.x(q1).unwrap();
    circuit.y(q2).unwrap();
    circuit.z(q0).unwrap();
    circuit.s(q1).unwrap();
    circuit.sdg(q2).unwrap();
    circuit.t(q0).unwrap();
    circuit.tdg(q1).unwrap();
    circuit.phase(q2, 0.37).unwrap();
    circuit.rx(q0, 0.31).unwrap();
    circuit.ry(q1, -0.29).unwrap();
    circuit.rz(q2, 0.43).unwrap();
    circuit.rxy(q0, 0.27, -0.19).unwrap();
    circuit.xy(q1, 0.41).unwrap();
    circuit.u(q2, 0.23, -0.17, 0.11).unwrap();
    circuit.x2p(q0).unwrap();
    circuit.x2m(q1).unwrap();
    circuit.y2p(q2).unwrap();
    circuit.y2m(q0).unwrap();
    circuit.xy2p(q1, 0.13).unwrap();
    circuit.xy2m(q2, -0.21).unwrap();
    circuit
}

fn two_qubit_gate_suite() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let q3 = Qubit::new(3);
    let mut circuit = Circuit::new(4);
    circuit.h(q0).unwrap();
    circuit.rx(q1, 0.17).unwrap();
    circuit.ry(q2, -0.23).unwrap();
    circuit.rz(q3, 0.29).unwrap();
    circuit.cx(q0, q2).unwrap();
    circuit.cy(q1, q3).unwrap();
    circuit.cz(q2, q0).unwrap();
    circuit.swap(q0, q3).unwrap();
    circuit.crx(q3, q1, 0.31).unwrap();
    circuit.cry(q2, q0, -0.37).unwrap();
    circuit.crz(q1, q2, 0.43).unwrap();
    circuit.rxx(q0, q1, 0.19).unwrap();
    circuit.ryy(q2, q3, -0.27).unwrap();
    circuit.rzz(q0, q2, 0.33).unwrap();
    circuit.rzx(q3, q1, -0.39).unwrap();
    circuit.fsim(q1, q2, 0.21, -0.35).unwrap();
    circuit
}

fn two_qubit_gate_suite_without_fsim() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let q3 = Qubit::new(3);
    let mut circuit = Circuit::new(4);
    circuit.h(q0).unwrap();
    circuit.rx(q1, 0.17).unwrap();
    circuit.ry(q2, -0.23).unwrap();
    circuit.rz(q3, 0.29).unwrap();
    circuit.cx(q0, q2).unwrap();
    circuit.cy(q1, q3).unwrap();
    circuit.cz(q2, q0).unwrap();
    circuit.swap(q0, q3).unwrap();
    circuit.crx(q3, q1, 0.31).unwrap();
    circuit.cry(q2, q0, -0.37).unwrap();
    circuit.crz(q1, q2, 0.43).unwrap();
    circuit.rxx(q0, q1, 0.19).unwrap();
    circuit.ryy(q2, q3, -0.27).unwrap();
    circuit.rzz(q0, q2, 0.33).unwrap();
    circuit.rzx(q3, q1, -0.39).unwrap();
    circuit
}

fn controlled_rotation_suite() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.h(q0).unwrap();
    circuit.rx(q1, 0.11).unwrap();
    circuit.ry(q2, -0.13).unwrap();
    circuit.crx(q0, q1, 0.23).unwrap();
    circuit.cry(q1, q2, -0.31).unwrap();
    circuit.crz(q2, q0, 0.41).unwrap();
    circuit.crx(q2, q1, -0.29).unwrap();
    circuit.cry(q0, q2, 0.37).unwrap();
    circuit.crz(q1, q0, -0.43).unwrap();
    circuit
}

fn ising_gate_suite() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.h(q0).unwrap();
    circuit.rx(q1, 0.17).unwrap();
    circuit.ry(q2, 0.19).unwrap();
    circuit.rxx(q0, q1, 0.23).unwrap();
    circuit.ryy(q1, q2, -0.29).unwrap();
    circuit.rzz(q0, q2, 0.31).unwrap();
    circuit.rzx(q2, q1, -0.37).unwrap();
    circuit
}

fn fsim_circuit() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.rx(q0, 0.17).unwrap();
    circuit.ry(q1, -0.19).unwrap();
    circuit.fsim(q0, q1, 0.13, 0.41).unwrap();
    circuit
}

fn swap_gate_suite() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.h(q0).unwrap();
    circuit.rx(q1, 0.17).unwrap();
    circuit.ry(q2, -0.19).unwrap();
    circuit.swap(q0, q2).unwrap();
    circuit.swap(q1, q2).unwrap();
    circuit
}

fn multi_controlled_gate_suite() -> Circuit {
    let qubits = (0..5).map(Qubit::new).collect::<Vec<_>>();
    let mut circuit = Circuit::new(5);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(3, StandardGate::X))),
            vec![qubits[0], qubits[1], qubits[2], qubits[3]],
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::RZ))),
            vec![qubits[1], qubits[2], qubits[4]],
            vec![ParameterValue::Fixed(0.31)],
            None,
        )
        .unwrap();
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(1, StandardGate::SWAP))),
            vec![qubits[0], qubits[3], qubits[4]],
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(1, StandardGate::FSIM))),
            vec![qubits[2], qubits[0], qubits[4]],
            vec![ParameterValue::Fixed(0.17), ParameterValue::Fixed(-0.23)],
            None,
        )
        .unwrap();
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::XY2P))),
            vec![qubits[0], qubits[1], qubits[2]],
            vec![ParameterValue::Fixed(0.29)],
            None,
        )
        .unwrap();
    circuit
}

fn long_range_device_circuit() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let q3 = Qubit::new(3);
    let mut circuit = Circuit::new(4);
    circuit.h(q0).unwrap();
    circuit.rx(q1, 0.17).unwrap();
    circuit.ry(q2, -0.19).unwrap();
    circuit.rz(q3, 0.23).unwrap();
    circuit.cx(q0, q3).unwrap();
    circuit.crx(q3, q1, 0.31).unwrap();
    circuit.rzz(q0, q2, -0.37).unwrap();
    circuit.fsim(q1, q3, 0.21, -0.27).unwrap();
    circuit.swap(q0, q2).unwrap();
    circuit
}

fn dense_four_qubit_device_circuit() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let q3 = Qubit::new(3);
    let mut circuit = Circuit::new(4);
    circuit.h(q0).unwrap();
    circuit.h(q1).unwrap();
    circuit.rx(q2, 0.11).unwrap();
    circuit.ry(q3, -0.13).unwrap();
    circuit.cx(q0, q2).unwrap();
    circuit.cz(q1, q3).unwrap();
    circuit.rxx(q0, q3, 0.23).unwrap();
    circuit.ryy(q1, q2, -0.29).unwrap();
    circuit.crz(q3, q0, 0.31).unwrap();
    circuit
}

fn ising_device_circuit() -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let q3 = Qubit::new(3);
    let mut circuit = Circuit::new(4);
    circuit.rx(q0, 0.17).unwrap();
    circuit.ry(q1, -0.19).unwrap();
    circuit.rz(q2, 0.23).unwrap();
    circuit.h(q3).unwrap();
    circuit.rxx(q0, q3, 0.29).unwrap();
    circuit.ryy(q1, q2, -0.31).unwrap();
    circuit.rzz(q0, q2, 0.37).unwrap();
    circuit.rzx(q3, q1, -0.41).unwrap();
    circuit.fsim(q2, q3, 0.13, -0.17).unwrap();
    circuit
}

// ── Pure logical optimization ──

#[test]
fn compile_result_exposes_step_queries() {
    let result = compile_normal(&Circuit::new(1));

    let step = result
        .step("canonicalize.input")
        .expect("normal workflow should report input canonicalization");
    assert_eq!(step.name, "canonicalize.input");
    assert!(result.step("missing.step").is_none());
    assert!(!result.step_changed("missing.step"));
}

#[test]
fn compile_pipeline_collapses_degenerate_u_to_single_rz() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            Instruction::Standard(StandardGate::U),
            vec![q0],
            vec![
                ParameterValue::Fixed(0.0),
                ParameterValue::Fixed(0.3),
                ParameterValue::Fixed(0.7),
            ],
            None,
        )
        .unwrap();
    let basis = vec![
        StandardGate::RZ,
        StandardGate::X2P,
        StandardGate::X,
        StandardGate::CZ,
    ];

    let result = compile_to_basis(&circuit, basis.clone());

    assert_only_standard_gates(&result.circuit, &basis);
    let physical_ops = result
        .circuit
        .operations()
        .iter()
        .filter(|operation| {
            !matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::GPhase)
            )
        })
        .count();
    assert_eq!(
        physical_ops,
        1,
        "theta=0 U should compile to a single RZ through the full pipeline: {:?}",
        result.circuit.operations()
    );
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
}

#[test]
fn compile_pipeline_preserves_entangling_circuit_with_degenerate_u() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cz(q0, q1).unwrap();
    circuit
        .append(
            Instruction::Standard(StandardGate::U),
            vec![q0],
            vec![
                ParameterValue::Fixed(0.0),
                ParameterValue::Fixed(0.3),
                ParameterValue::Fixed(0.7),
            ],
            None,
        )
        .unwrap();
    circuit
        .append(
            Instruction::Standard(StandardGate::U),
            vec![q1],
            vec![
                ParameterValue::Fixed(std::f64::consts::FRAC_PI_2),
                ParameterValue::Fixed(-0.2),
                ParameterValue::Fixed(0.9),
            ],
            None,
        )
        .unwrap();
    let basis = vec![
        StandardGate::RZ,
        StandardGate::X2P,
        StandardGate::X,
        StandardGate::CZ,
    ];

    let result = compile_to_basis(&circuit, basis.clone());

    assert_only_standard_gates(&result.circuit, &basis);
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
}

#[test]
fn compile_bell_to_h_cz_basis() {
    let circuit = bell_circuit();
    let result = compile(
        &circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Basis(vec![
                Instruction::Standard(StandardGate::H),
                Instruction::Standard(StandardGate::CZ),
            ]),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap();

    assert!(result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![
            StandardGate::H,
            StandardGate::H,
            StandardGate::CZ,
            StandardGate::H
        ]
    );
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
    let ops = result.circuit.operations();
    assert_eq!(ops[0].qubits.as_slice(), &[Qubit::new(0)]); // H on q0
    assert_eq!(ops[1].qubits.as_slice(), &[Qubit::new(1)]); // H on q1
    assert_eq!(ops[2].qubits.as_slice(), &[Qubit::new(0), Qubit::new(1)]); // CZ
    assert_eq!(ops[3].qubits.as_slice(), &[Qubit::new(1)]); // H on q1
}

#[test]
fn compile_qft3_without_target_basis_preserves_unitary() {
    let circuit = qft3_circuit();
    let result = compile_normal(&circuit);

    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
    assert!(!contains_high_level_gate(&result.circuit));
}

#[test]
fn compile_preserves_unitary_for_varied_logical_inputs() {
    let mut controlled_rotation = Circuit::new(3);
    controlled_rotation.h(Qubit::new(0)).unwrap();
    controlled_rotation
        .crx(Qubit::new(0), Qubit::new(1), 0.31)
        .unwrap();
    controlled_rotation
        .cry(Qubit::new(1), Qubit::new(2), -0.27)
        .unwrap();
    controlled_rotation
        .crz(Qubit::new(2), Qubit::new(0), 0.19)
        .unwrap();

    for circuit in [
        bell_circuit(),
        qft3_circuit(),
        controlled_rotation,
        toffoli_circuit(),
        fsim_circuit(),
    ] {
        let result = compile_normal(&circuit);

        assert_compiled_matrix_equivalent(&result.circuit, &circuit);
        assert!(!contains_high_level_gate(&result.circuit));
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn compile_preserves_unitary_for_generated_small_circuits(circuit in generated_small_matrix_circuit()) {
        let result = compile_normal(&circuit);

        assert_compiled_matrix_equivalent(&result.circuit, &circuit);
        prop_assert!(!contains_high_level_gate(&result.circuit));
    }

    #[test]
    fn compile_with_same_seed_is_deterministic_for_generated_routable_circuits(
        circuit in generated_small_routable_circuit()
    ) {
        let basis = qcis_cz_basis();
        let device = Device::line("property-line", 5)
            .unwrap()
            .with_native_gates(native_basis(&basis))
            .unwrap();
        let config = CompileConfig {
                         mode: CompileMode::Enhanced,
                         target: CompileTarget::Device(DeviceCompileTarget {
                            device,
                             initial_layout: None,
                             seed: Some(2026),
                         }),
                         resource_policy: ResourcePolicy::default(),
                     };

        let first = compile(&circuit, config.clone()).unwrap();
        let second = compile(&circuit, config).unwrap();

        prop_assert_eq!(
            stable_circuit_debug(&first.circuit),
            stable_circuit_debug(&second.circuit)
        );
        prop_assert_eq!(first.changed, second.changed);
        prop_assert_eq!(first.steps, second.steps);
    }
}

#[test]
fn compile_with_same_seed_is_deterministic() {
    let circuit = dense_four_qubit_device_circuit();
    let basis = qcis_cz_basis();
    let device = Device::ring("deterministic-ring", 4)
        .unwrap()
        .with_native_gates(native_basis(&basis))
        .unwrap();
    let config = CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::Device(DeviceCompileTarget {
            device,
            initial_layout: None,
            seed: Some(1234),
        }),
        resource_policy: ResourcePolicy::default(),
    };

    let first = compile(&circuit, config.clone()).unwrap();
    let second = compile(&circuit, config).unwrap();

    assert_eq!(
        stable_circuit_debug(&first.circuit),
        stable_circuit_debug(&second.circuit)
    );
    assert_eq!(first.changed, second.changed);
    assert_eq!(first.mode, second.mode);
    assert_eq!(first.steps, second.steps);
}

#[test]
fn compile_with_same_seed_and_initial_layout_is_deterministic() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();
    let layout = Layout::from_pairs(&[(0, 2), (1, 0)], 3).unwrap();
    let config = CompileConfig {
        mode: CompileMode::Enhanced,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: Device::line("initial-layout-line", 3)
                .unwrap()
                .with_native_gates(native_basis(&[StandardGate::H, StandardGate::CX]))
                .unwrap(),
            initial_layout: Some(layout),
            seed: Some(99),
        }),
        resource_policy: ResourcePolicy::default(),
    };

    let first = compile(&circuit, config.clone()).unwrap();
    let second = compile(&circuit, config).unwrap();

    assert_eq!(
        stable_circuit_debug(&first.circuit),
        stable_circuit_debug(&second.circuit)
    );
    assert_eq!(first.changed, second.changed);
    assert_eq!(first.steps, second.steps);
    assert!(
        first
            .steps
            .iter()
            .find(|step| step.name == "route.sabre")
            .and_then(|step| step.reason.as_deref())
            .is_some_and(|reason| reason.contains("supplied initial layout"))
    );
}

#[test]
fn compile_qft3_reports_unsupported_h_cz_target_basis() {
    let circuit = qft3_circuit();
    let err = compile(
        &circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Basis(vec![
                Instruction::Standard(StandardGate::H),
                Instruction::Standard(StandardGate::CZ),
            ]),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap_err();

    assert!(matches!(
        err,
        CompilerError::InvalidInput(reason) if reason.contains("CRZ")
    ));
}

#[test]
fn compile_cancels_adjacent_self_inverse_across_full_pipeline() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();
    circuit.x(q0).unwrap();
    circuit.h(q0).unwrap();
    circuit.x(q0).unwrap();

    let result = compile_normal(&circuit);

    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
    // H·X·H·X = (H·X·H)·X — H and X don't cancel directly, but the pipeline
    // should canonicalize and apply knowledge-rule optimizations.
    assert!(
        standard_ops(&result.circuit).len() <= 4,
        "optimization should not increase gate count"
    );
}

#[test]
fn compile_merges_consecutive_same_axis_rotations() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rz(q0, 0.25).unwrap();
    circuit.rz(q0, 0.5).unwrap();
    circuit.rz(q0, -0.75).unwrap();

    let result = compile(
        &circuit,
        CompileConfig {
            mode: CompileMode::Enhanced,
            target: CompileTarget::Logical,
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap();

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
}

#[test]
fn compile_merges_symbolic_single_qubit_rotation() {
    let q0 = Qubit::new(0);
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(1);
    circuit.rz(q0, theta.clone()).unwrap();
    circuit.rz(q0, 0.5).unwrap();

    let result = compile_normal(&circuit);

    assert!(result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::RZ]);
    let merged = operation_parameter(&result.circuit, &result.circuit.operations()[0].params[0]);
    assert!(merged.provably_equal(&(theta.clone() + Parameter::from(0.5)), 1e-12));
    assert!(merged.get_symbols().contains("theta"));
    assert_bindings_preserve_semantics(
        &circuit,
        &result.circuit,
        &[
            binding_case(&[("theta", 0.0)]),
            binding_case(&[("theta", 0.25)]),
            binding_case(&[("theta", -0.5)]),
            binding_case(&[("theta", PI / 2.0)]),
        ],
    );
}

#[test]
fn compile_preserves_parameterized_two_qubit_decomposition() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let theta = Parameter::symbol("theta");
    let phi = Parameter::symbol("phi");
    let mut circuit = Circuit::new(2);
    circuit.rx(q0, 0.17).unwrap();
    circuit.crz(q0, q1, theta.clone()).unwrap();
    circuit.fsim(q0, q1, 0.13, phi.clone()).unwrap();

    let basis = qcis_cz_basis();
    let result = compile_to_basis(&circuit, basis.clone());

    assert!(result.step_changed("translate.target_basis"));
    assert_only_standard_gates(&result.circuit, &basis);
    assert!(result.circuit.uses_symbol("theta"));
    assert!(result.circuit.uses_symbol("phi"));
    assert_bindings_preserve_semantics(
        &circuit,
        &result.circuit,
        &[
            binding_case(&[("theta", 0.0), ("phi", 0.0)]),
            binding_case(&[("theta", 0.2), ("phi", -0.3)]),
            binding_case(&[("theta", PI / 4.0), ("phi", -PI / 7.0)]),
        ],
    );
}

#[test]
fn compile_preserves_parameterized_mc_gate_decomposition() {
    let qubits = (0..4).map(Qubit::new).collect::<Vec<_>>();
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(4);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::RZ))),
            vec![qubits[0], qubits[1], qubits[3]],
            vec![ParameterValue::Param(theta.clone())],
            None,
        )
        .unwrap();

    let result = compile_normal(&circuit);

    assert!(result.step_changed("decompose.mc_gates"));
    assert!(!contains_high_level_gate(&result.circuit));
    assert!(result.circuit.uses_symbol("theta"));
    assert_bindings_preserve_semantics(
        &circuit,
        &result.circuit,
        &[
            binding_case(&[("theta", 0.0)]),
            binding_case(&[("theta", 0.31)]),
            binding_case(&[("theta", -PI / 3.0)]),
        ],
    );
}

#[test]
fn compile_routes_parameterized_circuit_and_preserves_semantics() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let theta = Parameter::symbol("theta");
    let phi = Parameter::symbol("phi");
    let mut circuit = Circuit::new(3);
    circuit.rx(q0, theta.clone()).unwrap();
    circuit.rz(q2, phi.clone()).unwrap();
    circuit.cx(q0, q2).unwrap();
    circuit.rzz(q1, q2, 0.37).unwrap();

    let result = compile(
        &circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Device(DeviceCompileTarget {
                device: Device::line("param-line", 3)
                    .unwrap()
                    .with_native_gates(native_basis(&[
                        StandardGate::H,
                        StandardGate::RX,
                        StandardGate::RZ,
                        StandardGate::RZZ,
                        StandardGate::CX,
                    ]))
                    .unwrap(),
                initial_layout: None,
                seed: Some(9),
            }),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap();

    assert!(
        result
            .steps
            .iter()
            .any(|step| step.name == "route.sabre" && !step.skipped)
    );
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
fn compile_target_basis_translation_preserves_parameterized_semantics() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.crz(q0, q1, theta.clone()).unwrap();
    circuit.ry(q1, -0.19).unwrap();

    let basis = qcis_cz_basis();
    let result = compile_to_basis(&circuit, basis.clone());

    assert!(result.step_changed("translate.target_basis"));
    assert_only_standard_gates(&result.circuit, &basis);
    assert!(result.circuit.uses_symbol("theta"));
    assert_bindings_preserve_semantics(
        &circuit,
        &result.circuit,
        &[
            binding_case(&[("theta", 0.0)]),
            binding_case(&[("theta", 0.41)]),
            binding_case(&[("theta", -PI / 5.0)]),
        ],
    );
}

// ── Decomposition ──

#[test]
fn compile_decomposes_toffoli_into_standard_gates() {
    let circuit = toffoli_circuit();

    let result = compile_normal(&circuit);

    assert!(!contains_high_level_gate(&result.circuit));
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::CCX]);
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
}

#[test]
fn compile_decomposes_c3x_with_fallback_to_no_auxiliary() {
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

    let result = compile_normal(&circuit);

    assert!(result.step_changed("decompose.mc_gates"));
    assert!(!contains_high_level_gate(&result.circuit));
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
}

#[test]
fn compile_lowers_common_gates_to_qcis_native_basis() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.x(q1).unwrap();
    circuit.y(q0).unwrap();
    circuit.rx(q0, 0.31).unwrap();
    circuit.ry(q1, -0.27).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.crx(q1, q0, 0.19).unwrap();
    circuit.cry(q0, q1, -0.41).unwrap();
    circuit.rzz(q0, q1, 0.53).unwrap();

    let basis = qcis_native_basis();
    let result = compile_to_basis(&circuit, basis.clone());

    assert!(result.step_changed("translate.target_basis"));
    assert_only_standard_gates(&result.circuit, &basis);
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
}

#[test]
fn compile_converts_x2p_and_y2p_to_xy2p_basis() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.x2p(q0).unwrap();
    circuit.y2p(q1).unwrap();

    let result = compile_to_basis(&circuit, vec![StandardGate::XY2P]);

    assert!(result.step_changed("translate.target_basis"));
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::XY2P; 2]);
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
}

#[test]
fn compile_converts_xy2p_to_x2p_rz_basis() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.xy2p(q0, 0.37).unwrap();

    let basis = vec![StandardGate::RZ, StandardGate::X2P];
    let result = compile_to_basis(&circuit, basis.clone());

    assert!(result.step_changed("translate.target_basis"));
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::RZ, StandardGate::X2P, StandardGate::RZ]
    );
    assert_only_standard_gates(&result.circuit, &basis);
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
}

#[test]
fn compile_decomposes_multi_controlled_qcis_half_rotations() {
    for (gate, params) in [
        (StandardGate::X2P, vec![]),
        (StandardGate::Y2P, vec![]),
        (StandardGate::XY2P, vec![ParameterValue::Fixed(0.73)]),
    ] {
        let mut circuit = Circuit::new(4);
        circuit
            .append(
                Instruction::McGate(Box::new(MCGate::new(3, gate))),
                vec![Qubit::new(0), Qubit::new(1), Qubit::new(2), Qubit::new(3)],
                params,
                None,
            )
            .unwrap();

        let result = compile_normal(&circuit);

        assert!(result.step_changed("decompose.mc_gates"));
        assert!(!contains_high_level_gate(&result.circuit));
        assert_compiled_matrix_equivalent(&result.circuit, &circuit);
    }
}

#[test]
fn compile_lowers_single_qubit_suite_to_qcis_x_half_basis() {
    let circuit = single_qubit_gate_suite();
    let basis = vec![
        StandardGate::RZ,
        StandardGate::X2P,
        StandardGate::X2M,
        StandardGate::GPhase,
    ];

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_single_qubit_suite_to_qcis_y_half_basis() {
    let circuit = single_qubit_gate_suite();
    let basis = vec![
        StandardGate::RZ,
        StandardGate::Y2P,
        StandardGate::Y2M,
        StandardGate::GPhase,
    ];

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_single_qubit_suite_to_qcis_xy_half_basis() {
    let circuit = single_qubit_gate_suite();
    let basis = vec![
        StandardGate::RZ,
        StandardGate::XY2P,
        StandardGate::XY2M,
        StandardGate::GPhase,
    ];

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_single_qubit_suite_to_ion_trap_rx_ry_basis() {
    let circuit = single_qubit_gate_suite();
    let basis = vec![StandardGate::RX, StandardGate::RY, StandardGate::GPhase];

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_two_qubit_suite_to_qcis_cz_basis() {
    let circuit = two_qubit_gate_suite();
    let basis = qcis_cz_basis();

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_ccx_to_clifford_t_cx_basis() {
    let circuit = {
        let mut circuit = Circuit::new(3);
        circuit
            .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
            .unwrap();
        circuit
    };
    let basis = vec![
        StandardGate::H,
        StandardGate::CX,
        StandardGate::T,
        StandardGate::TDG,
    ];

    let result = compile_to_basis_checked(&circuit, &basis);

    assert!(standard_ops(&result.circuit).contains(&StandardGate::CX));
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::CCX));
}

#[test]
fn compile_lowers_ccx_to_clifford_t_cz_basis() {
    let circuit = {
        let mut circuit = Circuit::new(3);
        circuit
            .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
            .unwrap();
        circuit
    };
    let basis = vec![
        StandardGate::H,
        StandardGate::CZ,
        StandardGate::T,
        StandardGate::TDG,
    ];

    let result = compile_to_basis_checked(&circuit, &basis);

    assert!(standard_ops(&result.circuit).contains(&StandardGate::CZ));
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::CCX));
}

#[test]
fn compile_lowers_ccx_to_ion_trap_rx_ry_rzz_basis() {
    let circuit = {
        let mut circuit = Circuit::new(3);
        circuit
            .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
            .unwrap();
        circuit
    };
    let basis = vec![
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZZ,
        StandardGate::GPhase,
    ];

    let result = compile_to_basis_checked(&circuit, &basis);

    assert!(!standard_ops(&result.circuit).contains(&StandardGate::CCX));
}

#[test]
fn compile_lowers_two_qubit_suite_to_cx_native_basis() {
    let circuit = two_qubit_gate_suite_without_fsim();
    let basis = vec![
        StandardGate::H,
        StandardGate::S,
        StandardGate::SDG,
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZ,
        StandardGate::CX,
        StandardGate::GPhase,
    ];

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_two_qubit_suite_to_cz_native_basis() {
    let circuit = two_qubit_gate_suite_without_fsim();
    let basis = vec![
        StandardGate::H,
        StandardGate::S,
        StandardGate::SDG,
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZ,
        StandardGate::CZ,
        StandardGate::GPhase,
    ];

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_two_qubit_suite_to_ion_trap_rx_ry_rzz_basis() {
    let circuit = two_qubit_gate_suite();
    let basis = vec![
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZZ,
        StandardGate::GPhase,
    ];

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_controlled_rotations_to_rzz_native_basis() {
    let circuit = controlled_rotation_suite();
    let basis = vec![
        StandardGate::H,
        StandardGate::RX,
        StandardGate::RZ,
        StandardGate::RZZ,
        StandardGate::GPhase,
    ];

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_controlled_rotations_to_rzx_native_basis() {
    let circuit = controlled_rotation_suite();
    let basis = vec![
        StandardGate::H,
        StandardGate::S,
        StandardGate::SDG,
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZ,
        StandardGate::RZX,
        StandardGate::GPhase,
    ];

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_controlled_rotations_to_ion_trap_rx_ry_rzz_basis() {
    let circuit = controlled_rotation_suite();
    let basis = vec![
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZZ,
        StandardGate::GPhase,
    ];

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_swap_to_ising_exchange_basis() {
    let circuit = swap_gate_suite();
    let basis = vec![
        StandardGate::H,
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RXX,
        StandardGate::RYY,
        StandardGate::RZZ,
        StandardGate::GPhase,
    ];

    let result = compile_to_basis_checked(&circuit, &basis);

    assert!(standard_ops(&result.circuit).contains(&StandardGate::RXX));
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::SWAP));
}

#[test]
fn compile_lowers_swap_to_ion_trap_rx_ry_rzz_basis() {
    let circuit = swap_gate_suite();
    let basis = vec![
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZZ,
        StandardGate::GPhase,
    ];

    let result = compile_to_basis_checked(&circuit, &basis);

    assert!(!standard_ops(&result.circuit).contains(&StandardGate::SWAP));
}

#[test]
fn compile_lowers_ising_suite_to_rzz_native_basis() {
    let circuit = ising_gate_suite();
    let basis = vec![
        StandardGate::H,
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZ,
        StandardGate::RZZ,
        StandardGate::GPhase,
    ];

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_ising_suite_to_ion_trap_rx_ry_rzz_basis() {
    let circuit = ising_gate_suite();
    let basis = vec![
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZZ,
        StandardGate::GPhase,
    ];

    compile_to_basis_checked(&circuit, &basis);
}

#[test]
fn compile_lowers_fsim_to_ising_exchange_basis() {
    let circuit = fsim_circuit();
    let basis = vec![
        StandardGate::H,
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZ,
        StandardGate::RXX,
        StandardGate::RYY,
        StandardGate::RZZ,
        StandardGate::GPhase,
    ];

    let result = compile_to_basis_checked(&circuit, &basis);

    assert!(standard_ops(&result.circuit).contains(&StandardGate::RXX));
    assert!(standard_ops(&result.circuit).contains(&StandardGate::RYY));
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::FSIM));
}

#[test]
fn compile_lowers_fsim_to_ion_trap_rx_ry_rzz_basis() {
    let circuit = fsim_circuit();
    let basis = vec![
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZZ,
        StandardGate::GPhase,
    ];

    let result = compile_to_basis_checked(&circuit, &basis);

    assert!(!standard_ops(&result.circuit).contains(&StandardGate::FSIM));
}

#[test]
fn compile_lowers_multi_controlled_suite_to_qcis_cz_basis() {
    let circuit = multi_controlled_gate_suite();
    let basis = qcis_cz_basis();
    let result = compile_to_basis_checked(&circuit, &basis);

    assert!(result.step_changed("decompose.mc_gates"));
    assert!(!contains_high_level_gate(&result.circuit));
}

#[test]
fn compile_lowers_multi_controlled_suite_to_ion_trap_rx_ry_rzz_basis() {
    let circuit = multi_controlled_gate_suite();
    let basis = vec![
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZZ,
        StandardGate::GPhase,
    ];

    let result = compile_to_basis_checked(&circuit, &basis);

    assert!(result.step_changed("decompose.mc_gates"));
    assert!(!contains_high_level_gate(&result.circuit));
}

// ── Device routing + basis translation ──

#[test]
fn compile_ghz3_routes_on_line_device_and_lowers_to_h_cz() {
    let circuit = ghz_circuit(3);
    let device = Device::line("test-device", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CZ),
        ])
        .unwrap();

    let result = compile(
        &circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Device(DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: Some(42),
            }),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap();

    assert!(
        result
            .steps
            .iter()
            .any(|step| step.name == "route.sabre" && !step.skipped)
    );
    assert!(result.step_changed("lower.device_instructions"));
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
    for op in result.circuit.operations() {
        assert!(matches!(
            op.instruction,
            Instruction::Standard(StandardGate::H | StandardGate::CZ)
        ));
    }
}

#[test]
fn compile_ghz5_routes_on_line_device() {
    let circuit = ghz_circuit(5);
    let device = Device::line("test-device", 5)
        .unwrap()
        .with_native_gates(native_basis(&[StandardGate::H, StandardGate::CX]))
        .unwrap();

    let result = compile(
        &circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Device(DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: Some(17),
            }),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap();

    assert!(
        result
            .steps
            .iter()
            .any(|step| step.name == "route.sabre" && !step.skipped)
    );
    assert!(result.circuit.qubits().len() <= 5);
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
}

#[test]
fn compile_toffoli_on_4q_line_device_decomposes_ccx_before_routing() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(4);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
            vec![q0, q1, q2],
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();
    circuit.h(Qubit::new(3)).unwrap();
    let device = Device::line("test-device", 4)
        .unwrap()
        .with_native_gates(native_basis(&[
            StandardGate::H,
            StandardGate::T,
            StandardGate::TDG,
            StandardGate::CX,
        ]))
        .unwrap();
    let topology = device.topology().clone();

    let result = compile(
        &circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Device(DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: Some(17),
            }),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap();

    assert!(result.step_changed("decompose.routing_basis"));
    assert!(
        result
            .steps
            .iter()
            .any(|step| step.name == "route.sabre" && !step.skipped)
    );
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::CCX));
    assert_two_qubit_operations_supported_by_topology(&result.circuit, &topology);
    assert!(result.circuit.qubits().len() <= topology.num_qubits());
}

#[test]
fn compile_toffoli_routing_basis_prefers_cz_native_decomposition() {
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
            vec![Qubit::new(0), Qubit::new(1), Qubit::new(2)],
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();
    let device = Device::line("cz-native-line", 3)
        .unwrap()
        .with_native_gates(native_basis(&[
            StandardGate::H,
            StandardGate::T,
            StandardGate::TDG,
            StandardGate::CZ,
            StandardGate::GPhase,
        ]))
        .unwrap();
    let topology = device.topology().clone();

    let result = compile(
        &circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Device(DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: Some(19),
            }),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap();

    assert!(result.step_changed("decompose.routing_basis"));
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::CCX));
    assert!(standard_ops(&result.circuit).contains(&StandardGate::CZ));
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::CX));
    assert_two_qubit_operations_supported_by_topology(&result.circuit, &topology);
    assert!(result.circuit.qubits().len() <= topology.num_qubits());
}

#[test]
fn compile_routing_basis_preserves_existing_two_qubit_standard_gates() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.rzz(q0, q1, 0.37).unwrap();
    circuit.crz(q0, q1, 0.19).unwrap();
    circuit.fsim(q0, q1, 0.11, -0.23).unwrap();
    let device = Device::line("two-qubit-line", 2)
        .unwrap()
        .with_native_gates(native_basis(&[
            StandardGate::RZZ,
            StandardGate::CRZ,
            StandardGate::FSIM,
        ]))
        .unwrap();

    let result = compile(
        &circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Device(DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: Some(23),
            }),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap();

    let routing_basis = result
        .steps
        .iter()
        .find(|step| step.name == "decompose.routing_basis")
        .expect("routing basis step should be reported");
    assert!(!routing_basis.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::RZZ, StandardGate::CRZ, StandardGate::FSIM]
    );
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
}

#[test]
fn compile_long_range_circuit_on_line_device_to_qcis_native_basis() {
    let circuit = long_range_device_circuit();
    let basis = qcis_cz_basis();
    let device = Device::line("line-qcis", 4)
        .unwrap()
        .with_native_gates(native_basis(&basis))
        .unwrap();

    let result = compile_on_device_checked(&circuit, device, 101, &basis);

    assert!(result.step_changed("lower.device_instructions"));
    assert!(result.circuit.qubits().len() <= 4);
}

#[test]
fn compile_long_range_circuit_on_ring_device_to_qcis_native_basis() {
    let circuit = long_range_device_circuit();
    let basis = qcis_cz_basis();
    let device = Device::ring("ring-qcis", 4)
        .unwrap()
        .with_native_gates(native_basis(&basis))
        .unwrap();

    let result = compile_on_device_checked(&circuit, device, 102, &basis);

    assert!(result.step_changed("lower.device_instructions"));
}

#[test]
fn compile_dense_circuit_on_bidirectional_line_to_cz_native_basis() {
    let circuit = dense_four_qubit_device_circuit();
    let basis = vec![
        StandardGate::H,
        StandardGate::S,
        StandardGate::SDG,
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZ,
        StandardGate::CZ,
        StandardGate::GPhase,
    ];
    let device = Device::bidirectional_line("bidir-line-cz", 4)
        .unwrap()
        .with_native_gates(native_basis(&basis))
        .unwrap();

    let result = compile_on_device_checked(&circuit, device, 103, &basis);

    assert!(result.step_changed("lower.device_instructions"));
}

#[test]
fn compile_dense_circuit_on_star_device_to_cx_native_basis() {
    let circuit = dense_four_qubit_device_circuit();
    let basis = vec![
        StandardGate::H,
        StandardGate::S,
        StandardGate::SDG,
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZ,
        StandardGate::CX,
        StandardGate::GPhase,
    ];
    let device = Device::star("star-cx", 4, 0)
        .unwrap()
        .with_native_gates(native_basis(&basis))
        .unwrap();

    let result = compile_on_device_checked(&circuit, device, 104, &basis);

    assert!(result.step_changed("lower.device_instructions"));
}

#[test]
fn compile_ising_circuit_on_grid_device_to_ising_native_basis() {
    let circuit = ising_device_circuit();
    let basis = vec![
        StandardGate::H,
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZ,
        StandardGate::RXX,
        StandardGate::RYY,
        StandardGate::RZZ,
        StandardGate::GPhase,
    ];
    let device = Device::grid("grid-ising", 2, 3)
        .unwrap()
        .with_native_gates(native_basis(&basis))
        .unwrap();

    let result = compile_on_device_checked(&circuit, device, 105, &basis);

    assert!(result.step_changed("lower.device_instructions"));
}

// ── Enhanced mode ──

#[test]
fn compile_enhanced_ghz3_runs_post_routing_and_skips_target_cleanup() {
    let circuit = ghz_circuit(3);
    let device = Device::line("test-device", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CZ),
        ])
        .unwrap();

    let result = compile(
        &circuit,
        CompileConfig {
            mode: CompileMode::Enhanced,
            target: CompileTarget::Device(DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: Some(42),
            }),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap();

    assert!(
        result
            .steps
            .iter()
            .any(|step| step.name == "route.sabre" && !step.skipped)
    );
    assert!(
        result
            .steps
            .iter()
            .any(|step| step.name == "optimize.post_routing" && !step.skipped)
    );
    let target_cleanup = result
        .steps
        .iter()
        .find(|step| step.name == "optimize.target_cleanup")
        .unwrap();
    assert!(target_cleanup.skipped);
    assert!(!target_cleanup.changed);
    assert_eq!(
        target_cleanup.reason.as_deref(),
        Some("no explicit target basis configured")
    );
    for op in result.circuit.operations() {
        assert!(matches!(
            op.instruction,
            Instruction::Standard(StandardGate::H | StandardGate::CZ)
        ));
    }
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
}

// ── Error paths ──

#[test]
fn compile_reports_error_for_unsupported_target_basis() {
    let circuit = bell_circuit();
    let err = compile(
        &circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Basis(vec![Instruction::Standard(StandardGate::CZ)]),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap_err();

    assert!(!format!("{err}").is_empty());
}

#[test]
fn compile_rejects_circuit_wider_than_device() {
    let mut circuit = Circuit::new(4);
    circuit.h(Qubit::new(0)).unwrap();
    let device = Device::line("test-device", 2).unwrap();

    let err = compile(
        &circuit,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Device(DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: None,
            }),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap_err();

    assert!(format!("{err}").contains("4 logical qubits"));
}

#[test]
fn test_qasm() {
    use crate::ir::qasm2::loads;

    let c = loads(
        r#"
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
creg c[2];
rz(pi/2) q[0];
sx q[0];
rz(pi/2) q[0];
rz(pi/2) q[1];
sx q[1];
rz(-pi/2) q[1];
cx q[0],q[1];
rz(pi/2) q[0];
sx q[0];
rz(pi/2) q[0];
measure q[0] -> c[0];
measure q[1] -> c[1];
"#,
    )
    .unwrap();
    let basis = vec![
        StandardGate::CZ,
        StandardGate::X,
        StandardGate::RZ,
        StandardGate::X2P,
    ];
    let result = compile(
        &c,
        CompileConfig {
            mode: CompileMode::Enhanced,
            target: CompileTarget::Basis(native_basis(&basis)),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .unwrap();

    assert!(result.step_changed("translate.target_basis"));
    assert!(
        standard_ops(&result.circuit)
            .iter()
            .all(|gate| basis.contains(gate))
    );
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::RY));
}

fn trivial_bvlike_circuit(num_qubits: u32) -> Circuit {
    // benchpress trivial_bvlike motif: an up ladder of CXs into the target,
    // X(target), Z(last control), then the mirrored down ladder. Everything
    // commutes out except the X and Z.
    let target = Qubit::new(num_qubits - 1);
    let last_control = Qubit::new(num_qubits - 2);
    let mut circuit = Circuit::new(num_qubits as usize);
    for control in 0..num_qubits - 1 {
        circuit.cx(Qubit::new(control), target).unwrap();
    }
    circuit.x(target).unwrap();
    circuit.z(last_control).unwrap();
    for control in (0..num_qubits - 1).rev() {
        circuit.cx(Qubit::new(control), target).unwrap();
    }
    circuit
}

#[test]
fn compile_normal_cancels_trivial_bvlike_motif_small() {
    let circuit = trivial_bvlike_circuit(4);

    let result = compile_normal(&circuit);

    assert!(result.changed);
    assert_eq!(result.circuit.operations().len(), 2);
    assert_compiled_matrix_equivalent(&result.circuit, &circuit);
}

#[test]
fn compile_normal_cancels_trivial_bvlike_motif_at_scale() {
    let num_qubits = 20u32;
    let circuit = trivial_bvlike_circuit(num_qubits);

    let result = compile_normal(&circuit);

    assert!(result.changed);
    let operations = result.circuit.operations();
    assert_eq!(operations.len(), 2);
    assert!(matches!(
        operations[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert_eq!(
        operations[0].qubits.as_slice(),
        &[Qubit::new(num_qubits - 1)]
    );
    assert!(matches!(
        operations[1].instruction,
        Instruction::Standard(StandardGate::Z)
    ));
    assert_eq!(
        operations[1].qubits.as_slice(),
        &[Qubit::new(num_qubits - 2)]
    );
}
