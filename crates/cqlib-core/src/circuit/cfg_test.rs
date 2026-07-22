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

use crate::circuit::cfg::{BasicBlock, CircuitCFG, FlowEdge, Terminator};
use crate::circuit::{
    Circuit, CircuitError, ClassicalControlOp, ClassicalExpr, ClassicalType, Instruction,
    Operation, Qubit, StandardGate,
};
use rustworkx_core::petgraph::prelude::NodeIndex;
use smallvec::smallvec;

#[test]
fn basic_block_tracks_operations_and_terminator() {
    let mut block = BasicBlock::new().with_label("entry");
    assert!(block.is_empty());
    assert_eq!(block.label(), Some("entry"));

    block.push_operation(Operation {
        instruction: Instruction::Standard(StandardGate::H),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![],
        label: None,
    });
    block.set_terminator(Terminator::Return);

    assert_eq!(block.len(), 1);
    assert!(block.has_terminator());
}

#[test]
fn empty_circuit_round_trips() {
    let circuit = Circuit::new(2);
    let cfg = CircuitCFG::from_circuit(&circuit).unwrap();

    assert_eq!(cfg.num_blocks(), 1);
    let entry = cfg.entry_block().unwrap();
    assert!(matches!(
        cfg.data[entry].terminator(),
        Some(Terminator::Return)
    ));

    let recovered = cfg.to_circuit().unwrap();
    assert_eq!(recovered.num_qubits(), 2);
    assert!(recovered.operations().is_empty());
}

#[test]
fn classical_data_and_linear_operations_round_trip() {
    let mut circuit = Circuit::new(2);
    let flag = circuit.var(ClassicalType::Bool);
    circuit.h(Qubit::new(0)).unwrap();
    let measured = circuit.measure(Qubit::new(0)).unwrap();
    circuit
        .store(flag, ClassicalExpr::bit_to_bool(measured.expr()).unwrap())
        .unwrap();

    let recovered = CircuitCFG::from_circuit(&circuit)
        .unwrap()
        .to_circuit()
        .unwrap();

    assert_eq!(recovered.classical_vars(), circuit.classical_vars());
    assert_eq!(recovered.classical_values(), circuit.classical_values());
    assert_eq!(recovered.operations().len(), circuit.operations().len());
    assert!(matches!(
        recovered.operations()[1].instruction,
        Instruction::ClassicalData(_)
    ));
    assert!(matches!(
        recovered.operations()[2].instruction,
        Instruction::ClassicalData(_)
    ));
}

#[test]
fn if_without_else_round_trips_as_absent_else() {
    let mut circuit = Circuit::new(2);
    let measured = circuit.measure(Qubit::new(0)).unwrap();
    let condition = ClassicalExpr::bit_to_bool(measured.expr()).unwrap();
    circuit
        .if_(condition, |body| {
            body.x(Qubit::new(1))?;
            Ok(())
        })
        .unwrap();

    let cfg = CircuitCFG::from_circuit(&circuit).unwrap();
    assert!(
        cfg.blocks()
            .any(|(_, block)| matches!(block.terminator(), Some(Terminator::Branch(_))))
    );

    let recovered = cfg.to_circuit().unwrap();
    match &recovered.operations()[1].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            assert_eq!(op.then_body().operations().len(), 1);
            assert!(op.else_body().is_none());
        }
        _ => panic!("expected if operation"),
    }
}

#[test]
fn if_else_round_trip_distinguishes_empty_else() {
    let mut circuit = Circuit::new(1);
    let condition = ClassicalExpr::bool_literal(true);
    circuit
        .if_else(
            condition,
            |body| {
                body.h(Qubit::new(0))?;
                Ok(())
            },
            |_body| Ok(()),
        )
        .unwrap();

    let recovered = CircuitCFG::from_circuit(&circuit)
        .unwrap()
        .to_circuit()
        .unwrap();

    match &recovered.operations()[0].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            assert_eq!(op.then_body().operations().len(), 1);
            assert!(op.else_body().unwrap().operations().is_empty());
        }
        _ => panic!("expected if operation"),
    }
}

#[test]
fn while_loop_round_trips_with_continue() {
    let mut circuit = Circuit::new(1);
    let keep_running = circuit.var(ClassicalType::Bool);
    circuit
        .store(keep_running, ClassicalExpr::bool_literal(true))
        .unwrap();
    circuit
        .while_(keep_running.expr(), |body| {
            body.continue_loop()?;
            Ok(())
        })
        .unwrap();

    let cfg = CircuitCFG::from_circuit(&circuit).unwrap();
    assert!(cfg.blocks().any(|(node, _)| cfg.is_loop_header(node)));

    let recovered = cfg.to_circuit().unwrap();
    match &recovered.operations()[1].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::While(op)) => {
            assert_eq!(op.body().operations().len(), 1);
            assert!(matches!(
                op.body().operations()[0].instruction,
                Instruction::ClassicalControl(ClassicalControlOp::Continue)
            ));
        }
        _ => panic!("expected while operation"),
    }
}

