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

use binding_c::circuit::{
    CClassicalExpr, CClassicalType, CClassicalValueInfo, CClassicalVar,
    CQLIB_CLASSICAL_EXPR_BINARY, CQLIB_CLASSICAL_EXPR_BIT_LITERAL,
    CQLIB_CLASSICAL_EXPR_BIT_VEC_LITERAL, CQLIB_CLASSICAL_EXPR_BOOL_LITERAL,
    CQLIB_CLASSICAL_EXPR_CAST, CQLIB_CLASSICAL_EXPR_COMPARE, CQLIB_CLASSICAL_EXPR_CONCAT,
    CQLIB_CLASSICAL_EXPR_EXTRACT_BIT, CQLIB_CLASSICAL_EXPR_EXTRACT_BITS,
    CQLIB_CLASSICAL_EXPR_PACK_BITS, CQLIB_CLASSICAL_EXPR_SELECT, CQLIB_CLASSICAL_EXPR_UINT_LITERAL,
    CQLIB_CLASSICAL_EXPR_UNARY, CQLIB_CLASSICAL_EXPR_VALUE, CQLIB_CLASSICAL_EXPR_VAR,
    CQLIB_CLASSICAL_TYPE_BIT, CQLIB_CLASSICAL_TYPE_BIT_VEC, CQLIB_CLASSICAL_TYPE_BOOL,
    CQLIB_CLASSICAL_TYPE_UINT, circuit_classical_vars, circuit_classical_vars_len, circuit_free,
    circuit_measure_bits, circuit_measure_bits_into, circuit_measure_into, circuit_new,
    circuit_num_operations, circuit_store, circuit_validate, circuit_validate_classical_expr,
    circuit_validate_classical_var, circuit_var, classical_expr_and, classical_expr_bit_literal,
    classical_expr_bit_to_bool, classical_expr_bit_vec_literal, classical_expr_bit_vec_to_uint,
    classical_expr_bool_literal, classical_expr_concat, classical_expr_eq,
    classical_expr_extract_bit, classical_expr_extract_bits, classical_expr_free,
    classical_expr_ge, classical_expr_gt, classical_expr_kind, classical_expr_le,
    classical_expr_lt, classical_expr_ne, classical_expr_not, classical_expr_or,
    classical_expr_pack_bits, classical_expr_select, classical_expr_simplified,
    classical_expr_to_bool, classical_expr_to_uint, classical_expr_ty, classical_expr_uint_literal,
    classical_expr_values, classical_expr_values_len, classical_expr_var, classical_expr_vars,
    classical_expr_vars_len, classical_expr_xor, classical_var_expr, classical_var_free,
    classical_var_id, classical_var_index, classical_var_ty,
};

/// Coerces an owned handle to the borrowed pointer type used by array inputs.
fn c(ptr: *mut CClassicalExpr) -> *const CClassicalExpr {
    ptr
}

fn expr_ty_parts(ptr: *const CClassicalExpr) -> (u32, u32) {
    let mut tag = u32::MAX;
    let mut width = u32::MAX;
    assert_eq!(classical_expr_ty(ptr, &mut tag, &mut width), 0);
    (tag, width)
}

fn expr_kind(ptr: *const CClassicalExpr) -> u32 {
    let mut kind = u32::MAX;
    assert_eq!(classical_expr_kind(ptr, &mut kind), 0);
    kind
}

