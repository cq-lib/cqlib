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

//! Python bindings for the exact-physical native fixed-point optimizer.

use super::resynthesis::PyTwoQubitBlockResynthesisConfig;
use crate::circuit::PyCircuit;
use crate::compile::error::compiler_error_to_py_err;
use crate::device::device_impl::PyDevice;
use cqlib_core::compile::transform::{
    NativeOptimizationResult, NativeOptimizationSummary, NativeOptimizer,
};
use pyo3::prelude::*;
use std::sync::Arc;

/// Exact physical cost summary at one native optimizer checkpoint.
#[pyclass(
    name = "NativeOptimizationSummary",
    module = "cqlib.compile.transform.native_optimization",
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyNativeOptimizationSummary {
    inner: NativeOptimizationSummary,
}

impl From<NativeOptimizationSummary> for PyNativeOptimizationSummary {
    fn from(inner: NativeOptimizationSummary) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNativeOptimizationSummary {
    #[getter]
    fn native_two_qubit_ops(&self) -> u64 {
        self.inner.native_two_qubit_ops
    }

    #[getter]
    fn native_two_qubit_depth(&self) -> u64 {
        self.inner.native_two_qubit_depth
    }

    #[getter]
    fn total_native_depth(&self) -> u64 {
        self.inner.total_native_depth
    }

    #[getter]
    fn native_total_ops(&self) -> u64 {
        self.inner.native_total_ops
    }

    #[getter]
    fn predicted_log_error(&self) -> Option<f64> {
        self.inner.predicted_log_error
    }

    #[getter]
    fn unavailable_error_count(&self) -> u64 {
        self.inner.unavailable_error_count
    }

    #[getter]
    fn imputed_error_count(&self) -> u64 {
        self.inner.imputed_error_count
    }

    fn __repr__(&self) -> String {
        format!(
            "NativeOptimizationSummary(native_two_qubit_ops={}, native_two_qubit_depth={}, total_native_depth={}, native_total_ops={}, predicted_log_error={:?}, unavailable_error_count={}, imputed_error_count={})",
            self.inner.native_two_qubit_ops,
            self.inner.native_two_qubit_depth,
            self.inner.total_native_depth,
            self.inner.native_total_ops,
            self.inner.predicted_log_error,
            self.inner.unavailable_error_count,
            self.inner.imputed_error_count,
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

/// Structured result returned by [`PyNativeOptimizer`].
#[pyclass(
    name = "NativeOptimizationResult",
    module = "cqlib.compile.transform.native_optimization",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyNativeOptimizationResult {
    inner: NativeOptimizationResult,
}

impl From<NativeOptimizationResult> for PyNativeOptimizationResult {
    fn from(inner: NativeOptimizationResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNativeOptimizationResult {
    #[getter]
    fn circuit(&self) -> PyCircuit {
        self.inner.circuit.clone().into()
    }

    #[getter]
    fn changed(&self) -> bool {
        self.inner.changed
    }

    #[getter]
    fn rounds(&self) -> u8 {
        self.inner.rounds
    }

    #[getter]
    fn restored_best(&self) -> bool {
        self.inner.restored_best
    }

    #[getter]
    fn before(&self) -> PyNativeOptimizationSummary {
        self.inner.before.into()
    }

    #[getter]
    fn after(&self) -> PyNativeOptimizationSummary {
        self.inner.after.into()
    }

    fn __repr__(&self) -> String {
        format!(
            "NativeOptimizationResult(changed={}, rounds={}, restored_best={})",
            if self.inner.changed { "True" } else { "False" },
            self.inner.rounds,
            if self.inner.restored_best {
                "True"
            } else {
                "False"
            },
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Reusable exact-physical native fixed-point optimizer.
#[pyclass(
    name = "NativeOptimizer",
    module = "cqlib.compile.transform.native_optimization",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyNativeOptimizer {
    inner: Arc<NativeOptimizer<'static>>,
    resynthesis: PyTwoQubitBlockResynthesisConfig,
    enhanced: bool,
}

impl std::fmt::Debug for PyNativeOptimizer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PyNativeOptimizer")
            .field("device", &self.inner.device().name())
            .field("enhanced", &self.enhanced)
            .field("max_rounds", &self.inner.max_rounds())
            .field("max_stale_rounds", &self.inner.max_stale_rounds())
            .finish()
    }
}

#[pymethods]
impl PyNativeOptimizer {
    /// Creates a reusable optimizer over an immutable device snapshot.
    ///
    /// `enhanced` selects the production normal/enhanced resynthesis defaults
    /// and round budgets. An explicit `resynthesis` replaces only the
    /// resynthesis configuration; explicit round limits replace the matching
    /// production budget.
    #[new]
    #[pyo3(signature = (device, *, enhanced=false, resynthesis=None, max_rounds=None, max_stale_rounds=None))]
    fn new(
        device: PyRef<'_, PyDevice>,
        enhanced: bool,
        resynthesis: Option<PyTwoQubitBlockResynthesisConfig>,
        max_rounds: Option<u8>,
        max_stale_rounds: Option<u8>,
    ) -> PyResult<Self> {
        let resynthesis = resynthesis
            .unwrap_or_else(|| PyTwoQubitBlockResynthesisConfig::unconstrained(enhanced));
        let max_rounds = max_rounds.unwrap_or(if enhanced {
            NativeOptimizer::ENHANCED_MAX_ROUNDS
        } else {
            NativeOptimizer::NORMAL_MAX_ROUNDS
        });
        let max_stale_rounds = max_stale_rounds.unwrap_or(if enhanced {
            NativeOptimizer::ENHANCED_MAX_STALE_ROUNDS
        } else {
            NativeOptimizer::NORMAL_MAX_STALE_ROUNDS
        });
        let inner = NativeOptimizer::new_owned(
            device.inner.clone(),
            resynthesis.inner.clone(),
            max_rounds,
            max_stale_rounds,
        )
        .map_err(compiler_error_to_py_err)?;
        Ok(Self {
            inner: Arc::new(inner),
            resynthesis,
            enhanced,
        })
    }

    #[getter]
    fn device(&self) -> PyDevice {
        self.inner.device().clone().into()
    }

    #[getter]
    fn enhanced(&self) -> bool {
        self.enhanced
    }

    #[getter]
    fn resynthesis(&self) -> PyTwoQubitBlockResynthesisConfig {
        self.resynthesis.clone()
    }

    #[getter]
    fn max_rounds(&self) -> u8 {
        self.inner.max_rounds()
    }

    #[getter]
    fn max_stale_rounds(&self) -> u8 {
        self.inner.max_stale_rounds()
    }

    /// Optimizes an exact-native physical circuit without modifying the input.
    fn run(
        &self,
        py: Python<'_>,
        circuit: PyRef<'_, PyCircuit>,
    ) -> PyResult<PyNativeOptimizationResult> {
        let optimizer = Arc::clone(&self.inner);
        let circuit = circuit.inner.clone();
        py.detach(move || optimizer.run(&circuit).map(Into::into))
            .map_err(compiler_error_to_py_err)
    }

    fn __repr__(&self) -> String {
        format!(
            "NativeOptimizer(device=Device(name={:?}), enhanced={}, max_rounds={}, max_stale_rounds={})",
            self.inner.device().name(),
            if self.enhanced { "True" } else { "False" },
            self.inner.max_rounds(),
            self.inner.max_stale_rounds(),
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

pub(crate) fn register_native_optimization_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(parent.py(), "native_optimization")?;
    module.add_class::<PyNativeOptimizationSummary>()?;
    module.add_class::<PyNativeOptimizationResult>()?;
    module.add_class::<PyNativeOptimizer>()?;
    parent.add_submodule(&module)?;
    parent.py().import("sys")?.getattr("modules")?.set_item(
        "cqlib._native.compile.transform.native_optimization",
        &module,
    )?;
    Ok(())
}
