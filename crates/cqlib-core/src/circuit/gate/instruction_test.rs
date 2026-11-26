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

use crate::circuit::{
    Circuit, CircuitId, ClassicalExpr, ClassicalType, ClassicalValue, ClassicalVar, Qubit,
};

#[test]
fn circuit_backed_unitary_instruction_matrix_uses_msb_first_qubits() {
    use crate::circuit::circuit_to_matrix;
    use crate::circuit::test_utils::assert_matrix_approx_eq;

    for order in [vec![], vec![7], vec![0, 1], vec![1, 0], vec![9, 2, 7]] {
        let mut numeric_order = order.clone();
        numeric_order.sort_unstable();
        for parameterized in [false, true] {
            let mut body =
                Circuit::from_qubits(order.iter().copied().map(Qubit::new).collect()).unwrap();
            if let Some(&first) = numeric_order.first() {
                body.rx(
                    Qubit::new(first),
                    if parameterized {
                        Parameter::symbol("theta")
                    } else {
                        Parameter::from(0.7)
                    },
                )
                .unwrap();
                if numeric_order.len() > 1 {
                    body.cx(
                        Qubit::new(first),
                        Qubit::new(*numeric_order.last().unwrap()),
                    )
                    .unwrap();
                }
            }
            body.set_global_phase(if parameterized {
                Parameter::symbol("theta")
            } else {
                Parameter::from(0.7)
            });
            let gate = UnitaryGate::new("positional", order.len() as u16, u16::from(parameterized))
                .with_circuit(Arc::new(FrozenCircuit::new(body.clone())))
                .unwrap();
            let params = if parameterized { vec![0.7] } else { vec![] };
            let bindings = Some(std::collections::HashMap::from([("theta", 0.7)]));
            let bound = body.assign_parameters(&bindings).unwrap();
            let lsb_order: Vec<usize> = numeric_order.iter().map(|&q| q as usize).collect();
            // The definition API keeps its existing numeric-ID, LSB-first basis.
            assert_matrix_approx_eq(
                gate.matrix_for_params(&params).unwrap().as_ref(),
                &circuit_to_matrix(&bound, Some(&lsb_order)).unwrap(),
                1e-12,
            );

            let instruction = Instruction::UnitaryGate(Box::new(gate));
            let matrix = instruction.matrix(&params).unwrap();
            let qubits: Vec<Qubit> = [11, 3, 8][..order.len()]
                .iter()
                .copied()
                .map(Qubit::new)
                .collect();
            let mut expected = Circuit::from_qubits(qubits.clone()).unwrap();
            if let Some(&first) = qubits.first() {
                expected.rx(first, Parameter::from(0.7)).unwrap();
                if qubits.len() > 1 {
                    expected.cx(first, *qubits.last().unwrap()).unwrap();
                }
            }
            expected.set_global_phase(Parameter::from(0.7));
            let msb_order: Vec<usize> = qubits.iter().rev().map(|q| q.index()).collect();
            assert_matrix_approx_eq(
                matrix.as_ref(),
                &circuit_to_matrix(&expected, Some(&msb_order)).unwrap(),
                1e-12,
            );
            let operation = crate::circuit::Operation {
                instruction: instruction.clone(),
                qubits: qubits.into_iter().collect(),
                params: params
                    .into_iter()
                    .map(crate::circuit::CircuitParam::Fixed)
                    .collect(),
                label: None,
            };
            assert_matrix_approx_eq(operation.matrix().unwrap().as_ref(), matrix.as_ref(), 1e-12);
            if parameterized {
                for invalid in [vec![], vec![f64::NAN], vec![f64::INFINITY], vec![0.7, 0.2]] {
                    assert!(instruction.matrix(&invalid).is_none());
                }
            }
        }
    }
}

#[test]
fn unitary_instruction_matrix_keeps_explicit_matrix_order_and_precedence() {
    use crate::circuit::symbolic_matrix::standard_gate_symbolic_matrix;

    let mut body = Circuit::from_qubits(vec![Qubit::new(7), Qubit::new(2)]).unwrap();
    body.cx(Qubit::new(2), Qubit::new(7)).unwrap();
    let frozen = Arc::new(FrozenCircuit::new(body));
    let numeric = StandardGate::CX.matrix(&[]).unwrap();
    for with_circuit in [false, true] {
        for symbolic in [false, true] {
            let gate = UnitaryGate::new("explicit_cx", 2, 0);
            let gate = if symbolic {
                gate.with_symbolic_matrix(
                    std::iter::empty::<String>(),
                    standard_gate_symbolic_matrix(StandardGate::CX, &[]).unwrap(),
                )
            } else {
                gate.with_matrix(numeric.clone().into_owned())
            }
            .unwrap();
            let gate = if with_circuit {
                gate.with_circuit(frozen.clone()).unwrap()
            } else {
                gate
            };
            let instruction = Instruction::UnitaryGate(Box::new(gate));
            let matrix = instruction.matrix(&[]).unwrap();
            assert_eq!(matrix.as_ref(), numeric.as_ref());
            assert_eq!(matches!(matrix, Cow::Borrowed(_)), !symbolic);
        }
    }
}

