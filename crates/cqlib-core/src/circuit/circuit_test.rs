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

use crate::circuit::Qubit;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::circuit_param::{CircuitParam, ParameterValue};
use crate::circuit::error::CircuitError;
use crate::circuit::gate::classical_data::ClassicalDataOp;
use crate::circuit::gate::{Directive, Instruction, StandardGate, UnitaryGate};
use crate::circuit::operation::ValueOperation;
use crate::circuit::parameter::Parameter;
use crate::circuit::{
    ClassicalControlOp, ClassicalExpr, ClassicalType, ClassicalValue, ClassicalVar, ControlBody,
    IfOp, Operation, SwitchCase, SwitchOp, ValueClassicalControlOp, ValueControlBody,
    ValueInstruction, WhileOp,
};
use smallvec::smallvec;
use std::collections::HashSet;
use std::f64::consts::PI;

fn control_operation(op: ClassicalControlOp) -> Operation {
    Operation {
        instruction: Instruction::ClassicalControl(op),
        qubits: smallvec![],
        params: smallvec![],
        label: None,
    }
}

#[test]
fn test_circuit_basic_construction() {
    let mut circuit = Circuit::new(3);
    assert_eq!(circuit.num_qubits(), 3);
    assert_eq!(circuit.width(), 3);

    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);

    // Test simple append
    let _ = circuit.h(q0);
    let _ = circuit.cx(q0, q1);
    let _ = circuit.rx(q2, 1.5);

    assert_eq!(circuit.data.len(), 3);

    // Check operation details
    let op0 = &circuit.data[0];
    assert!(matches!(
        op0.instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert_eq!(op0.qubits[0], q0);

    let op1 = &circuit.data[1];
    assert!(matches!(
        op1.instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    let expected: smallvec::SmallVec<[Qubit; 3]> = smallvec![q0, q1];
    assert_eq!(op1.qubits, expected);
}

#[test]
fn test_circuit_qubit_validation() {
    let mut circuit = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q3 = Qubit::new(3); // Does not exist

    let res = circuit.h(q3);
    assert!(matches!(res, Err(CircuitError::QubitNotFound(3))));

    let res = circuit.cx(q0, q3);
    assert!(matches!(res, Err(CircuitError::QubitNotFound(3))));
}

#[test]
fn test_operation_rejects_duplicate_qubits() {
    let mut circuit = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    assert!(matches!(
        circuit.cx(q0, q0),
        Err(CircuitError::DuplicateQubits)
    ));
    assert!(matches!(
        circuit.swap(q1, q1),
        Err(CircuitError::DuplicateQubits)
    ));

    let mut circuit = Circuit::new(3);
    assert!(matches!(
        circuit.ccx(q0, q0, q1),
        Err(CircuitError::DuplicateQubits)
    ));
}

#[test]
fn test_from_qubits_and_add() {
    let q0 = Qubit::new(0);
    let q2 = Qubit::new(2);

    // Non-contiguous qubits
    let mut circuit = Circuit::from_qubits(vec![q0, q2]).unwrap();
    assert_eq!(circuit.num_qubits(), 2);
    assert!(circuit.qubits.contains(&q0));
    assert!(circuit.qubits.contains(&q2));
    assert!(!circuit.qubits.contains(&Qubit::new(1)));

    // Duplicate check
    let res = Circuit::from_qubits(vec![q0, q0]);
    assert!(matches!(res, Err(CircuitError::DuplicateQubits)));

    // Add qubits
    let q1 = Qubit::new(1);
    circuit.add_qubits(vec![q1]).unwrap();
    assert_eq!(circuit.num_qubits(), 3);
    assert!(circuit.qubits.contains(&q1));

    // Add duplicate
    let res = circuit.add_qubits(vec![q0]);
    assert!(matches!(res, Err(CircuitError::DuplicateQubits)));
}

#[test]
fn from_operations_builds_circuit_and_interns_symbolic_parameters() {
    let theta = Parameter::symbol("theta");
    let operations = vec![
        ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::H)),
            qubits: smallvec![Qubit::new(2)],
            params: smallvec![],
            label: Some("prepare".into()),
        },
        ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::Standard(
                StandardGate::RX,
            )),
            qubits: smallvec![Qubit::new(4)],
            params: smallvec![ParameterValue::Param(theta.clone())],
            label: None,
        },
        ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::Standard(
                StandardGate::RZ,
            )),
            qubits: smallvec![Qubit::new(2)],
            params: smallvec![ParameterValue::Fixed(0.25)],
            label: None,
        },
    ];

    let circuit =
        Circuit::from_operations(vec![Qubit::new(2), Qubit::new(4)], operations, None, None)
            .unwrap();

    assert_eq!(circuit.qubits(), vec![Qubit::new(2), Qubit::new(4)]);
    assert_eq!(circuit.operations().len(), 3);
    assert_eq!(circuit.operations()[0].label.as_deref(), Some("prepare"));
    assert_eq!(circuit.parameters().len(), 1);
    assert!(circuit.parameters().contains(&theta));
    assert!(circuit.symbols().contains("theta"));
    assert!(matches!(
        circuit.operations()[1].params.as_slice(),
        [CircuitParam::Index(0)]
    ));
    assert!(matches!(
        circuit.operations()[2].params.as_slice(),
        [CircuitParam::Fixed(value)] if value.to_bits() == 0.25f64.to_bits()
    ));
}

#[test]
fn from_operations_rejects_duplicate_qubit_declarations() {
    let result =
        Circuit::from_operations(vec![Qubit::new(0), Qubit::new(0)], Vec::new(), None, None);

    assert!(matches!(result, Err(CircuitError::DuplicateQubits)));
}

#[test]
fn from_operations_rejects_unknown_operation_qubits() {
    let operations = vec![ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::H)),
        qubits: smallvec![Qubit::new(1)],
        params: smallvec![],
        label: None,
    }];

    let result = Circuit::from_operations(vec![Qubit::new(0)], operations, None, None);

    assert!(matches!(result, Err(CircuitError::QubitNotFound(1))));
}

#[test]
fn from_operations_rejects_duplicate_operation_qubits() {
    let operations = vec![ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::CX)),
        qubits: smallvec![Qubit::new(0), Qubit::new(0)],
        params: smallvec![],
        label: None,
    }];

    let result = Circuit::from_operations(vec![Qubit::new(0)], operations, None, None);

    assert!(matches!(result, Err(CircuitError::DuplicateQubits)));
}

#[test]
fn append_rejects_non_finite_fixed_parameters() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);

    assert!(matches!(
        circuit.rx(q0, f64::NAN),
        Err(CircuitError::InvalidParameterValue(0, value)) if value.is_nan()
    ));
    assert!(matches!(
        circuit.rz(q0, f64::INFINITY),
        Err(CircuitError::InvalidParameterValue(0, value)) if value.is_infinite()
    ));
    assert!(circuit.operations().is_empty());
}

#[test]
fn append_rejects_fixed_instruction_arity_mismatches_without_mutation() {
    let mut circuit = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let theta = Parameter::symbol("theta");

    assert!(matches!(
        circuit.append(Instruction::Standard(StandardGate::CX), [q0], [], None),
        Err(CircuitError::QubitCountMismatch {
            expected: 2,
            actual: 1
        })
    ));
    assert!(matches!(
        circuit.append(Instruction::Standard(StandardGate::H), [q0, q1], [], None,),
        Err(CircuitError::QubitCountMismatch {
            expected: 1,
            actual: 2
        })
    ));
    assert!(matches!(
        circuit.append(
            Instruction::Standard(StandardGate::RX),
            [q0],
            std::iter::empty(),
            None,
        ),
        Err(CircuitError::ParameterCountMismatch {
            expected: 1,
            actual: 0
        })
    ));
    assert!(matches!(
        circuit.append(
            Instruction::Standard(StandardGate::RX),
            [q0],
            [ParameterValue::Param(theta), ParameterValue::Fixed(0.5),],
            None,
        ),
        Err(CircuitError::ParameterCountMismatch {
            expected: 1,
            actual: 2
        })
    ));

    assert!(circuit.operations().is_empty());
    assert!(circuit.parameters().is_empty());
    assert!(circuit.symbols().is_empty());
}

