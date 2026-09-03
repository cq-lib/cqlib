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

use super::CommutativeCancellation;
use crate::circuit::circuit_to_matrix::circuit_to_matrix;
use crate::circuit::test_utils::assert_matrix_approx_eq;
use crate::circuit::{
    Circuit, ClassicalControlOp, ClassicalExpr, Instruction, Parameter, Qubit, StandardGate,
};
use crate::compile::transform::{
    ResolvedTransform, RewriteEdits, TransformOutcome, TransformerTestExt,
};

const EPSILON: f64 = 1e-9;

fn run(circuit: &Circuit) -> ResolvedTransform {
    CommutativeCancellation::new()
        .transform_resolved(circuit, None)
        .unwrap()
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

#[test]
fn reports_exact_deletions_around_preserved_commuting_operations() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.x(q0).unwrap();
    circuit.h(q1).unwrap();
    circuit.x(q0).unwrap();

    let (outcome, edits) = CommutativeCancellation::new()
        .transform_with_rewrite_edits(&circuit)
        .unwrap();
    let TransformOutcome::Changed(changed) = outcome else {
        panic!("expected cancellation to change the circuit");
    };
    assert_eq!(standard_ops(&changed), vec![StandardGate::H]);
    let RewriteEdits::Linear {
        old_len,
        new_len,
        replacements,
    } = edits
    else {
        panic!("expected exact linear edits");
    };
    assert_eq!((old_len, new_len), (3, 1));
    assert_eq!(replacements.len(), 2);
    assert_eq!(replacements[0].old, 0..1);
    assert_eq!(replacements[0].new, 0..0);
    assert_eq!(replacements[1].old, 2..3);
    assert_eq!(replacements[1].new, 1..1);
}

/// Exact unitary comparison including global phase.
fn assert_exact_unitary(actual: &Circuit, expected: &Circuit) {
    let actual_matrix = circuit_to_matrix(actual, None).unwrap();
    let expected_matrix = circuit_to_matrix(expected, None).unwrap();
    assert_matrix_approx_eq(&actual_matrix, &expected_matrix, EPSILON);
}

#[test]
fn cancels_cx_pair_across_wide_window() {
    let (q0, q1, q2) = (Qubit::new(0), Qubit::new(1), Qubit::new(2));
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q2).unwrap();
    // More than the knowledge rewriter's 16-operation scan window; every
    // gate sits on a disjoint wire and trivially commutes with the CX pair.
    for index in 0..17 {
        let angle = 0.01 * f64::from(index);
        circuit.rx(q1, angle).unwrap();
        circuit.rz(q1, angle).unwrap();
    }
    circuit.cx(q0, q2).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert_eq!(result.circuit.operations().len(), 34);
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::CX));
    assert_exact_unitary(&result.circuit, &circuit);
}

#[test]
fn cancels_rotations_on_control_wire_between_cx_pair() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.rz(q0, 0.3).unwrap();
    circuit.x(q1).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert_eq!(
        standard_ops(&result.circuit),
        vec![StandardGate::RZ, StandardGate::X]
    );
    assert_exact_unitary(&result.circuit, &circuit);
}

#[test]
fn cancels_self_inverse_gate_pairs() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.cz(q0, q1).unwrap();
    circuit.cz(q0, q1).unwrap();
    circuit.h(q0).unwrap();
    circuit.h(q0).unwrap();
    circuit.y(q0).unwrap();
    circuit.y(q0).unwrap();
    circuit.x(q1).unwrap();
    circuit.x(q1).unwrap();
    circuit.z(q1).unwrap();
    circuit.z(q1).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert_exact_unitary(&result.circuit, &circuit);
}

#[test]
fn keeps_first_of_odd_cx_run() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::CX]);
    assert_exact_unitary(&result.circuit, &circuit);
}

#[test]
fn cancels_pair_with_mismatched_wire_set_indices() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::H]);
    assert_exact_unitary(&result.circuit, &circuit);
}