#[test]
fn classical_var_lifecycle_and_queries() {
    let circuit = circuit_new(2);

    let bit = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    let boolean = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    let uint = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_UINT, 8);
    let bitvec = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT_VEC, 5);
    assert!(!bit.is_null());
    assert!(!boolean.is_null());
    assert!(!uint.is_null());
    assert!(!bitvec.is_null());

    assert_eq!(classical_var_id(bit), 0);
    assert_eq!(classical_var_index(bit), 0);
    assert_eq!(classical_var_id(bitvec), 3);

    let (mut tag, mut width) = (u32::MAX, u32::MAX);
    assert_eq!(classical_var_ty(bit, &mut tag, &mut width), 0);
    assert_eq!((tag, width), (CQLIB_CLASSICAL_TYPE_BIT, 1));
    assert_eq!(classical_var_ty(uint, &mut tag, &mut width), 0);
    assert_eq!((tag, width), (CQLIB_CLASSICAL_TYPE_UINT, 8));
    assert_eq!(classical_var_ty(bitvec, &mut tag, &mut width), 0);
    assert_eq!((tag, width), (CQLIB_CLASSICAL_TYPE_BIT_VEC, 5));

    // var -> expression
    let expr = classical_var_expr(uint);
    assert!(!expr.is_null());
    assert_eq!(expr_kind(expr), CQLIB_CLASSICAL_EXPR_VAR);
    assert_eq!(expr_ty_parts(expr), (CQLIB_CLASSICAL_TYPE_UINT, 8));

    // type table snapshot
    assert_eq!(circuit_classical_vars_len(circuit), 4);
    let mut buffer = [
        CClassicalType { tag: 9, width: 9 },
        CClassicalType { tag: 9, width: 9 },
        CClassicalType { tag: 9, width: 9 },
        CClassicalType { tag: 9, width: 9 },
    ];
    assert_eq!(circuit_classical_vars(circuit, buffer.as_mut_ptr(), 4), 0);
    assert_eq!(buffer[0].tag, CQLIB_CLASSICAL_TYPE_BIT);
    assert_eq!(buffer[0].width, 1);
    assert_eq!(buffer[2].tag, CQLIB_CLASSICAL_TYPE_UINT);
    assert_eq!(buffer[2].width, 8);
    assert_eq!(buffer[3].tag, CQLIB_CLASSICAL_TYPE_BIT_VEC);
    assert_eq!(buffer[3].width, 5);

    classical_expr_free(expr);
    classical_var_free(bit);
    classical_var_free(boolean);
    classical_var_free(uint);
    classical_var_free(bitvec);
    circuit_free(circuit);
}

#[test]
fn classical_var_error_paths() {
    let circuit = circuit_new(1);
    assert!(circuit_var(std::ptr::null_mut(), CQLIB_CLASSICAL_TYPE_BIT, 0).is_null());
    assert!(circuit_var(circuit, 99, 0).is_null()); // unknown tag
    assert!(circuit_var(circuit, CQLIB_CLASSICAL_TYPE_UINT, 0).is_null()); // zero width
    assert!(circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT_VEC, 0).is_null());

    assert_eq!(classical_var_id(std::ptr::null()), u32::MAX);
    assert_eq!(classical_var_index(std::ptr::null()), u32::MAX);
    assert_eq!(classical_var_ty(std::ptr::null(), &mut 0, &mut 0), -1);
    assert!(classical_var_expr(std::ptr::null()).is_null());
    classical_var_free(std::ptr::null_mut()); // no-op

    let var = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    let mut tag = 0u32;
    assert_eq!(classical_var_ty(var, std::ptr::null_mut(), &mut tag), -1);
    assert_eq!(classical_var_ty(var, &mut tag, std::ptr::null_mut()), -1);

    // len mismatch in type snapshot
    assert_eq!(circuit_classical_vars_len(circuit), 1);
    let mut buffer = [CClassicalType { tag: 0, width: 0 }];
    assert_eq!(circuit_classical_vars(circuit, buffer.as_mut_ptr(), 2), -8);
    assert_eq!(circuit_classical_vars(circuit, std::ptr::null_mut(), 1), -1);
    assert_eq!(
        circuit_classical_vars(std::ptr::null(), buffer.as_mut_ptr(), 1),
        -1
    );

    classical_var_free(var);
    circuit_free(circuit);
}

#[test]
fn measure_bits_returns_bitvec_value_expr() {
    let circuit = circuit_new(3);
    let qubits = [2u32, 0, 1];
    let expr = circuit_measure_bits(circuit, qubits.as_ptr(), 3);
    assert!(!expr.is_null());
    assert_eq!(expr_kind(expr), CQLIB_CLASSICAL_EXPR_VALUE);
    assert_eq!(expr_ty_parts(expr), (CQLIB_CLASSICAL_TYPE_BIT_VEC, 3));
    assert_eq!(classical_expr_vars_len(expr), 0);
    assert_eq!(classical_expr_values_len(expr), 1);

    let mut infos = [CClassicalValueInfo {
        index: u32::MAX,
        tag: u32::MAX,
        width: u32::MAX,
    }];
    assert_eq!(classical_expr_values(expr, infos.as_mut_ptr(), 1), 0);
    assert_eq!(infos[0].index, 0);
    assert_eq!(infos[0].tag, CQLIB_CLASSICAL_TYPE_BIT_VEC);
    assert_eq!(infos[0].width, 3);
    assert_eq!(classical_expr_values(expr, infos.as_mut_ptr(), 2), -8);
    assert_eq!(
        classical_expr_values(std::ptr::null(), infos.as_mut_ptr(), 1),
        -1
    );

    assert_eq!(circuit_validate_classical_expr(circuit, expr), 0);
    assert_eq!(circuit_num_operations(circuit), 1);

    classical_expr_free(expr);
    circuit_free(circuit);
}