#[test]
fn append_rejects_circuit_gate_parameter_mismatch_immediately() {
    let theta = Parameter::symbol("theta");
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), theta).unwrap();
    let gate = inner.to_gate("rx").unwrap();

    let mut circuit = Circuit::new(1);
    let result = circuit.append(gate, [Qubit::new(0)], [], None);

    assert!(matches!(
        result,
        Err(CircuitError::ParameterCountMismatch {
            expected: 1,
            actual: 0
        })
    ));
    assert!(circuit.operations().is_empty());
}

#[test]
fn remove_operation_deletes_top_level_gate() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    circuit.h(q0).unwrap();
    circuit.x(q0).unwrap();
    circuit.z(q0).unwrap();

    let removed = circuit.remove_operation(1).unwrap();

    assert!(matches!(
        removed.instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert_eq!(circuit.operations().len(), 2);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::Standard(StandardGate::Z)
    ));
    circuit.validate().unwrap();
}

#[test]
fn remove_operation_rejects_out_of_bounds_without_mutation() {
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();

    let result = circuit.remove_operation(1);

    assert!(matches!(
        result,
        Err(CircuitError::OperationIndexOutOfBounds { index: 1, len: 1 })
    ));
    assert_eq!(circuit.operations().len(), 1);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn remove_operation_deletes_legacy_measure_directive_without_classical_compaction() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    circuit
        .append(
            Instruction::Directive(Directive::Measure),
            [q0],
            std::iter::empty(),
            None,
        )
        .unwrap();
    circuit.h(q0).unwrap();

    let removed = circuit.remove_operation(0).unwrap();

    assert!(matches!(
        removed.instruction,
        Instruction::Directive(Directive::Measure)
    ));
    assert!(circuit.classical_values().is_empty());
    assert_eq!(circuit.operations().len(), 1);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn remove_operation_compacts_unused_measurement_values_and_rewrites_reads() {
    let mut circuit = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let target = circuit.var(ClassicalType::Bit);
    circuit.measure(q0).unwrap();
    let second = circuit.measure(q1).unwrap();
    circuit.store(target, second.expr()).unwrap();

    let removed = circuit.remove_operation(0).unwrap();

    assert!(matches!(
        removed.instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert_eq!(circuit.classical_values(), &[ClassicalType::Bit]);
    let Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }) =
        &circuit.operations()[0].instruction
    else {
        panic!("remaining first operation should be a measurement");
    };
    assert_eq!(result.index(), 0);
    let Instruction::ClassicalData(ClassicalDataOp::Store { value, .. }) =
        &circuit.operations()[1].instruction
    else {
        panic!("remaining second operation should be a store");
    };
    assert!(
        value
            .values()
            .contains(&ClassicalValue::new(circuit.id(), 0, ClassicalType::Bit))
    );
    circuit.validate().unwrap();
}

#[test]
fn remove_operation_rejects_measurement_still_in_use_without_mutation() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    let target = circuit.var(ClassicalType::Bit);
    let measured = circuit.measure(q0).unwrap();
    circuit.store(target, measured.expr()).unwrap();

    let result = circuit.remove_operation(0);

    assert!(matches!(
        result,
        Err(CircuitError::ClassicalValueStillInUse {
            index: 0,
            context
        }) if context == "operation 1"
    ));
    assert_eq!(circuit.operations().len(), 2);
    assert_eq!(circuit.classical_values(), &[ClassicalType::Bit]);
    circuit.validate().unwrap();
}

#[test]
fn remove_operation_deletes_control_flow_as_whole_and_compacts_body_values() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    circuit.h(q0).unwrap();
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.measure(q0)?;
            Ok(())
        })
        .unwrap();
    circuit.measure(q0).unwrap();

    let removed = circuit.remove_operation(1).unwrap();

    assert!(matches!(
        removed.instruction,
        Instruction::ClassicalControl(ClassicalControlOp::If(_))
    ));
    assert_eq!(circuit.operations().len(), 2);
    assert_eq!(circuit.classical_values(), &[ClassicalType::Bit]);
    let Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }) =
        &circuit.operations()[1].instruction
    else {
        panic!("remaining trailing operation should be a measurement");
    };
    assert_eq!(result.index(), 0);
    circuit.validate().unwrap();
}

#[test]
fn remove_operation_deletes_circuit_gate_as_outer_operation() {
    let q0 = Qubit::new(0);
    let mut inner = Circuit::new(1);
    inner.h(q0).unwrap();
    let gate = inner.to_gate("inner").unwrap();

    let mut circuit = Circuit::new(1);
    circuit
        .append(gate, [q0], std::iter::empty(), None)
        .unwrap();
    circuit.x(q0).unwrap();

    let removed = circuit.remove_operation(0).unwrap();

    assert!(matches!(removed.instruction, Instruction::CircuitGate(_)));
    assert_eq!(circuit.operations().len(), 1);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
}

#[test]
fn remove_operation_keeps_unused_symbolic_parameters() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    let theta = Parameter::symbol("theta");
    circuit.rx(q0, theta.clone()).unwrap();
    circuit.h(q0).unwrap();

    let removed = circuit.remove_operation(0).unwrap();

    assert!(matches!(
        removed.instruction,
        Instruction::Standard(StandardGate::RX)
    ));
    assert_eq!(circuit.operations().len(), 1);
    assert!(circuit.parameters().contains(&theta));
    assert!(circuit.symbols().contains("theta"));
    circuit.index(0).unwrap();
    circuit.validate().unwrap();
}

#[test]
fn remove_operations_empty_input_does_not_mutate() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    circuit.h(q0).unwrap();
    circuit.measure(q0).unwrap();

    let removed = circuit.remove_operations([]).unwrap();

    assert!(removed.is_empty());
    assert_eq!(circuit.operations().len(), 2);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert_eq!(circuit.classical_values(), &[ClassicalType::Bit]);
    circuit.validate().unwrap();
}

#[test]
fn remove_operations_deduplicates_indices() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    circuit.h(q0).unwrap();
    circuit.x(q0).unwrap();
    circuit.z(q0).unwrap();

    let removed = circuit.remove_operations([1, 1, 1]).unwrap();

    assert_eq!(removed.len(), 1);
    assert!(matches!(
        removed[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert_eq!(circuit.operations().len(), 2);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::Standard(StandardGate::Z)
    ));
    circuit.validate().unwrap();
}

#[test]
fn remove_operations_deletes_multiple_gates_in_original_index_order() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    circuit.h(q0).unwrap();
    circuit.x(q0).unwrap();
    circuit.z(q0).unwrap();
    circuit.y(q0).unwrap();

    let removed = circuit.remove_operations([3, 1]).unwrap();

    assert_eq!(removed.len(), 2);
    assert!(matches!(
        removed[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert!(matches!(
        removed[1].instruction,
        Instruction::Standard(StandardGate::Y)
    ));
    assert_eq!(circuit.operations().len(), 2);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::Standard(StandardGate::Z)
    ));
    circuit.validate().unwrap();
}

#[test]
fn remove_operations_rejects_out_of_bounds_without_mutation() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    circuit.h(q0).unwrap();
    circuit.x(q0).unwrap();

    let result = circuit.remove_operations([0, 2]);

    assert!(matches!(
        result,
        Err(CircuitError::OperationIndexOutOfBounds { index: 2, len: 2 })
    ));
    assert_eq!(circuit.operations().len(), 2);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    circuit.validate().unwrap();
}

