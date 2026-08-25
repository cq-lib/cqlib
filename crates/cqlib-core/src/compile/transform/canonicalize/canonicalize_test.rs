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

use super::{CanonicalizeConfig, Canonicalizer, canonicalize_circuit};
use crate::circuit::gate::FrozenCircuit;
use crate::circuit::{
    Circuit, CircuitError, CircuitGate, CircuitParam, ClassicalControlOp, ClassicalDataOp,
    ClassicalExpr, ClassicalType, Directive, Instruction, MCGate, Parameter, ParameterValue, Qubit,
    StandardGate, UnitaryGate, ValueInstruction, ValueOperation, circuit_to_matrix,
};
use crate::compile::test_utils::generated_small_matrix_circuit;
use crate::compile::transform::{RewriteEdits, TransformOutcome, TransformerTestExt};
use ndarray::array;
use num_complex::Complex64;
use proptest::prelude::*;
use smallvec::smallvec;

#[test]
fn reports_exact_canonicalization_deletion_provenance() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();
    circuit.i(q0).unwrap();
    circuit.x(q0).unwrap();

    let (outcome, edits) = Canonicalizer::production()
        .transform_with_rewrite_edits(&circuit)
        .unwrap();
    assert!(matches!(outcome, TransformOutcome::Changed(_)));
    let RewriteEdits::Linear {
        old_len,
        new_len,
        replacements,
    } = edits
    else {
        panic!("expected exact linear edits");
    };
    assert_eq!((old_len, new_len), (3, 2));
    assert_eq!(replacements.len(), 1);
    assert_eq!(replacements[0].old, 1..2);
    assert_eq!(replacements[0].new, 1..1);
}