#[test]
fn measure_error_paths() {
    let circuit = circuit_new(2);
    assert!(circuit_measure_bits(std::ptr::null_mut(), std::ptr::null(), 0).is_null());
    assert!(circuit_measure_bits(circuit, std::ptr::null(), 0).is_null()); // empty list
    let empty: [u32; 0] = [];
    assert!(circuit_measure_bits(circuit, empty.as_ptr(), 0).is_null());
    let oob = [0u32, 5];
    assert!(circuit_measure_bits(circuit, oob.as_ptr(), 2).is_null()); // qubit out of bounds

    let target = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    assert!(circuit_measure_into(std::ptr::null_mut(), 0, target).is_null());
    assert!(circuit_measure_into(circuit, 0, std::ptr::null()).is_null());
    assert!(circuit_measure_into(circuit, 9, target).is_null()); // out of bounds

    let bits_target = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT_VEC, 2);
    let qubits = [0u32, 1];
    let expr = circuit_measure_bits_into(circuit, qubits.as_ptr(), 2, bits_target);
    assert!(!expr.is_null());
    assert!(circuit_measure_bits_into(circuit, qubits.as_ptr(), 2, std::ptr::null()).is_null());
    assert!(
        circuit_measure_bits_into(std::ptr::null_mut(), qubits.as_ptr(), 2, bits_target).is_null()
    );

    classical_expr_free(expr);
    classical_var_free(bits_target);
    classical_var_free(target);
    circuit_free(circuit);
}

#[test]
fn measure_into_appends_measure_and_store() {
    let circuit = circuit_new(2);
    let target = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    let expr = circuit_measure_into(circuit, 0, target);
    assert!(!expr.is_null());
    assert_eq!(expr_kind(expr), CQLIB_CLASSICAL_EXPR_VALUE);
    assert_eq!(expr_ty_parts(expr), (CQLIB_CLASSICAL_TYPE_BIT, 1));
    assert_eq!(circuit_num_operations(circuit), 2); // measure + store
    assert_eq!(circuit_validate(circuit), 0);
    assert_eq!(circuit_validate_classical_var(circuit, target), 0);
    assert_eq!(circuit_validate_classical_expr(circuit, expr), 0);

    // wrong target type: Bool var cannot receive a Bit measurement
    let bad = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    assert!(circuit_measure_into(circuit, 1, bad).is_null());
    classical_var_free(bad);

    classical_expr_free(expr);
    classical_var_free(target);
    circuit_free(circuit);
}

#[test]
fn measure_bits_into_appends_measure_and_store() {
    let circuit = circuit_new(3);
    let target = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT_VEC, 2);
    let qubits = [1u32, 0];
    let expr = circuit_measure_bits_into(circuit, qubits.as_ptr(), 2, target);
    assert!(!expr.is_null());
    assert_eq!(expr_ty_parts(expr), (CQLIB_CLASSICAL_TYPE_BIT_VEC, 2));
    assert_eq!(circuit_num_operations(circuit), 2);
    assert_eq!(circuit_validate(circuit), 0);

    // width mismatch: 2-qubit measurement into a 3-wide variable
    let wide = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT_VEC, 3);
    assert!(circuit_measure_bits_into(circuit, qubits.as_ptr(), 2, wide).is_null());
    classical_var_free(wide);

    classical_expr_free(expr);
    classical_var_free(target);
    circuit_free(circuit);
}

