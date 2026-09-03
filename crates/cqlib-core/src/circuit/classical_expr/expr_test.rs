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

use super::{ClassicalExpr, ClassicalExprKind};
use crate::circuit::{CircuitId, ClassicalType, ClassicalValue, ClassicalVar};
use std::sync::OnceLock;

fn test_circuit_id() -> CircuitId {
    static ID: OnceLock<CircuitId> = OnceLock::new();
    *ID.get_or_init(CircuitId::new)
}
use std::collections::HashMap;

#[test]
fn literals_have_static_types_and_validate_widths() {
    assert_eq!(ClassicalExpr::bool_literal(true).ty(), ClassicalType::Bool);
    assert_eq!(ClassicalExpr::bit_literal(false).ty(), ClassicalType::Bit);
    assert_eq!(
        ClassicalExpr::uint_literal(8, 255).unwrap().ty(),
        ClassicalType::uint(8).unwrap()
    );
    assert_eq!(
        ClassicalExpr::bit_vec_literal(3, 0b101).unwrap().ty(),
        ClassicalType::bit_vec(3).unwrap()
    );

    assert!(ClassicalExpr::uint_literal(0, 0).is_err());
    assert!(ClassicalExpr::uint_literal(129, 0).is_err());
    assert!(ClassicalExpr::uint_literal(3, 8).is_err());
}

#[test]
fn boolean_and_bit_operations_are_typed_separately() {
    let b0 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 0, ClassicalType::Bool));
    let b1 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 1, ClassicalType::Bool));
    assert_eq!(
        ClassicalExpr::try_and(b0.clone(), b1.clone()).unwrap().ty(),
        ClassicalType::Bool
    );
    assert_eq!(
        ClassicalExpr::try_not(b0).unwrap().ty(),
        ClassicalType::Bool
    );

    let bit0 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 2, ClassicalType::Bit));
    let bit1 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 3, ClassicalType::Bit));
    assert_eq!(
        ClassicalExpr::try_xor(bit0.clone(), bit1).unwrap().ty(),
        ClassicalType::Bit
    );

    assert!(ClassicalExpr::try_and(b1, bit0).is_err());
}

#[test]
fn rust_bit_operators_build_boolean_and_bit_expressions() {
    let b0 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 0, ClassicalType::Bool));
    let b1 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 1, ClassicalType::Bool));

    assert_eq!((!b0.clone()).ty(), ClassicalType::Bool);
    assert_eq!((b0.clone() & b1.clone()).ty(), ClassicalType::Bool);
    assert_eq!((b0.clone() | b1.clone()).ty(), ClassicalType::Bool);
    assert_eq!((b0 ^ b1).ty(), ClassicalType::Bool);

    let bit0 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 2, ClassicalType::Bit));
    let bit1 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 3, ClassicalType::Bit));

    assert_eq!((!bit0.clone()).ty(), ClassicalType::Bit);
    assert_eq!((bit0.clone() & bit1.clone()).ty(), ClassicalType::Bit);
    assert_eq!((bit0.clone() | bit1.clone()).ty(), ClassicalType::Bit);
    assert_eq!((bit0 ^ bit1).ty(), ClassicalType::Bit);
}

#[test]
fn classical_handles_convert_directly_to_expressions() {
    let var = ClassicalVar::new(test_circuit_id(), 0, ClassicalType::Bool);
    let value = ClassicalValue::new(test_circuit_id(), 1, ClassicalType::Bit);

    let var_expr = ClassicalExpr::from(var);
    let value_expr = ClassicalExpr::from(value);

    assert_eq!(var_expr.ty(), ClassicalType::Bool);
    assert_eq!(value_expr.ty(), ClassicalType::Bit);
}

#[test]
fn classical_handles_can_use_expression_operators_directly() {
    let flag = ClassicalVar::new(test_circuit_id(), 0, ClassicalType::Bool);
    let ready = ClassicalVar::new(test_circuit_id(), 1, ClassicalType::Bool);
    let value = ClassicalValue::new(test_circuit_id(), 2, ClassicalType::Bool);

    assert_eq!((!flag).ty(), ClassicalType::Bool);
    assert_eq!((flag & ready).ty(), ClassicalType::Bool);
    assert_eq!((flag | ready.expr()).ty(), ClassicalType::Bool);
    assert_eq!((flag.expr() ^ ready).ty(), ClassicalType::Bool);
    assert_eq!((flag & value).ty(), ClassicalType::Bool);

    let bit_var = ClassicalVar::new(test_circuit_id(), 3, ClassicalType::Bit);
    let bit_value = ClassicalValue::new(test_circuit_id(), 4, ClassicalType::Bit);

    assert_eq!((!bit_value).ty(), ClassicalType::Bit);
    assert_eq!((bit_var & bit_value).ty(), ClassicalType::Bit);
}

