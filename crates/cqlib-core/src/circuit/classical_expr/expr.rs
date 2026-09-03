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

//! Typed, side-effect-free runtime classical expressions.
//!
//! Expressions read [`ClassicalVar`] and [`ClassicalValue`] handles and combine
//! them into typed runtime values. They do not measure qubits, write classical
//! storage, or transfer control. Control-flow statements consume these
//! expressions in a later layer.
//!
//! Build expressions with the typed constructors on [`ClassicalExpr`].
//! [`ClassicalVar`] and [`ClassicalValue`] handles can be converted directly
//! into expressions, so `var.expr()`, `ClassicalExpr::from(var)`, and passing a
//! handle to a `try_*` builder are equivalent expression sources. Leaf
//! constructors such as [`ClassicalExpr::bool_literal`] and
//! [`ClassicalExpr::bit_literal`] are infallible. Constructors that validate
//! operand types return `Result<ClassicalExpr, CircuitError>`; the fallible
//! logical builders use a `try_` prefix, for example
//! [`ClassicalExpr::try_not`], [`ClassicalExpr::try_and`],
//! [`ClassicalExpr::try_or`], and [`ClassicalExpr::try_xor`].
//!
//! Prefer Rust bit operators when the expression types are already known:
//! `!source`, `lhs & rhs`, `lhs | rhs`, and `lhs ^ rhs`. Each source may be a
//! [`ClassicalExpr`], [`ClassicalVar`], or [`ClassicalValue`]. These operator
//! overloads panic if the operands are not matching `Bool` or matching `Bit`
//! expressions, because the standard operator traits cannot return `Result`.
//! Use the fallible `try_*` builders when invalid input should be reported as
//! [`CircuitError`] instead of a panic.
//!
//! ```rust
//! use cqlib_core::circuit::{CircuitId, ClassicalExpr, ClassicalType, ClassicalVar};
//!
//! let cid = CircuitId::new();
//! let a = ClassicalVar::new(cid, 0, ClassicalType::Bool);
//! let b = ClassicalVar::new(cid, 1, ClassicalType::Bool);
//!
//! let condition = !a & b;
//! assert_eq!(condition.ty(), ClassicalType::Bool);
//!
//! let checked = ClassicalExpr::try_and(ClassicalExpr::try_not(a)?, b)?;
//! assert_eq!(checked.ty(), ClassicalType::Bool);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::circuit::classical::{ClassicalType, ClassicalValue, ClassicalVar};
use crate::circuit::error::CircuitError;
use std::collections::{BTreeSet, HashMap};
use std::num::NonZeroU32;
use std::ops::{BitAnd, BitOr, BitXor, Not};

/// Unary operators for classical expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClassicalUnaryOp {
    /// Logical negation for `Bool`, or bit inversion for `Bit`.
    Not,
}

/// Binary operators for boolean and bit expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClassicalBinaryOp {
    /// Logical or bitwise conjunction.
    And,
    /// Logical or bitwise disjunction.
    Or,
    /// Logical or bitwise exclusive-or.
    Xor,
}

/// Comparison operators for classical expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClassicalCompareOp {
    /// Equality.
    Eq,
    /// Inequality.
    Ne,
    /// Strictly less than.
    Lt,
    /// Less than or equal.
    Le,
    /// Strictly greater than.
    Gt,
    /// Greater than or equal.
    Ge,
}

/// Explicit casts between classical expression types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClassicalCast {
    /// Interprets a runtime bit as a runtime boolean.
    BitToBool,
    /// Interprets an ordered bit-vector as a little-endian unsigned integer.
    BitVecToUInt,
}

/// A typed, side-effect-free runtime classical expression.
///
/// The expression node kind is exposed through [`ClassicalExpr::kind`] for
/// inspection and tooling. Prefer the typed constructors on this type when
/// building expressions; they centralize type checks and keep the AST valid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassicalExpr {
    node: Box<ClassicalExprNode>,
}

/// Typed node stored inside a [`ClassicalExpr`].
///
/// Its fields are private so callers cannot construct an expression whose
/// declared type disagrees with its node kind.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassicalExprNode {
    ty: ClassicalType,
    kind: ClassicalExprKind,
}

