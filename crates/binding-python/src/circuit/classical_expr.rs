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

//! Python binding for the typed, side-effect-free classical expression AST.
//!
//! Construction delegates to `cqlib-core`, keeping type validation and AST
//! invariants in one place. Python operators are provided only for the logical
//! and bitwise operations whose meaning matches the core expression model.

use crate::circuit::classical::{PyClassicalType, PyClassicalValue, PyClassicalVar};
use crate::circuit::error::CircuitError as PyCircuitError;
use crate::utils::hash_value;
use cqlib_core::circuit::ClassicalExpr;
use pyo3::prelude::*;

/// Typed classical expression used by dynamic-circuit control flow.
#[pyclass(name = "ClassicalExpr", module = "cqlib.circuit", from_py_object)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PyClassicalExpr {
    pub(crate) inner: ClassicalExpr,
}

#[pymethods]
impl PyClassicalExpr {
    #[staticmethod]
    fn var(var: PyClassicalVar) -> Self {
        Self {
            inner: ClassicalExpr::var(var.inner),
        }
    }

    #[staticmethod]
    fn value(value: PyClassicalValue) -> Self {
        Self {
            inner: ClassicalExpr::value(value.inner),
        }
    }

    #[staticmethod]
    fn bool_literal(value: bool) -> Self {
        Self {
            inner: ClassicalExpr::bool_literal(value),
        }
    }

    #[staticmethod]
    fn bit_literal(value: bool) -> Self {
        Self {
            inner: ClassicalExpr::bit_literal(value),
        }
    }

    #[staticmethod]
    fn uint_literal(width: u32, value: u128) -> PyResult<Self> {
        ClassicalExpr::uint_literal(width, value)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn bit_vec_literal(width: u32, value: u128) -> PyResult<Self> {
        ClassicalExpr::bit_vec_literal(width, value)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    #[getter]
    fn ty(&self) -> PyClassicalType {
        self.inner.ty().into()
    }

    fn not_(&self) -> PyResult<Self> {
        ClassicalExpr::try_not(self.inner.clone())
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn and_(&self, rhs: PyClassicalExpr) -> PyResult<Self> {
        ClassicalExpr::try_and(self.inner.clone(), rhs.inner)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn or_(&self, rhs: PyClassicalExpr) -> PyResult<Self> {
        ClassicalExpr::try_or(self.inner.clone(), rhs.inner)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn xor(&self, rhs: PyClassicalExpr) -> PyResult<Self> {
        ClassicalExpr::try_xor(self.inner.clone(), rhs.inner)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn bit_to_bool(&self) -> PyResult<Self> {
        ClassicalExpr::bit_to_bool(self.inner.clone())
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn to_bool(&self) -> PyResult<Self> {
        self.inner
            .clone()
            .to_bool()
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn bit_vec_to_uint(&self) -> PyResult<Self> {
        ClassicalExpr::bit_vec_to_uint(self.inner.clone())
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn to_uint(&self) -> PyResult<Self> {
        self.inner
            .clone()
            .to_uint()
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn equal(lhs: PyClassicalExpr, rhs: PyClassicalExpr) -> PyResult<Self> {
        ClassicalExpr::eq(lhs.inner, rhs.inner)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn not_equal(lhs: PyClassicalExpr, rhs: PyClassicalExpr) -> PyResult<Self> {
        ClassicalExpr::ne(lhs.inner, rhs.inner)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn lt(lhs: PyClassicalExpr, rhs: PyClassicalExpr) -> PyResult<Self> {
        ClassicalExpr::lt(lhs.inner, rhs.inner)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn le(lhs: PyClassicalExpr, rhs: PyClassicalExpr) -> PyResult<Self> {
        ClassicalExpr::le(lhs.inner, rhs.inner)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn gt(lhs: PyClassicalExpr, rhs: PyClassicalExpr) -> PyResult<Self> {
        ClassicalExpr::gt(lhs.inner, rhs.inner)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn ge(lhs: PyClassicalExpr, rhs: PyClassicalExpr) -> PyResult<Self> {
        ClassicalExpr::ge(lhs.inner, rhs.inner)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn select(
        condition: PyClassicalExpr,
        then_expr: PyClassicalExpr,
        else_expr: PyClassicalExpr,
    ) -> PyResult<Self> {
        ClassicalExpr::select(condition.inner, then_expr.inner, else_expr.inner)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn extract_bit(&self, index: u32) -> PyResult<Self> {
        ClassicalExpr::extract_bit(self.inner.clone(), index)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn extract_bits(&self, offset: u32, width: u32) -> PyResult<Self> {
        ClassicalExpr::extract_bits(self.inner.clone(), offset, width)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn concat(parts: Vec<PyClassicalExpr>) -> PyResult<Self> {
        ClassicalExpr::concat(parts.into_iter().map(|part| part.inner))
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn pack_bits(bits: Vec<PyClassicalExpr>) -> PyResult<Self> {
        ClassicalExpr::pack_bits(bits.into_iter().map(|bit| bit.inner))
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn simplified(&self) -> Self {
        Self {
            inner: self.inner.simplified(),
        }
    }

    fn is_bool_true(&self) -> bool {
        self.inner.is_bool_true()
    }

    fn is_bool_false(&self) -> bool {
        self.inner.is_bool_false()
    }

    fn is_bit_true(&self) -> bool {
        self.inner.is_bit_true()
    }

    fn is_bit_false(&self) -> bool {
        self.inner.is_bit_false()
    }

    fn __invert__(&self) -> PyResult<Self> {
        self.not_()
    }

    fn __and__(&self, rhs: PyClassicalExpr) -> PyResult<Self> {
        self.and_(rhs)
    }

    fn __or__(&self, rhs: PyClassicalExpr) -> PyResult<Self> {
        self.or_(rhs)
    }

    fn __xor__(&self, rhs: PyClassicalExpr) -> PyResult<Self> {
        self.xor(rhs)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!("ClassicalExpr({:?})", self.inner.kind())
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

impl From<ClassicalExpr> for PyClassicalExpr {
    fn from(inner: ClassicalExpr) -> Self {
        Self { inner }
    }
}