#[test]
fn cancels_cz_pair_with_swapped_qargs() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.cz(q0, q1).unwrap();
    circuit.cz(q1, q0).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert_exact_unitary(&result.circuit, &circuit);
}

#[test]
fn does_not_cancel_reversed_cx_or_cy() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut cx_circuit = Circuit::new(2);
    cx_circuit.cx(q0, q1).unwrap();
    cx_circuit.cx(q1, q0).unwrap();

    let cx_result = run(&cx_circuit);

    assert!(!cx_result.changed);
    assert_eq!(cx_result.circuit, cx_circuit);

    let mut cy_circuit = Circuit::new(2);
    cy_circuit.cy(q0, q1).unwrap();
    cy_circuit.cy(q1, q0).unwrap();

    let cy_result = run(&cy_circuit);

    assert!(!cy_result.changed);
    assert_eq!(cy_result.circuit, cy_circuit);
}

#[test]
fn labeled_operations_are_hard_barriers() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit
        .append(
            Instruction::Standard(StandardGate::H),
            [q0],
            std::iter::empty(),
            Some("keep"),
        )
        .unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = run(&circuit);

    assert!(!result.changed);
    assert_eq!(result.circuit, circuit);
}

#[test]
fn does_not_cancel_across_non_commuting_gate() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.x(q0).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = run(&circuit);

    assert!(!result.changed);
    assert_eq!(result.circuit, circuit);
}

#[test]
fn symbolic_parameter_gate_can_join_sets_without_blocking() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.rz(q0, theta).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::RZ]);
    assert!(result.circuit.uses_symbol("theta"));
}

#[test]
fn non_gate_instructions_are_hard_barriers() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));

    let mut measure_circuit = Circuit::new(2);
    measure_circuit.h(q0).unwrap();
    measure_circuit.measure(q0).unwrap();
    measure_circuit.h(q0).unwrap();
    let measure_result = run(&measure_circuit);
    assert!(!measure_result.changed);
    assert_eq!(measure_result.circuit, measure_circuit);

    let mut reset_circuit = Circuit::new(2);
    reset_circuit.h(q0).unwrap();
    reset_circuit.reset(q0).unwrap();
    reset_circuit.h(q0).unwrap();
    let reset_result = run(&reset_circuit);
    assert!(!reset_result.changed);
    assert_eq!(reset_result.circuit, reset_circuit);

    let mut delay_circuit = Circuit::new(2);
    delay_circuit.h(q0).unwrap();
    delay_circuit.delay(q0, 100.0.into()).unwrap();
    delay_circuit.h(q0).unwrap();
    let delay_result = run(&delay_circuit);
    assert!(!delay_result.changed);
    assert_eq!(delay_result.circuit, delay_circuit);

    let mut barrier_circuit = Circuit::new(2);
    barrier_circuit.cx(q0, q1).unwrap();
    barrier_circuit.barrier(vec![q0, q1]).unwrap();
    barrier_circuit.cx(q0, q1).unwrap();
    let barrier_result = run(&barrier_circuit);
    assert!(!barrier_result.changed);
    assert_eq!(barrier_result.circuit, barrier_circuit);
}

#[test]
fn recurses_into_control_flow_bodies() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.x(q0).unwrap();
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.cx(q0, q1)?;
                body.cx(q0, q1)
            },
            |body| {
                body.h(q1)?;
                body.h(q1)
            },
        )
        .unwrap();
    circuit.x(q0).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    // The two X gates outside cannot cancel across the control-flow barrier.
    assert_eq!(result.circuit.operations().len(), 3);
    let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
        &result.circuit.operations()[1].instruction
    else {
        panic!("expected if operation");
    };
    assert!(op.then_body().operations().is_empty());
    assert!(op.else_body().unwrap().operations().is_empty());
}

#[test]
fn control_flow_is_a_scope_barrier() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| body.x(q1))
        .unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = run(&circuit);

    assert!(!result.changed);
    assert_eq!(result.circuit, circuit);
}