#[test]
fn validate_rejects_foreign_handles() {
    let circuit_a = circuit_new(1);
    let circuit_b = circuit_new(1);
    let var_a = circuit_var(circuit_a, CQLIB_CLASSICAL_TYPE_BIT, 0);
    let expr_a = classical_var_expr(var_a);

    assert_eq!(circuit_validate_classical_var(circuit_a, var_a), 0);
    assert_eq!(circuit_validate_classical_var(circuit_b, var_a), -3);
    assert_eq!(circuit_validate_classical_var(std::ptr::null(), var_a), -1);
    assert_eq!(
        circuit_validate_classical_var(circuit_a, std::ptr::null()),
        -1
    );

    assert_eq!(circuit_validate_classical_expr(circuit_a, expr_a), 0);
    assert_eq!(circuit_validate_classical_expr(circuit_b, expr_a), -3);
    assert_eq!(
        circuit_validate_classical_expr(std::ptr::null(), expr_a),
        -1
    );
    assert_eq!(
        circuit_validate_classical_expr(circuit_a, std::ptr::null()),
        -1
    );

    classical_expr_free(expr_a);
    classical_var_free(var_a);
    circuit_free(circuit_a);
    circuit_free(circuit_b);
}

#[test]
fn literal_expressions() {
    let bit = classical_expr_bit_literal(true);
    assert_eq!(expr_kind(bit), CQLIB_CLASSICAL_EXPR_BIT_LITERAL);
    assert_eq!(expr_ty_parts(bit), (CQLIB_CLASSICAL_TYPE_BIT, 1));

    let boolean = classical_expr_bool_literal(false);
    assert_eq!(expr_kind(boolean), CQLIB_CLASSICAL_EXPR_BOOL_LITERAL);
    assert_eq!(expr_ty_parts(boolean), (CQLIB_CLASSICAL_TYPE_BOOL, 1));

    let uint = classical_expr_uint_literal(8, 42, 0);
    assert!(!uint.is_null());
    assert_eq!(expr_kind(uint), CQLIB_CLASSICAL_EXPR_UINT_LITERAL);
    assert_eq!(expr_ty_parts(uint), (CQLIB_CLASSICAL_TYPE_UINT, 8));

    // 128-bit literal split into lo/hi halves: 2^64
    let wide = classical_expr_uint_literal(128, 0, 1);
    assert!(!wide.is_null());
    assert_eq!(expr_ty_parts(wide), (CQLIB_CLASSICAL_TYPE_UINT, 128));

    let bitvec = classical_expr_bit_vec_literal(4, 0b1011, 0);
    assert!(!bitvec.is_null());
    assert_eq!(expr_kind(bitvec), CQLIB_CLASSICAL_EXPR_BIT_VEC_LITERAL);
    assert_eq!(expr_ty_parts(bitvec), (CQLIB_CLASSICAL_TYPE_BIT_VEC, 4));

    // invalid literals
    assert!(classical_expr_uint_literal(0, 0, 0).is_null()); // zero width
    assert!(classical_expr_uint_literal(8, 256, 0).is_null()); // value overflow
    assert!(classical_expr_uint_literal(129, 0, 0).is_null()); // width over 128
    assert!(classical_expr_bit_vec_literal(0, 0, 0).is_null());
    assert!(classical_expr_bit_vec_literal(3, 8, 0).is_null());

    classical_expr_free(bit);
    classical_expr_free(boolean);
    classical_expr_free(uint);
    classical_expr_free(wide);
    classical_expr_free(bitvec);
    classical_expr_free(std::ptr::null_mut()); // no-op
}

