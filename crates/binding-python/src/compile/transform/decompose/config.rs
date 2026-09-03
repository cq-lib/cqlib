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

use crate::circuit::PyInstruction;
use crate::compile::error::compiler_error_to_py_err;
use crate::compile::resource::{PyResourceLimits, PyResourcePolicy};
use crate::compile::target_basis_item::PyTargetBasisItem;
use crate::utils::{hash_value, python_string_literal};
use cqlib_core::circuit::{Instruction, StandardGate};
use cqlib_core::compile::CompilerError;
use cqlib_core::compile::transform::decompose::unitary::{
    TwoQubitSynthesisTarget, TwoQubitUnitaryDecomposeBasis,
};
use cqlib_core::compile::transform::decompose::{McGateDecomposeConfig, UnitaryDecomposeConfig};
use pyo3::prelude::*;

/// Output interaction basis used for numeric two-qubit unitary synthesis.
#[pyclass(
    name = "TwoQubitUnitaryDecomposeBasis",
    module = "cqlib.compile.transform.decompose",
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyTwoQubitUnitaryDecomposeBasis {
    pub(crate) inner: TwoQubitUnitaryDecomposeBasis,
}

#[pymethods]
impl PyTwoQubitUnitaryDecomposeBasis {
    #[staticmethod]
    fn pauli_rotations() -> Self {
        Self {
            inner: TwoQubitUnitaryDecomposeBasis::PauliRotations,
        }
    }

    #[staticmethod]
    fn cx() -> Self {
        Self {
            inner: TwoQubitUnitaryDecomposeBasis::Cx,
        }
    }

    #[staticmethod]
    fn cy() -> Self {
        Self {
            inner: TwoQubitUnitaryDecomposeBasis::Cy,
        }
    }

    #[staticmethod]
    fn cz() -> Self {
        Self {
            inner: TwoQubitUnitaryDecomposeBasis::Cz,
        }
    }

    #[staticmethod]
    fn rzz() -> Self {
        Self {
            inner: TwoQubitUnitaryDecomposeBasis::Rzz,
        }
    }

    fn __repr__(&self) -> &'static str {
        self.repr_value()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }
}

impl PyTwoQubitUnitaryDecomposeBasis {
    /// Returns the eval-able Python constructor form of this basis.
    pub(crate) const fn repr_value(&self) -> &'static str {
        match self.inner {
            TwoQubitUnitaryDecomposeBasis::PauliRotations => {
                "TwoQubitUnitaryDecomposeBasis.pauli_rotations()"
            }
            TwoQubitUnitaryDecomposeBasis::Cx => "TwoQubitUnitaryDecomposeBasis.cx()",
            TwoQubitUnitaryDecomposeBasis::Cy => "TwoQubitUnitaryDecomposeBasis.cy()",
            TwoQubitUnitaryDecomposeBasis::Cz => "TwoQubitUnitaryDecomposeBasis.cz()",
            TwoQubitUnitaryDecomposeBasis::Rzz => "TwoQubitUnitaryDecomposeBasis.rzz()",
        }
    }
}

/// Configuration for matrix-backed unitary decomposition.
#[pyclass(
    name = "UnitaryDecomposeConfig",
    module = "cqlib.compile.transform.decompose",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyUnitaryDecomposeConfig {
    pub(crate) inner: UnitaryDecomposeConfig,
    two_qubit_basis: Option<PyTwoQubitUnitaryDecomposeBasis>,
    target_basis: Option<Vec<Instruction>>,
}

#[pymethods]
impl PyUnitaryDecomposeConfig {
    #[new]
    #[pyo3(signature = (*, two_qubit_basis=None, target_basis=None, recurse_control_flow=true))]
    fn new(
        py: Python<'_>,
        two_qubit_basis: Option<PyTwoQubitUnitaryDecomposeBasis>,
        target_basis: Option<Vec<PyTargetBasisItem>>,
        recurse_control_flow: bool,
    ) -> PyResult<Self> {
        if two_qubit_basis.is_some() && target_basis.is_some() {
            return Err(compiler_error_to_py_err(CompilerError::InvalidInput(
                "two_qubit_basis and target_basis are mutually exclusive".to_string(),
            )));
        }
        // `None` selects the core default (unconstrained) synthesis target so
        // that `UnitaryDecomposeConfig()` matches omitting the config in
        // `decompose_unitaries`. `target_basis` mirrors the compiler workflow
        // path via `TwoQubitSynthesisTarget::from_instructions`; legacy
        // Pauli-rotation output is still available via
        // `TwoQubitUnitaryDecomposeBasis.pauli_rotations()`.
        let target_basis = target_basis
            .map(|basis| {
                basis
                    .into_iter()
                    .map(PyTargetBasisItem::into_instruction)
                    .collect::<PyResult<Vec<_>>>()
            })
            .transpose()?;
        let two_qubit_target = match &target_basis {
            Some(basis) => py
                .detach(|| TwoQubitSynthesisTarget::from_instructions(Some(basis)))
                .map_err(compiler_error_to_py_err)?,
            None => two_qubit_basis.map_or_else(TwoQubitSynthesisTarget::default, |basis| {
                target_for_basis(basis.inner)
            }),
        };
        Ok(Self {
            inner: UnitaryDecomposeConfig {
                two_qubit_target,
                recurse_control_flow,
            },
            two_qubit_basis,
            target_basis,
        })
    }