#[test]
fn repeated_runs_converge_and_then_report_no_change() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.h(q1).unwrap();
    circuit.h(q1).unwrap();
    circuit.cx(q0, q1).unwrap();

    // Round 1: only the H-H pair shares a cancellation key and is removed.
    let first = run(&circuit);
    assert!(first.changed);
    assert_eq!(
        standard_ops(&first.circuit),
        vec![StandardGate::CX, StandardGate::CX]
    );
    assert!(first.circuit.operations().len() < circuit.operations().len());

    // Round 2: the exposed CX-CX pair is removed.
    let second = run(&first.circuit);
    assert!(second.changed);
    assert!(second.circuit.operations().is_empty());
    assert_exact_unitary(&second.circuit, &circuit);

    // Converged: the pass reports no change and returns the input unchanged.
    let third = run(&second.circuit);
    assert!(!third.changed);
    assert_eq!(third.circuit, second.circuit);
}

#[test]
fn runs_only_delete_operations() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.h(q1).unwrap();
    circuit.h(q1).unwrap();
    circuit.cx(q0, q1).unwrap();

    let mut current = circuit;
    loop {
        let result = run(&current);
        assert!(result.circuit.operations().len() <= current.operations().len());
        assert_eq!(result.changed, result.circuit != current);
        if !result.changed {
            break;
        }
        current = result.circuit;
    }
}

#[test]
fn reports_no_change_and_preserves_circuit_when_nothing_cancels() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.rz(q1, 0.7).unwrap();

    let result = run(&circuit);

    assert!(!result.changed);
    assert_eq!(result.circuit, circuit);
}

#[test]
fn supports_non_contiguous_qubit_ids() {
    let (q1, q3) = (Qubit::new(1), Qubit::new(3));
    let mut circuit = Circuit::from_qubits(vec![q1, q3]).unwrap();
    circuit.cx(q1, q3).unwrap();
    circuit.z(q1).unwrap();
    circuit.cx(q1, q3).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::Z]);
}

#[test]
fn cancels_even_cx_run_completely() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    for _ in 0..4 {
        circuit.cx(q0, q1).unwrap();
    }

    let result = run(&circuit);

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert_exact_unitary(&result.circuit, &circuit);
}

#[test]
fn cancels_cy_pair_in_same_direction() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.cy(q0, q1).unwrap();
    circuit.cy(q0, q1).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert_exact_unitary(&result.circuit, &circuit);
}

#[test]
fn cancels_independent_buckets_in_one_block() {
    let (q0, q1, q2, q3) = (Qubit::new(0), Qubit::new(1), Qubit::new(2), Qubit::new(3));
    let mut circuit = Circuit::new(4);
    circuit.h(q0).unwrap();
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.cz(q2, q3).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.cz(q3, q2).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert_exact_unitary(&result.circuit, &circuit);
}

#[test]
fn keeps_single_cz_of_odd_swapped_run() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.cz(q0, q1).unwrap();
    circuit.cz(q1, q0).unwrap();
    circuit.cz(q0, q1).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert_eq!(result.circuit.operations().len(), 1);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::CZ)
    ));
    assert_exact_unitary(&result.circuit, &circuit);
}

#[test]
fn cancels_x_pair_across_cx_on_target_wire() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.x(q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.x(q1).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert_eq!(standard_ops(&result.circuit), vec![StandardGate::CX]);
    assert_exact_unitary(&result.circuit, &circuit);
}

#[test]
fn does_not_cancel_across_unprovable_symbolic_gate() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    // RX(theta) on the control qubit does not commute with CX for general
    // theta, so the pair must survive.
    circuit.rx(q0, theta).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = run(&circuit);

    assert!(!result.changed);
    assert_eq!(result.circuit, circuit);
}