#[test]
fn fallible_logical_builders_accept_classical_handles() {
    let flag = ClassicalVar::new(test_circuit_id(), 0, ClassicalType::Bool);
    let ready = ClassicalVar::new(test_circuit_id(), 1, ClassicalType::Bool);
    let measured = ClassicalValue::new(test_circuit_id(), 2, ClassicalType::Bit);

    assert_eq!(
        ClassicalExpr::try_and(flag, ready).unwrap().ty(),
        ClassicalType::Bool
    );
    assert!(ClassicalExpr::try_and(flag, measured).is_err());
}

#[test]
fn comparisons_return_bool_and_enforce_ordered_uints() {
    let bit0 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 0, ClassicalType::Bit));
    let bit1 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 1, ClassicalType::Bit));
    assert_eq!(
        ClassicalExpr::eq(bit0.clone(), bit1.clone()).unwrap().ty(),
        ClassicalType::Bool
    );
    assert!(ClassicalExpr::lt(bit0, bit1).is_err());

    let u0 = ClassicalExpr::var(ClassicalVar::new(
        test_circuit_id(),
        2,
        ClassicalType::uint(4).unwrap(),
    ));
    let u1 = ClassicalExpr::var(ClassicalVar::new(
        test_circuit_id(),
        3,
        ClassicalType::uint(4).unwrap(),
    ));
    assert_eq!(ClassicalExpr::ge(u0, u1).unwrap().ty(), ClassicalType::Bool);
}

#[test]
fn casts_are_explicit() {
    let bit = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 0, ClassicalType::Bit));
    assert_eq!(
        ClassicalExpr::bit_to_bool(bit).unwrap().ty(),
        ClassicalType::Bool
    );
    let bit = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 3, ClassicalType::Bit));
    assert_eq!(bit.to_bool().unwrap().ty(), ClassicalType::Bool);

    let bits = ClassicalExpr::var(ClassicalVar::new(
        test_circuit_id(),
        1,
        ClassicalType::bit_vec(5).unwrap(),
    ));
    assert_eq!(
        ClassicalExpr::bit_vec_to_uint(bits).unwrap().ty(),
        ClassicalType::uint(5).unwrap()
    );
    let bits = ClassicalExpr::var(ClassicalVar::new(
        test_circuit_id(),
        4,
        ClassicalType::bit_vec(5).unwrap(),
    ));
    assert_eq!(
        bits.to_uint().unwrap().ty(),
        ClassicalType::uint(5).unwrap()
    );

    assert!(ClassicalExpr::bit_to_bool(ClassicalExpr::bool_literal(true)).is_err());
    assert!(ClassicalExpr::bool_literal(true).to_bool().is_err());
    assert!(
        ClassicalExpr::bit_vec_to_uint(ClassicalExpr::var(ClassicalVar::new(
            test_circuit_id(),
            2,
            ClassicalType::uint(5).unwrap()
        )))
        .is_err()
    );
    assert!(
        ClassicalExpr::var(ClassicalVar::new(
            test_circuit_id(),
            5,
            ClassicalType::uint(5).unwrap()
        ))
        .to_uint()
        .is_err()
    );
}

#[test]
fn select_requires_bool_condition_and_matching_branch_types() {
    let condition = ClassicalExpr::bool_literal(true);
    let then_expr = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 0, ClassicalType::Bit));
    let else_expr = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 1, ClassicalType::Bit));

    assert_eq!(
        ClassicalExpr::select(condition, then_expr, else_expr)
            .unwrap()
            .ty(),
        ClassicalType::Bit
    );

    assert!(
        ClassicalExpr::select(
            ClassicalExpr::bit_literal(true),
            ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 2, ClassicalType::Bit)),
            ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 3, ClassicalType::Bit)),
        )
        .is_err()
    );
    assert!(
        ClassicalExpr::select(
            ClassicalExpr::bool_literal(true),
            ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 4, ClassicalType::Bit)),
            ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 5, ClassicalType::Bool)),
        )
        .is_err()
    );
}

#[test]
fn extraction_uses_little_endian_indices() {
    let value = ClassicalExpr::var(ClassicalVar::new(
        test_circuit_id(),
        0,
        ClassicalType::bit_vec(8).unwrap(),
    ));
    assert_eq!(
        ClassicalExpr::extract_bit(value.clone(), 0).unwrap().ty(),
        ClassicalType::Bit
    );
    assert_eq!(
        ClassicalExpr::extract_bits(value.clone(), 2, 3)
            .unwrap()
            .ty(),
        ClassicalType::bit_vec(3).unwrap()
    );

    assert!(ClassicalExpr::extract_bit(value.clone(), 8).is_err());
    assert!(ClassicalExpr::extract_bits(value, 7, 2).is_err());
}