    #[getter]
    fn two_qubit_basis(&self) -> Option<PyTwoQubitUnitaryDecomposeBasis> {
        self.two_qubit_basis
    }

    #[getter]
    fn target_basis(&self) -> Option<Vec<PyInstruction>> {
        self.target_basis
            .as_ref()
            .map(|basis| basis.iter().cloned().map(Into::into).collect::<Vec<_>>())
    }

    #[getter]
    fn recurse_control_flow(&self) -> bool {
        self.inner.recurse_control_flow
    }

    fn __repr__(&self) -> String {
        let two_qubit_basis = self.two_qubit_basis.map_or_else(
            || "None".to_string(),
            |basis| basis.repr_value().to_string(),
        );
        let target_basis = format_target_basis_repr(&self.target_basis);
        format!(
            "UnitaryDecomposeConfig(two_qubit_basis={}, target_basis={}, recurse_control_flow={})",
            two_qubit_basis,
            target_basis,
            if self.inner.recurse_control_flow {
                "True"
            } else {
                "False"
            }
        )
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

/// Formats an optional target-basis instruction list for Python reprs.
pub(crate) fn format_target_basis_repr(target_basis: &Option<Vec<Instruction>>) -> String {
    target_basis.as_ref().map_or_else(
        || "None".to_string(),
        |basis| {
            format!(
                "[{}]",
                basis
                    .iter()
                    .map(|instruction| python_string_literal(&instruction.name()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
    )
}

pub(crate) fn target_for_basis(basis: TwoQubitUnitaryDecomposeBasis) -> TwoQubitSynthesisTarget {
    let (native_1q, native_2q) = match basis {
        TwoQubitUnitaryDecomposeBasis::PauliRotations => (
            vec![StandardGate::U],
            vec![StandardGate::RXX, StandardGate::RYY, StandardGate::RZZ],
        ),
        TwoQubitUnitaryDecomposeBasis::Cx => (vec![StandardGate::U], vec![StandardGate::CX]),
        TwoQubitUnitaryDecomposeBasis::Cy => (vec![StandardGate::U], vec![StandardGate::CY]),
        TwoQubitUnitaryDecomposeBasis::Cz => (vec![StandardGate::U], vec![StandardGate::CZ]),
        TwoQubitUnitaryDecomposeBasis::Rzz => (
            vec![StandardGate::U, StandardGate::H, StandardGate::RX],
            vec![StandardGate::RZZ],
        ),
    };
    TwoQubitSynthesisTarget::from_standard_gates(native_1q, native_2q, false)
        .expect("legacy two-qubit basis must define a valid synthesis target")
}

/// Configuration for resource-aware multi-controlled-gate decomposition.
#[pyclass(
    name = "McGateDecomposeConfig",
    module = "cqlib.compile.transform.decompose",
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyMcGateDecomposeConfig {
    pub(crate) inner: McGateDecomposeConfig,
}

#[pymethods]
impl PyMcGateDecomposeConfig {
    #[new]
    #[pyo3(signature = (*, resource_policy=None, resource_limits=None))]
    fn new(
        resource_policy: Option<PyResourcePolicy>,
        resource_limits: Option<PyResourceLimits>,
    ) -> Self {
        Self {
            inner: McGateDecomposeConfig {
                resource_policy: resource_policy.map_or_else(Default::default, |value| value.inner),
                resource_limits: resource_limits.map_or_else(Default::default, |value| value.inner),
            },
        }
    }

    #[getter]
    fn resource_policy(&self) -> PyResourcePolicy {
        self.inner.resource_policy.into()
    }

    #[getter]
    fn resource_limits(&self) -> PyResourceLimits {
        self.inner.resource_limits.into()
    }

    fn __repr__(&self) -> String {
        let max_total_qubits = self
            .inner
            .resource_limits
            .max_total_qubits
            .map_or_else(|| "None".to_string(), |value| value.to_string());
        format!(
            "McGateDecomposeConfig(resource_policy=ResourcePolicy(max_pre_layout_clean_ancillas={}, allow_dirty_borrowing={}), resource_limits=ResourceLimits(max_total_qubits={}))",
            self.inner.resource_policy.max_pre_layout_clean_ancillas,
            if self.inner.resource_policy.allow_dirty_borrowing {
                "True"
            } else {
                "False"
            },
            max_total_qubits,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }
}
