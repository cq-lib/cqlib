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

//! Python wrappers for construction-time classical control flow.
//!
//! Bodies contain [`ValueOperation`](cqlib_core::circuit::ValueOperation)
//! values rather than circuit-local storage operations. This preserves the
//! core distinction between self-contained construction IR and interned circuit
//! storage IR.

use crate::circuit::circuit_impl::PyCircuit;
use crate::circuit::classical::{PyClassicalValue, PyClassicalVar};
use crate::circuit::classical_expr::PyClassicalExpr;
use crate::circuit::error::CircuitError as PyCircuitError;
use crate::circuit::operation::PyValueOperation;
use cqlib_core::circuit::{
    ControlBody, ControlBodyTransaction, ExternalControlScope, SwitchCase, ValueClassicalControlOp,
    ValueControlBody, ValueSwitchCase,
};
use pyo3::prelude::*;

/// Ordered construction-time operations owned by one control-flow region.
#[pyclass(name = "ValueControlBody", module = "cqlib.circuit", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyValueControlBody {
    pub(crate) inner: ValueControlBody,
}

impl From<ValueControlBody> for PyValueControlBody {
    fn from(inner: ValueControlBody) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyValueControlBody {
    #[new]
    fn new(operations: Vec<PyValueOperation>) -> Self {
        Self {
            inner: ValueControlBody::new(operations.into_iter().map(|op| op.inner).collect()),
        }
    }

    #[getter]
    fn operations(&self) -> Vec<PyValueOperation> {
        self.inner
            .operations()
            .iter()
            .cloned()
            .map(PyValueOperation::from)
            .collect()
    }

    fn __len__(&self) -> usize {
        self.inner.operations().len()
    }

    /// Returns true if this body directly or recursively contains measurement.
    fn has_measurement(&self) -> bool {
        self.inner.has_measurement()
    }

    /// Returns true if this body directly or recursively reads `value`.
    fn reads_value(&self, value: PyClassicalValue) -> bool {
        self.inner.reads_value(value.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "ValueControlBody({} operations)",
            self.inner.operations().len()
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Exact integer match and body used by a construction-time switch.
#[pyclass(name = "ValueSwitchCase", module = "cqlib.circuit", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyValueSwitchCase {
    pub(crate) inner: ValueSwitchCase,
}

impl From<ValueSwitchCase> for PyValueSwitchCase {
    fn from(inner: ValueSwitchCase) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyValueSwitchCase {
    #[new]
    fn new(value: u128, body: PyValueControlBody) -> Self {
        Self {
            inner: ValueSwitchCase::new(value, body.inner),
        }
    }

    #[getter]
    fn value(&self) -> u128 {
        self.inner.value
    }

    #[getter]
    fn body(&self) -> PyValueControlBody {
        self.inner.body.clone().into()
    }

    fn __repr__(&self) -> String {
        format!("ValueSwitchCase({})", self.inner.value)
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Construction-time classical control-flow operation.
///
/// This wraps `ValueClassicalControlOp`; it is not a quantum gate and has no
/// unitary matrix representation.
#[pyclass(name = "ClassicalControlOp", module = "cqlib.circuit", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyClassicalControlOp {
    pub(crate) inner: ValueClassicalControlOp,
}

/// Temporary callback builder used by `Circuit.switch`.
#[pyclass(name = "_SwitchBuilder", module = "cqlib.circuit", skip_from_py_object)]
pub struct PySwitchBuilder {
    pub(crate) circuit: Py<PyCircuit>,
    pub(crate) transaction: Option<ControlBodyTransaction>,
    pub(crate) cases: Vec<SwitchCase>,
    pub(crate) default: Option<ControlBody>,
    pub(crate) closed: bool,
}

impl PySwitchBuilder {
    pub(crate) fn new(circuit: Py<PyCircuit>, transaction: ControlBodyTransaction) -> Self {
        Self {
            circuit,
            transaction: Some(transaction),
            cases: Vec::new(),
            default: None,
            closed: false,
        }
    }

    fn transaction(&self) -> PyResult<&ControlBodyTransaction> {
        if self.closed {
            return Err(PyCircuitError::new_err(
                "switch builder is only valid during its callback",
            ));
        }
        self.transaction
            .as_ref()
            .ok_or_else(|| PyCircuitError::new_err("switch builder transaction is unavailable"))
    }
}

#[pymethods]
impl PySwitchBuilder {
    /// Adds an exact-value switch case through a scoped circuit callback.
    fn value<'py>(
        &mut self,
        py: Python<'py>,
        value: u128,
        body: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        self.transaction()?;
        let transaction = {
            let mut circuit = self.circuit.bind(py).borrow_mut();
            let transaction = circuit.inner.begin_control_body_transaction();
            circuit
                .inner
                .enter_external_control_body(&transaction, ExternalControlScope::Switch);
            transaction
        };
        if let Err(error) = body.call1((self.circuit.bind(py),)) {
            self.circuit
                .bind(py)
                .borrow_mut()
                .inner
                .rollback_control_body_transaction(transaction);
            return Err(error);
        }
        let body = {
            let mut circuit = self.circuit.bind(py).borrow_mut();
            let body = circuit.inner.finish_external_control_body(&transaction);
            circuit.inner.commit_control_body_transaction(transaction);
            body
        };
        self.cases.push(SwitchCase::new(value, body));
        Ok(())
    }

    /// Adds the switch default case through a scoped circuit callback.
    fn default<'py>(&mut self, py: Python<'py>, body: &Bound<'py, PyAny>) -> PyResult<()> {
        if self.default.is_some() {
            return Err(PyCircuitError::new_err(
                "switch default case is already defined",
            ));
        }
        self.transaction()?;
        let transaction = {
            let mut circuit = self.circuit.bind(py).borrow_mut();
            let transaction = circuit.inner.begin_control_body_transaction();
            circuit
                .inner
                .enter_external_control_body(&transaction, ExternalControlScope::Switch);
            transaction
        };
        if let Err(error) = body.call1((self.circuit.bind(py),)) {
            self.circuit
                .bind(py)
                .borrow_mut()
                .inner
                .rollback_control_body_transaction(transaction);
            return Err(error);
        }
        let body = {
            let mut circuit = self.circuit.bind(py).borrow_mut();
            let body = circuit.inner.finish_external_control_body(&transaction);
            circuit.inner.commit_control_body_transaction(transaction);
            body
        };
        self.default = Some(body);
        Ok(())
    }
}

impl From<ValueClassicalControlOp> for PyClassicalControlOp {
    fn from(inner: ValueClassicalControlOp) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyClassicalControlOp {
    #[staticmethod]
    #[pyo3(signature = (condition, then_body, else_body=None))]
    fn if_(
        condition: PyClassicalExpr,
        then_body: PyValueControlBody,
        else_body: Option<PyValueControlBody>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: ValueClassicalControlOp::new_if(
                condition.inner,
                then_body.inner,
                else_body.map(|body| body.inner),
            )
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?,
        })
    }

    #[staticmethod]
    fn while_(condition: PyClassicalExpr, body: PyValueControlBody) -> PyResult<Self> {
        Ok(Self {
            inner: ValueClassicalControlOp::new_while(condition.inner, body.inner)
                .map_err(|error| PyCircuitError::new_err(error.to_string()))?,
        })
    }

    #[staticmethod]
    fn for_uint(
        var: PyClassicalVar,
        start: PyClassicalExpr,
        stop: PyClassicalExpr,
        step: PyClassicalExpr,
        body: PyValueControlBody,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: ValueClassicalControlOp::new_for(
                var.inner,
                start.inner,
                stop.inner,
                step.inner,
                body.inner,
            )
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?,
        })
    }

