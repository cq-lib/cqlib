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

//! Python binding for non-unitary circuit directives.
//!
//! Directives are instructions rather than unitary gates. They do not expose a
//! matrix; only barriers have an inverse.

use crate::utils::hash_value;
use cqlib_core::circuit::gate::directive::Directive;
use pyo3::prelude::*;

/// Non-unitary barrier, measurement, or reset instruction.
#[pyclass(name = "Directive", module = "cqlib.circuit.gates", from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyDirective {
    pub(crate) inner: Directive,
}

impl From<Directive> for PyDirective {
    fn from(inner: Directive) -> Self {
        Self { inner }
    }
}

impl From<PyDirective> for Directive {
    fn from(py: PyDirective) -> Self {
        py.inner
    }
}

#[pymethods]
impl PyDirective {
    /// Creates a barrier directive.
    ///
    /// A barrier prevents gate reordering across its boundary during optimization.
    /// Useful for timing-critical sequences or hardware constraints.
    ///
    /// # Examples
    ///
    /// ```python
    /// barrier = Directive.barrier()
    /// ```
    #[staticmethod]
    fn barrier() -> Self {
        PyDirective {
            inner: Directive::Barrier,
        }
    }

    /// Creates a measure directive.
    ///
    /// A measurement operation that collapses qubit state to classical bit.
    /// Measures the qubit in the computational basis and stores the result (0 or 1).
    ///
    /// # Examples
    ///
    /// ```python
    /// measure = Directive.measure()
    /// ```
    #[staticmethod]
    fn measure() -> Self {
        PyDirective {
            inner: Directive::Measure,
        }
    }

    /// Creates a reset directive.
    ///
    /// A reset operation that prepares qubit in |0> state.
    /// Forces the qubit into the ground state regardless of its current state.
    ///
    /// # Examples
    ///
    /// ```python
    /// reset = Directive.reset()
    /// ```
    #[staticmethod]
    fn reset() -> Self {
        PyDirective {
            inner: Directive::Reset,
        }
    }

    /// Returns the name of the directive.
    ///
    /// # Returns
    ///
    /// A string: "barrier", "measure", or "reset".
    fn name(&self) -> String {
        self.inner.name().to_string()
    }

    /// Returns true if this is a barrier directive.
    fn is_barrier(&self) -> bool {
        matches!(self.inner, Directive::Barrier)
    }

    /// Returns true if this is a measure directive.
    fn is_measure(&self) -> bool {
        matches!(self.inner, Directive::Measure)
    }

    /// Returns true if this is a reset directive.
    fn is_reset(&self) -> bool {
        matches!(self.inner, Directive::Reset)
    }

    /// Returns the inverse directive when one exists.
    fn inverse(&self) -> Option<Self> {
        self.inner.inverse().map(Self::from)
    }

    fn __repr__(&self) -> String {
        match self.inner {
            Directive::Barrier => "Directive.barrier()".to_string(),
            Directive::Measure => "Directive.measure()".to_string(),
            Directive::Reset => "Directive.reset()".to_string(),
        }
    }

    fn __str__(&self) -> String {
        self.name()
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyDirective>() {
            return Ok(false);
        }
        let other = other.extract::<PyDirective>()?;
        Ok(self.inner == other.inner)
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

#[cfg(test)]
mod tests {
    use super::PyDirective;

    #[test]
    fn only_barrier_has_an_inverse() {
        assert!(PyDirective::barrier().inverse().is_some());
        assert!(PyDirective::measure().inverse().is_none());
        assert!(PyDirective::reset().inverse().is_none());
    }

    #[test]
    fn names_delegate_to_stable_core_names() {
        assert_eq!(PyDirective::barrier().name(), "barrier");
        assert_eq!(PyDirective::measure().name(), "measure");
        assert_eq!(PyDirective::reset().name(), "reset");
        assert_eq!(PyDirective::barrier().__str__(), "barrier");
    }
}