#[test]
fn circuit_gate_operation_matrix_respects_positional_qubits() {
    for order in [[0, 1], [1, 0], [7, 2]] {
        let mut body = Circuit::from_qubits(order.map(Qubit::new).to_vec()).unwrap();
        body.x(Qubit::new(order[0])).unwrap();
        let instruction = body.to_gate("first_x").unwrap();
        let matrix = instruction.matrix(&[]).unwrap();
        // First argument is the MSB of a gate-local matrix: X tensor I.
        for row in 0..4 {
            for col in 0..4 {
                let expected = if row == (col ^ 2) { 1.0 } else { 0.0 };
                assert_eq!(matrix[[row, col]], Complex64::new(expected, 0.0));
            }
        }
    }
}

#[test]
fn circuit_gate_operation_matrix_binds_numeric_arguments() {
    let mut body = Circuit::from_qubits(vec![Qubit::new(7)]).unwrap();
    body.rx(Qubit::new(7), Parameter::symbol("theta")).unwrap();
    let instruction = body.to_gate("rx").unwrap();
    for angle in [0.37_f64, -0.61] {
        let matrix = instruction.matrix(&[angle]).unwrap();
        let mut operation = crate::circuit::Operation {
            instruction: instruction.clone(),
            qubits: smallvec::smallvec![Qubit::new(7)],
            params: smallvec::smallvec![crate::circuit::CircuitParam::Fixed(angle)],
            label: None,
        };
        assert_eq!(operation.matrix().unwrap().as_ref(), matrix.as_ref());
        operation.params[0] = crate::circuit::CircuitParam::Index(0);
        assert!(matches!(
            operation.matrix(),
            Err(crate::circuit::CircuitError::SymbolicParameterError)
        ));
        for row in 0..2 {
            for col in 0..2 {
                let expected = if row == col {
                    Complex64::new((angle / 2.0).cos(), 0.0)
                } else {
                    Complex64::new(0.0, -(angle / 2.0).sin())
                };
                assert!((matrix[[row, col]] - expected).norm() < 1e-12);
            }
        }
    }
}

#[test]
fn standard_gate_query_distinguishes_instruction_kinds() {
    assert_eq!(
        Instruction::Standard(StandardGate::H).standard_gate(),
        Some(StandardGate::H)
    );
    assert_eq!(
        Instruction::Directive(Directive::Barrier).standard_gate(),
        None
    );
}

#[test]
fn stable_names_cover_every_instruction_category() {
    let circuit_id = CircuitId::new();
    let measured = ClassicalValue::new(circuit_id, 0, ClassicalType::Bit);
    let circuit_gate = CircuitGate::new("composite", FrozenCircuit::new(Circuit::new(1))).unwrap();
    let cases = [
        (Instruction::Standard(StandardGate::CX), "CX", "standard"),
        (
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
            "C2-X",
            "mcgate",
        ),
        (
            Instruction::UnitaryGate(Box::new(UnitaryGate::new("oracle", 1, 0))),
            "oracle",
            "unitary",
        ),
        (
            Instruction::CircuitGate(Box::new(circuit_gate)),
            "composite",
            "circuit",
        ),
        (
            Instruction::Directive(Directive::Barrier),
            "barrier",
            "directive",
        ),
        (
            Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result: measured }),
            "measure_bit",
            "classical_data",
        ),
        (
            Instruction::ClassicalControl(ClassicalControlOp::Break),
            "break",
            "classical_control",
        ),
        (Instruction::Delay, "delay", "delay"),
    ];

    for (instruction, expected_name, expected_type) in cases {
        assert_eq!(instruction.name(), expected_name);
        assert_eq!(instruction.instruction_type(), expected_type);
        assert_eq!(instruction.to_string(), expected_name);
    }

    assert_eq!(MCGate::new(0, StandardGate::H).name(), "H");
    assert_eq!(Directive::Measure.name(), "measure");
    assert_eq!(Directive::Reset.name(), "reset");
}