#[test]
fn logic_and_comparison_expressions() {
    let a = classical_expr_bool_literal(true);
    let b = classical_expr_bool_literal(false);
    let bit = classical_expr_bit_literal(true);
    let u8lit = classical_expr_uint_literal(8, 0, 0);

    let not = classical_expr_not(a);
    assert_eq!(expr_kind(not), CQLIB_CLASSICAL_EXPR_UNARY);
    assert_eq!(expr_ty_parts(not), (CQLIB_CLASSICAL_TYPE_BOOL, 1));

    let and = classical_expr_and(a, b);
    assert_eq!(expr_kind(and), CQLIB_CLASSICAL_EXPR_BINARY);
    assert_eq!(expr_ty_parts(and), (CQLIB_CLASSICAL_TYPE_BOOL, 1));

    let or = classical_expr_or(a, b);
    assert_eq!(expr_ty_parts(or), (CQLIB_CLASSICAL_TYPE_BOOL, 1));
    let xor = classical_expr_xor(bit, bit);
    assert_eq!(expr_ty_parts(xor), (CQLIB_CLASSICAL_TYPE_BIT, 1));

    // type mismatches
    assert!(classical_expr_and(a, bit).is_null()); // Bool vs Bit
    assert!(classical_expr_not(u8lit).is_null()); // UInt operand
    assert!(classical_expr_or(std::ptr::null(), b).is_null());
    assert!(classical_expr_xor(a, std::ptr::null()).is_null());

    let x = classical_expr_uint_literal(8, 10, 0);
    let y = classical_expr_uint_literal(8, 20, 0);
    type BinaryBuilder =
        extern "C" fn(*const CClassicalExpr, *const CClassicalExpr) -> *mut CClassicalExpr;
    let builders: [(BinaryBuilder, &str); 6] = [
        (classical_expr_eq, "eq"),
        (classical_expr_ne, "ne"),
        (classical_expr_lt, "lt"),
        (classical_expr_le, "le"),
        (classical_expr_gt, "gt"),
        (classical_expr_ge, "ge"),
    ];
    for (builder, name) in builders {
        let expr = builder(x, y);
        assert!(!expr.is_null(), "{name} must succeed on UInt operands");
        assert_eq!(expr_kind(expr), CQLIB_CLASSICAL_EXPR_COMPARE);
        assert_eq!(expr_ty_parts(expr), (CQLIB_CLASSICAL_TYPE_BOOL, 1));
        classical_expr_free(expr);
    }

    // eq works on all matching types; ordered comparisons need UInt
    let eq_bits = classical_expr_eq(bit, bit);
    assert!(!eq_bits.is_null());
    classical_expr_free(eq_bits);
    assert!(classical_expr_lt(a, b).is_null()); // Bool operands
    assert!(classical_expr_gt(a, b).is_null());
    assert!(classical_expr_eq(a, bit).is_null()); // type mismatch
    assert!(classical_expr_ge(std::ptr::null(), y).is_null());

    classical_expr_free(not);
    classical_expr_free(and);
    classical_expr_free(or);
    classical_expr_free(xor);
    classical_expr_free(a);
    classical_expr_free(b);
    classical_expr_free(bit);
    classical_expr_free(u8lit);
    classical_expr_free(x);
    classical_expr_free(y);
}