#[test]
fn parameter_table_is_rebuilt_and_unused_params_are_removed() {
    let mut circuit = Circuit::new(1);
    circuit.add_parameter(Parameter::symbol("unused"));
    let theta = Parameter::symbol("theta");
    circuit.rz(Qubit::new(0), theta.clone() - theta).unwrap();
    circuit.h(Qubit::new(0)).unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert!(result.changed);
    assert!(result.circuit.parameters().is_empty());
    assert_eq!(result.circuit.operations().len(), 1);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn top_level_gphase_is_folded_into_circuit_global_phase() {
    let mut circuit = Circuit::new(1);
    circuit.set_global_phase(Parameter::from(0.125));
    circuit
        .append(
            Instruction::Standard(StandardGate::GPhase),
            std::iter::empty::<Qubit>(),
            [ParameterValue::Fixed(0.25)],
            Some("ignored"),
        )
        .unwrap();
    circuit.h(Qubit::new(0)).unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert_eq!(result.circuit.operations().len(), 1);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert!((result.circuit.global_phase().evaluate(&None).unwrap() - 0.375).abs() < 1e-12);
}

#[test]
fn body_gphase_is_merged_into_first_body_operation() {
    let mut circuit = Circuit::new(2);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.h(Qubit::new(1))?;
            body.append(
                Instruction::Standard(StandardGate::GPhase),
                std::iter::empty::<Qubit>(),
                [ParameterValue::Fixed(0.5)],
                None,
            )?;
            body.append(
                Instruction::Standard(StandardGate::GPhase),
                std::iter::empty::<Qubit>(),
                [ParameterValue::Fixed(0.25)],
                None,
            )?;
            Ok(())
        })
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::If(gate)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected if");
    };

    assert_eq!(gate.then_body().operations().len(), 2);
    assert!(matches!(
        gate.then_body().operations()[0].instruction,
        Instruction::Standard(StandardGate::GPhase)
    ));
    match gate.then_body().operations()[0].params.as_slice() {
        [CircuitParam::Fixed(value)] => assert_eq!(*value, 0.75),
        params => panic!("expected one fixed GPhase parameter, got {params:?}"),
    }
    assert!(matches!(
        gate.then_body().operations()[1].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn body_zero_gphase_is_removed() {
    let mut circuit = Circuit::new(2);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.append(
                Instruction::Standard(StandardGate::GPhase),
                std::iter::empty::<Qubit>(),
                [ParameterValue::Fixed(0.0)],
                Some("zero-phase"),
            )?;
            body.h(Qubit::new(1))?;
            Ok(())
        })
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::If(gate)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected if");
    };

    assert_eq!(gate.then_body().operations().len(), 1);
    assert!(matches!(
        gate.then_body().operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn false_body_and_while_body_keep_independent_local_phase() {
    let mut circuit = Circuit::new(2);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.append(
                    Instruction::Standard(StandardGate::GPhase),
                    std::iter::empty::<Qubit>(),
                    [ParameterValue::Fixed(0.25)],
                    None,
                )
            },
            |body| {
                body.append(
                    Instruction::Standard(StandardGate::GPhase),
                    std::iter::empty::<Qubit>(),
                    [ParameterValue::Fixed(0.5)],
                    None,
                )
            },
        )
        .unwrap();
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.append(
                Instruction::Standard(StandardGate::GPhase),
                std::iter::empty::<Qubit>(),
                [ParameterValue::Fixed(0.75)],
                None,
            )?;
            body.h(Qubit::new(1))?;
            Ok(())
        })
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::If(if_gate)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected if");
    };
    let Instruction::ClassicalControl(ClassicalControlOp::While(while_gate)) =
        &result.circuit.operations()[1].instruction
    else {
        panic!("expected while");
    };

    match if_gate.then_body().operations()[0].params.as_slice() {
        [CircuitParam::Fixed(value)] => assert_eq!(*value, 0.25),
        params => panic!("expected one fixed true-body phase parameter, got {params:?}"),
    }

    let false_body = if_gate.else_body().expect("expected false body");
    match false_body.operations()[0].params.as_slice() {
        [CircuitParam::Fixed(value)] => assert_eq!(*value, 0.5),
        params => panic!("expected one fixed false-body phase parameter, got {params:?}"),
    }

    match while_gate.body().operations()[0].params.as_slice() {
        [CircuitParam::Fixed(value)] => assert_eq!(*value, 0.75),
        params => panic!("expected one fixed while-body phase parameter, got {params:?}"),
    }
    assert!(matches!(
        while_gate.body().operations()[1].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn nested_control_flow_is_recursively_canonicalized() {
    let mut circuit = Circuit::new(3);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.while_(ClassicalExpr::bool_literal(true), |nested| {
                nested.i(Qubit::new(1))?;
                nested.h(Qubit::new(2))?;
                Ok(())
            })
        })
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::If(if_gate)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected if");
    };
    let Instruction::ClassicalControl(ClassicalControlOp::While(while_gate)) =
        &if_gate.then_body().operations()[0].instruction
    else {
        panic!("expected nested while");
    };

    assert_eq!(
        result.circuit.operations()[0].qubits.as_slice(),
        &[Qubit::new(2)]
    );
    assert_eq!(
        if_gate.then_body().operations()[0].qubits.as_slice(),
        &[Qubit::new(2)]
    );
    assert_eq!(while_gate.body().operations().len(), 1);
    assert!(matches!(
        while_gate.body().operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn mc_gate_collapses_to_standard_gate() {
    let mut circuit = Circuit::new(2);
    circuit
        .append(
            Instruction::McGate(Box::new(MCGate::new(1, StandardGate::X))),
            [Qubit::new(0), Qubit::new(1)],
            std::iter::empty(),
            Some("cx-label"),
        )
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    assert_eq!(
        result.circuit.operations()[0].label.as_deref(),
        Some("cx-label")
    );
}

#[test]
fn noops_are_removed_even_when_labeled() {
    let mut circuit = Circuit::new(2);
    circuit
        .append(
            Instruction::Standard(StandardGate::I),
            [Qubit::new(0)],
            std::iter::empty(),
            Some("drop-i"),
        )
        .unwrap();
    circuit
        .append(
            Instruction::Delay,
            [Qubit::new(0)],
            [ParameterValue::Fixed(0.0)],
            Some("drop-delay"),
        )
        .unwrap();
    circuit
        .append(
            Instruction::Standard(StandardGate::RXX),
            [Qubit::new(0), Qubit::new(1)],
            [ParameterValue::Fixed(0.0)],
            Some("drop-rxx"),
        )
        .unwrap();
    circuit
        .append(
            Instruction::Standard(StandardGate::U),
            [Qubit::new(0)],
            [
                ParameterValue::Fixed(0.0),
                ParameterValue::Fixed(0.0),
                ParameterValue::Fixed(0.0),
            ],
            Some("drop-u"),
        )
        .unwrap();
    circuit.x(Qubit::new(1)).unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert_eq!(result.circuit.operations().len(), 1);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
}

#[test]
fn config_can_preserve_noops_when_disabled() {
    let mut circuit = Circuit::new(1);
    circuit.i(Qubit::new(0)).unwrap();
    circuit.h(Qubit::new(0)).unwrap();

    let canonicalizer = Canonicalizer::new(CanonicalizeConfig::new().drop_noops(false));
    let result = canonicalizer.run(&circuit).unwrap();

    assert_eq!(result.circuit.operations().len(), 2);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::I)
    ));
}

