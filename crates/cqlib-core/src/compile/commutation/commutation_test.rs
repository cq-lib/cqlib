// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::checker::{Commutation, CommutationChecker, CommutationConfig, CommutationResult};
use crate::circuit::gate::FrozenCircuit;
use crate::circuit::{
    Circuit, CircuitGate, CircuitId, ClassicalControlOp, ClassicalDataOp, ClassicalType,
    ClassicalValue, Directive, Instruction, Parameter, Qubit, StandardGate, UnitaryGate,
};
use ndarray::Array2;
use num_complex::Complex64;
use std::f64::consts::{FRAC_PI_2, PI};

fn numeric_params(gate: StandardGate) -> Vec<Parameter> {
    match gate.num_params() {
        0 => vec![],
        1 => vec![Parameter::from(0.731)],
        2 => vec![Parameter::from(0.731), Parameter::from(-1.127)],
        3 => vec![
            Parameter::from(0.731),
            Parameter::from(-1.127),
            Parameter::from(0.419),
        ],
        count => panic!("unexpected standard-gate parameter count {count}"),
    }
}

fn ordered_qargs(width: usize) -> Vec<Vec<Qubit>> {
    let qubits = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    match width {
        0 => vec![vec![]],
        1 => qubits.iter().map(|&qubit| vec![qubit]).collect(),
        2 => qubits
            .iter()
            .flat_map(|&first| {
                qubits
                    .iter()
                    .copied()
                    .filter(move |&second| second != first)
                    .map(move |second| vec![first, second])
            })
            .collect(),
        3 => vec![
            vec![qubits[0], qubits[1], qubits[2]],
            vec![qubits[0], qubits[2], qubits[1]],
            vec![qubits[1], qubits[0], qubits[2]],
            vec![qubits[1], qubits[2], qubits[0]],
            vec![qubits[2], qubits[0], qubits[1]],
            vec![qubits[2], qubits[1], qubits[0]],
        ],
        _ => vec![],
    }
}

fn algebra_only_checker() -> CommutationChecker {
    CommutationChecker::with_config(CommutationConfig {
        enable_rule_oracle: false,
        enable_matrix_fallback: false,
        ..CommutationConfig::default()
    })
}

fn assert_exact(result: CommutationResult) {
    assert_eq!(result, Some(Commutation::Exact));
}

fn assert_pi_phase(result: CommutationResult) {
    let Some(Commutation::UpToGlobalPhase(phase)) = result else {
        panic!("expected global phase commutation");
    };
    assert!((phase.evaluate(&None).unwrap() - PI).abs() < 1e-10);
}

#[test]
fn circuit_backed_unitary_matrix_fallback_respects_targets_and_parameters() {
    for order in [vec![0, 1], vec![1, 0], vec![7, 2], vec![9, 2, 7]] {
        let mut numeric_order = order.clone();
        numeric_order.sort_unstable();
        let qubits: Vec<Qubit> = [5, 1, 8][..order.len()]
            .iter()
            .copied()
            .map(Qubit::new)
            .collect();
        for active in 0..order.len() {
            for parameterized in [false, true] {
                let mut body =
                    Circuit::from_qubits(order.iter().copied().map(Qubit::new).collect()).unwrap();
                body.rx(
                    Qubit::new(numeric_order[active]),
                    if parameterized {
                        Parameter::symbol("theta")
                    } else {
                        Parameter::from(0.7)
                    },
                )
                .unwrap();
                body.set_global_phase(Parameter::from(0.19));
                let instruction = Instruction::UnitaryGate(Box::new(
                    UnitaryGate::new("rx_one_port", order.len() as u16, u16::from(parameterized))
                        .with_circuit(std::sync::Arc::new(FrozenCircuit::new(body)))
                        .unwrap(),
                ));
                let params = if parameterized {
                    vec![Parameter::from(0.7)]
                } else {
                    vec![]
                };
                let z = Instruction::Standard(StandardGate::Z);
                for checker in [
                    CommutationChecker::builtin(),
                    CommutationChecker::with_config(CommutationConfig {
                        enable_rule_oracle: false,
                        ..CommutationConfig::default()
                    }),
                ] {
                    for (position, &qubit) in qubits.iter().enumerate() {
                        for result in [
                            checker.check(&instruction, &qubits, &params, &z, &[qubit], &[]),
                            checker.check(&z, &[qubit], &[], &instruction, &qubits, &params),
                        ] {
                            if position == active {
                                assert!(result.is_none(), "order={order:?}, active={active}");
                            } else {
                                assert_exact(result);
                            }
                        }
                    }
                }
                if parameterized {
                    assert!(
                        super::matrix::matrix_commutation(
                            &instruction,
                            &qubits,
                            &[Parameter::symbol("unresolved")],
                            &z,
                            &[qubits[active]],
                            &[],
                            4,
                        )
                        .is_none()
                    );
                }
                assert!(
                    super::matrix::matrix_commutation(
                        &instruction,
                        &qubits,
                        &params,
                        &z,
                        &[qubits[active]],
                        &[],
                        order.len() - 1,
                    )
                    .is_none()
                );
            }
        }
    }
}

