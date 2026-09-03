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

//! Structural simplification for classical expressions.
//!
//! This module provides bottom-up algebraic simplification of [`ClassicalExpr`]
//! ASTs. Simplification eliminates literal-driven redundancies and structural
//! identities without evaluating runtime variable or value reads.
//!
//! # Rules
//!
//! - **Double negation**: `not(not(x)) → x`
//! - **Identity elements**: `and(x, true) → x`, `or(x, false) → x`,
//!   `xor(x, false) → x` (and symmetric forms)
//! - **Idempotence**: `and(x, x) → x`, `or(x, x) → x`
//! - **Self-inverse**: `xor(x, x) → false`
//! - **Complement**: `and(x, not(x)) → false`, `or(x, not(x)) → true`,
//!   `xor(x, not(x)) → true`
//! - **Comparison reflexivity**: `eq(x, x) → true`, `ne(x, x) → false`,
//!   `lt(x, x) → false`, `gt(x, x) → false`, `le(x, x) → true`,
//!   `ge(x, x) → true`
//! - **Select folding**: `select(true, a, b) → a`,
//!   `select(false, a, b) → b`
//! - **Cast folding**: `bit_to_bool(BitLiteral(v)) → BoolLiteral(v)`,
//!   `bit_vec_to_uint(BitVecLiteral{w, v}) → UIntLiteral(w, v)`
//!
//! # Non-goals
//!
//! - Value-dependent constant folding (e.g. `and(x, false) → false` removes
//!   runtime variable reads and belongs to a higher-level optimization pass)
//! - Commutation normalization (e.g. `and(a, b) → and(b, a)`)
//! - Bit-width-aware extract/concat inversion (deferred)

use crate::circuit::classical::ClassicalType;
use crate::circuit::classical_expr::expr::{
    ClassicalBinaryOp, ClassicalCast, ClassicalCompareOp, ClassicalExpr, ClassicalExprKind,
    ClassicalUnaryOp,
};

/// Returns a structurally simplified copy of `expr`.
///
/// The result has the same static type as the input and reads a subset of the
/// input's runtime classical variables and values. Simplification is
/// idempotent: applying it twice yields the same expression.
///
/// Rules that require constructing a `UInt` or `BitVec` zero/one literal are
/// skipped when the type is wider than the current 128-bit literal
/// representation. In that case the original, type-correct expression shape
/// is preserved.
pub fn simplify(expr: &ClassicalExpr) -> ClassicalExpr {
    simplify_node(expr)
}