#[test]
fn quantum_gate_classification_excludes_side_effecting_instructions() {
    let circuit_id = CircuitId::new();
    let measured = ClassicalValue::new(circuit_id, 0, ClassicalType::Bit);

    for instruction in [
        Instruction::Standard(StandardGate::H),
        Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
        Instruction::UnitaryGate(Box::new(UnitaryGate::new("u", 1, 0))),
        Instruction::CircuitGate(Box::new(
            CircuitGate::new("c", FrozenCircuit::new(Circuit::new(1))).unwrap(),
        )),
    ] {
        assert!(instruction.is_quantum_gate());
    }

    for instruction in [
        Instruction::Directive(Directive::Measure),
        Instruction::Directive(Directive::Reset),
        Instruction::Directive(Directive::Barrier),
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result: measured }),
        Instruction::ClassicalControl(ClassicalControlOp::Continue),
        Instruction::Delay,
    ] {
        assert!(!instruction.is_quantum_gate());
    }
}

#[test]
fn reports_measurements_and_classical_value_reads() {
    let circuit_id = CircuitId::new();
    let result = ClassicalValue::new(circuit_id, 0, ClassicalType::Bit);
    let target = ClassicalVar::new(circuit_id, 0, ClassicalType::Bit);

    let measurement = Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result });
    assert!(measurement.has_measurement());
    assert!(!measurement.reads_value(result));

    let store = Instruction::ClassicalData(ClassicalDataOp::Store {
        target,
        value: ClassicalExpr::value(result),
    });
    assert!(!store.has_measurement());
    assert!(store.reads_value(result));

    let gate = Instruction::Standard(StandardGate::X);
    assert!(!gate.has_measurement());
    assert!(!gate.reads_value(result));
}

#[test]
fn canonicalize_form_collapses_supported_mc_gate_forms() {
    let cases = [
        (MCGate::new(0, StandardGate::X), StandardGate::X),
        (MCGate::new(1, StandardGate::X), StandardGate::CX),
        (MCGate::new(2, StandardGate::X), StandardGate::CCX),
        (MCGate::new(1, StandardGate::CX), StandardGate::CCX),
        (MCGate::new(1, StandardGate::Y), StandardGate::CY),
        (MCGate::new(1, StandardGate::Z), StandardGate::CZ),
        (MCGate::new(1, StandardGate::RX), StandardGate::CRX),
        (MCGate::new(1, StandardGate::RY), StandardGate::CRY),
        (MCGate::new(1, StandardGate::RZ), StandardGate::CRZ),
    ];

    for (mc_gate, expected) in cases {
        let expected_controls = expected.num_ctrl_qubits();
        let expected_qubits = expected.num_qubits();
        let expected_params = expected.num_params();
        let canonical = Instruction::McGate(Box::new(mc_gate)).canonicalize_form();

        let Instruction::Standard(actual) = canonical else {
            panic!("expected standard gate {expected:?}");
        };
        assert_eq!(actual, expected);
        assert_eq!(actual.num_ctrl_qubits(), expected_controls);
        assert_eq!(actual.num_qubits(), expected_qubits);
        assert_eq!(actual.num_params(), expected_params);
    }
}

#[test]
fn canonicalize_form_keeps_non_promotable_mc_gate_forms() {
    let cases = [
        MCGate::new(3, StandardGate::X),
        MCGate::new(1, StandardGate::H),
        MCGate::new(1, StandardGate::RXX),
    ];

    for mc_gate in cases {
        let expected_controls = mc_gate.num_ctrl_qubits();
        let expected_qubits = mc_gate.num_qubits();
        let expected_params = mc_gate.num_params();
        let canonical = Instruction::McGate(Box::new(mc_gate)).canonicalize_form();

        let Instruction::McGate(actual) = canonical else {
            panic!("expected MCGate");
        };
        assert_eq!(actual.num_ctrl_qubits(), expected_controls);
        assert_eq!(actual.num_qubits(), expected_qubits);
        assert_eq!(actual.num_params(), expected_params);
    }
}

#[test]
fn control_on_mc_gate_counts_inherent_base_controls_once() {
    let instruction = Instruction::McGate(Box::new(MCGate::new(1, StandardGate::CX)));

    let controlled = instruction.control(1).unwrap();

    let Instruction::McGate(actual) = controlled else {
        panic!("expected MCGate");
    };
    assert_eq!(actual.base_gate(), &StandardGate::X);
    assert_eq!(actual.num_ctrl_qubits(), 3);
    assert_eq!(actual.num_qubits(), 4);
}

#[test]
fn gate_arity_reports_fixed_gate_like_instructions() {
    let unitary = UnitaryGate::new("custom", 2, 3);
    let circuit_gate = CircuitGate::new("composite", FrozenCircuit::new(Circuit::new(4))).unwrap();

    let cases = [
        (Instruction::Standard(StandardGate::H), Some((1, 0))),
        (Instruction::Standard(StandardGate::RX), Some((1, 1))),
        (Instruction::Standard(StandardGate::GPhase), Some((0, 1))),
        (
            Instruction::McGate(Box::new(MCGate::new(2, StandardGate::RZ))),
            Some((3, 1)),
        ),
        (Instruction::UnitaryGate(Box::new(unitary)), Some((2, 3))),
        (
            Instruction::CircuitGate(Box::new(circuit_gate)),
            Some((4, 0)),
        ),
        (Instruction::Directive(Directive::Measure), Some((1, 0))),
        (Instruction::Directive(Directive::Reset), Some((1, 0))),
        (Instruction::Delay, Some((1, 1))),
    ];

    for (instruction, expected) in cases {
        assert_eq!(instruction.gate_arity(), expected);
    }
}

