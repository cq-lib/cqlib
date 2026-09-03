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

//! Runtime classical state produced while simulating a circuit.
//!
//! The circuit IR stores classical handles, not concrete values. During
//! simulation this module fills those handles with runtime data:
//!
//! - [`ClassicalValue`] handles address immutable operation results such as
//!   measurements by circuit identity and table index.
//! - [`ClassicalVar`] handles address mutable storage written by `store` using
//!   the same circuit-local ownership model.
//!
//! `BitVec` values use [`Outcome`] so their bit order matches the rest of the
//! device/result layer: bit index `0` is stored as the least-significant bit and
//! string formatting prints the most-significant bit on the left.

use crate::circuit::{
    Circuit, CircuitError, CircuitId, ClassicalBinaryOp, ClassicalCast, ClassicalCompareOp,
    ClassicalExpr, ClassicalExprKind, ClassicalType, ClassicalUnaryOp, ClassicalValue,
    ClassicalVar,
};
use crate::device::Outcome;
use crate::qis::QisError;

/// A typed runtime classical value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuntimeValue {
    Bit(bool),
    Bool(bool),
    UInt { width: u32, value: u128 },
    BitVec { width: u32, bits: Outcome },
}

impl RuntimeValue {
    /// Returns the static circuit type represented by this runtime value.
    pub fn ty(&self) -> ClassicalType {
        match self {
            Self::Bit(_) => ClassicalType::Bit,
            Self::Bool(_) => ClassicalType::Bool,
            Self::UInt { width, .. } => ClassicalType::uint(*width).unwrap(),
            Self::BitVec { width, .. } => ClassicalType::bit_vec(*width).unwrap(),
        }
    }

    /// Returns a bit-string for `Bit` and `BitVec` values using MSB-left display order.
    pub fn to_bitstring(&self) -> Option<String> {
        match self {
            Self::Bit(value) => Some(if *value { "1" } else { "0" }.to_string()),
            Self::BitVec { width, bits } => Some(bits.to_bitstring(*width as usize)),
            Self::Bool(_) | Self::UInt { .. } => None,
        }
    }

    /// Builds a `BitVec` from little-endian bits.
    ///
    /// `bits[0]` becomes bit index 0, the least-significant bit. This is the
    /// same convention used by `measure_bits`, `pack_bits`, and `concat`.
    pub(crate) fn bit_vec_from_lsb(bits: &[bool]) -> Self {
        Self::BitVec {
            width: bits.len() as u32,
            bits: Outcome::from_lsb_bits(bits),
        }
    }

    fn bit_at(&self, index: u32) -> Result<bool, QisError> {
        match self {
            Self::UInt { width, value } if index < *width && index < 128 => {
                Ok((value >> index) & 1 == 1)
            }
            Self::BitVec { width, bits } if index < *width => Ok(bits.is_one(index as usize)),
            _ => Err(qis_unsupported(
                "cannot read the requested bit from runtime value",
            )),
        }
    }

    fn bits_lsb(&self) -> Result<Vec<bool>, QisError> {
        match self {
            Self::Bit(value) => Ok(vec![*value]),
            Self::BitVec { width, bits } => Ok((0..*width)
                .map(|index| bits.is_one(index as usize))
                .collect()),
            Self::Bool(_) | Self::UInt { .. } => Err(qis_unsupported(
                "runtime value cannot be interpreted as BitVec bits",
            )),
        }
    }

    fn as_u128(&self) -> Result<u128, QisError> {
        match self {
            Self::UInt { value, .. } => Ok(*value),
            Self::BitVec { width, bits } if *width <= 128 => Ok((0..*width)
                .fold(0u128, |value, index| {
                    value | ((bits.is_one(index as usize) as u128) << index)
                })),
            Self::BitVec { .. } => Err(qis_unsupported(
                "BitVec values wider than 128 bits cannot be cast to UInt",
            )),
            Self::Bit(_) | Self::Bool(_) => Err(qis_unsupported(
                "runtime value cannot be interpreted as an unsigned integer",
            )),
        }
    }
}

/// Runtime storage for immutable circuit values and mutable classical variables.
#[derive(Debug, Clone)]
pub struct ClassicalState {
    circuit_id: CircuitId,
    value_types: Vec<ClassicalType>,
    values: Vec<Option<RuntimeValue>>,
    var_types: Vec<ClassicalType>,
    vars: Vec<Option<RuntimeValue>>,
}

impl ClassicalState {
    pub(crate) fn for_circuit(circuit: &Circuit) -> Self {
        Self {
            circuit_id: circuit.id(),
            value_types: circuit.classical_values().to_vec(),
            values: vec![None; circuit.classical_values().len()],
            var_types: circuit.classical_vars().to_vec(),
            vars: vec![None; circuit.classical_vars().len()],
        }
    }