fn simplify_node(expr: &ClassicalExpr) -> ClassicalExpr {
    match expr.kind() {
        ClassicalExprKind::Var(_)
        | ClassicalExprKind::Value(_)
        | ClassicalExprKind::BoolLiteral(_)
        | ClassicalExprKind::BitLiteral(_)
        | ClassicalExprKind::UIntLiteral { .. }
        | ClassicalExprKind::BitVecLiteral { .. } => expr.clone(),

        ClassicalExprKind::Unary { op, expr: inner } => {
            let inner = simplify_node(inner);
            simplify_unary(*op, &inner).unwrap_or_else(|| {
                ClassicalExpr::from_kind(
                    inner.ty(),
                    ClassicalExprKind::Unary {
                        op: *op,
                        expr: inner,
                    },
                )
            })
        }

        ClassicalExprKind::Binary { op, lhs, rhs } => {
            let lhs = simplify_node(lhs);
            let rhs = simplify_node(rhs);
            simplify_binary(*op, &lhs, &rhs).unwrap_or_else(|| {
                let ty = lhs.ty();
                ClassicalExpr::from_kind(ty, ClassicalExprKind::Binary { op: *op, lhs, rhs })
            })
        }

        ClassicalExprKind::Compare { op, lhs, rhs } => {
            let lhs = simplify_node(lhs);
            let rhs = simplify_node(rhs);
            simplify_compare(*op, &lhs, &rhs).unwrap_or_else(|| {
                ClassicalExpr::from_kind(
                    ClassicalType::Bool,
                    ClassicalExprKind::Compare { op: *op, lhs, rhs },
                )
            })
        }

        ClassicalExprKind::Cast { cast, expr: inner } => {
            let inner = simplify_node(inner);
            simplify_cast(*cast, &inner).unwrap_or_else(|| {
                let ty = match cast {
                    ClassicalCast::BitToBool => ClassicalType::Bool,
                    ClassicalCast::BitVecToUInt => {
                        if let ClassicalType::BitVec(width) = inner.ty() {
                            ClassicalType::UInt(width)
                        } else {
                            // Unreachable: cast constructors validate types
                            ClassicalType::Bool
                        }
                    }
                };
                ClassicalExpr::from_kind(
                    ty,
                    ClassicalExprKind::Cast {
                        cast: *cast,
                        expr: inner,
                    },
                )
            })
        }

        ClassicalExprKind::Select {
            condition,
            then_expr,
            else_expr,
        } => {
            let condition = simplify_node(condition);
            // Only fold when the condition resolves to a literal.
            if let Some(result) = simplify_select(&condition, then_expr, else_expr) {
                return result;
            }
            let then_expr = simplify_node(then_expr);
            let else_expr = simplify_node(else_expr);
            ClassicalExpr::from_kind(
                then_expr.ty(),
                ClassicalExprKind::Select {
                    condition,
                    then_expr,
                    else_expr,
                },
            )
        }

        ClassicalExprKind::ExtractBit { value, index } => {
            let value = simplify_node(value);
            ClassicalExpr::from_kind(
                ClassicalType::Bit,
                ClassicalExprKind::ExtractBit {
                    value,
                    index: *index,
                },
            )
        }

        ClassicalExprKind::ExtractBits {
            value,
            offset,
            width,
        } => {
            let value = simplify_node(value);
            ClassicalExpr::from_kind(
                ClassicalType::BitVec(*width),
                ClassicalExprKind::ExtractBits {
                    value,
                    offset: *offset,
                    width: *width,
                },
            )
        }

        ClassicalExprKind::Concat { parts } => {
            let mut changed = false;
            let simplified: Vec<ClassicalExpr> = parts
                .iter()
                .map(|part| {
                    let s = simplify_node(part);
                    if !changed && &s != part {
                        changed = true;
                    }
                    s
                })
                .collect();
            if !changed {
                return expr.clone();
            }
            let total_width = simplified
                .iter()
                .fold(0u32, |acc, part| acc.saturating_add(part.ty().width()));
            let Some(width) = std::num::NonZeroU32::new(total_width) else {
                // Unreachable: concat requires at least one part
                return expr.clone();
            };
            ClassicalExpr::from_kind(
                ClassicalType::BitVec(width),
                ClassicalExprKind::Concat {
                    parts: simplified.into_boxed_slice(),
                },
            )
        }

        ClassicalExprKind::PackBits { bits } => {
            let mut changed = false;
            let simplified: Vec<ClassicalExpr> = bits
                .iter()
                .map(|bit| {
                    let s = simplify_node(bit);
                    if !changed && &s != bit {
                        changed = true;
                    }
                    s
                })
                .collect();
            if !changed {
                return expr.clone();
            }
            let Some(width) = std::num::NonZeroU32::new(simplified.len() as u32) else {
                return expr.clone();
            };
            ClassicalExpr::from_kind(
                ClassicalType::BitVec(width),
                ClassicalExprKind::PackBits {
                    bits: simplified.into_boxed_slice(),
                },
            )
        }
    }
}

/// R1: `not(not(x))` → `x`
fn simplify_unary(op: ClassicalUnaryOp, child: &ClassicalExpr) -> Option<ClassicalExpr> {
    if op != ClassicalUnaryOp::Not {
        return None;
    }
    match child.kind() {
        ClassicalExprKind::Unary {
            op: ClassicalUnaryOp::Not,
            expr: grandchild,
        } => Some(grandchild.clone()),
        _ => None,
    }
}