/// Public node kind for inspecting a [`ClassicalExpr`] AST.
///
/// The variants mirror the expression builders on [`ClassicalExpr`]. External
/// callers can pattern-match this enum through [`ClassicalExpr::kind`], while
/// expression construction should continue to use the typed builders.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClassicalExprKind {
    /// Read a mutable circuit-local variable.
    Var(ClassicalVar),
    /// Read an immutable circuit-local SSA value.
    Value(ClassicalValue),
    /// Boolean literal.
    BoolLiteral(bool),
    /// Single-bit literal.
    BitLiteral(bool),
    /// Fixed-width unsigned integer literal.
    UIntLiteral {
        /// Static integer width.
        width: NonZeroU32,
        /// Literal value, constrained to the declared width.
        value: u128,
    },
    /// Fixed-width ordered bit-vector literal.
    BitVecLiteral {
        /// Static bit-vector width.
        width: NonZeroU32,
        /// Packed little-endian literal value.
        value: u128,
    },
    /// Unary operation.
    Unary {
        /// Unary operator.
        op: ClassicalUnaryOp,
        /// Operand expression.
        expr: ClassicalExpr,
    },
    /// Binary boolean or bit operation.
    Binary {
        /// Binary operator.
        op: ClassicalBinaryOp,
        /// Left operand.
        lhs: ClassicalExpr,
        /// Right operand.
        rhs: ClassicalExpr,
    },
    /// Comparison producing a boolean result.
    Compare {
        /// Comparison operator.
        op: ClassicalCompareOp,
        /// Left operand.
        lhs: ClassicalExpr,
        /// Right operand.
        rhs: ClassicalExpr,
    },
    /// Explicit conversion between classical types.
    Cast {
        /// Conversion operation.
        cast: ClassicalCast,
        /// Source expression.
        expr: ClassicalExpr,
    },
    /// Conditional expression analogous to a typed ternary operator.
    Select {
        /// Boolean selection condition.
        condition: ClassicalExpr,
        /// Value selected when the condition is true.
        then_expr: ClassicalExpr,
        /// Value selected when the condition is false.
        else_expr: ClassicalExpr,
    },
    /// Extract one bit from an integer or bit-vector expression.
    ExtractBit {
        /// Source expression.
        value: ClassicalExpr,
        /// Zero-based little-endian bit index.
        index: u32,
    },
    /// Extract a contiguous bit range.
    ExtractBits {
        /// Source expression.
        value: ClassicalExpr,
        /// Zero-based little-endian starting offset.
        offset: u32,
        /// Number of bits in the result.
        width: NonZeroU32,
    },
    /// Concatenate bit-vector expressions in the documented builder order.
    Concat {
        /// Expressions being concatenated.
        parts: Box<[ClassicalExpr]>,
    },
    /// Pack individual `Bit` expressions into a bit vector.
    PackBits {
        /// Bits in increasing little-endian bit-index order.
        bits: Box<[ClassicalExpr]>,
    },
}

impl ClassicalExpr {
    /// Creates an expression that reads the current runtime value of `var`.
    pub fn var(var: ClassicalVar) -> Self {
        Self {
            node: Box::new(ClassicalExprNode {
                ty: var.ty(),
                kind: ClassicalExprKind::Var(var),
            }),
        }
    }

    /// Creates an expression that reads an immutable runtime classical value.
    pub fn value(value: ClassicalValue) -> Self {
        Self {
            node: Box::new(ClassicalExprNode {
                ty: value.ty(),
                kind: ClassicalExprKind::Value(value),
            }),
        }
    }