#[test]
fn circuit_backed_unitary_matrix_fallback_preserves_global_phase_proof() {
    let mut body = Circuit::new(2);
    body.x(Qubit::new(0)).unwrap();
    body.set_global_phase(Parameter::from(0.19));
    let instruction = Instruction::UnitaryGate(Box::new(
        UnitaryGate::new("x_first", 2, 0)
            .with_circuit(std::sync::Arc::new(FrozenCircuit::new(body)))
            .unwrap(),
    ));
    let qubits = [Qubit::new(5), Qubit::new(1)];
    let z = Instruction::Standard(StandardGate::Z);
    let checker = CommutationChecker::builtin();
    assert_pi_phase(checker.check(&instruction, &qubits, &[], &z, &[qubits[0]], &[]));
    assert_pi_phase(checker.check(&z, &[qubits[0]], &[], &instruction, &qubits, &[]));
    assert_exact(checker.check(&instruction, &qubits, &[], &z, &[qubits[1]], &[]));
}

#[test]
fn circuit_gate_operation_matrix_does_not_certify_false_commutation() {
    let mut body = Circuit::new(2);
    body.rx(Qubit::new(0), Parameter::from(0.37)).unwrap();
    let instruction = body.to_gate("rx_first").unwrap();
    let qubits = [Qubit::new(0), Qubit::new(1)];
    let checker = CommutationChecker::with_config(CommutationConfig {
        enable_rule_oracle: false,
        ..CommutationConfig::default()
    });
    assert!(
        checker
            .check(
                &instruction,
                &qubits,
                &[],
                &Instruction::Standard(StandardGate::Z),
                &[qubits[0]],
                &[],
            )
            .is_none()
    );
}

#[test]
fn circuit_gate_matrix_fallback_respects_targets_parameters_and_phase() {
    for order in [[0, 1], [1, 0], [7, 2]] {
        for parameterized in [false, true] {
            let mut body = Circuit::from_qubits(order.map(Qubit::new).to_vec()).unwrap();
            body.rx(
                Qubit::new(order[0]),
                if parameterized {
                    Parameter::symbol("theta")
                } else {
                    Parameter::from(0.37)
                },
            )
            .unwrap();
            let instruction = body.to_gate("rx_first").unwrap();
            let params = if parameterized {
                vec![Parameter::from(0.37)]
            } else {
                vec![]
            };
            let qubits = [Qubit::new(5), Qubit::new(1)];
            for checker in [
                CommutationChecker::builtin(),
                CommutationChecker::with_config(CommutationConfig {
                    enable_rule_oracle: false,
                    ..CommutationConfig::default()
                }),
            ] {
                for (position, &qubit) in qubits.iter().enumerate() {
                    let z = Instruction::Standard(StandardGate::Z);
                    for result in [
                        checker.check(&instruction, &qubits, &params, &z, &[qubit], &[]),
                        checker.check(&z, &[qubit], &[], &instruction, &qubits, &params),
                    ] {
                        if position == 0 {
                            assert!(result.is_none());
                        } else {
                            assert_exact(result);
                        }
                    }
                }
            }
            if parameterized {
                assert!(
                    super::matrix::matrix_commutation(
                        &instruction,
                        &qubits,
                        &[Parameter::symbol("unresolved")],
                        &Instruction::Standard(StandardGate::Z),
                        &[qubits[0]],
                        &[],
                        4,
                    )
                    .is_none()
                );
            }
        }
    }
    let mut body = Circuit::new(2);
    body.x(Qubit::new(0)).unwrap();
    let x = body.to_gate("x_first").unwrap();
    let z = Instruction::Standard(StandardGate::Z);
    let checker = CommutationChecker::builtin();
    assert_pi_phase(checker.check(
        &x,
        &[Qubit::new(0), Qubit::new(1)],
        &[],
        &z,
        &[Qubit::new(0)],
        &[],
    ));
    assert_pi_phase(checker.check(
        &z,
        &[Qubit::new(0)],
        &[],
        &x,
        &[Qubit::new(0), Qubit::new(1)],
        &[],
    ));
}