#[test]
fn remove_operations_deletes_measurement_with_same_batch_store() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    let target = circuit.var(ClassicalType::Bit);
    let measured = circuit.measure(q0).unwrap();
    circuit.store(target, measured.expr()).unwrap();

    let removed = circuit.remove_operations([0, 1]).unwrap();

    assert_eq!(removed.len(), 2);
    assert!(matches!(
        removed[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert!(matches!(
        removed[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));
    assert!(circuit.operations().is_empty());
    assert!(circuit.classical_values().is_empty());
    circuit.validate().unwrap();
}

#[test]
fn remove_operations_rejects_measurement_still_used_by_remaining_operation() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    let target = circuit.var(ClassicalType::Bit);
    let measured = circuit.measure(q0).unwrap();
    circuit.store(target, measured.expr()).unwrap();

    let result = circuit.remove_operations([0]);

    assert!(matches!(
        result,
        Err(CircuitError::ClassicalValueStillInUse {
            index: 0,
            context
        }) if context == "operation 1"
    ));
    assert_eq!(circuit.operations().len(), 2);
    assert_eq!(circuit.classical_values(), &[ClassicalType::Bit]);
    circuit.validate().unwrap();
}

#[test]
fn remove_operations_deletes_measurement_with_same_batch_control_flow() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    let measured = circuit.measure(q0).unwrap();
    let condition = measured.expr().to_bool().unwrap();
    circuit
        .if_(condition, |body| {
            body.x(q0)?;
            Ok(())
        })
        .unwrap();

    let removed = circuit.remove_operations([0, 1]).unwrap();

    assert_eq!(removed.len(), 2);
    assert!(matches!(
        removed[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert!(matches!(
        removed[1].instruction,
        Instruction::ClassicalControl(ClassicalControlOp::If(_))
    ));
    assert!(circuit.operations().is_empty());
    assert!(circuit.classical_values().is_empty());
    circuit.validate().unwrap();
}

#[test]
fn remove_operations_compacts_measurement_values_after_batch_delete() {
    let mut circuit = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let target = circuit.var(ClassicalType::Bit);
    circuit.h(q0).unwrap();
    let first = circuit.measure(q0).unwrap();
    circuit.store(target, first.expr()).unwrap();
    circuit.x(q1).unwrap();
    circuit.measure(q1).unwrap();

    let removed = circuit.remove_operations([1, 2]).unwrap();

    assert_eq!(removed.len(), 2);
    assert!(matches!(
        removed[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert!(matches!(
        removed[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));
    assert_eq!(circuit.operations().len(), 3);
    assert_eq!(circuit.classical_values(), &[ClassicalType::Bit]);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    let Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }) =
        &circuit.operations()[2].instruction
    else {
        panic!("remaining measurement should be compacted");
    };
    assert_eq!(result.index(), 0);
    circuit.validate().unwrap();
}

#[test]
fn from_operations_rejects_non_finite_fixed_parameters() {
    let operations = vec![ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::RX)),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![ParameterValue::Fixed(f64::NAN)],
        label: None,
    }];

    let result = Circuit::from_operations(vec![Qubit::new(0)], operations, None, None);

    assert!(matches!(
        result,
        Err(CircuitError::InvalidParameterValue(0, value)) if value.is_nan()
    ));
}

#[test]
fn from_operations_rejects_non_finite_body_fixed_parameters() {
    let body = ValueControlBody::new(vec![ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::RX)),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![ParameterValue::Fixed(f64::NAN)],
        label: None,
    }]);
    let operations = vec![ValueOperation {
        instruction: ValueInstruction::ClassicalControl(ValueClassicalControlOp::If {
            condition: ClassicalExpr::bool_literal(true),
            then_body: body,
            else_body: None,
        }),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![],
        label: None,
    }];

    let result = Circuit::from_operations(vec![Qubit::new(0)], operations, None, None);

    assert!(matches!(
        result,
        Err(CircuitError::InvalidParameterValue(0, value)) if value.is_nan()
    ));
}

#[test]
fn from_operations_interns_value_control_body_parameters() {
    let theta = Parameter::symbol("theta");
    let body = ValueControlBody::new(vec![ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::RX)),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![ParameterValue::Param(theta.clone())],
        label: None,
    }]);

    let circuit = Circuit::from_operations(
        vec![Qubit::new(0)],
        vec![ValueOperation {
            instruction: ValueInstruction::ClassicalControl(ValueClassicalControlOp::If {
                condition: ClassicalExpr::bool_literal(true),
                then_body: body,
                else_body: None,
            }),
            qubits: smallvec![Qubit::new(0)],
            params: smallvec![],
            label: None,
        }],
        None,
        None,
    )
    .unwrap();

    let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
        &circuit.operations()[0].instruction
    else {
        panic!("expected if operation");
    };
    let body_op = &op.then_body().operations()[0];
    assert!(matches!(
        body_op.params.as_slice(),
        [CircuitParam::Index(0)]
    ));
    assert_eq!(circuit.parameters().get_index(0), Some(&theta));
}

#[test]
fn from_operations_rejects_storage_control_wrapped_as_value_instruction() {
    let body = ControlBody::new(vec![Operation {
        instruction: Instruction::Standard(StandardGate::RX),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![CircuitParam::Index(0)],
        label: None,
    }]);
    let op = IfOp::new(ClassicalExpr::bool_literal(true), body, None).unwrap();

    let result = Circuit::from_operations(
        vec![Qubit::new(0)],
        vec![ValueOperation {
            instruction: ValueInstruction::Instruction(Instruction::ClassicalControl(
                ClassicalControlOp::If(op),
            )),
            qubits: smallvec![Qubit::new(0)],
            params: smallvec![],
            label: None,
        }],
        None,
        None,
    );

    assert!(matches!(result, Err(CircuitError::InvalidOperation(_))));
}

#[test]
fn map_param_keeps_constants_out_of_parameter_table() {
    let mut circuit = Circuit::new(1);

    let param = circuit
        .map_param(Parameter::from(-0.0))
        .expect("constant parameter should map");

    assert!(matches!(
        param,
        CircuitParam::Fixed(value) if value.to_bits() == 0.0f64.to_bits()
    ));
    assert!(circuit.parameters().is_empty());
}

#[test]
fn map_param_interns_symbolic_parameters() {
    let mut circuit = Circuit::new(1);
    let theta = Parameter::symbol("theta");

    let first = circuit
        .map_param(theta.clone())
        .expect("symbolic parameter should map");
    let second = circuit
        .map_param(theta.clone())
        .expect("symbolic parameter should map");

    assert!(matches!(first, CircuitParam::Index(0)));
    assert!(matches!(second, CircuitParam::Index(0)));
    assert_eq!(circuit.parameters().len(), 1);
    assert!(circuit.parameters().contains(&theta));
    assert!(circuit.symbols().contains("theta"));
}

#[test]
fn resolve_parameter_and_parameter_value_report_missing_index() {
    let circuit = Circuit::new(1);
    let missing = CircuitParam::Index(3);

    assert!(matches!(
        circuit.resolve_parameter(&missing),
        Err(CircuitError::InvalidParameterIndex(3))
    ));
    assert!(matches!(
        circuit.parameter_value(&missing),
        Err(CircuitError::InvalidParameterIndex(3))
    ));
}

#[test]
fn parameter_value_resolves_fixed_and_indexed_values() {
    let mut circuit = Circuit::new(1);
    let theta = Parameter::symbol("theta");
    let indexed = circuit.map_param(theta.clone()).unwrap();

    assert!(matches!(
        circuit.parameter_value(&CircuitParam::Fixed(0.5)).unwrap(),
        ParameterValue::Fixed(value) if value.to_bits() == 0.5f64.to_bits()
    ));
    assert!(matches!(
        circuit.parameter_value(&indexed).unwrap(),
        ParameterValue::Param(param) if param == theta
    ));
}