#[test]
fn canonicalizer_implements_transformer_trait() {
    let mut circuit = Circuit::new(1);
    circuit.i(Qubit::new(0)).unwrap();

    let transformer = &Canonicalizer::production();
    let result = transformer.transform_resolved(&circuit, None).unwrap();

    assert!(result.changed);
    assert!(result.circuit.operations().is_empty());
}

#[test]
fn config_can_preserve_top_level_gphase_when_disabled() {
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            Instruction::Standard(StandardGate::GPhase),
            std::iter::empty::<Qubit>(),
            [ParameterValue::Fixed(0.25)],
            None,
        )
        .unwrap();

    let canonicalizer = Canonicalizer::new(CanonicalizeConfig::new().fold_gphase(false));
    let result = canonicalizer.run(&circuit).unwrap();

    assert_eq!(result.circuit.operations().len(), 1);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::GPhase)
    ));
    match result.circuit.operations()[0].params.as_slice() {
        [CircuitParam::Fixed(value)] => assert_eq!(*value, 0.25),
        params => panic!("expected one fixed top-level GPhase parameter, got {params:?}"),
    }
}

#[test]
fn config_can_skip_control_flow_recursion() {
    let mut circuit = Circuit::new(2);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.append(
                Instruction::Standard(StandardGate::I),
                [Qubit::new(1)],
                std::iter::empty(),
                Some("preserve-body"),
            )
        })
        .unwrap();

    let canonicalizer = Canonicalizer::new(CanonicalizeConfig::new().recurse_control_flow(false));
    let result = canonicalizer.run(&circuit).unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::If(gate)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected if");
    };

    assert_eq!(gate.then_body().operations().len(), 1);
    assert!(matches!(
        gate.then_body().operations()[0].instruction,
        Instruction::Standard(StandardGate::I)
    ));
    assert_eq!(
        gate.then_body().operations()[0].label.as_deref(),
        Some("preserve-body")
    );
}

#[test]
fn config_can_preserve_barrier_shape_when_disabled() {
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::Directive(Directive::Barrier),
            [Qubit::new(2), Qubit::new(0), Qubit::new(1)],
            std::iter::empty(),
            Some("keep-barrier"),
        )
        .unwrap();

    let canonicalizer = Canonicalizer::new(CanonicalizeConfig::new().canonicalize_barriers(false));
    let result = canonicalizer.run(&circuit).unwrap();
    let op = &result.circuit.operations()[0];

    assert_eq!(
        op.qubits.as_slice(),
        &[Qubit::new(2), Qubit::new(0), Qubit::new(1)]
    );
    assert_eq!(op.label.as_deref(), Some("keep-barrier"));
}

#[test]
fn barrier_scopes_are_canonicalized_and_merged() {
    let mut circuit = Circuit::new(4);
    circuit
        .append(
            Instruction::Directive(Directive::Barrier),
            [Qubit::new(2), Qubit::new(1)],
            std::iter::empty(),
            Some("drop-label"),
        )
        .unwrap();
    circuit
        .append(
            Instruction::Directive(Directive::Barrier),
            [Qubit::new(1), Qubit::new(2), Qubit::new(3)],
            std::iter::empty(),
            None,
        )
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert_eq!(result.circuit.operations().len(), 1);
    let op = &result.circuit.operations()[0];
    assert!(matches!(
        op.instruction,
        Instruction::Directive(Directive::Barrier)
    ));
    assert_eq!(
        op.qubits.as_slice(),
        &[Qubit::new(1), Qubit::new(2), Qubit::new(3)]
    );
    assert!(op.label.is_none());
}