#[test]
fn pack_bits_and_concat_build_bit_vectors() {
    let bit0 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 0, ClassicalType::Bit));
    let bit1 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 1, ClassicalType::Bit));
    let packed = ClassicalExpr::pack_bits([bit0.clone(), bit1.clone()]).unwrap();
    assert_eq!(packed.ty(), ClassicalType::bit_vec(2).unwrap());

    let vec3 = ClassicalExpr::var(ClassicalVar::new(
        test_circuit_id(),
        2,
        ClassicalType::bit_vec(3).unwrap(),
    ));
    let concat = ClassicalExpr::concat([bit0, vec3]).unwrap();
    assert_eq!(concat.ty(), ClassicalType::bit_vec(4).unwrap());

    assert!(ClassicalExpr::pack_bits([ClassicalExpr::bool_literal(true)]).is_err());
    assert!(ClassicalExpr::concat([ClassicalExpr::bool_literal(true)]).is_err());
    assert!(ClassicalExpr::concat(std::iter::empty()).is_err());
}

#[test]
fn measurement_store_bit_recognizes_single_bit_updates() {
    let target = ClassicalVar::new(test_circuit_id(), 0, ClassicalType::bit_vec(3).unwrap());
    let result = ClassicalValue::new(test_circuit_id(), 0, ClassicalType::Bit);
    let target_expr = target.expr();
    let preserved = |index| ClassicalExpr::extract_bit(target_expr.clone(), index).unwrap();

    let store = ClassicalExpr::pack_bits([preserved(0), result.expr(), preserved(2)]).unwrap();
    assert_eq!(store.measurement_store_bit(target, result), Some(1));

    let duplicated =
        ClassicalExpr::pack_bits([result.expr(), result.expr(), preserved(2)]).unwrap();
    assert_eq!(duplicated.measurement_store_bit(target, result), None);

    let wrong_source =
        ClassicalExpr::pack_bits([preserved(1), result.expr(), preserved(2)]).unwrap();
    assert_eq!(wrong_source.measurement_store_bit(target, result), None);

    let wrong_width = ClassicalExpr::pack_bits([preserved(0), result.expr()]).unwrap();
    assert_eq!(wrong_width.measurement_store_bit(target, result), None);
}

#[test]
fn variables_are_collected_recursively() {
    let bit0 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 0, ClassicalType::Bit));
    let bit1 = ClassicalExpr::var(ClassicalVar::new(test_circuit_id(), 1, ClassicalType::Bit));
    let condition = ClassicalExpr::bit_to_bool(bit0.clone()).unwrap();
    let expr = ClassicalExpr::select(condition, bit0, bit1).unwrap();

    let vars = expr.vars();
    assert_eq!(vars.len(), 2);
    assert!(vars.contains(&ClassicalVar::new(test_circuit_id(), 0, ClassicalType::Bit)));
    assert!(vars.contains(&ClassicalVar::new(test_circuit_id(), 1, ClassicalType::Bit)));
}

#[test]
fn remap_classical_ids_rewrites_nested_var_and_value_reads() {
    let old_var = ClassicalVar::new(test_circuit_id(), 0, ClassicalType::Bit);
    let new_var = ClassicalVar::new(test_circuit_id(), 7, ClassicalType::Bit);
    let old_value = ClassicalValue::new(test_circuit_id(), 0, ClassicalType::Bit);
    let new_value = ClassicalValue::new(test_circuit_id(), 5, ClassicalType::Bit);

    let condition = ClassicalExpr::bit_to_bool(old_value.expr()).unwrap();
    let packed = ClassicalExpr::pack_bits([old_var.expr(), old_value.expr()]).unwrap();
    let extracted = ClassicalExpr::extract_bit(packed, 1).unwrap();
    let expr = ClassicalExpr::select(condition, old_var.expr(), extracted).unwrap();

    let var_map = HashMap::from([(old_var, new_var)]);
    let value_map = HashMap::from([(old_value, new_value)]);
    let remapped = expr.remap_classical_ids(&var_map, &value_map).unwrap();

    assert_eq!(remapped.ty(), expr.ty());
    assert!(matches!(remapped.kind(), ClassicalExprKind::Select { .. }));
    assert!(remapped.vars().contains(&new_var));
    assert!(!remapped.vars().contains(&old_var));
    assert!(remapped.values().contains(&new_value));
    assert!(!remapped.values().contains(&old_value));
}

#[test]
fn remap_classical_ids_requires_every_referenced_handle() {
    let expr = ClassicalValue::new(test_circuit_id(), 0, ClassicalType::Bit).expr();
    let var_map = HashMap::new();
    let value_map = HashMap::new();

    assert!(expr.remap_classical_ids(&var_map, &value_map).is_err());
}
