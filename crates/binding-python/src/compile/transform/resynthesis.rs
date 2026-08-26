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

//! Python bindings for numeric two-qubit block resynthesis.

use super::PyTransformResult;
use super::decompose::config::{
    PyTwoQubitUnitaryDecomposeBasis, format_target_basis_repr, target_for_basis,
};
use crate::circuit::{PyCircuit, PyInstruction};
use crate::compile::commutation::PyCommutationConfig;
use crate::compile::error::compiler_error_to_py_err;
use crate::compile::target_basis_item::PyTargetBasisItem;
use crate::device::device_impl::PyDevice;
use crate::utils::python_bool;
use cqlib_core::circuit::Instruction;
use cqlib_core::compile::CompilerError;
use cqlib_core::compile::transform::{
    DeviceResynthesisPlacement, ResynthesizeTwoQubitBlocks, Transformer,
    TwoQubitBlockResynthesisConfig, resynthesize_two_qubit_blocks,
    resynthesize_two_qubit_blocks_for_device,
};
use pyo3::prelude::*;

/// Registers resynthesis bindings as `_native.compile.transform.resynthesis`.
pub(crate) fn register_resynthesis_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "resynthesis")?;
    m.add_class::<PyTwoQubitBlockResynthesisConfig>()?;
    m.add_class::<PyResynthesizeTwoQubitBlocks>()?;
    m.add_function(pyo3::wrap_pyfunction!(
        py_resynthesize_two_qubit_blocks,
        &m
    )?)?;
    let device_resynthesis =
        pyo3::wrap_pyfunction!(py_resynthesize_two_qubit_blocks_for_device, &m)?;
    device_resynthesis.setattr("__module__", "cqlib.compile.transform.resynthesis")?;
    m.add_function(device_resynthesis)?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile.transform.resynthesis", &m)?;
    Ok(())
}

/// Configuration for numeric two-qubit block resynthesis.
#[pyclass(
    name = "TwoQubitBlockResynthesisConfig",
    module = "cqlib.compile.transform.resynthesis",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTwoQubitBlockResynthesisConfig {
    pub(crate) inner: TwoQubitBlockResynthesisConfig,
    two_qubit_basis: Option<PyTwoQubitUnitaryDecomposeBasis>,
    target_basis: Option<Vec<Instruction>>,
}

impl PyTwoQubitBlockResynthesisConfig {
    pub(crate) fn unconstrained(enhanced: bool) -> Self {
        let target = Default::default();
        let inner = if enhanced {
            TwoQubitBlockResynthesisConfig::enhanced(target)
        } else {
            TwoQubitBlockResynthesisConfig::normal(target)
        };
        Self {
            inner,
            two_qubit_basis: None,
            target_basis: None,
        }
    }
}

