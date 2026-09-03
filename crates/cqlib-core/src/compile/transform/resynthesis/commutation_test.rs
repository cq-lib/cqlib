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
    CircuitParam, Instruction, Operation, Parameter, Qubit, StandardGate, ValueClassicalControlOp,
    ValueInstruction, ValueOperation, circuit_param::ParameterValue,
};
use crate::compile::commutation::CommutationConfig;
use smallvec::{SmallVec, smallvec};

fn checker() -> CachedCommutation {
    CachedCommutation::new(CommutationConfig {
        enable_rule_oracle: true,
        enable_matrix_fallback: false,
        max_matrix_qubits: 4,
    })
}

fn op(gate: StandardGate, qubits: &[Qubit], params: &[f64]) -> Operation {
    Operation {
        instruction: Instruction::Standard(gate),
        qubits: qubits.iter().copied().collect(),
        params: params.iter().copied().map(CircuitParam::Fixed).collect(),
        label: None,
    }
}

fn view<'a>(
    order: usize,
    operation: &'a Operation,
    params: SmallVec<[Parameter; 3]>,
) -> OperationView<'a> {
    OperationView {
        order,
        operation,
        params,
    }
}

#[test]
fn disjoint_source_operations_commute() {
    let h = op(StandardGate::H, &[Qubit::new(0)], &[]);
    let x = op(StandardGate::X, &[Qubit::new(1)], &[]);
    let h_view = view(0, &h, smallvec![]);
    let x_view = view(1, &x, smallvec![]);

    assert!(checker().commute_ops(&h_view, &x_view));
}

#[test]
fn same_operation_commutes_without_cache_entry() {
    let h = op(StandardGate::H, &[Qubit::new(0)], &[]);
    let h_view = view(7, &h, smallvec![]);
    let mut checker = checker();

    assert!(checker.commute_ops(&h_view, &h_view));
    assert_eq!(checker.cache_len(), 0);
}

#[test]
fn symbolic_same_axis_rotations_commute() {
    let first = op(StandardGate::RZ, &[Qubit::new(0)], &[]);
    let second = op(StandardGate::RZ, &[Qubit::new(0)], &[]);
    let first_view = view(0, &first, smallvec![Parameter::symbol("a")]);
    let second_view = view(1, &second, smallvec![Parameter::symbol("b")]);

    assert!(checker().commute_ops(&first_view, &second_view));
}

#[test]
fn same_qubit_h_and_x_do_not_commute() {
    let h = op(StandardGate::H, &[Qubit::new(0)], &[]);
    let x = op(StandardGate::X, &[Qubit::new(0)], &[]);
    let h_view = view(0, &h, smallvec![]);
    let x_view = view(1, &x, smallvec![]);

    assert!(!checker().commute_ops(&h_view, &x_view));
}

#[test]
fn reversed_source_query_reuses_normalized_cache_key() {
    let h = op(StandardGate::H, &[Qubit::new(0)], &[]);
    let x = op(StandardGate::X, &[Qubit::new(1)], &[]);
    let h_view = view(3, &h, smallvec![]);
    let x_view = view(9, &x, smallvec![]);
    let mut checker = checker();

    assert!(checker.commute_ops(&h_view, &x_view));
    assert_eq!(checker.cache_len(), 1);
    assert!(checker.commute_ops(&x_view, &h_view));
    assert_eq!(checker.cache_len(), 1);
}

#[test]
fn empty_crossed_or_replacements_are_safe() {
    let op = op(StandardGate::H, &[Qubit::new(0)], &[]);
    let op_view = view(0, &op, smallvec![]);
    let replacement = ValueOperation::from_standard(StandardGate::X, [Qubit::new(0)], []);
    let checker = checker();

    assert!(checker.replacements_commute_with_crossed(&[], std::slice::from_ref(&replacement)));
    assert!(checker.replacements_commute_with_crossed(&[&op_view], &[]));
}

#[test]
fn disjoint_replacement_commutes_with_crossed_operation() {
    let crossed = op(StandardGate::H, &[Qubit::new(0)], &[]);
    let crossed_view = view(0, &crossed, smallvec![]);
    let replacement = ValueOperation::from_standard(StandardGate::X, [Qubit::new(1)], []);

    assert!(checker().replacements_commute_with_crossed(&[&crossed_view], &[replacement]));
}

#[test]
fn shared_non_commuting_replacement_is_rejected() {
    let crossed = op(StandardGate::H, &[Qubit::new(0)], &[]);
    let crossed_view = view(0, &crossed, smallvec![]);
    let replacement = ValueOperation::from_standard(StandardGate::X, [Qubit::new(0)], []);

    assert!(!checker().replacements_commute_with_crossed(&[&crossed_view], &[replacement]));
}

#[test]
fn classical_control_replacement_is_rejected() {
    let crossed = op(StandardGate::H, &[Qubit::new(0)], &[]);
    let crossed_view = view(0, &crossed, smallvec![]);
    let replacement = ValueOperation {
        instruction: ValueInstruction::ClassicalControl(ValueClassicalControlOp::Break),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![],
        label: None,
    };

    assert!(!checker().replacements_commute_with_crossed(&[&crossed_view], &[replacement]));
}

#[test]
fn symbolic_parameters_do_not_panic() {
    let crossed = op(StandardGate::RZ, &[Qubit::new(0)], &[]);
    let crossed_view = view(0, &crossed, smallvec![Parameter::symbol("theta")]);
    let replacement = ValueOperation::from_standard(
        StandardGate::RZ,
        [Qubit::new(0)],
        [ParameterValue::Param(Parameter::symbol("phi"))],
    );

    assert!(checker().replacements_commute_with_crossed(&[&crossed_view], &[replacement]));
}

#[test]
fn fixed_replacement_params_bridge_to_checker() {
    let crossed = op(StandardGate::RZ, &[Qubit::new(0)], &[0.25]);
    let crossed_view = view(0, &crossed, smallvec![Parameter::from(0.25)]);
    let replacement = ValueOperation::from_standard(
        StandardGate::RZ,
        [Qubit::new(0)],
        [ParameterValue::Fixed(0.5)],
    );

    assert!(checker().replacements_commute_with_crossed(&[&crossed_view], &[replacement]));
}