#[test]
fn select_extract_pack_concat_expressions() {
    let cond = classical_expr_bool_literal(true);
    let ten = classical_expr_uint_literal(8, 10, 0);
    let twenty = classical_expr_uint_literal(8, 20, 0);

    let selected = classical_expr_select(cond, ten, twenty);
    assert_eq!(expr_kind(selected), CQLIB_CLASSICAL_EXPR_SELECT);
    assert_eq!(expr_ty_parts(selected), (CQLIB_CLASSICAL_TYPE_UINT, 8));
    assert!(classical_expr_select(ten, ten, twenty).is_null()); // non-Bool condition
    assert!(classical_expr_select(cond, ten, cond).is_null()); // branch type mismatch
    assert!(classical_expr_select(std::ptr::null(), ten, twenty).is_null());

    let eighth = classical_expr_uint_literal(8, 0xFF, 0);
    let bit = classical_expr_extract_bit(eighth, 3);
    assert_eq!(expr_kind(bit), CQLIB_CLASSICAL_EXPR_EXTRACT_BIT);
    assert_eq!(expr_ty_parts(bit), (CQLIB_CLASSICAL_TYPE_BIT, 1));
    assert!(classical_expr_extract_bit(eighth, 8).is_null()); // out of bounds
    assert!(classical_expr_extract_bit(cond, 0).is_null()); // Bool operand
    assert!(classical_expr_extract_bit(std::ptr::null(), 0).is_null());

    let nibble = classical_expr_extract_bits(eighth, 4, 4);
    assert_eq!(expr_kind(nibble), CQLIB_CLASSICAL_EXPR_EXTRACT_BITS);
    assert_eq!(expr_ty_parts(nibble), (CQLIB_CLASSICAL_TYPE_BIT_VEC, 4));
    assert!(classical_expr_extract_bits(eighth, 4, 5).is_null()); // range overflow
    assert!(classical_expr_extract_bits(eighth, 0, 0).is_null()); // zero width

    let bit0 = classical_expr_bit_literal(true);
    let bit1 = classical_expr_bit_literal(false);
    let packed = classical_expr_pack_bits([c(bit0), c(bit1)].as_ptr(), 2);
    assert_eq!(expr_kind(packed), CQLIB_CLASSICAL_EXPR_PACK_BITS);
    assert_eq!(expr_ty_parts(packed), (CQLIB_CLASSICAL_TYPE_BIT_VEC, 2));
    let empty: [*const CClassicalExpr; 0] = [];
    assert!(classical_expr_pack_bits(empty.as_ptr(), 0).is_null());
    assert!(classical_expr_pack_bits([c(cond)].as_ptr(), 1).is_null()); // non-Bit operand
    let null_entry: [*const CClassicalExpr; 1] = [std::ptr::null()];
    assert!(classical_expr_pack_bits(null_entry.as_ptr(), 1).is_null());

    let low = classical_expr_bit_vec_literal(2, 0b01, 0);
    let concatenated = classical_expr_concat([c(low), c(bit0)].as_ptr(), 2);
    assert_eq!(expr_kind(concatenated), CQLIB_CLASSICAL_EXPR_CONCAT);
    assert_eq!(
        expr_ty_parts(concatenated),
        (CQLIB_CLASSICAL_TYPE_BIT_VEC, 3)
    );
    assert!(classical_expr_concat(empty.as_ptr(), 0).is_null());
    assert!(classical_expr_concat([c(cond)].as_ptr(), 1).is_null()); // Bool part

    classical_expr_free(selected);
    classical_expr_free(bit);
    classical_expr_free(nibble);
    classical_expr_free(packed);
    classical_expr_free(concatenated);
    classical_expr_free(cond);
    classical_expr_free(ten);
    classical_expr_free(twenty);
    classical_expr_free(eighth);
    classical_expr_free(bit0);
    classical_expr_free(bit1);
    classical_expr_free(low);
}

#[test]
fn cast_expressions() {
    let bit = classical_expr_bit_literal(true);
    let to_bool = classical_expr_bit_to_bool(bit);
    assert_eq!(expr_kind(to_bool), CQLIB_CLASSICAL_EXPR_CAST);
    assert_eq!(expr_ty_parts(to_bool), (CQLIB_CLASSICAL_TYPE_BOOL, 1));
    assert!(classical_expr_bit_to_bool(to_bool).is_null()); // Bool operand
    assert!(classical_expr_bit_to_bool(std::ptr::null()).is_null());

    let alias_bool = classical_expr_to_bool(bit);
    assert!(!alias_bool.is_null());
    assert_eq!(expr_ty_parts(alias_bool), (CQLIB_CLASSICAL_TYPE_BOOL, 1));

    let bitvec = classical_expr_bit_vec_literal(6, 0b101010, 0);
    let to_uint = classical_expr_bit_vec_to_uint(bitvec);
    assert_eq!(expr_ty_parts(to_uint), (CQLIB_CLASSICAL_TYPE_UINT, 6));
    assert!(classical_expr_bit_vec_to_uint(bit).is_null()); // Bit operand
    assert!(classical_expr_bit_vec_to_uint(std::ptr::null()).is_null());

    let alias_uint = classical_expr_to_uint(bitvec);
    assert!(!alias_uint.is_null());
    assert_eq!(expr_ty_parts(alias_uint), (CQLIB_CLASSICAL_TYPE_UINT, 6));

    classical_expr_free(to_bool);
    classical_expr_free(alias_bool);
    classical_expr_free(to_uint);
    classical_expr_free(alias_uint);
    classical_expr_free(bit);
    classical_expr_free(bitvec);
}

#[test]
fn simplified_expression_folds_literals() {
    // and(b, true) simplifies to b
    let b = classical_expr_bool_literal(false);
    let t = classical_expr_bool_literal(true);
    let expr = classical_expr_and(b, t);
    let simplified = classical_expr_simplified(expr);
    assert!(!simplified.is_null());
    assert_eq!(expr_kind(simplified), CQLIB_CLASSICAL_EXPR_BOOL_LITERAL);
    assert_eq!(expr_ty_parts(simplified), (CQLIB_CLASSICAL_TYPE_BOOL, 1));
    assert!(classical_expr_simplified(std::ptr::null()).is_null());

    // kind/ty error paths
    let mut tag = 0u32;
    assert_eq!(classical_expr_kind(std::ptr::null(), &mut tag), -1);
    assert_eq!(classical_expr_kind(b, std::ptr::null_mut()), -1);
    assert_eq!(classical_expr_ty(std::ptr::null(), &mut tag, &mut tag), -1);

    classical_expr_free(simplified);
    classical_expr_free(expr);
    classical_expr_free(b);
    classical_expr_free(t);
}