    #[staticmethod]
    #[pyo3(signature = (target, cases, default=None))]
    fn switch(
        target: PyClassicalExpr,
        cases: Vec<PyValueSwitchCase>,
        default: Option<PyValueControlBody>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: ValueClassicalControlOp::new_switch(
                target.inner,
                cases.into_iter().map(|case| case.inner).collect(),
                default.map(|body| body.inner),
            )
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?,
        })
    }

    #[staticmethod]
    fn break_() -> Self {
        Self {
            inner: ValueClassicalControlOp::Break,
        }
    }

    #[staticmethod]
    fn continue_() -> Self {
        Self {
            inner: ValueClassicalControlOp::Continue,
        }
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            ValueClassicalControlOp::If { .. } => "if",
            ValueClassicalControlOp::While { .. } => "while",
            ValueClassicalControlOp::For { .. } => "for",
            ValueClassicalControlOp::Switch { .. } => "switch",
            ValueClassicalControlOp::Break => "break",
            ValueClassicalControlOp::Continue => "continue",
        }
    }

    #[getter]
    fn condition(&self) -> Option<PyClassicalExpr> {
        match &self.inner {
            ValueClassicalControlOp::If { condition, .. }
            | ValueClassicalControlOp::While { condition, .. } => Some(condition.clone().into()),
            _ => None,
        }
    }

    #[getter]
    fn then_body(&self) -> Option<PyValueControlBody> {
        match &self.inner {
            ValueClassicalControlOp::If { then_body, .. } => Some(then_body.clone().into()),
            _ => None,
        }
    }

    #[getter]
    fn else_body(&self) -> Option<PyValueControlBody> {
        match &self.inner {
            ValueClassicalControlOp::If { else_body, .. } => else_body.clone().map(Into::into),
            _ => None,
        }
    }

    #[getter]
    fn body(&self) -> Option<PyValueControlBody> {
        match &self.inner {
            ValueClassicalControlOp::While { body, .. }
            | ValueClassicalControlOp::For { body, .. } => Some(body.clone().into()),
            _ => None,
        }
    }

    #[getter]
    fn var(&self) -> Option<PyClassicalVar> {
        match &self.inner {
            ValueClassicalControlOp::For { var, .. } => Some((*var).into()),
            _ => None,
        }
    }

    #[getter]
    fn start(&self) -> Option<PyClassicalExpr> {
        match &self.inner {
            ValueClassicalControlOp::For { start, .. } => Some(start.clone().into()),
            _ => None,
        }
    }

    #[getter]
    fn stop(&self) -> Option<PyClassicalExpr> {
        match &self.inner {
            ValueClassicalControlOp::For { stop, .. } => Some(stop.clone().into()),
            _ => None,
        }
    }

    #[getter]
    fn step(&self) -> Option<PyClassicalExpr> {
        match &self.inner {
            ValueClassicalControlOp::For { step, .. } => Some(step.clone().into()),
            _ => None,
        }
    }

    #[getter]
    fn target(&self) -> Option<PyClassicalExpr> {
        match &self.inner {
            ValueClassicalControlOp::Switch { target, .. } => Some(target.clone().into()),
            _ => None,
        }
    }

    #[getter]
    fn cases(&self) -> Vec<PyValueSwitchCase> {
        match &self.inner {
            ValueClassicalControlOp::Switch { cases, .. } => {
                cases.iter().cloned().map(Into::into).collect()
            }
            _ => Vec::new(),
        }
    }

    #[getter]
    fn default(&self) -> Option<PyValueControlBody> {
        match &self.inner {
            ValueClassicalControlOp::Switch { default, .. } => default.clone().map(Into::into),
            _ => None,
        }
    }

    /// Returns true if this control operation recursively contains measurement.
    fn has_measurement(&self) -> bool {
        self.inner.has_measurement()
    }

    /// Returns true if this control operation recursively reads `value`.
    fn reads_value(&self, value: PyClassicalValue) -> bool {
        self.inner.reads_value(value.inner)
    }

    fn __repr__(&self) -> String {
        format!("ClassicalControlOp({})", self.kind())
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cqlib_core::circuit::gate::ClassicalDataOp;
    use cqlib_core::circuit::{
        CircuitId, ClassicalExpr, ClassicalType, ClassicalValue, Instruction, Qubit,
        ValueInstruction, ValueOperation,
    };

    #[test]
    fn if_control_is_fully_observable() {
        let control = PyClassicalControlOp::from(ValueClassicalControlOp::If {
            condition: ClassicalExpr::bool_literal(true),
            then_body: ValueControlBody::new(vec![]),
            else_body: Some(ValueControlBody::new(vec![])),
        });

        assert!(control.condition().unwrap().inner.is_bool_true());
        assert!(control.then_body().is_some());
        assert!(control.else_body().is_some());
    }

    #[test]
    fn value_body_reports_measurement_through_python_wrapper() {
        let value = ClassicalValue::new(CircuitId::default(), 0, ClassicalType::Bit);
        let body = PyValueControlBody::from(ValueControlBody::new(vec![ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::ClassicalData(
                ClassicalDataOp::MeasureBit { result: value },
            )),
            qubits: smallvec::smallvec![Qubit::new(0)],
            params: smallvec::smallvec![],
            label: None,
        }]));

        assert!(body.has_measurement());
    }

    #[test]
    fn control_reports_value_reads_through_python_wrapper() {
        let value = ClassicalValue::new(CircuitId::default(), 0, ClassicalType::Bit);
        let control = PyClassicalControlOp::from(ValueClassicalControlOp::If {
            condition: value.expr().to_bool().unwrap(),
            then_body: ValueControlBody::new(vec![]),
            else_body: None,
        });

        assert!(control.reads_value(PyClassicalValue::from(value)));
    }
}