#[test]
fn identity_commutes_exactly() {
    let checker = CommutationChecker::builtin();
    let result = checker.check(
        &Instruction::Standard(StandardGate::I),
        &[Qubit::new(0)],
        &[],
        &Instruction::Standard(StandardGate::H),
        &[Qubit::new(0)],
        &[],
    );

    assert_exact(result);
}

#[test]
fn disjoint_operations_commute_exactly() {
    let checker = CommutationChecker::builtin();
    let result = checker.check(
        &Instruction::Standard(StandardGate::H),
        &[Qubit::new(0)],
        &[],
        &Instruction::Standard(StandardGate::X),
        &[Qubit::new(1)],
        &[],
    );

    assert_exact(result);
}

#[test]
fn circuit_gate_same_application_respects_signature_order() {
    let mut definition = Circuit::new(1);
    definition
        .rx(Qubit::new(0), Parameter::symbol("theta"))
        .unwrap();
    definition
        .ry(Qubit::new(0), Parameter::symbol("phi"))
        .unwrap();

    let ordered = Instruction::CircuitGate(Box::new(
        CircuitGate::with_signature(
            "G",
            FrozenCircuit::new(definition.clone()),
            ["theta".to_string(), "phi".to_string()],
        )
        .unwrap(),
    ));
    let reversed = Instruction::CircuitGate(Box::new(
        CircuitGate::with_signature(
            "G",
            FrozenCircuit::new(definition),
            ["phi".to_string(), "theta".to_string()],
        )
        .unwrap(),
    ));
    let checker = algebra_only_checker();
    let qubits = [Qubit::new(0)];
    let params = [Parameter::from(0.37), Parameter::from(-0.81)];

    assert_exact(checker.check(&ordered, &qubits, &params, &ordered, &qubits, &params));
    assert!(
        checker
            .check(&ordered, &qubits, &params, &reversed, &qubits, &params)
            .is_none()
    );
}

#[test]
fn same_application_shortcut_excludes_side_effecting_operations() {
    let checker = algebra_only_checker();
    let qubits = [Qubit::new(0)];
    let result = ClassicalValue::new(CircuitId::new(), 0, ClassicalType::Bit);
    let cases = [
        (Instruction::Directive(Directive::Measure), vec![]),
        (Instruction::Directive(Directive::Reset), vec![]),
        (Instruction::Delay, vec![Parameter::from(1.0)]),
        (
            Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }),
            vec![],
        ),
        (
            Instruction::ClassicalControl(ClassicalControlOp::Break),
            vec![],
        ),
    ];

    for (instruction, params) in cases {
        assert!(
            checker
                .check(
                    &instruction,
                    &qubits,
                    &params,
                    &instruction,
                    &qubits,
                    &params,
                )
                .is_none(),
            "side-effecting instruction {instruction:?} used the same-application shortcut"
        );
    }
}

#[test]
fn symbolic_rz_family_commutes_exactly() {
    let checker = CommutationChecker::builtin();
    let result = checker.check(
        &Instruction::Standard(StandardGate::RZ),
        &[Qubit::new(0)],
        &[Parameter::symbol("a")],
        &Instruction::Standard(StandardGate::RZ),
        &[Qubit::new(0)],
        &[Parameter::symbol("b")],
    );

    assert_exact(result);
}