#[test]
fn test_multi_control_logic() {
    let mut circuit = Circuit::new(4);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let q3 = Qubit::new(3);

    // X -> CX (1 control)
    circuit
        .multi_control(StandardGate::X, [q0], [q1], [])
        .unwrap();
    let op = &circuit.data[0];
    assert!(matches!(
        op.instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    let expected: smallvec::SmallVec<[Qubit; 3]> = smallvec![q0, q1];
    assert_eq!(op.qubits, expected);

    // X -> CCX (2 controls)
    circuit
        .multi_control(StandardGate::X, [q0, q1], [q2], [])
        .unwrap();
    let op = &circuit.data[1];
    assert!(matches!(
        op.instruction,
        Instruction::Standard(StandardGate::CCX)
    ));
    let expected: smallvec::SmallVec<[Qubit; 3]> = smallvec![q0, q1, q2];
    assert_eq!(op.qubits, expected);

    // X -> MCX (3 controls, extended)
    circuit
        .multi_control(StandardGate::X, [q0, q1, q2], [q3], [])
        .unwrap();
    let op = &circuit.data[2];
    if let Instruction::McGate(ext) = &op.instruction {
        assert_eq!(ext.base_gate(), &StandardGate::X);
        assert_eq!(ext.num_ctrl_qubits(), 3);
    } else {
        panic!("Expected Extended instruction for 3-control X");
    }
}

#[test]
fn test_parametric_operations() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);

    // Fixed parameter
    circuit.rx(q0, PI).unwrap();
    if let CircuitParam::Fixed(val) = circuit.data[0].params[0] {
        assert!((val - PI).abs() < 1e-10);
    } else {
        panic!("Expected fixed parameter");
    }

    // Symbolic parameter
    let theta = Parameter::symbol("theta");
    circuit.rz(q0, theta.clone()).unwrap();

    // Check it was interned
    assert_eq!(circuit.parameters.len(), 1);
    assert!(circuit.symbols.contains("theta"));

    if let CircuitParam::Index(idx) = circuit.data[1].params[0] {
        assert_eq!(idx, 0);
    } else {
        panic!("Expected indexed parameter");
    }
}

#[test]
fn test_inverse_basic() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);

    circuit.h(q0).unwrap();
    circuit.s(q0).unwrap(); // S -> Sdg

    let inv_circuit = circuit.inverse().unwrap();

    assert_eq!(inv_circuit.data.len(), 2);

    // Order should be reversed: Sdg then H
    let op0 = &inv_circuit.data[0];
    assert!(matches!(
        op0.instruction,
        Instruction::Standard(StandardGate::SDG)
    ));

    let op1 = &inv_circuit.data[1];
    assert!(matches!(
        op1.instruction,
        Instruction::Standard(StandardGate::H)
    )); // H inv is H
}

#[test]
fn test_inverse_parametric() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    let theta = Parameter::symbol("theta");

    // RX(theta)
    circuit.rx(q0, theta.clone()).unwrap();

    let inv_circuit = circuit.inverse().unwrap();
    let op = &inv_circuit.data[0];

    // Should be RX(-theta)
    assert!(matches!(
        op.instruction,
        Instruction::Standard(StandardGate::RX)
    ));

    if let CircuitParam::Index(idx) = op.params[0] {
        let _ = &inv_circuit.parameters[idx as usize];
    } else {
        panic!("Expected indexed parameter for inverted rotation");
    }
}

#[test]
fn test_to_gate_and_circuit_gate() {
    // 1. Define a sub-circuit: H(q0), CX(q0, q1)
    let mut sub = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    sub.h(q0).unwrap();
    sub.cx(q0, q1).unwrap();

    // 2. Convert to Gate
    let bell_gate_inst = sub.to_gate("bell").unwrap();

    // 3. Use in main circuit
    let mut main = Circuit::new(3);
    // Apply bell gate to q1, q2
    // to_gate produced an instruction, we need to append it
    // Note: append expects params. bell_gate has 0 params.
    main.append(
        bell_gate_inst.clone(),
        [Qubit::new(1), Qubit::new(2)],
        [],
        Some("my_bell"),
    )
    .unwrap();

    assert_eq!(main.data.len(), 1);
    if let Instruction::CircuitGate(gate) = &main.data[0].instruction {
        assert_eq!(gate.name.as_ref(), "bell");
        assert_eq!(gate.num_qubits(), 2);
        assert_eq!(gate.num_params(), 0);
    } else {
        panic!("Expected Circuit instruction");
    }
}

#[test]
fn test_assign_parameters() {
    use std::collections::HashMap;

    let mut circuit = Circuit::new(2);
    let q0 = Qubit::new(0);
    let a = Parameter::symbol("a");
    let b = Parameter::symbol("b");

    // rx(a) q0
    circuit.rx(q0, a.clone()).unwrap();
    // rz(a + b) q0
    circuit.rz(q0, a.clone() + b.clone()).unwrap();

    // Case 1: Partial assignment a = 1.0
    // Expected:
    // rx(1.0) q0 -> Fixed(1.0)
    // rz(1.0 + b) q0 -> Index(new_param)
    let mut bindings = HashMap::new();
    bindings.insert("a", 1.0);

    let assigned_circuit = circuit.assign_parameters(&Some(bindings)).unwrap();

    assert_eq!(assigned_circuit.data.len(), 2);

    // Check first op: rx(1.0)
    if let CircuitParam::Fixed(val) = assigned_circuit.data[0].params[0] {
        assert!((val - 1.0).abs() < 1e-10);
    } else {
        panic!(
            "Expected Fixed(1.0) for rx, got {:?}",
            assigned_circuit.data[0].params[0]
        );
    }

    // Check second op: rz(1.0 + b)
    if let CircuitParam::Index(idx) = assigned_circuit.data[1].params[0] {
        let param = &assigned_circuit.parameters[idx as usize];
        let symbols = param.get_symbols();
        assert!(symbols.contains("b"));
        assert!(!symbols.contains("a"));
    } else {
        panic!(
            "Expected Index for rz, got {:?}",
            assigned_circuit.data[1].params[0]
        );
    }

    // Case 2: Full assignment a = 1.0, b = 2.0
    let mut bindings = HashMap::new();
    bindings.insert("a", 1.0);
    bindings.insert("b", 2.0);

    let assigned_circuit = circuit.assign_parameters(&Some(bindings)).unwrap();

    // Check second op: rz(1.0 + 2.0) -> rz(3.0) -> Fixed(3.0)
    if let CircuitParam::Fixed(val) = assigned_circuit.data[1].params[0] {
        assert!((val - 3.0).abs() < 1e-10);
    } else {
        panic!(
            "Expected Fixed(3.0) for rz, got {:?}",
            assigned_circuit.data[1].params[0]
        );
    }
}

#[test]
fn test_parameterized_unitary_appends_fixed_and_symbolic_params() {
    let mut circuit = Circuit::new(1);
    let theta = Parameter::symbol("theta");
    let gate = UnitaryGate::new("CustomU", 1, 2);

    circuit
        .unitary_with_params(
            gate,
            vec![Qubit::new(0)],
            vec![ParameterValue::Fixed(0.25), ParameterValue::Param(theta)],
        )
        .unwrap();

    assert_eq!(circuit.data.len(), 1);
    assert!(matches!(
        circuit.data[0].instruction,
        Instruction::UnitaryGate(_)
    ));
    assert!(matches!(
        circuit.data[0].params[0],
        CircuitParam::Fixed(0.25)
    ));
    assert!(matches!(circuit.data[0].params[1], CircuitParam::Index(_)));
    assert_eq!(circuit.parameters().len(), 1);
    assert!(circuit.symbols().contains("theta"));
}

#[test]
fn test_parameterized_unitary_validates_param_count() {
    let mut circuit = Circuit::new(1);
    let gate = UnitaryGate::new("CustomU", 1, 2);

    let err = circuit
        .unitary_with_params(gate.clone(), vec![Qubit::new(0)], vec![0.1.into()])
        .unwrap_err();
    assert!(matches!(
        err,
        CircuitError::ParameterCountMismatch {
            expected: 2,
            actual: 1
        }
    ));

    let err = circuit.unitary(gate, vec![Qubit::new(0)]).unwrap_err();
    assert!(matches!(
        err,
        CircuitError::ParameterCountMismatch {
            expected: 2,
            actual: 0
        }
    ));
}