    /// Returns the runtime result produced for an immutable circuit value.
    ///
    /// Returns `None` when the handle belongs to another circuit, has a
    /// mismatched type, is outside the value table, or has not been produced.
    pub fn value(&self, value: ClassicalValue) -> Option<&RuntimeValue> {
        if value.circuit_id() != self.circuit_id {
            return None;
        }
        self.value_types
            .get(value.index() as usize)
            .filter(|ty| **ty == value.ty())?;
        self.values.get(value.index() as usize)?.as_ref()
    }

    /// Returns the current runtime value of a mutable classical variable.
    ///
    /// Returns `None` when the handle belongs to another circuit, has a
    /// mismatched type, is outside the variable table, or has not been stored.
    pub fn var(&self, var: ClassicalVar) -> Option<&RuntimeValue> {
        if var.circuit_id() != self.circuit_id {
            return None;
        }
        self.var_types
            .get(var.index() as usize)
            .filter(|ty| **ty == var.ty())?;
        self.vars.get(var.index() as usize)?.as_ref()
    }

    pub(crate) fn set_value(
        &mut self,
        target: ClassicalValue,
        value: RuntimeValue,
    ) -> Result<(), QisError> {
        if target.circuit_id() != self.circuit_id {
            return Err(CircuitError::ForeignClassicalHandle {
                kind: "classical value",
                index: target.index(),
            }
            .into());
        }
        let expected = self
            .value_types
            .get(target.index() as usize)
            .copied()
            .ok_or_else(|| qis_unsupported("measurement result is not owned by this circuit"))?;
        if expected != target.ty() || expected != value.ty() {
            return Err(qis_unsupported(
                "measurement result type does not match circuit IR",
            ));
        }
        self.values[target.index() as usize] = Some(value);
        Ok(())
    }

    pub(crate) fn store(
        &mut self,
        target: ClassicalVar,
        expr: &ClassicalExpr,
    ) -> Result<(), QisError> {
        if target.circuit_id() != self.circuit_id {
            return Err(CircuitError::ForeignClassicalHandle {
                kind: "classical variable",
                index: target.index(),
            }
            .into());
        }
        let expected = self
            .var_types
            .get(target.index() as usize)
            .copied()
            .ok_or_else(|| qis_unsupported("store target is not owned by this circuit"))?;
        let value = self.evaluate(expr)?;
        if expected != target.ty() || expected != value.ty() {
            return Err(qis_unsupported(
                "store value type does not match its target",
            ));
        }
        self.vars[target.index() as usize] = Some(value);
        Ok(())
    }

    /// Re-associates results produced from a transformed circuit with the
    /// source circuit whose handles are exposed to the caller.
    pub(crate) fn rebind_to_circuit(&mut self, circuit: &Circuit) -> Result<(), QisError> {
        if self.value_types.as_slice() != circuit.classical_values()
            || self.var_types.as_slice() != circuit.classical_vars()
        {
            return Err(CircuitError::InvalidOperation(
                "cannot rebind classical state to a circuit with different classical tables"
                    .to_string(),
            )
            .into());
        }
        self.circuit_id = circuit.id();
        Ok(())
    }