#[test]
fn symbolic_rx_ry_is_not_proven_commuting() {
    let checker = CommutationChecker::builtin();
    let result = checker.check(
        &Instruction::Standard(StandardGate::RX),
        &[Qubit::new(0)],
        &[Parameter::symbol("a")],
        &Instruction::Standard(StandardGate::RY),
        &[Qubit::new(0)],
        &[Parameter::symbol("b")],
    );

    assert!(result.is_none());
}

#[test]
fn controlled_rule_commutes_cx_with_rz_on_control() {
    let checker = CommutationChecker::builtin();
    let result = checker.check(
        &Instruction::Standard(StandardGate::CX),
        &[Qubit::new(0), Qubit::new(1)],
        &[],
        &Instruction::Standard(StandardGate::RZ),
        &[Qubit::new(0)],
        &[Parameter::symbol("theta")],
    );

    assert_exact(result);
}

#[test]
fn algebraic_checker_proves_controlled_axis_without_rule_or_matrix() {
    let checker = algebra_only_checker();
    let result = checker.check(
        &Instruction::Standard(StandardGate::CX),
        &[Qubit::new(0), Qubit::new(1)],
        &[],
        &Instruction::Standard(StandardGate::RZ),
        &[Qubit::new(0)],
        &[Parameter::symbol("theta")],
    );

    assert_exact(result);
}

#[test]
fn pauli_interactions_use_symplectic_commutation() {
    let checker = CommutationChecker::builtin();
    let result = checker.check(
        &Instruction::Standard(StandardGate::RXX),
        &[Qubit::new(0), Qubit::new(1)],
        &[Parameter::symbol("a")],
        &Instruction::Standard(StandardGate::RZZ),
        &[Qubit::new(0), Qubit::new(1)],
        &[Parameter::symbol("b")],
    );

    assert_exact(result);
}

#[test]
fn matrix_fallback_returns_global_phase_for_x_z() {
    let checker = algebra_only_checker();
    let result = checker.check(
        &Instruction::Standard(StandardGate::X),
        &[Qubit::new(0)],
        &[],
        &Instruction::Standard(StandardGate::Z),
        &[Qubit::new(0)],
        &[],
    );

    assert_pi_phase(result);
}

#[test]
fn h_x_does_not_commute_even_up_to_global_phase() {
    let checker = CommutationChecker::builtin();
    let result = checker.check(
        &Instruction::Standard(StandardGate::H),
        &[Qubit::new(0)],
        &[],
        &Instruction::Standard(StandardGate::X),
        &[Qubit::new(0)],
        &[],
    );

    assert!(result.is_none());
}

#[test]
fn pi_rotations_use_pauli_product_phase() {
    let checker = algebra_only_checker();
    let result = checker.check(
        &Instruction::Standard(StandardGate::RX),
        &[Qubit::new(0)],
        &[Parameter::from(PI)],
        &Instruction::Standard(StandardGate::RZ),
        &[Qubit::new(0)],
        &[Parameter::from(PI)],
    );

    assert_pi_phase(result);
}

#[test]
fn symbolic_same_axis_rotations_commute_algebraically() {
    let checker = algebra_only_checker();
    let result = checker.check(
        &Instruction::Standard(StandardGate::RXX),
        &[Qubit::new(0), Qubit::new(1)],
        &[Parameter::symbol("a")],
        &Instruction::Standard(StandardGate::RXX),
        &[Qubit::new(0), Qubit::new(1)],
        &[Parameter::symbol("b")],
    );

    assert_exact(result);
}

#[test]
fn symbolic_anti_commuting_rotations_are_conservative() {
    let checker = algebra_only_checker();
    let result = checker.check(
        &Instruction::Standard(StandardGate::RXX),
        &[Qubit::new(0), Qubit::new(1)],
        &[Parameter::symbol("a")],
        &Instruction::Standard(StandardGate::RZX),
        &[Qubit::new(0), Qubit::new(1)],
        &[Parameter::symbol("b")],
    );

    assert!(result.is_none());
}