#[test]
fn barrier_partial_overlap_and_non_adjacent_barriers_are_not_merged() {
    let mut circuit = Circuit::new(4);
    circuit
        .append(
            Instruction::Directive(Directive::Barrier),
            [Qubit::new(0), Qubit::new(1)],
            std::iter::empty(),
            None,
        )
        .unwrap();
    circuit
        .append(
            Instruction::Directive(Directive::Barrier),
            [Qubit::new(1), Qubit::new(2)],
            std::iter::empty(),
            None,
        )
        .unwrap();
    circuit.h(Qubit::new(0)).unwrap();
    circuit
        .append(
            Instruction::Directive(Directive::Barrier),
            [Qubit::new(0), Qubit::new(1), Qubit::new(2)],
            std::iter::empty(),
            None,
        )
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert_eq!(result.circuit.operations().len(), 4);
    assert_eq!(
        result.circuit.operations()[0].qubits.as_slice(),
        &[Qubit::new(0), Qubit::new(1)]
    );
    assert_eq!(
        result.circuit.operations()[1].qubits.as_slice(),
        &[Qubit::new(1), Qubit::new(2)]
    );
    assert!(matches!(
        result.circuit.operations()[2].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn empty_barrier_is_preserved_as_global_boundary() {
    let mut circuit = Circuit::new(1);
    let q0 = Qubit::new(0);
    circuit.x(q0).unwrap();
    circuit
        .append(
            Instruction::Directive(Directive::Barrier),
            std::iter::empty::<Qubit>(),
            std::iter::empty(),
            Some("global-boundary"),
        )
        .unwrap();
    circuit.x(q0).unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert_eq!(result.circuit.operations().len(), 3);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert!(matches!(
        result.circuit.operations()[1].instruction,
        Instruction::Directive(Directive::Barrier)
    ));
    assert!(result.circuit.operations()[1].qubits.is_empty());
    assert!(matches!(
        result.circuit.operations()[2].instruction,
        Instruction::Standard(StandardGate::X)
    ));
}

#[test]
fn global_and_local_adjacent_barriers_merge_to_global_boundary() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    for barriers in [
        vec![Vec::new(), vec![q0, q1]],
        vec![vec![q0, q1], Vec::new()],
    ] {
        let mut circuit = Circuit::new(2);
        for qubits in barriers {
            circuit.barrier(qubits).unwrap();
        }

        let result = canonicalize_circuit(&circuit).unwrap();

        assert_eq!(result.circuit.operations().len(), 1);
        assert!(matches!(
            result.circuit.operations()[0].instruction,
            Instruction::Directive(Directive::Barrier)
        ));
        assert!(result.circuit.operations()[0].qubits.is_empty());
    }
}

#[test]
fn control_flow_qubits_are_recomputed_in_global_order() {
    let mut circuit = Circuit::new(3);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.h(Qubit::new(2))?;
            body.i(Qubit::new(1))?;
            Ok(())
        })
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert_eq!(
        result.circuit.operations()[0].qubits.as_slice(),
        &[Qubit::new(2)]
    );
}