    /// Evaluates a side-effect-free classical expression against current runtime state.
    ///
    /// This intentionally does not execute control flow. It is used by
    /// `ClassicalDataOp::Store` after measurements have populated `values`.
    pub(crate) fn evaluate(&self, expr: &ClassicalExpr) -> Result<RuntimeValue, QisError> {
        match expr.kind() {
            ClassicalExprKind::Var(var) => self
                .var(*var)
                .cloned()
                .ok_or_else(|| qis_unsupported("classical variable is read before being stored")),
            ClassicalExprKind::Value(value) => self
                .value(*value)
                .cloned()
                .ok_or_else(|| qis_unsupported("classical value is read before being produced")),
            ClassicalExprKind::BoolLiteral(value) => Ok(RuntimeValue::Bool(*value)),
            ClassicalExprKind::BitLiteral(value) => Ok(RuntimeValue::Bit(*value)),
            ClassicalExprKind::UIntLiteral { width, value } => Ok(RuntimeValue::UInt {
                width: width.get(),
                value: *value,
            }),
            ClassicalExprKind::BitVecLiteral { width, value } => Ok(RuntimeValue::BitVec {
                width: width.get(),
                bits: Outcome::from_lsb_bits(
                    &(0..width.get())
                        .map(|index| (value >> index) & 1 == 1)
                        .collect::<Vec<_>>(),
                ),
            }),
            ClassicalExprKind::Unary { op, expr } => {
                let value = self.evaluate(expr)?;
                match (op, value) {
                    (ClassicalUnaryOp::Not, RuntimeValue::Bit(value)) => {
                        Ok(RuntimeValue::Bit(!value))
                    }
                    (ClassicalUnaryOp::Not, RuntimeValue::Bool(value)) => {
                        Ok(RuntimeValue::Bool(!value))
                    }
                    _ => Err(qis_unsupported("invalid unary classical expression")),
                }
            }
            ClassicalExprKind::Binary { op, lhs, rhs } => {
                let lhs = self.evaluate(lhs)?;
                let rhs = self.evaluate(rhs)?;
                evaluate_binary(*op, lhs, rhs)
            }
            ClassicalExprKind::Compare { op, lhs, rhs } => {
                let lhs = self.evaluate(lhs)?;
                let rhs = self.evaluate(rhs)?;
                evaluate_compare(*op, lhs, rhs)
            }
            ClassicalExprKind::Cast { cast, expr } => {
                let value = self.evaluate(expr)?;
                match (cast, value) {
                    (ClassicalCast::BitToBool, RuntimeValue::Bit(value)) => {
                        Ok(RuntimeValue::Bool(value))
                    }
                    (ClassicalCast::BitVecToUInt, RuntimeValue::BitVec { width, bits }) => {
                        Ok(RuntimeValue::UInt {
                            width,
                            value: RuntimeValue::BitVec { width, bits }.as_u128()?,
                        })
                    }
                    _ => Err(qis_unsupported("invalid classical cast")),
                }
            }
            ClassicalExprKind::Select {
                condition,
                then_expr,
                else_expr,
            } => match self.evaluate(condition)? {
                RuntimeValue::Bool(true) => self.evaluate(then_expr),
                RuntimeValue::Bool(false) => self.evaluate(else_expr),
                _ => Err(qis_unsupported("select condition is not a Bool")),
            },
            ClassicalExprKind::ExtractBit { value, index } => {
                Ok(RuntimeValue::Bit(self.evaluate(value)?.bit_at(*index)?))
            }
            ClassicalExprKind::ExtractBits {
                value,
                offset,
                width,
            } => {
                let value = self.evaluate(value)?;
                let bits = (0..width.get())
                    .map(|index| value.bit_at(offset + index))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(RuntimeValue::bit_vec_from_lsb(&bits))
            }
            ClassicalExprKind::Concat { parts } => {
                let mut bits = Vec::new();
                for part in parts.iter() {
                    bits.extend(self.evaluate(part)?.bits_lsb()?);
                }
                Ok(RuntimeValue::bit_vec_from_lsb(&bits))
            }
            ClassicalExprKind::PackBits { bits } => {
                let bits = bits
                    .iter()
                    .map(|bit| match self.evaluate(bit)? {
                        RuntimeValue::Bit(value) => Ok(value),
                        _ => Err(qis_unsupported("pack_bits input is not a Bit")),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(RuntimeValue::bit_vec_from_lsb(&bits))
            }
        }
    }
}

fn evaluate_binary(
    op: ClassicalBinaryOp,
    lhs: RuntimeValue,
    rhs: RuntimeValue,
) -> Result<RuntimeValue, QisError> {
    let apply = |lhs: bool, rhs: bool| match op {
        ClassicalBinaryOp::And => lhs & rhs,
        ClassicalBinaryOp::Or => lhs | rhs,
        ClassicalBinaryOp::Xor => lhs ^ rhs,
    };
    match (lhs, rhs) {
        (RuntimeValue::Bit(lhs), RuntimeValue::Bit(rhs)) => Ok(RuntimeValue::Bit(apply(lhs, rhs))),
        (RuntimeValue::Bool(lhs), RuntimeValue::Bool(rhs)) => {
            Ok(RuntimeValue::Bool(apply(lhs, rhs)))
        }
        _ => Err(qis_unsupported("invalid binary classical expression")),
    }
}

fn evaluate_compare(
    op: ClassicalCompareOp,
    lhs: RuntimeValue,
    rhs: RuntimeValue,
) -> Result<RuntimeValue, QisError> {
    let result = match op {
        ClassicalCompareOp::Eq => lhs == rhs,
        ClassicalCompareOp::Ne => lhs != rhs,
        ClassicalCompareOp::Lt
        | ClassicalCompareOp::Le
        | ClassicalCompareOp::Gt
        | ClassicalCompareOp::Ge => match (lhs, rhs) {
            (
                RuntimeValue::UInt {
                    width: lhs_width,
                    value: lhs,
                },
                RuntimeValue::UInt {
                    width: rhs_width,
                    value: rhs,
                },
            ) if lhs_width == rhs_width => match op {
                ClassicalCompareOp::Lt => lhs < rhs,
                ClassicalCompareOp::Le => lhs <= rhs,
                ClassicalCompareOp::Gt => lhs > rhs,
                ClassicalCompareOp::Ge => lhs >= rhs,
                ClassicalCompareOp::Eq | ClassicalCompareOp::Ne => unreachable!(),
            },
            _ => {
                return Err(qis_unsupported(
                    "ordered comparison operands are not matching UInts",
                ));
            }
        },
    };
    Ok(RuntimeValue::Bool(result))
}

fn qis_unsupported(message: impl Into<String>) -> QisError {
    QisError::UnsupportedOperation(message.into())
}

#[cfg(test)]
mod tests {
    use super::{ClassicalState, RuntimeValue};
    use crate::circuit::{Circuit, CircuitError, ClassicalExpr, ClassicalType, Qubit};
    use crate::qis::QisError;

    #[test]
    fn evaluates_literals_and_bit_vector_operations() {
        let circuit = Circuit::new(0);
        let state = ClassicalState::for_circuit(&circuit);
        let packed = ClassicalExpr::pack_bits([
            ClassicalExpr::bit_literal(true),
            ClassicalExpr::bit_literal(false),
            ClassicalExpr::bit_literal(true),
        ])
        .unwrap();

        let value = state.evaluate(&packed).unwrap();
        assert_eq!(value.to_bitstring().as_deref(), Some("101"));
    }

    #[test]
    fn stores_and_reads_a_variable() {
        let mut circuit = Circuit::new(0);
        let var = circuit.var(ClassicalType::Bit);
        let mut state = ClassicalState::for_circuit(&circuit);

        state.store(var, &ClassicalExpr::bit_literal(true)).unwrap();

        assert_eq!(state.var(var), Some(&RuntimeValue::Bit(true)));
    }

    #[test]
    fn rejects_handles_owned_by_another_circuit() {
        let mut owner = Circuit::new(1);
        let owner_var = owner.var(ClassicalType::Bit);
        let owner_value = owner.measure(Qubit::new(0)).unwrap().value();
        let mut state = ClassicalState::for_circuit(&owner);
        state
            .set_value(owner_value, RuntimeValue::Bit(true))
            .unwrap();
        state
            .store(owner_var, &ClassicalExpr::bit_literal(true))
            .unwrap();

        let mut other = Circuit::new(1);
        let other_var = other.var(ClassicalType::Bit);
        let other_value = other.measure(Qubit::new(0)).unwrap().value();

        assert_eq!(state.value(other_value), None);
        assert_eq!(state.var(other_var), None);
        assert!(matches!(
            state.set_value(other_value, RuntimeValue::Bit(false)),
            Err(QisError::CircuitError(
                CircuitError::ForeignClassicalHandle {
                    kind: "classical value",
                    ..
                }
            ))
        ));
        assert!(matches!(
            state.store(other_var, &ClassicalExpr::bit_literal(false)),
            Err(QisError::CircuitError(
                CircuitError::ForeignClassicalHandle {
                    kind: "classical variable",
                    ..
                }
            ))
        ));
    }

    #[test]
    fn runtime_value_compares_by_value_and_hashes_consistently() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let hash_of = |value: &RuntimeValue| {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        };

        assert_eq!(
            RuntimeValue::UInt {
                width: 8,
                value: 42
            },
            RuntimeValue::UInt {
                width: 8,
                value: 42
            }
        );
        assert_ne!(
            RuntimeValue::UInt {
                width: 8,
                value: 42
            },
            RuntimeValue::UInt {
                width: 8,
                value: 43
            }
        );
        assert_ne!(RuntimeValue::Bit(true), RuntimeValue::Bool(true),);

        let bit_vec = RuntimeValue::BitVec {
            width: 3,
            bits: crate::device::Outcome::from_bitstring("101").unwrap(),
        };
        let bit_vec_again = RuntimeValue::BitVec {
            width: 3,
            bits: crate::device::Outcome::from_bitstring("101").unwrap(),
        };
        assert_eq!(bit_vec, bit_vec_again);
        assert_eq!(hash_of(&bit_vec), hash_of(&bit_vec_again));
        assert_eq!(
            hash_of(&RuntimeValue::UInt {
                width: 8,
                value: 42
            }),
            hash_of(&RuntimeValue::UInt {
                width: 8,
                value: 42
            })
        );
    }
}