#[test]
fn test_decompose() {
    use std::collections::HashMap;

    let mut inner = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let theta = Parameter::symbol("theta");
    let beta = Parameter::symbol("beta");

    inner.h(q0).unwrap();
    inner.rx(q0, theta).unwrap();
    inner.rz(q1, beta.clone() + 1.0).unwrap();

    let gate = inner.to_gate("c1").unwrap();

    let mut outer = Circuit::new(2);
    let qa = Qubit::new(0);
    let qb = Qubit::new(1);
    let gamma = Parameter::symbol("gamma");
    let delta = Parameter::symbol("delta");

    outer
        .append(
            gate,
            vec![qb, qa],
            vec![
                ParameterValue::Param(gamma.clone()),
                ParameterValue::Param(gamma.clone() + delta.clone()),
            ],
            None,
        )
        .unwrap();

    // 3. Decompose
    let decomposed = outer.decompose().unwrap();

    // 4. Verify
    assert_eq!(decomposed.num_qubits(), 2);
    assert_eq!(
        decomposed.symbols().get_index(0),
        Some(&"gamma".to_string())
    );
    assert_eq!(
        decomposed.symbols().get_index(1),
        Some(&"delta".to_string())
    );

    assert_eq!(decomposed.data.len(), 3);

    assert!(matches!(
        decomposed.data[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert_eq!(decomposed.data[0].qubits[0], qb);

    assert!(matches!(
        decomposed.data[1].instruction,
        Instruction::Standard(StandardGate::RX)
    ));
    assert_eq!(decomposed.data[1].qubits[0], qb);
    if let CircuitParam::Index(idx) = decomposed.data[1].params[0] {
        let p = &decomposed.parameters[idx as usize];
        // Should evaluate to same as gamma with bindings
        let mut bind = HashMap::new();
        bind.insert("gamma", 2.0);
        bind.insert("delta", 3.0);
        assert_eq!(p.evaluate(&Some(bind)).unwrap(), 2.0);
    } else {
        panic!("Expected Index param for RX");
    }

    // Op 3: RZ(gamma+delta+1, qA)
    assert!(matches!(
        decomposed.data[2].instruction,
        Instruction::Standard(StandardGate::RZ)
    ));
    assert_eq!(decomposed.data[2].qubits[0], qa);
    if let CircuitParam::Index(idx) = decomposed.data[2].params[0] {
        let p = &decomposed.parameters[idx as usize];
        // gamma=2, delta=3 -> 2+3+1 = 6.0
        let mut bind = HashMap::new();
        bind.insert("gamma", 2.0);
        bind.insert("delta", 3.0);
        assert_eq!(p.evaluate(&Some(bind)).unwrap(), 6.0);
    } else {
        panic!("Expected Index param for RZ");
    }
}

#[test]
fn test_decompose_nested() {
    let mut l1 = Circuit::new(1);
    l1.h(Qubit::new(0)).unwrap();
    let g1 = l1.to_gate("g1").unwrap();

    let mut l2 = Circuit::new(1);
    let theta = Parameter::symbol("theta");
    l2.rx(Qubit::new(0), theta.clone()).unwrap();
    l2.append(g1, [Qubit::new(0)], [], None).unwrap();
    let g2 = l2.to_gate("g2").unwrap();

    let mut top = Circuit::new(1);
    let phi = Parameter::symbol("phi");
    top.append(g2, [Qubit::new(0)], [ParameterValue::Param(phi)], None)
        .unwrap();

    let flat = top.decompose().unwrap();
    assert_eq!(flat.data.len(), 2);

    assert!(matches!(
        flat.data[0].instruction,
        Instruction::Standard(StandardGate::RX)
    ));
    if let CircuitParam::Index(idx) = flat.data[0].params[0] {
        let p = &flat.parameters[idx as usize];
        let set1: HashSet<String> = ["phi".to_string()].into_iter().collect();
        assert_eq!(p.get_symbols(), set1);
    }

    assert!(matches!(
        flat.data[1].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn test_classical_variable_allocation_and_data_ops() {
    let mut circuit = Circuit::new(3);
    let bit = circuit.var(ClassicalType::Bit);
    let bool_var = circuit.var(ClassicalType::Bool);
    let uint = circuit.var(ClassicalType::uint(8).unwrap());
    let bit_vec = circuit.var(ClassicalType::bit_vec(3).unwrap());

    assert_eq!(bit.id(), 0);
    assert_eq!(bit.ty(), ClassicalType::Bit);
    assert_eq!(bool_var.id(), 1);
    assert_eq!(bool_var.ty(), ClassicalType::Bool);
    assert_eq!(uint.id(), 2);
    assert_eq!(uint.ty(), ClassicalType::uint(8).unwrap());
    assert_eq!(bit_vec.id(), 3);
    assert_eq!(bit_vec.ty(), ClassicalType::bit_vec(3).unwrap());
    assert_eq!(ClassicalType::uint(0), None);
    assert_eq!(ClassicalType::bit_vec(0), None);

    let measured_bit = circuit.measure_into(Qubit::new(0), bit).unwrap();
    assert_eq!(measured_bit.ty(), ClassicalType::Bit);
    assert_eq!(measured_bit.width(), 1);
    assert_eq!(measured_bit.qubits(), &[Qubit::new(0)]);
    let measured_bits = circuit
        .measure_bits_into([Qubit::new(0), Qubit::new(1), Qubit::new(2)], bit_vec)
        .unwrap();
    assert_eq!(measured_bits.ty(), ClassicalType::bit_vec(3).unwrap());
    assert_eq!(measured_bits.width(), 3);
    assert_eq!(
        measured_bits.qubits(),
        &[Qubit::new(0), Qubit::new(1), Qubit::new(2)]
    );
    circuit
        .store(uint, ClassicalExpr::uint_literal(8, 3).unwrap())
        .unwrap();

    assert!(matches!(
        circuit.data[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }) if result == measured_bit.value()
    ));
    assert!(matches!(
        circuit.data[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { target, .. }) if target == bit
    ));
    assert!(matches!(
        circuit.data[2].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBits { result }) if result == measured_bits.value()
    ));
    assert!(matches!(
        circuit.data[3].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { target, .. }) if target == bit_vec
    ));
    assert!(matches!(
        circuit.data[4].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { target, .. }) if target == uint
    ));
}

#[test]
fn test_measurement_drives_expression_control_flow() {
    let mut circuit = Circuit::new(2);

    let measured = circuit.measure(Qubit::new(0)).unwrap();
    let condition = ClassicalExpr::bit_to_bool(measured.expr()).unwrap();
    circuit
        .if_(condition, |body| {
            body.x(Qubit::new(1))?;
            Ok(())
        })
        .unwrap();

    assert_eq!(circuit.data.len(), 2);
    match &circuit.data[1].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            assert_eq!(op.then_body().operations().len(), 1);
            assert!(op.else_body().is_none());
        }
        instruction => panic!("expected if control op, got {instruction:?}"),
    }
}

#[test]
fn test_if_else_with_false_branch() {
    let mut circuit = Circuit::new(2);

    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |then_body| {
                then_body.x(Qubit::new(1))?;
                Ok(())
            },
            |else_body| {
                else_body.z(Qubit::new(1))?;
                Ok(())
            },
        )
        .unwrap();

    assert_eq!(circuit.data.len(), 1);
    match &circuit.data[0].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            assert_eq!(op.then_body().operations().len(), 1);
            assert_eq!(op.else_body().unwrap().operations().len(), 1);
        }
        instruction => panic!("expected if control op, got {instruction:?}"),
    }
}