#[test]
fn for_body_is_recursively_canonicalized_and_loop_metadata_is_preserved() {
    let mut circuit = Circuit::new(2);
    let loop_var = circuit.var(ClassicalType::uint(8).unwrap());
    circuit
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 4).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |body, _index| {
                body.i(Qubit::new(0))?;
                body.h(Qubit::new(1))?;
                Ok(())
            },
        )
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::For(op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected for");
    };

    assert_eq!(op.var(), loop_var);
    assert_eq!(op.body().operations().len(), 1);
    assert!(matches!(
        op.body().operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert_eq!(
        result.circuit.operations()[0].qubits.as_slice(),
        &[Qubit::new(1)]
    );
}

#[test]
fn switch_bodies_are_recursively_canonicalized_and_break_is_preserved() {
    let mut circuit = Circuit::new(2);
    circuit
        .switch(ClassicalExpr::uint_literal(2, 1).unwrap(), |switch| {
            switch.value(1, |body| {
                body.i(Qubit::new(0))?;
                body.h(Qubit::new(1))?;
                Ok(())
            })?;
            switch.default(|body| body.break_loop())
        })
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected switch");
    };

    assert_eq!(op.cases().len(), 1);
    assert_eq!(op.cases()[0].body().operations().len(), 1);
    assert!(matches!(
        op.cases()[0].body().operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    let default = op.default().expect("expected default case");
    assert!(matches!(
        default.operations()[0].instruction,
        Instruction::ClassicalControl(ClassicalControlOp::Break)
    ));
}

#[test]
fn while_continue_is_preserved() {
    let mut circuit = Circuit::new(1);
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.continue_loop()
        })
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::While(op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected while");
    };

    assert!(matches!(
        op.body().operations()[0].instruction,
        Instruction::ClassicalControl(ClassicalControlOp::Continue)
    ));
}

#[test]
fn classical_data_operations_are_preserved() {
    let mut circuit = Circuit::new(2);
    let bit = circuit.var(ClassicalType::Bit);
    let bits = circuit.var(ClassicalType::bit_vec(2).unwrap());
    let measurement = circuit.measure_into(Qubit::new(0), bit).unwrap();
    circuit
        .measure_bits_into([Qubit::new(0), Qubit::new(1)], bits)
        .unwrap();
    circuit
        .store(bit, ClassicalExpr::bit_literal(true))
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert_eq!(result.circuit.classical_vars(), circuit.classical_vars());
    assert_eq!(
        result.circuit.classical_values(),
        circuit.classical_values()
    );
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert!(matches!(
        result.circuit.operations()[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));
    assert!(matches!(
        result.circuit.operations()[2].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBits { .. })
    ));
    assert!(matches!(
        result.circuit.operations()[3].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));
    assert_eq!(measurement.qubits(), &[Qubit::new(0)]);
}

#[test]
fn unused_classical_variable_table_entries_are_preserved() {
    let mut circuit = Circuit::new(1);
    let _unused = circuit.var(ClassicalType::Bool);
    circuit.h(Qubit::new(0)).unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert_eq!(result.circuit.classical_vars(), circuit.classical_vars());
    assert!(result.circuit.classical_values().is_empty());
    assert_eq!(result.circuit.operations().len(), 1);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn self_store_removal_allows_unused_classical_variable_entry() {
    let mut circuit = Circuit::new(1);
    let flag = circuit.var(ClassicalType::Bool);
    circuit.store(flag, flag.expr()).unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert_eq!(result.circuit.classical_vars(), circuit.classical_vars());
    assert!(result.circuit.operations().is_empty());
}

#[test]
fn expression_simplification_may_leave_classical_variable_entry_unused() {
    let mut circuit = Circuit::new(1);
    let flag = circuit.var(ClassicalType::Bool);
    let condition = ClassicalExpr::eq(flag.expr(), flag.expr()).unwrap();
    circuit
        .if_(condition, |body| {
            body.h(Qubit::new(0))?;
            Ok(())
        })
        .unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert_eq!(result.circuit.classical_vars(), circuit.classical_vars());
    let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected if");
    };
    assert_eq!(op.condition(), &ClassicalExpr::bool_literal(true));
}

#[test]
fn unknown_qubit_is_rejected() {
    let result = Circuit::from_operations(
        vec![Qubit::new(0)],
        vec![ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::H)),
            qubits: smallvec![Qubit::new(1)],
            params: smallvec![],
            label: None,
        }],
        None,
        None,
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn duplicate_non_barrier_qubit_is_rejected_during_construction() {
    let err = Circuit::from_operations(
        vec![Qubit::new(0)],
        vec![ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::Standard(
                StandardGate::CX,
            )),
            qubits: smallvec![Qubit::new(0), Qubit::new(0)],
            params: smallvec![],
            label: None,
        }],
        None,
        None,
    )
    .unwrap_err();

    assert!(matches!(err, CircuitError::DuplicateQubits));
}