/// R2-R13: identity elements, idempotence, self-inverse, complement
fn simplify_binary(
    op: ClassicalBinaryOp,
    lhs: &ClassicalExpr,
    rhs: &ClassicalExpr,
) -> Option<ClassicalExpr> {
    let ty = lhs.ty();

    match op {
        ClassicalBinaryOp::And => {
            if rhs.is_bool_true() || rhs.is_bit_true() {
                return Some(lhs.clone());
            }
            if lhs.is_bool_true() || lhs.is_bit_true() {
                return Some(rhs.clone());
            }
        }
        ClassicalBinaryOp::Or => {
            if rhs.is_bool_false() || rhs.is_bit_false() {
                return Some(lhs.clone());
            }
            if lhs.is_bool_false() || lhs.is_bit_false() {
                return Some(rhs.clone());
            }
        }
        ClassicalBinaryOp::Xor => {
            if rhs.is_bool_false() || rhs.is_bit_false() {
                return Some(lhs.clone());
            }
            if lhs.is_bool_false() || lhs.is_bit_false() {
                return Some(rhs.clone());
            }
        }
    }

    if lhs == rhs {
        match op {
            ClassicalBinaryOp::And | ClassicalBinaryOp::Or => {
                return Some(lhs.clone());
            }
            ClassicalBinaryOp::Xor => {
                return ty.zero_literal().ok();
            }
        }
    }

    // ── complement: and(x, not(x)) → false, or(x, not(x)) → true,
    //     xor(x, not(x)) → true. Check both lhs/rhs directions. ────
    for (a, b) in [(lhs, rhs), (rhs, lhs)] {
        if let ClassicalExprKind::Unary {
            op: ClassicalUnaryOp::Not,
            expr: x,
        } = b.kind()
            && a == x
        {
            return match op {
                ClassicalBinaryOp::And => ty.zero_literal(),
                ClassicalBinaryOp::Or | ClassicalBinaryOp::Xor => ty.one_literal(),
            }
            .ok();
        }
    }

    None
}

/// R14-R19: comparison reflexivity
fn simplify_compare(
    op: ClassicalCompareOp,
    lhs: &ClassicalExpr,
    rhs: &ClassicalExpr,
) -> Option<ClassicalExpr> {
    if lhs != rhs {
        return None;
    }
    match op {
        ClassicalCompareOp::Eq | ClassicalCompareOp::Le | ClassicalCompareOp::Ge => {
            Some(ClassicalExpr::bool_literal(true))
        }
        ClassicalCompareOp::Ne | ClassicalCompareOp::Lt | ClassicalCompareOp::Gt => {
            Some(ClassicalExpr::bool_literal(false))
        }
    }
}

/// R20-R21: `select(true, a, b) → a`, `select(false, a, b) → b`
fn simplify_select(
    condition: &ClassicalExpr,
    then_expr: &ClassicalExpr,
    else_expr: &ClassicalExpr,
) -> Option<ClassicalExpr> {
    match condition.kind() {
        ClassicalExprKind::BoolLiteral(true) => Some(simplify_node(then_expr)),
        ClassicalExprKind::BoolLiteral(false) => Some(simplify_node(else_expr)),
        _ => None,
    }
}

/// R22-R23: cast-of-literal folding
fn simplify_cast(cast: ClassicalCast, child: &ClassicalExpr) -> Option<ClassicalExpr> {
    match (cast, child.kind()) {
        (ClassicalCast::BitToBool, ClassicalExprKind::BitLiteral(v)) => {
            Some(ClassicalExpr::bool_literal(*v))
        }
        (ClassicalCast::BitVecToUInt, ClassicalExprKind::BitVecLiteral { width, value }) => Some(
            ClassicalExpr::uint_literal(width.get(), *value)
                .expect("bit-vec literal value must be valid for uint of same width"),
        ),
        _ => None,
    }
}

#[cfg(test)]
#[path = "simplify_test.rs"]
mod simplify_test;