#[test]
fn test_while_loop_requires_terminal_break_and_continue() {
    let mut circuit = Circuit::new(2);

    assert!(circuit.break_loop().is_err());
    assert!(circuit.continue_loop().is_err());

    let error = circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.h(Qubit::new(1))?;
            body.break_loop()?;
            body.continue_loop()?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(
        error,
        CircuitError::NonTerminalControlTransfer { .. }
    ));
    assert!(circuit.data.is_empty());

    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| body.break_loop())
        .unwrap();

    let mut continue_circuit = Circuit::new(1);
    continue_circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.continue_loop()
        })
        .unwrap();
}

#[test]
fn test_raw_if_body_rejects_out_of_scope_break() {
    let mut circuit = Circuit::new(1);
    let body = ControlBody::new(vec![control_operation(ClassicalControlOp::Break)]);
    let op = IfOp::new(ClassicalExpr::bool_literal(true), body, None).unwrap();

    assert!(circuit.append_control(ClassicalControlOp::If(op)).is_err());
}

#[test]
fn test_raw_if_body_rejects_out_of_scope_continue() {
    let mut circuit = Circuit::new(1);
    let body = ControlBody::new(vec![control_operation(ClassicalControlOp::Continue)]);
    let op = IfOp::new(ClassicalExpr::bool_literal(true), body, None).unwrap();

    assert!(circuit.append_control(ClassicalControlOp::If(op)).is_err());
}

#[test]
fn test_raw_while_body_rejects_nonterminal_nested_control_transfer() {
    let mut circuit = Circuit::new(1);
    let if_body = ControlBody::new(vec![
        control_operation(ClassicalControlOp::Break),
        control_operation(ClassicalControlOp::Continue),
    ]);
    let if_op = IfOp::new(ClassicalExpr::bool_literal(true), if_body, None).unwrap();
    let while_body = ControlBody::new(vec![control_operation(ClassicalControlOp::If(if_op))]);
    let while_op = WhileOp::new(ClassicalExpr::bool_literal(true), while_body).unwrap();

    assert!(matches!(
        circuit.append_control(ClassicalControlOp::While(while_op)),
        Err(CircuitError::NonTerminalControlTransfer { .. })
    ));
}

#[test]
fn test_raw_switch_body_allows_break_but_rejects_continue_without_loop() {
    let mut circuit = Circuit::new(1);
    let break_body = ControlBody::new(vec![control_operation(ClassicalControlOp::Break)]);
    let break_switch = SwitchOp::new(
        ClassicalExpr::uint_literal(2, 0).unwrap(),
        vec![SwitchCase::new(0, break_body)],
        None,
    )
    .unwrap();

    assert!(
        circuit
            .append_control(ClassicalControlOp::Switch(break_switch))
            .is_ok()
    );

    let mut circuit = Circuit::new(1);
    let continue_body = ControlBody::new(vec![control_operation(ClassicalControlOp::Continue)]);
    let continue_switch = SwitchOp::new(
        ClassicalExpr::uint_literal(2, 0).unwrap(),
        vec![SwitchCase::new(0, continue_body)],
        None,
    )
    .unwrap();

    assert!(
        circuit
            .append_control(ClassicalControlOp::Switch(continue_switch))
            .is_err()
    );
}

#[test]
fn test_switch_inside_loop_allows_continue_to_outer_loop() {
    let mut circuit = Circuit::new(1);

    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.switch(ClassicalExpr::uint_literal(2, 0).unwrap(), |case| {
                case.value(0, |case_body| case_body.continue_loop())?;
                Ok(())
            })
        })
        .unwrap();
}

#[test]
fn test_switch_builder_captures_cases_and_default() {
    let mut circuit = Circuit::new(2);
    let state = circuit.var(ClassicalType::uint(2).unwrap());

    circuit
        .switch(state.expr(), |case| {
            case.value(0, |body| {
                body.x(Qubit::new(0))?;
                Ok(())
            })?;
            case.value(1, |body| {
                body.h(Qubit::new(1))?;
                Ok(())
            })?;
            case.value(2, |body| {
                body.h(Qubit::new(1))?;
                Ok(())
            })?;
            case.default(|body| {
                body.z(Qubit::new(0))?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap();

    match &circuit.data[0].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) => {
            assert_eq!(op.cases().len(), 3);
            assert!(op.default().is_some());
        }
        instruction => panic!("expected switch control op, got {instruction:?}"),
    }
}

#[test]
fn test_control_flow_inverse_error() {
    let mut circuit = Circuit::new(2);

    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.h(Qubit::new(1))?;
            Ok(())
        })
        .unwrap();

    let result = circuit.inverse();
    assert!(matches!(
        result.unwrap_err(),
        CircuitError::IrreversibleOperation
    ));
}

#[test]
fn test_body_error_rolls_back_captured_operations() {
    let mut circuit = Circuit::new(1);

    let result = circuit.if_(ClassicalExpr::bool_literal(true), |body| {
        body.x(Qubit::new(0))?;
        Err(CircuitError::InvalidOperation("stop".to_string()))
    });

    assert!(result.is_err());
    assert!(circuit.data.is_empty());
}

#[test]
fn test_control_op_constructor_error_rolls_back_captured_state() {
    let mut circuit = Circuit::new(1);

    let result = circuit.if_(ClassicalExpr::uint_literal(2, 1).unwrap(), |body| {
        let _scratch = body.var(ClassicalType::Bool);
        let _measured = body.measure(Qubit::new(0))?;
        Ok(())
    });

    assert!(result.is_err());
    assert!(circuit.data.is_empty());
    assert!(circuit.classical_vars.is_empty());
    assert!(circuit.classical_values().is_empty());
}

#[test]
fn test_decompose_rejects_classical_control() {
    let mut circuit = Circuit::new(2);

    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.x(Qubit::new(0))?;
            Ok(())
        })
        .unwrap();

    assert!(circuit.decompose().is_err());
}

#[test]
fn test_decompose_remaps_classical_data_identity() {
    let mut circuit = Circuit::new(1);
    let target = circuit.var(ClassicalType::Bool);
    let measured = circuit.measure(Qubit::new(0)).unwrap();
    circuit
        .store(target, ClassicalExpr::bit_to_bool(measured.expr()).unwrap())
        .unwrap();

    let decomposed = circuit.decompose().unwrap();

    assert_ne!(decomposed.id(), circuit.id());
    assert_eq!(decomposed.classical_vars(), circuit.classical_vars());
    assert_eq!(decomposed.classical_values(), circuit.classical_values());
    decomposed.validate().unwrap();

    match &decomposed.operations()[0].instruction {
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }) => {
            assert_eq!(result.circuit_id(), decomposed.id());
        }
        other => panic!("expected measurement, got {other:?}"),
    }
}

#[test]
fn test_compose_basic_with_mapping() {
    // Create qc1 with qubits 1, 3, 5
    let mut qc1 = Circuit::new(0);
    let q1 = Qubit::new(1);
    let q3 = Qubit::new(3);
    let q5 = Qubit::new(5);
    qc1.add_qubits(vec![q1, q3, q5]).unwrap();
    qc1.h(q1).unwrap();

    // Create qc2 with qubits 1, 2
    let mut qc2 = Circuit::new(0);
    let q2 = Qubit::new(2);
    qc2.add_qubits(vec![q1, q2]).unwrap();
    qc2.x(q1).unwrap();

    // Compose: map qc2's q1 -> qc1's q3, qc2's q2 -> qc1's q1
    let result = qc1.compose(&qc2, Some(&[q3, q1]));
    assert!(result.is_ok(), "compose should succeed");

    // Verify: qc1 qubit count unchanged, qubits are {1, 3, 5}
    assert_eq!(qc1.num_qubits(), 3);
    let expected: Vec<Qubit> = vec![q1, q3, q5];
    assert_eq!(qc1.qubits(), expected);

    // Verify: 2 operations (1 from qc1, 1 from qc2)
    assert_eq!(qc1.data.len(), 2);
}