#[test]
fn vars_collection_returns_owned_handles() {
    let circuit = circuit_new(2);
    let a = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    let b = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    let expr_a = classical_expr_var(a);
    let expr_b = classical_var_expr(b);
    assert!(!expr_a.is_null());
    assert_eq!(expr_kind(expr_a), CQLIB_CLASSICAL_EXPR_VAR);
    assert!(classical_expr_var(std::ptr::null()).is_null());
    let combined = classical_expr_and(expr_a, expr_b);

    assert_eq!(classical_expr_vars_len(combined), 2);
    assert_eq!(classical_expr_vars_len(std::ptr::null()), 0);

    let mut handles: [*mut CClassicalVar; 2] = [std::ptr::null_mut(); 2];
    assert_eq!(classical_expr_vars(combined, handles.as_mut_ptr(), 2), 0);
    // BTreeSet order: ascending id, so a (id 0) then b (id 1)
    assert_eq!(classical_var_id(handles[0]), 0);
    assert_eq!(classical_var_id(handles[1]), 1);

    // collected handles behave like the originals
    assert_eq!(circuit_validate_classical_var(circuit, handles[0]), 0);
    let reread = classical_var_expr(handles[1]);
    assert_eq!(expr_kind(reread), CQLIB_CLASSICAL_EXPR_VAR);

    // error paths
    assert_eq!(classical_expr_vars(combined, handles.as_mut_ptr(), 1), -8);
    assert_eq!(
        classical_expr_vars(std::ptr::null(), handles.as_mut_ptr(), 2),
        -1
    );
    assert_eq!(classical_expr_vars(combined, std::ptr::null_mut(), 2), -1);

    classical_expr_free(reread);
    for handle in handles {
        classical_var_free(handle);
    }
    classical_expr_free(combined);
    classical_expr_free(expr_a);
    classical_expr_free(expr_b);
    classical_var_free(a);
    classical_var_free(b);
    circuit_free(circuit);
}

#[test]
fn circuit_store_roundtrip() {
    let circuit = circuit_new(2);

    // store a Bool literal into a Bool variable
    let flag = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    let value = classical_expr_bool_literal(true);
    assert_eq!(circuit_store(circuit, flag, value), 0);
    assert_eq!(circuit_num_operations(circuit), 1);
    assert_eq!(circuit_validate(circuit), 0);

    // store a measured bit (converted to Bool) into another Bool variable
    let bit_target = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    let measured = circuit_measure_into(circuit, 0, bit_target);
    assert!(!measured.is_null());
    let condition = classical_expr_bit_to_bool(measured);
    let other = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    assert_eq!(circuit_store(circuit, other, condition), 0);
    assert_eq!(circuit_num_operations(circuit), 4); // store + measure + store + store
    assert_eq!(circuit_validate(circuit), 0);

    // error paths
    assert_eq!(circuit_store(std::ptr::null_mut(), flag, value), -1);
    assert_eq!(circuit_store(circuit, std::ptr::null(), value), -1);
    assert_eq!(circuit_store(circuit, flag, std::ptr::null()), -1);
    // type mismatch: Bit variable cannot receive a Bool expression
    let bit_target2 = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    assert_eq!(circuit_store(circuit, bit_target2, value), -3);
    // foreign variable
    let other_circuit = circuit_new(1);
    let foreign = circuit_var(other_circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    assert_eq!(circuit_store(circuit, foreign, value), -3);

    classical_expr_free(measured);
    classical_expr_free(condition);
    classical_expr_free(value);
    classical_var_free(flag);
    classical_var_free(other);
    classical_var_free(bit_target);
    classical_var_free(bit_target2);
    classical_var_free(foreign);
    circuit_free(other_circuit);
    circuit_free(circuit);
}