#[test]
fn rxy_same_planar_axis_commutes_algebraically() {
    let checker = algebra_only_checker();
    let phi = Parameter::symbol("phi");
    let result = checker.check(
        &Instruction::Standard(StandardGate::RXY),
        &[Qubit::new(0)],
        &[Parameter::symbol("a"), phi.clone()],
        &Instruction::Standard(StandardGate::RXY),
        &[Qubit::new(0)],
        &[Parameter::symbol("b"), phi],
    );

    assert_exact(result);
}

#[test]
fn rxy_pi_orthogonal_axes_returns_global_phase() {
    let checker = algebra_only_checker();
    let result = checker.check(
        &Instruction::Standard(StandardGate::RXY),
        &[Qubit::new(0)],
        &[Parameter::from(PI), Parameter::from(0.0)],
        &Instruction::Standard(StandardGate::RXY),
        &[Qubit::new(0)],
        &[Parameter::from(PI), Parameter::from(FRAC_PI_2)],
    );

    assert_pi_phase(result);
}

#[test]
fn controlled_axis_target_rotation_commutes_without_rule_or_matrix() {
    let checker = algebra_only_checker();
    let result = checker.check(
        &Instruction::Standard(StandardGate::CRX),
        &[Qubit::new(0), Qubit::new(1)],
        &[Parameter::symbol("a")],
        &Instruction::Standard(StandardGate::RX),
        &[Qubit::new(1)],
        &[Parameter::symbol("b")],
    );

    assert_exact(result);
}

#[test]
fn controlled_axis_wrong_target_axis_is_conservative() {
    let checker = algebra_only_checker();
    let result = checker.check(
        &Instruction::Standard(StandardGate::CX),
        &[Qubit::new(0), Qubit::new(1)],
        &[],
        &Instruction::Standard(StandardGate::RZ),
        &[Qubit::new(1)],
        &[Parameter::symbol("theta")],
    );

    assert!(result.is_none());
}

#[test]
fn ccx_control_and_target_axis_commute_algebraically() {
    let checker = algebra_only_checker();
    let control_result = checker.check(
        &Instruction::Standard(StandardGate::CCX),
        &[Qubit::new(0), Qubit::new(1), Qubit::new(2)],
        &[],
        &Instruction::Standard(StandardGate::RZ),
        &[Qubit::new(1)],
        &[Parameter::symbol("theta")],
    );
    let target_result = checker.check(
        &Instruction::Standard(StandardGate::CCX),
        &[Qubit::new(0), Qubit::new(1), Qubit::new(2)],
        &[],
        &Instruction::Standard(StandardGate::RX),
        &[Qubit::new(2)],
        &[Parameter::symbol("theta")],
    );

    assert_exact(control_result);
    assert_exact(target_result);
}

#[test]
fn fsim_family_commutes_on_same_pair() {
    let checker = algebra_only_checker();
    let result = checker.check(
        &Instruction::Standard(StandardGate::FSIM),
        &[Qubit::new(0), Qubit::new(1)],
        &[Parameter::symbol("a"), Parameter::symbol("b")],
        &Instruction::Standard(StandardGate::FSIM),
        &[Qubit::new(1), Qubit::new(0)],
        &[Parameter::symbol("c"), Parameter::symbol("d")],
    );

    assert_exact(result);
}

#[test]
fn fsim_commutes_with_symmetric_diagonal_family() {
    let checker = algebra_only_checker();
    let result = checker.check(
        &Instruction::Standard(StandardGate::FSIM),
        &[Qubit::new(0), Qubit::new(1)],
        &[Parameter::symbol("a"), Parameter::symbol("b")],
        &Instruction::Standard(StandardGate::RZZ),
        &[Qubit::new(1), Qubit::new(0)],
        &[Parameter::symbol("theta")],
    );

    assert_exact(result);
}

#[test]
fn fsim_with_single_rz_is_conservative() {
    let checker = algebra_only_checker();
    let result = checker.check(
        &Instruction::Standard(StandardGate::FSIM),
        &[Qubit::new(0), Qubit::new(1)],
        &[Parameter::symbol("a"), Parameter::symbol("b")],
        &Instruction::Standard(StandardGate::RZ),
        &[Qubit::new(0)],
        &[Parameter::symbol("theta")],
    );

    assert!(result.is_none());
}