#[test]
fn test_compose_without_mapping() {
    // Create qc1 with qubits 1, 3, 5
    let mut qc1 = Circuit::new(0);
    let q1 = Qubit::new(1);
    let q3 = Qubit::new(3);
    let q5 = Qubit::new(5);
    qc1.add_qubits(vec![q1, q3, q5]).unwrap();
    qc1.h(q1).unwrap();

    // Create qc2 with qubits 6, 7
    let mut qc2 = Circuit::new(0);
    let q6 = Qubit::new(6);
    let q7 = Qubit::new(7);
    qc2.add_qubits(vec![q6, q7]).unwrap();
    qc2.cx(q6, q7).unwrap();

    // Compose without mapping: qc2's qubits are appended
    let result = qc1.compose(&qc2, None);
    assert!(result.is_ok(), "compose should succeed");

    // Verify: 3 + 2 = 5 qubits
    assert_eq!(qc1.num_qubits(), 5);

    // Verify: all qubits present
    let expected: Vec<Qubit> = vec![q1, q3, q5, q6, q7];
    assert_eq!(qc1.qubits(), expected);

    // Verify: 2 operations (1 from qc1, 1 from qc2)
    assert_eq!(qc1.data.len(), 2);
}

#[test]
fn compose_without_mapping_merges_qubits_with_matching_ids() {
    let mut qc1 = Circuit::new(1);
    qc1.h(Qubit::new(0)).unwrap();

    let mut qc2 = Circuit::new(1);
    qc2.x(Qubit::new(0)).unwrap();

    qc1.compose(&qc2, None).unwrap();

    assert_eq!(qc1.qubits(), vec![Qubit::new(0)]);
    assert_eq!(qc1.operations().len(), 2);
    assert_eq!(qc1.operations()[1].qubits.as_slice(), &[Qubit::new(0)]);
}

#[test]
fn test_compose_empty_circuit() {
    let mut qc1 = Circuit::new(2);
    qc1.h(Qubit::new(0)).unwrap();

    let qc2 = Circuit::new(0);

    let result = qc1.compose(&qc2, None);
    assert!(result.is_ok());

    assert_eq!(qc1.num_qubits(), 2);
    assert_eq!(qc1.data.len(), 1);
}

#[test]
fn test_compose_qubit_count_mismatch() {
    let mut qc1 = Circuit::new(0);
    let q1 = Qubit::new(1);
    let q3 = Qubit::new(3);
    qc1.add_qubits(vec![q1, q3]).unwrap();

    let mut qc2 = Circuit::new(0);
    let q2 = Qubit::new(2);
    qc2.add_qubits(vec![q1, q2, q3]).unwrap();

    // Mapping only 2 qubits, but qc2 has 3
    let result = qc1.compose(&qc2, Some(&[q1, q3]));
    assert!(matches!(
        result,
        Err(CircuitError::QubitCountMismatch {
            expected: 3,
            actual: 2
        })
    ));
}

#[test]
fn test_compose_nonexistent_target_qubit() {
    let mut qc1 = Circuit::new(0);
    let q1 = Qubit::new(1);
    qc1.add_qubits(vec![q1]).unwrap();

    let mut qc2 = Circuit::new(0);
    let q2 = Qubit::new(2);
    qc2.add_qubits(vec![q2]).unwrap();

    // Try to map qc2's q2 to q3, which does not exist in qc1
    let q3 = Qubit::new(3);
    let result = qc1.compose(&qc2, Some(&[q3]));
    assert!(matches!(result, Err(CircuitError::QubitNotFound(3))));
}

#[test]
fn test_compose_with_parameters() {
    // qc1 with parameterized gate
    let mut qc1 = Circuit::new(1);
    let theta = Parameter::symbol("theta");
    qc1.rx(Qubit::new(0), theta.clone()).unwrap();

    // qc2 with different parameterized gate
    let mut qc2 = Circuit::new(1);
    let phi = Parameter::symbol("phi");
    qc2.ry(Qubit::new(0), phi.clone()).unwrap();

    let result = qc1.compose(&qc2, None);
    assert!(result.is_ok());

    // Both parameters should be present
    assert_eq!(qc1.parameters().len(), 2);
    assert!(qc1.symbols().contains("theta"));
    assert!(qc1.symbols().contains("phi"));
}

#[test]
fn test_compose_preserves_operation_order() {
    // qc1: Bell state preparation
    let mut qc1 = Circuit::new(2);
    qc1.h(Qubit::new(0)).unwrap();
    qc1.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    // qc2: Bell measurement
    let mut qc2 = Circuit::new(2);
    qc2.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    qc2.h(Qubit::new(0)).unwrap();

    let result = qc1.compose(&qc2, None);
    assert!(result.is_ok());

    // Total: 2 + 2 = 4 operations
    assert_eq!(qc1.data.len(), 4);

    // Verify order: h, cx (from qc1), cx, h (from qc2)
    assert!(matches!(
        qc1.data[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert!(matches!(
        qc1.data[1].instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    assert!(matches!(
        qc1.data[2].instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    assert!(matches!(
        qc1.data[3].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn test_compose_remaps_classical_data_ids() {
    let mut qc1 = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let left_bit = qc1.var(ClassicalType::Bit);
    qc1.measure(q0).unwrap();
    qc1.store(left_bit, ClassicalExpr::bit_literal(true))
        .unwrap();

    let mut qc2 = Circuit::new(2);
    let right_bit = qc2.var(ClassicalType::Bit);
    qc2.measure_into(q0, right_bit).unwrap();

    qc1.compose(&qc2, Some(&[q1, q0])).unwrap();

    assert_eq!(
        qc1.classical_vars(),
        &[ClassicalType::Bit, ClassicalType::Bit]
    );
    assert_eq!(
        qc1.classical_values(),
        &[ClassicalType::Bit, ClassicalType::Bit]
    );

    match &qc1.data[2].instruction {
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }) => {
            assert_eq!(
                *result,
                ClassicalValue::new(qc1.id(), 1, ClassicalType::Bit)
            );
            assert_eq!(qc1.data[2].qubits.as_slice(), &[q1]);
        }
        instruction => panic!("expected remapped measure_bit, got {instruction:?}"),
    }

    match &qc1.data[3].instruction {
        Instruction::ClassicalData(ClassicalDataOp::Store { target, value }) => {
            assert_eq!(*target, ClassicalVar::new(qc1.id(), 1, ClassicalType::Bit));
            assert!(
                value
                    .values()
                    .contains(&ClassicalValue::new(qc1.id(), 1, ClassicalType::Bit))
            );
        }
        instruction => panic!("expected remapped store, got {instruction:?}"),
    }
}

#[test]
fn test_compose_remaps_measurement_driven_if_control() {
    let mut qc1 = Circuit::new(3);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    qc1.measure(q0).unwrap();

    let mut qc2 = Circuit::new(2);
    let measured = qc2.measure(q0).unwrap();
    let condition = ClassicalExpr::bit_to_bool(measured.expr()).unwrap();
    qc2.if_(condition, |body| {
        body.x(Qubit::new(1))?;
        Ok(())
    })
    .unwrap();

    qc1.compose(&qc2, Some(&[q2, q1])).unwrap();

    match &qc1.data[1].instruction {
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }) => {
            assert_eq!(
                *result,
                ClassicalValue::new(qc1.id(), 1, ClassicalType::Bit)
            );
            assert_eq!(qc1.data[1].qubits.as_slice(), &[q2]);
        }
        instruction => panic!("expected remapped measure_bit, got {instruction:?}"),
    }

    match &qc1.data[2].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            assert!(op.condition().values().contains(&ClassicalValue::new(
                qc1.id(),
                1,
                ClassicalType::Bit
            )));
            assert_eq!(op.then_body().operations()[0].qubits.as_slice(), &[q1]);
            assert_eq!(qc1.data[2].qubits.as_slice(), &[q1]);
        }
        instruction => panic!("expected remapped if control op, got {instruction:?}"),
    }
}

#[test]
fn test_compose_remaps_loop_switch_bodies_and_nested_params() {
    let mut qc1 = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    qc1.var(ClassicalType::Bool);

    let mut qc2 = Circuit::new(2);
    let counter = qc2.var(ClassicalType::uint(4).unwrap());
    let selector = qc2.var(ClassicalType::uint(2).unwrap());

    qc2.for_uint(
        counter,
        ClassicalExpr::uint_literal(4, 0).unwrap(),
        ClassicalExpr::uint_literal(4, 2).unwrap(),
        ClassicalExpr::uint_literal(4, 1).unwrap(),
        |body, _| {
            body.h(q0)?;
            Ok(())
        },
    )
    .unwrap();
    qc2.switch(selector.expr(), |case| {
        case.value(0, |body| {
            body.x(q1)?;
            Ok(())
        })?;
        case.default(|body| {
            body.z(q0)?;
            Ok(())
        })?;
        Ok(())
    })
    .unwrap();
    qc2.while_(ClassicalExpr::bool_literal(true), |body| {
        body.rx(q1, Parameter::symbol("theta"))?;
        Ok(())
    })
    .unwrap();

    qc1.compose(&qc2, Some(&[q1, q0])).unwrap();

    assert_eq!(qc1.classical_vars().len(), 3);
    assert_eq!(qc1.parameters().len(), 1);

    match &qc1.data[0].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::For(op)) => {
            assert_eq!(
                op.var(),
                ClassicalVar::new(qc1.id(), 1, ClassicalType::uint(4).unwrap())
            );
            assert_eq!(op.body().operations()[0].qubits.as_slice(), &[q1]);
        }
        instruction => panic!("expected remapped for control op, got {instruction:?}"),
    }

    match &qc1.data[1].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) => {
            assert!(op.target().vars().contains(&ClassicalVar::new(
                qc1.id(),
                2,
                ClassicalType::uint(2).unwrap()
            )));
            assert_eq!(
                op.cases()[0].body().operations()[0].qubits.as_slice(),
                &[q0]
            );
            assert_eq!(
                op.default().unwrap().operations()[0].qubits.as_slice(),
                &[q1]
            );
        }
        instruction => panic!("expected remapped switch control op, got {instruction:?}"),
    }

    match &qc1.data[2].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::While(op)) => {
            let body_op = &op.body().operations()[0];
            assert_eq!(body_op.qubits.as_slice(), &[q0]);
            assert!(matches!(body_op.params[0], CircuitParam::Index(0)));
        }
        instruction => panic!("expected remapped while control op, got {instruction:?}"),
    }
}