#[test]
fn gate_arity_excludes_variable_or_contextual_instructions() {
    assert_eq!(
        Instruction::Directive(Directive::Barrier).gate_arity(),
        None
    );
    assert_eq!(
        Instruction::ClassicalControl(ClassicalControlOp::Break).gate_arity(),
        None
    );
}

#[test]
fn classical_control_instruction_is_non_unitary_and_not_controllable() {
    let instruction = Instruction::ClassicalControl(ClassicalControlOp::Continue);

    assert!(instruction.matrix(&[]).is_none());
    assert!(instruction.inverse(&[]).is_none());
    assert!(instruction.control(1).is_none());
    assert_eq!(instruction.to_string(), "continue");
}

#[test]
fn classical_control_op_converts_to_instruction() {
    let instruction: Instruction = ClassicalControlOp::Break.into();

    assert!(matches!(
        instruction,
        Instruction::ClassicalControl(ClassicalControlOp::Break)
    ));
}

#[test]
fn payload_accessors_match_predicates_for_every_variant() {
    let circuit_id = CircuitId::new();
    let measured = ClassicalValue::new(circuit_id, 0, ClassicalType::Bit);
    let circuit_gate = CircuitGate::new("composite", FrozenCircuit::new(Circuit::new(1))).unwrap();

    let cases = [
        Instruction::Standard(StandardGate::H),
        Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
        Instruction::UnitaryGate(Box::new(UnitaryGate::new("oracle", 1, 0))),
        Instruction::CircuitGate(Box::new(circuit_gate)),
        Instruction::Directive(Directive::Barrier),
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result: measured }),
        Instruction::ClassicalControl(ClassicalControlOp::Break),
        Instruction::Delay,
    ];

    for instruction in cases {
        assert_eq!(
            instruction.standard_gate().is_some(),
            instruction.is_standard()
        );
        assert_eq!(instruction.mc_gate().is_some(), instruction.is_mcgate());
        assert_eq!(
            instruction.unitary_gate().is_some(),
            instruction.is_unitary()
        );
        assert_eq!(
            instruction.circuit_gate().is_some(),
            instruction.is_circuit_gate()
        );
        assert_eq!(
            instruction.directive().is_some(),
            instruction.is_directive()
        );
        assert_eq!(
            instruction.classical_data().is_some(),
            instruction.is_classical_data()
        );
        assert_eq!(
            instruction.classical_control().is_some(),
            instruction.is_classical_control()
        );
    }
}

#[test]
fn payload_accessors_return_the_stored_payload() {
    let circuit_id = CircuitId::new();
    let measured = ClassicalValue::new(circuit_id, 0, ClassicalType::Bit);
    let target = ClassicalVar::new(circuit_id, 0, ClassicalType::Bit);

    let mc = Instruction::McGate(Box::new(MCGate::new(2, StandardGate::RZ)));
    let mc_gate = mc.mc_gate().unwrap();
    assert_eq!(mc_gate.num_ctrl_qubits(), 2);
    assert_eq!(mc_gate.base_gate(), &StandardGate::RZ);

    let unitary = Instruction::UnitaryGate(Box::new(UnitaryGate::new("oracle", 2, 1)));
    let unitary_gate = unitary.unitary_gate().unwrap();
    assert_eq!(unitary_gate.label(), "oracle");
    assert_eq!(unitary_gate.num_qubits(), 2);

    let circuit_gate = CircuitGate::new("composite", FrozenCircuit::new(Circuit::new(1))).unwrap();
    let circuit = Instruction::CircuitGate(Box::new(circuit_gate));
    assert_eq!(circuit.circuit_gate().unwrap().name(), "composite");

    let directive = Instruction::Directive(Directive::Measure);
    assert_eq!(directive.directive(), Some(Directive::Measure));

    let store = Instruction::ClassicalData(ClassicalDataOp::Store {
        target,
        value: ClassicalExpr::value(measured),
    });
    assert_eq!(store.classical_data().unwrap().target(), Some(target));

    let control = Instruction::ClassicalControl(ClassicalControlOp::Continue);
    assert!(matches!(
        control.classical_control(),
        Some(ClassicalControlOp::Continue)
    ));
}
