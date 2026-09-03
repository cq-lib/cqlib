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

use crate::circuit::{PyParameter, PyValueOperation};
use crate::compile::knowledge::library::PyRuleLibrary;
use crate::utils::python_bool;
use cqlib_core::circuit::{Instruction, Parameter, Qubit, ValueOperation};
use cqlib_core::compile::commutation::{
    Commutation, CommutationChecker, CommutationConfig, algebraic_commutation, check_commutation,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Python wrapper for a proven commutation relationship.
#[pyclass(
    name = "Commutation",
    module = "cqlib.compile.commutation",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCommutation {
    inner: Commutation,
}

impl From<Commutation> for PyCommutation {
    fn from(inner: Commutation) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCommutation {
    /// Creates an exact commutation proof.
    #[staticmethod]
    fn exact() -> Self {
        Commutation::Exact.into()
    }

    /// Creates a commutation proof valid up to the supplied global phase.
    #[staticmethod]
    fn up_to_global_phase(phase: PyParameter) -> Self {
        Commutation::UpToGlobalPhase(phase.into_inner()).into()
    }

    /// Returns whether the operations commute without a global phase.
    fn is_exact(&self) -> bool {
        self.inner.is_exact()
    }

    /// Returns the proof's global phase, or zero for exact commutation.
    #[getter]
    fn phase(&self) -> PyParameter {
        self.inner.phase().into()
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            Commutation::Exact => "Commutation.exact()".to_string(),
            Commutation::UpToGlobalPhase(phase) => format!(
                "Commutation.up_to_global_phase(Parameter({:?}))",
                phase.to_string()
            ),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Python wrapper for commutation checker configuration.
#[pyclass(
    name = "CommutationConfig",
    module = "cqlib.compile.commutation",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCommutationConfig {
    pub(crate) inner: CommutationConfig,
}

impl From<CommutationConfig> for PyCommutationConfig {
    fn from(inner: CommutationConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCommutationConfig {
    /// Creates a commutation checker configuration.
    #[new]
    #[pyo3(signature = (*, enable_rule_oracle=true, enable_matrix_fallback=true, max_matrix_qubits=4))]
    fn new(
        enable_rule_oracle: bool,
        enable_matrix_fallback: bool,
        max_matrix_qubits: usize,
    ) -> Self {
        Self {
            inner: CommutationConfig {
                enable_rule_oracle,
                enable_matrix_fallback,
                max_matrix_qubits,
            },
        }
    }

    #[getter]
    fn enable_rule_oracle(&self) -> bool {
        self.inner.enable_rule_oracle
    }

    #[getter]
    fn enable_matrix_fallback(&self) -> bool {
        self.inner.enable_matrix_fallback
    }

    #[getter]
    fn max_matrix_qubits(&self) -> usize {
        self.inner.max_matrix_qubits
    }

    fn __repr__(&self) -> String {
        self.repr_value()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

impl PyCommutationConfig {
    /// Returns the eval-able Python constructor form of this configuration.
    pub(crate) fn repr_value(&self) -> String {
        format!(
            "CommutationConfig(enable_rule_oracle={}, enable_matrix_fallback={}, max_matrix_qubits={})",
            python_bool(self.inner.enable_rule_oracle),
            python_bool(self.inner.enable_matrix_fallback),
            self.inner.max_matrix_qubits
        )
    }
}

/// Reusable Python wrapper around the core commutation checker.
#[pyclass(
    name = "CommutationChecker",
    module = "cqlib.compile.commutation",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCommutationChecker {
    inner: CommutationChecker,
}

#[pymethods]
impl PyCommutationChecker {
    /// Builds a checker with builtin rules and default configuration.
    #[staticmethod]
    fn builtin() -> Self {
        Self {
            inner: CommutationChecker::builtin(),
        }
    }

    /// Builds a checker with builtin rules and an explicit configuration.
    #[staticmethod]
    fn with_config(config: PyCommutationConfig) -> Self {
        Self {
            inner: CommutationChecker::with_config(config.inner),
        }
    }

    /// Builds a checker using commute rules from an already loaded library.
    ///
    /// Only rules registered under ``RuleKind.commute()`` participate in
    /// proofs. ``config=None`` uses the default configuration, matching
    /// :meth:`builtin`. Raises ``ValueError`` when ``enable_rule_oracle`` is
    /// disabled, because the library would otherwise be silently ignored.
    #[staticmethod]
    #[pyo3(signature = (library, config=None))]
    fn from_library(
        library: PyRef<'_, PyRuleLibrary>,
        config: Option<PyCommutationConfig>,
    ) -> PyResult<Self> {
        let config = config.unwrap_or_else(|| CommutationConfig::default().into());
        if !config.inner.enable_rule_oracle {
            return Err(PyValueError::new_err(
                "config.enable_rule_oracle must be True when building a checker from a library",
            ));
        }
        Ok(Self {
            inner: CommutationChecker::from_library(&library.inner, config.inner),
        })
    }

    /// Returns a copy of the active checker configuration.
    #[getter]
    fn config(&self) -> PyCommutationConfig {
        self.inner.config().clone().into()
    }

    /// Checks whether two self-contained operation values commute.
    fn check(
        &self,
        lhs: PyRef<'_, PyValueOperation>,
        rhs: PyRef<'_, PyValueOperation>,
    ) -> Option<PyCommutation> {
        check_pair(&self.inner, &lhs.inner, &rhs.inner).map(Into::into)
    }

    fn __repr__(&self) -> String {
        let config = self.inner.config();
        format!(
            "CommutationChecker(config=CommutationConfig(enable_rule_oracle={}, enable_matrix_fallback={}, max_matrix_qubits={}))",
            config.enable_rule_oracle, config.enable_matrix_fallback, config.max_matrix_qubits
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Checks commutation using the shared builtin checker.
#[pyfunction(name = "check_commutation")]
pub fn py_check_commutation(
    lhs: PyRef<'_, PyValueOperation>,
    rhs: PyRef<'_, PyValueOperation>,
) -> Option<PyCommutation> {
    check_builtin_pair(&lhs.inner, &rhs.inner).map(Into::into)
}

/// Checks commutation using only the symbolic algebra oracle.
#[pyfunction(name = "algebraic_commutation")]
pub fn py_algebraic_commutation(
    lhs: PyRef<'_, PyValueOperation>,
    rhs: PyRef<'_, PyValueOperation>,
) -> Option<PyCommutation> {
    algebraic_pair(&lhs.inner, &rhs.inner).map(Into::into)
}

fn operation_parts(operation: &ValueOperation) -> Option<(&Instruction, &[Qubit], Vec<Parameter>)> {
    let instruction = operation.instruction.as_instruction()?;
    let params = operation.params.iter().map(Parameter::from).collect();
    Some((instruction, operation.qubits.as_slice(), params))
}

fn check_pair(
    checker: &CommutationChecker,
    lhs: &ValueOperation,
    rhs: &ValueOperation,
) -> Option<Commutation> {
    let (lhs_inst, lhs_qubits, lhs_params) = operation_parts(lhs)?;
    let (rhs_inst, rhs_qubits, rhs_params) = operation_parts(rhs)?;
    checker.check(
        lhs_inst,
        lhs_qubits,
        &lhs_params,
        rhs_inst,
        rhs_qubits,
        &rhs_params,
    )
}

fn check_builtin_pair(lhs: &ValueOperation, rhs: &ValueOperation) -> Option<Commutation> {
    let (lhs_inst, lhs_qubits, lhs_params) = operation_parts(lhs)?;
    let (rhs_inst, rhs_qubits, rhs_params) = operation_parts(rhs)?;
    check_commutation(
        lhs_inst,
        lhs_qubits,
        &lhs_params,
        rhs_inst,
        rhs_qubits,
        &rhs_params,
    )
}

fn algebraic_pair(lhs: &ValueOperation, rhs: &ValueOperation) -> Option<Commutation> {
    let (lhs_inst, lhs_qubits, lhs_params) = operation_parts(lhs)?;
    let (rhs_inst, rhs_qubits, rhs_params) = operation_parts(rhs)?;
    algebraic_commutation(
        lhs_inst,
        lhs_qubits,
        &lhs_params,
        rhs_inst,
        rhs_qubits,
        &rhs_params,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cqlib_core::circuit::{
        ParameterValue, StandardGate, ValueClassicalControlOp, ValueInstruction,
    };
    use std::f64::consts::PI;

    fn operation(
        gate: StandardGate,
        qubits: impl IntoIterator<Item = Qubit>,
        params: impl IntoIterator<Item = ParameterValue>,
    ) -> ValueOperation {
        ValueOperation::from_standard(gate, qubits, params)
    }

    #[test]
    fn value_operations_preserve_symbolic_parameters() {
        let lhs = operation(
            StandardGate::RZ,
            [Qubit::new(0)],
            [ParameterValue::Param(Parameter::symbol("a"))],
        );
        let rhs = operation(
            StandardGate::RZ,
            [Qubit::new(0)],
            [ParameterValue::Param(Parameter::symbol("b"))],
        );

        assert_eq!(check_builtin_pair(&lhs, &rhs), Some(Commutation::Exact));
    }

    #[test]
    fn algebraic_pair_preserves_global_phase_proof() {
        let lhs = operation(StandardGate::X, [Qubit::new(0)], []);
        let rhs = operation(StandardGate::Z, [Qubit::new(0)], []);

        let Some(Commutation::UpToGlobalPhase(phase)) = algebraic_pair(&lhs, &rhs) else {
            panic!("expected a commutation proof up to global phase");
        };
        assert!((phase.evaluate(&None).unwrap() - PI).abs() < 1e-10);
    }

    #[test]
    fn classical_control_is_outside_commutation_checker() {
        let classical = ValueOperation {
            instruction: ValueInstruction::ClassicalControl(ValueClassicalControlOp::Break),
            qubits: Default::default(),
            params: Default::default(),
            label: None,
        };
        let gate = operation(StandardGate::X, [Qubit::new(0)], []);

        assert_eq!(check_builtin_pair(&classical, &gate), None);
    }
}