#[test]
fn swap_commutes_with_symmetric_pauli_interaction_but_not_rzx() {
    let checker = algebra_only_checker();
    let symmetric = checker.check(
        &Instruction::Standard(StandardGate::SWAP),
        &[Qubit::new(0), Qubit::new(1)],
        &[],
        &Instruction::Standard(StandardGate::RXX),
        &[Qubit::new(1), Qubit::new(0)],
        &[Parameter::symbol("theta")],
    );
    let asymmetric = checker.check(
        &Instruction::Standard(StandardGate::SWAP),
        &[Qubit::new(0), Qubit::new(1)],
        &[],
        &Instruction::Standard(StandardGate::RZX),
        &[Qubit::new(1), Qubit::new(0)],
        &[Parameter::symbol("theta")],
    );

    assert_exact(symmetric);
    assert!(asymmetric.is_none());
}

#[test]
fn symbolic_u_with_x_is_conservative() {
    let checker = CommutationChecker::builtin();
    let result = checker.check(
        &Instruction::Standard(StandardGate::U),
        &[Qubit::new(0)],
        &[
            Parameter::symbol("theta"),
            Parameter::symbol("phi"),
            Parameter::symbol("lambda"),
        ],
        &Instruction::Standard(StandardGate::X),
        &[Qubit::new(0)],
        &[],
    );

    assert!(result.is_none());
}

#[test]
fn matrix_fallback_respects_max_qubits() {
    let checker = CommutationChecker::with_config(CommutationConfig {
        max_matrix_qubits: 4,
        ..CommutationConfig::default()
    });
    let identity = Array2::<Complex64>::eye(32);
    let wide_identity = UnitaryGate::new("WideIdentity", 5, 0)
        .with_matrix(identity)
        .unwrap();
    let result = checker.check(
        &Instruction::UnitaryGate(Box::new(wide_identity)),
        &[
            Qubit::new(0),
            Qubit::new(1),
            Qubit::new(2),
            Qubit::new(3),
            Qubit::new(4),
        ],
        &[],
        &Instruction::Standard(StandardGate::H),
        &[Qubit::new(0)],
        &[],
    );

    assert!(result.is_none());
}

#[test]
fn every_numeric_standard_gate_commutation_claim_matches_direct_matrix_check() {
    use super::matrix::matrix_commutation;

    let checker = algebra_only_checker();
    for &lhs_gate in StandardGate::all() {
        let lhs_inst = Instruction::Standard(lhs_gate);
        let lhs_params = numeric_params(lhs_gate);
        for lhs_qargs in ordered_qargs(lhs_gate.num_qubits()) {
            for &rhs_gate in StandardGate::all() {
                // The direct matrix fallback intentionally does not represent
                // zero-qubit GPhase operations; their scalar commutation is
                // covered separately by the cheap checker path.
                if lhs_gate == StandardGate::GPhase || rhs_gate == StandardGate::GPhase {
                    continue;
                }
                let rhs_inst = Instruction::Standard(rhs_gate);
                let rhs_params = numeric_params(rhs_gate);
                for rhs_qargs in ordered_qargs(rhs_gate.num_qubits()) {
                    let Some(claim) = checker.check(
                        &lhs_inst,
                        &lhs_qargs,
                        &lhs_params,
                        &rhs_inst,
                        &rhs_qargs,
                        &rhs_params,
                    ) else {
                        continue;
                    };
                    let matrix = matrix_commutation(
                        &lhs_inst,
                        &lhs_qargs,
                        &lhs_params,
                        &rhs_inst,
                        &rhs_qargs,
                        &rhs_params,
                        3,
                    );
                    assert!(
                        matrix.is_some(),
                        "false commutation claim {claim:?}: {lhs_gate:?}{lhs_qargs:?} vs {rhs_gate:?}{rhs_qargs:?}"
                    );
                    assert_eq!(
                        claim.is_exact(),
                        matrix.as_ref().is_some_and(Commutation::is_exact),
                        "wrong commutation phase class: {lhs_gate:?}{lhs_qargs:?} vs {rhs_gate:?}{rhs_qargs:?}"
                    );
                }
            }
        }
    }
}