#[test]
fn invalid_arity_is_rejected_during_circuit_construction() {
    let err = Circuit::from_operations(
        vec![Qubit::new(0)],
        vec![ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::Standard(
                StandardGate::CX,
            )),
            qubits: smallvec![Qubit::new(0)],
            params: smallvec![],
            label: None,
        }],
        None,
        None,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        CircuitError::QubitCountMismatch {
            expected: 2,
            actual: 1
        }
    ));
}

#[test]
fn parameter_count_mismatch_is_rejected_during_circuit_construction() {
    let err = Circuit::from_operations(
        vec![Qubit::new(0)],
        vec![ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::Standard(
                StandardGate::RX,
            )),
            qubits: smallvec![Qubit::new(0)],
            params: smallvec![],
            label: None,
        }],
        None,
        None,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        CircuitError::ParameterCountMismatch {
            expected: 1,
            actual: 0
        }
    ));
}

#[test]
fn non_finite_fixed_parameter_is_rejected() {
    let mut circuit = Circuit::new(1);
    let err = circuit
        .append(
            Instruction::Standard(StandardGate::RX),
            [Qubit::new(0)],
            [ParameterValue::Fixed(f64::NAN)],
            None,
        )
        .unwrap_err();

    assert!(matches!(
        err,
        CircuitError::InvalidParameterValue(0, value) if value.is_nan()
    ));
}

#[test]
fn classical_measurement_reset_circuit_gate_and_unitary_gate_are_preserved() {
    let mut inner = Circuit::new(1);
    inner.x(Qubit::new(0)).unwrap();
    let circuit_gate = CircuitGate::new("inner_x", FrozenCircuit::new(inner)).unwrap();
    let unitary = UnitaryGate::new("custom_x", 1, 0)
        .with_matrix(array![
            [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
        ])
        .unwrap();
    let mut circuit = Circuit::new(1);
    circuit.measure(Qubit::new(0)).unwrap();
    circuit.reset(Qubit::new(0)).unwrap();
    circuit
        .append(
            Instruction::CircuitGate(Box::new(circuit_gate)),
            [Qubit::new(0)],
            std::iter::empty(),
            Some("composite"),
        )
        .unwrap();
    circuit.unitary(unitary, vec![Qubit::new(0)]).unwrap();

    let result = canonicalize_circuit(&circuit).unwrap();

    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert!(matches!(
        result.circuit.operations()[1].instruction,
        Instruction::Directive(Directive::Reset)
    ));
    assert!(matches!(
        result.circuit.operations()[2].instruction,
        Instruction::CircuitGate(_)
    ));
    assert!(matches!(
        result.circuit.operations()[3].instruction,
        Instruction::UnitaryGate(_)
    ));
    assert_eq!(
        result.circuit.operations()[2].label.as_deref(),
        Some("composite")
    );
}

#[test]
fn matrix_is_strictly_preserved_without_control_flow() {
    let mut circuit = Circuit::new(2);
    circuit.set_global_phase(Parameter::from(0.5));
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.rx(Qubit::new(1), Parameter::from(0.0)).unwrap();
    circuit.barrier(vec![Qubit::new(1), Qubit::new(0)]).unwrap();

    let before = circuit_to_matrix(&circuit, None).unwrap();
    let result = canonicalize_circuit(&circuit).unwrap();
    let after = circuit_to_matrix(&result.circuit, None).unwrap();

    let before = before.as_slice().expect("matrix storage is contiguous");
    let after = after.as_slice().expect("matrix storage is contiguous");
    assert_eq!(before.len(), after.len());
    for (index, (before, after)) in before.iter().zip(after).enumerate() {
        let diff = (*before - *after).norm();
        assert!(
            diff <= 1e-10,
            "matrix entry {index} differs: before={before}, after={after}, diff={diff}"
        );
    }

    assert!(result.circuit.operations().iter().all(|operation| {
        !matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::GPhase)
        )
    }));
}