#[test]
fn does_not_cancel_swap_or_ccx_in_p0_scope() {
    let (q0, q1, q2) = (Qubit::new(0), Qubit::new(1), Qubit::new(2));
    let mut swap_circuit = Circuit::new(2);
    swap_circuit.swap(q0, q1).unwrap();
    swap_circuit.swap(q0, q1).unwrap();
    let swap_result = run(&swap_circuit);
    assert!(!swap_result.changed);
    assert_eq!(swap_result.circuit, swap_circuit);

    let mut ccx_circuit = Circuit::new(3);
    ccx_circuit.ccx(q0, q1, q2).unwrap();
    ccx_circuit.ccx(q0, q1, q2).unwrap();
    let ccx_result = run(&ccx_circuit);
    assert!(!ccx_result.changed);
    assert_eq!(ccx_result.circuit, ccx_circuit);
}

#[test]
fn preserves_existing_global_phase_exactly() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.set_global_phase(Parameter::from(0.375));
    circuit.h(q0).unwrap();
    circuit.h(q0).unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
    assert!(
        result
            .circuit
            .global_phase()
            .provably_equal(&Parameter::from(0.375), 1e-12)
    );
    assert_exact_unitary(&result.circuit, &circuit);
}

#[test]
fn recurses_into_nested_control_flow_bodies() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.x(q0)?;
            body.if_else(
                ClassicalExpr::bool_literal(true),
                |nested| {
                    nested.cx(q0, q1)?;
                    nested.cx(q0, q1)
                },
                |nested| {
                    nested.h(q1)?;
                    nested.h(q1)
                },
            )
        })
        .unwrap();

    let result = run(&circuit);

    assert!(result.changed);
    let Instruction::ClassicalControl(ClassicalControlOp::While(while_op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected while operation");
    };
    assert_eq!(while_op.body().operations().len(), 2);
    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &while_op.body().operations()[1].instruction
    else {
        panic!("expected nested if operation");
    };
    assert!(if_op.then_body().operations().is_empty());
    assert!(if_op.else_body().unwrap().operations().is_empty());
}

#[test]
fn cancels_trivial_bvlike_motif_at_scale() {
    // benchpress trivial_bvlike: an up ladder of CXs, X(target), Z(last
    // control), and the mirrored down ladder cancel to exactly X and Z.
    let num_qubits = 20u32;
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

    let result = run(&circuit);

    assert!(result.changed);
    assert_eq!(result.circuit.operations().len(), 2);
    let first = &result.circuit.operations()[0];
    let second = &result.circuit.operations()[1];
    assert!(matches!(
        first.instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert_eq!(first.qubits.as_slice(), &[target]);
    assert!(matches!(
        second.instruction,
        Instruction::Standard(StandardGate::Z)
    ));
    assert_eq!(second.qubits.as_slice(), &[last_control]);
}

#[test]
fn cancels_long_cx_chain_on_wide_circuit() {
    // 100-qubit line: a CX chain and its mirror. Adjacent chain links do not
    // commute (each link's target is the next link's control), so each run
    // peels only the innermost exposed pair; repeated runs converge to the
    // empty circuit. Doubles as a performance smoke test for wide circuits.
    let num_qubits = 100u32;
    let mut circuit = Circuit::new(num_qubits as usize);
    for index in 0..num_qubits - 1 {
        circuit
            .cx(Qubit::new(index), Qubit::new(index + 1))
            .unwrap();
    }
    for index in (0..num_qubits - 1).rev() {
        circuit
            .cx(Qubit::new(index), Qubit::new(index + 1))
            .unwrap();
    }

    let mut current = circuit;
    let mut rounds = 0;
    loop {
        let result = run(&current);
        assert!(result.circuit.operations().len() <= current.operations().len());
        rounds += 1;
        assert!(rounds <= num_qubits as usize, "did not converge");
        if !result.changed {
            break;
        }
        current = result.circuit;
    }
    assert!(rounds > 1);
    assert!(current.operations().is_empty());
}

#[test]
fn handles_circuit_with_only_barrier_candidates() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut circuit = Circuit::new(2);
    circuit.rz(q0, 0.5).unwrap();
    circuit.rz(q1, 0.25).unwrap();
    circuit.cx(q0, q1).unwrap();

    let result = run(&circuit);

    assert!(!result.changed);
    assert_eq!(result.circuit, circuit);
}