#[test]
fn test_classical_handles_are_rejected_by_other_circuits() {
    let mut owner = Circuit::new(1);
    let owner_var = owner.var(ClassicalType::Bit);
    let owner_measurement = owner.measure(Qubit::new(0)).unwrap();

    let mut other = Circuit::new(1);
    assert!(matches!(
        other.store(owner_var, ClassicalExpr::bit_literal(true)),
        Err(CircuitError::ForeignClassicalHandle {
            kind: "classical variable",
            ..
        })
    ));

    let other_var = other.var(ClassicalType::Bit);
    assert!(matches!(
        other.store(other_var, owner_measurement.expr()),
        Err(CircuitError::ForeignClassicalHandle {
            kind: "classical value",
            ..
        })
    ));
}

#[test]
fn test_clone_allocates_new_classical_identity_and_remaps_operations() {
    let mut original = Circuit::new(1);
    let original_var = original.var(ClassicalType::Bit);
    let condition = original.var(ClassicalType::Bool);
    original.measure_into(Qubit::new(0), original_var).unwrap();
    original
        .if_(condition.expr(), |body| body.rz(Qubit::new(0), "theta"))
        .unwrap();

    let cloned = original.clone();
    assert_ne!(original.id(), cloned.id());
    assert_eq!(original, cloned);
    let mut different = cloned.clone();
    different.x(Qubit::new(0)).unwrap();
    assert_ne!(original, different);
    assert!(matches!(
        cloned
            .clone()
            .store(original_var, ClassicalExpr::bit_literal(true)),
        Err(CircuitError::ForeignClassicalHandle { .. })
    ));
    assert!(cloned.validate().is_ok());
}

#[test]
fn test_if_body_measurement_value_cannot_escape() {
    let mut circuit = Circuit::new(1);
    let output = circuit.var(ClassicalType::Bit);
    let mut escaped = None;

    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            escaped = Some(body.measure(Qubit::new(0))?.expr());
            Ok(())
        })
        .unwrap();

    assert!(matches!(
        circuit.store(output, escaped.unwrap()),
        Err(CircuitError::ClassicalValueOutOfScope { .. })
    ));
}

#[test]
fn test_loop_and_switch_measurement_values_cannot_escape() {
    let mut while_circuit = Circuit::new(1);
    let while_output = while_circuit.var(ClassicalType::Bit);
    let mut while_value = None;
    while_circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            while_value = Some(body.measure(Qubit::new(0))?.expr());
            body.break_loop()
        })
        .unwrap();
    assert!(matches!(
        while_circuit.store(while_output, while_value.unwrap()),
        Err(CircuitError::ClassicalValueOutOfScope { .. })
    ));

    let mut for_circuit = Circuit::new(1);
    let for_output = for_circuit.var(ClassicalType::Bit);
    let loop_var = for_circuit.var(ClassicalType::uint(2).unwrap());
    let mut for_value = None;
    for_circuit
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(2, 0).unwrap(),
            ClassicalExpr::uint_literal(2, 2).unwrap(),
            ClassicalExpr::uint_literal(2, 1).unwrap(),
            |body, _| {
                for_value = Some(body.measure(Qubit::new(0))?.expr());
                Ok(())
            },
        )
        .unwrap();
    assert!(matches!(
        for_circuit.store(for_output, for_value.unwrap()),
        Err(CircuitError::ClassicalValueOutOfScope { .. })
    ));

    let mut switch_circuit = Circuit::new(1);
    let switch_output = switch_circuit.var(ClassicalType::Bit);
    let selector = switch_circuit.var(ClassicalType::uint(2).unwrap());
    let mut switch_value = None;
    switch_circuit
        .switch(selector.expr(), |cases| {
            cases.value(0, |body| {
                switch_value = Some(body.measure(Qubit::new(0))?.expr());
                Ok(())
            })?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        switch_circuit.store(switch_output, switch_value.unwrap()),
        Err(CircuitError::ClassicalValueOutOfScope { .. })
    ));
}

#[test]
fn test_body_measurement_can_update_outer_variable() {
    let mut circuit = Circuit::new(1);
    let output = circuit.var(ClassicalType::Bit);

    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            let measurement = body.measure(Qubit::new(0))?;
            body.store(output, measurement.expr())?;
            body.break_loop()
        })
        .unwrap();

    assert!(circuit.validate().is_ok());
}

#[test]
fn test_from_operations_rejects_undefined_and_duplicate_values() {
    let circuit_id = crate::circuit::CircuitId::new();
    let value = ClassicalValue::new(circuit_id, 0, ClassicalType::Bit);
    let target = ClassicalVar::new(circuit_id, 0, ClassicalType::Bit);
    let undefined = Circuit::from_operations(
        vec![Qubit::new(0)],
        vec![ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::ClassicalData(
                ClassicalDataOp::Store {
                    target,
                    value: value.expr(),
                },
            )),
            qubits: smallvec![],
            params: smallvec![],
            label: None,
        }],
        Some(vec![ClassicalType::Bit]),
        Some(vec![ClassicalType::Bit]),
    );
    assert!(matches!(
        undefined,
        Err(CircuitError::UndefinedClassicalValue { .. })
    ));

    let measure = || ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::ClassicalData(
            ClassicalDataOp::MeasureBit { result: value },
        )),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![],
        label: None,
    };
    let duplicate = Circuit::from_operations(
        vec![Qubit::new(0)],
        vec![measure(), measure()],
        None,
        Some(vec![ClassicalType::Bit]),
    );
    assert!(matches!(
        duplicate,
        Err(CircuitError::DuplicateClassicalValueDefinition { .. })
    ));
}