    /// Creates a boolean literal expression.
    pub fn bool_literal(value: bool) -> Self {
        Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::Bool,
                kind: ClassicalExprKind::BoolLiteral(value),
            }),
        }
    }

    /// Creates a bit literal expression.
    pub fn bit_literal(value: bool) -> Self {
        Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::Bit,
                kind: ClassicalExprKind::BitLiteral(value),
            }),
        }
    }

    /// Creates an unsigned integer literal with a non-zero width.
    ///
    /// Literal construction currently supports constants up to 128 bits wide.
    pub fn uint_literal(width: u32, value: u128) -> Result<Self, CircuitError> {
        let Some(width) = NonZeroU32::new(width) else {
            return Err(CircuitError::InvalidOperation(
                "UInt literal width must be non-zero".to_string(),
            ));
        };
        if width.get() > 128 {
            return Err(CircuitError::InvalidOperation(format!(
                "UInt literal width {} exceeds the 128-bit literal limit",
                width.get()
            )));
        }
        if width.get() < 128 && value >= (1u128 << width.get()) {
            return Err(CircuitError::InvalidOperation(format!(
                "UInt literal value {value} does not fit in width {}",
                width.get()
            )));
        }
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::UInt(width),
                kind: ClassicalExprKind::UIntLiteral { width, value },
            }),
        })
    }

    /// Creates a bit-vector literal with a non-zero width.
    ///
    /// Literal construction currently supports constants up to 128 bits wide.
    pub fn bit_vec_literal(width: u32, value: u128) -> Result<Self, CircuitError> {
        let Some(width) = NonZeroU32::new(width) else {
            return Err(CircuitError::InvalidOperation(
                "BitVec literal width must be non-zero".to_string(),
            ));
        };
        if width.get() > 128 {
            return Err(CircuitError::InvalidOperation(format!(
                "BitVec literal width {} exceeds the 128-bit literal limit",
                width.get()
            )));
        }
        if width.get() < 128 && value >= (1u128 << width.get()) {
            return Err(CircuitError::InvalidOperation(format!(
                "BitVec literal value {value} does not fit in width {}",
                width.get()
            )));
        }
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::BitVec(width),
                kind: ClassicalExprKind::BitVecLiteral { width, value },
            }),
        })
    }

    /// Returns this expression's static type.
    pub fn ty(&self) -> ClassicalType {
        self.node.ty
    }

    /// Returns the expression node kind.
    ///
    /// This exposes the classical expression AST for inspection. Prefer the
    /// typed constructors on [`ClassicalExpr`] when building expressions so
    /// expression invariants remain centralized.
    pub fn kind(&self) -> &ClassicalExprKind {
        &self.node.kind
    }

    /// Returns a copy of this expression with circuit-local classical handles remapped.
    ///
    /// Every [`ClassicalVar`] and [`ClassicalValue`] referenced by this
    /// expression must have an entry in the corresponding map. Literal and
    /// structural nodes keep their static type and shape.
    pub fn remap_classical_ids(
        &self,
        var_map: &HashMap<ClassicalVar, ClassicalVar>,
        value_map: &HashMap<ClassicalValue, ClassicalValue>,
    ) -> Result<Self, CircuitError> {
        let kind = match &self.node.kind {
            ClassicalExprKind::Var(var) => {
                let mapped = var_map.get(var).copied().ok_or_else(|| {
                    CircuitError::InvalidOperation(format!(
                        "missing classical variable remap for id {}",
                        var.id()
                    ))
                })?;
                ClassicalExprKind::Var(mapped)
            }
            ClassicalExprKind::Value(value) => {
                let mapped = value_map.get(value).copied().ok_or_else(|| {
                    CircuitError::InvalidOperation(format!(
                        "missing classical value remap for id {}",
                        value.index()
                    ))
                })?;
                ClassicalExprKind::Value(mapped)
            }
            ClassicalExprKind::BoolLiteral(value) => ClassicalExprKind::BoolLiteral(*value),
            ClassicalExprKind::BitLiteral(value) => ClassicalExprKind::BitLiteral(*value),
            ClassicalExprKind::UIntLiteral { width, value } => ClassicalExprKind::UIntLiteral {
                width: *width,
                value: *value,
            },
            ClassicalExprKind::BitVecLiteral { width, value } => ClassicalExprKind::BitVecLiteral {
                width: *width,
                value: *value,
            },
            ClassicalExprKind::Unary { op, expr } => ClassicalExprKind::Unary {
                op: *op,
                expr: expr.remap_classical_ids(var_map, value_map)?,
            },
            ClassicalExprKind::Binary { op, lhs, rhs } => ClassicalExprKind::Binary {
                op: *op,
                lhs: lhs.remap_classical_ids(var_map, value_map)?,
                rhs: rhs.remap_classical_ids(var_map, value_map)?,
            },
            ClassicalExprKind::Compare { op, lhs, rhs } => ClassicalExprKind::Compare {
                op: *op,
                lhs: lhs.remap_classical_ids(var_map, value_map)?,
                rhs: rhs.remap_classical_ids(var_map, value_map)?,
            },
            ClassicalExprKind::Cast { cast, expr } => ClassicalExprKind::Cast {
                cast: *cast,
                expr: expr.remap_classical_ids(var_map, value_map)?,
            },
            ClassicalExprKind::Select {
                condition,
                then_expr,
                else_expr,
            } => ClassicalExprKind::Select {
                condition: condition.remap_classical_ids(var_map, value_map)?,
                then_expr: then_expr.remap_classical_ids(var_map, value_map)?,
                else_expr: else_expr.remap_classical_ids(var_map, value_map)?,
            },
            ClassicalExprKind::ExtractBit { value, index } => ClassicalExprKind::ExtractBit {
                value: value.remap_classical_ids(var_map, value_map)?,
                index: *index,
            },
            ClassicalExprKind::ExtractBits {
                value,
                offset,
                width,
            } => ClassicalExprKind::ExtractBits {
                value: value.remap_classical_ids(var_map, value_map)?,
                offset: *offset,
                width: *width,
            },
            ClassicalExprKind::Concat { parts } => ClassicalExprKind::Concat {
                parts: parts
                    .iter()
                    .map(|part| part.remap_classical_ids(var_map, value_map))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            },
            ClassicalExprKind::PackBits { bits } => ClassicalExprKind::PackBits {
                bits: bits
                    .iter()
                    .map(|bit| bit.remap_classical_ids(var_map, value_map))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            },
        };

        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: self.node.ty,
                kind,
            }),
        })
    }

    /// Returns all runtime classical variables read by this expression.
    pub fn vars(&self) -> BTreeSet<ClassicalVar> {
        let mut vars = BTreeSet::new();
        self.collect_vars(&mut vars);
        vars
    }

    /// Returns all immutable runtime classical values read by this expression.
    pub fn values(&self) -> BTreeSet<ClassicalValue> {
        let mut values = BTreeSet::new();
        self.collect_values(&mut values);
        values
    }

    /// Adds every runtime classical variable read by this expression to `out`.
    pub fn collect_vars(&self, out: &mut BTreeSet<ClassicalVar>) {
        match &self.node.kind {
            ClassicalExprKind::Var(var) => {
                out.insert(*var);
            }
            ClassicalExprKind::Value(_) => {}
            ClassicalExprKind::BoolLiteral(_)
            | ClassicalExprKind::BitLiteral(_)
            | ClassicalExprKind::UIntLiteral { .. }
            | ClassicalExprKind::BitVecLiteral { .. } => {}
            ClassicalExprKind::Unary { expr, .. } | ClassicalExprKind::Cast { expr, .. } => {
                expr.collect_vars(out);
            }
            ClassicalExprKind::Binary { lhs, rhs, .. }
            | ClassicalExprKind::Compare { lhs, rhs, .. } => {
                lhs.collect_vars(out);
                rhs.collect_vars(out);
            }
            ClassicalExprKind::Select {
                condition,
                then_expr,
                else_expr,
            } => {
                condition.collect_vars(out);
                then_expr.collect_vars(out);
                else_expr.collect_vars(out);
            }
            ClassicalExprKind::ExtractBit { value, .. }
            | ClassicalExprKind::ExtractBits { value, .. } => {
                value.collect_vars(out);
            }
            ClassicalExprKind::Concat { parts } => {
                for part in parts.iter() {
                    part.collect_vars(out);
                }
            }
            ClassicalExprKind::PackBits { bits } => {
                for bit in bits.iter() {
                    bit.collect_vars(out);
                }
            }
        }
    }

    /// Adds every immutable runtime classical value read by this expression to `out`.
    pub fn collect_values(&self, out: &mut BTreeSet<ClassicalValue>) {
        match &self.node.kind {
            ClassicalExprKind::Value(value) => {
                out.insert(*value);
            }
            ClassicalExprKind::Var(_)
            | ClassicalExprKind::BoolLiteral(_)
            | ClassicalExprKind::BitLiteral(_)
            | ClassicalExprKind::UIntLiteral { .. }
            | ClassicalExprKind::BitVecLiteral { .. } => {}
            ClassicalExprKind::Unary { expr, .. } | ClassicalExprKind::Cast { expr, .. } => {
                expr.collect_values(out);
            }
            ClassicalExprKind::Binary { lhs, rhs, .. }
            | ClassicalExprKind::Compare { lhs, rhs, .. } => {
                lhs.collect_values(out);
                rhs.collect_values(out);
            }
            ClassicalExprKind::Select {
                condition,
                then_expr,
                else_expr,
            } => {
                condition.collect_values(out);
                then_expr.collect_values(out);
                else_expr.collect_values(out);
            }
            ClassicalExprKind::ExtractBit { value, .. }
            | ClassicalExprKind::ExtractBits { value, .. } => {
                value.collect_values(out);
            }
            ClassicalExprKind::Concat { parts } => {
                for part in parts.iter() {
                    part.collect_values(out);
                }
            }
            ClassicalExprKind::PackBits { bits } => {
                for bit in bits.iter() {
                    bit.collect_values(out);
                }
            }
        }
    }

    /// Creates a `not` expression for `Bool` or `Bit` values.
    pub fn try_not(expr: impl Into<Self>) -> Result<Self, CircuitError> {
        let expr = expr.into();
        match expr.ty() {
            ClassicalType::Bool | ClassicalType::Bit => Ok(Self {
                node: Box::new(ClassicalExprNode {
                    ty: expr.ty(),
                    kind: ClassicalExprKind::Unary {
                        op: ClassicalUnaryOp::Not,
                        expr,
                    },
                }),
            }),
            ty => Err(CircuitError::InvalidOperation(format!(
                "not expects Bool or Bit, got {ty:?}"
            ))),
        }
    }

    /// Creates an `and` expression for matching `Bool` or matching `Bit` values.
    pub fn try_and(lhs: impl Into<Self>, rhs: impl Into<Self>) -> Result<Self, CircuitError> {
        let lhs = lhs.into();
        let rhs = rhs.into();
        let ty = lhs.ty();
        if ty != rhs.ty() {
            return Err(CircuitError::InvalidOperation(format!(
                "binary operands must have the same type, got {:?} and {:?}",
                lhs.ty(),
                rhs.ty()
            )));
        }
        match ty {
            ClassicalType::Bool | ClassicalType::Bit => Ok(Self {
                node: Box::new(ClassicalExprNode {
                    ty,
                    kind: ClassicalExprKind::Binary {
                        op: ClassicalBinaryOp::And,
                        lhs,
                        rhs,
                    },
                }),
            }),
            ty => Err(CircuitError::InvalidOperation(format!(
                "binary And expects Bool or Bit operands, got {ty:?}"
            ))),
        }
    }

    /// Creates an `or` expression for matching `Bool` or matching `Bit` values.
    pub fn try_or(lhs: impl Into<Self>, rhs: impl Into<Self>) -> Result<Self, CircuitError> {
        let lhs = lhs.into();
        let rhs = rhs.into();
        let ty = lhs.ty();
        if ty != rhs.ty() {
            return Err(CircuitError::InvalidOperation(format!(
                "binary operands must have the same type, got {:?} and {:?}",
                lhs.ty(),
                rhs.ty()
            )));
        }
        match ty {
            ClassicalType::Bool | ClassicalType::Bit => Ok(Self {
                node: Box::new(ClassicalExprNode {
                    ty,
                    kind: ClassicalExprKind::Binary {
                        op: ClassicalBinaryOp::Or,
                        lhs,
                        rhs,
                    },
                }),
            }),
            ty => Err(CircuitError::InvalidOperation(format!(
                "binary Or expects Bool or Bit operands, got {ty:?}"
            ))),
        }
    }

    /// Creates an `xor` expression for matching `Bool` or matching `Bit` values.
    pub fn try_xor(lhs: impl Into<Self>, rhs: impl Into<Self>) -> Result<Self, CircuitError> {
        let lhs = lhs.into();
        let rhs = rhs.into();
        let ty = lhs.ty();
        if ty != rhs.ty() {
            return Err(CircuitError::InvalidOperation(format!(
                "binary operands must have the same type, got {:?} and {:?}",
                lhs.ty(),
                rhs.ty()
            )));
        }
        match ty {
            ClassicalType::Bool | ClassicalType::Bit => Ok(Self {
                node: Box::new(ClassicalExprNode {
                    ty,
                    kind: ClassicalExprKind::Binary {
                        op: ClassicalBinaryOp::Xor,
                        lhs,
                        rhs,
                    },
                }),
            }),
            ty => Err(CircuitError::InvalidOperation(format!(
                "binary Xor expects Bool or Bit operands, got {ty:?}"
            ))),
        }
    }

    /// Creates an equality comparison. Operands must have the same type.
    pub fn eq(lhs: Self, rhs: Self) -> Result<Self, CircuitError> {
        if lhs.ty() != rhs.ty() {
            return Err(CircuitError::InvalidOperation(format!(
                "comparison operands must have the same type, got {:?} and {:?}",
                lhs.ty(),
                rhs.ty()
            )));
        }
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::Bool,
                kind: ClassicalExprKind::Compare {
                    op: ClassicalCompareOp::Eq,
                    lhs,
                    rhs,
                },
            }),
        })
    }

    /// Creates an inequality comparison. Operands must have the same type.
    pub fn ne(lhs: Self, rhs: Self) -> Result<Self, CircuitError> {
        if lhs.ty() != rhs.ty() {
            return Err(CircuitError::InvalidOperation(format!(
                "comparison operands must have the same type, got {:?} and {:?}",
                lhs.ty(),
                rhs.ty()
            )));
        }
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::Bool,
                kind: ClassicalExprKind::Compare {
                    op: ClassicalCompareOp::Ne,
                    lhs,
                    rhs,
                },
            }),
        })
    }

    /// Creates an unsigned less-than comparison.
    pub fn lt(lhs: Self, rhs: Self) -> Result<Self, CircuitError> {
        if lhs.ty() != rhs.ty() {
            return Err(CircuitError::InvalidOperation(format!(
                "comparison operands must have the same type, got {:?} and {:?}",
                lhs.ty(),
                rhs.ty()
            )));
        }
        if !matches!(lhs.ty(), ClassicalType::UInt(_)) {
            return Err(CircuitError::InvalidOperation(format!(
                "ordered comparisons require UInt operands, got {:?}",
                lhs.ty()
            )));
        }
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::Bool,
                kind: ClassicalExprKind::Compare {
                    op: ClassicalCompareOp::Lt,
                    lhs,
                    rhs,
                },
            }),
        })
    }

    /// Creates an unsigned less-than-or-equal comparison.
    pub fn le(lhs: Self, rhs: Self) -> Result<Self, CircuitError> {
        if lhs.ty() != rhs.ty() {
            return Err(CircuitError::InvalidOperation(format!(
                "comparison operands must have the same type, got {:?} and {:?}",
                lhs.ty(),
                rhs.ty()
            )));
        }
        if !matches!(lhs.ty(), ClassicalType::UInt(_)) {
            return Err(CircuitError::InvalidOperation(format!(
                "ordered comparisons require UInt operands, got {:?}",
                lhs.ty()
            )));
        }
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::Bool,
                kind: ClassicalExprKind::Compare {
                    op: ClassicalCompareOp::Le,
                    lhs,
                    rhs,
                },
            }),
        })
    }

    /// Creates an unsigned greater-than comparison.
    pub fn gt(lhs: Self, rhs: Self) -> Result<Self, CircuitError> {
        if lhs.ty() != rhs.ty() {
            return Err(CircuitError::InvalidOperation(format!(
                "comparison operands must have the same type, got {:?} and {:?}",
                lhs.ty(),
                rhs.ty()
            )));
        }
        if !matches!(lhs.ty(), ClassicalType::UInt(_)) {
            return Err(CircuitError::InvalidOperation(format!(
                "ordered comparisons require UInt operands, got {:?}",
                lhs.ty()
            )));
        }
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::Bool,
                kind: ClassicalExprKind::Compare {
                    op: ClassicalCompareOp::Gt,
                    lhs,
                    rhs,
                },
            }),
        })
    }

    /// Creates an unsigned greater-than-or-equal comparison.
    pub fn ge(lhs: Self, rhs: Self) -> Result<Self, CircuitError> {
        if lhs.ty() != rhs.ty() {
            return Err(CircuitError::InvalidOperation(format!(
                "comparison operands must have the same type, got {:?} and {:?}",
                lhs.ty(),
                rhs.ty()
            )));
        }
        if !matches!(lhs.ty(), ClassicalType::UInt(_)) {
            return Err(CircuitError::InvalidOperation(format!(
                "ordered comparisons require UInt operands, got {:?}",
                lhs.ty()
            )));
        }
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::Bool,
                kind: ClassicalExprKind::Compare {
                    op: ClassicalCompareOp::Ge,
                    lhs,
                    rhs,
                },
            }),
        })
    }

    /// Explicitly casts a `Bit` expression to `Bool`.
    pub fn bit_to_bool(expr: Self) -> Result<Self, CircuitError> {
        if expr.ty() != ClassicalType::Bit {
            return Err(CircuitError::InvalidOperation(format!(
                "bit_to_bool expects Bit, got {:?}",
                expr.ty()
            )));
        }
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::Bool,
                kind: ClassicalExprKind::Cast {
                    cast: ClassicalCast::BitToBool,
                    expr,
                },
            }),
        })
    }

    /// Explicitly casts a `BitVec` expression to a little-endian `UInt`.
    pub fn bit_vec_to_uint(expr: Self) -> Result<Self, CircuitError> {
        match expr.ty() {
            ClassicalType::BitVec(width) => Ok(Self {
                node: Box::new(ClassicalExprNode {
                    ty: ClassicalType::UInt(width),
                    kind: ClassicalExprKind::Cast {
                        cast: ClassicalCast::BitVecToUInt,
                        expr,
                    },
                }),
            }),
            ty => Err(CircuitError::InvalidOperation(format!(
                "bit_vec_to_uint expects BitVec, got {ty:?}"
            ))),
        }
    }

    /// Converts this `Bit` expression to `Bool`.
    pub fn to_bool(self) -> Result<Self, CircuitError> {
        Self::bit_to_bool(self)
    }

    /// Converts this `BitVec` expression to a little-endian `UInt`.
    pub fn to_uint(self) -> Result<Self, CircuitError> {
        Self::bit_vec_to_uint(self)
    }

    /// Creates an expression that chooses between two same-typed values.
    pub fn select(condition: Self, then_expr: Self, else_expr: Self) -> Result<Self, CircuitError> {
        if condition.ty() != ClassicalType::Bool {
            return Err(CircuitError::InvalidOperation(format!(
                "select condition expects Bool, got {:?}",
                condition.ty()
            )));
        }
        if then_expr.ty() != else_expr.ty() {
            return Err(CircuitError::InvalidOperation(format!(
                "select branches must have the same type, got {:?} and {:?}",
                then_expr.ty(),
                else_expr.ty()
            )));
        }
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: then_expr.ty(),
                kind: ClassicalExprKind::Select {
                    condition,
                    then_expr,
                    else_expr,
                },
            }),
        })
    }

    /// Extracts a single bit from a `BitVec` or `UInt`.
    ///
    /// Index `0` is the least-significant bit.
    pub fn extract_bit(value: Self, index: u32) -> Result<Self, CircuitError> {
        let width = match value.ty() {
            ClassicalType::UInt(width) | ClassicalType::BitVec(width) => width.get(),
            ty => {
                return Err(CircuitError::InvalidOperation(format!(
                    "extract_bit expects UInt or BitVec, got {ty:?}"
                )));
            }
        };
        if index >= width {
            return Err(CircuitError::InvalidOperation(format!(
                "extract_bit index {index} out of bounds for width {width}"
            )));
        }
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::Bit,
                kind: ClassicalExprKind::ExtractBit { value, index },
            }),
        })
    }

    /// Extracts a bit range from a `BitVec` or `UInt`.
    ///
    /// `offset` is little-endian: offset `0` starts at the least-significant bit.
    pub fn extract_bits(value: Self, offset: u32, width: u32) -> Result<Self, CircuitError> {
        let source_width = match value.ty() {
            ClassicalType::UInt(width) | ClassicalType::BitVec(width) => width.get(),
            ty => {
                return Err(CircuitError::InvalidOperation(format!(
                    "extract_bits expects UInt or BitVec, got {ty:?}"
                )));
            }
        };
        let Some(width) = NonZeroU32::new(width) else {
            return Err(CircuitError::InvalidOperation(
                "extract_bits width must be non-zero".to_string(),
            ));
        };
        let Some(end) = offset.checked_add(width.get()) else {
            return Err(CircuitError::InvalidOperation(
                "extract_bits range overflows u32".to_string(),
            ));
        };
        if end > source_width {
            return Err(CircuitError::InvalidOperation(format!(
                "extract_bits range [{offset}, {end}) out of bounds for width {source_width}"
            )));
        }
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::BitVec(width),
                kind: ClassicalExprKind::ExtractBits {
                    value,
                    offset,
                    width,
                },
            }),
        })
    }

    /// Concatenates bit-vector values into a larger bit-vector.
    ///
    /// The first part occupies the least-significant output bits.
    pub fn concat(parts: impl IntoIterator<Item = Self>) -> Result<Self, CircuitError> {
        let parts: Vec<Self> = parts.into_iter().collect();
        if parts.is_empty() {
            return Err(CircuitError::InvalidOperation(
                "concat requires at least one part".to_string(),
            ));
        }

        let mut total_width = 0u32;
        for part in &parts {
            match part.ty() {
                ClassicalType::Bit => {
                    let Some(next_width) = total_width.checked_add(1) else {
                        return Err(CircuitError::InvalidOperation(
                            "concat width overflows u32".to_string(),
                        ));
                    };
                    total_width = next_width;
                }
                ClassicalType::BitVec(width) => {
                    let Some(next_width) = total_width.checked_add(width.get()) else {
                        return Err(CircuitError::InvalidOperation(
                            "concat width overflows u32".to_string(),
                        ));
                    };
                    total_width = next_width;
                }
                ty => {
                    return Err(CircuitError::InvalidOperation(format!(
                        "concat expects Bit or BitVec parts, got {ty:?}"
                    )));
                }
            }
        }

        let Some(width) = NonZeroU32::new(total_width) else {
            return Err(CircuitError::InvalidOperation(
                "concat width must be non-zero".to_string(),
            ));
        };
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::BitVec(width),
                kind: ClassicalExprKind::Concat {
                    parts: parts.into_boxed_slice(),
                },
            }),
        })
    }

    /// Returns `true` when this expression is the boolean literal `true`.
    pub fn is_bool_true(&self) -> bool {
        matches!(self.kind(), ClassicalExprKind::BoolLiteral(true))
    }

    /// Returns `true` when this expression is the boolean literal `false`.
    pub fn is_bool_false(&self) -> bool {
        matches!(self.kind(), ClassicalExprKind::BoolLiteral(false))
    }

    /// Returns `true` when this expression is the bit literal `true` (1).
    pub fn is_bit_true(&self) -> bool {
        matches!(self.kind(), ClassicalExprKind::BitLiteral(true))
    }

    /// Returns `true` when this expression is the bit literal `false` (0).
    pub fn is_bit_false(&self) -> bool {
        matches!(self.kind(), ClassicalExprKind::BitLiteral(false))
    }

    /// Returns a structurally simplified copy of this expression.
    ///
    /// Simplification eliminates literal-driven redundancies and algebraic
    /// identities without evaluating runtime variable or value reads. It is
    /// idempotent: applying it twice yields the same expression.
    ///
    /// See [`simplify`](super::simplify::simplify) for the full rule set.
    pub fn simplified(&self) -> Self {
        super::simplify::simplify(self)
    }

    /// Creates an expression from a type-tag and node kind.
    ///
    /// This is a low-level constructor used by the simplifier and other
    /// internal passes. Prefer the typed constructors for normal circuit
    /// building.
    pub(crate) fn from_kind(ty: ClassicalType, kind: ClassicalExprKind) -> Self {
        Self {
            node: Box::new(ClassicalExprNode { ty, kind }),
        }
    }

    /// Packs single-bit expressions into a `BitVec`.
    ///
    /// The first bit becomes output index `0`, the least-significant bit.
    pub fn pack_bits(bits: impl IntoIterator<Item = Self>) -> Result<Self, CircuitError> {
        let bits: Vec<Self> = bits.into_iter().collect();
        if bits.is_empty() {
            return Err(CircuitError::InvalidOperation(
                "pack_bits requires at least one bit".to_string(),
            ));
        }
        for bit in &bits {
            if bit.ty() != ClassicalType::Bit {
                return Err(CircuitError::InvalidOperation(format!(
                    "pack_bits expects Bit expressions, got {:?}",
                    bit.ty()
                )));
            }
        }

        let Some(width) = NonZeroU32::new(bits.len() as u32) else {
            return Err(CircuitError::InvalidOperation(
                "pack_bits width must be non-zero".to_string(),
            ));
        };
        Ok(Self {
            node: Box::new(ClassicalExprNode {
                ty: ClassicalType::BitVec(width),
                kind: ClassicalExprKind::PackBits {
                    bits: bits.into_boxed_slice(),
                },
            }),
        })
    }

    /// Returns the target bit replaced by a single-bit measurement result.
    ///
    /// The expression must reconstruct the complete `BitVec` target in index
    /// order, preserving every other bit from the target itself and using
    /// `result` exactly once.
    pub(crate) fn measurement_store_bit(
        &self,
        target: ClassicalVar,
        result: ClassicalValue,
    ) -> Option<u32> {
        let ClassicalType::BitVec(width) = target.ty() else {
            return None;
        };
        let ClassicalExprKind::PackBits { bits } = self.kind() else {
            return None;
        };
        if bits.len() != width.get() as usize {
            return None;
        }

        let mut measured_index = None;
        for (index, bit) in bits.iter().enumerate() {
            match bit.kind() {
                ClassicalExprKind::Value(value) if *value == result => {
                    if measured_index.replace(index as u32).is_some() {
                        return None;
                    }
                }
                ClassicalExprKind::ExtractBit {
                    value,
                    index: source_index,
                } if *source_index == index as u32
                    && matches!(value.kind(), ClassicalExprKind::Var(var) if *var == target) => {}
                _ => return None,
            }
        }
        measured_index
    }
}