#[test]
fn for_loop_round_trips_without_lowering_to_while() {
    let mut circuit = Circuit::new(1);
    let i = circuit.var(ClassicalType::uint(8).unwrap());
    circuit
        .for_uint(
            i,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 4).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |body, _i| {
                body.h(Qubit::new(0))?;
                Ok(())
            },
        )
        .unwrap();

    let cfg = CircuitCFG::from_circuit(&circuit).unwrap();
    assert!(
        cfg.blocks()
            .any(|(_, block)| matches!(block.terminator(), Some(Terminator::ForLoop { .. })))
    );

    let recovered = cfg.to_circuit().unwrap();
    match &recovered.operations()[0].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::For(op)) => {
            assert_eq!(op.var(), i);
            assert_eq!(op.body().operations().len(), 1);
        }
        _ => panic!("expected for operation"),
    }
}

#[test]
fn switch_round_trips_cases_default_and_break() {
    let mut circuit = Circuit::new(2);
    let state = circuit.var(ClassicalType::uint(2).unwrap());
    circuit
        .store(state, ClassicalExpr::uint_literal(2, 1).unwrap())
        .unwrap();
    circuit
        .switch(state.expr(), |case| {
            case.value(0, |body| {
                body.x(Qubit::new(0))?;
                Ok(())
            })?;
            case.value(1, |body| {
                body.break_loop()?;
                Ok(())
            })?;
            case.default(|body| {
                body.z(Qubit::new(1))?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap();

    let cfg = CircuitCFG::from_circuit(&circuit).unwrap();
    assert!(
        cfg.blocks()
            .any(|(_, block)| matches!(block.terminator(), Some(Terminator::Switch(_))))
    );

    let recovered = cfg.to_circuit().unwrap();
    match &recovered.operations()[1].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) => {
            assert_eq!(op.cases().len(), 2);
            assert!(op.default().is_some());
            assert!(matches!(
                op.cases()[1].body().operations()[0].instruction,
                Instruction::ClassicalControl(ClassicalControlOp::Break)
            ));
        }
        _ => panic!("expected switch operation"),
    }
}

#[test]
fn break_continue_must_be_terminal_in_body() {
    let mut circuit = Circuit::new(1);
    let keep_running = circuit.var(ClassicalType::Bool);
    circuit
        .store(keep_running, ClassicalExpr::bool_literal(true))
        .unwrap();
    let error = circuit
        .while_(keep_running.expr(), |body| {
            body.break_loop()?;
            body.h(Qubit::new(0))?;
            Ok(())
        })
        .unwrap_err();

    assert!(matches!(
        error,
        CircuitError::NonTerminalControlTransfer { .. }
    ));
}

#[test]
fn invalid_cfg_missing_true_branch_is_rejected() {
    let mut cfg = CircuitCFG::new(1);
    let entry = cfg.add_block(BasicBlock::new().with_label("entry"));
    let false_target = cfg.add_block(BasicBlock::new().with_label("false"));
    cfg.set_entry_block(entry);
    cfg.data[entry].set_terminator(Terminator::Branch(ClassicalExpr::bool_literal(true)));
    cfg.data[false_target].set_terminator(Terminator::Return);
    cfg.add_edge(entry, false_target, FlowEdge::FalseBranch);

    let error = cfg.to_circuit().unwrap_err();
    assert!(error.to_string().contains("TrueBranch"));
}

#[test]
fn invalid_cfg_unknown_classical_value_is_rejected() {
    let mut cfg = CircuitCFG::new(1);
    let entry = cfg.add_block(BasicBlock::new().with_label("entry"));
    cfg.set_entry_block(entry);
    let value = crate::circuit::ClassicalValue::new(
        crate::circuit::CircuitId::new(),
        0,
        ClassicalType::Bit,
    );
    cfg.data[entry].set_terminator(Terminator::Branch(
        ClassicalExpr::bit_to_bool(value.expr()).unwrap(),
    ));
    let then_block = cfg.add_block(BasicBlock::new().with_label("then"));
    let else_block = cfg.add_block(BasicBlock::new().with_label("else"));
    cfg.add_edge(entry, then_block, FlowEdge::TrueBranch);
    cfg.add_edge(entry, else_block, FlowEdge::FalseBranch);

    let error = cfg.validate().unwrap_err();
    assert!(error.to_string().contains("unknown classical value"));
}

#[test]
fn add_edge_rejects_unknown_endpoint() {
    let mut cfg = CircuitCFG::new(1);
    let entry = cfg.add_block(BasicBlock::new());
    assert!(
        cfg.add_edge(entry, NodeIndex::new(99), FlowEdge::Unconditional)
            .is_none()
    );
}