#[test]
fn canonicalization_is_idempotent() {
    let mut circuit = Circuit::new(2);
    circuit.i(Qubit::new(0)).unwrap();
    circuit.barrier(vec![Qubit::new(1), Qubit::new(0)]).unwrap();
    circuit.barrier(vec![Qubit::new(0), Qubit::new(1)]).unwrap();
    circuit.h(Qubit::new(1)).unwrap();

    let first = canonicalize_circuit(&circuit).unwrap();
    let second = canonicalize_circuit(&first.circuit).unwrap();

    assert!(first.changed);
    assert!(!second.changed);
}

#[test]
fn canonical_transform_reports_stable_input_without_an_output_circuit() {
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();

    let (outcome, edits) = Canonicalizer::production()
        .transform_with_rewrite_edits(&circuit)
        .unwrap();

    assert!(matches!(outcome, TransformOutcome::Unchanged));
    assert!(matches!(
        edits,
        RewriteEdits::Linear { replacements, .. } if replacements.is_empty()
    ));
}

#[test]
fn canonical_transform_normalizes_global_phase_accumulator_residue() {
    let mut circuit = Circuit::new(1);
    circuit.set_global_phase(Parameter::from(-3.330_669_073_875_469_6e-16));
    circuit.h(Qubit::new(0)).unwrap();

    let first = Canonicalizer::production().run(&circuit).unwrap();
    assert!(first.changed);
    assert_eq!(first.circuit.global_phase(), Parameter::from(0.0));

    let second = Canonicalizer::production().run(&first.circuit).unwrap();
    assert!(!second.changed);
}

#[test]
fn canonicalization_is_idempotent_for_mixed_production_input() {
    let mut circuit = Circuit::new(3);
    circuit.set_global_phase(Parameter::from(0.25));
    circuit
        .append(
            Instruction::Standard(StandardGate::GPhase),
            Vec::<Qubit>::new(),
            [ParameterValue::Fixed(0.5)],
            None,
        )
        .unwrap();
    circuit.i(Qubit::new(0)).unwrap();
    circuit.rx(Qubit::new(1), 0.0).unwrap();
    circuit
        .barrier(vec![Qubit::new(2), Qubit::new(0), Qubit::new(1)])
        .unwrap();
    circuit
        .barrier(vec![Qubit::new(0), Qubit::new(2), Qubit::new(1)])
        .unwrap();
    circuit.h(Qubit::new(2)).unwrap();
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.h(Qubit::new(1))?;
                body.append(
                    Instruction::Standard(StandardGate::GPhase),
                    std::iter::empty::<Qubit>(),
                    [ParameterValue::Fixed(0.125)],
                    None,
                )
            },
            |body| {
                body.append(
                    Instruction::Standard(StandardGate::GPhase),
                    std::iter::empty::<Qubit>(),
                    [ParameterValue::Fixed(0.25)],
                    None,
                )?;
                body.i(Qubit::new(2))?;
                Ok(())
            },
        )
        .unwrap();

    let first = canonicalize_circuit(&circuit).unwrap();
    let second = canonicalize_circuit(&first.circuit).unwrap();

    assert!(first.changed);
    assert!(!second.changed);
    assert_eq!(
        first.circuit.operations().len(),
        second.circuit.operations().len()
    );
    assert_eq!(first.circuit.global_phase(), second.circuit.global_phase());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn canonicalization_is_idempotent_for_generated_small_circuits(
        circuit in generated_small_matrix_circuit()
    ) {
        let before = circuit_to_matrix(&circuit, None).unwrap();
        let first = canonicalize_circuit(&circuit).unwrap();
        let second = canonicalize_circuit(&first.circuit).unwrap();
        let after = circuit_to_matrix(&first.circuit, None).unwrap();

        prop_assert!(!second.changed);
        let before = before.as_slice().expect("matrix storage is contiguous");
        let after = after.as_slice().expect("matrix storage is contiguous");
        prop_assert_eq!(before.len(), after.len());
        for (index, (before, after)) in before.iter().zip(after).enumerate() {
            let diff = (*before - *after).norm();
            prop_assert!(
                diff <= 1e-10,
                "matrix entry {index} differs: before={before}, after={after}, diff={diff}"
            );
        }
    }
}