impl From<ClassicalVar> for ClassicalExpr {
    fn from(var: ClassicalVar) -> Self {
        Self::var(var)
    }
}

impl From<ClassicalValue> for ClassicalExpr {
    fn from(value: ClassicalValue) -> Self {
        Self::value(value)
    }
}

macro_rules! impl_expr_not {
    ($ty:ty) => {
        impl Not for $ty {
            type Output = ClassicalExpr;

            fn not(self) -> Self::Output {
                ClassicalExpr::try_not(self)
                    .expect("ClassicalExpr ! operator requires a Bool or Bit expression")
            }
        }
    };
}

macro_rules! impl_expr_binary_ops {
    ($ty:ty) => {
        impl<Rhs> BitAnd<Rhs> for $ty
        where
            Rhs: Into<ClassicalExpr>,
        {
            type Output = ClassicalExpr;

            fn bitand(self, rhs: Rhs) -> Self::Output {
                ClassicalExpr::try_and(self, rhs).expect(
                    "ClassicalExpr & operator requires matching Bool or matching Bit expressions",
                )
            }
        }

        impl<Rhs> BitOr<Rhs> for $ty
        where
            Rhs: Into<ClassicalExpr>,
        {
            type Output = ClassicalExpr;

            fn bitor(self, rhs: Rhs) -> Self::Output {
                ClassicalExpr::try_or(self, rhs).expect(
                    "ClassicalExpr | operator requires matching Bool or matching Bit expressions",
                )
            }
        }

        impl<Rhs> BitXor<Rhs> for $ty
        where
            Rhs: Into<ClassicalExpr>,
        {
            type Output = ClassicalExpr;

            fn bitxor(self, rhs: Rhs) -> Self::Output {
                ClassicalExpr::try_xor(self, rhs).expect(
                    "ClassicalExpr ^ operator requires matching Bool or matching Bit expressions",
                )
            }
        }
    };
}

impl_expr_not!(ClassicalExpr);
impl_expr_not!(ClassicalVar);
impl_expr_not!(ClassicalValue);
impl_expr_binary_ops!(ClassicalExpr);
impl_expr_binary_ops!(ClassicalVar);
impl_expr_binary_ops!(ClassicalValue);

#[cfg(test)]
#[path = "expr_test.rs"]
mod expr_test;