#[pymethods]
impl PyTwoQubitBlockResynthesisConfig {
    /// Creates a bounded numeric resynthesis configuration.
    #[allow(clippy::too_many_arguments)]
    #[new]
    #[pyo3(signature = (*, two_qubit_basis=None, target_basis=None, enhanced=false, max_block_ops=None, max_crossed_ops=None, max_scan_span=None, skip_labeled_ops=true, recurse_control_flow=true, commutation=None))]
    fn new(
        py: Python<'_>,
        two_qubit_basis: Option<PyTwoQubitUnitaryDecomposeBasis>,
        target_basis: Option<Vec<PyTargetBasisItem>>,
        enhanced: bool,
        max_block_ops: Option<usize>,
        max_crossed_ops: Option<usize>,
        max_scan_span: Option<usize>,
        skip_labeled_ops: bool,
        recurse_control_flow: bool,
        commutation: Option<PyCommutationConfig>,
    ) -> PyResult<Self> {
        if two_qubit_basis.is_some() && target_basis.is_some() {
            return Err(compiler_error_to_py_err(CompilerError::InvalidInput(
                "two_qubit_basis and target_basis are mutually exclusive".to_string(),
            )));
        }
        // `None` selects the core default (unconstrained) synthesis target so
        // that `TwoQubitBlockResynthesisConfig()` matches the core `Default`.
        // `target_basis` mirrors the compiler workflow path via
        // `TwoQubitSynthesisTarget::from_instructions`; legacy Pauli-rotation
        // output is still available via
        // `TwoQubitUnitaryDecomposeBasis.pauli_rotations()`.
        let target_basis = target_basis
            .map(|basis| {
                basis
                    .into_iter()
                    .map(PyTargetBasisItem::into_instruction)
                    .collect::<PyResult<Vec<_>>>()
            })
            .transpose()?;
        let target = match &target_basis {
            Some(basis) => py
                .detach(|| {
                    cqlib_core::compile::transform::decompose::unitary::TwoQubitSynthesisTarget::from_instructions(Some(basis))
                })
                .map_err(compiler_error_to_py_err)?,
            None => two_qubit_basis.map_or_else(
                cqlib_core::compile::transform::decompose::unitary::TwoQubitSynthesisTarget::default,
                |basis| target_for_basis(basis.inner),
            ),
        };
        let mut inner = if enhanced {
            TwoQubitBlockResynthesisConfig::enhanced(target)
        } else {
            TwoQubitBlockResynthesisConfig::normal(target)
        };
        if let Some(value) = max_block_ops {
            inner.max_block_ops = value;
        }
        if let Some(value) = max_crossed_ops {
            inner.max_crossed_ops = value;
        }
        if let Some(value) = max_scan_span {
            inner.max_scan_span = value;
        }
        inner.skip_labeled_ops = skip_labeled_ops;
        inner.recurse_control_flow = recurse_control_flow;
        if let Some(value) = commutation {
            inner.commutation = value.inner;
        }
        Ok(Self {
            inner,
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
    fn max_block_ops(&self) -> usize {
        self.inner.max_block_ops
    }

    #[getter]
    fn max_crossed_ops(&self) -> usize {
        self.inner.max_crossed_ops
    }

    #[getter]
    fn max_scan_span(&self) -> usize {
        self.inner.max_scan_span
    }

    #[getter]
    fn skip_labeled_ops(&self) -> bool {
        self.inner.skip_labeled_ops
    }

    #[getter]
    fn recurse_control_flow(&self) -> bool {
        self.inner.recurse_control_flow
    }

    #[getter]
    fn commutation(&self) -> PyCommutationConfig {
        self.inner.commutation.clone().into()
    }

    fn __repr__(&self) -> String {
        let two_qubit_basis = self
            .two_qubit_basis
            .map_or("None", |basis| basis.repr_value());
        let target_basis = format_target_basis_repr(&self.target_basis);
        let commutation = PyCommutationConfig::from(self.inner.commutation.clone()).repr_value();
        format!(
            "TwoQubitBlockResynthesisConfig(two_qubit_basis={}, target_basis={}, max_block_ops={}, max_crossed_ops={}, max_scan_span={}, skip_labeled_ops={}, recurse_control_flow={}, commutation={})",
            two_qubit_basis,
            target_basis,
            self.inner.max_block_ops,
            self.inner.max_crossed_ops,
            self.inner.max_scan_span,
            python_bool(self.inner.skip_labeled_ops),
            python_bool(self.inner.recurse_control_flow),
            commutation,
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

/// Reusable transformer for numeric two-qubit block resynthesis.
#[pyclass(
    name = "ResynthesizeTwoQubitBlocks",
    module = "cqlib.compile.transform.resynthesis",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyResynthesizeTwoQubitBlocks {
    config: PyTwoQubitBlockResynthesisConfig,
}

#[pymethods]
impl PyResynthesizeTwoQubitBlocks {
    #[new]
    #[pyo3(signature = (config=None))]
    fn new(py: Python<'_>, config: Option<PyTwoQubitBlockResynthesisConfig>) -> Self {
        Self {
            config: config.unwrap_or_else(|| {
                PyTwoQubitBlockResynthesisConfig::new(
                    py, None, None, false, None, None, None, true, true, None,
                )
                .expect("default resynthesis config must be valid")
            }),
        }
    }

    #[getter]
    fn config(&self) -> PyTwoQubitBlockResynthesisConfig {
        self.config.clone()
    }

    /// Runs resynthesis without mutating the input circuit.
    fn run(&self, py: Python<'_>, circuit: PyRef<'_, PyCircuit>) -> PyResult<PyTransformResult> {
        let circuit = circuit.inner.clone();
        let config = self.config.inner.clone();
        py.detach(move || {
            let outcome = ResynthesizeTwoQubitBlocks::new(config).transform(&circuit, None)?;
            Ok(PyTransformResult::from_outcome(circuit, outcome))
        })
        .map_err(compiler_error_to_py_err)
    }

    fn __repr__(&self) -> String {
        format!(
            "ResynthesizeTwoQubitBlocks(config={})",
            self.config.__repr__()
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.config.inner == other.config.inner
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Resynthesizes fixed numeric two-qubit blocks without mutating `circuit`.
#[pyfunction(name = "resynthesize_two_qubit_blocks")]
#[pyo3(signature = (circuit, config=None))]
fn py_resynthesize_two_qubit_blocks(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    config: Option<PyTwoQubitBlockResynthesisConfig>,
) -> PyResult<PyTransformResult> {
    let circuit = circuit.inner.clone();
    let config = config.map_or_else(
        || {
            PyTwoQubitBlockResynthesisConfig::new(
                py, None, None, false, None, None, None, true, true, None,
            )
            .expect("default resynthesis config must be valid")
            .inner
        },
        |value| value.inner,
    );
    py.detach(move || {
        let outcome = resynthesize_two_qubit_blocks(&circuit, config)?;
        Ok(PyTransformResult::from_outcome(circuit, outcome))
    })
    .map_err(compiler_error_to_py_err)
}

/// Runs device-aware resynthesis without mutating `circuit` or `device`.
#[pyfunction(name = "resynthesize_two_qubit_blocks_for_device")]
#[pyo3(signature = (circuit, device, *, placement="pre_layout_envelope", config=None))]
fn py_resynthesize_two_qubit_blocks_for_device(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
    device: PyRef<'_, PyDevice>,
    placement: &str,
    config: Option<PyTwoQubitBlockResynthesisConfig>,
) -> PyResult<PyTransformResult> {
    let placement = match placement {
        "pre_layout_envelope" => DeviceResynthesisPlacement::PreLayoutEnvelope,
        "exact_physical" => DeviceResynthesisPlacement::ExactPhysical,
        value => {
            return Err(compiler_error_to_py_err(CompilerError::InvalidInput(
                format!(
                    "unknown device resynthesis placement {value:?}; expected \
                     'pre_layout_envelope' or 'exact_physical'"
                ),
            )));
        }
    };
    let circuit = circuit.inner.clone();
    let device = device.inner.clone();
    let config = config.map_or_else(
        || {
            PyTwoQubitBlockResynthesisConfig::new(
                py, None, None, false, None, None, None, true, true, None,
            )
            .expect("default resynthesis config must be valid")
            .inner
        },
        |value| value.inner,
    );
    py.detach(move || {
        let outcome =
            resynthesize_two_qubit_blocks_for_device(&circuit, &device, placement, config)?;
        Ok(PyTransformResult::from_outcome(circuit, outcome))
    })
    .map_err(compiler_error_to_py_err)
}
